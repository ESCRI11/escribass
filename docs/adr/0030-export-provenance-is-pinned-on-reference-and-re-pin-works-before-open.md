# ADR 0030 — Export provenance is pinned on reference, and re-pin works before open

- **Status:** Accepted (2026-10-22)
- **Affects:** `core/src/project.rs` (`Lock`, M5 PR 8; the recovery operation, M5 PR 9); a new
  `escribass-repin` binary and the window's recovery view (M5 PR 9); `tests/determinism/` (the
  first `lock.json` with an `artefacts` block, M5 PR 8); ADR 0010 §2 and §3 and ADR 0027 §3,
  amended in place; ADR 0003 §3, noted
- **Builds on:** ADR 0010 §1 (`lock.json` is not a proto and holds no paths or parameter lists),
  §2 (a pin is written on first reference by `Project::write` and never removed) and §3 (a
  mismatch refuses to open, and re-pinning is explicit); ADR 0004 §3 (the ref flip is the commit
  point); ADR 0012 §3 and ADR 0020 §5 (the kernel's lock on `.escri/lock`); ADR 0019 (a proposal
  is applied as one patch); ADR 0021 §4 (a block absent until used is absent from the file); ADR
  0027 §1 (a pin is a fact the child stated, written once, on a compile somebody kept), §2
  (checked at compile, never at open) and §3 (`artefacts` and `models` reserved, keyed by content
  hash); ADR 0028 (the export and conversion records); ADR 0029 §4 (`recipe_mismatch`)
- **Recorded in:** `docs/specs.md` §10, §11 and §15.

## Context

§11's third bullet requires every external tool version to be recorded in `lock.json`. ADR 0027
§3 reserved two keys for what an export adds, **`artefacts`** and **`models`**, each a map keyed
by content hash, and left their fields to the ADR that designed the record. ADR 0028 has done
that: an export record names its input, its target, its recipe, its members, its class, its
ports and its parameters, and a conversion record does the same for a converted model. This ADR
says what of that reaches `lock.json`, when, and what is checked against it.

It also answers a question ADR 0010 §3 deferred twice. A project whose referenced plugin pin
disagrees with this build refuses to open, `lock_mismatch`, and "re-pinning is always
explicit". M1 shipped no tool for it — the operator edits the text — and the reason given was
that a UI showing what changes did not exist yet. M5 is the first milestone to change a bundled
plugin's pin under existing projects: Airwindows is re-pinned, and the plugin manifest has not
been rebuilt since 2026-09-09. And the place a person meets the refusal is a window that cannot
open the project at all, so a recovery reachable only from an open `Session` is unreachable in
exactly the case it exists for.

## Decisions

### 1. `artefacts[export_hash]` records what made the export, and nothing it does

```json
{
  "schema_version": 1,
  "engine": { … },
  "plugins": { … },
  "artefacts": {
    "<export_hash>": {
      "recipe": { … },
      "target": "vst3/x86_64-linux"
    }
  },
  "models": {
    "<conversion_hash>": { … }
  }
}
```

**An entry is the record's `target` and `recipe`, copied once, keyed by the record's own hash.**
`recipe`'s fields are ADR 0031's — the Cmajor, CLAP SDK, clap-wrapper and VST3 SDK commits, the
compiler and its flags, Cmajor's optimisation level and the recipe's own edits to generated
code. The copy is what §11 asks for, a project's tool versions readable in `lock.json` without
opening an asset. It cannot drift from the record, which is immutable and whose hash is the
key, so an entry can disagree with its record only if one of them was edited by hand (decision
4).

**Nothing else is in it**, as ADR 0010 §1 keeps paths and parameter lists out of `plugins`: no
path, because the loading cache is machine-specific (ADR 0028 §3); no parameter interface,
which is the record's and a second copy would be the one that stops agreeing; no member list,
for the same reason.

**No project-wide "current DSP toolchain".** `toolchains.generator` is one selector per
project, because a generator's output is notes and one DSL version describes them all (ADR 0027
§1). An export is a binary, and its identity already includes how it was made (ADR 0028 §2). So
two exports in one project may carry two recipes, and a recompile under a changed recipe is a
new record and a new entry — shown as a recipe change in the diff before Apply — that never
rewrites the old one's provenance.

### 2. `models[conversion_hash]` records a conversion, keyed by the conversion and not the model

An entry is the conversion record's recipe: what converted the graph, with what package
closure, through what adapter. Its fields are ADR 0032's, with PR 12. **The key is the conversion
record's hash, not `model_hash`**, so one graph converted under two toolchains survives on two
branches as two entries. A `model_hash → latest plugin` map would be a lookup rewritten on every
conversion, which is not a pin (ADR 0028 §5).

### 3. Written on first retained reference, on every path that applies one, and never removed

**`Project::write` already recomputes `plugins` on every write**: the pins that exist, plus an
entry for every referenced plugin that has none (ADR 0010 §2). It does the same for these two
blocks. Every `export_hash` the song references whose record is present gains an entry if it
has none, and every conversion that record reaches gains a `models` entry. Existing entries are
never rewritten and never dropped. Because the reference is what pins and the write is
common, **every path that can apply a reference pins it**:

- `compile_instrument`;
- a raw `apply_patch`;
- a proposal's Apply;
- `undo` and `redo`;
- `merge_branch`;
- the write that follows `switch_branch`.

A pin that waited for a particular tool would be missing after any of the others. **A reference to a record that is not present writes no
entry** and plays as `export_missing` (ADR 0028 §4): a pin is written from what a record says,
and there is nothing to copy.

**Never removed, for ADR 0010 §2's reason and one more.** Undo appends an inverse entry that
re-adds a reference, and a block recomputed from the current song would drop a pin on delete and
re-add it on undo. Here a re-added entry could only be the same entry — its key is a content hash
— so the hazard is smaller than for a plugin, and the rule is the same one sentence.

**Absent until used, and absent from the file when absent** (ADR 0021 §4's shape). No existing
golden's `lock.json` gains a block, and M5 PR 8's instrument and effect goldens are the first that
carry one.

**The order is the commit point's**, extended to assets as ADR 0028 §3 extends it:

1. member blobs;
2. the export record;
3. `lock.json`;
4. the patch entry;
5. `song.json`;
6. `refs.json`, whose rename is the commit (ADR 0004 §3).

`Project::write` already writes `lock.json` before the entry. A crash can therefore leave an
unreferenced blob, or a monotone entry nothing references yet, and **never a committed
reference to half a bundle**.

**A proposal's export is pinned when the proposal is applied, by the same write.** That differs
from a generator. ADR 0027 §1 accepted that a model's compile writes the generator's
`toolchain_version` and no project block. It stays exactly as it is, and for its own reason: a
selector written from a fork would be a pin no entry explains. An export pin is not a selector.
It is a fact about a record the applied patch references, and the common write computes it from
that reference. So no approved export ever has a pin waiting for a later manual compile.

### 4. Checked at compile and before a load, never at open, and never against an installed compiler

| When | What is compared | A disagreement is |
|---|---|---|
| **compile** | the recipe the child reports having run against the approved recipe `core` asked for (ADR 0029 §2) | `recipe_mismatch`, operator; nothing written |
| **before a device is loaded** — a preview or a render | the record against its hash, every member against its entry, the record's `target` against this machine's, and the `artefacts` entry, when present, against the record | `export_corrupt`, `export_missing`, `target_unsupported` or `lock_mismatch`, all operator |
| **open** | nothing in `artefacts` or `models` | — |

**Never at open, for ADR 0027 §2's reason.** A project with exported devices opens and renders on
a machine with no Cmajor, no CMake and no compiler. Comparing a recipe at open would refuse
exactly the project this milestone promises plays without them. A recipe is compared when a
compile is asked for, because that is when it matters and the only time it can be wrong. A
render loads a binary that was built, and its integrity is what is checked before it is handed
to the engine. The approved toolchain of the day is never the test, so an export made last year
under a recipe since retired still plays, and says what made it.

**`lock_mismatch` keeps its meaning.** It is the operator's refusal when a pin and the thing it
pins disagree. For a plugin the other side is the build (ADR 0010 §3). For an export it is the
record, and only a hand edit can make them disagree.

### 5. Re-pin is one `core` operation, reachable before open, applied as a reviewed patch to `lock.json`

**Deliberately superseding ADR 0010 §2's "by no tool" and §3's "re-pinning is the operator
editing `lock.json`".** That was right while nothing changed a bundled plugin's pin under
existing projects, and while no window could show what a re-pin changes. Both now hold. A hand
edit is also the one re-pin path with no preview and no check against what the file said when
the person read it.

**The operation**, `core::repin`, works on a project that cannot be opened. It takes the
project's kernel lock (ADR 0012 §3; ADR 0020 §5) without building a `Session` and reads
`lock.json` and the build manifest. Then:

- **A dry run** answers, for each pin the song references that disagrees with this build, the
  old and the new: `commit` and `version`, per plugin id. It also returns the RFC 6902 patch over
  `lock.json` that would make them agree — `replace` on exactly those fields and nothing else —
  and the SHA-256 of `lock.json` as it was read. It writes nothing.
- **Apply** takes that patch and that hash back. It refuses, `lock_moved`, if `lock.json` no
  longer hashes to what was read. Otherwise it applies the patch, writes the file atomically
  (sibling and rename), releases the lock, and answers with the same old-and-new receipt. The
  next `open` then runs every normal check again.

**What it refuses**, each a structured error and never a fallback:

- a plugin the build does not have — a pin cannot be moved to nothing;
- a change of identity, because a re-pin moves a plugin's commit and version and never which
  plugin it is;
- anything in `artefacts` or `models`, which are content-addressed and have nothing to move to;
- an export failing its integrity checks, which a re-pin cannot repair;
- a moved lock.

**There is no "open anyway" and no "ignore all mismatches".** Either would solve the refusal by
deleting the guarantee ADR 0010 §3 exists for.

**No song-log entry, and the reason is ADR 0010 §1's.** `lock.json` is not song state; the log
records changes to the song (CLAUDE.md #1, #2). The audit of a re-pin is its receipt: printed by
the CLI, shown by the window, and visible in the repository's own history of `lock.json` when
the project is committed. A second journal of lock edits would be a second history.

**One operation, two callers, no model.** The window, meeting `lock_mismatch`, shows a recovery
view **before** normal open: the old and new pins and Apply. The CLI, `escribass-repin`, prints
the same receipt and applies only with `--apply`. Both call the one `core` function. It is not a
`SongTools` RPC, because it runs where no session exists. It is never offered to the model, which
has nothing to review it against. The engine block keeps re-pinning on open (ADR 0010 §3); this
is about the pins that refuse.

### 6. ADR 0003 §3's "completed at M5" is not closed here

The blocks are decided. They are not implemented. ADR 0003 §3 says `lock.json` is completed at M5.
That sentence becomes true when PR 8 writes the first `artefacts` entry and PR 12 the first
`models` entry, and M5's close records it then. A decision that says a file is complete before
any code writes the new blocks would be the sentence that stops being true by being early.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Put nothing in `lock.json`; the record is the pin | §11 requires the tool versions in `lock.json`, and an asset is not where a person or a reviewer reads them |
| Copy the whole record into the entry | A second copy of the interface and the members — the copy ADR 0010 §1 refuses for parameter lists |
| One project-wide DSP toolchain, as `toolchains.generator` | Two exports on two branches made by two recipes are both valid; one selector would make one of them wrong |
| Key `models` by `model_hash` | One graph converted twice keeps one entry, rewritten, which is not a pin |
| Pin only in `compile_instrument` | A raw patch, a proposal's Apply, undo, redo, merge or a branch switch would leave a referenced export unpinned |
| Compare recipes at open | Refuses to open a project that renders without any compiler, which is the milestone's proof point |
| Leave re-pin as editing the text | No preview, no check that the file is what the person read, and unreachable from a window that cannot open the project |
| A re-pin `SongTools` RPC | It needs a session, and the case it exists for is a project no session can open |
| An "ignore mismatches" open flag | Solves the refusal by deleting the guarantee |
| Record re-pins in the song log | `lock.json` is not song state, and its own history is where its changes live |

## Consequences

- **M5 PR 8**: `Lock` gains `artefacts` and `models`, both absent until used, written by
  `Project::write`. The pre-load checks and `target_unsupported` arrive. The instrument and effect
  goldens are the first `lock.json`s with the block. No existing golden's `lock.json` moves.
- **M5 PR 9** (`m5.9-repin`): `core::repin`, `escribass-repin` and the window's recovery view,
  exercised on a real mismatched project. The test pins a fixture to a deliberately changed
  **existing** commit (trap 18), with a concurrent change and a cancelled apply under the kernel
  lock, and with a plugin that is missing or replaced.
- **M5 PR 12**: the first `models` entry, with ADR 0032's fields.
- ADR 0010 §2 and §3 are amended in place, dated: the no-tool rule is superseded deliberately,
  and why. ADR 0027 §3 is amended: the two blocks have their shapes. ADR 0003 §3 gains a dated
  line saying when it closes. ADR 0022 §5 and ADR 0023 §6 placed the re-pin tool, and their row
  now reads M5 PR 9.
- `docs/specs.md` §10's `lock.json` description and §11's third bullet say what is recorded and
  when it is checked, and §15 gains the rows.
- **What this rests on that is unmeasured**: that a project carrying exports opens and renders
  on a machine with no compiler at all — PR 8's proof point — and that the recovery view is
  reachable from the refusal a person actually meets.
