//! Patch DAG tests: ancestry, replay, and refs (ADR 0001 §1, §2; ADR 0004).

use escribass_core::{
    diff, entry, timestamp_from_ms, validate, History, IdSource, Op, SeededIds,
};
use escribass_schema::song::{Author, Provenance, Song};
use serde_json::{json, Value};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");
const BASS: &str = "01M1FPMP00TRACKBASS0000002";

fn fixture() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).expect("fixture")).unwrap()
}

fn empty_song() -> Value {
    serde_json::to_value(Song::default()).unwrap()
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

/// A history whose root builds the fixture from a default song, plus one edit on top.
fn chain() -> (History, SeededIds, Value) {
    let mut ids = SeededIds::default();
    let mut log = History::new();

    let root_id = ids.next_id();
    let root = entry(root_id.clone(), vec![], "create", &diff(&empty_song(), &fixture()), provenance(), 1);
    log.append(root).unwrap();
    log.create_ref("main", &root_id).unwrap();
    log.set_head("main").unwrap();

    let mut louder = fixture();
    louder["tracks"][BASS]["mix"]["gain_db"] = json!(-3.0);
    let next_id = ids.next_id();
    log.append(entry(next_id.clone(), vec![root_id], "set_param", &diff(&fixture(), &louder), provenance(), 1))
        .unwrap();
    log.advance("main", &next_id).unwrap();

    (log, ids, louder)
}

// ---- replay ----

#[test]
fn replay_starts_from_a_default_song_not_an_empty_object() {
    // ADR 0002 §4 emits every no-presence field, so `replace` is legal from the first op.
    // Starting from `{}` would make the root patch a pile of `add`s that diverge from what
    // the tool API produces.
    let (log, _, _) = chain();
    let root = log.entries().keys().next().unwrap().clone();
    assert_eq!(log.materialise(&root).unwrap(), fixture());
}

#[test]
fn a_chain_replays_to_the_document_it_recorded() {
    let (log, _, expected) = chain();
    let head = log.head_id().unwrap().to_string();
    assert_eq!(log.materialise(&head).unwrap(), expected);

    // And the result is still a valid song, which is the whole point of replaying rather
    // than trusting a cached document (ADR 0004).
    let song: Song = serde_json::from_value(log.materialise(&head).unwrap()).unwrap();
    assert_eq!(validate(&song), vec![]);
}

#[test]
fn materialising_twice_gives_the_same_bytes() {
    let (log, _, _) = chain();
    let head = log.head_id().unwrap().to_string();
    assert_eq!(log.materialise(&head).unwrap(), log.materialise(&head).unwrap());
}

// ---- the shape of the graph ----

#[test]
fn a_fork_gives_two_independent_documents() {
    let (mut log, mut ids, _) = chain();
    let base = log.head_id().unwrap().to_string();

    let mut darker = log.materialise(&base).unwrap();
    darker["tracks"][BASS]["name"] = json!("Sub");
    let side = ids.next_id();
    log.append(entry(side.clone(), vec![base.clone()], "set_param",
        &diff(&log.materialise(&base).unwrap(), &darker), provenance(), 1)).unwrap();
    log.create_ref("try-darker-chorus", &side).unwrap();

    assert_eq!(log.materialise(&side).unwrap()["tracks"][BASS]["name"], json!("Sub"));
    assert_eq!(log.materialise(&base).unwrap()["tracks"][BASS]["name"], json!("Bass"));
}

#[test]
fn a_diamond_replays_every_entry_exactly_once_and_in_a_stable_order() {
    // A merge entry has two parents, so ancestors form a DAG rather than a chain and the
    // shared root must not be applied twice.
    let (mut log, mut ids, _) = chain();
    let base = log.head_id().unwrap().to_string();
    let doc = log.materialise(&base).unwrap();

    let mut left_doc = doc.clone();
    left_doc["tracks"][BASS]["name"] = json!("Left");
    let left = ids.next_id();
    log.append(entry(left.clone(), vec![base.clone()], "set_param", &diff(&doc, &left_doc), provenance(), 1)).unwrap();

    let mut right_doc = doc.clone();
    right_doc["version"] = json!(900);
    let right = ids.next_id();
    log.append(entry(right.clone(), vec![base.clone()], "set_param", &diff(&doc, &right_doc), provenance(), 1)).unwrap();

    // The merge carries what the other side changed, and names both parents.
    let mut merged = left_doc.clone();
    merged["version"] = json!(900);
    let merge = ids.next_id();
    log.append(entry(merge.clone(), vec![left.clone(), right.clone()], "merge_branch",
        &diff(&left_doc, &merged), provenance(), 1)).unwrap();

    let order: Vec<&str> = log.ancestry(&merge).unwrap().iter().map(|e| e.id.as_str()).collect();
    assert_eq!(order.len(), 5, "root, edit, left, right, merge — each once: {order:?}");
    let mut unique = order.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), order.len(), "an entry was replayed twice");

    // Parents always precede their children.
    let at = |id: &str| order.iter().position(|x| *x == id).unwrap();
    assert!(at(&left) < at(&merge) && at(&right) < at(&merge) && at(&base) < at(&left));
    assert_eq!(log.materialise(&merge).unwrap(), merged);
    assert_eq!(log.ancestry(&merge).unwrap().iter().map(|e| &e.id).collect::<Vec<_>>(),
               log.ancestry(&merge).unwrap().iter().map(|e| &e.id).collect::<Vec<_>>());
}

// ---- errors rather than panics ----

#[test]
fn an_entry_naming_a_parent_that_is_not_in_the_log_is_refused() {
    let (mut log, mut ids, _) = chain();
    let orphan = entry(ids.next_id(), vec!["01M1FPMP00MSSNG00000000001".to_string()],
        "set_param", &[], provenance(), 1);
    assert_eq!(log.append(orphan).unwrap_err().rule, "parent_missing");
}

#[test]
fn a_cycle_cannot_be_built_at_all() {
    // Not a test of cycle *detection* but of its impossibility: append refuses an entry whose
    // parent is absent, so neither half of a two-entry cycle can be inserted first.
    let mut log = History::new();
    let (a, b) = ("01M1FPMP00AAAA0000000000001", "01M1FPMP00BBBB0000000000001");
    assert_eq!(
        log.append(entry(a, vec![b.to_string()], "x", &[], provenance(), 1)).unwrap_err().rule,
        "parent_missing"
    );
    assert_eq!(
        log.append(entry(b, vec![a.to_string()], "x", &[], provenance(), 1)).unwrap_err().rule,
        "parent_missing"
    );
}

#[test]
fn an_id_is_never_reused() {
    let (mut log, _, _) = chain();
    let existing = log.head_id().unwrap().to_string();
    let clash = entry(existing, vec![], "x", &[], provenance(), 1);
    assert_eq!(log.append(clash).unwrap_err().rule, "entry_exists");
}

#[test]
fn materialising_an_unknown_entry_is_an_error() {
    let (log, _, _) = chain();
    assert_eq!(log.materialise("01M1FPMP00MSSNG00000000001").unwrap_err().rule, "entry_missing");
}

// ---- refs ----

#[test]
fn switching_moves_head_and_appends_nothing() {
    let (mut log, mut ids, _) = chain();
    let base = log.head_id().unwrap().to_string();
    let doc = log.materialise(&base).unwrap();

    let mut darker = doc.clone();
    darker["tracks"][BASS]["name"] = json!("Sub");
    let side = ids.next_id();
    log.append(entry(side.clone(), vec![base], "set_param", &diff(&doc, &darker), provenance(), 1)).unwrap();
    log.create_ref("try-darker-chorus", &side).unwrap();

    let before = log.entries().len();
    let ops = log.switch("try-darker-chorus", &doc).unwrap();

    assert_eq!(log.entries().len(), before, "switching is navigation, not history");
    assert_eq!(log.refs().head, "try-darker-chorus");
    assert_eq!(escribass_core::apply(&doc, &ops).unwrap(), darker, "the patch reaches the branch");

    // And back again, byte for byte.
    let back = log.switch("main", &darker).unwrap();
    assert_eq!(escribass_core::apply(&darker, &back).unwrap(), doc);
}

#[test]
fn discarding_a_branch_leaves_its_entries_inert() {
    let (mut log, mut ids, _) = chain();
    let base = log.head_id().unwrap().to_string();
    let side = ids.next_id();
    log.append(entry(side.clone(), vec![base], "set_param", &[], provenance(), 1)).unwrap();
    log.create_ref("scratch", &side).unwrap();

    let count = log.entries().len();
    log.delete_ref("scratch").unwrap();
    assert!(log.refs().refs.get("scratch").is_none());
    assert_eq!(log.entries().len(), count, "entries stay on disk, unreferenced (ADR 0001 §2)");
    assert!(log.get(&side).is_some());
}

#[test]
fn head_always_names_a_ref() {
    let (mut log, _, _) = chain();
    assert_eq!(log.delete_ref("main").unwrap_err().rule, "delete_head");
    assert_eq!(log.set_head("nope").unwrap_err().rule, "ref_missing");
    assert_eq!(log.switch("nope", &fixture()).unwrap_err().rule, "ref_missing");
}

#[test]
fn a_ref_name_that_breaks_adr_0001_is_refused() {
    let (mut log, _, _) = chain();
    let head = log.head_id().unwrap().to_string();
    assert_eq!(log.create_ref("Scratch", &head).unwrap_err().rule, "ref_name_charset");
    assert_eq!(log.create_ref("/leading", &head).unwrap_err().rule, "ref_name_slash");
    assert_eq!(log.create_ref("main", &head).unwrap_err().rule, "ref_exists");
    assert_eq!(log.advance("ghost", &head).unwrap_err().rule, "ref_missing");
}

#[test]
fn a_ref_can_only_point_at_an_entry_that_exists() {
    let (mut log, _, _) = chain();
    assert_eq!(
        log.create_ref("scratch", "01M1FPMP00MSSNG00000000001").unwrap_err().rule,
        "entry_missing"
    );
}

#[test]
fn an_op_that_no_longer_applies_names_the_entry() {
    // A hand-mangled log should say which entry broke, not fail somewhere in serde.
    let mut log = History::new();
    let bad: Vec<Op> = serde_json::from_value(json!([{"op": "replace", "path": "/nope", "value": 1}])).unwrap();
    let id = "01M1FPMP00BROKEN000000000001";
    log.append(entry(id, vec![], "x", &bad, provenance(), 1)).unwrap();
    let e = log.materialise(id).unwrap_err();
    assert_eq!(e.rule, "replay_failed");
    assert!(e.message.contains(id), "{}", e.message);
}
