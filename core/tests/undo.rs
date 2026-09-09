//! `undo` and `redo` (ADR 0005 §4, §5's tool list).
//!
//! ADR 0005 decided the mechanism a milestone before there was a caller, and the argument it
//! made is the thing to test: undo **appends an inverse entry** rather than rewinding a ref,
//! because a rewind decrements entity `version` and §4.3's optimistic concurrency needs it
//! monotonic. So the claims here are not "the gain went back to what it was" — that is the
//! easy half — but that the log grew, the reversed entry is still in it, and no number any
//! client could be holding went backwards.
//!
//! Every project is built by `Project::create` and changed only through `Session`
//! (CLAUDE.md #2).

mod common;
use common::manifest;

use escribass_core::{ops_of, FixedClock, Project, SeededIds, Session};
use escribass_proto::tools::{
    ApplyPatchRequest, CreateBranchRequest, RedoRequest, SwitchBranchRequest, ToolResult,
    UndoRequest,
};
use escribass_schema::song::{Author, Song};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");
const BASS: &str = "01M1FPMP00TRACKBASS0000002";
const AT: i64 = 1_788_307_200_000;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-undo-{}-{}.escri",
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
    let song: Song =
        serde_json::from_str(&std::fs::read_to_string(FIXTURE).expect("fixture")).unwrap();
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let project =
        Project::create(&dir.0, &song, &mut ids, &clock, Author::Human, manifest()).unwrap();
    (dir, Session::new(project, Box::new(ids), Box::new(clock), Author::Model))
}

/// One `apply_patch`, applied. The tool API is the only way to change a song here.
fn set_gain(session: &mut Session, to: f64) -> ToolResult {
    let ops = json!([{
        "op": "replace", "path": format!("/tracks/{BASS}/mix/gain_db"), "value": to
    }]);
    let request = ApplyPatchRequest { patch: serde_json::to_vec(&ops).unwrap(), dry_run: false };
    let result = session.apply_patch(&request).expect("an ordinary edit");
    assert!(result.valid, "{:?}", result.errors);
    result
}

fn gain(session: &Session) -> f64 {
    let song = session.get_song().song.unwrap();
    song.tracks[BASS].mix.as_ref().unwrap().gain_db
}

fn undo(session: &mut Session) -> ToolResult {
    session.undo(&UndoRequest { dry_run: false }).expect("undo is not an operator error")
}

fn redo(session: &mut Session) -> ToolResult {
    session.redo(&RedoRequest { dry_run: false }).expect("redo is not an operator error")
}

/// Every entity's `version`, keyed by its id, read out of the document as it stands.
///
/// A walk over `Value` rather than over `Song` for the reason `bump_versions` is one: the rule
/// is a statement about a shape — an object with a string `id` and a numeric `version` — and
/// ten typed implementations of it drift as entities are added (ADR 0005 §2).
fn versions(session: &Session) -> BTreeMap<String, u64> {
    fn walk(value: &Value, into: &mut BTreeMap<String, u64>) {
        match value {
            Value::Object(map) => {
                if let (Some(Value::String(id)), Some(version)) = (map.get("id"), map.get("version"))
                {
                    if let Some(number) = version.as_u64() {
                        into.insert(id.clone(), number);
                    }
                }
                map.values().for_each(|held| walk(held, into));
            }
            Value::Array(items) => items.iter().for_each(|held| walk(held, into)),
            _ => {}
        }
    }
    let mut found = BTreeMap::new();
    walk(&serde_json::to_value(session.get_song().song.unwrap()).unwrap(), &mut found);
    found
}

/// The ids in the log, so a test can say the old entry is still there.
fn entries(session: &Session) -> Vec<String> {
    session.get_history().entries.keys().cloned().collect()
}

// ---- the mechanism (ADR 0005 §4) ----

#[test]
fn undo_appends_an_inverse_entry_rather_than_removing_the_one_it_reverses() {
    let (_dir, mut session) = opened();
    let edit = set_gain(&mut session, -7.5);
    let before = entries(&session);
    assert!(before.contains(&edit.entry_id), "the edit is in the log");

    let undone = undo(&mut session);
    assert!(undone.valid, "{:?}", undone.errors);
    assert_eq!(gain(&session), -6.5, "the document went back");

    let after = entries(&session);
    assert!(after.contains(&edit.entry_id), "the reversed entry is still in the log (§5's audit trail)");
    assert!(after.contains(&undone.entry_id), "and the inverse entry is beside it");
    assert_eq!(after.len(), before.len() + 1, "the log grew; nothing was rewound");

    // The inverse entry names the entry it reversed as its parent, so the chain is a chain and
    // `HEAD` moved forward. A rewind would have moved it back to `edit`'s parent.
    let history = session.get_history();
    let entry = &history.entries[&undone.entry_id];
    assert_eq!(entry.parents, vec![edit.entry_id.clone()]);
    assert_eq!(entry.tool, "undo", "the audit trail says what happened, not what unhappened");
    assert_eq!(history.refs.unwrap().refs["main"], undone.entry_id, "main advanced");
}

#[test]
fn undo_leaves_every_version_monotonic() {
    let (_dir, mut session) = opened();
    let start = versions(&session);
    set_gain(&mut session, -7.5);
    let edited = versions(&session);
    assert!(edited[BASS] > start[BASS], "an ordinary edit bumps");

    undo(&mut session);
    let undone = versions(&session);
    for (id, number) in &edited {
        assert!(
            undone.get(id).copied().unwrap_or(0) >= *number,
            "`{id}` went backwards, from {number} to {:?} — which is what ADR 0005 §4 refused \
             the rewind for: a client holding one number would later be handed it again over \
             different content",
            undone.get(id)
        );
    }
    assert_eq!(undone[BASS], edited[BASS] + 1, "the inverse is an edit like any other");

    // And again through a redo, since redo is the same operation pointed the other way.
    redo(&mut session);
    let redone = versions(&session);
    for (id, number) in &undone {
        assert!(redone.get(id).copied().unwrap_or(0) >= *number, "`{id}` went backwards on redo");
    }
    assert_eq!(gain(&session), -7.5, "and the content came back");
}

#[test]
fn the_recorded_entry_is_the_inverse_patch_and_not_a_repeat_of_the_original() {
    let (_dir, mut session) = opened();
    set_gain(&mut session, -7.5);
    let undone = undo(&mut session);

    let history = session.get_history();
    let ops = ops_of(&history.entries[&undone.entry_id]).expect("the entry's ops");
    let gain_op = ops
        .iter()
        .find(|op| op.path().ends_with("/mix/gain_db"))
        .expect("the inverse writes the gain back");
    assert_eq!(
        serde_json::to_value(gain_op).unwrap()["value"],
        json!(-6.5),
        "the entry restores the old value"
    );
    // The same patch the caller was shown, byte for byte: `ToolResult.patch` is what §9 has a
    // person approve, and an entry that said something else would make that approval a fiction.
    let shown: Value = serde_json::from_slice(&undone.patch).expect("the patch is an array");
    let recorded: Value = serde_json::from_str(&escribass_core::ops_text(&ops)).unwrap();
    assert_eq!(shown, recorded);
}

// ---- the cursor (ADR 0005 §4's session-held stack) ----

#[test]
fn a_second_undo_walks_further_back_rather_than_undoing_the_undo() {
    let (_dir, mut session) = opened();
    set_gain(&mut session, -7.5);
    set_gain(&mut session, -3.0);

    undo(&mut session);
    assert_eq!(gain(&session), -7.5, "one press: back one edit");
    undo(&mut session);
    assert_eq!(
        gain(&session),
        -6.5,
        "two presses: back two edits. Reading HEAD again instead of the cursor would find the \
         first undo entry, and reversing that puts -3.0 back — a redo wearing ⌘Z's clothes"
    );

    redo(&mut session);
    assert_eq!(gain(&session), -7.5);
    redo(&mut session);
    assert_eq!(gain(&session), -3.0);
    assert_eq!(
        session.redo(&RedoRequest { dry_run: false }).unwrap().errors[0].rule,
        "nothing_to_redo",
        "the cursor is back at the tip"
    );
}

#[test]
fn any_other_commit_clears_what_could_be_redone() {
    let (_dir, mut session) = opened();
    set_gain(&mut session, -7.5);
    undo(&mut session);
    set_gain(&mut session, -1.0);

    let refused = session.redo(&RedoRequest { dry_run: false }).unwrap();
    assert!(!refused.valid);
    assert_eq!(refused.errors[0].rule, "nothing_to_redo");
    assert_eq!(gain(&session), -1.0, "and the edit that cleared it is untouched");
}

#[test]
fn switching_branch_clears_it_too() {
    let (_dir, mut session) = opened();
    set_gain(&mut session, -7.5);
    undo(&mut session);
    session
        .create_branch(&CreateBranchRequest {
            name: "elsewhere".to_string(),
            at_entry_id: String::new(),
            dry_run: false,
        })
        .unwrap();
    let switched = session
        .switch_branch(&SwitchBranchRequest { name: "elsewhere".to_string(), dry_run: false })
        .unwrap();
    assert!(switched.valid);

    // `version` counts per branch (ADR 0005 §4's caveat), so a redo here would restore a
    // document from another line of history onto this one.
    assert_eq!(
        session.redo(&RedoRequest { dry_run: false }).unwrap().errors[0].rule,
        "nothing_to_redo"
    );
}

// ---- the two refusals, and the dry run ----

#[test]
fn undo_at_the_beginning_of_a_history_is_a_refusal_and_not_a_failure() {
    let (_dir, mut session) = opened();
    // A fresh project has exactly one entry, and it has no parent.
    let refused = session.undo(&UndoRequest { dry_run: false }).unwrap();
    assert!(!refused.valid);
    assert_eq!(refused.errors[0].rule, "nothing_to_undo");
    assert!(refused.patch.is_empty());
    assert_eq!(entries(&session).len(), 1, "and nothing was written");
}

#[test]
fn a_dry_run_shows_the_patch_it_would_record_and_writes_nothing() {
    let (_dir, mut session) = opened();
    set_gain(&mut session, -7.5);
    let before = entries(&session);

    let previewed = session.undo(&UndoRequest { dry_run: true }).unwrap();
    assert!(previewed.valid, "{:?}", previewed.errors);
    assert!(previewed.entry_id.is_empty(), "a preview mints no entry");
    assert_eq!(entries(&session), before, "and appends nothing");
    assert_eq!(gain(&session), -7.5, "the document is where it was");

    // ADR 0006 §3: a dry run is the first half of the apply path, not a second implementation
    // of it — so the two patches are the same bytes, and the preview did not consume the undo.
    let applied = undo(&mut session);
    assert_eq!(previewed.patch, applied.patch);
    assert_eq!(gain(&session), -6.5);
}
