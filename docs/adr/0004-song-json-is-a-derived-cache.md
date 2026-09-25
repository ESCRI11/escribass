# ADR 0004 — `song.json` is a derived cache of the patch log

- **Status:** Accepted (2026-09-03)
- **Affects:** the project store (M0.2); `docs/specs.md` §10; ADR 0001
- **Builds on:** ADR 0001 (the log is a DAG), ADR 0002 §4 (the canonical form)
- **Recorded in:** `docs/specs.md` §15.

## Context

A commit writes three things: a patch entry under `patches/`, the new `song.json`, and
`refs.json` with the ref advanced. `std::fs::rename` makes each file individually atomic; it
does not make the three of them atomic together. A crash between the second and third leaves
`song.json` one edit ahead of what `HEAD` points at.

Nothing in §10 says which is then right. Without an answer the next commit builds on a
document that does not match its own history, and the divergence never announces itself —
the failure shows up months later as a replay that produces a different song.

The question is prior to the store's design, not incidental to it, because it decides the
write order, what `open` does, and whether replay is a repair path or a debugging tool.

## Decision

### 1. The patch log is authoritative on disk; `song.json` is a derived cache

`song.json` is materialised from the log by replay. Where they disagree, the log is right.

This is not a second representation of song state (CLAUDE.md #1, §14.2). The model is still
the only source of truth; the log and the document are two serialisations of the *same*
model — one as the sequence of validated changes that produced it, one as a snapshot of the
result. What #14.2 forbids is a second *authoritative* state, such as a per-view model or
ad-hoc JSON alongside the canonical document. Naming the snapshot as derived is what keeps
that distinction honest, which is why it is written down here rather than assumed.

`song.json` is kept, rather than dropped in favour of replay on every load, because §2.6
[MUST] requires the project be readable and diffable text. A directory whose song exists
only as a thousand patch files is neither.

### 2. `open` replays and compares; a mismatch is a structured error

Loading a project replays the log from the root to `HEAD` and compares the result with
`song.json`. (~~byte for byte~~ **corrected 2026-09-24, at M3's close**: `verify_against_replay`
compares the replay against the parsed document as two `serde_json::Value`s, so a `song.json`
that is equivalent but not canonical passes here rather than being reported. The canonical
form is enforced where it is written, not where it is read; `json_objects_are_still_written_in_key_order`
in `tests/determinism.rs` is what holds it.) A mismatch is reported — with the paths that differ — and never
silently resolved in either direction.

Silently preferring the log would discard an edit a user may have made deliberately.
Silently preferring the document would let a hand-edited `song.json` diverge from its own
history, which §5 forbids in the first place ("agents never read or write the project file
directly"). Reporting is the same posture §10 already takes on missing assets: "reported by
the validator, never silently substituted."

> `ponytail:` replay on open is O(history), the same cost ADR 0001 already accepts per branch
> switch. Cache the materialised document per ref if a large project drags; snapshot only if
> caching is not enough.

### 3. Write order is entry → `song.json` → `refs.json`, and the ref flip is the commit

Each file is written to a temporary file in the same directory and renamed into place.

Ordering follows from decision 1. The entry is appended first: an entry nothing references is
inert, which ADR 0001 §2 already relies on for discarded branches. `refs.json` is written
last, so advancing the ref is the moment the commit becomes real. A crash before it leaves an
orphan entry and a `song.json` that is ahead — both detected by decision 2, and both
recoverable by replaying to the ref that is still current.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| `song.json` authoritative, the log purely historical | A crash leaves the two disagreeing with no way to tell which is right, and no recovery path. It also makes the log decorative: nothing ever reads it, so nothing ever notices when it is wrong. |
| Log authoritative, `open` silently rewrites `song.json` from replay | Cheaper, and it hides exactly the event worth knowing about. A hand-edited document — which §5 forbids — would be silently reverted with no report. |
| Drop `song.json` and replay on every load | Violates §2.6: the project stops being readable, diffable text. It would also put the whole model behind a replay for every consumer, including `git diff`. |
| A write-ahead journal or a single packed file | Solves a problem three renames and a comparison already solve, at the cost of a format nothing else in the system needs. |

## Consequences

- **The store (M0.2 PR 5, PR 6).** `open` performs a replay. `commit` writes in the order
  above. A mismatch is a structured error with the same `path`/`rule`/`message` shape as
  `Violation`, so the orchestrator handles it like any other. (~~and an orphan entry~~
  **corrected 2026-09-24, at M3's close**: an entry no ref reaches is **loaded and inert**, not
  reported — which is what ADR 0001 §2 requires and what
  `an_orphan_entry_left_by_a_crash_does_not_become_history` in `core/tests/project.rs` asserts.
  Two ADRs disagreed and the code kept the other one; this one was wrong.)
- **The determinism suite (M0.4).** Replay-and-compare on open is the same assertion the
  suite makes, so every test that opens a project exercises it for free.
- **§10** gains one sentence naming `song.json` as derived. ADR 0001 is unaffected: it already
  specifies materialise-and-diff as replay from the root, which is this decision's mechanism.
- **No new dependency, no schema change**, so no `buf breaking` implications and no
  golden-render pass.
