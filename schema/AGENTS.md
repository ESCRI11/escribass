# AGENTS.md — schema/

The song model, in every language. `schema/` is one package three times: a Cargo crate (`Cargo.toml`), an npm package (`package.json`), a Python project (`pyproject.toml`), each serving types generated from the two `.proto` files here. The choices the protos make and why: `docs/adr/0001-song-history-as-patch-dag.md`, `docs/adr/0002-song-proto-v1.md`, `docs/specs.md` §4. Repo-wide rules, toolchain and the four checks: `/AGENTS.md`.

## Files

| Path | What | Hand or generated |
|---|---|---|
| `song.proto` | Package `escribass.song.v1`. The only representation of song state (CLAUDE.md #1). | hand |
| `history.proto` | Package `escribass.history.v1`: patch-DAG transport, `PatchEntry` and `Refs`. Imports `song.proto`; never the reverse (ADR 0002 §6). | hand |
| `buf.gen.yaml` | One plugin entry per language target, with the options that produce the canonical JSON form (ADR 0002 §4). | hand |
| `codegen.sh` | `buf format -w`, `buf lint`, `rm -rf gen`, `buf generate`. `--check` is the drift gate: fails if `gen/` or `*.proto` differ after regenerating. | hand |
| `gen/rust/`, `gen/ts/`, `gen/python/` | Generated output, committed for review (specs §4.1). **`codegen.sh` deletes `gen/` whole on every run.** Never edit, never add a file under it. | generated |
| `Cargo.toml`, `src/lib.rs` | Crate `escribass-schema`. `lib.rs` is the module tree only; it `include!`s `gen/rust/**` by `CARGO_MANIFEST_DIR`. | hand |
| `package.json`, `package-lock.json`, `tsconfig.json` | Package `@escribass/schema`, `private`. Also installs the codegen toolchain: `buf`, `protoc-gen-es`. | hand |
| `pyproject.toml`, `uv.lock`, `.python-version` | Project `escribass-schema`: `betterproto2` runtime, `betterproto2-compiler` for codegen. No `[build-system]` (see Consumers). | hand |
| `tests/` | One round-trip test per language, all reading `/tests/fixtures/song/minimal.json`. | hand |

Never here: the validator, patch application or canonical writer (`core/`, M0.2); service definitions (`/proto`, specs §13); a project file.

## Commands

The four checks in `/AGENTS.md` all run against this directory today. Beyond them:

```
export PATH="$HOME/.cargo/bin:$PATH"
./schema/codegen.sh                                   # regenerate all targets; run from anywhere
cargo test -p escribass-schema                        # the Rust round-trip alone
UPDATE_FIXTURES=1 cargo test -p escribass-schema       # rewrite /tests/fixtures (see /tests/AGENTS.md)
schema/node_modules/.bin/buf breaking --against '.git#branch=main'   # wire compatibility, from the root
```

## Consumers

| From | Depend on it as | Set by |
|---|---|---|
| A Rust crate (`core/`, M0.2) | `escribass-schema = { path = "../schema" }`; types at `escribass_schema::song` and `::history`; `pbjson_types` is re-exported for the well-known types | `Cargo.toml`; `src/lib.rs` |
| TypeScript (`app/`) | A path dependency, `"@escribass/schema": "file:../schema"` — the package is `private`, so never a registry one — importing `@escribass/schema/song` and `/history` | `package.json` `exports`, `private` |
| Python (`ai/`) | `schema/gen/python` on `sys.path`, then `escribass_schema.escribass.song.v1`, as `tests/test_roundtrip.py` does, until `pyproject.toml` gains a `[build-system]`. The extra `escribass_schema` level exists because betterproto2 emits `from ....message_pool import …`, which needs a package above `escribass` | `pyproject.toml` comment; `buf.gen.yaml` comment |

## Rules

| Rule | Set by |
|---|---|
| Field numbers are never changed or reused; `buf breaking` with the `FILE` category is the check | `/buf.yaml` `breaking:` |
| Every entity collection is `map<string, T>` keyed by `T.id`; `repeated` only for `PatchEntry.parents` and lists never patched element-wise | ADR 0001 §3 |
| Every entity carries `string id = 1; Provenance provenance = 2; uint32 version = 3;` — `AutomationPoint`, `TempoEvent`, `TimeSignatureEvent` carry `id` only | specs §4.3; ADR 0002 §2; ADR 0001 §3, correction |
| Ticks are `int32`; the only 64-bit field is `Generator.seed` | ADR 0002 §1 |
| Sum types are `oneof` with message arms; no parallel `kind` enum | ADR 0002 §3 |
| `optional` only where absent differs from zero | ADR 0002 §5 |
| Canonical JSON — proto field names, defaults emitted, map keys sorted — comes from the `protoc-gen-prost-serde` options in `buf.gen.yaml`, never from a hand-written serializer | ADR 0002 §4; specs §4.1 |
| `syntax = "proto3"`, not Editions | ADR 0002 §10 |
| Patch ops travel as JSON text in `PatchEntry.ops` (`bytes`), never as a proto message | ADR 0002 §11 |
| Formatting is `buf format`'s; `codegen.sh` applies it | `codegen.sh` |

## Adding things

- **A field or message:** ADR first (`docs/adr/AGENTS.md`). Edit the `.proto`; `./schema/codegen.sh`; if it is a new kind of construct (map, oneof, optional, 64-bit, well-known type) extend `build()` in `tests/roundtrip.rs`; `UPDATE_FIXTURES=1 cargo test -p escribass-schema`; review the fixture diff; run the TS and Python tests, which read that fixture. Commit `gen/` and the fixture together with the `.proto`.
- **A language target:** one plugin entry in `buf.gen.yaml` with `out: gen/<lang>`; the hand-written package shell at `schema/`, not under `gen/`; a round-trip test in `tests/` reading the same fixture; the plugin version in `lock.baseline.json` under `schema`. `codegen.sh` needs no change. **C++ is not one of them** (ADR 0008 §4): the engine's own CMake generates it at build time from `song.proto` and `render.proto`, because a committed `.pb.cc` pins a `protoc` that must equal the vendored runtime. `schema/gen/` stays at three languages and `codegen.sh --check` is unaffected.
- **A test:** `tests/*.rs` (cargo auto-discovers), `tests/*.test.ts`, `tests/test_*.py`. TS and Python assert reading only; Rust writes the fixture, because `core` is the only writer of a project file (specs §5, §10).
- **A `.proto` file:** only with an ADR saying why it is not a message in an existing file (ADR 0002 §6 is the precedent). Flat at `schema/`; the lint exceptions in `/buf.yaml` cover the flat layout §13 fixes.
