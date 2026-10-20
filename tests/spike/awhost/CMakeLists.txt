cmake_minimum_required(VERSION 3.22)
project(awhost VERSION 1.0.0 LANGUAGES C CXX)
set(CMAKE_CXX_STANDARD 17)
add_subdirectory(${JUCE_DIR} juce_build)
juce_add_console_app(awhost PRODUCT_NAME "awhost")
target_sources(awhost PRIVATE main.cpp)
target_compile_definitions(awhost PRIVATE
    JUCE_PLUGINHOST_VST3=1
    JUCE_PLUGINHOST_VST=0
    JUCE_PLUGINHOST_AU=0
    JUCE_WEB_BROWSER=0
    JUCE_USE_CURL=0
    JUCE_JACK=0
    JUCE_ALSA=0
    JUCE_MODAL_LOOPS_PERMITTED=0
    JUCE_STANDALONE_APPLICATION=1)
target_link_libraries(awhost PRIVATE
    juce::juce_audio_processors
    juce::juce_audio_utils
    juce::juce_cryptography
    juce::juce_recommended_config_flags)
