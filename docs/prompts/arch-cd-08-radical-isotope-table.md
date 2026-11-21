# arch-cd-08: Periodic table picker, and keys for radicals and isotopes

Study rows: B10, B18 of `docs/study/arch-cd.md`. Priority MUST. Size guess S. Depends on: nothing; uses arch-web-11's popover when it is on `main`.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-08 radical-isotope-table: received, working on claude/cd-08-radical-isotope-table`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- a periodic table picker: the page draws the grid Rust gives (symbol, row, column, block); a click or Enter picks `atom:<el>`
- key `.` over a hovered or focused atom cycles the radical (none → doublet → singlet → none); `Shift+I` opens the isotope field of the atom popover (arch-web-11) or, until it exists, a field validated in Rust
- radical dots and the mass number painted as now (already supported by `atom.set rad` / `iso`)

Out (do not build here, say so in the PR if you meet it):
- the atom popover itself (arch-web-11)
- lone pairs (arch-cd-25)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `ops.rs` `AtomSet` (`iso`, `rad` exist), `editor_tools.rs` `key`, `mzlab_mol::is_element` and the element table of `mzlab-mol` (`elements.rs`).
- Study facts: MarvinSketch opens the periodic table with Ctrl+E (MS/marvin_shortcuts-in-marvinsketch.html); ChemDoodle warns about a mass number not in its isotope table (CDD-G §14.6.3); ChemDoodle Web added an isotope popover in 9.2.0 (CDW/installation/changelog).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); rule 15 (CLI-native, agent-first): the feature is first a Rust command with a machine-readable form — `draw_edit/1` JSON ops, or a setting `--set <section>.<key>=<value>` of `mzlab-cli mol draw` — with stable JSON keys and error keys, deterministic bytes and no browser in the path; the page is one client of it, and a feature without a CLI route and a native replay test is incomplete; Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-08-radical-isotope-table` from `origin/main`.

1. Keys in the hotkey path (with arch-cd-03's table if merged, else next to `element_key`); `iso` validated against a known range for the element (refuse with `error.draw2.isotope`), 0 takes it away.
2. `periodic_table()` export: a JSON list from the `mzlab-mol` element table (no new data in JS); the page renders buttons with `aria-label` names from i18n.
3. A mass number unknown for the element is allowed but warned (geometry-free warning in `state`).

Tests to add (synthetic data, small):
- radical cycle and isotope set, each one undo step; refused values change nothing; the molfile carries `M  RAD` / `M  ISO`
- the periodic table JSON lists every element of the table once, in order

UI: a periodic table button and dialog-free picker (popover); e2e: open the table with the keyboard, pick Se, place it; press `.` over an atom; touch: pick from the table with a tap. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

Docs: keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice: what it does, the op/setting names, what is not built); update the row(s) of `docs/study/arch-cd.md` to `built` in the same PR.

CLI route and native replay (rule 15): elements, radicals and isotopes are `atom.set el|rad|iso` ops (replayed natively); the periodic table is a pure function `periodic_table()` of `crates/mzdraw` whose JSON is also a replay output (`out: "elements"`) compared by `tests/parity.mjs`.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-08-radical-isotope-table`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
