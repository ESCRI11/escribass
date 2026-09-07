// The window: one decoded `Song`, and views that are pure selectors over it (ADR 0012 §2).
//
// There is no store here, and there is not going to be one. The song is read once from
// `get_song` and held whole; when an edit lands — PR 5 is the first that makes one — the
// answer is another `get_song` and a wholesale replacement, never a local RFC 6902 apply and
// never a normalised cache. §14.2 forbids the second representation, ADR 0016 §3 keeps every
// library that would quietly introduce one out of `package.json`, and ADR 0012 §5's
// projection golden is what would catch a hand-written one.

import { useEffect, useMemo, useState } from "react";
import { fromJson } from "@bufbuild/protobuf";
import type { JsonValue } from "@bufbuild/protobuf";
import { SongSchema, type Song } from "@escribass/schema/song";
import { arrangement } from "./arrangement.js";
import { Timeline } from "./timeline.js";
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

export function App() {
  const [song, setSong] = useState<Song | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  useEffect(() => {
    tool("get_song", {})
      .then((answer) => setSong(freeze(fromJson(SongSchema, answer as JsonValue))))
      .catch((e: unknown) => setFailure(String(e)));
  }, []);

  const view = useMemo(() => (song ? arrangement(song) : null), [song]);

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

  return (
    <main>
      <header>
        <span className="tempo">{view.bpm.toFixed(2)} BPM</span>
        <span className="signature">
          {view.numerator}/{view.denominator}
        </span>
      </header>
      <div className="arrangement">
        <div className="gutter">
          <div className="gutter-head" />
          {view.tracks.map((track) => (
            <div className="track" key={track.id}>
              <span className="name">{track.name}</span>
              <span className="kind">{track.kind}</span>
            </div>
          ))}
        </div>
        <Timeline view={view} />
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
        <span className="mode">read-only</span>
      </footer>
    </main>
  );
}
