# ADR 0017 — A gesture is previewed continuously and committed once

- **Status:** Accepted (2026-09-09)
- **Affects:** `app/src/App.tsx`, `app/src/canvas.tsx`; `docs/specs.md` §5, §9; every gesture M2
  and M3 add
- **Builds on:** ADR 0006 §3 (`dry_run` is the pure first half of the apply path, never a second
  implementation of it); ADR 0012 §2 (one decoded `Song`, no local apply, no second
  implementation of core's rules in the frontend), §4 (a held preview is applied
  optimistically); ADR 0005 §4 (undo appends an inverse entry)
- **Recorded in:** `docs/specs.md` §5, §9 and §15.

## Context

§5 says every control is a tool call and §14.3 says nothing in `app` writes `song.json`. Neither
says **when** the call is made, and a drag is where that stops being a detail: dragging a note
across a window fires a pointer event every few milliseconds, and each one describes a different
document.

`docs/plan.md`'s M2 trap 2 stated the problem and deliberately left it open for this pull
request:

> **A drag is not one tool call.** Dragging a note fires a hundred pointer events. One
> `set_notes` each puts a hundred entries in the log §5 calls both the audit trail and the undo
> history, and ⌘Z then undoes one pixel. Coalescing on release is the obvious fix and has a
> sharp edge: the intermediate states are unvalidated, so a drag can pass through a position the
> validator would refuse and land somewhere legal, and the refusal a user should have seen at
> the boundary never happens.

It was left open on purpose — "there is no answer right for every gesture, and PR 5 decides it
against a real drag rather than an imagined one" — so what follows is decided from measurement.
The measurements are in decision 5.

The sharp edge dissolves once the two halves of "coalescing" are separated. What makes
intermediate states unvalidated is not committing once; it is **sending nothing until the
release**. Those are independent choices, and this ADR takes one of each.

## Decisions

### 1. One gesture is one entry in the log, whatever it costs in pointer events

A drag produces exactly one `PatchEntry`. The log is the audit trail *and* the undo history
(§5), and both readings agree: nobody wants a hundred rows in a history view saying a note moved
a pixel, and ⌘Z has to undo the drag rather than the last frame of it.

This is a statement about the **log**, not about the tool API. Nothing about it requires the
intermediate positions to be hidden from `core`, and decision 2 does not hide them.

### 2. Every position the gesture passes through is a `dry_run`, one call in flight at a time

The frontend sends the position under the pointer as `set_notes` with `dry_run: true` and draws
what comes back. A dry run is `Project::prepare`: it applies, bumps versions, validates and
re-derives the patch, and it **writes nothing, mints no id and appends no entry** (ADR 0006 §3,
M0.3 PR 11). So a position can be asked about as often as the pointer moves, and asking costs
the log nothing.

Calls are not queued. One is in flight; a position arriving while it is runs only if it is
still the newest when that flight lands, and anything a later position overtook is dropped. A
preview describes a position, only the newest position is on screen, and a stale answer has
nothing to say. That is eight lines in `App.tsx` and it is the whole rate limit — no timer, no
debounce interval, no scheduler.

The last position always reaches the tool API: a move either fires immediately or is queued and
fires when the flight lands, and the release adds no call of its own.

### 3. A refused position is drawn refused, and the frontend contains no rule of its own

This is the answer to the sharp edge, and it is the reason decision 2 exists.

The three ways a note drag leaves §4.4 are `pitch_out_of_range`, `tick_negative` and
`note_outside_clip`, and a drag reaches all three by moving a few tens of pixels. When the tool
API refuses the position under the pointer, the dashed proposal is drawn in the refusal colour
and the pane names the `rule` and the message the validator returned. Let go there and nothing
is applied: the pane offers `Discard` and no `Apply`.

**A drag that crosses a refused position and lands on a legal one applies, and that is correct.**
The intermediate positions were never states of the document — they were candidate values, each
one put to the validator and answered. What lands is what the person asked for, and it is legal.
What the trap was really protecting against is a boundary crossed *invisibly*, and that is what
decision 2 removes: the one thing on screen that changes at the boundary is the note being
dragged.

The frontend implements none of this itself. There is no `pitch < 0 || pitch > 127` in `app/`,
and there is not going to be one: §4.4 belongs to the validator, and a copy of it in TypeScript
is exactly the second implementation ADR 0012 §2 refused for RFC 6902 apply, arriving as a
convenience. Clamping the drag to a legal range would be worse than either — it is that second
implementation *and* it makes the boundary silent, which is the failure the plan asked this pull
request to avoid in the words "make it observable rather than silently legal".

The one thing the frontend does decide is how tall to draw the roll: the drawn pitch band grows
to hold the proposal, clamped to MIDI 0–127. A legal drag is therefore always visible, and an
illegal one is drawn against the edge in the refusal colour rather than followed off the canvas.

### 4. Releasing the pointer proposes; a person applies

The release ends the gesture and leaves the last dry run on screen as §9's diff — the
`ToolResult.patch` a commit would record, version bumps included — with `Apply` and `Discard`.
Applying is the same call with `dry_run: false`, followed by `get_song` and a wholesale
replacement of the decoded song (ADR 0012 §2). It is applied optimistically: the model may have
moved under the held preview, and §4.3's `version` check is what refuses if it did (ADR 0012 §4).

`ponytail:` **approval per gesture is M2's shape, not a law of direct manipulation.** §9 asks for
a diff before applying and this is the first control that has one, so it gets the flow whole,
where it can be reviewed. A gesture that commits on release with the diff shown afterwards is a
reasonable thing to want, and ⌘Z is what would make it safe; the trigger is the first time
per-gesture approval is measured as friction rather than argued about, and the row is in
`docs/plan.md`'s deferred ledger.

`undo` and `redo` are the exception and are applied directly. An undo reverses a change that was
already approved, and asking for approval to withdraw approval is a dialog with nothing in it.

### 5. What was measured, and on what

Measured on this machine (Linux x86-64, WSL2), release builds, against the project
`tests/determinism/render/script.json` builds — five tracks, four clips, 21 entries — driven
through a real `escribass-mcp` process, which is an upper bound on the cost of the app's path
because the app's is an in-process call with no pipe and no JSON-RPC framing.

| Measured | Result |
|---|---|
| One `set_notes` **dry run** | median 1.0–2.8 ms over two runs, p95 4.2 ms. Writes nothing |
| One `set_notes` **applied** | median 16–21 ms at 21 entries in the log |
| The same applied call at 300 entries | median 30.6 ms — **half again as expensive as the first**, because `Project::write` rewrites every entry file on every commit |
| 300 applied `set_notes` in a row | 6.7 s, and 321 files in `patches/` |
| A real drag in the window: pointer positions injected | 100, over about 0.8 s |
| …that reached the tool API | 30, every one a dry run |
| …entries appended by the drag | **0** |
| …entries appended by pressing `Apply` | **1** |
| …entries appended by ⌘Z, and by ⇧⌘Z | **1 each** — undo appends (ADR 0005 §4) |
| The dragged note's `version` across move · undo · redo | 1 → 2 → 3 → 4 |

The drag deliberately went left past tick 0 before turning back and landing legally, which is
the trap's scenario exactly. At the boundary the note drew refused and the pane read
`tick_negative ticks are never negative`; on release the pane showed a five-operation patch and
the document had not moved.

**The number that decides it is not the dry run's cost; it is the applied call's slope.** One
entry per pointer event is not merely a hundred rows in the history — it is a hundred writes
that each rewrite the whole log, so the *hundredth* costs more than the first, and a session
that keeps dragging gets slower for the rest of its life. That is `docs/plan.md`'s trap 8 and
the "known gap" it names, arriving in the pull request the plan predicted it would ("PR 5 is the
first PR that appends in a loop"), and it is a second, independent reason for decision 1 that
had nothing to do with the audit trail.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| One applied `set_notes` per pointer event | Measured: 300 of them take 6.7 s and the per-call cost rises by half as the log grows, because `Project::write` is O(history). And ⌘Z then undoes one pixel, which is what trap 2 said and is still true |
| Coalesce on release, send nothing before it | The sharp edge, unanswered: the boundary is crossed with nothing on screen changing, and the refusal arrives — if at all — as a surprise on release. Also gives up the live diff, which is the thing §9 asks for |
| Coalesce on release, and check §4.4 in TypeScript to keep the boundary visible | A second implementation of the validator in a third language, which is ADR 0012 §2's rejected shape one language over. Its failure mode is a UI that refuses what core accepts, or accepts what core refuses, with nothing comparing them |
| Clamp the drag to legal positions | Both of the above at once: it is §4.4 reimplemented, *and* it makes the boundary silent — a note that stops moving with no reason given. "Silently legal" is the outcome the plan named |
| Throttle the dry runs to a fixed interval | An interval is a number nobody can choose correctly: too long and the diff lags the pointer, too short and it queues. "One in flight, newest wins" adapts to whatever the call actually costs and has no number in it |
| Commit on release, undo to correct | Loses §9's diff for the first control that could have one, and makes every drag an entry even when the person was only looking. Kept as a `ponytail:` deferral in decision 4 rather than refused outright |

## Consequences

- **`app/` gains one gesture and no gesture framework.** Dragging a note in the roll, and
  nothing else — no marquee, no multi-select, no resize handle, no clip drag in the timeline.
  Each of those is a second hit region with its own arithmetic, and what this pull request had to
  decide is answered as well by one gesture as by six.
- **Every gesture M2 and M3 add follows decisions 1–3**: propose with dry runs, commit once,
  never re-implement a validator rule to predict a refusal. A gesture that cannot be expressed as
  repeated dry runs of one tool call is a gesture that needs its own ADR.
- **`app/src/time.ts` gains `unscale`**, the exact inverse of `scale`, beside it. PR 4 left the
  inverse out on purpose and named that file as where it would go; two `width / ticks`
  conversions in two files is the rounding-rule drift its header already argues against.
- **The frontend holds three numbers per gesture** — a note id, a tick and a pitch. That is not
  song state and not a copy of any: the note being moved is still the model's, and the document
  does not change until it changes (ADR 0012 §2).
- **Trap 8 is now measured rather than predicted.** `Project::write`'s O(history) cost is 30 ms
  at 300 entries against 17 ms at 21, on a small project. Decision 1 keeps a drag from
  multiplying it, and does not fix it; the known gap stays open with a number attached.
- **No golden moves for any of this.** The projection golden is unaffected because no projection
  changed — the drag is drawn from the projection plus a pointer offset, and `pianoroll.ts` and
  `arrangement.ts` are untouched.
