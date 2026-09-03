//! `create` and `commit`: the mutation entry point (§5, ADR 0004).
//!
//! Everything a project contains here was produced by `create` and `commit` — no test builds
//! a history by hand or writes `song.json` — so CLAUDE.md #2 holds end to end.

use escribass_core::{
    to_canonical_json, validate, Clock, FixedClock, Op, Project, SeededIds,
};
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
            "escribass-commit-{}-{}.escri",
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

fn sources() -> (SeededIds, FixedClock) {
    (SeededIds::default(), FixedClock(AT))
}

/// One op: change a note's pitch.
fn set_pitch(to: i64) -> Vec<Op> {
    serde_json::from_value(json!([{"op": "replace", "path": format!("{NOTE}/pitch"), "value": to}]))
        .unwrap()
}

/// The invariant ADR 0004 rests on: what the log replays to is what is on disk.
fn assert_head_matches_disk(project: &Project) {
    let head = project.history().head_id().expect("HEAD");
    let replayed: Song =
        serde_json::from_value(project.history().materialise(head).unwrap()).unwrap();
    assert_eq!(
        to_canonical_json(&replayed).unwrap(),
        std::fs::read_to_string(project.root().join("song.json")).unwrap(),
        "a replay of the log must equal song.json byte for byte"
    );
}

// ---- create ----

#[test]
fn create_writes_a_project_whose_root_entry_builds_the_song() {
    let dir = Scratch::new();
    let (mut ids, clock) = sources();
    let project = Project::create(&dir.0, &fixture_song(), &mut ids, &clock).unwrap();

    assert_eq!(project.history().entries().len(), 1);
    assert_eq!(project.history().refs().head, "main");
    assert_eq!(project.song(), &fixture_song());
    assert_head_matches_disk(&project);
    assert_eq!(Project::open(&dir.0).unwrap().song(), &fixture_song());
}

#[test]
fn create_refuses_an_invalid_song_and_writes_nothing() {
    let dir = Scratch::new();
    let (mut ids, clock) = sources();
    let mut broken = fixture_song();
    broken.tracks.get_mut(BASS).unwrap().mix.as_mut().unwrap().pan = 4.0;

    let e = Project::create(&dir.0, &broken, &mut ids, &clock).unwrap_err();
    assert_eq!(e.rule, "song_invalid");
    assert!(e.message.contains("pan_out_of_range"), "{}", e.message);
    assert!(!dir.0.join("song.json").exists(), "nothing was written");
}

#[test]
fn create_refuses_to_overwrite_an_existing_project() {
    let dir = Scratch::new();
    let (mut ids, clock) = sources();
    Project::create(&dir.0, &fixture_song(), &mut ids, &clock).unwrap();
    let e = Project::create(&dir.0, &fixture_song(), &mut ids, &clock).unwrap_err();
    assert_eq!(e.rule, "project_exists");
}

// ---- commit ----

#[test]
fn a_commit_records_the_patch_and_advances_the_branch() {
    let dir = Scratch::new();
    let (mut ids, clock) = sources();
    let mut project = Project::create(&dir.0, &fixture_song(), &mut ids, &clock).unwrap();

    let id = project.commit("set_notes", &set_pitch(45), Author::Model, &mut ids, &clock).unwrap();

    assert_eq!(project.history().entries().len(), 2);
    assert_eq!(project.history().head_id().unwrap(), id);
    assert_head_matches_disk(&project);

    let entry = project.history().get(&id).unwrap();
    assert_eq!(entry.tool, "set_notes");
    assert_eq!(entry.provenance.as_ref().unwrap().created_at.as_ref().unwrap().seconds, AT / 1000);

    // The caller's op plus the version bumps it caused (ADR 0005 §1): the note, its clip, and
    // the song. The bumps are *in* the entry, which is the whole point of where they happen —
    // recorded elsewhere, every replay would come out a version behind `song.json`.
    let recorded = escribass_core::ops_of(entry).unwrap();
    assert_eq!(recorded[0], set_pitch(45)[0]);
    let bumps: Vec<(&str, &serde_json::Value)> = recorded[1..]
        .iter()
        .map(|op| match op {
            Op::Replace { path, value } => (path.as_str(), value),
            other => panic!("a bump is a replace, not {other:?}"),
        })
        .collect();
    assert_eq!(
        bumps,
        vec![
            (&*format!("{NOTE}/version"), &json!(2)),
            ("/clips/01M1FPMP00CPCHRS0000000006/version", &json!(3)),
            ("/version", &json!(215)),
        ]
    );
}

#[test]
fn the_replay_invariant_holds_after_every_commit() {
    // The single assertion standing between the log and a document core cannot read.
    let dir = Scratch::new();
    let (mut ids, clock) = sources();
    let mut project = Project::create(&dir.0, &fixture_song(), &mut ids, &clock).unwrap();

    for pitch in [40, 41, 42, 43, 44] {
        project.commit("set_notes", &set_pitch(pitch), Author::Model, &mut ids, &clock).unwrap();
        assert_head_matches_disk(&project);
    }
    // And a project rebuilt from disk equals the live one.
    let reopened = Project::open(&dir.0).unwrap();
    assert_eq!(reopened.song(), project.song());
    assert_eq!(reopened.history(), project.history());
}

#[test]
fn a_commit_that_fails_validation_leaves_the_project_untouched() {
    let dir = Scratch::new();
    let (mut ids, clock) = sources();
    let mut project = Project::create(&dir.0, &fixture_song(), &mut ids, &clock).unwrap();
    let before_song = std::fs::read_to_string(dir.0.join("song.json")).unwrap();
    let before_head = project.history().head_id().unwrap().to_string();

    // 200 is outside MIDI's 0-127.
    let e = project.commit("set_notes", &set_pitch(200), Author::Model, &mut ids, &clock).unwrap_err();
    assert_eq!(e.rule, "song_invalid");
    assert!(e.message.contains("pitch_out_of_range"), "{}", e.message);

    assert_eq!(project.history().entries().len(), 1, "no orphan entry");
    assert_eq!(project.history().head_id().unwrap(), before_head, "the ref did not move");
    assert_eq!(std::fs::read_to_string(dir.0.join("song.json")).unwrap(), before_song);
    assert_eq!(std::fs::read_dir(dir.0.join("patches")).unwrap().count(), 1);
}

#[test]
fn an_op_that_is_legal_json_and_illegal_for_the_schema_is_refused() {
    // The ADR 0002 §11 defect: 43.0 applies cleanly to a Value and produces a document that
    // is not a Song. The re-deserialisation in commit is what catches it.
    let dir = Scratch::new();
    let (mut ids, clock) = sources();
    let mut project = Project::create(&dir.0, &fixture_song(), &mut ids, &clock).unwrap();

    let float_pitch: Vec<Op> =
        serde_json::from_value(json!([{"op": "replace", "path": format!("{NOTE}/pitch"), "value": 43.5}])).unwrap();
    let e = project.commit("set_notes", &float_pitch, Author::Model, &mut ids, &clock).unwrap_err();
    assert_eq!(e.rule, "op_illegal_for_schema");
    assert!(e.message.contains("pitch"), "the error names the ops that touched it: {}", e.message);
    assert_eq!(project.history().entries().len(), 1, "nothing was recorded");
}

#[test]
fn an_op_that_does_not_apply_is_refused_before_anything_else() {
    let dir = Scratch::new();
    let (mut ids, clock) = sources();
    let mut project = Project::create(&dir.0, &fixture_song(), &mut ids, &clock).unwrap();
    let missing: Vec<Op> =
        serde_json::from_value(json!([{"op": "replace", "path": "/nope", "value": 1}])).unwrap();
    assert_eq!(
        project.commit("x", &missing, Author::Human, &mut ids, &clock).unwrap_err().rule,
        "path_not_found"
    );
    assert_eq!(project.history().entries().len(), 1);
}

// ---- branching, end to end ----

#[test]
fn branch_edit_and_switch_back_returns_the_original_byte_for_byte() {
    let dir = Scratch::new();
    let (mut ids, clock) = sources();
    let mut project = Project::create(&dir.0, &fixture_song(), &mut ids, &clock).unwrap();
    let original = to_canonical_json(project.song()).unwrap();

    project.commit("set_notes", &set_pitch(45), Author::Model, &mut ids, &clock).unwrap();
    let edited = to_canonical_json(project.song()).unwrap();
    assert_ne!(edited, original);

    // A ref at the root is a branch; switching to it returns the earlier document.
    let root = project.history().ancestry(project.history().head_id().unwrap()).unwrap()[0]
        .id
        .clone();
    let mut history = project.history().clone();
    history.create_ref("before", &root).unwrap();
    let current = serde_json::to_value(project.song()).unwrap();
    let ops = history.switch("before", &current).unwrap();

    let back: Song = serde_json::from_value(escribass_core::apply(&current, &ops).unwrap()).unwrap();
    assert_eq!(to_canonical_json(&back).unwrap(), original);
    assert_eq!(validate(&back), vec![]);
}

// ---- determinism (§11) ----

#[test]
fn two_identical_scripted_sessions_produce_byte_identical_directories() {
    // M0.4's claim, proved early: same input, same bytes, with no normalisation step.
    let run = |dir: &PathBuf| {
        let (mut ids, clock) = sources();
        let mut project = Project::create(dir, &fixture_song(), &mut ids, &clock).unwrap();
        for pitch in [40, 41, 42] {
            project.commit("set_notes", &set_pitch(pitch), Author::Model, &mut ids, &clock).unwrap();
        }
        let mut files: Vec<(String, String)> = Vec::new();
        for entry in walkdir(dir) {
            let name = entry.strip_prefix(dir).unwrap().to_string_lossy().into_owned();
            files.push((name, std::fs::read_to_string(&entry).unwrap()));
        }
        files.sort();
        files
    };

    fn walkdir(root: &PathBuf) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).unwrap() {
                let path = e.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    found.push(path);
                }
            }
        }
        found
    }

    let (a, b) = (Scratch::new(), Scratch::new());
    let (first, second) = (run(&a.0), run(&b.0));
    assert_eq!(first, second, "two identical sessions must produce identical bytes");
    assert!(first.len() >= 7, "song.json, refs.json, lock.json and four entries: {}", first.len());
}

#[test]
fn the_clock_is_the_only_source_of_time() {
    // A different clock changes only the timestamps, proving nothing reads the wall clock.
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let project = Project::create(&dir.0, &fixture_song(), &mut ids, &FixedClock(0)).unwrap();
    let head = project.history().head_id().unwrap();
    let created = project.history().get(head).unwrap().provenance.as_ref().unwrap();
    assert_eq!(created.created_at.as_ref().unwrap().seconds, 0);
    assert_eq!(FixedClock(AT).now().seconds, AT / 1000);
}

#[test]
fn a_commit_records_the_effect_not_the_callers_spelling() {
    // Proto3 JSON has more than one spelling for a value: `"64"` is a legal int32, and a
    // `Song` hands it back as `64`. An entry that kept the caller's spelling would replay to a
    // document that differs from the `song.json` written beside it, and `open` would refuse a
    // project that had just been committed cleanly.
    let dir = Scratch::new();
    let (mut ids, clock) = sources();
    let mut project = Project::create(&dir.0, &fixture_song(), &mut ids, &clock).unwrap();

    let quoted: Vec<Op> = serde_json::from_value(
        json!([{"op": "replace", "path": format!("{NOTE}/pitch"), "value": "64"}]),
    )
    .unwrap();
    let id = project.commit("set_pitch", &quoted, Author::Human, &mut ids, &clock).unwrap();

    let recorded: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.0.join("patches").join(format!("{id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(recorded["ops"][0]["value"], json!(64), "the log holds the canonical spelling");

    // The invariant that matters: what was written can be read back.
    assert_eq!(Project::open(&dir.0).unwrap().song(), project.song());
}
