// The history view, as a pure function of the log (docs/specs.md §5, §9; ADR 0001, ADR 0012 §2).
//
// §5 calls the patch log two things at once — the undo/redo history and the **audit trail** —
// and this is the view that makes the second one a thing a person can read. M0.3 shipped a
// defect where MCP's `get_history` dropped `provenance` on the way out; a field existing on the
// wire and a person being able to see who made a change are different claims, and this file
// makes the second.
//
// It projects the log, not the song, and that is the one thing to be careful about: history is
// **not** song state (CLAUDE.md #1 is not in play — `core/src/history.rs` says the same about
// its own serialisation adapter), so holding the answer is not a second representation of the
// model. Nothing here derives a document from the log; `get_song` does that, in `core`.
//
// What it deliberately does **not** show is a version. `version` counts per branch (ADR 0005
// §4's caveat), so two branches can hold the same number over different content and a column
// of them would read as a sequence when it is not one. A row's identity is its ULID, which is
// unique across every branch by construction.

import { fromJson } from "@bufbuild/protobuf";
import type { JsonValue } from "@bufbuild/protobuf";
import { timestampDate } from "@bufbuild/protobuf/wkt";
import { ProvenanceSchema } from "@escribass/schema/song";

/**
 * One entry of the log, as `get_history` carries it.
 *
 * Hand-written, and allowed to be for `App.tsx`'s `ToolAnswer` reason: this is the *tool API's*
 * answer rather than song state, and §4.1's rule is about model types. `PatchEntry.ops` is
 * `bytes` in `history.proto` and would arrive base64 through the generated decoder, while
 * `core/src/call.rs` deliberately sends the RFC 6902 array — ADR 0006 §6's exception, the same
 * one `ToolResult.patch` takes. Decoding through the generated type would ask for the encoding
 * this boundary exists to avoid.
 *
 * `provenance` is the exception to the exception: it **is** a model type, so it is not
 * described here and is decoded by the generated `ProvenanceSchema` below (§4.1).
 */
export interface LogEntry {
  readonly id: string;
  readonly parents: readonly string[];
  readonly tool: string;
  readonly ops: readonly unknown[];
  readonly provenance: JsonValue;
  readonly schema_version: number;
}

/** What `get_history` answers with: every entry keyed by id, `HEAD`, and the refs
 *  (`core/src/call.rs`, `history_json`; `song_tools.proto`, `HistoryResponse`). */
export interface HistoryAnswer {
  readonly entries: Readonly<Record<string, LogEntry>>;
  readonly head: string;
  readonly refs: Readonly<Record<string, string>>;
}

export interface LogRow {
  readonly id: string;
  /** The tool that made the change — including `undo` and `redo`, which are entries like any
   *  other. Nothing marks them out, because nothing about them is different: undo appends an
   *  inverse entry rather than rewinding a ref (ADR 0005 §4), so an undo is a thing that
   *  happened and the audit trail says so. What skips them is the ⌘Z *walk*, in the session,
   *  because ⌘Z means the change before this one; a reader of the log is not walking it. */
  readonly tool: string;
  /** `AUTHOR_MODEL` as a word — the provenance column §5 asks for. */
  readonly author: string;
  /** ISO 8601, in UTC. Never a locale format: this description is goldened (ADR 0012 §5), and
   *  `toLocaleString` would make the golden a property of the machine that ran it. */
  readonly createdAt: string;
  /** How many RFC 6902 operations the entry carries. Zero is legal and means one thing only:
   *  a merge whose every conflict was resolved in this branch's favour, which changes no
   *  document and still joins two lines of history (ADR 0015 §3). */
  readonly ops: number;
  readonly parents: readonly string[];
  /** Two parents, which is the only thing that distinguishes a merge (ADR 0001 §1). */
  readonly merge: boolean;
  /** On the current branch: reachable from `HEAD` along first parents. An entry that is not is
   *  still in the log and still shown — a discarded branch's entries stay, unreferenced and
   *  inert (ADR 0001 §2), and an audit trail that hid them would not be one. */
  readonly onBranch: boolean;
  /** The branch names pointing at this entry, sorted. */
  readonly refs: readonly string[];
}

export interface Branch {
  readonly name: string;
  readonly entryId: string;
  /** Whether `HEAD` names this ref. There is no detached state (ADR 0001 §2). */
  readonly head: boolean;
}

export interface Log {
  /** The ref `HEAD` names. */
  readonly head: string;
  readonly branches: readonly Branch[];
  /** Newest first. ULIDs sort lexicographically in creation order (ADR 0001 §3), so this is
   *  chronological without reading a clock — and without trusting the object's key order,
   *  which is what the reversal half of the projection golden checks. */
  readonly entries: readonly LogRow[];
}

/** `AUTHOR_MODEL` → `model`. The same shape `mixer.ts` uses for `TrackKind`, and kept beside
 *  its own view for the same reason: a spelling shared between two views makes one depend on
 *  the other, and neither is the model's. */
const AUTHORS = ["unspecified", "human", "model"];

/**
 * The entries reachable from `HEAD` along **first parents**.
 *
 * First parents and not every ancestor, which is `core/src/history.rs`'s rule and is the same
 * one for a different reason: there a merge entry's ops already carry the other side's
 * contribution, so replaying that side again applies it twice; here the first-parent chain is
 * what "this branch" means, and the branch that was merged *in* is a branch of its own.
 */
function chain(answer: HistoryAnswer): ReadonlySet<string> {
  const reached = new Set<string>();
  let at: string | undefined = answer.refs[answer.head];
  while (at !== undefined && !reached.has(at)) {
    reached.add(at);
    at = answer.entries[at]?.parents[0];
  }
  return reached;
}

/**
 * The log, as the view draws it.
 *
 * `ponytail:` the whole log crosses on every read and every row is projected. A session's worth
 * of edits is a few hundred entries and this is a table; the escape, if a long-lived project
 * ever drags, is the one ADR 0012 §2 already names for the song — a narrower read, not a cache
 * of entries held here.
 */
export function patchLog(answer: HistoryAnswer): Log {
  const onBranch = chain(answer);

  const branches = Object.entries(answer.refs)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, entryId]) => ({ name, entryId, head: name === answer.head }));

  // Derived from the sorted branch list rather than sorted a second time: two sorts over one
  // map is one sort a test can be blind to, and the reversal half of the projection golden
  // only reaches an order something actually reorders.
  const pointing = new Map<string, string[]>();
  for (const held of branches) {
    pointing.set(held.entryId, [...(pointing.get(held.entryId) ?? []), held.name]);
  }

  const entries = Object.values(answer.entries)
    .sort((a, b) => b.id.localeCompare(a.id))
    .map((entry) => {
      // Through the generated decoder, because `Provenance` is a model type and §4.1 forbids
      // a hand-written one. It is also what turns `AUTHOR_MODEL` into the enum this indexes,
      // rather than a string this file would have to know the spelling of.
      const made = fromJson(ProvenanceSchema, entry.provenance as never);
      return {
        id: entry.id,
        tool: entry.tool,
        author: AUTHORS[made.author] ?? "unspecified",
        createdAt: made.createdAt ? timestampDate(made.createdAt).toISOString() : "",
        ops: entry.ops.length,
        parents: entry.parents,
        merge: entry.parents.length > 1,
        onBranch: onBranch.has(entry.id),
        refs: pointing.get(entry.id) ?? [],
      };
    });

  return { head: answer.head, branches, entries };
}
