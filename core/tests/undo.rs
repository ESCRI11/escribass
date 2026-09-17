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
    ApplyPatchRequest, CreateBranchRequest, MergeBranchRequest, MergeSide, RedoRequest,
    SwitchBranchRequest, ToolResult, UndoRequest,
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
fn undo_after_a_redo_keeps_going_backwards_rather_than_forwards() {
    // The log's own `undo` and `redo` entries are not changes a person made, and walking them
    // as though they were takes the document forward: the inverse of an inverse is the thing
    // itself. Found by pressing the key four times, which is the sequence edit · ⌘Z · ⇧⌘Z · ⌘Z
    // · ⌘Z — and the fifth press is where it showed.
    let (_dir, mut session) = opened();
    set_gain(&mut session, -7.5);
    undo(&mut session);
    redo(&mut session);
    assert_eq!(gain(&session), -7.5, "the redo put it back");

    undo(&mut session);
    assert_eq!(gain(&session), -6.5, "and ⌘Z takes it away again");
    let refused = session.undo(&UndoRequest { dry_run: false }).unwrap();
    assert!(
        !refused.valid,
        "there is one edit in this history, so the second press has nothing left — walking the \
         undo entries instead would have restored -7.5, which is forwards"
    );
    assert_eq!(refused.errors[0].rule, "nothing_to_undo");
    assert_eq!(gain(&session), -6.5);
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
fn a_redo_never_reaches_into_another_line_of_history() {
    // `version` counts per branch (ADR 0005 §4's caveat), so a redo that restored a document
    // from another line would put that line's numbers on this one. What prevents it is not a
    // cursor cleared on the way across — there is none to clear — but that what can be redone
    // is read off *this* branch's first-parent chain, which the other line's undo is not on.
    let (_dir, mut session) = opened();
    let root = session.get_history().refs.unwrap().refs["main"].clone();
    set_gain(&mut session, -7.5);
    undo(&mut session);
    session
        .create_branch(&CreateBranchRequest {
            name: "elsewhere".to_string(),
            at_entry_id: root,
            dry_run: false,
        })
        .unwrap();
    let switch = |session: &mut Session, name: &str| {
        let switched = session
            .switch_branch(&SwitchBranchRequest { name: name.to_string(), dry_run: false })
            .unwrap();
        assert!(switched.valid, "{:?}", switched.errors);
    };

    switch(&mut session, "elsewhere");
    assert_eq!(
        session.redo(&RedoRequest { dry_run: false }).unwrap().errors[0].rule,
        "nothing_to_redo",
        "`elsewhere` branched before the edit, so nothing on it was ever undone"
    );

    // And going back is not a loss: main's log still says its edit was undone, so ⇧⌘Z there
    // still puts it back. A session-held cursor forgot this on the way across.
    switch(&mut session, "main");
    let redone = redo(&mut session);
    assert!(redone.valid, "{:?}", redone.errors);
    assert_eq!(gain(&session), -7.5);
}

// ---- across sessions: the log is the cursor (ADR 0005 §4, amended 2026-09-15) ----
//
// Every test above runs in one `Session`, and so did every undo test before M2 PR 11 — which is
// why a cursor that lived in the session, empty in every fresh one, passed all of them while ⌘Z
// in a reopened window re-applied an edit it had already reversed. Each test here closes the
// project and opens it again, as quitting and relaunching the app does.

/// The same project in a new session, as a relaunched process would open it: the first session
/// is gone and nothing of it survives but what it wrote.
fn reopened(dir: &Scratch, session: Session) -> Session {
    drop(session);
    let project = Project::open(&dir.0, manifest()).expect("the project reopens");
    // A counter far past anything an earlier session minted: ids are never reused (§4.3), and a
    // fresh `SeededIds::default()` would mint the first session's entry ids again.
    static RELAUNCHES: AtomicUsize = AtomicUsize::new(1);
    let ids = SeededIds::new(AT, 1_000_000 * RELAUNCHES.fetch_add(1, Ordering::Relaxed) as u64);
    Session::new(project, Box::new(ids), Box::new(FixedClock(AT)), Author::Model)
}

#[test]
fn a_fresh_session_does_not_undo_forwards_past_undos_it_did_not_make() {
    // The review's reproduction, as a person meets it: edit, edit, ⌘Z, ⌘Z, close the window,
    // reopen, ⌘Z. Both edits are already reversed, so there is nothing left to undo. With the
    // cursor empty the walk started at `HEAD`, skipped both undo entries, landed on the second
    // edit — already undone — and restored the document *before* it: the first edit came back,
    // recorded under the tool name `undo`.
    let (dir, mut session) = opened();
    set_gain(&mut session, -7.5);
    set_gain(&mut session, -3.0);
    undo(&mut session);
    undo(&mut session);
    assert_eq!(gain(&session), -6.5);

    let mut session = reopened(&dir, session);
    let before = entries(&session);
    let refused = session.undo(&UndoRequest { dry_run: false }).unwrap();
    assert_eq!(gain(&session), -6.5, "an undo may never bring back an edit");
    assert!(!refused.valid, "both edits are undone: {}", refused.summary);
    assert_eq!(refused.errors[0].rule, "nothing_to_undo");
    assert_eq!(entries(&session), before, "and nothing was written");
}

#[test]
fn a_fresh_session_undoes_the_change_still_in_effect() {
    // One prior undo, then a relaunch. The walk from `HEAD` skipped the undo entry and landed
    // on the edit it had already reversed, so the fresh ⌘Z restored the document that was
    // already there: "no change", no entry, and the key silently did nothing.
    let (dir, mut session) = opened();
    set_gain(&mut session, -7.5);
    set_gain(&mut session, -3.0);
    undo(&mut session);
    assert_eq!(gain(&session), -7.5);

    let mut session = reopened(&dir, session);
    let undone = undo(&mut session);
    assert!(undone.valid, "{:?}", undone.errors);
    assert_eq!(gain(&session), -6.5, "the first edit, which is the one still in effect, goes");
    assert!(!undone.entry_id.is_empty(), "and the press recorded an entry: {}", undone.summary);
}

#[test]
fn redo_crosses_sessions_too() {
    // The same defect pointed the other way: `redo` read the session's list of undone entries
    // and a fresh session has none, so everything undone before a relaunch could never be
    // put back — while the log beside it said exactly what had been undone.
    let (dir, mut session) = opened();
    set_gain(&mut session, -7.5);
    set_gain(&mut session, -3.0);
    undo(&mut session);
    undo(&mut session);

    let mut session = reopened(&dir, session);
    let redone = redo(&mut session);
    assert!(redone.valid, "{:?}", redone.errors);
    assert_eq!(gain(&session), -7.5, "the change undone last comes back first");

    // Across a second relaunch, mid-way, for the same reason.
    let mut session = reopened(&dir, session);
    redo(&mut session);
    assert_eq!(gain(&session), -3.0);
    assert_eq!(
        session.redo(&RedoRequest { dry_run: false }).unwrap().errors[0].rule,
        "nothing_to_redo",
        "and then everything is back"
    );
}

// ---- what the log says, which a session-held cursor got wrong in one session too ----

#[test]
fn an_edit_made_after_an_undo_is_never_walked_back_into() {
    // edit A · edit B · ⌘Z · edit C. B was undone and then superseded: C is the edit that
    // replaced it, which is what "any other commit clears redo" means. So ⌘Z twice is C then A.
    //
    // The session-held walk skipped the undo entry and landed on B, restored the document
    // before B — which, with C already reversed, was the document already there — and called
    // the press "no change". Worse, it pushed B onto its list, so ⇧⌘Z later restored B: a
    // document with B and without C, which this history never contained after B was undone.
    let (_dir, mut session) = opened();
    set_gain(&mut session, -7.5); // A
    set_gain(&mut session, -3.0); // B
    undo(&mut session);
    set_gain(&mut session, -1.0); // C

    undo(&mut session);
    assert_eq!(gain(&session), -7.5, "C is reversed");
    let second = undo(&mut session);
    assert_eq!(gain(&session), -6.5, "and then A — not B again: {}", second.summary);

    redo(&mut session);
    assert_eq!(gain(&session), -7.5, "A is back");
    redo(&mut session);
    assert_eq!(gain(&session), -1.0, "and then C, never B");
}

#[test]
fn a_merge_that_changed_nothing_is_not_a_change_to_undo() {
    // Resolving every conflict in this branch's favour records a merge entry with no
    // operations, because what it records is the join (ADR 0015 §3, extended). Reversing it
    // restores the document already there, so a cursor read off the log could never move past
    // it and ⌘Z would answer "no change" for ever. It is skipped like the log's own undo and
    // redo entries, since ⌘Z means the change before this one and it changed nothing.
    let (_dir, mut session) = opened();
    set_gain(&mut session, -7.5);
    session
        .create_branch(&CreateBranchRequest {
            name: "other".to_string(),
            at_entry_id: String::new(),
            dry_run: false,
        })
        .unwrap();
    set_gain(&mut session, -3.0);
    let switch = |session: &mut Session, name: &str| {
        let switched = session
            .switch_branch(&SwitchBranchRequest { name: name.to_string(), dry_run: false })
            .unwrap();
        assert!(switched.valid, "{:?}", switched.errors);
    };
    switch(&mut session, "other");
    set_gain(&mut session, -1.0);
    switch(&mut session, "main");

    let merge = |session: &mut Session, resolve| {
        session
            .merge_branch(&MergeBranchRequest { name: "other".to_string(), dry_run: false, resolve })
            .unwrap()
    };
    let conflicts = merge(&mut session, Default::default());
    assert!(!conflicts.valid, "both sides wrote the gain");
    let ours = conflicts.errors.iter().map(|e| (e.path.clone(), MergeSide::Ours as i32)).collect();
    let merged = merge(&mut session, ours);
    assert!(merged.valid, "{:?}", merged.errors);
    assert!(!merged.entry_id.is_empty(), "the join is recorded");
    assert_eq!(gain(&session), -3.0, "and it kept this branch's value");

    let undone = undo(&mut session);
    assert_eq!(gain(&session), -7.5, "⌘Z reversed the last change, not the join: {}", undone.summary);
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
