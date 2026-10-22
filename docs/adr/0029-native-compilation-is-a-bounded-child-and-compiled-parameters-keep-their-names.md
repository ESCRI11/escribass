# ADR 0029 — Native compilation is a bounded child, and compiled parameters keep their names

- **Status:** Accepted (2026-10-22)
- **Affects:** `proto/export.proto` and `proto/song_tools.proto` (M5 PR 4); `engine/src/main.cpp`
  (inspection and the loader, M5 PR 6); `compilers/dsp/` (M5 PR 7); `core/src/session.rs`,
  `core/src/render.rs`, `core/src/validate.rs` and a new `core/src/export.rs` (M5 PR 8); the DSP
  code view in `app/` (M5 PR 10); `OFFERED` (M5 PR 11); ADRs 0007 §1, §2 and §6, 0008 §1, 0013
  §2, 0015 §1, 0019, 0022 §1 and §3 and 0026 §2, amended in place
- **Builds on:** ADR 0006 §1 and §2 (one `ToolResult` for what produces ops; caller and operator
  errors); ADR 0007 (the engine receives a resolved plan and never a `Song`); ADR 0008 (a fresh
  process per render, failure an exit code); ADR 0010 §4 (a device's parameter value is
  normalised `0..1`); ADR 0013 (preview); ADR 0015 §1 (the endpoint-ordinal hazard, recorded for
  this ADR); ADR 0019 (a proposal is a fork); ADR 0022 §3 (three kinds of failure); ADR 0024 §1,
  §2, §4 and §7 (a child `core` spawns, told its command, naming its socket, bounded); ADR 0026
  §1 and §2 (one RPC, and the two tools §5 named); ADR 0028 (what an export is); `docs/plan.md`,
  "What the spike found, measurements 1–3 (run 2026-10-22, after U9)"
- **Recorded in:** `docs/specs.md` §5, §7.2, §8, §9, §13 and §15.

## Context

§5 has named `define_instrument_source` and `compile_instrument` since the first draft, and ADR
0026 §2 left both to M5. M4 built a compiler boundary that works: a child `core` spawns per
call, one gRPC call, a diagnostic the caller can fix, a result that becomes one patch, and a
project that renders without its compiler. A native export keeps that shape. It changes four
things about it.

- **It takes twenty seconds, not three hundred milliseconds.** The spike's 36 exports built in a
  median 21.2 s. A dry run that builds and an Apply that builds again would be forty seconds for
  one change, and two builds the person never compared.
- **Its result is a binary and an interface, not notes.** The interface is what a song's
  parameters refer to, and the spike measured how badly the obvious key for it behaves.
- **Its tools mislead in ways the generator's do not.** The converter exits 0 on an invalid
  model, `cmaj play --dry-run` exits 0 on a compile error, `cmaj render` exits 1 and says nothing,
  and `cmaj generate` exits 1 and leaves files behind.
- **What it builds has to load beside other things it built.** The spike found two ways it does
  not: every wrapped plugin deadlocked the engine, and two versions of one processor ran one
  binary.

The spike measured the interfaces this ADR decides, so none of them is a guess.

## Decisions

### 1. The DSP compiler is a fresh child `core` spawns per compile, told `--dsp <cmd>`

ADR 0024 §1's shape, unchanged in every respect but the work. **`compilers/dsp/` is a Python
driver** on the interpreter §17 already pins, using the generated server and `grpclib` the
generative compiler uses, and adding no dependency of its own. `core` is told its command as
`--dsp <cmd>`, and never searches for it. The child names its socket as the one `unix:<path>`
line of its stdout, serves one call and exits, and its stdin is a pipe `core` holds (ADR 0024 §2,
amended). It **orchestrates** `cmaj generate`, CMake and the C++ compiler as grandchildren. It is
not in `engine`, which stays schema-agnostic and links no compiler. It is not in `ai`, which has
no audio path and holds no session (CLAUDE.md #6; ADR 0020 §1).

**The converter is a library this driver calls**, not a second service. `compilers/neural/` is a
package the DSP environment depends on by path, selected when the device is a `ModelRef`, and
it ends in the same Cmajor export (ADR 0023 §3). One `uv` environment owns both. What it
converts is ADR 0032's.

### 2. One RPC, `Export.Compile`, and a bundle crosses back as bytes

```proto
package escribass.export.v1;

service Export {
  rpc Compile(CompileRequest) returns (CompileResponse);
}

// Everything a compile reads, and nothing else: never the Song, a project path, a patch or audio.
message CompileRequest {
  oneof input {
    string cmajor_source = 1;   // the source asset's text, UTF-8
    bytes onnx_model = 2;       // a self-contained graph (ADR 0032)
  }
  DeviceKind kind = 3;          // INSTRUMENT or EFFECT
  Recipe expected = 4;          // the approved recipe; the child builds with it or refuses
}

message CompileResponse {
  Recipe actual = 1;            // what ran, stated by the child (ADR 0027 §1's rule)
  oneof result {
    Bundle bundle = 2;
    Diagnostic diagnostic = 3;
  }
}

message Bundle {
  map<string, bytes> files = 1;        // relative path inside the .vst3 -> bytes
  repeated Endpoint endpoints = 2;     // from the generated code: identifier, handle, annotation
}

message Diagnostic {
  string stage = 1;   // "generate", "convert", "build" or "inspect"
  int32 line = 2;     // 1-based, in the source; 0 when the stage has no source position
  int32 column = 3;
  string message = 4;
}
```

The text is PR 4's to write. This is what it has to say. **The bundle crosses as bytes**, in a
bounded unary answer: when the call returns, the child that built it is gone, and a path into
its directory would name a file nothing owns. The spike's largest bundle was 2.6 MB. The
limit is a configurable byte bound, measured in PR 7 and set above gRPC's 4 MiB default. A
bundle that exceeds it is an error, never a truncation. **An unset `result` is not an answer,
and an empty bundle is not an export** — M1's `RenderResult::decode(&[]) == Ok`, met a third
time. **The endpoint table comes from the generated code**, which is the one place that maps an
identifier to its handle (decision 5). `core` owns every hash and every write. The child returns
files and facts, never a patch.

`ExportRecord` (ADR 0028 §2) lives in the same file, since it is assembled from this answer.
`Recipe` is ADR 0031's to fill.

### 3. Two tools, by §5's names, for instruments and effects alike, and one read

```proto
// Saves Cmajor source for a device: installs the source asset and points the reference at it,
// in one entry. Compiles nothing. Targets exactly one of: an existing Cmajor device (its
// source is replaced; an existing export stays and becomes stale), an instrument track
// (set_track_instrument's rules), or a new effect (add_effect's rules, index optional).
message DefineInstrumentSourceRequest {
  string source = 1;
  oneof target {
    string device_id = 2;
    string instrument_track_id = 3;
    EffectSlot effect = 4;
  }
  bool dry_run = 5;
}

// Compiles the saved source of a device into an export and points the reference at it.
message CompileInstrumentRequest {
  string device_id = 1;
  bool dry_run = 2;
}

// A read: the device's saved source, its export state and, when there is one, its record.
// Never compiles and never writes.
message GetDeviceSourceRequest {
  string device_id = 1;
}
```

**`define_instrument_source` is one gesture**: the source becomes an asset and the reference
names it. It reuses `set_track_instrument`'s and `add_effect`'s rules rather than restating them,
so a device it creates sits in a chain exactly where `add_effect` would have put it. A dry run
writes no asset. Apply writes the asset before the entry, so a crash between them leaves an
unreferenced blob, which is ADR 0004's commit point extended to assets and not a new store. **A
Faust target is refused**, `export_unsupported`, because Faust is import-only (§7.2).

**`compile_instrument` names a saved device and compiles what it holds.** No tool compiles an
unsaved editor buffer, so the source a compile read is always the one the song names. Freshness
is then a comparison of two hashes, and a draft never becomes an export by accident. A
`ModelRef` device — added by `add_asset` and the ordinary `set_track_instrument` or `add_effect`
with a `neural` reference — is compiled by the same tool, which selects the converter by the
reference's kind (decision 1).

**A dry run builds and holds one candidate**, and Apply commits what was shown. A dry run of
`compile_instrument` runs the child, verifies the bundle, assembles the record (ADR 0028 §2) and
keeps all of it in **session-owned scratch** outside the project. It also keeps the base values
it was made against: the device's `source_hash`, its current `export_hash` and the recipe. It
writes no asset, no lock and no entry, and answers with the patch (`export_hash` set) and a
summary that compares the new record's parameters with the old one's. A later non-dry-run
`compile_instrument` on the same device **applies the held candidate rather than building
again**, if its base values still hold at the paths its patch writes and its scratch blobs are
still present. Otherwise it is refused, `document_moved`, naming what moved, as a proposal is
refused (ADR 0019 §2). Measured exports are reproducible, so a second build would very likely
match the first, but "very likely" is not what a person approved. A source edited after the dry
run therefore never gets the old binary attached to it: its `source_hash` is one of the base
values. There is one candidate per session, and a new dry run replaces it. Applying installs the
members and the record (ADR 0028 §3), then the patch through `Session::run`, under
`compile_instrument`, as every tool ends.

**`get_device_source` is read-only**, and selecting a device never compiles it. It answers with
the source text, its hash, the derived state (ADR 0028 §4) and the record if there is one. That
is everything the DSP view shows and the smallest read a model needs to edit an existing source
(decision 8).

### 4. Failures keep their stage, the child is bounded, and so is a render

**Caller-fixable**, `valid = false`, fed back to a model whole and counted against the three
(ADR 0022 §3):

- `source_error` — `stage`, line, column and the compiler's own text;
- `graph_unsupported` — the node and operator named, before conversion (ADR 0032);
- `device_unknown`, `not_compilable` — a plugin or sampler device;
- `export_unsupported`, `export_pending`, `export_stale`;
- `param_unknown`, `param_incompatible` (decision 5);
- `state_unsupported` (decision 7);
- `document_moved`.

**Operator**, a `ProjectError` that ends a model's turn at the host and is never fed back:

- `compiler_missing` — no `--dsp`;
- `compiler_failed` — the child would not start, named no socket, died, answered nothing or an
  unset `result`, returned an empty bundle or one missing an expected file, or returned an
  endpoint table the inspected binary contradicts. A C++ build failure of generated code is
  here too: the toolchain's, not the author's;
- `compiler_timeout`;
- `recipe_mismatch` — what ran is not what was approved (ADR 0030 §4);
- `export_missing`, `export_corrupt` (ADR 0028 §4), and `target_unsupported`, an export built
  for another target (ADR 0030 §4);
- `runtime_missing` — the bundle will not load: a shared object of U5's closure is absent;
- `engine_timeout` — below.

**Status, diagnostic and expected outputs are all read, and none is trusted alone.** A stage
succeeds when its exit status is 0, its diagnostics contain no error, *and* the file it was
meant to produce exists. `cmaj generate` exits 1 with `file:line:col: error:` and still leaves
four helper files behind. `cmaj play --dry-run` prints an error and exits 0. The converter
exits 0 on an invalid model. A directory that is not empty proves nothing. The generator's "the
CPU limit's `SIGKILL` is the author's" rule is **not** copied: it does not survive a driver with
native grandchildren.

**Bounds.** One compile in flight per session. A **five-minute** wall clock, held as a field so a
test can shrink it, as ADR 0024 §4's minute is. Native build parallelism is bounded by the
number PR 5 measures against the CI runner's memory. The toolchain itself is built at setup,
outside this bound. **On timeout, host end-of-file or a person's cancel, the whole process group
the child leads is killed and reaped**, and its scratch is removed. A compiler grandchild that
ignores its parent's death is the case the tests drive. The generator's five CPU seconds and 512
MiB are not a native build's budget and are not reused.

**`render_export` gains a wall clock, which it never had** (amends ADR 0008 §1). The spike's
first render of a wrapped Cmajor plugin deadlocked the engine, and `core` waited until a person
killed the process. The bound is one minute plus ten times the plan's own length, held as a
field. Past it the engine's process group is killed and the call answers `engine_timeout`, an
operator error, with the tail of what the engine said, and never a WAV.

### 5. A compiled device's parameter is its **source endpoint identifier**

ADR 0015 §1 recorded the hazard and left the answer here. A Cmajor device's `Instrument.params`
and `Effect.params` keys, and a `ParamRef.param` that names one, are the **endpoint identifier**
the source declares — `gain`, not `2` and not `Gain`. The record maps each one to the actual
VST3 `ParamID`, its plain range and its default (ADR 0028 §2). A value is still the normalised
`0..1` of ADR 0010 §4, normalised against the record's range. The spike measured why neither
alternative key survives:

| Edit to the source | `ParamID` | Display name | Endpoint identifier |
|---|---|---|---|
| insert a control above | moves: a lane on 2 drove Drive | unchanged | unchanged |
| reorder | moves: a lane on 2 drove Tilt | unchanged | unchanged |
| rename the label | unchanged | changes | unchanged |
| change the range | unchanged, but every stored value means something else | unchanged | unchanged — and the record's range changes |
| rename the identifier | unchanged | unchanged | changes |
| remove | the next control takes its number | gone | gone |

The endpoint identifier is the one key that changes only when the source's author changes
*that* control. Bundled plugins keep their numeric `ParamID`s (ADR 0010 §4), because a bundled
plugin's ids are stable across the builds this project pins and a compiled device's are not.

**Every write resolves against the device's current record.** `param_unknown` fires for an
identifier the record does not hold. That holds whether the export is current or stale: a stale
export still describes the interface the song was written against. On a pending device,
`export_pending` fires for any key, because nothing yet says what the source declares.

**A new export is checked against the song before it replaces the old one.** At Apply,
`compile_instrument` compares the new record with the old for every identifier the song refers
to on that device — its `params` keys and every lane that targets it. An identifier that is
missing, or whose `min`, `max` or `step` changed, refuses the replacement, `param_incompatible`,
naming each identifier and what changed. A moved `ParamID`, a new title or a new default
refuses nothing: lowering resolves names afresh (decision 6), and a default is what a source
edit is entitled to change. The person repairs the references with `apply_patch` and compiles
again. A held candidate stays valid across that repair, because the repair does not touch the
path its patch writes.

**The mapping is read from the generated code and checked against the binary.** The child
reads `EndpointHandles` and the annotation in `programDetailsJSON`. `core` compares them with
what the engine's inspection reports — the same count, the same `ParamID`s and the same titles —
and refuses as `compiler_failed` if they disagree. The spike found them agreeing in all ten
exports it read three ways, so the check costs nothing while it holds and catches the day it
does not.

### 6. What reaches the engine: a resolved path, a class and concrete ids — loaded by export, not by name

**`core` lowers, and `render::compile` stays pure.** `core` builds a **verified artefact index**
outside the validator and the compiler: for each referenced, current export, the reconstructed
bundle's path (ADR 0028 §3), its class identity and its identifier-to-`ParamID` map. `compile`
takes it as an argument, as it takes the asset index (ADR 0007 §4), and a compiled device cannot
be lowered without it. The plan carries the path, the VST3 class id, concrete `ParamID`s and
normalised values. It carries no source, no record, no `lock.json` and no model graph (amends ADR
0007 §1 and §2).

**The engine gains a generic inspection mode**, `--inspect <bundle.vst3>`, reporting a bundle's
class, ports and parameters. Bundled `--scan` stays strict, refusing a component it has no
compiled-in commit for. The spike had to pass `surge_xt` as a stand-in component to scan a Cmajor
export at all, which wrote Surge's commit against it — a scan that lies about provenance is not
an inspection.

**The loader is keyed by export, not by `vendor/name`.** The manifest and the engine's `byId`
cache are keyed by `vendor/name`, so two exports of one source — an edit and its predecessor, on
two tracks or two branches — cannot both be named to it. The engine resolves a compiled device by
the plan's path and class id instead. JUCE's own identifier string already hashes the file's
path, and its `KnownPluginList` kept two same-named bundles apart in the spike.

**And the binary has to allow it.** Loaded into one process through the engine's own JUCE, the
second export of a processor name ran the **first** one's code. A `STB_GNU_UNIQUE` plugin factory
is keyed by the generated C++ class, and glibc binds it process-wide whatever `RTLD_LOCAL` says.
That held even with a different CLAP id and VST3 class id. So every export is compiled with
`-fvisibility=hidden -fvisibility-inlines-hidden -fno-gnu-unique` and linked with `-z nodelete`.
The last is required too: without it JUCE's `dlclose` unmapped the library and the engine
segfaulted in `exit()` after answering. With all four, each bundle ran its own code in either
load order and the engine rendered and exited cleanly. The flags are recipe facts and ADR 0031
records them. This decision is what they are for.

**The recipe writes the patch metadata.** Its `ID` is derived from the device's input hash, so
two different sources never share a VST3 class id, which the wrapper derives from the `ID` as a
SHA-1 name GUID. Its name comes from the source's main processor. Its kind comes from the
request. None of it lives only in a dialog or a process argument.

### 7. Parameter-only devices, and a preview replaced by a fresh process

**A compiled device with non-empty `state` is refused**, `state_unsupported`, by the validator.
Nothing produces opaque state for an export in v1, and no compatibility contract says what state
written against one export means to the next. This is not the deferred preset and state-storage
work firing. It is one device kind declining a field until a producer exists.

**A preview whose plan changes an export identity is a fresh preview process**, started at the
transport tick the person asked for. That covers a device compiled again, or a branch switched
to another export. A plan equal to the one playing still builds nothing (ADR 0013 §2). Opaque
plugin state is not carried across, and needs no carrying, by the decision above. A mapped `.so`
is never overwritten, because a cache path names one export for ever (ADR 0028 §3). **This was
not measured**: preview needs an audio device and the spike's machine has none. What it rests on
is the in-process measurement in decision 6 and the plan-equality rule M2 measured.

### 8. Proposals stage their assets, the editor never compiles on selection, and the model waits for both

**A compile inside a proposal builds into scratch the session owns**, outside the project root a
proposal's clone shares (ADR 0019). Nothing reaches `assets/` or `lock.json` while it is a
proposal. On Apply, the assets the final patch reaches are promoted before the entry, and the
lock is written by the common path (ADR 0030 §3). Reject, an Edit that drops the reference, a
failure and a cancelled turn all remove the scratch. `add_asset` stays withheld from the model:
a fork's root *is* the real project's and `add_asset` writes to it straight out (ADR 0019,
Consequences), beside ADR 0022 §1's reason that a model has no bytes to give it. This decision
does not change either.

**The DSP view reuses `app/src/code.tsx`**, with CodeMirror and the pinned lint package. A draft
is local text. Save is `define_instrument_source`. Export is a dry run of `compile_instrument`,
then the diff, then Apply. Diagnostics sit at the stage and line they name. Its status comes
from `get_device_source`. Nothing compiles because a device was selected, because a twenty-second
compile on a click is the defect M4's status dry run would have been at native cost. No hasher
lives in TypeScript or in the bar view (ADR 0024 §6).

**The model is offered `define_instrument_source`, `compile_instrument` and `get_device_source`
only when proposal staging and cancellation are proved** (U8; PR 11). A `source_error` is the
first kind of failure, fed back and counted. Every operator error above ends the turn. No file
path, compiler flag, shell command or binary is anything the model can supply. `set_param`
stays withheld, and nothing here offers a plugin catalogue.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Compile inside `engine` or `ai` | `engine` would link a compiler and `ai` would hold a session; both are boundaries CLAUDE.md #6 draws |
| The child writes the bundle to a path `core` reads | The child that owns the directory has exited when the answer arrives; a path to nothing owned is a race |
| Apply builds again rather than holding the dry run's candidate | Two builds, forty seconds, and a person approving bytes they never saw |
| Compile whatever the editor buffer holds | The export and the saved source could disagree, and freshness would need a third hash |
| Key a compiled device's parameter by `ParamID` | Measured: an insert, a reorder and a removal move it, and a lane drove the wrong control with no refusal |
| Key it by display name | Measured: a label edit changes it while nothing about the control does; labels need not be unique |
| Accept a range change silently | Measured: every stored normalised value doubles its meaning with no refusal anywhere |
| Keep `--scan` as the inspection path | A stand-in component writes another plugin's commit against a compiled export |
| Key the loader by `vendor/name` | Two versions of one instrument share a name by construction |
| Name exports distinctly instead of hiding symbols | Measured to work, but every version of one source shares its processor name; the flags fix the cause rather than the symptom |
| Leave `render_export` without a timeout | Measured: a deadlocked plugin hung the call until a person killed it |
| Carry opaque state across a preview restart | No producer and no contract; a parameter-only device needs none |

## Consequences

- **M5 PR 4**: `proto/export.proto` whole (`Export`, `ExportRecord`, `Recipe`, `Endpoint`, `Diagnostic`)
  and the three request messages on `SongTools`, regenerated for every consumer. The two tools
  answer `UNIMPLEMENTED` until PR 8 and join neither `IMPLEMENTED` nor `OFFERED`.
- **M5 PR 6**: `--inspect`, the loader keyed by export, and two same-named exports in one plan,
  driven with real bundles from PR 5's recipe.
- **M5 PR 7**: `compilers/dsp/` and `Export.Compile`, with every exit-code trap above watched
  failing, a grandchild that ignores shutdown, and independent builds compared.
- **M5 PR 8**: the tools, the candidate, lowering, the refusals, the render wall clock, and the
  instrument and effect goldens. It is the working-export checkpoint.
- **M5 PR 10**: the DSP view. **M5 PR 11**: the three tools offered, on a hand-written transcript.
- ADRs amended in place, dated:
  - **0007** §1 and §2 — a compiled device crosses as a path, a class and `ParamID`s; §6's
    `render_unsupported` for these kinds gives way to decision 4's refusals.
  - **0008** §1 — the render wall clock.
  - **0013** §2 — a fresh process on an export change.
  - **0015** §1 — the hazard answered.
  - **0019** — staged assets.
  - **0022** §1 and §3 — the tools and their failure kinds.
  - **0026** §2 — the M5 tools have their shapes.
- `docs/specs.md` §5 gains the tools' shapes. §7.2's exit-code sentence is corrected:
  `cmaj render` exits 1 and says nothing, and `cmaj generate` leaves files behind. §8 says
  how a compiled device is loaded. §9 describes the DSP view. §13's `/proto` line gains
  `Export`. §15 gains the rows.
- **What this rests on that is unmeasured**: preview replacement on a real device; a second
  machine's toolchain giving the same endpoint table; and a converted graph's endpoints, which
  ADR 0032 decides.
