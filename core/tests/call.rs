//! The one dispatch, driven through both carriers that use it (ADR 0012 §1).
//!
//! `escribass_core::call` exists so that `app` does not write a second `match` on a tool name
//! beside the one the MCP server had — ADR 0006 §1's sixteen copies of the contract, a
//! milestone later, in a place no test drives. A claim like that is worth exactly what checks
//! it, so this suite runs one script two ways and compares the answers:
//!
//! - **through the MCP envelope**, as a real `escribass-mcp` subprocess over real pipes, which
//!   is the carrier M0.4's determinism suite already drives and compares against gRPC;
//! - **through `call` directly**, which is what a `#[tauri::command]` body is. `ponytail:` the
//!   Tauri envelope itself is not driven here — it is three lines (take the session lock,
//!   call, hand back `structured`) and it lives in `app/src-tauri`, which a `cargo test` on a
//!   machine with no webview cannot build. What this suite pins is the half that could drift,
//!   which is the answer. If that envelope ever grows a decision, it needs its own test in
//!   `app/`, not a third comparison here.
//!
//! If the two ever disagree, somebody has written a second dispatch.

mod common;

use escribass_core::call::{call, Answer, ErrorKind, IMPLEMENTED};
use escribass_core::{new_song, FixedClock, Project, SeededIds, Session};
use escribass_schema::song::Author;
use serde_json::{json, Map, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The instant and seed both runs use, so the two agree about every id they mint.
const AT: i64 = 1_788_307_200_000;

/// The master track `new_song` mints, which the script edits so that a call actually commits.
const MASTER: &str = "01M1FPMP000000000000000002";

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-call-{name}-{}-{}.escri",
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

/// One step of the script: a tool and its arguments.
fn step(name: &str, arguments: Value) -> (&str, Map<String, Value>) {
    let Value::Object(arguments) = arguments else { panic!("arguments are an object") };
    (name, arguments)
}

/// The script both carriers run.
///
/// Chosen for the arms that are not a one-line `decode`: a read that renders canonical text, a
/// dry run, an apply that commits, `apply_patch` with the RFC 6902 array ADR 0006 §6 exempts
/// from base64, a refusal the session answers rather than fails, and the log.
fn script() -> Vec<(&'static str, Map<String, Value>)> {
    vec![
        step("get_song", json!({})),
        step("add_track", json!({"name": "Bass", "kind": "TRACK_KIND_INSTRUMENT", "dry_run": true})),
        step("add_track", json!({"name": "Bass", "kind": "TRACK_KIND_INSTRUMENT"})),
        step("set_tempo", json!({"tick": 0, "bpm": 128.0})),
        step(
            "apply_patch",
            json!({"patch": [{"op": "replace", "path": format!("/tracks/{MASTER}/name"), "value": "Out"}]}),
        ),
        // Refused, not failed: a `ToolResult` with `valid: false` and the rule it broke.
        step("set_tempo", json!({"tick": 0, "bpm": -1.0})),
        step("get_song", json!({})),
        step("get_history", json!({})),
    ]
}

/// Runs the script in-process, exactly as the Tauri command does.
fn through_call(directory: &Path) -> Vec<Value> {
    let mut ids = SeededIds::new(AT, 1);
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock, Author::Model);
    let project =
        Project::create(directory, &song, &mut ids, &clock, Author::Model, common::manifest())
            .expect("the project is created");
    let mut session =
        Session::new(project, Box::new(ids), Box::new(clock), Author::Model);

    script()
        .iter()
        .map(|(name, arguments)| match call(&mut session, name, arguments) {
            Ok(Answer { text, structured, refused }) => {
                json!({"text": text, "structured": structured, "refused": refused})
            }
            Err(e) => json!({"error": e.message, "kind": format!("{:?}", e.kind)}),
        })
        .collect()
}

/// Runs the script through a real `escribass-mcp` process, and shapes its answers the same way.
fn through_mcp(directory: &Path) -> Vec<Value> {
    let mut requests = String::new();
    let mut push = |value: Value| {
        requests.push_str(&serde_json::to_string(&value).expect("a request serialises"));
        requests.push('\n');
    };
    push(json!({
        "jsonrpc": "2.0", "id": 0, "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1"}
        }
    }));
    push(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    for (id, (name, arguments)) in script().iter().enumerate() {
        push(json!({
            "jsonrpc": "2.0", "id": id + 1, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        }));
    }

    let mut child = Command::new(env!("CARGO_BIN_EXE_escribass-mcp"))
        .args([
            "--create",
            "--manifest",
            common::MANIFEST,
            "--seed-ids",
            &format!("{AT}:1"),
            "--fixed-clock",
            &AT.to_string(),
        ])
        .arg(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the server binary runs");
    child.stdin.take().expect("stdin").write_all(requests.as_bytes()).expect("the script is sent");
    let finished = child.wait_with_output().expect("the server exits when stdin closes");
    assert!(
        finished.status.success(),
        "the server failed: {}",
        String::from_utf8_lossy(&finished.stderr)
    );

    let frames: Vec<Value> = String::from_utf8(finished.stdout)
        .expect("stdout is utf-8")
        .lines()
        .map(|l| serde_json::from_str(l).expect("every frame is JSON"))
        .collect();

    (1..=script().len())
        .map(|id| {
            let frame = frames
                .iter()
                .find(|v| v["id"] == json!(id))
                .unwrap_or_else(|| panic!("no response with id {id}"));
            if let Some(error) = frame.get("error") {
                return json!({
                    "error": error["message"],
                    "kind": if error["code"] == json!(-32602) { "BadRequest" } else { "Broken" },
                });
            }
            let result = &frame["result"];
            json!({
                "text": result["content"][0]["text"],
                "structured": result["structuredContent"],
                "refused": result["isError"],
            })
        })
        .collect()
}

#[test]
fn the_two_carriers_answer_the_same_way() {
    let mine = Scratch::new("direct");
    let theirs = Scratch::new("mcp");
    let direct = through_call(&mine.0);
    let over_mcp = through_mcp(&theirs.0);

    assert_eq!(direct.len(), script().len());
    for (i, ((name, _), (a, b))) in
        script().iter().zip(direct.iter().zip(over_mcp.iter())).enumerate()
    {
        assert_eq!(
            a, b,
            "step {i} (`{name}`) came out differently through `call` and through MCP"
        );
    }
}

#[test]
fn a_song_crosses_as_the_models_own_field_order() {
    // ADR 0006 §6's second byte-level rule, now a property of `call` rather than of one
    // carrier. `serde_json::to_value` would alphabetise: `id` would precede `provenance`,
    // and the frontend would be reading a different document from the one on disk.
    let directory = Scratch::new("canonical");
    let answers = through_call(&directory.0);
    let text = answers[0]["text"].as_str().expect("a song answers with text");
    let order: Vec<&str> =
        text.lines().filter_map(|l| l.strip_prefix("  \"")).filter_map(|l| l.split('"').next()).collect();
    assert_eq!(
        &order[..4],
        &["id", "provenance", "version", "schema_version"],
        "the song came back alphabetised, not canonical"
    );
}

#[test]
fn a_patch_crosses_as_an_array_and_never_as_base64() {
    // ADR 0006 §6's first rule. The step in the script that uses it applied, so the array was
    // read as RFC 6902 rather than as text to be base64-decoded.
    let directory = Scratch::new("patch");
    let answers = through_call(&directory.0);
    let applied = &answers[4]["structured"];
    assert_eq!(applied["valid"], json!(true), "{applied}");
    assert!(applied["patch"].is_array(), "the patch came back as {}", applied["patch"]);
}

#[test]
fn a_refused_call_is_an_answer_and_not_an_error() {
    // ADR 0006 §2: a call the validator refuses is something the caller can act on, so it
    // comes back as a result with `valid: false` — over MCP as a *tool* error, which is what
    // lets a model self-correct, and to the UI as something to draw rather than to throw.
    let directory = Scratch::new("refusal");
    let answers = through_call(&directory.0);
    let refused = &answers[5];
    assert_eq!(refused["refused"], json!(true));
    assert_eq!(refused["structured"]["valid"], json!(false));
    assert!(!refused["structured"]["errors"].as_array().expect("errors").is_empty());
}

#[test]
fn every_advertised_tool_has_an_arm() {
    // `IMPLEMENTED` is what both carriers advertise and what `call` dispatches, and a name in
    // one and not the other is a call that can only fail — a spent turn for a model, a dead
    // control for a user. Empty arguments, so most of these are refused or fail to decode;
    // what must never come back is "no tool".
    let directory = Scratch::new("arms");
    let mut ids = SeededIds::new(AT, 1);
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock, Author::Model);
    let project =
        Project::create(&directory.0, &song, &mut ids, &clock, Author::Model, common::manifest())
            .expect("the project is created");
    let mut session = Session::new(project, Box::new(ids), Box::new(clock), Author::Model);

    for name in IMPLEMENTED {
        if let Err(e) = call(&mut session, name, &Map::new()) {
            assert!(!e.message.starts_with("no tool"), "`{name}` is advertised with no arm");
        }
    }
}

#[test]
fn an_unknown_tool_is_the_callers_mistake_and_not_the_operators() {
    // The distinction ADR 0006 §2 draws, and the whole reason `CallError` carries a kind: MCP
    // turns `BadRequest` into `invalid_params` so §6's retry loop sees it, and `Broken` into
    // `internal_error` so it leaves the loop.
    let directory = Scratch::new("unknown");
    let mut ids = SeededIds::new(AT, 1);
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock, Author::Model);
    let project =
        Project::create(&directory.0, &song, &mut ids, &clock, Author::Model, common::manifest())
            .expect("the project is created");
    let mut session = Session::new(project, Box::new(ids), Box::new(clock), Author::Model);

    let e = call(&mut session, "open_project", &Map::new()).expect_err("there is no such tool");
    assert_eq!(e.kind, ErrorKind::BadRequest);
    assert!(e.message.contains("open_project"), "{}", e.message);
}
