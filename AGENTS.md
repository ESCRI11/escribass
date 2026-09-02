# AGENTS.md — repository root

`CLAUDE.md` is binding and is not repeated here. Read it, then `docs/specs.md` §13 (layout), §4, §5, §11, §17.

## What is here

| Path | Role | Written by |
|---|---|---|
| `schema/` | The song model: `*.proto`, codegen, generated types and tests for Rust, TypeScript, Python. See `schema/AGENTS.md`. | hand + `schema/codegen.sh` |
| `tests/` | Cross-language fixtures now; determinism suite at M0.4. See `tests/AGENTS.md`. | tests |
| `docs/` | `specs.md` (architecture source of truth), `adr/` (decisions), `landscape-2026-09.md`, `wireframes.html`. | hand |
| `Cargo.toml` | Cargo workspace. Members: `schema`; `core` joins at M0.2. | hand |
| `Cargo.lock` | Integrity hashes for crates.io packages (specs §17). Never edit. | cargo |
| `buf.yaml` | buf workspace: module `schema` now, `proto` at M0.3; lint and breaking config. Lives here, not in `schema/`, because buf v2 wants one `buf.yaml` at the common ancestor of every module. | hand |
| `rust-toolchain.toml` | Rust 1.98.0; mirrors `lock.baseline.json` `schema.rust.toolchain`. | hand |
| `lock.baseline.json` | Every pinned dependency and toolchain (specs §17). | hand |

Top-level directories are fixed by specs §13. `core/`, `proto/`, `app/`, `ai/`, `compilers/`, `engine/` do not exist yet; M0 creates only `core/` (M0.2) and `proto/` (M0.3). Any directory not in §13 needs an ADR first (CLAUDE.md, Repo layout).

Never at the root: source code, generated code, project files, or any representation of song state other than `schema/song.proto` (CLAUDE.md #1).

## Toolchain

| Tool | Version | Install | Pinned in |
|---|---|---|---|
| Rust | 1.98.0 | `rustup` reads `rust-toolchain.toml` | `lock.baseline.json` → `schema.rust` |
| `protoc-gen-prost`, `protoc-gen-prost-serde` | 0.5.0, 0.4.0 | `cargo install protoc-gen-prost@0.5.0 protoc-gen-prost-serde@0.4.0` | same |
| Node | ≥ 22 | — | `schema.typescript.node` |
| `buf`, `protoc-gen-es`, `tsx`, `typescript` | see lock | `cd schema && npm ci` | `schema/package-lock.json` |
| Python, `uv`, `betterproto2-compiler` | 3.12 | `cd schema && uv sync` | `schema/uv.lock` |

## Checks — all four before any step is called done (CLAUDE.md, Working style)

```
export PATH="$HOME/.cargo/bin:$PATH"
./schema/codegen.sh --check
cargo test
cd schema && npx tsc --noEmit && node --import tsx --test tests/*.test.ts
cd schema && uv run python -m unittest discover -s tests
```

## Rules that cross directories

| Rule | Set by |
|---|---|
| `schema/song.proto` is the only representation of song state | CLAUDE.md #1; specs §2.1, §14.2 |
| Schema change: ADR in `docs/adr/` first, then a row in specs §15 | CLAUDE.md #5; specs §14.1 |
| No hand-written model types in any language | specs §4.1 |
| All mutations are JSON Patch through the tool API, tests included | CLAUDE.md #2; specs §5, §14.3 |
| No unseeded randomness, no wall clock, in `core`, compilers, `engine` | CLAUDE.md #3; specs §11; ADR 0001 §5 |
| New dependency: ask first; pin in `lock.baseline.json`; registry packages by exact version with hashes in the lockfiles | CLAUDE.md #4; specs §17 |
| JUCE is never upgraded independently of Tracktion Engine | CLAUDE.md #4; specs §17 |
| Engine is schema-agnostic; AI is audio-agnostic | CLAUDE.md #6; specs §14.6 |

## Adding things

- **A top-level directory:** ADR, then a line in specs §13, then the directory.
- **An ADR:** `docs/adr/NNNN-slug.md`, next number (`0003`). Copy the header and section structure of `docs/adr/0002-song-proto-v1.md`. Add the decision to specs §15 in the same PR.
- **A Rust crate (`core/` at M0.2):** its §13 directory; add it to `members` in `Cargo.toml`; depend on the model with `escribass-schema = { path = "../schema" }`.
- **A proto module (`proto/` at M0.3):** add `- path: proto` under `modules:` in `buf.yaml`. `import "song.proto"` resolves across modules in the workspace.
- **A consumer of the TypeScript types (`app/`):** depend on `schema/` as a path package, `@escribass/schema`.
- **A consumer of the Python types (`ai/`):** put `schema/gen/python` on `sys.path` as `schema/tests/test_roundtrip.py` does, until `schema/pyproject.toml` gains a `[build-system]` (see its comment).
