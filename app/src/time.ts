// Musical time, and the one place it becomes pixels (§4.2).
//
// The model's only musical time is integer ticks at a fixed 960 PPQ. A view's is pixels. Every
// conversion between the two lives here and there is exactly one of each, for the reason
// ADR 0007 §3 gives one layer down for tick→sample: two conversion sites grow two rounding
// rules, and the day they disagree a clip is drawn a pixel from where it is clicked.
//
// Two things a tick-linear timeline gets wrong by default, and both are in this file:
//
//   * **A bar is not a constant number of ticks.** `ticksPerBar` computed once from the first
//     `TimeSignatureEvent` is right until a second one says otherwise, and then every bar line
//     after it is wrong — silently, because the grid still looks like a grid. [`bars`] walks
//     the map instead.
//   * **A tempo change moves no pixel, and that is the point.** The x axis is ticks, so a
//     `TempoEvent` at tick 3840 leaves every clip exactly where it was: bar 3 is bar 3 at any
//     tempo, which is why a musical timeline is drawn in ticks and not in seconds. What the
//     tempo map decides is *duration*, so the one thing a read-only view can get wrong by
//     assuming a constant tempo is how long the song is — and [`seconds`] integrates the map
//     rather than sampling it once at tick 0.

import type { Song } from "@escribass/schema/song";

/** 960 ticks to the quarter note, fixed by `schema_version` 1 (§4.2). */
export const PPQ = 960;

/**
 * A bar of the grid: the number a ruler shows, where it starts, and how long it is.
 *
 * `ticks` varies with the time signature, and a signature change that lands mid-bar truncates
 * the bar in progress rather than being moved to the next bar line. Truncating is what the
 * model says happened — the event carries a tick, not a bar — and pretending otherwise would
 * put the view's bar lines somewhere the document does not.
 */
export interface Bar {
  readonly number: number;
  readonly startTick: number;
  readonly ticks: number;
}

export interface TempoMark {
  readonly tick: number;
  readonly bpm: number;
}

export interface SignatureMark {
  readonly tick: number;
  readonly numerator: number;
  readonly denominator: number;
}

// What a view draws when the map says nothing. Every project `core` creates has an event at
// tick 0 for both maps, so these are reached only by a document that lost one — and a view
// that divided by an absent tempo would render nothing at all, which reads as a broken window
// rather than as a broken document.
const OPENING_TEMPO: TempoMark = { tick: 0, bpm: 120 };
const OPENING_SIGNATURE: SignatureMark = { tick: 0, numerator: 4, denominator: 4 };

// ponytail: a ceiling on the grid, not on the song. Ticks are int32, so a clip at tick 2×10⁹
// is a legal document and would ask for 559,000 bars — a frozen window, not a slow one. At
// 4096 the ruler stops and everything past it draws without grid lines. The upgrade path is
// the viewport PR 5 needs anyway: generate the bars that are on screen, not all of them.
const MAX_BARS = 4096;

/** Ordered by tick, then by id so two events at one tick are ordered by the document and not
 *  by how the map happened to iterate (ADR 0001 §3). */
const byTick = (a: { tick: number; id: string }, b: { tick: number; id: string }) =>
  a.tick - b.tick || a.id.localeCompare(b.id);

/** The tempo map as an ordered list, with an opening event if the document has none. */
export function tempoMarks(song: Song): TempoMark[] {
  const marks = Object.values(song.tempoMap?.events ?? {})
    .sort(byTick)
    // §4.4 requires tempo > 0; a document that broke it would divide by zero here.
    .map((event) => ({ tick: event.tick, bpm: event.bpm > 0 ? event.bpm : OPENING_TEMPO.bpm }));
  return marks.length > 0 && marks[0].tick <= 0 ? marks : [OPENING_TEMPO, ...marks];
}

/** The time-signature map as an ordered list, with an opening event if the document has none. */
export function signatureMarks(song: Song): SignatureMark[] {
  const marks = Object.values(song.timeSignatureMap?.events ?? {})
    .sort(byTick)
    .map((event) => ({
      tick: event.tick,
      numerator: event.numerator,
      denominator: event.denominator,
    }));
  return marks.length > 0 && marks[0].tick <= 0 ? marks : [OPENING_SIGNATURE, ...marks];
}

function barTicks(signature: SignatureMark): number {
  const ticks = (PPQ * 4 * signature.numerator) / signature.denominator;
  // §4.4 has a rule for tempo and none for the signature, so 0/0 is a document the validator
  // accepts. A bar of zero ticks is a loop that never advances — a hung window, and the one
  // failure a read-only view must not have. Fall back to a bar rather than to nothing.
  return Number.isFinite(ticks) && ticks >= 1 ? ticks : PPQ * 4;
}

/**
 * The bar grid from tick 0 to at least `throughTick`, following the time-signature map.
 *
 * `minBars` is what keeps an empty project from drawing a ruler with nothing on it.
 */
export function bars(song: Song, throughTick: number, minBars = 1): Bar[] {
  const marks = signatureMarks(song);
  const grid: Bar[] = [];
  for (let i = 0; i < marks.length && grid.length < MAX_BARS; i += 1) {
    const perBar = barTicks(marks[i]);
    const until = i + 1 < marks.length ? marks[i + 1].tick : Infinity;
    let at = Math.max(0, marks[i].tick);
    while (
      at < until &&
      (grid.length < minBars || at < throughTick) &&
      grid.length < MAX_BARS
    ) {
      const ticks = Math.min(perBar, until - at);
      grid.push({ number: grid.length + 1, startTick: at, ticks });
      at += ticks;
    }
  }
  return grid;
}

/**
 * Where `tick` falls in seconds, integrating the tempo map segment by segment.
 *
 * The only place in a read-only view where a tempo change changes an answer. `core` derives
 * sample positions the same way for a render (ADR 0007 §3); this is not that computation and
 * makes no claim to agree with it to the sample — it is a duration a person reads.
 */
export function seconds(song: Song, tick: number): number {
  const marks = tempoMarks(song);
  let total = 0;
  for (let i = 0; i < marks.length && marks[i].tick < tick; i += 1) {
    const from = Math.max(0, marks[i].tick);
    const to = Math.min(tick, i + 1 < marks.length ? marks[i + 1].tick : tick);
    if (to > from) total += ((to - from) / PPQ) * (60 / marks[i].bpm);
  }
  return total;
}

/**
 * The tick→pixel mapping, and the only one. Linear through the origin, so the same closure
 * converts a position and a length.
 *
 * It rounds nothing: a canvas takes fractional coordinates, and the one place a half-pixel
 * matters — a hairline that would otherwise straddle two device pixels — snaps at the point of
 * drawing, where it is a drawing decision rather than a second rule about what a tick means.
 */
export function scale(width: number, ticks: number): (tick: number) => number {
  const perTick = width / Math.max(1, ticks);
  return (tick: number) => tick * perTick;
}

/**
 * The pixel→tick mapping: [`scale`]'s exact inverse, beside it for that reason.
 *
 * PR 4 left this out on purpose — there was no gesture to convert for — and named this file as
 * where it would go when it had a caller. It has one now: a drag reads a pointer position and
 * has to say which tick it is over, and a second `width / ticks` written at the pointer
 * handler is the two-rounding-rules problem this module's header describes, arriving from the
 * other direction. A note would then be dropped a pixel from where it was drawn.
 *
 * It rounds nothing either. A gesture decides how to round — a drag rounds the *delta*, so a
 * note lands where the pointer moved it rather than where the pointer happens to be.
 */
export function unscale(width: number, ticks: number): (x: number) => number {
  const perTick = width / Math.max(1, ticks);
  return (x: number) => (perTick > 0 ? x / perTick : 0);
}
