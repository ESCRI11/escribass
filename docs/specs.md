# Platform specification: AI-driven, code-defined music creation

Status: draft v0.4 · Audience: LLM coding agents and human contributors · Last updated: 2026-09-02

This document is the source of truth for architecture decisions. Agents working on any part of the system must read the sections marked **[MUST]** before writing code. Where a decision is still open it is marked **[OPEN]**; do not resolve open decisions unilaterally.

---

## 1. Purpose and positioning

Build an open-source desktop application (macOS, Windows, Linux) that produces **complete songs** from a **typed, deterministic song model** that both humans and AI agents can edit.

The product is not a prompt-to-audio generator. Its differentiator is **controllability**: every element of a song (notes, arrangement, instruments, effects, mix) is explicit, addressable, editable, and reproducible. "Change the bass line in bar 17 and re-render, everything else identical" must always be possible.

Non-goals for v1: live performance / low-latency live coding, recording audio input, collaboration over network, mobile.

## 2. Guiding principles [MUST]

1. **The song model is the only source of truth.** UI views, code views, and AI edits are projections of, or patches to, the model. Nothing else is persisted as authoritative state.
2. **Determinism.** Same project file + same pinned versions → bit-identical rendered audio. Every source of randomness carries an explicit seed stored in the project.
3. **AI edits through a validated tool API, never free text into the file.** LLMs call typed tools; the core validates, computes a diff, and applies or rejects. Free-form code (patterns, DSP) only enters through compilers that validate before the result touches the model.
4. **Flat events are primary; generators are optional.** A project with no generative code is a first-class project. Generative code compiles down to flat events.
5. **Everything downstream of the model is replaceable.** Renderers, instruments, AI providers, and UI are behind interfaces; none of them may leak into the schema.
6. **Text-first, git-friendly persistence.** Projects are readable, diffable text plus content-addressed binary assets.

## 3. System overview

Five tiers, top to bottom:

| Tier | Component | Language / tech | Process |
|---|---|---|---|
| 1 | Desktop UI | Tauri shell, React frontend, CodeMirror 6, canvas/WebGL timeline | `app` |
| 1 | AI orchestrator | Python sidecar: LLM loop (OpenRouter in v1), symbolic music models, analysis, tool definitions | `ai` |
| 2 | Song model core | Rust library: schema, validation, tool API, JSON Patch, project store, DAWproject I/O | `app` (embedded in Tauri host) |
| 3 | Generative compiler | Python DSL → seeded flat events (sandboxed) | `ai` |
| 3 | DSP compiler | Cmajor source (primary) or Faust import → CLAP → VST3 via clap-wrapper | `engine` (Cmajor JIT for editing, export for persistence) |
| 3 | Neural runtime | ONNX Runtime hosting RAVE/DDSP/codec models, wrapped as CLAP | `engine` |
| 4 | Render engine | Tracktion Engine (C++), VST3 host (CLAPs wrapped), bundled OSS instruments, real-time preview, offline bit-exact render | `engine` |
| 5 | Outputs | Preview audio, WAV/stems export, DAWproject export | `engine` |

Inter-process communication: `app` ↔ `ai` over gRPC; `app` ↔ `engine` over gRPC (control and transport only). Preview audio plays directly from the `engine` process to the audio device; audio is never streamed over IPC. All three processes ship in one installer and are supervised by `app`.

That is the steady state, reached at M2. In M1 there is no `app`: the `Render` service is **defined** in `/proto` so `buf breaking` guards it, and the engine is driven by `core` as a fresh subprocess per render over stdio — one `RenderPlan` in, one `RenderResult` out. M2 implements the gRPC server and deletes the stdio path rather than keeping both (ADR 0008 §1).

## 4. Song model [MUST]

### 4.1 Schema definition

- Defined once in **Protobuf** (`schema/song.proto`), versioned with `schema_version`.
- Generated types for Rust (core), C++ (engine), TypeScript (UI), Python (AI). Hand-written model types in any language are forbidden.
- Canonical serialization for the project file is **pretty-printed JSON** derived from the proto (stable key order). Binary proto is used on the wire only.

### 4.2 Layers

Each layer materialises into the layer below. Layer 2 + layer 3 together are always sufficient to render.

**Layer 1 — Generative (optional)**
- `Generator { id, kind (strudel | python | …), source, seed: u64, toolchain_version, target: TrackId | ClipId, params }`
- `FormRule { id, sections order, transitions, target_length }`
- Compiles into layer-2 entities; compiled output is stored alongside the source so the project renders without re-running the compiler.

**Layer 2 — Composition**
- Time base: integer ticks, **960 PPQ**. No floats for musical time. Absolute sample positions are derived at render time from the tempo map.
- `TempoMap { events: [{tick, bpm}] }`, `TimeSignatureMap`, `Section { id, name, start_tick, end_tick }`, `Marker`.
- `Track { id, name, kind (instrument | audio | bus | master), instrument: InstrumentRef, fx_chain: [EffectRef], routing, mix: {gain, pan, mute, solo} }`.
- `Clip { id, track_id, start_tick, length_ticks, loop, content: NoteClip | AudioClip }`.
- `Note { id, pitch (MIDI 0–127, microtonal offset optional), start_tick (clip-relative), length_ticks, velocity, per-note expression: [{param, value}] }`.
- `Automation { id, target: ParamRef, points: [{tick, value, curve}] }`.

**Layer 3 — Sound**
- `Instrument { id, kind: plugin | faust | cmajor | neural | sampler, ref (plugin id + version | source hash | model hash), state (serialized), params }`.
- `Effect { … same shape … }`.
- `SampleMap` (SFZ-compatible) for samplers.
- `Routing { sends, sidechains }`, `RenderTarget { master | stems, sample_rate, bit_depth, dither }`.

### 4.3 Cross-cutting fields on every entity

- `id`: stable, globally unique, never reused (ULID).
- `provenance`: `{ author: human | model, model_id?, prompt_id?, tool_call_id?, created_at }`.
- `version`: monotonically increasing per entity for optimistic concurrency.

### 4.4 Invariants enforced by the validator

- No overlapping clips on the same track unless `track.allow_overlap`.
- Every `InstrumentRef`/`EffectRef` resolves to a known plugin/source/model with pinned version.
- Every `Generator` has a non-null `seed` and `toolchain_version`.
- Notes inside clip bounds; ticks non-negative; tempo > 0.
- Automation targets resolve to real parameters of the referenced instrument/effect.
- Total render length finite and derivable from layer 2 alone.

## 5. Edit model and tool API [MUST]

- All mutations are **JSON Patch** (RFC 6902) operations against the canonical JSON. The patch log is the undo/redo history and the audit trail. The log is a **DAG**: every entry names its parent, and named refs in `refs.json` are branches. `HEAD` always names a ref. See ADR 0001.
- The core exposes a **typed tool API** (gRPC service `SongTools`) used by the UI and the AI identically. Representative tools:
  - `add_track`, `set_track_instrument`, `add_effect`, `set_param`
  - `add_clip`, `set_notes`, `transpose`, `quantize`, `add_automation`
  - `set_tempo`, `add_section`, `move_section`, `set_form`
  - `define_generator`, `compile_generator`, `define_instrument_source`, `compile_instrument`
  - `render_preview`, `render_export`
  - `create_branch`, `switch_branch`, `delete_branch`, `merge_branch`
- Every tool supports `dry_run=true` returning `{ valid, errors[], patch, summary, entry_id }` without applying. `dry_run` is the first half of the apply path, never a second implementation of it (ADR 0006 §3). A tool that produces no patch answers with its own message instead — `add_asset` with the address it wrote, `render_export` with the hash the engine reported — because `patch` and `entry_id` are then permanently empty (ADR 0006 §1, extended).
- Every tool call is validated against §4.4 before apply. Invalid calls return structured errors the LLM can act on. An invalid call is a result, not a transport failure — §6's retry loop has to be able to see it (ADR 0006 §2).
- **Undo appends an inverse entry; it never rewinds a ref.** Rewinding would decrement entity `version`, and §4.3's optimistic concurrency needs it monotonic (ADR 0005 §4).
- Entity `version` is bumped by core inside the commit pipeline, before the recorded patch is derived, so the bump is *in* the entry (ADR 0005 §1).
- Agents never read or write the project file directly. Use the tool API.

## 6. AI orchestrator (`ai` process)

- Python 3.12+, `grpcio`, `pydantic` models generated from the proto.
- Responsibilities:
  1. **LLM loop**: system prompt + song summary + tool schemas → tool calls → dry-run → apply. On validation error, feed the error back and retry (max 3). Provider-agnostic behind an OpenAI-compatible client abstraction; **v1 uses OpenRouter** so models can be switched by config. Local inference (llama.cpp, vLLM, Ollama) is a config change, not a code change, but no local LLM ships in v1.
  2. **Symbolic generation**: MIDI-domain models for melody, harmony, drums, variation. Output is always normalised to layer-2 entities via tools.
  3. **Analysis**: key/chord/structure detection, tempo estimation, stem separation (Demucs-class), reference-track structure extraction.
  4. **Code generation for compilers**: patterns (layer 1) and DSP sources (layer 3), always submitted through `compile_*` tools, never applied unvalidated.
- The orchestrator has **no direct audio path** and never blocks the UI; long tasks are jobs with progress.

## 7. Compilers

### 7.1 Generative compiler
- Inputs: `Generator.source`, `seed`, `toolchain_version`, current tempo/time-signature maps.
- Backend (decided): a small **Python DSL** (Pydantic-validated, sandboxed subprocess, no network, no filesystem, wall-clock disabled) running inside the `ai` process. Chosen for LLM friendliness and to avoid embedding a JS runtime and AGPL Strudel code in the core. Strudel may be added later as a second `Generator.kind`.
- Output: deterministic list of layer-2 notes/automation for the target clip/track. Must be a pure function of inputs; any wall-clock or unseeded randomness is a bug.

### 7.2 DSP compiler
- Primary language (decided): **Cmajor**. Faust is supported as an import path (Faust → C++ → CLAP), not as an authoring language in the UI.
- Two modes:
  - *Editing loop*: Cmajor **JIT** inside the `engine` process (libCmajPerformer) so AI- or user-authored instruments load instantly without a build step.
  - *Persistence*: `cmaj generate --target=clap` exports a CLAP, which is then wrapped to **VST3 via clap-wrapper**; the wrapped binary and its parameter manifest are content-hashed into `assets/` and referenced by hash from the project. Exported binaries are what `lock.json` pins and what golden tests render.
- Compilation and export run in a subprocess with a timeout; compiler diagnostics are returned to the caller (and therefore to the LLM).

### 7.3 Neural runtime
- ONNX Runtime; models are content-hashed and pinned in the project. Real-time-capable models (RAVE, DDSP-style) are wrapped as CLAP plugins. Non-real-time models live in `ai`.

## 8. Render engine (`engine` process)

- **Tracktion Engine** (C++, GPL/commercial, built on JUCE 8 AGPL) provides timeline, tempo map, tracks, clips, automation, VST3 hosting, real-time playback, and offline rendering.
- Plugin format policy: the engine hosts **VST3 only** in v1. Everything produced in-house (Cmajor/Faust instruments, neural wrappers) is emitted as CLAP and wrapped to VST3 with clap-wrapper. Native CLAP hosting is a later optimisation, never a dependency.
- Time-stretch: Rubber Band (GPL). Elastique is not used. Its configuration is pinned, not defaulted — offline processing, the R3 engine, threading disabled, and every other option group written out explicitly — because a pinned version of a phase vocoder is not a pinned output (ADR 0011 §3). It is built from upstream's own `single/RubberBandSingle.cpp`, which is one source file with no external FFT and nothing to configure: the full build system would pick an FFT from whatever the build machine has installed, and that choice changes the output as surely as an option does.
- Receives a **`RenderPlan`**: the materialised layer 2 + layer 3 snapshot, compiled by `core`, flat, ordered and already resolved. It builds a Tracktion edit from it and either streams preview audio or renders offline. It never receives a `Song` (ADR 0007 §1).
- **One render, one process.** The engine is spawned per render and exits with its answer; nothing is pooled, warmed or reused, because a resident plugin instance makes a render depend on the render before it (ADR 0008 §2). A live process for preview playback is M2's.
- Offline render must be **bit-exact across runs** on the same platform. Precisely: same `.escri` + same engine binary + same plugin binaries + same OS and CPU → identical PCM. In M1 that is Linux x86-64 on the image and compiler §17 pins; macOS, Windows and cross-CPU are unclaimed, not contradicted (ADR 0009 §1). Measured in M1 PR 11: all four goldens reproduce byte for byte across two x86-64 CPU models (Zen 5 with AVX-512, Zen 3 without) and two separate builds, sfizz included — evidence for the broader claim and not the claim itself. Read again in PR 13, the gap is wider than "the same dispatched path": sfizz's AVX switch is empty at this pin, so its render path is scalar-or-SSE on **every** x86-64 machine and no dispatcher on it can choose differently. The cross-CPU question is open on the mechanism, not merely on the sample (ADR 0009 §6).
- Bundled open-source instruments: **Surge XT, sfizz and Dexed**, each with the golden-render test §11 requires. Airwindows is a set of effects and moves to M4 with clap-wrapper (ADR 0010 §5).
- Every surface that computes audio carries a determinism note, not only the plugins: Rubber Band (see above) and sample-rate conversion for an asset that does not match the render rate join the three plugins below (ADR 0009 §4). Cross-platform differences are documented per surface. The plugin notes are what M1 PR 6 read in the pinned sources, not reputation:
  - **Dexed** `bce5dee` — pure FM. No runtime CPU dispatch, no resampling; the only variation is its own build, which links with `-flto` (`juce_recommended_lto_flags`). Expected bit-exact for a fixed compiler, and the cheapest of the three to bless. ~~No random number generator~~ — **corrected 2026-09-07 in PR 13: it has one, on exactly one path.** `msfa/lfo.cc` runs an 8-bit LCG (`randstate_ = (randstate_ * 179 + 17) & 0xff`) for LFO waveform 5, sample-and-hold, and `randstate_` is a `uint8_t` on a class with no constructor that neither `reset()` nor `keydown()` initialises — indeterminate memory, not a constant seed, so it cannot be seeded from a plan any more than Surge XT's can. Waveform 5 is therefore forbidden in a fixture, beside Surge's `rand_pm1` and sfizz's `*_random`. No golden is wrong today: the DX7 init voice is waveform 0, and every other waveform reads `phase_` alone.
  - **Surge XT** `f7b97c6` — **its global RNG is seeded from the wall clock.** `SurgeStorage.h` defines `STORAGE_USES_INDEPENDENT_RNG 1` and constructs `std::minstd_rand` from `std::chrono::system_clock::now().time_since_epoch().count()`, and the one API that would reseed it, `seed_rand`, is commented out. So ADR 0009 §3's "disable rather than seed" is not a preference for Surge, it is the only option: a fixture must avoid every path that reads it — oscillator random start phase, unison detune, the sample-and-hold LFO shape, and the effects that call `rand_pm1` — because nothing can make those repeat. ~~**Measured in PR 7:** the factory init patch does not reach it~~ — **corrected in PR 11: it does.** PR 7's three "fresh processes" rendered to one path, and until PR 11 a render over an existing file appended a second RIFF file rather than replacing it, so all three hashes were the first run's `data` chunk. Rendered properly, Surge XT at its factory patch gives a different hash every run; setting `A Osc 1 Retrigger` (`1217754326`) to 1.0 makes four fresh processes agree exactly, and that one parameter is what `tests/renders/surge_xt` exists to set. Compiles against a vendored SIMDe rather than dispatching at run time.
  - **Rubber Band** `1d95888`, built from `single/RubberBandSingle.cpp` — the built-in resampler (`USE_BQRESAMPLER`), hard-defined by that file rather than found on the build machine, and the built-in FFT (`USE_BUILTIN_FFT`) — which that file defines as the `#else` of a `#if defined(__APPLE__)` whose other arm takes `HAVE_VDSP` (precise, 2026-09-07 in PR 13). So it is this repository's choice on Linux and not a property of the library, which is the same edge §8's "macOS unclaimed" sits on. **No runtime CPU dispatch at all**: every SIMD path in the library is a compile-time `#ifdef` on `HAVE_IPP` or `HAVE_VDSP` and neither is defined, so unlike sfizz it is not a candidate for the cross-CPU experiment. Threading is compiled out by `NO_THREADING` *and* refused by `OptionThreadingNever`, and the option is in any case read only by the R2 engine, which the pinned option word does not select. Its default logger writes to `stderr`, so ADR 0008 §1 survives it; measured, it writes nothing at all. A stretched clip rendered identically in three fresh processes (`9bb47b31…`).
  - **Sample-rate conversion** — `juce::LagrangeInterpolator`, a fixed 5-point Lagrange polynomial with no options and no runtime dispatch, whose five-sample history `reset()` zeroes (`Interpolators::Lagrange` is `GenericInterpolator<LagrangeTraits, 5>`; "four" corrected 2026-09-07 in PR 13); its output is a pure function of the input, the ratio and the pinned JUCE. The engine converts an asset to the render's rate itself, once, before the stretch and the fades, so it is the only rate conversion in a render. **It has no anti-aliasing filter**, so downsampling folds everything above the new Nyquist; M1's fixtures are at the render rate and a band-limited resampler is the upgrade when a project downsamples something bright. A second instance is Tracktion's own: `WaveNode` reads every wave clip through one even at a 1:1 ratio, where the kernel is a delta and the steady state comes back bit-identical — but its 2-sample base latency is not compensated, so **an audio clip sounds two samples after its position**. Re-measured 2026-09-07 in PR 13 with a ramp asset whose every sample is identifiable: source sample *k* lands at the clip's start plus *k* plus two, and the clip's last two source samples fall off the end of its window. **Why upstream's fix does not reach it:** Tracktion does compensate this, in `LagrangeResamplerReader::readSamples`, which reads `getBaseLatency()` extra source frames on the first block and drops the matching destination frames — and that reader belongs to `WaveNodeRealTime`. Upstream's commit says so in its own subject line: `319afc0` (2024-07-23) removed the latency "when using `AudioClipBase::setUsesProxy (false)`". A clip the engine places `canUseProxy()`, so `EditNodeBuilder` builds the legacy `WaveNode` instead, which has no such path; the commit is an ancestor of our pin and changes nothing here. Upstream's, deterministic, and recorded so that the offset in a golden is not a mystery.
  - **sfizz** `4e70dc0`, built from sfizz-ui `6ef7b89` — **dispatches SIMD at run time, and at this pin it dispatches scalar or SSE and never AVX** (read again 2026-09-07 in PR 13; this row previously said "scalar, SSE or AVX per operation"). `SIMDHelpers.cpp` reads `cpuid::cpuinfo` and has two switches: the `has_sse()` one carries 21 real cases, and the `has_avx()` one, at `SIMDHelpers.cpp:99-106`, contains `default: break;` and nothing else. So no operation is ever pointed at an AVX implementation, the three translation units compiled `-mavx` build code the dispatcher never installs, and every x86-64 CPU has SSE — which makes the render path's ISA the same on every machine that can run the binary at all. The library's one live runtime AVX choice is `effects/Strings.cpp:38-39`, which picks `ResonantArrayAVX` for the `strings` effect, and a plain `<region>` fixture does not reach it. That is why ADR 0009 §6 no longer claims sfizz can answer trap 1 at this pin. Its randomness is seeded from a constant (`fast_rand`'s `mem` is `0` by in-class initialiser) and the generator is a `static` in a header — one instance per translation unit, not the process-wide singleton this row used to say, which changes the mechanism and not the conclusion. Its `*_random` opcodes stay out of fixtures. sfizz compiles its DSP core with `-ffast-math` upstream; the engine's build replaces that function with one that adds nothing (ADR 0009 §3, `engine/cmake/sfizz-no-fast-math/`). **How it finds a sample, measured in M1 PR 8b:** `FilePool::checkSample` joins the `sample=` name to the directory the SFZ was loaded from, and falls back — only on a miss — to a case-insensitive directory walk whose order is the filesystem's. An SFZ in `assets/` therefore reaches its siblings and nothing else, which is why what M1 renders is an SFZ naming its samples by their own asset hash (ADR 0007 §2, extended); the exact match hits and the walk is never reached. A sample it cannot resolve costs a dropped region and no message at all — the removal is a `DBG` a release build compiles out — so the engine reads the SFZ and refuses one whose samples are not beside it, rather than rendering the silence. Two more: sfizz switches to its *freewheeling* quality settings for an offline render, so a golden is deliberately not what a preview will play; and with no SFZ loaded it is not silent, since its default patch is `<region>sample=*sine`.
- **The pan law and the master's own gain are the engine's pins, not the document's** (recorded 2026-09-07 in PR 13, after a review found them in code and in no document). `Mix.pan` names a position and never a law, and the law decides the two gains that position becomes: the engine sets `te::setDefaultPanLaw (te::PanLawLinear)` before it builds an edit, and puts the master fader at 0 dB rather than Tracktion's own -3 dB default. Linear is the one law of the five that leaves a centred track at exactly its `gain_db`, and the cost is that a hard pan is +6 dB on the surviving side. Both are process-wide statics of Tracktion's, so a version bump that moved either would move every golden with a non-centred track and no pull request to blame — which is exactly what ADR 0011 §2 refused to leave implicit for a fade shape and ADR 0002 §8 for an automation curve.
- The engine has **no knowledge of the schema beyond the materialised snapshot**; it never reads project files. A path it is handed — an asset to read, a WAV to write — is input or output, not a project file (ADR 0007 §2, ADR 0008 §1). Two consequences of that, both found in M1 PR 11 and both about files whose names carry no meaning: an asset is named by its own SHA-256 and therefore has **no extension**, so its format is read from its header rather than from its name; and a render **removes its destination before writing**, because JUCE opens an existing file at its end and would otherwise append a second RIFF file whose audio no reader would reach (ADR 0009 §2, §4).

## 9. Desktop UI (`app` process)

- Tauri (Rust host) with a web frontend. The Rust host embeds the song model core and supervises `ai` and `engine`.
- Views (all projections of the model): arrangement/timeline, piano roll, mixer, instrument/effect editors, code views (generator source, DSP source), AI panel (conversation + pending patch preview + apply/reject), history (patch log).
- The AI panel always shows the **diff** before applying; users can apply, reject, or edit.

## 10. Persistence and interoperability

- Project directory, named `<name>.escri/` and presented by the OS as a single item where the platform supports package bundles: `song.json` (canonical model), `patches/` (append-only DAG of patch entries), `refs.json` (branch pointers and `HEAD`), `assets/` (content-addressed samples, compiled plugins, models), `lock.json` (pinned versions of every toolchain, plugin, and model; the platform baseline is in §17 and `lock.baseline.json`).
- Import/export: **DAWproject** (lossless for layer 2 + plugin references), MIDI (layer 2 notes), WAV/stems.
- `song.json` is a **derived cache** of `patches/`: the log is authoritative, and opening a project replays it and reports any mismatch rather than silently preferring either side (ADR 0004). The snapshot is kept because §2.6 requires the project be readable, diffable text.
- Assets referenced by hash; missing assets are reported by the validator, never silently substituted.

## 11. Determinism checklist [MUST]

Before merging any change, confirm:
- No unseeded randomness anywhere in `core`, compilers, or `engine`.
- No wall-clock dependence in compilation or rendering.
- All external tool versions recorded in `lock.json` and checked at load. From M1 that is the
  engine's submodule commits and one entry per plugin the song references, added on first
  reference and never removed; a referenced plugin this build cannot match refuses to open with
  `lock_mismatch`, and re-pinning a plugin is always explicit. The engine block re-pins on open
  instead: a newer engine still renders the song, and what a render was made with travels with
  the render (ADR 0010 §1–§3, ADR 0008 §5).
- A golden-render test exists for every bundled instrument (render fixture → hash compare).
  **The comparison is over the WAV's `data` chunk, never the whole file**, and the committed
  `.sha256` hashes the PCM payload: JUCE writes a `bext` chunk carrying `OriginationDate` and
  `OriginationTime`, so the audio is deterministic and the file is not (ADR 0009 §2). From M1
  that is `tests/renders.rs`, behind the `renders` cargo feature so it is absent where there is
  no engine to run rather than a test that skips itself; it renders each fixture twice for
  nondeterminism and against committed bytes for drift, and a mismatch names the first differing
  sample. It compares the commits the engine was built from against `lock.baseline.json` first,
  because a stale engine is a wrong answer and not a golden failure (ADR 0008 §5).
- **The determinism suite in `tests/` passes.** It drives `core` through the tool API over a
  real server process and checks the claim three ways: two runs against each other, each
  against a committed golden, and one transport against the other. Two runs agreeing catches
  nondeterminism; only the golden catches drift, where a dependency changes a serialisation
  detail and both runs are wrong together (M0.4).

## 12. Licensing

- Application: **AGPL-3.0** (decided). Tracktion Engine is GPL-3.0 and depends on JUCE 8, whose open-source licence is AGPL-3.0; the combination is distributed under AGPL terms. Commercial relicensing is explicitly deferred; the engine sits behind the gRPC boundary so it could be replaced if that ever becomes necessary.
- Cmajor and Faust generated code carry no copyleft; user-authored code, instrument sources, and rendered audio belong to the user.
- Bundled plugins and libraries: only AGPL-compatible open-source licences (Surge XT, sfizz, Dexed, Airwindows, Rubber Band, clap-wrapper MIT). Rubber Band, checked when it was vendored in M1 PR 8, is **GPL-2.0-or-later** with a commercial alternative we do not take: the "or later" reaches GPL-3.0, which AGPL-3.0 §13 permits combining with, so the combination is distributed under AGPL terms as the first bullet already says. Nothing here needs a commercial licence, and vendoring it as a submodule built into the engine binary is the case its licence is written for.

## 13. Repository layout

```
/schema          song.proto, history.proto, codegen scripts, generated types
/core            Rust: model, validator, tool API, patch log, DAWproject I/O
/app             Tauri host (Rust) + web frontend
/ai              Python orchestrator, tool bindings, models
/compilers       generative/ (Python DSL), dsp/ (Cmajor, Faust import, clap-wrapper)
/engine          C++ (Tracktion Engine), VST3 hosting, Cmajor JIT, render service
/proto           gRPC service definitions (SongTools, Render, Jobs)
/tests           golden renders, schema fixtures, determinism suite
/docs            this file, ADRs
```

This list governs **source** directories. Tooling and configuration at the repository root —
`.github/`, `.gitignore`, `rust-toolchain.toml`, `buf.yaml`, the workspace `Cargo.toml` — is
not in its scope and needs no ADR.

## 14. Agent working rules [MUST]

1. Read §2, §4, §5, §11 before any task. Do not modify the schema without an ADR in `/docs/adr/`.
2. Never introduce a second representation of song state (no per-view models, no ad-hoc JSON).
3. All model mutations go through the tool API, including in tests.
4. Any new source of randomness must accept and record a seed.
5. Any new external dependency (plugin, model, toolchain) must be pinned in `lock.json` and covered by a golden test.
6. Keep the engine schema-agnostic; keep the AI audio-agnostic.
7. When unsure whether something belongs in the model, ask: "would a different renderer need it?" If yes, it belongs in the model; if it is renderer-specific, it does not.

## 15. Decision log

| Decision | Chosen | Rationale | Date |
|---|---|---|---|
| DSP language | Cmajor primary, Faust import | Active project, official CLAP export, embeddable JIT, source-injection hook suited to AI authoring | 2026-09-02 |
| CLAP hosting | VST3 via clap-wrapper | Native CLAP hosting in Tracktion unverified; VST3 path is mature | 2026-09-02 |
| Licence | AGPL-3.0 | Forced by Tracktion + JUCE 8; monetisation deferred | 2026-09-02 |
| Generative language | Python DSL; Strudel deferred | Better LLM target, simpler sandboxing, keeps AGPL Strudel out of core | 2026-09-02 |
| Frontend | React + canvas/WebGL timeline | Ecosystem and tooling; rendering is custom either way | 2026-09-02 |
| Local LLM in v1 | No; provider-agnostic design | v1 on OpenRouter for easy model switching | 2026-09-02 |
| Preview audio | Engine process plays directly | No audio over IPC | 2026-09-02 |
| Cmajor placement | JIT for editing, exported CLAP→VST3 for persistence | Fast iteration + reproducibility | 2026-09-02 |
| Time-stretch | Rubber Band | AGPL-compatible | 2026-09-02 |
| Song history | Patch DAG + named refs; git as interop only | Branching falls out of the existing patch log; git's line merge is wrong for a structured model (ADR 0001) | 2026-09-02 |
| Entity collections | id-keyed maps, not `repeated` fields | RFC 6902 paths into arrays break under concurrent inserts (ADR 0001) | 2026-09-02 |
| Entity id generation | Injectable id source, seeded in tests | Reconciles §4.3 ULIDs with §11's no-unseeded-randomness rule (ADR 0001) | 2026-09-02 |
| Project file extension | `.escri` on the project directory; contents stay `.json` | Bundle gives file association and identity; standard extensions inside keep §2.6 diffability and editor/LLM legibility | 2026-09-02 |
| Tick integer width | `int32` (960 PPQ, ~559k bars) | proto3 JSON encodes 64-bit ints as strings, which would break §2.6 readable/diffable text (ADR 0002) | 2026-09-02 |
| Canonical JSON form | snake_case, defaults emitted, map keys sorted, 2-space | Always-present fields make RFC 6902 `replace`/`test` well-defined (ADR 0002) | 2026-09-02 |
| Patch entry on disk | Six keys including `schema_version`; `patches/<26-char ULID>.json`, flat | The proto's field is an on-disk concern, and ADR 0001's example was one field stale (ADR 0001 §1, corrected) | 2026-09-03 |
| `song.json` authority | Derived cache; the patch log is authoritative on disk | A commit touches three files and only each is atomic; without an answer a crash leaves them silently diverged (ADR 0004) | 2026-09-03 |
| Patch op representation | Canonical JSON text on disk; `bytes` on the wire | Modelling RFC 6902 values in protobuf made the patch log nondeterministic and produced documents core could not re-read (ADR 0002 §11) | 2026-09-02 |
| Tempo/time-signature events | id-keyed maps, like every other collection | An array index is not a stable patch path under concurrent branch inserts (ADR 0001 §3, corrected) | 2026-09-02 |
| Entity `version` on merge | Maintained by core, never in a tool op; resolved as max+1 | Otherwise every merge conflicts by construction on `/…/version` (ADR 0001 §4) | 2026-09-02 |
| Milestone scope | §16 authoritative; eight unplaced items placed; neural runtime in v1 at M4 | Unplaced scope is invisible scope, and §16 had fallen behind decisions binding elsewhere (ADR 0003) | 2026-09-02 |
| `FormRule` | Deferred to M4 | Least-specified entity in §4 and nothing consumes it before the generative compiler; additive to add (ADR 0002) | 2026-09-02 |
| Determinism, how it is checked | A suite in `tests/` driving a real server process, compared against a committed golden | Two runs in one job agree even when a dependency has changed a serialisation detail underneath them; only something committed earlier disagrees (M0.4) | 2026-09-03 |
| Entity `version` bumping | In the commit pipeline, between applying ops and re-deserialising | A bump taken after the patch is derived lives only in `song.json`, so every replay is one version behind the file beside it (ADR 0005 §1, §2) | 2026-09-03 |
| Undo mechanism | Append an inverse entry; never rewind a ref | A rewind decrements `version`, so §4.3's concurrency check can see one number with two different contents (ADR 0005 §4) | 2026-09-03 |
| Tool API wire shape | One RPC per tool, one shared `ToolResult`, `Violation` as the only wire error | Per-tool response messages are copies of one contract, free to drift; and an invalid call must stay inside §6's retry loop rather than becoming a transport failure (ADR 0006 §1, §2) | 2026-09-03 |
| MCP server | `rmcp` SDK; tool `inputSchema` generated from the protobuf descriptor | A hand-written schema is a second description of the model, and it drifts silently — the only symptom is that the model never learns a new field exists (ADR 0006 §6) | 2026-09-03 |
| Project identity | One project per process, named at launch; no `open_project` tool | An MCP stdio process is not a session, and two writers on one `.escri` break ADR 0004's write ordering (ADR 0006 §5) | 2026-09-03 |
| Engine input | A `RenderPlan` compiled by `core` — flat, ordered, resolved — never a `Song` | "Schema-agnostic" cannot mean the engine links nothing from the model; it means nothing needing the model's structure crosses, or its semantics get reimplemented in C++ (ADR 0007 §1) | 2026-09-04 |
| Plan vs. model | Leaf messages reused by value with §4.3 blanked; structural messages plan-local and `repeated` | A plan-local `PlanNote` is the mirrored shape ADR 0006 §4 forbids, and a plan is never patched, so ADR 0001 §3's map rule has nothing to protect (ADR 0007 §2, §5) | 2026-09-04 |
| Engine process shape | A fresh subprocess per render over stdio in M1; `Render` defined now, implemented over gRPC at M2 | A reused plugin instance ramps from its previous value, so render N depends on render N−1; and grpc++ would be vendored for a client that is its own parent (ADR 0008 §1, §2) | 2026-09-05 |
| Engine C++ codegen | Generated at build time by the engine's CMake; never committed | A committed `.pb.cc` pins a `protoc` that must equal the vendored runtime — two pins that must agree, with the stale one silent (ADR 0008 §4) | 2026-09-05 |
| What a render golden compares | The WAV's `data` chunk; the committed `.sha256` hashes the PCM payload | Three spike runs produced identical PCM and three different file hashes — JUCE writes `bext` with `OriginationDate`/`OriginationTime`, so a file hash would fail about once per second of build time and read as flakiness (ADR 0009 §2) | 2026-09-05 |
| Render determinism, scope | Linux x86-64 on a pinned image and compiler; cross-CPU unproven until CI measures it | A pin fixes the source; the bits also depend on what the compiler emitted and what the CPU chose at run time, and sfizz and JUCE dispatch SIMD at run time (ADR 0009 §1, §6) | 2026-09-05 |
| `lock.json` v2 | The engine's submodule commits plus one entry per referenced plugin, added on first reference and never removed | A block derived purely from the current song loses a pin on an ordinary delete, and undo (ADR 0005 §4) then re-pins from the running build — a silent re-pin caused by pressing undo (ADR 0010 §1, §2) | 2026-09-05 |
| A referenced plugin this build cannot match | Refuse to open, `lock_mismatch`; re-pinning is always explicit | The strict reading of §11, in the shape `schema_version_mismatch` already has. Rendering silence or substituting produces a wrong render that hashes differently with no error anywhere (ADR 0010 §3) | 2026-09-05 |
| A bundled plugin's identity in the manifest | `"<vendor>/<class name>"`, and a parameter is its numeric `ParamID` with its display name beside it | JUCE's identifier string hashes the plugin's path, which cannot go in a committed `lock.json`; and parameter names are not unique — Surge XT repeats 176 of 2855 (ADR 0010 §4, refined) | 2026-09-06 |
| A plan parameter's value domain | The plugin's normalised value, `0.0` to `1.0`, in `params` and on every `AutomationPoint` | VST3 exposes exactly one numeric domain to a host and it is normalised; a plugin's own units exist only as the display string beside it, which is the same fact that already forced the key to be the `ParamID` (ADR 0010 §4, extended) | 2026-09-06 |
| Cost of a fresh engine process | Measured, not assumed: 520 ms of process and framework start-up, plus 65 ms for one Dexed and under 400 ms for all three bundled plugins | ADR 0008 §2 traded reuse for determinism and left the price unquantified; the fixed half dominates, so a resident engine would buy back what it cannot spend safely (ADR 0008 §2, measured) | 2026-09-06 |
| Automation's interpolation axis | `LINEAR` is a straight line in **ticks**; the engine splits a segment at each tempo event inside it | Tracktion's own parameter curve is in seconds, and the two definitions diverge exactly where a tempo changes. Ticks are §4.2's one musical time, which is the axis a point is stored on (ADR 0002 §8) | 2026-09-06 |
| Surge XT's global RNG | Seeded from the wall clock; nothing can reseed it | Answers the question ADR 0009 §4 put to PR 6. `STORAGE_USES_INDEPENDENT_RNG` is 1 and `seed_rand` is commented out, so a fixture must avoid every path that reads it rather than seed it (ADR 0009 §4, answered) | 2026-09-06 |
| A newer engine than the project pins | Re-pin on open, do not refuse | A missing plugin makes a song unrenderable; a newer engine renders it fine and only risks bit-exactness. There is one engine and a project cannot choose it, so refusing would refuse every project at once, and §17's golden-render pass belongs on the PR that moves the pin (ADR 0010 §3) | 2026-09-05 |
| `plugin_unknown`, `param_unknown` | Validator rules, resolved against a build manifest the engine generates and `core` requires | §4.4 has wanted both since M0.2 and neither could resolve without knowing what this build hosts; an optional manifest would give both rules a silent skip arm (ADR 0010 §4) | 2026-09-05 |
| `AudioClip` | Gains `gain_db`, `fade_in_ticks`, `fade_out_ticks`, `time_stretch`; the fade formula is ours and stretch is a flag | An audio track has no device, so no `ParamRef` can reach a clip's level; and a stretch *ratio* would have to be computed by a caller that cannot read the asset (ADR 0011 §1, §3) | 2026-09-05 |
| Airwindows | Deferred to M4, with clap-wrapper | Its pinned repository may not build a Linux VST3, and §11's golden per bundled *instrument* is met by the three synths; M4 already has the CLAP→VST3 path (ADR 0003 §4, amended; ADR 0010 §5) | 2026-09-05 |
| A stretched loop whose clip is not a whole number of loops | Refused with `render_unsupported`, naming `loop_length_ticks` | Each loop iteration crosses as its own plan clip and the engine stretches to the clip it is handed, so a short last iteration would stretch to the wrong length; the plan cannot yet say otherwise, and a plan wrong by construction is not handed to the engine quietly (ADR 0007 §6, amended) | 2026-09-05 |
| Who computes `pcm_sha256` | The engine, with `juce::SHA256` from the pinned JUCE, reading back the `data` chunk it already checks for length | `RenderResult` is what a render reports about itself and M2's `Render` returns it whole; the module is in the JUCE already pinned, so the count of dependencies did not move; and a RIFF walker in `core`'s production path is audio knowledge on the wrong side of CLAUDE.md #6 (ADR 0009 §2, amended) | 2026-09-06 |
| Compile-time ISA for the engine | `-march=x86-64 -mtune=generic`, applied to every vendored source | The SSE2 baseline is all Tracktion, JUCE and choc require; a wider one buys speed M1 does not need and lets the compiler choose instructions that round differently, and it has no FMA to contract. Any other `-march` on a compile line fails the build (ADR 0009 §3) | 2026-09-06 |
| An audio clip's gain, fades and stretch | Applied by the engine into a buffer; Tracktion is handed a file of exactly the clip's length with no gain, fade or stretch of its own | ADR 0011 §2 makes the fade formula ours, so it cannot be a property set on a Tracktion clip — and once the samples are being touched, the stretch and the rate conversion come with them, which puts every surface that computes audio in one place with one determinism note each (ADR 0011 §2, §3; ADR 0009 §4) | 2026-09-07 |
| How Rubber Band is built | Upstream's `single/RubberBandSingle.cpp`, not its own build system | The full build picks an FFT from what is installed on the build machine, and a phase vocoder over two FFTs is two different signals — a hazard the pinned option word says nothing about. The single file hard-defines the built-in FFT and resampler and compiles threading out (ADR 0009 §4, written; ADR 0011 §3, completed) | 2026-09-07 |
| Sample-rate conversion for an asset | `juce::LagrangeInterpolator`, in the engine, once, before the stretch and the fades | Fixed 5-point Lagrange: no options, no runtime dispatch, a history `reset()` zeroes. No anti-alias filter, so downsampling folds — recorded as the note rather than hidden, since M1's fixtures are at the render rate (ADR 0009 §4, written) | 2026-09-07 |
| How an SFZ finds its samples | The SFZ names them by their own asset hash, and `assets/` being one flat directory is what resolves them | An SFZ is not self-contained and `add_asset` takes bytes, not names; sfizz resolves a `sample=` against the directory the SFZ came from, so an SFZ stored in `assets/` reaches its siblings — which are the other assets, under their hashes. Needs no staging, no rewriting and no schema change; the general case (a library carrying its own names) needs an asset that knows its name and is refused loudly until then (ADR 0007 §2, extended) | 2026-09-07 |
| A sampler's SFZ in the plan | `PlanInstrument.sfz_path`, the absolute path beside the `Instrument` that carries the hash | The audio clip's rule applied rather than a new one; embedding the bytes would carry the SFZ without the samples it names (ADR 0007 §2, extended) | 2026-09-07 |
| An SFZ this engine cannot resolve | Refused by the **engine**, not by compile, naming the `sample=` it could not find | Caller-fixable, so by ADR 0007 §6 it belongs in compile — but deciding it means reading the file and compile is pure. The one caller-fixable refusal in M1 that arrives as an exit code, rather than making compile impure for it (ADR 0007 §6, extended) | 2026-09-07 |
| What `render_export` answers with | `RenderResponse` — `valid`, `errors`, a `summary`, and the engine's `RenderResult` — not the shared `ToolResult` | A render produces no ops, so `patch` and `entry_id` would be permanently empty, and the hash and build commits it reports have nowhere to go in `ToolResult`, where §18.2 publishes the first and the render suite compares the second. `AddAsset` set the precedent; taken before any client had called the RPC (ADR 0006 §1, extended) | 2026-09-07 |
| Where `core` finds the engine binary | `--engine`, told, never searched; optional, and the call that needs it refuses as an operator error when it is absent | The rule ADR 0010 §4 set for the manifest, applied one artefact over: a search path that silently finds a stale engine renders against a build nobody chose. Optional rather than required because only `render_export` needs one, and a process that only edits must still start on a machine with no engine build (ADR 0008 §2) | 2026-09-07 |
| What a `SamplerRef` pins | The bundled plugin that plays the SFZ, in `lock.json`'s `plugins` block, named by a top-level `sampler` in the build manifest | The `sfz_hash` pins the *patch*; the build that turns it into samples decides every one of them, and a sampler-only project pinned nothing at all, leaving §11's [MUST] unsatisfied for a whole device kind (ADR 0010 §1, corrected; §4, extended) | 2026-09-07 |
| Pan law and master gain | `PanLawLinear`, and the master fader at 0 dB, set by the engine before it builds an edit | Nothing in the model names a law, and the law decides what `Mix.pan` becomes; both are process-wide statics in Tracktion, so a bump moving either moves every golden with a non-centred track and no PR to blame. Linear is the only one of the five that leaves a centred track at exactly its `gain_db`. Found in code and in no document by M1 PR 13's review (§8) | 2026-09-07 |
| The engine's C++ protobuf | protobuf v21.12 as a submodule; `protoc` built from it | The last line before the runtime depends on abseil, and what Ubuntu 24.04 packages; building `protoc` from the same checkout makes runtime and compiler one pin. **A new dependency pending sign-off** (ADR 0008 §4, pinned; CLAUDE.md #4) | 2026-09-06 |

Remaining open items **[OPEN]**: neural runtime packaging (ONNX Runtime linked into engine vs. separate process — must be resolved before M4, ADR 0003 §7); minimum supported OS versions; symbolic model choice for v1 melody/drum generation; whether §6's analysis features and symbolic generation are v1 scope at all (ADR 0003, Still unplaced).

## 16. Milestones

Scope placement is recorded in ADR 0003. Live status is `docs/plan.md`; the arc in user terms
is `docs/roadmap.md`. Both defer to this section.

1. **M0 – Schema & core**: proto, Rust core, validator, patch log with branching and merge, tool API over gRPC and MCP, JSON persistence, tests.
2. **M1 – Render**: engine process, Tracktion edit builder, the bundled instruments of §8 with a golden-render test each (§11), offline WAV render.
3. **M2 – UI**: Tauri app, timeline, piano roll, mixer and history as model projections, preview playback.
4. **M3 – AI loop**: Python sidecar, tool-calling loop with dry-run/diff/apply, AI panel. Preceded by the Libretto-grammar ADR required by §18.2.
5. **M4 – Compilers**: Cmajor JIT + CLAP export + clap-wrapper, Python DSL generator with seeds, neural runtime (§7.3), code views (§9), `lock.json` completed with compiled artefacts.
6. **M5 – Interop & polish**: DAWproject import/export, MIDI import/export (§10), REAPER RPP path, `song.proto` published as a standalone versioned artifact (§18.2), stems, installer bundling all three processes.

`lock.json` itself is created by the project store at M0 — it is part of the project directory
(§10) and is checked at load (§11). M4 adds the compiled artefacts to it.

## 17. Pinned toolchain baseline [MUST]

Resolved from upstream git on 2026-09-02. Agents pin **commit hashes**, not tags or branches; tags are listed for readability only. Upgrades require an ADR and a full golden-render pass. This table is mirrored in `/lock.baseline.json`. A project's `lock.json` recorded only `schema_version` until M1. From M1 it also records the engine's submodule commits and one entry per plugin the song references, added on first reference and compared at load, with the engine block re-pinned rather than refused (ADR 0010 §1–§3); M4 adds the compiled artefacts and model hashes (ADR 0003 §3). Only what a project uses is copied into it, never the whole table.

| Component | Version / tag | Commit | Date | Notes |
|---|---|---|---|---|
| Tracktion Engine | `develop` (post-v3.2.0) | `0e02f709c4088b2aec427ba6bbbfee3639139bb9` | 2026-09-02 | v3.2.0 (2025-05-15) is 16 months old; active development is on `develop`. Re-pin monthly until a v3.3 tag lands. |
| JUCE | 8.0.13 (Tracktion submodule) | `37c894f83d379179b2070d437ccd0f1cd9af9576` | 2026-05-21 | **Use the commit Tracktion pins, not JUCE latest.** JUCE 9.0.1 (2026-08-10) exists but Tracktion has not adopted it; do not mix. |
| Protobuf (C++ runtime and `protoc`, engine) | v21.12 | `f0dc78d7e6e331b8c6bb2d5283e06aa26883ca7c` | 2022-12-12 | Submodule; the engine's CMake builds `protoc` from it and generates the C++ for `schema/song.proto` and `proto/render.proto` at build time (ADR 0008 §4). The last line before abseil, and what Ubuntu 24.04 packages. **Added 2026-09-06 (M1 PR 5), pending sign-off.** |
| Cmajor | 1.0.3177 | `024a208515f15e43271d9b2ea85ee22a2233384b` | 2026-07-28 | Provides `cmaj` CLI and `libCmajPerformer`. |
| clap-wrapper | v0.16.0 | `1cca996e96f29ab2be7ae9f8cfe532bbc92e1dd6` | 2026-08-08 | CLAP → VST3 projection. |
| CLAP SDK | 1.2.10 | tag `1.2.10` | — | Header-only; pinned via tag hash at vendoring time. |
| ONNX Runtime | v1.29.0 | `2e2543fbe9fae542f921d47a72d21d5a4ef0b710` | 2026-08-11 | Neural runtime. |
| Rubber Band | v4.0.0 | `1d95888bec3ae0a17c0c4af791810d5a63f6bc35` | 2024-10-25 | Time-stretch (GPL-2.0-or-later, compatible with this repository's AGPL-3.0 through the "or later"). Vendored 2026-09-07 in M1 PR 8 and built from `single/RubberBandSingle.cpp`, so the FFT and resampler are the library's own rather than the build machine's. Its option word is pinned in full by ADR 0011 §3. |
| Faust | 2.85.9 | tag `2.85.9` | — | Import path only; pin hash when vendored. |
| Surge XT | release_xt_1.3.4 | `f7b97c682ade0b87da85ca5968b63d5c7c98e68d` | 2024-08-11 | Bundled instrument, `surge-synthesizer/surge`. Resolved 2026-09-05. |
| sfizz | 1.2.3 | `4e70dc0bef53b41f2853ed46e26f5911114c92d0` | 2024-01-14 | SFZ engine, **library only**: its CMake builds a library and a JACK client, no VST3 (spike, 2026-09-04). Not a submodule of ours: sfizz-ui pins its `library` submodule at exactly this commit, verified 2026-09-06 in PR 6, so this row records what sfizz-ui vendors. |
| sfizz-ui | 1.2.3 | `6ef7b89b6e5aa914593c7f3ca19b859915c30337` | 2024-01-14 | **The bundled sampler plugin**, and the submodule the engine builds. `sfztools/sfizz-ui` is where the VST3 is built; the `sfizz` pin alone yields none (ADR 0010). Its bundle declares two classes, `sfizz` and `sfizz-multi`. |
| Dexed | v1.0.1 | `bce5deee7c41bf5515b806d0b7de8b5c0bb49467` | 2025-11-29 | Bundled FM synth, `asb2m10/dexed`. Resolved 2026-09-05. |
| Airwindows | `main` | `ab0d1df871b8` | 2026-09-02 | No release tags upstream; pin by commit. **Deferred to M4** with clap-wrapper (ADR 0010 §5). |
| Tauri | tauri-v2.11.5 / CLI 2.11.4 | tag | — | Desktop shell. |
| Python (`ai`) | 3.12.x | — | — | Pin exact patch in `ai/.python-version`; lock deps with `uv`. |
| LLM provider | OpenRouter | — | — | Model ids pinned per project in `lock.json` under `ai.model`. |
| CI image (golden renders) | `ubuntu-24.04` | — | — | The image every M1 golden render is valid for. `ubuntu-latest` moves, and every golden would drift with no PR to blame (ADR 0009 §5). |
| C++ compiler (golden renders) | g++ 13.3 | — | — | What the spike ran and what the goldens are blessed under. `engine/`'s CMake pins the compile-time baseline at `-march=x86-64 -mtune=generic` (SSE2: all Tracktion, JUCE and choc require, and no FMA to contract; chosen in PR 5), sets `-ffp-contract=off` on every vendored source, and fails the build on `-ffast-math` or a second `-march` (ADR 0009 §3). |

Rules:
- `lock.baseline.json` is the only place these values live in code; CI fails if a submodule or vendored dependency drifts from it.
- **Registry packages** (crates.io, npm, PyPI) have no commit to pin. They are recorded by exact version, and their integrity hashes live in `Cargo.lock`, `package-lock.json` and `uv.lock`, which are committed. This covers build-time tooling and **Rust dependencies that ship in the product**: a crates.io release is immutable and its hash is verified on every build, which is at least as strong as a git commit. Anything vendored or linked from source — C++ libraries, plugins, engines — is still pinned by commit (ADR 0006).
- Golden-render fixtures are regenerated **only** in the same PR that changes a pin, and the diff must be explained in the ADR.
- JUCE is never upgraded independently of Tracktion Engine.

## 18. Competitive positioning and strategy [MUST read before roadmap changes]

Source: landscape analysis of 2026-09-02 (see `docs/landscape-2026-09.md`). Summary of findings agents must not contradict in product decisions:

- No shipping product or published system combines all of: typed persistent song model as source of truth, validated dry-run→diff→apply tool API, bit-exact reproducible rendering, instruments-as-code, a real C++ render engine with plugin hosting, open source, model-agnostic LLM. Each competitor lacks at least two of these.
- Closest overlaps, by threat: Waveform MCP (same Tracktion lineage, 107 tools, no typed model/determinism), Producer Pal (polished GPL MCP bridge into Ableton), chuk-mcp-music (typed Score IR, validation, deterministic MIDI, no render), Mozart AI / Suno Studio (funded, audio-first, closed), Libretto (paper: LLM-native bar-structured grammar + structural evaluation, no released code found), AbletonMCP / REAPER MCP family, ACE-Step 1.5 (editable open audio model, explicitly seed-sensitive).
- Determinism and instruments-as-code are the two pillars almost universally absent and structurally hard for audio-model-based competitors to copy.

### 18.1 Defensible differentiators (protect these in every design decision)
1. Bit-exact determinism with lock file (§11, §17).
2. Instruments and effects as code inside a versioned, reproducible graph (§7.2).
3. Typed song model as source of truth with validated dry-run→diff→apply (§4, §5).
4. AGPL, open schema, model-agnostic LLM layer (§6, §12).

### 18.2 Staged recommendations

*Stage 1 — establish the wedge (M0–M3)*
- **Expose the tool API (§5) as an MCP server from M0.** Same tool definitions, served over MCP, so Claude/Cursor/any MCP client can drive a project immediately. This is a hard requirement, not a nice-to-have: MCP is the de facto agent protocol in audio software and the alternative is competing with 20+ hobby bridges on their terms.
- **Lead every demo with reproducibility**: the canonical demo is "edit bar 17, re-render, and everything the edit did not reach is byte for byte what it was". Publish render hashes in golden tests and in release notes.

  **Amended 2026-09-07, when M1 PR 12 measured it.** This bullet used to say "WAV diff shows bytes changed only in bar 17", and that half is not true — not because controllability is weak, but because an instrument is not a pure function of the current block. A note's release tail outlives its note-off, so an edit inside bar 17 goes on changing samples after bar 17 ends: measured on this build, a half-bar note's tail runs about 5,800 frames (0.12 s) past its note-off, and the same edit to a whole-bar note puts the last differing frame 24,119 frames *inside bar 18*. "Only bar 17" is an artefact of how long the edited note is, not a property of the platform, and a demo that claimed it would be caught by the first person who tried it with a longer note.

  What is true is the half the claim is actually sold on, and `tests/renders.rs` asserts it exactly with no tolerance: **nothing before the edit moves by one sample.** What comes after is measured and printed rather than asserted, bounded only by the one failure that would matter — a difference that never recovers, which would mean the edit changed the instrument's state for the rest of the song rather than an instrument decaying (§1, ADR 0009 §2).
- **Adopt a Libretto-style grammar for the composition layer's LLM-facing view**: integer onset slots on a bar grid (already implied by 960 PPQ ticks), explicit voices, bar-level blocks; and adopt its structural evaluation axes (rhythm, harmony, melody, texture, form, within-song variation) as the AI orchestrator's self-check metrics. Record this in an ADR before M3.

*Stage 2 — interoperability and credibility (M4–M5)*
- Ship DAWproject import/export (Bitwig, Studio One, Cubase, Nuendo, Cubasis, VST Live, Fender Studio) and a REAPER RPP path via the existing converter ecosystem. Position as "escape hatch to any pro DAW"; do not claim Logic/Ableton/Pro Tools/FL support.
- Publish the song schema (`schema/song.proto`) as a standalone, documented, versioned artifact and promote it as an AI-readable song-project standard. No such standard exists; owning it is a defensibility play.

*Stage 3 — positioning*
- Primary audiences: developers, educators, and reproducibility-minded producers/composers. Do not chase the prompt-to-song mass market owned by Suno/Mozart/Google Flow.
- Messaging hierarchy: reproducible → typed/inspectable → code-defined instruments → open. "Symbolic vs audio" is secondary and may erode as editable audio models improve.

### 18.3 Watch list (re-check quarterly)
- Mozart AI or Suno shipping a typed, exportable, schema-validated project model.
- A DAW vendor shipping an official *control* agent with validation (Ableton's Claude connector is knowledge-only as of 2026-04).
- ACE-Step successors adding fine-grained or note-locked, deterministic editing.
- Waveform MCP or Producer Pal adding a persistent typed model with diffs.

### 18.4 Triggers that change strategy
| If this happens | Then |
|---|---|
| A funded incumbent ships a typed, exportable editable model | Double down on determinism + instruments-as-code + open schema; accelerate MCP exposure |
| Deterministic, note-locked audio editing is demonstrated | Drop "audio can't edit" framing; message on typed model + reproducibility + open |
| A DAW vendor ships an official validated agent | Emphasise open, model-agnostic, code-defined stack; pursue DAWproject interop with that vendor |

### 18.5 Reading list for agents (before M0)
1. Libretto, arXiv 2606.22708 — grammar and evaluation axes.
2. Waveform MCP (jarmstrong158) — tool taxonomy on Tracktion; study gaps.
3. Producer Pal (adamjmurray, GPL-3.0) — MCP + REST design reference.
4. chuk-mcp-music — typed Score IR, validation, deterministic MIDI compiler.
5. ACE-Step 1.5, arXiv 2602.00744 — editable audio model and its non-determinism.
6. DAWproject spec (bitwig/dawproject) — interop target.
7. NotaGen (IJCAI 2025) — hostable open symbolic model with ABC/MusicXML/MIDI export.
