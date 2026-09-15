//! The golden-render suite (ADR 0009; `docs/specs.md` §11, §18.2).
//!
//! **Behind the `renders` feature, and absent without it.** This suite needs a built engine and
//! three VST3s, which `cargo test` from the workspace root has no way to produce; a `#[test]`
//! that noticed that and returned early would be the quiet skip M0.4 exists to prevent. A
//! feature is absent where it cannot run and loud where it must — `cargo test -p escribass-tests
//! --features renders` compiles it and then fails, with instructions, if the engine is not there.
//!
//! **Two comparisons, because they are two questions** (M0.4's lesson, one medium over):
//!
//! - *A against B* — the same fixture rendered twice, in two fresh server processes and so two
//!   fresh engine processes (ADR 0008 §2), must produce identical PCM. This catches
//!   nondeterminism: a wall clock read inside a plugin, an unseeded RNG, a scheduling-dependent
//!   summation order.
//! - *Against the golden* — the render must match bytes committed earlier. This catches
//!   **drift**, which the first comparison structurally cannot: a plugin upgrade, a moved pin
//!   or a changed filter coefficient produces the same wrong bytes twice and the two runs agree.
//!
//! **The comparison is over the WAV's `data` chunk and only that** (ADR 0009 §2). JUCE writes a
//! `bext` chunk carrying `OriginationDate` and `OriginationTime`, so the file differs between
//! two runs that produced identical audio — the spike measured three identical PCM payloads and
//! three different file hashes. Chunk order, everything outside `data`, and file length are not
//! compared. The whole WAV is committed anyway, because the file is what gives a reviewer a diff
//! and a listener something to play; it is the comparison that is narrowed, not the artefact.
//!
//! **There is no tolerance, and adding one is not a fix.** A tolerance makes "two CPUs round
//! differently" and "a plugin upgrade changed a filter" the same observation, and telling those
//! apart is the product (ADR 0009 §2, §6; §18.1). A plugin that proves non-deterministic gets a
//! documented per-plugin note in the PR that discovers it, never a global slackening.
//!
//! Every fixture is built by driving the tool API over a real `escribass-mcp` process, like
//! every other song in these suites (CLAUDE.md #2). Nothing here writes a `song.json`.
//!
//! **What this suite does not have to catch.** ADR 0007 §4 splits a render into the plan `core`
//! compiles and the rendering the engine does, and the determinism suite already goldens the
//! plan. So a mismatch here with those plan goldens green is the engine's, and a `core` change
//! that would move a render shows up there first — which is why no fixture here carries a plan
//! golden of its own, and why the engine CI job can leave `core/` out of what triggers it.
//!
//! `ponytail:` the fixtures live in a scratch directory with a run-specific path, so a plan
//! golden here would have to have that path erased from it the way `determinism.rs` erases its
//! asset root. The upgrade, if a render ever disagrees with a plan that looks right, is to
//! commit the plan beside the WAV and normalise the two paths in it.
//!
//! **The bar-17 demo lives here too** (M1 PR 12, at the bottom of this file). It is not a
//! golden — it commits nothing and compares two renders of the same session — but it needs the
//! same engine, the same WAV walker and the same sample-by-sample report, and a second harness
//! for that would be a second thing to keep true.

#[path = "common/mod.rs"]
mod common;
use common::{refuse_if_older_than_source, speak, workspace, Scratch, AT};

use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/renders");

/// Where `.github/workflows/checks.yml` builds the engine, and where CMake puts it.
const ENGINE: &str = "engine/build/escribass_engine_artefacts/Release/escribass_engine";

/// Every fixture, named rather than discovered, and checked against the directory below.
///
/// §11 requires a golden render for **every bundled instrument**; these are the three of §8,
/// plus an audio clip, which ADR 0011 defines as gain, fades *and* time-stretch and which
/// `audio_clip` therefore exercises all three of. Naming them here is what makes "a fixture
/// nobody runs" and "a fixture that vanished" both fail rather than pass quietly.
const NAMES: [&str; 4] = ["audio_clip", "dexed", "sfizz", "surge_xt"];

/// Every component the engine embeds a commit for (`engine/src/provenance.h.in`).
///
/// Named here rather than taken from whatever the engine happened to report, so an engine that
/// stops reporting one fails instead of passing with less to compare (ADR 0008 §5, trap 8).
const COMPONENTS: [&str; 8] = [
    "tracktion_engine",
    "juce",
    "protobuf",
    // The transport, since M2 PR 9. `lock.baseline.json` has had an `engine.grpc` entry since
    // M2 PR 1 and it was inert until this list and the engine job's named it (ADR 0013 §1) —
    // a vendored dependency nothing compares is one that drifts in silence (trap 8).
    "grpc",
    "rubberband",
    "surge_xt",
    "sfizz_ui",
    "dexed",
];

/// Where the engine build is, told by an environment variable or found where CMake puts it.
///
/// `core` is told and never searches (ADR 0008 §2, ADR 0010 §4), and this is a test harness
/// rather than `core`: it looks in exactly one place, the path `.github/workflows/checks.yml`
/// builds to, and what makes that safe is that the engine reports the commits it was compiled
/// from and [`stale`] refuses a build that is not the pinned one. A search that could find the
/// wrong engine is only dangerous when nothing checks which engine it found.
///
/// **Which engine, and how old.** `stale` compares *submodule* commits, so it says the vendored
/// trees are the pinned ones and nothing at all about `engine/src`: editing `main.cpp` and not
/// rebuilding left this suite green, and `bless` behind the same non-check would commit a
/// golden from a binary predating its own source. So the engine is checked against its sources
/// here too, by the same walk `common` uses for the cargo binaries (M1 PR 13).
fn told(variable: &str, default: &str) -> PathBuf {
    let path =
        std::env::var_os(variable).map(PathBuf::from).unwrap_or_else(|| workspace().join(default));
    assert!(
        path.exists(),
        "{} is not there.\n\
         This suite renders through a real engine, which cargo does not build. Run\n\
         `cmake -S engine -B engine/build -G Ninja -DCMAKE_BUILD_TYPE=Release` and\n\
         `cmake --build engine/build --target manifest`, or set {variable}.",
        path.display()
    );
    if variable == "ESCRIBASS_ENGINE" {
        refuse_if_older_than_source(
            "the engine",
            &path,
            &common::engine_sources(),
            "CMake is outside the cargo graph, so nothing rebuilt it for you. Run\n\
             `cmake --build engine/build --target manifest`.",
        );
    }
    path
}

// ---------------------------------------------------------------------------
// Rendering a fixture
// ---------------------------------------------------------------------------

/// One tool call. No `refused` field: a golden fixture that refuses renders nothing, so a
/// refusal here is a failure rather than a claim (`tests/determinism.rs` has the other case).
#[derive(Debug, Deserialize)]
struct Step {
    tool: String,
    #[serde(default)]
    args: Value,
}

/// What one render produced.
struct Rendered {
    /// The whole file, which is what gets committed.
    file: Vec<u8>,
    /// The `data` chunk, which is what gets compared.
    pcm: Vec<u8>,
    format: Format,
    /// The engine's own hash of that same `data` chunk (ADR 0009 §2).
    reported: String,
}

/// One tool call, as `tools/call` takes it.
fn call(tool: &str, args: Value) -> Value {
    json!({"name": tool, "arguments": args})
}

/// Runs a scripted conversation through one `escribass-mcp` process and reads back every WAV it
/// rendered, in the order it rendered them.
///
/// One process for the whole script, which is what makes the bar-17 demo below a *re-render of
/// the same project* rather than two projects that happen to differ. The engine is still a
/// fresh process per render, so nothing carries from one to the next (ADR 0008 §2).
///
/// Every step must succeed. A golden fixture that refuses renders nothing, and so does an edit
/// whose path was mistyped — which is the failure this would otherwise hide behind two
/// identical WAVs.
fn drive(what: &str, project: &Path, steps: &[Value]) -> (Vec<Value>, Vec<Rendered>) {
    let engine = told("ESCRIBASS_ENGINE", ENGINE);
    // The manifest a real build wrote, not `tests/fixtures/manifest.json`: that one is a
    // committed subset with the machine's plugin paths dropped, and an engine handed it would
    // have nothing to load (ADR 0010 §4).
    let manifest = told("ESCRIBASS_MANIFEST", "engine/build/manifest.json");

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
            "jsonrpc": "2.0", "id": position + 1, "method": "tools/call", "params": step
        }));
    }

    let frames = speak(
        &[
            "--create",
            "--manifest",
            &manifest.display().to_string(),
            "--engine",
            &engine.display().to_string(),
            "--seed-ids",
            &format!("{AT}:1"),
            "--fixed-clock",
            AT,
            "--author",
            "model",
        ],
        project,
        &requests,
    );

    let mut answers = Vec::new();
    let mut rendered = Vec::new();
    for (position, step) in steps.iter().enumerate() {
        let id = position + 1;
        let frame = frames
            .iter()
            .find(|frame| frame["id"] == json!(id))
            .unwrap_or_else(|| panic!("{what} step {id} got no answer"));
        let tool = step["name"].as_str().expect("a step names its tool");
        // A missing engine, a plugin that will not instantiate and a crash are all operator
        // errors and arrive as a protocol error rather than as `valid: false` (ADR 0006 §2,
        // ADR 0008 §1), so the message is what a person needs and the assertion prints it whole.
        assert!(frame.get("error").is_none(), "{what} step {id} (`{tool}`) failed: {}", frame["error"]);
        let content = &frame["result"]["structuredContent"];
        assert!(
            content.get("valid") != Some(&json!(false)),
            "{what} step {id} (`{tool}`) was refused: {content}"
        );
        answers.push(content.clone());
        if tool == "render_export" && step["arguments"]["dry_run"] != json!(true) {
            let wav = PathBuf::from(
                step["arguments"]["output_path"].as_str().expect("a render names its output"),
            );
            rendered.push(read_back(what, id, content, &wav));
        }
    }
    (answers, rendered)
}

/// Checks a `render_export` answer and reads the WAV it wrote.
fn read_back(what: &str, id: usize, content: &Value, wav: &Path) -> Rendered {
    let reported = content["result"]["pcm_sha256"]
        .as_str()
        .unwrap_or_else(|| panic!("{what} step {id} rendered and reported no hash: {content}"))
        .to_string();

    // Before a single sample is compared (ADR 0008 §5). A drifted submodule would otherwise
    // arrive as a golden diff, which says the audio changed and not why.
    let commits: BTreeMap<String, String> =
        serde_json::from_value(content["result"]["commits"].clone())
            .unwrap_or_else(|e| panic!("{what}'s commits are not a map: {e}"));
    let drifted = stale(&commits);
    assert!(
        drifted.is_empty(),
        "the engine that rendered {what} is not the one lock.baseline.json pins:\n{}\n\n\
         Rebuild it from the pinned submodules — this is not a golden failure, and blessing \
         one against this engine would commit whatever it happens to produce (ADR 0008 §5).",
        drifted.join("\n")
    );

    let file = std::fs::read(wav).unwrap_or_else(|e| {
        panic!("{what} reported a render and {} is not readable: {e}", wav.display())
    });
    let (format, pcm) = data(&file, &format!("{what}'s render"));
    // The engine's number, checked against a second one computed here: another language,
    // another SHA-256, and another walk of the same file. `asset_hash` is `core`'s hasher for
    // §10's content addressing — borrowed for its bytes-in, hex-out, not for its meaning — so
    // an engine that hashed the wrong span, or a walker here that found the wrong chunk, is a
    // disagreement rather than two mistakes agreeing.
    assert_eq!(
        escribass_core::asset_hash(&pcm),
        reported,
        "{what}: the engine's hash is not of the `data` chunk this suite read (ADR 0009 §2)"
    );
    // A plugin that loaded, was never given the note and rendered silence exits zero and says
    // nothing. Blessing that as a golden would pin the failure (`checks.yml` asserts the same
    // thing of its own renders, for the same reason).
    assert!(pcm.iter().any(|byte| *byte != 0), "{what} rendered silence");
    let _ = std::fs::remove_file(wav);
    Rendered { file, pcm, format, reported }
}

/// Runs a fixture's script through `escribass-mcp` and renders it for real.
///
/// The `render_export` call is appended here rather than written in the script, because its
/// `output_path` is a scratch path that only exists at run time — and `render.proto` requires an
/// absolute one. Everything before it is the fixture.
fn render(name: &str) -> Rendered {
    let directory = Scratch::new("renders", name);
    let wav = directory.0.with_extension("wav");

    let path = PathBuf::from(FIXTURES).join(name).join("script.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let steps: Vec<Step> = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{} is not a script: {e}", path.display()));

    let mut calls: Vec<Value> =
        steps.iter().map(|step| call(&step.tool, step.args.clone())).collect();
    calls.push(call(
        "render_export",
        json!({"output_path": wav.display().to_string(), "dry_run": false}),
    ));

    let (_, mut rendered) = drive(&format!("`{name}`"), &directory.0, &calls);
    assert_eq!(rendered.len(), 1, "`{name}` asked for one render");
    rendered.pop().expect("one render")
}

/// Every component whose compiled-in commit is not the one `lock.baseline.json` pins.
///
/// Trap 8 is M0.4's own defect one language over, and worse: `cargo` does not know the engine
/// exists, let alone that it is stale, and its inputs are submodules, which drift by being left
/// alone. So the engine embeds what it was compiled from (ADR 0008 §5) and this compares it —
/// **naming the component**, which a golden diff cannot do.
///
/// Pure, so the check itself is testable without an engine that is actually stale.
fn stale(reported: &BTreeMap<String, String>) -> Vec<String> {
    let text = std::fs::read_to_string(workspace().join("lock.baseline.json"))
        .expect("lock.baseline.json is readable");
    let lock: Value = serde_json::from_str(&text).expect("lock.baseline.json is JSON");
    let mut wrong = Vec::new();
    for component in COMPONENTS {
        let Some(built) = reported.get(component) else {
            wrong.push(format!("  {component}: the engine reported no commit for it at all"));
            continue;
        };
        // The engine's own two lists: the vendored tree under `engine`, the bundled plugins
        // under `bundled_plugins`. `sfizz` is in neither direction — what is compiled is
        // sfizz-ui's VST3, so `sfizz_ui` is the row that describes the binary (trap 11).
        let pinned = lock["engine"][component]["commit"]
            .as_str()
            .or_else(|| lock["bundled_plugins"][component]["commit"].as_str());
        match pinned {
            None => wrong.push(format!(
                "  {component}: built from {built}, and lock.baseline.json pins nothing by that name"
            )),
            Some(pinned) if pinned != built => {
                wrong.push(format!("  {component}: built from {built}, lock.baseline.json pins {pinned}"))
            }
            Some(_) => {}
        }
    }
    wrong
}

// ---------------------------------------------------------------------------
// The WAV, walked here rather than trusted
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Format {
    channels: u16,
    sample_rate: u32,
    bits: u16,
}

/// The `fmt ` fields and the `data` chunk of a RIFF/WAVE file.
///
/// A walker of its own in Rust, which ADR 0009 §2 notes and which pays for itself twice: the
/// report below needs the sample width to say *which sample* differs, and hashing what this
/// returns computes the number the engine reported a second way, in another language, from
/// another parse of the same file. `core` has no RIFF walker and must not grow one — audio
/// belongs on the engine's side of CLAUDE.md #6 — and this is a test.
fn data(bytes: &[u8], what: &str) -> (Format, Vec<u8>) {
    assert!(
        bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        "{what} is not a RIFF/WAVE file"
    );
    let (mut format, mut found) = (None, None);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().expect("four bytes")) as usize;
        let body = bytes.get(at + 8..at + 8 + size).unwrap_or_else(|| {
            panic!(
                "{what}: chunk {} at {at} claims {size} bytes and the file has {}",
                String::from_utf8_lossy(id),
                bytes.len()
            )
        });
        match id {
            b"fmt " => {
                assert!(size >= 16, "{what}: a `fmt ` chunk of {size} bytes");
                format = Some(Format {
                    channels: u16::from_le_bytes(body[2..4].try_into().expect("two bytes")),
                    sample_rate: u32::from_le_bytes(body[4..8].try_into().expect("four bytes")),
                    bits: u16::from_le_bytes(body[14..16].try_into().expect("two bytes")),
                });
            }
            b"data" => found = Some(body.to_vec()),
            // Everything else is the container, and the container is not deterministic: `bext`
            // carries the clock. Skipped rather than compared (ADR 0009 §2).
            _ => {}
        }
        // Chunks are word-aligned; an odd size is followed by a pad byte that is not in it.
        at += 8 + size + (size & 1);
    }
    let format = format.unwrap_or_else(|| panic!("{what} has no `fmt ` chunk"));
    let found = found.unwrap_or_else(|| panic!("{what} has no `data` chunk"));
    assert!(
        format.bits % 8 == 0 && format.bits > 0 && format.bits <= 32,
        "{what}: {} bits per sample",
        format.bits
    );
    (format, found)
}

/// One sample, little-endian and sign-extended from the format's width.
fn sample(pcm: &[u8], at: usize, width: usize) -> i32 {
    let mut value = 0i32;
    for (position, byte) in pcm[at..at + width].iter().enumerate() {
        value |= (*byte as i32) << (8 * position);
    }
    let unused = 32 - 8 * width as u32;
    (value << unused) >> unused
}

/// Where two `data` chunks differ, over the samples they share.
///
/// Split out from [`differences`] in M1 PR 12, which needs the same scan as numbers rather than
/// as prose: "where did it start" is what the goldens ask, "where did it stop" is what locality
/// asks, and one walk answers both.
#[derive(Debug, Clone, Copy)]
struct Spread {
    /// Index of the first differing sample, interleaved — so `frame * channels + channel`.
    first: usize,
    /// Index of the last. `last - first + 1` is how far the change reaches, `count` how solid it is.
    last: usize,
    count: usize,
    /// Both values at [`Spread::first`], left then right.
    values: (i32, i32),
    /// The largest absolute difference anywhere, in counts of the format's full scale.
    largest: i64,
}

/// Scans two `data` chunks for the samples they disagree on, or `None` if they agree.
///
/// Only over the samples both hold: a length difference is the caller's to report, and it is
/// a different finding from a changed sample.
fn spread(format: Format, left: &[u8], right: &[u8]) -> Option<Spread> {
    let width = (format.bits / 8) as usize;
    let shared = left.len().min(right.len()) / width * width;
    let (mut found, mut count, mut largest) = (None::<Spread>, 0usize, 0i64);
    for at in (0..shared).step_by(width) {
        let (a, b) = (sample(left, at, width), sample(right, at, width));
        if a != b {
            count += 1;
            largest = largest.max((a as i64 - b as i64).abs());
            match &mut found {
                None => {
                    found = Some(Spread {
                        first: at / width,
                        last: at / width,
                        count: 0,
                        values: (a, b),
                        largest: 0,
                    })
                }
                Some(so_far) => so_far.last = at / width,
            }
        }
    }
    found.map(|so_far| Spread { count, largest, ..so_far })
}

/// How two `data` chunks differ, in the terms a person can act on — or `None` when they do not.
///
/// M0.4's harness reports RFC 6902 operations rather than two documents; this is the same rule
/// for audio. "The files differ" over two megabytes is not a finding. The first differing sample
/// with both its values, how many differ and by how much are what separate "one plugin changed a
/// coefficient" from "the whole render moved by a block" (ADR 0009 §2).
fn differences(format: Format, left: &[u8], right: &[u8]) -> Option<String> {
    if left == right {
        return None;
    }
    let width = (format.bits / 8) as usize;
    let frame = width * format.channels.max(1) as usize;
    let mut report = Vec::new();
    if left.len() != right.len() {
        report.push(format!(
            "  different lengths: left holds {} frames and right {} ({} against {} bytes)",
            left.len() / frame,
            right.len() / frame,
            left.len(),
            right.len(),
        ));
    }
    let shared = left.len().min(right.len()) / width * width;
    match spread(format, left, right) {
        Some(found) => {
            let full = 1i64 << (format.bits - 1);
            let channels = format.channels.max(1) as usize;
            report.push(format!(
                "  {} of {} samples differ, first at sample {} \
                 (frame {}, channel {}): left {}, right {}",
                found.count,
                shared / width,
                found.first,
                found.first / channels,
                found.first % channels,
                found.values.0,
                found.values.1,
            ));
            // Where it stops, which is half of what a locality question asks and useful in a
            // golden failure too: a change that reaches the last sample of the render is a
            // different diagnosis from one that ends with a note.
            report.push(format!(
                "  last at sample {} (frame {}), so the change spans {} frames",
                found.last,
                found.last / channels,
                found.last / channels - found.first / channels + 1,
            ));
            report.push(format!(
                "  largest difference {} of {full} full scale ({:.9})",
                found.largest,
                found.largest as f64 / full as f64
            ));
        }
        // Reachable only through the length branch above: one is a prefix of the other.
        None => report.push("  the shorter is a prefix of the longer, sample for sample".to_string()),
    }
    Some(report.join("\n"))
}

// ---------------------------------------------------------------------------
// The goldens
// ---------------------------------------------------------------------------

fn golden(name: &str) -> PathBuf {
    PathBuf::from(FIXTURES).join(name).join("golden.wav")
}

fn updating() -> bool {
    std::env::var("UPDATE_FIXTURES").is_ok()
}

/// Writes a golden and the `.sha256` beside it.
///
/// The whole WAV, and a hash of the `data` chunk alone: a release note carrying a hash that
/// changes with the clock is worse than no hash, and §18.2 publishes this number (ADR 0009 §2).
fn bless(name: &str, rendered: &Rendered) {
    let at = golden(name);
    std::fs::write(&at, &rendered.file).expect("the golden is writable");
    std::fs::write(at.with_extension("wav.sha256"), format!("{}\n", rendered.reported))
        .expect("the hash is writable");
    eprintln!(
        "blessed {name}: {} frames at {} Hz, {}-bit, {} channels, pcm {}",
        rendered.pcm.len() / (rendered.format.bits / 8 * rendered.format.channels.max(1)) as usize,
        rendered.format.sample_rate,
        rendered.format.bits,
        rendered.format.channels,
        rendered.reported,
    );
}

// ---------------------------------------------------------------------------
// The claims
// ---------------------------------------------------------------------------

#[test]
fn every_fixture_is_one_of_the_named_ones() {
    let mut found: Vec<String> = std::fs::read_dir(FIXTURES)
        .expect("the fixture directory is readable")
        .map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    assert_eq!(found, NAMES, "a fixture nobody renders is a fixture nobody trusts");
}

/// A against B: nondeterminism.
///
/// Two fresh server processes, so two fresh engine processes, so two fresh plugin instances —
/// which is the guarantee ADR 0008 §2 buys and the reason a warm smoother cannot leak from one
/// render into the next.
#[test]
fn every_fixture_renders_the_same_twice_in_two_fresh_processes() {
    for name in NAMES {
        let (a, b) = (render(name), render(name));
        assert_eq!(a.format, b.format, "`{name}`: two runs disagree about the format");
        if let Some(report) = differences(a.format, &a.pcm, &b.pcm) {
            panic!(
                "`{name}` rendered twice and the two disagree.\n\
                 Left is the first run, right is the second.\n{report}"
            );
        }
        assert_eq!(a.reported, b.reported, "`{name}`: identical PCM hashed differently");
    }
}

/// Against the golden: drift.
///
/// The half two runs cannot check. A moved pin, an upgraded plugin or a changed coefficient
/// produces the same bytes twice, and only something committed earlier disagrees.
#[test]
fn every_fixture_still_renders_what_was_committed() {
    for name in NAMES {
        let rendered = render(name);
        if updating() {
            bless(name, &rendered);
            continue;
        }
        let at = golden(name);
        let committed = std::fs::read(&at).unwrap_or_else(|e| {
            panic!(
                "`{name}` has no golden at {} ({e}).\n\
                 Write one with `UPDATE_FIXTURES=1 cargo test -p escribass-tests --features renders`, \
                 and read the diff before committing it.",
                at.display()
            )
        });
        let (format, pcm) = data(&committed, &format!("`{name}`'s golden"));
        assert_eq!(format, rendered.format, "`{name}`: the golden's format is not this render's");
        if let Some(report) = differences(format, &pcm, &rendered.pcm) {
            panic!(
                "`{name}` no longer renders what was committed.\n\
                 Left is the golden, right is this run.\n{report}\n\n\
                 This is drift, not flakiness: nothing here has a tolerance, and adding one \
                 would hide the next change too (ADR 0009 §2). If the change is intended — a \
                 moved pin, in the pull request that moves it — \
                 `UPDATE_FIXTURES=1 cargo test -p escribass-tests --features renders` rewrites \
                 the golden, and §17 says the diff is explained in the ADR."
            );
        }
        // The number §18.2 publishes, checked against both the engine's answer and a second
        // hash of the bytes this suite parsed out for itself.
        let committed_hash = std::fs::read_to_string(at.with_extension("wav.sha256"))
            .unwrap_or_else(|e| panic!("`{name}` has a golden and no `.sha256`: {e}"));
        assert_eq!(
            committed_hash.trim(),
            rendered.reported,
            "`{name}`: the committed hash is not what this render reports, and the PCM matched — \
             so the `.sha256` was written from something other than this `data` chunk"
        );
    }
}

/// The comparison, seen to fail.
///
/// A golden test that has only ever passed has not been tested. One sample of one committed
/// golden is moved by one count here, and what comes back has to name the sample rather than
/// say the files differ.
#[test]
fn a_single_changed_sample_is_reported_by_index() {
    let committed = std::fs::read(golden("dexed")).expect("the dexed golden is committed");
    let (format, pcm) = data(&committed, "the dexed golden");
    assert!(differences(format, &pcm, &pcm).is_none(), "a golden differs from itself");

    let mut changed = pcm.clone();
    // Sample 1000: past the fade-in of anything, and inside every fixture's shortest render.
    let at = 1000 * (format.bits / 8) as usize;
    changed[at] = changed[at].wrapping_add(1);
    let report = differences(format, &pcm, &changed).expect("one changed byte is a difference");
    assert!(report.contains("first at sample 1000"), "{report}");
    assert!(report.contains("1 of "), "one sample changed, and the count says so: {report}");
    assert!(report.contains("largest difference 1 "), "{report}");

    // And a truncated render is reported as a length rather than as 24,000 differing samples.
    let report = differences(format, &pcm, &pcm[..pcm.len() / 2]).expect("half a render differs");
    assert!(report.contains("different lengths"), "{report}");
}

/// The provenance check, seen to fail.
///
/// [`stale`] runs before any sample is compared, and its whole value is that it names the
/// component. Proving that needs a doctored map rather than a doctored engine: a build from
/// another commit is not something a test can produce, and the comparison is the same either
/// way (ADR 0008 §5).
#[test]
fn a_stale_engine_is_named_rather_than_arriving_as_a_golden_diff() {
    let text = std::fs::read_to_string(workspace().join("lock.baseline.json")).expect("readable");
    let lock: Value = serde_json::from_str(&text).expect("JSON");
    let honest: BTreeMap<String, String> = COMPONENTS
        .iter()
        .map(|component| {
            let commit = lock["engine"][component]["commit"]
                .as_str()
                .or_else(|| lock["bundled_plugins"][component]["commit"].as_str())
                .unwrap_or_else(|| panic!("lock.baseline.json pins no commit for {component}"));
            (component.to_string(), commit.to_string())
        })
        .collect();
    assert!(stale(&honest).is_empty(), "{:?}", stale(&honest));

    let mut drifted = honest.clone();
    drifted.insert("sfizz_ui".to_string(), "0".repeat(40));
    let report = stale(&drifted);
    assert_eq!(report.len(), 1, "{report:?}");
    assert!(
        report[0].contains("sfizz_ui") && report[0].contains("lock.baseline.json pins"),
        "{report:?}"
    );

    // And an engine that reports less than it used to is not silently agreed with.
    let mut missing = honest.clone();
    missing.remove("rubberband");
    assert!(stale(&missing)[0].contains("no commit for it at all"), "{:?}", stale(&missing));
}

// ---------------------------------------------------------------------------
// The bar-17 demo (§1, §18.2)
// ---------------------------------------------------------------------------
//
// §1 states the product claim: *"Change the bass line in bar 17 and re-render, everything else
// identical" must always be possible*, and §18.2 makes it the canonical demo. Everything above
// this line tests that a render repeats; this tests that an **edit is local**, which is a
// different claim and the one the product is sold on.
//
// **What is asserted and what is measured, because they are not the same half.**
//
// *Before the edit is the falsifiable half, and it is asserted exactly.* If one sample before
// bar 17's first changes, controllability is broken — not the test. There is no tolerance here
// for the same reason there is none in the goldens (ADR 0009 §2).
//
// *After the edit, "only bar 17" is not true and asserting it would be asserting a wish.* A
// note's release tail outlives its note-off, and an instrument is not a pure function of the
// current block. So the spread past the edit is **measured, printed, and bounded only by
// something with a reason**: a difference that never recovered would mean the edit changed the
// instrument's state for the rest of the song, and that is what the ceiling below watches for.
// The numbers this run measured go to stderr, so a reader of a CI log sees them.

/// 960 PPQ (§4.2) in the 4/4 `new_song` writes: one bar is four beats of 960 ticks.
const BAR: i64 = 4 * 960;

/// Bar 17 starts after sixteen whole bars. Bars are one-based on a timeline; ticks are not.
const BAR_17: i64 = 16 * BAR;

/// The fixture runs to bar 18 inclusive, so the edited bar has a bar after it to be local *to*.
const BARS: i64 = 18;

/// The tempo `new_song` writes, which nothing in this script moves (`core/src/session.rs`).
///
/// Named rather than derived: the sample bar 17 falls on is arithmetic over this, and reading
/// it back from the document would make the test agree with whatever the document said.
const BPM: i64 = 120;

/// The bass line: one note a bar, walking, low enough to be a bass part.
const PITCHES: [i64; 4] = [36, 43, 38, 45];

/// What bar 17's note is edited to — an octave up from the 36 the pattern puts there.
const EDITED: i64 = 48;

/// Ids this script mints, named literally the way every determinism script does (`tests/AGENTS.md`).
///
/// Under `--seed-ids` an id is a pure function of how many were minted before it, so writing
/// them out is what makes a change in mint order fail loudly rather than quietly edit a
/// different note. The assertion below checks the note is the one at bar 17 before anything is
/// rendered, which is what makes a literal id safe to write.
const CLIP: &str = "01M1FPMP000000000000000009";
const BAR_17_NOTE: &str = "01M1FPMP00000000000000000T";

/// The plan `core` compiles for a song, as text a line-by-line comparison can use.
///
/// ADR 0007 §4 splits a render in two — the plan `core` compiles and the rendering the engine
/// does — and says a mismatch is diagnosed by comparing the plan first. Doing that here is what
/// turns "the audio changed before bar 17" into either "core moved something" or "the engine
/// did", which are different bugs in different processes.
fn plan_of(song: &Value) -> String {
    let song = escribass_core::from_canonical_json(
        &serde_json::to_string(song).expect("a song value serialises"),
    )
    .expect("get_song answers with canonical JSON");
    // No assets: this fixture is notes and a plugin, and `compile` opens no file anyway.
    let plan = escribass_core::compile(&song, &BTreeMap::new()).expect("the fixture compiles");
    serde_json::to_string_pretty(&plan).expect("a plan serialises")
}

#[test]
fn a_note_edited_in_bar_17_changes_nothing_before_bar_17() {
    let directory = Scratch::new("renders", "bar17");
    // Two paths, never one. PR 11 found that a render over an existing file appended a second
    // RIFF and every reader took the first, so two renders to one path compared a run against
    // itself. The engine replaces its destination properly now; the habit is what made that
    // visible.
    let before_wav = directory.0.with_extension("before.wav");
    let after_wav = directory.0.with_extension("after.wav");

    // One note a bar, keyed so the map's own order is bar order: `add_clip` mints from a
    // `BTreeMap`, so the note at bar 17 is the seventeenth id minted (`core/src/tools.rs`).
    let notes: BTreeMap<String, Value> = (0..BARS)
        .map(|bar| {
            (
                format!("n{bar:02}"),
                json!({
                    "pitch": PITCHES[bar as usize % PITCHES.len()],
                    "start_tick": bar * BAR,
                    // Half a bar, so each note's tail has the rest of its bar to decay in.
                    "length_ticks": BAR / 2,
                    "velocity": 100,
                }),
            )
        })
        .collect();

    let steps = vec![
        call(
            "add_track",
            json!({
                "name": "Bass", "kind": "TRACK_KIND_INSTRUMENT",
                // Dexed: pure FM, no RNG, no runtime dispatch, no resampling — the cheapest
                // deterministic instrument of the three, and this test is about the edit and
                // not about the synthesiser (§8).
                "ref": {"plugin": {"plugin_id": "Digital Suburban/Dexed", "version": "1.0.1"}}
            }),
        ),
        call(
            "add_clip",
            json!({
                "track_id": "01M1FPMP000000000000000006",
                "start_tick": 0, "length_ticks": BARS * BAR,
                "note_clip": {"notes": notes},
            }),
        ),
        call("get_song", json!({})),
        call("render_export", json!({"output_path": before_wav.display().to_string(), "dry_run": false})),
        // The edit, as a model or a UI would make it: one RFC 6902 operation through the tool
        // API, never a write to `song.json` (CLAUDE.md #2). That it goes through the pipeline
        // is the half of the demo the audio cannot show.
        call(
            "apply_patch",
            json!({"patch": [
                {"op": "replace", "path": format!("/clips/{CLIP}/note_clip/notes/{BAR_17_NOTE}/pitch"),
                 "value": EDITED}
            ]}),
        ),
        call("get_song", json!({})),
        call("render_export", json!({"output_path": after_wav.display().to_string(), "dry_run": false})),
    ];

    let (answers, rendered) = drive("the bar-17 demo", &directory.0, &steps);
    let [before, after] = &rendered[..] else { panic!("two renders, one before and one after") };

    // The literal id is the note at bar 17, checked before a sample is read. A change in mint
    // order would otherwise edit some other bar and the test would still pass.
    let note = &answers[2]["clips"][CLIP]["note_clip"]["notes"][BAR_17_NOTE];
    assert_eq!(
        note["start_tick"],
        json!(BAR_17),
        "`{BAR_17_NOTE}` is no longer the note at bar 17 (tick {BAR_17}); \
         ids are minted in order and this script names them literally (tests/AGENTS.md)"
    );
    assert_eq!(answers[5]["clips"][CLIP]["note_clip"]["notes"][BAR_17_NOTE]["pitch"], json!(EDITED));

    // ADR 0007 §4's split, done first: the plan changed in exactly one number, so anything the
    // audio does before bar 17 is the engine's and not `core`'s.
    let (was, now) = (plan_of(&answers[2]), plan_of(&answers[5]));
    assert_eq!(was.lines().count(), now.lines().count(), "the edit changed the plan's shape");
    // Trimmed, because the indentation is the plan's nesting and not the claim.
    let moved: Vec<(&str, &str)> = was
        .lines()
        .zip(now.lines())
        .filter(|(a, b)| a != b)
        .map(|(a, b)| (a.trim(), b.trim()))
        .collect();
    assert_eq!(
        moved,
        vec![(
            format!("\"pitch\": {},", PITCHES[0]).as_str(),
            format!("\"pitch\": {EDITED},").as_str(),
        )],
        "the plan should differ in one note's pitch and nothing else"
    );

    // Bar 17 in samples, under this fixture's tempo map: ticks are musical time and the engine
    // derives sample positions from the tempo map at render time (§4.2). At 120 BPM and 48 kHz
    // that is tick 61440 → frame 1,536,000, exactly 32 seconds in.
    assert_eq!(before.format, after.format, "the two renders disagree about the format");
    let format = before.format;
    let channels = format.channels.max(1) as i64;
    let rate = format.sample_rate as i64;
    let frame_of = |tick: i64| tick * 60 * rate / (BPM * 960);
    let (bar_17_frame, bar_18_frame) = (frame_of(BAR_17), frame_of(BAR_17 + BAR));
    let width = (format.bits / 8) as usize;
    let cut = (bar_17_frame * channels) as usize * width;

    // The falsifiable half. Everything the edit did not reach is bit-identical, asserted whole
    // rather than sampled — and reported through the same walker a golden failure uses, so a
    // violation names the sample rather than saying the prefixes differ.
    if let Some(report) = differences(format, &before.pcm[..cut], &after.pcm[..cut]) {
        panic!(
            "editing a note in bar 17 changed audio *before* bar 17 (frame {bar_17_frame}).\n\
             Left is the render before the edit, right is after it.\n{report}\n\n\
             This is §1's claim failing, not a threshold to loosen: the plan differed in one \
             note's pitch, so whatever moved here moved inside the engine."
        );
    }

    let found = spread(format, &before.pcm, &after.pcm)
        .expect("an edited note changes the audio it plays in");
    assert_eq!(
        before.pcm.len(),
        after.pcm.len(),
        "the edit changed the render's length; §1's claim is about content, and this is not it"
    );
    assert_eq!(
        found.first as i64,
        bar_17_frame * channels,
        "the first differing sample is bar 17's first, or the edit did not land where it was aimed"
    );

    // The measured half. Printed rather than tuned into an assertion — `ponytail:` this is a
    // number this build produced, not a bound the platform promises.
    let last_frame = found.last as i64 / channels;
    let note_off = frame_of(BAR_17 + BAR / 2);
    eprintln!(
        "bar 17 starts at tick {BAR_17}, frame {bar_17_frame} ({} s) of {} frames.\n\
         The edit changed {} of {} samples, from frame {bar_17_frame} to frame {last_frame} \
         — {} frames ({:.4} s) past the edited note's own note-off at frame {note_off}, and \
         {} frames before bar 18 at frame {bar_18_frame}.\n\
         Largest difference {} of {} full scale.",
        bar_17_frame as f64 / rate as f64,
        before.pcm.len() / (width * channels as usize),
        found.count,
        before.pcm.len() / width,
        last_frame - note_off,
        (last_frame - note_off) as f64 / rate as f64,
        bar_18_frame - last_frame,
        found.largest,
        1i64 << (format.bits - 1),
    );

    // ponytail: one bar, because a release tail is bounded and plugin state carried forward is
    // not. Measured on this build the tail runs ~5,800 frames (0.12 s) past the note-off, so
    // this ceiling is two orders of magnitude of headroom and is not a tuned number; what it
    // catches is the failure that matters — an edit whose effect never ends, which is state
    // divergence rather than an instrument decaying. It is deliberately *not* "inside bar 17":
    // that is true of this fixture's arithmetic (a half-bar note leaves half a bar for its
    // tail) and false in general — the same edit with a whole-bar note puts the last differing
    // frame 24,119 frames into bar 18 (measured in M1 PR 12, and §18.2 amended to say so).
    assert!(
        last_frame - note_off < frame_of(BAR),
        "the edit was still changing audio {} frames after the note it edited ended; \
         a release tail is bounded and this is not one",
        last_frame - note_off
    );

    // The check itself, seen to fail: a golden test that has only ever passed has not been
    // tested, and neither has this one. One sample moved by one count *before* bar 17 is
    // exactly the violation the panic above exists for, and it costs no render.
    let mut doctored = before.pcm.clone();
    doctored[cut - width] = doctored[cut - width].wrapping_add(1);
    let report = differences(format, &before.pcm[..cut], &doctored[..cut])
        .expect("a changed sample before bar 17 is a locality violation");
    assert!(report.contains(&format!("first at sample {}", bar_17_frame * channels - 1)), "{report}");
}

// ---------------------------------------------------------------------------
// The fader, measured (M2 PR 6, ADR 0015)
// ---------------------------------------------------------------------------
//
// A lane that compiles into the plan and moves no sample is this PR's silent failure, and a
// golden cannot tell the two apart: a fader that never moved would be blessed as readily as one
// that did. So the fader is **measured against the formula**, the way M1 PR 8 measured the
// audio-clip fades rather than listening to them.
//
// The reference is a render of the same project *without* the lanes. Dividing one render by the
// other cancels the instrument, the note and every rounding either of them did, and leaves the
// two numbers `Mix` holds — which is the whole of what a mix lane is supposed to change:
//
//   left(n)  = plain(n) · g(n) · (1 - p(n))     under `PanLawLinear` (§8)
//   right(n) = plain(n) · g(n) · (1 + p(n))
//
// so `g` is the half-sum of the two channels' ratios and `g·p` the half-difference. Nothing is
// assumed about the reference's own level, and no asset has to be constructed to have one.
//
// **What separates the right answer from the plausible wrong ones.** `LINEAR` on a `gain_db`
// lane is a straight line **in decibels**, because ADR 0002 §8's formula is ours and ADR 0015
// §2's unit is the model's. Two other readings would each render something that sounds like a
// fade, and at this ramp's midpoint the three disagree by more than a factor of four:
//
//   in decibels, which is ours                        10^(-18/20) = 0.126
//   in Tracktion's slider position, two endpoints                 = 0.288
//   in linear amplitude                                           = 0.508
//
// The assertion below is tight enough that only the first passes, which is the point of making
// it a measurement.

/// Where the ramps run, in the 120 bpm and 4/4 `new_song` writes: two seconds.
const RIDE_TICKS: i64 = 3840;
/// The gain ramp's ends, in decibels — clear of the fader's own floor and ceiling (`-100` and
/// `+6`), so what is measured is the curve and not a clamp.
const RIDE_DB: (f64, f64) = (0.0, -36.0);
/// The pan ramp's ends. Short of hard left and right so that neither channel is driven to
/// silence, where a ratio would be a division by the quantiser.
const RIDE_PAN: (f64, f64) = (-0.75, 0.75);
/// Only frames where the reference is at least this much of full scale are measured: a ratio
/// taken where the reference is near a zero crossing is noise about a quotient of two small
/// integers, not a measurement of the fader.
const RIDE_FLOOR: f64 = 0.05;

/// The value our formula gives at `tick` for a `LINEAR` segment between `ends` (ADR 0002 §8).
fn ride(ends: (f64, f64), tick: f64) -> f64 {
    ends.0 + (ends.1 - ends.0) * (tick / RIDE_TICKS as f64)
}

#[test]
fn a_fader_ride_renders_the_curve_the_formula_draws() {
    let directory = Scratch::new("renders", "fader");
    let plain_wav = directory.0.with_extension("plain.wav");
    let rode_wav = directory.0.with_extension("rode.wav");
    let point = |tick: i64, value: f64| json!({"tick": tick, "value": value, "curve": "CURVE_LINEAR"});
    let lane = |param: &str, ends: (f64, f64)| {
        json!({
            // The track's own id, which is what makes this a mix lane rather than a device's:
            // ids are unique across every collection, so one field addresses both (ADR 0015 §1).
            "target": {"device_id": "01M1FPMP000000000000000006", "param": param},
            "points": {"a": point(0, ends.0), "b": point(RIDE_TICKS, ends.1)},
        })
    };

    let steps = vec![
        call(
            "add_track",
            json!({
                "name": "Lead", "kind": "TRACK_KIND_INSTRUMENT",
                // Dexed for the reason the bar-17 demo takes it: this test is about the fader
                // and not about the synthesiser, and Dexed is the deterministic one (§8).
                "ref": {"plugin": {"plugin_id": "Digital Suburban/Dexed", "version": "1.0.1"}}
            }),
        ),
        // One note held for the whole render, so there is signal to divide by everywhere.
        call(
            "add_clip",
            json!({
                "track_id": "01M1FPMP000000000000000006",
                "start_tick": 0, "length_ticks": RIDE_TICKS,
                "note_clip": {"notes": {"a": {"pitch": 60, "start_tick": 0,
                                              "length_ticks": RIDE_TICKS, "velocity": 100}}},
            }),
        ),
        call("render_export", json!({"output_path": plain_wav.display().to_string(), "dry_run": false})),
        call("add_automation", lane("gain_db", RIDE_DB)),
        call("add_automation", lane("pan", RIDE_PAN)),
        call("render_export", json!({"output_path": rode_wav.display().to_string(), "dry_run": false})),
    ];

    let (answers, rendered) = drive("the fader ride", &directory.0, &steps);
    let [plain, rode] = &rendered[..] else { panic!("two renders, one plain and one automated") };
    for (position, what) in [(3, "the gain lane"), (4, "the pan lane")] {
        assert_eq!(answers[position]["valid"], json!(true), "{what} was refused: {}", answers[position]);
    }
    assert_eq!(plain.format, rode.format, "the two renders disagree about the format");

    // The first claim, and the one a plan golden cannot make: the lanes changed samples at all.
    let format = plain.format;
    assert!(
        differences(format, &plain.pcm, &rode.pcm).is_some(),
        "the fader lanes compiled into the plan and moved no sample — which is what a lane that \
         validates, compiles and renders as nothing looks like from every other test"
    );

    let width = (format.bits / 8) as usize;
    let channels = format.channels as usize;
    assert_eq!(channels, 2, "the pan half of this measurement needs two channels");
    let full = (1i64 << (format.bits - 1)) as f64;
    let floor = RIDE_FLOOR * full;
    // A model tick in frames, under the tempo this fixture never changes: 120 bpm and 960 PPQ
    // is 1920 ticks a second (§4.2).
    let per_tick = format.sample_rate as f64 / 1920.0;
    // Where a frame's gain comes from. Tracktion reads a curve once per **sub**-block and holds
    // it, and the sub-block is not `kBlockSize`: `PluginNode::prepareToPlay` sets
    // `max(128, 128 * round(rate / 44100))` for any plugin with automation and processes the
    // render block in pieces that size (tracktion_PluginNode.cpp). That gate is also why the
    // four committed goldens could not move — a plan with no lane leaves it off and the whole
    // block is one call — and 128 frames at 48 kHz is 2.7 ms, four times finer than the block.
    let sub_block = std::cmp::max(128, 128 * (format.sample_rate as f64 / 44100.0).round() as usize);

    let frames = plain.pcm.len().min(rode.pcm.len()) / (width * channels);
    let (mut measured, mut worst_gain, mut worst_pan) = (0usize, 0.0f64, 0.0f64);
    let (mut at_gain, mut at_pan) = (String::new(), String::new());
    let (mut first, mut last) = (frames, 0usize);
    let mut midpoint: Option<(f64, f64)> = None;
    for frame in 0..frames {
        let of = |pcm: &[u8], channel: usize| {
            sample(pcm, (frame * channels + channel) * width, width) as f64
        };
        let (left, right) = (of(&plain.pcm, 0), of(&plain.pcm, 1));
        if left.abs() < floor || right.abs() < floor {
            continue;
        }
        let (gl, gr) = (of(&rode.pcm, 0) / left, of(&rode.pcm, 1) / right);
        let (gain, pan) = ((gl + gr) / 2.0, (gr - gl) / (gr + gl));

        let tick = (frame - frame % sub_block) as f64 / per_tick;
        let expected_gain = 10f64.powf(ride(RIDE_DB, tick) / 20.0);
        let expected_pan = ride(RIDE_PAN, tick);
        measured += 1;
        first = first.min(frame);
        last = frame;
        if midpoint.is_none() && tick >= RIDE_TICKS as f64 / 2.0 {
            midpoint = Some((tick, gain));
        }
        // Relative for the gain, because a fader is a ratio and 0.1% of -30 dB is not 0.1% of
        // 0 dB; absolute for the pan, which is a position on a line and not a scale.
        if ((gain - expected_gain) / expected_gain).abs() > worst_gain {
            worst_gain = ((gain - expected_gain) / expected_gain).abs();
            at_gain = format!(
                "frame {frame} (tick {tick:.1}): {gain:.9} against {expected_gain:.9}, \
                 which is {:.4} dB against {:.4} dB",
                20.0 * gain.log10(),
                ride(RIDE_DB, tick)
            );
        }
        if (pan - expected_pan).abs() > worst_pan {
            worst_pan = (pan - expected_pan).abs();
            at_pan = format!("frame {frame} (tick {tick:.1}): {pan:.9} against {expected_pan:.9}");
        }
    }

    // The numbers this run measured, so a reader of a CI log sees them rather than a pass.
    eprintln!(
        "fader ride: {measured} of {frames} frames above {:.0}% of full scale, \
         from frame {first} to {last}\n  \
         worst gain {worst_gain:.3e} relative, at {at_gain}\n  \
         worst pan  {worst_pan:.3e} absolute, at {at_pan}",
        RIDE_FLOOR * 100.0
    );
    // A measurement of the middle of a ramp is a measurement of one number, so how much of the
    // ramp was reached is asserted too — in decibels, which is what the claim is about. The
    // ends are the instrument's, not the fader's: Dexed's attack is over by frame 671 and its
    // decay falls under the floor around frame 86,000, which still leaves more than 30 dB of
    // ride, and 30 dB is where a curve read in the wrong domain has long since diverged.
    let covered = ride(RIDE_DB, first as f64 / per_tick) - ride(RIDE_DB, last as f64 / per_tick);
    eprintln!("  {covered:.2} dB of the {:.0} dB ride measured", RIDE_DB.0 - RIDE_DB.1);
    assert!(measured > frames / 8, "only {measured} of {frames} frames were above the floor");
    assert!(covered > 30.0, "only {covered:.2} dB of the ride was above the floor");

    // The bounds. What is left after the block quantisation is taken out is the 24-bit
    // quantisation of two renders divided by each other — the reference is at least 5% of full
    // scale, so about 1e-4 — plus the subdivision `faderPieces` leaves, which is 1.1e-6 dB and
    // therefore not what these numbers are made of. A ramp read in slider position or in linear
    // amplitude is out by 1.3 and 3.0 at the midpoint, so the failure this catches is not a
    // near miss.
    assert!(
        worst_gain < 1e-3,
        "the rendered gain is not the curve `LINEAR` in decibels draws: worst {worst_gain:.3e} \
         relative, at {at_gain}"
    );
    assert!(
        worst_pan < 1e-3,
        "the rendered pan is not the line the lane draws: worst {worst_pan:.3e}, at {at_pan}"
    );

    // The comparison, seen to discriminate. A test whose tolerance is wide enough to accept the
    // wrong answer has measured nothing, so the two wrong answers are computed here at the
    // ramp's own midpoint and asserted to be nowhere near what came back.
    let (tick, gain) = midpoint.expect("the ramp's midpoint is above the floor");
    let ours = 10f64.powf(ride(RIDE_DB, tick) / 20.0);
    // Tracktion's fader domain, from `tracktion_AudioUtilities.cpp`: a straight line drawn
    // between the two endpoints' slider positions rather than between their decibels.
    let position = |db: f64| ((db - 6.0) / 20.0).exp();
    let through = tick / RIDE_TICKS as f64;
    let straight_position = position(RIDE_DB.0) + (position(RIDE_DB.1) - position(RIDE_DB.0)) * through;
    let in_position = 10f64.powf((20.0 * straight_position.ln() + 6.0) / 20.0);
    // And the reading that sounds most plausible of all: a straight line in linear amplitude.
    let ends = (10f64.powf(RIDE_DB.0 / 20.0), 10f64.powf(RIDE_DB.1 / 20.0));
    let in_amplitude = ends.0 + (ends.1 - ends.0) * through;
    eprintln!(
        "  at tick {tick:.0}: measured {gain:.6}, in decibels {ours:.6}, \
         in slider position {in_position:.6}, in linear amplitude {in_amplitude:.6}"
    );
    assert!(((gain - ours) / ours).abs() < 1e-3, "{gain} is not {ours}");
    for (wrong, what) in [(in_position, "slider position"), (in_amplitude, "linear amplitude")] {
        assert!(
            (wrong - gain).abs() > gain,
            "a ramp read in {what} would give {wrong}, which this measurement cannot tell from \
             the {gain} it got — the tolerance above is measuring nothing"
        );
    }
}

// ---------------------------------------------------------------------------
// Preview on a device (M2 PR 10)
// ---------------------------------------------------------------------------

/// **An export process cannot be asked to preview, and it is gRPC that refuses** (ADR 0013 §3).
///
/// The mode is which service the process serves: `--render` registers `Render` alone, so a
/// `Preview` stream opened on it is answered `UNIMPLEMENTED` by gRPC's dispatch before a line of
/// the engine runs. That is what keeps an export from ever sharing a process — and a plugin's
/// smoothers — with something that was played (docs/plan.md, M2 trap 6), and it is asserted
/// against the real binary because the claim is about the binary. It needs no device: the
/// process asked is one that never looks for one.
#[test]
fn an_export_process_refuses_a_preview_by_grpcs_own_dispatch() {
    use escribass_proto::render::preview_client::PreviewClient;
    use escribass_proto::render::{PreviewCommand, PreviewStop};
    use std::io::BufRead;

    let engine = told("ESCRIBASS_ENGINE", ENGINE);
    let manifest = told("ESCRIBASS_MANIFEST", "engine/build/manifest.json");
    let mut child = std::process::Command::new(&engine)
        .arg(&manifest)
        .arg("--render")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("the engine starts");
    let mut address = String::new();
    std::io::BufReader::new(child.stdout.take().expect("a stdout"))
        .read_line(&mut address)
        .expect("the engine names its socket");

    let refused = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(async {
            let channel = tonic::transport::Endpoint::from_shared(address.trim().to_string())
                .expect("an address")
                .connect()
                .await
                .expect("the export process is listening");
            let command = PreviewCommand {
                command: Some(escribass_proto::render::preview_command::Command::Stop(PreviewStop {})),
            };
            PreviewClient::new(channel)
                .preview(tonic::codegen::tokio_stream::iter(vec![command]))
                .await
                .map(|_| ())
        });
    let _ = child.kill();
    let _ = child.wait();

    let status = refused.expect_err("an export process served a preview");
    assert_eq!(status.code(), tonic::Code::Unimplemented, "{status}");
}

/// **A preview playing through this machine's audio device — untested wherever this suite runs
/// in CI, and ignored so that it says so every time.**
///
/// Everything a preview does that is not sound is tested without a device: `core`'s half
/// against a model of the stream (`core/tests/preview.rs`), the tool through both transports
/// (`determinism.rs`), and the engine refusing loudly on a runner with no sound card (the engine
/// job). What is left needs a device, and no CI runner has one (docs/plan.md, M2 trap 13): the
/// engine building a live edit from a real plan, its transport moving on the device's clock, a
/// loop wrapping, a seek landing while it plays, and a stopped transport starting again from
/// where it stopped without its plan being built twice.
///
/// It asserts what a transport reports and never what it sounds like — a person with a speaker
/// is the only check on that, and nothing here pretends otherwise. On a machine with a device:
///
/// ```text
/// cargo test -p escribass-tests --features renders -- --ignored a_preview_plays
/// ```
///
/// On Linux the device is ALSA's default PCM, because that is what this JUCE build speaks. A
/// machine whose sound goes through PulseAudio (WSLg included) needs `libasound2-plugins` and a
/// default PCM of `type pulse` (ADR 0013 §4).
#[test]
#[ignore = "needs an audio output device, and CI has none (docs/plan.md, M2 trap 13)"]
fn a_preview_plays_on_this_machines_audio_device() {
    use escribass_proto::render::{PreviewLoop, PreviewSeek, PreviewState, PreviewStop};
    use escribass_proto::tools::render_preview_request::Command;
    use escribass_proto::tools::{PreviewFrom, RenderPreviewRequest};

    let engine = told("ESCRIBASS_ENGINE", ENGINE);
    let manifest = told("ESCRIBASS_MANIFEST", "engine/build/manifest.json");

    // Built through the tool API over a real server, as every fixture is (CLAUDE.md #2), and
    // then opened as the host opens one: a session built as a library (ADR 0012 §3).
    let directory = Scratch::new("renders", "preview");
    let path = PathBuf::from(FIXTURES).join("surge_xt").join("script.json");
    let steps: Vec<Step> = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let calls: Vec<Value> = steps.iter().map(|step| call(&step.tool, step.args.clone())).collect();
    drive("`preview`", &directory.0, &calls);

    let hosts = std::sync::Arc::new(escribass_core::Manifest::read(&manifest).expect("the manifest"));
    let project = escribass_core::Project::open(&directory.0, hosts).expect("the fixture opens");
    let mut session = escribass_core::Session::new(
        project,
        Box::new(escribass_core::SeededIds::default()),
        Box::new(escribass_core::FixedClock(0)),
        escribass_schema::song::Author::Human,
    );
    session.set_engine(escribass_core::Engine::new(&engine, &manifest));

    let mut send = |command: Command, dry_run: bool| {
        let answer = session
            .render_preview(&RenderPreviewRequest { command: Some(command), dry_run })
            .unwrap_or_else(|e| panic!("the preview failed: [{}] {}", e.rule, e.message));
        assert!(answer.valid, "{:?}", answer.errors);
        answer.event.expect("a live transport reports where it is")
    };
    let playing = send(Command::Play(PreviewFrom { start_tick: Some(0) }), false);
    assert_eq!(playing.state(), PreviewState::Playing);
    // A beat long, inside the fixture's one-beat clip, so a wrap is seen within a second.
    send(Command::Loop(PreviewLoop { start_tick: 0, end_tick: 960 }), false);

    // Polled with dry runs, which send nothing (song_tools.proto, `RenderPreviewRequest`). The
    // wait is this test's and bounds only how long it looks: three seconds at the fixture's
    // tempo is six beats, and a loop of one wraps five times in it.
    let (mut furthest, mut wrapped) = (0, false);
    for _ in 0..300 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        let now = send(Command::Stop(PreviewStop {}), true);
        assert_eq!(now.state(), PreviewState::Playing, "a looping transport stopped by itself");
        assert!(now.tick < 960, "the transport left its loop at tick {}", now.tick);
        wrapped |= now.tick < furthest;
        furthest = furthest.max(now.tick);
        if wrapped {
            break;
        }
    }
    assert!(furthest > 0, "the transport never moved: the device is not pulling blocks");
    assert!(wrapped, "the transport reached tick {furthest} and never wrapped round its loop");

    let sought = send(Command::Seek(PreviewSeek { tick: 480 }), false);
    assert!((480..960).contains(&sought.tick), "a seek to 480 answered {}", sought.tick);
    let stopped = send(Command::Stop(PreviewStop {}), false);
    assert_eq!(stopped.state(), PreviewState::Stopped);
    let resumed = send(Command::Play(PreviewFrom { start_tick: None }), false);
    assert_eq!(resumed.state(), PreviewState::Playing);
    eprintln!(
        "preview: furthest tick {furthest} before wrapping, seek answered {}, stopped at {}, \
         resumed at {}",
        sought.tick, stopped.tick, resumed.tick
    );
    drop(session);
}
