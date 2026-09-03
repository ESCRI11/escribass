//! The clip, note, automation and arrangement tools of §5.
//!
//! Driven through `Session`, so each case also exercises the version bump, the validator and
//! the entry that gets written.

use escribass_core::{new_song, FixedClock, Project, SeededIds, Session};
use escribass_proto::tools::add_clip_request::Content as AddClipContent;
use escribass_proto::tools::{
    AddAutomationRequest, AddClipRequest, AddSectionRequest, AddTrackRequest, MoveSectionRequest,
    QuantizeRequest, SetNotesRequest, SetTempoRequest, ToolResult, TransposeRequest,
};
use escribass_schema::song::{
    Author, AutomationPoint, Curve, DeviceRef, Note, NoteClip, ParamRef, SourceRef, TrackKind,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const AT: i64 = 1_788_307_200_000;
/// 960 PPQ (§4.2), so a bar of 4/4 is 3840 and a sixteenth is 240.
const SIXTEENTH: i32 = 240;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-clips-{}-{}.escri",
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

fn cmajor() -> Option<DeviceRef> {
    Some(DeviceRef {
        kind: Some(escribass_schema::song::device_ref::Kind::Cmajor(SourceRef {
            source_hash: "8b31c0de4f9c00000000000000000000".to_string(),
        })),
    })
}

/// A session with one instrument track, and that track's id.
fn opened() -> (Scratch, Session, String) {
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock);
    let project = Project::create(&dir.0, &song, &mut ids, &clock).unwrap();
    let mut session = Session::new(project, Box::new(ids), Box::new(clock), Author::Model);

    let added = session
        .add_track(&AddTrackRequest {
            name: "Bass".to_string(),
            kind: TrackKind::Instrument as i32,
            r#ref: cmajor(),
            dry_run: false,
        })
        .unwrap();
    assert!(added.valid, "{:?}", added.errors);

    let track = session
        .project()
        .song()
        .tracks
        .values()
        .max_by_key(|t| t.index)
        .map(|t| t.id.clone())
        .unwrap();
    (dir, session, track)
}

/// A note the caller supplies: no id, no provenance, no version — core owns all three.
fn note(pitch: i32, start_tick: i32) -> Note {
    Note {
        pitch,
        start_tick,
        length_ticks: SIXTEENTH,
        velocity: 100,
        microtonal_cents: 0.0,
        ..Default::default()
    }
}

fn notes(given: [(&str, Note); 2]) -> BTreeMap<String, Note> {
    given.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn rules(result: &ToolResult) -> Vec<&str> {
    result.errors.iter().map(|e| e.rule.as_str()).collect()
}

/// A clip on the track, four bars long, with two notes in it.
fn clip_with_notes(session: &mut Session, track: &str) -> (String, Vec<String>) {
    let added = session
        .add_clip(&AddClipRequest {
            track_id: track.to_string(),
            start_tick: 0,
            length_ticks: 3840,
            content: Some(AddClipContent::NoteClip(NoteClip {
                notes: notes([("a", note(60, 0)), ("b", note(64, 1000))]),
            })),
            dry_run: false,
        })
        .unwrap();
    assert!(added.valid, "{:?}", added.errors);

    let clip = session.project().song().clips.keys().next().unwrap().clone();
    let mut ids: Vec<String> =
        clip_notes(session, &clip).keys().cloned().collect();
    ids.sort();
    (clip, ids)
}

fn clip_notes(session: &Session, clip: &str) -> BTreeMap<String, Note> {
    match &session.project().song().clips[clip].content {
        Some(escribass_schema::song::clip::Content::NoteClip(notes)) => notes.notes.clone(),
        other => panic!("not a note clip: {other:?}"),
    }
}

// ---- add_clip ----

#[test]
fn a_clip_arrives_with_its_notes_minted() {
    let (_dir, mut session, track) = opened();
    let (clip, ids) = clip_with_notes(&mut session, &track);

    let held = clip_notes(&session, &clip);
    assert_eq!(held.len(), 2);
    for id in &ids {
        // The caller's keys were `a` and `b`; core mints real ids (ADR 0006 §4).
        assert_ne!(id.as_str(), "a");
        assert_eq!(held[id].id, *id, "the key and the note's own id agree");
        assert_eq!(held[id].version, 1);
        assert!(held[id].provenance.is_some());
    }
    assert_eq!(session.project().song().clips[&clip].track_id, track);
}

#[test]
fn a_clip_with_no_content_is_an_empty_note_clip() {
    let (_dir, mut session, track) = opened();
    let result = session
        .add_clip(&AddClipRequest {
            track_id: track,
            start_tick: 0,
            length_ticks: 3840,
            content: None,
            dry_run: false,
        })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let clip = session.project().song().clips.keys().next().unwrap().clone();
    assert!(clip_notes(&session, &clip).is_empty());
}

#[test]
fn a_clip_on_an_unknown_track_is_refused() {
    let (_dir, mut session, _track) = opened();
    let result = session
        .add_clip(&AddClipRequest {
            track_id: "01ZZZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
            start_tick: 0,
            length_ticks: 3840,
            content: None,
            dry_run: false,
        })
        .unwrap();

    assert_eq!(rules(&result), vec!["track_unknown"]);
    assert_eq!(result.errors[0].path, "/track_id");
}

#[test]
fn a_note_outside_its_clip_is_refused_by_the_validator() {
    let (_dir, mut session, track) = opened();
    let result = session
        .add_clip(&AddClipRequest {
            track_id: track,
            start_tick: 0,
            length_ticks: 480,
            content: Some(AddClipContent::NoteClip(NoteClip {
                notes: notes([("a", note(60, 0)), ("b", note(64, 10_000))]),
            })),
            dry_run: false,
        })
        .unwrap();

    assert!(!result.valid);
    assert!(rules(&result).contains(&"note_outside_clip"), "{:?}", rules(&result));
}

// ---- set_notes ----

#[test]
fn set_notes_replaces_the_whole_set_and_keeps_ids_it_recognises() {
    // Replacement is what an editor and a generator both mean by "the notes are now these".
    // Keeping a recognised id matters for merge: an edit to one note stays a change to that
    // note rather than a delete and an insert (ADR 0001 §4).
    let (_dir, mut session, track) = opened();
    let (clip, ids) = clip_with_notes(&mut session, &track);
    let kept = ids[0].clone();

    let result = session
        .set_notes(&SetNotesRequest {
            clip_id: clip.clone(),
            notes: notes([(&kept, note(72, 0)), ("fresh", note(67, 480))]),
            dry_run: false,
        })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let held = clip_notes(&session, &clip);
    assert_eq!(held.len(), 2, "the set was replaced, not merged");
    assert_eq!(held[&kept].pitch, 72, "the recognised id kept its place");
    assert!(!held.contains_key(&ids[1]), "the note that was left out is gone");
}

#[test]
fn set_notes_on_an_audio_clip_is_refused() {
    let (_dir, mut session, track) = opened();
    session
        .add_clip(&AddClipRequest {
            track_id: track,
            start_tick: 0,
            length_ticks: 3840,
            content: Some(AddClipContent::AudioClip(escribass_schema::song::AudioClip {
                asset_hash: "8b31c0de4f9c00000000000000000000".to_string(),
            })),
            dry_run: false,
        })
        .unwrap();
    let clip = session.project().song().clips.keys().next().unwrap().clone();

    let result = session
        .set_notes(&SetNotesRequest {
            clip_id: clip,
            notes: notes([("a", note(60, 0)), ("b", note(64, 480))]),
            dry_run: false,
        })
        .unwrap();

    assert_eq!(rules(&result), vec!["clip_kind_mismatch"]);
}

// ---- transpose ----

#[test]
fn transpose_emits_one_op_per_note() {
    // Granularity is load-bearing, not cosmetic: merge auto-resolves by comparing op paths
    // (ADR 0001 §4), so a whole-clip patch would make every pair of edits to one clip conflict.
    let (_dir, mut session, track) = opened();
    let (clip, _) = clip_with_notes(&mut session, &track);

    let result = session
        .transpose(&TransposeRequest {
            clip_id: clip.clone(),
            semitones: 12,
            note_ids: vec![],
            dry_run: true,
        })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let patch: Value = serde_json::from_slice(&result.patch).unwrap();
    let pitch_ops = patch
        .as_array()
        .unwrap()
        .iter()
        .filter(|op| op["path"].as_str().unwrap().ends_with("/pitch"))
        .count();
    assert_eq!(pitch_ops, 2, "one op per note, not one for the clip: {patch}");
}

#[test]
fn transpose_moves_only_the_notes_it_is_given() {
    let (_dir, mut session, track) = opened();
    let (clip, ids) = clip_with_notes(&mut session, &track);
    let before = clip_notes(&session, &clip);

    let result = session
        .transpose(&TransposeRequest {
            clip_id: clip.clone(),
            semitones: 2,
            note_ids: vec![ids[0].clone()],
            dry_run: false,
        })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let after = clip_notes(&session, &clip);
    assert_eq!(after[&ids[0]].pitch, before[&ids[0]].pitch + 2);
    assert_eq!(after[&ids[1]].pitch, before[&ids[1]].pitch, "the other note did not move");
}

#[test]
fn transpose_out_of_range_is_refused_not_clamped() {
    // Silently rewriting what a caller asked for is how a tool stops being predictable. The
    // rule already exists in the one place §4.4 puts it, so the tool does not repeat it.
    let (_dir, mut session, track) = opened();
    let (clip, _) = clip_with_notes(&mut session, &track);

    let result = session
        .transpose(&TransposeRequest {
            clip_id: clip.clone(),
            semitones: 100,
            note_ids: vec![],
            dry_run: false,
        })
        .unwrap();

    assert!(!result.valid);
    assert!(rules(&result).contains(&"pitch_out_of_range"), "{:?}", rules(&result));
    assert_eq!(clip_notes(&session, &clip).values().map(|n| n.pitch).min(), Some(60));
}

#[test]
fn a_note_id_that_is_not_in_the_clip_is_refused() {
    // Skipping it would tell a caller its edit landed when it did not.
    let (_dir, mut session, track) = opened();
    let (clip, _) = clip_with_notes(&mut session, &track);

    let result = session
        .transpose(&TransposeRequest {
            clip_id: clip,
            semitones: 1,
            note_ids: vec!["01ZZZZZZZZZZZZZZZZZZZZZZZZ".to_string()],
            dry_run: false,
        })
        .unwrap();

    assert_eq!(rules(&result), vec!["note_unknown"]);
}

// ---- quantize ----

#[test]
fn quantize_snaps_to_the_nearest_grid_line_with_ties_rounding_up() {
    // Integer arithmetic and a stated tie rule are both §11's requirement: a float round would
    // depend on the platform's rounding mode exactly at the halfway point, which is where a
    // musician's deliberately placed note tends to sit.
    let (_dir, mut session, track) = opened();
    let (clip, ids) = clip_with_notes(&mut session, &track);

    let result = session
        .quantize(&QuantizeRequest {
            clip_id: clip.clone(),
            grid_ticks: SIXTEENTH,
            note_ids: vec![],
            dry_run: false,
        })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let after = clip_notes(&session, &clip);
    assert_eq!(after[&ids[0]].start_tick, 0, "already on the grid");
    // 1000 is 4.17 sixteenths; the nearest line is 960.
    assert_eq!(after[&ids[1]].start_tick, 960);
}

#[test]
fn a_tie_rounds_up() {
    let (_dir, mut session, track) = opened();
    let added = session
        .add_clip(&AddClipRequest {
            track_id: track,
            start_tick: 0,
            length_ticks: 3840,
            content: Some(AddClipContent::NoteClip(NoteClip {
                // Exactly halfway between 0 and 240.
                notes: notes([("a", note(60, 120)), ("b", note(64, 119))]),
            })),
            dry_run: false,
        })
        .unwrap();
    assert!(added.valid, "{:?}", added.errors);
    let clip = session.project().song().clips.keys().next().unwrap().clone();

    session
        .quantize(&QuantizeRequest {
            clip_id: clip.clone(),
            grid_ticks: SIXTEENTH,
            note_ids: vec![],
            dry_run: false,
        })
        .unwrap();

    let after = clip_notes(&session, &clip);
    let mut ticks: Vec<i32> = after.values().map(|n| n.start_tick).collect();
    ticks.sort_unstable();
    assert_eq!(ticks, vec![0, 240], "119 rounds down, the tie at 120 rounds up");
}

#[test]
fn a_grid_of_zero_is_refused_at_the_argument() {
    let (_dir, mut session, track) = opened();
    let (clip, _) = clip_with_notes(&mut session, &track);

    for grid in [0, -240] {
        let result = session
            .quantize(&QuantizeRequest {
                clip_id: clip.clone(),
                grid_ticks: grid,
                note_ids: vec![],
                dry_run: false,
            })
            .unwrap();
        assert_eq!(rules(&result), vec!["grid_not_positive"], "grid {grid}");
        assert_eq!(result.errors[0].path, "/grid_ticks");
    }
}

// ---- automation ----

#[test]
fn an_automation_lane_targets_a_device_that_exists() {
    let (_dir, mut session, track) = opened();
    let device = session.project().song().tracks[&track].instrument.clone().unwrap().id;

    let result = session
        .add_automation(&AddAutomationRequest {
            target: Some(ParamRef { device_id: device, param: "drive".to_string() }),
            points: [(
                "p".to_string(),
                AutomationPoint { tick: 0, value: 0.5, curve: Curve::Linear as i32, ..Default::default() },
            )]
            .into_iter()
            .collect(),
            dry_run: false,
        })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let lane = session.project().song().automation.values().next().unwrap().clone();
    assert_eq!(lane.version, 1);
    assert_eq!(lane.points.len(), 1);
    // Points carry an id and no provenance (ADR 0002 §2); only the id is core's to mint.
    let (key, point) = lane.points.iter().next().unwrap();
    assert_ne!(key.as_str(), "p");
    assert_eq!(point.id, *key);
}

#[test]
fn an_automation_point_that_is_not_finite_is_refused_at_its_own_path() {
    let (_dir, mut session, track) = opened();
    let device = session.project().song().tracks[&track].instrument.clone().unwrap().id;

    let result = session
        .add_automation(&AddAutomationRequest {
            target: Some(ParamRef { device_id: device, param: "drive".to_string() }),
            points: [(
                "p".to_string(),
                AutomationPoint { tick: 0, value: f64::NAN, curve: Curve::Linear as i32, ..Default::default() },
            )]
            .into_iter()
            .collect(),
            dry_run: false,
        })
        .unwrap();

    assert_eq!(rules(&result), vec!["double_not_finite"]);
    assert_eq!(result.errors[0].path, "/points/p/value", "the error names the point given");
}

#[test]
fn an_automation_lane_on_an_unknown_device_is_refused() {
    let (_dir, mut session, _track) = opened();
    let result = session
        .add_automation(&AddAutomationRequest {
            target: Some(ParamRef {
                device_id: "01ZZZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
                param: "drive".to_string(),
            }),
            points: [(
                "p".to_string(),
                AutomationPoint { tick: 0, value: 0.5, curve: Curve::Linear as i32, ..Default::default() },
            )]
            .into_iter()
            .collect(),
            dry_run: false,
        })
        .unwrap();

    assert_eq!(rules(&result), vec!["device_unknown"]);
}

// ---- tempo ----

#[test]
fn set_tempo_upserts_the_event_at_that_tick() {
    // Two events at one tick have no defined order: §4.2 derives order from `tick`, and a map
    // has no positions to break the tie with.
    let (_dir, mut session, _track) = opened();
    let before = session.project().song().tempo_map.clone().unwrap().events;
    assert_eq!(before.len(), 1);

    let result = session
        .set_tempo(&SetTempoRequest { bpm: 132.0, tick: 0, dry_run: false })
        .unwrap();
    assert!(result.valid, "{:?}", result.errors);

    let after = session.project().song().tempo_map.clone().unwrap().events;
    assert_eq!(after.len(), 1, "the event at tick 0 was replaced, not duplicated");
    assert_eq!(after.values().next().unwrap().bpm, 132.0);
    assert_eq!(after.keys().next(), before.keys().next(), "and it kept its id");
}

#[test]
fn set_tempo_at_a_new_tick_adds_an_event() {
    let (_dir, mut session, _track) = opened();
    session.set_tempo(&SetTempoRequest { bpm: 90.0, tick: 3840, dry_run: false }).unwrap();

    let events = session.project().song().tempo_map.clone().unwrap().events;
    assert_eq!(events.len(), 2);
    assert!(events.values().any(|e| e.tick == 3840 && e.bpm == 90.0));
}

#[test]
fn a_tempo_that_is_not_a_number_is_refused_before_it_becomes_null() {
    let (_dir, mut session, _track) = opened();
    let result = session
        .set_tempo(&SetTempoRequest { bpm: f64::INFINITY, tick: 0, dry_run: false })
        .unwrap();

    assert_eq!(rules(&result), vec!["double_not_finite"]);
    assert_eq!(result.errors[0].path, "/bpm");
}

#[test]
fn a_tempo_of_zero_is_refused_by_the_validator() {
    let (_dir, mut session, _track) = opened();
    let result = session.set_tempo(&SetTempoRequest { bpm: 0.0, tick: 0, dry_run: false }).unwrap();
    assert!(rules(&result).contains(&"tempo_not_positive"), "{:?}", rules(&result));
}

// ---- sections ----

#[test]
fn a_section_is_a_label_over_a_range_and_moving_it_moves_nothing_else() {
    // §4.2 makes a section a name over ticks, not a container. A tool that dragged clips along
    // would be inventing an arrangement model the schema does not have.
    let (_dir, mut session, track) = opened();
    let (clip, _) = clip_with_notes(&mut session, &track);
    let clip_start = session.project().song().clips[&clip].start_tick;

    session
        .add_section(&AddSectionRequest {
            name: "Verse".to_string(),
            start_tick: 0,
            end_tick: 3840,
            dry_run: false,
        })
        .unwrap();
    let section = session.project().song().sections.keys().next().unwrap().clone();

    let result = session
        .move_section(&MoveSectionRequest {
            section_id: section.clone(),
            start_tick: 3840,
            end_tick: 7680,
            dry_run: false,
        })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let moved = &session.project().song().sections[&section];
    assert_eq!((moved.start_tick, moved.end_tick), (3840, 7680));
    assert_eq!(session.project().song().clips[&clip].start_tick, clip_start, "the clip stayed");
}

#[test]
fn moving_a_section_that_is_not_there_is_refused() {
    let (_dir, mut session, _track) = opened();
    let result = session
        .move_section(&MoveSectionRequest {
            section_id: "01ZZZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
            start_tick: 0,
            end_tick: 3840,
            dry_run: false,
        })
        .unwrap();

    assert_eq!(rules(&result), vec!["section_unknown"]);
}

#[test]
fn a_section_that_ends_before_it_starts_is_refused_by_the_validator() {
    let (_dir, mut session, _track) = opened();
    let result = session
        .add_section(&AddSectionRequest {
            name: "Backwards".to_string(),
            start_tick: 3840,
            end_tick: 0,
            dry_run: false,
        })
        .unwrap();

    assert!(!result.valid);
    assert!(rules(&result).contains(&"section_not_positive"), "{:?}", rules(&result));
}

// ---- across the eight ----

#[test]
fn a_scripted_arrangement_is_reproducible() {
    let build = || {
        let (dir, mut session, track) = opened();
        let (clip, _) = clip_with_notes(&mut session, &track);
        session
            .quantize(&QuantizeRequest {
                clip_id: clip.clone(),
                grid_ticks: SIXTEENTH,
                note_ids: vec![],
                dry_run: false,
            })
            .unwrap();
        session
            .transpose(&TransposeRequest {
                clip_id: clip,
                semitones: -12,
                note_ids: vec![],
                dry_run: false,
            })
            .unwrap();
        session.set_tempo(&SetTempoRequest { bpm: 132.0, tick: 0, dry_run: false }).unwrap();
        session
            .add_section(&AddSectionRequest {
                name: "Verse".to_string(),
                start_tick: 0,
                end_tick: 3840,
                dry_run: false,
            })
            .unwrap();
        std::fs::read_to_string(dir.0.join("song.json")).unwrap()
    };
    assert_eq!(build(), build());
}

#[test]
fn quantize_does_not_overflow_at_the_top_of_the_tick_range() {
    // A clip may be `i32::MAX` long, so a note can sit near the top of the range. Rounding in
    // `i32` overflowed there: a panic in a debug build, and in release a wrap to a negative
    // tick refused with a rule that describes the wrong problem.
    let (_dir, mut session, track) = opened();
    let far = i32::MAX - 300;
    let added = session
        .add_clip(&AddClipRequest {
            track_id: track,
            start_tick: 0,
            length_ticks: i32::MAX,
            content: Some(AddClipContent::NoteClip(NoteClip {
                notes: notes([("a", note(60, far)), ("b", note(64, 0))]),
            })),
            dry_run: false,
        })
        .unwrap();
    assert!(added.valid, "{:?}", added.errors);
    let clip = session.project().song().clips.keys().next().unwrap().clone();

    // Whatever the outcome, it is a result rather than a panic.
    let result = session
        .quantize(&QuantizeRequest {
            clip_id: clip,
            grid_ticks: SIXTEENTH,
            note_ids: vec![],
            dry_run: false,
        })
        .unwrap();
    if !result.valid {
        assert!(rules(&result).iter().all(|r| *r != "tick_negative"), "{:?}", rules(&result));
    }
}
