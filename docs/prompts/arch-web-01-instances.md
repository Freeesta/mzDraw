# arch-web-01: Editor instances in the wasm bridge

Prompt for one DEV chat of mzDraw (`Freeesta/mzDraw`), proposed by the study `docs/study/arch-web.md` (§7, slice 01).

## Confirm message
Your first message, to COO-MZDRAW (the coordinator; its session id is in your launch message; use `send_message` with that id), one line:
`DEV-INSTANCES: received; branch claude/arch-web-01-instances from origin/main; reading CLAUDE.md, docs/mzdraw.md, docs/study/arch-web.md.`

## Role
You are DEV-INSTANCES, a development chat. You build one small slice, open one PR ready for review, and report. The coordinator merges with the full head SHA; you never merge.

## Scope
- In: several independent editors in one page: the bridge exposes an instance (an exported struct or a numbered table of editors) with the functions that today act on the one global; `free` on disposal; the page uses one instance.
- Out: the custom element (slice 02), any change to `crates/mzdraw` editing behaviour, new features.
- Anything else you notice: one line to COO-MZDRAW, not in this PR.

## Context (read first)
- `CLAUDE.md` (rules 1-15), `docs/built/` (the Built state is what exists; do not rebuild it), `docs/study/arch-web.md` (the rows of §4 named in §7 for slice 01, and §5 for the API).
- `crates/mzdraw-wasm/src/lib.rs` keeps one `Page` in a `thread_local` (`PAGE`); every exported function (`pointer`, `key`, `command`, `clipboard`, `paste`, `zoom`, `pan`, `fit`, `add`, `add_bytes`, `open`, `save`, `png_plan`, `png_finish`, `set_style`, `set_data`, `annotate`, `set`, `style`, `labels`, `scene`, `strings`, `state`, `replay`) acts on it.
- `docs/study/arch-web.md` §4.8 row "several editors on one page" and §5.6: every embedding slice depends on this one. JSME, OpenChemLib, Kekule.js and ChemDoodle all allow several instances per page.
- `replay` and the pure functions (`version`, `name`, `styles`, `settings_schema`, `ann_schema`, `saved_summary`) need no instance and stay free functions.

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
1. Add an exported `Editor` handle (wasm-bindgen struct `MzEditor` or a table keyed by `u32` behind `create()` / `dispose(id)`; choose the one that costs fewer bytes and say the numbers in the PR) with every instance function as a method. The check of the `mzlab` feature runs per instance.
2. Keep the old free functions for one release as thin wrappers on a default instance, so `web/mzdraw.js` can move in the same PR; mark them deprecated in their doc comment. Move `web/mzdraw.js` to the handle.
3. Unit test (Rust, native): two editors edited in turn keep separate documents, histories, selections and styles. e2e: the page still passes every step; add a step that makes a second instance from the console of the test (two documents, no cross-talk).

## Tests
- Native unit tests of the handle; e2e as above (mouse, touch and keyboard steps unchanged).
- Keep the touch, pen and keyboard steps of `tests/e2e.mjs` passing, and add your own steps in the same style (one `step(...)` per behaviour).

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
