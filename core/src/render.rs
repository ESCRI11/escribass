//! Compiling a song into the plan the engine renders (ADR 0007).
//!
//! The engine never receives a `Song`. It receives a `RenderPlan` — flat, ordered, already
//! resolved — and every question that needs the model's structure to answer is answered here,
//! once: which track a clip is on, what order a chain is in, what a loop unrolls to, which
//! tracks `solo` and `mute` silence, which device an automation lane targets, and how long
//! the render is (ADR 0007 §1). The engine renders exactly `length_ticks` ticks with no
//! opinion of its own.
//!
//! `compile` is **pure**: it reads no file, takes no clock and uses no randomness. The plan is
//! a function of the document and the asset index, which is what lets the determinism suite
//! golden it — a render mismatch then splits into "core changed the plan" and "the engine
//! changed the rendering" by comparing the plan first (ADR 0007 §4).
//!
//! **Every order in the plan comes from a stated rule, never from iteration.** The model's
//! collections are maps keyed by id; a `BTreeMap` iterates in id order and every sort below is
//! stable, so where a rule leaves a tie, the tie breaks by id. That is deterministic and it is
//! written down — which is the whole difference from an order that falls out of a hash table.
//!
//! The song has passed §4.4 — the session holds no other kind (ADR 0007 §4) — so nothing here
//! re-checks what the validator refuses. Where a shape the validator forbids would make this
//! code loop or panic, it is guarded and named rather than assumed away.

use crate::validate::Violation;
use escribass_proto::render::{
    plan_clip, PlanAudio, PlanClip, PlanEffect, PlanInstrument, PlanLane, PlanNotes, PlanTrack,
    RenderPlan,
};
use escribass_schema::song::{
    clip, device_ref, AutomationPoint, Clip, DeviceRef, Effect, Instrument, Mix, Note,
    RenderKind, Song, TempoEvent, Track, TrackKind,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The rule every M1 capability refusal carries (ADR 0007 §6): the document is valid, and
/// this engine cannot turn it into sound. The validator never uses it — validity and
/// renderability are different questions.
pub const UNSUPPORTED: &str = "render_unsupported";

/// Compiles a valid song into the plan the engine renders.
///
/// `assets` is the index from content hash to absolute path, which is what `Project` knows
/// about `assets/`; nothing here opens one. The plan's `output_path` is left empty: where the
/// WAV goes is `render_export`'s argument, not the document's, and the session fills it in.
///
/// Fails with **every** reason, never the first, and with `Vec<Violation>` rather than an
/// error type: an unsupported device, a missing asset or a routing M1 cannot honour are all
/// things the caller can fix by calling differently, which is ADR 0006 §2's line drawn by
/// return type, as `prepare` draws it. Violations are sorted, so the refusal is byte-stable.
pub fn compile(
    song: &Song,
    assets: &BTreeMap<String, PathBuf>,
) -> Result<RenderPlan, Vec<Violation>> {
    let mut compiler = Compiler { assets, refused: Vec::new() };
    let plan = compiler.plan(song);
    if compiler.refused.is_empty() {
        Ok(plan)
    } else {
        compiler.refused.sort();
        Err(compiler.refused)
    }
}

struct Compiler<'a> {
    assets: &'a BTreeMap<String, PathBuf>,
    refused: Vec<Violation>,
}

/// Automation points by device id, then by parameter, before they are nested under their
/// device.
type Lanes<'a> = BTreeMap<&'a str, BTreeMap<&'a str, Vec<AutomationPoint>>>;

impl Compiler<'_> {
    fn refuse(&mut self, path: impl Into<String>, rule: &'static str, message: impl Into<String>) {
        self.refused.push(Violation { path: path.into(), rule, message: message.into() });
    }

    fn plan(&mut self, song: &Song) -> RenderPlan {
        self.check_target(song);
        let length_ticks = self.length(song);

        let masters: Vec<&Track> =
            song.tracks.values().filter(|t| t.kind == TrackKind::Master as i32).collect();
        let master = match masters[..] {
            [master] => Some(master),
            _ => {
                self.refuse(
                    "/tracks",
                    "master_track_count",
                    format!("a song has exactly one master track, found {}", masters.len()),
                );
                None
            }
        };

        // `solo` on any track silences every track that is not soloed; `mute` silences its own
        // whatever `solo` says, which is what a mixer's mute button means. Master is outside
        // the solo rule — it has nothing to be soloed against, and no mixer offers it — and its
        // mute is the render's: a muted master is silence of the right length.
        let any_solo = song.tracks.values().any(|t| {
            t.kind != TrackKind::Master as i32 && t.mix.as_ref().is_some_and(|m| m.solo)
        });
        let master_muted = master.and_then(|m| m.mix.as_ref()).is_some_and(|m| m.mute);
        let sounds = |track: &Track| {
            let mix = track.mix.clone().unwrap_or_default();
            !master_muted && !mix.mute && (mix.solo || !any_solo)
        };

        let mut lanes = lanes(song);
        let mut clips = clips_by_track(song);

        // Mixer order is `Track.index`. The validator forbids a tie; a stable sort would break
        // one by id.
        let mut ordered: Vec<&Track> = song.tracks.values().collect();
        ordered.sort_by_key(|t| t.index);

        let mut tracks = Vec::new();
        for track in ordered {
            // A track that does not sound is not rendered, so nothing on it is something M1
            // cannot render: muting the Cmajor track is how a project exports the rest of
            // itself before M4. Its clips, devices and lanes are dropped with it.
            if track.kind == TrackKind::Master as i32 || !sounds(track) {
                continue;
            }
            let at = format!("/tracks/{}", track.id);
            if track.kind == TrackKind::Bus as i32 {
                self.refuse(
                    format!("{at}/kind"),
                    UNSUPPORTED,
                    "a bus is the mixer's, which is M2's; in M1 every track outputs to master",
                );
                continue;
            }
            self.check_routing(&at, track, master);
            let own = clips.remove(track.id.as_str()).unwrap_or_default();
            tracks.push(self.track(&at, track, own, &mut lanes));
        }

        let master = master.map(|m| {
            let at = format!("/tracks/{}", m.id);
            self.check_routing(&at, m, Some(m));
            let own = if master_muted {
                Vec::new()
            } else {
                clips.remove(m.id.as_str()).unwrap_or_default()
            };
            self.track(&at, m, own, &mut lanes)
        });

        // Tick order; ties by id. `set_tempo` upserts by tick, so a valid song has no tie.
        let mut tempo: Vec<&TempoEvent> =
            song.tempo_map.iter().flat_map(|map| map.events.values()).collect();
        tempo.sort_by_key(|e| e.tick);

        RenderPlan {
            tracks,
            master,
            tempo: tempo.into_iter().map(|e| TempoEvent { id: String::new(), ..e.clone() }).collect(),
            target: song.render_target.clone(),
            length_ticks,
            output_path: String::new(),
        }
    }

    /// Where the render ends: the last clip's end or the last section's, whichever is later
    /// (docs/plan.md, decisions of 2026-09-04). A section that outlasts every clip is a
    /// deliberate stretch of silence — an outro someone named — and cutting it at the last
    /// note would make the arrangement shorter than its author wrote it.
    ///
    /// Over every clip, sounding or not: the length is a property of the song, so soloing two
    /// bars of an eight-bar song exports eight bars with six of silence, as a DAW's export
    /// range would.
    fn length(&mut self, song: &Song) -> i32 {
        let mut end: i64 = 0;
        for clip in song.clips.values() {
            // Widened so the sum cannot overflow; the validator bounds neither field's sum.
            let clip_end = clip.start_tick as i64 + clip.length_ticks as i64;
            if clip_end > i32::MAX as i64 {
                self.refuse(
                    format!("/clips/{}/length_ticks", clip.id),
                    UNSUPPORTED,
                    format!("the clip ends at tick {clip_end}, past the {} a plan can hold", i32::MAX),
                );
            }
            end = end.max(clip_end);
        }
        for section in song.sections.values() {
            end = end.max(section.end_tick as i64);
        }
        end.min(i32::MAX as i64) as i32
    }

    fn check_target(&mut self, song: &Song) {
        let Some(target) = &song.render_target else { return };
        if target.kind == RenderKind::Stems as i32 {
            self.refuse("/render_target/kind", UNSUPPORTED, "stems are M5's; M1 renders the master");
        }
        // Dither is a noise source, and §2.2 requires every source of randomness to carry a
        // seed stored in the project — which a bool cannot (ADR 0007 §6).
        if target.dither {
            self.refuse(
                "/render_target/dither",
                UNSUPPORTED,
                "dither is a noise source and a bool carries no seed for it (§2.2)",
            );
        }
    }

    /// Every plan track outputs to master (ADR 0007 §6). Naming master explicitly means what
    /// absence means; naming anything else is the mixer's, which is M2's.
    fn check_routing(&mut self, at: &str, track: &Track, master: Option<&Track>) {
        let Some(routing) = &track.routing else { return };
        if let Some(output) = &routing.output_track_id {
            if master.is_none_or(|m| m.id != *output) {
                self.refuse(
                    format!("{at}/routing/output_track_id"),
                    UNSUPPORTED,
                    format!("`{output}` is a bus; in M1 every track outputs to master"),
                );
            }
        }
        if !routing.sends.is_empty() {
            self.refuse(
                format!("{at}/routing/sends"),
                UNSUPPORTED,
                "sends are the mixer's, which is M2's",
            );
        }
        if !routing.sidechains.is_empty() {
            self.refuse(
                format!("{at}/routing/sidechains"),
                UNSUPPORTED,
                "sidechains are the mixer's, which is M2's",
            );
        }
    }

    /// The device kinds M4 compiles and hosts (ADR 0007 §6). A plugin outside the bundled set
    /// never reaches here: that is the validator's `plugin_unknown` (ADR 0010 §4).
    fn check_device(&mut self, at: &str, device: Option<&DeviceRef>) {
        let arm = match device.and_then(|d| d.kind.as_ref()) {
            Some(device_ref::Kind::Cmajor(_)) => "cmajor",
            Some(device_ref::Kind::Faust(_)) => "faust",
            Some(device_ref::Kind::Neural(_)) => "neural",
            _ => return,
        };
        self.refuse(
            format!("{at}/ref/{arm}"),
            UNSUPPORTED,
            format!(
                "`{arm}` devices are compiled and hosted from M4; M1 hosts the bundled plugins \
                 and the sampler"
            ),
        );
    }

    fn track(&mut self, at: &str, track: &Track, clips: Vec<&Clip>, lanes: &mut Lanes) -> PlanTrack {
        let instrument = track.instrument.as_ref().map(|held| {
            self.check_device(&format!("{at}/instrument"), held.r#ref.as_ref());
            PlanInstrument {
                // The §4.3 fields cross empty (ADR 0007 §2).
                instrument: Some(Instrument {
                    id: String::new(),
                    provenance: None,
                    version: 0,
                    ..held.clone()
                }),
                lanes: take_lanes(lanes, &held.id),
            }
        });

        // Chain order is `Effect.index` (ADR 0007 §1). The validator forbids a tie.
        let mut chain: Vec<&Effect> = track.fx_chain.values().collect();
        chain.sort_by_key(|e| e.index);
        let effects = chain
            .into_iter()
            .map(|held| {
                self.check_device(&format!("{at}/fx_chain/{}", held.id), held.r#ref.as_ref());
                PlanEffect {
                    effect: Some(Effect {
                        id: String::new(),
                        provenance: None,
                        version: 0,
                        ..held.clone()
                    }),
                    lanes: take_lanes(lanes, &held.id),
                }
            })
            .collect();

        // `mute` and `solo` were resolved in `plan` and cross false: a `solo` the engine could
        // see is one it could act on, and that is a second implementation of what it means
        // (ADR 0007 §1). They are consumed, not carried (ADR 0007 §5).
        let mix = track.mix.clone().map(|m| Mix { mute: false, solo: false, ..m });

        let clips = clips.into_iter().flat_map(|c| self.clip(c)).collect();
        PlanTrack { instrument, effects, mix, clips }
    }

    /// One plan clip per loop iteration. A looping clip repeats its first `loop_length_ticks`
    /// of content to fill `length_ticks` (song.proto), so it arrives as that many clips, the
    /// last cut short where the clip ends; a clip that does not loop is one iteration of its
    /// own length. The same shape serves notes and audio, because an audio iteration is the
    /// asset played again from its start — a clip of its own and nothing more — and
    /// `Note.start_tick` stays clip-relative either way (ADR 0007 §3).
    fn clip(&mut self, held: &Clip) -> Vec<PlanClip> {
        let at = format!("/clips/{}", held.id);
        let unit = held.loop_length_ticks.unwrap_or(held.length_ticks);
        // The validator refuses both; guarded because the loop below would otherwise not end.
        if unit <= 0 || held.length_ticks <= 0 {
            return Vec::new();
        }

        let mut iterations = Vec::new();
        let mut offset = 0;
        while offset < held.length_ticks {
            iterations.push((offset, unit.min(held.length_ticks - offset)));
            offset = offset.saturating_add(unit);
        }
        // `length` refused a clip whose end does not fit an i32; saturating so the refused
        // plan is discarded rather than panicked over on the way out.
        let placed = |offset: i32, length: i32, content| PlanClip {
            start_tick: held.start_tick.saturating_add(offset),
            length_ticks: length,
            content: Some(content),
        };

        match &held.content {
            // The validator's `oneof_unset`.
            None => Vec::new(),

            Some(clip::Content::NoteClip(notes)) => {
                // Start order; ties by id.
                let mut ordered: Vec<&Note> = notes.notes.values().collect();
                ordered.sort_by_key(|n| n.start_tick);
                iterations
                    .into_iter()
                    .map(|(offset, length)| {
                        // The first `unit` ticks of content, cut at the iteration's end: a plan
                        // clip's notes lie inside it, as §4.4 requires of the model's. A note
                        // starting at or past the loop length is not loop content and never
                        // sounds, which is what "repeat the first loop_length_ticks" says.
                        let inside = ordered.iter().filter(|n| n.start_tick < length).map(|n| Note {
                            id: String::new(),
                            provenance: None,
                            version: 0,
                            length_ticks: n.length_ticks.min(length.saturating_sub(n.start_tick)),
                            ..(*n).clone()
                        });
                        let notes = PlanNotes { notes: inside.collect() };
                        placed(offset, length, plan_clip::Content::Notes(notes))
                    })
                    .collect()
            }

            Some(clip::Content::AudioClip(audio)) => {
                let Some(path) = self.assets.get(&audio.asset_hash) else {
                    self.refuse(
                        format!("{at}/audio_clip/asset_hash"),
                        "asset_missing",
                        format!(
                            "`{}` is not in assets/; add_asset is what puts it there",
                            audio.asset_hash
                        ),
                    );
                    return Vec::new();
                };
                // A stretched loop stretches the asset to `loop_length_ticks` and repeats that
                // unit (ADR 0011 §3). Each iteration crosses as its own clip, and the engine
                // stretches to the clip it is handed — so a short last iteration would stretch
                // to the wrong length. Refused rather than rendered wrong; the additive field
                // that would express it is the `ponytail:` on `PlanAudio` in render.proto.
                let stretched_loop = audio.time_stretch && held.loop_length_ticks.is_some();
                if stretched_loop && held.length_ticks % unit != 0 {
                    self.refuse(
                        format!("{at}/loop_length_ticks"),
                        UNSUPPORTED,
                        format!(
                            "a stretched loop fills a whole number of loops in M1: {} ticks is not \
                             a multiple of {unit}, and the last iteration would stretch to the \
                             wrong length (ADR 0011 §3)",
                            held.length_ticks
                        ),
                    );
                    return Vec::new();
                }
                let path = path.to_string_lossy().into_owned();
                iterations
                    .into_iter()
                    .map(|(offset, length)| {
                        let audio = PlanAudio { clip: Some(audio.clone()), path: path.clone() };
                        placed(offset, length, plan_clip::Content::Audio(audio))
                    })
                    .collect()
            }
        }
    }
}

/// Every automation point in the song, by the device and parameter it targets.
///
/// One lane per parameter: two `Automation` entities naming the same parameter are one curve
/// as far as a sample is concerned, so their points merge — tick order, ties by automation id
/// and then point id, which is the map order a stable sort keeps. A lane whose device sits on
/// a track that does not sound is never taken and is dropped with the track: it could not
/// change a sample. A `device_id` naming nothing is the validator's `device_unknown` and does
/// not reach compile; if it did, it would be left here the same way.
fn lanes(song: &Song) -> Lanes<'_> {
    let mut lanes: Lanes = BTreeMap::new();
    for automation in song.automation.values() {
        let Some(target) = &automation.target else { continue };
        let lane = lanes
            .entry(target.device_id.as_str())
            .or_default()
            .entry(target.param.as_str())
            .or_default();
        // Keyed for patch paths only (ADR 0002 §2); the key crosses empty (ADR 0007 §2).
        for point in automation.points.values() {
            lane.push(AutomationPoint { id: String::new(), ..point.clone() });
        }
    }
    for lane in lanes.values_mut().flat_map(|by_param| by_param.values_mut()) {
        lane.sort_by_key(|p| p.tick);
    }
    lanes
}

/// The lanes of one device, in parameter-name order.
fn take_lanes(lanes: &mut Lanes, device: &str) -> Vec<PlanLane> {
    lanes
        .remove(device)
        .unwrap_or_default()
        .into_iter()
        .map(|(param, points)| PlanLane { param: param.to_string(), points })
        .collect()
}

/// Every clip on its track, in timeline order: start tick, ties by id. Without
/// `allow_overlap` the validator forbids a tie; with it, id order is the rule.
fn clips_by_track(song: &Song) -> BTreeMap<&str, Vec<&Clip>> {
    let mut by_track: BTreeMap<&str, Vec<&Clip>> = BTreeMap::new();
    for clip in song.clips.values() {
        by_track.entry(clip.track_id.as_str()).or_default().push(clip);
    }
    for clips in by_track.values_mut() {
        clips.sort_by_key(|c| c.start_tick);
    }
    by_track
}
