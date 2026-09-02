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

## Current milestone: M0 — Schema & core
Deliver, in this order, each as a separate PR with tests:
1. `schema/song.proto` implementing §4, plus codegen for Rust, TypeScript, Python (C++ later).
2. `core/`: Rust crate with model types (generated), validator (§4.4), JSON Patch log, canonical JSON persistence.
3. `proto/SongTools` gRPC service (§5) with `dry_run`, implemented in `core`, and the same tools exposed as an MCP server (§18.2).
4. `tests/`: schema fixtures and a determinism suite (same input → identical canonical JSON and patch log).

Out of scope for M0: engine, UI, AI orchestrator, compilers. Do not scaffold them.

## Toolchain
- Rust stable, `cargo`; Protobuf via `prost`/`tonic`.
- Python 3.12 with `uv`; generated Pydantic models only.
- TypeScript types generated from the proto; no hand-written model types anywhere.

## Working style
- Small PRs, one milestone step each. Write the ADR first when a decision is needed.
- Run the full test suite before declaring a step done. Never skip failing tests.
- Ask before adding a dependency not in `lock.baseline.json`.
