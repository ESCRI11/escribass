//! `compile`: what the plan resolves that the engine does not (ADR 0007 §1), and what M1
//! refuses (ADR 0007 §6).
//!
//! Every song here is built through `Session` — the tool API — never by writing fields into a
//! stored document (CLAUDE.md #2). What a shape the tool API cannot produce directly needs,
//! `apply_patch` gives it: a loop length, a mute, a routing, a render kind.
//!
//! The plan golden in `tests/determinism` is what pins the bytes; these pin the rules, one
//! claim each, so a golden that moves can be read against a rule that changed.

mod common;
use common::manifest;

use escribass_core::render::UNSUPPORTED;
use escribass_core::{compile, new_song, FixedClock, Project, SeededIds, Session};
use escribass_proto::render::{plan_clip, PlanClip, PlanTrack, RenderPlan};
use escribass_proto::tools::add_clip_request::Content as AddClipContent;
use escribass_proto::tools::{
    AddAssetRequest, AddAutomationRequest, AddClipRequest, AddEffectRequest, AddSectionRequest,
    AddTrackRequest, ApplyPatchRequest, SetTempoRequest, ToolResult,
};
use escribass_schema::song::device_ref::Kind;
use escribass_schema::song::{
    AudioClip, Author, AutomationPoint, Curve, DeviceRef, ModelRef, Note, NoteClip, ParamRef,
    PluginRef, SamplerRef, SourceRef, TrackKind,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const AT: i64 = 1_788_307_200_000;
const BAR: i32 = 3840;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-render-{}-{}.escri",
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

fn opened() -> (Scratch, Session) {
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock, Author::Model);
    let project = Project::create(&dir.0, &song, &mut ids, &clock, Author::Model, manifest()).unwrap();
    (dir, Session::new(project, Box::new(ids), Box::new(clock), Author::Model))
}

fn device(kind: Kind) -> Option<DeviceRef> {
    Some(DeviceRef { kind: Some(kind) })
}

/// Surge XT as it names itself: vendor and class name, which is what the VST3 factory
/// reports and what the manifest keys an entry by (ADR 0010 §4, refined in PR 6).
const SURGE: &str = "Surge Synth Team/Surge XT";

/// Three of Surge XT's own parameter ids, from the manifest fixture. They are opaque
/// integers because that is all a VST3 host is shown: JUCE's wrapper hashes the readable
/// internal id away, and display names are not unique (ADR 0010 §4). The names are here for
/// the reader; `param_unknown` compares the ids.
const CUTOFF: &str = "1945359057"; // A Filter 1 Cutoff
const RESONANCE: &str = "8095466"; // A Filter 1 Resonance
const DRIVE: &str = "1243907205"; // A Waveshaper Drive

fn plugin() -> Option<DeviceRef> {
    device(Kind::Plugin(PluginRef {
        plugin_id: SURGE.to_string(),
        version: "1.3.4".to_string(),
    }))
}

fn source(hash: &str) -> SourceRef {
    SourceRef { source_hash: hash.to_string() }
}

fn ok(result: ToolResult) -> ToolResult {
    assert!(result.valid, "{:?}", result.errors);
    result
}

/// The one key `after` has that `before` did not: the id of what a tool just added.
fn added(before: &BTreeSet<String>, after: impl Iterator<Item = String>) -> String {
    let mut new: Vec<String> = after.filter(|k| !before.contains(k)).collect();
    assert_eq!(new.len(), 1, "expected one new entity, found {new:?}");
    new.remove(0)
}

fn track(session: &mut Session, kind: TrackKind, r#ref: Option<DeviceRef>) -> String {
    let before: BTreeSet<String> = session.project().song().tracks.keys().cloned().collect();
    ok(session
        .add_track(&AddTrackRequest { name: "t".to_string(), kind: kind as i32, r#ref, dry_run: false })
        .unwrap());
    added(&before, session.project().song().tracks.keys().cloned())
}

fn instrument_of(session: &Session, track: &str) -> String {
    session.project().song().tracks[track].instrument.as_ref().unwrap().id.clone()
}

fn master_of(session: &Session) -> String {
    let tracks = session.project().song().tracks.values();
    tracks.filter(|t| t.kind == TrackKind::Master as i32).map(|t| t.id.clone()).next().unwrap()
}

fn cmajor() -> Option<DeviceRef> {
    device(Kind::Cmajor(source("8b31c0de4f9c00000000000000000000")))
}

fn sampler(hash: &str) -> Option<DeviceRef> {
    device(Kind::Sampler(SamplerRef { sfz_hash: hash.to_string() }))
}

fn effect(session: &mut Session, track: &str, r#ref: Option<DeviceRef>, index: u32) -> String {
    let before: BTreeSet<String> =
        session.project().song().tracks[track].fx_chain.keys().cloned().collect();
    ok(session
        .add_effect(&AddEffectRequest {
            track_id: track.to_string(),
            r#ref,
            index: Some(index),
            dry_run: false,
        })
        .unwrap());
    added(&before, session.project().song().tracks[track].fx_chain.keys().cloned())
}

fn note(pitch: i32, start_tick: i32, length_ticks: i32) -> Note {
    Note { pitch, start_tick, length_ticks, velocity: 100, ..Default::default() }
}

fn clip(session: &mut Session, track: &str, start: i32, length: i32, notes: &[Note]) -> String {
    let before: BTreeSet<String> = session.project().song().clips.keys().cloned().collect();
    let notes = notes.iter().enumerate().map(|(i, n)| (i.to_string(), n.clone())).collect();
    ok(session
        .add_clip(&AddClipRequest {
            track_id: track.to_string(),
            start_tick: start,
            length_ticks: length,
            content: Some(AddClipContent::NoteClip(NoteClip { notes })),
            dry_run: false,
        })
        .unwrap());
    added(&before, session.project().song().clips.keys().cloned())
}

fn audio_clip(session: &mut Session, track: &str, length: i32, audio: AudioClip) -> String {
    let before: BTreeSet<String> = session.project().song().clips.keys().cloned().collect();
    ok(session
        .add_clip(&AddClipRequest {
            track_id: track.to_string(),
            start_tick: 0,
            length_ticks: length,
            content: Some(AddClipContent::AudioClip(audio)),
            dry_run: false,
        })
        .unwrap());
    added(&before, session.project().song().clips.keys().cloned())
}

fn automation(session: &mut Session, device: &str, param: &str, points: &[(i32, f64)]) {
    let points = points
        .iter()
        .enumerate()
        .map(|(i, (tick, value))| {
            let point = AutomationPoint {
                tick: *tick,
                value: *value,
                curve: Curve::Linear as i32,
                ..Default::default()
            };
            (i.to_string(), point)
        })
        .collect();
    ok(session
        .add_automation(&AddAutomationRequest {
            target: Some(ParamRef { device_id: device.to_string(), param: param.to_string() }),
            points,
            dry_run: false,
        })
        .unwrap());
}

fn patch(session: &mut Session, ops: Value) {
    let request = ApplyPatchRequest { patch: serde_json::to_vec(&ops).unwrap(), dry_run: false };
    ok(session.apply_patch(&request).unwrap());
}

fn set(session: &mut Session, path: String, value: Value) {
    patch(session, json!([{"op": "add", "path": path, "value": value}]));
}

/// The index `compile` takes, read from the directory the session wrote — the way the
/// session will build it for `render_export`.
fn assets(dir: &Scratch) -> BTreeMap<String, PathBuf> {
    std::fs::read_dir(dir.0.join("assets"))
        .map(|listing| {
            listing
                .map(|entry| {
                    let path = entry.unwrap().path();
                    (path.file_name().unwrap().to_string_lossy().into_owned(), path)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn compiled(session: &Session) -> RenderPlan {
    compile(session.project().song(), &BTreeMap::new()).unwrap_or_else(|e| panic!("{e:?}"))
}

fn refused(session: &Session, assets: &BTreeMap<String, PathBuf>) -> Vec<(String, &'static str)> {
    match compile(session.project().song(), assets) {
        Ok(plan) => panic!("compiled rather than refused: {plan:?}"),
        Err(v) => v.into_iter().map(|v| (v.path, v.rule)).collect(),
    }
}

fn starts(track: &PlanTrack) -> Vec<i32> {
    track.clips.iter().map(|c| c.start_tick).collect()
}

fn pitches(clip: &PlanClip) -> Vec<(i32, i32, i32)> {
    match &clip.content {
        Some(plan_clip::Content::Notes(held)) => {
            held.notes.iter().map(|n| (n.pitch, n.start_tick, n.length_ticks)).collect()
        }
        other => panic!("not a note clip: {other:?}"),
    }
}

// ---- what the plan resolves (ADR 0007 §1) ----

#[test]
fn an_empty_song_is_master_and_nothing_else() {
    let (_dir, session) = opened();
    let plan = compiled(&session);

    assert!(plan.tracks.is_empty());
    let master = plan.master.expect("a master");
    assert!(master.instrument.is_none() && master.effects.is_empty() && master.clips.is_empty());
    assert_eq!(master.mix.unwrap().gain_db, 0.0);
    // `new_song`'s one tempo event, with its key blanked.
    assert_eq!(plan.tempo.len(), 1);
    assert_eq!((plan.tempo[0].tick, plan.tempo[0].bpm, plan.tempo[0].id.as_str()), (0, 120.0, ""));
    assert_eq!(plan.length_ticks, 0);
    // `render_export`'s argument, not the document's (ADR 0007 §4).
    assert_eq!(plan.output_path, "");
}

#[test]
fn clips_arrive_on_their_track_in_timeline_order() {
    let (_dir, mut session) = opened();
    let a = track(&mut session, TrackKind::Instrument, plugin());
    let b = track(&mut session, TrackKind::Instrument, plugin());
    // Added later-first, and to the second track first: neither insertion order nor id order
    // is timeline order, and the plan must not be either.
    clip(&mut session, &b, 2 * BAR, BAR, &[note(72, 0, 240)]);
    clip(&mut session, &a, BAR, BAR, &[note(60, 0, 240)]);
    clip(&mut session, &a, 0, BAR, &[note(62, 0, 240)]);
    clip(&mut session, &b, 0, BAR, &[note(74, 0, 240)]);

    let plan = compiled(&session);
    assert_eq!(plan.tracks.len(), 2, "tracks in mixer order, master apart");
    assert_eq!(starts(&plan.tracks[0]), vec![0, BAR]);
    assert_eq!(starts(&plan.tracks[1]), vec![0, 2 * BAR]);
    assert_eq!(pitches(&plan.tracks[0].clips[0]), vec![(62, 0, 240)]);
    assert_eq!(pitches(&plan.tracks[1].clips[1]), vec![(72, 0, 240)]);
    assert_eq!(plan.length_ticks, 3 * BAR);
}

#[test]
fn the_chain_is_in_index_order() {
    let (_dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Instrument, plugin());
    effect(&mut session, &t, plugin(), 7);
    effect(&mut session, &t, plugin(), 3);
    effect(&mut session, &t, plugin(), 5);

    let plan = compiled(&session);
    let indexes: Vec<u32> =
        plan.tracks[0].effects.iter().map(|e| e.effect.as_ref().unwrap().index).collect();
    assert_eq!(indexes, vec![3, 5, 7]);
}

#[test]
fn a_loop_arrives_unrolled() {
    // A clip of 3000 ticks looping its first 960: three whole iterations and one of 120.
    // Its content is the first 960 ticks — the note at 800 is cut at the loop's end, the note
    // at 1200 is not content and never sounds — and the last iteration keeps only what fits.
    let (_dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Instrument, plugin());
    let notes = [note(60, 0, 240), note(64, 800, 400), note(67, 1200, 240)];
    let c = clip(&mut session, &t, BAR, 3000, &notes);
    set(&mut session, format!("/clips/{c}/loop_length_ticks"), json!(960));

    let plan = compiled(&session);
    let clips = &plan.tracks[0].clips;
    assert_eq!(starts(&plan.tracks[0]), vec![BAR, BAR + 960, BAR + 1920, BAR + 2880]);
    assert_eq!(clips.iter().map(|c| c.length_ticks).collect::<Vec<_>>(), vec![960, 960, 960, 120]);
    for whole in &clips[..3] {
        assert_eq!(pitches(whole), vec![(60, 0, 240), (64, 800, 160)]);
    }
    assert_eq!(pitches(&clips[3]), vec![(60, 0, 120)]);
    assert_eq!(plan.length_ticks, BAR + 3000);
}

#[test]
fn solo_and_mute_decide_which_tracks_arrive() {
    let (_dir, mut session) = opened();
    let ids = [
        track(&mut session, TrackKind::Instrument, plugin()),
        track(&mut session, TrackKind::Instrument, plugin()),
        track(&mut session, TrackKind::Instrument, plugin()),
    ];
    // Gains tell the tracks apart in a plan that carries no name.
    for (n, id) in ids.iter().enumerate() {
        set(&mut session, format!("/tracks/{id}/mix/gain_db"), json!(-(n as f64) - 1.0));
    }
    let gains = |plan: &RenderPlan| -> Vec<f64> {
        plan.tracks.iter().map(|t| t.mix.as_ref().unwrap().gain_db).collect()
    };

    set(&mut session, format!("/tracks/{}/mix/mute", ids[1]), json!(true));
    assert_eq!(gains(&compiled(&session)), vec![-1.0, -3.0], "mute drops its own track");

    set(&mut session, format!("/tracks/{}/mix/solo", ids[2]), json!(true));
    let plan = compiled(&session);
    assert_eq!(gains(&plan), vec![-3.0], "solo drops every track that is not soloed");
    let mix = plan.tracks[0].mix.as_ref().unwrap();
    assert!(!mix.solo && !mix.mute, "both cross false: they were applied here (ADR 0007 §5)");

    // Mute beats solo: a track that is both is silent, as its mute button says.
    set(&mut session, format!("/tracks/{}/mix/mute", ids[2]), json!(true));
    assert!(compiled(&session).tracks.is_empty());
}

#[test]
fn a_muted_master_is_silence_of_the_right_length() {
    let (_dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Instrument, plugin());
    clip(&mut session, &t, 0, BAR, &[note(60, 0, 240)]);
    let master = master_of(&session);
    set(&mut session, format!("/tracks/{master}/mix/mute"), json!(true));

    let plan = compiled(&session);
    assert!(plan.tracks.is_empty());
    assert_eq!(plan.length_ticks, BAR, "the length is the song's, not what sounds");
}

#[test]
fn automation_nests_under_the_device_it_targets() {
    let (_dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Instrument, plugin());
    let i = instrument_of(&session, &t);
    let e = effect(&mut session, &t, plugin(), 0);
    let muted = track(&mut session, TrackKind::Instrument, plugin());
    let orphan = instrument_of(&session, &muted);
    set(&mut session, format!("/tracks/{muted}/mix/mute"), json!(true));

    // Two entities on one parameter are one curve; a later entity's earlier point sorts first.
    automation(&mut session, &i, CUTOFF, &[(0, 0.1), (1920, 0.9)]);
    automation(&mut session, &i, CUTOFF, &[(960, 0.5)]);
    automation(&mut session, &i, RESONANCE, &[(0, 0.3)]);
    automation(&mut session, &e, DRIVE, &[(480, 0.7)]);
    // A lane on a device the render never sees changes no sample: dropped with its track.
    automation(&mut session, &orphan, CUTOFF, &[(0, 1.0)]);

    let plan = compiled(&session);
    assert_eq!(plan.tracks.len(), 1);
    let instrument = plan.tracks[0].instrument.as_ref().unwrap();
    let lanes: Vec<(&str, Vec<(i32, f64)>)> = instrument
        .lanes
        .iter()
        .map(|l| (l.param.as_str(), l.points.iter().map(|p| (p.tick, p.value)).collect()))
        .collect();
    assert_eq!(
        lanes,
        vec![(CUTOFF, vec![(0, 0.1), (960, 0.5), (1920, 0.9)]), (RESONANCE, vec![(0, 0.3)])],
        "lanes in parameter order, points in tick order"
    );
    let effect = &plan.tracks[0].effects[0];
    assert_eq!(effect.lanes.len(), 1);
    assert_eq!((effect.lanes[0].param.as_str(), effect.lanes[0].points[0].tick), (DRIVE, 480));
    assert!(instrument.lanes.iter().all(|l| l.points.iter().all(|p| p.id.is_empty())));
}

#[test]
fn a_lane_naming_a_track_nests_under_that_tracks_mix() {
    // ADR 0015 §1: ids are globally unique across every collection, so the same `device_id`
    // that names a device names a track, and `take_lanes` needs no second lookup to tell them
    // apart. ADR 0015 §2: the points are decibels and -1..1, not a plugin's normalised 0..1,
    // which is why -24.0 below is a lane value and not a violation.
    let (_dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Instrument, plugin());
    let master = master_of(&session);
    let muted = track(&mut session, TrackKind::Instrument, plugin());
    set(&mut session, format!("/tracks/{muted}/mix/mute"), json!(true));

    automation(&mut session, &t, "pan", &[(0, -1.0), (1920, 1.0)]);
    automation(&mut session, &t, "gain_db", &[(0, -24.0), (960, 0.0)]);
    automation(&mut session, &master, "gain_db", &[(0, -6.0)]);
    // A fader on a track that does not sound is dropped with it, as a device's lane is.
    automation(&mut session, &muted, "gain_db", &[(0, 12.0)]);

    let plan = compiled(&session);
    assert_eq!(plan.tracks.len(), 1);
    let lanes: Vec<(&str, Vec<(i32, f64)>)> = plan.tracks[0]
        .mix_lanes
        .iter()
        .map(|l| (l.param.as_str(), l.points.iter().map(|p| (p.tick, p.value)).collect()))
        .collect();
    assert_eq!(
        lanes,
        vec![("gain_db", vec![(0, -24.0), (960, 0.0)]), ("pan", vec![(0, -1.0), (1920, 1.0)])],
        "lanes in parameter order, points in tick order"
    );
    // The instrument on the same track keeps its own lanes, and neither took the other's.
    assert!(plan.tracks[0].instrument.as_ref().unwrap().lanes.is_empty());
    let master = plan.master.as_ref().unwrap();
    assert_eq!(master.mix_lanes.len(), 1, "the master fader is automated like any other");
    assert_eq!(master.mix_lanes[0].param, "gain_db");
    assert!(master.mix_lanes[0].points.iter().all(|p| p.id.is_empty()));
}

#[test]
fn the_render_ends_at_the_last_clip_or_section() {
    // docs/plan.md, decisions of 2026-09-04: whichever is later. A section that outlasts
    // every clip is an outro someone named, and it renders as the silence they wrote.
    let (_dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Instrument, plugin());
    clip(&mut session, &t, 0, BAR, &[note(60, 0, 240)]);
    assert_eq!(compiled(&session).length_ticks, BAR);

    ok(session
        .add_section(&AddSectionRequest {
            name: "Outro".to_string(),
            start_tick: BAR,
            end_tick: 3 * BAR,
            dry_run: false,
        })
        .unwrap());
    assert_eq!(compiled(&session).length_ticks, 3 * BAR, "the section extends past the clip");

    clip(&mut session, &t, 3 * BAR, 100, &[]);
    assert_eq!(compiled(&session).length_ticks, 3 * BAR + 100, "and a clip extends past it");
}

#[test]
fn the_tempo_map_is_in_tick_order() {
    let (_dir, mut session) = opened();
    for (bpm, tick) in [(90.0, 2 * BAR), (132.5, BAR)] {
        ok(session.set_tempo(&SetTempoRequest { bpm, tick, dry_run: false }).unwrap());
    }
    let plan = compiled(&session);
    let map: Vec<(i32, f64)> = plan.tempo.iter().map(|e| (e.tick, e.bpm)).collect();
    assert_eq!(map, vec![(0, 120.0), (BAR, 132.5), (2 * BAR, 90.0)]);
}

#[test]
fn the_plan_carries_no_id_provenance_or_version() {
    // ADR 0007 §2's check, and ADR 0001's: history metadata never reaches the engine. Walked
    // over the serialised plan so a message embedded later is covered without a new case.
    // `PluginRef.version` is a string naming a release and stays; an entity version is a
    // number and must be 0.
    let (_dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Instrument, plugin());
    let i = instrument_of(&session, &t);
    let e = effect(&mut session, &t, plugin(), 0);
    clip(&mut session, &t, 0, BAR, &[note(60, 0, 240)]);
    automation(&mut session, &i, CUTOFF, &[(0, 0.1)]);
    automation(&mut session, &e, DRIVE, &[(0, 0.1)]);
    ok(session.set_tempo(&SetTempoRequest { bpm: 90.0, tick: BAR, dry_run: false }).unwrap());

    let plan = serde_json::to_value(compiled(&session)).unwrap();
    let mut seen = 0;
    let mut pending = vec![(String::new(), &plan)];
    while let Some((at, value)) = pending.pop() {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    let path = format!("{at}/{key}");
                    match key.as_str() {
                        "id" => assert_eq!(inner, &json!(""), "{path}"),
                        "provenance" => panic!("{path} is present"),
                        "version" if inner.is_number() => assert_eq!(inner, &json!(0), "{path}"),
                        _ => {}
                    }
                    if ["id", "provenance", "version"].contains(&key.as_str()) {
                        seen += 1;
                    }
                    pending.push((path, inner));
                }
            }
            Value::Array(items) => pending.extend(items.iter().map(|v| (at.clone(), v))),
            _ => {}
        }
    }
    assert!(seen >= 8, "the walk found only {seen} §4.3 fields, so it is not walking the plan");
}

// ---- what M1 refuses (ADR 0007 §6) ----

#[test]
fn everything_m1_cannot_render_is_refused_at_once() {
    // One document carrying every refusal, so the claim "all of them, not the first" is the
    // thing asserted. The validator accepts all of it: a bus is a legal routing target, a
    // sidechain names an effect on this track and a track in this song, and a Cmajor source
    // is a device the model can hold (ADR 0007 §6).
    let (_dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Instrument, cmajor());
    let faust = effect(&mut session, &t, device(Kind::Faust(source("3c1de4f98b"))), 0);
    let model = ModelRef { model_hash: "ab12".to_string() };
    let neural = effect(&mut session, &t, device(Kind::Neural(model)), 1);
    // A sampler is an instrument. The model holds one DeviceRef in both places, so a sampler
    // on a chain is a legal document and not a renderable one.
    let sampled = effect(&mut session, &t, sampler("3c1de4f98b"), 2);
    let bus = track(&mut session, TrackKind::Bus, None);
    patch(
        &mut session,
        json!([
            {"op": "add", "path": format!("/tracks/{t}/routing/output_track_id"), "value": bus},
            {"op": "add", "path": format!("/tracks/{t}/routing/sends/{bus}"), "value": -6.0},
            {"op": "add", "path": format!("/tracks/{t}/routing/sidechains/{faust}"), "value": bus},
            {"op": "replace", "path": "/render_target/kind", "value": "RENDER_KIND_STEMS"},
            {"op": "replace", "path": "/render_target/dither", "value": true},
        ]),
    );

    let refused = refused(&session, &BTreeMap::new());
    assert!(refused.iter().all(|(_, rule)| *rule == UNSUPPORTED), "{refused:?}");
    let mut paths: Vec<String> = refused.into_iter().map(|(path, _)| path).collect();
    let mut expected = vec![
        "/render_target/dither".to_string(),
        "/render_target/kind".to_string(),
        format!("/tracks/{bus}/kind"),
        format!("/tracks/{t}/fx_chain/{faust}/ref/faust"),
        format!("/tracks/{t}/fx_chain/{neural}/ref/neural"),
        format!("/tracks/{t}/fx_chain/{sampled}/ref/sampler"),
        format!("/tracks/{t}/instrument/ref/cmajor"),
        format!("/tracks/{t}/routing/output_track_id"),
        format!("/tracks/{t}/routing/sends"),
        format!("/tracks/{t}/routing/sidechains"),
    ];
    expected.sort();
    assert_eq!(paths, expected, "sorted, so the refusal is byte-stable");
    paths.dedup();
    assert_eq!(paths.len(), expected.len());
}

#[test]
fn a_note_m1_cannot_voice_is_refused_rather_than_flattened() {
    // Both fields cross into the plan and both would render as though they were zero, which
    // is a note at the wrong pitch and a render that reports success (ADR 0007 §6). Refused
    // once per note, not once per loop iteration.
    let (_dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Instrument, plugin());
    let bent = Note { microtonal_cents: -13.5, ..note(60, 0, 240) };
    let c = clip(&mut session, &t, 0, 960, &[bent, note(62, 480, 240)]);
    let ids: Vec<String> = match &session.project().song().clips[&c].content {
        Some(escribass_schema::song::clip::Content::NoteClip(n)) => n.notes.keys().cloned().collect(),
        _ => panic!("the clip is a note clip"),
    };
    // Looping, so every note is in the plan twice and the refusal is still one per note.
    set(&mut session, format!("/clips/{c}/loop_length_ticks"), json!(480));
    set(&mut session, format!("/clips/{c}/note_clip/notes/{}/expression/pressure", ids[1]), json!(0.5));

    let refused = refused(&session, &BTreeMap::new());
    assert_eq!(
        refused,
        vec![
            (format!("/clips/{c}/note_clip/notes/{}/microtonal_cents", ids[0]), UNSUPPORTED),
            (format!("/clips/{c}/note_clip/notes/{}/expression/pressure", ids[1]), UNSUPPORTED),
        ]
    );
}

#[test]
fn what_does_not_sound_is_not_examined() {
    // Muting the Cmajor track is how a project exports the rest of itself before M4. Naming
    // master explicitly is the same as not naming it.
    let (_dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Instrument, cmajor());
    let kept = track(&mut session, TrackKind::Instrument, plugin());
    let master = master_of(&session);
    assert_eq!(refused(&session, &BTreeMap::new()).len(), 1);

    set(&mut session, format!("/tracks/{t}/mix/mute"), json!(true));
    set(&mut session, format!("/tracks/{kept}/routing/output_track_id"), json!(master));
    assert_eq!(compiled(&session).tracks.len(), 1);
}

#[test]
fn an_audio_clip_crosses_beside_its_asset_and_a_missing_one_is_refused() {
    let (dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Audio, None);
    let hash = session
        .add_asset(&AddAssetRequest { content: b"RIFF".to_vec(), dry_run: false })
        .unwrap()
        .asset_hash;
    let audio = AudioClip {
        asset_hash: hash.clone(),
        gain_db: -3.0,
        fade_in_ticks: 10,
        fade_out_ticks: 20,
        time_stretch: false,
    };
    let c = audio_clip(&mut session, &t, BAR, audio.clone());

    // Caller-fixable, so a violation and not a panic (ADR 0006 §2): the asset is not in the
    // index this compile was handed.
    assert_eq!(
        refused(&session, &BTreeMap::new()),
        vec![(format!("/clips/{c}/audio_clip/asset_hash"), "asset_missing")]
    );

    let plan = compile(session.project().song(), &assets(&dir)).unwrap();
    let clips = &plan.tracks[0].clips;
    assert_eq!(clips.len(), 1);
    match &clips[0].content {
        Some(plan_clip::Content::Audio(held)) => {
            assert_eq!(held.clip.as_ref().unwrap(), &audio, "AudioClip by value, every field");
            assert_eq!(PathBuf::from(&held.path), dir.0.join("assets").join(&hash));
        }
        other => panic!("not audio: {other:?}"),
    }
    assert!(plan.tracks[0].instrument.is_none());
}

#[test]
fn a_stretched_loop_fills_a_whole_number_of_loops_or_is_refused() {
    // The `ponytail:` on PlanAudio, met: each iteration crosses as a clip the engine stretches
    // to, so a short last iteration would stretch to the wrong length (ADR 0011 §3). Refused
    // rather than rendered wrong; unstretched, the short iteration simply plays and stops.
    let (dir, mut session) = opened();
    let t = track(&mut session, TrackKind::Audio, None);
    let hash = session
        .add_asset(&AddAssetRequest { content: b"RIFF".to_vec(), dry_run: false })
        .unwrap()
        .asset_hash;
    let stretched = AudioClip { asset_hash: hash.clone(), time_stretch: true, ..Default::default() };
    let c = audio_clip(&mut session, &t, 2000, stretched);
    set(&mut session, format!("/clips/{c}/loop_length_ticks"), json!(960));
    let index = assets(&dir);

    assert_eq!(
        refused(&session, &index),
        vec![(format!("/clips/{c}/loop_length_ticks"), UNSUPPORTED)]
    );

    set(&mut session, format!("/clips/{c}/length_ticks"), json!(1920));
    let plan = compile(session.project().song(), &index).unwrap();
    assert_eq!(starts(&plan.tracks[0]), vec![0, 960]);

    set(&mut session, format!("/clips/{c}/length_ticks"), json!(2000));
    set(&mut session, format!("/clips/{c}/audio_clip/time_stretch"), json!(false));
    let plan = compile(session.project().song(), &index).unwrap();
    let lengths: Vec<i32> = plan.tracks[0].clips.iter().map(|c| c.length_ticks).collect();
    assert_eq!(lengths, vec![960, 960, 80]);
}

#[test]
fn a_sampler_crosses_beside_its_sfz_and_a_missing_one_is_refused() {
    // The audio clip's shape, applied to the instrument (ADR 0007 §2, extended): the SFZ is an
    // asset, so it crosses as the absolute path core resolved from its hash, and the engine
    // learns nothing about where it came from. What the SFZ *says* — which samples it plays,
    // and whether `assets/` holds them — is the file's content, and compile reads no file.
    let (dir, mut session) = opened();
    let sfz = b"<region>\nsample=beef\n".to_vec();
    let hash = session
        .add_asset(&AddAssetRequest { content: sfz, dry_run: false })
        .unwrap()
        .asset_hash;
    let t = track(&mut session, TrackKind::Instrument, sampler(&hash));
    clip(&mut session, &t, 0, BAR, &[note(60, 0, 240)]);

    assert_eq!(
        refused(&session, &BTreeMap::new()),
        vec![(format!("/tracks/{t}/instrument/ref/sampler/sfz_hash"), "asset_missing")],
        "caller-fixable, like an audio clip whose asset is not in the index"
    );

    let plan = compile(session.project().song(), &assets(&dir)).unwrap();
    let instrument = plan.tracks[0].instrument.as_ref().unwrap();
    assert_eq!(PathBuf::from(&instrument.sfz_path), dir.0.join("assets").join(&hash));
    // The hash crosses too: SamplerRef is carried by value inside Instrument, and the path is
    // beside it rather than in place of it.
    match instrument.instrument.as_ref().unwrap().r#ref.as_ref().unwrap().kind.as_ref() {
        Some(escribass_schema::song::device_ref::Kind::Sampler(held)) => {
            assert_eq!(held.sfz_hash, hash)
        }
        other => panic!("not a sampler: {other:?}"),
    }
    // Every other device kind leaves it empty, which is what tells the engine which it is.
    track(&mut session, TrackKind::Instrument, plugin());
    let plan = compile(session.project().song(), &assets(&dir)).unwrap();
    assert!(plan.tracks[1].instrument.as_ref().unwrap().sfz_path.is_empty());
}
