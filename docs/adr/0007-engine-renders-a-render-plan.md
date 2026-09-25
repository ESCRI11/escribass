# ADR 0007 — The engine renders a `RenderPlan` compiled by core, never a `Song`

- **Status:** Accepted (2026-09-04)
- **Affects:** `proto/render.proto` (new, M1); `core/src/render.rs` (new, M1); `engine/` (new, M1);
  `docs/specs.md` §8
- **Builds on:** ADR 0001 Consequences (history never reaches the engine), ADR 0002 §6 and
  Consequences (the snapshot is its own message and strips provenance), ADR 0006 §2 (the
  caller-fixable line by signature) and §4 (a mirrored entity shape is a second representation)
- **Recorded in:** `docs/specs.md` §8 and §15.

## Context

§8 says the engine "receives a materialised layer 2 + layer 3 snapshot, builds a Tracktion
edit" and "has no knowledge of the schema beyond the materialised snapshot". CLAUDE.md #6 says
the engine is schema-agnostic. ADR 0002's Consequences place the snapshot as "its own message
under `proto/`" with provenance stripped, and ADR 0001's Consequences forbid any history
metadata from reaching it.

Between them they say what the snapshot is not. They do not say what it is: what
"schema-agnostic" means for a process that must still know what a note is; which side resolves
a clip to its track, or a solo to the tracks it silences; which side turns ticks into samples;
and — the question CLAUDE.md #1 asks of every new message — how a second protobuf describing a
song avoids becoming a second representation of it.

Each has an answer that looks fine and moves the model's semantics into C++. M0's lesson is
that a boundary is right at the byte level or it is not right, so this ADR fixes the boundary
before either side of it exists.

## Decisions

### 1. The engine never receives a `Song`. It receives a `RenderPlan`: flat, ordered, resolved

"Schema-agnostic" cannot mean the engine links nothing from `song.proto` — `render.proto`
imports it (decision 2), and protoc emits every message in the file. It means **nothing that
requires knowing the model's structure crosses the boundary**. The engine is handed no
id-keyed map to walk, no `Clip.track_id` to resolve, no `Mix.solo` to resolve against every
other track, no `loop_length_ticks` to expand, no `DeviceRef` arm it cannot render, no
`Effect.index` to sort by, and no history field. It receives tracks in order, each with a
device chain in order, clips of notes or audio placed on a timeline, automation lanes on named
parameters, a tempo map, a target and a length. It knows what sound is and nothing about how a
song is stored.

The line is §14.7's question run the other way. "Would a different renderer need it?" decides
what belongs in the model; "does answering it need the model?" decides what belongs in core.
Resolving a reference, deriving an order, expanding a loop, dropping the tracks that `solo` and
`mute` silence, and ending the render at the last clip or section (`docs/plan.md`, decisions
of 2026-09-04) all need the model, so core does them, once, and the engine renders exactly
`length_ticks` ticks with no opinion of its own. The alternative — the engine doing any of it —
is a second implementation of the model's semantics in a language the validator does not run
in, and every model change becomes an engine change.

### 2. Leaf messages cross by value with the §4.3 fields blanked; structural messages are plan-local and `repeated`

A message from `song.v1` is reused **by value** in the plan when it holds no entity collection
and no id that names another entity. Under today's schema that is `Note`, `Instrument`,
`Effect`, `Mix`, `AutomationPoint`, `TempoEvent`, `DeviceRef` with its arms, `RenderTarget`,
and `AudioClip` (ADR 0011). Their `id`, `provenance` and `version` cross as empty — that is
what ADR 0002's "strips provenance" means concretely — and a test asserts the serialised plan
contains no non-empty one, which is the check ADR 0001's Consequences asked for.

Reuse rather than copy, because a plan-local `PlanNote { pitch, start_tick, … }` is the
mirrored shape ADR 0006 §4 forbids: a field added to `Note` would never reach it, and nothing
would break loudly enough to notice. Blank rather than strip, because three empty fields on the
wire cost nothing and a second `Note` without them is the mirror again.

Everything structural — the track list, a track's clip list, an effect chain, an automation
lane's points, the tempo map — is a plan-local message using `repeated`, never a map. ADR 0001
§3 makes entity collections maps so RFC 6902 paths survive concurrent inserts; a plan is never
patched, so the hazard does not exist, and `repeated` carries the one thing the engine needs
that the model deliberately does not store: order. Musical order is derived, never stored
(ADR 0001 §3); compile is where it is derived.

An audio clip crosses as `AudioClip` by value beside the **absolute path** of its asset,
resolved by core from `asset_hash`. §8's "never reads project files" is about the `.escri` —
its layout, its hashing, its history. A file the engine is handed a path to is input, and the
engine learns nothing about where it came from. Embedding the bytes instead would put every
sample of every asset through stdin and into the plan golden.

**Extended 2026-09-07, when the sampler met it.** A sampler's SFZ crosses the same way:
`PlanInstrument` carries `Instrument` by value — `SamplerRef.sfz_hash` included — beside
`sfz_path`, the absolute path core resolved from that hash. That is this decision applied
rather than a new one; what makes it worth recording is *why* a path is the only thing that
could have crossed. An SFZ is not self-contained. It names its samples with relative paths,
and `assets/` is content-addressed — one file per asset under its SHA-256, no directory and
no extension (§10, `core/src/project.rs`) — so the bytes of an SFZ would arrive without the
samples they name, and no message this ADR could add would carry them either.

What resolves it is the store's own flat shape. sfizz looks a `sample=` up against the
directory the SFZ was loaded from (`FilePool::checkSample`), so an SFZ stored in `assets/`
reaches its siblings and nothing else — and a sibling there is another asset, addressed by
its own hash. **M1 renders exactly that case: an SFZ whose `sample=` values are the hashes of
the assets beside it.** It needs no staging directory, no rewriting, no name on an asset and
no change to `add_asset`, which takes bytes and returns a hash.

The general case — an SFZ downloaded with its samples under the names it was written against
— is refused rather than half-supported, because supporting it needs an asset that knows its
own name, and that is a `song.proto` question with an ADR of its own. Refused *loudly*: sfizz
drops a region whose sample it cannot find and reports nothing (ADR 0009 §4), so the
alternative is a track of silence that exits zero.

### 3. Time crosses as ticks plus tempo events; the engine converts

`Note.start_tick`, clip positions, fade lengths and automation ticks cross as the integers the
model holds, with the tempo map beside them, and the engine places them on Tracktion's own
tempo sequence. One owner of tick→sample is one fewer rounding site: if core converted to
samples, the engine would have to convert samples back into the beat time Tracktion's timeline
is built on, and two conversions with two rounding rules are two places a bit can move.

The spike did not exercise this — it rendered a tone generator with no tempo map — so PR 7
does, against the plan golden and the render golden together. **The recorded fallback**, if
Tracktion's conversion proves unstable across tempo changes: core computes every sample
position itself, in `i64` with one defined rounding rule, the way `quantize` already fixes its
tie rule (`docs/plan.md`, M0.3), and the plan carries samples beside the ticks. The conversion
then lives in code we own and test, at the cost of the second conversion site. Whichever
holds, the model is unchanged: ticks are the only musical time (§4.2).

### 4. `core::render::compile(song, assets) -> Result<RenderPlan, Vec<Violation>>`, pure

`song` is a `Song` that has passed §4.4 — the session holds no other kind. `assets` is an index
from content hash to path, which is what `Project` knows about `assets/`; the function reads no
file. The plan is therefore a pure function of the document and the index, which is what lets
PR 4 golden it: the M0 claim (same input, same bytes) and the M1 claim (same plan, same PCM)
meet at this message, and a render mismatch splits into "core changed the plan" and "the
engine changed the rendering" by comparing the plan first. That is M0.4's "paths, not two
40 KB blobs" applied to audio.

The error type is `Vec<Violation>` because every way compile fails — an unsupported device, a
missing asset, a routing the engine cannot honour — is something the caller can fix by calling
differently. ADR 0006 §2 draws the caller/operator line by return type, and `prepare` already
draws it this way; `render_export` inherits it without a classifier.

### 5. The plan is derived, and a descriptor-driven test is what proves it is not a second model

The plan is never persisted, never edited, never read back and never diffed; it has the
standing of a WAV, not of `song.json`. ADR 0004 made the same distinction for the derived
cache: what CLAUDE.md #1 forbids is a second *authoritative* state.

Prose saying "keep the plan in sync with the schema" is the promise ADR 0006 §4 records
failing silently, so the guarantee is a test. `core/tests/render_coverage.rs` walks
`song.v1`'s file descriptor and requires every field of every message to be in exactly one of
three places: carried into the plan by value (its message is embedded in `render.proto`);
named on a **consumed** list — fields compile reads to derive plan content rather than copy it
(`Clip.track_id`, `Track.kind`, `Mix.solo`, `Mix.mute`, `Clip.loop_length_ticks`,
`Effect.index`, `Section.end_tick`); or named on an **ignored** list with a reason
(`provenance` and `version` everywhere, `Track.name`, `Marker`, `Generator` — whose compiled
output is already layer 2 — `TimeSignatureMap`, which changes no sample). A field added to
`song.proto` that is on no list fails the test, instead of rendering as nothing.

The test proves coverage, not use: that a consumed field actually changes the plan is the plan
golden's job, and that it changes the sound is the render golden's.

### 6. M1 refuses what it cannot render with `render_unsupported`; the validator still accepts it

compile refuses, naming the field: `DeviceRef.cmajor`, `.faust` and `.neural` (M4);
`Routing.sends`, `Routing.sidechains`, an `output_track_id` naming a bus, and a
`TRACK_KIND_BUS` track (~~the mixer, M2~~ — **see the amendment below**); `RENDER_KIND_STEMS` (M5); and `RenderTarget.dither`
set, because dither is a noise source and §2.2 requires every source of randomness to carry a
seed stored in the project, which a `bool` cannot. Nothing else. A plugin outside the bundled
set never reaches compile: ADR 0010 makes that the validator's `plugin_unknown`.

The validator does not refuse these, because validity and renderability are different
questions. A song referencing a Cmajor source is a song the model can hold, edit, branch,
merge and export — M4's fixtures, M5's DAWproject import and a project written by a newer
build all depend on that — and whether *this* engine can turn it into sound is a fact about
the engine. Putting that fact in the validator couples the model to the renderer's
capabilities, which is CLAUDE.md #6 in reverse. The validator checks reference integrity
(§4.4: the reference resolves to something pinned); compile checks capability; each fails in
one place with one rule.

**Amended 2026-09-05, when compile met it.** One refusal more than the list above, under the
same rule: a looping audio clip with `time_stretch` set whose `length_ticks` is not a whole
number of `loop_length_ticks`. The plan carries each loop iteration as a clip of its own and
the engine stretches the asset to the clip it is handed (ADR 0011 §3), so the short last
iteration would stretch to the wrong length — which `render.proto`'s `ponytail:` on `PlanAudio`
had already recorded the plan cannot yet say otherwise about. "Nothing else" was written
before compile existed, and a plan wrong by construction is not something to hand the engine
quietly. The refusal names `Clip.loop_length_ticks` and lifts the day an additive field names
the stretched unit.

**Extended 2026-09-07, in PR 8b.** One more, and one that deliberately stays outside compile.
The refusal is a `sampler` on an **effect**: `Effect.ref` and `Instrument.ref` are one
`DeviceRef` type, so the model can hold it, and sfizz voices notes while an effect chain is
handed audio. It joins the list under the same rule, naming `Effect.ref.sampler`. A sampler
*instrument* is not refused — it is what PR 8b renders, which is why this decision never
listed the arm.

The one that stays outside is the SFZ itself. An SFZ this engine cannot resolve is
caller-fixable, and by the reasoning above it belongs here — but deciding it means reading the
file, and compile is pure (decision 4). So the engine refuses it instead, naming the `sample=`
it could not find, and it is the one caller-fixable refusal in M1 that arrives as an exit
code. Moving it here means giving compile the file reading its signature was defined not to
do; that is a question for M2, not a reason to make compile impure now.

**Amended 2026-09-24, at M3's close, walking this ADR against the code.** Two sentences above
handed work to M2 and M2 closed without it, which is the failure ADR 0003 exists to prevent and
the one M2's own close caught in ADR 0010's Consequences. Both are corrected here and neither
is a code defect: compile refuses exactly what it should, and what was wrong was the milestone
the refusal named.

**`Routing.sends`, `Routing.sidechains`, a bus `output_track_id` and `TRACK_KIND_BUS` are not
M2's, and are in no milestone.** This section read "(the mixer, M2)" because §16 gives M2 a
mixer — but ADR 0003 §5 placed *the mixer view*, and what M2 built is a form over `Mix`:
`gain_db`, `pan`, `mute` and `solo`, written through `apply_patch`, with no routing surface at
all. Routing is a change to what the engine builds — a bus track, a send with its own gain, a
sidechain input — and it was never M2's to deliver. It is not M3's either: M3 adds no engine
capability and the loop reaches a fader through `ParamRef`, which needs none of it (ADR 0015
§1). So the scope is **unplaced**, which is worse than deferred because nothing walks it. A
deferred-ledger row now carries it with a trigger something can produce, and the three refusal
messages in `core/src/render.rs` stop naming a finished milestone: they say the mixer's routing
is unplaced, which is true and stays true until someone places it.

**The SFZ refusal's "question for M2" was not asked.** An SFZ this engine cannot resolve is
still refused by the engine, naming the `sample=` it could not find, and compile is still pure.
M2 gave no reason to move it and did not; M3 does not touch the engine. The paragraph stands as
a description of what the code does, and the question is retired rather than re-dated: moving
it would mean giving compile file access its signature was defined not to have, and that
argument is not a milestone's to settle but an ADR's, whenever someone wants it settled.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Send the `Song`; the engine resolves | The engine walks id-keyed maps and learns what `solo`, `loop_length_ticks`, `Routing` and `Effect.index` mean — the model's semantics reimplemented in C++, contradicting §8's "no knowledge of the schema beyond the snapshot". Every schema change becomes an engine change. |
| A plan-local copy of every message (`PlanNote`, `PlanMix`, …) | The mirrored shape of ADR 0006 §4; a field added to `song.proto` never reaches it and nothing notices. |
| Maps in the plan, as in the model | ADR 0001 §3's reason — patch-path stability — does not apply to a message that is never patched, and the engine would have to derive order the way core already has. |
| Core converts to sample positions | A second conversion site inside Tracktion's timeline. Kept as decision 3's fallback, not its default. |
| `compile` on `Session`, reading `assets/` itself | Not pure; the plan golden would need a project directory, and a WAV mismatch could no longer be split into plan and render. |
| A `RenderError` type, or `ProjectError` | Puts caller-fixable failures on the operator side of ADR 0006 §2's line. The four existing error types already share `Violation`'s shape. |
| The validator refuses unrenderable songs | Couples §4.4 to the engine's capabilities; a project from a later build would fail to open here. |
| Asset bytes embedded in the plan | Every sample through stdin, protobuf message limits, and a plan golden that embeds audio. A path is input, not a project file. |

## Consequences

- **PR 3** adds `proto/render.proto` (package `escribass.render.v1`, importing `song.proto`).
  §13 already lists `Render` under `/proto`, so no new-directory ADR. `buf breaking` covers it
  from that PR.
- **PR 4** adds `core/src/render.rs`, the coverage test and the plan golden under `tests/`.
  `render_unsupported` joins the stable rule ids.
- **PR 7** settles decision 3 against the render golden; if the fallback is taken, this
  decision is amended in place and the plan gains sample positions beside ticks.
- **ADR 0008** fixes how the plan reaches the engine; **ADR 0011** adds the audio clip fields
  that cross by value here.
- **PR 8b** adds `PlanInstrument.sfz_path` under decision 2 as extended, and the restricted
  case it renders is recorded in `render.proto` beside the field.
- ~~`proto/` still generates Rust only~~ **(true when written; `proto/gen/` holds Rust,
  TypeScript from M2 PR 3 and Python from M3 PR 3 — ADR 0006 §7, carried out)**; the engine's
  C++ is generated at build time (ADR 0008 §4).
- The `docs/plan.md` deferred row for user VST3 plugins is unchanged: a plugin outside the
  bundled set is refused loudly, by ADR 0010, ~~until M2 places them~~ — **M2 did not place
  them and the row was retriggered on 2026-09-07 (ADR 0014 §3) to name an event instead: when
  the build manifest can describe a plugin this build did not bundle.**
