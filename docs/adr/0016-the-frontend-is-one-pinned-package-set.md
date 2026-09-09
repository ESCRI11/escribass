# ADR 0016 — `app`'s frontend is one pinned package set, and the thing it renders in is not pinnable at all

- **Status:** Accepted (2026-09-07)
- **Affects:** `lock.baseline.json` (`app`); `docs/specs.md` §17; `app/package.json` and
  `app/package-lock.json` (M2, PR 2)
- **Builds on:** ADR 0006 (§17's registry-package rule, and `proto/` generating TypeScript at
  M2); ADR 0012 §2 (the frontend holds one decoded `Song` and nothing else), §5 (the projection
  golden); ADR 0010 (a pin ADR carries `lock.baseline.json` with it)
- **Recorded in:** `docs/specs.md` §15 and §17.

## Context

`lock.baseline.json` says `app.react.version: null` and `app.codemirror.version: "6.x"`. Neither
is a pin. §15 decided React in 2026-09 and §3 lists CodeMirror 6, so the *choices* were made;
what was never made is the pin, and a range is not one — `6.x` resolves to a different tree every
month, which is the exact failure §17 exists to prevent, sitting inside the file that exists to
prevent it.

The rest of §17 is pinned by commit. `app/` cannot be: npm packages have no commit, which §17
already anticipated — registry packages are recorded by exact version with their integrity
hashes in a committed lockfile, and `schema.typescript` has been enumerated that way since M0.1.
So the small half of this decision is "apply the rule that exists".

The large half is that the frontend's dependency tree is bigger than everything else in the
repository combined, and that the thing it actually renders in — WebKitGTK on Linux, WKWebView
on macOS, WebView2 on Windows — belongs to the operating system, moves under the user, and
cannot be pinned by anybody (trap 7).

## Decisions

### 1. `app/`'s direct dependencies are enumerated by exact version; the tree is the lockfile's

`lock.baseline.json`'s `app` block lists every **direct** dependency by exact version, in the
shape `schema.typescript` already uses, and `app/package-lock.json` carries the integrity hashes
of the tree those resolve to. That is §17's registry rule unchanged, applied to a fourth
package set.

Enumerating the direct dependencies rather than recording the lockfile by reference is what makes
the file readable as a decision log. `schema.typescript` is nine entries a reviewer can scan; a
sentence saying "see `app/package-lock.json`" is a pointer to eight hundred packages and says
nothing about which of them anyone chose. The lockfile is the integrity record; this table is the
choice.

### 2. The set, and it is one sign-off

```
react                19.2.8      react-dom          19.2.8
@types/react         19.2.18     @types/react-dom   19.2.7
vite                 8.2.2       @tauri-apps/api    2.11.1
@tauri-apps/cli      2.11.4      codemirror         6.0.2
```

and three that are already pinned and gain a consumer rather than an entry: `typescript` 5.9.3,
`@bufbuild/protobuf` 2.14.1 (the runtime the generated TypeScript imports) and `tsx` 4.23.13
(which runs the projection golden under `node:test`), all three of them
`lock.baseline.json`'s `schema.typescript` already.

**This whole list is the sign-off under CLAUDE.md #4, taken once.** Anything added beyond it —
a router, a virtualiser, an icon set, a date picker — goes back to the user before it is
installed, and a transitive dependency arriving because a direct one moved is a pin change and
therefore an ADR. The reason to take it as one list rather than nine approvals is that a
frontend's dependency count is not a sequence of small decisions; it is one decision about how
many small decisions to allow.

**Amended 2026-09-08 in M2 PR 4: a fourth of those, `@types/node` 24.13.3.** The list above was
written against a golden that had not been written yet, and it left out the one package the
decision it serves requires. §3 below rules out a test framework because `node:test` and `tsx`
suffice — and a file importing `node:test`, `node:assert` and `node:fs` has no types for any of
them, so `tsc --noEmit` cannot check it. The choices were to leave the golden outside the type
check, which is the silently-skipped check this repository keeps finding, or to add a package
that is **already a §17 pin**, at the same version, for the same reason as the other three: it
gains a consumer, not an entry. It is a `@types` package, so it reaches no runtime and cannot
reach a render (§4 below). This is an amendment rather than a new sign-off because CLAUDE.md #4
asks about dependencies not in `lock.baseline.json`, and this one has been in it since M0.1.

Two entries need their own sentence.

**`@tauri-apps/cli` 2.11.4 and `@tauri-apps/api` 2.11.1** are the JavaScript halves of a Tauri
already pinned at 2.11.5 as a Rust crate. They are separate packages with separate version lines
and §17's Tauri row lists the first two of the three; the row now names all three.

**`codemirror` 6.0.2 has no consumer until M4.** Code views are ADR 0003 §8's, and §9 places
them there. Pinning it now is not scope creep — it is deleting the `"6.x"` that is in the file
today, because a wrong pin is worse than a pin for something unbuilt: the range would be
re-resolved silently on the first `npm install` in M4 and nobody would read it as a change. It is
pinned, unused, and noted as unused, in the shape §17 already carries for Faust ("import path
only; pin hash when vendored").

### 3. What is deliberately absent, and what each absence costs

An empty row in a dependency list is a decision, and these are the ones a React application
usually makes without noticing:

| Not taken | What it would have done | Why not |
|---|---|---|
| A state library (Redux, Zustand, Jotai, TanStack Query) | Hold the song | ADR 0012 §2 holds one decoded `Song` and re-reads it. Every one of these is a store, and a store over a document is the normalised second representation §14.2 forbids — arriving as a dependency rather than as a design decision, which is trap 1's whole shape. |
| A test framework (Vitest, Jest) | Run the projection golden | `node:test` plus `tsx`, both already here — with `@types/node` for the same three imports, added in §2's amendment. A golden comparison is `assert.deepEqual` against a committed file; a framework buys watch mode and mocking, and mocking is what a pure selector does not need. |
| A component or CSS framework (MUI, Tailwind, shadcn) | Look like something | A timeline on canvas, a mixer of faders and a patch log are not a form library's problem, and §15 already decided the timeline is custom rendering under any framework. Plain CSS. |
| `@vitejs/plugin-react` | Fast Refresh | What it buys is preserving component state across an edit, and ADR 0012 §2 means the frontend *has* no state worth preserving: a reload re-reads `get_song` and rebuilds every view from it. Vite compiles `.tsx` without it; the cost is a full reload per edit, on a local page. Its dependency footprint is the smaller half of the argument and is stated precisely rather than inflated: one direct dependency (`@rolldown/pluginutils`) and four declared peers, of which three — `oxc-transform-react`, `@rolldown/plugin-babel`, `babel-plugin-react-compiler` — are marked optional, so only Vite is actually required. |
| A charting or virtualisation library | Draw the timeline | §15 decided canvas/WebGL, and its rationale — "rendering is custom either way" — is the same one. |

`ponytail:` no bundler was the rung below Vite and it does not hold. React ships CommonJS and
bare specifiers do not resolve in a browser, so `tsc` plus an import map is more configuration
than Vite is, not less. Vite is the one build dependency and it earns the row.

### 4. §17's golden-render pass, and a pin that cannot reach a render

§17 requires "an ADR and a full golden-render pass" of any change to its table, and
`lock.baseline.json`'s own `notes` repeat it. The rule is worth stating why it exists before
saying where it stops: a pin fixes a source, and the bits also depend on what the compiler
emitted and what the CPU chose, so a moved pin is only proven harmless by rendering the goldens
again (ADR 0009 §5).

**No `app.*` package can reach a render.** The frontend is downstream of `core`, talks to the
host in-process (ADR 0012 §1), and never touches `engine`, `RenderPlan` or a WAV; a different
React would have to change `core`'s output to change a golden, and it has no path to. So the
pass is vacuous for this block, and saying so here is the point — a `[MUST]` rule that is
silently skipped is a rule that gets skipped again for something that *did* matter.

§17's rules gain one appended line making that explicit and scoping it: the pass is required of
any pin that can reach a render, and a pin that cannot records in its ADR why not. The gRPC pin
that lands in the same change as this one (ADR 0013 §1) is on the other side of that line and
takes the pass in full.

It also takes it in fact, not only in principle. CI's path gate excludes `docs/` and the root
prose files from both jobs and excludes `lock.baseline.json` from neither, so the pull request
carrying this ADR builds the engine and renders all four goldens on three runners — which is the
pass, run by the rule rather than by someone remembering it.

### 5. The webview is not pinnable, and §17 says so rather than pretending

§17 pins by commit or exact version. The engine the frontend renders in ships with the operating
system and moves under the user. Nothing about the **model** depends on it — every claim §11
makes is about `core`, the compilers and `engine`, none of which is in a webview — but anything
that compares an image does, which is why ADR 0012 §5's golden is a serialisable description of
what a view would draw and not a screenshot. A screenshot golden would be the flakiest artefact
in this repository and would fail on a system update with no pull request to blame.

§17 gains that as a note on the Tauri row rather than as a new row, because a row implies a
version we chose.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Record the lockfile by reference: "`app/package-lock.json` is the pin" | True and useless as a decision log. §17 is read to find out what was chosen; a pointer to eight hundred packages answers a different question. The lockfile stays as the integrity record — this is in addition, not instead. |
| Pin React by a range, e.g. `^19` | What `codemirror: "6.x"` already is, and the reason this ADR exists. |
| Leave `app.react` null until PR 2 installs something | Then the pin is whatever `npm install` resolved on the day, chosen by nobody and reviewed as a lockfile diff. |
| Vendor the frontend dependencies as submodules, like the C++ ones | §17 already decided this: a crates.io or npm release is immutable and its hash is verified on every install, which is at least as strong as a git commit. Vendoring eight hundred packages is a different repository. |
| Skip pinning `codemirror` until M4 | Leaves a range in the file for two milestones, and a range re-resolves silently. |
| Amend §17 to exempt registry packages from the golden pass generally | Too wide: `sha2` is a registry package that hashes assets, and `tonic` is one that carries a plan. The line is whether a pin can reach a render, not how it is distributed. |

## Consequences

- `lock.baseline.json`'s `app` block goes from three entries, two of which were not pins, to
  seven that are — plus a `_note` and a `_reused` block naming the three that gain a consumer
  rather than an entry, and saying which entry has no consumer yet.
- §17's Tauri row names all three Tauri packages and carries the webview note; the table gains a
  frontend row and a gRPC row (ADR 0013 §1); the rules gain one appended line.
- PR 2 writes `app/package.json` against exactly this list and commits `app/package-lock.json`
  with it. A `package.json` in this pull request would be code, and this pull request has none.
- CI gains an `app` job eventually; what it runs is the projection golden (ADR 0012 §5) and
  `tsc --noEmit`. Where it sits in the path gate is PR 2's, since `app/` does not exist yet and
  the gate's lists are exclusions — a new top-level directory runs every job until someone
  decides otherwise, which is the direction that costs runner minutes rather than coverage.
- Nothing here is a §17 pin change that can move a golden, and the golden pass runs anyway
  because `lock.baseline.json` is outside both path gates.
