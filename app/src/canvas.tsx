// The two canvases: the arrangement timeline and the piano roll (§15, Frontend: "React +
// canvas/WebGL timeline").
//
// Canvas rather than a div per clip because §15 decided it and because the reason it gives —
// rendering is custom either way — is already true here: a bar grid, a clip block and a note
// are shapes, not documents. What this file is *not* is a renderer with a scene graph. Each
// component takes the projection its selector produced and paints it, once, whenever it
// changes. Nothing here reads the `Song`, and nothing here decides an order.
//
// Both views are in one file because they are one problem twice: size a canvas in device
// pixels, read the palette off CSS, paint a bar grid, paint blocks on it. [`useCanvas`] is
// that shared boilerplate and is not an abstraction over "a view" — it knows nothing about
// what is drawn on it.
//
// `ponytail:` no zoom, no scroll, no hit testing and no selection by pointer. The whole song
// is scaled to the width it has, and the roll is chosen from a `<select>`. PR 5 makes a clip
// draggable and needs pixel → tick as well as tick → pixel; inventing the inverse here against
// no interaction would be inventing it twice, and `time.ts` is where it will go when it has a
// caller.

import { useEffect, useRef } from "react";
import type { Arrangement } from "./arrangement.js";
import type { PianoRoll } from "./pianoroll.js";
import { isBlackKey, pitchName } from "./pianoroll.js";
import { scale } from "./time.js";

const TEMPO_ROW = 16;
const RULER_ROW = 18;
const SECTION_ROW = 14;
/** The arrangement's header: tempo and signature marks, bar numbers, sections. The gutter
 *  beside it is sized from this constant rather than from a matching number in the stylesheet,
 *  because the two have to agree to the pixel and nothing would catch them drifting: a lane
 *  half a row off its track name is a bug you see and cannot test for. */
export const HEAD = TEMPO_ROW + RULER_ROW + SECTION_ROW;
/** One track lane, and one gutter row. */
export const ROW = 30;
const GAP = 3;

/** The piano roll's own keyboard column and header. */
const KEYS = 38;
const ROLL_HEAD = 18;
const NOTE_ROW = 12;

/** Reads a CSS custom property off the canvas, so the palette lives in one place. */
function ink(element: HTMLElement, name: string): string {
  return getComputedStyle(element).getPropertyValue(name).trim();
}

/** A vertical hairline lands between device pixels unless it is snapped, and a 1px line drawn
 *  at an integer x is drawn 2px wide and grey. The snap is a drawing decision and lives here;
 *  `time.ts` converts ticks to pixels and rounds nothing (see its header). */
const hairline = (x: number) => Math.round(x) + 0.5;

/**
 * Sizes a canvas in device pixels and paints it, on mount, on every change, and on resize.
 *
 * The effect has no dependency array: `paint` closes over the projection and is rebuilt each
 * render, so a repaint per render is what this asks for and what a canvas wants. React only
 * renders this tree when the song or the chosen clip changes.
 */
function useCanvas(height: number, paint: (context: CanvasRenderingContext2D, width: number) => void) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const element = canvas.current;
    if (!element) return;
    const draw = () => {
      const width = element.clientWidth;
      const ratio = window.devicePixelRatio || 1;
      element.width = Math.max(1, Math.round(width * ratio));
      element.height = Math.max(1, Math.round(height * ratio));
      element.style.height = `${height}px`;
      const context = element.getContext("2d");
      if (!context) return;
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      context.font = "11px system-ui, sans-serif";
      context.textBaseline = "middle";
      paint(context, width);
    };
    draw();
    // The canvas is sized in device pixels from its CSS width, so a resize is a repaint.
    const observer = new ResizeObserver(draw);
    observer.observe(element);
    return () => observer.disconnect();
  });
  return canvas;
}

/** Draws `text` clipped to a box, so a long name in a short block is cut rather than spilled. */
function clipped(
  context: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  box: [number, number, number, number],
): void {
  context.save();
  context.beginPath();
  context.rect(box[0], box[1], box[2], box[3]);
  context.clip();
  context.fillText(text, x, y);
  context.restore();
}

export function Timeline({ view, selected }: { view: Arrangement; selected: string | null }) {
  const height = HEAD + view.tracks.length * ROW;

  const canvas = useCanvas(height, (context, width) => {
    const element = context.canvas;
    const surface = ink(element, "--surface");
    const panel = ink(element, "--panel");
    const line = ink(element, "--line");
    const faint = ink(element, "--faint");
    const text = ink(element, "--text");
    const dim = ink(element, "--dim");
    const block = ink(element, "--block");
    const audio = ink(element, "--audio");
    const blockText = ink(element, "--block-text");
    const accent = ink(element, "--accent");

    const x = scale(width, view.lengthTicks);

    context.fillStyle = surface;
    context.fillRect(0, 0, width, height);
    context.fillStyle = panel;
    context.fillRect(0, 0, width, HEAD);

    // Row 1: the time base, where the document changes it. A tempo change moves nothing on
    // this axis — the x axis is ticks — so the only way a reader sees one is if it is drawn.
    for (const mark of view.signatures) {
      context.fillStyle = dim;
      context.fillText(`${mark.numerator}/${mark.denominator}`, x(mark.tick) + 4, TEMPO_ROW / 2);
    }
    for (const mark of view.tempo) {
      context.fillStyle = text;
      context.fillText(`${mark.bpm} BPM`, x(mark.tick) + 30, TEMPO_ROW / 2);
      context.strokeStyle = accent;
      context.beginPath();
      context.moveTo(hairline(x(mark.tick)), 0);
      context.lineTo(hairline(x(mark.tick)), TEMPO_ROW);
      context.stroke();
    }

    // Row 2: the bar grid, and a line the whole height for each bar.
    for (const bar of view.bars) {
      const at = hairline(x(bar.startTick));
      context.strokeStyle = bar.number === 1 ? line : faint;
      context.beginPath();
      context.moveTo(at, TEMPO_ROW);
      context.lineTo(at, height);
      context.stroke();
      // Every bar number would be a grey smear at 200 bars, so they thin out as they crowd.
      const every = Math.max(1, Math.ceil(56 / Math.max(1, x(bar.ticks))));
      if ((bar.number - 1) % every === 0) {
        context.fillStyle = dim;
        context.fillText(`${bar.number}`, at + 3, TEMPO_ROW + RULER_ROW / 2);
      }
    }

    // Row 3: sections. Drawn as spans because that is what a `Section` is — a start and an end
    // tick — and a marker-style tick would lose the half of it that matters.
    const sectionTop = TEMPO_ROW + RULER_ROW;
    for (const section of view.sections) {
      const left = x(section.startTick);
      const wide = Math.max(2, x(section.endTick) - left);
      context.fillStyle = accent;
      context.globalAlpha = 0.3;
      context.fillRect(left, sectionTop + 2, wide, SECTION_ROW - 4);
      context.globalAlpha = 1;
      context.fillStyle = text;
      clipped(context, section.name, left + 5, sectionTop + SECTION_ROW / 2, [
        left,
        sectionTop,
        wide,
        SECTION_ROW,
      ]);
    }

    context.strokeStyle = line;
    context.beginPath();
    context.moveTo(0, hairline(HEAD));
    context.lineTo(width, hairline(HEAD));
    context.stroke();

    view.tracks.forEach((track, row) => {
      const top = HEAD + row * ROW;
      context.strokeStyle = faint;
      context.beginPath();
      context.moveTo(0, hairline(top + ROW));
      context.lineTo(width, hairline(top + ROW));
      context.stroke();

      for (const clip of track.clips) {
        const left = x(clip.startTick);
        // At least two pixels, so a very short clip is visible rather than invisible.
        const wide = Math.max(2, x(clip.lengthTicks));
        const box: [number, number, number, number] = [
          left + 1,
          top + GAP,
          wide - 2,
          ROW - 2 * GAP,
        ];
        context.fillStyle = clip.kind === "audio" ? audio : block;
        context.beginPath();
        context.roundRect(box[0], box[1], box[2], box[3], 3);
        context.fill();

        // A looping clip repeats its first `loopLengthTicks`; the repeats are drawn as the
        // seams they are, so a 4-bar clip looping a bar does not read as four bars of content.
        // Seams closer together than a few pixels are a smear rather than a reading — and a
        // legal document can ask for two billion of them, which is a frozen window and not a
        // slow one, so the same test is the guard.
        if (clip.loopLengthTicks !== undefined && x(clip.loopLengthTicks) >= 3) {
          context.save();
          context.beginPath();
          context.rect(box[0], box[1], box[2], box[3]);
          context.clip();
          context.strokeStyle = blockText;
          context.globalAlpha = 0.35;
          for (let at = clip.loopLengthTicks; at < clip.lengthTicks; at += clip.loopLengthTicks) {
            const seam = hairline(left + x(at));
            context.beginPath();
            context.moveTo(seam, box[1]);
            context.lineTo(seam, box[1] + box[3]);
            context.stroke();
          }
          context.restore();
        }

        if (clip.id === selected) {
          context.strokeStyle = accent;
          context.lineWidth = 2;
          context.beginPath();
          context.roundRect(box[0], box[1], box[2], box[3], 3);
          context.stroke();
          context.lineWidth = 1;
        }

        context.fillStyle = blockText;
        clipped(context, clip.label, left + 6, top + ROW / 2, box);
      }
    });
  });

  return <canvas ref={canvas} className="timeline" />;
}

export function Roll({ view }: { view: PianoRoll }) {
  const rows = view.highPitch - view.lowPitch + 1;
  const height = ROLL_HEAD + rows * NOTE_ROW;

  const canvas = useCanvas(height, (context, width) => {
    const element = context.canvas;
    const surface = ink(element, "--surface");
    const panel = ink(element, "--panel");
    const line = ink(element, "--line");
    const faint = ink(element, "--faint");
    const text = ink(element, "--text");
    const dim = ink(element, "--dim");
    const note = ink(element, "--note");
    const blockText = ink(element, "--block-text");
    const accent = ink(element, "--accent");

    const lane = Math.max(1, width - KEYS);
    const x = scale(lane, view.lengthTicks);
    // Pitch increases upwards, which is the one thing every piano roll agrees on.
    const y = (pitch: number) => ROLL_HEAD + (view.highPitch - pitch) * NOTE_ROW;

    context.fillStyle = surface;
    context.fillRect(0, 0, width, height);

    for (let pitch = view.lowPitch; pitch <= view.highPitch; pitch += 1) {
      const top = y(pitch);
      if (isBlackKey(pitch)) {
        context.fillStyle = faint;
        context.fillRect(KEYS, top, lane, NOTE_ROW);
      }
      context.strokeStyle = faint;
      context.beginPath();
      context.moveTo(KEYS, hairline(top + NOTE_ROW));
      context.lineTo(width, hairline(top + NOTE_ROW));
      context.stroke();

      // The keyboard, in the canvas rather than beside it, so a row and its key cannot drift
      // apart the way a CSS column and a canvas would. Black keys are short as well as dark,
      // which is how a keyboard is read at a glance rather than by colour alone.
      context.fillStyle = isBlackKey(pitch) ? line : text;
      context.fillRect(0, top + 1, isBlackKey(pitch) ? KEYS * 0.62 : KEYS - 2, NOTE_ROW - 2);
      if (pitch % 12 === 0) {
        context.fillStyle = surface;
        context.fillText(pitchName(pitch), 4, top + NOTE_ROW / 2);
      }
    }

    context.fillStyle = panel;
    context.fillRect(0, 0, width, ROLL_HEAD);
    for (const bar of view.bars) {
      const at = hairline(KEYS + x(bar.startTick));
      if (at < KEYS) continue;
      context.strokeStyle = line;
      context.beginPath();
      context.moveTo(at, 0);
      context.lineTo(at, height);
      context.stroke();
      context.fillStyle = dim;
      context.fillText(`${bar.number}`, at + 3, ROLL_HEAD / 2);
    }

    // Guarded as the timeline's seams are, and for both of the same reasons.
    if (view.loopLengthTicks !== undefined && x(view.loopLengthTicks) >= 3) {
      context.strokeStyle = accent;
      context.setLineDash([4, 3]);
      for (let at = view.loopLengthTicks; at < view.lengthTicks; at += view.loopLengthTicks) {
        const seam = hairline(KEYS + x(at));
        context.beginPath();
        context.moveTo(seam, ROLL_HEAD);
        context.lineTo(seam, height);
        context.stroke();
      }
      context.setLineDash([]);
    }

    for (const item of view.notes) {
      const left = KEYS + x(item.startTick);
      const wide = Math.max(2, x(item.lengthTicks));
      // A microtonal offset is a fraction of a semitone, so it is a fraction of a row. It is
      // drawn rather than rounded away: a note 50 cents sharp is not the note below it.
      const top = y(item.pitch) - (item.microtonalCents / 100) * NOTE_ROW;
      context.fillStyle = note;
      // Velocity as opacity. A roll that shows every note identically is showing half of one.
      context.globalAlpha = 0.35 + (0.65 * Math.min(127, Math.max(0, item.velocity))) / 127;
      context.beginPath();
      context.roundRect(left, top + 1, wide, NOTE_ROW - 2, 2);
      context.fill();
      context.globalAlpha = 1;
      if (wide > 26) {
        context.fillStyle = blockText;
        clipped(context, item.name, left + 3, top + NOTE_ROW / 2, [
          left,
          top,
          wide,
          NOTE_ROW,
        ]);
      }
    }

    context.strokeStyle = line;
    context.beginPath();
    context.moveTo(hairline(KEYS), 0);
    context.lineTo(hairline(KEYS), height);
    context.moveTo(0, hairline(ROLL_HEAD));
    context.lineTo(width, hairline(ROLL_HEAD));
    context.stroke();
  });

  return <canvas ref={canvas} className="roll" />;
}
