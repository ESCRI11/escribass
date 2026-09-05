"""Python half of the cross-language agreement test.

The AI orchestrator (docs/specs.md §6) reads the same project files `core` writes. Only
reading is asserted: `core` is the only writer (§5, §10), and this side's ``to_json`` is not
the canonical writer — it emits camelCase and a ``Z`` timestamp.
"""

import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "gen" / "python"))

import pydantic  # noqa: E402

from escribass_schema.escribass.song.v1 import Note, Song  # noqa: E402

FIXTURES = pathlib.Path(__file__).resolve().parents[2] / "tests" / "fixtures"


def fixture(name: str) -> str:
    return (FIXTURES / name).read_text()


class TestSongFixture(unittest.TestCase):
    def setUp(self) -> None:
        self.song = Song.from_json(fixture("song/minimal.json"))

    def test_reads_canonical_fixture(self) -> None:
        self.assertEqual(self.song.schema_version, 1)
        self.assertEqual(self.song.tracks["01M1FPMP00TRACKBASS0000002"].name, "Bass")
        self.assertEqual(
            self.song.tracks["01M1FPMP00TRACKBASS0000002"].instrument.ref.cmajor.source_hash,
            "8b31c0de4f9c00000000000000000000",
        )
        self.assertEqual(self.song.sections["01M1FPMP00SECTCHRS00000009"].start_tick, 61440)

    def test_tempo_events_are_keyed_not_positional(self) -> None:
        self.assertEqual(list(self.song.tempo_map.events), ["01M1FPMP00TEMP00000000000E"])
        self.assertEqual(self.song.tempo_map.events["01M1FPMP00TEMP00000000000E"].bpm, 92.0)
        self.assertEqual(
            self.song.time_signature_map.events["01M1FPMP00TMESG0000000000F"].numerator, 4
        )

    def test_oneof_and_optional_presence(self) -> None:
        clip = self.song.clips["01M1FPMP00CPCHRS0000000006"]
        self.assertEqual(clip.start_tick, 61440)
        self.assertIsNotNone(clip.note_clip)
        self.assertIsNone(clip.audio_clip)
        self.assertIsNone(clip.loop_length_ticks, "unset optional stays absent")
        self.assertEqual(len(clip.note_clip.notes), 2)
        self.assertEqual(clip.note_clip.notes["01M1FPMP00NTEG100000000007"].pitch, 43)
        self.assertEqual(clip.note_clip.notes["01M1FPMP00NTED200000000008"].expression["timbre"], 0.62)

    def test_audio_clip_gain_fades_and_stretch(self) -> None:
        clip = self.song.clips["01M1FPMP00CPGTR0000000000H"]
        self.assertIsNone(clip.note_clip)
        self.assertEqual(clip.audio_clip.asset_hash, "3f7a9c1e5b2d00000000000000000000")
        self.assertEqual(clip.audio_clip.gain_db, -4.5)
        self.assertEqual(clip.audio_clip.fade_in_ticks, 240)
        self.assertEqual(clip.audio_clip.fade_out_ticks, 480)
        self.assertIs(clip.audio_clip.time_stretch, True)

    def test_64_bit_seed_survives_the_json_string_encoding(self) -> None:
        self.assertEqual(self.song.generators["01M1FPMP00GENCHRS00000000D"].seed, 9007199254740993)

    def test_binary_round_trip(self) -> None:
        self.assertEqual(Song.parse(bytes(self.song)), self.song)


class TestParsingIsNotValidation(unittest.TestCase):
    """Pins where validation does and does not happen, so nobody relies on the wrong one.

    Constructors are pydantic-validated. ``from_json`` is not: it coerces. That is the path
    `ai` actually uses, so M0.2's validator is what has to catch these — not the model.
    """

    def test_constructor_rejects_wrong_types_and_out_of_range(self) -> None:
        with self.assertRaises(pydantic.ValidationError):
            Note(id="x", pitch="not a number")
        with self.assertRaises(pydantic.ValidationError):
            Note(id="x", start_tick=2**31)

    def test_from_json_coerces_instead_of_rejecting(self) -> None:
        self.assertEqual(Note.from_json('{"pitch": 43.9}').pitch, 43)
        self.assertEqual(Note.from_json('{"pitch": true}').pitch, 1)


if __name__ == "__main__":
    unittest.main()
