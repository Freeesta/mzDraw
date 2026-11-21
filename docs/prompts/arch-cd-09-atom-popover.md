# arch-cd-09: Atom popover: element, charge, isotope, radical, hydrogens, label in one place

Study rows: B13 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: prompt 08 (isotope validation) if merged.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-09 atom-popover: received, working on claude/cd-09-atom-popover`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- a popover next to an atom (double-click, long press with a finger, `Enter` on the focused atom with `Alt`), never a modal dialog; every field reachable by `Tab`, `Escape` closes, `Enter` applies
- the fields read and written through one `atom.set` (one undo step); Rust validates and returns the field list with keys and values

Out (do not build here, say so in the PR if you meet it):
- bond properties (a later slice)
- query properties (OUT)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `docs/mzdraw.md` interaction rule 4 (no modal dialog for atom properties, a small popover usable from the keyboard).
- `ops.rs` `AtomSet` (all fields exist), `crates/mzdraw-wasm/src/lib.rs` `state` (what the page shows around the canvas).
- Study facts: MarvinSketch's Atom menu sets charge, valence, radical, isotope, map, alias (MS/marvin_atom-menu.html); BIOVIA Draw opens atom properties on right-click (BD-DS24).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-09-atom-popover` from `origin/main`.

1. `Editor::atom_fields(id)` → JSON `{id, fields:[{key, value, kind, min, max}]}`; `Editor::set_atom_fields(id, json)` → one `atom.set` or a refusal with the key of the first bad field.
2. The page builds the popover from that JSON (labels from i18n), places it beside the atom away from the pointer, keeps it inside the window.
3. Touch: long press already selects; a second long press on the selected atom opens the popover.

Tests to add (synthetic data, small):
- fields of a synthetic charged isotope atom; a bad charge is refused and changes nothing; one undo step for several fields

UI: the popover; e2e with mouse (double-click), keyboard (Tab through, Enter) and touch (long press); screenshot of the popover at deviceScaleFactor 2. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-09-atom-popover`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
