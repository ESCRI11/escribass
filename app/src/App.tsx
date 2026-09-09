// The window: one decoded `Song`, and views that are pure selectors over it (ADR 0012 §2).
//
// There is no store here, and there is not going to be one. The song is read once from
// `get_song` and held whole; when an edit lands the answer is another `get_song` and a
// wholesale replacement, never a local RFC 6902 apply and never a normalised cache. §14.2
// forbids the second representation, ADR 0016 §3 keeps every library that would quietly
// introduce one out of `package.json`, and ADR 0012 §5's projection golden is what would catch
// a hand-written one.
//
// **The edit path (M2 PR 5), in one paragraph.** A gesture proposes a value; every value it
// passes through is sent as a **dry run**, which validates and writes nothing; the answer is
// the RFC 6902 patch a commit would record, and it is drawn as a diff a person approves (§9).
// Applying is the same call without `dry_run`, and then `get_song` again. So a gesture is one
// entry in the log however many pointer events it fired, and every value it passed through has
// been past the validator rather than around it. That is ADR 0017, and it is written out here
// because without it the two calls below look like one call made twice.
//
// **The gesture is the call (M2 PR 7).** PR 5 had one gesture and could name it; PR 7 has four
// — a note drag, a fader ride, a pan, a mute — over two tools, so what is held is the *call*
// rather than the note: `Gesture` is a tool name, its arguments, and what the view should draw
// while the pointer is down. Nothing else changed about the flow, which is the point. A
// gesture that could not be expressed as repeated dry runs of one tool call would need its own
// ADR (ADR 0017, Consequences), and none of the three PR 7 adds is one.
//
// **The log is a view too (M2 PR 8).** `get_history` is read beside `get_song` and held
// decoded, and `history.ts` projects it exactly as `mixer.ts` projects the song. It is not a
// second representation of song state: nothing here derives a document from the log, which is
// `core`'s job and `get_song`'s answer. Branch switching goes through it, and it is where a
// merge conflict is settled — per path, on the *same* `merge_branch` call, which is the whole
// of ADR 0015 §3.
//
// Nothing below is song state. `Gesture` is a call waiting to be made, `preview` is an answer
// the tool API gave, and `chosen`, `device` and `branch` are names the user picked.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fromJson, toJson } from "@bufbuild/protobuf";
import type { JsonValue } from "@bufbuild/protobuf";
import { NoteSchema, SongSchema, type Song } from "@escribass/schema/song";
import { arrangement } from "./arrangement.js";
import type { Arrangement } from "./arrangement.js";
import { pianoRoll } from "./pianoroll.js";
import { mixer } from "./mixer.js";
import { devices, editor } from "./params.js";
import type { Manifest } from "./params.js";
import { Params, Strips } from "./form.js";
import type { Touch, Write } from "./form.js";
import { HEAD, ROW, Roll, Timeline } from "./canvas.js";
import type { Moved } from "./canvas.js";
import { patchLog } from "./history.js";
import type { HistoryAnswer, Log } from "./history.js";
import { build, tool } from "./tool.js";

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

/**
 * A gesture in progress, or the proposal it left behind for approval — as the tool call it is.
 *
 * `tool` and `args` are one call made twice: `dry_run: true` while it is being proposed and
 * `dry_run: false` when a person applies it (ADR 0017 §2, §4). The two payloads beside them
 * are what a view draws while the gesture is live, and neither is song state — `moved` is
 * three numbers describing a drag (`canvas.tsx`) and `touched` is a control and the number
 * under the pointer (`form.tsx`). The document does not change until it changes.
 */
interface Gesture {
  readonly tool: string;
  readonly args: Record<string, JsonValue>;
  readonly moved?: Moved;
  readonly touched?: Touch;
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

/** Which view the lower pane is showing. Four panes and a `<select>`-free switch, because
 *  four buttons are four buttons; a router arrives when there is something to route. */
type Pane = "roll" | "mixer" | "params" | "history";

export function App() {
  const [song, setSong] = useState<Song | null>(null);
  /** The patch log, as `get_history` answered it — §5's audit trail, read beside the song and
   *  held decoded so the view over it is a pure selector like every other (ADR 0012 §2). */
  const [history, setHistory] = useState<HistoryAnswer | null>(null);
  /** What this build can host — the map the parameter editor is a form over (ADR 0014 §1).
   *  Read once: it describes the running process, not the project, and cannot change under it. */
  const [manifest, setManifest] = useState<Manifest | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [pane, setPane] = useState<Pane>("roll");
  const [chosen, setChosen] = useState<string | null>(null);
  const [device, setDevice] = useState<string | null>(null);
  /** Which branch the branch buttons act on. A choice and not an edit: nothing is written
   *  until a button is pressed, exactly as choosing a clip writes nothing. */
  const [branch, setBranch] = useState<string | null>(null);
  /** What is typed in the new-branch box. Not a branch until `create_branch` says so. */
  const [naming, setNaming] = useState("");
  /** What is typed in the editor's search box. Not a projection and not an edit: 2855 rows is
   *  a list nobody can read, and narrowing it writes nothing. */
  const [filter, setFilter] = useState("");
  /** The gesture in flight, or the one waiting for approval. */
  const [gesture, setGesture] = useState<Gesture | null>(null);
  /** The tool API's answer about it: the patch to approve, or why it is refused. */
  const [preview, setPreview] = useState<ToolAnswer | null>(null);
  /** The last thing a tool refused outright — `nothing_to_undo`, and its neighbours. */
  const [notice, setNotice] = useState<string | null>(null);
  /** Whether the pointer is still down. It gates the diff pane, and the reason is layout, not
   *  taste: the pane takes space, taking space moves what is under the pointer — the roll's
   *  note, or a fader's thumb — and a gesture is measured against where the control is
   *  (ADR 0017 §3, §5). */
  const [held, setHeld] = useState(false);

  // Both reads, together. The log is re-read whenever the song is because every applied call
  // appends to it — including `undo`, which appends an inverse entry rather than rewinding
  // (ADR 0005 §4), so a history view refreshed only on an edit would be wrong exactly where a
  // person is looking to check what undo did.
  const read = useCallback(
    () =>
      Promise.all([tool("get_song", {}), tool("get_history", {})])
        .then(([held, log]) => {
          setSong(freeze(fromJson(SongSchema, held as JsonValue)));
          setHistory(freeze(log as HistoryAnswer));
        })
        .catch((e: unknown) => setFailure(String(e))),
    [],
  );

  useEffect(() => {
    void read();
    void build()
      .then((answer) => setManifest(freeze(answer as Manifest)))
      .catch((e: unknown) => setFailure(String(e)));
  }, [read]);

  const view = useMemo(() => (song ? arrangement(song) : null), [song]);
  const strips = useMemo(() => (song ? mixer(song) : null), [song]);
  const log = useMemo(() => (history ? patchLog(history) : null), [history]);

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

  // Every device an editor can be opened on, in the mixer's own order and for the same reason.
  const editable = useMemo(() => (song ? devices(song) : []), [song]);

  // Derived, never stored: a chosen id that the song no longer has falls back to the first
  // clip rather than leaving the roll pointed at something that is gone. Holding the fallback
  // in state instead would be a second copy of a fact the document already carries.
  const openId = chosen !== null && rollable.some((c) => c.id === chosen) ? chosen : rollable[0]?.id;
  const roll = useMemo(
    () => (song && openId !== undefined ? pianoRoll(song, openId) : null),
    [song, openId],
  );

  const deviceId =
    device !== null && editable.some((d) => d.id === device) ? device : editable[0]?.id;
  const editing = useMemo(
    () => (song && manifest && deviceId !== undefined ? editor(song, manifest, deviceId) : null),
    [song, manifest, deviceId],
  );

  // Derived, never stored, for the reason `openId` is: a branch the log no longer has falls
  // back rather than leaving the buttons pointed at a name that is gone. The default is the
  // first branch that is **not** `HEAD`, since switching to or merging the branch you are on
  // is the one thing neither button can usefully do.
  const branches = log?.branches ?? [];
  const picked =
    branch !== null && branches.some((held) => held.name === branch)
      ? branch
      : (branches.find((held) => !held.head)?.name ?? log?.head);

  // One dry run in flight at a time, and the newest value wins. A drag fires pointer events
  // faster than a round trip returns, and queueing them all would leave the diff a hundred
  // answers behind the pointer; dropping the ones overtaken costs nothing, because a preview
  // describes a value and only the newest value is on screen (ADR 0017 §2).
  const flight = useRef<{ busy: boolean; queued: Gesture | null }>({ busy: false, queued: null });

  async function previewAt(next: Gesture): Promise<void> {
    flight.current.busy = true;
    try {
      setPreview((await tool(next.tool, { ...next.args, dry_run: true })) as ToolAnswer);
    } catch (e: unknown) {
      setFailure(String(e));
    } finally {
      flight.current.busy = false;
      const queued = flight.current.queued;
      flight.current.queued = null;
      if (queued) void previewAt(queued);
    }
  }

  /** Every value the gesture passes through, asked of the tool API and of nothing else.
   *
   *  There is no `pitch < 0 || pitch > 127` in this file, no `0 <= value <= 1`, and there is
   *  not going to be one: §4.4 is the validator's, and a copy of it in TypeScript is the
   *  second implementation ADR 0012 §2 refuses for RFC 6902 apply, arriving as a convenience
   *  (ADR 0017 §3). A fader whose travel the model does not share — `gain_db` is unbounded —
   *  makes that concrete: the control's ends are a drawing decision and the refusal, when
   *  there is one, still comes from `core`. */
  function propose(next: Gesture | null): void {
    setGesture(next);
    setNotice(null);
    if (next === null) {
      setPreview(null);
      return;
    }
    if (flight.current.busy) {
      flight.current.queued = next;
      return;
    }
    void previewAt(next);
  }

  /** §9's second half: the same call without `dry_run`, then a fresh `get_song`.
   *
   *  Applied optimistically rather than re-previewed first — the model can have moved under a
   *  held preview, and §4.3's `version` check is what refuses if it did (ADR 0012 §4). */
  async function apply(): Promise<void> {
    if (!gesture) return;
    try {
      const answer = (await tool(gesture.tool, {
        ...gesture.args,
        dry_run: false,
      })) as ToolAnswer;
      if (!answer.valid) {
        setPreview(answer);
        return;
      }
      setGesture(null);
      setPreview(null);
      await read();
    } catch (e: unknown) {
      setFailure(String(e));
    }
  }

  /** One conflicting path, settled — ADR 0015 §3's whole mechanism, and it is one line of
   *  state on a call that already exists.
   *
   *  The picks ride on the **same** `merge_branch` call: choosing a side re-proposes it as
   *  another dry run, and `Apply` is that same call once more without `dry_run`. So there is
   *  no `resolve_conflict` tool, nothing held between the two calls, and no merge that is
   *  half committed — the second call either records one entry with two parents or refuses,
   *  exactly as the first one did. A path this merge is not in conflict about comes back as
   *  `resolution_unknown` rather than quietly dropping the other side's change. */
  function resolveAt(path: string, side: string): void {
    if (!gesture) return;
    const picks = { ...(gesture.args.resolve as Record<string, string> | undefined), [path]: side };
    propose({ ...gesture, args: { ...gesture.args, resolve: picks } });
  }

  /** The calls that are applied straight away: ⌘Z, ⇧⌘Z and a branch switch.
   *
   *  ⌘Z is a tool call and nothing else (docs/plan.md, M2 trap 10). There is no stack in this
   *  file: `undo` appends an inverse entry through the same pipeline every other tool goes
   *  through, and the session holds how far back it has walked (ADR 0005 §4) — so a second
   *  window, an agent's edit or a branch switch cannot leave the key disagreeing with the log,
   *  which is what a frontend stack would do the moment any of those happened. Not previewed
   *  either: an undo reverses a change that was already approved, and asking for approval to
   *  withdraw approval is a dialog with nothing in it.
   *
   *  `switch_branch` joins them for the same reason one step further on. It **appends no
   *  entry** (ADR 0001 §2) — history that recorded navigation would grow every time somebody
   *  looked at a branch — so there is nothing for a person to approve into the audit trail,
   *  and switching back is the whole of undoing it. It does replace the document under every
   *  open view, and the replacement is another `get_song` and a wholesale swap: a
   *  re-projection, never a patch applied in this file (ADR 0012 §2). A roll open on a clip
   *  the new branch does not have, or an editor open on a device it does not have, falls back
   *  by itself, because `openId` and `deviceId` are derived from the document rather than
   *  stored beside it. */
  async function direct(name: string, args: Record<string, unknown> = {}): Promise<void> {
    try {
      const answer = (await tool(name, args)) as ToolAnswer;
      if (!answer.valid) {
        setNotice(`${name}: ${answer.errors.map((e) => e.message).join("; ")}`);
        return;
      }
      setNotice(null);
      setGesture(null);
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
      void direct(event.shiftKey ? "redo" : "undo");
    };
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  });

  // A gesture that has not moved anything yet has nothing to approve: the dry run comes back
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
  if (!song || !view || !strips) return <main className="waiting">Reading the project…</main>;

  const clips = view.tracks.reduce((total, track) => total + track.clips.length, 0);
  const opening = view.tempo[0];
  const signature = view.signatures[0];
  const written = (write: Write) =>
    propose({ tool: write.tool, args: write.args, touched: write.touched });

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

        <section className="detail">
          <div className="pane-head">
            {/* Three buttons, not a router and not tabs from a library: what a tab strip is,
                for three panes, is three buttons and a piece of state (ADR 0016 §3). */}
            <span className="panes">
              {(["roll", "mixer", "params", "history"] as const).map((name) => (
                <button
                  key={name}
                  className={pane === name ? "pane on" : "pane"}
                  onClick={() => setPane(name)}
                  aria-pressed={pane === name}
                >
                  {name === "params" ? "parameters" : name === "roll" ? "piano roll" : name}
                </button>
              ))}
            </span>

            {pane === "roll" && rollable.length > 0 ? (
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
            ) : null}
            {pane === "roll" && roll ? (
              <span className="dim">
                bar {barOf(view, roll.startTick)} · {roll.notes.length} note
                {roll.notes.length === 1 ? "" : "s"}
              </span>
            ) : null}
            {pane === "roll" && rollable.length === 0 ? (
              <span className="dim">no note clips</span>
            ) : null}

            {pane === "params" && editable.length > 0 ? (
              <select
                value={deviceId ?? ""}
                onChange={(event) => setDevice(event.target.value)}
                aria-label="device"
              >
                {editable.map((held) => (
                  <option key={held.id} value={held.id}>
                    {held.track} · {held.label}
                    {held.index === undefined ? "" : ` #${held.index}`}
                  </option>
                ))}
              </select>
            ) : null}
            {pane === "params" && editing ? (
              <>
                <input
                  type="search"
                  value={filter}
                  onChange={(event) => setFilter(event.target.value)}
                  placeholder="name or id"
                  aria-label="filter parameters"
                />
                {/* Two counts because they are two maps: what the plugin declares, and what
                    this document overrides. Opening the editor changed neither. */}
                <span className="dim">
                  {editing.declares} declared · {editing.set} set
                </span>
              </>
            ) : null}

            {pane === "history" && log ? (
              <span className="dim">
                on {log.head} · {log.entries.length} entr{log.entries.length === 1 ? "y" : "ies"}
              </span>
            ) : null}

            {said !== null ? <span className="notice">{said}</span> : null}
          </div>

          {pane === "roll" ? (
            roll ? (
              <Roll
                view={roll}
                moved={gesture?.moved ?? null}
                refused={preview?.valid === false}
                onMoved={(next) => {
                  if (next) setHeld(true);
                  propose(
                    next && openId !== undefined
                      ? {
                          tool: "set_notes",
                          args: { clip_id: openId, notes: movedNotes(song, openId, next) },
                          moved: next,
                        }
                      : null,
                  );
                }}
                onReleased={() => setHeld(false)}
              />
            ) : (
              <p className="empty">Nothing to show here yet.</p>
            )
          ) : pane === "history" ? (
            log ? (
              <>
                {/* The branch controls live in the pane *body*, not the head. The head may not
                    wrap and its children may not shrink (ADR 0017 §3, as PR 7 corrected it),
                    which is right for a bar a drag is measured against and wrong for six
                    controls that would then be clipped instead. Nothing in this pane is
                    dragged, so the bar is free to wrap here. */}
                <div className="branchbar">
                  <select
                    value={picked ?? ""}
                    onChange={(event) => setBranch(event.target.value)}
                    aria-label="branch"
                  >
                    {log.branches.map((held) => (
                      <option key={held.name} value={held.name}>
                        {held.name}
                        {held.head ? " (here)" : ""}
                      </option>
                    ))}
                  </select>
                  {/* All three are disabled on `HEAD`, which is the one branch none of them
                      can act on: switching to where you are answers "already here", merging a
                      branch into itself the same, and deleting `HEAD` is `delete_head`
                      (`core/src/history.rs`). Disabled rather than hidden, so the bar keeps
                      its shape as the choice changes. */}
                  <button
                    className="pane"
                    disabled={picked === undefined || picked === log.head}
                    onClick={() => picked && void direct("switch_branch", { name: picked })}
                  >
                    Switch
                  </button>
                  <button
                    className="pane"
                    disabled={picked === undefined || picked === log.head}
                    onClick={() =>
                      picked && propose({ tool: "merge_branch", args: { name: picked } })
                    }
                  >
                    Merge into {log.head}
                  </button>
                  <button
                    className="pane"
                    disabled={picked === undefined || picked === log.head}
                    onClick={() => picked && void direct("delete_branch", { name: picked })}
                  >
                    Delete
                  </button>
                  {/* A new ref at the current entry, which copies no data (ADR 0001 §2). The
                      name is the user's and its rules are the validator's — ASCII
                      `[a-z0-9._/-]`, no `..` — so there is no pattern checked here, for
                      ADR 0017 §3's reason one form over: a rule predicted in TypeScript is a
                      second implementation of one that already exists. */}
                  <input
                    value={naming}
                    onChange={(event) => setNaming(event.target.value)}
                    placeholder="new branch"
                    aria-label="new branch name"
                  />
                  <button
                    className="pane"
                    disabled={naming.trim() === ""}
                    onClick={() => {
                      const name = naming.trim();
                      setNaming("");
                      setBranch(name);
                      void direct("create_branch", { name });
                    }}
                  >
                    Branch
                  </button>
                </div>
                <PatchLog view={log} />
              </>
            ) : (
              <p className="empty">Reading the log…</p>
            )
          ) : pane === "mixer" ? (
            <Strips
              view={strips}
              touched={gesture?.touched}
              onWrite={written}
              onHeld={setHeld}
              onOpen={(id) => {
                setDevice(id);
                setPane("params");
              }}
              opened={deviceId}
            />
          ) : editing ? (
            <Params
              view={editing}
              touched={gesture?.touched}
              filter={filter}
              onWrite={written}
              onHeld={setHeld}
            />
          ) : (
            <p className="empty">
              {manifest === null ? "Reading the build manifest…" : "This song has no devices."}
            </p>
          )}

          {!held && gesture && proposal ? (
            <Pending
              answer={proposal}
              onApply={apply}
              onDiscard={() => propose(null)}
              // Only a merge can be resolved, so only a merge is handed the form. A conflict
              // is a `Violation` like any other refusal; what makes it settleable is that the
              // call it came from takes a `resolve` map (ADR 0015 §3).
              resolution={
                gesture.tool === "merge_branch"
                  ? {
                      theirs: String(gesture.args.name),
                      picks: (gesture.args.resolve as Record<string, string> | undefined) ?? {},
                      onPick: resolveAt,
                    }
                  : undefined
              }
            />
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
        <span className="mode">
          {pane === "roll"
            ? "drag a note"
            : pane === "mixer"
              ? "ride a fader"
              : pane === "params"
                ? "set a parameter"
                : "switch or merge a branch"}
        </span>
      </footer>
    </main>
  );
}

/**
 * The diff, before it is applied — §9's "always shows the diff before applying", for every
 * control that is a tool call.
 *
 * What it shows is `ToolResult.patch`: the **re-derived** RFC 6902 operations a commit would
 * record, version bumps included (ADR 0006 §1). Not a description of them, and not a diff this
 * file computed — the patch a person approves here is byte for byte the patch that lands,
 * which is what makes approving it mean anything (ADR 0006 §3).
 *
 * A refused value shows its rules instead and offers no Apply. `rule` is the stable id §5
 * promises a caller can act on, and it is shown beside the sentence rather than hidden behind
 * it, because it is the half that does not change with wording.
 *
 * **A merge conflict is the one refusal that can be answered here** (ADR 0015 §3). It is not a
 * different pane and not a different flow: the same list of violations, with two buttons on
 * the rows whose `rule` is `merge_conflict`, and the answer goes back on the call that
 * produced them. ADR 0017 has nothing to add — it is about a *gesture*, and what makes a
 * gesture special is that the layout may not move under a pointer that is down. Nothing here
 * is dragged: the picks are buttons, the pane is already open, and a reflow between two clicks
 * is a reflow nobody is measuring a distance against.
 */
function Pending({
  answer,
  onApply,
  onDiscard,
  resolution,
}: {
  answer: ToolAnswer;
  onApply: () => void;
  onDiscard: () => void;
  /** Present only for a merge: the other branch's name, the picks so far, and where a new one
   *  goes. `undefined` everywhere else, which is what keeps a fader's refusal from growing
   *  buttons that would call a tool it is not. */
  resolution?: { theirs: string; picks: Record<string, string>; onPick: (path: string, side: string) => void };
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
              {resolution && violation.rule === "merge_conflict" ? (
                <span className="sides">
                  {(
                    [
                      ["MERGE_SIDE_OURS", "keep mine"],
                      ["MERGE_SIDE_THEIRS", `take ${resolution.theirs}`],
                    ] as const
                  ).map(([side, label]) => (
                    <button
                      key={side}
                      className={resolution.picks[violation.path] === side ? "side on" : "side"}
                      aria-pressed={resolution.picks[violation.path] === side}
                      onClick={() => resolution.onPick(violation.path, side)}
                    >
                      {label}
                    </button>
                  ))}
                </span>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/**
 * The patch log, and the provenance column §5's audit trail is made of (plate 5).
 *
 * Every entry in the log, not only the current branch's: a discarded branch's entries stay
 * where they are, unreferenced and inert (ADR 0001 §2), and a trail that hid them would not be
 * an audit trail. What says which is which is one class — the rows off the current branch are
 * dimmed, and `history.ts` decides which those are.
 *
 * `undo` and `redo` appear as themselves, with no marking and no filter, because an undo is a
 * thing that happened rather than a thing that unhappened (ADR 0005 §4). The ⌘Z *walk* skips
 * them, in the session, for a reason that is about walking and not about reading.
 *
 * There is no version column, deliberately: `version` counts per branch (ADR 0005 §4's
 * caveat), so a column of them would read as one sequence over what are several.
 */
function PatchLog({ view }: { view: Log }) {
  return (
    <table className="log">
      <thead>
        <tr>
          <th>entry</th>
          <th>tool</th>
          <th>by</th>
          <th>when</th>
          <th className="count">ops</th>
          <th>at</th>
        </tr>
      </thead>
      <tbody>
        {view.entries.map((row) => (
          <tr key={row.id} className={row.onBranch ? "here" : "elsewhere"}>
            {/* The tail of the ULID, with the whole of it a hover away. A ULID's leading
                characters are its millisecond timestamp, so in one session they are all the
                same and the tail is the half that tells two entries apart. */}
            <td className="eid" title={row.id}>
              {row.id.slice(-8)}
            </td>
            <td>
              {row.tool}
              {row.merge ? <span className="dim"> ⑂ {row.parents.length}</span> : null}
            </td>
            <td className="who">{row.author}</td>
            <td className="when">{row.createdAt.slice(0, 19).replace("T", " ")}</td>
            <td className="count">{row.ops}</td>
            <td className="at">{row.refs.join(" · ")}</td>
          </tr>
        ))}
      </tbody>
    </table>
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
