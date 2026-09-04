//! Entity `version` maintenance (ADR 0005 §1–§3).
//!
//! The rule is one statement about a shape, so most of these test `bump_versions` directly on
//! documents. The last two go through `commit`, because *where* the bump happens is the half
//! of the decision that a unit test cannot see.

use escribass_core::{bump_versions, FixedClock, Op, Project, SeededIds};
use escribass_schema::song::{Author, Song};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");
const BASS: &str = "01M1FPMP00TRACKBASS0000002";
const MASTER: &str = "01M1FPMP00TRACKMASTER00003";
const CLIP: &str = "01M1FPMP00CPCHRS0000000006";
const NOTE: &str = "01M1FPMP00NTEG100000000007";

fn fixture() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).expect("fixture")).unwrap()
}

/// Applies `edit` to a copy of the fixture, bumps, and hands back the result.
fn bumped(edit: impl FnOnce(&mut Value)) -> Value {
    let (patched, disputed) = bumped_with_disputes(edit);
    assert!(disputed.is_empty(), "unexpected dispute: {disputed:?}");
    patched
}

fn bumped_with_disputes(edit: impl FnOnce(&mut Value)) -> (Value, Vec<escribass_core::Violation>) {
    let before = fixture();
    let mut patched = before.clone();
    edit(&mut patched);
    let disputed = bump_versions(&before, &mut patched, false);
    (patched, disputed)
}

fn version_at(doc: &Value, pointer: &str) -> u64 {
    doc.pointer(pointer)
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("no version at {pointer}"))
}

// ---- the rule ----

#[test]
fn a_changed_entity_and_all_its_ancestors_bump() {
    // Editing one note bumps the note, its clip and the song — §4.3's optimistic concurrency
    // is per entity at every level, so a client holding any of the three learns it is stale.
    let after = bumped(|d| {
        d["clips"][CLIP]["note_clip"]["notes"][NOTE]["pitch"] = json!(45);
    });

    assert_eq!(version_at(&after, &format!("/clips/{CLIP}/note_clip/notes/{NOTE}/version")), 2);
    assert_eq!(version_at(&after, &format!("/clips/{CLIP}/version")), 3);
    assert_eq!(version_at(&after, "/version"), 215);
}

#[test]
fn an_untouched_entity_keeps_its_number() {
    let after = bumped(|d| d["tracks"][BASS]["mix"]["gain_db"] = json!(-3.0));

    assert_eq!(version_at(&after, &format!("/tracks/{BASS}/version")), 4);
    assert_eq!(version_at(&after, "/version"), 215);
    // A sibling track, and a device on the changed track that was not itself touched.
    assert_eq!(version_at(&after, &format!("/tracks/{MASTER}/version")), 1);
    assert_eq!(version_at(&after, &format!("/tracks/{BASS}/instrument/version")), 1);
    assert_eq!(version_at(&after, &format!("/clips/{CLIP}/version")), 2);
}

#[test]
fn a_new_entity_lands_at_one() {
    // Tools construct an entity with `version: 0`; the rule takes it from there.
    let after = bumped(|d| {
        d["clips"][CLIP]["note_clip"]["notes"]["01M1FPMP00NTENEW00000000008"] = json!({
            "id": "01M1FPMP00NTENEW00000000008",
            "provenance": {"author": "AUTHOR_MODEL", "created_at": "2026-09-03T00:00:00Z"},
            "version": 0,
            "pitch": 60,
            "microtonal_cents": 0.0,
            "start_tick": 0,
            "length_ticks": 480,
            "velocity": 100,
            "expression": {}
        });
    });

    assert_eq!(
        version_at(&after, &format!("/clips/{CLIP}/note_clip/notes/01M1FPMP00NTENEW00000000008/version")),
        1
    );
    assert_eq!(version_at(&after, &format!("/clips/{CLIP}/version")), 3);
    // The note that was already there did not change.
    assert_eq!(version_at(&after, &format!("/clips/{CLIP}/note_clip/notes/{NOTE}/version")), 1);
}

#[test]
fn a_removal_bumps_the_container_that_lost_it() {
    let after = bumped(|d| {
        d["clips"][CLIP]["note_clip"]["notes"].as_object_mut().unwrap().remove(NOTE);
    });

    assert_eq!(version_at(&after, &format!("/clips/{CLIP}/version")), 3);
    assert_eq!(version_at(&after, "/version"), 215);
    assert_eq!(version_at(&after, &format!("/tracks/{BASS}/version")), 3);
}

#[test]
fn a_merge_resolves_to_the_higher_of_the_two_plus_one_even_when_they_are_adjacent() {
    // The case a single shared rule got wrong. `max(L, L+1) + 1` is `L + 2`; a rule that
    // preferred the ordinary answer whenever the caller had already stated it returned `L + 1`
    // — a number the other branch had handed out for different content.
    let before = fixture();
    let mut patched = before.clone();
    patched["tracks"][BASS]["name"] = json!("Bass (merged)");
    patched["tracks"][BASS]["version"] = json!(4); // exactly one ahead of ours
    assert!(bump_versions(&before, &mut patched, true).is_empty());
    assert_eq!(version_at(&patched, &format!("/tracks/{BASS}/version")), 5);
}

#[test]
fn an_ordinary_edit_ignores_a_number_the_caller_states() {
    // The other half of the split: outside a merge, `now` is not an input. A caller cannot
    // push a version forward by stating a high one.
    let (patched, disputed) = bumped_with_disputes(|d| {
        d["tracks"][BASS]["name"] = json!("Renamed");
        d["tracks"][BASS]["version"] = json!(4_000_000_000u64);
    });
    assert_eq!(disputed.first().map(|v| v.rule), Some("version_not_writable"));
    assert_eq!(version_at(&patched, &format!("/tracks/{BASS}/version")), 4);
}

#[test]
fn a_merge_resolves_to_the_higher_of_the_two_plus_one() {
    // ADR 0001 §4's rule, which this one produces without a case of its own: the incoming
    // side's ops carry its own number, `before` holds ours, and `max + 1` beats both.
    let before = fixture();
    let mut patched = before.clone();
    patched["tracks"][BASS]["name"] = json!("Bass (merged)");
    patched["tracks"][BASS]["version"] = json!(9); // what the other branch had reached
    let disputed = bump_versions(&before, &mut patched, true);

    assert_eq!(version_at(&patched, &format!("/tracks/{BASS}/version")), 10);
    // Never disputed on the merge path: these are core's own numbers, arriving from the other
    // branch, which is why the two modes are separate (ADR 0005 §2).
    assert!(disputed.is_empty(), "{disputed:?}");
}

#[test]
fn a_plugin_version_is_not_an_entity_version() {
    // `PluginRef.version` is a string naming a plugin release, and `PluginRef` has no id. If
    // the rule keyed on the field name alone, `set_track_instrument` could not pin a version.
    let before = fixture();
    let mut patched = before.clone();
    patched["tracks"][BASS]["instrument"]["ref"] =
        json!({"plugin": {"plugin_id": "com.surge-synth.surge-xt", "version": "1.3.4"}});
    assert!(bump_versions(&before, &mut patched, false).is_empty(), "a plugin pin is not an entity");

    assert_eq!(patched["tracks"][BASS]["instrument"]["ref"]["plugin"]["version"], json!("1.3.4"));
    assert_eq!(version_at(&patched, &format!("/tracks/{BASS}/instrument/version")), 2);
}

#[test]
fn an_event_without_a_version_field_is_left_alone() {
    // TempoEvent, TimeSignatureEvent and AutomationPoint carry an id and no version
    // (ADR 0002 §2), so they fail the second half of the test and never gain one.
    let after = bumped(|d| {
        let events = d["tempo_map"]["events"].as_object_mut().unwrap();
        let key = events.keys().next().unwrap().clone();
        events[&key]["bpm"] = json!(132.0);
    });

    let event = after["tempo_map"]["events"].as_object().unwrap().values().next().unwrap();
    assert!(event.get("version").is_none(), "an event gained a version: {event}");
    assert_eq!(version_at(&after, "/version"), 215);
}

#[test]
fn an_unchanged_document_bumps_nothing() {
    let before = fixture();
    let mut patched = before.clone();
    assert!(bump_versions(&before, &mut patched, false).is_empty());
    assert_eq!(patched, before);
}

// ---- the guard (ADR 0005 §3) ----

#[test]
fn a_version_the_caller_chose_is_disputed() {
    let (_, disputed) = bumped_with_disputes(|d| {
        d["tracks"][BASS]["version"] = json!(9);
        d["version"] = json!(300);
    });
    let rules: Vec<&str> = disputed.iter().map(|v| v.rule).collect();
    assert_eq!(rules, vec!["version_not_writable", "version_not_writable"]);
}

#[test]
fn a_version_spelled_as_a_string_is_disputed() {
    // Proto3 JSON accepts `"1"` for a `uint32`, so a path-based guard would never see this —
    // the same leniency that made the log disagree with `song.json` in M0.2.
    let (patched, disputed) = bumped_with_disputes(|d| {
        d["tracks"][BASS]["name"] = json!("Renamed");
        d["tracks"][BASS]["version"] = json!("1");
    });
    assert_eq!(disputed.first().map(|v| v.rule), Some("version_not_writable"));
    assert_eq!(version_at(&patched, &format!("/tracks/{BASS}/version")), 4, "and core still wins");
}

#[test]
fn a_version_removed_by_the_caller_is_disputed() {
    let (patched, disputed) = bumped_with_disputes(|d| {
        d["tracks"][BASS].as_object_mut().unwrap().remove("version");
    });
    assert_eq!(disputed.first().map(|v| v.rule), Some("version_not_writable"));
    // Nothing else about the track changed, so core restores the number rather than bumping.
    assert_eq!(version_at(&patched, &format!("/tracks/{BASS}/version")), 3);
}

#[test]
fn a_plugin_version_is_never_disputed() {
    let (_, disputed) = bumped_with_disputes(|d| {
        d["tracks"][BASS]["instrument"]["ref"] =
            json!({"plugin": {"plugin_id": "com.surge-synth.surge-xt", "version": "1.3.4"}});
    });
    assert!(disputed.is_empty(), "{disputed:?}");
}

#[test]
fn the_number_core_computes_is_not_disputed() {
    // The round trip §9 needs: a patch this API returned carries the bumps it caused, and
    // feeding it back must not be refused for saying what core already said.
    let (_, disputed) = bumped_with_disputes(|d| {
        d["tracks"][BASS]["name"] = json!("Renamed");
        d["tracks"][BASS]["version"] = json!(4);
        d["version"] = json!(215);
    });
    assert!(disputed.is_empty(), "{disputed:?}");
}

// ---- through the pipeline ----

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-version-{}-{}.escri",
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

fn opened() -> (Scratch, Project, SeededIds, FixedClock) {
    let dir = Scratch::new();
    let (mut ids, clock) = (SeededIds::default(), FixedClock(1_788_307_200_000));
    let song: Song = serde_json::from_value(fixture()).unwrap();
    let project = Project::create(&dir.0, &song, &mut ids, &clock, Author::Human).unwrap();
    (dir, project, ids, clock)
}

fn set_gain(to: f64) -> Vec<Op> {
    serde_json::from_value(json!([
        {"op": "replace", "path": format!("/tracks/{BASS}/mix/gain_db"), "value": to}
    ]))
    .unwrap()
}

#[test]
fn a_commit_refuses_an_op_that_writes_a_version() {
    let (_dir, mut project, mut ids, clock) = opened();
    let ops: Vec<Op> = serde_json::from_value(json!([
        {"op": "replace", "path": format!("/tracks/{BASS}/version"), "value": 9}
    ]))
    .unwrap();

    let refused = project.commit("apply_patch", &ops, Author::Model, &mut ids, &clock).unwrap_err();
    assert_eq!(refused.rule, "version_not_writable");
    // Refused before anything moved: no entry, no advanced ref, no rewritten file.
    assert_eq!(project.history().entries().len(), 1);
}

#[test]
fn preparing_a_change_produces_exactly_what_committing_it_records() {
    // ADR 0006 §3: a dry run is this code path, not a copy of it. If the two ever diverge, the
    // patch a caller approves stops being the patch that gets written.
    let (_dir, mut project, mut ids, clock) = opened();

    let previewed = project.prepare(&set_gain(-3.0)).unwrap();
    let ops = previewed.ops().to_vec();
    // Preparing touches nothing.
    assert_eq!(project.history().entries().len(), 1);

    let id = project.commit("set_param", &set_gain(-3.0), Author::Model, &mut ids, &clock).unwrap();
    let entry = project.history().get(&id).unwrap();
    assert_eq!(escribass_core::ops_of(entry).unwrap(), ops);
}

#[test]
fn versions_survive_a_replay_of_the_log() {
    // The reason the bump sits where it does. If it happened after the recorded diff was
    // taken, `song.json` would carry the new numbers and a replay would not, and `open` would
    // refuse a project that had committed cleanly.
    let (dir, mut project, mut ids, clock) = opened();
    project.commit("set_param", &set_gain(-3.0), Author::Model, &mut ids, &clock).unwrap();
    project.commit("set_param", &set_gain(-4.0), Author::Model, &mut ids, &clock).unwrap();

    let reopened = Project::open(&dir.0).unwrap();
    assert_eq!(reopened.song(), project.song());
    assert_eq!(reopened.song().tracks[BASS].version, 5); // 3, then two commits
    assert_eq!(reopened.song().version, 216);
}
