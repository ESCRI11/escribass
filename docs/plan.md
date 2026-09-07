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
| M0.4 | Determinism suite in `tests/` | done | PRs #29–#36 |
| — | **M0 complete.** Schema, core, tool API, determinism suite | done | — |

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
| 10 | `m1.10-render-export` | `Session::render_export` and `add_asset` over both transports |
| 11 | `m1.11-goldens` | `tests/renders.rs` behind a feature; A-vs-B and golden WAVs, including an audio clip |
| 12 | `m1.12-locality` | The bar-17 demo as a test |
| 13 | `m1.13-review-fixes` | Whole-stack review findings — M0 averaged four to sixteen per milestone |
| 14 | `m1.14-close` | Docs, the §11 line checked, `CLAUDE.md` to M2 |

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
8. **The suite validates a stale engine** — M0.4's exact defect, one language over. The engine
   embeds the submodule commits it was built from and the suite compares them.
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
    with no PR to blame.
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
    golden that now contains an asset changes what the determinism suite compares.

### Deferred again, with reasons

`Instrument.state` as a content hash: nothing in M1 *writes* a state — fixtures use factory
defaults plus `params` — so an ADR now would design against no producer, which is ADR 0002 §7's
reason. Revisit at M2 with the first plugin editor.

Render tail: `length_ticks` ends at the last clip or section. A `RenderTarget.tail` field is a
`song.proto` change with its own ADR when someone wants release tails.

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
| User VST3 plugins | §8 says "VST3 host" and §16 never says user plugins, so nothing places them. M1 refuses a plugin outside the bundled manifest, which makes the gap loud rather than silent | M2, when `app` could show a plugin browser | M1 planning, 2026-09-04 |
| `RenderTarget.tail` for release tails | A render ends at the last clip or section. Every golden controls its own content, so this does not affect the determinism claim — it affects whether a real export sounds truncated | when someone exports something with a long release | M1 planning, 2026-09-04 |
| Recursive merge, for a criss-cross base | Two branches that each merge a third leave `merge_base` with no single answer, and it refuses rather than guessing which history is the truth. The fix is to merge the bases and use the result — the same shape as the interactive resolution already deferred there | M2 | review, 2026-09-03 |
| Undo/redo **tools** | ADR 0005 §4 settles the mechanism — an inverse entry, never a rewind. The tools themselves have no consumer until ⌘Z exists | M2 | ADR 0005 §4 |
| `lock.json` beyond `schema_version` | ~~Nothing to pin until compiled artefacts and models exist~~ — the M1 half is **closed** in PR 9: the engine's submodule commits and one entry per referenced plugin. What is left is M4's, the compiled artefacts and model hashes | M4 | ADR 0003 §3; §17 |

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
