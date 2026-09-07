//! `.escri` project directory tests (§10, ADR 0004).
//!
//! Every project under test is built in memory and written by `Project::write`, then read
//! back. No test hand-writes `song.json`, so CLAUDE.md #2 holds even in the loader's tests —
//! the only exception is where a test deliberately corrupts a file to prove `open` notices.

mod common;
use common::manifest;

use escribass_core::{diff, entry, timestamp_from_ms, History, IdSource, Project, SeededIds};
use escribass_schema::history::Refs;
use escribass_schema::song::{Author, Provenance, Song};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");
const BASS: &str = "01M1FPMP00TRACKBASS0000002";

fn fixture_value() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).expect("fixture")).unwrap()
}

/// A directory of its own per test, removed on drop even when the test fails.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-{}-{}.escri",
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

fn provenance() -> Provenance {
    Provenance {
        author: Author::Human as i32,
        model_id: None,
        prompt_id: None,
        tool_call_id: None,
        created_at: Some(timestamp_from_ms(1_788_307_200_000)),
    }
}

/// A project whose log builds the fixture from a default song, plus one edit.
fn sample(root: &PathBuf) -> Project {
    let mut ids = SeededIds::default();
    let mut log = History::new();
    let empty = serde_json::to_value(Song::default()).unwrap();

    let root_id = ids.next_id();
    log.append(entry(root_id.clone(), vec![], "create", &diff(&empty, &fixture_value()), provenance(), 1))
        .unwrap();
    log.create_ref("main", &root_id).unwrap();
    log.set_head("main").unwrap();

    let mut louder = fixture_value();
    louder["tracks"][BASS]["mix"]["gain_db"] = json!(-3.0);
    let next = ids.next_id();
    log.append(entry(next.clone(), vec![root_id], "set_param", &diff(&fixture_value(), &louder), provenance(), 1))
        .unwrap();
    log.advance("main", &next).unwrap();

    Project::new(root, serde_json::from_value(louder).unwrap(), log, manifest())
}

// ---- the round trip ----

#[test]
fn a_project_written_and_reopened_is_unchanged() {
    let dir = Scratch::new();
    let mut original = sample(&dir.0);
    original.write().unwrap();

    let reopened = Project::open(&dir.0, manifest()).unwrap();
    assert_eq!(reopened.song(), original.song());
    assert_eq!(reopened.history(), original.history());
    assert_eq!(reopened, original);
}

#[test]
fn the_directory_holds_exactly_what_section_10_names() {
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();

    let mut found: Vec<String> = std::fs::read_dir(&dir.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    assert_eq!(found, vec!["assets", "lock.json", "patches", "refs.json", "song.json"]);

    // One file per entry, named by its id.
    let patches: Vec<String> = std::fs::read_dir(dir.0.join("patches"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(patches.len(), 2);
    for name in &patches {
        assert!(name.ends_with(".json") && name.len() == 31, "{name}");
    }
}

#[test]
fn writing_twice_produces_identical_bytes() {
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();
    let first = std::fs::read_to_string(dir.0.join("song.json")).unwrap();
    project.write().unwrap();
    assert_eq!(std::fs::read_to_string(dir.0.join("song.json")).unwrap(), first);
}

#[test]
fn song_json_on_disk_is_the_canonical_form() {
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.0.join("song.json")).unwrap(),
        escribass_core::to_canonical_json(project.song()).unwrap()
    );
}

// ---- ADR 0004: the log is authoritative ----

#[test]
fn a_song_that_does_not_match_a_replay_is_reported_not_repaired() {
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();

    // Stand in for a hand-edited file, which §5 forbids and ADR 0004 says to report.
    let text = std::fs::read_to_string(dir.0.join("song.json")).unwrap();
    std::fs::write(dir.0.join("song.json"), text.replace("\"name\": \"Bass\"", "\"name\": \"Edited\"")).unwrap();

    let e = Project::open(&dir.0, manifest()).unwrap_err();
    assert_eq!(e.rule, "song_diverged");
    assert!(e.message.contains("/tracks/"), "the error names the differing path: {}", e.message);
    // Reported, not repaired.
    assert!(std::fs::read_to_string(dir.0.join("song.json")).unwrap().contains("Edited"));
}

#[test]
fn an_orphan_entry_left_by_a_crash_does_not_become_history() {
    // refs.json is written last, so a crash before it leaves an entry nothing references.
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();

    let head = project.history().head_id().unwrap().to_string();
    let orphan = entry("01M1FPMP00RPHAN000000000099", vec![head], "set_param", &[], provenance(), 1);
    std::fs::write(
        dir.0.join("patches").join("01M1FPMP00RPHAN000000000099.json"),
        escribass_core::entry_to_json(&orphan).unwrap(),
    )
    .unwrap();

    let reopened = Project::open(&dir.0, manifest()).unwrap();
    assert_eq!(reopened.history().entries().len(), 3, "the orphan is loaded");
    assert_eq!(reopened.song(), project.song(), "but it is not part of HEAD's history");
}

// ---- what open refuses ----

#[test]
fn a_patch_file_whose_name_disagrees_with_its_id_is_refused() {
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();

    let head = project.history().head_id().unwrap();
    std::fs::copy(
        dir.0.join("patches").join(format!("{head}.json")),
        dir.0.join("patches").join("01M1FPMP00CPYCPY000000000001.json"),
    )
    .unwrap();
    assert_eq!(Project::open(&dir.0, manifest()).unwrap_err().rule, "entry_filename_mismatch");
}

#[test]
fn a_file_that_is_not_a_patch_is_ignored() {
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();
    std::fs::write(dir.0.join("patches").join("notes.txt"), "scratch").unwrap();
    std::fs::write(dir.0.join("patches").join(".song.json.swp"), "editor").unwrap();
    assert!(Project::open(&dir.0, manifest()).is_ok());
}

#[test]
fn a_lock_from_another_schema_version_is_refused_at_load() {
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();
    std::fs::write(dir.0.join("lock.json"), "{\n  \"schema_version\": 99\n}\n").unwrap();
    let e = Project::open(&dir.0, manifest()).unwrap_err();
    assert_eq!(e.rule, "schema_version_mismatch");
    assert!(e.message.contains("99"), "{}", e.message);
}

#[test]
fn a_missing_member_names_the_file() {
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();
    std::fs::remove_file(dir.0.join("refs.json")).unwrap();
    let e = Project::open(&dir.0, manifest()).unwrap_err();
    assert_eq!(e.rule, "unreadable");
    assert!(e.path.ends_with("refs.json"), "{}", e.path);

    assert_eq!(Project::open(std::env::temp_dir().join("no-such.escri"), manifest()).unwrap_err().rule, "unreadable");
}

// ---- atomic writes ----

#[test]
fn a_write_leaves_no_temporary_files_behind() {
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();
    for directory in [&dir.0, &dir.0.join("patches")] {
        for found in std::fs::read_dir(directory).unwrap() {
            let name = found.unwrap().file_name().to_string_lossy().into_owned();
            assert!(!name.ends_with(".tmp"), "left behind: {name}");
        }
    }
}

#[test]
fn a_failed_write_leaves_the_previous_file_intact() {
    // The point of writing through a rename: a reader sees the old file or the new one, never
    // half of one. Simulated by making the temporary path unwritable.
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();
    let before = std::fs::read_to_string(dir.0.join("song.json")).unwrap();

    std::fs::create_dir(dir.0.join(".song.json.tmp")).unwrap();
    assert!(project.write().is_err(), "a directory in the way must fail the write");
    assert_eq!(std::fs::read_to_string(dir.0.join("song.json")).unwrap(), before);
}

#[test]
fn a_log_loads_when_a_child_id_sorts_before_its_parent() {
    // Id order is not ancestry. An entry that arrived from another branch, or one minted
    // across a clock adjustment, can sort below its own parent — and that log is still a
    // valid DAG, so loading it must not depend on the ids happening to be monotonic.
    let later = "01ZZZZZZZZZZZZZZZZZZZZZZZZ";
    let earlier = "01AAAAAAAAAAAAAAAAAAAAAAAA";

    let empty = serde_json::to_value(Song::default()).unwrap();
    let mut louder = fixture_value();
    louder["tracks"][BASS]["mix"]["gain_db"] = json!(-3.0);

    let root = entry(later, vec![], "create", &diff(&empty, &fixture_value()), provenance(), 1);
    let child = entry(
        earlier,
        vec![later.to_string()],
        "set_param",
        &diff(&fixture_value(), &louder),
        provenance(),
        1,
    );

    let refs = Refs {
        head: "main".to_string(),
        refs: [("main".to_string(), earlier.to_string())].into_iter().collect(),
    };
    let log = History::from_parts(vec![child, root], refs).unwrap();
    assert_eq!(log.head_id(), Some(earlier));
    assert_eq!(log.materialise(earlier).unwrap(), louder);
}

#[test]
fn a_log_missing_a_parent_is_still_refused() {
    let ghost = "01ZZZZZZZZZZZZZZZZZZZZZZZZ";
    let empty = serde_json::to_value(Song::default()).unwrap();
    let orphan = entry(
        "01AAAAAAAAAAAAAAAAAAAAAAAA",
        vec![ghost.to_string()],
        "create",
        &diff(&empty, &fixture_value()),
        provenance(),
        1,
    );
    let refs = Refs {
        head: "main".to_string(),
        refs: [("main".to_string(), "01AAAAAAAAAAAAAAAAAAAAAAAA".to_string())]
            .into_iter()
            .collect(),
    };
    assert_eq!(History::from_parts(vec![orphan], refs).unwrap_err().rule, "parent_missing");
}
