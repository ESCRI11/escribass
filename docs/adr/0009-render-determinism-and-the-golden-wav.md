# ADR 0009 — A render is bit-exact for one pinned toolchain on one platform, and the golden is the WAV's `data` chunk

- **Status:** Accepted (2026-09-05)
- **Affects:** `engine/` (new, M1); `tests/renders.rs` (new, M1); `docs/specs.md` §8, §11, §17
- **Builds on:** ADR 0007 (a render splits into plan and rendering), ADR 0008 §2 (a fresh
  process per render) and §5 (the engine reports what it was built from)
- **Recorded in:** `docs/specs.md` §8, §11, §15 and §17.

## Context

§2.2 says "same project file + same pinned versions → bit-identical rendered audio". §8 says
"offline render must be **bit-exact across runs** on the same platform; cross-platform
differences are documented per plugin". §11 requires "a golden-render test for every bundled
instrument (render fixture → hash compare)". §18.1 makes this differentiator number one and
§18.2 asks that render hashes be published in release notes.

Four statements of a claim, and none of them says what is compared, what "the same platform"
admits, or which of the many places a float can move is allowed to move. M0.4 learned the
expensive version of this lesson in text: two runs of the same binary agree even when both are
wrong, so only something committed earlier disagrees. Audio has the same structure and a
second problem M0.4 did not have — the container is not the content, and half the bytes in a
WAV file are neither.

PR 0 was run to answer that rather than guess it. It rendered two seconds of a built-in tone
generator to WAV, headless, on Ubuntu 24.04 x86-64 with g++ 13.3 against Tracktion `0e02f70`,
three times in three separate processes.

## Decisions

### 1. The claim: one `.escri`, one engine binary, one set of plugin binaries, one OS and CPU

**Claimed.** Same `.escri` project + same engine binary + same plugin binaries + same operating
system and CPU → **identical PCM**. In M1 that is **Linux x86-64 on Ubuntu 24.04 with g++
13.3**, the image and compiler the spike ran on and the ones decision 5 pins. Those are the
conditions under which every committed golden in M1 is valid, and the golden file records them
beside itself.

**Not claimed.** macOS, Windows, or any non-x86-64 architecture. M1 builds and goldens Linux
x86-64 and says nothing about the others — not "they differ", which would also be a claim, but
nothing. §15's minimum-supported-OS item stays `[OPEN]`; this ADR states what M1 measured, and
choosing the platforms the *product* supports is not an agent's decision (CLAUDE.md, `[OPEN]`).

**Unproven, and only CI can settle it.** Whether the same binary produces the same bytes on two
different runner CPUs of the same architecture — decision 6.

The claim is deliberately narrower than §2.2's sentence, which reads as though pinned versions
alone were sufficient. They are not: a pin fixes the source, and the bits also depend on what
the compiler did with it and what the CPU did with that. §2.2 is the goal; this is the part of
it M1 can prove, and stating the narrow version is what makes the broad one testable later
rather than quietly assumed.

### 2. The comparison is over the `data` chunk. The file is not deterministic and never was

This is the spike's finding, and it inverts what trap 2 guessed.

Three separate processes produced **byte-identical PCM** — 576,000 bytes, hashing `30fafbdc…`
every time — and **three different whole-file hashes**. JUCE's WAV writer emits a `bext` chunk
(Broadcast Wave Extension) carrying `OriginationDate` and `OriginationTime`, and the third run
happened to cross a second boundary:

```
OriginationDate  run1='2026-09-04'   run3='2026-09-04'
OriginationTime  run1='18:43:11'     run3='18:43:12'
```

So:

- **The golden comparison reads the `data` chunk and only the `data` chunk.** Chunk order,
  chunk contents outside `data`, and file length are not compared.
- **`RenderResult`'s hash is a hash of the PCM payload**, never of the file. So is the
  `.sha256` committed beside each golden WAV, which is the number §18.2 publishes in release
  notes: a release note carrying a hash that changes with the clock is worse than no hash.
- The golden WAV is committed whole, because the file is what gives a reviewer a diff and a
  listener something to play. It is the *comparison* that is narrowed, not the artefact.

Had this been left to the obvious design — hash the file — the suite would have failed roughly
once per second of build time. It would not have arrived as a finding about WAV containers. It
would have arrived as flakiness, been retried until green, and taught everyone who saw it that
the render golden is unreliable. That is the cost this ADR is paying two paragraphs to avoid,
and it is why PR 0 existed.

On mismatch the report names the **first differing sample index, the number of differing
samples, and the maximum absolute difference**. A WAV diff is otherwise two megabytes against
two megabytes; those three numbers separate "one plugin changed a coefficient" from "the whole
render moved by a block", which is M0.4's "paths, not two 40 KB blobs" applied to audio.

There is **no tolerance by default**. A plugin that proves non-deterministic gets a documented
tolerance *and* a per-plugin note in §8, in the PR that discovers it — never a tolerance
applied globally, which would hide every other plugin's drift behind the worst one's.

**Amended 2026-09-07, in PR 11: a render removes its destination first, and until it did, the
comparison read the wrong `data` chunk.** This section says the comparison is over the `data`
chunk, singular, which assumed a WAV has one. Tracktion opens the destination with
`juce::File::createOutputStream()`, and that positions at the *end* of an existing file — so a
render over a file that is already there **appends a second, complete RIFF file after the
first**. Measured: rendering one plan twice to one path leaves a file of exactly twice the size
whose `bext` origination time is still the first run's. Every reader takes the first `data`
chunk, so the engine's own read-back (ADR 0008 §3) hashed the *previous* render and reported it
as this one's, and the length check passed because the first chunk is the right length. The
engine now deletes the destination before rendering, which is both what an export means and what
makes a second render to one path a real question. The cost of not having found this is in
decision 4's Surge row.

**Settled 2026-09-06, in PR 5: the engine computes `pcm_sha256`.** This section fixed what
the hash is of and left who computes it unsaid, and the spike's note that the engine need not
hash — `juce::SHA256` lives in `juce_cryptography`, which Tracktion does not link — had been
copied into `lock.baseline.json` and `core/Cargo.toml` as though it were a decision. It was a
statement about dependency count, and the count does not move: `juce_cryptography` is a module
of the JUCE already pinned. Three things put the hash in the engine. `RenderResult` is what a
render reports about itself, and `Render.Render` (ADR 0008 §1) returns that message whole at
M2, so a field the engine never fills is a service that lies about its own answer. ADR 0008 §3
already has the engine read its output back rather than trust a return value, and the check
that landed is that the `data` chunk holds exactly the frames the plan's length comes to at
its sample rate — the walk to that chunk is the hash's input, so hashing it is one more line.
And CLAUDE.md #6 puts audio on the engine's side of the line: a RIFF walker in `core`'s
production path would be audio knowledge on the wrong side of it. `tests/renders.rs` (PR 11)
keeps a walker of its own in Rust for the first-differing-sample report, which is a test, and
it computes the hash a second way for free.

### 3. Each hazard is a build requirement, with the reason attached

None of these are style. Each is a specific way the same source produces different bits, and
each is listed with what it does so a later reader does not relax one on the grounds that it
looks arbitrary.

- **Single-threaded, fixed block size.** Float addition is not associative, so the order in
  which a mixer sums voices or tracks decides the result's low bits; a thread pool chooses that
  order by scheduling. The render runs on one thread with one block size for the whole render
  (trap 4). This also makes the next item sufficient.
- **FTZ and DAZ set on the render thread.** Denormals are both a determinism hazard and a
  performance cliff: a filter tail costs 100× more and its bits depend on a *per-thread* flag
  (trap 3). Per-thread is why it matters that decision 3's first item holds — one thread is one
  flag to set, and any future thread would need its own.
- **`-ffp-contract=off`, and no `-ffast-math` anywhere on the compile line.** Both compilers
  fuse `a*b+c` into an FMA on a capable `-march`, which is a different result from the
  unfused pair, and a vendored `CMakeLists` is free to add `-ffast-math` to its own targets
  (trap 5). PR 5 sets it explicitly for the engine and everything it compiles, and the engine
  job **greps the generated compile commands for `-ffast-math`** and fails on a hit. A flag that arrives through a submodule upgrade is
  otherwise invisible until a golden moves.
- **A pinned compile-time ISA.** The `-march` the engine and its vendored DSP compile against
  is named explicitly in `engine/`'s CMake rather than left to the compiler's default, which
  varies with distribution. The exact baseline is chosen in PR 5 against what Tracktion
  requires, and recorded in §17 with the image. This pins what the *compiler* may emit; it does
  not pin what a library chooses at *run time*, which is decision 6's problem and the reason
  cross-CPU is unproven.
- **No randomness in a fixture.** §2.2 requires every source of randomness to carry a seed
  stored in the project, and a plugin's internal RNG is a source the project cannot reach. So
  the golden fixtures disable them rather than seed them: Surge XT's random start phase and
  unison detune randomisation, sfizz's `*_random` opcodes (trap 7). A fixture that needs
  randomness to sound right is not a fixture M1 can golden. **Measured 2026-09-07, in PR 11:
  this bullet is load-bearing for Surge and the disabling is one parameter.** Surge XT at its
  factory patch renders a different hash every time; setting `A Osc 1 Retrigger`
  (`1217754326`) to 1.0 makes four fresh processes agree exactly. So `tests/renders/surge_xt`
  carries that `set_param`, and it is the fixture's whole reason for having one.

### 4. Every DSP surface carries a determinism note; there are five, not three

§8 documents cross-platform differences "per plugin", which reads as though plugins were the
only place audio is computed. They are not. Each of these gets a note in §8 recording what it
is, what makes it deterministic, and what it is known to be sensitive to:

| Surface | Note |
|---|---|
| Dexed | Pure FM synthesis, no resampling, no runtime dispatch found. Expected exact, and the cheapest of the three to bless first. |
| Surge XT | Random start phase, unison detune randomisation and noise sources, all disabled in the fixture (decision 3). **Answered 2026-09-06, in PR 6: it is not seeded from a constant — it is seeded from the wall clock.** `SurgeStorage.h` defines `STORAGE_USES_INDEPENDENT_RNG 1` and constructs its `std::minstd_rand` from `std::chrono::system_clock::now().time_since_epoch().count()`; the one call that would reseed it, `seed_rand`, is commented out. So decision 3's "disable rather than seed" is the only option here, not a preference: a fixture must avoid every path that reads it — oscillator start phase, unison detune, the sample-and-hold LFO shape and the effects that call `rand_pm1` — because nothing reachable from a plan can make them repeat. **Measured 2026-09-06, in PR 7:** the factory init patch does not reach it. One note through Surge XT at its defaults, rendered in three fresh processes, produced identical PCM (`42069eed…`), and so did a fourth run with a parameter set and a `Global Volume` lane on it. So the fixture obligation is real but narrow — it constrains which patch a fixture selects, not whether Surge can be goldened at all. |
| sfizz | Resamples, and dispatches SIMD at run time. The most likely of the three to differ across CPUs, which makes it the one decision 6's experiment must include. **Written 2026-09-07, in PR 8b**, where an SFZ was first loaded into it, and it has three parts this row did not name. **How a sample is found:** `FilePool::checkSample` joins the sample's name to the directory the SFZ was loaded from and, only if that misses, walks the path segment by segment through a case-insensitive `directory_iterator` — whose order is the filesystem's. M1's SFZ names its samples by their own asset hash, so the exact match hits and the scan is never reached; the engine refuses an SFZ it cannot resolve before sfizz sees it (ADR 0007 §2, extended), which is what keeps that fallback out of a render. **What an unresolvable sample does:** nothing audible and nothing visible — the region is dropped and the removal is a `DBG`, which a release build compiles out. Measured: an SFZ whose sample cannot be read renders a track of silence and exits zero, which is why the refusal above is a check and not a comment. **Offline is not playback:** sfizz keeps a second pair of quality settings for freewheeling (`freewheelingSampleQuality`, `freewheelingOscillatorQuality`) and switches to them when the host declares an offline render, so what a golden holds is deliberately not what a preview will play at M2. Also measured: the same note through the same SFZ hashed identically in five fresh processes (`8eb5e18d…` at 48 kHz/24-bit), and sfizz with no SFZ at all is not silent — its default patch is `<region>sample=*sine`, so a sampler that loaded nothing sounds like a sine rather than like nothing. |
| Rubber Band | A phase vocoder: modes, internal buffering and its own threading. Its configuration is pinned by ADR 0011 (offline, R3 engine, threading disabled), and it is a summation-order hazard of its own if that threading option is ever relaxed. **Written 2026-09-07, in PR 8**, and it moved in two directions. Worse than this row assumed: the *build* chooses the FFT, and the option word says nothing about that — the full build system takes FFTW, IPP, KissFFT or vDSP from whatever is installed, and a phase vocoder over two FFTs is two different signals. The engine builds upstream's `single/RubberBandSingle.cpp`, which hard-defines `USE_BUILTIN_FFT` and `USE_BQRESAMPLER`, so the choice belongs to this repository. Better than this row assumed: there is no runtime CPU dispatch anywhere in the library — every SIMD path is a compile-time `#ifdef` on `HAVE_IPP`/`HAVE_VDSP`, neither defined — so unlike sfizz it is not a candidate for decision 6's cross-CPU disagreement; and its threading is compiled out by `NO_THREADING` as well as refused by the option, with the option itself read only by the R2 engine. Measured: a stretched clip hashed identically in three fresh processes. |
| Sample-rate conversion | An asset whose sample rate differs from the render target is converted, and the converter is a DSP surface like any other. It is deterministic because JUCE is pinned by commit; M1's audio fixture uses an asset at the render rate so the golden does not depend on it, and a rate-mismatched asset is a documented note rather than a silent resample. **Written 2026-09-07, in PR 8: it is `juce::LagrangeInterpolator`, and there are two of them.** The engine converts the asset to the render's rate itself, once, before the stretch and the fades — a fixed 5-point Lagrange polynomial with no options, no runtime dispatch and a four-sample history that `reset()` zeroes, so its output is a pure function of the input and the ratio. It has no anti-alias filter, so downsampling folds; that is the note, not a defect to hide. **Written 2026-09-07, in PR 11: an asset in `assets/` has no file extension, and JUCE picks a reader by extension.** `AudioFormatManager::createReaderFor (const File&)` asks each format `canHandleFile`, which compares the extension; an asset is named by its own SHA-256 (§10), so *every* audio clip in a real project failed with "not an audio file this engine reads". PR 8's check never saw it, because it handed the engine a path of its own ending in `.wav`. The engine takes the stream overload instead, which asks each format to read the header — the only honest question about a file whose name is a hash. The second interpolator is Tracktion's: `WaveNode::processSection` reads every wave clip through a `juce::LagrangeInterpolator` even at a 1:1 ratio. At exactly 1:1 the kernel is a delta and the samples come back bit-identical — measured over a rendered clip, zero difference — but the interpolator's 2-sample base latency is not compensated, so **an audio clip sounds two samples after its position**. Deterministic, upstream's, and recorded here so that the offset in a golden is not a mystery. |

Rubber Band is the one that would have been missed. It arrives as "time-stretch, already
pinned at 4.0.0" (§17) and looks like a settled dependency, but a pinned *version* of a phase
vocoder is not a pinned *output*: the same input at two option settings is two different
signals, both correct. Trap 15 caught it; ADR 0011 pins the settings; this table is where the
obligation lives.

### 5. The CI image and the compiler are pins, not job configuration

`ubuntu-latest` moves. When it does, the compiler changes, every golden drifts at once, and
there is no PR to blame — the failing run is whichever one happened to be next (trap 12). That
is not a flaky test; it is an unpinned dependency wearing a job label.

So the engine and render jobs name **`ubuntu-24.04`**, and the compiler version is recorded in
§17 beside it, under the same rule as every other pin: an upgrade requires an ADR and a full
golden-render pass (§17). Goldens are then only ever regenerated in the PR that changes the pin
they depend on, which is §17's existing rule and M0.4's `UPDATE_FIXTURES` discipline.

The alternative — a container image pinned by digest — is stricter and is the right answer if
the runner image proves to move underneath a version label. It is not taken now because it adds
a registry dependency and a second thing to keep current, and the label plus the recorded
compiler version already makes a drift visible as a diff in the job log rather than as a
mystery.

### 6. Cross-CPU is unproven. Here is what is claimed, what would confirm it, and the retreat

sfizz and JUCE dispatch SIMD at **run time**: the same binary takes an AVX2 path on one machine
and an SSE4 path on another, and the two round differently (trap 1). Decision 3's compile-time
`-march` pin does nothing about this — it constrains what the compiler emits, not what a
library selects for itself after it starts.

The spike could not answer it: it ran on one machine.

- **Claimed today:** identical PCM on the pinned image, compiler and CPU. Every M1 golden is
  blessed under exactly that.
- **What would confirm the broader claim:** the same engine binary and the same plan producing
  the same PCM hash on two GitHub runners with different CPU models. PR 11 runs that comparison
  and reports it; the fixture must include **sfizz**, which is the surface most likely to
  disagree.
- **The retreat, if it fails:** the claim narrows rather than the tests loosening. Either the
  golden job is pinned to a single runner ISA — a self-hosted or larger runner with a fixed CPU
  — or the claim becomes same-machine and the golden becomes a local-only gate, with CI
  comparing plan bytes (ADR 0007 §4) and leaving audio to a nightly on fixed hardware. Both are
  worse; neither is a tolerance. A tolerance would make the difference between "two CPUs round
  differently" and "a plugin upgrade changed a filter" unobservable, and observing that
  difference is the product (§18.1.1).

A golden is not blessed on a claim this ADR has not yet tested. Until PR 11 reports, the M1
goldens are valid for the pinned image and CPU, and they say so.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Hash the whole WAV file | The spike proved the file is not deterministic — `bext` carries `OriginationDate` and `OriginationTime`. It would fail about once per second of build time, and arrive as flakiness rather than as a finding. |
| Strip `bext` on write and hash the file | A patched writer inside a pinned JUCE, maintained across upgrades, to make a container comparable — when the comparison only ever wanted the audio. |
| Commit only the `.sha256`, not the WAV | A hash mismatch with nothing to diff and nothing to listen to. M0.4's report exists because "two blobs differ" is not a finding. |
| A tolerance in samples, applied globally | Hides every plugin's drift behind the worst one's, and makes "the CPU rounds differently" indistinguishable from "the filter changed". Per-plugin, documented, on discovery only. |
| Claim cross-platform bit-exactness now | Unmeasured on macOS and Windows, and §15's minimum-OS item is `[OPEN]`. A claim is not a goal. |
| Claim cross-CPU on the strength of the pins | Compile-time `-march` does not constrain sfizz's and JUCE's *runtime* dispatch. Only two runners can answer it. |
| `ubuntu-latest` with the compiler pinned by apt | The image carries more than the compiler — libc, the kernel's CPU exposure, the default `-march` of the distribution's gcc. Pinning one of them is the illusion of pinning. |
| Multi-threaded render with a deterministic reduction tree | A correct reduction order can be imposed, but every dependency's own threading would need the same treatment — including Rubber Band's. Single-threaded is the same guarantee with nothing to maintain. |

## Consequences

- **PR 5** carries the flags of decision 3 into `engine/`'s CMake, the `-ffast-math` grep, and
  the `ubuntu-24.04` pin on the engine job.
- **PR 6** answers the Surge XT RNG question and produces the first per-plugin notes.
- **PR 8** applies decision 4's Rubber Band row, whose settings ADR 0011 pins.
- **PR 11** adds `tests/renders.rs` behind a cargo feature, the `data`-chunk comparison, the
  first-differing-sample report, the golden WAVs and their `.sha256`, and the two-runner
  experiment decision 6 depends on. The engine-provenance check of ADR 0008 §5 runs first.
- **§8** gains the five determinism notes of decision 4 and the platform statement of decision
  1; **§11**'s golden-render line gains what is compared; **§17** gains the image and compiler
  rows.
- **§15's minimum-OS item stays `[OPEN]`.** This ADR narrows what M1 claims; it does not decide
  what the product supports.
- `docs/plan.md`'s "Platform matrix — deferred to M1" row is answered for M1: Linux x86-64,
  named image, named compiler, cross-CPU pending PR 11.
