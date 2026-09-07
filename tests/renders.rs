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

#[path = "common/mod.rs"]
mod common;
use common::{speak, Scratch, AT};

use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/renders");

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
const COMPONENTS: [&str; 7] = [
    "tracktion_engine",
    "juce",
    "protobuf",
    "rubberband",
    "surge_xt",
    "sfizz_ui",
    "dexed",
];

fn workspace() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("a workspace root")
}

/// Where the engine build is, told by an environment variable or found where CMake puts it.
///
/// `core` is told and never searches (ADR 0008 §2, ADR 0010 §4), and this is a test harness
/// rather than `core`: it looks in exactly one place, the path `.github/workflows/checks.yml`
/// builds to, and what makes that safe is that the engine reports the commits it was compiled
/// from and [`stale`] refuses a build that is not the pinned one. A search that could find the
/// wrong engine is only dangerous when nothing checks which engine it found.
fn told(variable: &str, default: &str) -> PathBuf {
    let path = std::env::var_os(variable).map(PathBuf::from).unwrap_or_else(|| workspace().join(default));
    assert!(
        path.exists(),
        "{} is not there.\n\
         This suite renders through a real engine, which cargo does not build. Run\n\
         `cmake -S engine -B engine/build -G Ninja -DCMAKE_BUILD_TYPE=Release` and\n\
         `cmake --build engine/build --target manifest`, or set {variable}.",
        path.display()
    );
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

/// Runs a fixture's script through `escribass-mcp` and renders it for real.
///
/// The `render_export` call is appended here rather than written in the script, because its
/// `output_path` is a scratch path that only exists at run time — and `render.proto` requires an
/// absolute one. Everything before it is the fixture.
fn render(name: &str) -> Rendered {
    let directory = Scratch::new("renders", name);
    let wav = directory.0.with_extension("wav");
    let engine = told("ESCRIBASS_ENGINE", "engine/build/escribass_engine_artefacts/Release/escribass_engine");
    // The manifest a real build wrote, not `tests/fixtures/manifest.json`: that one is a
    // committed subset with the machine's plugin paths dropped, and an engine handed it would
    // have nothing to load (ADR 0010 §4).
    let manifest = told("ESCRIBASS_MANIFEST", "engine/build/manifest.json");

    let path = PathBuf::from(FIXTURES).join(name).join("script.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let steps: Vec<Step> = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{} is not a script: {e}", path.display()));

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
    let last = steps.len() + 1;
    requests.push(json!({
        "jsonrpc": "2.0", "id": last, "method": "tools/call",
        "params": {"name": "render_export", "arguments": {
            "output_path": wav.display().to_string(), "dry_run": false,
        }}
    }));

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
        &directory.0,
        &requests,
    );
    let answer = |id: usize| -> &Value {
        frames.iter().find(|frame| frame["id"] == json!(id)).unwrap_or_else(|| {
            panic!("`{name}` step {id} got no answer")
        })
    };

    for (position, step) in steps.iter().enumerate() {
        let frame = answer(position + 1);
        let content = &frame["result"]["structuredContent"];
        assert!(
            frame.get("error").is_none() && content.get("valid") != Some(&json!(false)),
            "`{name}` step {} (`{}`) did not succeed: {frame}",
            position + 1,
            step.tool
        );
    }
    // A missing engine, a plugin that will not instantiate and a crash are all operator errors
    // and arrive as a protocol error rather than as `valid: false` (ADR 0006 §2, ADR 0008 §1),
    // so the message is what a person needs and the assertion prints it whole.
    let frame = answer(last);
    assert!(frame.get("error").is_none(), "`{name}` did not render: {}", frame["error"]);
    let content = &frame["result"]["structuredContent"];
    assert_eq!(content["valid"], json!(true), "`{name}` was refused: {content}");
    let reported = content["result"]["pcm_sha256"]
        .as_str()
        .unwrap_or_else(|| panic!("`{name}` rendered and reported no hash: {content}"))
        .to_string();

    // Before a single sample is compared (ADR 0008 §5). A drifted submodule would otherwise
    // arrive as a golden diff, which says the audio changed and not why.
    let commits: BTreeMap<String, String> = serde_json::from_value(content["result"]["commits"].clone())
        .unwrap_or_else(|e| panic!("`{name}`'s commits are not a map: {e}"));
    let drifted = stale(&commits);
    assert!(
        drifted.is_empty(),
        "the engine that rendered `{name}` is not the one lock.baseline.json pins:\n{}\n\n\
         Rebuild it from the pinned submodules — this is not a golden failure, and blessing \
         one against this engine would commit whatever it happens to produce (ADR 0008 §5).",
        drifted.join("\n")
    );

    let file = std::fs::read(&wav).unwrap_or_else(|e| panic!("`{name}` reported a render and {} is not readable: {e}", wav.display()));
    let (format, pcm) = data(&file, &format!("`{name}`'s render"));
    assert_eq!(
        escribass_core::asset_hash(&pcm),
        reported,
        "`{name}`: the engine's hash is not of the `data` chunk this suite read (ADR 0009 §2)"
    );
    // A plugin that loaded, was never given the note and rendered silence exits zero and says
    // nothing. Blessing that as a golden would pin the failure (`checks.yml` asserts the same
    // thing of its own renders, for the same reason).
    assert!(pcm.iter().any(|byte| *byte != 0), "`{name}` rendered silence");
    let _ = std::fs::remove_file(&wav);
    Rendered { file, pcm, format, reported }
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
            None => wrong.push(format!("  {component}: built from {built}, and lock.baseline.json pins nothing by that name")),
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
            panic!("{what}: chunk {} at {at} claims {size} bytes and the file has {}", String::from_utf8_lossy(id), bytes.len())
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
    assert!(format.bits % 8 == 0 && format.bits > 0 && format.bits <= 32, "{what}: {} bits per sample", format.bits);
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
    let (mut first, mut differing, mut largest) = (None, 0usize, 0i64);
    for at in (0..shared).step_by(width) {
        let (a, b) = (sample(left, at, width), sample(right, at, width));
        if a != b {
            differing += 1;
            largest = largest.max((a as i64 - b as i64).abs());
            first.get_or_insert((at / width, a, b));
        }
    }
    match first {
        Some((index, a, b)) => {
            let full = 1i64 << (format.bits - 1);
            report.push(format!(
                "  {differing} of {} samples differ, first at sample {index} \
                 (frame {}, channel {}): left {a}, right {b}",
                shared / width,
                index / format.channels.max(1) as usize,
                index % format.channels.max(1) as usize,
            ));
            report.push(format!(
                "  largest difference {largest} of {full} full scale ({:.9})",
                largest as f64 / full as f64
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
            panic!("`{name}` rendered twice and the two disagree.\nLeft is the first run, right the second.\n{report}");
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
    assert!(report[0].contains("sfizz_ui") && report[0].contains("lock.baseline.json pins"), "{report:?}");

    // And an engine that reports less than it used to is not silently agreed with.
    let mut missing = honest.clone();
    missing.remove("rubberband");
    assert!(stale(&missing)[0].contains("no commit for it at all"), "{:?}", stale(&missing));
}
