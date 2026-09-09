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

    let frames = speak(
        &[
            "--create",
            "--manifest",
            MANIFEST,
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
    for name in ["every_tool", "refusals", "branches", "render", "undo"] {
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
    for name in ["every_tool", "refusals", "branches", "render", "undo"] {
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

#[test]
fn every_implemented_tool_is_scripted() {
    // The `call!` macro panics on a tool it does not know, but only if a script calls one — so
    // an RPC could be implemented, advertised, and never exercised here. This is what makes
    // `tests/AGENTS.md`'s "add it to a script" a rule rather than a suggestion.
    let scripted: std::collections::BTreeSet<String> =
        ["every_tool", "refusals", "branches", "render", "undo"]
        .iter()
        .flat_map(|name| script(name))
        .map(|step| step.tool)
        .collect();

    for tool in escribass_core::call::IMPLEMENTED {
        assert!(scripted.contains(*tool), "`{tool}` is implemented and no script calls it");
    }
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
    assert!(
        message.contains("engine/"),
        "the guard must name the engine source it is older than, and said: {message}"
    );
}
