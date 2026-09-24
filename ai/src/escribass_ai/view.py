"""The bar view the model reads, and the reading of the document every axis shares (ADR 0018 §1).

What the model is given instead of the song. `get_song` is not among the tools it is offered
(ADR 0018 §2), so this text has to carry every id and every field a tool it *can* call will
take — which is why ids are written out in full and velocities are written at all: `set_notes`
"replaces the clip's whole note set", so a model keeping one note and moving another re-sends
both, and an id it does not have is an id `core` mints again with a new provenance.

Three rules earn their own paragraph, because each is a thing a plausible implementation gets
wrong and the golden would not notice on a smaller document.

  * **A bar is not a constant number of ticks.** [`bars`] walks the time-signature map event by
    event, exactly as `app/src/time.ts` does one language over and for the reason its header
    gives: `ticks_per_bar` computed once from the first event is right until a second one says
    otherwise and silently wrong after it. A signature event landing mid-bar truncates the bar
    in progress, because the event carries a tick and not a bar.
  * **A note's tick is clip-relative and the tool takes it that way.** The view lists a note
    under the bar its *absolute* onset falls in, with its clip-relative tick verbatim, and the
    clip's line above it says where that clip's tick 0 sits in the bar. Both operands of the
    one subtraction a model may need are then on the page — the alternative, bar-relative
    onsets converted back by the model, is the arithmetic the spike watched a model get wrong
    (a quarter note written as 480 ticks).
  * **A loop is shown as what sounds.** [`iterations`] is `core::render::compile`'s unrolling,
    not a second rule beside it: a looping clip repeats its first `loop_length_ticks` to fill
    `length_ticks`, the last iteration cut where the clip ends. The notes are listed once,
    under the first iteration, with their *written* values — what `set_notes` re-sends — and a
    note the loop cuts or never reaches is marked rather than dropped, because it is in the
    document and a model asked to fix a clip should see it.

This module is pure in ADR 0018 §5's sense: no randomness, no clock, no I/O, no environment,
no network. `ai/tests/test_view.py` holds that claim as a golden, as
`app/tests/projection.test.ts` holds the frontend's.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import NamedTuple

from escribass_schema.escribass.song.v1 import (
    Automation,
    Clip,
    DeviceRef,
    Effect,
    Instrument,
    Note,
    Song,
    TempoEvent,
    TimeSignatureEvent,
    Track,
    TrackKind,
)

__all__ = [
    "PPQ",
    "Bar",
    "Strike",
    "audio_events",
    "bars",
    "document_end",
    "iterations",
    "number",
    "strikes",
    "view",
    "voices",
]

#: 960 ticks to the quarter note, fixed by `schema_version` 1 (docs/specs.md §4.2).
PPQ = 960

# ponytail: a ceiling on the view, not on the song — the same one `app/src/time.ts` takes for
# the same reason. Ticks are int32, so a clip at tick 2×10⁹ is a legal document asking for
# 559,000 bar blocks, which is a hung sidecar rather than a slow one. The view stops here and
# says so on the header line. The upgrade path is a window: the bars around what the prompt is
# about, not all of them — which is what a prompt-sized view will need anyway long before 4096
# bars of text would fit in one.
MAX_BARS = 4096

# What a view reads when the map says nothing. Every project `core` creates has an event at
# tick 0 for both maps, so these are reached only by a document that lost one — and a view
# that divided by an absent signature would have no grid at all, which reads as a broken tool
# rather than as a broken document. `app/src/time.ts` opens the same two doors.
_OPENING_TEMPO = TempoEvent(id="", tick=0, bpm=120.0)
_OPENING_SIGNATURE = TimeSignatureEvent(id="", tick=0, numerator=4, denominator=4)

_TRACK_KIND = {
    TrackKind.UNSPECIFIED: "unspecified",
    TrackKind.INSTRUMENT: "instrument",
    TrackKind.AUDIO: "audio",
    TrackKind.BUS: "bus",
    TrackKind.MASTER: "master",
}


class Bar(NamedTuple):
    """A bar of the grid: the number a block is headed with, where it starts, how long it is.

    `beat_ticks` travels with it because a beat is the denominator's and not the song's, so
    "on a beat" (ADR 0018 §4, syncopation) is a question only a bar can answer.
    """

    number: int
    start_tick: int
    ticks: int
    beat_ticks: int

    @property
    def end_tick(self) -> int:
        return self.start_tick + self.ticks


@dataclass(frozen=True)
class Strike:
    """One note as it sounds, once per loop iteration that reaches it.

    `sounding` is what `compile` would play — the written length cut at the iteration's end —
    and `written` is what the document holds and what `set_notes` re-sends. The axes measure
    the first; the view writes the second and marks the difference (ADR 0018 §1, §4).
    """

    track_id: str
    clip_id: str
    note_id: str
    tick: int
    pitch: int
    sounding: int
    written: int
    velocity: int


class Audio(NamedTuple):
    """One iteration of an audio clip: a voice with one opaque event (ADR 0018 §4, texture)."""

    track_id: str
    clip_id: str
    tick: int
    length: int


def number(value: float) -> str:
    """A double as the view writes it: `120`, `-4.5`, `0.25`.

    Trailing `.0` is stripped because a gain of `0` and a gain of `0.0` are the same number and
    printing both spellings in one document invites a model to think they are not. `repr` is
    CPython's shortest round-tripping form, so nothing here rounds and nothing is locale's.
    """
    if value.is_integer():  # False for nan and inf, which then fall through to repr
        return str(int(value))
    return repr(value)


def _ordered(events, *, by) -> list:
    """Ordered by the model, never by the map (ADR 0001 §3).

    Ties break on the id, so two events at one tick are ordered by the document rather than by
    whichever key the map happened to yield first — which is what the key-reversed golden of
    ADR 0018 §5 exists to catch.
    """
    return sorted(events, key=by)


def tempo_marks(song: Song) -> list[TempoEvent]:
    """The tempo map in tick order, with an opening event if the document has none."""
    marks = _ordered((song.tempo_map.events if song.tempo_map else {}).values(), by=lambda e: (e.tick, e.id))
    return marks if marks and marks[0].tick <= 0 else [_OPENING_TEMPO, *marks]


def signature_marks(song: Song) -> list[TimeSignatureEvent]:
    """The time-signature map in tick order, with an opening event if the document has none."""
    marks = _ordered(
        (song.time_signature_map.events if song.time_signature_map else {}).values(),
        by=lambda e: (e.tick, e.id),
    )
    return marks if marks and marks[0].tick <= 0 else [_OPENING_SIGNATURE, *marks]


def _bar_ticks(event: TimeSignatureEvent) -> tuple[int, int]:
    """A bar and a beat, in ticks, for one signature. `960 × 4 × n / d` and `960 × 4 / d`.

    §4.4 has a rule for tempo and none for the signature, so `0/0` is a document the validator
    accepts and a bar of zero ticks is a loop that never ends. Fall back to 4/4 rather than to
    nothing, which is the door `app/src/time.ts` opens for the same document.
    """
    if event.denominator <= 0 or event.numerator <= 0:
        return PPQ * 4, PPQ
    beat = PPQ * 4 // event.denominator
    ticks = beat * event.numerator
    return (ticks, beat) if ticks >= 1 and beat >= 1 else (PPQ * 4, PPQ)


def document_end(song: Song) -> int:
    """The furthest tick the document reaches.

    The arrangement view's rule (`app/src/arrangement.ts`) plus the two kinds of thing that
    view does not draw: a marker, and an automation point. A lane that rides to tick 9600 is
    part of the song even where nothing sounds, and the render fixture's master fade is exactly
    that.
    """
    reaches = [0]
    reaches += [clip.start_tick + clip.length_ticks for clip in song.clips.values()]
    reaches += [section.end_tick for section in song.sections.values()]
    reaches += [marker.tick for marker in song.markers.values()]
    reaches += [event.tick for event in tempo_marks(song)]
    reaches += [event.tick for event in signature_marks(song)]
    reaches += [point.tick for lane in song.automation.values() for point in lane.points.values()]
    return max(reaches)


def bars(song: Song, through_tick: int) -> list[Bar]:
    """The bar grid from tick 0 through `through_tick`, following the time-signature map.

    An empty document is zero bars and nothing is padded — unlike the frontend's grid, which
    draws eight so a new project has a ruler. A view has no ruler to draw and a bar block for a
    bar with nothing in it is tokens spent saying nothing.
    """
    marks = signature_marks(song)
    grid: list[Bar] = []
    for index, mark in enumerate(marks):
        per_bar, beat = _bar_ticks(mark)
        until = marks[index + 1].tick if index + 1 < len(marks) else None
        at = max(0, mark.tick)
        while at < through_tick and len(grid) < MAX_BARS:
            if until is not None and at >= until:
                break
            # A signature event that lands mid-bar ends the bar in progress where it lands: the
            # event carries a tick and not a bar, and moving it to the next bar line would put
            # the grid somewhere the document does not.
            ticks = per_bar if until is None else min(per_bar, until - at)
            grid.append(Bar(len(grid) + 1, at, ticks, beat))
            at += ticks
    return grid


def bar_of(grid: list[Bar], tick: int) -> Bar | None:
    """The bar containing `tick`, or `None` past the end of the grid."""
    for bar in grid:
        if bar.start_tick <= tick < bar.end_tick:
            return bar
    return None


def _at(grid: list[Bar], tick: int) -> str:
    """A tick as `@bar tick n`, or as a bare tick where the grid does not reach."""
    bar = bar_of(grid, tick)
    return f"@{bar.number} tick {tick - bar.start_tick}" if bar else f"tick {tick}"


def voices(song: Song) -> list[Track]:
    """Every track in the model's order: `index`, then id for a document that repeats one."""
    return _ordered(song.tracks.values(), by=lambda t: (t.index, t.id))


def clips_of(song: Song, track_id: str) -> list[Clip]:
    """One voice's clips, by `start_tick` then id."""
    return _ordered(
        (clip for clip in song.clips.values() if clip.track_id == track_id),
        by=lambda c: (c.start_tick, c.id),
    )


def iterations(clip: Clip) -> list[tuple[int, int]]:
    """`(offset, length)` per loop iteration — `core::render::compile`'s unrolling, in Python.

    A looping clip repeats its first `loop_length_ticks` of content to fill `length_ticks`, so
    it sounds as that many iterations with the last cut where the clip ends; a clip that does
    not loop is one iteration of its own length. Not a second rule beside `compile`'s: the
    axes measure what would be rendered, and a view that unrolled a loop differently from the
    renderer would describe a song nobody can hear (ADR 0018 §4, common rules).
    """
    unit = clip.loop_length_ticks if clip.loop_length_ticks is not None else clip.length_ticks
    # The validator refuses both; guarded because the loop below would otherwise not end.
    if unit <= 0 or clip.length_ticks <= 0:
        return []
    # ponytail: no cap on the count, deliberately. A clip of two billion ticks looping every
    # one is a legal document and two billion iterations here, but `core::render::compile` has
    # exactly that loop and no cap either — and an axis that stopped counting where `compile`
    # kept playing would describe a song nobody hears, which is the one thing this may not do.
    # The ceiling is the model's, and it moves in `core` or not at all.
    spans = []
    offset = 0
    while offset < clip.length_ticks:
        spans.append((offset, min(unit, clip.length_ticks - offset)))
        offset += unit
    return spans


def strikes(song: Song) -> list[Strike]:
    """Every note of the document as it sounds, unrolled over the loops.

    A note at or past the loop length is not loop content and never sounds, so it is absent
    here and present in the view — the two readings the document supports, each where it
    belongs.
    """
    order = {track.id: position for position, track in enumerate(voices(song))}
    played: list[Strike] = []
    # The walk is unordered on purpose: the key below is a total order over what it produces —
    # a note id is unique across the document and a looping clip's repeats differ by tick — so
    # one sort at the end is the whole of the guarantee, and two more above it would be two
    # more places for it to be deleted without a test noticing (measured, M3 PR 6).
    for clip in song.clips.values():
        if clip.note_clip is None:
            continue
        for offset, length in iterations(clip):
            for note in clip.note_clip.notes.values():
                if note.start_tick >= length:
                    continue
                played.append(
                    Strike(
                        track_id=clip.track_id,
                        clip_id=clip.id,
                        note_id=note.id,
                        tick=clip.start_tick + offset + note.start_tick,
                        pitch=note.pitch,
                        sounding=min(note.length_ticks, length - note.start_tick),
                        written=note.length_ticks,
                        velocity=note.velocity,
                    )
                )
    # No test here catches this sort's removal, and the honest reason is that every axis
    # re-orders what it reads — by tick, into a set, or through a sweep — so none of them can
    # tell. It stays because the next consumer is PR 8's loop, and a list whose order is the
    # map's is the defect ADR 0001 §3 names, one that a golden finds only once something
    # iterates it.
    return _ordered(played, by=lambda s: (order.get(s.track_id, 0), s.tick, s.pitch, s.note_id))


def audio_events(song: Song) -> list[Audio]:
    """Every audio iteration of the document — one opaque event each (ADR 0018 §4, texture)."""
    order = {track.id: position for position, track in enumerate(voices(song))}
    heard = [
        Audio(clip.track_id, clip.id, clip.start_tick + offset, length)
        for clip in song.clips.values()
        if clip.audio_clip is not None
        for offset, length in iterations(clip)
    ]
    return _ordered(heard, by=lambda a: (order.get(a.track_id, 0), a.tick, a.clip_id))


# --- what a device, a clip and a note are written as -------------------------------------


def _device(ref: DeviceRef | None) -> str:
    """A `DeviceRef` in full. The hash is never abbreviated: an abbreviation is a thing a model
    expands, and Gemini dropped a character from a 26-character id in the spike."""
    if ref is None:
        return "no device"
    if ref.plugin is not None:
        return f"plugin {ref.plugin.plugin_id} {ref.plugin.version}"
    if ref.sampler is not None:
        # Said here, once, rather than by a rule on the track's name: the document holds a hash
        # and nothing else about what those keys play, and a General MIDI drum map would have
        # the view assert a fact the song does not carry (ADR 0018 §1).
        return (
            f"sampler sfz {ref.sampler.sfz_hash}"
            " (keys are whatever the SFZ maps; may be unpitched)"
        )
    if ref.cmajor is not None:
        return f"cmajor source {ref.cmajor.source_hash}"
    if ref.faust is not None:
        return f"faust source {ref.faust.source_hash}"
    if ref.neural is not None:
        return f"neural model {ref.neural.model_hash}"
    return "no device"


def _held(device: Instrument | Effect) -> str:
    """A device's id and ref, with how many parameters are set and never which.

    `set_param` is withheld from the model (ADR 0022 §1), so a value it cannot act on is
    tokens; and Surge XT's 2,855 `ParamID`s in a prompt are 2,855 invitations to the call the
    spike watched set the wrong parameter and report success.
    """
    count = len(device.params)
    set_ = f" ({count} param{'s' if count != 1 else ''} set)" if count else ""
    return f"{device.id} {_device(device.ref)}{set_}"


def _routing(track: Track) -> list[str]:
    """Routing, only where it is not the default. The default is most tracks."""
    routing = track.routing
    if routing is None:
        return []
    parts = []
    if routing.output_track_id:
        parts.append(f"out {routing.output_track_id}")
    for target, amount in _ordered(routing.sends.items(), by=lambda pair: pair[0]):
        parts.append(f"send {target} {number(amount)}")
    for target, source in _ordered(routing.sidechains.items(), by=lambda pair: pair[0]):
        parts.append(f"sidechain {target} from {source}")
    return parts


def _voice_line(track: Track) -> str:
    parts = [f"{track.index} {track.name} {track.id} {_TRACK_KIND.get(track.kind, 'unspecified')}"]
    if track.instrument is not None:
        parts.append(_held(track.instrument))
    chain = _ordered(track.fx_chain.values(), by=lambda e: (e.index, e.id))
    if chain:
        parts.append("fx " + ", ".join(_held(effect) for effect in chain))
    mix = track.mix
    if mix is not None:
        parts.append(f"gain {number(mix.gain_db)} pan {number(mix.pan)}")
        if mix.mute:
            parts.append("MUTED")
        if mix.solo:
            parts.append("SOLO")
    parts += _routing(track)
    if track.allow_overlap:
        parts.append("overlap allowed")
    return "  " + " · ".join(parts)


def _content(clip: Clip) -> str:
    if clip.note_clip is not None:
        count = len(clip.note_clip.notes)
        return f"{count} note{'s' if count != 1 else ''}"
    audio = clip.audio_clip
    if audio is None:
        return "no content"
    parts = [f"audio {audio.asset_hash}"]
    if audio.gain_db:
        parts.append(f"gain {number(audio.gain_db)}")
    if audio.fade_in_ticks or audio.fade_out_ticks:
        parts.append(f"fade {audio.fade_in_ticks}/{audio.fade_out_ticks}")
    if audio.time_stretch:
        parts.append("stretch")
    return " ".join(parts)


def _note_line(clip: Clip, note: Note, unit: int) -> str:
    """One note, at its clip tick, with its written length — and what the loop does to it.

    The cut is measured against the loop unit and not against the iteration, because every
    iteration but a short last one cuts the same way and the iteration list above already says
    where the short one is.
    """
    parts = [f"{note.pitch}@{note.start_tick}>{note.length_ticks} v{note.velocity} #{note.id}"]
    if note.microtonal_cents:
        parts.append(f"{number(note.microtonal_cents)} cents")
    # §4.4 keeps a note inside its clip, so for a clip that does not loop neither branch below
    # can fire in a valid document. They are written for the one that does, and worded for both
    # rather than asserting "the loop" over a document that has none.
    named = "loop length" if clip.loop_length_ticks is not None else "clip length"
    if note.start_tick >= unit:
        parts.append(f"(at or past {named} {unit}: never sounds)")
    elif note.start_tick + note.length_ticks > unit:
        parts.append(f"(cut to {unit - note.start_tick} by the {named.split()[0]})")
    return "    " + " ".join(parts)


def view(song: Song) -> str:
    """The whole view: a header, the voices, the clips, one block per bar, then the lanes.

    Ordering is the model's everywhere — voices by `index`, clips by voice then `start_tick`
    then id, notes by `start_tick` then id, lanes by id, points by tick — which is what the
    key-reversed golden exists to catch (ADR 0012 §5, amended; ADR 0018 §1).
    """
    end = document_end(song)
    grid = bars(song, end)
    lines: list[str] = []

    truncated = " (the view stops here)" if len(grid) == MAX_BARS else ""
    where = f" ({_at(grid, end)})" if grid else ""
    lines.append(
        f"song {song.id} · {PPQ} ticks per quarter · {len(grid)} bar{'s' if len(grid) != 1 else ''}"
        f"{truncated} · ends at tick {end}{where}"
    )
    for mark in signature_marks(song):
        per_bar, beat = _bar_ticks(mark)
        lines.append(
            f"signature {mark.numerator}/{mark.denominator} from {_at(grid, mark.tick)}"
            f" · a bar is {per_bar} ticks, a beat {beat}"
        )
    lines.append(
        "tempo "
        + " · ".join(f"{number(mark.bpm)} from {_at(grid, mark.tick)}" for mark in tempo_marks(song))
    )

    sections = _ordered(song.sections.values(), by=lambda s: (s.start_tick, s.id))
    lines.append(
        "sections: "
        + (
            " · ".join(
                f"{section.name} {section.id} {_at(grid, section.start_tick)}"
                f" – {_at(grid, section.end_tick)} ({section.start_tick}–{section.end_tick})"
                for section in sections
            )
            or "none"
        )
    )
    markers = _ordered(song.markers.values(), by=lambda m: (m.tick, m.id))
    lines.append(
        "markers: "
        + (
            " · ".join(f"{marker.name} {marker.id} {_at(grid, marker.tick)}" for marker in markers)
            or "none"
        )
    )
    generators = _ordered(song.generators.values(), by=lambda g: g.id)
    # `source` is abstracted: it is code for M4's compiler, and free-form text a model reads
    # back is not this grammar (ADR 0018 §1).
    lines.append(
        "generators: "
        + (
            " · ".join(
                f"{generator.id} {generator.kind.name.lower()}"
                f" → {generator.clip_id or generator.track_id or 'nothing'}"
                for generator in generators
            )
            or "none"
        )
    )

    lines.append("voices (order · name · id · kind · device · fx · mix)")
    lines += [_voice_line(track) for track in voices(song)]

    ordered_clips = [clip for track in voices(song) for clip in clips_of(song, track.id)]
    names = {track.id: track.name for track in voices(song)}
    lines.append("clips (id · voice · start tick · length · loop · content)")
    for clip in ordered_clips:
        loop = f" loop {clip.loop_length_ticks}" if clip.loop_length_ticks is not None else ""
        lines.append(
            f"  {clip.id} {names.get(clip.track_id, clip.track_id)}"
            f" {clip.start_tick} {clip.length_ticks}{loop} · {_content(clip)}"
        )
    # Orphans last rather than dropped: a clip whose `track_id` names no track is the
    # validator's `track_unknown` and never reaches a valid document, but a view that silently
    # lost one would be a view a model cannot use to fix it.
    orphans = _ordered(
        (clip for clip in song.clips.values() if clip.track_id not in names),
        by=lambda c: (c.start_tick, c.id),
    )
    for clip in orphans:
        lines.append(f"  {clip.id} (no voice {clip.track_id}) {clip.start_tick} {clip.length_ticks}")

    lines.append(
        "notes are pitch@clip-tick>length vVelocity #id;"
        " a note's clip tick = its bar tick − where the clip's tick 0 sits"
    )

    # One blank line before the blocks and one before the lanes; none between bars, because a
    # bar header already begins with `@` and 4,096 blank lines is 4,096 tokens saying nothing.
    lines.append("")
    for bar in grid:
        lines.append(_bar_header(song, bar))
        lines += _bar_block(song, bar, ordered_clips, names)

    lines.append("")
    lines.append("automation (id · target · points as tick:value curve)")
    lanes = _ordered(song.automation.values(), by=lambda a: a.id)
    for lane in lanes:
        points = _ordered(lane.points.values(), by=lambda p: (p.tick, p.id))
        written = ", ".join(
            f"{point.tick}:{number(point.value)} {point.curve.name.lower()}" for point in points
        )
        lines.append(f"  {lane.id} {_target(song, lane)} · {written or 'no point'}")
    if not lanes:
        lines.append("  none")

    return "\n".join(lines) + "\n"


def _target(song: Song, lane: Automation) -> str:
    """A lane's target, resolved. Ids are unique across collections, so one lookup serves a
    track's fader and a device's parameter and the two can never collide (ADR 0015 §1)."""
    target = lane.target
    if target is None:
        return "no target"
    for track in song.tracks.values():
        if track.id == target.device_id:
            return f"track {track.name} {target.param}"
        if track.instrument is not None and track.instrument.id == target.device_id:
            return f"device {target.device_id} ({track.name} instrument) param {target.param}"
        for effect in track.fx_chain.values():
            if effect.id == target.device_id:
                return f"device {target.device_id} ({track.name} fx) param {target.param}"
    return f"device {target.device_id} (no such device) param {target.param}"


def _bar_header(song: Song, bar: Bar) -> str:
    """`@n tick t`, then whatever the document places in this bar."""
    parts = [f"@{bar.number} tick {bar.start_tick}"]
    for mark in tempo_marks(song):
        if bar.start_tick <= mark.tick < bar.end_tick:
            offset = mark.tick - bar.start_tick
            parts.append(f"{number(mark.bpm)} bpm" + (f" at tick {offset}" if offset else ""))
    for mark in signature_marks(song):
        if bar.start_tick <= mark.tick < bar.end_tick:
            offset = mark.tick - bar.start_tick
            parts.append(f"{mark.numerator}/{mark.denominator}" + (f" at tick {offset}" if offset else ""))
    for section in _ordered(song.sections.values(), by=lambda s: (s.start_tick, s.id)):
        if section.start_tick < bar.end_tick and section.end_tick > bar.start_tick:
            first = max(section.start_tick, bar.start_tick) - bar.start_tick
            last = min(section.end_tick, bar.end_tick) - bar.start_tick
            parts.append(f"section {section.name} from tick {first} to {last}")
    for marker in _ordered(song.markers.values(), by=lambda m: (m.tick, m.id)):
        if bar.start_tick <= marker.tick < bar.end_tick:
            parts.append(f"marker {marker.name} at tick {marker.tick - bar.start_tick}")
    return " · ".join(parts)


def _bar_block(song: Song, bar: Bar, ordered_clips: list[Clip], names: dict[str, str]) -> list[str]:
    """One bar's voices: every clip that sounds in it, and the notes whose onset falls in it."""
    # ponytail: every clip is unrolled again for every bar, which is bars × clips × iterations
    # for a view that is bars × clips long. At the size a prompt can hold — the render fixture
    # is 3 bars and 4 clips — it is nothing, and the upgrade path is the same one `MAX_BARS`
    # names: unroll once into a list bucketed by bar, when there is a document that needs it.
    lines: list[str] = []
    for clip in ordered_clips:
        spans = iterations(clip)
        here = [
            (offset, length)
            for offset, length in spans
            if clip.start_tick + offset < bar.end_tick
            and clip.start_tick + offset + length > bar.start_tick
        ]
        notes = (
            _ordered(clip.note_clip.notes.values(), by=lambda n: (n.start_tick, n.id))
            if clip.note_clip is not None
            else []
        )
        # A note is listed under the bar its absolute onset falls in — its onset in the clip's
        # *first* iteration, since a loop's later iterations are named rather than listed.
        mine = [
            note
            for note in notes
            if bar.start_tick <= clip.start_tick + note.start_tick < bar.end_tick
        ]
        if not here and not mine:
            continue
        parts = [
            f"{names.get(clip.track_id, clip.track_id)} clip {clip.id}"
            f" tick 0 at bar tick {clip.start_tick - bar.start_tick}"
        ]
        if clip.loop_length_ticks is not None:
            unit = clip.loop_length_ticks
            starting = [
                (offset, length)
                for offset, length in here
                if bar.start_tick <= clip.start_tick + offset < bar.end_tick
            ]
            played = ", ".join(
                f"{clip.start_tick + offset - bar.start_tick}"
                + (f" (cut to {length})" if length != unit else "")
                for offset, length in starting
            )
            parts.append(f"loops every {unit}: " + (f"plays at {played}" if played else "sounding from an earlier bar"))
        lines.append("  " + " · ".join(parts))
        unit = clip.loop_length_ticks if clip.loop_length_ticks is not None else clip.length_ticks
        lines += [_note_line(clip, note, unit) for note in mine]
    if not lines:
        lines.append("  (no voice plays)")
    return lines
