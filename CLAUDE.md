# CLAUDE.md

Open-source (AGPL-3.0) desktop platform for AI-driven, code-defined music creation.
Read `docs/specs.md` before any task. Sections marked [MUST] are binding. Sections marked [OPEN] are not yours to decide: stop and ask.

## Non-negotiables
1. The song model (`schema/song.proto`) is the only source of truth. Never add a second representation of song state.
2. All mutations go through the tool API as JSON Patch, including in tests. Never write `song.json` directly.
3. Determinism: no unseeded randomness, no wall-clock dependence, in `core`, `compilers`, or `engine`.
4. Every external dependency is pinned by commit in `lock.baseline.json`. JUCE is never upgraded independently of Tracktion Engine.
5. Schema changes require an ADR in `docs/adr/` before code.
6. The engine is schema-agnostic; the AI process is audio-agnostic.

## Repo layout
See `docs/specs.md` §13. Do not create top-level directories not listed there without an ADR.

## Current milestone: M2 — UI

M0 and M1 are complete: the schema, `core` and the tool API below the engine; the render
engine above it. §16 makes M2 the Tauri app — timeline, piano roll, mixer and history as
projections of the model — plus preview playback, which is the live process `Render` gets its
gRPC implementation for (ADR 0008 §1).

Live status, deferred items and known gaps: `docs/plan.md`. Read `docs/specs.md` §16 and
ADR 0003 before starting a step; they place what M2 owns and what it does not.

## Completed: M1 — Render engine

1. ADRs 0007–0011: what crosses to the engine, how it runs, what "deterministic" means, what
   is pinned, and what an audio clip is.
2. `AudioClip` gains `gain_db`, fades and `time_stretch` — M1's only schema change (ADR 0011).
3. `proto/render.proto` and `core/src/render.rs`: `compile(song, assets) -> RenderPlan`, pure;
   `render_export` and `add_asset` on `SongTools`, over both transports.
4. `engine/`: a fresh C++ subprocess per render over stdio, building a Tracktion edit and
   hosting Surge XT, sfizz and Dexed as VST3, with audio clips at their gain, fades and
   Rubber Band stretch. Its C++ protobuf is generated at build time by its own CMake
   (ADR 0008 §4) — the codegen M0 step 1 deferred, landed where the engine consumes it.
5. `lock.json` v2: the engine's submodule commits and one entry per referenced plugin, added
   on first reference and compared at load — `lock_mismatch`, `plugin_unknown`,
   `param_unknown` and `param_out_of_range`, resolved against the engine's build manifest.
6. `tests/renders.rs`, behind the `renders` feature: a golden render per bundled instrument
   and one for an audio clip, each rendered twice and against committed PCM (§11), plus the
   bar-17 demo §18.2 leads with.

## Completed: M0 — Schema & core

1. `schema/song.proto` and `history.proto`, with codegen for Rust, TypeScript and Python.
2. `core/`: model types, validator (§4.4), the patch DAG, canonical JSON persistence, the
   `.escri` project store.
3. `proto/SongTools` with `dry_run`, implemented in `core` and served over both gRPC and MCP.
4. `tests/`: the determinism suite — same input, identical canonical JSON and patch log,
   checked against a committed golden and across both transports.

## Toolchain
- Rust stable, `cargo`; Protobuf via `prost`/`tonic`.
- Python 3.12 with `uv`; generated Pydantic models only.
- TypeScript types generated from the proto; no hand-written model types anywhere.

## Working style
- Small PRs, one milestone step each. Write the ADR first when a decision is needed.
- **Never commit to `main`.** Branch, push, open a PR, merge when CI is green. `buf breaking`
  runs on pull requests only, so work that skips the PR skips the wire-compatibility check.
- Run the full test suite before declaring a step done. Never skip failing tests.
- Ask before adding a dependency not in `lock.baseline.json`.
