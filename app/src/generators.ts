// The generator list, as a pure function of the model (docs/specs.md §9, ADR 0012 §2).
//
// What a code view needs off the document and nothing more: the source the editor opens on, the
// seed and the toolchain its title bar shows, and whether this generator has *ever* compiled.
// There is no status word here. `never` and `compiled` are read off `compiled_hash`'s emptiness
// — the bar view's own rule, so the two readers of this field agree on its wording (ADR 0026
// §4) — and `stale` is not derivable from a document at all: it is `core`'s comparison of a
// fresh hash against this one, and the one hasher is `core`'s (ADR 0024 §6, trap 3). So this
// view carries the **fact** (`compiled`) and `code.tsx` says the word, which is what ADR 0024
// §6 means by "the projection golden sees … whether `compiled_hash` is empty, and not the
// status word".
//
// Order is the model's own: by id, which is the order `ai`'s bar view prints generators in, so
// the model's list and the window's `<select>` cannot disagree about which generator is first.
// A ULID sorts by when it was minted, so that is also definition order.

import type { Generator, Song } from "@escribass/schema/song";

/** What a generator's `target` oneof says, flattened — `track_id | clip_id` (ADR 0002 §3). */
export interface Target {
  /** `clip`, `track`, or `none` for a document the validator would refuse (§4.4). */
  readonly kind: "clip" | "track" | "none";
  readonly id: string;
}

export interface GeneratorView {
  readonly id: string;
  /** `GENERATOR_KIND_PYTHON` with the prefix dropped, as the arrangement spells a track kind. */
  readonly kind: string;
  readonly target: Target;
  /** The code the editor opens on, exactly as the document holds it. */
  readonly source: string;
  /** A `uint64`, as the **string** it crosses JSON as (ADR 0002 §1) and as `apply_patch` would
   *  write it. Never a JavaScript number: the schema fixture's seed is 9007199254740993, which
   *  is 2⁵³+1 and the first integer a double cannot hold (trap 10). */
  readonly seed: string;
  /** What compiled it last, written by the first compile from what the child reported — empty
   *  until then (ADR 0027 §1). */
  readonly toolchainVersion: string;
  /** Whether `compiled_hash` is set. **Not** whether it is current: see the file header. */
  readonly compiled: boolean;
  /** How many `params` the document sets for this generator. A count and not the map, for
   *  `mixer.ts`'s reason — the editor shows the source, and `params` is edited the way the
   *  source is, by `apply_patch` on its path (ADR 0026 §2). */
  readonly params: number;
}

export interface Generators {
  readonly generators: readonly GeneratorView[];
}

/** `GENERATOR_KIND_PYTHON` → `python`, the word `ai`'s view prints (ADR 0026 §4). */
function generatorKind(generator: Generator): string {
  const named = ["unspecified", "python"];
  return named[generator.kind] ?? "unspecified";
}

function target(generator: Generator): Target {
  switch (generator.target.case) {
    case "clipId":
      return { kind: "clip", id: generator.target.value };
    case "trackId":
      return { kind: "track", id: generator.target.value };
    default:
      // `generator_target_unset` — legal on the wire, refused by the validator (§4.4). Named
      // rather than drawn blank, as `deviceLabel` names an unset device.
      return { kind: "none", id: "" };
  }
}

/** Every generator in the song, in id order. */
export function generators(song: Song): Generators {
  return {
    generators: Object.values(song.generators)
      .sort((a, b) => a.id.localeCompare(b.id))
      .map((generator) => ({
        id: generator.id,
        kind: generatorKind(generator),
        target: target(generator),
        source: generator.source,
        seed: String(generator.seed),
        toolchainVersion: generator.toolchainVersion,
        compiled: generator.compiledHash !== "",
        params: Object.keys(generator.params).length,
      })),
  };
}

/** The three words §9 names, and nothing else may be shown in their place. */
export type Status = "never" | "compiled" | "stale";

/**
 * The status word, from the document and from `core`'s last answer about this generator.
 *
 * **`never` is the document's**, read off `compiled_hash`'s emptiness — the same expression
 * `ai/src/escribass_ai/view.py` prints `never` or `compiled` from, and the same one
 * `core/src/session.rs` tests when it decides whether a dry run can answer *up to date* without
 * spawning anything. One field, three readers, one rule.
 *
 * **`stale` is `core`'s** and is never worked out here: it is the dry run of `compile_generator`
 * coming back with a patch, where *up to date* comes back valid with none (ADR 0024 §6). So
 * `answered` is `"stale"` or `"compiled"` when a dry run has been made and is standing, and
 * `null` otherwise — and with nothing standing the word is the document's, which is exactly what
 * the bar view shows a model (ADR 0026 §4).
 *
 * `never` wins over an answer, because a generator that has never compiled is `never` whatever a
 * dry run would write: the diff on screen is its *first* compile, not a refresh.
 */
export function status(view: GeneratorView, answered: Status | null): Status {
  if (!view.compiled) return "never";
  return answered ?? "compiled";
}

/** One diagnostic, where the editor puts it. */
export interface Diagnostic {
  /** 1-based, the child's own (ADR 0026 §1, as M4 PR 3 wrote it). */
  readonly line: number;
  /** 1-based, or `0` when the child had no position — `core` then writes the line alone. */
  readonly column: number;
  /** The child's own words, with `core`'s `line:column` taken off the front. */
  readonly message: string;
}

/**
 * The `generator_error` in a refusal, read back to the line it names.
 *
 * `core` writes the position in front of the child's text — `"4:24: `/` (Div) is not in the
 * generator DSL…"`, or the line alone when the column is 0 — and does not rewrite the words,
 * because the same text a person reads is what a model retries against (ADR 0026 §3). This
 * takes the position back off so the editor can underline the token, and hands the rest over
 * **unchanged**: the message is the whole fix, and a view that summarised it would be throwing
 * away the thing PR 4 wrote forty-eight of these to teach.
 *
 * Every other refusal `compile_generator` can give — `generator_unknown`,
 * `target_not_note_clip` — names no line, so it stays in the pane head where every refusal is
 * shown and nothing is put in the editor's margin that is not about the source.
 */
export function diagnosticAt(
  errors: readonly { readonly rule: string; readonly message: string }[],
): Diagnostic | null {
  const said = errors.find((broken) => broken.rule === "generator_error");
  if (!said) return null;
  const at = /^(\d+)(?::(\d+))?: ([\s\S]*)$/.exec(said.message);
  // A `generator_error` whose message does not start with a position: nothing in `core` writes
  // one today, and a view that assumed otherwise would silently drop the refusal. Shown at the
  // top of the file rather than nowhere.
  if (!at) return { line: 1, column: 0, message: said.message };
  return { line: Number(at[1]), column: Number(at[2] ?? 0), message: at[3] };
}
