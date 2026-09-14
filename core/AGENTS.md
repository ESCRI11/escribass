# AGENTS.md — core/

Crate `escribass-core`. Everything that reads or writes song state. Model types come from
`escribass-schema` and are never redefined here (specs §4.1). Repo-wide rules: `/AGENTS.md`.
The schema itself: `/schema/AGENTS.md`.

## Files

| Path | What | Status |
|---|---|---|
| `src/canonical.rs` | Canonical JSON persistence: `to_canonical_json`, `from_canonical_json`, `CanonicalError`. | done |
| `tests/canonical.rs` | Fixed-point, idempotence, non-finite rejection, seeded double round-trip, key order. Reads `/tests/fixtures/song/minimal.json`. | done |
| `src/patch.rs` | RFC 6902 `apply` and `diff`, and the RFC 6901 pointers they address with. Operates on `serde_json::Value`; knows nothing about `Song`. | done |
| `src/history.rs` | The on-disk shape of `patches/*.json` and `refs.json`, the ADR 0001 §2 ref-name rules, and `History`: the DAG, `ancestry`, `materialise`, refs and `switch`. No I/O. | done |
| `src/manifest.rs` | `Manifest`: what one engine build can host, as `escribass_engine --scan` wrote it. Read, never written. | done |
| `src/validate.rs` | `validate(song, manifest)` → every `Violation` (path, stable rule id, message), not just the first. §4.4 plus the rules in ADR 0002 Consequences. | done |
| `tests/validate.rs` | One rule per test, each breaking the fixture in exactly one way. | done |
| `tests/common/mod.rs` | What more than one suite shares: the manifest fixture, the shell script that stands in for the engine, a real `Render` server for it to name, and a model of the engine's side of the `Preview` stream. Not a test target — a module. | done |
| `src/project.rs` | The `.escri` directory: `open`, `write`, `lock.json` v2 and its pins, atomic writes, and `ProjectLock` — the `O_EXCL` directory lock a process holds while the project is open, never broken automatically (ADR 0012 §3). Reads and writes only. | done |
| `Project::create`, `Project::commit` | The mutation entry point. `commit` is `prepare` then `record`. | done |
| `Project::prepare`, `Project::record` | The two halves: `prepare` applies, bumps, validates and re-derives the patch, touching nothing; `record` mints the id, appends, advances and writes. A dry run *is* `prepare`. | done |
| `src/tools.rs` | The typed tools of §5, as pure functions from arguments to operations. No I/O, no validation of what §4.4 already covers. | done |
| `tests/tools_devices.rs` | `add_track`, `set_track_instrument`, `add_effect`, `set_param`, driven through `Session`. | done |
| `tests/tools_clips.rs` | `add_clip`, `set_notes`, `transpose`, `quantize`, `add_automation`, `set_tempo`, the two section tools. | done |
| `src/merge.rs` | Three-way conflict detection by RFC 6902 path (ADR 0001 §4). No content-aware resolution: a structured error is actionable, a silent choice is a song nobody wrote. | done |
| `tests/tools_merge.rs` | `merge_branch`: both sides of the conflict line, `version` resolution, and the auto-merge that produces an invalid song. | done |
| `tests/tools_branches.rs` | `create_branch`, `switch_branch`, `delete_branch`. Mostly one claim from several angles: leaving a branch and coming back is byte for byte. | done |
| `src/grpc.rs` | The gRPC surface: the generated `SongTools` trait over the same `Session`. Translation only. | done |
| `src/bin/escribass-grpc.rs` | The server binary. `--manifest` is required. Loopback and no TLS — the service edits local files with no authentication. | done |
| `tests/grpc.rs` | Over a real socket with the generated client: the `Ok(valid=false)` / `Status` line, and the two MCP hazards that do not arise here. | done |
| `src/call.rs` | `call(session, name, args)`: the one dispatch from a tool name and a JSON object to a `Session` method, and the two byte-level exceptions ADR 0006 §6 names. Carrier-independent — MCP and the Tauri command are both envelopes around it (ADR 0012 §1). | done |
| `tests/call.rs` | The same script through both carriers — a real `escribass-mcp` subprocess and `call` in-process — compared answer by answer. What makes "one dispatch" a claim rather than a comment. | done |
| `src/mcp.rs` | The MCP surface: `ServerHandler`, the advertised tool list, and the `inputSchema` rewrite the JSON-text exception needs. A JSON-RPC envelope around `call` and nothing else. | done |
| `src/bin/escribass-mcp.rs` | The server binary. Project and `--manifest` are launch arguments; `--seed-ids` / `--fixed-clock` make a session reproducible. | done |
| `tests/mcp.rs` | Driven as a real subprocess over real pipes — where this layer's failures actually live. | done |
| `src/descriptor.rs` | `tool_schemas`: the protobuf descriptor turned into one JSON Schema per tool (ADR 0006 §6). Proto3 JSON's own mapping, with proto field names. `message_fields`: the same index read out as messages and fields, for the coverage guard. | done |
| `tests/descriptor.rs` | Correspondence with the proto, one mapping rule per test. Every failure here is otherwise silent. | done |
| `src/render.rs` | `compile`: a valid `Song` and an index from asset hash to path → the `RenderPlan` the engine renders, or every `Violation` (ADR 0007 §4). Pure; every order from a stated rule. | done |
| `tests/render.rs` | What the plan resolves and what M1 refuses, one claim each, every song built through `Session`. | done |
| `tests/render_coverage.rs` | The field-coverage guard (ADR 0007 §5): every `song.v1` field is carried, consumed, or ignored with a reason, and a field on no list fails. | done |
| `src/engine.rs` | `Engine`: where the engine binary is, and how a caller reaches it over gRPC (ADR 0013 §3) — one fresh process per `render`, failure is an exit code; and `Preview`, the one live process a session plays through, with the stream to it (M2 PR 10). Every failure it returns is an operator's. | done |
| `tests/engine.rs` | `render_export` from both sides of ADR 0006 §2's line, driven against a fake engine that is a shell script. What a *real* engine does is `tests/renders.rs`'s. | done |
| `tests/preview.rs` | `render_preview` from both sides of that line: the process and its mode, one process across commands, the plan a play carries, answers told from the transport's own events, refusals sent nowhere, a machine with no device, a stream the engine ends, the process leaving with its stream, and an export beside a preview in a process of its own. Against a model of the stream — what a device does is `tests/renders.rs`'s ignored test. | done |
| `src/session.rs` | The tool API, implemented once: `Session`, the reads, `apply_patch`, the summary, `-0.0` normalisation on input, and `new_song`. Every carrier dispatches here and decides nothing. | done |
| `tests/session.rs` | Dry run equals the recorded patch; refusals keep their rule; the `Ok(valid=false)` / `Err` line. | done |
| `src/version.rs` | `bump_versions` (ADR 0005 §2) and `version_writes`, the guard behind `version_not_writable`. Operates on `Value`, like `patch.rs`. | done |
| `tests/version.rs` | The rule, one case per test, plus three through the pipeline. | done |
| `tests/undo.rs` | `undo` and `redo` (ADR 0005 §4): the log grows rather than rewinds, no entity's `version` goes backwards, a second press walks one change further, and any other commit or a branch switch clears the cursor. | done |
| `src/id.rs` | `IdSource`: `UlidSource` (production) and `SeededIds` (deterministic). Crockford base32, monotonic within a millisecond. | done |
| `src/clock.rs` | `Clock`: `SystemClock` and `FixedClock`. Every clock yields whole milliseconds. | done |

## Commands

```
export PATH="$HOME/.cargo/bin:$PATH"
cargo test -p escribass-core
```

## Rules

| Rule | Set by |
|---|---|
| Never define a model type here; use `escribass_schema::song` / `::history` | specs §4.1 |
| No unseeded randomness and no wall clock: the id source and the clock are constructor parameters, never globals. `SystemClock` is the only place `core` reads wall-clock time; `UlidSource` the only place it takes entropy | specs §11; CLAUDE.md #3; ADR 0001 §5 |
| Entropy comes from `std`'s `RandomState`, not a crate: ULID's tail is a uniqueness requirement, not a secrecy one. Marked `ponytail:` in `src/id.rs` with the upgrade path | CLAUDE.md #4 |
| Mutations are JSON Patch through the tool API, in tests too — never a direct field write to a stored song | CLAUDE.md #2; specs §5, §14.3 |
| `commit` re-deserialises through `Song` before recording. An op can be legal JSON and illegal for the schema; applying it to a `Value` alone would succeed and produce a document `core` cannot read | ADR 0002 §11 |
| Every check happens before `self` is touched, **and the write happens before it too**: `record` and the branch tools build the next state beside this one and swap it in only once it is on disk. A failed write must not leave the session a commit ahead of the directory | specs §5; ADR 0004 |
| The `version` bump runs inside `prepare`, between applying the ops and re-deserialising — the only position where it lands in the diff `prepare` re-derives. Anywhere later and the log replays one version behind `song.json` | ADR 0005 §1 |
| An entity is an object with a string `id` **and a numeric `version`**. Both halves: `PluginRef.version` is a string naming a plugin release, and bumping it would break `set_track_instrument` | ADR 0005 §2 |
| `bump_versions` has **two modes**: an ordinary edit bumps by one and ignores the number in the value; a merge resolves to `max + 1` and disputes nothing. One rule serving both collapsed when the branches were adjacent | ADR 0005 §2, §3 |
| A caller that asks for a different `version` than core computes is refused with `version_not_writable`. The guard is a **comparison**, not a filter on op paths: a path filter cannot see a version inside a whole-entity value, cannot tell `"1"` from `1`, and refuses this API's own output | ADR 0005 §3, revised |
| `prepare` fails with `Vec<Violation>`, `record` with `ProjectError`. That *is* ADR 0006 §2's line — `prepare` touches no file, so every way it fails is caller-fixable — so the session needs no classifier over rule names | ADR 0006 §2 |
| `-0.0` is normalised in `Session::run`, through which every mutating tool passes. Normalising only in `apply_patch` left the validator's `negative_zero` firing on typed-tool input, which it should never do | ADR 0002 §4 |
| Tool schemas are derived from `escribass_proto::DESCRIPTOR`, never hand-written. A hand-written schema drifts, and the only symptom is a model that never learns a field exists | ADR 0006 §6 |
| A tool with no proto comment has no description. `tests/descriptor.rs` fails on one, so documenting an RPC is not optional | ADR 0006 §6 |
| `ToolResult.patch` and `PatchEntry.ops` are `bytes`, so the generated serde impl base64s them. MCP payloads are built field by field, with the patch parsed into a JSON array | ADR 0006 §6 |
| A song sent to any carrier is rendered by `to_canonical_json`, never `serde_json::to_value` — the text half carries the model's field order | `src/call.rs`; `src/canonical.rs`; ADR 0002 §4 |
| Nothing but the transport writes to stdout in `escribass-mcp`. One `println!` desynchronises the JSON-RPC stream; diagnostics go to stderr | §18.2 |
| `mcp::IMPLEMENTED` advertises only tools that are wired up. Advertising one that is not spends a model's turn on a call that can only fail | ADR 0006 |
| A tool checks only what the validator structurally cannot: that an argument names something in *this* song, and that a double is finite before it becomes a JSON number. Everything else is §4.4's | specs §5 |
| A tool builds entities at `version: 0` and lets the pipeline take them to 1. Setting a version in a tool would be writing a field core owns | ADR 0005 §2, §3 |
| Tools emit `add`, not `replace`: RFC 6902 `add` on an existing object member replaces it, so one op covers both cases and no tool has to ask the document which it is in | RFC 6902 |
| An instrument track is created *with* its instrument. ADR 0002 requires `instrument` present exactly when `kind == INSTRUMENT`, so a two-call sequence would pass through a document §4.4 refuses | ADR 0002 |
| Merge compares **re-derived diffs**, never a tool's own ops, and `diff` recurses to leaves — so granularity comes from the diff rather than from how a tool phrases itself. `transpose` and `quantize` still emit one op per note because that is what they mean, not because merge needs it | ADR 0001 §4; `src/patch.rs` |
| `quantize` is integer arithmetic with ties rounding up. A float round would depend on the platform's rounding mode exactly where a deliberately placed note sits | specs §11 |
| A tool refuses an id it does not recognise rather than skipping it. A caller whose misspelled id returns success has been told its edit landed when it did not | specs §5 |
| `History::patch_to` takes `&self` and `History::switch` takes `&mut self`. A dry-run switch built on the mutating one would move `HEAD` in memory, and the next commit would land on a branch nobody chose. The borrow checker is the guard, not a test | ADR 0001 §2; ADR 0006 §3 |
| The branch tools append no entry, so they skip `run` entirely. `entry_id` stays empty because a branch operation is not a commit | ADR 0001 §2 |
| Merge compares op **paths**, and an *entity's own* `version` is excluded — `PluginRef.version` is a string pin and two branches pinning differently is a real conflict. Both branches bump `version` on every entity they touch, so counting it would make every merge conflict by construction | ADR 0001 §4 |
| A merge goes through `prepare_merge`, not `prepare`: its ops legitimately carry entity versions, which are core's own and are what ADR 0005 §2 resolves to `max + 1`. Stripping them would let a client's version go backwards | ADR 0005 §2, §4 |
| Every minting tool builds its patch from `self.ids.fork()`; only a real apply installs the advanced fork. A preview that burned an id would return a patch whose ids differ from the ones that land, breaking ADR 0006 §3 exactly where §9 has a person approve a diff | ADR 0006 §3 |
| Replay follows the **first-parent** chain, not every ancestor. A merge entry's ops are the diff from `parents[0]`, so replaying the other side as well applies its changes twice — invisible for `add`/`replace`, fatal for `remove` | ADR 0001 §1, corrected |
| A tool never `expect`s on serialisation. prost keeps an unknown enum as a raw `i32` and the generated serializer fails on it; a panic there happens inside a transport's lock and takes the server down for every later call | ADR 0006 |
| Both servers recover a poisoned lock rather than propagating it. Nothing can leave a session half-mutated, so one panic must not end the process | ADR 0006 |
| `set_notes` **preserves** provenance and version for a note it recognises and mints them only for a new one. Re-minting rewrites the authorship of every note in the clip on every call — a no-op under a fixed clock, which is why it needs a test that moves one | ADR 0006 §4 |
| A tool schema omits `id`, `provenance` and `version` for an embedded entity: core sets them, and an advertised argument that is discarded teaches a model a contract that is not real | ADR 0006 §4 |
| `apply_patch` over MCP is decoded by hand, so it validates by hand. `as_bool().unwrap_or(false)` read `"true"` as false and applied a request meant as a preview | ADR 0006 §3 |
| A call that changes nothing records nothing: an entry with no operations claims something happened | specs §5 |
| `undo` and `redo` go through `prepare_merge`, not `prepare`, and they are the second caller to need it: their ops carry entity `version`s read back out of a document core wrote, so the guard would refuse this API's own history. `max(ours, theirs) + 1` then takes every restored entity to one past where it is now, which is what keeps undo monotonic | ADR 0005 §2, §3, §4 |
| The undone-entry cursor is cleared in **`Session::append`**, the one place this session appends, plus `switch_branch`, which appends nothing and still moves the document. A cursor into the first-parent chain means nothing once the chain grows a tip for another reason, and a `redo` then restores a document from before that edit | ADR 0005 §4 |
| `undo` reads the cursor, not `HEAD`, once anything has been undone. Reading `HEAD` again finds the undo entry the previous call appended, and reversing *that* is a redo — the wrong answer to a second press of the same key | ADR 0005 §4, extended |
| The walk **skips the log's own `undo` and `redo` entries**. ⌘Z means the change before this one, not the entry before this one: after edit · undo · redo the log ends with a redo whose parent is an undo, and walking those as edits takes the document *forwards*, because the inverse of an inverse is the thing itself | ADR 0005 §4, extended |
| The engine binary is **told, never searched** (`--engine`), like the manifest — but optional, because only `render_export` and `render_preview` need one. A session told nothing refuses that call as an operator error rather than skipping it, and a dry run still works | ADR 0008 §2; ADR 0010 §4 |
| `render_export` splits at ADR 0006 §2's line and nowhere else: `compile`'s refusals are `valid = false`, and an engine that is missing, crashes or answers with something else is `ProjectError`. A crash reported as a refusal is three retries a model cannot spend | ADR 0006 §2; ADR 0008 §1 |
| The engine is spawned fresh per render, in `--render` mode, and **names its own socket on its stdout**; `core` reads that one line, dials, calls once and waits. The line is printed only once the server is listening, so there is nothing to poll or sleep on. Nothing is pooled or reused: a resident plugin instance makes a render depend on the render before it | ADR 0008 §1, §2; ADR 0013 §3 |
| The verdict on a call that fails is the **child's exit status**, never the transport's. A refused connection, a stream that ends mid-call and a `Status` in place of a message all mean the engine will not answer, and what a person can act on is its exit code and the tail of its stderr: non-zero is `engine_failed`, zero-with-no-answer is `engine_unreadable` | ADR 0006 §2; ADR 0013 §3 |
| stderr is drained on a thread of its own. The render happens while this process is blocked on a call, so a stderr pipe filled by JUCE's and a plugin's chatter would stop the engine mid-render and the call would never return | ADR 0008 §1 |
| A preview is **one live process per session**, spawned in `--preview` mode by the first `play` and held until the session drops, which closes its stream; the engine then stops and exits on its own. It is never what `render_export` uses — an export spawns afresh, whatever is playing | ADR 0013 §3; M2 trap 6 |
| A preview command's answer is the event carrying **its own count** (`PreviewEvent.applied`), not the next event to arrive: the transport writes events while it plays, and one written while a command is on the wire arrives before that command's answer. Every event on the way updates where the transport last said it was, which is what `play` with no tick starts from and what a dry run reports | ADR 0013 §2, amended |
| A preview that ends instead of answering is reported — exit status and stderr, as for a render — and **forgotten**, so the next `play` starts another rather than sending into a dead stream. A seek, loop or stop with none is `preview_idle`, the caller's to fix | ADR 0006 §2; ADR 0013 §2, amended |
| A transport translates and decides nothing. Anything a caller could get two different answers to from gRPC and MCP belongs in `session.rs` | ADR 0006 |
| Never serialise a song through `serde_json::Value`: its `Map` is a `BTreeMap` and sorts struct field names as well as map keys, silently changing the canonical form | `src/canonical.rs` module note; ADR 0002 §4 |
| The document has no arrays — every collection is a map keyed by entity id — so `patch` rejects one rather than implementing index handling that cannot be reached | ADR 0001 §3 |
| RFC 6901 escaping and unescaping live together in `src/patch.rs`; they must stay exact inverses | RFC 6901 |
| Never write `patches/*.json` with the generated `PatchEntry` serde impl: `ops` is `bytes`, so it base64-encodes, producing a file that round-trips and still violates ADR 0001 §1 and §2.6. `src/history.rs` owns the disk form | ADR 0002 §11 |
| Replay starts from a default `Song`, never `{}`: the canonical form emits every no-presence field, so `replace` is legal from the first op | ADR 0002 §4; ADR 0004 |
| The log is authoritative and `song.json` is a derived cache: `open` replays and compares, reporting a mismatch rather than repairing it | ADR 0004 |
| Write order is entries, then `song.json`, then `refs.json` — the ref flip is the commit point, so a crash leaves an inert orphan | ADR 0004 |
| `switch` moves `HEAD` and appends nothing — history that recorded navigation would grow every time somebody looked at a branch | ADR 0001 §2 |
| `diff` recurses to the leaf. A coarse whole-subtree diff would make every pair of edits to one track collide on the same path and conflict under merge | ADR 0001 §4 |
| `serde_json` carries the `float_roundtrip` feature | ADR 0002 §4 |
| The writer rejects non-finite doubles; it does not rewrite values. `-0.0` is the tool API's to normalise, presence the validator's, timestamp precision the clock's | ADR 0002 §4, as amended 2026-09-02 |
| The validator takes the build manifest as an **argument, never an `Option`**, and both binaries refuse to start without `--manifest` (`manifest_missing`). An optional manifest gives `plugin_unknown` and `param_unknown` a silent skip arm, and the determinism suite would then pass on a machine with no engine and prove nothing | ADR 0010 §4 |
| A parameter is named by the plugin's own `ParamID`, not its display name — Surge XT repeats 176 of its 2855 — and a parameter's value is normalised `0..1`. Both are what a VST3 shows a host, and neither is a choice | ADR 0010 §4, refined and extended |
| A device that is not a plugin is **not** judged by the manifest: a Cmajor, Faust or neural device declares its parameters in a source M4 compiles. Unchecked for a stated reason, in one marked place, with a test that says so | ADR 0010 §4 |
| `lock.json`'s plugin block is **monotone**: `write` adds a pin for every referenced plugin that has none and removes nothing. A pure function of the current song loses a pin on an ordinary delete, and ADR 0005 §4's undo would re-pin from the running build | ADR 0010 §2 |
| `open` compares only the pins the song *currently* references, so a stale pin is inert, and refuses a disagreement with `lock_mismatch` — a `ProjectError`, because every fix is an operator action. The `engine` block is the exception and re-pins: one engine, not chosen per project | ADR 0010 §3 |
| A `SamplerRef` is pinned like a `PluginRef`, at the id the manifest's `sampler` names. Its `sfz_hash` pins the patch and says nothing about the build that turns it into samples, and a sampler-only project pinned nothing at all until M1 PR 13 | ADR 0010 §1, corrected |
| A new dependency needs asking first, then a `lock.baseline.json` entry | CLAUDE.md #4; specs §17 |
| Every order in the plan comes from a stated rule — mixer index, `Effect.index`, start tick, tick, parameter name — with ties by id, which is what a `BTreeMap` iterates in under a stable sort. Never from a hash table: a single-run test cannot catch that | ADR 0007 §1; CLAUDE.md #3 |
| `compile` examines only what sounds. A track `mute` or `solo` silences is dropped before its devices, routing and lanes are looked at, so muting the Cmajor track is how a project exports the rest of itself before M4. The render length is the song's regardless: the last clip or section, sounding or not | ADR 0007 §1, §6 |
| `id`, `provenance` and `version` cross the plan empty, and so do `Mix.mute` and `Mix.solo`, because compile has applied them: a `solo` the engine could see is one it could act on. `PluginRef.version` is a pin, not a §4.3 field, and stays | ADR 0007 §2, §5 |
| A field added to `song.proto` must be carried, consumed or ignored *with a reason* in `tests/render_coverage.rs`, or the guard fails. The lists are data; an ignored entry without a reason is where a field that should have rendered hides | ADR 0007 §5 |
| `compile` leaves `output_path` empty: where the WAV goes is `render_export`'s argument, not the document's — and a preview's plan keeps it empty, since it writes no file | ADR 0007 §4 |

## Adding things

- **A rule to the validator:** it belongs in ADR 0002 Consequences first if it is implied by
  the schema shape, or §4.4 if it is architectural. Add a `check_*` in `src/validate.rs` and a
  test in `tests/validate.rs` that breaks the fixture in exactly one way. `rule` ids are a
  stable API — tests and the AI orchestrator match on them, so they do not change with wording.
- **Both of M0.2's structural limits are closed** (M1 PR 9). A `DeviceRef` now resolves against
  the build manifest (`plugin_unknown`) and a `ParamRef` against the plugin's own parameter ids
  (`param_unknown`), because the engine finally says what it can host. What is left unresolved
  is a *non-plugin* device's parameters, which need the compiler M4 brings; that is marked in
  `check_device` and has a test that fails the day it stops being true.
- **A normalisation:** decide where the value *enters* before writing code. The writer is not
  the default answer — see the Rules row above and ADR 0002 §4.
- **A test:** `tests/*.rs`. Anything asserting byte-level output reads the shared fixture
  rather than embedding JSON, so a canonical-form change shows up in one place
  (`/tests/AGENTS.md`).
