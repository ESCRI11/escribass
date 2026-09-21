# ADR 0022 — The model is offered twelve tools, and a failure is one of three kinds

- **Status:** Accepted (2026-09-21)
- **Affects:** `core/src/call.rs` (`OFFERED` beside `IMPLEMENTED`), `core/src/descriptor.rs`
  (the schemas, filtered); `ai/` (the loop, the scripted provider, the tests); `tests/` (the
  end-to-end golden); `.github/workflows/checks.yml`; `docs/specs.md` §6 and §15;
  `docs/plan.md`'s deferred ledger
- **Builds on:** ADR 0006 §2 (caller-fixable is `valid = false`, operator is `Err`, split once
  in the session) and §6 (tool `inputSchema` generated from the descriptor); ADR 0018 §1 (what
  the view carries, lanes' targets included) and §2 (`get_song` withheld; withholding is the
  lever that works); ADR 0015 §1 (a `ParamRef` naming a track needs no manifest); ADR 0014 §1,
  extended (a `get_manifest` tool was refused because it would hand the AI a plugin
  catalogue); ADR 0019 §1 (every offered call is executed against the proposal); ADR 0020 §3
  (the schemas cross per prompt); ADR 0012 §5 (a golden from a fixed document)
- **Recorded in:** `docs/specs.md` §6 and §15.

## Context

Twenty-five tools are implemented (`core/src/call.rs`, `IMPLEMENTED`). Two are already
decided for the model: `get_song` is withheld (ADR 0018 §2) and `set_param` is withheld
(question 7, the user's decision). `docs/plan.md`'s question 13 asks which of the rest the
model is offered; question 6, narrowed by those two, asks what the model still needs a
`ParamID` for; question 8 asks what the retry loop retries and where "three" lives; question
11 asks how any of it is tested without a model or a key; and question 14 whether the re-pin
tool lands here.

The spike's numbers bear on each (plan, "What the spike found"; one day, cited as evidence).
With fourteen tools offered — the twelve below plus `set_param` and `get_song` — five routes
drove the API at 44/45 or better on the nine feasible instructions. **Eight calls by three
models asked `set_param` for a new track's `gain_db`** and were told `device_unknown`, and all
but one of those runs went on to `apply_patch` `/tracks/<id>/mix/gain_db`: no typed tool sets
a mix, and the models expect one. A single-call edit was **three model turns** — dry run,
apply, answer — which is how a 9,000-token first turn became 28,000 to 39,000 billed prompt
tokens per edit. Gemini returned an **upstream 429** mid-run, "temporarily rate-limited
upstream", which is neither of ADR 0006 §2's two kinds and which the SDK, left at its default,
would have retried twice before the loop knew. And on the one `set_param` instruction two
models guessed ids that did not exist and then set one copied from an automation lane already
in the song, reporting success — a valid call that did the wrong thing, which no validator
sees.

## Decisions

### 1. Twelve tools are offered, `apply_patch` among them; the list is data, and `dry_run` is not in what the model sees

**Offered**, in the order `IMPLEMENTED` advertises them: `apply_patch`, `add_track`,
`set_track_instrument`, `add_effect`, `add_clip`, `set_notes`, `transpose`, `quantize`,
`add_automation`, `set_tempo`, `add_section`, `move_section`. Every one produces ops on the
document and nothing else, and every one is executed against the proposal (ADR 0019 §1).

**Withheld**, thirteen, each for a stated reason: `get_song` (ADR 0018 §2); `get_song_at` and
`get_history`, reads of the log a model acting on the document has no use for, and the log is a
person's to walk; `set_param` (question 7); `add_asset`, which takes bytes the model does not
have; `render_export`, which writes a file at a path the model chose, and `render_preview`,
which starts an engine the model cannot hear (CLAUDE.md #6); `undo` and `redo`, which reverse
a person's approved change; and the four branch tools, which move the session's `HEAD` under
the window — and which ADR 0019 rejected as the proposal mechanism.

**`apply_patch` is offered**, and the reason is the spike's finding above: it is the only way
the model reaches a track's mix, a deletion, a rename, a clip's bounds or a loop length, since
no typed tool does any of those, and three models found it unprompted. Its two hazards are
closed: a caller-written provenance is overwritten by core (ADR 0021 §1), and a version claim
is disputed (ADR 0005 §3). Typed tools for those edits — a `set_mix`, a `remove_*` — are
deferred to the ledger with a trigger that can fire: the first measurement in which a model
gets the RFC 6902 wrong where a typed tool would not have let it. The spike measured no
unparseable patch.

**The list is data.** A constant `OFFERED` beside `IMPLEMENTED` in `core/src/call.rs`, a test
that it is a subset, and the schemas the host hands `ai` per prompt (ADR 0020 §3) are the
descriptor's own (ADR 0006 §6) filtered by it — never a second surface written in Python (plan,
trap 11). What differs by author is what is offered; what is accepted is the same for every
caller (trap 18).

**`dry_run` is removed from the schemas the model sees.** Every offered call is executed
against the proposal and a person approves the whole (ADR 0019 §2), so a dry run is meaningless
to the model and costs it a turn: the spike's three turns per single-call edit become two, and
the tokens per edit fall by the first turn's share. The field stays on the wire and in the
request; a model that sends it anyway, from habit, gets a dry run of the proposal (ADR 0019
§1). The system prompt says once that nothing is applied until a person approves.

### 2. No `list_params`; a `ParamID` reaches the model only as the document carries it

With `set_param` and `get_song` withheld, the one offered tool that takes a `ParamRef` is
`add_automation`, and a `ParamRef` has two targets. Naming a **track** needs `gain_db` or
`pan` and no manifest at all (ADR 0015 §1) — the fader ride, the commonest automation there is,
and every argument for it is in the view. Naming a **device** needs a `ParamID`, and the view
carries every lane's target, device id and `ParamID` (ADR 0018 §1's table), so a lane the
document already has can be automated again and no parameter the document does not already
automate can be named. That is the whole of what the model can do with a device parameter in
M3, and it is stated rather than widened.

So there is **no `list_params` tool and no manifest handed to `ai`**. Question 6's default,
(a), was written when `set_param` was in the offered list; with it withheld, a tool returning
2,855 ids would return ids nothing the model is offered can act on except by automating them —
which is `set_param` over time, and the reason `set_param` is withheld (194 of those ids reach
an RNG nothing can seed; two models set the wrong one and said otherwise) applies to a lane
point unchanged. ADR 0018 §1's reason against showing `params` values is the same reason one
level up. `list_params` is deferred to the ledger with the trigger that makes it useful: **the
first time `set_param` is offered to the model**, which is the denylist landing (question 7's
row, still open). An MCP client is unchanged: it has `get_song` and the display names its own
client shows it.

### 3. Three kinds of failure, told apart once each, with a bounded cost per turn

ADR 0006 §2 draws caller-fixable from operator, in the session, by return type. A hosted model
adds a third kind the ADR did not have to name, and the loop has to know all three or it spends
retries on a wall (plan, trap 2; M1 PR 13's engine that exited 0 having written nothing).

- **A refusal** — `valid = false`, every `Violation` with `path`, `rule` and `message` — goes
  back to the model as the call's result, whole, because "a model fixing one problem at a time
  wastes them" (`ToolResult`). The model retries by calling differently, and the loop counts:
  **three refused calls in one turn** and the turn ends, the person told which calls were
  refused and why. Per turn rather than per call, because a loop cannot tell a retry of one
  call from a new call, and per turn is the tightest of the plan's three bounds — the spike's
  worst runs were 19 and 58 refused guesses on one instruction, which is the shape "three"
  exists to cap, in money.
- **An operator error** — `Err(ProjectError)`, a project that will not write, a lock, a
  `lock_mismatch` — **ends the turn at the host** before the model sees it. The stream is
  closed, the proposal is dropped, and the panel shows the rule and message as the project's,
  because nothing a model says differently would fix it and a model told to retry would spend
  its turn learning that. Zero retries.
- **A provider failure** — a 429, a 5xx, a timeout, a response the SDK cannot parse, and **a
  response whose `prompt_tokens` is below a floor derived from the bytes the loop sent**, which
  is how the spike caught a provider silently discarding a message whose content was JSON — is
  split **in `ai`, at the one place the provider is called**, and nowhere else. It is retried with backoff a
  bounded number of times there, using the SDK's own `max_retries` set by the loop rather than
  left at its default so the count is ours, and then **surfaces as the provider's failure**:
  the stream ends with a status naming it, distinct from a refusal and from a project error,
  and the panel says the provider failed. It is **never fed back to the model** as a
  validation error, since it is not one, and never counted against the three.

A fourth thing is none of the three and is named so it is not mistaken for one: a response
with **neither a call nor text** — the spike's `finish_reason: length` after 8,192 tokens of
reasoning — is the model exhausting its output, not the provider failing, and a retry with
backoff reproduces it. The turn ends, the person is told the model ran out of room, and nothing
is retried or fed back.

Beside the refusal budget, **a cap on model responses per turn** — twelve, the number the spike
ran with and hit only on the instruction nothing offered could satisfy — because a model can
loop on valid calls too. Both numbers are the loop's configuration and are named as numbers, not
rules; what the decision fixes is that each kind is split in exactly one place and that a test
exists per kind, watched failing first (plan, trap 1): a transcript that refuses four times and
a test counting three fed back and one turn ended; a transcript with a provider failure and a
test counting its retries and no feedback; an operator error and a test counting zero retries
and a closed stream.

**§6.1 gains the sentence** that names the third kind, so the spec says what ADR 0006 §2's
two-way split does not: a provider's own failure is neither the caller's to fix by calling
differently nor an operator's project error, and it is neither retried as the first nor
reported as the second.

### 4. Tested by a scripted provider; the project goldened end to end; no key in CI, ever

**The scripted provider** is the `openai` client replaced by a replayer of a recorded
transcript — the provider's responses and the request *bodies* only, never headers (plan,
trap 10) — so a turn is a pure function of the transcript. Two goldens, the shape ADR 0012 §5
gave the frontend one language over:

- In `ai/`, under `unittest` (stdlib; ADR 0016 §3's reason): the **event stream** a transcript
  produces, byte for byte, against a committed file. The loop's decisions — what is fed back,
  what ends a turn, what the view says after each call — are visible there without a host.
- In `tests/`, the determinism suite's shape: a prompt driven through the **real `ai` process**
  with the scripted provider and **`core`'s client** (ADR 0020 §4), the proposal applied, the
  project compared with a committed golden and driven twice (§11's last bullet). "Same
  transcript → same log" is the claim, and it is the one the suite can check without a key.

A **live run** against OpenRouter exists behind an environment variable and is skipped loudly
when the variable is absent, printing why — the device test's shape (M2 trap 13; ADR 0013 §4).
No test spends money by default, and **no fixture contains a key**: a test asserts no file under
`ai/tests/` or `tests/` carries the key's prefix, and the recorder saves nothing that could.
`ai/`'s tests join the `checks` job in **PR 5**, the pull request that creates the directory,
because the path gate already lets an `ai/` change through to a job with no step for it (plan,
trap 17) — and every one of these will be run on one machine until a job on `main` goes green
again (trap 6).

### 5. The re-pin tool is not M3's

Question 14, closed as the plan's default. ADR 0010's Consequences promised a re-pin tool and
M2 built none; the ledger row's trigger is "the first `lock_mismatch` a person meets, which
needs a plugin pin to have moved", and M3 moves no plugin pin. `lock_mismatch` is raised at
**open**, which the bundled loop never does; an MCP client opening a project can meet it, as a
person can, and the answer is ADR 0010 §3's — edit the text — until the trigger fires. The row
stands, and this ADR says so rather than letting a second milestone's silence read as a second
promise (plan, trap 5).

## Alternatives considered

**The list (question 13)**

| Alternative | Rejected because |
|---|---|
| Offer every implemented tool | `render_export` writes where the model says, `render_preview` starts an engine it cannot hear, the branch tools move `HEAD` under the window, and a tool that can only fail spends a turn (`core/AGENTS.md`) |
| Withhold `apply_patch` too | No typed tool sets a mix, deletes, renames or resizes; three models reached for it within the spike's nine instructions, and the forgery it allowed is closed by ADR 0021 §1 |
| Add `set_mix` and `remove_*` now | Six RPCs and six tool functions in PR 3 for edits the raw pipeline already expresses and the spike watched models express correctly; deferred with a trigger rather than built on a guess |
| Keep `dry_run` in the model's schemas | A third turn per edit for a preview the model does not compose and a person never sees, at 28,000 to 39,000 tokens an edit |
| A hand-written tool list in Python | The second surface trap 11 names; the descriptor already describes every tool and `IMPLEMENTED` already orders them |

**Parameters (question 6)**

| Alternative | Rejected because |
|---|---|
| (a) `list_params(device_id)` | Returns 2,855 ids nothing the model is offered can act on except by automating them, which is `set_param` over time and inherits its reason for being withheld |
| (b) The manifest handed to `ai` at launch | The plugin catalogue ADR 0014 §1 refused to hand the AI as a side effect, handed on purpose, for the same non-use |
| Show device `ParamID`s with their display names in the view | The names live in the manifest, which `ai` does not have; and a name beside an id is what let two models report the wrong parameter as the right one |

**Failures (question 8)**

| Alternative | Rejected because |
|---|---|
| Three retries per tool call | A loop cannot tell a retry from a new call without call identity it does not have; per turn is tighter and countable |
| Three per prompt across turns | A follow-up prompt inherits a spent budget; the unit a person acts on is the turn |
| Treat a provider failure as `Err` and stop | A 429 is transient by definition and a retry with backoff usually succeeds; stopping on the first is the spike's Gemini run lost to one |
| Treat a provider failure as `valid = false` | Three refusals spent on a wall that is not the caller's, and the model told its call was wrong when it was not (trap 2) |
| Leave the SDK's `max_retries` at its default | Two retries the loop cannot see or count, which the spike set to zero to be able to see them at all |
| A dollar budget guard in M3 | Needs prices per endpoint and a reservation ledger; the spike's guard was watched refusing before it was trusted and still counted a failed call's reservation as spend. The response cap bounds spend per turn without a price list |

**Tests (question 11)**

| Alternative | Rejected because |
|---|---|
| Mock the SDK's HTTP layer | Replays bytes the SDK then parses, so a fixture carries headers; replaying the SDK's *responses* carries the key nowhere |
| A Rust fake `Assistant` server instead of the real `ai` process | Tests the host half against a description of the sidecar rather than the sidecar; the real process with a scripted provider costs `uv`, which `checks` already installs |
| Skip the live run entirely | A run nobody can ever make is a claim nobody can ever check; behind a variable, skipped loudly, it is a run a person can make |

## Consequences

- **PR 3** (`m3.3-proto-py`) adds no `list_params` and no request field for this ADR; the
  service is ADR 0020 §3's alone.
- **PR 5** (`m3.5-ai-shell`): the scripted provider with one recorded transcript, `ai/`'s
  tests under `unittest`, and their step in `checks.yml`; the no-key assertion from the first
  fixture.
- **PR 7** (`m3.7-unseedable`) is **not needed**: question 7 was decided as (c) by the user and
  the withholding is decision 1's list. The plan's row says so rather than being deleted, so
  the numbering every trap and question cites stays true.
- **PR 8** (`m3.8-loop`): `OFFERED`, the subset test, the filtered schemas without `dry_run`;
  the three kinds split where decision 3 puts them, the refusal budget and the response cap,
  a test per kind watched failing first; the end-to-end golden in `tests/`, driven twice; the
  live run behind its variable.
- `docs/specs.md` §6 names the twelve, the three kinds and where "three" counts; §6.1 gains
  the third kind's sentence.
- `docs/plan.md`'s ledger gains two rows: `list_params`, on the first offer of `set_param`;
  and typed tools for a mix, a deletion, a rename and a clip's bounds, on the first measurement
  in which a model gets the RFC 6902 wrong where a typed tool would not.
- "What M3 will not claim": not that the model can name a plugin parameter the document does
  not already automate; not a dollar cap; and not that the twelve are the right twelve for M4,
  which adds the compilers' tools and decides them then.
