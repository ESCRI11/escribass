# AGENTS.md — tests/

Shared, cross-language test inputs. The determinism suite lands here at M0.4 (CLAUDE.md, M0 step 4; specs §13). Per-language unit tests live with their package (`schema/tests/`), not here.

| Path | What | Written by |
|---|---|---|
| `fixtures/song/minimal.json` | Canonical JSON (ADR 0002 §4) of the song built by `build()` in `schema/tests/roundtrip.rs`. Read by the Rust, TypeScript and Python round-trip tests. | `UPDATE_FIXTURES=1 cargo test -p escribass-schema` |

## Rules

| Rule | Set by |
|---|---|
| Never hand-edit a fixture; the Rust test writes it from generated types. From M0.4, fixtures come through the tool API. | CLAUDE.md #2; `schema/tests/roundtrip.rs` header |
| A fixture changes only in the PR that changes the `.proto` or the canonical form, and its diff is reviewed there | specs §17 (same rule for golden renders) |
| Fixture inputs are byte-stable: fixed timestamps, fixed ids, no wall clock, no unseeded randomness | specs §11; ADR 0001 §5 |

## Adding things

- **A fixture:** a `build_*()` and a `*_FIXTURE` path constant in `schema/tests/roundtrip.rs`; write it with `UPDATE_FIXTURES=1`; read it from the TS and Python tests too.
- **The M0.4 determinism suite:** here. Drive `core` through the tool API (CLAUDE.md #2); compare canonical JSON and the patch log byte for byte, with no normalisation step (ADR 0001 §5 and Consequences).
