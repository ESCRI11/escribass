# ADR 0001 — Song history is a patch DAG with named refs

- **Status:** Accepted (2026-09-02)
- **Affects:** `schema/song.proto` (§4), patch log (§5, §10), determinism suite (§11)
- **Supersedes:** nothing
- **Recorded in:** `docs/specs.md` §15; §5 and §10 amended to match.

## Context

§5 makes the patch log the undo/redo history and the audit trail. §10 stores it as an
append-only log under `patches/`. Today that log is a flat list: one linear history, one
present state.

We want to try an idea on a song — a darker chorus, a different bass line — keep it if it
works, discard it if it does not, and compare the two without copying the project
directory. That is branching.

A flat list cannot express it. But the log is already a sequence of validated,
content-bearing, provenance-carrying commits; it is a commit log that only lacks the two
fields that make a commit log a version control system.

This ADR is written before `schema/song.proto` because the decision constrains the schema.
Retrofitting it after M0.1 would be a breaking schema change.

## Decision

### 1. The patch log is a DAG, not a list

Every patch entry gains `parents`: the ULIDs of the entries it was applied to — empty at the
root, one normally, two for a merge (a repeated field rather than a scalar, so decision 4
below needs no second shape). The log stays append-only (§10 unchanged); entries are never rewritten or deleted.

```
patches/01K4F31M2T.json
{
  "id":     "01K4F31M2T",
  "parents": ["01K4F2QN8B"],
  "tool":   "set_notes",
  "ops":    [ { "op": "replace", "path": "/clips/…/notes/…/pitch", "value": 43 } ],
  "provenance": { "author": "model", "model_id": "…", "tool_call_id": "…", "created_at": "…" }
}
```

A merge entry (§4) carries two entries in `parents`; nothing else about the format changes.

### 2. Branches are named refs in a single `refs.json`

```json
{
  "head": "main",
  "refs": {
    "main":              "01K4F31M2T",
    "try-darker-chorus": "01K4F2Z10Q"
  }
}
```

- **Branch** = add a ref at the current node. No data is copied.
- **Discard** = remove the ref. Orphaned entries stay on disk, inert, a few hundred bytes
  each. Garbage collection is not implemented and may never need to be.
- **Switch** = materialise the target node's canonical JSON, diff it against the current
  document, apply the resulting patch.

One file rather than one file per ref: `core` is the only writer (§3 embeds it in `app`,
and §5 routes every mutation through the tool API), so per-ref files buy no concurrency,
while ref names as filenames collide on case-insensitive filesystems — `refs/Main` against
`refs/main` is a well-known git bug class we get to skip.

**`HEAD` always names a ref.** There is no detached state. Inspecting an earlier version is
a read-only projection — `get_song_at(patch_id)` — not a checkout, which is both simpler to
implement and a better answer than git's for a music application. Editing from an old point
means naming a branch there first.

Ref names are ASCII `[a-z0-9._/-]`, no leading or trailing slash, no `..`, and two refs may
not differ only by case. The case rule is unnecessary given `refs.json`, but it costs one
line in the validator, keeps names safe if the store ever moves to files, and spares users
two branches they cannot tell apart.

### 3. Entity collections are id-keyed maps, not repeated fields [MUST]

This is the binding consequence and the reason the ADR precedes the proto.

RFC 6902 paths into an array are positional. If `notes` is a `repeated` field, a branch
that inserts a note at index 1 invalidates every `/notes/3/…` path on any sibling branch —
silently, and in a way that produces a valid patch applied to the wrong note. There is no
rebase algorithm that recovers from this, because the information needed is not in the
patch.

§5 requires RFC 6902 [MUST]. Branching requires stable paths. Together they force
id-keyed maps:

```proto
message Clip {
  string id           = 1;
  string track_id     = 2;
  int64  start_tick   = 3;
  int64  length_ticks = 4;
  map<string, Note> notes = 5;   // key == Note.id (ULID)
}
```

```json
"notes": {
  "01K4F2QN8B": { "id": "01K4F2QN8B", "pitch": 43, "start_tick": 1440 }
}
```

The path `/clips/…/note_clip/notes/01K4F2QN8B/pitch` is stable for the life of the note.

Applies to every collection of ULID-bearing entities: tracks, clips, notes, automation
points, effects, sections, generators.

Corollaries:

- **Musical order is derived, never stored.** Notes order by `start_tick`, sections by
  `start_tick`, effects by an explicit `index` field on the chain. A collection's storage
  position carries no meaning. This is correct independently of branching.
- **Canonical JSON key order is lexical by ULID.** ULIDs sort lexicographically in
  creation order, so §4.1's stable key order requirement is satisfied by sorting keys, and
  the result is also human-legible.
- **The key duplicates `id`.** §4.3 requires `id` on every entity, so both are kept and the
  validator (§4.4) enforces `key == value.id`.
- **Corrected 2026-09-02:** an earlier version of this ADR exempted sequences "whose
  elements have no identity", naming `TempoMap.events`. That was the wrong criterion. The
  hazard is array-index instability, not identity: a branch inserting a ritardando early
  shifts every later index, so a sibling branch's edit to a later event has a disjoint path,
  auto-merges under decision 4, and lands on the wrong event — silently. `TempoMap.events`
  and `TimeSignatureMap.events` are keyed like everything else, with an `id` and no
  provenance (ADR 0002 §2). The only `repeated` fields left in the schema are
  `PatchEntry.parents` and lists that are genuinely positional and never patched
  element-wise.

### 4. Merge auto-resolves disjoint paths; conflicts are structured errors

`merge_branch` compares op paths from both branches against their common ancestor.
Disjoint paths merge automatically. The same path touched on both sides is a conflict,
returned as a structured error naming the path and both values — never auto-resolved by
heuristic. The result is an ordinary patch entry with two parents and its own provenance.

With `dry_run=true` the tool returns the full conflict list without applying, which is the
natural shape for §5's dry-run contract and gives the AI orchestrator (§6) something it can
act on and retry against.

**`version` is not merged.** §4.3's per-entity `version` is maintained by `core`, never
written by a tool-authored op, and resolved on merge as `max(a, b) + 1`. Without this rule
every merge conflicts by construction: two edits to any one entity both write `replace
/…/version` at the same path, and so would every pair of edits to a song via `Song.version`.

Interactive conflict *resolution* — choosing side A or side B per path, editing a merged
result — is deferred until there is a UI to host it (M2 at the earliest). Designing that
API now would mean designing it against conflicts nobody has hit yet.

### 5. Entity ids come from an injectable source

§4.3 requires ULIDs. A ULID is a 48-bit millisecond timestamp plus 80 random bits, and
`core` mints them — which is unseeded randomness and wall-clock dependence inside `core`,
both forbidden by §11.

`core` therefore takes an id source as a constructor parameter: a real ULID generator in
production, a seeded deterministic generator in tests. §14.4 already requires exactly this
of any new source of randomness.

The alternative — real ULIDs everywhere, with the determinism suite normalising ids away
before comparing — was rejected. It leaves the §11 violation in place and only hides it,
the normaliser cannot catch an id that leaks into a *value* rather than a key, and it
downgrades "same input → identical canonical JSON" to "identical after rewriting", which is
weaker than the claim §18.2 asks us to publish render hashes behind.

### 6. Git is an interop format, not the storage engine

No `libgit2`, no `gix`, no new dependency in `lock.baseline.json`.

§2.6 already requires text-first, git-friendly persistence, so `git init` inside a project
directory works today and is the right answer for sharing a song on GitHub. Internal
branching is model-aware; external git is the escape hatch. Same relationship the model has
with DAWproject.

## Alternatives considered

**Storage engine**

| Alternative | Rejected because |
|---|---|
| Embed git (libgit2 / gix) as the store | Creates a second representation of history alongside `patches/`, violating §14.2. Git's line-based three-way merge on `song.json` conflicts spuriously on unrelated edits and can merge into a syntactically valid, musically wrong song. Binary assets need LFS; they are already sha256-addressed in `assets/` and belong outside the DAG. |
| Keep the flat log; copy the project directory to experiment | No shared ancestry, so no comparison and no merge. Assets duplicated. This is what users do today in every DAW and it is the problem. |
| `repeated` fields plus a non-RFC path syntax (`/notes[id=X]`) | §5 specifies RFC 6902 [MUST]. Diverging from it costs every off-the-shelf patch library in four languages. |
| `repeated` fields, patches rewrite whole arrays | Diffs become unreadable and every concurrent edit to a clip conflicts. Defeats the purpose of §5. |
| Snapshot `song.json` per save point | Storage grows with project size rather than edit count, and there is no op-level provenance to merge on. |

**Sub-decisions**

| Question | Rejected option | Rejected because |
|---|---|---|
| Id generation | Content-derived ids (hash of entity + parent) | Contradicts §4.3's ULID requirement; identical notes collide without salting; "never reused" breaks under undo then redo. Would need its own ADR. |
| Id generation | Zeroed clock plus per-project counter | Two branches mint the same counter and collide on merge; not globally unique across projects, so clips cannot be copied between them. |
| Merge scope | Defer merge entirely past M0 | Branches you cannot merge are half the feature, and an MCP client (§18.2) cannot complete the loop. |
| Merge scope | Full interactive resolution in M0.3 | Largest M0 scope add, designed blind against conflicts nobody has hit. |
| Refs layout | One file per ref under `refs/` | Ref names must encode to safe filenames and collide on case-insensitive filesystems; the concurrency it buys is worthless with a single writer. |
| `HEAD` | Allow detached `HEAD` | An extra mode every panel and every tool must handle, to solve a problem a read-only projection already solves. |

## Consequences

**Schema (M0.1).** Collections become maps. Every entity keeps its ULID. Order fields
(`start_tick`, effect chain `index`) become load-bearing rather than incidental.

**Core (M0.2).** Patch entries gain `parents`. A `refs.json` with `head` and `refs`. An
injectable id source on the session constructor.

Materialise-and-diff is the single mechanism behind both undo and branch switching: replay
from the root to the target node, diff against the current document, apply. It needs no
inverse ops stored in the log, no lowest-common-ancestor search, and it stays correct
across merge nodes where a naive inverse-walk would not.

> `ponytail:` replay is O(history) per switch. Fine for a session's worth of edits; cache
> the last materialised document per ref if a large project drags, and add periodic
> snapshots only if caching is not enough.

**Tool API (M0.3).** Four additions: `create_branch`, `switch_branch`, `delete_branch`,
`merge_branch`. Like every other tool they support `dry_run`.

**Determinism (§11).** Version-control metadata — patch ids, refs, `HEAD`, `created_at` —
must never reach the render engine. §8 already sends `engine` a materialised layer 2 +
layer 3 snapshot; that snapshot must contain no history. With the injectable id source of
§5 above, M0.4's determinism suite compares canonical JSON and patch logs byte for byte,
with no normalisation step.

**Strategy (§18).** Branching a song strengthens differentiator §18.1.3 and is absent from
every competitor in the §18 landscape, including the two on the watch list. It falls out of
the architecture already committed to rather than being bolted on.

## Deferred

- Interactive conflict resolution (§4 above) — waits for a UI, M2 at the earliest.
- Garbage collection of orphaned patch entries. They are small and inert; revisit only if a
  real project's `patches/` becomes a problem.
- Remotes, or any notion of pushing a branch anywhere. External git covers sharing.
