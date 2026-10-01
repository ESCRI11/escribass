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
//! That is what the committed goldens are for. So the claim is checked three ways: two
//! processes against each other, each against what was committed, and MCP against gRPC.
//!
//! Every script is deterministic only because the binary is told to be: `--seed-ids` and
//! `--fixed-clock` are the sole route by which `core` is permitted to be reproducible (§11,
//! ADR 0001 §5), and nothing here reads a clock of its own.

#[path = "common/mod.rs"]
mod common;
use common::{binary, engine_sources, refuse_if_older_than_source, speak, Scratch, AT};

use escribass_core::diff;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const SCRIPTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/determinism");

/// The build manifest every script runs against (ADR 0010 §4).
///
/// A committed **subset** of a real `escribass_engine --scan`, not the manifest a build
/// writes — that one is generated at build time and never committed, because a committed
/// description of a plugin binary is a second pin on it with the stale one silent. The
/// binaries refuse to start without a manifest and the validator takes it as an argument
/// rather than an `Option`, so there is no path on which a missing one quietly means
/// "everything is valid"; a machine with no engine build runs these scripts against this
/// file, and every plugin id and parameter id in it came out of a real plugin.
///
/// It is also what puts an `engine` block and a `plugins` block in every golden `lock.json`.
/// Moving a pin therefore moves this file *and* four goldens, in one pull request, which is
/// §17's rule for golden renders arriving where it was always going to.
const MANIFEST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/manifest.json");

/// The scripts any machine with a built workspace can drive.
const SCRIPTED: [&str; 5] = ["every_tool", "refusals", "branches", "render", "undo"];

/// The scripts that **compile**, and so need `core` told a `--generator` (ADR 0024 §1).
///
/// Separate from [`SCRIPTED`] because a compile spawns a real Python child out of
/// `compilers/generative/`, which cargo cannot produce — so the tests that run these are
/// behind the `generators` cargo feature, for `renders`' and `ai`'s reason: a `#[test]` that
/// noticed there was no `uv` and returned would be the quiet skip this suite exists to
/// prevent. `every_implemented_tool_is_scripted` still reads them, because reading a
/// `script.json` needs no compiler and a tool that no script calls must fail everywhere.
///
/// `compile` is M4 PR 6's: its two steps build the clip a **model** then writes a generator
/// into, so the script itself compiles nothing and the turn does. Naming it here is what puts
/// the sandbox on that session and the staleness guard on that path (trap 5).
const COMPILES: [&str; 2] = ["generators", "compile"];

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

/// The `--generator` flag, for the scripts that compile and for nothing else.
///
/// Reaching the staleness guard from here rather than from each test is deliberate: both
/// transports and every repeat go through this one function, so there is no path on which a
/// golden is compared against an environment from before the change (trap 5).
fn telling_it_about_a_compiler(name: &str) -> Vec<String> {
    if !COMPILES.contains(&name) {
        return Vec::new();
    }
    vec!["--generator".to_string(), common::generator_command()]
}

fn run(name: &str, clock: &str) -> Run {
    let steps = script(name);
    let directory = Scratch::new("determinism", name);

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

    let mut flags: Vec<String> = vec![
        "--create".to_string(),
        "--manifest".to_string(),
        MANIFEST.to_string(),
        "--seed-ids".to_string(),
        format!("{AT}:1"),
        "--fixed-clock".to_string(),
        clock.to_string(),
        "--author".to_string(),
        // Passed rather than left to the binary's default: `escribass-grpc` defaults to
        // `human` and `escribass-mcp` to `model`, so a cross-transport comparison would
        // differ in every `provenance.author` if neither said which it wanted.
        "model".to_string(),
    ];
    flags.extend(telling_it_about_a_compiler(name));
    let frames = speak(
        &flags.iter().map(String::as_str).collect::<Vec<_>>(),
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

    // The tools that answer with something other than a `ToolResult`: three reads, which
    // return what they read, and `add_asset`, which returns where it put something. None of
    // them produces ops for the pipeline to record or refuse, so none has a `valid` to carry
    // (ADR 0006 §1).
    const NOT_TOOL_RESULTS: [&str; 4] =
        ["get_song", "get_song_at", "get_history", "add_asset"];
    match (&step.refused, content.get("valid").and_then(Value::as_bool)) {
        // Matched by name rather than by the field's absence, so a mutating tool whose result
        // lost its shape fails here instead of passing quietly.
        (None, None) if NOT_TOOL_RESULTS.contains(&step.tool.as_str()) => {}
        (None, None) => panic!("step {id} (`{}`) answered without `valid`: {content}", step.tool),
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
    // `isError` is what ADR 0006 §2's table gives a model's retry loop, and it is carried
    // beside `valid` rather than derived from it — so the two can disagree, and nothing else
    // here would notice. The gRPC driver builds a synthetic frame without one, which is why
    // this is conditional rather than required.
    if let Some(flagged) = frame["result"].get("isError") {
        let expected = content.get("valid").is_some_and(|valid| valid != &json!(true));
        assert_eq!(
            flagged,
            &json!(expected),
            "step {id} (`{}`): `isError` disagrees with `valid`",
            step.tool
        );
    }
    content.clone()
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

/// Files only, which is what makes `assets/` compare correctly in both states: empty, it
/// contributes nothing on either side, so git's inability to store an empty directory never
/// reads as a difference; non-empty, every asset is compared by name and by bytes like any
/// other file. `add_asset` is what first puts one there (ADR 0011, Consequences).
///
/// `lock` is the one name skipped, and for the same reason the others are all kept: it is not
/// part of the project. It is a live process's claim on the directory (ADR 0012 §3), holding a
/// pid, removed when that process exits — so it is neither reproducible nor content, and a
/// harness that kills its server leaves one behind where a harness that closes stdin does not.
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
                if name == "lock" {
                    continue;
                }
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
        named.insert("origin.json".to_string(), origin().into_bytes());
        named.insert("plan.json".to_string(), plan(&run.directory.0));
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

/// The document every replay starts from.
///
/// A patch log is not self-contained. `core` replays from a default `Song` rather than from
/// `{}`, because the canonical form emits every no-presence field (ADR 0002 §4) and that is
/// what makes `replace` legal from the first operation. Anything else replaying the log —
/// the TypeScript and Python halves of this claim, or a person with `jq` — needs the same
/// starting point, and would otherwise fail on the root entry with no idea why.
///
/// So it is committed beside the goldens. `ponytail:` a project does not carry its own origin
/// today; if one ever has to be replayed by something that has never seen this schema, that is
/// the thing to add to the `.escri` directory.
fn origin() -> String {
    escribass_core::to_canonical_json(&escribass_schema::song::Song::default())
        .expect("a default Song serialises")
}

fn pretty(value: &Value) -> Vec<u8> {
    let mut text = serde_json::to_string_pretty(value).expect("a Value serialises");
    text.push('\n');
    text.into_bytes()
}

/// What `compile` says about the project a run produced: the plan, or every reason there is
/// none (ADR 0007 §4, §6).
///
/// The plan joins every golden because it is where M0's claim and M1's meet — same input,
/// same bytes, same plan — and a render mismatch later splits into "core changed the plan"
/// and "the engine changed the rendering" by comparing this file first. A refusal is golden
/// too: `every_tool` holds a Faust effect, and that M1 says so, naming the field, is a claim.
///
/// The asset index is built from the directory listing under a fixed root. `compile` reads no
/// file, so the path is opaque to it, and a run's temporary directory in a golden would be
/// the one kind of input this suite exists to keep out.
fn plan(root: &Path) -> Vec<u8> {
    let manifest = std::sync::Arc::new(
        escribass_core::Manifest::read(MANIFEST).expect("the manifest fixture is readable"),
    );
    let project =
        escribass_core::Project::open(root, manifest).expect("a run leaves a project that opens");
    let assets: BTreeMap<String, PathBuf> = std::fs::read_dir(root.join("assets"))
        .map(|listing| {
            listing
                .map(|entry| {
                    let name = entry.expect("an entry").file_name().to_string_lossy().into_owned();
                    (name.clone(), Path::new("/escri/assets").join(name))
                })
                .collect()
        })
        .unwrap_or_default();
    match escribass_core::compile(project.song(), &assets) {
        // Serialised from the plan itself, never through a `Value`, which would sort struct
        // fields and lose the proto's order (`core/src/canonical.rs`).
        Ok(plan) => {
            let mut text = serde_json::to_string_pretty(&plan).expect("a plan serialises");
            text.push('\n');
            text.into_bytes()
        }
        Err(refused) => {
            let reasons: Vec<Value> = refused
                .iter()
                .map(|v| json!({"path": v.path, "rule": v.rule, "message": v.message}))
                .collect();
            pretty(&json!({"refused": reasons}))
        }
    }
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
            // By characters, not bytes: `Value::to_string` does not escape non-ASCII, and a
            // byte slice through a `é` panics — while building the report that explains a
            // failure, which is the worst possible moment.
            match text.char_indices().nth(60) {
                Some((at, _)) => format!("{}…", &text[..at]),
                None => text,
            }
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

    let text = report.join("\n");
    for line in text.lines() {
        let trimmed = line.trim();
        // A file that appears or vanishes with the clock, or one that stopped being JSON, is
        // reported as prose rather than as a path — so it would slip past a loop that only
        // inspected paths, which is exactly the blind spot a leak would hide in.
        assert!(
            !trimmed.starts_with("only on the") && !trimmed.starts_with("not JSON"),
            "the clock changed which files exist, or their shape:\n{text}"
        );
        if trimmed.starts_with('/') {
            let path = trimmed.split_whitespace().next().unwrap_or_default();
            assert!(
                path.ends_with("/created_at"),
                "the clock changed something that is not a timestamp: {path}\n{text}"
            );
        }
    }
}

#[test]
fn every_project_reopens_in_a_fresh_process() {
    // ADR 0004's invariant, asserted through the binary: `Project::open` replays the log and
    // compares it with `song.json`, so a server that starts at all has agreed the two match.
    // A second process also proves the first left nothing in memory that the directory needs.
    for name in SCRIPTED {
        reopens(name);
    }
}

/// Starts a second server on a project and reads it back.
///
/// `Project::open` replays the log and compares it with `song.json`, so a server that starts at
/// all has agreed the two match — which is where ADR 0004's `song_diverged` and `replay_failed`
/// surface. Every script is reopened rather than one: a merge that produced an unreplayable log
/// only shows up on the script that merges.
///
/// One limit worth knowing: `switch_branch` rewrites `song.json` from a replay, so a script
/// that switches *after* the point where a log and a document could diverge has already
/// reconciled them by the time this runs. On such a script the golden is what catches a
/// divergence, not this.
fn reopens(name: &str) {
    let session = run(name, AT);

    let frames = speak(
        &["--manifest", MANIFEST, "--seed-ids", &format!("{AT}:1"), "--fixed-clock", AT,
          "--author", "model"],
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
        .unwrap_or_else(|| panic!("`{name}` did not reopen"));
    let text = reopened["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("`{name}` reopened without a song: {reopened}"));
    let on_disk = std::fs::read_to_string(session.directory.0.join("song.json"))
        .expect("the project has a song");
    assert_eq!(text, on_disk, "`{name}`: a fresh process read a different song than the file");
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

#[test]
fn undoing_and_redoing_are_reproducible_and_leave_the_log_longer() {
    // ADR 0005 §4's claim, driven through a real process. The script is what a person does with
    // ⌘Z: four edits, a previewed undo, two real ones, two redos, a redo at the tip, and an
    // ordinary commit in the middle of the cursor — and what the golden pins is that the log
    // only ever **grew**, that the entry each undo reversed is still in it, and what the
    // document looked like at each of the four `get_song` steps.
    //
    // The two `get_song` steps between the undos are the half a single end-state golden would
    // miss: they are what distinguishes "⌘Z twice walks two edits back" from "⌘Z twice undoes
    // its own undo", which produce the same tip and different middles.
    let first = run("undo", AT);
    let second = run("undo", AT);
    assert_same("undo was not reproducible", &Snapshot::of(&first), &Snapshot::of(&second));
    assert_matches_golden("undo", &first);

    // Read off the golden rather than asserted from memory: every entry the script appended is
    // still there, and the two the undos reversed are among them (§5's audit trail).
    let history = std::fs::read_dir(first.directory.0.join("patches")).expect("patches");
    assert_eq!(
        history.count(),
        11,
        "one root entry, four edits, four undo/redo entries counted below, one more undo and \
         one `set_tempo` — a rewind would have left fewer"
    );
}

#[test]
fn the_plan_is_reproducible() {
    // ADR 0007 §4's claim: the plan is a pure function of the document and the asset index.
    // The script is what compile resolves — clips added out of order, a chain out of index
    // order, a note loop with a short last iteration, a stretched audio loop, a muted track
    // with automation on its device, two entities on one parameter, a section past the last
    // clip, and — since M2 PR 6 — lanes whose `ParamRef` names a *track* rather than a device,
    // on an ordinary track and on the master, in the model's own units (ADR 0015 §1, §2). The
    // gain ramp crosses this script's tempo change on purpose, since that is the one place the
    // engine has to split a segment. The golden is the plan those produce; every other
    // script's `plan.json` is pinned by its own golden test.
    let first = run("render", AT);
    let second = run("render", AT);
    assert_same("the plan was not reproducible", &Snapshot::of(&first), &Snapshot::of(&second));
    assert_matches_golden("render", &first);
}

// ---------------------------------------------------------------------------
// Compiling a generator, end to end (ADR 0024 §8)
// ---------------------------------------------------------------------------
//
// Behind the `generators` cargo feature, for the `ai` module's reason and by the same
// mechanism: every compile in the script spawns a real Python child out of
// `compilers/generative/`, which needs `uv` and a synced environment cargo cannot produce.
//
//     cd compilers/generative && uv sync --locked
//     cargo test -p escribass-tests --features generators
//
// What it drives is the **real** compiler through the **real** server binaries, over both
// transports, which is the pairing neither half can check alone: `compilers/generative/`'s 43
// tests know nothing about a document, and `core/tests/generator.rs` drives a shell script
// beside a `Generate` server in its own process. Here a source in a document becomes notes in
// a clip, a hash, a `toolchain_version`, a block in `lock.json` and an entry in the log —
// twice, and against bytes committed earlier.
#[cfg(feature = "generators")]
mod through_the_sandbox {
    use super::*;

    #[test]
    fn compiling_twice_is_the_same_bytes_and_the_ones_committed() {
        // §11's three ways, on a tier CLAUDE.md #3 names for the first time since M0: two
        // runs against each other catch a clock, a pid, a socket path or a temp directory
        // reaching the recorded bytes — a compile spawns a process and every one of those is
        // in reach — and the committed golden catches drift, which two runs in one job
        // cannot, because both produce the same wrong bytes.
        let first = run("generators", AT);
        let second = run("generators", AT);
        assert_same(
            "the same script compiled to different projects",
            &Snapshot::of(&first),
            &Snapshot::of(&second),
        );
        assert_matches_golden("generators", &first);
    }

    #[test]
    fn the_two_transports_compile_the_same_way() {
        // M0.3's review found MCP's hand-decoded `apply_patch` reading `"dry_run": "true"` as
        // false and applying; the two tools added here are decoded by the generated
        // deserializer on both sides, and this is what says so rather than assuming it. A
        // compile is the first tool whose answer depends on a *child process*, so it is also
        // the first place a transport could differ by starting one differently.
        let over_mcp = run("generators", AT);
        let over_grpc = run_over_grpc("generators", AT);
        assert_same(
            "`generators` came out differently over the two transports",
            &Snapshot::of(&over_mcp),
            &Snapshot::of(&over_grpc),
        );
    }

    #[test]
    fn a_compiled_project_reopens_in_a_fresh_process() {
        // ADR 0004's invariant on a project a compiler wrote, and ADR 0027 §2's: a second
        // process opens it with a `toolchains` block in its `lock.json`, compares **nothing**
        // in that block, and agrees that `song.json` is a replay of the log. A project with a
        // generator opens on a machine with no compiler, which is what makes the compiled
        // notes layer 2 rather than a dependency of opening at all.
        reopens("generators");
    }

    #[test]
    fn what_the_generators_golden_covers_that_a_single_compile_would_not() {
        // Named claims, read off the committed golden rather than off a fresh run, so the
        // golden cannot quietly stop covering them (docs/plan.md, M3 trap 1).
        let committed = Snapshot::read("generators");
        let answers: Vec<Value> = serde_json::from_slice::<Value>(
            committed.0.get("responses.json").expect("the golden has responses.json"),
        )
        .expect("responses.json is JSON")
        .as_array()
        .expect("an array")
        .clone();
        let steps = script("generators");
        let at = |tool: &str, nth: usize| -> Value {
            answers
                .iter()
                .zip(&steps)
                .filter(|(_, step)| step.tool == tool)
                .map(|(answer, _)| answer.clone())
                .nth(nth)
                .unwrap_or_else(|| panic!("no {nth}th `{tool}` in the golden"))
        };

        // 1. **A dry run and the apply that follows it write the same patch**, note ids
        //    included — which is what a person approving a diff is promised (ADR 0006 §3).
        assert_eq!(at("compile_generator", 0)["patch"], at("compile_generator", 1)["patch"]);
        assert_eq!(at("compile_generator", 0)["entry_id"], json!(""), "a dry run recorded an entry");
        assert_ne!(at("compile_generator", 1)["entry_id"], json!(""), "the apply recorded none");

        // 2. **Up to date spawns nothing and says nothing changed** (ADR 0024 §6).
        assert_eq!(at("compile_generator", 2)["summary"], json!("up to date"));
        assert_eq!(at("compile_generator", 2)["patch"], Value::Null);

        // 3. **Editing the source stales it**, and the next dry run has a diff again — the
        //    whole of what the code view's three words are read off.
        assert_ne!(at("compile_generator", 3)["patch"], Value::Null);

        // 4. **The notes are the DSL's**, produced by the pinned interpreter from a seed above
        //    2⁵³: fourteen from the worked example, six after the hats are deleted.
        let notes = |answer: Value| {
            answer["patch"]
                .as_array()
                .expect("a patch")
                .iter()
                .filter(|op| {
                    op["path"].as_str().unwrap_or_default().contains("/note_clip/notes/")
                        && op["op"] == json!("add")
                })
                .count()
        };
        assert_eq!(
            notes(at("compile_generator", 1)),
            14,
            "the worked example compiles to fourteen notes"
        );
        assert_eq!(notes(at("compile_generator", 4)), 6, "and six once the hats are deleted");

        // 5. **A generator that emits nothing writes its hash and no notes** — zero notes is
        //    an answer, and a different one from no answer at all (`proto/generate.proto`).
        let empty = at("compile_generator", 5);
        assert_eq!(notes(empty.clone()), 0);
        assert!(
            empty["patch"]
                .as_array()
                .expect("a patch")
                .iter()
                .any(|op| op["path"].as_str().unwrap_or_default().ends_with("compiled_hash")),
            "{empty:?}"
        );

        // 6. **Each of the three refusals the script can reach**, with nothing written.
        for (nth, rule) in [(6, "target_not_note_clip"), (7, "generator_error"), (8, "generator_unknown")] {
            let refused = at("compile_generator", nth);
            assert_eq!(refused["valid"], json!(false), "{rule}");
            let rules: Vec<&str> = refused["errors"]
                .as_array()
                .expect("errors")
                .iter()
                .filter_map(|e| e["rule"].as_str())
                .collect();
            assert_eq!(rules, vec![rule]);
        }
        // The diagnostic is the child's own words with its line in front of them, which is
        // what a person reads and what a model retries against (ADR 0026 §3, Plate 3).
        let said = at("compile_generator", 7)["errors"][0]["message"]
            .as_str()
            .expect("a message")
            .to_string();
        assert!(said.starts_with("1:1: "), "it names where: {said}");
        assert!(said.contains("Import"), "it names what: {said}");

        // 7. **`lock.json` carries the block**, with the version the child stated and the
        //    interpreter §17 pins — the one place in this repository where that string is
        //    compared by byte (ADR 0027 §1, its unmeasured half).
        let lock: Value = serde_json::from_slice(committed.0.get("lock.json").expect("lock.json"))
            .expect("lock.json is JSON");
        assert_eq!(lock["toolchains"], json!({"generator": {"dsl": "1", "python": "3.12.12"}}));

        // 8. **Every compiled note is `core`'s**: a ULID from the seeded source and the
        //    session's own provenance, because the sandbox returns §4.3 blank (trap 9).
        let song: Value = serde_json::from_slice(committed.0.get("song.json").expect("song.json"))
            .expect("song.json is JSON");
        let compiled = song["clips"]["01M1FPMP000000000000000009"]["note_clip"]["notes"]
            .as_object()
            .expect("the compiled clip");
        assert_eq!(compiled.len(), 6);
        for (id, note) in compiled {
            assert_eq!(id.len(), 26, "that is not a ULID: {id}");
            assert_eq!(note["id"], json!(id));
            assert_eq!(note["provenance"]["author"], json!("AUTHOR_MODEL"));
            assert_eq!(note["provenance"]["created_at"], json!("2026-09-02T00:00:00+00:00"));
        }
    }

    #[test]
    fn a_runaway_generator_is_a_generator_error_and_not_the_wall_clock() {
        // ADR 0024 §4 and §7, as amended in M4 PR 4, asserted against the **real** child and
        // through the real server: a loop that cannot end reaches the child's own
        // `RLIMIT_CPU`, which it catches and answers as an ordinary diagnostic — so it comes
        // back as `generator_error` naming the line it was on, and never as
        // `generator_timeout`, which is the wall clock's alone. Outside the golden, because
        // `--cpu-seconds 1` is a different command from the one the script runs under and the
        // measurement is a second or two.
        let directory = Scratch::new("determinism", "runaway");
        let source = "x = 0\nwhile True:\n    x = x + 1\n";
        let requests = vec![
            json!({
                "jsonrpc": "2.0", "id": 0, "method": "initialize",
                "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                           "clientInfo": {"name": "escribass-tests", "version": "1"}}
            }),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                "name": "add_track",
                "arguments": {"name": "T", "kind": "TRACK_KIND_INSTRUMENT", "ref": {"plugin": {
                    "plugin_id": "Surge Synth Team/Surge XT", "version": "1.3.4"}}}}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "add_clip",
                "arguments": {"track_id": "01M1FPMP000000000000000006",
                              "start_tick": 0, "length_ticks": 3840}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "define_generator",
                "arguments": {"kind": "GENERATOR_KIND_PYTHON", "seed": "7", "source": source,
                              "clip_id": "01M1FPMP000000000000000009"}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {
                "name": "compile_generator",
                "arguments": {"generator_id": "01M1FPMP00000000000000000B"}}}),
        ];
        let frames = speak(
            &[
                "--create", "--manifest", MANIFEST,
                "--seed-ids", &format!("{AT}:1"), "--fixed-clock", AT, "--author", "model",
                "--generator", &format!("{} --cpu-seconds 1", common::generator_command()),
            ],
            &directory.0,
            &requests,
        );
        let answer = frames.iter().find(|f| f["id"] == json!(4)).expect("an answer");
        let content = &answer["result"]["structuredContent"];
        assert_eq!(content["valid"], json!(false), "a loop that cannot end was not refused");
        assert_eq!(content["errors"][0]["rule"], json!("generator_error"),
            "the CPU limit is not the wall clock (ADR 0024 §7)");
        let said = content["errors"][0]["message"].as_str().expect("a message");
        assert!(said.contains("CPU limit"), "{said}");
        // Which of the loop's two lines the signal lands on is the scheduler's to decide, and
        // that it lands on one of them is the claim: a limit that reported line 0 would be
        // the diagnostic Plate 3 complains about.
        assert!(
            said.starts_with("2:") || said.starts_with("3:"),
            "it names the line the generator was on: {said}"
        );
    }
}

// ---------------------------------------------------------------------------
// The loop, end to end (ADR 0022 §4)
// ---------------------------------------------------------------------------
//
// Behind the `ai` cargo feature, for `renders.rs`'s reason and by the same mechanism: it needs
// `uv` and a synced `ai/` environment, which `cargo test` cannot produce, and a `#[test]` that
// noticed that and returned early would be the quiet skip this suite exists to prevent. Without
// the feature the code below does not exist; with it, it insists.
//
//     cd ai && uv sync --locked
//     cargo test -p escribass-tests --features ai
//
// What it drives is the **real** `ai` process with the scripted provider, and **`core`'s own
// client** — `Assistant`, `Sidecar::turn`, `Session`, the proposal (ADR 0020 §4). A fake
// `Assistant` server in Rust would test the host half against a description of the sidecar
// rather than the sidecar, which is the alternative ADR 0022 §4 rejected. The one thing it does
// not drive is a model: "same transcript → same log" is the claim, and it is the one a suite
// can check without a key (docs/plan.md, "What 'deterministic' means with a model in the loop").
#[cfg(feature = "ai")]
mod through_the_sidecar {
    use super::*;
    use escribass_core::{Assistant, FixedClock, Project, SeededIds, Session, Sidecar};
    use escribass_schema::song::Author;
    use std::time::{Duration, Instant};

    /// Where the turn's ids start.
    ///
    /// Past everything the script minted, because the script ran in another process with its
    /// own seeded source and two sources at the same counter would mint the same ULIDs. 100 is
    /// far enough to be obviously past, and it is what makes the transcript's
    /// `01M1FPMP000000000000000034` — 100 in Crockford base32 — a number a reader can check.
    const TURN_SEED: u64 = 100;

    /// The prompt `proposal`'s transcript was recorded against, on 2026-09-24.
    const ASKED: &str = "add a lead line over the bass";

    /// The demo §1 states the product claim with and §18.2 leads every demo with: *"Change the
    /// bass line in bar 17 and re-render, everything else identical"*. `roadmap.md` makes M3's
    /// proof point that demo **driven by the assistant, with the diff on screen before apply**,
    /// and that is what the `bar17` script and its transcript are.
    const BAR_17_PROMPT: &str = "move the bass note in bar 17 up an octave";

    fn ai_command(transcript: &Path) -> Vec<String> {
        let ai = common::workspace().join("ai");
        vec![
            "uv".to_string(),
            "run".to_string(),
            "--project".to_string(),
            ai.display().to_string(),
            "escribass-ai".to_string(),
            "--transcript".to_string(),
            transcript.display().to_string(),
        ]
    }

    /// What one turn left behind: the project the script built, the session it ran on, the
    /// sidecar, and the turn itself — everything a person's decision is made against.
    ///
    /// Split out from [`drive`] in PR 9, because Apply is no longer the only thing that
    /// happens next: Reject and Edit are the other two controls §9 names, and each needs the
    /// same turn driven to the same point and then decided differently (ADR 0019 §3).
    struct Turned {
        run: Run,
        session: Session,
        sidecar: Sidecar,
        turn: escribass_core::Turn,
    }

    /// One prompt, driven to its end against a project the script built.
    ///
    /// The project is created and edited by a **real `escribass-mcp` process** first, exactly
    /// as every other script is, so nothing here writes `song.json` and every mutation before
    /// the turn came through the tool API (CLAUDE.md #2). The turn's own mutations come through
    /// `core::call`, which is the same dispatch.
    fn turned(name: &str, clock: &str, prompt: &str) -> Turned {
        let mut session_run = run(name, clock);
        let at: i64 = clock.parse().expect("the clock is milliseconds");

        let manifest = std::sync::Arc::new(
            escribass_core::Manifest::read(MANIFEST).expect("the manifest fixture is readable"),
        );
        let project = Project::open(&session_run.directory.0, manifest)
            .expect("the script left a project that opens");
        let mut session = Session::new(
            project,
            Box::new(SeededIds::new(at, TURN_SEED)),
            Box::new(FixedClock(at)),
            // The window's author, which is what it stays: the model's calls never enter
            // through it (ADR 0021 §1, Consequences).
            Author::Human,
        );

        // The sandbox, for a script that compiles — told as a command and never searched for
        // (ADR 0024 §1), exactly as the two server binaries are told it. Behind the feature
        // because a compile spawns a real Python child out of `compilers/generative/`, which
        // cargo cannot build; without it this session has none and `compile_generator` is
        // `generator_missing`, which is a refusal rather than a quiet skip.
        #[cfg(feature = "generators")]
        {
            if COMPILES.contains(&name) {
                session.set_sandbox(escribass_core::Sandbox::new(
                    common::generator_command()
                        .split_whitespace()
                        .map(str::to_string)
                        .collect(),
                ));
            }
        }

        let transcript = PathBuf::from(SCRIPTS).join(name).join("transcript.json");
        let mut sidecar: Sidecar = Assistant::new(ai_command(&transcript))
            .start()
            .expect("the real sidecar starts; run `cd ai && uv sync --locked` if it does not");

        // What the panel draws **as it grows** (ADR 0019 §2): the watcher `core` calls after
        // every event, here counting how many times the patch was computable before the turn
        // ended. A loop that only produced a proposal at the end would leave this at zero,
        // which is what the assertion downstream reads.
        let mut watched: Vec<usize> = Vec::new();
        let turn = sidecar
            .turn(&mut session, prompt, &[], |growing, session| {
                let ops = session
                    .proposal()
                    .and_then(|proposal| proposal.patch().ok())
                    .map(|prepared| prepared.ops().len())
                    .unwrap_or(0);
                let _ = growing;
                watched.push(ops);
            })
            .expect("the turn answers");
        session_run.results.push(json!({ "watched": watched }));
        session_run.results.push(json!({
            "turn": {
                "end": format!("{:?}", turn.end),
                "model_id": turn.model_id,
                "reply": turn.recorded.reply,
                "calls": turn.recorded.calls.iter().map(|done| {
                    let call = done.call.as_ref().expect("a completed call has its call");
                    let result = done.result.as_ref().expect("and its result");
                    json!({
                        "name": call.name,
                        "call_id": call.call_id,
                        "args": call.args_json,
                        "valid": result.valid,
                        "summary": result.summary,
                        "errors": result.errors.iter().map(|e| json!({
                            "path": e.path, "rule": e.rule, "message": e.message,
                        })).collect::<Vec<_>>(),
                    })
                }).collect::<Vec<_>>(),
            }
        }));

        Turned { run: session_run, session, sidecar, turn }
    }

    /// The same turn, applied — the golden path, and what a person pressing **Apply** does.
    fn drive(name: &str, clock: &str, prompt: &str) -> Run {
        let Turned { mut run, mut session, sidecar, turn } = turned(name, clock, prompt);

        // Nothing is applied by the loop: a proposal ends in the stream and waits for a person
        // (ADR 0019 §2). This test is that person.
        let proposal = session.proposal().expect("the turn left a proposal");
        let pending = proposal.patch().expect("the patch is computable after every call");
        run.results.push(json!({
            "pending": {
                "ops": serde_json::from_str::<Value>(&escribass_core::ops_text(pending.ops()))
                    .expect("a patch is JSON"),
                "calls": proposal.calls(),
                "prompt_id": proposal.prompt_id(),
            }
        }));

        let applied = session.apply_proposal(&turn.model_id).expect("the project writes");
        assert!(applied.valid, "the proposal was refused: {applied:?}");
        run.results.push(json!({
            "applied": {
                "entry_id": applied.entry_id,
                "summary": applied.summary,
                "patch": serde_json::from_slice::<Value>(&applied.patch).expect("a patch is JSON"),
            }
        }));

        println!("{}", sidecar.stop());
        run
    }

    #[test]
    fn a_scripted_turn_produces_the_same_project_twice_and_the_one_committed() {
        // §11's shape, with a model above the tool API. Two runs against each other catch a
        // clock or an unseeded source; the committed golden catches drift, which two runs in
        // one job cannot, because both produce the same wrong bytes (docs/plan.md, M0.4).
        let first = drive("proposal", AT, ASKED);
        let second = drive("proposal", AT, ASKED);
        assert_same(
            "the same transcript produced different projects",
            &Snapshot::of(&first),
            &Snapshot::of(&second),
        );
        assert_matches_golden("proposal", &first);
    }

    #[test]
    fn what_the_golden_covers_that_a_single_call_turn_would_not() {
        // Named claims, read off the committed golden rather than off a fresh run, so the
        // golden cannot quietly stop covering them (docs/plan.md, M3 trap 1: a check that
        // cannot fail). Every one of these is false for a loop that never composes anything.
        let committed = Snapshot::read("proposal");
        let answers: Value = serde_json::from_slice(
            committed.0.get("responses.json").expect("the golden has responses.json"),
        )
        .expect("responses.json is JSON");
        let turn = &answers.as_array().expect("an array").iter()
            .find(|answer| answer.get("turn").is_some())
            .expect("the golden has a turn")["turn"];

        // 1. **Two mutating calls**, not one, and both accepted.
        let calls = turn["calls"].as_array().expect("calls");
        assert_eq!(calls.len(), 2, "the golden stopped being a multi-call turn");
        assert!(calls.iter().all(|call| call["valid"] == json!(true)), "{calls:?}");
        assert_eq!(calls[0]["name"], json!("add_track"));
        assert_eq!(calls[1]["name"], json!("add_clip"));

        // 2. **The second call names the id the first call minted** — the whole of what a fork
        //    buys over a dry run, and the thing the spike watched every model get refused for.
        let minted = calls[0]["summary"].as_str().expect("a summary names the path");
        let named = calls[1]["args"].as_str().expect("the arguments as the model wrote them");
        let id = minted
            .split('/')
            .nth(2)
            .expect("`1 op: /tracks/<id>, …` names the track")
            .split(|c: char| !c.is_ascii_alphanumeric())
            .next()
            .expect("a ULID is alphanumeric");
        assert_eq!(id.len(), 26, "that is not a ULID: {minted}");
        assert!(named.contains(id), "the second call named `{id}`? {named}");

        // 3. **One entry** for both calls, and its patch touches a track *and* a clip — which a
        //    single-call golden cannot show, because one call cannot make both.
        let applied = &answers.as_array().expect("an array").iter()
            .find(|answer| answer.get("applied").is_some())
            .expect("the golden has an apply")["applied"];
        let paths: Vec<String> = applied["patch"]
            .as_array()
            .expect("a patch is an array")
            .iter()
            .map(|op| op["path"].as_str().unwrap_or_default().to_string())
            .collect();
        assert!(paths.iter().any(|path| path.starts_with("/tracks/")), "{paths:?}");
        assert!(paths.iter().any(|path| path.starts_with("/clips/")), "{paths:?}");

        // 4. **The log grew by exactly one**, whatever the number of calls (ADR 0017 §1).
        let entries = committed.0.keys().filter(|name| name.starts_with("patches/")).count();
        assert_eq!(entries, 4, "one create, two script steps, one proposal");

        // 5. **The entry names the model and the prompt**, and its entities name their call
        //    (ADR 0021 §2) — visible in the committed log itself.
        let entry = committed
            .0
            .iter()
            .find(|(name, bytes)| {
                name.starts_with("patches/")
                    && String::from_utf8_lossy(bytes).contains("\"tool\": \"proposal\"")
            })
            .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
            .expect("the golden has a `proposal` entry");
        assert!(entry.contains("\"AUTHOR_MODEL\""), "{entry}");
        assert!(entry.contains("deepseek/deepseek-v4.1-flash"), "{entry}");
        // The ids the **model** gave its two calls on 2026-09-24, written literally as every
        // fixture id here is: a transcript that is quietly replaced by another recording, or
        // by a hand-written one, fails on this line rather than passing on a shape.
        let first = "\"tool_call_id\": \"call_01a0d371167372d787b4e9cb\"";
        let second = "\"tool_call_id\": \"call_01a0d3711db47618a6f3fac2\"";
        assert!(entry.contains(first), "{entry}");
        assert!(entry.contains(second), "{entry}");

        // 6. **The project records the model it was sent to**, on first use (ADR 0021 §4).
        let lock: Value =
            serde_json::from_slice(committed.0.get("lock.json").expect("lock.json"))
                .expect("lock.json is JSON");
        assert_eq!(lock["ai"]["model"], json!("deepseek/deepseek-v4.1-flash"));
        assert_eq!(lock["ai"]["provider"], json!("openrouter"));
    }

    // -----------------------------------------------------------------------
    // A model defines a generator and compiles it (M4 PR 6; ADR 0026 §3)
    // -----------------------------------------------------------------------
    //
    // Behind **both** features, which is what the turn needs: the real `ai` process for the
    // loop and a real `compilers/generative` child for the compile. `cargo test -p
    // escribass-tests --features ai,generators` is what runs it, and CI runs exactly that.
    //
    // The transcript is **hand-written** and says so in its own file, as `four-refusals.json`
    // does, because recording one costs a paid call and CLAUDE.md #7 says nothing calls a
    // paid service without the user's confirmation. So this is not evidence that a model
    // writes the DSL — that is U10, unmeasured — and the file says that too. What it is
    // evidence of is the loop: the two tools offered, a diagnostic crossing back whole, a
    // source edited by `apply_patch` on its path, and one patch at the end for a person.

    /// The prompt the hand-written transcript answers.
    #[cfg(feature = "generators")]
    const COMPILE_PROMPT: &str = "write me a drum generator for the empty clip";

    /// The generator the turn mints: counter 100 in Crockford base32, as [`TURN_SEED`] says,
    /// and the id the transcript's own calls name. Written literally for `tests/AGENTS.md`'s
    /// reason — a change in mint order should fail here rather than quietly compile something
    /// else.
    #[cfg(feature = "generators")]
    const COMPILED: &str = "01M1FPMP000000000000000034";

    #[cfg(feature = "generators")]
    #[test]
    fn a_model_defines_compiles_reads_the_diagnostic_and_compiles_again() {
        // §11's shape with a model above the tool API **and** a compiler below it, which is
        // the first time both children are in one claim. Two runs against each other catch a
        // clock, a pid or a socket path reaching the bytes — a compile spawns a process and
        // every one of those is in reach — and the committed golden catches drift, which two
        // runs in one job cannot.
        let first = drive("compile", AT, COMPILE_PROMPT);
        let second = drive("compile", AT, COMPILE_PROMPT);
        assert_same(
            "the same transcript and the same compiler produced different projects",
            &Snapshot::of(&first),
            &Snapshot::of(&second),
        );
        assert_matches_golden("compile", &first);
    }

    #[cfg(feature = "generators")]
    #[test]
    fn what_the_compile_golden_covers_that_a_turn_without_a_compiler_would_not() {
        // Named claims, read off the committed golden rather than off a fresh run, so the
        // golden cannot quietly stop covering them (docs/plan.md, M3 trap 1).
        let committed = Snapshot::read("compile");
        let answers: Value = serde_json::from_slice(
            committed.0.get("responses.json").expect("the golden has responses.json"),
        )
        .expect("responses.json is JSON");
        let answers = answers.as_array().expect("an array");
        let turn = &answers
            .iter()
            .find(|answer| answer.get("turn").is_some())
            .expect("the golden has a turn")["turn"];
        let calls = turn["calls"].as_array().expect("calls");

        // 1. **Four calls, and the two new tools among them.** A turn that only defined would
        //    prove nothing about the child; one that only compiled would prove nothing about
        //    the fork.
        assert_eq!(
            calls.iter().map(|c| c["name"].as_str().unwrap_or("")).collect::<Vec<_>>(),
            ["define_generator", "compile_generator", "apply_patch", "compile_generator"],
        );

        // 2. **The second was refused, by the real compiler, with the line it refused on.**
        //    This is the whole of ADR 0026 §3: `valid = false`, rule `generator_error`, the
        //    child's own `line:column` and its own sentence, which `core` does not rewrite.
        assert_eq!(calls[1]["valid"], json!(false));
        let refusal = &calls[1]["errors"].as_array().expect("violations")[0];
        assert_eq!(refusal["path"], json!(format!("/generators/{COMPILED}/source")));
        assert_eq!(refusal["rule"], json!("generator_error"));
        assert_eq!(
            refusal["message"],
            json!(concat!(
                "4:24: `/` (Div) is not in the generator DSL: ",
                "write a // b, or Fraction(a, b) for an exact ratio (ADR 0024 §3)"
            )),
        );

        // 3. **And the model acted on it**: the third call patches the path the refusal
        //    named, which is how a source is edited — there is no `set_generator_source`
        //    (ADR 0026 §2) — and the fourth compile was accepted.
        assert!(
            calls[2]["args"]
                .as_str()
                .expect("the arguments as the model wrote them")
                .contains(&format!("/generators/{COMPILED}/source")),
            "{}",
            calls[2]["args"]
        );
        assert_eq!(calls[3]["valid"], json!(true));

        // 4. **The notes are in the committed document**, so the child really ran and what it
        //    wrote survived the fork, the patch and the apply. Eight: a kick on each of four
        //    beats and a hat between them.
        let song: Value =
            serde_json::from_slice(committed.0.get("song.json").expect("song.json"))
                .expect("song.json is JSON");
        let notes = song["clips"]["01M1FPMP000000000000000009"]["note_clip"]["notes"]
            .as_object()
            .expect("the clip the model compiled into");
        assert_eq!(notes.len(), 8);

        // 5. **The log grew by exactly one**, under `proposal`, for four calls two of which
        //    spawned a process (ADR 0017 §1, ADR 0019 §4).
        assert_eq!(
            committed.0.keys().filter(|name| name.starts_with("patches/")).count(),
            4,
            "one create, two script steps, one proposal"
        );

        // 6. **The accepted consequence, visible in the golden a reader opens** (ADR 0027 §1,
        //    amended 2026-10-01). A compile inside a proposal writes the generator's
        //    `toolchain_version` — it is in the patch a person applies — and does **not**
        //    write the project's `toolchains` block, because `record_toolchain` fires only on
        //    a compile that commits an entry and this one committed a `proposal`. So a
        //    project whose compiles have all been a model's carries a compiled generator and
        //    an unpinned `lock.json`, and the two cannot disagree because
        //    `toolchain_mismatch` compares the generator's own version as well as the block.
        assert_eq!(song["generators"][COMPILED]["toolchain_version"], json!("1"));
        assert!(!song["generators"][COMPILED]["compiled_hash"]
            .as_str()
            .expect("a compiled generator carries its hash")
            .is_empty());
        let lock: Value = serde_json::from_slice(committed.0.get("lock.json").expect("lock.json"))
            .expect("lock.json is JSON");
        assert!(
            lock.get("toolchains").is_none(),
            "a model's compile pinned the project; only a committing compile does (ADR 0027 §1)"
        );
    }

    // -----------------------------------------------------------------------
    // The bar-17 demo, driven by the assistant (roadmap.md's M3 proof point)
    // -----------------------------------------------------------------------
    //
    // §1's product claim is "change the bass line in bar 17 and re-render, everything else
    // identical", §18.2 makes it the canonical demo, and M1 PR 12 made it a render test. What
    // `roadmap.md` asks of M3 is the same demo **driven by the assistant, diff on screen
    // before apply** — so what is asserted below is the panel's claim rather than the
    // engine's: the diff exists before anything is written, the project has not moved while it
    // is on screen, and each of §9's three controls does what ADR 0019 §3 says it does.
    //
    // It renders nothing. M1 PR 12 already measured what an edit to bar 17 does to audio, down
    // to the sample; repeating that here would be a second engine test wearing a panel's
    // clothes, and this suite has no engine.

    /// The clip the `bar17` script builds and the note in its seventeenth bar.
    ///
    /// Written literally, as every determinism script's ids are: under `--seed-ids` an id is a
    /// pure function of how many were minted before it, so a change in mint order fails here
    /// rather than quietly editing a different note (`tests/AGENTS.md`). They are the same two
    /// ids `tests/renders.rs` names, because that script has the same shape.
    const BAR_17_CLIP: &str = "01M1FPMP000000000000000009";
    const BAR_17_NOTE: &str = "01M1FPMP00000000000000000T";
    /// 960 PPQ, 4/4: bar 17 starts after sixteen whole bars.
    const BAR_17_TICK: i64 = 16 * 4 * 960;

    /// The pitches of the clip's eighteen notes, in bar order.
    fn bass_line(session: &Session) -> Vec<(i64, i64)> {
        let clip = &session.project().song().clips[BAR_17_CLIP];
        let escribass_schema::song::clip::Content::NoteClip(notes) =
            clip.content.as_ref().expect("a note clip")
        else {
            panic!("the bar-17 clip is not a note clip")
        };
        let mut line: Vec<(i64, i64)> = notes
            .notes
            .values()
            .map(|note| (i64::from(note.start_tick), i64::from(note.pitch)))
            .collect();
        line.sort();
        line
    }

    #[test]
    fn the_bar_17_demo_shows_the_diff_before_anything_is_applied() {
        let Turned { mut run, mut session, sidecar, turn } =
            turned("bar17", AT, BAR_17_PROMPT);

        // **The diff is on screen and the project has not moved.** This is the whole of the
        // proof point, and it is two assertions: there is a patch to read, and reading it has
        // cost the document nothing (ADR 0019 §1).
        let before = bass_line(&session);
        let entries_before = session.project().history().entries().len();
        let proposal = session.proposal().expect("the turn left a proposal");
        let pending = proposal.patch().expect("the patch is computable after every call");
        let ops: Value = serde_json::from_str(&escribass_core::ops_text(pending.ops()))
            .expect("a patch is JSON");
        let paths: Vec<&str> = ops
            .as_array()
            .expect("a patch is an array")
            .iter()
            .map(|op| op["path"].as_str().unwrap_or_default())
            .collect();
        assert!(
            paths.iter().any(|path| path
                == &format!("/clips/{BAR_17_CLIP}/note_clip/notes/{BAR_17_NOTE}/pitch")),
            "the proposal does not move bar 17's note: {paths:?}"
        );
        // Nothing before bar 17, and nothing after it: the *document* claim §1 makes, at the
        // level this suite can make it. Only the one note, its clip's version and the song's.
        for path in &paths {
            assert!(
                path.starts_with(&format!("/clips/{BAR_17_CLIP}/note_clip/notes/{BAR_17_NOTE}/"))
                    || path == &format!("/clips/{BAR_17_CLIP}/version")
                    || path == &"/version",
                "the proposal reached past bar 17's note: {path}"
            );
        }
        assert_eq!(bass_line(&session), before, "reading the diff changed the document");
        assert_eq!(session.project().history().entries().len(), entries_before);

        // **Drawn as it grows** (ADR 0019 §2): the watcher fired while the turn was still
        // running, and the patch was already computable then. Without it a panel could only
        // show a proposal once the model had stopped, which for the one live turn this
        // repository has measured would have been fourteen seconds of nothing.
        let watched = run
            .results
            .iter()
            .find_map(|answer| answer.get("watched"))
            .and_then(Value::as_array)
            .expect("the turn was watched")
            .clone();
        assert!(!watched.is_empty(), "the watcher never fired: nothing could be drawn");
        assert!(
            watched.iter().any(|ops| ops.as_u64().unwrap_or(0) > 0),
            "every event of the turn had an empty patch: {watched:?}"
        );

        run.results.push(json!({
            "pending": {
                "ops": ops,
                "calls": proposal.calls(),
                "prompt_id": proposal.prompt_id(),
            }
        }));

        // And then a person applies. One entry, whatever the number of calls (ADR 0017 §1).
        let applied = session.apply_proposal(&turn.model_id).expect("the project writes");
        assert!(applied.valid, "{applied:?}");
        run.results.push(json!({
            "applied": {
                "entry_id": applied.entry_id,
                "summary": applied.summary,
                "patch": serde_json::from_slice::<Value>(&applied.patch).expect("a patch is JSON"),
            }
        }));
        assert_eq!(session.project().history().entries().len(), entries_before + 1);
        let entry =
            session.project().history().get(&applied.entry_id).expect("the entry").clone();
        assert_eq!(entry.tool, "proposal");

        // The one note moved an octave, and the seventeen others are where they were.
        let after = bass_line(&session);
        let moved: Vec<_> = before
            .iter()
            .zip(&after)
            .filter(|(was, now)| was != now)
            .map(|(was, now)| (was.0, was.1, now.1))
            .collect();
        assert_eq!(moved, vec![(BAR_17_TICK, 36, 48)], "more than bar 17's note moved");

        println!("{}", sidecar.stop());
        assert_matches_golden("bar17", &run);
    }

    #[test]
    fn rejecting_the_bar_17_proposal_leaves_the_project_byte_for_byte_where_it_was() {
        // ADR 0019 §3: Reject drops the fork and **records nothing**. Counting entries is the
        // check that cannot fail here (docs/plan.md, M3 trap 1) — a Reject that quietly wrote
        // `song.json`, moved a ref or rewrote `lock.json` would pass it — so what is compared
        // is every byte under the project directory, through the same `files` the goldens use.
        let Turned { run, mut session, sidecar, .. } = turned("bar17", AT, BAR_17_PROMPT);
        assert!(session.proposal().is_some(), "the turn left no proposal");
        let before = files(&run.directory.0);

        assert!(session.reject(), "there was nothing to reject");
        assert!(session.proposal().is_none());
        let after = files(&run.directory.0);
        assert_eq!(
            before.keys().collect::<Vec<_>>(),
            after.keys().collect::<Vec<_>>(),
            "a rejected proposal added or removed a file"
        );
        for (name, bytes) in &before {
            assert_eq!(bytes, &after[name], "a rejected proposal changed `{name}`");
        }
        println!("{}", sidecar.stop());
    }

    #[test]
    fn editing_the_bar_17_proposal_applies_the_persons_own_patch() {
        // §9's third control, end to end (ADR 0019 §3). The person takes the proposal's patch
        // text, changes the number in it, and applies it as **theirs**: the entry says
        // `apply_patch`, `AUTHOR_HUMAN` and no `model_id`, and the `prompt_id` is kept so the
        // row still leads to this conversation.
        let Turned { run, mut session, sidecar, .. } = turned("bar17", AT, BAR_17_PROMPT);
        let proposal = session.proposal().expect("the turn left a proposal");
        let prompt_id = proposal.prompt_id().to_string();
        let text = escribass_core::ops_text(
            proposal.patch().expect("the patch is computable").ops(),
        );

        // A patch a person mistyped is refused **in place**, and the proposal is still there
        // to edit — ADR 0017 §3's refusal path, which ADR 0019 §3 asks for by name.
        let refused = session
            .edit_proposal(b"[{\"op\": \"replace\", \"path\": \"/clips/nope/x\", \"value\": 1}]")
            .expect("a refusal is an answer");
        assert!(!refused.valid, "{refused:?}");
        assert!(session.proposal().is_some(), "a mistyped patch threw the turn away");
        assert_eq!(files(&run.directory.0).len(), files(&run.directory.0).len());

        // The person's own number: 43, not the model's 48.
        let mine = text.replace("48", "43");
        assert_ne!(mine, text, "the proposal does not carry the model's pitch");
        let applied = session.edit_proposal(mine.as_bytes()).expect("the project writes");
        assert!(applied.valid, "{applied:?}");
        assert!(session.proposal().is_none(), "an applied edit left the proposal pending");

        let entry =
            session.project().history().get(&applied.entry_id).expect("the entry").clone();
        assert_eq!(entry.tool, "apply_patch", "an edit is the person's own apply_patch");
        let made = entry.provenance.expect("an entry carries provenance");
        assert_eq!(made.author, Author::Human as i32, "a person owns the bytes they wrote");
        assert_eq!(made.model_id, None);
        assert_eq!(made.prompt_id.as_deref(), Some(prompt_id.as_str()));
        assert_eq!(
            bass_line(&session).iter().find(|(tick, _)| *tick == BAR_17_TICK),
            Some(&(BAR_17_TICK, 43)),
            "the edit did not land on bar 17's note"
        );
        println!("{}", sidecar.stop());
    }

    // -----------------------------------------------------------------------
    // Reconciling the ledger against the provider's own counter (ADR 0022 §4)
    // -----------------------------------------------------------------------

    /// The ledger `provider.Live` writes, as a person reads it: the rows, in order.
    ///
    /// Outside the repository on purpose — it is a record of real money and not a fixture —
    /// and named here rather than passed, because the ceiling is only a ceiling if every run
    /// counts against the same total (`ai/src/escribass_ai/provider.py`, `SPEND`).
    fn ledger() -> Vec<Value> {
        let path = dirs_home().join(".escribass").join("spend.jsonl");
        let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("a ledger row is JSON"))
            .collect()
    }

    fn dirs_home() -> PathBuf {
        PathBuf::from(std::env::var("HOME").expect("a home directory"))
    }

    fn charged(rows: &[Value]) -> f64 {
        rows.iter().map(|row| row["charged_usd"].as_f64().unwrap_or_default()).sum()
    }

    /// One free query against OpenRouter's own metering, through `escribass-ai`.
    ///
    /// `--account-usage` and `--generation-cost` print one number and exit, building no
    /// provider and spending nothing (`ai/src/escribass_ai/__init__.py`). They are here
    /// because until 2026-09-24 the two functions behind them had **no caller at all**, while
    /// §6.1 and ADR 0022 §4 both said the ledger is reconciled against the provider's counter.
    /// It was, once, by hand.
    fn counter(arguments: &[&str]) -> f64 {
        let ai = common::workspace().join("ai");
        let said = Command::new("uv")
            .args(["run", "--project", &ai.display().to_string(), "escribass-ai"])
            .args(arguments)
            .output()
            .expect("uv runs");
        assert!(
            said.status.success(),
            "escribass-ai {arguments:?} failed: {}",
            String::from_utf8_lossy(&said.stderr)
        );
        String::from_utf8_lossy(&said.stdout)
            .trim()
            .parse()
            .unwrap_or_else(|e| panic!("{arguments:?} printed something that is not a number: {e}"))
    }

    /// Whether the provider's own counter agrees with what the ledger recorded.
    ///
    /// Pure, so the comparison is testable without a run and without a cent — which is the
    /// point of it existing as a function at all. The slack is a hundredth of what was spent,
    /// floored at a ten-millionth of a dollar: the ledger records `usage.cost` per call as the
    /// provider stated it, and the account counter sums the provider's own numbers, so the two
    /// differ only by rounding. A difference bigger than that is either a call this repository
    /// made and did not ledger, or a ledger row for a call that was never billed — and both are
    /// things a person must be told about (CLAUDE.md #7).
    fn reconciled(moved: f64, ledgered: f64) -> Result<String, String> {
        let slack = (ledgered.abs() * 0.01).max(1e-7);
        let said = format!(
            "OpenRouter's own counter moved by ${moved:.8}; the ledger recorded \
             ${ledgered:.8} (slack ${slack:.8})"
        );
        if (moved - ledgered).abs() <= slack {
            Ok(said)
        } else {
            Err(format!("{said} — they differ by ${:.8}", moved - ledgered))
        }
    }

    #[test]
    fn the_reconciliation_notices_a_counter_that_disagrees() {
        // The half of ADR 0022 §4's reconciliation that can be checked without spending:
        // the comparison itself. Watched failing first with the comparison written as
        // `moved >= ledgered`, which passes for every overcharge there is.
        assert!(reconciled(0.00376174, 0.00376174).is_ok());
        // Rounding either way is not a disagreement.
        assert!(reconciled(0.003762, 0.00376174).is_ok());
        // A call that was made and not ledgered is, and so is a row for a call nobody billed.
        let over = reconciled(0.0075, 0.00376174).expect_err("twice the spend agreed");
        assert!(over.contains("they differ by $0.003738"), "{over}");
        assert!(reconciled(0.0, 0.00376174).is_err(), "a counter that did not move agreed");
        // And a run that spent nothing reconciles against a counter that did not move, rather
        // than dividing by it.
        assert!(reconciled(0.0, 0.0).is_ok());
    }

    /// The live run (ADR 0022 §4), **skipped loudly**.
    ///
    /// `#[ignore]`d rather than conditional, for `renders.rs`'s device test's reason: a test
    /// that noticed the variable was missing and returned would be the quiet skip this suite
    /// exists to prevent, and this one would also spend the user's money the moment the
    /// variable happened to be set. It prints why on every run of the suite.
    ///
    /// ```text
    /// OPENROUTER_API_KEY=… cargo test -p escribass-tests --features ai -- --ignored --nocapture
    /// ```
    ///
    /// **Nothing else in this repository spends money**, and nothing calls a paid service
    /// without the user saying so (CLAUDE.md #7). What this run is *for* is the one thing a
    /// transcript cannot be: evidence that a real model, offered these twelve schemas, composes
    /// a multi-call proposal through this loop.
    ///
    /// **It records.** Every request body the loop built and every response the provider gave
    /// is written to `target/live-turn.json`, in the format `--transcript` reads — the body
    /// only, so a recording cannot carry `Authorization: Bearer …` (M3 trap 10), which this
    /// test asserts rather than assumes. `target/` and not the fixture tree, because an
    /// attempt that fails mid-turn would otherwise overwrite a good recording with half of a
    /// bad one: installing what it leaves is a deliberate act, and the printed path is the
    /// invitation to make it.
    ///
    /// **The prompt is `drive`'s**, word for word, against the same project at the same clock
    /// and the same seed — so what it records is a recording of the very turn the scripted
    /// golden replays, and the two are comparable rather than merely alike.
    ///
    /// The spend is capped before each call and ledgered by `provider.Live`
    /// (`~/.escribass/spend.jsonl`), which is where a person reads what this cost.
    #[test]
    #[ignore = "spends money: set OPENROUTER_API_KEY and run with --ignored"]
    fn a_live_model_drives_the_loop() {
        let Ok(_key) = std::env::var("OPENROUTER_API_KEY") else {
            panic!(
                "OPENROUTER_API_KEY is not set, so there is no live run to make.\n\
                 This test is `#[ignore]`d and spends the user's money when it runs; it is \
                 here so that a person can make the run, not so that CI can (ADR 0022 §4)."
            );
        };
        let recording = common::workspace().join("target").join("live-turn.json");
        let _ = std::fs::remove_file(&recording);
        println!(
            "about to spend money against OpenRouter with the default model. Nothing else in \
             this suite does (CLAUDE.md #7). The ceiling and the ledger are \
             `escribass_ai.provider`'s; this run records to {}",
            recording.display()
        );

        // **Either side of the run** (ADR 0022 §4). Read before anything is spent, so the
        // difference afterwards is this run's and nobody else's.
        let ledgered_before = ledger();
        let account_before = counter(&["--account-usage"]);
        println!(
            "before: OpenRouter says ${account_before:.8} all time; the ledger has {} row(s) \
             totalling ${:.8}",
            ledgered_before.len(),
            charged(&ledgered_before)
        );

        let session_run = run("proposal", AT);
        let at: i64 = AT.parse().expect("the clock is milliseconds");
        let manifest = std::sync::Arc::new(
            escribass_core::Manifest::read(MANIFEST).expect("the manifest fixture is readable"),
        );
        let project =
            Project::open(&session_run.directory.0, manifest).expect("a project that opens");
        let mut session = Session::new(
            project,
            Box::new(SeededIds::new(at, TURN_SEED)),
            Box::new(FixedClock(at)),
            Author::Human,
        );

        // No `--transcript`: the sidecar builds the real client, which reads the key from this
        // process's environment and nowhere else (ADR 0020 §4).
        let ai = common::workspace().join("ai");
        let mut sidecar = Assistant::new(vec![
            "uv".to_string(),
            "run".to_string(),
            "--project".to_string(),
            ai.display().to_string(),
            "escribass-ai".to_string(),
            "--record".to_string(),
            recording.display().to_string(),
        ])
        .start()
        .expect("the real sidecar starts");

        // `drive`'s own prompt, so the recording replaces the hand-written transcript rather
        // than sitting beside it as a differently-worded near-miss.
        let turn = sidecar
            .turn(&mut session, "add a lead line over the bass", &[], |_, _| {})
            .expect("the turn answers");
        println!("{:#?}", turn.recorded);
        println!("model that answered: {}", turn.model_id);
        let proposal = session.proposal().expect("the turn left a proposal");
        println!("calls: {:?}", proposal.calls());
        match proposal.patch() {
            Ok(pending) => println!("{}", escribass_core::ops_text(pending.ops())),
            Err(refused) => println!("the proposal will not apply: {refused:?}"),
        }
        // Deliberately **not** applied: what a live run is for is watching a real model compose
        // a proposal, and a golden of a nondeterministic turn is the thing this milestone
        // refuses to write (docs/plan.md, "What M3 will not claim"). Replaying the recording
        // through `drive` is what applies it, and that turn *is* deterministic.
        println!("{}", sidecar.stop());

        // What the recording must be before anybody considers committing it. The key check is
        // first and is not a formality: a recorder that saved what the SDK sends rather than
        // what it was handed would put `Authorization: Bearer …` in `git log` for ever, where
        // it would parse, replay and pass every test that did not read it (M3 trap 10).
        let recorded = std::fs::read_to_string(&recording).expect("the run recorded a file");
        let key = std::env::var("OPENROUTER_API_KEY").expect("checked above");
        assert!(!recorded.contains(&key), "the recording carries the key");
        assert!(!recorded.contains("Authorization"), "the recording carries headers");
        assert!(!recorded.contains("Bearer"), "the recording carries headers");
        let parsed: Value = serde_json::from_str(&recorded).expect("a recording is JSON");
        let exchanges = parsed["exchanges"].as_array().expect("it has exchanges");
        assert!(!exchanges.is_empty(), "it recorded nothing");
        for exchange in exchanges {
            let request = exchange["request"].as_object().expect("a request is the body");
            // The body as `turn.run` built it and nothing around it: were a client, a set of
            // headers or an `api_key` ever to arrive here, this is where it would be seen.
            let mut keys: Vec<&str> = request.keys().map(String::as_str).collect();
            keys.sort_unstable();
            assert_eq!(keys, ["max_tokens", "messages", "model", "temperature", "tools"]);
            assert!(exchange["response"]["id"].is_string(), "a response names its generation");
        }
        println!(
            "recorded {} exchange(s) to {}; note: {}",
            exchanges.len(),
            recording.display(),
            parsed["note"].as_str().unwrap_or_default()
        );

        // -------------------------------------------------------------------
        // The reconciliation (ADR 0022 §4; CLAUDE.md #7)
        // -------------------------------------------------------------------
        //
        // **Performed here, by this code path**, which until 2026-09-24 nothing was: the two
        // functions that read OpenRouter's counter had no caller, and the sentence in §6.1 and
        // in the ADR described something a person had done once by hand (M3 review).
        let ledgered_after = ledger();
        let rows = &ledgered_after[ledgered_before.len()..];
        let ledgered = charged(rows);
        assert!(!rows.is_empty(), "the run spent nothing and wrote no ledger row");
        println!(
            "the ledger gained {} row(s) totalling ${ledgered:.8}; running total ${:.8}",
            rows.len(),
            charged(&ledgered_after)
        );

        // **The account figure lags.** Seconds after the recorded run of 2026-09-24 it had not
        // moved; minutes later it had, by exactly the ledger's total. So it is polled, and
        // every read is printed — a person watching this test watches it arrive. It is
        // **bounded**: an assertion that ignored the lag would be flaky and one that waited
        // for ever would be worse, so if it has not settled inside the bound this fails and
        // says so, with both numbers and the command to check it by hand. A reconciliation
        // that gave up quietly is the check that cannot fail this whole PR is about.
        let deadline = Instant::now() + Duration::from_secs(300);
        let moved = loop {
            let now = counter(&["--account-usage"]) - account_before;
            println!("  OpenRouter has moved by ${now:.8} of ${ledgered:.8} so far");
            if reconciled(now, ledgered).is_ok() || Instant::now() >= deadline {
                break now;
            }
            std::thread::sleep(Duration::from_secs(10));
        };
        match reconciled(moved, ledgered) {
            Ok(said) => println!("reconciled: {said}"),
            Err(why) => panic!(
                "{why}. The counter had five minutes to settle. Check it by hand with \
                 `uv run --project ai escribass-ai --account-usage` before believing either \
                 number (ADR 0022 §4)"
            ),
        }

        // And per call, for the rows whose response carried a generation id: the provider's
        // own per-generation figure against what the ledger charged. This is what explains a
        // difference above rather than leaving it as one number disagreeing with another.
        for row in rows {
            let Some(id) = row["generation_id"].as_str() else { continue };
            let theirs = counter(&["--generation-cost", id]);
            let ours = row["charged_usd"].as_f64().expect("a row is charged something");
            println!("  {id}: OpenRouter ${theirs:.8}, ledger ${ours:.8}");
            if let Err(why) = reconciled(theirs, ours) {
                panic!("generation {id} does not reconcile: {why}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The other transport
// ---------------------------------------------------------------------------

/// Runs a script against `escribass-grpc` instead, and shapes its answers like MCP's.
///
/// "A transport translates and decides nothing" (ADR 0006) is a claim with two
/// implementations, and the only way to check it is to make both answer the same questions. The
/// M0.3 reviews found a bug in exactly this seam — MCP's hand-decoded `apply_patch` read
/// `"dry_run": "true"` as false and applied — so the seam gets a test rather than a comment.
fn run_over_grpc(name: &str, clock: &str) -> Run {
    use escribass_proto::tools::song_tools_client::SongToolsClient;
    use escribass_proto::tools::*;

    let steps = script(name);
    let directory = Scratch::new("determinism", &format!("{name}-grpc"));

    // The port is chosen by binding and letting go, which leaves a moment for something else to
    // take it. `escribass-grpc` prints the address it was *asked* for rather than the one it
    // bound, so `--listen 127.0.0.1:0` cannot be read back; serving a listener directly would
    // need `tokio-stream` as a direct dependency (CLAUDE.md #4) to fix a test-only problem.
    let address = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a free port")
        .local_addr()
        .expect("its address")
        .to_string();

    let child = Command::new(binary("escribass-grpc"))
        .args([
            "--create",
            "--manifest",
            MANIFEST,
            "--seed-ids",
            &format!("{AT}:1"),
            "--fixed-clock",
            clock,
            // Passed rather than defaulted: this binary defaults to `human` and `escribass-mcp`
            // to `model`, so every `provenance.author` would differ if neither said which.
            "--author",
            "model",
            "--listen",
            &address,
        ])
        // The same command the other transport is told, from the same guarded helper: a
        // cross-transport comparison in which one side compiled with a different child would
        // compare two different claims (ADR 0006: a transport decides nothing).
        .args(telling_it_about_a_compiler(name))
        .arg(&directory.0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the server binary runs");
    // `Child` has no `Drop`, so a panic in any step below would leave this server running and
    // still bound to its port — which the next test's bind-and-drop could then be handed.
    let _served = Served(child);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");

    let results = runtime.block_on(async {
        let endpoint = format!("http://{address}");
        let mut client = None;
        for _ in 0..750 {
            if let Ok(connected) = SongToolsClient::connect(endpoint.clone()).await {
                client = Some(connected);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let mut client = client.unwrap_or_else(|| panic!("the server never came up on {endpoint}"));

        let mut answers = Vec::with_capacity(steps.len());
        for (position, step) in steps.iter().enumerate() {
            let id = position + 1;
            let args = step.args.clone();

            /// One arm per RPC. A tool this match does not know panics naming it, rather
            /// than being silently skipped — but only if a script calls it, which is why
            /// `every_implemented_tool_is_scripted` exists as well.
            macro_rules! call {
                ($($name:literal => $method:ident : $request:ty),* $(,)?) => {
                    match step.tool.as_str() {
                        $($name => {
                            let request: $request = serde_json::from_value(args)
                                .unwrap_or_else(|e| panic!("step {id} (`{}`): {e}", step.tool));
                            shaped(&answered(client.$method(request)).await)
                        })*
                        // Built directly rather than through the deserializer: `patch` is
                        // `bytes`, so that route wants base64, and a script writes the RFC 6902
                        // array a model sends. `core/src/mcp.rs` does the same for the same
                        // reason — the asymmetry is the proto's, not a difference in what the
                        // two transports mean.
                        "apply_patch" => {
                            let text = serde_json::to_string_pretty(&args["patch"])
                                .expect("a Value serialises")
                                + "\n";
                            let request = ApplyPatchRequest {
                                patch: text.into_bytes(),
                                dry_run: args.get("dry_run").and_then(Value::as_bool).unwrap_or(false),
                            };
                            shaped(&answered(client.apply_patch(request)).await)
                        }
                        "get_song" => song_shaped(answered(client.get_song(GetSongRequest {})).await),
                        "get_song_at" => {
                            let request: GetSongAtRequest = serde_json::from_value(args).unwrap();
                            song_shaped(answered(client.get_song_at(request)).await)
                        }
                        "get_history" => {
                            history_shaped(answered(client.get_history(GetHistoryRequest {})).await)
                        }
                        // Its own arm, like the reads: an address is not a `ToolResult`.
                        // `content` is `bytes` and arrives base64, which the generated
                        // deserializer decodes — the symmetry `apply_patch` does not have,
                        // because an asset really is opaque binary (`core/src/mcp.rs`).
                        "add_asset" => {
                            let request: AddAssetRequest = serde_json::from_value(args)
                                .unwrap_or_else(|e| panic!("step {id} (`{}`): {e}", step.tool));
                            let answer = answered(client.add_asset(request)).await;
                            json!({"asset_hash": answer.asset_hash})
                        }
                        // Its own arm, like `add_asset`: a render answers with what it
                        // produced (song_tools.proto, `RenderResponse`). Every scripted step
                        // is a dry run, so no engine is started on either transport — what a
                        // suite with no engine build cannot cover, `tests/AGENTS.md` names.
                        "render_export" => {
                            let request: RenderExportRequest = serde_json::from_value(args)
                                .unwrap_or_else(|e| panic!("step {id} (`{}`): {e}", step.tool));
                            render_shaped(&answered(client.render_export(request)).await)
                        }
                        // Its own arm, like `render_export`: a preview answers with where
                        // the transport is (song_tools.proto, `PreviewResponse`). No scripted
                        // step starts one — a dry run, or a refusal for a transport that is
                        // not there — so `event` is null on both transports.
                        "render_preview" => {
                            let request: RenderPreviewRequest = serde_json::from_value(args)
                                .unwrap_or_else(|e| panic!("step {id} (`{}`): {e}", step.tool));
                            preview_shaped(&answered(client.render_preview(request)).await)
                        }
                        unknown => panic!("step {id}: the suite does not know `{unknown}`"),
                    }
                };
            }

            let answer = call! {
                "add_track" => add_track: AddTrackRequest,
                "set_track_instrument" => set_track_instrument: SetTrackInstrumentRequest,
                "add_effect" => add_effect: AddEffectRequest,
                "set_param" => set_param: SetParamRequest,
                "add_clip" => add_clip: AddClipRequest,
                "set_notes" => set_notes: SetNotesRequest,
                "transpose" => transpose: TransposeRequest,
                "quantize" => quantize: QuantizeRequest,
                "add_automation" => add_automation: AddAutomationRequest,
                "set_tempo" => set_tempo: SetTempoRequest,
                "add_section" => add_section: AddSectionRequest,
                "move_section" => move_section: MoveSectionRequest,
                "define_generator" => define_generator: DefineGeneratorRequest,
                "compile_generator" => compile_generator: CompileGeneratorRequest,
                "undo" => undo: UndoRequest,
                "redo" => redo: RedoRequest,
                "create_branch" => create_branch: CreateBranchRequest,
                "switch_branch" => switch_branch: SwitchBranchRequest,
                "delete_branch" => delete_branch: DeleteBranchRequest,
                "merge_branch" => merge_branch: MergeBranchRequest,
            };
            check(step, id, &json!({"result": {"structuredContent": answer}}));
            answers.push(answer);
        }
        answers
    });

    Run { directory, results }
}

/// A server that dies with the harness, however the harness ends.
struct Served(std::process::Child);

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Awaits one call, or fails rather than waiting for ever.
///
/// A server that never answers would otherwise hang the test, and a hung test inherits CI's
/// six-hour default — a failure that costs a runner and reports nothing.
async fn answered<T>(
    call: impl std::future::Future<Output = Result<tonic::Response<T>, tonic::Status>>,
) -> T {
    match tokio::time::timeout(std::time::Duration::from_secs(30), call).await {
        Ok(Ok(response)) => response.into_inner(),
        Ok(Err(status)) => panic!("the call failed: {status}"),
        Err(_) => panic!("the call did not answer within 30s"),
    }
}

/// A `ToolResult` in the shape MCP puts on the wire, so the two can be compared directly.
///
/// The patch is parsed rather than left as bytes, which is what `core/src/mcp.rs` does — and
/// the reason it has to: over MCP `patch` is `bytes` and would otherwise arrive base64-encoded.
fn shaped(result: &escribass_proto::tools::ToolResult) -> Value {
    json!({
        "valid": result.valid,
        "errors": result.errors.iter().map(|e| json!({
            "path": e.path, "rule": e.rule, "message": e.message,
        })).collect::<Vec<_>>(),
        "patch": if result.patch.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&result.patch).expect("the patch is RFC 6902 text")
        },
        "summary": result.summary,
        "entry_id": result.entry_id,
    })
}

/// A `RenderResponse` in the shape MCP puts on the wire (`core/src/mcp.rs`).
///
/// `result` goes through the generated serializer here for the same reason it does there, and
/// this line is half the point: a second hand copy would drop a new field identically on both
/// sides of the comparison, so the parity check would keep passing while both transports lost
/// it (M1 PR 13). `result` is null unless an engine ran, which no scripted step does: every one
/// is a dry run.
fn render_shaped(response: &escribass_proto::tools::RenderResponse) -> Value {
    json!({
        "valid": response.valid,
        "errors": response.errors.iter().map(|e| json!({
            "path": e.path, "rule": e.rule, "message": e.message,
        })).collect::<Vec<_>>(),
        "summary": response.summary,
        "result": response.result,
    })
}

/// A `PreviewResponse` in the shape MCP puts on the wire, `event` through the generated
/// serializer for `render_shaped`'s reason.
fn preview_shaped(response: &escribass_proto::tools::PreviewResponse) -> Value {
    json!({
        "valid": response.valid,
        "errors": response.errors.iter().map(|e| json!({
            "path": e.path, "rule": e.rule, "message": e.message,
        })).collect::<Vec<_>>(),
        "summary": response.summary,
        "event": response.event,
    })
}

/// A song, rendered by the canonical writer and parsed — the same document MCP sends as text.
fn song_shaped(response: escribass_proto::tools::SongResponse) -> Value {
    let song = response.song.expect("a read returns a song");
    let text = escribass_core::to_canonical_json(&song).expect("a song serialises");
    serde_json::from_str(&text).expect("canonical JSON parses")
}

/// The log, in the on-disk shape — which is what MCP sends, and where dropping a field once
/// cost `provenance` from the audit trail.
fn history_shaped(response: escribass_proto::tools::HistoryResponse) -> Value {
    let entries: serde_json::Map<String, Value> = response
        .entries
        .iter()
        .map(|(id, entry)| {
            let text = escribass_core::entry_to_json(entry).expect("an entry serialises");
            (id.clone(), serde_json::from_str(&text).expect("an entry parses"))
        })
        .collect();
    let refs = response.refs.expect("a history has refs");
    json!({"entries": entries, "head": refs.head, "refs": refs.refs})
}

#[test]
fn the_two_transports_answer_the_same_way() {
    // Both dispatch to one `Session`, so the risk is not that the logic differs — it is that a
    // transport reshapes, reclassifies or quietly drops something on the way out. That is not
    // hypothetical: MCP's `get_history` once lost `provenance`, and its hand-decoded
    // `apply_patch` once read `"dry_run": "true"` as false and applied.
    for name in SCRIPTED {
        let over_mcp = run(name, AT);
        let over_grpc = run_over_grpc(name, AT);
        assert_same(
            &format!("`{name}` came out differently over the two transports"),
            &Snapshot::of(&over_mcp),
            &Snapshot::of(&over_grpc),
        );
    }
}

#[test]
fn the_origin_is_the_document_a_replay_starts_from() {
    // Pins what `origin.json` is, so the cross-language replays are checking the same claim
    // this one is: every field present, every id empty, every collection empty.
    let document: Value = serde_json::from_str(&origin()).expect("the origin is JSON");
    assert_eq!(document["id"], json!(""));
    assert_eq!(document["version"], json!(0));
    assert_eq!(document["tracks"], json!({}));
    // Scalars and maps are present at their defaults, which is what makes `replace` legal from
    // the first operation. A message field that is absent stays absent — `tempo_map` and
    // `render_target` arrive as `add` operations in the root entry, not as `replace`.
    assert!(document.get("tempo_map").is_none(), "an unset message is omitted, not defaulted");
}

/// What this suite covers of `render_export`, and what it deliberately does not.
///
/// Every scripted step is a **dry run**: it compiles the song and starts no process, which is
/// the whole of what a machine with no engine build can honestly check (ADR 0006 §3). The
/// `checks` job is one of those — it does not build the engine, and a real render needs the
/// binary, three plugins and forty minutes. What is left over is the engine half: the spawn,
/// the WAV, the hash and the commits. **`tests/renders.rs` (M1 PR 11) owns that**, behind the
/// cargo feature that makes it absent where it cannot run rather than silently passing.
///
/// So the seam itself is what this asserts. Asked for a real render, a server told no engine
/// answers as an operator error and writes nothing — the one outcome that must never be a
/// quiet success, because a dry run and a render that did nothing look identical from here.
#[test]
fn a_real_render_needs_an_engine_and_says_so_rather_than_pretending() {
    let directory = Scratch::new("determinism", "no_engine");
    let wav = directory.0.with_extension("wav");
    let call = |id: usize| {
        json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": "render_export", "arguments": {
                "output_path": wav.display().to_string(), "dry_run": false,
            }}
        })
    };
    let requests = vec![
        json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": "escribass-tests", "version": "1"}
            }
        }),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        call(1),
    ];

    // Told nothing: `engine_unset`. The flag is optional so a process that only edits needs no
    // engine build, and this is what keeps "optional" from meaning "skipped".
    let frames = speak(&["--create", "--manifest", MANIFEST], &directory.0, &requests);
    let refused = frames.iter().find(|f| f["id"] == json!(1)).expect("an answer");
    let message = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("engine_unset"), "expected an operator error, got {refused}");
    assert_eq!(refused["error"]["code"], json!(-32603), "an operator error, not a refusal");
    assert!(!wav.exists(), "nothing was rendered and nothing should have been written");

    // Told wrongly: `engine_missing`. A path that is not there is never searched around.
    let absent = directory.0.join("no-such-engine");
    let frames = speak(
        &["--manifest", MANIFEST, "--engine", &absent.display().to_string()],
        &directory.0,
        &requests[..2].iter().cloned().chain([call(1)]).collect::<Vec<_>>(),
    );
    let refused = frames.iter().find(|f| f["id"] == json!(1)).expect("an answer");
    let message = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("engine_missing"), "expected an operator error, got {refused}");
    assert!(!wav.exists());
}

/// What this suite covers of `render_preview`, and what it cannot.
///
/// A preview's real effect is sound, which no script can observe and no runner here can
/// produce: CI has no sound card (docs/plan.md, M2 trap 13). So the scripts carry what is a
/// function of the document — a dry-run play whose summary is compiled from the plan, and two
/// refusals that are real calls, one of them for a transport that is not there — and the engine
/// half is `core/tests/preview.rs` against a model of the stream, and `tests/renders.rs`'s
/// ignored test on a machine with a device.
///
/// **The seam is what this asserts**, as the test above does for a render: asked to play for
/// real, a server told no engine answers as an operator error, and one told a missing engine
/// says that — never a `valid: true` that played nothing, which a caller could not tell from a
/// preview that did.
#[test]
fn a_real_preview_needs_an_engine_and_says_so_rather_than_pretending() {
    let directory = Scratch::new("determinism", "no_engine_preview");
    let handshake = [
        json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": "escribass-tests", "version": "1"}
            }
        }),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "render_preview", "arguments": {"play": {"start_tick": 0}}}
        }),
    ];

    let frames = speak(&["--create", "--manifest", MANIFEST], &directory.0, &handshake);
    let refused = frames.iter().find(|f| f["id"] == json!(1)).expect("an answer");
    let message = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("engine_unset"), "expected an operator error, got {refused}");
    assert_eq!(refused["error"]["code"], json!(-32603), "an operator error, not a refusal");

    let absent = directory.0.join("no-such-engine");
    let frames = speak(
        &["--manifest", MANIFEST, "--engine", &absent.display().to_string()],
        &directory.0,
        &handshake,
    );
    let refused = frames.iter().find(|f| f["id"] == json!(1)).expect("an answer");
    let message = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("engine_missing"), "expected an operator error, got {refused}");
}

/// The tools whose real effect leaves the document — a file written, sound played — beside the
/// test in this file that asserts their seam instead, because no script can observe the effect.
///
/// A tool scripted **only as dry runs** tests nothing its real call does, and until M2 PR 10 the
/// guard below would have been satisfied by exactly that. So a dry-run-only tool must be one of
/// these, and every one of these must name a test that is here.
const BEYOND_THE_DOCUMENT: [(&str, &str); 2] = [
    ("render_export", "a_real_render_needs_an_engine_and_says_so_rather_than_pretending"),
    ("render_preview", "a_real_preview_needs_an_engine_and_says_so_rather_than_pretending"),
];

#[test]
fn every_implemented_tool_is_scripted() {
    // The `call!` macro panics on a tool it does not know, but only if a script calls one — so
    // an RPC could be implemented, advertised, and never exercised here. This is what makes
    // `tests/AGENTS.md`'s "add it to a script" a rule rather than a suggestion.
    let steps: Vec<Step> = SCRIPTED
        .iter()
        .chain(COMPILES.iter())
        .flat_map(|name| script(name))
        .collect();
    let this_file = include_str!("determinism.rs");

    for tool in escribass_core::call::IMPLEMENTED {
        let calls: Vec<&Step> = steps.iter().filter(|step| step.tool == *tool).collect();
        assert!(!calls.is_empty(), "`{tool}` is implemented and no script calls it");

        let seam = BEYOND_THE_DOCUMENT.iter().find(|(beyond, _)| beyond == tool);
        let only_dry = calls.iter().all(|step| step.args.get("dry_run") == Some(&json!(true)));
        assert!(
            !only_dry || seam.is_some(),
            "`{tool}` is scripted only as dry runs, which exercise nothing its real call does. \
             Script a real call, or add it to BEYOND_THE_DOCUMENT with the test that asserts \
             its seam"
        );
        if let Some((_, test)) = seam {
            assert!(
                this_file.contains(&format!("fn {test}()")),
                "`{tool}` names `{test}` as the test of its seam, and there is no such test"
            );
        }
    }
}

#[test]
fn make_run_opens_against_the_build_manifest_when_there_is_one() {
    // M2 PR 11. `make run` defaulted to the committed manifest fixture, a subset with no Dexed,
    // so a project written against a real build refused to open with `lock_mismatch` on a
    // person's first launch. The default is now the build's manifest where one exists, and the
    // fixture otherwise with a line saying what that costs. Run against the real Makefile from
    // a directory that does or does not hold a build, with no MANIFEST in the environment.
    let default = |with_build: bool| {
        let dir = Scratch::new("determinism", "make");
        std::fs::create_dir_all(dir.0.join("engine/build")).expect("a scratch directory");
        if with_build {
            std::fs::write(dir.0.join("engine/build/manifest.json"), "{}").expect("a manifest");
        }
        let ran = Command::new("make")
            .arg("-s")
            .arg("-C")
            .arg(&dir.0)
            .arg("-f")
            .arg(common::workspace().join("Makefile"))
            .arg("help")
            .env_remove("MANIFEST")
            .output()
            .expect("make runs");
        assert!(ran.status.success(), "{}", String::from_utf8_lossy(&ran.stderr));
        let said = String::from_utf8(ran.stdout).expect("utf-8");
        let chosen = said
            .lines()
            .find_map(|line| line.strip_prefix("MANIFEST = "))
            .expect("`make help` names the manifest")
            .to_string();
        (chosen, said)
    };

    assert_eq!(default(true).0, "engine/build/manifest.json", "a build's manifest, when there is one");
    let (chosen, said) = default(false);
    assert_eq!(chosen, "tests/fixtures/manifest.json", "and the fixture when there is not");
    assert!(said.contains("lock_mismatch"), "saying what the fixture cannot open: {said}");
}

/// Runs one of `checks.yml`'s path-gate steps against a `git` that reports one changed file,
/// and answers whether it decided to skip.
///
/// The step is **run as the workflow has it**, not retyped: its `run:` block is read out of
/// the YAML by the name of its step, so a gate edited in the workflow and not here fails here
/// rather than quietly stopping a job from testing what changed.
#[cfg(unix)]
fn gate_skips(step_name: &str, scratch: &str, changed: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;

    let workflow = std::fs::read_to_string(common::workspace().join(".github/workflows/checks.yml"))
        .expect("the workflow");
    let step = workflow
        .split_once(step_name)
        .and_then(|(_, rest)| rest.split_once("run: |\n"))
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("no gate step named `{step_name}` in checks.yml"));
    let script: String = step
        .lines()
        .take_while(|line| line.is_empty() || line.starts_with("          "))
        .map(|line| format!("{}\n", line.get(10..).unwrap_or("")))
        .collect();

    let dir = Scratch::new("determinism", scratch);
    std::fs::create_dir_all(&dir.0).expect("a scratch directory");
    let git = dir.0.join("git");
    std::fs::write(&git, "#!/bin/sh\ncase \"$1\" in rev-parse) echo base ;; diff) echo \"$CHANGED\" ;; esac\n")
        .expect("a git that reports one change");
    std::fs::set_permissions(&git, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let gate = dir.0.join("gate.sh");
    std::fs::write(&gate, script).expect("the step, as a file");

    let env = dir.0.join("env");
    let ran = Command::new("bash")
        .arg(&gate)
        .env("PATH", format!("{}:{}", dir.0.display(), std::env::var("PATH").unwrap_or_default()))
        .env("CHANGED", changed)
        .env("GITHUB_ENV", &env)
        .env("GITHUB_OUTPUT", dir.0.join("output"))
        .output()
        .expect("bash runs the step");
    assert!(ran.status.success(), "the gate step failed: {}", String::from_utf8_lossy(&ran.stderr));
    std::fs::read_to_string(&env).unwrap_or_default().contains("SKIP=1")
}

#[cfg(unix)]
#[test]
fn the_engine_job_renders_a_change_to_compile_or_to_the_client() {
    // M2 PR 11. The engine job's path gate skipped every `core/`-only pull request, and
    // `renders` and `cross-cpu` inherit its decision — so a change to `compile` or to the
    // engine client, the two files between the document and a WAV, rendered no golden in CI.
    // Written into no test, and so into no pull request, until a review found it.
    let skips = |changed: &str| {
        gate_skips(
            "- name: Does this pull request touch anything the engine is built from?",
            "engine-gate",
            changed,
        )
    };

    // A gate that never skips would pass the two assertions after these, so it is shown able to.
    assert!(skips("core/src/session.rs"), "the rest of `core/` still skips the engine build");
    assert!(skips("docs/plan.md"), "and so does prose");
    // M4 PR 4: Python that no C++ target reads. Its M5 siblings under `compilers/` are not
    // excluded with it, and the assertion after this one is what keeps that true.
    assert!(
        skips("compilers/generative/src/escribass_generative/dsl.py"),
        "the generative compiler is Python the engine is not built from"
    );
    for file in [
        "core/src/render.rs",
        "core/src/engine.rs",
        "compilers/dsp/CMakeLists.txt",
        "compilers/neural/convert.py",
    ] {
        assert!(!skips(file), "a pull request touching only `{file}` skipped the engine, so no golden renders");
    }
}

#[cfg(unix)]
#[test]
fn the_checks_job_runs_for_every_tier_it_has_a_step_for() {
    // M3 trap 17, and M4 trap 6 one directory over: a path gate that lets a directory's
    // changes through to a job with **no step that runs its tests** is the same defect as a
    // gate that skips a job which does have one. The second half — that a step exists — is
    // `checks.yml`'s to show by running; this is the first half, which is the one a later
    // exclusion could silently undo.
    let skips = |changed: &str| {
        gate_skips(
            "- name: Does this pull request touch anything these checks read?",
            "checks-gate",
            changed,
        )
    };

    assert!(skips("docs/plan.md"), "prose alone still skips these checks");
    assert!(skips("engine/src/main.cpp"), "and so does the engine's C++");
    for file in [
        "ai/src/escribass_ai/view.py",
        "compilers/generative/src/escribass_generative/dsl.py",
        "compilers/generative/tests/test_dsl.py",
        "compilers/generative/uv.lock",
    ] {
        assert!(
            !skips(file),
            "`{file}` changed and the checks job skipped, so the step that tests it never ran"
        );
    }
}

#[test]
fn the_staleness_guard_walks_the_sandboxs_sources_too() {
    // **Trap 5**, which is M1 PR 13's hole one child over. The `generators` golden is
    // produced by a Python process out of `compilers/generative/`, and a `uv sync` that was
    // not run passes every byte of it: the child reports the version its *installed*
    // metadata says, so a `pyproject.toml` bumped to `2` and not synced still answers `1`,
    // matches the golden's `lock.json`, and the suite calls it green.
    //
    // Checked here rather than inside the feature-gated module because it needs no `uv`: the
    // question is whether the guard's roots reach the three files, and a stub dated 1970 is
    // older than all of them. The guard itself is `common::generator_command`'s, so every
    // run of the script on either transport goes through it.
    let dir = Scratch::new("determinism", "sandbox-staleness");
    std::fs::create_dir_all(&dir.0).expect("a scratch directory");
    let pretend = dir.0.join("RECORD");
    std::fs::write(&pretend, b"an environment synced before the last edit").expect("a stub");
    std::fs::File::options()
        .write(true)
        .open(&pretend)
        .expect("the stub opens")
        .set_times(std::fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
        .expect("a modification time can be set");

    let roots = common::generator_sources();
    // A root that is not there cannot be newer than anything, so a list of three paths that
    // have been renamed would make the guard pass silently — which is the shape of the hole
    // it exists to close.
    for root in &roots {
        assert!(root.exists(), "`{}` is a staleness root and is not there", root.display());
    }
    let refused = std::panic::catch_unwind(|| {
        refuse_if_older_than_source(
            "the generative compiler's environment",
            &pretend,
            &roots,
            "run `uv sync --locked`",
        )
    })
    .expect_err("an environment synced in 1970 is older than every source it was built from");
    let message = refused
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| refused.downcast_ref::<&str>().map(|s| s.to_string()).unwrap_or_default());
    let named = message
        .split_once("is older than ")
        .and_then(|(_, rest)| rest.split_once(".\n"))
        .map(|(path, _)| common::workspace().join(path));
    assert!(
        named.is_some_and(|path| roots.contains(&path)),
        "the guard must name the source it is older than, and said: {message}"
    );
}

#[test]
fn the_staleness_guard_walks_the_engine_sources_too() {
    // M1 PR 13, B2. `renders.rs` compared the seven commits the engine embeds and called that
    // its provenance check; those are *submodule* HEADs, so editing `engine/src/main.cpp`
    // moved none of them and the golden suite — `bless` included — stayed green against a
    // binary predating its own source. Demonstrated by editing `main.cpp`, not rebuilding, and
    // watching the goldens pass.
    //
    // Checked here rather than in `renders.rs` because it needs no engine, and the suite that
    // does need one is behind a feature whose CI job skips a `core/`-only change.
    let dir = Scratch::new("determinism", "staleness");
    std::fs::create_dir_all(&dir.0).expect("a scratch directory");
    let pretend = dir.0.join("escribass_engine");
    std::fs::write(&pretend, b"an engine built before the last edit").expect("a stub binary");
    std::fs::File::options()
        .write(true)
        .open(&pretend)
        .expect("the stub opens")
        .set_times(std::fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
        .expect("a modification time can be set");

    let refused = std::panic::catch_unwind(|| {
        refuse_if_older_than_source("the engine", &pretend, &engine_sources(), "rebuild it")
    })
    .expect_err("a binary dated 1970 is older than every engine source");
    let message = refused
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| refused.downcast_ref::<&str>().map(|s| s.to_string()).unwrap_or_default());
    // Named, and one of the roots: which one is whichever file was touched last, and since M2
    // PR 11 that can be a `.proto` rather than something under `engine/`.
    let named = message
        .split_once("is older than ")
        .and_then(|(_, rest)| rest.split_once(".\n"))
        .map(|(path, _)| common::workspace().join(path));
    assert!(
        named.is_some_and(|path| engine_sources().iter().any(|root| path.starts_with(root))),
        "the guard must name the engine source it is older than, and said: {message}"
    );

    // M2 PR 11. A binary dated 1970 is older than *any* root, so the check above passes whatever
    // the roots are — and they did not reach the two `.proto` files the engine's CMake generates
    // C++ from. Touching both left the render suite green; touching `main.cpp` did not. So the
    // roots are asserted against what the CMake actually reads, not against a list typed here
    // twice: every `${REPO}/….proto` in `engine/CMakeLists.txt` must lie under one of them.
    let cmake = std::fs::read_to_string(common::workspace().join("engine/CMakeLists.txt")).expect("CMakeLists");
    let protos: std::collections::BTreeSet<&str> = cmake
        .split(|c: char| c.is_whitespace() || c == ')')
        .filter_map(|word| word.strip_prefix("${REPO}/"))
        .filter(|path| path.ends_with(".proto"))
        .collect();
    assert!(
        protos.contains("proto/render.proto") && protos.contains("schema/song.proto"),
        "the engine generates C++ from both protos, and this read found {protos:?}"
    );
    let roots = engine_sources();
    for proto in protos {
        let file = common::workspace().join(proto);
        assert!(
            roots.iter().any(|root| file.starts_with(root)),
            "the engine's CMake generates C++ from `{proto}`, and no staleness root reaches it: a \
             change to it would leave the render suite green against an engine built before it"
        );
    }
}
