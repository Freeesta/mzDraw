# Backlog follow-ups: the queue and the backlog made true

Docs only: no code and no test changed.

- `docs/plan/queue.md`, table "Follow-ups without a prompt": one row each for the follow-ups of tp-09, tp-11, tp-05b, BRAND, plan-03, plan-16, plan-22, tp-04, the text panel (#99), the CDXML colour round trip, the two flaky timing tests, `tests/feedback.rs`, and the mzLab agreements. Rows that already existed (E18 `scheme:orient-series`, the stereo checks of #46, the freeze with equal molecules, the fidelity row) were not repeated. The InChIKey row now says it is NOT wanted (owner decision).
- Rows whose source I could not find in a built note or in the code say "to be specified" in the note column: the `'` guard of the TP table CSV, the doc drift of tp-09, mzlab-mol `map`, the `ObjSet` scan of the proposal code, the `describe_move` texts, the PFAS level texts (not checked against the primary source), the grapheme limits, the cap of `to_smiles`.
- Rows from the open PR #100 (isotope pattern, lasso values, values dialog) are not added here: that PR adds them. The rows of PR #91 and PR #92 (merged) are removed from the table, and the tp-05b row cites its note on `main`.
- `docs/plan/backlog.md`: the stale status column ("N") of the tables of sections 9.4, 10.3 and 11.1 is removed (the state is in the queue); the `Shift+T` key of tp-11, which is not bound, is removed.
- `CLAUDE.md`: rule 16, the product name lives in `brand.toml` (`crates/mzdraw/tests/brand.rs`, with `brand_no_literals`, enforces it).
