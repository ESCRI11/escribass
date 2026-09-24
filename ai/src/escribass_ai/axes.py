"""The six descriptive axes the model reads beside the view (ADR 0018 §4).

Libretto's fingerprint is 29 axes over six families, each converted to a percentile against a
frozen 314-song corpus. This repository has no corpus, no key (U1) and no chord label, and an
uncalibrated percentile is a number with no meaning — so what follows is the six *families*,
taking the paper's own definitions wherever one needs neither a corpus, a key, a label nor a
transcendental function, and deferring the rest by name in the ADR rather than approximating
it. Every value here is an integer or an exact `fractions.Fraction`: an entropy or a standard
deviation reaches `libm`, whose last bit is the platform's, and a golden that compares bytes
cannot carry one (ADR 0009 §3's reason, one layer up).

**Nothing about an axis decides whether a proposal is applied.** A person does (docs/specs.md
§9). No value here is compared to a threshold, none appears in a `Violation`, none crosses to
`core`, and none is used to retry — because a model that reports success on the wrong parameter
would report success against a metric too, and a metric that refused one author's patch is a
validator rule nobody wrote an ADR for (docs/plan.md, M3 trap 18).

Three definitions are worth stating once, because every axis below rests on them.

  * **An onset is a note's.** The paper's 𝒪 is a distinct (voice, tick) pair, so two notes one
    voice strikes at one tick are two notes and one onset. Audio has no notes and so no onsets:
    the single place an audio clip counts is [`texture`]'s voice count, sounding fraction and
    polyphony, where ADR 0018 §4 calls it "a voice with one opaque event". That is what makes
    the fixture's bar 1 eight notes over eight onsets while three voices sound.
  * **Everything is measured over the unrolled document** — what `compile` would play, loops
    and all (`view.strikes`) — with a note's sounding span clipped to the bar being measured.
  * **Per bar, not only per song.** Every axis in the paper's Appendix B is a per-song value.
    "Bar 3 has no onset on any voice" is something a model can act on and a song-level mean is
    not, so the per-bar reading is the one here and the song total sits beside it.

Where a definition divides by nothing — a bar with no onset, a line with no interval, a maximum
over no chord — the value is `none`, except where the paper sets one: an ascending ratio with
no interval is 1/2.
"""

from __future__ import annotations

from fractions import Fraction

from escribass_schema.escribass.song.v1 import Section, Song

from .view import PPQ, Bar, Strike, audio_events, bars, document_end, strikes, voices

__all__ = ["form", "harmony", "melody", "rhythm", "texture", "variation"]

#: A pitch class is `pitch mod 12`, and 0 is C in every octave convention MIDI has — which is
#: why a class may be named where a *pitch* may not (ADR 0018 §1: "C4" is 60 in one convention
#: and 72 in another, and that is a wrong-but-valid call waiting to happen).
_CLASSES = ("C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B")

#: The major scale's degrees, in semitones. Chromaticism maximises over the twelve roots, so
#: this is the only scale knowledge here and it names no key (ADR 0018 §4).
_MAJOR = (0, 2, 4, 5, 7, 9, 11)

#: A 16th is a quarter over four; a triplet eighth a quarter over three.
_SIXTEENTH = PPQ // 4
_TRIPLET = PPQ // 3


def _ratio(value: Fraction | None) -> str:
    """An exact rational, or `none` where the definition divides by nothing."""
    return "none" if value is None else str(value)


def _over(part: int, whole: int) -> Fraction | None:
    return Fraction(part, whole) if whole else None


# ponytail: a voice is labelled by its track name, which two tracks may share — the view's
# voice table is where the id is, and these rows sit under it. If that ever confuses a reader
# of a real document, append the id here; nothing else has to move.
def _named(song: Song) -> list[tuple[str, str]]:
    """Every voice as `(track id, name)`, in the model's order."""
    return [(track.id, track.name) for track in voices(song)]


def _samplers(song: Song) -> set[str]:
    """The tracks whose instrument is an SFZ. Their pitches may not be pitches, and the
    harmony axis says so beside the numbers rather than dropping the voice (ADR 0018 §1)."""
    return {
        track.id
        for track in song.tracks.values()
        if track.instrument is not None
        and track.instrument.ref is not None
        and track.instrument.ref.sampler is not None
    }


def _grid(song: Song) -> list[Bar]:
    return bars(song, document_end(song))


def _in(bar: Bar, played: list[Strike]) -> list[Strike]:
    """The notes whose onset falls in this bar."""
    return [note for note in played if bar.start_tick <= note.tick < bar.end_tick]


def _spans(bar: Bar, played: list[Strike]) -> list[tuple[Strike, int, int]]:
    """Every note sounding anywhere in this bar, clipped to it: `(note, from, to)`."""
    clipped = []
    for note in played:
        first = max(note.tick, bar.start_tick)
        last = min(note.tick + note.sounding, bar.end_tick)
        if last > first:
            clipped.append((note, first, last))
    return clipped


def _ticks(spans: list[tuple[int, int]]) -> int:
    """The measure of a union of half-open spans — overlapping notes are sounding once."""
    total = 0
    reach = None
    for first, last in sorted(spans):
        if reach is None or first > reach:
            total += last - first
            reach = last
        elif last > reach:
            total += last - reach
            reach = last
    return total


def _polyphony(spans: list[tuple[int, int]]) -> int:
    """The most notes sounding at once — a sweep over the ends, so no tick is visited twice."""
    edges = sorted([(first, 1) for first, _ in spans] + [(last, -1) for _, last in spans])
    most = at = 0
    for _, step in edges:
        at += step
        most = max(most, at)
    return most


# --- rhythm --------------------------------------------------------------------------------


def _rhythm_row(counted: list[tuple[Bar, list[Strike]]]) -> str:
    """Counts over one row's worth of notes: a bar, a bar and a voice, the song, or a voice.

    One function for all four, because "on a beat" is a *bar's* question — the beat is the
    signature's — and a song row that averaged the per-bar answers would be a different number.
    So every row is a list of bars with the notes that fall in them, and a bar row is a list of
    one.

    An onset is a (voice, tick) pair, so Keys striking where Lead already strikes is a second
    onset and not the same one. The smallest gap is the exception and is taken over the
    *distinct ticks* of the row: between two voices sounding together the interval is zero, and
    "the fastest this row moves" is the question worth answering.
    """
    onsets = sorted(
        {(bar.start_tick, bar.beat_ticks, note.track_id, note.tick) for bar, here in counted for note in here}
    )
    notes = [note for _, here in counted for note in here]
    ticks = sorted({note.tick for note in notes})
    gaps = [b - a for a, b in zip(ticks, ticks[1:])]
    on_beat = sum(1 for start, beat, _, tick in onsets if beat and (tick - start) % beat == 0)
    sixteenth = sum(1 for start, _, _, tick in onsets if (tick - start) % _SIXTEENTH == 0)
    triplet = sum(1 for start, _, _, tick in onsets if (tick - start) % _TRIPLET == 0)
    neither = sum(
        1
        for start, _, _, tick in onsets
        if (tick - start) % _SIXTEENTH and (tick - start) % _TRIPLET
    )
    return (
        f"{len(onsets)} onset{'s' if len(onsets) != 1 else ''} · {on_beat} on a beat"
        f" · syncopation {_ratio(_over(len(onsets) - on_beat, len(onsets)))}"
        f" · on the 16th grid {sixteenth}, the triplet-8th grid {triplet}, neither {neither}"
        f" · smallest gap {min(gaps) if gaps else 'none'}"
        f" · mean written length {_ratio(_over(sum(note.written for note in notes), len(notes)))}"
    )


def rhythm(song: Song) -> str:
    """Onsets, where they fall, and how fast they come — per bar, per voice, and per song."""
    grid = _grid(song)
    played = strikes(song)
    counted = [(bar, _in(bar, played)) for bar in grid]

    def mine(rows: list[tuple[Bar, list[Strike]]], track_id: str) -> list[tuple[Bar, list[Strike]]]:
        return [(bar, [note for note in here if note.track_id == track_id]) for bar, here in rows]

    lines = ["rhythm · an onset is a (voice, tick) pair over notes; a beat is the signature's"]
    for bar, here in counted:
        lines.append(f"@{bar.number} " + _rhythm_row([(bar, here)]))
        for track_id, name in _named(song):
            if any(note.track_id == track_id for note in here):
                lines.append(f"  {name} " + _rhythm_row(mine([(bar, here)], track_id)))
    lines.append("song " + _rhythm_row(counted))
    for track_id, name in _named(song):
        if any(note.track_id == track_id for note in played):
            lines.append(f"  {name} " + _rhythm_row(mine(counted, track_id)))
    return "\n".join(lines) + "\n"


# --- harmony -------------------------------------------------------------------------------


def _mass(spans: list[tuple[Strike, int, int]]) -> list[int]:
    """The duration-weighted pitch-class mass: twelve integers, ticks sounding per class."""
    weights = [0] * 12
    for note, first, last in spans:
        weights[note.pitch % 12] += last - first
    return weights


def _chromaticism(weights: list[int]) -> Fraction | None:
    """`1 − max over the twelve roots of (mass on the major scale at that root / total mass)`.

    It needs no key precisely because it maximises over the roots — which is how the paper
    defines it, and why it survives U1 having left this repository without a key detector.
    """
    total = sum(weights)
    if not total:
        return None
    best = max(sum(weights[(root + degree) % 12] for degree in _MAJOR) for root in range(12))
    return 1 - Fraction(best, total)


def _harmony_row(weights: list[int]) -> str:
    sounding = [(index, weight) for index, weight in enumerate(weights) if weight]
    heaviest = max((weight for _, weight in sounding), default=0)
    # "At or above 30% of the heaviest", in integers: `10·w ≥ 3·max` rather than `w / max ≥ 0.3`,
    # which is the same question asked without a float (ADR 0018 §4).
    prominent = [_CLASSES[index] for index, weight in sounding if 10 * weight >= 3 * heaviest]
    written = ", ".join(f"{_CLASSES[index]} {weight}" for index, weight in sounding)
    return (
        f"{written or 'no mass'} · {len(sounding)} class{'es' if len(sounding) != 1 else ''}"
        f" · prominent {', '.join(prominent) or 'none'}"
        f" · chromaticism {_ratio(_chromaticism(weights))}"
    )


def harmony(song: Song) -> str:
    """What sounds, weighted by how long it sounds — per bar over every voice and per voice."""
    grid = _grid(song)
    played = strikes(song)
    samplers = _samplers(song)
    lines = [
        "harmony · pitch-class mass is ticks sounding, class = pitch mod 12 (0 = C);"
        " a class not named has none; no key is claimed"
    ]
    for bar in grid:
        spans = _spans(bar, played)
        lines.append(f"@{bar.number} " + _harmony_row(_mass(spans)))
        for track_id, name in _named(song):
            mine = [span for span in spans if span[0].track_id == track_id]
            if mine:
                mark = " · sampler: its keys are whatever the SFZ maps" if track_id in samplers else ""
                lines.append(f"  {name} " + _harmony_row(_mass(mine)) + mark)
    return "\n".join(lines) + "\n"


# --- melody --------------------------------------------------------------------------------


def _line(notes: list[Strike]) -> list[int]:
    """The voice's line: the highest note at each onset.

    A stated rule and not a detector — ADR 0018 §4 refuses to identify a melody voice at all,
    so every voice gets a line and the reader chooses which one is the tune.
    """
    highest: dict[int, int] = {}
    for note in notes:
        highest[note.tick] = max(highest.get(note.tick, note.pitch), note.pitch)
    return [highest[tick] for tick in sorted(highest)]


def _melody_row(notes: list[Strike], *, intervals: bool) -> str:
    pitches = [note.pitch for note in notes]
    line = _line(notes)
    moves = [b - a for a, b in zip(line, line[1:])]
    # The paper's set M drops a repeated pitch, so both ratios are over the non-zero intervals.
    moved = [move for move in moves if move]
    stepwise = sum(1 for move in moved if abs(move) <= 2)
    ascending = sum(1 for move in moved if move > 0)
    written = " ".join(f"{move:+d}" for move in moves) if intervals else ""
    return (
        f"range {min(pitches)}–{max(pitches)} ({max(pitches) - min(pitches)})"
        f" · {len(set(pitches))} distinct"
        + (f" · intervals {written or 'none'}" if intervals else "")
        + f" · steps {stepwise} of {len(moved)} = {_ratio(_over(stepwise, len(moved)))}"
        # The one value the paper sets where its own definition divides by nothing.
        f" · ascending {ascending} of {len(moved)} = "
        + (_ratio(_over(ascending, len(moved))) if moved else "1/2")
    )


def melody(song: Song) -> str:
    """Each voice's line, bar by bar and over the song. No voice is identified as the melody."""
    grid = _grid(song)
    played = strikes(song)
    lines = [
        "melody · a voice's line takes the highest note at a simultaneous onset;"
        " range and distinct count every note, intervals only the line;"
        " a repeated pitch is dropped before the ratios"
    ]
    for track_id, name in _named(song):
        mine = [note for note in played if note.track_id == track_id]
        if not mine:
            continue
        lines.append(name)
        for bar in grid:
            here = _in(bar, mine)
            lines.append(f"  @{bar.number} " + (_melody_row(here, intervals=True) if here else "silent"))
        lines.append("  song " + _melody_row(mine, intervals=False))
    return "\n".join(lines) + "\n"


# --- texture -------------------------------------------------------------------------------


def texture(song: Song) -> str:
    """How many voices, how thick, and how much of the bar each of them fills."""
    grid = _grid(song)
    played = strikes(song)
    heard = audio_events(song)
    lines = [
        "texture · a voice sounds when a note or an audio iteration does,"
        " and an audio iteration is one opaque event"
    ]
    for bar in grid:
        spans = _spans(bar, played)
        audio = [
            (event.track_id, max(event.tick, bar.start_tick), min(event.tick + event.length, bar.end_tick))
            for event in heard
            if event.tick < bar.end_tick and event.tick + event.length > bar.start_tick
        ]
        here = _in(bar, played)
        onsets = {(note.track_id, note.tick) for note in here}
        struck: dict[tuple[str, int], list[int]] = {}
        for note in here:
            struck.setdefault((note.track_id, note.tick), []).append(note.pitch)
        widths = [max(pitches) - min(pitches) for pitches in struck.values() if len(pitches) > 1]
        sounding = {span[0].track_id for span in spans} | {event[0] for event in audio}
        lines.append(
            f"@{bar.number} {len(sounding)} voice{'s' if len(sounding) != 1 else ''}"
            f" · {len(here)} note{'s' if len(here) != 1 else ''} over {len(onsets)} onset{'s' if len(onsets) != 1 else ''}"
            f" · mean simultaneity {_ratio(_over(len(here), len(onsets)))}"
            f" · widest chord {max(widths) if widths else 'none'}"
        )
        for track_id, name in _named(song):
            mine = [(first, last) for note, first, last in spans if note.track_id == track_id]
            mine += [(first, last) for voice, first, last in audio if voice == track_id]
            if not mine:
                continue
            sounding = _ticks(mine)
            lines.append(
                f"  {name} polyphony {_polyphony(mine)}"
                f" · sounding {sounding} of {bar.ticks} = {_ratio(_over(sounding, bar.ticks))}"
            )
    return "\n".join(lines) + "\n"


# --- form ----------------------------------------------------------------------------------


def _letter(index: int) -> str:
    """`A`…`Z`, then `AA`. Twenty-six distinct bars is already a song with no repetition."""
    name = ""
    while True:
        name = chr(ord("A") + index % 26) + name
        index = index // 26 - 1
        if index < 0:
            return name


def _jaccard(one: frozenset, other: frozenset) -> Fraction:
    """The paper's overlap. Two empty bars overlap fully — they are the same bar."""
    union = len(one | other)
    return Fraction(len(one & other), union) if union else Fraction(1)


def form(song: Song) -> str:
    """Which bars are the same bar, spelled as letters, and how much they overlap."""
    grid = _grid(song)
    played = strikes(song)
    order = {track_id: position for position, (track_id, _) in enumerate(_named(song))}
    sets = [
        frozenset(
            (order.get(note.track_id, 0), note.tick - bar.start_tick, note.pitch)
            for note in _in(bar, played)
        )
        for bar in grid
    ]

    lettering: dict[frozenset, str] = {}
    letters = []
    for held in sets:
        letters.append(lettering.setdefault(held, _letter(len(lettering))))

    # ponytail: every pair of bars, which is the paper's own definition — self-similarity is
    # the mean overlap over all of them — and quadratic in the bar count. At `view.MAX_BARS` it
    # is eight million set intersections, which is a slow answer rather than a wrong one, and
    # nothing in a prompt is that long. Sample the pairs if that ever stops being true.
    #
    # The sums below are of `Fraction`s rather than of integers, so `_over` does not serve:
    # the mean of exact rationals is itself exact, which is the whole of why this axis can be
    # goldened byte for byte where an entropy could not.
    pairs = [(one, other) for one in range(len(sets)) for other in range(one + 1, len(sets))]
    similarity = (
        sum((_jaccard(sets[one], sets[other]) for one, other in pairs), Fraction(0)) / len(pairs)
        if pairs
        else None
    )
    consecutive = [_jaccard(sets[at], sets[at + 1]) for at in range(len(sets) - 1)]
    novelty = (
        sum((1 - overlap for overlap in consecutive), Fraction(0)) / len(consecutive)
        if consecutive
        else None
    )

    lines = [
        "form · a bar is its set of (voice, onset in bar, pitch);"
        " length and velocity are not compared"
    ]
    lines.append(
        f"bars {' '.join(letters) or 'none'}"
        f" · {len(lettering)} distinct of {len(sets)} = {_ratio(_over(len(lettering), len(sets)))}"
    )
    lines.append(f"self-similarity {_ratio(similarity)} · novelty {_ratio(novelty)}")
    lines.append(
        "consecutive overlap "
        + (
            " · ".join(
                f"@{at + 1}~@{at + 2} {overlap}" for at, overlap in enumerate(consecutive)
            )
            or "none"
        )
    )
    sections = sorted(song.sections.values(), key=lambda s: (s.start_tick, s.id))
    lines.append(
        "sections "
        + (
            " · ".join(
                f"{section.name} @{_first(grid, section)}–@{_last(grid, section)}"
                f" over {' '.join(_over_bars(grid, letters, section)) or 'no bar'}"
                for section in sections
            )
            or "none"
        )
    )
    return "\n".join(lines) + "\n"


def _covered(grid: list[Bar], section: Section) -> list[Bar]:
    return [
        bar for bar in grid if section.start_tick < bar.end_tick and section.end_tick > bar.start_tick
    ]


def _first(grid: list[Bar], section: Section) -> str:
    covered = _covered(grid, section)
    return str(covered[0].number) if covered else "?"


def _last(grid: list[Bar], section: Section) -> str:
    covered = _covered(grid, section)
    return str(covered[-1].number) if covered else "?"


def _over_bars(grid: list[Bar], letters: list[str], section: Section) -> list[str]:
    return [letters[bar.number - 1] for bar in _covered(grid, section)]


# --- within-song variation -------------------------------------------------------------------


def _pattern(bar: Bar, notes: list[Strike]) -> tuple[tuple[int, int, int], ...]:
    """A voice's bar, as what a reader would compare: onset in bar, sounding length, pitch."""
    return tuple(
        sorted(
            (
                note.tick - bar.start_tick,
                min(note.sounding, bar.end_tick - note.tick),
                note.pitch,
            )
            for note in notes
        )
    )


def _class(before: tuple, after: tuple) -> str:
    if not before and not after:
        return "silent"
    if not before:
        return "enters"
    if not after:
        return "leaves"
    if before == after:
        return "identical"
    if [(onset, length) for onset, length, _ in before] == [(onset, length) for onset, length, _ in after]:
        return "same rhythm"
    if sorted(pitch for *_, pitch in before) == sorted(pitch for *_, pitch in after):
        return "same pitches"
    return "different"


def variation(song: Song) -> str:
    """Each bar against the one before it, per voice.

    The paper's own definition is each windowed standard deviation over "the corpus standard
    deviation of axis a" — a corpus measure twice over, deferred in ADR 0018 §4. What is left
    is the comparison a reader can make by eye, which is also the one a model can act on.
    """
    grid = _grid(song)
    played = strikes(song)
    lines = [
        "within-song variation · each bar against the one before it;"
        " `silent` is the absence of the six classes, not a seventh"
    ]
    for track_id, name in _named(song):
        mine = [note for note in played if note.track_id == track_id]
        if not mine:
            continue
        patterns = [_pattern(bar, _in(bar, mine)) for bar in grid]
        classes = [
            f"@{bar.number} {_class(patterns[at - 1] if at else (), patterns[at])}"
            for at, bar in enumerate(grid)
        ]
        distinct = len({held for held in patterns if held})
        lines.append(
            f"{name} "
            + " · ".join(classes)
            + f" · {distinct} distinct bar pattern{'s' if distinct != 1 else ''}"
        )
    return "\n".join(lines) + "\n"
