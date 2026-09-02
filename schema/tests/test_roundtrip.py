"""Python half of the cross-language agreement test.

The AI orchestrator (docs/specs.md §6) reads the same project files `core` writes, through
generated Pydantic models. Only parsing is asserted: `core` is the only writer (§5, §10).
"""

import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "gen" / "python"))

import pydantic  # noqa: E402

from escribass_schema.escribass.history.v1 import PatchEntry  # noqa: E402
from escribass_schema.escribass.song.v1 import Note, Song  # noqa: E402

FIXTURES = pathlib.Path(__file__).resolve().parents[2] / "tests" / "fixtures"


def fixture(name: str) -> str:
    return (FIXTURES / name).read_text()


class TestSongFixture(unittest.TestCase):
    def setUp(self) -> None:
        self.song = Song.from_json(fixture("song/minimal.json"))

    def test_reads_canonical_fixture(self) -> None:
        self.assertEqual(self.song.schema_version, 1)
        self.assertEqual(self.song.tracks["01K4F2T001"].name, "Bass")
        self.assertEqual(
            self.song.tracks["01K4F2T001"].instrument.ref.cmajor.source_hash,
            "8b31c0de4f9c00000000000000000000",
        )

    def test_oneof_and_optional_presence(self) -> None:
        clip = self.song.clips["01K4F2QN8B"]
        self.assertEqual(clip.start_tick, 61440)
        self.assertIsNotNone(clip.note_clip)
        self.assertIsNone(clip.audio_clip)
        self.assertIsNone(clip.loop_length_ticks, "unset optional stays absent")
        self.assertEqual(clip.note_clip.notes["01K4F2N001"].pitch, 43)
        self.assertEqual(clip.note_clip.notes["01K4F2N002"].expression["timbre"], 0.62)

    def test_64_bit_seed_survives_the_json_string_encoding(self) -> None:
        self.assertEqual(self.song.generators["01K4F2G001"].seed, 9007199254740993)

    def test_json_round_trip(self) -> None:
        self.assertEqual(Song.from_json(self.song.to_json()), self.song)

    def test_binary_round_trip(self) -> None:
        self.assertEqual(Song.parse(bytes(self.song)), self.song)


class TestPatchEntryFixture(unittest.TestCase):
    def test_ops_are_rfc_6902_operations(self) -> None:
        entry = PatchEntry.from_json(fixture("history/minimal.json"))
        self.assertEqual(entry.parents, ["01K4F2QN8B"])
        self.assertEqual(entry.ops[0].op, "replace")
        self.assertEqual(
            entry.ops[0].path, "/clips/01K4F2QN8B/note_clip/notes/01K4F2N001/pitch"
        )
        self.assertEqual(entry.ops[1].op, "remove")


class TestGeneratedModelsAreValidated(unittest.TestCase):
    """CLAUDE.md requires generated *Pydantic* models, so construction must validate."""

    def test_wrong_type_is_rejected(self) -> None:
        with self.assertRaises(pydantic.ValidationError):
            Note(id="x", pitch="not a number")

    def test_out_of_range_int32_is_rejected(self) -> None:
        with self.assertRaises(pydantic.ValidationError):
            Note(id="x", start_tick=2**31)


if __name__ == "__main__":
    unittest.main()
