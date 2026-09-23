# AGENTS.md — proto/

The tool API's wire contract: `docs/specs.md` §5, shaped by `docs/adr/0006-tool-api-wire-shape.md`.

Every mutation in the system arrives through this service. Agents never read or write the
project file directly (§5), so what is not here cannot be done to a song.

## What is here

| Path | Role | Written by |
|---|---|---|
| `song_tools.proto` | The `SongTools` service, its request messages, `ToolResult` and `Violation`. | hand |
| `render.proto` | The engine's boundary: `RenderPlan`, `RenderResult`, the `Preview` command and event messages, and the engine's two services — `Render`, served since M2 PR 9, and `Preview`, since PR 10 (ADR 0007, ADR 0008, ADR 0013). | hand |
| `assistant.proto` | The AI sidecar's boundary: one service, `Assistant`, one bidirectional stream per prompt, and the messages that cross it. **`ai` serves and `app` dials** (ADR 0020 §1, §3). Defined in M3 PR 3, served from PR 5. | hand |
| `buf.gen.yaml` | Five plugins: `protoc-gen-prost`, `protoc-gen-prost-serde`, `protoc-gen-tonic`, `protoc-gen-es`, `protoc-gen-python_betterproto2`. | hand |
| `codegen.sh` | `buf format -w`, `buf lint`, `rm -rf gen`, `buf generate`, then the Python import rewrite below. `--check` is the drift gate. | hand |
| `gen/rust/` | Generated output, committed for review (§4.1). **`codegen.sh` deletes `gen/` whole on every run.** Never edit, never add a file under it. | generated |
| `gen/ts/` | Generated TypeScript, for `app` (M2 PR 3, ADR 0006 §7). Same rules as `gen/rust/`: wholly generated, deleted whole on every run. | generated |
| `gen/python/` | Generated Python, for `ai` (M3 PR 3, ADR 0006 §7 as ADR 0020 §1 narrows it): a **server and no client**, because the sidecar serves `Assistant` and dials nothing. Same rules as the other two, plus one: `codegen.sh` deletes the model's re-emitted packages and rewrites their imports — see the second trap below. `tests/test_generated_python.py` is what checks it. | generated |
| `package.json`, `package-lock.json`, `tsconfig.json` | Package `@escribass/proto`, `private`, exporting `./tools` and `./render` — and **not** `assistant_pb.ts`, which is generated because excluding one file from a plugin is a second thing to keep in step, and unexported because nothing in the window dials `ai`: `core` does. Two runtime packages and a `file:../schema` link — no toolchain of its own: `schema`'s pinned `protoc-gen-es` generates and `schema`'s pinned `tsc` checks. | hand |
| `gen/descriptor.binpb` | The compiled `FileDescriptorSet`, imports included. `core` turns it into a JSON Schema per tool, so the schemas and the Rust types come from one artefact. Exposed as `escribass_proto::DESCRIPTOR`. | generated |
| `src/lib.rs` | The hand-written module tree that `include!`s the generated file. Nothing else. | hand |
| `tests/contract.rs` | The five things about this file a change could break silently. | hand |
| `tests/test_generated_python.py` | That `gen/python` imports, that its `Assistant` server maps the one RPC, and that there is **one** `Song` — `escribass_schema`'s. Runs in `schema`'s environment (M3 PR 3). | hand |

The workspace `buf.yaml` is at the repository root, not here: buf v2 wants one at the common
ancestor of every module, and `song_tools.proto` imports `schema/song.proto`.

```
./proto/codegen.sh            # regenerate; run from anywhere
./proto/codegen.sh --check    # the CI drift gate
cargo test -p escribass-proto
npm --prefix proto ci                                   # once, for the two runtime packages
schema/node_modules/.bin/tsc --noEmit --project proto   # the generated TypeScript compiles
```

```
cd schema && uv run python -m unittest discover -s ../proto/tests   # the generated Python
```

That line was owed to M3 PR 5 for one day and landed here instead: the generated
`AssistantBase` imports `grpclib`, `grpclib` was approved by name and unpinned, and the user
decided on 2026-09-23 that a check which is itself the package's first use is exactly where
U4 pins it (`lock.baseline.json`, `ai.grpclib`; the struck-through ledger row of 2026-09-22
in `docs/plan.md` keeps the reasoning). It runs from `schema/`, because `proto/` has no
Python environment of its own — the same borrowing as `tsc` above.

`codegen.sh --check` proves the generated code matches the `.proto`; **`test_generated_python.py`
is what proves it imports**, and — the half that matters — that a `Prompt`'s `song` is
`escribass_schema`'s `Song` class. An import alone would pass against the duplicate the next
section describes, because a duplicate imports as happily as the real thing.

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
| Rust, TypeScript and — since M3 PR 3 — Python | Each language arrived in the milestone that gave it a consumer: TypeScript with `app` in M2 PR 2, Python with the sidecar that serves `Assistant`. "Generating it now pulls `grpclib` in to satisfy nothing" was right about the package and about the timing; it was wrong only about who would call what, since `ai` serves rather than dials. | ADR 0006 §7, carried out and narrowed |
| The Python is a **server and no client** | `ai` serves one service and dials nothing, so `server_generation=async, client_generation=none`. The direction itself was decided by this plugin: it emits a `grpclib` client *and* server under `client_generation=async, server_generation=async`, a synchronous `grpcio` client and no server otherwise, and no `grpcio` server at all — so the process that serves is the one whose gRPC is generated. | ADR 0020 §1, §2 |
| `Assistant` is `ai`'s service, not the host's | The lock puts the session in `app`, so the only question was direction; the model's calls come **back** over the stream as `{name, args}`, which is the envelope `core::call` already takes, and `ai` gets no `SongTools` stub to be told not to use. A `Jobs` service beside it was refused: a stream carries progress, cancellation and the answer already. | ADR 0020 §1, §3 |
| A call's arguments cross as **JSON text**, not a `oneof` | Twelve arms mirroring `song_tools.proto` are the second description ADR 0006 §4 refuses, free to drift from the first — and the host hands the object to `call` exactly as the MCP server does. Parsing it in `ai` would be a second validator in a third language. | ADR 0020 §3; `docs/plan.md`, M3 trap 12 |
| No provenance and no `list_params` on this wire | The three ids travel with the proposal, inside `core`: a field here is a field a caller can lie in. And with `set_param` withheld, a tool returning 2,855 `ParamID`s returns ids nothing the model is offered can act on. | ADR 0021 §2; ADR 0022 §2 |
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

It has a Python half too, and the same sentence covers it: betterproto2's `to_json` writes
camelCase and a `Z` timestamp (`schema/tests/test_roundtrip.py` says so in its own header), so
a sidecar that built a request from the generated model and handed its `to_json` to the tool
API would send `startTick`. The schemas the model is given are **the descriptor's**, never the
Pydantic model's — which is why `assistant.proto`'s `ToolSchema.input_schema` carries the text
`core` produced rather than anything `ai` can derive.

## The second trap, which is Python's alone

`betterproto2-compiler` generates a module for **every file in the request** — it ignores
`file_to_generate` — and it has no `extern_path` and no `rewrite_imports`. So `buf generate`
re-emits `song.proto` and `history.proto` under `gen/python`, byte for byte identical to
`schema/`'s copy and a **different class at run time**. That is the second `Song` the four
`extern_path` lines and the two `rewrite_imports` in `buf.gen.yaml` exist to prevent, arriving
by a route neither of them covers, and it imports and type-checks perfectly well.

`codegen.sh` deletes those packages after `buf generate` and rewrites the imports that named
them to `escribass_schema`, then refuses the run if a relative import into the model survived
or if no rewritten one exists. Both greps matter: the rewrite is two regular expressions over
generated text, and a compiler that changed how it spells a cross-package import would leave
them matching nothing and commit a tree importing three deleted packages (ADR 0020 §1,
extended 2026-09-22).

`tests/test_generated_python.py` is the other end of it, and it is the end that would catch a
rewrite that ran and got it *wrong* rather than one that did not run: it builds a `Prompt`
with `escribass_schema`'s `Song` and asserts the field holds that class. Against a tree
regenerated without the rewrite it does not even reach the assertion — pydantic refuses the
value, "Input should be a dictionary or an instance of `Song`", with the same name on both
sides of the sentence.

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
  imports, and a `file:` link resolves its own from its own directory. Python was added this
  way in M3 PR 3 and cost no package file at all — it borrows `schema/`'s environment, as the
  TypeScript borrows `schema/`'s `tsc` — but it did cost the rewrite below, because its plugin
  has no way to say "this type lives in another package". **Check that first** for any
  language after it: a target that cannot be told where the model lives will generate a second
  one, and it will compile.

`buf breaking` runs on pull requests only, so work that skips the PR skips the wire check
(CLAUDE.md, Working style).
