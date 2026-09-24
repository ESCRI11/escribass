"""What the sidecar promises: one line on stdout, one answer per prompt, no key anywhere.

Run from `ai/`:

    uv run python -m unittest discover -s tests

The process is driven as a **real subprocess over a real socket**, which is where this layer's
failures live — `core/tests/mcp.rs` made the same choice for the same reason. What a Python
unit test of the handler would prove is that a generator yields two values; what a person needs
to know is whether the thing `core` spawns prints an address, answers on it, and leaves with a
status when its stdin closes (ADR 0020 §4, §5).

`unittest`, because it is the standard library's and this repository adds no test framework
where one is not needed (ADR 0016 §3, one language over).
"""

from __future__ import annotations

import asyncio
import contextlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from escribass_ai import provider as provider_module
from escribass_ai.provider import (
    CEILING_USD,
    BudgetExhausted,
    Exhausted,
    Live,
    Prices,
    Scripted,
    live,
)
from escribass_proto.escribass.assistant.v1 import (
    AssistantCommand,
    AssistantEvent,
    CallResult,
    Prompt,
)
from escribass_proto.escribass.tools.v1 import ToolResult
from escribass_schema.escribass.song.v1 import Song
from openai.types.chat import ChatCompletion
from grpclib.client import Channel
from grpclib.const import Cardinality

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
#: The one **recorded** exchange (M3 PR 5), read by the provider tests below.
RECORDED = HERE / "transcripts" / "one-answer.json"
#: What the process test drives: the **recorded** two-call turn of 2026-09-24, so what
#: crosses the socket is the loop replaying what a real model really returned (ADR 0022 §4).
#: It lives in the other fixture tree because `tests/determinism.rs` drives the same bytes.
TRANSCRIPT = ROOT / "tests" / "determinism" / "proposal" / "transcript.json"

# What `core` spawns, as this environment has it: `uv run --project ai escribass-ai` resolves
# to the console script beside the interpreter running these tests (ADR 0020 §4).
SIDECAR = Path(sys.executable).with_name("escribass-ai")

PROMPT = "/escribass.assistant.v1.Assistant/Prompt"


def recorded() -> dict:
    return json.loads(RECORDED.read_text(encoding="utf-8"))


class Started:
    """The sidecar, started as `core` starts it: one line read, then the socket it named."""

    def __init__(self, *arguments: str) -> None:
        self.child = subprocess.Popen(
            [str(SIDECAR), *arguments],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    def __enter__(self) -> "Started":
        return self

    def __exit__(self, *_: object) -> None:
        if self.child.poll() is None:
            self.child.kill()
        for pipe in (self.child.stdin, self.child.stdout, self.child.stderr):
            if pipe is not None and not pipe.closed:
                pipe.close()
        self.child.wait(timeout=30)

    def address(self) -> str:
        assert self.child.stdout is not None
        return self.child.stdout.readline().strip()

    def stop(self) -> tuple[int, str]:
        """Closes its stdin, which is how the host says the session is over."""
        assert self.child.stdin is not None
        self.child.stdin.close()
        return self.child.wait(timeout=30), self.child.stderr.read() if self.child.stderr else ""


async def one_prompt(socket: str, text: str) -> list[AssistantEvent]:
    """Dials the socket and drives one turn, as `core::assistant` does in Rust.

    The host half in miniature: the prompt goes out, every call comes back, and each is
    answered on the same stream before the next event arrives. What it does **not** do is
    execute anything — the real host runs each call against a proposal in `core`
    (`core/src/assistant.rs`), and that meeting is `tests/determinism.rs`'s.
    """
    async with Channel(path=socket) as channel:
        async with channel.request(
            PROMPT, Cardinality.STREAM_STREAM, AssistantCommand, AssistantEvent
        ) as stream:
            await stream.send_message(
                AssistantCommand(
                    prompt=Prompt(
                        text=text,
                        song=Song(id="01ARZ3NDEKTSV4RRFFQ69G5FAV"),
                        model_id="deepseek/deepseek-v4.1-flash",
                    )
                )
            )
            events = []
            try:
                async for event in stream:
                    events.append(event)
                    if event.call is not None:
                        await stream.send_message(
                            AssistantCommand(
                                result=CallResult(
                                    call_id=event.call.call_id,
                                    result=ToolResult(valid=True, summary="1 op", patch=b"[]"),
                                )
                            )
                        )
            finally:
                # Half-closing is what the host does by letting go of its half (ADR 0013 §2);
                # grpclib asks for it explicitly, and a stream that failed is already closed.
                with contextlib.suppress(Exception):
                    await stream.end()
            return events


class TestTheProcess(unittest.TestCase):
    def test_it_prints_one_line_naming_its_socket_and_answers_on_it(self) -> None:
        with Started("--transcript", str(TRANSCRIPT)) as sidecar:
            address = sidecar.address()
            self.assertTrue(address.startswith("unix:"), address)
            socket = address.removeprefix("unix:")
            # The line is the readiness as well as the address: nothing sleeps, polls or
            # retries here, and a connection refused would be this assertion failing.
            self.assertTrue(Path(socket).is_socket(), f"{socket} is not a socket")

            events = asyncio.run(one_prompt(socket, "add a lead line"))

            transcript = json.loads(TRANSCRIPT.read_text(encoding="utf-8"))
            last = transcript["exchanges"][-1]["response"]
            said = last["choices"][0]["message"]["content"]
            # Two calls and then the answer, over a real socket: the loop, not a canned reply.
            self.assertEqual(
                [e.call.name for e in events if e.call], ["add_track", "add_clip"]
            )
            self.assertEqual([e.text.text for e in events if e.text], [said])
            done = [e.done for e in events if e.done]
            self.assertEqual([d.text for d in done], [said])
            # The model the response **named**, which is what the log records — not the id the
            # prompt asked for (ADR 0021 §2).
            self.assertEqual([d.model_id for d in done], [last["model"]])

            status, said_on_stderr = sidecar.stop()
            self.assertEqual(status, 0, said_on_stderr)
            # **Stdout carries the address and nothing else.** grpclib and the openai SDK both
            # report through `logging`, whose default handler writes to stderr, and this is
            # what checks that rather than trusting it: a library that printed would land here
            # (`escribass-mcp` keeps the same discipline for its JSON-RPC stream, §18.2).
            assert sidecar.child.stdout is not None
            self.assertEqual(sidecar.child.stdout.read(), "")

    def test_a_transcript_that_is_not_one_stops_it_before_it_serves(self) -> None:
        with Started("--transcript", str(HERE / "no-such-transcript.json")) as sidecar:
            status = sidecar.child.wait(timeout=30)
            out, err = sidecar.child.communicate()
            # An exit status and a sentence on stderr is the whole of what `core` reads from a
            # child that will not start — so nothing may go to stdout, where an address goes.
            self.assertEqual(status, 2)
            self.assertEqual(out, "")
            self.assertIn("no-such-transcript.json", err)


class TestTheScriptedProvider(unittest.TestCase):
    def test_it_replays_in_order_and_then_says_it_has_nothing_left(self) -> None:
        provider = Scripted.read(RECORDED)
        first = provider.answer()
        self.assertEqual(first.model, recorded()["exchanges"][0]["response"]["model"])
        with self.assertRaises(Exhausted):
            provider.answer()

    def test_the_recorded_exchange_trips_the_token_floor_the_loop_added(self) -> None:
        """The floor, measured against a real response rather than a constructed one.

        `one-answer.json` is a true record: 68 prompt tokens for the 250-byte prompt the spike
        sent with no tools and no view. The loop sends a system prompt, the bar view and the six
        axes — nine kilobytes and rising — so that response cannot honestly be an answer to it,
        and `provider.ask` says so (ADR 0022 §3). It is the same check that catches a provider
        silently discarding a message, and this is the one place a real response exercises it.
        """
        with Started("--transcript", str(RECORDED)) as sidecar:
            socket = sidecar.address().removeprefix("unix:")
            with self.assertRaises(Exception) as raised:
                asyncio.run(one_prompt(socket, "What can you change about this song?"))
            # It is retried its bounded number of times and then the transcript runs out,
            # which is this fixture's own limit rather than any of the three kinds.
            self.assertIn("no answer left", str(raised.exception))
            self.assertEqual(sidecar.stop()[0], 0)

    def test_a_response_the_sdk_cannot_parse_fails_when_the_file_is_read(self) -> None:
        # At read time, so the process refuses to start rather than failing mid-turn — and so
        # that what the loop consumes is the SDK's own type either way (ADR 0022 §4).
        broken = HERE / "broken.json"
        broken.write_text(json.dumps({"exchanges": [{"response": {"id": "x"}}]}))
        try:
            with self.assertRaises(Exception):
                Scripted.read(broken)
        finally:
            broken.unlink()


class TestTheCeilingAndTheLedger(unittest.TestCase):
    """CLAUDE.md #7's three mechanisms, checked without a network and without a cent.

    The client is a stub, the prices are handed in, and the ledger is a scratch file — so what
    is under test is the arithmetic and the order of events, which is the whole of what stands
    between a loop and an unbounded bill.
    """

    #: Made up, and in the right proportion: $1 a million in, $10 a million out.
    PRICED = Prices(0.000001, 0.00001, 131072, "2026-09-24", "a test")

    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.ledger = Path(self.directory.name) / "spend.jsonl"
        self.addCleanup(self.directory.cleanup)

    def stub(self, answer: object) -> object:
        """An `openai.OpenAI` in the one shape `Live.call` uses, counting what it was asked."""
        made: list[dict] = []

        class Completions:
            def create(_self, **body: object) -> object:
                made.append(dict(body))
                if isinstance(answer, Exception):
                    raise answer
                return answer

        class Chat:
            completions = Completions()

        class Client:
            chat = Chat()

        client = Client()
        client.made = made  # type: ignore[attr-defined]
        return client

    def body(self, tokens: int = 1000) -> dict:
        return {
            "model": "deepseek/deepseek-v4.1-flash",
            "messages": [{"role": "user", "content": "x" * 100}],
            "max_tokens": tokens,
        }

    def answered(self) -> ChatCompletion:
        recorded = json.loads(RECORDED.read_text(encoding="utf-8"))
        return ChatCompletion.model_validate(recorded["exchanges"][0]["response"])

    def test_a_call_that_would_cross_the_ceiling_is_never_made(self) -> None:
        # Watched failing first: with the check written after `create` the stub recorded one
        # call and this assertion read `1`, which is the whole difference between a ceiling
        # and a receipt.
        client = self.stub(self.answered())
        over = int((CEILING_USD / self.PRICED.completion) * 2)
        provider = Live(client, self.ledger, priced=self.PRICED)
        with self.assertRaises(BudgetExhausted) as refused:
            provider.call(self.body(over))
        self.assertEqual(client.made, [], "the call went out anyway")
        self.assertIn("refusing to spend", str(refused.exception))
        self.assertIn(f"${CEILING_USD:.2f}", str(refused.exception))
        self.assertFalse(self.ledger.exists(), "a call nobody made is not a charge")

    def test_the_ledger_records_the_estimate_the_cost_and_the_running_total(self) -> None:
        provider = Live(self.stub(self.answered()), self.ledger, priced=self.PRICED)
        provider.call(self.body())
        provider.call(self.body())
        rows = [json.loads(line) for line in self.ledger.read_text().splitlines()]
        self.assertEqual(len(rows), 2)
        # The recorded response carries the provider's own `usage.cost`, so the ledger charges
        # what it says rather than what the estimate feared.
        self.assertEqual(rows[0]["cost_usd"], 8.965e-05)
        self.assertEqual(rows[0]["charged_usd"], 8.965e-05)
        self.assertGreater(rows[0]["estimated_usd"], rows[0]["cost_usd"])
        self.assertAlmostEqual(rows[1]["running_total_usd"], 2 * 8.965e-05, places=8)
        self.assertEqual(rows[0]["ceiling_usd"], CEILING_USD)
        self.assertEqual(rows[0]["priced_at"]["source"], "a test")

    def test_the_total_survives_the_process_that_wrote_it(self) -> None:
        # The hole a per-process ceiling leaves: every retry of a recording session would
        # spend the whole grant again. The ledger is read back, so it does not.
        first = Live(self.stub(self.answered()), self.ledger, priced=self.PRICED)
        first.call(self.body())
        second = Live(self.stub(self.answered()), self.ledger, priced=self.PRICED)
        self.assertAlmostEqual(second._spent, 8.965e-05, places=8)

    def test_a_call_that_failed_is_charged_at_its_worst_case(self) -> None:
        # Nothing here can tell whether a failed call was billed, so it is counted as if it
        # was — over-counting, which is the only safe direction for a ceiling, and the row
        # says which it is so reconciliation can explain the gap.
        provider = Live(self.stub(RuntimeError("boom")), self.ledger, priced=self.PRICED)
        with self.assertRaises(RuntimeError):
            provider.call(self.body())
        row = json.loads(self.ledger.read_text().splitlines()[0])
        self.assertIsNone(row["cost_usd"])
        self.assertEqual(row["charged_usd"], row["estimated_usd"])
        self.assertIn("boom", row["failed"])

    def test_a_recording_is_a_transcript_and_carries_no_headers(self) -> None:
        # M3 trap 10, as an assertion rather than as a property of how the recorder happens to
        # be written: what is saved is the body that was handed to the SDK, and the SDK is
        # what adds `Authorization: Bearer …` afterwards.
        recording = Path(self.directory.name) / "recorded.json"
        provider = Live(
            self.stub(self.answered()), self.ledger, record_to=recording, priced=self.PRICED
        )
        provider.call(self.body())
        written = recording.read_text(encoding="utf-8")
        self.assertNotIn("Authorization", written)
        self.assertNotIn("api_key", written)
        self.assertNotIn("Bearer", written)
        saved = json.loads(written)
        self.assertEqual(
            sorted(saved["exchanges"][0]["request"]), ["max_tokens", "messages", "model"]
        )
        # And it is a transcript: the thing that replays it is what reads it back.
        replayer = Scripted.read(recording)
        self.assertEqual(replayer.answer().model, self.answered().model)


class TestTheKey(unittest.TestCase):
    def test_no_fixture_carries_the_key(self) -> None:
        """Counts, and prints only the count (`docs/plan.md`, M3 trap 10).

        A transcript recorder that saved headers would put `Authorization: Bearer …` in
        `git log` for ever, where it would parse, replay and pass every test that did not
        read it. This is the test that reads it.

        **Both fixture trees**, as ADR 0022 §4 asks: `ai/tests/` holds the transcripts the loop
        replays, and `tests/` holds the one the end-to-end golden drives. One test over both,
        because the second tree is the one nobody would remember to add a second test for.
        """
        secrets = [
            value
            for name in ("OPENROUTER_API_KEY", "OPENAI_API_KEY")
            if (value := os.environ.get(name))
        ]
        # The two files that **name** the shapes, being the tests that look for them: this one,
        # and the live run, which asserts its own recording carries no header (ADR 0022 §4).
        # They are still read for the real key — only the prefix count skips them, because a
        # file exempted outright is a file a key could be pasted into.
        names_the_shape = {Path(__file__), ROOT / "tests" / "determinism.rs"}
        found = 0
        looked = 0
        for tree in (HERE, ROOT / "tests"):
            for path in sorted(tree.rglob("*")):
                # The caches Python and cargo write beside the fixtures.
                if not path.is_file():
                    continue
                if "__pycache__" in path.parts or "target" in path.parts:
                    continue
                looked += 1
                text = path.read_text(encoding="utf-8", errors="replace")
                found += sum(text.count(secret) for secret in secrets)
                if path in names_the_shape:
                    continue
                # The prefixes too, so the check still means something on a machine where
                # neither variable is set — which is every CI runner (ADR 0022 §4).
                found += text.count("sk-or-v1-") + text.count("Authorization")
        print(f"key occurrences in {looked} files under ai/tests/ and tests/: {found}")
        self.assertEqual(found, 0)

    def test_the_client_is_given_its_key_and_its_endpoint(self) -> None:
        # The defect this guards is not hypothetical: the spike found `OPENAI_API_KEY` set on
        # the machine it ran on, so an `openai.OpenAI()` built with no arguments would have
        # worked — against the wrong account and the wrong endpoint (ADR 0020 §4).
        with mock.patch.dict(
            os.environ,
            {"OPENROUTER_API_KEY": "not-a-key", "OPENAI_API_KEY": "the-wrong-one"},
        ):
            provider = live()
        # `live` returns the ceiling and the ledger wrapped around the client, not the client
        # (ADR 0022 §4, amended): the one object that can spend money is the one that counts.
        self.assertEqual(provider.client.api_key, "not-a-key")
        self.assertEqual(
            str(provider.client.base_url).rstrip("/"), "https://openrouter.ai/api/v1"
        )
        self.assertEqual(provider.ledger, provider_module.SPEND)

    def test_without_the_variable_there_is_no_client_at_all(self) -> None:
        with mock.patch.dict(os.environ, {"OPENAI_API_KEY": "the-wrong-one"}):
            os.environ.pop("OPENROUTER_API_KEY", None)
            with self.assertRaises(RuntimeError):
                live()


if __name__ == "__main__":
    unittest.main()
