"""The generative compiler's process: one `Compile`, on a socket it names, then gone.

```text
escribass-generative [--cpu-seconds N] [--memory-mb N]
```

**The whole protocol for starting it is one line.** The socket is named here, in a directory
of this process's own, and `unix:<path>` is printed to stdout **after** the server is
listening — so the address and the readiness are one fact and `core` has nothing to poll, no
port to guess and no sleep to tune (ADR 0013 §3, two processes over; `ai/src/escribass_ai`,
whose `serve` this is a shorter copy of). The corollary is the rule the package keeps:
**stdout carries that line and nothing else.** `grpclib` reports through `logging`, whose
default handler writes to stderr, and `tests/test_sandbox.py` asserts the one line rather than
trusting that. The DSL cannot print at all: `print` is not in its namespace.

**A fresh process per compile** (ADR 0024 §1): nothing survives between two compiles, so a
compile cannot depend on the compile before it, and a limit that kills the child costs a
restart and nothing else. The child is told **nothing about the project** — no path, no
`lock.json`, no `song.json`, not even the clip's id. It is handed what it compiles and hands
back notes (ADR 0026 §1).

**Two ways it ends, and the first to happen wins.** It serves one `Compile` and stops, with
the answer's trailers explicitly on the wire before anything is closed; and it stops when the
host closes its stdin, which is how a child learns nobody is waiting any more (ADR 0020 §5,
one process over) and what keeps a `core` that died from leaving a server on an abandoned
socket. `SIGINT` and `SIGTERM` stop it too, through `grpclib`'s own handler.

**What "sandbox" claims here, and what it does not** (ADR 0024 §4). Purity is the DSL's, by
absence, and is in [`escribass_generative.dsl`]. This module owns the other half — a limit,
enforced at the process:

  * `RLIMIT_CPU`, so a generator that loops for ever is a refusal and not a hang (M0.4's rule
    that a hang is not a failure unless one is imposed). The soft limit arrives as `SIGXCPU`,
    which is **caught** and turned into the ordinary refusal shape with the line the source
    was on; the hard limit a few seconds above it is what stops a runaway in the answering
    path itself.
  * `RLIMIT_AS`, so a generator that allocates for ever is a `MemoryError` and a refusal.
  * `PYTHONHASHSEED=0`, which is the second of ADR 0024 §3's two locks on `set`. The first is
    that `Set`, `SetComp` and the name `set` are outside the language. Both, because a
    default nobody can see is how Surge XT's `A Osc 1 Retrigger` hid for two pull requests —
    and because `hash()` of a `str` is randomised per process, which the spike measured as
    three iteration orders from three runs of one source. It cannot be set from inside a
    running interpreter, so a child that was not given it **re-executes itself** with it
    rather than trusting whoever spawned it; `core` sets it in the child's environment and
    the re-exec never happens on that path.

**Not a security boundary against a hostile author** (ADR 0024 §4), and nothing here should
be read as one. The threat it is built against is a sloppy author — a model reaching for
`import random` and `math.sin`. A real sandbox would be built on this process boundary later
(a seccomp filter, a namespace, a separate user) and nothing here would have to be undone.
"""

from __future__ import annotations

import argparse
import asyncio
import os
import platform
import resource
import shutil
import signal
import sys
import tempfile
from importlib.metadata import version as _installed_version
from types import FrameType

from escribass_proto.escribass.generate.v1 import (
    CompileRequest,
    CompileResponse,
    Diagnostic,
    GenerateBase,
    Notes,
)
from escribass_schema.escribass.song.v1 import GeneratorKind
from grpclib.const import Handler
from grpclib.server import Server
from grpclib.utils import graceful_exit

from . import dsl

__all__ = ["Generate", "dsl_version", "main", "python_version", "serve"]

# The compiler's configuration, as numbers rather than as rules (ADR 0022 §3's phrasing;
# ADR 0024 §4). A generator doing real work writes a few thousand notes in well under a second
# and allocates a few megabytes, so both are two or three orders of magnitude of headroom and
# neither is a figure a legitimate source has to think about. They are flags as well as
# defaults because a test that watches a loop refused by each has to be able to shrink them:
# waiting out the default would be slow, and allocating half a gigabyte on a runner to prove
# a limit works is a worse test than allocating a third of one.
CPU_SECONDS = 5
MEMORY_MB = 512

# The CPU already spent when the limit is imposed, which the budget is counted **from**
# (`_impose`). Without it the number would quietly be "five seconds minus however long this
# interpreter took to start", which is a figure that changes with the machine — and on a slow
# enough one the limit would fire before the call it is there to bound ever arrived.
_measured_budget = CPU_SECONDS

# How far above the soft CPU limit the hard one sits. The soft limit is caught and turned into
# an answer, and answering costs CPU; the hard limit is what is left if that goes wrong, and
# `SIGKILL` from it is a child that exited without answering, which `core` reports as
# `generator_failed` (ADR 0024 §7).
_CPU_GRACE = 5


def dsl_version() -> str:
    """The DSL's version — `pyproject.toml`'s, read from the installed metadata.

    One copy of the number, owned by the thing the number is about (ADR 0010 §4; ADR 0027 §1).
    `core` compares what this says against the project's `toolchains.generator` block and the
    generator's own `toolchain_version`; a constant in Rust mirroring this one is the copy
    that stops agreeing.
    """
    return _installed_version("escribass-generative")


def python_version() -> str:
    """The interpreter this is running on, as §17 writes it: `3.12.12`."""
    return platform.python_version()


class Generate(GenerateBase):
    """The one RPC, `Generate.Compile` (`proto/generate.proto`; ADR 0026 §1).

    Synchronous inside an async handler on purpose: the process serves exactly one call and
    exits, so there is no second request for a blocking compile to starve, and an executor
    would put the author's code on a thread where `SIGXCPU` is not delivered to it.
    """

    def __init__(self, served: asyncio.Event) -> None:
        self._served = served

    async def compile(self, message: CompileRequest) -> CompileResponse:
        answer = CompileResponse(dsl_version=dsl_version(), python_version=python_version())
        if message.kind is not GeneratorKind.PYTHON:
            # A kind this child does not implement is **its own** refusal rather than `core`'s
            # guess about what the command it was told can compile (`generate.proto`, `kind`).
            answer.diagnostic = Diagnostic(
                line=0,
                column=0,
                message=(
                    f"{message.kind.name} is not a kind this compiler implements; it compiles"
                    " GENERATOR_KIND_PYTHON (docs/specs.md §15)"
                ),
            )
            return answer
        try:
            notes = dsl.run(message)
        except dsl.Refused as refused:
            answer.diagnostic = Diagnostic(
                line=refused.line, column=refused.column, message=refused.message
            )
            return answer
        # An empty list is an answer, and a different one from no answer at all: a generator
        # that legitimately emits nothing sets `notes` to `[]`, so `core` reads an unset
        # `result` as `generator_failed` rather than as zero notes (`generate.proto`).
        answer.notes = Notes(notes=notes)
        return answer

    def __mapping__(self) -> dict[str, Handler]:
        """The generated mapping, with the one handler wrapped so the child leaves after it.

        ADR 0024 §2's "one call, and the child exits", written where it cannot race the
        answer: `send_trailing_metadata` is called explicitly — grpclib documents it as the
        thing its request handler would otherwise do implicitly at exit — so the response
        **and** its trailers are on the wire before anything is told to stop. A handler that
        raised instead sets nothing and the child waits for its stdin, which is the safe way
        round: a child that outlives its purpose is tidied by the host letting go, and one
        that leaves early is an answer the caller never read.
        """
        return {
            path: handler._replace(func=self._once(handler.func))
            for path, handler in super().__mapping__().items()
        }

    def _once(self, func):  # type: ignore[no-untyped-def]
        async def served(stream) -> None:  # type: ignore[no-untyped-def]
            await func(stream)
            await stream.send_trailing_metadata()
            self._served.set()

        return served


async def serve() -> None:
    """Serves `Generate` on a socket of this process's own until the one call is answered."""
    served = asyncio.Event()
    server = Server([Generate(served)])
    # `mkdtemp`, not a path a caller chose: a parent-chosen path needs entropy or a pid in
    # `core`, which is what CLAUDE.md #3 keeps out of it. `mkdtemp` makes it 0700, so the
    # socket is this user's (`ai/src/escribass_ai`, same two sentences).
    directory = tempfile.mkdtemp(prefix="escribass-generative-")
    socket = os.path.join(directory, "generate.sock")
    try:
        with graceful_exit([server]):
            await server.start(path=socket)
            # After `start`, never before: this line is the readiness as well as the address.
            print(f"unix:{socket}", flush=True)
            closed = asyncio.ensure_future(server.wait_closed())
            answered = asyncio.ensure_future(served.wait())
            orphaned = asyncio.ensure_future(_stdin_closed())
            await asyncio.wait(
                [closed, answered, orphaned], return_when=asyncio.FIRST_COMPLETED
            )
            for pending in (answered, orphaned):
                # Cancelled rather than left pending: a task still waiting at interpreter exit
                # is a warning on stderr from a process that did nothing wrong. `closed` is
                # never cancelled — it awaits grpclib's own future, and cancelling the wait
                # cancels that future.
                pending.cancel()
            if closed.done():
                await closed
            else:
                server.close()
    finally:
        # The socket goes with the process, so nothing is left listening on a path a later run
        # would meet.
        shutil.rmtree(directory, ignore_errors=True)


async def _stdin_closed() -> None:
    """Waits for the host to let go of our stdin, which is how a session ends (ADR 0020 §5)."""
    reader = asyncio.StreamReader()
    loop = asyncio.get_running_loop()
    try:
        transport, _ = await loop.connect_read_pipe(
            lambda: asyncio.StreamReaderProtocol(reader), sys.stdin
        )
    except (OSError, ValueError):
        # No stdin to watch — started with the descriptor closed. Then the only way out is the
        # call being answered or a signal, and waiting for ever is better than the
        # alternative: a task that completes here stops the server before it is dialled.
        print("escribass-generative: no stdin to watch", file=sys.stderr)
        await asyncio.Event().wait()
        return
    try:
        # Nothing is expected to arrive; what is being waited for is the end of the pipe.
        while await reader.read(4096):
            pass
    finally:
        transport.close()


def _with_hash_seed() -> None:
    """Re-executes this process with `PYTHONHASHSEED=0` unless it already has it.

    The interpreter fixes its hash seed before any of this runs, so it cannot be set from
    inside: the only way for the child to *own* ADR 0024 §3's second lock, rather than trust
    whoever spawned it, is to start again. `core` sets the variable in the child's environment
    and this returns immediately on that path, so the cost — one extra interpreter start — is
    paid only by a shell driving the compiler by hand.

    A failure to re-exec exits rather than carrying on: continuing would leave the process
    running with one of the two locks silently missing, which is exactly the state the two
    exist to make impossible.
    """
    if os.environ.get("PYTHONHASHSEED") == "0":
        return
    try:
        # `sys.orig_argv` is the interpreter's own command line, so this is right for a
        # console script, for `-m` and for a path — `sys.argv` would have lost the `-m`.
        os.execve(sys.executable, sys.orig_argv, {**os.environ, "PYTHONHASHSEED": "0"})
    except OSError as unstartable:
        print(
            f"escribass-generative: cannot re-exec with PYTHONHASHSEED=0: {unstartable}",
            file=sys.stderr,
        )
        raise SystemExit(2) from None


def _on_sigxcpu(_signal: int, frame: FrameType | None) -> None:
    """Turns the soft CPU limit into the ordinary refusal, with the line it was reached on."""
    # One shot: the answer has to get out, and the kernel re-sends SIGXCPU every second after
    # the soft limit. The hard limit a few seconds up is what is left if the answer hangs.
    signal.signal(signal.SIGXCPU, signal.SIG_IGN)
    line = 0
    at = frame
    while at is not None:
        if at.f_code.co_filename == dsl.SOURCE:
            line = at.f_lineno
            break
        at = at.f_back
    plural = "" if _measured_budget == 1 else "s"
    raise dsl.Refused(
        f"the generator reached the compiler's CPU limit of {_measured_budget} second{plural}"
        " (ADR 0024 §4)",
        line,
    )


def _impose(cpu_seconds: int, memory_mb: int) -> None:
    """Sets the two limits on this process, before it executes anything (ADR 0024 §4).

    `RLIMIT_CPU` counts from process start, so the budget is added to what starting this
    interpreter has already cost. That reading of `getrusage` is the one measurement in this
    package and it decides nothing about the output: a compile that finishes produces the same
    notes whatever it was allowed, and a compile that does not produces a refusal. CLAUDE.md
    #3 is about what a compiler *answers*, and the limit is the thing that stops there being
    no answer at all.
    """
    global _measured_budget
    _measured_budget = cpu_seconds
    spent = resource.getrusage(resource.RUSAGE_SELF)
    budget = int(spent.ru_utime + spent.ru_stime) + 1 + cpu_seconds
    _, hard = resource.getrlimit(resource.RLIMIT_CPU)
    ceiling = budget + _CPU_GRACE
    if hard != resource.RLIM_INFINITY:
        ceiling = min(ceiling, hard)
    resource.setrlimit(resource.RLIMIT_CPU, (min(budget, ceiling), ceiling))
    signal.signal(signal.SIGXCPU, _on_sigxcpu)
    memory = memory_mb << 20
    _, hard = resource.getrlimit(resource.RLIMIT_AS)
    if hard != resource.RLIM_INFINITY:
        memory = min(memory, hard)
    resource.setrlimit(resource.RLIMIT_AS, (memory, memory))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="escribass-generative",
        description=(
            "The escribass generative compiler: serves one Generate.Compile on a Unix socket "
            "it names, prints `unix:<path>` on stdout once it is listening, and exits."
        ),
    )
    parser.add_argument(
        "--cpu-seconds",
        type=int,
        default=CPU_SECONDS,
        help=(
            "the process's CPU budget. Reaching it is a refusal naming the line the generator "
            "was on, never a hang (ADR 0024 §4)"
        ),
    )
    parser.add_argument(
        "--memory-mb",
        type=int,
        default=MEMORY_MB,
        help="the process's address-space budget. Reaching it is a refusal (ADR 0024 §4)",
    )
    options = parser.parse_args(argv)
    # Before the limits, because resource limits survive an exec and the CPU this process has
    # already spent would be counted against the budget of the one that replaces it.
    _with_hash_seed()
    _impose(options.cpu_seconds, options.memory_mb)
    asyncio.run(serve())
    return 0
