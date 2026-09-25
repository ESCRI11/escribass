# AGENTS.md — repository root

`CLAUDE.md` is binding and is not repeated here. Read it, then `docs/specs.md` §2, §4, §5, §11 (the set specs §14.1 requires), §13 (layout) and §17 (pins).

## What is here

| Path | Role | Written by |
|---|---|---|
| `schema/` | The song model: `*.proto`, codegen, generated types and per-language round-trip tests. See `schema/AGENTS.md`. | hand + `schema/codegen.sh` |
| `proto/` | The tool API: `song_tools.proto`, the wire contract of §5, and `render.proto`, the engine's. Generated Rust and — since M2 PR 3 gave it a consumer — TypeScript. See `proto/AGENTS.md`. | hand + `proto/codegen.sh` |
| `core/` | Rust: model round-trip, validator, patch log, project store. See `core/AGENTS.md`. | hand |
| `tests/` | Cross-language fixtures and the determinism suite (`escribass-tests`). See `tests/AGENTS.md`. | hand + tests |
| `app/` | The desktop UI: the Tauri host in `app/src-tauri/` (Rust, embedding `core`) and the frontend at `app/` (React, Vite). One decoded `Song` and pure selectors over it (ADR 0012 §2); one gesture, previewed with `dry_run` at every position and committed once (ADR 0017). | hand |
| `docs/` | `specs.md` (architecture source of truth), `adr/`, `landscape-2026-09.md`, `wireframes.html`, `plan.md`, `roadmap.md`. See `docs/AGENTS.md` and `docs/adr/AGENTS.md`. | hand |
| `Cargo.toml` | Cargo workspace. Members: `schema`, `proto`, `core`, `tests`, `app/src-tauri`. The last is **not** a default member: building it needs WebKitGTK's development headers and a built `app/dist`, so `cargo test` skips it and CI's `app` job runs `cargo test -p escribass-app` instead. | hand |
| `Cargo.lock` | Integrity hashes for crates.io packages (specs §17). Never edit. | cargo |
| `buf.yaml` | buf workspace: modules `schema` and `proto`; lint and breaking config. `proto` relaxes three STANDARD rules that contradict §5's service shape. At the root, not in `schema/`, because buf v2 wants one `buf.yaml` at the common ancestor of every module. | hand |
| `rust-toolchain.toml` | Rust 1.98.0; mirrors `lock.baseline.json` `schema.rust.toolchain`. | hand |
| `lock.baseline.json` | Every pinned dependency and toolchain (specs §17). | hand |

Top-level directories are fixed by specs §13. `compilers/` does not exist yet — M4 PR 4 creates `compilers/generative/` (ADR 0024 §8); M0 created `core/` (M0.2) and `proto/` (M0.3), M1 PR 5 created `engine/`, M2 PR 2 created `app/`, and M3 PR 5 created `ai/`. Any directory not in §13 needs an ADR first (CLAUDE.md, Repo layout).

Never at the root: source code, generated code, project files, or any representation of song state other than `schema/song.proto` (CLAUDE.md #1).

## Toolchain

| Tool | Version | Install | Pinned in |
|---|---|---|---|
| Rust | 1.98.0 | `rustup` reads `rust-toolchain.toml` | `lock.baseline.json` → `schema.rust` |
| `protoc-gen-prost`, `protoc-gen-prost-serde` | 0.5.0, 0.4.0 | `cargo install protoc-gen-prost@0.5.0 protoc-gen-prost-serde@0.4.0` | same |
| `buf` | also drives `proto/`; the workspace `buf.yaml` covers both modules | `cd schema && npm ci` | `schema/package-lock.json` |
| Node | 25.6.1 (`protoc-gen-es` needs ≥ 22) | — | `schema.typescript.node` |
| `buf`, `protoc-gen-es`, `tsx`, `typescript` | see lock | `cd schema && npm ci` | `schema/package-lock.json` |
| `@bufbuild/protobuf` for `proto/gen/ts` | 2.14.1, the version `schema` already pins | `npm --prefix proto ci` | `proto/package-lock.json` |
| Python, `uv`, `betterproto2-compiler` | 3.12 | `cd schema && uv sync` | `schema/uv.lock` |
| The AI sidecar's own environment (`ai/`, M3 PR 5) | Python 3.12.12, `grpclib`, `openai`, `httpx2` | `cd ai && uv sync` | `ai/uv.lock` |

`make` lists the shortcuts for all of this — running the app, the Vite loop, a scratch
project, and the checks below. It shells out to exactly these commands and CI does not use it.

## Checks — all four before any step is called done (CLAUDE.md, Working style)

```
export PATH="$HOME/.cargo/bin:$PATH"
./schema/codegen.sh --check
./proto/codegen.sh --check
cargo test
cd schema && npx tsc --noEmit && node --import tsx --test tests/*.test.ts
npm --prefix proto ci && cd schema && npx tsc --noEmit --project ../proto
cd schema && uv run python -m unittest discover -s tests
cd schema && uv run python -m unittest discover -s ../proto/tests
cd ai && uv sync --locked && uv run python -m unittest discover -s tests
cargo test -p escribass-tests --features ai   # after the line above: it spawns the real `ai`
```

The last two are the newer halves of the Python check: `proto/`'s generated tree imports and
reaches *the* model (M3 PR 3), and the AI sidecar starts, names a socket, drives a two-call
turn from a transcript and leaves with a status (M3 PR 5, extended in PR 8). Both run in an
environment of their own — `proto/` borrows `schema`'s, `ai/` has one, with the exact Python
patch §17 pins.

The `ai` **cargo feature** is the loop's end-to-end golden (M3 PR 8): a prompt driven through
the real `ai` process with a scripted provider and `core`'s own client, the proposal applied,
the project compared with committed bytes and driven twice. It is a feature rather than a test
that notices `uv` is missing and returns, for the reason `renders` is (`tests/AGENTS.md`). Its
**live** half spends money and is `#[ignore]`d: nothing in this repository calls a paid service
without the user saying so (CLAUDE.md #7).

`cargo test` builds the four default members. The fifth, `app/src-tauri`, needs
`libwebkit2gtk-4.1-dev` and a built `app/dist`; where both are present, add:

```
npm --prefix schema ci --omit=dev   # `@escribass/schema` is a `file:` link, and its own
                                    # imports resolve from `schema/node_modules`, not app's
cd app && npm ci && npx tsc --noEmit && npm run build
cargo test -p escribass-app --features custom-protocol   # the shipping path: without the
                                                         # feature the host dials `devUrl`
                                                         # and never embeds `app/dist`
```

## Working by pull request

`main` is never committed to directly (CLAUDE.md, Working style). One branch and one PR per
milestone step.

```
git switch -c m0.2-validator          # <milestone>-<step>
# ... work, and run the four checks above locally
git push -u origin m0.2-validator
gh pr create --fill
gh pr checks --watch
gh pr merge --squash --delete-branch
```

Two hooks live in `.githooks/`. `pre-push` refuses a direct push to `main`: it is
client-side, because GitHub branch protection and rulesets both require Pro on a private
repository, so the server will accept a direct push and this is what stops one being sent.
`commit-msg` enforces the seven rules of https://chris.beams.io/posts/git-commit/ against
each commit message. Both are client-side; `--no-verify` bypasses either deliberately. A
fresh clone must enable them:

```
git config core.hooksPath .githooks
```

`.github/workflows/checks.yml` runs the four checks on every push and PR. It additionally
runs `buf breaking` against the PR's base branch — that check exists only on pull requests,
because comparing a branch against itself proves nothing.

## Rules that cross directories

| Rule | Set by |
|---|---|
| `schema/song.proto` is the only representation of song state | CLAUDE.md #1; specs §2.1, §14.2 |
| Schema change: ADR first; what lands with it is in `docs/adr/AGENTS.md` | CLAUDE.md #5; specs §14.1 |
| No hand-written model types in any language | specs §4.1 |
| All mutations are JSON Patch through the tool API, tests included | CLAUDE.md #2; specs §5, §14.3 |
| No unseeded randomness, no wall clock, in `core`, compilers, `engine` | CLAUDE.md #3; specs §11; ADR 0001 §5 |
| New dependency: ask first; pin in `lock.baseline.json`; registry packages by exact version with hashes in the lockfiles | CLAUDE.md #4; specs §17 |
| Engine is schema-agnostic; AI is audio-agnostic | CLAUDE.md #6; specs §14.6 |

## Adding things

- **A top-level directory:** ADR, then its line in specs §13, then the directory.
- **An ADR:** `docs/adr/AGENTS.md`.
- **A Rust crate (`core/` at M0.2):** its §13 directory; add it to `members` in `Cargo.toml`.
- **A proto module (`proto/` at M0.3):** add `- path: proto` under `modules:` in `buf.yaml`. `import "song.proto"` resolves across modules in the workspace.
- **Depending on the model from any crate or package:** `schema/AGENTS.md`, Consumers.
