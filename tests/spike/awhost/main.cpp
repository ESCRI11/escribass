// m5.1-spike. Build: cmake -S tests/spike/awhost -B <dir> -G Ninja -DCMAKE_BUILD_TYPE=Release
//   -DJUCE_DIR=<JUCE 51d11a2b checkout> -DCMAKE_CXX_FLAGS='-march=x86-64 -mtune=generic -ffp-contract=off'
// Run:   awhost '<Airwindows Consolidated.vst3>' <scenario> [args]   (scenarios: info, plain,
//   plaindry, probe, one, two, rev, srandtwo, reset, reprepare, switch, sweep, bypass, input,
//   randcost, randtrace). Not built by CI; it exists so a measurement can be re-taken.
// m5.1-spike measurement 4: a minimal VST3 host for the Airwindows Consolidated JUCE plugin.
// Hosts through juce_audio_processors, the same format implementation `engine/--scan` uses, so
// what it reports about ids and parameters is what this project's own host sees.
//
// Nothing here is meant to be merged. It exists to take numbers.

#include <juce_audio_processors/juce_audio_processors.h>
#include <juce_audio_utils/juce_audio_utils.h>
#include <juce_cryptography/juce_cryptography.h>

#include <cstdlib>
#include <iostream>
#include <vector>

namespace
{
constexpr double kSampleRate = 48000.0;
constexpr int kBlockSize = 512;
constexpr int kBlocks = 94; // 48,128 frames, just over a second

// A fixed-seed xorshift, ours and not the plugin's, so the input is the same every run on every
// process. Two seeds so L and R are decorrelated.
struct Noise
{
    uint32_t s;
    explicit Noise (uint32_t seed) : s (seed) {}
    float next()
    {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        return ((double) s / 4294967295.0 - 0.5) * 0.5f;
    }
};

juce::AudioPluginFormatManager& formats()
{
    static juce::AudioPluginFormatManager m;
    static bool once = false;
    if (! once)
    {
        m.addFormat (new juce::VST3PluginFormat());
        once = true;
    }
    return m;
}

std::unique_ptr<juce::AudioPluginInstance> load (const juce::String& path, juce::PluginDescription& descOut)
{
    auto& format = *formats().getFormat (0);
    juce::OwnedArray<juce::PluginDescription> types;
    format.findAllTypesForFile (types, path);
    if (types.isEmpty())
    {
        std::cerr << "no VST3 class in " << path << "\n";
        return {};
    }
    descOut = *types[0];
    juce::String error;
    auto inst = formats().createPluginInstance (*types[0], kSampleRate, kBlockSize, error);
    if (inst == nullptr)
        std::cerr << "createPluginInstance: " << error << "\n";
    return inst;
}

// The only route a host has to pick an Airwindows effect: the opaque state chunk, keyed by the
// effect's *name*. There is no parameter and no program for it (see the findings). A VST3 host's
// own state is an envelope around the plugin's chunk, so a selection is a read-modify-write of
// what the host already holds: handing the plugin a bare JUCE chunk is accepted and ignored.
bool selectEffect (juce::AudioPluginInstance& inst, const juce::String& effect, bool quiet = true)
{
    juce::MemoryBlock hostState;
    inst.getStateInformation (hostState);
    auto outer = juce::AudioProcessor::getXmlFromBinary (hostState.getData(), (int) hostState.getSize());
    if (outer == nullptr)
    {
        std::cerr << "host state is not JUCE binary xml\n";
        return false;
    }
    auto* component = outer->getChildByName ("IComponent");
    if (component == nullptr)
    {
        std::cerr << "host state has no IComponent child\n";
        return false;
    }
    juce::MemoryBlock inner;
    if (! inner.fromBase64Encoding (component->getAllSubText().trim()))
    {
        std::cerr << "IComponent is not a base64 MemoryBlock\n";
        return false;
    }
    auto innerXml = juce::AudioProcessor::getXmlFromBinary (inner.getData(), (int) inner.getSize());
    if (innerXml == nullptr || ! innerXml->hasTagName ("awconsolidated"))
    {
        std::cerr << "inner chunk is not an awconsolidated element\n";
        return false;
    }
    innerXml->setAttribute ("currentProcessorName", effect);
    juce::MemoryBlock newInner;
    juce::AudioProcessor::copyXmlToBinary (*innerXml, newInner);
    component->deleteAllChildElements();
    component->getFirstChildElement() == nullptr ? void() : void();
    // replace the text node
    for (auto* c = component->getFirstChildElement(); c != nullptr;)
    {
        auto* next = c->getNextElement();
        component->removeChildElement (c, true);
        c = next;
    }
    component->addTextElement (newInner.toBase64Encoding());
    juce::MemoryBlock newHostState;
    juce::AudioProcessor::copyXmlToBinary (*outer, newHostState);
    inst.setStateInformation (newHostState.getData(), (int) newHostState.getSize());

    // Read it back, because a state call that changes nothing returns void.
    juce::MemoryBlock check;
    inst.getStateInformation (check);
    auto o2 = juce::AudioProcessor::getXmlFromBinary (check.getData(), (int) check.getSize());
    juce::MemoryBlock i2;
    juce::String got ("<unreadable>");
    if (o2 != nullptr)
        if (auto* c2 = o2->getChildByName ("IComponent"))
            if (i2.fromBase64Encoding (c2->getAllSubText().trim()))
                if (auto x2 = juce::AudioProcessor::getXmlFromBinary (i2.getData(), (int) i2.getSize()))
                    got = x2->getStringAttribute ("currentProcessorName");
    if (! quiet)
        std::cout << "  selection asked for '" << effect << "', plugin reports '" << got << "'\n";
    if (got != effect)
    {
        std::cerr << "SELECTION DID NOT TAKE: asked '" << effect << "', got '" << got << "'\n";
        return false;
    }
    return true;
}

void setSlot (juce::AudioPluginInstance& inst, int slot, float value)
{
    if (auto* p = inst.getHostedParameter (slot))
        p->setValue (value);
}

void prepare (juce::AudioPluginInstance& inst)
{
    inst.setPlayConfigDetails (2, 2, kSampleRate, kBlockSize);
    inst.prepareToPlay (kSampleRate, kBlockSize);
}

// Renders kBlocks blocks of the fixed input and appends the interleaved float32 to `out`.
void render (juce::AudioPluginInstance& inst, std::vector<float>& out)
{
    juce::AudioBuffer<float> buf (2, kBlockSize);
    juce::MidiBuffer midi;
    Noise nl (0x12345678), nr (0x9e3779b9);
    for (int b = 0; b < kBlocks; ++b)
    {
        for (int n = 0; n < kBlockSize; ++n)
        {
            buf.setSample (0, n, nl.next());
            buf.setSample (1, n, nr.next());
        }
        inst.processBlock (buf, midi);
        for (int n = 0; n < kBlockSize; ++n)
        {
            out.push_back (buf.getSample (0, n));
            out.push_back (buf.getSample (1, n));
        }
    }
}

void write (const std::vector<float>& pcm, const juce::String& path)
{
    juce::File f (path);
    f.deleteFile();
    juce::FileOutputStream os (f);
    os.write (pcm.data(), pcm.size() * sizeof (float));
    os.flush();
    const auto sha = juce::SHA256 (pcm.data(), pcm.size() * sizeof (float)).toHexString();
    double peak = 0.0, energy = 0.0;
    size_t nonzero = 0;
    for (auto v : pcm)
    {
        peak = juce::jmax (peak, (double) std::abs (v));
        energy += (double) v * v;
        if (v != 0.0f)
            ++nonzero;
    }
    std::cout << "  wrote " << path << " frames=" << pcm.size() / 2 << " sha256=" << sha
              << " peak=" << peak << " rms=" << std::sqrt (energy / (double) pcm.size())
              << " nonzero=" << nonzero << "\n";
}

void dumpParams (juce::AudioPluginInstance& inst, const char* tag)
{
    std::cout << "params[" << tag << "] n=" << inst.getParameters().size() << "\n";
    for (int i = 0; i < inst.getParameters().size(); ++i)
    {
        auto* p = inst.getHostedParameter (i);
        std::cout << "  idx=" << i << " id=" << p->getParameterID() << " name='" << p->getName (64)
                  << "' value=" << p->getValue() << " default=" << p->getDefaultValue()
                  << " text='" << p->getText (p->getValue(), 32) << "'"
                  << " steps=" << p->getNumSteps() << " automatable=" << (int) p->isAutomatable()
                  << " discrete=" << (int) p->isDiscrete() << "\n";
    }
}
} // namespace

int main (int argc, char** argv)
{
    const juce::ScopedJuceInitialiser_GUI juceInit;
    if (argc < 3)
    {
        std::cerr << "usage: awhost <plugin.vst3> <scenario> [args]\n";
        return 2;
    }
    const juce::String path (argv[1]);
    const juce::String scenario (argv[2]);
    auto arg = [argc, argv] (int i, const char* def) { return i < argc ? juce::String (argv[i]) : juce::String (def); };

    juce::PluginDescription desc;

    if (scenario == "info")
    {
        auto inst = load (path, desc);
        if (! inst)
            return 1;
        std::cout << "name=" << desc.name << "\nmanufacturer=" << desc.manufacturerName
                  << "\nversion=" << desc.version << "\nformat=" << desc.pluginFormatName
                  << "\ncategory=" << desc.category << "\nuniqueId=" << desc.uniqueId
                  << "\ndeprecatedUid=" << desc.deprecatedUid << "\nisInstrument=" << (int) desc.isInstrument
                  << "\nfileOrIdentifier=" << desc.fileOrIdentifier << "\n";
        std::cout << "numPrograms=" << inst->getNumPrograms() << " current=" << inst->getCurrentProgram() << "\n";
        for (int i = 0; i < juce::jmin (5, inst->getNumPrograms()); ++i)
            std::cout << "  program " << i << " = '" << inst->getProgramName (i) << "'\n";
        std::cout << "tailSeconds=" << inst->getTailLengthSeconds() << " bypassParam="
                  << (inst->getBypassParameter() != nullptr ? inst->getBypassParameter()->getName (32) : juce::String ("<none>")) << "\n";
        dumpParams (*inst, "as loaded");
        // and after a selection, to see which of the above moves
        const auto effect = arg (3, "Galactic");
        selectEffect (*inst, effect);
        std::cout << "--- after selecting '" << effect << "' ---\n";
        dumpParams (*inst, effect.toRawUTF8());
        return 0;
    }

    // Reads the plugin's own state back, which is the only way a host can see which effect is
    // selected, and shows what each step of the sequence actually changed.
    if (scenario == "probe")
    {
        const auto effect = arg (3, "Channel5");
        auto inst = load (path, desc);
        if (! inst)
            return 1;
        auto showState = [&inst] (const char* tag) {
            juce::MemoryBlock b;
            inst->getStateInformation (b);
            auto xml = juce::AudioProcessor::getXmlFromBinary (b.getData(), (int) b.getSize());
            std::cout << "state[" << tag << "] size=" << b.getSize() << " -> "
                      << (xml != nullptr ? xml->toString (juce::XmlElement::TextFormat().singleLine())
                                         : juce::String ("<not JUCE xml: ") + juce::String::toHexString (b.getData(), (int) juce::jmin ((size_t) 48, b.getSize())) + ">")
                      << "\n";
        };
        auto showSlots = [&inst] (const char* tag) {
            std::cout << "slots[" << tag << "]";
            for (int i = 0; i < 5; ++i)
            {
                auto* p = inst->getHostedParameter (i);
                std::cout << " " << i << ":'" << p->getName (16) << "'=" << p->getValue();
            }
            std::cout << "\n";
        };
        showState ("as loaded");
        showSlots ("as loaded");
        selectEffect (*inst, effect);
        showState ("after select");
        showSlots ("after select");
        for (int i = 0; i < 5; ++i)
            setSlot (*inst, i, i == 4 ? 1.0f : 0.5f);
        showState ("after setValue");
        showSlots ("after setValue");
        prepare (*inst);
        std::vector<float> pcm;
        render (*inst, pcm);
        showState ("after render");
        showSlots ("after render");
        write (pcm, arg (4, "/dev/null"));
        // and now the same thing with the selection applied after prepareToPlay
        selectEffect (*inst, effect);
        showState ("select after prepare");
        showSlots ("select after prepare");
        std::vector<float> pcm2;
        render (*inst, pcm2);
        write (pcm2, arg (5, "/dev/null"));
        return 0;
    }

    // No selection, no setValue: load, prepare, render. If this varies per process then nothing
    // this host does is responsible.
    if (scenario == "plain" || scenario == "plaindry")
    {
        auto inst = load (path, desc);
        if (! inst)
            return 1;
        if (scenario == "plaindry")
            setSlot (*inst, 4, 0.0f); // Galactic's Dry/Wet to fully dry
        prepare (*inst);
        std::cout << "busIn=" << inst->getBus (true, 0)->getNumberOfChannels()
                  << " busOut=" << inst->getBus (false, 0)->getNumberOfChannels()
                  << " totalIn=" << inst->getTotalNumInputChannels()
                  << " totalOut=" << inst->getTotalNumOutputChannels()
                  << " doublePrecision=" << (int) inst->isUsingDoublePrecision()
                  << " latency=" << inst->getLatencySamples()
                  << " numBusesIn=" << inst->getBusCount (true)
                  << " numBusesOut=" << inst->getBusCount (false) << "\n";
        std::vector<float> pcm;
        render (*inst, pcm);
        write (pcm, arg (3, "/dev/null"));
        return 0;
    }

    // Where in the unseeded rand() sequence each step of a load lands. glibc's rand() from the
    // implicit seed 1 begins 1804289383, 846930886, 1681692777, ... so the index of a draw says
    // how many draws everything before it consumed.
    if (scenario == "randtrace")
    {
        std::vector<int> seq;
        {
            // the reference sequence, taken in a child-free way: reseed to 1 (the implicit seed)
            std::srand (1);
            for (int i = 0; i < 100000; ++i)
                seq.push_back (std::rand());
            std::srand (1);
        }
        auto where = [&seq] (int v) {
            for (size_t i = 0; i < seq.size(); ++i)
                if (seq[i] == v)
                    return (long) i;
            return -1L;
        };
        std::cout << "at main entry: draw index " << where (std::rand()) << "\n";
        juce::PluginDescription d2;
        auto& format = *formats().getFormat (0);
        juce::OwnedArray<juce::PluginDescription> types;
        format.findAllTypesForFile (types, path);
        std::cout << "after findAllTypesForFile: draw index " << where (std::rand()) << "\n";
        juce::String error;
        auto inst = formats().createPluginInstance (*types[0], kSampleRate, kBlockSize, error);
        std::cout << "after createPluginInstance: draw index " << where (std::rand()) << "\n";
        prepare (*inst);
        std::cout << "after prepareToPlay: draw index " << where (std::rand()) << "\n";
        return 0;
    }

    if (scenario == "one" || scenario == "srandone")
    {
        const auto effect = arg (3, "Galactic");
        const float wet = arg (4, "1.0").getFloatValue();
        const auto out = arg (5, "/dev/null");
        if (scenario == "srandone")
            std::srand (1);
        auto inst = load (path, desc);
        if (! inst)
            return 1;
        selectEffect (*inst, effect);
        for (int i = 0; i < 5; ++i)
            setSlot (*inst, i, i == 4 ? wet : 0.5f);
        prepare (*inst);
        std::vector<float> pcm;
        render (*inst, pcm);
        write (pcm, out);
        return 0;
    }

    // Two instances in one process. `order` decides which is constructed first; both render the
    // same input with the same settings, so any difference is the process's own construction
    // history and not the graph.
    if (scenario == "two" || scenario == "rev" || scenario == "srandtwo")
    {
        const auto effect = arg (3, "Galactic");
        const float wet = arg (4, "1.0").getFloatValue();
        const auto outA = arg (5, "/dev/null");
        const auto outB = arg (6, "/dev/null");
        const bool reversed = (scenario == "rev");
        const bool reseed = (scenario == "srandtwo");

        std::unique_ptr<juce::AudioPluginInstance> first, second;
        if (reseed)
            std::srand (1);
        first = load (path, desc);
        if (reseed)
            std::srand (1);
        second = load (path, desc);
        if (! first || ! second)
            return 1;
        // A is the one we call A regardless of which was built first.
        auto& a = reversed ? *second : *first;
        auto& b = reversed ? *first : *second;
        for (auto* inst : { &a, &b })
        {
            selectEffect (*inst, effect);
            for (int i = 0; i < 5; ++i)
                setSlot (*inst, i, i == 4 ? wet : 0.5f);
            prepare (*inst);
        }
        std::vector<float> pa, pb;
        render (a, pa);
        render (b, pb);
        std::cout << (reversed ? "reversed construction order (B built first)\n" : "forward construction order (A built first)\n");
        std::cout << " A:\n"; write (pa, outA);
        std::cout << " B:\n"; write (pb, outB);
        return 0;
    }

    // reset() twice over the same input. If reset resets, pass 2 equals pass 1.
    if (scenario == "reset" || scenario == "reprepare")
    {
        const auto effect = arg (3, "Galactic");
        const float wet = arg (4, "1.0").getFloatValue();
        const auto out1 = arg (5, "/dev/null");
        const auto out2 = arg (6, "/dev/null");
        auto inst = load (path, desc);
        if (! inst)
            return 1;
        selectEffect (*inst, effect);
        for (int i = 0; i < 5; ++i)
            setSlot (*inst, i, i == 4 ? wet : 0.5f);
        prepare (*inst);
        std::vector<float> p1, p2;
        render (*inst, p1);
        if (scenario == "reset")
        {
            inst->reset();
            std::cout << "called reset() between passes\n";
        }
        else
        {
            inst->releaseResources();
            prepare (*inst);
            std::cout << "called releaseResources() + prepareToPlay() between passes\n";
        }
        render (*inst, p2);
        std::cout << " pass1:\n"; write (p1, out1);
        std::cout << " pass2:\n"; write (p2, out2);
        return 0;
    }

    // What a selection change does to the ten slots: values, names and the sound they mean.
    if (scenario == "switch")
    {
        const auto effectA = arg (3, "Galactic");
        const auto effectB = arg (4, "Console8BussIn");
        const auto outA = arg (5, "/dev/null");
        const auto outB = arg (6, "/dev/null");
        auto inst = load (path, desc);
        if (! inst)
            return 1;
        selectEffect (*inst, effectA);
        for (int i = 0; i < 10; ++i)
            setSlot (*inst, i, 0.75f);
        prepare (*inst);
        std::cout << "--- A = " << effectA << ", every slot driven to 0.75 ---\n";
        dumpParams (*inst, "A");
        std::vector<float> pa;
        render (*inst, pa);
        std::cout << " A:\n"; write (pa, outA);

        // Selection change only. No parameter is touched afterwards.
        selectEffect (*inst, effectB);
        std::cout << "--- B = " << effectB << ", selection changed and nothing else ---\n";
        dumpParams (*inst, "B");
        std::vector<float> pb;
        render (*inst, pb);
        std::cout << " B:\n"; write (pb, outB);
        return 0;
    }

    // A single slot swept, to see whether it moves the sound at all (a bypassed instance is not
    // the measurement).
    if (scenario == "sweep")
    {
        const auto effect = arg (3, "Galactic");
        const int slot = arg (4, "0").getIntValue();
        const auto prefix = arg (5, "/dev/null");
        for (const float v : { 0.0f, 0.25f, 0.5f, 0.75f, 1.0f })
        {
            auto inst = load (path, desc);
            if (! inst)
                return 1;
            selectEffect (*inst, effect);
            for (int i = 0; i < 5; ++i)
                setSlot (*inst, i, i == 4 ? 1.0f : 0.5f);
            setSlot (*inst, slot, v);
            prepare (*inst);
            std::vector<float> pcm;
            render (*inst, pcm);
            std::cout << " slot " << slot << " = " << v << ":\n";
            write (pcm, prefix + juce::String (v) + ".raw");
        }
        return 0;
    }

    // Bypass, for the "a selected but bypassed instance is not the measurement" clause.
    if (scenario == "bypass")
    {
        const auto effect = arg (3, "Galactic");
        const auto out = arg (4, "/dev/null");
        auto inst = load (path, desc);
        if (! inst)
            return 1;
        selectEffect (*inst, effect);
        for (int i = 0; i < 5; ++i)
            setSlot (*inst, i, i == 4 ? 1.0f : 0.5f);
        if (auto* bp = inst->getBypassParameter())
            bp->setValue (1.0f);
        prepare (*inst);
        std::vector<float> pcm;
        render (*inst, pcm);
        write (pcm, out);
        return 0;
    }

    // The input itself, for comparison against a bypassed or dry render.
    if (scenario == "input")
    {
        std::vector<float> pcm;
        Noise nl (0x12345678), nr (0x9e3779b9);
        for (int b = 0; b < kBlocks; ++b)
            for (int n = 0; n < kBlockSize; ++n)
            {
                pcm.push_back (nl.next());
                pcm.push_back (nr.next());
            }
        write (pcm, arg (3, "/dev/null"));
        return 0;
    }

    // How many rand() draws one instance construction costs, measured from outside by seeding
    // and counting how far the sequence has advanced.
    if (scenario == "randcost")
    {
        std::srand (1);
        std::vector<int> expected;
        for (int i = 0; i < 64; ++i)
            expected.push_back (std::rand());
        std::srand (1);
        auto inst = load (path, desc);
        if (! inst)
            return 1;
        const int after = std::rand();
        int draws = -1;
        for (size_t i = 0; i < expected.size(); ++i)
            if (expected[i] == after)
                draws = (int) i;
        std::cout << "rand() draws consumed by one createPluginInstance: " << draws
                  << " (next value " << after << ")\n";
        std::srand (1);
        auto i2 = load (path, desc);
        auto i3 = load (path, desc);
        const int after3 = std::rand();
        int draws3 = -1;
        std::srand (1);
        std::vector<int> e2;
        for (int i = 0; i < 4096; ++i)
            e2.push_back (std::rand());
        for (size_t i = 0; i < e2.size(); ++i)
            if (e2[i] == after3)
                draws3 = (int) i;
        std::cout << "rand() draws consumed by two createPluginInstance: " << draws3 << "\n";
        return 0;
    }

    std::cerr << "unknown scenario " << scenario << "\n";
    return 2;
}
