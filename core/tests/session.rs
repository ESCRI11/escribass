//! The tool API's one implementation (§5, ADR 0006).
//!
//! Every project here is built by `Project::create` and changed only through `Session`, so
//! CLAUDE.md #2 holds: no test writes `song.json` or hand-builds an entry.

mod common;
use common::manifest;

use escribass_core::{new_song, ops_of, to_canonical_json, validate, Project, Session};
use escribass_core::{FixedClock, SeededIds};
use escribass_proto::tools::{AddTrackRequest, ApplyPatchRequest, GetSongAtRequest};
use escribass_schema::song::{Author, Song};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");
const BASS: &str = "01M1FPMP00TRACKBASS0000002";
const NOTE: &str = "/clips/01M1FPMP00CPCHRS0000000006/note_clip/notes/01M1FPMP00NTEG100000000007";
const AT: i64 = 1_788_307_200_000;

fn fixture_value() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).expect("fixture")).unwrap()
}

fn fixture_song() -> Song {
    serde_json::from_value(fixture_value()).unwrap()
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-session-{}-{}.escri",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn opened() -> (Scratch, Session) {
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let project = Project::create(&dir.0, &fixture_song(), &mut ids, &clock, Author::Human, manifest()).unwrap();
    let session = Session::new(project, Box::new(ids), Box::new(clock), Author::Model);
    (dir, session)
}

/// An `apply_patch` request from a JSON array of operations.
fn patch(ops: Value, dry_run: bool) -> ApplyPatchRequest {
    ApplyPatchRequest { patch: serde_json::to_vec(&ops).unwrap(), dry_run }
}

fn set_gain(to: Value) -> Value {
    json!([{"op": "replace", "path": format!("/tracks/{BASS}/mix/gain_db"), "value": to}])
}

// ---- reads ----

#[test]
fn get_song_returns_the_open_document() {
    let (_dir, session) = opened();
    assert_eq!(session.get_song().song.unwrap(), fixture_song());
}

#[test]
fn get_song_at_replays_the_log_to_an_entry() {
    let (_dir, mut session) = opened();
    let root = session.project().history().head_id().unwrap().to_string();

    session.apply_patch(&patch(set_gain(json!(-3.0)), false)).unwrap();
    assert_ne!(session.get_song().song.unwrap(), fixture_song());

    let at_root = session.get_song_at(&GetSongAtRequest { entry_id: root }).unwrap();
    assert_eq!(at_root.song.unwrap(), fixture_song(), "the root entry rebuilds what was created");
}

#[test]
fn get_song_at_an_entry_that_is_not_there_is_an_operator_error() {
    let (_dir, session) = opened();
    let e = session
        .get_song_at(&GetSongAtRequest { entry_id: "01ZZZZZZZZZZZZZZZZZZZZZZZZ".to_string() })
        .unwrap_err();
    assert_eq!(e.rule, "entry_missing");
}

#[test]
fn get_history_returns_every_entry_and_the_refs() {
    let (_dir, mut session) = opened();
    session.apply_patch(&patch(set_gain(json!(-3.0)), false)).unwrap();

    let history = session.get_history();
    assert_eq!(history.entries.len(), 2);
    let refs = history.refs.unwrap();
    assert_eq!(refs.head, "main");
    assert_eq!(refs.refs.len(), 1);
}

// ---- dry run is the real path (ADR 0006 §3) ----

#[test]
fn a_dry_run_returns_the_patch_the_commit_then_records() {
    // The assertion that keeps dry-run honest. If these two ever differ, the patch a user
    // approves in the review pane stops being the patch that gets written (§9).
    let (_dir, mut session) = opened();

    let previewed = session.apply_patch(&patch(set_gain(json!(-3.0)), true)).unwrap();
    assert!(previewed.valid);
    assert!(previewed.entry_id.is_empty(), "a dry run mints no entry");

    let applied = session.apply_patch(&patch(set_gain(json!(-3.0)), false)).unwrap();
    assert_eq!(applied.patch, previewed.patch);
    assert_eq!(applied.summary, previewed.summary);

    let entry = session.project().history().get(&applied.entry_id).unwrap();
    assert_eq!(escribass_core::ops_text(&ops_of(entry).unwrap()).into_bytes(), applied.patch);
}

#[test]
fn a_dry_run_touches_nothing() {
    let (dir, mut session) = opened();
    let before = std::fs::read_to_string(dir.0.join("song.json")).unwrap();
    let head = session.project().history().head_id().unwrap().to_string();

    let result = session.apply_patch(&patch(set_gain(json!(-3.0)), true)).unwrap();
    assert!(result.valid);

    assert_eq!(std::fs::read_to_string(dir.0.join("song.json")).unwrap(), before);
    assert_eq!(session.project().history().entries().len(), 1);
    assert_eq!(session.project().history().head_id().unwrap(), head);
    assert_eq!(session.get_song().song.unwrap(), fixture_song());
}

#[test]
fn the_patch_a_dry_run_returns_is_rfc_6902_text_not_base64() {
    // ADR 0006 §6: `patch` is `bytes`, and the *generated* serde impl would base64 it. What
    // the session puts in the field is the canonical text itself.
    let (_dir, mut session) = opened();
    let result = session.apply_patch(&patch(set_gain(json!(-3.0)), true)).unwrap();
    let text = String::from_utf8(result.patch).unwrap();

    assert!(text.starts_with("[\n"), "{text}");
    assert!(text.ends_with("]\n"), "{text}");
    let ops: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(ops[0]["op"], "replace");
}

// ---- refusals are results, not failures (ADR 0006 §2) ----

#[test]
fn every_refusal_keeps_the_rule_core_gave_it() {
    // The four error types §5's caller matches on, each reaching the wire unchanged. A rule id
    // is API: §6's retry loop and the orchestrator branch on it.
    let cases: [(Value, &str); 5] = [
        // the patch does not apply
        (json!([{"op": "replace", "path": "/tracks/nope/name", "value": "x"}]), "path_not_found"),
        // legal JSON, illegal for the schema — the ADR 0002 §11 defect
        (json!([{"op": "replace", "path": format!("{NOTE}/pitch"), "value": 43.5}]),
            "op_illegal_for_schema"),
        // the result is a song, and an invalid one
        (json!([{"op": "replace", "path": format!("{NOTE}/pitch"), "value": 200}]),
            "pitch_out_of_range"),
        // core maintains `version` (ADR 0005 §3)
        (json!([{"op": "replace", "path": format!("/tracks/{BASS}/version"), "value": 9}]),
            "version_not_writable"),
        // an op the deserializer will not accept at all
        (json!([{"op": "frobnicate", "path": "/version"}]), "patch_unreadable"),
    ];

    for (ops, expected) in cases {
        let (_dir, mut session) = opened();
        let result = session.apply_patch(&patch(ops, false)).expect("a refusal is not a failure");
        assert!(!result.valid, "{expected} should refuse");
        assert_eq!(result.errors.first().map(|e| e.rule.as_str()), Some(expected));
        assert!(result.patch.is_empty(), "a refused call proposes no patch");
        assert!(result.entry_id.is_empty());
        assert_eq!(session.project().history().entries().len(), 1, "{expected} wrote something");
    }
}

#[test]
fn a_refusal_carries_every_violation_not_the_first() {
    // §6 gives a model three retries; one problem per round trip spends them on a document
    // that had four.
    let (_dir, mut session) = opened();
    let result = session
        .apply_patch(&patch(
            json!([
                {"op": "replace", "path": format!("{NOTE}/pitch"), "value": 200},
                {"op": "replace", "path": format!("{NOTE}/velocity"), "value": 300},
                {"op": "replace", "path": format!("/tracks/{BASS}/mix/pan"), "value": 4.0}
            ]),
            false,
        ))
        .unwrap();

    assert!(!result.valid);
    let rules: Vec<&str> = result.errors.iter().map(|e| e.rule.as_str()).collect();
    assert!(rules.contains(&"pitch_out_of_range"), "{rules:?}");
    assert!(rules.contains(&"velocity_out_of_range"), "{rules:?}");
    assert!(rules.contains(&"pan_out_of_range"), "{rules:?}");
}

#[test]
fn a_broken_project_is_an_error_not_a_refusal() {
    // The other side of ADR 0006 §2's line. An unwritable directory is nothing a model can fix
    // by calling differently, so it must not arrive inside the retry loop.
    let (dir, mut session) = opened();
    std::fs::create_dir(dir.0.join(".song.json.tmp")).unwrap();

    let e = session.apply_patch(&patch(set_gain(json!(-3.0)), false)).unwrap_err();
    assert_eq!(e.rule, "unwritable");
}

// ---- input normalisation (ADR 0002 §4) ----

#[test]
fn negative_zero_is_normalised_where_it_enters() {
    // `-0.0` is a distinct double that serialises as `-0.0`, so a song carrying one is not
    // byte-identical to the same song carrying `0.0`. ADR 0002 §4 puts the fix at this
    // boundary; the validator's `negative_zero` rule is what catches anything that gets past.
    let (_dir, mut session) = opened();
    let result = session
        .apply_patch(&patch(
            json!([{"op": "replace", "path": format!("/tracks/{BASS}/mix/pan"), "value": -0.0}]),
            false,
        ))
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let song = session.get_song().song.unwrap();
    let text = to_canonical_json(&song).unwrap();
    assert!(!text.contains("-0.0"), "a negative zero reached the document");
    assert!(validate(&song, &manifest()).is_empty());
}

// ---- summary ----

#[test]
fn the_summary_is_derived_from_the_patch_and_is_stable() {
    let (_dir, mut session) = opened();
    let once = session.apply_patch(&patch(set_gain(json!(-3.0)), true)).unwrap();
    let twice = session.apply_patch(&patch(set_gain(json!(-3.0)), true)).unwrap();

    assert_eq!(once.summary, twice.summary);
    // One caller op plus the version bumps it caused (ADR 0005 §2).
    assert!(once.summary.starts_with("3 ops: "), "{}", once.summary);
    assert!(once.summary.contains("gain_db"), "{}", once.summary);
}

// ---- a new project ----

#[test]
fn a_new_song_is_the_smallest_valid_document() {
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock, Author::Model);

    assert!(validate(&song, &manifest()).is_empty(), "{:?}", validate(&song, &manifest()));
    assert_eq!(song.tracks.len(), 1, "a master track and nothing else");
    assert!(song.clips.is_empty());
    assert_eq!(song.tempo_map.as_ref().unwrap().events.len(), 1);
    assert_eq!(song.provenance.unwrap().created_at.unwrap().seconds, AT / 1000);
}

#[test]
fn two_new_songs_from_the_same_sources_are_identical() {
    // §11: no unseeded randomness, no wall clock. This is what lets M0.4 compare two scripted
    // sessions byte for byte.
    let build = || {
        let mut ids = SeededIds::default();
        to_canonical_json(&new_song(&mut ids, &FixedClock(AT), Author::Model)).unwrap()
    };
    assert_eq!(build(), build());
}

#[test]
fn a_session_can_be_driven_from_a_new_song() {
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock, Author::Model);
    let project = Project::create(&dir.0, &song, &mut ids, &clock, Author::Human, manifest()).unwrap();
    let mut session = Session::new(project, Box::new(ids), Box::new(clock), Author::Model);

    let master = song.tracks.keys().next().unwrap().clone();
    let result = session
        .apply_patch(&patch(
            json!([{"op": "replace", "path": format!("/tracks/{master}/name"), "value": "Out"}]),
            false,
        ))
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    assert_eq!(Project::open(&dir.0, manifest()).unwrap().song(), session.project().song());
}

#[test]
fn a_dry_run_of_a_minting_tool_returns_the_patch_the_apply_then_records() {
    // ADR 0006 §3 promises a dry run shows what a commit would record. A tool that mints ids
    // used to consume them on the preview, so the apply that followed carried different ones —
    // the documents were equivalent and the promise was not, which matters because §9 has a
    // person approve the patch before it is applied.
    let (_dir, mut session) = opened();
    let request = escribass_proto::tools::AddTrackRequest {
        name: "Bass".to_string(),
        kind: escribass_schema::song::TrackKind::Instrument as i32,
        r#ref: Some(escribass_schema::song::DeviceRef {
            kind: Some(escribass_schema::song::device_ref::Kind::Cmajor(
                escribass_schema::song::SourceRef {
                    source_hash: "8b31c0de4f9c00000000000000000000".to_string(),
                },
            )),
        }),
        dry_run: true,
    };

    let previewed = session.add_track(&request).unwrap();
    let previewed_twice = session.add_track(&request).unwrap();
    assert_eq!(previewed_twice.patch, previewed.patch, "two previews disagreed");

    let applied = session.add_track(&AddTrackRequest { dry_run: false, ..request }).unwrap();
    assert_eq!(applied.patch, previewed.patch, "the apply differed from what was approved");
    assert!(!applied.entry_id.is_empty());
}

#[test]
fn a_dry_run_does_not_advance_the_ids_a_later_call_uses() {
    // The same property from the other side: previewing never burns an id, so a session that
    // previews twice and applies once produces the same project as one that just applies.
    let with_previews = {
        let (dir, mut session) = opened();
        let ops = set_gain(json!(-3.0));
        session.apply_patch(&patch(ops.clone(), true)).unwrap();
        session.apply_patch(&patch(ops.clone(), true)).unwrap();
        session.apply_patch(&patch(ops, false)).unwrap();
        std::fs::read_to_string(dir.0.join("song.json")).unwrap()
    };
    let without = {
        let (dir, mut session) = opened();
        session.apply_patch(&patch(set_gain(json!(-3.0)), false)).unwrap();
        std::fs::read_to_string(dir.0.join("song.json")).unwrap()
    };
    assert_eq!(with_previews, without);
}

#[test]
fn the_patch_this_api_returns_can_be_applied_back_through_it() {
    // §9 has a user apply, reject or *edit* a proposed diff, and `ApplyPatchRequest.patch` is
    // documented as the form `ToolResult.patch` returns. That patch carries the version bumps
    // it caused (ADR 0006 §1), so a guard that refused every op writing a version refused the
    // API's own output — the round trip had no working path at all.
    let (_dir, mut session) = opened();
    let previewed = session.apply_patch(&patch(set_gain(json!(-3.0)), true)).unwrap();
    assert!(previewed.valid, "{:?}", previewed.errors);

    let applied = session
        .apply_patch(&ApplyPatchRequest { patch: previewed.patch.clone(), dry_run: false })
        .unwrap();

    assert!(applied.valid, "{:?}", applied.errors);
    assert_eq!(applied.patch, previewed.patch, "applying the preview changed it");
}

#[test]
fn a_failed_write_leaves_the_session_where_the_disk_is() {
    // The caller is told nothing happened; the session must agree. Committing in memory and
    // then failing to write left every later call building on state the disk never saw.
    let (dir, mut session) = opened();
    let before = session.get_song().song.unwrap();
    let entries = session.project().history().entries().len();
    std::fs::create_dir(dir.0.join(".song.json.tmp")).unwrap();

    let failed = session.apply_patch(&patch(set_gain(json!(-3.0)), false)).unwrap_err();
    assert_eq!(failed.rule, "unwritable");

    assert_eq!(session.get_song().song.unwrap(), before, "the session committed anyway");
    assert_eq!(session.project().history().entries().len(), entries, "an entry survived");
}
