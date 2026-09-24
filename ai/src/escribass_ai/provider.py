"""Where an answer comes from.

Two of them, and only one is reachable in M3 PR 5.

[`Scripted`] replays a **recorded transcript**: the provider's responses and the request
bodies, parsed by the SDK's own `ChatCompletion` model, so a turn is a pure function of a file
and a test can run anywhere, with no key and no money (ADR 0022 §4; question 11). It is the
only provider this pull request wires up — "it answers a prompt with no model", and the loop
that would call a real one is PR 8's.

[`live`] is the four lines that make the real client, and they are here rather than in PR 8
for one reason: the SDK reads **`OPENAI_API_KEY`** from the environment when it is not told a
key, and the spike found that variable set on the machine it ran on (ADR 0020 §4). A loop
written in a hurry against `openai.OpenAI()` would therefore work, quietly, against the wrong
account and the wrong endpoint. So the key and the `base_url` are passed explicitly, here,
once, with a test that says so.

**The key reaches this process from the environment and nowhere else** — never a flag, because
`ps` shows flags; never `lock.json`, because it is committed; never a transcript, because a
transcript is a fixture (U3; `docs/plan.md`, M3 trap 10).
"""

from __future__ import annotations

import json
import os
from pathlib import Path

import openai
from openai.types.chat import ChatCompletion

#: OpenRouter's OpenAI-compatible endpoint — §6's "OpenAI-compatible client", literally, and
#: `lock.baseline.json`'s `ai.provider`. Passed to the SDK explicitly for the same reason the
#: key is: its default is OpenAI's own.
OPENROUTER = "https://openrouter.ai/api/v1"


def live() -> openai.OpenAI:
    """The real client, given its key and its endpoint rather than left to find them.

    Nothing in this pull request calls it: the loop is PR 8's and this process answers from a
    transcript. What it is here for is that the *next* pull request has one place to reach for,
    and that the environment rule has a test before it has a caller.
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
    return openai.OpenAI(api_key=key, base_url=OPENROUTER)


class Exhausted(RuntimeError):
    """The transcript ran out of recorded answers before the turn ran out of questions."""


class Scripted:
    """The provider, replaced by a replayer of one recorded transcript (ADR 0022 §4).

    The file holds a list of exchanges, each a `request` — the **body** that was sent, never
    its headers, which is what keeps `Authorization: Bearer …` out of a fixture for ever
    (`docs/plan.md`, M3 trap 10) — and the `response` the provider returned. The responses are
    parsed by `ChatCompletion` at read time, so a transcript that is not the shape the SDK
    produces fails when the process starts rather than in the middle of a turn, and so that
    what the loop consumes is the same type whether it came from a socket or from a file.

    `ponytail:` it replays in order and does not compare the request it is given with the one
    that was recorded, because this pull request builds no request at all. The upgrade path is
    PR 8's, where a turn sends several: a replayer that matches on the recorded request is what
    keeps a transcript honest once the order alone stops identifying an exchange.
    """

    def __init__(self, exchanges: list[ChatCompletion]) -> None:
        self._remaining = list(exchanges)

    @classmethod
    def read(cls, path: Path) -> "Scripted":
        recorded = json.loads(path.read_text(encoding="utf-8"))
        return cls(
            [ChatCompletion.model_validate(e["response"]) for e in recorded["exchanges"]]
        )

    def answer(self) -> ChatCompletion:
        """The next recorded response, as the SDK would have returned it."""
        if not self._remaining:
            raise Exhausted("the transcript has no answer left to give")
        return self._remaining.pop(0)
