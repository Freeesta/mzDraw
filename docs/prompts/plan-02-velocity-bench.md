# plan-02: drawing-speed benchmark `velocity/1` on a fixed task set

Proposed by `docs/plan/backlog.md` (row 10; Gemini better question 1; backlog section 2). Size M. Wave 1. Depends on: none.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-02 velocity-bench: received, starting on branch `claude/plan-02-velocity-bench`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. This slice writes tests and tools only; it changes no editor code. It answers with numbers where mzDraw loses to other editors on raw drawing speed.

## Scope
In:
- `tests/data/velocity/tasks.json`: 12 tasks, each `{id, goal, target_smiles, objects?}`: benzene; ethanol; isobutanol; ibuprofen; caffeine; anthracene; a steroid skeleton (gonane); (2R,3R)-butane-2,3-diol with wedges; glycine zwitterion; a Boc-protected amine; a two-component reaction with an arrow and a condition text; three structures in a row.
- `tests/data/velocity/mzdraw.json`: for each task, the shortest input path known in mzDraw, as a list of steps (`click x y`, `drag x0 y0 x1 y1`, `key K`, `type TEXT`) in canvas units. Write them by hand with the tools that exist today.
- `tests/velocity.mjs` (Playwright, Chromium): serves `site/`, plays each path, counts input actions (one per press, key or drag), measures wall time to the first moment the canonical SMILES of `save("smiles")` equals the target's (median of 5), and writes `velocity/1` JSON to stdout: `{"format":"velocity/1","tool":"mzdraw","commit","rows":[{"task","actions","ms_median","ok"}]}`.
- Agent speed: for each task, the `draw_edit/1` patch that draws it (hand-written), its op count and bytes, and the native time of `replay` (median of 20), in the same report under `"agent"`.
- `tools/velocity/ketcher.mjs` (outside CI): fetches a pinned Ketcher standalone release (Apache-2.0; write the exact version and URL), plays `tests/data/velocity/ketcher.json` (paths written by hand for Ketcher's own tools) and writes the same JSON with `"tool":"ketcher"`.
- `docs/plan/velocity-protocol.md`: the protocol for a person timing the same tasks in a desktop editor they are licensed for (counts, screen recording, the JSON row with `"measured_by":"person"`). No product is named as the target of fidelity.
- `tests/data/velocity/baseline.json`: the first mzDraw report (actions only; times vary by machine).

Out (do not do here):
- Any editor change that would make a path shorter (that is what the report is for: list the three largest gaps in the PR).
- Committing any Ketcher file.

## CLI twin (rule 15)
- `node tests/velocity.mjs > velocity.json` (exit 0 when every task reaches its target, 1 when one does not, with the task on stderr).
- The agent half runs with no browser: `cargo run --example replay` on each patch.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `tests/e2e.mjs` (how the page is driven, touch and keyboard included), `web/mzdraw.js` (which events reach Rust).
- Ketcher: https://github.com/epam/ketcher (releases, `documentation/help.md` for its tools and keys).

## Agreement needed before the PR
None: this slice adds no `draw_edit/1` operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-02-velocity-bench` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Write the tasks and their target SMILES; check each target round-trips through `save("smiles")` of a document opened from it.
2. Write the mzDraw paths and the patches; play them; fix a path, never the counter.
3. Write the Ketcher runner and paths; run it once and put the result in the PR (not committed).
4. Write the protocol page.

## Tests to add
- `tests/velocity.mjs --check`: fails when an mzDraw task's action count is above `baseline.json` or a task no longer reaches its target (the regression guard), and is added to the acceptance list in `CLAUDE.md` Commands by the coordinator, not by you.
- A unit check that `tasks.json` targets are valid SMILES read by `mzlab-mol` (in `crates/mzdraw/tests/velocity_tasks.rs`).
- Touch: at least two tasks are played with touch events, one with the keyboard only.
- **Fails without the change**: `--check` with a baseline one action lower than measured must fail (test the guard itself with a doctored baseline in a temp dir).

## Independent proof
Correctness must not rest on our own claims. The comparison is against an independent program run by the same harness (pinned Ketcher) and, by hand, against a person's timing in a desktop editor. The targets are checked by canonical SMILES of `mzlab-mol`; a second check of each target with RDKit `Chem.CanonSmiles` sits in `tools/velocity/check_targets.py` (output committed for the 12 targets). Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

