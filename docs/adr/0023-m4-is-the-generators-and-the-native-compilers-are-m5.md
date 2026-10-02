# ADR 0023 — M4 is the generators, the native compilers are M5, and there is no JIT in v1

- **Status:** Accepted (2026-09-25)
- **Affects:** `docs/specs.md` §3, §7.2, §7.3, §8, §9, §12, §13, §15, §16, §17 and §18.2;
  `docs/plan.md`; `docs/roadmap.md`; `CLAUDE.md`; ADR 0002 §7 and Consequences, ADR 0003
  §3, §4, §7, §8, §9 and Consequences, ADR 0010 §5, ADR 0015 §1, ADR 0007 §6, ADR 0014 §2 and
  ADR 0020 §5, each amended in place
- **Builds on:** ADR 0003 §1 (§16 is authoritative, so renumbering it is an ADR) and its
  Consequences ("if M4 needs splitting, that is a later ADR against a real schedule"); ADR 0003
  §7, resolved 2026-09-25 (the neural runtime is a separate process); ADR 0009 §1 and §6 (what
  "deterministic" claims, and that one machine is not evidence); ADR 0010 §1 (a pin is a commit,
  never a branch) and §5 (Airwindows deferred "with clap-wrapper"); ADR 0008 §2 (a fresh process
  per render, and why); ADR 0022 §5 (a deferral is said in an ADR rather than by silence)
- **Recorded in:** `docs/specs.md` §15, with §3, §7.2, §7.3, §8, §9, §12, §13, §16, §17 and
  §18.2 amended to match.

## Context

ADR 0003 placed eight pieces of unplaced scope in 2026-09-02 and said in its own Consequences
that M4 "grows the most" and that splitting it would be "a later ADR against a real schedule,
not a guess now". The M4 plan (`docs/plan.md`, "The split, and where the seam is") is that
schedule: read as one milestone, M4 is a compiler in Python, a compiler toolchain in C++ with a
JIT in the audio process, a plugin-export pipeline with a compile step on the user's machine, a
second native runtime with its own process and pins, two code views, an effects set, a schema
change and `lock.json`'s last block — twenty-odd pull requests into an engine build no runner
has executed since 2026-09-09.

The plan asked the user ten questions and an agent seventeen. The spike (PR #83, branch
`m4.0-spike`, **never merged**) took nine of ten measurements on 2026-09-25 against `main` at
`1e61d09`, on one machine; its findings are carried onto `main` in `docs/plan.md`, "What the
spike found (PR 0, run 2026-09-25)", and this ADR cites them from there. The user answered the
ten on the same day. Three of the answers reverse or renumber a recorded decision, which is why
they are an ADR and not a plan row: the split renumbers §16 (ADR 0003 §1), the JIT answer
reverses §15's "JIT for editing" row of 2026-09-02, and the neural answer narrows what §7.3
describes to what the spike found the toolchain can do.

Two of the plan's premises were measured wrong, in opposite directions, and both are recorded
here rather than quietly corrected. The plan expected the JIT and the exported C++ to disagree
to the bit; they agree exactly at the default optimisation level. The plan read `cmaj_clap` as
needing CLAP headers only; it needs GTK3 and WebKit2GTK, and the built plugin's closure is 129
shared objects.

## Decisions

### 1. M4 is the generators; M5 is the DSP compilers and the neural path, with Airwindows; M6 is interop and polish

**U1, decided by the user.** §16 is renumbered: **M4 – Generators**, **M5 – DSP compilers and
the neural path**, **M6 – Interop & polish**, the last being §16's former M5 line unchanged.

M4 is exactly the half that is ours end to end: the seeded Python DSL (§7.1), its sandbox, the
two tools `define_generator` and `compile_generator`, `Generator.compiled_hash`, the generator
code view, and `lock.json`'s `toolchains` block. **It adds no external dependency** — not one
package: `pydantic`, `grpclib` and `uv_build` are pinned, and `ast`, `fractions`, `random` and
`resource` are the standard library. It runs under the `checks` job with no engine build, and
what it claims is integer arithmetic a golden can hold. M5 is everything native: Cmajor,
clap-wrapper, a CLAP SDK, a VST3 SDK, a compile step on the user's machine, the ONNX converter,
Airwindows, and the DSP code view.

The seam is *native against not*, and the reason it is the right seam is not only that the
first half is cheaper. It is that the generators exercise every seam the native half needs from
`core` before any native risk is taken: the compile-tool contract, compiled output beside its
source, staleness by hash, diagnostics fed to the model, a code view over a source field, and a
toolchain pin that is checked at compile and not at open. M5 then has only native questions
left. The alternative seam — JIT now, export later — puts LLVM into the audio process first,
with no runner, and delivers an instrument that previews and never renders; not splitting is a
milestone that cannot close for months. Both were the plan's to lay out and the user's to
refuse.

**Which §11 bullet each half answers**, because the plan's ADR row asked for it. M4 answers the
first bullet's `compilers` half — no unseeded randomness and no clock in the sandbox (ADR 0024
§4) — the third bullet's `lock.json` half for a toolchain (ADR 0027 §2), and the determinism
suite's bullet with a sixth script, `generators`, driven over both transports and goldened. M5
answers the golden-render bullet for each compiled device kind, and `lock.json`'s `artefacts`
and `models` blocks. Neither half changes what the four golden WAVs claim.

**What moves with the renumbering, each amended in place and dated:**

- ADR 0003 §3: `lock.json` is "completed" at **M5**, with the `toolchains` block landing in M4
  (ADR 0027 §1). ADR 0010 §1's "M4 adds the compiled artefacts" reads M5 with it.
- ADR 0003 §4 and ADR 0010 §5: Airwindows is **M5's**, and not "with clap-wrapper" — decision 5.
- ADR 0003 §7: the neural runtime is **M5's**, and what v1's neural path *is* is decision 3.
- ADR 0003 §8: the code views split with their compilers — the generator view is M4's, the DSP
  view M5's.
- ADR 0003 §9: MIDI I/O, the REAPER path and schema publishing are **M6's**. §18.2's "Stage 2
  (M4–M5)" reads M5–M6.
- ADR 0007 §6's "(M4)" beside `DeviceRef.cmajor`, `.faust` and `.neural` reads M5, and its
  "(M5)" beside `RENDER_KIND_STEMS` reads M6; ADR 0014 §2's and ADR 0020 §5's installer, M6.
  Each carries a one-line dated note rather than a rewrite, because the decision in each is
  unchanged and only the milestone's number moved.

`roadmap.md`'s spine and `CLAUDE.md`'s milestone sections move with it. The installer waits one
milestone more, which is the split's one cost to a user, and it is accepted.

### 2. No JIT in v1: a Cmajor instrument is exported and loaded, and §15's "JIT for editing" is reversed

**U3, decided by the user as option (c).** An edit to a Cmajor source is *export and load*:
generate, compile, wrap, load into the live preview. There is no `libCmajPerformer` in the
engine, no `CmajPlugin` bundled, and no LLVM in any process this repository ships. §15's row
"Cmajor placement — JIT for editing, exported CLAP→VST3 for persistence" of 2026-09-02 is
**reversed**, in place and dated, and §7.2's two modes become one.

The reversal rests on a measurement rather than on a preference, and the measurement went the
opposite way from what the plan assumed. Item 3 of "What 'deterministic' means for a compiler"
said the JIT and the exported C++ were "two compilers over one source with no reason to agree to
the bit", and that is what made the JIT a *preview* by necessity. Measured on 2026-09-25: one
patch rendered through `cmaj render --engine=llvm` (the LLVM JIT), through `--engine=cpp` (the
generated C++ the export compiles) and through the built, clap-wrapped `.clap` driven by a CLAP
host gave **one PCM hash, `262509a0…`, 0 differing samples of 96,000**, at the toolchain's
default optimisation level and at `-O0` through `-O3`; a second patch written to break it —
per-sample `sin`, `cos`, `exp`, `log`, `sqrt`, `tanh` and eight multiply-add accumulators —
agreed too. They part company only at **`-O4`**, where 8,499 of 96,000 samples differ, peaking
at 7.451e-09, which is −162.6 dBFS and still a different hash. So the JIT is not forced to be a
preview by the numbers; whether to carry it at all is a cost question, and the cost was measured
beside it: an export is **≈20 s cold** on this machine (generate 0.095 s, configure 0.61 s, build
19.5 s), and *worse* warm, because regenerating rewrites `entry.cpp` and nothing is incremental.

What (c) buys is a stronger claim than M2's *live preview · not the render*: for a Cmajor device
the preview and the export are **the same binary**, so a person hears what a render will hash.
What it costs is a twenty-second loop that a person will feel and a model will not. The JIT is
therefore a **deferred-ledger row with a trigger a person can fire**: the first session in which
the export latency is counted as friction — measured, not argued — and the row names what would
then test the alternatives (`libCmajPerformer` in the engine against trap 13; `CmajPlugin` as a
fourth bundled plugin against its GTK and WebKit cost, decision 4).

Two things follow for M5 and are said now so its plan inherits them. **The optimisation level
is a pin**: an exported artefact's `lock.json` record (ADR 0027 §3) names the `-O` it was built
at, because `-O4` is a different hash from `-O2` inside one engine. And Plate 3's *export
pending* state does not disappear with the JIT; it changes meaning: a Cmajor device whose
source has no export yet cannot be **played** either, and `render_unsupported` on it becomes
`export_pending` (M5's `core` PR), naming the tool that lifts it.

### 3. The neural path in v1 is the converter alone; no runtime process and no model ship

**U6, decided by the user as option (c), with (b) as the runtime's shape if it ever comes.**
Cmajor's tree ships `tools/onnx/onnxToCmajor.py`, which converts an ONNX graph over sixteen
operators into pure Cmajor. In v1 that converter **is** the neural path: a model that rides it
is exported, wrapped, hashed and rendered under the DSP pipeline (decision 2), and there is **no
fourth process**. ADR 0003 §7's decision that the runtime is a separate process stands and is
not exercised: the process it describes is not built in v1. ONNX Runtime therefore reaches no
render and no binary, and §17's row says so.

The spike narrowed this below what the plan hoped, and the ADR records the narrowing rather than
the hope. A feed-forward graph (`MatMul`/`Add`/`Tanh`/`MatMul`) converts, compiles, renders and
exports — 16.9 s to a `.clap` the CLAP host drove to 76,800 non-zero samples — and a `GRU` +
`Squeeze` + `MatMul` + `Tanh` graph converts and compiles. But **`Gemm`'s bias does not
broadcast** (a DDSP-shaped `GRU` + `Gemm` fails in the Cmajor compiler, `Cannot connect Bd.out
(float32[1]) to node_3.add (float32[16, 1])`), so a dense layer must be written as `MatMul`
plus a full-rank `Add`. And **an operator outside the sixteen is not refused**: `Softmax`
converted, `onnxToCmajor.py` exited 0, and the failure arrived one stage later from the Cmajor
compiler as `Type references are not allowed in this context`, which does not name the operator.
So "refused, with the operator named" — U6's own sentence — is **something M5 writes**: the
pipeline checks the graph's operator set against the sixteen *before* it converts, and names the
first operator outside it. That is a decision about shape and not machinery; it is one walk over
the graph's nodes.

Two more of the spike's findings are M5's traps and are recorded in the plan: `cmaj play
--dry-run --stop-on-error` exits 0 on a compile error and `cmaj render` writes a zero-length
`data` chunk and exits 0, so a compile step reads stderr and never the status; and the converted
patch's I/O is the model's tensor shape (64 in, 16 out on the GRU graph), so something adapts it
before it is an effect.

**No model ships.** §17 pins a runtime and no weights, §14.5 wants a golden per model, and the
golden for the neural path is a **synthetic ONNX graph**, committed, so a person's model is a
person's asset and this repository redistributes nothing it cannot. The runtime, if ever, is
option (b) — a fresh process per render that takes a track's notes and automation and writes a
stem *asset* the engine plays as an audio clip, so its output is hashed and its dispatch is
contained at compile time exactly as an export's is — and it is a deferred row triggered by the
first model a person brings that the converter refuses. **Its directory is `compilers/neural/`**,
under §13's existing `/compilers` line, so it is not a top-level directory and ADR 0003 §7's
"an ADR of its own" is satisfied by this section and by M5's neural ADR rather than by a
directory ADR. Which transport it would speak is nothing in v1, and is not decided against no
consumer.

One measurement that could not be taken is recorded as such: `onnxruntime` 1.29.0 exposes no
knob over MLAS's instruction-set dispatch and this machine has one CPU and no emulator, so item 4
of "What 'deterministic' means" — that a runtime's kernels choose by ISA at run time — stays
certain in principle and unmeasured here. It bears on nothing v1 ships.

### 4. The pin set for M5 is approved by measurement, and lands when M5 vendors it

**U4, U5 and U7, decided by the user on the spike's numbers.** Recorded here as *approved* under
CLAUDE.md #4, with where each lands; **`lock.baseline.json` does not move in this pull
request**, for the rule the user set at M3 (U4: pinned by exact version "when it is first
added, not before") and for the file's own rule (`commit: null` "must be resolved and filled in
when the dependency is first vendored"). M5's vendoring pull request moves it and takes §17's
golden-render pass, which every one of these pins can reach.

| Component | Approved as | Why this and not the other |
|---|---|---|
| **Cmajor** | Source at `024a208515f15e43271d9b2ea85ee22a2233384b` (1.0.3177), `cmaj` and its plugin helpers only, with `3rdParty/llvm` (`cmajor-lang/llvm`, `c6380f9`) checked out **sparse** to `release/linux/x64` — 361 MiB in 2,273 files, prebuilt static libraries, linked and never rebuilt — and `include/choc` at `a08bfd8`. Every one of its thirty-two submodules is declared with an SSH URL and is rewritten at checkout | Measured: 57.9 s for the submodules, 15.5 s for the sparse LLVM, and a Release build of **126.9 s wall, 947 s CPU, 2.4 GiB peak, 128 MiB binary, zero warnings** under the engine's own `-march=x86-64 -mtune=generic -ffp-contract=off`. A release binary's asset can be re-uploaded under its tag; a commit cannot. The LLVM is a vendor binary under a commit, which is the standing a PyPI wheel has under §17's registry rule, and it is written down as that |
| **CLAP SDK** | **1.2.10 (`195b42a`)**, §17's row — and **not** Cmajor's own `3rdParty/clap` at `df8f16c`, which is CLAP **1.2.0** | clap-wrapper v0.16.0's `src/clap_proxy.h:28` includes `clap/ext/draft/gain-adjustment-metering.h`, which 1.2.0 does not ship and 1.2.10 does. And there is **one CLAP SDK by construction**: `guarantee_clap` returns early `if (TARGET clap)`, and the generated project defines `clap` over whatever `--clapIncludePath` was given, so the SDK the caller passes to `cmaj generate` is the one clap-wrapper uses and `CLAP_SDK_ROOT` is reported unused. The full commit is resolved from the tag when M5 vendors it, as §17's row has always said |
| **clap-wrapper** | v0.16.0 (`1cca996e96f29ab2be7ae9f8cfe532bbc92e1dd6`), passed as `CLAP_WRAPPER_PATH`; not Cmajor's two-year-older fork at `fd24bbe` | `target_add_vst3_wrapper(TARGET … OUTPUT_NAME …)` in the 2026 wrapper is exactly the call the generated project makes, so nothing drifted. Its static libraries are not built `-fPIC`, and the VST3 link fails without **`CMAKE_POSITION_INDEPENDENT_CODE=ON`** — one cache variable, documented nowhere the generated project points at, recorded here so M5 does not find it in a link error |
| **VST3 SDK** | `v3.8.0_build_66`, `9fad9770f2ae8542ab1a548a68c1ad1ac690abe0`, with **four submodules** (`base`, `public.sdk`, `pluginterfaces`, `cmake`): 6.3 s and 34.7 MiB. **A new §17 row**, signed off 2026-09-25 | clap-wrapper's `base_sdks.cmake` fetches it from GitHub *at configure time* under `CLAP_WRAPPER_DOWNLOAD_DEPENDENCIES`, which Cmajor's generated project sets `TRUE` unconditionally — ADR 0009 §4's Rubber Band hazard exactly, the build machine deciding a pin. Vendored and passed as `VST3_SDK_ROOT` with every download flag off, **the configure takes 0.67 s and reaches no network**, and that configure run with the network unreachable is M5's check that can fail (trap 14). It is a *second* VST3 SDK beside the one JUCE embeds for hosting; the two never meet, since one builds a plugin and the other loads it |
| **Airwindows** | `baconpaul/airwin2rack` at `b6eef0a` (`libs/airwindows` `d22a25b`, `libs/sst-rackhelpers` `f5f3332`), built as a **JUCE plugin** (`BUILD_JUCE_PLUGIN=ON`) — Dexed's shape, an `ExternalProject` with its own JUCE — with **JUCE 8.0.4 and `clap-juce-extensions` overridden** (`FETCHCONTENT_SOURCE_DIR_*`, or vendored), and no clap-wrapper anywhere | `airwindows/airwindows` builds VST2 and AU and no Linux VST3, which was ADR 0010 §5's suspicion, confirmed. airwin2rack builds one in three minutes (configure 50.4 s, build 113.4 s, 2,194 targets, 878 MiB) and `--scan` opens it headlessly with **seven** shared objects in its closure. But its `src-juce/CMakeLists.txt` fetches `clap-juce-extensions` at **`GIT_TAG main`** at configure time — a moving branch, which breaks ADR 0010 §1's rule by construction — so both fetches are overridden to commits M5 names. See decision 5 for what the plugin's shape does to `add_effect` |

Two facts about the **build image** come with the set, and both correct the plan:

- **`libjack-jackd2-dev` is a build dependency of `cmaj`, of Dexed and of airwin2rack** on the
  pinned image. `cmaj` wants JACK in three places — `modules/CMakeLists.txt:447` makes
  `pkg_check_modules(JACK REQUIRED jack)` a hard gate and then never uses `JACK_LIBRARIES`;
  choc's `choc_RtAudioPlayer.h:34` defines `__UNIX_JACK__ 1` on Linux so RtAudio includes
  `jack/jack.h`; `tools/command/CMakeLists.txt:64` appends `-ljack`. The spike's machine has no
  `jack.pc`, `libjack.so` or `jack/jack.h`, so its `cmaj` was built with **three one-line
  deviations** from `024a208` — the `REQUIRED` dropped, the define dropped, `-ljack` dropped —
  and airwin2rack needed `-DJUCE_JACK=0`. That is the same missing header that has stopped the
  plugin manifest being rebuilt since 2026-09-09. **The package is installed rather than the
  three lines deleted**: M5 builds the pinned tree unmodified, and the deviation is recorded here
  so nobody mistakes the spike's build for the pin's. The user is installing it.
- **The Cmajor tool *and* `cmaj_clap` need GTK3 and WebKit2GTK on Linux.** The plan read
  "`cmaj_clap` needs CLAP headers only"; it was wrong. `tools/command/CMakeLists.txt` requires
  both for the command-line tool, and `helpers/common/CMakeLists.txt`, which the generated CLAP
  project pulls in, makes both `REQUIRED` for `cmaj_clap` too. The built `.clap` has eleven
  direct `DT_NEEDED` entries including `libwebkit2gtk-4.1`, `libgtk-3`, `libsoup-3.0` and
  `libjavascriptcoregtk-4.1`, and **129 shared objects in its closure** — GL, EGL, X11, dbus,
  at-spi, gstreamer and `libenchant`, a spell checker, in the audio path. It loads headlessly
  regardless. What this costs is M5's engine image growing by the whole desktop stack, and a
  plugin whose determinism note must say that none of those 129 objects is on the render path —
  which is an assumption until M5's golden holds across two independent builds, and is named as
  one.

Both facts were what made U3's option (b), `CmajPlugin` as a fourth bundled plugin, more
expensive than it read, and they are part of why (c) held.

### 5. Airwindows is M5's, as one JUCE plugin whose `Replace` parameter selects the effect

ADR 0003 §4 (amended 2026-09-05) and ADR 0010 §5 deferred Airwindows to M4 "with clap-wrapper",
because its pinned repository "may not build a Linux VST3". Half of that premise was wrong and
the half that was right leads elsewhere, as U7's row said and the spike confirmed: the
maintained build is airwin2rack, a JUCE plugin, and it needs no clap-wrapper at all. **Both
sections are amended in place**: Airwindows is **M5's**, re-pinned to airwin2rack at the commit
in decision 4, landing as a fourth bundled plugin in M1 PR 6's shape with its golden, in the
pull request that also rebuilds the manifest for the first time since 2026-09-09.

The spike read the plugin's shape and it wants one sentence now, before anything is built on
it. The scan reports `Airwindows/Airwindows Consolidated`, version `1.2026.269`, **fourteen
parameters** — `Replace`, `Brightness`, `Detune`, `Bigness`, `Dry/Wet`, five slots reported as
`-`, `Bypass`, `Input Level`, `Output Level`, `Mono Behaviour`. That is **one plugin whose
`Replace` parameter selects the effect**, with five generic controls whose meaning changes with
it. So `add_effect` over Airwindows is a plugin id plus a parameter *value*, and a `ParamRef`
to `Brightness` means something different depending on `Replace` — which is the §2.2 randomness
row's problem in another hat: a parameter whose meaning depends on another parameter. M5's
Airwindows ADR decides how the document names an effect — a `Replace` value in `params`, or
something the validator can read — and this ADR records only that the question exists and is
not answered by a plugin id.

The re-pin tool's ledger row is **retriggered to that pull request** (question 16), because it
is the first that moves a plugin pin since M1; ADR 0022 §5 said the same for M3 and this ADR
says it for M4 rather than letting a fourth milestone's silence read as a promise.

### 6. The ledger rows due at this planning are decided, and each says how

The fifth walk of the deferred ledger (2026-09-24) sent four rows here, and question 7, 15 and
16 add three dispositions of their own:

- **`FormRule`** (ADR 0002 §7, "deferred to M4") — **re-deferred on an event**, and ADR 0002 §7
  is amended in place. The consumer the deferral waited for is the generative compiler, and the
  compiler does not need it: a generator reads the layer-2 `sections` it is handed (ADR 0026
  §1), which already carry order and length. What a `FormRule` would add is a rule to re-lay
  sections when `target_length` changes — a second compiler, for which nothing has asked. Trigger:
  the first time a person wants a form re-laid rather than re-typed. **`set_form`** goes with it:
  without a rule it is `add_section` × N and one `apply_patch`, both of which the model holds;
  trigger: the first measurement in which a model fails to lay a form out with the tools it has.
  `proto/song_tools.proto`'s comment moves its milestone rather than losing its sentence.
- **`SourceRef.export_hash` and the `Generator` compiled-source hash** — **split**:
  `Generator.compiled_hash` is M4's under ADR 0025 §1; `export_hash` waits for its producer,
  M5's export pipeline, under ADR 0025 §2.
- **`lock.json` beyond `schema_version`** — **split**: the `toolchains` block is M4's under ADR
  0027 §1; `artefacts` and `models` are M5's, their names reserved by ADR 0027 §3.
- **Strudel** — re-deferred unchanged, **after M5**: §15's reason, a JS runtime and AGPL Strudel
  in the core, is untouched by anything M4 learns, and `GeneratorKind` has left room since M0.1.
- **Faust** (question 15) — not M4's and **not M5's without a consumer**. §7.2 keeps it as an
  import path and §17 says "pin hash when vendored"; nothing has asked to import one, and a second
  DSP toolchain doubles decision 4's pin set for no caller. `DeviceRef.faust` stays validator-
  accepted and compile-refused; a ledger row carries it on the first source somebody wants
  imported.
- **The re-pin tool** (question 16) — retriggered to M5's Airwindows pull request, decision 5.

### 7. Three things a person owns, deferred with their triggers, and one decision that is not yet in effect

- **U9 — whether §12's "Cmajor … generated code carr[ies] no copyleft" holds for an export.**
  Deferred by the user: a person reads Cmajor's licence page **before M5's first export lands**,
  and §12's sentence is confirmed or corrected then. An exported plugin is generated C++ *plus*
  the helpers `cmaj generate` unzips beside it (`cmaj_CLAPPlugin.h`, `cmaj_plugin_helpers`), which
  come from a GPLv3-or-commercial tree; whether a person's exported instrument is GPL-derived is
  a licence reading nobody in this repository has made. **M4 exports nothing, so nothing blocks.**
  §12 carries the sentence as unverified with this trigger; it is a **deferred check** in the
  ledger, the third of that kind.
- **U10 — no paid measurement before the DSL exists.** CLAUDE.md #7 stands; `CEILING_USD` in
  `ai/src/escribass_ai/provider.py` equals the ledger's spent total, so every live run fails
  closed until a person raises the number in a commit. Whether the default model writes the DSL
  is offered **once, after M4 PR 6**, at a stated cap, if the user wants the number; until then
  the DSL is designed for a human author and a model retries against the diagnostics it is fed
  (ADR 0026 §3), which is §6.1's loop doing its job. M4 PR 6's transcript is hand-written and
  says so, as `four-refusals.json` does.
- **U2 — the repository is going public**, to restore CI: public repositories' Actions minutes
  are free, and the `cross-cpu` job needs GitHub's own pool, which nothing else provides. **At
  the moment this pull request opens the repository is still private** (`gh repo view` on
  2026-09-25), every run since M2 PR 8 has reported `failure` with zero steps executed, and PR
  #83's and #82's runs this morning did the same; so this pull request cannot claim to be the
  first a runner sees until the change takes effect, and its description says which it was.
  What going public changes in the record: `.github/workflows/checks.yml`'s second gate note
  ("this repository is private on a plan where the branch-protection API answers 403") and the
  root `AGENTS.md`'s hooks paragraph ("branch protection and rulesets both require Pro on a
  private repository") stop being the reason those two are shaped as they are, and both are
  corrected when the visibility is; `cross-cpu`, which ~~has **never run**~~ **had last run on
  2026-09-09, twenty-five times before that, and whose evidence included two Intel parts nobody
  had read** — `renders` and
  `cross-cpu` last executed on 2026-09-09 and the matrix's chance of drawing two CPU models has
  not been taken since — becomes the first place ADR 0009 §6's question can be asked again; and
  "every number is one machine's" stops being true of the next pull request that touches the
  engine, not of this one.

  **Corrected 2026-10-02, at M4's close.** "Never run" was false when it was written and the
  clause after it said so in the same breath — a job that "last executed on 2026-09-09" has run.
  ADR 0009 §6's 2026-10-01 amendment established forty-three executions, twenty-five of them M1's
  and M2's, and wrote the rule this bullet breaks: *before writing "for the first time" or
  "never", list the runs.* That amendment did not propagate here, against
  `docs/adr/AGENTS.md`'s "other ADR text that relied on the old decision changes in the same
  commit" — so the sentence ADR 0009 corrected survived two days in the ADR beside it. The
  decision this bullet records (U2, public) is untouched; what was wrong is a claim about this
  repository's own evidence, for the third time in one ADR family.

## Alternatives considered

**The split (U1)**

| Alternative | Rejected because |
|---|---|
| One milestone, as §16 had it | Twenty-odd pull requests and two whole-stack reviews into an engine build no runner has executed since 2026-09-09, with nothing shipped until all of it lands — the exact thing ADR 0003's Consequences warned of |
| Split at "JIT now, export later" | LLVM into the audio process first, with no runner (trap 13), delivering an instrument that previews and never renders — the weaker of the two proof points the roadmap names; and the measurement then removed the JIT altogether |
| Split but keep Airwindows in M4 | The generators half has no effect-chain consumer, and the first pull request to touch the plugin set meets the manifest not rebuilt since 2026-09-09 and the missing `jack/jack.h`; it is one plugin PR of M1 PR 6's shape whenever it lands, and it lands with the milestone that builds the engine |

**The JIT (U3)**

| Alternative | Rejected because |
|---|---|
| (a) `libCmajPerformer.so` inside the engine | An abort inside the JIT is an abort on the audio thread — ADR 0003 §7's reason for moving ONNX Runtime *out*, one dependency over — and `cmajor-lang/llvm` is prebuilt by upstream's `build.pl`, so `-ffp-contract=off` and `-march=x86-64` reach none of it and `check_flags.cmake` cannot see inside an archive |
| (b) Cmajor's own `CmajPlugin` as a fourth bundled plugin | A fourth JUCE, GTK3 and WebKit2GTK in the engine image (decision 4 measured the closure at 129 objects), a patch loaded headlessly by a mechanism the spike did not find, and a preview that is still *not the render* |
| (c) with the JIT re-added when latency is measured as friction | **Taken.** The cost of being wrong is a twenty-second loop a person will feel, and the row's trigger is exactly that measurement |

**The neural path (U6)**

| Alternative | Rejected because |
|---|---|
| (a) A real-time bridge: a CLAP shim exchanging blocks with the ONNX process over shared memory every callback | The largest native item in the milestone, built for a model nobody has named; and item 4's dispatch hazard would then sit on the render path of every project that used it |
| (b) The runtime as an offline compiler now | No model ships and none has been asked for; (b) is kept as the runtime's *shape* when one is, so the row's trigger is a real request rather than a milestone |
| Ship a model to have a golden | §17 pins a runtime and no weights; a real model's weights are a redistribution question this repository has not read, and a synthetic graph goldens the path without it |

**The pins (U4, U5, U7)**

| Alternative | Rejected because |
|---|---|
| Cmajor's 1.0.3177 release binary | Two assets the API refuses to list, hashes not read, and a tag under which an asset can be re-uploaded; a commit needs no such check |
| Cmajor's own CLAP submodule as the pin | CLAP 1.2.0 lacks the draft header clap-wrapper v0.16.0 includes; it does not compile |
| Cmajor's clap-wrapper fork | 2024-08-19, two years older than §17's pin, for a generated project that already accepts `CLAP_WRAPPER_PATH` |
| Let clap-wrapper download the VST3 SDK | A configure that reaches the network is the build machine deciding a pin — ADR 0009 §4's hazard, and ADR 0010 §1's rule broken |
| airwin2rack at `GIT_TAG main` as it ships | A moving branch inside a pin; overridden to commits |
| Delete `cmaj`'s three JACK lines, as the spike had to | A tree that differs from the pin by three lines is not the pin; the package is installed instead and the deviation is recorded here so the spike's build is not mistaken for it |
| Move `lock.baseline.json` in this pull request | U4's rule — pinned when first added — and the file's own `commit: null` rule both say the vendoring pull request moves it; and the file is excluded from neither CI path gate, so touching it in a decisions PR would ask for a golden-render pass on a change that renders nothing |

## Consequences

- §16 is renumbered; §15 gains this ADR's rows and strikes "JIT for editing" in place;
  §3's tier table, §7.2, §7.3, §8, §9, §12, §13, §17 and §18.2 are amended to match. Three
  `[OPEN]` items remain in §15 and this ADR walks into none of them.
- ADR 0002 §7 and Consequences, ADR 0003 §3, §4, §7, §8, §9 and Consequences, ADR 0010 §1 and
  §5, ADR 0007 §6, ADR 0014 §2 and ADR 0020 §5 are amended in place and dated; ADR 0015 §1 gains
  the hazard ADR 0026's Consequences record for M5 (a CLAP parameter id is a Cmajor endpoint
  ordinal).
- `docs/plan.md` carries the spike's findings onto `main`, records the ten decisions with their
  dates, rewrites items 2 and 3 of "What 'deterministic' means" against the measurement, and
  gains the M5 traps this ADR names. `docs/roadmap.md` and `CLAUDE.md` move to the split.
- **Nothing is pinned, nothing is installed, and no golden moves.** `lock.baseline.json` is
  byte-identical to `main`'s; the `jack` package is the user's to install and M5's to require.
- The ledger gains rows for the JIT, Faust, the neural runtime process, U9's licence reading and
  U10's measurement, each with the trigger above; the four rows due here are dispositioned as
  decision 6 says.
- **What this ADR rests on that is unmeasured**, named so M5's plan tests it rather than inherits
  it: that none of `cmaj_clap`'s 129 shared objects is on the render path (a golden across two
  independent builds on two machines would say); that the byte-identical export the spike
  measured on one machine holds on a second (U2's runner); and that a synthetic ONNX graph is
  representative of what a person will bring, which nothing can measure until a person brings one.
