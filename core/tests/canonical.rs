//! Canonical persistence tests.
//!
//! The claim under test is §11's: same project file → identical bytes. The fixed-point test
//! below is that claim in one assertion, against the fixture the schema crate generates.

use escribass_core::{from_canonical_json, to_canonical_json, CanonicalError};
use escribass_schema::song::*;
use std::collections::BTreeMap;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");

fn fixture_text() -> String {
    std::fs::read_to_string(FIXTURE).expect("fixture must exist; see tests/AGENTS.md")
}

fn song_with_gain(gain_db: f64) -> Song {
    let track = Track {
        id: "01K4F2T001".to_string(),
        mix: Some(Mix { gain_db, pan: 0.0, mute: false, solo: false }),
        ..Default::default()
    };
    Song {
        id: "01K4F2S000".to_string(),
        schema_version: 1,
        tracks: [(track.id.clone(), track)].into_iter().collect(),
        ..Default::default()
    }
}

#[test]
fn the_fixture_is_a_fixed_point() {
    // Load, write, and get the same bytes. This is the whole determinism claim: whatever
    // `core` writes, reading it and writing it again changes nothing (§11).
    let text = fixture_text();
    let song = from_canonical_json(&text).expect("the fixture must parse");
    assert_eq!(to_canonical_json(&song).unwrap(), text);
}

#[test]
fn writing_is_idempotent_across_a_second_cycle() {
    let once = to_canonical_json(&from_canonical_json(&fixture_text()).unwrap()).unwrap();
    let twice = to_canonical_json(&from_canonical_json(&once).unwrap()).unwrap();
    assert_eq!(once, twice);
}

#[test]
fn core_agrees_with_the_schema_crates_serialisation() {
    // The fixture is written by the schema crate's test with a plain `to_string_pretty`.
    // If `core` ever serialises through `serde_json::Value`, struct fields sort
    // alphabetically and this fails — which is the point of the assertion.
    let song = from_canonical_json(&fixture_text()).unwrap();
    let ours = to_canonical_json(&song).unwrap();
    let theirs = format!("{}\n", serde_json::to_string_pretty(&song).unwrap());
    assert_eq!(ours, theirs);
    assert!(ours.find("\"id\"").unwrap() < ours.find("\"schema_version\"").unwrap());
}

#[test]
fn non_finite_doubles_are_rejected_and_the_error_names_the_field() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let err = to_canonical_json(&song_with_gain(bad)).unwrap_err();
        assert_eq!(
            err,
            CanonicalError::Unrepresentable { path: "/tracks/01K4F2T001/mix/gain_db".to_string() },
            "{bad} must be rejected at its path"
        );
        assert!(err.to_string().contains("gain_db"));
    }
}

#[test]
fn a_finite_song_is_accepted() {
    let text = to_canonical_json(&song_with_gain(-6.5)).unwrap();
    assert!(text.contains("\"gain_db\": -6.5"));
    assert!(text.ends_with("}\n"), "exactly one trailing newline");
}

#[test]
fn doubles_survive_the_round_trip_bit_for_bit() {
    // The §11 property at the level `core` actually writes: every finite double that goes
    // into a project file comes back out identical. Fails without serde_json's
    // `float_roundtrip` (ADR 0002 §4). Seeded LCG — no wall clock, no unseeded randomness.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut checked = 0;
    for _ in 0..20_000 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let v = f64::from_bits(state);
        if !v.is_finite() {
            continue;
        }
        let text = to_canonical_json(&song_with_gain(v)).unwrap();
        let back = from_canonical_json(&text).unwrap();
        let got = back.tracks["01K4F2T001"].mix.as_ref().unwrap().gain_db;
        assert_eq!(got.to_bits(), v.to_bits(), "{v:e} did not survive as {text}");
        checked += 1;
    }
    assert!(checked > 15_000, "expected a meaningful sample, got {checked}");
}

#[test]
fn negative_zero_reaches_the_file_unchanged() {
    // Deliberate: the writer serialises faithfully rather than rewriting values behind the
    // model's back. Normalising -0.0 is the tool API's job on input, and the validator
    // rejects it — both M0.2/M0.3, neither this module. Pins the current behaviour so the
    // change is visible when it lands.
    let text = to_canonical_json(&song_with_gain(-0.0)).unwrap();
    assert!(text.contains("\"gain_db\": -0.0"));
}

#[test]
fn unknown_fields_are_rejected_rather_than_dropped() {
    let mut text = fixture_text();
    text = text.replace("\"schema_version\": 1,", "\"schema_version\": 1,\n  \"tempo_bpm\": 120,");
    assert!(from_canonical_json(&text).is_err());
}

#[test]
fn map_keys_are_written_in_sorted_order() {
    let mut tracks = BTreeMap::new();
    for id in ["01K4F2T009", "01K4F2T001", "01K4F2T005"] {
        tracks.insert(
            id.to_string(),
            Track { id: id.to_string(), mix: Some(Mix::default()), ..Default::default() },
        );
    }
    let text = to_canonical_json(&Song { tracks, ..Default::default() }).unwrap();
    let at = |k: &str| text.find(k).unwrap();
    assert!(at("01K4F2T001") < at("01K4F2T005") && at("01K4F2T005") < at("01K4F2T009"));
}
