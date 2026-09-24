# ADR 0020 — `ai` serves one stream the host executes, and a dead holder's lock is stale

- **Status:** Accepted (2026-09-21)
- **Affects:** `proto/assistant.proto` (new, M3 PR 3) and `proto/`'s Python target; `ai/`
  (new, M3 PR 5); `core/src/assistant.rs` (new — the spawn-and-dial the engine already has,
  one process over) and `core/src/project.rs` (`ProjectLock::take`); `app/src-tauri`;
  `docs/specs.md` §3, §6, §10, §13, §15, §17 and §18.2; `lock.baseline.json`
- **Builds on:** ADR 0012 §1 (`app` opens no network port; one dispatch, `call(session,
  name, args)`) and §3 (`.escri/lock`, taken with `create_new` — amended here); ADR 0013 §2 (a
  stream is the session identifier; cancel is close; a failure ends the stream with a status)
  and §3 (the engine names its own socket and prints `unix:<path>` once listening; the verdict
  on a failed call is the child's exit status); ADR 0008 §2 (a binary is told, never searched);
  ADR 0006 §7 (Python for `proto/` at M3 — carried out here, narrowed); ADR 0019 §1 (the
  model's calls are executed against a proposal, by the carrier's decision)
- **Recorded in:** `docs/specs.md` §3, §6, §10, §13, §15, §17 and §18.2.

## Context

§3 says `app` ↔ `ai` is gRPC and that `app` supervises `ai` as it supervises `engine`; it does
not say which side listens. That is `docs/plan.md`'s question 1, and the lock decides more of
it than it looks: `ai` cannot open the project itself, because `.escri/lock` refuses a second
process (ADR 0012 §3), so whatever `ai` reaches is **`app`'s session**, and the question is the
direction of the socket. Question 2 asks what crosses it and whether §13's `Jobs` — a service
named there and in §6 and defined nowhere in three milestones — is finally honoured or
amended. Question 12 asks who spawns `ai`, what it is told, and what the window shows when it
dies. And U4 left the Python gRPC stack to whichever one `betterproto2-compiler` 0.10.1
generates a service for.

The spike answered the last of those with a fact that ties the others together (plan, "What
the spike found"): with `client_generation=async` and `server_generation=async` the pinned
compiler emits a **`grpclib` client and a `grpclib` server**; with its defaults it emits a
synchronous `grpcio` client and **no server**, and there is no `grpcio` server option at all.
So a design in which `ai` serves is a design whose server is generated code, and a design in
which `app` serves needs `grpcio` handlers registered by hand or `grpcio-tools`' own `_pb2`
types — a second set of Python model types beside the Pydantic ones (§4.1). Five round trips
were run to make sure the generated pieces speak to each other and to the real `tonic` server,
over a Unix socket where the generated code can and over TCP where `escribass-grpc` can only
take a `SocketAddr`.

The spike also found a defect in the process the product already ships (plan, "Found beside the
six questions"), reproduced independently from the main thread. **Every Claude Code session
left `.escri/lock` behind** — 50 runs of 50, naming a process that no longer existed, so the
next open was refused `project_locked`. `escribass-mcp` removes the lock when its stdin closes
and leaves it on `SIGTERM`, `SIGINT` and `SIGKILL`: nothing handles a signal, a Rust `Drop`
does not run in a process a signal ended, and a signal is how Claude Code ends its stdio
servers. So the client §18.2 advertises leaves the project unopenable until a person deletes a
file by hand, which ADR 0012 §3 says nothing does for them. U5 narrowed §18.2's promise to
"one process at a time"; this narrows it again unless the lock is fixed, and the lock is
`core`'s to fix.

## Decisions

### 1. `ai` serves one bidirectional stream; `app` dials it; the model's tool calls come back over it and the host executes them

Question 1's (b), for the reasons the plan gave and one the spike added. `ai` serves a gRPC
service, **`Assistant`**, on a Unix socket it names; `app` dials it; a prompt goes in, and the
model's tool calls come **back** on the same stream as `{name, args}` — exactly the envelope
`call(session, name, args)` already takes (ADR 0012 §1), so the host becomes the third
envelope around the one dispatch, and `ai` holds no `SongTools` client at all. The host
executes each call against the proposal (ADR 0019 §1) and answers on the stream with the
`ToolResult` and the document as the proposal now has it.

This keeps three things true at once that (a) would have had to argue for. `app` opens
nothing: dialling a socket a child named is what it already does for the engine (ADR 0013 §3),
and ADR 0012 §1's "no port" needs no reading. The model's calls land on the dispatch the
determinism suite drives and the window uses, with the host — which knows which stream a call
arrived on — deciding that they are proposed. And `ai` never has the session, the project path
or a `SongTools` stub, so "agents never read or write the project file directly" (§5) is a
property of what the process is given rather than of what it is told not to do.

What the spike added: only `grpclib` has a generated server, so the process that *serves* is
the one whose gRPC is generated, and under (b) that is `ai`. Under (a) the Rust side already
serves (`tonic`) and Python would dial with generated code either way — but (a) needs a second
service regardless, because the panel's prompt has to reach `ai` somehow, and the only way in
is for `ai` to serve too. (a) is therefore (c), two paths for one thing, in disguise.

**ADR 0006 §7, carried out and narrowed.** `proto/` gains a Python target in PR 3 — one plugin
entry in `proto/buf.gen.yaml`, the pinned `betterproto2-compiler`, `server_generation=async`
and `client_generation=none`, because `ai` serves one service and dials nothing. What `ai`
consumes is `Assistant`; the `SongTools` and `Preview` servers the same run generates have no
consumer and are left unused rather than excluded, since a per-file plugin configuration is a
second thing to keep in step. That is narrower than "Python for `SongTools`", which ADR 0006
§7 assumed, and the plan asked that the narrowing be said.

**Extended 2026-09-22, in PR 3, on a fact this decision did not have.** "Left unused rather
than excluded" was written about the two *servers* and turns out to be true of more than
that: `betterproto2-compiler` generates a module for **every file in the request, imports
included** — it ignores `file_to_generate` — and it has no `extern_path` and no
`rewrite_imports`. So the run also re-emits `song.proto` and `history.proto` under
`proto/gen/python`, byte for byte identical to `schema/`'s copy and a **different class at run
time**. That is not an unused server, it is the second `Song` the four `extern_path` lines and
the two `rewrite_imports` in `proto/buf.gen.yaml` exist to prevent for Rust and TypeScript
(CLAUDE.md #1, ADR 0006 §4), and it would have been a real defect rather than a tidiness one:
the bar view is a pure function of `escribass_schema`'s `Song` (ADR 0018 §2) and would not have
taken the `Song` a `Prompt` arrives carrying.

So `proto/codegen.sh` deletes the re-emitted `escribass/song`, `escribass/history` and
`google` packages after `buf generate` and rewrites the imports that named them to
`escribass_schema`, with two `grep`s that fail the run if either the rewrite missed a line or
matched none. That is the Python spelling of the options the other two targets set, done by
the script because the plugin has none — and it is the whole of what this decision's "one
plugin entry" costs in practice. The alternative, generating `proto`'s Python into
`schema/gen/python`, was rejected: two scripts would write one tree, each deleting it whole,
and `proto/codegen.sh --check` hashes only its own `gen/`, so running them in the wrong order
would drop the `Assistant` module and the drift gate would pass.

### 2. The stack is `grpclib`, and each package is pinned where it is first installed

U4 approved "a Python gRPC stack — `grpcio` with `grpcio-tools`, or `grpclib`, whichever
`betterproto2-compiler` 0.10.1 generates service stubs for", and the `openai` SDK **with
`httpx2`**, the package it actually carries. Decision 1 settles the first half: **`grpclib`**,
as `betterproto2`'s `grpclib` extra plus the package itself, and no `grpcio` — the sidecar
serves, only `grpclib` has a generated server, and the runtime the generated `SongToolsBase`
and `AssistantBase` import is `betterproto2_grpclib`. Nothing else: `unittest` is stdlib
(ADR 0016 §3's reason, one language over), `pydantic` is pinned, and an agent framework, a
prompt library, a tokenizer or a vector store each returns to the user before it is installed.

**When they are pinned.** The plan's PR 2 row said "the Python packages by exact version" here;
the user's U4 decision, recorded later in the same file, says each is pinned "by exact version
in `lock.baseline.json` and §17 when it is first added, not before". The later,
user-confirmed decision wins, and it is the right one: this pull request installs nothing, and
a version pinned in prose today would be re-resolved by `uv` in PR 5 and either agree by luck or
drift in silence — the `codemirror: "6.x"` shape ADR 0016 §1 refused. So §17 and
`lock.baseline.json` record the three packages by **name**, approved and unpinned, and **PR 5**
(`m3.5-ai-shell`), which first runs `uv lock`, writes the exact versions and `uv.lock`'s hashes
beside them in the same change.

**Amended 2026-09-23 by the user, for `grpclib` alone.** "PR 5" was this ADR's reading of
*where* each package is first added, and it was wrong for one of the three: PR 3 generates a
server that imports `grpclib`, and the check that the generated tree imports — and that its
`Song` is `escribass_schema`'s class and not the copy `betterproto2` re-emits (decision 1,
extended) — cannot run without it. So `grpclib` is first added in **PR 3**, and U4's rule,
unchanged, pins it there: 0.4.9, resolved by `uv`, hashed in `schema/uv.lock`, recorded in
§17 and `lock.baseline.json` in that same change. It lives in `schema/`'s dev group, because
`proto/` has no Python environment of its own on purpose; PR 5 declares it again for `ai/` at
this version. `openai` and `httpx2` are untouched — nothing before PR 5 imports either, so
for them the paragraph above stands exactly as written. A row that says a pin is owed is not a pin, and the file's own
`commit: null` idiom already says so for two other rows. The spike's scratch environment used
`grpclib` 0.4.9 and `openai` 3.14.1 with `httpx2` 2.13.0; those are what it measured with, not
what is pinned.

### 3. What crosses: a prompt and its results in, text and calls out; no `Jobs` service

One RPC, `Assistant.Prompt`, bidirectional streaming, one stream per prompt — the shape ADR 0013
§2 chose for `Preview` and for the same reasons: the stream is the turn's identifier, closing it
is cancellation for free, and a failure ends it with a status rather than an error arm.

Host → `ai`, two commands: the **prompt** — its text, the `Song` the view is computed from
(binary proto on the wire, §4.1; discarded by `ai` when the turn ends, trap 7), the offered
tools' schemas as the host filtered them (ADR 0022 §1), the model id the project records
(ADR 0021 §4), and the conversation so far, so `ai` holds nothing between streams (ADR 0021
§3) — and a **call's result**: the call id, the `ToolResult` by value, and the `Song` as the
proposal now has it, from which `ai` recomputes the view (ADR 0018 §2). `ai` → host, three
events: **text**, streamed as the model produces it; a **call** — the provider's call id, the
tool name, the arguments as the JSON object the model wrote, and the model id the response
named; and **done**, with the model's final text and that model id. The arguments cross as
JSON text and not as a `oneof` of the twelve request messages, because twelve typed arms
mirroring `song_tools.proto` are the second description ADR 0006 §4 refuses, and the host
hands the object to `call` exactly as MCP does. PR 3 writes the messages; they are named here
so that `buf breaking` compares `proto/`'s whole M3 shape once against a `main` that has not
moved (plan, trap 14; M2 trap 12), and so that this decision is not the one ADR 0013 §2 had to
be amended for.

**There is no `Jobs` service.** §6's "long tasks are jobs with progress" is true of this loop
and is satisfied by the stream: progress is the events, cancellation is the close, the answer
is `done`. A `Jobs` service beside it would be a second way to ask about the same thing, with a
handle nobody needs (ADR 0006 §5's objection to a project handle, one noun over). §13's line is
**amended** to name what `proto/` holds — `SongTools`, `Render`, `Preview`, `Assistant` — and
the trigger for a job that is not a stream is a long task that is not a conversation, which
nothing in M3 or M4's plan makes: a compile is a tool with a timeout (§7.2).

### 4. `core` spawns `ai` as it spawns the engine: told a command, reads one line, dials

Question 12. The engine already shows the whole shape (`core/src/engine.rs`, ADR 0013 §3): a
child is started from a path it was *told*, its stdout is read for **one line**, `unix:<path>`,
printed after the server is listening, and the address is dialled over a Unix socket with the
`tonic` and `tokio` already pinned. `ai` gets the same, in a sibling module: `core` is told a
**command** — argv, which on a build tree is `uv run --project <ai/> escribass-ai` and which
`core` does not inspect — starts it, reads the line, and holds the connection for the session's
life. That keeps the session's process-management in one place with one set of rules, and it
keeps `core` ignorant of Python in the only sense that matters: it knows a command, as it knows
a path, and nothing about what the command is. The host tells it, as it tells it `--engine`.

The plan's default said the host should spawn `ai` "because the session is `core`'s and `core`
should know nothing about a Python interpreter". Putting the spawn in `core` is chosen over it
for the reason the loop's tests need: the loop's host half — the proposal, the stream client,
the routing — has to be drivable from `tests/` with no window (ADR 0022 §4), and `tests/`
already spawns `core`'s binaries and the engine; a spawn that only the Tauri host could perform
would leave the end-to-end golden behind a display.

What `ai` is told: **never the project path** (§5); no socket, since it names its own; the model
id and the tools per prompt, on the stream; and the key from **the environment and nowhere
else** — `OPENROUTER_API_KEY`, inherited, never a flag (`ps` shows flags), never `lock.json`,
never a transcript (U3; plan, trap 10). The sidecar passes the key and `base_url` to the SDK
explicitly, because the SDK otherwise reads `OPENAI_API_KEY`, which the spike found set on the
machine it ran on.

The window's health dot for `ai` is **the child's exit status, read** — the `Preview::drop` gap
(plan, "Known gaps") is not repeated in its sibling: a sidecar that dies is reported with its
status and the tail of its stderr, and the next prompt starts another. The dot for a process
that is running is a fact `core` can state; the dot never says more than that.

### 5. A lock whose recorded holder no longer exists is stale, and is replaced

**Amending ADR 0012 §3, and §10 with it.** The rule was "never broken automatically, because a
lock file whose owner may still be alive is not something a program can adjudicate". The
premise holds and the rule was wider than it: the file *records its holder's pid*, and a pid
that names no process is something a program can adjudicate. So `ProjectLock::take` now reads
the pid in an existing lock and, when no process by that pid exists, removes the file, takes
the lock, and says so in its result — "replaced a lock left by process N, which is gone" — so a
person learns a crash happened rather than nothing. A lock naming a **live** pid is refused
exactly as before; a lock with **no** pid — one written by hand, or one whose holder took the file and
failed the best-effort write of its pid — is refused as before, because there is nothing to
check. Nothing is broken
while its holder may be alive; that sentence is the one that stands.

The check is `/proc/<pid>` on Linux, which is the platform M3 claims (ADR 0014 §2) and costs no
dependency; `ponytail:` macOS and Windows get theirs with the installer (M5), and until then a
build for either refuses as today. A recycled pid can only refuse — a new process wearing a
dead holder's number is alive — so the check errs on the side the old rule chose. It still
assumes a local filesystem, as the lock always has.

**Amended 2026-09-24, in M3 PR 10, and the amendment removes the mechanism the two paragraphs
above describe.** Two things the decision got wrong, both found by the M3 review.

The first is a claim in the paragraph below this one and in the Consequences: that removing the
stale file and re-creating it is still one `O_EXCL`, so "if another process replaced it in the
same instant, this `create` finds the file there and refuses". That is true in one ordering
only. A and B both open a crashed project; both read the same stale pid; A unlinks, creates and
holds; B — which read before A's unlink — then unlinks **A's fresh lock** and creates its own.
Both hold, both write, and the case is ordinary rather than exotic: double-clicking the app on
the last project after a crash. Watched failing first, in
`core/tests/project.rs::two_openers_of_one_crashed_project_do_not_both_get_it`.

The second is pid reuse, which the paragraph above says "can only refuse". It can, in the
direction it names. What it cannot do is stop the *other* direction from being wrong for a
different reason: the file is what decides, and the file is not the holder. A lock file whose
contents no longer match the process holding it — overwritten by hand, or by a build that
wrote a number and died — sent a second opener straight past a live holder.

**The lock is the kernel's from now on.** `ProjectLock::take` opens `.escri/lock` and takes an
advisory lock on the open file with `std::fs::File::try_lock` — stable since Rust 1.89, so
still no dependency (CLAUDE.md #4) — and holds the descriptor for as long as it has the project
open. The kernel releases the lock when that descriptor closes, which happens when the process
ends however it ends: cleanly, by `SIGKILL`, or by a crash. So the whole of what §5 set out to
do is done by something that cannot be raced and has no number to adjudicate, and the thing it
set out to do is unchanged: *a lock a dead holder left no longer stops the next process from
opening the project*.

What the pid becomes is a **label**. It is written into the file under the lock, and it is read
for exactly two purposes, neither of them a decision: a refusal names who to close, and a file
that is **not empty** when a taker acquires the lock says its last holder never reached its
`Drop` — which is the crash report this decision asked for, kept word for word. A clean close
empties the file.

**Nothing unlinks it.** That is the other half of closing the race: a file another process may
already have open is not ours to take away, and a `remove_file` in `Drop` is the same window in
the other direction. `.escri/lock` now stays on disk, empty, between sessions, which is how
`cargo`'s own lock files behave.

**One claim this reverses**: "not that a lock with no pid in it is replaced" (`docs/plan.md`,
"What M3 will not claim"). There is nothing to replace and nothing to adjudicate — if nobody is
holding the file, the next opener gets it, whatever the file says.

Why not signal handlers instead, in the two binaries: they would release the lock on `SIGTERM`
and `SIGINT` and leave it on `SIGKILL` and on a crash, which is the case ADR 0012 §3 was written
about and the case the pid check covers. A handler is not refused — a binary may gain one so a
`Preview`'s child is reaped on a clean interrupt — but it is not the remedy. Why not a lease
or a heartbeat: a clock in `core` (CLAUDE.md #3) and a second file format, to answer a question
the pid already answers.

**§18.2 is narrowed as U5 decided, and this decision is what makes the narrowed promise
true.** An external MCP client drives a project **with the window closed, one process at a
time**; the lock refuses the second (ADR 0012 §3); and when the client's process ends by
signal, as Claude Code's do, the next open proceeds instead of asking a person to delete a file.
It lands in **PR 4**, `core`'s pull request in this milestone, beside the provenance stamp
(ADR 0021 §1) — both are `core` closing a hole the spike reached through the tool API — with a
test watched failing first: a child that took the lock and was killed with `SIGKILL`, then a
`take` that succeeds and names it; and a child still alive, then a `take` that is refused.
Its test is loud, so it does not muddy that pull request's silent half.

## Alternatives considered

**Direction (question 1)**

| Alternative | Rejected because |
|---|---|
| (a) `app` serves `SongTools` over a Unix socket; `ai` is a generated client | The panel's prompt still has to reach `ai`, so `ai` serves too and this is (c). It also gives Python a `SongTools` stub the loop must be told not to use for `get_song` (ADR 0018 §2) and `set_param` (question 7), where (b) withholds by construction; and a listening socket in `app` is a reading of ADR 0012 §1 rather than the sentence |
| (c) Both — `SongTools` in `app` for external clients, a stream for the bundled model | Two paths for one thing, and U5 already decided the external client's path is `escribass-mcp` with the window closed |
| `grpcio`, with handlers registered by hand or `grpcio-tools`' `_pb2` types | The pinned compiler generates no `grpcio` server; hand-registered handlers are a hand-written service description, and `_pb2` types are a second set of Python model types beside the Pydantic ones (§4.1) |
| Pin `grpclib`, `openai` and `httpx2` by version in this pull request | Nothing is installed here; a version written in prose is re-resolved by `uv` in PR 5 and drifts in silence if it disagrees. U4, as the user confirmed it, pins on first install |

**Wire shape (question 2)**

| Alternative | Rejected because |
|---|---|
| A `Jobs` service beside the stream | A second way to ask about one thing, with a handle nobody needs; the stream already carries progress, cancellation and the answer. §13 named it and nothing defined it in three milestones |
| Unary RPCs — `Prompt`, then `Poll` | Invents the job handle the stream makes unnecessary, and gives cancellation a second mechanism |
| Tool arguments as a `oneof` of the request messages | Twelve arms mirroring `song_tools.proto`, free to drift from it (ADR 0006 §4); the host takes a JSON object already |
| An error arm on the event | ADR 0013 §2's reason: a failure ends the stream with a status, and a provider failure after its retries is one (ADR 0022 §3) |

**Spawn (question 12)**

| Alternative | Rejected because |
|---|---|
| The Tauri host spawns `ai` and hands `core` the address | Puts the loop's host half behind a window, so the end-to-end golden (ADR 0022 §4) cannot run from `tests/`; the engine's spawn is `core`'s and this is the same shape |
| `core` finds `uv` or a Python on the path | A search finds the wrong one silently — ADR 0008 §2's argument for `--engine`, unchanged |
| The key on the command line or in `lock.json` | `ps` shows flags and `lock.json` is committed (U3; trap 10) |
| The schemas handed to `ai` once at launch | A launch-time contract that a per-prompt tool list (ADR 0022 §1) would then have to match; per prompt, the list and the schemas are one message |

**The lock**

| Alternative | Rejected because |
|---|---|
| Keep "never broken automatically" and document the `rm` | 50 runs of 50 through the client §18.2 sells left a project unopenable; a rule whose every trigger is a person deleting a file is a rule the product pays for daily |
| Signal handlers in both binaries | Cover `SIGTERM` and `SIGINT`; do not cover `SIGKILL` or a crash, which the pid check does. Complementary, not the remedy |
| A lease with a heartbeat | A clock in `core` and a second file format for a question the pid answers |
| `kill(pid, 0)` through `libc` | A direct dependency `lock.baseline.json` does not list, for what `/proc` gives on the one platform M3 claims |

## Consequences

- **PR 3** (`m3.3-proto-py`): `proto/assistant.proto` with the one RPC and the messages
  decision 3 names, and the Python target with `server_generation=async` and
  `client_generation=none`, its entry under `tool_api` in `lock.baseline.json` as TypeScript's
  is — the whole M3 shape of `proto/` in one change, since `buf breaking` compares it once
  (trap 14). No provenance on the wire (ADR 0021 §2) and no `list_params` (ADR 0022 §2), so
  the shape is one service. **Done 2026-09-22**, with the extension to decision 1 above and
  one thing first left owed: the generated `AssistantBase` imports `grpclib`, which decision 2
  places in PR 5, so the Python committed here was byte-compared by
  `proto/codegen.sh --check` and **imported by nothing**. That was written down as a deferred
  check rather than skipped. **The user closed it on 2026-09-23, in this same pull request**:
  the check is what first *adds* `grpclib`, so PR 3 is where U4 pins it, and decision 2's "PR
  5" is amended to that extent — see its own paragraph. `grpclib` 0.4.9, resolved by `uv`, in
  `schema/`'s dev group; `proto/tests/test_generated_python.py` imports the tree and asserts
  that a `Prompt`'s `song` is `escribass_schema`'s `Song`, which an import alone would not,
  since a duplicate imports as happily as the real thing.
- **PR 4** (`m3.4-provenance`): `ProjectLock::take` replaces a dead holder's lock and says so,
  with the two tests above; §10's and ADR 0012 §3's sentences change in this pull request,
  ahead of the code, as the ADR convention requires. **Done 2026-09-23.** The sentences had
  landed in PR 2 and needed no further change. `take` is two `O_EXCL` attempts rather than one
  — the stale file is removed and re-created, so a second process that replaced it in the same
  instant wins and this one is refused, by a holder that is alive. The tests spawn a real
  `escribass-mcp`, and the one that is killed asserts the premise first: the lock is still
  there after the signal, which is the `Drop` that did not run. What the test had to learn that
  the decision did not say: a killed child its parent has not waited on is a **zombie**, and
  `/proc/<pid>` exists for a zombie — so the test reaps before it takes the lock, and in
  production a zombie holder reads as alive and refuses, which is the side this errs on
  anyway.

  **Undone 2026-09-24 in M3 PR 10**, by decision 5's own amendment: the two `O_EXCL` attempts
  are gone with the rest of the pid protocol, and the sentence above — "a second process that
  replaced it in the same instant wins and this one is refused" — was the claim the review
  disproved. It holds only when the second process's *unlink* precedes the first's *create*,
  and the opposite ordering had both of them holding. The tests stay: the killed `escribass-mcp`
  still leaves its lock behind, and the next opener still names it — what changed is that the
  kernel released the lock when the process died, so nothing has to be adjudicated to find that
  out. Two tests joined them, both watched failing first against the pid protocol: a live
  holder whose lock file has been overwritten with a dead pid, and two threads opening one
  crashed project two hundred times over.
- **PR 5** (`m3.5-ai-shell`): `ai/` with `grpclib`, `openai` and `httpx2` pinned by exact
  version in `pyproject.toml`, `uv.lock`, `lock.baseline.json` and §17 in the same change;
  `.python-version` with the exact patch §17 asks for; the generated `Assistant` server on a
  socket it names; `core/src/assistant.rs` spawning it and reading the line; the window's
  health dot from the exit status. It answers a prompt with the scripted provider's text and
  proposes nothing. PR 3's deferred check is **not** owed here any more: `grpclib` was
  pinned there on 2026-09-23 and `proto/tests/test_generated_python.py` runs in `checks`
  already. What this pull request adds for the package is its declaration in
  `ai/pyproject.toml` — `betterproto2[pydantic,grpclib]` plus the package — at the version
  `lock.baseline.json` already names. **Done 2026-09-23**, with `openai` 3.19.0 and `httpx2`
  2.13.1 as `uv` resolved them. Three things this decision did not have to say and the build
  did. **`ai` depends on the generated trees as packages**, not by `sys.path`: the deferred
  `[build-system]` row fired as written, and it took a `proto/pyproject.toml` with it, because
  the sidecar imports `escribass_proto` as well as `escribass_schema` — both editable, since
  `codegen.sh` deletes `gen/` whole on every run and a built copy would be the second `Song`
  decision 1 spent a paragraph preventing. **Stopping it is closing its stdin**: a sidecar has
  no last call to end on, so the pipe is the signal — `escribass-mcp`'s own, named in this
  ADR's Context — and it is what makes an exit status of 0 distinguishable from a crash for
  §5's dot. And **a failed turn does not kill a live sidecar**, which is the one place the
  engine's shape could not be copied: an engine is born with its work and dies with its answer
  (ADR 0008 §2), where this serves a session.
- **PR 8** (`m3.8-loop`): the stream becomes a loop. `Sidecar::answer` — PR 5's collect-and-
  half-close, whose own `ponytail:` said both would change here — is **deleted** rather than
  kept beside `Sidecar::turn`, because two ways to take a turn is one that stops matching what
  `core` actually does; its three tests now drive `turn` against a real `Session`. The send half
  stays open for the whole turn, the host answers each `ToolCall` with a `CallResult` on it, and
  closing it is how the host ends a turn whose refusal budget ran out (ADR 0013 §2). **Done
  2026-09-24**, with one thing decision 3 did not have to say: a status the sidecar *returned*
  and a transport that broke have to be told apart at the host, because `ai` reports a provider
  failure it has already classified as a gRPC status and a dead socket looks the same from a
  string. They are separate values in `core/src/assistant.rs` for that reason, and a child that
  is gone overrides both.
- `docs/specs.md`: §3 says which side listens and what comes back; §6 names `grpclib` where it
  said `grpcio`; §10 and ADR 0012 §3 say what a stale lock is; §13 names the four services and
  loses `Jobs`; §17 and `lock.baseline.json` gain the three packages by name, unpinned until PR
  5; §18.2 is narrowed as U5 decided, with the lock's sentence.
- "What M3 will not claim": not that an MCP client and the window edit one project at once —
  the lock refuses it — and ~~not that a lock left by an older build, with no pid in it, is
  replaced~~, **struck 2026-09-24 by decision 5's amendment**: with the kernel holding the
  lock there is nothing in the file to adjudicate, so a lock nobody is holding is taken
  whatever it says.
- **PR 10** (`m3.10-review-fixes`, 2026-09-24) carries two amendments of this ADR's, both from
  the M3 review and both written into the decisions above. Decision 5's lock is the kernel's
  rather than the file's. And decision 4's "the host closes our stdin to stop us" turned out to
  be true only when the sidecar was **idle**: `grpclib`'s `Server.close` cancels the request it
  is serving, but `Server.wait_closed` then waits for the open connection as well, and the
  thing holding that connection is the host that has just let go — a window on its way out,
  whose turn thread is parked on a call nobody will answer. Measured at 30 s and counting, with
  the process, its socket and the project's `.escri/lock` all still alive. `serve` now closes
  and leaves on that path rather than waiting. `core` gains `Halt`, a handle to that pipe held
  by somebody who is *not* holding the `Sidecar` — because a turn borrows it for the whole of
  a turn, and the window's exit path took the same lock — and `app` calls it before it stops
  the sidecar, so closing the window ends the turn with the window.

