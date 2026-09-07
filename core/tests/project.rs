//! `.escri` project directory tests (§10, ADR 0004).
//!
//! Every project under test is built in memory and written by `Project::write`, then read
//! back. No test hand-writes `song.json`, so CLAUDE.md #2 holds even in the loader's tests —
//! the only exception is where a test deliberately corrupts a file to prove `open` notices.

mod common;
use common::{manifest, MANIFEST};

use escribass_core::{
    diff, entry, new_song, timestamp_from_ms, FixedClock, History, IdSource, Op, Project,
    ProjectLock, SeededIds, Session,
};
use escribass_proto::tools::AddTrackRequest;
use escribass_schema::history::Refs;
use escribass_schema::song::{device_ref, Author, DeviceRef, Provenance, SamplerRef, Song, TrackKind};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");
const BASS: &str = "01M1FPMP00TRACKBASS0000002";

fn fixture_value() -> Value {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).expect("fixture")).unwrap()
}

/// A directory of its own per test, removed on drop even when the test fails.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-{}-{}.escri",
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

fn provenance() -> Provenance {
    Provenance {
        author: Author::Human as i32,
        model_id: None,
        prompt_id: None,
        tool_call_id: None,
        created_at: Some(timestamp_from_ms(1_788_307_200_000)),
    }
}

/// A project whose log builds the fixture from a default song, plus one edit.
fn sample(root: &PathBuf) -> Project {
    let mut ids = SeededIds::default();
    let mut log = History::new();
    let empty = serde_json::to_value(Song::default()).unwrap();

    let root_id = ids.next_id();
    log.append(entry(root_id.clone(), vec![], "create", &diff(&empty, &fixture_value()), provenance(), 1))
        .unwrap();
    log.create_ref("main", &root_id).unwrap();
    log.set_head("main").unwrap();

    let mut louder = fixture_value();
    louder["tracks"][BASS]["mix"]["gain_db"] = json!(-3.0);
    let next = ids.next_id();
    log.append(entry(next.clone(), vec![root_id], "set_param", &diff(&fixture_value(), &louder), provenance(), 1))
        .unwrap();
    log.advance("main", &next).unwrap();

    Project::new(root, serde_json::from_value(louder).unwrap(), log, manifest())
}

// ---- the round trip ----

#[test]
fn a_project_written_and_reopened_is_unchanged() {
    let dir = Scratch::new();
    let mut original = sample(&dir.0);
    original.write().unwrap();

    let reopened = Project::open(&dir.0, manifest()).unwrap();
    assert_eq!(reopened.song(), original.song());
    assert_eq!(reopened.history(), original.history());
    assert_eq!(reopened, original);
}

#[test]
fn the_directory_holds_exactly_what_section_10_names() {
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();

    let mut found: Vec<String> = std::fs::read_dir(&dir.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    assert_eq!(found, vec!["assets", "lock.json", "patches", "refs.json", "song.json"]);

    // One file per entry, named by its id.
    let patches: Vec<String> = std::fs::read_dir(dir.0.join("patches"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(patches.len(), 2);
    for name in &patches {
        assert!(name.ends_with(".json") && name.len() == 31, "{name}");
    }
}

#[test]
fn writing_twice_produces_identical_bytes() {
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();
    let first = std::fs::read_to_string(dir.0.join("song.json")).unwrap();
    project.write().unwrap();
    assert_eq!(std::fs::read_to_string(dir.0.join("song.json")).unwrap(), first);
}

#[test]
fn song_json_on_disk_is_the_canonical_form() {
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.0.join("song.json")).unwrap(),
        escribass_core::to_canonical_json(project.song()).unwrap()
    );
}

// ---- ADR 0004: the log is authoritative ----

#[test]
fn a_song_that_does_not_match_a_replay_is_reported_not_repaired() {
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();

    // Stand in for a hand-edited file, which §5 forbids and ADR 0004 says to report.
    let text = std::fs::read_to_string(dir.0.join("song.json")).unwrap();
    std::fs::write(dir.0.join("song.json"), text.replace("\"name\": \"Bass\"", "\"name\": \"Edited\"")).unwrap();

    let e = Project::open(&dir.0, manifest()).unwrap_err();
    assert_eq!(e.rule, "song_diverged");
    assert!(e.message.contains("/tracks/"), "the error names the differing path: {}", e.message);
    // Reported, not repaired.
    assert!(std::fs::read_to_string(dir.0.join("song.json")).unwrap().contains("Edited"));
}

#[test]
fn an_orphan_entry_left_by_a_crash_does_not_become_history() {
    // refs.json is written last, so a crash before it leaves an entry nothing references.
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();

    let head = project.history().head_id().unwrap().to_string();
    let orphan = entry("01M1FPMP00RPHAN000000000099", vec![head], "set_param", &[], provenance(), 1);
    std::fs::write(
        dir.0.join("patches").join("01M1FPMP00RPHAN000000000099.json"),
        escribass_core::entry_to_json(&orphan).unwrap(),
    )
    .unwrap();

    let reopened = Project::open(&dir.0, manifest()).unwrap();
    assert_eq!(reopened.history().entries().len(), 3, "the orphan is loaded");
    assert_eq!(reopened.song(), project.song(), "but it is not part of HEAD's history");
}

// ---- what open refuses ----

#[test]
fn a_patch_file_whose_name_disagrees_with_its_id_is_refused() {
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();

    let head = project.history().head_id().unwrap();
    std::fs::copy(
        dir.0.join("patches").join(format!("{head}.json")),
        dir.0.join("patches").join("01M1FPMP00CPYCPY000000000001.json"),
    )
    .unwrap();
    assert_eq!(Project::open(&dir.0, manifest()).unwrap_err().rule, "entry_filename_mismatch");
}

#[test]
fn a_file_that_is_not_a_patch_is_ignored() {
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();
    std::fs::write(dir.0.join("patches").join("notes.txt"), "scratch").unwrap();
    std::fs::write(dir.0.join("patches").join(".song.json.swp"), "editor").unwrap();
    assert!(Project::open(&dir.0, manifest()).is_ok());
}

#[test]
fn a_lock_from_another_schema_version_is_refused_at_load() {
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();
    std::fs::write(dir.0.join("lock.json"), "{\n  \"schema_version\": 99\n}\n").unwrap();
    let e = Project::open(&dir.0, manifest()).unwrap_err();
    assert_eq!(e.rule, "schema_version_mismatch");
    assert!(e.message.contains("99"), "{}", e.message);
}

// ---- lock.json v2 (ADR 0010) ----

const SURGE: &str = "Surge Synth Team/Surge XT";
const SFIZZ: &str = "SFZTools/sfizz";
const SURGE_FX: &str = "/tracks/01M1FPMP00TRACKBASS0000002/fx_chain/01M1FPMP00FXSRGE0000000005";

fn lock(root: &PathBuf) -> Value {
    serde_json::from_str(&std::fs::read_to_string(root.join("lock.json")).unwrap()).unwrap()
}

fn write_lock(root: &PathBuf, lock: &Value) {
    std::fs::write(root.join("lock.json"), serde_json::to_string_pretty(lock).unwrap()).unwrap();
}

fn manifest_value() -> Value {
    serde_json::from_str(&std::fs::read_to_string(MANIFEST).unwrap()).unwrap()
}

/// A manifest that hosts nothing, with this one's engine — the build that lost a plugin.
fn without_plugins() -> std::sync::Arc<escribass_core::Manifest> {
    let mut described = manifest_value();
    described["plugins"] = json!({});
    let at = std::env::temp_dir().join(format!("escribass-empty-manifest-{}.json", std::process::id()));
    std::fs::write(&at, described.to_string()).unwrap();
    std::sync::Arc::new(escribass_core::Manifest::read(&at).unwrap())
}

#[test]
fn a_referenced_plugin_is_pinned_from_the_build_manifest() {
    // ADR 0010 §1 and §2. The engine block is recorded too, so a person reading the file can
    // see what the last write was made against — §11's "all external tool versions recorded".
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();

    let lock = lock(&dir.0);
    assert_eq!(lock["plugins"][SURGE]["version"], json!("1.3.4"));
    assert_eq!(lock["plugins"][SURGE]["commit"], json!("f7b97c682ade0b87da85ca5968b63d5c7c98e68d"));
    assert!(lock["engine"]["tracktion_engine"].is_string(), "{lock}");
    // Nothing machine-specific: the file is committed to the user's repository and
    // byte-compared by the determinism suite (ADR 0010 §1).
    assert!(lock["plugins"][SURGE].get("path").is_none(), "{lock}");
    assert!(lock["plugins"][SURGE].get("params").is_none(), "{lock}");
}

#[test]
fn a_sampler_pins_the_plugin_that_plays_it() {
    // ADR 0010 §1, amended 2026-09-07 in PR 13. "No entry for `SamplerRef` — the hash IS the
    // pin" was true of the *patch* and false of everything that turns it into samples: the SFZ
    // is played by a bundled plugin whose build decides every sample of the render. So
    // `tests/renders/sfizz`, which names no `plugin_id` anywhere, wrote an empty `plugins`
    // block; nothing in its `lock.json` moved when sfizz_ui moved, and `lock_mismatch` had no
    // entry to fire on. §11's **[MUST]** with a whole device kind outside it.
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let clock = FixedClock(1_788_307_200_000);
    let song = new_song(&mut ids, &clock, Author::Model);
    let project =
        Project::create(&dir.0, &song, &mut ids, &clock, Author::Model, manifest()).unwrap();
    let mut session = Session::new(project, Box::new(ids), Box::new(clock), Author::Model);
    // Through the tool API, like every other mutation (CLAUDE.md #2).
    session
        .add_track(&AddTrackRequest {
            name: "Piano".to_string(),
            kind: TrackKind::Instrument as i32,
            r#ref: Some(DeviceRef {
                kind: Some(device_ref::Kind::Sampler(SamplerRef {
                    sfz_hash: "3c1de4f98b".to_string(),
                })),
            }),
            dry_run: false,
        })
        .expect("a sampler track");

    let written = lock(&dir.0);
    assert_eq!(
        written["plugins"][SFIZZ]["commit"],
        json!("6ef7b89b6e5aa914593c7f3ca19b859915c30337"),
        "the build that plays the SFZ is pinned, from the manifest's own `sampler`: {written}"
    );

    // And the pin is live, which is the only reason to write one down.
    let mut edited = written;
    edited["plugins"][SFIZZ]["commit"] = json!("0".repeat(40));
    write_lock(&dir.0, &edited);
    let e = Project::open(&dir.0, manifest()).unwrap_err();
    assert_eq!(e.rule, "lock_mismatch");
    assert!(e.message.contains(SFIZZ), "{}", e.message);
}

#[test]
fn a_pin_outlives_the_reference_that_made_it() {
    // ADR 0010 §2's monotone rule. A block derived purely from the current song would lose
    // this pin, and ADR 0005 §4's undo would then re-pin from whatever build is running.
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();

    // A counter past the ones `sample` already minted: the log is append-only and an id is a
    // pure function of the counter under a seed.
    let mut ids = SeededIds::new(1_788_307_200_000, 99);
    let remove: Vec<Op> =
        serde_json::from_value(json!([{"op": "remove", "path": SURGE_FX}])).unwrap();
    project
        .commit("apply_patch", &remove, Author::Human, &mut ids, &FixedClock(1_788_307_200_000))
        .expect("removing the effect is a legal edit");

    assert!(project.song().tracks[BASS].fx_chain.is_empty(), "the reference is gone");
    assert_eq!(lock(&dir.0)["plugins"][SURGE]["version"], json!("1.3.4"), "and the pin is not");
}

#[test]
fn a_referenced_plugin_this_build_cannot_match_refuses_to_open() {
    // §11's load check, ADR 0010 §3: the same shape `schema_version_mismatch` has, and the
    // strict reading — nothing opens, nothing is substituted, nothing is silently re-pinned.
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();

    let mut edited = lock(&dir.0);
    edited["plugins"][SURGE]["commit"] = json!("0000000000000000000000000000000000000000");
    write_lock(&dir.0, &edited);

    let e = Project::open(&dir.0, manifest()).unwrap_err();
    assert_eq!(e.rule, "lock_mismatch");
    assert!(e.message.contains(SURGE) && e.message.contains("0000000"), "{}", e.message);

    // The other arm: the pin agrees with itself and this build has no such plugin at all.
    write_lock(&dir.0, &lock(&dir.0));
    let e = Project::open(&dir.0, without_plugins()).unwrap_err();
    assert_eq!(e.rule, "lock_mismatch");
    assert!(e.message.contains("cannot host it"), "{}", e.message);
}

#[test]
fn a_pin_the_song_no_longer_references_is_inert() {
    // The cost of monotone, and what makes it affordable: `open` compares only the pins the
    // song currently references, so a stale entry is a record of history, not a hostage.
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();

    let mut edited = lock(&dir.0);
    edited["plugins"]["Some Vendor/A Plugin We Dropped"] =
        json!({"commit": "0000000000000000000000000000000000000000", "version": "0.1"});
    write_lock(&dir.0, &edited);

    Project::open(&dir.0, manifest()).expect("a pin nothing references cannot refuse anything");
}

#[test]
fn the_engine_block_re_pins_on_open_rather_than_refusing() {
    // ADR 0010 §3's one exception, and the reason it is one: there is a single engine and a
    // project cannot choose it, so refusing would refuse every project on the machine at once.
    // What a render was made with travels with the render (ADR 0008 §5), not with this file.
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();

    let mut edited = lock(&dir.0);
    edited["engine"]["tracktion_engine"] = json!("0000000000000000000000000000000000000000");
    write_lock(&dir.0, &edited);

    let mut reopened = Project::open(&dir.0, manifest()).expect("a newer engine still opens");
    reopened.write().unwrap();
    assert_eq!(
        lock(&dir.0)["engine"],
        manifest_value()["engine"],
        "the running build's commits, written back on the next write"
    );
}

#[test]
fn a_lock_written_before_m1_still_opens() {
    // `engine` and `plugins` default to empty, and an absent pin is one that has not been
    // added yet rather than a disagreement (ADR 0010 §2). The next write adds it.
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();
    std::fs::write(dir.0.join("lock.json"), "{\n  \"schema_version\": 1\n}\n").unwrap();

    let mut reopened = Project::open(&dir.0, manifest()).expect("a v1 lock is not a mismatch");
    reopened.write().unwrap();
    assert_eq!(lock(&dir.0)["plugins"][SURGE]["version"], json!("1.3.4"));
}

#[test]
fn a_missing_member_names_the_file() {
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();
    std::fs::remove_file(dir.0.join("refs.json")).unwrap();
    let e = Project::open(&dir.0, manifest()).unwrap_err();
    assert_eq!(e.rule, "unreadable");
    assert!(e.path.ends_with("refs.json"), "{}", e.path);

    assert_eq!(Project::open(std::env::temp_dir().join("no-such.escri"), manifest()).unwrap_err().rule, "unreadable");
}

// ---- atomic writes ----

#[test]
fn a_write_leaves_no_temporary_files_behind() {
    let dir = Scratch::new();
    sample(&dir.0).write().unwrap();
    for directory in [&dir.0, &dir.0.join("patches")] {
        for found in std::fs::read_dir(directory).unwrap() {
            let name = found.unwrap().file_name().to_string_lossy().into_owned();
            assert!(!name.ends_with(".tmp"), "left behind: {name}");
        }
    }
}

#[test]
fn a_failed_write_leaves_the_previous_file_intact() {
    // The point of writing through a rename: a reader sees the old file or the new one, never
    // half of one. Simulated by making the temporary path unwritable.
    let dir = Scratch::new();
    let mut project = sample(&dir.0);
    project.write().unwrap();
    let before = std::fs::read_to_string(dir.0.join("song.json")).unwrap();

    std::fs::create_dir(dir.0.join(".song.json.tmp")).unwrap();
    assert!(project.write().is_err(), "a directory in the way must fail the write");
    assert_eq!(std::fs::read_to_string(dir.0.join("song.json")).unwrap(), before);
}

#[test]
fn a_log_loads_when_a_child_id_sorts_before_its_parent() {
    // Id order is not ancestry. An entry that arrived from another branch, or one minted
    // across a clock adjustment, can sort below its own parent — and that log is still a
    // valid DAG, so loading it must not depend on the ids happening to be monotonic.
    let later = "01ZZZZZZZZZZZZZZZZZZZZZZZZ";
    let earlier = "01AAAAAAAAAAAAAAAAAAAAAAAA";

    let empty = serde_json::to_value(Song::default()).unwrap();
    let mut louder = fixture_value();
    louder["tracks"][BASS]["mix"]["gain_db"] = json!(-3.0);

    let root = entry(later, vec![], "create", &diff(&empty, &fixture_value()), provenance(), 1);
    let child = entry(
        earlier,
        vec![later.to_string()],
        "set_param",
        &diff(&fixture_value(), &louder),
        provenance(),
        1,
    );

    let refs = Refs {
        head: "main".to_string(),
        refs: [("main".to_string(), earlier.to_string())].into_iter().collect(),
    };
    let log = History::from_parts(vec![child, root], refs).unwrap();
    assert_eq!(log.head_id(), Some(earlier));
    assert_eq!(log.materialise(earlier).unwrap(), louder);
}

#[test]
fn a_log_missing_a_parent_is_still_refused() {
    let ghost = "01ZZZZZZZZZZZZZZZZZZZZZZZZ";
    let empty = serde_json::to_value(Song::default()).unwrap();
    let orphan = entry(
        "01AAAAAAAAAAAAAAAAAAAAAAAA",
        vec![ghost.to_string()],
        "create",
        &diff(&empty, &fixture_value()),
        provenance(),
        1,
    );
    let refs = Refs {
        head: "main".to_string(),
        refs: [("main".to_string(), "01AAAAAAAAAAAAAAAAAAAAAAAA".to_string())]
            .into_iter()
            .collect(),
    };
    assert_eq!(History::from_parts(vec![orphan], refs).unwrap_err().rule, "parent_missing");
}

// ---- the directory lock (ADR 0012 §3) ----

#[test]
fn a_second_opener_is_refused_and_the_lock_is_never_broken() {
    // The race `app` makes ordinary rather than hypothetical: two processes on one `.escri`,
    // interleaving ADR 0004's three renames. What closes it is one `O_EXCL`, and what makes
    // the choice a real one is the second half of this test — the refusal leaves the lock
    // exactly where it was. A crashed process therefore leaves a project that says why it
    // will not open, which is git's `index.lock` bargain, taken deliberately.
    let dir = Scratch::new();
    std::fs::create_dir_all(&dir.0).expect("a directory to lock");

    let held = ProjectLock::take(&dir.0).expect("the first opener takes it");
    assert!(dir.0.join("lock").exists());

    let refused = ProjectLock::take(&dir.0).expect_err("the second opener is refused");
    assert_eq!(refused.rule, "project_locked");
    assert!(
        refused.message.contains(&std::process::id().to_string()),
        "the refusal names the process holding it: {}",
        refused.message
    );
    assert!(dir.0.join("lock").exists(), "a refused take must not remove the lock");

    drop(held);
    assert!(!dir.0.join("lock").exists(), "a clean close removes it");
    ProjectLock::take(&dir.0).expect("and the next opener gets it");
}
