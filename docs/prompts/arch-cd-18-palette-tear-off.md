# arch-cd-18: Palettes, refinement 1 on top of R1 (PR #35): tear-off flyouts, home on double-click, the Palettes menu with check marks

Study rows: P02, P03, P04, K01 of `docs/study/arch-cd.md`. Priority MUST. Size guess S. Depends on: PR #35 (R1, `crates/mzdraw/src/palettes.rs`, `docs/mzdraw-ui.md` §4-§5) merged on `main`.

This prompt refines what R1 builds; it adds no second layout model. If PR #35 is not on `main` when you start, tell COO-MZDRAW and wait.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-18 palette-tear-off: received, working on claude/cd-18-palette-tear-off`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `origin/main` into your branch only on a real conflict, then rerun the acceptance commands and push. Never edit the step count of `tests/e2e.mjs` by hand.

## Scope

In:
- tear-off: a flyout of a palette (its kinds, e.g. the six bond kinds) dragged by its header becomes a floating palette of its own; in the `layout` object it is one more palette entry with `from` (the source palette id) and the same fields R1 uses (`dock`, `x`, `y`, `orient`, `hidden`); closing it removes the entry
- a double-click (or `Home` on a focused handle) on a palette handle puts that palette back where the preset has it (one palette, not the whole reset)
- the Palettes menu lists every palette with a check mark for visible, from the Rust model; `Reset layout` stays the last entry and is reachable when every palette is hidden

Out (do not build here, say so in the PR if you meet it):
- a second layout model, a `ui` section or `ui.rs` (R1's `layout` object is the one model)
- named layouts and icon size (arch-cd-19)
- palettes on a second monitor

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw/src/palettes.rs` (`Layout::press/drag/release/key/toggle/reset/arranged_json`, `Dock`, presets), `crates/mzdraw/tests/palettes.rs` (the property test over random sequences), `docs/mzdraw-ui.md` §4-§5 (the decisions; keep them).
- `docs/study/arch-cd.md` §4: ChemDraw tears sub-palettes off by their title and docks toolbars by double-click (CD-UG17, 17.1); complaints to design against: palettes that vanish for good, layouts lost without administrator rights, windows off-screen after a monitor change (CD-KB articles*).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); rule 15 (CLI-native, agent-first): the feature is first a Rust command with a machine-readable form — `draw_edit/1` JSON ops, or a setting `--set <section>.<key>=<value>` of `mzlab-cli mol draw` — with stable JSON keys and error keys, deterministic bytes and no browser in the path; the page is one client of it, and a feature without a CLI route and a native replay test is incomplete; Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-18-palette-tear-off` from `origin/main`.

1. Extend `Layout` with torn-off palettes (id `<from>.torn` or a counter; choose and document), read and written in the `layout` object like the others (only what differs from the preset; unknown keys kept).
2. A drag that starts on a flyout header goes through `press/drag/release` as for a palette handle; R1's snapping and clamping apply unchanged.
3. `Layout::home(id)` and the Palettes menu model `Layout::menu_json()` (id, i18n key, visible); the page paints them.
4. Extend R1's property test: random sequences with tear-offs and homes keep every palette inside the viewport and round-trip through the settings text.

Tests to add (synthetic data, small):
- tear off the bonds flyout, move it, write the settings, read them back: the same arranged JSON; home of one palette leaves the others where they are; reset removes torn-off palettes

UI: tear-off and the Palettes menu; e2e: tear off the bonds flyout with the mouse, move it, reload, it is where it was; double-click its handle; the same with touch (one finger on the header) and with the keyboard (Enter on the header, arrows, Home); screenshots of a torn-off palette and of the menu. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

Docs: keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice: what it does, the op/setting names, what is not built); update the row(s) of `docs/study/arch-cd.md` to `built` in the same PR.

CLI route and native replay (rule 15): the layout is a setting, not a `draw_edit/1` op (`docs/mzdraw-ui.md` §5): the CLI twin is `--set layout=<object>`; add a replay output `out: "layout:<w>,<h>"` (the arranged JSON for a viewport) if R1 has none, and a parity entry with a torn-off palette, so the arranged rectangles are compared wasm == native.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-18-palette-tear-off`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
