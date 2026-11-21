# plan-01: one table of the `draw_edit/1` operations, served as JSON

Proposed by `docs/plan/backlog.md` (row 12; finding F2: the log of mzDraw has 12 operations, the reader of mzLab 7). Size S. Wave 1. Depends on: none (COO 16 agreement on ownership, backlog Q1).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-01 op-table: received, starting on branch `claude/plan-01-op-table`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. This slice adds no edit operation: it makes the language one table that the reader, the writer, the docs and mzLab can all read.

## Scope
In:
- A new module `crates/mzdraw/src/schema.rs`: `OPS: &[OpSpec]`, one row per operation of the log (`atom.add`, `atom.set`, `atom.ann`, `atom.data`, `bond.add`, `bond.set`, `obj.add`, `obj.set`, `move`, `remove`, `rotate`, `flip`, `ring`): name, fields (key, JSON type, required, unit, bounds), the inverse it records, `since` (the `mzdraw/1` version) and `mzlab` (true when the reader of mzLab, `mzlab-feat/src/chem/edit.rs`, accepts it).
- `schema::json()` writes the table as `draw-edit-schema/1` (`{"format","ops":[{"op","fields":[...],"mzlab"}]}`), by hand, keys in a fixed order (no derived serde, no `f64` through `{}`).
- `Op::from_value` checks the op name and the field set against the table (one source of truth; an unknown field is refused with `error.draw2.opField`, as today for unknown ops). The existing readers per op stay; the table is what they are checked against.
- Replay: `"out":"schema"` returns the JSON; `Editor::save("schema")` is the door.
- `docs/mzdraw.md`: a section "Edit language" generated from the table (a test compares the section with `schema::json()` so the doc cannot drift).

Out (do not do here):
- Any new operation or field (each comes with its own slice and COO 16's agreement).
- Changes in mzLab: the report of which ops mzLab lacks goes to the coordinator, who brings it to COO 16.

## CLI twin (rule 15)
- `{"out":"schema"}` through the replay request (the route of `mzlab-cli mol draw`); stdout is the JSON, exit 0.
- A log with an op outside the table exits with the existing error key and code (no new code).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `crates/mzdraw/src/ops.rs` (`Op::from_value`, `Op::write`), `crates/mzdraw/src/replay.rs`.
- mzLab `crates/mzlab-feat/src/chem/edit.rs` and `edit_op.rs` on mzLab `main` (read through `git show`; do not change mzLab): the 7 operations it reads and their field names.
- mzLab `docs/agenti/mol-draw.md` (the language as mzLab documents it).

## Agreement needed before the PR
COO 16 owns `draw_edit/1` (backlog Q1). Before the PR, send the coordinator the table of the 13 ops with the `mzlab` column and the field names that differ, and wait for the answer on who owns the table. Without an answer, open the PR with the table marked "proposal" in its doc section.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-01-op-table` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Write `schema.rs` from the readers in `ops.rs` (read every arm; the table must say what the code accepts, nothing more).
2. Fill the `mzlab` column by reading mzLab's `edit.rs` at the pinned commit `ccd4680` and at `main`; write both commits in a comment.
3. Wire the check into `Op::from_value`, the replay `out` and `save`.
4. Generate the doc section and its test.

## Tests to add
- `crates/mzdraw/tests/schema.rs`: every op the property test of `ops.rs` generates is in the table with exactly its fields; every field in the table is read by the op's reader (a log with only that field set is accepted).
- A log with `{"op":"atom.set","id":1,"bogus":1}` is refused with `error.draw2.opField`.
- Determinism: the same build gives the same schema bytes; parity case `"out":"schema"` in `tests/data/mzdraw_log.json`.
- No e2e change is expected; run it anyway.
- **Fails without the change**: the schema test on a log with an unknown field passes today (unknown fields are ignored or refused per op, not by one rule); it must fail before `schema.rs` and pass after.

## Independent proof
Correctness must not rest on our own claims. The `mzlab` column is checked by a test that reads a list of mzLab's op names copied into `tests/data/mzlab_ops.txt` with the mzLab commit it came from; the coordinator re-checks it against mzLab's own `edit.rs` when the pin moves (rule 9). Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Acceptance (all green before the PR)
```
cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
tools/build_site.sh
cargo build --locked -p mzdraw --example replay && node tests/parity.mjs
node tests/e2e.mjs
```
If a golden output moves because of a WANTED change: `MZDRAW_BLESS=1 cargo test -p mzdraw --test replay`, and say in the PR which lines moved and why.

## Messages
- First message to the coordinator, before any work: the confirm line above, verbatim.
- Ask the coordinator only when blocked by a decision that is not yours (a cap raise, a new op, a scope question); otherwise decide and say why in the PR.
- When the PR is open: one message with the PR link, the full head SHA, the module size before and after (bytes), the test counts, the independent-proof result, and one line per acceptance command (pass/fail).

## Handoff at about 150k tokens
At about 150k tokens of context, stop starting new work: commit what builds, push the WIP branch (no force) with a one-line state note at the top of the PR description or in the last commit message ("state: done X; next Y"), and send the coordinator: branch, last commit SHA, what is done, what is left as a numbered list, the exact next command to run, and any decision taken that the next chat must keep.

