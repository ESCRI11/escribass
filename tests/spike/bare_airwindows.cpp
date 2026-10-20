// m5.1-spike. Build: g++ -O2 -march=x86-64 -mtune=generic -ffp-contract=off -std=c++17
//   -I<airwin2rack>/src -o bare tests/spike/bare_airwindows.cpp libairwin-registry.a libawdoc_resources.a
// Run:   bare <EffectName> <instances> <which-to-hash>
// No JUCE. Constructs Airwindows effects straight out of the registry and hashes the output,
// to decide whether the per-process variation lives in the effect or in the wrapper.
#include "AirwinRegistry.h"
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

static uint64_t fnv (const void* p, size_t n)
{
    auto* b = (const unsigned char*) p;
    uint64_t h = 1469598103934665603ull;
    for (size_t i = 0; i < n; ++i) { h ^= b[i]; h *= 1099511628211ull; }
    return h;
}
struct Noise { uint32_t s; float next() { s ^= s << 13; s ^= s >> 17; s ^= s << 5;
    return (float) (((double) s / 4294967295.0 - 0.5) * 0.5); } };

int main (int argc, char** argv)
{
    const std::string name = argc > 1 ? argv[1] : "Galactic";
    const int instances = argc > 2 ? atoi (argv[2]) : 1;
    const int which = argc > 3 ? atoi (argv[3]) : 0;      // which instance's output to hash
    const bool zeroTails = argc > 4 && std::string (argv[4]) == "zerotails";
    AirwinConsolidatedBase::defaultSampleRate = 48000;

    auto it = AirwinRegistry::nameToIndex.find (name);
    if (it == AirwinRegistry::nameToIndex.end()) { printf ("no such effect %s\n", name.c_str()); return 1; }
    const auto& reg = AirwinRegistry::registry[it->second];
    printf ("effect=%s registryIndex=%d nParams=%d ordering=%d category=%s\n",
            reg.name.c_str(), it->second, reg.nParams, reg.ordering, reg.category.c_str());

    std::vector<std::unique_ptr<AirwinConsolidatedBase>> fx;
    for (int i = 0; i < instances; ++i)
    {
        fx.push_back (reg.generator());
        fx.back()->setSampleRate (48000.f);
        if (zeroTails)
        {
            // Every Airwindows delay line is declared one element longer than the constructor
            // initialises. Nothing reads them at these parameter values, but zeroing the whole
            // object's tail is the cheapest way to ask whether they matter.
        }
    }
    auto* e = fx[(size_t) which].get();
    for (int p = 0; p < reg.nParams; ++p)
        e->setParameter (p, p == 4 ? 1.0f : 0.5f);

    const int block = 512, blocks = 94;
    std::vector<float> l (block), r (block), ol (block), orr (block), out;
    Noise nl { 0x12345678u }, nr { 0x9e3779b9u };
    float* in[2] = { l.data(), r.data() };
    float* op[2] = { ol.data(), orr.data() };
    for (int b = 0; b < blocks; ++b)
    {
        for (int n = 0; n < block; ++n) { l[(size_t) n] = nl.next(); r[(size_t) n] = nr.next(); }
        e->processReplacing (in, op, block);
        for (int n = 0; n < block; ++n) { out.push_back (ol[(size_t) n]); out.push_back (orr[(size_t) n]); }
    }
    double peak = 0, energy = 0;
    for (auto v : out) { if (fabs (v) > peak) peak = fabs (v); energy += (double) v * v; }
    printf ("instances=%d hashed=%d fnv=%016lx peak=%.6g rms=%.6g first=%.9g %.9g\n",
            instances, which, (unsigned long) fnv (out.data(), out.size() * sizeof (float)),
            peak, sqrt (energy / out.size()), out[0], out[1]);
    return 0;
}
