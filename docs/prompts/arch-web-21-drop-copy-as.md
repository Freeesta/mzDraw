# arch-web-21: Drop a file on the canvas; copy as MOL / KET / SMILES / image; PNG and HTML on the clipboard

Prompt for one DEV chat of mzDraw (`Freeesta/mzDraw`), proposed by the study `docs/study/arch-web.md` (§7, slice 21).

## Confirm message
Your first message, to COO-MZDRAW (the coordinator; its session id is in your launch message; use `send_message` with that id), one line:
`DEV-DROP-COPY-AS: received; branch claude/arch-web-21-drop-copy-as from origin/main; reading CLAUDE.md, docs/mzdraw.md, docs/study/arch-web.md.`

## Role
You are DEV-DROP-COPY-AS, a development chat. You build one small slice, open one PR ready for review, and report. The coordinator merges with the full head SHA; you never merge.

## Scope
- In: files dropped on the canvas go through the same door as Open (`add_bytes`, import report shown); keys `Ctrl+Shift+M` (molfile), `Ctrl+Shift+K` (KET), and free keys from the key table of slice 06 for SMILES and the image (named in the PR); copy writes `image/png` (with the embedded document) and `text/html` (the SVG) next to today's formats, one `ClipboardItem` per write, custom web formats only after `ClipboardItem.supports()`.
- Out: native clipboard formats of the desktop app (CDXML, EMF), pasting images of other programs.
- Anything else you notice: one line to COO-MZDRAW, not in this PR.

## Context (read first)
- `CLAUDE.md` (rules 1-14), `docs/mzdraw.md` (the section "Built state" is what exists; do not rebuild it), `docs/study/arch-web.md` (the rows of §4 named in §7 for slice 21, and §5 for the API).
- Ketcher: `Mod+Shift+M/K/F` copy as MOL / KET / image (help 3.21.0). Clipboard API WD 2026-06-24: `text/plain`, `text/html`, `image/png` mandatory; custom `web` formats in Chromium 104 and Firefox 161, not Safari; one item per write in Chrome and Firefox; a user gesture is needed in Firefox and Safari (BCD 8.1.10).
- Today copy writes the molfile as plain text and the document under `application/x-mzdraw+json` (`docs/mzdraw.md` Slice 2a); the PNG is made by the page from the module's SVG (`png_plan`, `png_finish`).

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
1. Rust: the keys in the binding table returning page flags (as `ASK_LABEL` does) that ask the page to copy a given format; the format bytes from `save`.
2. JS: drop handler (`dragover`, `drop`) forwarding bytes; clipboard writes as above; no decision in JS beyond feature detection.
3. Tests: e2e drop of a synthetic molfile and a KET file (DataTransfer in Playwright), copy as each format read back from the clipboard in Chromium; Rust unit tests of the key flags.

## Tests
- As in Work, keyboard and touch steps kept.
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
