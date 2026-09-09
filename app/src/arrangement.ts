// The arrangement view, as a pure function of the model (docs/specs.md §9, ADR 0012 §2).
//
// A projection and nothing else: it takes the one decoded `Song` and returns a plain,
// serialisable description of what the timeline would draw. It holds no state, reads no clock,
// and has no way to reach the host. That shape is what ADR 0012 §5's projection golden
// compares against committed bytes — `app/tests/projection.test.ts` — so a store that had
// drifted from the document, or an order taken from a map's iteration rather than from the
// document, shows up as a golden diff.
//
// Model types come from `schema/`'s generated TypeScript. There is no hand-written type for
// anything the model already names (§4.1, CLAUDE.md, Toolchain).

import type { Clip, Song, Track } from "@escribass/schema/song";
import { bars, seconds, signatureMarks, tempoMarks } from "./time.js";
import type { Bar, SignatureMark, TempoMark } from "./time.js";

export type ClipKind = "note" | "audio" | "empty";

export interface ClipBlock {
  readonly id: string;
  readonly kind: ClipKind;
  readonly startTick: number;
  readonly lengthTicks: number;
  /** Present means looping: the first `loopLengthTicks` of content repeat to fill the clip. */
  readonly loopLengthTicks?: number;
  /** What the block says on it: the clip's content, in a word. */
  readonly label: string;
}

export interface TrackRow {
  readonly id: string;
  readonly name: string;
  /** `TRACK_KIND_INSTRUMENT` with the prefix dropped: `instrument`, `audio`, `bus`, `master`. */
  readonly kind: string;
  readonly index: number;
  readonly clips: readonly ClipBlock[];
}

export interface SectionSpan {
  readonly id: string;
  readonly name: string;
  readonly startTick: number;
  readonly endTick: number;
}

export interface Arrangement {
  /** Where the document ends: the furthest tick anything in it reaches. */
  readonly endTick: number;
  /** How far the drawing extends, in ticks: the last bar line at or past `endTick`. */
  readonly lengthTicks: number;
  /** `endTick` in seconds, integrated over the tempo map (see `time.ts`). */
  readonly seconds: number;
  readonly bars: readonly Bar[];
  readonly tempo: readonly TempoMark[];
  readonly signatures: readonly SignatureMark[];
  readonly sections: readonly SectionSpan[];
  readonly tracks: readonly TrackRow[];
}

function kindOf(clip: Clip): ClipKind {
  switch (clip.content.case) {
    case "noteClip":
      return "note";
    case "audioClip":
      return "audio";
    default:
      // A clip whose content oneof is unset is legal on the wire and refused by the validator
      // (§4.4). Drawing it as empty beats drawing nothing, which would look like a missing
      // clip rather than a broken one.
      return "empty";
  }
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
      return "empty";
  }
}

/** `TRACK_KIND_INSTRUMENT` → `instrument`. */
function trackKind(track: Track): string {
  const named = ["unspecified", "instrument", "audio", "bus", "master"];
  return named[track.kind] ?? "unspecified";
}

/**
 * The arrangement the timeline draws.
 *
 * Order is the model's own and never storage order: tracks by `index`, clips and sections by
 * `startTick`, time-base events by `tick` (§4.2, ADR 0001 §3). Every comparison falls back to
 * the id, so the result is a function of the document rather than of how a map iterated —
 * which is a claim the golden checks and a comment could only assert.
 */
export function arrangement(song: Song): Arrangement {
  const byTrack = new Map<string, ClipBlock[]>();
  for (const clip of Object.values(song.clips)) {
    const blocks = byTrack.get(clip.trackId) ?? [];
    blocks.push({
      id: clip.id,
      kind: kindOf(clip),
      startTick: clip.startTick,
      lengthTicks: clip.lengthTicks,
      ...(clip.loopLengthTicks === undefined ? {} : { loopLengthTicks: clip.loopLengthTicks }),
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
      kind: trackKind(track),
      index: track.index,
      clips: byTrack.get(track.id) ?? [],
    }));

  const sections = Object.values(song.sections)
    .sort((a, b) => a.startTick - b.startTick || a.id.localeCompare(b.id))
    .map((section) => ({
      id: section.id,
      name: section.name,
      startTick: section.startTick,
      endTick: section.endTick,
    }));

  const tempo = tempoMarks(song);
  const signatures = signatureMarks(song);

  // Everything the document places on the timeline, not only the clips: a section that runs
  // past the last clip, or a tempo change after it, is part of the song and would otherwise be
  // drawn off the end of the ruler. The `render` fixture has both.
  const end = [
    ...Object.values(song.clips).map((clip) => clip.startTick + clip.lengthTicks),
    ...sections.map((section) => section.endTick),
    ...tempo.map((mark) => mark.tick),
    ...signatures.map((mark) => mark.tick),
  ].reduce((far, tick) => Math.max(far, tick), 0);

  // At least eight bars, so an empty project still has a ruler to look at.
  const grid = bars(song, end, 8);
  const last = grid[grid.length - 1];
  const lengthTicks = last.startTick + last.ticks;

  return {
    endTick: end,
    lengthTicks,
    seconds: seconds(song, end),
    bars: grid,
    tempo,
    signatures,
    sections,
    tracks,
  };
}
