# arch-std-02: conformance report, geometry and label checks

Proposed by the study `docs/study/arch-std.md` (feature rows 16; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-02 std-report-geometry: received, starting on branch `claude/arch-std-02-std-report-geometry`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Builds on the report `std/1` of arch-std-01 (merged before you start; if it is not on `main`, tell the coordinator and wait).

## Scope
In:
- The `std.geom.*` and `std.label.chargeHidden` checks of `docs/study/arch-std.md` §4 in `crates/mzdraw/src/conform.rs`: `atomClash` (two atoms closer than 0.3 bond length), `atomOnBond` (an atom within 0.15 bond length of a bond it is not on, measured to the segment), `crossing` (two bonds without a shared atom that intersect), `length` (a bond more than 15 % off the median length of its structure), `labelClash` (two label boxes overlapping, or a label box crossed by a bond it does not end on; the boxes are the ones `scene.rs` already computes for bond clearance), `chargeHidden`.
- A uniform grid over atoms and bond midpoints (as the hit-test) so a 500-atom drawing is checked in well under one frame; measure it natively and write the number into `docs/mzdraw.md`.

Out (do not do here):
- Any change to the 2D layout of `mzlab-mol`, moving atoms, the gap at crossings (arch-std-13).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.1, §4).
- `crates/mzdraw/src/scene.rs` (label boxes, `Painter::clear`), `crates/mzdraw/src/conform.rs` (from arch-std-01), `tests/data/flaw_sheet.smi` (synthetic sheet of labels).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-02-std-report-geometry` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Add the checks to the table of `conform.rs`; reuse the label geometry of `scene.rs` through a function both call (no second copy of the text metrics).
2. i18n texts `draw2.std.<key>`; the panel of arch-std-01 shows them with no JS change beyond the new keys.
3. Bench: a native test or example that builds a synthetic 500-atom drawing and times the report (release build), number in `docs/mzdraw.md`.

## Tests to add
- `crates/mzdraw/tests/conform.rs`: per check a synthetic positive and a near-miss negative (two atoms at 0.29 and 0.31 bond length; an atom on a bond at 0.14 and 0.16; two crossing chains and two touching at one atom; a bond at 1.16 and 1.14 of the median; `CO2H` next to `NH2` overlapping and apart).
- The flaw sheet (`tests/data/flaw_sheet.smi`, laid out by `mzlab-mol`) gives no `atomClash`, `atomOnBond` or `labelClash`: pin the count it gives today, so a layout or label regression shows up.
- Parity request with `"out":"std"` on a drawing with clashes. e2e: drag an atom onto another with the mouse and with touch; the panel shows `atomClash`; undo with `Ctrl+Z` clears it.

## Acceptance (all green before the PR)
```
cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
tools/build_site.sh
cargo build --locked -p mzdraw --example replay && node tests/parity.mjs
node tests/e2e.mjs
```
If a golden output moves because of a WANTED change: `MZDRAW_BLESS=1 cargo test -p mzdraw --test replay`, and say in the PR which lines moved and why.

## Messages
- First message to the coordinator, before any work: the confirm line above, verbatim.
- Ask the coordinator only when blocked by a decision that is not yours (a cap raise, a scope question); otherwise decide and say why in the PR.
- When the PR is open: one message with the PR link, the full head SHA, the module size before and after (bytes), the test counts, and one line per acceptance command (pass/fail).

## Handoff at 200k tokens
At about 200k tokens of context, stop starting new work: commit what builds with a message that says what is left, push the branch (no force), and send the coordinator a handoff message: branch, last commit SHA, what is done, what is left as a numbered list, the exact next command to run, and any decision taken that the next chat must keep.
