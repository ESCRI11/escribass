// The render engine (docs/specs.md §8): one RenderPlan in on stdin, one WAV out at the path
// the plan names, one RenderResult out on stdout, exit. A fresh process per render, so nothing
// here outlives a render and nothing is reused (ADR 0008 §2).
//
// stdout carries protobuf bytes and nothing else. Every line this file writes goes to stderr,
// and JUCE's own logger does the same on Linux (juce_SystemStats_linux.cpp: outputDebugString
// is std::cerr), which is why nothing here installs a logger (ADR 0008 §1).
//
// ponytail: this PR renders the plan's length and nothing on it — no track, clip, device or
// lane is read, and every plan is silence of the right length. PR 7 hosts the instruments and
// places notes and automation, PR 8 plays audio clips; the plan's shape is already the one they
// fill in.
//
// `--scan` is the build's second use of this binary (ADR 0010 §4). It opens each bundled VST3
// once, asks it what it is, and writes the manifest. Scanning is not hosting: it instantiates a
// plugin and reads its description and parameter list, and it never puts one in a render graph,
// feeds it a sample or sets a parameter. That half is PR 7's. It lives here rather than in a
// second binary because the provenance below and the VST3 loading above it are the same two
// things a render needs, and a second JUCE console app would compile every JUCE module again
// for the sake of one file.

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

namespace te = tracktion;
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

int run()
{
    // ADR 0009 §3: FTZ and DAZ, set before any thread exists so the render thread inherits
    // them (Linux copies the FP environment on clone). Tracktion sets them again on that
    // thread in RenderTask::runJob (disableDenormalisedNumberSupport); this is the process's
    // own statement of the same thing, and it covers any thread JUCE starts first.
    _MM_SET_FLUSH_ZERO_MODE (_MM_FLUSH_ZERO_ON);
    _MM_SET_DENORMALS_ZERO_MODE (_MM_DENORMALS_ZERO_ON);

    // One plan, read to end-of-stream (ADR 0008 §1).
    const std::string bytes ((std::istreambuf_iterator<char> (std::cin)), std::istreambuf_iterator<char>());
    RenderPlan plan;
    if (! plan.ParseFromString (bytes))
        return fail (kBadPlan, "stdin is not a RenderPlan (" + std::to_string (bytes.size()) + " bytes)");

    const juce::ScopedJuceInitialiser_GUI juceInit;
    const TempDir scratch;
    te::Engine engine (std::make_unique<Storage> (scratch.dir), std::make_unique<te::UIBehaviour>(), std::make_unique<Behaviour>());
    auto* wav = engine.getAudioFileFormatManager().getWavFormat();

    if (const auto why = check (plan, *wav); ! why.empty())
        return fail (kBadPlan, why);
    const auto& target = plan.target();

    // An empty edit: one audio track and the master, both silent. Master volume at 0 dB so the
    // level is the plan's when PR 7 applies Mix, not Tracktion's -3 dB default.
    auto edit = te::Edit::createEdit ({ engine,
                                        te::createEmptyEdit (engine),
                                        te::ProjectItemID (1, {}),
                                        te::Edit::forRendering,
                                        nullptr,
                                        te::Edit::getDefaultNumUndoLevels(),
                                        [&engine] { return engine.getPropertyStorage().getAppCacheFolder().getChildFile ("plan.tracktionedit"); },
                                        {},
                                        1,
                                        0.0f });

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
    if (argc != 1)
        return fail (kBadPlan, "usage: escribass_engine [--version | --scan <manifest.json> <component> <plugin.vst3> ...] < plan.binpb > result.binpb");
    return run();
}
