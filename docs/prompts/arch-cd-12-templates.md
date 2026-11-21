# arch-cd-12: Template library: groups of ready fragments placed like rings

Study rows: A14, A17, A12 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-12 templates: received, working on claude/cd-12-templates`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- a built-in library as data in Rust (`crates/mzdraw/src/templates.rs` or a data file compiled in): groups rings, bicyclics, heterocycles, amino acids (L, as SMILES with stereo), sugars (pyranoses as chairs and Haworth-free 2D), protecting groups, cyclohexane chairs; each entry is a SMILES with an attachment atom, laid out once by the 2D layout
- placement: on the canvas, on an atom (attachment) or fused on a bond (like `ring`), with auto-merge of atoms within the merge distance; `Space` held while placing suppresses the merge
- a palette group (sub-palettes come with prompt 18; until then a list)

Out (do not build here, say so in the PR if you meet it):
- user templates (prompt 13)
- glassware or any artwork (OUT)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `build.rs` `ring` (placement on an atom or a bond, `MERGE`), `ops.rs` `Ring` (one op that expands to atoms and bonds), `layout.rs` (layout of a SMILES).
- Write every entry yourself from chemistry knowledge (public-domain structures as SMILES); copy no template file of any product.
- Study facts: BIOVIA Draw ships protecting groups and named-reaction templates (BD-DS24, BD-T104); MarvinSketch template groups (MS/marvin_templates-in-marvinsketch.html); ChemDoodle templates follow ring placement rules (CDD-G §4.12) and merge overlapping atoms, Space suppresses (CDD-G §2.3.1).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-12-templates` from `origin/main`.

1. Op `template {name, atom?, bond?, x, y, merge}` that expands (like `ring`) into atoms and bonds; the log keeps the expanded ops or the template op — choose the one whose replay cannot change when the library changes (the expanded ops), and say why in the PR.
2. Library: about 40 entries to start, each with a test that its SMILES parses and lays out without overlap.
3. Command `template:<name>`; the page lists groups and names from `templates()` (JSON from Rust).

Tests to add (synthetic data, small):
- each entry placed on the canvas gives its own SMILES back; fused on a benzene bond gives the expected ring system; merge on and off
- property test with the expanded ops; parity entry

UI: template palette; e2e: place a chair and an amino acid with the mouse, one on an atom with touch, check SMILES; keyboard: pick from the list and place on the focused atom. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-12-templates`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
