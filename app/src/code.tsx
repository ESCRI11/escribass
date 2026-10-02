// The generator code view — CodeMirror over `Generator.source` (docs/specs.md §9, ADR 0024 §5,
// §6; ADR 0026 §2). CodeMirror's first consumer in this repository: 6.0.2 was pinned at M2 with
// no consumer at all (ADR 0016 §2), and this is it.
//
// **The buffer is a draft and never a second representation of the song** (CLAUDE.md #1, trap
// 7). What this file holds is text somebody is typing, which is the same kind of thing as the
// new-branch box and the prompt box in `App.tsx` — not a generator's source until `apply_patch`
// says so. Three rules keep it that way, and each is visible from outside:
//
//   1. **Only Save reads the buffer.** Compile sends a generator *id*, and `core` compiles what
//      the document holds — so nothing compiles text nobody has committed. Compile is therefore
//      disabled while the buffer differs from the document and says why, which is trap 7's rule
//      made visible instead of merely obeyed.
//   2. **A clean buffer follows the document**, every time it is re-read: a model's patch, a
//      branch switch or an undo lands here as it lands in every other view (ADR 0012 §2).
//   3. **A dirty buffer is never clobbered, and a document that moved under one is *said*.** The
//      draft remembers the text it was taken from, so "the document moved" is a comparison and
//      not a guess, and the author chooses — Save overwrites, Discard takes theirs. That is trap
//      7's "a re-read replaces the buffer or asks", with both arms.
//
// **The status word** is `never`, `compiled` or `stale`, and the window computes only the first
// two. They are `compiled_hash`'s emptiness, which is the word `ai`'s bar view prints off the
// same field (ADR 0026 §4); `stale` is `core`'s comparison of a fresh hash against the stored
// one, and it reaches the window as the answer to a **dry run of `compile_generator`** — an
// empty patch is *up to date*, a patch is stale (ADR 0024 §6). There is no hasher here and there
// is not going to be one: two hashers that canonicalise one key differently is trap 3, with a
// status word as the symptom.
//
// **Compile is a gesture**, and nothing new: `compile_generator` as a dry run, the patch it
// answers with drawn as the diff, and Apply the same call once more without `dry_run` — ADR 0017
// §4, through the same `propose`/`Pending`/`apply` three the roll and the mixer go through. Save
// is a gesture too, for the same reason and against the obvious objection that a person can
// already see what they typed: §9 asks for the diff before applying from every control that is a
// tool call, and what the diff shows is `core`'s own re-derived `replace` with the `version`
// bumps beside it — which is the half the editor cannot show.

import { useEffect, useRef } from "react";
import { EditorView, basicSetup } from "codemirror";
// Declared and pinned at 6.9.7 in `lock.baseline.json` and `app/package.json`, because this
// file imports it (CLAUDE.md #4; §17's rule is about what the build imports, not about how npm
// happened to hoist it). It first arrived as `codemirror` 6.0.2's own dependency and was
// imported from there, which made it a direct dependency in fact and an undeclared one on
// paper — the shape of unstated assumption this repository keeps finding, so the user's answer
// was to say it out loud. Declaring it changed no resolution: 108 lockfile entries identical,
// 25 installed packages identical, one line added to the root's dependency list.
import { setDiagnostics } from "@codemirror/lint";
import type { Diagnostic as Said, GeneratorView, Status } from "./generators.js";

/**
 * What a person is typing, and the document text it was taken from.
 *
 * `from` is what makes "the document moved under this edit" a comparison rather than a
 * suspicion. It is **not** song state: nothing derives a document from it, nothing but [`Code`]
 * reads it, and it goes when the draft goes.
 */
export interface Draft {
  /** Which generator this draft belongs to, so a draft cannot be shown over another's source. */
  readonly id: string;
  readonly from: string;
  readonly text: string;
}

export function Code({
  view,
  openable,
  draft,
  status,
  checked,
  said,
  onOpen,
  onDraft,
  onSave,
  onCompile,
}: {
  view: GeneratorView;
  /** Every generator this editor can be opened on, in the model's own order. */
  openable: readonly GeneratorView[];
  /** The draft for *this* generator, or `null` when the editor is showing the document. */
  draft: Draft | null;
  status: Status;
  /** What `core` answered a standing dry run with when it had nothing to apply — *up to date*,
   *  in its own words — or `null` when nothing is standing or the answer carried a patch. */
  checked: string | null;
  /** The compile's own diagnostic, at the line the child named, or `null`. */
  said: Said | null;
  onOpen: (id: string) => void;
  onDraft: (draft: Draft | null) => void;
  /** One `apply_patch` on `/generators/{id}/source`, proposed (ADR 0026 §2). */
  onSave: () => void;
  /** One dry run of `compile_generator` (ADR 0024 §6). */
  onCompile: () => void;
}) {
  const host = useRef<HTMLDivElement | null>(null);
  const editor = useRef<EditorView | null>(null);

  const dirty = draft !== null && draft.text !== view.source;
  const moved = dirty && draft.from !== view.source;

  // What a draft taken *now* would be taken from: the document's text while the buffer is clean.
  // Read at the first keystroke rather than when the view was built, so an edit begun after the
  // document moved under a clean buffer records the text the author was looking at.
  const from = useRef(view.source);
  if (!dirty) from.current = view.source;

  // The callback the update listener closes over, held in a ref so the listener is installed
  // once per generator instead of being rebuilt whenever the parent re-renders. A CodeMirror
  // view rebuilt mid-keystroke loses the cursor, which is why neither this nor the buffer's
  // current text is in the effect's dependency list.
  const typed = useRef(onDraft);
  typed.current = onDraft;

  // One view per generator opened. Switching builds a new one, which is also how the undo
  // history stops reaching across two sources — CodeMirror's history belongs to the buffer, and
  // a ⌘Z that walked back into the previous generator's text would be editing the wrong file.
  useEffect(() => {
    const parent = host.current;
    if (!parent) return;
    const id = view.id;
    const held = new EditorView({
      doc: draft?.text ?? view.source,
      extensions: [
        basicSetup,
        EditorView.updateListener.of((update) => {
          if (!update.docChanged) return;
          // Every keystroke lands here and **no keystroke reaches the tool API** (trap 8): a
          // compile's dry run spawns a Python interpreter, and wired to this event that is a
          // process per character, presenting as a laggy editor and living in `core`. What a
          // keystroke does is move text inside this file.
          typed.current({ id, from: from.current, text: update.state.doc.toString() });
        }),
      ],
      parent,
    });
    editor.current = held;
    return () => {
      held.destroy();
      editor.current = null;
    };
  }, [view.id]);

  // Rules 2 and 3: a clean buffer is replaced by the document, a dirty one is left alone.
  useEffect(() => {
    const held = editor.current;
    if (!held || dirty) return;
    const showing = held.state.doc.toString();
    if (showing === view.source) return;
    held.dispatch({ changes: { from: 0, to: showing.length, insert: view.source } });
  }, [view.source, dirty]);

  // The diagnostic, at the line the child named (ADR 0026 §3). `setDiagnostics` installs its own
  // state field, so the editor carries nothing until a compile is refused, and clearing it is
  // the same call with an empty list.
  useEffect(() => {
    const held = editor.current;
    if (!held) return;
    if (said === null) {
      held.dispatch(setDiagnostics(held.state, []));
      return;
    }
    // The child counts lines and columns from 1 (ADR 0026 §1, as M4 PR 3 wrote it); a column of
    // 0 means it had no position, and the whole line is marked instead. A line past the end of
    // the buffer is reachable — the author may have deleted some since — so it is clamped rather
    // than throwing inside a view.
    const doc = held.state.doc;
    const line = doc.line(Math.min(Math.max(said.line, 1), doc.lines));
    const at = said.column > 0 ? Math.min(line.from + said.column - 1, line.to) : line.from;
    held.dispatch(
      setDiagnostics(held.state, [
        // **From the column to the end of the line**, never one character. The child gives a
        // point rather than a range, and a point drawn as a point is a wavy underline under one
        // letter: `1:1` on `import math` marked the `i` and nothing else, which in the window is
        // invisible unless you know where to look. Marking the rest of the line says "from
        // here", which is what a position without a length honestly means.
        { from: at, to: line.to, severity: "error", message: said.message },
      ]),
      // Put the cursor where the refusal is, so the line the child named is the line the author
      // is looking at even when the editor is scrolled somewhere else.
      { selection: { anchor: at }, scrollIntoView: true },
    );
  }, [said]);

  return (
    <>
      {/* The view's own title bar, in the pane **body** rather than in the pane head — the
          branchbar's precedent and its reason (`App.tsx`): the head may not wrap and its
          children may not shrink, which is right for a bar a drag is measured against and wrong
          for a row of controls that would be clipped instead. Nothing here is dragged. */}
      <div className="codebar">
        {/* Disabled while there is an unsaved edit, which is the one place a draft could be
            lost without anybody being told. `ponytail:` one draft at a time — a draft per
            generator arrives when somebody wants two open, and the Save and Discard that clear
            this are the two controls beside it. */}
        <select
          value={view.id}
          disabled={dirty}
          onChange={(event) => onOpen(event.target.value)}
          aria-label="generator"
          title={dirty ? WHY_NOT_OPEN : undefined}
        >
          {openable.map((held) => (
            <option key={held.id} value={held.id}>
              {held.id} · {held.kind}
            </option>
          ))}
        </select>
        {/* The target's **kind** and not its id. The id was here and the bar then wrapped at the
            pane's real width — 860 px, not the window's 1200 — which put the status word on the
            button row, away from the seed and the toolchain §9 asks it to sit beside. A clip id
            is also the one fact on this bar nobody can act on from here, so it is the one that
            goes on hover. Measured in the window, 2026-10-01. */}
        <span className="dim" title={view.target.id}>
          → {view.target.kind === "none" ? "nothing" : view.target.kind}
        </span>
        {/* A `uint64` as the string it crosses JSON as, never a double (trap 10). */}
        <span className="dim">seed {view.seed}</span>
        <span className="dim">
          {/* Empty until the first compile writes what the child reported (ADR 0027 §1). */}
          toolchain {view.toolchainVersion === "" ? "—" : view.toolchainVersion}
        </span>
        <span className={`state ${status}`} title={EXPLAINED[status]}>
          {status}
        </span>
        {/* `core`'s own summary, when it answered and there was nothing to apply. Without it a
            Compile on an up-to-date generator changes **nothing on screen** — the word was
            already `compiled`, no diff opens, and the button reads as broken. Driven in the
            window, where it looked exactly like that. A compile that *does* find work says so
            with its diff, so this is the one answer that would otherwise be silent. */}
        {checked === null ? null : <span className="dim">· {checked}</span>}
        {dirty ? (
          <span className="state edited" title={UNSAVED}>
            edited
          </span>
        ) : null}
        {moved ? (
          <span className="state moved" title={MOVED}>
            the document moved
          </span>
        ) : null}
        <span className="buttons">
          <button className="pane" disabled={!dirty} onClick={onSave}>
            Save
          </button>
          <button className="pane" disabled={!dirty} onClick={() => onDraft(null)}>
            Discard
          </button>
          <button
            className="pane"
            disabled={dirty}
            onClick={onCompile}
            title={dirty ? WHY_NOT_COMPILE : COMPILES}
          >
            Compile
          </button>
        </span>
      </div>
      <div className="editor" ref={host} />
    </>
  );
}

/** What each status word means, where a person can hover it. The second is the one that matters:
 *  `compiled` is "it has compiled", not "it is up to date", and only `core` knows the second. */
const EXPLAINED: Record<Status, string> = {
  never:
    "This generator has never compiled — its `compiled_hash` is empty, so no note in this song " +
    "came from it. Press Compile.",
  compiled:
    "This generator has compiled. Whether it is still up to date is a comparison only `core` " +
    "can make: it hashes everything a compile would be handed and compares that with what the " +
    "last compile recorded. Pressing Compile is what turns this word into `compiled` or `stale`.",
  stale:
    "`core` hashed what a compile would be handed now and it differs from what the last compile " +
    "recorded: the source, the seed, the params, the tempo or signature map, the sections or " +
    "the target clip's bounds have moved since. The notes in the song are the old ones until " +
    "the compile below is applied.",
};

const UNSAVED =
  "What is in the editor is not in the document. Save writes it as one `apply_patch` on this " +
  "generator's source, with the diff to approve first, as every other control in this window " +
  "does; Discard throws it away and shows the document again.";

const MOVED =
  "The document's source changed while this edit was open — a model's patch, an undo, or a " +
  "branch switch. Your text is kept rather than overwritten: Save replaces theirs with yours, " +
  "Discard takes theirs.";

const WHY_NOT_COMPILE =
  "A compile reads the document, never this buffer: `compile_generator` is given a generator's " +
  "id and `core` compiles the source the song holds. Save first, or Discard.";

const WHY_NOT_OPEN = "Save or discard this edit first — it is the only copy of it.";

const COMPILES =
  "Compiles this generator in a fresh sandbox process and replaces the target clip's notes " +
  "whole — as a dry run first, so the patch is on screen before anything is written. A " +
  "generator that is already up to date answers without starting a process at all.";
