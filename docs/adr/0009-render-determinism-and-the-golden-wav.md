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
different runner CPUs of the same architecture — decision 6. **Measured 2026-09-07, in PR 11,
and it moved but did not close:** all four goldens reproduce bit-exactly across two different
x86-64 CPU models, sfizz included. The claim above is unchanged, because what the measurement
covers is narrower than what it looks like it covers; decision 6 says exactly what was and was
not held constant.

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

**Extended 2026-09-07, in PR 13: the render goes to a sibling `.part` and is renamed onto the
destination only after every check has passed.** Deleting the destination up front answers the
appending, and it opens a second hole in the same place: a render killed halfway left a
truncated WAV — measured at 43 MB, its `data` chunk declaring more than the file held — exactly
where the last good render had been, with the engine never reaching its own length check to say
so. A POSIX `rename` is the fix and it subsumes the first one: what a reader sees is the old
file or the new one and never half of either, and the `.part` a render writes is a path nobody
else writes. (`std::rename`, not `juce::File::moveFileTo`, which unlinks the destination before
renaming and so has a window where neither file is there.)

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
| Dexed | Pure FM synthesis, no resampling, no runtime dispatch found. Expected exact, and the cheapest of the three to bless first. **Corrected 2026-09-07, in PR 13: "no random number generator" was wrong, on exactly one path.** `msfa/lfo.cc` runs an 8-bit LCG — `randstate_ = (randstate_ * 179 + 17) & 0xff` — for LFO waveform 5, sample-and-hold, and `randstate_` is a `uint8_t` on a class with no constructor that neither `reset()` nor `keydown()` initialises. Indeterminate memory, not a constant seed, so decision 3's "disable rather than seed" applies to it exactly as it does to Surge XT: **LFO waveform 5 is forbidden in a fixture**, beside Surge's `rand_pm1` and sfizz's `*_random`. Nothing is wrong today — the DX7 init voice is waveform 0, and every other waveform reads `phase_` alone — which is precisely why this belongs in writing rather than in a golden that happens to pass. It matters more here than the count of RNGs suggests: §8 introduced these notes as "what M1 PR 6 read in the pinned sources, not reputation", and Dexed is the instrument the bar-17 demo chose *because* this row called it the cheapest deterministic one. |
| Surge XT | Random start phase, unison detune randomisation and noise sources, all disabled in the fixture (decision 3). **Answered 2026-09-06, in PR 6: it is not seeded from a constant — it is seeded from the wall clock.** `SurgeStorage.h` defines `STORAGE_USES_INDEPENDENT_RNG 1` and constructs its `std::minstd_rand` from `std::chrono::system_clock::now().time_since_epoch().count()`; the one call that would reseed it, `seed_rand`, is commented out. So decision 3's "disable rather than seed" is the only option here, not a preference: a fixture must avoid every path that reads it — oscillator start phase, unison detune, the sample-and-hold LFO shape and the effects that call `rand_pm1` — because nothing reachable from a plan can make them repeat. ~~**Measured 2026-09-06, in PR 7:** the factory init patch does not reach it.~~ **Corrected 2026-09-07, in PR 11: it does, and PR 7 was reading a file it had not rewritten.** Those three "fresh processes" rendered to one output path, and decision 2's amendment says what that produced: runs two and three appended a second RIFF file, and every reader — the engine's own read-back included — took run one's `data` chunk. Three identical hashes were one render counted three times. With the engine deleting its destination first, Surge XT at its factory patch renders **a different hash every run**: five fresh processes, five hashes. So the obligation is the wide one this row states above, and the path the factory patch reaches is the first one named — the oscillator's random start phase. Setting `A Osc 1 Retrigger` (`1217754326`) to 1.0 makes four fresh processes agree exactly, and that is the one `set_param` in `tests/renders/surge_xt`. |
| sfizz | Resamples, and dispatches SIMD at run time. The most likely of the three to differ across CPUs, which makes it the one decision 6's experiment must include. **Written 2026-09-07, in PR 8b**, where an SFZ was first loaded into it, and it has three parts this row did not name. **How a sample is found:** `FilePool::checkSample` joins the sample's name to the directory the SFZ was loaded from and, only if that misses, walks the path segment by segment through a case-insensitive `directory_iterator` — whose order is the filesystem's. M1's SFZ names its samples by their own asset hash, so the exact match hits and the scan is never reached; the engine refuses an SFZ it cannot resolve before sfizz sees it (ADR 0007 §2, extended), which is what keeps that fallback out of a render. **What an unresolvable sample does:** nothing audible and nothing visible — the region is dropped and the removal is a `DBG`, which a release build compiles out. Measured: an SFZ whose sample cannot be read renders a track of silence and exits zero, which is why the refusal above is a check and not a comment. **Offline is not playback:** sfizz keeps a second pair of quality settings for freewheeling (`freewheelingSampleQuality`, `freewheelingOscillatorQuality`) and switches to them when the host declares an offline render, so what a golden holds is deliberately not what a preview will play at M2. Also measured: the same note through the same SFZ hashed identically in five fresh processes (`8eb5e18d…` at 48 kHz/24-bit), and sfizz with no SFZ at all is not silent — its default patch is `<region>sample=*sine`, so a sampler that loaded nothing sounds like a sine rather than like nothing. |
| Rubber Band | A phase vocoder: modes, internal buffering and its own threading. Its configuration is pinned by ADR 0011 (offline, R3 engine, threading disabled), and it is a summation-order hazard of its own if that threading option is ever relaxed. **Written 2026-09-07, in PR 8**, and it moved in two directions. Worse than this row assumed: the *build* chooses the FFT, and the option word says nothing about that — the full build system takes FFTW, IPP, KissFFT or vDSP from whatever is installed, and a phase vocoder over two FFTs is two different signals. The engine builds upstream's `single/RubberBandSingle.cpp`, which hard-defines `USE_BUILTIN_FFT` and `USE_BQRESAMPLER`, so the choice belongs to this repository. Better than this row assumed: there is no runtime CPU dispatch anywhere in the library — every SIMD path is a compile-time `#ifdef` on `HAVE_IPP`/`HAVE_VDSP`, neither defined — so unlike sfizz it is not a candidate for decision 6's cross-CPU disagreement; and its threading is compiled out by `NO_THREADING` as well as refused by the option, with the option itself read only by the R2 engine. Measured: a stretched clip hashed identically in three fresh processes. |
| Sample-rate conversion | An asset whose sample rate differs from the render target is converted, and the converter is a DSP surface like any other. It is deterministic because JUCE is pinned by commit; M1's audio fixture uses an asset at the render rate so the golden does not depend on it, and a rate-mismatched asset is a documented note rather than a silent resample. **Written 2026-09-07, in PR 8: it is `juce::LagrangeInterpolator`, and there are two of them.** The engine converts the asset to the render's rate itself, once, before the stretch and the fades — a fixed 5-point Lagrange polynomial with no options, no runtime dispatch and a **five**-sample history that `reset()` zeroes (`Interpolators::Lagrange` is `GenericInterpolator<LagrangeTraits, 5>`; "four" corrected 2026-09-07 in PR 13), so its output is a pure function of the input and the ratio. It has no anti-alias filter, so downsampling folds; that is the note, not a defect to hide. **Written 2026-09-07, in PR 11: an asset in `assets/` has no file extension, and JUCE picks a reader by extension.** `AudioFormatManager::createReaderFor (const File&)` asks each format `canHandleFile`, which compares the extension; an asset is named by its own SHA-256 (§10), so *every* audio clip in a real project failed with "not an audio file this engine reads". PR 8's check never saw it, because it handed the engine a path of its own ending in `.wav`. The engine takes the stream overload instead, which asks each format to read the header — the only honest question about a file whose name is a hash. The second interpolator is Tracktion's: `WaveNode::processSection` reads every wave clip through a `juce::LagrangeInterpolator` even at a 1:1 ratio. At exactly 1:1 the kernel is a delta and the steady state comes back bit-identical, but the interpolator's 2-sample base latency is not compensated, so **an audio clip sounds two samples after its position**. **Re-measured 2026-09-07, in PR 13, because a review read Tracktion's compensation code and concluded the offset was not real.** It is: a ramp asset whose every sample is identifiable, placed at tick 480 of a 120 bpm 48 kHz render, comes back with source sample *k* at output frame 12002 + *k*, and the clip's last two source samples fall off the end of its window. What the review found is also real and is the missing half of this note — Tracktion *does* compensate, in `LagrangeResamplerReader::readSamples`, which reads `getBaseLatency()` extra source frames on the first block and drops the matching destination frames under a `hasBeenReset` that initialises `true`. That reader belongs to `WaveNodeRealTime`, and upstream's commit says so in its own subject line: `319afc0` (2024-07-23) removed the latency "when using `AudioClipBase::setUsesProxy (false)`". A clip the engine places `canUseProxy()`, so `EditNodeBuilder` builds the legacy `WaveNode` instead, which has no such path; the commit is an ancestor of our pin and changes nothing for a proxied clip. Deterministic, upstream's, and recorded here so that the offset in a golden is not a mystery. |

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

**Run 2026-09-07, in PR 11. Nothing failed, nothing retreated, and the experiment as written
is still open.** Read the three results separately, because they are not the same result:

1. **Same binary, three runners, one CPU model.** The `renders` matrix rendered all four
   goldens on three `ubuntu-24.04` runners with the binary the `engine` job built and uploaded.
   All three reproduced the committed bytes exactly. GitHub gave all three the **same** CPU —
   `AMD EPYC 7763 64-Core Processor` — so this says the binary is reproducible and says
   *nothing at all* about cross-CPU. The `cross-cpu` job reports that as **inconclusive**, in
   those words, rather than as a pass; it will say something different the run a second model
   turns up, and that is now standing rather than a thing someone has to remember to try.
2. **Two builds, two CPU models, identical PCM.** The goldens were blessed on the development
   machine — `AMD Ryzen AI 9 HX PRO 370`, Zen 5, `avx512f` present — on Ubuntu 24.04 with
   g++ 13.3.0, and reproduced byte for byte on the EPYC 7763, Zen 3, with **no `avx512f`** and
   a separately compiled engine. All four fixtures, sfizz included. That is a stronger
   statement than decision 1 makes in one respect (two builds, not one binary) and it is
   genuinely two CPU models.
3. **What result 2 does not show, and this is the part worth writing down.** Trap 1's hazard is
   a *dispatcher choosing differently* — AVX2 here, SSE4 there. It did not happen here. What
   result 2 measures is that the *same* SIMD path rounds identically on two microarchitectures,
   which is worth knowing and is not the question.

**Amended 2026-09-07, in PR 13, and it is worse than "not shown".** This decision put sfizz in
the experiment because §8 said its dispatcher "selects scalar, SSE or AVX per operation", and
result 3 above reasoned from that — both runners have AVX, so both took the AVX path. Read at
the pin, that is not what the code does. `SIMDHelpers.cpp:99-106` is the AVX switch and its
whole body is `default: break;`; the SSE switch below it carries 21 real cases. No operation is
ever pointed at an AVX implementation, the three translation units compiled `-mavx` build code
the dispatcher never installs, and every x86-64 CPU has SSE. The library's one live runtime AVX
choice is `effects/Strings.cpp:38-39`, which picks `ResonantArrayAVX` for the `strings` effect;
a plain `<region>` fixture does not reach it.

So the honest answer is that **the experiment as built cannot answer trap 1 through sfizz at
this pin**, and not that it happened to draw two similar CPUs. There is no dispatcher on the
render path left to choose differently: `tests/renders/sfizz` takes the same code on every
machine that can run the binary. Two runners disagreeing would be evidence of something, but
two runners agreeing is no longer even weak evidence for the broader claim.

Decision 1's claim therefore stays exactly where it was, for a reason one level deeper than the
one this section gave. What would change the answer is a fixture that reaches a live
dispatcher — the `strings` effect, or a sfizz bump that fills the AVX switch in — and until
there is one, the cross-CPU question is open on the mechanism and not merely on the sample.
Widening it on one pair of AMD parts that provably took the same code path would be the "a
claim is not a goal" this ADR already refuses in its alternatives table.

**Amended 2026-09-30, in M4 PR 3. The experiment finally sampled real hardware variety, and one
of the draws was an Intel part.** The repository is public (plan, U2, decided 2026-09-25), CI
executed steps again on 2026-09-30 after a milestone and a half of refused runs, and `cross-cpu` —
written in PR 11 on 2026-09-07 — ~~**ran for the first time ever**~~ **ran for the first time
since 2026-09-09** (corrected 2026-10-01 in M4 PR 7; see the amendment below, which also
corrects the Intel sentence further down). It has now run three times:

| Run | Pull request | Runner | CPU | SIMD its dispatchers could pick from |
|---|---|---|---|---|
| `36744002164` | #85, `m4.2-schema` | 1 | `AMD EPYC 7763 64-Core Processor` | `avx avx2 sse4_2` |
| `36744002164` | #85, `m4.2-schema` | 2 | `AMD EPYC 9V45 96-Core Processor` | `avx avx2 avx512f sse4_2` |
| `36744002164` | #85, `m4.2-schema` | 3 | `AMD EPYC 9V45 96-Core Processor` | `avx avx2 avx512f sse4_2` |
| `36750716757` | #86, `m4.3-proto` | 1 | `AMD EPYC 7763 64-Core Processor` | `avx avx2 sse4_2` |
| `36750716757` | #86, `m4.3-proto` | 2 | `AMD EPYC 9V74 80-Core Processor` | `avx avx2 avx512f sse4_2` |
| `36750716757` | #86, `m4.3-proto` | 3 | `AMD EPYC 9V74 80-Core Processor` | `avx avx2 sse4_2` |
| `36753140163` | #86, `m4.3-proto` | 1 | **`Intel(R) Xeon(R) 6973P-C`** | `avx avx2 avx512f sse4_2` |
| `36753140163` | #86, `m4.3-proto` | 2 | `AMD EPYC 7763 64-Core Processor` | `avx avx2 sse4_2` |
| `36753140163` | #86, `m4.3-proto` | 3 | `AMD EPYC 9V74 80-Core Processor` | `avx avx2 avx512f sse4_2` |

**Four model strings, two vendors, nine render jobs, thirty-six golden comparisons, and every one
of them identical.** Each job rendered all four fixtures — `audio_clip`, `dexed`, `sfizz`,
`surge_xt` — and compared its own PCM against the committed bytes with no tolerance
(`every_fixture_still_renders_what_was_committed`), and rendered each twice in two fresh processes
beside it. In every run `engine` built **one** binary that the three `renders` jobs downloaded, so
each row is the same machine code on different hardware rather than two builds agreeing. The
sampling is ongoing by construction: every pull request adds three rows, and this table is the
three runs that existed when it was written.

**Two things in the table that were not expected, one in each direction.**

The first is `Intel(R) Xeon(R) 6973P-C` on the last run. ~~Until it appeared, this section could
say that no Intel part had ever executed a golden render for this repository, and an earlier
draft of this amendment said exactly that, as a limit, two hours before the run that disproved
it.~~ **That sentence was false when it was written, and the amendment below has the runs that
disprove it.** **The cross-vendor x86-64 case has now been exercised, and it agreed**: an Intel
Xeon and two AMD EPYCs, in one run, from one binary all three downloaded, produced the committed
PCM for all four fixtures. ~~That is the half of §6's question this repository has carried as
unanswerable since M1~~ — it was exercised at M1 and nobody read the log. It is also **one run
with one Intel part**, which is a sample and not a rate, and the table is how it stays one.

The second is `AMD EPYC 9V74` appearing twice on run `36750716757` with **different feature sets**
— one runner advertising `avx512f` and one not, masked by the hypervisor. A model name is
therefore not a proxy for what a runtime dispatcher can see, and `cross-cpu`'s count of "2 models"
on that run under-reported a sample that held three feature sets.

**What this is, and the limits that survive it.** It is stronger than result 2, which was two
builds with one blessed on the development machine and a pairing that was chosen: this is one
binary on parts nobody selected, three times, and the parts differ in a feature a dispatcher could
key on. Anything on the render path selecting by AVX-512 has had six chances across the three runs
to choose differently — including between two runners bearing one model name, and between two
vendors — and not one of thirty-six hashes moved.

What is still **not** shown, and the wording does not drift past it: every part is an **x86-64
server processor**, so this is nothing about ARM or any other ISA and nothing about a consumer
CPU; every runner is **Ubuntu 24.04 on GitHub's hosted pool**, so it is nothing about macOS or
Windows; the goldens themselves are still blessed on one developer machine; and the Intel row is
one draw.

**Decision 1's claim does not move, and the reason is the mechanism rather than timidity.** §1
stays "identical PCM on the pinned image, compiler and CPU", with this section as the evidence
beside it. Two vendors agreeing is a strong sample answer and it is not a mechanism answer: the
paragraph below is still true, and it is vendor-independent. Until a fixture reaches a live
runtime dispatcher, every row in that table is the *same* code path executing on different
silicon, which is a real and reassuring thing to know and is not the hazard trap 1 named. What
would let §1 and §2.2 widen is the table continuing to fill this way across many pull requests
*and* a fixture that gives a dispatcher a choice to make.

**And the PR 13 mechanism finding is untouched.** sfizz's AVX switch is still empty at this pin,
so `tests/renders/sfizz` took the same SSE code in all nine jobs, and *its* agreement is still not
evidence about a dispatcher choosing differently. What the three runs add is that the hazard was,
for the first time, given somewhere to appear — across two vendors and four feature sets. It did
not appear. The fixture that would make the test conclusive is still the one this section named:
something that reaches a live dispatcher, the `strings` effect or a sfizz bump that fills the AVX
switch in.

**Amended 2026-10-01, in M4 PR 7, which walked the run history instead of the last three runs.
Two of the sentences above are wrong, and they are wrong in opposite directions.**

`cross-cpu` has executed — both of its gated steps `success` — in ~~**thirty-seven**~~
**forty-three** runs (thirty-seven on 2026-10-01; **recounted 2026-10-02 at M4's close**, where
the six executions of M4 PRs 7 and 8 and their merges had already made the number stale — a count
of one's own evidence goes out of date by the evidence arriving, which is the argument for dating
it rather than stating it), not three. Twenty-five of them are M1's and M2's, from run
`34120276911` (PR #50, `m1.11-goldens`, 2026-09-07, the run that wrote the job) to run
`34392687932` (the push of `17356bd`, 2026-09-09, the last green run on `main` before the
blackout). Their model counts, in order,
are 1, 1, 1, 2, 3, 1, 1, 3, 1, 2, 3, 1, 2, 2, 2, 2, 2, 2, 3, 2, 2, 1, 1, 3, 2. So the job ran
for the first time **since 2026-09-09**, not for the first time ever, and "the first time since
it was written" is the sentence that should have been written.

**And an Intel part had rendered these goldens three weeks before the row that says none had.**
Run `34124486045` (PR #51, `m1.12-locality`, 2026-09-07, *the day after* the job was written)
drew `AMD EPYC 7763 64-Core Processor` and `Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz` and
reported "CPU models seen: 2"; run `34390985402` (PR #62, `m2.8-history`, 2026-09-09) drew three,
including `INTEL(R) XEON(R) PLATINUM 8573C`. Every one of those runs compared the four committed
PCM hashes with no tolerance and every one agreed. **So the cross-vendor x86-64 case was
exercised at M1 and the evidence sat in a run log nobody read**, while this section, the plan and
`specs.md` §11 all went on calling it unanswerable for two milestones. What was true in the
2026-09-07 amendment is narrower and survives: *PR 11's own run* drew one model three times, and
that is what it said.

That is the finding, and it is not about CPUs. **A claim this repository makes about its own
evidence is worth exactly as much as the last walk of the evidence**, and the walks here have
been of the last three runs rather than of the record. The rule §6a already states — a
cross-CPU claim names the runs it rests on — is what makes the correction possible at all, and
it is now also the rule for *reading*: before writing "for the first time" or "never", list the
runs.

**The record, completed.** Six model strings have executed a golden render for this repository,
across two vendors: `AMD EPYC 7763 64-Core Processor`, `AMD EPYC 9V74 80-Core Processor`,
`AMD EPYC 9V45 96-Core Processor`, `Intel(R) Xeon(R) 6973P-C`,
`Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz` and `INTEL(R) XEON(R) PLATINUM 8573C`. The
~~twelve~~ **eighteen** executions since CI came back, in order, with the pull request and the
count — the last six added 2026-10-02 at M4's close, which is what keeps this table a record
rather than a snapshot:

| Run | Ref | Models |
|---|---|---|
| `36742529816` | `main`, #84's merge | 2 — EPYC 7763, EPYC 9V74 |
| `36744002164` | #85, M4 PR 2 | 2 — EPYC 7763, EPYC 9V45 |
| `36748500385` | `main`, #85's merge | **1** — EPYC 7763 ×3, warned |
| `36750716757` | #86, M4 PR 3 | 2 — EPYC 7763, EPYC 9V74 ×2 (the pair with different flags) |
| `36753140163` | #86, M4 PR 3 | 3 — EPYC 7763, EPYC 9V74, **Xeon 6973P-C** |
| `36754774610` | #86, M4 PR 3 | 2 — EPYC 7763 ×2, EPYC 9V74 |
| `36830201753` | `main`, #86's merge | 3 — EPYC 7763, EPYC 9V74, **Xeon Platinum 8573C** |
| `36834898970` | #87, M4 PR 4 | 3 — EPYC 7763 `[avx avx2 sse4_2]`, EPYC 9V74 same, **Xeon 6973P-C `[avx avx2 avx512f sse4_2]`** |
| `36860277354` | `main`, #87's merge | 3 — the same three lines |
| `36867215121` | #88, M4 PR 5 | 2 — EPYC 7763 ×2 `[avx avx2 sse4_2]`, **Xeon Platinum 8370C `[… avx512f …]`** |
| `36871836825` | `main`, #88's merge | **1** — EPYC 7763 ×3, warned |
| `36876840013` | #89, M4 PR 6 | 3 — EPYC 7763, and **two EPYC 9V74s split by their flags**, one with `avx512f` and one without |
| `36887266727` | `main`, #89's merge, **attempt 2** | 3 — EPYC 7763, EPYC 9V74 `[… avx512f …]`, **Xeon 6973P-C `[… avx512f …]`** |
| `36893329861` | #90, M4 PR 7 | 2 — EPYC 7763 ×2, **Xeon Platinum 8370C `[… avx512f …]`** |
| `36917300767` | #90, M4 PR 7 | 3 — EPYC 7763, EPYC 9V74 `[avx avx2 sse4_2]`, **XEON PLATINUM 8573C `[… avx512f …]`** |
| `36980841282` | `main`, #90's merge | 3 — EPYC 7763, EPYC 9V74 `[avx avx2 sse4_2]`, **EPYC 9V45 `[… avx512f …]`** — the first 9V45 to be counted with its flags |
| `36992228042` | #91, M4 PR 8 | 3 — EPYC 7763, EPYC 9V74 `[… avx512f …]`, **Xeon 6973P-C `[… avx512f …]`** |
| `36995454955` | `main`, #91's merge | 2 — EPYC 7763, EPYC 9V74 `[… avx512f …]` ×2 |

From `36834898970` the line carries the flags as well as the model (~~a ledger row §6a left
open~~ — **closed 2026-10-01 in M4 PR 4**, and §6a's own last paragraph is struck for it), which
is why the AVX-512 column only exists from there — and why `36876840013` reads 3 where the old
counter would have read 2. **AVX-512 against non-AVX-512 in one run, from one binary, is recorded
~~four times~~ ten** — the four of 2026-10-01 (`36834898970`, `36860277354`, `36867215121`,
`36876840013`) and every one of the six added above, which is every flagged run but
`36871836825`'s one-model draw — one of them, `36876840013`, between two runners bearing the
*same* model name. Nothing moved: every render job in every one of the **forty-three** executions
reproduced all four committed hashes, M1's and M2's twenty-five included.

The limits in the paragraph below are unchanged by any of this, and one is sharper for it:
every part in the whole record, M1's included, is an **x86-64 server processor on GitHub's
hosted Ubuntu pool**, and the goldens are still blessed on one developer machine.

### 6a. The job warns rather than fails on a one-model draw, and that stays

`cross-cpu` passes with a `::warning` when every runner draws the same model, so a **green
`cross-cpu` does not by itself mean two CPUs were compared**. That is not hypothetical and not
rare: PR 11's own run drew one model three times, and on pull request #84 the job reported success
with both of its real steps skipped behind the engine gate. The three runs above drew **2, 2 and
3** models — the pool assigns them and nothing here can ask — so variety is luck, and the next run
may draw one model three times and go green having compared nothing.

**Decided 2026-09-30: it stays a warning.** Failing on a one-model draw would be worse than the
gap it closes. The runner pool is not ours to ask anything of, so a failing job would be re-run
until it drew two models — which is selecting the evidence rather than collecting it, and is the
"a claim is not a goal" this ADR refuses in its own alternatives table. A required check that goes
red for reasons unrelated to the change is also the check people learn to ignore, and this one has
exactly one job: to be believed the day it disagrees.

**Confirmed 2026-10-01, in M4 PR 7, with the rate.** Of the ~~twelve~~ **eighteen** executions
since CI came back, **two drew one model and warned** (`36748500385` and `36871836825`, both
pushes to `main`), and of M1's and M2's twenty-five, ~~**eight**~~ **nine** did. ~~So a green
`cross-cpu` means "compared" about four times in five~~ **So eleven of forty-three executions
warned, and a green `cross-cpu` means "compared" about three times in four** — and the exception
is not rare.

**Recounted 2026-10-02 at M4's close, and the correction is a subtraction this ADR could have
made for itself.** "Eight" was wrong the day it was written: §6's own list of M1's and M2's
twenty-five counts, nine lines above, prints `1, 1, 1, 2, 3, 1, 1, 3, 1, 2, 3, 1, 2, 2, 2, 2, 2,
2, 3, 2, 2, 1, 1, 3, 2`, and that sequence holds **nine** `1`s — confirmed independently against
the `##[warning]` annotations of all forty-three logs, which give eleven warned and thirty-two
noticed. So the data in §6 and the summary in §6a disagreed, in the same pull request, about a
number one of them had already written down; and the rate the summary drew from it was the right
fraction of the wrong window — four-in-five is the twelve post-blackout runs alone, not the record
the sentence claims to be about. **§6's own rule — before writing "for the first time" or
"never", list the runs — needs a second clause: before writing a rate, divide the list you have
already printed.** The decision stands for the reason above; the warning is loud in the run's
annotations and the table in §6 is where a reader finds out which runs were which.

What keeps "green" from being read as "compared" is a rule about writing rather than a rule about
CI: **a cross-CPU claim in this repository names the runs it rests on.** The table above gives
three run ids and every row they produced; the 2026-09-07 and PR 13 amendments gave the date and
the pull request. A claim with no run behind it is the thing to refuse, not a green badge.

~~One improvement the evidence does justify, and this pull request does not make: the job counts
**model strings**, and the 9V74 pair proves a model string is not a feature set, so the count can
under-report its own sample. Counting the model-and-flags line instead is one line of shell. It is
not made here because this is a `proto/` pull request and a workflow change is outside the engine
gate's exclusions, so it would rebuild the engine for a counter; it is a row in `docs/plan.md`'s
deferred ledger, triggered by the next pull request that touches `checks.yml`.~~ **Made
2026-10-01 in M4 PR 4, which was that pull request; the ledger row is closed and §6's table carries
the flags from `36834898970` on.** The paragraph is struck rather than deleted because the thing
worth keeping is that it was written in the present tense and stayed there: §6 of this same ADR
recorded the closure on 2026-10-01 and this section, one heading away, went on describing an open
row until M4's close read both halves together. The job now counts `model [flags]`
(`.github/workflows/checks.yml`, the `cpu` step's `GITHUB_OUTPUT` line and `cross-cpu`'s
`CPU model-and-SIMD lines seen`).

The retreat of this decision is still not taken, because nothing failed.

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
