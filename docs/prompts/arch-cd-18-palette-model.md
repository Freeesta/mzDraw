# arch-cd-18: Palettes, part 1: tool groups, show/hide, remembered layout, reset, kept inside the window

Study rows: K01, K05, K09, P01, P04, P05, P06, P07, P08 of `docs/study/arch-cd.md`. Priority MUST. Size guess M. Depends on: nothing.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-18 palette-model: received, working on claude/cd-18-palette-model`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `main` into your branch only on a real conflict.

## Scope

In:
- a layout model in Rust (`crates/mzdraw/src/ui.rs`): palettes (id, tools, face tool of each group), docked edge and order, visibility; serialised as section `ui` of `mzdraw-settings/1`; not part of the document, no undo step
- tool groups: a group button shows its face tool; pressing and holding (or a small corner mark) opens its sub-palette; the variant picked becomes the face
- View menu: show/hide each palette with a check mark; a key to show/hide all palettes; `Reset layout` always reachable (also when everything is hidden)
- clamp: after a resize or zoom change no palette is outside the window
- keyboard: Tab to a palette, arrows inside, Enter opens a group, Escape closes; SVG icons only (sharp at any scale)

Out (do not build here, say so in the PR if you meet it):
- tear-off and docking by drag (prompt 19)
- named layouts and customising tools (P09, P10)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `docs/study/arch-cd.md` §4 (what ChemDraw, MarvinSketch, BIOVIA, ChemDoodle do and what users complain about).
- `web/index.html`, `web/mzdraw.js`, `web/mzdraw.css` (today's toolbar); `settings.rs` (sections, OPFS keeping in the page).
- Rule 2: JS renders the palettes from the model JSON and forwards clicks and keys; which palette is where, what is visible and what the face of a group is are decided in Rust.

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-18-palette-model` from `origin/main`.

1. `ui::Layout` with `to_json`/`from_json` by hand, bounds checked; `Editor::ui()` gives the model, `Editor::ui_set("ui.palette.bonds.visible=false")` changes it, `command("ui-reset")` resets.
2. Groups: bonds (single, double, triple, wedges, chain), rings (3-8, benzene), atoms, charges, arrows (with prompt 06 kinds when merged), text, labels, edit (select, erase).
3. Page: render from the model; keep the model in OPFS with the settings; never lose it on an unknown key (unknown keys kept).
4. Complaints to design against (study §4): a palette that vanishes for good, layouts lost without admin rights, windows off-screen after a monitor change, tiny icons on 4K screens.

Tests to add (synthetic data, small):
- model round trip; reset gives the default bytes; clamping of a palette at x = 5000 in a 1280 px window; a hidden-everything layout still offers reset

UI: the new toolbar with groups and the View menu; e2e: open the bond group, pick hashed, the face changes; hide the ring palette, reload, still hidden, reset brings it back; keyboard walk through palettes; touch: groups open with a tap; screenshots of the default layout and of an open group. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-18-palette-model`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
