# arch-cd-15: Align, distribute, rotate by a typed angle and scale the selection

Study rows: I06, I07, I08 of `docs/study/arch-cd.md`. Priority MUST (I06), SHOULD (I07-I09). Size guess S. Depends on: arch-std-15 (op `atoms.xy`, PR #45) for scaling.

## Confirm message

Your first action: send COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; reach it with the `send_message` tool of the claude-code-remote server, by session id, not by name) one line: `arch-cd-15 align-distribute: received, working on claude/cd-15-align-distribute`. Then read, then work.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`, site https://freeesta.github.io/mzDraw/). You build ONE small slice, open ONE pull request ready for review and never merge it: COO-MZDRAW merges with the full head SHA. Never push to `main`, never rebase, never force-push; merge `origin/main` into your branch only on a real conflict, then rerun the acceptance commands and push. Never edit the step count of `tests/e2e.mjs` by hand.

## Scope

In:
- align the selected structures and objects (left, centre, right, top, middle, bottom) on their boxes; distribute three or more horizontally or vertically with equal gaps; centre on the view
- rotate the selection by a typed angle (a field); the 1° / 15° keyboard steps (`Alt+arrow`, `Shift+Alt+arrow`) belong to arch-std-05 and are not built here
- scale the selection by a factor (atoms through arch-std-15's `atoms.xy`, so the bond length changes only if the user asks)

Out (do not build here, say so in the PR if you meet it):
- groups (H12)
- guides while dragging (M has them; later)

## Context

Read first: `CLAUDE.md` (the rules override this prompt), `docs/mzdraw.md` ("Built state": what exists; do not rebuild it), the rows named above in `docs/study/arch-cd.md` (what the desktop editors do, with sources), and the files named below.

- `ops.rs` `Move`, `Rotate`, `Flip`; `Document::bounds`; `layout::pieces`; the command list of `crates/mzdraw-wasm/src/lib.rs` `command`.
- Study facts: ChemDraw aligns to the highest selected object, distributes three or more (CD-UG17); the new Marvin rotates 1° with Alt+arrow and 15° with Shift+Alt+arrow (M/marvin_shortcuts-and-tricks.html).

Rules that bite in this slice: all logic in Rust, JavaScript only forwards events and paints (rule 2); no new crate or npm package without a reason in the PR (rule 3); synthetic data only (rule 4); never skip or loosen a test (rule 5); every new operation has an exact inverse and joins the property test "apply then undo returns the identical document" (`random_step` in `crates/mzdraw/tests/ops.rs`, rule 11); every page function is producible from the CLI route and replays natively to the same bytes (rule 10, `tests/parity.mjs`); rule 15 (CLI-native, agent-first): the feature is first a Rust command with a machine-readable form — `draw_edit/1` JSON ops, or a setting `--set <section>.<key>=<value>` of `mzlab-cli mol draw` — with stable JSON keys and error keys, deterministic bytes and no browser in the path; the page is one client of it, and a feature without a CLI route and a native replay test is incomplete; Rust returns i18n keys, never sentences (`web/lang/mzdraw.js`, keys `draw2.*`). Never copy text, icons or artwork of another product; ideas from the study only.

## Work

Branch `claude/cd-15-align-distribute` from `origin/main`.

1. Commands `align:<edge>`, `distribute:h|v`, `rotate:<deg>`, `scale:<f>`, `nudge:<dx>,<dy>` produce existing ops only (`move`, `rotate`, `atoms.xy`), one undo step each.
2. Keys: arrow-key nudging and the rotation keys are arch-std-05's; if it is on `main`, call its commands, else leave the keys out and say so.

Tests to add (synthetic data, small):
- three synthetic structures aligned left share the left edge to 4 decimals; distribute gives equal gaps; rotate 90 four times returns the document; scale 2 then 0.5 returns it

UI: an Arrange group (align and distribute buttons, an angle field); e2e: select two pieces with a band, align top, check boxes from `state()`; the same from the keyboard (Tab to the Arrange buttons) and with touch. Screenshots at deviceScaleFactor 2 (`MZDRAW_SHOTS=<dir> node tests/e2e.mjs`), attached to the PR description; keep the touch/pen and keyboard steps of `tests/e2e.mjs` green and add yours.

Docs: keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice: what it does, the op/setting names, what is not built); update the row(s) of `docs/study/arch-cd.md` to `built` in the same PR.

CLI route and native replay (rule 15): every command produces existing ops (`move`, `rotate`, `atoms.xy`), so the log replays natively; also accept the commands in the replay request (`"commands": ["align:left", ...]`, run on the selection given by `"select"`) so an agent can align without computing offsets; parity entry.

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

When your context reaches about 200k tokens, stop starting new work: commit and push what passes, open (or update) the PR, and send COO-MZDRAW a handoff message: what is done, what is left (as a numbered list a fresh chat can follow), the branch `claude/cd-15-align-distribute`, the last green command, and any trap you found. A successor continues on the same branch from that message and this prompt.
