# Delivery plan

Status as of 2026-09-17. This file tracks **state**: what is done, what is next, and what
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
| M0.4 | Determinism suite in `tests/` | done | PRs #29–#36 |
| — | **M0 complete.** Schema, core, tool API, determinism suite | done | — |
| M1.1–1.7 | The M1 plan, ADRs 0007–0011, `AudioClip`, `render.proto`, `compile`, the engine, its three plugins, VST3 hosting | done | PRs #38–#45 |
| M1.8, 8b | Asset playback with gain, fades and Rubber Band stretch; the SFZ sampler | done | PRs #46, #47 |
| M1.9 | `lock.json` v2, `lock_mismatch`, `plugin_unknown`, `param_unknown` | done | PR #48 |
| M1.10 | `render_export` over both transports | done | PR #49 |
| M1.11 | Four golden renders, and the two defects blessing them found | done | PR #50 |
| M1.12 | The bar-17 demo as a test, and §18.2's claim narrowed to what it can carry | done | PR #51 |
| M1.13 | A four-lane review of M1: seven blockers, eleven majors | done | PR #52 |
| M1.14 | §11 walked line by line against the code; docs closed; `CLAUDE.md` to M2 | done | PR #53 |
| — | **M1 complete.** Render engine, four goldens, `lock.json` v2 | done | — |
| — | The M2 plan: twelve questions, none answered | done | PR #54 |
| M2.1 | The twelve answered: ADRs 0012–0016, the §15 rows, §17's gRPC and frontend pins | done | PR #55 |
| M2.2 | `app/`: the Tauri host, the frontend, the shared `call` dispatch, `.escri/lock` | done | PR #56 |
| M2.3 | `render.proto`'s whole M2 shape — `Preview` and `mix_lanes` — and TypeScript for `proto/` | done | PR #57 |
| M2.4 | The piano roll, the arrangement completed, and ADR 0012 §5's projection golden | done | PR #58 |
| M2.5 | The first control that is a tool call, its diff, `undo`/`redo`, and ADR 0017 | done | PR #59 |
| M2.6 | `ParamRef` reaching a fader: the validator's two domains, `mix_lanes` compiled, the engine driving Tracktion's fader with our formula | done | PR #60 |
| M2.7 | The mixer and §9's seventh view, the generic parameter editor over the build manifest | done | PR #61 |
| M2.8 | The patch log with its provenance column, branch switching, and `merge_branch`'s per-path resolution | done | PR #62 |
| M2.9 | `Render` over gRPC, the stdio path deleted, and the four goldens unmoved | done | PR #63 |
| M2.10 | Preview: the live process, the device, the `Preview` stream, `render_preview`, and the window's transport — ADR 0013 §4's thread shape measured first | done | PR #64 |
| M2.11 | A whole-stack review of M2: a fresh session's ⌘Z going forwards, answers lost after stdin closes, two CI holes, and the export crash reproduced and refused | done | PR #65 |
| M2.12 | §11 walked against the code, the ledger walked a fourth time, `CLAUDE.md` to M3 — and what M2 leaves unverified written down where a reader will find it | done | this PR |
| — | **M2 complete.** Tauri app, five views as projections, preview playback over gRPC — with three merges and no green run on `main` since PR 8, see "M2, closed" | done | — |
| — | The M3 plan: five questions for the user, fourteen an agent can propose answers to, none answered | done | PR #67 |
| — | The user's four answers, and the default model measured before it was recorded | done | PRs #68, #70 |
| M3.0 | The spike: five model routes on the tool API over MCP and OpenRouter, cost per edit, what `betterproto2` generates for a service, Surge XT's RNG paths counted — its code unmerged, its numbers in "What the spike found" | done | PR #69 |
| M3.1 | ADR 0018 alone: the bar view, read-only, in `ai`, six axes as counts, what was read | done | PR #71 |
| M3.2 | The twelve remaining questions answered: ADRs 0019–0022, their §15 rows, §3/§5/§6/§9/§10/§13/§17/§18.2 amended, `lock.baseline.json`'s `ai` block, the two defects the spike found decided, the ledger rows ADR 0018 assigned here | done | PR #72 |
| M3.3 | `proto/assistant.proto` — one service, one bidirectional stream, ADR 0020 §3's messages, no provenance and no `list_params` — and Python for `proto/`, a server and no client, with the model's re-emitted packages deleted and their imports sent to `escribass_schema`; `grpclib` 0.4.9 pinned here at the user's decision, so the check that it all imports and that there is one `Song` lands with the code | done | PR #73 |
| M3.4 | `prepare` decides every entity's `provenance` on every path and `prepare_merge` leaves it, which closes the forgery the spike found; `ProjectLock::take` replaces a lock whose recorded pid names no process and says so, which closes the lock every signalled MCP client left behind. No golden moved | done | this PR |
| M3.5 | `ai/`: the package, its lock and its exact Python, `grpclib`/`openai`/`httpx2` pinned, the generated `Assistant` served over a socket it names, the scripted provider and one recorded transcript, and a process `core` spawns and the window watches. `ai/`'s tests joined the `checks` job here | done | PR #75 |
| M3.6 | The Libretto view and the six axes, pure, goldened against the render fixture and its key-reversed twin — a projection golden in a third language — with constructed cases for what that fixture cannot exercise. ADR 0018 §1's hand-written view corrected in three lines; its §4 figures recomputed and all held | done | this PR |

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
tools (M4), `render_export` (M1), interactive conflict resolution (M2), and TypeScript/Python
codegen for `proto/` (M2, M3 — nothing consumes it before then). ~~`render_preview` (M1)~~ —
**corrected 2026-09-07**: it was written here as M1's beside `render_export` and shipped in
neither, while `proto/song_tools.proto` said it waits for M2's live engine process. It is M2's,
and ADR 0013 §2 is what a preview turned out to be.

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
| MCP `get_song` text block | in the reopen check, against `song.json` | — | via `song.json`, which is byte-compared |
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

- **Clock variant** — run `every_tool` a minute apart; the diff must be non-empty *and
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
| 5 | `m0.4-cross-language` | TS and Python replay the golden log |
| 6 | `m0.4-review-fixes` | One blocker and four should-fixes from a whole-stack review |
| 7 | `m0.4-close` | `docs/plan.md`, the §11 line, M0 closed |

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

## M0, closed

Four steps, thirty-odd pull requests, five whole-stack reviews. What M0 delivers: one
representation of a song (`schema/song.proto`), a `core` that validates it and records every
change as a patch in a DAG, a tool API those changes must go through — served over gRPC and
MCP — and a suite that proves the same input gives the same bytes.

The reviews earned their place. Between them they found a merge that made a project
unopenable, an unknown enum that killed the gRPC server permanently, an approve-then-apply
flow with no working path, a merge that repeated a version number, a preview that wrote for
real, and a determinism suite that could validate a stale binary and pass. Every one of those
looked correct in review and was wrong in a way only a test or a mutation could show.

## M1 — render engine

Planned 2026-09-04 against `main` at `e2dc08e`. As with M0.4, the reasoning is the expensive
part and none of it is in code yet.

### Decisions taken, 2026-09-04

Eight questions the plan raised, answered before code:

| Question | Decided | Consequence |
|---|---|---|
| Airwindows, whose pinned repo may not build a Linux VST3 | **Defer to M4**, with clap-wrapper | Amends ADR 0003 §4. §11's "a golden per bundled instrument" is met by the three synths; Airwindows is an effect |
| Audio clips in M1 | **Yes, in full**: gain, fades *and* time-stretch | Triggers exactly the clause ADR 0002 §8 wrote for it — "when M1 renders audio clips". A `song.proto` change, so an ADR precedes it (CLAUDE.md #5), and Rubber Band is vendored (already pinned at 4.0.0) |
| A project pinning a plugin this build lacks | **Refuse to open**, `lock_mismatch` | The strict reading of §11. Same shape as `schema_version_mismatch`: an operator error, not something a model retries into. Re-pinning becomes explicit, never a side effect |
| Minimum supported OS versions (§15 `[OPEN]`) | **Linux x86-64 only in M1** | ADR 0009 names the image and compiler its goldens are valid for. macOS and Windows stay unclaimed; §15's item stays open |
| Engine transport, given §3 says gRPC | **Define `Render` now, speak stdio in M1, implement gRPC at M2** | ADR 0006 §7's precedent: the `.proto` is the artefact that must be right. `buf breaking` guards it from M1; grpc++ is not vendored for a client that would be itself |
| Render tail | **End at the last clip or section** | No schema change. A `RenderTarget.tail` field waits for someone who wants release tails |
| Hashing for `add_asset` | **`sha2` 0.10** | The alternative — the engine computing it — puts the engine in the project-writing path, contradicting CLAUDE.md #6 |
| Still unplaced | **User VST3 plugins** belong to no milestone (§8 says "VST3 host"; §16 never says user plugins) | ADR 0003's lesson is that unplaced scope is invisible scope. Decide at M2, when `app` could show a plugin browser |

Audio clips are the one that grows M1. `AudioClip` is `{ asset_hash }` today, so rendering one
needs the fields §8 deferred, and time-stretch adds a **second DSP surface** to pin and golden
alongside the three plugins — Rubber Band is a phase vocoder with its own modes and threading,
and it gets a determinism note of its own in ADR 0009, exactly as each plugin does.

### What was already decided, so M1 does not re-decide it

| Decided | Where | Consequence |
|---|---|---|
| The engine receives a materialised layer 2 + layer 3 snapshot, builds a Tracktion edit, never reads project files | §8 | Something compiles the song into that snapshot, and it is not the engine |
| M1 defines the snapshot as its own message under `proto/` and strips provenance | ADR 0002 Consequences | `proto/render.proto`; not a `Song` |
| History metadata — patch ids, refs, `HEAD`, `created_at` — never reaches the engine | ADR 0001 Consequences | The snapshot carries no provenance and no version |
| Automation is limited to `LINEAR` and `HOLD` with defined formulas | ADR 0002 §8 | The engine implements *our* formulas, not Tracktion's curve shapes |
| `lock.json` is `schema_version` only until M1/M4 give it something to pin | ADR 0003 §3; §17 | M1 adds plugins and the engine pin |
| The validator cannot resolve a `DeviceRef` to a pinned plugin, or a `ParamRef` to a real parameter, "until the engine arrives (M1)" | `core/src/validate.rs`; `core/AGENTS.md` | **M1 closes both**, which means M1 produces a plugin manifest |

### The central question: how a schema-agnostic engine renders a song

`CLAUDE.md` #6 says the engine is schema-agnostic. That cannot mean it links nothing from
`song.proto`; it means **the engine never receives a `Song`** — no id-keyed map, no
`Clip.track_id` to resolve, no `solo` to resolve against other tracks, no loop to expand, no
history field.

It receives `RenderPlan` (`proto/render.proto`): flat, ordered, already resolved. Leaf
messages — `Note`, `Mix`, `AutomationPoint`, `TempoEvent`, `RenderTarget`, `DeviceRef` — are
reused **by value** with `provenance` and `version` blanked, because a plan-local `Note` is the
mirrored shape ADR 0006 §4 forbids. Structural messages are plan-local: `repeated`, not maps,
since ADR 0001 §3's rule is about RFC 6902 path stability and a plan is never patched.

Time crosses as **ticks plus tempo events**, and the engine converts. One owner of tick→sample
is one fewer boundary; the fallback — core computing sample positions in integer arithmetic —
is recorded in the ADR if the spike shows Tracktion's conversion is unstable.

`core::render::compile(song, assets) -> Result<RenderPlan, Vec<Violation>>`. Pure. The error
type is `Vec<Violation>` because every failure is caller-fixable — ADR 0006 §2's line by
signature, as `prepare` already does it.

**Not a second representation of song state** (CLAUDE.md #1): the plan is derived, never
persisted, never edited, never read back — the same standing as a WAV. Backed by a test, not
prose: a descriptor-driven guard walks every field of every `song.v1` message and requires each
to be carried into the plan or on an explicit allowlist with a reason. A field added to
`song.proto` that affects sound and never reaches the plan fails that test instead of silently
rendering as nothing.

M1 **refuses** with `render_unsupported`: Cmajor, Faust and neural device refs (M4 — but the
*validator* still accepts them, since validity and renderability are different questions);
`Routing` sends, sidechains and bus outputs (M2's mixer); a plugin not in the bundled manifest.
Audio clips are rendered, not refused (decision above).

### What "deterministic" means for a render

**Claimed:** same `.escri` + same engine binary + same plugin binaries + same OS and CPU
architecture → byte-identical PCM. In M1 that is **Linux x86-64 on the pinned image and
compiler**, which is §8's "bit-exact across runs on the same platform".

**Not claimed:** cross-OS, cross-architecture, or cross-CPU on the same OS — that last only
after the spike measures it, because runtime SIMD dispatch is real (trap 1 below).

The golden is the **WAV, compared byte for byte on the PCM payload**, with a `.sha256` beside
it: the file gives the diff, the hash gives the release note. No tolerance by default; a plugin
that proves non-deterministic gets a documented tolerance *and* a §8 per-plugin note. On
mismatch the report names the first differing sample, the count, and the max absolute
difference — the audio equivalent of M0.4's "paths, not two 40 KB blobs".

Honestly about plugins: Dexed is pure FM and should be exact. sfizz resamples and has runtime
SIMD dispatch. Surge XT has random start phase, unison detune randomisation and noise sources;
a fixture must disable them, and whether its global RNG is seeded from a constant is a spike
question.

### Process shape

`engine/` is a CMake project linking Tracktion Engine, JUCE and protobuf C++, building one
binary. In M1 it is driven as **a fresh subprocess per render** reading one `RenderPlan` on
stdin and writing a `RenderResult` on stdout — no plugin instance reuse, no warm smoothers, no
state between renders, which is the cheapest determinism guarantee available. (M2 PR 9 kept the
process shape and replaced the pipe: the same fresh subprocess now serves one `Render` call over
a Unix socket it names on its own stdout, and grpc++ joined the list of things this project
links. The stdio path was deleted in the same pull request rather than kept beside it.)

The `Render` gRPC service is **defined** in `render.proto` in M1 so `buf breaking` guards it,
and **implemented** at M2 when `app` exists to hold a live connection. That is ADR 0006 §7's
own precedent — the `.proto` is the artefact that must be right — and it avoids vendoring
grpc++ for a client that would be itself.

C++ codegen runs at **build time** via CMake, not committed. Generated `.pb.cc` embeds a
runtime-version check, so committing it pins a `protoc` that must equal the vendored runtime —
two pins that must agree. Build-time generation makes it one pin and makes drift a compile
error in the engine job, which is a stronger gate than a hash. This changes `schema/AGENTS.md`'s
"C++ at M1: one plugin entry in `buf.gen.yaml`" line, written before that coupling was seen.

### What the spike found (PR 0, run 2026-09-04)

Run on Ubuntu 24.04 x86-64, g++ 13.3, Tracktion `0e02f70` — the image ADR 0009 proposes. The
spike renders two seconds of a built-in tone generator to WAV, headless.

**The finding that matters: the render is deterministic and the file is not.** Three separate
processes produced byte-identical **PCM** — 576,000 bytes, `30fafbdc…` every time — and three
different *file* hashes. JUCE's WAV writer emits a `bext` chunk (Broadcast Wave Extension)
carrying `OriginationDate` and `OriginationTime`, and the third run crossed a second boundary:

```
OriginationDate  run1='2026-09-04'   run3='2026-09-04'
OriginationTime  run1='18:43:11'     run3='18:43:12'
```

A golden that hashed the file would have failed roughly once per second of build time, and it
would have arrived as flakiness rather than as a finding. **Compare the `data` chunk, never the
file**, and `RenderResult`'s hash is a hash of the payload. Trap 2 was a guess; it is now
evidence, and ADR 0009 cites it.

**A render is asynchronous.** Tracktion's own test utilities use `EditRenderer::render(params,
callback)` and pump a dispatch loop until it fires. `Renderer::renderToFile` — the
synchronous-looking wrapper — silently produced nothing: right preconditions, no error, empty
file. So the engine binary's `main()` runs a message loop; it is not "call a function and
exit", which is the shape ADR 0008 would otherwise have described.

**`Renderer::Parameters::tracksToDo` documents itself as "if this is empty, all tracks will be
rendered"**, and the implementation requires `countNumberOfSetBits() > 0`, returning an empty
`File` with no error when it is not met. Set the bits explicitly.

**Headless works and needs no display.** `EngineBehaviour::autoInitialiseDeviceManager()`,
`addSystemAudioIODeviceTypes()` and `shouldOpenAudioInputByDefault()` all return `false`, and
the binary rendered with `DISPLAY` and `WAYLAND_DISPLAY` unset. The device manager turned out
not to be the blocker either way — enabling it changed nothing.

X11 **headers** are still required to *build*: JUCE compiles `juceaide` (which links
`juce_gui_basics`) during configure, before any `JUCE_USE_XRANDR=0` of ours applies. The CI apt
list is not optional. `webkit2gtk` and `gtk+` are *not* needed — they are reported missing and
the configure succeeds — so the list is shorter than JUCE's documented desktop set.

Four more, each of which would have cost a CI round trip:

| Found | Consequence |
|---|---|
| Tracktion declares JUCE as `git@github.com:…` — an **SSH URL no keyless runner can clone**. `insteadOf` did *not* take; overriding `submodule.modules/juce.url` did | A required step in the engine job |
| Linux needs **`-latomic`**: `tracktion_engine_playback` references `__atomic_store` for 16-byte atomics GCC does not lower inline | One line in the engine's CMake |
| **`juce::SHA256` lives in `juce_cryptography`**, which Tracktion does not pull in | The engine need not hash at all — core has `sha2` approved, so one hasher in the system rather than two |
| **`sfizz` 1.2.3 builds no VST3** — its CMake produces a library and a JACK client; the plugin is in `sfztools/sfizz-ui` | Trap 11 confirmed: `lock.baseline.json`'s `sfizz` pin yields no plugin, and ADR 0010 adds `sfizz_ui` |

**Verified rather than assumed:** Tracktion `0e02f70` pins JUCE at exactly the commit in
`lock.baseline.json`, so §17's "use the commit Tracktion pins, not JUCE latest" holds today.

**Build cost:** 2 minutes wall, 23 minutes CPU, on 24 cores from cold with no ccache — well
inside the budget §7 assumed.

**Still open, and only CI can answer it:** whether the same binary hashes identically on two
different runner CPUs. That decides whether ADR 0009 claims cross-CPU on one OS, or retreats to
same-machine. Everything else the spike was for is answered.

### ADRs, before code

| ADR | Records |
|---|---|
| 0007 | The engine renders a `RenderPlan` compiled by core, never a `Song` |
| 0008 | The engine is a fresh subprocess per render, over stdio until M2 gives gRPC a consumer |
| 0009 | A render is bit-exact for one pinned toolchain on one platform, and the golden is the WAV |
| 0010 | `lock.json` pins the engine and every referenced plugin, added on first reference, compared at load |
| 0011 | `AudioClip` gains gain, fades and stretch, and M1 renders it |

Five rather than one, because each answers a different reviewer question — what crosses, how it
runs, what "same" means, what is pinned, and what an audio clip is — and the M0.2–M0.4 lesson
is that a PR mixing concerns gets reviewed for the loud one. **0011 is a schema ADR**, so it
precedes the `.proto` change (CLAUDE.md #5, `docs/adr/AGENTS.md`).

### PRs

| # | Branch | Adds |
|---|---|---|
| 0 | `m1.0-spike` (**never merged**) | Headless Tracktion render of one note through Surge on the CI image. Answers the questions the ADRs cannot honestly be written without |
| 1 | `m1.1-adrs` | ADR 0007–0011, spec amendments, resolved pins. No code |
| 2 | `m1.2-audio-clip` | `AudioClip` gains gain, fades and stretch; codegen; the `schema/` fixture and its three round-trip suites. **A schema change, alone** |
| 3 | `m1.3-render-proto` | `render.proto`; `RenderExport` and `AddAsset` on `SongTools` |
| 4 | `m1.4-compile` | `core/src/render.rs`, the field-coverage guard, the plan golden |
| 5 | `m1.5-engine-skeleton` | `engine/` CMake, submodules, the CI job — rendering **silence of the right length** |
| 6 | `m1.6-plugins` | Three plugin submodules, manifests, the bundle cache |
| 7 | `m1.7-host` | VST3 loading, MIDI, tempo, automation, single-threaded fixed-block render |
| 8 | `m1.8-audio` | Asset playback, gain and fades, Rubber Band vendored and pinned for stretch |
| 8b | `m1.8b-sampler` | `Instrument.kind: sampler` — an SFZ from `assets/` loaded into sfizz. §16 puts the sampler in M1 and no PR owned it; §11 wants a golden per bundled instrument, so PR 11's sfizz fixture depends on this. **Done.** The row's premise was half wrong: sfizz with no SFZ is *not* silent — its default patch is `<region>sample=*sine`, so what a fixture would have goldened is a sine and not a sampler. The real hazard is one layer in, and measured: an SFZ whose sample cannot be resolved renders silence and exits zero |
| 9 | `m1.9-lock` | `Lock` v2, `lock_mismatch`, `plugin_unknown`, `param_unknown`. **The silent PR**: the M0.4 goldens regenerate here and nowhere else. **Done.** Every byte that moved has one of three causes — the plugin id, two parameter ids, and `lock.json`'s two new blocks — and the goldens were re-derived from the old ones by that substitution alone to prove it. `param_out_of_range` came along with them, since PR 7 had already established the domain |
| 10 | `m1.10-render-export` | `Session::render_export` over both transports. **Done.** The row said "and `add_asset`", which PR 3 had already delivered on both transports and the determinism suite already scripts — so this PR is `render_export` alone. Two things it settled that the row did not name: the engine binary is **told** (`--engine`), never searched, for the reason the manifest is (ADR 0010 §4); and `RenderExport` answers with `RenderResponse` rather than the shared `ToolResult`, because a render produces no ops and the hash it reports has nowhere else to go (ADR 0006 §1, extended). The determinism suite scripts it as a **dry run**, since the `checks` job builds no engine; the engine half is PR 11's, and `tests/AGENTS.md` says so where a reader will hit it |
| 11 | `m1.11-goldens` | `tests/renders.rs` behind a feature; A-vs-B and golden WAVs, including an audio clip. Also **deletes the `RPC_SAME_RESPONSE_TYPE` exemption in the root `buf.yaml`**: it exists only while `main` still carries `RenderExport`'s old response type, which is what `buf breaking --against` compares to. **Done**, and it found two defects that nothing before it could have. **A render did not replace its output**: Tracktion opens the destination at end-of-file, so a second render to one path appended a whole second RIFF file and every reader — the engine's own read-back included — took the first `data` chunk. Every "renders the same twice" check that reused one path was therefore comparing a render against itself, which is why **Surge XT was believed deterministic at its factory patch and is not**: it needs `A Osc 1 Retrigger` set, which is trap 7 arriving exactly where trap 7 said it would. And **an asset in `assets/` could not be played at all**: JUCE picks a reader by file extension and a content-addressed asset has none, so every audio clip in a real project failed — invisible until a fixture built through the tool API rendered one |
| 12 | `m1.12-locality` | The bar-17 demo as a test. **Done.** It also corrected §18.2's wording: "bytes changed only in bar 17" is false in general — a note's release tail outlives its note-off, so how far an edit reaches is a property of the note's length, not of the platform. What the test asserts exactly is that nothing *before* the edit moves; what comes after is measured and printed |
| 13 | `m1.13-review-fixes` | Whole-stack review findings — M0 averaged four to sixteen per milestone. **Done**, and the number was eighteen: seven blockers and eleven majors from four independent lanes (determinism, the tool-API boundary, the C++ engine, ADR-versus-code correspondence). The lesson is one sentence: **every one of them was behind a passing check**, and six of them exit 0 — an engine that renders nothing, a leftover scratch file that changes the audio, a NaN that renders silence, a killed render that destroys the last good one, a stdout that could not be written, and a golden suite that never checked the engine binary against `engine/src`. One reported finding did not survive measurement and is recorded as a correction in the other direction (ADR 0009 §4, the two-sample offset) |
| 14 | `m1.14-close` | Docs, the §11 line checked, `CLAUDE.md` to M2. **Done.** §11's five bullets were walked against the code rather than from memory and all five hold; what enforces each is named in the pull request, and §11 already names most of them in its own text because PR 13 rewrote it to describe what the code does. Nothing was added, because nothing was missing |

PR 0 is a spike that is thrown away: the ADRs cannot be written honestly without knowing
Tracktion's API for device-less construction, whether JUCE needs X11 to instantiate a VST3
headlessly, and whether the same binary hashes identically on two runner CPUs.

The render suite is a **cargo feature** on `escribass-tests`, so `cargo test` from the root
compiles it away. A `#[test]` that returned early would be the quiet skip M0.4 exists to
prevent; a feature is absent where it cannot run and loud where it must.

### Traps

1. **Two runs agree, the golden differs, and the cause is the CPU.** sfizz and JUCE dispatch
   SIMD at runtime; AVX2 on one runner and SSE4 on another round differently. Reads as
   flakiness. Pin the ISA; bless a golden only after the spike hashes identically on two CPUs.
   **PR 11 runs the experiment** (ADR 0009 §6): the `renders` job renders the four goldens on
   three `ubuntu-24.04` runners with the binary the `engine` job built and uploaded, comparing
   against the same committed bytes with no tolerance, and `cross-cpu` reports which CPU models
   actually turned up. One model across the matrix is reported as **inconclusive** rather than
   as a pass, because the runner pool is not ours to choose. **First run, 2026-09-07:** three
   runners, one CPU model (`AMD EPYC 7763`), all four goldens reproduced — inconclusive, and
   reported in those words. Separately, the goldens were blessed on an `AMD Ryzen AI 9 HX PRO
   370` and reproduce on the EPYC byte for byte, which is two CPU models and two builds; ADR
   0009 §6 records why that is evidence and not the answer, and the trap stays open.
2. ~~**The WAV header carries a date or a software tag**~~ — **confirmed by the spike**: JUCE
   emits a `bext` chunk with `OriginationDate` and `OriginationTime`. Compare the `data` chunk.
3. **Denormals** — without FTZ/DAZ a filter tail is 100× slower and its bits depend on a
   per-thread flag.
4. **Summation order under a thread pool** — float addition is not associative. Single-threaded.
5. **`-ffp-contract`** — both compilers fuse `a*b+c` on a capable `-march`; a vendored
   `CMakeLists` adding `-ffast-math` changes bits.
6. ~~**`setState` then `setParam`, in a fresh process**~~ — **settled in PR 7, and it had a
   second half the trap did not name.** The order is state first, parameters second, and it is
   proved rather than asserted: a Dexed state captured with `Cutoff` at 0.05 renders one hash
   with no parameters, and the same state plus `params { Cutoff: 1.0 }` renders *exactly* the
   hash of a plan with no state at all. The half that would have been missed is that 1.0 is
   also the value Tracktion cached when the plugin was created, and
   `AutomatableParameter::setParameterValue` returns without writing when the cached value
   already equals the one being set — so a parameter routed through Tracktion would have been
   dropped precisely when the state disagreed with it. The engine writes the plugin's own
   parameter instead. The trap's other half, smoothing from a reused instance, is ADR 0008 §2's
   fresh process and needs nothing here.
7. **Randomness inside the fixture** — Surge start phase and unison detune, sfizz `*_random`.
   **Settled in PR 11, and the trap was right where PR 7 thought it was not.** Surge XT's
   factory patch does reach the wall-clock RNG — five fresh renders, five hashes — and PR 7's
   three agreeing hashes were one render read three times, through the output file it had not
   replaced. `A Osc 1 Retrigger` (`1217754326`) at 1.0 is the whole fix, and four fresh
   processes then agree exactly. sfizz's `*_random` opcodes stay out of the fixture, and the
   SFZ names its sample by the asset hash (PR 8b) so nothing walks a directory.
8. **The suite validates a stale engine** — M0.4's exact defect, one language over. The engine
   embeds the submodule commits it was built from and the suite compares them. **Closed in PR
   11**: `tests/renders.rs` compares `RenderResult.commits` against `lock.baseline.json` before
   it reads a sample, and a mismatch fails naming the component and saying not to bless a golden
   against that build. Proved by a doctored map in a test that needs no engine, and by moving a
   pin in `lock.baseline.json` and watching a real render refuse.
9. ~~**JUCE `add_subdirectory` twice**~~ — **confirmed and handled in PR 6**: Surge vendors
   `surge-synthesizer/JUCE`, Dexed vendors `juce-framework/JUCE` at another commit, Tracktion a
   third. Each plugin is an `ExternalProject` with its own configure and its own target
   namespace (`engine/cmake/plugins.cmake`), so the collision cannot occur rather than being
   managed. The determinism flags cross as `CMAKE_{C,CXX}_FLAGS`, since `add_compile_options`
   does not.
10. ~~**Headless JUCE may need a display to instantiate a VST3.**~~ — **answered in PR 6: it
    does not.** `escribass_engine --scan` opens all three bundled VST3s through
    `juce::VST3PluginFormat`, instantiates each with `AudioPluginFormatManager` and reads its
    parameters, with `DISPLAY` and `WAYLAND_DISPLAY` unset — including sfizz's, whose plugin
    links VSTGUI. A CI runner has no display at all, so the engine job is the standing check.
    Two things the trap did not name and PR 7 will meet: X11, xcb, cairo and pango **headers**
    are needed to *build* (the plugin links them even where nothing draws), and sfizz writes
    `[sfizz] new synth` when it is constructed — to **stderr**, so ADR 0008 §1's "stdout
    carries protobuf bytes and nothing else" survives a hosted plugin, but only just.
11. ~~**sfizz's VST3 lives in `sfizz-ui`**~~ — **confirmed by the spike**: `sfizz` 1.2.3's CMake
    builds a library and a JACK client, no VST3.
12. **`ubuntu-latest` moves** — an image update changes the compiler and every golden drifts
    with no PR to blame. **Held in PR 11**: the `renders` job names `ubuntu-24.04` like the
    `engine` job it takes its binary from, so the goldens are only ever compared on the image
    §17 pins. `cross-cpu` does nothing but read text files and may sit on `ubuntu-latest`.
13. ~~**`every_tool`'s plugin id is invented.**~~ — **settled in PR 9**, and the trap was
    right about all of it. `com.surge-synth.surge-xt` and `org.surge-synth.surge-xt` are now
    `Surge Synth Team/Surge XT` across 23 files, and `cutoff` and `drive` are `1945359057` and
    `1243907205`, which the manifest fixture carries with their display names beside them so a
    reader can tell what they are. The half the trap did not name is where the *tests* get a
    manifest, since the real one is never committed: they read a committed subset of a real
    `--scan`, and the engine CI job asserts it is still a subset of what the build declares.
    The original text follows. Once the validator resolves plugin ids,
    `com.surge-synth.surge-xt` must be a real manifest id or `checks` goes red. **PR 6 learned
    the real ones**, and they are what PR 9 must write into `tests/determinism/*/script.json`
    and `tests/fixtures/song/minimal.json`: `Surge Synth Team/Surge XT`, `SFZTools/sfizz`,
    `SFZTools/sfizz-multi` and `Digital Suburban/Dexed` — the vendor and the class name as the
    VST3 factory reports them, which is all a VST3 offers that is not a path hash or a 32-bit
    number. Three files, four classes: sfizz-ui's bundle declares two. A parameter is worse
    off: the manifest's `params` maps the plugin's own parameter id to its display name,
    because the names are not unique (Surge XT repeats 176 of 2855, one per unassigned effect
    slot) and the ids are opaque integers. `ParamRef.param` matching the **key** is what PR 9
    has to settle.
14. **`Instrument.state` must not join `JSON_TEXT_FIELDS`** — it is opaque binary, and the
    comment there already names it as the counter-example.
15. ~~**Rubber Band is a second DSP surface.**~~ — **settled in PR 8, and the trap named the
    wrong half.** The options are pinned in full by ADR 0011 §3 and the determinism note is in
    ADR 0009 §4, as the trap asked. But the threading it warned about is inert: the option is
    read only by the R2 engine, which the pinned word does not select, and the vendored build
    compiles threading out entirely. What the trap missed is that the **build** picks the FFT —
    the library's own build system takes FFTW, IPP, KissFFT or vDSP from whatever is installed
    on the machine, and a phase vocoder over two FFTs is two different signals with every
    option identical. The engine builds upstream's `single/RubberBandSingle.cpp`, which
    hard-defines the built-in FFT and resampler. There is no runtime CPU dispatch anywhere in
    the library, so unlike sfizz it is not a candidate for trap 1. A stretched clip hashed
    identically in three fresh processes.
16. **An audio asset makes `assets/` non-empty for the first time.** `Project::write` creates
    the directory and M0's comparison ignores it because git cannot store an empty one; a
    golden that now contains an asset changes what the determinism suite compares. **Settled
    where it landed**: the determinism suite's `every_tool` golden already carries assets, and
    PR 11's render fixtures carry theirs as base64 inside `script.json` rather than as files,
    because what a render golden commits is the WAV. The trap's real sting turned out to be one
    layer down and is trap 7's neighbour: an asset is named by its own hash and so has **no
    extension**, which is what stopped the engine reading one at all until PR 11.

### Deferred again, with reasons

`Instrument.state` as a content hash: nothing in M1 *writes* a state — fixtures use factory
defaults plus `params` — so an ADR now would design against no producer, which is ADR 0002 §7's
reason. Revisit at M2 with the first plugin editor.

Render tail: `length_ticks` ends at the last clip or section. A `RenderTarget.tail` field is a
`song.proto` change with its own ADR when someone wants release tails.

## M1, closed

Fourteen pull requests, five ADRs, one four-lane review. What M1 delivers: a `RenderPlan`
compiled by `core` from the song and rendered by a C++ engine that never sees a `Song` — a
fresh process per render, hosting Surge XT, sfizz and Dexed as VST3 and playing audio clips at
their gain, fades and Rubber Band stretch — four golden WAVs that reproduce byte for byte, and
a `lock.json` recording the build each of them was made with. §11's checklist was walked bullet
by bullet against the code at the close, and all five hold.

M0's lesson held and sharpened. The review returned eighteen defects and what they had in
common was that none of them was a red test: six exited 0. An engine that rendered nothing and
reported success. A leftover scratch file that changed the audio. A render that appended a
second RIFF file instead of replacing the first, which made three "renders the same twice"
checks compare one render against itself — and hid the fact that Surge XT at its factory patch
seeds itself from the wall clock. A golden suite that had never once compared the engine binary
against `engine/src`.

What M1 does not claim is as much of the point as what it does. macOS, Windows and any CPU
other than the x86-64 the goldens were blessed on are unclaimed rather than contradicted (ADR
0009 §1). The cross-CPU experiment ran on three runners and drew one CPU model, and PR 13 read
sfizz's dispatcher again and found the AVX switch empty at this pin — so trap 1 stays open on
the mechanism and not merely on the sample (ADR 0009 §6).

## M2 — UI

Planned 2026-09-07 against `main` at `831fbc5`; the twelve questions it raised were answered
2026-09-07 against `ddd6982`, in five ADRs and no code. Same shape as M1: the planning PR asks,
a second PR answers, and neither is allowed to be the other, because an ADR written before its
question is settled is what that separation exists to prevent.

### What was already decided, so M2 does not re-decide it

| Decided | Where | Consequence |
|---|---|---|
| `app` is a Tauri host in Rust with a web frontend, and the host **embeds `core`** and supervises `ai` and `engine` | §3, tier 1 and tier 2 rows; §9 | `core` is a library dependency of `app`, not a server it dials |
| React, with the timeline drawn on canvas/WebGL, and CodeMirror 6 | §15 (Frontend); §3 | Not M2's choice to make again. The timeline is custom rendering under any framework, which is what §15's rationale already says. What was never decided was the *pin*, and ADR 0016 takes it |
| `app/` is in §13 | §13; `AGENTS.md` | No new-directory ADR. The precedent is `engine/`, created in M1 PR 5 under the same line |
| Every view is a projection, and nothing but the model is persisted as authoritative state | §2.1, §14.2, CLAUDE.md #1 | No per-view model and no editable client store. The constraint most easily broken by an ordinary performance fix (trap 1), and now the one ADR 0012 §5's golden checks |
| Every control is a tool call, and the UI uses the same API the AI does | §5, §14.3; wireframes, "Scope check" | A drag is a `set_notes`, and nothing in `app` writes `song.json`. *When* the call is made is trap 2, and it is still open on purpose |
| The mixer and the history view are M2's, not only the timeline and the roll | ADR 0003 §5 | Four of §9's seven views — five, since ADR 0014 §1 added the editors. The AI panel is M3 (§16) and code views are M4 (ADR 0003 §8) |
| `Render` gains its gRPC implementation, and **the stdio path is deleted rather than kept beside it** | ADR 0008 §1; §3 | `core/src/engine.rs`, its EOF framing, `render_export` and the render suite's driver all move together, in the order implement · move · delete (trap 5) |
| Preview audio plays from the engine straight to the device; audio is never streamed over IPC | §15 (Preview audio); §3 | The engine opens an audio device for the first time. "Headless works and needs no display" was measured for instantiating a VST3, not for a device (trap 13) |
| TypeScript codegen for `proto/` lands at M2 | ADR 0006 §7 | The service, not the model — `schema/` has generated TypeScript since M0.1, which is why PR 2's window needs no `proto/` types to draw a `Song`. Python waits for M3 |
| Undo and redo become **tools**, because ⌘Z is their first consumer | ADR 0005 §4; deferred ledger | Undo appends an inverse entry and never rewinds a ref (§5). A frontend undo stack is trap 10 |
| Bit-exactness is claimed for Linux x86-64 on one pinned image and compiler | ADR 0009 §1 | M2 inherits that claim and does not widen it, and ADR 0014 §2 targets the same platform |

### Decisions taken, 2026-09-07

Twelve questions the plan raised, answered before code. Five ADRs, 0012 to 0016, the §15 rows
they force, two new §17 pins, and amendments in place to ADR 0003 §5, ADR 0006 §5 and ADR 0010 §4.

| Question | Decided | Consequence |
|---|---|---|
| Is `app` one process or two, and what does the webview call? | **One process.** One Tauri command carrying a tool name and its arguments, dispatched through the function `core/src/mcp.rs` already dispatches through | The drift objection dissolves rather than being accepted: the UI runs on the **MCP** path, which M0.4 already drives through a real process and compares against gRPC. `core/src/mcp.rs`'s `match` is lifted into `call(session, name, args)` in PR 2 — a `core` change in a UI milestone. `app` opens no network port (ADR 0012 §1) |
| How does the frontend hold what it draws, without being a second representation? | **One decoded `Song`**, re-read after every applied call; every view a pure selector | No local RFC 6902 apply and no normalised store. The escape from a slow re-read is a *narrower read*, written down now so the performance fix in PR 7 is not a design decision nobody made (trap 1). Types are `schema/`'s generated TypeScript; §4.1 forbids any other kind (ADR 0012 §2) |
| Which schema changes does the mixer force, and do they land in M2? | **One of the three, and it is not a schema change at all.** `ParamRef` reaches a fader by naming a track; dense `index` defers; interactive merge resolution lands as one optional field | Ids are already globally unique across collections, so `ParamRef { device_id, param }` addresses `Mix.gain_db` and `Mix.pan` with no `song.proto` field added — no ADR-before-code coupling, no `buf breaking` run, and **the M0.4 goldens do not move at all**. The *plan* goldens do, in PR 3, where `PlanTrack` gains `mix_lanes` (ADR 0015) |
| What *is* preview playback? | **A compiled `RenderPlan` played from a tick**, over one bidirectional-streaming `Preview` RPC; the plan is replaced, never diffed | Decides `render.proto`'s M2 shape, so it lands once and early in PR 3 while `buf breaking` still compares it against an unmoved `main` (trap 12). The stream is the session identifier, so no handle is invented (ADR 0013 §2) |
| What is the engine's lifetime once preview exists? | **A live process for preview, a fresh one per offline render.** One binary, one transport, two modes | ADR 0008 §2 survives whole rather than being reasoned around: an export never shares plugin instances with something that was played before it, and the guarantee stays the operating system's rather than becoming discipline in C++ (trap 6, ADR 0013 §3) |
| Does gRPC in the engine mean vendoring grpc++? | **Yes, at v1.54.3** — the newest release whose own `third_party/protobuf` is exactly the commit §17 pins | So gRPC's protobuf and ours are one pin, not two that must agree. **Verified rather than assumed**: it builds against `engine/vendor/protobuf` under the pinned flags and emits nothing `check_flags.cmake` forbids. Two residuals go to PR 0 (ADR 0013 §1) |
| What does "the determinism suite" mean for a UI? | **A projection golden**: each view rendered from a fixed `Song` to a serialisable description, compared against committed bytes | §11 gains a bullet in the ADR PR, not at the close, because it is the mechanical check question 2's answer needs. `node:test` and `tsx`, no browser, no display, no driver. It claims nothing about pixels or wiring and says so (ADR 0012 §5) |
| Where do §9's instrument/effect editors live? | **M2**, as a generic parameter editor over the build manifest | Amends ADR 0003 §5. Placing them is what made the three ledger rows' triggers checkable — and two of the three turn out **not** to be M2's, which is only visible once there is a real editor to compare them against (ADR 0014 §1, §3) |
| What does the UI do with a dry run it is holding? | **Apply optimistically**; §4.3's `version` check refuses | The only option that adds no rule. Its corollary closes trap 11 for free: a dry run already returns real ids minted from a fork, so the frontend never mints one (ADR 0012 §4) |
| Who may open an `.escri`, and what enforces it? | **A `Session` per project**, built by the host as a library — and `.escri/lock`, taken with `create_new` and never broken automatically | Amends ADR 0006 §5: the rule is one *session* per project, and the tool surface it was protecting is unchanged. The known gap closes in PR 2, where the shell first opens a project (ADR 0012 §3) |
| What does §17 gain, and what is a pin for a frontend? | **Enumeration by exact version**, as `schema.typescript` is, and the set is **one sign-off** | `react: null` and `codemirror: "6.x"` were non-pins inside the file that exists to prevent them. Seven pinned `app` entries and three reused; no state library, no test framework, no CSS framework, no Fast Refresh plugin, each with its cost written out. §17's golden-render pass is scoped to pins that can reach a render (ADR 0016) |
| Which platforms does M2 target? | **Linux x86-64 only**, as M1 | macOS and Windows unclaimed, not contradicted — ADR 0009 §1's own formulation. **§15's `[OPEN]` minimum-supported-OS-versions item is not resolved by this** and stays in §15's closing paragraph: this answers M2's scope, not a product commitment that outlives M2 (ADR 0014 §2) |

**The two that removed the most work.** Question 3 was framed as "three ADRs and two schema
changes before a pixel is drawn"; it is one semantics change, no `song.proto` field, and the
M0.4 goldens untouched, because ADR 0001 §3's globally unique ids already do what a new field
would have done. And question 6 was the one that could have cost the milestone a re-plan —
grpc++ against a protobuf from 2022 — and it was answered by looking at what gRPC's own
submodule pins rather than by arguing about it.

### The unverified half, and where it goes

Question 6 was answered by measurement, and the measurement did not cover everything. What was
established, cheaply, on 2026-09-07: gRPC v1.53.0 through v1.54.3 pin `third_party/protobuf` at
`f0dc78d…`, the commit `lock.baseline.json` already records, and v1.55.0 moves to protobuf 22;
v1.54.3 configured in 9.4 s and built `libgrpc++.a` and `grpc_cpp_plugin` in 2 m 53 s over 1345
targets against `engine/vendor/protobuf` itself, under `-march=x86-64 -mtune=generic
-ffp-contract=off`, emitting no `-ffast-math`, no second `-march`, and only the `-maes` and
`-msse4.1` on abseil's randen that `engine/cmake/check_flags.cmake` already allows; and the
bundled plugins are `ExternalProject`s, so sfizz-ui's own abseil is not in the engine's tree and
collides with nothing.

What was **not** established: that gRPC coexists with the engine's existing
`add_subdirectory(vendor/protobuf)` in one tree — its `module` provider would add protobuf a
second time and collide on every target — and what it costs on a two-core runner in a job that
is already up to an hour cold. Both are PR 0's, and both fail as a red build rather than as a
wrong answer, which is why the decision does not wait on them and the code does.

### PRs

| # | Branch | Adds |
|---|---|---|
| 0 | `m2.0-spike` (**never merged**) | The two measurements above, plus the third the ADRs could not take: whether a JUCE audio device opens and plays while the same process serves a socket, and what pumps the message loop while it does (ADR 0013 §4). M1's PR 0 is the precedent — a spike exists because an ADR cannot honestly be written without it, and PR 1's grpc++ verification is why this one is two questions shorter than the plan expected |
| 1 | `m2.1-adrs` | ADRs 0012–0016, their §15 rows, §17's gRPC and frontend pins with `lock.baseline.json` mirroring them, and the amendments in place to ADR 0003 §5, ADR 0006 §5 and ADR 0010 §4. **No code** |
| 2 | `m2.2-app-shell` | **The first PR that produces a window a person can open**, and deliberately the smallest one that can. `app/`: the Tauri host embedding `core`, `package.json` against ADR 0016 §2's list, the frontend build, its place in the workspace and in CI — and one window that opens an `.escri`, takes its lock, and draws the arrangement from a real `get_song`. It exercises ADR 0012 §1's call path end to end rather than stubbing it, including the `core/src/mcp.rs` refactor into `call(session, name, args)`: a transport a later PR is the first to exercise is the second wire path this milestone exists to avoid. **What it does not do**: no editing, no piano roll, no mixer, no history, no editors, no preview, no engine, and no `song.json` hash in the status bar — that number is a §11 claim and waits for `core` to report it rather than for the UI to compute it (trap 4). Named for what it delivers, as M1's PR 5 was: a window that opens a project and draws it |
| 3 | `m2.3-proto-ts` | TypeScript codegen for `proto/` (ADR 0006 §7), and `render.proto`'s **whole** M2 shape in one change — `Preview` with its command and event messages (ADR 0013 §2) and `PlanTrack.mix_lanes` (ADR 0015 §2) — so `buf breaking` compares it once against a `main` that has not moved (trap 12). The plan goldens gain `"mix_lanes": []` here and nowhere else |
| 4 | `m2.4-read-views` | The piano roll, and the arrangement completed. Read-only: projections of `get_song` with no edit path at all, so ADR 0012 §2's answer is reviewed on its own. **ADR 0012 §5's projection golden lands here**, with the first views it can cover, rather than at the close |
| 5 | `m2.5-edits` | The first control that is a tool call, its dry-run and diff, and the undo/redo tools behind ⌘Z. Where trap 2 gets decided against a real drag. **Done**, and the trap's sharp edge came apart under measurement: "coalescing" bundled *commit once* with *send nothing until the release*, and only the second is what makes an intermediate position unvalidated. ADR 0017 takes one of each — every position is a `dry_run`, one entry is committed — so a drag that crosses a refusal is drawn refused at the boundary and a drag that lands legally applies. The deciding number was not the dry run's cost but `Project::write`'s slope (trap 8): 30.6 ms per applied call at 300 entries against 17 ms at 21 |
| 6 | `m2.6-mix-automation` | `ParamRef` reaching a fader: the validator arm, `param_out_of_range`'s two domains, `compile` emitting `mix_lanes`, and the engine driving Tracktion's track volume and pan with **our** curve formulas (ADR 0015 §1, §2). No `song.proto` change, so no M0.4 goldens move; the render goldens must not move either, and any byte that does needs a named cause |
| 7 | `m2.7-mixer-and-editors` | The mixer over the automation PR 6 laid down, and §9's seventh view: the generic parameter editor over the build manifest (ADR 0014 §1). Two forms over two maps, in one PR because they are the same form. **Done**, and being the same form is what kept the two *numbers* apart: one `<input type="range">`, two builders, and a domain carried as a value — the mixer writes `Mix`'s own units by `apply_patch`, the editor writes a plugin's normalised `0..1` by `set_param`, and `set_param` answers a track id with `device_unknown`, which is the same boundary `mute` and `solo` meet as `ParamRef` targets (ADR 0015 §1). ADR 0017 transferred whole: 60 pointer positions on a fader became 32 dry runs and 0 entries, `Apply` one, ⌘Z one. What it did **not** transfer is the refusal — a fader's travel is inside the validator's range in two of the three domains and unbounded in the third, so no legal fader can be refused, and the only refusal a form can reach is a parameter this build's manifest does not declare. Two defects only the window could show: the detail pane's `flex-basis: auto` made 2855 rows shrink the arrangement to a sliver, and a long refusal wrapped the pane head mid-drag and moved every row under it by 24 px — ADR 0017 §3's own defect in the element chosen to avoid it |
| 8 | `m2.8-history` | The patch-log view with its provenance column, branch switching, and `merge_branch`'s per-path resolution (ADR 0015 §3). **Done.** The row named three things and the fourth was the one worth finding: what `merge_branch` *returns* for a conflict, which ADR 0015 §3 had assumed rather than checked. It matched — one `merge_conflict` `Violation` per path, keyed by the other side's operation path — so the map is keyed by a string the caller was already handed and nothing translates. Resolution is then a **filter on the other side's ops**: theirs wins by being applied, ours by not being, and there is no code path an unconflicted merge does not already take. Two things fell out that the ADR did not name and now does: a pick for a path that merge is not in conflict about is refused (`resolution_unknown`), because ignoring it either lets a typo pass for a choice or edits the merge; and a merge resolved entirely to this branch still records its two-parent entry with **empty `ops`** — the one place a call that changes no document writes one, since what it records is the join and without it the same conflict returns for ever. Driven in the window against a real conflict: two branches riding one track's `gain_db` and `pan`, resolved one each way, and the merge landed with `gain_db` mine and `pan` theirs. ADR 0017 does **not** extend to the form — nothing is dragged, so nothing is measuring a distance against the layout — but the pane head's no-wrap rule still earns its keep, because a conflict message is the longest refusal in the application |
| 9 | `m2.9-engine-grpc` | `Render` over gRPC, the stdio path deleted, `core/src/engine.rs` and the render suite moved onto the new transport, and gRPC added to `tests/renders.rs`'s `COMPONENTS` and the engine job's list. **The silent PR**: the four goldens must not move, and any byte that does needs a named cause (trap 5). **Done, and no byte moved.** The build questions ADR 0013 §1 left open were answered by building: gRPC owns the one `add_subdirectory` of our protobuf, aimed at our submodule, and costs 11 m 16 s cold on two cores and 6.1 s warm. The design question the row did not anticipate is how `core` learns where to dial — the engine names its own socket and prints it once the server is listening, so the address and the readiness are one line and nothing polls or sleeps (ADR 0013 §3, amended). One flake was found and fixed rather than lived with: this suite writes the program it then executes, and under load `exec` refused it with `ETXTBSY` twice in forty runs. And one consequence nothing predicted: the engine's provenance is *one* list, so `grpc` joining it reaches the build manifest's `engine` block and therefore every project's `lock.json` — five determinism goldens gained one line each, which is `tests/AGENTS.md`'s pin rule arriving rather than a surprise, and it was CI's fixture-subset step that said so |
| 10 | `m2.10-preview` | Preview playback: the live process, the audio device, the `Preview` stream, and `render_preview` — one of §5's tools, and the milestone it has been between since M0.3. **Done, and the measurement came first**, because PR 0 never took it: ADR 0013 §4's shape **held** — `main` pumps the JUCE loop and drains the stream's commands between turns, gRPC reads and writes on its own threads, the device calls back on ALSA's — measured against a real JUCE ALSA device on ALSA's `null` PCM, since this machine has **no ALSA device at all** and JUCE speaks no PulseAudio. Every later command answers in one 10 ms turn; a first play, cold, in about 190 ms. The package a person installs to hear it here is named in ADR 0013 §4 (`libasound2-plugins`, and a default PCM of `type pulse`). The design question the row did not name was **which event answers which command**: the transport writes events of its own while it plays, order cannot tell them from an answer, and `PreviewEvent.applied` is the one field `render.proto`'s preview shape took after PR 3. Three things became checks rather than sentences: a machine with no device is exit 6 before any socket exists (the engine job runs it), an export refuses to render if it was offered a device type (every render in CI runs that), and an export process answers a `Preview` stream `UNIMPLEMENTED` (asserted against the real binary, and a mutant that served one failed it). The scripting guard grew a tooth it lacked: a tool scripted only as dry runs now fails it unless it names the test that asserts its seam. The window plays, stops and returns to the start, draws the engine's own tick, and says *live preview · not the render* beside the button. **Found on the way and not fixed here:** a real export of `tests/determinism/render` crashes the engine on `main` too — heap corruption after Rubber Band warns about a 0.0853 stretch ratio — which is M1's and PR 11's (Known gaps) |
| 11 | `m2.11-review-fixes` | A whole-stack review's findings. M0 averaged four to sixteen per milestone and M1 returned eighteen; budgeting a PR for it is cheaper than discovering it. **Done.** One blocker, three majors, three minors, and the Known gap it was asked only to investigate turned out to be a second blocker. ⌘Z in a fresh session went *forwards*, because the undo cursor lived in the session: it now lives nowhere, and the log is replayed as an undo stack on every press (ADR 0005 §4, amended). `escribass-mcp` dropped answers still queued five seconds after stdin closed and applied calls in the scheduler's order: a transport adapter now serves one request at a time and withholds EOF (ADR 0006 §6, extended). The render suite's staleness guard did not walk the two protos the engine generates C++ from, and the engine job skipped a pull request touching only `compile` or the engine client. The export crash reproduced every time once driven with seeded ids, and is Rubber Band overflowing at a ratio of 6000: refused on the library's own warning (ADR 0011 §3, extended). Every fix has a test that was watched failing first. No render golden moved; one determinism golden did, by one sentence — `nothing_to_redo`'s message stopped saying "this session" |
| 12 | `m2.12-close` | Docs walked against the code, the deferred ledger walked again, `CLAUDE.md` to M3. **Done**, and the walk was held to M1's standard — each of §11's six bullets named against the test, CI step or code path that enforces it, by reading and by running, never from memory — and it turned up nothing missing from the code and one thing missing from the *record*: every §11 enforcer has run only on one machine since M2 PR 8, because GitHub has refused every job since, on the pull requests and on `main`. The "M2, closed" section below says so, with the run ids. Two more findings, both left where they are: ADR 0010's Consequences promised M2 "a re-pin tool and the UI that makes `lock_mismatch` recoverable without a text editor" and M2 delivered neither (a new ledger row); and `Preview::drop` discards the engine's exit status (a Known gap, found in PR 11). Three sign-offs in `lock.baseline.json` and §15 were first listed as what they were — claims about a person's decision that the repository cannot verify — neither removed nor confirmed; the user then confirmed all three (2026-09-17). Nothing here is code |

**Which rows can be sized now, and which cannot.** PR 2 is the one that changed most: question 1
made it a Tauri host with an embedded session rather than a host with a server and a proxy, and
it moved *ahead* of the codegen PR because the window's first view needs `schema/`'s TypeScript
and an argument-free `get_song`, not `proto/`'s. PRs 4, 5, 7 and 8 are all sized by question 2's
answer, which is now one re-read per edit — measurable in PR 4, and with a written escape if it
is too slow. PR 6 is one PR rather than three, because question 3 cost no schema change. PR 9 is
still a build investigation before it is a feature, but a much shorter one than the plan feared:
what it integrates is a library already shown to compile against our protobuf.

The split follows M0.2's lesson, which M1 confirmed twice: PR 4 is the loud concern (do the views
draw the model), PR 9 the silent one (did anything about the audio change when the transport
did). Mixing them gets the silent half reviewed as plumbing.

### Traps

1. **The store that becomes a second model.** React's answer to many views over one document is
   a normalised store, and it arrives as a performance fix in PR 7, not as a design decision in
   PR 4. §14.2 forbids it. **Answered in part:** ADR 0012 §2 decides the shape and ADR 0012 §5's
   projection golden is the mechanical check the prose could not be, and ADR 0016 §3 keeps every
   library that would make one out of the dependency list. What is left is that a store can still
   be hand-written, and the golden is what would catch it.
2. ~~**A drag is not one tool call.**~~ — **decided in PR 5 against a real drag, as ADR 0017,
   and the sharp edge turned out to rest on a hidden assumption.** "Coalescing on release" was
   read as two things at once — commit once, *and send nothing until the release* — and only the
   second is what leaves an intermediate position unvalidated. PR 5 takes one of each: every
   position the pointer passes through goes out as a **`dry_run`**, which validates and writes
   nothing, and one entry is committed when a person approves the diff. So a drag that crosses a
   refused position is drawn refused *at the boundary*, naming the rule, and a drag that then
   lands somewhere legal applies — correctly, because the intermediate positions were candidate
   values and never states of the document. The frontend contains no rule of its own; predicting
   the refusal in TypeScript would be ADR 0012 §2's rejected second implementation one language
   over, and clamping the drag would be that *and* silent.

   Measured, not argued. A real drag in the window: **100 pointer positions, 30 of them reaching
   the tool API as dry runs, 0 entries in the log**; `Apply` appended one; ⌘Z appended one more.
   Driving it also found the one defect nothing else could have: the diff pane opening *during*
   the drag moved the roll 42 px, so a 48 px drag landed two semitones from where it was aimed
   instead of four. Nothing may reflow while a pointer is down (ADR 0017 §3, §5).
   And the number that actually decided it is not the dry run's cost (1–3 ms, writes nothing)
   but the applied call's slope: **30.6 ms at 300 entries against 17 ms at 21**, because
   `Project::write` rewrites every entry file. One call per pointer event is a hundred writes
   that each rewrite the whole log — which is trap 8, arriving in the pull request this file
   predicted it would.
3. **Preview and export do not agree, and that is correct.** sfizz switches to freewheeling
   quality settings for an offline render, so §8 already says a golden is deliberately not what
   a preview plays; Surge XT's factory patch reaches a wall-clock RNG unless `A Osc 1 Retrigger`
   is set; and the audio device's sample rate is the user's while the render's is
   `RenderTarget`'s, which puts a resampler in one path and not the other. A UI that publishes a
   hash and plays a different sound will be reported as a bug by the first person who checks.
   **Answered in PR 10 where a person presses play**: the button sits beside *live preview · not
   the render*, with those three reasons on hover; `PreviewResponse` carries no hash; and the
   window shows no render hash anywhere (trap 4). PR 10 found a fourth, smaller difference and
   wrote it down: Tracktion's reported transport position is its UI's, refreshed by a timer and
   held for 200 ms after a seek, so a playhead can trail the audio by a timer period.
4. **The status bar is a `[MUST]` rendered as a widget.** Plate 1 shows the `song.json` hash,
   the patch count, the last render hash and `lock.json 14/14 verified`; Plate 6 shows "0
   differing samples outside the edited range". Every one of those is a §11 claim, and a widget
   that computes it a second way is a second implementation of the thing the suite exists to
   check. They come from the same code or they drift, and the drift is invisible until a demo.
   PR 2 is on record as showing none of them for exactly this reason.
5. ~~**Deleting stdio deletes the only tested path.**~~ — **held, in PR 9, and the order is in
   the history rather than only in this line.** Three commits: the engine gained its gRPC server
   with stdio still there; `core`, `core/tests/engine.rs`, `core/tests/mcp.rs`, the engine job's
   six shell steps and `tests/renders.rs`'s `COMPONENTS` moved onto it; then stdio went. The
   goldens did not move — `audio_clip`, `dexed` and `surge_xt` reproduced their committed bytes
   over the new transport *before* the old one was deleted, and one plan hashed identically down
   a pipe and down a socket. What the trap did not predict is where the cost landed: not in the
   engine, which gained a service class and a wait loop, but in the six CI steps that drove the
   engine by writing to its stdin and cannot dial gRPC from a shell. They drive `render-once`, a
   cargo binary that is `core`'s own `Engine` behind an argv, which makes the engine job build
   Rust for the first time and those steps exercise the client that ships.
6. **A live engine is a resident plugin instance, which is the thing ADR 0008 §2 refused.**
   **Answered:** ADR 0013 §3 keeps preview and export in different processes, so an export never
   inherits a smoother. The trap survives as the thing that must not be optimised away — one
   process serving both is the obvious saving, and it is the one that cannot be taken. **Held in
   PR 10, by test rather than by care:** an export while a preview plays is a second process in
   `--render` mode (`core/tests/preview.rs` counts them), and a `Preview` stream opened on an
   export process is answered `UNIMPLEMENTED` by gRPC (`tests/renders.rs`, against the real
   binary; a mutant engine that also served `Preview` failed it). What a preview *does* keep
   resident is deliberate and inside its own process: a play of the plan already loaded builds
   nothing, so a stop and a play reuse the instances.
7. **The webview cannot be pinned.** §17 pins by commit or exact version; the engine the
   frontend renders in ships with the operating system, moves under the user, and paints a
   canvas differently across versions. **Recorded rather than solved:** §17's Tauri row says so,
   and ADR 0012 §5's golden is a description of what a view would draw and never a screenshot,
   which is what keeps the trap out of CI.
8. **`Project::write` is O(history) and `app` is the first long session.** The known gap names
   this trigger exactly: "a session that stays open and keeps appending, which is `app`". Every
   edit rewrites every entry file. It will present as UI lag, be diagnosed in the frontend, and
   live in `core`. PR 5 is the first PR that appends in a loop. **Measured in PR 5 rather than
   fixed**: an applied `set_notes` on a five-track project takes ~17 ms at 21 entries and
   ~30.6 ms at 300 — half again as expensive by the three-hundredth — and 300 in a row take
   6.7 s. That measurement is half of ADR 0017's argument for one entry per gesture, and the gap
   stays open with a number attached rather than a prediction.
9. **Two writers, and nothing locks the directory. Answered:** ADR 0012 §3 takes `.escri/lock`
   in PR 2, with `create_new`, reported and never broken automatically. What is left is the
   failure mode that choice buys — a crashed process leaves a project that will not open until
   someone removes the file — which is git's `index.lock` bargain and is taken deliberately.
10. **Undo is not a stack.** ADR 0005 §4: undo appends an inverse entry and never rewinds a ref.
    Every editor framework ships an undo stack, and one here disagrees with the log the moment a
    branch is switched or a second writer commits. Plate 5's entire point is that there is no
    second stack. ADR 0016 §3's empty dependency list is half the defence; the other half is that
    ⌘Z calls a tool. **Held in PR 5**: ⌘Z is `tool("undo", {})` and nothing else, and what the
    session holds is a cursor into the *log's* first-parent chain — entry ids, cleared by any
    other commit and by a branch switch, which is the case a frontend stack gets wrong.
    **And sprung one level down in PR 11:** a stack in the *session* disagreed with the log the
    moment the process was relaunched — its cursor was empty, and ⌘Z re-applied an edit. The
    session now holds nothing; the chain is replayed as an undo stack on every press (ADR 0005
    §4, amended).
11. **Ids that the UI mints. Answered:** ADR 0012 §4 keys a pending edit by the ids the dry run
    already minted from a fork, so nothing in the frontend generates one. The trap survives as a
    rule with no enforcement: CLAUDE.md #3 names `core`, compilers and `engine`, and a
    `Math.random()` in `app/` is caught by review or not at all.
12. **`buf breaking` is the only guard on `render.proto`, and it runs on pull requests only.**
    Both changes it needs — `Preview` and `mix_lanes` — land in PR 3 together, for this reason.
13. **CI has no sound card.** M1 measured that headless works and needs no display, for
    *instantiating* a VST3. Opening an audio device is a different question, and preview
    playback would be the first thing in this repository that cannot be tested where everything
    else is tested. PR 0 measures it; ADR 0012 §5's golden does not pretend to cover it.
    **Measured in PR 10, and true.** What is tested without a device is everything that is not
    sound: `core`'s whole half of the stream against a model of the engine (the process and its
    mode, one process across commands, the plan a play carries, answers told from chatter, the
    refusals, a machine with no device, a stream the engine ends, the process leaving when its
    stream closes); the tool through both transports in the determinism suite; and, in the
    engine job, the real binary on a runner with no sound card exiting 6 before it announces a
    socket. What is not tested in CI is the engine's half on a device — a live edit, a moving
    transport, a loop, a seek, a resume — and it is **untested loudly**: `tests/renders.rs`
    carries it as an ignored test whose reason prints on every run of the render suite, which
    passes against ALSA's `null` PCM and fails on a machine with no device saying so. Sound
    reaching a speaker, real-time pacing and underruns are measured nowhere, and say so.

### What M2 will not claim

- **Not that the UI is deterministic** in the sense §11's first bullet means. What ADR 0012 §5
  claims is narrower and is written that way: every view is a pure function of the model. No
  golden covers a pixel, a frame time or a gesture.
- **Not macOS or Windows.** Linux x86-64, on the image and compiler §17 pins, exactly as M1
  (ADR 0014 §2). Unclaimed, not contradicted.
- **Not that a preview sounds like an export.** §8 already says the opposite for sfizz, and
  trap 3 lists two more reasons.
- **Not the AI panel.** It is M3 (§16), and it is drawn in Plate 1 of the wireframes, which is
  exactly why this needs saying: the plate a reader remembers is the one M2 does not build.
- **Not code views** (M4, ADR 0003 §8), **not DAWproject or MIDI** (M5), **not an installer**
  (M5). M2 runs from a build tree.
- **Not the plugin's own editor.** M2's seventh view is a form over the build manifest showing
  normalised values beside the names the plugin reports; a VST3 window inside the engine process
  is a different feature (ADR 0014 §1).
- **Not that a user-chosen patch is reproducible.** §8 forbids Surge XT's `rand_pm1` paths,
  Dexed's LFO waveform 5 and sfizz's `*_random` *in a fixture*, and M2 is the milestone that
  lets a user choose one. Closing it needs a denylist of `ParamID`s the build manifest cannot
  yet produce (ADR 0014 §3).
- **Not a resolution of any `[OPEN]` item.** Minimum supported OS versions is the one M2 walks
  into, and ADR 0014 §2 answers M2's scope without touching it. The other three are unchanged.

Every line above was checked against `main` at `b0520f4` at the close and every one still holds:
no view goldens a pixel, CI names `ubuntu-24.04` and nothing else, the play button sits beside
*live preview · not the render*, `app/src` has no AI panel and imports nothing from `codemirror`,
the parameter editor is `form.tsx` over the manifest and hosts no plugin window, no denylist
exists, and §15's closing paragraph lists the same four `[OPEN]` items it did on 2026-09-07.

## M2, closed

Twelve pull requests, six ADRs, one whole-stack review. What M2 delivers: a Tauri host that
embeds `core` and dispatches every control through the function the MCP server already
dispatched through; five of §9's views as pure projections of one decoded `Song`, with a golden
that proves it; ⌘Z as a tool; `merge_branch` finished one path at a time; a `ParamRef` that
reaches a fader with no schema change; the engine serving `Render` and `Preview` over gRPC with
the pipe deleted; and preview playback, measured to the edge of the sound card and not past it.
The four golden WAVs are byte-for-byte what M1 blessed.

M0's and M1's lesson held a third time, and this milestone's review found the same class of
defect one layer up: not a wrong render but a wrong *session* — a fresh process whose ⌘Z walked
past undos it had not made and re-applied an edit, recorded as `undo`. The cursor that had to
stay alive to be right was state the log should carry, and now does (ADR 0005 §4, amended).

**What this close verified, and on what.** §11's six bullets were walked as M1's close walked
five: for each, the enforcer was named and then read or run. All six have one. Unseeded
randomness and wall-clock reads are structural in `core` (`clock.rs`, `id.rs`) and absent from
`engine/src` (its one `mkdtemp` names a directory, not a sample); the lock is `Project::open`,
which the Tauri host calls too; the goldens are `tests/renders.rs`; the projection golden is
`app/tests/projection.test.ts`; the determinism suite is `tests/determinism.rs`. On 2026-09-17,
on this machine — an AMD Ryzen AI 9 HX PRO 370 under WSL2, Ubuntu 24.04, g++ 13.3 — against an
engine rebuilt from `main`'s `engine/src`: both codegen checks clean; 422 workspace tests, 17
determinism tests, 9 render tests with the device test ignored, 6 projection tests, the host's
test, schema's three suites, `buf lint`, `buf format` and `buf breaking` against `main`, all
pass; the manifest fixture is still a subset of the built manifest; a preview with no device
exits 6 with nothing on stdout. Every one of those numbers is from **one machine**, and that is
the first carried item.

**What M2 leaves unverified**, recorded here because the pull requests that carry it are not
where a reader of this file will look:

- **M2 PRs 9, 10 and 11 were merged on local CI runs, and `main` has no green run since PR 8.**
  A GitHub billing limit refused the jobs — `renders` and `cross-cpu` on PR 9 (#63), every job on
  PRs 10 (#64) and 11 (#65) — and refused them again on each merge commit: the runs on `main` for
  `8bc6487`, `8b0678f` and `b0520f4` are all `failure`, and the last success on `main` is
  `17356bd` (M2 PR 8, 2026-09-09). The local runs are recorded as comments on #63, #64 and #65,
  and GitHub's check history for those three pull requests shows failures, not passes. This
  pull request is documentation only, so CI's path gate would skip it even if the limit lifted;
  it changes none of the above. Until a full run on `main` succeeds: `checks`, `app` and
  `engine` last ran on GitHub for PR 9's head, `renders` and `cross-cpu` for PR 8's merge, and
  everything since has been exercised on one machine and one CPU model — which is exactly what
  ADR 0009 §6 says is not evidence.
- **Preview has never played on a real audio device anywhere.** The thread shape was measured
  against ALSA's `null` PCM, which does not pace (ADR 0013 §4);
  `a_preview_plays_on_this_machines_audio_device` is `#[ignore]`d and prints why on every run.
  Sound reaching a speaker, real-time pacing and underruns are measured nowhere, and
  `roadmap.md`'s "press play and hear it" is a description of M2's intent, not of anything
  measured.
- **ADR 0009 §6's cross-CPU question is unanswerable at the current sfizz pin**: its AVX dispatch
  table is empty (M1 PR 13), and M2 changed no plugin pin. `cross-cpu` has not run since PR 8
  either way.
- **`Preview::drop` ignores the engine's exit status** (`core/src/engine.rs`), so a preview
  engine that crashes after its last command goes unreported. Found in PR 11, not fixed; a Known
  gap below.
- **Logs written by the pre-PR-11 `undo` defect are not repaired.** A project whose log already
  carries an `undo` that went forwards is read as it says (ADR 0005 §4, amended); a Known gap.
- **The plugin manifest was not rebuilt in PR 11 or here.** Dexed's build needs `jack/jack.h`,
  not installed on the machine every local run was made on; the engine binary is current and
  the manifest is the 2026-09-09 build, which the fixture-subset check still matches because no
  pin has moved. CI's `Build` step, which regenerates it, has not run since PR 8.
- **ADR 0010's Consequences promised M2 a re-pin tool** and the UI that makes `lock_mismatch`
  recoverable without a text editor, and nothing in M2's plan, its ADRs or its code delivered
  either — ADR 0010 §3's "delete the entry and the next write pins it" is still the only way to
  re-pin. A ledger row below carries it rather than the sentence in ADR 0010 carrying it alone.
- ~~**Three sign-offs the repository cannot verify.**~~ **Confirmed by the user, 2026-09-17.**
  `lock.baseline.json` and §15/§17 record protobuf v21.12 as "signed off 2026-09-07" (changed from
  "pending sign-off" in M2 PR 2, whose commit says the user said so), gRPC v1.54.3 as "approved as
  a dependency under CLAUDE.md #4", and the frontend package set as "one sign-off" (ADR 0016 §2).
  This close first listed them as claims no agent could verify, because the repository does not
  hold who decided; asked directly, the user confirmed all three. The recorded words and dates
  stand as written — the confirmation is of them, not a replacement.

What M2 does not claim is above, checked; what M1 did not claim, M2 inherits unchanged: Linux
x86-64 on one image and compiler, and nothing about any other CPU.

## M3 — AI loop

Planned 2026-09-17 against `main` at `1b87fea`. Same reason as the M0.4, M1 and M2 plans: the
reasoning is the expensive part, none of it is in code yet, and a conversation is not where it
should live. M1's planning PR raised eight questions and a second PR answered them; M2's raised
twelve and a second PR answered those. This is the first half only, and the separation is the
point — an ADR written before its question is settled is what that separation exists to
prevent. **Nothing below is decided.** Where a question has a default an agent would propose,
the default is written beside it and the question stays open; where a question is a person's,
`CLAUDE.md` says so and it is listed apart.

One thing is different about this milestone and shapes the whole plan. Every milestone so far
put something deterministic under the tool API — a validator, a render, a view. M3 puts
something **nondeterministic above it**: a hosted language model, which cannot be seeded,
cannot be pinned by anyone, and changes under its own name. What the determinism guarantee can
honestly mean once that is true is worked out below, before the questions, because half of the
questions turn on it.

### What was already decided, so M3 does not re-decide it

| Decided | Where | Consequence |
|---|---|---|
| `ai` is a **Python 3.12 sidecar** under `uv`, and its model types are the generated Pydantic ones — nothing hand-written | §3, tier 1; §6; §17; CLAUDE.md, Toolchain; §4.1 | `schema/gen/python` has existed since M0.1 and `schema/tests/test_roundtrip.py` reads the fixture with it. `proto/` generates no Python: ADR 0006 §7 deferred it to M3, and this is M3 (PR 3) |
| `app` ↔ `ai` is **gRPC**, and `app` supervises `ai` as it supervises `engine` | §3; §9 | Not a pipe, not MCP, not HTTP. But `app` **opens no network port** (ADR 0012 §1), so the socket is a Unix one, and the engine already shows the shape: it names its own socket and prints it once listening (ADR 0013 §3). *Which* side listens is question 1 |
| The AI edits through the tool API as JSON Patch, exactly as the UI does, and holds **no second representation** of the song | CLAUDE.md #1, #2; §2.1, §2.3, §5, §14.2 | The model never sees `song.json` and never writes it; what it sees is a read and what it does is a tool call. A Libretto view is a projection and is discarded, never stored (trap 7) |
| The loop is **system prompt + song summary + tool schemas → tool calls → dry run → apply**, a validation error fed back, **three retries at most**; provider-agnostic behind an OpenAI-compatible client; **OpenRouter in v1**; no local LLM ships | §6.1; §15 | The shape is given; what is not given is which failures count as "validation error" (question 8), what the summary is (the Libretto ADR), and what a retry sends back |
| The AI process is **audio-agnostic** and has no audio path | CLAUDE.md #6; §6; §14.6 | It cannot listen to a preview or a render. Anything in the loop that needs to know how something sounds needs an answer that is not "ask the model" — and M3 as specified has no such thing, which is the honest reading (see "What M3 will not claim") |
| The **Libretto-grammar ADR precedes M3** and "constrains M3's design rather than following from it" | §16; §18.2; ADR 0003 §6 | It is not one question among the rest. It is PR 1 on its own, before the ADRs that answer the questions, and "The Libretto ADR, first" below says what it has to decide |
| A caller-fixable failure is `valid = false`; an operator failure is `Err`; **the split is made once, in the session** | ADR 0006 §2; §5 | Written for exactly this loop, two milestones before it had a consumer. M3 is its first real one, and a misclassified error is a retry loop that cannot succeed (trap 2) |
| A proposal is **dry runs, committed once, and a person applies**; the panel shows the RFC 6902 diff before anything lands; a person may apply, reject or edit | §9; ADR 0017 §1–§4 and its Consequences ("every gesture M2 and M3 add") | The AI panel is the same flow with a different author. What ADR 0017 did not have to face is a proposal made of *several* tool calls, and that is question 9 |
| **Every entity and every log entry carries `Provenance`**, with `author` `HUMAN` or `MODEL` and optional `model_id`, `prompt_id`, `tool_call_id`; the history view already shows the author column | §4.3; `song.proto`; `history.proto`; ADR 0012 §1; §9 | The fields have existed since M0.1 and **nothing has ever set the three optional ones**: `core` writes `None` for all three at every site (`tools.rs`, `project.rs`, `session.rs`). Filling them is M3's, and *how* they reach the log is question 3 |
| Tool schemas are **generated from the protobuf descriptor**, never hand-written, and omit `id`, `provenance` and `version` | ADR 0006 §4, §6 | The model's tool definitions are the MCP `inputSchema`s that already exist. No second description of the API is written in Python (trap 11) |
| `ai/` is in §13 | §13; `AGENTS.md` | No new-directory ADR. The precedent is `engine/` in M1 PR 5 and `app/` in M2 PR 2 |
| CLAUDE.md #3 names **`core`, `compilers` and `engine`** — not `ai` | CLAUDE.md #3; §11 | An LLM is nondeterministic by nature and the rule was written not to pretend otherwise. What M3 claims instead is below. §7.1's DSL, which runs *inside* `ai`, is M4's and must be pure — so `ai` is not exempt from the rule for ever, only for the model |
| Linux x86-64 only, on the image and compiler §17 pins | ADR 0009 §1; ADR 0014 §2 | M3 inherits the claim and does not widen it. A sidecar is a fourth process on the same machine |
| The `[OPEN]` items in §15 are a person's | CLAUDE.md, line 4; §15 | Two of the four sit directly in M3's path and are listed below as questions **for the user** |

**The deferred ledger and the known gaps send four rows and two carried items here.** The
§2.2 randomness row — refusing a plugin parameter that reaches an RNG nothing can seed — was
retriggered in M2 PR 11 to "**M3, before its tool-calling loop can call `set_param`**", and it
is the one row whose revisit point is an event this milestone produces on a schedule
(question 7). `schema/pyproject.toml`'s missing `[build-system]` waits on "when `ai/` depends
on it", which is PR 5. `Project::write`'s O(history) rewrite is a known gap whose text now
names "M3's loop, which commits without a person between calls" as the next thing that could
make it a defect rather than a number (trap 9). ADR 0010's Consequences promised M2 a re-pin
tool and M2 delivered none; its trigger — a `lock_mismatch` a person meets — has not fired,
and whether M3 fires it is question 14. And two of the items "M2, closed" carried forward are
not M3's to fix and are M3's to state: **CI has not gone green on `main` since M2 PR 8**, so
every check named in this plan will be run on one machine until GitHub runs a job again
(trap 6); and **preview has never played on a real audio device**, which the AI panel does not
change and does not depend on.

Two more things sit in no milestone and are placed here by this plan rather than by §16. §13
lists `/proto` as holding "`SongTools`, `Render`, `Jobs`", and §6 says "long tasks are jobs
with progress" — and no `Jobs` service has ever been defined, placed or mentioned since. M3's
loop is the first long task, so it is where the sentence is either honoured or amended
(question 2). And §17's Python row says "pin exact patch in `ai/.python-version`" while
`lock.baseline.json` records `"3.12"` — the row describes a file that does not exist, and PR 5
is where it starts to.

### What "deterministic" means with a model in the loop

§2.2 promises "same project file + same pinned versions → bit-identical rendered audio", and
"every source of randomness carries an explicit seed stored in the project". An LLM is a
source of randomness with no seed anyone can honour: temperature 0 is not determinism, a
`seed` parameter is honoured by some providers and ignored by others, OpenRouter routes one
model name across several providers, and the weights behind a name are replaced without the
name changing — which is `ubuntu-latest` (M1 trap 12) one layer up, with money attached. No
test in this repository can make two runs of a hosted model agree, and no test should be
written that pretends to.

So the claim is drawn at the tool API, and it is the same line CLAUDE.md #3 already draws.
**Below the line, nothing changes.** A model's edit enters as a tool call, becomes RFC 6902 ops
through the same `prepare`, is validated by the same rules, bumps the same versions, and is
appended to the same log with `Author::Model` in its provenance. M0.4's suite already proves
that replaying that log is byte-identical whoever wrote it, and M1's that rendering the result
is byte-identical whatever wrote it. **A song is therefore reproducible from its log even when
its author is not** — and that is not a weakening of §2.2, because a *person* dragging a note
was never seeded either. §2.2's boundary was always the model → render path; a human and an
LLM sit on the same side of it, as authors. The determinism guarantee survives a
nondeterministic author the way it survives a human one: by recording what the author did,
not by predicting it.

What M3 can add, and should be held to, is narrower and real. **The `ai` process's own code is
a pure function of what the model said.** The song view it renders, the six axes it computes,
the loop's decisions — which failure retries, what goes back to the model, what becomes a
proposal — contain no randomness and no clock of their own, and are goldened the way ADR 0012
§5 goldens a view: from a fixed `Song`, to a serialisable description, compared against
committed bytes. And the loop as a whole is goldened against a **scripted model**: a fake
provider replaying a recorded transcript, driven twice, producing a project compared byte for
byte with a committed one (question 11). "Same transcript → same log" is the claim, and it is
the one the suite can check without a key.

Three things follow that the ADRs should say outright. **Provenance is what makes the claim
useful**: a log entry that says `AUTHOR_MODEL` with no `model_id` records that *something*
wrote it; one that names the model and the tool call is an audit trail (§5's word) that a
reader can act on, which is why question 3 is not cosmetic. **A model id in `lock.json` is a
choice, not a pin**: §17's "model ids pinned per project under `ai.model`" cannot mean what
"pinned" means everywhere else in that table, and a status bar reading `lock.json 14/14
verified` must not count it (trap 15). And **the model is never told it is deterministic**: a
system prompt that says "your edits are reproducible" is true of the log and false of the
model, and the panel's wording has to make the same distinction the wireframes' *live preview
· not the render* makes for sound.

### The Libretto ADR, first

§18.2 asks for "a Libretto-style grammar for the composition layer's LLM-facing view — integer
onset slots on a bar grid (already implied by 960 PPQ ticks), explicit voices, bar-level blocks
— and its structural evaluation axes (rhythm, harmony, melody, texture, form, within-song
variation) as the AI orchestrator's self-check metrics", recorded "in an ADR before M3". ADR
0003 §6 places that ADR before M3 and says it constrains M3's design. It is therefore PR 1,
alone, and every question below that touches what the model reads or how it is checked is
answered downstream of it. What it has to decide, with the facts that bear on each:

1. **What the view is, concretely.** A bar-block is text; the model reads text. The ADR has to
   write one down — for a `Song` with two tracks, a clip each, a tempo change and a time
   signature — derived from ticks by a stated rule, and say which of §4.2's fields it carries
   and which it abstracts away. Libretto's paper abstracts velocity, micro-timing, timbre and
   **unpitched percussion** (`docs/landscape-2026-09.md`, Area 4). This project's bundled
   sampler plays drum kits and the wireframes' first track is `Drums · sfizz · kit_808.sfz`;
   a grammar that cannot say what a drum track does is a view the loop cannot use for the
   first thing a person asks. The ADR decides what is added to the paper's grammar for that,
   or says drums are outside it.
2. **Read only, or read and write.** §18.2 says "view". A model that *writes* in the grammar —
   a bar-block of notes that `ai` turns into `set_notes` — is a compiler in §2.3's sense ("free-form
   code only enters through compilers that validate before the result touches the model"), and
   a second way of writing notes beside the typed tool. A model that reads the grammar and
   writes through `set_notes` in ticks needs no compiler and no second path. The ADR decides
   which, and if it is the first, the compiler is M3's and needs its own golden.
3. **Where it is computed.** In `ai`, as a Python projection over the generated Pydantic `Song`
   (a second language projecting the model, which the projection golden has done in
   TypeScript since M2 PR 4); or in `core`, as a read tool every carrier serves, so an MCP
   client (§18.2 Stage 1) gets the same view the sidecar does. The second makes the view part
   of the tool API's surface and reviewed under `buf breaking`; the first keeps `proto/`
   unchanged and puts the golden under `unittest`.
4. **What the six axes are made of, and what they are for.** Libretto's are "corpus-calibrated"
   — a statistical space fitted to a corpus. This repository has no corpus and will not have
   one in M3. An uncalibrated axis is a number with no meaning attached, so the ADR has to say
   per axis what is computed from the `Song` alone (a rhythm axis can be onset density per
   slot; a harmony axis needs a key, which §6.3's key detection is `[OPEN]`), which are
   deferred, and what the model is told about them. They are **self-check text the model
   reads**, never a gate: nothing about a metric may decide whether a proposal is applied — a
   person does (§9), and a metric that refused a patch would be a validator rule nobody wrote
   an ADR for.
5. **That they are pure.** Six functions of the `Song`, no randomness, no clock, goldened
   against `tests/determinism/render/expected/song.json` — the same fixture ADR 0012 §5 chose
   for having something to get wrong — and against the same document with every map's keys
   reversed, for ADR 0012 §5's reason.
6. **What was read.** Libretto is arXiv 2606.22708 and the landscape found **no released
   code**. Whatever M3 builds is this repository's own reading of a paper, and the ADR says
   which sections it took the grammar and the axes from, so that a later reader comparing the
   two knows which is the source and which the interpretation.

The spike (PR 0) informs this ADR and does not decide it: whether a bar-block view lowers a
real model's invalid-call rate against a raw `get_song` is a number the ADR should cite, and it
is a measurement about one model on one day, dated as such (trap 4).
### Decisions taken, 2026-09-17

The five questions below were the user's. Four were answered directly, and the fifth is not
reached. After the spike, the user also chose the default model — measured before it was
recorded — and took question 7, which an agent could have proposed. The other thirteen an agent
can propose an answer to are unchanged, and are decided with their ADRs (PR 2), after the
Libretto ADR (PR 1).

| Question | Decided | Consequence |
|---|---|---|
| U1 — are §6's symbolic generation and analysis v1 scope? | **Out of M3; v1 scope decided later.** The `[OPEN]` item in §15 stays open | M3 is §16's loop, sidecar and panel, exactly as this plan sized it. The Libretto harmony axis has no key to compute against, and the Libretto ADR says so rather than inventing one. One thing the question surfaced stays true for whoever reopens it: §6.3's analysis, as written, needs audio — stem separation, tempo estimation, reference-track extraction — while §6's own last bullet and CLAUDE.md #6 keep the `ai` process audio-agnostic, so "analysis in v1" cannot mean analysis in `ai` as §6 describes it |
| U2 — which symbolic model for melody and drums? | **Not reached.** It was reachable only if U1 said yes | Stays `[OPEN]` in §15 beside U1, and is taken whenever U1 is |
| U3 — which model, and what does its pin mean? | **Recorded, not pinned.** `lock.json`'s `ai.model` records the *choice*; every entry's `provenance.model_id` records the model that *actually answered*. No reproducibility is claimed for a hosted model — only for the song, from its log | §17's "model ids pinned per project" is read as a record, not a guarantee, and §17 is amended with the ADR to say so. `OPENROUTER_API_KEY` reaches the sidecar from the environment and nowhere else: never a flag, never `lock.json`, never a recorded transcript (trap 10). **The default is `deepseek/deepseek-v4.1-flash`**, chosen by the user after the spike and **measured before it was recorded** — see "DeepSeek V4.1 Flash, measured before it became the default" below. It is not the model the spike's main run measured (`deepseek/deepseek-v4-flash`, a different release), which is why it was measured separately |
| U4 — every new dependency, for sign-off | **Both approved.** A Python gRPC stack — `grpcio` with `grpcio-tools`, or `grpclib`, whichever `betterproto2-compiler` 0.10.1 generates service stubs for, which PR 0 checks — and the `openai` Python SDK as the OpenRouter client. **Corrected after the spike:** the SDK today (3.14.1) carries `httpx2`, a differently named package, not `httpx` — and the user approved `httpx2` explicitly | Each is pinned by exact version in `lock.baseline.json` and §17 when it is first added, not before. **Nothing else is approved**: an agent framework, a prompt library, a tokenizer or a vector store returns to the user before it is installed (CLAUDE.md #4) |
| U5 — may an external MCP client edit beside the window? | **No; one process at a time.** An external MCP client drives a project only while the window does not have it open | `.escri/lock` (ADR 0012 §3) and ADR 0004's single-writer ordering stand unchanged. §18.2 Stage 1's "any MCP client can drive a project immediately" is narrowed to say it holds with the window closed — a spec amendment that lands with its ADR in PR 2, not here |
| Question 7 — the §2.2 randomness row, before the loop calls `set_param` | **`set_param` is withheld from the model** — option (c). The model is not offered the tool | The spike measured why a denylist cannot land as a list: 194 of Surge XT's 2,855 `ParamID`s reach an RNG nothing can seed by one stated rule, and 24 to 2,283 by others; a *default* value already reaches it (A Osc 1 Retrigger); changing an oscillator or effect type resets its dependents; and `Instrument.state` can switch these paths on and `apply_patch` can write it. So it needs a rule over values and combinations. V4.1 Flash then made the case concrete: on the one `set_param` instruction it set the **wrong** parameter and reported success, as V4 Flash had. The randomness row stays open for a later milestone; a person can still set parameters in the window |

### Decisions taken, 2026-09-21

The twelve questions the user's answers and ADR 0018 left, answered before code — four ADRs,
0019 to 0022, the §15 rows they force, amendments in place to ADR 0006 §4 and §7 and ADR 0012
§3, and the two defects the spike found decided with the PR that fixes each. Two things changed
between the plan and the answers, and both are said rather than inherited. **The spike changed
question 9**: the plan's default was a fork chain the model composes out of dry runs, and no
model composes anything out of dry runs, so the composing moved into `core`. And **the plan
contradicted itself on when the Python packages are pinned**: PR 2's row said "by exact
version" here, the user's U4 row says "when it is first added, not before"; the later,
user-confirmed decision wins, PR 2's row is amended below, and the pins are PR 5's.

| Question | Decided | Consequence |
|---|---|---|
| 9 — how is a proposal of several calls previewed? | **A proposal**: the model's calls applied in `core` to a fork of the document — cloned in memory, a fork of ids **kept** across calls, `prepare` on the copy — so the second call's track exists and nothing is written. **A person approves one patch, once, when the model's turn ends**: `diff(current, proposal)` with versions as one `prepare` computes, committed through `Session::run` under the tool name `proposal` | 116 successful runs and not one built a multi-call edit out of dry runs; every chain applied its first step and named the id the result returned. So the model is not asked to compose: it applies step by step, as it does anyway, against the fork. ADR 0017 §1 holds — one entry, one `Project::write` — and a document that moved is refused by the `version` check as a held drag is (ADR 0012 §4). The branch alternative moves `HEAD` under the window and writes the whole log per step; one call per proposal would have refused a third of the successful edits (ADR 0019 §1, §2) |
| 10 — what may the panel apply, and what may a person edit? | **Apply, Reject, Edit**, Edit last. Reject drops the fork and records nothing; Edit is the person's own `apply_patch`, `AUTHOR_HUMAN`, with the `prompt_id` kept and ADR 0017 §3's refusal path; one proposal pending at a time, a new prompt waits | A person who changed the bytes owns the bytes. The entry says `proposal`, the model and the prompt; the calls it was composed from are in the conversation under that `prompt_id`, and `history.proto` gains no field (ADR 0019 §3, §4) |
| 1 — who listens, and what crosses? | **(b): `ai` serves one bidirectional stream, `app` dials**, and the model's calls come back over it as a name and arguments the host executes against the proposal. Python for `proto/` with a server and no client (ADR 0006 §7, narrowed) | The lock puts the session in `app`; (a) still needs `ai` to serve for the prompt to reach it, so it is two paths. The spike settled the stack with it: only `grpclib` has a generated server (ADR 0020 §1) |
| U4, completed — which gRPC stack, and when pinned | **`grpclib`**, `openai`, `httpx2`; nothing else. Named in §17 and `lock.baseline.json` now, **pinned by exact version in PR 5**, where they are first installed | The user's "when it is first added, not before" wins over this file's earlier PR 2 row; a version written in prose today would be re-resolved by `uv` in PR 5 and drift in silence (ADR 0020 §2) |
| 2 — `ai`'s wire shape, and is `Jobs` a service? | One RPC, `Assistant.Prompt`, one stream per prompt; a prompt with the `Song`, the offered schemas, the model id and the conversation in; text, calls and `done` out; a call's result and the `Song` after it back in; arguments as JSON text. **No `Jobs`**; §13 amended | A stream is a job with progress. The messages are named so `buf breaking` compares `proto/`'s whole M3 shape once (ADR 0020 §3; trap 14) |
| 12 — how does `ai` ship and start? | **`core` spawns it as it spawns the engine**: told a command, reads one `unix:<path>` line, dials over the socket with the `tonic` already pinned; the health dot is the exit status, read. Never the project path; the key from the environment alone | The loop's host half has to be drivable from `tests/` with no window, and `tests/` already spawns `core`'s children. `core` knows a command as it knows a path (ADR 0020 §4) |
| The stale lock (found by the spike) | **A lock whose recorded pid names no process is stale and replaced**, said so; a live pid and a missing pid refuse as before. `/proc` on Linux. **PR 4**, with a `SIGKILL`ed holder as the test | Amends ADR 0012 §3: its premise stands and its rule was wider. Signal handlers would miss `SIGKILL` and a crash, which is the case the rule was about. §18.2 narrowed as U5 decided, and this is what makes the narrowed promise true (ADR 0020 §5) |
| 3 — how does the model's provenance reach the log? | Through the **proposal**, which carries `AUTHOR_MODEL`, the `prompt_id` and per call the `model_id` the response named and the `tool_call_id`; entities get the call's, the entry the turn's. No request gains an author field; the session's author stays the default; `app` **keeps** `Author::Human` | The proposal is the one place a model's calls enter, so trap 3's six sites are untouched. `model_id` is what *answered*, `prompt_id` is the SHA-256 of the prompt's text by the store's one hasher — stable, replayable, verifiable. An MCP client's model is anonymous and the log says so (ADR 0021 §2) |
| The provenance forgery (found by the spike) | **Overwrite, in `prepare`, on every path**: a new entity gets the call's provenance, an existing one keeps its own, `prepare_merge` leaves both. Not a refusal. Ids stay the caller's. **PR 4**, the silent PR — no golden is expected to move, and any byte that does is named | A comparison would refuse every previewed patch under a real clock (`created_at`) and Edit's patch on entities the person now owns. A dry run's ids are the keys a pending edit is applied by (ADR 0012 §4). Extends ADR 0006 §4 to the raw pipeline (ADR 0021 §1) |
| 4 — where does the conversation live, and what is a `prompt_id`? | **The host's**, sent whole per prompt so `ai` holds nothing between streams, persisted beside the project and outside it as JSON Lines per song id under the app data directory, read back on open. `prompt_id` is the hash of the prompt's text | Not song state; not in the `.escri` because a prompt may carry text a person would not commit. Persisted because the log's ids must name something on the machine that made them. A log can still name prompts a machine no longer has, and the history view says so (ADR 0021 §3) |
| U3, completed — what `ai.model` is | **A record, not a pin**: `lock.json`'s `ai` block written on first use and never by a tool; `provenance.model_id` per entry; the default `deepseek/deepseek-v4.1-flash`; never counted verified | §17's row amended; trap 15 (ADR 0021 §4) |
| 13 — which tools is the model offered? | **Twelve** — the op-producing ones, `apply_patch` included — and thirteen withheld each with a reason. `OFFERED` beside `IMPLEMENTED`; the schemas the descriptor's filtered by it; **`dry_run` removed** from what the model sees | Every offered call lands on the proposal, so a dry run costs the model a turn for nothing: three turns an edit become two. `apply_patch` stays because no typed tool sets a mix, deletes, renames or resizes, three models found it unprompted, and its forgery is closed (ADR 0022 §1) |
| 6 — how does the model learn a plugin's parameters? (narrowed) | **It does not**: no `list_params`, no manifest to `ai`. A track's `gain_db`/`pan` need none; a device parameter reaches the model only as the document already automates it. Deferred until `set_param` is offered | With `set_param` withheld, 2,855 ids would be ids nothing the model is offered can act on except by automating them, which is `set_param` over time with the reason it was withheld unchanged (ADR 0022 §2) |
| 8 — what does the loop retry, and where does "three" live? | **Three kinds, split once each**: a refusal fed back whole, **three refused calls per turn**; an operator error ends the turn at the host, zero retries; a provider failure retried with backoff in `ai` at the one call site, surfaced as the provider's, never fed back. A cap of twelve responses per turn. §6.1 names the third kind | A loop cannot tell a retry from a new call, and per turn is the tightest bound; the spike's worst runs were 19 and 58 refused guesses. A provider that silently dropped the song is caught by its token count. A test per kind, watched failing first (ADR 0022 §3; traps 1, 2) |
| 11 — how is the loop tested, and what does CI run? | A **scripted provider** replaying a recorded transcript; the event stream goldened in `ai/` under `unittest`; the project goldened end to end in `tests/` through the real `ai` process and `core`'s client, twice; the live run behind a variable, skipped loudly; no key in a fixture, asserted; `ai/`'s tests in `checks` from PR 5 | "Same transcript → same log" is the claim the suite can check without a key (ADR 0022 §4; traps 6, 10, 17) |
| 14 — does the re-pin tool land in M3? | **No.** The row's trigger has not fired and M3 moves no plugin pin | Said in an ADR rather than by silence (ADR 0022 §5; trap 5) |
| 5, 7 | Closed before this pass: ADR 0018 (the view, read-only, in `ai`; `get_song` withheld) and the user's `set_param` withholding | Taken as given throughout |

### What the spike found (PR 0, run 2026-09-17)

Run on this machine (WSL2, Ubuntu 24.04, x86-64) against `main` at `99f92f3`, with `escribass-mcp`
built from it, `engine/build/manifest.json` as its manifest, and every project a scratch copy of
the render fixture: `tests/determinism/render/script.json` replayed through the tool API, whose
`get_song` came back byte-identical to `tests/determinism/render/expected/song.json` (15,011
bytes). The spike's code lives outside the repository and is not merged; its numbers are here.

**One instruction set, written before any model ran.** Ten instructions a person might type, each
with the fewest mutating calls the typed tools need and whether a later call must name an id an
earlier one mints, recorded beside it: I1 tempo at the start (`set_tempo`), I2 transpose the Keys
clip an octave (`transpose`), I3 unmute and pan the Pad (`apply_patch`, no typed mix tool), I4 an
`Intro` section (`add_section`), I5 one more note in an existing clip, keeping the others
(`set_notes` with the whole set), **I6** a Surge XT `Bass` track with a one-bar clip (`add_track`,
then `add_clip` on the id it mints), **I7** a `Hook` track, a two-bar clip and −6 dB (three calls,
two of them on the new track's id), **I8** a `Chorus` section and a tempo change (two independent
calls), I9 four chords on the Pad (`add_clip` with its notes inline), and I10 "turn on the
parameter named `A Osc 1 Retrigger`" — written as the control nothing offered could satisfy,
because `set_param` takes a `ParamID` and no tool reveals one (question 6). **So the share of
instructions needing several calls is this set's, by construction**: 3 of the 9 feasible ones, 2 of
them dependent. What a model does with them is what was measured. A checker per instruction reads
the final `get_song` — the edit asked for, and no other track, clip, section, lane or tempo event
changed — and each was watched **failing on the unedited song and passing on a scripted correct
edit made through the tool API** before any model ran. Every run was told the same thing: the
server's own MCP instructions ("call it first [as a dry run], read the returned RFC 6902 patch,
then call again to apply") and one paragraph — 960 ticks per quarter, 4/4, a bar is 3,840 ticks,
MIDI 60 is middle C, make the change so it is applied. The model applied its own edits; no person
was in the loop. Fourteen tools were offered — question 13's default, with `set_param` in it — and
the other eleven withheld.

Two halves. **Half A** is Claude Code 2.1.274 as the MCP client, on the user's subscription:
`claude-opus-5[1m]`, 10 instructions × 3, and `claude-sonnet-5`, × 2, 14:43–14:53 CEST. **Half B**
is the `openai` Python SDK against OpenRouter, the fourteen tools sent as OpenAI function schemas
built from the MCP `inputSchema`s — the envelope the sidecar will send — with the whole canonical
`get_song` text in the first user message, 14:40–15:50 CEST: `deepseek/deepseek-v4-flash` × 5,
`google/gemini-3.8-flash` × 2 (I10 once), `anthropic/claude-sonnet-5` × 1 plus I6–I8 again. The
three were chosen to span OpenRouter's price list for tool-capable models on the day — $0.089,
$0.75 and $2.00 per million input tokens as listed — with two not Anthropic's: DeepSeek V4 Flash as
the cheapest credible open-weights model, served by sixteen providers and so the widest test of
routing; Gemini 3.8 Flash as a closed mid-tier model whose function-calling dialect is not
OpenAI's; Claude Sonnet 5 because the wireframes name `openrouter/anthropic/claude` and Opus 5 was
already in Half A. **Every number below is one model id, on one day, routed to whichever provider
OpenRouter picked** (trap 4), cited as evidence and never as the claim (trap 16).

**Question 9 first, because it is the one that shapes M3.** Across both halves, 116 runs of I1–I9
succeeded; **42 used more than one applied mutating call, and 28 had a call naming an entity an
earlier call in the same instruction had created** — I6 and I7 every time they succeeded, and I9
twice, when DeepSeek added the clip and then its notes with `set_notes`. **Not one successful run
composed a multi-call edit out of dry runs.** Every dependent chain applied its earlier call for
real and then named the id that call's result returned. Where a model previewed at all, it
previewed one step, applied it, and previewed the next: DeepSeek and Gemini dry-ran each step of I6
and I7 after applying the one before. **Through Claude Code, Opus 5 and Sonnet 5 skipped the dry
run entirely on I6 and I7, 10 runs of 10.** Opus 5 did so while dry-running every single-call
instruction and both of I8's independent calls, 3 runs of 3 each — it stopped previewing exactly
where a preview cannot compose. Sonnet 5 rarely dry-ran anything. Trap 8 was reached once: DeepSeek
(I9, rep 1) dry-ran `add_clip`, dry-ran `set_notes` on the clip id that preview had minted, was
refused `clip_unknown`, and recovered by applying `add_clip` for real. No model ever used the one
way a several-entity edit is already a single call — `apply_patch` with ids it mints itself
(below). And a turn can carry several calls: DeepSeek sent more than one tool call in 18 turns and
Sonnet 5 in 7 (Gemini never), most of them I8's two independent edits, dry-run together and then
applied together.

What that does to question 9: **(c) is a restriction, not a non-issue.** One mutating call per
proposal would have refused I6, I7 and I8 — a new track with anything on it, and any two edits in
one sentence — which is also the bass-line example the question itself uses. (b) has to carry the
ids a prepare mints into the next prepare, because that is exactly what every model did with the
ids a real apply returned. And the single-call half of it already works end to end: **`apply_patch`
accepts a dry run's returned patch verbatim** — its `version` ops equal what `core` computes, while
`/version` 99 is refused `version_not_writable` ("this asked for 99 where core computes 25") — and
the ids the preview showed become the real ones. One DeepSeek run (I5, rep 4) applied its edit that
way unprompted. It matters because the other way does not keep them: the same `add_track` applied
with `dry_run: false` after its dry run minted a different track id from the one the preview had
shown (`…ECTRWWD1FDJS6C9KK8` previewed, `…EFMX5FHZ7JEFSTE7VX` applied), which is `core/src/id.rs`'s
own warning — and ADR 0017 §4 applies by re-sending the call, which is safe for a drag of notes
that already exist and is not for a proposal that adds a track.

**Whether a real model drives the tool API at all: yes, all five routes,** on I1–I9.

| Route | Runs | I1–I9 succeeded | Invalid calls, I1–I9 | I10 |
|---|---|---|---|---|
| `claude-opus-5[1m]` via Claude Code | 30 | 27/27 | 0 of 81 | **3/3 succeeded** — derived the `ParamID` (below); one `param_unknown` each first |
| `claude-sonnet-5` via Claude Code | 20 | 17/18 | 3 of 45 | 0/2; said it could not map the name; one tried a `Bash` tool it was not offered |
| `deepseek/deepseek-v4-flash` via OpenRouter | 50 | 44/45 | 4 of 140 | 0/5: 58 invalid calls, 4 runs hit the 12-turn cap, 1 **false claim** |
| `google/gemini-3.8-flash` via OpenRouter | 19 | 16/18 | 2 of 62 | 0/1: 6 invalid calls in 8 turns, $0.175, then the budget guard refused its next call |
| `anthropic/claude-sonnet-5` via OpenRouter | 13 | 12/12 | 0 of 37 | 0/1: 8,192 completion tokens reasoning about the hash, `finish_reason: length`, declined to guess |

The failures that were not the control were all **valid calls doing the wrong thing**, which no
validator can see and only the checkers did: Sonnet 5 wrote I5's quarter note as 480 ticks, an
eighth; DeepSeek met I7's "volume −6 dB" with an `add_automation` lane on `gain_db` and left the
mix at 0 and said it was done. Gemini's two were cut, not wrong — one by an upstream 429 and one by
the guard. The invalid calls were few and recoverable because the refusals are precise: **eight
calls by three models asked `set_param` for the new track's `gain_db`** and got `device_unknown`
("is not an instrument or effect"), and all but one of those runs went on to `apply_patch`
`/tracks/<id>/mix/gain_db` — no typed tool sets a mix, and the models expect one (question 13).
Sonnet 5 sent `plugin_id: "Surge XT"` without a version, was told which four plugins this build
hosts, and fixed it. Gemini dropped one `0` from a 26-character id, got `clip_unknown`, read the
song and fixed it. And I10 is the loudest thing in the table. **Opus 5 computed the `ParamID`
itself** — JUCE's `String::hashCode` (`31·h + c`, masked to 31 bits) of Surge's storage name
`a_osc1_retrigger` is 1217754326, which checks — and said it "couldn't confirm that it's
specifically the retrigger switch", because the validator confirms only that an id is one of the
2,855. **DeepSeek guessed three ids that did not exist, then set 1945359057 — an id it copied from
an automation lane already in the song — and replied that it had set "A Osc 1 Retrigger" (id
1945359057)**. The call was valid, the parameter was wrong, and the summary was false. Question 6
is not "a model cannot learn a `ParamID`"; it is that without a tool one model derives it, one
declines, and one invents it and says it succeeded — and §9's diff before apply is the only thing
between that sentence and a person believing it.

**What one edit costs**, on successful I1–I9 runs, with the whole canonical `get_song` in context,
as OpenRouter billed it (`usage.cost`):

| Model, as served | First-turn prompt | Prompt tokens per edit, median | $ per edit, median (single-call / multi-call) | $ max | Seconds, median / max |
|---|---|---|---|---|---|
| DeepSeek V4 Flash — DeepInfra, Venice | 8,739–8,783 | 28,562 | 0.0009 (0.0008 / 0.0022) | 0.0049 | 16.4 / 105.2 |
| Gemini 3.8 Flash — Google | 9,133–9,181 | 31,479 | 0.0265 (0.0192 / 0.0327) | 0.1468 | 15.4 / 314.0 |
| Claude Sonnet 5 — Amazon Bedrock | 11,894–11,962 | 38,645 | 0.0847 (0.0794 / 0.1189) | 0.1654 | 14.7 / 29.9 |

A single-call edit is three model turns — the dry run, the apply, the answer — which is why a
9,000-token first turn becomes 28,000–39,000 billed prompt tokens. DeepSeek's providers cached 85%
of them automatically and Gemini's 50%; Sonnet 5 cached nothing, because nothing sent Anthropic's
`cache_control`, so its number is the uncached price of the envelope as the sidecar would send it
unless PR 8 adds the marker. The failed control costs more than a success: up to $0.0083 and 476
seconds for DeepSeek's twelve-turn loops, $0.175 for Gemini's eight turns, $0.136 for Sonnet 5's
one long answer — which is question 8's point about where "three" counts, in money. Gemini's
latency is the spread, not the median: one turn took 107 seconds and one edit 314.

Where the first turn goes, measured as prompt tokens of each text alone (DeepSeek's pinned to
DeepInfra, with a twenty-token question appended): the fourteen tool schemas are about 3,390 tokens
to DeepSeek and 1,750 to Gemini; the canonical `get_song` is 5,199 and 7,196 — its compact
key-sorted form 3,791 and 5,198. **A throwaway bar-block view** — written only to size the
comparison, **not the Libretto ADR's grammar** (ids, names, devices, mix, tempo, sections, and each
note as `pitch@bar.beat[.tick] len velocity [id]`; no provenance, versions or empty maps) — is
2,294 bytes against 15,011, and 1,060 tokens to DeepSeek and 1,596 to Gemini: a fifth of the JSON.
Run as the first message instead of the JSON, on DeepSeek, I1–I9 × 3, with `get_song` still
offered: **the model called `get_song` anyway in 25 runs of 27** (3 of 45 with the JSON in front of
it), so the first turn fell from 8,760 tokens to 4,655 and the whole edit rose, 36,297 tokens and
$0.0020 against 28,562 and $0.0009 at the median. Success was 27/27 against 44/45 and invalid calls
2.8% against 2.9% — **no measurable change in invalid calls**, on one model, from a view that was
not the ADR's. What the number says to PR 1 is narrower than "views help": a read-only summary
beside a tool that returns the full document was not trusted as the document.

**Whether tool calling survives OpenRouter's routing: the schemas did; the routing is the risk.**
No provider refused the descriptor-derived schemas — `additionalProperties: false`, maps as
`additionalProperties` schemas, no `required` — and across 453 tool calls in Half B there was no
unknown tool name, no unparseable argument string, and no camelCase field (none in Half A's 143
either; trap 11 did not fire). One argument arrived in the wrong shape: DeepSeek sent `note_clip`
as a *string* holding malformed JSON, which `core` answered as JSON-RPC `invalid_params` ("invalid
type: string …, expected struct escribass.song.v1.NoteClip") rather than a refusal, and the model
corrected it the next turn. Gemini returned `reasoning_details` on all 86 of its turns and they
went back verbatim with no error. What the router changed was elsewhere:

| Found | Consequence |
|---|---|
| **One of DeepSeek V4 Flash's providers silently discards a message whose content is a JSON document.** Pinned to OpenInference, the canonical `get_song` plus a question — "the name of the track at index 4, and its note" — reported 8 prompt tokens, answered `ok`, and cost $0.00000068; the compact JSON the same. The bar-block text with the same question: 1,060 tokens, answered `Keys`. Pinned to DeepInfra: 5,199 and 3,791 tokens, answered `Keys 48`. Unpinned, a request *without* tools was routed there; every tool-calling turn went to DeepInfra, Venice or once Novita | Nothing in the response says the song was dropped except the token count. The sidecar pins its providers (`provider.order` or `only`, `allow_fallbacks: false`) or compares `prompt_tokens` against what it sent; "OpenRouter" is not one backend, and price-based routing changed with the request's shape |
| Claude Sonnet 5 was served by **Amazon Bedrock on all 44 turns**, never Anthropic's own API; Gemini by `Google` on all 86 | A model id names weights, not a service — the `provenance.model_id` U3 records says nothing of who ran them |
| Gemini returned an **upstream 429** mid-run ("temporarily rate-limited upstream") | Question 8's third kind, seen live. The SDK was set to `max_retries=0`, so nothing retried it unseen; a sidecar left on the default retries it twice before the loop knows |
| `usage.cost` is in every response without asking for it, beside `cost_details.upstream_inference_cost` and `is_byok`, and summed to within $0.00000012 of the key's own counter over 545 priced calls. `/api/v1/generation?id=` returned 404 three seconds after a call | The response is the ledger; there is no need to query it afterwards |

**What `betterproto2-compiler` 0.10.1 generates: both stacks, chosen by option, and a server only
for grpclib.** Two plugin options decide it (`settings.py`, `plugin/parser.py`):
`client_generation` — `none`, `sync` (the default), `async`, and three combinations — and
`server_generation`, `none` (the default) or `async`. With the one option `schema/buf.gen.yaml`
passes today, `pydantic_dataclasses`, `song_tools.proto` becomes `SongToolsStub(channel:
grpc.Channel)` calling `channel.unary_unary(...)` under `import grpc` — **a synchronous grpcio
client, and no server**. With `client_generation=async` and `server_generation=async` it becomes
`SongToolsStub(betterproto2_grpclib.ServiceStub)` and
`SongToolsBase(betterproto2_grpclib.ServiceBase)` under `import grpclib` — **an async grpclib
client and a grpclib server**. There is no grpcio server option. The runtime declares both as
extras, `grpcio>=1.72.1` and `grpclib>=0.4.8`. Five round trips were run, with grpcio 1.84.0 and
grpclib 0.4.9 in a scratch environment: the generated grpclib server on a Unix socket; the
generated grpclib client to it over the socket; the generated grpcio client to the same grpclib
server over `unix:<path>`; and each client against the real tonic `escribass-grpc` (`get_song`, and
a dry-run `add_section` answered valid) — over TCP, because that binary takes a `SocketAddr` and
has no Unix listener. **So the stack follows question 1.** Under its default (b) `ai` serves, and
only grpclib has a generated server; under (a) `ai` only dials and either works. grpcio with a
server means registering handlers by hand, or `grpcio-tools`' own `_pb2` classes — a second set of
Python model types beside the Pydantic ones.

**How many of Surge XT's 2,855 `ParamID`s reach an RNG nothing can seed: 194 by the rule below, 24
to 2,283 by the rules around it — and no denylist of `ParamID`s can be the whole answer.** Read at
`f7b97c6`. Every `ParamID` was mapped by hashing Surge's JUCE parameter ids (its storage names,
`SurgeSynthProcessor.cpp:1689`) as dumped from the engine's own compiled Surge libraries — all
2,855, no collisions: 766 Surge parameters, 8 macros, a bypass, and **2,080 JUCE MIDI-CC proxy
parameters**. Every RNG site was read in context, and a scratch harness linking those libraries
rendered each setting in two fresh processes more than a second apart — 278 settings, each as two
such pairs, every verdict agreeing — driving `SurgeSynthProcessor` directly, not through Tracktion
or the VST3 wrapper. Its base is Surge's constructor state (no data directory, so not "Init Saw"),
and "the fixture" is that plus `A Osc 1 Retrigger` = 1, the one `set_param`
`tests/renders/surge_xt` makes. The unseedable sources are three: `storage->rand*` from a
`minstd_rand` seeded by `system_clock` (§8's note); C `rand()`, which `SurgeSynthesizer.cpp:86`
seeds with `srand(time(nullptr))`; and `std::random_device`.

| Rule | ParamIDs |
|---|---|
| **Headline**: a value that, with `Instrument.state` empty and only `params` or automation set, selects or switches on an unseedable RNG consumer that reaches the render — type and mode selectors, mute, solo, retrigger and routing switches, and amounts whose zero is that RNG's off (Osc Drift, Extra Noise, Chaos, Knock); plain gains and mixes not counted | **194**: 23 per scene × 2, 4 global, 16 FX types, 128 FX-slot parameters |
| One parameter changed from the fixture opens a path, **each confirmed by rendering** | **24**: `a_osc1_retrigger`, `a_osc1_type`, `a_drift`, `a_fm_switch`, `scenemode`, `scene_active`, mute and solo of oscillators 2 and 3, both ring modulators and the noise source, and FX Type (Tape) in the 8 A and Global slots |
| Reaches only through a modulation routing, which lives in `state` | 72 more (LFO Type Noise/S&H/MSEG, Trigger Mode Random, Deform below 0 on Envelope, × 24 LFOs), plus 32 amounts |
| Its **default** value is the one that reaches | Strictly 1, `A Osc 1 Retrigger`; 15 have their default on the open side (6 retriggers, 6 oscillator types, 2 oscillator-1 mutes, FX Chain Bypass) |
| Meaning depends on another selector | Oscillator parameters: 6 of 42. FX-slot parameters: 128 of 192 |
| The 2,080 MIDI-CC proxies | 0 through `params`; 224 are wired to modulation sources by default and need a routing |
| Constant-seeded only (repeatable in a fresh single-threaded process, order-dependent) | 2 (the waveshaper's Fuzz shapes); Twist engines 7–15, Nimbus and Bonsai are too, already counted elsewhere |
| Range | 24 · 60 (headline without selector-dependent) · **194** · 275 (plus gains and sends that can silence a path; a floor) · 379 (plus routing-only) · 2,283 (plus MIDI CCs through routing or learn) |

Four facts the count cannot carry and question 7 has to. **A default reaches the RNG**, so a rule
that only refuses values cannot close it: it would have to *require* `Retrigger` on every
oscillator that is heard. **A type change resets its dependent parameters at the first block** —
oscillator and FX-slot values written in the same `params` map as the type are overwritten
(rendered), so those values reach the engine only through automation lanes or `state`. **FX Type
Tape reaches `std::random_device` at its default settings** wherever its slot receives audio. And
**with arbitrary `state` every parameter can matter**, since `state` carries the routings — and
`apply_patch` writes `state` like any other field (a dry run replacing the Lead's with three bytes
was valid). Two cautions for any golden that compares renders: C `rand()` is seeded in *whole
seconds*, so two renders in the same second agreed 3 times of 3 with Drift on; and Alias's noise
has an 8-bit seed, so one A/B pair in five agreed by chance. Two sites a grep for the RNG finds are
dead code — the Vocoder's `rand_pm1` sits inside a `/* */` block and `Reverb1.h:403` is commented
out. Confidence: the 24, the defaults and the MIDI-CC count rest on source and renders and are
high; the 194 is a per-`ParamID` classification with two judgment calls (which Airwindows amounts
are RNG-specific; Split Point) and is medium-high; the Airwindows gates beyond their defaults were
not rendered. Not examined: Init Saw as a base, user configuration, the contents of `state`,
LuaJIT's own PRNG, whether anything calls the VST3 SDK's `FUID::generate` (which reseeds `rand()`
from a pointer), and the real engine path. **194 is a validator rule over values and combinations,
with a required value and a `state` escape beside it — not a PR-sized list**, which is the case
question 7 wrote (c) for.

| Found beside the six questions | Consequence |
|---|---|
| **Every Claude Code session left `.escri/lock` behind** — 50 runs of 50, naming a process that no longer existed, so the next open was refused `project_locked`. Reproduced without Claude Code: `escribass-mcp` removes the lock when stdin closes (exit 0) and leaves it on SIGTERM, SIGINT and SIGKILL — nothing handles a signal, so `ProjectLock`'s `Drop` never runs — and Claude Code ends its stdio servers by signal | §18.2 Stage 1's "any MCP client can drive a project" holds for one session per project; after it, a person deletes a file by hand, which ADR 0012 §3 says nothing does for them. U5 narrowed the promise to "with the window closed", and this narrows it again. `core`'s to fix, not M3's to route around |
| **`apply_patch` lets a caller mint entity ids and write entity `provenance`.** One `apply_patch` adding a track, its instrument and a clip on it, with ids the caller chose, is valid once each entity carries a `provenance`; a section added with `AUTHOR_HUMAN` and `created_at` 1999 was stored exactly so, while its log entry said `AUTHOR_MODEL` at the real time. A caller's `version: 1` came back 0 | ADR 0006 §4's "a caller cannot set them" holds for the typed tools and not for the raw pipeline. Trap 3's six sites have a seventh: a model can stamp an entity human through the tool question 10 hands a person for Edit. It is also the one existing way a several-entity edit is one call, and no model used it |
| The tool schemas never say **960 PPQ**; `song.proto` does, in a comment the descriptor carries to no tool | Every run here was told in the prompt. An MCP client that is not told has to infer it from the ticks |
| Claude Code hands its model the MCP **`structuredContent`** — compact, key-sorted JSON, 9,526 bytes for the fixture — not the canonical `text` (15,011) | What a client's model reads is the client's choice; the view question is not only the sidecar's |
| **`--bare` cannot use a Claude subscription** — its help says OAuth is never read, only `ANTHROPIC_API_KEY` or an `apiKeyHelper` — and `--safe-mode` keeps OAuth but drops `--mcp-config` servers. What isolated the runs was `--setting-sources "" --disable-slash-commands --strict-mcp-config --tools ""` with `--permission-mode dontAsk` and an allowed/disallowed tool pair; each run's init reported `apiKeySource: none`, no plugins, no skills, no hook events, and the fourteen tools | So a later measurement through Claude Code does not bill an API key by accident |
| The `openai` SDK today, **3.14.1, carries `httpx2` 2.13.0** — a differently named package — with `httpcore2`, `jiter`, `anyio`, `truststore`, `sniffio`, `h11` and `idna`; and it reads `OPENAI_API_KEY` when no key is passed, which is also set on this machine | U4 approved "the `openai` Python SDK … with the `httpx` it carries"; PR 2's pin names what it actually carries. The sidecar passes the key and `base_url` explicitly, or another provider's key goes to OpenRouter |

**What Half A could and could not say.** It answered capability, questions 1 and 9, for two Claude
models through a real MCP client — the §18.2 path, with the client's own prompt, its own choice of
`structuredContent`, and its own dry-run habits. It could not answer cost: subscription usage is
not a per-token price, so no dollar figure is given for it, and its token counts include Claude
Code's own system prompt. It could not see routing, because Claude Code talks to Anthropic
directly. And it is not the sidecar's envelope: its tool schemas reach the model through Claude
Code's conversion, not as the OpenAI function schemas Half B sent.

**What it spent.** Half B was capped at **$3.00** for every model combined by a guard that, before
each request, booked a worst case — the request body's bytes plus 3,000 as an upper bound on input
tokens, times the highest input-side price any tool-capable endpoint of that model lists, plus
twice `max_tokens` at the highest output-side price — under a file lock, and refused the call if
spend plus that would pass the cap or the model's allocation. It was watched refusing before it was
trusted: against a client that fails if reached, at a tiny cap, a tiny allocation, one micro-dollar
under the worst case, a missing `max_tokens`, spend-so-far plus worst case, and a second caller
while the first was in flight; then with the real client at a $0.0001 cap, where the client object
was never constructed. In service it refused twice, both Gemini's allocation. **Total spent:
$2.42112049** by the sum of `usage.cost` over 545 priced calls — DeepSeek $0.16915284, Gemini
$0.86625165, Sonnet 5 $1.38571600 — and **$2.421120374 by OpenRouter's own counter for the key**,
read before the first call and after the last. No request's reported cost exceeded 32% of its
booked worst case, and no reported prompt exceeded its bound. The key was read from the environment
only, and is in no file the spike wrote.

**Still open after the spike.** Sonnet 5 through OpenRouter ran once per instruction, and Gemini's
second repeat lost I6 to a 429 and I9 to the guard. Nothing measured next month's weights under the
same names. Temperature was each model's default, and no run used Anthropic's `cache_control`. The
bar-block result is one model and a view that is not PR 1's. The Surge count is a harness verdict,
not an engine render, and says nothing of Dexed's waveform 5 or sfizz's `*_random`.

#### DeepSeek V4.1 Flash, measured before it became the default (run 2026-09-17)

The user chose `deepseek/deepseek-v4.1-flash` as the default, on condition that it was measured
first: the run above measured `deepseek/deepseek-v4-flash`, a different release, and none of its
numbers transfer. It ran the **same fixed instruction set, checkers, turn cap and repetitions**,
changing only the model id — every instruction five times, nothing cut. Run 16:18–17:17 CEST;
V4 Flash's figures are the run above's, 14:40–15:50 CEST the same day, reproduced exactly by the
comparison script.

| | V4 Flash | V4.1 Flash |
|---|---|---|
| I1–I9 success | 44/45 | **45/45** |
| I1–I9 invalid calls | 4/140 (2.9%) | 8/139 (5.8%) — every one a refusal it recovered from; none malformed |
| I10 (`set_param`) success | 0/5 | 0/5 |
| Cost per edit, median | $0.0009 | **$0.0014** (about 1.5×) |
| Seconds per edit, median | 16.4 | 4.8 — but see routing |
| Several applied calls in one instruction | 16/44 | 15/45 |

- **It repeated V4 Flash's most serious mistake.** On I10, after 19 refused guesses, it copied
  `1945359057` from an automation lane already in the song — **A Filter 1 Cutoff** — set it to
  1.0, and reported the task done. A valid call that did the wrong thing and said otherwise is
  exactly what withholding `set_param` (question 7) removes. Four of its five I10 runs ended with
  no reply at all, having spent the 8,192-token output limit reasoning, three of them hashing
  parameter names by hand.
- **It reasoned about being tested**, calling I10 "a benchmark task" in at least four of five
  runs. Weigh I10 with that in mind.
- **The headline price is not the price paid.** $0.15/$0.60 per million is listed by only three of
  nineteen tool-capable endpoints. Routing sent 57% of spend to Together at $0.30/$1.20, and
  DeepSeek's own endpoint doubles at weekday peak hours. Latency followed the provider, not the
  model: a median of 37 s on DeepInfra against 3.75 s on Together. **Cost and latency are
  properties of routing**, so M3's default is a model *and* a provider policy.
- **It hit the dry-run id trap live.** Twice it applied `add_track` and, in the same turn,
  dry-ran `add_clip` on the id an earlier dry run had shown; the apply had minted a different id,
  so the dry run was refused `track_unknown`. It recovered the next turn. No run built a
  multi-call edit out of dry runs.
- **OpenInference does not serve V4.1 Flash** (checked 16:14 and 17:17), so the provider found
  above to silently drop a message whose content is JSON is not reachable for the default model
  today. That is a fact about one day's routing, not a guarantee.

**Spend: $0.227909182**, cumulative **$2.649029556 of the user's $3.00 cap**, confirmed against
OpenRouter's own counter for the key rather than taken from the ledger. The budget guard was shown
refusing before it spent. One ledger correction was made mid-run and checked afterwards: the
spike's guard had held an uncharged, rate-limited Gemini call at its $0.1777 worst case as though
it had been spent, and an appended entry released it — bringing the ledger into agreement with
OpenRouter's counter to within $0.00000012. **A guard that counts a failed call's reservation as
spend is conservative and wrong**, and it nearly forced cutting the hardest instructions; M3's
real budget accounting settles a reservation when a call fails, not only when it succeeds.

### The open questions — for the user

`CLAUDE.md` line 4: "Sections marked [OPEN] are not yours to decide: stop and ask." Five
questions below are a person's for that reason or for CLAUDE.md #4's. **Decided 2026-09-17 — see "Decisions taken" above**; the questions are kept as the record of what was weighed. An agent may lay out the
options and their costs, which is what each row does, and may not take one.

| # | Question | What bears on it |
|---|---|---|
| U1 | **Are §6's analysis features and symbolic generation v1 scope at all?** `[OPEN]`, ADR 0003 "Still unplaced" | §6.2 (melody, harmony, drums, variation from MIDI-domain models) and §6.3 (key, chord and structure detection, tempo estimation, Demucs-class stem separation) are orchestrator responsibilities, so M3 is the only milestone that could carry them — and M3 as §16 specifies it is the loop and the panel. Taking them in makes M3 the widest milestone after M4 and brings model weights, a second inference stack and the `[OPEN]` item below with them. Leaving them out leaves the Libretto harmony axis (item 4 above) without a key to compute against, which the ADR then says. **This plan assumes "not in M3" for sizing and says so in every row it affects; the assumption is the user's to confirm or reverse** |
| U2 | **Symbolic model choice for v1 melody and drum generation.** `[OPEN]`, §15 | Only reachable if U1 says yes. The landscape's candidates are NotaGen (IJCAI 2025, open weights, ABC/MusicXML/MIDI out), Anticipatory Music Transformer, and Magenta RealTime; each is a weight file that would be content-hashed and pinned (§7.3, §17) and a Python inference dependency under CLAUDE.md #4. Not an agent's, and not this plan's to narrow |
| U3 | **Which model, and what does its pin mean?** `lock.baseline.json` has `"provider": "openrouter"`, `"model": null` | Three parts, all a person's. *Which id* is the default — it is a cost per call, and the spike (PR 0) will report what one edit costs in tokens and money for two or three candidates rather than one. *What "pinned" means*: §17's "model ids pinned per project in `lock.json` under `ai.model`" cannot mean what every other row means, because a hosted model changes under its name and nothing can verify one; the honest record is the name in `lock.json` as the *choice* and the name in every entry's `provenance.model_id` as *what actually wrote it* (question 3, trap 15). *The key*: `OPENROUTER_API_KEY` reaches the sidecar from the environment and nowhere else — never a flag (`ps` shows flags), never `lock.json` (committed), never a recorded transcript (trap 10) — and the plan assumes that and asks |
| U4 | **Every new dependency, for sign-off before any is installed** (CLAUDE.md #4) | The sidecar needs, at minimum: **a Python gRPC stack** — `grpcio` with `grpcio-tools`, or `grpclib`, whichever `betterproto2-compiler` 0.10.1 (already pinned) generates service stubs for; ADR 0006 §7 assumed `grpclib`, and PR 0 checks which the pinned compiler actually emits before anyone chooses. **An HTTP client for OpenRouter** — the `openai` Python SDK is the literal "OpenAI-compatible client" §6 names and carries `httpx` with it; plain `httpx` is smaller and means writing the tool-calling envelope by hand. **Nothing else** is proposed: `pydantic` is pinned, `unittest` is stdlib (ADR 0016 §3's reason, one language over), and no agent framework, no prompt library, no tokenizer, no vector store — each is a decision a Python LLM project usually makes without noticing, and each is refused here for ADR 0016 §3's reason with its cost written out in the ADR. On the Rust side nothing new: `tonic` already serves `SongTools`, and `tokio`'s `net` feature is already on. Listed here for a person; none is added by this plan |
| U5 | **May an external MCP client and the window edit one project at once?** | Not `[OPEN]` in §15, but it is §18's promise and §18 is "[MUST read before roadmap changes]". §18.2 Stage 1 sells "Claude/Cursor/any MCP client can drive a project immediately", ADR 0012 §3's context says §18.2 "actively sells leaving an MCP client pointed at the same directory" — and ADR 0012 §3's decision, `.escri/lock`, refuses a second process on the directory while the window has it. Both are correct and they contradict each other while a window is open. M3 is where the *bundled* model reaches the session (question 1), and the same mechanism is what an external client would use — so the option exists to have `app` serve MCP on the same session, and the option exists to leave §18.2 true only with the window closed. Which the product wants is a person's; the plan assumes the second and says so in "What M3 will not claim" |

### The open questions — an agent can propose an answer

Fourteen. Each changes what gets built rather than how, which is the test M1 and M2 used for
what had to be answered before code. A **default** is stated where this plan has one; the
question is open regardless. **All fourteen are now decided** — 5 by ADR 0018, 7 by the user,
the other twelve by ADRs 0019–0022 on 2026-09-21 (see "Decisions taken, 2026-09-21"); the table
is kept as the record of what was weighed, defaults included, and where an answer departs from
its default the ADR says why.

| # | Question | Options, and what each costs |
|---|---|---|
| 1 | **How does `ai` reach the session — who listens, and what crosses?** | The lock decides more than it looks. `ai` cannot open the project itself: `.escri/lock` refuses a second process (ADR 0012 §3), and `escribass-grpc` is a second process. So the sidecar reaches **`app`'s session**, and the question is the direction. (a) `app` serves `SongTools` over a Unix socket it names, and `ai` is an ordinary generated client — §3's sentence read literally, Python codegen for `SongTools` (ADR 0006 §7), a listening socket in `app` that ADR 0012 §1 permitted only as "not a network port", and a second `Session` holder beside the Tauri command whose author is not the window's (question 3). (b) `ai` serves, `app` dials, and the model's tool calls come **back** over a bidirectional stream as `{name, args}` — which is exactly the envelope `call(session, name, args)` already takes, so the host becomes the third envelope around the one dispatch (ADR 0012 §1) and Python holds no `SongTools` client at all; it needs the tool *schemas*, which the host hands it once, and the generated `Song` for reading. `app` opens nothing and supervises `ai` the way it supervises the engine — spawn, read one `unix:<path>` line, dial (ADR 0013 §3). (c) Both — a `SongTools` server in `app` for external clients (U5) and a stream for the bundled model — is two paths for one thing. **Default: (b)**, because it puts the model's calls on the dispatch the determinism suite already drives, gives the host the author and the ids, and keeps ADR 0012 §1 true without a reading. Its cost is that `proto/`'s Python target generates the *new* service and not `SongTools`, which is a narrower thing than ADR 0006 §7 promised and should be said |
| 2 | **What is `ai`'s wire shape, and is `Jobs` a service?** | Whatever question 1 picks, `proto/` gains a service and `buf breaking` guards it on pull requests only — so it lands **once, early, whole** (PR 3; M2 trap 12). What crosses: a prompt in; a stream of events out — model text, a tool call proposed, its dry-run result, a proposal ready, an error; cancellation, which a stream's close gives for free as `Preview`'s does (ADR 0013 §2). §6 says "long tasks are jobs with progress" and §13 names a `Jobs` service that has never existed. A streaming RPC *is* a job with progress; a `Jobs` service beside it would be a second way to ask about the same thing. **Default: no `Jobs` service; §13's line is amended to what exists**, and the ADR says why a stream is enough until something that is not a conversation needs a job |
| 3 | **How does the model's provenance reach the log?** | `Author` is **per session**: `Session::new(.., author)`, `--author human\|model` on both binaries (the MCP one defaults to `model`, the gRPC one to `human`, and the determinism suite passes both explicitly), and `app` hardcodes `Author::Human`. One session per project (ADR 0012 §3) shared by the window and the model means the model's edits are stamped `HUMAN` unless the author moves. And `model_id`, `prompt_id`, `tool_call_id` are `None` at every site that builds a `Provenance`. Options: (a) author on **every request message** — an additive field on each of the twenty-odd requests that can record an entry, on the wire for every carrier, and a caller that lies about being human; (b) a second session on the project — refused by the write ordering ADR 0004 depends on and by ADR 0012 §3; (c) **author is a parameter of the call, not of the session**: `Session::call` takes a `Provenance` with the call, the Tauri command passes the window's, the sidecar's path passes `MODEL` with the three ids, and the two binaries keep their flag as the default for callers that have no other way to say. On the wire it appears only where `ai`'s calls enter, which under question 1(b) is inside the host. **Default: (c)**. Either way the determinism goldens are touched — every `provenance` in five scripts — so it is a silent PR (PR 4) whose every changed byte is named. Trap 3: the author is set at three sites today and a change that misses one keeps stamping `HUMAN` |
| 4 | **Where does the conversation live, and what is a `prompt_id`?** | A `prompt_id` in a log entry has to name something, or it is a dangling reference in what §5 calls the audit trail. The conversation is **not song state** — CLAUDE.md #1 is about the song — so it does not go in `song.json` and does not go through the log. Options: (a) not persisted: the panel is per window session, `prompt_id` names a thing that is gone when the window closes, and the audit trail records that a prompt existed; (b) persisted **in** the `.escri` — a fifth artefact beside §10's four, which is a §10 change and an ADR, and a thing branches and merges know nothing about; (c) persisted beside the project and outside it, keyed by project, with `prompt_id` a **content hash** of the prompt text so it is stable, replayable and says nothing a reader cannot verify. A prompt may also contain text a person would not commit, and `patches/` is committed. **Default: (c), with the hash**, and an honest sentence in the ADR that a log can then name prompts a machine no longer has |
| 5 | **Read-only view, or a write grammar — and where is it computed?** | This is the Libretto ADR's items 2 and 3, listed here so the table is complete. **Default: the model reads the grammar and writes through the typed tools in ticks**, so no second way of writing notes exists; **computed in `ai`**, goldened under `unittest` like the TypeScript projection is under `node:test`, with a note that an MCP client does not get it — moving it to `core` as a read tool is additive if that ever matters |
| 6 | **How does the model learn a plugin's parameters?** | `set_param` takes a `ParamID` (ADR 0010 §4) and the model has no way to learn one: ADR 0014 §1 refused a `get_manifest` tool "because it would hand the AI a plugin catalogue as a side effect of drawing a form, which is a decision about §6's surface" — and this is the decision. Without one the model calls `set_param` with a display name and gets `param_unknown` for ever, and it is *by design* that the frontend never predicts a refusal (ADR 0017 §3), so nothing else will tell it. Options: (a) a **read tool**, `list_params(device_id)` say, returning the plugin's `ParamID`s with their display names — a `proto/` change (PR 3), and MCP clients get it too; (b) the manifest handed to `ai` at launch by the host, as the form gets it by a second Tauri command — no tool, no proto change, and an external client gets nothing; (c) nothing, and `set_param` is not among the tools the model is offered in M3. Two facts bear on it: display names are **not unique** (Surge XT's 2855 carry 2679 distinct), so a tool that takes a name is a tool that guesses; and the denylist of question 7 is precisely a fact about parameters that such a tool is the natural place to surface. **Default: (a)**, keyed by `ParamID`, names beside |
| 7 | **What does M3 do about the §2.2 randomness row before its loop calls `set_param`?** **Decided by the user 2026-09-17: `set_param` is withheld from the model** — see "Decisions taken" | The row is due now and the blocker is unchanged: "the source audit that produces the `ParamID` denylist is the work". Surge XT's `rand_pm1` callers, Dexed's LFO waveform 5, sfizz's `*_random` opcodes — §8 names them in English, and the manifest cannot produce them because they are facts about a plugin's *source*, not what a VST3 reports. Options: (a) **the audit, whole, for all three**, as its own PR: a hand-written denylist beside the manifest, versioned by the plugin commit it was read against, and a validator rule (`param_unseedable`, an ADR since it is a rule) that refuses the value for **every** author — a rule that refused only a model would make §5's "identically" false (trap 18); (b) the audit for Surge XT alone, which is where the 2855 are, with Dexed's one waveform and sfizz's opcodes handled as they are today; (c) defer again, with `set_param` withheld from the model's tools until it lands, and "What M3 will not claim" saying so. "Half a denylist is worse than none, because it looks complete" (ADR 0014 §3) rules out shipping (a) partially. **Default: (a) as PR 7, before the loop PR**, sized by PR 0's count of how many of the 2855 reach an RNG path, and if that count makes it larger than a PR, (c) with the withholding — never a list that is missing entries and does not say so |
| 8 | **What does the retry loop retry, what does it send back, and where does "three" live?** | ADR 0006 §2 gives two kinds: `valid = false` (retry, differently) and `Err` (no retry helps). A hosted model adds a **third the ADR did not have to name**: the provider's own failure — a 429, a 5xx, a timeout, a malformed tool call the provider emitted — which is neither the caller's to fix by calling differently nor an operator's project error. §6's "on validation error, feed the error back and retry (max 3)" says nothing about it. Options for the third kind: retry with backoff a bounded number of times and then surface it as the panel's own error, distinct from both of ADR 0006 §2's; or treat it as an `Err` and stop. What goes back on `valid = false`: **every** `Violation`, `path` `rule` `message`, as `ToolResult` already says ("a model fixing one problem at a time wastes them"). Where "three" counts: per tool call, per proposal, or per prompt — the options differ by an order of magnitude in what a prompt can cost. **Default: three per tool call, a named third category with its own bounded retry, and a test per category that was watched failing first** (trap 1). M1 PR 13's finding is the reason this is a question and not a detail: an engine that exited 0 having written nothing was `valid: true`, and the loop's equivalent is a provider error reported as a refusal, spent three times |
| 9 | **How is a proposal of several tool calls previewed, when dry runs do not compose?** | The central technical question, and ADR 0017 did not have to face it: one gesture is one tool. A model asked to "add a bass line" will call `add_track`, then `add_clip` on the id the first would mint, then `set_notes` on the id the second would mint — and **a dry run writes nothing, so the second call's track does not exist**. The ids are real (ADR 0012 §4) and the entity is not. Options: (a) **the model works on a branch**: `create_branch`, apply each call for real with `Author::Model`, and Apply is `merge_branch`, Reject is `delete_branch` — every step validated and in the log, the audit trail literal, a rejected proposal a deleted branch whose entries stay (ADR 0001 §2), the panel's diff the merge's dry run. Cost: `switch_branch` moves **the session's one HEAD** under the window, so the person and the model cannot be on different branches of one session; either the window follows the proposal branch (which is what a dashed pending clip *is*), or a per-caller HEAD, which is a `refs.json` change and an ADR. And it fires trap 9 with every step. (b) **a proposal is a fork**: `prepare` is pure and `ids.fork()` exists, so a sequence of prepares chained on a forked in-memory document produces one combined patch, `diff(current, final)`, committed as **one entry** on approval — ADR 0017 §1 transferred whole, and no branch machinery. Cost: the entry's `tool` is `apply_patch` and the model's individual calls are not in the log unless the entry gains a way to say them; and it is new `core` machinery on the same seam ADR 0006 §3 says has no second implementation. (c) one mutating call per proposal — no new machinery and no bass line. **Default: (b), with the entry recording the calls it was composed from** — ADR 0006 §3's argument that `dry_run` *is* `prepare` extends to a chain of them and to nothing else; PR 0 measures how often a real instruction needs more than one call, which is what says whether (c) is a restriction or a non-issue |
| 10 | **What may the panel apply, and what may a person edit?** | §9: "users can apply, reject, or edit". The wireframes draw Apply, Reject, Edit and *⌘Z undoes*. Apply is the proposal with `dry_run: false` (whatever question 9 makes that). Edit is a person changing the RFC 6902 text and applying it through `apply_patch` — which is why that tool exists (`song_tools.proto`) — and it is the one control in the application where a person writes a patch by hand, so it needs the refusal path ADR 0017 §3 gives a drag. Two pending things on one session — the model's proposal and a person's held drag — are settled by §4.3's `version` check as ADR 0012 §4 already settles one, and what M3 owes is the same sentence: the refusal names what moved. **Default: all three controls, with Edit landing last and only after Apply and Reject are driven against a real proposal** |
| 11 | **How is the loop tested without a model, and what does CI run?** | No key in CI, ever, and no money spent by a test. `core/tests/engine.rs` drives `render_export` against a fake engine that is a shell script; the loop gets the same: a **scripted provider** replaying a recorded transcript of model turns, so the sidecar is driven twice and the project it produces is compared with a committed golden — the determinism suite's shape, one process over. A **live run** exists behind an environment variable and is `#[ignore]`d loudly, printing why, as the device test is (M2 trap 13). And the Python tests need a CI step: the `checks` job already installs `uv` and runs `schema/`'s, and its path gate would let an `ai/` change through — to a job with no step that runs `ai/`'s tests (trap 17). **Default: as stated**, plus `ai/`'s tests in the `checks` job in PR 5, the PR that creates the directory |
| 12 | **How does `ai` ship and start?** | Who spawns it — the Tauri host (as it holds the session) or `core`'s session (as it spawns the engine); how Python is found — a `uv`-managed environment from `ai/uv.lock`, run from a build tree in M3 as `app` is, with packaging M5's; what it is told at launch — never the project path (§5: "agents never read or write the project file directly"), the socket, the key from the environment, and the schemas under question 1(b); what the window shows — the wireframes' three health dots, and "the UI keeps working when it does" die. And the same class of defect `Preview::drop` has (Known gaps): a sidecar that dies after its last message exits into nothing. **Default: the host spawns it, because the session is `core`'s and `core` should know nothing about a Python interpreter**; the dot is the process's exit status, read |
| 13 | **Which tools is the model offered, and are any withheld?** | Twenty-five are implemented. Some are not a model's to call from a panel: `render_export` writes a file at a path the model chose; `render_preview` starts an engine and the model cannot hear it (CLAUDE.md #6); the four branch tools move the session's HEAD under the window (question 9); `undo` and `redo` reverse a person's approved change; `add_asset` takes bytes the model does not have. Offering a tool "spends a model's turn on a call that can only fail" (`core/AGENTS.md` on `IMPLEMENTED`). **Default: the model is offered the tools that produce ops on the document and nothing that leaves it or moves HEAD**, with `set_param` conditional on question 7, and the list is data the schemas are filtered by rather than a second hand-written surface |
| 14 | **Does the re-pin tool land in M3?** | ADR 0010's Consequences promised M2 "a re-pin tool and the UI that makes `lock_mismatch` recoverable without a text editor", M2 built neither, and the ledger row's trigger is "the first `lock_mismatch` a person meets, which needs a plugin pin to have moved". M3 moves no plugin pin. And `lock_mismatch` is raised at **open**, which a model never does — so "an AI will hit it too" is true of an MCP client opening a project and not of the bundled loop. A re-pin is a `lock.json` write outside the patch log, which is the objection the ledger row already records. **Default: not M3's; the row's trigger stands and has not fired**, and the plan says so rather than letting a second milestone's silence look like a second promise |

### PRs

| # | Branch | Adds |
|---|---|---|
| 0 | `m3.0-spike` (**never merged**) | The measurements the ADRs cannot honestly be written without, each a number with a date and a model name beside it. **Whether a real model drives the tool API at all** through the descriptor-generated schemas, over OpenRouter, for two or three candidate ids: calls per instruction, invalid-call rate, and whether structured tool calling survives the routing. **What one edit costs** — tokens, seconds, money — with the full `get_song` (15 KB for the render fixture, 5 KB for `every_tool`'s) against a bar-block view of the same song, which is the number the Libretto ADR cites. **How often a real instruction needs more than one mutating call** (question 9). **What `betterproto2-compiler` 0.10.1 actually generates** for a service — `grpclib` or `grpcio` stubs — on a Unix socket, before U4 names a dependency. And **how many of Surge XT's 2855 `ParamID`s reach an RNG path**, a count that sizes PR 7 and decides between question 7's (a) and (c). M2's spike never took its measurement and PR 10 had to; this one records its numbers in this file, in the ADRs' "measured" sentences, before PR 1 is written. **Run 2026-09-17**; its numbers are in "What the spike found" |
| 1 | `m3.1-libretto-adr` | **ADR 0018 alone**: the grammar, its direction, where it lives, the six axes and what each is made of, that they are pure, and what was read. Its §15 row, and §18.2's "record this in an ADR before M3" satisfied. No code, and no other ADR, because everything after it is downstream of it (ADR 0003 §6) |
| 2 | `m3.2-adrs` | The ADRs the fourteen questions resolve into, their §15 rows, and the §17 changes U3 and U4 settle with `lock.baseline.json` mirroring them — ~~the Python packages by exact version~~ **the Python packages by name, pinned by exact version in PR 5 where they are first installed** (this row contradicted the user's U4 decision, recorded below it, and the later, user-confirmed decision wins — ADR 0020 §2), `ai.model` as what it is; §13's `Jobs` line amended or honoured (question 2); §6.1 gaining the sentence that names the third failure kind (question 8). **No code**. The user's five questions are answered here or the rows that need them wait. **Done 2026-09-21**: ADRs 0019–0022, the two defects the spike found decided with PR 4 named for both, and "Decisions taken, 2026-09-21" |
| 3 | `m3.3-proto-py` | `proto/`'s **whole M3 shape in one change**: ~~the `ai` service in whichever direction question 1 picks, provenance where question 3 puts it on the wire, `list_params` if question 6 says so~~ **`proto/assistant.proto` — `Assistant.Prompt`, one bidirectional stream, the messages ADR 0020 §3 names; no provenance on the wire (question 3 put it in the proposal, ADR 0021 §2) and no `list_params` (ADR 0022 §2)** — so `buf breaking` compares it once against a `main` that has not moved (M2 trap 12). And **Python codegen for `proto/`** (ADR 0006 §7), narrowed to what question 1 needs — `server_generation=async`, `client_generation=none`, since `ai` serves and dials nothing — with its entry under `tool_api` in `lock.baseline.json` as TypeScript's is. **Done 2026-09-22**: nine messages and one service, `buf breaking` clean because a new file and a new service are additive; ADR 0020 §1 extended for a fact it did not have — `betterproto2-compiler` re-emits `song.proto` and `history.proto`, which `codegen.sh` now deletes and re-imports from `escribass_schema`, because it has no `extern_path`; and one deferred check recorded in the ledger, because the generated `AssistantBase` imports the `grpclib` that PR 5 was to pin — **which the user then closed on 2026-09-23 by pinning `grpclib` 0.4.9 here instead**, so `proto/tests/test_generated_python.py` imports the tree and asserts a `Prompt`'s `song` is `escribass_schema`'s class, watched failing first against a tree regenerated without the rewrite |
| 4 | `m3.4-provenance` | `core`: ~~the author travels with the call (question 3)~~ **`prepare` decides every entity's `provenance` on every path — a new entity gets the call's, an existing one keeps its own, `prepare_merge` leaves both — which closes the forgery the spike found (ADR 0021 §1)**; the two binaries keep their flag as a default; ~~`app` stops hardcoding `Author::Human` where a model can call~~ **`app` keeps `Author::Human`, because under ADR 0020 §1 the model's calls never enter through the window's author** (the three ids reach the log through the proposal, PR 8). **The silent PR**: ~~five determinism goldens carry provenance on every entity and every entry~~ **no golden is expected to move — no script's `apply_patch` adds an entity — and any byte that does is named**. Beside it, the loud half: **`ProjectLock::take` replaces a lock whose recorded pid names no process, and says so** (ADR 0020 §5; ADR 0012 §3, amended), with a `SIGKILL`ed holder and a live one as the tests, watched failing first. **Done 2026-09-23, and no golden moved** — none of the five, and not the projection golden, exactly as ADR 0021 predicted, because no script's `apply_patch` adds an entity and a typed tool's minted provenance is byte-identical to the one the walk writes over it. Two things neither ADR had to say. The provenance walk runs **before** `bump_versions`: a caller who rewrote nothing but a provenance has then changed nothing at all, and bumping first would have recorded an entry whose only operation raised a version for a field that had been put straight back. And `take` is two `O_EXCL` attempts, not one — the stale file is removed and re-created, so a process that replaced it in the same instant wins and this one is refused by a holder that is alive. The forgery test failed first with `AUTHOR_HUMAN` where `AUTHOR_MODEL` belongs; the lock test failed first with `project_locked`, having already asserted the premise that the signalled holder left its lock behind |
| 5 | `m3.5-ai-shell` | **The first PR that produces a process a person can see**, and the smallest one that can. `ai/`: the package, `pyproject.toml`, `uv.lock`, `.python-version` with the exact patch §17 asks for, `schema/pyproject.toml`'s `[build-system]` (the ledger row's trigger, fired), **`grpclib`, `openai` and `httpx2` pinned by exact version here, in `lock.baseline.json` and in §17 in one change (U4; ADR 0020 §2)**, the generated `Assistant` server over a socket it names, the scripted provider of question 11 with one recorded transcript, and a process **`core` spawns as it spawns the engine** (`core/src/assistant.rs`; ADR 0020 §4), the window shows a health dot for from its exit status, and survives the death of. It answers a prompt with no model — the transcript's answer — and proposes nothing. `ai/`'s tests join the `checks` job here, not later (trap 17). ~~And with them PR 3's deferred check: the generated `escribass_proto` imported and its `AssistantBase` registered~~ — **PR 3 kept it: the user pinned `grpclib` there on 2026-09-23, so the import check and the one that a `Prompt`'s `song` is `escribass_schema`'s class already run** (ledger row, closed). What this PR still owes is `grpclib` in `ai/pyproject.toml` at the version `lock.baseline.json` already names. No loop, no view, no panel. **Done 2026-09-23.** `openai` **3.19.0** and `httpx2` **2.13.1**, resolved by `uv lock` in `ai/` and not the 3.14.1/2.13.0 the spike ran — which is the whole of why U4 pins on first install. Five things the row did not say. **The transcript is a real recorded exchange**, not a hand-written fixture: one turn against `deepseek/deepseek-v4.1-flash` through OpenRouter, $0.00008965, saved as the request *body* and the response with no headers — recorded for `tests/AGENTS.md`'s reason about invented plugin ids, and it carries four things a hand-written copy would have got wrong (`provider`, `native_finish_reason`, a `reasoning` field beside the content, and the `model` the response itself names, which is what `Done.model_id` records). **The `[build-system]` trigger cost a second package file**: `proto/pyproject.toml`, the Python half of the `package.json` already beside it, because `ai` imports `escribass_proto` as well — both depended on **editable**, since `codegen.sh` deletes `gen/` whole and a built copy would be a stale second `Song`. It also cost §17's only range, `uv_build>=0.10.2,<0.11.0`, recorded as a range with the reason rather than pinned to look tidy. **Stopping the sidecar is closing its stdin** — `escribass-mcp`'s own signal, which ADR 0020's Context names — because a sidecar has no last call to end on; that is what makes `exited 0` mean something a health dot can distinguish from a crash. **The engine's spawn is shared, not copied**: `listening` and `ended` moved to free functions in `core/src/engine.rs` and both children go through them, because trap 3 is about the twin that stops matching. And **the one place the engine's shape could not be copied**: a turn that fails while the child is alive does *not* kill it, since a sidecar serves a session where an engine serves a call — which has its own test. Trap 17 is closed by step 5/5 of the `checks` job, and an `ai/`-only diff was run through the gate's own `grep` to prove it arrives there. No golden moved |
| 6 | `m3.6-view-and-axes` | The Libretto view and the six axes as PR 1 decided them, pure, goldened against `tests/determinism/render/expected/song.json` and its key-reversed twin — a projection golden in a third language, for ADR 0012 §5's reason. Read-only: nothing here calls a mutating tool. **Done 2026-09-24.** Seven pure functions in `ai/src/escribass_ai/`, seven committed golden files, and the twin. Four things the row did not say. **The ADR's hand was wrong in three lines and right in every figure**: §1's written-out view lost the signature event's tick, left out `generators: none` where it wrote `markers: none`, and spelled the double 1.0 two ways four lines apart — all three corrected in the ADR, dated, since the block is a projection of a rule the table states. Every one of §4's worked figures — 3/7, 160, C 1800 and E 480, chromaticism 0, three ascending of six, 8/8 = 1, no chord width, `A B C`, 1/27, 17/18 — the implementation reproduces independently. **The key reversal reaches further in Python than in TypeScript**: a `dict` preserves insertion order for every key, so unlike `app/tests/projection.test.ts` it does separate the two orderings for an integer-keyed `ParamID` map, which is asserted rather than assumed. **Which sort each test carries was measured**, by deleting them one at a time: the golden alone catches the clip order and the effect chain (the fixture's ULIDs disagree with both), and the reversal alone catches the voice order, the lanes, the points, the bar block's notes and the tempo map — five orderings a golden over this fixture cannot see. And **the constructed cases are three**, not the row's two: ADR 0018 §5 adds a voice striking two notes at one tick, because the fixture's only coincidence is tick 960 between two *different* voices and the paper's chord width is within a voice — so over the fixture alone `widest chord none` and `mean simultaneity 1` are what a right implementation and one that never looked would both golden. No existing golden moved |
| 7 | `m3.7-unseedable` | ~~The §2.2 randomness row: the source audit of Surge XT, Dexed and sfizz, the denylist beside the manifest versioned by plugin commit, the validator rule and its ADR, refusing the value for every author. Or, if PR 0's count says the audit is larger than a PR, question 7's (c): `set_param` withheld from the model's tools and "What M3 will not claim" saying why. **Before PR 8**, because the row is due before the loop can call `set_param`~~ **Not needed.** PR 0 counted 194 `ParamID`s by one rule and 24 to 2,283 by others, with a default that reaches the RNG and a `state` escape, so the audit is a validator rule over values and combinations and not a PR-sized list; the user took (c) on 2026-09-17 and the withholding is ADR 0022 §1's tool list, which lands in PR 8. The number is kept so every trap and question that cites it stays true; the §2.2 randomness row stays open, retriggered below |
| 8 | `m3.8-loop` | The loop: prompt → the view and the schemas → tool calls → **a proposal in `core` — the fork, the kept ids, `call`, the one patch, `apply` through `run` under `proposal` (ADR 0019)** — with `OFFERED`, the filtered schemas without `dry_run`, and the three ids travelling with the proposal (ADR 0021 §2; the `Lock` struct gains `ai`, written on first use); `valid = false` fed back with every violation and three refused calls ending the turn, `Err` ending it at the host, the third kind retried its bounded way in `ai` and surfaced as the provider’s, each watched failing first (ADR 0022 §3); the scripted provider’s transcripts as fixtures, the event stream goldened in `ai/` and the project goldened end to end in `tests/`, driven twice; the live run behind an environment variable, skipped loudly (ADR 0022 §4). Trap 9’s number re-measured with a proposal driving one write. **What it does not do**: nothing is applied — a proposal ends in the stream and waits for PR 9. **Done 2026-09-24.** A `Proposal` is a **session on a copy of the project that records nothing**, so the model’s calls go through `core::call` — the same dispatch the window and the MCP server use — and the only gate is the offered list, which is also what keeps the branch tools and `add_asset` from writing to the real root out of a clone. Six things the row did not say. **The premise is a test**: two dry runs, the second naming the first’s id, refused `track_unknown`, so the rest of the file fixes something rather than asserting a design. **The patch is prepared against the project as it stood**, not as it is — `restore_versions` leaves the diff with no version claims at all, so the claims of `before + 1` come from that one `prepare`, and preparing against the moved document instead would merge over a person’s edit in silence. **The version guard refuses a document that moved by two entries and not by one**, measured both ways and written into ADR 0019: a stale claim of `before + 1` equals the number the entity holds after exactly one edit, which ADR 0005 §3 reads as disputing nothing; and because every change bumps the song’s own `version`, two entries anywhere refuse at `/version`. **Applying takes the session past every id the proposal minted** — found by reading the first golden, where the `proposal` entry and the track it created were the same 26 characters, and fixed by taking whichever source is ahead rather than swapping, because a person may edit while the turn runs and a swap takes the session backwards (`entry_exists`). **The transcripts are hand-written** from the shape of PR 5’s one recorded exchange, because recording a turn costs a paid call and CLAUDE.md #7 says nothing spends the user’s money without the user — which the live run behind `OPENROUTER_API_KEY` is what replaces. And **the recorded transcript trips the token floor**, which is evidence rather than an awkwardness: its 68 prompt tokens record a 250-byte prompt and the loop sends nine kilobytes. Trap 9 re-measured: six proposed calls in 6.1 ms with no write, one apply in 3.9 ms with one. **No existing golden moved** — not one of the five, not `app/tests/projection.golden.json`, not the four WAVs.

**Then, the same day, the user granted the paid call, and the hand-written sentence above stopped being true.** Three things landed on this branch after the row was first written. **A paid call is capped, ledgered and reconciled** (CLAUDE.md #7; ADR 0022 §4, amended): `provider.Live` prices the request about to go out at its worst case against a committed ceiling of $0.25 and raises `BudgetExhausted` **before** the call rather than after the receipt, writes one line per call to `~/.escribass/spend.jsonl` — read back at startup, so a second attempt inherits the first's spend — and `account_usage` reads OpenRouter's own counter either side of a run. The ceiling reverses ADR 0022's own rejection of a dollar guard, which held only while no call was authorised. **The loop now sends `max_tokens`** (8,192), because "what could this call cost" has no answer while a completion is unbounded: without it the worst case is the provider's 131,072-token limit and $0.25 refuses the fourth call of a turn that really costs a fifth of a cent. **The live run records**, to `target/live-turn.json`, asserting its own file carries no header and no key before anybody considers committing it. It was run once and composed on the first attempt: `add_track`, then `add_clip` naming `01M1FPMP000000000000000034` — the id the first call's result returned — then the reply. Three exchanges, two calls, **zero refusals**, 14.6 s, **$0.00376174**, against an account counter that moved by exactly $0.00376174. That recording is now `tests/determinism/proposal/transcript.json` and both multi-call goldens are driven from it; `ai/tests/transcripts/two-calls.json` is **deleted** rather than copied, because two copies of one recording is the twin trap 3 is about. **Four goldens moved, each with a cause**: `ai/tests/golden/turn.json` and `tests/determinism/proposal/expected/` for the recording and for `max_tokens`, and the proposal's `proposal` entry renamed `…039` → `…03B` because the model wrote four notes where the hand-written fixture wrote two, and two more minted ids move the entry's own. `app/tests/projection.golden.json` and the four WAVs still did not move |
| 9 | `m3.9-panel` | The AI panel: the conversation — **held by the host, sent whole per prompt, persisted beside the project as JSON Lines per song id and read back on open (ADR 0021 §3)** — the proposal drawn dashed as it grows where the wireframes draw it, the RFC 6902 diff, **Apply and Reject first, Edit last** (ADR 0019 §3), `proposal` rows in the history view with the model named and a `prompt_id` this machine does not have said so, and the wording that says what the model is and is not (the *live preview · not the render* precedent; nothing counts `ai.model` as verified, trap 15). ADR 0017's shape, with a proposal in place of a drag. The bar-17 demo driven by the assistant, diff on screen before apply — `roadmap.md`'s proof point — is this PR's own test, against the scripted provider |
| 10 | `m3.10-review-fixes` | A whole-stack review's findings. M0 averaged four to sixteen per milestone, M1 returned eighteen, M2 a blocker, three majors and a heap corruption nobody had filed; budgeting a PR for it is cheaper than discovering it |
| 11 | `m3.11-close` | Docs walked against the code, §11 walked bullet by bullet with the enforcer named for each, the ledger walked a fifth time, `CLAUDE.md` to M4 — and what M3 leaves unverified written where a reader will find it |

**Which rows cannot be sized yet, and why.** PR 3 is entirely question 1's answer — a
`SongTools` server in `app` with a Python client is a different amount of work from a stream
the host executes, and only the first needs `SongTools` generated in Python. PR 4 is question
3's, and its size is how many sites set an author today (three) and how many goldens move
(five). PR 6 has no size until PR 1 says which axes can be computed without a corpus and
without U1. PR 7 has no size until PR 0 counts — it may be a PR or it may be the withholding.
PR 8 is question 9's: a fork chain is `core` work before it is a loop, a branch is none, and
one-call-per-proposal is the smallest loop and the least useful. PR 9 is the size it looks
only if question 10 leaves Edit for last. Only PRs 0, 1, 2, 5, 10 and 11 are the size they
look.

The split follows M0.2's lesson, which M1 and M2 each confirmed: PR 8 is the loud concern
(does a model produce a valid proposal), PR 4 the silent one (did every entry in five goldens
change provenance the way the ADR says and no other way). Mixing them gets the silent half
reviewed as plumbing — and this time the silent half is the audit trail.
### Traps

Written from what actually went wrong in M0, M1 and M2, applied one milestone over — not from
a list of generic risks.

1. **A check that cannot fail.** A loop test against a scripted provider whose every turn is a
   valid call proves that valid calls are applied, which nothing doubted. The retry path
   needs a transcript that returns an invalid call four times and a test that counts three
   retries and one surfaced failure; the `Err` path needs a transcript that hits one and a
   test that counts zero retries. M2 PR 11's rule — every fix has a test that was watched
   failing first — applies before there is a fix, to every branch of question 8.
2. **A misclassified error is a retry loop that cannot succeed.** M1 PR 13 found an engine that
   exited 0 having written nothing reported as `valid: true`. The loop's version is a provider
   429 reported as `valid = false` — three retries spent on a wall that is not the caller's —
   or a `plugin_unknown` reported as `Err`, which the panel then shows as a corrupt project.
   ADR 0006 §2 made the split once, in the session; the third kind question 8 names has to be
   split in one place too, and a test per kind is what keeps the three from drifting into two.
3. **A defect fixed at one site and left at its twin.** The author is set in three places today
   — parsed in `escribass-grpc`, parsed in `escribass-mcp`, hardcoded in `app`'s host — and
   `Provenance` is built with three `None`s in three more (`tools.rs`, `project.rs`,
   `session.rs`). Making provenance travel with the call (question 3) touches all six or one of
   them keeps stamping `HUMAN` with no ids, and the history view will show it as a person's
   edit. M2 PR 7 found ADR 0017 §3's own defect in the element chosen to avoid it; this is the
   same shape with a wider spread.
4. **A "measured" claim that was false.** PR 7's "the factory init patch does not reach Surge's
   RNG" was read off three renders that had appended to one file. M3's spike measures a
   *hosted model* — "it drives the API reliably", "the bar-block view halves invalid calls" —
   and every such number is true of one model id on one day, routed to one provider, and may
   be false next week under the same name. A spike number goes in this file with its model id
   and its date, is cited as evidence and never as the claim, and nothing in an ADR says
   "reliably" without the sentence after it saying on what.
5. **A promised change that never happened.** ADR 0010 promised M2 a re-pin tool and M2's plan
   never listed it, so nothing built it. ADR 0006 §7 promises Python codegen "at M3"; §17 says
   `ai/.python-version` pins an exact patch; `schema/pyproject.toml` waits on "when `ai/`
   depends on it"; ADR 0003 §6 promises the Libretto ADR before M3. Each is in a PR row above
   by name, because a promise that is only in an ADR is the one that gets kept by nobody.
6. **Testing something CI cannot run — twice over.** A hosted model needs a key CI does not
   have and money a test must not spend, so the live run is ignored loudly, as the device test
   is. And GitHub has refused every job since M2 PR 8, so an `ai/` CI step written now is a
   step nobody has watched pass on a runner — "merged on local runs" is how M2 PRs 9–11 landed
   and how M3's will until the limit lifts. Every green number in this milestone's PRs is from
   one machine, and each PR says so, as "M2, closed" does.
7. **The view that becomes a second representation.** A Libretto bar-block is text derived
   from the `Song`, and the moment it is cached between turns, edited, or written back it is
   CLAUDE.md #1's failure in a fourth language. M2 trap 1 was answered by a golden that proves
   a view is a pure function of the model; the same golden, in Python, is the only thing that
   would catch a summary that drifted from the document — the prose has existed since §2.1 and
   would not.
8. **Dry runs do not compose.** A dry run writes nothing and mints real ids from a fork, so the
   second call of a two-call proposal names an entity that does not exist. The model will do
   exactly this on the first real instruction anyone types, and a panel that shows "valid" for
   the first call and `track_unknown` for the second has shown a person a proposal that cannot
   be applied. Question 9 exists because ADR 0017 answered one gesture and a proposal is
   several.
9. **`Project::write` is O(history), and the loop is the first author with no person between
   commits.** Measured in M2 PR 5: 17 ms at 21 entries, 30.6 ms at 300, 6.7 s for 300 in a
   row. A branch-per-proposal (question 9(a)) commits every model call; a fork (9(b)) commits
   once per approval; the difference is trap 8 of M2 arriving as a design decision rather than
   as UI lag. Whichever is chosen, the number is re-measured with a model driving.
10. **The key in a fixture.** A transcript recorder that saves the request saves its headers,
    and `Authorization: Bearer …` is then in `tests/`. It parses, replays, passes every test
    that does not read it — M0.3 shipped MCP dropping `provenance` for a milestone by the same
    mechanism — and is in `git log` for ever. The recorder saves responses and the request
    *body* only, a test asserts no fixture contains the key's prefix, and the key reaches the
    process from the environment alone (U3).
11. **The model learns a contract that is not real.** ADR 0006 §4's argument, one language
    over. The tool schemas are proto3 JSON with **proto field names** — `core/src/call.rs`
    decodes with `preserve_proto_field_names`, so `start_tick` — and betterproto2's `to_json`
    emits **camelCase and a `Z` timestamp** (`schema/tests/test_roundtrip.py`, its own header).
    A sidecar that builds a `Note` from the generated type and hands its `to_json` to the tool
    API sends `startTick`, which the tool refuses or ignores. `proto/AGENTS.md` already names
    the TypeScript half of this trap; the Python half is the same sentence, and the schemas the
    model is given are the descriptor's, never the Pydantic model's.
12. **A second validator, in Python.** `from_json` coerces and constructors validate
    (`test_roundtrip.py`, `TestParsingIsNotValidation`), so a sidecar that constructs a `Note`
    from the model's output gets a `pydantic.ValidationError` *before* the call — and turning
    that into a refusal the loop retries on is a second implementation of §4.4, in a third
    language, which is exactly the shape ADR 0017 §3 refused for a drag. The tool API is the
    validator; the sidecar sends what the model said and lets `core` refuse it, with the rule.
13. **Two pending things on one session.** A person's held drag and the model's proposal are
    both dry runs against the same document; whichever applies second is refused by §4.3's
    `version` check (ADR 0012 §4), and the refusal has to say *what moved* — "the model edited
    this clip while you were dragging it" — rather than "version 3 is not version 4". The
    wireframes draw both pending at once and never say which wins.
14. **`buf breaking` is the only guard on `proto/`, and it runs on pull requests only.** The
    `ai` service, provenance on the wire, `list_params` — all three land in PR 3 together,
    for M2 trap 12's reason; spread across PRs 3, 4 and 8 they are compared against a `main`
    that already moved.
15. **A "verified" readout counting a thing that cannot be verified.** The wireframes' status
    bar reads `lock.json 14/14 verified`, and `ai.model` will be a line in that file. A hosted
    model id verifies nothing — there is no commit, no hash, no build to compare — and a count
    that includes it is a check that cannot fail dressed as one that passed. M2 trap 4 kept
    every §11 number out of the status bar until `core` reported it; this one is not `core`'s
    to report at all.
16. **The spike's number is not the decision.** M2's question 6 was answered by measurement
    and the measurement did not cover everything — coexistence and cost were "the unverified
    half", and PR 9 paid for them. PR 0 will measure a model on a day; what it cannot measure
    is next month's model, and the ADRs say which half is unverified rather than reading a
    sample as a mechanism (ADR 0009 §6's own lesson).
17. **`ai/` runs the `checks` job and the job runs nothing of it.** The path gate skips only
    `docs/`, `engine/`, `.githooks/` and root prose, so a change under `ai/` runs the job — to
    a job whose steps do not know `ai/` exists. A green run then says nothing, which is
    `checks.yml`'s own words for a gate that gets it wrong: "it does not fail, it just stops
    testing something and nobody notices". PR 5 adds the step in the PR that adds the
    directory.
18. **A rule that depends on who is asking.** Refusing a parameter for `Author::Model` that a
    person may set makes §5's "used by the UI and the AI identically" false, and is a rule by
    author that no ADR has ever written. The denylist (question 7), the retry categories, the
    tool list — none of them may check the author to decide what is valid. What differs by
    author is what is *offered* (question 13), never what is *accepted*.

### What M3 will not claim

- **Not that the same prompt produces the same song.** A hosted model cannot be seeded, and no
  test pretends it can. What is claimed is narrower and is written that way above: the song is
  reproducible from its log, and the sidecar's own functions are pure and goldened.
- **Not that `ai` is deterministic in CLAUDE.md #3's sense.** It is not in the list and this
  does not put it there. §7.1's DSL, when M4 puts one inside it, is.
- **Not that the model is pinned.** `ai.model` is a name a person chose; what answers to it
  moves. `provenance.model_id` records what actually wrote an entry, and that is the record.
- **Not symbolic generation and not analysis** (§6.2, §6.3), unless U1 says they are v1 — and
  then not in M3 without a re-plan. The Libretto axes that need a key say so rather than
  computing one.
- **Not code generation for the compilers** (§6.4): M4's, with the compilers.
- **Not a local LLM** (§15): a config change, and a config nobody has tested.
- **Not that a user-chosen or model-chosen patch is reproducible**, unless PR 7 lands the
  denylist whole. Half of one is not claimed either.
- **Not that an external MCP client and the window can edit one project at once.** The lock
  refuses it (ADR 0012 §3), and whether that changes is U5.
- **Not that the model can hear anything.** CLAUDE.md #6, and nothing in M3 gives it ears: a
  proposal is judged by a person who pressed play, and the sidecar never learns what they
  heard.
- **Not macOS or Windows.** Linux x86-64, on the image and compiler §17 pins, exactly as M1 and
  M2 (ADR 0014 §2). Unclaimed, not contradicted.
- **Not anything CI has verified**, until a run on `main` goes green again. Every number will
  be one machine's, as "M2, closed" already says of M2's.
- **Not that a preview has played on a device.** M3 does not touch the engine.
- **Not the plugin's own editor, not code views** (M4), **not DAWproject or MIDI** (M5), **not an
  installer** (M5): M3 runs from a build tree, and so does its sidecar.
- **Not a resolution of any `[OPEN]` item.** Two are in M3's path — symbolic model choice, and
  whether §6's analysis and generation are v1 at all — and both are U1 and U2 above, asked and
  not answered. The other two are unchanged.
- ~~**Not that a real model has driven this loop.**~~ **Amended 2026-09-24: one has.** The
  user granted one recording session at a $0.25 ceiling, the live run was made, and
  `tests/determinism/proposal/transcript.json` is the recording it left — so "a hosted model,
  offered these twelve schemas, composes a multi-call proposal through this loop, the second
  call naming the id the first returned" is now a measurement of *this* loop and not the
  spike's of another. What is still **not** claimed: that it does so reliably, because that was
  one turn and no repeat was bought; that a turn is reproducible, because a hosted model is not
  seedable and only the *replay* is deterministic; and that a real 429 or a fourth real refusal
  has ever been seen by this loop — those two transcripts stay hand-written and say so
  (ADR 0022 §4, amended).
- **Not that a pending proposal survives any concurrent edit.** It survives an edit elsewhere
  and one intervening entry; two entries anywhere refuse it, at `/version` if nowhere more
  specific, and exactly one edit to an entity it touched merges over that edit in silence —
  both measured, both named in ADR 0019's Consequences. That is ADR 0005 §3's optimistic apply
  as a held drag has always had it, not something M3 widened.
- **Not per-call approval, and not the model's steps in the log.** A person approves a
  proposal's whole patch once; the entry says `proposal` and what changed, and the calls it was
  composed from are in the conversation, not in `patches/` (ADR 0019 §2, §4).
- **Not that an MCP client's entries name their model.** `escribass-mcp --author model` writes
  `AUTHOR_MODEL` and no `model_id`, because nothing on that wire says which model is on the
  other side (ADR 0021 §2). Nor that a `prompt_id` resolves on a machine other than the one
  that made it: the conversation lives beside the project and travels with nothing.
- **Not that the model can name a plugin parameter the document does not already automate.**
  No `list_params` and no manifest reach it (ADR 0022 §2); a track's fader and pan need neither.
- **Not typed tools for a mix, a deletion, a rename or a clip's bounds.** The model reaches
  them through `apply_patch`, as the mixer does; typed tools wait on a measurement (ADR 0022 §1).
- **Not a dollar cap.** Three refused calls and twelve responses bound a turn; a budget in money
  needs a price list per endpoint and a reservation ledger the spike's own guard got wrong once
  (ADR 0022 §3).
- **Not that a lock with no pid in it is replaced.** Only one whose recorded holder is
  demonstrably gone; a lock written by hand, or by a build that failed to write its pid, is
  refused as it always was (ADR 0020 §5).

## After M0

One line each; §16 has the definitions, and ADR 0003 placed what §16 had left out. M1 render engine and first golden render · M2 Tauri UI
· M3 AI loop · M4 compilers · M5 interop and installer.

## Deferred, on purpose

Each of these was raised, judged, and put off. None is forgotten; none is blocking.

Walked again at M1's close, 2026-09-07. No row was left waiting on an M1 event: PR 9 closed the
M1 half of `lock.json` and PR 13 moved `Instrument.state` off the trigger that had already
passed, and both rows say so below.

Walked a third time when M2's questions were answered, 2026-09-07, and this is the pass that
changed the most. Two rows are **closed** by ADR 0015 — `ParamRef` reaching mix params, and
interactive merge conflict resolution — and four are **retriggered**: every row whose revisit
point was a milestone rather than an event now names an event, because M2's own question 8
found three rows waiting on a milestone that had never been placed. A trigger that names a
milestone cannot be checked at that milestone's close; it can only be argued about.

Walked a fourth time at M2's close, 2026-09-17, one row at a time, asking of each whether its
trigger had fired and whether it *could* fire. Three rows closed during M2 and say so in their
own text (`ParamRef` to mix params, interactive merge resolution, and the undo tools). No open
row has a trigger that fired and was ignored, and every open row's trigger is a condition
something can produce: PR 11 had already re-tied the one row whose
trigger nothing was scheduled to produce — the §2.2 randomness gap — to M3, and it is the one
row **due before M3 starts its loop**. Two rows were checked against the window rather than the
prose: no loop control exists in `App.tsx` (play, stop and a seek to tick 0 are the transport),
and nothing in `app/` or the tool API writes an `Instrument.state`. One row is **added**, for a
promise an ADR made and no PR kept.

Four rows added and one closed on 2026-09-21 by M3 PR 2, which is not a walk: ADR 0018's
Consequences assigned two rows to it — the Libretto package, and the axes deferred on a corpus —
and ADRs 0022 §1 and §2 defer two things with triggers of their own. The Libretto row is added
already closed, because the provenance read its trigger named was made before this pass and is
recorded on PR #71.

A fifth row added on 2026-09-22 by M3 PR 3, also not a walk, and it is a **deferred check**
rather than deferred work — the first row here of that kind, and it was written down for the
reason every trap in this file is: a check nobody has run is not a check, and PR 3 first
committed generated Python that nothing imported. **The user closed it on 2026-09-23, in that
same pull request**, reading U4's "pinned when it is first added" as satisfied by the check
itself being what adds the package — so `grpclib` 0.4.9 is pinned in PR 3 and the import
check landed beside the code it checks. The row stays, struck through: the shortest-lived
entry this table has held, and the one that shows what the table is for.

A sixth row added on 2026-09-24 by M3 PR 8 — the second **deferred check**, that no real model
had driven this loop — was closed the same day, in the same pull request, when the user granted
one paid recording session. It stays struck through for the same reason the fifth does: the
deferral was real, the reasoning was sound while nobody had authorised a call, and a reader
should be able to see both that and what the run then measured.

| Item | Why deferred | Revisit at | Source |
|---|---|---|---|
| ~~The Libretto package (MIT) as a dependency~~ | **Closed 2026-09-21, on a read made 2026-09-17 and recorded on PR #71.** Not needed after ADR 0018 §4: nothing on the package's surface would be called — its fingerprint, genre bands and `copy_risk` need the corpus and are verdicts, its encoder and decoder read MIDI `ai` never sees, and its per-song floats over its own grammar are not per-bar exact rationals over the `Song`. Its licence text carves the data out of the MIT grant, and the corpus is identifiable transcriptions of copyrighted works — `song_0001` is named, with its MusicBrainz id — with no redistribution grant in the chain; "for research reproducibility" is a stated purpose, not a licence term. Had it been wanted: thirty Python packages, 103 MB of package data, a 283 MB clone, roughly ten times the approved sidecar stack | closed unless the corpus-free decision of ADR 0018 §4 is reopened | ADR 0018 §6, Consequences; PR #71 |
| The axes ADR 0018 §4 defers by name — entropies, standard deviations, the paper's bass, and within-song variation as the paper defines it | Each needs a corpus this repository may redistribute, a key (U1), or a transcendental function whose last bit is the platform's, which a byte-compared golden cannot carry | A corpus this repository may redistribute, or U1 reopened; the entropies wait on a golden that compares numerically rather than by byte, which nothing here does | ADR 0018 §4, Consequences |
| `list_params`, or any way the model learns a `ParamID` the document does not carry | With `set_param` withheld, 2,855 ids would be ids nothing the model is offered can act on except by automating them, which is `set_param` over time with the reason it was withheld unchanged. A `ParamRef` naming a track needs none | The first time `set_param` is offered to the model — the §2.2 randomness row's denylist landing | ADR 0022 §2 |
| Typed tools for a track's mix, a deletion, a rename and a clip's bounds (`set_mix`, `remove_*`, …) | No typed tool does any of them and the model reaches all of them through `apply_patch`, as the mixer does; three models found it unprompted within nine instructions and the spike measured no unparseable patch. Six RPCs and six tool functions built on a guess is worse than one raw tool whose two hazards are closed | The first measurement in which a model gets the RFC 6902 wrong where a typed tool would not have let it | ADR 0022 §1 |
| ~~**No real model has driven this loop.**~~ **Closed 2026-09-24, in the pull request that opened it**, by the user granting one recording session at a ceiling of $0.25 (CLAUDE.md #7). `a_live_model_drives_the_loop` was run with the key and **records**: `tests/determinism/proposal/transcript.json` is now the real turn — three exchanges against `deepseek/deepseek-v4.1-flash`, the request bodies without headers, committed as the recorder wrote them — and both multi-call goldens are driven from that one file. **One attempt, and the first:** `add_track`, then `add_clip` naming `01M1FPMP000000000000000034`, the id the first call’s result returned, then the reply. Zero refusals, zero provider failures, no retries, 14.6 s, **$0.00376174** — ledgered, and reconciled against OpenRouter’s own counter, which moved by the same $0.00376174 (47.51704692 → 47.52080866). The spike’s two live hits of the id trap did not recur. What the row said while it stood: every transcript was built from the shape of the one exchange M3 PR 5 really recorded, with only `choices` and `usage.prompt_tokens` changed, because a recorded transcript needs a paid call. **What is still hand-written, and still says so:** `rate-limited.json` and `four-refusals.json` — a 429 and four consecutive refusals cannot be summoned from a real provider on demand, so those two remain constructed cases. And one attempt is not a rate: nothing here claims how often a model gets this right | ~~The first time the user approves spending on it~~ — **approved and made 2026-09-24**. Next: a second recording, if ever, when the loop’s prompt or the offered twelve change enough that a 2026-09-24 turn stops being evidence about this loop | M3 PR 8, 2026-09-24; run and closed the same day |
| ~~The generated Python under `proto/gen/python` is imported, type-checked and run by nothing~~ | **Closed 2026-09-23, in the pull request that opened it — one day and no milestone.** It was deferred because the generated `AssistantBase` imports `grpclib`, and `grpclib` was approved by name and unpinned until PR 5, U4 pinning a package "when it is first added, not before" (ADR 0020 §2); pinning it in PR 3 looked like contradicting, one pull request later, the decision PR 2 had just recorded. **The user read it the other way and closed it**: the check *is* what first adds the package, so PR 3 is where U4 says it is pinned. `grpclib` 0.4.9, resolved by `uv`, in `schema/`'s dev group because `proto/` borrows that environment; `proto/tests/test_generated_python.py` imports the tree and asserts that a `Prompt`'s `song` is `escribass_schema`'s `Song` class — the half that matters, since an import alone passes against the duplicate `betterproto2` re-emits — watched failing first against a tree regenerated without `codegen.sh`'s rewrite. The row is kept struck through rather than deleted: the deferral happened, and a reader should see that the reasoning was sound and the decision was a person's | ~~M3 PR 5~~ — **closed in M3 PR 3** | M3 PR 3, 2026-09-22; closed by the user 2026-09-23 |

| Item | Why deferred | Revisit at | Source |
|---|---|---|---|
| `FormRule` | Least-specified entity in §4; nothing consumes it before the generative compiler | M4 | ADR 0002 §7 |
| `Instrument.state` as a content hash instead of inline `bytes` | Plugin states are large base64 in a file §2.6 wants diffable — but adding a hash field and deprecating `state` is additive, not breaking. ~~Revisit before M1 renders a plugin~~ — that trigger passed at PR 7 and M1 re-judged it: nothing in M1 *writes* a state. ~~M2, with the first plugin editor~~ — **retriggered 2026-09-07**: M2 *has* an editor (ADR 0014 §1) and it still writes no state. A generic parameter editor writes `params`, a map from `ParamID` to a normalised double; `state` is the plugin's own opaque blob and only the plugin's own serialisation produces one. So there is still no producer, which is ADR 0002 §7's reason unchanged | The first thing that **writes** an `Instrument.state`: a plugin's own VST3 editor, or a preset import. Neither is M2's | review, 2026-09-02; re-judged M1; retriggered by ADR 0014 §3 |
| ~~`ParamRef` reaching track mix params (gain, pan, mute)~~ | **Closed 2026-09-07 by ADR 0015 §1** — and it cost no schema change at all. Ids are already globally unique across collections (ADR 0001 §3), so a `ParamRef` whose `device_id` resolves to a track addresses `Mix.gain_db` and `Mix.pan` with no field added. `mute` and `solo` stay unaddressable on purpose: a double automating a boolean needs a threshold rule that would be ours and pinned | **closed in M2 PR 6**, 2026-09-09 | review, 2026-09-02 |
| Dense unique `index` on tracks and effects | Inserting mid-list renumbers everything, and two branches inserting at one index auto-merge into an invalid document. Deferred again at M0.3: the merge pipeline makes that failure loud (the validator refuses it) rather than silent, and closing it properly is a `song.proto` change with its own ADR. ~~M2, with the mixer~~ — **deferred again 2026-09-07** (ADR 0015 §4): the mixer is a milestone, not an event, and a mixer that draws a chain in order and adds at the end never renumbers. M2 is on record as offering no reorder gesture | The first gesture that reorders an effect chain or inserts a track mid-list | review, 2026-09-03; ADR 0015 §4 |
| ~~Interactive merge conflict resolution~~ | **Closed 2026-09-07 by ADR 0015 §3** and **built 2026-09-09 in M2 PR 8**: one optional per-path resolution on a second `merge_branch` call. No new tool and no merge state held between calls, which is the shape ADR 0006 §5 refused for projects. What the build added to the decision is in ADR 0015 §3's extension: `resolution_unknown`, and a merge with no ops that records an entry anyway | closed — M2 PR 8, 2026-09-09 | ADR 0001 §4 |
| Windowing the parameter editor's rows | Surge XT declares 2855 parameters and the editor renders every one: **1.65–1.79 s** to first paint on this machine, against 270–380 ms for the mixer and the roll, measured in the window. `content-visibility: auto` — a native property, not a library — took it from 2.0 s, which is a fifth and not a fix; a search box is what makes 2855 rows usable, and it is not what makes them fast. Deferred rather than solved because a virtual list is a scroll implementation with its own bugs, bought for one second on an action a person takes rarely, and because the ceiling is known: the cost is linear in what the manifest declares (ADR 0016 §3) | A plugin whose editor takes longer to open than a person will wait, or the first time the second is measured as friction rather than noticed | M2 PR 7, 2026-09-09 |
| Garbage collection of orphaned patch entries | Entries are small and inert | only if a real project makes it a problem | ADR 0001 Deferred |
| `SourceRef.export_hash`, `Generator` compiled-source hash | Needed for "export pending" and "compiled · stale"; nothing produces either yet | M4 | ADR 0002 Consequences |
| Strudel as a second `Generator.kind` | Python DSL is the v1 target | after M4 | §15 |
| ~~`schema/pyproject.toml` `[build-system]`~~ | **Closed 2026-09-23 in M3 PR 5**, by the trigger firing exactly as written: `ai/` depends on it. The sidecar imports `escribass_schema` and `escribass_proto`, and the alternative — `sys.path.insert` in the package's `__init__`, which is what the tests do — would carry this repository's directory layout inside the product, where an installer (M5) is the first thing to make it false. So `schema/` and `proto/` are packages, `ai/` depends on both by path and **editable**, because `codegen.sh` deletes `gen/` whole on every run and a built copy would be a stale second `Song`. What it cost is one row in §17 that is a range rather than a pin, said out loud there: `uv`'s own build backend | closed — M3 PR 5, 2026-09-23 | `schema/AGENTS.md` |
| Native CLAP hosting | VST3 via clap-wrapper is the mature path | never a dependency | §8 |
| User VST3 plugins | §8 says "VST3 host" and §16 never says user plugins, so nothing places them. M1 refuses a plugin outside the bundled manifest, which makes the gap loud rather than silent. ~~M2, when `app` could show a plugin browser~~ — **retriggered 2026-09-07**: M2's editor is a *view over* the build manifest, and the manifest describes what this build hosts. A browser is not a view over one; it is a scanner that puts things into one, and it drags `lock.json` with it, since a user's plugin has no submodule commit to pin | When the build manifest can describe a plugin this build did not bundle | M1 planning, 2026-09-04; retriggered by ADR 0014 §3 |
| `RenderTarget.tail` for release tails | A render ends at the last clip or section. Every golden controls its own content, so this does not affect the determinism claim — it affects whether a real export sounds truncated. **Measured in PR 12**, so the trigger is no longer abstract: a half-bar note's release runs about 5,800 frames (0.12 s) past its note-off on this build, and that is what a render ending at the last clip cuts off | when someone exports something with a long release | M1 planning, 2026-09-04 |
| Recursive merge, for a criss-cross base | Two branches that each merge a third leave `merge_base` with no single answer, and it refuses rather than guessing which history is the truth. The fix is to merge the bases and use the result. ~~M2~~ — **deferred 2026-09-07** (ADR 0015 §3): refusing is the current behaviour and it is loud, and *nothing in the repository produces a criss-cross history yet*. M2's history view is what makes branch merging ordinary enough for one to appear. **The trigger is now genuinely reachable** (M2 PR 8): merging two branches that have each merged a third is four clicks in the window. Nothing has produced one yet, and `merge_base_ambiguous` is what would say so | A real criss-cross base | review, 2026-09-03; ADR 0015 §3; reachable from M2 PR 8 |
| ~~Undo/redo **tools**~~ | **Closed 2026-09-09 in M2 PR 5.** `undo` and `redo` are RPCs with `dry_run` and the shared `ToolResult`, and the one thing ADR 0005 §4 named without specifying — the session-held stack — is a cursor into the log's first-parent chain that **skips the log's own undo and redo entries**, cleared by any other commit and by a branch switch. Both halves were found by pressing the key: reading `HEAD` again undoes the undo, and walking the mechanism's own entries takes the document forwards (ADR 0005 §4, extended). **Amended 2026-09-15 in M2 PR 11**: the session-held cursor is gone, because it was empty in every fresh session — two edits, two undos, a relaunch and one more undo re-applied the first edit. The chain is replayed as an editor's undo stack on every press instead (ADR 0005 §4, amended) | closed — M2 PR 5; amended M2 PR 11 | ADR 0005 §4 |
| Committing a direct-manipulation gesture on release, without a separate approval | ADR 0017 §4 gives the first control §9's flow whole — release proposes, a person applies — because it is the first control that has a diff to show and the flow should be reviewed where it can be seen. Whether *every* gesture should keep asking is a different question: undo is what would make committing on release safe, and it exists now | The first time per-gesture approval is measured as friction rather than argued about — a session where the Apply click is counted | ADR 0017 §4 |
| `lock.json` beyond `schema_version` | ~~Nothing to pin until compiled artefacts and models exist~~ — the M1 half is **closed** in PR 9: the engine's submodule commits and one entry per referenced plugin. What is left is M4's, the compiled artefacts and model hashes | M4 | ADR 0003 §3; §17 |
| A re-pin tool, and a window that recovers from `lock_mismatch` | ADR 0010's Consequences said "**M2** gains a re-pin tool and the UI that makes `lock_mismatch` recoverable without a text editor", and M2 delivered neither: no PR row named it, no M2 ADR placed it, and the only way to re-pin is still ADR 0010 §3's — an operator deletes the entry and the next write pins it. Found at M2's close, walking ADR 0010 against the code. Not built there and then because a tool is a `song_tools.proto` change and a re-pin is a `lock.json` write outside the patch log, which is a decision and not a closing PR's | The first `lock_mismatch` a person meets, which needs a plugin pin to have moved — and no bundled pin has moved since M1 | ADR 0010 Consequences; M2 PR 12 |
| Loop and seek controls in the window | `render_preview` takes a loop and a seek, and the window offers play, stop and back-to-start: one control per thing a person has asked for, and a loop needs a range gesture on the timeline, which is a second hit region ADR 0017 would have to be applied to | The first time someone wants to hear a bar loop while editing it — the tool already does it, and a loop survives the replacement plan an edit sends | M2 PR 10 |
| Choosing the audio device, its rate and its buffer | A preview plays on ALSA's default output at whatever the device offers, and nothing in the window or on the command line chooses another. A device list is a view over the machine, and the one this repository has measured on had none | A machine whose default ALSA output is not the one wanted, or a buffer the default underruns on | M2 PR 10 |
| Refusing a plugin parameter that reaches an RNG nothing can seed | §8 forbids Surge XT's `rand_pm1`, Dexed's LFO waveform 5 and sfizz's `*_random` **in a fixture**, and nothing refuses them in a user's song. §11's first bullet is about our own code and holds; §2.2's promise — "every source of randomness carries an explicit seed stored in the project" — is wider, and neither of those two RNGs can be seeded at all (Surge's is the wall clock with `seed_rand` commented out; Dexed's `randstate_` is indeterminate memory). Closing it is a validator rule and therefore an ADR — and the rule has no producer: something must say which parameter values reach an unseedable RNG, and today that is prose in §8 for three plugins vetted by hand. Deferred rather than opened as an M2 question because M2 adds no plugin and no randomness: the gap is M1's, unchanged, and an ADR now would design a denylist against a build manifest that carries none, which is ADR 0002 §7's reason ~~The first milestone that lets a user *choose* a patch or supply a plugin~~ — **retriggered 2026-09-07** (ADR 0014 §3). That trigger has now fired: M2 places a parameter editor and choosing a patch is exactly how a user reaches those RNGs. It fired and the work still cannot be done, and the blocker is the second fact, not the first: closing it needs a **denylist of `ParamID`s per plugin in the build manifest**, and §8's prose names the paths in English — "oscillator random start phase, unison detune, the sample-and-hold LFO shape, and the effects that call `rand_pm1`" — which becomes `ParamID`s only by auditing Surge XT's 2855 parameters against its source. That is a source audit, and no user interface produces one. Half a denylist is worse than none, because it looks complete. ~~A build manifest that can say which parameters reach an unseedable RNG~~ — **retriggered 2026-09-15 in M2 PR 11**: that trigger named a capability, and nothing in §16 or this plan is scheduled to produce it, so it could never fire and the row could only wait for ever. What does arrive on a schedule is the reason to close it: M3's loop calls `set_param` with values a model chose, so the RNG paths stop needing a person to wander into them, and a model told only "valid" will not avoid what nothing refuses | ~~**M3**, before its tool-calling loop can call `set_param`~~ — **retriggered 2026-09-21**: the user took question 7's (c), so M3's loop never calls `set_param` (ADR 0022 §1 withholds it), and PR 0's count sized the work as a validator rule over values and combinations with a required default and a `state` escape, not a list — 194 `ParamID`s by one rule, 24 to 2,283 by others. The trigger is **the first time `set_param` is offered to the model**, which is when the audit is due; a person can still set any parameter in the window, and "What M3 will not claim" says a hand-chosen patch on those paths is not claimed reproducible | §8's per-plugin notes; M2 planning, 2026-09-07; retriggered by ADR 0014 §3; retriggered by M2 PR 11's review; retriggered by ADR 0022 §1 |

## Known gaps

- ~~**`apply_patch` stores a caller-written entity `provenance` verbatim**~~ — **closed
  2026-09-23 in M3 PR 4** (ADR 0021 §1). A model could stamp an entity human: a session run as
  `--author model` applied a section claiming `AUTHOR_HUMAN`, created in 1999, and `get_song`
  returned it so while the log entry said `AUTHOR_MODEL` at the real time, which is ADR 0006
  §4's "core overwrites" holding for the typed tools and not for the raw pipeline. `prepare`
  now decides every entity's provenance on every path — a new entity gets the call's, an
  existing one keeps its own, `prepare_merge` leaves both — as an overwrite and not a refusal,
  because a comparison would refuse every previewed patch under a real clock. Ids stay the
  caller's, since a dry run's ids are the keys a pending edit is applied by. **What remains, and
  is not a defect to fix:** a project whose log already contains a forged entity is read exactly
  as it stands. The log is append-only and authoritative (ADR 0004), so rewriting one would be a
  worse thing than the entry it corrected; nothing migrates, and no such project is known to
  exist outside the spike's scratch directories.
- ~~**Every MCP client that ends its server by signal leaves `.escri/lock` behind**~~ —
  **closed 2026-09-23 in M3 PR 4** (ADR 0020 §5; ADR 0012 §3, amended). 50 Claude Code sessions
  of 50, reproduced without it: `escribass-mcp` removes the lock when stdin closes and leaves
  it on `SIGTERM`, `SIGINT` and `SIGKILL`, because nothing handles a signal and `ProjectLock`'s
  `Drop` does not run in a process one ended. It still leaves it — no signal handler was added,
  and none would have covered `SIGKILL` or a crash. What changed is the next open: a lock whose
  recorded pid names no process is stale, and `take` replaces it and says which process left
  it; a live pid and a missing pid refuse as before. **What remains:** the check is `/proc`, so
  a macOS or Windows build refuses exactly as today until the installer milestone gives it its
  own (M5), and a pid a dead holder's number was recycled into reads as alive — a spurious
  refusal, never two writers, because a process records its own pid when it takes the lock.
- ~~**A real export of `tests/determinism/render` crashes the engine**~~ — **closed 2026-09-15 in
  M2 PR 11.** `corrupted double-linked list`, killed by a signal, after Rubber Band warned that the
  ideal input hop of 0.0853 was below its minimum. PR 11's review first recorded it as not
  reproduced — about twenty exports, all exit 0 and one hash — and that was a harness defect, not
  an intermittent crash: its driver omitted `--seed-ids`, so the script's literal ids named nothing,
  most of its steps were refused unread, and the stretched clip was never added. Driven with seeded
  ids, as the determinism suite drives it, it crashed **every** time — six exports and one preview
  in seven, with no load and no heap checking needed. The cause is the library at a ratio it cannot
  make, measured under AddressSanitizer against `RubberBandSingle.cpp` alone: the fixture stretches
  four samples over 24,000 frames, a ratio of 6000; past 512 at 48 kHz R3 clamps its input hop to
  one sample, its output hop becomes the ratio, and past 4096 `R3Stretcher::synthesiseChannel`
  writes beyond a 4096-sample accumulator. The engine now hands the stretcher a logger and refuses
  the clip on anything it says, before the first `process` — `engine_failed`, exit 2, the library's
  own sentence — on the export and the preview path alike (ADR 0011 §3, extended;
  `tests/renders.rs`, `a_stretch_rubber_band_cannot_make_is_refused_rather_than_corrupting_the_heap`).
  No golden stretches past 2, which is how four goldens stayed green over it; the fixture's own
  `render_export` is a dry run and stays one, since the stretch it asks for is now a refusal.

- **`Project::write` rewrites every entry file on every commit** — O(history) I/O per call.
  ~~Invisible while histories are short; the tool API is what will make it visible.~~ That
  trigger has now passed and the gap did not: M1's render fixtures drive the longest scripts in
  the repository through the tool API and the suites still finish in seconds, because a script
  is tens of entries and a process is one project. What would make it visible is a session that
  stays open and keeps appending, which is `app`. ~~Revisit at M2.~~ **Revisited in M2 and
  measured, not fixed**: an applied `set_notes` costs 17 ms at 21 entries and 30.6 ms at 300,
  and 300 in a row take 6.7 s (ADR 0017 §5). ADR 0017 §1 keeps a drag from multiplying it — one
  entry per gesture — and the loop in `Project::write` is unchanged. What would make it a defect
  rather than a number is a project whose history is long enough for one commit to be felt,
  which nothing has produced; ~~M3's loop, which commits without a person between calls, is the
  next thing that could.~~ **Re-measured 2026-09-24 in M3 PR 8, and the loop turned out not to
  be that thing**: a proposal commits **once**, at approval, whatever the number of model calls
  (ADR 0019 §1). Six proposed calls at 21 entries cost **6.1 ms** and no write at all; the apply
  that followed cost **3.9 ms** — one `Project::write`, the same one a person's single edit
  pays. The design that *would* have made this a defect is the branch-per-proposal ADR 0019
  rejected, which writes the whole log per model call. Trap 9 is therefore closed as a trigger
  and the row stays open as a number: what could still make it a defect is a long history, and
  nothing in M3 produces one. The loop adds one write a turn outside the log — `record_ai`, on
  the first prompt in a project — and it writes `lock.json` alone for exactly this reason.

- **`Preview::drop` discards the engine's exit status.** `core/src/engine.rs` closes the
  stream, waits for the process to leave, kills it if it will not, and `let _ = self.child.wait()`
  — so a preview engine that crashes after answering its last command exits non-zero into
  nothing, and the caller learns only that the next play starts a fresh process. Found by M2
  PR 11's review and not fixed there; recorded at the close rather than fixed there either,
  because a close is reviewed as documentation. The render path is not affected: an export's
  exit status is its verdict (ADR 0013 §3). **Still open, and deliberately not repeated in the
  sibling**: M3 PR 5's `Sidecar` reads the status the moment the child is gone, keeps it with
  the tail of its stderr, and hands it back from `stop` as well as from the dot — which is what
  ADR 0020 §5 asked for in as many words. Doing the same for `Preview` is a different change:
  a drop has nobody to tell, so it wants somewhere to put the sentence, and the window's
  transport is where it would go.

- **A log written by the pre-PR-11 `undo` is read as it stands.** Before M2 PR 11 a fresh
  session's ⌘Z could re-apply an edit and record it under the tool name `undo`; a project whose
  log carries one is not repaired, because the log is the record and rewriting it is what
  ADR 0001 refuses (ADR 0005 §4, amended). Replaying such a log as an undo stack treats that
  entry as an undo, which is what the log says it is. No project in the repository carries one.
- ~~**No lock file on an `.escri` directory.**~~ ADR 0001 §2 assumes a single writer and ADR
  0004's commit is three renames; two processes on one project would race them. M0.3 made it
  structural (one project per process, ADR 0006 §5) rather than enforced, and M1's second
  process did not change that — the engine is handed an asset to read and a WAV to write and
  never opens a project directory at all (CLAUDE.md #6, ADR 0007 §2). **Decided 2026-09-07 and
  closing in M2 PR 2** (ADR 0012 §3, amending ADR 0006 §5): `core` takes `.escri/lock` with
  `create_new`, holding the pid, released on a clean close, reported and never broken
  automatically. The bargain that buys is git's `index.lock` bargain — a crashed process leaves
  a project that will not open until someone removes a file — and it is preferred to two
  interleaved renames and a log that no longer matches the `song.json` beside it.

## Open — not ours to decide

`docs/specs.md` §15 marks these `[OPEN]`; `CLAUDE.md` says stop and ask. None blocked M0, M1 or
M2. **Two are in M3's path**, and M3's plan lists both as questions for the user — U1 and U2 —
and takes neither: whether §6's analysis and symbolic generation are v1 at all decides whether
the second is reachable, and the plan is sized on the assumption that they are not in M3, an
assumption the user confirms or reverses before the decisions PR.

Minimum supported OS versions is the one M2 walks into, and it is **still open**. ADR 0014 §2
answers a different question — which platform M2 *targets*, which is Linux x86-64, as M1 — and
that is scope, not a product commitment. What version of any operating system this ships against
outlives M2 and belongs with the installer, where `roadmap.md` already places it (M5). The item
stays in §15's closing paragraph unchanged, and M2 is on record as not resolving it.

~~Also worth a person's attention rather than an agent's, though they are sign-offs and not
`[OPEN]` items: three dependency approvals are recorded in this repository as **given**, and the
repository cannot show that they were.~~ **Resolved 2026-09-17: the user confirmed all three.**
What follows is kept as the record of why the question was asked. ~~protobuf v21.12 has carried "pending sign-off" since
M1 PR 5~~ — M2 PR 2 (#56) changed those words to "signed off 2026-09-07" in §15, in §17's table
and in `lock.baseline.json`, and its commit message says the user said so. grpc++ v1.54.3 is
recorded as "approved as a dependency under CLAUDE.md #4" (ADR 0013 §1), and it brings that
same protobuf commit as its own submodule, so the first sign-off is load-bearing for two things.
The frontend package set is recorded as "one sign-off" (ADR 0016 §2; §15; `lock.baseline.json`).
An agent cannot give any of the three and cannot verify any of the three from what is committed;
M2's close (2026-09-17) walked past them deliberately — neither removed, reworded nor confirmed
— and listed them here for the user to confirm or retract. The user confirmed all three on
2026-09-17, so they are decisions rather than claims.

- Neural runtime packaging: ONNX Runtime linked into `engine`, or a separate process. Now due
  before M4, which is where the neural runtime lands (ADR 0003 §7).
- Minimum supported OS versions.
- Symbolic model choice for v1 melody and drum generation.
- Whether §6's analysis features (key, chord and structure detection, tempo estimation, stem
  separation) and symbolic generation are v1 scope at all (ADR 0003, Still unplaced).
