//! Tests for the injectable id source and clock.
//!
//! The point of both is §11: no unseeded randomness and no wall clock inside `core`. What
//! that buys is asserted here — a seeded run is reproducible, and every id it mints is a
//! ULID the validator accepts.

mod common;
use common::manifest;

use escribass_core::{validate, Clock, FixedClock, IdSource, SeededIds, SystemClock, UlidSource};
use escribass_core::{from_canonical_json, timestamp_from_ms};
use escribass_schema::song::*;

const CROCKFORD: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");

fn is_ulid(id: &str) -> bool {
    id.len() == 26 && id.chars().all(|c| CROCKFORD.contains(c))
}

#[test]
fn seeded_ids_are_reproducible() {
    // The property the determinism suite (M0.4) depends on: same seed, same ids, so canonical
    // JSON can be compared byte for byte with no normalisation step.
    let run = || {
        let mut ids = SeededIds::default();
        (0..50).map(|_| ids.next_id()).collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

#[test]
fn seeded_ids_differ_by_seed() {
    let take = |seed| {
        let mut ids = SeededIds::new(1_788_307_200_000, seed);
        ids.next_id()
    };
    assert_ne!(take(1), take(2));
}

#[test]
fn every_minted_id_is_a_ulid_the_validator_accepts() {
    let mut seeded = SeededIds::default();
    let mut real = UlidSource::default();
    for _ in 0..1_000 {
        for id in [seeded.next_id(), real.next_id()] {
            assert!(is_ulid(&id), "`{id}` is not a ULID");
        }
    }

    // The end-to-end check: a minted id, put in a song, passes `id_not_ulid`.
    let mut song = from_canonical_json(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    let marker_id = UlidSource::default().next_id();
    song.markers.insert(
        marker_id.clone(),
        Marker {
            id: marker_id,
            provenance: song.provenance.clone(),
            version: 1,
            name: "Drop".to_string(),
            tick: 61_440,
        },
    );
    assert_eq!(validate(&song, &manifest()), vec![]);
}

#[test]
fn ids_are_unique_and_sort_in_creation_order() {
    // The tail increments within a millisecond, so ids minted in one burst stay ordered —
    // which is what makes canonical JSON map keys chronological (ADR 0002 §4).
    let mut source = UlidSource::default();
    let ids: Vec<_> = (0..10_000).map(|_| source.next_id()).collect();

    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "ids must sort in creation order");

    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "ids must never repeat");
}

#[test]
fn a_frozen_clock_makes_ids_deterministic_without_seeding_them() {
    // Two UlidSources on the same fixed clock share a timestamp half but not the random tail:
    // freezing time alone is not reproducibility, which is why SeededIds exists.
    let mut a = UlidSource::new(FixedClock(1_788_307_200_000));
    let mut b = UlidSource::new(FixedClock(1_788_307_200_000));
    let (x, y) = (a.next_id(), b.next_id());
    assert_eq!(x[..10], y[..10], "same millisecond, same timestamp half");
    assert_ne!(x, y, "different tails");
}

#[test]
fn timestamps_are_whole_milliseconds() {
    // ADR 0002 §4 stores millisecond precision, and the validator rejects finer. Producing it
    // at the clock means nothing downstream has to truncate.
    for ms in [0_i64, 1, 999, 1_788_307_200_123, -1, -1_001] {
        let t = timestamp_from_ms(ms);
        assert_eq!(t.nanos % 1_000_000, 0, "{ms} produced sub-millisecond nanos");
        assert!((0..1_000_000_000).contains(&t.nanos), "{ms} produced out-of-range nanos");
    }
    assert_eq!(timestamp_from_ms(1_788_307_200_000).seconds, 1_788_307_200);
    assert_eq!(timestamp_from_ms(1_788_307_200_123).nanos, 123_000_000);
}

#[test]
fn the_fixed_clock_returns_what_it_was_given_and_the_system_clock_moves() {
    assert_eq!(FixedClock(42).now_ms(), 42);
    assert_eq!(FixedClock(42).now().seconds, 0);
    assert_eq!(FixedClock(42).now().nanos, 42_000_000);

    // Sanity, not precision: the real clock is past 2020 and its nanos are whole ms.
    let now = SystemClock.now();
    assert!(now.seconds > 1_577_836_800, "system clock looks wrong: {now:?}");
    assert_eq!(now.nanos % 1_000_000, 0);
}
