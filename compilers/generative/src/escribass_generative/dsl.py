"""The DSL: a subset of Python over integers and fractions, and the one place a source runs.

Three things are here and nothing else is: the `ast` allowlist that refuses a source **before
it executes**, the namespace it is executed with, and [`run`], which does both and hands back
notes. The process around it — the socket, the limits, the one `Compile` — is
[`escribass_generative`]; keeping the language in a module of its own is what lets every
refusal below be fed by a unit test with no socket and no subprocess (ADR 0024 §8).

**It is a subset of Python, not a dialect** (ADR 0024 §3). Everything accepted means what
CPython means by it, so a model that knows Python needs no second grammar — and everything
refused is refused by *absence*, with the node's name and its line, rather than by a rule
somebody has to have thought of. The two halves of that:

  * **The allowlist.** Every node of the parsed source is compared against [`_ALLOWED`] and
    anything outside it is a refusal. `import`, `class`, `lambda`, `try`, `raise`, `with`,
    `global`, `nonlocal`, `yield`, `await`, every `async`, `match`, `*` unpacking, `del`,
    `assert`, `:=`, `@`, `/`, a set literal, a set comprehension, a generator expression and a
    `float`, `complex` or `bytes` constant are all outside it. So is an attribute beginning
    with `_`, which is how `().__class__.__bases__` is reached in every `eval` escape ever
    written, a decorator, a `*args` or `**kwargs` parameter, and any free name that is not in
    the namespace below.
  * **The namespace is the builtins.** The source is executed with `__builtins__` bound to
    exactly the dict [`namespace`] returns, so there is no `__import__` for an `import` to
    use even if its node were allowed, and no `open`, `eval`, `exec`, `getattr` or `globals`
    to reach around the list with. Two locks on the same door, which is the shape ADR 0024 §3
    chose deliberately for `set` and which costs nothing to keep for the rest.

**What this is not.** Not a security boundary (ADR 0024 §4). An `ast` allowlist has been
escaped before; what it is built against is a sloppy author — a model reaching for
`import random` and `math.sin` — and what it buys is *purity by construction*: there is no
name for a clock, a file, a socket or an environment variable, so a generator cannot read one.
That is CLAUDE.md #3 held by absence rather than by discipline. A resource limit, which is the
other half of what "sandbox" claims here, belongs to the process and is in
[`escribass_generative`].

**No `float`, anywhere** (ADR 0024 §3), for ADR 0018 §4's reason: a transcendental's last bit
is the platform's and a byte-compared golden cannot carry it. Held twice, as `set` is. A float
literal is refused at parse and so is `/`, because `bar` and `beat` return an `int` and
CPython's `int / int` is float division — `beat(1) / 3` is `320.0`, not the `Fraction(320, 1)`
ADR 0024 §3's worked example claimed, which is what the amendment of 2026-10-01 corrects. And
every value that reaches a note is checked anyway: `pow(2, -1)` and `2 ** -1` are floats
CPython hands an author whatever an allowlist says about literals, and [`_whole`] refuses one
naming the argument.
"""

from __future__ import annotations

import ast
import math
import random
import resource
from fractions import Fraction
from types import TracebackType
from typing import Any, NamedTuple

from escribass_proto.escribass.generate.v1 import CompileRequest
from escribass_schema.escribass.song.v1 import Note

__all__ = ["PPQ", "SOURCE", "Refused", "namespace", "run"]

#: Ticks to the quarter note, fixed by `schema_version` 1 (docs/specs.md §4.2).
PPQ = 960

#: The filename a source is compiled under. It names no file — the child reads none — and it
#: is what a traceback is searched for to find the author's line rather than ours.
SOURCE = "<generator>"

# What a document that lost an event means, and the fallback for one that kept an unusable
# one. `app/src/time.ts` and `ai/src/escribass_ai/view.py` make the same two choices for the
# same two reasons: §4.4 has a rule for tempo and none for the signature, so `0/0` is a
# document the validator accepts and a bar of zero ticks is a loop that never ends.
_OPENING_SIGNATURE = (4, 4)
_OPENING_TEMPO = Fraction(120)


class Refused(Exception):
    """A refusal with a position: the author's to fix, and the whole of what crosses back.

    One exception type for every way a compile can be the caller's fault, because the wire has
    one refusal shape — a `Diagnostic`, whose rule is `core`'s `generator_error` (ADR 0026 §1).
    A rule id invented here would be a second error vocabulary beside the one every other
    refusal in this system comes from (ADR 0006 §2), so this carries a line, a column and the
    child's own words and names no rule.

    Both positions are **1-based**, as `SyntaxError` reports them and one more than `ast`'s
    `col_offset`; `0` means there is none, which is the ordinary case for an exception raised
    at run time (`proto/generate.proto`, `Diagnostic`).
    """

    def __init__(self, message: str, line: int = 0, column: int = 0) -> None:
        super().__init__(message)
        self.message = message
        self.line = line
        self.column = column


# ---------------------------------------------------------------------------
# The allowlist
# ---------------------------------------------------------------------------

_ALLOWED = frozenset(
    {
        # ADR 0024 §3's list, verbatim and in its order.
        "Module", "Expr", "Assign", "AugAssign", "AnnAssign",
        "For", "While", "If", "Break", "Continue", "Pass",
        "FunctionDef", "Return", "Call", "Name", "Constant",
        "BinOp", "UnaryOp", "BoolOp", "Compare", "IfExp",
        "List", "Tuple", "Dict", "ListComp", "DictComp",
        "Subscript", "Slice", "Attribute", "JoinedStr", "FormattedValue",
        # The grammar's glue: the nodes `ast` puts between the ones above, which ADR 0024 §3
        # does not list because they are not constructs an author writes — a `Load`, an `Add`,
        # a `for` clause inside a comprehension. They are listed rather than exempted, because
        # an exemption is a hole shaped like a rule nobody re-reads: `MatMult` (`@`), `Div`
        # (`/`) and `Del` are deliberately absent from this half, and `del` from the half
        # above. `/` is absent because it is the one operator that **makes a float** —
        # `960 / 3` is `320.0` and not `Fraction(320, 1)`, whatever an author expects — and
        # ADR 0024 §3's rule is that the author writes `a // b` or `Fraction(a, b)` instead
        # (ADR 0024 §3, amended 2026-10-01 where its worked example said otherwise).
        "Load", "Store", "arguments", "arg", "keyword", "comprehension",
        "Add", "Sub", "Mult", "FloorDiv", "Mod", "Pow",
        "LShift", "RShift", "BitOr", "BitXor", "BitAnd",
        "UAdd", "USub", "Not", "Invert",
        "And", "Or",
        "Eq", "NotEq", "Lt", "LtE", "Gt", "GtE", "Is", "IsNot", "In", "NotIn",
    }
)

# A readable name beside the class name, for the nodes whose class name alone would not tell
# an author what they wrote. The class name is always in the message too: it is what ADR 0024
# §3 promises ("refused with the node's name and its line") and what a test asserts on.
_PHRASE = {
    "Import": "`import`",
    "ImportFrom": "`from … import`",
    "ClassDef": "`class`",
    "Lambda": "`lambda`",
    "Try": "`try`",
    "TryStar": "`try` with `except*`",
    "Raise": "`raise`",
    "With": "`with`",
    "Global": "`global`",
    "Nonlocal": "`nonlocal`",
    "Yield": "`yield`",
    "YieldFrom": "`yield from`",
    "Await": "`await`",
    "AsyncFunctionDef": "`async def`",
    "AsyncFor": "`async for`",
    "AsyncWith": "`async with`",
    "Starred": "`*` unpacking",
    "Set": "a set literal",
    "SetComp": "a set comprehension",
    "GeneratorExp": "a generator expression",
    "Delete": "`del`",
    "Del": "`del`",
    "Assert": "`assert`",
    "NamedExpr": "`:=`",
    "MatMult": "`@`",
    "Div": "`/`",
    "Match": "`match`",
}

# What to write instead, where there is one obvious answer and the absence would otherwise
# read as an oversight.
_INSTEAD = {
    "Div": ": write a // b, or Fraction(a, b) for an exact ratio",
    "Set": ": the DSL has no set, because str hashing is randomised per process; a list or a"
    " dict keeps the source's order",
    "SetComp": ": the DSL has no set, because str hashing is randomised per process; a list"
    " comprehension keeps the source's order",
}

#: What a `Constant` may hold. ADR 0024 §3: a value is an `int`, a `Fraction`, a `str`, a
#: `bool` or `None` — and a `Fraction` is built by a call, never written as a literal.
_CONSTANTS = (int, str, bool, type(None))


def _bound(tree: ast.AST) -> set[str]:
    """Every name the source itself binds, anywhere in it.

    Deliberately scope-blind: a name bound inside a function counts as bound at module level
    too. The allowlist's question is whether a name reaches **outside** the DSL, not whether
    the author's scoping is right — a genuine unbound local is CPython's `NameError` at run
    time, which comes back as a refusal with its line like any other exception.
    """
    names: set[str] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Name) and not isinstance(node.ctx, ast.Load):
            names.add(node.id)
        elif isinstance(node, ast.FunctionDef):
            names.add(node.name)
        elif isinstance(node, ast.arg):
            names.add(node.arg)
    return names


def _refusals(tree: ast.AST, free: frozenset[str]) -> list[Refused]:
    """Every node of the source weighed against the allowlist, with its position.

    Iterative rather than recursive, and that is not a style choice: a source is an author's
    text, `x = 1 + 1 + …` twenty thousand times over is a tree twenty thousand deep, and a
    recursive walk over it would raise `RecursionError` **outside** the refusal path — which
    `core` would report as `generator_failed`, an operator error, for something that is
    plainly the author's to fix. Found by feeding one.
    """
    found: list[Refused] = []
    # Position carried down from the parent: a node either has both of its own or neither.
    stack: list[tuple[ast.AST, int, int]] = [(tree, 0, 0)]
    while stack:
        node, line, column = stack.pop()
        # Statements and expressions carry a position; operators and contexts do not, and
        # inherit, so `x @ y` is reported at `x` rather than at line 0.
        if hasattr(node, "lineno"):
            line, column = node.lineno, node.col_offset + 1
        name = type(node).__name__
        if name not in _ALLOWED:
            found.append(
                Refused(
                    f"{_PHRASE.get(name, name)} ({name}) is not in the generator DSL"
                    f"{_INSTEAD.get(name, '')} (ADR 0024 §3)",
                    line,
                    column,
                )
            )
        elif name == "Constant" and type(node.value) not in _CONSTANTS:  # type: ignore[attr-defined]
            kind = type(node.value).__name__  # type: ignore[attr-defined]
            found.append(
                Refused(
                    f"a {kind} constant is not in the generator DSL: its numbers are int and"
                    " Fraction, so write Fraction(a, b) or a // b (ADR 0024 §3)",
                    line,
                    column,
                )
            )
        elif name == "Attribute" and node.attr.startswith("_"):  # type: ignore[attr-defined]
            found.append(
                Refused(
                    f"the attribute `{node.attr}` begins with `_`, which is not in the"  # type: ignore[attr-defined]
                    " generator DSL (ADR 0024 §3)",
                    line,
                    column,
                )
            )
        elif name == "Name" and isinstance(node.ctx, ast.Load) and node.id not in free:  # type: ignore[attr-defined]
            found.append(
                Refused(
                    f"the name `{node.id}` is not in the generator DSL's namespace"  # type: ignore[attr-defined]
                    " (ADR 0024 §3)",
                    line,
                    column,
                )
            )
        elif name == "FunctionDef" and node.decorator_list:  # type: ignore[attr-defined]
            found.append(
                Refused(
                    f"a decorator on `{node.name}` is not in the generator DSL"  # type: ignore[attr-defined]
                    " (ADR 0024 §3)",
                    line,
                    column,
                )
            )
        elif name == "arguments" and (node.vararg or node.kwarg):  # type: ignore[attr-defined]
            star = node.vararg or node.kwarg  # type: ignore[attr-defined]
            found.append(
                Refused(
                    f"the parameter `{star.arg}` is a `*` or `**` parameter, and the generator"
                    " DSL takes positional and keyword parameters only (ADR 0024 §3)",
                    line,
                    column,
                )
            )
        for child in ast.iter_child_nodes(node):
            stack.append((child, line, column))
    return found


def check(tree: ast.AST, free: frozenset[str]) -> None:
    """Raises the **first** refusal in source order, or returns.

    First by position rather than first found, because `ast`'s traversal order is the
    grammar's and an author reads top to bottom. One refusal rather than all of them, because
    the wire carries one `Diagnostic` (ADR 0026 §1) and a model retries against the first
    thing it is told (ADR 0026 §3).
    """
    found = _refusals(tree, free)
    if found:
        raise min(found, key=lambda refusal: (refusal.line, refusal.column))


# ---------------------------------------------------------------------------
# The namespace (ADR 0024 §3's table, in its order)
# ---------------------------------------------------------------------------


class Clip(NamedTuple):
    """`clip` — the bounds the output must fit. `start` is absolute, `length` is ticks."""

    start: int
    length: int


class Rng:
    """`rng` — a `random.Random` seeded with `Generator.seed`, with six draws and no seventh.

    A wrapper rather than the `random.Random` itself, because `random.Random` carries
    `random()`, `uniform()` and `gauss()`, all of which return a float — and a float is the
    one thing that would make a golden the platform's rather than the model's (ADR 0024 §3).
    The six below are the six the spike measured to one digest across five CPython minors.
    """

    def __init__(self, seed: int) -> None:
        self._random = random.Random(seed)

    def getrandbits(self, k: int) -> int:
        return self._random.getrandbits(k)

    def randrange(self, start: int, stop: int | None = None, step: int = 1) -> int:
        return self._random.randrange(start, stop, step)

    def randint(self, a: int, b: int) -> int:
        return self._random.randint(a, b)

    def choice(self, sequence: Any) -> Any:
        return self._random.choice(sequence)

    def shuffle(self, sequence: Any) -> None:
        self._random.shuffle(sequence)

    def sample(self, population: Any, k: int) -> list[Any]:
        return self._random.sample(population, k)


def _whole(value: Any, where: str, low: int | None = None, high: int | None = None) -> int:
    """A value as a whole number of ticks, or a refusal naming the argument.

    An `int` is itself; an integral `Fraction` is its numerator; **a `Fraction` that is not
    integral is refused, never rounded** (ADR 0024 §3) — `Fraction(beat(1), 3)` is
    `Fraction(320, 1)` and legal, `Fraction(beat(1), 7)` is `Fraction(960, 7)` and is refused
    naming `start`. Anything else, a float included, is refused by its type: this is the
    second of the two locks on floats, the first being that `/` is outside the language, and
    it is the one that catches `2 ** -1`, which CPython hands an author whatever an allowlist
    says about literals.
    """
    if isinstance(value, Fraction):
        if value.denominator != 1:
            raise Refused(
                f"{where} is {value}, which is not a whole number of ticks; the DSL refuses"
                " a fraction of a tick rather than rounding it (ADR 0024 §3)"
            )
        value = int(value)
    if not isinstance(value, int):
        raise Refused(
            f"{where} is a {type(value).__name__}; the generator DSL's numbers are int and"
            " Fraction (ADR 0024 §3)"
        )
    if low is not None and not low <= value <= high:  # type: ignore[operator]
        raise Refused(f"{where} is {value}, which is outside {low}–{high} (ADR 0024 §3)")
    return value


def namespace(request: CompileRequest, notes: list[Note]) -> dict[str, Any]:
    """The whole of what a source can reach: ADR 0024 §3's table as a dict.

    It is handed to `exec` as `__builtins__`, so it is the *only* place a free name resolves.
    Additions to it are additive and move no golden; a name removed, or a name whose meaning
    changes, bumps the DSL's version (`pyproject.toml`; ADR 0027 §1), because a golden would
    move.
    """
    clip = Clip(request.clip_start_tick, request.clip_length_ticks)
    # In the order `core` built them, which ADR 0026 §1 fixes as tick order with ties by name.
    # Not re-sorted here: that order is part of the request `compiled_hash` is taken over
    # (ADR 0024 §6), so a child that quietly corrected it would hide the one thing a hash over
    # the request is for.
    tempo = list(request.tempo)
    signature = list(request.signature)

    def signature_at(tick: int) -> tuple[int, int]:
        at = _whole(tick, "signature_at(): tick")
        found = _OPENING_SIGNATURE
        for event in signature:
            if event.tick <= at:
                found = (int(event.numerator), int(event.denominator))
        return found if _span(found) is not None else _OPENING_SIGNATURE

    def tempo_at(tick: int) -> Fraction:
        at = _whole(tick, "tempo_at(): tick")
        found = _OPENING_TEMPO
        for event in tempo:
            # §4.4 requires tempo > 0; a document that broke it would divide by zero in every
            # consumer, so the opening value stands in, as it does in the two views.
            if event.tick <= at and event.bpm > 0:
                found = Fraction(event.bpm)
        return found

    def _walk(count: int, whole_bar: bool, where: str) -> int:
        """`bar(n)` and `beat(n)`: the n-th unit of **the clip**, counted from its start.

        Bar 0 and beat 0 are the clip's own tick 0, and each step takes the signature in force
        where it lands — walked event by event, as `app/src/time.ts` walks it, rather than
        from one `ticks_per_bar` computed once, which is right until a second signature event
        says otherwise and silently wrong after it. A signature event that lands mid-unit ends
        the unit in progress where it lands, for the reason the two views give: the event
        carries a tick and not a bar.
        """
        n = _whole(count, where, 0, 2**31 - 1)
        at = clip.start
        # ponytail: one step per unit, so `bar(10**9)` is a loop the CPU limit refuses rather
        # than an answer. The upgrade path is to advance in bulk between signature events,
        # which is worth writing the day a generator legitimately asks for a millionth bar.
        for _ in range(n):
            span = _span(signature_at(at), whole_bar)
            following = [event.tick for event in signature if event.tick > at]
            if following and at + span > min(following):
                span = min(following) - at
            at += span
        return at - clip.start

    def bar(n: int) -> int:
        return _walk(n, True, "bar(): n")

    def beat(n: int) -> int:
        return _walk(n, False, "beat(): n")

    def note(pitch: Any, start: Any, length: Any, velocity: Any = 100) -> None:
        """Adds a note to the output, at the clip-relative tick the DSL counts in.

        `id`, `provenance` and `version` stay empty: `core` mints the ids from its injectable
        source and `prepare` stamps the provenance on the way in (ADR 0024 §5). A sandbox that
        minted ids would stop `--seed-ids` being a pure function of the script.

        The ranges are `note`'s own argument contract (ADR 0024 §3's table) and are refused
        here because here is where the line number is. What the clip's **bounds** mean for a
        note is §4.4's rule and stays the validator's (ADR 0024 §5), so this refuses no note
        for where it lands.
        """
        notes.append(
            Note(
                pitch=_whole(pitch, "note(): pitch", 0, 127),
                start_tick=_whole(start, "note(): start"),
                length_ticks=_whole(length, "note(): length"),
                velocity=_whole(velocity, "note(): velocity", 1, 127),
            )
        )

    return {
        "note": note,
        "PPQ": PPQ,
        "bar": bar,
        "beat": beat,
        "clip": clip,
        "rng": Rng(request.seed),
        # A copy, so the author may rewrite it without the request the hash was taken over
        # changing underneath the compile that is reading it.
        "params": dict(request.params),
        "sections": [
            (section.name, section.start_tick, section.end_tick) for section in request.sections
        ],
        "tempo_at": tempo_at,
        "signature_at": signature_at,
        # The standard library's, unchanged. `floor` and `ceil` are `math`'s, which call
        # `Fraction.__floor__` and `__ceil__` — integer arithmetic throughout, and three of the
        # operations the spike's digest covers.
        "Fraction": Fraction,
        "int": int,
        "str": str,
        "bool": bool,
        "len": len,
        "range": range,
        "enumerate": enumerate,
        "zip": zip,
        "min": min,
        "max": max,
        "abs": abs,
        "sum": sum,
        "sorted": sorted,
        "reversed": reversed,
        "divmod": divmod,
        "pow": pow,
        "floor": math.floor,
        "ceil": math.ceil,
        "round": round,
    }


def _span(signature: tuple[int, int], whole_bar: bool = True) -> int | None:
    """A bar or a beat in ticks for one signature: `960 × 4 × n / d` and `960 × 4 / d`.

    `None` when the signature cannot describe a grid — `0/0`, or a denominator so large that a
    beat rounds to zero ticks, both of which the validator permits and both of which would be
    a loop that never ends. `signature_at` turns that into 4/4, as the two views do.
    """
    numerator, denominator = signature
    if numerator <= 0 or denominator <= 0:
        return None
    beat = PPQ * 4 // denominator
    bar = beat * numerator
    if beat < 1 or bar < 1:
        return None
    return bar if whole_bar else beat


# ---------------------------------------------------------------------------
# Running one
# ---------------------------------------------------------------------------


def _line(traceback: TracebackType | None) -> int:
    """The author's line, from the deepest frame of the traceback that is in their source."""
    line = 0
    while traceback is not None:
        if traceback.tb_frame.f_code.co_filename == SOURCE:
            line = traceback.tb_lineno
        traceback = traceback.tb_next
    return line


def run(request: CompileRequest) -> list[Note]:
    """Parses, refuses or executes one source, and returns the notes it wrote.

    Raises [`Refused`] for everything that is the author's to fix — the allowlist, a syntax
    error, an exception during execution, a non-integral tick, a pitch or velocity out of
    range, and the CPU limit arriving as a signal the process turned into one of these. There
    is no second kind: an operator error is something `core` decides about the child, never
    something the child says about itself (ADR 0024 §7).
    """
    try:
        tree = ast.parse(request.source, filename=SOURCE)
    except SyntaxError as broken:
        # `SyntaxError.offset` is already 1-based, unlike `ast`'s `col_offset`; it is not
        # converted, and the two call sites being different is why the 1-based promise is
        # written down in `proto/generate.proto` rather than left to be read off the code.
        raise Refused(f"{broken.msg}", broken.lineno or 0, broken.offset or 0) from None
    except (ValueError, RecursionError, MemoryError) as broken:
        # A source with a NUL byte in it, or one whose tree is deeper than the parser's own
        # stack — `x = 1 + 1 + …` twenty thousand times raises `RecursionError` from
        # `ast.parse` itself, with no position. All three are the author's, and none of them
        # may leave this function as anything but a refusal.
        raise Refused(f"{type(broken).__name__}: {broken}") from None

    notes: list[Note] = []
    space = namespace(request, notes)
    check(tree, frozenset(space) | frozenset(_bound(tree)))

    # `__builtins__` is the namespace and nothing else, which is the second of the two locks:
    # there is no `__import__` for an `import` to reach, and no `open`, `eval`, `exec`,
    # `getattr` or `globals` to reach around the allowlist with (ADR 0024 §3).
    scope: dict[str, Any] = {"__builtins__": space, "__name__": "generator"}
    try:
        exec(compile(tree, SOURCE, "exec"), scope)  # noqa: S102 — the whole point of the file
    except Refused as refused:
        # From `note()`, from `_whole`, or from the CPU limit's handler: it has the words and
        # may not have the line.
        refused.line = refused.line or _line(refused.__traceback__)
        raise
    except MemoryError as exhausted:
        line = _line(exhausted.__traceback__)
        # The notes are the likeliest thing holding the memory, and the answer still has to be
        # built. Dropped before anything else is attempted.
        notes.clear()
        scope.clear()
        limit = resource.getrlimit(resource.RLIMIT_AS)[0]
        raise Refused(
            f"MemoryError: the generator asked for more memory than the compiler's limit of"
            f" {limit // (1 << 20)} MiB allows (ADR 0024 §4)",
            line,
        ) from None
    except Exception as failed:  # noqa: BLE001 — every failure here is the author's
        raise Refused(f"{type(failed).__name__}: {failed}", _line(failed.__traceback__)) from None
    return notes
