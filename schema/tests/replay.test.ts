// ADR 0002 §11's claim, checked rather than asserted: the patch log is ordinary RFC 6902 over
// ordinary JSON, so *any* implementation can replay it — not only the one that wrote it.
//
// This walks the golden the determinism suite commits: `refs.json` names HEAD, the entries are
// followed by first parent back to the root, and their operations are applied to an empty
// document. The result must equal `song.json` byte for byte.
//
// The pointer apply below is deliberately hand-rolled and about ten lines. A JSON Patch library
// would test the library; ten lines test the claim, and adding a dependency to check that no
// dependency is needed would be its own answer.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const golden = (script: string, p: string) =>
  readFileSync(new URL(`../../tests/determinism/${script}/expected/${p}`, import.meta.url), "utf8");

type Op = { op: string; path: string; value?: unknown; from?: string };
type Entry = { id: string; parents: string[]; ops: Op[] };

// RFC 6901: `~1` becomes `/` and `~0` becomes `~`, in that order — the reverse turns `~01`
// into `/` instead of `~1`.
const tokens = (pointer: string) =>
  pointer === "" ? [] : pointer.slice(1).split("/").map((t) => t.replace(/~1/g, "/").replace(/~0/g, "~"));

function apply(document: unknown, ops: Op[]): unknown {
  let root = document;
  for (const op of ops) {
    const path = tokens(op.path);
    if (path.length === 0) {
      assert.equal(op.op, "replace", `cannot ${op.op} the whole document`);
      root = op.value;
      continue;
    }
    let at = root as Record<string, unknown>;
    for (const token of path.slice(0, -1)) {
      at = at[token] as Record<string, unknown>;
      assert.ok(at !== undefined, `no ${op.path}`);
    }
    const last = path[path.length - 1];
    // RFC 6902 distinguishes these, and so does core's `diff`. Treating `replace` as
    // assignment would accept a log a strict library rejects — which is the very thing this
    // file claims can read it.
    if (op.op === "remove") {
      assert.ok(last in at, `remove of absent ${op.path}`);
      delete at[last];
    } else if (op.op === "replace") {
      assert.ok(last in at, `replace of absent ${op.path}`);
      at[last] = op.value;
    } else if (op.op === "add") {
      at[last] = op.value;
    } else assert.fail(`the log should not contain \`${op.op}\``);
  }
  return root;
}

/// The entries from the root to HEAD, by first parent — the replay order `core` uses.
///
/// Not every ancestor: a merge entry's operations are the diff from `parents[0]`, so they
/// already carry what the other side contributed. Replaying that side as well applies its
/// changes twice, which is invisible for `add` and fatal for `remove`.
function chain(script: string): Entry[] {
  const refs = JSON.parse(golden(script, "refs.json")) as { head: string; refs: Record<string, string> };
  const entries: Entry[] = [];
  let at: string | undefined = refs.refs[refs.head];
  while (at) {
    const entry = JSON.parse(golden(script, `patches/${at}.json`)) as Entry;
    entries.push(entry);
    at = entry.parents[0];
  }
  return entries.reverse();
}

for (const name of ["every_tool", "branches"]) {
test(`replays the committed \`${name}\` log into the committed song`, () => {
  const entries = chain(name);
  assert.ok(entries.length > 1, "the golden has a log to replay");

  // Not `{}`. A patch log is not self-contained: the canonical form emits every no-presence
  // scalar and map, so `replace` is legal from the first operation only against a document
  // that already has them. `origin.json` is that starting point, committed beside the log.
  let document: unknown = JSON.parse(golden(name, "origin.json"));
  for (const entry of entries) document = apply(document, entry.ops);

  // Compared as documents, not as text. This side's serialiser is not the canonical writer:
  // `JSON.stringify` keeps insertion order where the canonical form uses proto field order, and
  // writes `90` where it writes `90.0` (ADR 0002 §4). That the *bytes* are canonical is Rust's
  // claim and Rust's test. The claim here is the one ADR 0002 §11 makes — that the log is
  // ordinary RFC 6902 over ordinary JSON, so anything can replay it into the same document.
  assert.deepEqual(document, JSON.parse(golden(name, "song.json")));
});
}

test("the log is RFC 6902 that needs no schema to read", () => {
  // Nothing above imported a generated type. The entries are plain JSON, which is what §2.6
  // means by a project being readable and diffable, and what makes the log recoverable by
  // something that has never seen this schema.
  for (const entry of chain("branches")) {
    for (const op of entry.ops) {
      assert.ok(["add", "replace", "remove"].includes(op.op), `unexpected \`${op.op}\``);
      assert.ok(op.path === "" || op.path.startsWith("/"), `not a pointer: ${op.path}`);
    }
  }
});
