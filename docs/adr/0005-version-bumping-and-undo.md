# ADR 0005 — Core bumps entity `version` inside the commit pipeline, and undo appends an inverse entry

- **Status:** Accepted (2026-09-03)
- **Affects:** the tool API (M0.3); `core/src/project.rs`; `docs/specs.md` §5; ADR 0001 §4
- **Builds on:** ADR 0001 §4 (core maintains `version`, merge resolves it as max+1), ADR 0004
  (the log is authoritative, `song.json` is derived)
- **Recorded in:** `docs/specs.md` §5 and §15.

## Context

Two items have sat in `docs/plan.md`'s deferred ledger since M0.2, both pointing at M0.3, and
both for the same reason: they are decisions about the *mutation pipeline*, and until M0.3
there was no pipeline and no caller to design against.

**Entity `version`.** ADR 0001 §4 says core maintains it and no tool op ever writes it. That
is half an answer. The other half is *when*, and it is not free: `Project::commit` records
`diff(before, after)` — the effect re-derived from the round-tripped `Song`, not the caller's
ops — so anything core changes outside that diff is invisible to the log. A bump applied
after the diff is computed would live in `song.json` and nowhere else, and every replay would
produce a document one version behind the file beside it. ADR 0004's whole guarantee is that
this cannot happen.

**Undo.** §5 calls the patch log "the undo/redo history" and ADR 0001 names materialise-and-
diff as the mechanism, but neither chooses between the two ways to spend it: move the branch
ref backwards, or append an entry that reverses the last one. The choice is not cosmetic —
one of them makes §4.3's `version` field lie.

They are decided together because the second constrains the first. The version rule below was
chosen so that undo stays monotonic; a different rule would have forced a different undo.

## Decisions

### 1. The bump happens between applying the ops and re-deserialising the document

`Project::commit`'s existing order is: serialise `self.song` to `before`, apply the caller's
ops, re-deserialise through `Song`, validate, re-serialise to `after`, record
`diff(before, after)`. The bump is inserted as one pure step on the `Value`:

```
let mut patched = apply(&before, ops)?;
bump_versions(&before, &mut patched);      // this decision
let song: Song = serde_json::from_value(patched)?;
```

That single placement satisfies both halves of ADR 0001 §4. The bump is core's, computed from
the document rather than requested by anyone, so no tool op carries it — and it happens
*before* `diff(before, after)` is taken, so the resulting `replace /…/version` operations are
in the entry that gets written. Replay converges. The property established for value
spellings — the log can never say something `song.json` does not — extends to versions with
no further machinery.

It operates on `serde_json::Value`, not on `Song`. The convention below is one rule about a
shape; expressing it over the typed tree would mean ten near-identical implementations that
drift as entities are added. This is not the pattern `core/AGENTS.md` prohibits: that rule
forbids *serialising output* through `Value`, because `serde_json::Map` alphabetises struct
fields. Nothing here serialises. `commit` already round-trips through `Value` to apply a
patch at all.

### 2. An entity is an object with a string `id` and a numeric `version`; a changed entity bumps to max+1

`bump_versions(before, patched)` walks the patched document. For every object carrying both a
string `id` member and a numeric `version` member, if any leaf beneath it other than its own
`version` differs from `before`, or the object is new, set

```
version = max(before.version, patched.version) + 1
```

Otherwise restore the value from `before`. Everything else follows without a special case:

- **Ancestors bump.** Editing a note bumps the note, its clip, and `Song` — which is what
  §4.3's optimistic concurrency is for at every level.
- **New entities land at 1.** Tools construct them with `version: 0`.
- **Merge needs no separate rule.** On a merge the right side's ops carry `R` and `before`
  holds `L`, so `max(L, R) + 1` is exactly ADR 0001 §4's stated resolution. The rule that
  makes ordinary commits correct makes merges correct too.
- **`PluginRef.version` is untouched.** It is a `string` (`schema/song.proto`), so it fails
  the numeric test. Keying on the field *name* alone would break `set_track_instrument`,
  which exists to pin a plugin version.
- **`AutomationPoint`, `TempoEvent` and `TimeSignatureEvent` are skipped by construction.**
  ADR 0002 §2 gives them no `version` field, so they never match.

### 3. A tool-authored op that writes `version` is refused, with rule `version_not_writable`

The typed tools never emit one. The raw `apply_patch` tool — which M0.4 needs in order to
drive core through the tool API at all (`CLAUDE.md` #2) — could, so it is guarded.

**Revised 2026-09-03, when the guard met its first caller.** The original text said the check
was on the operation: "any operation whose final reference token is `version`, on an object
matching decision 2, is rejected before the patch is applied". Inspecting paths cannot work.
It does not see a version arriving inside a whole-entity value, it cannot tell `"1"` from `1`
— the proto3 JSON leniency that made the log disagree with `song.json` in M0.2 — and it
refuses this API's own output, because a patch `ToolResult` returns carries the bumps it
caused (ADR 0006 §1). §9 has a person approve a diff and then apply it, so a guard that
refuses every patch the API produces leaves that flow with no working path.

The guard is therefore a **comparison, not a filter**: decision 2 computes the number, and
where the caller ended up asking for a different one, that is reported as
`version_not_writable`. It catches every route in, and it lets a previewed patch be applied
unchanged, because a patch that states what core computes is disputing nothing.

**The two modes are separate rules, not one rule with a shortcut.** An ordinary edit bumps by
one and ignores whatever number arrived in the value; a merge resolves to `max(ours, theirs) +
1` and disputes nothing, because those numbers are core's own arriving from the other branch.
Written as a single rule that preferred the ordinary answer whenever the caller had already
stated it, the two collapsed exactly when the other branch was one ahead: `max(L, L+1) + 1` is
`L+2`, and the shortcut returned `L+1` — a number the other branch had already handed out for
different content, which is the repeat §4.3's concurrency check cannot survive. Two short
rules beat one clever one.

Two exemptions fall out rather than being written. An entity that did not exist before has no
number a client could be holding, so a version arriving with a new entity is accepted.  And an
entity whose `id` changed in place is a *different* entity — `set_track_instrument` replaces
one wholesale — so the number the old one had is not a claim about the new one.

### 4. Undo appends an inverse entry; it never rewinds a ref

`undo` computes `diff(current, materialise(parents[0]))` and commits it through the ordinary
pipeline under the tool name `undo`. `redo` is the same operation against the entry that was
undone. The session holds the stack of undone entry ids, and any other commit clears it —
the model every editor already implements.

**A caveat this decision only argues for one branch.** `version` counts per branch, so two
branches can hold the same number over different content — switching between them is not a
rewind, and no operation lies, but a client checking a version has to know which ref it is on.
The merge rule (`max + 1`) is what makes the number monotonic again once the branches join. A
concurrency check that spans branches would need the ref alongside the number; nothing needs
one before M2.

Rewinding the branch ref was the cheaper option and is rejected because it breaks §4.3. A
rewind *decrements* every version it touches, so a client holding version 5 of a track can
later be handed a different version 5. Optimistic concurrency built on a number that repeats
is not concurrency control; it is a race with a comment. Appending an inverse leaves versions
monotonic — by decision 2, the inverse diff writes the old value and the rule takes it to
`before + 1` — and it keeps the audit trail §5 asks for, in which an undo is a thing that
happened rather than a thing that unhappened.

The `undo` and `redo` **tools** are deferred to M2, where ⌘Z gives them a consumer
(`docs/plan.md`). This ADR is not deferred with them: decision 2 was chosen to make decision 4
monotonic, and a later reader needs to know that the two are load-bearing on each other.

**Extended 2026-09-09, when the tools landed in M2 PR 5.** "The session holds the stack of undone
entry ids" was right and half specified, and the half it left out is what that stack is walked
*through*. It is a cursor into the log's **first-parent chain**, and the walk must **skip the
log's own `undo` and `redo` entries**, because ⌘Z means "the change before this one" and not "the
entry before this one". Two failures follow from getting it wrong, and both were found by
pressing the key rather than by reading the log:

- Reading `HEAD` again instead of the cursor finds the undo entry the previous call appended,
  and reversing that is a redo.
- After edit · undo · redo the log ends with a redo whose parent is an undo. Walking those two
  as though they were edits takes the document **forwards**, because the inverse of an inverse
  is the thing itself. `core/tests/undo.rs` fails on exactly that sequence when the skip is
  removed.

The cursor is cleared by any other commit — in the one place a session appends — and by a branch
switch, which appends nothing and still moves the document: `version` counts per branch, so a
redo across a switch would restore a document from another line of history. It is session state,
so a process that has just opened the project has undone nothing and its first `undo` reverses
whatever the log ends with; the log records entries, not key presses, and there is nothing
honest to recover.

Both tools go through `Project::prepare_merge` rather than `prepare`, and they are the second
caller that needs it: their ops are `diff(current, materialise(…))`, so the versions in them were
read back out of a document core wrote, and decision 3's guard would otherwise refuse this API's
own history. `max(ours, theirs) + 1` then takes every restored entity to one *past* where it is
now, which is decision 2 doing exactly what decision 4 was written to need.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Bump after `diff(before, after)`, writing only to `song.json` | The log would replay to a document one version behind the file beside it, and ADR 0004's `open` would refuse a project that had committed cleanly. This is the defect `92d6c74` fixed for value spellings, re-introduced by a different route. |
| Let tools declare which entities they touched | Every tool then carries a second, hand-maintained description of its own effect, and the two drift silently the first time a tool's ops change. The diff already knows. |
| Bump only the entity a tool names, not its ancestors | `Song.version` would never move, so nothing could detect a concurrent edit to the song as a whole, which is §4.3's stated purpose. |
| Version as a content hash instead of a counter | Removes the bump entirely, and removes ordering with it: "is this newer" becomes unanswerable, and merge loses ADR 0001 §4's max+1 rule. |
| Undo by rewinding the branch ref | Decrements versions, so §4.3's field can repeat a value with different content. Redo also needs a session-held stack of orphaned ids anyway, so it is not simpler where it matters. |
| Undo by rewriting or deleting the last entry | The log is append-only (ADR 0001 §1) and it is the audit trail (§5). An audit trail that can be edited is not one. |

## Consequences

- **M0.3 PR 3** adds `bump_versions` and splits `commit` into a pure `prepare` and a
  side-effecting `record`, which is also what `dry_run` needs (ADR 0006).
- **Merge (M0.3 PR 9)** excludes `/…/version` leaves from conflict detection and lets decision
  2 resolve them. Without that exclusion every merge would conflict by construction on
  `version`, which is the failure ADR 0001 §4 anticipated.
- **§5** gains one sentence naming undo as an appended entry rather than a rewind.
- **The determinism suite (M0.4)** is unaffected in form: version bumps are a deterministic
  function of the two documents, so two identical scripted sessions still produce identical
  bytes.
- **No schema change**, so no `buf breaking` implication. `version` already exists on every
  entity (ADR 0002 §2).
- **The `docs/plan.md` deferred rows** for entity `version` bumping and undo/redo policy are
  closed by this ADR; the undo/redo *tools* move to M2.
