//! The MCP server (docs/specs.md §18.2).
//!
//! ```text
//! escribass-mcp --manifest <m.json> <project.escri>           serve an existing project
//! escribass-mcp --manifest <m.json> --create <project.escri>  create one, then serve it
//! ```
//!
//! Flags:
//!
//! ```text
//! --manifest <path>        **required**, and no default: what this build can host
//!                          (ADR 0010 §4). Without it `plugin_unknown` and `param_unknown`
//!                          would have a silent skip arm, so this process refuses to start
//! --author human|model     provenance on everything this process writes, the project it
//!                          creates included (default: model)
//! --seed-ids <ms>:<n>      deterministic ids instead of ULIDs from the clock and entropy
//! --fixed-clock <ms>       a fixed instant instead of the system clock
//! ```
//!
//! The last two exist so a scripted session can be compared byte for byte through a real
//! server process, which is what M0.4 needs and what §11 means by determinism. They are the
//! only way `core` is allowed to be deterministic: it never reads the wall clock or takes
//! entropy on its own (CLAUDE.md #3).
//!
//! **The project is a launch argument, not a tool.** An MCP stdio process is not a session,
//! and two writers on one `.escri` would race the three renames ADR 0004's commit depends on
//! (ADR 0006 §5).
//!
//! Nothing here writes to stdout but the transport. A stray `println!` corrupts the JSON-RPC
//! stream; diagnostics go to stderr.

use escribass_core::{
    new_song, Clock, FixedClock, IdSource, Manifest, Project, SeededIds, Session, SongTools,
    SystemClock, UlidSource,
};
use escribass_schema::song::Author;
use rmcp::ServiceExt;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("escribass-mcp: {message}");
            ExitCode::FAILURE
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn run() -> Result<(), String> {
    let options = Options::parse(std::env::args().skip(1))?;

    // `ponytail:` two boxes rather than a generic session, so both binaries can build one from
    // flags without monomorphising the whole server per clock. Swap if a profile ever cares.
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

    let project = if options.create {
        let song = new_song(&mut *ids, &*clock, options.author);
        Project::create(&options.project, &song, &mut *ids, &*clock, options.author, manifest)
            .map_err(|e| format!("cannot create {}: {e}", options.project.display()))?
    } else {
        Project::open(&options.project, manifest)
            .map_err(|e| format!("cannot open {}: {e}", options.project.display()))?
    };

    let session = Session::new(project, ids, clock, options.author);
    let server = SongTools::new(session)?;

    // rmcp answers every protocol version it knows, so a client on the current revision and
    // one still sending `initialize` both work against this process.
    let running = server
        .serve(rmcp::transport::io::stdio())
        .await
        .map_err(|e| format!("the MCP transport failed to start: {e}"))?;
    running.waiting().await.map_err(|e| format!("the MCP server stopped: {e}"))?;
    Ok(())
}

struct Options {
    project: PathBuf,
    manifest: PathBuf,
    create: bool,
    author: Author,
    seed: Option<(i64, u64)>,
    fixed_clock: Option<i64>,
}

impl Options {
    fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut project = None;
        let mut manifest = None;
        let mut create = false;
        let mut author = Author::Model;
        let mut seed = None;
        let mut fixed_clock = None;

        let mut arguments = arguments.peekable();
        while let Some(argument) = arguments.next() {
            let mut value = |name: &str| {
                arguments.next().ok_or_else(|| format!("{name} needs a value"))
            };
            match argument.as_str() {
                "--create" => create = true,
                "--manifest" => manifest = Some(PathBuf::from(value("--manifest")?)),
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
            create,
            author,
            seed,
            fixed_clock,
        })
    }
}

const USAGE: &str = "usage: escribass-mcp --manifest <manifest.json> [--create] [--author human|model] \
                     [--seed-ids <ms>:<n>] [--fixed-clock <ms>] <project.escri>";
