# Reads compile_commands.json and fails on any flag ADR 0009 §3 forbids. Run as a build step
# before the engine compiles, so `cmake --build` itself is what fails.
#
# -Ofast and -funsafe-math-optimizations are -ffast-math by other spellings; -march is
# checked because a second baseline in a vendored tree is the same hazard from the other
# side — code compiled for a wider ISA than the pin.

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
