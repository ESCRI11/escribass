// m5.1-spike measurement 2: two VST3 bundles that report the same class, loaded into one process
// through the engine's own JUCE, the way the engine's VST3 format loads them. Which binary runs?
//   twohost <a.vst3> <b.vst3> [...]
#include <juce_audio_processors/juce_audio_processors.h>
#include <iostream>

int main (int argc, char** argv)
{
    juce::ScopedJuceInitialiser_GUI gui;
    juce::VST3PluginFormat vst3;
    juce::KnownPluginList known;
    std::vector<std::unique_ptr<juce::AudioPluginInstance>> instances;
    std::vector<juce::PluginDescription> descs;
    for (int i = 1; i < argc; ++i)
    {
        juce::OwnedArray<juce::PluginDescription> types;
        vst3.findAllTypesForFile (types, juce::String::fromUTF8 (argv[i]));
        if (types.isEmpty()) { std::cerr << "no class in " << argv[i] << "\n"; return 2; }
        auto d = *types[0];
        known.addType (d);
        descs.push_back (d);
        juce::String error;
        auto inst = vst3.createInstanceFromDescription (d, 48000.0, 512, error);
        if (! inst) { std::cerr << "createInstance " << argv[i] << ": " << error << "\n"; return 3; }
        inst->setPlayConfigDetails (2, 2, 48000.0, 512);
        inst->prepareToPlay (48000.0, 512);
        std::cout << "loaded " << i << " name=" << d.name << " uniqueId=" << d.uniqueId << " deprecatedUid=" << d.deprecatedUid
                  << " identifier=" << d.createIdentifierString() << "\n";
        instances.push_back (std::move (inst));
    }
    std::cout << "knownPluginList types=" << known.getNumTypes() << "\n";
    for (size_t i = 0; i < descs.size(); ++i)
    {
        auto found = known.getTypeForIdentifierString (descs[i].createIdentifierString());
        std::cout << "lookup " << (i + 1) << " -> " << (found ? found->fileOrIdentifier : juce::String ("<none>")) << "\n";
    }
    // All instances alive at once; each processes a block of 0.5. Its gain says which binary ran.
    for (size_t i = 0; i < instances.size(); ++i)
    {
        juce::AudioBuffer<float> b (2, 512);
        for (int c = 0; c < 2; ++c) juce::FloatVectorOperations::fill (b.getWritePointer (c), 0.5f, 512);
        juce::MidiBuffer m;
        instances[i]->processBlock (b, m);
        std::cout << "instance " << (i + 1) << " out[0][100]=" << b.getSample (0, 100) << " -> gain " << b.getSample (0, 100) / 0.5f << "\n";
    }
    for (auto& i : instances) i->releaseResources();
    instances.clear();
    return 0;
}
