//! `render_export`: the two halves of ADR 0006 §2's line, and the gRPC protocol between them
//! (ADR 0008 §1, ADR 0013 §3).
//!
//! The engine this suite drives is a **shell script beside a `Render` server in this process**,
//! not `escribass_engine`. That is the point twice over. What is tested here is `core`'s side of
//! the boundary — the address it reads, the call it makes, the plan that crosses, the result
//! that comes back, and which failures are a refusal and which are an operator's — and a real
//! render would test the engine instead, take forty minutes, and need three plugins. And the
//! split is what makes the unhappy paths reachable at all: the script is the process `core`
//! spawns and waits on, so an exit code, a silent stdout and an address nothing is listening on
//! are each one line of shell, while a real engine would have to be broken to produce them.
//! What a real engine does with a real plan is the render suite, which builds one.
//!
//! A script rather than a Rust helper binary because a helper binary would ship in
//! `core/src/bin/`. `#[cfg(unix)]` for the ones that need one: this repository claims Linux
//! x86-64 and nothing else (ADR 0009 §1), so a shell — and a Unix socket — is a fair assumption
//! where a golden already is.

mod common;
use common::{fake_server, manifest, Served, MANIFEST};

use escribass_core::{new_song, Engine, FixedClock, Project, SeededIds, Session};
use escribass_proto::render::RenderResult;
use escribass_proto::tools::{AddEffectRequest, AddTrackRequest, RenderExportRequest};
use escribass_schema::song::device_ref::Kind;
use escribass_schema::song::{Author, DeviceRef, SourceRef, TrackKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const AT: i64 = 1_788_307_200_000;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-engine-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch directory");
        Self(path)
    }

    fn at(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn opened(dir: &Scratch) -> Session {
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock, Author::Model);
    let project =
        Project::create(dir.at("p.escri"), &song, &mut ids, &clock, Author::Model, manifest())
            .expect("a project");
    Session::new(project, Box::new(ids), Box::new(clock), Author::Model)
}

/// An engine that is a script, in this suite's scratch directory (`common::fake_engine`).
#[cfg(unix)]
fn fake_engine(dir: &Scratch, body: &str) -> PathBuf {
    common::fake_engine(&dir.0, body)
}

/// A `Render` server, and the script that sends `core` to it having done `body` first.
#[cfg(unix)]
fn engine_serving(dir: &Scratch, answer: Result<RenderResult, String>) -> (Served, PathBuf) {
    let served = fake_server(&dir.0, answer);
    let engine = fake_engine(dir, &served.address());
    (served, engine)
}

fn export(path: &Path, dry_run: bool) -> RenderExportRequest {
    RenderExportRequest { output_path: path.display().to_string(), dry_run }
}

// ---- the happy path, and what crosses (ADR 0008 §1, ADR 0013 §3) ----

#[cfg(unix)]
#[test]
fn a_render_hands_over_the_plan_and_answers_with_what_the_engine_reported() {
    // The whole protocol in one claim: the engine is spawned in `--render` mode with the
    // manifest as its argument, it names a socket on its stdout, one `Render` call carries one
    // plan there, and the hash comes back to the caller — which is the reason this call does
    // not answer with a `ToolResult` (song_tools.proto).
    let dir = Scratch::new();
    let mut session = opened(&dir);
    session
        .add_track(&AddTrackRequest {
            name: "Lead".to_string(),
            kind: TrackKind::Instrument as i32,
            r#ref: Some(DeviceRef {
                kind: Some(Kind::Plugin(escribass_schema::song::PluginRef {
                    plugin_id: "Surge Synth Team/Surge XT".to_string(),
                    version: "1.3.4".to_string(),
                })),
            }),
            dry_run: false,
        })
        .expect("a track");

    let answer = RenderResult {
        pcm_sha256: "ab".repeat(32),
        commits: [("juce".to_string(), "deadbeef".to_string())].into_iter().collect(),
    };
    let (served, engine) = engine_serving(&dir, Ok(answer.clone()));
    session.set_engine(Engine::new(&engine, MANIFEST));

    let wav = dir.at("out.wav");
    let response = session.render_export(&export(&wav, false)).expect("the render runs");
    assert!(response.valid, "{:?}", response.errors);
    assert_eq!(response.result, Some(answer), "the engine's own answer, whole");
    assert!(response.summary.contains("track"), "{}", response.summary);
    assert!(
        !response.summary.contains("out.wav"),
        "the summary carries no path: {}",
        response.summary
    );

    // What the engine was handed. `output_path` is this call's argument rather than the
    // document's, and `compile` leaves it empty for the session to fill in.
    let plan = served.plan();
    assert_eq!(plan.output_path, wav.display().to_string());
    assert_eq!(plan.tracks.len(), 1);
    assert_eq!(
        std::fs::read_to_string(dir.at("argument")).expect("the argument").trim(),
        MANIFEST,
        "the engine is handed the manifest core validated against (ADR 0010 §4)"
    );
    // The mode is which service the process serves (ADR 0013 §3). A process spawned to export
    // registers `Render` alone, so `Preview` on it is refused by gRPC rather than by a check
    // somebody wrote — which only holds if `core` actually asks for that mode.
    assert_eq!(
        std::fs::read_to_string(dir.at("mode")).expect("the mode").trim(),
        "--render",
        "the engine is spawned to serve Render and nothing else (ADR 0013 §3)"
    );
}

#[cfg(unix)]
#[test]
fn a_dry_run_compiles_and_starts_no_process() {
    // ADR 0006 §3's first half, one boundary over: the plan is compiled and described, and the
    // engine is not run. The fake engine records that it ran, so "no process" is asserted
    // rather than assumed.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let (served, engine) = engine_serving(&dir, Ok(RenderResult::default()));
    session.set_engine(Engine::new(&engine, MANIFEST));

    let wav = dir.at("out.wav");
    let response = session.render_export(&export(&wav, true)).expect("a dry run");
    assert!(response.valid);
    assert_eq!(response.result, None, "no engine ran, so there is nothing to report");
    assert!(!dir.at("argument").exists(), "the engine process was started on a dry run");
    assert!(!served.called(), "a dry run called Render");
    assert!(!wav.exists(), "a dry run wrote a file");
}

// ---- what a caller can fix: `valid = false` (ADR 0006 §2, ADR 0007 §6) ----

#[test]
fn what_compile_refuses_is_a_refusal_and_no_engine_runs() {
    // A Faust effect is a valid document this milestone cannot render (ADR 0007 §6). It comes
    // back inside the result, with `valid = false`, because the caller fixes it by calling
    // differently — and it must never reach the engine, which is why the engine here does not
    // exist: if anything tried to spawn it, this test would fail as an operator error instead.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    unrenderable(&mut session);
    session.set_engine(Engine::new(dir.at("no-such-engine"), MANIFEST));

    let response = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect("a compile refusal is not an operator error");
    assert!(!response.valid);
    let rules: Vec<&str> = response.errors.iter().map(|e| e.rule.as_str()).collect();
    assert_eq!(rules, ["render_unsupported"], "{:?}", response.errors);
    assert!(response.result.is_none());
}

#[test]
fn a_relative_output_path_is_refused_beside_every_other_reason() {
    // Resolving the path against this process's working directory would write the WAV
    // somewhere the caller cannot predict, so it is a refusal like compile's own — and it
    // arrives *with* compile's own, sorted. A model that fixes one problem per call spends
    // §6's three retries on one render (ADR 0006 §1).
    let dir = Scratch::new();
    let mut session = opened(&dir);
    unrenderable(&mut session);
    session.set_engine(Engine::new(dir.at("no-such-engine"), MANIFEST));

    let response = session
        .render_export(&RenderExportRequest {
            output_path: "out.wav".to_string(),
            dry_run: false,
        })
        .expect("not an operator error");
    assert!(!response.valid);
    let rules: Vec<&str> = response.errors.iter().map(|e| e.rule.as_str()).collect();
    assert_eq!(rules, ["output_path_relative", "render_unsupported"], "{:?}", response.errors);
    assert_eq!(response.errors[0].path, "/output_path");
}

/// A song this engine cannot render: a Surge track carrying a Faust effect, which the
/// validator accepts and this milestone refuses (ADR 0007 §6). Built through the tool API, like
/// every other song in these suites (CLAUDE.md #2).
fn unrenderable(session: &mut Session) {
    let track = session
        .add_track(&AddTrackRequest {
            name: "Lead".to_string(),
            kind: TrackKind::Instrument as i32,
            r#ref: Some(DeviceRef {
                kind: Some(Kind::Plugin(escribass_schema::song::PluginRef {
                    plugin_id: "Surge Synth Team/Surge XT".to_string(),
                    version: "1.3.4".to_string(),
                })),
            }),
            dry_run: false,
        })
        .expect("a track");
    session
        .add_effect(&AddEffectRequest {
            track_id: added(&track),
            r#ref: Some(DeviceRef {
                kind: Some(Kind::Faust(SourceRef { source_hash: "3c1de4f98b".to_string() })),
            }),
            index: None,
            dry_run: false,
        })
        .expect("an effect");
}

// ---- what only an operator can fix: `Err` (ADR 0006 §2, ADR 0008 §1) ----
//
// **Every gRPC failure mode lands here through the same door.** A connection that is refused, a
// stream that ends mid-call and a `Status` returned instead of a message all mean "the engine is
// not going to answer", and `core` then asks the *engine* what became of it rather than asking
// the transport what it saw: a non-zero exit is `engine_failed` and carries what the engine said,
// and an exit of 0 with no answer is `engine_unreadable`, which is M1 PR 13's defect — a render
// that never happened, reported as a success — kept caught on the new transport.

#[test]
fn a_session_that_was_told_no_engine_says_so_rather_than_looking_for_one() {
    // `--engine` is told, never searched, for the reason `--manifest` is (ADR 0010 §4): a
    // search that finds a stale engine renders against a build nobody chose. Absent is loud
    // here rather than a skip somewhere else.
    let dir = Scratch::new();
    let session = opened(&dir);
    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("no engine is an operator error");
    assert_eq!(failed.rule, "engine_unset", "{}", failed.message);

    // And a dry run still works, which is what lets a process that will never render still
    // preview one.
    assert!(session.render_export(&export(&dir.at("out.wav"), true)).unwrap().valid);
}

#[test]
fn an_engine_that_is_not_there_is_an_operator_error() {
    let dir = Scratch::new();
    let mut session = opened(&dir);
    session.set_engine(Engine::new(dir.at("no-such-engine"), MANIFEST));
    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("a missing binary is an operator error");
    assert_eq!(failed.rule, "engine_missing", "{}", failed.message);
    assert!(failed.path.ends_with("no-such-engine"), "{}", failed.path);
}

#[cfg(unix)]
#[test]
fn an_engine_that_exits_non_zero_before_serving_is_an_operator_error_carrying_what_it_said() {
    // ADR 0008 §1: failure is an exit code. Reported as `ProjectError`, never as a violation —
    // a crash inside §6's retry loop is three turns a model cannot spend usefully. An engine
    // that dies before it names a socket is the ordinary shape of "this build cannot host what
    // you asked for", and the reason is on its stderr where every other engine failure is.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let engine = fake_engine(&dir, "echo 'render failed: no such plugin' >&2\nexit 3");
    session.set_engine(Engine::new(&engine, MANIFEST));

    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("a crash is an operator error");
    assert_eq!(failed.rule, "engine_failed", "{}", failed.message);
    assert!(failed.message.contains("exited 3"), "{}", failed.message);
    assert!(failed.message.contains("no such plugin"), "{}", failed.message);
}

#[cfg(unix)]
#[test]
fn an_engine_that_names_no_socket_and_exits_zero_is_an_operator_error() {
    // The case the test above cannot reach, and the one this transport makes new: a process
    // that starts, says nothing and exits 0. There is no address to dial and no failure to
    // report, so without a check this is a render that never happened arriving as a success —
    // which is what `song_tools.proto` §8 calls an operator error and never a refusal.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let engine = fake_engine(&dir, "true");
    session.set_engine(Engine::new(&engine, MANIFEST));

    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("an engine that serves nothing is an operator error");
    assert_eq!(failed.rule, "engine_unreadable", "{}", failed.message);
    assert!(failed.message.contains("stdout"), "{}", failed.message);
    assert!(!dir.at("out.wav").exists(), "nothing was rendered");
}

#[cfg(unix)]
#[test]
fn an_address_nothing_is_listening_on_is_an_operator_error() {
    // Connection refused. The engine named a socket and then was not there to answer on it,
    // which cannot happen to a real one — it prints the address only after gRPC has returned a
    // listening server — and is exactly what a half-written one would do. It exited 0, so it is
    // unreadable rather than failed: there is nothing in a log to look up.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let engine = fake_engine(&dir, &format!("echo unix:{}", dir.at("nothing.sock").display()));
    session.set_engine(Engine::new(&engine, MANIFEST));

    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("an address nothing answers on is an operator error");
    assert_eq!(failed.rule, "engine_unreadable", "{}", failed.message);
    assert!(failed.message.contains("did not answer"), "{}", failed.message);
}

#[cfg(unix)]
#[test]
fn an_engine_that_will_not_answer_and_will_not_die_is_killed_rather_than_waited_for() {
    // The one failure that would not be a failure at all. Every path in `core` that gives up on
    // an engine ends by asking the child what became of it, and a `wait` on one that is still
    // running is an unbounded block in the thread the whole tool API answers from. This engine
    // names a socket nothing is listening on and then sleeps for five minutes; the call comes
    // back, and it comes back as an operator error rather than never.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let engine = fake_engine(
        &dir,
        // `exec`, so the process `core` spawned *is* the one that lingers. A plain `sleep` would
        // be a grandchild holding the stderr pipe open after its parent was killed, and reading
        // that pipe to end of stream is how the engine's last words are collected — so the test
        // would hang on the very thing it exists to prove cannot. The engine spawns nothing,
        // which is why `core` need not care and this test must.
        &format!("echo unix:{}\nexec sleep 300", dir.at("nothing.sock").display()),
    );
    session.set_engine(Engine::new(&engine, MANIFEST));

    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("an engine that will not answer is an operator error");
    assert_eq!(failed.rule, "engine_failed", "{}", failed.message);
    assert!(failed.message.contains("did not answer"), "{}", failed.message);
    assert!(failed.message.contains("killed by a signal"), "{}", failed.message);
}

#[cfg(unix)]
#[test]
fn a_stream_that_ends_mid_call_is_the_engines_failure() {
    // A connection that is accepted and then dropped — the shape of an engine that segfaults
    // with the call in flight. gRPC reports a broken stream; the verdict is still the engine's
    // exit code, because that is the thing a person can act on.
    let dir = Scratch::new();
    let socket = dir.at("hangup.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket).expect("a socket");
    std::thread::spawn(move || {
        for connection in listener.incoming() {
            drop(connection);
        }
    });

    let mut session = opened(&dir);
    let engine = fake_engine(
        &dir,
        &format!("echo unix:{}\necho 'Segmentation fault' >&2\nexit 4", socket.display()),
    );
    session.set_engine(Engine::new(&engine, MANIFEST));

    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("a stream that ends mid-call is an operator error");
    assert_eq!(failed.rule, "engine_failed", "{}", failed.message);
    assert!(failed.message.contains("exited 4"), "{}", failed.message);
    assert!(failed.message.contains("Segmentation fault"), "{}", failed.message);
}

#[cfg(unix)]
#[test]
fn a_status_instead_of_a_result_is_the_engines_failure_and_not_a_refusal() {
    // ADR 0008 §1 again, in the shape the transport gives it: a render the engine could not do
    // ends the call with a gRPC status and the process with a non-zero exit, and never with a
    // `RenderResult` carrying errors. It reaches the caller as an operator error, so a model is
    // not put in a retry loop that cannot succeed.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let served = fake_server(&dir.0, Err("Surge XT would not instantiate".to_string()));
    let engine =
        fake_engine(&dir, &format!("{}\necho 'plugin failed to load' >&2\nexit 3", served.address()));
    session.set_engine(Engine::new(&engine, MANIFEST));

    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("a status is an operator error");
    assert_eq!(failed.rule, "engine_failed", "{}", failed.message);
    assert!(failed.message.contains("exited 3"), "{}", failed.message);
    assert!(failed.message.contains("plugin failed to load"), "{}", failed.message);
    assert!(served.called(), "the plan did reach the engine");
}

#[cfg(unix)]
#[test]
fn an_engine_that_answers_with_no_hash_is_an_operator_error() {
    // proto3 has no required fields, so a `RenderResult` with every field at its default is a
    // well-formed answer: without this check it is a successful render with an empty
    // `pcm_sha256` and no file on disk. The hash is the answer, so its absence is the absence
    // of one — the same defect M1 PR 13 found when an empty stdout decoded cleanly.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let (_served, engine) = engine_serving(&dir, Ok(RenderResult::default()));
    session.set_engine(Engine::new(&engine, MANIFEST));

    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("an engine that renders nothing is an operator error");
    assert_eq!(failed.rule, "engine_unreadable", "{}", failed.message);
    assert!(failed.message.contains("pcm_sha256"), "{}", failed.message);
    assert!(!dir.at("out.wav").exists(), "nothing was rendered");
}

/// The id of the entity a tool's patch added, read from the patch it returned.
///
/// Through the tool API, like every other read of what a tool did (CLAUDE.md #2).
fn added(result: &escribass_proto::tools::ToolResult) -> String {
    let ops: serde_json::Value =
        serde_json::from_slice(&result.patch).expect("the patch is RFC 6902 text");
    let path = ops[0]["path"].as_str().expect("an op has a path");
    path.rsplit('/').next().expect("a path has a last segment").to_string()
}
