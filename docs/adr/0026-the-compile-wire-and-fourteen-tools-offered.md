# ADR 0026 — The compile crosses one wire, the model is offered fourteen tools, and a diagnostic is a refusal

- **Status:** Accepted (2026-09-25)
- **Affects:** `proto/generate.proto` (new) and `proto/song_tools.proto` (M4 PR 3), with
  `proto/`'s Rust, TypeScript and Python regenerated; `core/src/call.rs` (`OFFERED`, M4 PR 6);
  `ai/src/escribass_ai/view.py` and its golden (M4 PR 6); `docs/specs.md` §5, §6 and §13;
  ADR 0022 §1 and §3 and ADR 0018 §1, amended in place; ADR 0015 §1, amended with a hazard
- **Builds on:** ADR 0006 §1 (one `ToolResult` for every tool that produces ops) and §4
  (requests reuse `song.v1` types by value, `core` overwrites §4.3); ADR 0007 §2 (leaf messages
  cross by value with §4.3 blanked; structural messages plan-local and `repeated`); ADR 0013 §2
  and ADR 0020 §3 (a service's whole shape lands in one pull request so `buf breaking` compares
  it once — M2 trap 12, M3 trap 14, M4 trap 4); ADR 0022 §1 (`OFFERED` is data beside
  `IMPLEMENTED`) and §3 (three kinds of failure, split once each); ADR 0018 §1 and §2 (the bar
  view, and that withholding is the lever); ADR 0024 §2, §5, §6 and §7 (the transport, what a
  compile writes, the hash, the failure kinds)
- **Recorded in:** `docs/specs.md` §5, §6, §13 and §15.

## Context

ADR 0024 decides that a compile is one call to a child over gRPC and what the DSL is; this ADR
writes down what crosses, which tools carry it, what the model is offered, and what the model
reads about generators. Questions 8, 12 and 13 of the plan. It is one ADR because the four are
one wire seen from four sides, and because M4 PR 3 lands `proto/`'s whole M4 shape in one
change — `generate.proto` and the two `SongTools` RPCs — so `buf breaking` compares it once
against a `main` that has not moved (trap 4).

ADR 0022 §1 offered the model twelve of twenty-five tools and said in its own Consequences that
the twelve were not claimed to be "the right twelve for M4, which adds the compilers' tools and
decides them then". This is then.

## Decisions

### 1. `proto/generate.proto`: one service, one RPC, request and response reusing `song.v1` by value

```proto
package escribass.generate.v1;
import "song.proto";

service Generate {
  rpc Compile(CompileRequest) returns (CompileResponse);
}

// What the sandbox is handed, and all of it: never the Song (ADR 0024 §1). Leaf messages by
// value with §4.3 blanked (ADR 0007 §2), repeated fields in tick order, ties by name. Its
// canonical JSON is what Generator.compiled_hash is the SHA-256 of (ADR 0024 §6), which is why
// nothing is in here that a compile does not read, and nothing a compile reads is elsewhere.
message CompileRequest {
  escribass.song.v1.GeneratorKind kind = 1;
  string source = 2;
  uint64 seed = 3;
  map<string, string> params = 4;
  repeated escribass.song.v1.TempoEvent tempo = 5;
  repeated escribass.song.v1.TimeSignatureEvent signature = 6;
  repeated escribass.song.v1.Section sections = 7;
  int32 clip_start_tick = 8;
  int32 clip_length_ticks = 9;
}

message CompileResponse {
  // The child's own, stated by the child on every answer and by nothing else (ADR 0027 §1):
  // the DSL's version and the interpreter's. core compares; the child never refuses on them.
  string dsl_version = 1;
  string python_version = 2;
  oneof result {
    Notes notes = 3;
    Diagnostic diagnostic = 4;
  }
}

// The clip's whole note set, set_notes semantics, with id, provenance and version EMPTY:
// core mints the ids and prepare stamps the provenance (ADR 0024 §5).
message Notes {
  repeated escribass.song.v1.Note notes = 1;
}

// One refusal, the author's to fix. Line and column are the source's; the rule is core's
// (generator_error), so the child names no rule.
message Diagnostic {
  int32 line = 1;
  int32 column = 2;
  string message = 3;
}
```

Three things the shape decides. **`toolchain_version` does not cross**: the child states its
own version in every response, `core` compares it against the document's and the lock's (ADR
0027 §2), and the request stays exactly the set of things a compile reads — which is what lets
`compiled_hash` be defined as the hash of the request and nothing else (ADR 0024 §6). **`seed`
is `uint64`**, so it crosses JSON as a string (ADR 0002 §1) and reaches `random.Random` as the
integer; the fixture's seed is above 2⁵³ on purpose and PR 4's digest test uses one above 2⁶³
(trap 10). **No `Song`, no track, no clip id**: the sandbox is a compiler and is handed what it
compiles; a section's id, provenance and version are blanked as a plan's are.

One RPC and no `Version` call, because the version travels on the answer to the only call there
is; a second RPC to ask a question the first already answers is ADR 0020 §3's `Jobs` one size
down.

### 2. `define_generator` and `compile_generator` on `SongTools`, and no `set_generator_source`

```proto
// Adds a Generator (§4.2, layer 1). Compiles nothing: one compile per click (ADR 0024 §5).
// toolchain_version is not a field — the first compile writes it from what the child reports
// (ADR 0027 §1). The target may name a track or a clip, as the schema allows; in M4 a compile
// writes a note clip and refuses the rest, target_not_note_clip (ADR 0024 §5).
message DefineGeneratorRequest {
  escribass.song.v1.GeneratorKind kind = 1;
  string source = 2;
  uint64 seed = 3;
  map<string, string> params = 4;
  oneof target {
    string track_id = 5;
    string clip_id = 6;
  }
  bool dry_run = 7;
}

// Compiles a generator: spawns the sandbox, replaces the target clip's notes whole, sets
// compiled_hash and toolchain_version, one entry (ADR 0024 §5). A dry run whose inputs hash
// to compiled_hash answers "up to date" and spawns nothing (ADR 0024 §6).
message CompileGeneratorRequest {
  string generator_id = 1;
  bool dry_run = 2;
}
```

Both return `ToolResult` (ADR 0006 §1: every tool that produces ops). **Editing a source is
`apply_patch` on `/generators/{id}/source`**, as the mixer writes `Mix` and for the same reason:
a `set_generator_source` is one RPC and one tool function for a `replace` the raw pipeline
already expresses, and the typed-tools ledger row's trigger — the first measurement in which a
model gets the RFC 6902 wrong where a typed tool would not have let it — has not fired (ADR
0022 §1). `seed` and `params` are edited the same way.

**`define_generator` compiles nothing.** A model that defines and compiles in one turn makes two
calls, which is one more than it would like and one fewer process than compiling on define
would cost every definition a person then edits before compiling. A dry run of
`define_generator` is the ordinary validator pass over the added entity.

**The refusals**, each a `Violation` with its rule (ADR 0024 §7): `generator_unknown`,
`target_not_note_clip`, `generator_error` with `line:column` and the child's text,
`generator_timeout`; and the operator errors `generator_missing`, `toolchain_mismatch` and
`generator_failed`. **Wait for M5**, and the proto comment says so instead of saying "M4":
`define_instrument_source` and `compile_instrument`. `set_form` waits on `FormRule`'s event
(ADR 0023 §6).

**One state the wire cannot express and does not try to**: a compile in progress. A compile is
one call that blocks for its length, bounded by the timeout; a progress stream for a call that
takes tens of milliseconds to a few seconds is the `Jobs` service ADR 0020 §3 refused. If a
generator ever takes long enough to want progress, the limit is what says so first.

### 3. `OFFERED` gains two — fourteen of twenty-seven — and a diagnostic is a refusal fed back and counted

`define_generator` and `compile_generator` join `OFFERED` in `IMPLEMENTED`'s order (ADR 0022
§1): **fourteen offered, thirteen withheld**, the thirteen unchanged with their reasons. Both
produce ops on the document and nothing else, and both are executed against the proposal (ADR
0019 §1) — a compile on a proposal spawns the sandbox and writes the fork's clip, and a person
applies the whole once. The schemas the host hands `ai` are the descriptor's own filtered by the
list, with `dry_run` removed, as before; **ADR 0022 §1 is amended in place** to say fourteen.

**A compile that fails is caller-fixable and goes back whole.** `generator_error` and
`generator_timeout` are `valid = false` with the child's `line:column` and text in the message,
fed back to the model as the call's result exactly as any refusal is, and **counted against the
three refused calls per turn** (ADR 0022 §3) — the plan's default, taken, because "the same text
the user reads is what the LLM retries against" (Plate 3) is §6.1's loop doing its job, and a
compile error is the one refusal in this repository whose message is the whole fix. Nothing new
is split: `generator_missing`, `toolchain_mismatch` and a child that died end the turn at the
host as every operator error does, and no compile involves a provider. **ADR 0022 §3 is amended
in place** with the sentence that a compile's diagnostic is the first kind.

**What is not measured, and said so.** Whether a model writes the DSL — at all, or reliably — is
U10, deferred until the DSL exists (ADR 0023 §7). M4 PR 6's transcript, in which the scripted
model defines and compiles a generator, is **hand-written** and says so in its own file as
`four-refusals.json` does; the loop is designed for a human author and a model that retries
against the diagnostic, which the three-refusal budget already bounds.

### 4. The bar view's generator line carries the seed and whether it has ever compiled — never `stale`

ADR 0018 §1's view prints `generators: none` or one entry per generator — id, kind, target. From
M4 the model is offered `compile_generator`, so it has to see what there is to compile. **Each
entry gains the seed and one word, `never` or `compiled`**, read from `compiled_hash`'s
emptiness:

```
generators: 01M1FPMP00000000000000002A python → 01M1FPMP00000000000000001Q seed 7 compiled
```

**It does not say `stale`**, and the reason is trap 3 and not modesty: the view is computed in
`ai` over the generated `Song` (ADR 0018 §3), `ai` has no hasher and must not grow one, and
*stale* is `core`'s comparison (ADR 0024 §6). A model that changed a source and did not compile
knows it did; a model that wants fresh notes compiles. **ADR 0018 §1's table is amended in
place** — the `Generator` row carries `seed` and the status word, and still abstracts `source`,
because code a model reads back is not the grammar. The view's golden moves by that line and
nothing else, in M4 PR 6, and the key-reversed twin moves with it.

`source` stays abstracted for the reason ADR 0018 §1 gave and one more: a model that wants to
edit a source it did not write would read it back through… nothing, since `get_song` is
withheld (ADR 0018 §2). That is a gap and it is named rather than closed: the escape ADR 0018
§2 allows is to add to the projection, never to hand back the JSON, and printing every
generator's source in every prompt is the wrong size of addition until a measurement says a
model needs it. Deferred with that trigger.

### 5. §13 names `Generate`

`/proto`'s line gains `Generate` beside `SongTools`, `Render`, `Preview` and `Assistant`, in the
same change as the service, as ADR 0020 §3 did for `Assistant`.

## Alternatives considered

**The wire (question 13)**

| Alternative | Rejected because |
|---|---|
| Send the `Song` and let the sandbox find its inputs | A compiler handed a document it must navigate, a fourth reader of the model's structure, and a hash over more than a compile reads |
| Carry `toolchain_version` in the request | Either the child refuses on it — an operator error crossing as a diagnostic — or it is ignored; and it would put a pin inside the hash (ADR 0024 §6) |
| A `Version` RPC | A second call to learn what the first call's answer already carries |
| The child returns a patch | RFC 6902 in a fourth language (ADR 0012 §2) |
| A streaming `Compile` with progress | `Jobs` one size down (ADR 0020 §3); the timeout bounds the wait |

**The tools (question 8)**

| Alternative | Rejected because |
|---|---|
| `set_generator_source` | The typed-tools row's trigger has not fired; `apply_patch` on the path is what the mixer already does |
| `define_generator` compiles as it defines | A process per definition a person then edits before compiling, and two things in one entry that a dry run cannot show apart |
| A `toolchain_version` field on `define_generator` | A model cannot know it and a person should not have to; the first compile writes what the child reports (ADR 0027 §1) |
| Withhold `compile_generator` and let a person press Compile | The one tool whose refusal is a diagnostic the model can act on, withheld from the author most likely to need it |
| A compile diagnostic as its own kind, outside the three | A fourth kind for a refusal that is exactly the first kind's shape: `valid = false`, a message, a call the caller can fix |

**The view (question 12)**

| Alternative | Rejected because |
|---|---|
| Say `stale` in the view | Needs a hasher in `ai` — the two-hasher trap in the one place it would be invisible |
| Print each generator's source | Every prompt carries every generator's code, for a model that in M4 has not been measured writing any (U10) |
| Say nothing new | A model offered `compile_generator` with no way to know what exists |

## Consequences

- **M4 PR 3** (`m4.3-proto`): `proto/generate.proto` whole, the two RPCs and their requests on
  `SongTools`, `proto/`'s three language targets regenerated, and the comment at the foot of
  `song_tools.proto` moving its milestone. `buf breaking` sees `proto/`'s whole M4 shape once
  (trap 4) — locally against `main`, until U2 takes effect. The generated Python server is what
  `compilers/generative` implements, so `proto/tests/test_generated_python.py`'s identity check
  already covers it.
- **M4 PR 6** (`m4.6-model`): `OFFERED` and its subset test, the view's generator line and its
  golden, a compile diagnostic fed back as a refusal and counted — watched failing first — and a
  hand-written transcript in which the model defines and compiles.
- §5 names the two tools and their refusals; §6.1 says fourteen; §13 names `Generate`.
- ADR 0022 §1 and §3, and ADR 0018 §1, amended in place and dated.
- **A hazard for M5, written down now because it bites a decision M2 made.** The spike read
  `cmaj_CLAPPlugin.h:524`: a CLAP parameter's id is `endpointHandle`, Cmajor's endpoint handle
  in **declaration order** — so inserting an input above another in a source **renumbers** the
  parameters below it, and a `ParamRef` (ADR 0015) over a compiled device is keyed to something
  the source's *shape* decides rather than its names. A `ParamRef` naming a device's parameter
  `2` means `Detune` today and `Gain` after an edit that adds an input above it, and nothing in
  the document would say so. **ADR 0015 §1 is amended in place with the hazard and its
  trigger**: the first `ParamRef` that names a compiled device's parameter, which is M5's export
  record deciding what a `ParamID` for a Cmajor device *is* — an ordinal, or a name the record
  maps to one. Not M4's, and not decided here against no artefact.
- The ledger gains a row for the model reading a generator's source, on the trigger in §4.
