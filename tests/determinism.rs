//! The determinism suite (`CLAUDE.md` M0 step 4; `docs/specs.md` §11).
//!
//! §11's claim is that the same input produces the same bytes. This drives a scripted session
//! through a **real server process** — `escribass-mcp`, over stdio — because `CLAUDE.md` #2
//! says mutations go through the tool API, and the library path cannot catch flag parsing,
//! transport serialisation, or a stray write to stdout.
//!
//! **Two runs agreeing is not the whole claim.** They catch nondeterminism — a process-random
//! `HashMap` seed, a wall clock, entropy — and they cannot catch *drift*: a dependency that
//! changes a serialisation detail produces the same wrong bytes twice, and the two runs agree.
//! That is what the committed goldens are for, and they arrive in the next step. This one
//! builds the harness and proves it can tell two runs apart at all.
//!
//! Every script is deterministic only because the binary is told to be: `--seed-ids` and
//! `--fixed-clock` are the sole route by which `core` is permitted to be reproducible (§11,
//! ADR 0001 §5), and nothing here reads a clock of its own.

use escribass_core::diff;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The instant every script runs at, and the seed every id is minted from.
///
/// 2026-09-02T00:00:00Z, the same instant the schema fixtures use. Under this seed an entity
/// id is a pure function of how many were minted before it: `new_song` mints four and
/// `Project::create` one, so the first id a tool mints is counter 6. That is why the scripts
/// name ids literally — a change in mint order changes what a script means, and it should say
/// so loudly rather than quietly still passing.
const AT: &str = "1788307200000";

const SCRIPTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/determinism");

// ---------------------------------------------------------------------------
// Finding the binary
// ---------------------------------------------------------------------------

/// The path to a binary this workspace builds.
///
/// `CARGO_BIN_EXE_<name>` is only set for integration tests of the package that *owns* the
/// binary, and this package does not own it — so the target directory is found from the test
/// executable's own path instead: `target/<profile>/deps/<test>` gives `target/<profile>` two
/// levels up. That survives `CARGO_TARGET_DIR` and a custom profile.
///
/// The binary is not built here on purpose: cargo holds the build-directory lock while tests
/// run, so a nested `cargo build` deadlocks. `cargo test` from the workspace root builds every
/// binary before any test runs, which is the supported way in; running this package alone can
/// find nothing to drive, and says so rather than hanging.
fn binary(name: &str) -> PathBuf {
    let executable = std::env::current_exe().expect("the test executable has a path");
    let target = executable
        .parent()
        .and_then(Path::parent)
        .expect("a test executable lives in <target>/<profile>/deps");
    let path = target.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    assert!(
        path.exists(),
        "`{name}` is not built at {}.\n\
         Run `cargo test` from the workspace root, which builds the binaries first; \
         `cargo test -p escribass-tests` on its own cannot.",
        path.display()
    );
    path
}

// ---------------------------------------------------------------------------
// Scripts
// ---------------------------------------------------------------------------

/// One tool call, and what the script expects of it.
///
/// `refused` makes a step self-checking. Without it, a step that fails for an unrelated reason
/// surfaces much later as a mismatch between two 40 KB documents; with it, the failure names
/// the tool and the rules that came back, at the step that caused it.
#[derive(Debug, Deserialize)]
struct Step {
    tool: String,
    #[serde(default)]
    args: Value,
    /// The `rule` this step must be refused with. Absent means it must succeed.
    #[serde(default)]
    refused: Option<String>,
}

fn script(name: &str) -> Vec<Step> {
    let path = PathBuf::from(SCRIPTS).join(name).join("script.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not a script: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// Running one
// ---------------------------------------------------------------------------

/// What a run produced: the project on disk, and what the server said at each step.
struct Run {
    directory: Scratch,
    /// One entry per step, in order, as the server returned it.
    results: Vec<Value>,
}

fn run(name: &str, clock: &str) -> Run {
    let steps = script(name);
    let directory = Scratch::new(name);

    let mut requests = vec![
        json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "escribass-tests", "version": "1"}
            }
        }),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    ];
    for (position, step) in steps.iter().enumerate() {
        requests.push(json!({
            "jsonrpc": "2.0", "id": position + 1, "method": "tools/call",
            "params": {"name": step.tool, "arguments": step.args}
        }));
    }

    let frames = speak(
        &[
            "--create",
            "--seed-ids",
            &format!("{AT}:1"),
            "--fixed-clock",
            clock,
            "--author",
            // Passed rather than left to the binary's default: `escribass-grpc` defaults to
            // `human` and `escribass-mcp` to `model`, so a cross-transport comparison would
            // differ in every `provenance.author` if neither said which it wanted.
            "model",
        ],
        &directory.0,
        &requests,
    );

    let mut results = Vec::with_capacity(steps.len());
    for (position, step) in steps.iter().enumerate() {
        let id = position + 1;
        let frame = frames
            .iter()
            .find(|frame| frame["id"] == json!(id))
            .unwrap_or_else(|| panic!("step {id} (`{}`) got no answer", step.tool));
        results.push(check(step, id, frame));
    }

    Run { directory, results }
}

/// Asserts a step did what the script said it would, and returns the part worth comparing.
fn check(step: &Step, id: usize, frame: &Value) -> Value {
    if let Some(error) = frame.get("error") {
        panic!("step {id} (`{}`) was a protocol error: {error}", step.tool);
    }
    let content = &frame["result"]["structuredContent"];
    let rules: Vec<&str> = content["errors"]
        .as_array()
        .map(|errors| errors.iter().filter_map(|e| e["rule"].as_str()).collect())
        .unwrap_or_default();

    match (&step.refused, content.get("valid").and_then(Value::as_bool)) {
        // A read has no `valid`; nothing to check beyond it having answered.
        (None, None) => {}
        (None, Some(true)) => {}
        (None, Some(false)) => {
            panic!("step {id} (`{}`) was refused: {rules:?}", step.tool)
        }
        (Some(expected), Some(false)) => assert!(
            rules.contains(&expected.as_str()),
            "step {id} (`{}`) expected `{expected}`, got {rules:?}",
            step.tool
        ),
        (Some(expected), _) => {
            panic!("step {id} (`{}`) expected `{expected}` and was accepted", step.tool)
        }
    }
    content.clone()
}

/// Sends a scripted conversation to `escribass-mcp` and returns its answers.
///
/// The whole script goes in before anything is read, which is simple and bounded: a script
/// larger than the stdin pipe buffer would deadlock, so the size is asserted rather than left
/// to be discovered as a hang.
fn speak(flags: &[&str], project: &Path, requests: &[Value]) -> Vec<Value> {
    let mut conversation = String::new();
    for request in requests {
        conversation.push_str(&serde_json::to_string(request).expect("a request serialises"));
        conversation.push('\n');
    }
    assert!(
        conversation.len() < 60_000,
        "the script is {} bytes, near the stdin pipe buffer; \
         send it line by line rather than all at once",
        conversation.len()
    );

    let mut child = Command::new(binary("escribass-mcp"))
        .args(flags)
        .arg(project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the server binary runs");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(conversation.as_bytes())
        .expect("the script is sent");

    let finished = child.wait_with_output().expect("the server exits when stdin closes");
    assert!(
        finished.status.success(),
        "the server failed: {}",
        String::from_utf8_lossy(&finished.stderr)
    );
    String::from_utf8(finished.stdout)
        .expect("stdout is utf-8")
        .lines()
        .map(|line| {
            serde_json::from_str(line).unwrap_or_else(|e| panic!("a frame is not JSON: {e}\n{line}"))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Comparing
// ---------------------------------------------------------------------------

/// Everything one run produced, as named bytes: the project's files, plus the answers.
///
/// One shape for both comparisons. Two runs against each other and a run against a committed
/// golden are the same question asked of different sources, and giving them one representation
/// means one report, one diff, and no chance of the two drifting into checking different
/// things.
struct Snapshot(BTreeMap<String, Vec<u8>>);

/// Files only: `assets/` is created empty and git cannot store an empty directory, so its
/// absence would read as a difference between a fresh run and a checkout.
fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let listing = std::fs::read_dir(&directory)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", directory.display()));
        for entry in listing {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let name = path
                    .strip_prefix(root)
                    .expect("everything is under the root")
                    .to_string_lossy()
                    // Normalised so the report reads the same on a future Windows runner.
                    .replace('\\', "/");
                found.insert(name, std::fs::read(&path).expect("a project file is readable"));
            }
        }
    }
    found
}

impl Snapshot {
    fn of(run: &Run) -> Self {
        let mut named = files(&run.directory.0);
        named.insert("responses.json".to_string(), pretty(&Value::Array(run.results.clone())));
        Snapshot(named)
    }

    fn read(name: &str) -> Self {
        let at = PathBuf::from(SCRIPTS).join(name).join("expected");
        assert!(
            at.exists(),
            "`{name}` has no golden at {}.\n\
             Write one with `UPDATE_FIXTURES=1 cargo test`, then read the diff before committing.",
            at.display()
        );
        Snapshot(files(&at))
    }

    fn write(&self, name: &str) {
        let at = PathBuf::from(SCRIPTS).join(name).join("expected");
        let _ = std::fs::remove_dir_all(&at);
        for (relative, bytes) in &self.0 {
            let path = at.join(relative);
            std::fs::create_dir_all(path.parent().expect("a file has a parent"))
                .expect("the golden directory is writable");
            std::fs::write(&path, bytes).expect("the golden is writable");
        }
    }
}

fn pretty(value: &Value) -> Vec<u8> {
    let mut text = serde_json::to_string_pretty(value).expect("a Value serialises");
    text.push('\n');
    text.into_bytes()
}

fn updating() -> bool {
    std::env::var("UPDATE_FIXTURES").is_ok()
}

/// The differences between two snapshots, as something a person can act on.
///
/// A determinism failure that says only `left != right` over two 40 KB documents is a test
/// nobody can use. This reports RFC 6902 operations between the parsed documents, so the
/// output names paths.
fn differences(left: &Snapshot, right: &Snapshot) -> Vec<String> {
    let mut report = Vec::new();
    let names: std::collections::BTreeSet<&String> = left.0.keys().chain(right.0.keys()).collect();
    for name in names {
        match (left.0.get(name), right.0.get(name)) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => report.push(format!("  {name}\n{}", ops(a, b))),
            (Some(_), None) => report.push(format!("  {name}\n      only on the left")),
            (None, Some(_)) => report.push(format!("  {name}\n      only on the right")),
            (None, None) => unreachable!("the name came from one of them"),
        }
    }
    report
}

/// The operations that take `a` to `b`, or a byte position when they are not both JSON.
fn ops(a: &[u8], b: &[u8]) -> String {
    let (Ok(before), Ok(after)) =
        (serde_json::from_slice::<Value>(a), serde_json::from_slice::<Value>(b))
    else {
        let at = a.iter().zip(b).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
        return format!("      not JSON; first differing byte at {at}");
    };
    // `diff` replaces an array whole, so an array is re-keyed by index first: otherwise a
    // changed operation inside a patch entry reports as "the whole `ops` array changed", which
    // is the unreadable output this function exists to avoid.
    let (before, after) = (indexed(before), indexed(after));
    diff(&before, &after)
        .iter()
        .map(|op| format!("      {} {}", op.path(), summarise(op, &before, &after)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Rewrites every array as an object keyed by index, so a diff recurses into it.
fn indexed(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Object(
            items.into_iter().enumerate().map(|(at, v)| (at.to_string(), indexed(v))).collect(),
        ),
        Value::Object(map) => {
            Value::Object(map.into_iter().map(|(k, v)| (k, indexed(v))).collect())
        }
        other => other,
    }
}

fn summarise(op: &escribass_core::Op, before: &Value, after: &Value) -> String {
    let shown = |value: Option<&Value>| match value {
        Some(value) => {
            let text = value.to_string();
            if text.len() > 60 { format!("{}…", &text[..60]) } else { text }
        }
        None => "absent".to_string(),
    };
    format!("{} -> {}", shown(before.pointer(op.path())), shown(after.pointer(op.path())))
}

fn assert_same(claim: &str, left: &Snapshot, right: &Snapshot) {
    let report = differences(left, right);
    assert!(report.is_empty(), "{claim}\n{}", report.join("\n"));
}

/// Compares a run with what was committed, or writes the golden when asked to.
///
/// This is the half that catches **drift**. Two runs in one job agree even when a dependency
/// has changed a serialisation detail underneath them, because both produce the same wrong
/// bytes; only something committed earlier disagrees.
///
/// `UPDATE_FIXTURES=1` blesses whatever ran, including a deterministically wrong output. The
/// guard is the rule in `tests/AGENTS.md`: a golden changes only in the pull request that
/// changes the canonical form or a tool's semantics, and its diff is reviewed there — §17's
/// rule for golden renders, applied here.
fn assert_matches_golden(name: &str, run: &Run) {
    let produced = Snapshot::of(run);
    if updating() {
        produced.write(name);
        return;
    }
    let report = differences(&Snapshot::read(name), &produced);
    assert!(
        report.is_empty(),
        "`{name}` no longer produces what was committed.\n\
         Left is the golden, right is this run.\n{}\n\n\
         If the change is intended, `UPDATE_FIXTURES=1 cargo test` rewrites it — and the diff \
         belongs in the pull request that changes the canonical form or a tool's semantics.",
        report.join("\n")
    );
}

// ---------------------------------------------------------------------------
// A place to work
// ---------------------------------------------------------------------------

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-determinism-{name}-{}-{}.escri",
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

// ---------------------------------------------------------------------------
// The claims
// ---------------------------------------------------------------------------

#[test]
fn every_tool_twice_is_the_same_bytes() {
    // §11, through a real process rather than the library: two servers, two directories, one
    // script covering every tool this milestone implements.
    let first = run("every_tool", AT);
    let second = run("every_tool", AT);
    assert_same(
        "the same script produced different projects",
        &Snapshot::of(&first),
        &Snapshot::of(&second),
    );
}

#[test]
fn the_comparison_notices_a_clock() {
    // A comparison that cannot fail proves nothing, and a determinism suite is exactly the
    // kind of test that can quietly compare nothing at all.
    //
    // Moving the clock is also the strongest wall-clock-leak detector available: *every*
    // differing path must be a `created_at`. Anything else differing means something read a
    // clock it should not have, and this fails.
    let fixed = run("every_tool", AT);
    let later = run("every_tool", "1788307260000");

    let report = differences(&Snapshot::of(&fixed), &Snapshot::of(&later));
    assert!(!report.is_empty(), "a minute of clock made no difference; the comparison is blind");

    for line in report.join("\n").lines().filter(|line| line.trim_start().starts_with('/')) {
        let path = line.trim().split_whitespace().next().unwrap_or_default();
        assert!(
            path.ends_with("/created_at") || path.ends_with("/seconds"),
            "the clock changed something that is not a timestamp: {path}\n{}",
            report.join("\n")
        );
    }
}

#[test]
fn a_project_reopens_in_a_fresh_process() {
    // ADR 0004's invariant, asserted through the binary: `Project::open` replays the log and
    // compares it with `song.json`, so a server that starts at all has agreed the two match.
    // A second process also proves the first left nothing in memory that the directory needs.
    let session = run("every_tool", AT);

    let frames = speak(
        &["--seed-ids", &format!("{AT}:1"), "--fixed-clock", AT, "--author", "model"],
        &session.directory.0,
        &[
            json!({
                "jsonrpc": "2.0", "id": 0, "method": "initialize",
                "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                           "clientInfo": {"name": "escribass-tests", "version": "1"}}
            }),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                   "params": {"name": "get_song", "arguments": {}}}),
        ],
    );

    let reopened = frames
        .iter()
        .find(|frame| frame["id"] == json!(1))
        .expect("the reopened server answered");
    let text = reopened["result"]["content"][0]["text"]
        .as_str()
        .expect("get_song returns canonical text");
    let on_disk = std::fs::read_to_string(session.directory.0.join("song.json"))
        .expect("the project has a song");
    assert_eq!(text, on_disk, "a fresh process read a different song than the file holds");
}

#[test]
fn every_tool_still_produces_what_was_committed() {
    // The half two runs cannot prove. A dependency that changes how a float is written, or
    // feature unification that flips `serde_json::Map` to insertion order, produces the same
    // wrong bytes in both runs of the test above — and only something committed earlier
    // disagrees.
    assert_matches_golden("every_tool", &run("every_tool", AT));
}

#[test]
fn json_objects_are_still_written_in_key_order() {
    // Naming the hazard rather than letting the golden fail mysteriously. `serde_json::Map` is
    // a `BTreeMap` unless the `preserve_order` feature is on anywhere in the tree, and feature
    // unification means any crate can turn it on for everyone: `rmcp` already depends on
    // `indexmap` directly. If that happens, `diff` op order changes, `bump_versions` walks in a
    // different order, and every `add` value in the log changes key order — a workspace-wide
    // change to the canonical form with no local cause to find.
    assert_eq!(json!({"b": 1, "a": 2}).to_string(), r#"{"a":2,"b":1}"#);
}

#[test]
fn a_refusal_leaves_no_trace() {
    // The claim: a call that was refused changes nothing at all — not the document, not the
    // log, and not the ids the *next* call mints. That last one is the subtle half, and it is
    // why this script ends with a second `add_track`: its ids are pinned in the golden, so a
    // refusal that quietly burned one would move them and fail here.
    let first = run("refusals", AT);
    let second = run("refusals", AT);
    assert_same("refusals were not reproducible", &Snapshot::of(&first), &Snapshot::of(&second));
    assert_matches_golden("refusals", &first);
}

#[test]
fn branching_and_merging_are_reproducible() {
    // The claim: navigation is reproducible and records nothing, a merge records one entry with
    // two parents, and a merge that conflicts writes nothing at all. The script ends by
    // replaying the root entry, so the golden also pins what the song looked like before any of
    // it — which is the property a discarded branch depends on (ADR 0001 §2).
    let first = run("branches", AT);
    let second = run("branches", AT);
    assert_same("branching was not reproducible", &Snapshot::of(&first), &Snapshot::of(&second));
    assert_matches_golden("branches", &first);
}
