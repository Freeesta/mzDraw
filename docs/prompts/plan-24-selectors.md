# plan-24: chemical selectors that resolve to ids before the log

Proposed by `docs/plan/backlog.md` §9 (row 73; second Gemini input, idea 1 "ChemSel", reduced). Size M. Wave 4. Depends on: plan-20, plan-23, and the first plan-19 run (see the start condition).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-24-selectors: received, starting on branch `claude/plan-24-selectors`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only. Read-only: a selector is resolved to concrete ids and the agent puts those ids in its ops, so the log, the inverses and the parity do not change.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- **Start condition**: start only if the first `--arm report+digest` run of plan-19 shows addressing errors (wrong atom, wrong piece) in at least 3 of the 10 surgical-edit tasks; otherwise tell the coordinator and stop (the row is dropped).
- A new module `crates/mzdraw/src/select.rs`; request field `"select":{...}` with `"out":"select"` writes `select/1` (`{"format":"select/1","count","matches":[{"atoms","bonds"}]}`). The selector is a JSON object (coordinator, 2026-11-21: no mini-language), every key optional, all keys must hold:
  - `smiles`: a substructure, matched with `mzlab-mol`'s `SubQuery` / `matches` (SMILES only; SMARTS and query features are out, rule 13);
  - `el`: an element; `piece`: the 1-based piece in left-to-right order; `side`: `reactant` or `product` (pieces left or right of the first reaction arrow);
  - `order`: `leftmost`, `rightmost`, `topmost`, `bottommost` (by match centroid; ties by smallest id) to pick one match;
  - `count`: the number of matches wanted (named `count`, not `expect`, which is plan-20's request-level field).
- No match, or not `count` matches, is a verdict (plan-20's rule): exit 1 with `draw-result/1`, `failed.stage` `select`, key `error.draw2.selectNone` or `error.draw2.selectCount`, and the matches in `failed.candidates`. A malformed selector (unknown key, bad SMILES) is an invalid request: exit 2 with `error/1`.
- Add arm `select` to plan-19's runner.

Out (do not do here):
- Selectors inside ops (a `select` field on every op): DEFER; it would change every op of `draw_edit/1` (COO 16) and is only worth it if this slice passes its kill test.
- A CIP predicate (after arch-web-16, through the glue).
- SMARTS, recursive or spatial combinators.

## CLI twin (rule 15)
- `{"import":...,"select":{"smiles":"c1ccccc1Cl","side":"product","count":1},"out":"select"}`; stdout `select/1`, exit 0; no match or a count mismatch is exit 1 with `draw-result/1` (plan-20's rule).
- Native replay test; no page change.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md`, `docs/plan/backlog.md` §9.
- `mzlab-mol` `substruct.rs`, `query.rs` at the pin; how `io.rs` builds the `Mol` of a piece and maps record indices back to document ids.

## Agreement needed before the PR
No `draw_edit/1` op. One row each for the request field `select` and the output `select` in the `request` section of plan-01's table (D13).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-24-selectors` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Check the start condition.
2. Index ↔ id map, matching, filters, order.
3. Errors, runner arm, doc section.

## Tests to add
- Para-chloro on the product of a two-piece reaction gives exactly the expected ids; `count:1` on a symmetric match is exit 1 with both candidates in `failed.candidates`.
- Matches are invariant under renumbering: the same drawing built in another op order gives the same atoms (by element and position).
- Property: every returned id exists and its element matches the query atom.
- **Fails without the change**: the request field is refused before the slice.

## Independent proof
Correctness must not rest on our own claims. For the plan-03 set, the number of matches of 5 fixed SMILES queries equals RDKit's `GetSubstructMatches` count (uniquify on) in the tool. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
Run plan-19 with arm `report+digest+select` against `report+digest`. If first-try exact-edit accuracy on the 10 surgical-edit tasks gains less than 20 points, or selector errors exceed 10% of calls, drop the runner arm and mark the output experimental in `docs/mzdraw.md` (say so in `docs/agent-suite/`).

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
