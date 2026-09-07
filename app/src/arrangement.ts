// The arrangement view, as a pure function of the model (docs/specs.md §9, ADR 0012 §2).
//
// A projection and nothing else: it takes the one decoded `Song` and returns a plain,
// serialisable description of what the timeline would draw. It holds no state, reads no
// clock, and has no way to reach the host. That shape is not decoration — it is what ADR
// 0012 §5's projection golden compares against committed bytes, and the golden lands in
// PR 4 with the views it can cover. Keeping the shape now costs nothing and means the golden
// is written against a projection rather than a projection being retrofitted to a golden.
//
// Model types come from `schema/`'s generated TypeScript. There is no hand-written type for
// anything the model already names (§4.1, CLAUDE.md, Toolchain).

import type { Clip, Song, Track } from "@escribass/schema/song";

/** 960 ticks to the quarter note, fixed (§4.2). */
export const PPQ = 960;

export interface ClipBlock {
  readonly id: string;
  readonly startTick: number;
  readonly lengthTicks: number;
  /** What the block says on it: the clip's content, in a word. */
  readonly label: string;
}

export interface TrackRow {
  readonly id: string;
  readonly name: string;
  /** `TRACK_KIND_INSTRUMENT` with the prefix dropped: `instrument`, `audio`, `bus`, `master`. */
  readonly kind: string;
  readonly clips: readonly ClipBlock[];
}

export interface Arrangement {
  readonly bpm: number;
  readonly numerator: number;
  readonly denominator: number;
  /** How far the drawing extends, in ticks: the song, rounded up to a whole bar. */
  readonly lengthTicks: number;
  readonly ticksPerBar: number;
  readonly tracks: readonly TrackRow[];
}

/** The tempo in force at tick 0, or 120 if the map is empty. */
function openingTempo(song: Song): number {
  const events = Object.values(song.tempoMap?.events ?? {});
  if (events.length === 0) return 120;
  return events.reduce((a, b) => (a.tick <= b.tick ? a : b)).bpm;
}

/** The time signature in force at tick 0, or 4/4. */
function openingSignature(song: Song): { numerator: number; denominator: number } {
  const events = Object.values(song.timeSignatureMap?.events ?? {});
  if (events.length === 0) return { numerator: 4, denominator: 4 };
  const first = events.reduce((a, b) => (a.tick <= b.tick ? a : b));
  return { numerator: first.numerator, denominator: first.denominator };
}

function label(clip: Clip): string {
  switch (clip.content.case) {
    case "noteClip": {
      const notes = Object.keys(clip.content.value.notes).length;
      return `${notes} note${notes === 1 ? "" : "s"}`;
    }
    case "audioClip":
      return clip.content.value.assetHash.slice(0, 8);
    default:
      // A clip whose content oneof is unset is legal on the wire and refused by the
      // validator (§4.4). Drawing it as empty beats drawing nothing at all, which would
      // look like a missing clip rather than a broken one.
      return "empty";
  }
}

/** `TRACK_KIND_INSTRUMENT` → `instrument`. */
function kindOf(track: Track): string {
  const named = ["unspecified", "instrument", "audio", "bus", "master"];
  return named[track.kind] ?? "unspecified";
}

/**
 * The arrangement the timeline draws.
 *
 * Order is the model's own and never storage order: tracks by `index`, clips by `startTick`
 * (§4.2, ADR 0001 §3). Two tracks with the same index fall back to their ids so the result is
 * a function of the document and not of how a map happened to iterate.
 */
export function arrangement(song: Song): Arrangement {
  const { numerator, denominator } = openingSignature(song);
  const ticksPerBar = (PPQ * 4 * numerator) / denominator;

  const byTrack = new Map<string, ClipBlock[]>();
  for (const clip of Object.values(song.clips)) {
    const blocks = byTrack.get(clip.trackId) ?? [];
    blocks.push({
      id: clip.id,
      startTick: clip.startTick,
      lengthTicks: clip.lengthTicks,
      label: label(clip),
    });
    byTrack.set(clip.trackId, blocks);
  }
  for (const blocks of byTrack.values()) {
    blocks.sort((a, b) => a.startTick - b.startTick || a.id.localeCompare(b.id));
  }

  const tracks = Object.values(song.tracks)
    .sort((a, b) => a.index - b.index || a.id.localeCompare(b.id))
    .map((track) => ({
      id: track.id,
      name: track.name,
      kind: kindOf(track),
      clips: byTrack.get(track.id) ?? [],
    }));

  const end = Object.values(song.clips).reduce(
    (far, clip) => Math.max(far, clip.startTick + clip.lengthTicks),
    0,
  );
  // At least eight bars, so an empty project still has a ruler to look at.
  const bars = Math.max(8, Math.ceil(end / ticksPerBar));

  return {
    bpm: openingTempo(song),
    numerator,
    denominator,
    lengthTicks: bars * ticksPerBar,
    ticksPerBar,
    tracks,
  };
}
