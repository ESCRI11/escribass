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
  * **A value on its way out of the arithmetic is checked**, which is the third lock and the
    one the review of 2026-10-02 added (ADR 0024 §3, §4, amended; M4 PR 8). An allowlist reads
    *parsed source*, and three things happen in places a parse cannot see: `str.format`'s
    replacement fields are a second language inside a string constant, so
    `"{0.__globals__[random]._os.environ[HOME]}".format(note)` returned `$HOME` past the
    `_`-attribute rule; `repr` of anything whose repr is the default carries a heap address, so
    `f"{rng}"` was velocity 45 on one run of a source and velocity 3 on the next; and `**` is
    integer arithmetic until its exponent turns out to be a `Fraction`, after which it is
    `libm`. So `format` and `format_map` are refused by name, [`_shown`] decides what may
    become text, and [`_power`] and [`_modulo`] check the two operators' results — the last two
    through [`_guarded`], which rewrites the node into a call after the allowlist has run.

**What this is not.** Not a security boundary (ADR 0024 §4). An `ast` allowlist has been
escaped before; what it is built against is a sloppy author — a model reaching for
`import random` and `math.sin` — and what it buys is *purity by construction*: there is no
name for a clock, a file, a socket or an environment variable, and no route from an object in
the namespace to one, so a generator cannot read one. That is CLAUDE.md #3 held by absence
rather than by discipline — **and "no route" is a claim that was false until 2026-10-02**,
which is why the third lock above is written down as a lock and not as a tidy-up. A resource
limit, which is the other half of what "sandbox" claims here, belongs to the process and is in
[`escribass_generative`].

**No `float`, anywhere** (ADR 0024 §3), for ADR 0018 §4's reason: a transcendental's last bit
is the platform's and a byte-compared golden cannot carry it. Held three times now. A float
literal is refused at parse and so is `/`, because `bar` and `beat` return an `int` and
CPython's `int / int` is float division — `beat(1) / 3` is `320.0`, not the `Fraction(320, 1)`
ADR 0024 §3's worked example claimed, which is what the amendment of 2026-10-01 corrects.
`2 ** -1`, `2 ** Fraction(1, 2)` and `pow(2, Fraction(1, 2))` are the floats CPython hands an
author whatever an allowlist says about literals, and [`_power`] refuses them **where they are
made**: `int((2 ** Fraction(1, 2)) * 10**15) % 128` was a legal pitch until 2026-10-02, because
`int()` launders a float into a whole number before [`_whole`] can see it. And every value that
reaches a note is checked anyway, by [`_whole`], naming the argument.
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

#: The attributes refused by **name**, beside the `_` rule, and the one reason both have.
#:
#: `"{0.__globals__[random]._os.environ[HOME]}".format(note)` returns `$HOME`, and
#: `"{0.__globals__[__file__]}".format(note)` returns the install path. Neither is an escape
#: the allowlist missed: `str.format`'s replacement-field syntax is **a second language inside
#: a string constant**, and a walker over the parsed source cannot see into a string. So the
#: `_`-attribute rule — the one that stops `().__class__.__bases__` — does not apply to the
#: attribute chain a format field walks, and `format` and `format_map` hand an author arbitrary
#: attribute access with it. There is no narrowing of the field syntax that would be safe, so
#: the two methods that read it are outside the language and the f-string takes their place:
#: an f-string's values are expressions the allowlist *does* see, and [`_shown`] checks each one
#: (ADR 0024 §3, amended 2026-10-02; M4 PR 8).
#:
#: `%` is the third door into the same room and is shut at run time rather than here, because
#: `a % b` is integer arithmetic until `a` turns out to be a `str` — see [`_modulo`].
_SUBLANGUAGE = {
    "format": ": its replacement fields are a second language inside a string constant, which"
    " the allowlist cannot see into — `{0.__globals__[__file__]}` reaches the host through it."
    " Write an f-string, whose values the DSL checks",
    "format_map": ": its replacement fields are a second language inside a string constant,"
    " which the allowlist cannot see into — `{0.__globals__[__file__]}` reaches the host"
    " through it. Write an f-string, whose values the DSL checks",
}

#: What a `Constant` may hold. ADR 0024 §3: a value is an `int`, a `Fraction`, a `str`, a
#: `bool` or `None` — and a `Fraction` is built by a call, never written as a literal.
_CONSTANTS = (int, str, bool, type(None))

#: The two operators the DSL checks the **result** of, and the name each is rewritten to call.
#:
#: Neither can be checked by looking at the source: `a ** b` is integer arithmetic until `b`
#: turns out to be `Fraction(1, 2)` and `a % b` is integer arithmetic until `a` turns out to be
#: a `str`, and in both cases the operands are expressions. So [`_guarded`] rewrites the node
#: into a call to [`_power`] or [`_modulo`], **after** the allowlist has read the author's own
#: tree (ADR 0024 §3, amended 2026-10-02; M4 PR 8).
#:
#: **The names are not identifiers, on purpose.** `compile` takes any string as a `Name`'s id,
#: and `<pow>` is not something a source can write — so it is also not something a source can
#: *shadow*. A guard injected under `__pow__` would be disarmed by an author writing
#: `def __pow__(a, b):\n    return a ** b`, which the allowlist permits: it refuses an
#: *attribute* beginning with `_`, never a name. `tests/test_dsl.py` feeds exactly that source.
_GUARDED = {"Pow": "<pow>", "Mod": "<mod>"}

#: The name an f-string's values are checked under, by the same rewrite and for the same reason.
_SHOW = "<show>"


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
        elif name == "Attribute" and node.attr in _SUBLANGUAGE:  # type: ignore[attr-defined]
            found.append(
                Refused(
                    f"`.{node.attr}` is not in the generator DSL"  # type: ignore[attr-defined]
                    f"{_SUBLANGUAGE[node.attr]} (ADR 0024 §3)",  # type: ignore[attr-defined]
                    line,
                    column,
                )
            )
        elif name == "AugAssign" and type(node.op).__name__ in _GUARDED:  # type: ignore[attr-defined]
            operator = {"Pow": "**=", "Mod": "%="}[type(node.op).__name__]  # type: ignore[attr-defined]
            found.append(
                Refused(
                    f"`{operator}` is not in the generator DSL: `**` and `%` are the two"
                    f" operators whose **result** the DSL checks — one for a float, the other"
                    f" for a string being formatted — and an augmented assignment is not a"
                    f" form it checks. Write `x = x {operator[:-1]} n` (ADR 0024 §3)",
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
    naming `start`. Anything else — a `str`, a `list`, a `None`, and a float if one ever
    reappears — is refused by its type.

    It is the **last** of three locks on floats, not the one that catches them: `/` is outside
    the language, [`_power`] refuses the float where `**` makes it, and this is the boundary
    check that would catch one arriving from somewhere nobody has thought of. Until 2026-10-02
    it was the only lock after `/`, and `int()` walked around it (see the module note).
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


#: What the DSL will turn into text — at `str()`, in an f-string, and nowhere else.
#:
#: The leak is `repr`. A closure's default repr is `<function note at 0x7f…>`, a
#: `random.Random`'s is `<random.Random object at 0x…>`, a bound method's and a `zip` object's
#: are the same shape — and that address is where the allocator happened to put something,
#: which ASLR moves between two runs of **one** source. `f"{rng}"` compiled to velocity 45 and
#: then to velocity 3, and `str(note)` to a different length each run, so a generator whose
#: notes were a function of that text had a stable `compiled_hash` and unstable notes
#: (CLAUDE.md #3; docs/specs.md §11; ADR 0024 §4, amended 2026-10-02).
#:
#: This is the list rather than a denial of the known-leaky types, because the leaky set is
#: open: every object in the language has bound methods, and each of those has an address.
_SHOWABLE = (int, str, bool, type(None), Fraction)


def _shown(value: Any, where: str) -> Any:
    """`value` unchanged if its text is the source's, or a refusal naming the type it is not.

    Recurses into a `list`, a `tuple` and a `dict`, because `str([rng])` leaks exactly what
    `str(rng)` does. A `Clip` is a `NamedTuple` and so arrives as a tuple of ints, whose repr
    is its fields; `sections` and `params` are the same shape.
    """
    if isinstance(value, _SHOWABLE):
        return value
    if isinstance(value, (list, tuple)):
        for item in value:
            _shown(item, where)
        return value
    if isinstance(value, dict):
        for key, item in value.items():
            _shown(key, where)
            _shown(item, where)
        return value
    raise Refused(
        f"{where} of a {type(value).__name__} is not in the generator DSL: its text would be"
        " the host's rather than the source's — a function's, an iterator's and a bound"
        " method's default repr all carry a heap address, which moves between two runs of one"
        " source. The DSL shows an int, a Fraction, a str, a bool, None, and a list, tuple or"
        " dict of those (ADR 0024 §3)"
    )


def _text(value: Any) -> str:
    """`str`, as the namespace offers it: the standard library's, over a checked value."""
    return str(_shown(value, "str()"))


def _power(base: Any, exponent: Any, modulus: Any = None) -> Any:
    """`a ** b` and `pow(a, b)`: the one operation in the DSL that can make a float.

    `2 ** -1` is `0.5`, `2 ** Fraction(1, 2)` is `1.4142135623730951` and
    `(-8) ** Fraction(1, 3)` is a complex number — all three from `libm`, whose last bit is the
    platform's, and all three laundered into a tick by `int()` or `round()` before [`_whole`]
    ever sees a float (ADR 0024 §3, "no `libm` anywhere a golden can see"; found 2026-10-02).
    Refused **where it is made**, so no float exists in the language at all and nothing
    downstream has to know about one.
    """
    result = base**exponent if modulus is None else pow(base, exponent, modulus)
    if isinstance(result, (float, complex)):
        raise Refused(
            f"{base} ** {exponent} is {result}, a {type(result).__name__}: the DSL's numbers"
            " are int and Fraction, and this one came out of libm, whose last bit is the"
            " platform's rather than the source's. A negative integer exponent of an int, and"
            " any non-integral exponent, leave the language — write Fraction(a, b) for an exact"
            " ratio, and nothing at all for an irrational one (ADR 0024 §3)"
        )
    return result


def _modulo(left: Any, right: Any) -> Any:
    """`a % b`: the remainder of an `int` or a `Fraction`, and nothing else — because `%` formats.

    `"%r" % (note,)` is [`_shown`]'s leak reached through an operator rather than through a name.
    **And `bytes` is the same operator again**, which is why this checks for a number rather than
    against a `str`: `("%a".encode() % (note,)).decode()` compiled to velocity 53 on the first
    sweep *after* the `str` arm was written, because PEP 461 gives `bytes.__mod__` its own `%a`
    and `str` was the only type being refused. Two sublanguages, one operator, and a denial of
    the types that have one is a list that was already wrong once — so the DSL's `%` is numeric,
    text is the f-string's job, and the f-string is checked (ADR 0024 §3).
    """
    if not isinstance(left, (int, Fraction)):
        raise Refused(
            f"`%` on a {type(left).__name__} is not in the generator DSL: on a str or bytes it"
            " formats, and its conversions carry whatever repr the value has —"
            ' `"%r" % (rng,)` and `"%a".encode() % (rng,)` are both a heap address. The DSL\'s'
            " `%` is the remainder of an int or a Fraction; write an f-string for text, whose"
            " values the DSL checks (ADR 0024 §3)"
        )
    return left % right


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
        #
        # Two are **not** unchanged, and are the only two: `str` is the standard library's over
        # a checked value ([`_shown`]) and `pow` is it over a checked result ([`_power`]). Both
        # narrow what was accepted and neither changes what an accepted source means, so
        # `dsl_version` stays 1 (ADR 0027 §1; the user's decision, 2026-10-02).
        "Fraction": Fraction,
        "int": int,
        "str": _text,
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
        "pow": _power,
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


def _guarded(tree: ast.AST) -> ast.AST:
    """Rewrites the three places a value leaves the DSL's arithmetic into a checked call.

    An f-string's value, `a ** b` and `a % b`, each wrapped in a call to [`_shown`],
    [`_power`] or [`_modulo`] under a name no source can write. It runs **after** [`check`], so
    what the allowlist reads and what a refusal's line points at are the author's own tree, and
    it runs **whatever the allowlist said**, so a hole in the allowlist is not a way past these
    three (ADR 0024 §3's two locks, a third time).

    Iterative, for [`_refusals`]' reason: a source is an author's text and a recursive walk over
    a tree twenty thousand deep would raise outside the refusal path. A node is rewritten as a
    *field of its parent*, and the parent's children are read back after the rewrite, so
    `2 ** (3 % 4)` is guarded twice and `f"{2 ** x}"` three times.
    """
    stack: list[ast.AST] = [tree]
    while stack:
        node = stack.pop()
        for field, value in ast.iter_fields(node):
            if isinstance(value, list):
                for index, item in enumerate(value):
                    if isinstance(item, ast.AST):
                        value[index] = _wrap(item)
            elif isinstance(value, ast.AST):
                setattr(node, field, _wrap(value))
        stack.extend(ast.iter_child_nodes(node))
    # No `ast.fix_missing_locations` after this, and that is deliberate: it recurses, and this
    # function is iterative for a reason. Every node [`_call`] makes is given its position by
    # `copy_location`, which copies all four attributes `compile` wants, so there is nothing
    # left to fix — and a recursive pass over an author's twenty-thousand-deep tree would raise
    # where a refusal is expected.
    return tree


def _wrap(node: ast.AST) -> ast.AST:
    """One node, guarded if it is one of the three — and itself if it is not."""
    if isinstance(node, ast.FormattedValue):
        # The value, not the node: `!r`, the conversion and the format spec all take the value
        # as it is, so checking it on the way in covers `f"{rng!r}"` and `f"{rng:>10}"` too.
        node.value = _call(_SHOW, [node.value], node.value)
        return node
    if isinstance(node, ast.BinOp) and type(node.op).__name__ in _GUARDED:
        return _call(_GUARDED[type(node.op).__name__], [node.left, node.right], node)
    return node


def _call(name: str, arguments: list[ast.expr], at: ast.AST) -> ast.Call:
    """A call to one of the guards, at the position of the node it replaces."""
    function = ast.copy_location(ast.Name(id=name, ctx=ast.Load()), at)
    return ast.copy_location(ast.Call(func=function, args=arguments, keywords=[]), at)


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
    # `getattr` or `globals` to reach around the allowlist with (ADR 0024 §3). The three guards
    # beside it are the third lock, reachable only from the tree [`_guarded`] rewrote: their
    # names are not identifiers, so an author can neither call nor shadow one.
    scope: dict[str, Any] = {
        "__builtins__": space,
        "__name__": "generator",
        _SHOW: lambda value: _shown(value, "formatting"),
        _GUARDED["Pow"]: _power,
        _GUARDED["Mod"]: _modulo,
    }
    try:
        exec(compile(_guarded(tree), SOURCE, "exec"), scope)  # noqa: S102 — the point of the file
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
