# The three bundled instruments of docs/specs.md §8 — Surge XT, sfizz and Dexed — built from the
# submodules under vendor/ at the commits lock.baseline.json pins (ADR 0010 §5; Airwindows is
# M4's).
#
# Each is a separate CMake project rather than an add_subdirectory. Surge vendors its own JUCE
# and Dexed another, Tracktion brings a third, and three sets of juce_* targets in one project
# collide by name — docs/plan.md trap 9, hit on day one exactly as it says. ExternalProject
# gives each plugin its own configure, its own cache and its own target namespace; the cost is a
# nested build, which is the cheapest thing that makes the collision impossible rather than
# managed.

include(ExternalProject)

# ADR 0009 §3 on every plugin compile line as on the engine's. add_compile_options does not
# cross into a nested project, so the same flags travel as CMAKE_{C,CXX}_FLAGS instead. The
# compiler travels too: a plugin built by a different g++ than the engine is a different set of
# bits under the same pin (ADR 0009 §1).
set(PLUGIN_FLAGS "-march=x86-64 -mtune=generic -ffp-contract=off")
set(PLUGIN_DIR ${CMAKE_BINARY_DIR}/plugins)

# name    the component's key in lock.baseline.json, which is also what --scan is handed
# SOURCE  the submodule directory under vendor/
# TARGET  the one target to build; a plugin repository also builds standalones, CLAPs, LV2s and
#         test runners, and none of them is what M1 hosts
# VST3    where that target leaves the bundle, relative to the nested build directory
# ARGS    what the plugin's own CMake needs to be told
function(escribass_plugin name)
    cmake_parse_arguments(P "" "SOURCE;TARGET;VST3" "ARGS" ${ARGN})
    set(build ${PLUGIN_DIR}/${name})

    # JUCE resolves its plugin copy directories from $ENV{HOME} at configure time and Dexed
    # asks for the copy, so an unredirected build installs a VST3 into the developer's ~/.vst3
    # — and then a scan could open the one in the home directory rather than the one just
    # built. HOME points inside the build tree, which also keeps anything else these trees do
    # with a home directory inside it.
    set(env ${CMAKE_COMMAND} -E env HOME=${build}/home)

    ExternalProject_Add(${name}
        SOURCE_DIR ${VENDOR}/${P_SOURCE}
        BINARY_DIR ${build}
        CONFIGURE_COMMAND ${env} ${CMAKE_COMMAND} -S <SOURCE_DIR> -B <BINARY_DIR> -G ${CMAKE_GENERATOR}
            -DCMAKE_BUILD_TYPE=Release
            -DCMAKE_C_COMPILER=${CMAKE_C_COMPILER} -DCMAKE_CXX_COMPILER=${CMAKE_CXX_COMPILER}
            -DCMAKE_C_FLAGS=${PLUGIN_FLAGS} -DCMAKE_CXX_FLAGS=${PLUGIN_FLAGS}
            -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
            # CI's ccache. A nested project inherits nothing from this one, and the plugins are
            # most of what the cache is for.
            -DCMAKE_C_COMPILER_LAUNCHER=${CMAKE_C_COMPILER_LAUNCHER}
            -DCMAKE_CXX_COMPILER_LAUNCHER=${CMAKE_CXX_COMPILER_LAUNCHER}
            ${P_ARGS}
        # BUILD_ALWAYS, because ExternalProject otherwise stamps the build as done and never
        # looks again: an edited plugin source, or a submodule moved to another commit, would
        # leave the old bundle in place and the manifest would describe it. That is trap 8 one
        # layer out. The nested ninja is a no-op when nothing changed.
        BUILD_COMMAND ${env} ${CMAKE_COMMAND} --build <BINARY_DIR> --target ${P_TARGET}
        BUILD_ALWAYS TRUE
        BUILD_BYPRODUCTS ${build}/${P_VST3}
        INSTALL_COMMAND ""
        TEST_COMMAND ""
        USES_TERMINAL_BUILD TRUE)

    # ADR 0009 §3's guard, on the plugin's own compile commands and between its configure and
    # its compile, which is where the engine's own runs. A vendored CMakeLists adding
    # -ffast-math or a second -march is the hazard this was written for, and sfizz is a live
    # instance of it — see cmake/sfizz-no-fast-math/SfizzConfig.cmake.
    ExternalProject_Add_Step(${name} flags
        COMMAND ${CMAKE_COMMAND} -DCOMPILE_COMMANDS=${build}/compile_commands.json
                                 -P ${CMAKE_CURRENT_SOURCE_DIR}/cmake/check_flags.cmake
        DEPENDEES configure DEPENDERS build ALWAYS TRUE
        COMMENT "${name}: no -ffast-math, one -march")

    set(${name}_VST3 ${build}/${P_VST3} PARENT_SCOPE)
endfunction()

escribass_plugin(surge_xt
    SOURCE surge
    TARGET surge-xt_VST3
    VST3 "src/surge-xt/surge-xt_artefacts/Release/VST3/Surge XT.vst3"
    # SURGE_SKIP_WERROR: Surge compiles its own tree with -Werror, which turns any warning a
    # compiler we pin and it does not adds into a build failure in a tree we do not maintain.
    # The other two switch off a standalone and a test runner M1 never runs.
    ARGS -DSURGE_SKIP_WERROR=TRUE -DSURGE_BUILD_TESTRUNNER=OFF -DSURGE_SKIP_STANDALONE=TRUE)

escribass_plugin(sfizz_ui
    # Trap 11, confirmed by the spike: sfizz 1.2.3 builds a library and a JACK client and no
    # VST3. This repository is where the plugin is, and it vendors the sfizz that
    # lock.baseline.json pins as its own `library` submodule.
    SOURCE sfizz-ui
    TARGET plugins_vst3
    VST3 "sfizz.vst3"
    # CMAKE_MODULE_PATH puts the -ffast-math shim in front of sfizz's own cmake directory; see
    # that file. The rest turns off what M1 does not host — the LV2 and Pure Data plugins, the
    # JACK client (whose headers would otherwise be a build dependency), the SMF renderer and
    # the shared library — and LTO, which triples the link for a plugin nobody profiles.
    ARGS -DCMAKE_MODULE_PATH=${CMAKE_CURRENT_SOURCE_DIR}/cmake/sfizz-no-fast-math
         -DESCRIBASS_SFIZZ_CMAKE=${VENDOR}/sfizz-ui/library/cmake
         -DPLUGIN_LV2=OFF -DPLUGIN_LV2_UI=OFF -DPLUGIN_PUREDATA=OFF
         -DSFIZZ_JACK=OFF -DSFIZZ_RENDER=OFF -DSFIZZ_SHARED=OFF -DENABLE_LTO=OFF)

escribass_plugin(dexed
    SOURCE dexed
    TARGET Dexed_VST3
    VST3 "Source/Dexed_artefacts/Release/VST3/Dexed.vst3")

# ------------------------------------------------------------------------------------------
# The build manifest (ADR 0010 §4)
# ------------------------------------------------------------------------------------------
#
# Generated here and never committed, for ADR 0008 §4's reason applied to a different artefact:
# a committed manifest is a second description of a plugin binary that must agree with the
# plugin binary, and the stale one is silent. It is written into the build tree, which
# engine/build/ already keeps out of git.
#
# ponytail: the build root, not $<TARGET_FILE_DIR:escribass_engine>, which CMake will not
# evaluate in a custom command's OUTPUT. Where a shipped engine looks for its manifest is a
# question PR 9 answers, when core is the one opening it.
set(MANIFEST ${CMAKE_BINARY_DIR}/manifest.json)
add_custom_command(
    OUTPUT ${MANIFEST}
    COMMAND $<TARGET_FILE:escribass_engine> --scan ${MANIFEST}
            surge_xt ${surge_xt_VST3}
            sfizz_ui ${sfizz_ui_VST3}
            dexed    ${dexed_VST3}
    DEPENDS escribass_engine ${surge_xt_VST3} ${sfizz_ui_VST3} ${dexed_VST3}
    COMMENT "--scan: the build manifest")
add_custom_target(manifest ALL DEPENDS ${MANIFEST})
add_dependencies(manifest surge_xt sfizz_ui dexed)
