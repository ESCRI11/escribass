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
import { diagnosticAt, generators, status } from "../src/generators.js";
import { assistant as aiPanel, dashed, dashedNotes, moved, refusalLine } from "../src/assistant.js";
import type { Called, PanelAnswer } from "../src/assistant.js";
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

// The code view needs a song with **generators in it**, and `render`'s has none — `"generators":
// {}`, as every determinism golden but one does. A golden cannot fail for an input it has never
// been given, which is the finding M4 PR 6 made one reader over when ADR 0026 §4 predicted the
// bar view's golden would move and it could not. So the generators come from a second document,
// and it is the same one `ai/tests/test_view.py` reads for the same reason: PR 5's `generators`
// golden, built through the tool API like every other (CLAUDE.md #2), whose four generators are
// two compiled and two never, with a seed above 2⁵³ beside one that fits in a byte, a target
// that is a **track** rather than a clip, and a source that cannot compile at all.
const GENERATORS = new URL(
  "../../tests/determinism/generators/expected/song.json",
  import.meta.url,
);

// What `ai` prints about those same four generators, as committed bytes (ADR 0026 §4). The
// editor's status word and the bar view's are read from one field by two readers in two
// languages, and this file is where they are compared: trap 3's shape is two readers of one
// hash disagreeing, and a status word is how it would show.
const BAR_VIEW = new URL("../../ai/tests/golden/view-generators.txt", import.meta.url);

// The AI panel needs a **turn**, and a model's log row needs an entry a model wrote. Both come
// from the `proposal` determinism golden, which is the end-to-end run of M3 PR 8: a real
// recorded transcript replayed through the real `ai` process, its two calls executed against a
// proposal, and the proposal applied as one entry. So the panel is goldened over the turn this
// repository actually produced rather than over a conversation written here — and if that run
// changes, this golden moves with it and says so.
const TURN = new URL("../../tests/determinism/proposal/expected/", import.meta.url);

/** The prompt that turn answered, as `tests/determinism.rs` sends it. Written literally, as
 *  every fixture id here is: a golden that read it from somewhere would agree with whatever it
 *  found. */
const ASKED = "add a lead line over the bass";

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
const written: unknown = JSON.parse(read(GENERATORS));
const writing: Song = fromJson(SongSchema, written as never);

/** The `proposal` golden's own log: three entries a person wrote and one a model did. */
function readModelLog(): HistoryAnswer {
  const at = new URL("patches/", TURN);
  const entries = Object.fromEntries(
    readdirSync(at)
      .filter((name) => name.endsWith(".json"))
      .sort()
      .map((name) => {
        const entry = JSON.parse(read(new URL(name, at))) as LogEntry;
        return [entry.id, entry];
      }),
  );
  const refs = JSON.parse(read(new URL("refs.json", TURN))) as {
    head: string;
    refs: Record<string, string>;
  };
  return { entries, head: refs.head, refs: refs.refs };
}

const modelLogged: unknown = readModelLog();

/**
 * What the host answered while that turn was pending, and after it was applied.
 *
 * Built from the golden's `responses.json`, which records the turn as `Sidecar::turn` produced
 * it and the patch as `Proposal::patch` computed it — the same two values `panel.rs` puts on
 * `Live`. Nothing here is invented: the call ids are the ones the model was given on
 * 2026-09-24, the summaries are the tool API's own, and the operations are the patch Apply
 * sent.
 */
function readTurn(): { pending: PanelAnswer; settled: PanelAnswer } {
  const answers = JSON.parse(read(new URL("responses.json", TURN))) as Record<string, any>[];
  const turn = answers.find((answer) => answer.turn !== undefined)!.turn;
  const waiting = answers.find((answer) => answer.pending !== undefined)!.pending;
  const applied = answers.find((answer) => answer.applied !== undefined)!.applied;
  const calls: Called[] = turn.calls.map((call: Record<string, unknown>) => ({
    call_id: call.call_id as string,
    name: call.name as string,
    args: call.args as string,
    model_id: turn.model_id as string,
    valid: call.valid as boolean,
    summary: call.summary as string,
    errors: call.errors as never,
  }));
  return {
    // What the window drew **before apply**, which is the proof point §18.2 leads with: the
    // diff on screen, nothing written, three controls.
    pending: {
      provider: "openrouter",
      model: "deepseek/deepseek-v4.1-flash",
      at: "/home/a/.local/share/dev.escribass.app/conversations/01M1FPMPSONG.jsonl",
      conversation: [],
      live: {
        prompt: ASKED,
        prompt_id: waiting.prompt_id,
        at: "2026-09-24T12:00:00Z",
        model_id: turn.model_id,
        calls,
        reply: turn.reply,
        running: false,
        ended: turn.end,
        patch: waiting.ops,
        refused: [],
        song: null,
        refusal: null,
      },
    },
    // And after: the turn is in the conversation with what became of it, and nothing is
    // pending (ADR 0021 §3).
    settled: {
      provider: "openrouter",
      model: "deepseek/deepseek-v4.1-flash",
      at: "",
      conversation: [
        {
          prompt_id: waiting.prompt_id,
          prompt: ASKED,
          at: "2026-09-24T12:00:00Z",
          provider: "openrouter",
          model_id: turn.model_id,
          calls,
          reply: turn.reply,
          outcome: { applied: applied.entry_id },
        },
      ],
      live: null,
    },
  };
}

const turns = readTurn();

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
function project(
  from: Song,
  hosts: Manifest,
  history: HistoryAnswer,
  modelHistory: HistoryAnswer,
  generating: Song,
) {
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
    // The code view, over the **second** document — see `GENERATORS` above. What it carries is
    // the source, the seed as the string a `uint64` crosses JSON as, the toolchain and whether
    // `compiled_hash` is empty; what it deliberately does not carry is the status word, which
    // is a tool's answer rather than a projection (ADR 0024 §6). So the four rows below are
    // exactly what a title bar can say without asking `core` anything.
    generators: generators(generating),
    // The one view whose input is not the song. It is here rather than in a golden of its own
    // because what §11's fifth bullet claims is about *every* view, and a second file would
    // be a second place to forget one (ADR 0012 §5).
    log: patchLog(history),
    // A log with a **model's** entry in it: `proposal`, the model that answered beside the
    // author, and the `prompt_id` that leads to the conversation (ADR 0021 §2). The
    // `branches` log above has none, and a column nothing in a fixture fills is a column a
    // golden cannot see.
    modelLog: patchLog(modelHistory),
    // The AI panel, before the proposal is applied and after (§9; ADR 0019 §2, §3). It is a
    // projection of the host's answer rather than of the song — but it is the same claim:
    // what the window draws is a function of what it was given, and the one place it reads
    // the document is to name what moved.
    assistant: {
      pending: aiPanel(turns.pending, from),
      settled: aiPanel(turns.settled, from),
    },
  };
}

test("every view is a pure function of the model", () => {
  const before = toJson(SongSchema, song);
  const manifestBefore = JSON.stringify(declared);
  const logBefore = JSON.stringify(logged);
  const modelLogBefore = JSON.stringify(modelLogged);
  const writingBefore = toJson(SongSchema, writing);
  const actual = `${JSON.stringify(
    project(song, build, logged as HistoryAnswer, modelLogged as HistoryAnswer, writing),
    null,
    2,
  )}\n`;

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
  assert.equal(JSON.stringify(modelLogged), modelLogBefore, "a projection wrote to the log");
  assert.deepStrictEqual(
    toJson(SongSchema, writing),
    writingBefore,
    "a projection wrote to the generators' document",
  );
  assert.deepStrictEqual(
    project(song, build, logged as HistoryAnswer, modelLogged as HistoryAnswer, writing),
    project(song, build, logged as HistoryAnswer, modelLogged as HistoryAnswer, writing),
    "two projections of one song differ",
  );
});

// A turn that ended on its refusal budget, which no fixture in this tree has: the recorded run
// composed on the first attempt with **zero** refusals (ADR 0022 §4), so every claim the panel
// makes about a refused turn went unchecked until the M3 review found it (2026-09-24,
// mutations A2 and A3). It is the recorded turn with the two things a refused one differs by:
// the calls came back `valid: false`, carrying the tool API's own violation, and the host
// ended it `Refused` rather than `Answered` (`core::TurnEnd`).
//
// Asserted rather than goldened. What the golden is for is a whole projection of a whole
// fixture; what these are is two sentences about one function, and a second recorded turn is
// not something this repository can buy with a hand-written one.
function aRefusedTurn(live: Partial<PanelAnswer["live"] & object> = {}): PanelAnswer {
  const answered = turns.pending;
  const was = answered.live!;
  return {
    ...answered,
    live: {
      ...was,
      ended: "Refused",
      // The **last** call refused and the ones before it accepted, which is the shape a turn
      // that ran out of budget really has — and the shape that tells a count of refusals apart
      // from a count of calls.
      calls: was.calls.map((call, at) =>
        at < was.calls.length - 1
          ? call
          : {
              ...call,
              valid: false,
              summary: "",
              errors: [
                {
                  path: "/clips/01M1FPMP000000000000000036/track_id",
                  rule: "track_unknown",
                  message: "no track `01M1FPMP00NOSUCHTRACK00000`",
                },
              ],
            },
      ),
      ...live,
    },
  };
}

test("a turn that ended on its refusal budget says how many, and offers nothing to apply", () => {
  // The panel's note for `ended === "Refused"`, and `pending.applicable`. Watched failing
  // first: with the note's `filter` dropped the count read `0 calls`, and with `applicable`
  // read off `running` alone the empty proposal below still offered Apply.
  assert.equal(
    aiPanel(aRefusedTurn(), song).messages.at(-1)!.note,
    "the turn ended: 1 call was refused",
  );

  // Nothing to apply, because there is nothing to approve: a refused turn leaves the proposal
  // pending with whatever the accepted calls put in it, and here that is nothing.
  const nothing = aiPanel(aRefusedTurn({ patch: [] }), song);
  assert.equal(nothing.pending!.applicable, false);
  assert.equal(nothing.pending!.summary, "the model proposed no change");

  // A turn still running is never applicable either, whatever it has proposed so far.
  const running = aiPanel(aRefusedTurn({ running: true, ended: "" }), song);
  assert.equal(running.pending!.applicable, false);
  assert.equal(running.messages.at(-1)!.note, "answering…");
});

test("a proposal whose patch will not prepare offers nothing and says what moved", () => {
  // The other shape `panel.rs` leaves behind: `patch` null and `refused` carrying the
  // violations (`Live::watch`). The two are exclusive, which is why `applicable` does not test
  // both — see the comment on it. `document_moved` is the rule `apply_proposal` refuses a
  // moved document with, and the panel names the thing rather than the path (ADR 0019 §2,
  // amended 2026-09-24).
  const panel = aiPanel(
    aRefusedTurn({
      patch: null,
      refused: [
        {
          // A clip the fixture really has, so the panel can name the track it sits on
          // rather than echoing a path back at a person.
          path: "/clips/01M1FPMP00000000000000000K/note_clip/notes/x/pitch",
          rule: "document_moved",
          message: "this changed after the proposal was composed",
        },
      ],
    }),
    song,
  );
  assert.equal(panel.pending!.applicable, false);
  assert.deepEqual(panel.pending!.refusals, [
    "the clip on Lead changed while this proposal was pending",
  ]);
});

// The status word, and the one thing about it that cannot be goldened: that two readers of
// `compiled_hash` in two languages say the same word about the same generator. The editor's is
// `status()` in `generators.ts`; the model's is the `generators:` line of `ai`'s bar view, which
// is committed bytes. If either ever grows a rule of its own — a hash recomputed here, a
// `stale` guessed there — this is what notices (trap 3; ADR 0026 §4).
test("the editor and the model's bar view read one field and say the same word", () => {
  const line = read(BAR_VIEW)
    .split("\n")
    .find((held) => held.startsWith("generators: "))!
    .slice("generators: ".length);
  // `<id> python → <target> seed <n> compiled|never`, in id order, separated by ` · `.
  const printed = line.split(" · ").map((held) => {
    const parts = held.split(" ");
    return { id: parts[0], kind: parts[1], target: parts[3], seed: parts[5], word: parts[6] };
  });
  assert.equal(printed.length, 4, "the bar view's golden no longer holds four generators");

  const view = generators(writing).generators;
  assert.deepStrictEqual(
    view.map((held) => ({
      id: held.id,
      kind: held.kind,
      target: held.target.id,
      seed: held.seed,
      // Nothing standing from `core`, which is what a window shows when it opens: exactly what
      // the model is told, by construction.
      word: status(held, null),
    })),
    printed,
    "the window and the bar view disagree about a generator, its seed or whether it has compiled",
  );

  // Two compiled and two never, with `never` winning over an answer: a dry run of a generator
  // that has never compiled *does* come back with a patch, and the word is still `never`,
  // because that patch is its first compile rather than a refresh (`generators.ts`).
  assert.deepStrictEqual(
    view.map((held) => [status(held, "stale"), status(held, "compiled")]),
    [
      ["stale", "compiled"],
      ["stale", "compiled"],
      ["never", "never"],
      ["never", "never"],
    ],
  );

  // And the seed is a **string**, because `9007199254740993` is 2⁵³+1 and the first integer a
  // double cannot hold: through a `number` it would read 9007199254740992 here and in the patch
  // a save writes (trap 10).
  assert.equal(view[0].seed, "9007199254740993");
  assert.equal(Number(view[0].seed).toString(), "9007199254740992", "the reason it is a string");

  // A generator whose target is a **track** is a generator M4 refuses to compile,
  // `target_not_note_clip` — and the view still shows it, because a document that holds one is
  // valid and the editor is where a person would fix it (ADR 0024 §5).
  assert.deepStrictEqual(
    view.filter((held) => held.target.kind === "track").map((held) => held.id),
    ["01M1FPMP000000000000000019"],
  );
});

// What the editor puts in the margin, from what `core` puts in a refusal. The string is the one
// `tests/determinism/compile/` drove through the real child in M4 PR 6, so this is the same
// sentence the model was handed, read by the window.
test("a compile diagnostic is read back to the line and column the child named", () => {
  const said = diagnosticAt([
    {
      rule: "generator_error",
      message:
        "4:24: `/` (Div) is not in the generator DSL: write a // b, or Fraction(a, b) for an " +
        "exact ratio (ADR 0024 §3)",
    },
  ]);
  assert.equal(said?.line, 4);
  assert.equal(said?.column, 24);
  // The child's words, whole and unrewritten: the message is the whole fix (ADR 0026 §3).
  assert.equal(
    said?.message,
    "`/` (Div) is not in the generator DSL: write a // b, or Fraction(a, b) for an exact ratio " +
      "(ADR 0024 §3)",
  );

  // A column of 0 is how `core` says the child had no position, and it writes the line alone
  // (`core/src/session.rs`). The whole line is then what the editor marks.
  const whole = diagnosticAt([{ rule: "generator_error", message: "7: the CPU limit of 5 s" }]);
  assert.deepStrictEqual(whole, { line: 7, column: 0, message: "the CPU limit of 5 s" });

  // The two refusals that are about the document rather than the source carry no line, so
  // nothing goes in the margin — they are shown where every refusal is, in the pane head.
  assert.equal(
    diagnosticAt([
      { rule: "target_not_note_clip", message: "`…` is a track, and a compile writes a clip" },
      { rule: "generator_unknown", message: "`x` is not a generator in this song" },
    ]),
    null,
  );
  assert.equal(diagnosticAt([]), null);

  // A `generator_error` with no position at all: nothing writes one today, and dropping it
  // would lose the refusal silently, so it lands at the top of the file.
  assert.deepStrictEqual(
    diagnosticAt([{ rule: "generator_error", message: "something nobody formatted" }]),
    { line: 1, column: 0, message: "something nobody formatted" },
  );
});

test("a view is a function of the document, not of how its maps iterated", () => {
  const backwards = fromJson(SongSchema, reversed(document) as never);
  assert.deepStrictEqual(
    project(
      backwards,
      reversed(declared) as Manifest,
      reversed(logged) as HistoryAnswer,
      reversed(modelLogged) as HistoryAnswer,
      fromJson(SongSchema, reversed(written) as never),
    ),
    project(song, build, logged as HistoryAnswer, modelLogged as HistoryAnswer, writing),
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

// The case no turn in this repository produces, asserted against constructed values — the
// distinction `tests/AGENTS.md` draws and the one the bar grid above already uses. A proposal
// is refused when the document moved under it (ADR 0005 §3), and ADR 0012 §4 asks that the
// refusal **name what moved** rather than quoting a number. Nothing in the `proposal` golden
// is refused, so nothing in the golden above can carry this claim.
test("a refused proposal names what moved, from the document rather than a number", () => {
  const track = Object.values(song.tracks).find((held) => held.name === "Lead")!;
  const clip = Object.values(song.clips).find((held) => held.trackId === track.id)!;
  const section = Object.values(song.sections)[0];

  assert.equal(
    refusalLine(song, {
      path: `/tracks/${track.id}/version`,
      rule: "version_not_writable",
      message: "`version` is maintained by core and is never written by a tool op",
    }),
    "the Lead track changed while this proposal was pending",
  );
  // The song's own version, which is what refuses a proposal when two entries landed anywhere
  // at all (ADR 0019, Consequences).
  assert.equal(
    refusalLine(song, { path: "/version", rule: "version_not_writable", message: "…" }),
    "the song changed while this proposal was pending",
  );
  assert.equal(moved(song, `/clips/${clip.id}/start_tick`), "the clip on Lead");
  assert.equal(moved(song, `/sections/${section.id}/name`), `the ${section.name} section`);
  assert.equal(moved(song, `/tracks/${track.id}/instrument/params/9`), "the Lead track");
  assert.equal(moved(song, "/tempo_map/events/0/bpm"), "the tempo map");
  // A rule that is not the version guard keeps the validator's own sentence, which is the
  // half a person acts on; what is added is the name, never a rule invented here (ADR 0017 §3).
  assert.equal(
    refusalLine(song, {
      path: `/clips/${clip.id}/length_ticks`,
      rule: "clip_overlap",
      message: "clips overlap on a track that does not allow it",
    }),
    "the clip on Lead: clips overlap on a track that does not allow it",
  );
  // A path nothing in the document answers to is left as it is rather than guessed at.
  assert.equal(moved(song, "/clips/nope/start_tick"), "/clips/nope/start_tick");
  assert.equal(moved(null, "/tracks/x/version"), "/tracks/x/version");
});

// What the timeline and the roll draw dashed while a proposal is pending (ADR 0019 §2).
// Constructed values, because the shape they compare is a *projection's* and what has to be
// right about them is which of four cases each falls in — and the `proposal` golden's turn
// produces only one of the four.
test("a pending proposal is drawn over the model, never instead of it", () => {
  const note = (id: string, pitch: number) => ({ id, pitch, startTick: 0, lengthTicks: 480 });
  const clip = (id: string, startTick: number) => ({ id, startTick, lengthTicks: 960 });
  // A note moved, a note added, a note removed, and a note untouched.
  const before = [note("a", 60), note("b", 62), note("c", 64)];
  const after = [note("a", 72), note("b", 62), note("d", 67)];
  const { pending, gone } = dashedNotes(before, after);
  assert.deepStrictEqual(
    pending.map((held) => [held.id, held.pitch]),
    [
      ["a", 72],
      ["d", 67],
    ],
    "the moved note and the added one are what a proposal asks for",
  );
  assert.deepStrictEqual(gone.map((held) => held.id), ["c"], "only a removed note is gone");
  // The document's own notes are never touched: the roll goes on drawing all three solid, so
  // the moved one is visible in both places and Reject puts back something visible.
  assert.deepStrictEqual(before.map((held) => held.pitch), [60, 62, 64]);
  assert.deepStrictEqual(dashedNotes(before, before).pending, []);
  assert.deepStrictEqual(dashedNotes(before, before).gone, []);

  // **No proposal is not an empty proposal**, and the difference is a defect this had: with
  // `[]` every note in the document is one the proposal does not have, so the roll outlined
  // all of them in the refusal colour a second after Apply. Watched in the window.
  assert.deepStrictEqual(dashedNotes(before, null), { pending: [], gone: [] });
  assert.deepStrictEqual([...dashed([{ id: "t", clips: [clip("one", 0)] }], null)], []);

  // And the timeline: a clip the proposal added or changed is dashed, one it left alone is
  // not. The comparison is over what the projection describes, so a clip that moved, grew,
  // changed content or started looping is caught and one whose track's mix changed is not.
  assert.deepStrictEqual(
    [
      ...dashed(
        [{ id: "t", clips: [clip("one", 0), clip("two", 960)] }],
        [{ id: "t", clips: [clip("one", 0), clip("two", 1920), clip("three", 0)] }],
      ),
    ],
    ["two", "three"],
  );
});
