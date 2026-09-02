//! Validator tests (docs/specs.md §4.4, ADR 0002 Consequences).
//!
//! Each rule is exercised by breaking a valid song in exactly one way and asserting its id
//! appears. The shared starting point is the fixture, so a rule that fires on a *correct*
//! song shows up immediately in `the_fixture_is_valid`.

use escribass_core::{from_canonical_json, validate};
use escribass_schema::song::*;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");
const BASS: &str = "01M1FPMP00TRACKBASS0000002";
const MASTER: &str = "01M1FPMP00TRACKMASTER00003";
const CLIP: &str = "01M1FPMP00CPCHRS0000000006";
const NOTE_G1: &str = "01M1FPMP00NTEG100000000007";

fn valid_song() -> Song {
    from_canonical_json(&std::fs::read_to_string(FIXTURE).expect("fixture")).expect("parses")
}

/// Breaks a valid song in one way and returns the rule ids that fire.
fn rules(mutate: impl FnOnce(&mut Song)) -> Vec<&'static str> {
    let mut song = valid_song();
    mutate(&mut song);
    validate(&song).into_iter().map(|v| v.rule).collect()
}

fn assert_fires(rule: &str, mutate: impl FnOnce(&mut Song)) {
    let fired = rules(mutate);
    assert!(fired.contains(&rule), "expected `{rule}`, got {fired:?}");
}

#[test]
fn the_fixture_is_valid() {
    // The strongest assertion here: a realistic song passes every rule. If a rule is wrong,
    // this fails before any of the negative tests do.
    assert_eq!(validate(&valid_song()), vec![]);
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
    assert_eq!(validate(&song), validate(&song));
}

#[test]
fn violations_carry_a_json_pointer_to_the_offending_value() {
    let mut song = valid_song();
    song.tracks.get_mut(BASS).unwrap().mix.as_mut().unwrap().pan = 4.0;
    let v = validate(&song);
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
    assert_eq!(validate(&song), vec![], "allow_overlap permits it (§4.4)");
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
fn a_generator_records_the_toolchain_it_was_compiled_with() {
    assert_fires("toolchain_version_empty", |s| {
        s.generators.values_mut().for_each(|g| g.toolchain_version.clear());
    });
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
