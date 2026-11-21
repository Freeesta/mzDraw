# tp-02: anchoring by hand: the arrow tool, `pathway:anchor`, `pathway:route`

Proposed by `docs/plan/backlog.md` §11 (row 82) and `docs/study/tp-draw.md` (verdict V2, part 2). Size S. Depends on: tp-01; the CTX-MENU page PR (the context-menu items; without it they are model-only and reached from the keyboard command table). Wave: right after tp-01.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-02-anchor-tools: received, starting on branch `claude/tp-02-anchor-tools`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust and the page (paint and forward only). tp-01 gave arrows anchors; this slice lets a person make them without thinking about it, and lets an agent re-route after raw ops.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- **Arrow tool.** An arrow drawn from inside one structure's box to inside another's is anchored at once and routed, in the same undo step. `from` is the smallest atom id of the start structure and `to` the smallest of the end structure.
- **`pathway:anchor`.** On a selection of one arrow, this command anchors each end to the nearest structure: nearest by box distance, within 1 bond length, ties by the smallest atom id. An end with no structure in reach stays free and is reported.
- **`pathway:route`.** Re-routes every anchored arrow in one undo step; it emits no op when nothing moves.
- **Context menu** (`context.rs`): on an arrow, "Anchor to structures" (`pathway.anchor`) and "Free ends" (`pathway.free`). On the canvas, "Route arrows" (`pathway.route`).
- **Keyboard.** The two commands get entries in arch-std-06's command table; if it has not merged, they get no keys yet and are reached through the context menu, which is keyboard-operable.
- **Live line.** Says in words what was anchored ("arrow anchored: C7H8O → C7H8O2").

Out (do not do here):
- Layout (tp-05), labels (tp-07).

## CLI twin (rule 15)
- `{"commands":[{"command":"pathway:anchor","args":{"arrow":30}}]}`, `{"commands":[{"command":"pathway:route"}]}`; with `"dry_run":true` (plan-20) the ops are reported and not applied.
- Request rows (plan-01 `request` section, D13): the command names `pathway:anchor`, `pathway:route` (the rows of `command` values).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state"), `docs/plan/backlog.md` (sections 0, 4, 5, 9), `docs/study/tp-draw.md`.
- tp-01 (`route`, follow hook, `commands`), `editor_tools.rs` (arrow tool), `context.rs`, `editor_keys.rs`.

## Agreement needed before the PR
No `draw_edit/1` op or field (tp-01 added the fields). Request rows only (D13).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-02-anchor-tools` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Arrow tool anchoring; the two commands; the context-menu items.
2. Live-line text (Rust); i18n keys.
3. Goldens: a parent and two TPs, arrows drawn with the tool, the parent moved by keyboard, the arrows following.

## Tests to add
- The tool: an arrow started inside structure A and ended inside B has `from`/`to` set to the smallest ids. An arrow ended on empty canvas is free.
- `pathway:anchor` tie-break and reach. `pathway:route` is idempotent (a second run gives no op). Undo of each returns the identical document.
- e2e (Chromium): draw two structures and an arrow between them with the mouse, then with touch, then with a pen; move one structure with `Alt+Arrow`; the SVG equals the golden. Anchor a free arrow from the context menu by keyboard.
- Screenshots at deviceScaleFactor 2 in the PR: before and after an anchored arrow follows a move.
- **Fails without the change**: an arrow drawn between two structures has no `from` on `main`.

## Independent proof
Correctness must not rest on our own claims. The velocity task of tp-01's kill test, counted before and after (plan-02), and parity wasm == native on the new log cases. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Kill test
Report tp-01's kill test here (action counts before and after). Only one of plan-02's 12 velocity tasks has an arrow (the two-component reaction), so add three synthetic arrow tasks to `tests/data/velocity/tasks.json` in this slice: tp-01's "parent with three TPs", a two-step chain, and a star of four. If the tool anchors a wrong structure in any of these four tasks, change the rule to "anchor only on release inside a box" (not "nearest") and say so.

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

