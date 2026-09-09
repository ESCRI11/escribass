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
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { test } from "node:test";
import { create, fromJson, toJson } from "@bufbuild/protobuf";
import { SongSchema, type Song } from "@escribass/schema/song";
import { arrangement } from "../src/arrangement.js";
import { pianoRoll } from "../src/pianoroll.js";
import { mixer } from "../src/mixer.js";
import { devices, editor } from "../src/params.js";
import type { Manifest } from "../src/params.js";
import { patchLog } from "../src/history.js";
import type { HistoryAnswer, LogEntry } from "../src/history.js";
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

// The seventh view is a form over the **build manifest** as well as over the song (ADR 0014
// §1), so the suite needs one — and it is the committed subset every other suite validates
// against (`tests/AGENTS.md`), never a manifest written here. Its three Surge XT parameter ids
// came out of a real `--scan`, which is what makes a golden row of this editor a row a real
// plugin would produce; a hand-typed id would golden just as deterministically and mean
// nothing. It declares three of Surge XT's 2855, which is the subset the fixtures name.
const MANIFEST = new URL("../../tests/fixtures/manifest.json", import.meta.url);

// The history view needs a **log**, and the log is not in `song.json` — so it comes from a
// second determinism golden, `branches`, which is the one script that produces a log worth
// projecting: two branches, a merge with two parents, a conflict resolved per path, and a
// branch deleted whose entries stay behind. Everything below is exactly what `get_history`
// answers with (`core/src/call.rs`, `history_json`), because `entry_to_json` writes the wire
// shape and the file shape from one function — the files *are* the answer.
const LOG = new URL("../../tests/determinism/branches/expected/", import.meta.url);

const read = (at: URL) => readFileSync(at, "utf8");
const document: unknown = JSON.parse(read(FIXTURE));
const song: Song = fromJson(SongSchema, document as never);
const declared: unknown = JSON.parse(read(MANIFEST));
const build = declared as Manifest;

/** `patches/*.json` and `refs.json`, in the shape `get_history` returns them. */
function readLog(): HistoryAnswer {
  const at = new URL("patches/", LOG);
  const entries = Object.fromEntries(
    readdirSync(at)
      .filter((name) => name.endsWith(".json"))
      .sort()
      .map((name) => {
        const entry = JSON.parse(read(new URL(name, at))) as LogEntry;
        return [entry.id, entry];
      }),
  );
  const refs = JSON.parse(read(new URL("refs.json", LOG))) as {
    head: string;
    refs: Record<string, string>;
  };
  return { entries, head: refs.head, refs: refs.refs };
}

const logged: unknown = readLog();

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

/** Every view, from the one song, the one manifest and the one log, in the order the window
 *  builds them. */
function project(from: Song, hosts: Manifest, history: HistoryAnswer) {
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
    mixer: mixer(from),
    // One editor per device, in the mixer's own order, for the same reason the rolls follow
    // the arrangement's: the window's `<select>` is filled from that list, so a device the
    // mixer does not place is a device no editor can be opened on.
    //
    // The fixture covers what this view has to get right: two Surge XT instruments with no
    // overrides at all, two Surge XT effects one of which holds exactly one, and a sampler
    // instrument whose `ref` this build resolves to no plugin — so `rows` is empty there, and
    // that emptiness is the projection agreeing with the validator rather than a gap.
    editors: devices(from).map((held) => editor(from, hosts, held.id)),
    // The one view whose input is not the song. It is here rather than in a golden of its own
    // because what §11's fifth bullet claims is about *every* view, and a second file would
    // be a second place to forget one (ADR 0012 §5).
    log: patchLog(history),
  };
}

test("every view is a pure function of the model", () => {
  const before = toJson(SongSchema, song);
  const manifestBefore = JSON.stringify(declared);
  const logBefore = JSON.stringify(logged);
  const actual = `${JSON.stringify(project(song, build, logged as HistoryAnswer), null, 2)}\n`;

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
  assert.equal(JSON.stringify(declared), manifestBefore, "a projection wrote to the manifest");
  assert.equal(JSON.stringify(logged), logBefore, "a projection wrote to the log");
  assert.deepStrictEqual(
    project(song, build, logged as HistoryAnswer),
    project(song, build, logged as HistoryAnswer),
    "two projections of one song differ",
  );
});

test("a view is a function of the document, not of how its maps iterated", () => {
  const backwards = fromJson(SongSchema, reversed(document) as never);
  assert.deepStrictEqual(
    project(backwards, reversed(declared) as Manifest, reversed(logged) as HistoryAnswer),
    project(song, build, logged as HistoryAnswer),
    "an order came from the map's iteration rather than from the model (ADR 0001 §3)",
  );
});

// What the reversal reaches in the **log** (M2 PR 8), which is the one map added since the
// note below and the one that note does *not* apply to. A patch entry's key is a ULID —
// `01M1FPMP000000000000000005` — and a ULID is not an integer-like string, so JavaScript
// leaves the key order as `JSON.parse` found it and `Object.entries(…).reverse()` really
// reverses it. Same for `refs`, whose keys are branch names.
//
// Measured by deleting each sort, which is the only way to know which test is carrying which
// claim:
//
// - Deleting the entries' `sort` by id fails **both** — the golden as well, because the files
//   list ascending and the golden is newest first.
// - Deleting the refs' `sort` by name fails **only this one**. `refs.json` is already written
//   in sorted order (ADR 0002 §4), so "sorted by the view" and "whatever the object iterated"
//   are the same list until the keys are reversed. That is exactly the gap PR 4 built the
//   reversal for, and it is real here in a way it is not for a `ParamID` map.

// What the reversal above **cannot** reach, measured rather than assumed: JavaScript re-sorts
// integer-like object keys numerically ascending, before any code here runs. A `ParamID` is a
// `uint32` written out as a string, so `Instrument.params` and the manifest's `params` come
// back from `JSON.parse` in ascending numeric order whatever order the file had, and
// `Object.entries(…).reverse()` on one is a no-op — the reversal cannot separate "sorted by
// the model" from "whatever the map iterated" for a parameter map, in the way it can for every
// other map here. Checked by deleting the editor's `sort`: the golden and the reversal both
// stayed green, which is PR 4's finding one map over.
//
// The reversal is still run over the manifest, because the maps that are *not* integer-keyed —
// tracks, effects, plugins — are what the strip and device order come from. The parameter
// form's own order is asserted directly instead, which is `tests/AGENTS.md`'s constructed-value
// distinction and the same shape the bar grid below uses for the case no fixture contains.
test("a parameter form is keyed by ParamID, ordered by it, and never by the name", () => {
  const twoOfOneName = create(SongSchema, {
    tracks: {
      t: {
        id: "t",
        name: "T",
        instrument: { id: "d", ref: { kind: { case: "plugin", value: { pluginId: "p" } } } },
      },
    },
  });
  const hosts: Manifest = {
    engine: {},
    sampler: "s",
    // Two parameters with the **same display name**, which is not a contrivance: Surge XT's
    // 2855 carry 2679 distinct names, 176 fewer than there are parameters, in groups of twelve
    // — one per unassigned effect slot. A form keyed by name would show one row here.
    //
    // `b` and `a` are not VST3 ids and are here on purpose: an id JavaScript does *not* treat
    // as an array index is the only kind whose order the language leaves alone, so they are
    // what makes this assertion sensitive to the sort at all. They also exercise the string
    // comparison behind it, which is what a device whose parameters are named rather than
    // numbered would need.
    plugins: {
      p: { commit: "c", version: "1", params: { "40": "Drive", "9": "Drive", b: "B", a: "A" } },
    },
  };
  const form = editor(twoOfOneName, hosts, "d");
  assert.deepStrictEqual(
    form?.rows.map((row) => [row.id, row.name, row.value, row.declared]),
    [
      ["9", "Drive", null, true],
      ["40", "Drive", null, true],
      ["a", "A", null, true],
      ["b", "B", null, true],
    ],
    "two ids sharing a name collapsed, or an order that was the map's rather than the ids'",
  );

  // Opening an editor writes nothing, and an unset parameter is `null` rather than zero: the
  // manifest carries no defaults, so this view does not know where an untouched control sits
  // and does not claim to (ADR 0014 §1).
  assert.equal(form?.declares, 4);
  assert.equal(form?.set, 0);
});

test("a value this build cannot name is shown rather than dropped", () => {
  // A project written against a fuller manifest, opened against a smaller one — which is the
  // repository's own default (`make run` passes `tests/fixtures/manifest.json`). The validator
  // refuses such a document with `param_unknown`, and `Project::open` does not run the
  // validator, so it reaches a window. A row that vanished would be a value in the document
  // that nothing on screen can see.
  const stale = create(SongSchema, {
    tracks: {
      t: {
        id: "t",
        instrument: {
          id: "d",
          ref: { kind: { case: "plugin", value: { pluginId: "p" } } },
          params: { "7": 0.25, "12": 0.5 },
        },
      },
    },
  });
  const hosts: Manifest = {
    engine: {},
    sampler: "s",
    plugins: { p: { commit: "c", version: "1", params: { "12": "Known" } } },
  };
  assert.deepStrictEqual(
    editor(stale, hosts, "d")?.rows.map((row) => [row.id, row.declared, row.value]),
    [
      ["7", false, 0.25],
      ["12", true, 0.5],
    ],
  );

  // A device this build resolves to no plugin has no rows and no overrides to show either —
  // `core/src/validate.rs` does not resolve a `SamplerRef` against the manifest, so neither
  // does this, and offering sfizz's controls here would offer writes nothing refuses.
  const sampler = create(SongSchema, {
    tracks: {
      t: {
        id: "t",
        instrument: { id: "d", ref: { kind: { case: "sampler", value: { sfzHash: "abcdef01" } } } },
      },
    },
  });
  const form = editor(sampler, hosts, "d");
  assert.deepStrictEqual([form?.device, form?.declares, form?.rows.length], [
    "sampler abcdef01",
    0,
    0,
  ]);
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
