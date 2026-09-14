# AGENTS.md — proto/

The tool API's wire contract: `docs/specs.md` §5, shaped by `docs/adr/0006-tool-api-wire-shape.md`.

Every mutation in the system arrives through this service. Agents never read or write the
project file directly (§5), so what is not here cannot be done to a song.

## What is here

| Path | Role | Written by |
|---|---|---|
| `song_tools.proto` | The `SongTools` service, its request messages, `ToolResult` and `Violation`. | hand |
| `render.proto` | The engine's boundary: `RenderPlan`, `RenderResult`, the `Preview` command and event messages, and the engine's two services — `Render`, served since M2 PR 9, and `Preview`, since PR 10 (ADR 0007, ADR 0008, ADR 0013). | hand |
| `buf.gen.yaml` | Four plugins: `protoc-gen-prost`, `protoc-gen-prost-serde`, `protoc-gen-tonic`, `protoc-gen-es`. | hand |
| `codegen.sh` | `buf format -w`, `buf lint`, `rm -rf gen`, `buf generate`. `--check` is the drift gate. | hand |
| `gen/rust/` | Generated output, committed for review (§4.1). **`codegen.sh` deletes `gen/` whole on every run.** Never edit, never add a file under it. | generated |
| `gen/ts/` | Generated TypeScript, for `app` (M2 PR 3, ADR 0006 §7). Same rules as `gen/rust/`: wholly generated, deleted whole on every run. | generated |
| `package.json`, `package-lock.json`, `tsconfig.json` | Package `@escribass/proto`, `private`, exporting `./tools` and `./render`. Two runtime packages and a `file:../schema` link — no toolchain of its own: `schema`'s pinned `protoc-gen-es` generates and `schema`'s pinned `tsc` checks. | hand |
| `gen/descriptor.binpb` | The compiled `FileDescriptorSet`, imports included. `core` turns it into a JSON Schema per tool, so the schemas and the Rust types come from one artefact. Exposed as `escribass_proto::DESCRIPTOR`. | generated |
| `src/lib.rs` | The hand-written module tree that `include!`s the generated file. Nothing else. | hand |
| `tests/contract.rs` | The five things about this file a change could break silently. | hand |

The workspace `buf.yaml` is at the repository root, not here: buf v2 wants one at the common
ancestor of every module, and `song_tools.proto` imports `schema/song.proto`.

```
./proto/codegen.sh            # regenerate; run from anywhere
./proto/codegen.sh --check    # the CI drift gate
cargo test -p escribass-proto
npm --prefix proto ci                                   # once, for the two runtime packages
schema/node_modules/.bin/tsc --noEmit --project proto   # the generated TypeScript compiles
```

## Why it looks like this

| Decision | Reason | Where |
|---|---|---|
| One `ToolResult` for every mutating RPC | Every tool does the same thing: produce ops, which the pipeline records or refuses. Sixteen response messages would be sixteen copies of one contract, each free to drift. | ADR 0006 §1 |
| Reads return their own messages | `valid`, `patch`, `summary` and `entry_id` mean nothing for a read, and a result whose fields are mostly inapplicable teaches callers to ignore fields. | ADR 0006 §1 |
| `AddAsset` and `RenderExport` too | Neither produces ops, so `patch` and `entry_id` would be permanently empty — and each has one thing the caller actually needs: an address, and the hash of what was rendered beside the commits it was rendered by. `RenderExport` was declared as a `ToolResult` in PR 3 and changed in PR 10, before any client had called it; the root `buf.yaml` carried a one-release breaking-check exemption for that, and PR 11 deleted it once `main` no longer held the old type — the `breaking` block has no `except` today. | ADR 0006 §1, amended |
| `ToolResult.patch` is `bytes` | RFC 6902 values cross unmodelled. Modelling them in protobuf made the patch log nondeterministic and produced documents `core` could not re-read. | ADR 0002 §11 |
| Requests embed `escribass.song.v1` types | A hand-written `NoteSpec` mirroring `Note` is a second representation of song state under another name, and it stops matching the first time a field is added. | ADR 0006 §4, CLAUDE.md #1 |
| `Violation` rather than a per-transport error | `Violation`, `PatchError`, `HistoryError` and `ProjectError` in `core` already carry `path`/`rule`/`message`. This transports that shape; it does not add a fifth. | ADR 0006 §2 |
| `dry_run` on every request, defaulting to false | §5's name and polarity. Inverting it would make the wire disagree with the spec that names it. | ADR 0006 §3 |
| Rust and TypeScript, not Python | TypeScript's consumer arrived in M2 PR 2 and the codegen followed in PR 3; Python's is M3's orchestrator and does not exist, so generating it now pulls `grpclib` into `schema/` to satisfy nothing. | ADR 0006 §7 |
| The TypeScript reaches the model by package name, not by generating it | `rewrite_imports` in `buf.gen.yaml` is the TypeScript spelling of the Rust `extern_path` lines beside it. Left alone the generated code imports `./song_pb.js`, a file this module must not generate: a second `Song` in the tree compiles perfectly well and is wrong. `@escribass/schema`'s `"./*_pb.js"` export exists to answer that rewrite. | CLAUDE.md #1, ADR 0006 §4 |
| `RenderPreview` answers with `PreviewResponse`, and takes `PreviewFrom` rather than `PreviewPlay` | A preview records nothing, and where the transport is has nowhere to go in `ToolResult`. Its `play` cannot carry `PreviewPlay`, whose plan is `core`'s to compile from the document and never a caller's; the other three commands reuse the engine's messages whole. | ADR 0006 §1, extended; ADR 0013 §2, amended |
| `PreviewEvent.applied` | The one field the preview shape took after PR 3 settled it. The transport writes events of its own while it plays, and an answer can only be told from them by the count of commands it answers. Additive, so `buf breaking` had nothing to say. | ADR 0013 §2, amended |
| `Preview` is a service of its own | ADR 0013 §3 makes the engine's mode *which service the process serves*: an export process registers `Render` alone, so gRPC refuses a preview on it rather than a check written in C++. | ADR 0013 §2, amended; §3 |
| A `tonic` server *and* client | `core` implements the server. The client's first consumer is `core`'s own end-to-end test, which needs something to call with; M2's app is the next. Hand-rolling one would be more code than generating it. | ADR 0006 §7 |
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

The same trap has a TypeScript half, now that `gen/ts` exists. What `app` sends and receives is
**not** proto3 JSON of these messages: `core/src/call.rs` decodes arguments with
`preserve_proto_field_names`, so the fields are `snake_case` where `protobuf-es` writes
`camelCase` by default, and it builds `patch` as an RFC 6902 array where `toJson` would write
base64. A consumer that round-trips through the generated schemas must ask for
`useProtoFieldName` and must not put `patch` through them. The generated types describe the
*shape*; `call.rs` and the MCP `inputSchema` describe the bytes.

## Adding things

- **An RPC:** add the request message and the `rpc` line, keeping `ToolResult` as the return
  unless it is a read. **Give it a leading comment** — that comment is the tool's description
  over MCP, and a test fails if any tool has none. `./proto/codegen.sh`. Update the count in
  `tests/contract.rs::every_mutating_rpc_returns_the_shared_result`. Commit `gen/` with the
  `.proto`.
- **A field on an existing request:** additive, so `buf breaking` is satisfied. Never renumber.
- **A whole entity in a request:** use the `escribass.song.v1` message. If one does not exist,
  that is a `schema/` change and needs an ADR first (`docs/adr/AGENTS.md`).
- **A language target:** one plugin entry in `buf.gen.yaml`, the version in
  `lock.baseline.json` under `tool_api`, and a consumer that actually needs it. TypeScript was
  added this way in M2 PR 3, and cost a `package.json` beside it — the generated code has
  imports, and a `file:` link resolves its own from its own directory.

`buf breaking` runs on pull requests only, so work that skips the PR skips the wire check
(CLAUDE.md, Working style).
