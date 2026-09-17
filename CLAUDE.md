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

## Current milestone: M3 — AI loop

M0, M1 and M2 are complete: the schema, `core` and the tool API; the render engine; the Tauri
app with its views as projections of the model and preview playback. §16 makes M3 the AI loop —
a Python sidecar, a tool-calling loop with dry-run/diff/apply, and the AI panel — preceded by
the Libretto-grammar ADR §18.2 requires (ADR 0003 §6). **M3 is not planned yet**, and that is
deliberate: planning it is its own step, as M1's and M2's were.

Live status, deferred items and known gaps: `docs/plan.md`. Read `docs/specs.md` §16, §18.2 and
ADR 0003 before starting a step. Read `docs/plan.md`'s "M2, closed" first: it records what M2
leaves unverified — three merges with no green CI, a preview never played on a real device, and
three dependency sign-offs the repository cannot confirm — and one ledger row that is due
before M3's loop can call `set_param`.

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
