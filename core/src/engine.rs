//! Handing one plan to one engine process, over gRPC (ADR 0008 §1, ADR 0013 §3) — and, since
//! M2 PR 10, holding the one live process a preview plays through ([`Preview`]).
//!
//! The whole protocol: spawn the binary in `--render` mode with the manifest as its argument,
//! read **one line** from its stdout — the address of the Unix socket it is now serving
//! `Render` on — dial it, make one call, and take the exit code as the verdict. There is no
//! session, because the process is born with its work and dies with its answer (ADR 0008 §2),
//! and no port to agree on, because the engine names its own socket in a `mkdtemp` directory of
//! its own and tells us where it is.
//!
//! **That one line is the address and the readiness at once, and that is what keeps it from
//! racing.** The engine prints it only after gRPC has returned a listening server, so a caller
//! holding the line cannot be refused a connection: there is nothing to poll, no port to guess
//! and no sleep to tune. Reading it is a blocking read on a pipe — it returns when the engine
//! is ready or when the engine is gone, and both are answers.
//!
//! **Every failure here is an operator's** (ADR 0006 §2). By the time a plan crosses this
//! boundary the validator has refused an unresolvable reference, ADR 0010's load check has
//! refused a plugin this build lacks, and `compile` has refused what this milestone cannot
//! render — so what is left is a binary that is missing, will not start, crashes, or answers
//! with something that is not a `RenderResult`. None of those is fixable by calling
//! differently, and reporting one as a refusal would put a model in a retry loop that cannot
//! succeed.
//!
//! **A gRPC failure is never itself the verdict.** A connection that is refused, a stream that
//! ends mid-call and a `Status` returned instead of a message all mean the same thing here —
//! the engine is not going to answer — and the useful report is the one the engine already
//! wrote: its exit code and the last of its stderr. So every unhappy path below ends at the
//! same place, [`Engine::verdict`], which asks the child what became of it rather than asking
//! the transport what it saw.
//!
//! Nothing here reads a clock or takes entropy to decide *what* is rendered: a subprocess is a
//! place both could enter `core` by the back door (CLAUDE.md #3). The plan is a pure function
//! of the document, the engine's answer is a function of the plan, the socket path is the
//! engine's own to invent, and the three clocks this file does read decide only failure and which
//! of two failures — see [`NAMING_ITS_SOCKET`], [`ANSWERING`] and [`LEAVING_OF_ITS_OWN_ACCORD`].
//! A preview's position is the device's clock, read by the engine and reported here, never
//! measured here. The
//! environment the child inherits is the operator's;
//! `LD_LIBRARY_PATH` for a plugin's own shared libraries is the reason it is inherited rather
//! than cleared.

use crate::project::ProjectError;
use escribass_proto::render::preview_client::PreviewClient;
use escribass_proto::render::render_client::RenderClient;
use escribass_proto::render::{PreviewCommand, PreviewEvent, RenderPlan, RenderResult};
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, ChildStderr, Command, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;
// `tokio-stream`'s receiver adapter, reached through the re-export `tonic` keeps for its own
// generated code rather than as a dependency of ours — the path `core/tests/common` already
// takes for the fake engine's listener. Naming the crate directly would be a new entry under
// CLAUDE.md #4 for one type (docs/plan.md, M0.4 traps).
use tonic::codegen::tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::transport::Endpoint;

/// How long the engine has to name its socket before it is given up on.
///
/// **A clock, in the failure path alone** (CLAUDE.md #3). It decides *whether*
/// a render failed and never a sample: an engine that names its socket is then served for as
/// long as it takes, and every render that finishes finishes identically whatever this is set
/// to. It is the same bargain the engine's own dispatch loop takes for the same reason
/// (`engine/src/main.cpp`, `kStalledIterations`), and without it an engine that starts and never
/// binds — a socket path too long for `sun_path`, a gRPC that fails silently — stops the whole
/// tool API on a pipe read that will not return.
///
/// A minute rather than a second because what is being waited for is process start, JUCE and
/// Tracktion initialisation: 520 ms measured warm in M1 PR 7, on a machine doing nothing else.
const NAMING_ITS_SOCKET: Duration = Duration::from_secs(60);

/// How long a live preview has to answer one command before it is given up on.
///
/// The third clock, and in the failure path alone for the reason [`NAMING_ITS_SOCKET`] is: an
/// answer that arrives is used whenever it arrives, and what is decided here is only that an
/// engine which never answers stops holding the tool API. A minute, because the slowest command
/// is a `play` that builds an edit and instantiates every plugin in the plan.
const ANSWERING: Duration = Duration::from_secs(60);

/// How many ten-millisecond polls an engine that failed gets to exit before it is killed.
///
/// The last thing here that watches a clock, in the failure path and nowhere else,
/// and it is counted in iterations for the reason the engine's own dispatch bound is
/// (`engine/src/main.cpp`): what it decides is which of two failures is reported, never a
/// sample. Five seconds is far more than a failing engine takes to leave.
const LEAVING_OF_ITS_OWN_ACCORD: u32 = 500;

/// Where the engine is, and the manifest it is handed.
///
/// Both are **told, never searched** — the rule ADR 0010 §4 set for the manifest, for the same
/// reason one level over: a search path that silently finds a stale engine renders a project
/// against a build nobody chose, and the symptom is a golden that moved with no PR to blame
/// (`docs/plan.md`, trap 8). The manifest is the one `core` itself validated against, so the
/// engine hosts the plugins the validator resolved rather than whatever a second file says.
#[derive(Debug, Clone)]
pub struct Engine {
    pub binary: PathBuf,
    pub manifest: PathBuf,
}

impl Engine {
    pub fn new(binary: impl Into<PathBuf>, manifest: impl Into<PathBuf>) -> Self {
        Self { binary: binary.into(), manifest: manifest.into() }
    }

    /// Renders one plan, in a process of its own, and returns what the engine reported.
    ///
    /// The WAV lands at `plan.output_path`; this returns the engine's own answer about it —
    /// the hash of the `data` chunk and the commits the binary was built from (ADR 0008 §5).
    pub fn render(&self, plan: &RenderPlan) -> Result<RenderResult, ProjectError> {
        let (mut child, said, address) = self.spawn("--render")?;

        // One call, and its failure is not read for a reason — `verdict` asks the engine.
        match render_one(&address, plan) {
            Err(_) => Err(self.verdict(&mut child, said, "the engine did not answer the call")),
            Ok(result) => {
                let finished = child.wait().map_err(|e| {
                    self.broke("engine_failed", format!("the engine did not finish: {e}"))
                })?;
                if !finished.success() {
                    return Err(self.broke(
                        "engine_failed",
                        format!(
                            "the engine answered and then {}{}",
                            how(&finished),
                            tail(said),
                        ),
                    ));
                }
                // Answering is not the same as having rendered. proto3 has no required fields,
                // so a `RenderResult` with every field at its default is a well-formed answer,
                // and an engine that returned one would come back as a successful render with
                // an empty hash and no file. `song_tools.proto` §8 says the opposite in as many
                // words: an engine that "writes nothing is an operator error and never a
                // refusal". The hash is the answer, so its absence is the absence of one.
                if result.pcm_sha256.is_empty() {
                    return Err(self.broke(
                        "engine_unreadable",
                        format!(
                            "the engine answered with a RenderResult carrying no pcm_sha256; \
                             without a hash it is not an answer, and nothing was written \
                             (ADR 0008 §1){}",
                            tail(said),
                        ),
                    ));
                }
                Ok(result)
            }
        }
    }

    /// Starts a live preview: one engine process in `--preview` mode, and the one `Preview`
    /// stream it plays for as long as the returned handle lives (ADR 0013 §2, §3).
    ///
    /// **A process of its own, and never the one an export runs in.** `render` above spawns
    /// afresh in `--render` mode every time, whatever is playing, and that process registers
    /// `Render` alone — so the guarantee ADR 0008 §2 bought with process death is gRPC's
    /// dispatch here and not a rule anyone maintains (docs/plan.md, M2 trap 6).
    ///
    /// A machine with no audio output device fails **here**, loudly: the engine opens the
    /// device before it serves anything, and exits `6` saying which device types it found and
    /// what they listed — which reaches the caller as `engine_failed` carrying that sentence.
    pub fn preview(&self) -> Result<Preview, ProjectError> {
        let (child, said, address) = self.spawn("--preview")?;
        let (commands, outgoing) = tokio::sync::mpsc::unbounded_channel();
        let (arrived, events) = mpsc::channel();

        // The stream lives on a thread and a runtime of its own, for `render_one`'s reason: this
        // is called from a synchronous tool, possibly already inside the gRPC server's runtime.
        // What crosses back is every event, in order, and then how the stream ended.
        let stream = std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build()
            else {
                return;
            };
            runtime.block_on(async move {
                let ended = async {
                    let channel = Endpoint::from_shared(address)
                        .map_err(|e| e.to_string())?
                        .connect()
                        .await
                        .map_err(|e| e.to_string())?;
                    let mut inbound = PreviewClient::new(channel)
                        // A plan is `core`'s own, as it is for `render_one`.
                        .max_encoding_message_size(usize::MAX)
                        .preview(UnboundedReceiverStream::new(outgoing))
                        .await
                        .map_err(|status| status.to_string())?
                        .into_inner();
                    while let Some(event) = inbound.message().await.map_err(|s| s.to_string())? {
                        if arrived.send(Ok(event)).is_err() {
                            break;
                        }
                    }
                    Ok::<_, String>("the engine closed the stream".to_string())
                };
                let (Ok(why) | Err(why)) = ended.await;
                let _ = arrived.send(Err(why));
            });
        });

        Ok(Preview {
            engine: self.clone(),
            child,
            said,
            commands: Some(commands),
            events,
            stream: Some(stream),
            sent: 0,
            latest: None,
        })
    }

    /// Spawns the engine in one mode and reads the address it serves on, or reports what it
    /// did instead.
    fn spawn(
        &self,
        mode: &str,
    ) -> Result<(Child, Option<JoinHandle<Vec<u8>>>, String), ProjectError> {
        let mut child = Command::new(&self.binary)
            .arg(&self.manifest)
            .arg(mode)
            // Nothing arrives that way any more, and a child that inherits this process's stdin
            // would compete with the MCP server for it (ADR 0006 §6).
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| self.broke("engine_missing", format!("cannot start the engine: {e}")))?;

        // **stderr is drained on a thread of its own, and that is not tidiness.** A render's
        // stderr carries JUCE's warnings and every plugin's chatter, and the render now happens
        // while this process is blocked on a gRPC call rather than inside one `wait_with_output`
        // that polls both pipes. A full stderr pipe would stop the engine mid-render and the
        // call would never return — the deadlock the stdio path only avoided by never being in
        // this position. A preview, which can run for an hour, needs it more.
        let said = child.stderr.take().map(drain);

        match self.listening(&mut child) {
            Ok(address) => Ok((child, said, address)),
            Err(why) => Err(self.verdict(&mut child, said, why)),
        }
    }

    /// The address the engine printed, or what it did instead.
    ///
    /// The read is bounded because a blocking one is not: an engine that starts, binds nothing
    /// and never exits would hold this thread — and with it the tool API — for good. What the
    /// bound buys is a failure; see [`NAMING_ITS_SOCKET`].
    fn listening(&self, child: &mut Child) -> Result<String, &'static str> {
        let Some(stdout) = child.stdout.take() else {
            return Err("the engine was started without a stdout to name its socket on");
        };
        let (sent, arriving) = mpsc::channel();
        // The rest of stdout is read and dropped so the pipe cannot fill either; the engine
        // writes nothing after the address, and an engine that did would otherwise wedge on it.
        std::thread::spawn(move || {
            let mut lines = BufReader::new(stdout).lines();
            let _ = sent.send(lines.next().and_then(Result::ok));
            for _ in lines {}
        });
        match arriving.recv_timeout(NAMING_ITS_SOCKET) {
            Ok(Some(line)) if !line.trim().is_empty() => Ok(line.trim().to_string()),
            // An empty line, or end of stream with none: the engine's stdout closed without an
            // address, which for a live process means it is on its way out.
            Ok(_) => Err("the engine closed its stdout without naming a socket to serve on"),
            // Not killed here: `verdict` kills whatever is still running, for every path that
            // reaches it, so the one that ends in a hang is not the one somebody forgot.
            Err(_) => Err("the engine started and named no socket within a minute"),
        }
    }

    /// What became of a child that did not answer, as the caller's error.
    ///
    /// **The verdict is the child's exit status, not the transport's.** An engine that exited
    /// non-zero failed and said why on its stderr, which is what a person needs; an engine that
    /// exited 0 without answering is the defect M1 PR 13 found on the other transport — a render
    /// that never happened, reported as a success — and it is unreadable rather than failed,
    /// because nothing about it is a crash to be looked up in a log.
    ///
    /// **One that is still running is given a moment and then killed**, rather than waited for.
    /// Every path into here has already established that this engine is not going to answer, and
    /// `wait` on a live one is an unbounded block in the thread the tool API answers from — the
    /// one thing this file bounds everywhere else.
    ///
    /// The moment is not politeness: an engine whose call has just failed is on its way out and
    /// its **exit code is the verdict**, so killing it the instant the call returns would trade
    /// "exited 3, and here is why" for "killed by a signal" — this file's own answer thrown away
    /// on a race. Five seconds is far past what a failing engine takes to leave, and the loop
    /// stops the moment it has, so a real failure costs a poll or two. What is left after it is
    /// a binary that will not exit at all, which is a hang and is reported instead.
    fn verdict(
        &self,
        child: &mut Child,
        said: Option<JoinHandle<Vec<u8>>>,
        what: &'static str,
    ) -> ProjectError {
        for _ in 0..LEAVING_OF_ITS_OWN_ACCORD {
            match child.try_wait() {
                Ok(Some(_)) => break,
                _ => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        if matches!(child.try_wait(), Ok(None)) {
            let _ = child.kill();
        }
        let Ok(finished) = child.wait() else {
            return self.broke("engine_failed", format!("{what}, and could not be waited for"));
        };
        let rule = if finished.success() { "engine_unreadable" } else { "engine_failed" };
        self.broke(rule, format!("{what}; it {}{}", how(&finished), tail(said)))
    }

    fn broke(&self, rule: &'static str, message: String) -> ProjectError {
        ProjectError { path: self.binary.display().to_string(), rule, message }
    }
}

/// A live preview: the engine process playing, and the stream to it (ADR 0013 §2, §3).
///
/// **Its lifetime is the preview's.** Dropping it closes the stream, the engine stops the
/// transport and exits, and a process that owns one and dies closes it for free — the
/// operating system's guarantee rather than a shutdown path someone maintains (ADR 0013 §2).
pub struct Preview {
    engine: Engine,
    child: Child,
    said: Option<JoinHandle<Vec<u8>>>,
    /// `None` once dropped, which is what ends the stream.
    commands: Option<tokio::sync::mpsc::UnboundedSender<PreviewCommand>>,
    /// Every event, in order, then one `Err` saying how the stream ended.
    events: mpsc::Receiver<Result<PreviewEvent, String>>,
    stream: Option<JoinHandle<()>>,
    /// How many commands have been sent, which is what the answer to the last one carries.
    sent: i32,
    latest: Option<PreviewEvent>,
}

impl Preview {
    /// Sends one command and returns the engine's answer to it.
    ///
    /// **The answer is the event carrying this command's count** (`PreviewEvent.applied`), not
    /// the next event to arrive: while the transport plays it writes events of its own, and one
    /// written while this command was still on the wire arrives after it was sent and before it
    /// was applied. Those update [`Preview::latest`] on the way past.
    ///
    /// Any failure ends the preview. The stream ending, a refusal from the engine and an answer
    /// that never comes all mean it is not going to play any more, and — as for a render — the
    /// report is the engine's exit status and its stderr, never the transport's view of it.
    pub fn send(&mut self, command: PreviewCommand) -> Result<PreviewEvent, ProjectError> {
        let delivered = self.commands.as_ref().is_some_and(|c| c.send(command).is_ok());
        if !delivered {
            return Err(self.ended("the preview stream was already closed"));
        }
        self.sent += 1;
        loop {
            match self.events.recv_timeout(ANSWERING) {
                Ok(Ok(event)) => {
                    self.latest = Some(event);
                    if event.applied >= self.sent {
                        return Ok(event);
                    }
                }
                Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(self.ended("the engine ended the preview instead of answering"));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(self.ended("the engine did not answer the preview within a minute"));
                }
            }
        }
    }

    /// Where the transport last said it was, having read everything it has said since.
    ///
    /// An engine that has ended the stream in the meantime is reported, not hidden: a preview
    /// that stopped existing between two calls is the same operator error the next command would
    /// have met.
    pub fn latest(&mut self) -> Result<Option<PreviewEvent>, ProjectError> {
        loop {
            match self.events.try_recv() {
                Ok(Ok(event)) => self.latest = Some(event),
                Err(mpsc::TryRecvError::Empty) => return Ok(self.latest),
                Ok(Err(_)) | Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(self.ended("the engine ended the preview"));
                }
            }
        }
    }

    fn ended(&mut self, what: &'static str) -> ProjectError {
        self.commands = None;
        let said = self.said.take();
        self.engine.verdict(&mut self.child, said, what)
    }
}

impl Drop for Preview {
    /// Closes the stream, gives the engine the moment it takes to stop and leave, and kills it
    /// if it will not — the bound [`LEAVING_OF_ITS_OWN_ACCORD`] already sets for a render.
    fn drop(&mut self) {
        self.commands = None;
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
        // The process is gone, so its connection is, so the stream's thread has returned.
        if let Some(stream) = self.stream.take() {
            let _ = stream.join();
        }
    }
}

/// One `Render` call, on a runtime and a thread of its own.
///
/// **Both are required rather than tidy.** This is a synchronous call — `render_export` is a
/// tool like any other and answers in one turn — and when the tool API is reached over gRPC it
/// is already running on that server's runtime, where `block_on` panics. A thread of its own is
/// what makes one blocking call legal from either transport, and it costs one thread per render
/// in a process that spawns a whole engine for the same render.
///
/// `ponytail:` a channel per call, built and dropped around it. There is one call, to a server
/// that exits after serving it, so there is nothing a pool could hold on to.
fn render_one(address: &str, plan: &RenderPlan) -> Result<RenderResult, String> {
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
                    RenderClient::new(channel)
                        // The plan is `core`'s own and the engine's limit is off, so nothing
                        // here caps a message: a project large enough to pass gRPC's four
                        // megabytes would otherwise meet a refusal the transport invented
                        // rather than one the validator states (ADR 0007 §6).
                        .max_encoding_message_size(usize::MAX)
                        .render(plan.clone())
                        .await
                        .map(|answer| answer.into_inner())
                        .map_err(|status| status.to_string())
                })
            })
            .join()
            .expect("the thread that calls the engine")
    })
}

/// Reads a pipe to end of stream, on a thread, so it can never fill.
fn drain(mut pipe: ChildStderr) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut said = Vec::new();
        let _ = pipe.read_to_end(&mut said);
        said
    })
}

/// How a process ended, for the message a person reads.
fn how(finished: &std::process::ExitStatus) -> String {
    match finished.code() {
        Some(code) => format!("exited {code}"),
        None => "was killed by a signal".to_string(),
    }
}

/// The engine's last words.
///
/// Trimmed to the tail: a render's stderr carries JUCE's and every plugin's chatter, and what
/// says why it failed is the end of it. Joining the draining thread is also what waits for the
/// pipe to close, so nothing the engine said on its way out is missed.
fn tail(said: Option<JoinHandle<Vec<u8>>>) -> String {
    let said = said.and_then(|thread| thread.join().ok()).unwrap_or_default();
    let text = String::from_utf8_lossy(&said);
    let tail: Vec<&str> = text.lines().rev().take(3).collect();
    if tail.is_empty() {
        return " and said nothing".to_string();
    }
    format!(": {}", tail.into_iter().rev().collect::<Vec<_>>().join("; "))
}
