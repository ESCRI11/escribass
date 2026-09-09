// The window: one decoded `Song`, and views that are pure selectors over it (ADR 0012 §2).
//
// There is no store here, and there is not going to be one. The song is read once from
// `get_song` and held whole; when an edit lands the answer is another `get_song` and a
// wholesale replacement, never a local RFC 6902 apply and never a normalised cache. §14.2
// forbids the second representation, ADR 0016 §3 keeps every library that would quietly
// introduce one out of `package.json`, and ADR 0012 §5's projection golden is what would catch
// a hand-written one.
//
// **The edit path (M2 PR 5), in one paragraph.** A drag proposes a position; every position it
// passes through is sent as a `set_notes` **dry run**, which validates and writes nothing; the
// answer is the RFC 6902 patch a commit would record, and it is drawn as a diff a person
// approves (§9). Applying is the same call without `dry_run`, and then `get_song` again. So a
// gesture is one entry in the log however many pointer events it fired, and every position it
// passed through has been past the validator rather than around it. That is ADR 0017, and it is
// written out here because without it the two `set_notes` calls below look like one call made
// twice.
//
// Nothing below is song state. `moved` is three numbers describing a gesture, `preview` is an
// answer the tool API gave, and `chosen` is a clip id the user picked.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fromJson, toJson } from "@bufbuild/protobuf";
import type { JsonValue } from "@bufbuild/protobuf";
import { NoteSchema, SongSchema, type Song } from "@escribass/schema/song";
import { arrangement } from "./arrangement.js";
import type { Arrangement } from "./arrangement.js";
import { pianoRoll } from "./pianoroll.js";
import { HEAD, ROW, Roll, Timeline } from "./canvas.js";
import type { Moved } from "./canvas.js";
import { tool } from "./tool.js";

/**
 * Freezes the decoded song, so a view that writes to the model throws where it wrote.
 *
 * Development builds only, which is ADR 0012 §2's wording and the right trade: the cost is a
 * walk of the whole document, and the failure it catches — a view mutating the model and
 * drifting until someone notices a value the document does not have — is one a developer
 * meets long before a user does.
 */
function freeze<T>(value: T): T {
  if (!import.meta.env.DEV) return value;
  if (value === null || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const held of Object.values(value)) freeze(held);
  return Object.freeze(value);
}

/** Seconds as `m:ss`. The only number here the tempo map decides (see `time.ts`). */
function clock(total: number): string {
  const whole = Math.round(total);
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, "0")}`;
}

/**
 * What a mutating tool answers with, as `ToolResult` crosses a JSON carrier (ADR 0006 §1, §6).
 *
 * Hand-written, and allowed to be: this is the *tool API's* result, not song state, and §4.1's
 * rule is about model types. `proto/gen/ts` has a generated `ToolResult` and it is the wrong
 * shape here — `patch` is `bytes` there and base64 on the wire, while `core/src/call.rs`
 * deliberately sends the RFC 6902 array instead, which is the one exception ADR 0006 §6 pins.
 * Decoding through the generated type would ask for the encoding this boundary exists to avoid.
 */
interface ToolAnswer {
  readonly valid: boolean;
  readonly errors: readonly { readonly path: string; readonly rule: string; readonly message: string }[];
  readonly patch: readonly unknown[] | null;
  readonly summary: string;
  readonly entry_id: string;
}

/** The arguments a `set_notes` needs to move one note, built from the model rather than from
 *  the projection.
 *
 * `set_notes` replaces the clip's **whole** note set, so every note has to be sent back — and
 * sent back whole. A request built from the roll's projection would carry the five fields a
 * roll draws and silently drop `expression`, which is a map the roll shows nothing of yet
 * (`pianoroll.ts`). That is why this reaches past the view to the decoded `Song` and encodes
 * each note with the generated encoder.
 *
 * `id`, `provenance` and `version` are stripped because they are core's, not arguments: the
 * tool's own schema omits them for an embedded entity (ADR 0006 §4), and `set_notes` keeps
 * them for a note whose *key* it already holds (`core/src/tools.rs`).
 */
function movedNotes(song: Song, clipId: string, moved: Moved): Record<string, JsonValue> {
  const clip = song.clips[clipId];
  if (clip?.content.case !== "noteClip") return {};
  return Object.fromEntries(
    Object.entries(clip.content.value.notes).map(([id, note]) => {
      const at =
        id === moved.noteId ? { ...note, startTick: moved.startTick, pitch: moved.pitch } : note;
      const { id: _id, provenance: _provenance, version: _version, ...rest } = toJson(
        NoteSchema,
        at,
      ) as Record<string, JsonValue>;
      return [id, rest as JsonValue];
    }),
  );
}

export function App() {
  const [song, setSong] = useState<Song | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [chosen, setChosen] = useState<string | null>(null);
  /** The gesture in flight, or the position it left behind for approval. */
  const [moved, setMoved] = useState<Moved | null>(null);
  /** The tool API's answer about that position: the patch to approve, or why it is refused. */
  const [preview, setPreview] = useState<ToolAnswer | null>(null);
  /** The last thing a tool refused outright — `nothing_to_undo`, and its neighbours. */
  const [notice, setNotice] = useState<string | null>(null);
  /** Whether the pointer is still down. It gates the diff pane, and the reason is layout, not
   *  taste: the pane takes space, taking space moves the roll, and moving the roll moves the
   *  note out from under the cursor mid-drag (`canvas.tsx`, `onReleased`). */
  const [held, setHeld] = useState(false);

  const read = useCallback(
    () =>
      tool("get_song", {})
        .then((answer) => setSong(freeze(fromJson(SongSchema, answer as JsonValue))))
        .catch((e: unknown) => setFailure(String(e))),
    [],
  );

  useEffect(() => {
    void read();
  }, [read]);

  const view = useMemo(() => (song ? arrangement(song) : null), [song]);

  // The clips a roll can be opened on, in the arrangement's own order — derived from the
  // projection rather than listed a second time, so the `<select>` and the timeline cannot
  // disagree about what a clip is called or where it sits.
  const rollable = useMemo(
    () =>
      (view?.tracks ?? []).flatMap((track) =>
        track.clips
          .filter((clip) => clip.kind === "note")
          .map((clip) => ({ id: clip.id, label: `${track.name} · ${clip.label}` })),
      ),
    [view],
  );

  // Derived, never stored: a chosen id that the song no longer has falls back to the first
  // clip rather than leaving the roll pointed at something that is gone. Holding the fallback
  // in state instead would be a second copy of a fact the document already carries.
  const openId = chosen !== null && rollable.some((c) => c.id === chosen) ? chosen : rollable[0]?.id;
  const roll = useMemo(
    () => (song && openId !== undefined ? pianoRoll(song, openId) : null),
    [song, openId],
  );

  // One dry run in flight at a time, and the newest position wins. A drag fires pointer events
  // faster than a round trip returns, and queueing them all would leave the diff a hundred
  // answers behind the pointer; dropping the ones overtaken costs nothing, because a preview
  // describes a position and only the newest position is on screen (ADR 0017 §2).
  const flight = useRef<{ busy: boolean; queued: Moved | null }>({ busy: false, queued: null });
  const clipId = roll?.clipId;

  async function previewAt(at: Moved, clip: string, from: Song): Promise<void> {
    flight.current.busy = true;
    try {
      setPreview(
        (await tool("set_notes", {
          clip_id: clip,
          notes: movedNotes(from, clip, at),
          dry_run: true,
        })) as ToolAnswer,
      );
    } catch (e: unknown) {
      setFailure(String(e));
    } finally {
      flight.current.busy = false;
      const next = flight.current.queued;
      flight.current.queued = null;
      if (next) void previewAt(next, clip, from);
    }
  }

  /** Every position the gesture passes through, asked of the tool API and of nothing else.
   *
   *  There is no `pitch < 0 || pitch > 127` in this file, and there is not going to be one:
   *  §4.4 is the validator's, and a copy of it in TypeScript is the second implementation
   *  ADR 0012 §2 refuses for RFC 6902 apply, arriving as a convenience (ADR 0017 §3). */
  function propose(next: Moved | null): void {
    setMoved(next);
    setNotice(null);
    if (next !== null) setHeld(true);
    if (next === null || !song || clipId === undefined) {
      setPreview(null);
      return;
    }
    if (flight.current.busy) {
      flight.current.queued = next;
      return;
    }
    void previewAt(next, clipId, song);
  }

  /** §9's second half: the same call without `dry_run`, then a fresh `get_song`.
   *
   *  Applied optimistically rather than re-previewed first — the model can have moved under a
   *  held preview, and §4.3's `version` check is what refuses if it did (ADR 0012 §4). */
  async function applyMove(): Promise<void> {
    if (!moved || !song || clipId === undefined) return;
    try {
      const answer = (await tool("set_notes", {
        clip_id: clipId,
        notes: movedNotes(song, clipId, moved),
        dry_run: false,
      })) as ToolAnswer;
      if (!answer.valid) {
        setPreview(answer);
        return;
      }
      setMoved(null);
      setPreview(null);
      await read();
    } catch (e: unknown) {
      setFailure(String(e));
    }
  }

  /** ⌘Z and ⇧⌘Z, which are tool calls and nothing else (docs/plan.md, M2 trap 10).
   *
   *  There is no stack in this file. `undo` appends an inverse entry through the same pipeline
   *  every other tool goes through, and the session holds how far back it has walked
   *  (ADR 0005 §4) — so a second window, an agent's edit or a branch switch cannot leave the
   *  key disagreeing with the log, which is what a frontend stack would do the moment any of
   *  those happened. Not previewed either: an undo reverses a change that was already
   *  approved, and asking for approval to withdraw approval is a dialog with nothing in it. */
  async function press(name: "undo" | "redo"): Promise<void> {
    try {
      const answer = (await tool(name, {})) as ToolAnswer;
      if (!answer.valid) {
        setNotice(`${name}: ${answer.errors.map((e) => e.message).join("; ")}`);
        return;
      }
      setNotice(null);
      setMoved(null);
      setPreview(null);
      await read();
    } catch (e: unknown) {
      setFailure(String(e));
    }
  }

  // No dependency array, as `useCanvas` has none and for the same reason: the handler closes
  // over this render's state, and rebinding one listener is cheaper than reasoning about
  // which of them it needs. `ponytail:` two keys, matched here — a shortcut *system* is what
  // arrives when there are twenty, and there are two.
  useEffect(() => {
    const pressed = (event: KeyboardEvent) => {
      if (event.key.toLowerCase() !== "z" || !(event.metaKey || event.ctrlKey)) return;
      event.preventDefault();
      void press(event.shiftKey ? "redo" : "undo");
    };
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  });

  // A gesture that has not moved the note yet has nothing to approve: the dry run comes back
  // valid with no operations, because a call that changes nothing records nothing
  // (`core/src/session.rs`). A pane offering to apply that would be offering to apply nothing.
  const proposal = preview && (!preview.valid || (preview.patch?.length ?? 0) > 0) ? preview : null;

  // What the pane head says: a refusal while the gesture is still running, so the boundary is
  // named where it is crossed without the diff pane opening under the pointer (ADR 0017 §3),
  // and otherwise whatever a tool last refused outright.
  const said =
    preview && !preview.valid
      ? preview.errors.map((violation) => `${violation.rule}: ${violation.message}`).join("; ")
      : notice;

  if (failure !== null) {
    return (
      <main className="failure">
        <h1>The project could not be read</h1>
        <pre>{failure}</pre>
      </main>
    );
  }
  if (!song || !view) return <main className="waiting">Reading the project…</main>;

  const clips = view.tracks.reduce((total, track) => total + track.clips.length, 0);
  const opening = view.tempo[0];
  const signature = view.signatures[0];

  return (
    <main>
      <header>
        <span className="tempo">{opening.bpm.toFixed(2)} BPM</span>
        <span className="signature">
          {signature.numerator}/{signature.denominator}
        </span>
        <span className="dim">
          {view.bars.length} bar{view.bars.length === 1 ? "" : "s"} · {clock(view.seconds)}
        </span>
      </header>

      <div className="views">
        <section className="arrangement">
          <div className="gutter">
            <div className="gutter-head" style={{ height: HEAD }} />
            {view.tracks.map((track) => (
              <div className="track" key={track.id} style={{ height: ROW }}>
                <span className="name">{track.name}</span>
                <span className="kind">{track.kind}</span>
              </div>
            ))}
          </div>
          <Timeline view={view} selected={roll?.clipId ?? null} />
        </section>

        <section className="roll-pane">
          <div className="pane-head">
            <span className="dim">piano roll</span>
            {rollable.length === 0 ? (
              <span className="dim">no note clips</span>
            ) : (
              // A native `<select>`, which is the whole of "choose a clip" and needs no
              // library (ADR 0016 §3). Choosing one is not an edit: nothing is written, and
              // the roll it opens is `pianoRoll(song, id)` recomputed from the same song.
              <select
                value={openId ?? ""}
                onChange={(event) => setChosen(event.target.value)}
                aria-label="clip"
              >
                {rollable.map((clip) => (
                  <option key={clip.id} value={clip.id}>
                    {clip.label}
                  </option>
                ))}
              </select>
            )}
            {roll ? (
              <span className="dim">
                bar {barOf(view, roll.startTick)} · {roll.notes.length} note
                {roll.notes.length === 1 ? "" : "s"}
              </span>
            ) : null}
            {said !== null ? <span className="notice">{said}</span> : null}
          </div>
          {roll ? (
            <Roll
              view={roll}
              moved={moved}
              refused={preview?.valid === false}
              onMoved={propose}
              onReleased={() => setHeld(false)}
            />
          ) : (
            <p className="empty">Nothing to show here yet.</p>
          )}
          {!held && moved && proposal ? (
            <Pending answer={proposal} onApply={applyMove} onDiscard={() => propose(null)} />
          ) : null}
        </section>
      </div>

      {/*
        The status bar shows nothing §11 claims. Plate 1 of the wireframes puts the
        `song.json` hash, the patch count, the last render hash and `lock.json 14/14
        verified` here, and every one of those is a determinism claim: a widget that
        computes one a second way is a second implementation of the thing the suite exists
        to check, and the drift is invisible until a demo. They arrive when `core` reports
        them (docs/plan.md, M2 trap 4).
      */}
      <footer>
        <span>
          {view.tracks.length} track{view.tracks.length === 1 ? "" : "s"}
        </span>
        <span>
          {clips} clip{clips === 1 ? "" : "s"}
        </span>
        <span>
          {view.sections.length} section{view.sections.length === 1 ? "" : "s"}
        </span>
        <span className="dim">⌘Z undo · ⇧⌘Z redo</span>
        <span className="mode">drag a note</span>
      </footer>
    </main>
  );
}

/**
 * The diff, before it is applied — §9's "always shows the diff before applying", for the one
 * control that is a tool call.
 *
 * What it shows is `ToolResult.patch`: the **re-derived** RFC 6902 operations a commit would
 * record, version bumps included (ADR 0006 §1). Not a description of them, and not a diff this
 * file computed — the patch a person approves here is byte for byte the patch that lands,
 * which is what makes approving it mean anything (ADR 0006 §3).
 *
 * A refused position shows its rules instead and offers no Apply. `rule` is the stable id §5
 * promises a caller can act on, and it is shown beside the sentence rather than hidden behind
 * it, because it is the half that does not change with wording.
 */
function Pending({
  answer,
  onApply,
  onDiscard,
}: {
  answer: ToolAnswer;
  onApply: () => void;
  onDiscard: () => void;
}) {
  return (
    <div className={answer.valid ? "pending" : "pending refused"}>
      <div className="pending-head">
        <strong>{answer.valid ? "unapplied edit" : "refused"}</strong>
        <span className="dim">{answer.summary}</span>
        <span className="buttons">
          {answer.valid ? <button onClick={onApply}>Apply</button> : null}
          <button onClick={onDiscard}>Discard</button>
        </span>
      </div>
      {answer.valid ? (
        <pre>{JSON.stringify(answer.patch, null, 1)}</pre>
      ) : (
        <ul>
          {answer.errors.map((violation) => (
            <li key={`${violation.path}/${violation.rule}`}>
              <code>{violation.rule}</code> {violation.message}{" "}
              <span className="dim">{violation.path}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/** Which bar a tick falls in, read off the grid the arrangement already computed rather than
 *  divided out again — the time signature can change, so there is no bar width to divide by. */
function barOf(view: Arrangement, tick: number): number {
  let number = 1;
  for (const bar of view.bars) {
    if (bar.startTick > tick) break;
    number = bar.number;
  }
  return number;
}
