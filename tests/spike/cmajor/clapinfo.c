/* m5.1-spike: what an exported .clap declares — descriptor, ports and every parameter's CLAP id.
 *   clapinfo <plugin.clap> */
#include <clap/clap.h>
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
static const void *ext (const clap_host_t *h, const char *id) { (void) h; (void) id; return NULL; }
static void noop (const clap_host_t *h) { (void) h; }
int main (int argc, char **argv)
{
    if (argc != 2) { fprintf (stderr, "usage: clapinfo <plugin.clap>\n"); return 2; }
    void *lib = dlopen (argv[1], RTLD_NOW | RTLD_LOCAL);
    if (! lib) { fprintf (stderr, "dlopen: %s\n", dlerror()); return 3; }
    const clap_plugin_entry_t *entry = dlsym (lib, "clap_entry");
    if (! entry || ! entry->init (argv[1])) { fprintf (stderr, "entry\n"); return 4; }
    const clap_plugin_factory_t *fac = entry->get_factory (CLAP_PLUGIN_FACTORY_ID);
    const clap_plugin_descriptor_t *d = fac->get_plugin_descriptor (fac, 0);
    printf ("clap id=%s name=%s vendor=%s version=%s features=", d->id, d->name, d->vendor, d->version);
    for (const char *const *f = d->features; f && *f; ++f) printf ("%s ", *f);
    printf ("\n");
    clap_host_t host = { CLAP_VERSION, NULL, "spike", "spike", "", "0", ext, noop, noop, noop };
    const clap_plugin_t *p = fac->create_plugin (fac, &host, d->id);
    if (! p || ! p->init (p)) { fprintf (stderr, "init\n"); return 5; }
    const clap_plugin_audio_ports_t *ap = p->get_extension (p, CLAP_EXT_AUDIO_PORTS);
    const clap_plugin_note_ports_t *np = p->get_extension (p, CLAP_EXT_NOTE_PORTS);
    for (int in = 1; in >= 0; --in)
    {
        for (uint32_t i = 0; ap && i < ap->count (p, in); ++i)
        { clap_audio_port_info_t a; ap->get (p, i, in, &a); printf ("audio %s id=%u '%s' channels=%u\n", in ? "in" : "out", a.id, a.name, a.channel_count); }
        for (uint32_t i = 0; np && i < np->count (p, in); ++i)
        { clap_note_port_info_t n; np->get (p, i, in, &n); printf ("note %s id=%u '%s'\n", in ? "in" : "out", n.id, n.name); }
    }
    const clap_plugin_params_t *pp = p->get_extension (p, CLAP_EXT_PARAMS);
    for (uint32_t i = 0; pp && i < pp->count (p); ++i)
    {
        clap_param_info_t pi; memset (&pi, 0, sizeof pi);
        pp->get_info (p, i, &pi);
        double v = 0; pp->get_value (p, pi.id, &v);
        printf ("param %u id=%u name='%s' module='%s' min=%g max=%g default=%g value=%g flags=0x%x\n",
                i, pi.id, pi.name, pi.module, pi.min_value, pi.max_value, pi.default_value, v, pi.flags);
    }
    p->destroy (p);
    entry->deinit();
    return 0;
}
