//! The branch tools of §5: `create_branch`, `switch_branch`, `delete_branch` (ADR 0001 §2).
//!
//! Branching is the feature the patch DAG exists for, and the property that makes it worth
//! having is that it copies nothing and loses nothing: leave a branch, come back, and the song
//! is byte for byte what it was. Most of these tests are that claim from one angle or another.

use escribass_core::{new_song, FixedClock, Project, SeededIds, Session};
use escribass_proto::tools::{
    AddSectionRequest, AddTrackRequest, CreateBranchRequest, DeleteBranchRequest,
    SetTempoRequest, SwitchBranchRequest, ToolResult,
};
use escribass_schema::song::{Author, DeviceRef, SourceRef, TrackKind};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const AT: i64 = 1_788_307_200_000;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-branch-{}-{}.escri",
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
    let song = new_song(&mut ids, &clock);
    let project = Project::create(&dir.0, &song, &mut ids, &clock).unwrap();
    (dir, Session::new(project, Box::new(ids), Box::new(clock), Author::Model))
}

fn cmajor() -> Option<DeviceRef> {
    Some(DeviceRef {
        kind: Some(escribass_schema::song::device_ref::Kind::Cmajor(SourceRef {
            source_hash: "8b31c0de4f9c00000000000000000000".to_string(),
        })),
    })
}

fn branch(name: &str, at: &str, dry_run: bool) -> CreateBranchRequest {
    CreateBranchRequest {
        name: name.to_string(),
        at_entry_id: at.to_string(),
        dry_run,
    }
}

fn switch(name: &str, dry_run: bool) -> SwitchBranchRequest {
    SwitchBranchRequest { name: name.to_string(), dry_run }
}

fn rules(result: &ToolResult) -> Vec<&str> {
    result.errors.iter().map(|e| e.rule.as_str()).collect()
}

/// One commit, so a branch has somewhere to diverge from.
fn add_a_track(session: &mut Session, name: &str) {
    let result = session
        .add_track(&AddTrackRequest {
            name: name.to_string(),
            kind: TrackKind::Instrument as i32,
            r#ref: cmajor(),
            dry_run: false,
        })
        .unwrap();
    assert!(result.valid, "{:?}", result.errors);
}

fn song_text(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("song.json")).unwrap()
}

// ---- create ----

#[test]
fn a_branch_names_a_position_and_copies_nothing() {
    let (_dir, mut session) = opened();
    let entries_before = session.project().history().entries().len();

    let result = session.create_branch(&branch("try-darker-chorus", "", false)).unwrap();

    assert!(result.valid, "{:?}", result.errors);
    assert!(result.entry_id.is_empty(), "a branch is not a commit");
    assert!(result.patch.is_empty(), "nothing in the song changed");
    assert_eq!(session.project().history().entries().len(), entries_before, "no entry was added");
    assert_eq!(session.project().history().refs().refs.len(), 2);
    assert_eq!(session.project().history().refs().head, "main", "creating does not switch");
}

#[test]
fn a_branch_can_name_an_earlier_entry() {
    let (_dir, mut session) = opened();
    let root = session.project().history().head_id().unwrap().to_string();
    add_a_track(&mut session, "Bass");

    let result = session.create_branch(&branch("from-the-start", &root, false)).unwrap();

    assert!(result.valid, "{:?}", result.errors);
    assert_eq!(session.project().history().refs().refs["from-the-start"], root);
}

#[test]
fn a_ref_name_that_breaks_adr_0001_is_refused() {
    let (_dir, mut session) = opened();
    for (name, rule) in [
        ("Scratch", "ref_name_charset"),
        ("/leading", "ref_name_slash"),
        ("a/../b", "ref_name_dotdot"),
        ("main", "ref_exists"),
    ] {
        let result = session.create_branch(&branch(name, "", false)).unwrap();
        assert!(!result.valid, "`{name}` was accepted");
        assert_eq!(rules(&result), vec![rule], "for `{name}`");
    }
}

#[test]
fn a_branch_from_an_entry_that_is_not_there_is_refused() {
    let (_dir, mut session) = opened();
    let result = session
        .create_branch(&branch("ghost", "01ZZZZZZZZZZZZZZZZZZZZZZZZ", false))
        .unwrap();
    assert_eq!(rules(&result), vec!["entry_missing"]);
}

// ---- switch ----

#[test]
fn leaving_a_branch_and_coming_back_returns_the_song_byte_for_byte() {
    // The claim branching is for. If this ever fails, the DAG is a filing system rather than a
    // way to try things.
    let (dir, mut session) = opened();
    add_a_track(&mut session, "Bass");
    let on_main = song_text(&dir.0);

    session.create_branch(&branch("darker", "", false)).unwrap();
    session.switch_branch(&switch("darker", false)).unwrap();
    session.set_tempo(&SetTempoRequest { bpm: 88.0, tick: 0, dry_run: false }).unwrap();
    session
        .add_section(&AddSectionRequest {
            name: "Verse".to_string(),
            start_tick: 0,
            end_tick: 3840,
            dry_run: false,
        })
        .unwrap();
    assert_ne!(song_text(&dir.0), on_main, "the branch diverged");

    session.switch_branch(&switch("main", false)).unwrap();

    assert_eq!(song_text(&dir.0), on_main);
    assert_eq!(session.project().history().refs().head, "main");
}

#[test]
fn switching_appends_nothing() {
    // A log that recorded navigation would grow every time somebody looked at a branch
    // (ADR 0001 §2).
    let (_dir, mut session) = opened();
    add_a_track(&mut session, "Bass");
    session.create_branch(&branch("darker", "", false)).unwrap();
    let entries = session.project().history().entries().len();

    session.switch_branch(&switch("darker", false)).unwrap();
    session.switch_branch(&switch("main", false)).unwrap();

    assert_eq!(session.project().history().entries().len(), entries);
}

#[test]
fn a_switch_returns_the_patch_that_gets_there() {
    let (_dir, mut session) = opened();
    let root = session.project().history().head_id().unwrap().to_string();
    add_a_track(&mut session, "Bass");
    session.create_branch(&branch("from-the-start", &root, false)).unwrap();

    let result = session.switch_branch(&switch("from-the-start", false)).unwrap();

    assert!(result.valid, "{:?}", result.errors);
    assert!(result.entry_id.is_empty(), "switching is not a commit");
    let patch: Value = serde_json::from_slice(&result.patch).unwrap();
    let removes = patch
        .as_array()
        .unwrap()
        .iter()
        .filter(|op| op["op"] == "remove" && op["path"].as_str().unwrap().starts_with("/tracks/"))
        .count();
    assert_eq!(removes, 1, "the track added on main is removed on the way back: {patch}");
}

#[test]
fn a_dry_run_switch_does_not_move_head() {
    // The trap this PR is mostly about. `History::switch` sets `refs.head` as a side effect, so
    // a preview built by calling it and not writing would leave the *next* commit landing on a
    // branch nobody chose — a failure with no error, visible only later as history on the
    // wrong ref. `patch_to` is the pure half, and this is the assertion that keeps it used.
    let (dir, mut session) = opened();
    add_a_track(&mut session, "Bass");
    session.create_branch(&branch("darker", "", false)).unwrap();
    session.switch_branch(&switch("darker", false)).unwrap();
    session.set_tempo(&SetTempoRequest { bpm: 88.0, tick: 0, dry_run: false }).unwrap();

    let on_darker = song_text(&dir.0);
    let previewed = session.switch_branch(&switch("main", true)).unwrap();

    assert!(previewed.valid, "{:?}", previewed.errors);
    assert!(!previewed.patch.is_empty(), "the preview shows what would change");
    assert_eq!(session.project().history().refs().head, "darker", "HEAD moved on a dry run");
    assert_eq!(song_text(&dir.0), on_darker, "the document changed on a dry run");

    // And the commit that follows still lands where the caller is, not where the preview
    // pointed.
    session.set_tempo(&SetTempoRequest { bpm: 90.0, tick: 0, dry_run: false }).unwrap();
    assert_eq!(session.project().history().refs().head, "darker");
}

#[test]
fn a_dry_run_switch_previews_the_patch_the_real_one_applies() {
    let (_dir, mut session) = opened();
    add_a_track(&mut session, "Bass");
    session.create_branch(&branch("darker", "", false)).unwrap();
    session.switch_branch(&switch("darker", false)).unwrap();
    session.set_tempo(&SetTempoRequest { bpm: 88.0, tick: 0, dry_run: false }).unwrap();

    let previewed = session.switch_branch(&switch("main", true)).unwrap();
    let applied = session.switch_branch(&switch("main", false)).unwrap();

    assert_eq!(applied.patch, previewed.patch);
}

#[test]
fn switching_to_a_branch_that_does_not_exist_is_refused() {
    let (_dir, mut session) = opened();
    let result = session.switch_branch(&switch("nope", false)).unwrap();
    assert_eq!(rules(&result), vec!["ref_missing"]);
}

// ---- delete ----

#[test]
fn a_deleted_branch_leaves_its_entries_inert() {
    // ADR 0001 §2: discarding a branch discards the *name*. Entries are small, and an entry
    // nothing references is exactly what a crash leaves behind, so the store already tolerates
    // them.
    let (_dir, mut session) = opened();
    session.create_branch(&branch("scratch", "", false)).unwrap();
    session.switch_branch(&switch("scratch", false)).unwrap();
    add_a_track(&mut session, "Experiment");
    let entries = session.project().history().entries().len();

    session.switch_branch(&switch("main", false)).unwrap();
    let result = session.delete_branch(&DeleteBranchRequest {
        name: "scratch".to_string(),
        dry_run: false,
    })
    .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    assert!(!session.project().history().refs().refs.contains_key("scratch"));
    assert_eq!(session.project().history().entries().len(), entries, "the entries remain");
}

#[test]
fn deleting_the_branch_head_is_on_is_refused() {
    let (_dir, mut session) = opened();
    let result = session
        .delete_branch(&DeleteBranchRequest { name: "main".to_string(), dry_run: false })
        .unwrap();

    assert_eq!(rules(&result), vec!["delete_head"]);
    assert!(session.project().history().refs().refs.contains_key("main"));
}

#[test]
fn a_dry_run_delete_deletes_nothing() {
    let (_dir, mut session) = opened();
    session.create_branch(&branch("scratch", "", false)).unwrap();

    let result = session
        .delete_branch(&DeleteBranchRequest { name: "scratch".to_string(), dry_run: true })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    assert!(session.project().history().refs().refs.contains_key("scratch"));
}

#[test]
fn a_dry_run_create_creates_nothing() {
    let (_dir, mut session) = opened();
    let result = session.create_branch(&branch("scratch", "", true)).unwrap();

    assert!(result.valid, "{:?}", result.errors);
    assert!(!session.project().history().refs().refs.contains_key("scratch"));
}

// ---- on disk ----

#[test]
fn branch_state_survives_a_reopen() {
    let (dir, mut session) = opened();
    add_a_track(&mut session, "Bass");
    session.create_branch(&branch("darker", "", false)).unwrap();
    session.switch_branch(&switch("darker", false)).unwrap();
    session.set_tempo(&SetTempoRequest { bpm: 88.0, tick: 0, dry_run: false }).unwrap();

    let reopened = Project::open(&dir.0).unwrap();
    assert_eq!(reopened.song(), session.project().song());
    assert_eq!(reopened.history().refs().head, "darker");
    assert_eq!(reopened.history().refs().refs.len(), 2);
}

#[test]
fn two_identical_branching_sessions_produce_identical_projects() {
    let build = || {
        let (dir, mut session) = opened();
        add_a_track(&mut session, "Bass");
        session.create_branch(&branch("darker", "", false)).unwrap();
        session.switch_branch(&switch("darker", false)).unwrap();
        session.set_tempo(&SetTempoRequest { bpm: 88.0, tick: 0, dry_run: false }).unwrap();
        session.switch_branch(&switch("main", false)).unwrap();
        (song_text(&dir.0), std::fs::read_to_string(dir.0.join("refs.json")).unwrap())
    };
    assert_eq!(build(), build());
}
