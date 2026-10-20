/* m5.1-spike. Build: gcc -shared -fPIC -O1 -o shim.so tests/spike/srand_shim.c -ldl
   Run:   LD_PRELOAD=./shim.so <anything>   */
/* Logs every srand()/rand() the process makes, so the reseed can be named rather than inferred. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
static long nrand = 0;
void srand (unsigned s) {
    static void (*real)(unsigned);
    if (!real) real = dlsym (RTLD_NEXT, "srand");
    fprintf (stderr, "[shim] srand(%u = 0x%x) after %ld rand() calls\n", s, s, nrand);
    nrand = 0;
    real (s);
}
int rand (void) {
    static int (*real)(void);
    if (!real) real = dlsym (RTLD_NEXT, "rand");
    ++nrand;
    return real();
}
