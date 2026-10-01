"""The generated Python is real code, and it reaches *the* model rather than a copy of it.

`proto/codegen.sh --check` regenerates this tree and compares it byte for byte, which proves
it matches the `.proto` and proves nothing about whether it imports. This is the other half,
and it landed here rather than in M3 PR 5 because the user closed that deferral on
2026-09-23 (`docs/plan.md`, the ledger row of 2026-09-22).

Two things are asserted, and the second is the one worth the file.

**It imports.** The generated `AssistantBase` does `import grpclib` at module scope, so
nothing could import this tree until `grpclib` was pinned. That is now the case
(`lock.baseline.json`, `ai.grpclib`).

**`Song` is `escribass_schema`'s class.** `betterproto2-compiler` generates a module for every
file in the request, has no `extern_path`, and therefore re-emits `song.proto` beside the
service — a second `Song` that imports and type-checks perfectly well and is a different class
at run time. `codegen.sh` deletes that copy and rewrites the imports; an import check alone
would pass against the duplicate and prove nothing, because a duplicate imports just as
happily. So the identity is asserted through the field a `Prompt` actually carries
(CLAUDE.md #1, ADR 0006 §4; ADR 0020 §1, extended).

**Extended 2026-09-30, in M4 PR 3, for `Generate`.** The plan's row said this file's identity
check "already covers it"; it covered it in one direction and not the other. The rewrite in
`codegen.sh` is a `sed` over `escribass/*/v1/__init__.py`, whose glob reaches `generate/v1` by
matching rather than by anyone deciding it should — checked by regenerating without the guard,
which re-emits `escribass/song` and leaves `generate/v1` naming it `from ...song import v1` —
and the one thing not covered at all was the **server**: `compilers/generative` implements
`GenerateBase` in PR 4, and a `.proto` that generated a client and no server would compile,
import and pass every assertion here before being discovered there. So the mapping is asserted
as `Assistant`'s is, and the identity again through `CompileRequest`, which is the message
`Generator.compiled_hash` is the hash of (ADR 0024 §6).

Run from `schema/`'s environment, which is where the compiler and `grpclib` are pinned:

    cd schema && uv run python -m unittest discover -s ../proto/tests
"""

import importlib.util
import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]

# Both generated trees on the path, as `schema/tests/test_roundtrip.py` puts one there and for
# the same reason: neither package declares a `[build-system]` yet.
for tree in (ROOT / "schema" / "gen" / "python", ROOT / "proto" / "gen" / "python"):
    sys.path.insert(0, str(tree))

from escribass_proto.escribass.assistant.v1 import (  # noqa: E402
    AssistantBase,
    Prompt,
    ToolSchema,
)
from escribass_proto.escribass.generate.v1 import (  # noqa: E402
    CompileRequest,
    CompileResponse,
    GenerateBase,
    Notes,
)
from escribass_schema.escribass.song.v1 import Note, Song, TempoEvent  # noqa: E402


class TestTheServerStubIsReal(unittest.TestCase):
    def test_the_assistant_server_maps_the_one_rpc(self) -> None:
        # `__mapping__` is what grpclib's Server reads, so this is the shape PR 5 registers.
        mapping = AssistantBase().__mapping__()
        self.assertEqual(list(mapping), ["/escribass.assistant.v1.Assistant/Prompt"])
        handler = mapping["/escribass.assistant.v1.Assistant/Prompt"]
        self.assertEqual(handler.cardinality.name, "STREAM_STREAM")

    def test_the_generate_server_maps_the_one_rpc(self) -> None:
        # The stub `compilers/generative` subclasses in M4 PR 4 (ADR 0024 §8). A `.proto` that
        # generated a client and no server would compile, import, and be discovered in that
        # pull request rather than this one — `client_generation=none, server_generation=async`
        # in `buf.gen.yaml` is what decides it, and only a mapping proves it took.
        mapping = GenerateBase().__mapping__()
        self.assertEqual(list(mapping), ["/escribass.generate.v1.Generate/Compile"])
        handler = mapping["/escribass.generate.v1.Generate/Compile"]
        self.assertEqual(handler.cardinality.name, "UNARY_UNARY")
        self.assertIs(handler.request_type, CompileRequest)
        self.assertIs(handler.reply_type, CompileResponse)

    def test_an_answer_with_no_arm_is_not_zero_notes(self) -> None:
        # generate.proto, `CompileResponse.result`: a generator that legitimately emits nothing
        # answers `notes` with an empty list, so emptiness cannot mean failure. The two states
        # have to be distinguishable on the wire, or `core` reads a child that returned without
        # compiling as a successful compile of no notes — which is `RenderResult::decode(&[])`
        # returning `Ok`, one boundary over (M1).
        silent = CompileResponse(dsl_version="1", python_version="3.12.12", notes=Notes(notes=[]))
        compiled = CompileResponse.parse(bytes(silent))
        self.assertIsNotNone(compiled.notes, "a compile that emitted nothing still answered")
        self.assertEqual(compiled.notes.notes, [])

        nothing = CompileResponse.parse(bytes(CompileResponse()))
        self.assertIsNone(nothing.notes)
        self.assertIsNone(nothing.diagnostic)

    def test_a_tool_schema_carries_the_descriptors_json_text(self) -> None:
        # Not a `google.protobuf.Struct`: the host serialises what `core` built and `ai`
        # forwards it (assistant.proto, `ToolSchema.input_schema`).
        schema = ToolSchema(name="add_track", description="", input_schema='{"type":"object"}')
        self.assertEqual(ToolSchema.parse(bytes(schema)).input_schema, '{"type":"object"}')


class TestThereIsOneSong(unittest.TestCase):
    def test_a_prompt_carries_the_schema_packages_song(self) -> None:
        # Watched failing 2026-09-23 against a tree regenerated with the rewrite removed, and
        # it does not reach the assertion: pydantic refuses `escribass_schema`'s `Song` for a
        # field typed as the copy — "Input should be a dictionary or an instance of Song",
        # with the same name on both sides. That refusal *is* the defect, one milestone early.
        prompt = Prompt(text="raise the bass", song=Song(id="01ARZ3NDEKTSV4RRFFQ69G5FAV"))
        self.assertIs(
            type(prompt.song),
            Song,
            "a Prompt's `song` is a second Song class — codegen.sh's rewrite did not happen",
        )

    def test_a_compile_request_and_its_notes_carry_the_schema_packages_types(self) -> None:
        # The same assertion one service over, and it needs making again rather than inheriting
        # the one above: the rewrite in `codegen.sh` is a `sed` over `escribass/*/v1/__init__.py`
        # whose glob now matches a module it was not written for, and a compiler that spelled
        # this file's cross-package import differently would leave it matching nothing here
        # while still matching there. `CompileRequest` is what `Generator.compiled_hash` is the
        # hash of (ADR 0024 §6), so a second `TempoEvent` in it is a hash over a different type.
        request = CompileRequest(tempo=[TempoEvent(id="01ARZ3NDEKTSV4RRFFQ69G5FAV", bpm=120.0)])
        self.assertIs(type(request.tempo[0]), TempoEvent)
        self.assertIs(type(Notes(notes=[Note(pitch=60)]).notes[0]), Note)

    def test_the_generated_module_imports_the_model_from_escribass_schema(self) -> None:
        assistant = sys.modules["escribass_proto.escribass.assistant.v1"]
        self.assertIs(getattr(assistant, "__song__v1__").Song, Song)
        generate = sys.modules["escribass_proto.escribass.generate.v1"]
        self.assertIs(getattr(generate, "__song__v1__").Note, Note)

    def test_the_model_is_not_re_emitted_under_escribass_proto(self) -> None:
        for gone in (
            "escribass_proto.escribass.song",
            "escribass_proto.escribass.history",
            "escribass_proto.google",
        ):
            self.assertIsNone(
                importlib.util.find_spec(gone), f"{gone} is a second copy of the model"
            )


if __name__ == "__main__":
    unittest.main()
