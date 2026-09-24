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
"""

from __future__ import annotations

import asyncio
import json
import os
from pathlib import Path
from typing import Any

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


async def ask(client: "Scripted | openai.OpenAI", body: dict[str, Any]) -> ChatCompletion:
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


def _once(client: "Scripted | openai.OpenAI", body: dict[str, Any]) -> ChatCompletion:
    if isinstance(client, Scripted):
        return client.answer(body)
    return client.chat.completions.create(**body)


def live() -> openai.OpenAI:
    """The real client, given its key, its endpoint and its retry budget rather than left to
    find them.

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
    return openai.OpenAI(api_key=key, base_url=OPENROUTER, max_retries=PROVIDER_RETRIES)


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
