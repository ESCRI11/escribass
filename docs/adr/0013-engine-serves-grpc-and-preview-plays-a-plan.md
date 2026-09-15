# ADR 0013 — The engine serves gRPC, preview plays a `RenderPlan` from a tick, and an export still gets its own process

- **Status:** Accepted (2026-09-07)
- **Affects:** `engine/` (its CMake, `main.cpp`, a new server); `proto/render.proto`;
  `core/src/engine.rs`; `tests/renders.rs`; `docs/specs.md` §3, §8 and §17 — and, from M2 PR 10,
  `proto/song_tools.proto`, `core/src/session.rs`, `app/`, and §5 and §9
- **Builds on:** ADR 0008 §1 (`Render` defined in M1, implemented over gRPC at M2, and the
  stdio path deleted rather than kept), §2 (one render, one process), §3 (`main()` runs a
  message loop), §4 (the engine's C++ protobuf, pinned at v21.12), §5 (the engine reports what
  it was built from); ADR 0007 (what crosses is a `RenderPlan`); ADR 0009 §1 (what
  bit-exactness is claimed over)
- **Recorded in:** `docs/specs.md` §3, §5, §8, §9, §15 and §17.

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

**Two things were not verified here, and PR 9 settled both by building them.**

*The coexistence.* `engine/CMakeLists.txt` already did `add_subdirectory(vendor/protobuf)`, and
gRPC's `module` provider adds protobuf again — `cmake/protobuf.cmake` calls
`add_subdirectory(${PROTOBUF_ROOT_DIR})` unconditionally, with no `if(TARGET libprotobuf)` guard,
so the two collide on every target protobuf defines. The answer is neither of the two this
paragraph guessed at: **gRPC owns the one add, and `PROTOBUF_ROOT_DIR` aims it at our submodule.**
Our own `add_subdirectory` goes. The one visible consequence is a path — protobuf's binary
directory is `third_party/protobuf` under grpc's, so `protoc` moved and the engine job names it
there.

*The cost on two cores.* Measured 2026-09-09, on two pinned cores of an AMD Ryzen AI 9 HX 370 at
g++ 13.3, building `grpc++` and `grpc_cpp_plugin`: **13 m 24 s wall cold** over 1741 targets, of
which 2 m 08 s is the protobuf this job already built — so gRPC's own addition is **11 m 16 s**
(gRPC 9 m 38 s, abseil 53 s, BoringSSL 31 s, re2 12 s, c-ares and zlib under 5 s each). **Warm,
with a populated ccache and the build tree thrown away, the same 1741 targets take 6.1 s**: 1547
direct hits, and 1.8 CPU-seconds of archiving and linking in total. The engine job's ccache key
names `lock.baseline.json`, which this pull request changes, so the run that merges it pays the
cold price once and every run after it pays the warm one.

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

**Amended 2026-09-15, in PR 10, which was the first caller that had to wait on an answer.**
The shape above held, and three things it did not say had to be decided before anything could
use it.

*Which event answers which command.* `PreviewEvent` carried a tick and a state, and both kinds of
event the stream needs share it: the answer to a command, and what the transport reports of its
own accord while it plays — the tick moving, and the stop at the end of the plan. With nothing to
tell them apart, order is the only correlation, and order cannot do it: an event the transport
writes while a command is still on the wire arrives after that command was sent and before it was
applied, so a caller that took the next event as its answer would answer a seek with where the
transport was a moment before the seek. Two ways out were refused — events only in answer to
commands, which leaves an edit made while playing no way to learn where playback has reached; and
a tick-and-state heuristic, which is a guess — and one taken: **`PreviewEvent.applied`, the number
of commands the stream has applied when the event was written.** A caller waiting on its *n*-th
command waits for `applied == n` and lets every earlier event update where it last heard the
transport was. It is field 3, additive, and it is the one change `render.proto`'s preview shape
took after PR 3 settled it; `buf breaking` has nothing to say about it.

*What a replacement replaces.* A `PreviewPlay` whose plan is equal to the one loaded — compared
field by field, as protobuf defines equality — builds nothing: the transport moves to `start_tick`
and plays, and the plugin instances stay. That is the only way `PreviewStop`'s promise to keep
them means anything, since a play after a stop sends the plan it stopped. A different plan builds
a new edit, and **the loop is carried across**, because a loop is the transport's and the
transport is new — a person looping a bar while editing it keeps looping it. Not diffed, still:
the comparison decides whether to rebuild, never what to change.

*How a preview is a tool.* §5 lists `render_preview`, and the four commands become its one
request — `play`, `seek`, `loop`, `stop` — with `play` the one arm that cannot reuse the engine's
own message, because `PreviewPlay` carries a plan and the plan is `core`'s to compile from the
document, never a caller's to hand over (ADR 0007 §1). So it is `PreviewFrom { optional int32
start_tick }`, and **absent means where the transport has reached** — which is how an edit made
while playing is heard: the host applies the edit, then plays again with no tick, and `core`, which
has been reading the events, fills it in. The answer is `PreviewResponse`, carrying the engine's
event beside `valid`, `errors` and `summary` (ADR 0006 §1, extended). A **dry run** compiles and
checks, starts nothing and sends nothing — and because it moves nothing its event is the
transport's latest word, which is how a window asks where playback is without changing it. The
session holds the one live preview; a seek, loop or stop with none is refused, `preview_idle`, for
the caller to fix by playing first, and a preview that ends instead of answering is reported as an
operator error and forgotten, so the next play starts another.

### 3. Two lifetimes in one binary, one transport: a live process for preview, a fresh one per export

The engine is launched in one of two modes and the caller dials the socket it serves on. In
**preview** mode the process lives as long as its `Preview` stream. In **render** mode a fresh
process is spawned per offline render, serves exactly one `Render` call, and exits.

**Amended 2026-09-09, in PR 9, which had to write the launch.** The sentence above said the
socket path travels *to* the engine, on argv. It travels the other way: the engine creates a
`mkdtemp` directory of its own, binds a socket inside it, and prints `unix:<path>` on stdout as
its **first and only line**; the caller reads that line and dials it. Three things decide it, and
none of them was visible before there was a caller.

*Readiness.* A path on argv says where to dial and says nothing about when. The caller would
then have to poll, retry or sleep — and a retry loop tuned to a machine is the class of flake
this repository has spent two milestones refusing. Printed *after* `BuildAndStart` has returned a
listening server, one line is the address and the readiness at once: a caller holding it cannot
be refused a connection, and a blocking read of that line returns either when the engine is ready
or when the engine is gone, both of which are answers.

*Whose entropy.* A parent-chosen path has to be unique, and the two ways to make one unique are a
random name and the process id. CLAUDE.md #3 keeps unseeded randomness out of `core`, and the pid
is the exact defect M1 PR 13 found in this binary's own scratch directory — `escribass_engine.<pid>`
leaked on every killed render and pid reuse walked into it, which is why that directory became a
`mkdtemp` in the first place. The engine already owns one. Naming the socket inside it costs
nothing, is 0700 by construction, and dies with the process that made it.

*Which stdout.* ADR 0008 §1's rule — stdout carries the protocol and nothing else, every log line
goes to stderr — is unchanged and is what makes a single line readable at all. What crosses it is
one address instead of one `RenderResult`, and it is checked after the flush for the reason M1 PR
13 added that check: an answer that never left the process used to be exit 0 and a caller waiting.

`core` therefore spawns `escribass_engine <manifest.json> --render`, reads one line, dials, calls
once, and takes the **child's exit status** as the verdict — never the transport's. A refused
connection, a stream that ends mid-call and a `Status` in place of a message all mean the engine
is not going to answer, and the report a person can act on is the one the engine already wrote:
its exit code and the tail of its stderr. A non-zero exit is `engine_failed`; an exit of 0 with no
answer is `engine_unreadable`, which is M1 PR 13's defect — a render that never happened, reported
as a success — kept caught on the new transport.

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

**Amended 2026-09-15, in PR 10, which built the other lifetime.** Three things are now enforced
rather than described.

*The device before the address.* A preview process opens the machine's default output while its
Tracktion engine is constructed, and **only then** serves: with no device it exits 6 having said
which device types it found and what they listed, before any socket exists. A caller therefore
meets a machine with no sound card as `engine_failed` carrying that sentence — never as a stream
that answers every command and plays nothing — and a runner with no sound card is where that is
checked (the engine job).

*An export is headless, checked.* The whole difference between the modes is one boolean passed to
the engine's `EngineBehaviour`, and with it false Tracktion registers no audio device type at all.
So `render` refuses to proceed if it was offered one, and because that holds on a runner exactly as
on a desk with a speaker, every render in CI is the check — not a golden that would only have moved
where there is a device to move it.

*gRPC's refusal, observed.* An engine started with `--render` registers `Render` alone, and a
`Preview` stream opened on it is answered `UNIMPLEMENTED` by gRPC's dispatch. `tests/renders.rs`
asserts that against the real binary — and a mutant engine that also registered a preview service
failed it — so "an export never shares a process with playback" is a test and not a sentence. The
live preview is held by `core`'s session, and an export while it plays is a second process in
`--render` mode, which `core/tests/preview.rs` counts.

### 4. The message thread stays the main thread; the server runs beside it

ADR 0008 §3 already has `main()` run a JUCE message loop, because `EditRenderer::render` is
asynchronous and the synchronous-looking wrapper silently produced an empty file. Preview adds a
second demand on that thread: an audio device's callback runs on the device's own thread, and a
gRPC server wants threads of its own.

The shape is the ordinary one and it is written down so PR 10 does not rediscover it: the JUCE
message thread **is** `main`, the gRPC server runs on its own threads, and everything a service
handler does to the edit or the device is posted to the message thread. Nothing touches
Tracktion off it.

**Measured 2026-09-15, in PR 10 — and the shape held.** This paragraph said the shape was
unverified and gave the measurement to PR 0, which never took it. PR 10 took it before building on
it, on an AMD Ryzen AI 9 HX PRO 370 under WSL2 (Ubuntu 24.04, g++ 13.3), with the engine this PR
builds and a real `Preview` stream from `core`.

*What pumps the loop:* the preview mode's own `runDispatchLoopUntil (10)`, turned by `main` for as
long as the stream is open — the same call an export turns while it waits for its render (ADR 0008
§3). Between turns, on `main`, the engine takes the commands that have arrived, applies them to the
edit and the transport, and hands back the events to write. The device pulls blocks on a thread of
its own (JUCE's ALSA thread), gRPC reads on one thread and writes on another, and Tracktion is
touched on `main` alone. **One refinement of the sentence above:** "posted to the message thread"
is a queue under a mutex that the loop drains on each turn, not `MessageManager::callAsync` — for
the reason `Render`'s handler already polls: a message posted from gRPC's thread can outlive the
edit it names, and a queue drained by the one thread that owns the edit cannot.

*Whether a device opens here: no, and that is this machine's, not the design's.* JUCE on Linux is
built with ALSA and without JACK (`JUCE_ALSA=1`, `JUCE_JACK=0`), and speaks neither PulseAudio nor
PipeWire. This machine has no ALSA sound card (`/dev/snd` holds only `timer`) and no ALSA plugin
directory, so ALSA lists **0 output devices** and the preview exits 6 saying so. The sound it has
is WSLg's PulseAudio server, which ALSA reaches only through the `pulse` PCM plugin: **a person
here installs `libasound2-plugins`** (Ubuntu 24.04 universe, 1.2.7.1-1ubuntu5) **and points ALSA's
default at it** — the package ships the file as `/etc/alsa/conf.d/99-pulseaudio-default.conf.example`,
so `sudo cp` it to the same name without `.example`, or put its two stanzas, `pcm.!default { type
pulse }` and `ctl.!default { type pulse }`, in `~/.asoundrc`. A machine on PipeWire needs
`pipewire-alsa` instead. None of that was installed here.

*What was measured instead, and what it is worth.* ALSA ships a `null` PCM that needs no card and
no plugin, and a process-local `~/.asoundrc` naming it the default gave JUCE a real
`ALSAAudioIODevice` — `ALSA 'default'` at 44 100 Hz in blocks of 512 — whose thread called the
engine's callback while the same process served the stream. Against it, from `core`'s side of the
socket: the first play answered in **187–197 ms**, three runs, which is process start, JUCE and
Tracktion, the device and a Surge XT edit together; every command after it in **9–11 ms**, which is
the dispatch loop's ten-millisecond turn and nothing else; a play of the same plan in that same
turn, building nothing, where a plan changed by an edit rebuilt in 48 ms. A loop of one beat was
seen to wrap while playing, a seek sent while playing landed on its tick, a stop answered
`STOPPED`, the transport stopped itself at the end of the plan, and closing the stream stopped it
and the process **exited 0** without being killed. That establishes the thread shape and the loop,
and nothing about sound or time: the null PCM does not block, so the transport ran several times faster than
real time, and no speaker was involved. **Not measured on any machine, and said so:** audio
reaching a speaker, real-time pacing, underruns under load, and a device rate other than 44.1 kHz.
`tests/renders.rs` carries all of it as an ignored test that runs on a machine with a device, and
passes against the null PCM.

Two things the measurement showed that no document had. **Tracktion's reported position is its
own UI's**: `TransportControl::getPosition()` is refreshed from the playhead by a timer on the
message thread, and ignored for 200 ms after a `setPosition` (`lastUserDragTime`, a
`Time::getMillisecondCounter` debounce), so what a `PreviewEvent` reports can trail the audio by a
timer period and stands still for a moment after a seek. It moves no sample and decides only what
is reported. And **a real export of `tests/determinism/render` crashes the engine** — `corrupted
double-linked list`, killed by a signal, after Rubber Band warns that a stretch ratio of 0.0853
"yields ideal inhop < minimum" — on `main`'s engine as on this one. It is M1's audio-clip path, not
preview's, and is recorded for the review in `docs/plan.md` rather than fixed here.

So preview **is** the first thing in this repository that cannot be tested where everything else is
tested, as trap 13 predicted, and ADR 0012 §5's projection golden does not pretend to cover it. What
is tested without a device is everything that is not sound (`core/tests/preview.rs`, the
determinism suite, the engine job); what needs one is an ignored test that says why every time the
render suite runs.

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
- **And the reported build is not only reported.** The engine's provenance is one list, and
  `--scan` writes it into the build manifest's `engine` block, which `Project::write` copies
  into every project's `lock.json` (ADR 0010 §1, §4). So gRPC becoming an engine submodule
  commit reaches `tests/fixtures/manifest.json` and five determinism goldens — by exactly one
  line each, the pin itself, with no `song.json`, patch or plan touched. That is `tests/AGENTS.md`'s
  own rule ("moving a §17 pin moves the fixture and the goldens in one pull request") arriving,
  and it is the right answer rather than an accident: ADR 0010 §1 says `lock.json` records the
  engine's submodule commits, and a second, shorter list for the manifest would be the drift
  ADR 0008 §4 refuses one layer down. **Found by CI**, in the step that exists to compare the
  two files, which is the only place both exist at once.
- The engine binary grows a mode flag and names its own socket, and `core/src/engine.rs` stops
  writing to a pipe. The `renders` cargo feature and the four goldens are unchanged in content
  and changed in how they are reached — which is the whole risk of PR 9 and why it is on its
  own. **Done 2026-09-09, and the goldens did not move**: `audio_clip`, `dexed` and `surge_xt`
  reproduced their committed bytes over the new transport before the old one was deleted, and
  the three CI render legs carry `sfizz`, whose VST3 will not load on the author's machine.
- The engine job's six shell steps needed a client, because a shell script cannot dial gRPC.
  They drive `tests/render_once.rs`, which is `core`'s own `Engine` behind an argv — a client in
  the engine's own C++ build would have been a second implementation of this boundary, which is
  ADR 0006 §1's objection one boundary over.
- Preview and export **still do not agree**, and that stays correct: sfizz uses freewheeling
  quality settings offline, Surge XT's factory patch reaches a wall-clock RNG unless
  `A Osc 1 Retrigger` is set, and the device's sample rate is the user's while the render's is
  `RenderTarget`'s. §8 already says the first; a UI that publishes a hash and plays a different
  sound is a support ticket unless it says so where the user can see it. **Said so, from PR 10:**
  the window's play button sits beside the words *live preview · not the render*, with the three
  reasons on hover; nothing a preview reports carries a hash; and the window shows no render hash
  at all (M2 trap 4).
- Nothing here widens ADR 0009 §1. Linux x86-64, one image, one compiler.
