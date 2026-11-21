# tp-11: the step field on the page (type a step, pick a site, derive)

Proposed by `docs/plan/backlog.md` §11 (row 91) and `docs/study/tp-draw.md` (verdict V5, part 2). Size S. Depends on: tp-10, and tp-10's kill test passed; the CTX-MENU page PR (the item that opens the field by touch and pen). Wave: after tp-10.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-11-step-field: received, starting on branch `claude/tp-11-step-field`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. tp-10 gave the steps and their sites in Rust and on the CLI. This slice gives the person a fast field for them, on the page. The page only paints and forwards: the list, the filter, the sites and the preview are decided in Rust.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- **Opening the field.** On a focused or selected structure, key `Shift+T` opens the step field (`.` is the radical key, `docs/mzdraw.md` keyboard map; `Shift+T` is unbound today and goes in arch-std-06's command table). The context menu of a structure gets the item "Apply step…" (`context.rs` id `tp.step`), which opens the same field: that is the way in for touch and pen. The field is a one-line input listing tp-10's table, filtered as you type. The filter matches the name and the delta (`+O`), and the mass shift too when arch-web-17 is there (`+15.99`). The filter is `fn filter(text) -> Vec<step>` in Rust.
- **Sites of the chosen step** (atoms, or bonds for bond steps): they are ringed on the canvas (overlay), and the focused site also shows a ghost of the added atoms. The overlay is not in the document and makes no op until `Enter`.
- **Keys, only while the step field is open:**
  - `[` / `]` move to the previous / next site;
  - `Enter` derives (`tp:derive` with `step` and `site`; one undo step);
  - `Escape` closes the field.
  
  Outside the field, bare `[` and `]` stay unbound, as they are today; `Alt+[` / `Alt+]` keep rotating the selection (`editor_tools.rs`, arch-std-05), field open or not. The scoping is decided in Rust (the editor's key handler knows the field is open), not in JS. `Tab` is **not** used: the canvas never traps it (`docs/built/keyboard-outline.md`).
- **Touch and pen**: tap a ringed site, then tap "Derive".
- **Live line**: it says the step, the delta, the site ("hydroxylation, +O, at C7, site 2 of 5").

Out (do not do here):
- Any change to the table or the site tests (tp-10 is the contract).

## CLI twin (rule 15)
- Unchanged from tp-10. The page's log of a step derive replayed natively gives the page's bytes (parity case). The field and overlay are page-only views of `out:sites`.
- Request rows: none new.

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state: keyboard editing, drag-free editing), `docs/plan/backlog.md` (sections 0, 4, 5, 9), `docs/study/tp-draw.md`.
- tp-10, `editor_keys.rs`, `editor_tools.rs` (`[` `]` today), the overlay of `scene.rs`, `web/mzdraw.js`.

## Agreement needed before the PR
None: no op, field or request row.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-11-step-field` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. `filter`, field state in the editor, scoped keys.
2. Overlay: rings and ghost.
3. Page field (paint and forward); live-line text.

## Tests to add
- Unit: with the field closed, bare `[`/`]` do nothing and `Alt+[`/`Alt+]` rotate (the existing tests stay green). With it open, bare `[`/`]` move between sites.
- `filter("+O")` lists hydroxylation and n-oxidation, in table order.
- e2e:
  - keyboard: `Shift+T`, type `+O`, `]` twice, `Enter`;
  - touch: long press the structure, "Apply step…", tap the second ring, tap "Derive";
  - pen: the same;
  the SVG equals the golden each time.
- Screenshots at deviceScaleFactor 2 in the PR: the field open with the rings and the ghost.
- **Fails without the change**: `Shift+T` does nothing on `main`.

## Independent proof
Correctness must not rest on our own claims. The velocity task of tp-03's kill test (plan-02), done with the field against `tp:derive` from the context menu plus editing: the action counts are committed. Parity wasm == native on the step-derive log cases. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Kill test
If the field does not cut the velocity task's actions by at least 30% against derive-then-edit, remove the key `Shift+T` and keep the field behind the context menu only (say so in the PR).

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

