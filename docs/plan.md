# Delivery plan

Status as of 2026-09-02. This file tracks **state**: what is done, what is next, and what
was deliberately put off. It does not define the milestones — `docs/specs.md` §16 does — and
it does not set rules — `CLAUDE.md` does. When they disagree, they win and this file is
stale.

## Where we are

| Step | Deliverable | Status | Commit |
|---|---|---|---|
| — | Docs, wireframes, ADR 0001 (patch DAG + named refs) | done | `2bc4442` |
| M0.1 | `schema/song.proto`, `history.proto`, codegen for Rust/TS/Python | done | `c0dc9da` |
| M0.1 | Two determinism defects found by review, fixed | done | `b402dc0` |
| M0.1 | `schema/` restructured; AGENTS.md files | done | `5fe160c` |
| — | CI running the four checks; §13 scoped to source directories | done | `89aead1` |
| M0.2 | `core/`: canonical writer, non-finite rejection | done | `HEAD` |
| M0.2 | `core/`: validator | done | `HEAD` |
| M0.2 | `core/`: injectable id source and clock | done | `HEAD` |
| M0.2 | `core/`: RFC 6902 apply, diff | in review | PR #5, #6 |
| M0.2 | `core/`: on-disk history shape, the patch DAG | in review | PR #7, #8 |
| M0.2 | `core/`: `.escri` project store | in review | PR #10 |
| M0.2 | `core/`: `create` and `commit` | in review | PR #11 |
| M0.3 | `proto/SongTools` gRPC with `dry_run`, and the same tools over MCP | **next** | — |
| M0.3 | `proto/SongTools` gRPC with `dry_run`, and the same tools over MCP | not started | — |
| M0.4 | Schema fixtures and determinism suite | not started | — |

Nothing is pushed. C++ codegen waits for M1 (`CLAUDE.md`, M0 step 1).

## M0.2 — `core/`

A Rust crate at `core/`, workspace member, depending on `escribass-schema = { path = "../schema" }`.
Scope is fixed by decisions already made, not open for redesign:

- **Validator** — §4.4, plus the rules ADR 0002 Consequences lists: `key == value.id` on every
  map; required oneofs set; no enum left `*_UNSPECIFIED`; `Effect.index` unique per chain and
  `Track.index` unique; exactly one `MASTER`; `output_track_id` and send keys resolve to `BUS`
  or `MASTER`; sidechain keys resolve to effects on that track; `ParamRef.device_id` resolves;
  tempo map non-empty with an event at tick 0; all doubles finite. Also: ULID keys
  canonicalised to uppercase Crockford, and duplicate JSON object keys rejected rather than
  last-wins.
- ~~**Injectable id source** and **clock**~~ — done. `UlidSource` takes entropy from `std`'s
  `RandomState` rather than a new dependency; ULID's tail is a uniqueness requirement, not a
  secrecy one.
- **Canonical writer** (ADR 0002 §4) — `serde_json` with `float_roundtrip`; rejects non-finite
  doubles; normalises `-0.0`; normalises message-field presence; timestamps `Z`-suffixed at
  millisecond precision.
- ~~**Patch log as a DAG**~~ and ~~**project store**~~ — done. Six PRs: apply, diff, the
  on-disk shape, the DAG, the store, and `create`/`commit`.

## M0.3 — tool API

`proto/SongTools` (a new top-level directory, already allowed by §13; add `- path: proto` to
`buf.yaml`). Every tool takes `dry_run`. Beyond §5's list: `create_branch`, `switch_branch`,
`delete_branch`, `merge_branch` (ADR 0001 §4 — auto-merge disjoint paths, structured error on
conflict). The same tools are exposed over MCP in the same step; §18.2 calls that a hard
requirement, not a nice-to-have.

## M0.4 — determinism suite

In `tests/`. Drives `core` through the tool API, never the file (`CLAUDE.md` #2). Same input
→ identical canonical JSON and identical patch log, byte for byte.

## After M0

One line each; §16 has the definitions, and ADR 0003 placed what §16 had left out. M1 render engine and first golden render · M2 Tauri UI
· M3 AI loop · M4 compilers · M5 interop and installer.

## Deferred, on purpose

Each of these was raised, judged, and put off. None is forgotten; none is blocking.

| Item | Why deferred | Revisit at | Source |
|---|---|---|---|
| `FormRule` | Least-specified entity in §4; nothing consumes it before the generative compiler | M4 | ADR 0002 §7 |
| `Instrument.state` as a content hash instead of inline `bytes` | Plugin states are large base64 in a file §2.6 wants diffable — but adding a hash field and deprecating `state` is additive, not breaking | before M1 renders a plugin | review, 2026-09-02 |
| `ParamRef` reaching track mix params (gain, pan, mute) | The commonest automation in any DAW is not addressable today; additive to fix | M2, when the mixer exists | review, 2026-09-02 |
| Dense unique `index` on tracks and effects | Inserting mid-list renumbers everything, and two branches inserting at one index auto-merge into an invalid document | M0.3, with merge | review, 2026-09-02 |
| Interactive merge conflict resolution | Designing the API with no UI and no real conflicts | M2 | ADR 0001 §4 |
| Garbage collection of orphaned patch entries | Entries are small and inert | only if a real project makes it a problem | ADR 0001 Deferred |
| `SourceRef.export_hash`, `Generator` compiled-source hash | Needed for "export pending" and "compiled · stale"; nothing produces either yet | M4 | ADR 0002 Consequences |
| Strudel as a second `Generator.kind` | Python DSL is the v1 target | after M4 | §15 |
| `schema/pyproject.toml` `[build-system]` | Consumers use `sys.path`; no wheel needed yet | when `ai/` depends on it | `schema/AGENTS.md` |
| Native CLAP hosting | VST3 via clap-wrapper is the mature path | never a dependency | §8 |
| Entity `version` bumping | Core maintains it (ADR 0001 §4), but the bump must be recorded **in the entry's ops** or replay diverges from the live document. No tool drives it until M0.3, so designing it now means designing against no caller | M0.3 | ADR 0001 §4 |
| Undo/redo policy | §5 calls the log the undo history and ADR 0001 names the mechanism, but nothing chooses between rewinding the ref and appending an inverse entry. ADR-shaped when taken | M0.3 | §5; ADR 0001 |
| `lock.json` beyond `schema_version` | Nothing to pin until compiled artefacts and models exist | M1, M4 | ADR 0003 §3; §17 |

## Known gaps

- **CI has never run.** `.github/workflows/checks.yml` exists as of `89aead1` and every
  command in it is verified from a clean clone, but nothing is pushed, so GitHub has not
  executed it once. `setup-node` resolving `"25"`, the cache action and the `GITHUB_BASE_REF`
  substitution are unproven until the first push.

## Open — not ours to decide

`docs/specs.md` §15 marks these `[OPEN]`; `CLAUDE.md` says stop and ask. None blocks M0.

- Neural runtime packaging: ONNX Runtime linked into `engine`, or a separate process. Now due
  before M4, which is where the neural runtime lands (ADR 0003 §7).
- Minimum supported OS versions.
- Symbolic model choice for v1 melody and drum generation.
- Whether §6's analysis features (key, chord and structure detection, tempo estimation, stem
  separation) and symbolic generation are v1 scope at all (ADR 0003, Still unplaced).
