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
use crate::call::{CallError, ErrorKind};
use crate::session::Session;
use escribass_proto::assistant::assistant_client::AssistantClient;
use escribass_proto::assistant::{
    assistant_command, assistant_event, AssistantCommand, CallResult, CompletedCall, Prompt,
    ToolCall, ToolSchema as WireSchema, Turn as AssistantTurn,
};
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

// ---------------------------------------------------------------------------
// The turn (ADR 0019 §1, ADR 0020 §3, ADR 0022 §3)
// ---------------------------------------------------------------------------

/// How many calls a turn may have refused before it ends (ADR 0022 §3).
///
/// **Per turn, not per call**: a loop cannot tell a retry of one call from a new call — it has
/// no call identity to compare — and per turn is the tightest of the bounds the plan weighed.
/// The number is the loop's configuration and not a rule; what is fixed is that it is counted
/// in exactly one place, which is here, because the host is what executes a call and therefore
/// what sees it refused. The spike's worst runs were 19 and 58 refused guesses on one
/// instruction, which is the shape this caps, in money.
pub const REFUSALS_PER_TURN: usize = 3;

/// How a turn ended.
///
/// Three of the four ways are here; the fourth is not a value at all. A provider failure, a
/// model that ran out of room and the sidecar itself each end the stream with a status and
/// leave through `Err(ProjectError)`, because a failure that travels as an arm is one a caller
/// can forget to read (ADR 0013 §2, ADR 0022 §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnEnd {
    /// The model answered and asked for nothing more (ADR 0020 §3's `done`).
    Answered,
    /// [`REFUSALS_PER_TURN`] calls in this turn were refused and the turn ended. The proposal
    /// is still pending, with whatever the accepted calls put in it, and the person is told
    /// which calls were refused and why — every violation is in `recorded.calls`.
    Refused,
}

/// One finished turn, and what it left behind.
#[derive(Debug, Clone, PartialEq)]
pub struct Turn {
    /// The conversation's row for this turn — the prompt, every call with what it was answered,
    /// and the reply — which is what the next prompt carries back so `ai` holds nothing between
    /// streams (ADR 0021 §3).
    pub recorded: AssistantTurn,
    /// The model the turn's **last** response named: what actually answered, never the id that
    /// was asked for, and what the entry records when a person applies (ADR 0021 §2).
    pub model_id: String,
    pub end: TurnEnd,
}

impl Sidecar {
    /// Drives one turn to its end: the prompt out, the model's calls executed against the
    /// session's proposal, each result back, until `done` (ADR 0019 §1, ADR 0020 §3).
    ///
    /// **Nothing is applied.** The turn leaves a proposal pending on the session; a person
    /// applies or rejects it, and until they do the project has not been touched (ADR 0019 §2).
    ///
    /// ADR 0022 §3's three kinds meet here, and each leaves by a different door:
    ///
    /// - a **refusal** goes back to the model whole, violations included, and is counted —
    ///   [`REFUSALS_PER_TURN`] of them end the turn, which is the only counting this loop does;
    /// - an **operator error** ends the turn at the host before the model sees it, with zero
    ///   retries, because nothing the model says differently would fix a project that will not
    ///   write and a model told to retry would spend its turn learning that;
    /// - a **provider failure** never reaches here as a call at all. It is retried inside `ai`,
    ///   at the one place the provider is called, and arrives as a status on the closed stream —
    ///   `provider_failed`, which is neither of the other two.
    ///
    /// A fourth thing is named so it is not mistaken for one of the three: a response with
    /// neither a call nor text, and a model that will not stop calling, both end the stream
    /// `turn_unfinished`. That is the model exhausting itself rather than the provider failing,
    /// and a retry reproduces it.
    pub fn turn(
        &mut self,
        session: &mut Session,
        text: &str,
        conversation: &[AssistantTurn],
    ) -> Result<Turn, ProjectError> {
        // Content-addressed by the store's one hasher, so the same text asked twice names the
        // same id and a scripted transcript replays to the same log (ADR 0021 §2).
        let prompt_id = crate::project::asset_hash(text.as_bytes());
        session.propose(&prompt_id)?;
        let outcome = self.asked(session, text, conversation);
        if outcome.is_err() {
            // An operator error, or the sidecar's own: the proposal goes, because nothing a
            // person could act on came back and a half-built fork nobody has seen is not a
            // thing to leave pending (ADR 0022 §3). `propose` is outside this, because a turn
            // refused for a proposal that is already pending must not drop *that* one.
            session.reject();
        }
        outcome
    }

    /// Everything after the proposal is open: the model recorded, the prompt built, the stream
    /// driven.
    fn asked(
        &mut self,
        session: &mut Session,
        text: &str,
        conversation: &[AssistantTurn],
    ) -> Result<Turn, ProjectError> {
        // Written on first use, never rewritten by a tool (ADR 0021 §4). Before the prompt goes
        // out rather than after, because what it records is the model a prompt **was sent to**,
        // and a turn that fails was still sent. It is also the one place in a turn where the
        // project itself can refuse — nothing else here writes — so a project that will not
        // write ends the turn here, with zero retries and before the model is asked anything
        // (ADR 0022 §3).
        let (provider, model) = {
            let (provider, model) = session.ai();
            (provider.to_string(), model.to_string())
        };
        session.record_ai(&provider, &model)?;

        let prompt = Prompt {
            text: text.to_string(),
            song: Some(session.project().song().clone()),
            tools: offered(),
            model_id: model,
            conversation: conversation.to_vec(),
        };
        self.drive(session, prompt, text)
    }

    fn drive(
        &mut self,
        session: &mut Session,
        prompt: Prompt,
        text: &str,
    ) -> Result<Turn, ProjectError> {
        let address = self.address.clone();
        let driven = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| Interrupted::Transport(e.to_string()))?;
                    runtime.block_on(one_turn(&address, session, prompt, text))
                })
                .join()
                .expect("the thread that drives the assistant")
        });
        driven.map_err(|why| self.interrupted(why))
    }

    /// What an interrupted turn means, and which of ADR 0022 §3's kinds it is.
    ///
    /// The distinction is the gRPC **status code**, and it is readable only because a status
    /// the sidecar *returned* and a transport that broke are different values by the time they
    /// reach here ([`Interrupted`]). A dead child overrides both: its exit status is the more
    /// useful sentence, and a provider cannot have failed inside a process that is gone.
    fn interrupted(&mut self, why: Interrupted) -> ProjectError {
        let status = match why {
            Interrupted::Said(status) => status,
            Interrupted::Transport(said) => return self.did_not_answer(&said),
            // The project's own, carried out of the stream (ADR 0022 §3). Zero retries, and the
            // sidecar is left alone: it did nothing wrong.
            Interrupted::Operator(said) => return self.broke("project_failed", said),
        };
        if !matches!(self.child.try_wait(), Ok(None)) {
            return self.did_not_answer(&status.message().to_string());
        }
        let rule = match status.code() {
            // The provider's own failure, after `ai` retried it its bounded number of times.
            // Never a refusal and never a project error: the caller cannot fix it by calling
            // differently and no operator of the project can fix it either (ADR 0022 §3).
            tonic::Code::Unavailable => "provider_failed",
            // The model produced neither a call nor text, or would not stop calling. Not the
            // provider failing, and a retry with backoff reproduces it.
            tonic::Code::ResourceExhausted => "turn_unfinished",
            _ => "assistant_failed",
        };
        self.broke(rule, status.message().to_string())
    }
}

/// One turn on the stream, driven to its end.
///
/// The send half stays open for the whole turn, which is what makes this a loop rather than
/// PR 5's single question: the host answers each `ToolCall` with a `CallResult` on the same
/// stream, and the stream is the turn's identifier (ADR 0020 §3).
async fn one_turn(
    address: &str,
    session: &mut Session,
    prompt: Prompt,
    text: &str,
) -> Result<Turn, Interrupted> {
    let channel = Endpoint::from_shared(address.to_string())
        .map_err(|e| Interrupted::Transport(e.to_string()))?
        .connect()
        .await
        .map_err(|e| Interrupted::Transport(e.to_string()))?;
    let (commands, outgoing) = tokio::sync::mpsc::unbounded_channel();
    let send = |command: assistant_command::Command| {
        commands
            .send(AssistantCommand { command: Some(command) })
            .map_err(|e| Interrupted::Transport(e.to_string()))
    };
    send(assistant_command::Command::Prompt(prompt))?;

    let mut inbound = AssistantClient::new(channel)
        // A prompt carries the whole `Song`, as a plan does — so nothing here caps a message
        // and a large project meets the validator's limits rather than a refusal the transport
        // invented (ADR 0007 §6).
        .max_encoding_message_size(usize::MAX)
        .max_decoding_message_size(usize::MAX)
        .prompt(UnboundedReceiverStream::new(outgoing))
        .await
        .map_err(Interrupted::Said)?
        .into_inner();

    let mut recorded =
        AssistantTurn { prompt: text.to_string(), calls: vec![], reply: String::new() };
    let mut refusals = 0usize;

    while let Some(event) = inbound.message().await.map_err(Interrupted::Said)? {
        match event.event {
            // Streamed as the model produces it, for the panel to draw (PR 9). This build's
            // sidecar sends one fragment per turn; several would concatenate the same way.
            Some(assistant_event::Event::Text(said)) => recorded.reply.push_str(&said.text),
            Some(assistant_event::Event::Done(done)) => {
                recorded.reply = done.text;
                return Ok(Turn { recorded, model_id: done.model_id, end: TurnEnd::Answered });
            }
            Some(assistant_event::Event::Call(call)) => {
                // The model that made *this* call, which on the path below is the turn's last
                // response — there is no later one, because the turn ends here (ADR 0021 §2).
                let model_id = call.model_id.clone();
                let result = execute(session, &call)?;
                let refused = !result.valid;
                recorded.calls.push(CompletedCall {
                    call: Some(call.clone()),
                    result: Some(result.clone()),
                });
                send(assistant_command::Command::Result(CallResult {
                    call_id: call.call_id,
                    result: Some(result),
                    // The document as the proposal now has it, from which `ai` recomputes the
                    // view (ADR 0018 §2). Sent again rather than diffed: a diff here would be a
                    // second RFC 6902 applied where the validator does not run.
                    song: session.proposal().map(|p| p.song().clone()),
                }))?;
                if refused {
                    refusals += 1;
                    if refusals >= REFUSALS_PER_TURN {
                        // Closing the stream is how a turn is cancelled (ADR 0013 §2), and
                        // dropping the send half is closing it. The proposal stays pending with
                        // whatever the accepted calls put in it.
                        return Ok(Turn { recorded, model_id, end: TurnEnd::Refused });
                    }
                }
            }
            // A stream that says nothing is a sidecar with a defect, not a turn.
            None => {
                return Err(Interrupted::Transport(
                    "the assistant sent an event with no arm (assistant.proto)".to_string(),
                ))
            }
        }
    }
    Err(Interrupted::Transport(
        "the assistant ended the turn without answering (assistant.proto, `Done`)".to_string(),
    ))
}

/// One of the model's calls, executed against the proposal (ADR 0019 §1).
///
/// ADR 0006 §2's split, read one layer up. A `ToolResult` — valid or refused — is what goes
/// back to the model; a `Broken` call error is the operator's and leaves the loop entirely, as
/// `Err`, with zero retries.
///
/// A `BadRequest` is a refusal like any other and is **shaped** into a `ToolResult` here rather
/// than being a third thing on the wire: a tool name the model invented, an argument that will
/// not decode and a tool it was not offered are all things it can fix by calling differently,
/// which is the definition ADR 0006 §2 gives. Its `path` is `/` because there is no operation
/// to point at — the call never became one.
fn execute(
    session: &mut Session,
    call: &ToolCall,
) -> Result<escribass_proto::tools::ToolResult, Interrupted> {
    use escribass_proto::tools::ToolResult;

    let arguments: serde_json::Map<String, serde_json::Value> =
        match serde_json::from_str(&call.args_json) {
            Ok(serde_json::Value::Object(map)) => map,
            // Not parsed into a request and not validated here: the tool API is the validator,
            // and a second one in the host would be the same drift trap 12 names for Python.
            // All this decides is whether there is an object to hand over at all.
            _ => {
                return Ok(refused_call(
                    "arguments_unreadable",
                    format!("`{}`: arguments are a JSON object, not {}", call.name, call.args_json),
                ))
            }
        };
    let proposal = session.proposal_mut().expect("a turn opens its proposal before it runs");
    match proposal.call(&call.name, &arguments, &call.model_id, &call.call_id) {
        Ok(answer) => Ok(answer.result.unwrap_or_else(|| ToolResult {
            valid: !answer.refused,
            errors: vec![],
            patch: vec![],
            summary: answer.text,
            entry_id: String::new(),
        })),
        Err(CallError { kind: ErrorKind::BadRequest, message }) => {
            Ok(refused_call("call_unreadable", message))
        }
        // Structurally unreachable on a fork today, and kept because "today" is the argument:
        // a proposal records nothing and writes nothing, so none of the twelve can reach a
        // `Project::write`, a lock or a `head_unset`. What a turn *can* meet is the project
        // refusing to record the model it was sent to, which happens in `turn` before the
        // prompt goes out. If an offered tool ever gains an operator failure, it leaves here,
        // with zero retries (ADR 0022 §3).
        Err(CallError { kind: ErrorKind::Broken, message }) => Err(Interrupted::Operator(message)),
    }
}

fn refused_call(rule: &str, message: String) -> escribass_proto::tools::ToolResult {
    escribass_proto::tools::ToolResult {
        valid: false,
        errors: vec![escribass_proto::tools::Violation {
            path: "/".to_string(),
            rule: rule.to_string(),
            message,
        }],
        patch: vec![],
        summary: String::new(),
        entry_id: String::new(),
    }
}

/// The schemas the model is offered, from the descriptor `core` already carries (ADR 0022 §1).
///
/// `expect` rather than a result: the descriptor is compiled into this binary and `OFFERED` is
/// a constant beside `IMPLEMENTED`, so the only way this fails is a build whose two halves
/// disagree — which is a defect at start-up, not a turn's failure to report.
fn offered() -> Vec<WireSchema> {
    crate::descriptor::offered_schemas(escribass_proto::DESCRIPTOR)
        .expect("the offered tools are the descriptor's own")
        .into_iter()
        .map(|tool| WireSchema {
            name: tool.name,
            description: tool.description,
            // Text rather than a `Struct`: the schema is handed to the provider verbatim, and
            // modelling it on the wire would describe a second time what the descriptor
            // produced once (assistant.proto).
            input_schema: serde_json::to_string(&tool.input_schema).expect("a schema serialises"),
        })
        .collect()
}

/// Why a turn stopped, in the one distinction the host cannot recover afterwards: a status the
/// sidecar **returned** — which is `ai` reporting a failure it has already classified — against
/// a transport that broke, which is nobody's report at all.
enum Interrupted {
    Said(tonic::Status),
    Transport(String),
    /// A project error met while executing one of the model's calls (ADR 0022 §3). Its message
    /// is the one every carrier has always shown — `<path> [<rule>]: <message>` — because that
    /// is what `core::call` hands back, and re-splitting it to rebuild the value it came from
    /// would be a second spelling of the same three fields.
    Operator(String),
}
