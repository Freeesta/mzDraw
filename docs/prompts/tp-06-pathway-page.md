# tp-06: pathway layout on the page, and the Graphviz proof

Proposed by `docs/plan/backlog.md` §11 (row 86) and `docs/study/tp-draw.md` (verdict V2, part 4). Size S. Depends on: tp-05, tp-02. Wave: right after tp-05.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-06-pathway-page: received, starting on branch `claude/tp-06-pathway-page`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. tp-05 built `pathway:tree` in Rust. This slice gives it to the person on the page, through the mouse, touch, pen and keyboard. It also adds the check by an independent program.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- **Button.** "Arrange pathway" in the objects palette (UI-2 R1 layout table; paint and forward). It is also on the canvas context menu (`context.rs` item `pathway.tree`) and has an entry in arch-std-06's command table.
- **Settings.** `pathway.dir`, `pathway.order` and `pathway.col_gap` in the settings panel. The fields come from `settings::schema_json()`; the page only paints them.
- **Live line.** It says what the command did, in words decided in Rust: "Pathway arranged: 7 structures, 6 arrows, 1 gap widened". A refusal shows its key's text (`error.draw2.schemeCycle`, ...).
- **Graphviz proof.** `tools/fidelity/pathway.py`.

Out (do not do here):
- Any change to the layout rules (tp-05 is the contract).

## CLI twin (rule 15)
- Unchanged from tp-05: `{"commands":[{"command":"pathway:tree"}]}`. The page's log replayed natively gives the page's bytes (parity case).
- Request rows: none new.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state"), `docs/plan/backlog.md` (sections 0, 4, 5, 9), `docs/study/tp-draw.md`, tp-05 `pathway.rs`.
- `palettes.rs`, `context.rs`, `web/mzdraw.js` (forwarding), `tests/e2e.mjs`.

## Agreement needed before the PR
None: no op, field or request row.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-06-pathway-page` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Palette entry, context-menu item, command-table entry, settings fields.
2. Live-line text and keys.
3. `tools/fidelity/pathway.py` and its committed reference.

## Tests to add
- e2e (Chromium): the messy star of tp-05, arranged
  - by mouse;
  - by touch on a reloaded page;
  - with a pen;
  - from the keyboard (command entry);
  the SVG equals tp-05's golden each time. Then a cycle drawn by hand shows the refusal text, and nothing moves.
- Screenshots at deviceScaleFactor 2 in the PR: the messy star before and after, and the binary tree with `dir=down`.
- Parity: the page's log of "Arrange pathway" equals the native replay bytes.
- **Fails without the change**: the button does not exist on `main` (the e2e locator fails).

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/pathway.py` gives the same graphs to Graphviz `dot -Grankdir=LR -Tjson`, fetched or installed at run time:
- **Ranks.** It reads each node's `pos` from the JSON. Its rank is the index of the node's x among the distinct x values, sorted, after rounding to 1 pt.
- **Crossings.** It counts crossings itself, as pairwise proper intersections of the straight segments between node positions. Dot's splines are not used.
- **Pass conditions.**
  - For every tree, the ranks equal our layers, and our primary-edge crossing count (0) is ≤ dot's.
  - For the DAG, the rank differences are listed, not failed: dot minimises the total edge length, and we use the longest path.
- **Without Graphviz.** If `dot` is absent, the script prints `skipped: dot not installed`, exits 0, and the PR says so. The Rust properties of tp-05 stay the contract.

Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Kill test
Show the owner the before/after screenshots of three pathways (star, unbalanced tree, convergent DAG). If the owner judges any of them worse than their ChemDraw habit, list what is wrong as properties to add to tp-05 (a follow-up). Ship the button anyway, since it is undoable, and say so.

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

