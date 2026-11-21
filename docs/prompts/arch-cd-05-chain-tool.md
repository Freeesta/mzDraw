# arch-cd-05: Chain tool: drag out a zigzag, the carbon count shown while dragging

Study rows: A13 of `docs/study/arch-cd.md`. Priority MUST. Size guess S. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-05 chain-tool: received, working on claude/cd-05-chain-tool`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- tool `chain` (key `X`): press on an atom or the canvas, drag; the zigzag follows the pointer at 120° with one bond length per step; the count is shown next to the pointer; release adds the chain as one undo step
- the first bond leaves at the sprout angle of the start atom; the zigzag side flips with the pointer

Out (do not build here, say so in the PR if you meet it):
- Fischer or freeform chains (CDD has them; LATER)
- chains in rings

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `editor_tools.rs` (press/drag/release of the bond tool, `Overlay::preview`), `build.rs` `sprout_angle`.
- Study facts: MarvinSketch's chain tool sets the count by dragging and follows the mouse (MS/marvin_draw-chains.html); ChemDoodle has linear, Fischer and freeform chains (CDD-G §2.2.4.5).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-05-chain-tool` from `origin/main`.

1. `Tool::Chain`; the preview is a list of segments in `Overlay` (painted lighter, as the bond preview); count = floor(distance along the drag direction / step) clamped to 1-30.
2. Release: `atom.add` + `bond.add` ops in one step (no new op needed); on an existing end atom the chain bonds to it; a chain end that lands on an atom closer than the merge distance of `build.rs` bonds to it.
3. Touch: a finger drag uses the same path; pen as mouse.

Tests to add (synthetic data, small):
- a drag of 4.2 bond lengths gives 4 new carbons; angles 120°; one undo step; from an atom with two neighbours the first bond takes the widest gap
- parity: a chain step in `tests/data/mzdraw_log.json`

UI: a chain button in the bond group; e2e with mouse and touch: drag a chain, read the count from the live line and the atoms from `state()`. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-05-chain-tool`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
