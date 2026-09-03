//! The four device tools of §5: `add_track`, `set_track_instrument`, `add_effect`, `set_param`.
//!
//! Driven through `Session`, so every case here also exercises the pipeline behind it — the
//! version bump, the validator, and the entry that gets written. A tool that produced an
//! invalid song would fail here even if its own arguments were fine, which is the arrangement
//! ADR 0006 §1 is for.

use escribass_core::{new_song, FixedClock, Project, SeededIds, Session};
const AT: i64 = 1_788_307_200_000;
use escribass_proto::tools::{
    AddEffectRequest, AddTrackRequest, SetParamRequest, SetTrackInstrumentRequest, ToolResult,
};
use escribass_schema::song::{Author, DeviceRef, SourceRef, TrackKind};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};


struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-tools-{}-{}.escri",
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

/// A session on a new project: one master track, a tempo event, nothing else.
fn opened() -> (Scratch, Session) {
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock);
    let project = Project::create(&dir.0, &song, &mut ids, &clock).unwrap();
    (dir, Session::new(project, Box::new(ids), Box::new(clock), Author::Model))
}

fn instrument_track(session: &mut Session, name: &str) -> String {
    let result = session
        .add_track(&AddTrackRequest {
            name: name.to_string(),
            kind: TrackKind::Instrument as i32,
            r#ref: cmajor(),
            dry_run: false,
        })
        .unwrap();
    assert!(result.valid, "{:?}", result.errors);
    newest_track(session)
}

/// The track with the highest index — the one `add_track` just appended.
fn newest_track(session: &Session) -> String {
    let song = session.project().song();
    song.tracks
        .values()
        .max_by_key(|t| t.index)
        .map(|t| t.id.clone())
        .expect("a song always has a track")
}

fn cmajor() -> Option<DeviceRef> {
    Some(DeviceRef {
        kind: Some(escribass_schema::song::device_ref::Kind::Cmajor(SourceRef {
            source_hash: "8b31c0de4f9c00000000000000000000".to_string(),
        })),
    })
}

fn rules(result: &ToolResult) -> Vec<&str> {
    result.errors.iter().map(|e| e.rule.as_str()).collect()
}

// ---- add_track ----

#[test]
fn a_new_track_appends_after_the_current_last() {
    let (_dir, mut session) = opened();
    let master_index = session.project().song().tracks.values().next().unwrap().index;

    let result = session
        .add_track(&AddTrackRequest {
            name: "Bass".to_string(),
            kind: TrackKind::Instrument as i32,
            r#ref: cmajor(),
            dry_run: false,
        })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let song = session.project().song();
    assert_eq!(song.tracks.len(), 2);
    let added = song.tracks.values().find(|t| t.name == "Bass").unwrap();
    assert_eq!(added.index, master_index + 1);
    // ADR 0005 §2: a tool builds an entity at version 0 and the pipeline takes it to 1.
    assert_eq!(added.version, 1);
    assert_eq!(added.provenance.as_ref().unwrap().author, Author::Model as i32);
}

#[test]
fn a_track_with_no_kind_is_refused_by_the_validator_not_by_the_tool() {
    // The tool checks nothing the validator can check. `kind` unset is proto3's default, and
    // §4.4 already refuses an unspecified enum — a second check here would be a second place
    // to change when that rule moves.
    let (_dir, mut session) = opened();
    let result = session
        .add_track(&AddTrackRequest {
            name: "Nameless".to_string(),
            kind: 0,
            r#ref: None,
            dry_run: false,
        })
        .unwrap();

    assert!(!result.valid);
    assert!(rules(&result).contains(&"enum_unspecified"), "{:?}", rules(&result));
    assert_eq!(session.project().song().tracks.len(), 1, "nothing was written");
}

#[test]
fn a_second_master_track_is_refused() {
    let (_dir, mut session) = opened();
    let result = session
        .add_track(&AddTrackRequest {
            name: "Master 2".to_string(),
            kind: TrackKind::Master as i32,
            r#ref: None,
            dry_run: false,
        })
        .unwrap();

    assert!(!result.valid);
    assert!(rules(&result).contains(&"master_track_count"), "{:?}", rules(&result));
}

// ---- set_track_instrument ----

#[test]
fn an_instrument_track_arrives_with_its_instrument() {
    // ADR 0002: `instrument` is present exactly when `kind == INSTRUMENT`. There is no valid
    // song in which an instrument track is waiting to be filled in, so the tool cannot leave
    // one there — §5 validates every call, and a two-call sequence would have to pass through
    // a document §4.4 refuses.
    let (_dir, mut session) = opened();
    let track = instrument_track(&mut session, "Bass");

    let instrument = session.project().song().tracks[&track].instrument.clone().unwrap();
    assert_eq!(instrument.version, 1);
    assert!(instrument.r#ref.is_some());
}

#[test]
fn an_instrument_track_without_an_instrument_is_refused_at_the_argument() {
    let (_dir, mut session) = opened();
    let result = session
        .add_track(&AddTrackRequest {
            name: "Bass".to_string(),
            kind: TrackKind::Instrument as i32,
            r#ref: None,
            dry_run: false,
        })
        .unwrap();

    assert!(!result.valid);
    assert_eq!(rules(&result), vec!["instrument_presence"]);
    assert_eq!(result.errors[0].path, "/ref", "the error names the argument, not a track");
}

#[test]
fn an_instrument_on_a_bus_track_is_refused_at_the_argument() {
    let (_dir, mut session) = opened();
    let result = session
        .add_track(&AddTrackRequest {
            name: "Reverb bus".to_string(),
            kind: TrackKind::Bus as i32,
            r#ref: cmajor(),
            dry_run: false,
        })
        .unwrap();

    assert!(!result.valid);
    assert_eq!(rules(&result), vec!["instrument_presence"]);
}

#[test]
fn setting_an_instrument_twice_replaces_it() {
    // `add` on an existing object member replaces it (RFC 6902), so a tool never has to ask
    // the document which case it is in.
    let (_dir, mut session) = opened();
    let track = instrument_track(&mut session, "Bass");
    let request =
        SetTrackInstrumentRequest { track_id: track.clone(), r#ref: cmajor(), dry_run: false };

    session.set_track_instrument(&request).unwrap();
    let first = session.project().song().tracks[&track].instrument.clone().unwrap().id;
    let result = session.set_track_instrument(&request).unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let second = session.project().song().tracks[&track].instrument.clone().unwrap().id;
    assert_ne!(first, second, "the instrument was replaced, not merged");
}

#[test]
fn an_instrument_on_an_unknown_track_is_refused_by_the_tool() {
    let (_dir, mut session) = opened();
    let result = session
        .set_track_instrument(&SetTrackInstrumentRequest {
            track_id: "01ZZZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
            r#ref: cmajor(),
            dry_run: false,
        })
        .unwrap();

    assert!(!result.valid);
    assert_eq!(rules(&result), vec!["track_unknown"]);
    assert_eq!(result.errors[0].path, "/track_id", "the error names the argument");
}

#[test]
fn an_instrument_on_a_master_track_is_refused() {
    let (_dir, mut session) = opened();
    let master = newest_track(&session);
    let result = session
        .set_track_instrument(&SetTrackInstrumentRequest {
            track_id: master,
            r#ref: cmajor(),
            dry_run: false,
        })
        .unwrap();

    assert!(!result.valid);
    assert!(rules(&result).contains(&"instrument_presence"), "{:?}", rules(&result));
}

// ---- add_effect ----

#[test]
fn effects_append_in_chain_order() {
    let (_dir, mut session) = opened();
    let track = instrument_track(&mut session, "Bass");

    for _ in 0..3 {
        let result = session
            .add_effect(&AddEffectRequest {
                track_id: track.clone(),
                r#ref: cmajor(),
                index: None,
                dry_run: false,
            })
            .unwrap();
        assert!(result.valid, "{:?}", result.errors);
    }

    let chain = &session.project().song().tracks[&track].fx_chain;
    let mut indexes: Vec<u32> = chain.values().map(|e| e.index).collect();
    indexes.sort_unstable();
    assert_eq!(indexes, vec![0, 1, 2]);
}

#[test]
fn two_effects_at_one_index_are_refused() {
    // The deferred `index` item (`docs/plan.md`): the failure is loud, not silent.
    let (_dir, mut session) = opened();
    let track = instrument_track(&mut session, "Bass");

    let at_zero = AddEffectRequest {
        track_id: track.clone(),
        r#ref: cmajor(),
        index: Some(0),
        dry_run: false,
    };
    assert!(session.add_effect(&at_zero).unwrap().valid);
    let result = session.add_effect(&at_zero).unwrap();

    assert!(!result.valid);
    assert!(rules(&result).contains(&"effect_index_duplicate"), "{:?}", rules(&result));
}

// ---- set_param ----

#[test]
fn a_parameter_is_found_by_device_id_alone() {
    // §4.3 makes ids globally unique, so a caller does not have to know which track a device
    // is on — which is also what `ParamRef` relies on.
    let (_dir, mut session) = opened();
    let track = instrument_track(&mut session, "Bass");
    let device = session.project().song().tracks[&track].instrument.clone().unwrap().id;

    let result = session
        .set_param(&SetParamRequest {
            device_id: device,
            param: "drive".to_string(),
            value: 0.62,
            dry_run: false,
        })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let params = &session.project().song().tracks[&track].instrument.as_ref().unwrap().params;
    assert_eq!(params["drive"], 0.62);
}

#[test]
fn a_non_finite_value_is_refused_at_the_argument() {
    // `json!(f64::NAN)` is `Value::Null`, not a number. Without this check a NaN becomes a
    // null inside an operation and surfaces much later as `op_illegal_for_schema`, naming a
    // JSON pointer rather than the argument that was wrong.
    let (_dir, mut session) = opened();
    let track = instrument_track(&mut session, "Bass");
    let device = session.project().song().tracks[&track].instrument.clone().unwrap().id;

    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let result = session
            .set_param(&SetParamRequest {
                device_id: device.clone(),
                param: "drive".to_string(),
                value: bad,
                dry_run: false,
            })
            .unwrap();
        assert!(!result.valid, "{bad} was accepted");
        assert_eq!(rules(&result), vec!["double_not_finite"]);
        assert_eq!(result.errors[0].path, "/value");
    }
}

#[test]
fn a_parameter_name_with_a_slash_is_escaped_into_the_pointer() {
    // RFC 6901: `/` in a reference token is `~1`. A parameter called `filter/cutoff` would
    // otherwise address a nested object that does not exist.
    let (_dir, mut session) = opened();
    let track = instrument_track(&mut session, "Bass");
    let device = session.project().song().tracks[&track].instrument.clone().unwrap().id;

    let result = session
        .set_param(&SetParamRequest {
            device_id: device,
            param: "filter/cutoff".to_string(),
            value: 0.5,
            dry_run: false,
        })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let patch: Value = serde_json::from_slice(&result.patch).unwrap();
    assert!(
        patch[0]["path"].as_str().unwrap().contains("~1"),
        "the slash was not escaped: {}",
        patch[0]["path"]
    );
    let params = &session.project().song().tracks[&track].instrument.as_ref().unwrap().params;
    assert_eq!(params["filter/cutoff"], 0.5);
}

#[test]
fn an_unknown_device_and_an_unnamed_parameter_are_refused() {
    let (_dir, mut session) = opened();
    let unknown = session
        .set_param(&SetParamRequest {
            device_id: "01ZZZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
            param: "drive".to_string(),
            value: 0.5,
            dry_run: false,
        })
        .unwrap();
    assert_eq!(rules(&unknown), vec!["device_unknown"]);

    let track = instrument_track(&mut session, "Bass");
    let device = session.project().song().tracks[&track].instrument.clone().unwrap().id;

    let unnamed = session
        .set_param(&SetParamRequest {
            device_id: device,
            param: String::new(),
            value: 0.5,
            dry_run: false,
        })
        .unwrap();
    assert_eq!(rules(&unnamed), vec!["param_empty"]);
}

// ---- across the four ----

#[test]
fn a_dry_run_of_a_typed_tool_writes_nothing_and_previews_the_same_patch() {
    let (dir, mut session) = opened();
    let before = std::fs::read_to_string(dir.0.join("song.json")).unwrap();

    let previewed = session
        .add_track(&AddTrackRequest {
            name: "Bass".to_string(),
            kind: TrackKind::Instrument as i32,
            r#ref: cmajor(),
            dry_run: true,
        })
        .unwrap();

    assert!(previewed.valid, "{:?}", previewed.errors);
    assert!(previewed.entry_id.is_empty());
    assert_eq!(std::fs::read_to_string(dir.0.join("song.json")).unwrap(), before);
    assert_eq!(session.project().song().tracks.len(), 1);
}

#[test]
fn a_scripted_session_is_reproducible() {
    // §11 through the typed tools rather than through raw patches: the ids these tools mint
    // come from the injected source, so the same script twice is the same bytes twice.
    let build = || {
        let (dir, mut session) = opened();
        session
            .add_track(&AddTrackRequest {
                name: "Bass".to_string(),
                kind: TrackKind::Instrument as i32,
                r#ref: cmajor(),
                dry_run: false,
            })
            .unwrap();
        let track = newest_track(&session);
        session
            .add_effect(&AddEffectRequest {
                track_id: track,
                r#ref: cmajor(),
                index: None,
                dry_run: false,
            })
            .unwrap();
        let song = std::fs::read_to_string(dir.0.join("song.json")).unwrap();
        let refs = std::fs::read_to_string(dir.0.join("refs.json")).unwrap();
        (song, refs)
    };
    assert_eq!(build(), build());
}

#[test]
fn an_enum_the_schema_does_not_know_is_refused_not_panicked() {
    // prost keeps an unknown enum as its raw i32 and the generated serializer then fails on
    // it. Panicking here would happen *inside* a transport's lock: one gRPC request carrying
    // `kind: 99` would take the process down and poison the session for every later call,
    // while MCP's JSON deserializer rejects the same value up front. Two transports must not
    // disagree about what a call means (ADR 0006).
    let (_dir, mut session) = opened();
    let result = session
        .add_track(&AddTrackRequest {
            name: "Bass".to_string(),
            kind: 99,
            r#ref: None,
            dry_run: false,
        })
        .unwrap();

    assert!(!result.valid);
    assert_eq!(rules(&result), vec!["enum_unknown"]);
}

#[test]
fn set_notes_keeps_the_provenance_of_a_note_it_recognises() {
    // Re-minting provenance would rewrite the author and `created_at` of every note in a clip
    // on every call. Under a fixed clock that is a no-op, which is exactly why it needs a test
    // that moves the clock: with a real one it loses authorship, bumps notes nobody touched,
    // and makes two branches conflict on timestamps for notes neither meant to change.
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let created = FixedClock(AT);
    let song = new_song(&mut ids, &created);
    let project = Project::create(&dir.0, &song, &mut ids, &created).unwrap();
    let mut session = Session::new(project, Box::new(ids), Box::new(created), Author::Human);
    let track = instrument_track(&mut session, "Bass");

    let note = |pitch: i32| escribass_schema::song::Note {
        pitch,
        start_tick: 0,
        length_ticks: 240,
        velocity: 100,
        ..Default::default()
    };
    session
        .add_clip(&escribass_proto::tools::AddClipRequest {
            track_id: track,
            start_tick: 0,
            length_ticks: 3840,
            content: Some(escribass_proto::tools::add_clip_request::Content::NoteClip(
                escribass_schema::song::NoteClip {
                    notes: [("a".to_string(), note(60))].into_iter().collect(),
                },
            )),
            dry_run: false,
        })
        .unwrap();
    let clip = session.project().song().clips.keys().next().unwrap().clone();
    let held = match &session.project().song().clips[&clip].content {
        Some(escribass_schema::song::clip::Content::NoteClip(n)) => n.notes.clone(),
        other => panic!("{other:?}"),
    };
    let (id, before) = held.iter().next().unwrap();

    // A later session, a minute on, run by a model rather than the person who wrote the note.
    let reopened = Project::open(&dir.0).unwrap();
    let mut later = Session::new(
        reopened,
        Box::new(SeededIds::new(AT + 60_000, 100)),
        Box::new(FixedClock(AT + 60_000)),
        Author::Model,
    );
    let result = later
        .set_notes(&escribass_proto::tools::SetNotesRequest {
            clip_id: clip.clone(),
            notes: [(id.clone(), note(60))].into_iter().collect(),
            dry_run: false,
        })
        .unwrap();
    assert!(result.valid, "{:?}", result.errors);

    let after = match &later.project().song().clips[&clip].content {
        Some(escribass_schema::song::clip::Content::NoteClip(n)) => n.notes.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(after[id].provenance, before.provenance, "the note's authorship was rewritten");
    assert_eq!(after[id].version, before.version, "an unchanged note bumped");
}

#[test]
fn negative_zero_is_normalised_for_a_typed_tool_too() {
    // `core/AGENTS.md` says the validator's `negative_zero` should never fire on tool input.
    // Normalising only in `apply_patch` left every typed tool to be refused by it.
    let (_dir, mut session) = opened();
    let track = instrument_track(&mut session, "Bass");
    let device = session.project().song().tracks[&track].instrument.clone().unwrap().id;

    let result = session
        .set_param(&SetParamRequest {
            device_id: device,
            param: "pan".to_string(),
            value: -0.0,
            dry_run: false,
        })
        .unwrap();

    assert!(result.valid, "{:?}", result.errors);
    let text = escribass_core::to_canonical_json(session.project().song()).unwrap();
    assert!(!text.contains("-0.0"), "a negative zero reached the document");
}
