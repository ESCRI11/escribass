//! Round-trip and canonical-JSON tests for the generated model types.
//!
//! There is no validator yet (M0.2), so what these prove is narrow and deliberate: that the
//! generated types express §4, that the canonical JSON of ADR 0002 §4 is what the serde
//! impls actually produce, and that Rust, TypeScript and Python all read the same bytes.
//!
//! The fixture is written *by this test* from generated types (`UPDATE_FIXTURES=1`), never
//! by hand — the nearest thing to CLAUDE.md #2 available until M0.4 can drive fixtures
//! through the tool API.

use escribass_schema::history::{PatchEntry, Refs};
use escribass_schema::pbjson_types;
use escribass_schema::song::*;
use std::collections::BTreeMap;

const SONG_FIXTURE: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");

/// Fixed instant so the fixture is byte-stable: 2026-09-02T00:00:00Z.
const CREATED_AT: i64 = 1_788_307_200;

fn prov(author: Author, tool_call_id: Option<&str>) -> Option<Provenance> {
    Some(Provenance {
        author: author as i32,
        model_id: tool_call_id.map(|_| "anthropic/claude-opus-5".to_string()),
        prompt_id: None,
        tool_call_id: tool_call_id.map(str::to_string),
        created_at: Some(pbjson_types::Timestamp { seconds: CREATED_AT, nanos: 0 }),
    })
}

fn note(id: &str, pitch: i32, start_tick: i32, expression: &[(&str, f64)]) -> Note {
    Note {
        id: id.to_string(),
        provenance: prov(Author::Model, Some("call_01H")),
        version: 1,
        pitch,
        microtonal_cents: 0.0,
        start_tick,
        length_ticks: 480,
        velocity: 104,
        expression: expression.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
    }
}

/// Two tracks, one clip of two notes, automation, tempo, a section and a generator.
/// Exercises every construct the schema has: maps, both oneofs, `optional` presence,
/// a well-known type, and a 64-bit integer.
fn build() -> Song {
    let bass = Track {
        id: "01K4F2T001".to_string(),
        provenance: prov(Author::Human, None),
        version: 3,
        name: "Bass".to_string(),
        kind: TrackKind::Instrument as i32,
        index: 0,
        instrument: Some(Instrument {
            id: "01K4F2A000".to_string(),
            provenance: prov(Author::Model, Some("call_01A")),
            version: 1,
            r#ref: Some(DeviceRef {
                kind: Some(device_ref::Kind::Cmajor(SourceRef {
                    source_hash: "8b31c0de4f9c00000000000000000000".to_string(),
                })),
            }),
            state: vec![0x01, 0x02],
            params: [("drive".to_string(), 0.62)].into_iter().collect(),
        }),
        fx_chain: [(
            "01K4F2E001".to_string(),
            Effect {
                id: "01K4F2E001".to_string(),
                provenance: prov(Author::Human, None),
                version: 1,
                r#ref: Some(DeviceRef {
                    kind: Some(device_ref::Kind::Plugin(PluginRef {
                        plugin_id: "org.surge-synth.surge-xt".to_string(),
                        version: "1.3.4".to_string(),
                    })),
                }),
                state: Vec::new(),
                params: BTreeMap::new(),
                index: 0,
            },
        )]
        .into_iter()
        .collect(),
        routing: Some(Routing {
            output_track_id: None,
            sends: BTreeMap::new(),
            sidechains: BTreeMap::new(),
        }),
        mix: Some(Mix { gain_db: -6.5, pan: 0.0, mute: false, solo: false }),
        allow_overlap: false,
    };

    let master = Track {
        id: "01K4F2T002".to_string(),
        provenance: prov(Author::Human, None),
        version: 1,
        name: "Master".to_string(),
        kind: TrackKind::Master as i32,
        index: 1,
        instrument: None,
        fx_chain: BTreeMap::new(),
        routing: Some(Routing {
            output_track_id: None,
            sends: BTreeMap::new(),
            sidechains: BTreeMap::new(),
        }),
        mix: Some(Mix { gain_db: 0.0, pan: 0.0, mute: false, solo: false }),
        allow_overlap: false,
    };

    // Bar 17 at 960 PPQ in 4/4: (17 - 1) * 4 * 960.
    let clip = Clip {
        id: "01K4F2QN8B".to_string(),
        provenance: prov(Author::Model, Some("call_01B")),
        version: 2,
        track_id: "01K4F2T001".to_string(),
        start_tick: 61_440,
        length_ticks: 7_680,
        loop_length_ticks: None,
        content: Some(clip::Content::NoteClip(NoteClip {
            notes: [
                ("01K4F2N001".to_string(), note("01K4F2N001", 43, 0, &[])),
                ("01K4F2N002".to_string(), note("01K4F2N002", 38, 1_440, &[("timbre", 0.62)])),
            ]
            .into_iter()
            .collect(),
        })),
    };

    let automation = Automation {
        id: "01K4F2U001".to_string(),
        provenance: prov(Author::Human, None),
        version: 1,
        target: Some(ParamRef {
            device_id: "01K4F2A000".to_string(),
            param: "cutoff".to_string(),
        }),
        points: [
            (
                "01K4F2P001".to_string(),
                AutomationPoint {
                    id: "01K4F2P001".to_string(),
                    tick: 61_440,
                    value: 0.2,
                    curve: Curve::Linear as i32,
                },
            ),
            (
                "01K4F2P002".to_string(),
                AutomationPoint {
                    id: "01K4F2P002".to_string(),
                    tick: 69_120,
                    value: 0.8,
                    curve: Curve::Hold as i32,
                },
            ),
        ]
        .into_iter()
        .collect(),
    };

    Song {
        id: "01K4F2S000".to_string(),
        provenance: prov(Author::Human, None),
        version: 214,
        schema_version: 1,
        tempo_map: Some(TempoMap {
            events: [(
                "01K4F2M001".to_string(),
                TempoEvent { id: "01K4F2M001".to_string(), tick: 0, bpm: 92.0 },
            )]
            .into_iter()
            .collect(),
        }),
        time_signature_map: Some(TimeSignatureMap {
            events: [(
                "01K4F2M002".to_string(),
                TimeSignatureEvent {
                    id: "01K4F2M002".to_string(),
                    tick: 0,
                    numerator: 4,
                    denominator: 4,
                },
            )]
            .into_iter()
            .collect(),
        }),
        sections: [(
            "01K4F2C003".to_string(),
            Section {
                id: "01K4F2C003".to_string(),
                provenance: prov(Author::Model, Some("call_01C")),
                version: 1,
                name: "Chorus".to_string(),
                start_tick: 61_440,
                end_tick: 92_160,
            },
        )]
        .into_iter()
        .collect(),
        markers: BTreeMap::new(),
        tracks: [(bass.id.clone(), bass), (master.id.clone(), master)].into_iter().collect(),
        clips: [(clip.id.clone(), clip)].into_iter().collect(),
        automation: [(automation.id.clone(), automation)].into_iter().collect(),
        generators: [(
            "01K4F2G001".to_string(),
            Generator {
                id: "01K4F2G001".to_string(),
                provenance: prov(Author::Model, Some("call_01D")),
                version: 1,
                kind: GeneratorKind::Python as i32,
                source: "def generate(ctx):\n    return pattern(ctx.harmony)\n".to_string(),
                // Larger than 2^53: proves the JSON string encoding of 64-bit integers
                // survives every language's parser.
                seed: 9_007_199_254_740_993,
                toolchain_version: "0.4.1".to_string(),
                target: Some(generator::Target::ClipId("01K4F2QN8B".to_string())),
                params: BTreeMap::new(),
            },
        )]
        .into_iter()
        .collect(),
        render_target: Some(RenderTarget {
            kind: RenderKind::Master as i32,
            sample_rate: 48_000,
            bit_depth: 24,
            dither: false,
        }),
    }
}

/// The operations for the canonical bar-17 edit, as an RFC 6902 array.
///
/// `serde_json::Map` is a `BTreeMap` by default, so object keys sort; integers stay
/// integers. Both properties are why ops are authored as canonical JSON rather than
/// modelled in protobuf (ADR 0002 §11).
fn ops_json() -> serde_json::Value {
    serde_json::json!([
        {
            "op": "replace",
            "path": "/clips/01K4F2QN8B/note_clip/notes/01K4F2N001/pitch",
            "value": 43
        },
        {
            // 64-bit fields are JSON strings in the canonical form, so an op that writes
            // one must carry a string or the value is silently rounded by some parsers.
            "op": "replace",
            "path": "/generators/01K4F2G001/seed",
            "value": "9007199254740993"
        },
        { "op": "remove", "path": "/clips/01K4F2QN8B/note_clip/notes/01K4F2N002" }
    ])
}

fn ops_text() -> String {
    let mut s = serde_json::to_string_pretty(&ops_json()).unwrap();
    s.push('\n');
    s
}

/// Applies an RFC 6902 op the way any patch library would: resolve the JSON Pointer, then
/// assign or delete. Deliberately not a dependency — the point is to behave like an
/// arbitrary third-party implementation, not like ours.
fn apply_ops(doc: &mut serde_json::Value, ops: &serde_json::Value) {
    for op in ops.as_array().unwrap() {
        let path = op["path"].as_str().unwrap();
        match op["op"].as_str().unwrap() {
            "replace" | "add" => {
                *doc.pointer_mut(path).expect("path must resolve") = op["value"].clone();
            }
            "remove" => {
                let (parent, key) = path.rsplit_once('/').unwrap();
                doc.pointer_mut(parent)
                    .and_then(|p| p.as_object_mut())
                    .expect("parent must be an object")
                    .remove(key)
                    .expect("key must exist");
            }
            other => panic!("unhandled op {other}"),
        }
    }
}

fn read_or_write(path: &str, actual: &str) -> String {
    if std::env::var("UPDATE_FIXTURES").is_ok() {
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
        std::fs::write(path, actual).unwrap();
    }
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e}\nrun with UPDATE_FIXTURES=1 to create it"))
}

fn canonical(value: &impl serde::Serialize) -> String {
    let mut s = serde_json::to_string_pretty(value).unwrap();
    s.push('\n');
    s
}

#[test]
fn song_matches_canonical_fixture() {
    let song = build();
    let actual = canonical(&song);
    assert_eq!(actual, read_or_write(SONG_FIXTURE, &actual));
}

#[test]
fn json_round_trips() {
    let song = build();
    let parsed: Song = serde_json::from_str(&canonical(&song)).unwrap();
    assert_eq!(song, parsed);
}

#[test]
fn binary_round_trips() {
    use prost::Message;
    let song = build();
    let bytes = song.encode_to_vec();
    assert_eq!(song, Song::decode(&bytes[..]).unwrap());
}

#[test]
fn defaults_are_emitted_and_unset_optionals_are_absent() {
    let json = canonical(&build());
    // A no-presence field at its zero value is still written, so RFC 6902 `replace` and
    // `test` always have a path to address (ADR 0002 §4).
    assert!(json.contains("\"microtonal_cents\": 0.0"));
    assert!(json.contains("\"expression\": {}"));
    assert!(json.contains("\"markers\": {}"));
    assert!(json.contains("\"allow_overlap\": false"));
    // An unset `optional` is genuinely absent, so `add`/`remove` are meaningful for it.
    assert!(!json.contains("loop_length_ticks"));
    assert!(!json.contains("prompt_id"));
    // 64-bit integers are JSON strings; ticks are not.
    assert!(json.contains("\"seed\": \"9007199254740993\""));
    assert!(json.contains("\"start_tick\": 61440"));
}

#[test]
fn camel_case_is_accepted_and_unknown_fields_are_rejected() {
    let camel = r#"{"id": "x", "version": 1, "trackId": "t",
                    "startTick": 61440, "lengthTicks": 480}"#;
    let clip: Clip = serde_json::from_str(camel).expect("camelCase input must parse");
    assert_eq!(clip.start_tick, 61_440);

    // Anything not in the schema is an error rather than silently dropped: a typo in a
    // hand-edited project file must not be mistaken for a default.
    let unknown = r#"{"id": "x", "tempo_bpm": 120}"#;
    assert!(serde_json::from_str::<Song>(unknown).is_err(), "unknown fields must be rejected");
    let wrong_message = r#"{"id": "x", "schemaVersion": 1}"#;
    assert!(serde_json::from_str::<Clip>(wrong_message).is_err(), "fields of another message must be rejected");
}

#[test]
fn refs_round_trip() {
    let refs = Refs {
        head: "main".to_string(),
        refs: [
            ("main".to_string(), "01K4F31M2T".to_string()),
            ("try-darker-chorus".to_string(), "01K4F2Z10Q".to_string()),
        ]
        .into_iter()
        .collect(),
    };
    let parsed: Refs = serde_json::from_str(&canonical(&refs)).unwrap();
    assert_eq!(refs, parsed);
}

#[test]
fn map_keys_serialise_sorted_whatever_order_they_arrive_in() {
    // Parsed from text with keys out of order, so this fails if the generated maps ever
    // stop being ordered — unlike asserting that a BTreeMap is sorted, which cannot fail.
    let refs: Refs =
        serde_json::from_str(r#"{"head": "main", "refs": {"zulu": "1", "alpha": "2"}}"#).unwrap();
    let json = canonical(&refs);
    assert!(json.find("alpha").unwrap() < json.find("zulu").unwrap());
}

#[test]
fn a_patch_applied_by_an_ordinary_library_leaves_a_document_core_can_read() {
    // The case that matters: `core` writes a patch, something else applies it, `core` reads
    // the result. Modelling op values in protobuf made this fail — `google.protobuf.Value`
    // is double-valued, so a pitch came back as `43.0` and the int32 field rejected it.
    let mut doc: serde_json::Value = serde_json::from_str(&canonical(&build())).unwrap();
    apply_ops(&mut doc, &ops_json());

    let patched: Song = serde_json::from_value(doc)
        .expect("core must be able to read the document its own patch produced");

    let clip::Content::NoteClip(notes) =
        patched.clips["01K4F2QN8B"].content.clone().unwrap()
    else {
        unreachable!()
    };
    assert_eq!(notes.notes.len(), 1, "the removed note is gone");
    assert_eq!(notes.notes["01K4F2N001"].pitch, 43);
    assert_eq!(patched.generators["01K4F2G001"].seed, 9_007_199_254_740_993);
}

#[test]
fn an_op_writing_a_float_into_an_integer_field_is_rejected_rather_than_truncated() {
    let mut doc: serde_json::Value = serde_json::from_str(&canonical(&build())).unwrap();
    let path = "/clips/01K4F2QN8B/note_clip/notes/01K4F2N001/pitch";
    *doc.pointer_mut(path).unwrap() = serde_json::json!(43.5);
    assert!(
        serde_json::from_value::<Song>(doc).is_err(),
        "a non-integral pitch must be an error, not a silent truncation"
    );
}

#[test]
fn patch_entry_carries_its_ops_verbatim_over_the_wire() {
    use prost::Message;
    let entry = PatchEntry {
        id: "01K4F31M2T".to_string(),
        parents: vec!["01K4F2QN8B".to_string()],
        tool: "set_notes".to_string(),
        ops: ops_text().into_bytes(),
        provenance: prov(Author::Model, Some("call_01B")),
        schema_version: 1,
    };
    let decoded = PatchEntry::decode(&entry.encode_to_vec()[..]).unwrap();
    assert_eq!(decoded, entry);
    // Byte-identical to what goes in patches/*.json: the wire carries the file, not a
    // re-encoding of it.
    assert_eq!(String::from_utf8(decoded.ops).unwrap(), ops_text());
}

#[test]
fn doubles_survive_a_canonical_round_trip_bit_for_bit() {
    // Fails without serde_json's `float_roundtrip`: its default parser is off by one ULP on
    // roughly a third of doubles, so load-then-save would not be a fixed point (§11).
    // Seeded LCG, so this is deterministic and depends on no wall clock.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut checked = 0;
    for _ in 0..40_000 {
        state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let v = f64::from_bits(state);
        if !v.is_finite() {
            continue;
        }
        let mix = Mix { gain_db: v, pan: 0.0, mute: false, solo: false };
        let text = serde_json::to_string(&mix).unwrap();
        let back: Mix = serde_json::from_str(&text).unwrap();
        assert_eq!(back.gain_db.to_bits(), v.to_bits(), "{v:e} did not survive as {text}");
        checked += 1;
    }
    assert!(checked > 30_000, "expected a meaningful sample, got {checked}");
}

#[test]
fn non_finite_doubles_do_not_survive_and_so_the_writer_must_reject_them() {
    // Pins the reason ADR 0002 lists "all doubles finite" as a rule the *writer* enforces,
    // not just the validator: one NaN from a plugin param would otherwise write a file that
    // cannot be read back.
    let mix = Mix { gain_db: f64::NAN, ..Default::default() };
    let text = serde_json::to_string(&mix).unwrap();
    assert!(text.contains("null"), "non-finite doubles serialise as null: {text}");
    assert!(serde_json::from_str::<Mix>(&text).is_err(), "and null does not read back");
}

#[test]
fn timestamps_parse_in_both_the_z_and_offset_forms() {
    // pbjson writes `+00:00`; proto3's canonical JSON is `Z`, and TS and Python both write
    // `Z`. Pinning that Rust reads both keeps M0.2 free to normalise the written form.
    let z: Provenance =
        serde_json::from_str(r#"{"author": "AUTHOR_HUMAN", "created_at": "2026-09-02T00:00:00Z"}"#)
            .unwrap();
    let offset: Provenance = serde_json::from_str(
        r#"{"author": "AUTHOR_HUMAN", "created_at": "2026-09-02T00:00:00+00:00"}"#,
    )
    .unwrap();
    assert_eq!(z, offset);
    assert_eq!(z.created_at.unwrap().seconds, CREATED_AT);
}
