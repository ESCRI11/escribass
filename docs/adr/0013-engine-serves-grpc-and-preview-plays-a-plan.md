# ADR 0013 — The engine serves gRPC, preview plays a `RenderPlan` from a tick, and an export still gets its own process

- **Status:** Accepted (2026-09-07)
- **Affects:** `engine/` (its CMake, `main.cpp`, a new server); `proto/render.proto`;
  `core/src/engine.rs`; `tests/renders.rs`; `docs/specs.md` §3, §8 and §17
- **Builds on:** ADR 0008 §1 (`Render` defined in M1, implemented over gRPC at M2, and the
  stdio path deleted rather than kept), §2 (one render, one process), §3 (`main()` runs a
  message loop), §4 (the engine's C++ protobuf, pinned at v21.12), §5 (the engine reports what
  it was built from); ADR 0007 (what crosses is a `RenderPlan`); ADR 0009 §1 (what
  bit-exactness is claimed over)
- **Recorded in:** `docs/specs.md` §3, §8, §15 and §17.

## Context

ADR 0008 §1 committed M2 to two things and left the third open. The two: `Render` gains its
gRPC implementation, and **the stdio path is deleted rather than kept beside it**, because two
transports for one boundary is one tested transport and one that is not. The one left open:
ADR 0008 §2's Consequences handed preview's process lifetime forward on purpose — "M2's to
decide against a real UI" — rather than generalising "one render, one process" into a rule
about a process that plays.

§8 says the engine "either streams preview audio or renders offline". §16 says "preview
playback". Nothing in either says what a preview *is*, which is a problem with a deadline:
`render.proto`'s shape is guarded by `buf breaking`, which runs on pull requests only, so a
service that grows across two PRs is compared against a `main` that already moved (trap 12).

And under both sits a dependency question the milestone cannot start without. Implementing
gRPC in C++ means vendoring grpc++ into a build that already compiles JUCE, Tracktion and three
VST3s — against a protobuf deliberately pinned at **v21.12**, the last line before the runtime
depends on abseil (ADR 0008 §4). A 2022 protobuf constrains which gRPC releases are even
candidates, and "it should be fine" is exactly the class of assumption M1's PR 0 existed to
refuse. This ADR was not written until that was measured.

## Decisions

### 1. grpc++ is vendored, pinned at v1.54.3 — the newest release whose own protobuf is our protobuf

`engine/vendor/grpc` becomes a submodule at `868412b573a0663c8db41558498caf44098f4390`
(v1.54.3, `grpc/grpc`), and §17 gains a row for it. The dependency is approved under
CLAUDE.md #4.

The version is not chosen for its number. gRPC carries protobuf as its own submodule, and
**v1.53.0 through v1.54.3 pin it at `f0dc78d7e6e331b8c6bb2d5283e06aa26883ca7c` — byte for byte
the commit `lock.baseline.json` already records for protobuf v21.12.** v1.52.2 and earlier pin
v21.9 (`24487dd`); v1.55.0 moves to protobuf 22.x, which is where abseil becomes protobuf's own
dependency and where ADR 0008 §4's reason for the pin stops holding. So v1.54.3 is the last
release on the far side of that line, and the pin it brings is not a second pin that must agree
with ours — **it is ours**. That is ADR 0008 §4's own argument ("two pins that must agree, with
the stale one silent") applied one layer out, and it is the whole reason to prefer an older
gRPC to a newer one.

**Measured, 2026-09-07, rather than assumed.** grpc v1.54.3 was configured and built against
`engine/vendor/protobuf` itself — the repository's own checkout, at `f0dc78d` — with
`gRPC_PROTOBUF_PROVIDER=module` pointed at it and `CMAKE_CXX_FLAGS` set to the engine's pinned
`-march=x86-64 -mtune=generic -ffp-contract=off`. It configured in 9.4 s and built
`libgrpc++.a` and `grpc_cpp_plugin` in **2 m 53 s wall over 1345 targets** (≈52 CPU-minutes,
20 jobs, g++ 13.3). It emits no `-ffast-math`, no second `-march`, and exactly two ISA flags
beyond the baseline — `-maes` and `-msse4.1`, on abseil's `randen_detect.cc` and
`randen_hwaes.cc` — which are **already on `engine/cmake/check_flags.cmake`'s allowlist**, for
that same code reached through sfizz. The flag guard would not fire, and ADR 0009 §3's
compile-time ISA pin survives the addition intact.

abseil arrives with it, at `b971ac5250ea8de900eae9f95e06548d14cd95fe` (LTS 20230125) — pinned
by grpc's commit, as JUCE is pinned by Tracktion's. It collides with nothing: the bundled
plugins are `ExternalProject_Add`, separate CMake projects by `engine/cmake/plugins.cmake`'s own
first line, so sfizz-ui's vendored abseil (`c2435f8`, the same LTS release) has never been in
the engine's tree and does not become so now.

**Two things are not verified and PR 0 settles them before PR 9 depends on either.** The spike
built gRPC in a tree of its own; `engine/CMakeLists.txt` already does
`add_subdirectory(vendor/protobuf)`, so gRPC's `module` provider would add protobuf a *second*
time and collide on every target — the integration must hand gRPC the protobuf targets that
exist rather than let it bring its own, and which of `gRPC_PROTOBUF_PROVIDER=package` or a
pre-populated target set does that cleanly is a build question, not a design one. And 52
CPU-minutes on 20 cores is not 52 CPU-minutes on a two-core runner in a job that is already up
to an hour cold; what it costs there, warm and cold, is the second measurement. Both are the
kind of thing that arrives as a red CI job rather than a wrong answer, which is why the
decision does not wait on them and the code does.

### 2. Preview is a compiled `RenderPlan` played from a tick, over one bidirectional stream

A preview is not a new artefact. It is the **same `RenderPlan`** `compile` already produces
(ADR 0007 §4), played from a tick instead of written to a file. `render.proto` gains one RPC:

```
rpc Preview (stream PreviewCommand) returns (stream PreviewEvent);
```

The first `PreviewCommand` carries a plan and a start tick; later ones seek, set a loop, stop,
or carry a replacement plan. `PreviewEvent` carries transport position and state. The stream's
lifetime **is** the preview's: closing it stops playback, and `app` dying closes it for free,
which is a guarantee the operating system enforces rather than a shutdown path someone
maintains.

One RPC rather than four, because four unary calls need a session identifier to say which
preview they mean, and inventing one is the surface ADR 0006 §5 declined to invent for
projects. A stream is the identifier.

The plan is **replaced, never diffed**. A note drag recompiles the plan and sends it again.
That is the same cost shape as ADR 0012 §2's re-read and it has the same escape hatch: if it
is measurably too slow the answer is to compile *less* — a plan for a range — never a second
patch format for a message ADR 0007 §5 already proved is derived. A plan diff would be a second
RFC 6902 for a document that is not the model, applied in C++, where the validator does not
run; the reason is ADR 0007 §1's reason.

**Amended 2026-09-08, when PR 3 had to write the messages.** The decision above named
`PreviewCommand` and `PreviewEvent`, said what the first command carries and what the later ones
do, and left the shape of both — and the service they hang off — unwritten. `render.proto`
cannot be written without either, and `buf breaking` guards it from the moment it lands, so both
are settled here rather than discovered in PR 10.

**Four message arms of one `oneof`:** `PreviewPlay { RenderPlan plan; int32 start_tick; }`,
`PreviewSeek { int32 tick; }`, `PreviewLoop { int32 start_tick; int32 end_tick; }`, and an empty
`PreviewStop {}`. `PreviewPlay` is *also* the replacement plan — "carries a plan and a start
tick" and "carries a replacement plan" are one message, because replacing a plan is starting
again from wherever the transport has reached, and a second arm differing only in name is a
second thing to keep in step. An empty loop range — `end_tick` at or before `start_tick` —
clears the loop, so nothing `optional` carries the absence of one. `PreviewStop` is its own empty
message rather than `google.protobuf.Empty`, because an arm that may need a field later cannot
grow one if it is the well-known empty. Message arms rather than bare scalars is ADR 0002 §3's
rule for the model, kept here for the reason it was made there.

`PreviewEvent { int32 tick; PreviewState state; }`, with `PreviewState` one of `PLAYING` and
`STOPPED`. **There is no error arm.** A failure ends the stream with a gRPC status, exactly as a
failed render is an exit code and never a `RenderResult` carrying errors (ADR 0008 §1) — a
transport already has a channel for "this did not work", and a second one is a value someone
forgets to read.

**And `Preview` is its own service, not a second RPC on `Render`.** Decision 3 below says the
mode is *which service the process serves*, and that sentence has a meaning only if there are
two. It is also the only version that leaves decision 3's guarantee where decision 3 puts it: a
process spawned to export registers `Render` alone, so a `Preview` call on it is refused by gRPC
rather than by a check someone wrote in C++ — the difference between ADR 0008 §2 being enforced
and being maintained by discipline. `lock.baseline.json`'s gRPC entry has said "the engine's
Render and Preview services", in the plural, since PR 1.

### 3. Two lifetimes in one binary, one transport: a live process for preview, a fresh one per export

The engine is launched with a socket path on argv and `app` dials it. In **preview** mode the
process lives as long as its `Preview` stream. In **render** mode a fresh process is spawned per
offline render, serves exactly one `Render` call, and exits.

This keeps ADR 0008 §2 whole rather than reasoning around it. Its argument was never about
transport: a plugin's parameter smoothers ramp from their previous value, so an export sharing a
process with a preview is an export that depends on what was played before it, which is
CLAUDE.md #3 by another route. A single long-lived process serving both would put that back on
the table on day one, and "we destroy the plugins between" is — in ADR 0008's own words — a
claim maintained by discipline in C++ and checked by nothing, whose failure mode is a golden
that passes alone and fails in a suite (trap 6). A resident process that *spawns* a child per
export buys the same guarantee for a second process-management path; the parent gains nothing
by being in the middle, since `app` already supervises processes (§3).

So there is one binary, one transport and two modes — and the mode is which service the process
serves, not a second protocol. §3's "`app` ↔ `engine` over gRPC" becomes literally true, and
the stdio path goes: `core/src/engine.rs`'s EOF framing, `render_export`'s driver and
`tests/renders.rs`'s four goldens all move onto the new transport in one PR, in the order
implement · move · delete, so there is never a window in which the goldens are compared through
a transport nothing has exercised (trap 5).

Determinism is unchanged and unwidened. An export is still a process born with its work and dead
with its answer; ADR 0009 §1's claim covers exactly what it covered, and the four goldens must
not move when the transport does. Any byte that does needs a named cause.

### 4. The message thread stays the main thread; the server runs beside it

ADR 0008 §3 already has `main()` run a JUCE message loop, because `EditRenderer::render` is
asynchronous and the synchronous-looking wrapper silently produced an empty file. Preview adds a
second demand on that thread: an audio device's callback runs on the device's own thread, and a
gRPC server wants threads of its own.

The shape is the ordinary one and it is written down so PR 10 does not rediscover it: the JUCE
message thread **is** `main`, the gRPC server runs on its own threads, and everything a service
handler does to the edit or the device is posted to the message thread. Nothing touches
Tracktion off it.

That shape is unverified. M1 measured that headless works and needs no display for
*instantiating* a VST3; opening an audio device is a different question, and CI has no sound
card (trap 13). PR 0's second measurement is whether a JUCE audio device opens and plays while
the same process serves a socket, and what pumps the loop while it does. If it does not open
headlessly, preview is the first thing in this repository that cannot be tested where everything
else is tested — which is a fact about M2 either way, and ADR 0012 §5's projection golden does
not pretend to cover it.

## Alternatives considered

**The transport**

| Alternative | Rejected because |
|---|---|
| A framed protocol over a unix socket, no grpc++ | stdio renamed. It makes §3's "`app` ↔ `engine` over gRPC" false in the milestone that was supposed to make it true, and it hand-rolls framing, cancellation and streaming for a preview session — the parts of gRPC that are actually load-bearing here. |
| Put the `Render`/`Preview` server on the Rust side and keep a private engine protocol | The same objection one layer over, plus a hop: `app` would speak gRPC to a Rust shim that speaks something else to the engine, and the something else is the protocol we did not want to write. |
| gRPC at a recent release (1.6x, 1.7x) | Requires protobuf 22 or later, which is where the runtime takes abseil as a dependency — the exact line ADR 0008 §4 pinned v21.12 to stay behind. Moving it is an engine-wide protobuf upgrade and a full golden pass, in the milestone that also writes the UI. |
| gRPC v1.52.2 or earlier | Pins protobuf v21.9, not v21.12: a second protobuf commit in the tree, or ours overridden by theirs. v1.54.3 is the release where the two are one commit. |

**What a preview is**

| Alternative | Rejected because |
|---|---|
| Load a plan once, then seek and loop inside it forever, with edits applied incrementally | The incremental half is a plan diff wearing a transport, with all of its problems and none of its honesty. Seeking and looping inside a loaded plan is kept; only the incremental edit is refused. |
| A plan diff sent per edit | A second patch format, for a derived message, applied in C++. ADR 0007 §5's coverage test exists because a plan is not a model; giving it a patch format makes it one. |
| Preview renders to a temporary WAV and the UI plays that | Latency proportional to the render, no transport, and the first edit invalidates it. Also puts audio in `app`, which §15's "no audio over IPC" decision and §3 both keep out. |

**The engine's lifetime**

| Alternative | Rejected because |
|---|---|
| One long-lived process serving preview *and* `Render` | Puts ADR 0008 §2's whole argument back on the table: an export whose first block depends on what a preview played into the same plugin instance. The guarantee would go from enforced by the operating system to maintained by discipline in C++. |
| A resident process that spawns a child per export | Reaches the same guarantee through a second process-management path, inside the process, when `app` already supervises processes. Most machinery, no extra safety. |

## Consequences

- `proto/render.proto` gains `Preview`, its command and event messages, **once and early** —
  PR 3, so `buf breaking` compares one change against a `main` that has not moved (trap 12).
- §17 gains a gRPC row and `lock.baseline.json` an `engine.grpc` entry. Both consumers of that
  file read hardcoded component lists (`tests/renders.rs`'s `COMPONENTS`, and the engine job's
  `for component in …`), so the entry is inert until PR 9 adds gRPC to them and to the engine's
  reported build (ADR 0008 §5, whose list goes from seven to eight).
- The engine binary grows a mode flag and a socket path, and `core/src/engine.rs` stops writing
  to a pipe. The `renders` cargo feature and the four goldens are unchanged in content and
  changed in how they are reached — which is the whole risk of PR 9 and why it is on its own.
- Preview and export **still do not agree**, and that stays correct: sfizz uses freewheeling
  quality settings offline, Surge XT's factory patch reaches a wall-clock RNG unless
  `A Osc 1 Retrigger` is set, and the device's sample rate is the user's while the render's is
  `RenderTarget`'s. §8 already says the first; a UI that publishes a hash and plays a different
  sound is a support ticket unless it says so where the user can see it.
- Nothing here widens ADR 0009 §1. Linux x86-64, one image, one compiler.
