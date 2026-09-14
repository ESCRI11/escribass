# Reads compile_commands.json and fails on any flag ADR 0009 §3 forbids. Run as a build step
# before the engine compiles, so `cmake --build` itself is what fails.
#
# -Ofast and -funsafe-math-optimizations are -ffast-math by other spellings; -march is
# checked because a second baseline in a vendored tree is the same hazard from the other
# side — code compiled for a wider ISA than the pin.
#
# **`-march=` is not the only spelling of that hazard** (M1 PR 13). A translation unit compiled
# `-mavx2` is code for a wider ISA exactly as `-march=haswell` is, and the header above claimed
# to catch it while the regex looked for one of the two. sfizz carries seven such flags today
# and every one of them is deliberate, so the second check is an allowlist rather than a ban:
# what it exists to catch is Dexed or Surge XT acquiring one, because their CPU independence is
# what makes their goldens portable (docs/specs.md §8), and a bump adding `-mavx2` to either
# would otherwise pass in silence.

file(READ ${COMPILE_COMMANDS} commands)

string(REGEX MATCHALL "-ffast-math|-Ofast|-funsafe-math-optimizations" fast ${commands})
if(fast)
    list(REMOVE_DUPLICATES fast)
    message(FATAL_ERROR "${fast} reached a compile line; ADR 0009 §3 forbids it. Find the target in ${COMPILE_COMMANDS}")
endif()

string(REGEX MATCHALL "-march=[^ \"\\\\]+" arches ${commands})
list(REMOVE_DUPLICATES arches)
list(REMOVE_ITEM arches "-march=x86-64")
if(arches)
    message(FATAL_ERROR "${arches} reached a compile line beside the pinned -march=x86-64; ADR 0009 §3 pins one ISA. Find the target in ${COMPILE_COMMANDS}")
endif()

# Named one by one rather than as "-m<anything>": a path segment, `-Wno-multichar` and
# `-Wno-class-memaccess` all match a loose pattern, and a check that cries wolf is a check
# someone widens until it says nothing.
set(isa_flags
    "avx" "avx2" "avx512f" "avx512bw" "avx512dq" "avx512vl"
    "sse" "sse2" "sse3" "sse4" "sse4.1" "sse4.2" "ssse3"
    "aes" "fma" "fma4" "f16c" "bmi" "bmi2" "popcnt" "pclmul" "abm" "xop" "sha" "vaes" "gfni")

# The documented exceptions, each with the reason it is allowed. Every one is on a translation
# unit that dispatches at run time, so a wide path is compiled and then chosen — or not — by
# `cpuid`. ADR 0009 §6's experiment is the claim that rests on this, and it is why the exception
# is written down beside the check rather than assumed from the absence of a failure.
#
#   -mavx     ResonantArrayAVX / ResonantStringAVX / HelpersAVX — the wide implementations
#             sfizz's `SIMDHelpers.cpp` and `effects/Strings.cpp` select between at run time.
#   -msse4.1  abseil's randen_detect.cc and randen_hwaes.cc, its hardware-AES PRNG, which is
#   -maes     dispatched the same way and is not on a render path at all.
#   -msse2    the x86-64 baseline `-march=x86-64` already implies; a restatement, not a widening.
#
# **The randen pair now reaches this check twice, and needed no new entry** (M2 PR 9). It was
# sfizz's vendored abseil, in a plugin's own compile database; grpc brings abseil of the same LTS
# release into the *engine's* tree, and those two files are the only thing in 2281 gRPC, abseil,
# BoringSSL, re2, c-ares and zlib translation units that carries an ISA flag at all — measured,
# not predicted, and the reason ADR 0013 §1 could say the guard would not fire.
set(allowed "-mavx" "-msse2" "-msse4.1" "-maes")

set(found "")
foreach(flag IN LISTS isa_flags)
    string(REPLACE "." "\\." pattern "${flag}")
    # A leading character that cannot be part of a path or another flag, so `/home/-mts` and
    # `-Wno-class-memaccess` do not match, and nothing wider may follow the name.
    if(commands MATCHES "[ \"]-m${pattern}[ \"]")
        list(APPEND found "-m${flag}")
    endif()
endforeach()
if(found)
    list(REMOVE_DUPLICATES found)
    list(REMOVE_ITEM found ${allowed})
endif()
if(found)
    message(FATAL_ERROR "${found} reached a compile line: that is code for a wider ISA than the "
                        "-march=x86-64 pin, which ADR 0009 §3 forbids and ADR 0009 §6's cross-CPU "
                        "claim depends on. If it is deliberate, it belongs in docs/specs.md §8 "
                        "beside sfizz's and in the allowlist here. Find the target in ${COMPILE_COMMANDS}")
endif()
