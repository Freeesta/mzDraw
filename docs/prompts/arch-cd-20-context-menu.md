# arch-cd-20: Context menu on atoms, bonds, objects, the selection and the canvas

Study rows: K03 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: nothing (uses commands that exist; later prompts add entries).

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-20 context-menu: received, working on claude/cd-20-context-menu`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- right-click, long press with the pen, `Shift+F10` / the context-menu key on the focused atom: a menu of the commands that apply to what is under the pointer (element, charge, bond order and kind, wedge turn, delete, copy, expand/contract when built, clean when built, properties when built)
- the menu model (entries, enabled, checked, key shown) comes from Rust; the page draws it and sends the chosen command id

Out (do not build here, say so in the PR if you meet it):
- new commands: the menu only lists what exists

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw-wasm/src/lib.rs` `command`, `editor_touch.rs` (long press selects with a finger: keep that; the menu opens on a second long press or with the pen's barrel button), the shortcut registry of prompt 17 if merged.
- Study facts: BIOVIA Draw opens atom, bond and fragment properties from right-click (BD-DS24); MarvinSketch's Atom menu (MS/marvin_atom-menu.html).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-20-context-menu` from `origin/main`.

1. `Editor::menu(x, y)` → JSON entries for the hit; `command(id)` runs them; entries carry i18n keys.
2. The page keeps the menu inside the window, closes it on Escape and outside clicks, moves focus into it and back.

Tests to add (synthetic data, small):
- menu of an atom, a double bond, an arrow, a selection, the empty canvas: entries and enabled flags; running an entry equals the command

UI: the menu; e2e: right-click an atom and set N; keyboard: context-menu key on the focused atom; touch/pen path; screenshot. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-20-context-menu`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
