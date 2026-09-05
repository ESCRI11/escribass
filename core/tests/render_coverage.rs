//! The field-coverage guard (ADR 0007 §5): what proves the plan is derived and not a second
//! model.
//!
//! Prose saying "keep the plan in sync with the schema" is the promise ADR 0006 §4 records
//! failing silently, so the guarantee is this test. It walks every field of every `song.v1`
//! message in the compiled descriptor — the same artefact the Rust types and the tool schemas
//! come from, so a field cannot exist in one and not here — and requires each to be in
//! exactly one of three places:
//!
//! - **carried** into the plan by value, because its message is embedded in `render.proto`;
//! - **consumed** — read by `compile` to derive plan content rather than copied — and named
//!   on the list below;
//! - **ignored**, named on the list below *with the reason* no sample depends on it.
//!
//! A field added to `song.proto` that is on no list fails here, instead of rendering as
//! nothing. A field on both lists fails too, because that is a list nobody re-read.
//!
//! What "carried" means for a listed field: a message crosses by value whole, so every field
//! of an embedded message is carried *unless a list says otherwise* — `id`, `provenance` and
//! `version` cross empty (ADR 0007 §2), and `Mix.mute` and `Mix.solo` cross false because
//! compile has already applied them. That those fields really are blank in the plan is
//! `tests/render.rs`'s claim, and that a consumed field changes the plan is the plan golden's
//! (ADR 0007 §5: the guard proves coverage, not use).

use escribass_core::{message_fields, Field};
use escribass_proto::DESCRIPTOR;
use std::collections::{BTreeMap, BTreeSet};

const SONG: &str = ".escribass.song.v1.";
const RENDER: &str = ".escribass.render.v1.";

/// Fields `compile` reads to derive plan content rather than copy it: `(message, field)`.
const CONSUMED: &[(&str, &str)] = &[
    // The root: each collection is walked, the target is checked and then copied whole.
    ("Song", "tempo_map"),
    ("Song", "sections"),
    ("Song", "tracks"),
    ("Song", "clips"),
    ("Song", "automation"),
    ("Song", "render_target"),
    ("TempoMap", "events"),
    // Where a section ends is where the render may end; where it starts moves nothing.
    ("Section", "end_tick"),
    // A track becomes a PlanTrack, or master, or nothing.
    ("Track", "id"),
    ("Track", "kind"),
    ("Track", "index"),
    ("Track", "instrument"),
    ("Track", "fx_chain"),
    ("Track", "routing"),
    ("Track", "mix"),
    ("Routing", "output_track_id"),
    ("Routing", "sends"),
    ("Routing", "sidechains"),
    // Applied by compile, so they cross false: a `solo` the engine could see is one it
    // could act on (ADR 0007 §1).
    ("Mix", "mute"),
    ("Mix", "solo"),
    // What `ParamRef.device_id` resolves against; both cross blank.
    ("Instrument", "id"),
    ("Effect", "id"),
    ("Effect", "index"),
    ("NoteClip", "notes"),
    // A clip becomes one plan clip per loop iteration, on its track.
    ("Clip", "track_id"),
    ("Clip", "start_tick"),
    ("Clip", "length_ticks"),
    ("Clip", "loop_length_ticks"),
    ("Clip", "note_clip"),
    ("Clip", "audio_clip"),
    // A lane is nested under the device the ref names, keyed by the parameter.
    ("ParamRef", "device_id"),
    ("ParamRef", "param"),
    ("Automation", "target"),
    ("Automation", "points"),
];

/// Fields no sample depends on: `(message, field, reason)`.
///
/// `("*", field, …)` covers that field on every *entity* — a message with `id`, `provenance`
/// and `version` — and nothing else, because `PluginRef.version` is a string naming a plugin
/// release and must cross. `(message, "*", …)` covers every field of one message.
const IGNORED: &[(&str, &str, &str)] = &[
    (
        "*",
        "provenance",
        "history metadata never reaches the engine (ADR 0001 Consequences); crosses empty \
         where its message crosses (ADR 0007 §2)",
    ),
    (
        "*",
        "version",
        "an edit count for merge (ADR 0005 §2); crosses as 0 where its message crosses \
         (ADR 0007 §2)",
    ),
    ("Provenance", "*", "the fields of a provenance: who, with what, when — none is a sound"),
    ("Song", "id", "the document's identity; the plan is of its contents"),
    (
        "Song",
        "schema_version",
        "checked against lock.json at load (§11); a song compile is handed was already accepted",
    ),
    (
        "Song",
        "time_signature_map",
        "a bar line changes no sample: ticks are the only musical time (§4.2, ADR 0007 §5)",
    ),
    ("TimeSignatureMap", "*", "see Song.time_signature_map"),
    ("TimeSignatureEvent", "*", "see Song.time_signature_map"),
    ("Song", "markers", "navigation labels; nothing sounds at a marker"),
    ("Marker", "*", "see Song.markers"),
    (
        "Song",
        "generators",
        "a generator's compiled output is already the target clip's notes, which are layer 2 \
         and rendered as such (song.proto, ADR 0007 §5)",
    ),
    ("Generator", "*", "see Song.generators"),
    (
        "Section",
        "id",
        "keyed for patch paths (ADR 0001 §3); a section reaches the plan only as a length",
    ),
    ("Section", "name", "a label"),
    ("Section", "start_tick", "only where a section ends can move the render's end"),
    ("Track", "name", "display only"),
    (
        "Track",
        "allow_overlap",
        "a permission the validator checks at edit time (§4.4); the clips arrive either way",
    ),
    (
        "TempoEvent",
        "id",
        "keyed for patch paths only (ADR 0002 §2); breaks an order tie as the map's key and \
         crosses empty",
    ),
    (
        "AutomationPoint",
        "id",
        "keyed for patch paths only (ADR 0002 §2); breaks an order tie as the map's key and \
         crosses empty",
    ),
    (
        "Note",
        "id",
        "keyed for patch paths (ADR 0001 §3); breaks an order tie as the map's key and crosses \
         empty",
    ),
    (
        "Clip",
        "id",
        "keyed for patch paths (ADR 0001 §3); breaks an order tie as the map's key and names a \
         refusal's path; a plan clip has no id",
    ),
    (
        "Automation",
        "id",
        "keyed for patch paths (ADR 0001 §3); breaks a merge-order tie as the map's key; a \
         lane is keyed by its parameter",
    ),
];

/// The §4.3 message that never crosses: the walk does not descend through it, so
/// `Provenance` is not embedded and its fields must be accounted for like any other.
const NEVER_CROSSES: &str = "provenance";

fn model() -> BTreeMap<String, Vec<Field>> {
    message_fields(DESCRIPTOR).expect("the descriptor decodes")
}

fn is_entity(fields: &[Field]) -> bool {
    ["id", "provenance", "version"].iter().all(|f| fields.iter().any(|held| held.name == *f))
}

/// The `song.v1` messages a plan carries by value: reachable from a `render.v1` message
/// through message-typed fields, other than the one that crosses empty.
fn embedded(messages: &BTreeMap<String, Vec<Field>>) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut pending: Vec<String> = messages.keys().filter(|n| n.starts_with(RENDER)).cloned().collect();
    while let Some(name) = pending.pop() {
        for field in &messages[&name] {
            if field.name == NEVER_CROSSES {
                continue;
            }
            if let Some(inner) = &field.message {
                if inner.starts_with(SONG) && found.insert(inner.clone()) {
                    pending.push(inner.clone());
                }
            }
        }
    }
    found
}

fn short(name: &str) -> &str {
    name.strip_prefix(SONG).unwrap_or(name)
}

fn consumed(message: &str, field: &str) -> bool {
    CONSUMED.contains(&(message, field))
}

fn ignored(message: &str, field: &str, entity: bool) -> bool {
    IGNORED.iter().any(|(m, f, _)| {
        let message_matches = *m == message || (*m == "*" && entity);
        message_matches && (*f == field || *f == "*")
    })
}

#[test]
fn every_field_of_the_model_is_carried_consumed_or_ignored() {
    let messages = model();
    let carried = embedded(&messages);

    let mut failures = Vec::new();
    for (name, fields) in messages.iter().filter(|(n, _)| n.starts_with(SONG)) {
        let message = short(name);
        let entity = is_entity(fields);
        for field in fields {
            let consumed = consumed(message, &field.name);
            let ignored = ignored(message, &field.name, entity);
            match (carried.contains(name), consumed, ignored) {
                (_, true, true) => failures.push(format!(
                    "{message}.{} is on both the consumed and the ignored list",
                    field.name
                )),
                (false, false, false) => failures.push(format!(
                    "{message}.{} reaches the plan nowhere: its message does not cross, and no \
                     list names it. Does a sample depend on it?",
                    field.name
                )),
                _ => {}
            }
        }
    }
    assert!(failures.is_empty(), "\n{}\n", failures.join("\n"));
}

#[test]
fn the_messages_that_cross_by_value_are_the_leaves_adr_0007_names() {
    // Rule 1 of ADR 0007 §2: a message crosses whole only if it holds no entity collection
    // and no id naming another entity. Pinning the set keeps a future `Clip` or `Track` from
    // being embedded by accident — which would carry `track_id` into the plan.
    let found: BTreeSet<String> = embedded(&model()).iter().map(|n| short(n).to_string()).collect();
    let expected: BTreeSet<String> = [
        "Note", "Instrument", "Effect", "Mix", "AutomationPoint", "TempoEvent", "DeviceRef",
        "PluginRef", "SourceRef", "ModelRef", "SamplerRef", "RenderTarget", "AudioClip",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(found, expected);
}

#[test]
fn every_list_entry_names_a_field_that_exists() {
    // A field renamed or removed in song.proto must take its list entry with it, or the list
    // quietly describes a model that no longer exists.
    let messages = model();
    let field_exists = |message: &str, field: &str| {
        messages
            .get(&format!("{SONG}{message}"))
            .is_some_and(|fields| fields.iter().any(|f| f.name == field))
    };
    let entities: Vec<&str> = messages
        .iter()
        .filter(|(n, f)| n.starts_with(SONG) && is_entity(f))
        .map(|(n, _)| short(n))
        .collect();

    for (message, field) in CONSUMED {
        assert!(field_exists(message, field), "consumed: {message}.{field} does not exist");
    }
    for (message, field, reason) in IGNORED {
        assert!(!reason.trim().is_empty(), "ignored: {message}.{field} gives no reason");
        match (*message, *field) {
            ("*", field) => assert!(
                entities.iter().any(|e| field_exists(e, field)),
                "ignored: no entity has a `{field}` field"
            ),
            (message, "*") => assert!(
                messages.contains_key(&format!("{SONG}{message}")),
                "ignored: {message} does not exist"
            ),
            (message, field) => {
                assert!(field_exists(message, field), "ignored: {message}.{field} does not exist")
            }
        }
    }
}
