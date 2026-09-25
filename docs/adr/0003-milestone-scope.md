# ADR 0003 — Milestone scope

- **Status:** Accepted (2026-09-02)
- **Affects:** `docs/specs.md` §16 and §15; `docs/plan.md`; `docs/roadmap.md`
- **Builds on:** nothing; this places scope, it does not change architecture
- **Recorded in:** `docs/specs.md` §15.

## Context

§16 gave one line per milestone and was headed "(proposed)" while `CLAUDE.md`, `plan.md` and
`roadmap.md` all treated it as authoritative. Writing the roadmap surfaced two problems: §16
had fallen behind decisions already binding elsewhere, and eight pieces of scope specified in
§3, §6, §7.3, §8, §9, §10, §11 and §18.2 sat in no milestone at all.

Unplaced scope is not deferred scope. Deferred scope is in `plan.md`'s ledger with a revisit
point; unplaced scope is invisible, and gets discovered late by whoever needs it.

## Decisions

### 1. §16 is authoritative

The word "(proposed)" is dropped. `CLAUDE.md` derives M0's four steps from it and both
planning documents defer to it; it has been authoritative in practice since the first commit.

### 2. M0 includes the MCP surface, branching and merge

Already binding through `CLAUDE.md` M0 step 3, §18.2 Stage 1 ("hard requirement, not a
nice-to-have") and ADR 0001 §2 and §4. §16's line simply had not caught up.

### 3. `lock.json` is created at M0, completed at M4

§16 listed "lock file" under M4. The file is part of the `.escri` project directory (§10) and
is written by the project store at M0.2; §11 requires it checked at load. What M4 adds is the
compiled artefacts — exported CLAP/VST3 binaries, model hashes — that do not exist until the
compilers do.

- **Amended 2026-09-25, when M4 was split (ADR 0023 §1):** "completed at M4" reads **completed
  at M5**. M4 adds the `toolchains` block — the DSL's version and its interpreter's, written on
  the first compile and checked at compile rather than at load (ADR 0027 §1, §2) — and M5 adds
  the `artefacts` and `models` blocks the sentence above describes, whose names ADR 0027 §3
  reserves. "Checked at load" was true of the engine and the plugins and is not true of a
  toolchain, and §11 now says so.

### 4. M1 carries the remaining bundled instruments

§16 said "one bundled synth". §8 names Surge XT, sfizz, Dexed and Airwindows as what makes a
fresh install produce usable sound, and §11 requires a golden-render test per bundled
instrument. One synth proves the render path; the set is what M1 delivers.

- **Amended 2026-09-05:** all four, minus Airwindows, which moves to **M4**. This decision read
  §8's candidate list as four bundled *instruments*; three of them are. Airwindows is a set of
  effects, so §11's "a golden-render test for every bundled instrument" is met in full by Surge
  XT, sfizz and Dexed — and the spike found its pinned repository may not build a Linux VST3 at
  all. M4 already vendors clap-wrapper (§7.2, and §16's M4 line), which turns its packaging from
  a build investigation inside the render milestone into the CLAP→VST3 path M4 uses anyway.
  M1's deliverable was never "four plugins"; it was a render path with a golden per instrument,
  and it still delivers that (ADR 0010 §5).
- **Amended 2026-09-25 (ADR 0023 §5):** Airwindows is **M5's**, and not "with clap-wrapper".
  Read on 2026-09-25 and built by the spike: `airwindows/airwindows` builds VST2 and AU and no
  Linux VST3, which was the suspicion above, confirmed; the maintained consolidated build,
  `baconpaul/airwin2rack`, is a **JUCE plugin** — Dexed's shape, an `ExternalProject` with its
  own JUCE, no clap-wrapper anywhere — and it builds a Linux VST3 the engine's `--scan` opens
  headlessly. So the premise that made M4 its home (the CLAP→VST3 path) was the wrong premise,
  and its right home is the milestone that builds the engine, with the re-pin to a different
  repository the user approved. It is one plugin of fourteen parameters whose `Replace` selects
  the effect, which is a question for M5's own ADR before `add_effect` names one.

### 5. M2 carries the mixer and the history view

§16 listed timeline and piano roll. §9 lists seven views. The mixer and the patch-log history
are projections of the model and need nothing beyond M0 and M1, so they belong with the other
projections.

- **Amended 2026-09-07:** and the **instrument and effect editors**, which is §9's seventh view
  and which this decision missed. It read §9's list as six views placed and one — the code
  views — deferred to decision 8; the editors were named in the same breath as the code views
  and placed nowhere, so the milestone that gets a plugin editor was never written down. That
  is the failure this ADR exists to prevent, in the ADR itself. They are M2's, on this
  decision's own test: a projection of the model needing nothing beyond M0 and M1. What M2
  builds is the *generic* editor — a form over the build manifest, which already maps a
  `ParamID` to a display name (ADR 0010 §4) — and not the plugin's own VST3 window, which needs
  a window handle inside the engine process and is a different feature. Three rows of
  `plan.md`'s deferred ledger waited on "the first milestone that lets a user choose a patch";
  with the milestone named, all three are restated against what this editor actually does, and
  two of them turn out not to be M2's (ADR 0014 §1, §3).

### 6. The Libretto-grammar ADR precedes M3

§18.2 Stage 1 requires it "before M3" and it was in no milestone. It sets the LLM-facing view
of the composition layer and the orchestrator's structural self-check axes, so it constrains
M3's design rather than following from it.

### 7. The neural runtime is in v1, at M4

§3 lists it as a tier-3 component, §7.3 specifies it, `ModelRef` and `DeviceRef.neural` are
already in `song.proto`, and the wireframes show a neural pad track. It belonged to no
milestone.

M4 is where it goes: real-time models are wrapped as CLAP and projected to VST3 exactly as
Cmajor instruments are (§7.2, §8), so it shares M4's export and hashing path rather than
needing its own.

Consequence: ONNX Runtime (pinned in §17) becomes a v1 dependency, and §15's open question of
whether it links into `engine` or runs as a separate process must be resolved before M4
starts. It remains `[OPEN]` — this ADR places the work, it does not decide the packaging.

**Resolved 2026-09-25, by the user, before M4 starts — as this section asked.** The neural
runtime is a **separate process**. It does not link into `engine`.

Two reasons, and the second is the one that makes it not merely tidy. It keeps CLAUDE.md #6
clean: the engine stays schema-agnostic and gains no machine-learning dependency, so ONNX
Runtime never enters the binary that hosts the audio graph. And **a crash or a version clash in
ONNX Runtime cannot take the audio thread with it** — linked in, an abort inside the runtime,
or a symbol collision between its own vendored protobuf, BLAS or threading library and
Tracktion's, ends the render or the preview. Out of process it ends a process whose death the
supervisor already knows how to report, which is the bargain ADR 0008 §1 took for the engine
itself and ADR 0020 §4 took again for `ai`.

The cost is accepted rather than argued away: a transport to define and a second binary to pin.
This repository already pays it twice — the engine over gRPC on a socket it names, and `ai`
over its own — so the shape is known, the supervision is written, and neither was the expensive
part of M1 or M3.

**What is settled and what is not.** Settled: it does not link into `engine`. Still M4's, and
its planning's rather than this ADR's, is the packaging *detail* — which transport it speaks,
what exactly is pinned beside the ONNX Runtime commit §17 already carries, and whether the
real-time CLAP wrapper §7.3 describes lives with it or with the DSP compiler. §13 owes it a
directory, which is a top-level directory and therefore an ADR of its own (CLAUDE.md, Repo
layout). None of that blocks M4 being planned; all of it is inside M4.

The `[OPEN]` marker is removed from §15 and from `docs/plan.md`'s "Open — not ours to decide",
because it is answered. §3's tier table said the process was `engine` and is corrected with it.

**Amended 2026-09-25, the same day, by ADR 0023 §1 and §3.** "M4 is where it goes" reads
**M5**, under the split. And what the detail paragraph above left to M4's planning is
answered narrower than this section imagined: **in v1 the neural path is Cmajor's own
ONNX-to-Cmajor converter alone**, riding the DSP export path, and **the separate process this
section decided is not built in v1** — nothing ships that would run in it, and no model ships
either. The decision that it *is* a separate process when it exists stands untouched; what the
spike found is that a converter covers what v1 has a consumer for, minus two things M5 must
write itself (a `Gemm` bias that does not broadcast, and an out-of-subset operator that is not
refused with its name). So of the four things this section said M4's planning owed: the
**transport** is nothing in v1 and is not decided against no consumer; **what else is pinned**
is the `onnx` PyPI package the converter imports, returning to a person before M5 installs it,
while ONNX Runtime's §17 row reaches no binary; the **CLAP wrapper** is the DSP pipeline's,
since a converted model *is* a Cmajor patch; and the **directory** is `compilers/neural/`,
under §13's existing `/compilers` line, so it is not a top-level directory and needs no ADR of
its own after all — this amendment and M5's neural ADR are the record.

### 8. Code views are M4

§9 lists them among the views, but they edit generator and DSP source, neither of which exists
before M4. Building them at M2 would mean building an editor for nothing.

- **Amended 2026-09-25 (ADR 0023 §1):** the two views split with their compilers. The
  **generator** view — CodeMirror over `Generator.source`, ADR 0016 §2's pin's first consumer —
  is M4's; the **DSP** view lands with the export pipeline in M5. The reasoning above is
  unchanged: each editor lands with the thing it edits.

### 9. M5 carries MIDI I/O, the REAPER path, and publishing the schema

MIDI import/export is in §10 and was in no milestone. The REAPER RPP path and publishing
`song.proto` as a standalone documented, versioned artifact are both §18.2 Stage 2, which maps
to M4–M5; both are interop, so both are M5. §18.2 calls owning the schema standard a
defensibility play, which makes it deliverable scope rather than marketing.

- **Renumbered 2026-09-25 (ADR 0023 §1):** the milestone this section calls M5 — interop,
  polish and the installer — is **M6**, because M4 was split at its native seam and the DSP
  compilers took the M5 number. Nothing placed here moves; §18.2's "Stage 2 (M4–M5)" reads
  M5–M6.

## Still unplaced

The analysis features of §6 (key, chord and structure detection, tempo estimation,
Demucs-class stem separation) and symbolic generation (melody, harmony, drums, variation) are
substantial scope that §16 places nowhere. They are orchestrator responsibilities, so M3 is
the only candidate, but M3 as specified is the tool-calling loop and the panel.

Whether they are v1 is a product decision, not an agent's. Added to §15's open items.

## Consequences

- §16 is rewritten; §15 gains a row for this ADR and a fourth open item.
- `docs/roadmap.md`'s arc, and its risk item on §16 being thin, are updated in the same change.
- Nothing here changes the schema, so no code changes and no golden-render pass.
- M4 grows the most: compilers, neural runtime and code views. If M4 needs splitting, that is
  a later ADR against a real schedule, not a guess now. **That ADR is 0023, 2026-09-25**: the
  M4 plan laid the schedule out, the spike measured the native half, and the user split it at
  the native seam — M4 the generators, M5 the DSP compilers and the neural path, M6 interop.
