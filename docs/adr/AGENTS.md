# AGENTS.md — docs/adr/

Architecture decision records. One precedes any change to `schema/*.proto` (`CLAUDE.md` #5; `docs/specs.md` §14.1), any new top-level directory (`CLAUDE.md`, Repo layout) and any change to a §17 pin (specs §17). What an ADR changes in `specs.md`, and how: `docs/AGENTS.md`.

## Index

| ADR | Decided, by decision number | Amended |
|---|---|---|
| `0001-song-history-as-patch-dag.md` | §1 patch log is a DAG, `parents` on every entry · §2 branches are refs in one `refs.json`, `HEAD` always names a ref, no detached state · §3 **[MUST]** every entity collection is an id-keyed map; order derived, never stored; canonical key order lexical by ULID · §4 merge auto-resolves disjoint paths, same path is a structured error, `version` resolves as max+1 · §5 ids from an injectable, seedable source · §6 git is interop only, no libgit2/gix. `## Deferred`: interactive conflict resolution (M2), GC of orphans, remotes | §3, corrected 2026-09-02: `TempoMap.events` and `TimeSignatureMap.events` are maps; "no identity" was the wrong exemption |
| `0002-song-proto-v1.md` | §1 ticks `int32`, `Generator.seed` the only 64-bit field · §2 `id`/`provenance`/`version` as plain fields on every entity, `AutomationPoint` id-only · §3 sum types are `oneof` of messages · §4 canonical JSON: snake_case, defaults emitted, map keys sorted, 2-space; writer rules for doubles, presence, timestamps · §5 `optional` only where absent ≠ zero · §6 `song.proto` and `history.proto` are separate packages, history imports song · §7 `FormRule` deferred to M4, `Marker` kept · §8 the §4.2 reductions table · §9 seed non-null structurally · §10 proto3, not Editions · §11 patch ops are canonical JSON text, `PatchEntry.ops` is `bytes` · §12 `PatchEntry.schema_version`. Consequences: the M0.2 validator rules beyond §4.4, an injectable clock, M1 snapshot strips provenance, M4 hashes | §11, revised 2026-09-02: replaced the `Op` protobuf message, "before any consumer existed" |
| `0003-milestone-scope.md` | §16 is authoritative for milestone scope; eight items that §16 had left unplaced are placed: MCP server and branching in M0 · `lock.json` created at M0, completed at M4 · bundled instruments M1 · mixer and history view M2 · a Libretto ADR before M3 · neural runtime and code views M4 · MIDI/REAPER interop and schema publishing M5. `## Still unplaced`: whether §6's analysis features are v1 at all | — |
| `0004-song-json-is-a-derived-cache.md` | §1 the patch log is authoritative on disk, `song.json` is a derived cache — not a second representation, two serialisations of one model · §2 `open` replays and compares; a mismatch is a structured error, never silently resolved either way · §3 write order entry → `song.json` → `refs.json`, and the ref flip is the commit point | — |
| `0005-version-bumping-and-undo.md` | §1 the `version` bump sits between applying ops and re-deserialising, so it lands *inside* the recorded diff · §2 an entity is an object with a string `id` and a numeric `version`; a changed entity goes to max+1, which also gives ADR 0001 §4's merge rule for free · §3 a tool-authored op writing `version` is refused, rule `version_not_writable` · §4 undo appends an inverse entry and never rewinds a ref, because a rewind decrements `version`. Undo/redo *tools* deferred to M2 | — |
| `0006-tool-api-wire-shape.md` | §1 one RPC per tool, all returning a shared `ToolResult { valid, errors, patch, summary, entry_id }` · §2 `Violation` is the only wire error type; `Ok(valid=false)` is a caller error, `Err` is an operator error · §3 `dry_run` keeps §5's name and polarity, and is the pure first half of the apply path · §4 requests reuse `song.v1` entity types, core overwrites §4.3 fields · §5 one project per process, named at launch, no `open_project` tool · §6 MCP via `rmcp`; `patch` crosses as a JSON array, never base64; `inputSchema` generated from the protobuf descriptor · §7 `proto/` generates Rust only until M2/M3 · §8 `buf lint` exceptions scoped to the `proto` module | — |

Next number: `0007`. Deferred items from any of them are tracked with revisit points in `docs/plan.md`, "Deferred, on purpose".

## Conventions, as the two files use them

| Convention | As used |
|---|---|
| File name | `NNNN-slug.md`: four digits, kebab-case slug |
| Title | `# ADR NNNN — <the decision as a sentence>` |
| Header bullets | `Status`, `Affects` (files and specs §), `Supersedes` (0001) or `Builds on` (0002), `Recorded in` (the specs sections changed; always §15) |
| Status | Only `Accepted (YYYY-MM-DD)` occurs. Nothing has been proposed-then-accepted or superseded; 0001's `Supersedes: nothing` is the header slot for the latter |
| Body | `## Context` · `## Decision` or `## Decisions`, with `### N. <title>` per decision · `## Consequences`. 0001 adds `## Alternatives considered` (tables: alternative / rejected because) and `## Deferred` |
| Citing | From outside: `ADR 0002 §11` is decision 11 of that file. Inside an ADR, bare `§5` is `specs.md` §5; its own decisions are "decision 4" or "§4 above" |
| ADR vs code | 0001 preceded the proto (`2bc4442`, then `c0dc9da`); 0002 landed in the same commit as the proto (`c0dc9da`). Code is never earlier than its ADR (`CLAUDE.md` #5) |

## Accepting one — in the same change

| Also in the change | Set by |
|---|---|
| A `specs.md` §15 row citing the ADR and decision, `(ADR 0002 §11)` | 0001 and 0002 `Recorded in:`; the §15 rows dated 2026-09-02 |
| Every `specs.md` section whose meaning the ADR changes, rewritten and listed under `Recorded in:` | 0001: "§5 and §10 amended to match" |
| Schema ADR: the `.proto`, `schema/gen/`, `/tests/fixtures/song/minimal.json` — same PR or later, never earlier (`schema/AGENTS.md`, Adding things) | `CLAUDE.md` #5; `c0dc9da`, `b402dc0` |
| Dependency or pin ADR: `lock.baseline.json` | `CLAUDE.md` #4; specs §17 |
| New-directory ADR: its line in specs §13 | `CLAUDE.md`, Repo layout |

## Amending an accepted one — in place

Done twice, both in `b402dc0`:

- The decision's own section is edited. It opens with a dated bold lead: `**Corrected 2026-09-02:**` (0001 §3, appended as a corollary bullet) or `**Revised 2026-09-02, before any consumer existed.**` (0002 §11, retitled and rewritten).
- The text states what the earlier version said and why it was wrong; the old wording survives only in git.
- `Status` keeps `Accepted (2026-09-02)`, the original date.
- `specs.md` §15 gets a row for the amendment, cited as `(ADR 0001 §3, corrected)`.
- Other ADR text that relied on the old decision changes in the same commit: 0002 §8's last row points at 0001's correction; 0001 §3 and 0002 §2 both had their example path fixed to `/clips/…/note_clip/notes/{id}/pitch`.
- Code and fixtures the amendment invalidates change in the same commit.

0002 §11 records the condition it was amended under: no consumer existed. Superseding by a new ADR has not been used.
