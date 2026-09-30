# ADR 0025 — `Generator` gains `compiled_hash`, and `SourceRef.export_hash` waits for its producer

- **Status:** Accepted (2026-09-25)
- **Affects:** `schema/song.proto` (`Generator`), `schema/gen/`,
  `tests/fixtures/song/minimal.json` and its three round-trip suites (M4 PR 2); ADR 0002 §7
  and Consequences, amended in place
- **Builds on:** ADR 0002 §7 (no field against no consumer), §8 (the reductions table), §4
  (canonical JSON emits defaults), §5 (`optional` only where absent ≠ zero), §12 (a replay and
  its schema boundary) and Consequences (the two hashes deferred to M4); ADR 0011 §4 (an
  additive change keeps `schema_version` at 1, and the replay hazard it recorded); ADR 0024 §6
  (what the hash is over, and where it is computed); ADR 0023 §6 (the ledger row, split)
- **Recorded in:** `docs/specs.md` §4.2 and §15.

## Context

ADR 0002's Consequences deferred two fields to M4, "both additive and both surfaced by the
wireframes": `SourceRef.export_hash` for Plate 3's *export pending*, and a `Generator`
compiled-source hash for Plate 3's *compiled · stale*. The ledger's fifth walk sent the row
here. M2 and M3 changed `schema/` not at all — it is byte-identical to M1's close — so this is
the first `song.proto` change since ADR 0011, and CLAUDE.md #5 wants the ADR before the
`.proto`. This is that ADR, and it lands alone in M4 PR 2 as ADR 0011 landed alone in M1 PR 2.

Under ADR 0023 §1's split the two fields have producers in different milestones, and ADR 0002
§7's own rule — a field defined against no consumer is defined wrong — decides what happens to
each.

## Decisions

### 1. `Generator.compiled_hash`, a `string`, field 11, not `optional`

```proto
message Generator {
  …
  map<string, string> params = 10;

  // The SHA-256 of the inputs the last compile was made from — the CompileRequest core built
  // (ADR 0026 §1), hashed by the store's one hasher (ADR 0024 §6). Empty until the first
  // compile. Written by compile_generator and by no other tool; a value unequal to the hash of
  // the current inputs is what the code view calls "stale", and it describes the source's
  // inputs, never the notes, which a person may edit (§2.4).
  string compiled_hash = 11;
}
```

**Not `optional`**, by ADR 0002 §5's rule: "absent" and "never compiled" are the same statement,
so the empty string is the meaningful default and presence carries nothing. Canonical JSON
emits defaults (ADR 0002 §4), so every `Generator` in every `song.json` written from M4 PR 2
carries `"compiled_hash": ""` until it is compiled, and an ordinary edit to it is a `replace`.

**A `string` and not `bytes`**, as `SourceRef.source_hash`, `ModelRef.model_hash`,
`SamplerRef.sfz_hash` and `AudioClip.asset_hash` already are: a hex digest in a file §2.6 wants
readable, and `bytes` would cross the JSON carrier as base64 (ADR 0006 §6).

**Written by `compile_generator` and by no other tool.** A caller's `apply_patch` may write
`/generators/{id}/compiled_hash` — nothing refuses a path the schema has, and the mixer writes
`Mix` that way — and what it buys is a status word that lies until the next compile recomputes
it, which the log records as that person's edit. It is not `version`, which ADR 0005 §3 refuses
because `core` maintains it; the hash is a fact about a compile, and a person who writes one by
hand has made a statement the log attributes to them. **The model** reaches it only through
`compile_generator` in practice, since a model writing a hash by `apply_patch` has no way to
compute one, and ADR 0026 §3 says nothing about it either way rather than inventing a refusal
for a path nobody has a reason to write.

**`schema_version` stays 1, and `buf breaking` has nothing to report** (ADR 0011 §4's
precedent). The field number is new and nothing is renamed or reused. A document written before
this change replays into the new type and takes the empty default, which is exactly the
previous meaning: never compiled, because nothing could compile before M4.

**The replay hazard ADR 0011 §4 recorded, checked again.** `Project::verify_against_replay`
compares the replayed log against the re-serialised song, and canonical JSON emits defaults; so
a project written *before* this change whose log contains an `add` of a `Generator` would have a
ten-key value in its log and an eleven-key value in its song, and `open` would report
`song_diverged`. **No such project exists**: `tests/fixtures/song/minimal.json` carries a
`Generator` but is a constructed value with no log (`tests/AGENTS.md`); no determinism script
adds one — M4 PR 5's `generators` script is the first — and no typed tool could produce one
before `define_generator`. The hazard is recorded a second time so the next additive change
checks it a third, and so the first change that *would* strand a real document is the one that
bumps `SCHEMA_VERSION` and owes a migration.

### 2. `SourceRef.export_hash` is **not** added here; it lands with the export pipeline, in M5

Nothing produces one until `cmaj generate → compile → wrap → hash` exists (ADR 0023 §2), which
is M5's, and ADR 0002 §7 refused to define `FormRule` against no consumer for exactly this
reason. Adding the field now would be a string every `SourceRef` carries empty for a milestone,
described by a comment that guesses what M5's export record will put in it. **ADR 0002's
Consequences are amended in place**: the row is split, `compiled_hash` here and `export_hash`
in M5's schema ADR, which lands before M5's `.proto` as this one lands before M4's.

What M5's ADR will have to decide, named so it is not discovered late (trap 21): whether
`export_hash` names the wrapped binary alone or the **export record** — the small asset naming
the binary's hash, the toolchain that made it *and the parameters the source declares*, which is
what lets `param_unknown` resolve for a compiled device without instantiating anything (plan,
question 17; `core/src/validate.rs`'s `ponytail:` at the parameter check). One hash naming one
thing is the shape; which thing is M5's.

### 3. Nothing else changes in `song.proto`

`FormRule` gains no message (ADR 0023 §6; ADR 0002 §7 amended). `Generator.params` stays
`map<string, string>` (ADR 0002 §8: "typed params when the DSL defines a parameter schema"),
and the DSL reads them as strings the author converts (ADR 0024 §3). `GeneratorKind` keeps its
one value and its room. `Generator.toolchain_version` keeps its meaning and its field, gains a
real value for the first time in M4 PR 5, and has its §4.4 non-empty rule narrowed to a compiled
generator — a validator change, not a schema one (ADR 0027 §1; trap 11).

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Add `export_hash` now, since it is additive | Additive is not free: a string on every `SourceRef` with no producer for a milestone, described against a record M5 has not designed (ADR 0002 §7's rule, and question 17's own list) |
| `optional string compiled_hash` | Presence would carry nothing "" does not; ADR 0002 §5 uses `optional` only where absent differs from zero |
| Store the compile's inputs beside the hash, so "stale" needs no recomputation | A copy of the tempo map, the signature map and the sections inside every generator — a second representation of song state, §14.2 |
| A `compiled_at` timestamp | "Stale" would depend on a clock (CLAUDE.md #3), and a timestamp says *when*, not *from what* |
| Refuse `apply_patch` on `compiled_hash` | A rule for a path nobody has a reason to write, guarding a status word the next compile recomputes; `version` is refused because `core` owns it, and this field is a recorded fact a person may misstate as they may misstate a name |
| Bump `SCHEMA_VERSION` to 2 | Additive, with a default that is the previous meaning; a bump would refuse every existing project with no migration to offer (ADR 0011 §4) |

## Consequences

- **M4 PR 2** (`m4.2-schema`): the field, `schema/gen/` for Rust, TypeScript and Python
  regenerated, `tests/fixtures/song/minimal.json` gaining `"compiled_hash": ""` and its three
  round-trip suites — **and nothing else**. No determinism golden carries a generator, so none
  should move; the fixture is the one file expected to, and any other byte is named.
- `song.proto`'s comment on `Generator` is rewritten to say what the field means, and its
  "compiled output is the target clip's notes" sentence stays, because it is still true.
- §4.2's Layer 1 gains the field in its `Generator` line; §15 gains this ADR's row.
- ADR 0002 §7 and Consequences amended in place, dated; the ledger row split as ADR 0023 §6
  says.
- **The producer is M4 PR 5** (ADR 0024 §5) and **the consumer is M4 PR 7**, the generator code
  view; a field with both in the same milestone is what ADR 0002 §7 asks for.
