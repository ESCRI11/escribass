# ADR 0031 — The native export recipe uses the approved pins without downloads

- **Status:** Accepted (2026-11-17)
- **Affects:** `compilers/dsp/` — `recipe.py`, `test_recipe.py` and four submodules under
  `vendor/`; `.gitmodules`; `lock.baseline.json` (`dsp`); `engine/cmake/check_flags.cmake` (an
  empty allowlist); `tests/dsp.rs`, `tests/fixtures/dsp/`, `tests/baseline.rs` and the path-gate
  test in `tests/determinism.rs`; `.github/workflows/checks.yml` (the `dsp` job); ADR 0009 §3
  and §4, ADR 0023 §2 and §4, ADR 0028 §2 and ADR 0030 §1, amended in place
- **Builds on:** ADR 0009 §3 (each hazard a build requirement) and §5 (the image and the compiler
  are pins); ADR 0010 §1 (a pin is a commit); ADR 0023 §2 (export and load, no JIT) and §4 (the
  approved native set); ADR 0028 §2 (an export record carries its recipe); ADR 0029 §4 (no
  signal trusted alone) and §6 (the symbol flags); ADR 0030 §1 and §4 (the recipe is pinned on
  reference and compared at compile); the user's decisions 5, 6 and 9 (`docs/plan.md`, "What U2,
  U3 and U4 resolve to" and "What the spike found (PR 1, run 2026-10-20)")
- **Recorded in:** `docs/specs.md` §7.2, §15 and §17.

## Context

ADR 0023 §4 approved the native pin set on the M4 spike's measurements, and the M5 spike found
what that set still could not do: **no Cmajor export played in this engine as `cmaj generate`
leaves it**. Its generated CMake is an Apple recipe on Linux, its download flag is forced on,
the CLAP helper it copies into every export deadlocks JUCE's headless host in `prepareToPlay`
and leaves every instrument without an event bus, and a second export of one processor runs the
first one's code. The spike fixed each by hand, in a scratch tree, with nothing pinned.

This ADR is the recipe those fixes become: which trees, fetched how, `cmaj` built how, the
generated project edited how, compiled with which flags, checked for what, and recorded as what.
It lands with the vendoring it describes, in M5 PR 5. What it does not decide: the driver that
serves `Export.Compile` (PR 7), the engine's inspection and loader (PR 6), and anything `core`
does with an export (PR 8).

**The licence reading this rests on is the user's, decision 6 of 2026-10-22, and §12 already
says it**: a distributed exported plugin is a GPLv3 combined work, because the helpers an export
compiles beside the user's generated C++ are GPLv3 or commercial, except `cmaj_CLAPPlugin.h`,
which is ISC. The two edits in decision 5 below are to that ISC file, in each export's own copy.
The exports this pull request makes are test fixtures nobody distributes.

## Decisions

### 1. Four submodules, and the closure each carries

| Submodule (`compilers/dsp/vendor/`) | Commit | What reaches what |
|---|---|---|
| `cmajor` | `024a208515f15e43271d9b2ea85ee22a2233384b` | Built as the `cmaj` tool only. Its 32 submodules come with it: LLVM (decision 2), `include/choc` at `a08bfd8…`, 28 Boost libraries, and the tool's internal CLAP **1.2.0** and clap-wrapper fork. Boost and LLVM reach `cmaj` and no export. **choc reaches every export**: `cmaj generate` copies the include tree it was built from beside the generated C++ |
| `clap` | `195b42a004144fab0b3cf95e9c067187d15365b7` (1.2.10) | The one CLAP SDK an export compiles against |
| `clap-wrapper` | `1cca996e96f29ab2be7ae9f8cfe532bbc92e1dd6` (v0.16.0) | The CLAP→VST3 wrapper, compiled into every export. No submodules |
| `vst3sdk` | `9fad9770f2ae8542ab1a548a68c1ad1ac690abe0` (`v3.8.0_build_66`) | What the wrapper wraps *to*. `base`, `public.sdk`, `pluginterfaces` and `cmake`, at the commits the tag records; `doc`, `tutorials` and `vstgui4` are never fetched |

`lock.baseline.json` gains what CLAUDE.md says it owed: **`dsp.cmajor.llvm`**, the sub-pin with
its sparse selection; **`dsp.cmajor.choc`**, because it enters an export; **`dsp.clap_sdk.commit`**,
which was `null`; and **`dsp.vst3_sdk`**, the row §17 called new and the baseline never had,
with its four submodule commits. Cmajor's other 30 submodules are pinned by its commit, as JUCE
is pinned by Tracktion's, and are checked rather than restated: the recipe refuses a checkout in
which `git submodule status` shows any of them drifted, missing or conflicted. Cmajor's internal
CLAP 1.2.0 and wrapper fork stay in its tree and are **not** export pins — the recipe refuses an
export whose compile lines reach either.

`tests/baseline.rs` reads the four gitlinks out of this repository's index on every run of the
cheap `checks` job, in both directions: each gitlink against its row, and every submodule in
`.gitmodules` against some row. What sits inside a submodule — a nested commit, a version in a
header — is read by `recipe.py` from the checkout in the `dsp` job, never copied from the
baseline it is compared with.

### 2. The checkout is HTTPS, shallow and sparse, and it is the only step with a network

`recipe.py checkout` is the whole of the fetching:

- the four submodules at depth 1, at their gitlinks;
- Cmajor's submodules at depth 1, each **redirected to HTTPS in that checkout's own git config**,
  because Cmajor declares all 32 with SSH URLs no keyless runner can clone. `.gitmodules` is
  upstream source and is not edited;
- **LLVM as one commit and one directory**: an empty repository, a sparse selection of
  `release/linux/x64`, and a blobless fetch of exactly the commit the parent records. That is
  upstream's prebuilt static libraries — 361 MiB in 2,273 files — linked into `cmaj` and never
  rebuilt, so the engine's flags reach none of it. Its standing is a vendor binary's, said in
  its row;
- the VST3 SDK's four submodules.

**Nothing after it reaches the network, and that is tested rather than asserted.** `recipe.py
build` builds `cmaj` in a user and network namespace with no route out (`unshare -rn`), in CI
as on a developer's machine. `test_recipe.py` configures a generated project in the same kind of
namespace, after showing that the namespace cannot connect to anything. And with the network
**up**, a configure whose VST3 SDK root is an empty directory fails at clap-wrapper's own signal
file and fetches nothing — the "fails instead of fetching" half, which an offline success alone
cannot show.

**CPM.cmake (decision 5) is vendored with its first consumer, and the export recipe is not it.**
clap-wrapper includes CPM only to download an SDK or a macOS filesystem shim; with every
`*_DOWNLOAD_DEPENDENCIES` off and every SDK root given, it never does on Linux, and the recipe
refuses a configure log that says it did. airwin2rack's include is unconditional, so the
vendored CPM 0.40.2 lands in M5 PR 13 with it, at the hash decision 5 recorded. Vendoring a file
here that nothing reads would be a pin with no reader, which `tests/baseline.rs` exists to refuse.

### 3. `cmaj` is built unmodified, with its version told

`recipe.py build` configures Cmajor's own CMake with Ninja, `Release`, `BUILD_CMAJ=ON` and
`BUILD_CMAJ_LIB`, `BUILD_PLUGIN` and `BUILD_EXAMPLES` off, with `gcc-13`/`g++-13` named rather
than whatever `c++` is on the path, and with the engine's `-march=x86-64 -mtune=generic
-ffp-contract=off` on every line. Two things are told rather than found:

- **`CMAJ_VERSION=1.0.3177`.** Cmajor's CMake otherwise runs `git describe --tags`, and a tag
  is the one thing about a checkout that can move under an unchanged commit. `cmaj --version`
  must then report exactly that, read from its `Cmajor Version:` line — the next line is a build
  date, which is not a version.
- **`WEBKIT2_GTK_VERSION=webkit2gtk-4.1`.** The CMake's own rule is "4.1 if found, else 4.0",
  which lets a machine without 4.1 build a different binary under an unchanged recipe.

"Unmodified" is checked: `git status --porcelain --ignore-submodules=none` must be empty in all
four trees and every nested one. The only local difference from upstream is git configuration,
which is not a working-tree change.

### 4. The generated project gets three anchored edits, and its SDKs at configure

`cmaj generate --target=clap` writes a project that builds a `.clap` on Linux and its VST3 only
inside `if (APPLE)`, with clap-wrapper's downloads forced on. The recipe makes **three** edits
to the generated `CMakeLists.txt`, and no others: downloads off, AUv2 off, and the six VST3
lines moved out of the Apple branch, leaving the rest of that branch as upstream wrote it. Each
edit is an anchor and a replacement, and **an anchor that occurs zero times or twice stops the
recipe**, naming the edit — upstream's template having moved is a review, not a guess. The
anchors are data in `recipe.py`, and `test_recipe.py` makes each one miss and each one double
against a freshly generated template.

**No SDK path is written into the generated tree.** `--clapIncludePath` and `--clapWrapperPath`
are not passed to `cmaj generate`, which would write this machine's absolute paths into its
`CMakeLists.txt`. Every root is given at configure instead: `CLAP_INCLUDE_PATH` and
`CLAP_SDK_ROOT` at the vendored CLAP, `CLAP_WRAPPER_PATH`, `VST3_SDK_ROOT`, and
`CLAP_WRAPPER_DOWNLOAD_DEPENDENCIES=OFF`. None is searched for: clap-wrapper's own fallback
looks in `libs/`, `./` and `../`. **`CMAKE_POSITION_INDEPENDENT_CODE=ON`**, because clap-wrapper's
static libraries are not `-fPIC` and the VST3 link fails without it (ADR 0023 §4). Only the
VST3 target is built; the `.clap` beside it would compile the generated C++ a second time for a
file nothing loads.

### 5. Decision 9: two edits to each export's own copy of `cmaj_CLAPPlugin.h`

Approved by the user on 2026-10-22. `cmaj generate` copies the ISC-licensed helper into every
export's `helpers/clap/`, and it is that copy the recipe edits; the vendored tree is not touched.
Each edit is anchored exactly as decision 4's are:

1. **The AOT constructor fills the note-port cache**, beside the parameter and audio-port caches
   it already fills. Without it a generated plugin declares no note port until a host activates
   it, a VST3 host reads its buses before then, and every wrapped instrument is deaf (trap 25).
2. **An AOT plugin's `activate` does not notify the host.** Upstream notifies four changes
   unconditionally — its own comment says so — clap-wrapper turns that into a synchronous
   `restartComponent`, and JUCE 8.0.13's headless host spins in `prepareToPlay` on a lock its
   caller holds, for ever (trap 23). For a generated plugin nothing can change between
   construction and activation; a JIT-loaded patch still notifies.

### 6. What the compiler is told, and what the binary is checked for

Every export compiles with **`-march=x86-64 -mtune=generic -ffp-contract=off`**, the engine's
pin (ADR 0009 §3), and **`-fvisibility=hidden -fvisibility-inlines-hidden -fno-gnu-unique`**,
and links with **`-Wl,-z,nodelete`** (ADR 0029 §6), under CMake's `Release` on `g++-13`. The
recipe then checks the build rather than its own command line:

- **every flag present on every compile line**, read from `compile_commands.json` — the half
  `engine/cmake/check_flags.cmake` never asked;
- **every forbidden flag absent**, by that same guard, run with `-DALLOWED_ISA=` so that **no**
  ISA exception applies: sfizz's `-mavx` and abseil's `-maes` are reasons about those sources,
  not permission for an export or for `cmaj`. The same holds for `cmaj`'s 19 compile lines;
- **one CLAP SDK**: no compile line reaches Cmajor's internal `3rdParty/`;
- **in the binary, no `STB_GNU_UNIQUE` symbol and `FLAGS_1` carrying `NODELETE`**, read with
  `readelf`. Measured: 93 exported symbols — the three VST3 entry points, `clap_entry` and
  libstdc++'s instantiations — where the spike's unflagged build exported 3,168 naming the
  generated class alone.

**Cmajor's own optimisation level is not an input of an export, and the recipe passes none.**
`-O` is read by Cmajor's LLVM back end and its `cpp` performer, and by nothing `--target=clap`
writes: the default, `-O0` and `-O4` generate **byte-identical trees**, all 145 files, and
`test_recipe.py` re-measures that on every run, so a Cmajor re-pin that starts reading it fails
there. What optimises an export is the host compiler's `Release` `-O3 -DNDEBUG`, which the
recipe names. This corrects ADR 0023 §2's "the optimisation level is a pin", which was measured
through the JIT and the `cpp` engine and not through an export.

**Build parallelism is a bound, `EXPORT_JOBS = 4`, for every export and for `cmaj` too**,
chosen against the runner's memory as ADR 0029 §4 asks; it decides how long a build takes and
nothing about its bytes. Measured on the first CI run: an export at four jobs peaked at
**0.7–1.1 GiB in all** and took 26–28 s, while `cmaj` at ninja's default parallelism peaked at
**7.5 GiB** in 184 s — which is why `cmaj`'s build is now held to the same four. Held there, on
the second run, it peaked at **5.0 GiB** in 252 s, of a runner's 4 cores and 15.6 GiB.

### 7. Actual-build provenance: what a recipe is, read from what ran

`recipe.py build` writes `compilers/dsp/build/toolchain.json`, and every export copies it beside
its bundle as `recipe.json`, with the compiler facts re-read at export. Every value is read
from a checkout or from a tool, never from `lock.baseline.json`, which it is then compared with:

| Field | Read from |
|---|---|
| `cmajor`, `llvm`, `choc`, `clap_sdk`, `clap_wrapper`, `vst3_sdk` and the SDK's four submodules | `git rev-parse HEAD` in each checkout |
| `cmajor_version` | `cmaj --version`'s `Cmajor Version:` line |
| `cc`, `cxx` | `gcc-13 -dumpfullversion`, `g++-13 -dumpfullversion` |
| `linker` | the `GNU ld` line the compiler driver's linker prints |
| `build_type`, `cxx_flags`, `link_flags`, `webkit` | the recipe's own constants, which the checks in decision 6 hold the build to |
| `edits` | the SHA-256 of decision 4's and 5's anchors and replacements, as data — the one number that changes exactly when an edit does |

These are **`Recipe`'s fields**, which ADR 0029 §2 left to this ADR. They reach
`proto/export.proto` with their first producer, the driver in M5 PR 7, additively; until then
the message stays empty and nothing hashes it (ADR 0028 §2). An export refuses to run against a
`cmaj` whose `toolchain.json` names other commits than the checkout now holds.

### 8. The build and runtime closure, for this Linux milestone (U5)

Building `cmaj` needs **`libjack-jackd2-dev`** — its CMake makes JACK a hard requirement it then
never links — and **GTK3 and WebKit2GTK 4.1** headers, for the tool and for the helpers every
export links. An export's binary has **twelve `DT_NEEDED` entries and a closure of 129 shared
objects** (`ldd`), measured again here and the same as the spike's: the desktop stack, in the
audio path, never initialised by a headless render. That none of the 129 is on the render path
remains the **assumption** ADR 0023 §4 named; two matching renders here are evidence for the
path exercised and not a proof about the rest. The `dsp` job installs the closure by name; no
package is new to the repository's images except `libgtk-3-dev` and `libwebkit2gtk-4.1-dev`,
which the `app` job already installs.

### 9. Proved by rendering, not by scanning

A scan instantiates a plugin and never prepares it, so it passes over the deadlock and the
missing bus alike (trap 23). `tests/dsp.rs`, behind a `dsp` feature, exports three fixtures
through the recipe and **renders every one through the real engine**, each under a two-minute
deadline that kills the engine's whole process group:

- an instrument playing A4: peak 0.5 at the default Level, 352 cycles in 0.8 s, the same PCM
  from two fresh engine processes, and peak 0.25 with its Level `ParamID` set to a quarter;
- two effects with **one processor name**, one passing the left channel at half and the other
  the right at a quarter, in one engine process, in both load orders — each must run its own
  code (trap 24).

Each check was watched failing first, with the recipe weakened one piece at a time; the record is
in M5 PR 5's description and `docs/plan.md`. One result there is sharper than the spike's: with
the symbol flags off and the recipe's own `GNU_UNIQUE` check removed, the second export's module
**reports the first one's class** — the engine refused it as "declares no class called 'Recipe
Two B'" before a sample was rendered. The collision is in the factory, which is what a VST3 host
asks first, and not only in the processing.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Patch the vendored Cmajor tree, or clap-wrapper, instead of each export's copy of the helper | Moves an approved pin, or leaves instruments deaf; decision 9 chose the copy |
| Edit Cmajor's `.gitmodules` to HTTPS | An upstream source edit; local git config does the same and leaves the tree byte-identical |
| Check out all of `cmajor-lang/llvm` | Every platform's prebuilt libraries, for one platform's build |
| Keep clap-wrapper's downloads on with a pre-seeded CPM cache | A hash makes a download safe and not reproducible (decision 5), and the SDK it would fetch is CLAP 1.2.6, approved nowhere |
| Pass the SDK paths to `cmaj generate` | Writes this machine's absolute paths into every generated tree, which then differs between two machines for no reason in the source |
| Pin Cmajor's `-O` anyway | Measured to change nothing an export is built from; a field that cannot be checked is a field that lies |
| `sed` the template | A pattern that matches nothing "succeeds"; an anchor must match exactly once |
| Rely on `check_flags.cmake`'s existing allowlist for exports | The exceptions are reasons about sfizz and abseil |
| Prove the export with `--scan` | Measured: the scan passes a plugin whose render deadlocks |

## Consequences

- `compilers/dsp/` exists, with the approved trees and the recipe; `lock.baseline.json`'s `dsp`
  block is complete for the export recipe, and **its ccache key moves, so the engine job's next
  build is cold**. `bundled_plugins.airwindows` and the `onnx` rows are untouched: they are PRs
  13 and 12's.
- The `dsp` CI job runs after `engine`, on its uploaded binary: checkout, `cmaj` built offline,
  `test_recipe.py`, and `tests/dsp.rs` offline. The engine job's path gate now has positive cases
  for `compilers/dsp/` and the fixtures, `compilers/generative/` stays excluded, and the `dsp`
  job inherits the engine's decision.
- **Measured on the runner** (`ubuntu-24.04`, the first run of the `dsp` job): the checkout
  32 s cold; `cmaj` 184 s and 7.5 GiB at its peak at ninja's default parallelism; each export
  26–28 s, of which the build is 26–27 s, and 0.7–1.1 GiB at its peak; `test_recipe.py` 3 s; the
  render test 85 s for three exports and six renders. The job's apt step took 18 minutes at
  about 70 kB/s, a mirror's cost rather than the recipe's, and has its own thirty-minute bound.
  On the second run, with `cmaj` held to four jobs: 252 s and 5.0 GiB, exports 40 s and
  0.7–0.9 GiB on a slower draw, the apt step 36 s and the whole job under nine minutes.
- ADRs amended in place, dated: **0009** §3 (the ISA pin reaches exports and `cmaj`, with no
  exception) and §4 (a sixth surface); **0023** §2 (`-O` is not an export input) and §4 (vendored,
  and how); **0028** §2 and **0030** §1 (a recipe holds no Cmajor optimisation level).
- **Not decided here, and not measured:** two independent builds of one export compared across
  machines, which PR 7 makes a standing check; the engine's inspection and a loader keyed by
  export, PR 6; and whether any of the 129 shared objects is ever on a render path.
