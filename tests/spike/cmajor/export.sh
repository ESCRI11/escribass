#!/usr/bin/env bash
# m5.1-spike measurements 1-3 (docs/plan.md, "What the spike found, measurements 1-3"). Not run by
# CI; kept so a number can be re-taken. SPIKE_ROOT is a scratch directory outside the repository:
#   setup_cmaj.sh $SPIKE_ROOT/tree   Cmajor 024a208 + sparse LLVM, built; then clone clap 195b42a,
#                                    clap-wrapper 1cca996 and vst3sdk 9fad9770 (4 submodules) beside it
#   export.sh <patch> <outdir>       one export; PATCH_HELPER=1 applies the two helper edits, and
#                                    EXTRA_FLAGS / EXTRA_LDFLAGS carry the visibility and nodelete flags
#   clapinfo.c, vst3info.cpp         what an export declares, read without JUCE
#   twohost/                         two bundles in one process, through the engine's own JUCE
#   m1_render.py, m1_matrix.py       one song through core and the engine against each export
#   m3_graphs.py, m3_convert.sh, m3_check.py, m3_engine.py, preflight.py   measurement 3
#
# One Cmajor patch -> .clap and wrapped .vst3, through the approved pins, no downloads.
#   export.sh <patch.cmajorpatch> <outdir>
set -euo pipefail
SP=${SPIKE_ROOT:?set SPIKE_ROOT to the scratch directory}
T=$SP/tree; CM=$T/cmajor/build/tools/command/cmaj
PATCH=$1; OUT=$2; mkdir "$OUT"  # a fresh directory per export; an existing one is an error
s0=$(date +%s.%N)
"$CM" generate --target=clap --output="$OUT/src" --clapIncludePath="$T/clap/include" --clapWrapperPath="$T/clap-wrapper" "$PATCH" > "$OUT/generate.log" 2>&1
s1=$(date +%s.%N)
python3 - "$OUT/src/CMakeLists.txt" <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
s = s.replace("set(CLAP_WRAPPER_DOWNLOAD_DEPENDENCIES TRUE)", "set(CLAP_WRAPPER_DOWNLOAD_DEPENDENCIES FALSE)")
s = s.replace("set(CLAP_WRAPPER_BUILD_AUV2 TRUE)", "set(CLAP_WRAPPER_BUILD_AUV2 FALSE)")
cut = s.index("if (CLAP_WRAPPER_PATH)\n    if (APPLE)")
s = s[:cut] + """if (CLAP_WRAPPER_PATH)
    # m5.1-spike: the generated project builds a VST3 only on Apple; this is the Linux tail.
    set(VST3_TARGET ${CMAJ_TARGET_NAME}_vst3)
    add_library(${VST3_TARGET} MODULE)
    target_link_libraries(${VST3_TARGET} PRIVATE cmaj_clap)
    target_sources(${VST3_TARGET} PRIVATE entry.cpp)
    target_add_vst3_wrapper(TARGET ${VST3_TARGET} OUTPUT_NAME ${CMAJ_CMAKE_PROJECT_NAME})
    target_compile_definitions(${VST3_TARGET} PRIVATE CMAJOR_DLL=1)
endif()
"""
open(p, "w").write(s)
PY
if [ "${PATCH_HELPER:-0}" = 1 ]; then
python3 - "$OUT/src/helpers/clap/cmaj_CLAPPlugin.h" <<'PY'
# m5.1-spike: two edits to the export's own copy of the ISC-licensed CLAP helper, measured, not adopted.
import sys
p = sys.argv[1]; s = open(p).read()
a = """            updateParameterInfoCachesFromLoadedPatch();
            updateAudioPortInfoCachesFromLoadedPatch();
        }"""
assert s.count(a) == 1
s = s.replace(a, """            updateParameterInfoCachesFromLoadedPatch();
            updateAudioPortInfoCachesFromLoadedPatch();
            updateNotePortInfoCachesFromLoadedPatch();
        }""")
b = """    notifyHostAboutLatestChanges();

    return true;"""
assert s.count(b) == 1
s = s.replace(b, """    if (environment.engineType != Environment::EngineType::AOT)
        notifyHostAboutLatestChanges();

    return true;""")
open(p, "w").write(s)
PY
fi
F="-march=x86-64 -mtune=generic -ffp-contract=off ${EXTRA_FLAGS:-}"
cmake -S "$OUT/src" -B "$OUT/build" -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  -DCMAKE_C_FLAGS="$F" -DCMAKE_CXX_FLAGS="$F" -DCLAP_SDK_ROOT="$T/clap" -DVST3_SDK_ROOT="$T/vst3sdk" \
  -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DCMAKE_MODULE_LINKER_FLAGS="${EXTRA_LDFLAGS:-}" > "$OUT/configure.log" 2>&1
s2=$(date +%s.%N)
cmake --build "$OUT/build" -j 8 > "$OUT/build.log" 2>&1
s3=$(date +%s.%N)
printf 'generate %.2f s, configure %.2f s, build %.2f s\n' "$(echo "$s1-$s0"|bc)" "$(echo "$s2-$s1"|bc)" "$(echo "$s3-$s2"|bc)"
find "$OUT/build" -maxdepth 2 \( -name '*.clap' -o -name '*.vst3' \) | sort
