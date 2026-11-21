# plan-12: a drawn parent → product saved as a `trasf/1` transformation record

Proposed by `docs/plan/backlog.md` (row 47; Gemini bold idea 2 (reframed: `trasf/1` exists in `mzlab-mol`)). Size M. Wave 5. Depends on: plan-11.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-12 trasf-out: received, starting on branch `claude/plan-12-trasf-out`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. This ties drawings to the transformation-product screening of mzLab with no new engine.

## Scope
In:
- `save("trasf")`: the drawing must hold one arrow with one structure before it and one after it (else `error.draw2.trasfShape`); the maps of plan-11 give the atom map; the record is built by `mzlab-mol`'s `transform.rs` from the two `mol/1` records (no second writer here).
- A product drawn without a complete map is written as the uncertain form (`delta`, `sites`) when `mzlab-mol` supports it from our inputs, else refused with `error.draw2.trasfMap` naming the unmapped atoms.
- Open: a `trasf/1` file with its two records (or the records it names given beside it) is drawn as parent → arrow → product, maps shown.

Out (do not do here):
- Reaction SMARTS (query language, rule 13).
- Open Reaction Database export (dropped).

## CLI twin (rule 15)
- `{"out":"trasf"}` in the replay request; in mzLab the record is read by its own reader (a test in our repo calls `mzlab_mol::transform` to read our bytes back).

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- mzLab `crates/mzlab-mol/src/transform.rs`, `transform_chain.rs`, `delta.rs` and the study `docs/studi/TRANSFORMATION_RECORD.md` on mzLab (read; do not change).

## Agreement needed before the PR
No new op. If `transform.rs` needs a constructor from two records and a map that it does not expose at the pinned commit, that is a COO 16 change: ask first.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-12-trasf-out` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Shape check of the drawing; build the record through `mzlab-mol`.
2. Open path.
3. Synthetic hydroxylation, N-demethylation and ester hydrolysis.

## Tests to add
- Each synthetic transformation: save, read back with `mzlab-mol`, the delta formula equals the one written in the test (`+O`, `-CH2`, `+H2O-...`).
- Refusals with their keys; parity; e2e: draw parent, arrow, product, map two atoms, save.
- **Fails without the change**: `save("trasf")` does not exist; the delta test fails.

## Independent proof
Correctness must not rest on our own claims. RDKit applies the mapped reaction (`rdChemReactions.ReactionFromSmarts` on the mapped reaction SMILES we write, product checked by InChIKey) for the synthetic set in `tools/fidelity/trasf.py`; mzLab's own `trasf/1` tests are the second check. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

