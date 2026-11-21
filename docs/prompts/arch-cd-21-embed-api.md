# arch-cd-21: Embedding API: the editor in another page with a small, stable interface

Study rows: L04 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-21 embed-api: received, working on claude/cd-21-embed-api`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- a custom element `<mz-draw>` (plain JS, no framework) that loads the module and exposes: `load(text)`, `save(kind)`, `smiles()`, `molfile()`, `svg()`, `onchange` event with the log of the step, `readonly` attribute, `tools` attribute (a preset name: `full`, `one-molecule`, `reaction`)
- every method maps to an existing Rust export; the element holds no chemistry
- a demo page `web/embed.html` and a test that embeds two editors on one page without interference

Out (do not build here, say so in the PR if you meet it):
- the mzLab page itself (its own repository)
- iframes, workers, frameworks

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw-wasm/src/lib.rs` (the module keeps one editor in a thread-local: two editors on one page need an editor handle; add an id parameter or an `EditorHandle` class through wasm-bindgen — measure both and choose)
- `docs/mzdraw.md` (`TPDraw` API wanted by mzLab: `load`, `smiles`, `partial`, `mark`, `highlight`).
- Study facts: ChemDraw JS attaches to a DOM node (CD-JSX); Marvin JS is an iframe reached through a promise (MJ/marvin-js_how-to-embed.html); ChemDoodle Web's sketcher takes options such as `oneMolecule`, `includeToolbar`, `useServices` (CDW/tutorial/2d-structure-canvases/sketcher-canvas).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-21-embed-api` from `origin/main`.

1. Several editors per module (handle); the existing page uses handle 0 and keeps working.
2. Tool presets in Rust (which palettes and tools exist), used by prompt 18's model when merged.
3. A short API doc in `docs/` (methods, events, attributes, stability promise: semver of the bridge `version()`).

Tests to add (synthetic data, small):
- two editors on one page: drawing in one leaves the other's document unchanged; `readonly` refuses edits but allows selection and copy; `onchange` delivers the log JSON

UI: the demo page; e2e on `embed.html` with mouse, keyboard and touch; screenshot of two embedded editors. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-21-embed-api`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
