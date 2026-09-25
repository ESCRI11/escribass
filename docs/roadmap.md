# Roadmap

The arc from today to v1, at milestone granularity. Three documents describe planning; this
is the widest view and the least authoritative:

| Document | Holds | When they disagree |
|---|---|---|
| [`specs.md`](specs.md) §16 | The milestone definitions | it wins |
| [`plan.md`](plan.md) | Live status, deferred ledger, known gaps for the current milestone | it wins over this file |
| `roadmap.md` (this file) | What each milestone means for a user, what blocks what, what would change course | this file is stale |

No dates. The repo has none; sequencing and dependency are the only claims made here.

## Where we stand (2026-09-02)

`schema/song.proto` and `history.proto` exist, with generated Rust, TypeScript and Python
types, one cross-language fixture, ADR 0001 (history is a patch DAG) and ADR 0002 (schema v1).
That is M0.1 of four M0 steps. No `core/`, no tool API, no sound, no CI, nothing pushed.
`plan.md` has the step table.

## The arc

Each milestone in user terms, its proof point (§18.2: lead every demo with reproducibility),
and which §18.1 differentiator stops being aspirational.

**M0 — Schema & core.** A song is a directory of readable JSON you can `git diff` (§2.6,
§10). Any MCP client — Claude, Cursor — builds and edits one through typed tools; every call
is `dry_run`-able, every applied call is one RFC 6902 entry in an append-only history (§5,
§18.2). Branch a song, try an idea, merge or discard it (ADR 0001). Nothing makes a sound.
*Proof:* the same tool calls twice give byte-identical `song.json` and `patches/`, with no
normalisation step (ADR 0001 §5). *Real:* differentiator 3, and the open-schema half of 4.

**M1 — Render.** Render the project to a WAV offline and get the same bytes on every run, on
one platform (§8). The bundled instruments of §8 make sound, each with a golden-render test
(§11). *Proof:* the canonical demo, from a
terminal — edit bar 17 through the tool API, re-render, and a sample diff shows changes
confined to bar 17. Render hashes land in golden tests (§11). *Real:* differentiator 1.

**M2 — UI.** Open the project in a desktop app, see the arrangement and the piano roll, press
play and hear it; open the mixer, the instrument and effect editors, and the patch-log history
(§9). Every view is a read of the model; every control is a tool call (§2.1). The editors were
added 2026-09-07 — §9's seventh view sat in no milestone at all until ADR 0014 §1 placed it.
Linux x86-64, as M1: macOS and Windows are unclaimed rather than contradicted (ADR 0014 §2).
*Proof:* the bar-17 demo with the edit visible on the timeline and in the roll. *Real:*
nothing new; differentiator 3 becomes something a person can see.

**M3 — AI loop.** Ask for a change in plain language inside the app. The assistant proposes
a patch, the panel shows the diff, and you apply, reject **or edit** it (§9) — the third
control was added when the question was decided, and a person who changes the bytes owns them
(ADR 0019 §3). Invalid proposals go back to the model as structured errors; "three" turned out
to mean **three refused calls in one turn**, beside a cap of twelve model responses, because a
loop cannot tell a retry from a new call (§6.1; ADR 0022 §3). The model is a config value, via
OpenRouter — recorded, never pinned, since nothing verifies a hosted model. *Proof:* the
bar-17 demo driven by the assistant, diff on screen before apply. *Real:* differentiator 4's
model-agnostic LLM layer. **Delivered 2026-09-24**; what it does not claim is in
`plan.md`'s "M3, closed".

**M4 — Compilers.** Write, or ask for, an instrument in Cmajor; hear it at once under JIT;
export it to a content-hashed VST3 so the render reproduces after the JIT is gone (§7.2).
Write a generator in the seeded Python DSL and compile it to notes (§7.1). Load a neural
instrument, wrapped as CLAP like any other (§7.3). Edit generator and DSP source in the app
(§9). `lock.json` gains the compiled artefacts, completing what M0 began (§10, §11). *Proof:* same seed, same
notes; change the seed and only the generated clip changes; an exported instrument re-renders
bit-identical. *Real:* differentiator 2; differentiator 1 complete, lock file included.

**M5 — Interop & polish.** Take the song to Bitwig, Studio One or Cubase as DAWproject and
bring it back; import and export MIDI; export stems (§10). Take it to REAPER through the RPP
path (§18.2 Stage 2). One installer for macOS, Windows and Linux carrying `app`, `ai` and
`engine` (§3). The schema ships as a standalone documented, versioned artifact (§18.2).
*Proof:* a DAWproject round trip, and per-stem hashes with the untouched stems identical
(wireframes, Plate 6). *Real:* differentiator 4 in full.

**v1 is M5 landed.** Every sentence of §1 is then true of a shipping build, all four §18.1
differentiators are demonstrable rather than argued, and the audience is the one §18.2 Stage 3
names: developers, educators, reproducibility-minded producers and composers.

## Dependency spine

```
M0  core · tool API · MCP
├── M1  engine · offline render
│   ├── M2  preview playback
│   ├── M4  Cmajor JIT and CLAP→VST3 export      (both run in `engine`, §7.2)
│   └── M5  stems
├── M2  timeline · piano roll                     (projections of the model; no engine needed)
│   └── M3  AI panel
├── M3  Python sidecar · tool-calling loop        (needs only the gRPC API, §6)
│   └── M4  Python DSL generator                  (runs inside `ai`, §7.1)
└── M5  DAWproject I/O                            (lives in `core`, §3, §13)
    M5  installer                                 needs all three processes: M1, M2, M3
```

Hard blocks: M0 blocks everything — the model is the only state (§2.1) and the tool API the
only way to change it (§5). M1 blocks the audio half of M2, the DSP half of M4, and stems.
M2 blocks the AI panel, not the AI loop. M3's sidecar blocks the DSL compiler (§7.1).

Looks sequential, is not: M1 and M2's editing views share nothing but M0. Most of M3 exists
at M0 — an external MCP client does dry-run → diff → apply from day one (§18.2); M3 adds the
bundled orchestrator, the retry loop, the panel, and the Libretto-style grammar ADR §18.2
wants before M3. M4's two halves are independent. DAWproject I/O can begin the day M0
closes; the installer cannot begin before M3.

## What v1 is not

| Not in v1 | Source |
|---|---|
| Live performance, low-latency live coding | §1 |
| Recording audio input | §1 |
| Collaboration over a network; remotes for branches | §1; ADR 0001 Deferred |
| Mobile | §1 |
| A prompt-to-audio generator, or a run at the Suno / Mozart / Google Flow market | §1; §18.2 Stage 3 |
| A local LLM in the box — the provider stays a config change | §6; §15 |
| Native CLAP hosting — VST3 via clap-wrapper, always | §8; §15 |
| Strudel as a generator language | §7.1; §15 |
| Faust as an authoring language in the UI — import only | §7.2 |
| Logic, Ableton, Pro Tools or FL Studio interop claims | §18.2 Stage 2 |
| Git as the storage engine — interop only | ADR 0001 §6 |
| Bit-exactness across platforms — per platform, documented per plugin | §8 |
| Commercial relicensing | §12 |

## What would make us rewrite this

External, from §18.4, current as of the 2026-09 landscape:

| If | Then |
|---|---|
| A funded incumbent ships a typed, exportable, editable model | Double down on determinism, instruments-as-code and the open schema; pull MCP exposure forward |
| Deterministic, note-locked audio editing is demonstrated | Drop the "audio can't edit" framing; message on typed model, reproducibility, open |
| A DAW vendor ships an official validated control agent | Emphasise open, model-agnostic, code-defined; pursue DAWproject interop with that vendor |

Watch list, re-checked quarterly (§18.3), last checked 2026-09-02: Mozart AI or Suno shipping
a typed project model; a DAW vendor's validated agent (Ableton's connector is knowledge-only
as of 2026-04); ACE-Step successors with note-locked deterministic editing; Waveform MCP or
Producer Pal adding a persistent typed model with diffs.

Internal, already flagged by the spec: M1's render turning out not bit-exact on one platform
— §8 asserts it, nothing tests it yet; the engine sits behind gRPC so it can be replaced
(§12), but that is a rewrite of M1. Tracktion pinned to `develop`, re-pinned monthly until a
v3.3 tag (§17): a re-pin that moves a golden hash needs an ADR and a full golden pass. The
four `[OPEN]` items in §15 — symbolic model (~~M3~~ **unplaced: M3 asked it as U2, and it was
not reached because U1 was answered "not in M3"**), neural runtime packaging (~~due now~~ — **answered 2026-09-25: a
separate process, ADR 0003 §7, resolved**), minimum OS versions (M5 installer), and whether §6's analysis and
symbolic generation are v1 at all — each block where they sit, and none is an agent's to decide.

## Where the risk sits

Not a register. The specific places this plan is most likely to break, judged from the spec
and from what M0.1 taught.

**1. Determinism fails at boundaries, and we have the evidence.** M0.1 was declared done;
review then found two defects (`b402dc0`), both at a serialisation boundary: a patch log
that could not be byte-stable (a `HashMap` in a precompiled crate), and patches that wrote
`43.0` into an `int32` field Rust rejected, TypeScript accepted and Python truncated. The
same review found `serde_json` off by one ULP on ~30% of doubles, tempo events keyed by array
index, and a merge rule that conflicted on every entity by construction. Each was decided on
paper in an ADR and wrong in execution; ADR 0002 §11 was rewritten before any consumer
existed. Four languages, three processes, and every boundary ahead is the same hazard class:
core → engine snapshot (M1), Tracktion and each bundled plugin (M1), JIT versus exported
binary (M4 — Plate 3's "export pending"), the DSL sandbox (M4). Assume each needs what M0.1
needed: a test that runs it, not an ADR that describes it.

**2. M1 is first contact with code we do not own.** M0 is ours to make deterministic. M1
stakes the pitch on Tracktion, JUCE and a third-party synth producing the same bytes twice,
on a `develop` pin (§17). DawDreamer shows headless JUCE rendering can be reproducible
(landscape, Area 3); it does not show any given plugin is. If the one bundled synth is not,
the canonical demo has no sound.

**3. No CI, and the gate arrives last.** `plan.md` says it: everything `b402dc0` fixed is
protected by habit. The determinism suite is M0.4, after `core` and the tool API, so the two
largest pieces of M0 are built before the byte-for-byte check exists.

**4. Merge is designed blind.** ADR 0001 §4 auto-merges disjoint paths and deferred interactive
resolution to M2; ADR 0015 §3 takes it there as one optional per-path field on `merge_branch`,
which is the smallest thing that finishes a merge. One hole stays open: dense `index` on tracks
and effects lets two valid branches auto-merge into an invalid document (`plan.md`, deferred to
M0.3). Closing it is a schema change — another ADR against the foundation — and its trigger is
now an event rather than a milestone: the first gesture that reorders a chain or inserts a track
mid-list, which M2 is on record as not offering (ADR 0015 §4).

**5. M0 carries everything; M4 now carries the rest.** M0 holds the model, the tool API,
branching, merge and an MCP surface, and its scope has already grown once (`b402dc0`). ADR
0003 then placed eight pieces of scope that sat in no milestone, and M4 took three — the
compilers, the neural runtime and the code views — on top of what it had. Both concentrations
are worth watching: M0 because everything blocks on it, M4 because it is now the widest
milestone, and its ONNX packaging question was `[OPEN]` until 2026-09-25, when it was answered
the way the risk reads best: a separate process, so a runtime that falls over does not take the
audio thread with it (ADR 0003 §7, resolved). Two gaps stay open by decision
rather than oversight: whether §6's analysis features and symbolic generation are v1 at all.
