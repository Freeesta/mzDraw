# arch-std-15: Clean up a drawing with the layout of `mzlab-mol`

Proposed by the study `docs/study/arch-std.md` (feature rows 2; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-15 clean-up: received, starting on branch `claude/arch-std-15-clean-up`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. The layout is not yours: call `mzlab-mol` at the pinned commit, write no layout code (COO 16 rule).

## Scope
In:
- Wire `crates/mzdraw/src/layout.rs::clean_up` (today a stub that returns the drawing unchanged) to `mzlab-mol` at the pin `ccd4680`: `Mol::layout_2d_fixed` with the atoms that are not selected held fixed (the whole structure when nothing is selected), and the overlap removal of `layout/clean.rs` that `layout_2d` already runs. The result is scaled from 1.5 Å bonds to one bond length (as `layout::computed` does) and placed so the fixed atoms do not move.
- One new operation with an exact inverse: `{"op":"atoms.xy","atoms":[ids],"xy":[[x,y],...]}` sets the positions of the listed atoms; its inverse holds the old positions. Clean up is one undo step (`Ctrl+L`, a button "Clean up").
- What `mzlab-mol` could not do (it returns `checked == false` or E/Z mismatches through `ez_mismatches`) is said in words under the canvas, never hidden.

Out (do not do here):
- Changes to the layout of `mzlab-mol` (report a bad case to the coordinator with a synthetic SMILES), moving objects (arrows, texts), cleaning a reaction scheme as a whole.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.1 geometry and overlap (IUPAC 2008 GR-3, GR-4.3), §2.10 overlap 8).
- `mzlab-mol` at `ccd4680`: `src/layout/mod.rs` (`layout_2d`, `layout_2d_checked`, `layout_2d_fixed`, `ez_mismatches`), `src/layout/clean.rs` (read it in `~/.cargo/git/checkouts/` after `cargo fetch`).
- `crates/mzdraw/src/layout.rs`, `crates/mzdraw/src/ops.rs`, `crates/mzdraw/tests/layout.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-15-clean-up` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. `clean_up(doc, selection) -> Vec<(id, x, y)>` through `mzlab-mol`; the editor turns it into one `atoms.xy` op.
2. Operation reader and writer by hand (`Op::from_value`, `Op::write`), validation (ids exist, finite coordinates within `MAX_COORD`), inverse.
3. Button and key; the live line says how many atoms moved.

## Tests to add
- Unit: a drawing with two overlapping atoms gets no `std.geom.atomClash` after clean-up (if arch-std-02 is merged; else assert the minimum distance); a selected ring is cleaned while the rest stays exactly in place; undo gives the identical document; E/Z of a drawn double bond is kept or the mismatch is said.
- Property test: `atoms.xy` in the random sequences of `tests/ops.rs`.
- Parity: a log with `atoms.xy`, and a replay request that cleans (if clean-up is a command of the request; else the op from the page log). e2e: clean up with the button by mouse and touch, with `Ctrl+L` on the keyboard.

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
