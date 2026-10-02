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

## Current milestone: M5 — DSP compilers and the neural path

M0 to M4 are complete: the schema, `core` and the tool API; the render engine and four golden
WAVs; the Tauri app with its views as projections of the model and preview playback; the AI loop —
a Python sidecar, a proposal a person applies, and the panel; and the **generators** — a seeded
Python DSL behind an `ast` allowlist, compiled in a subprocess `core` spawns per compile, with the
generator code view and `lock.json`'s `toolchains` block. §16 makes M5 the **native** half of what
was once one milestone (ADR 0023 §1, the user's decision of 2026-09-25): Cmajor export through
`cmaj generate --target=clap`, compiled under the engine's pinned flags and wrapped to VST3 via
clap-wrapper, with **no JIT in v1** (§7.2); the ONNX-to-Cmajor converter as the whole of v1's
neural path — **no runtime process and no model shipped** (§7.3, ADR 0023 §3); **Airwindows** as a
fourth bundled plugin, built from `baconpaul/airwin2rack` as a JUCE plugin needing no clap-wrapper
(§8, ADR 0023 §5); the DSP code view (§9); `lock.json`'s `artefacts` and `models` blocks, which is
where ADR 0003 §3's "`lock.json` completed" now falls; and `SourceRef.export_hash`, which finally
has a producer. Interop, polish and the installer are **M6**.

**M5 is not planned yet, and that is deliberate**: planning it is its own step, as M1's, M2's,
M3's and M4's were, and it is the step after M4's close. `docs/plan.md`'s "M5, sketched from here"
is the rows a planner would otherwise rediscover — twelve of them, sized by the M4 spike's own
native measurements — and it is a sketch, not a plan.

**What M5's planning owes.** ADR 0003 §7's `[OPEN]` item is **answered and its detail is paid**:
the neural runtime is a separate process that never links into `engine`, decided by the user on
2026-09-25, and ADR 0023 §3 then answered the four things §7 had left to M4's planning, narrower
than §7 imagined — in v1 there is **no transport**, because the process is not built and no model
ships; what else would be pinned is the `onnx` PyPI package the converter imports, which **returns
to a person before M5 installs it** (CLAUDE.md #4); §7.3's CLAP wrapper is the DSP pipeline's,
since a converted model *is* a Cmajor patch; and the directory is `compilers/neural/`, under §13's
existing `/compilers` line, so it is **not** a top-level directory and needs no ADR of its own.
Do not reopen any of that. What is genuinely owed is M5's own: its ADRs — the export record and
`SourceRef.export_hash` (schema before the `.proto`, CLAUDE.md #5), the export pipeline as a
subprocess with a timeout and diagnostics, the engine loading an artefact from a path, the
`artefacts` and `models` blocks extending ADR 0027 §3, Airwindows re-pinned and its
effect-selecting `Replace` parameter decided **before `add_effect` names one**, and U9's licence
reading recorded; a `ParamID` for a compiled device — an endpoint ordinal or a name mapped to one —
which is due before the first `add_automation` can target one (ADR 0015 §1's hazard); and §11's own
M5 bullet, a golden per compiled device kind.

**Four things to carry into every M5 step.** **`lock.baseline.json` moves when M5 vendors, and the
first pull request that touches it should expect a cold engine build**, because the ccache key
hashes that file. What it owes: Cmajor's `3rdParty/llvm` sub-pin, the CLAP SDK's commit, a **VST3
SDK row it does not have at all** — §17 calls it "a new row" and the baseline has no counterpart —
and `bundled_plugins.airwindows`, still upstream `ab0d1df871b8` with a note that reads "deferred to
M4 with clap-wrapper", a sentence four documents have struck. **A person reads Cmajor's licence
before M5's first export** and confirms or corrects §12's "generated code carries no copyleft"
(U9). **The plugin manifest has not been rebuilt since 2026-09-09** and M5's Airwindows pull
request is the first that must, with `libjack-jackd2-dev` installed rather than JACK compiled out.
And **a push to `main` is watched by nobody**: `main` has no branch protection, there is one
workflow and no schedule, so a push run that fails is a page nobody opens — #89's sat cancelled for
a day (docs/plan.md, "M4, closed").

Spending still fails closed at a **committed grant**: `GRANT_USD` in
`ai/src/escribass_ai/provider.py` is `0.0` and is read before any price is fetched, because a
hosted route listed at $0 is still a call to a metered account — raising it in a commit is what a
grant is. The ledger is three rows and $0.00376174, and **no paid measurement of a model writing
the DSL has been made** (U10).

Live status, deferred items and known gaps: `docs/plan.md`. Read `docs/specs.md` §16, §7 and
ADRs 0023–0027 before starting a step, and **read `docs/plan.md`'s "M4, closed" first**: it records
what M4 leaves unverified — ARM and consumer silicon untested with every one of six CPU models an
x86-64 server part, no evidence any real model writes this DSL, a preview that has still never
reached a speaker, one surviving route out of the DSL's purity, and a `lock.json` a model's compile
leaves unpinned.

## Completed: M4 — Generators

1. ADRs 0023–0027: the split at the native seam and what M5 and M6 now own; the sandbox as a
   subprocess `core` spawns per compile and the DSL as a Python subset over `int` and `Fraction`
   behind an `ast` allowlist, claiming purity and a resource limit and **not** security;
   `Generator.compiled_hash` alone, with `SourceRef.export_hash` waiting for M5's producer;
   `proto/generate.proto`, the two tools and fourteen offered; and a `toolchains` block checked at
   compile and never at open. **One `song.proto` change**, field 11, additive, `schema_version`
   still 1 — the first since M1 — and `proto/generate.proto` is new.
2. `compilers/generative/`: a Python 3.12.12 package under `uv` at DSL version `1`, adding **no
   external dependency** to the approved set. An `ast` allowlist, a namespace written out in the
   ADR and asserted against the code, a seeded `rng` of six draws, `PYTHONHASHSEED=0` by re-exec,
   and `RLIMIT_CPU`/`RLIMIT_AS` in the child beside a wall clock in `core`. 49 tests: every
   forbidden construct fed and watched refused with its line, each arm watched failing first.
3. `core`: the two tools, ending in `Session::run` as every tool does; the sandbox spawned per
   compile and told `--generator <cmd>`; the clip's notes replaced whole with `compiled_hash`
   beside them, one entry under `compile_generator`; three refusals and four operator errors, each
   watched failing first; `lock.json`'s `toolchains` block on the first committing compile; and the
   determinism suite's **sixth and seventh** scripts, `generators` and `compile`, over both
   transports.
4. `app/`: the generator code view, CodeMirror's first consumer — Save is `apply_patch`, Compile is
   a dry run then a diff then Apply, the child's diagnostic sits at the line it named, and the title
   bar reads `never · compiled · stale`, with *stale* learned from `core` and never hashed in the
   frontend. Driven by hand in the window and five defects found that way, ⌘Z reaching the
   document's `undo` from inside a text field among them.
5. **The sandbox's purity claim was false, and that is the milestone's finding.** An `ast` allowlist
   reads parsed source, and a format string is a second language inside a string constant, `repr` is
   a function of the heap, and `**` is integer arithmetic until its exponent is a `Fraction` — so
   `$HOME`, the install path and an ASLR-dependent velocity all compiled with a stable
   `compiled_hash`. Five refusals and a third lock close them; `dsl_version` stays `1`, because
   refusing more moves no golden. **One route is documented rather than closed**: an exception's
   *message* can carry a default repr, bounded by the absence of `try` and hashed nowhere.
6. `tests/`: `generators` and `compile` are goldened over both transports, the second driven by the
   model; `tests/baseline.rs` reads `lock.baseline.json` **both ways** and found three dependencies
   nobody had pinned. **The four golden WAVs did not move**, and M4 is the first milestone every
   pull request of which a runner validated end to end — ten `pull_request` runs, eight green pushes,
   and `cross-cpu` executed eighteen more times.

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
   was spent, and the ceiling was lowered to what it cost — and from M4 PR 8 the *gate* is
   `GRANT_USD`, committed at zero and read before the price, because a route listed at $0 is still
   a call to a metered account and the arithmetic let one out.
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
