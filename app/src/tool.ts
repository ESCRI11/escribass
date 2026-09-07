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
