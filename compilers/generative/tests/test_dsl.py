"""The language: every construct outside it fed and watched refused, and the numbers it keeps.

Run from `compilers/generative/`:

    uv run python -m unittest discover -s tests

`unittest`, because it is the standard library's and this repository adds no test framework
where one is not needed (ADR 0016 §3, two languages over).

**Why this file is long and flat.** An allowlist that has never been fed a denied construct is
the check that cannot fail — the shape M0.4 exists to prevent, one tier over. So
[`Refusals`] is a table with a row per thing the DSL forbids, each fed as a source and each
asserted to come back naming *what* was refused and *where*; and [`EachArmIsLoadBearing`]
removes an arm and watches the same source get through, which is the half of trap 1 that says
a test can fail (ADR 0024 §4, "each failing first against an allowlist with that arm
removed").

The other three claims here are CLAUDE.md #3's, in the milestone that first names `compilers`
in it: the same seed twice is the same bytes, a changed seed changes the notes, and the
arithmetic the DSL is made of gives the digest the spike measured on five CPython minors.
"""

from __future__ import annotations

import ast
import hashlib
import json
import random
import sys
import unittest
from fractions import Fraction
from unittest import mock

from escribass_generative import dsl
from escribass_proto.escribass.generate.v1 import CompileRequest
from escribass_schema.escribass.song.v1 import (
    GeneratorKind,
    Section,
    TempoEvent,
    TimeSignatureEvent,
)


def request(source: str, *, seed: int = 7, start: int = 0, length: int = 3840) -> CompileRequest:
    """A compile of `source` over one bar of 4/4, with a tempo change and two sections.

    The maps are not empty on purpose: `bar`, `beat`, `tempo_at` and `signature_at` all read
    them, and a fixture with one event at tick 0 would let a walker that ignores every event
    after the first pass everything (`app/src/time.ts`'s own warning, two consumers over).
    """
    return CompileRequest(
        kind=GeneratorKind.PYTHON,
        source=source,
        seed=seed,
        params={"density": "3", "mode": "swing"},
        tempo=[
            TempoEvent(id="01T0", tick=0, bpm=120.0),
            TempoEvent(id="01T1", tick=1920, bpm=140.5),
        ],
        signature=[TimeSignatureEvent(id="01S0", tick=0, numerator=4, denominator=4)],
        sections=[
            Section(id="01A", name="intro", start_tick=0, end_tick=3840),
            Section(id="01B", name="verse", start_tick=3840, end_tick=11520),
        ],
        clip_start_tick=start,
        clip_length_ticks=length,
    )


def refusal(source: str, **kwargs: object) -> dsl.Refused:
    try:
        dsl.run(request(source, **kwargs))  # type: ignore[arg-type]
    except dsl.Refused as refused:
        return refused
    raise AssertionError(f"compiled without refusing:\n{source}")


# Every construct the DSL forbids, the source that reaches it, and the words the refusal has
# to carry. ADR 0024 §3's "refused by absence" list, its two "refused by name" rules, the
# argument contract of `note` in its namespace table, and the three ways execution itself can
# be the author's fault.
#
# `await`, `async for` and `async with` have no row of their own: all three are syntax errors
# outside an `async def`, which is the row above them, so the only source that reaches them is
# one the first refusal already stops.
FORBIDDEN = {
    "import": ("import math\n", "Import"),
    "from … import": ("from math import sin\n", "ImportFrom"),
    "class": ("class Drum:\n    pass\n", "ClassDef"),
    "lambda": ("tighten = lambda x: x\n", "Lambda"),
    "try": ("try:\n    note(60, 0, 1)\nexcept Exception:\n    pass\n", "Try"),
    "raise": ("raise ValueError('no')\n", "Raise"),
    "with": ("with clip as c:\n    pass\n", "With"),
    "global": ("def f():\n    global PPQ\n", "Global"),
    "nonlocal": ("def f():\n    x = 1\n    def g():\n        nonlocal x\n", "Nonlocal"),
    "yield": ("def f():\n    yield 1\n", "Yield"),
    "async def": ("async def f():\n    await g()\n", "AsyncFunctionDef"),
    "* unpacking": ("note(*[60, 0, 1])\n", "Starred"),
    "a set literal": ("kit = {36, 38, 42}\n", "Set"),
    "a set comprehension": ("kit = {p for p in range(3)}\n", "SetComp"),
    "a generator expression": ("total = sum(p for p in range(3))\n", "GeneratorExp"),
    "del": ("x = 1\ndel x\n", "Delete"),
    "assert": ("assert PPQ == 960\n", "Assert"),
    ":=": ("if (n := 3) > 2:\n    pass\n", "NamedExpr"),
    "@": ("x = 2 @ 3\n", "MatMult"),
    "/": ("x = beat(1) / 7\n", "Div"),
    "match": ("match PPQ:\n    case 960:\n        pass\n", "Match"),
    "a float constant": ("x = 1.5\n", "a float constant"),
    "a complex constant": ("x = 1j\n", "a complex constant"),
    "a bytes constant": ("x = b'kick'\n", "a bytes constant"),
    "a dunder attribute": ("x = clip.__class__\n", "`__class__` begins with `_`"),
    "a private attribute": ("x = rng._random\n", "`_random` begins with `_`"),
    "a decorator": ("@int\ndef f():\n    pass\n", "a decorator on `f`"),
    "*args": ("def f(*args):\n    pass\n", "`args` is a `*` or `**` parameter"),
    "**kwargs": ("def f(**kw):\n    pass\n", "`kw` is a `*` or `**` parameter"),
    "an unknown module": ("x = math.floor(1)\n", "the name `math`"),
    "eval": ("eval('1')\n", "the name `eval`"),
    "exec": ("exec('1')\n", "the name `exec`"),
    "open": ("open('/etc/passwd')\n", "the name `open`"),
    "__import__": ("__import__('os')\n", "the name `__import__`"),
    "globals": ("globals()\n", "the name `globals`"),
    "getattr": ("getattr(clip, 'start')\n", "the name `getattr`"),
    "print": ("print('hello')\n", "the name `print`"),
    "input": ("input()\n", "the name `input`"),
    "set": ("kit = set([36, 38])\n", "the name `set`"),
    "float": ("x = float(1)\n", "the name `float`"),
    "a clock": ("x = time()\n", "the name `time`"),
    "a syntax error": ("def (:\n", "invalid syntax"),
    "a fraction of a tick": (
        "note(60, Fraction(960, 7), 480)\n",
        "start is 960/7, which is not a whole number of ticks",
    ),
    "a float argument": ("note(60, 0, 2 ** -1)\n", "length is a float"),
    "a pitch out of range": ("note(200, 0, 480)\n", "pitch is 200, which is outside 0–127"),
    "a velocity out of range": (
        "note(60, 0, 480, 0)\n",
        "velocity is 0, which is outside 1–127",
    ),
    "unbounded recursion": ("def f(n):\n    return f(n + 1)\n\n\nf(0)\n", "RecursionError"),
    "an exception": ("x = [1, 2][9]\n", "IndexError"),
}


class Refusals(unittest.TestCase):
    def test_every_forbidden_construct_is_refused_naming_itself(self) -> None:
        for what, (source, named) in FORBIDDEN.items():
            with self.subTest(what):
                refused = refusal(source)
                self.assertIn(named, refused.message, f"{what}: {refused.message}")
                self.assertGreaterEqual(refused.line, 1, f"{what} was refused without a line")

    def test_a_refusal_points_at_the_line_and_column_it_is_about(self) -> None:
        # 1-based, both of them: one more than `ast`'s `col_offset`, which is the conversion
        # `proto/generate.proto` names as an off-by-one no test that ignores the number would
        # catch.
        refused = refusal("note(60, 0, 480)\nx = 1\nkit = {36, 38}\n")
        self.assertEqual((refused.line, refused.column), (3, 7))

    def test_the_first_refusal_in_source_order_is_the_one_returned(self) -> None:
        # Two refusals, the later one deeper in the tree. A traversal that reported what it
        # found first would answer with the import.
        refused = refusal("kit = {36}\nimport math\n")
        self.assertIn("Set", refused.message)

    def test_a_source_too_deep_for_the_parser_is_a_refusal_and_not_a_crash(self) -> None:
        # Found by feeding one. `x = 1 + 1 + …` twenty thousand times is a tree deeper than
        # CPython's own parser stack, and `ast.parse` raises `RecursionError` rather than
        # `SyntaxError` — which, uncaught, leaves `run` as something that is not a refusal at
        # all, and `core` reports `generator_failed`, an **operator** error, for a source an
        # author wrote. The allowlist's walk is iterative for the same reason.
        self.assertIn("RecursionError", refusal("x = " + "+".join(["1"] * 20000) + "\n").message)
        self.assertIn(
            "too many nested parentheses",
            refusal("x = " + "(" * 5000 + "1" + ")" * 5000 + "\n").message,
        )

    def test_a_syntax_error_keeps_its_own_column(self) -> None:
        # `SyntaxError.offset` is already 1-based and is not converted; `ast`'s is. The two
        # paths meeting at one promise is why this is asserted and not assumed.
        source = "note(60, 0, 480))\n"
        refused = refusal(source)
        self.assertEqual(refused.line, 1)
        self.assertGreaterEqual(refused.column, 1)


class EachArmIsLoadBearing(unittest.TestCase):
    """Trap 1: an arm removed, and the same source watched getting through.

    Without this the table above would pass against an allowlist that refused everything for
    one reason, or against one arm doing all the work.
    """

    def test_the_node_allowlist_is_what_refuses_each_node(self) -> None:
        for what, (source, _) in FORBIDDEN.items():
            try:
                tree = ast.parse(source)
            except SyntaxError:
                continue  # the syntax-error row: there is no tree and so no arm to remove
            outside = {
                type(node).__name__
                for node in ast.walk(tree)
                if type(node).__name__ not in dsl._ALLOWED
            }
            if not outside:
                continue  # a name, an attribute, an argument: not this arm's row
            with self.subTest(what):
                # Every node this construct is made of, not only the one the table names: a
                # `try` is a `Try` and an `ExceptHandler`, a `match` is three nodes, and
                # removing one arm of three would prove nothing.
                with mock.patch.object(dsl, "_ALLOWED", dsl._ALLOWED | outside):
                    try:
                        dsl.run(request(source))
                    except dsl.Refused as still:
                        # Some of these then fail for the *other* lock — `import math` finds
                        # no `__import__` in the namespace, which is the point of there being
                        # two — but none of them may still be refused for its node.
                        for node in outside:
                            self.assertNotIn(f"({node})", still.message, f"{what}: {still.message}")

    def test_the_constant_arm_is_what_refuses_a_float_literal(self) -> None:
        with mock.patch.object(dsl, "_CONSTANTS", (int, str, bool, type(None), float)):
            # It parses and runs now; what stops it is the boundary check in `note`, which is
            # the second lock on floats and is asserted separately above.
            self.assertEqual(dsl.run(request("x = 1.5\n")), [])

    def test_the_namespace_is_the_builtins_even_with_no_allowlist_at_all(self) -> None:
        # The second lock, on its own: the allowlist is skipped entirely and the source is
        # executed with exactly what `namespace` returns. Nothing a sloppy author reaches for
        # exists, so an allowlist hole is not an escape by itself (ADR 0024 §3).
        space = dsl.namespace(request(""), [])
        for reach in ("__import__('os')", "open('/etc/passwd')", "eval('1')", "set()"):
            with self.subTest(reach):
                with self.assertRaises(NameError):
                    exec(reach, {"__builtins__": space})  # noqa: S102


class TwoLocksOnSet(unittest.TestCase):
    """`hash(str)` is randomised per process, so a set is not a property of the source.

    ADR 0024 §3 forbids `set` twice over — the nodes and the name are outside the language,
    *and* the child runs under `PYTHONHASHSEED=0` — because a default nobody can see is how
    Surge XT's `A Osc 1 Retrigger` hid for two pull requests. This half shows the first lock
    holding **in a process whose hash seed is random**; `tests/test_sandbox.py` measures the
    second one on its own.
    """

    def test_the_language_refuses_a_set_whatever_the_hash_seed_is(self) -> None:
        self.assertEqual(
            sys.flags.hash_randomization,
            1,
            "this test is only evidence in a process with a randomised hash seed; run it with"
            " PYTHONHASHSEED unset",
        )
        for source in ("kit = {36, 38}\n", "kit = {p for p in range(3)}\n", "kit = set()\n"):
            with self.subTest(source.strip()):
                self.assertIn("set", refusal(source).message.lower())


class Namespace(unittest.TestCase):
    def test_the_worked_example_compiles_to_the_fourteen_notes_the_adr_writes_out(self) -> None:
        # ADR 0024 §3's example, verbatim, and its prose answer: kicks at 0, 960, 1920 and
        # 2880 of length 480 and velocity 100; snares at 960 and 2880 of length 480 and
        # velocity 110; eight hats at 0, 480, … 3360 of length 240, whose velocities are the
        # first eight draws of `random.Random(7).randrange(0, 30)` plus 60.
        source = (
            "for b in range(4):\n"
            "    note(36, beat(b), beat(1) // 2, 100)\n"
            "    if b % 2 == 1:\n"
            "        note(38, beat(b), beat(1) // 2, 110)\n"
            "    for h in range(2):\n"
            "        note(42, beat(b) + h * (beat(1) // 2), beat(1) // 4, 60 + rng.randrange(0, 30))\n"
        )
        notes = dsl.run(request(source))
        self.assertEqual(len(notes), 14)
        kicks = [n for n in notes if n.pitch == 36]
        self.assertEqual([n.start_tick for n in kicks], [0, 960, 1920, 2880])
        self.assertEqual({n.length_ticks for n in kicks}, {480})
        self.assertEqual({n.velocity for n in kicks}, {100})
        snares = [n for n in notes if n.pitch == 38]
        self.assertEqual([n.start_tick for n in snares], [960, 2880])
        self.assertEqual({(n.length_ticks, n.velocity) for n in snares}, {(480, 110)})
        hats = sorted((n for n in notes if n.pitch == 42), key=lambda n: n.start_tick)
        self.assertEqual([n.start_tick for n in hats], [0, 480, 960, 1440, 1920, 2400, 2880, 3360])
        self.assertEqual({n.length_ticks for n in hats}, {240})
        draws = random.Random(7)
        self.assertEqual(
            [n.velocity for n in hats], [60 + draws.randrange(0, 30) for _ in range(8)]
        )

    def test_a_note_crosses_with_its_identity_blank(self) -> None:
        # `core` mints the ids and `prepare` stamps the provenance (ADR 0024 §5): a sandbox
        # that filled either would stop `--seed-ids` being a pure function of the script.
        note = dsl.run(request("note(60, 0, 480)\n"))[0]
        self.assertEqual(note.id, "")
        self.assertIsNone(note.provenance)
        self.assertEqual(note.version, 0)

    def test_bar_and_beat_are_counted_from_the_clips_own_start(self) -> None:
        notes = dsl.run(request("note(60, bar(0), 1)\nnote(61, bar(1), 1)\nnote(62, beat(3), 1)\n", start=1920))
        self.assertEqual([n.start_tick for n in notes], [0, 3840, 2880])

    def test_a_signature_change_ends_the_bar_in_progress_where_it_lands(self) -> None:
        # The rule `app/src/time.ts` and the bar view both keep: the event carries a tick and
        # not a bar, so moving it to the next bar line would put the grid somewhere the
        # document does not.
        compile = request("note(60, bar(1), 1)\nnote(61, bar(2), 1)\n")
        compile.signature = [
            TimeSignatureEvent(id="01S0", tick=0, numerator=4, denominator=4),
            TimeSignatureEvent(id="01S1", tick=2400, numerator=3, denominator=4),
        ]
        self.assertEqual([n.start_tick for n in dsl.run(compile)], [2400, 5280])

    def test_an_unusable_signature_falls_back_to_four_four_rather_than_looping(self) -> None:
        # §4.4 has a rule for tempo and none for the signature, so `0/0` is a document the
        # validator accepts and a bar of zero ticks is a loop that never ends.
        compile = request("note(60, bar(1), 1)\n")
        compile.signature = [TimeSignatureEvent(id="01S0", tick=0, numerator=0, denominator=0)]
        self.assertEqual(dsl.run(compile)[0].start_tick, 3840)

    def test_tempo_at_is_an_exact_fraction_of_the_double_that_crossed(self) -> None:
        notes = dsl.run(
            request(
                "a = tempo_at(0)\n"
                "b = tempo_at(2000)\n"
                "note(60, 0, a.numerator // a.denominator)\n"
                "note(61, 0, b.numerator // b.denominator)\n"
            )
        )
        self.assertEqual([n.length_ticks for n in notes], [120, 140])
        space = dsl.namespace(request(""), [])
        self.assertEqual(space["tempo_at"](2000), Fraction(140.5))

    def test_signature_at_sections_params_and_clip_are_what_crossed(self) -> None:
        space = dsl.namespace(request("", start=1920, length=960), [])
        self.assertEqual(space["signature_at"](0), (4, 4))
        self.assertEqual(
            space["sections"], [("intro", 0, 3840), ("verse", 3840, 11520)]
        )
        self.assertEqual(space["params"], {"density": "3", "mode": "swing"})
        self.assertEqual((space["clip"].start, space["clip"].length), (1920, 960))
        self.assertEqual(space["PPQ"], 960)

    def test_the_namespace_is_adr_0024_s_table_and_nothing_else(self) -> None:
        # A name added here is additive and moves no golden; a name removed, or a name whose
        # meaning changes, bumps the DSL's version (ADR 0027 §1). This is the test that makes
        # either of those a deliberate act.
        self.assertEqual(
            sorted(dsl.namespace(request(""), [])),
            sorted(
                [
                    "note", "PPQ", "bar", "beat", "clip", "rng", "params", "sections",
                    "tempo_at", "signature_at",
                    "Fraction", "int", "str", "bool", "len", "range", "enumerate", "zip",
                    "min", "max", "abs", "sum", "sorted", "reversed", "divmod", "pow",
                    "floor", "ceil", "round",
                ]
            ),
        )

    def test_rng_offers_the_six_draws_the_spike_measured_and_no_float(self) -> None:
        space = dsl.namespace(request(""), [])
        offered = {name for name in dir(space["rng"]) if not name.startswith("_")}
        self.assertEqual(
            offered, {"getrandbits", "randrange", "randint", "choice", "shuffle", "sample"}
        )
        self.assertIn("the name `random`", refusal("x = random()\n").message)

    def test_a_generator_that_writes_nothing_is_an_empty_list_and_not_a_failure(self) -> None:
        self.assertEqual(dsl.run(request("x = 1\n")), [])

    def test_the_subset_is_a_language_a_person_can_write_in(self) -> None:
        # The other direction from the table above, and it is the one an allowlist gets wrong
        # by being safe: everything ADR 0024 §3 *allows*, in one source. A list and a dict
        # comprehension, an f-string, a function with a docstring and a default, a `while`
        # with a `break`, an `AugAssign`, an annotated assignment, a slice, a negative index,
        # a conditional expression, and ten of the standard-library names.
        source = (
            'def swing(tick, amount=10):\n'
            '    """A docstring is a Constant str."""\n'
            '    return tick + amount if tick % 960 else tick\n'
            '\n'
            'KIT = {"kick": 36, "snare": 38}\n'
            'velocities = [60 + v for v in range(4)]\n'
            'pairs = {name: pitch for name, pitch in [("hat", 42)]}\n'
            'label = f"{KIT[\'kick\']}-{pairs[\'hat\']}"\n'
            'offsets = (0, 480, 960)\n'
            'total = 0\n'
            'n = 0\n'
            'while True:\n'
            '    n = n + 1\n'
            '    if n > 3:\n'
            '        break\n'
            '    total += n\n'
            'for index, offset in enumerate(offsets[0:3]):\n'
            '    note(KIT["kick"] if index == 0 else pairs["hat"], swing(offset), 240,'
            ' velocities[index])\n'
            'note(KIT["snare"], 0, min(total, 240), max(1, abs(-5)))\n'
            'x: int = sum([1, 2, 3])\n'
            'note(60, 0, divmod(x, 2)[0], len(label))\n'
            'note(61, 0, pow(2, 5), round(Fraction(300, 7)) % 128)\n'
            'note(62, 0, sorted(reversed([3, 1, 2]))[-1], ceil(Fraction(1, 2)) + floor(PPQ // 20))\n'
            'for a, b in zip([1], [2]):\n'
            '    note(63, 0, a + b, signature_at(0)[0] + tempo_at(0).numerator // 2)\n'
        )
        self.assertEqual(
            [(n.pitch, n.start_tick, n.length_ticks, n.velocity) for n in dsl.run(request(source))],
            [
                (36, 0, 240, 60),
                (42, 490, 240, 61),
                (42, 960, 240, 62),
                (38, 0, 6, 5),
                (60, 0, 3, 5),
                (61, 0, 32, 43),
                (62, 0, 3, 49),
                (63, 0, 3, 64),
            ],
        )


class Determinism(unittest.TestCase):
    """CLAUDE.md #3, in the milestone that first names `compilers` in it."""

    SOURCE = (
        "for b in range(8):\n"
        "    note(rng.randint(36, 48), beat(b), beat(1) // 2, rng.randrange(40, 120))\n"
    )

    def test_the_same_seed_twice_is_the_same_bytes(self) -> None:
        from escribass_proto.escribass.generate.v1 import Notes

        first = bytes(Notes(notes=dsl.run(request(self.SOURCE, seed=2**63 + 1))))
        second = bytes(Notes(notes=dsl.run(request(self.SOURCE, seed=2**63 + 1))))
        self.assertEqual(first, second)
        # A seed above 2^63 on purpose: it crosses JSON as a string and reaches
        # `random.Random` as the integer, and one that passed through a double on the way
        # would still compile — to different notes, silently (`generate.proto`, `seed`).
        self.assertEqual(
            hashlib.sha256(first).hexdigest(),
            hashlib.sha256(bytes(Notes(notes=dsl.run(request(self.SOURCE, seed=2**63 + 1))))).hexdigest(),
        )

    def test_a_changed_seed_changes_the_notes(self) -> None:
        one = dsl.run(request(self.SOURCE, seed=7))
        two = dsl.run(request(self.SOURCE, seed=8))
        self.assertEqual(len(one), len(two))
        self.assertNotEqual(
            [(n.pitch, n.velocity) for n in one], [(n.pitch, n.velocity) for n in two]
        )

    def test_a_seed_one_apart_at_the_top_of_the_range_still_changes_the_notes(self) -> None:
        one = dsl.run(request(self.SOURCE, seed=2**64 - 1))
        two = dsl.run(request(self.SOURCE, seed=2**64 - 2))
        self.assertNotEqual([n.pitch for n in one], [n.pitch for n in two])


class CrossPatchBytes(unittest.TestCase):
    """The spike's digest, as a check rather than as a sentence (ADR 0024 §8).

    `tests/spike/numeric_domain.py` on the never-merged `m4.0-spike` branch printed
    `1730edd3…` on CPython **3.11.14, 3.12.3, 3.12.12, 3.13.12 and 3.14.3** — five
    interpreters, one digest, only the recorded version string differing (docs/plan.md, M4 PR
    0, measurement 9). That is the whole of the evidence behind question 3's platform-free
    sentence, and the sentence is worth nothing if the next interpreter quietly disagrees.

    The computation below is the spike's, unchanged, so what fails here is the claim and not
    a rewrite of it. It is **not** written in the DSL: the spike uses `hashlib`, `json` and a
    `lambda`, none of which the DSL has. What it covers is the layer underneath — the seeded
    `rng`'s six draws, a 400-step `Fraction` chain reduced onto integer ticks through
    `__ceil__`, `__floor__` and `round`, and a block of `divmod`, `//`, `%` and three-argument
    `pow` — which is exactly what every value in the namespace above is made of.
    """

    SPIKE = "1730edd3f9825ec91c0b0456f477a15fcf2687d37c8d7224cc0f77a7bf741755"

    def test_the_numeric_domain_gives_the_digest_five_interpreters_gave(self) -> None:
        rng = random.Random(2**63 + 1)
        draws = [rng.getrandbits(32) for _ in range(64)]
        draws += [rng.randrange(0, 960) for _ in range(64)]
        draws += [rng.randint(21, 108) for _ in range(64)]
        draws += [rng.choice([1, 2, 3, 4, 6, 8, 12, 16]) for _ in range(64)]
        seq = list(range(32))
        rng.shuffle(seq)
        draws += seq
        draws += rng.sample(range(128), 32)
        rng_sha = hashlib.sha256(json.dumps(draws, separators=(",", ":")).encode()).hexdigest()

        f = Fraction(1, 3)
        chain = []
        for n in range(1, 400):
            f = (f + Fraction(n, n * n + 1)) * Fraction(n + 1, n + 2)
            if n % 40 == 0:
                f = Fraction(f.numerator % (10**24) or 1, f.denominator % (10**24) or 1)
            chain.append((f.numerator, f.denominator))
        ppq = 960
        ticks = [(Fraction(num, den) * ppq).__ceil__() % 100000 for num, den in chain[:64]]
        ticks += [(Fraction(num, den) * ppq).__floor__() % 100000 for num, den in chain[:64]]
        ticks += [round(Fraction(num, den) * ppq) % 100000 for num, den in chain[:64]]
        fraction_sha = hashlib.sha256(
            json.dumps([chain, ticks], separators=(",", ":")).encode()
        ).hexdigest()

        ints: list[int] = []
        for n in range(1, 200):
            q, r = divmod(n * 9973, 7919)
            ints += [q, r, (-n) // 7, (-n) % 7, pow(n, 17, 2**61 - 1)]
        pairs = sorted(((n % 7, n) for n in range(100)), key=lambda p: p[0])
        int_sha = hashlib.sha256(
            json.dumps([ints, pairs], separators=(",", ":")).encode()
        ).hexdigest()

        self.assertEqual(
            hashlib.sha256((rng_sha + fraction_sha + int_sha).encode()).hexdigest(), self.SPIKE
        )

    def test_the_digest_is_reachable_through_the_dsl_s_own_arithmetic(self) -> None:
        # The half of the spike the DSL can actually express, driven through the language
        # rather than beside it: the same six draws and the same `Fraction`-to-tick reduction,
        # written in the subset and compiled. It carries the `rng` and `Fraction` halves of
        # the digest above into the thing a generator author touches.
        source = (
            "for i in range(64):\n"
            "    note(60, rng.getrandbits(32) % 100000, 1)\n"
            "f = Fraction(1, 3)\n"
            "for n in range(1, 64):\n"
            "    f = (f + Fraction(n, n * n + 1)) * Fraction(n + 1, n + 2)\n"
            "    note(61, ceil(f * PPQ) % 100000, 1)\n"
            "    note(62, floor(f * PPQ) % 100000, 1)\n"
            "    note(63, round(f * PPQ) % 100000, 1)\n"
        )
        starts = [n.start_tick for n in dsl.run(request(source, seed=2**63 + 1))]
        draws = random.Random(2**63 + 1)
        self.assertEqual(starts[:64], [draws.getrandbits(32) % 100000 for _ in range(64)])
        self.assertEqual(
            hashlib.sha256(json.dumps(starts, separators=(",", ":")).encode()).hexdigest(),
            "5a9cd67fe9520aea768869670e06a09b325692462f0880a2357fe269557f1689",
        )


if __name__ == "__main__":
    unittest.main()
