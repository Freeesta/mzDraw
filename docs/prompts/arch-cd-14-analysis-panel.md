# arch-cd-14: Analysis panel: formula, MW, exact mass, m/z, elemental analysis of the selection

Study rows: F01 of `docs/study/arch-cd.md`. Priority MUST. Size guess S. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-14 analysis-panel: received, working on claude/cd-14-analysis-panel`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- a panel that shows, for the selection or (nothing selected) each structure: Hill formula, average mass, monoisotopic mass, m/z of [M+H]+ and [M−H]− by default, elemental analysis (% by mass); abbreviations count as their atoms
- values computed in Rust by the engine mzLab already has (`selection/1` of `mzlab-feat`, feature `sel`), through the bridge like `check/1`
- copy of the values as text

Out (do not build here, say so in the PR if you meet it):
- live captions on the canvas (F02, later)
- adduct choice and isotope pattern (F03, later)
- logP and other descriptors (F05)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw/src/outline.rs` (Hill formula already counted on the record), `crates/mzdraw-wasm/src/lib.rs` (`recheck` pattern; `state` JSON), `crates/mzdraw-wasm/Cargo.toml` (today `mzlab-feat` with feature `check` only; `sel` adds `chem::sel::sel_json`, `selection/1`: label/1 structure, average weight, counts). `mzlab-mol` has no mass table, so the numbers come from `mzlab-feat` in the bridge, as `check/1` does, and never from `crates/mzdraw`.
- Masses: monoisotopic from the most abundant isotope, average from standard atomic weights — use mzLab's tables, never type numbers into JS.
- Study facts: ChemDraw's Analysis window covers formula, exact mass, MW, m/z and elemental analysis in every tier (CD-DC26); MarvinSketch's analysis box adds isotope composition and updates live (MS/marvin_analysis-box.html).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-14-analysis-panel` from `origin/main`.

1. `Editor` or bridge function `analysis()` → JSON per structure `{formula, mw, mono, mz: [{adduct, mz}], ea: [{el, pct}]}`, numbers written through `mzlab_mol::fixed` or serde_json (never `f64` through std formatting: it costs 22 kB).
2. The page shows the panel next to the outline; `aria-live` stays on the announcement line only.
3. CLI twin: `mzlab-cli mol` on the same molfile gives `selection/1` / `label/1`; the panel's JSON must be derived from exactly that output (no number computed twice). Add a native test in `crates/mzdraw-wasm` (it is an `rlib` too) that compares the panel JSON with `sel_json` on the molfile of the same document, and say in the PR that `tests/parity.mjs` does not cover it (the replay of `crates/mzdraw` has no `mzlab-feat`).
4. Measure the size of adding the `sel` feature to the bridge and report it.

Tests to add (synthetic data, small):
- caffeine, ethanol, a charged synthetic ion: formula and masses to 4 decimals against hand-computed values; selection of one piece of two; an abbreviation counted as its atoms

UI: the panel; e2e: draw ethanol, read the panel; select one of two structures; screenshot. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-14-analysis-panel`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
