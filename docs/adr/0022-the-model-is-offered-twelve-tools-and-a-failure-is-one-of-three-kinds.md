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

**Amended 2026-09-25 (ADR 0026 §3): fourteen of twenty-seven, from M4 PR 6.** `define_generator`
and `compile_generator` join the list in `IMPLEMENTED`'s order; both produce ops and nothing
else, and both are executed against the proposal — a compile on a proposal spawns the sandbox
and writes the fork's clip. The thirteen withheld are unchanged, with their reasons. The
Consequences' sentence that these twelve were not claimed to be the right twelve for M4 is
answered rather than deleted.

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

  **Extended 2026-09-24, in M3 PR 10: the host closes its half *before* feeding back the
  refusal that ends the turn.** The counting stays in one place, which is this decision's
  point; what changed is the order of two lines. Feeding the third refusal back and closing
  afterwards left an exchange nobody consumes: `turn.run` answers a result and loops straight
  to `ask()`, reaching `anext(commands)` — where it would learn the turn is over — only after
  the provider has already been asked a fourth time. Measured in-process: **four provider
  requests for three refusals**. Against the real sidecar it did not happen, three runs of
  three, because tonic's close won the race — and a turn that costs money must not depend on
  winning a race. Nothing is lost by not sending it: the refusal is in the conversation, so
  the person reads it and the *next* prompt carries it back to the model, which is the only
  point at which the model could still act on it. The alternative — `ai` counting the
  `valid=false` results it has fed back and stopping at the same three — is the same number in
  two files, which this decision refuses.
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

**Extended 2026-09-25 (ADR 0026 §3): a compile's diagnostic is the first kind.** From M4 a
`compile_generator` that fails in the sandbox — the allowlist refusing a node, an exception, a
non-integral tick, the limit — is `valid = false` with the child's `line:column` and text in
the message, fed back whole exactly as a validator refusal is, and **counted against the
three**; a sandbox that will not start, or a toolchain that does not match the project's, is
the second kind and ends the turn at the host. Nothing is split in a new place, and no compile
involves a provider.

**Narrowed 2026-10-01, in M4 PR 6, where the tool was actually offered: a compile's *timeout*
is the second kind, not the first** (ADR 0024 §7, amended, which carries the argument). The
first kind is a failure the caller can fix by calling differently, and a wall clock that ran
out is not one: the child's own CPU and memory limits fire well inside it and arrive as the
diagnostic above, carrying the line, so what reaches the timeout is a wedged child and a
message with nothing in it to act on. Three of those would end a turn having told the model
nothing — this decision's own wall, met from the inside. The two tests are
`a_compile_diagnostic_is_fed_back_whole_and_three_of_them_end_the_turn` and
`a_compile_the_model_cannot_fix_ends_the_turn_at_the_host_and_is_never_fed_back`, which drive
the **same** four calls and differ only in what the sandbox does.

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

**Amended 2026-09-24: the run has been made, and a spend is capped, ledgered and reconciled.**

This section shipped with a gap it named in its own Consequences — every transcript was
hand-written, because recording one costs a paid call and CLAUDE.md #7 gives nobody the right
to make one. The user granted that call on **2026-09-24**, for one recording session and
nothing else, at a ceiling of **$0.25**. What the grant changes here is two things.

**The transcripts are recorded.** `tests/determinism/proposal/transcript.json` is the real
turn: three exchanges against `deepseek/deepseek-v4.1-flash` through OpenRouter, the request
bodies this loop built and the responses the provider gave, written by
`a_live_model_drives_the_loop` through `escribass-ai --record` and committed byte for byte as
the recorder left them. Both goldens are driven from that one file — `ai/`'s event stream and
`tests/`'s project — because two copies of a recording is the twin that stops matching (trap 3).
What stays **hand-written is named**: `rate-limited.json` and `four-refusals.json`, because a
429 and four consecutive refusals cannot be summoned from a real provider on demand, and
`one-answer.json`, which was recorded in PR 5 and is kept for the token floor it trips.

**A paid call is capped before it is made, ledgered after, and reconciled against the
provider's own counter** — CLAUDE.md #7's three mechanisms, all three in `provider.Live`,
which is the only object in this repository that can spend money and which a scripted turn
never constructs:

- **The gate** is `GRANT_USD` and **the budget** is `CEILING_USD`, both committed constants and
  deliberately not environment variables, since a cap an operator raises from the shell is not a
  cap. **Amended 2026-09-24 in M3 PR 10, by the user's decision: the ceiling is $0.00376174 —
  what the grant actually bought — and not $0.25.** The grant was one recording session; it
  produced the transcript; it is spent.

  **Amended again 2026-10-02 in M4 PR 8, by the user's decision, because the ceiling alone was
  fail-closed by *consequence* and not by rule.** The reasoning below — the ceiling equals the
  ledger's total, so any call whose worst case is above zero is refused — is sound and has a
  hole exactly where it says "above zero". A route the catalogue prices at **$0.00** has a worst
  case of exactly zero; `0.00376174 + 0 > 0.00376174` is **false**; and `ai.model` is a text
  field a person edits in a project's `lock.json`, so a `:free` route is a one-word change away.
  A hosted model at $0 is still a call to a metered account, which is what CLAUDE.md #7 is about.
  So `GRANT_USD` is **$0.0** and is read **before the price is even fetched**: at zero no call
  goes out whatever it is priced at, and a grant is a person raising that number in a commit. The
  ceiling keeps its job — is the grant spent? — with `>` corrected to `>=`, because a call whose
  worst case is exactly what is left is a call that could land on the ceiling. Two gates, each
  with a test that watches it refuse: a price can no longer be the only thing between a loop and
  a bill.
  Leaving $0.246 behind a constant is a standing authorisation for whatever runs next, machine-
  wide and all-time, and CLAUDE.md #7 says a previous authorisation does not carry. Any live
  run now fails closed until a person raises the line in code, where a reviewer sees it. The
  `ponytail:` that noted a deleted ledger or a moved `HOME` resets the total is answered by the
  same number rather than by a mechanism: the three recorded calls were *estimated* at
  $0.0045, $0.0048 and $0.0052 each, every one of them above the ceiling on its own, so the
  first call of a turn is refused against an empty ledger exactly as against a full one. The
  enforcement point is the constant; the ledger is a record. It is checked against a
  **conservative worst case** priced from the request about to go out — its bytes floored at
  two per token, plus the whole of the completion it allows — and never against what a call
  turned out to cost, because a guard that reads the receipt has already paid. Prices are data:
  they are fetched from OpenRouter's own `/api/v1/models` and recorded in the ledger with their
  date, and when they cannot be fetched the fallback is the dearest model listed, which at this
  ceiling refuses every call rather than guessing cheaply. The refusal is `BudgetExhausted`,
  raised at `provider.ask` — the same one place §3 puts the third kind — and it is **neither**
  of the three: the provider did nothing wrong, no different call would succeed, and the turn
  ends `FAILED_PRECONDITION` with the three numbers a person needs. Naming a maximum completion
  is what makes "what this call could cost" answerable at all, so the loop now sends
  `max_tokens` (`turn.MAX_COMPLETION_TOKENS`, 8,192); without it the worst case is the
  provider's own limit of 131,072 tokens and the ceiling refuses the fourth call of a turn that
  really costs a fifth of a cent.
- **The ledger** is one JSON line per call in `~/.escribass/spend.jsonl` — estimate, the
  response's own `usage.cost`, the running total, the prices and their date — outside the
  repository, because it records real money and is not a fixture. It is **read back at
  startup**, so the ceiling holds across attempts and across processes; a per-process ceiling
  would let every retry of a recording session spend the whole grant again. A call that failed
  is charged at its worst case, which over-counts on purpose, and the row says so.
- **The reconciliation** is `account_usage` and `generation_cost` against OpenRouter's own
  `/api/v1/key` and `/api/v1/generation`, read either side of a run and compared with the
  ledger. A ledger that only ever agrees with itself is the check that cannot fail which this
  repository has found in every milestone (trap 1). Measured on 2026-09-24: the account counter
  read 47.51704692 before and 47.52080866 after, a difference of **$0.00376174**, against a
  ledger total of **$0.00376174** — and each of the three generations agreed with
  `/api/v1/generation` to nine decimal places. One property worth writing down because a future
  run will meet it: the account figure **lags**. Read within seconds of the run it had not
  moved at all, and it settled minutes later; the per-generation figure was right immediately.

  **Amended the same day, in M3 PR 10: "compared with the ledger" was done by a person and by
  no code path.** The two functions had zero callers anywhere in the repository and the live
  run read neither, so the sentence above described a thing that had happened once rather than
  a thing that happens. They now have one: `escribass-ai --account-usage` and
  `--generation-cost <id>` print one number and exit, building no provider and spending
  nothing, and `a_live_model_drives_the_loop` reads the account counter either side of the
  turn, prints both, and asserts the difference against the rows the run appended — then
  asserts each new row against `/api/v1/generation` by id. **The lag is handled by waiting, and
  the wait is bounded**: the counter is polled for five minutes, every read printed, and a run
  that has not settled by then **fails** rather than passing on a figure that never arrived. An
  assertion that ignored the lag would be flaky; one that waited for ever would be worse; one
  that gave up quietly would be the defect this whole amendment is about. The comparison itself
  is a pure function with a test of its own, so it can go red without a cent being spent.

`ponytail:` one ceiling, one ledger, one reconciliation, and no framework around any of them —
no budget abstraction, no cost model, no price cache. Two known ceilings, both stated in
`provider.py`: the ledger is appended to by one process at a time, and a second sidecar on the
same machine would need a lock around the file; and the byte-per-token floor is an estimate, not
a tokenizer, which is why it over-states.
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
| A dollar budget guard in M3 | ~~Needs prices per endpoint and a reservation ledger; the spike's guard was watched refusing before it was trusted and still counted a failed call's reservation as spend. The response cap bounds spend per turn without a price list~~ — **reversed 2026-09-24, in §4's amendment.** The moment a real call was authorised, the response cap stopped being enough: twelve responses bound the *count* and say nothing about the bill, and CLAUDE.md #7 asks for a cap in code before each call. The two objections held and were answered rather than dodged — the prices are read from the provider's own catalogue instead of written down per endpoint, and a failed call's reservation is still counted as spend, which the reconciliation against the provider's counter now makes visible instead of silent |

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
  live run behind its variable. **Done 2026-09-24.** The numbers decision 3 left as the loop's
  configuration: **three** refused calls a turn, **twelve** responses a turn, **three** provider
  retries at 0.25 s doubling, and a token floor of one token per **twenty** bytes of messages
  sent. Four things the decision did not have to say.

  **The refusal budget is the host's and the response cap is `ai`'s**, because each is counted
  where it is seen: the host executes a call, so it is what sees one refused, and it ends the
  turn by closing the stream (ADR 0013 §2's "cancel is close"). `ai` sees the responses.
  Neither number exists twice.

  **An operator failure is barely reachable inside a turn, and that is a property worth
  stating rather than a gap.** A proposal records nothing and writes nothing, so none of the
  twelve can reach a `Project::write`, a lock or a `head_unset`; the arm is kept and
  documented. What a turn *can* meet is the project refusing to record the model it was sent
  to (ADR 0021 §4), which happens before the prompt goes out — zero retries, nothing fed back,
  and the test for the second kind is exactly that.

  **A provider failure and a transport failure are told apart by whether `ai` returned a
  status.** The host maps `UNAVAILABLE` to `provider_failed` and `RESOURCE_EXHAUSTED` to
  `turn_unfinished` — the fourth thing, a model with neither a call nor text, or one that will
  not stop calling — and everything else, including a socket that broke and a child that died,
  stays `assistant_failed`. A dead child overrides all of it: a provider cannot have failed
  inside a process that is gone.

  ~~**The transcripts are hand-written, and this is where that is said.**~~ **Closed
  2026-09-24, in the same pull request**, by the user's grant of one recording session at a
  ceiling of $0.25. The multi-call transcript is now the recording, the two goldens are driven
  from it, and §4's amendment says what was recorded and what it cost. What the sentence said
  while it stood: every transcript was built from the shape of
  `ai/tests/transcripts/one-answer.json`, the one exchange M3 PR 5 really recorded, with only
  `choices` and `usage.prompt_tokens` changed, because recording a turn costs a paid call and
  `CLAUDE.md` #7 says nothing calls a paid service without the user's confirmation.

  **What the run measured, in one attempt.** `deepseek/deepseek-v4.1-flash`, offered these
  twelve schemas and this system prompt, answered "add a lead line over the bass" with
  `add_track` and then `add_clip` naming `01M1FPMP000000000000000034` — the track id the
  **first call's result returned** — and then wrote its reply: three exchanges, two calls,
  **zero refusals, zero provider failures and no retries**, in 14.6 s for $0.00376174. The
  spike's two live hits of the id trap (2026-09-17, a different loop) did not recur, which is
  what ADR 0019's fork was built to prevent and is now measured rather than argued. One attempt
  is one attempt: it is not a claim about how often a model gets this right, and nothing here
  was run twice to find a nicer one.

  **What a recorded transcript still does not make true.** A hosted model is not seedable and
  the same prompt may return something else tomorrow (`docs/plan.md`, "What 'deterministic'
  means with a model in the loop"); what is deterministic is the *replay*, and that is the
  claim the two goldens check. And `rate-limited.json` and `four-refusals.json` remain
  hand-written, so "a real 429 is retried and never fed back" and "a real model's fourth
  refusal ends the turn" are still constructed cases.

  One measured consequence of the token floor, kept because it is evidence: replaying the
  *recorded* `one-answer.json` through the loop is refused as a provider failure. Its 68 prompt
  tokens are a true record of the spike's 250-byte prompt, and the loop sends nine kilobytes, so
  that response cannot honestly be an answer to it. `ai/tests/test_sidecar.py` asserts it, which
  is the one place a real response exercises the floor.
- `docs/specs.md` §6 names the twelve, the three kinds and where "three" counts; §6.1 gains
  the third kind's sentence.
- `docs/plan.md`'s ledger gains two rows: `list_params`, on the first offer of `set_param`;
  and typed tools for a mix, a deletion, a rename and a clip's bounds, on the first measurement
  in which a model gets the RFC 6902 wrong where a typed tool would not.
- **PR 10** (`m3.10-review-fixes`, 2026-09-24) carries three of this ADR's amendments: the
  ceiling is what the grant bought (§4), the reconciliation is performed by a code path (§4),
  and the host closes before the third refusal goes out (§3). It also closes the two test gaps
  the M3 review's mutation table found in this decision's own claims — no test drove a
  `call_unreadable` or an `arguments_unreadable` refusal through the budget, and nothing
  checked that `ai` counts no refusals of its own.
- "What M3 will not claim": not that the model can name a plugin parameter the document does
  not already automate; ~~not a dollar cap~~ — **amended 2026-09-24: there is one**, a ceiling
  checked in code before each call, ledgered and reconciled (§4's amendment), which M3 declined
  to build while no call was authorised — and not that the twelve are the right twelve for M4,
  which adds the compilers' tools and decides them then. **Decided 2026-09-25: fourteen**, the
  twelve plus `define_generator` and `compile_generator` (ADR 0026 §3; §1 above, amended).
