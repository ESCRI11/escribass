# ADR 0012 — The webview calls the tool API through the host, and holds one decoded `Song`

- **Status:** Accepted (2026-09-07)
- **Affects:** `app/` (new, M2); `core/src/mcp.rs`; `core/src/project.rs`; `core/src/session.rs`;
  `docs/specs.md` §3, §5, §9 and §11
- **Builds on:** ADR 0006 §1 (one contract, not sixteen copies of it), §5 (one project per
  process), §6 (the MCP surface, and its two byte-level rules); ADR 0004 (the three-rename
  commit); ADR 0005 §4 (undo appends an inverse entry); ADR 0001 §5 (ids come from an
  injectable source)
- **Recorded in:** `docs/specs.md` §3, §5, §9, §11 and §15.

## Context

§3 puts `core` *inside* the Tauri host: "Rust library … `app` (embedded in Tauri host)". §5
says the tool API is "used by the UI and the AI identically". §14.2 forbids a second
representation of song state. All three are about the same boundary and none of them says what
the **webview** calls, what it holds between calls, or what stops the store it holds from
becoming the second representation.

M2's plan asked that at four places — questions 1, 2, 9 and 10 — and they are one question
asked of the carrier, the cache, the pending edit and the project handle. Answering them apart
would answer them inconsistently.

The thing that decides all four is already in the repository and is easy to miss: **there is
exactly one dispatch from a tool name to a session call, and it is in `core/src/mcp.rs`.** The
gRPC service is generated from the descriptor and calls `Session` directly; the MCP server owns
a hand-written `match` on the tool name because JSON-RPC hands it a name and a JSON object.
A third carrier can either write that `match` again — which is ADR 0006 §1's sixteen copies,
one milestone later — or call the one that exists.

## Decisions

### 1. One process. The webview calls one Tauri command, dispatched through the MCP path

`app` is a single OS process: the Tauri host, `core` linked into it, and a webview. The
webview's only route to the model is one command —

```
invoke("tool", { name: "set_notes", args: { … } })  ->  { … }
```

— and the host implements it by calling the same function the MCP server calls. `core/src/mcp.rs`'s
`call_tool` body is lifted into a carrier-independent `call(session, name, args) -> Result<Value>`;
the MCP server becomes the JSON-RPC envelope around it and the Tauri command becomes the second
envelope. **No second dispatch is written.**

That is what makes §5's "used by the UI and the AI identically" true rather than aspirational.
The UI is not on a path free to drift from the tested one; it is on the **MCP** path, which
M0.4's determinism suite already drives through a real process and compares against gRPC byte
for byte, and for which ADR 0006 §6 already pinned the two rules that bite — `patch` crosses as
a JSON array and never base64, and `get_song` renders through `to_canonical_json` and never
`serde_json::to_value`. A frontend that got either wrong would be reading a different document
from the one on disk, and both are already someone else's test.

The command surface stays one command rather than one per tool for the same reason. Twenty
`#[tauri::command]` functions are twenty places for a field to be forgotten; the argument names
are the request message's, and the request messages are generated (PR 3 adds TypeScript for
`proto/`), so the frontend builds a typed request and hands it over as JSON. ADR 0006 §6's
existing test — every RPC has a tool, every request field has a schema property — becomes the
check on the UI's surface too, at no cost, because there is one table for it to check.

`app` opens **no network port**. A desktop application that binds a socket to talk to itself is
a listening service on a user's machine; §3's gRPC is `app` ↔ `ai` and `app` ↔ `engine`, both of
them processes `app` supervises, and neither is the webview.

### 2. The frontend holds one decoded `Song`, re-read after every applied call

One `Song`, decoded from `get_song` through the generated TypeScript types (§4.1 forbids
hand-written ones). Every view is a **pure selector** over it. There is no per-view store, no
normalised entity cache, no client-side mutation. After a call that applied, the frontend
re-reads `get_song` and replaces the object wholesale; nothing merges.

The rejected shortcut is the interesting one. `ToolResult.patch` is right there, and applying it
locally would avoid the re-read — at the cost of a **second RFC 6902 apply, in a third
language**, whose failure mode is a view and a `song.json` that disagree with nothing comparing
them. M0.4's cross-language replay proves TypeScript *can* apply the log; ADR 0006 §3's reason
for `dry_run` being the first half of the apply path rather than a second implementation of it
is precisely why it should not.

The cost is a canonical round trip per edit, in-process, with no network and no proxy. If that
is ever measurably too slow, **the escape is a narrower read, not a second apply** — a
`get_song` that returns a subtree keeps one apply implementation and one document; a local
patch does not. Writing the escape down now is the point: the performance fix that arrives in
PR 7 without a design decision is trap 1, and the store it introduces is the second
representation CLAUDE.md #1 forbids.

The decoded `Song` is frozen at the boundary in development builds, so a view that writes to the
model throws where it wrote instead of drifting until someone notices a mixer showing a gain the
model does not have.

### 3. A `Session` per project, built by the host as a library; the `.escri` directory gets a lock

ADR 0006 §5 made one project per process **structural**: named at launch, no `open_project`
tool. A desktop application has File · Open and Recent, and §18.2 actively sells leaving an MCP
client pointed at the same directory. Two of the three options in front of it are worse than the
rule they would replace: an `open_project` *tool* is the wire change that decision refused, and
a process per window makes `app` re-exec itself to open a second document.

**Amending ADR 0006 §5:** the rule is one *session* per project, not one project per process,
and the tool API surface is unchanged — which is what §5 was protecting. The host constructs a
`Session` as a library call, exactly as `--create` was never a tool, and keeps one session per
project directory, so two windows on one project share one session rather than racing each
other inside one address space.

The cross-process race is what is left, and it is the known gap: nothing locks the directory,
and ADR 0004's commit is three renames under a single-writer assumption. `app` is the first
thing that makes two writers ordinary rather than hypothetical, so M2 closes it. On opening a
project, `core` creates `.escri/lock` with `create_new` — one `O_EXCL`, no dependency — holding
the pid, and removes it on a clean close. A lock that already exists is reported as an operator
error naming the pid; **it is never broken automatically**, because a lock file whose owner may
still be alive is not something a program can adjudicate, and git's `index.lock` has taught a
generation of users what to do with the message. A crash therefore leaves a project that says
why it will not open, which is the failure this is chosen for over the alternative: two
interleaved renames and a patch log that no longer matches the `song.json` beside it.

`ponytail:` advisory, and it assumes a local filesystem. A network filesystem where `O_EXCL` is
not atomic gets no protection from this; nothing in v1 is expected to run a project over NFS,
and a real lock protocol is worth writing when one does.

**Amended 2026-09-21, in the M3 ADRs, after the spike measured what "never broken
automatically" cost.** Every Claude Code session — 50 of 50 — left `.escri/lock` behind naming
a process that no longer existed, because a signal is how that client ends its MCP server and a
`Drop` does not run in a process a signal ended; the next open was refused until a person deleted
the file. The premise above holds and the rule was wider than it: the file *records its holder's
pid*, and a pid that names no process is something a program can adjudicate. From ADR 0020 §5 a
lock whose recorded holder no longer exists is **stale**: `take` replaces it and says so in its
result, a lock naming a live pid is refused exactly as before, and one with no pid is refused
because there is nothing to check. Nothing is broken while its holder may be alive; that is the
sentence that stands. Landed in M3 PR 4.

### 4. A held dry run is applied optimistically, and the frontend mints no ids

The wireframes draw an unapplied edit dashed in the timeline and a pending row at the head of
the patch log. Between the dry run and the apply the model can move: an agent edit, a branch
switch, a second window.

The UI **applies, and lets §4.3's `version` check refuse**. It does not re-run the dry run and
diff the two, and it does not hold the model still while an edit is pending. Both alternatives
add a rule; the optimistic one adds none, because the check already exists and already has the
error a user reads. What M2 owes it is that the refusal names what moved rather than saying the
version was wrong.

The corollary closes trap 11. A pending edit needs a key, and the nearest value is an id — but
`core` mints ids from an injectable, seedable source (ADR 0001 §5), and anything the frontend
generates for itself is unseeded randomness one process away from the model, in a language
CLAUDE.md #3 does not name. It does not have to: **a dry run already returns real ids**, minted
from a fork so the preview burns none (M0.3 PR 11), and those are the keys. They are the ids the
apply will mint, and if the apply is refused they are discarded with the preview.

### 5. §11 gains a projection golden, and that is what "the determinism suite" means for a UI

§11 is a `[MUST]` and `app` appears in neither its first bullet nor CLAUDE.md #3, both of which
name `core`, compilers and `engine`. Answering "nothing changes" would be a sentence somebody
has to write into a `[MUST]` section, and it would make decision 2's failure mode permanently
invisible.

So: from a fixed `Song` — already committed, and built through the tool API, because there is no
other way to write a song in this repository (CLAUDE.md #2) — each view's projection function
returns a serialisable description of what it would draw, and that description is goldened. It
runs under `node:test` with `tsx`, both already pinned; it needs no browser, no display and no
driver.

**Amended 2026-09-08 in M2 PR 4, when the first views existed to point it at.** This named
`tests/determinism/every_tool/expected/song.json` — "which every tool has already touched" — and
that is the wrong half of what a *view* golden needs. Every tool having touched a document says
nothing about whether the document has anything for a view to get wrong, and `every_tool`'s has
almost nothing: one clip, two notes of equal length on one track, and its tempo change at tick
3840, which is the last tick of the song. A projection that ignored the tempo map, the loop
field, the audio content case, the second track and every ordering question would golden
identically to one that did not.

The fixture is **`tests/determinism/render/expected/song.json`**, which the same suite builds
the same way from `determinism/render/script.json`. Five tracks with indices to order by, four
clips across three of them — one audio, two looping — six notes at four pitches and four
lengths, a section that ends past the last clip, and a tempo change at tick 3840 with three and
a half thousand ticks of music after it. It is the document `compile` is already goldened
against, so a view and a `RenderPlan` are read from one song.

Two things the golden's first draft could not catch on its own, both found by breaking it on
purpose rather than by reasoning about it, and both now covered:

- **Order.** A protobuf map decodes to a JavaScript object and `Object.values` returns insertion
  order, which for a song read from disk is the file's key order — lexical by ULID (ADR 0002
  §4), and a ULID sorts by when it was minted. In any fixture whose entities were created in the
  order they occur, "sorted by the model" and "whatever the map iterated" are the same list.
  Deleting the piano roll's `sort` left the golden green. The suite therefore also projects the
  same document with every object's keys reversed and requires the same answer, which is
  ADR 0001 §3's rule stated as a test rather than as a comment.
- **A case no fixture contains.** Nothing in the repository has a second time-signature event and
  no tool mints one, so the piecewise bar grid — the thing a single `ticksPerBar` gets silently
  wrong — has no golden input. Those conversions are asserted directly against constructed
  values, which is the distinction `tests/AGENTS.md` already draws for the schema fixture: a
  constructed value exercising a pure function is not a document under test, and CLAUDE.md #2 is
  about mutations.

It proves exactly one thing and it is the thing that matters: **every view is a pure function of
the model.** A store that has drifted from the document shows up as a golden diff rather than as
a mixer showing a gain nothing has. It proves nothing about wiring, and a driven session that
would — a UI automation driver replaying gestures and comparing `song.json` byte for byte — buys
that for a browser driver, a display in CI and a flake class this repository has never had. The
strongest reading of "every control is a tool call" is not worth being the first test here that
fails for reasons nobody can reproduce.

## Alternatives considered

**The carrier**

| Alternative | Rejected because |
|---|---|
| One `#[tauri::command]` per tool | A third hand-written dispatch beside gRPC's and MCP's, free to drift from both, and ADR 0006 §1's argument arriving a milestone later in a different language. |
| The host runs the `SongTools` gRPC server and the webview is a grpc-web or Connect client | Puts the UI on the tested wire path — which decision 1 gets for free through MCP — and charges a browser transport, a proxy or a Connect dependency, a second serialisation of every timeline read, and a listening socket on a desktop machine. §3 names no proxy. |
| Two OS processes, host and server | Contradicts §3's table, which puts `core` *in* `app`, and buys a boundary whose only purpose is to be crossed. |

**What the frontend holds**

| Alternative | Rejected because |
|---|---|
| Apply `ToolResult.patch` locally to skip the re-read | A second RFC 6902 apply in a third language. Its failure mode is a view and a `song.json` that disagree, with nothing comparing them — the exact shape ADR 0006 §3 refused for `dry_run`. |
| A normalised view store | What a React application looks like by default, and what §14.2 forbids. Arrives as a performance fix, not as a design decision. |
| Hand-written TypeScript model types | §4.1, flatly. |

**The project handle**

| Alternative | Rejected because |
|---|---|
| An `open_project` tool | The wire change ADR 0006 §5 refused, for reasons M2 does not remove: an MCP stdio process is still not a session. |
| A host process per project, re-exec'd per window | Makes File · Open spawn an operating-system process and leaves two windows on one project racing anyway, since the race is between processes. |
| Leave the directory unlocked, as M0 and M1 did | M0.3 made the single writer structural by giving one process one project. `app` plus an MCP client on one `.escri` is not a corner case; it is what §18.2 sells. |

**The pending edit**

| Alternative | Rejected because |
|---|---|
| Re-run the dry run before applying and diff the two | Two round trips and a new rule about what a difference means, to replace a check that already exists and already refuses. |
| Forbid the model moving while an edit is pending | A lock by another name, held by a user interface, over a model an agent is also editing. |

**The check**

| Alternative | Rejected because |
|---|---|
| Nothing new; `app` is outside §11 | Leaves the largest new surface in the repository uncovered and decision 2's failure mode silent — and it is a sentence someone must write into a `[MUST]` section, not an omission. |
| A driven session with a UI automation driver | A browser driver, a display in CI and a flake class this repository has never had, for wiring coverage. Revisit if a projection golden proves to be catching the wrong half. |

## Consequences

- `core/src/mcp.rs` is refactored before `app/` exists: the tool `match` becomes
  `call(session, name, args)`, and the MCP server keeps only the JSON-RPC envelope, the
  `inputSchema` generation and the two byte-level rules. This is a `core` change in a UI
  milestone and lands in PR 2, not PR 5 — the shell is the first thing that calls a tool.
- `core` gains the `.escri` lock and a structured error for a lock it did not take. The
  determinism suite opens projects; its fixtures acquire and release one, which is the first
  test that a second open is refused.
- `app/` needs no new-directory ADR: §13 lists it already, and `engine/` set the precedent in
  M1 PR 5.
- §11 gains a bullet and therefore a `[MUST]` section changes; §3, §5 and §9 gain the sentences
  that say what the webview calls and what it holds.
- The frontend's package set carries no state library and no test framework, and ADR 0016 pins
  it that way on purpose: decision 2 is a decision a dependency could quietly reverse.
- What is **not** decided here: coalescing a drag into one tool call (trap 2). It is a tool-call
  granularity question, it has no answer that is right for every gesture, and it belongs to
  PR 5 against a real drag rather than to an ADR against an imagined one. The sharp edge is
  recorded in the plan: intermediate states are unvalidated, so a coalesced drag can pass
  through a position the validator would refuse.
