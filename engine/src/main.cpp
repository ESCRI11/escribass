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
// ponytail: this PR hosts devices, places notes and drives automation. An audio clip is PR 8's
// and is refused rather than dropped, because a render that completes with a clip missing is
// the failure ADR 0010 §3 spends a table refusing: it sounds wrong and says nothing.
//
// `--scan` is the build's second use of this binary (ADR 0010 §4). It opens each bundled VST3
// once, asks it what it is, and writes the manifest. It lives here rather than in a second
// binary because the provenance below and the VST3 loading above it are the same two things a
// render needs, and a second JUCE console app would compile every JUCE module again for the
// sake of one file.

#include <tracktion_engine/tracktion_engine.h>
#include <juce_cryptography/juce_cryptography.h>

#include "provenance.h"
#include "render.pb.h"

#include <pmmintrin.h>
#include <xmmintrin.h>

#include <atomic>
#include <cstdio>
#include <iostream>
#include <iterator>
#include <map>
#include <string>
#include <string_view>
#include <unistd.h>
#include <vector>

namespace te = tracktion;
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

// Builds the edit the plan describes.
//
// Everything it reads is already resolved (ADR 0007 §1): mixer order, chain order, loop
// expansion, solo and mute, and which device each automation lane targets. Nothing here sorts,
// searches for a track, or decides what sounds — `core/src/render.rs` did all of it once, and
// a second opinion in C++ is the second implementation ADR 0007 exists to prevent.
class Builder {
public:
    Builder (te::Edit& e, Plugins& p, const RenderPlan& plan_, te::TimePosition end_)
        : edit (e), plugins (p), plan (plan_), end (end_)
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
            auto plugin = device (held.ref(), held.state(), held.params(), source.instrument().lanes());
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
        return notes (target, source);
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
                                                       const google::protobuf::RepeatedPtrField<PlanLane>& lanes)
    {
        if (! ref.has_plugin())
            // compile refuses the compiled kinds (ADR 0007 §6) and lets a SamplerRef through,
            // because §16 puts the sampler in M1 — but a sampler is an SFZ from assets/ and
            // assets are PR 8's, so nothing hosts one yet. Refused rather than rendered silent.
            return tl::unexpected (std::string ("a device in the plan is not a plugin; a sampler is"
                                                " an SFZ from assets/, which no PR has hosted yet"));
        const auto& id = ref.plugin().plugin_id();
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
        const auto at = [this] (int tick)
        {
            return edit.tempoSequence.toTime (te::BeatPosition::fromBeats (tick / (double) kPpq));
        };

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

    // One MIDI clip per track, spanning the render, with every note at its absolute beat.
    //
    // `compile` already unrolled every loop and cut every note to the clip holding it
    // (core/src/render.rs), so a plan clip's boundary carries nothing left for the engine to
    // reproduce — and one clip is one fewer Tracktion behaviour (loop flags, content offset,
    // clip length, per-clip mute) standing between a note and its MIDI.
    //
    // ponytail: PR 8's audio clips are Tracktion clips, because an asset has a length, a gain
    // and fades of its own that belong to the clip. A note has none of that.
    std::string notes (te::AudioTrack& target, const PlanTrack& source)
    {
        te::MidiClip::Ptr midi;
        for (const auto& clip : source.clips())
        {
            if (clip.has_audio())
                return "the plan has an audio clip; playing one is PR 8's, and a render that "
                       "left it out would sound wrong and say nothing (ADR 0010 §3)";
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

    te::Edit& edit;
    Plugins& plugins;
    const RenderPlan& plan;
    const te::TimePosition end;
    std::vector<int> tempoTicks;
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
