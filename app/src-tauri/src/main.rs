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
//! --generator <command>    the command that starts the generative compiler — on a build tree
//!                          `"uv run --no-sync --project compilers/generative escribass-generative"`
//!                          — told and never searched, as the other two children are (ADR 0024
//!                          §1). Absent, the window still edits and the code view still opens:
//!                          pressing Compile says `generator_missing` and names this flag, the
//!                          way pressing play without an engine says `engine_unset`
//! ```
//!
//! And, from M4 PR 7, a third child this process can be told how to start: `--generator`, the
//! generative compiler `core` spawns per compile (ADR 0024 §1). It arrives with the window's
//! Compile button and not before it — M4 PR 5 gave the flag to both server binaries and
//! deliberately left it off this one, because a host with no way to ask for a compile would have
//! been carrying an argument nothing could reach. Like `--engine`, it is optional and never
//! searched for: a session that only edits starts on a machine with no Python environment at
//! all, and the one call that needs it is refused as an operator error rather than skipped.
//!
//! No `--create`, and no File · Open yet. The project is a launch argument, as it is for both
//! server binaries; a menu that opens a second one wants a session per project directory and a
//! window per session, and ADR 0012 §3 has decided what that means without this step having to
//! build it.

// Tauri's own recommendation, and it matters on Windows, where the process would otherwise
// come with a console window attached. Debug builds keep the console so `eprintln!` lands
// somewhere a developer can see it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod panel;

use escribass_core::call::call;
use escribass_core::{
    Assistant, Clock, Engine, Halt, Health, Manifest, Project, ProjectLock, Sandbox, Session,
    Sidecar, SystemClock, UlidSource,
};
use escribass_schema::song::Author;
use panel::{Live, Outcome, Panel};
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};

/// The session, behind the one lock that makes a window a single writer (ADR 0001 §2).
///
/// An `Arc` and not plain managed state, because a turn is driven on a thread of its own: the
/// model decides how long it takes, and a `#[tauri::command]` that held the event loop for a
/// hosted model's answer would be a window that stops repainting for fifteen seconds.
type Held = Arc<Mutex<Session>>;

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
type Locked = Arc<Mutex<Option<ProjectLock>>>;

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
    /// How this window ends a turn it cannot reach (`escribass_core::Halt`).
    ///
    /// [`drive`] holds `sidecar` for the whole of a turn, and a turn is as long as a model
    /// takes, so [`stop`] — which takes the same lock — waited behind it: the window
    /// disappeared and the process, its socket and `.escri/lock` stayed until the model
    /// answered (M3 review, 2026-09-24). This handle does not take that lock, and closing the
    /// sidecar's stdin is what it is: `ai` cancels the request it is serving and leaves, the
    /// stream breaks under the turn, and `stop` then gets the lock and reads the exit status
    /// as it always did (ADR 0020 §5).
    halt: Option<Halt>,
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
    session: tauri::State<'_, Held>,
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
    // `try_lock`, and the reason is a turn: driving one holds the sidecar for as long as the
    // model takes, and a poll that waited for it would block one of Tauri's command threads a
    // second at a time for the length of the turn. A lock that is held is held by something
    // using the sidecar, which took it while the child was there — so `running` is what this
    // window knows, and the moment the turn ends the next poll reads the status for real. A
    // sidecar that died mid-turn is reported by the turn itself, which is where a person is
    // looking (ADR 0020 §5).
    match ai.sidecar.try_lock() {
        Ok(mut held) => dot(held.as_mut().map(Sidecar::health), ai.refused.as_deref()),
        Err(std::sync::TryLockError::Poisoned(poisoned)) => {
            dot(poisoned.into_inner().as_mut().map(Sidecar::health), ai.refused.as_deref())
        }
        Err(std::sync::TryLockError::WouldBlock) => dot(Some(Health::Running), None),
    }
}

// ---------------------------------------------------------------------------
// The AI panel (docs/specs.md §9; ADR 0019 §2, §3; ADR 0021 §3)
// ---------------------------------------------------------------------------
//
// Three commands, and they are the fourth, fifth and sixth things the webview can reach. None
// of them is a second route to the model: [`prompt`] starts a turn whose calls go to
// `core::call` like every other call in this process (ADR 0019 §1), [`panel`] reads what that
// turn has produced, and [`settle`] is the person's decision about it.
//
// **A turn runs on a thread of its own and holds the session for its whole length.** That is
// what one writer means with a model in the loop (ADR 0001 §2): while a turn is running the
// window cannot edit, because the thing it would edit is what the turn is proposing against.
// What it *can* do is draw, and that is why [`panel`] takes no session lock — everything it
// answers with was computed by the watcher `core` calls after every event, on the turn's own
// thread (`panel::Live::watch`).
//
// `ponytail:` a turn is unbounded and there is no Cancel. Dropping what holds the stream is
// how a turn is cancelled (ADR 0013 §2) and the thread holds it, so cancelling means a flag
// the watcher reads and an error out of `one_turn`; the trigger is the first turn somebody
// wants to stop, and until then closing the window ends it with the process.

/// What the panel is drawing: the conversation, and the turn in flight or pending.
#[tauri::command]
fn panel(held: tauri::State<'_, Arc<Mutex<Panel>>>) -> Value {
    held.lock().unwrap_or_else(PoisonError::into_inner).to_json()
}

/// Sends one prompt, and answers as soon as the turn has started.
///
/// It does not wait for the model. A command that did would hold a Tauri worker for however
/// long a hosted model takes — fifteen seconds in the one live run this repository has made —
/// with nothing on screen until it returned, which is the opposite of drawing the proposal as
/// it grows (ADR 0019 §2).
#[tauri::command]
fn prompt(
    text: String,
    session: tauri::State<'_, Held>,
    ai: tauri::State<'_, Arc<Ai>>,
    held: tauri::State<'_, Arc<Mutex<Panel>>>,
) -> Result<(), String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("there is nothing to ask".to_string());
    }
    let (session, ai, panel) =
        (Arc::clone(session.inner()), Arc::clone(ai.inner()), Arc::clone(held.inner()));
    {
        let mut panel = panel.lock().unwrap_or_else(PoisonError::into_inner);
        // A window fact, not a validator rule: this window draws one turn, so it starts one.
        // The structural guard is `Session::propose`, which refuses a second proposal whatever
        // asks for it (ADR 0019 §3) — including an MCP client on this project, which this
        // check cannot see.
        if panel.live.is_some() {
            return Err("apply or reject the pending proposal first: one turn is one proposal                         (ADR 0019 §3)"
                .to_string());
        }
        // The same hasher `core` uses inside the turn, so the id this window shows and the id
        // the log records are one value computed by one function (ADR 0021 §2).
        let prompt_id = escribass_core::asset_hash(text.as_bytes());
        panel.live = Some(Live::started(&text, &prompt_id, now()));
    }
    std::thread::spawn(move || drive(text, session, ai, panel));
    Ok(())
}

/// One turn, driven to its end on a thread of its own.
fn drive(text: String, session: Held, ai: Arc<Ai>, panel: Arc<Mutex<Panel>>) {
    // Read before the session is taken, and sent whole: `ai` holds nothing between streams,
    // so what a turn knows of its own past is what this window sends it (ADR 0021 §3).
    let conversation = {
        let panel = panel.lock().unwrap_or_else(PoisonError::into_inner);
        panel.conversation.wire()
    };
    let mut session = session.lock().unwrap_or_else(PoisonError::into_inner);
    let mut sidecar = ai.sidecar.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(sidecar) = sidecar.as_mut() else {
        return failed(
            &panel,
            "assistant_missing: this window has no assistant. Start it with              `--ai <command>` (ADR 0020 §4)",
        );
    };

    let watching = Arc::clone(&panel);
    let watch = move |turn: &escribass_proto::assistant::Turn, session: &Session| {
        let mut panel = watching.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(live) = panel.live.as_mut() {
            live.watch(turn, session);
        }
    };

    match sidecar.turn(&mut session, &text, &conversation, watch) {
        Ok(turn) => {
            let (provider, model) = session.ai();
            let mut panel = panel.lock().unwrap_or_else(PoisonError::into_inner);
            // The project may have recorded the model on this very prompt (ADR 0021 §4), so
            // the panel's header is refreshed from what the session now says.
            panel.provider = provider.to_string();
            panel.model = model.to_string();
            if let Some(live) = panel.live.as_mut() {
                live.finished(&turn, &session);
            }
        }
        // Nothing is pending: `core` drops the proposal when a turn fails, because a half-built
        // fork nobody has seen is not a thing to leave for a person to decide (ADR 0022 §3).
        // So the turn is settled here and now, with how it failed, and the panel goes idle.
        Err(e) => failed(&panel, &format!("{} [{}]: {}", e.path, e.rule, e.message)),
    }
}

/// A turn that never reached a proposal: recorded with how it failed, and the panel is idle.
fn failed(panel: &Arc<Mutex<Panel>>, why: &str) {
    let mut panel = panel.lock().unwrap_or_else(PoisonError::into_inner);
    let provider = panel.provider.clone();
    if let Some(live) = panel.live.take() {
        let turn = live.settled(&provider, Outcome::Failed(why.to_string()));
        panel.conversation.append(turn);
    }
}

/// Apply, Reject or Edit — §9's three controls, in the order ADR 0019 §3 puts them.
///
/// **Apply** commits the proposal's patch as one entry under `proposal`, with the model as its
/// author. **Reject** drops the fork: nothing was written, so nothing is undone and the log
/// records nothing. **Edit** applies the person's own patch text as the person's, with the
/// prompt's id kept — and when it will not apply it is refused in place and the proposal stays
/// pending (ADR 0017 §3).
///
/// A refusal comes back as the `ToolResult` shape every other refusal in this window arrives
/// in, and is kept on the live turn so the panel can draw it beside the patch it refused.
#[tauri::command]
fn settle(
    action: String,
    patch: Option<String>,
    session: tauri::State<'_, Held>,
    held: tauri::State<'_, Arc<Mutex<Panel>>>,
) -> Result<Value, String> {
    // Read before the session is taken. A running turn holds the session, so a `settle` that
    // took it first would queue behind the turn and then apply a proposal the person decided
    // about before it existed.
    let live = {
        let panel = held.lock().unwrap_or_else(PoisonError::into_inner);
        match &panel.live {
            None => return Err("there is no proposal to decide".to_string()),
            Some(live) if live.running => {
                return Err("the model is still answering".to_string())
            }
            Some(live) => live.clone(),
        }
    };
    let mut session = session.lock().unwrap_or_else(PoisonError::into_inner);
    let (settled, answer) = match action.as_str() {
        "apply" => {
            let result =
                session.apply_proposal(&live.model_id).map_err(|e| e.message)?;
            let entry = result.entry_id.clone();
            (result.valid.then_some(Outcome::Applied(entry)), panel::result_json(&result))
        }
        "reject" => {
            session.reject();
            (Some(Outcome::Rejected), serde_json::json!({ "valid": true, "errors": [] }))
        }
        "edit" => {
            let text = patch.unwrap_or_default();
            let result = session.edit_proposal(text.as_bytes()).map_err(|e| e.message)?;
            let entry = result.entry_id.clone();
            let kept = result.valid && !entry.is_empty();
            (kept.then_some(Outcome::Edited(entry)), panel::result_json(&result))
        }
        other => return Err(format!("`{other}` is not one of apply, reject, edit")),
    };

    let mut panel = held.lock().unwrap_or_else(PoisonError::into_inner);
    let provider = panel.provider.clone();
    match settled {
        Some(outcome) => {
            if let Some(live) = panel.live.take() {
                let turn = live.settled(&provider, outcome);
                panel.conversation.append(turn);
            }
        }
        // Refused, so the proposal is still pending and the window says why (ADR 0017 §3).
        None => {
            if let Some(live) = panel.live.as_mut() {
                live.refusal = answer.clone();
            }
        }
    }
    Ok(answer)
}

/// Now, as the log renders an instant — the one clock this host reads.
fn now() -> Value {
    serde_json::to_value(SystemClock.now()).unwrap_or(Value::Null)
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
    let held: Locked = Arc::new(Mutex::new(Some(lock)));
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
    // Told or absent, never searched, for the engine's reason one child over (ADR 0024 §1). It
    // is handed no project path and no manifest: a compiler is handed what it compiles, and
    // `compile_generator` refuses `generator_missing` when this window was told nothing.
    if let Some(generator) = &options.generator {
        session.set_sandbox(Sandbox::new(generator.clone()));
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
    let halt = started.as_ref().map(Sidecar::halt);
    let ai = Arc::new(Ai { sidecar: Mutex::new(started), refused, halt });

    // What the panel's header names, and what a prompt goes to: the project's record, or the
    // baseline default when it records none (ADR 0021 §4). Read here, once, because the
    // session is held for as long as a turn takes and the panel is drawn while one runs.
    let (provider, model) = {
        let (provider, model) = session.ai();
        (provider.to_string(), model.to_string())
    };
    // The conversation is keyed by the **song's** id, which survives a rename or a move of the
    // project directory (ADR 0021 §3).
    let song_id = session.project().song().id.clone();

    let title = format!(
        "{} — escribass",
        options
            .project
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| options.project.display().to_string())
    );

    let app = tauri::Builder::default()
        .manage(Arc::new(Mutex::new(session)) as Held)
        .manage(hosts)
        .manage(Arc::clone(&ai))
        .invoke_handler(tauri::generate_handler![
            tool, manifest, assistant, panel, prompt, settle
        ])
        .setup(move |app| {
            use tauri::Manager;
            // The project this window is looking at, named where a person can see it. The
            // status bar deliberately shows no §11 number (docs/plan.md, M2 trap 4).
            if let Some(window) = app.get_webview_window("main") {
                window.set_title(&title)?;
            }
            // The conversation, read back beside the project and outside it (ADR 0021 §3).
            // Here rather than above, because the application data directory is Tauri's to
            // name and there is no handle to ask until now.
            let data = app.path().app_data_dir().ok();
            app.manage(Arc::new(Mutex::new(Panel {
                provider,
                model,
                conversation: panel::Conversation::read(data, &song_id),
                live: None,
            })));
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
    // **First, and without the lock**: a turn in flight is holding `sidecar`, and the line
    // below would wait for the model rather than for the window (see [`Ai::halt`]). Closing
    // the sidecar's stdin ends the turn, which releases the lock this then takes.
    if let Some(halt) = &ai.halt {
        halt.halt();
    }
    let held = ai.sidecar.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(sidecar) = held {
        eprintln!("escribass-app: {}", sidecar.stop());
    }
}

/// Drops the lock guard, which removes `.escri/lock`. Idempotent.
fn take(held: &Locked) {
    held.lock().unwrap_or_else(PoisonError::into_inner).take();
}

struct Options {
    project: PathBuf,
    manifest: PathBuf,
    engine: Option<PathBuf>,
    /// The sidecar's command, as argv. `core` is told a command and does not inspect it
    /// (ADR 0020 §4).
    ai: Option<Vec<String>>,
    /// The generative compiler's command, as argv, on the same terms (ADR 0024 §1).
    generator: Option<Vec<String>>,
}

impl Options {
    fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut project = None;
        let mut manifest = None;
        let mut engine = None;
        let mut ai = None;
        let mut generator = None;
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
                // The same bargain as `--ai` above and as both server binaries' own
                // `--generator`: one string split on whitespace, so a path containing a space
                // cannot be expressed, and the upgrade path is the flag repeated once per word.
                "--generator" => {
                    let command = arguments.next().ok_or("--generator needs a value")?;
                    let words: Vec<String> =
                        command.split_whitespace().map(str::to_string).collect();
                    if words.is_empty() {
                        return Err("--generator needs a command to run".to_string());
                    }
                    generator = Some(words);
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
            generator,
        })
    }
}

const USAGE: &str = "usage: escribass-app --manifest <manifest.json> \
[--engine <escribass_engine>] [--ai <command>] [--generator <command>] <project.escri>";

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
        // And the generative compiler, on exactly the sidecar's terms (ADR 0024 §1): a whole
        // command, optional, and told rather than searched for — a window with none still edits
        // and still opens the code view, and pressing Compile says `generator_missing`.
        let compiling = parse(&[
            "--manifest",
            "m.json",
            "--generator",
            "uv run --no-sync --project compilers/generative escribass-generative",
            "p.escri",
        ])
        .unwrap();
        assert_eq!(
            compiling.generator.as_deref(),
            Some(
                [
                    "uv",
                    "run",
                    "--no-sync",
                    "--project",
                    "compilers/generative",
                    "escribass-generative"
                ]
                .map(String::from)
                .as_slice()
            )
        );
        assert!(parse(&["--manifest", "m.json", "p.escri"]).unwrap().generator.is_none());
        assert!(parse(&["--manifest", "m.json", "p.escri", "--generator"]).is_err());
        assert!(parse(&["--manifest", "m.json", "--generator", " ", "p.escri"]).is_err());
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
