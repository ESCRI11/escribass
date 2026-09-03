# AGENTS.md — proto/

The tool API's wire contract: `docs/specs.md` §5, shaped by `docs/adr/0006-tool-api-wire-shape.md`.

Every mutation in the system arrives through this service. Agents never read or write the
project file directly (§5), so what is not here cannot be done to a song.

## What is here

| Path | Role | Written by |
|---|---|---|
| `song_tools.proto` | The `SongTools` service, its request messages, `ToolResult` and `Violation`. | hand |
| `buf.gen.yaml` | One plugin pair: `protoc-gen-prost` and `protoc-gen-prost-serde`. Rust only. | hand |
| `codegen.sh` | `buf format -w`, `buf lint`, `rm -rf gen`, `buf generate`. `--check` is the drift gate. | hand |
| `gen/rust/` | Generated output, committed for review (§4.1). **`codegen.sh` deletes `gen/` whole on every run.** Never edit, never add a file under it. | generated |
| `src/lib.rs` | The hand-written module tree that `include!`s the generated file. Nothing else. | hand |
| `tests/contract.rs` | The three things about this file a change could break silently. | hand |

The workspace `buf.yaml` is at the repository root, not here: buf v2 wants one at the common
ancestor of every module, and `song_tools.proto` imports `schema/song.proto`.

```
./proto/codegen.sh            # regenerate; run from anywhere
./proto/codegen.sh --check    # the CI drift gate
cargo test -p escribass-proto
```

## Why it looks like this

| Decision | Reason | Where |
|---|---|---|
| One `ToolResult` for every mutating RPC | Every tool does the same thing: produce ops, which the pipeline records or refuses. Sixteen response messages would be sixteen copies of one contract, each free to drift. | ADR 0006 §1 |
| Reads return their own messages | `valid`, `patch`, `summary` and `entry_id` mean nothing for a read, and a result whose fields are mostly inapplicable teaches callers to ignore fields. | ADR 0006 §1 |
| `ToolResult.patch` is `bytes` | RFC 6902 values cross unmodelled. Modelling them in protobuf made the patch log nondeterministic and produced documents `core` could not re-read. | ADR 0002 §11 |
| Requests embed `escribass.song.v1` types | A hand-written `NoteSpec` mirroring `Note` is a second representation of song state under another name, and it stops matching the first time a field is added. | ADR 0006 §4, CLAUDE.md #1 |
| `Violation` rather than a per-transport error | `Violation`, `PatchError`, `HistoryError` and `ProjectError` in `core` already carry `path`/`rule`/`message`. This transports that shape; it does not add a fifth. | ADR 0006 §2 |
| `dry_run` on every request, defaulting to false | §5's name and polarity. Inverting it would make the wire disagree with the spec that names it. | ADR 0006 §3 |
| Rust codegen only | TypeScript's consumer is M2's app, Python's is M3's orchestrator. Neither exists; generating stubs now pulls `grpclib` into `schema/` to satisfy nothing. | ADR 0006 §7 |
| No `tonic` plugin yet | This module is the contract. The server that speaks it, and its dependencies, arrive at the gRPC step. | `docs/plan.md` |
| Three `buf lint` rules relaxed | `SERVICE_SUFFIX`, `RPC_RESPONSE_STANDARD_NAME` and `RPC_REQUEST_RESPONSE_UNIQUE` all contradict the shape §5 specifies. Scoped to this module in the root `buf.yaml`; `schema/` keeps the full rule set. | ADR 0006 §8 |

## The trap, named once

`patch` is `bytes`, so the **generated serde impl base64-encodes it**:

```
serialize_field("patch", pbjson::private::base64::encode(&self.patch).as_str())
```

That impl is correct proto3 JSON and wrong for MCP, where the patch must arrive as a JSON
array a model can read. Building an MCP payload with `serde_json::to_value(&result)` produces
something that parses, round-trips and passes any test that does not read it — the same defect
`core/src/history.rs` documents for `PatchEntry.ops`. Build the payload field by field, and
parse the patch from its canonical text.

`tests/contract.rs` pins this behaviour so it stays known rather than rediscovered.

## Adding things

- **An RPC:** add the request message and the `rpc` line, keeping `ToolResult` as the return
  unless it is a read. `./proto/codegen.sh`. Update the count in
  `tests/contract.rs::every_mutating_rpc_returns_the_shared_result`. Commit `gen/` with the
  `.proto`.
- **A field on an existing request:** additive, so `buf breaking` is satisfied. Never renumber.
- **A whole entity in a request:** use the `escribass.song.v1` message. If one does not exist,
  that is a `schema/` change and needs an ADR first (`docs/adr/AGENTS.md`).
- **A language target:** one plugin entry in `buf.gen.yaml`, the version in
  `lock.baseline.json` under `tool_api`, and a consumer that actually needs it.

`buf breaking` runs on pull requests only, so work that skips the PR skips the wire check
(CLAUDE.md, Working style).
