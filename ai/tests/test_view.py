"""The projection golden, one language over (ADR 0018 §5; ADR 0012 §5; docs/specs.md §11).

What §11's fifth bullet means for the sidecar, and the whole of what it means here: **the view
and the six axes are pure functions of the model.** From a fixed `Song` each one renders to
text, and that text is compared against committed bytes. It runs under `unittest`, which is the
standard library's — as `node:test` was chosen over a framework for the frontend (ADR 0016 §3)
— and it needs no process, no socket and no key, unlike everything else in this directory.

The fixture is `tests/determinism/render/expected/song.json`, the same document
`app/tests/projection.test.ts` projects and for the same reason: it is a determinism golden, so
it came through the tool API like every other (CLAUDE.md #2), and it has something for a view
to get wrong — five tracks, four clips across three of them, one audio and two looping, a note
at tick 800 that is on no 16th or triplet grid, a note past its loop's end that never sounds, a
section running past the last clip, and a tempo change with three and a half thousand ticks of
music after it.

What it does **not** have is a voice striking two notes at one tick, so the paper's maximum
chord width and any simultaneity above 1 would golden as *none* and 1 for a right
implementation and a wrong one alike. ADR 0018 §5 therefore asks for that case, a second
time-signature event and a clip spanning a bar line to be asserted against constructed values,
and the three tests at the bottom are those. A constructed `Song` handed to a pure function is
not a document under test, so CLAUDE.md #2 is not in play (ADR 0018 §5).

Bless a deliberate change with:

    UPDATE_FIXTURES=1 uv run python -m unittest discover -s tests

and review the diff, exactly as `tests/AGENTS.md` requires of every other golden here: the
variable blesses whatever ran, including a deterministically wrong projection.
"""

from __future__ import annotations

import ast
import json
import os
import pathlib
import unittest

from escribass_ai import axes
from escribass_ai.view import bars, document_end, view
from escribass_schema.escribass.song.v1 import (
    Clip,
    Mix,
    Note,
    NoteClip,
    Song,
    TimeSignatureEvent,
    TimeSignatureMap,
    Track,
    TrackKind,
)

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
FIXTURE = ROOT / "tests" / "determinism" / "render" / "expected" / "song.json"
GOLDEN = HERE / "golden"

#: The seven pure functions of ADR 0018 §5, each with the file it is goldened against.
PROJECTIONS = {
    "view": view,
    "rhythm": axes.rhythm,
    "harmony": axes.harmony,
    "melody": axes.melody,
    "texture": axes.texture,
    "form": axes.form,
    "variation": axes.variation,
}


def document() -> dict:
    return json.loads(FIXTURE.read_text(encoding="utf-8"))


def reversed_keys(value):
    """The same document with every object's keys in the opposite order.

    A protobuf map decodes to a `dict`, and a `dict` yields insertion order — which for a song
    read from disk is the file's key order, lexical by ULID (ADR 0002 §4). A ULID sorts by the
    time it was minted, so in any fixture whose entities were created in the order they occur,
    "sorted by the model" and "whatever the map iterated" are the same list and a projection
    that dropped its `sort` would golden identically. That is what M2 PR 4 measured one language
    over, and it is why this exists.

    **Python reaches further than JavaScript does here**, and the difference is worth writing
    down. `app/tests/projection.test.ts` records that JavaScript re-sorts integer-like object
    keys numerically before any code runs, so its reversal is a no-op on a `ParamID` map and
    cannot separate the two orderings for one. Python does no such thing: `dict` preserves
    insertion order for every key, so reversing `{"40": …, "9": …}` really does hand the model a
    map that yields `9` first. Measured here, in `test_the_reversal_reaches_an_integer_keyed_map`,
    rather than assumed from the language's reputation.
    """
    if isinstance(value, list):
        return [reversed_keys(held) for held in value]
    if isinstance(value, dict):
        return {key: reversed_keys(held) for key, held in reversed(list(value.items()))}
    return value


def track(id_: str, name: str, index: int = 0) -> Track:
    return Track(id=id_, name=name, kind=TrackKind.INSTRUMENT, index=index, mix=Mix())


def note(id_: str, pitch: int, start: int, length: int) -> Note:
    return Note(id=id_, pitch=pitch, start_tick=start, length_ticks=length, velocity=100)


class TestTheProjectionGolden(unittest.TestCase):
    def setUp(self) -> None:
        self.document = document()
        self.song = Song.from_dict(self.document)

    def test_each_projection_is_the_committed_bytes(self) -> None:
        for name, projection in PROJECTIONS.items():
            with self.subTest(projection=name):
                at = GOLDEN / f"{name}.txt"
                actual = projection(self.song)
                if os.environ.get("UPDATE_FIXTURES") == "1":
                    at.parent.mkdir(exist_ok=True)
                    at.write_text(actual, encoding="utf-8")
                    continue
                self.assertEqual(actual, at.read_text(encoding="utf-8"))

    def test_a_projection_reads_the_document_and_writes_nothing(self) -> None:
        # Pure means pure: projecting must not touch what it read. `Song` is a pydantic model
        # with mutable maps on it, so this is a real possibility and not a formality — a
        # `setdefault` on `song.clips` while walking it would pass every other test here.
        before = self.song.to_dict()
        for name, projection in PROJECTIONS.items():
            with self.subTest(projection=name):
                once, twice = projection(self.song), projection(self.song)
                self.assertEqual(once, twice, "two projections of one song differ")
        self.assertEqual(self.song.to_dict(), before, "a projection wrote to the model")

    # Which test carries which ordering, measured by deleting each `sort` in turn — the only way
    # to know, and the same walk `app/tests/projection.test.ts` records for the frontend:
    #
    #   * The **golden alone** fails for a voice's clips (`00K` starts at tick 3840 and `00Q` at
    #     0, so the file's ULID order is not the model's) and for the effect chain (`00E` has
    #     `index` 5 and `00G` has 2). Those two the fixture catches on its own.
    #   * The **reversal alone** fails for the voice order, the automation lanes, a lane's
    #     points, a bar block's notes and the tempo map — five orderings where the fixture's
    #     entities were minted in the order they occur, so "sorted by the model" and "whatever
    #     the map iterated" are the same list until the keys are reversed. This is the gap ADR
    #     0012 §5 was amended for, five times over in one document.
    #   * **Neither** fails for the section list, the signature map or the audio iterations —
    #     the fixture has one of each, and a list of one has no order to get wrong. Nor for
    #     `view.strikes`'s own sort, because every axis re-orders what it reads; it is kept for
    #     the consumer that will not, and its docstring says so.
    def test_a_projection_is_a_function_of_the_document_not_of_how_its_maps_iterated(self) -> None:
        backwards = Song.from_dict(reversed_keys(self.document))
        for name, projection in PROJECTIONS.items():
            with self.subTest(projection=name):
                self.assertEqual(
                    projection(backwards),
                    projection(self.song),
                    "an order came from the map's iteration rather than from the model (ADR 0001 §3)",
                )

    def test_the_reversal_reaches_an_integer_keyed_map(self) -> None:
        # The premise of the test above, for the one map `app/tests/projection.test.ts` records
        # its own reversal cannot reach. Asserted rather than believed, because if Python ever
        # behaved as JavaScript does the reversal would silently stop covering `Instrument.params`
        # and `Effect.params` and this suite would not notice.
        parsed = Song.from_dict(
            reversed_keys(
                {
                    "tracks": {
                        "t": {
                            "id": "t",
                            "instrument": {
                                "id": "d",
                                "params": {"40": 0.25, "9": 0.5},
                            },
                        }
                    }
                }
            )
        )
        self.assertEqual(
            list(parsed.tracks["t"].instrument.params),
            ["9", "40"],
            "Python re-sorted an integer-like map key, as JavaScript does",
        )


class TestPurity(unittest.TestCase):
    """CLAUDE.md #3 names `core`, compilers and `engine`; ADR 0018 §5 puts these seven
    functions on the pure side of the line the M3 plan draws for `ai` at the model.

    The goldens above would catch a clock or a random number the first time they ran twice, and
    catch it as a mystery. This catches it at the import, where it is a sentence."""

    ALLOWED = {
        "__future__",
        "dataclasses",
        "fractions",
        "typing",
        "escribass_schema.escribass.song.v1",
        "view",  # the relative import the axes make of the shared reading of the document
    }

    def test_neither_module_imports_a_clock_a_random_number_or_the_world(self) -> None:
        import escribass_ai.axes
        import escribass_ai.view

        for module in (escribass_ai.view, escribass_ai.axes):
            with self.subTest(module=module.__name__):
                tree = ast.parse(pathlib.Path(module.__file__).read_text(encoding="utf-8"))
                imported = set()
                for node in ast.walk(tree):
                    if isinstance(node, ast.Import):
                        imported |= {alias.name for alias in node.names}
                    elif isinstance(node, ast.ImportFrom):
                        imported.add(node.module or "")
                self.assertLessEqual(
                    imported,
                    self.ALLOWED,
                    "a pure projection grew an import — a clock, a source of randomness, the"
                    " filesystem or the network would all arrive this way",
                )


class TestWhatTheFixtureCannotExercise(unittest.TestCase):
    """The three cases ADR 0018 §5 names, against constructed values.

    A golden cannot fail for an input it has never been given, and no `.escri` in this
    repository contains any of these: no tool mints a second time-signature event
    (`core/src/call.rs` has `set_tempo` and no `set_time_signature`), the render fixture's four
    clips all sit inside one bar, and nothing in it strikes two notes at one tick.
    """

    def test_a_bar_is_as_long_as_the_signature_in_force_says_it_is(self) -> None:
        changes = Song(
            id="signatures",
            time_signature_map=TimeSignatureMap(
                events={
                    "a": TimeSignatureEvent(id="a", tick=0, numerator=4, denominator=4),
                    "b": TimeSignatureEvent(id="b", tick=7680, numerator=3, denominator=4),
                }
            ),
            tracks={"t": track("t", "Line")},
            clips={
                "c": Clip(
                    id="c",
                    track_id="t",
                    start_tick=0,
                    length_ticks=13440,
                    note_clip=NoteClip(notes={"n": note("n", 60, 0, 480)}),
                )
            },
        )
        self.assertEqual(
            [tuple(bar) for bar in bars(changes, document_end(changes))],
            [(1, 0, 3840, 960), (2, 3840, 3840, 960), (3, 7680, 2880, 960), (4, 10560, 2880, 960)],
            "a constant ticks-per-bar would put every bar after the change in the wrong place",
        )

        # A signature event carries a tick, not a bar, so one landing mid-bar ends the bar in
        # progress. Moving it to the next bar line would put the grid where the document is not.
        mid_bar = Song(
            id="mid-bar",
            time_signature_map=TimeSignatureMap(
                events={
                    "a": TimeSignatureEvent(id="a", tick=0, numerator=4, denominator=4),
                    "b": TimeSignatureEvent(id="b", tick=5760, numerator=3, denominator=4),
                }
            ),
            tracks={"t": track("t", "Line")},
            clips={
                "c": Clip(
                    id="c",
                    track_id="t",
                    start_tick=0,
                    length_ticks=8640,
                    note_clip=NoteClip(notes={"n": note("n", 60, 0, 480)}),
                )
            },
        )
        self.assertEqual(
            [tuple(bar) for bar in bars(mid_bar, document_end(mid_bar))],
            [(1, 0, 3840, 960), (2, 3840, 1920, 960), (3, 5760, 2880, 960)],
        )

        # An empty document is zero bars and nothing is padded — the frontend draws eight so a
        # new project has a ruler; a view has no ruler and a bar block for an empty bar is
        # tokens spent saying nothing.
        empty = Song(id="empty")
        self.assertEqual(bars(empty, document_end(empty)), [])

    def test_a_clip_that_spans_a_bar_line_is_listed_under_both(self) -> None:
        # The rule this exercises is ADR 0018 §1's: a note is listed under the bar its
        # *absolute* onset falls in, with its clip-relative tick verbatim, and the clip's line
        # says where its tick 0 sits in *that* bar — which for a bar the clip started before is
        # a negative number, and both operands of the model's one subtraction are then on the
        # line. A view that listed every note under the clip's own bar would put `67@3360` in
        # bar 1, where nothing sounds at tick 3360.
        crossing = Song(
            id="crossing",
            tracks={"t": track("t", "Line")},
            clips={
                "c": Clip(
                    id="c",
                    track_id="t",
                    start_tick=1920,
                    length_ticks=3840,
                    note_clip=NoteClip(
                        notes={
                            "n1": note("n1", 60, 0, 480),
                            "n2": note("n2", 64, 1920, 480),
                            "n3": note("n3", 67, 3360, 480),
                        }
                    ),
                )
            },
        )
        self.assertEqual(
            view(crossing).split("notes are pitch")[1].split("\n\nautomation")[0],
            """@clip-tick>length vVelocity #id; a note's clip tick = its bar tick − where the clip's tick 0 sits

@1 tick 0 · 120 bpm · 4/4
  Line clip c tick 0 at bar tick 1920
    60@0>480 v100 #n1
@2 tick 3840
  Line clip c tick 0 at bar tick -1920
    64@1920>480 v100 #n2
    67@3360>480 v100 #n3""",
        )

    def test_chromaticism_maximises_over_the_roots_rather_than_assuming_C(self) -> None:
        # A fourth case, found the way the other three were: by breaking the code and watching
        # what stayed green. Every pitch class the render fixture sounds — C, E and G — is in C
        # major, so `max over the twelve roots` and `the scale on C` golden identically over it.
        # That is the one harmony value that could be wrong and pass, and it is the value ADR
        # 0018 §4 leans on to say the axis "needs no key because it maximises over the twelve".
        #
        # G, B and F# are G major and not C major: with a root of C the mass on the scale is
        # 1920 of 2880 and chromaticism is 1/3; maximised over the roots it is 0.
        in_g = Song(
            id="in-g",
            tracks={"t": track("t", "Line")},
            clips={
                "c": Clip(
                    id="c",
                    track_id="t",
                    start_tick=0,
                    length_ticks=3840,
                    note_clip=NoteClip(
                        notes={
                            "n1": note("n1", 67, 0, 960),
                            "n2": note("n2", 71, 960, 960),
                            "n3": note("n3", 66, 1920, 960),
                        }
                    ),
                )
            },
        )
        self.assertIn(
            "@1 F# 960, G 960, B 960 · 3 classes · prominent F#, G, B · chromaticism 0\n",
            axes.harmony(in_g),
            "chromaticism read one root rather than the best of twelve",
        )

    def test_a_voice_striking_two_notes_at_one_tick(self) -> None:
        # The case the fixture cannot reach at all: its only coincidence is tick 960, where Keys
        # and Lead sound together — two *different* voices, which is why the paper's within-voice
        # chord width gives none there. Over the fixture alone, `widest chord none` and
        # `mean simultaneity 1` are what a right implementation and an implementation that never
        # looked would both golden.
        chord = Song(
            id="chord",
            tracks={"t": track("t", "Keys")},
            clips={
                "c": Clip(
                    id="c",
                    track_id="t",
                    start_tick=0,
                    length_ticks=3840,
                    note_clip=NoteClip(
                        notes={
                            "n1": note("n1", 60, 0, 960),
                            "n2": note("n2", 64, 0, 960),
                            "n3": note("n3", 67, 0, 960),
                            "n4": note("n4", 62, 1920, 960),
                        }
                    ),
                )
            },
        )
        self.assertEqual(
            axes.texture(chord),
            "texture · a voice sounds when a note or an audio iteration does,"
            " and an audio iteration is one opaque event\n"
            "@1 1 voice · 4 notes over 2 onsets · mean simultaneity 2 · widest chord 7\n"
            "  Keys polyphony 3 · sounding 1920 of 3840 = 1/2\n",
        )
        # Four notes and two onsets, so the rhythm axis counts the triad once: the paper's 𝒪 is
        # a distinct (voice, tick) pair, and this is the only test in the repository that can
        # tell that definition from "one onset per note".
        self.assertIn("@1 2 onsets ·", axes.rhythm(chord))
        # And the line takes the highest of the three, which is a stated rule and not a detector.
        self.assertIn("@1 range 60–67 (7) · 4 distinct · intervals -5 ·", axes.melody(chord))


if __name__ == "__main__":
    unittest.main()
