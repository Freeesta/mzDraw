# plan-22: what a format loses, listed before the file is written

Proposed by `docs/plan/backlog.md` §9 (row 71; second Gemini input, idea 5, "format loss manifest"). Size M. Wave 3. Depends on: plan-20 (the `expect` arm), plan-15 (diff, to compare a round trip).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-22-loss-manifest: received, starting on branch `claude/plan-22-loss-manifest`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. Rust only. Rule 13 already says a file we READ is opened with its unsupported parts listed; this is the same promise for the files we WRITE.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- A new module `crates/mzdraw/src/loss.rs`: one table of what each writer carries (`molfile` V2000, `molfile-v3000`, `smiles`, `sdf`, `ket`, `cdxml`, `cml`, `rxn`, `svg`, `mzdraw`) per feature of the document: objects by kind, annotations, data, abbreviations, R labels, wedge kinds, enhanced stereo (when arch-std-12 merges), text runs, highlight, atom maps, charges, isotopes, radicals.
- `fn loss(doc, format) -> Vec<Loss>` with `{what, atoms, bonds, objects}` for each thing of THIS document the format drops or changes; `"out":"loss:FORMAT"` writes `loss/1` (`{"format":"loss/1","target","items":[...]}`, ordered by `what` then smallest id).
- The `expect` arm `{"lossless":"FORMAT"}` of plan-20.
- The table is checked by a property test, not trusted: for random documents and each format, write then read back; every difference found by `diff` (plan-15) must be in `loss`, and every item of `loss` must be a real difference (no false alarm).

Out (do not do here):
- The page (a warning in the save chooser comes with arch-std-17 / arch-web-21 later; list it as not built).
- Fixing any writer: a loss found that a writer could carry goes to the PR as a numbered list for the coordinator.

## CLI twin (rule 15)
- `{"import":...,"out":"loss:molfile"}`; `{"expect":[{"lossless":"molfile"}],"report":true}`.
- Native replay test with the page's bytes for the parity cases.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md`, `docs/mzdraw-formats.md` (what each format holds), `docs/plan/backlog.md` §9.
- Every writer: `io.rs`, `ket.rs`, `cdxml.rs`, `cml.rs`, `rxn.rs`, `svg.rs`; `diff.rs` of plan-15.

## Agreement needed before the PR
No `draw_edit/1` op. One row each for the output `loss:FORMAT` and the `expect` arm `lossless` in the `request` section of plan-01's table (D13).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-22-loss-manifest` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Table and `loss`.
2. Round-trip property test.
3. Output, `expect` arm, doc section (the table rendered in `docs/mzdraw-formats.md`, compared with `loss.rs` by a test).

## Tests to add
- Property: round trip difference ⊆ `loss` and `loss` ⊆ round trip difference, for every format, over random documents.
- An arrow in a molfile, an annotation in SMILES, a wavy bond in V2000: each listed with its id.
- Parity cases; native replay test.
- **Fails without the change**: the property cannot compile before `loss.rs`; a table that forgets objects for `smiles` fails it.

## Independent proof
Correctness must not rest on our own claims. plan-03's tool reads our files with RDKit and Open Babel; for its set, every difference it finds in InChI must be explained by an item of `loss` (a line added to its report). If plan-03 is not merged, say so and leave the check as a numbered follow-up. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date.

## Kill test
If the round-trip property finds that no format loses anything an agent would care about beyond what `docs/mzdraw-formats.md` already says in words, ship only the test and the doc table and drop the output (say so in the PR).

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
