// TypeScript half of the cross-language agreement test: the types generated for the UI must
// read the exact bytes `core` writes. Only reading is asserted — `core` is the only writer
// of a project file (docs/specs.md §5, §10), and this side's `toJson` is not the canonical
// writer (it emits `92` where the canonical form has `92.0`, and `Z` where pbjson writes an
// offset).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { equals, fromJsonString, toJsonString } from "@bufbuild/protobuf";
import { SongSchema } from "../gen/ts/song_pb.js";

const fixture = (p: string) =>
  readFileSync(new URL(`../../tests/fixtures/${p}`, import.meta.url), "utf8");

const song = () => fromJsonString(SongSchema, fixture("song/minimal.json"));

test("reads the canonical song fixture", () => {
  const s = song();
  assert.equal(s.schemaVersion, 1);
  assert.equal(s.tracks["01M1FPMP00TRACKBASS0000002"].name, "Bass");
  assert.equal(s.tracks["01M1FPMP00TRACKBASS0000002"].instrument?.ref?.kind.case, "cmajor");
  assert.equal(s.tracks["01M1FPMP00TRACKBASS0000002"].fxChain["01M1FPMP00FXSRGE0000000005"].index, 0);
  assert.equal(s.sections["01M1FPMP00SECTCHRS00000009"].startTick, 61440);
});

test("tempo and time-signature events are keyed, not positional", () => {
  const s = song();
  assert.deepEqual(Object.keys(s.tempoMap!.events), ["01M1FPMP00TEMP00000000000E"]);
  assert.equal(s.tempoMap!.events["01M1FPMP00TEMP00000000000E"].bpm, 92);
  assert.equal(s.timeSignatureMap!.events["01M1FPMP00TMESG0000000000F"].numerator, 4);
});

test("reads both oneofs and distinguishes an unset optional", () => {
  const clip = song().clips["01M1FPMP00CPCHRS0000000006"];
  assert.equal(clip.startTick, 61440);
  assert.equal(clip.content.case, "noteClip");
  assert.equal(clip.loopLengthTicks, undefined, "unset optional stays absent");
  if (clip.content.case !== "noteClip") throw new Error("unreachable");
  assert.equal(Object.keys(clip.content.value.notes).length, 2);
  assert.equal(clip.content.value.notes["01M1FPMP00NTEG100000000007"].pitch, 43);
  assert.equal(clip.content.value.notes["01M1FPMP00NTED200000000008"].expression["timbre"], 0.62);
});

test("reads an audio clip with its gain, fades and stretch flag", () => {
  const clip = song().clips["01M1FPMP00CPGTR0000000000H"];
  assert.equal(clip.content.case, "audioClip");
  if (clip.content.case !== "audioClip") throw new Error("unreachable");
  assert.equal(clip.content.value.assetHash, "3f7a9c1e5b2d00000000000000000000");
  assert.equal(clip.content.value.gainDb, -4.5);
  assert.equal(clip.content.value.fadeInTicks, 240);
  assert.equal(clip.content.value.fadeOutTicks, 480);
  assert.equal(clip.content.value.timeStretch, true);
});

test("a 64-bit field crosses as a string and lands as an exact bigint", () => {
  // Written as a JSON number this would round to 9007199254740992 here, which is why the
  // canonical form encodes 64-bit integers as strings.
  assert.equal(song().generators["01M1FPMP00GENCHRS00000000D"].seed, 9007199254740993n);
});

test("re-reads its own output unchanged", () => {
  const s = song();
  const json = toJsonString(SongSchema, s, {
    alwaysEmitImplicit: true,
    useProtoFieldName: true,
  });
  assert.ok(equals(SongSchema, s, fromJsonString(SongSchema, json)));
});
