# plan-03: format fidelity checked by RDKit, Open Babel and InChI

Proposed by `docs/plan/backlog.md` (row 14; Gemini better question 3; independent proof for every format slice). Size M. Wave 1. Depends on: none (backlog Q5 on fetched corpora).

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV plan-03 fidelity-ref: received, starting on branch `claude/plan-03-fidelity-ref`.

## Role
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. This slice writes a harness and reference files; it changes no reader or writer. Its numbers are the independent proof the format slices cite.

## Scope
In:
- `tests/data/fidelity/set.smi`: 200 synthetic structures written by hand or by a generator script kept in `tools/fidelity/gen.py` with a fixed seed: rings, fused rings, charges, isotopes, radicals, tetrahedral centres with wedges, E/Z double bonds, abbreviations, R groups, two-piece salts.
- `tools/fidelity/run.py`: for each structure and each format we write (molfile V2000, V3000, SDF, SMILES, KET, CDXML, CML, RXN where it applies), takes our file (from the native `replay` example), reads it with RDKit and with Open Babel, and writes the standard InChI and InChIKey (InChI 1.07.5 through RDKit's bundled InChI and the `inchi-1` binary when present) next to the InChI of the input SMILES. It writes `tests/data/fidelity/reference.json`: per structure and format, `{"inchikey_in","inchikey_out_rdkit","inchikey_out_obabel","lost"}` plus the program versions and the date.
- The reverse direction: RDKit and Open Babel write molfile, SDF and SMILES for the same set; we read them (`Editor::open`) and write the InChIKey of our record (through `mzlab-mol`'s SMILES out, then RDKit in the tool).
- `crates/mzdraw/tests/fidelity.rs`: reads `reference.json` and checks, without RDKit, that our current outputs are byte-identical to the ones the reference was made from (a hash per file), so a writer change forces a rerun of the tool.
- A summary table in `docs/mzdraw-formats.md` (success rate per format and direction, the versions, the date).

Out (do not do here):
- Fixing a reader or writer (each failure found goes to the PR as a numbered list for the coordinator to dispatch).
- Committing any third-party file or dataset.

## CLI twin (rule 15)
- `python3 tools/fidelity/run.py --replay target/debug/examples/replay > tests/data/fidelity/reference.json` (exit 0; 1 on a tool error; diagnostics on stderr).
- The Rust test runs with no Python.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state" is what exists), `docs/plan/backlog.md` (sections 0, 4 and 5: findings, this row, your zone).
- `crates/mzdraw/src/editor.rs` (`save`), the writers (`ket.rs`, `cdxml.rs`, `cml.rs`, `rxn.rs`), `docs/mzdraw-formats.md`.
- RDKit (BSD-3, pip `rdkit`), Open Babel 3.1.1 (GPL-2.0, run as a separate program only, never linked), InChI 1.07.5 (MIT, https://github.com/IUPAC-InChI/InChI/releases).

## Agreement needed before the PR
None: this slice adds no `draw_edit/1` operation or field and changes nothing in `mzlab-mol` / `mzlab-feat`. If you find you need one, stop and ask the coordinator (it asks COO 16).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/plan-03-fidelity-ref` from `origin/main`; never push to `main`, never rebase, never force-push; merge `main` only on a real conflict. Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. Write the set and the generator.
2. Write `run.py` with pinned versions; record them in the JSON.
3. Run it; commit `reference.json`; list every structure whose InChIKey changes, by format, in the PR.
4. Write the Rust hash test and the docs table.

## Tests to add
- `fidelity.rs`: the outputs of the 200 structures match the hashes in `reference.json`.
- A deliberate change to one writer (in a test-only copy) must make `fidelity.rs` fail with the structure id and format in the message.
- **Fails without the change**: before this slice there is no check that a writer change alters InChI identity; the hash test fails on any writer byte change until the reference is rerun.

## Independent proof
Correctness must not rest on our own claims. The InChIKey is computed by RDKit and Open Babel, programs independent of mzLab, on our files, and compared with the InChIKey of the input. InChI is the IUPAC identifier and its reference implementation is the InChI Trust's. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

