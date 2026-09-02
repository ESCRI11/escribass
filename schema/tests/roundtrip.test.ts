// TypeScript half of the cross-language agreement test: the types generated for the UI
// must read the exact bytes `core` writes. Only parsing is asserted — `core` is the only
// writer of a project file (docs/specs.md §5, §10).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { equals, fromJsonString, toJsonString } from "@bufbuild/protobuf";
import { SongSchema } from "../gen/ts/song_pb.js";
import { PatchEntrySchema } from "../gen/ts/history_pb.js";

const fixture = (p: string) =>
  readFileSync(new URL(`../../tests/fixtures/${p}`, import.meta.url), "utf8");

test("reads the canonical song fixture", () => {
  const song = fromJsonString(SongSchema, fixture("song/minimal.json"));

  assert.equal(song.schemaVersion, 1);
  assert.equal(song.tracks["01K4F2T001"].name, "Bass");
  assert.equal(song.tracks["01K4F2T001"].instrument?.ref?.kind.case, "cmajor");

  const clip = song.clips["01K4F2QN8B"];
  assert.equal(clip.startTick, 61440);
  assert.equal(clip.content.case, "noteClip");
  assert.equal(clip.loopLengthTicks, undefined, "unset optional stays absent");
  if (clip.content.case !== "noteClip") throw new Error("unreachable");
  assert.equal(clip.content.value.notes["01K4F2N001"].pitch, 43);
  assert.equal(clip.content.value.notes["01K4F2N002"].expression["timbre"], 0.62);

  // 64-bit integers cross as strings in JSON and land as bigint here.
  assert.equal(song.generators["01K4F2G001"].seed, 9007199254740993n);
});

test("round-trips through the canonical form", () => {
  const song = fromJsonString(SongSchema, fixture("song/minimal.json"));
  const json = toJsonString(SongSchema, song, {
    alwaysEmitImplicit: true,
    useProtoFieldName: true,
  });
  assert.ok(equals(SongSchema, song, fromJsonString(SongSchema, json)));
});

test("reads a patch entry as RFC 6902 operations", () => {
  const entry = fromJsonString(PatchEntrySchema, fixture("history/minimal.json"));
  assert.deepEqual(entry.parents, ["01K4F2QN8B"]);
  assert.equal(entry.ops[0].op, "replace");
  assert.equal(entry.ops[0].path, "/clips/01K4F2QN8B/note_clip/notes/01K4F2N001/pitch");
  assert.equal(entry.ops[1].op, "remove");
  assert.equal(entry.ops[1].value, undefined);
});
