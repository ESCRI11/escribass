// §9's seventh view — the generic instrument and effect editor — as a pure function of the
// model *and* of the build manifest (ADR 0014 §1, ADR 0012 §2).
//
// It is the only projection here that takes a second argument, and the reason is the whole
// shape of the view: a plugin's parameters are not in the song. The manifest maps each
// plugin's numeric `ParamID` to the display name the plugin reports (ADR 0010 §4), and the
// song holds a **sparse** map of the ones somebody set. So the rows come from the manifest and
// the values come from the model, and opening an editor writes nothing — a form that filled
// `Instrument.params` with 2855 defaults on open would turn looking at a plugin into a commit,
// and the manifest carries no defaults to fill it with anyway.
//
// **The key is the `ParamID` and the name is a label.** VST3 names are not unique: Surge XT's
// 2855 parameters carry 2679 distinct names, 176 fewer than there are parameters, in 16 groups
// of twelve — one group per unassigned effect slot, every one of them called `FX A1 -` and its
// neighbours. A form keyed by display name would silently collapse a dozen controls onto one.
// That is the same fact that keys the manifest by id and that `param_unknown` resolves against
// (`core/src/validate.rs`, `core/src/manifest.rs`).
//
// **What it cannot show.** A number between 0 and 1 beside a name, and no unit: VST3 exposes
// exactly one numeric domain to a host and it is normalised, so a user cannot read "−6 dB" off
// this. That limit is ADR 0014 §1's, written down in advance rather than discovered, and it is
// honoured here rather than worked around — a plugin's own display string exists only inside a
// live instance, which is the engine, and asking it is a round trip per pixel. Deferred with a
// trigger: when a preview session already holds an engine process (ADR 0013 §3).

import type { Song } from "@escribass/schema/song";
import { deviceLabel, devicesOf } from "./mixer.js";
import type { StripDevice } from "./mixer.js";

/**
 * What one engine build can host (`core/src/manifest.rs`, ADR 0010 §4).
 *
 * Hand-written, and allowed to be, for `App.tsx`'s `ToolAnswer` reason: §4.1 forbids a
 * hand-written type for anything the **model** names, and this is not in `song.proto` at all.
 * It is what the running build says about itself, it has no protobuf, and the host serialises
 * `core`'s own parsed struct — so this interface is that struct's shape and nothing else's.
 */
export interface ManifestPlugin {
  readonly commit: string;
  readonly version: string;
  /** `ParamID` → display name. Neither side is unique in the way you would hope: the ids are
   *  unique and opaque, the names are readable and repeat. */
  readonly params: Readonly<Record<string, string>>;
}

export interface Manifest {
  readonly engine: Readonly<Record<string, string>>;
  readonly plugins: Readonly<Record<string, ManifestPlugin>>;
  /** Which plugin class plays a `SamplerRef`. Not used to resolve parameters; see `editor`. */
  readonly sampler: string;
}

export interface ParamRow {
  /** The plugin's own `ParamID` — the key of `Instrument.params` and what a `set_param` names
   *  (ADR 0010 §4). Never the display name. */
  readonly id: string;
  /** What the plugin calls it, or `""` for a value this build cannot name. */
  readonly name: string;
  /** The override the model holds, or `null` where it holds none. `null` is not zero: an
   *  unset parameter is at whatever the plugin's own default is, which the manifest does not
   *  carry and this view therefore does not claim to know. */
  readonly value: number | null;
  /** Whether this build's manifest declares it. False means the document holds a value under
   *  an id this build cannot resolve — which the validator refuses (`param_unknown`) but
   *  `Project::open` does not run, so it reaches a window whenever a project written against
   *  a fuller manifest is opened against a smaller one. Shown rather than dropped, because a
   *  value in the document that no view can see is the silent half of that mismatch. */
  readonly declared: boolean;
}

export interface DeviceEditor {
  readonly deviceId: string;
  readonly trackId: string;
  readonly trackName: string;
  readonly kind: "instrument" | "effect";
  /** The plugin id, or what the device names instead (`mixer.ts`). */
  readonly device: string;
  /** How many rows the plugin declares, and how many the model overrides. Two numbers because
   *  they are two maps, and the second is a subset of the first in any valid document. */
  readonly declares: number;
  readonly set: number;
  readonly rows: readonly ParamRow[];
}

/**
 * A `ParamID` order that is a function of the ids and of nothing else.
 *
 * Numeric, because a VST3 `ParamID` is a `uint32` written out as a string, with a string
 * comparison behind it so a device whose parameters are named rather than numbered still gets
 * a total order. It is deliberately **not** the manifest's own key order, which is the order
 * the plugin declared its parameters in and is the order a plugin's own window shows: that
 * order is a JSON object's insertion order, and JavaScript does not keep it — an integer-like
 * key is re-sorted numerically ascending by the language itself, before any code here runs.
 * So declaration order is not available to this view at all, and sorting explicitly is what
 * says so rather than accidentally agreeing with it.
 */
function byId(a: string, b: string): number {
  const left = Number(a);
  const right = Number(b);
  return Number.isFinite(left) && Number.isFinite(right)
    ? left - right || a.localeCompare(b)
    : a.localeCompare(b);
}

/** One device, found by the id that names it and nothing else. */
interface Found {
  readonly track: string;
  readonly trackName: string;
  readonly kind: "instrument" | "effect";
  readonly label: string;
  /** The plugin whose parameters this device's are, or `null` when it has none this build can
   *  resolve. */
  readonly pluginId: string | null;
  /** The overrides the document holds: `ParamID` → normalised value, sparse. */
  readonly params: Readonly<Record<string, number>>;
}

/**
 * The device an id names, searched exactly as `core/src/tools.rs`'s `device_path` searches —
 * over every track, because ids are globally unique across collections (§4.3, ADR 0001 §3) so
 * at most one track can answer and the order tracks are walked in cannot change the answer.
 *
 * Which plugin it *is* follows `core/src/validate.rs`'s `plugin_of`, exactly: only a
 * `PluginRef` resolves. A `SamplerRef` does **not**, even though `Manifest.sampler` names the
 * plugin that plays one — because the validator does not resolve it either, so a sampler's
 * `params` go unchecked, and a form that offered sfizz's 543 controls there would be offering
 * writes nothing refuses and the renderer ignores. Cmajor, Faust and neural devices declare
 * their parameters in a source M4 compiles, which is the `ponytail:` note the validator
 * already carries beside the same line.
 */
function deviceIn(song: Song, deviceId: string): Found | null {
  for (const track of Object.values(song.tracks)) {
    const instrument = track.instrument?.id === deviceId;
    const held = instrument ? track.instrument : track.fxChain[deviceId];
    if (!held) continue;
    return {
      track: track.id,
      trackName: track.name,
      kind: instrument ? "instrument" : "effect",
      label: deviceLabel(held.ref),
      pluginId: held.ref?.kind.case === "plugin" ? held.ref.kind.value.pluginId : null,
      params: held.params,
    };
  }
  return null;
}

/**
 * The editor for one device, or `null` when the id names no device in this song.
 *
 * The rows are the plugin's declared parameters, joined with whatever the document holds under
 * an id the plugin does not declare — a union rather than one map or the other, because
 * dropping either half loses something a person needs to see: the plugin's parameters are what
 * can be set, and the document's keys are what *is* set.
 */
export function editor(song: Song, manifest: Manifest, deviceId: string): DeviceEditor | null {
  const found = deviceIn(song, deviceId);
  if (!found) return null;

  const declared = (found.pluginId === null ? undefined : manifest.plugins[found.pluginId])
    ?.params;
  const ids = [...new Set([...Object.keys(declared ?? {}), ...Object.keys(found.params)])].sort(
    byId,
  );

  return {
    deviceId,
    trackId: found.track,
    trackName: found.trackName,
    kind: found.kind,
    device: found.label,
    declares: Object.keys(declared ?? {}).length,
    set: Object.keys(found.params).length,
    rows: ids.map((id) => ({
      id,
      name: declared?.[id] ?? "",
      value: found.params[id] ?? null,
      declared: declared?.[id] !== undefined,
    })),
  };
}

/**
 * Every device in the song, in the order the mixer draws them — so the editor's `<select>` and
 * the strips agree about what a device is called and where it sits.
 */
export function devices(song: Song): readonly (StripDevice & { readonly track: string })[] {
  return Object.values(song.tracks)
    .sort((a, b) => a.index - b.index || a.id.localeCompare(b.id))
    .flatMap((track) => devicesOf(track).map((held) => ({ ...held, track: track.name })));
}
