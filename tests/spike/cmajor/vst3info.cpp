// m5.1-spike: what a VST3 module itself declares — class ids and every parameter's ParamID,
// read through the VST3 SDK's own interfaces rather than through JUCE's view of them.
//   vst3info <bundle.vst3>
#include "pluginterfaces/base/funknown.h"
#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivsthostapplication.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstmessage.h"
#include <dlfcn.h>
#include <filesystem>
#include <cstdio>
#include <string>
using namespace Steinberg;

static std::string u8 (const Vst::TChar* s)
{
    std::string r;
    for (; s && *s; ++s) r += (*s < 128 ? (char) *s : '?');
    return r;
}
static std::string hex (const TUID id)
{
    char b[33];
    for (int i = 0; i < 16; ++i) std::snprintf (b + 2 * i, 3, "%02X", (unsigned char) id[i]);
    return b;
}

struct Host : Vst::IHostApplication
{
    tresult PLUGIN_API getName (Vst::String128 name) override
    {
        const char* n = "spike";
        int i = 0;
        for (; n[i]; ++i) name[i] = n[i];
        name[i] = 0;
        return kResultOk;
    }
    tresult PLUGIN_API createInstance (TUID, TUID, void** obj) override { *obj = nullptr; return kNotImplemented; }
    tresult PLUGIN_API queryInterface (const TUID iid, void** obj) override
    {
        if (FUnknownPrivate::iidEqual (iid, FUnknown::iid) || FUnknownPrivate::iidEqual (iid, Vst::IHostApplication::iid))
        { *obj = this; return kResultOk; }
        *obj = nullptr;
        return kNoInterface;
    }
    uint32 PLUGIN_API addRef() override { return 1; }
    uint32 PLUGIN_API release() override { return 1; }
};

int main (int argc, char** argv)
{
    if (argc != 2) { std::fprintf (stderr, "usage: vst3info <bundle.vst3>\n"); return 2; }
    namespace fs = std::filesystem;
    fs::path so;
    for (auto& e : fs::directory_iterator (fs::path (argv[1]) / "Contents" / "x86_64-linux"))
        if (e.path().extension() == ".so") so = e.path();
    void* h = dlopen (so.c_str(), RTLD_NOW | RTLD_LOCAL);
    if (! h) { std::fprintf (stderr, "dlopen: %s\n", dlerror()); return 3; }
    using Entry = bool (*) (void*);
    if (auto entry = (Entry) dlsym (h, "ModuleEntry"); entry && ! entry (h)) { std::fprintf (stderr, "ModuleEntry failed\n"); return 4; }
    using GetFactory = IPluginFactory* (*)();
    auto getFactory = (GetFactory) dlsym (h, "GetPluginFactory");
    if (! getFactory) { std::fprintf (stderr, "no GetPluginFactory\n"); return 5; }
    IPluginFactory* f = getFactory();
    PFactoryInfo fi;
    f->getFactoryInfo (&fi);
    std::printf ("factory vendor=%s url=%s flags=%d\n", fi.vendor, fi.url, fi.flags);
    IPluginFactory2* f2 = nullptr;
    f->queryInterface (IPluginFactory2::iid, (void**) &f2);
    Host host;
    for (int32 c = 0; c < f->countClasses(); ++c)
    {
        PClassInfo ci;
        f->getClassInfo (c, &ci);
        std::printf ("class %d cid=%s category='%s' name='%s'", c, hex (ci.cid).c_str(), ci.category, ci.name);
        if (f2) { PClassInfo2 c2; f2->getClassInfo2 (c, &c2); std::printf (" sub='%s' vendor='%s' version='%s'", c2.subCategories, c2.vendor, c2.version); }
        std::printf ("\n");
        if (std::string (ci.category) != kVstAudioEffectClass) continue;
        Vst::IComponent* comp = nullptr;
        if (f->createInstance (ci.cid, Vst::IComponent::iid, (void**) &comp) != kResultOk || ! comp) { std::printf ("  createInstance failed\n"); continue; }
        if (comp->initialize (&host) != kResultOk) std::printf ("  component initialize != ok\n");
        for (auto dir : { Vst::kInput, Vst::kOutput })
            for (auto type : { Vst::kAudio, Vst::kEvent })
                for (int32 b = 0; b < comp->getBusCount (type, dir); ++b)
                {
                    Vst::BusInfo bi;
                    comp->getBusInfo (type, dir, b, bi);
                    std::printf ("  bus %s %s %d '%s' channels=%d\n", dir == Vst::kInput ? "in" : "out", type == Vst::kAudio ? "audio" : "event", b, u8 (bi.name).c_str(), bi.channelCount);
                }
        Vst::IEditController* ec = nullptr;
        bool separate = false;
        if (comp->queryInterface (Vst::IEditController::iid, (void**) &ec) != kResultOk || ! ec)
        {
            TUID ctl;
            if (comp->getControllerClassId (ctl) == kResultOk && f->createInstance (ctl, Vst::IEditController::iid, (void**) &ec) == kResultOk && ec)
            {
                separate = true;
                ec->initialize (&host);
                std::printf ("  controller cid=%s (separate)\n", hex (ctl).c_str());
                // As a host does: connect the two halves, so a controller that learns its
                // parameters from its component (JUCE's) has them to report.
                Vst::IConnectionPoint *a = nullptr, *b = nullptr;
                comp->queryInterface (Vst::IConnectionPoint::iid, (void**) &a);
                ec->queryInterface (Vst::IConnectionPoint::iid, (void**) &b);
                if (a && b) { a->connect (b); b->connect (a); }
            }
        }
        if (! ec) { std::printf ("  no edit controller\n"); continue; }
        if (! separate) std::printf ("  controller: single component\n");
        for (int32 i = 0; i < ec->getParameterCount(); ++i)
        {
            Vst::ParameterInfo pi;
            ec->getParameterInfo (i, pi);
            Vst::String128 s0, s1;
            ec->getParamStringByValue (pi.id, 0.0, s0);
            ec->getParamStringByValue (pi.id, 1.0, s1);
            std::printf ("  param %d id=%u title='%s' short='%s' units='%s' steps=%d default=%.9g flags=0x%x unit=%d plain[0..1]=[%.9g, %.9g] text[0..1]=['%s', '%s']\n",
                         i, pi.id, u8 (pi.title).c_str(), u8 (pi.shortTitle).c_str(), u8 (pi.units).c_str(), pi.stepCount,
                         pi.defaultNormalizedValue, pi.flags, pi.unitId, ec->normalizedParamToPlain (pi.id, 0.0), ec->normalizedParamToPlain (pi.id, 1.0),
                         u8 (s0).c_str(), u8 (s1).c_str());
        }
    }
    return 0;
}
