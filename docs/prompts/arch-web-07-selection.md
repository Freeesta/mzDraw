# arch-web-07: Lasso, fragment selection, a band without dragging, focus on bonds and objects

Prompt for one DEV chat of mzDraw (`Freeesta/mzDraw`), proposed by the study `docs/study/arch-web.md` (§7, slice 07).

## Confirm message
Your first message, to COO-MZDRAW (the coordinator; its session id is in your launch message; use `send_message` with that id), one line:
`DEV-SELECTION: received; branch claude/arch-web-07-selection from origin/main; reading CLAUDE.md, docs/mzdraw.md, docs/study/arch-web.md.`

## Role
You are DEV-SELECTION, a development chat. You build one small slice, open one PR ready for review, and report. The coordinator merges with the full head SHA; you never merge.

## Scope
- In: lasso selection, a click that selects the connected piece, a band made by two clicks (WCAG 2.5.7), Shift+click adds, the keyboard focus on a bond or an object (Tab never taken).
- Out: moving and rotating by keys and the "Move here" mode (`arch-std-05`), rotate handle, context menu (slice 12).
- Anything else you notice: one line to COO-MZDRAW, not in this PR.

## Context (read first)
- `CLAUDE.md` (rules 1-15), `docs/built/` (the Built state is what exists; do not rebuild it), `docs/study/arch-web.md` (the rows of §4 named in §7 for slice 07, and §5 for the API).
- Ketcher cycles rectangle, lasso, fragment selection with Esc (Ketcher 3.21.0 help); WCAG 2.2 SC 2.5.7 needs a non-drag alternative for every drag (the Understanding page's own example is a selection box by two corner clicks); the new Marvin documents objects cycled from the keyboard.
- The band selection and the atom focus exist (`editor_tools.rs`, `editor_keys.rs`); hit-testing is in Rust.

## Rules
- English only, everywhere (code, comments, texts, commits, PR).
- All logic in Rust (`crates/mzdraw`, bridge `crates/mzdraw-wasm`); JavaScript only forwards pointer, wheel and key events, calls the module and paints the display list. No chemistry and no decision in JS.
- No bundler, no framework, no new crate or npm package unless the PR states the reason.
- Synthetic data only in tests and examples (molecules you write yourself as SMILES or by hand).
- Every new edit operation has an exact inverse and is added to the random sequence of the property test (`random_step` in `crates/mzdraw/tests/ops.rs`: apply then undo gives the identical document bytes, redo the identical result), and to the log reader and writer tests of the same file.
- CLI first (CLAUDE.md rule 15): a feature without a CLI route and a test that replays it natively is incomplete. Every page function has its scriptable twin: a `draw_edit/1` op, a `section.key=value` setting, or a field of the replay request; the native replay (`cargo run -p mzdraw --example replay`) gives the same bytes as the page (`tests/parity.mjs`); add a case to your own file `tests/data/log/<area>.json` and bless it only after a wanted change (`MZDRAW_BLESS=1 cargo test -p mzdraw --test replay`).
- Never skip, disable or loosen a test. Never push to `main`, never rebase, never force-push; merge `main` into the branch only on a real conflict. The PR is opened ready for review and is never merged by you.
- Keep the size cap of `tools/wasm-cap.toml` as it is on `main` when you start; do not edit `measured` (it is gone: the build prints the size) and report the change in bytes in the PR.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, what is built and what is not), and mark the rows of `docs/study/arch-web.md` §4 that you build as `B`.

## Work
1. Rust: tools `select:lasso`, `select:piece`; the two-click band in the select tool (first click on empty space and release without moving arms it, the second click closes it; Esc cancels); bond and object focus with announcements by key.
2. Tests: Rust unit tests for each; e2e with mouse, touch (lasso by finger) and keyboard (two-click band by Enter at two focus points is not needed; keyboard users select with Space on the focus).

## Tests
- As in Work; property test unchanged (no new op).
- Keep the touch, pen and keyboard steps of `tests/e2e.mjs` passing, and add your own steps in the same style (one `step(...)` per behaviour).
- UI change: screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), desktop 1280×720 and tablet 768×1024, light and dark if the theme tokens exist on `main`; attach them to the PR.

## Acceptance (all green, on your branch, before the PR)
```
cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
tools/build_site.sh
cargo build --locked -p mzdraw --example replay && node tests/parity.mjs
node tests/e2e.mjs
```
- The PR body says: what was built, what was not, the module size before and after (bytes), the new ops or settings with one example each, the test counts.

## Messages
- Only to COO-MZDRAW, by `send_message`. A decision you need: one message with the options (two at most) and your recommendation first, then keep working on what does not depend on it.
- When the PR is open: one message with the PR link, the full head SHA, the size change, and the test lines (`cargo test` summary, parity count, e2e steps).

## Handoff at 200k tokens
When your context reaches about 200k tokens: start nothing new; bring the branch to green or revert the unfinished step; commit and push (never force); send COO-MZDRAW one message: branch, last commit SHA, done, not done, the next step, open questions. The coordinator starts a successor with this prompt plus your message.
