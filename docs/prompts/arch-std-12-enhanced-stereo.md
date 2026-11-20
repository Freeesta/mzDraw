# arch-std-12: enhanced stereo groups and the chiral flag

Proposed by the study `docs/study/arch-std.md` (feature rows 13; priority SHOULD). One small slice.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV arch-std-12 enhanced-stereo: received, starting on branch `claude/arch-std-12-enhanced-stereo`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- The document keeps enhanced-stereo groups per structure: `abs`, `and<n>`, `or<n>` (n ≥ 1) on stereocentres, and the chiral flag; one new operation with an exact inverse: `{"op":"stereo.group","atoms":[...],"g":"and1"}` (`"g":""` removes the atoms from any group; the inverse stores each atom's previous group).
- Read from molfile V3000 (`MDLV30/STEABS`, `STERELn`, `STERACn` collections) and V2000 chiral flag, CXSMILES (the enhanced-stereo fields of the extension block) and KET; written to V3000, CXSMILES only if CXSMILES is written (else listed as lost), KET. The import note "enhanced-stereo groups not kept" goes away for what is now kept.
- Drawn: `&1`, `or1` next to each centre and "abs" / "and enantiomer" text under the structure as the style says (`stereo_labels` on/off).

Out (do not do here):
- Recomputing InChI with enhanced stereo (outside the wasm), stereo perception (the record decides which atoms are centres).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/study/arch-std.md` (§2.2 (InChI v2026.1 reads the V3000 collections), §2.4 molfile and CXSMILES rows).
- Chemaxon CXSMILES page (https://docs.chemaxon.com/latest/formats_chemaxon-extended-smiles-and-smarts-cxsmiles-and-cxsmarts.html), "Enhanced stereochemical representation"; the CTfile V3000 collections (BIOVIA CTfile Formats; if you cannot read it, say so and implement from the files `mzlab-mol` already reads).
- `crates/mzdraw/src/io.rs` (import notes), `crates/mzdraw/src/ket.rs`, `crates/mzdraw/src/ops.rs`.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust (`crates/mzdraw`); `web/mzdraw.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/arch-std-12-enhanced-stereo` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Size: update `measured` in `tools/wasm-cap.toml` when the module moves. The owner lifted the size limit for now, but `cap` is still the owner's: if the module passes it, send the numbers to the coordinator and raise `cap` only on its answer, with the reason in the PR.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product.

## Work
1. Document field (`stg` on an atom, kept by older readers as an unknown field, written after the known fields as the format rule says).
2. Operation, inverse, property test; page: a small popover on a selected centre with Abs / And n / Or n (keyboard reachable).
3. Readers and writers; `docs/mzdraw-formats.md` §1a.

## Tests to add
- Unit: each group kind read and written in V3000, KET; round trip identical; the inverse restores the previous group.
- Parity: a log with `stereo.group`. e2e: set "and1" on two centres by mouse, by keyboard, by touch; the label shows; save V3000 has the collection.

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
