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
  anything automated them.
- ADR 0007 §5's descriptor-driven coverage test is unaffected: no `song.v1` field is added, and
  `Mix` was already carried into the plan.
- §4.4 gains a bullet and §5 a sentence. Neither is renumbered.
- Two of the three ledger rows the mixer was said to force stay deferred, with triggers that name
  an event rather than a milestone; the third — `ParamRef` reaching mix params — is closed here.
