# AGENTS.md — docs/

Prose only: the architecture spec, its decision records, one research report, one wireframe set, two planning trackers. Nothing here is built, generated or tested. Repo-wide rules and checks: `/AGENTS.md`. The ADRs: `adr/AGENTS.md`.

## Files

| Path | What | Binding? |
|---|---|---|
| `specs.md` | Architecture source of truth (its line 5). Header: `draft v0.4`, last updated 2026-09-02. **[MUST]** sections: §2, §4, §5, §11, §14, §17; §18 is "[MUST read before roadmap changes]". | Yes — the only document that is |
| `adr/` | One record per decision; each lands with its `specs.md` §15 row. See `adr/AGENTS.md`. | Through §15 |
| `landscape-2026-09.md` | Competitive and academic research of 2026-09-02, companion to §18. Its line 3: "Point-in-time; re-check quarterly"; §18.3's heading sets the same cadence. Its "Strategic implications" defers to §18 as "the binding version". | No |
| `wireframes.html` | Six plates of the §9 desktop shell (M2), `draft 0.1`, footer "keyed to docs/specs.md v0.4". Its header: "Nothing here is built". Its "What the spec dictates / What I invented here" section separates §9's requirements from its own layout choices. Plates are cited by number from ADR 0002 (§2: Plate 2; Consequences: Plate 3). | No |
| `plan.md` | Status per milestone step, deferred items with revisit points, known gaps, the [OPEN] list, and a closing retrospective per milestone. Its header: milestones are §16's and rules are `CLAUDE.md`'s — "when they disagree, they win and this file is stale". Linked from `CLAUDE.md`, current-milestone section. | No |
| `roadmap.md` | Milestones in user terms, the dependency spine, what v1 is not, what would change course. Its own header: the least authoritative of the three planning documents — §16 wins, then `plan.md`; "No dates". | No |

## Reading `specs.md`

| Notation | Meaning | Source |
|---|---|---|
| `[MUST]` on a heading | Binding; read before writing code | specs line 5; `CLAUDE.md` line 4 |
| `[OPEN]` | Undecided, and not for an agent to decide: stop and ask. They are the "Remaining open items" paragraph that closes §15 — **four** since ADR 0003 added the fourth on 2026-09-02, and **three** since 2026-09-25, when the user resolved the neural runtime's packaging on the day ADR 0003 §7 said it was due; this line said three until 2026-09-07, and `roadmap.md` had counted four the whole time | `CLAUDE.md` line 4; specs line 5 |
| `§N.M` where §N has subsection headings | The subsection: §4.4 is the validator invariants | §4, §7, §18 |
| `§N.M` where §N is a numbered list | Item M: §2.6 is text-first persistence, §14.1 is "read §2, §4, §5, §11", §18.1.3 is differentiator 3 | usage in §15; ADR 0001 §5, §6; ADR 0002 §1, §6 |

## Amending `specs.md`

| Change | Goes with it | Set by |
|---|---|---|
| Any decision | A §15 row — `Decision · Chosen · Rationale · Date` — citing its ADR and decision number where one exists: `(ADR 0002 §11)` | §15; ADR headers, `Recorded in:` |
| A decision touching `schema/*.proto` | The ADR first; code never earlier than the ADR | `CLAUDE.md` #5; §14.1; `adr/AGENTS.md` |
| A decision that changes what a section says | That section rewritten in the same change; the ADR's `Recorded in:` names it | ADR 0001 header: "§5 and §10 amended to match" |
| Resolving an [OPEN] item | A human decides. Recording it: a §15 row; the item leaves the "Remaining open items" list | `CLAUDE.md` line 4; specs line 5 |
| A new top-level directory | An ADR, then its line in the §13 tree | `CLAUDE.md`, Repo layout |
| A pin in §17 | An ADR, and a full golden-render pass **if the pin can reach a render**; a pin that cannot says why in its ADR rather than skipping in silence (§17's last rule, 2026-09-07). `lock.baseline.json` mirrors the table and changes with it | §17 intro and rules; `lock.baseline.json` `notes`; ADR 0016 §4 |
| Anything in §18 | Must not contradict `landscape-2026-09.md`; that file's quarterly re-check falls due 2026-12 | §18 intro; §18.3 |
| Numbering | Section and list-item numbers are cited from `CLAUDE.md`, `buf.yaml`, both ADRs, `wireframes.html`, `plan.md`, `roadmap.md`, every `AGENTS.md`, both `schema/*.proto`, `schema/Cargo.toml`, `schema/buf.gen.yaml`, `schema/codegen.sh`, `schema/src/lib.rs` and the three round-trip tests. New material goes at the end of a section or list; nothing is renumbered | `grep -rl '§' --exclude-dir={node_modules,.venv,target,gen} .` |

The header line (`Status: draft v0.4 · … · Last updated: 2026-09-02`) was not bumped by `b402dc0`'s §15 and §17 amendments; there is no version-bump convention yet. `wireframes.html`'s footer names `v0.4`.
