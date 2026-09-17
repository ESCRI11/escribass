// The render engine (docs/specs.md §8): one `Render` call in over gRPC, one WAV out at the path
// the plan names, one RenderResult back, exit. A fresh process per render, so nothing here
// outlives a render and nothing is reused (ADR 0008 §2).
//
// stdout carries the socket address this process serves on and nothing else — one line, printed
// once the server is listening, which is how a caller learns both where to dial and that it may
// (ADR 0013 §3, amended). Every other line this file writes goes to stderr, and JUCE's own
// logger does the same on Linux (juce_SystemStats_linux.cpp: outputDebugString is std::cerr),
// which is why nothing here installs a logger (ADR 0008 §1).
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
#include "render.grpc.pb.h"
#include "render.pb.h"

#include <google/protobuf/util/message_differencer.h>
#include <grpcpp/grpcpp.h>

#include <pmmintrin.h>
#include <xmmintrin.h>

#include <algorithm>
#include <atomic>
#include <cmath>
#include <condition_variable>
#include <cstdio>
#include <cstdlib>
#include <deque>
#include <limits>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <string_view>
#include <thread>
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

// What an `Instrument.ref.sampler` is played by. The model names an SFZ and never a plugin, so
// *which* sampler plays it is the renderer's fact and lives here. §8 bundles sfizz; what pins
// it is this binary's own sfizz_ui commit, which every RenderResult reports (ADR 0008 §5) and
// which `--scan` now writes into the manifest under `sampler`, so a project that references a
// sampler pins that build the way a project that references a plugin does (ADR 0010 §1,
// amended 2026-09-07 in PR 13). Declared up here rather than beside the sampler code below
// because `scan` is above it and needs the same one fact.
constexpr const char* kSamplerPluginId = "SFZTools/sfizz";

// Failure is an exit code (ADR 0008 §1). By the time a plan is here every caller-fixable
// failure was refused upstream, so a plan this engine cannot take is an operator error like
// the other two, and the code only says which stage the render died in.
enum Exit : int {
    kOk = 0,
    kBadPlan = 2,       // stdin was not a plan, or names something no valid song produces
    kRenderFailed = 3,  // Tracktion reported an error, or produced no file
    kBadOutput = 4,     // the file is not the WAV the plan asked for
    kScanFailed = 5,    // --scan: a bundled plugin did not open, or said nothing about itself
    kNoDevice = 6,      // --preview: no audio output device opened, so there is nothing to play on
};

// Every line under this binary's name is one of these, and only these: the reason a run did not
// happen. The window shows that line first and the rest of stderr on hover (app/src/App.tsx).
int fail (Exit code, const std::string& why)
{
    std::fprintf (stderr, "escribass_engine: %s\n", why.c_str());
    return code;
}

// **Headless for an export, and a device for a preview — by mode, and nothing else differs**
// (ADR 0013 §3). An export is what M1's spike measured (2026-09-04): no device manager, no
// system audio devices, no input, and it stays exactly that, because a render that opened a
// device would be a render whose process state depends on the machine's sound server. A
// preview is the one process that wants the device, so it alone asks Tracktion to open the
// system's default output when the engine is constructed. Input is never opened in either:
// §1 puts recording out of v1, and an input device is a second clock nothing here reads.
//
// getNumberOfCPUsToUseForAudio is the single-thread pin of ADR 0009 §3: Tracktion hands the
// render's node player this minus one worker threads (tracktion_NodeRenderContext.cpp), and
// zero workers means every block is processed on the render thread in graph order. A preview
// keeps it, which is not a determinism claim — nothing about a preview is bit-exact — but the
// same graph order for both is one fewer difference to explain between them (trap 3).
class Behaviour : public te::EngineBehaviour {
public:
    explicit Behaviour (bool live_) : live (live_) {}

private:
    bool autoInitialiseDeviceManager() override { return live; }
    bool addSystemAudioIODeviceTypes() override { return live; }
    bool shouldOpenAudioInputByDefault() override { return false; }
    int getNumberOfCPUsToUseForAudio() override { return 1; }

    const bool live;
};

// Tracktion keeps a settings file and a temp directory under the user's application data
// directory. A render process must leave nothing behind and start from nothing (ADR 0008
// §2), so both live in a directory this process creates and deletes. The deletion is a
// separate guard that outlives the Engine: PropertyStorage's own destructor saves the
// settings file, so a directory deleted any earlier comes back.
//
// **Unique by construction, not by pid** (M1 PR 13). The name was `escribass_engine.<pid>`,
// which is neither: pids are reused, this directory leaked four times out of four when a
// render was `SIGKILL`ed, and `/tmp` is world-writable, so any local process could create the
// path in advance. Both roads led to the same place — a *pre-existing* `clipN.wav`, which
// `createOutputStream` opens at end-of-file rather than truncating, so the clip is appended
// after whatever was there and `createReaderFor` takes the first, stale RIFF payload. One plan
// then rendered three different hashes depending only on what was in the directory, exit 0 and
// stderr empty each time; the length check downstream cannot see it, because the placement and
// the render length are still the plan's and only the samples are wrong.
//
// `mkdtemp` creates the directory atomically at 0700 with a name nobody can predict, which
// answers both. Its entropy names a filesystem path and reaches no sample: nothing in the
// output carries it, the clip inside is named `clip1.wav` either way, and the edit is never
// persisted — so CLAUDE.md #3 is not in play here (`docs/specs.md` §8).
struct TempDir {
    TempDir()
    {
        auto pattern = juce::File::getSpecialLocation (juce::File::tempDirectory)
                           .getChildFile ("escribass_engine.XXXXXX")
                           .getFullPathName()
                           .toStdString();
        std::vector<char> name (pattern.begin(), pattern.end());
        name.push_back ('\0');
        if (mkdtemp (name.data()) != nullptr)
            dir = juce::File (juce::String::fromUTF8 (name.data()));
    }

    ~TempDir()
    {
        if (dir != juce::File())
            dir.deleteRecursively();
    }

    juce::File dir;
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

// Every float and double anywhere in a message, by reflection, refusing the first that is not
// finite (M1 PR 13).
//
// One walk rather than a guard per reader. `normalised()` gates every `params` value and every
// `AutomationPoint.value` through `juce::jlimit`, which returns its argument when both
// comparisons are false — so NaN went straight through it and on into a plugin; `gain_db` is
// read somewhere else entirely and had no gate at all, and a NaN there renders the clip as
// pure silence, exit 0, with a hash identical to an empty plan's. Reflection is what makes this
// one place instead of a list: a float added to the plan is covered the day it is added, which
// a hand-written field list is not.
std::string nonFinite (const google::protobuf::Message& message, const std::string& path)
{
    namespace pb = google::protobuf;
    const auto* descriptor = message.GetDescriptor();
    const auto* reflection = message.GetReflection();

    for (int f = 0; f < descriptor->field_count(); ++f)
    {
        const auto* field = descriptor->field (f);
        const auto repeated = field->is_repeated();
        const auto type = field->cpp_type();
        if (type != pb::FieldDescriptor::CPPTYPE_DOUBLE && type != pb::FieldDescriptor::CPPTYPE_FLOAT
            && type != pb::FieldDescriptor::CPPTYPE_MESSAGE)
            continue;

        const auto count = repeated ? reflection->FieldSize (message, field)
                         : (type == pb::FieldDescriptor::CPPTYPE_MESSAGE
                                ? (reflection->HasField (message, field) ? 1 : 0)
                                : 1);
        for (int n = 0; n < count; ++n)
        {
            const auto here = path + "." + field->name() + (repeated ? "[" + std::to_string (n) + "]" : "");
            if (type == pb::FieldDescriptor::CPPTYPE_MESSAGE)
            {
                const auto& child = repeated ? reflection->GetRepeatedMessage (message, field, n)
                                             : reflection->GetMessage (message, field);
                if (auto why = nonFinite (child, here); ! why.empty())
                    return why;
                continue;
            }
            const double value =
                type == pb::FieldDescriptor::CPPTYPE_DOUBLE
                    ? (repeated ? reflection->GetRepeatedDouble (message, field, n)
                                : reflection->GetDouble (message, field))
                    : (double) (repeated ? reflection->GetRepeatedFloat (message, field, n)
                                         : reflection->GetFloat (message, field));
            if (! std::isfinite (value))
                return here + " is " + std::to_string (value) + ", which is not a number to render";
        }
    }
    return {};
}

// Refuses what no valid song compiles to. The validator and compile already refused every
// caller-fixable shape (ADR 0008 §1); this is the trust boundary, not a second validator.
//
// Shared by both modes, since a preview plays the same plan an export writes (ADR 0013 §2).
// What only a file needs — somewhere absolute to write it, and something to write — is
// `render`'s own, below: a preview has no `output_path`, and a plan of no length is a
// transport that reaches its end at once rather than a file with nothing in it.
std::string check (const RenderPlan& plan, juce::AudioFormat& wav)
{
    if (auto why = nonFinite (plan, "plan"); ! why.empty())
        return why;
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
    if (plan.length_ticks() < 0)
        return "length_ticks is " + std::to_string (plan.length_ticks()) + ", which is not a length";
    if (plan.tempo_size() == 0 || plan.tempo (0).tick() != 0)
        return "the tempo map has no event at tick 0 (§4.4)";
    for (const auto& event : plan.tempo())
        if (! (event.bpm() > 0.0))
            return "a tempo of " + std::to_string (event.bpm()) + " at tick " + std::to_string (event.tick());
    // A ceiling on the render, which `length_ticks > 0` is not (M1 PR 13). It is an `int32`, so
    // the largest plan a caller can hand over is 2^31-1 ticks, which at 20 bpm is a twelve-day
    // render and a file of a couple of hundred gigabytes with nothing to refuse it. The bound
    // is the one the code already needs rather than a taste: every frame count downstream — a
    // clip's buffer, the read-back's arithmetic — is an `int`. Taken against the *slowest*
    // tempo in the map, which is a true upper bound on the duration wherever the changes
    // actually fall; at 48 kHz it lands a little over twelve hours.
    auto slowest = plan.tempo (0).bpm();
    for (const auto& event : plan.tempo())
        slowest = std::min (slowest, event.bpm());
    const auto atMostFrames =
        (plan.length_ticks() / (double) kPpq) * (60.0 / slowest) * (double) target.sample_rate();
    if (atMostFrames > (double) std::numeric_limits<int>::max())
        return "length_ticks is " + std::to_string (plan.length_ticks()) + ", which at "
               + std::to_string (slowest) + " bpm is up to " + std::to_string ((juce::int64) atMostFrames)
               + " sample frames; this engine renders what fits an int";
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
    // Which of those classes plays a `SamplerRef` (M1 PR 13). The id is compiled in below and
    // was nowhere else, so a sampler-only project pinned *nothing* about the sampler: the SFZ's
    // hash pins the patch, not the build that plays it, and `lock_mismatch` had no entry to
    // fire on when sfizz moved — §11's [MUST] unsatisfied for one whole device kind. Stated by
    // the engine rather than known by `core`, because it is a fact about this binary and the
    // manifest is already where this binary says what it can host (ADR 0010 §4, CLAUDE.md #6).
    root->setProperty ("sampler", juce::String (kSamplerPluginId));

    const juce::File out (args[0]);
    if (! out.replaceWithText (juce::JSON::toString (juce::var (root), false) + "\n"))
        return fail (kScanFailed, "could not write the manifest to " + args[0].toStdString());
    return kOk;
}

// -----------------------------------------------------------------------------------------
// Hosting the plan (PR 7)
// -----------------------------------------------------------------------------------------

// A parameter value of a **device** in a plan — `Instrument.params`, `Effect.params` and every
// `AutomationPoint.value` under a device — is the plugin's own **normalised** value. That is
// the only domain a VST3 offers a host: the format's `ParamValue` is 0..1 and JUCE hands it
// through unchanged, which is the same fact that leaves ADR 0010 §4's manifest keying a
// parameter by its opaque `ParamID`. Out of range is clamped rather than refused, because
// Tracktion's own parameter range clamps it either way and a value that means nothing is PR 9's
// `param_unknown` — a caller error, which by ADR 0008 §1 is not one the engine is left to
// discover.
//
// A **mix** lane is the exception, and the three functions below it (ADR 0015 §2): its points
// are the model's own units, and each has to be carried into whatever domain Tracktion's fader
// keeps that parameter in.
float normalised (double value)
{
    return (float) juce::jlimit (0.0, 1.0, value);
}

// `Mix.pan` and Tracktion's pan parameter are the same number in the same range, so a pan lane
// needs no conversion at all — and, `LINEAR` being straight in the value, no subdivision below.
//
// What the **pan law** does happens after this and to something else: it turns one position
// into two channel gains, once per block, at the `PanLawLinear` §8 pins. Under that law a
// straight line in `pan` is a straight line in each channel's linear gain (`g·(1∓pan)`), and a
// hard pan is +6 dB on the surviving side — which is the law's property, already recorded, and
// not a shape this curve has to carry.
float panPosition (double pan)
{
    // Not Tracktion's own `setPan`, which snaps |pan| <= 0.005 to centre. That dead zone is for
    // a mouse on a knob; a lane says what it says.
    return (float) juce::jlimit (-1.0, 1.0, pan);
}

// **Tracktion's fader parameter is not decibels.** Its value is a *slider position*,
// `exp((dB - 6) / 20)` (`decibelsToVolumeFaderPosition`, tracktion_AudioUtilities.cpp), and the
// header says so: "NB the units used here are slider position". So the model's `gain_db`
// (ADR 0015 §2) is carried across here, and — because the curve interpolates linearly in
// *position* — a straight line in decibels is an exponential in the domain being interpolated.
// Two endpoints alone would render a ramp from -60 dB to 0 dB passing through -12.9 dB at its
// midpoint where our formula says -30 dB. `faderPieces` is what makes the segment straight
// again; see `place`.
//
// The two bounds are Tracktion's, not ours: below -100 dB the position is silence, and the
// parameter's range stops at 1, which is +6 dB. Clamped in double before the cast, because
// ADR 0015 §2 leaves `gain_db` unbounded and converting 1e300 to a float is not defined.
//
// ponytail: that ceiling means a `Mix.gain_db` above +6 dB renders as +6 dB — the ceiling a
// *static* `Mix` has had since M1 PR 7, since `setVolumeDb` clamps the same parameter, and it
// is stated here rather than discovered. The upgrade path, if a project ever needs more, is a
// gain the engine applies ahead of the fader, the way an audio clip's `gain_db` is applied
// ahead of Tracktion (ADR 0011 §2) — not a wider Tracktion parameter, which does not exist.
constexpr double kFaderMinDb = -100.0;
constexpr double kFaderMaxDb = 6.0;

float faderPosition (double db)
{
    if (! (db > kFaderMinDb))
        return 0.0f;
    return juce::jlimit (0.0f, 1.0f, te::decibelsToVolumeFaderPosition ((float) std::min (db, kFaderMaxDb)));
}

// How many straight pieces one `LINEAR` segment is split into so that the line stays straight
// in the model's units after the mapping above. `one` is every domain whose mapping is affine —
// a plugin's normalised value and `pan` — where a straight segment is already straight.
//
// The step is in decibels because the error is: linear interpolation of `exp(k·t)` over a piece
// spanning `d` dB is off by about `d²/368` dB at its middle, so 0.02 dB per piece keeps the
// worst point inside 1.1e-6 dB — about one count of 24-bit full scale, which is the bar M1 PR 8
// held the audio-clip fades to. It is in any case dominated by the rate at which the curve is *read*:
// Tracktion holds a parameter for a whole automation sub-block, `max(128, 128 * round(rate /
// 44100))` frames — 2.7 ms at 48 kHz — so a fast ramp is a staircase at that pitch however many
// points are under it (tracktion_PluginNode.cpp).
//
// Bounded by clamping the span to the fader's own useful range first, because `gain_db` is
// unbounded and `ceil(2e9 / 0.02)` is not a number of points to allocate. `place` bounds it
// again by the segment's tick span, since two points cannot share a tick and be two points.
constexpr double kFaderStepDb = 0.02;

juce::int64 one (double, double)
{
    return 1;
}

juce::int64 faderPieces (double from, double to)
{
    const auto reach = std::abs (juce::jlimit (kFaderMinDb, kFaderMaxDb, to)
                                 - juce::jlimit (kFaderMinDb, kFaderMaxDb, from));
    return (juce::int64) std::max (1.0, std::ceil (reach / kFaderStepDb));
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
// runtime CPU dispatch, and `reset()` zeroes the five samples of history it carries —
// `Interpolators::Lagrange` is `GenericInterpolator<LagrangeTraits, 5>`, and "four" here was
// wrong until M1 PR 13 — so its output is a pure function of the input, the ratio and the JUCE
// commit §17 pins. `ratio` is
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

// Every message Rubber Band sends a logger, kept rather than printed (M2 PR 11).
//
// At the library's default debug level only its level-0 messages reach a logger, and every one
// of those in 4.0.0 is a warning that what it is about to compute, or just computed, cannot be
// trusted — none is progress or chatter. One of them is the only notice given before a memory
// error: past a stretch ratio of 512 at 48 kHz R3 clamps its input hop to one sample, its output
// hop becomes the ratio, and past 4096 `synthesiseChannel` writes beyond a 4096-sample
// accumulator. The first render of `tests/determinism/render` to reach it asked for 6000, printed
// "Ratio yields ideal inhop < minimum, results may be suspect" and died in glibc's allocator,
// every time. So the library's own warning is the refusal, taken from the library rather than
// from a copy of its hop arithmetic here that a pin bump could quietly leave behind.
struct Objection final : RubberBand::RubberBandStretcher::Logger
{
    std::string said;

    void log (const char* message) override { keep (message); }
    void log (const char* message, double a) override
    {
        keep (std::string (message) + " (" + juce::String (a).toStdString() + ")");
    }
    void log (const char* message, double a, double b) override
    {
        keep (std::string (message) + " (" + juce::String (a).toStdString() + ", "
              + juce::String (b).toStdString() + ")");
    }
    void keep (std::string message)
    {
        if (said.empty())
            said = std::move (message);
    }
};

// The asset stretched to fill the clip, pitch unchanged (ADR 0011 §3). `frames` is the clip's
// length in samples, and the ratio is derived from it here rather than carried in the plan
// because `compile` reads no file and so cannot know the asset's duration (ADR 0007 §4). Empty on
// success; otherwise what Rubber Band objected to, and `buffer` is untouched.
//
// Refused **before** the first `process`, which is where an unmakeable ratio corrupts the heap,
// and again after the last `retrieve`, because a warning while processing is the library saying
// its output is wrong — and a render does not write audio its own time-stretcher disowned. A
// stretch it accepts logs nothing at all (ADR 0009 §4, measured), so neither check can move a
// golden. It is the engine's refusal and not compile's for the SFZ's reason: deciding it means
// reading the asset (ADR 0007 §6, extended).
//
// The block size is Rubber Band's own: `getSamplesRequired()` is the documented mode for a
// caller with no external constraint, and taking it means there is no block-size constant of
// ours for a golden to depend on. Draining while `available()` is positive and stopping at
// zero is what the library's own command line does when threading is off, which here it always
// is (main/main.cpp, the "completing" loop).
std::string stretch (juce::AudioBuffer<float>& buffer, double rate, int frames)
{
    const auto& in = buffer;
    const auto channels = (size_t) in.getNumChannels();
    const auto total = in.getNumSamples();

    const auto objection = std::make_shared<Objection>();
    RubberBand::RubberBandStretcher stretcher ((size_t) rate, channels, objection, kStretchOptions,
                                               frames / (double) total, 1.0);
    stretcher.setExpectedInputDuration ((size_t) total);

    std::vector<const float*> input (channels);
    for (size_t channel = 0; channel < channels; ++channel)
        input[channel] = in.getReadPointer ((int) channel);
    // Any number of samples at a time is allowed in the study pass, and there is one clip's
    // worth of them, so it is one call.
    stretcher.study (input.data(), (size_t) total, true);
    if (! objection->said.empty())
        return objection->said;

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
    if (! objection->said.empty())
        return objection->said;
    buffer = std::move (out);
    return {};
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

// Where a model tick falls on an edit's timeline. The tempo map is on the edit's own sequence,
// so one owner converts tick to time and there is one rounding site rather than two
// (ADR 0007 §3). Widened to 64 bits because an absolute tick is summed before it is converted
// (ADR 0002 §1).
te::TimePosition timeOf (te::Edit& edit, juce::int64 tick)
{
    return edit.tempoSequence.toTime (te::BeatPosition::fromBeats (tick / (double) kPpq));
}

// The other way, for a preview's transport: the model tick a position is in, floored, and held
// inside an `int32` because that is what a tick is on the wire (ADR 0002 §1).
int tickOf (te::Edit& edit, te::TimePosition time)
{
    const auto ticks = std::floor (edit.tempoSequence.toBeats (time).inBeats() * kPpq);
    return (int) juce::jlimit (0.0, (double) std::numeric_limits<int>::max(), ticks);
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
        if (auto fader = edit.getMasterVolumePlugin(); fader != nullptr)
            if (const auto why = strip (*fader, master); ! why.empty())
                return why;
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
        if (auto* fader = target.getVolumePlugin(); fader != nullptr)
            if (const auto why = strip (*fader, source); ! why.empty())
                return why;
        return clips (target, source);
    }

    // The strip's fader: the level and the position the plan holds still, then the lanes that
    // move them (ADR 0015 §1). `mute` and `solo` were applied by compile and cross false
    // (ADR 0007 §1), so a mix is a level and a position and nothing else.
    std::string strip (te::VolumeAndPanPlugin& fader, const PlanTrack& source)
    {
        // A pin, not a default, for the reason the pan law above it is one (§8): Tracktion
        // ramps a fader towards each block's target over 15 ms to hide zipper noise, and that
        // ramp is a shape *it* chose — a second renderer would not have it, and a version bump
        // could move it with no pull request to blame. ADR 0002 §8 refuses exactly that for an
        // automation curve, so the fader is driven with no smoothing of its own and what a
        // sample hears is our formula, read at the fixed rate ADR 0009 §3's single thread and
        // fixed block make repeatable.
        //
        // It moves no static render: with an unchanging mix the target equals the current value
        // on every block and `juce::SmoothedValue::setTargetValue` returns without ramping
        // either way, which is why every M1 golden is byte-identical across this line.
        //
        // ponytail: the ceiling is that a fast ramp is a staircase at the automation sub-block
        // — `max(128, 128 * round(rate / 44100))` frames, which `PluginNode::prepareToPlay`
        // turns on for any plugin that has automation and which is 2.7 ms at 48 kHz. That gate
        // is also why no committed golden could move here: a plan with no lane leaves it off.
        // The upgrade path, if the staircase is ever audible, is a finer read — a number
        // Tracktion owns — rather than a smoother whose shape belongs to the renderer.
        fader.smoothingRampTimeSeconds = 0.0;

        if (source.has_mix())
        {
            fader.setVolumeDb ((float) source.mix().gain_db());
            fader.setPan ((float) source.mix().pan());
        }

        for (const auto& lane : source.mix_lanes())
        {
            // The two names, and there is no third: `mute` and `solo` are booleans a double
            // cannot address without a threshold rule (ADR 0015 §1). The validator refuses
            // anything else, so this is the trust boundary saying so rather than a second
            // validator — and a lane silently dropped here would be a fader that never moved.
            if (lane.param() == "gain_db")
                place (*fader.volParam, lane, faderPosition, faderPieces);
            else if (lane.param() == "pan")
                place (*fader.panParam, lane, panPosition);
            else
                return "a mix lane names '" + lane.param() + "'; a track automates `gain_db` or `pan`";
        }
        return {};
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

    // The tempo map was put on the edit's sequence before this builder ran (`editFor`).
    te::TimePosition at (juce::int64 tick)
    {
        return timeOf (edit, tick);
    }

    // ADR 0002 §8's two curves, and they are **ours** rather than Tracktion's. `curve` on a
    // point is the shape of the segment from that point to the next; before the first point the
    // value is the first point's, and after the last it is the last's.
    //
    //   LINEAR  the value moves in a straight line to the next point, **in ticks** — §4.2's one
    //           musical time, and the axis a point is stored on.
    //   HOLD    the value stays at this point's until the next point, where it steps.
    //
    // A `LINEAR` segment is **split**, at every tick where the straight line our formula draws
    // would stop being the straight line Tracktion draws between two curve points. There are
    // two such places and they are independent, so the ticks are collected and the segment cut
    // at all of them at once:
    //
    //   *A tempo event inside the segment.* Tracktion's parameter curve is in seconds
    //   (tracktion_AutomatableParameter.cpp, where AutomationCurveSource builds it with
    //   TimeBase::time), so a straight segment there is straight in seconds; ours is straight
    //   in ticks. Within one tempo the two are the same line and across a change they are not.
    //   Every piece then lies in one constant tempo, and the result is our formula rather than
    //   an approximation to it.
    //
    //   *A mapping that is not affine.* Tracktion interpolates in the parameter's own domain,
    //   and the fader's domain is a slider position rather than decibels (`faderPosition`), so
    //   a line straight in dB is an exponential there. `pieces` cuts it into steps small enough
    //   that the chord is the curve to well under a 24-bit count; unlike the tempo split this
    //   one is an approximation with a stated bound, because no finite number of straight
    //   pieces is an exponential. Every other domain passes `one` and is cut only by tempo.
    //
    // HOLD is a second point at the segment's end carrying the segment's own value: Tracktion
    // draws a straight line between two equal values, which is the hold, and the next point at
    // that same instant is the step.
    void place (te::AutomatableParameter& param,
                const PlanLane& lane,
                float (*value) (double) = normalised,
                juce::int64 (*pieces) (double, double) = one)
    {
        auto& curve = param.getCurve();
        std::vector<juce::int64> splits;

        for (int i = 0; i < lane.points_size(); ++i)
        {
            const auto& point = lane.points (i);
            curve.addPoint (at (point.tick()), value (point.value()), 0.0f, nullptr);
            if (i + 1 == lane.points_size())
                break;

            const auto& next = lane.points (i + 1);
            if (point.curve() == escribass::song::v1::CURVE_HOLD)
            {
                curve.addPoint (at (next.tick()), value (point.value()), 0.0f, nullptr);
                continue;
            }

            const juce::int64 span = (juce::int64) next.tick() - point.tick();
            splits.clear();
            for (const auto tick : tempoTicks)
                if (tick > point.tick() && tick < next.tick())
                    splits.push_back (tick);
            // At most one piece per tick: two points cannot share a tick and still be two
            // points. The cap binds only where a segment has fewer ticks than the pieces it
            // asks for, and each piece is then a single tick — finer than the model can say.
            const auto cuts = std::min (span, pieces (point.value(), next.value()));
            for (juce::int64 n = 1; n < cuts; ++n)
                splits.push_back (point.tick() + (span * n) / cuts);
            std::sort (splits.begin(), splits.end());
            splits.erase (std::unique (splits.begin(), splits.end()), splits.end());

            for (const auto tick : splits)
            {
                const auto through = (tick - point.tick()) / (double) span;
                curve.addPoint (at (tick),
                                value (point.value() + through * (next.value() - point.value())),
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
        // `toSamples` answers in `int64_t` and every buffer below counts frames in `int`, so
        // the narrowing is the dangerous step and it is done once, after a refusal (M1 PR 13).
        // It used to be a bare `(int)` cast: `length_ticks` is an `int32` that `check` bounded
        // in neither direction, and 1073741824 ticks at 120 bpm and 48 kHz is 26,843,545,600
        // samples, which wraps modulo 2^32 back to a *positive* 1,073,741,824 — past the
        // `<= 0` guard and into an 8.6 GB `AudioBuffer`. `juce::AudioBuffer` allocates through
        // `HeapBlock<char, true>` and nothing here catches, so that aborted the process; the
        // wrapped values that do allocate are worse, because the scratch clip is then the
        // wrong length while `insertWaveClip` still places it over the un-truncated range —
        // wrong audio, exit 0.
        const auto span = te::toSamples (at (last), rate) - start;
        if (span <= 0)
            return "an audio clip of " + std::to_string (clip.length_ticks())
                   + " ticks is under one sample long at this tempo and rate";
        if (span > (juce::int64) std::numeric_limits<int>::max())
            return "an audio clip of " + std::to_string (clip.length_ticks()) + " ticks is "
                   + std::to_string (span) + " sample frames at this tempo and rate, which is "
                     "more than one buffer holds";
        const auto frames = (int) span;

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
            if (const auto why = stretch (buffer, rate, frames); ! why.empty())
                return "an audio clip stretches '" + source.path() + "' from "
                       + std::to_string (buffer.getNumSamples()) + " to " + std::to_string (frames)
                       + " sample frames, and Rubber Band will not: " + why;

        // Exactly the clip, which is what makes the fades below run over the clip's length and
        // not the asset's: a shorter asset ends in silence, a longer one stops, and both are
        // what ADR 0011 §3 says an unstretched clip does.
        buffer.setSize (buffer.getNumChannels(), frames, true, true, false);
        // The two fade lengths narrow the same way the clip's own length does, and for the same
        // reason: `fade_in_ticks` is an `int32` nothing bounds, and a fade that overruns `int`
        // would arrive at `shape` as a wrong — possibly negative, and so silently skipped —
        // ramp length. They are not clamped to the clip, because ADR 0011 §2 gives a fade
        // longer than its clip a meaning: the ramp simply never reaches unity.
        const auto fadeIn = te::toSamples (at (first + held.fade_in_ticks()), rate) - start;
        const auto fadeOut = te::toSamples (at (last), rate)
                             - te::toSamples (at (last - held.fade_out_ticks()), rate);
        if (fadeIn > (juce::int64) std::numeric_limits<int>::max()
            || fadeOut > (juce::int64) std::numeric_limits<int>::max())
            return "an audio clip's fade is longer than one buffer holds";
        // §4.4 leaves `gain_db` unbounded above, and a finite one still leaves the finite
        // doubles: 1e9 dB is 10^5e7, which is `inf`, and an `inf` gain renders the whole clip
        // as silence with exit 0 and nothing on stderr. What the plan carries is checked at the
        // boundary (`check`); this is the one value the boundary cannot check, because it is
        // the *conversion* that overflows and not the number.
        if (! std::isfinite (std::pow (10.0, held.gain_db() / 20.0)))
            return "an audio clip's gain of " + std::to_string (held.gain_db())
                   + " dB is not a finite gain";
        shape (buffer, held.gain_db(), (int) fadeIn, (int) fadeOut);

        const auto processed = scratch.getChildFile ("clip" + juce::String (++placed) + ".wav");
        // Mirrors the destination's own delete below, and for the identical reason: JUCE opens
        // an existing file at its end, so a `clipN.wav` that is already there would be appended
        // to and `createReaderFor` would take the stale payload in front. `TempDir` now makes
        // the directory unpredictable, which is the root fix; this keeps the invariant local to
        // the one call that depends on it rather than to a decision three hundred lines away.
        if (processed.existsAsFile() && ! processed.deleteFile())
            return "could not replace " + processed.getFullPathName().toStdString();
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
        // above guarantees, the kernel is a delta and the steady state comes back bit-identical,
        // but the interpolator's two-sample base latency is not compensated, so **the clip
        // sounds two samples after its position**.
        //
        // **Re-measured in M1 PR 13, and it is real.** A ramp asset whose sample k is a known
        // value, placed at tick 480 of a 120 bpm 48 kHz render, comes back with source sample k
        // at output frame 12002 + k, and the clip's last two source samples fall off the end of
        // its window. **Why upstream's fix does not apply:** Tracktion does compensate this, in
        // `LagrangeResamplerReader::readSamples` — it reads `getBaseLatency()` extra source
        // frames on the first block and drops the matching destination frames, guarded by a
        // `hasBeenReset` that initialises `true` — and that reader belongs to
        // `WaveNodeRealTime`. Upstream's commit says so in its own subject: `319afc0`
        // (2024-07-23) removed the latency "when using AudioClipBase::setUsesProxy (false)".
        // A clip here `canUseProxy()`, so `EditNodeBuilder` builds the legacy `WaveNode`
        // instead, which has no such path. The commit is an ancestor of our pin and changes
        // nothing for a proxied clip.
        //
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

// A pin, not a default: the pan law is a process-wide static in Tracktion
// (tracktion_AudioUtilities.cpp) that any module could set, and it decides the two gains a
// `Mix.pan` becomes. Linear is what Tracktion itself starts at, and it is the only one of the
// five that leaves a centred track at exactly its `gain_db`; the cost is that a hard pan is
// +6 dB on the surviving side. Nothing in the model names a law, so this is the engine stating
// which one it renders, not a choice the document makes. Set by both modes, before either
// builds an edit, because a preview that panned by another law would be a fourth reason
// preview and export disagree, and one nobody chose (trap 3).
void pinPanLaw()
{
    te::setDefaultPanLaw (te::PanLawLinear);
}

// The edit a plan describes, for either mode: an export renders it, a preview plays it
// (ADR 0013 §2). **One builder**, which is the point — a preview edit assembled by a second
// function would be a second reading of the plan in C++, and "the preview plays the same plan"
// would stop being true the first time the two disagreed. What differs is the role, and the
// role is Tracktion's own: `forRendering` refuses a playback context, `forEditing` wants one.
//
// Returns the edit and where the plan ends, or why it cannot be built.
tl::expected<std::pair<std::unique_ptr<te::Edit>, te::TimePosition>, std::string>
    editFor (te::Engine& engine, Plugins& plugins, const RenderPlan& plan, te::Edit::EditRole role)
{
    // One audio track per plan track, and the master. Master volume at 0 dB so the level is the
    // plan's Mix, not Tracktion's -3 dB default. At least one track even for a plan with none,
    // because an edit's own default is one and a silent extra track sums exactly zero.
    auto edit = te::Edit::createEdit ({ engine,
                                        te::createEmptyEdit (engine),
                                        te::ProjectItemID (1, {}),
                                        role,
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
    if (const auto why = Builder (*edit, plugins, plan, end).build(); ! why.empty())
        return tl::unexpected (why);
    return std::make_pair (std::move (edit), end);
}

// **One render, and nothing about how the plan arrived.** Kept apart from `serve` below, which
// takes the plan off a gRPC call, because what this does is the milestone's claim and how the
// plan travelled is not: it is handed the manifest this build was told to host from, a scratch
// directory that outlives the render and dies with the process, and a plan.
//
// Returns `kOk` having filled `result`, or the stage the render died in, having already said
// why on stderr. Failure is a code and never a `RenderResult` carrying errors (ADR 0008 §1).
int render (const juce::var& manifest, const juce::File& scratch, const RenderPlan& plan, RenderResult& result)
{
    te::Engine engine (std::make_unique<Storage> (scratch), std::make_unique<te::UIBehaviour>(), std::make_unique<Behaviour> (false));
    auto* wav = engine.getAudioFileFormatManager().getWavFormat();
    pinPanLaw();

    // **Headless, checked rather than trusted** (ADR 0013 §3). Since M2 PR 10 this binary also
    // opens a device, and the difference is one boolean passed to `Behaviour`. With it false
    // Tracktion registers no device type at all, which holds on a runner with no sound card
    // exactly as on a machine with one — so every render in CI is the check that an export
    // process was never offered the machine's sound server, rather than a golden that would
    // only have moved on a desk with a speaker on it.
    if (! engine.getDeviceManager().deviceManager.getAvailableDeviceTypes().isEmpty())
        return fail (kRenderFailed, "an export process was offered audio devices, and an export is"
                                    " headless (ADR 0013 §3)");

    if (const auto why = check (plan, *wav); ! why.empty())
        return fail (kBadPlan, why);
    if (plan.length_ticks() == 0)
        return fail (kBadPlan, "length_ticks is 0; nothing to render");
    if (! juce::File::isAbsolutePath (plan.output_path()))
        return fail (kBadPlan, "output_path is not absolute: '" + plan.output_path() + "'");
    const auto& target = plan.target();

    Plugins plugins (engine, manifest);
    auto built = editFor (engine, plugins, plan, te::Edit::forRendering);
    if (! built)
        return fail (kBadPlan, built.error());
    auto& edit = built->first;
    const auto end = built->second;

    te::Renderer::Parameters params (*edit);
    // **The render writes a sibling `.part` and renames it, and the destination is untouched
    // until every check below has passed** (M1 PR 13). Before that the destination was deleted
    // up front and written in place, so a render killed halfway left a truncated WAV — measured
    // at 43 MB with a `data` chunk declaring more than the file held — exactly where the last
    // good render had been, and the engine never reached its own length check to say so. A
    // POSIX rename is atomic: what a reader sees is the old file or the new one and never half
    // of either.
    //
    // **The `.part` is removed first, and this is not tidiness.** Tracktion opens it with
    // `juce::File::createOutputStream()`, which positions at the *end* of an existing file, so a
    // render over one that is already there appends a second, complete RIFF file after the
    // first. The WAV then holds two `data` chunks; every reader takes the first, which is the
    // *previous* render. Measured in M1 PR 11: rendering one plan twice to one path leaves a
    // file of exactly twice the size whose `bext` origination time is still the first run's, and
    // the read-back below hashed that first chunk and reported the old audio as this render's
    // answer. That made every "renders the same twice" check that reused one path vacuous, and
    // it is why `tests/renders.rs` gives each render a path of its own as well. Writing to a
    // `.part` nobody else writes closes the same hole a second way.
    const juce::File finished (plan.output_path());
    params.destFile = finished.getSiblingFile (finished.getFileName() + ".part");
    if (params.destFile.existsAsFile() && ! params.destFile.deleteFile())
        return fail (kRenderFailed, "cannot replace " + params.destFile.getFullPathName().toStdString());
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
    if (handle == nullptr)
        // Never seen: `EditRenderer::render` always returns a handle. Checked because the loop
        // below asks it for progress and cancels it, and a null one there would be a hang.
        return fail (kRenderFailed, "Tracktion started no render and reported no failure");

    // **The loop is bounded, and the bound counts iterations rather than reading a clock**
    // (CLAUDE.md #3; M1 PR 13). `done` is set only by the callback above, nothing sets
    // `hasBeenCancelled` during a render, and `NodeRenderContext`'s "wait for any nodes to
    // render their sources" loop sleeps forever on a leaf node that never reports itself ready
    // — the hang the comment beside `setToFile` records hitting once and fixing one cause of.
    // `core` then blocks in `wait_with_output()` with no timeout of its own and the whole tool
    // API stops, which is a worse failure than any render.
    //
    // What is counted is dispatch iterations **without progress**, not iterations: a render
    // that is still moving is never interrupted however long it takes, and only one that has
    // stopped moving is given up on. The bound decides failure and never a sample — every
    // render that finishes finishes identically whatever it is set to.
    constexpr int kStalledIterations = 60'000;  // each waits up to 10 ms, so ten minutes still
    int stalled = 0;
    float furthest = -1.0f;
    bool gaveUp = false;
    while (! done)
    {
        juce::MessageManager::getInstance()->runDispatchLoopUntil (10);
        if (const auto progress = handle->getProgress(); progress > furthest)
        {
            furthest = progress;
            stalled = 0;
        }
        else if (! gaveUp && ++stalled >= kStalledIterations)
        {
            // Cancel, then keep pumping: `RenderTask` checks cancellation both in the leaf-node
            // wait and in its block loop, so the render thread returns and the callback fires,
            // which is what lets the handle join instead of blocking on a thread that is stuck.
            gaveUp = true;
            handle->cancel();
        }
    }
    handle.reset();

    if (gaveUp)
        return fail (kRenderFailed, "the render stopped making progress at "
                                        + std::to_string (furthest) + " and was cancelled");
    if (! error.empty())
        return fail (kRenderFailed, "render failed: " + error);
    if (! params.destFile.existsAsFile())
        return fail (kRenderFailed, "render reported success and wrote nothing at "
                                        + params.destFile.getFullPathName().toStdString());

    // What the file holds is checked against what the plan asked, not against what any call
    // returned (ADR 0008 §3), and the hash is of the data chunk alone (ADR 0009 §2).
    juce::FileInputStream in (params.destFile);
    const auto wrote = params.destFile.getFullPathName().toStdString();
    if (! in.openedOk())
        return fail (kBadOutput, "cannot read back " + wrote);
    const auto dataSize = seekToData (in);
    if (! dataSize)
        return fail (kBadOutput, wrote + ": " + dataSize.error());
    const auto expectedBytes = expectedFrames * 2 * (juce::int64) (target.bit_depth() / 8);
    if (*dataSize != expectedBytes)
        return fail (kBadOutput, "the data chunk holds " + std::to_string (*dataSize) + " bytes; the plan's "
                                     + std::to_string (expectedFrames) + " frames of " + std::to_string (target.bit_depth())
                                     + "-bit stereo are " + std::to_string (expectedBytes));

    result.set_pcm_sha256 (juce::SHA256 (in, *dataSize).toHexString().toStdString());
    for (const auto& commit : escribass::provenance::engineCommits)
        (*result.mutable_commits())[commit.component] = commit.sha;
    for (const auto& commit : escribass::provenance::pluginCommits)
        (*result.mutable_commits())[commit.component] = commit.sha;

    // Every check has passed, so the `.part` becomes the render. `std::rename` rather than
    // `juce::File::moveFileTo`, which unlinks the destination *before* renaming and so has a
    // window where neither file is there; POSIX `rename` replaces the destination in one step,
    // which is the whole point of writing a `.part` in the first place. The stream above is
    // still open on it, and a rename does not care.
    if (std::rename (params.destFile.getFullPathName().toRawUTF8(), plan.output_path().c_str()) != 0)
        return fail (kBadOutput, "rendered " + params.destFile.getFullPathName().toStdString()
                                     + " and could not move it to " + plan.output_path());

    return kOk;
}

// ------------------------------------------------------------------------------------------
// How a plan gets here (ADR 0013 §3)
// ------------------------------------------------------------------------------------------

// The build manifest, or an empty `var` having already said why (ADR 0010 §4).
juce::var manifestIn (const juce::File& file)
{
    const auto manifest = juce::JSON::parse (file);
    if (! manifest.isObject())
        fail (kBadPlan, "no build manifest at " + file.getFullPathName().toStdString()
                            + "; it is written by `cmake --build engine/build --target manifest` (ADR 0010 §4)");
    return manifest;
}

// The scratch directory a render runs in, or an empty `File` having already said why.
juce::File scratchIn (const TempDir& scratch)
{
    if (scratch.dir == juce::File())
        fail (kRenderFailed, "could not create a scratch directory under "
                                 + juce::File::getSpecialLocation (juce::File::tempDirectory)
                                       .getFullPathName()
                                       .toStdString());
    return scratch.dir;
}

// The one `Render` call this process serves, parked for the message thread.
//
// **Nothing touches Tracktion off the message thread** (ADR 0013 §4), and a gRPC handler runs
// on a thread of gRPC's own. So the handler's whole job is to hand the plan over, wait, and
// answer with what the message thread rendered — which is `main`, because `EditRenderer::render`
// is asynchronous and only a pumped dispatch loop ever calls it back (ADR 0008 §3).
class RenderService final : public escribass::render::v1::Render::Service {
public:
    ::grpc::Status Render (::grpc::ServerContext*, const RenderPlan* request, RenderResult* response) override
    {
        std::unique_lock<std::mutex> lock (mutex);
        // ADR 0008 §2's guarantee is the operating system's, not a rule someone follows: a
        // second render in this process would inherit the first one's plugin instances and
        // their parameter smoothers, so its first block would depend on what was rendered
        // before it. One call, then the process dies with its answer.
        if (plan != nullptr)
            return { ::grpc::StatusCode::RESOURCE_EXHAUSTED,
                     "this engine process serves one render and is already serving it (ADR 0008 §2)" };
        plan = request;
        answered.wait (lock, [this] { return done; });
        if (code != kOk)
            return { ::grpc::StatusCode::INTERNAL,
                     "the render failed and this process is exiting " + std::to_string (code)
                         + "; the reason is on its stderr (ADR 0008 §1)" };
        *response = std::move (result);
        return ::grpc::Status::OK;
    }

    // What the handler is waiting on, or nullptr. **Polled**, because the message thread cannot
    // block on a condition variable — it has a dispatch loop to pump.
    const RenderPlan* waiting()
    {
        const std::lock_guard<std::mutex> lock (mutex);
        return plan;
    }

    // The message thread's answer, which is what lets the handler return.
    void answer (int rendered, RenderResult&& what)
    {
        {
            const std::lock_guard<std::mutex> lock (mutex);
            code = rendered;
            result = std::move (what);
            done = true;
        }
        answered.notify_all();
    }

private:
    std::mutex mutex;
    std::condition_variable answered;
    const RenderPlan* plan = nullptr;
    RenderResult result;
    int code = kOk;
    bool done = false;
};

// The server a mode registers its one service on, listening, with its address on stdout — or
// nullptr, having already said why.
//
// **How the caller learns where to dial, and that it may.** The engine picks the socket rather
// than being told one: `TempDir` is a `mkdtemp`, so the directory is 0700, unique and this
// process's own, and the path needs no entropy in `core` — which is where CLAUDE.md #3 does not
// want any. The address is printed on stdout only *after* `BuildAndStart` has returned a
// listening server, so that one line is the address and the readiness at once: a caller holding
// it cannot be refused a connection, and there is nothing to poll, retry or sleep on. ADR 0013
// §3 had the path travelling the other way, on argv, and is amended in place with this.
//
// stdout is the address and nothing else, which is the rule stdio lived under and for the same
// reason (ADR 0008 §1): every log line, JUCE's warnings and every plugin's chatter go to stderr.
//
// **One service per server, and that is the mode** (ADR 0013 §3). Whatever is passed here is
// all this process serves, so a `Preview` call on an export process is answered UNIMPLEMENTED
// by gRPC's own dispatch — not refused by a check this file would have to remember to make.
std::unique_ptr<::grpc::Server> announce (::grpc::Service& service, const juce::File& dir, int& code)
{
    const auto socket = dir.getChildFile ("engine.sock").getFullPathName().toStdString();
    ::grpc::ServerBuilder builder;
    // Insecure because the socket *is* the security: a 0700 directory nobody else can enter. A
    // TCP port would be reachable by every process on the machine, and a render is a write to a
    // path its caller names.
    builder.AddListeningPort ("unix:" + socket, ::grpc::InsecureServerCredentials());
    // **No four-megabyte ceiling.** That is gRPC's default maximum received message, and stdio
    // had no limit at all — so a project large enough to pass it would meet a refusal the
    // transport invented, rather than one the validator or `compile` states (ADR 0007 §6).
    builder.SetMaxReceiveMessageSize (-1);
    builder.RegisterService (&service);
    auto server = builder.BuildAndStart();
    if (server == nullptr)
    {
        code = fail (kRenderFailed, "could not serve on unix:" + socket);
        return nullptr;
    }

    // Checked, the way the RenderResult on stdout is: an address that never left the process
    // leaves a caller waiting for a line that is not coming (M1 PR 13).
    std::printf ("unix:%s\n", socket.c_str());
    if (std::fflush (stdout) != 0)
    {
        code = fail (kBadOutput, "could not write the socket address to stdout");
        return nullptr;
    }
    return server;
}

// **Bounded, and the bound counts iterations rather than reading a clock** (CLAUDE.md #3),
// exactly as the render's own dispatch loop is. What it decides is failure and never a sample: a
// caller that dials is served whatever it then asks for, however long this waited. Without it,
// a parent that died between spawning this process and calling it leaves an engine pumping a
// dispatch loop for ever.
constexpr int kIdleIterations = 60'000;  // each waits up to 10 ms, so ten minutes

// `Render` over a Unix socket: the transport docs/specs.md §3 always described, and the one M2
// makes true (ADR 0013 §1). One process, one call, then exit (ADR 0013 §3). The stdio path this
// replaced was deleted in the same pull request rather than kept beside it — two transports for
// one boundary is one tested transport and one that is not.
int serve (const juce::File& manifestFile)
{
    const auto manifest = manifestIn (manifestFile);
    if (! manifest.isObject())
        return kBadPlan;

    const juce::ScopedJuceInitialiser_GUI juceInit;
    const TempDir scratch;
    const auto dir = scratchIn (scratch);
    if (dir == juce::File())
        return kRenderFailed;

    RenderService service;
    int failed = kOk;
    const auto server = announce (service, dir, failed);
    if (server == nullptr)
        return failed;

    const RenderPlan* plan = nullptr;
    for (int idle = 0; (plan = service.waiting()) == nullptr; ++idle)
    {
        if (idle >= kIdleIterations)
            return fail (kRenderFailed, "no Render call arrived");
        juce::MessageManager::getInstance()->runDispatchLoopUntil (10);
    }

    RenderResult result;
    const auto code = render (manifest, dir, *plan, result);
    service.answer (code, std::move (result));
    // Blocks until the handler above has returned and its answer is on the wire, which is what
    // makes exiting here safe.
    server->Shutdown();
    server->Wait();
    return code;
}

// ------------------------------------------------------------------------------------------
// Preview: the plan played from a tick, on the device (ADR 0013 §2, §3, §4)
// ------------------------------------------------------------------------------------------
//
// **What is a clock here, and what is not** (CLAUDE.md #3). A preview is real time, and nothing
// about it is claimed bit-exact (docs/plan.md, "What M2 will not claim"). The line #3 draws is
// around the engine's *decisions*: nothing below chooses a sample, a note, a parameter value or
// a plan by reading the time. What moves the transport is the audio device pulling blocks —
// the device's clock is the input, the way a plan is the input to an export, and Tracktion
// advances the position by the frames it rendered. Every event this file writes reports that
// position; the one decision it takes from it is to stop at the end of the plan, which is where
// an export stops too. What *does* read a clock is failure's alone: the idle bound above, and
// the dispatch loop's ten-millisecond wait, which decides how soon a command is picked up and
// never what it does. None of it is in `render`, which this mode does not call: an export stays
// exactly as headless, exactly as single-process, as it was (ADR 0013 §3).

using escribass::render::v1::PreviewCommand;
using escribass::render::v1::PreviewEvent;
using escribass::render::v1::PreviewPlay;

// The one `Preview` stream this process plays, between gRPC's threads and the message thread.
//
// **Nothing touches Tracktion off the message thread** (ADR 0013 §4). The handler runs on a
// thread of gRPC's, reads commands on a second thread of its own, and writes events on the one
// it was called on; the message thread — `main` — takes the commands, applies them, and hands
// back what to write. Both directions are a queue under one mutex, **polled** by the message
// thread for the reason `RenderService` is polled: it has a dispatch loop to pump and cannot
// block on a condition variable. Nothing is posted with `callAsync`, so no message can outlive
// the objects it would touch, which is the whole of the lifetime reasoning.
class PreviewService final : public escribass::render::v1::Preview::Service {
public:
    ::grpc::Status Preview (::grpc::ServerContext* context,
                            ::grpc::ServerReaderWriter<PreviewEvent, PreviewCommand>* stream) override
    {
        {
            const std::lock_guard<std::mutex> lock (mutex);
            // The process lives as long as its stream (ADR 0013 §3), so there is never a second:
            // two callers on one transport would each be moving the other's playhead.
            if (opened)
                return { ::grpc::StatusCode::RESOURCE_EXHAUSTED,
                         "this engine process plays one preview and is already playing it (ADR 0013 §3)" };
            opened = true;
        }

        // Read and Write may run concurrently on one stream, one of each (grpcpp's
        // sync_stream.h), which is what lets a command arrive while an event is being written.
        std::thread reader ([&]
        {
            PreviewCommand command;
            while (stream->Read (&command))
            {
                const std::lock_guard<std::mutex> lock (mutex);
                inbox.push_back (command);
            }
            {
                const std::lock_guard<std::mutex> lock (mutex);
                closed = true;
            }
            changed.notify_all();
        });

        auto status = ::grpc::Status::OK;
        for (;;)
        {
            std::unique_lock<std::mutex> lock (mutex);
            changed.wait (lock, [this] { return ! outbox.empty() || refusal.has_value() || closed; });
            if (refusal.has_value())
            {
                status = *refusal;
                break;
            }
            if (outbox.empty())
                break;  // closed, and nothing left to say
            const auto event = outbox.front();
            outbox.pop_front();
            lock.unlock();
            if (! stream->Write (event))
            {
                status = { ::grpc::StatusCode::CANCELLED, "the caller stopped reading" };
                break;
            }
        }

        // A reader still blocked in `Read` is released by cancelling the call, which a refusal
        // and a caller that stopped reading both need; a closed stream has released it already.
        if (! status.ok())
            context->TryCancel();
        reader.join();

        const std::lock_guard<std::mutex> lock (mutex);
        done = true;
        return status;
    }

    bool isOpen()
    {
        const std::lock_guard<std::mutex> lock (mutex);
        return opened;
    }

    bool isDone()
    {
        const std::lock_guard<std::mutex> lock (mutex);
        return done;
    }

    // The commands that have arrived, for the message thread, and whether the caller has closed.
    std::pair<std::vector<PreviewCommand>, bool> take()
    {
        const std::lock_guard<std::mutex> lock (mutex);
        std::vector<PreviewCommand> taken (inbox.begin(), inbox.end());
        inbox.clear();
        return { std::move (taken), closed };
    }

    void say (PreviewEvent event)
    {
        {
            const std::lock_guard<std::mutex> lock (mutex);
            outbox.push_back (std::move (event));
        }
        changed.notify_all();
    }

    // Ends the stream with a status. Failure is a status and never an event, as it is an exit
    // code and never a RenderResult (ADR 0013 §2, ADR 0008 §1).
    void refuse (::grpc::Status why)
    {
        {
            const std::lock_guard<std::mutex> lock (mutex);
            refusal = std::move (why);
        }
        changed.notify_all();
    }

private:
    std::mutex mutex;
    std::condition_variable changed;
    std::deque<PreviewCommand> inbox;
    std::deque<PreviewEvent> outbox;
    std::optional<::grpc::Status> refusal;
    bool opened = false;
    bool closed = false;
    bool done = false;
};

// The transport a plan plays on, and the one place a preview touches Tracktion. Every member is
// called on the message thread.
class Player {
public:
    Player (te::Engine& e, const juce::var& manifest) : engine (e), plugins (e, manifest) {}
    ~Player() { release(); }

    // One command, applied — or the status that ends the stream instead. What arrives here was
    // already refused upstream if a caller could fix it (`core`'s `preview_idle` and
    // `tick_negative`), so this is the trust boundary and not a second validator.
    ::grpc::Status apply (const PreviewCommand& command)
    {
        if (command.command_case() == PreviewCommand::kPlay)
            return play (command.play());
        if (command.command_case() == PreviewCommand::COMMAND_NOT_SET)
            return invalid ("a command with no arm set");
        if (edit == nullptr)
            // ADR 0013 §2: the first command carries the plan. A seek into nothing is not a
            // position, and answering it with one would be inventing a transport.
            return { ::grpc::StatusCode::FAILED_PRECONDITION, "nothing is loaded: the first command is a play (ADR 0013 §2)" };

        auto& transport = edit->getTransport();
        switch (command.command_case())
        {
            case PreviewCommand::kSeek:
                if (command.seek().tick() < 0)
                    return invalid ("a seek to tick " + std::to_string (command.seek().tick()));
                transport.setPosition (timeOf (*edit, command.seek().tick()));
                break;
            case PreviewCommand::kLoop:
                if (command.loop().start_tick() < 0 || command.loop().end_tick() < 0)
                    return invalid ("a loop from " + std::to_string (command.loop().start_tick()) + " to "
                                    + std::to_string (command.loop().end_tick()));
                loop = command.loop();
                applyLoop();
                break;
            case PreviewCommand::kStop:
                transport.stop (false, false);
                break;
            default:
                break;
        }
        ++applied;
        return ::grpc::Status::OK;
    }

    // Where the transport is, as the answer to the command just applied.
    PreviewEvent event()
    {
        PreviewEvent now;
        now.set_applied (applied);
        now.set_state (escribass::render::v1::PREVIEW_STATE_STOPPED);
        if (edit != nullptr)
        {
            auto& transport = edit->getTransport();
            now.set_tick (tickOf (*edit, transport.getPosition()));
            if (transport.isPlaying())
                now.set_state (escribass::render::v1::PREVIEW_STATE_PLAYING);
        }
        last = now;
        return now;
    }

    // An event the transport has earned on its own since the last one — the tick moved, or it
    // reached the end of the plan and stopped there — or nothing.
    //
    // Once per turn of the dispatch loop rather than on a timer of its own: the rate is then
    // whatever the loop already runs at, and there is no interval here for anybody to choose.
    std::optional<PreviewEvent> moved()
    {
        if (edit == nullptr)
            return std::nullopt;
        auto& transport = edit->getTransport();
        // The end of the plan is where an export stops, so it is where a preview stops. Not
        // while looping: a loop past the end is one somebody set.
        if (transport.isPlaying() && ! transport.looping
            && tickOf (*edit, transport.getPosition()) >= loaded.length_ticks())
            transport.stop (false, false);
        const auto before = last;
        const auto now = event();
        if (now.tick() == before.tick() && now.state() == before.state())
            return std::nullopt;
        return now;
    }

    // Stops the transport and frees the edit, here on the message thread, before the engine it
    // belongs to is destroyed.
    void release()
    {
        if (edit == nullptr)
            return;
        edit->getTransport().stop (false, true);
        edit.reset();
    }

private:
    ::grpc::Status play (const PreviewPlay& request)
    {
        if (request.start_tick() < 0)
            return invalid ("a play from tick " + std::to_string (request.start_tick()));

        // **Replaced, never diffed** (ADR 0013 §2) — and not replaced when it is the same plan.
        // A play after a stop sends the plan it stopped, and building that again would throw
        // away the plugin instances `PreviewStop` exists to keep.
        if (edit == nullptr || ! google::protobuf::util::MessageDifferencer::Equals (request.plan(), loaded))
        {
            if (const auto why = check (request.plan(), *engine.getAudioFileFormatManager().getWavFormat()); ! why.empty())
                return invalid (why);
            release();
            auto built = editFor (engine, plugins, request.plan(), te::Edit::forEditing);
            if (! built)
                return invalid (built.error());
            edit = std::move (built->first);
            loaded = request.plan();
            edit->getTransport().ensureContextAllocated();
            // The loop is the transport's and the transport is new, so it is carried over: a
            // person looping a bar while editing it keeps looping it through every edit.
            applyLoop();
        }

        auto& transport = edit->getTransport();
        transport.setPosition (timeOf (*edit, request.start_tick()));
        transport.play (false);
        ++applied;
        return ::grpc::Status::OK;
    }

    // ADR 0013 §2: an empty range — `end_tick` at or before `start_tick` — clears the loop.
    void applyLoop()
    {
        auto& transport = edit->getTransport();
        if (loop.end_tick() > loop.start_tick())
        {
            transport.setLoopRange ({ timeOf (*edit, loop.start_tick()), timeOf (*edit, loop.end_tick()) });
            transport.looping = true;
        }
        else
        {
            transport.looping = false;
        }
    }

    static ::grpc::Status invalid (const std::string& why)
    {
        return { ::grpc::StatusCode::INVALID_ARGUMENT, why };
    }

    te::Engine& engine;
    Plugins plugins;
    std::unique_ptr<te::Edit> edit;
    RenderPlan loaded;
    escribass::render::v1::PreviewLoop loop;
    PreviewEvent last;
    int applied = 0;
};

// Why there is no device, as specifically as the machine can say it.
std::string noDevice (juce::AudioDeviceManager& devices)
{
    std::string listed;
    for (auto* type : devices.getAvailableDeviceTypes())
    {
        type->scanForDevices();
        listed += " " + type->getTypeName().toStdString() + " lists "
                  + std::to_string (type->getDeviceNames (false).size()) + " output devices.";
    }
    if (listed.empty())
        listed = " JUCE offers no device type at all.";
    return "no audio output device opened, so there is nothing to play a preview on." + listed
           + " This build speaks ALSA, not PulseAudio or PipeWire, so a machine whose sound goes"
             " through a sound server needs ALSA's default PCM pointed at it (ADR 0013 §4)";
}

// `Preview` over a Unix socket, for as long as its one stream is open (ADR 0013 §3).
//
// **The device first, then the address.** An engine that cannot play has nothing to serve, so a
// machine with no output device is an exit code and a sentence on stderr before any socket
// exists — which is what reaches a caller as `engine_failed` with the reason in it, and what a
// runner with no sound card can check (docs/plan.md, M2 trap 13).
//
// **What pumps the message loop is this function, and only this function**: the same
// `runDispatchLoopUntil (10)` an export turns (ADR 0008 §3), here for as long as the stream is
// open. The device pulls blocks on a thread of its own (ALSA's, in JUCE), gRPC reads and writes
// on threads of its own, and everything between them that touches Tracktion — building the edit,
// moving the transport, reading where it is — happens here, between turns (ADR 0013 §4, as
// measured in M2 PR 10).
int preview (const juce::File& manifestFile)
{
    const auto manifest = manifestIn (manifestFile);
    if (! manifest.isObject())
        return kBadPlan;

    const juce::ScopedJuceInitialiser_GUI juceInit;
    const TempDir scratch;
    const auto dir = scratchIn (scratch);
    if (dir == juce::File())
        return kRenderFailed;

    pinPanLaw();
    auto owned = std::make_unique<te::Engine> (std::make_unique<Storage> (dir), std::make_unique<te::UIBehaviour>(),
                                               std::make_unique<Behaviour> (true));
    auto& engine = *owned;
    auto& devices = engine.getDeviceManager().deviceManager;
    auto* device = devices.getCurrentAudioDevice();
    if (device == nullptr || device->getActiveOutputChannels().countNumberOfSetBits() == 0)
    {
        // Said after the engine is gone, because its destructor logs a line of its own and the
        // caller reports the *last* lines of stderr: the reason belongs at the end, where it is
        // read, and not behind "Cleaning up temp files..".
        const auto why = noDevice (devices);
        owned.reset();
        return fail (kNoDevice, why);
    }
    // What the device is, for the person reading stderr: trap 3's third reason is that this rate
    // is the machine's and an export's is the plan's. Not under this binary's own name, which
    // `fail` keeps for the lines that say why something did not happen — the ones a caller
    // shows first.
    std::fprintf (stderr, "preview: playing on %s '%s' at %.0f Hz, %d frames a block\n",
                  device->getTypeName().toRawUTF8(), device->getName().toRawUTF8(),
                  device->getCurrentSampleRate(), device->getCurrentBufferSizeSamples());

    PreviewService service;
    int failed = kOk;
    const auto server = announce (service, dir, failed);
    if (server == nullptr)
        return failed;

    for (int idle = 0; ! service.isOpen(); ++idle)
    {
        if (idle >= kIdleIterations)
            return fail (kRenderFailed, "no Preview stream opened");
        juce::MessageManager::getInstance()->runDispatchLoopUntil (10);
    }

    Player player (engine, manifest);
    int code = kOk;
    while (! service.isDone())
    {
        juce::MessageManager::getInstance()->runDispatchLoopUntil (10);
        if (code != kOk)
            continue;  // refused; the handler is on its way out
        auto [commands, closed] = service.take();
        for (const auto& command : commands)
        {
            if (auto refused = player.apply (command); ! refused.ok())
            {
                code = fail (kBadPlan, refused.error_message());
                service.refuse (std::move (refused));
                break;
            }
            service.say (player.event());
        }
        if (code == kOk && ! closed)
            if (const auto moved = player.moved())
                service.say (*moved);
    }

    player.release();
    server->Shutdown();
    server->Wait();
    return code;
}

}  // namespace

int main (int argc, char** argv)
{
    // ADR 0009 §3: FTZ and DAZ, set before any thread exists so every thread cloned from this
    // one inherits them (Linux copies the FP environment on clone) — the render thread, and now
    // gRPC's own as well, which is why this is here rather than inside a render. Tracktion sets
    // them again on the render thread in RenderTask::runJob (disableDenormalisedNumberSupport);
    // this is the process's own statement of the same thing.
    _MM_SET_FLUSH_ZERO_MODE (_MM_FLUSH_ZERO_ON);
    _MM_SET_DENORMALS_ZERO_MODE (_MM_DENORMALS_ZERO_ON);

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
    // The mode is which service this process serves (ADR 0013 §3): `--render` registers
    // `Render` alone and `--preview` registers `Preview` alone, so a preview asked of an engine
    // spawned to export is refused by gRPC rather than by a check somebody remembered to write,
    // and an export can never be served by the process that is playing. An engine with no mode
    // serves nothing rather than guessing at one.
    if (argc == 3)
    {
        const auto manifest = juce::File::getCurrentWorkingDirectory().getChildFile (juce::String::fromUTF8 (argv[1]));
        if (std::string_view (argv[2]) == "--render")
            return serve (manifest);
        if (std::string_view (argv[2]) == "--preview")
            return preview (manifest);
    }
    return fail (kBadPlan, "usage: escribass_engine <manifest.json> (--render | --preview)"
                           "\n       escribass_engine [--version | --scan <manifest.json> <component> <plugin.vst3> ...]");
}
