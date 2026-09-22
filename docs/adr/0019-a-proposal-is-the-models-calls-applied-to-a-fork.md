# ADR 0019 — A proposal is the model's calls applied to a fork, and a person applies one entry

- **Status:** Accepted (2026-09-21)
- **Affects:** `core/src/session.rs` (a `Proposal` the session holds, as it holds the one live
  preview); `core/src/project.rs` (nothing new — `prepare` is what a proposal runs); the loop
  (M3 PR 8) and the AI panel (M3 PR 9); `docs/specs.md` §5, §9 and §15
- **Builds on:** ADR 0017 §1 (one gesture is one entry) and §4 (the release proposes and a person
  applies), whose Consequences say "a gesture that cannot be expressed as repeated dry runs of
  one tool call is a gesture that needs its own ADR" — this is that ADR; ADR 0006 §3 (`dry_run`
  is the pure first half of the apply path, never a second implementation of it); ADR 0012 §4
  (a held dry run is applied optimistically and §4.3's `version` check refuses; a dry run
  already returns real ids); ADR 0005 §3 (a patch that states the version core computes is
  disputing nothing); ADR 0001 §5 (ids from an injectable source, forked for a preview);
  ADR 0018 §2 (the view is re-read after every applied call)
- **Recorded in:** `docs/specs.md` §5, §9 and §15.

## Context

§9 says the AI panel "always shows the diff before applying; users can apply, reject, or
edit", and §16 calls M3 "a tool-calling loop with dry-run/diff/apply". ADR 0017 gave the first
control that flow whole — every position of a drag is a `dry_run`, one entry is committed, a
person applies — and left the one case it did not have to face to `docs/plan.md`'s question 9:
**a proposal made of several tool calls, when dry runs do not compose.**

They do not compose for a reason in `core/src/session.rs` and `core/src/id.rs`, not in any
model. Every dry run builds its patch from a **fork** of the session's id source and, being a
dry run, writes nothing; `from_tool` then puts the previous source back. So two dry runs in a
row mint the same ids from the same position, and neither leaves a track for the other to put a
clip on — the second call of "add a track, then a clip on it" names a track that exists in a
document that was thrown away. And an apply after a dry run does not keep the preview's ids
either: under a real clock the source has moved on, and the same `add_track` applied for real
mints a different id from the one the preview showed (`id.rs`, on `fork`; measured in the spike:
`…ECTRWWD1FDJS6C9KK8` previewed, `…EFMX5FHZ7JEFSTE7VX` applied). What *does* keep them is
`apply_patch` of the dry run's own patch, whose ids and version claims are exactly what `core`
computes (ADR 0005 §3; ADR 0012 §4) — which is how the window applies a drag.

The spike measured how models actually propose, and the result reframes the question (plan,
"What the spike found", run 2026-09-17; one day, five routes, cited as evidence and never as the
claim — trap 4). Of 116 successful runs of the nine feasible instructions, 42 applied more than
one mutating call and 28 had a call naming an entity an earlier call in the same instruction had
created. **Not one built a multi-call edit out of dry runs.** Every dependent chain applied its
first step for real and then named the id that call's result returned. Through Claude Code,
Opus 5 and Sonnet 5 skipped the dry run entirely on every multi-call instruction, 10 runs of 10
— Opus while dry-running every single-call instruction, so it stopped previewing exactly where a
preview cannot compose. DeepSeek V4.1 Flash, the default model, hit the trap live twice: it
applied `add_track`, dry-ran `add_clip` on the id an earlier dry run had shown, and was refused
`track_unknown`. One DeepSeek run applied its edit by re-sending the previewed patch through
`apply_patch`, unprompted; none minted its own ids.

Two conclusions the ADR is written from. Restricting a proposal to one call would have refused
I6, I7 and I8 — a new track with anything on it, and any two edits in one sentence, which is
the bass-line example the question itself uses. And a loop that *instructs* a model to preview
every step and apply none is asking for behaviour no model showed, in the one place the product
promises a person sees the diff first. ADR 0017's flow does not survive contact with how models
propose; what survives is its two halves separately — validate every step, commit once — and
the thing that has to change is **who composes the steps**. Not the model. `core`.

## Decisions

### 1. A proposal is the model's calls applied, in `core`, to a fork of the document

`core` gains a **`Proposal`**: the document as it stands, cloned in memory; a fork of the
session's id source, **kept** across calls rather than put back after each; and the calls so
far. The session holds at most one, as it holds at most one live preview — session state about a
running turn, never the document or the log. The model's calls are executed against it:
`Proposal::call(name, args)` runs the same `tools::*` function against the fork's song, with the
fork's ids and the proposal's provenance (ADR 0021 §2), `prepare`s the result on the fork, and
on `valid` swaps the fork's song for the prepared one. **It writes nothing and records
nothing.** What it answers is an ordinary `ToolResult` — `valid`, every `Violation`, the step's
own patch, its summary, and an empty `entry_id`, which is true: nothing was committed.

So the second call's track exists, in the fork, under the id the first call's result returned,
which is precisely what every model in the spike did with the ids a real apply returned. The
model is not asked to preview, to compose, or to know what a proposal is; it applies step by
step, as it does anyway, and nothing touches the project. A model that sends `dry_run: true`
inside a proposal gets a dry run of the fork — `prepare` without the swap — so the models that
preview first get the same answers as the ones that do not. The view the model reads between
calls (ADR 0018 §1) is computed over the fork's song, which is ADR 0018 §2's "re-read after
every applied call" with "applied" meaning applied to the proposal.

This is not a second implementation of anything. `prepare` is pure over a `Project`'s song and
`Project` is `Clone`; a proposal is `prepare` N times on a copy, with the same tool functions,
the same validator and the same version rule (ADR 0006 §3, ADR 0005 §1). What is new is the
holder and the fork of ids that is not put back. It costs the log nothing per step — trap 9's
`Project::write` is paid once, at approval — which is ADR 0017 §1's second reason arriving
unchanged.

Which calls go to the proposal is decided by the **carrier**, not by the author: a call that
arrives on the `ai` stream (ADR 0020 §1) is proposed, a call from the window or from an MCP
client is applied as it always was. The validator does not know the difference and is not told
(plan, trap 18: what differs by author is what is offered and where it lands, never what is
accepted).

### 2. A person approves the proposal's patch, once, when the model's turn ends

Said plainly, because the plan asked for it plainly: **a person approves one thing — the RFC
6902 patch from the document as it stands to the document the model's calls produced — once,
when the model's turn ends, and nothing of the model's touches the project before that.** The
turn ends when the model stops calling tools and answers (ADR 0020 §3's `done`). Until then the
panel may draw the proposal as it grows, dashed where the wireframes draw a pending edit,
because the patch below is computable after every call; the person acts on it when the model
is finished.

**What the patch is.** `diff(current, proposal.song)`, with `version`s as **one** `prepare`
would compute them: a new entity at 1, an entity the proposal touched at `before + 1`. The
fork's per-step bumps — a track edited three times in one proposal is at `before + 3` in the
fork — are scaffolding and never reach the log. Mechanically: the fork's final document with
every entity's `version` put back to the current document's (a new entity at 0, which is what a
tool builds one at), diffed against the current document, and `prepare`d once; the re-derived
`Prepared.ops` is the proposal's patch. It is the patch a single tool call producing that
document would have recorded, and it is what the panel shows.

**How it is applied.** Through `Session::run`, under the tool name **`proposal`** — the
function every mutating tool ends in, with a label. One entry, whatever the number of calls
(ADR 0017 §1). No new commit path: `run` prepares, describes or records, and the version rule
is applied by the same code to the same shape.

**When the document moved.** The patch carries version claims of `before + 1`, so it is applied
exactly as a held drag is (ADR 0012 §4): optimistically, and if a person edited an entity the
proposal touched between the turn's end and Apply, ADR 0005 §3's guard refuses with
`version_not_writable`, naming the entity. An edit elsewhere merges cleanly, as it does for a
drag. What M3 owes here is the sentence ADR 0012 §4 already owes — the refusal names *what
moved*, "the Lead track changed while this proposal was pending", and not a number — and the
way out is Reject and ask again, since the loop re-reads the document on every prompt. A
person's held drag and a pending proposal never see each other before one of them applies:
the drag's dry runs are against the document, the proposal's calls against the fork (plan,
trap 13).

### 3. Reject drops it, Edit is the person's, and there is one proposal at a time

**Reject** discards the proposal. Nothing was written, so nothing is undone and the log records
nothing; the conversation records the outcome (ADR 0021 §3). **Apply** is decision 2. **Edit**
is §9's third control and the one place in the application where a person writes RFC 6902 by
hand: the person edits the proposal's patch text and it is applied through `apply_patch`, as
the **person's** — `AUTHOR_HUMAN`, with the `prompt_id` kept so the history still leads to the
conversation (ADR 0021 §2). A person who changed the bytes owns the bytes; what the entry
records is a hand-written patch made from a proposal, which is what happened. It needs the
refusal path a drag has (ADR 0017 §3): a patch that will not apply is shown refused with the
rule, and the proposal stays pending.

The order is the plan's default and is kept: **Apply and Reject land first, Edit last**, after
both have been driven against a real proposal (PR 9), because Edit is the control with the most
ways to be wrong and the fewest users on day one.

**One proposal per session at a time.** A new prompt while one is pending is refused by the
loop — "apply or reject the pending proposal first" — rather than stacking a second fork on
the first or silently dropping what a person has not yet read. The model cannot start one on
its own; a turn is a proposal, and the person ends it.

### 4. What the entry says, and what it does not

The entry's `tool` is `proposal`; its `provenance` is the model's, with `model_id` and
`prompt_id` (ADR 0021 §2); its `ops` are the change. **The individual calls it was composed
from are not in the entry.** They live in the conversation, under the `prompt_id` the entry
carries (ADR 0021 §3), which is where a reader also finds the prompt they answered.
`history.proto` gains no field for them: a `repeated string calls` would be a schema change
(CLAUDE.md #5) carrying a list a reader would still need the conversation to interpret —
"add_clip" says nothing without its arguments, and its arguments are the ops. The audit trail
§5 asks for is who, when, from what, and what changed; the entry answers all four.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| **(a) The model works on a branch**: `create_branch`, every call applied for real, Apply is `merge_branch`, Reject is `delete_branch` | `switch_branch` moves the session's one `HEAD` under the window, so a person's edits made while the model works land on the proposal branch and Reject discards them. Every step is a `Project::write` — 17 ms at 21 entries, 30.6 ms at 300 and rising (ADR 0017 §5; trap 9). A rejected proposal's entries stay in the log for ever (ADR 0001 §2), which is honest and is also an audit trail of things that never happened to the song. And a merge conflict becomes the routine case rather than the rare one |
| **(c) One mutating call per proposal** | Refuses I6, I7 and I8: a third of the spike's successful edits applied more than one change, and "add a bass line" is a track, a clip and its notes |
| Instruct the model to preview every step and apply none | 0 of 116 successful runs did; 10 of 10 multi-call runs through Claude Code skipped the dry run; V4.1 Flash hit the id trap live, twice. A flow that depends on an instruction the spike watched every model ignore is not a flow |
| Apply for real and offer ⌘Z as Reject | §9's diff before apply is the product's claim (§18.1.3), not a preference. An undo is a second entry, so a rejected proposal is two rows in the audit trail, and between them the document was the model's |
| Have the model call `apply_patch` with ids it mints itself | No model did (0 of 116), and a self-minted id is unseeded randomness in the one place ADR 0001 §5 took it out of |
| Keep the fork's versions and commit through `prepare_merge` | `max(ours, theirs) + 1` takes a track touched three times from `v` to `v + 4`, and a merge disputes nothing (ADR 0005 §3) — so a person's concurrent edit to that track would be merged over in silence, which is the check ADR 0012 §4 exists to keep |
| One entry per model call, all committed at Apply | ⌘Z then undoes one call of a proposal a person approved as a whole, and the audit trail gains N rows for one approval — ADR 0017 §1's argument, with a model instead of a pointer |
| A `calls` field on `PatchEntry` | A `history.proto` change for a list that does not explain itself; `prompt_id` already reaches the conversation, which does |
| Approve every call as it arrives | The models do not preview per call, so the panel would interrupt a chain the model is mid-way through; and N dialogs per prompt is the friction ADR 0017 §4's `ponytail:` already suspected for one gesture, multiplied |
| A proposal in `ai`, holding its own copy of the song | `ai` has no `prepare`, no validator and no id source; it would need a second implementation of all three in Python, which is trap 12 three times over |

## Consequences

- **PR 8** (`m3.8-loop`) builds `Proposal` in `core` — the fork, the kept id source, `call`,
  the patch of decision 2, and `apply` through `run` under `proposal` — and the loop that
  routes the model's calls to it. Its tests drive a two-call proposal (a track, then a clip on
  it) and assert one entry, the previewed ids in the log, and `version_not_writable` when the
  document moved under it. Trap 9's number is re-measured with a proposal of several calls
  driving `Project::write` once.
- **PR 9** (`m3.9-panel`) draws the proposal dashed as it grows and offers Apply and Reject;
  Edit lands last, with ADR 0017 §3's refusal path. `proposal` rows appear in the history view
  with the model named (ADR 0021 §2).
- **No `song.proto` and no `history.proto` change**, and nothing in `proto/` for this decision:
  the proposal is `core`'s, and the stream that carries the model's calls is ADR 0020's.
- **The M0.4 goldens do not move.** No tool's ops change and no existing path is touched; a
  proposal driven end to end is a new script (ADR 0022 §4), not a change to the five.
- `docs/specs.md` §5 gains the sentence that says what a proposal is and what a person approves;
  §9's AI panel bullet says what its three controls do.
- ADR 0017's Consequences asked for this ADR and are satisfied by it; ADR 0017 itself is
  unchanged, because every gesture the window makes still follows its decisions 1–3.
- "What M3 will not claim" gains a line: not per-call approval, and not that a person sees the
  model's steps in the log — they see the change, and the conversation holds the steps.
