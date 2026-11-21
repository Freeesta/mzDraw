# arch-cd-19: Palettes, part 2: tear-off floating palettes and docking by drag

Study rows: P02, P03 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: prompt 18.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-19 palette-tear-off: received, working on claude/cd-19-palette-tear-off`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- a sub-palette dragged by its title bar becomes a floating palette above the canvas, with a title and a close box; it can be moved and closed; closed = hidden in the model
- a toolbar dragged to a window edge docks there (left, right, top, bottom) with a landing preview; a double-click on a title returns it to its home place
- all positions in the `ui` model (clamped, remembered, reset by prompt 18's command)

Out (do not build here, say so in the PR if you meet it):
- second monitor (P13, desktop later)
- named layouts (P09)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- prompt 18's `ui::Layout`; study §4 (ChemDraw tears sub-palettes off by their title and docks toolbars on edges, CD-UG17; MarvinSketch previews the landing place with a coloured border, MS-IOD/customizing-marvinsketch-gui.html; ChemDoodle Web floating trays flip near the edges, CDW/tutorial/advanced/sketcher-application-window).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-19-palette-tear-off` from `origin/main`.

1. Rust: `ui_drag(phase, id, x, y)` decides float vs dock (edge zones of N px), returns the landing preview and the final model; JS sends pointer events and paints what Rust says.
2. Floating palettes stay inside the window and do not cover the canvas centre on first tear-off; on narrow screens (`ui.preset=compact`) tear-off is off and groups open as a bottom bar.
3. Pointer: mouse, pen and touch (one finger drags a title; two fingers still pan the canvas).

Tests to add (synthetic data, small):
- drag sequences in Rust: tear off, move, dock left, double-click home; the model after each; clamping when the window shrinks

UI: floating palettes and docking; e2e: tear off the ring group, move it, reload, it is where it was; dock the bonds toolbar right; double-click returns it; the same with touch; screenshots of a floating palette and of the landing preview. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-19-palette-tear-off`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
