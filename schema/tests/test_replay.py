"""ADR 0002 §11's claim, checked rather than asserted.

The patch log is ordinary RFC 6902 over ordinary JSON, so anything can replay it — not only
the code that wrote it. This walks the golden the determinism suite commits: ``refs.json``
names HEAD, entries are followed by first parent to the root, and their operations are applied
to the committed origin. The result must be the committed song.

The pointer apply below is about ten lines on purpose. A JSON Patch library would test the
library; ten lines test the claim, and adding a dependency to show that no dependency is
needed would be its own answer.
"""

import json
import pathlib
import unittest

GOLDEN = pathlib.Path(__file__).resolve().parents[2] / "tests" / "determinism"

SCRIPTS = ("every_tool", "refusals", "branches")


def golden(script: str, name: str):
    return json.loads((GOLDEN / script / "expected" / name).read_text())


def tokens(pointer: str) -> list[str]:
    # RFC 6901: ``~1`` becomes ``/`` and ``~0`` becomes ``~``, in that order — the reverse
    # turns ``~01`` into ``/`` instead of ``~1``.
    if pointer == "":
        return []
    return [t.replace("~1", "/").replace("~0", "~") for t in pointer[1:].split("/")]


def apply(document, ops):
    for op in ops:
        path = tokens(op["path"])
        if not path:
            assert op["op"] == "replace", f"cannot {op['op']} the whole document"
            document = op["value"]
            continue
        at = document
        for token in path[:-1]:
            at = at[token]
        last = path[-1]
        # RFC 6902 distinguishes these, and so does core's ``diff``. Treating ``replace`` as
        # assignment would accept a log a strict library rejects — which is the very thing
        # this file claims can read it.
        if op["op"] == "remove":
            assert last in at, f"remove of absent {op['path']}"
            del at[last]
        elif op["op"] == "replace":
            assert last in at, f"replace of absent {op['path']}"
            at[last] = op["value"]
        elif op["op"] == "add":
            at[last] = op["value"]
        else:
            raise AssertionError(f"the log should not contain `{op['op']}`")
    return document


def chain(script: str):
    """The entries from the root to HEAD, by first parent — the replay order `core` uses.

    Not every ancestor: a merge entry's operations are the diff from ``parents[0]``, so they
    already carry what the other side contributed. Replaying that side as well applies its
    changes twice, which is invisible for ``add`` and fatal for ``remove``.
    """
    refs = golden(script, "refs.json")
    entries = []
    at = refs["refs"][refs["head"]]
    while at:
        entry = golden(script, f"patches/{at}.json")
        entries.append(entry)
        at = entry["parents"][0] if entry["parents"] else None
    return list(reversed(entries))


class TestReplay(unittest.TestCase):
    def test_replays_the_committed_logs_into_the_committed_songs(self):
        for script in SCRIPTS:
            with self.subTest(script=script):
                self._replays(script)

    def _replays(self, script: str):
        entries = chain(script)
        self.assertGreater(len(entries), 1, "the golden has a log to replay")

        # Not ``{}``. A patch log is not self-contained: the canonical form emits every
        # no-presence scalar and map, so ``replace`` is legal from the first operation only
        # against a document that already has them. ``origin.json`` is that starting point.
        document = golden(script, "origin.json")
        for entry in entries:
            document = apply(document, entry["ops"])

        # Compared as a document, not as text: this side's serialiser is not the canonical
        # writer (ADR 0002 §4). That the bytes are canonical is Rust's claim and Rust's test.
        self.assertEqual(document, golden(script, "song.json"))

    def test_the_log_needs_no_schema_to_read(self):
        # Nothing above imported a generated type. The entries are plain JSON, which is what
        # §2.6 means by a project being readable and diffable, and what makes a log recoverable
        # by something that has never seen this schema.
        for entry in chain("branches"):
            for op in entry["ops"]:
                self.assertIn(op["op"], ("add", "replace", "remove"))
                self.assertTrue(op["path"] == "" or op["path"].startswith("/"))


if __name__ == "__main__":
    unittest.main()
