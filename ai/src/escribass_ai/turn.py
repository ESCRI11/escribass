"""The tool-calling loop: one prompt, the model's calls, and the turn's end (ADR 0022 §3).

What this file is, in one sentence: it turns a `Prompt` into a list of chat messages, calls the
provider, and emits an `AssistantEvent` per thing the model did — text, a call, or `done` —
waiting for the host's `CallResult` after every call.

**Four things it deliberately does not do.**

1. It does not **validate**. The generated Pydantic constructors validate, so building a
   request from the model's output would raise before the call and turn §4.4 into something
   implemented twice, in a third language (`docs/plan.md`, M3 trap 12). The arguments cross as
   the JSON text the model wrote and `core` refuses them, with the rule.
2. It does not **apply** anything. Every call lands on a proposal in `core` and a person
   approves the whole, once, when the turn ends (ADR 0019). The system prompt says so, because
   a model that believes it has edited a song will say so to the person.
3. It does not **compose**. `dry_run` is out of the schemas the host sends (ADR 0022 §1), the
   model applies step by step as the spike measured every model doing, and the fork is what
   makes the second call's track exist.
4. It does not **judge**. No axis value is compared with a threshold here, none reaches a
   `Violation`, and none decides whether anything is applied — a person does (ADR 0018 §4).

**What the model reads.** The bar view and the six axes, derived from the `Song` the host sent
and discarded when the turn ends — never held, never edited, never written back, which is
CLAUDE.md #1's rule in a fourth language (M3 trap 7). The view is re-read after every applied
call, from the document the proposal now has (ADR 0018 §2).
"""

from __future__ import annotations

import json
from collections.abc import AsyncIterator
from typing import Any

import grpclib
from escribass_proto.escribass.assistant.v1 import (
    AssistantCommand,
    AssistantEvent,
    Done,
    Prompt,
    ReplyText,
    ToolCall,
)
from escribass_proto.escribass.tools.v1 import ToolResult
from escribass_schema.escribass.song.v1 import Song

from . import axes, view
from .provider import BudgetExhausted, Exhausted, ProviderFailure, ask

__all__ = ["MAX_COMPLETION_TOKENS", "RESPONSES_PER_TURN", "SYSTEM", "document", "run"]

#: How many responses the model gets in one turn before the turn ends.
#:
#: The loop's configuration, named as a number rather than as a rule (ADR 0022 §3). Twelve is
#: what the spike ran with, and it hit the cap on exactly one instruction — the one nothing it
#: was offered could satisfy. A model can loop on *valid* calls too, which the three-refusal
#: budget does not bound, and this is what bounds spend per turn without a price list.
RESPONSES_PER_TURN = 12

#: The most the model may write in one response, and the number a ceiling is priced against.
#:
#: Named here rather than left off the request, because "what this call could cost" has no
#: answer while a completion is unbounded: without it the worst case is the provider's own
#: limit — 131,072 tokens for the default model, $0.055 a call — and a $0.25 ceiling would
#: refuse the fourth call of a turn that really costs a fraction of a cent
#: (`provider._worst_case`). 8,192 is the spike's own ceiling on a reasoning response, which it
#: reached exactly once and on the instruction nothing offered could satisfy; a model that
#: reaches it here produces neither a call nor text, and the turn ends saying so (ADR 0022 §3's
#: fourth thing).
MAX_COMPLETION_TOKENS = 8192

#: Zero, and it is not a determinism claim. A hosted model cannot be seeded, temperature 0 is
#: not determinism, and no test in this repository pretends otherwise (docs/plan.md, "What
#: 'deterministic' means with a model in the loop"). It is here because a lower temperature
#: made fewer wrong-but-valid calls in the spike, which is a different claim.
TEMPERATURE = 0

SYSTEM = """\
You are the assistant inside escribass, a code-defined music editor. You edit one song by \
calling the tools you have been given.

What you read is a **view** of the song, not the song: a header, one block per bar, then the \
automation lanes. It carries every id and every field the tools take. Below it are six \
descriptive axes — rhythm, harmony, melody, texture, form and within-song variation.

Rules that are not negotiable, because the tools enforce them anyway:

- Positions and lengths are **ticks**, 960 to a quarter note. A note's `start_tick` is relative \
to its clip; the view says where each clip's tick 0 sits in the bar, so the arithmetic has both \
numbers on one line. Pitches are MIDI numbers, 0 to 127.
- Use the ids the view gives you, and the ids a tool's result returns. Do not invent an id, and \
do not write `id`, `version` or `provenance` — those are the editor's.
- A call that is refused comes back with every rule it broke. Read them all and call again.

Two things about what you are doing:

- **Nothing you do is applied.** Your calls build a proposal, and a person reads the whole \
change as one diff and applies, rejects or edits it. Say what you changed in the proposal, \
never that you changed the song.
- The axes are **counts over this document**, not judgements. No key, genre or norm is claimed; \
a sampler voice's pitches may not be pitches. Use them to check what you meant against what you \
did. A person decides.

Answer in plain prose when you are done calling tools."""


def document(song: Song) -> str:
    """The view and the six axes, as the model reads them (ADR 0018 §1, §4).

    `ponytail:` both are sent again after every applied call — about 7 KB for the render
    fixture, of which the view is 3.2 KB. That is what ADR 0018 §2's "re-read after every
    applied call" costs when the axes come with it, and the response cap is what bounds it. If
    a measurement ever shows the per-call tokens mattering more than the self-check does, the
    view alone is what a call's result carries and the axes stay on the prompt.
    """
    return "\n\n".join(
        [
            view.view(song),
            axes.rhythm(song),
            axes.harmony(song),
            axes.melody(song),
            axes.texture(song),
            axes.form(song),
            axes.variation(song),
        ]
    )


def answered(result: ToolResult) -> str:
    """What one call's result says to the model.

    A refusal comes back **whole** — every violation, with its path and its rule — because a
    model fixing one problem at a time wastes the three refusals a turn is allowed
    (song_tools.proto, `ToolResult`; ADR 0022 §3).
    """
    if result.valid:
        return f"applied to the proposal: {result.summary}"
    reasons = "\n".join(f"- {v.path} [{v.rule}]: {v.message}" for v in result.errors)
    return f"refused, and nothing changed:\n{reasons}"


def _tools(prompt: Prompt) -> list[dict[str, Any]]:
    """The offered schemas in the shape the SDK takes them.

    `input_schema` is the descriptor's own JSON Schema, filtered by the host and handed to the
    provider **verbatim**: it names proto fields (`start_tick`, never `startTick`), which is
    what `core` decodes, and a schema built from the generated Pydantic model instead would
    teach the model a contract the tool then refuses (M3 trap 11).
    """
    return [
        {
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": json.loads(tool.input_schema),
            },
        }
        for tool in prompt.tools
    ]


def _history(prompt: Prompt) -> list[dict[str, Any]]:
    """Every earlier turn, as the provider's message list.

    The conversation is the host's and arrives whole with every prompt, so `ai` holds nothing
    between streams (ADR 0021 §3). What is rebuilt here is what the model needs to read its own
    past: what was asked, what it called, what each call was answered, and what it replied. The
    **views** of those turns are not rebuilt — they described a document that has since moved,
    and the current one is in the message below them.
    """
    rebuilt: list[dict[str, Any]] = []
    for turn in prompt.conversation:
        rebuilt.append({"role": "user", "content": turn.prompt})
        if turn.calls:
            rebuilt.append(
                {
                    "role": "assistant",
                    "content": None,
                    "tool_calls": [
                        {
                            "id": done.call.call_id,
                            "type": "function",
                            "function": {
                                "name": done.call.name,
                                "arguments": done.call.args_json,
                            },
                        }
                        for done in turn.calls
                        if done.call is not None
                    ],
                }
            )
            for done in turn.calls:
                if done.call is None:
                    continue
                said = answered(done.result) if done.result is not None else "no result"
                rebuilt.append(
                    {"role": "tool", "tool_call_id": done.call.call_id, "content": said}
                )
        if turn.reply:
            rebuilt.append({"role": "assistant", "content": turn.reply})
    return rebuilt


async def run(
    commands: AsyncIterator[AssistantCommand], provider: object
) -> AsyncIterator[AssistantEvent]:
    """One turn, from the prompt to `done` (ADR 0020 §3).

    The stream is the turn: closing it cancels, and every failure ends it with a status rather
    than travelling as an arm a caller may forget to read (ADR 0013 §2).

    Three statuses, and each is one of the things ADR 0022 §3 names:

    - `UNAVAILABLE` — the **provider's** own failure, after `provider.ask` spent its retries.
      Never fed back to the model, because it is not the model's to fix.
    - `RESOURCE_EXHAUSTED` — the **model** exhausted itself: a response with neither a call nor
      text, which is the spike's `finish_reason: length` after 8,192 tokens of reasoning, or a
      model that would not stop calling. A retry with backoff reproduces both, so neither is
      retried.
    - `INVALID_ARGUMENT` / `FAILED_PRECONDITION` — the host or the transcript has a defect,
      or the spend ceiling would have been crossed and the call was never made
      (`provider.BudgetExhausted`; CLAUDE.md #7).

    A refusal is none of those. It goes back to the model as the call's result, and the **host**
    counts them: it is what executes a call, so it is what sees one refused, and it ends the
    turn by closing this stream (ADR 0022 §3).
    """
    command = await anext(commands)
    if command.prompt is None:
        # A `CallResult` first would answer a call nothing made. Refused with a status rather
        # than ignored: the host is the only caller, and a host that sent this has a defect
        # worth seeing.
        raise grpclib.GRPCError(
            grpclib.const.Status.INVALID_ARGUMENT,
            "the first message on a Prompt stream is the prompt (assistant.proto)",
        )
    prompt = command.prompt

    song = prompt.song if prompt.song is not None else Song()
    messages: list[dict[str, Any]] = [{"role": "system", "content": SYSTEM}]
    messages.extend(_history(prompt))
    messages.append({"role": "user", "content": f"{prompt.text}\n\n{document(song)}"})
    tools = _tools(prompt)

    for _ in range(RESPONSES_PER_TURN):
        body: dict[str, Any] = {
            "model": prompt.model_id,
            # A snapshot, not the list: `messages` grows as the turn goes on, and a transcript
            # or a recorder holding the live list would report every request as the last one.
            "messages": list(messages),
            "temperature": TEMPERATURE,
            "max_tokens": MAX_COMPLETION_TOKENS,
        }
        if tools:
            body["tools"] = tools
        try:
            answer = await ask(provider, body)
        except Exhausted as ran_out:
            # This build's own limit rather than any of the three kinds: a transcript with
            # nothing left is a fixture that stopped short, and saying so loudly is what keeps
            # it from reading as a model that stopped.
            raise grpclib.GRPCError(
                grpclib.const.Status.FAILED_PRECONDITION, str(ran_out)
            ) from ran_out
        except BudgetExhausted as unaffordable:
            # None of ADR 0022 §3's three kinds, and not the fourth either: the call was never
            # made. `FAILED_PRECONDITION` is what a transcript running out uses, for the same
            # reason — this build's own limit, which no retry and no different call gets past,
            # and which only a person deciding to spend more can lift (CLAUDE.md #7).
            raise grpclib.GRPCError(
                grpclib.const.Status.FAILED_PRECONDITION, str(unaffordable)
            ) from unaffordable
        except ProviderFailure as failed:
            # Narrow on purpose. `ask` raises exactly this once its retries are spent, and a
            # bare `except Exception` here would report a defect in this file as the provider's
            # failure — the misclassification M3 trap 2 is about, in the direction nobody
            # checks. Anything else leaves as itself and grpclib gives it `UNKNOWN`.
            raise grpclib.GRPCError(
                grpclib.const.Status.UNAVAILABLE, str(failed)
            ) from failed

        choice = answer.choices[0]
        calls = choice.message.tool_calls or []
        if calls:
            messages.append(
                {
                    "role": "assistant",
                    "content": choice.message.content,
                    "tool_calls": [
                        {
                            "id": call.id,
                            "type": "function",
                            "function": {
                                "name": call.function.name,
                                "arguments": call.function.arguments,
                            },
                        }
                        for call in calls
                    ],
                }
            )
            for call in calls:
                yield AssistantEvent(
                    call=ToolCall(
                        call_id=call.id,
                        name=call.function.name,
                        # Verbatim: parsing here would be the second validator M3 trap 12 names,
                        # and the host hands this object to `core::call` as MCP hands it one.
                        args_json=call.function.arguments,
                        # What **answered**, never the id that was asked for: a router may serve
                        # one turn from another provider under the same name (ADR 0021 §2).
                        model_id=answer.model,
                    )
                )
                try:
                    answering = await anext(commands)
                except StopAsyncIteration:
                    # The host closed its half: the turn was cancelled, or its refusal budget
                    # ran out. Either way there is nothing left to answer (ADR 0013 §2).
                    return
                if answering.result is None or answering.result.result is None:
                    raise grpclib.GRPCError(
                        grpclib.const.Status.INVALID_ARGUMENT,
                        "a CallResult carries the tool's result (assistant.proto)",
                    )
                result = answering.result
                said = answered(result.result)
                if result.result.valid and result.song is not None:
                    # Re-read after every applied call, from the document the proposal now has
                    # (ADR 0018 §2, with "applied" meaning applied to the proposal).
                    said = f"{said}\n\n{document(result.song)}"
                messages.append(
                    {"role": "tool", "tool_call_id": result.call_id, "content": said}
                )
            continue

        text = choice.message.content
        if not text:
            raise grpclib.GRPCError(
                grpclib.const.Status.RESOURCE_EXHAUSTED,
                "the model produced neither a call nor text: it ran out of room "
                f"(finish_reason: {choice.finish_reason})",
            )
        # Both, and they carry the same text: this build's SDK call is not streamed, so the
        # reply arrives whole. `ReplyText` is what a panel renders as it goes (PR 9) and `Done`
        # is what ends the turn.
        #
        # `ponytail:` one fragment per turn. Streaming means `stream=True`, a different retry
        # story at `provider.ask` and a transcript of chunks rather than responses; the panel
        # concatenates fragments either way, so nothing above this line changes when it lands.
        yield AssistantEvent(text=ReplyText(text=text))
        yield AssistantEvent(done=Done(text=text, model_id=answer.model))
        return

    raise grpclib.GRPCError(
        grpclib.const.Status.RESOURCE_EXHAUSTED,
        f"the model asked for more than {RESPONSES_PER_TURN} responses in one turn",
    )
