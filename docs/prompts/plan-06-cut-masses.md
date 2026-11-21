# plan-06: formula and exact m/z of the pieces of a cut

Proposed by `docs/plan/backlog.md` (row 28; Gemini card 1). Size M. Wave 3. Depends on: plan-04, plan-05.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-06 cut-masses: received, starting on branch `claude/plan-06-cut-masses`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Annotation, never prediction: the user chooses the bonds and the mode; the editor computes and shows the numbers.

## Scope
In:
- Through `glue::run("cut", ...)`: for each cut, the pieces of `cut/1` with formula, monoisotopic neutral mass and the m/z of the chosen ion form; the ion forms and adducts come from mzLab's table (the `adduct` argument; default `[M+H]+` in positive and `[M-H]-` in negative mode).
- Optional H shift per piece (`h`: integer from -3 to +3) for hydrogen transfers, a field of the cut object (`obj.set h`).
- A callout per piece placed by Rust beside the piece on the side away from the cut: formula with lowered digits, m/z to 4 decimals (the same rounding as mzLab), ion form. Painted on screen and in SVG/PNG.
- Replay: `"engines"` answers are part of the request result so the native replay (plan-04) gives the page's bytes.
- Page: mode, polarity, adduct and H shift in the cut's flyout (paint and forward only).

Out (do not do here):
- Matching to observed peaks (plan-07).
- Ranking cuts or predicting fragmentation.

## CLI twin (rule 15)
- `replay_glue` with `{"engines":[{"engine":"cut","args":{...}}]}`; in mzLab, `mzlab-cli mol cut FILE --set bonds=3-4` on the same molfile gives the same numbers (compared in a test).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- mzLab `src/chem/cut.rs`, `fragments.rs`, `mzlab-core/src/molecules/ion.rs` (adducts), `mzlab-core/src/nuclides.rs` (read; do not change).
- Electron mass: CODATA 2022, 5.485799090441e-4 u (https://physics.nist.gov/cgi-bin/cuu/Value?meu); check which value mzLab uses and say it in the PR.

## Agreement needed before the PR
New field `h` and `adduct` on the cut object: add to the plan-01 table and wait for COO 16's agreement.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-06-cut-masses` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Call the engine through the glue; map pieces back to ids.
2. Place and paint the callouts (no overlap with the drawing: reuse the label boxes of `scene.rs`).
3. Fields and flyout; goldens.

## Tests to add
- Ethyl acetate cut at the ester C–O, `hydrogen` mode, `[M+H]+`: formulas and m/z of both pieces equal a value written in the test and computed by hand from the nuclide table (show the sum in a comment).
- Negative mode and an H shift change the m/z by the right multiple of the proton mass.
- Parity wasm == native with the engine.
- e2e: change the polarity in the flyout with the keyboard; the callout text changes.
- **Fails without the change**: the hand-computed m/z test fails if the electron mass is not taken off for a cation (a 0.00055 difference at 4 decimals).

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/masses.py` computes the same pieces with RDKit (`Descriptors.ExactMolWt` of the piece SMILES, minus or plus the electron mass) for 50 synthetic cuts; the committed reference must agree within 1e-5 u. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

