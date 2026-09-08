// The window: one decoded `Song`, and views that are pure selectors over it (ADR 0012 §2).
//
// There is no store here, and there is not going to be one. The song is read once from
// `get_song` and held whole; when an edit lands — PR 5 is the first that makes one — the
// answer is another `get_song` and a wholesale replacement, never a local RFC 6902 apply and
// never a normalised cache. §14.2 forbids the second representation, ADR 0016 §3 keeps every
// library that would quietly introduce one out of `package.json`, and ADR 0012 §5's
// projection golden is what would catch a hand-written one.
//
// **This PR has no edit path at all**, not a disabled one and not a stubbed one, so that the
// answer above is reviewed on its own before anything can write (docs/plan.md, M2 PR 4). The
// one piece of state below is which clip the piano roll is looking at, and it is a string the
// user chose — not a copy of anything in the document.

import { useEffect, useMemo, useState } from "react";
import { fromJson } from "@bufbuild/protobuf";
import type { JsonValue } from "@bufbuild/protobuf";
import { SongSchema, type Song } from "@escribass/schema/song";
import { arrangement } from "./arrangement.js";
import type { Arrangement } from "./arrangement.js";
import { pianoRoll } from "./pianoroll.js";
import { HEAD, ROW, Roll, Timeline } from "./canvas.js";
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

export function App() {
  const [song, setSong] = useState<Song | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [chosen, setChosen] = useState<string | null>(null);

  useEffect(() => {
    tool("get_song", {})
      .then((answer) => setSong(freeze(fromJson(SongSchema, answer as JsonValue))))
      .catch((e: unknown) => setFailure(String(e)));
  }, []);

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
          </div>
          {roll ? <Roll view={roll} /> : <p className="empty">Nothing to show here yet.</p>}
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
        <span className="mode">read-only</span>
      </footer>
    </main>
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
