# arch-cd-19: Palettes, refinement 2 on top of R1 (PR #35): named layouts and icon size

Study rows: P09, P11 of `docs/study/arch-cd.md`. Priority SHOULD. Size guess S. Depends on: PR #35 merged; arch-cd-18 if it touches the same code.

This prompt refines R1's `layout` object; it adds no second layout model.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-19 named-layouts: received, working on claude/cd-19-named-layouts`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `origin/main` into your branch only on a real conflict, then rerun the acceptance commands and push. Never edit the step count of `tests/e2e.mjs` by hand.

## Scope

In:
- named layouts: save the current layout under a name, switch, rename, delete, export and import one file; kept in the settings file next to `layout` (e.g. `layouts: {name: <layout object>}`), only what differs from the preset
- icon size `small` (32 px items) or `large` (44 px, today's) as `layout.icons`, with R1's sizes and separators recomputed in Rust

Out (do not build here, say so in the PR if you meet it):
- customising which tools a palette holds (P10, later)
- a second layout model

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/built/` (the Built state: what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw/src/palettes.rs`, `docs/mzdraw-ui.md` §4-§5 (item 44 px, padding 4 px), `settings.rs` (unknown keys kept).
- Study facts: ChemDoodle saves, reverts and resets its workspace (CDD-G §2.2.7); MarvinSketch configurations can be copied, renamed, reset, imported and exported (MS-IOD/configurations-of-marvinsketch.html) and toolbars have large or small icons (MS-IOD/customizing-marvinsketch-gui.html).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); rule 15 (CLI-native, agent-first): the feature is first a Rust command with a machine-readable form — `draw_edit/1` JSON ops, or a setting `--set <section>.<key>=<value>` of `mzlab-cli mol draw` — with stable JSON keys and error keys, deterministic bytes and no browser in the path; the page is one client of it, and a feature without a CLI route and a native replay test is incomplete; Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-19-named-layouts` from `origin/main`.

1. Settings: `layouts` and `layout.icons` through the same reader and writer as R1 (refusal keys for bad names or objects).
2. `Editor::set("layout.save=<name>")`, `layout.use=<name>`, `layout.delete=<name>`; the Palettes menu lists the names.

Tests to add (synthetic data, small):
- save, switch, delete, round trip; small icons give 32 px items and the column heights of `docs/mzdraw-ui.md` §4 scaled; every palette stays inside the viewport

UI: the names in the Palettes menu and a size toggle; e2e: save a layout, change it, switch back; keyboard path; screenshots small and large. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

Docs: keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice: what it does, the op/setting names, what is not built); update the row(s) of `docs/study/arch-cd.md` to `built` in the same PR.

CLI route and native replay (rule 15): settings only (`--set layout.save=<name>`, `--set layout.use=<name>`, `--set layout.icons=small`), no `draw_edit/1` op; the replay output `out: "layout:<w>,<h>"` (from arch-cd-18 or R1) compares the arranged JSON with small icons wasm == native.

Size: the owner lifted the wasm size limit (the cap in `tools/wasm-cap.toml` is now only a runaway guard); rule 8 still holds: measure with `tools/build_site.sh`, there is no `measured` to write (the build prints the size); give the delta in the PR.

## Acceptance

All of these pass, and their output is quoted in the PR:
```
cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
tools/build_site.sh
cargo build --locked -p mzdraw --example replay && node tests/parity.mjs
node tests/e2e.mjs
```

If a golden output changes because of this slice (and only then), rewrite it with `MZDRAW_BLESS=1 cargo test -p mzdraw --test replay` and explain each changed file in the PR. Extend your own file `tests/data/log/<area>.json` with the new op(s) so that the parity test replays them.

## Messages

- To COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) only. Start: the confirm line above. Blocked (a test you cannot make pass, a decision that is not yours, a host refused): one message with the exact error, then wait or continue with the rest.
- Done: one message with the PR link, the full head SHA, the module size before → after, and the study rows now built.
- The PR is in English, ready for review (not a draft), titled after the feature, its body following the repository's usual sections, with the screenshots for UI changes.

## Handoff at 200k tokens

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-19-named-layouts`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
