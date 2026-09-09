# ADR 0014 — M2 places §9's seventh view and claims one platform

- **Status:** Accepted (2026-09-07)
- **Affects:** `docs/specs.md` §9, §15 and §16 (through ADR 0003); `docs/plan.md`'s deferred
  ledger; `app/` (M2)
- **Builds on:** ADR 0003 (§16 is authoritative for scope; §5 placed the mixer and the history
  view in M2, §8 placed code views in M4); ADR 0009 §1 (bit-exactness is claimed for Linux
  x86-64 on one image and compiler); ADR 0010 §4 (the build manifest maps a `ParamID` to a
  display name, and a parameter's value is normalised `0..1`)
- **Recorded in:** `docs/specs.md` §9 and §15; ADR 0003 §5 amended in place.

## Context

ADR 0003 exists because unplaced scope is invisible scope. It placed eight things §16 had left
in no milestone — and §9's seven views were placed six times. The **instrument and effect
editors** were not: §16 names timeline, piano roll, mixer, history and preview; ADR 0003 §5
placed the mixer and the history view; §8 placed code views in M4; §16 places the AI panel in
M3. The seventh view belongs to nothing.

That is not a tidiness problem. Three rows in `plan.md`'s deferred ledger — `Instrument.state`
as a content hash, user VST3 plugins, and the §2.2 randomness gap — all wait on some version of
*"the first milestone that lets a user choose a patch"*, and none of them can fire while the
milestone that would is unplaced. A trigger pointing at a milestone that does not exist is a
row that will be walked past at every close, which is exactly how these three got here.

The platform question is the mirror image: §1 names macOS, Windows and Linux, M1 claims Linux
x86-64 only, and a UI is the first artefact a user installs on an operating system rather than
builds. §15's `[OPEN]` item — minimum supported OS versions — is squarely in that path and is
not an agent's to resolve.

## Decisions

### 1. The instrument and effect editors are M2, as a generic parameter editor over the build manifest

**Amending ADR 0003 §5**, which placed the mixer and the history view in M2 and read §9's seven
views as six placed and one absent. The seventh is M2's, in the same sentence and for the same
reason: it is a projection of the model that needs nothing beyond M0 and M1.

What M2 builds is the **generic** editor: a form over the build manifest, which already maps a
plugin's numeric `ParamID` to the display name the plugin reports (ADR 0010 §4, refined), whose
values are already normalised `0..1` (ADR 0010 §4, extended), and which `core` already requires
by `--manifest` with no default and no search. Writing a control is `set_param`, which exists.
The marginal cost over the mixer M2 already owns is a second form over a second map, and it is
the smallest of the four views in the milestone.

The reason it is placed rather than deferred is the ledger, not the cost. Leaving it unplaced
leaves three rows with a trigger that can never fire; placing it lets each of them be restated
against what this editor actually does, which decision 3 does — and two of the three turn out
**not** to be M2's after all, for reasons that only become visible once there is a real editor
to compare them against. That is the value of placing scope: it converts a vague trigger into a
specific one.

Its known limit is written down rather than discovered: VST3 exposes exactly one numeric domain
to a host and it is normalised, so the editor shows a number between 0 and 1 beside a name, and
a user cannot read "−6 dB" off it. A plugin *can* render a display string per value, but only a
live instance can, which is the engine — a round trip per pixel. Deferred with a trigger of its
own: when a preview session is already holding an engine process (ADR 0013 §3), asking it to
format a value costs a message rather than a process.

**Extended 2026-09-09, in M2 PR 7, where the editor was built.** This decision said the editor is
a form over the build manifest and did not say how the manifest gets to the form, which turned
out to be the only thing in it that was not already there. `core` reads the manifest and the host
holds it; the webview could reach neither, because ADR 0012 §1's one command dispatches into
`escribass_core::call` and no *tool* answers a question about the running build.

The manifest crosses as a **second Tauri command**, `manifest`, taking no arguments and returning
the `Manifest` `core` parsed — not the file, so the machine-specific plugin paths the engine
writes for its own use never reach a window, and the editor's rows are exactly the keys
`param_unknown` will resolve a `set_param` against. ADR 0012 §1's rule is unchanged and is not
being read loosely: it makes `tool` the webview's one route to *the model*, and this is not the
model. Adding a `get_manifest` **tool** instead was rejected for the opposite reason to the usual
one — it is not that it costs a proto change, but that it would hand the AI a plugin catalogue as
a side effect of drawing a form, which is a decision about §6's surface and not this pull
request's to take.

Two things the editor needed that this decision assumed and did not state. The **`Instrument`
overrides a build cannot name are shown, not dropped**: `Project::open` does not run the
validator, so a project written against a fuller manifest opens against a smaller one — which is
the repository's own default, since `make run` passes the three-parameter fixture — and a value
in the document that no view can see is the silent half of that mismatch. And the rows are
ordered by **`ParamID`, numerically**, not by the manifest's key order: that order is the
plugin's own declaration order and JavaScript does not keep it, because an integer-like object
key is re-sorted numerically ascending by the language before any of our code runs. So the order
a plugin's own window would show is not available to this view at all, which is worth knowing
before anyone tries to restore it.

### 2. M2 targets Linux x86-64, exactly as M1 does

Same platform, same image, same compiler. macOS and Windows are **unclaimed, not contradicted**
— ADR 0009 §1's own formulation, and the one §8 already carries.

Building all three of §1's platforms would triple CI in the milestone that adds the largest new
surface in the repository, and Tauri's webview differs per operating system (WebKitGTK,
WKWebView, WebView2), so "it runs" is three separate answers with three separate rendering
engines behind them. Nothing is postponed by this that was not already postponed: §16 places the
installer in M5, and `roadmap.md` places minimum supported OS versions there with it.

**§15's `[OPEN]` item stays open.** This decision answers M2's scope, which is an agent's to
propose and a human's to accept; it does not answer what the minimum supported version of any
operating system is, which is a product commitment that outlives M2 and belongs to whoever ships
the installer. The item stays in §15's closing paragraph, unchanged, and M2 walks past it rather
than through it.

### 3. The three ledger rows get triggers that can fire

Each was waiting on "the milestone that lets a user choose a patch". That milestone is now
named, so each row is restated against what decision 1's editor actually does — and only by
doing that does it become clear that two of them were waiting on the wrong thing.

- **`Instrument.state` as a content hash.** The generic editor writes `params`, a map from
  `ParamID` to a normalised double. It never writes `state`, which is the plugin's own opaque
  blob and can only come from the plugin's own serialisation. So M2 still has no producer, and
  ADR 0002 §7's reason still holds — an ADR now designs against nothing. New trigger: **the
  first thing that writes an `Instrument.state`**, which is a plugin's own editor or a preset
  import, neither of which is M2's.
- **User VST3 plugins.** The editor is built from the build manifest, and the manifest describes
  what *this build* hosts. M1 refuses a plugin outside it (`plugin_unknown`, `lock_mismatch`) and
  M2 does not change that. A plugin browser is not a view over the manifest; it is a scanner that
  puts things *into* one, and it drags `lock.json` with it, since a user's plugin has no
  submodule commit to pin. New trigger: **when the manifest can describe a plugin this build did
  not bundle** — a real feature with a pinning problem attached, and not a side effect of an
  editor.
- **Refusing a parameter that reaches an RNG nothing can seed.** This one's trigger *does* fire:
  decision 1 is the milestone that lets a user choose a patch, and choosing one is how a user
  reaches Surge XT's wall-clock RNG, Dexed's sample-and-hold LFO or sfizz's `*_random`. It fires
  and the work still cannot be done, and the honest thing is to say which of those two facts is
  the blocker. It is the second: closing it needs a **denylist of `ParamID`s per plugin in the
  build manifest**, and nothing can generate one. §8's prose names the paths in English —
  "oscillator random start phase, unison detune, the sample-and-hold LFO shape, and the effects
  that call `rand_pm1`" — and turning that into `ParamID`s means auditing Surge XT's 2855
  parameters against its source. That is a source audit, and no user interface produces it. New
  trigger: **a build manifest that can say which parameters reach an unseedable RNG**, which is
  where the work actually is. Half a denylist is worse than none, because it looks complete.

  What M2 owes in the meantime is not silence: §8's three notes stay the record, and M2 does not
  claim §11's bit-exactness for a patch a user chose by hand on those paths.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Host the plugin's **own** VST3 editor | Needs a window handle inside the engine process, an editor whose lifetime is the plugin's, and a preview process to hold it. That is a different feature with a different risk profile, and it is the thing that would finally produce an `Instrument.state`. It is not refused forever; it is refused as M2's answer to "the seventh view is unplaced". |
| Place the editors in M4 beside the code views | Leaves the three ledger rows stranded for a second milestone, and pairs the editors with the wrong thing: code views edit *source* for compilers that do not exist yet (ADR 0003 §8), while a parameter editor edits a bundled plugin that has existed since M1. |
| Leave them unplaced and let M3 or M4 discover them | The exact failure ADR 0003 was written to stop, one milestone after it was written. |
| Ship the editor showing the plugin's own units | Requires a live plugin instance to format each value; the manifest carries names, not units, because a scan is what produces it. Deferred with a trigger (decision 1). |
| Target all three of §1's platforms at M2 | Triples CI in the widest milestone, requires the `[OPEN]` minimum-OS-versions answer this ADR is not allowed to give, and postpones nothing by being deferred — the installer is M5 either way. |
| Claim macOS or Windows "should work" without a runner | The thing ADR 0009 §1 refuses in a sentence: unclaimed is not the same as contradicted, and an untested claim is worse than no claim. |

## Consequences

- ADR 0003 §5 is amended in place, dated, in the same commit as this ADR — the shape ADR 0003 §4
  already has for Airwindows.
- §9 gains the sentence that says which editor M2 builds and what it shows; §15 gains a row per
  decision. §16's M2 line is not rewritten: "mixer and history as model projections" was already
  ADR 0003's to place, and the editors are placed the same way, in the ADR rather than by
  renumbering a section a dozen files cite.
- `plan.md`'s deferred ledger keeps all three rows and changes all three triggers. None of them
  is M2's.
- M2's PR table gains no PR for the editors' schema, because they need none: `set_param` and the
  manifest already exist.
- What M2 will not claim grows by one line: not that a user-chosen patch is reproducible on the
  three paths §8 names.
