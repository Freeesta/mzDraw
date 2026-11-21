# tp-06: the TP table (import a list, number the TPs, export the table)

Proposed by `docs/study/tp-draw.md` (verdict V6, idea 6 "live-linked SI table", reduced: no journal compiler). Size M. Depends on: tp-01, tp-04 (layout after import), tp-05 (deltas), arch-web-17 (masses and adduct m/z through the glue). Wave: after tp-05.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-06 tp-table: received, starting on branch `claude/tp-06-tp-table`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only.

## Why
A TP paper has a scheme and a table (number, name, formula, m/z, RT, Δ, confidence level), and the SI repeats both. Kept apart, they disagree. Here the table is a view of the drawing: one source, two outputs. The same list can come in (from a screening result or from an agent) and become the drawing.

## Scope
In:
- **Data per TP**, in the structure's existing SDF data (`atom.data`, no new op), with fixed names: `TP_ID` (shown name, e.g. `TP3`), `MZ` (observed m/z, number as text), `RT` (minutes), `ADDUCT` (one of the adduct table of arch-web-17, e.g. `[M+H]+`), `AREA` (optional, not drawn). The confidence level stays in `ann` (built). Names are reserved like the four `MZLAB_*` fields.
- **Import** `tp:import` (one undo step): rows `{smiles, name?, parent?, mz?, rt?, adduct?, level?}` as JSON or as CSV with a header row (`smiles,name,parent,mz,rt,adduct,level`, comma-separated, RFC 4180 quoting); `parent` is the `name` of another row (empty = a root). Each row becomes a structure (through the same door as a SMILES, coordinates computed), its data fields and annotation; each `parent` link becomes an anchored arrow (tp-01); then `scheme:tree` (tp-04) and `scheme:label` (tp-05) run in the same step. Rows that fail are listed with their line and key, never dropped silently; nothing is applied when any row fails, unless `"partial":true`.
- **Numbering** `tp:number` (one undo step, `atom.data`): `TP_ID` = `TP1`, `TP2`, ... in the order of `scheme.number` = `layout` (layer, then position in the layer; default) | `mz` | `rt`; the parent (a root) keeps its name. A name a person typed is kept unless `"force":true`. The name is drawn under the structure next to the level (`TP3 · L2b`), in the scene zone's label slot of annotations.
- **Export**: `save("tptable")` gives `tptable/1` JSON with stable keys; `save("tptable-csv")` the same rows as CSV. Columns, in this order: `tp_id`, `parent_id`, `generation` (tp-04 layer), `smiles` (the isomeric SMILES of `save("smiles")`), `formula` (Hill), `monoisotopic_mass` (4 decimals), `adduct`, `mz_calc` (4 decimals), `mz_obs`, `ppm` (1 decimal, `(obs − calc) / calc × 10^6`), `rt`, `form_diff` (tp-05 delta), `mass_diff` (4 decimals, signed), `level`, `identifier`. Rows ordered as `tp:number`. Empty cells stay empty, never `0`.
- The column names `form_diff` / `mass_diff` and the parent–TP pair rows follow what the NORMAN-SLE TP lists use (S66 has `Form_Diff`, `Mass_Diff`, `ParentOrTP`; study §1, read 2026-11-21); the CSV is "NORMAN-style", never called "NORMAN-ready" or "NORMAN-compliant" (no NORMAN validator was run).
- The SDF out (built) carries the same fields per record, so the drawing round-trips through SDF.

Out (do not do here):
- A table drawn on the canvas (a later slice, once the export is used); journal "preset compilers" (arch-std-16 export profiles cover sizes); InChIKey (no InChI in `mzlab-mol`: a COO 16 question, listed, not faked).
- Arrow width or dash from `AREA` (dropped in the study).

## CLI twin (rule 15)
- `{"command":"tp:import","args":{"csv":"smiles,name,parent,mz\nCCO,P,,47.0491\n..."}}` then `{"out":"tptable-csv"}`: a whole scheme and its table from one request, no browser; stdin streaming of the CSV is the replay request's `-` input. `"dry":true` validates the rows and returns the report only.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state": SDF data, annotations), `docs/mzdraw-annotations.md`, `docs/plan/backlog.md` (sections 0, 4 and 5), `docs/study/tp-draw.md` (§1, S66 columns).
- `io.rs` (SMILES door, SDF data), `ann.rs`, arch-web-17 (masses, adducts), tp-01, tp-04, tp-05.

## Agreement needed before the PR
None for ops. The reserved data names (`TP_ID`, `MZ`, `RT`, `ADDUCT`, `AREA`) are a document convention: tell the coordinator in the PR so mzLab's CLI can read the same names.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-06-tp-table` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. CSV reader (bounded: 10,000 rows, 1 MB, quoted fields) and JSON rows; import into ops; failure report.
2. `tp:number`; the name under the structure.
3. `tptable/1` and CSV writers (by hand, fixed key order); ppm and rounding rules written in the docs.

## Tests to add
- Round trip: import a synthetic 12-row list (a parent, two generations, one convergent TP), export `tptable-csv`, import that again: the same document bytes (ids aside, by a canonical comparison) and the same CSV bytes.
- Every `form_diff` + the parent formula = the TP formula; every `ppm` recomputed in the test from `mz_obs` and `mz_calc`.
- A row with a bad SMILES, an unknown parent, a cycle of parents: refused with line and key; `partial` applies the rest.
- Undo of `tp:import` and of `tp:number` returns the identical document.
- e2e: paste a CSV in the Add field (keyboard), then by touch with the file button; the table download equals the golden.
- **Fails without the change**: `tp:import` is an unknown command on `main`.

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/tptable.py` recomputes, with RDKit, the formula, monoisotopic mass, `[M+H]+` / `[M−H]−` m/z (electron mass 5.485799090441e-4 u, CODATA 2022) and the formula and mass differences of each row from our SMILES; the committed reference equals our CSV (masses within 1e-4). The script also checks that our column set maps one to one onto the S66 columns it names (`SMILES`, `MolecularFormula`, `ExactMass`, `Form_Diff`, `Mass_Diff`), read from the Zenodo record header at run time. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

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

