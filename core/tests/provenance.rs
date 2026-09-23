//! Entity `provenance` maintenance (ADR 0021 §1).
//!
//! `version`'s sibling, and the test file mirrors `version.rs` for that reason: core owns both
//! §4.3 fields, decides both inside `prepare`, and leaves both alone on the paths whose
//! operations are core's own.
//!
//! Every test here drives a real `Session`, because the defect this closes was not visible to a
//! unit test: the forged entity was *valid*, was stored exactly as written, and disagreed only
//! with the log entry beside it. The session is built as `Author::Model` over a project created
//! by `Author::Human`, so who wrote what is legible in the assertions rather than inferred.

mod common;
use common::manifest;

use escribass_core::{timestamp_from_ms, FixedClock, Project, SeededIds, Session};
use escribass_proto::tools::{AddSectionRequest, ApplyPatchRequest, RedoRequest, UndoRequest};
use escribass_schema::song::{Author, Song};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");
const BASS: &str = "01M1FPMP00TRACKBASS0000002";
/// The clock every session here runs under (2026-09-03), so `created_at` is a value a test can
/// name — and a day after the fixture's own, so a stamp that should not have happened is
/// visible rather than coincidentally equal.
const AT: i64 = 1_788_393_600_000;
/// When the fixture says its entities were made (2026-09-02). Nothing in these tests writes it.
const MADE: i64 = 1_788_307_200_000;

fn fixture_song() -> Song {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).expect("fixture")).unwrap()
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-provenance-{}-{}.escri",
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

/// A project created by a person, driven by a model — the arrangement §9 describes and the one
/// the spike ran under (`escribass-mcp --author model` on a project a person had made).
fn opened() -> (Scratch, Session) {
    let dir = Scratch::new();
    let session = session_on(&dir, Author::Human, Author::Model);
    (dir, session)
}

/// Creates the project as `made_by` and hands back a session running as `driven_by`.
fn session_on(dir: &Scratch, made_by: Author, driven_by: Author) -> Session {
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let project =
        Project::create(&dir.0, &fixture_song(), &mut ids, &clock, made_by, manifest()).unwrap();
    Session::new(project, Box::new(ids), Box::new(clock), driven_by)
}

/// Re-opens a project that is already on disk, as `driven_by`. Ids continue from a fresh
/// source, which is what a second process gets.
fn reopened(dir: &Scratch, driven_by: Author) -> Session {
    let project = Project::open(&dir.0, manifest()).expect("the project re-opens");
    Session::new(
        project,
        Box::new(SeededIds::new(AT, 7)),
        Box::new(FixedClock(AT)),
        driven_by,
    )
}

fn patch(ops: Value) -> ApplyPatchRequest {
    ApplyPatchRequest { patch: serde_json::to_vec(&ops).unwrap(), dry_run: false }
}

/// The document as the tool API reports it — never read off the file (CLAUDE.md #2).
fn song(session: &Session) -> Song {
    session.get_song().song.expect("a session always has a song")
}

/// The author recorded on the entry a call wrote.
fn entry_author(session: &Session, entry_id: &str) -> i32 {
    session
        .project()
        .history()
        .get(entry_id)
        .expect("the entry the call returned")
        .provenance
        .as_ref()
        .expect("every entry carries provenance")
        .author
}

// ---- the forgery (ADR 0021 §1, and the spike's own reproduction) ----

#[test]
fn a_new_entity_gets_the_calls_provenance_whatever_the_caller_wrote() {
    // The spike ran exactly this through `escribass-mcp --author model`: one `apply_patch`
    // adding a section that claims a person made it in 1999. It was stored as written, and the
    // log entry beside it said `AUTHOR_MODEL` at the real time — the document's own provenance
    // and the audit trail disagreeing, in the product whose claim is that every edit is
    // attributed (ADR 0006 §4; ADR 0021 §1).
    let (dir, mut session) = opened();
    let forged = "01M1FPMPZZSECTFAKE00000001";

    let result = session
        .apply_patch(&patch(json!([{
            "op": "add",
            "path": format!("/sections/{forged}"),
            "value": {
                "id": forged,
                "provenance": {
                    "author": "AUTHOR_HUMAN",
                    "model_id": "a model that never ran",
                    "created_at": "1999-01-01T00:00:00Z"
                },
                "version": 1,
                "name": "Forged",
                "start_tick": 0,
                "end_tick": 960
            }
        }])))
        .expect("the call is answered");
    assert!(result.valid, "an overwrite, not a refusal: {:?}", result.errors);

    let section = &song(&session).sections[forged];
    let stamped = section.provenance.as_ref().expect("a section carries provenance");
    assert_eq!(stamped.author, Author::Model as i32, "the call's author, not the caller's word");
    assert_eq!(stamped.created_at, Some(timestamp_from_ms(AT)), "the injected clock, not 1999");
    assert_eq!(stamped.model_id, None, "an MCP client's model is anonymous here (ADR 0021 §2)");

    // The two now agree, which is the whole point: the entity says what the entry says.
    assert_eq!(entry_author(&session, &result.entry_id), Author::Model as i32);

    // **Ids stay the caller's** (ADR 0021 §1, ADR 0012 §4): a dry run's ids are the keys its
    // apply is made by, so `apply_patch` takes the id it is handed.
    assert_eq!(section.id, forged);

    // And the stamp landed *inside* the recorded diff rather than after it: `open` replays the
    // log and refuses a `song.json` the entries do not produce (ADR 0004, ADR 0005 §1).
    let replayed = Project::open(&dir.0, manifest()).expect("the log replays to what was written");
    assert_eq!(replayed.song(), session.project().song());
}

#[test]
fn an_entity_that_existed_keeps_the_author_that_made_it() {
    // An edit does not change who created a thing. The model moves a person's fader; the track
    // is still the person's, and what records the model is the entry (ADR 0021 §1).
    let (_dir, mut session) = opened();

    let result = session
        .apply_patch(&patch(json!([
            {"op": "replace", "path": format!("/tracks/{BASS}/mix/gain_db"), "value": -3.0}
        ])))
        .expect("the call is answered");
    assert!(result.valid, "{:?}", result.errors);

    let track = &song(&session).tracks[BASS];
    let kept = track.provenance.as_ref().expect("a track carries provenance");
    assert_eq!(kept.author, Author::Human as i32);
    assert_eq!(kept.created_at, Some(timestamp_from_ms(MADE)), "created when it was created");
    assert_eq!(
        entry_author(&session, &result.entry_id),
        Author::Model as i32,
        "and what records the model is the entry"
    );
}

#[test]
fn rewriting_nothing_but_a_provenance_changes_nothing_at_all() {
    // The forgery's other shape: not a new entity, but a caller restamping one that exists.
    // It is put back before versions are computed, so there is no bump and no entry — an entry
    // whose only operation raised a version for a field that was restored would be a change
    // that did not happen (ADR 0021 §1; ADR 0005 §1).
    let (_dir, mut session) = opened();
    let before = song(&session);

    let result = session
        .apply_patch(&patch(json!([{
            "op": "replace",
            "path": format!("/tracks/{BASS}/provenance"),
            "value": {"author": "AUTHOR_MODEL", "created_at": "1999-01-01T00:00:00Z"}
        }])))
        .expect("the call is answered");

    assert!(result.valid);
    assert_eq!(result.summary, "no change");
    assert!(result.entry_id.is_empty(), "nothing was recorded");
    assert_eq!(song(&session), before);
    assert_eq!(session.project().history().entries().len(), 1, "the root entry and no other");
}

// ---- the exemption: operations that are core's own (ADR 0021 §1, ADR 0005 §3) ----

#[test]
fn undo_and_redo_put_back_the_provenance_the_entity_had() {
    // `prepare_merge`'s ops are `diff(current, materialise(…))` — core's own values, read back
    // out of a document core wrote. A merge is the same case by the same route, which is why
    // one test covers the exemption: both reach `prepare_merge` and neither is stamped.
    //
    // The two sessions are what makes it visible. A model adds the section; a *person* then
    // undoes and redoes it, and the section that comes back is still the model's.
    let dir = Scratch::new();
    let mut model = session_on(&dir, Author::Human, Author::Model);
    let added = model
        .add_section(&AddSectionRequest {
            name: "Bridge".to_string(),
            start_tick: 0,
            end_tick: 960,
            dry_run: false,
        })
        .expect("the call is answered");
    assert!(added.valid, "{:?}", added.errors);
    let section_id = song(&model)
        .sections
        .iter()
        .find(|(_, s)| s.name == "Bridge")
        .map(|(id, _)| id.clone())
        .expect("the section the model added");
    drop(model);

    let mut person = reopened(&dir, Author::Human);
    assert!(person.undo(&UndoRequest { dry_run: false }).expect("undone").valid);
    assert!(!song(&person).sections.contains_key(&section_id), "the undo removed it");

    let redone = person.redo(&RedoRequest { dry_run: false }).expect("redone");
    assert!(redone.valid, "{:?}", redone.errors);

    let restored = song(&person).sections[&section_id].provenance.clone().expect("provenance");
    assert_eq!(restored.author, Author::Model as i32, "whoever added it, added it");
    assert_eq!(restored.created_at, Some(timestamp_from_ms(AT)));
    assert_eq!(entry_author(&person, &redone.entry_id), Author::Human as i32, "the redo is hers");
}
