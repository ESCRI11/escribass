//! Round-trip and canonical-JSON tests for the generated model types.
//!
//! There is no validator yet (M0.2), so what these prove is narrow and deliberate: that the
//! generated types express §4, that the canonical JSON of ADR 0002 §4 is what the serde
//! impls actually produce, and that Rust, TypeScript and Python all read the same bytes.
//!
//! The fixture is written *by this test* from generated types (`UPDATE_FIXTURES=1`), never
//! by hand — the nearest thing to CLAUDE.md #2 available until M0.4 can drive fixtures
//! through the tool API.

use escribass_schema::history::{Op, PatchEntry, Refs};
use escribass_schema::pbjson_types;
use escribass_schema::song::*;
use std::collections::BTreeMap;

const SONG_FIXTURE: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests/fixtures/song/minimal.json");
const PATCH_FIXTURE: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests/fixtures/history/minimal.json");

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
        tempo_map: Some(TempoMap { events: vec![TempoEvent { tick: 0, bpm: 92.0 }] }),
        time_signature_map: Some(TimeSignatureMap {
            events: vec![TimeSignatureEvent { tick: 0, numerator: 4, denominator: 4 }],
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

fn minimal_patch_entry() -> PatchEntry {
    PatchEntry {
        id: "01K4F31M2T".to_string(),
        parents: vec!["01K4F2QN8B".to_string()],
        tool: "set_notes".to_string(),
        ops: vec![
            Op {
                op: "replace".to_string(),
                path: "/clips/01K4F2QN8B/note_clip/notes/01K4F2N001/pitch".to_string(),
                from: None,
                value: Some(pbjson_types::Value {
                    kind: Some(pbjson_types::value::Kind::NumberValue(43.0)),
                }),
            },
            Op {
                op: "remove".to_string(),
                path: "/clips/01K4F2QN8B/note_clip/notes/01K4F2N002".to_string(),
                from: None,
                value: None,
            },
        ],
        provenance: prov(Author::Model, Some("call_01B")),
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
fn patch_entry_matches_canonical_fixture() {
    let entry = minimal_patch_entry();
    let actual = canonical(&entry);
    assert_eq!(actual, read_or_write(PATCH_FIXTURE, &actual));
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
fn map_keys_are_sorted_regardless_of_insertion_order() {
    let mut reversed = build();
    let notes: Vec<_> = match reversed.clips.get("01K4F2QN8B").unwrap().content.clone().unwrap() {
        clip::Content::NoteClip(n) => n.notes.into_iter().collect(),
        _ => unreachable!(),
    };
    let mut backwards = BTreeMap::new();
    for (k, v) in notes.into_iter().rev() {
        backwards.insert(k, v);
    }
    reversed.clips.get_mut("01K4F2QN8B").unwrap().content =
        Some(clip::Content::NoteClip(NoteClip { notes: backwards }));

    let json = canonical(&reversed);
    let first = json.find("01K4F2N001").unwrap();
    let second = json.find("01K4F2N002").unwrap();
    assert!(first < second, "map keys must serialise in sorted order");
    assert_eq!(json, canonical(&build()));
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
