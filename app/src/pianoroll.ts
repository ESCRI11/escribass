// The piano roll, as a pure function of the model (docs/specs.md §9, ADR 0012 §2).
//
// This is the view where a user interface usually grows its second model. `NoteClip.notes` is
// a map keyed by id (ADR 0001 §3) and a roll wants notes ordered, positioned and bounded by a
// pitch range; the tempting shape is a note list built once, kept beside the song and nudged
// as the user edits. That is the normalised store §14.2 forbids, arriving as a convenience.
//
// So the derivation happens here, per render, from the one decoded `Song` and a clip id — and
// ADR 0012 §5's golden is what proves it rather than this comment. Nothing below is retained
// between calls.
//
// Two things it deliberately does not do, so that PR 5 and ADR 0015 stay open:
//
//   * No edit path, not even a disabled one. There is no selection model, no hover state and
//     no note geometry in reverse (pixel → tick): PR 4 is read-only so that ADR 0012 §2's
//     answer is reviewed on its own.
//   * No per-note parameter lane. ADR 0015 gives automation a `ParamRef` and M2 PR 6 is where
//     a lane gets drawn; `Note.expression` is a map the roll shows nothing of yet, and adding
//     a lane shaped for one note's expression would foreclose the shape PR 6 needs.

import type { Song } from "@escribass/schema/song";
import { bars } from "./time.js";
import type { Bar } from "./time.js";

export interface RollNote {
  readonly id: string;
  readonly pitch: number;
  /** Scientific pitch notation, in which MIDI 60 is C4. */
  readonly name: string;
  readonly microtonalCents: number;
  /** Clip-relative, as `Note.start_tick` is (§4.2). */
  readonly startTick: number;
  readonly lengthTicks: number;
  readonly velocity: number;
}

export interface PianoRoll {
  readonly clipId: string;
  readonly trackId: string;
  readonly trackName: string;
  /** Where the clip sits in the song, so the roll can say which bar it is looking at. */
  readonly startTick: number;
  readonly lengthTicks: number;
  readonly loopLengthTicks?: number;
  /** The lowest and highest rows drawn, inclusive. */
  readonly lowPitch: number;
  readonly highPitch: number;
  /** The song's bar grid, clipped to this clip and re-expressed clip-relative. The numbers
   *  stay the song's, so bar 5 in the roll is bar 5 in the arrangement. */
  readonly bars: readonly Bar[];
  readonly notes: readonly RollNote[];
}

const NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/** MIDI 60 → `C4`. Scientific pitch notation, which is what §4.2's "MIDI 0–127" implies and
 *  what a note name means outside a Yamaha manual. */
export function pitchName(pitch: number): string {
  return `${NAMES[((pitch % 12) + 12) % 12]}${Math.floor(pitch / 12) - 1}`;
}

/** Whether this pitch class is a black key. Drawn, not stored: it is a property of 12-tone
 *  equal temperament, not of the document. */
export function isBlackKey(pitch: number): boolean {
  return NAMES[((pitch % 12) + 12) % 12].length === 2;
}

// A roll with no notes still needs rows. C3 to C5 is the octave either side of middle C, which
// is where a first note usually lands.
const EMPTY_LOW = 48;
const EMPTY_HIGH = 72;
// Rows above and below the outermost note, so a note is not drawn against the edge.
const PADDING = 2;
// A roll shorter than an octave is a row of stripes rather than a keyboard.
const MINIMUM_ROWS = 12;

/**
 * The roll for one clip, or `null` if the id names nothing or names a clip whose content is
 * not notes — an audio clip has no roll, and saying so with `null` is what lets the window
 * draw the arrangement alone rather than an empty grid that looks broken. A note clip with no
 * notes in it *does* get a roll, because an empty bar of a real clip is something to look at.
 *
 * Note order is the document's: start tick, then pitch, then id. A map has no order, so one
 * has to be chosen, and choosing it here — rather than letting `Object.values` decide — is the
 * difference between a view that is a function of the song and one that is a function of how
 * the song was decoded.
 */
export function pianoRoll(song: Song, clipId: string): PianoRoll | null {
  const clip = song.clips[clipId];
  if (clip?.content.case !== "noteClip") return null;

  const notes = Object.values(clip.content.value.notes)
    .sort((a, b) => a.startTick - b.startTick || a.pitch - b.pitch || a.id.localeCompare(b.id))
    .map((note) => ({
      id: note.id,
      pitch: note.pitch,
      name: pitchName(note.pitch),
      microtonalCents: note.microtonalCents,
      startTick: note.startTick,
      lengthTicks: note.lengthTicks,
      velocity: note.velocity,
    }));

  const pitches = notes.map((note) => note.pitch);
  let lowPitch = pitches.length === 0 ? EMPTY_LOW : Math.min(...pitches) - PADDING;
  let highPitch = pitches.length === 0 ? EMPTY_HIGH : Math.max(...pitches) + PADDING;
  // Grow upwards first, then downwards, and clamp to MIDI's range last, so a clip of notes at
  // pitch 0 still gets a full keyboard instead of half of one.
  highPitch = Math.max(highPitch, lowPitch + MINIMUM_ROWS - 1);
  lowPitch = Math.max(0, Math.min(lowPitch, highPitch - MINIMUM_ROWS + 1));
  highPitch = Math.min(127, Math.max(highPitch, lowPitch + MINIMUM_ROWS - 1));

  // The song's grid, not a grid of this clip's own: a clip starting mid-bar is drawn starting
  // mid-bar, and a time-signature change inside it changes the bar lines inside it. The first
  // kept bar may begin before the clip, and its negative start is what says so.
  const grid = bars(song, clip.startTick + clip.lengthTicks)
    .filter((bar) => bar.startTick + bar.ticks > clip.startTick)
    .map((bar) => ({ ...bar, startTick: bar.startTick - clip.startTick }));

  return {
    clipId: clip.id,
    trackId: clip.trackId,
    trackName: song.tracks[clip.trackId]?.name ?? "",
    startTick: clip.startTick,
    lengthTicks: clip.lengthTicks,
    ...(clip.loopLengthTicks === undefined ? {} : { loopLengthTicks: clip.loopLengthTicks }),
    lowPitch,
    highPitch,
    bars: grid,
    notes,
  };
}
