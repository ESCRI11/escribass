//! The `SongTools` gRPC server (§5).
//!
//! ```text
//! escribass-grpc --manifest <m.json> <project.escri>            serve on 127.0.0.1:50051
//! escribass-grpc --manifest <m.json> --create <project.escri>   create one, then serve it
//! ```
//!
//! Flags:
//!
//! ```text
//! --manifest <path>        **required**, and no default: what this build can host
//!                          (ADR 0010 §4). Without it `plugin_unknown` and `param_unknown`
//!                          would have a silent skip arm, so this process refuses to start
//! --engine <path>          the engine binary `render_export` renders with. Told, never
//!                          searched, for the reason `--manifest` is (ADR 0008 §2); optional
//!                          because only that one call needs it, and without it the call is
//!                          refused as an operator error rather than skipped
//! --listen <addr>          default 127.0.0.1:50051
//! --author human|model     provenance on everything this process writes, the project it
//!                          creates included (default: human)
//! --seed-ids <ms>:<n>      deterministic ids instead of ULIDs from the clock and entropy
//! --fixed-clock <ms>       a fixed instant instead of the system clock
//! ```
//!
//! **This binary is a harness, not the product.** §3 puts the tool API inside the Tauri host at
//! M2, where `app` and `ai` reach it in-process and over the socket respectively; the reusable
//! part is [`escribass_core::grpc::Server`], and this is the ten lines that let a person or a
//! test drive it today.
//!
//! Loopback by default and no TLS: the service edits local files with no authentication of any
//! kind, so the only safe listener is one nothing else can reach. `ponytail:` when M2 needs a
//! non-loopback address, it needs authentication in the same change, not a flag.

use escribass_core::grpc::Server;
use escribass_core::{
    new_song, Clock, FixedClock, IdSource, Manifest, Project, ProjectLock, SeededIds, Session,
    SystemClock, UlidSource,
};
use escribass_schema::song::Author;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("escribass-grpc: {message}");
            ExitCode::FAILURE
        }
    }
}

#[tokio::main]
async fn run() -> Result<(), String> {
    let options = Options::parse(std::env::args().skip(1))?;

    let mut ids: Box<dyn IdSource + Send> = match options.seed {
        Some((ms, seed)) => Box::new(SeededIds::new(ms, seed)),
        None => Box::new(UlidSource::new(SystemClock)),
    };
    let clock: Box<dyn Clock + Send> = match options.fixed_clock {
        Some(ms) => Box::new(FixedClock(ms)),
        None => Box::new(SystemClock),
    };

    // Read before the project, because it is what decides whether the project is valid at
    // all: the validator resolves plugin ids and parameters against it, and `open` compares
    // it with what the project pinned (ADR 0010 §3, §4).
    let manifest = Arc::new(Manifest::read(&options.manifest).map_err(|e| e.to_string())?);

    // One writer per `.escri`, enforced rather than assumed (ADR 0012 §3). Taken before an
    // open, because reading a project another process is committing to is the race this
    // closes; taken after a create, because the directory has to exist to hold the file.
    // `_lock` and not `_`: the binding is what holds it open for the life of the process, and
    // the file is removed when this function returns.
    let (project, _lock) = if options.create {
        let song = new_song(&mut *ids, &*clock, options.author);
        let project =
            Project::create(&options.project, &song, &mut *ids, &*clock, options.author, manifest)
                .map_err(|e| format!("cannot create {}: {e}", options.project.display()))?;
        let lock = ProjectLock::take(&options.project).map_err(|e| e.to_string())?;
        (project, lock)
    } else {
        let lock = ProjectLock::take(&options.project).map_err(|e| e.to_string())?;
        let project = Project::open(&options.project, manifest)
            .map_err(|e| format!("cannot open {}: {e}", options.project.display()))?;
        (project, lock)
    };

    let mut session = Session::new(project, ids, clock, options.author);
    // The engine is told or absent, never searched (ADR 0008 §2, `core/src/engine.rs`). It is
    // handed the same manifest the validator resolved against, so what the engine hosts and
    // what `core` believes it hosts cannot be two files.
    if let Some(engine) = options.engine {
        session.set_engine(escribass_core::Engine::new(engine, options.manifest));
    }
    let server = Server::new(session);
    // The address this process was asked for, printed so a caller does not have to re-parse
    // its own flags. Not a readiness signal — `serve` binds below. stdout carries nothing
    // else; anything this process has to say goes to stderr.
    println!("{}", options.listen);

    tonic::transport::Server::builder()
        .add_service(server.into_service())
        .serve(options.listen)
        .await
        .map_err(|e| format!("the server stopped: {e}"))
}

struct Options {
    project: PathBuf,
    manifest: PathBuf,
    engine: Option<PathBuf>,
    listen: SocketAddr,
    create: bool,
    author: Author,
    seed: Option<(i64, u64)>,
    fixed_clock: Option<i64>,
}

impl Options {
    fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut project = None;
        let mut manifest = None;
        let mut engine = None;
        let mut listen: SocketAddr = "127.0.0.1:50051".parse().expect("a literal address");
        let mut create = false;
        let mut author = Author::Human;
        let mut seed = None;
        let mut fixed_clock = None;

        let mut arguments = arguments.peekable();
        while let Some(argument) = arguments.next() {
            let mut value =
                |name: &str| arguments.next().ok_or_else(|| format!("{name} needs a value"));
            match argument.as_str() {
                "--create" => create = true,
                "--manifest" => manifest = Some(PathBuf::from(value("--manifest")?)),
                "--engine" => engine = Some(PathBuf::from(value("--engine")?)),
                "--listen" => {
                    let text = value("--listen")?;
                    listen = text.parse().map_err(|_| format!("`{text}` is not an address"))?;
                }
                "--author" => {
                    author = match value("--author")?.as_str() {
                        "human" => Author::Human,
                        "model" => Author::Model,
                        other => return Err(format!("--author is human or model, not `{other}`")),
                    }
                }
                "--seed-ids" => {
                    let text = value("--seed-ids")?;
                    let (ms, counter) = text
                        .split_once(':')
                        .ok_or_else(|| format!("--seed-ids is <ms>:<n>, not `{text}`"))?;
                    seed = Some((
                        ms.parse().map_err(|_| format!("`{ms}` is not milliseconds"))?,
                        counter.parse().map_err(|_| format!("`{counter}` is not a number"))?,
                    ));
                }
                "--fixed-clock" => {
                    let text = value("--fixed-clock")?;
                    fixed_clock =
                        Some(text.parse().map_err(|_| format!("`{text}` is not milliseconds"))?);
                }
                "--help" | "-h" => return Err(USAGE.to_string()),
                flag if flag.starts_with('-') => return Err(format!("unknown flag `{flag}`")),
                path => project = Some(PathBuf::from(path)),
            }
        }

        Ok(Options {
            project: project.ok_or_else(|| format!("no project directory given\n{USAGE}"))?,
            // No default and no fallback. An absent manifest would give `plugin_unknown` and
            // `param_unknown` a silent skip arm, which is the quiet failure M0.4 exists to
            // prevent, so this process does not start without one (ADR 0010 §4).
            manifest: manifest.ok_or_else(|| format!(
                "--manifest [manifest_missing]: this build's manifest says what plugins and \
                 parameters exist, and the validator has no answer without it (ADR 0010 §4). \
                 It is written by `cmake --build engine/build --target manifest`\n{USAGE}"))?,
            engine,
            listen,
            create,
            author,
            seed,
            fixed_clock,
        })
    }
}

const USAGE: &str = "usage: escribass-grpc --manifest <manifest.json> [--engine <path>] [--create] \
                     [--listen <addr>] [--author human|model] [--seed-ids <ms>:<n>] \
                     [--fixed-clock <ms>] <project.escri>";
