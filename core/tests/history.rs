//! On-disk shape tests for `patches/*.json` and `refs.json`.
//!
//! The fixtures are written by this test from typed values, never by hand, exactly as
//! `schema/tests/roundtrip.rs` writes the song fixture.

use escribass_core::{
    check_refs, entry_from_json, entry_to_json, ops_of, ops_text, refs_from_json,
    refs_to_json, timestamp_from_ms, Op,
};
use escribass_schema::history::{PatchEntry, Refs};
use escribass_schema::song::{Author, Provenance};
use escribass_schema::SCHEMA_VERSION;
use serde_json::{json, Value};

const ENTRY_FIXTURE: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/history/patch_entry.json");
const REFS_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/history/refs.json");

const CREATED_AT: i64 = 1_788_307_200_000;
const ENTRY_ID: &str = "01M1FPMP00PTCHSTNTS00000016";
const PARENT_ID: &str = "01M1FPMP00CPCHRS0000000006";

fn provenance() -> Provenance {
    Provenance {
        author: Author::Model as i32,
        model_id: Some("anthropic/claude-opus-5".to_string()),
        prompt_id: None,
        tool_call_id: Some("call_01B".to_string()),
        created_at: Some(timestamp_from_ms(CREATED_AT)),
    }
}

fn ops() -> Vec<Op> {
    serde_json::from_value(json!([
        {
            "op": "replace",
            "path": "/clips/01M1FPMP00CPCHRS0000000006/note_clip/notes/01M1FPMP00NTEG100000000007/pitch",
            "value": 43
        },
        { "op": "remove", "path": "/clips/01M1FPMP00CPCHRS0000000006/note_clip/notes/01M1FPMP00NTED200000000008" }
    ]))
    .unwrap()
}

/// Built with every field named and no `..Default::default()`, deliberately: adding a field
/// to `history.proto` then breaks this test's *compilation*, forcing the disk shape to be
/// updated in the same change instead of silently ceasing to be persisted.
fn sample_entry() -> PatchEntry {
    PatchEntry {
        id: ENTRY_ID.to_string(),
        parents: vec![PARENT_ID.to_string()],
        tool: "set_notes".to_string(),
        ops: ops_text(&ops()).into_bytes(),
        provenance: Some(provenance()),
        schema_version: SCHEMA_VERSION,
    }
}

fn sample_refs() -> Refs {
    Refs {
        head: "main".to_string(),
        refs: [
            ("main".to_string(), ENTRY_ID.to_string()),
            ("try-darker-chorus".to_string(), PARENT_ID.to_string()),
        ]
        .into_iter()
        .collect(),
    }
}

fn read_or_write(path: &str, actual: &str) -> String {
    if std::env::var("UPDATE_FIXTURES").is_ok() {
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
        std::fs::write(path, actual).unwrap();
    }
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e}\nrun with UPDATE_FIXTURES=1 to create it"))
}

// ---- the shape ----

#[test]
fn an_entry_matches_its_fixture() {
    let actual = entry_to_json(&sample_entry()).unwrap();
    assert_eq!(actual, read_or_write(ENTRY_FIXTURE, &actual));
}

#[test]
fn ops_are_a_literal_rfc_6902_array_not_base64() {
    // The trap this module exists to avoid: `PatchEntry.ops` is `bytes`, and the generated
    // serde impl base64-encodes it. That file would round-trip cleanly and still violate
    // ADR 0001 §1 and §2.6.
    let text = entry_to_json(&sample_entry()).unwrap();
    let parsed: Value = serde_json::from_str(&text).unwrap();

    assert!(parsed["ops"].is_array(), "ops must be an array, got {}", parsed["ops"]);
    assert_eq!(parsed["ops"][0]["op"], "replace");
    assert_eq!(parsed["ops"][0]["value"], 43);
    assert!(
        text.contains("\"op\": \"replace\""),
        "the file must be readable and diffable (§2.6):\n{text}"
    );
}

#[test]
fn the_entry_keeps_the_field_order_adr_0001_specifies() {
    let text = entry_to_json(&sample_entry()).unwrap();
    let at = |k: &str| text.find(k).unwrap_or_else(|| panic!("{k} missing from:\n{text}"));
    for pair in ["\"id\"", "\"parents\"", "\"tool\"", "\"ops\"", "\"provenance\"", "\"schema_version\""]
        .windows(2)
    {
        assert!(at(pair[0]) < at(pair[1]), "{} must precede {}", pair[0], pair[1]);
    }
}

#[test]
fn provenance_is_written_by_the_generated_impl_so_it_matches_song_json() {
    let text = entry_to_json(&sample_entry()).unwrap();
    assert!(text.contains("\"author\": \"AUTHOR_MODEL\""), "{text}");
    assert!(text.contains("\"created_at\": \"2026-09-02T00:00:00+00:00\""), "{text}");
    // An unset optional stays absent, exactly as in song.json (ADR 0002 §5).
    assert!(!text.contains("prompt_id"), "{text}");
}

#[test]
fn an_entry_round_trips() {
    let original = sample_entry();
    let parsed = entry_from_json(&entry_to_json(&original).unwrap()).unwrap();
    assert_eq!(parsed, original);
    assert_eq!(ops_of(&parsed).unwrap(), ops());
    // And writing what was read reproduces the same bytes.
    assert_eq!(entry_to_json(&parsed).unwrap(), entry_to_json(&original).unwrap());
}

#[test]
fn an_entry_without_provenance_is_refused() {
    let mut bad = sample_entry();
    bad.provenance = None;
    assert_eq!(entry_to_json(&bad).unwrap_err().rule, "provenance_missing");
}

#[test]
fn an_unknown_member_is_rejected_rather_than_dropped() {
    let text = entry_to_json(&sample_entry()).unwrap().replace("\"tool\":", "\"tempo\": 120,\n  \"tool\":");
    assert_eq!(entry_from_json(&text).unwrap_err().rule, "entry_unreadable");
}

// ---- refs ----

#[test]
fn refs_match_their_fixture_and_round_trip() {
    let actual = refs_to_json(&sample_refs());
    assert_eq!(actual, read_or_write(REFS_FIXTURE, &actual));
    assert_eq!(refs_from_json(&actual).unwrap(), sample_refs());
    assert!(actual.find("\"main\"").unwrap() < actual.find("\"try-darker-chorus\"").unwrap());
}

#[test]
fn a_well_formed_set_of_refs_has_no_violations() {
    assert_eq!(check_refs(&sample_refs()), vec![]);
}

#[test]
fn every_ref_name_rule_from_adr_0001_is_checked() {
    let with = |name: &str| {
        let mut r = sample_refs();
        r.refs.insert(name.to_string(), ENTRY_ID.to_string());
        check_refs(&r).into_iter().map(|v| v.rule).collect::<Vec<_>>()
    };
    assert!(with("Main").contains(&"ref_name_charset"), "uppercase is not in the charset");
    assert!(with("has space").contains(&"ref_name_charset"));
    assert!(with("/leading").contains(&"ref_name_slash"));
    assert!(with("trailing/").contains(&"ref_name_slash"));
    assert!(with("a/../b").contains(&"ref_name_dotdot"));
    assert!(with("").contains(&"ref_name_empty"));
}

#[test]
fn two_refs_differing_only_by_case_collide() {
    // Harmless in refs.json, but indistinguishable to a reader and a collision if the store
    // ever moved to one file per ref (ADR 0001 §2).
    let mut refs = sample_refs();
    refs.refs.insert("MAIN".to_string(), ENTRY_ID.to_string());
    let rules: Vec<_> = check_refs(&refs).into_iter().map(|v| v.rule).collect();
    assert!(rules.contains(&"ref_name_case_collision"), "{rules:?}");
}

#[test]
fn head_always_names_an_existing_ref() {
    let mut dangling = sample_refs();
    dangling.head = "gone".to_string();
    assert!(check_refs(&dangling).iter().any(|v| v.rule == "head_dangling"));

    let mut unset = sample_refs();
    unset.head.clear();
    assert!(check_refs(&unset).iter().any(|v| v.rule == "head_unset"));
}
