//! ADR 0031's export recipe, through the real engine (docs/plan.md, M5 traps 23, 24 and 25).
//!
//! **Behind the `dsp` feature, and absent without it**, for `renders`' reason: it needs `cmaj`
//! built by `compilers/dsp/recipe.py build` and the engine built by its CMake, neither of which
//! cargo can produce, and a test that noticed and returned would be the quiet skip M0.4 exists
//! to prevent.
//!
//! ```text
//! python3 -I compilers/dsp/recipe.py checkout && python3 -I compilers/dsp/recipe.py build
//! cargo test -p escribass-tests --features dsp
//! ```
//!
//! **Why a render and not a scan.** `--scan` instantiates a wrapped Cmajor VST3 and never
//! prepares it, and `prepareToPlay` is where an export `cmaj generate` leaves as it is deadlocks
//! JUCE's headless host for ever; and a scan reads an instrument's parameters, not whether it
//! has a bus a note can arrive on. So every export here is **rendered**, under a deadline, and
//! what is asserted is the sound: the note at its pitch and level, the effect's arithmetic on
//! each channel, and two exports of one processor name each running their own code in one
//! engine process, in both load orders.
//!
//! **What this does not reach.** No song, no `core` tool and no export record: those are M5 PR
//! 8's, and PR 6 is the engine's own inspection. The plans here name each export the way the
//! engine names any plugin today — `vendor/name`, resolved to a path by a manifest written here
//! — which is the identity ADR 0029 §6 retires, used only because the engine has no other yet.
//! The two same-named exports therefore carry two patch names and one processor name: the
//! collision ADR 0029 §6 measured is keyed by the processor's C++ class, not by what the
//! manifest calls it.

#![cfg(unix)]

#[path = "common/mod.rs"]
mod common;
use common::{engine_sources, refuse_if_older_than_source, workspace, Scratch};

use escribass_proto::render::{RenderPlan, RenderResult};
use prost::Message;
use serde_json::{json, Value};
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const ENGINE: &str = "engine/build/escribass_engine_artefacts/Release/escribass_engine";
const RECIPE: &str = "compilers/dsp/recipe.py";

/// Longer than any render here takes by two orders of magnitude (each is under a second on the
/// machines measured), and short enough that the deadlock is a named failure rather than a job
/// that sits until GitHub's timeout. `render_export` gains its own wall clock in PR 8.
const DEADLINE: Duration = Duration::from_secs(120);

/// One second at 120 bpm, which is 48,000 frames at the rate every plan here asks for.
const SECOND: i64 = 1920;
const FRAMES: usize = 48_000;

#[test]
fn the_recipes_exports_play_in_the_engine() {
    let engine = workspace().join(ENGINE);
    assert!(
        engine.exists(),
        "no engine at {}: build it with `cmake --build engine/build --target manifest`",
        engine.display()
    );
    refuse_if_older_than_source("the engine", &engine, &engine_sources(), "Rebuild it.");

    let dir = Scratch::new("dsp", "exports");
    std::fs::create_dir_all(&dir.0).expect("a scratch directory");

    // ---- export, three times, through the recipe as a person runs it ----
    let mut plugins = serde_json::Map::new();
    for (fixture, patch, id) in [
        ("syn", "syn", "Escribass/Recipe Syn"),
        ("two_a", "two", "Escribass/Recipe Two A"),
        ("two_b", "two", "Escribass/Recipe Two B"),
    ] {
        let started = Instant::now();
        let ran = Command::new("python3")
            .arg("-I")
            .arg(workspace().join(RECIPE))
            .arg("export")
            .arg(workspace().join(format!("tests/fixtures/dsp/{fixture}/{patch}.cmajorpatch")))
            .arg(dir.0.join(fixture))
            .output()
            .expect("python3 runs the recipe");
        let said = String::from_utf8_lossy(&ran.stderr);
        assert!(
            ran.status.success(),
            "the recipe did not export {fixture}:\n{said}\n(`recipe.py checkout` and `recipe.py build` first)"
        );
        let bundle = String::from_utf8(ran.stdout).expect("a path").trim().to_string();
        assert!(Path::new(&bundle).is_dir(), "the recipe named {bundle:?}, which is not a bundle");
        eprintln!("{fixture}: exported in {:.1} s ({})", started.elapsed().as_secs_f64(), said.trim());
        plugins.insert(id.to_string(), json!({ "path": bundle }));
    }
    let manifest = dir.0.join("manifest.json");
    std::fs::write(&manifest, json!({ "plugins": plugins }).to_string()).expect("a manifest");

    let render = |name: &str, tracks: Value| -> (Vec<[f64; 2]>, String) {
        let output = dir.0.join(format!("{name}.wav"));
        let plan: RenderPlan = serde_json::from_value(json!({
            "tracks": tracks,
            "master": { "mix": { "gainDb": 0.0, "pan": 0.0 } },
            "tempo": [{ "tick": 0, "bpm": 120.0 }],
            "target": { "kind": "RENDER_KIND_MASTER", "sampleRate": 48000, "bitDepth": 24 },
            "lengthTicks": SECOND,
            "outputPath": output.display().to_string(),
        }))
        .expect("the plan is a RenderPlan");
        let result = render_once(&engine, &manifest, &plan, name);
        (read_wav(&output), result.pcm_sha256)
    };

    // ---- trap 25: the instrument has an event bus, and the note is the note ----
    let note = |params: Value| {
        json!([{
            "instrument": { "instrument": {
                "ref": { "plugin": { "pluginId": "Escribass/Recipe Syn" } },
                "params": params,
            } },
            "mix": { "gainDb": 0.0, "pan": 0.0 },
            "clips": [{ "startTick": 0, "lengthTicks": SECOND, "notes": { "notes": [
                { "pitch": 69, "startTick": 0, "lengthTicks": SECOND, "velocity": 100 }
            ] } }],
        }])
    };
    let (frames, first) = render("syn", note(json!({})));
    let (loudest, crossings) = (peak(&frames, 0), rising_crossings(&frames[4_800..43_200]));
    eprintln!("syn: peak {loudest:.6}, {crossings} rising crossings in 0.8 s");
    // A deaf instrument renders silence and exits 0, which is the failure trap 25 is about.
    assert!((loudest - 0.5).abs() < 0.005, "A4 at the default Level should peak at 0.5, not {loudest}");
    // 352 cycles in 0.8 s, give or take the one a window can cut; A#4 would be 373 and A3 176.
    assert!(
        (350..=354).contains(&crossings),
        "A4 is 352 rising zero crossings in 0.8 s, not {crossings}: the note arrived at another pitch"
    );
    let (_, second) = render("syn-again", note(json!({})));
    assert_eq!(first, second, "one plan, two fresh engine processes, two different PCM hashes");

    // The Level control reached by its VST3 `ParamID` — the endpoint's 1-based handle, `midiIn`
    // being 1 (ADR 0029 §5). A quarter of its 0..1 range is a quarter of full scale.
    let (frames, _) = render("syn-level", note(json!({ "2": 0.25 })));
    let quieter = peak(&frames, 0);
    assert!((quieter - 0.25).abs() < 0.003, "Level at 0.25 should peak at 0.25, not {quieter}");

    // ---- trap 24: two exports of one processor, one engine process, both orders ----
    let dc = dir.0.join("dc.wav");
    write_dc(&dc, 0.5);
    let effect = |id: &str| {
        json!({
            "effects": [{ "effect": { "ref": { "plugin": { "pluginId": id } }, "index": 0 } }],
            "mix": { "gainDb": 0.0, "pan": 0.0 },
            "clips": [{ "startTick": 0, "lengthTicks": SECOND,
                        "audio": { "clip": { "assetHash": "fixture" }, "path": dc.display().to_string() } }],
        })
    };
    for (name, order) in [("a-then-b", ["A", "B"]), ("b-then-a", ["B", "A"])] {
        let tracks = json!(order.map(|v| effect(&format!("Escribass/Recipe Two {v}"))));
        let (frames, _) = render(name, tracks);
        // A passes the left channel at half and B the right at a quarter. B running A's code —
        // what a `GNU_UNIQUE` factory did in the spike — is (0.5, 0); A running B's, (0, 0.25).
        // Past the first samples, where Tracktion's clip reader sits two samples late
        // (docs/specs.md §8, sample-rate conversion).
        let (left, right) = (frames[24_000][0], frames[24_000][1]);
        let worst = frames[100..FRAMES - 100]
            .iter()
            .map(|f| (f[0] - 0.25).abs().max((f[1] - 0.125).abs()))
            .fold(0.0, f64::max);
        eprintln!("{name}: L {left:.6}, R {right:.6}, worst deviation {worst:.2e}");
        assert!(
            worst < 1e-4,
            "{name}: each export should have run its own code, (0.25, 0.125); the render is \
             ({left}, {right}). Two exports of one processor collided in one process (ADR 0029 §6)"
        );
    }
}

/// One plan through `render-once` — `core`'s own `Engine` behind an argv — in a process group of
/// its own, so a deadline can stop the engine it spawned as well as itself.
fn render_once(engine: &Path, manifest: &Path, plan: &RenderPlan, name: &str) -> RenderResult {
    let mut child = Command::new(env!("CARGO_BIN_EXE_render-once"))
        .arg(engine)
        .arg(manifest)
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("render-once starts");
    child.stdin.take().expect("stdin").write_all(&plan.encode_to_vec()).expect("the plan is sent");
    let started = Instant::now();
    while child.try_wait().expect("render-once can be waited on").is_none() {
        if started.elapsed() > DEADLINE {
            // `--`, or procps' `kill` reads the negative group id as an option and kills nothing
            // — found the way it would be found, by a deadlocked demo that outlived its deadline.
            let _ = Command::new("kill").args(["-s", "KILL", "--", &format!("-{}", child.id())]).status();
            let _ = child.wait();
            panic!(
                "{name}: the engine did not answer in {DEADLINE:?}. An export whose CLAP \
                 `activate` notifies its host deadlocks JUCE's headless host in `prepareToPlay` — \
                 decision 9's second edit (docs/plan.md, M5 trap 23)"
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let done = child.wait_with_output().expect("render-once's output");
    assert!(
        done.status.success(),
        "{name}: the render failed: {}",
        String::from_utf8_lossy(&done.stderr)
    );
    let result = RenderResult::decode(&done.stdout[..]).expect("a RenderResult");
    assert_eq!(result.pcm_sha256.len(), 64, "{name}: an answer with no PCM hash is no answer");
    result
}

/// The `data` chunk of a 24-bit stereo WAV, as frames of two samples in `-1..1`.
fn read_wav(path: &Path) -> Vec<[f64; 2]> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        if &bytes[at..at + 4] == b"data" {
            let data = &bytes[at + 8..at + 8 + size];
            let sample = |i: usize| {
                let raw = i32::from_le_bytes([0, data[i], data[i + 1], data[i + 2]]) >> 8;
                raw as f64 / 8_388_608.0
            };
            let frames: Vec<[f64; 2]> = (0..size / 6).map(|n| [sample(n * 6), sample(n * 6 + 3)]).collect();
            assert_eq!(frames.len(), FRAMES, "{}: one second at 48 kHz", path.display());
            return frames;
        }
        at += 8 + size + (size & 1);
    }
    panic!("{} has no data chunk", path.display());
}

fn peak(frames: &[[f64; 2]], channel: usize) -> f64 {
    frames.iter().map(|f| f[channel].abs()).fold(0.0, f64::max)
}

fn rising_crossings(frames: &[[f64; 2]]) -> usize {
    frames.windows(2).filter(|w| w[0][0] < 0.0 && w[1][0] >= 0.0).count()
}

/// One second of a constant, mono, 24-bit, at the render's rate so nothing resamples.
fn write_dc(path: &PathBuf, level: f64) {
    let sample = ((level * 8_388_607.0).round() as i32).to_le_bytes();
    let data: Vec<u8> = (0..FRAMES).flat_map(|_| sample[..3].to_vec()).collect();
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&48_000u32.to_le_bytes());
    wav.extend_from_slice(&(48_000u32 * 3).to_le_bytes());
    wav.extend_from_slice(&3u16.to_le_bytes());
    wav.extend_from_slice(&24u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
    wav.extend_from_slice(&data);
    std::fs::write(path, wav).expect("the DC asset");
}
