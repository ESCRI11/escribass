# ADR 0024 — A generator compiles in a subprocess `core` spawns, and the DSL is a Python subset over integers and fractions

- **Status:** Accepted (2026-09-25)
- **Affects:** `compilers/generative/` (new, under §13's existing line; M4 PR 4);
  `core/src/session.rs` and `core/src/generator.rs` (a third child, M4 PR 5 — its own module
  beside `engine.rs`, whose `listening`, `ended` and `tail` it reuses); `core/src/bin/*`
  (`--generator`); `docs/specs.md` §7.1 and §3; `tests/determinism/generators/` (M4 PR 5);
  `.github/workflows/checks.yml` (M4 PR 4 and PR 5)
- **Builds on:** ADR 0008 §1 (stdout carries the protocol and nothing else; failure is an exit
  code) and §2 (a fresh process per call, and why); ADR 0013 §3 (the child names its socket and
  prints `unix:<path>` once it is listening; the verdict is the child's exit status); ADR 0020
  §1 (`ai` is a server holding no session and no project) and §4 (`core` spawns a child it is
  told a command for); ADR 0006 §2 (caller-fixable is `valid = false`, operator is `Err`); ADR
  0001 §5 (ids from an injectable source in `core`); ADR 0018 §4 (`Fraction`, no `libm`, for
  the reason a byte-compared golden cannot carry a transcendental); ADR 0021 §1 (`prepare`
  decides every entity's provenance); ADR 0023 §1 (M4 is the generators)
- **Recorded in:** `docs/specs.md` §7.1 and §15, with §3's tier table amended.

## Context

§7.1 decided the language in 2026-09-02 — "a small Python DSL (Pydantic-validated, sandboxed
subprocess, no network, no filesystem, wall-clock disabled) running inside the `ai` process" —
and decided nothing else: not what the subset is, not what "sandbox" claims, not what a compile
writes or what "stale" means. It was also written before ADR 0020 made `ai` a server that holds
no session and no project, and before `compile_generator` was a `SongTools` tool an MCP client
drives with the window closed and no `ai` running at all (§18.2, narrowed 2026-09-21). "Inside
the `ai` process" cannot be where a tool served by `core` runs.

M4 is the first milestone since M0 where CLAUDE.md #3 names the thing being built — `compilers`
— and where "same input, same output" is a property to be *designed* rather than measured in
somebody else's plugin. The plan's item 1 of "What 'deterministic' means for a compiler" says
what the claim is: the same source, seed, params, tempo map, signature map and target produce
the same notes, byte for byte, and the claim can honestly be platform-free because the DSL's
numeric domain is the model's own. The spike measured the one part of that sentence that could
not be reasoned to: `random.Random(2**63 + 1)` through `getrandbits`, `randrange`, `randint`,
`choice`, `shuffle` and `sample`, a 400-step `Fraction` chain reduced onto integer ticks through
`__ceil__`, `__floor__` and `round`, and a block of `divmod`, `//`, `%` and three-argument `pow`
gave **one digest, `1730edd3…`, on CPython 3.11.14, 3.12.3, 3.12.12, 3.13.12 and 3.14.3**. It
also found the hazard next door: `hash()` of a `str` is randomised per process, so iterating a
`set` of names gave three orders in three runs of one source, and `PYTHONHASHSEED=0` fixed it.

Questions 1, 2, 3, 4, 5, 6, 8's compile half, 11 and 14 of the plan are decided here. The wire
the compile crosses is ADR 0026's; the hash's home in `lock.json` is ADR 0027's.

## Decisions

### 1. The sandbox is a subprocess `core` spawns per compile, told `--generator <cmd>`

`compile_generator` runs the source in a **fresh process that `core` spawns for that compile
and that exits with its answer**, exactly as `render_export` spawns the engine: `core` is told
a command on its own command line — `--generator <cmd>`, beside `--engine` and `--ai`, optional
for the reason `--engine` is (a session that only edits must start on a machine with no compiler
installed), and the call that needs it refuses as an operator error, `generator_missing`, when
it is absent. Never searched, never defaulted (ADR 0010 §4's rule for the manifest, ADR 0008 §2's
for the engine). §7.1's "inside the `ai` process" is **amended**: the compiler is neither `ai`'s
nor `core`'s to run in-process. Not `ai`'s, because `ai` serves a stream and holds no session
(ADR 0020 §1), an MCP client compiles with no `ai` running, and `ai` never runs a compiler
(CLAUDE.md #6's spirit: the tier that talks to a model is not the tier that executes code). Not
`core`'s, because `core` is Rust and the DSL is Python, and because a process boundary is what
the sandbox's limits (decision 4) are enforced at.

**A fresh process per compile**, never a warm interpreter, for ADR 0008 §2's reason one tier up
and for the sandbox's own: nothing survives between two compiles, so a compile cannot depend on
the compile before it, and a limit that kills the child costs nothing to recover from. The
shape `core/src/engine.rs` already has for two children — spawn, read one `unix:<path>` line,
dial, read the exit status — serves a third; `Sidecar` (M3 PR 5) is the precedent for reading
the status the moment the child is gone, and this child is short-lived so `Preview::drop`'s
open gap does not recur. The child is told **nothing about the project**: no path, no
`lock.json`, no `song.json`; it is handed what it compiles (ADR 0026 §1) and hands back notes.

~~What the process costs per compile is **unmeasured** here~~ — **measured 2026-10-01, in M4
PR 5, and it is 280–370 ms** for a one-note compile driven end to end through `escribass-mcp`
on this machine. The child alone, from spawn to the socket line, is a median **242 ms** through
the console script and **366 ms** through `uv run --no-sync`, so the launcher is about 120 ms of
it; a thousand notes add about 80 ms and ten thousand take the whole call to 1.0 s. A third of a
second is nothing per click and would be unusable per keystroke, which is trap 8 felt rather
than argued, and the rule that makes it not matter is decision 5's: one compile per click,
never per keystroke.

### 2. The transport is gRPC over a socket the child names, one `Compile` call, exit

The child prints `unix:<path>` as the one line of its stdout once its server is listening (ADR
0013 §3, amended: the address and the readiness are one fact), `core` dials with the `tonic`
already pinned and makes **one** call, `Generate.Compile`, and the child exits. The verdict on
a call that fails is the child's exit status, never the transport's (ADR 0013 §3). Its stderr's
tail travels with an operator error as the engine's does.

**Amended 2026-10-01, in M4 PR 4, with the thing "and the child exits" left unsaid.** The
child answers, sends its trailers explicitly rather than at the handler's exit, and *then*
stops serving — the two orders apart being a race in which the server closes before the
response has left. It also stops when its **stdin closes**, which is the net for a `core` that
died before it called, or that called and crashed: a child that outlives its purpose is tidied
by the host letting go, as `ai`'s is (ADR 0020 §5). So `core` may let go of the child without
waiting, and must not start it with stdin already closed and then expect it to wait.
**Sharpened 2026-10-01, in M4 PR 5, where the sentence above turned out to have a sharp
edge**: the engine is spawned with `Stdio::null()` (ADR 0008 §2's reason — nothing arrives that
way and an inherited stdin would compete with the MCP server for it), and `/dev/null` is
*already at end of file*. A child started that way stops serving before it is dialled. So this
child's stdin is a **pipe `core` holds open for the length of the call** and drops on every
path out of it, which is `ai`'s shape rather than the engine's (ADR 0020 §4) and is named here
because "spawn it the way the engine is spawned" is the obvious wrong move.

gRPC rather than stdio or argv-and-files because the client exists on both sides — `tonic` in
`core`, `grpclib` in the Python the pinned `betterproto2-compiler` generates a server for (ADR
0020 §2) — and because the messages need a `.proto` under `buf breaking` whichever way they
cross: the transport is the cheaper *diff*, not the cheaper idea. M2 PR 9 deleted the stdio
path rather than keep a second transport, and this decision does not reintroduce one for one
call.

### 3. The DSL is a Python subset behind an `ast` allowlist, over `int` and `Fraction`, with one seeded `rng`

The source is **parsed with `ast` and refused before execution** unless every node and every
name is on the allowlist below; it is then executed with a `builtins` that holds the DSL's
namespace and nothing else — no `__import__`, so an `import` cannot succeed even if its node
were allowed, and no `open`, `eval`, `exec`, `getattr`, `globals` or `__builtins__` to reach
around the list. This is a **subset of Python, not a dialect**: everything the DSL accepts means
what CPython means by it, so a model that knows Python needs no second grammar, and everything
it refuses is refused with the node's name and its line.

**Nodes allowed:** `Module`, `Expr`, `Assign`, `AugAssign`, `AnnAssign` without a value's
annotation being evaluated, `For`, `While`, `If`, `Break`, `Continue`, `Pass`, `FunctionDef`
with positional and keyword parameters and no decorator, `Return`, `Call`, `Name`, `Constant`
whose value is an `int`, `str`, `bool` or `None`, `BinOp`, `UnaryOp`, `BoolOp`, `Compare`,
`IfExp`, `List`, `Tuple`, `Dict`, `ListComp`, `DictComp`, `Subscript`, `Slice`, `Attribute`
whose name does not begin with `_`, and `JoinedStr` with `FormattedValue` for messages. **Refused
by absence:** `Import`, `ImportFrom`, `ClassDef`, `Lambda`, `Try`, `Raise`, `With`, `Global`,
`Nonlocal`, `Yield`, `Await`, every `Async*`, `Starred`, `Set`, `SetComp`, `Delete`, `Assert`,
and a `Constant` that is a `float`, `complex` or `bytes`. **Refused by name:** any name not in the
namespace, and any attribute beginning with `_`.

**No `float`, anywhere.** A float literal is refused at parse; `/` on two integers is
`Fraction`'s job and the author writes `Fraction(a, b)` or `a // b`; nothing in the namespace
returns a float. **Amended 2026-10-01, in M4 PR 4, where implementing this proved one half of
it wrong**: `/` on two integers cannot be made `Fraction`'s job and is not. `bar` and `beat`
return an `int`, as the table below says, and CPython's `/` on two `int`s is float division —
`beat(1) / 3` is `320.0`, not the `Fraction(320, 1)` the worked example below claimed, and
there is no hook that would make it otherwise without replacing the `int` the table fixes. So
**`Div` is outside the language**, refused at parse with its line and naming what the sentence
above already says to write instead; the worked example's two `/` sentences are corrected with
it. The rule then holds **twice**, which is this decision's habit: `/` is gone, *and* every
value that reaches `note` is checked, because `pow(2, -1)` and `2 ** -1` are floats CPython
hands an author whatever an allowlist says about literals. This is ADR 0018 §4's reason — a transcendental's last bit is the platform's,
and a byte-compared golden cannot carry it — and it is what makes the platform-free claim
honest rather than hopeful. A value is an `int`, a `Fraction`, a `str`, a `bool`, `None`, or a
list, tuple or dict of those.

**No `set`, by two locks.** `hash(str)` is randomised per process, so a set of names iterates in
an order that is not a property of the source; the spike measured three orders in three runs.
The child is spawned with **`PYTHONHASHSEED=0`** (decision 4) *and* the `Set` and `SetComp`
nodes and the name `set` are outside the language. Both, because a default nobody can see is how
Surge XT's `A Osc 1 Retrigger` hid for two pull requests (trap 2): the environment variable is
belt and the allowlist is braces, and either alone would pass every test until a machine set the
other. `dict` is allowed, because a dict iterates in insertion order by the language's own
guarantee since 3.7, and `sorted` over strings is lexicographic and reads no hash.

**The namespace**, v1, written out so a reader and a model see the same list:

| Name | What it is |
|---|---|
| `note(pitch, start, length, velocity=100)` | Adds a note to the output. `pitch` 0–127, `start` and `length` in ticks relative to the clip's start, `velocity` 1–127, all **integers**; a `Fraction` that is not integral is a refusal naming the argument, never a rounding |
| `PPQ` | 960 |
| `bar(n)`, `beat(n)` | The tick, relative to the clip's start, at which bar *n* or beat *n* (zero-based) of the clip begins, walked from the signature map as `app/src/time.ts` walks it (ADR 0018 §1); an `int` |
| `clip` | `clip.start` (absolute tick) and `clip.length` (ticks) — the bounds the output must fit |
| `rng` | A `random.Random` seeded with `Generator.seed`, the integer, at compile start. `getrandbits`, `randrange`, `randint`, `choice`, `shuffle`, `sample` — the six the spike measured — and `random()` is refused because it returns a float |
| `params` | `Generator.params`, a dict of `str` to `str`; the author converts with `int(...)` or `Fraction(...)`. Typed params stay deferred (ADR 0002 §8) |
| `sections` | The song's sections as `(name, start_tick, end_tick)` tuples in absolute ticks, in tick order |
| `tempo_at(tick)`, `signature_at(tick)` | `Fraction` bpm, and `(numerator, denominator)`, in force at an absolute tick |
| `Fraction`, `int`, `str`, `bool`, `len`, `range`, `enumerate`, `zip`, `min`, `max`, `abs`, `sum`, `sorted`, `reversed`, `divmod`, `pow`, `floor`, `ceil`, `round` | The standard library's, unchanged; `floor`, `ceil` and `round` take a `Fraction` to an `int` by `fractions`' own definitions, which the spike measured stable |

Additions to the namespace are additive and move no golden; a name removed, or a name whose
meaning changes, bumps the DSL's version (ADR 0027 §1), because a golden would move.

**A worked example.** A clip of one bar of 4/4 at 960 PPQ, seed `7`:

```python
# A kick on every beat, a snare on two and four, a hat on every eighth whose
# velocity the seed decides.
for b in range(4):
    note(36, beat(b), beat(1) // 2, 100)
    if b % 2 == 1:
        note(38, beat(b), beat(1) // 2, 110)
    for h in range(2):
        note(42, beat(b) + h * (beat(1) // 2), beat(1) // 4, 60 + rng.randrange(0, 30))
```

compiles to fourteen notes, `set_notes` semantics over the whole clip: kicks at ticks 0, 960,
1920 and 2880 of length 480 and velocity 100; snares at 960 and 2880 of length 480 and velocity
110; eight hats at 0, 480, 960, … 3360 of length 240, whose velocities are the first eight
draws of `random.Random(7).randrange(0, 30)`, in order, plus 60 — the same eight on every
machine that runs the pinned interpreter, and on the four others the spike ran. `beat(1) // 2`
is `480`, an `int`; ~~`beat(1) / 3` would be `Fraction(320, 1)` and legal, and `beat(1) / 7`
handed to `note` is `Fraction(960, 7)` and refused, naming `start`~~ — **corrected
2026-10-01**: `/` is outside the language, `Fraction(beat(1), 3)` is `Fraction(320, 1)` and
legal, and `Fraction(beat(1), 7)` handed to `note` is `Fraction(960, 7)` and refused, naming
`start`. The output is a list the
child returns; the entry it becomes is decision 5's.

### 4. What "sandbox" claims: purity by construction and a limit on time and memory — not security

The word is used in §7.1 and this decision says exactly what it means, because "sandbox" is
read as a security boundary by default and this one is not.

**Purity by construction.** The names for a clock, a file, a socket, an environment variable
and an import do not exist in the namespace, and the nodes that could reach them do not exist
in the language. A generator therefore *cannot* read the time or the filesystem — not because a
call is intercepted, but because there is nothing to call. This is CLAUDE.md #3 held by absence,
which is stronger than held by discipline, and it is what the tests in M4 PR 4 exercise: every
construct outside the allowlist fed and watched refused with its line, each failing first
against an allowlist with that arm removed (trap 1).

**A limit, enforced at the process.** The child sets `resource.RLIMIT_CPU` and `RLIMIT_AS` on
itself before it executes anything, and `core` imposes a wall-clock timeout on the call; a
generator that loops for ever is a refusal, never a hang — M0.4's rule that
a hang is not a failure unless one is imposed; which refusal it is, the paragraph below
amends. The three numbers are the compiler's
configuration, named as numbers rather than as rules (ADR 0022 §3's phrasing), and PR 4 chooses
them where it can watch a loop refused by each.

**`PYTHONHASHSEED=0`** on the child, and nothing else in its environment that the DSL could
observe: it observes nothing, by the paragraph above, and the variable is there for the
interpreter's own dicts and the belt-and-braces of decision 3. **Amended 2026-10-01, in M4 PR
4: the child owns it rather than trusting its spawner.** A hash seed is fixed before any of
the child's own code runs, so it cannot be set from inside — and a lock that works only when
somebody else remembers it is the half-lock this decision exists to refuse. A child started
without it therefore **re-executes itself** with it; `core` sets it in the environment and the
re-exec never happens on that path, so the cost (one more interpreter start) is paid only by a
shell driving the compiler by hand.

**Not a security boundary against a hostile author**, and "What M4 will not claim" says so.
An `ast` allowlist has been escaped before, and the threat this is built against is a sloppy
author — a model reaching for `import random` and `math.sin` — rather than an adversary. The
process boundary is what a real sandbox would be built on later (a seccomp filter, a namespace,
a separate user), and nothing here would have to be undone to add one. A generator that a
hostile author writes runs with the permissions of the person who chose to compile it, which is
the same sentence that is true of every plugin the engine hosts.

~~**An assumption, named.** That `resource` limits set inside a child started by `uv run` bind
the interpreter that actually runs the source, and not only a launcher in front of it. It is
tested in PR 4 by a loop watched refused; if the launcher gets in the way, the command `core`
is told is the interpreter itself.~~ **Measured 2026-10-01 in M4 PR 4, and they bind.** The
same unbounded `while True`, through the console script and through
`uv run --no-sync --project compilers/generative escribass-generative`, was refused at
**2.04 s** either way at `--cpu-seconds 1` and at **6.05 s** at the default 5 — the budget plus
one interpreter start, with no launcher in the way; the memory limit refused a loop allocating
eight megabytes a turn in **0.28 s** at `--memory-mb 300`. `core` may be told either command.
The limits are the child's own `setrlimit` on itself, so what the measurement actually rules
out is a launcher that re-execs in front of the interpreter — and `uv` does not.

**The third number, `core`'s, added 2026-10-01 in M4 PR 5.** The wall clock this decision asks
`core` to impose is **a minute**, and it is a field on the sandbox rather than only a constant.
A minute because it has to sit above everything the child's own limits allow — five CPU seconds
soft, five more before the hard one, plus an interpreter start, measured at 6.05 s on the
default budget — and a wall clock below that would turn the child's own diagnostic into
`generator_timeout` and lose the line number with it. A field because a bound nothing has ever
been seen to fire is a bound nobody should believe (trap 1): the test that watches a sandbox
which never answers shrinks it to 400 ms, and with the bound removed that test does not fail,
it **hangs**, which is the defect's own shape.

**The two numbers, and where the budget starts.** `CPU_SECONDS` is 5 and `MEMORY_MB` is 512,
both flags as well as defaults, so a test can watch a loop refused by each without waiting out
the default or allocating half a gigabyte on a runner. The CPU budget is counted **from the
moment the limits are imposed** rather than from process start: `RLIMIT_CPU` counts the whole
process, and a number that silently meant "five seconds minus however long this interpreter
took to start" would fire before the call arrived on a slow enough machine.

**How the limit crosses, which the wire decides and not this section.** `CompileResponse` has
one refusal arm, a `Diagnostic`, whose rule is `core`'s `generator_error` (ADR 0026 §1) — so
the soft `RLIMIT_CPU` is **caught** in the child, as `SIGXCPU`, and answered as an ordinary
diagnostic carrying the line the generator was on and the number it exceeded. It reaches a
caller as `generator_error` with that text, not as `generator_timeout`, which stays the
wall-clock one `core` imposes (decision 7, amended). The hard limit a few seconds above it is
what is left if answering itself runs away, and that is a child killed without answering,
which is `generator_failed`.

### 5. A compile writes the target clip's notes and `compiled_hash`, in one entry, with ids minted by `core`

One entry under the tool name `compile_generator`: the target clip's notes **replaced whole**,
`set_notes` semantics (M0.3), and `Generator.compiled_hash` set (ADR 0025 §1). Every note's
`provenance` is the call's, decided by `prepare` on the way in (ADR 0021 §1), which is what
`song.proto`'s comment has said since M0.1 — "carrying this generator's compile call in their
provenance". Note **ids are minted by `core`'s injectable source** (ADR 0001 §5), never by the
sandbox, which returns notes with §4.3 blank (trap 9): that is what makes `--seed-ids` a pure
function of the script and the `generators` golden reproducible, and it means every note id
changes on every compile — nothing references a note id today, and the day something does (an
expression on the wire, a selection held across a compile) this is where it breaks.

**A dry run compiles and answers with the diff**; the code view's *Compile* is a dry run and its
*Apply* is the commit (ADR 0017 §4). **One compile per click, never per keystroke** (trap 8): a
keystroke is a draft in the editor's buffer, Save is one `apply_patch` on
`/generators/{id}/source`, and nothing is compiled that the document does not hold (trap 7).
The compile's output then goes through the validator like any `set_notes` — a pitch of 200 is
`pitch_out_of_range` on the proposal, a note past `clip.length` is refused by §4.4's "notes
inside clip bounds" — so the DSL needs no rules of its own about the model's ranges.
**Amended 2026-10-01, in M4 PR 4, splitting that sentence where its two halves actually fall.**
`note(pitch, start, length, velocity)`'s ranges are its own **argument contract**, written in
decision 3's table and listed in decision 7 as a `generator_error`, and the child refuses them
— because the child is where the **line number** is, and `pitch_out_of_range` on a note a
person cannot locate in their source is the diagnostic Plate 3 complains about. So a pitch of
200 is refused by the child first and the validator never sees it. What stays entirely the
validator's is the **clip's bounds**: a note past `clip.length` is §4.4's structural rule about
a document, not an argument's range, and the child refuses no note for where it lands.

**The target is a note clip.** `Generator.target` is `track_id | clip_id` (ADR 0002 §3) and the
validator accepts either. In M4 a compile writes a clip, and a generator whose target is a
track, or an audio clip, is refused `target_not_note_clip` — valid and uncompilable, the shape
ADR 0007 §6 gave *valid and unrenderable*. What a track target should mean (a clip the length of
the song? one per section?) is not decided against no consumer; a ledger row carries it on the
first request.

**A hand edit to compiled notes is allowed** (§2.4: flat events are primary) and is the log's to
show, not the hash's: *stale* describes the source's inputs, never the notes. The next compile
replaces them.

### 6. "Stale" is one hash, computed in `core` over exactly what crosses to the sandbox

`compiled_hash` is the **SHA-256, by the project store's one hasher (`sha2`, the one that hashes
an asset and a prompt), over the canonical JSON of the `CompileRequest` the compile was made
from** (ADR 0026 §1) — the kind, the source, the seed, the params, the tempo events, the
signature events, the sections and the target clip's bounds, with §4.3 blanked and the
`repeated` fields in tick order. Written by the compile; recomputed by whoever asks; **unequal is
stale, empty is never compiled.** Computed in `core` and nowhere else, because a hash computed
in Python and compared against one computed in Rust disagree the first time one canonicalises a
key order differently — M0.1's serialisation boundary with a status word as the symptom (trap 3).

Defining it over the request rather than over a list of fields is what keeps the definition one
sentence and keeps it honest: **whatever the sandbox is handed is what the hash covers, and
nothing it is not handed can make a compile stale.** A signature change makes a generator stale
because `bar(n)` reads the signature map, and the map crosses; a track rename does not, because
nothing crosses. `toolchain_version` is deliberately **not** in the request and not in the hash:
a toolchain is a *pin*, compared at compile (ADR 0027 §2), and a project moved to a newer DSL is
refused `toolchain_mismatch` until a person re-pins, at which point recompiling is right — a
hash that went stale on the same event would say the same thing twice.

**Who reads it.** The model's bar view says `never` or `compiled` from the hash's emptiness and
does not say `stale` (ADR 0026 §4), because the view is computed in `ai` over the `Song` and `ai`
has no hasher — one hasher, in `core`. The window's code view learns `stale` from `core`,
through the one dispatch, as the answer to a **dry run of `compile_generator`**: `core` computes
the request's hash first and, when it equals `compiled_hash`, answers *up to date* with an empty
patch and **spawns nothing**; when it differs, it compiles and answers with the diff. So a status
read on an up-to-date generator costs no process, and one on a stale generator costs the compile
a person is about to ask for anyway, with its diff already on screen. The projection golden sees
the document's fields — source, seed, toolchain, whether `compiled_hash` is empty — and not the
status word, which is a tool's answer and is asserted in the host's tests (trap 7).

### 7. Three kinds of failure for a compile, split where ADR 0022 §3 split them

- **A refusal**, `valid = false`, caller-fixable, fed back whole to a model and counted against
  the three (ADR 0022 §3): `generator_error` — the allowlist refusing a node or a name, an
  exception during execution, a non-integral tick, a pitch or velocity out of range — with
  `path` `/generators/{id}/source` and a message carrying the child's `line:column` and text, so
  "the same text the user reads is what the LLM retries against" (Plate 3); `generator_timeout`,
  ~~the limit or the wall clock~~ **the wall clock** — amended 2026-10-01, in M4 PR 4: the
  child's `RLIMIT_CPU` is caught and crosses as a `Diagnostic`, because the wire has one
  refusal arm and `core` names it `generator_error`, so the *limit* arrives as a
  `generator_error` whose message carries the limit and the line, and `generator_timeout` is
  what `core` says when nothing came back at all — the author's to fix too;
  `target_not_note_clip`; `generator_unknown`.
- **An operator error**, `Err`, ending a model's turn at the host: `generator_missing` (no
  `--generator`), `toolchain_mismatch` (ADR 0027 §2), and `generator_failed` — the child would
  not start, printed no socket line, or exited non-zero without answering, carrying the tail of
  its stderr (ADR 0013 §3's `engine_failed`, one child over).
- **Nothing here is the third kind.** A provider failure is `ai`'s and a compile involves no
  provider.

### 8. Where it lives, how it is tested, and what CI runs

`/compilers/generative/` — §13 has named `/compilers` and this subdirectory since the first
draft, so no directory ADR — a Python package under `uv` with its own `pyproject.toml`,
depending on `escribass-schema` and `escribass-proto` editable as `ai/` does (M3 PR 5's shape),
with a `.python-version` at §17's **3.12.12**, the same interpreter pin the sidecar carries. It
implements the `Generate` server the pinned `betterproto2-compiler` generates (ADR 0026 §1) and
exposes one console script `core` can be told as its `--generator`.

**Tested three ways, and each can fail** (trap 1):

- In `compilers/generative/tests/`, under `unittest`: every forbidden construct fed and watched
  refused with its line; a loop that never ends refused by each limit; the same seed twice giving
  the same bytes; a changed seed changing the notes; and the spike's digest — the `rng`,
  `Fraction` and integer block that gave `1730edd3…` on five interpreters — as a unit test that
  fails if the pinned interpreter ever gives another, which is question 3's platform-free claim
  as a check rather than a sentence.
- In `tests/`, the determinism suite's sixth script, `generators`, driven over both transports
  and against committed bytes (M4 PR 5), whose golden carries `lock.json`'s `toolchains` block
  (ADR 0027 §1). The staleness guard learns the third child (trap 5): a `uv sync` that was not
  run passes every golden otherwise.
- The tests join the `checks` job **in the pull request that creates the directory** (M3 trap
  17; M4 trap 6), which also adds `compilers/generative/` to the `engine` and `app` gates'
  exclusions and teaches `tests/determinism.rs`'s gate test the new line.

## Alternatives considered

**Where it runs (question 1)**

| Alternative | Rejected because |
|---|---|
| Inside `ai`, as §7.1 said | `ai` is a server holding no session (ADR 0020 §1); an MCP client with no `ai` running would have nothing to call; and the tier that talks to a model would execute code |
| Inside `core`, through an embedded interpreter | A Python runtime linked into the Rust crate every process embeds, for a limit that only a process boundary enforces |
| A warm interpreter kept between compiles | A compile that can depend on the compile before it — ADR 0008 §2's objection — and a limit that kills the child costs a restart |

**The transport (question 2)**

| Alternative | Rejected because |
|---|---|
| stdio with EOF framing | M2 PR 9 deleted that path rather than keep two transports; the messages need a `.proto` either way |
| argv and files | A source on a command line and notes in a temporary file, with a parser at each end that `buf breaking` never sees |

**The language (question 3)**

| Alternative | Rejected because |
|---|---|
| Full Python with `import` blocked | `math.sin` is one `Attribute` away, `float` is a literal, and the platform-free claim dies on the first `libm` call a golden sees |
| A language of our own | A second grammar a model has never seen, for no gain over a subset of one it has |
| Floats, rounded at the boundary | The rounding is where two platforms disagree, and ADR 0018 §4 already refused it for the axes |
| `set` allowed under `PYTHONHASHSEED=0` alone | A default nobody can see; the day the child runs without the variable, every golden with a set moves |
| `rng.random()` allowed | Returns a float |

**The sandbox (question 4)**

| Alternative | Rejected because |
|---|---|
| Claim a security boundary | An `ast` allowlist is not one, and a claim that is false about hostile input is worse than none |
| No limit, since the language cannot reach a clock | A `while True` reaches no clock and never ends; M0.4's rule needs an imposed failure |

**What a compile writes (question 5)**

| Alternative | Rejected because |
|---|---|
| The sandbox returns an RFC 6902 patch | A fourth language with a patch implementation in it, which ADR 0012 §2 refused for the frontend |
| The sandbox mints ids | `--seed-ids` stops being a pure function of the script, and every golden moves with the child's id source |
| Compile per keystroke, as a drag dry-runs per position | A drag's dry run costs 1–3 ms and writes nothing (ADR 0017 §5); a compile's spawns an interpreter (trap 8) |

**Stale (question 6)**

| Alternative | Rejected because |
|---|---|
| A timestamp | "Stale" would depend on a clock, in a compiler CLAUDE.md #3 names |
| The hash computed in the sandbox and returned | Two hashers, one in each language, disagreeing on the first canonicalisation difference (trap 3) |
| `toolchain_version` in the hash | Says twice what `toolchain_mismatch` already says once, and stales a generator on an event a person has to act on anyway |
| A `get_generator_status` read tool | A fourth read RPC for a word one dry run already answers without spawning when it is *up to date* |

## Consequences

- **M4 PR 4** (`m4.4-generator`) creates `compilers/generative/`, the allowlist, the namespace,
  the seeded `rng`, the limits, the `Generate` server, its tests and its `checks` step. **No
  `core` change**: it is a process a shell can drive. **Done 2026-10-01**, with four things
  this ADR had wrong or unsaid, each amended above and dated: `/` cannot be `Fraction`'s job
  and is out of the language (§3); the hash seed is the child's own, by re-exec, rather than
  its spawner's (§4); the CPU limit crosses as a `Diagnostic` and so as `generator_error`,
  leaving `generator_timeout` to the wall clock (§4, §7); and `note`'s ranges are refused by
  the child, where the line number is, while the clip's bounds stay the validator's (§5). The
  limits bind under `uv run`, measured. No external dependency was added.
- **M4 PR 5** (`m4.5-compile-tools`) is `core`'s: `--generator`, the third child in
  `engine.rs`'s shape, the two tools ending in `Session::run` as every tool does, the refusals
  above, the hash, and the `generators` script — **the silent PR**: no existing golden moves, and
  any byte that does is named. It measures the process cost per compile and writes it into the
  plan. **Done 2026-10-01**, and no existing golden moved: the five determinism goldens, the
  four render WAVs and `app/tests/projection.golden.json` are byte-identical to `main`'s, and
  `tests/determinism/generators/expected/` is the only new bytes. Three things this ADR had
  unsaid, each amended above and dated: the per-compile cost is 280–370 ms (§1); the child's
  stdin is a pipe `core` holds and emphatically not `Stdio::null()` (§2); and the wall clock is
  a minute and is a field, so a test can watch it fire (§4). The type `core` spawns it with is
  called `Sandbox` and not `Generator`, because `escribass_schema::song::Generator` is the
  document's entity and the flag and every rule id keep saying `generator`. No external
  dependency was added and no paid call was made.
- §7.1 is rewritten: the subprocess is `core`'s, the DSL is this subset, "sandbox" means this;
  §3's tier table stops saying the generative compiler runs in `ai`.
- `docs/plan.md`'s ledger gains a row for what a track target means, and trap 2 gains the
  measurement.
- **What this rests on that is unmeasured**: ~~the per-compile process cost (PR 5)~~ —
  **measured 2026-10-01** (§1, amended) — ~~and that
  the `resource` limits bind under the launcher (PR 4, watched)~~ — **measured 2026-10-01, and
  they do** (§4, amended). What it does **not** claim:
  platform-free bytes on a second platform — the arithmetic permits the claim and only a second
  platform running the `generators` golden makes it; until then it is Linux x86-64 like
  everything else.
