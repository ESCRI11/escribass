#!/usr/bin/env python3
"""The recipe's own guards, each made to fire (ADR 0031; docs/plan.md, M5 traps 3, 4 and 5).

    python3 -I compilers/dsp/test_recipe.py      after `recipe.py checkout` and `recipe.py build`

What an export does in the engine is `tests/dsp.rs`'s. This is what the recipe refuses before
anything is built: an upstream template that moved, a configure that would fetch, a level that
claims to be an input. Each needs `cmaj` and none needs a compile, so the file costs seconds.
"""

import socket
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import recipe  # noqa: E402

PATCH = HERE.parent.parent / "tests" / "fixtures" / "dsp" / "syn" / "syn.cmajorpatch"


def offline(*argv):
    """`argv` in a user and network namespace of its own: no interface but a loopback that is
    down, so nothing beyond this machine — or on it — can be reached."""
    return subprocess.run(["unshare", "-rn", *argv], capture_output=True, text=True)


class Recipe(unittest.TestCase):
    def setUp(self):
        if not recipe.CMAJ.is_file():
            self.fail(f"no {recipe.CMAJ}: run `recipe.py checkout` and `recipe.py build` first")
        self.scratch = tempfile.TemporaryDirectory(prefix="escribass-recipe-")
        self.dir = Path(self.scratch.name)

    def tearDown(self):
        self.scratch.cleanup()

    def generated(self, name="src"):
        src = self.dir / name
        recipe.generate(PATCH, src)
        return src

    def test_the_edits_apply_once_to_what_cmaj_generates(self):
        src = self.generated()
        recipe.apply_edits(src / "CMakeLists.txt", recipe.CMAKE_EDITS)
        recipe.apply_edits(src / "helpers" / "clap" / "cmaj_CLAPPlugin.h", recipe.HELPER_EDITS)
        text = (src / "CMakeLists.txt").read_text()
        self.assertIn("set(CLAP_WRAPPER_DOWNLOAD_DEPENDENCIES FALSE)", text)
        self.assertNotIn("DOWNLOAD_DEPENDENCIES TRUE", text)
        # The VST3 target is now outside every `if (APPLE)`.
        before_apple = text.split("if (CLAP_WRAPPER_PATH)\n")[2].split("    if (APPLE)")[0]
        self.assertIn("target_add_vst3_wrapper", before_apple)
        helper = (src / "helpers" / "clap" / "cmaj_CLAPPlugin.h").read_text()
        self.assertEqual(helper.count("decision 9"), 2)
        # No machine path is written into the tree: every SDK is given at configure.
        self.assertNotIn(str(recipe.REPO), text)

    def test_a_template_that_moved_stops_the_recipe(self):
        # Trap 3: an anchor upstream reworded matches nothing, and a recipe that carried on would
        # build the template as upstream left it — a `.clap` and no VST3, downloads on, a deaf
        # instrument, a deadlocking one. Each anchor, made to miss and made to occur twice.
        for path, edits in (("CMakeLists.txt", recipe.CMAKE_EDITS),
                            ("helpers/clap/cmaj_CLAPPlugin.h", recipe.HELPER_EDITS)):
            for name, anchor, _ in edits:
                for drift, doctored in (("moved", anchor.replace("(", "( ", 1).replace("    ", "  ", 1)),
                                        ("doubled", anchor + anchor)):
                    with self.subTest(edit=name, drift=drift):
                        src = self.generated(f"{name}-{drift}".replace(" ", "_").replace("'", ""))
                        target = src / path
                        text = target.read_text()
                        self.assertEqual(text.count(anchor), 1, "the fixture is the real template")
                        target.write_text(text.replace(anchor, doctored))
                        with self.assertRaisesRegex(recipe.RecipeError, f"“{name}” occurs [02] times"):
                            recipe.apply_edits(target, edits)

    def test_cmajors_optimisation_level_is_not_an_input_of_an_export(self):
        # ADR 0031 §5: measured, not assumed, and re-measured on every run, so a Cmajor re-pin
        # whose `--target=clap` starts reading `-O` fails here rather than in a golden.
        trees = {}
        for level in ("", "-O0", "-O4"):
            out = self.dir / f"gen{level or '-default'}"
            subprocess.run([str(recipe.CMAJ), "generate", "--target=clap", *([level] if level else []),
                            f"--output={out}", str(PATCH)], check=True, capture_output=True)
            trees[level] = {p.relative_to(out).as_posix(): p.read_bytes()
                            for p in sorted(out.rglob("*")) if p.is_file()}
        self.assertGreater(len(trees[""]), 100, "a whole generated tree, helpers and headers too")
        self.assertEqual(trees[""], trees["-O0"])
        self.assertEqual(trees[""], trees["-O4"])

    def test_a_configure_reaches_no_network_and_needs_none(self):
        # Trap 4: a configure with working internet proves nothing about offline closure. The
        # namespace is shown to be unreachable before it is trusted to prove anything.
        probe = offline(sys.executable, "-I", "-c",
                        "import socket; socket.create_connection(('1.1.1.1', 443), timeout=3)")
        self.assertNotEqual(probe.returncode, 0, "the namespace reached the network")
        self.assertIn("unreachable", probe.stderr)

        src = self.generated()
        recipe.apply_edits(src / "CMakeLists.txt", recipe.CMAKE_EDITS)
        ran = offline(sys.executable, "-I", "-c",
                      f"import sys; sys.path.insert(0, {str(HERE)!r}); import recipe; "
                      f"recipe.configure({str(src)!r}, {str(self.dir / 'build')!r})")
        self.assertEqual(ran.returncode, 0, ran.stderr)
        self.assertTrue((self.dir / "build" / "build.ninja").is_file())
        self.assertNotIn("including CPM", (self.dir / "configure.log").read_text())

    def test_a_missing_sdk_fails_instead_of_fetching(self):
        # With the network up, so that fetching is possible and the only thing stopping it is
        # the recipe: the VST3 SDK root given as an empty directory must be the end of it.
        src = self.generated()
        recipe.apply_edits(src / "CMakeLists.txt", recipe.CMAKE_EDITS)
        empty = self.dir / "no-vst3sdk"
        empty.mkdir()
        with self.assertRaisesRegex(recipe.RecipeError, "exited 1"):
            recipe.configure(src, self.dir / "build", vst3_sdk=empty)
        said = (self.dir / "configure.log").read_text()
        # clap-wrapper's own refusal, at its signal file, naming the root it was given.
        self.assertIn(f"There is no VST3 SDK at {empty}", " ".join(said.split()),
                      "refused for the SDK, and not for something else")
        self.assertNotIn("including CPM", said)
        self.assertEqual(sorted(p.name for p in (self.dir / "build").rglob("cpm")), [],
                         "nothing was fetched into the build tree")


if __name__ == "__main__":
    unittest.main(verbosity=2)
