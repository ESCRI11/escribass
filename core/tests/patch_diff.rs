//! Diff tests.
//!
//! The property that matters is the round trip: `apply(a, diff(a, b)) == b`, driven over
//! seeded mutations of the real fixture rather than toy documents, because the fixture is the
//! shape `core` will actually diff.

use escribass_core::{apply, diff, to_canonical_json, validate, Op};
use escribass_schema::song::Song;
use serde_json::{json, Value};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");

fn fixture() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).expect("fixture")).unwrap()
}

// ---- shape ----

#[test]
fn an_unchanged_document_produces_no_ops() {
    assert_eq!(diff(&fixture(), &fixture()), vec![]);
    assert_eq!(diff(&json!({"a": 1}), &json!({"a": 1})), vec![]);
}

#[test]
fn it_recurses_to_the_leaf_rather_than_replacing_a_subtree() {
    // The point of the granularity: M0.3's merge compares op paths, so two edits inside one
    // track must produce two disjoint paths, not two collisions on /tracks/{id}.
    let before = json!({"tracks": {"t1": {"name": "Bass", "index": 0}}});
    let after = json!({"tracks": {"t1": {"name": "Sub", "index": 0}}});
    assert_eq!(
        diff(&before, &after),
        vec![Op::Replace { path: "/tracks/t1/name".to_string(), value: json!("Sub") }]
    );
}

#[test]
fn added_and_removed_keys_become_add_and_remove() {
    let before = json!({"a": 1, "gone": 2});
    let after = json!({"a": 1, "fresh": 3});
    assert_eq!(
        diff(&before, &after),
        vec![
            Op::Remove { path: "/gone".to_string() },
            Op::Add { path: "/fresh".to_string(), value: json!(3) },
        ]
    );
}

#[test]
fn keys_needing_escapes_are_escaped() {
    let d = diff(&json!({"a/b": 1}), &json!({"a/b": 2}));
    assert_eq!(d, vec![Op::Replace { path: "/a~1b".to_string(), value: json!(2) }]);
    // And the escape survives the round trip through apply.
    assert_eq!(apply(&json!({"a/b": 1}), &d).unwrap(), json!({"a/b": 2}));
}

#[test]
fn op_order_is_deterministic() {
    let (a, mut b) = (fixture(), fixture());
    b["version"] = json!(999);
    b["tracks"]["01M1FPMP00TRACKBASS0000002"]["name"] = json!("Sub");
    assert_eq!(diff(&a, &b), diff(&a, &b));
}

// ---- the round trip, over seeded mutations of the fixture ----

/// Deterministic mutations of a document: no wall clock, no unseeded randomness, no
/// `proptest` dependency (CLAUDE.md #4), following the `SeededIds` precedent.
struct Mutator(u64);

impl Mutator {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    /// Every pointer in the document, leaves and objects alike.
    fn paths(value: &Value, path: &mut String, out: &mut Vec<String>) {
        out.push(path.clone());
        if let Value::Object(fields) = value {
            for (key, child) in fields {
                let len = path.len();
                path.push('/');
                path.push_str(&key.replace('~', "~0").replace('/', "~1"));
                Self::paths(child, path, out);
                path.truncate(len);
            }
        }
    }

    /// Applies one random change: retype a value, delete a key, or insert one.
    fn mutate(&mut self, doc: &Value) -> Value {
        let mut all = Vec::new();
        Self::paths(doc, &mut String::new(), &mut all);
        all.retain(|p| !p.is_empty());
        let target = &all[(self.next() as usize) % all.len()];

        let op = match self.next() % 3 {
            0 => Op::Replace { path: target.clone(), value: json!({"replaced": self.next()}) },
            1 => Op::Remove { path: target.clone() },
            _ => Op::Add { path: format!("{target}/inserted"), value: json!(self.next()) },
        };
        // `add` under a scalar is not a legal patch; skip those rather than assert on them.
        apply(doc, std::slice::from_ref(&op)).unwrap_or_else(|_| doc.clone())
    }
}

#[test]
fn applying_a_diff_reproduces_the_target_exactly() {
    let mut rng = Mutator(0x2545_F491_4F6C_DD1D);
    let base = fixture();
    for round in 0..500 {
        let mut mutated = base.clone();
        for _ in 0..(1 + round % 5) {
            mutated = rng.mutate(&mutated);
        }
        let patch = diff(&base, &mutated);
        let rebuilt = apply(&base, &patch)
            .unwrap_or_else(|e| panic!("round {round}: diff produced an inapplicable patch: {e}"));
        assert_eq!(rebuilt, mutated, "round {round}: patch did not reproduce the target");

        // And the reverse direction, which is what a branch switch actually runs.
        assert_eq!(apply(&mutated, &diff(&mutated, &base)).unwrap(), base, "round {round} reverse");
    }
}

#[test]
fn a_diff_between_two_real_songs_round_trips_through_song_and_validates() {
    // The end-to-end shape a commit takes: edit, diff, apply, back to Song, validate.
    let before = fixture();
    let mut after = before.clone();
    after["tracks"]["01M1FPMP00TRACKBASS0000002"]["mix"]["gain_db"] = json!(-3.0);
    after["clips"]["01M1FPMP00CPCHRS0000000006"]["note_clip"]["notes"]
        ["01M1FPMP00NTEG100000000007"]["pitch"] = json!(45);

    let patch = diff(&before, &after);
    assert_eq!(patch.len(), 2, "two edits, two ops: {patch:?}");

    let applied = apply(&before, &patch).unwrap();
    assert_eq!(applied, after, "the patched document equals the edited one");

    // The step that matters: back through Song. This is what catches an op that is legal
    // JSON but illegal for the schema — `43.0` at an int32 path (ADR 0002 §11).
    let song: Song = serde_json::from_value(applied).expect("a patched document is still a Song");
    assert_eq!(validate(&song), vec![]);
    assert_eq!(song.tracks["01M1FPMP00TRACKBASS0000002"].mix.as_ref().unwrap().gain_db, -3.0);
    assert!(to_canonical_json(&song).is_ok());
}

#[test]
fn the_fixture_still_contains_no_arrays() {
    // diff replaces an array wholesale rather than erroring, which is safe but coarse. That
    // is only acceptable while the document cannot contain one (ADR 0001 §3); this asserts
    // the assumption rather than trusting it.
    fn find(v: &Value, path: &mut String, out: &mut Vec<String>) {
        match v {
            Value::Array(_) => out.push(path.clone()),
            Value::Object(fields) => {
                for (k, child) in fields {
                    let len = path.len();
                    path.push('/');
                    path.push_str(k);
                    find(child, path, out);
                    path.truncate(len);
                }
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    find(&fixture(), &mut String::new(), &mut found);
    assert!(found.is_empty(), "arrays appeared in the document: {found:?}");
}
