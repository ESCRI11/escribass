// The render engine (docs/specs.md §8): one RenderPlan in on stdin, one WAV out at the path
// the plan names, one RenderResult out on stdout, exit. A fresh process per render, so nothing
// here outlives a render and nothing is reused (ADR 0008 §2).
//
// stdout carries protobuf bytes and nothing else. Every line this file writes goes to stderr,
// and JUCE's own logger does the same on Linux (juce_SystemStats_linux.cpp: outputDebugString
// is std::cerr), which is why nothing here installs a logger (ADR 0008 §1).
//
// The plan names its plugins by the id the build manifest declares (ADR 0010 §4), and the
// manifest's path is the engine's one argument: a render opens the exact binaries it names and
// walks no directory, which is what makes a fresh process per render cost one dlopen rather
// than a scan (ADR 0008 §2). Where a shipped engine finds that file is still PR 9's question —
// core is the one that will pass it — so it is an argument here and not a search.
//
// A sampler instrument is an SFZ from assets/ played by the bundled sfizz (PR 8b, §16). What
// M1 renders is the restricted case `checkSfz` below enforces, because assets/ is flat and
// content-addressed and an SFZ that reaches out of it reaches nothing.
//
// `--scan` is the build's second use of this binary (ADR 0010 §4). It opens each bundled VST3
// once, asks it what it is, and writes the manifest. It lives here rather than in a second
// binary because the provenance below and the VST3 loading above it are the same two things a
// render needs, and a second JUCE console app would compile every JUCE module again for the
// sake of one file.

#include <tracktion_engine/tracktion_engine.h>
#include <juce_cryptography/juce_cryptography.h>
#include <rubberband/RubberBandStretcher.h>

#include "provenance.h"
#include "render.pb.h"

#include <pmmintrin.h>
#include <xmmintrin.h>

#include <algorithm>
#include <atomic>
#include <cmath>
#include <cstdio>
#include <iostream>
#include <iterator>
#include <limits>
#include <map>
#include <memory>
#include <string>
#include <string_view>
#include <unistd.h>
#include <vector>

namespace te = tracktion;
using escribass::render::v1::PlanClip;
using escribass::render::v1::PlanLane;
using escribass::render::v1::PlanTrack;
using escribass::render::v1::RenderPlan;
using escribass::render::v1::RenderResult;

namespace {

// docs/specs.md §4.2: every tick in a plan is a model tick.
constexpr int kPpq = 960;

// ADR 0009 §3: one block size for the whole render. Tracktion's own default, named so it is a
// pin and not a default.
constexpr int kBlockSize = 512;

// Failure is an exit code (ADR 0008 §1). By the time a plan is here every caller-fixable
// failure was refused upstream, so a plan this engine cannot take is an operator error like
// the other two, and the code only says which stage the render died in.
enum Exit : int {
    kOk = 0,
    kBadPlan = 2,       // stdin was not a plan, or names something no valid song produces
    kRenderFailed = 3,  // Tracktion reported an error, or produced no file
    kBadOutput = 4,     // the file is not the WAV the plan asked for
    kScanFailed = 5,    // --scan: a bundled plugin did not open, or said nothing about itself
};

int fail (Exit code, const std::string& why)
{
    std::fprintf (stderr, "escribass_engine: %s\n", why.c_str());
    return code;
}

// Headless (spike, 2026-09-04): no device manager, no system audio devices, no input.
// getNumberOfCPUsToUseForAudio is the single-thread pin of ADR 0009 §3: Tracktion hands the
// render's node player this minus one worker threads (tracktion_NodeRenderContext.cpp), and
// zero workers means every block is processed on the render thread in graph order.
class Behaviour : public te::EngineBehaviour {
    bool autoInitialiseDeviceManager() override { return false; }
    bool addSystemAudioIODeviceTypes() override { return false; }
    bool shouldOpenAudioInputByDefault() override { return false; }
    int getNumberOfCPUsToUseForAudio() override { return 1; }
};

// Tracktion keeps a settings file and a temp directory under the user's application data
// directory. A render process must leave nothing behind and start from nothing (ADR 0008
// §2), so both live in a directory this process creates and deletes. The deletion is a
// separate guard that outlives the Engine: PropertyStorage's own destructor saves the
// settings file, so a directory deleted any earlier comes back.
struct TempDir {
    juce::File dir = juce::File::getSpecialLocation (juce::File::tempDirectory)
                         .getChildFile ("escribass_engine." + juce::String (getpid()));
    ~TempDir() { dir.deleteRecursively(); }
};

class Storage : public te::PropertyStorage {
public:
    explicit Storage (juce::File dir_) : te::PropertyStorage ("escribass_engine"), dir (std::move (dir_)) {}

    juce::File getAppPrefsFolder() override
    {
        dir.createDirectory();
        return dir;
    }

    juce::File getAppCacheFolder() override { return getAppPrefsFolder(); }

private:
    juce::File dir;
};

// Refuses what no valid song compiles to. The validator and compile already refused every
// caller-fixable shape (ADR 0008 §1); this is the trust boundary, not a second validator.
std::string check (const RenderPlan& plan, juce::AudioFormat& wav)
{
    if (! plan.has_target())
        return "the plan has no target";
    const auto& target = plan.target();
    if (target.kind() != escribass::song::v1::RENDER_KIND_MASTER)
        return "the target is not a master render; M1 renders the master (ADR 0007 §6)";
    if (! wav.getPossibleSampleRates().contains ((int) target.sample_rate()))
        return "sample rate " + std::to_string (target.sample_rate()) + " is not one the WAV format writes";
    if (! wav.getPossibleBitDepths().contains ((int) target.bit_depth()))
        return "bit depth " + std::to_string (target.bit_depth()) + " is not one the WAV format writes";
    if (target.dither())
        return "dither is a noise source without a seed; compile refuses it (ADR 0007 §6)";
    if (plan.length_ticks() <= 0)
        return "length_ticks is " + std::to_string (plan.length_ticks()) + "; nothing to render";
    if (plan.tempo_size() == 0 || plan.tempo (0).tick() != 0)
        return "the tempo map has no event at tick 0 (§4.4)";
    for (const auto& event : plan.tempo())
        if (! (event.bpm() > 0.0))
            return "a tempo of " + std::to_string (event.bpm()) + " at tick " + std::to_string (event.tick());
    if (! juce::File::isAbsolutePath (plan.output_path()))
        return "output_path is not absolute: '" + plan.output_path() + "'";
    return {};
}

// Walks the RIFF chunks of the file Tracktion wrote to its `data` chunk and leaves the stream
// at the first PCM byte. Returns the chunk's size, or an explanation.
tl::expected<juce::int64, std::string> seekToData (juce::FileInputStream& in)
{
    char id[4];
    auto readId = [&] { return in.read (id, 4) == 4; };
    if (! readId())
        return tl::unexpected (std::string ("the file is empty"));
    if (std::string_view (id, 4) == "RF64")
        // ponytail: JUCE switches to RF64 past 4 GB, where sizes live in a ds64 chunk. A render
        // that long is not one M1 goldens; walk ds64 when someone needs it.
        return tl::unexpected (std::string ("the file is RF64 (over 4 GB), which this engine does not walk"));
    if (std::string_view (id, 4) != "RIFF")
        return tl::unexpected (std::string ("the file is not RIFF"));
    in.readInt();  // RIFF size
    if (! readId() || std::string_view (id, 4) != "WAVE")
        return tl::unexpected (std::string ("the file is RIFF but not WAVE"));

    while (readId())
    {
        const auto size = (juce::int64) (juce::uint32) in.readInt();
        if (std::string_view (id, 4) == "data")
            return size;
        // Chunks are word-aligned; the pad byte is not counted in the size.
        in.setPosition (in.getPosition() + size + (size & 1));
    }
    return tl::unexpected (std::string ("the file has no data chunk"));
}

// -----------------------------------------------------------------------------------------
// --scan: the build manifest (ADR 0010 §4)
// -----------------------------------------------------------------------------------------

// A plugin id ends up as a key in a project's lock.json, which ADR 0010 §1 keeps free of
// anything machine-specific because that file is committed to the user's repository and
// byte-compared by the determinism suite. That rules out JUCE's own
// PluginDescription::createIdentifierString(), which hashes the plugin's path into the string.
// What is left, and what the VST3 factory actually reports for the class, is the vendor and
// the class name.
//
// ponytail: the ceiling is two plugins with one vendor and one name, which the three bundled
// ones are not. The upgrade path is the VST3 class UID — unique by construction, and reachable
// only past JUCE, which keeps it as the 32-bit hash in PluginDescription::uniqueId.
juce::String pluginIdOf (const juce::PluginDescription& desc)
{
    return desc.manufacturerName + "/" + desc.name;
}

// Opens each bundled VST3 once and writes what it says about itself. Argument order is
// `<manifest> <component> <path>...`, where the component is its key in lock.baseline.json;
// the commit comes from the provenance compiled in at configure time rather than from the
// caller, so the manifest cannot claim a commit the binary was not built against.
int scan (const juce::StringArray& args)
{
    if (args.size() < 3 || args.size() % 2 != 1)
        return fail (kScanFailed, "usage: --scan <manifest.json> <component> <plugin.vst3> ...");

    const juce::ScopedJuceInitialiser_GUI juceInit;
    juce::AudioPluginFormatManager manager;
    manager.addFormat (new juce::VST3PluginFormat());
    auto& format = *manager.getFormat (0);

    // Sorted, because ADR 0010 §1 wants the pins it feeds sorted and a manifest that reorders
    // itself between builds would make every diff of a derived file unreadable.
    std::map<juce::String, juce::var> plugins;

    for (int i = 1; i + 1 < args.size(); i += 2)
    {
        const auto& component = args[i];
        const auto& path = args[i + 1];

        const char* commit = nullptr;
        for (const auto& c : escribass::provenance::pluginCommits)
            if (component == c.component)
                commit = c.sha;
        if (commit == nullptr)
            return fail (kScanFailed, "no submodule commit is compiled in for '" + component.toStdString() + "'");

        juce::OwnedArray<juce::PluginDescription> types;
        format.findAllTypesForFile (types, path);
        if (types.isEmpty())
            // A module that fails to dlopen looks exactly like one that declares nothing:
            // JUCE reports neither. The hint is here because the difference costs an hour.
            return fail (kScanFailed, path.toStdString() + ": no VST3 audio effect class. Either the bundle"
                                          " declares none, or it did not load — check `ldd` on the .so inside it");

        for (const auto* desc : types)
        {
            // Instantiating is the only way to ask for the parameters, and it is where a
            // headless VST3 either works or does not (trap 10). 48 kHz and 512 frames are
            // the render's own rate and block size; a scan reads no audio, so they only have
            // to be something the plugin accepts.
            juce::String error;
            auto instance = manager.createPluginInstance (*desc, 48000.0, kBlockSize, error);
            if (instance == nullptr)
                return fail (kScanFailed, path.toStdString() + ": " + error.toStdString());

            // Both halves, because a VST3 host is shown exactly two things about a parameter
            // and neither does the whole job. The id is the plugin's own ParamID and is unique
            // by the format's rules; the name is what a person reads and is not — Surge XT
            // repeats 176 of its 2855 names, one per unassigned effect slot, so a name cannot
            // be what a ParamRef resolves through. The key is therefore the identifier and the
            // value is the label. PR 9 is where a ParamRef is checked against these keys.
            auto* params = new juce::DynamicObject();
            for (int n = 0; n < instance->getParameters().size(); ++n)
            {
                const auto* param = instance->getHostedParameter (n);
                params->setProperty (param->getParameterID(), param->getName (1024));
            }

            // Two entries under one id would leave the second one addressing the first one's
            // binary, which is ADR 0010 §3's lock_mismatch arriving as the wrong plugin
            // instead of as an error.
            const auto id = pluginIdOf (*desc);
            if (plugins.count (id) != 0)
                return fail (kScanFailed, "two plugins report the id '" + id.toStdString() + "'");

            auto* entry = new juce::DynamicObject();
            entry->setProperty ("commit", juce::String (commit));
            entry->setProperty ("params", juce::var (params));
            entry->setProperty ("path", path);
            entry->setProperty ("version", desc->version);
            plugins[id] = juce::var (entry);
        }
    }

    std::map<juce::String, juce::String> sortedEngine;
    for (const auto& c : escribass::provenance::engineCommits)
        sortedEngine[c.component] = c.sha;

    auto* engine = new juce::DynamicObject();
    for (const auto& [component, sha] : sortedEngine)
        engine->setProperty (component, sha);

    auto* pluginsVar = new juce::DynamicObject();
    for (const auto& [id, entry] : plugins)
        pluginsVar->setProperty (id, entry);

    auto* root = new juce::DynamicObject();
    root->setProperty ("engine", juce::var (engine));
    root->setProperty ("plugins", juce::var (pluginsVar));

    const juce::File out (args[0]);
    if (! out.replaceWithText (juce::JSON::toString (juce::var (root), false) + "\n"))
        return fail (kScanFailed, "could not write the manifest to " + args[0].toStdString());
    return kOk;
}

// -----------------------------------------------------------------------------------------
// Hosting the plan (PR 7)
// -----------------------------------------------------------------------------------------

// Every parameter value in a plan — `Instrument.params`, `Effect.params` and every
// `AutomationPoint.value` — is the plugin's own **normalised** value. That is the only domain
// a VST3 offers a host: the format's `ParamValue` is 0..1 and JUCE hands it through unchanged,
// which is the same fact that leaves ADR 0010 §4's manifest keying a parameter by its opaque
// `ParamID`. Out of range is clamped rather than refused, because Tracktion's own parameter
// range clamps it either way and a value that means nothing is PR 9's `param_unknown` — a
// caller error, which by ADR 0008 §1 is not one the engine is left to discover.
float normalised (double value)
{
    return (float) juce::jlimit (0.0, 1.0, value);
}

// The build manifest (ADR 0010 §4) and the plugin descriptions it resolves to.
//
// A file is opened once per render, at the exact path the manifest names, and no directory is
// walked. That is the scan ADR 0008 §2 removes so a fresh process per render costs one dlopen
// per referenced plugin; a manifest that has drifted from the binaries is caught here as a
// class the file does not declare, rather than by loading whatever is at the path.
//
// Each class the file declares is registered with Tracktion's own list, because that is where
// ExternalPlugin looks a description up when it is created — findMatchingPlugin() searches
// knownPluginList and nothing else (tracktion_ExternalPlugin.cpp).
class Plugins {
public:
    Plugins (te::Engine& e, juce::var manifest_) : engine (e), manifest (std::move (manifest_)) {}

    tl::expected<juce::PluginDescription, std::string> describe (const std::string& id)
    {
        if (const auto known = byId.find (id); known != byId.end())
            return known->second;

        const auto path = pathOf (id);
        if (path.isEmpty())
            // ADR 0010 §3 refuses a referenced plugin this build lacks when the project is
            // opened, so a plan carrying one is an operator error like the rest of them.
            return tl::unexpected ("the manifest names no plugin '" + id + "'");

        juce::OwnedArray<juce::PluginDescription> types;
        vst3.findAllTypesForFile (types, path);
        if (types.isEmpty())
            return tl::unexpected (path.toStdString() + ": no VST3 audio class. Either the bundle"
                                       " declares none, or it did not load — check `ldd` on the .so inside it");
        for (const auto* found : types)
        {
            engine.getPluginManager().knownPluginList.addType (*found);
            byId.emplace (pluginIdOf (*found).toStdString(), *found);
        }

        if (const auto known = byId.find (id); known != byId.end())
            return known->second;
        return tl::unexpected (path.toStdString() + " declares no class called '" + id
                               + "'; the manifest describes another build");
    }

private:
    juce::String pathOf (const std::string& id) const
    {
        if (auto* plugins = manifest["plugins"].getDynamicObject())
            for (const auto& entry : plugins->getProperties())
                // Compared as text. A plugin id is a vendor and a class name (ADR 0010 §4), so
                // it holds spaces and a slash and is not a juce::Identifier by that class's own
                // rules, even though the manifest's object stores it as one.
                if (entry.name.toString() == juce::String (id))
                    return entry.value["path"].toString();
        return {};
    }

    te::Engine& engine;
    juce::var manifest;
    juce::VST3PluginFormat vst3;
    std::map<std::string, juce::PluginDescription> byId;
};

// -----------------------------------------------------------------------------------------
// The sampler (PR 8b)
// -----------------------------------------------------------------------------------------

// What an `Instrument.ref.sampler` is played by. The model names an SFZ and never a plugin —
// which is why ADR 0010 §1 gives `SamplerRef` no lock.json entry, since it already references
// content by hash — so *which* sampler plays it is the renderer's fact and belongs here. §8
// bundles sfizz; what pins it is this binary's own sfizz_ui commit, which every RenderResult
// reports (ADR 0008 §5).
constexpr const char* kSamplerPluginId = "SFZTools/sfizz";

// The restricted case M1 renders, enforced rather than described.
//
// `assets/` is flat and content-addressed: one file per asset, named by its SHA-256, with no
// directory and no extension (core/src/project.rs, §10). sfizz resolves a `sample=` against
// the directory the SFZ was loaded from (FilePool::checkSample joins `rootDirectory` to it),
// so an SFZ stored in `assets/` can reach a sample only by naming a sibling — which, there,
// means naming it by its own hash. An SFZ carrying the relative paths of a sample library
// resolves none of them.
//
// The failure that makes this a check rather than a comment is silent. sfizz drops a region
// whose sample it cannot find and says nothing about it (Synth.cpp: the removal is a `DBG`,
// which a release build compiles out), so an unresolvable SFZ renders as a track of silence
// that reports success — the failure ADR 0010 §3 spends a table refusing.
//
// ponytail: a `sample=` value is read as one whitespace-delimited token, which is the
// restricted form and not the SFZ language — a name with a space in it, a `default_path`
// prefix or an `#include` is refused here rather than half-understood. The upgrade is the
// general case, an SFZ that arrives with its samples under the names it was written against,
// and that needs an asset that knows its own name; no ADR has decided one.
std::string checkSfz (const juce::File& sfz)
{
    if (! sfz.existsAsFile())
        // core resolved this path from `SamplerRef.sfz_hash` and refused a hash `assets/` does
        // not hold (core/src/render.rs), so what is left is a file that has gone since.
        return "'" + sfz.getFullPathName().toStdString() + "' is not a file";
    const auto text = sfz.loadFileAsString();

    // Comments first, so an opcode inside one is not read as an opcode.
    juce::String body;
    for (const auto& line : juce::StringArray::fromLines (text))
        body << line.upToFirstOccurrenceOf ("//", false, false) << "\n";

    for (const auto* directive : { "#include", "default_path" })
        if (body.containsIgnoreCase (directive))
            return std::string ("this SFZ uses `") + directive + "`, which moves where its samples"
                   " are looked for; M1 plays an SFZ whose samples are its siblings in assets/";

    const juce::String opcode ("sample=");
    int found = 0;
    for (auto at = body.indexOfIgnoreCase (opcode); at >= 0;
         at = body.indexOfIgnoreCase (at + 1, opcode))
    {
        // `sample` and not the tail of a longer opcode. Anything that is not an opcode
        // character ends the one before, which is how `<region>sample=` is one and
        // `hint_sample=` is not.
        const auto before = at == 0 ? juce::juce_wchar (' ') : body[at - 1];
        if (juce::CharacterFunctions::isLetterOrDigit (before) || before == '_')
            continue;
        ++found;
        const auto value = body.substring (at + opcode.length()).initialSectionNotContaining (" \t\r\n");
        if (value.isEmpty())
            return "this SFZ has a `sample=` with nothing after it";
        // sfizz's built-in generators — *sine, *saw, *noise and the rest — are not files and
        // resolve to nothing on disk.
        if (value.startsWithChar ('*'))
            continue;
        if (! sfz.getSiblingFile (value).existsAsFile())
            return "this SFZ plays `" + value.toStdString() + "`, which is not beside it in assets/."
                   " An SFZ in a content-addressed store names its samples by their own hash";
    }
    if (found == 0)
        return "this SFZ names no sample, so nothing on this track could sound";
    return {};
}

// The SFZ path sfizz's component state holds: past a little-endian `uint64` version, an
// IBStreamer `str8` — an `int32` byte count that includes the terminating NUL, then the bytes
// (VST3 SDK, `base/source/fstreamer.cpp`; sfizz's own order is `SfizzVstState::store`). Those
// two fields are the whole of what this file assumes about that format.
constexpr int kSfizzPathAt = 8 + 4;

tl::expected<std::string, std::string> sfzPathIn (const juce::MemoryBlock& state)
{
    const auto* bytes = static_cast<const char*> (state.getData());
    const auto size = (juce::int64) state.getSize();
    const auto count = size < kSfizzPathAt
                         ? 0
                         : (juce::int64) (juce::uint32) juce::ByteOrder::littleEndianInt (bytes + 8);
    if (count < 1 || kSfizzPathAt + count > size)
        return tl::unexpected ("sfizz's state is " + std::to_string (size) + " bytes declaring a "
                               + std::to_string (count) + "-byte path; this is not the state"
                               " SfizzVstState writes");
    return std::string (bytes + kSfizzPathAt, (size_t) count - 1);
}

// A VST3's component state, out of the XML JUCE wraps it in: `getStateInformation` writes one
// document with the component's and the controller's states base64-encoded inside it
// (juce_VST3PluginFormatImpl.h, `appendStateFrom`).
tl::expected<juce::MemoryBlock, std::string> componentStateOf (juce::AudioPluginInstance& instance)
{
    juce::MemoryBlock blob;
    instance.getStateInformation (blob);
    const auto head = juce::AudioProcessor::getXmlFromBinary (blob.getData(), (int) blob.getSize());
    const auto* component = head == nullptr ? nullptr : head->getChildByName ("IComponent");
    juce::MemoryBlock state;
    if (component == nullptr || ! state.fromBase64Encoding (component->getAllSubText()))
        return tl::unexpected (std::string ("the plugin's state is not the document JUCE writes"));
    return state;
}

// Loads an SFZ into an instance of sfizz.
//
// A state is the only channel: the plugin exposes no parameter and no host-sendable message
// that names a file, and `SfizzVstProcessor::setState` is what calls `loadSfzFileOrDefault`.
// So the plugin's *own* state is read back and the path spliced into it, rather than a state
// of ours written from scratch — which would freeze this build's copy of every default sfizz
// has, and would silently pin them to whatever they were the day it was written. Everything
// after the path is copied through untouched, so a field sfizz adds later travels with it.
std::string loadSfz (juce::AudioPluginInstance& instance, const juce::File& sfz)
{
    if (const auto why = checkSfz (sfz); ! why.empty())
        return why;

    const auto state = componentStateOf (instance);
    if (! state)
        return state.error();
    const auto held = sfzPathIn (*state);
    if (! held)
        return held.error();

    const auto path = sfz.getFullPathName();
    const auto length = (juce::uint32) (path.getNumBytesAsUTF8() + 1);
    const auto* bytes = static_cast<const char*> (state->getData());
    const auto tail = kSfizzPathAt + held->size() + 1;

    juce::MemoryBlock spliced;
    spliced.append (bytes, 8);
    const auto count = juce::ByteOrder::swapIfBigEndian (length);
    spliced.append (&count, sizeof (count));
    spliced.append (path.toRawUTF8(), length);
    spliced.append (bytes + tail, state->getSize() - tail);

    juce::XmlElement head ("VST3PluginState");
    head.createNewChildElement ("IComponent")->addTextElement (spliced.toBase64Encoding());
    juce::MemoryBlock blob;
    juce::AudioProcessor::copyXmlToBinary (head, blob);
    instance.setStateInformation (blob.getData(), (int) blob.getSize());

    // Read back, because every line above is an assumption about a format this repository does
    // not own. A state sfizz declined leaves it playing the sine of its own default patch — a
    // track that sounds, at the wrong everything, with no error anywhere. The read-back is also
    // the plugin agreeing it found the file: `SfizzVstProcessor::setState` searches the machine
    // for a path that does not exist and stores whatever it finds, which would come back as a
    // different path (and is a render reading a file nobody named — `checkSfz` above is what
    // keeps it out of reach).
    const auto after = componentStateOf (instance);
    if (! after)
        return after.error();
    const auto loaded = sfzPathIn (*after);
    if (! loaded)
        return loaded.error();
    if (*loaded != path.toStdString())
        return "sfizz kept '" + *loaded + "' rather than the SFZ the plan named";
    return {};
}

// -----------------------------------------------------------------------------------------
// Audio clips (PR 8)
// -----------------------------------------------------------------------------------------

// ADR 0011 §3's option word, written out in full. Every group Rubber Band 4.0.0 declares is
// named, the eight whose value is numerically zero included, because the point is not what the
// number comes to — it comes to `OptionEngineFiner | OptionThreadingNever` — but that a moved
// upstream default shows up as a diff in this file rather than as a golden nobody can account
// for (trap 15: a pinned version of a phase vocoder is not a pinned output).
//
// Two are worth reading rather than skimming. `OptionProcessOffline` is what runs the study
// pass, and it is also what makes Rubber Band pad and compensate its own delay so a stretched
// result has an exact start and duration — the property `audio` below relies on when it asks
// for a buffer the length of the clip. `OptionThreadingNever` is ADR 0009 §3's single thread,
// and it is inert twice over: the flag is read only by the R2 engine (R2Stretcher.cpp is the
// only file in the library that mentions it), and the single-file build compiles threading out
// with `NO_THREADING`. It is passed anyway, because a configuration that would change meaning
// if the engine choice moved is not a configuration.
constexpr auto kStretchOptions = RubberBand::RubberBandStretcher::OptionProcessOffline
                               | RubberBand::RubberBandStretcher::OptionEngineFiner
                               | RubberBand::RubberBandStretcher::OptionThreadingNever
                               | RubberBand::RubberBandStretcher::OptionStretchElastic
                               | RubberBand::RubberBandStretcher::OptionTransientsCrisp
                               | RubberBand::RubberBandStretcher::OptionDetectorCompound
                               | RubberBand::RubberBandStretcher::OptionPhaseLaminar
                               | RubberBand::RubberBandStretcher::OptionWindowStandard
                               | RubberBand::RubberBandStretcher::OptionSmoothingOff
                               | RubberBand::RubberBandStretcher::OptionFormantShifted
                               | RubberBand::RubberBandStretcher::OptionPitchHighSpeed
                               | RubberBand::RubberBandStretcher::OptionChannelsApart;

// ADR 0009 §4's sample-rate conversion, and the only place in the engine a rate is converted.
//
// `juce::LagrangeInterpolator` is a fixed 5-point Lagrange polynomial: no options to pin, no
// runtime CPU dispatch, and `reset()` zeroes the four samples of history it carries, so its
// output is a pure function of the input, the ratio and the JUCE commit §17 pins. `ratio` is
// the asset's rate over the render's — input samples consumed per output sample.
//
// ponytail: no anti-aliasing filter, so downsampling folds everything above the new Nyquist.
// M1's fixture is at the render rate and this path exists so a mismatched asset plays rather
// than being refused (ADR 0009 §4); a band-limited resampler is the upgrade the first time a
// project downsamples something bright.
juce::AudioBuffer<float> resampled (const juce::AudioBuffer<float>& in, double ratio, int frames)
{
    juce::AudioBuffer<float> out (in.getNumChannels(), frames);
    out.clear();
    juce::LagrangeInterpolator interpolator;
    for (int channel = 0; channel < in.getNumChannels(); ++channel)
    {
        interpolator.reset();
        // The overload that is told how much input exists: the five-point kernel reads past
        // the sample it is producing, and the last output frame is at the end of the asset.
        interpolator.process (ratio, in.getReadPointer (channel), out.getWritePointer (channel),
                              frames, in.getNumSamples(), 0);
    }
    return out;
}

// The asset stretched to fill the clip, pitch unchanged (ADR 0011 §3). `frames` is the clip's
// length in samples, and the ratio is derived from it here rather than carried in the plan
// because `compile` reads no file and so cannot know the asset's duration (ADR 0007 §4).
//
// The block size is Rubber Band's own: `getSamplesRequired()` is the documented mode for a
// caller with no external constraint, and taking it means there is no block-size constant of
// ours for a golden to depend on. Draining while `available()` is positive and stopping at
// zero is what the library's own command line does when threading is off, which here it always
// is (main/main.cpp, the "completing" loop).
juce::AudioBuffer<float> stretched (const juce::AudioBuffer<float>& in, double rate, int frames)
{
    const auto channels = (size_t) in.getNumChannels();
    const auto total = in.getNumSamples();

    RubberBand::RubberBandStretcher stretcher ((size_t) rate, channels, kStretchOptions,
                                               frames / (double) total, 1.0);
    stretcher.setExpectedInputDuration ((size_t) total);

    std::vector<const float*> input (channels);
    for (size_t channel = 0; channel < channels; ++channel)
        input[channel] = in.getReadPointer ((int) channel);
    // Any number of samples at a time is allowed in the study pass, and there is one clip's
    // worth of them, so it is one call.
    stretcher.study (input.data(), (size_t) total, true);

    juce::AudioBuffer<float> out (in.getNumChannels(), frames);
    out.clear();
    juce::AudioBuffer<float> block (in.getNumChannels(), 1);
    std::vector<float*> output (channels);
    int written = 0;

    const auto drain = [&]
    {
        for (int available = stretcher.available(); available > 0; available = stretcher.available())
        {
            block.setSize (in.getNumChannels(), available, false, false, true);
            for (size_t channel = 0; channel < channels; ++channel)
                output[channel] = block.getWritePointer ((int) channel);
            const auto got = (int) stretcher.retrieve (output.data(), (size_t) available);
            // Anything past the clip's end is dropped rather than kept: offline mode
            // compensates its own delay, so an overrun is the ratio's rounding and not a tail
            // that belongs anywhere. It must still be retrieved, or the stretcher stalls.
            const auto room = std::min (got, frames - written);
            for (int channel = 0; channel < in.getNumChannels() && room > 0; ++channel)
                out.copyFrom (channel, written, block, channel, 0, room);
            written = std::min (frames, written + got);
        }
    };

    for (int read = 0; read < total;)
    {
        const auto want = std::clamp ((int) stretcher.getSamplesRequired(), 1, total - read);
        for (size_t channel = 0; channel < channels; ++channel)
            input[channel] = in.getReadPointer ((int) channel, read);
        read += want;
        stretcher.process (input.data(), (size_t) want, read >= total);
        drain();
    }
    drain();
    return out;
}

// ADR 0011 §2's formula, and it is **ours** rather than Tracktion's fade shapes for the reason
// ADR 0002 §8 gives for the automation curves: a shape the renderer chose is one §11's
// bit-exactness is at the mercy of, and one a second renderer cannot reimplement.
//
//   g_in(n)  = 1 if f_in == 0, else min(1, n / f_in)
//   g_out(n) = 1 if f_out == 0, else min(1, (N - n) / f_out)
//   out(n)   = in(n) * 10^(gain_db / 20) * g_in(n) * g_out(n)
//
// Three properties are the formula's, not this code's, and they are what remove the special
// cases: `g_in(0)` is zero, which is the click a fade exists to remove; the two ramps multiply,
// so fades that overlap need no clamping rule; and both are linear in amplitude, because a
// dB-linear ramp never reaches silence.
//
// The three factors are combined in double and applied to the sample once. Float multiplication
// is not associative, so the order is a decision and not a formatting choice — this is the one
// the formula's own left-to-right reading gives once the constants are folded.
void shape (juce::AudioBuffer<float>& buffer, double gainDb, int fadeIn, int fadeOut)
{
    const auto frames = buffer.getNumSamples();
    const auto gain = std::pow (10.0, gainDb / 20.0);

    for (int n = 0; n < frames; ++n)
    {
        auto g = gain;
        if (fadeIn > 0)
            g *= std::min (1.0, n / (double) fadeIn);
        if (fadeOut > 0)
            g *= std::min (1.0, (frames - n) / (double) fadeOut);
        for (int channel = 0; channel < buffer.getNumChannels(); ++channel)
            buffer.getWritePointer (channel)[n] *= (float) g;
    }
}

// Builds the edit the plan describes.
//
// Everything it reads is already resolved (ADR 0007 §1): mixer order, chain order, loop
// expansion, solo and mute, and which device each automation lane targets. Nothing here sorts,
// searches for a track, or decides what sounds — `core/src/render.rs` did all of it once, and
// a second opinion in C++ is the second implementation ADR 0007 exists to prevent.
class Builder {
public:
    Builder (te::Edit& e, Plugins& p, const RenderPlan& plan_, te::TimePosition end_)
        : edit (e), plugins (p), plan (plan_), end (end_),
          rate (plan_.target().sample_rate()),
          scratch (e.engine.getPropertyStorage().getAppCacheFolder())
    {
        for (const auto& event : plan.tempo())
            tempoTicks.push_back (event.tick());
    }

    // Every track in the plan, onto the audio tracks the edit was created with, in order.
    std::string build()
    {
        const auto audio = te::getAudioTracks (edit);
        if (audio.size() < plan.tracks_size())
            // The edit was created with one audio track per plan track; a shortfall would index
            // past the end of the array, which juce::Array answers with a null rather than an
            // exception.
            return "the edit has " + std::to_string (audio.size()) + " audio tracks for the plan's "
                   + std::to_string (plan.tracks_size());
        for (int i = 0; i < plan.tracks_size(); ++i)
            if (const auto why = track (*audio[i], plan.tracks (i)); ! why.empty())
                return why;

        if (! plan.has_master())
            return {};
        const auto& master = plan.master();
        if (master.clips_size() > 0)
            // §4.4 gives the master no instrument, and Tracktion's master track holds no clips
            // either. The model permits a clip whose track_id names the master; the validator
            // does not yet refuse one, so this is where it stops, loudly.
            return "the plan puts a clip on the master track, which has no instrument to voice it";
        int slot = 0;
        for (const auto& effect : master.effects())
        {
            auto plugin = device (effect.effect().ref(), effect.effect().state(),
                                  effect.effect().params(), effect.lanes());
            if (! plugin)
                return plugin.error();
            edit.getMasterPluginList().insertPlugin (*plugin, slot++, nullptr);
        }
        if (auto fader = edit.getMasterVolumePlugin(); fader != nullptr && master.has_mix())
            mix (*fader, master.mix());
        return {};
    }

private:
    std::string track (te::AudioTrack& target, const PlanTrack& source)
    {
        // Chain order as the plan holds it: the instrument, then the effects, then the track's
        // own volume and pan, which Tracktion already put at the end of this list. A fader
        // after the chain is what a mixer strip is.
        int slot = 0;
        if (source.has_instrument())
        {
            const auto& held = source.instrument().instrument();
            auto plugin = device (held.ref(), held.state(), held.params(), source.instrument().lanes(),
                                  source.instrument().sfz_path());
            if (! plugin)
                return plugin.error();
            target.pluginList.insertPlugin (*plugin, slot++, nullptr);
        }
        for (const auto& effect : source.effects())
        {
            auto plugin = device (effect.effect().ref(), effect.effect().state(),
                                  effect.effect().params(), effect.lanes());
            if (! plugin)
                return plugin.error();
            target.pluginList.insertPlugin (*plugin, slot++, nullptr);
        }
        if (auto* fader = target.getVolumePlugin(); fader != nullptr && source.has_mix())
            mix (*fader, source.mix());
        return clips (target, source);
    }

    // `mute` and `solo` were applied by compile and cross false (ADR 0007 §1), so a mix is a
    // level and a position and nothing else.
    void mix (te::VolumeAndPanPlugin& fader, const escribass::song::v1::Mix& source)
    {
        fader.setVolumeDb ((float) source.gain_db());
        fader.setPan ((float) source.pan());
    }

    // One device on a chain: the plugin the manifest names, then its state, then its
    // parameters, then the automation that targets it.
    //
    // **State first, parameters second, and that order is the whole of it** (trap 6). A VST3
    // state is the entire plugin, every parameter included, so a parameter set before it is
    // silently overwritten by it — and only sometimes, because it shows up exactly where the
    // state and the parameter disagree, which is the case the parameter was written for. The
    // trap's other half, a value that ramps from a reused instance's previous one through the
    // plugin's own smoothing, is what ADR 0008 §2's fresh process removes; nothing here has to.
    //
    // ponytail: nothing in M1 writes an `Instrument.state` — fixtures are factory defaults plus
    // `params` (docs/plan.md, "Deferred again") — so no fixture exercises this path yet. It is
    // written now because the order is not discoverable afterwards: the first state to arrive
    // would simply render wrong.
    tl::expected<te::Plugin::Ptr, std::string> device (const escribass::song::v1::DeviceRef& ref,
                                                       const std::string& state,
                                                       const google::protobuf::Map<std::string, double>& params,
                                                       const google::protobuf::RepeatedPtrField<PlanLane>& lanes,
                                                       const std::string& sfz = {})
    {
        // Two kinds reach here. A plugin names itself; a sampler names an SFZ, and compile
        // resolved that to the path beside it (ADR 0007 §2, extended) — which plugin plays it is
        // this engine's decision, above. compile refuses every other kind (ADR 0007 §6), and a
        // sampler whose hash is not in assets/ never becomes a plan at all.
        std::string id;
        if (ref.has_plugin())
            id = ref.plugin().plugin_id();
        else if (ref.has_sampler() && juce::File::isAbsolutePath (sfz))
            id = kSamplerPluginId;
        else
            return tl::unexpected (std::string ("a device in the plan is neither a plugin nor a"
                                                " sampler with the absolute path of an SFZ"));
        const auto desc = plugins.describe (id);
        if (! desc)
            return tl::unexpected (desc.error());

        auto plugin = edit.getPluginCache().createNewPlugin (te::ExternalPlugin::xmlTypeName, *desc);
        auto* external = dynamic_cast<te::ExternalPlugin*> (plugin.get());
        if (external == nullptr)
            return tl::unexpected ("'" + id + "' did not become a hosted plugin");
        if (external->getAudioPluginInstance() == nullptr)
            external->initialiseFully();
        auto* instance = external->getAudioPluginInstance();
        if (instance == nullptr)
            return tl::unexpected ("'" + id + "': " + external->getLoadError().toStdString());

        // Trap 14: `state` is opaque binary and is never text. It reaches the plugin as the
        // bytes the model holds, unparsed and unexamined.
        if (! state.empty())
            instance->setStateInformation (state.data(), (int) state.size());

        // After the state and before the parameters, for the reason the order above is what it
        // is: the SFZ is spliced into whatever state the plugin holds at this point, so a
        // sampler that also carries an `Instrument.state` keeps everything in it except the
        // file it names — which the model already named, by hash.
        if (ref.has_sampler())
            if (const auto why = loadSfz (*instance, juce::File (sfz)); ! why.empty())
                return tl::unexpected ("'" + id + "': " + why);

        // The plugin's own parameter ids, which are what --scan wrote as the manifest's keys
        // and therefore what a `ParamRef.param` and a `params` key match (ADR 0010 §4).
        std::map<std::string, int> byParamId;
        for (int i = 0; i < instance->getParameters().size(); ++i)
            byParamId[instance->getHostedParameter (i)->getParameterID().toStdString()] = i;

        for (const auto& [param, value] : params)
        {
            const auto found = byParamId.find (param);
            if (found == byParamId.end())
                return tl::unexpected ("'" + id + "' declares no parameter '" + param + "'");
            // Written to the plugin's own parameter rather than through Tracktion's
            // AutomatableParameter, and that is the second half of trap 6. Tracktion caches the
            // value it read when the plugin was created and refreshes that cache from an
            // asynchronous callback, so a state applied since leaves the cache stale — and
            // AutomatableParameter::setParameterValue returns without writing when the cached
            // value already equals the one being set. A parameter would then be dropped exactly
            // when the stale value happened to match, which is a silence with no error in it.
            instance->getHostedParameter (found->second)->setValue (normalised (value));
        }

        for (const auto& lane : lanes)
        {
            const auto found = byParamId.find (lane.param());
            // Tracktion keys a hosted VST3's automation by the plugin's parameter *index*, not
            // by its ParamID: buildParameterList() reads an id only from an
            // AudioProcessorParameterWithID, and a hosted VST3's parameters are
            // HostedAudioProcessorParameters, so it falls back to juce::String(i)
            // (tracktion_ExternalPlugin.cpp). The manifest's key is translated through the
            // index above rather than assumed to be the same string.
            auto automatable = found == byParamId.end()
                                 ? te::AutomatableParameter::Ptr()
                                 : external->getAutomatableParameterByID (juce::String (found->second));
            if (automatable == nullptr)
                // A parameter the plugin does not declare, or declares and does not automate.
                // Both are PR 9's `param_unknown`; here neither can move a sample.
                return tl::unexpected ("'" + id + "' has no automatable parameter '" + lane.param() + "'");
            place (*automatable, lane);
        }

        return plugin;
    }

    // Where a model tick falls on the timeline. The tempo map was put on the edit's own
    // sequence before this builder ran, so one owner converts tick to time and there is one
    // rounding site rather than two (ADR 0007 §3). Widened to 64 bits because an absolute tick
    // is summed before it is converted (ADR 0002 §1).
    te::TimePosition at (juce::int64 tick)
    {
        return edit.tempoSequence.toTime (te::BeatPosition::fromBeats (tick / (double) kPpq));
    }

    // ADR 0002 §8's two curves, and they are **ours** rather than Tracktion's. `curve` on a
    // point is the shape of the segment from that point to the next; before the first point the
    // value is the first point's, and after the last it is the last's.
    //
    //   LINEAR  the value moves in a straight line to the next point, **in ticks** — §4.2's one
    //           musical time, and the axis a point is stored on.
    //   HOLD    the value stays at this point's until the next point, where it steps.
    //
    // Tracktion's parameter curve is in seconds (tracktion_AutomatableParameter.cpp, where
    // AutomationCurveSource builds it with TimeBase::time), and a straight segment there is a
    // straight line in seconds. Within one tempo the two are the same line; across a tempo
    // change they are not, so a LINEAR segment is split at every tempo event inside it, at the
    // value our formula gives for that tick. Each piece then lies in one constant tempo, where
    // the two definitions agree, and the result is our formula rather than an approximation to
    // it. HOLD is a second point at the segment's end carrying the segment's own value:
    // Tracktion draws a straight line between two equal values, which is the hold, and the next
    // point at that same instant is the step.
    void place (te::AutomatableParameter& param, const PlanLane& lane)
    {
        auto& curve = param.getCurve();

        for (int i = 0; i < lane.points_size(); ++i)
        {
            const auto& point = lane.points (i);
            curve.addPoint (at (point.tick()), normalised (point.value()), 0.0f, nullptr);
            if (i + 1 == lane.points_size())
                break;

            const auto& next = lane.points (i + 1);
            if (point.curve() == escribass::song::v1::CURVE_HOLD)
            {
                curve.addPoint (at (next.tick()), normalised (point.value()), 0.0f, nullptr);
                continue;
            }
            for (const auto tick : tempoTicks)
                if (tick > point.tick() && tick < next.tick())
                {
                    const auto through = (tick - point.tick()) / (double) (next.tick() - point.tick());
                    curve.addPoint (at (tick),
                                    normalised (point.value() + through * (next.value() - point.value())),
                                    0.0f, nullptr);
                }
        }

        // Tracktion builds a curve's read iterator on a 10 ms timer (the deferredUpdateTimer in
        // AutomationCurveSource). A render that started before it fired would read no automation
        // at all and one that started after would read all of it, which is a render that depends
        // on a clock — CLAUDE.md #3, arriving as an intermittently silent lane.
        param.updateStream();
    }

    // What a track plays: one MIDI clip per track spanning the render with every note at its
    // absolute beat, and one Tracktion audio clip per audio clip.
    //
    // Notes need no clip boundary of their own. `compile` already unrolled every loop and cut
    // every note to the clip holding it (core/src/render.rs), so a plan clip's edges carry
    // nothing left for the engine to reproduce — and one clip is one fewer Tracktion behaviour
    // (loop flags, content offset, clip length, per-clip mute) standing between a note and its
    // MIDI. An asset is the other case: it has a length, a gain and fades of its own, and
    // `audio` below is where those become samples.
    std::string clips (te::AudioTrack& target, const PlanTrack& source)
    {
        te::MidiClip::Ptr midi;
        for (const auto& clip : source.clips())
        {
            if (clip.has_audio())
            {
                if (const auto why = audio (target, clip); ! why.empty())
                    return why;
                continue;
            }
            if (! clip.has_notes())
                // compile's `oneof_unset` arm produces no plan clip at all, so this is unreachable
                // from a valid song and is here because the alternative is a silent skip.
                return "a clip in the plan carries neither notes nor audio";
            for (const auto& note : clip.notes().notes())
            {
                if (midi == nullptr)
                {
                    midi = target.insertMIDIClip (te::TimeRange (te::TimePosition(), end), nullptr);
                    if (midi == nullptr)
                        return "Tracktion would not put a MIDI clip on a track";
                }
                // §4.4 puts the note inside its clip and compile bounded the clip's end, so this
                // sum fits an int32; widened because ADR 0002 §1 says absolute tick arithmetic is
                // done in 64 bits.
                const auto start = (juce::int64) clip.start_tick() + note.start_tick();
                midi->getSequence().addNote (note.pitch(),
                                             te::BeatPosition::fromBeats (start / (double) kPpq),
                                             te::BeatDuration::fromBeats (note.length_ticks() / (double) kPpq),
                                             note.velocity(), 0, nullptr);
            }
        }
        return {};
    }

    // One audio clip: the asset, at the clip's position, at its gain, with its fades, and
    // stretched to fill the clip when it asks (ADR 0011 §2 and §3).
    //
    // Everything the model says about the clip is applied here, into a buffer, and what
    // Tracktion is handed is a file of exactly the clip's length with no gain, no fade and no
    // stretch of its own. That follows from one decision, ADR 0011 §2's: the fade shape is ours,
    // so it cannot be a property set on a Tracktion clip. Once the samples have to be touched
    // for that, the stretch and the rate conversion come along — and every surface that
    // computes audio is then in one place with one determinism note each (ADR 0009 §4), rather
    // than split across a question about what Tracktion did in between.
    //
    // ponytail: the whole asset is read into memory and one WAV per clip is written to the
    // process's scratch directory, which `TempDir` deletes on the way out. The ceiling is an
    // asset that does not fit in RAM, or a plan with hundreds of clips; the upgrade is a
    // streaming pass, and neither is a shape M1 renders.
    std::string audio (te::AudioTrack& target, const PlanClip& clip)
    {
        const auto& source = clip.audio();
        const auto& held = source.clip();
        const auto first = (juce::int64) clip.start_tick();
        const auto last = first + clip.length_ticks();

        if (held.fade_in_ticks() < 0 || held.fade_out_ticks() < 0)
            // A fade is a length. The formula would read a negative one as a ramp through zero
            // into an inverted signal, which is not something anyone asked for.
            return "an audio clip has a negative fade length";

        const auto start = te::toSamples (at (first), rate);
        const auto frames = (int) (te::toSamples (at (last), rate) - start);
        if (frames <= 0)
            return "an audio clip of " + std::to_string (clip.length_ticks())
                   + " ticks is under one sample long at this tempo and rate";

        juce::AudioFormatManager formats;
        formats.registerBasicFormats();
        const juce::File file (source.path());
        // The **stream** overload, not the `File` one: `createReaderFor (const File&)` asks each
        // format `canHandleFile`, which compares the file's *extension*, and an asset in
        // `assets/` is named by its own SHA-256 and has none (ADR 0007 §2, §10). Every audio
        // clip in a real project would otherwise fail here, which is what M1 PR 11's audio
        // golden found: PR 8's check handed the engine a path of its own ending in `.wav`, so
        // nothing had ever opened a content-addressed asset. The stream overload asks each
        // format to read the header instead, which is the only honest question about a file
        // whose name is a hash.
        const std::unique_ptr<juce::AudioFormatReader> reader (
            formats.createReaderFor (file.createInputStream()));
        if (reader == nullptr)
            // core resolved this path from the clip's `asset_hash` and refused a hash `assets/`
            // does not hold (core/src/render.rs), so what is left here is a file that is not
            // audio this build reads, or one that has gone since the plan was compiled.
            return "'" + source.path() + "' is not an audio file this engine reads";
        if (reader->numChannels < 1 || reader->numChannels > 2)
            return "'" + source.path() + "' has " + std::to_string (reader->numChannels)
                   + " channels; M1 renders a stereo master from mono and stereo assets";
        if (reader->lengthInSamples <= 0
            || reader->lengthInSamples > (juce::int64) std::numeric_limits<int>::max())
            return "'" + source.path() + "' holds " + std::to_string (reader->lengthInSamples)
                   + " sample frames, which is not something to play";
        if (! (reader->sampleRate > 0.0))
            return "'" + source.path() + "' declares a sample rate of "
                   + std::to_string (reader->sampleRate);

        juce::AudioBuffer<float> buffer ((int) reader->numChannels, (int) reader->lengthInSamples);
        if (! reader->read (&buffer, 0, buffer.getNumSamples(), 0, true, true))
            return "could not read '" + source.path() + "'";

        // ADR 0009 §4: an asset that does not match the render's rate is **converted, not
        // stretched**, and that happens whether or not the clip asks for a stretch.
        if (reader->sampleRate != rate)
            // At least one frame: an asset shorter than one frame at the render's rate would
            // otherwise become an empty buffer and a clip of silence with nothing to say so.
            buffer = resampled (buffer, reader->sampleRate / rate,
                                std::max (1, (int) std::llround (buffer.getNumSamples() * rate
                                                                 / reader->sampleRate)));

        // ADR 0011 §3: a stretched clip fills its own musical length. `compile` unrolled a loop
        // into one plan clip per iteration and refused one whose last iteration is short, so
        // the length here is the unit that repeats and never a truncated one (render.proto,
        // PlanAudio; ADR 0007 §6 as amended).
        if (held.time_stretch() && buffer.getNumSamples() != frames)
            buffer = stretched (buffer, rate, frames);

        // Exactly the clip, which is what makes the fades below run over the clip's length and
        // not the asset's: a shorter asset ends in silence, a longer one stops, and both are
        // what ADR 0011 §3 says an unstretched clip does.
        buffer.setSize (buffer.getNumChannels(), frames, true, true, false);
        shape (buffer, held.gain_db(),
               (int) (te::toSamples (at (first + held.fade_in_ticks()), rate) - start),
               (int) (te::toSamples (at (last), rate)
                      - te::toSamples (at (last - held.fade_out_ticks()), rate)));

        const auto processed = scratch.getChildFile ("clip" + juce::String (++placed) + ".wav");
        {
            juce::WavAudioFormat wav;
            std::unique_ptr<juce::OutputStream> out (processed.createOutputStream());
            if (out == nullptr)
                return "could not open " + processed.getFullPathName().toStdString() + " to write";
            // 32-bit float, said rather than inherited: `gain_db` has no upper bound in §4.4, so
            // an integer intermediate would clip a hot clip here — before the mix, the chain and
            // the master, which are where a render's level is actually decided.
            const std::unique_ptr<juce::AudioFormatWriter> writer (
                wav.createWriterFor (out, juce::AudioFormatWriter::Options {}
                                              .withSampleRate (rate)
                                              .withNumChannels (buffer.getNumChannels())
                                              .withBitsPerSample (32)
                                              .withSampleFormat (juce::AudioFormatWriterOptions::SampleFormat::floatingPoint)));
            if (writer == nullptr || ! writer->writeFromAudioSampleBuffer (buffer, 0, frames))
                return "could not write " + processed.getFullPathName().toStdString();
        }
        // The scope above closes the writer, and closing it is what finishes the file: a WAV's
        // `data` size is written by the writer's destructor, so a file handed to Tracktion
        // before that reads back as zero frames long — a clip of silence with nothing to say
        // it went wrong.

        // The file is at the render's rate and holds exactly the clip's frames, so nothing the
        // clip could carry is set: no stretch, no gain, no fade, no loop, no offset. The clip
        // is a placement.
        //
        // Tracktion still reads it through a `juce::LagrangeInterpolator` — `WaveNode::
        // processSection` does that for every wave clip whatever the ratio. At the 1:1 the line
        // above guarantees, the kernel is a delta and the samples come back bit-identical
        // (measured: zero difference over a rendered clip), but the interpolator's two-sample
        // base latency is not compensated, so the clip sounds two samples after its position.
        // Upstream's, deterministic, and written down in ADR 0009 §4 and §8 rather than worked
        // around here, because the workaround would be a compensation coupled to a JUCE
        // internal for 42 microseconds.
        const te::ClipPosition position { te::TimeRange (at (first), at (last)), te::TimeDuration() };
        auto inserted = target.insertWaveClip (processed.getFileNameWithoutExtension(), processed, position, false);
        if (inserted == nullptr)
            return "Tracktion would not put an audio clip on a track";

        // The path is forced absolute, because `insertWaveClip` stores a relative one and this
        // edit cannot resolve it: `SourceFileReference::findFileFromString` resolves a relative
        // source against the project manager's edit file, and a render edit has no project, so
        // the clip ends up pointing at a file that does not exist. That does not fail — a
        // WaveNode whose file is missing never reports itself ready, and NodeRenderContext's
        // `leafNodesReady` loop sleeps on it forever. A hang, not an error, and the engine's own
        // dispatch loop would wait behind it. Absolute is what the plan hands us in the first
        // place (ADR 0008 §1).
        inserted->getSourceFileReference().setToFile (processed, te::SourceFileReference::PathStyle::alwaysAbsolute, false);
        return {};
    }

    te::Edit& edit;
    Plugins& plugins;
    const RenderPlan& plan;
    const te::TimePosition end;
    std::vector<int> tempoTicks;

    // What an audio clip is resolved into samples with: the render's own rate, and the
    // directory `TempDir` deletes on the way out, which is where the processed clips go.
    const double rate;
    const juce::File scratch;
    int placed = 0;
};

int run (const juce::File& manifestFile)
{
    // ADR 0009 §3: FTZ and DAZ, set before any thread exists so the render thread inherits
    // them (Linux copies the FP environment on clone). Tracktion sets them again on that
    // thread in RenderTask::runJob (disableDenormalisedNumberSupport); this is the process's
    // own statement of the same thing, and it covers any thread JUCE starts first.
    _MM_SET_FLUSH_ZERO_MODE (_MM_FLUSH_ZERO_ON);
    _MM_SET_DENORMALS_ZERO_MODE (_MM_DENORMALS_ZERO_ON);

    const auto manifest = juce::JSON::parse (manifestFile);
    if (! manifest.isObject())
        return fail (kBadPlan, "no build manifest at " + manifestFile.getFullPathName().toStdString()
                                   + "; it is written by `cmake --build engine/build --target manifest` (ADR 0010 §4)");

    // One plan, read to end-of-stream (ADR 0008 §1).
    const std::string bytes ((std::istreambuf_iterator<char> (std::cin)), std::istreambuf_iterator<char>());
    RenderPlan plan;
    if (! plan.ParseFromString (bytes))
        return fail (kBadPlan, "stdin is not a RenderPlan (" + std::to_string (bytes.size()) + " bytes)");

    const juce::ScopedJuceInitialiser_GUI juceInit;
    const TempDir scratch;
    te::Engine engine (std::make_unique<Storage> (scratch.dir), std::make_unique<te::UIBehaviour>(), std::make_unique<Behaviour>());
    auto* wav = engine.getAudioFileFormatManager().getWavFormat();

    // A pin, not a default: the pan law is a process-wide static in Tracktion
    // (tracktion_AudioUtilities.cpp) that any module could set, and it decides the two gains a
    // `Mix.pan` becomes. Linear is what Tracktion itself starts at, and it is the only one of
    // the five that leaves a centred track at exactly its `gain_db`; the cost is that a hard
    // pan is +6 dB on the surviving side. Nothing in the model names a law, so this is the
    // engine stating which one it renders, not a choice the document makes.
    te::setDefaultPanLaw (te::PanLawLinear);

    if (const auto why = check (plan, *wav); ! why.empty())
        return fail (kBadPlan, why);
    const auto& target = plan.target();

    // One audio track per plan track, and the master. Master volume at 0 dB so the level is the
    // plan's Mix, not Tracktion's -3 dB default. At least one track even for a plan with none,
    // because an edit's own default is one and a silent extra track sums exactly zero.
    auto edit = te::Edit::createEdit ({ engine,
                                        te::createEmptyEdit (engine),
                                        te::ProjectItemID (1, {}),
                                        te::Edit::forRendering,
                                        nullptr,
                                        te::Edit::getDefaultNumUndoLevels(),
                                        [&engine] { return engine.getPropertyStorage().getAppCacheFolder().getChildFile ("plan.tracktionedit"); },
                                        {},
                                        (juce::uint32) std::max (1, plan.tracks_size()),
                                        0.0f });

    // Automation is read when the transport reads it, and AutomationCurveSource::setPosition
    // returns without moving a parameter when it does not. Stated rather than inherited: the
    // flag is a property of the edit, and a plan's lanes are not optional.
    edit->getAutomationRecordManager().setReadingAutomation (true);

    // The tempo map goes on Tracktion's sequence and Tracktion converts (ADR 0007 §3). A model
    // tempo event holds until the next; on a TempoSetting that is a curve of 1.0, which
    // tracktion_Bezier.h's getBezierEnds turns into "bpm1 up to the next setting, then step".
    // The validator put an event at tick 0, and it replaces the sequence's default setting.
    auto& tempo = edit->tempoSequence;
    tempo.getTempo (0)->set (te::BeatPosition(), plan.tempo (0).bpm(), 1.0f, false);
    for (int i = 1; i < plan.tempo_size(); ++i)
    {
        const auto& event = plan.tempo (i);
        tempo.insertTempo (te::BeatPosition::fromBeats (event.tick() / (double) kPpq), event.bpm(), 1.0f);
    }
    const auto end = tempo.toTime (te::BeatPosition::fromBeats (plan.length_ticks() / (double) kPpq));

    // The tempo map is on the sequence before this, because every tick the builder converts —
    // a note's beat, an automation point's time — is converted through it.
    Plugins plugins (engine, manifest);
    if (const auto why = Builder (*edit, plugins, plan, end).build(); ! why.empty())
        return fail (kBadPlan, why);

    te::Renderer::Parameters params (*edit);
    params.destFile = juce::File (plan.output_path());
    // **The destination is removed first, and this is not tidiness.** Tracktion opens it with
    // `juce::File::createOutputStream()`, which positions at the *end* of an existing file, so a
    // render over one that is already there appends a second, complete RIFF file after the
    // first. The WAV then holds two `data` chunks; every reader takes the first, which is the
    // *previous* render. Measured in M1 PR 11: rendering one plan twice to one path leaves a
    // file of exactly twice the size whose `bext` origination time is still the first run's, and
    // the read-back below hashed that first chunk and reported the old audio as this render's
    // answer. That made every "renders the same twice" check that reused one path vacuous, and
    // it is why this suite gives each render a path of its own as well.
    if (params.destFile.existsAsFile() && ! params.destFile.deleteFile())
        return fail (kRenderFailed, "cannot replace " + plan.output_path());
    params.audioFormat = wav;
    params.sampleRateForAudio = target.sample_rate();
    params.bitDepth = (int) target.bit_depth();
    params.blockSizeForAudio = kBlockSize;
    params.time = te::TimeRange (te::TimePosition(), end);
    // Spike, 2026-09-04: the doc says empty means every track; the code renders nothing.
    params.tracksToDo = te::toBitSet (te::getAllTracks (*edit));
    params.usePlugins = true;
    params.useMasterPlugins = true;
    params.ditheringEnabled = false;
    // Two channels whatever the graph says, and silence is not a failure: a plan of one
    // section and no clips is an outro someone named (render.proto, length_ticks).
    params.canRenderInMono = false;
    params.checkNodesForAudio = false;

    const auto expectedFrames = te::toSamples (params.time.getLength(), params.sampleRateForAudio);

    // ADR 0008 §3: the render is asynchronous. Start it, pump the message loop until it calls
    // back, then let the handle join its thread.
    std::atomic<bool> done { false };
    std::string error;
    auto handle = te::EditRenderer::render (params, [&] (auto outcome)
    {
        if (! outcome)
            error = outcome.error().empty() ? "Tracktion reported a failure without a message" : outcome.error();
        done = true;
    });
    while (! done)
        juce::MessageManager::getInstance()->runDispatchLoopUntil (10);
    handle.reset();

    if (! error.empty())
        return fail (kRenderFailed, "render failed: " + error);
    if (! params.destFile.existsAsFile())
        return fail (kRenderFailed, "render reported success and wrote nothing at " + plan.output_path());

    // What the file holds is checked against what the plan asked, not against what any call
    // returned (ADR 0008 §3), and the hash is of the data chunk alone (ADR 0009 §2).
    juce::FileInputStream in (params.destFile);
    if (! in.openedOk())
        return fail (kBadOutput, "cannot read back " + plan.output_path());
    const auto dataSize = seekToData (in);
    if (! dataSize)
        return fail (kBadOutput, plan.output_path() + ": " + dataSize.error());
    const auto expectedBytes = expectedFrames * 2 * (juce::int64) (target.bit_depth() / 8);
    if (*dataSize != expectedBytes)
        return fail (kBadOutput, "the data chunk holds " + std::to_string (*dataSize) + " bytes; the plan's "
                                     + std::to_string (expectedFrames) + " frames of " + std::to_string (target.bit_depth())
                                     + "-bit stereo are " + std::to_string (expectedBytes));

    RenderResult result;
    result.set_pcm_sha256 (juce::SHA256 (in, *dataSize).toHexString().toStdString());
    for (const auto& commit : escribass::provenance::engineCommits)
        (*result.mutable_commits())[commit.component] = commit.sha;
    for (const auto& commit : escribass::provenance::pluginCommits)
        (*result.mutable_commits())[commit.component] = commit.sha;
    if (! result.SerializeToOstream (&std::cout))
        return fail (kBadOutput, "could not write the RenderResult to stdout");
    std::cout.flush();
    return kOk;
}

}  // namespace

int main (int argc, char** argv)
{
    if (argc == 2 && std::string_view (argv[1]) == "--version")
    {
        for (const auto& commit : escribass::provenance::engineCommits)
            std::printf ("%s %s\n", commit.component, commit.sha);
        for (const auto& commit : escribass::provenance::pluginCommits)
            std::printf ("%s %s\n", commit.component, commit.sha);
        return kOk;
    }
    // The build calls this, not a render (ADR 0010 §4). It is a mode of the engine rather than
    // its own binary for the reason at the top of this file.
    if (argc >= 2 && std::string_view (argv[1]) == "--scan")
    {
        juce::StringArray args;
        for (int i = 2; i < argc; ++i)
            args.add (juce::String::fromUTF8 (argv[i]));
        return scan (args);
    }
    if (argc != 2)
        return fail (kBadPlan, "usage: escribass_engine <manifest.json> < plan.binpb > result.binpb"
                               "\n       escribass_engine [--version | --scan <manifest.json> <component> <plugin.vst3> ...]");
    return run (juce::File::getCurrentWorkingDirectory().getChildFile (juce::String::fromUTF8 (argv[1])));
}
