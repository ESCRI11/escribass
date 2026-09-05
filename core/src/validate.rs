//! Song validation (docs/specs.md §4.4, ADR 0002 Consequences).
//!
//! Every tool call is validated before it is applied (§5), so this runs on the *result* of a
//! proposed mutation, not on the patch. It returns **every** violation rather than the first:
//! §5 requires errors an LLM can act on, and a model that gets one error per round trip
//! spends its three retries (§6) on a document with four problems.
//!
//! Two limits are structural until later milestones, and are marked in the code where they
//! bite: resolving a `DeviceRef` to a *pinned* plugin needs `lock.json`, which the project
//! store writes (M0.2, next); resolving a `ParamRef` to a *real* parameter of a plugin needs
//! the plugin's manifest, which arrives with the engine (M1). Until then both are checked as
//! far as the model allows — the reference is well-formed and names something in this song.

use escribass_schema::song::*;
use std::collections::{BTreeMap, BTreeSet};

/// One broken rule.
///
/// `rule` is a stable identifier: tests and the AI orchestrator match on it, so it is part of
/// the API and does not change with the wording of `message`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    /// RFC 6901 JSON Pointer to the offending value.
    pub path: String,
    /// Stable machine-readable rule id, e.g. `"clip_overlap"`.
    pub rule: &'static str,
    /// What is wrong, in terms the author — human or model — can act on.
    pub message: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]: {}", self.path, self.rule, self.message)
    }
}

/// Validates a song against §4.4 and the rules the schema shape implies.
///
/// An empty result means valid. Violations are sorted, so the output is stable for a given
/// song and can be compared byte for byte in tests (§11).
pub fn validate(song: &Song) -> Vec<Violation> {
    let mut v = Violations::default();

    v.check_root(song);
    v.check_time_maps(song);
    v.check_sections(song);
    v.check_tracks(song);
    v.check_clips(song);
    v.check_automation(song);
    v.check_generators(song);

    v.0.sort();
    v.0
}

#[derive(Default)]
struct Violations(Vec<Violation>);

impl Violations {
    fn add(&mut self, path: impl Into<String>, rule: &'static str, message: impl Into<String>) {
        self.0.push(Violation { path: path.into(), rule, message: message.into() });
    }

    /// `id` is a Crockford base32 ULID: 26 uppercase characters, excluding I, L, O and U.
    fn check_id(&mut self, path: &str, id: &str) {
        const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        let ok = id.len() == 26 && id.chars().all(|c| ALPHABET.contains(c));
        if !ok {
            self.add(
                format!("{path}/id"),
                "id_not_ulid",
                format!("`{id}` is not a ULID: 26 uppercase Crockford base32 characters"),
            );
        }
    }

    /// Every map is keyed by its value's own id (ADR 0001 §3), so a patch path stays valid.
    fn check_keyed<T>(&mut self, path: &str, map: &BTreeMap<String, T>, id_of: impl Fn(&T) -> &str) {
        for (key, value) in map {
            let item = format!("{path}/{key}");
            self.check_id(&item, key);
            if key != id_of(value) {
                self.add(
                    item,
                    "key_id_mismatch",
                    format!("map key `{key}` must equal the entity's id `{}`", id_of(value)),
                );
            }
        }
    }

    fn check_finite(&mut self, path: &str, value: f64) {
        if !value.is_finite() {
            self.add(path, "double_not_finite", "must be a finite number");
        } else if value == 0.0 && value.is_sign_negative() {
            self.add(
                path,
                "negative_zero",
                "negative zero must be normalised to 0.0 before it is stored (ADR 0002 §4)",
            );
        }
    }

    fn check_provenance(&mut self, path: &str, provenance: Option<&Provenance>) {
        let Some(p) = provenance else {
            return self.add(path, "message_missing", "`provenance` is required on every entity");
        };
        if p.author == Author::Unspecified as i32 {
            self.add(format!("{path}/author"), "enum_unspecified", "author must be set");
        }
        // The clock yields milliseconds (ADR 0002 §4), so finer precision means the value
        // came from somewhere that is not the clock.
        if let Some(t) = &p.created_at {
            if t.nanos % 1_000_000 != 0 {
                self.add(
                    format!("{path}/created_at"),
                    "timestamp_precision",
                    "timestamps are stored at millisecond precision",
                );
            }
        }
    }

    fn check_root(&mut self, song: &Song) {
        self.check_id("", &song.id);
        self.check_provenance("/provenance", song.provenance.as_ref());
        if song.schema_version == 0 {
            self.add("/schema_version", "schema_version_unset", "schema_version must be set");
        }
        if song.render_target.is_none() {
            self.add("/render_target", "message_missing", "`render_target` is required");
        }
        if let Some(rt) = &song.render_target {
            if rt.kind == RenderKind::Unspecified as i32 {
                self.add("/render_target/kind", "enum_unspecified", "render kind must be set");
            }
        }
    }

    fn check_time_maps(&mut self, song: &Song) {
        let Some(tempo) = &song.tempo_map else {
            return self.add("/tempo_map", "message_missing", "`tempo_map` is required");
        };
        if tempo.events.is_empty() {
            self.add("/tempo_map/events", "tempo_map_empty", "a song needs at least one tempo");
        }
        if !tempo.events.values().any(|e| e.tick == 0) {
            self.add(
                "/tempo_map/events",
                "tempo_map_no_origin",
                "the tempo map needs an event at tick 0, or the song has no tempo at its start",
            );
        }
        self.check_keyed("/tempo_map/events", &tempo.events, |e| &e.id);
        for (key, e) in &tempo.events {
            let at = format!("/tempo_map/events/{key}");
            self.check_tick(&at, e.tick);
            self.check_finite(&format!("{at}/bpm"), e.bpm);
            if e.bpm <= 0.0 {
                self.add(format!("{at}/bpm"), "tempo_not_positive", "tempo must be greater than 0");
            }
        }

        let Some(sig) = &song.time_signature_map else {
            return self.add("/time_signature_map", "message_missing", "required");
        };
        self.check_keyed("/time_signature_map/events", &sig.events, |e| &e.id);
        for (key, e) in &sig.events {
            let at = format!("/time_signature_map/events/{key}");
            self.check_tick(&at, e.tick);
            if e.numerator == 0 || e.denominator == 0 {
                self.add(at, "time_signature_invalid", "numerator and denominator must be non-zero");
            }
        }
    }

    fn check_tick(&mut self, path: &str, tick: i32) {
        if tick < 0 {
            self.add(format!("{path}/tick"), "tick_negative", "ticks are never negative");
        }
    }

    fn check_sections(&mut self, song: &Song) {
        self.check_keyed("/sections", &song.sections, |s| &s.id);
        for (key, section) in &song.sections {
            let at = format!("/sections/{key}");
            self.check_provenance(&format!("{at}/provenance"), section.provenance.as_ref());
            if section.start_tick < 0 {
                self.add(format!("{at}/start_tick"), "tick_negative", "ticks are never negative");
            }
            if section.end_tick <= section.start_tick {
                self.add(
                    format!("{at}/end_tick"),
                    "section_not_positive",
                    "a section ends after it starts",
                );
            }
            if section.name.is_empty() {
                self.add(format!("{at}/name"), "name_empty", "a section is named");
            }
        }

        self.check_keyed("/markers", &song.markers, |m| &m.id);
        for (key, marker) in &song.markers {
            let at = format!("/markers/{key}");
            self.check_provenance(&format!("{at}/provenance"), marker.provenance.as_ref());
            self.check_tick(&at, marker.tick);
        }
    }

    fn check_tracks(&mut self, song: &Song) {
        self.check_keyed("/tracks", &song.tracks, |t| &t.id);

        let masters: Vec<_> = song
            .tracks
            .values()
            .filter(|t| t.kind == TrackKind::Master as i32)
            .map(|t| t.id.clone())
            .collect();
        if masters.len() != 1 {
            self.add(
                "/tracks",
                "master_track_count",
                format!("a song has exactly one master track, found {}", masters.len()),
            );
        }

        let mut indexes: BTreeMap<u32, usize> = BTreeMap::new();
        for track in song.tracks.values() {
            *indexes.entry(track.index).or_default() += 1;
        }

        for (key, track) in &song.tracks {
            let at = format!("/tracks/{key}");
            self.check_provenance(&format!("{at}/provenance"), track.provenance.as_ref());

            if track.kind == TrackKind::Unspecified as i32 {
                self.add(format!("{at}/kind"), "enum_unspecified", "track kind must be set");
            }
            if indexes[&track.index] > 1 {
                self.add(
                    format!("{at}/index"),
                    "track_index_duplicate",
                    format!("index {} is used by more than one track", track.index),
                );
            }

            let is_instrument = track.kind == TrackKind::Instrument as i32;
            match (&track.instrument, is_instrument) {
                (None, true) => self.add(
                    format!("{at}/instrument"),
                    "instrument_presence",
                    "an instrument track carries an instrument",
                ),
                (Some(_), false) => self.add(
                    format!("{at}/instrument"),
                    "instrument_presence",
                    "only an instrument track carries an instrument",
                ),
                _ => {}
            }
            if let Some(instrument) = &track.instrument {
                self.check_device(&format!("{at}/instrument"), &instrument.id,
                    instrument.provenance.as_ref(), instrument.r#ref.as_ref(), &instrument.params);
            }

            if track.mix.is_none() {
                self.add(format!("{at}/mix"), "message_missing", "`mix` is required");
            }
            if let Some(mix) = &track.mix {
                self.check_finite(&format!("{at}/mix/gain_db"), mix.gain_db);
                self.check_finite(&format!("{at}/mix/pan"), mix.pan);
                if mix.pan.is_finite() && !(-1.0..=1.0).contains(&mix.pan) {
                    self.add(format!("{at}/mix/pan"), "pan_out_of_range", "pan is -1.0 to 1.0");
                }
            }

            let mut effect_indexes: BTreeMap<u32, usize> = BTreeMap::new();
            for effect in track.fx_chain.values() {
                *effect_indexes.entry(effect.index).or_default() += 1;
            }
            self.check_keyed(&format!("{at}/fx_chain"), &track.fx_chain, |e| &e.id);
            for (ekey, effect) in &track.fx_chain {
                let eat = format!("{at}/fx_chain/{ekey}");
                self.check_device(&eat, &effect.id, effect.provenance.as_ref(),
                    effect.r#ref.as_ref(), &effect.params);
                if effect_indexes[&effect.index] > 1 {
                    self.add(
                        format!("{eat}/index"),
                        "effect_index_duplicate",
                        format!("index {} is used twice in this chain", effect.index),
                    );
                }
            }

            if track.routing.is_none() {
                self.add(format!("{at}/routing"), "message_missing", "`routing` is required");
            }
            if let Some(routing) = &track.routing {
                self.check_routing(&at, track, routing, song);
            }
        }
    }

    fn check_device(&mut self, path: &str, id: &str, provenance: Option<&Provenance>,
                    device: Option<&DeviceRef>, params: &BTreeMap<String, f64>) {
        self.check_id(path, id);
        self.check_provenance(&format!("{path}/provenance"), provenance);
        match device {
            // Resolving the reference to a *pinned* entry needs lock.json, which the project
            // store writes. Until then: the reference exists and names something.
            None => self.add(format!("{path}/ref"), "device_ref_unset", "a device needs a ref"),
            Some(d) => match &d.kind {
                None => self.add(format!("{path}/ref"), "oneof_unset", "ref kind must be set"),
                Some(kind) => {
                    let (field, value) = match kind {
                        device_ref::Kind::Plugin(p) => ("plugin", p.plugin_id.as_str()),
                        device_ref::Kind::Cmajor(s) => ("cmajor", s.source_hash.as_str()),
                        device_ref::Kind::Faust(s) => ("faust", s.source_hash.as_str()),
                        device_ref::Kind::Neural(m) => ("neural", m.model_hash.as_str()),
                        device_ref::Kind::Sampler(s) => ("sampler", s.sfz_hash.as_str()),
                    };
                    if value.is_empty() {
                        self.add(
                            format!("{path}/ref/{field}"),
                            "device_ref_empty",
                            "a device reference needs a plugin id or an asset hash",
                        );
                    }
                    if let device_ref::Kind::Plugin(p) = kind {
                        if p.version.is_empty() {
                            self.add(
                                format!("{path}/ref/plugin/version"),
                                "plugin_version_unpinned",
                                "a plugin reference is pinned to a version (§4.4)",
                            );
                        }
                    }
                }
            },
        }
        for (name, value) in params {
            self.check_finite(&format!("{path}/params/{name}"), *value);
        }
    }

    fn check_routing(&mut self, at: &str, track: &Track, routing: &Routing, song: &Song) {
        let is_bus_or_master = |id: &String| {
            song.tracks.get(id).is_some_and(|t| {
                t.kind == TrackKind::Bus as i32 || t.kind == TrackKind::Master as i32
            })
        };
        if let Some(output) = &routing.output_track_id {
            if !is_bus_or_master(output) {
                self.add(
                    format!("{at}/routing/output_track_id"),
                    "routing_target_invalid",
                    format!("`{output}` is not a bus or master track in this song"),
                );
            }
            if output == &track.id {
                self.add(
                    format!("{at}/routing/output_track_id"),
                    "routing_self",
                    "a track cannot route to itself",
                );
            }
        }
        for (target, level) in &routing.sends {
            let sat = format!("{at}/routing/sends/{target}");
            if !is_bus_or_master(target) {
                self.add(&sat, "routing_target_invalid",
                    format!("send target `{target}` is not a bus or master track"));
            }
            self.check_finite(&sat, *level);
        }
        for (effect_id, source) in &routing.sidechains {
            let sat = format!("{at}/routing/sidechains/{effect_id}");
            if !track.fx_chain.contains_key(effect_id) {
                self.add(&sat, "sidechain_effect_unknown",
                    format!("`{effect_id}` is not an effect on this track"));
            }
            if !song.tracks.contains_key(source) {
                self.add(&sat, "sidechain_source_unknown",
                    format!("sidechain source `{source}` is not a track in this song"));
            }
        }
    }

    fn check_clips(&mut self, song: &Song) {
        self.check_keyed("/clips", &song.clips, |c| &c.id);

        // §4.4: no two clips overlap on one track unless the track allows it.
        let mut by_track: BTreeMap<&str, Vec<&Clip>> = BTreeMap::new();
        for clip in song.clips.values() {
            by_track.entry(clip.track_id.as_str()).or_default().push(clip);
        }
        for (track_id, clips) in &by_track {
            if song.tracks.get(*track_id).is_some_and(|t| t.allow_overlap) {
                continue;
            }
            let mut spans: Vec<_> = clips
                .iter()
                .map(|c| (c.start_tick as i64, c.start_tick as i64 + c.length_ticks as i64, &c.id))
                .collect();
            spans.sort();
            for pair in spans.windows(2) {
                let (_, prev_end, prev_id) = &pair[0];
                let (next_start, _, next_id) = &pair[1];
                if next_start < prev_end {
                    self.add(
                        format!("/clips/{next_id}"),
                        "clip_overlap",
                        format!(
                            "overlaps `{prev_id}` on track `{track_id}`; set allow_overlap on \
                             the track to permit it"
                        ),
                    );
                }
            }
        }

        for (key, clip) in &song.clips {
            let at = format!("/clips/{key}");
            self.check_provenance(&format!("{at}/provenance"), clip.provenance.as_ref());
            if !song.tracks.contains_key(&clip.track_id) {
                self.add(format!("{at}/track_id"), "track_unknown",
                    format!("`{}` is not a track in this song", clip.track_id));
            }
            if clip.start_tick < 0 {
                self.add(format!("{at}/start_tick"), "tick_negative", "ticks are never negative");
            }
            if clip.length_ticks <= 0 {
                self.add(format!("{at}/length_ticks"), "length_not_positive",
                    "a clip has a positive length, or the render length is not derivable (§4.4)");
            }
            if let Some(loop_len) = clip.loop_length_ticks {
                if loop_len <= 0 {
                    self.add(format!("{at}/loop_length_ticks"), "length_not_positive",
                        "a loop length is positive when present");
                }
            }
            match &clip.content {
                None => self.add(format!("{at}/content"), "oneof_unset",
                    "a clip holds notes or audio"),
                Some(clip::Content::AudioClip(a)) => {
                    let aat = format!("{at}/audio_clip");
                    if a.asset_hash.is_empty() {
                        self.add(format!("{aat}/asset_hash"), "asset_hash_empty",
                            "an audio clip references an asset by hash");
                    }
                    self.check_finite(&format!("{aat}/gain_db"), a.gain_db);
                    // ADR 0011 §2's fade formula is total only over non-negative lengths: a
                    // negative one makes `n / f` negative and flips the signal's sign.
                    let fades =
                        [("fade_in_ticks", a.fade_in_ticks), ("fade_out_ticks", a.fade_out_ticks)];
                    for (field, ticks) in fades {
                        if ticks < 0 {
                            self.add(format!("{aat}/{field}"), "tick_negative",
                                "ticks are never negative");
                        }
                    }
                }
                Some(clip::Content::NoteClip(n)) => {
                    self.check_keyed(&format!("{at}/note_clip/notes"), &n.notes, |x| &x.id);
                    for (nkey, note) in &n.notes {
                        self.check_note(&format!("{at}/note_clip/notes/{nkey}"), note, clip);
                    }
                }
            }
        }
    }

    fn check_note(&mut self, at: &str, note: &Note, clip: &Clip) {
        self.check_provenance(&format!("{at}/provenance"), note.provenance.as_ref());
        if !(0..=127).contains(&note.pitch) {
            self.add(format!("{at}/pitch"), "pitch_out_of_range", "MIDI pitch is 0 to 127");
        }
        if !(0..=127).contains(&note.velocity) {
            self.add(format!("{at}/velocity"), "velocity_out_of_range", "velocity is 0 to 127");
        }
        if note.start_tick < 0 {
            self.add(format!("{at}/start_tick"), "tick_negative", "ticks are never negative");
        }
        if note.length_ticks <= 0 {
            self.add(format!("{at}/length_ticks"), "length_not_positive",
                "a note has a positive length");
        }
        // §4.4: notes lie inside their clip. Widened to i64 so the sum cannot overflow.
        let end = note.start_tick as i64 + note.length_ticks as i64;
        if note.start_tick >= 0 && note.length_ticks > 0 && end > clip.length_ticks as i64 {
            self.add(format!("{at}/length_ticks"), "note_outside_clip",
                format!("note ends at {end}, past the clip's {} ticks", clip.length_ticks));
        }
        self.check_finite(&format!("{at}/microtonal_cents"), note.microtonal_cents);
        for (name, value) in &note.expression {
            self.check_finite(&format!("{at}/expression/{name}"), *value);
        }
    }

    fn check_automation(&mut self, song: &Song) {
        self.check_keyed("/automation", &song.automation, |a| &a.id);

        // A device id is an instrument or effect anywhere in the song; ids are global (§4.3).
        let mut devices: BTreeSet<&str> = BTreeSet::new();
        for track in song.tracks.values() {
            if let Some(i) = &track.instrument {
                devices.insert(i.id.as_str());
            }
            devices.extend(track.fx_chain.values().map(|e| e.id.as_str()));
        }

        for (key, automation) in &song.automation {
            let at = format!("/automation/{key}");
            self.check_provenance(&format!("{at}/provenance"), automation.provenance.as_ref());
            match &automation.target {
                None => self.add(format!("{at}/target"), "message_missing", "`target` is required"),
                Some(target) => {
                    // §4.4 wants the parameter to be real, which needs the plugin's manifest
                    // and therefore the engine (M1). Until then: the device exists and a
                    // parameter is named.
                    if !devices.contains(target.device_id.as_str()) {
                        self.add(format!("{at}/target/device_id"), "device_unknown",
                            format!("`{}` is not an instrument or effect in this song",
                                target.device_id));
                    }
                    if target.param.is_empty() {
                        self.add(format!("{at}/target/param"), "param_empty",
                            "an automation target names a parameter");
                    }
                }
            }
            if automation.points.is_empty() {
                self.add(format!("{at}/points"), "automation_empty",
                    "automation with no points has no effect");
            }
            self.check_keyed(&format!("{at}/points"), &automation.points, |p| &p.id);
            for (pkey, point) in &automation.points {
                let pat = format!("{at}/points/{pkey}");
                self.check_tick(&pat, point.tick);
                self.check_finite(&format!("{pat}/value"), point.value);
                if point.curve == Curve::Unspecified as i32 {
                    self.add(format!("{pat}/curve"), "enum_unspecified", "curve must be set");
                }
            }
        }
    }

    fn check_generators(&mut self, song: &Song) {
        self.check_keyed("/generators", &song.generators, |g| &g.id);
        for (key, generator) in &song.generators {
            let at = format!("/generators/{key}");
            self.check_provenance(&format!("{at}/provenance"), generator.provenance.as_ref());
            if generator.kind == GeneratorKind::Unspecified as i32 {
                self.add(format!("{at}/kind"), "enum_unspecified", "generator kind must be set");
            }
            // §4.4: a non-null seed is structural (ADR 0002 §9), so only this remains.
            if generator.toolchain_version.is_empty() {
                self.add(format!("{at}/toolchain_version"), "toolchain_version_empty",
                    "a generator records the toolchain version it was compiled with (§4.4)");
            }
            if generator.source.is_empty() {
                self.add(format!("{at}/source"), "generator_source_empty", "a generator has source");
            }
            match &generator.target {
                None => self.add(format!("{at}/target"), "oneof_unset",
                    "a generator targets a track or a clip"),
                Some(generator::Target::TrackId(id)) if !song.tracks.contains_key(id) => {
                    self.add(format!("{at}/track_id"), "track_unknown",
                        format!("`{id}` is not a track in this song"));
                }
                Some(generator::Target::ClipId(id)) if !song.clips.contains_key(id) => {
                    self.add(format!("{at}/clip_id"), "clip_unknown",
                        format!("`{id}` is not a clip in this song"));
                }
                Some(_) => {}
            }
        }
    }
}
