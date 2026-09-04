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

## Current milestone: M1 — Render engine

M0 is complete: schema, `core`, the tool API over gRPC and MCP, and the determinism suite.
See `docs/plan.md` for what each step delivered and `docs/specs.md` §16 for what M1 is.

Live status, deferred items and known gaps: `docs/plan.md`. Read `docs/specs.md` §16 and
ADR 0003 before starting a step; they place what M1 owns and what it does not.

## Completed: M0 — Schema & core

1. `schema/song.proto` and `history.proto`, with codegen for Rust, TypeScript and Python.
2. `core/`: model types, validator (§4.4), the patch DAG, canonical JSON persistence, the
   `.escri` project store.
3. `proto/SongTools` with `dry_run`, implemented in `core` and served over both gRPC and MCP.
4. `tests/`: the determinism suite — same input, identical canonical JSON and patch log,
   checked against a committed golden and across both transports.

C++ codegen was deferred from step 1 to M1, where the engine gives it a consumer.

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
