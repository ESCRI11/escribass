"""The AI orchestrator sidecar (docs/specs.md §3 tier 1, §6; ADR 0020).

```text
escribass-ai --transcript <transcript.json>
```

One process, one service, one bidirectional stream per prompt. **`ai` serves and the host
dials** — the opposite direction from the engine and for a reason that is not symmetry: the
pinned `betterproto2-compiler` generates a `grpclib` *server* and no `grpcio` server at all, so
the process that serves is the one whose gRPC is generated (ADR 0020 §1, §2).

**The whole protocol for starting it is one line.** The socket is named here, in a directory of
this process's own, and `unix:<path>` is printed to stdout **after** the server is listening —
so the address and the readiness are one fact, and `core` has nothing to poll, no port to guess
and no sleep to tune (`core/src/assistant.rs`; ADR 0013 §3, one process over). The corollary is
the rule everything in this package keeps: **stdout carries that line and nothing else.** A
library that prints to stdout corrupts the protocol, which is the same discipline
`escribass-mcp` keeps for its JSON-RPC stream (§18.2) — `grpclib` and the `openai` SDK both
report through `logging`, whose default handler writes to stderr, and
`ai/tests/test_sidecar.py` asserts the one line rather than trusting that.

**What this process is told**: a command's worth of arguments and nothing about the project.
Never the project path — `.escri` is the host's and "agents never read or write the project
file directly" (§5) is a property of what `ai` is given, not of what it is told not to do.
Never a socket, since it names its own. Never a key on a flag: `OPENROUTER_API_KEY` reaches it
from the environment, and `provider.live` is the one place that reads it (ADR 0020 §4; U3).
The model id and the offered tools arrive per prompt, on the stream.

**What it does not do yet.** No loop, no view, no proposal: it answers one prompt with the
transcript's answer and asks for nothing. Every `Prompt` field but the text is carried and
ignored here on purpose — the Libretto view is PR 6's, the tool-calling loop and its three
kinds of failure are PR 8's, and a panel to show any of it is PR 9's. What this pull request
buys is a process a person can start, watch, and watch die (ADR 0020, Consequences, PR 5).

**The host closes our stdin to stop us.** A sidecar lives for the session, so nothing on the
gRPC stream says "we are done"; the pipe does, exactly as it does for `escribass-mcp`, whose
stdin closing is what releases its lock (ADR 0020, Context). Started with stdin at `/dev/null`
the process therefore stops at once, which is the honest reading of "nobody is holding this
open"; started from a terminal, ⌃D stops it. `SIGINT` and `SIGTERM` stop it too, through
`grpclib`'s own handler, so an operator's `kill` produces an exit status of 0 rather than a
signal the health dot would have to explain (ADR 0020 §5).
"""

from __future__ import annotations

import argparse
import asyncio
import os
import shutil
import sys
import tempfile
from collections.abc import AsyncIterator
from pathlib import Path

import grpclib
from escribass_proto.escribass.assistant.v1 import (
    AssistantBase,
    AssistantCommand,
    AssistantEvent,
    Done,
    ReplyText,
)
from grpclib.server import Server
from grpclib.utils import graceful_exit

from .provider import Exhausted, Scripted

__all__ = ["Assistant", "main", "serve"]


class Assistant(AssistantBase):
    """The one RPC, `Assistant.Prompt` (proto/assistant.proto, ADR 0020 §3).

    One prompt in, text and `done` out. The turn is over when the generator returns, which
    ends the stream — the shape ADR 0013 §2 chose for `Preview` and for the same reasons: the
    stream is the turn's identifier, closing it is cancellation for free, and a failure ends it
    with a status rather than travelling as an arm a caller may forget to read.
    """

    def __init__(self, provider: Scripted) -> None:
        self._provider = provider

    async def prompt(
        self, messages: AsyncIterator[AssistantCommand]
    ) -> AsyncIterator[AssistantEvent]:
        command = await anext(messages)
        if command.prompt is None:
            # A `CallResult` first would answer a call nothing made. Refused with a status
            # rather than ignored: the host is the only caller, and a host that sent this has
            # a defect worth seeing.
            raise grpclib.GRPCError(
                grpclib.const.Status.INVALID_ARGUMENT,
                "the first message on a Prompt stream is the prompt (assistant.proto)",
            )

        # The song, the offered schemas and the conversation are read by nothing yet. They are
        # on the wire because PR 3 put the milestone's whole shape there at once (M3 trap 14),
        # and the view that would read the song is PR 6's.
        try:
            answer = self._provider.answer()
        except Exhausted as ran_out:
            # Not a refusal and not a provider failure: a transcript with nothing left is this
            # build's own limit, and ADR 0022 §3's three kinds are about a loop that does not
            # exist yet. It ends the stream with a status, which is where every failure ends.
            raise grpclib.GRPCError(
                grpclib.const.Status.FAILED_PRECONDITION, str(ran_out)
            ) from ran_out

        choice = answer.choices[0]
        if choice.message.tool_calls:
            # A transcript whose model asked for a tool is a transcript for the loop, and the
            # loop is PR 8's. Refused loudly rather than dropped: a proposal silently thrown
            # away is the failure a person would discover by wondering why nothing happened.
            raise grpclib.GRPCError(
                grpclib.const.Status.UNIMPLEMENTED,
                "this build answers and proposes nothing; a transcript with tool calls needs "
                "the loop of M3 PR 8 (ADR 0019 §1)",
            )

        text = choice.message.content or ""
        # Both, and they carry the same text here because the recorded answer arrives whole.
        # `ReplyText` is what a panel renders as it goes (PR 9) and `Done` is what ends the
        # turn and names the model that **answered** — never the id that was asked for, since
        # a router may serve one turn from another provider under the same name (ADR 0021 §2).
        yield AssistantEvent(text=ReplyText(text=text))
        yield AssistantEvent(done=Done(text=text, model_id=answer.model))


async def serve(provider: Scripted) -> None:
    """Serves `Assistant` on a socket of this process's own until the host lets go."""
    server = Server([Assistant(provider)])
    # `mkdtemp`, not a path a caller chose: a parent-chosen path needs entropy or a pid in
    # `core`, which is what CLAUDE.md #3 keeps out of it and what M1 PR 13 had to fix in the
    # engine's own scratch directory. `mkdtemp` makes it 0700, so the socket is this user's.
    directory = tempfile.mkdtemp(prefix="escribass-ai-")
    socket = os.path.join(directory, "assistant.sock")
    try:
        with graceful_exit([server]):
            await server.start(path=socket)
            # After `start`, never before: this line is the readiness as well as the address.
            print(f"unix:{socket}", flush=True)
            # Two ways this ends, and the first to happen wins: a signal, which grpclib's own
            # handler turns into `close()`, or the host letting go of our stdin.
            serving = asyncio.ensure_future(server.wait_closed())
            orphaned = asyncio.ensure_future(_stdin_closed())
            await asyncio.wait([serving, orphaned], return_when=asyncio.FIRST_COMPLETED)
            # Cancelled rather than left pending: a task still waiting at interpreter exit is
            # a warning on stderr from a process that did nothing wrong. `serving` is never
            # cancelled — it awaits grpclib's own future, and cancelling the wait cancels
            # that future, so the close below would then raise instead of finishing.
            orphaned.cancel()
            if not serving.done():
                server.close()
            await serving
    finally:
        # The socket goes with the process, so nothing is left listening on a path a later run
        # would meet.
        shutil.rmtree(directory, ignore_errors=True)


async def _stdin_closed() -> None:
    reader = asyncio.StreamReader()
    loop = asyncio.get_running_loop()
    try:
        transport, _ = await loop.connect_read_pipe(
            lambda: asyncio.StreamReaderProtocol(reader), sys.stdin
        )
    except (OSError, ValueError):
        # No stdin to watch — started with the descriptor closed. Then the only way out is a
        # signal, and waiting for ever is better than the alternative: a task that completes
        # here stops the server, so a failure to *watch* would read as "the host let go".
        print("escribass-ai: no stdin to watch; stop me with a signal", file=sys.stderr)
        await asyncio.Event().wait()
        return
    try:
        # Nothing is expected to arrive; what is being waited for is the end of the pipe.
        while await reader.read(4096):
            pass
    finally:
        transport.close()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="escribass-ai",
        description=(
            "The escribass AI sidecar: serves Assistant on a Unix socket it names, and "
            "prints `unix:<path>` on stdout once it is listening."
        ),
    )
    parser.add_argument(
        "--transcript",
        required=True,
        type=Path,
        help=(
            "a recorded transcript to answer from. Required, because this build has no live "
            "provider: the loop that calls one is M3 PR 8 (ADR 0022 §4)"
        ),
    )
    options = parser.parse_args(argv)
    try:
        provider = Scripted.read(options.transcript)
    except Exception as unreadable:  # noqa: BLE001 — every way a file is not a transcript
        # Stderr and an exit code, which is the whole of what `core` reads when a child will
        # not start (ADR 0020 §5). Nothing on stdout: a caller reading the address line must
        # not be handed an error message where a socket goes.
        print(f"escribass-ai: {options.transcript}: {unreadable}", file=sys.stderr)
        return 2
    asyncio.run(serve(provider))
    return 0
