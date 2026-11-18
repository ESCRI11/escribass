#!/usr/bin/env python3
"""The native export recipe (ADR 0031): the approved trees checked out, `cmaj` built unmodified,
and one Cmajor patch exported to a Linux VST3 the engine can load.

    recipe.py checkout              the approved trees, over HTTPS, LLVM sparse (needs the network)
    recipe.py build                 `cmaj`, its flags checked and its provenance written (needs none)
    recipe.py export <patch> <out>  one .cmajorpatch -> <out>/bundle/<name>.vst3 (needs none)

Standard library only, run as `python3 -I`: PR 7's driver is what turns this into a compiler a
session spawns (ADR 0029 §1), and it reuses these steps rather than restating them. Every step
reads its tool's status, its diagnostics and the file it was meant to make, and trusts none of
the three alone (ADR 0029 §4; docs/plan.md, M5 trap 1).
"""

import argparse
import hashlib
import json
import os
import re
import shlex
import subprocess
import sys
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
VENDOR = HERE / "vendor"
CMAJOR = VENDOR / "cmajor"
CLAP = VENDOR / "clap"
WRAPPER = VENDOR / "clap-wrapper"
VST3 = VENDOR / "vst3sdk"
BUILD = HERE / "build"
CMAJ_BUILD = BUILD / "cmajor"
CMAJ = CMAJ_BUILD / "tools" / "command" / "cmaj"
TOOLCHAIN = BUILD / "toolchain.json"

# **The version is a pin, not a derivation** (ADR 0031 §3). Cmajor's CMake runs `git describe
# --tags` when it is not told, and a tag is the one thing about a checkout that can move under an
# unchanged commit. `tests/baseline.rs` reads this line against `dsp.cmajor.version`.
CMAJ_VERSION = "1.0.3177"

# The engine's compile-time ISA and contraction pin (ADR 0009 §3), on `cmaj` and on every export,
# and the three symbol flags plus the link flag without which a second export of one processor
# runs the first one's code, or crashes the engine in `exit()` (ADR 0029 §6).
ISA_FLAGS = ["-march=x86-64", "-mtune=generic", "-ffp-contract=off"]
SYMBOL_FLAGS = ["-fvisibility=hidden", "-fvisibility-inlines-hidden", "-fno-gnu-unique"]
LINK_FLAGS = ["-Wl,-z,nodelete"]
BUILD_TYPE = "Release"

# **The optimisation is the host compiler's, and only its** (ADR 0031 §5). Cmajor's own `-O` is
# read by its LLVM back end and its `cpp` performer and by nothing `--target=clap` writes:
# default, `-O0` and `-O4` generate byte-identical trees, which `test_recipe.py` re-measures on
# every run. So the recipe passes none, and what optimises an export is CMake's Release `-O3`.

# The compilers, by name rather than whatever `c++` is on the path: §17's g++ 13.3.
CC, CXX = "gcc-13", "g++-13"
# The webview library the helpers link, by name rather than "4.1 if found, else 4.0" — the
# generated CMake's own fallback, which would let a machine without 4.1 build a different
# binary under an unchanged recipe (U5's measured closure is 4.1).
WEBKIT = "webkit2gtk-4.1"

# Build parallelism, for `cmaj` and for every export, bounded by what the CI runner's memory
# measured (ADR 0029 §4; ADR 0031 §6): `cmaj` at ninja's default parallelism peaked at 7.5 GiB
# in all on the runner, and an export at four jobs at 1.1 GiB. It decides how long a build
# takes and nothing about its bytes.
EXPORT_JOBS = 4

LLVM_SPARSE = "release/linux/x64"
VST3_SUBMODULES = ["base", "cmake", "pluginterfaces", "public.sdk"]


class RecipeError(Exception):
    """A step that did not do what it was for. The message names the step and why."""


def run(*cmd, cwd=None, env=None, log=None, ok=(0,)):
    """Runs one command; its output goes to `log` (or is returned), and a status outside `ok`
    is an error carrying the command's own last lines."""
    shown = " ".join(shlex.quote(str(c)) for c in cmd)
    done = subprocess.run([str(c) for c in cmd], cwd=cwd, env=env, text=True,
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if log is not None:
        Path(log).write_text(done.stdout)
    if done.returncode not in ok:
        tail = "\n".join(done.stdout.splitlines()[-25:])
        raise RecipeError(f"`{shown}` exited {done.returncode}\n{tail}")
    return done.stdout


class Cost:
    """A step's wall time and the **total** memory in use beyond where it started, sampled from
    `/proc/meminfo` — every compiler and linker the step runs at once, which `time -v`'s largest
    single process is not. What `EXPORT_JOBS` was chosen against (ADR 0029 §4; ADR 0031 §6)."""

    def __enter__(self):
        self.started, self.base = time.monotonic(), self._available()
        self.low, self.done = self.base, threading.Event()
        self.thread = threading.Thread(target=self._sample, daemon=True)
        self.thread.start()
        return self

    def __exit__(self, *_):
        self.done.set()
        self.thread.join()
        self.seconds = time.monotonic() - self.started
        self.peak_mib = (self.base - self.low) // 1024

    def __str__(self):
        return f"{self.seconds:.1f} s, peak {self.peak_mib} MiB"

    def _sample(self):
        while not self.done.wait(0.2):
            self.low = min(self.low, self._available())

    @staticmethod
    def _available():
        with open("/proc/meminfo") as info:
            return next(int(l.split()[1]) for l in info if l.startswith("MemAvailable:"))


def git(*args, cwd):
    return run("git", *args, cwd=cwd).strip()


def baseline():
    return json.loads((REPO / "lock.baseline.json").read_text())


def head(tree):
    return git("rev-parse", "HEAD", cwd=tree)


def gitlink(parent, path):
    """The commit `parent` records for its submodule at `path`, read from its index: what the
    approved parent pins, as opposed to whatever happens to be checked out there."""
    out = git("ls-files", "--stage", "--", path, cwd=parent).split()
    if len(out) < 2 or out[0] != "160000":
        raise RecipeError(f"{parent} records no submodule at {path}")
    return out[1]


# ---------------------------------------------------------------------------------------------
# checkout
# ---------------------------------------------------------------------------------------------

def checkout():
    """Fetches the approved trees and nothing else (ADR 0031 §2).

    Cmajor declares its 32 submodules with SSH URLs no keyless runner can clone, so each is
    redirected to HTTPS in that checkout's own git config — `.gitmodules` is upstream source and
    is not edited. LLVM is upstream's prebuilt static libraries for every platform; only
    `release/linux/x64` is fetched, blobs and all, and only at the commit the parent records.
    The VST3 SDK's seven submodules are four, the four clap-wrapper's own fetch names."""
    tops = [p.relative_to(REPO) for p in (CMAJOR, CLAP, WRAPPER, VST3)]
    run("git", "submodule", "update", "--init", "--depth", "1", "--", *tops, cwd=REPO)

    modules = CMAJOR / ".gitmodules"
    names = {}
    for line in git("config", "-f", modules, "--get-regexp", r"^submodule\..*\.path$", cwd=CMAJOR).splitlines():
        key, path = line.split(" ", 1)
        names[path] = key[len("submodule."):-len(".path")]
    for path, name in names.items():
        url = git("config", "-f", modules, f"submodule.{name}.url", cwd=CMAJOR)
        https = re.sub(r"^git@github\.com:", "https://github.com/", url)
        if not https.startswith("https://github.com/"):
            raise RecipeError(f"Cmajor's submodule {path} is at {url}, which this recipe does not know how to reach")
        git("config", f"submodule.{name}.url", https, cwd=CMAJOR)
    others = sorted(p for p in names if p != "3rdParty/llvm")
    run("git", "submodule", "update", "--init", "--depth", "1", "--jobs", "8", "--", *others, cwd=CMAJOR)

    llvm = CMAJOR / "3rdParty" / "llvm"
    commit = gitlink(CMAJOR, "3rdParty/llvm")
    if not (llvm / ".git").exists():
        llvm.mkdir(parents=True, exist_ok=True)
        git("init", "-q", cwd=llvm)
        git("remote", "add", "origin", git("config", f"submodule.{names['3rdParty/llvm']}.url", cwd=CMAJOR), cwd=llvm)
        git("sparse-checkout", "set", LLVM_SPARSE, cwd=llvm)
    if head_or_none(llvm) != commit:
        run("git", "fetch", "-q", "--depth", "1", "--filter=blob:none", "origin", commit, cwd=llvm)
        run("git", "checkout", "-q", "--detach", commit, cwd=llvm)

    run("git", "submodule", "update", "--init", "--depth", "1", "--", *VST3_SUBMODULES, cwd=VST3)
    verify_checkout()


def head_or_none(tree):
    done = subprocess.run(["git", "rev-parse", "HEAD"], cwd=tree, capture_output=True, text=True)
    return done.stdout.strip() if done.returncode == 0 else None


def verify_checkout():
    """The checkouts against what their parents record and what `lock.baseline.json` pins, and
    against nothing either of them merely claims: every commit here is read out of a checkout.
    Returns the facts the provenance records."""
    pins = baseline()["dsp"]
    wrong = []

    def expect(what, actual, pinned):
        if actual != pinned:
            wrong.append(f"{what}: checked out {actual}, lock.baseline.json pins {pinned}")

    for name, tree, at in (("cmajor", CMAJOR, "compilers/dsp/vendor/cmajor"),
                           ("clap_sdk", CLAP, "compilers/dsp/vendor/clap"),
                           ("clap_wrapper", WRAPPER, "compilers/dsp/vendor/clap-wrapper"),
                           ("vst3_sdk", VST3, "compilers/dsp/vendor/vst3sdk")):
        expect(name, head(tree), pins[name]["commit"])
        expect(f"{name} (this repository's gitlink)", gitlink(REPO, at), pins[name]["commit"])

    # Every one of Cmajor's other 31 submodules at exactly the commit the approved parent
    # records — the 28 Boost libraries, choc, and the tool's internal CLAP 1.2.0 and wrapper
    # fork, none of which is an export pin (ADR 0031 §1). A `+` is a drifted one, a `-` one that
    # was never fetched, a `U` a conflict.
    for line in run("git", "submodule", "status", cwd=CMAJOR).splitlines():
        state, commit, path = line[0], *line[1:].split()[:2]
        if path == "3rdParty/llvm":
            continue
        if state != " ":
            wrong.append(f"cmajor/{path}: `git submodule status` says {line.strip()!r}")
    llvm = CMAJOR / "3rdParty" / "llvm"
    expect("cmajor/3rdParty/llvm", head_or_none(llvm), gitlink(CMAJOR, "3rdParty/llvm"))
    expect("cmajor.llvm", head_or_none(llvm), pins["cmajor"]["llvm"]["commit"])
    sparse = git("sparse-checkout", "list", cwd=llvm) if llvm.exists() else ""
    expect("cmajor.llvm sparse selection", sparse, pins["cmajor"]["llvm"]["sparse"])
    expect("cmajor.choc", head_or_none(CMAJOR / "include" / "choc"), pins["cmajor"]["choc"]["commit"])
    for sub in VST3_SUBMODULES:
        key = sub.replace(".", "_")
        expect(f"vst3_sdk/{sub}", head_or_none(VST3 / sub), pins["vst3_sdk"][key])
        expect(f"vst3_sdk/{sub} (the SDK's gitlink)", gitlink(VST3, sub), pins["vst3_sdk"][key])

    # **Unmodified** (ADR 0031 §3): no edit anywhere in the four trees or Cmajor's submodules.
    # Only local git config differs from upstream, and config is not a working-tree change.
    for tree in (CMAJOR, CLAP, WRAPPER, VST3):
        dirty = git("status", "--porcelain", "--ignore-submodules=none", cwd=tree)
        if dirty:
            wrong.append(f"{tree.relative_to(REPO)} is modified:\n{dirty}")

    # The versions, read from the sources that state them rather than from a tag.
    clap_h = (CLAP / "include" / "clap" / "version.h").read_text()
    clap_version = ".".join(re.search(rf"#define CLAP_VERSION_{part} (\d+)", clap_h).group(1)
                            for part in ("MAJOR", "MINOR", "REVISION"))
    expect("clap_sdk version (include/clap/version.h)", clap_version, pins["clap_sdk"]["version"])
    wrapper_version = re.search(r"project\(clap-wrapper[^)]*VERSION ([0-9.]+)",
                                (WRAPPER / "CMakeLists.txt").read_text()).group(1)
    expect("clap_wrapper version (CMakeLists.txt)", wrapper_version, pins["clap_wrapper"]["version"])
    vst3_version = re.search(r'#define kVstVersionString\s+"VST ([0-9.]+)"',
                             (VST3 / "pluginterfaces" / "vst" / "vsttypes.h").read_text()).group(1)
    expect("vst3_sdk version (pluginterfaces/vst/vsttypes.h)", vst3_version, pins["vst3_sdk"]["version"])
    expect("cmajor version (recipe.py's CMAJ_VERSION)", CMAJ_VERSION, pins["cmajor"]["version"])

    if wrong:
        raise RecipeError("the checkout is not the approved closure:\n  " + "\n  ".join(wrong))
    return {
        "cmajor": head(CMAJOR), "llvm": head(llvm), "choc": head(CMAJOR / "include" / "choc"),
        "clap_sdk": head(CLAP), "clap_wrapper": head(WRAPPER), "vst3_sdk": head(VST3),
        **{f"vst3_sdk_{sub.replace('.', '_')}": head(VST3 / sub) for sub in VST3_SUBMODULES},
    }


# ---------------------------------------------------------------------------------------------
# build: `cmaj`, unmodified
# ---------------------------------------------------------------------------------------------

def compiler_flags(flags):
    return " ".join(flags)


def check_flags(compile_commands, required):
    """Trap 5: the flags are read off every compile line, present as well as absent.

    Absent is `engine/cmake/check_flags.cmake`, the engine's own guard, run here **with no ISA
    exceptions**: sfizz's `-mavx` and abseil's `-maes` are reasons about those sources, and
    nothing in this tree has one. Present is every flag in `required`, on every line."""
    commands = json.loads(Path(compile_commands).read_text())
    if not commands:
        raise RecipeError(f"{compile_commands} holds no compile command")
    run("cmake", "-DALLOWED_ISA=", f"-DCOMPILE_COMMANDS={compile_commands}",
        "-P", REPO / "engine" / "cmake" / "check_flags.cmake")
    missing = sorted({(flag, entry["file"]) for entry in commands
                      for flag in required if flag not in shlex.split(entry["command"])})
    if missing:
        shown = "\n  ".join(f"{flag} is not on the line compiling {file}" for flag, file in missing[:20])
        raise RecipeError(f"{compile_commands}: {len(missing)} required flag(s) missing\n  {shown}")
    return len(commands)


def build(jobs=EXPORT_JOBS):
    """`cmaj` from the checkout as it stands, and its provenance (ADR 0031 §3, §7).

    Reaches no network: run it under `unshare -rn` and it builds the same. Nothing upstream is
    edited, so the one thing this step decides is how `cmaj` is configured — which is all here."""
    facts = verify_checkout()
    with Cost() as cost:
        run("cmake", "-S", CMAJOR, "-B", CMAJ_BUILD, "-G", "Ninja",
            f"-DCMAKE_C_COMPILER={CC}", f"-DCMAKE_CXX_COMPILER={CXX}",
            f"-DCMAKE_BUILD_TYPE={BUILD_TYPE}",
            f"-DCMAKE_C_FLAGS={compiler_flags(ISA_FLAGS)}", f"-DCMAKE_CXX_FLAGS={compiler_flags(ISA_FLAGS)}",
            "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
            "-DBUILD_CMAJ=ON", "-DBUILD_CMAJ_LIB=OFF", "-DBUILD_PLUGIN=OFF", "-DBUILD_EXAMPLES=OFF",
            f"-DCMAJ_VERSION={CMAJ_VERSION}", f"-DWEBKIT2_GTK_VERSION={WEBKIT}",
            log=BUILD / "cmajor-configure.log")
        lines = check_flags(CMAJ_BUILD / "compile_commands.json", ISA_FLAGS)
        run("cmake", "--build", CMAJ_BUILD, "--target", "cmaj", "-j", str(jobs),
            log=BUILD / "cmajor-build.log")

    # The version line, and not the whole of stdout: the next line is a build date, which is not
    # a fact about what was built from (docs/plan.md, the 2026-10-13 JACK check).
    said = run(CMAJ, "--version").splitlines()
    version = next((l.split(":", 1)[1].strip() for l in said if l.startswith("Cmajor Version:")), None)
    if version != CMAJ_VERSION:
        raise RecipeError(f"{CMAJ} says it is {version!r}; the recipe built {CMAJ_VERSION}")

    TOOLCHAIN.write_text(json.dumps({**facts, "cmajor_version": version, **toolchain()},
                                    indent=2, sort_keys=True) + "\n")
    print(f"cmaj {version}: {lines} compile lines, configured and built in {cost}", file=sys.stderr)


def toolchain():
    """What compiles and links, as the tools themselves say it: the actual half of the recipe,
    never a copy of what it was asked for (ADR 0031 §7; ADR 0027 §1's rule)."""
    linker = run(CXX, "-Wl,--version", "-xc++", "/dev/null", "-o", "/dev/null", ok=(0, 1)).splitlines()
    return {
        "cc": f"{CC} {run(CC, '-dumpfullversion').strip()}",
        "cxx": f"{CXX} {run(CXX, '-dumpfullversion').strip()}",
        "linker": next((l for l in linker if l.startswith("GNU ld")), "unknown"),
        "build_type": BUILD_TYPE,
        "cxx_flags": ISA_FLAGS + SYMBOL_FLAGS,
        "link_flags": LINK_FLAGS,
        "webkit": WEBKIT,
        "edits": edits_digest(),
    }


# ---------------------------------------------------------------------------------------------
# export: one patch, generated, adapted, built and inspected
# ---------------------------------------------------------------------------------------------

# **The recipe's own edits to generated code** (ADR 0031 §4, §5; decision 9). Each is an anchor
# that must occur **exactly once** in what `cmaj generate` wrote, and its replacement. An anchor
# found zero times or twice is upstream's template having moved, and the recipe stops rather
# than build what nobody reviewed — a `sed` that matches nothing "fixes" nothing (trap 3).
CMAKE_EDITS = [
    ("downloads off",
     "    set(CLAP_WRAPPER_DOWNLOAD_DEPENDENCIES TRUE)\n",
     "    set(CLAP_WRAPPER_DOWNLOAD_DEPENDENCIES FALSE)  # escribass: every SDK is given (ADR 0031 §4)\n"),
    ("no AUv2",
     "    set(CLAP_WRAPPER_BUILD_AUV2 TRUE)\n",
     "    set(CLAP_WRAPPER_BUILD_AUV2 FALSE)  # escribass: a macOS format (ADR 0031 §4)\n"),
    # The generated project builds its VST3 inside `if (APPLE)`, so on Linux it builds a `.clap`
    # and nothing else. The six lines move out of that branch and nothing else in it does.
    ("a Linux VST3",
     "if (CLAP_WRAPPER_PATH)\n"
     "    if (APPLE)\n"
     "        set(VST3_TARGET ${CMAJ_TARGET_NAME}_vst3)\n"
     "        add_library(${VST3_TARGET} MODULE)\n"
     "        target_link_libraries(${VST3_TARGET} PRIVATE cmaj_clap)\n"
     "        target_sources(${VST3_TARGET} PRIVATE entry.cpp)\n"
     "        target_add_vst3_wrapper(TARGET ${VST3_TARGET} OUTPUT_NAME ${CMAJ_CMAKE_PROJECT_NAME})\n"
     "        target_compile_definitions(${VST3_TARGET} PRIVATE CMAJOR_DLL=1)\n",
     "if (CLAP_WRAPPER_PATH)\n"
     "    # escribass: the VST3 on every platform, not only inside `if (APPLE)` (ADR 0031 §4)\n"
     "    set(VST3_TARGET ${CMAJ_TARGET_NAME}_vst3)\n"
     "    add_library(${VST3_TARGET} MODULE)\n"
     "    target_link_libraries(${VST3_TARGET} PRIVATE cmaj_clap)\n"
     "    target_sources(${VST3_TARGET} PRIVATE entry.cpp)\n"
     "    target_add_vst3_wrapper(TARGET ${VST3_TARGET} OUTPUT_NAME ${CMAJ_CMAKE_PROJECT_NAME})\n"
     "    target_compile_definitions(${VST3_TARGET} PRIVATE CMAJOR_DLL=1)\n"
     "    if (APPLE)\n"),
]

# Decision 9's two edits to the export's own copy of the ISC-licensed CLAP helper, approved by
# the user on 2026-10-22. The vendored tree is not touched; `cmaj generate` copies the helper
# into every export, and it is that copy that changes.
HELPER_EDITS = [
    # Without it a generated plugin declares no note port until a host activates it, and a VST3
    # host reads its buses before then: every wrapped instrument had no event input (trap 25).
    ("the AOT constructor fills the note-port cache",
     "            updateParameterInfoCachesFromLoadedPatch();\n"
     "            updateAudioPortInfoCachesFromLoadedPatch();\n"
     "        }\n",
     "            updateParameterInfoCachesFromLoadedPatch();\n"
     "            updateAudioPortInfoCachesFromLoadedPatch();\n"
     "            updateNotePortInfoCachesFromLoadedPatch();  // escribass: decision 9 (ADR 0031 §5)\n"
     "        }\n"),
    # Without it `activate` tells the host four things changed, clap-wrapper turns that into a
    # synchronous `restartComponent`, and JUCE's headless host spins in `prepareToPlay` for ever
    # (trap 23). Nothing can change between an AOT plugin's construction and its activation.
    ("an AOT plugin's activate does not notify the host",
     "    notifyHostAboutLatestChanges();\n"
     "\n"
     "    return true;\n",
     "    if (environment.engineType != Environment::EngineType::AOT)  // escribass: decision 9 (ADR 0031 §5)\n"
     "        notifyHostAboutLatestChanges();\n"
     "\n"
     "    return true;\n"),
]


def edits_digest():
    """The edits as data, hashed: the recipe's edits are an input of every export, and this is
    the one number that changes exactly when one of them does."""
    return hashlib.sha256(json.dumps([CMAKE_EDITS, HELPER_EDITS]).encode()).hexdigest()


def apply_edits(path, edits):
    text = Path(path).read_text()
    for name, anchor, replacement in edits:
        found = text.count(anchor)
        if found != 1:
            raise RecipeError(f"{path}: the anchor for “{name}” occurs {found} times, not once — "
                              f"upstream's template has moved, and the recipe will not guess (ADR 0031 §4)")
        text = text.replace(anchor, replacement)
    Path(path).write_text(text)


def generate(patch, src):
    """`cmaj generate --target=clap`, judged on all three of its outputs (ADR 0029 §4): it exits
    1 on a compile error and still leaves helper files behind, so a populated directory is not
    a success. No SDK path is passed, so nothing about this machine is written into the tree."""
    done = subprocess.run([str(CMAJ), "generate", "--target=clap", f"--output={src}", str(patch)],
                          capture_output=True, text=True)
    said = done.stdout + done.stderr
    errors = [l for l in said.splitlines() if ": error:" in l or l.startswith("error:")]
    produced = all((Path(src) / f).is_file() for f in ("entry.cpp", "CMakeLists.txt",
                                                       "helpers/clap/cmaj_CLAPPlugin.h"))
    if done.returncode != 0 or errors or not produced:
        raise RecipeError(f"cmaj generate {patch}: exit {done.returncode}, "
                          f"{'all' if produced else 'not all'} expected files written\n{said.strip()}")
    return said


def configure(src, build_dir, vst3_sdk=VST3, clap_sdk=CLAP, wrapper=WRAPPER):
    """The generated project, configured against the vendored SDKs and nothing found on the way
    (ADR 0031 §4). Every root is given explicitly; none is searched for, none is downloaded."""
    flags = compiler_flags(ISA_FLAGS + SYMBOL_FLAGS)
    log = Path(build_dir).parent / "configure.log"
    try:
        run("cmake", "-S", src, "-B", build_dir, "-G", "Ninja",
            f"-DCMAKE_C_COMPILER={CC}", f"-DCMAKE_CXX_COMPILER={CXX}",
            f"-DCMAKE_BUILD_TYPE={BUILD_TYPE}",
            f"-DCMAKE_C_FLAGS={flags}", f"-DCMAKE_CXX_FLAGS={flags}",
            f"-DCMAKE_MODULE_LINKER_FLAGS={compiler_flags(LINK_FLAGS)}",
            # clap-wrapper's static libraries are not -fPIC, and the VST3 link fails without it.
            "-DCMAKE_POSITION_INDEPENDENT_CODE=ON",
            "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
            f"-DCLAP_INCLUDE_PATH={Path(clap_sdk) / 'include'}", f"-DCLAP_SDK_ROOT={clap_sdk}",
            f"-DCLAP_WRAPPER_PATH={wrapper}", f"-DVST3_SDK_ROOT={vst3_sdk}",
            "-DCLAP_WRAPPER_DOWNLOAD_DEPENDENCIES=OFF", f"-DWEBKIT2_GTK_VERSION={WEBKIT}",
            log=log)
    finally:
        said = log.read_text() if log.exists() else ""
    # The bootstrap that would fetch CPM.cmake is never reached, rather than reached and served
    # from a cache: with every root given and downloads off it has no caller on Linux.
    if "including CPM" in said:
        raise RecipeError(f"{log}: clap-wrapper included CPM, which downloads; the recipe gives every SDK")
    return said


def inspect(src, build_dir):
    """What was built, read from the build rather than from the recipe (ADR 0031 §6)."""
    lines = check_flags(Path(build_dir) / "compile_commands.json", ISA_FLAGS + SYMBOL_FLAGS)
    commands = json.loads((Path(build_dir) / "compile_commands.json").read_text())

    # **One CLAP SDK and one wrapper** (ADR 0023 §4): no line reaches Cmajor's internal CLAP
    # 1.2.0 or its wrapper fork, and every CLAP include is the vendored 1.2.10's.
    internal = CMAJOR / "3rdParty"
    wrong = sorted({e["file"] for e in commands if str(internal) in e["command"]})
    if wrong:
        raise RecipeError(f"{len(wrong)} compile line(s) reach {internal}: {wrong[:3]}")

    bundles = sorted(Path(build_dir).glob(f"{BUILD_TYPE}/*.vst3"))
    if len(bundles) != 1:
        raise RecipeError(f"{build_dir}: {len(bundles)} .vst3 bundles where the recipe makes one")
    bundle = bundles[0]
    members = sorted(p.relative_to(bundle).as_posix() for p in bundle.rglob("*") if p.is_file())
    so = bundle / "Contents" / "x86_64-linux" / (bundle.stem + ".so")
    if not so.is_file() or so.stat().st_size == 0:
        raise RecipeError(f"{bundle} has no {so.relative_to(bundle)}: {members}")

    # The symbol flags, as the binary has them rather than as the command line asked: no
    # `STB_GNU_UNIQUE` symbol — the plugin factory is one, and glibc binds it process-wide —
    # and `NODELETE`, without which JUCE's `dlclose` unmaps the module and the engine crashes
    # in `exit()` (ADR 0029 §6).
    unique = [l for l in run("readelf", "--dyn-syms", "-W", so).splitlines() if " UNIQUE " in l]
    if unique:
        raise RecipeError(f"{so} has {len(unique)} GNU_UNIQUE symbol(s), e.g. {unique[0].split()[-1]}")
    flags_1 = [l for l in run("readelf", "-d", "-W", so).splitlines() if "(FLAGS_1)" in l]
    if not any("NODELETE" in l for l in flags_1):
        raise RecipeError(f"{so} is not NODELETE: {flags_1}")
    return {"bundle": str(bundle), "members": members, "compile_lines": lines}


def export(patch, out):
    """One patch to one loadable `.vst3` in a directory of its own (ADR 0031 §4–§6)."""
    out = Path(out)
    out.mkdir(parents=True)  # a fresh directory per export: an existing one is an error, not a reuse
    if not TOOLCHAIN.is_file():
        raise RecipeError(f"no {TOOLCHAIN}: run `recipe.py build` first")
    # The `cmaj` that will run is the one the checkout still describes: a re-pinned or edited
    # tree with an old build beside it is refused, not exported with (docs/plan.md, M5 trap 19).
    built = json.loads(TOOLCHAIN.read_text())
    stale = {k: (built.get(k), v) for k, v in verify_checkout().items() if built.get(k) != v}
    if stale:
        raise RecipeError(f"`cmaj` was built from another checkout ({stale}): run `recipe.py build`")
    src, build_dir = out / "src", out / "build"
    stages = {}

    with Cost() as stages["generate"]:
        generate(Path(patch).resolve(), src)
        apply_edits(src / "CMakeLists.txt", CMAKE_EDITS)
        apply_edits(src / "helpers" / "clap" / "cmaj_CLAPPlugin.h", HELPER_EDITS)
    with Cost() as stages["configure"]:
        configure(src, build_dir)

    target = re.search(r'^set\(CMAJ_TARGET_NAME "([^"]+)"\)$', (src / "CMakeLists.txt").read_text(), re.M)
    if not target:
        raise RecipeError(f"{src / 'CMakeLists.txt'} names no CMAJ_TARGET_NAME")
    with Cost() as stages["build"]:
        run("cmake", "--build", build_dir, "--target", f"{target.group(1)}_vst3", "-j", str(EXPORT_JOBS),
            log=out / "build.log")

    facts = inspect(src, build_dir)
    recipe = {**json.loads(TOOLCHAIN.read_text()), **toolchain()}
    (out / "recipe.json").write_text(json.dumps(recipe, indent=2, sort_keys=True) + "\n")
    print("; ".join(f"{k} {v}" for k, v in stages.items()), file=sys.stderr)
    print(facts["bundle"])
    return facts


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="step", required=True)
    sub.add_parser("checkout")
    built = sub.add_parser("build")
    built.add_argument("--jobs", type=int, default=EXPORT_JOBS)
    exported = sub.add_parser("export")
    exported.add_argument("patch")
    exported.add_argument("out")
    args = parser.parse_args(argv)
    try:
        if args.step == "checkout":
            checkout()
        elif args.step == "build":
            build(args.jobs)
        else:
            export(args.patch, args.out)
    except RecipeError as failed:
        print(f"recipe.py {args.step}: {failed}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
