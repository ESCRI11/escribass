//! `merge_branch` (ADR 0001 §4).
//!
//! The rule is small on purpose: disjoint paths auto-resolve, the same path is a structured
//! error. These tests are mostly about where the line falls and what happens on each side of
//! it — and about the two things a merge could get quietly wrong, `version` resolution and an
//! auto-merge that produces an invalid song.

use escribass_core::{new_song, FixedClock, Project, SeededIds, Session};
use escribass_proto::tools::add_clip_request::Content as AddClipContent;
use escribass_proto::tools::{
    AddClipRequest, AddSectionRequest, AddTrackRequest, CreateBranchRequest, MergeBranchRequest,
    SetNotesRequest, SetTempoRequest, SwitchBranchRequest, ToolResult, TransposeRequest,
};
use escribass_schema::song::{Author, DeviceRef, Note, NoteClip, SourceRef, TrackKind};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const AT: i64 = 1_788_307_200_000;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-merge-{}-{}.escri",
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

fn cmajor() -> Option<DeviceRef> {
    Some(DeviceRef {
        kind: Some(escribass_schema::song::device_ref::Kind::Cmajor(SourceRef {
            source_hash: "8b31c0de4f9c00000000000000000000".to_string(),
        })),
    })
}

fn opened() -> (Scratch, Session) {
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock, Author::Model);
    let project = Project::create(&dir.0, &song, &mut ids, &clock, Author::Human).unwrap();
    (dir, Session::new(project, Box::new(ids), Box::new(clock), Author::Model))
}

fn track(session: &mut Session, name: &str) -> String {
    let result = session
        .add_track(&AddTrackRequest {
            name: name.to_string(),
            kind: TrackKind::Instrument as i32,
            r#ref: cmajor(),
            dry_run: false,
        })
        .unwrap();
    assert!(result.valid, "{:?}", result.errors);
    session.project().song().tracks.values().max_by_key(|t| t.index).unwrap().id.clone()
}

fn branch(session: &mut Session, name: &str) {
    let made = session
        .create_branch(&CreateBranchRequest {
            name: name.to_string(),
            at_entry_id: String::new(),
            dry_run: false,
        })
        .unwrap();
    assert!(made.valid, "{:?}", made.errors);
}

fn switch(session: &mut Session, name: &str) {
    let moved = session
        .switch_branch(&SwitchBranchRequest { name: name.to_string(), dry_run: false })
        .unwrap();
    assert!(moved.valid, "{:?}", moved.errors);
}

fn merge(session: &mut Session, name: &str, dry_run: bool) -> ToolResult {
    session
        .merge_branch(&MergeBranchRequest { name: name.to_string(), dry_run })
        .unwrap()
}

fn rules(result: &ToolResult) -> Vec<&str> {
    result.errors.iter().map(|e| e.rule.as_str()).collect()
}

fn tempo(session: &mut Session, bpm: f64) {
    let set = session.set_tempo(&SetTempoRequest { bpm, tick: 0, dry_run: false }).unwrap();
    assert!(set.valid, "{:?}", set.errors);
}

fn note(pitch: i32) -> Note {
    Note { pitch, start_tick: 0, length_ticks: 240, velocity: 100, ..Default::default() }
}

fn clip_note_ids(session: &Session, clip: &str) -> Vec<String> {
    match &session.project().song().clips[clip].content {
        Some(escribass_schema::song::clip::Content::NoteClip(notes)) => {
            notes.notes.keys().cloned().collect()
        }
        other => panic!("not a note clip: {other:?}"),
    }
}

fn section(session: &mut Session, name: &str) {
    let added = session
        .add_section(&AddSectionRequest {
            name: name.to_string(),
            start_tick: 0,
            end_tick: 3840,
            dry_run: false,
        })
        .unwrap();
    assert!(added.valid, "{:?}", added.errors);
}


// ---- the happy path ----

#[test]
fn disjoint_edits_merge_into_one_entry_with_two_parents() {
    let (_dir, mut session) = opened();
    branch(&mut session, "other");

    // main changes the tempo.
    tempo(&mut session, 132.0);
    let ours_head = session.project().history().head_id().unwrap().to_string();

    // `other` adds a section — a different part of the document entirely.
    switch(&mut session, "other");
    section(&mut session, "Verse");
    let theirs_head = session.project().history().head_id().unwrap().to_string();

    switch(&mut session, "main");
    let result = merge(&mut session, "other", false);

    assert!(result.valid, "{:?}", result.errors);
    let entry = session.project().history().get(&result.entry_id).unwrap();
    assert_eq!(entry.parents, vec![ours_head, theirs_head], "one entry, two parents");
    assert_eq!(entry.tool, "merge_branch");

    // Both edits survived.
    let song = session.project().song();
    assert_eq!(song.tempo_map.as_ref().unwrap().events.values().next().unwrap().bpm, 132.0);
    assert_eq!(song.sections.len(), 1);
}

#[test]
fn a_merge_leaves_the_project_readable() {
    // ADR 0004's invariant, over the one entry shape that has two parents.
    let (dir, mut session) = opened();
    branch(&mut session, "other");
    tempo(&mut session, 132.0);
    switch(&mut session, "other");
    section(&mut session, "Verse");
    switch(&mut session, "main");
    merge(&mut session, "other", false);

    let reopened = Project::open(&dir.0).unwrap();
    assert_eq!(reopened.song(), session.project().song());
}

#[test]
fn a_dry_run_merge_writes_nothing_and_previews_the_same_patch() {
    let (dir, mut session) = opened();
    branch(&mut session, "other");
    tempo(&mut session, 132.0);
    switch(&mut session, "other");
    section(&mut session, "Verse");
    switch(&mut session, "main");

    let before = std::fs::read_to_string(dir.0.join("song.json")).unwrap();
    let entries = session.project().history().entries().len();

    let previewed = merge(&mut session, "other", true);
    assert!(previewed.valid, "{:?}", previewed.errors);
    assert!(previewed.entry_id.is_empty());
    assert_eq!(std::fs::read_to_string(dir.0.join("song.json")).unwrap(), before);
    assert_eq!(session.project().history().entries().len(), entries);

    let applied = merge(&mut session, "other", false);
    assert_eq!(applied.patch, previewed.patch);
}

// ---- conflicts ----

#[test]
fn the_same_path_on_both_sides_is_a_structured_error() {
    let (_dir, mut session) = opened();
    branch(&mut session, "other");
    tempo(&mut session, 132.0);
    switch(&mut session, "other");
    tempo(&mut session, 88.0);
    switch(&mut session, "main");

    let result = merge(&mut session, "other", false);

    assert!(!result.valid);
    assert_eq!(rules(&result), vec!["merge_conflict"]);
    let conflict = &result.errors[0];
    assert!(conflict.path.ends_with("/bpm"), "{}", conflict.path);
    // §9 puts this in front of a person, so it says what each side wanted.
    assert!(conflict.message.contains("132"), "{}", conflict.message);
    assert!(conflict.message.contains("88"), "{}", conflict.message);
}

#[test]
fn a_conflict_writes_nothing() {
    let (dir, mut session) = opened();
    branch(&mut session, "other");
    tempo(&mut session, 132.0);
    switch(&mut session, "other");
    tempo(&mut session, 88.0);
    switch(&mut session, "main");

    let before = std::fs::read_to_string(dir.0.join("song.json")).unwrap();
    let head = session.project().history().head_id().unwrap().to_string();
    let entries = session.project().history().entries().len();

    assert!(!merge(&mut session, "other", false).valid);

    assert_eq!(std::fs::read_to_string(dir.0.join("song.json")).unwrap(), before);
    assert_eq!(session.project().history().head_id().unwrap(), head);
    assert_eq!(session.project().history().entries().len(), entries);
}

#[test]
fn editing_inside_what_the_other_branch_removed_is_a_conflict() {
    // The prefix half of the rule. `diff` recurses to leaves, so a "replace this whole object"
    // never appears as one op — but a *removal* does: dropping a note is one `remove` for the
    // whole note. An edit to a field inside it is a different path that lives under the
    // removed one, and comparing paths for equality alone would auto-merge an edit into
    // something that is no longer there.
    let (_dir, mut session) = opened();
    let bass = track(&mut session, "Bass");
    let added = session
        .add_clip(&AddClipRequest {
            track_id: bass,
            start_tick: 0,
            length_ticks: 3840,
            content: Some(AddClipContent::NoteClip(NoteClip {
                notes: [("a".to_string(), note(60)), ("b".to_string(), note(64))]
                    .into_iter()
                    .collect(),
            })),
            dry_run: false,
        })
        .unwrap();
    assert!(added.valid, "{:?}", added.errors);
    let clip = session.project().song().clips.keys().next().unwrap().clone();
    let notes = clip_note_ids(&session, &clip);

    branch(&mut session, "other");

    // main moves the first note.
    let moved = session
        .transpose(&TransposeRequest {
            clip_id: clip.clone(),
            semitones: 2,
            note_ids: vec![notes[0].clone()],
            dry_run: false,
        })
        .unwrap();
    assert!(moved.valid, "{:?}", moved.errors);

    // `other` drops it, by sending a note set that does not include it.
    switch(&mut session, "other");
    let replaced = session
        .set_notes(&SetNotesRequest {
            clip_id: clip.clone(),
            notes: [(notes[1].clone(), note(64))].into_iter().collect(),
            dry_run: false,
        })
        .unwrap();
    assert!(replaced.valid, "{:?}", replaced.errors);

    switch(&mut session, "main");
    let result = merge(&mut session, "other", false);

    assert!(!result.valid, "an edit was merged into a note that had been removed");
    assert_eq!(rules(&result), vec!["merge_conflict"]);
    assert!(result.errors[0].path.contains(&notes[0]), "{}", result.errors[0].path);
}

#[test]
fn version_alone_never_conflicts() {
    // Both branches bump `version` on every entity they touch, so counting it would make every
    // merge conflict by construction — the failure ADR 0001 §4 anticipated.
    let (_dir, mut session) = opened();
    branch(&mut session, "other");
    tempo(&mut session, 132.0); // bumps /version on main
    switch(&mut session, "other");
    section(&mut session, "Verse"); // bumps /version on other
    switch(&mut session, "main");

    let result = merge(&mut session, "other", false);
    assert!(result.valid, "{:?}", result.errors);
}

#[test]
fn a_merged_version_is_higher_than_either_side() {
    // ADR 0005 §2's `max(ours, theirs) + 1`, which is what keeps §4.3's optimistic concurrency
    // honest across a merge: no client can be handed a number it has already seen.
    let (_dir, mut session) = opened();
    branch(&mut session, "other");

    tempo(&mut session, 132.0);
    let ours = session.project().song().version;

    switch(&mut session, "other");
    section(&mut session, "A");
    section(&mut session, "B");
    section(&mut session, "C");
    let theirs = session.project().song().version;
    assert!(theirs > ours, "the branch got further ahead: {theirs} vs {ours}");

    switch(&mut session, "main");
    merge(&mut session, "other", false);

    assert_eq!(session.project().song().version, theirs + 1);
}

// ---- an auto-merge that would produce an invalid song ----

#[test]
fn two_branches_adding_a_track_at_one_index_are_refused() {
    // The deferred `index` item (`docs/plan.md`). The two adds are at different paths, so the
    // merge rule auto-resolves them — and the result has two tracks claiming one position.
    // This is the test that keeps that failure loud: the validator refuses it rather than the
    // project quietly acquiring an ambiguous mixer order.
    let (_dir, mut session) = opened();
    branch(&mut session, "other");

    track(&mut session, "Bass");
    switch(&mut session, "other");
    track(&mut session, "Drums");
    switch(&mut session, "main");

    let result = merge(&mut session, "other", false);

    assert!(!result.valid, "two tracks at one index were merged in");
    assert!(rules(&result).contains(&"track_index_duplicate"), "{:?}", rules(&result));
}

// ---- edges ----

#[test]
fn merging_a_branch_that_is_already_here_does_nothing() {
    let (_dir, mut session) = opened();
    branch(&mut session, "other");

    let result = merge(&mut session, "other", false);
    assert!(result.valid, "{:?}", result.errors);
    assert!(result.entry_id.is_empty(), "nothing to record");
    assert!(result.patch.is_empty());
}

#[test]
fn merging_a_branch_with_nothing_new_records_nothing() {
    let (_dir, mut session) = opened();
    branch(&mut session, "other");
    tempo(&mut session, 132.0); // only main moved

    let result = merge(&mut session, "other", false);
    assert!(result.valid, "{:?}", result.errors);
    assert!(result.entry_id.is_empty());
}

#[test]
fn merging_a_branch_that_does_not_exist_is_refused() {
    let (_dir, mut session) = opened();
    let result = merge(&mut session, "nope", false);
    assert_eq!(rules(&result), vec!["ref_missing"]);
}

#[test]
fn a_merge_is_reproducible() {
    let build = |dir: &Path| {
        let mut ids = SeededIds::default();
        let clock = FixedClock(AT);
        let song = new_song(&mut ids, &clock, Author::Model);
        let project = Project::create(dir, &song, &mut ids, &clock, Author::Human).unwrap();
        let mut session = Session::new(project, Box::new(ids), Box::new(clock), Author::Model);

        branch(&mut session, "other");
        tempo(&mut session, 132.0);
        switch(&mut session, "other");
        section(&mut session, "Verse");
        switch(&mut session, "main");
        merge(&mut session, "other", false);

        std::fs::read_to_string(dir.join("song.json")).unwrap()
    };
    let first = Scratch::new();
    let second = Scratch::new();
    assert_eq!(build(&first.0), build(&second.0));
}

#[test]
fn the_merge_entry_carries_the_ops_that_were_recorded() {
    let (_dir, mut session) = opened();
    branch(&mut session, "other");
    tempo(&mut session, 132.0);
    switch(&mut session, "other");
    section(&mut session, "Verse");
    switch(&mut session, "main");

    let result = merge(&mut session, "other", false);
    let entry = session.project().history().get(&result.entry_id).unwrap();
    let recorded: Value =
        serde_json::from_slice(&escribass_core::ops_text(&escribass_core::ops_of(entry).unwrap()).into_bytes())
            .unwrap();
    let returned: Value = serde_json::from_slice(&result.patch).unwrap();
    assert_eq!(recorded, returned);
}

#[test]
fn a_merge_that_brings_in_a_removal_leaves_a_replayable_log() {
    // The defect that made this review worth running. A merge entry's ops are the diff from
    // `parents[0]`, so replaying *every* ancestor and then the merge applies the incoming
    // side's changes twice. `add` and `replace` are idempotent and hide it; `remove` is not —
    // the second one hits `path_not_found`, and the project will not reopen. Replay follows
    // first parents (ADR 0001 §1, amended).
    let (dir, mut session) = opened();
    let bass = track(&mut session, "Bass");
    let added = session
        .add_clip(&AddClipRequest {
            track_id: bass,
            start_tick: 0,
            length_ticks: 3840,
            content: Some(AddClipContent::NoteClip(NoteClip {
                notes: [("a".to_string(), note(60)), ("b".to_string(), note(64))]
                    .into_iter()
                    .collect(),
            })),
            dry_run: false,
        })
        .unwrap();
    assert!(added.valid, "{:?}", added.errors);
    let clip = session.project().song().clips.keys().next().unwrap().clone();
    let notes = clip_note_ids(&session, &clip);

    branch(&mut session, "other");
    section(&mut session, "Verse");

    switch(&mut session, "other");
    let dropped = session
        .set_notes(&SetNotesRequest {
            clip_id: clip.clone(),
            notes: [(notes[1].clone(), note(64))].into_iter().collect(),
            dry_run: false,
        })
        .unwrap();
    assert!(dropped.valid, "{:?}", dropped.errors);

    switch(&mut session, "main");
    let merged = merge(&mut session, "other", false);
    assert!(merged.valid, "{:?}", merged.errors);

    // Every way the log is read back must agree with the document beside it (ADR 0004).
    let reopened = Project::open(&dir.0).expect("the project reopens");
    assert_eq!(reopened.song(), session.project().song());
    assert_eq!(clip_note_ids(&session, &clip).len(), 1, "the note stayed dropped");
}

#[test]
fn a_plugin_pin_is_not_auto_resolved_by_the_merge_rule() {
    // `PluginRef.version` is a *string* pinning a release (§4.4 requires it pinned). Excluding
    // every path ending `/version` from conflict detection would let one branch's pin silently
    // win — which ADR 0001 §4 says never happens by heuristic. Both branches change only the
    // pin here, so the diff is exactly the path the exemption would have swallowed.
    let (_dir, mut session) = opened();
    let bass = track(&mut session, "Bass");
    let at = format!("/tracks/{bass}/instrument/ref");
    let pin = |v: &str| {
        serde_json::to_vec(&serde_json::json!([{
            "op": "replace",
            "path": format!("{at}/plugin/version"),
            "value": v
        }]))
        .unwrap()
    };
    let apply = |session: &mut Session, patch: Vec<u8>| {
        session
            .apply_patch(&escribass_proto::tools::ApplyPatchRequest { patch, dry_run: false })
            .unwrap()
    };

    // Put a plugin there first, so both branches change only its version.
    let plugin = serde_json::to_vec(&serde_json::json!([{
        "op": "replace",
        "path": at,
        "value": {"plugin": {"plugin_id": "com.surge-synth.surge-xt", "version": "1.0.0"}}
    }]))
    .unwrap();
    assert!(apply(&mut session, plugin).valid);

    branch(&mut session, "other");
    assert!(apply(&mut session, pin("2.0.0")).valid);
    switch(&mut session, "other");
    assert!(apply(&mut session, pin("3.0.0")).valid);
    switch(&mut session, "main");

    let result = merge(&mut session, "other", false);

    assert!(!result.valid, "two different pins were merged without a word");
    assert_eq!(rules(&result), vec!["merge_conflict"]);
    assert!(result.errors[0].path.ends_with("/plugin/version"), "{}", result.errors[0].path);
}

#[test]
fn a_merge_beats_a_branch_that_is_exactly_one_version_ahead() {
    // `max(L, L+1) + 1` is `L + 2`. A rule that preferred the ordinary bump whenever the caller
    // had already stated it returned `L + 1` — a number the other branch had handed out for
    // different content, which is the repeat §4.3's concurrency check cannot survive.
    let (_dir, mut session) = opened();
    section(&mut session, "Verse");
    let id = session.project().song().sections.keys().next().unwrap().clone();
    let at = |s: &Session| s.project().song().sections[&id].version;
    // Disjoint paths, so the merge itself resolves; only the numbers are the question.
    let edit = |s: &mut Session, field: &str, value: serde_json::Value| {
        let ops = serde_json::to_vec(&serde_json::json!([
            {"op": "replace", "path": format!("/sections/{id}/{field}"), "value": value}
        ]))
        .unwrap();
        let r = s
            .apply_patch(&escribass_proto::tools::ApplyPatchRequest { patch: ops, dry_run: false })
            .unwrap();
        assert!(r.valid, "{:?}", r.errors);
    };

    branch(&mut session, "other");
    edit(&mut session, "name", serde_json::json!("Chorus")); // main: one edit
    let ours = at(&session);

    switch(&mut session, "other");
    edit(&mut session, "start_tick", serde_json::json!(480));
    edit(&mut session, "start_tick", serde_json::json!(960)); // two edits: exactly one ahead
    let theirs = at(&session);
    assert_eq!(theirs, ours + 1, "the branches are adjacent, which is the case that broke");

    switch(&mut session, "main");
    let merged = merge(&mut session, "other", false);
    assert!(merged.valid, "{:?}", merged.errors);
    assert_eq!(at(&session), theirs + 1, "the merge repeated a number the other branch used");
}
