//! Starting the AI sidecar and talking to it (ADR 0020 §4) — the engine's spawn-and-dial, one
//! process over, and the two places it deliberately differs.
//!
//! The whole protocol is the engine's: start a child from a command `core` was **told**, read
//! **one line** from its stdout — `unix:<path>`, the address of the socket it is now serving
//! `Assistant` on, printed only once the server is listening — and dial it (ADR 0013 §3;
//! [`crate::engine`], which owns the two functions both use). That one line is the address and
//! the readiness at once, so there is nothing to poll, no port to guess and no sleep to tune;
//! and the path is the child's to invent, because a parent-chosen one needs entropy in `core`
//! (CLAUDE.md #3) or a pid — the defect M1 PR 13 fixed in the engine's scratch directory.
//!
//! **`core` is told a command, not a path, and does not inspect it.** On a build tree it is
//! `uv run --project <ai/> escribass-ai …`; in a bundle it will be something else. That keeps
//! `core` ignorant of Python in the only sense that matters — it knows a command as it knows
//! a binary — and it keeps the spawn here, in `core`, rather than in the Tauri host, so the
//! loop's host half can be driven from `tests/` with no window (ADR 0020 §4).
//!
//! **What the sidecar is told, and what it is never told.** Never the project path: `.escri`
//! is the host's, and §5's "agents never read or write the project file directly" is a
//! property of what `ai` is given rather than of what it is told not to do. Never a socket,
//! since it names its own. Never the key: `OPENROUTER_API_KEY` is inherited from this
//! process's environment, never passed as a flag, because `ps` shows flags (U3; `docs/plan.md`,
//! M3 trap 10). The environment is inherited whole for the engine's reason, one process over.
//!
//! **Where this differs from the engine, and why.**
//!
//! 1. **A sidecar outlives a call.** An engine is born with its work and dies with its answer
//!    (ADR 0008 §2), so every failed call there ends in a verdict that kills. A sidecar serves
//!    a session: a turn that fails while the child is still running is reported as the
//!    transport saw it and the child is left alone, because killing it would turn one bad turn
//!    into a dead assistant.
//! 2. **Its exit status is read, and kept.** ADR 0020 §5 asks for the window's health dot to be
//!    the child's exit status *read*, and says in as many words that [`crate::engine::Preview`]'s
//!    gap — a `Drop` that discards it — is not repeated here. So [`Sidecar::health`] reads the
//!    status the moment the child is gone and remembers it with the tail of its stderr, and
//!    [`Sidecar::stop`] hands the same sentence back at the end of a session. The dot never
//!    says more than that: running, or gone and how.
//!
//! **Stopping it is closing its stdin.** A sidecar has no "last call" to end on, so the pipe is
//! the signal — the same one `escribass-mcp` ends on (ADR 0020, Context), and the reason its
//! stdin is a pipe here where the engine's is `/dev/null`: this is a pipe of our own, so nothing
//! competes for the MCP server's stdin (ADR 0006 §6). A child that will not leave is killed,
//! after the grace the engine already measures for one that is on its way out.
//!
//! Nothing here reads a clock to decide *what* is answered. The one clock is
//! [`NAMING_ITS_SOCKET`], in the failure path alone, and a turn itself is
//! **unbounded**: how long a model takes is the model's, and the provider's own timeout is
//! `ai`'s to keep at the one place the provider is called (ADR 0022 §3). Cancelling a turn is
//! dropping what holds it, which closes the stream (ADR 0013 §2).

use crate::engine::{drain, ended, how, listening, tail, LEAVING_OF_ITS_OWN_ACCORD};
use crate::project::ProjectError;
use escribass_proto::assistant::assistant_client::AssistantClient;
use escribass_proto::assistant::{assistant_command, AssistantCommand, AssistantEvent, Prompt};
use std::process::{Child, Command, Stdio};
use std::thread::JoinHandle;
use std::time::Duration;
use tonic::codegen::tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::transport::Endpoint;

/// How long the sidecar has to name its socket before it is given up on.
///
/// A minute, as the engine gets: what is being waited for is a Python interpreter starting and
/// importing `openai`, `grpclib` and the generated models. A clock in the failure path alone —
/// it decides *whether* the sidecar started and never what it answers.
const NAMING_ITS_SOCKET: Duration = Duration::from_secs(60);

/// The command that starts the sidecar.
///
/// Told, never searched: a search would find whichever `uv` or Python is first on a path, which
/// is ADR 0008 §2's argument for `--engine` in the one language it has not been made in yet.
#[derive(Debug, Clone)]
pub struct Assistant {
    pub command: Vec<String>,
}

impl Assistant {
    pub fn new(command: Vec<String>) -> Self {
        Self { command }
    }

    /// Starts it, reads the line it names its socket on, and returns the running process.
    pub fn start(&self) -> Result<Sidecar, ProjectError> {
        let Some((program, arguments)) = self.command.split_first() else {
            return Err(self.broke("assistant_missing", "the assistant command is empty".into()));
        };
        let mut child = Command::new(program)
            .args(arguments)
            // **A pipe of our own, not `/dev/null` and not inherited.** Closing it is how a
            // session ends (see the module note); inheriting this process's would put the
            // sidecar in competition with the MCP server for stdin (ADR 0006 §6).
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                self.broke("assistant_missing", format!("cannot start the assistant: {e}"))
            })?;

        // Drained on a thread of its own, for the engine's reason: a full stderr pipe stops the
        // child, and this one lives for a session rather than for a call. It is also what the
        // health dot quotes when the sidecar dies (ADR 0020 §5).
        let said = child.stderr.take().map(drain);

        match listening(&mut child, NAMING_ITS_SOCKET, "assistant") {
            Ok(address) => Ok(Sidecar {
                assistant: self.clone(),
                child,
                said,
                address,
                gone: None,
                stdin_closed: false,
            }),
            Err(why) => {
                let gone = ended(&mut child, said, &why);
                let rule =
                    if gone.crashed { "assistant_failed" } else { "assistant_unreadable" };
                Err(self.broke(rule, gone.message))
            }
        }
    }

    fn broke(&self, rule: &'static str, message: String) -> ProjectError {
        ProjectError { path: self.command.join(" "), rule, message }
    }
}

/// Whether the sidecar is running, for the dot in the window (ADR 0020 §5).
///
/// Two states and no third, because two is what `core` can state as a fact. "Healthy" would be
/// a claim about a hosted model that nothing here can check, and M3 trap 15 is about exactly
/// that kind of readout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    Running,
    /// Gone, and how — its exit status and the tail of its stderr, read once and kept.
    Gone(String),
}

/// A running sidecar: the process, and the address it serves `Assistant` on.
///
/// **Its lifetime is the session's** (ADR 0020 §4). Dropping it closes its stdin, gives it the
/// moment it takes to leave, and kills it if it will not — so a host that dies takes its
/// sidecar with it rather than leaving a process holding a socket nobody will dial.
#[derive(Debug)]
pub struct Sidecar {
    assistant: Assistant,
    child: Child,
    said: Option<JoinHandle<Vec<u8>>>,
    address: String,
    /// What became of it, read once. Kept because the tail of its stderr can only be taken
    /// once and the window asks again every second.
    gone: Option<String>,
    stdin_closed: bool,
}

impl Sidecar {
    /// Where it serves — `unix:<path>`, as it named itself.
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Whether it is still running, or what became of it.
    ///
    /// **The exit status, read** — and the whole of what the health dot is (ADR 0020 §5). It
    /// is deliberately not `Result`: a dead sidecar is not an error until something asks it
    /// for a turn, and the window asks this on a timer.
    pub fn health(&mut self) -> Health {
        if let Some(gone) = &self.gone {
            return Health::Gone(gone.clone());
        }
        let said = match self.child.try_wait() {
            Ok(None) => return Health::Running,
            Ok(Some(finished)) => {
                format!("the assistant {}{}", how(&finished), tail(self.said.take()))
            }
            Err(e) => format!("the assistant could not be waited for: {e}"),
        };
        self.gone = Some(said.clone());
        Health::Gone(said)
    }

    /// Sends one prompt and returns every event of the turn, in order.
    ///
    /// The stream is the turn: one `Prompt` goes out, text and `done` come back, and the
    /// sidecar ending the stream ends the turn (ADR 0020 §3).
    ///
    /// `ponytail:` it collects the events rather than handing them over as they arrive, and it
    /// closes its own half of the stream after the prompt, because this build's sidecar asks
    /// for nothing back. Both change in M3 PR 8, where a turn is a loop: the host answers each
    /// `ToolCall` with a `CallResult` on this same stream, so the send half stays open and the
    /// events are consumed one at a time.
    pub fn answer(&mut self, prompt: Prompt) -> Result<Vec<AssistantEvent>, ProjectError> {
        match one_turn(&self.address, prompt) {
            Ok(events) => Ok(events),
            Err(why) => Err(self.did_not_answer(&why)),
        }
    }

    /// Closes its stdin, waits for it to leave, and says how it went.
    ///
    /// The status is read on this path too, rather than discarded by `Drop` — which is the
    /// gap ADR 0020 §5 names in [`crate::engine::Preview`] and asks not to be repeated.
    pub fn stop(mut self) -> String {
        self.close_stdin();
        for _ in 0..LEAVING_OF_ITS_OWN_ACCORD {
            if !matches!(self.child.try_wait(), Ok(None)) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        match self.health() {
            Health::Gone(said) => said,
            // `wait` above returned, so this arm is unreachable in practice; saying something
            // true is cheaper than an `expect` in a shutdown path.
            Health::Running => "the assistant is still running".to_string(),
        }
    }

    /// What a turn that did not answer means, which depends on whether the child is still there.
    fn did_not_answer(&mut self, why: &str) -> ProjectError {
        if matches!(self.child.try_wait(), Ok(None)) {
            // Alive, so it is not killed: a sidecar serves a session and one failed turn is not
            // a reason to end it. The report is what the call said, because the child has said
            // nothing final.
            return self
                .broke("assistant_failed", format!("the assistant did not answer: {why}"));
        }
        let gone = ended(&mut self.child, self.said.take(), "the assistant did not answer");
        let rule = if gone.crashed { "assistant_failed" } else { "assistant_unreadable" };
        self.gone = Some(gone.message.clone());
        self.broke(rule, gone.message)
    }

    fn close_stdin(&mut self) {
        // Idempotent: `stop` closes it and `Drop` runs afterwards.
        if !self.stdin_closed {
            drop(self.child.stdin.take());
            self.stdin_closed = true;
        }
    }

    fn broke(&self, rule: &'static str, message: String) -> ProjectError {
        self.assistant.broke(rule, message)
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        self.close_stdin();
        for _ in 0..LEAVING_OF_ITS_OWN_ACCORD {
            if !matches!(self.child.try_wait(), Ok(None)) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// One turn, on a runtime and a thread of its own.
///
/// Both are required rather than tidy, for [`crate::engine`]'s reason: this is a synchronous
/// call that may already be running inside a gRPC server's runtime, where `block_on` panics.
fn one_turn(address: &str, prompt: Prompt) -> Result<Vec<AssistantEvent>, String> {
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?;
                runtime.block_on(async {
                    let channel = Endpoint::from_shared(address.to_string())
                        .map_err(|e| e.to_string())?
                        .connect()
                        .await
                        .map_err(|e| e.to_string())?;
                    let (commands, outgoing) = tokio::sync::mpsc::unbounded_channel();
                    commands
                        .send(AssistantCommand {
                            command: Some(assistant_command::Command::Prompt(prompt)),
                        })
                        .map_err(|e| e.to_string())?;
                    // Half-closed at once: this build sends one message per turn (see
                    // `Sidecar::answer`'s note).
                    drop(commands);
                    let mut inbound = AssistantClient::new(channel)
                        // A prompt carries the whole `Song`, as a plan does — so nothing here
                        // caps a message and a large project meets the validator's limits
                        // rather than a refusal the transport invented (ADR 0007 §6).
                        .max_encoding_message_size(usize::MAX)
                        .prompt(UnboundedReceiverStream::new(outgoing))
                        .await
                        .map_err(|status| status.to_string())?
                        .into_inner();
                    let mut events = Vec::new();
                    while let Some(event) =
                        inbound.message().await.map_err(|status| status.to_string())?
                    {
                        events.push(event);
                    }
                    Ok::<_, String>(events)
                })
            })
            .join()
            .expect("the thread that calls the assistant")
    })
}
