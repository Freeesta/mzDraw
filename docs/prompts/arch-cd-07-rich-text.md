# arch-cd-07: Rich text: several lines, sub- and superscript runs, bold, italic, formula style

Study rows: H02, H10 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-07 rich-text: received, working on claude/cd-07-rich-text`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- text runs on text objects and on the text above/below arrows: `{"t":"H"},{"t":"2","sub":true},{"t":"O"}`, flags `sub`, `sup`, `b`, `i`; line breaks
- a formula mode: digits after an element symbol are lowered when typed (`H2O`, `CO2`); `^` and `_` in the text field for sup and sub
- a symbol picker for a short fixed list (°, ±, ‡, δ, →, ⇌, ×, µ, Greek letters)
- text metrics from the fixed width table in Rust (so wasm and native give the same bytes); SVG `tspan` runs; KET rich text and CDXML style runs out and in where the schema/SDK defines them

Out (do not build here, say so in the PR if you meet it):
- fonts other than the style's
- tables (H09)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `doc.rs` `Obj.text` (one line now), `scene.rs` text layout and the width table, `svg.rs`, `ket.rs` (reads rich-text content and joins it), `cdxml.rs` (style runs joined).
- `docs/mzdraw-formats.md` §5 already draws `runs` for text objects.
- Study facts: ChemDraw has a formula style for captions (CD-UG17); ChemDoodle sets sub/superscript size and shift in its text tool (CDD-G §10.11); the new Marvin has rich text with a formula tool (M/marvin_sketch-js-comparison.html).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-07-rich-text` from `origin/main`.

1. Add `runs` (optional; a plain `text` stays valid and is the single-run form, so old files read the same) with exact inverse through `obj.set runs`. Write the op by hand like the others: `Op::write` (writer) and `Op::read` / `Op::from_value` (reader) in `crates/mzdraw/src/ops.rs`, defaults left out; the reader refuses a malformed op with its error key and changes nothing.
2. Layout of runs in Rust: line height, baseline shift and scale of sub/sup from the style's `subscript_scale`; multiple lines left-aligned.
3. Text field: the page sends the typed string; Rust turns `_`, `^` and formula mode into runs (the decision is in Rust).
4. Readers: keep runs from KET and CDXML instead of joining them; the outline (`outline.rs`) reads the plain text of runs.

Tests to add (synthetic data, small):
- property test with runs; a text without runs is byte-identical to before; H2O in formula mode gives three runs with the 2 lowered
- SVG golden of a two-line caption; KET and CDXML round trip keep the runs

UI: the text field gains formula mode and the symbol picker; e2e: type `H2SO4, 25 ^oC` with the keyboard, check the runs in `save("mzdraw")` and the SVG; edit the text on touch. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

Docs: keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice: what it does, the op/setting names, what is not built); update the row(s) of `docs/study/arch-cd.md` to `built` in the same PR.

Size: the owner lifted the wasm size limit for now, but rule 8 still holds: measure with `tools/build_site.sh`, write the new size in `measured` of `tools/wasm-cap.toml` and the delta in the PR. If the module passes `cap`, send COO-MZDRAW the numbers and raise `cap` only when it answers, with the reason in the PR.

## Acceptance

All of these pass, and their output is quoted in the PR:
```
cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
tools/build_site.sh
cargo build --locked -p mzdraw --example replay && node tests/parity.mjs
node tests/e2e.mjs
```

If a golden output changes because of this slice (and only then), rewrite it with `MZDRAW_BLESS=1 cargo test -p mzdraw --test replay` and explain each changed file in the PR. Extend `tests/data/mzdraw_log.json` with the new op(s) so that the parity test replays them.

## Messages

- To COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) only. Start: the confirm line above. Blocked (a test you cannot make pass, a decision that is not yours, a host refused): one message with the exact error, then wait or continue with the rest.
- Done: one message with the PR link, the full head SHA, the module size before → after, and the study rows now built.
- The PR is in English, ready for review (not a draft), titled after the feature, its body following the repository's usual sections, with the screenshots for UI changes.

## Handoff at 200k tokens

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-07-rich-text`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
