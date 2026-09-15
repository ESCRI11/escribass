//! The wire contracts of ADR 0006 and ADR 0007, asserted where they can silently stop holding.
//!
//! There is no round-trip fixture here: `schema/tests/roundtrip.rs` covers the model, and
//! this crate adds no model types. What it covers is the handful of things about *these*
//! files that a future change could break without any other test noticing.

use escribass_proto::prost_types::{DescriptorProto, FileDescriptorSet};
use escribass_proto::render::{PlanClip, PlanNotes, PlanTrack, RenderPlan};
use escribass_proto::tools::{AddClipRequest, SongResponse, ToolResult, Violation};
use escribass_proto::DESCRIPTOR;
use escribass_schema::song::{Mix, Note, NoteClip, Song};
use prost::Message;

/// The plan's own package, fully qualified as the descriptor names it.
const RENDER: &str = ".escribass.render.v1.";

/// The canonical RFC 6902 text of one operation, as `ops_text` in `core` produces it.
const OPS: &str = "[\n  {\n    \"op\": \"replace\",\n    \"path\": \"/tracks/x/mix/gain_db\",\n    \"value\": -3.0\n  }\n]\n";

/// ADR 0006 §4: a request that carries a whole entity carries *the* entity, not a copy.
///
/// This is a compile-time assertion wearing a test's clothes. If `extern_path` is ever dropped
/// from `proto/buf.gen.yaml`, prost generates its own `Song` and `NoteClip` into this crate —
/// a second representation of song state (CLAUDE.md #1) that compiles perfectly well on its
/// own. It stops compiling here, because these are the model's types by name.
#[test]
fn requests_and_responses_carry_the_model_types_themselves() {
    let song = Song { id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(), ..Default::default() };
    let response = SongResponse { song: Some(song.clone()) };
    assert_eq!(response.song.as_ref().map(|s| s.id.as_str()), Some(song.id.as_str()));

    let request = AddClipRequest {
        track_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
        content: Some(escribass_proto::tools::add_clip_request::Content::NoteClip(
            NoteClip::default(),
        )),
        ..Default::default()
    };
    assert!(request.content.is_some());
}

/// ADR 0002 §11: the wire carries the same canonical JSON document the disk does. `patch` is
/// `bytes` so that RFC 6902 values cross unmodelled — modelling them in protobuf is what made
/// the patch log nondeterministic in M0.1.
#[test]
fn patch_text_crosses_the_wire_byte_for_byte() {
    let sent = ToolResult {
        valid: true,
        patch: OPS.as_bytes().to_vec(),
        summary: "1 op".to_string(),
        entry_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
        errors: vec![],
    };
    let received = ToolResult::decode(&*sent.encode_to_vec()).expect("a ToolResult decodes");
    assert_eq!(received.patch, sent.patch);
    assert_eq!(String::from_utf8(received.patch).unwrap(), OPS);
}

/// The trap ADR 0006 §6 names, pinned so it stays known.
///
/// `patch` is `bytes`, and proto3 JSON encodes bytes as base64 — so the *generated* serde impl
/// is correct and unusable for MCP, where the patch must arrive as a JSON array a model can
/// read. Building an MCP payload with `serde_json::to_value(&result)` produces a file that
/// parses, round-trips, and passes any test that does not look at it. The MCP layer builds its
/// object field by field instead; this test exists so that decision keeps its reason.
#[test]
fn the_generated_serde_impl_base64_encodes_patch() {
    let result = ToolResult { patch: OPS.as_bytes().to_vec(), ..Default::default() };
    let json = serde_json::to_value(&result).expect("a ToolResult serialises");
    let encoded = json["patch"].as_str().expect("`patch` serialises as a string, not an array");
    assert!(!encoded.starts_with('['), "base64, not RFC 6902 text: {encoded}");
    assert_ne!(encoded, OPS);
}

/// ADR 0006 §1: every mutating RPC returns the one result contract, and only an RPC that
/// produces no ops — a read, `AddAsset`, whose answer is an address, or `RenderExport`, whose
/// answer is the engine's — returns its own.
///
/// Read from the `.proto` rather than the generated code because the rule is about the service
/// definition, and because a new RPC added with its own response message would otherwise be
/// caught by nothing until a reviewer noticed.
#[test]
fn every_mutating_rpc_returns_the_shared_result() {
    const OWN_SHAPE: [(&str, &str); 6] = [
        ("GetSong", "SongResponse"),
        ("GetSongAt", "SongResponse"),
        ("GetHistory", "HistoryResponse"),
        ("AddAsset", "AssetResponse"),
        // Revised 2026-09-07, in M1 PR 10: a render records nothing, so `patch` and
        // `entry_id` would be permanently empty — and the hash it reports has nowhere to go
        // in `ToolResult` (song_tools.proto, `RenderResponse`; ADR 0006 §1, amended).
        ("RenderExport", "RenderResponse"),
        // M2 PR 10, for the same reason: a preview records nothing, and where the transport is
        // has nowhere to go in `ToolResult` (song_tools.proto, `PreviewResponse`).
        ("RenderPreview", "PreviewResponse"),
    ];
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/song_tools.proto"))
        .expect("song_tools.proto");

    let mut seen = 0;
    for line in text.lines().map(str::trim).filter(|l| l.starts_with("rpc ")) {
        let name = line["rpc ".len()..].split('(').next().unwrap().trim();
        let returns = line.rsplit("returns").next().unwrap().trim_matches(|c: char| {
            c.is_whitespace() || c == '(' || c == ')' || c == ';'
        });
        match OWN_SHAPE.iter().find(|(rpc, _)| *rpc == name) {
            Some((_, own)) => assert_eq!(returns, *own, "{name}"),
            None => assert_eq!(returns, "ToolResult", "{name} must return the shared contract"),
        }
        seen += 1;
    }
    assert_eq!(seen, 25, "every rpc in the service is checked");
}

// ---- render.proto (ADR 0007 §2) ----

/// ADR 0007 §2: a leaf message crosses into the plan *by value* — it is the model's type, not
/// a plan-local copy of it.
///
/// A compile-time assertion, like the one above for requests. Drop `extern_path` for
/// `render.proto` in `proto/buf.gen.yaml` and prost generates its own `Note` and `Mix` into
/// this crate: the mirrored shape ADR 0006 §4 forbids, which compiles fine on its own and
/// stops compiling here, because these are the model's types by name.
#[test]
fn the_plan_carries_the_model_types_themselves() {
    let plan = RenderPlan {
        tracks: vec![PlanTrack {
            mix: Some(Mix { gain_db: -3.0, pan: 0.0, mute: false, solo: false }),
            clips: vec![PlanClip {
                start_tick: 0,
                length_ticks: 960,
                content: Some(escribass_proto::render::plan_clip::Content::Notes(PlanNotes {
                    notes: vec![Note { pitch: 60, ..Default::default() }],
                })),
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let decoded = RenderPlan::decode(&*plan.encode_to_vec()).expect("a plan decodes");
    assert_eq!(decoded, plan);
}

/// ADR 0007 §2: everything structural in the plan is `repeated`, never a map. The model keys
/// its collections by id so patch paths survive concurrent inserts; a plan is never patched,
/// and `repeated` carries the one thing the model deliberately does not store — order. A map
/// reaching the engine would hand it a collection to derive order from, which is what ADR
/// 0007 §1 keeps on core's side.
///
/// Walked from the descriptor rather than the generated code, because a `map<...>` is a
/// synthetic nested message with `map_entry` set, and that is the one place it cannot hide.
///
/// Seeded from what the engine is *handed* — every RPC's argument — rather than from
/// `RenderPlan` by name. Revised 2026-09-08 in M2 PR 3: `PreviewCommand` carries a plan
/// (ADR 0013 §2) and is not called `Plan*`, and the coverage assertion at the end of this test
/// only required `Plan*` messages to have been reached, so a whole preview command tree would
/// have been added outside the walk without anything saying so. Reading the seeds and the
/// exemptions off the services means a later RPC joins the walk by existing.
#[test]
fn nothing_reachable_from_the_plan_is_a_map() {
    let set = FileDescriptorSet::decode(DESCRIPTOR).expect("the descriptor decodes");
    let mut messages = std::collections::BTreeMap::new();
    for file in &set.file {
        for message in &file.message_type {
            index(&format!(".{}.{}", file.package(), message.name()), message, &mut messages);
        }
    }

    let methods = || set.file.iter().flat_map(|f| &f.service).flat_map(|s| &s.method);
    let mut pending: Vec<String> = methods()
        .map(|m| m.input_type().to_string())
        .filter(|name| name.starts_with(RENDER))
        .collect();
    assert_eq!(pending.len(), 2, "the engine serves Render and Preview (ADR 0013 §3)");
    let mut visited = std::collections::BTreeSet::new();
    while let Some(name) = pending.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        let message = messages.get(&name).unwrap_or_else(|| panic!("{name} is not in the descriptor"));
        for field in &message.field {
            if field.r#type() == escribass_proto::prost_types::field_descriptor_proto::Type::Message {
                let target = messages.get(field.type_name()).expect("every message type resolves");
                assert!(
                    !target.options.as_ref().is_some_and(|o| o.map_entry()),
                    "{name}.{} is a map; a plan-local message is repeated, never keyed \
                     (ADR 0007 §2)",
                    field.name()
                );
                // The walk stops at the model's own messages. Rule 1 crosses them **whole**,
                // so what is inside one is the model's business, not the plan's: Note carries
                // a map<string, double> of per-note expression, which is a keyed set of
                // scalars rather than the entity collection ADR 0001 §3's rule is about.
                // Descending would assert the opposite of rule 1 — that a leaf is reshaped on
                // the way in — which is the mirrored shape ADR 0006 §4 forbids.
                if field.type_name().starts_with(RENDER) {
                    pending.push(field.type_name().to_string());
                }
            }
        }
    }

    // Every message of this package, so a new one cannot be added outside the walk. The
    // exceptions are what travels the other way — rule 2 is about a plan, and `RenderResult`
    // carries a `map<string, string>` of build commits on purpose (ADR 0008 §5) — and the
    // synthetic entry type that map generates.
    let returned: std::collections::BTreeSet<&str> =
        methods().map(|m| m.output_type()).filter(|n| n.starts_with(RENDER)).collect();
    for (name, message) in messages.iter().filter(|(n, _)| n.starts_with(RENDER)) {
        if returned.contains(name.as_str())
            || message.options.as_ref().is_some_and(|o| o.map_entry())
        {
            continue;
        }
        assert!(visited.contains(name), "{name} is plan-local and the walk never reached it");
    }
}

fn index<'a>(
    name: &str,
    message: &'a DescriptorProto,
    into: &mut std::collections::BTreeMap<String, &'a DescriptorProto>,
) {
    for nested in &message.nested_type {
        index(&format!("{name}.{}", nested.name()), nested, into);
    }
    into.insert(name.to_string(), message);
}

/// The error shape core already returns, transported rather than redefined (ADR 0006 §2).
#[test]
fn a_violation_carries_path_rule_and_message() {
    let sent = Violation {
        path: "/clips/x/note_clip/notes/y/pitch".to_string(),
        rule: "pitch_out_of_range".to_string(),
        message: "pitch is MIDI 0-127".to_string(),
    };
    assert_eq!(Violation::decode(&*sent.encode_to_vec()).unwrap(), sent);
}
