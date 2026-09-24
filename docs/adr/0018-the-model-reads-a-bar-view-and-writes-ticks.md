# ADR 0018 — The model reads a bar view and six descriptive axes, and writes ticks

- **Status:** Accepted (2026-09-17)
- **Affects:** `ai/` (new, M3 — the view and the axes are its first pure functions); the tools
  the sidecar offers the model; `docs/specs.md` §15 and §18.2
- **Builds on:** ADR 0003 §6 (this ADR precedes M3 and constrains it); ADR 0012 §2 (one decoded
  `Song`, every view a pure selector) and §5 (a projection golden from a fixed `Song`, and its
  key-reversed twin); ADR 0007 §6, amended (each loop iteration crosses as a plan clip of its own, with
  `Note.start_tick` staying clip-relative — §3); ADR 0006 §4 (a caller cannot
  set `id`, `provenance` or `version`); ADR 0017 §3 (no rule of the validator's is
  re-implemented beside it)
- **Recorded in:** `docs/specs.md` §15, and a pointer from §18.2.

## Context

§18.2 asks for "a Libretto-style grammar for the composition layer's LLM-facing view — integer
onset slots on a bar grid (already implied by 960 PPQ ticks), explicit voices, bar-level blocks
— and its structural evaluation axes (rhythm, harmony, melody, texture, form, within-song
variation) as the AI orchestrator's self-check metrics", recorded "in an ADR before M3". ADR
0003 §6 placed that ADR before M3 because it "constrains M3's design rather than following
from it", and `docs/plan.md`'s M3 section ("The Libretto ADR, first") wrote down the six things
it has to decide. This is that ADR, alone: no code, and no other ADR, because the fourteen
questions M3's other ADRs answer are downstream of what the model reads.

Three facts shape every decision below.

**The song is the only representation, and the model never sees it.** CLAUDE.md #1 and #2:
what the model reads is a projection and what it does is a tool call as JSON Patch. A view of
the song is therefore text derived from the `Song` and discarded — held between turns,
edited, or written back, it is the second representation in a fourth language (plan, M3
trap 7). The one precedent is `app/`: one decoded `Song`, every view a pure selector, goldened
against a fixed document and its key-reversed twin (ADR 0012 §2, §5).

**The spike measured what a view can and cannot do** (plan, "What the spike found", run
2026-09-17; one model on one day, evidence and never the claim — trap 4). A throwaway
bar-block view of the render fixture was 2,294 bytes against the canonical `get_song`'s 15,011,
and 1,060 tokens to DeepSeek against 5,199. Sent as the first message with `get_song` still
offered, **the model called `get_song` anyway in 25 runs of 27**, so the first turn fell and
the whole edit rose — 36,297 tokens against 28,562. Invalid calls did not move: 2.8% against
2.9%. And the failures that were not the control were all **valid calls doing the wrong thing**
— a quarter note written as 480 ticks, "volume −6 dB" met with an automation lane, a parameter
copied from an existing lane and reported as the one asked for — which no validator can see
and which a person reading the diff before apply is the only thing that catches (§9).

**Two things are already decided and this ADR does not reopen them.** §6's symbolic
generation and analysis are out of M3 (U1), so there is no key detector; and `set_param` is
withheld from the model (question 7), so no `ParamID` is a thing the model can act on.

The paper itself was read for this ADR, and decision 6 says how far and what was found there
that the landscape had not.

## Decisions

### 1. The view is a bar view of the document, written in the document's own units

The view is text: a header, then one block per bar. It is derived from the `Song` by the
rules below and by nothing else, and it carries every id and every field a tool the model is
offered can take, so that a model reading it needs no other read to act.

**The grid.** Bars follow the time-signature map event by event, as `app/src/time.ts` walks
it: bar 1 starts at tick 0; a bar is `960 × 4 × numerator / denominator` ticks; a signature
event that lands mid-bar ends that bar where it lands, because the event carries a tick and
not a bar. Bars are numbered from 1. A beat is `960 × 4 / denominator` ticks. The view extends
through the last bar containing the document's end — the furthest tick reached by a clip's
end, a section's end, a marker, a tempo or signature event or an automation point — which is
the arrangement view's rule (`app/src/arrangement.ts`) plus the two kinds of thing that view
does not draw. An empty document is zero bars; nothing is padded.

**Onsets are ticks.** Libretto's onset slot is an integer on a per-bar grid whose resolution
the header declares — a 16th-note grid in 4/4 has slots 1 to 16, with the beats at 1, 5, 9
and 13 (paper §3, as read). This model's grid is already fixed at 960 ticks to the quarter,
and a tick *is* an integer slot on it. A coarser slot would lose the document: the render
fixture has a note at tick 800, which is on no 16th or triplet grid. So an onset is written
as a tick and never as a slot, and the header says once what a tick is.

**Two ticks to a note, and the view writes the one the tool takes.** A note's position in the
document is clip-relative (`Note.start_tick`, §4.2), and `set_notes`, `transpose` and
`quantize` take it that way. Its position in a bar is `clip.start_tick + loop offset +
start_tick`. The view lists each note **once, under the bar its absolute onset falls in, with
its clip-relative tick verbatim**, and the voice line above it says where that clip's tick 0
sits in the bar — so the model reads rhythm from a bar and copies the number a tool wants,
and the one subtraction it may need has both operands on the line. The alternative — bar-
relative onsets, and the model converting back — is the arithmetic the spike watched a model
get wrong.

**A loop is shown as what sounds.** A looping clip lists its notes once, under its first
iteration, and names every later iteration by its bar tick, the last marked with its cut
length, exactly as `core::render::compile` unrolls it (ADR 0007 §6, amended). A note cut by the loop's
end is marked with its sounding length beside its written one; a note starting at or past
`loop_length_ticks` is listed and marked as never sounding, because it is in the document and
a model asked to fix a clip should see it. The written values are what the view carries,
because the written values are what `set_notes` re-sends.

**Pitch is a MIDI number.** Libretto writes `E4`; the model writes `pitch` 0–127, and "C4" is
60 in one convention and 72 in another, which is a wrong-but-valid call waiting to happen.

**What it carries, by §4.2 layer:**

| Layer | Carried | Abstracted |
|---|---|---|
| Time base | 960 PPQ stated in the header; every tempo and signature event, at its tick and bar | — |
| `Section`, `Marker` | id, name, span in bar and tick | — |
| `Track` | id, name, kind, order (`index`), instrument's id and `DeviceRef` (plugin id and version, or the SFZ, source or model hash in full), each effect's id and ref in chain order, `mix` in the model's units, `routing` where it is not the default, `allow_overlap` when true | An instrument's or effect's `state` (opaque bytes) and the *values* in its `params` — the count is shown. `set_param` is withheld, and 2,855 `ParamID`s in the prompt are 2,855 invitations to the call the spike saw |
| `Clip` | id, voice, `start_tick`, `length_ticks`, `loop_length_ticks`, content kind; an audio clip's asset hash, `gain_db`, fades and `time_stretch` | — |
| `Note` | id, `pitch`, `start_tick`, `length_ticks`, `velocity`; `microtonal_cents` when non-zero | `expression` — no tool the model is offered writes it and the axes do not read it |
| `Automation` | id, target (a track's name and `gain_db`/`pan`, or a device's id and `ParamID`), every point at its tick with value and curve — listed once, in absolute ticks, not per bar, because a lane is not a voice | — |
| `Generator` | id, kind, target | `source` — code for M4's compiler, and free-form text a model reads back is not the grammar |
| `RenderTarget` | — | Whole. The model is offered no render tool |
| §4.3 | `id`, wherever a tool takes one | `provenance` and `version` — the history view's, and a caller cannot write them (ADR 0006 §4) |

Velocity is carried where Libretto abstracts it (paper §3), because `set_notes` "replaces the
clip's whole note set" and a model keeping the other notes re-sends their velocities; ids are
carried for the same reason, since a re-sent note "carrying an id that already exists in this
clip keeps it" (`proto/song_tools.proto`), and without them every unchanged note is minted a
new id and a new provenance on every edit.

**The render fixture, written down.** `tests/determinism/render/expected/song.json`, derived
by hand from the rules above; PR 6's golden is the implementation's output, and where the two
differ PR 6 says which was wrong, the rule or the hand. Spelling — separators, abbreviations,
line breaks — is PR 6's to settle; what is carried and what is abstracted is not.

```
song 01M1FPMP000000000000000001 · 960 ticks per quarter · 3 bars · ends at tick 9600 (@3 tick 1920)
signature 4/4 from @1 tick 0 · a bar is 3840 ticks, a beat 960
tempo 120 from @1 tick 0 · 90 from @2 tick 0
sections: Outro 01M1FPMP00000000000000001G @3 tick 0 – @3 tick 1920 (7680–9600)
markers: none
generators: none
voices (order · name · id · kind · device · fx · mix)
  0 Master 01M1FPMP000000000000000002 master · gain 0 pan 0
  1 Lead 01M1FPMP000000000000000006 instrument · 01M1FPMP000000000000000007 plugin Surge Synth Team/Surge XT 1.3.4 · fx 01M1FPMP00000000000000000G plugin Surge Synth Team/Surge XT 1.3.4 (1 param set), 01M1FPMP00000000000000000E plugin Surge Synth Team/Surge XT 1.3.4 · gain -4.5 pan 0.25
  2 Pad 01M1FPMP000000000000000009 instrument · 01M1FPMP00000000000000000A plugin Surge Synth Team/Surge XT 1.3.4 · gain 0 pan 0 · MUTED
  3 Loop 01M1FPMP00000000000000000C audio · gain 0 pan 0
  4 Keys 01M1FPMP00000000000000001J instrument · 01M1FPMP00000000000000001K sampler sfz 3835bcb2ea8ef051dae9c95baadd1fbb3e03b85424f6324bdfea5b84d5e27597 (keys are whatever the SFZ maps; may be unpitched) · gain 0 pan 0
clips (id · voice · start tick · length · loop · content)
  01M1FPMP00000000000000000Q Lead 0 3000 loop 960 · 3 notes
  01M1FPMP00000000000000000K Lead 3840 3840 · 2 notes
  01M1FPMP00000000000000000X Loop 0 1920 loop 960 · audio 2286d76f7fa133642170637e5c7635d116dbd96cb8b9892281b945b350870636 gain -3 fade 100/200 stretch
  01M1FPMP00000000000000001N Keys 960 960 · 1 note
notes are pitch@clip-tick>length vVelocity #id; a note's clip tick = its bar tick − where the clip's tick 0 sits

@1 tick 0 · 120 bpm · 4/4
  Lead clip 01M1FPMP00000000000000000Q tick 0 at bar tick 0 · loops every 960: plays at 0, 960, 1920, 2880 (cut to 120)
    60@0>240 v100 #01M1FPMP00000000000000000R
    64@800>400 v90 #01M1FPMP00000000000000000S (cut to 160 by the loop)
    67@1200>240 v80 #01M1FPMP00000000000000000T (at or past loop length 960: never sounds)
  Loop clip 01M1FPMP00000000000000000X tick 0 at bar tick 0 · loops every 960: plays at 0, 960
  Keys clip 01M1FPMP00000000000000001N tick 0 at bar tick 960
    48@0>960 v70 #01M1FPMP00000000000000001P
@2 tick 3840 · 90 bpm
  Lead clip 01M1FPMP00000000000000000K tick 0 at bar tick 0
    60@0>480 v100 #01M1FPMP00000000000000000M
    67@960>480 v90 #01M1FPMP00000000000000000N
@3 tick 7680 · section Outro from tick 0 to 1920
  (no voice plays)

automation (id · target · points as tick:value curve)
  01M1FPMP000000000000000010 device 01M1FPMP000000000000000007 (Lead instrument) param 1945359057 · 0:0.2 linear, 1920:0.9 hold
  01M1FPMP000000000000000014 device 01M1FPMP00000000000000000G (Lead fx) param 1243907205 · 480:0.7 linear
  01M1FPMP000000000000000017 device 01M1FPMP000000000000000007 (Lead instrument) param 1945359057 · 960:0.5 linear
  01M1FPMP00000000000000001A device 01M1FPMP00000000000000000A (Pad instrument) param 1945359057 · 0:1 linear
  01M1FPMP00000000000000001R track Lead gain_db · 0:-24 linear, 7680:0 hold
  01M1FPMP00000000000000001W track Lead pan · 0:-1 linear, 3840:1 linear
  01M1FPMP000000000000000020 track Master gain_db · 0:0 linear, 9600:-6 linear
```

**Corrected 2026-09-24, in PR 6, which implemented it.** Three lines of the block above were
the hand's and are now the code's, edited in place; the old wording survives only in git, and
everything else in the block the implementation reproduces byte for byte. The signature line
read `signature 4/4 from @1` and now carries its tick: the table above carries "every tempo and
signature event, **at its tick and bar**", the tempo line four characters away already wrote
one, and a second signature landing mid-bar — the case decision 5 constructs precisely because
no fixture has it — would have had nowhere to say where it landed. `generators: none` was left
out altogether, while `markers: none` was written: a view that prints one and is silent about
the other leaves a reader unable to tell "this song has no generator" from "this view does not
show generators", and the table carries `Generator` as a layer like any other. And the Pad
lane's `0:1.0` is `0:1`: the hand wrote two spellings of the double 1.0 four lines apart — the
pan lane's `3840:1` is the same number — and one number formatter cannot write both, so the
trailing `.0` is stripped everywhere. What is *not* PR 6's to settle, and did not move: what is
carried and what is abstracted.

Ordering is the model's, never the map's: voices by `index`, clips by voice then
`start_tick` then id, notes by `start_tick` then id, lanes by id, points by tick — which is
what the key-reversed golden of decision 5 exists to catch (ADR 0012 §5, amended).

**Drums are inside the grammar, and the grammar adds nothing for them.** Libretto
"deliberately abstracts away velocity, micro-timing, original timbre, and unpitched
percussion" (paper §3, as read; the package's later grammar carries both — decision 6 — and
this rule rests on the document, not on the paper). This product's bundled sampler plays drum kits, and the
wireframes' first track is `Drums · sfizz · kit_808.sfz`. In this model a drum hit is a
`Note` on an instrument track whose device is a `SamplerRef`: its `pitch` is the key that
triggers a sample, and the document stores nothing else about it. The view cannot tell a kit
from a piano, because a `SamplerRef` is a hash, the view is a function of the `Song` alone,
and the asset bytes never reach `ai` (CLAUDE.md #6). So the grammar makes **no
pitched/unpitched distinction, because the document makes none**: a sampler voice's notes
are written as every other voice's, the voice line says `sampler` and that its keys are
whatever the SFZ maps, and the track's name — the person's own word for it — is beside that.
What is refused is a General MIDI drum map or a rule on the name "Drums": either would name a
hit "kick" on the view's own authority, which is the exact shape of the spike's wrong-but-valid
call — a fact asserted that the document does not carry. The axes that read pitch report per
voice (decision 4) so that a reader can discount one. If a later milestone wants hit names,
they live in the SFZ's own `<region>` keys, and reading an asset is a `core` read tool and an
ADR.

### 2. Read only. The model writes through the typed tools, in ticks

A model that *wrote* bar-blocks — text that `ai` parsed into `set_notes` — would be a
compiler in §2.3's sense ("free-form code only enters through compilers that validate before
the result touches the model") and a second way of writing notes beside the typed tool. The
spike removes the reason to want one: five routes drove the typed tools at 44/45 or better on
the achievable instructions with the raw JSON in front of them, and the failures were not
syntax but *meaning* — which a grammar does not fix and a parser adds a class to, since a
parse error is a refusal `core` never issued and the loop then retries on a second validator's
word (plan, M3 trap 12). `add_clip` already takes a `NoteClip` inline and `set_notes` the
whole set, so nothing a write grammar would express is unexpressible today.

So: the view is read-only, the model writes `Note`s in ticks through the tools it is offered,
and **no compiler exists in M3**. The header's sentence about ticks and the note line's shape
are the whole of what the model is told about writing; PR 6 is read-only and needs no compiler
golden.

**The full document does not stay reachable from the sidecar's loop.** This is the decision
the spike forced: a read-only summary beside a tool that returns the full document "was not
trusted as the document", and the view then cost more than it saved. Withholding is the only
lever the spike showed working — instructions did not — and it is the lever question 7
already pulled for `set_param`. So the sidecar **does not offer the model `get_song`**; the
view is the song summary §6.1 names, sent as the first message and re-read after every
applied call, which is ADR 0012 §2's re-read one process over. This is a decision about what
is *offered* to one author and never about what is *accepted* (plan, M3 trap 18): `get_song`
is unchanged for every carrier and every MCP client. Its corollary is the completeness rule
above — every argument an offered tool takes is in the view or is the model's to choose — and
if a measurement ever finds the model needing something the view lacks, **the escape is to
add it to the projection, never to hand back the JSON**, for the reason ADR 0012 §2 gave the
webview: a narrower read keeps one document and one implementation. Question 13's tool list
takes this as given.

### 3. It is computed in `ai`, in Python, over the generated `Song`

The view and the axes are functions in `ai/` over the generated Pydantic `Song`
(`schema/gen/python`), not a read tool in `core`.

The reasons, in order of weight. The view is prompt material — what one carrier sends one
author — and `core` is the tier that knows nothing of authors' prompts; putting it there puts
the model's reading of the song in the process that validates the song. A text view on the
wire would be a string field, so `buf breaking` — the one guard on `proto/` — could not see a
change to it, and "reviewed under `buf breaking`" would be a comfort nothing enforces. And
`ai` already holds a decoded `Song` for the same reason the webview does (ADR 0012 §2); the
precedent for a projection in a second language, goldened, is `app/tests/projection.test.ts`
since M2 PR 4, and this is the same shape in a third — under `unittest`, which is stdlib, as
`node:test` was chosen over a framework for the frontend (ADR 0016 §3; U4 approved nothing
else).

The cost is stated rather than argued around: an MCP client of §18.2 Stage 1 does not get the
view. It gets `get_song`, and what its model reads is the client's choice — the spike found
Claude Code hands its model the MCP `structuredContent`, 9,526 bytes, not the 15,011-byte
canonical text. If that ever matters, the view **moves** to `core` as a read tool, whole, and
`ai` reads it through the host; it is never copied, because two implementations of one
projection is the drift ADR 0006 §1 refused for dispatch, in two languages that already
disagree on field names (plan, M3 trap 11 — the view reads the generated model's attributes
and emits ticks; nothing from `to_json` reaches a tool).

### 4. The six axes are descriptive counts over the document — no corpus, no key, no gate

Libretto's axes are per-song values converted to percentiles "against a frozen 314-song
corpus" curated from the Lakh MIDI Dataset (paper §3; Appendix B's percentile divides by 314).
Its grammar carries a key in the header and a chord label in every bar, but **no axis in
Appendix B reads either**: chromaticism maximises over the twelve roots, the chord metrics
take their chord from the prominent pitch classes of each half-bar, root motion from the
lowest bass pitch of each bar — every one from the note tokens. (This ADR's first draft said
the chord metrics read the label; Appendix B, read in full, says otherwise — decision 6.)
This repository has no corpus and will not have one in M3 (decision 6 says what exists and
why it is not taken); it has no key (U1); and its model has no chord label. An
uncalibrated percentile is a number with no meaning, so the axes here are not percentiles.
They are **exact counts and ratios a reader can check against the view by hand**, taking the
paper's own definitions wherever a definition needs neither a corpus, a key, a label nor a
transcendental function, and deferring the rest by name.

Common rules. Every axis is computed over the unrolled document — what `compile` would play
(decision 1's loop rule) — with a note's sounding span clipped to the bar being measured. A
pitch class is `pitch mod 12`; `microtonal_cents` is ignored. Every value is an integer or an
exact rational (`fractions.Fraction`, stdlib), so the golden is byte-stable: an entropy or a
standard deviation reaches `libm`, whose last bit is the platform's, and a golden that
compares bytes cannot carry one (the reason ADR 0009 §3 pins an ISA, one layer up). Where the
paper's feature is defined per song, ours is per bar as well, because "bar 3 has no onset on
any voice" is what a model can act on and a song-level mean is not. An *onset* is the paper's
𝒪: a distinct (voice, tick) pair, so two notes one voice strikes at one tick are two notes and
one onset. Where the paper's definition divides by nothing — a bar with no onset, a line with
no interval, a maximum over no chord — the value is *none*, except where the paper sets one:
an ascending ratio with no interval is 1/2.

| Axis | Computed in M3, from the `Song` alone | Deferred, and why |
|---|---|---|
| **Rhythm** | Per bar and per voice: onsets; onsets on a beat; the paper's *syncopation rate* (onsets not on a beat boundary, over onsets); onsets on the 16th grid (multiples of 240 ticks), on the triplet-eighth grid (320), on neither; the smallest inter-onset interval in ticks; mean written length as a fraction. Song totals of the same | *Onset position entropy*, *duration CV*, *density variability* — entropy and standard deviation |
| **Harmony** | Per bar, over all note voices and again per voice: the paper's *duration-weighted pitch-class mass* — twelve integers, ticks sounding per class; *distinct pitch classes*; the *prominent* classes, at or above 30% of the heaviest, as `10·w ≥ 3·max`; *chromaticism*, `1 − max over roots of (mass on the major scale at that root / total mass)`, which needs no key because it maximises over the twelve. A sampler voice is reported and marked, never excluded | Any key: U1. *Chord change rate*, *chord vocabulary density*, *diminished/augmented colour* — the paper computes them from the prominent classes of each half-bar or bar, so they need no label and nothing here prevents them; deferred all the same, as a scope choice for PR 6 and not, as this ADR first said, because they read the chord label. *Root-motion entropy* (a logarithm) and *fourth-motion rate* — both read the paper's bass, "the lowest-μ voice", an identification this ADR does not adopt. *Pitch-class entropy* — a logarithm |
| **Melody** | Per voice, per bar and over the song: pitch range; distinct pitches; the interval sequence between successive onsets in signed semitones, where the line at a simultaneous onset is the **highest** note (a stated rule, not a detector); the paper's *step ratio* (intervals with magnitude ≤ 2, over intervals) and *ascending ratio*, both over the non-zero intervals only, as the paper's set *M* drops a repeated pitch | *Interval entropy* — a logarithm. The paper's *identified melody voice*: no voice is identified, every voice is reported, and the reader chooses |
| **Texture** | Per bar: the paper's *voice count* (voices with a note or an audio iteration sounding — an audio clip is a voice with one opaque event); the paper's *mean simultaneity* (notes over onsets); the paper's *maximum chord width* (the largest pitch span among the notes one voice strikes at one tick, over every voice and tick where it strikes two or more); each voice's greatest polyphony; each voice's sounding ticks over the bar's | — |
| **Form** | Over the song: a bar is the paper's set *A_b* of (voice, onset-in-bar, pitch) triples, and two bars are *equal* when their sets are — length and velocity are not compared; the bar sequence lettered by first occurrence (`A A B A`); the paper's *distinct-bar fraction*; the paper's *self-similarity*, the Jaccard overlap of each pair's sets, two empty bars overlapping fully, averaged over pairs as an exact fraction; *novelty rate*, `mean over consecutive pairs of (1 − overlap)`. The document's own sections are listed against the lettering | *Sections per 100 bars* — a checkerboard novelty kernel with a threshold at `mean + 0.5·std`; the document already carries sections |
| **Within-song variation** | Per voice, per bar, one of six classes against the previous bar: *identical*; *same rhythm* (equal onsets and lengths, different pitches); *same pitches* (equal pitch multiset, different rhythm); *different*; *enters* (silent before, playing now); *leaves*. And per voice the number of distinct bar patterns over the song | The paper's definition entire — each windowed standard deviation divided by "the corpus standard deviation of axis *a*" (Appendix B) — is a corpus measure twice over |

For the render fixture, from the view above and by hand (PR 6 checks the hand): bar 1 has
seven Lead onsets of which four are on a beat, so a syncopation rate of 3/7, three on no
grid, a smallest interval of 160 ticks; its pitch-class mass is C 1800 (Lead 840, Keys 960)
and E 480, two distinct classes, chromaticism 0; Lead's line in bar 1 is 60 64 60 64 60 64 60,
six intervals all leaps, three ascending; three voices sound; eight notes over eight onsets
(Lead's seven and Keys' one — Keys' tick 960 is Lead's too, but an onset is per voice), so a
mean simultaneity of 8/8 = 1; no chord width, because no voice strikes two notes at one tick
anywhere in the fixture. Bar 2 has two Lead onsets, both on a beat, C 480 and G 480, one
interval of +7, two notes over two onsets so simultaneity 1, no chord width; bar 3 has no
onset, so neither has a value. The bar sets are eight triples, two — `(Lead, 0, 60)` in both
— and none, so the form is `A B C`, distinct-bar fraction 3/3, self-similarity
`(1/9 + 0 + 0)/3 = 1/27` with `1/9 = 1/(8 + 2 − 1)`, novelty
`((1 − 1/9) + (1 − 0))/2 = 17/18`. Lead is *different* in bar 2 and *leaves* in bar 3; Keys
*leaves* in bar 2.

**Checked 2026-09-24 in PR 6, and the hand held.** Every figure in the paragraph above — the
seven Lead onsets and their 3/7, the three on no grid and the smallest interval of 160, the
mass of C 1800 and E 480 and the chromaticism of 0, the six intervals and three ascending, the
three voices, eight notes over eight onsets and a mean simultaneity of 1, the absent chord
width, bar 2 entire, bar 3's two *none*s, `A B C`, 3/3, 1/27, 17/18 and the four variation
classes — the implementation computes independently and reproduces exactly. Nothing in decision
4 is corrected. Two spellings differ and no value does: a count-ratio is written with its
counts beside the reduced fraction, so `3/3` reads `3 distinct of 3 = 1` and `8/8 = 1` reads
`8 notes over 8 onsets · mean simultaneity 1`.

**What they are for, and what they may never do.** The axes are text the model reads beside
the view — a description of the document as it stands, and of a proposal's document if
question 9's answer makes one readable — so that a model can check what it meant against
what it did before a person sees the diff. That is all. **Nothing about an axis decides
whether a proposal is applied**: a person does (§9), and a metric that refused a patch would
be a validator rule nobody wrote an ADR for, applied to one author, which is trap 18 exactly.
So no axis value is compared to a threshold anywhere in `ai`, none appears in a `Violation`,
none crosses to `core`, and none is used to retry. The spike's wrong-but-valid calls are the
reason this is stated in an ADR rather than assumed: a model that reports success on the wrong
parameter would report success against a metric too.

What the model is told, in one sentence the loop sends with them: these are counts over the
document, not judgements; no key, genre or norm is claimed; a sampler voice's pitches may not
be pitches; a person decides.

**The conditional is settled, 2026-09-24 in PR 8 — the sentence PR 6 deliberately left.**
Question 9's answer *does* make a proposal's document readable (ADR 0019 §1): the model's calls
run against a fork, so from the first call on there is a proposal's document, and it is the
only one the model has. So the loop sends the view and the six axes with the prompt, describing
the document the turn found, and sends both again after **every applied call**, computed over
the document the proposal now has — which is ADR 0018 §2's "re-read after every applied call"
with "applied" meaning applied to the proposal. A refused call changes no document and re-reads
nothing. The model therefore checks what it meant against what it did *while it is still doing
it*, which is what decision 4 asked for and what a description of the saved document could not
have given it. The cost is said rather than hidden: the two together are about 7 KB for the
render fixture and are sent per call, and what bounds that is the response cap (ADR 0022 §3),
not a rule here — `escribass_ai.turn.document` carries the `ponytail:` note and the upgrade
path, which is the view alone after a call. Nothing else in decision 4 changes: no axis is
compared with a threshold, none appears in a `Violation`, none crosses to `core`, and none
decides whether a proposal is applied.

### 5. The view and the six axes are pure, and goldened as the frontend's projections are

Seven functions of a `Song` — the view and one per axis — with no randomness, no clock, no
I/O, no environment and no network. CLAUDE.md #3 names `core`, compilers and `engine`; the
M3 plan draws the line for `ai` at the model ("the `ai` process's own code is a pure function
of what the model said"), and these are the first functions on the pure side of it.

The golden is ADR 0012 §5's, one language over: from
`tests/determinism/render/expected/song.json` — chosen there for having something for a view
to get wrong, and the document `compile` and the frontend are already goldened against — each
function's output is compared byte for byte with a committed file, under `unittest`, and the
same document with every map's keys reversed must give the same bytes, because a ULID-keyed
map iterates in creation order and a projection that lost its `sort` would otherwise golden
green (ADR 0012 §5, amended — measured there, not reasoned). Two things that fixture cannot
exercise are asserted against constructed values, as the frontend's bar grid is: a second
time-signature event, which no tool mints and the piecewise grid gets silently wrong, and a
clip spanning a bar line, which the fixture's four clips do not do; and a third, a voice
striking two notes at one tick, because the fixture never does and the paper's maximum chord
width and a simultaneity above 1 would otherwise be goldened only as *none* and 1.

This ADR decides it; **PR 6 implements it**, and PR 6 is read-only — nothing in it calls a
mutating tool (CLAUDE.md #2 is about mutations, and a constructed `Song` handed to a pure
function is not a document under test).

### 6. What was read, and what was found that the landscape had not

Libretto is arXiv 2606.22708 — Yichen Xu (University of California, Berkeley), *Libretto:
Giving LLM Agents a Sense of Musical Structure*, v1 submitted 21 June 2026, cs.SD.
`docs/landscape-2026-09.md` Area 4 (research of 2026-09-02) recorded its shape and "no public
code found", and §18.2's sentence was written from that record. For this ADR the paper was
first read on 2026-09-17 through a fetch that summarises, and re-read the same day in full
from the HTML rendering at `arxiv.org/html/2606.22708` (§1–§5, Appendix A, Appendix B),
beside the code repository's `README.md`, `DATA_PROVENANCE.md`, `FROZEN.md`, `CHANGELOG.md`,
`LICENSE`, `pyproject.toml`, `libretto/__init__.py`, `libretto/core/axes_v3.py` and the
header of `libretto/data/corpus_distribution.json`, at its head commit of 2026-07-15.
Quotations are verbatim from those files; every other statement about the paper or the
package is this repository's reading of it — **read, never measured**, since nothing of
Libretto's has been run here.

**What the grammar was taken from.** The paper's §3 (Methods): "a global header, a voice
declaration, and one block per bar. The header specifies key, meter, tempo, grid, and bar
count"; each bar "a required chord label followed by voice-specific note tokens";
"simultaneous pitches are joined with a plus sign"; "In a 16th-note 4/4 grid, for example, the
beat positions are slots 1, 5, 9, and 13"; and the representation "deliberately abstracts away
velocity, micro-timing, original timbre, and unpitched percussion". The spelled-out forms —
`VOICES: BASS, GTR, KEYS, HORNS, LEAD`, a block headed `@1 [Em]`, a note as `E4@1>1` — are
Appendix A's three grammar panels, not §3's.

**What the axes were taken from, and how many the paper has.** The paper's fingerprint is
**29 axes** — "35 candidate measurements reduced to 29 axes over rhythm, harmony, melody,
texture, form, and within-song variation" (Table 1) — defined one by one in Appendix B (Metric
Definitions): seven of rhythm, eight of harmony, five of melody, four of texture, four of form,
and one within-song variation. Those six are the paper's *families*, in its own six words in
the abstract and Table 1, and they are the six that §18.2 names and that decision 4's table is
organised by. So the "six axes" of this ADR's title are the paper's six families and not six
of its 29; inside each family decision 4 names which of the 29 it takes and which it defers.
Every axis in Appendix B is computed over the whole piece — **per song, not per bar** — which
is part of why decision 4 computes per bar: §18.2's "self-check metrics" read them as
something a loop could act on, and a per-song percentile is not that.

**Calibration, as the paper defines it.** "Each raw axis value is then converted to a
percentile against a frozen 314-song corpus" (§3); Appendix B's percentile is the number of
corpus values at or below the piece's, over 314, rounded, and an axis at or below the 5th or
at or above the 95th is a "degenerate extreme". The 314 are "314 real MIDI files spanning
eight genres ... curated from the Lakh MIDI Dataset" (§3), of which 255 carry a genre label
(Table 1's classification row; the package's `DATA_PROVENANCE.md`: "255 genre-labeled + 59
original = 314 songs (8 genres)"). The gates that consume the percentiles — a budget of
extreme axes, a genre-fit floor, a copy-risk threshold — belong to the paper's agent loop, and
nothing of them is taken here (decision 4).

**What was not taken, and why:** the chord label, because the model has no such entity and a
label the model wrote into a view would be state outside the song; the key, because it comes
from the paper's header and this document has none (U1); the percentiles, because they are
the corpus; and the grammar's own spelling, because decisions 1 and 2 put the tool's units
first. One reason this ADR's first draft gave was wrong and decision 4 now says so: it said
the paper's chord metrics read the chord label. Appendix B derives them from the note tokens
— the prominent pitch classes of each half-bar, and the lowest bass pitch of each bar — and
no axis reads the header's key either; key adherence is a gate of the paper's education task,
not an axis.

**The code exists.** The paper's "Code and Website" section names
`https://github.com/Xyc-arch/Libretto`, and on 2026-09-17 it resolved: public, its `LICENSE`
beginning "MIT License / Copyright (c) 2026 Yichen Xu", head commit 2026-07-15. It is a Python
package, `libretto` 3.0.0 (`pyproject.toml`: `numpy`, `scipy`, `scikit-learn`, `pretty_midi`,
`music21`, with `anthropic` an optional extra), holding a MIDI-to-grammar encoder and decoder,
the metric code, and, in the README's words, "Frozen data ships inside the package
(`libretto/data/`)". The landscape's "no public code found" was therefore wrong on 2026-09-02
or has since become so — the history as read does not say when the repository went public —
and that one claim is corrected in `docs/landscape-2026-09.md` in this pull request, dated;
the rest of that document waits for its quarterly re-check (§18.3).

**314 and 1,523 are two corpora, one release apart.** `CHANGELOG.md` v1.0.0 (2026-06-14) and
v2.0.0 (the same day; a rename of the package from `musicfp`) ship the "29-axis / 314-song /
2026-06-13 distribution" — the paper's, dated eight days before its submission. v3.0.0
(2026-07-04, tagged a "CORE change") says "the old 314-song corpus (single-author hand labels,
60 unlabeled) is superseded by 1612 MusicBrainz-genre-grounded songs across 11 genres" and
sets `DISTRIBUTION_VERSION = "39-axis / 1523-song / genre-balanced / 2026-07-06"`, which is
the README's number. So **314 is the paper's frozen corpus, and the package's through v2;
1,523 is the version string of v3's percentile distribution, which the paper never saw.** The
package's own files do not agree on what v3 counts: `FROZEN.md` lists "the 1612-song
genre-grounded corpus" of grammar files and "1525 precomputed 39-dim fingerprints";
`corpus_distribution.json`'s header says `n_songs: 1525` and calls its axis system "33
discovered axes" while carrying 39 (the v3.0.0 commit message says the docstrings "were stale
at 33/28-axis, 1497-song"); and `DATA_PROVENANCE.md` still describes the 314-song, 29-axis
`corpus_distribution_314.json` that v3.0.0 says it renamed and replaced. Which songs 1,523,
1,525 and 1,612 differ by is stated nowhere read, and this ADR does not guess.

**The package's axes are not the paper's.** The README's "39 discovered axes" are v3.0.0's:
`FROZEN.md` says they were "DISCOVERED from scratch by the `axis_evolve` self-loop ...
replacing the hand-authored 28 metric_discovery axes + `within_song_variation` (preserved in
git history)" — the paper's 29 replaced, not extended. `libretto/core/axes_v3.py` defines the
39 by name: `axis_chromaticism`, `axis_duration_cv`, `axis_onset_density` and `axis_pc_entropy`
keep a paper axis's name, while `axis_drum_ratio`, `axis_velocity_jitter`, `axis_swing_ratio`
and `axis_instrument_diversity` measure what the paper's grammar abstracts away. Neither that
file nor the distribution's per-axis `category` field (the first word of each name) groups
them into the paper's six families, so whether §18.2's six partition the 39 **cannot be
established from the package as read, and is not claimed**. The grammar moved with them:
`FROZEN.md` says the v3 corpus "carries `[prog=N]` GM instruments, `[drums]` percussion
voices, `^V` coarse velocity", so §3's abstraction of velocity and unpitched percussion
describes the paper's grammar and the package's through v2, not its current one. Decision 1's
drum rule rests on this document's shape and stands either way.

**The package is not a dependency of M3.** U4 approved a gRPC stack and the `openai` SDK and
"nothing else"; adopting it would make the axes what the paper's are — calibrated verdicts —
which decision 4 refuses on its own grounds; and its axes are v3's 39, not the 29 §18.2 was
written from. Its corpus is, in `DATA_PROVENANCE.md`'s words, "community MIDI transcriptions of
copyrighted compositions, re-encoded as text grammar and provided here for research
reproducibility", from the Lakh MIDI Dataset's `clean_midi` subset, whose lineage that file
gives as "CC-BY 4.0" — stated here as what the file says and no further; what it permits is a
person's to assess under CLAUDE.md #4, and the item sits in `docs/plan.md`'s deferred ledger
rather than being assumed away.

## Alternatives considered

**The view**

| Alternative | Rejected because |
|---|---|
| Libretto's grammar verbatim — 16th-note slots, note names, a chord label per bar | A 16th grid cannot write the fixture's note at tick 800; `E4` is a convention the model must map to a number a tool takes, and gets wrong by an octave; a chord label is an entity the document does not have and a model would then be *writing* state into a view. The paper's grid resolution is adaptive, but any grid coarser than a tick is lossy |
| The canonical `get_song`, or its compact key-sorted form | 5,199 and 3,791 tokens against 1,060, and neither is bar-structured — a model reading ticks has to find the bar itself. Measured, and it is the baseline the view is against |
| Bar-relative onsets in the blocks, and the model converts to clip ticks | The subtraction moves from the view to the model, and a quarter written as 480 is the spike's own example of what a model does with arithmetic it was not handed |
| Notes listed per clip, with the bar blocks referring to them | Every note twice — the two-representations problem inside one view — and twice the tokens |
| Ids omitted or abbreviated | `set_notes` keeps a re-sent note's id and mints one otherwise, and `transpose` and `quantize` take `note_ids`; an abbreviation is a thing the model expands, and Gemini dropped one `0` from a 26-character id in the spike |
| A General MIDI drum map, or "a track named Drums is unpitched" | The view would assert a fact the document does not carry — the spike's wrong-but-valid shape, in the view instead of a call. The SFZ knows; the `Song` does not |
| `params` values shown | The model cannot act on them (`set_param` withheld, question 7) and 2,855 ids in a prompt are an invitation to the call that set the wrong one and reported success |

**Direction**

| Alternative | Rejected because |
|---|---|
| A write grammar with a compiler into `set_notes` | A compiler in §2.3's sense and a second way of writing notes; a parser is a second validator in Python whose refusals `core` never issued (trap 12); and the spike found no syntax problem for it to solve |
| Keep `get_song` offered and instruct the model to prefer the view | 25 runs of 27 fetched the document anyway, with the instruction in front of it. Withholding is what worked for `set_param` |
| A narrower read tool in `core` (a subtree of the song) instead of a view | Keeps the JSON shape, which is the cost, and still has no bars in it. It is ADR 0012 §2's escape for a *slow* re-read, not for a read a model will not trust |

**Where**

| Alternative | Rejected because |
|---|---|
| A `get_view` read tool in `core`, served by every carrier | Prompt material in the tier that validates; a string field `buf breaking` cannot see inside; and the sidecar under question 1's default holds no `SongTools` client to call it with. Additive later, and it *moves* rather than being copied |
| Both — Python for the sidecar and Rust for MCP clients | Two implementations of one projection in two languages that already disagree on field names (trap 11); the drift ADR 0006 §1 refused |

**Axes**

| Alternative | Rejected because |
|---|---|
| Adopt the Libretto package (MIT) and its frozen corpus | Not approved under U4; its provenance is a person's to assess (decision 6); its axes are v3's 39, not the paper's 29; and it would make the axes calibrated verdicts, which decision 4 refuses independently of the dependency |
| A key detector, so the harmony axis can name a key | U1: §6.3's analysis is out of M3, and a detector here would be a `[OPEN]` item decided by an agent |
| Entropies and standard deviations, as the paper defines them | `libm` in a byte-compared golden. Exact rationals carry the same information a reader can check, and the deferred ones are named rather than approximated |
| Song-level values only, as the paper's are | A model acts on a bar; "the song's syncopation rate is 3/9" says nothing about where |
| An axis as a validator rule, or a threshold that refuses a proposal | §9: a person applies. A rule that refused one author's patch is trap 18; a rule that refused every author's is a §4.4 change with no ADR |
| No axes in M3, the view alone | §18.2 asks for them and ADR 0003 §6 places the ADR that decides them here; deferring them whole would be a promise kept by nobody (trap 5). What is deferred is named per axis |

## Consequences

- **PR 6** (`m3.6-view-and-axes`) builds seven pure functions in `ai/` over the generated
  `Song`, and their goldens: the render fixture, its key-reversed twin, and constructed cases
  for a second time signature and a bar-crossing clip, under `unittest`. Its golden text is the
  implementation's; where it differs from decision 1's hand-written example, PR 6 names which
  was wrong. It is read-only and calls no mutating tool.
- **PR 2** (`m3.2-adrs`) takes decisions 2 and 3 as given: question 5 is closed here, and
  question 13's list omits `get_song` for the model, with the view as the song summary. The
  system prompt's sentences about ticks, MIDI numbers and what the axes are not come from
  decisions 1 and 4.
- **PR 8** (`m3.8-loop`) sends the view as the first message and re-reads it after every
  applied call; which `Song` the axes describe for a proposal is question 9's, and the axes do
  not care. Nothing in the loop compares an axis to anything.
- **No `proto/` change and no `song.proto` change.** `buf breaking` has nothing to compare, and
  no golden of M0.4, M1 or M2 moves.
- `docs/specs.md` §15 gains this ADR's row and §18.2 a pointer to it; §18.2's "record this in
  an ADR before M3" is satisfied. §18.2's *claims* are untouched. Area 4's "no public code
  found" — that one claim, with its echoes in the landscape's table and caveats and in §18's
  overlap list — is corrected and dated in this pull request; the rest of the landscape waits
  for its quarterly re-check.
- `docs/plan.md`'s deferred ledger gains one row from decision 6 (the Libretto package, on a
  person's sign-off after they assess its provenance) and one from decision 4 (the deferred
  features, on a corpus this repository may redistribute) — added when PR 2 walks the ledger,
  since this pull request is the ADR alone.
- An MCP client does not get the view (decision 3), and "What M3 will not claim" says so.
