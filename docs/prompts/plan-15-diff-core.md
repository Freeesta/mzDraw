# plan-15: semantic diff of two documents as a `draw_edit/1` patch

Proposed by `docs/plan/backlog.md` (row 13; Gemini bold idea 3 (agent co-drawing), part 1). Size M. Wave 1. Depends on: plan-01 (can start in parallel; rebase nothing, add ops to its table when it merges).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-15 diff-core: received, starting on branch `claude/plan-15-diff-core`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only, no page: the base of the proposal layer (plan-16) and of `--diff` for agents.

## Scope
In:
- A new module `crates/mzdraw/src/diff.rs`: `fn diff(a: &Document, b: &Document) -> Vec<Op>` matching things by id (the ids are stable across edits), producing removes, adds and sets in a fixed order (removes of bonds, of atoms, of objects; adds of atoms, bonds, objects; sets; moves grouped by equal displacement), using only ops of the plan-01 table.
- `fn hunks(ops) -> Vec<Hunk>`: groups the patch into hunks a person can accept one by one: the ops touching one connected piece of the drawing, or one object; each hunk is a self-contained patch (applying any subset of hunks in order is valid, by test).
- Replay: `"diff":{"a":DOC,"b":DOC}` returns `{"ops","hunks"}`; `Editor::diff_with(doc)`.

Out (do not do here):
- Showing the diff (plan-16).
- Matching by structure when ids differ (a later question: two drawings of different origin).

## CLI twin (rule 15)
- Replay request with `"diff"`; stdout the patch; exit 0, or 2 when a document is invalid.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `doc.rs`, `ops.rs` (every op and its fields), `crates/mzdraw/tests/ops.rs` (the random document generator: reuse it).

## Agreement needed before the PR
None: this slice adds no `draw_edit/1` operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-15-diff-core` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Diff by id; deterministic order.
2. Hunks by connectivity.
3. Doc section.

## Tests to add
- Property: for random a and random edits giving b, `apply(diff(a,b), a) == b` (identical bytes), and `diff(a,a)` is empty.
- Property: applying any subset of hunks in order succeeds and gives a valid document.
- Determinism and parity case.
- **Fails without the change**: the property test cannot compile before `diff.rs`; once it does, a diff that emits sets before adds fails it.

## Independent proof
Correctness must not rest on our own claims. The property test is the proof (exact bytes of `mzdraw/1`); and the canonical SMILES of `apply(diff(a,b),a)` equals that of b as written by `mzlab-mol`. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

