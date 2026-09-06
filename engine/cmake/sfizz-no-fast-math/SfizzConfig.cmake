# sfizz compiles its DSP core with -ffast-math (library/cmake/SfizzConfig.cmake calls
# sfizz_enable_fast_math on sfizz_internal), which ADR 0009 §3 forbids on any compile line the
# engine's output depends on: it licenses reassociation and flushes the algebra the golden is a
# statement about. engine/cmake/check_flags.cmake exists to catch exactly this, and it does.
#
# The fix is not to weaken the guard and not to edit a pinned submodule. sfizz appends to
# CMAKE_MODULE_PATH rather than replacing it, so this directory goes in front of
# library/cmake and `include(SfizzConfig)` lands here instead: the real file runs, and the
# function it defined is then replaced with one that adds nothing.
#
# The rest of SfizzConfig — warnings, -msse2, the C++ standard — is upstream's and is left
# alone. -msse2 is the x86-64 baseline the engine already pins, so it changes nothing.
include(${ESCRIBASS_SFIZZ_CMAKE}/SfizzConfig.cmake)

function(sfizz_enable_fast_math NAME)
endfunction()
