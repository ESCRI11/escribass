// The projection golden (ADR 0012 §5; docs/specs.md §11, fifth bullet).
//
// What §11 means for a user interface, and the whole of what it means: **every view is a pure
// function of the model.** From a fixed `Song`, each view's projection returns a serialisable
// description of what it would draw, and that description is compared against committed bytes.
// It runs under `node:test` with `tsx`, both already pinned (ADR 0016 §2); it needs no browser,
// no display and no driver, and it compares no pixel.
//
// It claims nothing about wiring, timing or how anything looks. A driven session with a UI
// automation driver would claim more and would buy a browser driver, a display in CI and a
// flake class this repository has never had — ADR 0012 §5 weighed that and chose this.
//
// What it does catch is the failure ADR 0012 §2 is arranged against: a store that has drifted
// from the document, or an order taken from how a map happened to iterate rather than from the
// document, shows up here as a diff instead of as a mixer showing a gain nothing has.
//
// Bless a deliberate change with:
//
//     UPDATE_FIXTURES=1 npm --prefix app test
//
// and review the diff, exactly as `tests/AGENTS.md` requires of every other golden here: the
// variable blesses whatever ran, including a deterministically wrong projection.

import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { test } from "node:test";
import { create, fromJson, toJson } from "@bufbuild/protobuf";
import { SongSchema, type Song } from "@escribass/schema/song";
import { arrangement } from "../src/arrangement.js";
import { pianoRoll } from "../src/pianoroll.js";
import { bars, seconds } from "../src/time.js";

// The fixture is a determinism golden, which means it was built **through the tool API** and
// is blessed by `UPDATE_FIXTURES=1 cargo test` — CLAUDE.md #2, with no second way to write a
// song in the repository and none introduced here.
//
// `render`'s and not `every_tool`'s, which is what ADR 0012 §5 named when it had no view in
// front of it and is amended in place to say so. `every_tool` has one clip, two notes of equal
// length, and its tempo change at tick 3840 is the last tick of the song — so a projection
// that ignored the tempo map, the loop field, the audio content case and every ordering
// question would golden identically to one that did not. `render` has five tracks, four clips
// across three of them, one of them audio and two of them looping, six notes at four pitches
// and four lengths, a section that runs past the last clip, and a tempo change at tick 3840
// with three and a half thousand ticks of music after it.
const FIXTURE = new URL("../../tests/determinism/render/expected/song.json", import.meta.url);
const GOLDEN = new URL("./projection.golden.json", import.meta.url);

const read = (at: URL) => readFileSync(at, "utf8");
const document: unknown = JSON.parse(read(FIXTURE));
const song: Song = fromJson(SongSchema, document as never);

/**
 * The same document with every object's keys in the opposite order.
 *
 * A protobuf map decodes to a JavaScript object, and `Object.values` on one returns *insertion*
 * order — which for a song read from disk is the file's key order, lexical by ULID (ADR 0002
 * §4). A ULID sorts by the time it was minted, so in any fixture where entities were created
 * in the order they occur, "sorted by the model" and "whatever the map iterated" are the same
 * list, and a projection that dropped its `sort` would golden identically. Measured, not
 * assumed: deleting the roll's `sort` left this suite green until this existed.
 *
 * Reversing the keys separates the two. Nothing about the document changes — a map has no
 * order and proto3 JSON does not give it one — so every projection must answer the same way.
 */
function reversed(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(reversed);
  if (value === null || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>)
      .reverse()
      .map(([key, held]) => [key, reversed(held)]),
  );
}

/** Every view, from the one song, in the order the window builds them. */
function project(from: Song) {
  const view = arrangement(from);
  return {
    arrangement: view,
    // One roll per note clip, keyed by clip id, in the arrangement's own order — which is how
    // the window fills its `<select>`. A clip the arrangement does not place is a clip the
    // roll cannot be opened on, and that is a claim worth goldening rather than asserting.
    rolls: view.tracks
      .flatMap((track) => track.clips)
      .filter((clip) => clip.kind === "note")
      .map((clip) => pianoRoll(from, clip.id)),
  };
}

test("every view is a pure function of the model", () => {
  const before = toJson(SongSchema, song);
  const actual = `${JSON.stringify(project(song), null, 2)}\n`;

  if (process.env.UPDATE_FIXTURES === "1") {
    writeFileSync(GOLDEN, actual);
    return;
  }

  const expected = read(GOLDEN);
  // Deep-equal first, because its failure names the path that moved; the text comparison after
  // it is what notices a projection that reordered its own fields, which parsing would hide.
  assert.deepStrictEqual(JSON.parse(actual), JSON.parse(expected));
  assert.equal(actual, expected, "the projection's field order moved");

  // Pure means pure: projecting must not touch the document it read. The window freezes the
  // decoded song in development builds for the same reason (`App.tsx`), and this is the half
  // of that guard which runs in CI.
  assert.deepStrictEqual(toJson(SongSchema, song), before, "a projection wrote to the model");
  assert.deepStrictEqual(project(song), project(song), "two projections of one song differ");
});

test("a view is a function of the document, not of how its maps iterated", () => {
  const backwards = fromJson(SongSchema, reversed(document) as never);
  assert.deepStrictEqual(
    project(backwards),
    project(song),
    "an order came from the map's iteration rather than from the model (ADR 0001 §3)",
  );
});

// The two conversions in `time.ts`, checked directly rather than only through a golden. Both
// have a case no committed fixture contains, and a golden cannot fail for an input it has
// never been given.

test("a bar is as long as the time signature in force says it is", () => {
  // No `.escri` in the repository has a second time-signature event, and no tool mints one:
  // `set_tempo` writes the tempo map and there is no `set_time_signature` (core/src/call.rs,
  // IMPLEMENTED). So this is a constructed value exercising a pure function — the distinction
  // `tests/AGENTS.md` already draws for the schema fixture — and not a document under test.
  const changes = create(SongSchema, {
    timeSignatureMap: {
      events: {
        a: { id: "a", tick: 0, numerator: 4, denominator: 4 },
        b: { id: "b", tick: 7680, numerator: 3, denominator: 4 },
      },
    },
  });
  assert.deepStrictEqual(
    bars(changes, 13440).map((bar) => [bar.number, bar.startTick, bar.ticks]),
    [
      [1, 0, 3840],
      [2, 3840, 3840],
      [3, 7680, 2880],
      [4, 10560, 2880],
    ],
    "a constant ticks-per-bar would put every bar after the change in the wrong place",
  );

  // A signature event carries a tick, not a bar, so one landing mid-bar truncates the bar in
  // progress. Moving it to the next bar line would draw a grid the document does not have.
  const midBar = create(SongSchema, {
    timeSignatureMap: {
      events: {
        a: { id: "a", tick: 0, numerator: 4, denominator: 4 },
        b: { id: "b", tick: 5760, numerator: 3, denominator: 4 },
      },
    },
  });
  assert.deepStrictEqual(
    bars(midBar, 8640).map((bar) => [bar.number, bar.startTick, bar.ticks]),
    [
      [1, 0, 3840],
      [2, 3840, 1920],
      [3, 5760, 2880],
    ],
  );

  // An empty map is 4/4, and `minBars` is what stops an empty project drawing a bare ruler.
  assert.equal(bars(create(SongSchema, {}), 0, 8).length, 8);
});

test("a duration integrates the tempo map rather than sampling it", () => {
  // The fixture is 120 BPM to tick 3840 and 90 BPM after it, and ends at tick 9600 — the end
  // of its `Outro` section, which runs past the last clip.
  assert.equal(seconds(song, 0), 0);
  assert.equal(seconds(song, 3840), 2, "four beats at 120 BPM");
  // 3840 ticks at 120, then 5760 at 90: 2 s + 6 beats × ⅔ s. A view that read the tempo once
  // at tick 0 would say 5, a whole second short of a six-second song.
  assert.equal(seconds(song, 9600), 6);
  assert.equal(arrangement(song).seconds, 6);
});
