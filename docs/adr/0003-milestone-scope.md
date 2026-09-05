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

### 5. M2 carries the mixer and the history view

§16 listed timeline and piano roll. §9 lists seven views. The mixer and the patch-log history
are projections of the model and need nothing beyond M0 and M1, so they belong with the other
projections.

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

### 8. Code views are M4

§9 lists them among the views, but they edit generator and DSP source, neither of which exists
before M4. Building them at M2 would mean building an editor for nothing.

### 9. M5 carries MIDI I/O, the REAPER path, and publishing the schema

MIDI import/export is in §10 and was in no milestone. The REAPER RPP path and publishing
`song.proto` as a standalone documented, versioned artifact are both §18.2 Stage 2, which maps
to M4–M5; both are interop, so both are M5. §18.2 calls owning the schema standard a
defensibility play, which makes it deliverable scope rather than marketing.

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
  a later ADR against a real schedule, not a guess now.
