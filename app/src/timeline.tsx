// The timeline, drawn on a canvas (§15, Frontend: "React + canvas/WebGL timeline").
//
// Canvas rather than a div per clip because §15 decided it and because the reason it gives —
// rendering is custom either way — is already true here: a bar grid and a clip block are
// shapes, not documents. What this file is *not* is a renderer with a scene graph. It takes
// the projection [`arrangement`] produced and paints it, once, whenever it changes.
//
// `ponytail:` no zoom, no scroll, no hit testing and no selection. The whole song is scaled to
// the width it has. PR 4 completes the arrangement and PR 5 makes a clip draggable; both need
// a tick↔pixel mapping that survives a viewport, and inventing one here against no interaction
// would be inventing it twice.

import { useEffect, useRef } from "react";
import type { Arrangement } from "./arrangement.js";

const ROW = 30;
const RULER = 22;
const GAP = 3;

/** Reads a CSS custom property off the canvas, so the palette lives in one place. */
function ink(element: HTMLElement, name: string): string {
  return getComputedStyle(element).getPropertyValue(name).trim();
}

function paint(canvas: HTMLCanvasElement, view: Arrangement): void {
  const width = canvas.clientWidth;
  const height = RULER + view.tracks.length * ROW;
  const ratio = window.devicePixelRatio || 1;
  canvas.width = Math.round(width * ratio);
  canvas.height = Math.round(height * ratio);
  canvas.style.height = `${height}px`;

  const context = canvas.getContext("2d");
  if (!context) return;
  context.setTransform(ratio, 0, 0, ratio, 0, 0);

  const surface = ink(canvas, "--surface");
  const line = ink(canvas, "--line");
  const faint = ink(canvas, "--faint");
  const text = ink(canvas, "--text");
  const block = ink(canvas, "--block");
  const blockText = ink(canvas, "--block-text");

  context.fillStyle = surface;
  context.fillRect(0, 0, width, height);

  const perTick = width / view.lengthTicks;
  const bars = Math.round(view.lengthTicks / view.ticksPerBar);
  const barWidth = view.ticksPerBar * perTick;

  // The ruler, then a grid line down the whole height per bar.
  context.font = "11px system-ui, sans-serif";
  context.textBaseline = "middle";
  for (let bar = 0; bar < bars; bar += 1) {
    const x = Math.round(bar * barWidth) + 0.5;
    context.strokeStyle = bar === 0 ? line : faint;
    context.beginPath();
    context.moveTo(x, 0);
    context.lineTo(x, height);
    context.stroke();
    context.fillStyle = text;
    context.fillText(`${bar + 1}`, x + 4, RULER / 2);
  }
  context.strokeStyle = line;
  context.beginPath();
  context.moveTo(0, RULER + 0.5);
  context.lineTo(width, RULER + 0.5);
  context.stroke();

  view.tracks.forEach((track, row) => {
    const top = RULER + row * ROW;
    context.strokeStyle = faint;
    context.beginPath();
    context.moveTo(0, top + ROW + 0.5);
    context.lineTo(width, top + ROW + 0.5);
    context.stroke();

    for (const clip of track.clips) {
      const x = clip.startTick * perTick;
      // At least two pixels, so a very short clip is visible rather than invisible.
      const w = Math.max(2, clip.lengthTicks * perTick);
      context.fillStyle = block;
      context.beginPath();
      context.roundRect(x + 1, top + GAP, w - 2, ROW - 2 * GAP, 3);
      context.fill();
      context.save();
      context.beginPath();
      context.rect(x + 1, top + GAP, w - 2, ROW - 2 * GAP);
      context.clip();
      context.fillStyle = blockText;
      context.fillText(clip.label, x + 6, top + ROW / 2);
      context.restore();
    }
  });
}

export function Timeline({ view }: { view: Arrangement }) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const element = canvas.current;
    if (!element) return;
    const draw = () => paint(element, view);
    draw();
    // The canvas is sized in device pixels from its CSS width, so a resize is a repaint.
    const observer = new ResizeObserver(draw);
    observer.observe(element);
    return () => observer.disconnect();
  }, [view]);

  return <canvas ref={canvas} className="timeline" />;
}
