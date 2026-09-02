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
(`/clips/…/notes/{id}/pitch`). Repetition costs three lines per message and nothing in any
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

This needs no custom serialiser: `pbjson` exposes `preserve_proto_field_names`,
`emit_fields` and `btree_map`, and a `BTreeMap` iterates in sorted key order. The concrete
writer lands in `core` at M0.2.

### 5. `optional` only where absent differs from zero

Used on `Provenance.model_id`, `.prompt_id`, `.tool_call_id`, `Routing.output_track_id`,
`Clip.loop_length_ticks` and `Op.from`. Not used where the zero value is the meaningful
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

### 8. Reductions from §4.2, each additive to extend

| §4.2 says | v1 has | Upgrade path |
|---|---|---|
| `SampleMap` (SFZ-compatible) | `SamplerRef { sfz_hash }` | The SFZ file is the sample map; a structured form can be added when a tool needs to edit one. |
| `AudioClip` | `{ asset_hash }` | Gain, fades and time-stretch when M1 renders audio clips. |
| `Routing { sends, sidechains }` | `sends: map<track_id, double>` (dB), `sidechains: map<effect_id, track_id>` | Send messages if pre/post-fader or per-send params are needed. |
| `points: [{tick, value, curve}]` | `curve` enum with `LINEAR` and `HOLD` only | Any other interpolation formula is renderer-specific and would put bit-exactness (§11) at the mercy of the renderer. Add shapes with a defined formula. |
| `Generator.kind (strudel \| python \| …)` | `GENERATOR_KIND_PYTHON` only | §15 already defers Strudel. |
| `Generator.params` | `map<string, string>` | Typed params when the DSL defines a parameter schema. |

### 9. `Generator.seed` is a plain `uint64`, making §4.4's non-null rule structural

A no-presence field always has a value, so the compiler can never be handed a missing seed.
Seed `0` is a legitimate seed. The remaining §4.4 check is `toolchain_version != ""`.

### 10. `syntax = "proto3"`, not Editions

`prost-build` does not accept `edition = "2023"` (tokio-rs/prost#1031). proto3 is fully
supported and not deprecated.

### 11. `Op` is shaped so its JSON *is* an RFC 6902 operation

`Op { string op; string path; optional string from; google.protobuf.Value value; }`. The
proto3 JSON of an `Op` is a valid RFC 6902 operation object, so `PatchEntry.ops` is
consumable by any off-the-shelf patch library in any of the four languages without a
translation layer. M0.3's `dry_run` responses return the same message.

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
"compiled · stale").
