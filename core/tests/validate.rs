//! Validator tests (docs/specs.md §4.4, ADR 0002 Consequences).
//!
//! Each rule is exercised by breaking a valid song in exactly one way and asserting its id
//! appears. The shared starting point is the fixture, so a rule that fires on a *correct*
//! song shows up immediately in `the_fixture_is_valid`.

mod common;
use common::manifest;

use escribass_core::{from_canonical_json, validate};
use escribass_schema::song::*;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");
const BASS: &str = "01M1FPMP00TRACKBASS0000002";
const MASTER: &str = "01M1FPMP00TRACKMASTER00003";
const CLIP: &str = "01M1FPMP00CPCHRS0000000006";
const AUDIO_CLIP: &str = "01M1FPMP00CPGTR0000000000H";
const NOTE_G1: &str = "01M1FPMP00NTEG100000000007";
/// The fixture's Surge XT effect, and one of the parameter ids the manifest fixture declares
/// for it — `A Filter 1 Cutoff`, which is an opaque integer because that is all a VST3 host
/// is shown (ADR 0010 §4).
const SURGE_FX: &str = "01M1FPMP00FXSRGE0000000005";
const CUTOFF: &str = "1945359057";

fn valid_song() -> Song {
    from_canonical_json(&std::fs::read_to_string(FIXTURE).expect("fixture")).expect("parses")
}

/// Breaks a valid song in one way and returns the rule ids that fire.
fn rules(mutate: impl FnOnce(&mut Song)) -> Vec<&'static str> {
    let mut song = valid_song();
    mutate(&mut song);
    validate(&song, &manifest()).into_iter().map(|v| v.rule).collect()
}

/// The fixture's Surge XT reference, its parameters, and the Cmajor instrument's parameters.
fn plugin(song: &mut Song) -> &mut PluginRef {
    let effect = song.tracks.get_mut(BASS).unwrap().fx_chain.get_mut(SURGE_FX).unwrap();
    let Some(DeviceRef { kind: Some(device_ref::Kind::Plugin(p)) }) = &mut effect.r#ref else {
        unreachable!("the fixture's effect is a plugin")
    };
    p
}

fn surge_params(song: &mut Song) -> &mut std::collections::BTreeMap<String, f64> {
    &mut song.tracks.get_mut(BASS).unwrap().fx_chain.get_mut(SURGE_FX).unwrap().params
}

fn cmajor_params(song: &mut Song) -> &mut std::collections::BTreeMap<String, f64> {
    &mut song.tracks.get_mut(BASS).unwrap().instrument.as_mut().unwrap().params
}

fn assert_fires(rule: &str, mutate: impl FnOnce(&mut Song)) {
    let fired = rules(mutate);
    assert!(fired.contains(&rule), "expected `{rule}`, got {fired:?}");
}

#[test]
fn the_fixture_is_valid() {
    // The strongest assertion here: a realistic song passes every rule. If a rule is wrong,
    // this fails before any of the negative tests do.
    assert_eq!(validate(&valid_song(), &manifest()), vec![]);
}

#[test]
fn every_violation_is_reported_not_just_the_first() {
    // §5: errors must be something a model can act on, and §6 allows three retries. One error
    // per round trip would spend them on a document with four faults.
    let fired = rules(|s| {
        s.schema_version = 0;
        s.render_target = None;
        s.tracks.get_mut(BASS).unwrap().mix = None;
        s.clips.get_mut(CLIP).unwrap().length_ticks = 0;
    });
    assert!(fired.len() >= 4, "expected every fault, got {fired:?}");
}

#[test]
fn output_is_stable_for_a_given_song() {
    let mut song = valid_song();
    song.tracks.get_mut(BASS).unwrap().index = 1; // collides with master
    assert_eq!(validate(&song, &manifest()), validate(&song, &manifest()));
}

#[test]
fn violations_carry_a_json_pointer_to_the_offending_value() {
    let mut song = valid_song();
    song.tracks.get_mut(BASS).unwrap().mix.as_mut().unwrap().pan = 4.0;
    let v = validate(&song, &manifest());
    assert_eq!(v.len(), 1, "{v:?}");
    assert_eq!(v[0].path, format!("/tracks/{BASS}/mix/pan"));
    assert_eq!(v[0].rule, "pan_out_of_range");
}

// ---- identity and keying (ADR 0001 §3, ADR 0002 §2) ----

#[test]
fn a_map_key_must_equal_its_entitys_id() {
    assert_fires("key_id_mismatch", |s| {
        let mut clip = s.clips.remove(CLIP).unwrap();
        clip.id = "01M1FPMP00BADKEY0000000001".to_string();
        s.clips.insert(CLIP.to_string(), clip);
    });
}

#[test]
fn an_id_that_is_not_a_ulid_is_rejected() {
    // Crockford base32, 26 characters. The letters I, L, O and U are not in the alphabet.
    for bad in ["short", "01K4F2QN8B", "01M1FPMP00CPCHRS000000000U"] {
        let fired = rules(|s| {
            let mut clip = s.clips.remove(CLIP).unwrap();
            clip.id = bad.to_string();
            s.clips.insert(bad.to_string(), clip);
        });
        assert!(fired.contains(&"id_not_ulid"), "`{bad}` should be rejected, got {fired:?}");
    }
}

// ---- §4.4 invariants ----

#[test]
fn clips_may_not_overlap_unless_the_track_allows_it() {
    let overlap = |s: &mut Song| {
        let mut second = s.clips[CLIP].clone();
        second.id = "01M1FPMP00SECND00000000099".to_string();
        second.start_tick += 480; // inside the first clip
        s.clips.insert(second.id.clone(), second);
    };
    assert_fires("clip_overlap", overlap);

    let mut song = valid_song();
    overlap(&mut song);
    song.tracks.get_mut(BASS).unwrap().allow_overlap = true;
    assert_eq!(validate(&song, &manifest()), vec![], "allow_overlap permits it (§4.4)");
}

#[test]
fn a_note_may_not_run_past_the_end_of_its_clip() {
    assert_fires("note_outside_clip", |s| {
        let clip = s.clips.get_mut(CLIP).unwrap();
        let Some(clip::Content::NoteClip(n)) = &mut clip.content else { unreachable!() };
        n.notes.get_mut(NOTE_G1).unwrap().length_ticks = clip.length_ticks + 1;
    });
}

#[test]
fn ticks_are_never_negative_and_tempo_is_positive() {
    assert_fires("tick_negative", |s| s.clips.get_mut(CLIP).unwrap().start_tick = -1);
    assert_fires("tempo_not_positive", |s| {
        s.tempo_map.as_mut().unwrap().events.values_mut().for_each(|e| e.bpm = 0.0);
    });
}

#[test]
fn an_audio_clips_fades_are_ticks_and_its_gain_is_a_double_like_any_other() {
    // No rule of its own (ADR 0011 is silent on validation): §4.4's "ticks non-negative" and
    // ADR 0002's "all doubles finite" reach the new fields the way they reach every other.
    fn audio(s: &mut Song) -> &mut AudioClip {
        let Some(clip::Content::AudioClip(a)) = &mut s.clips.get_mut(AUDIO_CLIP).unwrap().content
        else {
            unreachable!()
        };
        a
    }
    assert_fires("tick_negative", |s| audio(s).fade_in_ticks = -1);
    assert_fires("tick_negative", |s| audio(s).fade_out_ticks = -1);
    assert_fires("double_not_finite", |s| audio(s).gain_db = f64::INFINITY);
    assert_fires("negative_zero", |s| audio(s).gain_db = -0.0);
}

#[test]
fn the_tempo_map_needs_an_event_at_the_songs_start() {
    // Without one the song has no tempo at tick 0, and §4.4's derivable render length fails.
    assert_fires("tempo_map_no_origin", |s| {
        s.tempo_map.as_mut().unwrap().events.values_mut().for_each(|e| e.tick = 1920);
    });
    assert_fires("tempo_map_empty", |s| s.tempo_map.as_mut().unwrap().events.clear());
}

#[test]
fn a_clip_needs_a_positive_length_or_the_render_length_is_not_derivable() {
    assert_fires("length_not_positive", |s| s.clips.get_mut(CLIP).unwrap().length_ticks = 0);
}

#[test]
fn a_generator_that_has_been_compiled_records_the_toolchain_it_was_compiled_with() {
    // §4.4, **narrowed by ADR 0027 §1 in M4 PR 5**: the rule is on the pair. A generator the
    // fixture presents as compiled by nothing carries `compiled_hash: ""`, so the pair has to
    // be made first — and that this line is needed at all is the narrowing.
    assert_fires("toolchain_version_empty", |s| {
        s.generators.values_mut().for_each(|g| {
            g.compiled_hash = "a".repeat(64);
            g.toolchain_version.clear();
        });
    });
}

#[test]
fn a_generator_nothing_has_compiled_may_say_nothing_about_a_toolchain() {
    // The other half of the narrowing, and the half `define_generator` depends on: a
    // generator that has just been defined has been compiled by nothing and has no version to
    // state. Before M4 PR 5 this song was refused, so `define_generator` could not have added
    // one without writing a placeholder that lies until the first compile overwrites it —
    // which is the shape ADR 0027 §1 exists to refuse.
    let fired = rules(|s| {
        s.generators.values_mut().for_each(|g| {
            g.compiled_hash.clear();
            g.toolchain_version.clear();
        });
    });
    assert!(!fired.contains(&"toolchain_version_empty"), "{fired:?}");
}

#[test]
fn a_plugin_reference_is_pinned_to_a_version() {
    assert_fires("plugin_version_unpinned", |s| {
        for track in s.tracks.values_mut() {
            for effect in track.fx_chain.values_mut() {
                if let Some(DeviceRef { kind: Some(device_ref::Kind::Plugin(p)) }) = &mut effect.r#ref {
                    p.version.clear();
                }
            }
        }
    });
}

// ---- resolved against the build manifest (ADR 0010 §4) ----
//
// The fixture's *effect* is Surge XT and its *instrument* is a Cmajor source, which is what
// makes these tests worth having in pairs: one device the manifest can answer for and one it
// cannot, in the same song.

#[test]
fn a_plugin_this_build_cannot_host_is_refused() {
    // §4.4's "resolves to a known plugin". `the_fixture_is_valid` is the other half: the id
    // the fixture does carry, `Surge Synth Team/Surge XT`, is one the manifest declares — so
    // this rule is shown both firing and not firing on the same field.
    assert_fires("plugin_unknown", |s| plugin(s).plugin_id = "org.nope.nothing".to_string());
}

#[test]
fn a_parameter_the_plugin_does_not_declare_is_refused() {
    // `cutoff` is the name a person would reach for and is not an identifier: the manifest
    // keys a parameter by the plugin's own `ParamID`, because display names are not unique.
    assert_fires("param_unknown", |s| {
        surge_params(s).insert("cutoff".to_string(), 0.5);
    });
    assert_fires("param_unknown", |s| {
        s.automation.values_mut().for_each(|a| {
            let target = a.target.as_mut().unwrap();
            target.device_id = SURGE_FX.to_string();
            target.param = "cutoff".to_string();
        });
    });
}

#[test]
fn a_real_parameter_id_resolves() {
    // The rule has to stop firing, or it is a rule that refuses everything.
    let mut song = valid_song();
    surge_params(&mut song).insert(CUTOFF.to_string(), 0.5);
    let target = song.automation.values_mut().next().unwrap().target.as_mut().unwrap();
    target.device_id = SURGE_FX.to_string();
    target.param = CUTOFF.to_string();
    assert_eq!(validate(&song, &manifest()), vec![]);
}

#[test]
fn a_plugin_parameter_value_is_normalised() {
    // ADR 0010 §4, extended in PR 7: VST3 shows a host exactly one numeric domain and it is
    // `0..1`. The engine clamps rather than refusing, so nothing downstream would report it.
    assert_fires("param_out_of_range", |s| {
        surge_params(s).insert(CUTOFF.to_string(), 1.5);
    });
    assert_fires("param_out_of_range", |s| {
        let automation = s.automation.values_mut().next().unwrap();
        let target = automation.target.as_mut().unwrap();
        target.device_id = SURGE_FX.to_string();
        target.param = CUTOFF.to_string();
        automation.points.values_mut().for_each(|p| p.value = -0.5);
    });
}

/// Points the fixture's one lane at `track`'s `param`, leaving one point at `value`.
///
/// The lane is the fixture's, so what changes is only where it points and what it carries —
/// which is the whole of what decides the domain (ADR 0015 §1, §2).
fn mix_lane<'a>(track: &'a str, param: &'a str, value: f64) -> impl FnOnce(&mut Song) + 'a {
    move |song: &mut Song| {
        let automation = song.automation.values_mut().next().unwrap();
        let target = automation.target.as_mut().unwrap();
        target.device_id = track.to_string();
        target.param = param.to_string();
        automation.points.values_mut().for_each(|p| p.value = value);
    }
}

#[test]
fn an_automation_target_may_name_a_track_and_then_it_is_gain_db_or_pan() {
    // ADR 0015 §1: ids are globally unique across every collection (§4.3), so a `ParamRef`
    // whose `device_id` is a track id addresses `Mix.gain_db` and `Mix.pan` — with no field
    // added to `song.proto`, which is why no M0.4 golden moved for this.
    let mut song = valid_song();
    mix_lane(BASS, "gain_db", -12.0)(&mut song);
    assert_eq!(validate(&song, &manifest()), vec![]);

    let mut song = valid_song();
    mix_lane(MASTER, "pan", -0.25)(&mut song);
    assert_eq!(validate(&song, &manifest()), vec![]);

    // The two that are refused, and the reason is that `AutomationPoint.value` is a double:
    // automating a boolean needs a threshold rule that would be ours and pinned.
    assert_fires("param_unknown", mix_lane(BASS, "mute", 1.0));
    assert_fires("param_unknown", mix_lane(BASS, "solo", 1.0));
    // A plugin's parameter id is not a track's, however real it is on the plugin.
    assert_fires("param_unknown", mix_lane(BASS, CUTOFF, 0.5));
    // And an id that is neither is still the failure it was.
    assert_fires("device_unknown", mix_lane("01ZZZZZZZZZZZZZZZZZZZZZZZZ", "gain_db", 0.0));
}

#[test]
fn param_out_of_range_judges_the_domain_the_target_resolved_to() {
    // The subtlety of ADR 0015 §2: one rule id, three domains, and `0.5` is legal in each and
    // means a different thing in each — mid-range to a plugin, a quiet fader, a little to the
    // right. A rule that conflated them would pass all three and be wrong about two.
    let mut song = valid_song();
    surge_params(&mut song).insert(CUTOFF.to_string(), 0.5);
    let mut with_plugin_lane = song.clone();
    mix_lane(SURGE_FX, CUTOFF, 0.5)(&mut with_plugin_lane);
    assert_eq!(validate(&with_plugin_lane, &manifest()), vec![]);

    // `gain_db` is decibels and unbounded, so 0.5 is legal and very nearly nothing, and so is
    // -60, and so is +40 — the engine's fader has a ceiling and the model does not.
    for db in [0.5, -60.0, 40.0, 0.0] {
        let mut song = valid_song();
        mix_lane(BASS, "gain_db", db)(&mut song);
        assert_eq!(validate(&song, &manifest()), vec![], "{db} dB is a legal fader ride");
    }

    // `pan` is -1..1, so 0.5 is legal and 1.5 is not — where the same 1.5 on `gain_db` above
    // was fine and the same 1.5 on a plugin parameter is `param_out_of_range` too.
    let mut song = valid_song();
    mix_lane(BASS, "pan", 0.5)(&mut song);
    assert_eq!(validate(&song, &manifest()), vec![]);
    assert_fires("param_out_of_range", mix_lane(BASS, "pan", 1.5));
    assert_fires("param_out_of_range", mix_lane(BASS, "pan", -1.5));

    // And the normalised domain is unchanged where it applies: 1.5 is out of a plugin's 0..1
    // while it is a perfectly ordinary number of decibels.
    let mut song = valid_song();
    surge_params(&mut song).insert(CUTOFF.to_string(), 0.5);
    mix_lane(SURGE_FX, CUTOFF, 1.5)(&mut song);
    let mut rules: Vec<&str> = validate(&song, &manifest()).into_iter().map(|v| v.rule).collect();
    rules.dedup();
    assert_eq!(rules, vec!["param_out_of_range"], "{rules:?}");
}

#[test]
fn a_device_that_is_not_a_plugin_is_not_judged_by_the_manifest() {
    // The fixture's Cmajor instrument carries `drive: 0.62` and its lane targets `cutoff`,
    // and neither is a plugin parameter: a Cmajor device declares its parameters in a source
    // M4 compiles, which this build cannot read. Unchecked for a stated reason, rather than
    // silently — and this is what says so, so that the day M4 arrives the test fails.
    let mut song = valid_song();
    cmajor_params(&mut song).insert("anything at all".to_string(), 7.5);
    assert_eq!(validate(&song, &manifest()), vec![]);
}

// ---- rules the schema shape implies (ADR 0002 Consequences) ----

#[test]
fn a_song_has_exactly_one_master_track() {
    assert_fires("master_track_count", |s| {
        s.tracks.get_mut(MASTER).unwrap().kind = TrackKind::Bus as i32;
    });
}

#[test]
fn track_and_effect_indexes_are_unique() {
    assert_fires("track_index_duplicate", |s| s.tracks.get_mut(BASS).unwrap().index = 1);
    assert_fires("effect_index_duplicate", |s| {
        let chain = &mut s.tracks.get_mut(BASS).unwrap().fx_chain;
        let mut second = chain.values().next().unwrap().clone();
        second.id = "01M1FPMP00FX2ND00000000098".to_string();
        chain.insert(second.id.clone(), second); // same index 0
    });
}

#[test]
fn an_instrument_belongs_to_an_instrument_track_and_only_there() {
    assert_fires("instrument_presence", |s| s.tracks.get_mut(BASS).unwrap().instrument = None);
    assert_fires("instrument_presence", |s| {
        let instrument = s.tracks[BASS].instrument.clone();
        s.tracks.get_mut(MASTER).unwrap().instrument = instrument;
    });
}

#[test]
fn routing_and_sends_target_a_bus_or_the_master() {
    assert_fires("routing_target_invalid", |s| {
        s.tracks.get_mut(BASS).unwrap().routing.as_mut().unwrap().output_track_id =
            Some(BASS.to_string()); // an instrument track, and itself
    });
    assert_fires("routing_target_invalid", |s| {
        s.tracks.get_mut(BASS).unwrap().routing.as_mut().unwrap().sends
            .insert(BASS.to_string(), -6.0);
    });
}

#[test]
fn automation_targets_a_device_that_exists() {
    assert_fires("device_unknown", |s| {
        s.automation.values_mut().for_each(|a| {
            a.target.as_mut().unwrap().device_id = "01M1FPMP00MSSNG00000000001".to_string();
        });
    });
}

#[test]
fn a_generator_targets_something_in_this_song() {
    assert_fires("clip_unknown", |s| {
        s.generators.values_mut().for_each(|g| {
            g.target = Some(generator::Target::ClipId("01M1FPMP00MSSNG00000000001".to_string()));
        });
    });
}

#[test]
fn an_enum_left_unspecified_is_rejected() {
    assert_fires("enum_unspecified", |s| s.tracks.get_mut(BASS).unwrap().kind = 0);
    assert_fires("enum_unspecified", |s| {
        s.automation.values_mut().for_each(|a| {
            a.points.values_mut().for_each(|p| p.curve = 0);
        });
    });
}

#[test]
fn a_required_message_may_not_be_absent() {
    assert_fires("message_missing", |s| s.tracks.get_mut(BASS).unwrap().routing = None);
    assert_fires("message_missing", |s| s.render_target = None);
    assert_fires("message_missing", |s| s.clips.get_mut(CLIP).unwrap().provenance = None);
}

// ---- rules moved here from the writer (ADR 0002 §4, amended) ----

#[test]
fn negative_zero_is_rejected_in_a_stored_song() {
    // The writer serialises faithfully; catching -0.0 is this module's job, because two equal
    // songs must not produce different bytes.
    assert_fires("negative_zero", |s| {
        s.tracks.get_mut(BASS).unwrap().mix.as_mut().unwrap().pan = -0.0;
    });
}

#[test]
fn non_finite_doubles_are_rejected_before_they_reach_the_writer() {
    assert_fires("double_not_finite", |s| {
        s.tracks.get_mut(BASS).unwrap().mix.as_mut().unwrap().gain_db = f64::NAN;
    });
}

#[test]
fn timestamps_finer_than_a_millisecond_are_rejected() {
    // The clock yields milliseconds (ADR 0002 §4), so finer precision means the value did not
    // come from the clock.
    assert_fires("timestamp_precision", |s| {
        s.provenance.as_mut().unwrap().created_at.as_mut().unwrap().nanos = 1;
    });
}
