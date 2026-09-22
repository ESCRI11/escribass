# ADR 0021 — Provenance is core's, and names the model, the prompt and the call

- **Status:** Accepted (2026-09-21)
- **Affects:** `core/src/project.rs` (`prepare`), `core/src/version.rs`'s sibling walk,
  `core/src/session.rs` (the proposal's provenance); `app/src-tauri` (the conversation and its
  file); `docs/specs.md` §5, §9, §10, §15 and §17; `lock.baseline.json`
- **Builds on:** ADR 0006 §4 (core overwrites §4.3's fields on the way in; a caller cannot set
  them — extended here to the raw pipeline); ADR 0005 §1 (a bump inside `prepare` lands inside
  the recorded diff) and §3 (the version guard is a comparison, and why a filter on paths could
  not be); ADR 0012 §4 (a dry run already returns real ids, and they are the keys); ADR 0019 §1
  and §4 (the proposal, and what its entry says); ADR 0010 §2 (a pin written on first
  reference, never rewritten by a tool); ADR 0001 §2 (the log is the audit trail)
- **Recorded in:** `docs/specs.md` §5, §9, §10, §15 and §17.

## Context

§4.3 puts `provenance` on every entity — `author`, and optional `model_id`, `prompt_id`,
`tool_call_id` — and `history.proto` on every entry. §5 calls the log the audit trail and §9's
history view shows the author column. The fields have existed since M0.1 and **nothing has ever
set the three optional ones**: `core` writes `None` for all three at every site that builds a
`Provenance` (`tools.rs`, `project.rs`, `session.rs`). `Author` is per session —
`Session::new(.., author)`, `--author` on both binaries, `Author::Human` hardcoded in the host —
which is right for every process that exists today and is `docs/plan.md`'s question 3 the
moment a model's calls reach the window's session. Question 4 asks what a `prompt_id` names
and where the conversation lives. U3 decided that a hosted model is recorded, not pinned.

The spike found a defect on this exact surface, reproduced independently from the main thread
(plan, "Found beside the six questions"). **`apply_patch` stores caller-written entity ids and
`provenance` verbatim.** A session run as `--author model` applied a section whose provenance
claimed `AUTHOR_HUMAN`, created in 1999; `get_song` returned it exactly so, while the log entry
that wrote it says `AUTHOR_MODEL` at the real time. ADR 0006 §4's "a caller cannot set them"
holds for the typed tools, which mint §4.3's fields and preserve an existing note's
(`tools.rs`, `minted_notes`), and not for the raw pipeline, which `prepare` guards for
`version` (ADR 0005 §3) and for nothing else. So a model can stamp an entity human, in the
product whose claim is that every edit is attributed — and `apply_patch` is a tool the model
is offered (ADR 0022 §1) and the one §9 hands a person for Edit.

The plan's determinism section says why this is not cosmetic: a log entry that says
`AUTHOR_MODEL` with no `model_id` records that *something* wrote it; one that names the model
and the prompt is an audit trail a reader can act on. The song is reproducible from its log even
when its author is not, and provenance is what makes that useful.

## Decisions

### 1. `provenance` is creation provenance, maintained by core and never writable — on every path

`prepare` gains a second walk beside `bump_versions`, over the same shape (`is_entity`: an
object with a string `id` and a numeric `version`), and it decides every entity's `provenance`
before the document is re-deserialised, so the result lands inside the recorded diff (ADR 0005
§1): **an entity that did not exist before gets the call's provenance**, author and ids and
`created_at` from the injected clock; **an entity that existed keeps the provenance it had**,
whatever arrived in its place, because an edit does not change who created a thing — the
entry's provenance records the edit's author. `prepare_merge` leaves provenance alone, as it
leaves the version dispute alone (ADR 0005 §3): a merge's new entities were minted by the other
branch's author and an undo's re-added entity by whoever added it, and those are core's own
values arriving from the log.

It is an **overwrite and not a refusal**, and the reason is the one ADR 0005 §3 met for
`version`. A guard that compared what the caller sent against what core would write would
refuse every previewed patch under a real clock: the dry run minted `created_at` when it ran,
the apply runs later, and §9's approve-then-apply — and ADR 0019 §2's Apply — send the dry
run's patch back verbatim. A comparison that ignored `created_at` would still refuse Edit,
where a person applies a patch whose new entities carry the *model's* provenance from the
proposal and should get the person's. There is nothing a caller can legitimately be
*disputing* about provenance, because it was never the caller's to state; so it is not a
`Violation`, it is core doing what ADR 0006 §4 said it does, on the one path that had not.

**Ids stay the caller's.** A dry run mints real ids from a fork and "those are the keys"
(ADR 0012 §4); applying the previewed patch is how they survive, so `apply_patch` must accept
the ids it is handed. An id is not an authorship claim, and the validator's `id_not_ulid` and
`key_id_mismatch` are its guard. The spike's "a caller can mint entity ids" is therefore left as
it is, and said: a caller that writes its own ULIDs into a patch gets them, exactly as a caller
re-sending a preview does.

The **six sites** trap 3 counted are unchanged: the session's `author` stays the default for
every call that arrives with no other word — the binaries' flag, the window's `Human` — and no
request message gains an author field, because a field on the wire is a field a caller can
lie in. The three ids reach the log through **one new site**, the proposal (decision 2), which
is the only place a model's calls enter under ADR 0020 §1.

### 2. The three ids: what each is, who sets it, and where it lands

- **`model_id`** is the model id **the provider's response named** — OpenRouter's `model` on
  the response that produced the call or the reply — and never the id that was asked for.
  U3's "what actually answered", per response, because a router may answer one turn from
  another provider under the same name and the entry should say what wrote it. It says nothing
  of who ran the weights; the serving provider is recorded in the conversation (decision 3),
  not the log.
- **`prompt_id`** is the **SHA-256 of the prompt's text**, UTF-8, lowercase hex — computed by
  the host with the one hasher the project store already has (`core::asset_hash`, §10), so a
  prompt is content-addressed as an asset is: stable, replayable by the scripted provider
  (ADR 0022 §4), and verifiable by anyone holding the text. The same text asked twice has the
  same id and two `created_at`s, which is what content addressing means.
- **`tool_call_id`** is the id **the provider assigned the call** (`tool_calls[].id`), which is
  the string that ties an entity to a line of the conversation.

They travel with the **proposal** (ADR 0019 §1): it is created with `AUTHOR_MODEL` and the
`prompt_id`; each call carries the `model_id` of the response that made it and its own
`tool_call_id`, so an entity minted in the proposal is stamped with the call that minted it;
and the entry the proposal commits carries `AUTHOR_MODEL`, the `model_id` of the turn's last
response, the `prompt_id`, and **no** `tool_call_id`, since a composed proposal has several and
the entities carry theirs. Edit (ADR 0019 §3) commits as the person — `AUTHOR_HUMAN`, no
`model_id`, the `prompt_id` kept.

An MCP client's model is anonymous to this server: `escribass-mcp --author model` keeps writing
`AUTHOR_MODEL` and no `model_id`, because nothing on the MCP wire says which model is on the
other side and a flag would be a claim the server cannot check. "What M3 will not claim" says so.

### 3. The conversation is the host's, persisted beside the project and outside it

The conversation is not song state (CLAUDE.md #1 is about the song), so it goes in neither
`song.json` nor the log. It is held by the host for the window's life and sent whole with each
prompt (ADR 0020 §3), so `ai` holds nothing between streams; and it is **persisted by the
host**, outside the `.escri`, as one append-only JSON Lines file per project under the
application data directory Tauri already names, keyed by the song's own id (`Song.id`, which
survives a rename or a move of the directory). Each line is a turn: `prompt_id`, the prompt,
when, `model_id` and the provider that served, the calls with their ids, names, arguments and
whether each was refused, the reply, and the outcome — applied as which entry, rejected,
edited into which entry, or failed how. The panel reads it back when the project is opened.

Outside the project, for the plan's two reasons: a prompt may contain text a person would not
commit, and `patches/` is committed; and a fifth artefact in §10 would be a thing branches and
merges know nothing about. Persisted at all, because the log's `prompt_id`s have to name
something on the machine that made them, and the calls a proposal was composed from live
nowhere else (ADR 0019 §4).

**The honest sentence.** A log can name prompts a machine no longer has: the project travels
with `git` and the conversation does not, and a person may delete it. The `prompt_id` then
records that a prompt with that hash existed and what it changed, and nothing more; the
history view shows the id and says the conversation is not on this machine.

### 4. `ai.model` is a record, not a pin; it is written on first use and never by a tool

§17's "model ids pinned per project in `lock.json` under `ai.model`" cannot mean what "pinned"
means in every other row: a hosted model changes under its name, no commit or hash verifies
one, and the plan's determinism section says why no test should pretend otherwise. U3 decided
the reading and §17 is amended to carry it. `lock.json`'s `ai` block records the **choice** — the
provider and the model id a person chose for this project; every entry's `provenance.model_id`
records what **actually answered** (decision 2); and the default, when a project records none,
is `lock.baseline.json`'s — `deepseek/deepseek-v4.1-flash`, chosen by the user and measured
before it was recorded (plan, "DeepSeek V4.1 Flash, measured before it became the default").

It is written the way a plugin pin is (ADR 0010 §2): on **first use** — the first prompt sent in
a project writes the model it was sent to, if the block is absent — never rewritten by a tool,
and changed by editing the text (§2.6) as a re-pin is until a tool exists. `core` reads it at
open and hands it to the loop; the `Lock` struct gains the block in the pull request that first
reads it (PR 8), and until then a project's `lock.json` carries no `ai` block.

Two rules follow. **The status bar never counts it as verified** (plan, trap 15): a model id
verifies nothing, and a readout that included it is a check that cannot fail dressed as one
that passed. And **the model is never told it is deterministic**: what is reproducible is the
song from its log, and the panel's wording keeps the distinction *live preview · not the
render* already keeps for sound.

## Alternatives considered

**The forgery**

| Alternative | Rejected because |
|---|---|
| Refuse a caller-written provenance by comparison, as `version_not_writable` does | `created_at` makes every previewed patch differ from what core would write, so the approve-then-apply flow — §9's and ADR 0019 §2's — would have no working path, which is the failure ADR 0005 §3 was revised to avoid |
| Refuse, ignoring `created_at` | Still refuses Edit, where a person's patch carries the proposal's `AUTHOR_MODEL` on new entities and should get `AUTHOR_HUMAN` |
| Strip `provenance` from the JSON schema and trust the model not to send it | The schema already omits it (ADR 0006 §4) and the spike sent it anyway, copied from a `get_song`; a rule enforced by the absence of a field is not a rule |
| Overwrite ids too | A dry run's ids are the keys a pending edit is held by (ADR 0012 §4), and re-sending the previewed patch is the one way they survive an apply |
| Withhold `apply_patch` from the model instead | Closes the model's route to a mix, a deletion, a rename and a clip's bounds, which no typed tool offers (ADR 0022 §1) — and leaves the hole open for Edit and for every MCP client |

**The ids**

| Alternative | Rejected because |
|---|---|
| Author on every request message | An additive field on twenty-odd requests, on the wire for every carrier, and a caller that lies about being human. The proposal is the one place a model's calls enter, and it is not on the wire |
| A second session on the project, with the model's author | Refused by ADR 0004's write ordering and by ADR 0012 §3 |
| `model_id` as the id that was asked for | U3's word is "what actually answered"; a router may answer under another id, and the log should not record the request as the fact |
| `prompt_id` as a ULID minted per prompt | Not replayable: the scripted provider's golden needs the same prompt to name the same id in two runs, and a hash does that with no id source |
| `prompt_id` as a hash of the whole conversation to that point | Unique per position, and unverifiable without the whole conversation; content addressing the text is what `assets/` already does and what a reader holding the text can check |

**The conversation**

| Alternative | Rejected because |
|---|---|
| Not persisted; the panel is per window | Every `prompt_id` in the log dangles the moment the window closes, on the machine that made it, by design |
| Persisted inside the `.escri`, a fifth artefact | A §10 change and an ADR, a thing branches and merges know nothing about, and text a person would not commit in a directory that is committed |
| The prompt's text in the entry | A `history.proto` change (CLAUDE.md #5) for text a person would not commit, in the log that is |
| `ai` holds the conversation between streams | State in the sidecar across turns, so a restarted sidecar forgets mid-conversation and a test needs a live one to be stateful against; the host holds it and sends it, so `ai` is a function of what it is sent |

**The model's record**

| Alternative | Rejected because |
|---|---|
| Treat `ai.model` as a pin and count it in the status bar | Nothing can verify it; trap 15 |
| A `set_model` tool | A tool writing `lock.json` outside the patch log, which is the objection the re-pin row already records (ADR 0010 §3); editing the text is enough until a person asks otherwise |
| Record the serving provider in the log's provenance | `song.proto` has no field for it, and the value is a fact about one request's routing, not about the entity; the conversation records it |

## Consequences

- **PR 4** (`m3.4-provenance`) is the silent pull request: the provenance walk in `prepare`,
  its exemption under `prepare_merge`, and the spike's reproduction as a test watched failing
  first — a section applied under `--author model` claiming `AUTHOR_HUMAN` in 1999 comes back
  `AUTHOR_MODEL` at the clock's time. **No determinism golden is expected to move**: no
  `apply_patch` in the five scripts adds an entity, and an existing entity keeps its provenance
  under the new walk as it did under the old absence of one. Any byte that does move is named
  in that pull request. The lock's fix lands beside it (ADR 0020 §5), with a loud test.
- **PR 8** (`m3.8-loop`): the proposal carries the three ids into every entity it mints and
  the entry it commits; the `Lock` struct gains `ai`, written on first use; the loop reads the
  project's model id and hands it to `ai` per prompt.
- **PR 9** (`m3.9-panel`): the conversation file, written after each turn and read on open; the
  history view shows `model_id` beside the author column and says when a `prompt_id` names a
  conversation this machine does not have; nothing in the status bar counts `ai.model`.
- **`app` keeps `Author::Human`.** The plan's PR 4 row said the host "stops hardcoding
  `Author::Human` where a model can call"; under ADR 0020 §1 the model's calls never enter
  through the window's own author, so the hardcode is correct and stays. The row is amended.
- ADR 0006 §4 is extended in place: core overwrites `provenance` on the raw pipeline too, and
  leaves ids to the validator.
- `docs/specs.md`: §5 gains the sentence beside `version`'s; §9 says what the history view
  shows for a model's entry; §10 says the conversation is not part of the project; §17's LLM
  provider row reads "recorded, not pinned", and `lock.baseline.json`'s `ai.model` names the
  default with the note that says what the name is.
- "What M3 will not claim": not that an MCP client's entries name their model, and not that a
  `prompt_id` resolves on a machine other than the one that made it.
