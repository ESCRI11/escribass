//! The desktop host (docs/specs.md §3 tier 1, §9; ADR 0012).
//!
//! ```text
//! escribass-app --manifest <manifest.json> [--engine <escribass_engine>] <project.escri>
//! ```
//!
//! One operating-system process: this host, `core` linked into it as a library, and a webview.
//! §3's gRPC is `app` ↔ `ai` and `app` ↔ `engine`, and the webview is neither. `app` opens no
//! network port, because a desktop application that binds a socket to talk to itself is a
//! listening service on a user's machine (ADR 0012 §1). What it does dial, from M2 PR 10, is the
//! engine: `core`'s session spawns one per export and one live one for a preview, each naming a
//! Unix socket of its own (ADR 0013 §3) — so the supervision §3 gives `app` is the session's, and
//! a preview ends when this process does, because its stream does.
//!
//! What the webview can reach is [`tool`], and that is the whole of what it can reach *of the
//! model*. One command carrying a tool name and its arguments, dispatched into
//! `escribass_core::call` — the same function the MCP server dispatches into, so §5's "used by
//! the UI and the AI identically" is one dispatch and not two that are free to drift.
//!
//! Beside it, [`manifest`], which answers a question about the running build rather than about
//! the song: what plugins it hosts and what parameters each declares. §9's seventh view is a
//! form over that map (ADR 0014 §1) and there is no tool that reads it, so this is the second
//! command — see its own note for why it is not one.
//!
//! And, from M3 PR 5, [`assistant`]: whether the AI sidecar this window started is still
//! running (ADR 0020 §5). A third command of the same kind as the second — a question about
//! this process, not about the song — and it is not a route to the model either: nothing can
//! be *asked* of `ai` from here yet, because the loop and the panel are PR 8's and PR 9's.
//! What the window shows is a dot, and what the dot is made of is the child's **exit status,
//! read**: a sidecar that dies says so, with its status and the tail of its stderr, and the
//! window goes on editing — the session, the engine and every view are untouched by its
//! absence.
//!
//! Flags:
//!
//! ```text
//! --manifest <path>        **required**, and no default: what this build can host
//!                          (ADR 0010 §4). Without it `plugin_unknown` and `param_unknown`
//!                          would have a silent skip arm, so this process refuses to start
//! --engine <path>          the engine binary `render_preview` and `render_export` start, told
//!                          and never searched for, as the two servers are (ADR 0008 §2). Absent,
//!                          the window still edits, and pressing play says `engine_unset`. An
//!                          engine plays the plugins at the paths this manifest names, so it
//!                          wants the manifest a build wrote, not the test fixture
//! --ai <command>           the command that starts the AI sidecar — on a build tree
//!                          `"uv run --project ai escribass-ai --transcript <t>"` — told and
//!                          never searched for, as the engine is (ADR 0020 §4). Absent, the
//!                          window still edits and the dot says there is none. The key is
//!                          **not** here: `OPENROUTER_API_KEY` is inherited from this
//!                          process's environment, because `ps` shows flags (U3)
//! ```
//!
//! No `--create`, and no File · Open yet. The project is a launch argument, as it is for both
//! server binaries; a menu that opens a second one wants a session per project directory and a
//! window per session, and ADR 0012 §3 has decided what that means without this step having to
//! build it.

// Tauri's own recommendation, and it matters on Windows, where the process would otherwise
// come with a console window attached. Debug builds keep the console so `eprintln!` lands
// somewhere a developer can see it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use escribass_core::call::call;
use escribass_core::{
    Assistant, Engine, Health, Manifest, Project, ProjectLock, Session, Sidecar, SystemClock,
    UlidSource,
};
use escribass_schema::song::Author;
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};

/// The `.escri` lock, held where the exit path can let go of it (ADR 0012 §3).
///
/// **Not** managed Tauri state, and the reason is the one thing about this file worth knowing.
/// Managed state lives behind an `Arc` the framework keeps handles to, so a guard put there is
/// not dropped when the window closes — it is dropped whenever the last `AppHandle` goes,
/// which is not a moment this process controls and was, when measured, never. A window closed
/// normally then left a lock its owner no longer held, and the project refused to open until
/// someone deleted a file by hand, *for having been closed properly*. That is the one failure
/// a lock which is never broken automatically cannot afford. So the guard is held here, in an
/// `Option` [`run`] empties on the way out.
type Held = Arc<Mutex<Option<ProjectLock>>>;

/// The sidecar this window started, or why it has none (ADR 0020 §4).
///
/// Held here for the same reason the lock is not managed state — [`run`] empties it on the way
/// out, and dropping a [`Sidecar`] closes the sidecar's stdin and waits for it to leave, which
/// is a thing that must happen while this process still exists. A window that ended without it
/// would leave a Python process holding a socket nobody will ever dial.
struct Ai {
    sidecar: Mutex<Option<Sidecar>>,
    /// Set when `--ai` was given and the sidecar would not start: the window opens anyway, as
    /// it does without an engine, and the dot says what happened rather than nothing.
    refused: Option<String>,
}

/// The webview's only route to the model.
///
/// A refusal is a *result* and comes back through `Ok`: `valid: false` and every rule it broke,
/// which is what §9's approve-then-apply flow draws. `Err` is a call that was never answered —
/// an unknown tool, arguments that would not decode, or an operator error (ADR 0006 §2).
///
/// What crosses is the answer's structured half. It is the same *document* as the canonical
/// text — parsed from it, for a song — with its keys in `serde_json`'s order rather than the
/// model's, which is all a generated deserializer on the other side needs and all ADR 0002
/// §11 asks of a wire form. `ponytail:` the text half stays behind because nothing in the
/// window reads a song as text yet; a diff view is the first thing that will, and it can have
/// both then.
#[tauri::command]
fn tool(
    session: tauri::State<'_, Mutex<Session>>,
    name: String,
    args: Map<String, Value>,
) -> Result<Value, String> {
    // One project, one writer (ADR 0001 §2). The webview is single-threaded and Tauri answers
    // commands on a pool, so the `Mutex` is what keeps two in-flight calls from interleaving.
    //
    // Recovered rather than propagated, for the reason the MCP server recovers it: nothing in
    // `call` can leave a session half-mutated, so one panicking call must not make every later
    // call fail.
    let mut session = session.lock().unwrap_or_else(PoisonError::into_inner);
    call(&mut session, &name, &args).map(|answer| answer.structured).map_err(|e| e.message)
}

/// What this build can host — the map §9's seventh view is a form over (ADR 0014 §1).
///
/// A **second** command, and the only other thing the webview can reach. ADR 0012 §1's rule is
/// that the webview's one route to *the model* is [`tool`], and this is not the model: it is
/// what the running build says about itself, it takes no arguments, it never changes while the
/// process lives, and no tool answers it — `IMPLEMENTED` is the tool surface the AI shares
/// (ADR 0006 §1), and putting a build fact on it would hand the AI a plugin catalogue this
/// pull request has no business deciding it should have.
///
/// What crosses is the `Manifest` `core` **parsed**, re-serialised — not the file. That is the
/// point rather than an accident: the editor's rows are then exactly the keys
/// `param_unknown` resolves a `set_param` against (`core/src/validate.rs`), so a control the
/// form draws is a control the validator will accept, and the plugin paths the file carries
/// for the engine's benefit never reach a window.
#[tauri::command]
fn manifest(held: tauri::State<'_, Arc<Manifest>>) -> Result<Value, String> {
    serde_json::to_value(held.inner().as_ref()).map_err(|e| e.to_string())
}

/// The AI sidecar this window started, and whether it is still there (ADR 0020 §5).
///
/// **The dot is the child's exit status, read** — and nothing more. There is no "healthy",
/// because a hosted model verifies nothing and a readout that counted it would be M3 trap 15
/// in a second place; there is no "busy", because nothing can ask it for a turn yet. Three
/// states: none was started, one is running, or one is gone and here is how.
///
/// Polled rather than pushed, as the transport is: one question at a time, asked again when
/// the last has landed. An event would need a channel from a thread watching a child, which is
/// a second mechanism for a fact `try_wait` already states.
///
/// `State<'_, Arc<Ai>>` and not `State<'_, Ai>`: Tauri resolves managed state by the type it
/// was handed, and `manage` below is handed the `Arc` the exit path also holds. The two must
/// name the same type or this command fails at run time in a window while compiling perfectly
/// — which is exactly how it was written the first time. [`manifest`] above has the pair
/// right, and is the precedent.
#[tauri::command]
fn assistant(ai: tauri::State<'_, Arc<Ai>>) -> Value {
    let mut held = ai.sidecar.lock().unwrap_or_else(PoisonError::into_inner);
    dot(held.as_mut().map(Sidecar::health), ai.refused.as_deref())
}

/// The two fields the webview reads (`app/src/tool.ts`, `Assisted`), from what `core` said.
///
/// Split out from the command because a command cannot be called without a window, and these
/// three strings are a contract with TypeScript: a `state` renamed here is a dot with no colour
/// there, and nothing would say so.
fn dot(health: Option<Health>, refused: Option<&str>) -> Value {
    let (state, said) = match health {
        None => ("unset", refused.unwrap_or_default().to_string()),
        Some(Health::Running) => ("running", String::new()),
        Some(Health::Gone(said)) => ("gone", said),
    };
    serde_json::json!({ "state": state, "said": said })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => u8::try_from(code).map(ExitCode::from).unwrap_or(ExitCode::FAILURE),
        Err(message) => {
            eprintln!("escribass-app: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<i32, String> {
    let options = Options::parse(std::env::args().skip(1))?;

    // Read before the project, because it is what decides whether the project is valid at all:
    // the validator resolves plugin ids and parameters against it, and `open` compares it with
    // what the project pinned (ADR 0010 §3, §4).
    //
    // `hosts` and not `manifest`, which is what it wants to be called: `generate_handler!`
    // expands to a path with the command's own name in it, resolved in *this* scope, so a local
    // binding called `manifest` shadows the [`manifest`] command and the macro fails inside its
    // own expansion. Renaming the binding is the fix that keeps the command's name, which is
    // what the webview calls it by.
    let hosts = Arc::new(Manifest::read(&options.manifest).map_err(|e| e.to_string())?);

    // The lock first, then the read: a project another process is committing to is exactly
    // what this refuses to open (ADR 0012 §3).
    let lock = ProjectLock::take(&options.project).map_err(|e| e.to_string())?;
    // A lock left by a process that is gone is replaced rather than refused, and the window
    // says so on stderr rather than silently: the crash a person is being told about is
    // usually an MCP client's, which ends by signal and runs no `Drop` (ADR 0020 §5).
    // `ponytail:` stderr, not a banner in the window. A place for it in the UI is the status
    // bar, and it costs a view decision this pull request has no reason to take.
    if let Some(said) = lock.replaced() {
        eprintln!("{said}");
    }
    let held: Held = Arc::new(Mutex::new(Some(lock)));
    let project = Project::open(&options.project, Arc::clone(&hosts))
        .map_err(|e| format!("cannot open {}: {e}", options.project.display()))?;

    // One session per project directory, built by the host as a library call — which is ADR
    // 0012 §3's amendment to ADR 0006 §5: the rule that was protecting the *tool surface* is
    // unchanged, and the constructor was never a tool. A window's edits are a person's,
    // whatever else is also editing this project.
    let mut session = Session::new(
        project,
        Box::new(UlidSource::new(SystemClock)),
        Box::new(SystemClock),
        Author::Human,
    );
    if let Some(engine) = &options.engine {
        session.set_engine(Engine::new(engine, &options.manifest));
    }

    // The sidecar, started as the engine is and by the same code in `core` (ADR 0020 §4). A
    // failure to start is **not** a failure to open: the window edits, renders and previews
    // without an assistant exactly as it does without an engine, and what a person gets instead
    // is a dot that says why. Nothing here is asked of it — the loop is PR 8's — so this
    // pull request's whole claim is that the process is started, watched, and let go of.
    let mut refused = None;
    let started = options.ai.as_ref().and_then(|command| {
        match Assistant::new(command.clone()).start() {
            Ok(sidecar) => Some(sidecar),
            Err(e) => {
                eprintln!("escribass-app: the assistant did not start: {}", e.message);
                refused = Some(e.message);
                None
            }
        }
    });
    let ai = Arc::new(Ai { sidecar: Mutex::new(started), refused });

    let title = format!(
        "{} — escribass",
        options
            .project
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| options.project.display().to_string())
    );

    let app = tauri::Builder::default()
        .manage(Mutex::new(session))
        .manage(hosts)
        .manage(Arc::clone(&ai))
        .invoke_handler(tauri::generate_handler![tool, manifest, assistant])
        .setup(move |app| {
            use tauri::Manager;
            // The project this window is looking at, named where a person can see it. The
            // status bar deliberately shows no §11 number (docs/plan.md, M2 trap 4).
            if let Some(window) = app.get_webview_window("main") {
                window.set_title(&title)?;
            }
            Ok(())
        })
        .build(tauri::generate_context!());
    // Not `?`: the sidecar is already running by now, and a window that will not open is the
    // one path where nobody is watching the dot. Stopping it here reads its status and says
    // so, which is the rule ADR 0020 §5 sets for every path rather than for the happy one.
    let app = match app {
        Ok(app) => app,
        Err(e) => {
            stop(&ai);
            take(&held);
            return Err(format!("the window could not start: {e}"));
        }
    };

    // `run_return` and not `run`, for the same reason the lock is not managed state:
    // `Builder::run` gives the event loop to `tao`, which ends the process with
    // `std::process::exit`, and nothing after it happens at all. `run_return` unwinds, so the
    // two releases below are reached — the first when the loop says it is exiting, the second
    // if it never said so. Releasing twice is free: the `Option` is empty the second time.
    let release = Arc::clone(&held);
    let sidecar = Arc::clone(&ai);
    let code = app.run_return(move |_handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            stop(&sidecar);
            take(&release);
        }
    });
    stop(&ai);
    take(&held);
    Ok(code)
}

/// Stops the sidecar and says how it went, on stderr. Idempotent, as [`take`] is.
///
/// **The status is read on this path too**, which is the whole of ADR 0020 §5's instruction
/// not to repeat `Preview::drop`'s gap: a sidecar that crashed earlier in the session is named
/// here even if nobody was looking at the dot when it happened.
fn stop(ai: &Arc<Ai>) {
    let held = ai.sidecar.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(sidecar) = held {
        eprintln!("escribass-app: {}", sidecar.stop());
    }
}

/// Drops the lock guard, which removes `.escri/lock`. Idempotent.
fn take(held: &Held) {
    held.lock().unwrap_or_else(PoisonError::into_inner).take();
}

struct Options {
    project: PathBuf,
    manifest: PathBuf,
    engine: Option<PathBuf>,
    /// The sidecar's command, as argv. `core` is told a command and does not inspect it
    /// (ADR 0020 §4).
    ai: Option<Vec<String>>,
}

impl Options {
    fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut project = None;
        let mut manifest = None;
        let mut engine = None;
        let mut ai = None;
        let mut arguments = arguments.peekable();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--manifest" => {
                    manifest = Some(PathBuf::from(
                        arguments.next().ok_or("--manifest needs a value")?,
                    ))
                }
                "--engine" => {
                    engine =
                        Some(PathBuf::from(arguments.next().ok_or("--engine needs a value")?))
                }
                // `ponytail:` one string, split on whitespace, so a command whose program or
                // arguments contain a space cannot be expressed. The upgrade path is the flag
                // repeated once per word, or an installer that knows where the sidecar lives
                // (M5) — and until one of those, the build-tree command has no space in it.
                "--ai" => {
                    let command = arguments.next().ok_or("--ai needs a value")?;
                    let words: Vec<String> =
                        command.split_whitespace().map(str::to_string).collect();
                    if words.is_empty() {
                        return Err("--ai needs a command to run".to_string());
                    }
                    ai = Some(words);
                }
                "--help" | "-h" => return Err(USAGE.to_string()),
                flag if flag.starts_with('-') => return Err(format!("unknown flag `{flag}`")),
                path => project = Some(PathBuf::from(path)),
            }
        }
        Ok(Options {
            project: project.ok_or_else(|| format!("no project directory given\n{USAGE}"))?,
            // No default and no fallback, exactly as the two server binaries have none: an
            // absent manifest would give `plugin_unknown` and `param_unknown` a silent skip
            // arm (ADR 0010 §4).
            manifest: manifest.ok_or_else(|| {
                format!(
                    "--manifest [manifest_missing]: this build's manifest says what plugins \
                     and parameters exist, and the validator has no answer without it (ADR \
                     0010 §4). It is written by `cmake --build engine/build --target \
                     manifest`\n{USAGE}"
                )
            })?,
            engine,
            ai,
        })
    }
}

const USAGE: &str = "usage: escribass-app --manifest <manifest.json> \
[--engine <escribass_engine>] [--ai <command>] <project.escri>";

#[cfg(test)]
mod tests {
    use super::Options;

    #[test]
    fn a_project_and_a_manifest_are_both_required() {
        let parse = |args: &[&str]| {
            Options::parse(args.iter().map(|s| s.to_string()))
        };
        assert!(parse(&["--manifest", "m.json", "p.escri"]).is_ok());
        assert!(parse(&["p.escri"]).is_err(), "a manifest is not optional");
        assert!(parse(&["--manifest", "m.json"]).is_err(), "a project is not optional");
        assert!(parse(&["--listen", "e", "p.escri"]).is_err(), "an unknown flag is refused");
        // The engine is optional and told, never searched for (ADR 0008 §2): a window with none
        // still edits, and play says so.
        let told = parse(&["--manifest", "m.json", "--engine", "e", "p.escri"]).unwrap();
        assert_eq!(told.engine.as_deref(), Some(std::path::Path::new("e")));
        assert!(parse(&["--manifest", "m.json", "p.escri"]).unwrap().engine.is_none());
        assert!(parse(&["--manifest", "m.json", "p.escri", "--engine"]).is_err());
        // The sidecar is a whole command, not a path, and it is optional in the same way: a
        // window with none still edits, and the dot says there is none (ADR 0020 §4, §5).
        let assisted =
            parse(&["--manifest", "m.json", "--ai", "uv run -p ai escribass-ai", "p.escri"])
                .unwrap();
        assert_eq!(
            assisted.ai.as_deref(),
            Some(["uv", "run", "-p", "ai", "escribass-ai"].map(String::from).as_slice())
        );
        assert!(parse(&["--manifest", "m.json", "p.escri"]).unwrap().ai.is_none());
        assert!(parse(&["--manifest", "m.json", "p.escri", "--ai"]).is_err());
        assert!(parse(&["--manifest", "m.json", "--ai", "  ", "p.escri"]).is_err());
    }

    #[test]
    fn the_dot_has_three_states_and_the_frontend_knows_all_three() {
        use super::dot;
        use escribass_core::Health;

        // `app/src/tool.ts` declares exactly these three, and `App.tsx` gives each a colour.
        assert_eq!(dot(None, None)["state"], "unset");
        let refused = dot(None, Some("cannot start the assistant"));
        assert_eq!(refused["said"], "cannot start the assistant");
        assert_eq!(dot(Some(Health::Running), None)["state"], "running");
        let gone = dot(Some(Health::Gone("the assistant exited 2: no transcript".into())), None);
        assert_eq!(gone["state"], "gone");
        // The status and the last words travel to the window, because that is what a person
        // acts on — not "the assistant is unavailable" (ADR 0020 §5).
        assert_eq!(gone["said"], "the assistant exited 2: no transcript");
    }
}
