//! The desktop host (docs/specs.md §3 tier 1, §9; ADR 0012).
//!
//! ```text
//! escribass-app --manifest <manifest.json> <project.escri>
//! ```
//!
//! One operating-system process: this host, `core` linked into it as a library, and a webview.
//! It is **not** a client of anything — §3's gRPC is `app` ↔ `ai` and `app` ↔ `engine`, both of
//! them processes `app` supervises at a later step, and the webview is neither. `app` opens no
//! network port, because a desktop application that binds a socket to talk to itself is a
//! listening service on a user's machine (ADR 0012 §1).
//!
//! What the webview can reach is [`tool`], and that is the whole of what it can reach *of the
//! model*. One command carrying a tool name and its arguments, dispatched into
//! `escribass_core::call` — the same function the MCP server dispatches into, so §5's "used by
//! the UI and the AI identically" is one dispatch and not two that are free to drift.
//!
//! Beside it, [`manifest`], which answers a question about the running build rather than about
//! the song: what plugins it hosts and what parameters each declares. §9's seventh view is a
//! form over that map (ADR 0014 §1) and there is no tool that reads it, so this is the second
//! and last command — see its own note for why it is not one.
//!
//! Flags:
//!
//! ```text
//! --manifest <path>        **required**, and no default: what this build can host
//!                          (ADR 0010 §4). Without it `plugin_unknown` and `param_unknown`
//!                          would have a silent skip arm, so this process refuses to start
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
use escribass_core::{Manifest, Project, ProjectLock, Session, SystemClock, UlidSource};
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
    let held: Held =
        Arc::new(Mutex::new(Some(ProjectLock::take(&options.project).map_err(|e| e.to_string())?)));
    let project = Project::open(&options.project, Arc::clone(&hosts))
        .map_err(|e| format!("cannot open {}: {e}", options.project.display()))?;

    // One session per project directory, built by the host as a library call — which is ADR
    // 0012 §3's amendment to ADR 0006 §5: the rule that was protecting the *tool surface* is
    // unchanged, and the constructor was never a tool. A window's edits are a person's,
    // whatever else is also editing this project.
    let session = Session::new(
        project,
        Box::new(UlidSource::new(SystemClock)),
        Box::new(SystemClock),
        Author::Human,
    );

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
        .invoke_handler(tauri::generate_handler![tool, manifest])
        .setup(move |app| {
            use tauri::Manager;
            // The project this window is looking at, named where a person can see it. The
            // status bar deliberately shows no §11 number (docs/plan.md, M2 trap 4).
            if let Some(window) = app.get_webview_window("main") {
                window.set_title(&title)?;
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|e| format!("the window could not start: {e}"))?;

    // `run_return` and not `run`, for the same reason the lock is not managed state:
    // `Builder::run` gives the event loop to `tao`, which ends the process with
    // `std::process::exit`, and nothing after it happens at all. `run_return` unwinds, so the
    // two releases below are reached — the first when the loop says it is exiting, the second
    // if it never said so. Releasing twice is free: the `Option` is empty the second time.
    let release = Arc::clone(&held);
    let code = app.run_return(move |_handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            take(&release);
        }
    });
    take(&held);
    Ok(code)
}

/// Drops the lock guard, which removes `.escri/lock`. Idempotent.
fn take(held: &Held) {
    held.lock().unwrap_or_else(PoisonError::into_inner).take();
}

struct Options {
    project: PathBuf,
    manifest: PathBuf,
}

impl Options {
    fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut project = None;
        let mut manifest = None;
        let mut arguments = arguments.peekable();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--manifest" => {
                    manifest = Some(PathBuf::from(
                        arguments.next().ok_or("--manifest needs a value")?,
                    ))
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
        })
    }
}

const USAGE: &str = "usage: escribass-app --manifest <manifest.json> <project.escri>";

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
        assert!(parse(&["--engine", "e", "p.escri"]).is_err(), "an unknown flag is refused");
    }
}
