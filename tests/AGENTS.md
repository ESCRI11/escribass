# AGENTS.md — tests/

Shared, cross-language test inputs, and the determinism suite (CLAUDE.md, M0 step 4; specs §11, §13). Per-language unit tests live with their package (`schema/tests/`), not here.

This is a Cargo package, `escribass-tests`, with no library: suites and their inputs only.

| Path | What | Written by |
|---|---|---|
| `fixtures/history/patch_entry.json`, `fixtures/history/refs.json` | The on-disk forms of ADR 0001 §1 and §2, written by `core/tests/history.rs`. | `UPDATE_FIXTURES=1 cargo test -p escribass-core` |
| `fixtures/song/minimal.json` | Canonical JSON (ADR 0002 §4) of the song built by `build()` in `schema/tests/roundtrip.rs`. Read by the Rust, TypeScript and Python round-trip tests. | `UPDATE_FIXTURES=1 cargo test -p escribass-schema` |
| `fixtures/manifest.json` | The build manifest every suite validates against (ADR 0010 §4): a **subset** of a real `escribass_engine --scan`, with the machine's plugin paths dropped. Read by `core/tests/common/mod.rs` and by the determinism suite, which passes it to both binaries as `--manifest`. | hand, from a real scan; see below |
| `determinism.rs` | The suite: it drives `escribass-mcp` and `escribass-grpc` as subprocesses — each given `--manifest fixtures/manifest.json` — and compares what they produce. | hand |
| `determinism/<name>/script.json` | A scripted session: `[{tool, args, refused?}]`. One script, one claim. | hand |
| `determinism/<name>/expected/` | What that script produced when it was last blessed: the project's files, plus `responses.json`, `origin.json` and `plan.json` — what `compile` says about the project, the `RenderPlan` or every reason there is none (ADR 0007 §4). | `UPDATE_FIXTURES=1 cargo test` |

## Rules

| Rule | Set by |
|---|---|
| Never hand-edit a fixture; the Rust test writes it from generated types | `schema/tests/roundtrip.rs` header |
| Determinism goldens come through the tool API. The **schema fixture** does not: it exercises `Generator`, `Marker`, `Instrument.state` and model provenance that no typed tool can produce before M4, and it is a constructed value rather than a mutation, so CLAUDE.md #2 is not in play | CLAUDE.md #2; ADR 0003 |
| A determinism script names entity ids **literally**. Under `--seed-ids` an id is a pure function of how many were minted before it, so a change in mint order changes what a script means — and should fail loudly rather than quietly still passing | specs §11 |
| A dry run mints from a discarded fork, so it consumes no id. A script's ids follow its *applied* steps only | ADR 0006 §3 |
| A patch log is **not self-contained**: replay starts from a default `Song`, not `{}`, because the canonical form emits every no-presence scalar and map. `origin.json` is committed beside the log so anything can replay it | ADR 0002 §4, §11 |
| A script writes `patch` as an RFC 6902 array. Each driver adapts it the way its own transport does — MCP rewrites it to canonical text, the gRPC driver base64s it for the `bytes` field. That asymmetry belongs to the proto, not to what the two transports mean | ADR 0006 §6 |
| `--author` is passed to both binaries explicitly: `escribass-grpc` defaults to `human` and `escribass-mcp` to `model`, which would show up in every `provenance.author` | `core/src/bin/` |
| The **manifest fixture is not the manifest**. The real one is generated at build time and never committed, because a committed description of a plugin binary is a second pin with the stale one silent. This is a test fixture holding the ids the fixtures use, taken from a real scan — and the engine CI job asserts it is still a subset of what the build declares, which is the only thing keeping it honest | ADR 0010 §4 |
| A plugin id and a parameter id in any fixture is one a real plugin reported. `com.surge-synth.surge-xt` and `org.surge-synth.surge-xt` were both invented and did not even agree with each other, which is how you can tell nothing ever checked them | ADR 0010 §4; M1 PR 9 |
| Every golden `lock.json` carries the fixture manifest's `engine` block, so **moving a §17 pin moves the fixture and four goldens in one pull request**. That is §17's golden-render rule reaching the determinism suite | ADR 0010 §1, §3 |
| A fixture or a golden changes only in the PR that changes the `.proto`, the canonical form, or a tool's semantics — and its diff is reviewed there. `UPDATE_FIXTURES=1` blesses whatever ran, including a deterministically wrong output; this rule is the only guard against that | specs §17 (same rule for golden renders) |
| Two runs agreeing catches nondeterminism; only the golden catches **drift**. A dependency that changes how a float is written, or feature unification flipping `serde_json::Map` to insertion order, produces the same wrong bytes twice | specs §11 |
| Fixture inputs are byte-stable: fixed timestamps, fixed ids, no wall clock, no unseeded randomness | specs §11; ADR 0001 §5 |

## Adding things

- **A fixture:** a `build_*()` and a `*_FIXTURE` path constant in `schema/tests/roundtrip.rs`; write it with `UPDATE_FIXTURES=1`; read it from the TS and Python tests too.
- **A plugin or a parameter to `fixtures/manifest.json`:** build the engine, run `cmake --build engine/build --target manifest`, and copy the entry — id, `commit`, `version`, and only the `params` the fixtures name — out of `engine/build/manifest.json`. Never the `path`: it is machine-specific and this file is committed. Never a value typed from memory: the CI subset check is what would catch it, and catching it in CI is slower than reading it off the scan.
- **The four scripts:** `every_tool` (the whole surface is reproducible, and a preview burns nothing), `refusals` (a refused call changes nothing — not the document, not the log, not the ids the next call mints), `branches` (navigation records nothing, a merge records one entry with two parents, a conflict writes nothing), `render` (what `compile` resolves — order, loops, solo and mute, lanes, the render length — is a pure function of the document, and the plan golden is where M0's claim and M1's meet).
- **`plan.json`** is written for every script, so a project M1 cannot render goldens its refusal, naming the field. Its asset index is built from the `assets/` listing under the fixed root `/escri/assets`: `compile` reads no file, so the path is opaque to it, and a run's temporary directory in a golden would be the one kind of input the suite exists to keep out.
- **A determinism script:** a directory under `determinism/` with a `script.json`. Add a test that runs it twice and compares. Give a step `"refused": "<rule>"` when it is meant to fail, so it is checked at the step rather than surfacing later as a mismatch between two large documents.
- **A cross-language check:** `schema/tests/replay.test.ts` and `schema/tests/test_replay.py` read the golden and replay it. They compare *documents*, not bytes — neither side's serialiser is the canonical writer, and that the bytes are canonical is Rust's claim.
- **A tool:** add it to a script — `every_implemented_tool_is_scripted` enforces this — add an arm to the gRPC driver's `call!`, and run the suite — the ids of every later step shift if the new tool mints any.

## Running it

```
cargo test                      # from the workspace root: builds the binaries, then runs
cargo test -p escribass-tests   # only works if the binaries are already built
```

`CARGO_BIN_EXE_<name>` is not set for a package that does not own the binary, so the suite finds the binaries from the test executable's own path. It cannot build them — cargo holds the build lock while tests run — and it **refuses to run against one older than `core/src`**, because `cargo test -p escribass-tests` rebuilds the libraries and not the binaries, and a suite that validates a stale build and passes is worse than one that fails.
