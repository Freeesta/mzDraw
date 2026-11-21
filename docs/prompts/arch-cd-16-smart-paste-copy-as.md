# arch-cd-16: Smart paste placement and Copy as SMILES / molfile / CDXML / KET

Study rows: J04, J06 of `docs/study/arch-cd.md`. Priority MUST. Size guess S. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-16 smart-paste-copy-as: received, working on claude/cd-16-smart-paste-copy-as`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- a paste (own clipboard or system text) lands at the centre of the view when that place is free, else at the nearest free place to its right and below; never on top of existing atoms; the view does not rezoom
- commands `copy:smiles`, `copy:molfile`, `copy:molfile-v3000`, `copy:cdxml`, `copy:ket`, `copy:svg` write the selection (or all) in that format as text to the system clipboard

Out (do not build here, say so in the PR if you meet it):
- desktop native clipboard formats (J07, prompt 22 follow-up)
- InChI (decision 5)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw/src/editor_edit.rs` (`paste`, `duplicate`, `clipboard`), `web/mzdraw.js` clipboard handling (the page only moves bytes), `Editor::save` kinds.
- Study facts: ChemDraw 23.0 pastes at the view centre or the nearest free place top-left without overlap (CD-RN23); MarvinSketch copies as SMILES and MRV with Ctrl+L / Ctrl+M (MS/marvin_shortcuts-in-marvinsketch.html).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-16-smart-paste-copy-as` from `origin/main`.

1. Placement in Rust: candidate offsets on a spiral of one bond length around the view centre, first box with no atom within 0.5 bond length; a `move` op after the paste ops in the same step.
2. `Editor::clipboard(kind)` grows the kinds; the selection is written alone (a sub-document).
3. Keys: `Ctrl+Shift+C` opens a small "copy as" chooser (page), each kind one button.

Tests to add (synthetic data, small):
- pasting benzene twice gives two non-overlapping rings; a paste into an empty view lands at its centre; every copy kind of the selection of two structures holds only the selected one

UI: the copy-as chooser; e2e: paste a SMILES twice, check no overlap; copy as CDXML and read the system clipboard in Chromium. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-16-smart-paste-copy-as`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
