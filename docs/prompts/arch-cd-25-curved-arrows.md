# arch-cd-25: Electron-pushing curved arrows anchored to atoms, bonds and lone pairs

Study rows: G05, B11 of `docs/study/arch-cd.md`. Priority SHOULD. Size guess M. Depends on: arch-web-13 (arrow kinds) if merged.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-25 curved-arrows: received, working on claude/cd-25-curved-arrows`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- lone pairs on atoms (`atom.set lp`: count, drawn as dot pairs at free sides; `Place lone pairs` fills the count from the valence)
- a curve object (`obj.add t:"curve"`) whose start and end are anchors (an atom, a bond, a lone pair of an atom, or a point), with a head `full` (pair) or `half` (one electron) and a bend; the curve follows its anchors when they move; an anchor removed turns the end into a point
- drawn in the SVG and written to CDXML and MRV-compatible structures only where the public docs define it; else listed

Out (do not build here, say so in the PR if you meet it):
- "show the result" of the arrows (CDD has it; LATER, with mzLab `mechanism/1`)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `doc.rs` `Obj` (points only today), `scene.rs` (arrow painting), `ops.rs` `Remove` (what happens to objects bound to removed atoms).
- Study facts: MarvinSketch snaps ends to radicals, lone pairs, charges and bonds and limits arrows per source (MS/marvin_electron-flow-arrows.html); Marvin JS removes an arrow when its source or target changes (MJ/marvin-js_reaction-mechanism.html); ChemDoodle lays pushers out automatically (CDD-G §10.13).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); rule 15 (CLI-native, agent-first): the feature is first a Rust command with a machine-readable form — `draw_edit/1` JSON ops, or a setting `--set <section>.<key>=<value>` of `mzlab-cli mol draw` — with stable JSON keys and error keys, deterministic bytes and no browser in the path; the page is one client of it, and a feature without a CLI route and a native replay test is incomplete; Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-25-curved-arrows` from `origin/main`.

1. Anchors as ids with a kind; geometry computed in the scene every frame (so a moved atom moves the curve without an op); the remove op stores the anchors it detaches, for an exact undo.
2. Tool `curve` with pair/half choice: press on a source, release on a target; the bend is chosen on the free side and adjustable by dragging the middle handle (`obj.set bend`).

Tests to add (synthetic data, small):
- property test with lone pairs and curves; moving an anchored atom changes the SVG of the curve and nothing in the document; removing the atom and undoing restores the anchor

UI: lone-pair and curved-arrow tools; e2e: draw a pair arrow from a lone pair to a carbon with the mouse and with touch, move the atom, save SVG; screenshot. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

Docs: keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice: what it does, the op/setting names, what is not built); update the row(s) of `docs/study/arch-cd.md` to `built` in the same PR.

CLI route and native replay (rule 15): lone pairs and curves are `draw_edit/1` ops (`atom.set lp`, `obj.add t:"curve"`, `obj.set bend`), replayed natively; anchors are ids, so a replayed move of an anchored atom moves the curve in the native SVG too (parity entry with a move after the curve).

Size: the owner lifted the wasm size limit (the cap in `tools/wasm-cap.toml` is now only a runaway guard); rule 8 still holds: measure with `tools/build_site.sh`, write the new size in `measured` and the delta in the PR.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-25-curved-arrows`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
