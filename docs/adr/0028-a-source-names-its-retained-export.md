# ADR 0028 — A source names its retained export

- **Status:** Accepted (2026-10-22)
- **Affects:** `schema/song.proto` (`SourceRef`, `ModelRef`), `schema/gen/`,
  `tests/fixtures/song/minimal.json` and its three round-trip suites (M5 PR 3);
  `proto/export.proto` (`ExportRecord`, M5 PR 4); `core/src/project.rs`, `core/src/validate.rs`
  and `core/src/render.rs` (the record, the loading cache and the refusals, M5 PR 8); ADR 0002
  Consequences and ADR 0025 §2, amended in place
- **Builds on:** ADR 0002 §4 (canonical JSON emits every no-presence field), §5 (`optional`
  only where absent differs from zero) and §7 (no field against no producer); ADR 0004 §1 and
  §3 (a derived file, and the commit point); ADR 0010 §1 (the content hash *is* the pin for a
  `SourceRef` or `ModelRef`) and §4 (a fact about a binary is stated by the binary); ADR 0011 §4
  and ADR 0025 §1 (an additive field and the replay hazard it carries); ADR 0025 §2 (the
  question this decides); ADR 0023 §2 and §3 (export-and-load, and the converter as the neural
  path); ADR 0027 §3 (`artefacts` and `models` reserved); `docs/plan.md`, "What the spike found,
  measurements 1–3 (run 2026-10-22, after U9)"
- **Recorded in:** `docs/specs.md` §4.2, §7.2, §10 and §15.

## Context

ADR 0002 deferred `SourceRef.export_hash` to the milestone that could produce one, and ADR 0025
§2 split it from `Generator.compiled_hash` and kept it waiting for M5's export pipeline. It left
one question for this ADR: whether the field names **the wrapped binary**, or a small **export
record** naming the binary, the toolchain that made it and the parameters the source declares.

The spike answered what the record has to carry before anybody guessed. A compiled device's
VST3 `ParamID` is the Cmajor **endpoint handle** — 1-based, inputs in declaration order and then
outputs, streams and events included — so an insert, a reorder or a removal moves it, and a
range edit keeps the id while moving the meaning of every stored normalised value. The binary's
parameter names are display labels, not endpoint identifiers; only the generated code says which
identifier has which handle. ADR 0010 §4's manifest, which maps an id to a name, is
byte-identical before and after a range change. So whatever the field names has to carry an
interface the binary cannot state about itself, which settles ADR 0025 §2's question for the
record.

Two further facts shape the rest. **The export is reproducible**: two independent exports of one
patch, in differently named directories, give the same `.so` and `.clap` byte for byte, so
identical inputs can give an identical record. **The bundle is one file**,
`Contents/x86_64-linux/<name>.so`, with no `moduleinfo.json` and no resources.

And ADR 0025 §1's comfort is not available. That ADR could say "no such project exists" of a
`Generator` in an old log, because nothing could mint one before M4. A `SourceRef` has been
mintable since M0. Four committed determinism logs add one — `branches`, `every_tool` (a Cmajor
and a Faust ref), `refusals` and `undo` — three of their songs still hold one, and the schema
fixture holds a Cmajor ref.

## Decisions

### 1. `export_hash` on both references, `optional`, absent meaning never exported

```proto
// A Cmajor or Faust source in assets/ by content hash (§10), and — once one has been made —
// the export record that says what was built from it (ADR 0028 §1, §2).
message SourceRef {
  string source_hash = 1;
  // Absent: never exported. Present: the SHA-256 of an export record in assets/, and never
  // empty. Written by compile_instrument and by no other tool (ADR 0029 §3).
  optional string export_hash = 2;
}

message ModelRef {
  string model_hash = 1;
  // As SourceRef's: the export record of the plugin converted from this model (ADR 0028 §5).
  optional string export_hash = 2;
}
```

**`optional`, and the reason is replay, not taste.** ADR 0025 §1 made `compiled_hash` a plain
string because absent and empty meant the same thing, and canonical JSON emits a plain field
at its default (ADR 0002 §4). Do that here and every one of the four logs above holds an `add`
whose value has one key, beside a re-serialised song whose value has two: `open` reports
`song_diverged`, which is exactly the hazard ADR 0025 §1 recorded and said the next additive
change must check again. An `optional` field is emitted only when present, so an old reference
stays as it was written and replays to the same bytes. ADR 0002 §5 holds in a different form:
**absent and present are the only two meaningful states**, because a present empty string is
refused (below), and absent is the meaning "no export yet" has always had.

**A `string`, hex SHA-256**, as every other content hash in the schema is (ADR 0025 §1).
**Two validator rules**, both caller-fixable: `export_hash_malformed` for a present value that
is not 64 lowercase hexadecimal characters, the empty string included; and
`export_unsupported` for an `export_hash` on a `faust` reference, since Faust is import-only
and compiles nothing in v1 (§7.2), so no tool could have produced one. **The legacy
`source_hash` placeholders are not tightened** — the four logs hold 32-character values that
are not SHA-256s — because a new rule about an old field would strand those projects for a
reason this ADR does not need.

**Written by `compile_instrument` and by no other tool** (ADR 0029 §3). A caller's
`apply_patch` can still write the path, as it can `compiled_hash`'s, and what it then states is
checked where it is used (decision 4) rather than refused for being written.

**`schema_version` stays 1 only if M5 PR 3 proves it on the existing logs**: every committed log
above opened, replayed and compared — the intermediate `get_song_at` states and `undo` included,
in all three serialisations — with no byte moved. If the proof fails, the pull request stops for
an explicit migration ADR. It does not relax `song_diverged`, and it does not regenerate history.

### 2. It names an **export record**, a content-addressed asset of canonical JSON

The field names neither the binary nor a bundle archive. It names a small, immutable **export
record**: canonical JSON from the store's one writer (ADR 0002 §4), hashed by the store's one
hasher, kept in `assets/` like any other asset. Its message is `ExportRecord`, in
`proto/export.proto` beside the wire it is built from (ADR 0029 §2). It lives there rather than
in `song.proto` because it describes a binary and not a song, and in a `.proto` at all because
the window and the source read a model is offered both show what it holds (ADR 0029 §3), and no
language here holds a hand-written model type (CLAUDE.md, Toolchain). It carries, and nothing else:

| Field | What it is | Why it is here |
|---|---|---|
| `input` | the `source_hash`, or the `model_hash` and the conversion record's hash (decision 5) | what the binary was built from; freshness is this against the reference (decision 4) |
| `target` | `vst3/x86_64-linux` | what loads it; the only target v1 builds (ADR 0014 §2) |
| `recipe` | the toolchain that actually ran: the Cmajor, CLAP SDK, clap-wrapper and VST3 SDK commits, the C/C++ compiler and its flags, Cmajor's optimisation level, and the recipe's own edits to generated code | an export's identity includes how it was made (ADR 0023 §2); its fields are ADR 0031's to fix |
| `members` | each file of the bundle, by relative path, to the SHA-256 of its blob | what reconstructs the bundle (decision 3); one entry for every Linux export the spike made |
| `class` | vendor, name, VST3 class id and CLAP id, as the built binary reports them | what the engine is told to load (ADR 0029 §6) |
| `ports` | audio input and output channel counts, and whether it takes MIDI | an instrument with no MIDI input is deaf, and the spike exported one that exited 0 |
| `params` | per **source endpoint identifier**: the VST3 `ParamID`, the display title, the plain `min`, `max` and `init`, and `step` where the source declares one | the interface the binary cannot state about itself; what a name resolves through (ADR 0029 §5) |

**`core` writes it, after inspecting what was built.** It is assembled from the child's answer —
the files, the generated endpoint table and the toolchain identity — checked against what the
engine's inspection of the built binary reports (ADR 0029 §6). A record is never edited: a new
compile writes a new record and changes the reference, and the old one stays where history
reaches it.

**The hash covers the whole record**, so identity follows content. Recompiling the same source
under the same recipe gives the same bytes and therefore the same `export_hash`, which is
idempotent rather than a new export. Recompiling under a changed recipe gives a different record,
and the old export keeps its own provenance. Two branches can therefore hold two exports of one
source made by two toolchains, and neither entry replaces the other.

### 3. Every member is a blob, and the bundle the engine loads is a disposable reconstruction

**No archive.** Each file of the bundle is stored as an ordinary asset, named by its SHA-256, in
the flat `assets/` that `add_asset` already writes — so there is no tar or zip dependency and no
archive timestamp to make two identical exports differ. The source, the record and the members
are all retained with the project and travel with it.

**What the engine loads is rebuilt, not stored.** `core` reconstructs the `.vst3` directory from
a verified record and its blobs into the application's cache, under the export hash —
`$XDG_CACHE_HOME/escribass/exports/<export_hash>/` — staging it and renaming it into place, so a
half-written bundle is never at a path the engine can be handed. A path there is **immutable**:
an export hash names one set of bytes for ever, so nothing is ever overwritten in place, which
M4's spike showed a host does not survive reliably. Deleting the cache costs a rebuild and nothing
else; **deleting a referenced blob from the project does not rebuild anything** — the device
refuses with `export_missing` rather than being compiled again under whatever toolchain is
installed today, because derived does not mean disposable when the promise is playback without
a compiler.

**Member names are refused before a byte is written** if any is absolute, contains `..`, repeats,
or would escape the staging directory, and anything but an ordinary file or directory is refused
too. v1's measured Linux bundle is one regular file, so the rule costs nothing it uses.

### 4. Pending, stale and current are derived by `core`, and the song may hold any of them

| State | What `core` sees | Preview or render of a plan holding the device |
|---|---|---|
| **pending** | no `export_hash` | refused, `export_pending` — caller-fixable, naming `compile_instrument` |
| **stale** | the record's `input` is not the reference's current `source_hash` (or `model_hash`) | refused, `export_stale` — caller-fixable, the same tool lifts it |
| **current** | they match, and the record and its members verify | renders, from the reconstruction |

**None of the three is a validator refusal of the song.** A saved source that still names its
previous export is a valid song — that is U10's decision, keeping the last export for history —
and refusing to save it would make the stale state impossible to reach. A device refuses to
*play* while it is pending or stale, and plays only an export built from the source it holds.

**Integrity is not freshness, and the two fail on different sides of ADR 0006 §2's line.** A
record that is missing, does not hash to its name or does not parse, or a member that is
missing or does not hash to its entry, is an **operator** error — `export_missing` or
`export_corrupt` — because no edit to the song repairs a damaged project directory. Freshness is
a comparison between two hashes the song itself holds, and its refusals are the caller's to fix
by compiling.

**Playback needs no compiler.** Opening a project and rendering it never spawns a compiler,
never reaches the network and never asks a model anything. A project copied to another machine
with the runtime closure U5 accepted and **no Cmajor at all** renders its exported devices from
the record and the blobs it carries. That is the proof point M5 PR 8 runs, and it is a claim
about the exercised path only — not that every library in the closure is inert.

### 5. A converted model's export reaches a **conversion record**, and that is what `models` keys

A `ModelRef`'s `model_hash` keeps naming the ONNX graph the person supplied. Its export record's
`input` names that hash **and a conversion record** — another content-addressed asset binding the
graph's hash, the hash of the Cmajor source the converter produced (stored as an asset, so what
was compiled can be read), the adapter that made a tensor graph into an audio device, and the
converter's own recipe. The spike's converted graph needed an explicit adapter graph, and the
recipe stripped the converted graph's own `[[ main ]]`; both are recipe facts the conversion
record names rather than guesses a reader has to reconstruct.

What a conversion record's fields are, which graphs are accepted and what the adapter is are
ADR 0032's, written with PR 12. This ADR fixes only that the record exists, that it is
content-addressed, and that its hash is the key of `lock.json`'s `models` block (ADR 0030 §2).
That key is the conversion's and not the model's, so one graph converted under two toolchains
keeps two entries, not one entry rewritten.

### 6. What is not added

No `export_hash` on `PluginRef` or `SamplerRef`, which pin through `lock.json`'s `plugins` block
(ADR 0010 §1). No status field in the song: pending, stale and current are derived, as ADR
0024 §6 derived *stale* for a generator. No copy of the parameter interface in the song, and no
`exported_at`. `Instrument.state` stays the field it is; a compiled device with non-empty state
is ADR 0029 §7's refusal.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| `export_hash` names the wrapped `.so` itself | The binary cannot say which endpoint identifier has which `ParamID`, or what a value's range is; the spike measured both. The interface would then live nowhere, or in a second place |
| A plain `string`, as `compiled_hash` is | Four committed logs would replay to a value one key short of the song they produced, and `open` would refuse them all with `song_diverged` (ADR 0025 §1's hazard, met) |
| Archive the bundle as one asset | A tar or zip dependency and an archive format's timestamps, for a Linux bundle that is one file |
| Keep the built bundle on disk and load it in place | A path that can be overwritten under a running host, and a project that stops rendering when a cache is cleared |
| Rebuild a missing export from the source at open | Opening would need a compiler and the network, and would silently produce the binary today's toolchain makes rather than the one the project recorded |
| Refuse to save a source whose export is stale | Makes U10's decided state impossible to reach, and couples saving text to a twenty-second compile |
| A `models` key of `model_hash` → latest plugin | One graph under two toolchains on two branches could not both survive; a lookup that is rewritten is not a pin |
| Tighten `source_hash` to a real SHA-256 now | Strands every project carrying one of the 32-character placeholders, for a rule this ADR does not need |

## Consequences

- **M5 PR 3** (`m5.3-export-ref`): the two fields, `schema/gen/` regenerated in three languages,
  the schema fixture, and the replay proof on the existing logs. **A schema change alone**, as
  M1 PR 2 and M4 PR 2 were. `buf breaking` and the absence tests both run.
- **M5 PR 4**: `ExportRecord` in `proto/export.proto`, generated for every consumer.
- **M5 PR 8**: `core` writes and verifies records, reconstructs bundles into the cache, and
  answers `export_pending`, `export_stale`, `export_missing` and `export_corrupt`. These retire
  the five sites in `core/src` that still say a Cmajor or neural device is something "M4"
  compiles (ADR 0007 §6, amended 2026-10-02). One of them is the `render_unsupported` message
  four determinism goldens hold, so those four goldens move in that pull request, named.
- ADR 0002's Consequences and ADR 0025 §2 are amended in place, dated: the producer has arrived,
  and the question ADR 0025 §2 left open is answered by decision 2.
- `docs/specs.md` §4.2's layer-3 line, §7.2's persistence bullet and §10's description of
  `assets/` say what is stored and what is derived; §15 gains this ADR's rows.
- **What this rests on that is unmeasured**: byte-identical exports were measured on one machine.
  A second machine or another compiler build may produce a different `.so`, which would be a
  different record and a different hash — correct, and visible in the diff — rather than a
  reproducibility failure. Whether it does is PR 7's standing check.
