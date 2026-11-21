# arch-cd-20: Context menu, refinement on top of PR #37: order, abbreviation target, touch and keyboard, every item tested and replayable

Study rows: K03, X03, X04, X06, X07, X08, X13 (§7) of `docs/study/arch-cd.md`. Priority MUST. Size guess S. Depends on: PR #37 (context menu model) and its page PR 2 merged on `main`.

This prompt refines the menu that PR #37 builds (`crates/mzdraw/src/context.rs`); it adds no second menu model. arch-web-12 is the same first slice as PR #37 and is not run again. If PR #37 or its page PR is not on `main`, tell COO-MZDRAW and wait.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-20 context-menu-refine: received, working on claude/cd-20-context-menu-refine`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `origin/main` into your branch only on a real conflict, then rerun the acceptance commands and push. Never edit the step count of `tests/e2e.mjs` by hand.

## Scope

In:
- order of every menu as §7 X04 of `docs/study/arch-cd.md`: frequent actions, the kinds of the target, the clipboard, then Delete last after a separator; no entry repeated on every menu
- an abbreviation target (an atom with a label): Expand, Contract, Edit label, Delete — Expand / Contract only when arch-web-18 is on `main`, else the label and Delete
- one test per item: run it through `context_run` on a synthetic document and check the document after it (the item does exactly what its label says)
- the CLI route of an item: the replay request takes `"context": [{"x":…, "y":…, "item":"bond.o.1"}]` (menu at the pixel of the default view, then the item), so agents and `tests/parity.mjs` reach every item natively
- touch and pen: decide how a long press opens the menu without losing today's long-press selection (open the menu on the same press once it has selected, or on a second long press), and the pen's barrel button as a right-click; keyboard: `Shift+F10` and the Menu key on the focused atom or bond, arrows, Left / Right for submenus, Enter / Space, Escape gives the focus back
- Change direction (`bond.reverse`) shown only for bonds that have a direction (wedges, and the dative bond of arch-web-20 when it exists), as Ketcher does (KE-SRC)

Out (do not build here, say so in the PR if you meet it):
- new commands (an entry appears when its feature lands: popover, isotope, enhanced stereo, double-bond side, clean, align: §7 X09-X12)
- the warning-mark menu (X15) and display toggles (X14): a later refinement

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw/src/context.rs` and `crates/mzdraw/tests/context.rs` (PR #37): `context_menu`, `context_menu_at`, `context_run`, `ASK`, the fingerprint that refuses a stale menu; `crates/mzdraw/src/replay.rs` (`out: "context:<x>,<y>"`).
- `crates/mzdraw/src/editor_touch.rs` (the long press selects, half a second, finger held still).
- `docs/study/arch-cd.md` §7: what ChemDraw, MarvinSketch, Marvin JS, the new Marvin, Ketcher and ChemDoodle put in each menu, and what users ran into (Ketcher #10377: an item that sets something else than its label; #3269: a click on a separator closed the menu; #4445: the selection rule; Marvin JS 14.11.10: a long press on an iPad opened the wrong menu).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); rule 15 (CLI-native, agent-first): the feature is first a Rust command with a machine-readable form — `draw_edit/1` JSON ops, or a setting `--set <section>.<key>=<value>` of `mzlab-cli mol draw` — with stable JSON keys and error keys, deterministic bytes and no browser in the path; the page is one client of it, and a feature without a CLI route and a native replay test is incomplete; Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-20-context-menu-refine` from `origin/main`.

1. Reorder the item lists in `context.rs` as X04 and add separators to the menu JSON (`{"sep": true}` or a group field; keep the ids stable).
2. Abbreviation target in `Target`; its items run existing ops.
3. Replay request field `context` (a list, applied in order after the log, before `out`); a refusal (stale, unknown item, a pixel with nothing) answers its error key with the index of the entry.
4. Tests: one per item id; the golden file gets the menus of the reordered targets (`MZDRAW_BLESS=1` only for the changed menus, each explained).

Tests to add (synthetic data, small):
- every item id of every target: the document after `context_run` is the one the label promises (synthetic ethane, benzene, an arrow, a text, a Boc label)
- a separator is not an item (it cannot be run); the abbreviation target appears for a labelled atom only
- replay with `context` gives the same bytes natively and in wasm

UI: the reordered menus with separators; e2e: right-click an atom and a bond with the mouse, a long press with a finger, `Shift+F10` on the focused atom; screenshots of the atom and the abbreviation menus at deviceScaleFactor 2. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

Docs: keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice: what it does, the op/setting names, what is not built); update the row(s) of `docs/study/arch-cd.md` to `built` in the same PR.

CLI route and native replay (rule 15): the menu is already a replay output (`out: "context:<x>,<y>"`, PR #37); this slice adds the replay field `context` that runs items natively, so every item has a CLI route (`mzlab-cli mol draw` passes the request) and a native replay test in `tests/parity.mjs`.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-20-context-menu-refine`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
