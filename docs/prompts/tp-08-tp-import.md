# tp-08: import a TP list and number the TPs

Proposed by `docs/plan/backlog.md` §11 (row 88) and `docs/study/tp-draw.md` (verdict V6, idea 6 "live-linked SI table", part 1). Size M. Depends on: tp-01, tp-05 (layout after import), plan-20. tp-07 is optional: when it is merged, the import also labels the arrows. Wave: after tp-05.

## Confirm message
Send to the coordinator COO-MZDRAW (session `session_01Ly8Apn8UoWuqCEhqt2snjF`; use `send_message` with that session id), first thing:
> DEV tp-08-tp-import: received, starting on branch `claude/tp-08-tp-import`.

## Role
Model: Sonnet, effort medium.
DEV chat of mzDraw (Rust molecule editor compiled to wasm, `Freeesta/mzDraw`). You write code, tests and the docs of this slice only. A suspect-screening result, or an agent, has a list: SMILES, name, parent, m/z, RT, level. This slice turns that list into the drawing in one step, and numbers the TPs. The table that comes back out is tp-09.

**Contract (COO 16, 2026-11-21, D13).** The request fields and outputs below surface through `mzlab-cli mol draw`. Each needs ONE row in the `request` section of plan-01's table (`schema.rs`); COO 16 reviews that table, not each PR, as long as no `draw_edit/1` op is added. A false postcondition is exit 1 (a verdict) with the `draw-result/1` document on stdout; errors and progress go to stderr; the exit codes stay mzLab's table 0..8, no new code. `draw-result/1` as plan-20 defines it is the source of truth: `mzlab-cli` follows it.

## Scope
In:
- **Module.** A new module `crates/mzdraw/src/tp_list.rs` (its own zone) holds the import and the numbering.
- **Data per TP**, kept in the structure's existing SDF data (`atom.data`; no new op), under fixed names:
  - `TP_ID`: the shown name, e.g. `TP3`;
  - `MZ`: the observed m/z, a number written as text;
  - `RT`: minutes;
  - `ADDUCT`: one of the adduct table of arch-web-17, e.g. `[M+H]+`;
  - `AREA`: optional, never drawn.

  The confidence level stays in `ann` (built). These names are reserved, like the four `MZLAB_*` fields.
- **Import, `tp:import`** (one undo step).
  - **Rows.** Each row is `{smiles, name?, parent?, mz?, rt?, adduct?, level?}`, given as JSON, or as CSV with the header `smiles,name,parent,mz,rt,adduct,level` (comma-separated, RFC 4180 quoting). `parent` is the `name` of another row; an empty `parent` makes a root.
  - **What each row becomes.** A structure through the same door as a SMILES (coordinates computed), with its data fields and annotation. Each `parent` link becomes an anchored arrow (tp-01).
  - **Same undo step.** `scheme:orient` (plan-14) runs on each parent → TP pair when plan-14 exists (backlog E28), then `pathway:tree` (tp-05), then `pathway:label` (tp-07) when it exists.
- **Bounds.** At most 10,000 rows and 1 MB.
- **Row failures** (a bad SMILES, an unknown parent, a cycle of parents, a bad number) are never dropped silently.
  - The import is atomic, as plan-20 is: if any row fails, nothing is applied and no `out` is written. The answer is exit 1, with `draw-result/1` `failed.stage` `command` and a new key `failed.rows`, `[{"line":n,"key":"error.draw2.tpRow..."}]` listing **every** failing row in line order, so one run reports all of them (backlog E29, D31). There is no partial import.
- **Numbering, `tp:number`** (one undo step, `atom.data`). It sets `TP_ID` = `TP1`, `TP2`, ... in the order of `pathway.number`:
  - `layout` (default): by layer, then by position in the layer;
  - `mz`;
  - `rt`.

  A root (the parent) keeps its name. A name a person typed (anything that is not `TP<n>`) is kept unless `"force":true`.
- **Shown name.** `TP_ID` is drawn under the structure, next to the level (`TP3 · L2b`), in the annotation label slot (scene zone).

Out (do not do here):
- The table writers and the SDF round trip (tp-09); a table drawn on the canvas (later).
- `AREA` drawn as arrow width or dash (dropped in the study, V4).

## CLI twin (rule 15)
- `{"commands":[{"command":"tp:import","args":{"csv":"smiles,name,parent,mz\nCCO,P,,47.0491\n..."}}],"out":"mzdraw"}`. The CSV can stream on stdin as the replay request's `-` input. `"dry_run":true` validates the rows and returns only `draw-result/1`.
- `{"commands":[{"command":"tp:number","args":{"force":false}}]}`.
- Request rows (plan-01 `request` section, D13):
  - the commands `tp:import` (args `csv`, `rows`) and `tp:number` (arg `force`);
  - the setting `pathway.number`;
  - the `draw-result/1` key `failed.rows`;
  - the reserved data names.

## Context (read first)
- `CLAUDE.md`, `docs/mzdraw.md` ("Built state": SDF data, annotations), `docs/mzdraw-annotations.md`, `docs/plan/backlog.md` (sections 0, 4, 5, 9), `docs/study/tp-draw.md` (§1 T4, the S66 columns).
- `io.rs` (SMILES door, SDF data), `ann.rs`, `outcome.rs` (plan-20), tp-01, tp-05, tp-07.

## Agreement needed before the PR
No `draw_edit/1` op or field. Request rows only (D13). Tell the coordinator in the PR that the reserved data names (`TP_ID`, `MZ`, `RT`, `ADDUCT`, `AREA`) are a document convention, so that mzLab's CLI reads the same names.

## Rules (from `CLAUDE.md`, binding)
- English only everywhere. All logic is in Rust; `web/*.js` only forwards events and paints, and decides nothing. No new crate or npm package unless the PR states why.
- Every new operation of the log `draw_edit/1` has an exact inverse and is added to the property test "apply then undo returns the identical document" (`crates/mzdraw/tests/ops.rs`). The page only emits commands: what the page does, the replay request (`crates/mzdraw/src/replay.rs`, the route of `mzlab-cli mol draw`) does with the same bytes (`tests/parity.mjs`).
- Synthetic data only in tests and examples (structures you write yourself). Never skip, disable or loosen a test.
- Branch `claude/tp-08-tp-import` from `origin/main`; never push to `main`, never rebase, never force-push. Hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR: merge `origin/main` only on a real conflict, rerun the acceptance commands, push. Do not edit the e2e step count (it is counted from the file now). Open the PR ready for review and never merge it: the coordinator merges.
- Stay in your zone (`docs/plan/backlog.md` §5). Append-only lines (Built state bullet, `measured`, i18n keys, log cases, `mod` lines, one match arm) may be added next to another chat.
- Engines of mzLab are called only through `crates/mzdraw-wasm/src/glue.rs` (plan-04), never from `lib.rs` directly, and never re-implemented here. `mzdraw` itself depends on `mzlab-mol`, `libm`, `serde`, `serde_json` only.
- No new literal of the product name in logic or user text: use `mzdraw::NAME` and the i18n key `draw2.name` (the name will change).
- UI changes come with screenshots at deviceScaleFactor 2 in the PR. Keep touch, pen and keyboard in the e2e tests.
- Keep the "Built state" of `docs/mzdraw.md` true (one bullet for this slice, "Not built" said plainly). Update `measured` in `tools/wasm-cap.toml` when the module moves; `cap` (4,000,000 B) is the owner's.
- A source behind a login or paywall is "not read", never guessed; copy no text, code, test file or artwork of another program or document; never claim fidelity to a product; never claim novelty without a prior-art link; patent clearance is "unverified".

## Work
1. CSV reader (bounded, quoted fields) and JSON rows; import into ops; the `failed.rows` report.
2. `tp:number`; the name under the structure.
3. Goldens: a synthetic 12-row list (a parent, two generations, one convergent TP).

## Tests to add
- Import of the 12 rows gives the golden document. A list with a bad SMILES, an unknown parent and a cycle of parents is refused as a whole, and `failed.rows` names all three lines with their keys; nothing is written.
- `tp:number` in each order. A typed name survives, and `force` replaces it.
- Undo of `tp:import` and of `tp:number` returns the identical document.
- e2e: paste the CSV in the Add field (keyboard); the same CSV through the file button (touch); the SVG equals the golden.
- Screenshots at deviceScaleFactor 2 in the PR: the imported 12-row pathway with its names and levels.
- **Fails without the change**: `tp:import` is an unknown command on `main`.

## Independent proof
Correctness must not rest on our own claims. `tools/fidelity/tpimport.py` reads the CSV with Python's `csv` module and RDKit, and checks that each of our structures (by `TP_ID`) has the canonical SMILES of its row. It also checks that the parent links equal our anchored arrows. Reference programs and data are fetched at run time by a script under `tools/`, never committed (rule 4: synthetic data only in tests); what is committed is the reference output for our synthetic inputs, or the success rate, with the program name, version and date. If a reference cannot be installed in the container, say so in the PR and leave the script runnable.

## Kill test
Time the import of a synthetic 200-row list natively. If it takes more than 2 s in release, or the drawn pathway breaks any tp-05 property, stop and report. A slow import is not shipped behind a progress bar.

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

