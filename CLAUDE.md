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
7. **Spending money needs the user's confirmation, every time.** A call to an external paid service — a hosted model API, any metered endpoint, anything billed to the user's account — is asked for and granted before it is made. A previous authorisation does not carry to the next task. When it is granted, the spend is capped in code *before* each call against a conservative worst case, ledgered, and reconciled against the provider's own counter.

## Repo layout
See `docs/specs.md` §13. Do not create top-level directories not listed there without an ADR.

## Current milestone: M4 — Generators

M0 to M3 are complete: the schema, `core` and the tool API; the render engine and four golden
WAVs; the Tauri app with its views as projections of the model and preview playback; and the AI
loop — a Python sidecar, a proposal a person applies, and the panel. **M4 was split on
2026-09-25** (ADR 0023 §1, the user's decision): §16's M4 is now the **generators** — the seeded
Python DSL (§7.1) and its sandbox, `define_generator` and `compile_generator`,
`Generator.compiled_hash`, the generator code view, and `lock.json`'s `toolchains` block — and it
adds **no external dependency**. Everything native is **M5**: Cmajor export through clap-wrapper
with **no JIT in v1** (§7.2, reversed on the spike's measurement), the ONNX-to-Cmajor converter
as the whole of v1's neural path with no runtime process and no model shipped (§7.3), Airwindows
via airwin2rack, the DSP code view, and `lock.json`'s `artefacts` and `models` blocks. Interop
and the installer are **M6**.

**M4 is planned and decided.** `docs/plan.md`'s M4 section holds the plan (PR #82), the spike's
findings (PR #83, never merged, carried onto `main`), the ten user decisions of 2026-09-25 and the
seventeen answered with them, and ADRs 0023–0027 are the decisions: the split and what M5 and M6
now own; the sandbox as a subprocess `core` spawns per compile and the DSL as a Python subset over
`int` and `Fraction` behind an `ast` allowlist, claiming purity and a resource limit and **not**
security (ADR 0024); `Generator.compiled_hash` alone, with `SourceRef.export_hash` waiting for
M5's producer (ADR 0025); `proto/generate.proto`, the two tools, fourteen tools offered and a
compile diagnostic fed back as a refusal (ADR 0026); and a `toolchains` block checked at compile
and never at open (ADR 0027). The PR table is the plan's, PR 2 next: the schema change alone.

**Three things to carry into every M4 step.** Nothing here is measured on a second machine: the
repository was still private when M4 PR 1 opened, no runner has executed a step since 2026-09-09,
and the user's decision to make it public (U2) had not taken effect — the first pull request a
runner sees says so. `CEILING_USD` in `ai/src/escribass_ai/provider.py` equals the ledger's
spent total, so every live run fails closed, and no paid measurement of a model writing the DSL
is made before the DSL exists (U10). And M5's pins are **approved, not vendored**:
`lock.baseline.json` moves when M5 uses them, `libjack-jackd2-dev` is a build dependency of the
pinned image the user is installing, and a person reads Cmajor's licence before M5's first export
(U9).

Live status, deferred items and known gaps: `docs/plan.md`. Read `docs/specs.md` §16, §7 and
ADRs 0023–0027 before starting a step. Read `docs/plan.md`'s "M3, closed" first: it records what
M3 leaves unverified — **a milestone and a half of merges no CI runner has seen**, one live model
turn and no second, a preview that has still never reached a speaker, and one paid call made in
M3 PR 5 before anybody asked, which is why non-negotiable 7 exists.

## Completed: M3 — AI loop

1. ADRs 0018–0022: what the model reads and that it writes ticks, a proposal as the model's calls
   applied to a fork, `ai` serving one stream with the host executing, provenance that names the
   model and the prompt, and twelve tools with three kinds of failure. **No `song.proto` change**:
   `schema/` is byte-identical to M1's close, as it was at M2's. `proto/assistant.proto` is new —
   one service, one bidirectional stream — and `song_tools.proto` is untouched.
2. `ai/`: a Python 3.12 package under `uv` with `grpclib`, `openai` and `httpx2` pinned, serving
   `Assistant.Prompt` over a Unix socket it names on its own stdout. It computes ADR 0018's
   bar-block view and six counted axes — seven pure functions, seven goldens, and a key-reversed
   twin — and never sees audio (CLAUDE.md #6). A scripted provider replays a recorded transcript;
   `provider.Live` is the one path that can spend.
3. `core`: a `Proposal` is a session on a copy of the project that records nothing, so the model's
   calls go through `core::call` — the same dispatch the window and the MCP server use — and the
   only gate is `OFFERED`, twelve of twenty-five, with `set_param` and `dry_run` withheld. One
   patch, once, at approval, under the tool name `proposal`; a document that moved at any path the
   patch writes to is refused `document_moved`. `prepare` decides every entity's provenance on
   every path, which closed a forgery. `.escri/lock` is the kernel's, under `File::try_lock`.
4. `app/`: the AI panel. The conversation is the host's, sent whole per prompt and persisted
   beside the project as JSON Lines per song id; the proposal is drawn dashed as it grows, with
   its RFC 6902 diff; Apply, Reject and Edit, one proposal pending at a time. The panel names the
   model the project records and never calls it verified, pinned or deterministic.
5. **Money.** Non-negotiable 7 was written mid-milestone, after a transcript was recorded against
   a real model without asking. A paid call is now priced at a conservative worst case and
   checked against `CEILING_USD` **before** it goes out, every call writes one line to
   `~/.escribass/spend.jsonl` read back at startup, and `escribass-ai --account-usage` /
   `--generation-cost` reconcile against OpenRouter's own counter. The grant was one session, it
   was spent, and the ceiling was lowered to what it cost.
6. `tests/`: the loop is goldened end to end against a scripted model — one prompt driven twice
   through the real `ai` process, compared with itself and against committed bytes — which is
   §11's seventh bullet, added at M3's close because the test had described itself that way and
   §11 had never named it. It is behind the `ai` cargo feature, so `cargo test` alone does not run
   it. `tests/determinism/bar17/` is §18.2's demo driven by the assistant. The live run is
   `#[ignore]`d and says on every run that it spends money.

## Completed: M2 — UI

1. ADRs 0012–0017: what the webview calls and holds, gRPC in the engine and what a preview is,
   the seventh view and the one platform, a `ParamRef` reaching a fader, the pinned frontend
   set, and a gesture as dry runs committed once. **No `song.proto` change**: `schema/` is
   byte-identical to M1's close. `render.proto` gains `Preview`, `PlanTrack.mix_lanes` and
   `PreviewEvent.applied`; `song_tools.proto` gains `undo`, `redo`, `render_preview` and
   `merge_branch`'s per-path resolutions.
2. `app/`: a Tauri host embedding `core`, with one `tool` command dispatching into `core::call`
   — the function the MCP server dispatches through — and a `manifest` command, and `.escri/lock`
   taken with `create_new`. The frontend holds one decoded `Song`, re-read after every applied
   call, and draws the arrangement, the piano roll, the mixer, the patch log with branch
   switching and merge resolution, and a generic parameter editor over the build manifest. Every
   position of a drag is a `dry_run` and one entry is committed. `app/tests/projection.test.ts`
   is §11's projection golden.
3. `core`: `call(session, name, args)`; `undo` and `redo` read off the log, with no cursor in
   the session; a `ParamRef` naming a track for `gain_db` or `pan`, and `compile` emitting
   `mix_lanes`; `render_preview` over the live `Preview` stream in `core/src/engine.rs`; and
   `InArrivalOrder`, which serves the MCP transport one request at a time.
4. `engine/`: grpc++ v1.54.3 vendored. One binary in two modes over a Unix socket it names on
   its own stdout — `--render` serves one `Render` call in a fresh process, `--preview` serves
   one `Preview` stream on the machine's default ALSA output and exits 6 without one — and the
   stdio path is deleted. A mix lane drives Tracktion's fader in slider position, split to
   0.02 dB with its smoothing zeroed; a stretch Rubber Band objects to is refused.
5. `tests/`: the four golden WAVs did not move over the new transport. `renders.rs` gains the
   fader ride, the export process refusing `Preview`, the stretch refusal, and a device test that
   is `#[ignore]`d because no machine this repository has run on had a real audio output — it has
   passed only against ALSA's `null` PCM, which does not pace. `render-once`
   is the client the engine job's shell steps render through, `determinism/undo` is a fifth
   script, and CI gains an `app` job.

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
