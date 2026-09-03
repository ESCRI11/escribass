# AGENTS.md — core/

Crate `escribass-core`. Everything that reads or writes song state. Model types come from
`escribass-schema` and are never redefined here (specs §4.1). Repo-wide rules: `/AGENTS.md`.
The schema itself: `/schema/AGENTS.md`.

## Files

| Path | What | Status |
|---|---|---|
| `src/canonical.rs` | Canonical JSON persistence: `to_canonical_json`, `from_canonical_json`, `CanonicalError`. | done |
| `tests/canonical.rs` | Fixed-point, idempotence, non-finite rejection, seeded double round-trip, key order. Reads `/tests/fixtures/song/minimal.json`. | done |
| `src/patch.rs` | RFC 6902 `apply` and `diff`, and the RFC 6901 pointers they address with. Operates on `serde_json::Value`; knows nothing about `Song`. | done |
| `src/history.rs` | The on-disk shape of `patches/*.json` and `refs.json`, the ADR 0001 §2 ref-name rules, and `History`: the DAG, `ancestry`, `materialise`, refs and `switch`. No I/O. | done |
| `src/validate.rs` | `validate` → every `Violation` (path, stable rule id, message), not just the first. §4.4 plus the rules in ADR 0002 Consequences. | done |
| `tests/validate.rs` | One rule per test, each breaking the fixture in exactly one way. | done |
| `src/project.rs` | The `.escri` directory: `open`, `write`, `lock.json`, atomic writes. Reads and writes only. | done |
| `Project::create`, `Project::commit` | The mutation entry point. `commit` is `prepare` then `record`. | done |
| `Project::prepare`, `Project::record` | The two halves: `prepare` applies, bumps, validates and re-derives the patch, touching nothing; `record` mints the id, appends, advances and writes. A dry run *is* `prepare`. | done |
| `src/tools.rs` | The typed tools of §5, as pure functions from arguments to operations. No I/O, no validation of what §4.4 already covers. | in progress |
| `tests/tools_devices.rs` | `add_track`, `set_track_instrument`, `add_effect`, `set_param`, driven through `Session`. | done |
| `tests/tools_clips.rs` | `add_clip`, `set_notes`, `transpose`, `quantize`, `add_automation`, `set_tempo`, the two section tools. | done |
| `src/merge.rs` | Three-way conflict detection by RFC 6902 path (ADR 0001 §4). No content-aware resolution: a structured error is actionable, a silent choice is a song nobody wrote. | done |
| `tests/tools_merge.rs` | `merge_branch`: both sides of the conflict line, `version` resolution, and the auto-merge that produces an invalid song. | done |
| `tests/tools_branches.rs` | `create_branch`, `switch_branch`, `delete_branch`. Mostly one claim from several angles: leaving a branch and coming back is byte for byte. | done |
| `src/mcp.rs` | The MCP surface: `ServerHandler`, the advertised tool list, and the two byte-level exceptions ADR 0006 §6 names. Translation only. | done |
| `src/bin/escribass-mcp.rs` | The server binary. Project as a launch argument; `--seed-ids` / `--fixed-clock` make a session reproducible. | done |
| `tests/mcp.rs` | Driven as a real subprocess over real pipes — where this layer's failures actually live. | done |
| `src/descriptor.rs` | `tool_schemas`: the protobuf descriptor turned into one JSON Schema per tool (ADR 0006 §6). Proto3 JSON's own mapping, with proto field names. | done |
| `tests/descriptor.rs` | Correspondence with the proto, one mapping rule per test. Every failure here is otherwise silent. | done |
| `src/session.rs` | The tool API, implemented once: `Session`, the reads, `apply_patch`, the summary, `-0.0` normalisation on input, and `new_song`. Both transports dispatch here and decide nothing. | done |
| `tests/session.rs` | Dry run equals the recorded patch; refusals keep their rule; the `Ok(valid=false)` / `Err` line. | done |
| `src/version.rs` | `bump_versions` (ADR 0005 §2) and `version_writes`, the guard behind `version_not_writable`. Operates on `Value`, like `patch.rs`. | done |
| `tests/version.rs` | The rule, one case per test, plus three through the pipeline. | done |
| `src/id.rs` | `IdSource`: `UlidSource` (production) and `SeededIds` (deterministic). Crockford base32, monotonic within a millisecond. | done |
| `src/clock.rs` | `Clock`: `SystemClock` and `FixedClock`. Every clock yields whole milliseconds. | done |

## Commands

```
export PATH="$HOME/.cargo/bin:$PATH"
cargo test -p escribass-core
```

## Rules

| Rule | Set by |
|---|---|
| Never define a model type here; use `escribass_schema::song` / `::history` | specs §4.1 |
| No unseeded randomness and no wall clock: the id source and the clock are constructor parameters, never globals. `SystemClock` is the only place `core` reads wall-clock time; `UlidSource` the only place it takes entropy | specs §11; CLAUDE.md #3; ADR 0001 §5 |
| Entropy comes from `std`'s `RandomState`, not a crate: ULID's tail is a uniqueness requirement, not a secrecy one. Marked `ponytail:` in `src/id.rs` with the upgrade path | CLAUDE.md #4 |
| Mutations are JSON Patch through the tool API, in tests too — never a direct field write to a stored song | CLAUDE.md #2; specs §5, §14.3 |
| `commit` re-deserialises through `Song` before recording. An op can be legal JSON and illegal for the schema; applying it to a `Value` alone would succeed and produce a document `core` cannot read | ADR 0002 §11 |
| Every check in `commit` happens before `self` is touched, so a rejected commit leaves no orphan entry and no advanced ref | specs §5 |
| The `version` bump runs inside `prepare`, between applying the ops and re-deserialising — the only position where it lands in the diff `prepare` re-derives. Anywhere later and the log replays one version behind `song.json` | ADR 0005 §1 |
| An entity is an object with a string `id` **and a numeric `version`**. Both halves: `PluginRef.version` is a string naming a plugin release, and bumping it would break `set_track_instrument` | ADR 0005 §2 |
| A tool-authored op that writes an entity `version` is refused, not silently overwritten. `bump_versions` would discard it anyway, which is the reason to say so | ADR 0005 §3 |
| `prepare` fails with `Vec<Violation>`, `record` with `ProjectError`. That *is* ADR 0006 §2's line — `prepare` touches no file, so every way it fails is caller-fixable — so the session needs no classifier over rule names | ADR 0006 §2 |
| `-0.0` is normalised in `session.rs`, where values enter. The validator's `negative_zero` catches what gets past; it should never fire on tool input | ADR 0002 §4 |
| Tool schemas are derived from `escribass_proto::DESCRIPTOR`, never hand-written. A hand-written schema drifts, and the only symptom is a model that never learns a field exists | ADR 0006 §6 |
| A tool with no proto comment has no description. `tests/descriptor.rs` fails on one, so documenting an RPC is not optional | ADR 0006 §6 |
| `ToolResult.patch` and `PatchEntry.ops` are `bytes`, so the generated serde impl base64s them. MCP payloads are built field by field, with the patch parsed into a JSON array | ADR 0006 §6 |
| A song sent over MCP is rendered by `to_canonical_json`, never `serde_json::to_value` — the text block carries the model's field order | `src/canonical.rs`; ADR 0002 §4 |
| Nothing but the transport writes to stdout in `escribass-mcp`. One `println!` desynchronises the JSON-RPC stream; diagnostics go to stderr | §18.2 |
| `mcp::IMPLEMENTED` advertises only tools that are wired up. Advertising one that is not spends a model's turn on a call that can only fail | ADR 0006 |
| A tool checks only what the validator structurally cannot: that an argument names something in *this* song, and that a double is finite before it becomes a JSON number. Everything else is §4.4's | specs §5 |
| A tool builds entities at `version: 0` and lets the pipeline take them to 1. Setting a version in a tool would be writing a field core owns | ADR 0005 §2, §3 |
| Tools emit `add`, not `replace`: RFC 6902 `add` on an existing object member replaces it, so one op covers both cases and no tool has to ask the document which it is in | RFC 6902 |
| An instrument track is created *with* its instrument. ADR 0002 requires `instrument` present exactly when `kind == INSTRUMENT`, so a two-call sequence would pass through a document §4.4 refuses | ADR 0002 |
| A tool that edits notes emits **one op per note**, never one for the clip: merge auto-resolves by comparing op paths, so a coarse patch makes every pair of edits to one clip conflict | ADR 0001 §4 |
| `quantize` is integer arithmetic with ties rounding up. A float round would depend on the platform's rounding mode exactly where a deliberately placed note sits | specs §11 |
| A tool refuses an id it does not recognise rather than skipping it. A caller whose misspelled id returns success has been told its edit landed when it did not | specs §5 |
| `History::patch_to` takes `&self` and `History::switch` takes `&mut self`. A dry-run switch built on the mutating one would move `HEAD` in memory, and the next commit would land on a branch nobody chose. The borrow checker is the guard, not a test | ADR 0001 §2; ADR 0006 §3 |
| The branch tools append no entry, so they skip `run` entirely. `entry_id` stays empty because a branch operation is not a commit | ADR 0001 §2 |
| Merge compares op **paths**, and `version` leaves are excluded. Both branches bump `version` on every entity they touch, so counting it would make every merge conflict by construction | ADR 0001 §4 |
| A merge goes through `prepare_merge`, not `prepare`: its ops legitimately carry entity versions, which are core's own and are what ADR 0005 §2 resolves to `max + 1`. Stripping them would let a client's version go backwards | ADR 0005 §2, §4 |
| A transport translates and decides nothing. Anything a caller could get two different answers to from gRPC and MCP belongs in `session.rs` | ADR 0006 |
| Never serialise a song through `serde_json::Value`: its `Map` is a `BTreeMap` and sorts struct field names as well as map keys, silently changing the canonical form | `src/canonical.rs` module note; ADR 0002 §4 |
| The document has no arrays — every collection is a map keyed by entity id — so `patch` rejects one rather than implementing index handling that cannot be reached | ADR 0001 §3 |
| RFC 6901 escaping and unescaping live together in `src/patch.rs`; they must stay exact inverses | RFC 6901 |
| Never write `patches/*.json` with the generated `PatchEntry` serde impl: `ops` is `bytes`, so it base64-encodes, producing a file that round-trips and still violates ADR 0001 §1 and §2.6. `src/history.rs` owns the disk form | ADR 0002 §11 |
| Replay starts from a default `Song`, never `{}`: the canonical form emits every no-presence field, so `replace` is legal from the first op | ADR 0002 §4; ADR 0004 |
| The log is authoritative and `song.json` is a derived cache: `open` replays and compares, reporting a mismatch rather than repairing it | ADR 0004 |
| Write order is entries, then `song.json`, then `refs.json` — the ref flip is the commit point, so a crash leaves an inert orphan | ADR 0004 |
| `switch` moves `HEAD` and appends nothing — history that recorded navigation would grow every time somebody looked at a branch | ADR 0001 §2 |
| `diff` recurses to the leaf. A coarse whole-subtree diff would make every pair of edits to one track collide on the same path and conflict under merge | ADR 0001 §4 |
| `serde_json` carries the `float_roundtrip` feature | ADR 0002 §4 |
| The writer rejects non-finite doubles; it does not rewrite values. `-0.0` is the tool API's to normalise, presence the validator's, timestamp precision the clock's | ADR 0002 §4, as amended 2026-09-02 |
| A new dependency needs asking first, then a `lock.baseline.json` entry | CLAUDE.md #4; specs §17 |

## Adding things

- **A rule to the validator:** it belongs in ADR 0002 Consequences first if it is implied by
  the schema shape, or §4.4 if it is architectural. Add a `check_*` in `src/validate.rs` and a
  test in `tests/validate.rs` that breaks the fixture in exactly one way. `rule` ids are a
  stable API — tests and the AI orchestrator match on them, so they do not change with wording.
- **Two limits are structural, not oversights.** A `DeviceRef` is checked as well-formed but
  not resolved against `lock.json` (needs the project store); a `ParamRef` is checked to name a
  device in this song but not a real parameter of it (needs the plugin manifest, M1). Both are
  marked in the code.
- **A normalisation:** decide where the value *enters* before writing code. The writer is not
  the default answer — see the Rules row above and ADR 0002 §4.
- **A test:** `tests/*.rs`. Anything asserting byte-level output reads the shared fixture
  rather than embedding JSON, so a canonical-form change shows up in one place
  (`/tests/AGENTS.md`).
