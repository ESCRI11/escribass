# Delivery plan

Status as of 2026-09-07. This file tracks **state**: what is done, what is next, and what
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
| M2.3 | `render.proto`'s whole M2 shape — `Preview` and `mix_lanes` — and TypeScript for `proto/` | done | this PR |

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
state between renders, which is the cheapest determinism guarantee available.

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
| 5 | `m2.5-edits` | The first control that is a tool call, its dry-run and diff, and the undo/redo tools behind ⌘Z. Where trap 2 gets decided against a real drag |
| 6 | `m2.6-mix-automation` | `ParamRef` reaching a fader: the validator arm, `param_out_of_range`'s two domains, `compile` emitting `mix_lanes`, and the engine driving Tracktion's track volume and pan with **our** curve formulas (ADR 0015 §1, §2). No `song.proto` change, so no M0.4 goldens move; the render goldens must not move either, and any byte that does needs a named cause |
| 7 | `m2.7-mixer-and-editors` | The mixer over the automation PR 6 laid down, and §9's seventh view: the generic parameter editor over the build manifest (ADR 0014 §1). Two forms over two maps, in one PR because they are the same form |
| 8 | `m2.8-history` | The patch-log view with its provenance column, branch switching, and `merge_branch`'s per-path resolution (ADR 0015 §3) |
| 9 | `m2.9-engine-grpc` | `Render` over gRPC, the stdio path deleted, `core/src/engine.rs` and the render suite moved onto the new transport, and gRPC added to `tests/renders.rs`'s `COMPONENTS` and the engine job's list. **The silent PR**: the four goldens must not move, and any byte that does needs a named cause (trap 5) |
| 10 | `m2.10-preview` | Preview playback: the live process, the audio device, the `Preview` stream, and `render_preview` — one of §5's tools, and the milestone it has been between since M0.3 |
| 11 | `m2.11-review-fixes` | A whole-stack review's findings. M0 averaged four to sixteen per milestone and M1 returned eighteen; budgeting a PR for it is cheaper than discovering it |
| 12 | `m2.12-close` | Docs walked against the code, the deferred ledger walked again, `CLAUDE.md` to M3 |

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
2. **A drag is not one tool call.** Dragging a note fires a hundred pointer events. One
   `set_notes` each puts a hundred entries in the log §5 calls both the audit trail and the undo
   history, and ⌘Z then undoes one pixel. Coalescing on release is the obvious fix and has a
   sharp edge: the intermediate states are unvalidated, so a drag can pass through a position
   the validator would refuse and land somewhere legal, and the refusal a user should have seen
   at the boundary never happens. **Deliberately not decided**: there is no answer right for
   every gesture, and PR 5 decides it against a real drag rather than an imagined one.
3. **Preview and export do not agree, and that is correct.** sfizz switches to freewheeling
   quality settings for an offline render, so §8 already says a golden is deliberately not what
   a preview plays; Surge XT's factory patch reaches a wall-clock RNG unless `A Osc 1 Retrigger`
   is set; and the audio device's sample rate is the user's while the render's is
   `RenderTarget`'s, which puts a resampler in one path and not the other. A UI that publishes a
   hash and plays a different sound will be reported as a bug by the first person who checks.
4. **The status bar is a `[MUST]` rendered as a widget.** Plate 1 shows the `song.json` hash,
   the patch count, the last render hash and `lock.json 14/14 verified`; Plate 6 shows "0
   differing samples outside the edited range". Every one of those is a §11 claim, and a widget
   that computes it a second way is a second implementation of the thing the suite exists to
   check. They come from the same code or they drift, and the drift is invisible until a demo.
   PR 2 is on record as showing none of them for exactly this reason.
5. **Deleting stdio deletes the only tested path.** `core/src/engine.rs`, `render_export` and
   the four goldens in `tests/renders.rs` all reach the engine over stdio today. ADR 0008 §1 is
   explicit that both are not kept, so the order matters: implement, move the suite, delete —
   never a window in which the goldens are compared through a transport nothing has exercised.
6. **A live engine is a resident plugin instance, which is the thing ADR 0008 §2 refused.**
   **Answered:** ADR 0013 §3 keeps preview and export in different processes, so an export never
   inherits a smoother. The trap survives as the thing that must not be optimised away — one
   process serving both is the obvious saving, and it is the one that cannot be taken.
7. **The webview cannot be pinned.** §17 pins by commit or exact version; the engine the
   frontend renders in ships with the operating system, moves under the user, and paints a
   canvas differently across versions. **Recorded rather than solved:** §17's Tauri row says so,
   and ADR 0012 §5's golden is a description of what a view would draw and never a screenshot,
   which is what keeps the trap out of CI.
8. **`Project::write` is O(history) and `app` is the first long session.** The known gap names
   this trigger exactly: "a session that stays open and keeps appending, which is `app`". Every
   edit rewrites every entry file. It will present as UI lag, be diagnosed in the frontend, and
   live in `core`. PR 5 is the first PR that appends in a loop.
9. **Two writers, and nothing locks the directory. Answered:** ADR 0012 §3 takes `.escri/lock`
   in PR 2, with `create_new`, reported and never broken automatically. What is left is the
   failure mode that choice buys — a crashed process leaves a project that will not open until
   someone removes the file — which is git's `index.lock` bargain and is taken deliberately.
10. **Undo is not a stack.** ADR 0005 §4: undo appends an inverse entry and never rewinds a ref.
    Every editor framework ships an undo stack, and one here disagrees with the log the moment a
    branch is switched or a second writer commits. Plate 5's entire point is that there is no
    second stack. ADR 0016 §3's empty dependency list is half the defence; the other half is that
    ⌘Z calls a tool.
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

| Item | Why deferred | Revisit at | Source |
|---|---|---|---|
| `FormRule` | Least-specified entity in §4; nothing consumes it before the generative compiler | M4 | ADR 0002 §7 |
| `Instrument.state` as a content hash instead of inline `bytes` | Plugin states are large base64 in a file §2.6 wants diffable — but adding a hash field and deprecating `state` is additive, not breaking. ~~Revisit before M1 renders a plugin~~ — that trigger passed at PR 7 and M1 re-judged it: nothing in M1 *writes* a state. ~~M2, with the first plugin editor~~ — **retriggered 2026-09-07**: M2 *has* an editor (ADR 0014 §1) and it still writes no state. A generic parameter editor writes `params`, a map from `ParamID` to a normalised double; `state` is the plugin's own opaque blob and only the plugin's own serialisation produces one. So there is still no producer, which is ADR 0002 §7's reason unchanged | The first thing that **writes** an `Instrument.state`: a plugin's own VST3 editor, or a preset import. Neither is M2's | review, 2026-09-02; re-judged M1; retriggered by ADR 0014 §3 |
| ~~`ParamRef` reaching track mix params (gain, pan, mute)~~ | **Closed 2026-09-07 by ADR 0015 §1** — and it cost no schema change at all. Ids are already globally unique across collections (ADR 0001 §3), so a `ParamRef` whose `device_id` resolves to a track addresses `Mix.gain_db` and `Mix.pan` with no field added. `mute` and `solo` stay unaddressable on purpose: a double automating a boolean needs a threshold rule that would be ours and pinned | closed — M2 PR 6 | review, 2026-09-02 |
| Dense unique `index` on tracks and effects | Inserting mid-list renumbers everything, and two branches inserting at one index auto-merge into an invalid document. Deferred again at M0.3: the merge pipeline makes that failure loud (the validator refuses it) rather than silent, and closing it properly is a `song.proto` change with its own ADR. ~~M2, with the mixer~~ — **deferred again 2026-09-07** (ADR 0015 §4): the mixer is a milestone, not an event, and a mixer that draws a chain in order and adds at the end never renumbers. M2 is on record as offering no reorder gesture | The first gesture that reorders an effect chain or inserts a track mid-list | review, 2026-09-03; ADR 0015 §4 |
| ~~Interactive merge conflict resolution~~ | **Closed 2026-09-07 by ADR 0015 §3**: one optional per-path resolution on a second `merge_branch` call. No new tool and no merge state held between calls, which is the shape ADR 0006 §5 refused for projects | closed — M2 PR 8 | ADR 0001 §4 |
| Garbage collection of orphaned patch entries | Entries are small and inert | only if a real project makes it a problem | ADR 0001 Deferred |
| `SourceRef.export_hash`, `Generator` compiled-source hash | Needed for "export pending" and "compiled · stale"; nothing produces either yet | M4 | ADR 0002 Consequences |
| Strudel as a second `Generator.kind` | Python DSL is the v1 target | after M4 | §15 |
| `schema/pyproject.toml` `[build-system]` | Consumers use `sys.path`; no wheel needed yet | when `ai/` depends on it | `schema/AGENTS.md` |
| Native CLAP hosting | VST3 via clap-wrapper is the mature path | never a dependency | §8 |
| User VST3 plugins | §8 says "VST3 host" and §16 never says user plugins, so nothing places them. M1 refuses a plugin outside the bundled manifest, which makes the gap loud rather than silent. ~~M2, when `app` could show a plugin browser~~ — **retriggered 2026-09-07**: M2's editor is a *view over* the build manifest, and the manifest describes what this build hosts. A browser is not a view over one; it is a scanner that puts things into one, and it drags `lock.json` with it, since a user's plugin has no submodule commit to pin | When the build manifest can describe a plugin this build did not bundle | M1 planning, 2026-09-04; retriggered by ADR 0014 §3 |
| `RenderTarget.tail` for release tails | A render ends at the last clip or section. Every golden controls its own content, so this does not affect the determinism claim — it affects whether a real export sounds truncated. **Measured in PR 12**, so the trigger is no longer abstract: a half-bar note's release runs about 5,800 frames (0.12 s) past its note-off on this build, and that is what a render ending at the last clip cuts off | when someone exports something with a long release | M1 planning, 2026-09-04 |
| Recursive merge, for a criss-cross base | Two branches that each merge a third leave `merge_base` with no single answer, and it refuses rather than guessing which history is the truth. The fix is to merge the bases and use the result. ~~M2~~ — **deferred 2026-09-07** (ADR 0015 §3): refusing is the current behaviour and it is loud, and *nothing in the repository produces a criss-cross history yet*. M2's history view is what makes branch merging ordinary enough for one to appear | A real criss-cross base | review, 2026-09-03; ADR 0015 §3 |
| Undo/redo **tools** | ADR 0005 §4 settles the mechanism — an inverse entry, never a rewind. The tools themselves have no consumer until ⌘Z exists | M2, PR 5 | ADR 0005 §4 |
| `lock.json` beyond `schema_version` | ~~Nothing to pin until compiled artefacts and models exist~~ — the M1 half is **closed** in PR 9: the engine's submodule commits and one entry per referenced plugin. What is left is M4's, the compiled artefacts and model hashes | M4 | ADR 0003 §3; §17 |
| Refusing a plugin parameter that reaches an RNG nothing can seed | §8 forbids Surge XT's `rand_pm1`, Dexed's LFO waveform 5 and sfizz's `*_random` **in a fixture**, and nothing refuses them in a user's song. §11's first bullet is about our own code and holds; §2.2's promise — "every source of randomness carries an explicit seed stored in the project" — is wider, and neither of those two RNGs can be seeded at all (Surge's is the wall clock with `seed_rand` commented out; Dexed's `randstate_` is indeterminate memory). Closing it is a validator rule and therefore an ADR — and the rule has no producer: something must say which parameter values reach an unseedable RNG, and today that is prose in §8 for three plugins vetted by hand. Deferred rather than opened as an M2 question because M2 adds no plugin and no randomness: the gap is M1's, unchanged, and an ADR now would design a denylist against a build manifest that carries none, which is ADR 0002 §7's reason ~~The first milestone that lets a user *choose* a patch or supply a plugin~~ — **retriggered 2026-09-07** (ADR 0014 §3). That trigger has now fired: M2 places a parameter editor and choosing a patch is exactly how a user reaches those RNGs. It fired and the work still cannot be done, and the blocker is the second fact, not the first: closing it needs a **denylist of `ParamID`s per plugin in the build manifest**, and §8's prose names the paths in English — "oscillator random start phase, unison detune, the sample-and-hold LFO shape, and the effects that call `rand_pm1`" — which becomes `ParamID`s only by auditing Surge XT's 2855 parameters against its source. That is a source audit, and no user interface produces one. Half a denylist is worse than none, because it looks complete | A build manifest that can say which parameters reach an unseedable RNG | §8's per-plugin notes; M2 planning, 2026-09-07; retriggered by ADR 0014 §3 |

## Known gaps

- **`Project::write` rewrites every entry file on every commit** — O(history) I/O per call.
  ~~Invisible while histories are short; the tool API is what will make it visible.~~ That
  trigger has now passed and the gap did not: M1's render fixtures drive the longest scripts in
  the repository through the tool API and the suites still finish in seconds, because a script
  is tens of entries and a process is one project. What would make it visible is a session that
  stays open and keeps appending, which is `app`. Revisit at M2.
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

`docs/specs.md` §15 marks these `[OPEN]`; `CLAUDE.md` says stop and ask. None blocked M0 or M1,
and none blocks M2.

Minimum supported OS versions is the one M2 walks into, and it is **still open**. ADR 0014 §2
answers a different question — which platform M2 *targets*, which is Linux x86-64, as M1 — and
that is scope, not a product commitment. What version of any operating system this ships against
outlives M2 and belongs with the installer, where `roadmap.md` already places it (M5). The item
stays in §15's closing paragraph unchanged, and M2 is on record as not resolving it.

Also worth a person's attention rather than an agent's, though it is a sign-off and not an
`[OPEN]` item: **protobuf v21.12 has carried "pending sign-off" since M1 PR 5**, in §15, in
§17's table and in `lock.baseline.json`'s own note. grpc++ v1.54.3 is approved and it brings
that same protobuf commit as its own submodule (ADR 0013 §1), which makes the outstanding
sign-off load-bearing for two things instead of one. Nobody has removed the words; nobody
should, unsigned.

- Neural runtime packaging: ONNX Runtime linked into `engine`, or a separate process. Now due
  before M4, which is where the neural runtime lands (ADR 0003 §7).
- Minimum supported OS versions.
- Symbolic model choice for v1 melody and drum generation.
- Whether §6's analysis features (key, chord and structure detection, tempo estimation, stem
  separation) and symbolic generation are v1 scope at all (ADR 0003, Still unplaced).
