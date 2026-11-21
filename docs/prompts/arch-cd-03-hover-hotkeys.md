# arch-cd-03: Hotkeys over the hovered bond and group hotkeys over an atom

Study rows: B02, B03 of `docs/study/arch-cd.md`. Priority MUST. Size guess S. Depends on: arch-std-06 (the command table of shortcuts) if it is on `main`: put the hover keys into that table.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-03 hover-hotkeys: received, working on claude/cd-03-hover-hotkeys`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `origin/main` into your branch only on a real conflict, then rerun the acceptance commands and push. Never edit the step count of `tests/e2e.mjs` by hand.

## Scope

In:
- over a hovered (or keyboard-walked) bond: `1` `2` `3` set the order, `W` / `Shift+W` / `Y` make it solid, hashed or wavy, a second press of the same wedge key turns it around
- over a hovered or focused atom: `M` Me, `E` Et, `Shift+F` CF3, `Shift+O` OH (letters already used for elements keep their meaning: pick group keys that do not clash and list them), a digit `n` with `Alt` sprouts a chain of n carbons, `Shift+n` (3-8) fuses a ring of n on the atom
- one undo step per key, announced; the key table lives in one Rust table (arch-std-06's command table when it is on `main`)

Out (do not build here, say so in the PR if you meet it):
- rebinding (arch-std-06)
- new bond kinds (arch-cd-04, arch-web-20)
- keys when the pointer is over nothing: they keep picking tools as today

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/built/` (the Built state: what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `crates/mzdraw/src/editor_tools.rs` `key` and `element_key` (elements over the focused, else hovered atom) and `crates/mzdraw/src/editor_keys.rs` `focus_key` (digits act on the walked bond only).
- `Overlay::hover_atom`, `hover_bond` in `scene.rs`; ring placement in `build.rs` (`ring`, `sprout_angle`).
- Study facts: ChemDraw hover keys 1/2/3, b, w, h, y, d on a bond, m, e, Shift+f on an atom (CD-UG17); ChemDoodle digits for chains and Shift+digit for rings, F turns a bond (CDD-K); the new Marvin has atom-context ring and acyl keys (M/marvin_shortcuts-and-tricks.html). Do not copy their key tables: choose ours so that nothing built changes meaning, and write the table in the help.

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); rule 15 (CLI-native, agent-first): the feature is first a Rust command with a machine-readable form — `draw_edit/1` JSON ops, or a setting `--set <section>.<key>=<value>` of `mzlab-cli mol draw` — with stable JSON keys and error keys, deterministic bytes and no browser in the path; the page is one client of it, and a feature without a CLI route and a native replay test is incomplete; Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-03-hover-hotkeys` from `origin/main`.

1. One table `hotkeys::TABLE` in a new module (target `atom`/`bond`, key, modifiers, command) used by `key()` before the tool keys; hovered thing first, then the focused atom / walked bond, else fall through to today's behaviour.
2. Groups are built from existing ops (`atom.add`, `bond.add`, `ring`) with the sprout angle of `build.rs`; abbreviations go through `atom.set lab` where the label table has them (Me, Et, CF3 if present; add CF3 and OH to `label.rs` only if they are missing and valid).
3. Help: the list of hover keys in the closed help of the page, generated from the table (Rust returns keys and parameters).

Tests to add (synthetic data, small):
- each key over a bond and over an atom gives the expected document (synthetic ethane, benzene); one undo step each; a key over empty canvas still picks its tool
- the log of a session with hover keys replays to the same bytes (parity entry)

UI: no new controls; e2e: hover a bond with the mouse and press `2`, `W`, `W`; hover an atom and press `M` and `Shift+6`; the same with the keyboard focus instead of the mouse. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

Docs: keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice: what it does, the op/setting names, what is not built); update the row(s) of `docs/study/arch-cd.md` to `built` in the same PR.

CLI route and native replay (rule 15): the hover keys produce existing `draw_edit/1` ops (`bond.set`, `atom.add`, `bond.add`, `ring`, `atom.set`), so the log of a session replays natively; add a replay request field `keys` (a list of `{key, mods, hover: atom or bond id}` applied through `Editor::key` before `out`) so that the key handling itself has a native test and a CLI route (`mzlab-cli mol draw --set edit=` replays the ops; agents can also send `keys`). Add an entry to your own file `tests/data/log/<area>.json` that uses `keys`.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-03-hover-hotkeys`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
