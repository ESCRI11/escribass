//! `render_export`: the two halves of ADR 0006 §2's line, and the stdio protocol between them
//! (ADR 0008 §1).
//!
//! The engine this suite drives is a **shell script**, not `escribass_engine`. That is the
//! point: what is tested here is `core`'s side of the boundary — the plan that goes in, the
//! result that comes back, and which failures are a refusal and which are an operator's — and
//! a real render would test the engine instead, take forty minutes, and need three plugins.
//! What a real engine does with a real plan is PR 11's render suite, which builds one.
//!
//! A script rather than a Rust helper binary because a helper binary would ship in
//! `core/src/bin/`. `#[cfg(unix)]` for the two that need one: M1 claims Linux x86-64 and
//! nothing else (ADR 0009 §1), so a shell is a fair assumption where a golden already is.

mod common;
use common::{manifest, MANIFEST};

use escribass_core::{new_song, Engine, FixedClock, Project, SeededIds, Session};
use escribass_proto::render::{RenderPlan, RenderResult};
use escribass_proto::tools::{AddEffectRequest, AddTrackRequest, RenderExportRequest};
use escribass_schema::song::device_ref::Kind;
use escribass_schema::song::{Author, DeviceRef, SourceRef, TrackKind};
use prost::Message;
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

#[cfg(unix)]
fn plan_it_was_given(dir: &Scratch) -> RenderPlan {
    let bytes = std::fs::read(dir.at("plan.binpb")).expect("the fake engine kept the plan");
    RenderPlan::decode(&bytes[..]).expect("what core wrote is a RenderPlan")
}

fn export(path: &Path, dry_run: bool) -> RenderExportRequest {
    RenderExportRequest { output_path: path.display().to_string(), dry_run }
}

// ---- the happy path, and what crosses (ADR 0008 §1) ----

#[cfg(unix)]
#[test]
fn a_render_hands_over_the_plan_and_answers_with_what_the_engine_reported() {
    // The whole protocol in one claim: one plan in on stdin, one RenderResult out on stdout,
    // the manifest as the engine's argument, and the hash reaching the caller — which is the
    // reason this call does not answer with a `ToolResult` (song_tools.proto).
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
    std::fs::write(dir.at("answer.binpb"), answer.encode_to_vec()).expect("the canned answer");
    let engine = fake_engine(&dir, &format!("cat '{}'", dir.at("answer.binpb").display()));
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
    let plan = plan_it_was_given(&dir);
    assert_eq!(plan.output_path, wav.display().to_string());
    assert_eq!(plan.tracks.len(), 1);
    assert_eq!(
        std::fs::read_to_string(dir.at("argument")).expect("the argument").trim(),
        MANIFEST,
        "the engine is handed the manifest core validated against (ADR 0010 §4)"
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
    let engine = fake_engine(&dir, "true");
    session.set_engine(Engine::new(&engine, MANIFEST));

    let wav = dir.at("out.wav");
    let response = session.render_export(&export(&wav, true)).expect("a dry run");
    assert!(response.valid);
    assert_eq!(response.result, None, "no engine ran, so there is nothing to report");
    assert!(!dir.at("plan.binpb").exists(), "the engine was started on a dry run");
    assert!(!wav.exists(), "a dry run wrote a file");
}

// ---- what a caller can fix: `valid = false` (ADR 0006 §2, ADR 0007 §6) ----

#[test]
fn what_compile_refuses_is_a_refusal_and_no_engine_runs() {
    // A Faust effect is a valid document M1 cannot render (ADR 0007 §6). It comes back inside
    // the result, with `valid = false`, because the caller fixes it by calling differently —
    // and it must never reach the engine, which is why the engine here does not exist: if
    // anything tried to spawn it, this test would fail as an operator error instead.
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
/// validator accepts and M1 refuses (ADR 0007 §6). Built through the tool API, like every
/// other song in these suites (CLAUDE.md #2).
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
    assert_eq!(failed.rule, "engine_unset");

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
    assert_eq!(failed.rule, "engine_missing");
    assert!(failed.path.ends_with("no-such-engine"), "{}", failed.path);
}

#[cfg(unix)]
#[test]
fn an_engine_that_exits_non_zero_is_an_operator_error_carrying_what_it_said() {
    // ADR 0008 §1: failure is an exit code. Reported as `ProjectError`, never as a violation —
    // a crash inside §6's retry loop is three turns a model cannot spend usefully.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let engine = fake_engine(&dir, "echo 'render failed: no such plugin' >&2\nexit 3");
    session.set_engine(Engine::new(&engine, MANIFEST));

    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("a crash is an operator error");
    assert_eq!(failed.rule, "engine_failed");
    assert!(failed.message.contains("exited 3"), "{}", failed.message);
    assert!(failed.message.contains("no such plugin"), "{}", failed.message);
}

#[cfg(unix)]
#[test]
fn stdout_that_is_not_a_render_result_is_an_operator_error() {
    // ADR 0008 §1's other half: stdout carries protobuf bytes and nothing else. An engine that
    // exits 0 having printed a log line to the wrong stream is broken, not refusing.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let engine = fake_engine(&dir, "echo 'all done!'");
    session.set_engine(Engine::new(&engine, MANIFEST));

    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("an undecodable answer is an operator error");
    assert_eq!(failed.rule, "engine_unreadable");
}

#[cfg(unix)]
#[test]
fn an_engine_that_exits_zero_saying_nothing_is_an_operator_error() {
    // The case the test above cannot reach: `echo` writes bytes that fail to decode, but an
    // engine that writes *nothing* hands `RenderResult::decode` an empty slice, which is a
    // valid proto3 message. Without a check on the hash this is a "successful" render with an
    // empty `pcm_sha256` and no file on disk — precisely what song_tools.proto §8 says is an
    // operator error and never a refusal.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let engine = fake_engine(&dir, "true");
    session.set_engine(Engine::new(&engine, MANIFEST));

    let failed = session
        .render_export(&export(&dir.at("out.wav"), false))
        .expect_err("an engine that renders nothing is an operator error");
    assert_eq!(failed.rule, "engine_unreadable");
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
