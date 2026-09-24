// The frontend's only route to the model (ADR 0012 §1).
//
// One command carrying a tool name and its arguments, not one command per tool: twenty
// `#[tauri::command]` functions are twenty places for a field to be forgotten, and the host
// dispatches this into the same `escribass_core::call` the MCP server dispatches into. So the
// UI is on a path M0.4's determinism suite already drives through a real process and compares
// against gRPC, rather than on a second one free to drift from it.
//
// It is not a wire. `app` is one process with `core` linked into it and opens no network port
// (§3); this is a function call with a JSON round trip in the middle.
//
// [`build`] and [`assistant`] are the other two commands, and neither is a second route to the
// model: one is what the running process can host, the other is whether the process it started
// is still running.
//
// [`panel`], [`ask`] and [`settle`] are M3 PR 9's, and none of them is a second route either:
// a prompt's calls come back over the sidecar's stream and the host executes each one through
// the same `escribass_core::call` (ADR 0019 §1, ADR 0020 §1), so what the model reaches is the
// dispatch above, with a proposal in place of the project.

import { invoke } from "@tauri-apps/api/core";
import type { PanelAnswer } from "./assistant.js";

/**
 * Calls one tool and returns the document it answered with.
 *
 * A call the session *refused* is an answer, not a throw: it comes back as the `ToolResult`
 * the tool API defines, with `valid: false` and every rule it broke (ADR 0006 §2). What
 * rejects is a call that was never answered — an unknown tool, unreadable arguments, or an
 * operator error such as a project that will not open.
 */
export function tool(name: string, args: Record<string, unknown>): Promise<unknown> {
  return invoke<unknown>("tool", { name, args });
}

/**
 * What this build can host (ADR 0010 §4) — the map §9's seventh view is a form over.
 *
 * The **second** command, and the only other thing the webview can reach. It is not a tool and
 * not the model: it answers a question about the running process — which plugins it hosts and
 * which parameters each declares — and nothing in `IMPLEMENTED` reads it, because a build fact
 * on the tool surface would be a plugin catalogue handed to the AI as a side effect of drawing
 * a form (`app/src-tauri/src/main.rs`, `manifest`).
 *
 * Read once. It describes the process, not the project, and cannot change under a window.
 */
export function build(): Promise<unknown> {
  return invoke<unknown>("manifest");
}

/** What the AI sidecar is doing, as the host reads it (ADR 0020 §5). */
export type Assisted = {
  /** `unset` — none was started; `running`; `gone` — it exited, and `said` is how. */
  state: "unset" | "running" | "gone";
  /** The child's exit status and the tail of its stderr, or why it never started. */
  said: string;
};

/**
 * Whether the sidecar this window started is still running.
 *
 * The **third** command, and of the second's kind: a question about this process, not about
 * the song. It asks nothing *of* the assistant — there is nothing to ask until the loop lands
 * (M3 PR 8) — and what comes back is the child's exit status, read. Never a claim about the
 * model: a hosted model verifies nothing, and a readout that said otherwise would be the
 * "14/14 verified" this window already refuses to draw (docs/plan.md, M3 trap 15).
 */
export function assistant(): Promise<Assisted> {
  return invoke<Assisted>("assistant");
}

/**
 * What the AI panel is drawing: the conversation, and the turn in flight or pending.
 *
 * The **fourth** command. It reads nothing of the song and takes no session lock in the host,
 * which is what lets it be asked while a turn is running — a turn holds the session for as
 * long as the model takes, and the proposal is drawn as it grows from what the host's watcher
 * left behind (ADR 0019 §2).
 */
export function panel(): Promise<PanelAnswer> {
  return invoke<PanelAnswer>("panel");
}

/**
 * Sends one prompt, and returns as soon as the turn has started (ADR 0020 §3).
 *
 * Not when the model has answered: a command that waited would hold a host thread for the
 * length of a hosted model's turn with nothing on screen until it returned. What comes back
 * is a rejection when there is nothing to ask or when this window already has a turn — one
 * proposal at a time (ADR 0019 §3).
 */
export function ask(text: string): Promise<void> {
  return invoke<void>("prompt", { text });
}

/**
 * Apply, Reject or Edit — §9's three controls (ADR 0019 §3).
 *
 * A refusal is an answer, as it is for [`tool`]: Apply meets §4.3's `version` check when the
 * document moved under the proposal, Edit meets the validator, and both leave the proposal
 * **pending** so a person can decide again (ADR 0017 §3).
 */
export function settle(
  action: "apply" | "reject" | "edit",
  patch?: string,
): Promise<{ valid: boolean; errors: readonly { path: string; rule: string; message: string }[] }> {
  return invoke("settle", { action, patch });
}
