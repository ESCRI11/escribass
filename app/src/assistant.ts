// The AI panel, as a pure function of what the host answers (docs/specs.md §9; ADR 0019 §2,
// §3; ADR 0021 §3).
//
// Every other view in this window is a projection of the `Song`; this one is a projection of
// the **conversation and the proposal**, which are the host's and not song state — the
// conversation because a prompt is not the song (ADR 0021 §3), the proposal because nothing of
// it has been written (ADR 0019 §1). What makes it the same kind of file as `mixer.ts` is the
// shape: input in, a serialisable description out, no state, no clock, no way to reach the
// host. The projection golden compares it against committed bytes like the rest (ADR 0012 §5).
//
// The one place it reads the `Song` is [`moved`], and that is deliberate. ADR 0012 §4 says the
// refusal a pending edit meets has to **name what moved** rather than quoting a number, and
// the only thing that can turn `/tracks/01M…/version` into "the Lead track" is the document.
// That is a lookup, not a rule: §4.4 stays the validator's, and nothing here decides whether
// anything is valid (ADR 0017 §3).
//
// **What this file will not say.** It never calls the model verified, pinned or deterministic.
// `ai.model` is a record of a person's choice and nothing checks it (ADR 0021 §4), so the
// panel names it the way the transport names a preview — with the caveat beside it, where the
// person is looking (M2 trap 3's precedent, M3 trap 15's rule).

import type { Song } from "@escribass/schema/song";

/** One rule something broke, as every carrier already sends one (§5). */
export interface Broken {
  readonly path: string;
  readonly rule: string;
  readonly message: string;
}

/** One of the model's calls, as the host records it (`panel.rs`, `Call`). */
export interface Called {
  readonly call_id: string;
  readonly name: string;
  /** The arguments the model wrote, verbatim (assistant.proto). */
  readonly args: string;
  readonly model_id: string;
  readonly valid: boolean;
  readonly summary: string;
  readonly errors: readonly Broken[];
}

/** What became of a turn — the panel's record, never the model's (ADR 0021 §3). */
export interface Became {
  readonly applied?: string;
  readonly rejected?: boolean;
  readonly edited?: string;
  readonly failed?: string;
}

/** One settled turn, as the conversation file holds it. */
export interface Settled {
  readonly prompt_id: string;
  readonly prompt: string;
  readonly at: string;
  readonly provider: string;
  readonly model_id: string;
  readonly calls: readonly Called[];
  readonly reply: string;
  readonly outcome: Became;
}

/** The turn in flight, or the one waiting for a decision (`panel.rs`, `Live`). */
export interface Live {
  readonly prompt: string;
  readonly prompt_id: string;
  readonly at: string;
  readonly model_id: string;
  readonly calls: readonly Called[];
  readonly reply: string;
  readonly running: boolean;
  /** `Answered`, or `Refused` when the turn ended on its third refused call (ADR 0022 §3). */
  readonly ended: string;
  /** The RFC 6902 operations a person approves — the diff §9 shows before applying. */
  readonly patch: readonly unknown[] | null;
  /** Why there is no patch, when there is none: the proposal's own violations. */
  readonly refused: readonly Broken[];
  /** The document the proposal has, so the timeline and the roll can draw it dashed. */
  readonly song: unknown | null;
  /** What Apply or Edit was last refused with, if it was. */
  readonly refusal: { readonly valid: boolean; readonly errors: readonly Broken[] } | null;
}

/** What the `panel` command answers with. */
export interface PanelAnswer {
  readonly provider: string;
  readonly model: string;
  /** Where the conversation is kept, for the panel to say so (ADR 0021 §3). */
  readonly at: string;
  readonly conversation: readonly Settled[];
  readonly live: Live | null;
}

/** One line under a model's message: the call, and what it was answered. */
export interface CallLine {
  readonly name: string;
  readonly args: string;
  /** `valid · 2 ops: …`, or the rules it broke. */
  readonly verdict: string;
  readonly refused: boolean;
}

export interface Message {
  readonly who: "you" | "model";
  readonly said: string;
  readonly calls: readonly CallLine[];
  /** What happened to the turn, in the panel's own words. Empty while it is running. */
  readonly note: string;
}

/** The pending proposal, as the panel draws it. */
export interface Pending {
  readonly running: boolean;
  /** The operations, as `ToolResult.patch` carries them (ADR 0006 §1) — the patch that lands,
   *  not a description of it. */
  readonly ops: readonly unknown[];
  /** `3 operations`, or what is in the way of there being any. */
  readonly summary: string;
  /** Whether there is anything to apply. A turn that changed nothing offers Reject alone, for
   *  the reason a gesture that has not moved does (`App.tsx`). */
  readonly applicable: boolean;
  /** Why the proposal cannot be applied as it stands, each naming **what moved** rather than a
   *  version number (ADR 0012 §4). */
  readonly refusals: readonly string[];
  /** The text Edit starts from: the patch, as a person would type it (ADR 0019 §3). */
  readonly text: string;
}

export interface Assistant {
  /** `openrouter · deepseek/deepseek-v4.1-flash`, as the wireframes' panel head names it. */
  readonly model: string;
  /** What that name is and is not. Never "verified" (M3 trap 15). */
  readonly caveat: string;
  readonly messages: readonly Message[];
  readonly pending: Pending | null;
  /** Whether a prompt can be sent: one turn is one proposal (ADR 0019 §3). */
  readonly asking: boolean;
  /** Where the conversation is kept, and what that means for a project that travels. */
  readonly kept: string;
}

/**
 * The sentence beside the model's name.
 *
 * The *live preview · not the render* precedent, one panel over: the caveat sits where the
 * fact is shown rather than in a help page, and it says the two things that are true — the
 * name is a record nothing verifies, and what is reproducible is the song from its log rather
 * than anything the model does (ADR 0021 §4; docs/plan.md, M3 trap 15).
 */
export const RECORDED_NOT_PINNED =
  "This is the model this project records in `lock.json` and the id a prompt is sent to — a " +
  "choice, not a pin. Nothing here verifies it: a hosted model changes under its own name, " +
  "no commit or hash describes one, and what actually answered is recorded per entry in the " +
  "log. The model is not deterministic and is never told it is; what is reproducible is the " +
  "song, from its patch log, whoever wrote it.";

/** What the panel says about where the conversation lives (ADR 0021 §3). */
export function keptAt(at: string): string {
  return at === ""
    ? "This conversation is this window's only: no application data directory could be named, " +
        "so nothing is written down and the log's prompt ids will name prompts this machine " +
        "does not have."
    : `The conversation is kept beside the project and outside it, at ${at} — one line per ` +
        "turn, keyed by the song's id. It is not part of the project and does not travel with " +
        "it: a project opened on another machine names prompts that machine does not have.";
}

/** `2 ops: /tracks/…` or the rules a call broke, whole (ADR 0022 §3). */
function verdict(call: Called): string {
  if (call.valid) return call.summary === "" ? "valid" : `valid · ${call.summary}`;
  const said = call.errors.map((broken) => `${broken.rule}: ${broken.message}`).join("; ");
  return `refused · ${said}`;
}

function lines(calls: readonly Called[]): readonly CallLine[] {
  return calls.map((call) => ({
    name: call.name,
    args: call.args,
    verdict: verdict(call),
    refused: !call.valid,
  }));
}

/** What became of a settled turn, in a sentence a reader can act on. */
function became(turn: Settled): string {
  if (turn.outcome.applied !== undefined) {
    return `applied as ${turn.outcome.applied}, by ${turn.model_id || "the model"}`;
  }
  if (turn.outcome.edited !== undefined) {
    return `edited and applied as ${turn.outcome.edited}, by you`;
  }
  if (turn.outcome.failed !== undefined) return `did not finish — ${turn.outcome.failed}`;
  return "rejected: nothing was written";
}

/**
 * Which thing in the document a violation is about, named from the document itself.
 *
 * ADR 0012 §4 asks the refusal to say what moved rather than which number was wrong, and
 * ADR 0019 §2 repeats it for a proposal: "the Lead track changed while this proposal was
 * pending". The path is the validator's and so is the rule; what this adds is the name the
 * person knows the thing by.
 */
export function moved(song: Song | null, path: string): string {
  // The path is the **canonical JSON**'s, which names proto fields (`tempo_map`, never
  // `tempoMap`); the decoded `Song` beside it names them the generated way. The three maps
  // this looks in — `tracks`, `clips`, `sections` — are spelt the same in both, and the two
  // that are not are named rather than indexed (ADR 0002 §4, ADR 0006 §6).
  const [, kind, id] = path.split("/");
  if (song === null) return path;
  if (kind === undefined || kind === "version" || kind === "") return "the song";
  const track = song.tracks[id ?? ""];
  if (kind === "tracks" && track) return `the ${track.name} track`;
  if (kind === "sections" && song.sections[id ?? ""]) {
    return `the ${song.sections[id ?? ""].name} section`;
  }
  const clip = kind === "clips" ? song.clips[id ?? ""] : undefined;
  if (clip) {
    const on = song.tracks[clip.trackId]?.name;
    return on === undefined ? "a clip" : `the clip on ${on}`;
  }
  // A device is not a top-level map: an `Instrument` and an `Effect` live inside their track
  // (`song.proto`), so `/tracks/<id>/instrument/...` is already the track above.
  if (kind === "tempo_map") return "the tempo map";
  if (kind === "time_signature_map") return "the time signature map";
  if (kind === "automation") return "an automation lane";
  return path;
}

/** One refusal, as the panel reads it out. */
export function refusalLine(song: Song | null, broken: Broken): string {
  if (broken.rule === "version_not_writable") {
    return `${moved(song, broken.path)} changed while this proposal was pending`;
  }
  return `${moved(song, broken.path)}: ${broken.message}`;
}

function pending(live: Live, song: Song | null): Pending {
  const ops = live.patch ?? [];
  const refusals = [
    ...live.refused.map((broken) => refusalLine(song, broken)),
    ...(live.refusal?.errors ?? []).map((broken) => refusalLine(song, broken)),
  ];
  const summary =
    refusals.length > 0
      ? refusals.join("; ")
      : ops.length === 0
        ? live.running
          ? "nothing proposed yet"
          : "the model proposed no change"
        : `${ops.length} operation${ops.length === 1 ? "" : "s"}`;
  return {
    running: live.running,
    ops,
    summary,
    // Nothing to approve is nothing to apply, which is the rule a gesture that has not moved
    // already follows (`App.tsx`): a pane offering to apply nothing is offering nothing.
    applicable: !live.running && ops.length > 0 && live.refused.length === 0,
    refusals,
    // Indented, because it is text a person edits by hand and the one place in this
    // application where they write RFC 6902 (ADR 0019 §3).
    text: JSON.stringify(ops, null, 1),
  };
}

/**
 * The panel, from what the host answered and the document the window is holding.
 *
 * `song` is the **project's** document, not the proposal's: it is what a refusal names, and a
 * proposal is refused precisely when the two disagree.
 */
export function assistant(answer: PanelAnswer, song: Song | null): Assistant {
  const messages: Message[] = [];
  for (const turn of answer.conversation) {
    messages.push({ who: "you", said: turn.prompt, calls: [], note: "" });
    messages.push({
      who: "model",
      said: turn.reply,
      calls: lines(turn.calls),
      note: became(turn),
    });
  }
  const live = answer.live;
  if (live !== null) {
    messages.push({ who: "you", said: live.prompt, calls: [], note: "" });
    messages.push({
      who: "model",
      said: live.reply,
      calls: lines(live.calls),
      note: live.running
        ? "answering…"
        : live.ended === "Refused"
          ? `the turn ended: ${live.calls.filter((call) => !call.valid).length} calls were refused`
          : "waiting for you",
    });
  }
  return {
    model: `${answer.provider} · ${answer.model}`,
    caveat: RECORDED_NOT_PINNED,
    messages,
    pending: live === null ? null : pending(live, song),
    asking: live === null,
    kept: keptAt(answer.at),
  };
}

/**
 * The clips the proposal has changed or added, by id — drawn dashed where the wireframes draw
 * a pending edit (plate 1, note 4; ADR 0019 §2).
 *
 * Compared as the projection describes them, so a clip that moved, grew, changed content or
 * started looping is pending and one the proposal only touched the mix of is not.
 *
 * `ponytail:` a clip the proposal **removes** is not drawn at all — the timeline draws the
 * proposal's own arrangement, so the row is simply not there, and the diff is what says it
 * went. The roll does draw a removed note, because that is the case plate 2 names by hand
 * ("otherwise Reject looks like data loss") and a note is where a person is looking at single
 * events. The upgrade path, if a removed clip ever needs the same treatment, is the union of
 * the two arrangements rather than the proposal's — which is a second merge of track rows, and
 * this is one line.
 */
export function dashed(
  current: readonly { readonly id: string; readonly clips: readonly unknown[] }[],
  /** `null` when there is no proposal — see [`dashedNotes`] for why that is this function's
   *  to say rather than its caller's. */
  proposed: readonly { readonly id: string; readonly clips: readonly unknown[] }[] | null,
): ReadonlySet<string> {
  if (proposed === null) return new Set();
  const before = new Map<string, string>();
  for (const track of current) {
    for (const clip of track.clips) {
      before.set((clip as { id: string }).id, JSON.stringify(clip));
    }
  }
  const changed = new Set<string>();
  for (const track of proposed) {
    for (const clip of track.clips) {
      const id = (clip as { id: string }).id;
      if (before.get(id) !== JSON.stringify(clip)) changed.add(id);
    }
  }
  return changed;
}

/**
 * What the proposal asks for, and what it takes away — **over** the model rather than instead
 * of it (ADR 0017 §3's picture, with a proposal in place of a drag).
 *
 * The roll goes on drawing the document's own notes solid. `pending` is what the proposal
 * would have instead, drawn dashed, so a note it moves is visible in both places at once and
 * a note it adds is visible at all. `gone` is what it removes, outlined in the refusal colour,
 * because a note that simply vanished before anybody approved anything would make Reject look
 * like data loss (wireframes, plate 2).
 */
export function dashedNotes<T extends { readonly id: string }>(
  current: readonly T[],
  /** The proposal's notes, or `null` when there is no proposal.
   *
   *  **`null` and not an empty list**, and the distinction is a defect this had: with no
   *  proposal every note in the document is a note the proposal does not have, so an empty
   *  list means "it removes all eighteen of them" and the roll outlines the whole clip in the
   *  refusal colour. Found a second after pressing Apply, in the window. The rule is here
   *  rather than at the call site because there will be more call sites than there are
   *  rules. */
  proposed: readonly T[] | null,
): { readonly pending: readonly T[]; readonly gone: readonly T[] } {
  if (proposed === null) return { pending: [], gone: [] };
  const before = new Map(current.map((note) => [note.id, JSON.stringify(note)]));
  const kept = new Set(proposed.map((note) => note.id));
  return {
    pending: proposed.filter((note) => before.get(note.id) !== JSON.stringify(note)),
    gone: current.filter((note) => !kept.has(note.id)),
  };
}
