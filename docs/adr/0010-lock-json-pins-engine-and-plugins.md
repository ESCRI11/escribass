# ADR 0010 — `lock.json` pins the engine and every referenced plugin, added on first reference and compared at load

- **Status:** Accepted (2026-09-05)
- **Affects:** `core/src/project.rs` (`Lock`), `core/src/validate.rs`; `engine/` (the build
  manifest, M1); `tests/determinism/*` goldens; `docs/specs.md` §8, §11, §17
- **Builds on:** ADR 0003 §3 (`lock.json` created at M0, completed later), ADR 0004 §1 and §2
  (a derived file, and a mismatch is a structured error), ADR 0006 §2 (caller vs operator),
  ADR 0007 §6 (validity and renderability are different questions), ADR 0008 §4 (an artefact
  derived from a pin is regenerated, never committed) and §5 (the engine reports its own
  provenance)
- **Recorded in:** `docs/specs.md` §8, §11, §15 and §17.

## Context

`lock.json` exists and holds one field. `core/src/project.rs` says so in a comment — "one field
until there is something else to pin — plugins, models and compiled artefacts arrive at M1 and
M4" — and ADR 0003 §3 and §17 say the same thing from the other side. §11 requires "all
external tool versions recorded in `lock.json` and checked at load", which today is satisfied
vacuously by `schema_version`.

M1 is the milestone that gives it something to pin, and the same milestone closes two limits
`core` has carried since M0.2 with the same revisit point. `core/src/validate.rs` opens by
naming them: resolving a `DeviceRef` to a *pinned* plugin, and resolving a `ParamRef` to a
*real* parameter of it, "needs the plugin's manifest, which arrives with the engine (M1)".
`core/AGENTS.md` repeats it: "Two limits are structural, not oversights."

They are one question, not three. §4.4's "every `InstrumentRef`/`EffectRef` resolves to a known
plugin with pinned version" needs to know what plugins exist and what they are called;
§4.4's "automation targets resolve to real parameters" needs to know what parameters they
have; and §11's load check needs to know what was pinned last time. All three want the same
artefact — a description of what this build can actually host — and M1 is where it first
exists.

What the documents leave open is the shape of the file, when an entry appears in it, and what
happens when it disagrees with the build. The last of those is the one with a wrong answer that
looks kind.

## Decisions

### 1. `lock.json` v2: the engine's commits, and one entry per referenced plugin

```json
{
  "schema_version": 1,
  "engine": {
    "juce": "37c894f83d379179b2070d437ccd0f1cd9af9576",
    "rubberband": "1d95888bec3ae0a17c0c4af791810d5a63f6bc35",
    "tracktion_engine": "0e02f709c4088b2aec427ba6bbbfee3639139bb9"
  },
  "plugins": {
    "com.surge-synth.surge-xt": {
      "commit": "f7b97c682ade0b87da85ca5968b63d5c7c98e68d",
      "version": "1.3.4"
    }
  }
}
```

Keys are sorted and the file stays pretty-printed JSON, because M0.4 byte-compares it across
two processes and two transports; a map with a nondeterministic order would fail that suite
rather than this one.

`lock.json` is **not** a protobuf message and does not become one. It is not song state — it
describes the build a song was authored against — so CLAUDE.md #1 is not in play, and making it
a proto would put a message in `song.proto` that no renderer needs, which §14.7 says does not
belong in the model.

Three things are deliberately absent:

- **No filesystem paths.** Where a plugin binary lives is machine-specific, and `lock.json` is
  committed to the user's git repository (§2.6) and byte-compared by the determinism suite. A
  path would make the file differ between two machines that pin exactly the same thing.
- **No parameter lists.** They are large, derived from the plugin binary, and belong to the
  manifest (decision 4), which is regenerated rather than stored.
- **No entry for `SourceRef`, `ModelRef` or `SamplerRef`.** Those already reference content by
  hash — the hash *is* the pin, and the asset is either in `assets/` or reported missing by
  §10's existing rule. M4 adds the compiled artefacts ADR 0003 §3 named; M1 adds only what M1
  can host.

The `engine` block holds submodule commits rather than a version string because that is what
ADR 0008 §5 has the engine report about itself, and a comparison between two things is easier
when both are the same thing.

### 2. A pin is written on first reference, never removed, and by no tool

A pin appears the first time a song references the plugin it pins. The mechanism is not a new
one: `Project::write` recomputes the `plugins` block on every write as *the pins that already
exist, plus an entry from the build manifest for every referenced `plugin_id` that has none*.
Existing entries are never rewritten and never dropped.

That makes it a derived file in exactly ADR 0004 §1's sense — `song.json` is derived from the
log, and `lock.json` is derived from the song, the prior lock and the build manifest — rather
than a document anyone authors. It is worth being precise about why this is not a mutation
sneaking past CLAUDE.md #2: **the mutation is the reference**, it is made by a tool
(`set_track_instrument`, `add_effect`, `apply_patch`), and it is in the patch log like every
other mutation. No entry can appear in `lock.json` that no logged op caused, and no tool writes
the file.

**Never removed** is the non-obvious half. The tempting rule is that the block is a pure
function of the current song, so deleting the last track using a plugin drops its pin. That
rule loses a pin through an ordinary edit, and undo makes it worse: ADR 0005 §4's undo appends
an *inverse* entry, which re-adds the reference, which would re-pin from whatever build is
running now. On a newer build that is a silent re-pin performed by pressing undo — precisely
the side effect decision 3 exists to prevent. Monotone is one sentence and has no such hole.

The cost of monotone is a stale pin: a project that once used Surge and no longer does still
carries the entry. That is why **the load check reads only the pins the song currently
references** (decision 3). An inert pin is a record of history, not a hostage.

### 3. A referenced plugin this build cannot match refuses to open, with `lock_mismatch`

`open` compares, for every `plugin_id` the song references, the project's pin against the build
manifest, and compares the `engine` block against the build's. Any disagreement — the plugin is
absent, or present at a different commit — is `lock_mismatch`, returned as `ProjectError`, on
the **operator** side of ADR 0006 §2's line. Nothing opens; nothing renders; nothing is
substituted.

This is the strict reading of §11, and it is the same shape as `schema_version_mismatch`, which
`Project::open` already implements one field over: a project written against a build this one
is not compatible with is refused, not guessed at. ADR 0004 §2 made the same call for a
`song.json` that disagrees with its log — "a mismatch is a structured error, never silently
resolved either way" — and the reasoning transfers unchanged. It is an operator error because
the fixes are all operator actions: install the build that matches, rebuild, or re-pin
deliberately. A model retrying a tool call cannot produce any of them, so putting it inside
§6's retry loop would only spend retries.

The lenient alternatives are each worse in a specific way:

| Lenient behaviour | What it produces |
|---|---|
| Open, and render silence for the missing plugin | A render that completes, sounds wrong, and hashes differently from its golden with no error anywhere. §10 already refuses this shape for assets: "missing assets are reported by the validator, never silently substituted". |
| Open, and substitute a similar plugin | The same, plus a guess about musical intent made by a version check. |
| Open, and re-pin to whatever this build has | Destroys the record of what the project was authored against, which is the only thing that makes §2.2's claim checkable. The first symptom is a golden that changed and a `lock.json` that says it did not. |
| Warn and continue | A warning in a process driven by an LLM over MCP is a line of text nothing reads. §11's "checked at load" then means "mentioned at load". |

Re-pinning is therefore always explicit. **M1 does not ship a tool for it**: the operator edits
`lock.json`, which is text §2.6 requires be readable, and the next `open` compares against the
new value. A `repin` tool waits for M2, where a UI can show what changes and ask — designing it
now would be designing against no consumer, which is ADR 0002 §7's reason for deferring
`FormRule` and the plan's for deferring `Instrument.state`.

**The engine block is the exception: it re-pins on open.** Everything above is about a
plugin, and a plugin pin fails in a way the engine pin cannot. A referenced plugin this build
lacks makes the song *unrenderable* — there is nothing to put in the chain — and which plugins
exist is a property of the installation that the project legitimately constrains. A newer
engine hosting the same plugins renders the same song fine; what an upgrade puts at risk is
bit-exactness against a golden, not whether the project opens.

Refusing on the engine block would also aim the error at the wrong thing. There is exactly one
engine and a project cannot choose it, so an upgrade would refuse *every* project on the
machine at once — which is not an operator error about any of them, and leaves the operator
hand-editing `lock.json` in every project directory to say what the installer already knows.
So `open` writes the running build's engine commits into the block and continues.

That does not lose the record the lenient table objects to losing, because the engine block was
never where it lived. ADR 0008 §5 has the engine embed its submodule commits and report them in
`RenderResult`: what a given render was made with travels with that render, which is where the
question is actually asked. §17's "a full golden-render pass" obligation stays on the pull
request that moves the pin — CI, where the goldens are — rather than being collected from users
one project at a time.

A plugin entry is still never rewritten (decision 2), so this is one block behaving differently
for a stated reason, not a general softening.

### 4. M1 produces a build manifest; `plugin_unknown` and `param_unknown` become validator rules

The engine build emits **one manifest** describing what this build can host, shipped beside the
engine binary:

- the `engine` block of decision 1, from the submodule commits ADR 0008 §5 embeds;
- for each bundled plugin: its `plugin_id` as the plugin itself reports it, its version, the
  vendored commit, the path to its binary, and its **parameter list** — the identifiers a
  `ParamRef.param` and an `Instrument.params` key must match.

**Refined 2026-09-06, in PR 6, which built the first manifest.** This section assumed a VST3
reports one id and one name per parameter. It reports neither in a form that does both jobs, and
the manifest's shape follows what is there rather than what was assumed:

- `plugin_id` is `"<vendor>/<class name>"` as the VST3 factory reports both — `Surge Synth
  Team/Surge XT`, `SFZTools/sfizz`, `SFZTools/sfizz-multi`, `Digital Suburban/Dexed`. JUCE's own
  `PluginDescription::createIdentifierString()` is not usable: it hashes the plugin's **path**
  into the string, and decision 1 keeps `lock.json` free of anything machine-specific. Three
  files give four entries, because sfizz-ui's bundle declares two audio classes; the manifest
  describes what can be hosted, not what was built.
- the **parameter list is a map from the plugin's own parameter id to its display name**, and
  the id is the identifier a `ParamRef.param` must match. Names alone cannot be: Surge XT
  repeats 176 of its 2855, one per unassigned effect slot. Ids alone would put opaque integers
  in a file §2.6 wants readable, so the name travels beside its id as the label an editor and a
  model read.
- there is no third option. A VST3 host is shown a numeric `ParamID` and a display string; the
  readable identifier a JUCE plugin uses internally is hashed away by JUCE's own VST3 wrapper
  before a host can see it.

**Extended 2026-09-06, in PR 7, which first set a parameter.** The same fact settles the other
half of what a `ParamRef` means, which this section had left open by talking only about the
key: a parameter's **value** — in `Instrument.params`, in `Effect.params` and on every
`AutomationPoint` — is the plugin's *normalised* value, `0.0` to `1.0`. That is not a choice
either. VST3 exposes exactly one numeric domain to a host, `Vst::ParamValue`, and it is
normalised; JUCE passes it through unchanged and a plugin's own units exist only as the display
string beside it. So `param_unknown` (below) has a companion the validator can also check — a
value outside `0..1` — and the engine clamps rather than refuses, because Tracktion's parameter
range clamps it either way and a caller error is not the engine's to discover (ADR 0008 §1).

It is **generated at build time and never committed**, for ADR 0008 §4's reason applied to a
different artefact: a committed manifest is a description of a plugin binary that must agree
with the plugin binary, which is two pins for one fact, and the stale one is silent. It is also
what ADR 0008 §2 needs so a fresh process can skip the VST3 scan: the engine opens the exact
paths the manifest names.

`core` reads it. The validator takes the manifest as a **constructor argument**, not an
`Option`, and the binaries refuse to start without one (`manifest_missing`, operator error).
An `Option` would give the two rules a "skip if absent" arm, and a rule that skips silently is
the quiet failure M0.4 was built to prevent — the suite would pass on a machine with no engine
and prove nothing.

Two rules follow, both caller-fixable and both returning `Ok(valid = false)` per ADR 0006 §2:

- **`plugin_unknown`** — a `DeviceRef.plugin.plugin_id` no manifest entry declares. This is
  what §4.4's "resolves to a known plugin with pinned version" has meant all along, and it is
  what ADR 0007 §6 relies on when it says a plugin outside the bundled set never reaches
  `compile`.
- **`param_unknown`** — a `ParamRef.param`, or a key of `Instrument.params`/`Effect.params`,
  that the referenced device's plugin does not declare. This closes §4.4's "automation targets
  resolve to real parameters", and it upgrades the existing `device_unknown` check, which
  today verifies only that the device exists in this song.

Neither rule refuses anything ADR 0007 §6 refuses, and the division there holds: `compile`
refuses what this engine cannot *render*, and the validator refuses what does not *resolve*.
A Cmajor device is valid and unrenderable in M1; a plugin id nothing declares is neither.

### 5. Airwindows is deferred to M4, with clap-wrapper. M1 bundles three synths

ADR 0003 §4 put "the remaining bundled instruments" in M1 on the strength of §8's candidate
list — Surge XT, sfizz, Dexed, Airwindows — and §11's golden per bundled instrument. Three of
those four are instruments. Airwindows is a set of effects, and its pinned repository may not
build a Linux VST3 at all.

So M1 bundles **Surge XT, sfizz and Dexed**, and §11's "a golden per bundled instrument" is met
in full by them. Airwindows moves to **M4**, which is where clap-wrapper is already scheduled
(§7.2, ADR 0003 §7) and where a CLAP-to-VST3 projection makes the packaging question a solved
one rather than a build investigation inside the render milestone. **ADR 0003 §4 is amended in
place** to record it.

This is not scope quietly dropped: M1's deliverable was never "four plugins", it was a render
path with a golden per instrument, and it delivers that. What M1 would have lost by keeping
Airwindows is the effect chain having no bundled effect to exercise — which it does not lose,
because `Effect` renders through the same device path as `Instrument` and Surge XT's own
effects exercise it.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| `lock.json` holds every pin in `lock.baseline.json` | The baseline describes the *platform*; a project pins what it *uses*. Copying the whole table makes every project refuse to open after any unrelated upgrade. |
| `lock.json` as a protobuf message | It is not song state, so it would be a message in `song.proto` no renderer needs (§14.7), or a fourth `.proto` for one file with three keys. |
| The plugins block is a pure function of the current song | Loses a pin on an ordinary delete, and undo (ADR 0005 §4) silently re-pins from the running build. Monotone has no such hole. |
| Store the plugin's path in `lock.json` | Machine-specific, in a file that is committed to the user's repository and byte-compared by the determinism suite. |
| Warn on mismatch and open anyway | §11's "checked at load" becomes "mentioned at load"; over MCP the warning has no reader. |
| Auto-repin a *plugin* on open | Destroys the record §2.2's claim is checked against; the symptom is a moved golden and a lock file that says nothing moved. The engine block is re-pinned (decision 3) because its record lives in `RenderResult`, not here. |
| Refuse on the engine block too, for symmetry | One engine, not chosen per project: an upgrade would refuse every project at once and be repaired by hand-editing each. Symmetry between a pin the project constrains and a pin the installation owns is not a property worth having. |
| `lock_mismatch` as a `Violation` (caller-fixable) | Every fix is an operator action — install, rebuild, or edit the pin. Inside §6's retry loop it would only spend retries. |
| Commit the manifest | Two pins for one fact — the manifest and the plugin binary — with the stale one silent. ADR 0008 §4's argument, one artefact over. |
| The validator takes an optional manifest | Gives both rules a silent skip arm, which is the defect M0.4 exists to prevent. |
| Bundle Airwindows in M1 anyway | A build investigation on a repository that may produce no Linux VST3, inside the milestone that has to prove bit-exactness. M4 already has clap-wrapper. |

## Consequences

- **PR 6** produces the manifest and answers what Surge XT, sfizz and Dexed actually report as
  their plugin ids and parameter names. Everything downstream needs those real strings.
- **PR 9** is where this ADR lands in code: `Lock` v2, `lock_mismatch`, `plugin_unknown`,
  `param_unknown`, and the validator's manifest argument. It is the plan's **silent PR** — the
  M0.4 goldens regenerate here and nowhere else, because `lock.json` is one of the files the
  suite byte-compares and it grows a `plugins` block for every script that references one.
- **Trap 13, and one more the trap did not name.** `tests/determinism/*/script.json` uses
  `com.surge-synth.surge-xt`, and `tests/fixtures/song/minimal.json` uses
  `org.surge-synth.surge-xt`. Both are invented; both must become the id the manifest declares,
  or `plugin_unknown` turns `checks` red. They are fixed in PR 9 with the goldens, and the
  spelling difference between them is itself the evidence that neither was ever checked.
- **`core`'s own tests** read a small manifest fixture. A test in the render suite (PR 11,
  behind the render feature, where a built engine exists) asserts the fixture's plugin ids and
  parameter names are a subset of the built manifest's — so the fixture cannot drift back into
  being invented.
- **ADR 0003 §4 is amended in place**; §15 gains a row for the amendment.
- **`docs/plan.md`'s deferred row** "`lock.json` beyond `schema_version`" is closed for its M1
  half. Its M4 half — compiled artefacts and model hashes — is unchanged.
- **§11**'s lock-check line gains what is compared and what happens on a mismatch; **§17**'s
  intro stops saying `lock.json` records only `schema_version`; **§8** records three bundled
  plugins rather than four candidates.
- **M2** gains a re-pin tool and the UI that makes `lock_mismatch` recoverable without a text
  editor. **M4** adds Airwindows and, per ADR 0003 §3, the compiled artefacts.
