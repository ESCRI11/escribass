"""Where an answer comes from, and the one place a provider failure is told from the other two.

[`Scripted`] replays a **recorded transcript**: the provider's responses and the request
bodies, parsed by the SDK's own `ChatCompletion` model, so a turn is a pure function of a file
and a test can run anywhere, with no key and no money (ADR 0022 §4; question 11).

[`live`] is the four lines that make the real client, and the reason they are explicit rather
than defaulted: the SDK reads **`OPENAI_API_KEY`** from the environment when it is not told a
key, and the spike found that variable set on the machine it ran on (ADR 0020 §4). A loop
written in a hurry against `openai.OpenAI()` would therefore work, quietly, against the wrong
account and the wrong endpoint.

**The key reaches this process from the environment and nowhere else** — never a flag, because
`ps` shows flags; never `lock.json`, because it is committed; never a transcript, because a
transcript is a fixture (U3; `docs/plan.md`, M3 trap 10).

## The third kind of failure, split here and nowhere else

ADR 0006 §2 draws caller-fixable from operator, in `core`'s session, by return type. A hosted
model adds a third kind that neither name fits — a rate limit, a server error, a timeout, a
response the SDK cannot parse, and a response whose token count says the prompt was dropped —
and ADR 0022 §3 puts the whole of it **at the one place the provider is called**, which is
[`ask`]. It is retried with backoff a bounded number of times there and then raised as
[`ProviderFailure`], which the turn ends the stream with. It is **never** fed back to the model
as a validation error, because it is not one, and never counted against the three refusals a
turn is allowed: three retries spent on a wall that is not the caller's is the failure M3
trap 2 names, and the spike met a live 429 mid-run.

Two mechanisms, each bounded by the same number and each handling what it can see. The SDK's
own `max_retries` is set by us rather than left at its default, so the count is ours (ADR 0022
§3); it covers what the SDK classifies — connection errors, 408, 409, 429 and 5xx. [`ask`]
covers what the SDK cannot see, because a response it parsed happily is still a failure if the
provider dropped what we sent. Neither wraps the other: an `openai` error that escapes the SDK
has already been retried our number of times, so it surfaces at once.

## The ceiling, the ledger and the provider's own counter

CLAUDE.md #7 asks for three things around a paid call, and [`Live`] is where all three are,
because it is the only object in this repository that can spend money.

1. **A grant, and a ceiling checked before each call against a conservative worst case.**
   [`GRANT_USD`] is the gate and is read **before a price is even fetched**: at zero no call
   goes out, whatever it is priced at, because a hosted route a catalogue lists at $0 is still
   a call to a metered account (added 2026-10-02, after review found exactly that call going
   out). [`CEILING_USD`] is then the budget — the whole of what one machine may spend —
   [`_worst_case`] prices the request *about to go out* (its bytes floored at
   [`BYTES_PER_TOKEN`], plus the whole of the completion it is allowed), and a call that would
   reach the line raises [`BudgetExhausted`] **without being made**. Never the cost it turned
   out to have: a guard that reads the receipt has already paid. The worst case is priced from
   the *listed* price, fetched from OpenRouter's own catalogue with [`prices`]; when that
   cannot be fetched the fallback is [`DEAREST`], the dearest model on OpenRouter the day this
   was written, so a run with no price list refuses rather than guesses cheaply.
2. **A ledger**, [`SPEND`], one JSON line per call: what it was estimated at, what the
   response's own `usage` says it cost, and the running total. It is read back at startup, so
   the ceiling holds **across attempts and across processes** — a per-process ceiling would let
   every retry of a recording session spend the whole grant again.
3. **Reconciliation against the provider's own counter**: [`account_usage`] reads what
   OpenRouter says this key has spent and [`generation_cost`] what it says one generation cost.
   A ledger that only ever agrees with itself is a check that cannot fail (`docs/plan.md`,
   M3 trap 1), so both are read **by the live run itself** — `escribass-ai --account-usage` and
   `--generation-cost <id>`, two free GETs and no provider object at all — either side of the
   turn, printed, and asserted against the rows the run appended
   (`tests/determinism.rs`, `a_live_model_drives_the_loop`). Until 2026-09-24 that comparison
   was made by hand once and by no code path ever, which is the same defect in the check as in
   the thing checked.

   **The account figure lags.** Seconds after the recorded run it had not moved; minutes later
   it had, by exactly the ledger's total. So the live run *waits* for it, bounded, printing
   each read, and fails if it has not settled inside the bound rather than passing on a figure
   that has not arrived — a reconciliation that gives up quietly is worse than none.

`ponytail:` one grant, one ceiling, one ledger, one reconciliation — no budget framework, no
cost model, no price cache. Both numbers are committed constants rather than environment
variables on purpose: a cap an operator can raise from the shell is not a cap. The upgrade
path, if a machine ever runs two sidecars at once, is a lock around the ledger file; today the
ledger is appended to by one process at a time and read whole at startup.
"""

from __future__ import annotations

import asyncio
import datetime
import json
import os
from pathlib import Path
from typing import Any, NamedTuple

import httpx2
import openai
from openai.types.chat import ChatCompletion

#: OpenRouter's OpenAI-compatible endpoint — §6's "OpenAI-compatible client", literally, and
#: `lock.baseline.json`'s `ai.provider`. Passed to the SDK explicitly for the same reason the
#: key is: its default is OpenAI's own.
OPENROUTER = "https://openrouter.ai/api/v1"

#: How many times a provider failure is retried before it is the provider's failure.
#:
#: Ours, and stated as a number rather than as a rule (ADR 0022 §3). Three, because a 429 is
#: transient by definition and a retry with backoff usually succeeds, and because a turn must
#: not sit behind a wall: at the backoff below the whole budget is under two seconds. The SDK's
#: own default is 2, and leaving it there would mean two retries the loop cannot see or count —
#: the thing the spike set to zero in order to see them at all.
PROVIDER_RETRIES = 3

#: Doubling from here: 0.25 s, 0.5 s, 1 s. It decides **when** a retry happens and never what
#: is answered, which is the rule M1 PR 13 applied to the engine's dispatch loop — a clock may
#: decide failure, never a sample (CLAUDE.md #3, one process over; `ai` is not in #3's list and
#: this is not what would put it there).
PROVIDER_BACKOFF = 0.25

#: The bytes-per-token a prompt is *floored* at when checking what the provider says it read.
#:
#: The spike caught a provider silently discarding a message whose content was JSON, and the
#: symptom was a `prompt_tokens` far below what had been sent (ADR 0022 §3). English runs about
#: four bytes to a token and JSON punctuation runs denser still, so twenty is a floor with a
#: five-fold margin: it cannot fire on a prompt that arrived and cannot miss one that was
#: mostly dropped. A response with no `usage` block is not evidence either way and is let
#: through.
TOKEN_FLOOR_BYTES = 20

#: OpenRouter's model catalogue, which is where a price comes from. Public, unauthenticated
#: and free: reading it is not a paid call, and neither is `/api/v1/key` or `/api/v1/generation`
#: below. What CLAUDE.md #7 is about is `/chat/completions`, and that is reached from exactly
#: one line of this file.
MODELS = "https://openrouter.ai/api/v1/models"

#: **The ceiling, in US dollars, on everything one machine spends** — every call, every retry,
#: every attempt, across processes, because [`SPEND`] is read back at startup.
#:
#: A committed constant and not an environment variable: a ceiling an operator can raise from
#: the shell is not a ceiling, and a change to this number should be a diff somebody reads.
#:
#: **It is what the grant of 2026-09-24 actually bought, to the cent it bought it at.** That
#: grant was one recording session, it produced `tests/determinism/proposal/transcript.json`,
#: and it cost $0.00376174 — so the number here is $0.00376174 and the grant is spent. CLAUDE.md
#: #7 says a previous authorisation does not carry to the next task; a residual $0.246 sitting
#: behind a $0.25 constant *is* that authorisation carrying, machine-wide and all-time, and the
#: next live run would have spent it without anybody being asked. Raising this line is how a
#: person grants the next one, in code, where a reviewer sees it (the user's decision,
#: 2026-09-24).
#:
#: **Deleting the ledger, or moving `HOME`, does not buy a call back.** The ledger is a record,
#: not the enforcement point — the enforcement point is [`GRANT_USD`], and no file a program can
#: delete is between it and a call.
CEILING_USD = 0.00376174

#: **What a person has authorised this machine to spend, in US dollars. It is zero.**
#:
#: The gate, where [`CEILING_USD`] is the budget: at zero **no call goes out, whatever it is
#: priced at**, and raising this line in a commit is what a grant is (CLAUDE.md #7, which says a
#: previous authorisation does not carry to the next task).
#:
#: **Why a constant and not the arithmetic.** Until 2026-10-02 fail-closed was a *consequence*
#: of the numbers: `CEILING_USD` equalled the ledger's total, every priced call's worst case was
#: above it on its own, and so every call was refused. Review found what that leaves out. A
#: route the catalogue prices at $0 — a `:free` one, and `ai.model` is a text field a person
#: edits in a project's `lock.json` — has a worst case of exactly $0.00, and
#: `0.00376174 + 0 > 0.00376174` is **false**: it went out. A hosted model at $0 is still a call
#: to a metered account, which is what CLAUDE.md #7 is about, so the price can no longer be what
#: decides. Two gates now, in this order, and each has a test that watches it refuse
#: (`ai/tests/test_sidecar.py`): this one, which asks whether anything at all was granted, and
#: the ceiling, which asks whether the grant is spent (the user's decision, 2026-10-02).
GRANT_USD = 0.0

#: Where the ledger is written: outside the repository, because it is a record of real money
#: and not a fixture, and in one fixed place, because the ceiling is only a ceiling if every
#: run counts against the same total.
SPEND = Path.home() / ".escribass" / "spend.jsonl"

#: Bytes per token when pricing a request that has not been sent yet — a **floor**, where
#: [`TOKEN_FLOOR_BYTES`] is the opposite kind of bound for the opposite purpose.
#:
#: English runs about four bytes to a token and the tool schemas run denser, so two halves the
#: real ratio and over-states the prompt roughly twofold. Over-stating is the only safe
#: direction here: an estimate that ran under the truth would let a call out that crosses the
#: ceiling. No tokenizer is imported for this — that is a dependency (CLAUDE.md #4) for a number
#: whose whole job is to be too big.
BYTES_PER_TOKEN = 2


class Prices(NamedTuple):
    """What a call costs per token, and where and when that was read."""

    prompt: float
    completion: float
    #: The most the provider will let one completion run to, used when the request names no
    #: `max_tokens` of its own.
    max_completion: int
    on: str
    source: str


#: The prices used when the catalogue cannot be read: `openai/o1-pro`, the dearest model
#: OpenRouter listed on 2026-09-24, at $150 and $600 per million tokens.
#:
#: "Conservative" has a direction. A run that cannot price itself must assume it is talking to
#: the most expensive thing on the network, which at [`CEILING_USD`] refuses every call — the
#: right answer, said out loud in the refusal, rather than a cheap guess that spends.
DEAREST = Prices(0.00015, 0.0006, 131072, "2026-09-24", "openai/o1-pro, the dearest listed")


class BudgetExhausted(RuntimeError):
    """The call was not made, because making it could have crossed [`CEILING_USD`].

    **Not** a [`ProviderFailure`]: the provider did nothing wrong and a retry would not help.
    It is an operator's failure in ADR 0006 §2's sense — nothing a model or a caller says
    differently would fix it — and it is raised at the one place a call goes out, which is the
    same place ADR 0022 §3 splits the third kind. The turn ends with `FAILED_PRECONDITION`,
    carrying the three numbers a person needs: spent, worst case, ceiling.
    """


class ProviderFailure(RuntimeError):
    """The provider's own failure: not the caller's to fix, and not the project's.

    Raised by [`ask`] once its retries are spent, and by [`Scripted`] for a transcript that
    records one. The turn ends the stream with it; nothing of it reaches the model.
    """


def _dropped_the_prompt(answer: ChatCompletion, sent: int) -> str | None:
    """Whether the provider's own token count says it never read what we sent."""
    usage = getattr(answer, "usage", None)
    if usage is None or usage.prompt_tokens is None:
        return None
    floor = sent // TOKEN_FLOOR_BYTES
    if usage.prompt_tokens >= floor:
        return None
    return (
        f"the provider read {usage.prompt_tokens} prompt tokens of the {sent} bytes sent, "
        f"below a floor of {floor}: it dropped most of the prompt"
    )


async def ask(client: "Scripted | Live", body: dict[str, Any]) -> ChatCompletion:
    """The one place the provider is called, and the one place its failures are told apart.

    `body` is the request as the SDK takes it — messages, model, tools, temperature — and is
    also what a transcript records, **without headers**, which is what keeps
    `Authorization: Bearer …` out of a fixture for ever (M3 trap 10).

    On a thread of its own because the `openai` client is synchronous and this runs inside the
    server's event loop: a blocking call there would stall the stream, the stdin watch and the
    signal handler at once.
    """
    sent = len(json.dumps(body.get("messages", []), sort_keys=True).encode("utf-8"))
    for attempt in range(PROVIDER_RETRIES + 1):
        try:
            answer = await asyncio.to_thread(_once, client, body)
            if (dropped := _dropped_the_prompt(answer, sent)) is not None:
                raise ProviderFailure(dropped)
            return answer
        except ProviderFailure as failed:
            if attempt == PROVIDER_RETRIES:
                raise ProviderFailure(
                    f"the provider failed after {PROVIDER_RETRIES} retries: {failed}"
                ) from failed
            await asyncio.sleep(PROVIDER_BACKOFF * 2**attempt)
        except openai.OpenAIError as failed:
            # The SDK has already retried what it can classify, PROVIDER_RETRIES times,
            # because that is what `live` sets. Retrying it again here would be a budget of
            # retries multiplied by a budget of retries.
            raise ProviderFailure(f"the provider failed: {failed}") from failed
    raise AssertionError("the loop above either returns or raises")


def _once(client: "Scripted | Live", body: dict[str, Any]) -> ChatCompletion:
    if isinstance(client, Scripted):
        return client.answer(body)
    # `Live.call` is where the ceiling is checked and the ledger written, which is why the
    # bare `openai` client is never handed to this function: a provider that can spend money
    # in this process is a `Live`, and there is one place that builds one.
    return client.call(body)


def prices(model: str) -> Prices:
    """What OpenRouter says `model` costs today, or [`DEAREST`] if it will not say.

    A price is data, not a guess, so it is read from the catalogue rather than written down
    here — and read *late*, at the first call and not when the client is built, so that
    constructing a provider touches no network and `ai/tests/` stays offline.
    """
    try:
        catalogue = httpx2.get(MODELS, timeout=20).raise_for_status().json()["data"]
        listed = next(one for one in catalogue if one["id"] == model)
        return Prices(
            prompt=float(listed["pricing"]["prompt"]),
            completion=float(listed["pricing"]["completion"]),
            max_completion=int(
                listed.get("top_provider", {}).get("max_completion_tokens")
                or listed["context_length"]
            ),
            on=datetime.date.today().isoformat(),
            source=MODELS,
        )
    except Exception:  # noqa: BLE001 — no network, no such model, a shape that moved
        return DEAREST


def _worst_case(body: dict[str, Any], priced: Prices) -> float:
    """The most this request could cost: every byte of it, plus the whole completion it allows.

    Not what it will cost and not what it did cost — the number a ceiling has to be checked
    against is the one that cannot be exceeded by what comes back.
    """
    sent = len(json.dumps(body, default=str).encode("utf-8"))
    allowed = int(body.get("max_tokens") or priced.max_completion)
    return sent / BYTES_PER_TOKEN * priced.prompt + allowed * priced.completion


def _spent_so_far(ledger: Path) -> float:
    """The running total, read off the ledger, so a second attempt inherits the first's spend."""
    if not ledger.exists():
        return 0.0
    return sum(
        json.loads(line)["charged_usd"]
        for line in ledger.read_text(encoding="utf-8").splitlines()
        if line.strip()
    )


class Live:
    """The real provider: the client, the ceiling it is held to, and the ledger it writes.

    Built by [`live`] and used by [`ask`] exactly as [`Scripted`] is, which is what keeps the
    ceiling on the *one* path that can spend: a turn driven from a transcript never constructs
    this class and can therefore never reach a price, a ledger or a paid call.
    """

    def __init__(
        self,
        client: openai.OpenAI,
        ledger: Path = SPEND,
        record_to: Path | None = None,
        priced: Prices | None = None,
    ) -> None:
        self.client = client
        self.ledger = ledger
        self.record_to = record_to
        self._priced = priced
        self._spent = _spent_so_far(ledger)
        self._exchanges: list[dict[str, Any]] = []

    def call(self, body: dict[str, Any]) -> ChatCompletion:
        """One paid call, capped before it goes out and ledgered whichever way it ends."""
        # **Before the price, and whatever the price is** (CLAUDE.md #7; see [`GRANT_USD`]). A
        # route listed at $0 is still a call to a metered account, and a catalogue that lies
        # about a price, or cannot be read at all, must not be able to buy one either.
        if GRANT_USD <= 0.0:
            raise BudgetExhausted(
                f"refusing to spend: GRANT_USD is ${GRANT_USD:.8f}, so no call goes out, "
                f"whatever it is priced at — a route a provider lists at $0 is still a call "
                f"to a metered account (CLAUDE.md #7). A grant is a person raising GRANT_USD "
                f"in a commit, and the ceiling with it"
            )
        priced = self._priced = self._priced or prices(body["model"])
        worst = _worst_case(body, priced)
        # `>=` and not `>`: a call whose worst case is exactly what is left is a call that could
        # land on the ceiling, and a ledger already *at* the ceiling has nothing left to spend
        # even on a call priced at nothing (found by review, 2026-10-02 — mutation D5).
        if self._spent + worst >= CEILING_USD:
            raise BudgetExhausted(
                f"refusing to spend: ${self._spent:.6f} is already on the ledger and this "
                f"call could cost ${worst:.6f}, which reaches the ceiling of "
                f"${CEILING_USD:.8f} (priced at ${priced.prompt}/${priced.completion} per "
                f"token from {priced.source}, {priced.on}; ledger {self.ledger})"
            )
        try:
            answer = self.client.chat.completions.create(**body)
        except BaseException as failed:
            # A call that failed may still have been billed, and nothing here can tell. It is
            # charged at its worst case, which over-counts on purpose — the only safe direction
            # for a ceiling — and the row says so, so reconciliation explains the gap rather
            # than hiding it (ADR 0022 §4's alternatives table, on the spike's guard).
            self._charge(priced, worst, None, repr(failed))
            raise
        self._charge(priced, worst, answer, None)
        if self.record_to is not None:
            self._record(body, answer)
        return answer

    def _charge(
        self,
        priced: Prices,
        worst: float,
        answer: ChatCompletion | None,
        failed: str | None,
    ) -> None:
        usage = getattr(answer, "usage", None)
        cost = getattr(usage, "cost", None)
        cost = float(cost) if cost is not None else None
        charged = cost if cost is not None else worst
        self._spent += charged
        row = {
            # Wall-clock, deliberately: this is a record of money and a person reads it by
            # date. CLAUDE.md #3's list is `core`, `compilers` and `engine`; `ai` is not on it
            # and no answer depends on this.
            "at": datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds"),
            "model": answer.model if answer is not None else None,
            "generation_id": answer.id if answer is not None else None,
            "estimated_usd": round(worst, 8),
            "cost_usd": cost,
            "charged_usd": round(charged, 8),
            "running_total_usd": round(self._spent, 8),
            "ceiling_usd": CEILING_USD,
            "prompt_tokens": getattr(usage, "prompt_tokens", None),
            "completion_tokens": getattr(usage, "completion_tokens", None),
            "priced_at": {
                "prompt": priced.prompt,
                "completion": priced.completion,
                "on": priced.on,
                "source": priced.source,
            },
            "failed": failed,
        }
        self.ledger.parent.mkdir(parents=True, exist_ok=True)
        with self.ledger.open("a", encoding="utf-8") as writing:
            writing.write(json.dumps(row, sort_keys=True) + "\n")

    def _record(self, body: dict[str, Any], answer: ChatCompletion) -> None:
        """The transcript, in [`Scripted.read`]'s own format, rewritten after every exchange.

        **The body and nothing else.** No client, no headers, no key: what is appended here is
        the same dictionary that was handed to the SDK, and the SDK is what adds
        `Authorization: Bearer …` on its way out (M3 trap 10). Rewritten whole rather than
        appended to because a JSON array cannot be appended to, and a run that dies mid-turn
        then leaves a file that still parses.
        """
        assert self.record_to is not None
        self._exchanges.append({"request": body, "response": answer.model_dump(mode="json")})
        recording = {
            "recorded": datetime.date.today().isoformat(),
            "model": body["model"],
            "note": (
                f"Recorded live against OpenRouter by `a_live_model_drives_the_loop` — the "
                f"request bodies this loop built and the responses the provider gave, with no "
                f"headers, so no key can ever be in here (ADR 0022 §4; docs/plan.md, M3 trap "
                f"10). {len(self._exchanges)} exchange(s); ${self._spent:.8f} on the ledger "
                f"after them, against a ceiling of ${CEILING_USD:.8f}."
            ),
            "exchanges": self._exchanges,
        }
        self.record_to.write_text(
            json.dumps(recording, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )


def live(record_to: Path | None = None) -> Live:
    """The real client, given its key, its endpoint and its retry budget rather than left to
    find them, wrapped in the ceiling and the ledger it may not spend without.

    `max_retries` is ours rather than the SDK's default of 2, so the count of what the SDK
    retries is a number this repository chose and a test can name (ADR 0022 §3).
    """
    key = os.environ.get("OPENROUTER_API_KEY")
    if not key:
        # An operator's error, in ADR 0006 §2's sense: nothing a model or a caller says
        # differently would fix it, so it is raised where the process starts rather than
        # turned into a refusal a loop would retry.
        raise RuntimeError(
            "OPENROUTER_API_KEY is not set: the key reaches `ai` from the environment and "
            "nowhere else (ADR 0020 §4)"
        )
    # `api_key` and `base_url` both explicit. Omitting either is the defect this function
    # exists to prevent: the SDK falls back to OPENAI_API_KEY and to api.openai.com, and a
    # machine that has both set answers happily from the wrong place.
    return Live(
        openai.OpenAI(api_key=key, base_url=OPENROUTER, max_retries=PROVIDER_RETRIES),
        record_to=record_to,
    )


def _counter(path: str) -> Any:
    """One authenticated GET against OpenRouter's own metering. Free; nothing here is billed."""
    key = os.environ.get("OPENROUTER_API_KEY")
    if not key:
        raise RuntimeError("OPENROUTER_API_KEY is not set")
    answer = httpx2.get(
        f"{OPENROUTER}{path}", headers={"Authorization": f"Bearer {key}"}, timeout=20
    )
    return answer.raise_for_status().json()["data"]


def account_usage() -> float:
    """What **OpenRouter** says this key has spent, all time (`GET /api/v1/key`).

    The other half of the reconciliation: read either side of a run, its difference is what the
    provider charged, and comparing that with the ledger's own total is the only way the ledger
    can be wrong out loud rather than agree with itself for ever.
    """
    return float(_counter("/key")["usage"])


def generation_cost(generation_id: str) -> float:
    """What OpenRouter says one generation cost (`GET /api/v1/generation?id=…`).

    Per-call truth, for the rows whose response carried no `usage.cost` of its own.
    """
    return float(_counter(f"/generation?id={generation_id}")["total_cost"])


class Exhausted(RuntimeError):
    """The transcript ran out of recorded answers before the turn ran out of questions."""


class Scripted:
    """The provider, replaced by a replayer of one recorded transcript (ADR 0022 §4).

    The file holds a list of exchanges, each carrying the `request` — the **body** that was
    sent, never its headers, which is what keeps `Authorization: Bearer …` out of a fixture for
    ever (`docs/plan.md`, M3 trap 10) — and then one of two things:

    - a `response`, the provider's own, parsed by `ChatCompletion` **at read time**, so a
      transcript that is not the shape the SDK produces fails when the process starts rather
      than in the middle of a turn, and so what the loop consumes is the same type whether it
      came from a socket or from a file;
    - a `failure`, which is how a transcript records the provider failing — a 429, a timeout, a
      response the SDK could not parse. It is what makes ADR 0022 §3's third kind testable
      without a network, a key or a wall to wait behind.

    `ponytail:` it replays **in order** and does not match the request it is given against the
    one that was recorded. PR 5 named matching as the upgrade path for when order stops
    identifying an exchange; a turn's requests are still one per response and in order, and
    what keeps the transcript honest is stronger than a match — [`asked`] records every body
    the loop built and `ai/tests/golden/` compares them byte for byte, so a request that drifts
    fails with the diff in front of the reader rather than with "no exchange matched".
    """

    def __init__(self, exchanges: list[ChatCompletion | ProviderFailure]) -> None:
        self._remaining = list(exchanges)
        self._asked: list[dict[str, Any]] = []

    @classmethod
    def read(cls, path: Path) -> "Scripted":
        recorded = json.loads(path.read_text(encoding="utf-8"))
        exchanges: list[ChatCompletion | ProviderFailure] = []
        for exchange in recorded["exchanges"]:
            if "failure" in exchange:
                exchanges.append(ProviderFailure(exchange["failure"]["message"]))
            else:
                exchanges.append(ChatCompletion.model_validate(exchange["response"]))
        return cls(exchanges)

    @property
    def asked(self) -> list[dict[str, Any]]:
        """Every request body the loop built, in order. What the event-stream golden compares
        beside the events, because what is fed back to the model lives here and not there."""
        return self._asked

    def answer(self, body: dict[str, Any] | None = None) -> ChatCompletion:
        """The next recorded response, as the SDK would have returned it."""
        if body is not None:
            self._asked.append(body)
        if not self._remaining:
            raise Exhausted("the transcript has no answer left to give")
        answered = self._remaining.pop(0)
        if isinstance(answered, ProviderFailure):
            raise answered
        return answered
