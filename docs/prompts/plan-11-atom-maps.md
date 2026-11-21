# plan-11: atom-to-atom maps on atoms, read and written in reaction files and SMILES

Proposed by `docs/plan/backlog.md` (row 35; Gemini bold idea 2 (reframed); built state: "Not built: atom-to-atom maps"). Size M. Wave 4. Depends on: arch-std-11 (same zone, runs before).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-11 atom-maps: received, starting on branch `claude/plan-11-atom-maps`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. 

## Scope
In:
- Atom field `map` (integer 1..999) in `mzdraw/1`; `atom.set` with `map` (0 or absent takes it away), exact inverse.
- Read: RXN V2000 and V3000 and molfile mapping numbers, SMILES `[CH3:4]` (through `mzlab-mol`'s reader if it keeps the class; check), KET `mapping` if present in its schema. Write: the same places; a format without maps lists them as lost.
- Display: the number at the lower right of the atom in a small size, setting `display.maps` (on, off), screen and SVG.
- Command `map:auto` is NOT in this slice (automatic mapping is a separate question for the coordinator).

Out (do not do here):
- Automatic atom mapping.
- `trasf/1` export (plan-12).

## CLI twin (rule 15)
- `{"op":"atom.set","id":3,"map":4}` replayed natively; `save("rxn")` writes the number in the atom line.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `rxn.rs`, `io.rs`, `ket.rs`, `doc.rs`, `ops.rs`; CTfile formats (BIOVIA, public PDF, 2020*): atom-atom mapping number field of the V2000 atom line and `AAMAP` of V3000; OpenSMILES 1.0 atom class.
- `mzlab-mol` molfile and SMILES readers: whether they keep the map; if not, list it as a gap for COO 16 and read the number in our own layer around them.

## Agreement needed before the PR
New atom field `map` in `draw_edit/1`: add to the plan-01 table and wait for COO 16. If `mzlab-mol` drops maps, the fix there is COO 16's.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-11-atom-maps` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Field, op, inverse, display.
2. Readers and writers per format; lost lists.
3. Goldens.

## Tests to add
- A synthetic esterification RXN with maps 1..6 reads back and writes the same map numbers on the same atoms (V2000 and V3000).
- SMILES `[CH3:1][OH:2]` in and out.
- Property test of ops with maps; e2e: set a map in the popover or the field with keyboard and touch.
- **Fails without the change**: the RXN round trip loses the maps today (they are not read).

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/maps.py`: RDKit reads our RXN and mapped SMILES (`AllChem.ReactionFromRxnBlock`, `GetAtomMapNum`) and the committed reference lists the same map per atom for 20 synthetic reactions. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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
- Ask the coordinator only when blocked by a decision that is not yours (a cap raise, a new op, a scope question); otherwise decide and say why in the PR.
- When the PR is open: one message with the PR link, the full head SHA, the module size before and after (bytes), the test counts, the independent-proof result, and one line per acceptance command (pass/fail).

## Handoff at about 150k tokens
At about 150k tokens of context, stop starting new work: commit what builds, push the WIP branch (no force) with a one-line state note at the top of the PR description or in the last commit message ("state: done X; next Y"), and send the coordinator: branch, last commit SHA, what is done, what is left as a numbered list, the exact next command to run, and any decision taken that the next chat must keep.

