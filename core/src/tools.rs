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
//! Ids are minted before the patch is built, so a dry run consumes them. `ponytail:` ids are
//! never reused (§4.3) and a burned one costs nothing; the alternative is a second id source
//! for previews, which is a second thing to keep in step for no gain.

use crate::clock::Clock;
use crate::id::IdSource;
use crate::patch::Op;
use crate::validate::Violation;
use escribass_proto::tools::{
    AddEffectRequest, AddTrackRequest, SetParamRequest, SetTrackInstrumentRequest,
};
use escribass_schema::song::{
    Author, Effect, Instrument, Mix, Provenance, Routing, Song, Track, TrackKind,
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
    let next_index = song.tracks.values().map(|t| t.index).max().map_or(0, |last| last + 1);
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

    Ok(vec![add(
        format!("/tracks/{id}"),
        serde_json::to_value(&track).expect("a Track serialises"),
    )])
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
        serde_json::to_value(&instrument).expect("an Instrument serialises"),
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
        .unwrap_or_else(|| track.fx_chain.values().map(|e| e.index).max().map_or(0, |l| l + 1));

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
        serde_json::to_value(&effect).expect("an Effect serialises"),
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
