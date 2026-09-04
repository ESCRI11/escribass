# Delivery plan

Status as of 2026-09-03. This file tracks **state**: what is done, what is next, and what
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
| M0.2 | `core/`: canonical writer, non-finite rejection | done | `1487a55` |
| M0.2 | `core/`: validator | done | `709ec5b` |
| M0.2 | `core/`: injectable id source and clock | done | `aa789b8` |
| M0.2 | ADR 0004: `song.json` is a derived cache | done | `2840e81` |
| M0.2 | `core/`: RFC 6902 apply, diff | done | `e51a3d9`, `087fdff` |
| M0.2 | `core/`: on-disk history shape, the patch DAG | done | `b079d5f`, `282d0b9` |
| M0.2 | `core/`: `.escri` project store | done | `d90c8bf` |
| M0.2 | `core/`: `create` and `commit` | done | `d18bb90` |
| M0.2 | Four defects found reviewing the stack, fixed at their own PRs | done | `1d97f99`, `0eb6b99`, `4e863ed`, `92d6c74` |
| M0.3 | ADR 0005 (`version` + undo), ADR 0006 (wire shape), dependency pins | done | PR #12 |
| M0.3 | `proto/SongTools` gRPC with `dry_run`, and the same tools over MCP | done | PRs #13–#23 |
| M0.3 | Ten findings from a whole-stack review, three of them blockers | done | PR #25 |
| M0.3 | Six more from an independent final review, two of them blockers | done | PR #26 |
| M0.3 | Landed on `main` as one integration PR | done | PR #27 |
| M0.4 | Determinism suite in `tests/` | **next** | — |

C++ codegen waits for M1 (`CLAUDE.md`, M0 step 1).

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

**Closed 2026-09-03.** A review of the six-PR stack found four defects; each was fixed on the
PR that introduced it rather than at the top, so no PR merged a known one. The one that
mattered: `commit` recorded the caller's ops but stored the round-tripped song, so a legal
alternative spelling (`"64"` for an `int32`) left the log replaying to a document `song.json`
did not match, and the next `open` refused a project that had committed cleanly. Entries now
carry `diff(before, after)`, re-derived from the same `Song` the file is written from.

## M0.3 — tool API

`proto/SongTools` (a new top-level directory, already allowed by §13; add `- path: proto` to
`buf.yaml`). Every tool takes `dry_run`. Beyond §5's list: `create_branch`, `switch_branch`,
`delete_branch`, `merge_branch` (ADR 0001 §4 — auto-merge disjoint paths, structured error on
conflict), and `apply_patch`, which M0.4 needs in order to drive core through the tool API at
all (`CLAUDE.md` #2). The same tools are exposed over MCP in the same step; §18.2 calls that a
hard requirement, not a nice-to-have.

Shape fixed by ADR 0005 (the bump sits between apply and re-deserialisation; undo appends an
inverse entry) and ADR 0006 (one shared `ToolResult`, `Violation` as the only wire error,
`dry_run` as the pure first half of the apply path, one project per process). Twelve PRs, plus a thirteenth for what a review of the whole stack found:

| # | Branch | Adds |
|---|---|---|
| 1 | `m0.3-adrs` | ADR 0005, ADR 0006, dependency pins, §5/§15/§17 rows. No code |
| 2 | `m0.3-proto` | `proto/song_tools.proto`, the buf module, prost codegen |
| 3 | `m0.3-version-bump` | `bump_versions`; `commit` split into pure `prepare` and `record` |
| 4 | `m0.3-session` | `Session`, `dry_run`, `apply_patch`, `get_song`, `get_song_at`, `get_history` |
| 5a | `m0.3-mcp` | Tool schemas derived from the protobuf descriptor |
| 5b | `m0.3-mcp-server` | The `rmcp` stdio server and `escribass-mcp` |
| 6 | `m0.3-tools-devices` | `add_track`, `set_track_instrument`, `add_effect`, `set_param` |
| 7 | `m0.3-tools-clips` | `add_clip`, `set_notes`, `transpose`, `quantize`, `add_automation`, `set_tempo`, sections |
| 8 | `m0.3-branches` | `create_branch`, `switch_branch`, `delete_branch` |
| 9 | `m0.3-merge` | Merge base, three-way by path, conflicts as `errors[]` |
| 10 | `m0.3-grpc` | `tonic` service impl and the `escribass-grpc` binary |
| 11 | `m0.3-dry-run-ids` | A dry run mints from a fork, so a preview burns no ids |
| 12 | `m0.3-review-fixes` | Ten findings from a whole-stack review, three of them blockers |
| 13 | `m0.3-final-fixes` | Six more from an independent final review, two of them blockers |

Out of M0.3, per ADR 0003: `set_form` (needs `FormRule`, M4), the four `compile_*`/`define_*`
tools (M4), `render_preview`/`render_export` (M1), interactive conflict resolution (M2), and
TypeScript/Python codegen for `proto/` (M2, M3 — nothing consumes it before then).

Tool semantics not pinned by §5, decided here: `set_notes` replaces a clip's whole note set;
`transpose` refuses an out-of-range result rather than clamping it; `quantize` snaps
`start_tick` with integer arithmetic and a fixed tie rule.

## M0.4 — determinism suite

In `tests/`. Drives `core` through the tool API, never the file (`CLAUDE.md` #2). Same input
→ identical canonical JSON and identical patch log, byte for byte.

Planned 2026-09-03 against the M0.3 tip. What follows is the plan in enough detail that it
does not have to be rediscovered; the reasoning is the expensive part, not the code.

### The claim, and why running it twice is not enough

Two runs in one CI job catch **nondeterminism** — a process-random `HashMap` seed, a wall
clock, entropy. They cannot catch **drift**: a dependency that changes a serialisation detail,
or Cargo feature unification that flips `serde_json::Map` to insertion order, produces the
*same wrong bytes* in both runs, and they agree. Only a **committed golden** catches that.

That is not hypothetical. `rmcp` depends on `indexmap` directly; if any crate in the tree ever
enables `serde_json/preserve_order`, `Map` becomes insertion-ordered workspace-wide — `diff`
op order changes, `bump_versions` walk order changes, and every `add` value in the log changes
key order. Invisible to run-A-vs-run-B. Loud against a golden.

So the suite proves the claim three ways: **two processes against each other**, **each against
a committed golden**, and **MCP against gRPC**.

### Decisions taken

| Decision | Chosen | Why |
|---|---|---|
| Where the suite lives | `tests/` as a Cargo package | `CLAUDE.md`'s M0 step 4 names it, and M1's golden renders land there too |
| What it drives | The `escribass-mcp` binary as a subprocess, and from PR 4 `escribass-grpc` too | §18.2 makes MCP the surface agents use; the strongest reading of CLAUDE.md #2 is a process, and the library path cannot catch flag parsing, transport serialisation or stdout hygiene |
| Cross-language | TS and Python replay the golden log with a hand-rolled pointer apply | Demonstrates ADR 0002 §11's "any off-the-shelf patch library can apply the file" in all three languages. No new dependency |
| Platform matrix | Deferred to M1 | §8 scopes bit-exactness per platform and M1 has audio to compare. Goldens already catch platform differences opportunistically — a macOS developer compares against Linux-produced goldens for free. Choosing runners also touches the `[OPEN]` minimum-OS-versions item |
| M0 close | M0.4 closes M0, and §11 gains "the determinism suite in `tests/` passes" | A check of the existing requirement, not a new one — but it edits a `[MUST]` section, so it lands in the final PR |
| ADR needed | **None** | No schema change, no new directory (`tests/` is in §13), no dependency, no pin change |

### What is compared, and how

| Artefact | A vs B | vs golden | MCP vs gRPC |
|---|---|---|---|
| `song.json`, `refs.json`, `lock.json`, `patches/*.json` | bytes | bytes | bytes |
| MCP `get_song` text block | bytes | — | vs `to_canonical_json` of the gRPC `Song` |
| `ToolResult` per step | structural | structural | structural |
| `get_history` | structural | structural | structural |
| `initialize`, `tools/list` frames | not compared | not compared | — |

Bytes where the artefact *is* bytes; structural where the encoding legitimately differs per
transport. The handshake carries `CARGO_PKG_VERSION` and `tools/list` changes with every proto
comment — pinning either in a golden would make each version bump a determinism failure, and
both are already covered by `core/tests/descriptor.rs` and `core/tests/mcp.rs`.

### Scripts and layout

```text
tests/
  Cargo.toml            package escribass-tests
  determinism.rs        driver, comparison, report, tests
  determinism/
    every_tool/script.json  expected/{song,refs,lock}.json patches/ responses.json
    refusals/…
    branches/…
  fixtures/             unchanged
```

A script is a JSON array of `{tool, args}` steps, with an optional `"refused": "<rule>"` that
makes the step self-checking — a step that fails unexpectedly fails **at the step**, naming the
tool, rather than surfacing later as a 40 KB golden mismatch. Ids are hard-coded because under
`--seed-ids` they are a pure function of the script prefix; a change in mint order changes the
goldens loudly, which is the point. Not generated from a seed: a fuzzer finds more and explains
nothing.

Three scripts, one claim each: `every_tool` (the whole surface is reproducible and previews
burn nothing), `refusals` (a refusal leaves no trace in ids, log or document), `branches`
(navigation and merge are reproducible, and a refused merge writes nothing).

### The comparison must be able to fail

A comparison that cannot fail proves nothing. Two guards:

- **Clock variant** — run `every_tool` at `T` and `T+1000`; the diff must be non-empty *and
  every differing path must end in `/created_at`*. A wall-clock-leak detector by exclusion, and
  simultaneously the proof that the comparison detects anything at all.
- **Map-order guard** — `json!({"b":1,"a":2}).to_string()` is `{"a":2,"b":1}`, naming the
  `preserve_order` hazard by intent rather than leaving the golden to fail mysteriously.

Failures report `escribass_core::diff` ops between the parsed documents, so a reviewer sees
paths rather than two 40 KB blobs. Arrays are re-keyed by index first, because `diff` replaces
an array whole.

### PRs

| # | Branch | Adds |
|---|---|---|
| 1 | `m0.4-harness` | The `tests/` package, MCP subprocess driver, script format, `every_tool`, A-vs-B comparison, reopen-through-a-fresh-process check, the report, the clock-variant self-test. Deletes the superseded `core/tests/mcp.rs` determinism test |
| 2 | `m0.4-golden` | `expected/` for `every_tool`, `UPDATE_FIXTURES=1` writer, golden comparison, the map-order guard, `.gitattributes` |
| 3 | `m0.4-scripts` | `refusals` and `branches` with goldens |
| 4 | `m0.4-grpc` | The gRPC subprocess driver; every script over both transports |
| 5 | `m0.4-close` | `docs/plan.md`, `tests/AGENTS.md` rules, the §11 line, M0 closed |
| 6 | `m0.4-cross-language` | TS and Python replay the golden log |

The split follows M0.2's and M0.3's lesson: PR 1 is the loud concern (does the plumbing produce
identical bytes twice), PR 2 the silent one (does today's output equal what was committed), PR
4 a second silent class (two transports drifting apart). Mixing them gets the silent half
reviewed as plumbing.

### Traps, found while planning

- **`CARGO_BIN_EXE_<name>` is only set for the package that owns the binary**, so a `tests/`
  package cannot use it. Locate via `current_exe()` → `deps/` → `target/<profile>/`. Do not
  nest `cargo build` inside a test: cargo holds the build lock while tests run. The harness
  must error clearly when the binary is missing rather than hanging.
- **The two binaries default `--author` differently** — `escribass-grpc` to `human`,
  `escribass-mcp` to `model`. Every cross-transport golden differs in `provenance.author`
  unless the harness passes it explicitly.
- **`escribass-grpc --listen 127.0.0.1:0` is unusable**: it prints the address it was asked
  for, not the one it bound. Pick the port by bind-and-drop, as `core/tests/grpc.rs` does.
  Fixing the binary would need `tokio-stream` as a direct dependency (CLAUDE.md #4).
- **`get_song` has two shapes over MCP** — the text block is canonical, `structuredContent` is
  an alphabetised `Value`. Byte-compare the text, golden the structured, never one against the
  other.
- **Float and timestamp formatting belong to dependencies** (`serde_json`'s float writer,
  `pbjson`'s `+00:00`). Either changing in an upgrade is invisible to A-vs-B and caught only by
  the golden.
- **`assets/` is an empty directory** and git cannot store one; compare files only.
- **Goldens are LF** — a Windows checkout with `autocrlf` rewrites them, hence `.gitattributes`.
- **`UPDATE_FIXTURES=1` blesses whatever ran**, including a deterministically wrong output. The
  only guard is the rule that a golden changes solely in the PR that changes the canonical form
  or a tool's semantics, with its diff reviewed there — §17's rule for renders, applied here.
- **A hang is not a failure** unless one is imposed: every gRPC call is wrapped in a 30-second
  timeout and the CI job carries `timeout-minutes`, because a hung test otherwise inherits
  GitHub's six-hour default and reports nothing.
- **`cargo test -p escribass-tests` does not rebuild the binaries** — only the libraries they
  link. The suite drives the binary, so it can validate a build from *before* your change and
  pass. It refuses to run against one older than `core/src`. This was found by two deliberate
  mutations that both "passed" until the binary was rebuilt by hand.

### Kept rather than replaced

The library-level determinism tests in `core/tests/` stay as layer guards — they fail nearer
the cause and cost nothing. Only `core/tests/mcp.rs`'s two-session test moves, because the
suite is its exact superset.

**Everything else in that file stays**, and one of them matters: the regression test for
`apply_patch` reading `"dry_run": "true"` as false and applying a request meant as a preview.
The suite structurally cannot replace it — a script step is a tool call whose arguments are
valid, and a malformed argument is a *protocol* error the harness treats as a broken script
rather than an outcome to record. It was deleted by accident once; the review caught it.

`tests/fixtures/song/minimal.json` stays a schema fixture written from generated types: it
exercises `Generator`, `Marker`, `Instrument.state` and model provenance that no typed tool can
produce before M4, and it is a constructed value rather than a mutation, so CLAUDE.md #2 is not
in play. `tests/AGENTS.md`'s "from M0.4, fixtures come through the tool API" becomes
"determinism goldens come through the tool API; the schema fixture is written from generated
types".

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
| Dense unique `index` on tracks and effects | Inserting mid-list renumbers everything, and two branches inserting at one index auto-merge into an invalid document. Deferred again at M0.3: the merge pipeline makes that failure loud (the validator refuses it) rather than silent, and closing it properly is a `song.proto` change with its own ADR | M2, with the mixer | review, 2026-09-03 |
| Interactive merge conflict resolution | Designing the API with no UI and no real conflicts | M2 | ADR 0001 §4 |
| Garbage collection of orphaned patch entries | Entries are small and inert | only if a real project makes it a problem | ADR 0001 Deferred |
| `SourceRef.export_hash`, `Generator` compiled-source hash | Needed for "export pending" and "compiled · stale"; nothing produces either yet | M4 | ADR 0002 Consequences |
| Strudel as a second `Generator.kind` | Python DSL is the v1 target | after M4 | §15 |
| `schema/pyproject.toml` `[build-system]` | Consumers use `sys.path`; no wheel needed yet | when `ai/` depends on it | `schema/AGENTS.md` |
| Native CLAP hosting | VST3 via clap-wrapper is the mature path | never a dependency | §8 |
| Recursive merge, for a criss-cross base | Two branches that each merge a third leave `merge_base` with no single answer, and it refuses rather than guessing which history is the truth. The fix is to merge the bases and use the result — the same shape as the interactive resolution already deferred there | M2 | review, 2026-09-03 |
| Undo/redo **tools** | ADR 0005 §4 settles the mechanism — an inverse entry, never a rewind. The tools themselves have no consumer until ⌘Z exists | M2 | ADR 0005 §4 |
| `lock.json` beyond `schema_version` | Nothing to pin until compiled artefacts and models exist | M1, M4 | ADR 0003 §3; §17 |

## Known gaps

- **`Project::write` rewrites every entry file on every commit** — O(history) I/O per call.
  Invisible while histories are short; the tool API is what will make it visible.
- **No lock file on an `.escri` directory.** ADR 0001 §2 assumes a single writer and ADR 0004's
  commit is three renames; two processes on one project would race them. M0.3 makes it
  structural (one project per process, ADR 0006 §5) rather than enforced. Revisit at M2, when
  `app` supervises the processes.

## Open — not ours to decide

`docs/specs.md` §15 marks these `[OPEN]`; `CLAUDE.md` says stop and ask. None blocks M0.

- Neural runtime packaging: ONNX Runtime linked into `engine`, or a separate process. Now due
  before M4, which is where the neural runtime lands (ADR 0003 §7).
- Minimum supported OS versions.
- Symbolic model choice for v1 melody and drum generation.
- Whether §6's analysis features (key, chord and structure detection, tempo estimation, stem
  separation) and symbolic generation are v1 scope at all (ADR 0003, Still unplaced).
