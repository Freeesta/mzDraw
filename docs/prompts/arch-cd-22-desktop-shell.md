# arch-cd-22: Desktop app, first slice: one window for macOS and Windows around the same module

Study rows: L01, L02 of `docs/study/arch-cd.md`. Priority MUST. Size guess L (this slice M). Depends on: a decision of the owner on the shell (see work 1).

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-22 desktop-shell: received, working on claude/cd-22-desktop-shell`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `origin/main` into your branch only on a real conflict, then rerun the acceptance commands and push. Never edit the step count of `tests/e2e.mjs` by hand.

## Scope

In:
- a measured choice of the shell: Tauri 2 (system webview, Rust), plain `wry` + `tao`, or a native Rust GUI; recorded in the PR with binary size, start time, licence, signing and update story for macOS and Windows
- a new crate `crates/mzdraw-desktop` that opens one window with the site files bundled (offline), a native menu (File: New, Open, Save, Save As, Recent; Edit: Undo, Redo, Copy, Paste; View) wired to the same `command` ids, native open/save dialogs, file associations for `.mzdraw` (and `.mol`, `.sdf`, `.cdxml` as open-with)
- CI builds for macOS and Windows as artifacts (unsigned is fine in this slice)

Out (do not build here, say so in the PR if you meet it):
- native clipboard formats (J07) and Office round trips (J08)
- auto-update (L03)
- code signing (needs the owner's certificates)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `CLAUDE.md` rule 3 (a new crate needs a reason in the PR; the dependency guard keeps `crates/mzdraw` on `mzlab-mol`, `libm`, `serde`, `serde_json` — the desktop crate is separate and must not change that).
- `docs/mzdraw.md` (desktop window is in the later list; `docs/mzdraw-formats.md` §2 desktop clipboard notes).
- Study facts: ChemDraw runs on Windows 10/11 and macOS 15/26 with only ChemDraw itself on the Mac (CD-HW26); BIOVIA Draw is Windows only (BD-DS24); ChemDoodle ships Windows, macOS (Intel and Apple Silicon) and Linux (CDD-HP).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); rule 15 (CLI-native, agent-first): the feature is first a Rust command with a machine-readable form — `draw_edit/1` JSON ops, or a setting `--set <section>.<key>=<value>` of `mzlab-cli mol draw` — with stable JSON keys and error keys, deterministic bytes and no browser in the path; the page is one client of it, and a feature without a CLI route and a native replay test is incomplete; Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-22-desktop-shell` from `origin/main`.

1. Before code: send COO-MZDRAW the comparison (one table) and wait for the owner's choice; another study may own the desktop shell — ask COO first.
2. The window loads the same `site/` build; files are read and written by the shell and handed to the module as bytes (`add_bytes`, `save`), so the desktop app and the page give the same bytes.
3. Settings and autosave live in the user's application-support folder, not in a registry (study §4 complaint).

Tests to add (synthetic data, small):
- a headless smoke test that starts the app, opens a synthetic molfile through the command path and saves `mzdraw/1` with the same bytes as the replay

UI: the window and its menus; screenshots on both systems from CI at scale 2 where the runner allows it; the page's e2e stays green. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

Docs: keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice: what it does, the op/setting names, what is not built); update the row(s) of `docs/study/arch-cd.md` to `built` in the same PR.

CLI route and native replay (rule 15): the window's menus call the same command ids as the page; every file the app writes comes from `Editor::save`; the app saves its session log, and a test replays that log natively (`cargo run --example replay`) to the same bytes as the app's own save. The CLI route stays `mzlab-cli mol draw`; no feature may exist only in the desktop app.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-22-desktop-shell`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
