# tp-09: the TP table out (JSON, CSV) and the SDF round trip

Proposed by `docs/plan/backlog.md` §11 (row 89) and `docs/study/tp-draw.md` (verdict V6, part 2). Size S. Depends on: tp-08, tp-07 (deltas), arch-web-17 (masses and adduct m/z through the glue, E6). Wave: after tp-08.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-09-tp-table: received, starting on branch `claude/tp-09-tp-table`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. A TP paper has a scheme and a table, and the SI repeats both; kept apart, they disagree. Here the table is a view of the drawing: one source, two outputs.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- **Module.** A new module `crates/mzdraw/src/tptable.rs` (its own zone).
- **`save("tptable")`** gives `tptable/1` JSON with stable keys. **`save("tptable-csv")`** gives the same rows as CSV.
- **Columns**, in this order:
  - `tp_id`, `parent_id`, `generation` (the tp-05 layer);
  - `smiles` (the isomeric SMILES of `save("smiles")`), `formula` (Hill);
  - `monoisotopic_mass` (4 decimals), `adduct`, `mz_calc` (4 decimals);
  - `mz_obs`, `ppm` (1 decimal, `(obs − calc) / calc × 10^6`), `rt`;
  - `form_diff` (the tp-07 delta), `mass_diff` (4 decimals, signed);
  - `level`, `identifier`.
- **Rows** are ordered as `tp:number`. Empty cells stay empty, never `0`.
- **Masses** and adduct m/z come from arch-web-17 (`selection/1` through the glue, E6). None is computed or typed here.
- **NORMAN-style.** The column names `form_diff` / `mass_diff` and the parent–TP pair rows follow what the NORMAN-SLE TP lists use: S66 has `Form_Diff`, `Mass_Diff` and `ParentOrTP` (study §1 T4, read 2026-11-21). The CSV is called "NORMAN-style", never "NORMAN-ready" or "compliant", because no NORMAN validator was run.
- **SDF round trip.** The SDF out (built) carries the reserved fields of tp-08 per record. Reading that SDF back gives the same data, annotations and table. The arrows are not in an SDF, so a re-read SDF has no pathway: say so in the import notes, listed, not dropped silently.

Out (do not do here):
- A table drawn on the canvas (a later slice, once the export is used).
- Journal "preset compilers": arch-std-16's export profiles cover sizes.
- InChIKey: there is no InChI in `mzlab-mol`. The column is absent, and the missing InChI is listed as a COO 16 question, never faked.

## CLI twin (rule 15)
- `{"import":DOC,"out":"tptable-csv"}`, `{"out":"tptable"}`, `{"out":"sdf"}`. These are deterministic bytes, with parity wasm == native.
- Request rows (plan-01 `request` section, D13): the outputs `tptable` and `tptable-csv`, and the `tptable/1` key table (write it in `docs/schemas/tptable-1.json`).

## Context (read first)
- `CLAUDE.md`, `docs/built/` (the Built state), `docs/plan/backlog.md` (sections 0, 4, 5, 7 E6, 9), `docs/study/tp-draw.md` (§1 T4).
- `io.rs` (SDF), tp-08 (reserved names), tp-07, arch-web-17, `outline.rs`.

## Agreement needed before the PR
No `draw_edit/1` op or field. Request rows only (D13).

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-09-tp-table` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files are gone (your note is your own `docs/built/<slice>.md`, there is no `measured` figure to edit, your golden cases go in your own file `tests/data/log/<area>.json`): merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (your own `docs/built/<slice>.md`, i18n keys, cases in your own file `tests/data/log/<area>.json`, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the Built state true: add your note as `docs/built/<slice>.md` (one file for this slice, "Not built" said plainly). No `measured` edit (the build prints the size; the cap in `tools/wasm-cap.toml` is the guard); `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. `tptable/1` and CSV writers, by hand, with a fixed key order. The ppm and rounding rules (half to even) are written in the docs.
2. SDF round trip of the reserved fields.
3. Goldens of tp-08's 12-row list.

## Tests to add
- Round trip: tp-08's CSV imported, `tptable-csv` exported, the export imported again. The result is the same document (compared canonically, ids aside) and the same CSV bytes.
- Every `form_diff` plus the parent formula equals the TP formula. Every `ppm` is recomputed in the test from `mz_obs` and `mz_calc`.
- SDF out, then in, gives the same data and annotations per structure.
- The download button gives the same bytes as `out` (e2e, keyboard and touch).
- Screenshots at deviceScaleFactor 2 in the PR: the Save list with the two table entries.
- **Fails without the change**: `"out":"tptable-csv"` is an unknown output on `main`.

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/tptable.py` recomputes, with RDKit, for each row from our SMILES:
- the formula and the monoisotopic mass;
- the `[M+H]+` / `[M−H]−` m/z, with the electron mass 5.485799090441e-4 u`*`;
- the formula and mass differences.

The committed reference equals our CSV, with masses within 1e-4.

The electron mass is the CODATA 2022 value as `docs/plan/backlog.md` §1.1 lists it ("partly" verified there; not re-read in this study). The DEV reads https://physics.nist.gov/cgi-bin/cuu/Value?meu and states the date read.

The script also checks that our columns map one to one onto the S66 columns it names (`SMILES`, `MolecularFormula`, `ExactMass`, `Form_Diff`, `Mass_Diff`), from the Zenodo record header read at run time. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Kill test
Ask the owner to paste the CSV into their usual SI table workflow once. If more than two columns must be renamed or recomputed by hand, list them as a follow-up change of the column table before anyone builds a canvas table.

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

