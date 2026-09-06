# The render engine: one binary, built from the submodules under vendor/ (docs/specs.md §8,
# ADR 0008). Everything here that is not a plain target definition exists because of a
# determinism hazard ADR 0009 §3 names, or a spike finding docs/plan.md records; each is
# cited where it is applied.

cmake_minimum_required(VERSION 3.22)
project(escribass_engine VERSION 0.1.0 LANGUAGES C CXX)

# ADR 0009 §1: what M1 claims is Linux x86-64, and the flags below are x86 flags. Another
# platform is not "unsupported", it is unmeasured, and a build that quietly succeeded there
# would look like a claim.
if(NOT CMAKE_SYSTEM_NAME STREQUAL "Linux" OR NOT CMAKE_SYSTEM_PROCESSOR MATCHES "x86_64|AMD64")
    message(FATAL_ERROR "the engine builds for Linux x86-64 only in M1 (ADR 0009 §1); this is ${CMAKE_SYSTEM_NAME} ${CMAKE_SYSTEM_PROCESSOR}")
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release CACHE STRING "" FORCE)
endif()
# The flag check below reads this; it is the ground truth of what reached the compiler.
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# ADR 0009 §3, applied to every target configured under this directory — the engine and all of
# vendor/ — before any of them is defined:
#   -march=x86-64      the compile-time ISA is a pin, not the distribution's default. The
#                      x86-64 baseline (SSE2) is what Tracktion, JUCE and choc require and
#                      nothing more; a wider baseline buys speed M1 does not need and lets
#                      the compiler pick instructions that round differently. Recorded in
#                      docs/specs.md §17 and lock.baseline.json ci.golden_render.
#   -ffp-contract=off  a*b+c must stay two roundings. Moot at this -march, which has no FMA,
#                      and kept so it stays true if the baseline is ever widened.
add_compile_options(-march=x86-64 -mtune=generic -ffp-contract=off)

# ------------------------------------------------------------------------------------------
# Submodules, and the commits they are at (ADR 0008 §5)
# ------------------------------------------------------------------------------------------

set(VENDOR ${CMAKE_CURRENT_SOURCE_DIR}/vendor)
foreach(sub tracktion_engine tracktion_engine/modules/juce protobuf)
    if(NOT EXISTS ${VENDOR}/${sub}/CMakeLists.txt)
        message(FATAL_ERROR "vendor/${sub} is not checked out; see the engine job in .github/workflows/checks.yml for the submodule steps (JUCE needs its URL overridden first)")
    endif()
endforeach()

# Taken from the checkout, never from lock.baseline.json: this reports what was compiled, so a
# drifted submodule is caught by the comparison rather than copied into it (ADR 0008 §5).
find_package(Git REQUIRED)
function(engine_commit_of dir out)
    execute_process(COMMAND ${GIT_EXECUTABLE} -C ${dir} rev-parse HEAD
                    OUTPUT_VARIABLE sha OUTPUT_STRIP_TRAILING_WHITESPACE RESULT_VARIABLE rc)
    if(rc)
        message(FATAL_ERROR "git rev-parse HEAD failed in ${dir}")
    endif()
    # Checking out another commit in the submodule must re-run configure, or the binary keeps
    # reporting the commit it was built from last time — trap 8 one level down.
    execute_process(COMMAND ${GIT_EXECUTABLE} -C ${dir} rev-parse --absolute-git-dir
                    OUTPUT_VARIABLE gitdir OUTPUT_STRIP_TRAILING_WHITESPACE)
    set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS ${gitdir}/HEAD)
    set(${out} ${sha} PARENT_SCOPE)
endfunction()
engine_commit_of(${VENDOR}/tracktion_engine              COMMIT_TRACKTION_ENGINE)
engine_commit_of(${VENDOR}/tracktion_engine/modules/juce COMMIT_JUCE)
engine_commit_of(${VENDOR}/protobuf                      COMMIT_PROTOBUF)
configure_file(src/provenance.h.in ${CMAKE_BINARY_DIR}/gen/provenance.h @ONLY)

# ------------------------------------------------------------------------------------------
# Protobuf: the runtime, protoc, and the C++ generated from the same .proto core uses
# (ADR 0008 §4). Generated into the build tree, never committed.
# ------------------------------------------------------------------------------------------

set(protobuf_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(protobuf_INSTALL OFF CACHE BOOL "" FORCE)
set(protobuf_WITH_ZLIB OFF CACHE BOOL "" FORCE)
set(protobuf_BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
add_subdirectory(${VENDOR}/protobuf EXCLUDE_FROM_ALL)

set(REPO ${CMAKE_CURRENT_SOURCE_DIR}/..)
set(GEN ${CMAKE_BINARY_DIR}/gen)
set(PROTOS ${REPO}/schema/song.proto ${REPO}/proto/render.proto)
add_custom_command(
    OUTPUT ${GEN}/song.pb.cc ${GEN}/song.pb.h ${GEN}/render.pb.cc ${GEN}/render.pb.h
    COMMAND ${CMAKE_COMMAND} -E make_directory ${GEN}
    # The vendored src/ is where google/protobuf/timestamp.proto lives; its C++ is in libprotobuf.
    COMMAND protoc -I ${REPO}/schema -I ${REPO}/proto -I ${VENDOR}/protobuf/src --cpp_out=${GEN} ${PROTOS}
    DEPENDS protoc ${PROTOS}
    COMMENT "protoc: song.proto and render.proto to C++")

# ------------------------------------------------------------------------------------------
# Tracktion Engine, which brings JUCE from the commit it pins (§17: never upgraded apart)
# ------------------------------------------------------------------------------------------

set(TE_ADD_EXAMPLES OFF CACHE BOOL "" FORCE)
add_subdirectory(${VENDOR}/tracktion_engine EXCLUDE_FROM_ALL)

# ------------------------------------------------------------------------------------------
# The engine
# ------------------------------------------------------------------------------------------

juce_add_console_app(escribass_engine
    PRODUCT_NAME "escribass_engine"
    NEEDS_CURL FALSE
    NEEDS_WEB_BROWSER FALSE)

target_sources(escribass_engine PRIVATE
    src/main.cpp
    ${GEN}/song.pb.cc
    ${GEN}/render.pb.cc)
target_include_directories(escribass_engine PRIVATE ${GEN})

target_compile_definitions(escribass_engine PRIVATE
    JUCE_STRICT_REFCOUNTEDPOINTER=1
    JUCE_MODAL_LOOPS_PERMITTED=1
    JUCE_USE_CURL=0
    JUCE_WEB_BROWSER=0
    # Headless: the device list would otherwise be logged at engine start.
    TRACKTION_LOG_DEVICES=0)

target_link_libraries(escribass_engine PRIVATE
    tracktion::tracktion_engine
    tracktion::tracktion_graph
    tracktion::tracktion_core
    # SHA-256 of the PCM (ADR 0009 §2, amended): a module of the pinned JUCE that Tracktion
    # does not pull in on its own.
    juce::juce_cryptography
    libprotobuf
    # Spike, 2026-09-04: tracktion_engine_playback references __atomic_store for 16-byte
    # atomics GCC does not lower inline.
    atomic)

# ADR 0009 §3: a vendored CMakeLists is free to put -ffast-math on its own targets, and it
# would be invisible until a golden moved. The generated compile commands are the ground
# truth, and the engine does not build until they have been read.
add_custom_target(engine_flags_check
    COMMAND ${CMAKE_COMMAND} -DCOMPILE_COMMANDS=${CMAKE_BINARY_DIR}/compile_commands.json
                             -P ${CMAKE_CURRENT_SOURCE_DIR}/cmake/check_flags.cmake
    COMMENT "compile_commands.json: no -ffast-math, one -march")
add_dependencies(escribass_engine engine_flags_check)
