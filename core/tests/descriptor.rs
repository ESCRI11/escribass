//! Tool schemas derived from the protobuf descriptor (ADR 0006 §6).
//!
//! These are the tests for a piece of code whose failures are all silent. A wrong schema does
//! not crash anything: the tool still works, the model just never learns how to call it
//! properly, or learns a field that is not there. So the assertions are about *correspondence*
//! with the proto rather than about any particular shape being pretty.

use escribass_core::{tool_names, tool_schemas, ToolSchema};
use escribass_proto::DESCRIPTOR;
use prost::Message;
use serde_json::{json, Value};

fn tools() -> Vec<ToolSchema> {
    tool_schemas(DESCRIPTOR).expect("the descriptor decodes")
}

fn tool(name: &str) -> ToolSchema {
    tools().into_iter().find(|t| t.name == name).unwrap_or_else(|| panic!("no tool {name}"))
}

fn property(tool: &ToolSchema, name: &str) -> Value {
    tool.input_schema["properties"][name].clone()
}

// ---- correspondence with the service ----

#[test]
fn every_rpc_in_the_service_becomes_a_tool() {
    // The count is asserted in `escribass-proto`'s own tests against the `.proto` text; here
    // the point is that the descriptor and the service agree, so an RPC cannot be added
    // without appearing.
    let names = tool_names(DESCRIPTOR).unwrap();
    assert_eq!(names.len(), 25);
    for expected in ["apply_patch", "get_song", "get_song_at", "get_history", "merge_branch",
                     "add_asset", "render_export", "render_preview", "undo", "redo"] {
        assert!(names.contains(&expected.to_string()), "{expected} missing from {names:?}");
    }
}

#[test]
fn the_engine_service_is_not_a_tool() {
    // `Render` and `Preview` (proto/render.proto) share the descriptor with `SongTools` and are
    // the engine's boundary, not a model's. Advertised, either would hand a model a `RenderPlan`
    // to fill in — the one message ADR 0007 §1 keeps out of every process but core. A model
    // previews through `render_preview`, which compiles the plan itself.
    let names = tool_names(DESCRIPTOR).unwrap();
    assert!(!names.contains(&"render".to_string()), "{names:?}");
    assert!(!names.contains(&"preview".to_string()), "{names:?}");
}

#[test]
fn names_are_the_snake_case_of_the_rpc() {
    let names = tool_names(DESCRIPTOR).unwrap();
    assert!(names.contains(&"set_track_instrument".to_string()), "{names:?}");
    assert!(!names.iter().any(|n| n.chars().any(|c| c.is_ascii_uppercase())), "{names:?}");
}

#[test]
fn every_tool_says_what_it_does() {
    // A tool with no description is one a model has to guess at. The text comes from the
    // proto's own comments, so this fails the moment an RPC is added without documenting it.
    for tool in tools() {
        assert!(!tool.description.is_empty(), "{} has no description", tool.name);
    }
}

#[test]
fn every_tool_takes_an_object_with_no_extra_properties() {
    for tool in tools() {
        assert_eq!(tool.input_schema["type"], json!("object"), "{}", tool.name);
        assert_eq!(tool.input_schema["additionalProperties"], json!(false), "{}", tool.name);
        assert!(tool.input_schema["properties"].is_object(), "{}", tool.name);
    }
}

#[test]
fn every_mutating_tool_advertises_dry_run() {
    // §5 gives every tool `dry_run`. A model that cannot see the field cannot preview.
    let reads = ["get_song", "get_song_at", "get_history"];
    for tool in tools().into_iter().filter(|t| !reads.contains(&t.name.as_str())) {
        assert_eq!(property(&tool, "dry_run"), json!({"type": "boolean"}), "{}", tool.name);
    }
}

// ---- the proto3 JSON mapping ----

#[test]
fn property_names_are_proto_names_not_camel_case() {
    // ADR 0002 §4 fixes snake_case for every document in this system. Advertising `trackId`
    // would teach a model to write documents that do not match the ones it reads.
    let tool = tool("add_effect");
    assert!(tool.input_schema["properties"].get("track_id").is_some());
    assert!(tool.input_schema["properties"].get("trackId").is_none());
}

#[test]
fn scalars_map_the_way_proto3_json_says() {
    let clip = tool("add_clip");
    assert_eq!(property(&clip, "start_tick")["type"], json!("integer"));
    assert_eq!(property(&clip, "dry_run")["type"], json!("boolean"));
    assert_eq!(property(&clip, "track_id")["type"], json!("string"));
    assert_eq!(property(&tool("set_param"), "value")["type"], json!("number"));
}

#[test]
fn bytes_are_a_base64_string() {
    let patch = property(&tool("apply_patch"), "patch");
    assert_eq!(patch["type"], json!("string"));
    assert_eq!(patch["contentEncoding"], json!("base64"));
}

#[test]
fn an_enum_offers_its_names() {
    let kind = property(&tool("add_track"), "kind");
    assert_eq!(kind["type"], json!("string"));
    let values: Vec<&str> = kind["enum"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert!(values.contains(&"TRACK_KIND_INSTRUMENT"), "{values:?}");
    assert!(values.contains(&"TRACK_KIND_UNSPECIFIED"), "{values:?}");
}

#[test]
fn a_map_field_is_an_object_with_typed_values() {
    // `map<string, Note>` is an object in proto3 JSON, not an array of pairs. It is also a
    // `repeated` field of a synthetic entry message underneath, so getting this wrong would
    // advertise an array and every call would fail.
    let notes = property(&tool("set_notes"), "notes");
    assert_eq!(notes["type"], json!("object"));
    let note = &notes["additionalProperties"];
    assert_eq!(note["type"], json!("object"));
    assert_eq!(note["properties"]["pitch"]["type"], json!("integer"));
}

#[test]
fn a_repeated_field_is_an_array() {
    let ids = property(&tool("transpose"), "note_ids");
    assert_eq!(ids["type"], json!("array"));
    assert_eq!(ids["items"]["type"], json!("string"));
}

#[test]
fn an_embedded_entity_does_not_ask_for_the_fields_core_overwrites() {
    // ADR 0006 §4: core sets `id`, `provenance` and `version` on the way in, and "a caller
    // cannot set them and is not asked to". Left in the schema, a model fills them in on every
    // call — inventing ids, writing provenance, choosing versions — and every one is
    // discarded. An advertised argument that is ignored is worse than an absent one.
    let note = property(&tool("set_notes"), "notes")["additionalProperties"].clone();
    let properties = note["properties"].as_object().expect("a note is an object");

    for owned in ["id", "provenance", "version"] {
        assert!(properties.get(owned).is_none(), "`{owned}` is still advertised: {properties:?}");
    }
    // The musical fields a caller *does* choose are still there.
    assert_eq!(properties["pitch"]["type"], json!("integer"));
    assert_eq!(properties["start_tick"]["type"], json!("integer"));
}

#[test]
fn a_request_field_that_is_not_overwritten_has_a_schema_property() {
    // ADR 0006 §6 claims this test exists; it did not. Without it a field added to a request
    // reaches the wire and never reaches a model, which is the drift the whole module is for.
    let set = escribass_proto::prost_types::FileDescriptorSet::decode(DESCRIPTOR)
        .expect("the descriptor decodes");
    let requests: std::collections::BTreeMap<&str, &escribass_proto::prost_types::DescriptorProto> =
        set.file
            .iter()
            .flat_map(|f| f.message_type.iter())
            .map(|m| (m.name(), m))
            .collect();

    for tool in tools() {
        let name = format!(
            "{}Request",
            tool.name.split('_').map(|part| {
                let mut c = part.chars();
                c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
            }).collect::<String>()
        );
        let Some(message) = requests.get(name.as_str()) else { panic!("no message {name}") };
        let properties = tool.input_schema["properties"].as_object().expect("an object");
        for field in &message.field {
            assert!(
                properties.contains_key(field.name()),
                "`{}` has no schema property for `{}`",
                tool.name,
                field.name()
            );
        }
    }
}

#[test]
fn a_real_oneof_says_to_set_exactly_one() {
    // Proto3 JSON has no marker for a oneof, so the constraint is said where a model reads it.
    let content = property(&tool("add_clip"), "note_clip");
    let description = content["description"].as_str().unwrap();
    assert!(description.contains("(content)"), "{description}");
    assert!(description.contains("audio_clip"), "{description}");
}

#[test]
fn a_proto3_optional_field_is_not_advertised_as_a_choice() {
    // `optional uint32 index` compiles to a *synthetic* one-member oneof named `_index`.
    // Reading that as a real oneof would tell a model `index` is a choice between
    // alternatives, when it is an ordinary optional field.
    let index = property(&tool("add_effect"), "index");
    assert_eq!(index["type"], json!("integer"));
    let description = index["description"].as_str().unwrap_or_default();
    assert!(!description.contains("exactly one"), "{description}");
}

#[test]
fn a_nested_message_is_expanded_not_left_opaque() {
    // The schema is what a model reads to build a `DeviceRef`; `{"type": "object"}` tells it
    // nothing and every call would be a guess.
    let device = property(&tool("set_track_instrument"), "ref");
    assert_eq!(device["properties"]["plugin"]["properties"]["plugin_id"]["type"], json!("string"));
}

// ---- stability ----

#[test]
fn the_schemas_are_the_same_every_time() {
    // §11: the tool list is part of what a session shows a model, so it has to be stable
    // across processes, not just within one.
    assert_eq!(tool_schemas(DESCRIPTOR).unwrap(), tool_schemas(DESCRIPTOR).unwrap());
}

#[test]
fn a_descriptor_that_is_not_one_is_an_error_not_a_panic() {
    assert!(tool_schemas(b"not a descriptor set at all").is_err());
}
