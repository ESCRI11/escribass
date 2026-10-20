// m5.1-spike. Built like bare_airwindows.cpp. Prints the registry's size, the two indices the
// processor's own nParams filter moves, and the alphabetical ordering the filter leaves alone.
#include "AirwinRegistry.h"
#include <cstdio>
int main()
{
    printf ("registered (ModuleAdd.h order): %zu\n", AirwinRegistry::registry.size());
    int gi = AirwinRegistry::nameToIndex.at ("Galactic"), ci = AirwinRegistry::nameToIndex.at ("Channel5");
    printf ("  Galactic index=%d ordering=%d   Channel5 index=%d ordering=%d\n",
            gi, AirwinRegistry::registry[gi].ordering, ci, AirwinRegistry::registry[ci].ordering);
    printf ("  first five by registry order: ");
    for (int i = 0; i < 5; ++i) printf ("%d=%s ", i, AirwinRegistry::registry[i].name.c_str());
    printf ("\n  with nParams > 10: ");
    int over = 0, maxp = 0;
    for (auto& r : AirwinRegistry::registry) { if (r.nParams > 10) ++over; if (r.nParams > maxp) maxp = r.nParams; }
    printf ("%d (max nParams = %d)\n", over, maxp);
    AirwinRegistry::filterAndRebuildRegistry ([] (auto& r) { return r.nParams > 10; });
    printf ("after the processor's own filter: %zu\n", AirwinRegistry::registry.size());
    gi = AirwinRegistry::nameToIndex.at ("Galactic"); ci = AirwinRegistry::nameToIndex.at ("Channel5");
    printf ("  Galactic index=%d ordering=%d   Channel5 index=%d ordering=%d\n",
            gi, AirwinRegistry::registry[gi].ordering, ci, AirwinRegistry::registry[ci].ordering);
    printf ("categories: %zu\n", AirwinRegistry::categories.size());
    return 0;
}
