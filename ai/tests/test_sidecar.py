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
import json
import os
import subprocess
import sys
import unittest
from pathlib import Path
from unittest import mock

from escribass_ai.provider import Exhausted, Scripted, live
from escribass_proto.escribass.assistant.v1 import (
    AssistantCommand,
    AssistantEvent,
    Prompt,
)
from escribass_schema.escribass.song.v1 import Song
from grpclib.client import Channel
from grpclib.const import Cardinality

HERE = Path(__file__).resolve().parent
TRANSCRIPT = HERE / "transcripts" / "one-answer.json"

# What `core` spawns, as this environment has it: `uv run --project ai escribass-ai` resolves
# to the console script beside the interpreter running these tests (ADR 0020 §4).
SIDECAR = Path(sys.executable).with_name("escribass-ai")

PROMPT = "/escribass.assistant.v1.Assistant/Prompt"


def recorded() -> dict:
    return json.loads(TRANSCRIPT.read_text(encoding="utf-8"))


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
    """Dials the socket and drives one turn, as `core::assistant` does in Rust."""
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
                ),
                end=True,
            )
            return [event async for event in stream]


class TestTheProcess(unittest.TestCase):
    def test_it_prints_one_line_naming_its_socket_and_answers_on_it(self) -> None:
        with Started("--transcript", str(TRANSCRIPT)) as sidecar:
            address = sidecar.address()
            self.assertTrue(address.startswith("unix:"), address)
            socket = address.removeprefix("unix:")
            # The line is the readiness as well as the address: nothing sleeps, polls or
            # retries here, and a connection refused would be this assertion failing.
            self.assertTrue(Path(socket).is_socket(), f"{socket} is not a socket")

            events = asyncio.run(one_prompt(socket, "What can you change about this song?"))

            answer = recorded()["exchanges"][0]["response"]
            said = answer["choices"][0]["message"]["content"]
            self.assertEqual([e.text.text for e in events if e.text], [said])
            done = [e.done for e in events if e.done]
            self.assertEqual([d.text for d in done], [said])
            # The model the response **named**, which is what the log records — not the id the
            # prompt asked for (ADR 0021 §2).
            self.assertEqual([d.model_id for d in done], [answer["model"]])
            # Nothing is proposed: this build has no loop (ADR 0019 §1, PR 8).
            self.assertEqual([e.call for e in events if e.call], [])

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
        provider = Scripted.read(TRANSCRIPT)
        first = provider.answer()
        self.assertEqual(first.model, recorded()["exchanges"][0]["response"]["model"])
        with self.assertRaises(Exhausted):
            provider.answer()

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


class TestTheKey(unittest.TestCase):
    def test_no_fixture_here_carries_the_key(self) -> None:
        """Counts, and prints only the count (`docs/plan.md`, M3 trap 10).

        A transcript recorder that saved headers would put `Authorization: Bearer …` in
        `git log` for ever, where it would parse, replay and pass every test that did not
        read it. This is the test that reads it.
        """
        secrets = [
            value
            for name in ("OPENROUTER_API_KEY", "OPENAI_API_KEY")
            if (value := os.environ.get(name))
        ]
        found = 0
        for path in sorted(HERE.rglob("*")):
            # Everything but this file, which names the shapes it is looking for, and the
            # caches Python writes beside it.
            if not path.is_file() or path == Path(__file__) or "__pycache__" in path.parts:
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            # The prefixes too, so the check still means something on a machine where neither
            # variable is set — which is every CI runner (ADR 0022 §4).
            found += sum(text.count(secret) for secret in secrets)
            found += text.count("sk-or-v1-") + text.count("Authorization")
        print(f"key occurrences under {HERE.name}/: {found}")
        self.assertEqual(found, 0)

    def test_the_client_is_given_its_key_and_its_endpoint(self) -> None:
        # The defect this guards is not hypothetical: the spike found `OPENAI_API_KEY` set on
        # the machine it ran on, so an `openai.OpenAI()` built with no arguments would have
        # worked — against the wrong account and the wrong endpoint (ADR 0020 §4).
        with mock.patch.dict(
            os.environ,
            {"OPENROUTER_API_KEY": "not-a-key", "OPENAI_API_KEY": "the-wrong-one"},
        ):
            client = live()
        self.assertEqual(client.api_key, "not-a-key")
        self.assertEqual(str(client.base_url).rstrip("/"), "https://openrouter.ai/api/v1")

    def test_without_the_variable_there_is_no_client_at_all(self) -> None:
        with mock.patch.dict(os.environ, {"OPENAI_API_KEY": "the-wrong-one"}):
            os.environ.pop("OPENROUTER_API_KEY", None)
            with self.assertRaises(RuntimeError):
                live()


if __name__ == "__main__":
    unittest.main()
