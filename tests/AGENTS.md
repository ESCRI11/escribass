# AGENTS.md — tests/

Shared, cross-language test inputs, and the determinism suite (CLAUDE.md, M0 step 4; specs §11, §13). Per-language unit tests live with their package (`schema/tests/`), not here.

This is a Cargo package, `escribass-tests`, with no library: suites and their inputs only.

| Path | What | Written by |
|---|---|---|
| `fixtures/history/patch_entry.json`, `fixtures/history/refs.json` | The on-disk forms of ADR 0001 §1 and §2, written by `core/tests/history.rs`. | `UPDATE_FIXTURES=1 cargo test -p escribass-core` |
| `fixtures/song/minimal.json` | Canonical JSON (ADR 0002 §4) of the song built by `build()` in `schema/tests/roundtrip.rs`. Read by the Rust, TypeScript and Python round-trip tests. | `UPDATE_FIXTURES=1 cargo test -p escribass-schema` |
| `determinism.rs` | The suite: it drives `escribass-mcp` as a subprocess and compares what two runs produce. | hand |
| `determinism/<name>/script.json` | A scripted session: `[{tool, args, refused?}]`. | hand |

## Rules

| Rule | Set by |
|---|---|
| Never hand-edit a fixture; the Rust test writes it from generated types | `schema/tests/roundtrip.rs` header |
| Determinism goldens come through the tool API. The **schema fixture** does not: it exercises `Generator`, `Marker`, `Instrument.state` and model provenance that no typed tool can produce before M4, and it is a constructed value rather than a mutation, so CLAUDE.md #2 is not in play | CLAUDE.md #2; ADR 0003 |
| A determinism script names entity ids **literally**. Under `--seed-ids` an id is a pure function of how many were minted before it, so a change in mint order changes what a script means — and should fail loudly rather than quietly still passing | specs §11 |
| A dry run mints from a discarded fork, so it consumes no id. A script's ids follow its *applied* steps only | ADR 0006 §3 |
| A fixture changes only in the PR that changes the `.proto` or the canonical form, and its diff is reviewed there | specs §17 (same rule for golden renders) |
| Fixture inputs are byte-stable: fixed timestamps, fixed ids, no wall clock, no unseeded randomness | specs §11; ADR 0001 §5 |

## Adding things

- **A fixture:** a `build_*()` and a `*_FIXTURE` path constant in `schema/tests/roundtrip.rs`; write it with `UPDATE_FIXTURES=1`; read it from the TS and Python tests too.
- **A determinism script:** a directory under `determinism/` with a `script.json`. Add a test that runs it twice and compares. Give a step `"refused": "<rule>"` when it is meant to fail, so it is checked at the step rather than surfacing later as a mismatch between two large documents.
- **A tool:** add it to a script, and run the suite — the ids of every later step shift if the new tool mints any.

## Running it

```
cargo test                      # from the workspace root: builds the binaries, then runs
cargo test -p escribass-tests   # only works if the binaries are already built
```

`CARGO_BIN_EXE_<name>` is not set for a package that does not own the binary, so the suite finds `escribass-mcp` from the test executable's own path. It cannot build it: cargo holds the build lock while tests run.
