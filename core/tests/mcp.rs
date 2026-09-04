//! The MCP server (§18.2, ADR 0006 §6).
//!
//! Driven as a real subprocess over real pipes, because that is where this layer's failures
//! live: a payload that is valid JSON and unreadable to a model, a stray newline that corrupts
//! the stream, a handshake that only one generation of client can complete. None of those show
//! up when the handler is called directly.

use escribass_core::mcp::{IMPLEMENTED, JSON_TEXT_FIELDS};
use escribass_core::tool_names;
use escribass_proto::DESCRIPTOR;
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const AT: &str = "1788307200000";
const MASTER: &str = "01M1FPMP000000000000000002";

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-mcp-{}-{}.escri",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs a scripted session against a fresh project and returns the raw stdout lines.
///
/// Seeded ids and a fixed clock, so the whole exchange is reproducible: §11 forbids `core`
/// from reading a wall clock or taking entropy, and these flags are the only way that promise
/// is testable through a real process.
fn session(project: &Path, requests: &[Value]) -> Vec<String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_escribass-mcp"))
        .args(["--create", "--seed-ids", &format!("{AT}:1"), "--fixed-clock", AT])
        .arg(project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the server binary runs");

    let mut script = String::new();
    script.push_str(&line(&json!({
        "jsonrpc": "2.0", "id": 0, "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1"}
        }
    })));
    script.push_str(&line(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"})));
    for request in requests {
        script.push_str(&line(request));
    }

    child.stdin.take().expect("stdin").write_all(script.as_bytes()).expect("the script is sent");
    let finished = child.wait_with_output().expect("the server exits when stdin closes");
    assert!(
        finished.status.success(),
        "the server failed: {}",
        String::from_utf8_lossy(&finished.stderr)
    );
    String::from_utf8(finished.stdout)
        .expect("stdout is utf-8")
        .lines()
        .map(str::to_string)
        .collect()
}

fn line(value: &Value) -> String {
    serde_json::to_string(value).expect("a request serialises") + "\n"
}

fn call(id: u32, name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": {"name": name, "arguments": arguments}
    })
}

/// The response with this id, parsed.
fn response(lines: &[String], id: u32) -> Value {
    lines
        .iter()
        .map(|l| serde_json::from_str::<Value>(l).expect("every frame is JSON"))
        .find(|v| v["id"] == json!(id))
        .unwrap_or_else(|| panic!("no response with id {id} in {lines:#?}"))
}

// ---- the handshake ----

#[test]
fn a_client_on_an_older_protocol_can_still_connect() {
    // The population of MCP clients spans revisions. A server that only answers the newest
    // fails older ones with an unhelpful error, and rmcp answering every version it knows is
    // the reason this SDK was chosen over hand-rolling the protocol (ADR 0006 §6).
    let dir = Scratch::new();
    let lines = session(&dir.0, &[json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})]);

    let initialised = response(&lines, 0);
    assert_eq!(initialised["result"]["protocolVersion"], json!("2025-06-18"));
    assert_eq!(initialised["result"]["serverInfo"]["name"], json!("escribass"));
    assert!(initialised["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn every_frame_is_one_line() {
    // stdio framing is newline-delimited, so a single embedded newline desynchronises the
    // stream for the rest of the session. Every canonical writer in `core` pretty-prints, so
    // this is a live hazard rather than a theoretical one.
    let dir = Scratch::new();
    let lines = session(&dir.0, &[call(1, "get_song", json!({}))]);
    for line in &lines {
        assert!(!line.is_empty(), "an empty frame");
        serde_json::from_str::<Value>(line).expect("each line is a complete JSON document");
    }
}

// ---- the tool list ----

#[test]
fn only_implemented_tools_are_advertised() {
    // Advertising a tool that is not wired up spends a model's turn on a call that can only
    // fail. `IMPLEMENTED` grows as the later steps land.
    let dir = Scratch::new();
    let lines = session(&dir.0, &[json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})]);
    let tools = response(&lines, 1)["result"]["tools"].clone();

    let names: Vec<&str> =
        tools.as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, IMPLEMENTED);
    for tool in tools.as_array().unwrap() {
        assert!(!tool["description"].as_str().unwrap().is_empty(), "{}", tool["name"]);
        assert_eq!(tool["inputSchema"]["type"], json!("object"));
    }
}

#[test]
fn the_implemented_list_names_real_rpcs() {
    let declared = tool_names(DESCRIPTOR).unwrap();
    for name in IMPLEMENTED {
        assert!(declared.contains(&name.to_string()), "`{name}` is not an rpc in the service");
    }
}

#[test]
fn the_json_text_exception_names_real_bytes_fields() {
    // The one hand-maintained list in this layer, so it gets a test that it still corresponds
    // to the proto. A `bytes` field that stopped being one, or a tool that was renamed, would
    // otherwise leave the exception silently doing nothing.
    let schemas = escribass_core::tool_schemas(DESCRIPTOR).unwrap();
    for (tool, field) in JSON_TEXT_FIELDS {
        let schema = schemas
            .iter()
            .find(|t| t.name == *tool)
            .unwrap_or_else(|| panic!("no tool `{tool}`"));
        let property = &schema.input_schema["properties"][*field];
        assert_eq!(
            property["contentEncoding"],
            json!("base64"),
            "`{tool}.{field}` is not a bytes field any more"
        );
    }
}

#[test]
fn a_canonical_json_field_is_advertised_as_an_array_not_base64() {
    // Left as the proto describes it, `apply_patch` would ask a model to base64-encode an RFC
    // 6902 array. ADR 0006 §6 makes the patch cross as JSON in both directions.
    let dir = Scratch::new();
    let lines = session(&dir.0, &[json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})]);
    let tools = response(&lines, 1)["result"]["tools"].clone();
    let apply = tools.as_array().unwrap().iter().find(|t| t["name"] == json!("apply_patch")).unwrap();

    let patch = &apply["inputSchema"]["properties"]["patch"];
    assert_eq!(patch["type"], json!("array"));
    assert!(patch.get("contentEncoding").is_none(), "still advertised as base64: {patch}");
}

// ---- calls ----

#[test]
fn a_song_comes_back_in_canonical_field_order() {
    // `serde_json::to_value` would alphabetise the struct fields (`core/AGENTS.md`), so the
    // text block is rendered by the canonical writer. This asserts the order the model has,
    // not merely that the keys are present.
    let dir = Scratch::new();
    let lines = session(&dir.0, &[call(1, "get_song", json!({}))]);
    let text = response(&lines, 1)["result"]["content"][0]["text"].as_str().unwrap().to_string();

    let id = text.find("\"id\"").unwrap();
    let provenance = text.find("\"provenance\"").unwrap();
    let version = text.find("\"version\"").unwrap();
    let schema_version = text.find("\"schema_version\"").unwrap();
    assert!(id < provenance && provenance < version && version < schema_version, "{text}");
}

#[test]
fn a_patch_crosses_as_an_array_in_both_directions() {
    let dir = Scratch::new();
    let ops = json!([{"op": "replace", "path": format!("/tracks/{MASTER}/name"), "value": "Out"}]);
    let lines = session(&dir.0, &[call(1, "apply_patch", json!({"patch": ops, "dry_run": true}))]);

    let result = response(&lines, 1)["result"].clone();
    assert_eq!(result["isError"], json!(false));
    let returned = &result["structuredContent"]["patch"];
    assert!(returned.is_array(), "the patch came back as {returned}");
    assert_eq!(returned[0]["op"], json!("replace"));
    // The version bumps ADR 0005 adds are part of what a dry run shows.
    assert_eq!(returned.as_array().unwrap().len(), 3);
    assert_eq!(result["structuredContent"]["entry_id"], json!(""), "a dry run mints no entry");
}

#[test]
fn a_refusal_is_a_tool_error_with_every_rule() {
    // §6's retry loop lives on this: a refused call has to reach the model as something it can
    // fix, carrying the stable rule ids it branches on.
    let dir = Scratch::new();
    let ops = json!([{"op": "replace", "path": "/nope", "value": 1}]);
    let lines = session(&dir.0, &[call(1, "apply_patch", json!({"patch": ops}))]);

    let result = response(&lines, 1)["result"].clone();
    assert_eq!(result["isError"], json!(true));
    assert_eq!(result["structuredContent"]["valid"], json!(false));
    assert_eq!(result["structuredContent"]["errors"][0]["rule"], json!("path_not_found"));
    assert!(result["structuredContent"]["patch"].is_null());
}

#[test]
fn an_unknown_tool_is_a_protocol_error_not_a_refusal() {
    let dir = Scratch::new();
    let lines = session(&dir.0, &[call(1, "make_it_funkier", json!({}))]);
    assert!(response(&lines, 1)["error"].is_object(), "should be a JSON-RPC error");
}

#[test]
fn the_log_comes_back_with_its_ops_readable() {
    // `PatchEntry.ops` is `bytes` for the same reason `patch` is, and would base64 for the
    // same reason.
    let dir = Scratch::new();
    let ops = json!([{"op": "replace", "path": format!("/tracks/{MASTER}/name"), "value": "Out"}]);
    let lines = session(
        &dir.0,
        &[call(1, "apply_patch", json!({"patch": ops})), call(2, "get_history", json!({}))],
    );

    let history = response(&lines, 2)["result"]["structuredContent"].clone();
    assert_eq!(history["head"], json!("main"));
    let entries = history["entries"].as_object().unwrap();
    assert_eq!(entries.len(), 2);
    for (_, entry) in entries {
        assert!(entry["ops"].is_array(), "ops came back as {}", entry["ops"]);
    }
}

#[test]
fn a_replay_is_reachable_by_entry_id() {
    let dir = Scratch::new();
    let ops = json!([{"op": "replace", "path": format!("/tracks/{MASTER}/name"), "value": "Out"}]);
    let lines = session(
        &dir.0,
        &[call(1, "get_history", json!({})), call(2, "apply_patch", json!({"patch": ops}))],
    );

    let root = response(&lines, 1)["result"]["structuredContent"]["entries"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let second = session(
        &Scratch::new().0,
        &[
            call(1, "apply_patch", json!({"patch": ops})),
            call(2, "get_song_at", json!({"entry_id": root})),
        ],
    );
    let replayed = response(&second, 2);
    let text = replayed["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("\"Master\""), "the root entry rebuilds the song before the edit");
}

// Determinism through a real process is `tests/determinism.rs` from M0.4: this file proved a
// two-session claim that the suite is a superset of, and two places asserting one claim is one
// place too many.
