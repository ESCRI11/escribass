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
- Every tool supports `dry_run=true` returning `{ valid, errors[], patch, summary }` without applying.
- Every tool call is validated against §4.4 before apply. Invalid calls return structured errors the LLM can act on.
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
- Time-stretch: Rubber Band (GPL). Elastique is not used.
- Receives a materialised layer 2 + layer 3 snapshot, builds a Tracktion edit, and either streams preview audio or renders offline.
- Offline render must be **bit-exact across runs** on the same platform; cross-platform differences are documented per plugin.
- Bundled open-source instruments and effects (candidates: Surge XT, sfizz, Dexed, Airwindows) so a fresh install produces usable sound.
- The engine has **no knowledge of the schema beyond the materialised snapshot**; it never reads project files.

## 9. Desktop UI (`app` process)

- Tauri (Rust host) with a web frontend. The Rust host embeds the song model core and supervises `ai` and `engine`.
- Views (all projections of the model): arrangement/timeline, piano roll, mixer, instrument/effect editors, code views (generator source, DSP source), AI panel (conversation + pending patch preview + apply/reject), history (patch log).
- The AI panel always shows the **diff** before applying; users can apply, reject, or edit.

## 10. Persistence and interoperability

- Project directory, named `<name>.escri/` and presented by the OS as a single item where the platform supports package bundles: `song.json` (canonical model), `patches/` (append-only DAG of patch entries), `refs.json` (branch pointers and `HEAD`), `assets/` (content-addressed samples, compiled plugins, models), `lock.json` (pinned versions of every toolchain, plugin, and model; the platform baseline is in §17 and `lock.baseline.json`).
- Import/export: **DAWproject** (lossless for layer 2 + plugin references), MIDI (layer 2 notes), WAV/stems.
- Assets referenced by hash; missing assets are reported by the validator, never silently substituted.

## 11. Determinism checklist [MUST]

Before merging any change, confirm:
- No unseeded randomness anywhere in `core`, compilers, or `engine`.
- No wall-clock dependence in compilation or rendering.
- All external tool versions recorded in `lock.json` and checked at load.
- A golden-render test exists for every bundled instrument (render fixture → hash compare).

## 12. Licensing

- Application: **AGPL-3.0** (decided). Tracktion Engine is GPL-3.0 and depends on JUCE 8, whose open-source licence is AGPL-3.0; the combination is distributed under AGPL terms. Commercial relicensing is explicitly deferred; the engine sits behind the gRPC boundary so it could be replaced if that ever becomes necessary.
- Cmajor and Faust generated code carry no copyleft; user-authored code, instrument sources, and rendered audio belong to the user.
- Bundled plugins and libraries: only AGPL-compatible open-source licences (Surge XT, sfizz, Dexed, Airwindows, Rubber Band, clap-wrapper MIT).

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
| Patch op representation | Canonical JSON text on disk; `bytes` on the wire | Modelling RFC 6902 values in protobuf made the patch log nondeterministic and produced documents core could not re-read (ADR 0002 §11) | 2026-09-02 |
| Tempo/time-signature events | id-keyed maps, like every other collection | An array index is not a stable patch path under concurrent branch inserts (ADR 0001 §3, corrected) | 2026-09-02 |
| Entity `version` on merge | Maintained by core, never in a tool op; resolved as max+1 | Otherwise every merge conflicts by construction on `/…/version` (ADR 0001 §4) | 2026-09-02 |
| `FormRule` | Deferred to M4 | Least-specified entity in §4 and nothing consumes it before the generative compiler; additive to add (ADR 0002) | 2026-09-02 |

Remaining open items **[OPEN]**: neural runtime packaging (ONNX Runtime linked into engine vs. separate process); minimum supported OS versions; symbolic model choice for v1 melody/drum generation.

## 16. Milestones (proposed)

1. **M0 – Schema & core**: proto, Rust core, validator, patch log, tool API, JSON persistence, tests.
2. **M1 – Render**: engine process, Tracktion edit builder, one bundled synth, offline WAV render, golden test.
3. **M2 – UI**: Tauri app, timeline + piano roll as model projections, preview playback.
4. **M3 – AI loop**: Python sidecar, tool-calling loop with dry-run/diff/apply, AI panel.
5. **M4 – Compilers**: Cmajor JIT + CLAP export + clap-wrapper, Python DSL generator with seeds, lock file.
6. **M5 – Interop & polish**: DAWproject import/export, stems, installer bundling all three processes.

## 17. Pinned toolchain baseline [MUST]

Resolved from upstream git on 2026-09-02. Agents pin **commit hashes**, not tags or branches; tags are listed for readability only. Upgrades require an ADR and a full golden-render pass. This table is mirrored in `/lock.baseline.json` and copied into every new project's `lock.json`.

| Component | Version / tag | Commit | Date | Notes |
|---|---|---|---|---|
| Tracktion Engine | `develop` (post-v3.2.0) | `0e02f709c4088b2aec427ba6bbbfee3639139bb9` | 2026-09-02 | v3.2.0 (2025-05-15) is 16 months old; active development is on `develop`. Re-pin monthly until a v3.3 tag lands. |
| JUCE | 8.0.13 (Tracktion submodule) | `37c894f83d379179b2070d437ccd0f1cd9af9576` | 2026-05-21 | **Use the commit Tracktion pins, not JUCE latest.** JUCE 9.0.1 (2026-08-10) exists but Tracktion has not adopted it; do not mix. |
| Cmajor | 1.0.3177 | `024a208515f15e43271d9b2ea85ee22a2233384b` | 2026-07-28 | Provides `cmaj` CLI and `libCmajPerformer`. |
| clap-wrapper | v0.16.0 | `1cca996e96f29ab2be7ae9f8cfe532bbc92e1dd6` | 2026-08-08 | CLAP → VST3 projection. |
| CLAP SDK | 1.2.10 | tag `1.2.10` | — | Header-only; pinned via tag hash at vendoring time. |
| ONNX Runtime | v1.29.0 | `2e2543fbe9fae542f921d47a72d21d5a4ef0b710` | 2026-08-11 | Neural runtime. |
| Rubber Band | v4.0.0 | `1d95888bec3ae0a17c0c4af791810d5a63f6bc35` | 2024-10-25 | Time-stretch (GPL). |
| Faust | 2.85.9 | tag `2.85.9` | — | Import path only; pin hash when vendored. |
| Surge XT | release_xt_1.3.4 | tag `release_xt_1.3.4` | — | Bundled instrument. |
| sfizz | 1.2.3 | tag `1.2.3` | — | Bundled sampler (SFZ). |
| Dexed | v1.0.1 | tag `v1.0.1` | — | Bundled FM synth. |
| Airwindows | `main` | `ab0d1df871b8` | 2026-09-02 | No release tags upstream; pin by commit. |
| Tauri | tauri-v2.11.5 / CLI 2.11.4 | tag | — | Desktop shell. |
| Python (`ai`) | 3.12.x | — | — | Pin exact patch in `ai/.python-version`; lock deps with `uv`. |
| LLM provider | OpenRouter | — | — | Model ids pinned per project in `lock.json` under `ai.model`. |

Rules:
- `lock.baseline.json` is the only place these values live in code; CI fails if a submodule or vendored dependency drifts from it.
- **Registry packages** (crates.io, npm, PyPI) have no commit to pin. They are recorded by exact version, and their integrity hashes live in `Cargo.lock`, `package-lock.json` and `uv.lock`, which are committed. This applies only to build-time tooling that does not ship in the product; anything vendored or linked is still pinned by commit.
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
- **Lead every demo with reproducibility**: the canonical demo is "edit bar 17, re-render, WAV diff shows bytes changed only in bar 17". Publish render hashes in golden tests and in release notes.
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
