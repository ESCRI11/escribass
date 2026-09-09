# ADR 0015 — A `ParamRef` reaches a track's fader, and it costs no schema change

- **Status:** Accepted (2026-09-07)
- **Affects:** `schema/song.proto` (**no change** — see decision 1); `proto/render.proto`
  (`PlanTrack`); `proto/song_tools.proto` (`merge_branch`); `core/src/validate.rs`;
  `core/src/render.rs`; `core/src/merge.rs`; `engine/`; `docs/specs.md` §4.4 and §5
- **Builds on:** ADR 0001 §3 (id-keyed maps, and ids unique across every collection), §4 (merge
  auto-resolves disjoint paths and refuses the same path); ADR 0002 §8 (the automation curve
  formulas are ours); ADR 0010 §4 (the build manifest, and a plan parameter's normalised value)
- **Recorded in:** `docs/specs.md` §4.4, §5 and §15; ADR 0010 §4 amended in place.

## Context

Three rows of `plan.md`'s deferred ledger name the mixer as their revisit point, and M2 is the
mixer. Taken at face value that is three ADRs and two `song.proto` changes before a pixel is
drawn, and a regeneration of the M0.4 goldens with each. Taken one at a time it is less than
that, and the difference is worth the reading.

The one that is real: **`ParamRef` cannot address a fader.** `ParamRef { device_id, param }`
resolves `device_id` to an `Instrument` or an `Effect` and `param` to a parameter that device
declares. `Mix { gain_db, pan, mute, solo }` is a field on `Track`, not an entity, so the
commonest automation in any DAW — a fader ride — has nothing to point at. Plate 1 of the
wireframes draws that lane. §4.4 requires automation targets to resolve, and today they resolve
only to plugin parameters, so the model can express a mix and cannot express a mix *changing*.

## Decisions

### 1. A `ParamRef` may name a track, and then `param` is `gain_db` or `pan`

`ParamRef.device_id` names a **track**, and `ParamRef.param` is one of two literal names,
`gain_db` and `pan`. That is the whole change, and it adds **no field to `song.proto`.**

It works because ADR 0001 §3 and §4.3 already did the work: ids are ULIDs, stable and globally
unique across every collection, so an id either names a track or names a device and never both.
`core/src/tools.rs`'s `device_path` already relies on exactly that property to let `set_param`
omit a track id. Resolution gains one arm, not a new mechanism, and `device_unknown` and
`param_unknown` gain a track's spelling of the same two failures.

The field's *name* stays `device_id` and is not renamed to something truer. Renaming a proto
field is wire-compatible in binary and is **not** compatible in JSON, and canonical JSON is the
project file (ADR 0002 §4) and every M0.4 golden. A field name is not worth a migration of every
project on disk; the comment says what it names.

**`mute` is refused, and `solo` with it.** Both are booleans and `AutomationPoint.value` is a
double, so automating one needs a threshold rule — is 0.4 muted? — and that rule would be ours,
pinned, and a determinism surface, exactly as ADR 0011 §2's fade shape and ADR 0002 §8's curve
formulas are. A step lane over a bool is a real feature in real DAWs and it is not one M2 wants;
refusing it removes a formula from the pinned set rather than adding one. `param_unknown` names
the two that are allowed. Revisit when someone asks for a mute lane, which is a request, not an
inference.

### 2. A mix lane's values are in the model's own units, and `PlanTrack` gains `mix_lanes`

ADR 0010 §4, extended, settled that a plan parameter's value is the plugin's **normalised**
`0.0`–`1.0`, because VST3 exposes exactly one numeric domain to a host and a plugin's own units
exist only as a display string. **Amended here:** that holds for a parameter of a *device*. A
mix parameter is not a plugin's; it is the model's own field, with the model's own units and the
model's own range — `gain_db` in decibels, unbounded above and below; `pan` in `-1.0`..`1.0`, as
`song.proto` has said since M0.1. There is no plugin to normalise against and no display string
to read a unit off, and inventing a normalisation would mean choosing a maximum gain, which is a
number nothing in the model has.

So a lane's `AutomationPoint.value` means whatever the parameter it targets means, and which of
the two it is follows from what `device_id` resolved to. The asymmetry is stated rather than
smoothed over, because `param_out_of_range` (ADR 0010 §4, settled) checks `0..1` and must now
check `-1..1` for `pan` and nothing at all for `gain_db`.

The plan carries it as `PlanTrack.mix_lanes`, a `repeated PlanLane` beside the `mix` that is
already there — the same nesting ADR 0007 §2 chose for a device's lanes, for the same reason: a
lane nests under what it automates rather than naming it, because a name in the plan is an id
crossing the boundary. **It lands in PR 3**, in the same change as `Preview` (ADR 0013 §2), so
`buf breaking` sees `render.proto`'s whole M2 shape once against a `main` that has not moved
(trap 12).

The M0.4 goldens do **not** move: they are `song.json`, `refs.json` and `patches/`, and no
`song.proto` field changed. The **plan** goldens do — canonical JSON emits defaults, so every
`expected/plan.json` gains `"mix_lanes": []` — and that is PR 3's mechanical diff, in the PR
that makes it.

**Extended 2026-09-09, in PR 6, where a lane first reached a fader.** This decision said the
lane's units are the model's and left unsaid what the *engine's* fader keeps. It is not
decibels: Tracktion's volume parameter is a **slider position**, `exp((dB - 6) / 20)`, and its
header says so — so the conversion is the engine's, on the way in, and it has two consequences
this decision did not foresee.

*A straight line in decibels is not a straight line in that domain.* Tracktion interpolates a
curve in the parameter's own units, so two endpoints render an exponential as a chord: a
0 dB → -36 dB ride passes through -10.8 dB at its midpoint where ADR 0002 §8's formula says
-18. The engine therefore **splits a `LINEAR` `gain_db` segment into pieces spanning at most
0.02 dB**, beside the tempo split it already made — which holds the chord within 1.1e-6 dB,
about one count of 24-bit full scale. Unlike the tempo split this is an approximation with a
stated bound rather than an exact reconstruction, because no finite number of straight pieces
is an exponential; it is in any case orders of magnitude below what the rate at which Tracktion
*reads* a curve already contributes — it holds a parameter for a whole automation sub-block,
2.7 ms at 48 kHz, which on the ride above is 0.048 dB. `pan` needs none of this: Tracktion's pan parameter *is* `Mix.pan`,
same number, same range, so the lane crosses unconverted and the pan law turns the position
into two gains afterwards, at the `PanLawLinear` §8 pins.

*The domain brings Tracktion's own bounds with it.* The parameter stops at 1, which is +6 dB,
and below -100 dB it is silence — so a `Mix.gain_db` above +6 renders as +6. That ceiling is
not new and is not this decision's: `setVolumeDb` clamps the same parameter, so a *static* mix
has had it since M1. It is written down here, and in §8, rather than discovered. The model
stays unbounded, as this decision says, because a ceiling in the model would be the invented
maximum gain the alternatives table rejects; if a project ever needs more, the upgrade is a
gain applied ahead of the fader, the way an audio clip's is (ADR 0011 §2).

### 3. `merge_branch` gains a per-path resolution; recursive merge stays deferred

ADR 0001 §4 auto-resolves disjoint paths and refuses the same path as a structured error, and
deferred interactive resolution to M2 for the reason M2 removes: "designing the API with no UI
and no real conflicts". M2 has the history view (ADR 0003 §5) and the conflicts are already
structured.

The smallest thing that finishes a merge is one optional field on `MergeBranchRequest`: a map
from the conflicting RFC 6902 path to the side that wins. A conflict comes back as it does
today; the user picks per path in the history view; the same call is made again carrying the
picks. No new tool, no session state between the two calls, no merge that is half-committed,
and `dry_run` works on the second call exactly as it does on the first. It is a `song_tools.proto`
change, additive, which `buf breaking` reports nothing about, and it is not a `song.proto`
change, so CLAUDE.md #5's ADR-before-schema rule is satisfied by this paragraph.

**Recursive merge — a criss-cross base — stays deferred**, with a trigger that can fire:
`merge_base` has no single answer when two branches have each merged a third, and *nothing in the
repository produces that history yet*. Refusing is the current behaviour and it is loud. The
trigger is a real criss-cross, and M2's history view is what makes branch merging ordinary
enough for one to appear.

**Extended 2026-09-09, in M2 PR 8, where the field was built and used.** This decision named the
shape and left four things to whoever implemented it. All four were settled against a real
conflict raised in the window — two branches riding the same track's `gain_db` and `pan` — rather
than against an imagined one, and the first of them is the one the decision could most easily
have got wrong:

*What a conflict actually returns matched the assumption.* `merge_branch` refuses with an
ordinary `ToolResult` — `valid: false`, `patch` empty, nothing written — carrying one
`merge_conflict` `Violation` per conflicting path, whose `path` is the **other** side's operation
path and whose message names both values (`core/src/merge.rs`). So "a map from the conflicting
RFC 6902 path" is a map keyed by a string the caller was already handed, and there was nothing to
translate.

*Resolution is a filter on the other side's operations, and nothing else.* Theirs wins at a path
by being applied, which is what an unconflicted merge already does to every one of them; ours
wins by that operation not being applied. There is no third value computed anywhere and no code
path a merge without conflicts does not already take. One consequence is worth naming because it
is a refusal rather than a result: choosing *theirs* at a path whose parent **this** branch
removed cannot be honoured — the operation arrives at a document where its parent is gone and
`prepare_merge` answers `path_not_found`. That is loud, and the answer is to resolve at the
parent path. A merge that reconstructed the subtree instead would be the strategy framework this
decision exists to avoid.

*A pick for a path that is not one of that merge's conflicts is refused*, with the rule
`resolution_unknown`. Ignoring it was the alternative and it is worse in both readings: as a typo
it leaves a person believing they chose something, and taken at face value it would drop the
other side's change at a path nothing disagreed about, which is editing the merge rather than
resolving it. It is also how a caller learns the conflicts moved under a held preview — the
merge's version of ADR 0012 §4's optimistic apply.

*A merge that keeps this branch's value everywhere still records its entry*, and it is the one
place in the tool API where a call that changes no document writes one. Everywhere else "a call
that changes nothing records nothing" is right; here what the entry records is not a change to
the document but the **join**, and without it the two branches stay unmerged, `merge_base` keeps
finding the old base, and the same conflict is reported for ever. The entry has two parents and
an empty `ops` array, which replays as the no-op it is.

The recursive-merge deferral is unchanged and its trigger is now genuinely reachable: the history
view makes branch merging ordinary, and merging two branches that have each merged a third is
four clicks. Nothing in the repository has produced one yet, and `merge_base_ambiguous` is what
would say so.

### 4. Dense unique `index` defers again, and M2 is held to the scope that makes that safe

The row's harm is real: inserting mid-list renumbers everything, and two branches inserting at
one index auto-merge into a document the validator refuses. It is also **not the mixer's**.
A mixer that draws a chain in `index` order and adds or removes at the end needs no renumbering
at all; what needs it is a *reorder*, and reordering an effect chain by dragging is not
something M2's mixer offers.

So the trigger stops being "M2, with the mixer" — which is a milestone, not an event — and
becomes **the first gesture that reorders an effect chain or inserts a track mid-list**. That is
a specific thing a specific PR would add, and M2 is on record as not adding it. The merge half
stays where M0.3 left it: the failure is loud, because the validator refuses the merged
document, rather than silent.

This is the one place this ADR chooses less than the ledger asked for, so the reason is written
plainly: closing it properly is a `song.proto` change and therefore an ADR, a proto PR, a
regeneration of every M0.4 golden and a walk through them byte by byte — the shape M1 PR 9 had
to do — bought for a gesture nothing in M2 makes.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Add a `TrackParamRef`, or a `oneof` arm on `ParamRef` | A schema change, an ADR-before-code coupling, a `buf breaking` run and a regeneration of every M0.4 golden — to express something two globally unique ids already express. ADR 0001 §3's uniqueness is not a coincidence to be re-solved. |
| Give `Mix` an `id` and make it an entity | Turns a value into an entity for one reference's sake, adds `provenance` and `version` to a struct of four scalars, and changes every `song.json` on disk. |
| Rename `device_id` to something truer, e.g. `target_id` | Wire-compatible in binary, **not** in JSON — and canonical JSON is the project file and every golden. A truer name is not worth migrating every project. |
| Automate `mute` and `solo` too | A double automating a bool needs a threshold rule that is ours, pinned and a determinism surface, for a lane M2 does not draw. |
| Normalise mix parameters to `0..1` like a plugin's | Needs a maximum gain, which nothing in the model has. A normalisation whose endpoints are invented is a number that means nothing on either side of the boundary. |
| Name mix lanes in the plan by track id rather than nesting them | ADR 0007 §1's rule: an id naming another entity is exactly what does not cross into the plan. |
| A `resolve_conflict` tool, or merge state held between calls | A second tool and a session that remembers a half-finished merge, for what one optional field on the existing call does. ADR 0006 §5's reason against a project handle applies to a merge handle unchanged. |
| Take dense `index` now, since the mixer is here | The mixer is not what needs it; a reorder gesture is, and M2 has none. It is a schema change and a goldens walk for a feature nobody is asking for in this milestone. |

## Consequences

- `core/src/validate.rs`: `device_unknown` and `param_unknown` resolve a track id;
  `param_out_of_range` gains `pan`'s `-1..1` and exempts `gain_db`. The M1 comment that the
  validator "needs the plugin's manifest" is unaffected — a track needs no manifest, which is
  why this rule can be written without one.
- `core/src/render.rs`: `compile` emits `PlanTrack.mix_lanes`, sorted, with the same tick-ordered
  points a device lane gets, and the same two curve formulas (ADR 0002 §8).
- `engine/`: a mix lane drives Tracktion's own track volume and pan parameters, and the engine
  still implements **our** curve formulas rather than Tracktion's — and does so on top of the pan
  law and the master gain §8 already pins, which is why those two were worth writing down before
  anything automated them. §2's extension says what that cost: a conversion into the fader's
  slider position, a subdivision to keep our line straight through it, and one more Tracktion
  default pinned beside the pan law — `smoothingRampTimeSeconds`, set to zero, because a 15 ms
  ramp towards each sub-block's target is a shape the renderer chose and ADR 0002 §8 refuses
  exactly that for an automation curve. None of it moves a render with no lane in it.
- ADR 0007 §5's descriptor-driven coverage test is unaffected: no `song.v1` field is added, and
  `Mix` was already carried into the plan.
- §4.4 gains a bullet and §5 a sentence. Neither is renumbered.
- Two of the three ledger rows the mixer was said to force stay deferred, with triggers that name
  an event rather than a milestone; the third — `ParamRef` reaching mix params — is closed here.
