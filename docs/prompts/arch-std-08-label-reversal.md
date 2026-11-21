# arch-std-08: group labels reversed on the left side

Proposed by the study `docs/study/arch-std.md` (feature rows 5; priority MUST). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-08 label-reversal: received, starting on branch `claude/arch-std-08-label-reversal`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- An atom label written as a group (`CO2H`, `OMe`, `CF3`, `NH2`, `NO2`, `SO3H`, `OAc`, the abbreviations of `label.rs` and any label the user types) is drawn so that its attachment atom faces the bond: when the bonds leave to the right of the atom, the label is written reversed by chemical units (`HO2C`, `MeO`, `F3C`, `H2N`, `O2N`, `HO3S`, `AcO`), with the attachment symbol centred on the atom as today.
- A reversal table for multi-letter units (an abbreviation stays one unit: `Boc`, `Ph`, `Ac`, `Me` are never split) and the rule for digits (a count stays after its symbol: `CO2H` → `HO2C`).
- The bond aims at the centre of the attachment symbol (its first letter, or the last letter of a reversed label), and that symbol's centre stands where the bond would end without a label (GR-2.1.5); counts and charges stay right of their own symbol (GR-2.1.6).
- The same rule in the SVG and the canvas (one code path in `scene.rs`).

Out (do not do here):
- Charges moved off a crowded corner (separate), abbreviation expansion, label typing UI.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.1 labels (IUPAC 2008 GR-2.1.5 bond aiming and GR-2.1.6 label reversal, read on https://iupac.qmul.ac.uk/drawing/drawing.html)).
- `crates/mzdraw/src/label.rs` (abbreviation table), `crates/mzdraw/src/scene.rs` (label layout, `Painter::clear`), `crates/mzdraw/tests/labels.rs`, `crates/mzdraw/tests/crowding.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-08-label-reversal` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. A tokenizer of a label into units (element symbol + count, abbreviation names, parentheses kept as units) and a reversal that keeps each unit's internal order.
2. Choose left or right from the bond directions already used for hydrogens; ties keep today's placement.
3. Bond clearance (`Painter::clear`) uses the reversed box.

## Tests to add
- Unit: a table of 20 labels with their reversed forms; `Boc` and `Ph` unchanged; a label with no bond keeps its written form.
- Golden SVGs that move: bless and list them; the flaw sheet gains two acid and ester labels on the left of a ring (synthetic).
- Parity request with a left-side `CO2H`. e2e: type `CO2H` on an atom whose bond comes from the right with the keyboard, and with touch, screenshot at deviceScaleFactor 2.

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
