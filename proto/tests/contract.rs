//! The wire contract of ADR 0006, asserted where it can silently stop holding.
//!
//! There is no round-trip fixture here: `schema/tests/roundtrip.rs` covers the model, and
//! this module adds no model types. What it covers is the three things about *this* file that
//! a future change could break without any other test noticing.

use escribass_proto::tools::{AddClipRequest, SongResponse, ToolResult, Violation};
use escribass_schema::song::{NoteClip, Song};
use prost::Message;

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

/// ADR 0006 §1: every mutating RPC returns the one result contract, and only the reads differ.
///
/// Read from the `.proto` rather than the generated code because the rule is about the service
/// definition, and because a new RPC added with its own response message would otherwise be
/// caught by nothing until a reviewer noticed.
#[test]
fn every_mutating_rpc_returns_the_shared_result() {
    const READS: [&str; 3] = ["GetSong", "GetSongAt", "GetHistory"];
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/song_tools.proto"))
        .expect("song_tools.proto");

    let mut seen = 0;
    for line in text.lines().map(str::trim).filter(|l| l.starts_with("rpc ")) {
        let name = line["rpc ".len()..].split('(').next().unwrap().trim();
        let returns = line.rsplit("returns").next().unwrap().trim_matches(|c: char| {
            c.is_whitespace() || c == '(' || c == ')' || c == ';'
        });
        if READS.contains(&name) {
            assert_eq!(returns, if name == "GetHistory" { "HistoryResponse" } else { "SongResponse" });
        } else {
            assert_eq!(returns, "ToolResult", "{name} must return the shared contract");
        }
        seen += 1;
    }
    assert_eq!(seen, 20, "every rpc in the service is checked");
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
