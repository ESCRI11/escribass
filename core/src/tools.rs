//! The typed tools of §5, as functions from arguments to operations.
//!
//! Each one builds an RFC 6902 patch and stops. Applying it, bumping versions, validating the
//! result and recording it are [`crate::session::Session`]'s, which is what keeps every tool
//! honest: a tool cannot skip validation, cannot write a file, and cannot invent a rule about
//! what a valid song is.
//!
//! **What a tool checks, and what it leaves to the validator.** A tool checks only what the
//! validator structurally cannot: that an argument names something in *this* song, and that a
//! double is finite before it becomes a JSON number. Everything else — an empty name, an
//! unspecified enum, a duplicate index, a second master track — is §4.4's job, and duplicating
//! those rules here would be two places to change and one place to forget.
//!
//! Ids are minted while the patch is built, from a **fork** of the session's source: a dry run
//! discards the fork, so previewing burns nothing and the apply that follows mints exactly the
//! ids the preview showed (ADR 0006 §3, which §9 needs since a person approves a patch before
//! it is applied).

use crate::clock::Clock;
use crate::history::{History, HistoryError};
use crate::id::IdSource;
use crate::patch::Op;
use crate::validate::Violation;
use escribass_proto::tools::add_clip_request::Content as AddClipContent;
use escribass_proto::tools::{
    AddAutomationRequest, AddClipRequest, AddEffectRequest, AddSectionRequest,
    AddTrackRequest, CreateBranchRequest, DeleteBranchRequest, MoveSectionRequest,
    QuantizeRequest, SetNotesRequest, SetParamRequest, SetTempoRequest,
    SetTrackInstrumentRequest, SwitchBranchRequest, TransposeRequest,
};
use escribass_schema::song::clip::Content;
use escribass_schema::song::{
    Author, Automation, AutomationPoint, Clip, Effect, Instrument, Mix, Note, NoteClip,
    Provenance, Routing, Section, Song, TempoEvent, Track, TrackKind,
};
use serde_json::{json, Value};

/// Where a device parameter lives, so `set_param` can name it without a track id (§4.3 makes
/// ids globally unique, which is what lets a caller omit one).
fn device_path(song: &Song, device_id: &str) -> Option<String> {
    for (track_id, track) in &song.tracks {
        if track.instrument.as_ref().is_some_and(|i| i.id == device_id) {
            return Some(format!("/tracks/{track_id}/instrument"));
        }
        if track.fx_chain.contains_key(device_id) {
            return Some(format!("/tracks/{track_id}/fx_chain/{device_id}"));
        }
    }
    None
}

fn provenance(author: Author, clock: &dyn Clock) -> Provenance {
    Provenance {
        author: author as i32,
        model_id: None,
        prompt_id: None,
        tool_call_id: None,
        created_at: Some(clock.now()),
    }
}

fn refuse(path: impl Into<String>, rule: &'static str, message: impl Into<String>) -> Vec<Violation> {
    vec![Violation { path: path.into(), rule, message: message.into() }]
}

/// `add` rather than `replace` throughout: RFC 6902 `add` on an existing object member
/// replaces it, so one op covers both creating and overwriting, and a tool never has to ask
/// the document which case it is in.
fn add(path: String, value: Value) -> Op {
    Op::Add { path, value }
}

/// An entity as a JSON value, or a refusal naming the argument that made it unserialisable.
///
/// prost keeps an enum value it does not know as the raw `i32`, and the generated serializer
/// then fails on it. `expect` here would panic *inside the transport's lock*: over gRPC one
/// request carrying `kind: 99` would take the process down and poison the session for every
/// later call.
///
/// The two transports still answer such a call differently — MCP's JSON deserializer refuses
/// the value before it reaches a tool, gRPC's binary decoding cannot — so gRPC returns
/// `enum_unknown` where MCP returns `invalid_params`. Both are refusals a caller can act on,
/// which is what ADR 0006 §2 asks for; they are not the *same* refusal.
fn entity_value<T: serde::Serialize>(path: &str, value: &T) -> Result<Value, Vec<Violation>> {
    serde_json::to_value(value).map_err(|e| {
        refuse(path, "enum_unknown", format!("this is not a value the schema knows: {e}"))
    })
}

// ---------------------------------------------------------------------------

/// §5 `add_track`.
///
/// `index` is the display and mixer order (§4.2 has none; the wireframe does). A new track goes
/// after the current last, which is what a caller means by "add".
///
/// An instrument track is created **with** its instrument. ADR 0002 requires `instrument`
/// present exactly when `kind == INSTRUMENT`, so there is no valid song in which an instrument
/// track is waiting to be filled in — a two-call sequence would have to pass through a
/// document §4.4 refuses, and §5 validates every call.
pub fn add_track(
    song: &Song,
    request: &AddTrackRequest,
    ids: &mut dyn IdSource,
    clock: &dyn Clock,
    author: Author,
) -> Result<Vec<Op>, Vec<Violation>> {
    let is_instrument = request.kind == TrackKind::Instrument as i32;
    match (is_instrument, request.r#ref.is_some()) {
        (true, false) => {
            return Err(refuse(
                "/ref",
                "instrument_presence",
                "an instrument track is created with its instrument (ADR 0002)",
            ))
        }
        (false, true) => {
            return Err(refuse(
                "/ref",
                "instrument_presence",
                "only an instrument track carries an instrument",
            ))
        }
        _ => {}
    }

    let id = ids.next_id();
    // Saturating: `index` is a `uint32` a caller can set through `apply_patch`, and a track at
    // `u32::MAX` would otherwise panic here in a debug build and wrap to 0 in a release one —
    // a duplicate index committed silently. Saturated, the validator refuses it with a rule.
    let next_index = song.tracks.values().map(|t| t.index).max().map_or(0, |last| last.saturating_add(1));
    let instrument = is_instrument.then(|| Instrument {
        id: ids.next_id(),
        provenance: Some(provenance(author, clock)),
        version: 0,
        r#ref: request.r#ref.clone(),
        state: Vec::new(),
        params: Default::default(),
    });

    let track = Track {
        id: id.clone(),
        provenance: Some(provenance(author, clock)),
        version: 0,
        name: request.name.clone(),
        kind: request.kind,
        index: next_index,
        instrument,
        fx_chain: Default::default(),
        routing: Some(Routing::default()),
        // A track that arrives silent or hard-panned would be a surprise; these are the
        // neutral values, not a policy.
        mix: Some(Mix { gain_db: 0.0, pan: 0.0, mute: false, solo: false }),
        allow_overlap: false,
    };

    Ok(vec![add(format!("/tracks/{id}"), entity_value("/kind", &track)?)])
}

/// §5 `set_track_instrument`. Replaces whatever the track had.
pub fn set_track_instrument(
    song: &Song,
    request: &SetTrackInstrumentRequest,
    ids: &mut dyn IdSource,
    clock: &dyn Clock,
    author: Author,
) -> Result<Vec<Op>, Vec<Violation>> {
    if !song.tracks.contains_key(&request.track_id) {
        return Err(refuse(
            "/track_id",
            "track_unknown",
            format!("`{}` is not a track in this song", request.track_id),
        ));
    }

    let instrument = Instrument {
        id: ids.next_id(),
        provenance: Some(provenance(author, clock)),
        version: 0,
        r#ref: request.r#ref.clone(),
        state: Vec::new(),
        params: Default::default(),
    };

    Ok(vec![add(
        format!("/tracks/{}/instrument", request.track_id),
        entity_value("/ref", &instrument)?,
    )])
}

/// §5 `add_effect`. Absent `index` appends after the chain's current last.
pub fn add_effect(
    song: &Song,
    request: &AddEffectRequest,
    ids: &mut dyn IdSource,
    clock: &dyn Clock,
    author: Author,
) -> Result<Vec<Op>, Vec<Violation>> {
    let Some(track) = song.tracks.get(&request.track_id) else {
        return Err(refuse(
            "/track_id",
            "track_unknown",
            format!("`{}` is not a track in this song", request.track_id),
        ));
    };

    let id = ids.next_id();
    let index = request
        .index
        .unwrap_or_else(|| {
            track.fx_chain.values().map(|e| e.index).max().map_or(0, |l| l.saturating_add(1))
        });

    let effect = Effect {
        id: id.clone(),
        provenance: Some(provenance(author, clock)),
        version: 0,
        r#ref: request.r#ref.clone(),
        state: Vec::new(),
        params: Default::default(),
        index,
    };

    Ok(vec![add(
        format!("/tracks/{}/fx_chain/{id}", request.track_id),
        entity_value("/ref", &effect)?,
    )])
}

/// §5 `set_param`.
///
/// The finiteness check is the one that matters. `json!(f64::NAN)` is `Value::Null`, not a
/// number, so a NaN arriving here would become a null in an operation and fail much later as a
/// confusing `op_illegal_for_schema` — pointing at a path rather than at the argument that was
/// wrong. §4.4 requires every double be finite; this is where that becomes answerable.
pub fn set_param(song: &Song, request: &SetParamRequest) -> Result<Vec<Op>, Vec<Violation>> {
    let Some(device) = device_path(song, &request.device_id) else {
        return Err(refuse(
            "/device_id",
            "device_unknown",
            format!("`{}` is not an instrument or effect in this song", request.device_id),
        ));
    };
    if request.param.is_empty() {
        return Err(refuse("/param", "param_empty", "a parameter is named"));
    }
    if !request.value.is_finite() {
        return Err(refuse(
            "/value",
            "double_not_finite",
            format!("`{}` is not a finite number (§4.4)", request.value),
        ));
    }

    Ok(vec![add(
        format!("{device}/params/{}", crate::patch::escape_token(&request.param)),
        json!(request.value),
    )])
}

// ---------------------------------------------------------------------------
// Clips and notes (§5)
// ---------------------------------------------------------------------------

/// The notes of a clip, or a refusal naming why it has none.
fn note_clip<'a>(song: &'a Song, clip_id: &str) -> Result<&'a NoteClip, Vec<Violation>> {
    let Some(clip) = song.clips.get(clip_id) else {
        return Err(refuse(
            "/clip_id",
            "clip_unknown",
            format!("`{clip_id}` is not a clip in this song"),
        ));
    };
    match &clip.content {
        Some(Content::NoteClip(notes)) => Ok(notes),
        _ => Err(refuse(
            "/clip_id",
            "clip_kind_mismatch",
            format!("`{clip_id}` is an audio clip; it has no notes"),
        )),
    }
}

/// The notes a call applies to: the ones named, or all of them when none are.
///
/// An unknown id is refused rather than skipped. A caller that misspells a note id and gets a
/// success back has been told its edit landed when it did not.
fn selected<'a>(
    notes: &'a NoteClip,
    note_ids: &[String],
) -> Result<Vec<(&'a String, &'a Note)>, Vec<Violation>> {
    if note_ids.is_empty() {
        return Ok(notes.notes.iter().collect());
    }
    let mut found = Vec::with_capacity(note_ids.len());
    for id in note_ids {
        let Some(note) = notes.notes.get_key_value(id) else {
            return Err(refuse(
                "/note_ids",
                "note_unknown",
                format!("`{id}` is not a note in this clip"),
            ));
        };
        found.push(note);
    }
    Ok(found)
}

/// §5 `add_clip`. Absent content makes an empty note clip.
pub fn add_clip(
    song: &Song,
    request: &AddClipRequest,
    ids: &mut dyn IdSource,
    clock: &dyn Clock,
    author: Author,
) -> Result<Vec<Op>, Vec<Violation>> {
    if !song.tracks.contains_key(&request.track_id) {
        return Err(refuse(
            "/track_id",
            "track_unknown",
            format!("`{}` is not a track in this song", request.track_id),
        ));
    }

    let id = ids.next_id();
    // Notes arriving with a clip are minted the same way `set_notes` mints them: a caller
    // never supplies an id, provenance or a version (ADR 0006 §4).
    let content = match &request.content {
        Some(AddClipContent::AudioClip(audio)) => Some(Content::AudioClip(audio.clone())),
        Some(AddClipContent::NoteClip(notes)) => {
            Some(Content::NoteClip(minted_notes(&notes.notes, &Default::default(), ids, clock, author)))
        }
        None => Some(Content::NoteClip(NoteClip::default())),
    };

    let clip = Clip {
        id: id.clone(),
        provenance: Some(provenance(author, clock)),
        version: 0,
        track_id: request.track_id.clone(),
        start_tick: request.start_tick,
        length_ticks: request.length_ticks,
        loop_length_ticks: None,
        content,
    };

    Ok(vec![add(format!("/clips/{id}"), entity_value("/content", &clip)?)])
}

/// Mints ids and provenance for a set of notes, keeping any id the clip already holds.
fn minted_notes(
    given: &std::collections::BTreeMap<String, Note>,
    existing: &std::collections::BTreeMap<String, Note>,
    ids: &mut dyn IdSource,
    clock: &dyn Clock,
    author: Author,
) -> NoteClip {
    let mut notes = std::collections::BTreeMap::new();
    for (key, note) in given {
        // A key the clip already holds is an edit to *that note*; anything else is a new note,
        // whatever the caller called it.
        //
        // Core mints §4.3's fields for a new entity and **preserves** them for one that
        // exists. Re-minting provenance would rewrite the author and `created_at` of every
        // note in the clip on every call — invisible under a fixed clock, and under a real one
        // it loses authorship, bumps notes nobody touched, and makes two branches that each
        // call `set_notes` conflict on timestamps for notes neither of them meant to change.
        match existing.get(key) {
            Some(held) => notes.insert(
                key.clone(),
                Note {
                    id: key.clone(),
                    provenance: held.provenance.clone(),
                    version: held.version,
                    ..note.clone()
                },
            ),
            None => {
                let id = ids.next_id();
                notes.insert(
                    id.clone(),
                    Note {
                        id,
                        provenance: Some(provenance(author, clock)),
                        version: 0,
                        ..note.clone()
                    },
                )
            }
        };
    }
    NoteClip { notes }
}

/// §5 `set_notes`. Replaces the clip's whole note set.
///
/// Replacement rather than merge because "the notes are now these" is what an editor and a
/// generator both mean, and a merge would leave no way to say "and nothing else". A note whose
/// key the clip already holds keeps its id, so an edit to one note is a change to that note
/// rather than a delete and an insert — which is what keeps two branches editing different
/// notes from conflicting (ADR 0001 §4).
pub fn set_notes(
    song: &Song,
    request: &SetNotesRequest,
    ids: &mut dyn IdSource,
    clock: &dyn Clock,
    author: Author,
) -> Result<Vec<Op>, Vec<Violation>> {
    let existing = note_clip(song, &request.clip_id)?;
    let notes = minted_notes(&request.notes, &existing.notes, ids, clock, author);

    Ok(vec![add(
        format!("/clips/{}/note_clip/notes", request.clip_id),
        entity_value("/notes", &notes.notes)?,
    )])
}

/// §5 `transpose`.
///
/// One op per note, not one for the whole clip: M0.3's merge auto-resolves by comparing op
/// paths (ADR 0001 §4), so a coarse whole-clip patch would make every pair of edits to one clip
/// conflict.
///
/// A result outside MIDI 0-127 is refused by the validator (`pitch_out_of_range`), not clamped
/// here. Silently rewriting what a caller asked for is how a tool stops being predictable, and
/// the rule already exists in the one place §4.4 puts it.
pub fn transpose(song: &Song, request: &TransposeRequest) -> Result<Vec<Op>, Vec<Violation>> {
    let notes = note_clip(song, &request.clip_id)?;
    let chosen = selected(notes, &request.note_ids)?;

    Ok(chosen
        .into_iter()
        .map(|(id, note)| Op::Replace {
            path: format!("/clips/{}/note_clip/notes/{id}/pitch", request.clip_id),
            // Saturating, so an absurd argument cannot wrap into a legal pitch. The validator
            // then refuses the result, which is the report the caller wants.
            value: json!(note.pitch.saturating_add(request.semitones)),
        })
        .collect())
}

/// §5 `quantize`.
///
/// Integer arithmetic throughout, and ties round up. Both matter for §11: a float round would
/// depend on the platform's rounding mode at the halfway point, and an unstated tie rule would
/// make two implementations of this tool disagree on exactly the notes a musician places
/// deliberately.
pub fn quantize(song: &Song, request: &QuantizeRequest) -> Result<Vec<Op>, Vec<Violation>> {
    if request.grid_ticks <= 0 {
        return Err(refuse(
            "/grid_ticks",
            "grid_not_positive",
            format!("a grid is a positive number of ticks, not `{}`", request.grid_ticks),
        ));
    }
    let notes = note_clip(song, &request.clip_id)?;
    let chosen = selected(notes, &request.note_ids)?;
    let grid = request.grid_ticks;

    Ok(chosen
        .into_iter()
        .map(|(id, note)| Op::Replace {
            path: format!("/clips/{}/note_clip/notes/{id}/start_tick", request.clip_id),
            value: json!(snap(note.start_tick, grid)),
        })
        .collect())
}

/// Rounds `tick` to the nearest multiple of `grid`, ties away from zero.
///
/// Ticks are never negative (§4.4), but the negative branch is here rather than assumed: a
/// tool that silently did the wrong thing on an input the validator would have caught is a
/// worse failure than one that does the right thing on it.
fn snap(tick: i32, grid: i32) -> i64 {
    let (tick, grid) = (i64::from(tick), i64::from(grid));
    let half = grid / 2;
    // In `i64`, because `tick + half` overflows `i32` for a tick near the top of the range —
    // which the validator permits, since a clip may be `i32::MAX` long. An out-of-range result
    // is refused by the schema on re-deserialisation, which is the report the caller wants;
    // a wrapped one would arrive as a negative tick with a misleading rule.
    if tick >= 0 {
        (tick + half) / grid * grid
    } else {
        -((-tick + half) / grid * grid)
    }
}

// ---------------------------------------------------------------------------
// Automation, time base and arrangement (§5)
// ---------------------------------------------------------------------------

/// §5 `add_automation`.
pub fn add_automation(
    song: &Song,
    request: &AddAutomationRequest,
    ids: &mut dyn IdSource,
    clock: &dyn Clock,
    author: Author,
) -> Result<Vec<Op>, Vec<Violation>> {
    let Some(target) = request.target.clone() else {
        return Err(refuse("/target", "required", "an automation lane names a parameter"));
    };
    if device_path(song, &target.device_id).is_none() {
        return Err(refuse(
            "/target/device_id",
            "device_unknown",
            format!("`{}` is not an instrument or effect in this song", target.device_id),
        ));
    }
    for (key, point) in &request.points {
        if !point.value.is_finite() {
            return Err(refuse(
                format!("/points/{key}/value"),
                "double_not_finite",
                format!("`{}` is not a finite number (§4.4)", point.value),
            ));
        }
    }

    let id = ids.next_id();
    // Points carry an id and no provenance (ADR 0002 §2), so only the id is core's to mint.
    let points = request
        .points
        .values()
        .map(|point| {
            let point_id = ids.next_id();
            (point_id.clone(), AutomationPoint { id: point_id, ..point.clone() })
        })
        .collect();

    let automation = Automation {
        id: id.clone(),
        provenance: Some(provenance(author, clock)),
        version: 0,
        target: Some(target),
        points,
    };

    Ok(vec![add(format!("/automation/{id}"), entity_value("/points", &automation)?)])
}

/// §5 `set_tempo`. Upserts the event at `tick`.
///
/// Upsert rather than append because two tempo events at one tick have no defined order —
/// §4.2 derives order from `tick`, and a map has no positions to break the tie with.
pub fn set_tempo(
    song: &Song,
    request: &SetTempoRequest,
    ids: &mut dyn IdSource,
) -> Result<Vec<Op>, Vec<Violation>> {
    if !request.bpm.is_finite() {
        return Err(refuse(
            "/bpm",
            "double_not_finite",
            format!("`{}` is not a finite number (§4.4)", request.bpm),
        ));
    }

    let existing = song
        .tempo_map
        .as_ref()
        .and_then(|map| map.events.values().find(|e| e.tick == request.tick));

    // Replacing only `bpm` on an existing event, rather than the whole event, keeps the patch
    // path specific enough for a merge to see two branches changing different things.
    if let Some(event) = existing {
        return Ok(vec![Op::Replace {
            path: format!("/tempo_map/events/{}/bpm", event.id),
            value: json!(request.bpm),
        }]);
    }

    let id = ids.next_id();
    let event = TempoEvent { id: id.clone(), tick: request.tick, bpm: request.bpm };
    Ok(vec![add(format!("/tempo_map/events/{id}"), entity_value("/bpm", &event)?)])
}

/// §5 `add_section`.
pub fn add_section(
    request: &AddSectionRequest,
    ids: &mut dyn IdSource,
    clock: &dyn Clock,
    author: Author,
) -> Result<Vec<Op>, Vec<Violation>> {
    let id = ids.next_id();
    let section = Section {
        id: id.clone(),
        provenance: Some(provenance(author, clock)),
        version: 0,
        name: request.name.clone(),
        start_tick: request.start_tick,
        end_tick: request.end_tick,
    };
    Ok(vec![add(format!("/sections/{id}"), entity_value("/name", &section)?)])
}

/// §5 `move_section`.
///
/// Moves the label, not the music. §4.2 makes a section a name over a range of ticks rather
/// than a container, so nothing inside it moves — and a tool that quietly dragged clips along
/// would be inventing an arrangement model the schema does not have.
pub fn move_section(song: &Song, request: &MoveSectionRequest) -> Result<Vec<Op>, Vec<Violation>> {
    if !song.sections.contains_key(&request.section_id) {
        return Err(refuse(
            "/section_id",
            "section_unknown",
            format!("`{}` is not a section in this song", request.section_id),
        ));
    }
    let at = format!("/sections/{}", request.section_id);
    Ok(vec![
        Op::Replace { path: format!("{at}/start_tick"), value: json!(request.start_tick) },
        Op::Replace { path: format!("{at}/end_tick"), value: json!(request.end_tick) },
    ])
}

// ---------------------------------------------------------------------------
// Branches (ADR 0001 §2)
// ---------------------------------------------------------------------------

/// A `HistoryError` as a refusal. Every way these three tools can fail is something the caller
/// can fix by calling differently, so none of them is an operator error (ADR 0006 §2).
fn as_violation(e: HistoryError) -> Vec<Violation> {
    vec![Violation { path: e.path, rule: e.rule, message: e.message }]
}

/// §5 `create_branch`. An empty `at_entry_id` means the current `HEAD`.
///
/// Returns the entry the new ref will point at, so the caller does not resolve `HEAD` twice.
pub fn check_create_branch(
    history: &History,
    request: &CreateBranchRequest,
) -> Result<String, Vec<Violation>> {
    let at = if request.at_entry_id.is_empty() {
        history.head_id().map(str::to_string).ok_or_else(|| {
            vec![Violation {
                path: "/at_entry_id".to_string(),
                rule: "head_unset",
                message: "HEAD names no entry to branch from".to_string(),
            }]
        })?
    } else {
        request.at_entry_id.clone()
    };
    history.check_create_ref(&request.name, &at).map_err(as_violation)?;
    Ok(at)
}

/// §5 `switch_branch`: the patch that would take the current document to that branch.
///
/// Uses `History::patch_to`, which does **not** move `HEAD`. `History::switch` does, so a dry
/// run built on it would leave the next commit landing on a branch nobody chose.
pub fn check_switch_branch(
    history: &History,
    song: &Song,
    request: &SwitchBranchRequest,
) -> Result<Vec<Op>, Vec<Violation>> {
    let current = serde_json::to_value(song).expect("a Song serialises");
    history.patch_to(&request.name, &current).map_err(as_violation)
}

/// §5 `delete_branch`.
pub fn check_delete_branch(
    history: &History,
    request: &DeleteBranchRequest,
) -> Result<(), Vec<Violation>> {
    history.check_delete_ref(&request.name).map_err(as_violation)
}
