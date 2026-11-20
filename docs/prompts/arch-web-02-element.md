# arch-web-02: The custom element `<mzdraw-editor>`

Prompt for one DEV chat of mzDraw (`Freeesta/mzDraw`), proposed by the study `docs/study/arch-web.md` (§7, slice 02).

## Confirm message
Your first message, to COO-MZDRAW (the coordinator; its session id is in your launch message; use `send_message` with that id), one line:
`DEV-ELEMENT: received; branch claude/arch-web-02-element from origin/main; reading CLAUDE.md, docs/mzdraw.md, docs/study/arch-web.md.`

## Role
You are DEV-ELEMENT, a development chat. You build one small slice, open one PR ready for review, and report. The coordinator merges with the full head SHA; you never merge.

## Scope
- In: an autonomous custom element in `web/mzdraw-editor.js` (plain ES module, no bundler): shadow root with the canvas, `ready` promise with calls queued before it, `load`, `save`, `apply`, `undo`, `redo`, `clear`, `document`, `selection`, `select`, `set`, `settings`, `command`, `focusAtom`, `MzdrawEditor.info()`; events `ready`, `change` (with the `draw_edit/1` ops of the step and `origin`), `input`, `selectionchange`, `toolchange`, `importreport`, `error`; attributes `tools`, `style-preset`, `module`; lazy import of the module on first connect; the page `web/index.html` becomes the element plus its side panel; a test that the element makes no request beyond its own files.
- Out: host adapter for files and storage (slice 03), theme and i18n hook (04), `readonly` and forms (05), iframe protocol (later).
- Anything else you notice: one line to COO-MZDRAW, not in this PR.

## Context (read first)
- `CLAUDE.md` (rules 1-14), `docs/mzdraw.md` (the section "Built state" is what exists; do not rebuild it), `docs/study/arch-web.md` (the rows of §4 named in §7 for slice 02, and §5 for the API).
- Read `docs/study/arch-web.md` §5 (the API proposal) and §1 (how the other editors are embedded: every one starts asynchronously and makes the host race; only Ketcher and Kekule report what changed).
- Slice 01 (instances) must be on `main`; if it is not, tell COO-MZDRAW and stop.
- The ops of a step are what the session records (`Session` in `crates/mzdraw/src/ops.rs`, `log_text`); the change event needs them from Rust per step, including the inverse ops applied by an undo: add one bridge call that returns the ops of the last step as `draw_edit/1` JSON (Rust writes it; JS passes the text through).
- W3C Web Platform Design Principles (TAG Note 2026-11-02): attributes mirrored as properties, Promises with an options dictionary and `AbortSignal`, events in the present tense, bubbling and composed, state on the element; customised built-ins are not in Safari, so the element is autonomous (BCD 8.1.10).

## Rules
- English only, everywhere (code, comments, texts, commits, PR).
- All logic in Rust (`crates/mzdraw`, bridge `crates/mzdraw-wasm`); JavaScript only forwards pointer, wheel and key events, calls the module and paints the display list. No chemistry and no decision in JS.
- No bundler, no framework, no new crate or npm package unless the PR states the reason.
- Synthetic data only in tests and examples (molecules you write yourself as SMILES or by hand).
- Every new edit operation has an exact inverse and is added to the random sequence of the property test (`random_step` in `crates/mzdraw/tests/ops.rs`: apply then undo gives the identical document bytes, redo the identical result), and to the log reader and writer tests of the same file.
- Every page function has its scriptable twin: a `draw_edit/1` op, a `section.key=value` setting, or a field of the replay request; the native replay (`cargo run -p mzdraw --example replay`) gives the same bytes as the page (`tests/parity.mjs`); add a case to `tests/data/mzdraw_log.json` and bless it only after a wanted change (`MZDRAW_BLESS=1 cargo test -p mzdraw --test replay`).
- Never skip, disable or loosen a test. Never push to `main`, never rebase, never force-push; merge `main` into the branch only on a real conflict. The PR is opened ready for review and is never merged by you.
- Keep the size cap of `tools/wasm-cap.toml` as it is on `main` when you start; update `measured` with the size you measure and report the change in bytes in the PR.
- Keep the section "Built state" of `docs/mzdraw.md` true (one bullet for this slice, what is built and what is not), and mark the rows of `docs/study/arch-web.md` §4 that you build as `B`.

## Work
1. Rust: `last_step()` on the instance (ops of the last committed step and its origin `edit|undo|redo|load|api`); `apply(json)` that runs a `draw_edit/1` array as one undo step, refused as a whole with an i18n key (the reader `Op::from_value` exists); `info()` (version, formats in and out from `formats.rs`, op names, settings schema).
2. JS: the element forwards events exactly as the page does today (move the code, do not duplicate it), paints the display list, and turns the flags returned by Rust into events. `tools` filters the toolbar buttons; an empty value hides the toolbar.
3. The page uses the element; every e2e step keeps passing on it. Add e2e steps: two elements on one test page; `change` carries ops that, replayed into the second element with `apply`, give the same `document`; `ready` queued calls; a refused `apply` fires `error` with its key; no request beyond the page's own files (Playwright request log).

## Tests
- Rust unit tests for `last_step`, `apply` (one undo step, refused as a whole), `info`; parity case for `apply` through the replay request; e2e as above with mouse, touch and keyboard.
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
