"""The loop: what it sends, what it feeds back, and how each kind of failure ends a turn.

Two claims, and they need two different shapes of test.

**What a transcript produces is a golden** (ADR 0022 §4). A turn is a pure function of the
transcript once the host's answers are fixed, so the whole of it — every event `ai` emits, and
every request body it built — is written to one file and compared byte for byte. The requests
are in there beside the events on purpose: what is fed back to the model, and what the view
says after every applied call, live in the request and not in the event stream, and a golden
that held only the events would pass for a loop that fed the model nothing.

**Each failure ends a turn its own way**, and ADR 0022 §3 asks for a test per kind *watched
failing first*. The three are here, plus the fourth thing that is none of them:

| What | Ends as | Fed back to the model? | Retried? |
|---|---|---|---|
| A refused call | the call's `ToolResult` | whole, every violation | by the model, three times a turn |
| A provider failure | `UNAVAILABLE` on the stream | never | inside `ask`, with backoff |
| The model out of room, or looping | `RESOURCE_EXHAUSTED` | never | never |

The loop is driven **directly**, as the async generator it is, with a list of commands for a
host: no server, no socket, no subprocess. `test_sidecar.py` is what drives the real process,
and `tests/determinism.rs` is what drives the real process against real `core`.

Bless a deliberate change with:

    UPDATE_FIXTURES=1 uv run python -m unittest discover -s tests

and read the diff, exactly as `tests/AGENTS.md` requires of every other golden here.
"""

from __future__ import annotations

import asyncio
import copy
import json
import os
import pathlib
import unittest
from collections.abc import AsyncIterator

import grpclib
from escribass_ai import provider as provider_module
from escribass_ai.provider import Exhausted, ProviderFailure, Scripted
from escribass_ai.turn import RESPONSES_PER_TURN, run
from escribass_proto.escribass.assistant.v1 import (
    AssistantCommand,
    CallResult,
    CompletedCall,
    Prompt,
    ToolCall,
    ToolSchema,
    Turn,
)
from escribass_proto.escribass.tools.v1 import ToolResult, Violation
from escribass_schema.escribass.song.v1 import (
    DeviceRef,
    Instrument,
    Mix,
    PluginRef,
    Song,
    Track,
    TrackKind,
)
from openai.types.chat import ChatCompletion

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
FIXTURE = ROOT / "tests" / "determinism" / "render" / "expected" / "song.json"
TRANSCRIPTS = HERE / "transcripts"
GOLDEN = HERE / "golden"

#: **The recorded turn**, and it lives in the other tree because there is one of it.
#:
#: `tests/determinism/proposal/transcript.json` is what `a_live_model_drives_the_loop` wrote on
#: 2026-09-24: three real exchanges against `deepseek/deepseek-v4.1-flash` through OpenRouter,
#: $0.00376174, the request bodies without their headers (ADR 0022 §4). Both goldens want the
#: same bytes — this file's event stream and `tests/determinism.rs`'s project — and a second
#: copy under `ai/tests/transcripts/` is the twin that stops matching (M3 trap 3). Reaching
#: across is what this file already does for the song fixture above.
RECORDED_TURN = ROOT / "tests" / "determinism" / "proposal" / "transcript.json"

#: The track id the model really named in the second call, which is the id `core` really minted
#: in the first — the whole of what a fork buys over a dry run (ADR 0019 §1). Written literally,
#: as every fixture id here is: a change in mint order should fail loudly (tests/AGENTS.md).
MINTED = "01M1FPMP000000000000000034"

#: Two of the twelve the host offers, so the golden carries a real schema without carrying
#: 12 KB of them. `core`'s own filter is what builds the real list (`OFFERED`), and
#: `core/tests/proposal.rs` is what checks it; what this file cares about is that whatever
#: arrives is handed to the provider verbatim.
OFFERED = [
    ToolSchema(
        name="add_track",
        description="Adds a track.",
        input_schema=json.dumps(
            {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "kind": {"type": "integer"},
                    "ref": {"type": "object"},
                },
                "additionalProperties": False,
            },
            sort_keys=True,
        ),
    ),
    ToolSchema(
        name="add_clip",
        description="Adds a clip to a track.",
        input_schema=json.dumps(
            {
                "type": "object",
                "properties": {
                    "track_id": {"type": "string"},
                    "start_tick": {"type": "integer"},
                    "length_ticks": {"type": "integer"},
                    "note_clip": {"type": "object"},
                },
                "additionalProperties": False,
            },
            sort_keys=True,
        ),
    ),
]


def fixture_song() -> Song:
    return Song.from_dict(json.loads(FIXTURE.read_text(encoding="utf-8")))


def with_a_lead(song: Song) -> Song:
    """The fixture with the track `add_track` minted, so the golden shows the view being
    re-read after an applied call (ADR 0018 §2).

    The ids are the ones `core` really minted in the recorded run — `…0034` for the track and
    `…0035` for its instrument — because the transcript's second call names `…0034` and a host
    that answered with a different id would be answering a call nobody made. The index is this
    fixture's own: it has five tracks where the recorded project had three, and a Song built to
    match the wrong one is the thing this file is not."""
    grown = Song.from_dict(song.to_dict())
    grown.tracks[MINTED] = Track(
        id=MINTED,
        name="Lead",
        kind=TrackKind.INSTRUMENT,
        index=5,
        mix=Mix(),
        instrument=Instrument(
            id="01M1FPMP000000000000000035",
            ref=DeviceRef(
                plugin=PluginRef(plugin_id="Surge Synth Team/Surge XT", version="1.3.4")
            ),
        ),
    )
    return grown


def applied(summary: str, song: Song) -> ToolResult:
    return ToolResult(valid=True, summary=summary, patch=b"[]", entry_id="")


def refused(rule: str, message: str) -> ToolResult:
    return ToolResult(
        valid=False, errors=[Violation(path="/track_id", rule=rule, message=message)]
    )


async def drive(
    prompt: Prompt, transcript: Scripted, answers: list[tuple[ToolResult, Song | None]]
) -> tuple[list, Scripted]:
    """One turn, with a host that answers each call from `answers` in order."""
    events: list = []
    answering = list(answers)
    seen: list[ToolCall] = []

    async def commands() -> AsyncIterator[AssistantCommand]:
        yield AssistantCommand(prompt=prompt)
        while True:
            # The loop yields a call and then asks for the next command, so by the time this
            # resumes the call it is answering is the last event emitted.
            call = seen[-1]
            result, song = answering.pop(0) if answering else (applied("nothing", None), None)
            yield AssistantCommand(
                result=CallResult(call_id=call.call_id, result=result, song=song)
            )

    async for event in run(commands(), transcript):
        events.append(event)
        if event.call is not None:
            seen.append(event.call)
    return events, transcript


def prompt_for(song: Song, text: str = "add a lead line", conversation=()) -> Prompt:
    return Prompt(
        text=text,
        song=song,
        tools=list(OFFERED),
        model_id="deepseek/deepseek-v4.1-flash",
        conversation=list(conversation),
    )


def transcript(name: str) -> Scripted:
    """A hand-written transcript by name; [`RECORDED_TURN`] is read by path, being the one
    file here that was not written by hand."""
    return Scripted.read(TRANSCRIPTS / name)


def as_text(events: list, asked: list[dict]) -> str:
    """The whole turn as one comparable document: what `ai` emitted, then what it sent."""
    return json.dumps(
        {
            "events": [event.to_dict() for event in events],
            "asked": asked,
        },
        indent=2,
        sort_keys=True,
        ensure_ascii=False,
    ) + "\n"


class TestTheTurnGolden(unittest.TestCase):
    def test_a_two_call_proposal_is_the_committed_bytes(self) -> None:
        # The multi-call turn ADR 0019 exists for, end to end on this side: the model adds a
        # track, reads the view again with the track in it, puts a clip on the id the first
        # call returned, and answers.
        song = fixture_song()
        grown = with_a_lead(song)
        replayer = Scripted.read(RECORDED_TURN)
        events, _ = asyncio.run(
            drive(
                prompt_for(song),
                replayer,
                [
                    (applied("1 op: /tracks/" + MINTED, grown), grown),
                    (applied("1 op: /clips/01M1FPMP000000000000000036", grown), grown),
                ],
            )
        )
        produced = as_text(events, replayer.asked)
        at = GOLDEN / "turn.json"
        if os.environ.get("UPDATE_FIXTURES") == "1":
            at.write_text(produced, encoding="utf-8")
            return
        self.assertEqual(
            produced,
            at.read_text(encoding="utf-8"),
            f"the turn no longer produces what was committed ({at.name})",
        )

    def test_the_golden_carries_the_things_a_single_call_one_would_not(self) -> None:
        """Named claims, so the golden cannot quietly stop covering them.

        A golden of a one-call turn would pass for a loop that never composed anything, never
        re-read the view and never fed a result back. These four assertions are what a
        multi-call one adds, read off the committed file rather than off a fresh run.
        """
        committed = json.loads((GOLDEN / "turn.json").read_text(encoding="utf-8"))
        calls = [event["call"] for event in committed["events"] if "call" in event]
        self.assertEqual(len(calls), 2, "the golden stopped being a multi-call turn")
        # 1. The second call names the id the first call's result returned.
        self.assertIn(MINTED, calls[1]["argsJson"])
        # 2. The result of the first call was fed back, with its summary.
        self.assertEqual(committed["asked"][1]["messages"][-1]["role"], "tool")
        self.assertIn("applied to the proposal", committed["asked"][1]["messages"][-1]["content"])
        # 3. The view was re-read after it: the track is in the document the model reads next,
        #    and was not in the one it read first (ADR 0018 §2).
        self.assertNotIn(MINTED, committed["asked"][0]["messages"][-1]["content"])
        self.assertIn(MINTED, committed["asked"][1]["messages"][-1]["content"])
        # 4. The assistant's own call is in the history it is sent next, so the model sees its
        #    own move. Without it a provider rejects the `tool` message that answers it.
        self.assertEqual(committed["asked"][1]["messages"][-2]["role"], "assistant")


class TestWhatIsFedBack(unittest.TestCase):
    def test_a_refusal_goes_back_whole(self) -> None:
        # Every violation, not the first: a model fixing one problem at a time wastes the three
        # refusals a turn is allowed (ADR 0022 §3).
        song = fixture_song()
        replayer = transcript("four-refusals.json")
        whole = ToolResult(
            valid=False,
            errors=[
                Violation(path="/track_id", rule="track_unknown", message="no such track"),
                Violation(path="/length_ticks", rule="length_positive", message="0 is not"),
            ],
        )
        asyncio.run(drive(prompt_for(song), replayer, [(whole, None)] * 4))
        fed = replayer.asked[1]["messages"][-1]["content"]
        self.assertIn("track_unknown", fed)
        self.assertIn("length_positive", fed)
        self.assertIn("nothing changed", fed)
        # A refusal changed no document, so no view is re-read with it (ADR 0018 §2).
        self.assertNotIn("ticks per quarter", fed)

    def test_the_conversation_is_rebuilt_from_what_the_host_sends(self) -> None:
        # `ai` holds nothing between streams: every earlier turn arrives with the prompt
        # (ADR 0021 §3), and this is what the model reads of it.
        song = fixture_song()
        earlier = Turn(
            prompt="make it louder",
            calls=[
                CompletedCall(
                    call=ToolCall(
                        call_id="old_1",
                        name="add_track",
                        args_json='{"name": "Pad"}',
                        model_id="deepseek/deepseek-v4.1-flash",
                    ),
                    result=applied("1 op: /tracks/x", None),
                )
            ],
            reply="added a Pad",
        )
        replayer = transcript("rate-limited.json")
        asyncio.run(drive(prompt_for(song, conversation=[earlier]), replayer, []))
        roles = [message["role"] for message in replayer.asked[0]["messages"]]
        self.assertEqual(roles[:5], ["system", "user", "assistant", "tool", "assistant"])
        self.assertEqual(replayer.asked[0]["messages"][1]["content"], "make it louder")
        # And the tool message carries **what the call was answered**, not a placeholder: a
        # model reading its own past has to see which of its earlier calls landed and with what
        # summary, which is the id the next call names (ADR 0019 §1). Watched failing first
        # with `_history`'s `answered(done.result)` replaced by `"no result"`.
        self.assertEqual(
            replayer.asked[0]["messages"][3],
            {
                "role": "tool",
                "tool_call_id": "old_1",
                "content": "applied to the proposal: 1 op: /tracks/x",
            },
        )
        self.assertEqual(replayer.asked[0]["messages"][-1]["content"].split("\n")[0], "add a lead line")

    def test_the_schemas_cross_verbatim(self) -> None:
        # The descriptor's own, handed to the provider as they arrived: proto field names, and
        # no second description of the API written here (M3 trap 11).
        song = fixture_song()
        replayer = transcript("rate-limited.json")
        asyncio.run(drive(prompt_for(song), replayer, []))
        offered = replayer.asked[0]["tools"]
        self.assertEqual([tool["function"]["name"] for tool in offered], ["add_track", "add_clip"])
        self.assertIn("start_tick", offered[1]["function"]["parameters"]["properties"])
        self.assertNotIn("dry_run", offered[0]["function"]["parameters"]["properties"])


class TestTheProviderFailure(unittest.TestCase):
    def setUp(self) -> None:
        # The backoff decides when a retry happens and never what is answered, so a test may
        # set it to nothing without changing a single decision (ADR 0022 §3).
        self._backoff = provider_module.PROVIDER_BACKOFF
        provider_module.PROVIDER_BACKOFF = 0.0

    def tearDown(self) -> None:
        provider_module.PROVIDER_BACKOFF = self._backoff

    def test_it_is_retried_and_the_model_is_never_told(self) -> None:
        # The spike's live 429, replayed. One failure, one retry, and the turn answers — with
        # nothing in the model's messages about it, because a provider failure is not a
        # validation error and feeding it back would spend a refusal on a wall (M3 trap 2).
        song = fixture_song()
        replayer = transcript("rate-limited.json")
        events, _ = asyncio.run(drive(prompt_for(song), replayer, []))
        self.assertEqual([event.done.text for event in events if event.done is not None],
                         ["The bass sits at -6 dB across the whole song."])
        # Two requests for one answer: the retry is visible, and it is the same body.
        self.assertEqual(len(replayer.asked), 2)
        self.assertEqual(replayer.asked[0], replayer.asked[1])
        said = json.dumps(replayer.asked[1]["messages"])
        self.assertNotIn("429", said)
        self.assertNotIn("rate-limited", said)

    def test_a_wall_ends_the_turn_as_the_providers_failure(self) -> None:
        # Watched failing first: with the retry loop written as `range(PROVIDER_RETRIES)` this
        # test saw three attempts, not four, and the message named two retries.
        song = fixture_song()
        failing = Scripted(
            [ProviderFailure("429 temporarily rate-limited upstream")]
            * (provider_module.PROVIDER_RETRIES + 1)
        )
        with self.assertRaises(grpclib.GRPCError) as raised:
            asyncio.run(drive(prompt_for(song), failing, []))
        self.assertEqual(raised.exception.status, grpclib.const.Status.UNAVAILABLE)
        self.assertIn("after 3 retries", raised.exception.message)
        self.assertEqual(len(failing.asked), provider_module.PROVIDER_RETRIES + 1)

    def test_a_response_whose_token_count_says_the_prompt_was_dropped_is_one(self) -> None:
        # The failure the SDK cannot see: it parses happily, and the `usage` block says the
        # provider read almost nothing of what was sent (ADR 0022 §3).
        song = fixture_song()
        recorded = json.loads((TRANSCRIPTS / "rate-limited.json").read_text())
        dropped = copy.deepcopy(recorded["exchanges"][1]["response"])
        dropped["usage"]["prompt_tokens"] = 12
        answers = [ChatCompletion.model_validate(dropped)] * (
            provider_module.PROVIDER_RETRIES + 1
        )
        with self.assertRaises(grpclib.GRPCError) as raised:
            asyncio.run(drive(prompt_for(song), Scripted(answers), []))
        self.assertEqual(raised.exception.status, grpclib.const.Status.UNAVAILABLE)
        self.assertIn("dropped most of the prompt", raised.exception.message)

    def test_a_response_that_arrived_whole_is_not_one(self) -> None:
        # The other half, because a floor that fires on an ordinary turn is worse than none:
        # the committed transcripts pass it, which is what `rate-limited.json` answering above
        # already shows, and this says so where a reader will look for it.
        song = fixture_song()
        replayer = Scripted.read(RECORDED_TURN)
        events, _ = asyncio.run(
            drive(prompt_for(song), replayer, [(applied("ok", None), None)] * 2)
        )
        self.assertTrue(any(event.done is not None for event in events))


class TestWhenTheModelExhaustsItself(unittest.TestCase):
    def test_neither_a_call_nor_text_ends_the_turn_and_is_not_retried(self) -> None:
        # The spike's `finish_reason: length` after 8,192 tokens of reasoning. Not the
        # provider failing — a retry with backoff reproduces it — so nothing is retried and
        # nothing is fed back (ADR 0022 §3).
        song = fixture_song()
        recorded = json.loads((TRANSCRIPTS / "rate-limited.json").read_text())
        empty = copy.deepcopy(recorded["exchanges"][1]["response"])
        empty["choices"][0]["message"]["content"] = None
        empty["choices"][0]["finish_reason"] = "length"
        silent = Scripted([ChatCompletion.model_validate(empty)] * 3)
        with self.assertRaises(grpclib.GRPCError) as raised:
            asyncio.run(drive(prompt_for(song), silent, []))
        self.assertEqual(raised.exception.status, grpclib.const.Status.RESOURCE_EXHAUSTED)
        self.assertIn("ran out of room", raised.exception.message)
        self.assertEqual(len(silent.asked), 1, "it was retried")

    def test_a_model_that_will_not_stop_calling_is_capped(self) -> None:
        # A model can loop on *valid* calls too, which the refusal budget does not bound
        # (ADR 0022 §3).
        song = fixture_song()
        recorded = json.loads(RECORDED_TURN.read_text())
        one_call = ChatCompletion.model_validate(recorded["exchanges"][0]["response"])
        forever = Scripted([one_call] * (RESPONSES_PER_TURN + 2))
        with self.assertRaises(grpclib.GRPCError) as raised:
            asyncio.run(
                drive(prompt_for(song), forever, [(applied("ok", None), None)] * 40)
            )
        self.assertEqual(raised.exception.status, grpclib.const.Status.RESOURCE_EXHAUSTED)
        self.assertIn(str(RESPONSES_PER_TURN), raised.exception.message)
        self.assertEqual(len(forever.asked), RESPONSES_PER_TURN)

    def test_a_transcript_that_runs_out_says_so_rather_than_answering(self) -> None:
        # This build's own limit, and none of the three kinds: a fixture that stopped short
        # must not read as a model that stopped.
        song = fixture_song()
        with self.assertRaises(grpclib.GRPCError) as raised:
            asyncio.run(drive(prompt_for(song), Scripted([]), []))
        self.assertEqual(raised.exception.status, grpclib.const.Status.FAILED_PRECONDITION)


class TestTheHostsHalf(unittest.TestCase):
    def test_a_stream_that_does_not_begin_with_a_prompt_is_refused(self) -> None:
        async def wrong() -> AsyncIterator[AssistantCommand]:
            yield AssistantCommand(result=CallResult(call_id="x"))

        async def turn() -> None:
            async for _ in run(wrong(), Scripted([])):
                pass

        with self.assertRaises(grpclib.GRPCError) as raised:
            asyncio.run(turn())
        self.assertEqual(raised.exception.status, grpclib.const.Status.INVALID_ARGUMENT)

    def test_the_sidecar_counts_no_refusals_of_its_own(self) -> None:
        # ADR 0022 §3 puts the refusal budget in **one** place, which is the host: it is what
        # executes a call and therefore what sees one refused, and a second counter here would
        # be the same number in two files. This is what that costs, measured rather than
        # argued — and it is why the host's order matters (see the test below).
        #
        # A host that feeds back all three refusals and *then* closes has already been asked a
        # fourth time: `run` appends the tool message and loops straight to `ask()`, and only
        # then does it reach `anext(commands)` and learn the turn is over. Four requests for
        # three refusals, one of them answered to nobody.
        #
        # If this ever reads three, somebody has put a refusal counter in `ai`. That is a
        # decision ADR 0022 §3 took the other way; take it again before changing this number.
        song = fixture_song()
        replayer = transcript("four-refusals.json")
        asyncio.run(self.closing_after(3, song, replayer))
        self.assertEqual(len(replayer.asked), 4)

    def test_a_host_that_stops_at_its_budget_costs_no_further_request(self) -> None:
        # **The host closes before feeding back the refusal that ended the turn**
        # (`core/src/assistant.rs`, M3 review 2026-09-24). Two results and then the close, and
        # the fourth request is not made — a decision rather than a race won by tonic's
        # teardown, which is what kept a live turn from paying for it before.
        #
        # Watched failing first as the test above: with three results fed back, four.
        song = fixture_song()
        replayer = transcript("four-refusals.json")
        events = asyncio.run(self.closing_after(2, song, replayer))
        self.assertEqual(len(replayer.asked), 3)
        # Three calls went out and the third was never answered, which is the shape the host
        # leaves behind: it executed the call, counted the refusal and closed.
        self.assertEqual(len([event for event in events if event.call is not None]), 3)

    @staticmethod
    async def closing_after(results: int, song: Song, replayer: Scripted) -> list:
        """A host that answers `results` calls with a refusal and then closes its half."""
        seen: list[ToolCall] = []

        async def commands() -> AsyncIterator[AssistantCommand]:
            yield AssistantCommand(prompt=prompt_for(song))
            for _ in range(results):
                yield AssistantCommand(
                    result=CallResult(
                        call_id=seen[-1].call_id,
                        result=refused("track_unknown", "no such track"),
                    )
                )

        events = []
        async for event in run(commands(), replayer):
            events.append(event)
            if event.call is not None:
                seen.append(event.call)
        return events

    def test_a_host_that_closes_mid_turn_ends_it(self) -> None:
        # Three refused calls end the turn at the **host**, and closing the stream is how it
        # says so (ADR 0013 §2). Here the host sends the prompt and nothing else.
        song = fixture_song()

        async def silent() -> AsyncIterator[AssistantCommand]:
            yield AssistantCommand(prompt=prompt_for(song))

        async def turn() -> list:
            return [event async for event in run(silent(), Scripted.read(RECORDED_TURN))]

        events = asyncio.run(turn())
        self.assertEqual(len(events), 1, "the loop carried on without an answer")
        self.assertIsNotNone(events[0].call)


if __name__ == "__main__":
    unittest.main()
