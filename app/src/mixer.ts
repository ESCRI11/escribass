// The mixer, as a pure function of the model (docs/specs.md §9, ADR 0012 §2).
//
// A strip is a `Track`'s `Mix` and the devices on it, and that is the whole projection. It
// holds no fader position, because a fader position is `Mix.gain_db` and there is only one of
// those (CLAUDE.md #1) — the control in `form.tsx` is drawn from this and from the gesture in
// flight, never from a number it keeps for itself. That is the one thing a mixer is most
// likely to get wrong here, and ADR 0012 §5's golden is what would catch it: a strip showing
// a gain nothing has is exactly the failure that golden is worded against.
//
// Order is the model's own and never storage order: tracks by `index`, effects by `index`,
// the instrument first because a chain runs from it (§4.2, ADR 0001 §3). Every comparison
// falls back to the id, so a `fx_chain` map that iterated differently answers the same.

import type { DeviceRef, Effect, Instrument, Song, Track } from "@escribass/schema/song";

/** What a device is, in a word — the label a strip shows and the editor's head repeats. */
export function deviceLabel(ref: DeviceRef | undefined): string {
  switch (ref?.kind.case) {
    case "plugin":
      return ref.kind.value.pluginId;
    case "cmajor":
    case "faust":
      return `${ref.kind.case} ${ref.kind.value.sourceHash.slice(0, 8)}`;
    case "neural":
      return `neural ${ref.kind.value.modelHash.slice(0, 8)}`;
    case "sampler":
      return `sampler ${ref.kind.value.sfzHash.slice(0, 8)}`;
    default:
      // `device_ref_unset` — legal on the wire, refused by the validator (§4.4). Named rather
      // than drawn blank, for `arrangement.ts`'s reason: a missing label reads as a missing
      // device instead of a broken one.
      return "unset";
  }
}

export interface StripDevice {
  readonly id: string;
  readonly kind: "instrument" | "effect";
  /** Chain position. Absent on the instrument, which is not in the chain. */
  readonly index?: number;
  readonly label: string;
  /** How many parameters the model overrides on this device. `Instrument.params` is sparse —
   *  a plugin has thousands and a song stores the ones that were set — so this is a count of
   *  the document's own keys and never of the plugin's (ADR 0014 §1). */
  readonly set: number;
}

export interface Strip {
  readonly id: string;
  readonly name: string;
  /** `TRACK_KIND_INSTRUMENT` with the prefix dropped, as the arrangement spells it. */
  readonly kind: string;
  readonly index: number;
  /** Decibels, unbounded above and below — the model's own units, not a plugin's normalised
   *  `0..1` and not a normalisation of them (ADR 0015 §2). */
  readonly gainDb: number;
  /** `-1.0` hard left to `1.0` hard right (`song.proto`). */
  readonly pan: number;
  readonly mute: boolean;
  readonly solo: boolean;
  readonly devices: readonly StripDevice[];
}

export interface Mixer {
  readonly strips: readonly Strip[];
}

/** `TRACK_KIND_INSTRUMENT` → `instrument`. The same four words `arrangement.ts` uses; kept
 *  beside its own view rather than shared, because sharing it would make one view's spelling
 *  depend on the other's and neither is the model's. */
function trackKind(track: Track): string {
  const named = ["unspecified", "instrument", "audio", "bus", "master"];
  return named[track.kind] ?? "unspecified";
}

function device(
  held: Instrument | Effect,
  kind: "instrument" | "effect",
  index?: number,
): StripDevice {
  return {
    id: held.id,
    kind,
    ...(index === undefined ? {} : { index }),
    label: deviceLabel(held.ref),
    set: Object.keys(held.params).length,
  };
}

/**
 * The devices on one track: its instrument, then its effect chain in `index` order.
 *
 * Exported because the editor opens on one of these and needs the same list in the same order
 * — a `<select>` whose order disagreed with the strip beside it would be two answers to one
 * question (`params.ts`).
 */
export function devicesOf(track: Track): readonly StripDevice[] {
  const chain = Object.values(track.fxChain)
    .sort((a, b) => a.index - b.index || a.id.localeCompare(b.id))
    .map((effect) => device(effect, "effect", effect.index));
  return track.instrument ? [device(track.instrument, "instrument"), ...chain] : chain;
}

/**
 * The mixer the strips are drawn from.
 *
 * `mute` and `solo` are carried as the booleans they are and nothing is derived from them.
 * Whether a soloed track silences its neighbours is a rule, and a rule invented in a view is
 * one the renderer does not share — the same reason the roll draws no rule of its own
 * (ADR 0017 §3).
 */
export function mixer(song: Song): Mixer {
  const strips = Object.values(song.tracks)
    .sort((a, b) => a.index - b.index || a.id.localeCompare(b.id))
    .map((track) => ({
      id: track.id,
      name: track.name,
      kind: trackKind(track),
      index: track.index,
      // `Mix` is a message and is absent on the wire when every field is its default; the
      // document always has one, because canonical JSON emits defaults (ADR 0002 §4). Read
      // through `??` rather than asserted, so a hand-written fixture without one draws a
      // strip at unity instead of throwing in a projection.
      gainDb: track.mix?.gainDb ?? 0,
      pan: track.mix?.pan ?? 0,
      mute: track.mix?.mute ?? false,
      solo: track.mix?.solo ?? false,
      devices: devicesOf(track),
    }));
  return { strips };
}
