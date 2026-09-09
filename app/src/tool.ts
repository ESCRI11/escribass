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
// [`build`] is the one other command, added in PR 7, and it is not a second route to the
// model: the build manifest is what the running process says about itself.

import { invoke } from "@tauri-apps/api/core";

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
