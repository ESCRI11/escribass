// The two forms: the mixer and the generic parameter editor (§9; ADR 0014 §1, ADR 0015 §2).
//
// One file because they are one form twice. A strip is a row of controls over `Mix`, a
// parameter row is a row of controls over a plugin's parameter map, and the control is the
// same control — a native `<input type="range">`, which is [`Fader`] and is nine lines. There
// is no form framework and no component library here for the same reason `canvas.tsx` has no
// scene graph: what these views need is a slider and a label (ADR 0016 §3).
//
// **What is not the same is the number.** The mixer writes the *model's own* units — `gain_db`
// in decibels, unbounded, and `pan` in `-1..1` — because a `Mix` field is the model's and has
// no plugin behind it to normalise against (ADR 0015 §2). The parameter editor writes a
// plugin's **normalised** `0..1`, because VST3 exposes exactly one numeric domain to a host
// (ADR 0010 §4). Every one of those three accepts `0.5`, and it means a quiet fader, a little
// to the right, and mid-range. So the domain is carried as a value and handed to the control,
// which is `core/src/validate.rs`'s own arrangement one process over: there the domain is
// decided where the `ParamRef` resolved and handed to `check_range`, "never inferred at the
// check, where a bare double could only be guessed at". [`Domain`] is that enum, in the form
// that draws the number rather than the rule that judges it, and the three arms are the same
// three.
//
// **And the call is not the same either**, which is the half a shared control could quietly
// blur. [`paramWrite`] is `set_param`; [`mixWrite`] is `apply_patch`. `set_param` resolves its
// `device_id` to an instrument or an effect and nothing else, so a track id reaches it as
// `device_unknown` — measured, not assumed — and `mute` and `solo` are not parameters at all
// (ADR 0015 §1). Both calls are the tool API, both take `dry_run`, and both are validated;
// they are simply not the same tool, and the two builders are two functions so that nothing
// has to decide which one it is at the point a pointer moves.
//
// Nothing here holds state. A fader is drawn from the model, or from the gesture in flight
// when the gesture is this one's, and never from a number of its own — a fader with local
// state is the second representation of song state CLAUDE.md #1 forbids, wearing the one
// disguise nobody questions.

import type { JsonValue } from "@bufbuild/protobuf";
import type { Mixer, Strip } from "./mixer.js";
import type { DeviceEditor, ParamRow } from "./params.js";

/**
 * What a number in this form is a number **in**, and therefore what the control's travel and
 * its readout mean. `core/src/validate.rs`'s `Domain`, in the same three arms.
 *
 * `min` and `max` are the control's *travel*, not the model's range: `gain_db` is unbounded in
 * the model on purpose, since a ceiling here would be the invented maximum gain ADR 0015 §2
 * rejected. [`Fader`] grows the travel to hold a value outside it rather than clamping, which
 * is `canvas.tsx`'s rule for the roll's pitch band arriving at a slider (ADR 0017 §3).
 */
export interface Domain {
  readonly min: number;
  readonly max: number;
  readonly step: number;
  readonly show: (value: number) => string;
}

/**
 * A plugin parameter's own normalised `0..1`, and **no unit** — deliberately.
 *
 * ADR 0014 §1 wrote this limit down in advance rather than leaving it to be discovered: VST3
 * exposes one numeric domain to a host and a plugin's own units exist only as a display string
 * a live instance produces, so a user cannot read "−6 dB" off this row and this form does not
 * pretend otherwise. Formatting it as anything but a bare number would be inventing the unit.
 */
export const NORMALISED: Domain = { min: 0, max: 1, step: 0.001, show: (v) => v.toFixed(3) };

/**
 * `Mix.gain_db`. The travel is −60 dB to +6 dB and the model is unbounded either side.
 *
 * `ponytail:` −60 is where a fader stops being useful rather than where the model stops, and
 * +6 is the ceiling the engine actually has — Tracktion's volume parameter tops out there, so
 * a `Mix.gain_db` above +6 renders as +6 (ADR 0015 §2, extended). A value outside the travel
 * is still drawn and still readable, because the control grows to hold it; the upgrade path,
 * if a project ever needs more, is the one that ADR names — a gain ahead of the fader, as an
 * audio clip has (ADR 0011 §2) — and not a wider slider.
 */
export const DECIBELS: Domain = {
  min: -60,
  max: 6,
  step: 0.1,
  show: (v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`,
};

/** `Mix.pan`, `-1.0` hard left to `1.0` hard right, read as a side and a percentage because
 *  "−0.25" is not how anybody says it. */
export const PAN: Domain = {
  min: -1,
  max: 1,
  step: 0.01,
  show: (v) =>
    Math.abs(v) < 0.005 ? "C" : `${v < 0 ? "L" : "R"}${Math.round(Math.abs(v) * 100)}`,
};

/**
 * The control a gesture is holding: which one, and the number the pointer is at.
 *
 * Not song state and not a copy of any — the same category as `canvas.tsx`'s [`Moved`], and
 * held for the same reason. A `<input type="range">` is a controlled element, so during a drag
 * its value has to come from somewhere, and the model has not moved yet: it comes from here,
 * and it stops existing when the gesture does. `key` identifies the control and is compared
 * only for equality.
 */
export interface Touch {
  readonly key: string;
  readonly value: number;
}

/** One tool call, built where the control that makes it is drawn. */
export interface Write {
  readonly tool: string;
  readonly args: Record<string, JsonValue>;
  readonly touched: Touch;
}

/**
 * A `Mix` field, written as the RFC 6902 operation it is.
 *
 * `apply_patch` and not `set_param`: `set_param` resolves a `device_id` to an instrument or an
 * effect (`core/src/tools.rs`, `device_path`), so a track id reaches it as `device_unknown`,
 * and `mute` and `solo` are not parameters in any sense — ADR 0015 §1 refuses them as
 * `ParamRef` targets because a double cannot address a boolean without a threshold rule that
 * would be ours and pinned. All four go the same way, which is the honest reading of "a `Mix`
 * is a field on `Track`": there is no parameter here to set.
 *
 * `add` rather than `replace`, for `core/src/tools.rs`'s reason — on an existing object member
 * RFC 6902 `add` replaces, so one op covers both cases and nothing has to ask the document
 * which it is in. What comes back in the diff is core's own re-derived `replace`, with the two
 * `version` bumps beside it (ADR 0005 §1).
 */
export function mixWrite(
  trackId: string,
  field: "gain_db" | "pan" | "mute" | "solo",
  value: number | boolean,
): Write {
  return {
    tool: "apply_patch",
    args: { patch: [{ op: "add", path: `/tracks/${trackId}/mix/${field}`, value }] },
    touched: { key: `${trackId}/${field}`, value: Number(value) },
  };
}

/** A plugin parameter, written as `set_param` — which is the tool ADR 0014 §1 said this view
 *  would need and found already there. `param` is the `ParamID`, never the display name. */
export function paramWrite(deviceId: string, param: string, value: number): Write {
  return {
    tool: "set_param",
    args: { device_id: deviceId, param, value },
    touched: { key: `${deviceId}/${param}`, value },
  };
}

/**
 * The one control both forms are made of.
 *
 * The travel grows to hold a value outside it instead of clamping to it. Clamping would move
 * the fader to a number nobody asked for the instant it was touched, which is the slider's
 * version of following a dragged note off the canvas — and `canvas.tsx` decided that one the
 * same way (ADR 0017 §3).
 *
 * `onChange` is React's name for the DOM `input` event, so it fires continuously through a
 * drag: every position the pointer passes through becomes a `dry_run` and is answered by the
 * validator, which is ADR 0017 §2 with no timer and no debounce interval. `onHeld` is separate
 * from it because **nothing may reflow while a pointer is down** (ADR 0017 §3): the diff pane
 * takes space, and space taken under a slider moves the slider out from under the thumb.
 */
function Fader({
  domain,
  value,
  label,
  onChange,
  onHeld,
}: {
  domain: Domain;
  value: number;
  /** What this control is, for a reader who cannot see the text beside it. */
  label: string;
  onChange: (value: number) => void;
  onHeld: (down: boolean) => void;
}) {
  return (
    <input
      type="range"
      aria-label={label}
      aria-valuetext={domain.show(value)}
      min={Math.min(domain.min, value)}
      max={Math.max(domain.max, value)}
      step={domain.step}
      value={value}
      onPointerDown={() => onHeld(true)}
      onChange={(event) => onChange(event.currentTarget.valueAsNumber)}
      onPointerUp={() => onHeld(false)}
      // A pointer the window took away mid-drag — focus lost, a gesture cancelled — ends the
      // hold exactly as a release does. Without it the diff pane would wait for a release
      // that is never coming.
      onPointerCancel={() => onHeld(false)}
      onLostPointerCapture={() => onHeld(false)}
    />
  );
}

/** What the control shows: the gesture's number if this is the control it is holding, and the
 *  model's otherwise. One line, and it is the whole of why no fader here keeps a value. */
function at(touched: Touch | undefined, key: string, held: number): number {
  return touched?.key === key ? touched.value : held;
}

/**
 * The mixer (§9, plate 4 of the wireframes).
 *
 * Rows rather than the plate's columns, because a row is what the parameter editor is and this
 * pull request's claim is that they are one form. Everything a strip draws is `mixer.ts`'s
 * projection; nothing is computed here.
 */
export function Strips({
  view,
  touched,
  onWrite,
  onHeld,
  onOpen,
  opened,
}: {
  view: Mixer;
  touched: Touch | undefined;
  onWrite: (write: Write) => void;
  onHeld: (down: boolean) => void;
  /** Opening a device's editor is not an edit: nothing is written and the same song is
   *  projected a second way. */
  onOpen: (deviceId: string) => void;
  opened: string | undefined;
}) {
  return (
    <div className="strips">
      {view.strips.map((strip) => (
        <Row
          key={strip.id}
          strip={strip}
          touched={touched}
          onWrite={onWrite}
          onHeld={onHeld}
          onOpen={onOpen}
          opened={opened}
        />
      ))}
    </div>
  );
}

function Row({
  strip,
  touched,
  onWrite,
  onHeld,
  onOpen,
  opened,
}: {
  strip: Strip;
  touched: Touch | undefined;
  onWrite: (write: Write) => void;
  onHeld: (down: boolean) => void;
  onOpen: (deviceId: string) => void;
  opened: string | undefined;
}) {
  const gain = at(touched, `${strip.id}/gain_db`, strip.gainDb);
  const pan = at(touched, `${strip.id}/pan`, strip.pan);
  // A toggle is a gesture too, and it proposes rather than commits — ADR 0017 §4 is the flow
  // for every gesture in M2 and a second one for buttons would be the second interaction model
  // the ADR exists to prevent. So the button draws the *proposal* while one is held, marked as
  // unapplied, exactly as the roll draws its dashed note over the model's own.
  const toggle = (field: "mute" | "solo", on: boolean) => {
    const key = `${strip.id}/${field}`;
    const proposed = touched?.key === key;
    const drawn = proposed ? touched.value === 1 : on;
    return (
      <button
        className={`toggle${drawn ? " on" : ""}${proposed ? " proposed" : ""}`}
        onClick={() => onWrite(mixWrite(strip.id, field, !on))}
        aria-pressed={drawn}
        title={field}
      >
        {field === "mute" ? "M" : "S"}
      </button>
    );
  };

  return (
    <div className="strip">
      <span className="name">{strip.name}</span>
      <span className="kind">{strip.kind}</span>
      <span className="devices">
        {strip.devices.length === 0 ? (
          <span className="dim">no devices</span>
        ) : (
          strip.devices.map((device) => (
            <button
              key={device.id}
              className={`device${device.id === opened ? " opened" : ""}`}
              onClick={() => onOpen(device.id)}
              // The count is the document's own overrides, not the plugin's parameter count:
              // `Instrument.params` is sparse and this is how many of them exist.
              title={`${device.kind}${device.index === undefined ? "" : ` #${device.index}`} · ${device.set} set`}
            >
              <span className="label">{device.label}</span>
              {device.set > 0 ? <span className="set">{device.set}</span> : null}
            </button>
          ))
        )}
      </span>
      <Fader
        domain={DECIBELS}
        value={gain}
        label={`${strip.name} gain`}
        onChange={(value) => onWrite(mixWrite(strip.id, "gain_db", value))}
        onHeld={onHeld}
      />
      <span className="readout">{DECIBELS.show(gain)}</span>
      <Fader
        domain={PAN}
        value={pan}
        label={`${strip.name} pan`}
        onChange={(value) => onWrite(mixWrite(strip.id, "pan", value))}
        onHeld={onHeld}
      />
      <span className="readout pan">{PAN.show(pan)}</span>
      {toggle("mute", strip.mute)}
      {toggle("solo", strip.solo)}
    </div>
  );
}

/**
 * The generic parameter editor (ADR 0014 §1).
 *
 * One row per parameter the plugin declares, plus any the document holds that it does not.
 *
 * `ponytail:` every row is rendered, all 2855 of Surge XT's — no windowing and no virtual list.
 * The ceiling is measured rather than guessed: **1.65–1.79 s to first paint** for Surge XT
 * against 270–380 ms for the mixer and the roll, in the window. `content-visibility: auto` in
 * the stylesheet is what is spent on it, because it is a native property rather than a
 * dependency, and it is a fifth of the cost and not a fix. The search box below is what makes
 * 2855 rows a list a person can *use*; it is not what makes them fast, and saying so is the
 * point — the upgrade path after this is a virtual list, on the trigger in `docs/plan.md`.
 */
export function Params({
  view,
  touched,
  filter,
  onWrite,
  onHeld,
}: {
  view: DeviceEditor;
  touched: Touch | undefined;
  filter: string;
  onWrite: (write: Write) => void;
  onHeld: (down: boolean) => void;
}) {
  const wanted = filter.trim().toLowerCase();
  const rows = wanted
    ? view.rows.filter(
        (row) => row.name.toLowerCase().includes(wanted) || row.id.includes(wanted),
      )
    : view.rows;
  if (rows.length === 0) {
    return (
      <p className="empty">
        {view.declares === 0
          ? `This build declares no parameters for ${view.device}. A device whose parameters come from a source the compilers read declares them there, and a sampler's are the SFZ's (ADR 0014 §1).`
          : `No parameter of ${view.device} matches “${filter}”.`}
      </p>
    );
  }
  return (
    <div className="params">
      {rows.map((row) => (
        <Param
          key={row.id}
          row={row}
          deviceId={view.deviceId}
          touched={touched}
          onWrite={onWrite}
          onHeld={onHeld}
        />
      ))}
    </div>
  );
}

function Param({
  row,
  deviceId,
  touched,
  onWrite,
  onHeld,
}: {
  row: ParamRow;
  deviceId: string;
  touched: Touch | undefined;
  onWrite: (write: Write) => void;
  onHeld: (down: boolean) => void;
}) {
  const key = `${deviceId}/${row.id}`;
  const proposed = touched?.key === key;
  // An unset parameter has no value at all — the plugin's own default is not in the manifest,
  // so this view does not know it and does not draw it as zero. The slider has to sit
  // somewhere, so it sits at the bottom of its travel and the readout says `unset` rather than
  // `0.000`; moving it is what writes the first value.
  const unset = row.value === null && !proposed;
  const value = at(touched, key, row.value ?? NORMALISED.min);
  return (
    <div className={`param${unset ? " unset" : ""}${row.declared ? "" : " undeclared"}`}>
      <span className="pname">{row.name || "not declared by this build"}</span>
      <span className="pid">{row.id}</span>
      <Fader
        domain={NORMALISED}
        value={value}
        // The id and not the name, for the reason the form is keyed by it: a screen reader
        // announcing "Drive" on twelve different controls is the collapse this view avoids.
        label={`${row.name || "undeclared"} (${row.id})`}
        onChange={(next) => onWrite(paramWrite(deviceId, row.id, next))}
        onHeld={onHeld}
      />
      <span className="readout">{unset ? "unset" : NORMALISED.show(value)}</span>
    </div>
  );
}
