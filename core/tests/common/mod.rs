//! Shared by the integration tests: the build manifest they validate against.
//!
//! Not a test target of its own — `tests/common/mod.rs` is a module, not a suite — so this
//! file holds only what more than one suite needs and cannot drift between them.
//!
//! `dead_code` off for the module: it is compiled into every suite that names it, and a helper
//! one suite does not happen to use is the normal case here, not a mistake.
#![allow(dead_code)]

use escribass_core::Manifest;
use escribass_proto::render::render_server::RenderServer;
use escribass_proto::render::{RenderPlan, RenderResult};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The manifest fixture (`/tests/fixtures/manifest.json`, `tests/AGENTS.md`).
///
/// The real manifest is generated at build time and never committed (ADR 0010 §4), so the
/// tests read a **committed subset of a real one**: the plugin id and the parameter ids the
/// fixtures use, taken from `escribass_engine --scan` of a built engine, with the machine's
/// plugin paths dropped. It is a fixture, not a build artefact — which is why it can be
/// committed and the manifest cannot.
///
/// What keeps it honest is that it is a *subset*: PR 11's render suite, which has a built
/// engine, asserts every id in here appears in the manifest that build wrote. That is the
/// guard against it drifting back into the invented ids this fixture replaced.
///
/// Reading it rather than building one in Rust is deliberate: the parse path is what the
/// binaries use, so a manifest this crate cannot read fails here rather than at startup.
pub fn manifest() -> Arc<Manifest> {
    Arc::new(Manifest::read(MANIFEST).expect("the manifest fixture is readable"))
}

/// Where it lives, for the tests that need the file rather than the parsed value.
pub const MANIFEST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/manifest.json");

/// An engine that is a shell script: it records the arguments it was given and then does
/// whatever `body` says. Written into `dir`, which must exist.
///
/// **A script cannot speak HTTP/2, and does not need to.** What these suites test is `core`'s
/// side of the boundary (ADR 0013 §3): the address it reads from the child's stdout, the call
/// it then makes, the plan that crosses, and which failures are a refusal and which are an
/// operator's. So the `Render` server is a real one — [`fake_server`], in this process — and
/// the script's whole job is to be the process `core` spawns, names a socket to, and waits on.
/// That is what puts an exit code, a signal, a silent stdout and an address nothing is
/// listening on all within reach of a test, which a real engine would not be.
///
/// Shared by `engine.rs`, which drives it through a `Session`, and by `mcp.rs`, which drives it
/// through a real server process over real pipes (M1 PR 13) — and one script rather than two,
/// because the second copy is the one that stops matching what `core` actually spawns.
///
/// `#[cfg(unix)]` because this repository claims Linux x86-64 and nothing else (ADR 0009 §1),
/// so a shell is a fair assumption where a golden already is.
#[cfg(unix)]
pub fn fake_engine(dir: &Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("fake-engine");
    let script = format!(
        // `--probe` is `startable` below asking whether this file can be executed yet, and it
        // answers before anything with a side effect — a probe that wrote `argument` would make
        // "a dry run started no process" pass for the wrong reason.
        "#!/bin/sh\n[ \"$1\" = --probe ] && exit 0\necho \"$1\" > '{argument}'\necho \"$2\" > '{mode}'\n{body}\n",
        argument = dir.join("argument").display(),
        mode = dir.join("mode").display(),
    );
    std::fs::write(&path, script).expect("the fake engine is writable");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    startable(&path);
    path
}

/// Waits until the script just written will actually start.
///
/// **`ETXTBSY`, and it is this suite's own doing** (M2 PR 9; measured at two failures in forty
/// runs under load, reported as `engine_missing: Text file busy`). A `#[test]` runs on a thread,
/// so while one test writes this file another is spawning an engine — and a child forked between
/// our `open` and our `close` inherits the writable descriptor and holds it until it `exec`s,
/// which is precisely what `exec` refuses with "Text file busy". The window is microseconds and
/// closes on its own.
///
/// So the harness proves the program it just wrote will start, rather than `core` retrying an
/// error it cannot meet in production: `core` never writes the engine it runs. The loop is
/// bounded and decides only whether this suite can proceed.
#[cfg(unix)]
fn startable(path: &Path) {
    // ETXTBSY. Written as a number because naming it would mean a `libc` dependency for one
    // constant, and it has been 26 on Linux for as long as there has been a Linux.
    const TEXT_FILE_BUSY: i32 = 26;
    for _ in 0..1_000 {
        match std::process::Command::new(path)
            .arg("--probe")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
        {
            Ok(_) => return,
            Err(e) if e.raw_os_error() == Some(TEXT_FILE_BUSY) => {
                std::thread::sleep(std::time::Duration::from_millis(1))
            }
            Err(e) => panic!("the fake engine at {} will not start: {e}", path.display()),
        }
    }
    panic!("{} was still Text file busy after a thousand tries", path.display());
}

/// A `Render` server on a Unix socket in `dir`, answering every call the same way.
///
/// It outlives the call and the suite: the runtime runs on a detached thread and the socket
/// dies with the scratch directory. A real engine serves one call and exits (ADR 0008 §2), and
/// nothing here pretends otherwise — the process `core` spawns and waits on is the script, and
/// this is only what answers on the address that script names.
///
/// `answer` is what the service returns: a `RenderResult`, or a message that becomes a gRPC
/// `Status`, which is how a real engine reports a render it could not do (ADR 0008 §1).
#[cfg(unix)]
pub fn fake_server(dir: &Path, answer: Result<RenderResult, String>) -> Served {
    let socket = dir.join("render.sock");
    let asked = Arc::new(Mutex::new(None));
    let service = Answering { answer, asked: asked.clone() };
    let bound = socket.clone();
    let (listening, ready) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime for the fake engine");
        runtime.block_on(async move {
            let socket = tokio::net::UnixListener::bind(&bound).expect("a socket to serve on");
            // Sent after the bind and never before, which is the same discipline the engine
            // itself keeps: what tells the caller where to dial also tells it that it may.
            listening.send(()).expect("the test is still waiting");
            tonic::transport::Server::builder()
                .add_service(RenderServer::new(service))
                .serve_with_incoming(
                    tonic::codegen::tokio_stream::wrappers::UnixListenerStream::new(socket),
                )
                .await
                .expect("the fake engine serves");
        });
    });
    ready.recv().expect("the fake engine binds its socket");
    Served { socket, asked }
}

/// A running [`fake_server`]: where it listens, and what it was asked.
#[cfg(unix)]
pub struct Served {
    pub socket: PathBuf,
    asked: Arc<Mutex<Option<RenderPlan>>>,
}

#[cfg(unix)]
impl Served {
    /// What `core` sent, as the generated server decoded it — not as a reader of a file guessed.
    pub fn plan(&self) -> RenderPlan {
        self.asked.lock().expect("the fake engine has not panicked").clone().expect(
            "the fake engine was called",
        )
    }

    /// Whether it was called at all, which is what a dry run must not do.
    pub fn called(&self) -> bool {
        self.asked.lock().expect("the fake engine has not panicked").is_some()
    }

    /// The line a fake engine prints to send `core` here.
    pub fn address(&self) -> String {
        format!("echo unix:{}", self.socket.display())
    }
}

#[cfg(unix)]
struct Answering {
    answer: Result<RenderResult, String>,
    asked: Arc<Mutex<Option<RenderPlan>>>,
}

#[cfg(unix)]
#[tonic::async_trait]
impl escribass_proto::render::render_server::Render for Answering {
    async fn render(
        &self,
        request: tonic::Request<RenderPlan>,
    ) -> Result<tonic::Response<RenderResult>, tonic::Status> {
        *self.asked.lock().expect("nothing else panicked") = Some(request.into_inner());
        match &self.answer {
            Ok(result) => Ok(tonic::Response::new(result.clone())),
            Err(why) => Err(tonic::Status::internal(why.clone())),
        }
    }
}

/// How a [`fake_preview`] behaves, beyond answering each command the way the engine does.
#[cfg(unix)]
#[derive(Clone, Copy, Default)]
pub struct Playing {
    /// Before each answer, write an event of the transport's own at this tick — carrying the
    /// count of the *previous* command, as a real one written while a command was on the wire
    /// would. What `core` must not mistake for the answer (`PreviewEvent.applied`).
    pub chatter: Option<i32>,
    /// After each answer, report the transport having moved on to this tick, as a playing one
    /// does between commands.
    pub moves_to: Option<i32>,
    /// End the stream with a status instead of applying the command with this count, which is
    /// how a real engine refuses a plan it cannot build (ADR 0013 §2).
    pub refuses: Option<i32>,
}

/// A `Preview` server on a Unix socket in `dir`, speaking the engine's protocol: one event per
/// command, carrying its count, and an empty file called `closed` once the stream has ended —
/// which is what a script standing in for the engine waits on before it leaves, so "the
/// process lives as long as its stream" is something a test can see (ADR 0013 §3).
///
/// A **model** of the engine's side, not the engine, for [`fake_engine`]'s reason: what is
/// tested with it is `core`'s half of the boundary. The engine's half needs an audio device,
/// which is `tests/renders.rs`'s ignored test and the engine job's no-device step.
#[cfg(unix)]
pub fn fake_preview(dir: &Path, behaviour: Playing) -> Heard {
    use escribass_proto::render::preview_command::Command;
    use escribass_proto::render::preview_server::PreviewServer;
    use escribass_proto::render::{PreviewCommand, PreviewEvent, PreviewState};

    let socket = dir.join("preview.sock");
    let closed = dir.join("closed");
    let heard = Arc::new(Mutex::new(Vec::new()));
    let service = Model { behaviour, heard: heard.clone(), closed };
    let bound = socket.clone();
    let (listening, ready) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime for the fake preview");
        runtime.block_on(async move {
            let socket = tokio::net::UnixListener::bind(&bound).expect("a socket to serve on");
            listening.send(()).expect("the test is still waiting");
            tonic::transport::Server::builder()
                .add_service(PreviewServer::new(service))
                .serve_with_incoming(
                    tonic::codegen::tokio_stream::wrappers::UnixListenerStream::new(socket),
                )
                .await
                .expect("the fake preview serves");
        });
    });
    ready.recv().expect("the fake preview binds its socket");
    return Heard { socket, heard };

    struct Model {
        behaviour: Playing,
        heard: Arc<Mutex<Vec<PreviewCommand>>>,
        closed: PathBuf,
    }

    #[tonic::async_trait]
    impl escribass_proto::render::preview_server::Preview for Model {
        type PreviewStream = std::pin::Pin<
            Box<
                dyn tonic::codegen::tokio_stream::Stream<Item = Result<PreviewEvent, tonic::Status>>
                    + Send,
            >,
        >;

        async fn preview(
            &self,
            request: tonic::Request<tonic::Streaming<PreviewCommand>>,
        ) -> Result<tonic::Response<Self::PreviewStream>, tonic::Status> {
            let mut inbound = request.into_inner();
            let (events, outgoing) = tokio::sync::mpsc::unbounded_channel();
            let (behaviour, heard, closed) = (self.behaviour, self.heard.clone(), self.closed.clone());
            tokio::spawn(async move {
                let (mut applied, mut tick, mut state) = (0, 0, PreviewState::Stopped);
                while let Ok(Some(command)) = inbound.message().await {
                    heard.lock().expect("nothing else panicked").push(command.clone());
                    let event = |tick, state: PreviewState, applied| PreviewEvent {
                        tick,
                        state: state as i32,
                        applied,
                    };
                    if let Some(noise) = behaviour.chatter {
                        let _ = events.send(Ok(event(noise, state, applied)));
                    }
                    if behaviour.refuses == Some(applied + 1) {
                        let _ = events.send(Err(tonic::Status::invalid_argument("a plan it cannot build")));
                        break;
                    }
                    match command.command {
                        Some(Command::Play(play)) => (tick, state) = (play.start_tick, PreviewState::Playing),
                        Some(Command::Seek(seek)) => tick = seek.tick,
                        Some(Command::Stop(_)) => state = PreviewState::Stopped,
                        Some(Command::Loop(_)) | None => {}
                    }
                    applied += 1;
                    let _ = events.send(Ok(event(tick, state, applied)));
                    if let Some(moved) = behaviour.moves_to {
                        tick = moved;
                        let _ = events.send(Ok(event(tick, state, applied)));
                    }
                }
                drop(events);
                std::fs::write(&closed, b"").expect("the scratch directory is writable");
            });
            Ok(tonic::Response::new(Box::pin(
                tonic::codegen::tokio_stream::wrappers::UnboundedReceiverStream::new(outgoing),
            )))
        }
    }
}

/// A running [`fake_preview`]: where it listens, and every command it was sent, in order.
#[cfg(unix)]
pub struct Heard {
    pub socket: PathBuf,
    heard: Arc<Mutex<Vec<escribass_proto::render::PreviewCommand>>>,
}

#[cfg(unix)]
impl Heard {
    pub fn commands(&self) -> Vec<escribass_proto::render::PreviewCommand> {
        self.heard.lock().expect("the fake preview has not panicked").clone()
    }
}
