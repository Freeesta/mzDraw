# arch-web-06: Shortcuts in the settings file: remap, switch off, only on focus

Prompt for one DEV chat of mzDraw (`Freeesta/mzDraw`), proposed by the study `docs/study/arch-web.md` (§7, slice 06).

## Confirm message
Your first message, to COO-MZDRAW (the coordinator; its session id is in your launch message; use `send_message` with that id), one line:
`DEV-SHORTCUTS: received; branch claude/arch-web-06-shortcuts from origin/main; reading CLAUDE.md, docs/mzdraw.md, docs/study/arch-web.md.`

## Role
You are DEV-SHORTCUTS, a development chat. You build one small slice, open one PR ready for review, and report. The coordinator merges with the full head SHA; you never merge.

## Scope
- In: the `shortcuts` section of `mzdraw-settings/1` (key to command, read through the one settings table, unknown keys kept as today), a setting to switch all single-key shortcuts off, single keys active only while the editor has focus; the key table drives the help list; conflicts refused with a key.
- Out: new commands (the missing keys of `docs/mzdraw-ui.md` §6 Q2 are bound only if COO-MZDRAW says Q2 is decided as option a).
- Anything else you notice: one line to COO-MZDRAW, not in this PR.

## Context (read first)
- `CLAUDE.md` (rules 1-14), `docs/mzdraw.md` (the section "Built state" is what exists; do not rebuild it), `docs/study/arch-web.md` (the rows of §4 named in §7 for slice 06, and §5 for the API).
- WCAG 2.2 SC 2.1.4: a single-key shortcut must be switchable off, remappable, or active only on focus; do all three. The ARIA Authoring Practices keyboard page lists combinations that clash with the system, the browser and assistive technology.
- Keys are decided in `Editor::key` (`crates/mzdraw/src/editor_tools.rs`) and `editor_keys.rs`; the settings table is `settings::FIELDS` and `Style::set`; the page forwards every key today.

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
1. Rust: a binding table (default = today's bindings, exactly) consulted by `Editor::key`; `set("shortcuts.<command>=<key>")` and the section in the settings file; `shortcuts.single=off`.
2. The page forwards keys only when the canvas has focus (check today's code; fix it if it listens on the document).
3. Tests: Rust unit tests (remap, off, conflict refused, unknown kept, round trip of the settings file bytes); parity case with a remapped key through the replay request's `set`; e2e: remap `n` to oxygen, switch single keys off, typing in the side panel never draws.

## Tests
- As in Work, with keyboard steps in e2e.
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
