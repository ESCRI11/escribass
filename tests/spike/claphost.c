/* M4 PR 0 spike: the smallest CLAP host that can render a plugin offline.
 *
 * On `m4.0-spike`, which is never merged.  Measurements 4, 5 and 7 are taken
 * with it: nothing in the tree could render an exported Cmajor plugin, and the
 * engine's --scan opens one but does not play it.  Build:
 *
 *   gcc -O2 -march=x86-64 -mtune=generic -ffp-contract=off \
 *       -I<clap-1.2.10>/include -o claphost tests/spike/claphost.c -ldl
 *
 * It writes raw interleaved float32 and no container, so there is no header to
 * carry a date -- M1's `bext` lesson, avoided rather than worked around.
 *
 *   claphost <plugin.clap> <frames> <blocksize> <out.f32>
 */
#include <clap/clap.h>
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const void *host_get_extension (const clap_host_t *h, const char *id) { (void) h; (void) id; return NULL; }
static void host_noop (const clap_host_t *h) { (void) h; }

int main (int argc, char **argv)
{
    if (argc != 5) { fprintf (stderr, "usage: claphost <plugin.clap> <frames> <block> <out.f32>\n"); return 2; }
    const char *path = argv[1];
    const uint32_t frames = (uint32_t) atoi (argv[2]);
    const uint32_t block  = (uint32_t) atoi (argv[3]);

    void *lib = dlopen (path, RTLD_NOW | RTLD_LOCAL);
    if (! lib) { fprintf (stderr, "dlopen: %s\n", dlerror()); return 3; }
    const clap_plugin_entry_t *entry = (const clap_plugin_entry_t *) dlsym (lib, "clap_entry");
    if (! entry) { fprintf (stderr, "no clap_entry\n"); return 4; }
    if (! entry->init (path)) { fprintf (stderr, "entry->init failed\n"); return 5; }

    const clap_plugin_factory_t *fac = (const clap_plugin_factory_t *) entry->get_factory (CLAP_PLUGIN_FACTORY_ID);
    if (! fac || fac->get_plugin_count (fac) == 0) { fprintf (stderr, "no factory/plugins\n"); return 6; }
    const clap_plugin_descriptor_t *desc = fac->get_plugin_descriptor (fac, 0);
    fprintf (stderr, "plugin: id=%s name=%s vendor=%s version=%s\n", desc->id, desc->name, desc->vendor, desc->version);

    clap_host_t host = {0};
    host.clap_version = CLAP_VERSION;
    host.name = "escribass-spike"; host.vendor = "escribass"; host.url = ""; host.version = "0";
    host.get_extension = host_get_extension;
    host.request_restart = host_noop; host.request_process = host_noop; host.request_callback = host_noop;

    const clap_plugin_t *p = fac->create_plugin (fac, &host, desc->id);
    if (! p || ! p->init (p)) { fprintf (stderr, "create/init failed\n"); return 7; }

    const clap_plugin_audio_ports_t *ports =
        (const clap_plugin_audio_ports_t *) p->get_extension (p, CLAP_EXT_AUDIO_PORTS);
    uint32_t nch = 2;
    if (ports && ports->count (p, false) > 0)
    {
        clap_audio_port_info_t info; memset (&info, 0, sizeof info);
        if (ports->get (p, 0, false, &info)) nch = info.channel_count;
    }
    uint32_t nin = 0, inch = 0;
    if (ports && ports->count (p, true) > 0)
    {
        clap_audio_port_info_t info; memset (&info, 0, sizeof info);
        if (ports->get (p, 0, true, &info)) { nin = 1; inch = info.channel_count; }
    }
    fprintf (stderr, "ports: in=%u (ch %u)  out ch=%u\n", nin, inch, nch);

    const clap_plugin_params_t *params = (const clap_plugin_params_t *) p->get_extension (p, CLAP_EXT_PARAMS);
    if (params)
        for (uint32_t i = 0; i < params->count (p); ++i)
        {
            clap_param_info_t pi; memset (&pi, 0, sizeof pi);
            if (params->get_info (p, i, &pi))
                fprintf (stderr, "param[%u] id=%u name=\"%s\" min=%g max=%g default=%g\n",
                         i, (unsigned) pi.id, pi.name, pi.min_value, pi.max_value, pi.default_value);
        }

    if (! p->activate (p, 48000.0, block, block)) { fprintf (stderr, "activate failed\n"); return 8; }
    if (! p->start_processing (p)) { fprintf (stderr, "start_processing failed\n"); return 9; }

    float **chans = calloc (nch, sizeof (float *));
    for (uint32_t c = 0; c < nch; ++c) chans[c] = calloc (block, sizeof (float));
    clap_audio_buffer_t out = {0};
    out.data32 = chans; out.channel_count = nch;

    FILE *f = fopen (argv[4], "wb");
    if (! f) { perror ("fopen"); return 10; }

    float **inchans = NULL;
    clap_audio_buffer_t in = {0};
    if (nin)
    {
        inchans = calloc (inch ? inch : 1, sizeof (float *));
        for (uint32_t c = 0; c < inch; ++c) inchans[c] = calloc (block, sizeof (float));
        in.data32 = inchans; in.channel_count = inch;
    }

    clap_process_t pr = {0};
    pr.audio_outputs = &out; pr.audio_outputs_count = 1;
    pr.audio_inputs = nin ? &in : NULL;  pr.audio_inputs_count = nin;
    pr.in_events = NULL; pr.out_events = NULL;
    pr.steady_time = 0;

    extern const clap_input_events_t  g_in;
    extern const clap_output_events_t g_out;
    pr.in_events = &g_in; pr.out_events = &g_out;

    for (uint32_t done = 0; done < frames; done += block)
    {
        uint32_t n = (frames - done < block) ? frames - done : block;
        pr.frames_count = n;
        pr.steady_time = (int64_t) done;
        for (uint32_t c = 0; c < nch; ++c) memset (chans[c], 0, block * sizeof (float));
        clap_process_status st = p->process (p, &pr);
        if (st == CLAP_PROCESS_ERROR) { fprintf (stderr, "process error at %u\n", done); return 11; }
        for (uint32_t i = 0; i < n; ++i)
            for (uint32_t c = 0; c < nch; ++c)
                fwrite (&chans[c][i], 4, 1, f);
    }

    fclose (f);
    p->stop_processing (p); p->deactivate (p); p->destroy (p);
    entry->deinit();
    fprintf (stderr, "rendered %u frames, %u channels\n", frames, nch);
    return 0;
}

static uint32_t in_size (const clap_input_events_t *l) { (void) l; return 0; }
static const clap_event_header_t *in_get (const clap_input_events_t *l, uint32_t i) { (void) l; (void) i; return NULL; }
static bool out_try_push (const clap_output_events_t *l, const clap_event_header_t *e) { (void) l; (void) e; return true; }
const clap_input_events_t  g_in  = { NULL, in_size, in_get };
const clap_output_events_t g_out = { NULL, out_try_push };
