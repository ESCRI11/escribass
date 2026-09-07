# ADR 0008 — The engine is a fresh subprocess per render, speaking stdio until M2 gives gRPC a consumer

- **Status:** Accepted (2026-09-05)
- **Affects:** `engine/` (new, M1); `proto/render.proto` (new, M1); `schema/AGENTS.md`;
  `docs/specs.md` §3 and §8
- **Builds on:** ADR 0006 §5 (one project per process), §6 (stdio hygiene on an MCP server) and
  §7 (`proto/` generates for the consumers that exist), ADR 0007 (what crosses is a `RenderPlan`)
- **Recorded in:** `docs/specs.md` §3, §8 and §15.

## Context

ADR 0007 fixes *what* crosses the boundary. The documents disagree about how it travels. §3
says `app` ↔ `engine` is gRPC, "control and transport only". §8 says the engine "receives a
materialised layer 2 + layer 3 snapshot, builds a Tracktion edit, and either streams preview
audio or renders offline". Neither says how many renders one engine process serves, what it
holds between them, or who speaks gRPC in a milestone that has no `app`.

M1 has no `app`. The only caller is `core`, inside `escribass-grpc` or `escribass-mcp`, and
the only thing it asks for is one offline render. A gRPC client there is a client talking to a
server it launched itself, one message at a time.

The question underneath is not transport. It is whether the engine has a lifetime. Everything
that makes a render reproducible — no warm smoothers, no plugin instance carried forward, no
device manager left open — is a statement about how long the process lives, and the transport
follows from it rather than the other way round.

## Decisions

### 1. `Render` is defined in `render.proto` in M1, implemented at M2; M1 speaks stdio

ADR 0006 §7 already settled this shape of question once: `proto/` generates Rust only because
the TypeScript and Python consumers do not exist yet, and "the `.proto` is the artefact that
has to be right, and it is right regardless of who has generated from it". The same split
applies one boundary over. The service is **defined** in PR 3, so `buf breaking` guards it from
M1 onward and the shape M2 implements is the shape M1 reviewed. It is **implemented** at M2,
when `app` exists to hold a connection.

Implementing it now would mean vendoring grpc++ into `engine/`: a second C++ protobuf runtime
coupling, its own pin, and its own build in a job that already builds JUCE, Tracktion and three
plugins — to carry one message per process between a parent and the child it just spawned.

This is consistent with §3 rather than an exception to it. §3 describes the steady state of
§3's own table: three supervised long-lived processes, one of which streams preview audio to
the device. M1 delivers none of that; it delivers offline rendering, where the process is born
with its work and dies with its answer. **Convergence is at M2**: the engine gains the gRPC
server, and the stdio path is *deleted*, not kept beside it. Two transports for one boundary
is exactly the drift ADR 0006 §1 refused to accept sixteen times over — and it would be worse
here, because the second one would be the untested one.

The framing is EOF, not a length prefix: exactly one `RenderPlan` arrives on stdin and is read
to end-of-stream, exactly one `RenderResult` leaves on stdout, and the process exits. A prefix
frames a stream of messages; there is no stream. **stdout carries protobuf bytes and nothing
else** — every log line, every JUCE warning, every plugin's own chatter goes to stderr. That
rule is not new: ADR 0006 §6's MCP server already lives under it, where a stray `println!`
corrupts the frame. It is worth restating because the engine links a large C++ tree that was
not written with it in mind.

The plan also names the **output path**, and the engine writes the WAV there. That is the same
shape as ADR 0007 §2's asset path: a file the engine is handed a path to is input or output,
and it learns nothing about the `.escri` layout by using one. §8's "never reads project files"
is about the project, not about the filesystem.

Failure is an exit code, not a `RenderResult` with errors in it. ADR 0006 §2 draws the
caller/operator line by who can fix it, and by the time a plan reaches the engine every
caller-fixable failure has already been taken: the validator refused an unresolvable reference
(§4.4), ADR 0010's load check refused a plugin this build lacks, and `compile` refused what M1
cannot render (ADR 0007 §6). What remains — a plugin that will not instantiate, a full disk, a
crash — is an operator error, and `render_export` surfaces it as one. A `RenderResult` carrying
a violation would put it back on the side of the line ADR 0006 §2 keeps for things a model can
retry into, and no retry fixes a segfault.

### 2. One render, one process. No engine is kept alive between renders

M1 spawns the engine, hands it a plan, takes a WAV, and lets it die. Nothing is pooled, warmed
or reused.

The argument is trap 6, and it is not about tidiness. A plugin's parameter smoothers ramp *from
their previous value*: the same automation applied to a fresh instance and to one that just
rendered something else produces two different first blocks. Order compounds it — a `setState`
applied after `setParam` overwrites the parameters, and the correct order is only correct on an
instance whose state is known. A resident engine therefore makes render N a function of render
N−1, and CLAUDE.md #3 and §11's first two lines are the statement that it must not be.

A resident engine could be made deterministic — destroy and recreate every plugin between
renders, reset every ramp — but that is a claim maintained by discipline in C++, checked by
nothing, and its failure mode is a golden that passes alone and fails in a suite. Process
death is the same guarantee enforced by the operating system.

**It costs, and the cost is paid per render**: process start, JUCE initialisation, plugin
instantiation. Two of those three are irreducible and small; the third — *finding* the plugins
— is the one that would dominate, because a VST3 scan walks directories and instantiates
everything it finds. That is what ADR 0010's manifest removes: the engine loads the exact
binaries the manifest names, at the paths it names, and never scans. The manifest is produced
once at build time, so the per-render cost is one `dlopen` per referenced plugin and nothing
else. **Measured 2026-09-06, in PR 7, which first hosted one.** One second of audio at 48 kHz on the
pinned compiler, five processes each, warm: a plan with no device at all is **520 ms**, which is
process start plus JUCE and Tracktion initialisation and is what every render pays; one Dexed
adds **65 ms**; all three bundled plugins on three tracks come to **914 ms**, so the whole
plugin half of a three-instrument render is under 400 ms. The fixed half dominates, and it is
the half a resident engine would save — which is the trade decision 2 declines, at a price now
known rather than assumed. The spike had measured build cost (two minutes wall from cold, 23
minutes CPU) and never start-up.

The reuse this forgoes buys nothing M1 wants. An offline render is not interactive, and §11's
golden tests each render once. M2's preview does need a live process, for a reason M1 does not
have — an open audio device — so its lifetime is M2's decision to make against a real UI,
not a generalisation of this one.

### 3. `main()` runs a message loop, because a render is asynchronous

The spike found this rather than reasoned it (`docs/plan.md`, "What the spike found"). Tracktion's
own test utilities call `EditRenderer::render(params, callback)` and pump a dispatch loop until
the callback fires. The synchronous-looking wrapper, `Renderer::renderToFile`, **silently
produced nothing**: right preconditions, no error returned, empty file on disk.

That is a fact about the binary's shape, so it belongs in the ADR that fixes the shape. The
engine's `main()` is not "decode, call a function, exit"; it is: read stdin to EOF, decode the
plan, build the edit, start the render, **pump the message loop until the completion callback
fires**, write the result, exit. Any design that assumed a blocking call — including the
obvious one — would have been written into this ADR and discovered in PR 5.

The same shape has a second instance, and it is why decision 1 checks the file rather than the
return value. `Renderer::Parameters::tracksToDo` documents itself as "if this is empty, all
tracks will be rendered", while the implementation requires `countNumberOfSetBits() > 0` and
returns an empty `File` with no error when it is not met. Two APIs in the render path fail by
producing nothing and saying nothing. The engine sets the track bits explicitly, and treats a
missing or empty output file as a failure regardless of what any call returned.

### 4. The engine's C++ protobuf is generated at build time by its own CMake, never checked in

CLAUDE.md defers C++ codegen from M0 step 1 to M1, "where the engine gives it a consumer", and
ADR 0006 §7 leaves `proto/` generating Rust only. This is where it lands — and it does not land
in `schema/codegen.sh`.

Generated `.pb.cc` embeds a protobuf **runtime version check**. Committing it therefore pins a
`protoc` that must equal the vendored runtime, and the two pins have to be moved together by
whoever upgrades either. A mismatch surfaces as a version assertion at engine start-up or a
link error, in a build nobody was changing. Generating at build time makes it **one** pin —
the vendored protobuf — and makes drift a compile error inside the engine's own CI job, which
is a stronger gate than a committed hash and a rule about keeping it fresh.

It is also the same reasoning ADR 0010 uses for the plugin manifest, and the reason both land
here: an artefact derived from a pinned input is regenerated from the pin, never committed
beside it, because a committed derivative is a second thing that can be stale.

The generated sources are `song.proto` and `render.proto`, read from `schema/` and `proto/`
directly. `schema/gen/` gains no fourth language, and `schema/codegen.sh --check` is
unaffected: it compares the three languages it generates, and C++ is not one of them.
`schema/AGENTS.md`'s line — "A language target (C++ at M1): one plugin entry in `buf.gen.yaml`"
— was written before this coupling was seen and is corrected in this commit.

**Pinned 2026-09-06, in PR 5: protobuf v21.12, `f0dc78d7e6e331b8c6bb2d5283e06aa26883ca7c`.**
This decision named "the vendored protobuf" without a version, and no C++ protobuf was in
`lock.baseline.json` to name. v21.12 is the smallest option that builds: the last line before
the runtime depends on abseil — a second submodule and minutes more in a job that already
builds JUCE and Tracktion — and the version Ubuntu 24.04 itself packages. The engine builds
`protoc` from the same checkout, which is what makes the runtime and the compiler one pin. It
is a new dependency under CLAUDE.md #4, added to `lock.baseline.json` under `engine.protobuf`
and to §17, pending sign-off.

### 5. The engine reports the commits it was built from, and the render suite compares them

Trap 8 is M0.4's own defect one language over, and worse. `cargo test -p escribass-tests` does
not rebuild the binaries it drives, so the suite could validate a build from *before* the
change under test and pass; two deliberate mutations proved it, and the harness now refuses a
binary older than `core/src`. The engine cannot be defended that way. It is built by CMake,
outside the cargo graph entirely — nothing in `cargo test` knows it exists, let alone that it
is stale — and its inputs are submodules, which drift by being left alone.

So the engine **embeds its provenance at configure time**: the commit of each submodule as
`git rev-parse` reports it — Tracktion Engine, JUCE, **protobuf**, Rubber Band, and each
bundled plugin, which is **seven** — compiled in as constants. (Protobuf added to this list
2026-09-07 in PR 13: §4's own amendment made it a pinned submodule in PR 7 and the binary has
reported it since, while this sentence still named six. ADR 0010 §1 was corrected for the same
drift in PR 9; this one was not.) It reports them in two places: on `--version`, and in `RenderResult`.
The render suite compares them against `lock.baseline.json` before it compares a single sample,
and a mismatch fails naming the component, not as a golden diff.

Embedding rather than reading `lock.baseline.json` at run time is the whole point. A file read
at run time reports what is on disk now; the question being asked is what this binary was
compiled against, and only the binary knows that. Taking the values from the submodules rather
than from `lock.baseline.json` at configure time is the same distinction one level down: it
reports the checkout that was actually compiled, so a drifted submodule is caught by the
comparison instead of being copied into it. That is §17's "CI fails if a submodule or vendored
dependency drifts" made mechanical.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Implement gRPC in the engine now | Vendors grpc++ and a second protobuf runtime coupling for a client that would be itself, in M1's already-large engine build. ADR 0006 §7's precedent is to define the `.proto` and generate for real consumers. |
| Keep stdio *and* gRPC after M2 | Two transports for one boundary, one of them exercised only by tests. ADR 0006 §1's reason: copies of one contract are free to drift, and the unused copy drifts first. |
| Length-prefixed framing on stdio | Frames a stream of messages; there is exactly one in each direction, and EOF already delimits it. |
| A resident engine, reset between renders | Determinism maintained by discipline in C++ and checked by nothing. Its failure mode is a golden that passes alone and fails in a suite — the class of defect M0's reviews kept finding. |
| A pool of warm engine processes | The same hazard as reuse, plus a scheduler deciding which process serves which render, which makes the output depend on scheduling. |
| Commit the generated C++ | Pins `protoc` and the vendored runtime as two values that must agree, with the mismatch surfacing in a build nobody was changing. |
| Generate C++ through `schema/codegen.sh` | Puts a C++ toolchain in the codegen gate that every non-engine PR must pass, to produce sources only the engine consumes. |
| The engine reads `lock.baseline.json` at run time to report versions | Reports what is on disk now, not what the binary was built against — which is the only question trap 8 asks. |
| `RenderResult` carries engine failures as violations | Puts an operator failure inside §6's retry loop; no retry fixes a crash. ADR 0006 §2 draws the line by who can fix it. |

## Consequences

- **PR 3** defines `service Render` in `proto/render.proto` alongside `RenderPlan` and
  `RenderResult`. It is generated for Rust like the rest of `proto/` (ADR 0006 §7) and
  implemented by nobody in M1; `buf breaking` covers it from that PR.
- **PR 5** adds `engine/` — already listed in §13, so no new-directory ADR — with the CMake
  that generates C++ from `schema/song.proto` and `proto/render.proto`, the stdio `main()` with
  its message loop, the embedded submodule commits, and the CI job. Three things the spike
  found are requirements on that job rather than decisions here: Tracktion declares JUCE with an
  **SSH URL** no keyless runner can clone, and `insteadOf` did not take — the job must override
  `submodule.modules/juce.url`; Linux needs **`-latomic`**, because `tracktion_engine_playback`
  references `__atomic_store` for 16-byte atomics GCC does not lower inline; and **X11 headers**
  must be installed to *build*, because JUCE compiles `juceaide` against `juce_gui_basics`
  during configure, before any define of ours applies. `webkit2gtk` and `gtk+` are reported
  missing and the configure succeeds, so the apt list is shorter than JUCE's documented set.
- **PR 7** measures what a fresh process costs and records it; no number is claimed here.
- **PR 11** compares the embedded commits against `lock.baseline.json` before comparing audio,
  which is trap 8's guard.
- **`schema/AGENTS.md`** loses its `buf.gen.yaml` line for C++ in this commit.
- **M2** implements `Render` over gRPC and deletes the stdio path. Preview playback's process
  lifetime is decided there, against a UI, and is not implied by decision 2.
- The `docs/plan.md` deferred row for a project lock file is unchanged: one engine process
  per render touches no `.escri` file, so it adds no second writer.
