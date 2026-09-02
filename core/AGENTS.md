# AGENTS.md — core/

Crate `escribass-core`. Everything that reads or writes song state. Model types come from
`escribass-schema` and are never redefined here (specs §4.1). Repo-wide rules: `/AGENTS.md`.
The schema itself: `/schema/AGENTS.md`.

## Files

| Path | What | Status |
|---|---|---|
| `src/canonical.rs` | Canonical JSON persistence: `to_canonical_json`, `from_canonical_json`, `CanonicalError`. | done |
| `tests/canonical.rs` | Fixed-point, idempotence, non-finite rejection, seeded double round-trip, key order. Reads `/tests/fixtures/song/minimal.json`. | done |
| `src/validate.rs` | `validate` → every `Violation` (path, stable rule id, message), not just the first. §4.4 plus the rules in ADR 0002 Consequences. | done |
| `tests/validate.rs` | One rule per test, each breaking the fixture in exactly one way. | done |
| patch log, `refs.json`, `.escri` project store | ADR 0001 §1–§2; specs §10 | M0.2, next |
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
| Never serialise a song through `serde_json::Value`: its `Map` is a `BTreeMap` and sorts struct field names as well as map keys, silently changing the canonical form | `src/canonical.rs` module note; ADR 0002 §4 |
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
