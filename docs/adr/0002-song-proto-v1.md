# ADR 0002 — Initial `song.proto` (schema v1)

- **Status:** Accepted (2026-09-02)
- **Affects:** `schema/song.proto`, `schema/history.proto`, canonical JSON (§4.1), M0.2 validator
- **Builds on:** ADR 0001 (id-keyed maps, derived order, history separation)
- **Recorded in:** `docs/specs.md` §15.

## Context

§4 describes the song model in prose. Turning it into Protobuf forces choices §4 does not
make: integer widths, how the cross-cutting fields of §4.3 attach, how sum types are
encoded, and what "canonical JSON with stable key order" means concretely. CLAUDE.md #5
requires these be recorded before code.

Nothing here contradicts a §15 decision or ADR 0001. Two entries reduce what §4.2 lists;
both are called out in decision 8 and are additive to extend.

## Decisions

### 1. Tick fields are `int32`

The proto3 JSON mapping encodes every 64-bit integer type as a JSON **string**. Ticks are
the most-read numbers in the file, so `int64` would print `"start_tick": "61440"` in a
format §2.6 [MUST] requires be readable and diffable.

`int32` at 960 PPQ reaches ~559,000 bars of 4/4 — about 13 days of audio at 120 bpm.
Signed rather than unsigned because tick arithmetic (deltas, transposition, clip-relative
conversion) underflows during intermediate computation, and unsigned underflow panics in
debug Rust; non-negativity is a §4.4 validator rule, not a type-level one.

`Generator.seed` stays `uint64` as §4.2 specifies. It is the one field that prints as a
string, and nobody diffs a seed numerically.

Absolute tick positions (`clip.start_tick + note.start_tick`) are computed in `i64` in
`core` to keep intermediate sums clear of the boundary.

### 2. §4.3 cross-cutting fields are plain fields on every entity

`string id = 1; Provenance provenance = 2; uint32 version = 3;` repeated on each entity
message, rather than one embedded `Meta` message.

Protobuf has no field mixins, so the alternative is a submessage — which would put `/meta/`
into every RFC 6902 path and contradict ADR 0001's own worked example
(`/clips/…/note_clip/notes/{id}/pitch`). Repetition costs three lines per message and nothing in any
generated language.

`AutomationPoint` carries `id` only. §4.2 writes points as anonymous `{tick, value, curve}`
tuples; ADR 0001 keys them solely so patch paths survive inserts. Their author is the
parent `Automation`'s. `Note` keeps full provenance, because per-note authorship is
displayed (wireframe Plate 2).

### 3. Sum types are `oneof`, and each arm is a message

`Clip.content { note_clip | audio_clip }`, `DeviceRef.kind { plugin | cmajor | faust |
neural | sampler }`, `Generator.target { track_id | clip_id }`.

The set case *is* the kind, so a parallel `kind` enum would duplicate it and permit
states where the two disagree. Arms are messages rather than bare scalars because widening
a scalar field to a message later is a breaking change, and §7.2 will extend the source
refs.

### 4. Canonical JSON is proto field names, defaults emitted, map keys sorted

The §4.1 canonical form is: **snake_case field names, every no-presence field emitted even
at its default, map keys sorted, two-space pretty print.**

Emitting defaults is the load-bearing part. Standard proto3 JSON printers omit
default-valued fields, which would make RFC 6902 `replace` and `test` fail against paths
that are absent only because a value happens to be zero. With defaults always present,
every plain field always exists, so ordinary edits are `replace`; only `optional` fields,
message fields and map entries legitimately appear and disappear, and those are `add` and
`remove`.

snake_case matches §4, ADR 0001 and the wireframes, and matches the attribute names in
generated Rust and Python. Every proto JSON parser accepts both casings on input.

This needs no custom serialiser for `song.json`: `pbjson` exposes
`preserve_proto_field_names`, `emit_fields` and `btree_map`, and a `BTreeMap` iterates in
sorted key order. The concrete writer lands in `core` at M0.2.

Three further degrees of freedom are part of the canonical form. **Amended 2026-09-02,
while implementing the writer:** the original text put all three on the writer. Only the
first belongs there. A writer that silently rewrites values puts the in-memory model and the
file out of agreement, which is the second-representation problem §14.2 exists to prevent, so
each normalisation now sits where the value enters rather than where it leaves.

- **Doubles — the writer's job.** `serde_json` must be built with `float_roundtrip`. Its
  default parser is off by one ULP on roughly a third of doubles, so load-then-save is not a
  fixed point and §11's "same project file → identical output" fails on the first re-save.
  Printing is already shortest-round-trip and stable. The writer **rejects non-finite
  doubles**: NaN and the infinities serialise as `null` and cannot be read back, so one NaN
  from a plugin parameter would otherwise write a project that cannot be opened. The error
  names the offending field by JSON Pointer.
- **`-0.0` — the tool API's job, on input.** It compares equal to `0.0` and prints
  differently, so two equal songs would produce different bytes. It is normalised where
  values enter, and the validator rejects it in a stored document. The writer serialises what
  the model holds.
- **Message presence — the validator's job.** `Track { routing: None }` and
  `Track { routing: Some(default) }` are semantically identical and produce different bytes.
  The validator requires `provenance`, `mix`, `routing`, `tempo_map`, `time_signature_map`
  and `render_target`, with `instrument` present exactly when `kind == INSTRUMENT`. A valid
  song therefore carries them all and there is nothing for the writer to fill in.
- **Timestamps — the clock's job.** The injectable clock yields millisecond precision, so
  nothing downstream truncates, and the validator rejects finer precision in a stored
  document. The text form is whatever `pbjson` emits: RFC 3339 with a `+00:00` offset. The
  original text specified a `Z` suffix to match proto3's canonical JSON, which was not worth
  it — this canonical form already departs from proto3's in two larger ways, field naming and
  default emission, so matching it on timestamps buys no compatibility and would cost a
  string pass over the serialised document. All three languages parse both forms.

### 5. `optional` only where absent differs from zero

Used on `Provenance.model_id`, `.prompt_id`, `.tool_call_id`, `Routing.output_track_id`
and `Clip.loop_length_ticks`. (~~`Op.from`~~ **corrected 2026-09-24, at M3's close**: §11's
revision of 2026-09-02 removed the `Op` message altogether — `PatchEntry.ops` is `bytes` —
and this list was not revised with it, so one ADR listed a field its own later section had
deleted.) Not used where the zero value is the meaningful
default — `Note.microtonal_cents` absent and `0.0` are the same statement.

### 6. Two files, two packages

`schema/song.proto` is `escribass.song.v1`. `schema/history.proto` is
`escribass.history.v1` and imports `song.proto` for `Provenance`; the dependency never runs
the other way.

§14.6 and ADR 0001 require that history never reach the render engine. With separate
packages a `Song` *cannot* carry a `PatchEntry` by construction, so M1's snapshot builder
only has to reason about one package.

### 7. `FormRule` is deferred to M4; `Marker` is kept

§4.2 gives `FormRule` four loosely named fields and nothing consumes it before the
generative compiler exists (§7.1, M4). Defining it now means defining it against no
consumer. Adding it later is additive — a new map on `Song` — and the M4 ADR will specify
it against a real caller. `Marker` is two fields and §4.2 names it, so it stays.

**Re-deferred 2026-09-25, at M4's planning, on an event rather than a milestone (ADR 0023
§6).** The consumer this section waited for arrived and does not consume it: a generator reads
the layer-2 `sections` it is handed (ADR 0026 §1), which already carry order and length, and
what a `FormRule` would add is a rule to re-lay sections when `target_length` changes — a
second compiler nothing has asked for. So "M4" above is not the date it lands; the trigger is
the first time a person wants a form re-laid rather than re-typed, and `set_form` waits with it
on the first measurement in which a model fails to lay a form out with the tools it holds
(`add_section` × N and `apply_patch`, both of which it has). The rule this section states — no
field against no consumer — is the one that re-deferred it, and the one ADR 0025 §2 applies to
`SourceRef.export_hash` in the same pass.

### 8. Reductions from §4.2, each additive to extend

| §4.2 says | v1 has | Upgrade path |
|---|---|---|
| `SampleMap` (SFZ-compatible) | `SamplerRef { sfz_hash }` | The SFZ file is the sample map; a structured form can be added when a tool needs to edit one. |
| `AudioClip` | `{ asset_hash }` | Gain, fades and time-stretch when M1 renders audio clips. **Taken 2026-09-05: ADR 0011** adds `gain_db`, `fade_in_ticks`, `fade_out_ticks` and `time_stretch`. |
| `Routing { sends, sidechains }` | `sends: map<track_id, double>` (dB), `sidechains: map<effect_id, track_id>` | Send messages if pre/post-fader or per-send params are needed. |
| `points: [{tick, value, curve}]` | `curve` enum with `LINEAR` and `HOLD` only | Any other interpolation formula is renderer-specific and would put bit-exactness (§11) at the mercy of the renderer. Add shapes with a defined formula. |
| `Generator.kind (strudel \| python \| …)` | `GENERATOR_KIND_PYTHON` only | §15 already defers Strudel. |
| `Generator.params` | `map<string, string>` | Typed params when the DSL defines a parameter schema. |
| `Track.instrument: InstrumentRef`, `fx_chain: [EffectRef]` | `Instrument` and `Effect` embedded in `Track` | Ids are globally unique and the entities have exactly one owner, so embedding removes a whole class of dangling-reference validation. `DeviceRef` is what §4.4's "InstrumentRef/EffectRef resolves" refers to. |
| `Note` per-note expression `[{param, value}]` | `map<string, double>` | Keyed by parameter name: path-stable without needing a ULID per expression value. |
| `Clip.loop` | `optional int32 loop_length_ticks` | Presence carries the boolean, so two fields cannot disagree. |
| `TempoMap.events`, `TimeSignatureMap.events` | id-keyed maps | See ADR 0001 §3's correction: an array index is not a stable patch path, whether or not the elements have identity. |

### 9. `Generator.seed` is a plain `uint64`, making §4.4's non-null rule structural

A no-presence field always has a value, so the compiler can never be handed a missing seed.
Seed `0` is a legitimate seed. The remaining §4.4 check is `toolchain_version != ""`.
(**Narrowed 2026-09-25, ADR 0027 §1**: to a generator that has been compiled — the first compile
writes the version from what the compiler reported, so a generator compiled by nothing has none
to state, and the check is on the pair `compiled_hash` / `toolchain_version`.)

### 10. `syntax = "proto3"`, not Editions

`prost-build` does not accept `edition = "2023"` (tokio-rs/prost#1031). proto3 is fully
supported and not deprecated.

### 11. Patch operations are canonical JSON text, not a protobuf message

**Revised 2026-09-02, before any consumer existed.** This decision originally modelled an
operation as `Op { op, path, from, google.protobuf.Value value }`, on the reasoning that its
proto3 JSON *is* a valid RFC 6902 operation object. The syntax was right and the effect was
wrong, in two ways that a review caught by running it:

1. **The patch log could not be byte-stable.** `pbjson_types::Struct.fields` is a `HashMap`,
   and the `btree_map` option reaches our generated messages but not a precompiled crate.
   The same `add` op serialised in four processes produced four key orders. Every
   `add_track`, `add_clip` and `add_note` entry carries an object value, so ADR 0001's
   promise that M0.4 compares patch logs byte for byte was unachievable.
2. **Our own patches produced documents `core` could not read.** `Value.number_value` is a
   `double`, so a pitch was written `43.0`; applying that with an ordinary JSON Pointer
   implementation and re-parsing failed on the `int32` field. The three languages disagreed
   about it — Rust rejected `43.0`, TypeScript accepted it, and Python silently truncated
   `43.5` to `43` and `true` to `1`.

Both follow from trying to model a dynamically typed format in a statically typed one. So:

- **`patches/*.json` is a literal RFC 6902 document**, written by `core`'s canonical writer
  from `serde_json::Value`. `serde_json::Map` is a `BTreeMap`, so object keys sort; integers
  stay integers. Any off-the-shelf patch library can apply the file, and the result parses.
- **`history.proto` is transport.** `PatchEntry.ops` is `bytes`, carrying the operations
  array as canonical JSON text. M0.3's `dry_run` responses return the same text.

  *Amended 2026-09-03:* this said "the bytes on the wire are the bytes on disk", which cannot
  hold — in the file the array is a member of an object and indented one level, while on the
  wire it stands alone. The guarantee that was actually being protected is that **both carry
  the same canonical JSON document, never a protobuf re-encoding of it**, and that is what
  prevents the two defects recorded above. Note the consequence for the writer: the generated
  serde impl base64-encodes a `bytes` field, so `patches/*.json` cannot be written with it —
  see `core/src/history.rs`.
- **Ops targeting a 64-bit field carry a JSON string**, matching the canonical form. Written
  as a number, a seed above 2^53 is exact in Rust and Python and lossy in TypeScript.

`history.proto` therefore no longer defines the on-disk patch shape; ADR 0001 §1 does.

### 12. `PatchEntry` records the schema version it was authored against

ADR 0001's materialise-and-diff replays from the root, so without it a replay cannot tell
when it crosses a schema boundary. One field now; unavailable retroactively later.

## Consequences

**Validator rules this shape implies (M0.2), beyond §4.4:** `key == value.id` on every map;
required oneofs set (`Clip.content`, `DeviceRef.kind`, `Generator.target`); no enum left at
`*_UNSPECIFIED`; `Effect.index` unique within a chain and `Track.index` unique; exactly one
`MASTER` track; `Routing.output_track_id` and send keys resolve to `BUS` or `MASTER` tracks;
sidechain keys resolve to effects on that track; `ParamRef.device_id` resolves; tempo map
non-empty with an event at tick 0; all doubles finite.

**`core` needs an injectable clock** for `Provenance.created_at`, the same pattern ADR 0001
established for the id source, and for the same §11 reason.

**M1** defines the engine snapshot as its own message under `proto/` and strips
`provenance` — ADR 0001 lists `created_at` among what must not reach the engine.

**Deferred to M4, both additive and both surfaced by the wireframes:** `SourceRef` needs an
`export_hash` for §7.2's exported CLAP→VST3 binary (Plate 3's "export pending"), and
`Generator` needs a compiled-source hash to detect stale compiled output (Plate 3's
"compiled · stale"). **Split 2026-09-25 (ADR 0025):** `Generator.compiled_hash` is M4's, with
a producer and a consumer in the same milestone; `SourceRef.export_hash` waits for the export
pipeline that would produce one, which is M5's under ADR 0023 §1 — §7's own rule applied to the
half of this sentence whose consumer is a milestone away.
