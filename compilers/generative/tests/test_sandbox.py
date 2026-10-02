"""The process: one line on stdout, one call on the socket it named, and the two limits.

Run from `compilers/generative/`:

    uv run python -m unittest discover -s tests

Driven as a **real subprocess over a real socket**, which is where this layer's failures live
— `ai/tests/test_sidecar.py` and `core/tests/mcp.rs` made the same choice for the same reason.
What a unit test of the handler would prove is that a coroutine returns a message; what a
person needs to know is whether the thing `core` spawns prints an address, answers on it,
refuses a loop that never ends, and leaves with a status.

Two of these are measurements rather than assertions about code that was written to pass
them. [`TheLimitsBind`] is ADR 0024 §4's **named assumption** — that `resource` limits set
inside a child started by `uv run` bind the interpreter that actually runs the source, and not
only a launcher in front of it — taken by watching a loop that cannot end be stopped, through
`uv run` and not only through the console script. [`HashSeed`] is the spike's measurement of
the hazard the second `set` lock exists for, re-taken here.
"""

from __future__ import annotations

import asyncio
import os
import shutil
import subprocess
import sys
import tomllib
import types
import unittest
from pathlib import Path
from unittest import mock

from escribass_proto.escribass.generate.v1 import (
    CompileRequest,
    CompileResponse,
)
from escribass_schema.escribass.song.v1 import GeneratorKind, TimeSignatureEvent
from grpclib.client import Channel
from grpclib.const import Cardinality

import escribass_generative

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent

# What `core` is told as its `--generator`, as this environment has it: the console script
# beside the interpreter running these tests (`ai/tests/test_sidecar.py`, same two lines).
SANDBOX = Path(sys.executable).with_name("escribass-generative")

COMPILE = "/escribass.generate.v1.Generate/Compile"

#: Eight names whose `set` iteration order the spike measured as three orders in three runs.
NAMES = "{'kick', 'snare', 'hat', 'ride', 'tom', 'clap', 'rim', 'cow'}"


def compiling(source: str, *, seed: int = 7, kind: GeneratorKind = GeneratorKind.PYTHON) -> CompileRequest:
    return CompileRequest(
        kind=kind,
        source=source,
        seed=seed,
        signature=[TimeSignatureEvent(id="01S0", tick=0, numerator=4, denominator=4)],
        clip_start_tick=0,
        clip_length_ticks=3840,
    )


class Started:
    """The compiler, started as `core` starts it: one line read, then the socket it named.

    `PYTHONHASHSEED=0` is passed unless a test says otherwise, because that is what `core`
    will do and it is the path that does not pay a second interpreter start.
    """

    def __init__(self, *arguments: str, command: list[str] | None = None, hash_seed: str | None = "0") -> None:
        environment = {**os.environ}
        environment.pop("PYTHONHASHSEED", None)
        if hash_seed is not None:
            environment["PYTHONHASHSEED"] = hash_seed
        #: The directory the child names its socket in, once it has named one — see `__exit__`.
        self.socket_directory: Path | None = None
        self.child = subprocess.Popen(
            [*(command or [str(SANDBOX)]), *arguments],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=environment,
        )

    def __enter__(self) -> "Started":
        return self

    def __exit__(self, *_: object) -> None:
        if self.child.poll() is None:
            self.child.kill()
        for pipe in (self.child.stdin, self.child.stdout, self.child.stderr):
            if pipe is not None and not pipe.closed:
                pipe.close()
        self.child.wait(timeout=60)
        # **A killed process runs no `finally`**, so the directory it made for its socket is
        # whoever killed it's to remove — this suite for the children here, and `core` for the
        # ones it spawns (`core/src/generator.rs`, `swept`). Three paths reach it: the hard CPU
        # limit, `core`'s wall clock, and this line (found by review, 2026-10-02).
        if self.socket_directory is not None:
            shutil.rmtree(self.socket_directory, ignore_errors=True)

    def address(self) -> str:
        assert self.child.stdout is not None
        line = self.child.stdout.readline().strip()
        if line.startswith("unix:"):
            self.socket_directory = Path(line[len("unix:") :]).parent
        return line

    def stop(self) -> tuple[int, str]:
        """Closes its stdin, which is how the host says nobody is waiting any more."""
        assert self.child.stdin is not None
        self.child.stdin.close()
        return self.child.wait(timeout=60), self.child.stderr.read() if self.child.stderr else ""


async def _call(socket: str, request: CompileRequest) -> CompileResponse:
    async with Channel(path=socket) as channel:
        async with channel.request(
            COMPILE, Cardinality.UNARY_UNARY, CompileRequest, CompileResponse
        ) as stream:
            await stream.send_message(request, end=True)
            answer = await stream.recv_message()
    assert answer is not None
    return answer


def compiled(request: CompileRequest, *arguments: str, **started: object) -> CompileResponse:
    """One compile, start to finish, through a child that is gone when this returns."""
    with Started(*arguments, **started) as child:  # type: ignore[arg-type]
        address = child.address()
        assert address.startswith("unix:"), f"no address line: {address!r}"
        answer = asyncio.run(_call(address[len("unix:") :], request))
        status, stderr = child.stop()
        assert status == 0, f"exit {status}: {stderr}"
    return answer


class TheProcess(unittest.TestCase):
    def test_stdout_carries_the_address_line_and_nothing_else(self) -> None:
        with Started() as child:
            address = child.address()
            self.assertTrue(address.startswith("unix:"), address)
            self.assertTrue(Path(address[len("unix:") :]).exists(), "the socket is listening")
            answer = asyncio.run(_call(address[len("unix:") :], compiling("note(60, 0, 480)\n")))
            self.assertEqual(len(answer.notes.notes), 1)
            status, _ = child.stop()
            self.assertEqual(status, 0)
            assert child.child.stdout is not None
            self.assertEqual(child.child.stdout.read(), "", "stdout after the address line")

    def test_one_call_and_the_child_is_gone_without_being_told(self) -> None:
        # ADR 0024 §2: one `Compile`, then exit. Nothing survives between two compiles, so a
        # compile cannot depend on the compile before it. The stdin is left open on purpose
        # here — if the child needed it closed, this would hang and the timeout would say so.
        with Started() as child:
            address = child.address()
            asyncio.run(_call(address[len("unix:") :], compiling("note(60, 0, 480)\n")))
            self.assertEqual(child.child.wait(timeout=60), 0)

    def test_it_leaves_with_a_status_when_its_stdin_closes_and_no_call_came(self) -> None:
        with Started() as child:
            child.address()
            status, stderr = child.stop()
            self.assertEqual(status, 0, stderr)

    def test_the_socket_goes_with_the_process(self) -> None:
        with Started() as child:
            address = child.address()[len("unix:") :]
            child.stop()
            self.assertFalse(Path(address).exists())
            self.assertFalse(Path(address).parent.exists())


class TheAnswer(unittest.TestCase):
    def test_every_answer_states_the_dsl_s_version_and_the_interpreter_s(self) -> None:
        # ADR 0027 §1: a fact about the child is stated by the child, and a constant in `core`
        # mirroring it is the copy that stops agreeing. `core` writes both into `lock.json`'s
        # `toolchains.generator` on the first compile that commits.
        declared = tomllib.loads((ROOT / "pyproject.toml").read_text())["project"]["version"]
        for answer in (
            compiled(compiling("note(60, 0, 480)\n")),
            compiled(compiling("kit = {36}\n")),
            compiled(compiling("x = 1\n", kind=GeneratorKind.UNSPECIFIED)),
        ):
            self.assertEqual(answer.dsl_version, declared)
            self.assertEqual(answer.dsl_version, "1")
            self.assertEqual(answer.python_version, "3.12.12")

    def test_notes_cross_and_a_generator_that_writes_nothing_crosses_an_empty_list(self) -> None:
        answer = compiled(compiling("x = 1\n"))
        self.assertIsNotNone(answer.notes, "an empty answer is `notes`, not an unset result")
        self.assertEqual(answer.notes.notes, [])
        self.assertIsNone(answer.diagnostic)

    def test_a_refusal_crosses_as_a_diagnostic_with_its_line_and_column(self) -> None:
        answer = compiled(compiling("note(60, 0, 480)\nkit = {36, 38}\n"))
        self.assertIsNone(answer.notes)
        self.assertEqual((answer.diagnostic.line, answer.diagnostic.column), (2, 7))
        self.assertIn("set literal", answer.diagnostic.message)

    def test_a_kind_this_compiler_does_not_implement_is_its_own_refusal(self) -> None:
        answer = compiled(compiling("note(60, 0, 480)\n", kind=GeneratorKind.UNSPECIFIED))
        self.assertIsNone(answer.notes)
        self.assertIn("UNSPECIFIED", answer.diagnostic.message)

    def test_the_same_seed_twice_through_two_processes_is_the_same_bytes(self) -> None:
        source = "for b in range(8):\n    note(rng.randint(36, 48), beat(b), 240, 100)\n"
        first = compiled(compiling(source, seed=2**63 + 1))
        second = compiled(compiling(source, seed=2**63 + 1))
        self.assertEqual(bytes(first), bytes(second))
        self.assertNotEqual(bytes(first), bytes(compiled(compiling(source, seed=2**63 + 2))))


class TheLimitsBind(unittest.TestCase):
    """ADR 0024 §4's limits, each watched stopping something that cannot stop itself.

    Both sources below are unbounded: `while True` with no `break`, and a loop that allocates
    eight megabytes a turn. Neither can pass by finishing, which is the thing that would make
    this suite say "the limit works" about a limit that was never reached.
    """

    FOREVER = "n = 0\nwhile True:\n    n = n + 1\n"
    GREEDY = "held = []\nwhile True:\n    held.append([0] * 1000000)\n"

    def test_a_loop_that_never_ends_is_refused_by_the_cpu_limit(self) -> None:
        answer = compiled(compiling(self.FOREVER), "--cpu-seconds", "1")
        self.assertIsNone(answer.notes)
        self.assertIn("CPU limit of 1 second", answer.diagnostic.message)
        # The line the generator was on when the signal arrived, walked off the frame: a
        # timeout with no position is the diagnostic Plate 3 complains about.
        self.assertIn(answer.diagnostic.line, (2, 3), answer.diagnostic.message)

    def test_a_loop_that_allocates_for_ever_is_refused_by_the_memory_limit(self) -> None:
        answer = compiled(compiling(self.GREEDY), "--memory-mb", "300")
        self.assertIsNone(answer.notes)
        self.assertIn("MemoryError", answer.diagnostic.message)
        self.assertIn("300 MiB", answer.diagnostic.message)

    def test_the_limits_bind_under_uv_run_and_not_only_a_launcher(self) -> None:
        # ADR 0024 §4 named this as an assumption rather than a measurement: "that `resource`
        # limits set inside a child started by `uv run` bind the interpreter that actually
        # runs the source, and not only a launcher in front of it. It is tested in PR 4 by a
        # loop watched refused; if the launcher gets in the way, the command `core` is told is
        # the interpreter itself." This is that test.
        uv = shutil.which("uv")
        self.assertIsNotNone(uv, "these tests run under `uv run`, so `uv` is on the path")
        answer = compiled(
            compiling(self.FOREVER),
            "--cpu-seconds",
            "1",
            command=[str(uv), "run", "--no-sync", "--project", str(ROOT), "escribass-generative"],
        )
        self.assertIsNone(answer.notes)
        self.assertIn("CPU limit", answer.diagnostic.message)

    def test_the_cpu_budget_is_counted_from_what_starting_the_interpreter_cost(self) -> None:
        # ADR 0024 §4: `RLIMIT_CPU` counts from process start, so the budget a flag names is
        # added to what starting this interpreter has already spent — without that, five seconds
        # is quietly "five seconds minus however long `uv` and three imports took", and on a slow
        # enough machine the limit fires before the call it bounds ever arrives. Nothing tested
        # the offset, so dropping it passed (M4 PR 8, mutation A21).
        #
        # Driven against a stubbed `resource`, because the real reading is what the offset is for
        # and a test that measured it would be measuring the runner.
        spent = types.SimpleNamespace(ru_utime=4.0, ru_stime=3.0)
        set_to: list[tuple[int, tuple[int, int]]] = []
        self.addCleanup(
            setattr, escribass_generative, "_measured_budget", escribass_generative.CPU_SECONDS
        )
        with (
            mock.patch.object(escribass_generative.resource, "getrusage", lambda _who: spent),
            mock.patch.object(
                escribass_generative.resource,
                "getrlimit",
                lambda _what: (escribass_generative.resource.RLIM_INFINITY,) * 2,
            ),
            mock.patch.object(
                escribass_generative.resource,
                "setrlimit",
                lambda what, pair: set_to.append((what, pair)),
            ),
            mock.patch.object(escribass_generative.signal, "signal", lambda *_: None),
        ):
            escribass_generative._impose(5, 512)
        budget = 7 + 1 + 5
        self.assertEqual(
            dict(set_to)[escribass_generative.resource.RLIMIT_CPU],
            (budget, budget + escribass_generative._CPU_GRACE),
            "the CPU already spent was not added to the budget",
        )
        # And the memory limit is the flag in bytes, which is the other half of the same call.
        self.assertEqual(
            dict(set_to)[escribass_generative.resource.RLIMIT_AS], (512 << 20, 512 << 20)
        )

    def test_a_generator_inside_the_limits_is_not_refused_by_them(self) -> None:
        # The other half of trap 1: a limit that refused everything would pass both tests
        # above.
        answer = compiled(
            compiling("for b in range(2000):\n    note(60, b, 1)\n"), "--cpu-seconds", "1"
        )
        self.assertEqual(len(answer.notes.notes), 2000)


class HashSeed(unittest.TestCase):
    """The second of ADR 0024 §3's two locks on `set`, and the hazard it is there for.

    The first lock — `Set`, `SetComp` and the name `set` outside the language — is exercised
    in `test_dsl.py` in a process whose hash seed is random, so it is shown holding without
    this one. This half shows this one holding without the first: a `set` of strings iterates
    in an order that is not a property of the source, and `PYTHONHASHSEED=0` fixes it.
    """

    def order(self, seed: str | None) -> str:
        environment = {**os.environ}
        environment.pop("PYTHONHASHSEED", None)
        if seed is not None:
            environment["PYTHONHASHSEED"] = seed
        return subprocess.run(
            [sys.executable, "-c", f"print(list({NAMES}))"],
            capture_output=True,
            text=True,
            env=environment,
            check=True,
        ).stdout

    def test_a_set_of_names_iterates_in_an_order_that_is_not_the_source_s(self) -> None:
        # The spike's measurement, re-taken: three runs of one source, three orders. This is
        # the one way a pure integer DSL still produces two answers.
        self.assertNotEqual(self.order("1"), self.order("2"))
        self.assertNotEqual(self.order("2"), self.order("3"))

    def test_python_hash_seed_zero_fixes_it(self) -> None:
        self.assertEqual(self.order("0"), self.order("0"))
        self.assertNotEqual(self.order("0"), self.order("1"))

    def test_the_serving_process_has_it_whoever_spawned_it(self) -> None:
        # It cannot be set from inside a running interpreter, so a child that was not given
        # it re-executes itself with it rather than trusting its spawner — which is what makes
        # this lock the child's own and not a line in `core` that a hand-driven compile skips.
        for given in (None, "0", "7"):
            with self.subTest(given=given):
                with Started(hash_seed=given) as child:
                    child.address()
                    environment = Path(f"/proc/{child.child.pid}/environ").read_bytes()
                    self.assertIn(b"PYTHONHASHSEED=0", environment.split(b"\0"))
                    child.stop()


class TheVersionIsOnePlace(unittest.TestCase):
    def test_the_dsl_version_is_pyproject_s_and_is_read_from_it(self) -> None:
        declared = tomllib.loads((ROOT / "pyproject.toml").read_text())["project"]["version"]
        self.assertEqual(escribass_generative.dsl_version(), declared)

    def test_the_python_version_is_the_pin_in_python_version(self) -> None:
        self.assertEqual(
            escribass_generative.python_version(), (ROOT / ".python-version").read_text().strip()
        )

    def test_both_are_read_and_neither_is_a_constant_that_happens_to_agree(self) -> None:
        # The two tests above compare each function to a file whose content it equals today —
        # which is exactly what `return "1"` and `return "3.12.12"` also pass (M4 PR 8,
        # mutations A14 and A15). ADR 0027 §1's claim is not that the numbers agree but that
        # **the child states a fact about itself**, so this moves the source each one reads and
        # watches the answer move with it. A constant does not move.
        with mock.patch.object(escribass_generative, "_installed_version") as reading:
            reading.return_value = "99.98.97"
            self.assertEqual(escribass_generative.dsl_version(), "99.98.97")
        reading.assert_called_once_with("escribass-generative")
        with mock.patch.object(
            escribass_generative.platform, "python_version", return_value="3.99.0"
        ):
            self.assertEqual(escribass_generative.python_version(), "3.99.0")


if __name__ == "__main__":
    unittest.main()
