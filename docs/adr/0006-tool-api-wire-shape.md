# ADR 0006 — The tool API's wire shape: one result message, one error type, one project per process

- **Status:** Accepted (2026-09-03)
- **Affects:** `proto/song_tools.proto` (new, M0.3); `core/` session, gRPC and MCP surfaces;
  `docs/specs.md` §5, §13, §18.2; `lock.baseline.json`
- **Builds on:** ADR 0002 §11 (patch ops are canonical JSON text, `bytes` on the wire),
  ADR 0004 (the log is authoritative), ADR 0005 (the commit pipeline)
- **Recorded in:** `docs/specs.md` §5, §15 and §17.

## Context

§5 specifies the tool API as a gRPC service `SongTools`, lists representative tools, and says
every one of them takes `dry_run=true` returning `{ valid, errors[], patch, summary }`.
§18.2 makes an MCP server exposing the same tools a hard requirement, and ADR 0003 §2 puts
both in M0.

What §5 does not say is the shape any of it takes on the wire: whether each tool gets its own
response message, how `core`'s four error types reach a caller, where a project's identity
lives when a server has more than one caller, and how a JSON-Schema-driven protocol like MCP
gets its schemas without a second hand-written description of the model.

Each of those has an answer that looks fine and is wrong at the byte level. M0.1 and M0.2 were
each bitten once by exactly that — a `HashMap` in generated code making the patch log
nondeterministic, a `bytes` field base64-encoding itself, `serde_json::to_value` alphabetising
struct fields, proto3 JSON accepting `"64"` for an `int32`. Every one passed a round-trip
test. This ADR fixes the shape before the surface exists, because a wire format is the
hardest thing in the system to change once anything speaks it.

## Decisions

### 1. One RPC per tool; every mutating tool returns a shared `ToolResult`

```proto
message ToolResult {
  bool valid = 1;
  repeated Violation errors = 2;
  bytes patch = 3;        // canonical RFC 6902 text, as ADR 0002 §11
  string summary = 4;
  string entry_id = 5;    // empty on a dry run
}
```

§5 names the first four members; `entry_id` is added because a caller that has just committed
needs to be able to name what it committed — to undo it, to branch from it, or to report it.

One message for every *mutating* tool, rather than sixteen near-identical ones, because every
one of them does the same thing: it produces ops, and the pipeline either records them or
reports why not. Per-tool response messages would be sixteen copies of one shape, differing
only in the name, and each one an opportunity for them to stop being identical.

Reads are not mutations and do not wear the shape. `GetSong`, `GetSongAt` and `GetHistory`
return what they read; `valid`, `patch`, `summary` and `entry_id` have no meaning for them,
and a result whose fields are mostly inapplicable teaches a caller to ignore fields. A read
that cannot be served fails as `Err` — there is no caller-fixable half.

`patch` is `bytes` holding the canonical JSON text produced by the same function that fills
`PatchEntry.ops`. This is ADR 0002 §11 applied at a second boundary: the wire carries the same
canonical *document* the disk does. It is the re-derived diff, so it includes ADR 0005's
version bumps — what a dry run shows is what a commit would record, exactly.

### 2. `Violation` is the only error shape on the wire; the split between "your call" and "the machine" is made once

`core` has four error types — `Violation`, `PatchError`, `HistoryError`, `ProjectError` — and
all four already carry `path`, `rule` and `message`. The wire gets one message with those
three fields, and the other three convert into it. This adds no fifth type; it transports the
one that already exists.

The split is made once, in the session, by return type:

| `Session::call` returns | Meaning | gRPC | MCP |
|---|---|---|---|
| `Ok` with `valid = true` | it worked | `OK` | `isError: false` |
| `Ok` with `valid = false` | the caller can fix this by calling differently | `OK`, `valid = false` | `isError: true` |
| `Err(ProjectError)` | only an operator can fix this | `FAILED_PRECONDITION` / `INTERNAL` | JSON-RPC `-32603` |

Validator rules, patch rules, `op_illegal_for_schema`, `ref_missing`, the `ref_name_*` family,
`delete_head` and merge conflicts are all the first kind. `unreadable`, `unwritable`,
`song_diverged`, `schema_version_mismatch` and `project_exists` are the second.

The line matters because §5 promises "structured errors the LLM can act on" and §6 builds a
three-retry loop on them. An invalid tool call is not a transport failure, and returning it as
one puts it where the retry loop cannot see it. Conversely a corrupt project is not something
a model should retry into.

### 3. `dry_run` stays as §5 names it, and is the first half of the real path

Every request message carries `bool dry_run`. The name and polarity are §5's.

Proto3 gives it a default of `false`, so a caller that omits the field applies for real. That
is the documented behaviour rather than a trap to design around: every mutation is validated
first (§5), reversible through the log (ADR 0001), and the alternative — inverting it to an
`apply` field — would make the wire disagree with the specification that names it, which is
the drift this ADR exists to prevent.

`dry_run` is not a parallel implementation. `Project::commit` splits at the point its own
comment already marks — "nothing above this line touched `self`" — into a pure `prepare` and
a side-effecting `record`. A dry run *is* `prepare`. There is no second code path to drift,
and the test that keeps it that way asserts the `patch` a dry run returns is byte-identical to
`ops_of(entry)` of the entry the same call then records.

### 4. Request messages reuse `song.v1` entity types

Where a tool takes a whole entity, its request embeds the `song.v1` message — `Note`, `Mix`,
`DeviceRef` — rather than a parallel `NoteSpec`. Core overwrites the §4.3 fields (`id`,
`provenance`, `version`) on the way in; a caller cannot set them and is not asked to.

A hand-written request shape mirroring an entity is a second representation of song state
(`CLAUDE.md` #1) wearing a different name, and it fails the way those always fail: a field
added to `song.proto` never reaches the tool, and nothing breaks loudly enough to notice.

### 5. One project per process, named at launch; there is no `open_project` tool

The project directory is a launch argument, and creating one is a flag on the binary
(`--create`), not a tool.

Two reasons converge. MCP's stdio transport is explicit that a server process is not a
session, so any state spanning requests must be an explicit identifier — a project handle
would be that identifier, and inventing one is more surface than M0 needs. And ADR 0001 §2's
single-writer assumption plus ADR 0004's three-rename commit are unsafe under two processes
sharing a directory: a lock file is M2's problem, when `app` supervises the processes. One
project per process makes the constraint structural instead of documented.

The session also owns the `IdSource` and `Clock` — ADR 0001's "session constructor", which
`core/src/project.rs` already points at. `Project::create` and `commit` keep taking them as
parameters, so M0.2's tests are untouched; the session passes its own through. Both binaries
accept flags selecting `SeededIds` and `FixedClock`, which is what lets M0.4 compare two
scripted sessions byte for byte *through a real server* rather than only through the library.

### 6. MCP is served by `rmcp`; `patch` crosses as a JSON array, never base64

The MCP server uses the official Rust SDK (`rmcp`, pinned in `lock.baseline.json`) rather than
a hand-rolled JSON-RPC loop. The protocol is a moving target with a live client population
across revisions; owning that is a maintenance cost with no product value.

Two byte-level rules, both non-obvious and both already proven to bite:

- **`ToolResult.patch` must not be serialised through its generated serde impl.** It is
  `bytes`, and the generated implementation base64-encodes it — the same defect
  `core/src/history.rs` documents for `PatchEntry.ops`. An MCP payload built with
  `serde_json::to_value(&result)` would carry `"patch": "W3sib3AiOi4uLg=="`, which parses,
  round-trips, and passes any test that does not read it. The MCP object is built field by
  field, with the patch parsed from its canonical text into a real JSON array.
- **`get_song` renders `to_canonical_json`, never `serde_json::to_value`.** `Value`'s map is a
  `BTreeMap`, so `to_value` alphabetises struct field names and silently produces a document
  in the wrong field order (`core/AGENTS.md`). Over gRPC this cannot arise; the encoding is
  binary protobuf.

Tool `inputSchema` is generated from the protobuf descriptor set, not written by hand. `buf`
emits the descriptor, `prost_types` decodes it, and a converter maps proto3 JSON types to JSON
Schema. A hand-written schema is a hand-maintained description of the model — it drifts the
first time a field is added, and the only symptom is that the model never learns the field
exists. A test asserts every RPC has a tool and every request field has a schema property.

### 7. `proto/` generates Rust only, until there is a consumer for anything else

`schema/` generates Rust, TypeScript and Python because §14.1 requires the *model* be
available in all three. The service is different: its consumers are M2's Tauri app and M3's
Python orchestrator, neither of which exists. Generating service stubs today would pull
`grpclib` into `schema/`'s Python environment to satisfy nothing.

Rust now; TypeScript at M2; Python at M3. The `.proto` is the artefact that has to be right,
and it is right regardless of who has generated from it.

### 8. `buf lint` exceptions are scoped to the `proto` module

`STANDARD` wants `SongToolsService` (`SERVICE_SUFFIX`), a response message per RPC named after
it (`RPC_RESPONSE_STANDARD_NAME`), and no response shared between RPCs
(`RPC_REQUEST_RESPONSE_UNIQUE`). All three contradict decision 1, and decision 1 is §5's
specified shape.

The exceptions are declared on the `proto` module in `buf.yaml` and cited, rather than
satisfied by sixteen wrapper messages that exist only to make a linter quiet. `schema/`'s
lint configuration is unchanged — the model keeps the full `STANDARD` rule set.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| A response message per tool | Sixteen copies of one shape whose only difference is a name, each free to drift from the others. §5 specifies one result contract. |
| A single `CallTool(name, json_args)` RPC | Throws away the typed API §5 requires and the generated schemas MCP needs, and moves every argument error from compile time to runtime. |
| A fifth error type for the wire | The four `core` types already share `path`/`rule`/`message`; a new one would be a translation layer with nothing to translate. |
| Every error as a gRPC `Status` | An invalid tool call becomes a transport failure, landing outside §6's retry loop — which is the one place it needs to land. |
| Hand-rolled JSON-RPC over stdio for MCP | Zero dependencies, but it makes us the maintainers of someone else's moving protocol, with client-compatibility bugs surfacing as "the server just doesn't work". |
| Hand-written MCP `inputSchema` | A second description of the model, drifting silently the first time a field is added — the exact failure `CLAUDE.md` #1 exists to prevent. |
| A project handle, or an `open_project` tool | More surface than M0 needs, and it invites two writers onto one `.escri`, which ADR 0004's write ordering does not survive. |

## Consequences

- **A new top-level `proto/`** — already listed in `docs/specs.md` §13, so no new-directory
  ADR is needed. `buf.yaml` gains the module; `buf breaking` covers it from that PR onward.
- **New dependencies**, recorded in `lock.baseline.json` per `CLAUDE.md` #4 and §17: `tonic`,
  `tonic-prost`, `tokio`, `rmcp`, `prost-types` as a direct dependency, and the
  `protoc-gen-tonic` codegen plugin. `tonic`'s version is pinned to the line that pairs with
  the existing `prost` 0.14 — a mismatched pair would put a second `prost::Message` in the
  tree and the generated `Song` would not satisfy the codec.
- **`schema/` does not change.** No `.proto` under `schema/` is touched and the song fixture
  does not move, so the TypeScript and Python suites are unaffected.
- **CI** gains `proto/gen` and the descriptor to the codegen drift gate. The plugin cache key
  in `.github/workflows/checks.yml` names its plugin set and must change when
  `protoc-gen-tonic` joins it, or a restored cache will fail `--check` for a missing plugin.
- **M0.4** can drive `core` through a real server process with seeded ids and a fixed clock,
  which is a stronger form of the determinism claim than the library-level one M0.2 proved.
