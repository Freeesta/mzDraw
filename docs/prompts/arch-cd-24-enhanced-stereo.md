# arch-cd-24: Enhanced stereo groups (ABS, AND, OR): assign, show, V3000 in and out

Study rows: C04 of `docs/study/arch-cd.md`. Priority SHOULD. Size guess M. Depends on: prompt 02 (labels in the scene) for the painting.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-24 enhanced-stereo: received, working on claude/cd-24-enhanced-stereo`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- a structure field of stereo groups: each wedged centre in at most one group `abs`, `and<n>`, `or<n>`; op `stereo.group {atoms, k, n}` with exact inverse
- labels `abs`, `&1`, `or1` next to the centres (setting `display.stereo_groups`)
- molfile V3000 `MDLV30/STEABS|STERAC|STEREL` collections read (today listed as lost) and written; CXSMILES tail out if `mzlab-mol` writes it

Out (do not build here, say so in the PR if you meet it):
- groups on double bonds and sulfoxides (BD has them, LATER)
- naming with groups (F08 OUT)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `io.rs` (V3000 read and the note that groups are not kept), `mzlab-mol` V3000 reader/writer (`molfile_v3000.rs`, `cxsmiles.rs` in the pinned rev): find what it keeps of the collections.
- Study facts: MarvinSketch assigns groups from the Atom menu or by typing `abs`, `or1`, `and1` (MS/marvin_stereochemistry-in-marvinsketch.html); BIOVIA Draw covers centres, axial stereo and double bonds (BD-DS24, BD-T65); ChemDoodle treats centres as absolute by default and sets or/and from the right-click menu (CDD-G §16.5).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-24-enhanced-stereo` from `origin/main`.

1. Document: `stereo` list on the first atom of the structure (as `data` and `ann` live there) or on the atoms — choose, keep it optional and versionless (unknown-field rules).
2. Op and inverse; the label tool accepts `abs`, `or1`, `and1` typed over a centre (as MarvinSketch).
3. V3000 in/out through `mzlab-mol` if it carries the collections; if not, send COO-MZDRAW the gap for an mzLab slice and implement the drawing part here only.

Tests to add (synthetic data, small):
- property test; a synthetic racemic `and1` pair round-trips through V3000; a centre removed leaves its group consistent

UI: group picker in the atom popover (prompt 09) or the context menu (prompt 20); e2e: assign `or1` to two centres, save V3000, reopen; keyboard path. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-24-enhanced-stereo`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
