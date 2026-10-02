# ADR 0027 — `lock.json` v3: a `toolchains` block, written on the first compile, checked at compile and never at open

- **Status:** Accepted (2026-09-25)
- **Affects:** `core/src/project.rs` (`Lock`, M4 PR 5); `core/src/session.rs`
  (`toolchain_mismatch`, M4 PR 5); `tests/determinism/generators/` (M4 PR 5);
  `compilers/generative/pyproject.toml` (its version, M4 PR 4); `docs/specs.md` §10, §11, §16
  and §17; ADR 0003 §3 and ADR 0010 §1, amended in place by ADR 0023 §1
- **Builds on:** ADR 0010 §1 (`lock.json` v2's shape: no paths, no parameter lists, not a
  proto), §2 (a pin is written on first reference, never removed, by no tool) and §3 (a
  mismatch is `lock_mismatch`, an operator error, and re-pinning is explicit); ADR 0021 §4 (the
  `ai` block: absent until first use, and absent from the file when absent, which is what kept
  the goldens still); ADR 0008 §5 and ADR 0010 §4 (a fact about a binary is stated by the
  binary, not held in `core` as a second copy); ADR 0023 §6 (the ledger row, split); ADR 0024
  §6 (the toolchain is a pin, not a hash input); ADR 0026 §1 (the child states its version on
  every answer)
- **Recorded in:** `docs/specs.md` §10, §11, §16, §17 and §15.

## Context

ADR 0003 §3 said `lock.json` is "created at M0, completed at M4"; ADR 0010 §1 built v2 —
the engine's commits and one entry per referenced plugin — and left "M4 adds the compiled
artefacts ADR 0003 §3 named"; ADR 0021 §4 added an `ai` block that is a record and not a pin.
§11's third bullet requires "all external tool versions recorded in `lock.json` and checked at
load". The plan's question 10 asks what the generators half adds, and its two corollaries under
"What 'deterministic' means" say the two things that decide it: **a project renders without any
compiler present**, and no compiler pin is checked at open.

Under ADR 0023 §1's split, what M4 has to record is one toolchain — the DSL and the interpreter
it runs on — and what M5 records is the toolchain that made each artefact. This ADR decides the
first and reserves the names of the second.

## Decisions

### 1. A `toolchains` block, written on the first compile from what the child reported, never rewritten by a tool

```json
{
  "schema_version": 1,
  "engine": { … },
  "plugins": { … },
  "toolchains": {
    "generator": {
      "dsl": "1",
      "python": "3.12.12"
    }
  }
}
```

**Written on the first `compile_generator` that commits**, from the `dsl_version` and
`python_version` the child stated in its answer (ADR 0026 §1) — never from a constant in
`core`, for ADR 0010 §4's reason: a fact about the child is stated by the child, and a second
copy in Rust is the copy that stops agreeing. **Absent until then, and absent from the file
when absent**, the `ai` block's shape (`skip_serializing_if`), which is what keeps every M0.4
golden's `lock.json` byte-identical through a milestone that adds a block: no determinism
script compiles, so none gains one, and M4 PR 5's `generators` script is the first whose golden
carries it. **Never rewritten by a tool and never removed** (ADR 0010 §2): re-pinning is
editing the text, as it is for a plugin.

**"That commits" is narrower than it reads, noted 2026-10-01 in M4 PR 5 and accepted
2026-10-01 in M4 PR 6.** A compile made *inside a proposal* writes nothing: its ops stay on
the fork and reach the log only as the one patch a person applies (ADR 0019 §1). So such a
compile writes `Generator.toolchain_version`, because that is in the patch, and does **not**
write this block — the next compile on the session does. The two cannot then contradict each
other, because decision 2 compares the generator's own version as well as the block, and a
project with no block still has that comparison; the only cost is that a project whose
compiles have all been a model's is unpinned until a person compiles once.

PR 5 named it as unreachable and left the decision to PR 6, where `OFFERED` gains
`compile_generator` (ADR 0026 §3) and the path opens. **The user decided it on 2026-10-01:
accept it, and make the acceptance visible.** The reasoning is the block's own: it is a *pin*,
and this ADR's whole shape is that a pin is written from a fact the child stated, once, on a
compile somebody kept. A proposal keeps nothing until a person applies, and what a person
applies is one patch under the tool name `proposal` — so a block written from a fork would be
a pin that no entry in the log explains, which is the defect ADR 0010 §2 is about arriving
through a different door. Writing it at apply instead would mean carrying the child's answer
back out of a fork that by construction records nothing, for a value the next compile writes
anyway and whose absence refuses nothing. The alternative with the best claim — pin at apply —
is therefore refused for the reason the whole file is organised around: **one fact, one
writer, one site.**

What a reader should expect, said plainly because `lock.json` is a file people open: **after a
model-driven compile, the generator carries `toolchain_version` and the project carries no
`toolchains` block.** That is correct and not a gap. It is held still by
`tests/determinism/compile/expected/lock.json`, whose keys are `schema_version`, `engine`,
`plugins` and `ai` and nothing else, beside a `song.json` in the same directory whose
generator reads `"toolchain_version": "1"`; and by `core/tests/generator.rs`,
`a_models_compile_writes_the_generators_version_and_the_block_waits_for_a_commit`, which then
compiles once on the session and watches the block appear.

**`schema_version` stays 1.** It is the *document's* schema version, which ADR 0025 §1 keeps at
1; "v2" and "v3" are names for the file's shape in prose, as ADR 0010 §1's "v2" was, and
nothing reads them. `Lock` keeps `deny_unknown_fields`, so a `core` from before M4 opening a
project that a later one compiled in refuses the file rather than ignoring a block it does not
know — the strict direction, ADR 0010 §3's, and the same thing `schema_version_mismatch` does
one field over.

**The DSL's version is this repository's own number**, and this decision says what it means
because nothing else does. It is the `version` of `compilers/generative/pyproject.toml`, starting
at **`1`**, and it is **bumped by an ADR when the meaning of any construct or name in the
language changes** — when a source that compiled before would compile to different notes — and
not when a name is added, since an addition moves no golden (ADR 0024 §3). **Nor when the
language refuses *more*, which is stated here because M4 PR 8 refused five more things and the
question was asked out loud** (the user's decision, 2026-10-02): a source that compiled before
and still compiles compiles to the same notes, and a source that stops compiling was never a
function of its own text — `f"{rng}"` gave two velocities from one source. So `dsl_version` stayed
`1` through the third lock, and this sentence is here so that a reader knows it was decided and
not overlooked. The asymmetry is the same one `buf breaking` has: removing is the breaking
direction, and refusing is not adding. The interpreter's
version beside it is §17's Python row, `3.12.12`, and the sandbox runs on that pin exactly as
the sidecar does: the spike measured the DSL's arithmetic stable across five CPython minors,
which is evidence that a patch bump would move nothing, and the pin is kept anyway because a
golden is compared by byte and evidence is not a claim.

**`Generator.toolchain_version` is written by the same compile from the same answer.** The field
has existed since M0.1 with §4.4's non-empty rule and no meaning — the fixture says `0.4.1`, a
made-up string (trap 11). From M4 PR 5 it holds the `dsl` the generator was last compiled under,
written by `compile_generator` beside `compiled_hash` and never by `define_generator`, which has
no such field (ADR 0026 §2). The block is the project's pin and the field is the generator's;
both are written from one answer, compared at one site (decision 2), and cannot disagree after a
compile. The fixture's generator therefore becomes **uncompilable by design** — `0.4.1` is not
`1` — and `tests/AGENTS.md` says so: a constructed value, not a project.

**One validator rule is narrowed with it.** §4.4 requires "every `Generator` has a non-null
`seed` and `toolchain_version`", and ADR 0002 §9 left `toolchain_version != ""` as the one
check the schema could not make structural. A generator `define_generator` has just added has
been compiled by nothing and has no version to state, so the rule reads, from M4 PR 5: **a
generator whose `compiled_hash` is non-empty carries a non-empty `toolchain_version`**, and a
never-compiled one may carry it empty — `toolchain_version_empty` fires on the pair, not on the
field. The alternative, a placeholder written at definition, is a string that lies until the
first compile overwrites it, which is the shape this whole decision exists to refuse. §4.4 is
amended to say so, and ADR 0002 §9 carries a dated line.

### 2. Compared at compile — `toolchain_mismatch`, an operator error — and never at open

Before a compile's result is written, `core` compares the child's stated `dsl` and `python`
against the `toolchains.generator` block if the project has one, and the child's `dsl` against
the generator's `toolchain_version` if the generator has been compiled before. **Any
disagreement is `toolchain_mismatch`**, returned as `ProjectError`, on the operator side of ADR
0006 §2's line, and **nothing is written**: not the notes, not the hash, not the block. The
fixes are all operator actions — `uv sync --locked` in `compilers/generative/`, install the
pinned interpreter, or edit the text to re-pin deliberately — and a model retrying a tool call
can produce none of them, so it ends the turn at the host (ADR 0022 §3; ADR 0024 §7). This is
ADR 0010 §3's `lock_mismatch` one block over, and its lenient alternatives are the same table
with the same wrong answers: compiling anyway under a different toolchain produces notes that
differ from the golden with no error anywhere.

It is **also trap 5's guard**: a `uv sync` that was not run, or an environment from before the
change, is a child stating a version the project did not record, and the compile is refused
rather than the golden passing on a stale sandbox.

**Never at open.** `Project::open` reads the block and compares nothing in it; `lock_mismatch`
at open stays what ADR 0010 §3 made it, about the engine and the plugins. A project with a
generator renders with **no compiler installed at all**, because the compiled notes are layer 2
and the render never asks for the compiler (§4.2: "the project renders without re-running the
compiler"; plan, "What 'deterministic' means", first corollary). §11's "checked at load" is
therefore **amended** to say what it means for a toolchain: recorded at compile, checked at
compile, and not a condition of opening. A toolchain is compared when a compile is asked for,
because that is when it matters and the only time it can be wrong.

### 3. `artefacts` and `models` are M5's; their names and one rule are reserved, nothing else

M5 records, for each exported artefact and each converted model, the toolchain that made it —
`cmaj`'s commit, the CLAP SDK, clap-wrapper, the VST3 SDK, the compiler and its **optimisation
level** (ADR 0023 §2) — so that "what a render was made with travels with the render" (ADR
0010). This ADR reserves two top-level keys, **`artefacts`** and **`models`**, and one rule:
**each is a map keyed by the artefact's content hash**, so an entry names one thing and the
thing names its entry, written on first reference and never removed (ADR 0010 §2). Their fields
are M5's schema ADR's to write against the export record it designs (ADR 0025 §2), and are not
sketched here: a shape reserved against no producer is ADR 0002 §7's mistake in a lock file.
ADR 0003 §3's "completed at M4" reads "completed at M5" (ADR 0023 §1).

### 4. `lock.baseline.json` gains nothing

The DSL is not an external dependency: it is this repository's own package, versioned by a
number this repository sets, and `lock.baseline.json` lists what is pinned *from outside*. Its
interpreter is already the `ai.python` row, and §17's Python row gains a sentence saying the
sandbox runs on the same pin rather than a second row saying the same version twice. The plan's
PR 1 row promised a `toolchains.generator` row here — "the DSL's own version, no external pin" —
and this decision declines it, because a version written in a JSON file beside a version in a
`pyproject.toml` is two copies of one number with a test to keep them equal, for a value that
already reaches `lock.json` from the one place that owns it. **Nothing is pinned by this ADR**,
and the file is byte-identical to `main`'s (ADR 0023 §4 says the same of M5's set).

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Check the toolchain at open, as §11 literally says | Refuses to open a project that renders fine without any compiler, on a machine that may have none — the engine block's reasoning in ADR 0010 §3, and it moves every M0.4 golden's `lock.json` |
| A constant `DSL_VERSION` in `core`, mirrored from `pyproject.toml` with a test | A second copy of a fact about the child, held by the thing that is not the child (ADR 0010 §4); the child already states it on every answer |
| `define_generator` writes `toolchain_version` | It would have to know a version only the child knows, or spawn one to ask; the first compile writes it, as a plugin pin is written on first reference |
| Put `toolchain_version` in `compiled_hash` instead of comparing it | Says twice what one comparison says once, and stales every generator on an event the person must act on anyway (ADR 0024 §6) |
| Warn on mismatch and compile anyway | Notes that differ from the golden with no error; "checked" becomes "mentioned" (ADR 0010 §3's table) |
| Sketch `artefacts` and `models`' fields now | A shape against no producer, which M5's export record would then have to fit or reopen |
| A `toolchains.generator` row in `lock.baseline.json` | Two copies of one number with a test between them, for a value the child already owns |
| Bump `lock.json`'s `schema_version` to 3 | It is the document's version, not the file's shape's, and nothing reads a file version |

## Consequences

- **M4 PR 5** (`m4.5-compile-tools`): `Lock` gains `toolchains`, absent until first compile;
  `compile_generator` writes it and `Generator.toolchain_version` from the child's answer;
  `toolchain_mismatch` before anything is written; the `generators` determinism script and its
  golden, which is the first `lock.json` in `tests/` with the block. **No existing golden
  moves**, and any byte that does is named. **Done 2026-10-01**, and none did: the block is
  `skip_serializing_if` as the `ai` block is, so the five determinism goldens' `lock.json`
  files are byte-identical to `main`'s. §4.4's rule is narrowed in `core/src/validate.rs` and
  the narrowing is load-bearing rather than tidy — without it `define_generator` could not add
  a generator at all, since one it has just added carries both fields empty.
- **M4 PR 6** (`m4.6-model`): `compile_generator` joins `OFFERED`, which makes decision 1's
  proposal case reachable. **Accepted on the user's decision of 2026-10-01**, written into
  decision 1 above with its reasoning and goldened in `tests/determinism/compile/`. No code in
  this file's area changed: the behaviour was already what decision 1 describes, and what PR 6
  added is the test that would fail if it stopped being.
- **M4 PR 4** (`m4.4-generator`): `compilers/generative/pyproject.toml`'s `version` is `1`, and
  the child reports it and `sys.version_info` on every `CompileResponse`.
- §10's description of `lock.json` gains the block; §11's third bullet gains the sentence that a
  toolchain is checked at compile and not at load; §16's "M4 adds the compiled artefacts" reads
  the split; §17's intro and Python row are amended.
- `tests/AGENTS.md` records that the schema fixture's generator is uncompilable by design
  (M4 PR 5).
- The ledger's `lock.json` row closes its M4 half here and keeps its M5 half.
- **What this rests on that is unmeasured**: that CPython 3.12.12 under `uv` is what the child
  reports as `python_version` on every machine the pin is installed on — a `uv`-managed
  interpreter should make it so, and the `generators` golden is the check, since its `lock.json`
  carries the string. **The check exists from 2026-10-01** and has passed on one machine; ~~it
  becomes a measurement the first time CI runs it~~ — **and it has been one since run
  `36867215121` (PR #88, M4 PR 5), corrected 2026-10-02: a hosted `ubuntu-latest` runner's
  `uv`-managed 3.12.12 reported the same string the golden carries, and every `--features
  generators` run since has repeated it.** It is a claim about a second *machine*, and a claim
  about a second platform only when there is one (ADR 0009 §1). The phrasing is the lesson:
  "becomes a measurement the first time CI runs it" is a sentence that stops being true without
  anybody editing it, which is how this repository's records go stale (docs/plan.md, "M4,
  mid-flight").
