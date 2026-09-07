# ADR 0011 — `AudioClip` gains gain, fades and a stretch flag, and M1 renders it

- **Status:** Accepted (2026-09-05)
- **Affects:** `schema/song.proto` (`AudioClip`), `schema/gen/`,
  `tests/fixtures/song/minimal.json`; `core/src/render.rs` (M1); `engine/` (M1);
  `docs/specs.md` §8
- **Builds on:** ADR 0002 §8 (the reductions table, and this row's own upgrade clause), §4
  (canonical JSON emits defaults), §5 (`optional` only where absent ≠ zero) and §12 (a replay
  and its schema boundary); ADR 0007 §2 (leaf messages cross by value) and §4 (`compile` reads
  no file)
- **Recorded in:** `docs/specs.md` §8 and §15.

## Context

`AudioClip` is `{ string asset_hash = 1; }`. That is deliberate, and ADR 0002 §8 wrote the
clause that ends it:

| §4.2 says | v1 has | Upgrade path |
|---|---|---|
| `AudioClip` | `{ asset_hash }` | Gain, fades and time-stretch **when M1 renders audio clips**. |

M1 renders audio clips (`docs/plan.md`, decisions of 2026-09-04), so the clause fires. This is
the ADR that CLAUDE.md #5 requires before the `.proto` changes, and its job is to spend the
clause exactly — not to design an audio editor.

The reason a bare `asset_hash` is not renderable is narrower than "it would be nice to have
gain". Three specific things are missing, and each of them is missing *in a way the rest of the
model cannot supply*:

- **Level.** `Mix.gain_db` is per *track*. The only other way to change a level is automation,
  and a `ParamRef` names a `device_id` — an `Instrument` or an `Effect`. An audio track has
  neither; `core/src/validate.rs` builds its device set from `track.instrument` and
  `track.fx_chain`, so no `ParamRef` in a valid song can point at an audio clip. A clip that is
  6 dB hot therefore has **no** representable fix short of editing the asset.
- **Boundaries.** An asset that does not begin and end at a zero crossing produces a
  discontinuity at the clip edge. That is not a cosmetic blemish: it is a full-scale transient,
  it is audible on every playback, and it is in the golden.
- **Fit.** A loop recorded at one tempo does not line up with a song at another. Without
  stretch, the only audio a project can use is audio already at its tempo.

## Decisions

### 1. Four fields, numbered 2 to 5

```proto
message AudioClip {
  string asset_hash = 1;

  // Clip gain. 0.0 is unity; an audio track has no device, so no ParamRef can reach this.
  double gain_db = 2;

  // Ramps at the clip's edges, clip-relative, in ticks. 0 is no fade.
  int32 fade_in_ticks = 3;
  int32 fade_out_ticks = 4;

  // Stretch the asset to fill the clip's musical length (ADR 0011 §3).
  bool time_stretch = 5;
}
```

Field 1 is the only one in use today, so 2–5 are free and nothing is renumbered.

Every default is the current behaviour: `gain_db = 0.0` is unity, both fades are zero, and
`time_stretch` is false. A document written before this change therefore means exactly what it
meant, which is what makes decision 4 possible.

None of them is `optional`. ADR 0002 §5 admits `optional` only where absent differs from zero,
and for all four the zero *is* the neutral value — unlike `Clip.loop_length_ticks`, where
absence carries the boolean.

Ticks, not seconds or samples, for the fades: §4.2 makes ticks the only musical time, and
ADR 0007 §3 carries ticks to the engine with the tempo map beside them. A fade expressed in
seconds would be a second time base in the model and a second conversion in the engine.

**What is deliberately not added.** An offset into the asset — trimming where playback starts
— is outside ADR 0002 §8's clause and is not needed to render: a clip shorter than its asset
plays its first `length_ticks` and stops, which is defined. It is additive whenever a tool
wants to trim, exactly as this change is. A fade *curve* field is decision 2. A stretch
*ratio* is decision 3.

### 2. The fade formula is ours, linear in amplitude, and it is total

The engine implements this and not Tracktion's own fade shapes, for the reason ADR 0002 §8
gives for the `Curve` enum: an interpolation the renderer chooses is one §11's bit-exactness is
at the mercy of, and a shape nobody wrote down cannot be reimplemented by a second renderer.

For a sample at index `n` from the clip's start, where `N` is the clip's length in samples and
`f_in`, `f_out` are `fade_in_ticks` and `fade_out_ticks` converted to samples on the same tempo
map the engine already uses for note positions (ADR 0007 §3):

```
g_in(n)  = 1                       if f_in  == 0
         = min(1, n / f_in)        otherwise

g_out(n) = 1                       if f_out == 0
         = min(1, (N - n) / f_out) otherwise

output(n) = input(n) * 10^(gain_db / 20) * g_in(n) * g_out(n)
```

Three properties are worth stating because they are what remove special cases:

- **`g_in(0) = 0` when a fade-in is set.** The first sample is silent. That is what removes the
  click, and a formula that started at the first sample's full value would not.
- **The two ramps multiply**, so fades that overlap — `fade_in_ticks + fade_out_ticks > N` —
  need no rule. Both ramps apply, neither reaches 1, and the result is continuous. A clamping
  rule would be a second thing to specify, test and get wrong.
- **Linear in amplitude, not in dB.** A dB-linear ramp never reaches silence, so it does not
  remove the discontinuity it exists for.

There is **no curve field**, and reusing `Curve` would have been the obvious mistake: it offers
`LINEAR` and `HOLD`, and `HOLD` as a fade shape is a step — the discontinuity itself. Adding an
equal-power shape is the natural extension and it is additive, but it earns its place at a
crossfade, and M1 has none: §4.4 forbids overlapping clips unless `track.allow_overlap`, and
nothing in M1 sets it. A shape gets added when something needs it and a formula is written for
it, which is ADR 0002 §8's standing rule.

### 3. Stretch is a flag; the ratio is the engine's, and Rubber Band's configuration is pinned here

`time_stretch` is a `bool`, not a ratio.

When it is false the asset plays at its natural rate from its start, and the clip ends at
`length_ticks` or when the asset runs out, whichever comes first. When it is true the asset is
time-stretched — pitch unchanged — to fill the clip's musical length: to `loop_length_ticks`
if the clip loops, so the stretched unit is what repeats, and to `length_ticks` otherwise.

A ratio would be a `double` in the model whose only correct value is derived from the asset's
duration and the tempo map. Core cannot compute it: `compile(song, assets)` takes an index from
hash to path and **reads no file** (ADR 0007 §4), and that purity is what lets PR 4 golden the
plan. So a ratio would have to be computed by the *caller* — which in this system is often an
LLM that would have to know an asset's frame count — and it would then be silently wrong the
moment a tempo event moved. The flag states the intent; the engine, which opens the file
anyway, derives the ratio. This is not the engine learning model semantics (ADR 0007 §1): the
asset's duration is a fact about an audio file, which is the engine's own subject.

Determinism is unaffected, because the ratio is a function of the document and the pinned,
content-addressed asset — the same two inputs everything else in the render is a function of.

**Rubber Band's configuration is pinned here**, because ADR 0009 §4's determinism note for it
needs something to point at. Trap 15's observation is that a pinned *version* of a phase
vocoder is not a pinned *output*: the same input at two option settings is two different
signals, both correct. Read off the pinned 4.0.0 header
(`1d95888bec3ae0a17c0c4af791810d5a63f6bc35`), the option word is written out in full rather
than OR-ing a few flags onto the defaults:

**Completed 2026-09-07, in PR 8.** The table below listed eight of the twelve option groups
`RubberBandStretcher.h` declares, and the sentence above it says "in full". The four missing
ones — `OptionStretch`, `OptionSmoothing`, `OptionFormant` and `OptionChannels` — are added to
the table's last row, all at their zero-valued defaults, for the same reason as the five that
were already there. Nothing changes about what the engine computes; what changes is that
"written out in full" is now true. The same PR found that the vendored build is a stronger pin
than this table alone: it is upstream's own `single/RubberBandSingle.cpp`, which hard-defines
`USE_BUILTIN_FFT` and `USE_BQRESAMPLER`, so the FFT is not chosen from what the build machine
happens to have installed — which would be a determinism hazard this ADR had not named, since
a phase vocoder over two FFTs is two different signals.

| Option | Why |
|---|---|
| `OptionProcessOffline` | The whole clip is known before the render starts; offline processing uses the study pass and is the higher-quality path. There is no real-time constraint in an offline render. It is also what makes Rubber Band pad and compensate its own delay so the result has an exact start and duration, which is what lets the engine ask for a buffer exactly the clip's length. |
| `OptionEngineFiner` | The R3 engine, for quality. Determinism does not prefer either engine — it requires only that the choice is fixed and not inherited from a default that a future version may move. |
| `OptionThreadingNever` | Trap 15 and ADR 0009 §3's single-thread requirement. `OptionThreadingAuto` is the default and lets Rubber Band decide, which makes the output a function of the machine's core count. **Measured 2026-09-07, in PR 8:** it is inert twice over, and passed anyway. The flag is read only by the R2 engine — `src/faster/R2Stretcher.cpp` is the only file in the library that mentions it — and the single-file build compiles threading out with `NO_THREADING`. A configuration that would change meaning if the engine choice moved is not a configuration. |
| `OptionTransientsCrisp`, `OptionDetectorCompound`, `OptionPhaseLaminar`, `OptionWindowStandard`, `OptionPitchHighSpeed`, `OptionStretchElastic`, `OptionSmoothingOff`, `OptionFormantShifted`, `OptionChannelsApart` | Each of these is the current default (numerically zero), so the word comes to `OptionEngineFiner` \| `OptionThreadingNever` and nothing else. They are written explicitly so that an upstream change to any of them shows up as a diff in our source rather than as a golden that moved for no reason anyone can find. `OptionStretchElastic` is marked obsolete in the pinned header and named for completeness, not effect. |

An asset whose sample rate differs from the render target is **converted, not stretched** —
that is resampling, and it happens whether or not `time_stretch` is set. ADR 0009 §4 lists the
converter as a DSP surface of its own for exactly that reason. **Chosen 2026-09-07, in PR 8:**
`juce::LagrangeInterpolator`, and its note is in §8.

### 4. `schema_version` stays 1, and `buf breaking` has nothing to report

Adding fields to an existing message is additive: no field number is changed or reused, so
`buf breaking`'s `FILE` category passes and the check needs no exception. Saying so explicitly
because the M1 plan puts a `buf breaking`-guarded `.proto` change in the same milestone, and
the two must not be confused.

`SCHEMA_VERSION` stays **1**. ADR 0002 §12 puts the version on a `PatchEntry` so "a replay can
tell when it crosses a schema boundary", and a boundary is one a replay has to *handle*. There
is nothing here to handle: a document written before this change replays into the new type and
takes the proto3 defaults, which decision 1 chose to be exactly the previous behaviour.

One consequence is worth recording, because it is general and this is the first schema change
since the project store existed. `Project::verify_against_replay` compares the replayed log
against `serde_json::to_value(&self.song)`, and canonical JSON emits defaults (ADR 0002 §4). So
for a project written *before* this change whose log contains an `add` of an `AudioClip`, the
replayed value has three keys and the re-serialised song has six, and `open` would report
`song_diverged` — blaming `song.json` for what is really a schema change.

**M1 strands nothing**, because no such project exists: nothing under `tests/` constructs an
`AudioClip` (the string appears in no fixture, script or golden), and `add_asset` — the tool
that makes an `asset_hash` mean anything — arrives in the same milestone. The hazard is
recorded here so the next additive change checks it rather than discovering it in someone's
project, and so the first change that *would* strand a real document is the one that bumps
`SCHEMA_VERSION` and owes a migration.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Leave `AudioClip` at `asset_hash` and render it as-is | An audio clip with no level control cannot be balanced by anything else in the model, and every clip edge is a click in the golden. ADR 0002 §8 wrote the upgrade clause for this milestone. |
| A `Curve` field on the fades | `Curve` offers `LINEAR` and `HOLD`; `HOLD` as a fade is the discontinuity being removed. One usable value is not a choice. |
| An equal-power fade shape now | Earns its place at a crossfade, and M1 has none — §4.4 forbids overlapping clips unless `allow_overlap`. Additive when something needs it, with a written formula, per ADR 0002 §8. |
| Use Tracktion's own fade types | The renderer choosing the shape is what ADR 0002 §8 refused for automation curves; §11's bit-exactness would rest on a curve nobody wrote down. |
| Fades in seconds or samples | A second time base in a model where §4.2 makes ticks the only musical time, and a second conversion in the engine (ADR 0007 §3). |
| `double stretch_ratio` | Its only correct value derives from the asset's duration, which `compile` cannot read (ADR 0007 §4) — so the caller computes it, and it goes silently wrong when a tempo event moves. |
| A per-clip Rubber Band mode | Makes the render's DSP configuration part of the document, so a determinism note would have to cover every setting rather than one. §14.7: a different renderer would not need it. |
| An `offset_frames` field for trimming | Outside ADR 0002 §8's clause and not needed to render — a short clip plays its first `length_ticks` and stops. Additive when a trim tool exists. |
| Bump `SCHEMA_VERSION` to 2 | The change is additive and its defaults are the previous behaviour, so a replay has nothing to handle (ADR 0002 §12). Bumping would refuse every existing project with no migration to offer them. |

## Consequences

- **PR 2** is this ADR's code and nothing else: the four fields in `schema/song.proto`,
  `./schema/codegen.sh` for all three languages, `tests/fixtures/song/minimal.json` regenerated
  with `UPDATE_FIXTURES=1`, and the Rust, TypeScript and Python round-trip suites that read it
  (`schema/AGENTS.md`, Adding things). No new construct is introduced — a `double`, two `int32`
  and a `bool` are all already exercised — so `build()` in `schema/tests/roundtrip.rs` needs only the
  new values, not a new case.
- **ADR 0002 §8's `AudioClip` row** gains a pointer to this ADR in the same commit, following
  the precedent that ADR text relying on a superseded state is updated with it
  (`docs/adr/AGENTS.md`, Amending).
- **PR 3** adds `AddAsset` to `SongTools`, which is what puts a file in `assets/` and makes an
  `asset_hash` resolvable. It needs a hasher; `docs/plan.md` records the decision that `core`
  hashes with `sha2` rather than the engine, which would put the engine in the project-writing
  path against CLAUDE.md #6. `sha2` 0.10 is pinned in `lock.baseline.json` by this PR; the
  spike found the engine could not hash cheaply anyway, since `juce::SHA256` lives in
  `juce_cryptography` and Tracktion does not pull it in.
- **Trap 16 lands here.** An audio asset makes `assets/` **non-empty for the first time**.
  `Project::write` creates the directory, and M0.4's comparison ignores it precisely because
  git cannot store an empty one — so the first golden containing an asset changes what the
  determinism suite compares, and the suite must compare `assets/` contents by name and bytes
  from that PR rather than skipping the directory.
- **PR 4** carries `AudioClip` into the `RenderPlan` by value (ADR 0007 §2), beside the
  absolute path core resolves from `asset_hash`. The field-coverage guard of ADR 0007 §5 covers
  the four new fields automatically, because the whole message is embedded.
- **PR 8** implements decisions 2 and 3: asset playback, the fade and gain formulas, and
  Rubber Band vendored at the pinned commit with the pinned option word. ADR 0009 §4's Rubber
  Band and sample-rate-conversion notes are written there.
- **PR 11** goldens a render containing an audio clip, which is what proves the formulas rather
  than asserting them.
- **§8** records the pinned Rubber Band configuration beside its existing time-stretch line;
  **§15** gains a row. §4.2's prose is unchanged — it says `AudioClip` and does not enumerate
  its fields.
