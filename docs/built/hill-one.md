# One Hill formula writer (hill-one), built

(`crates/mzdraw/src/outline.rs`, `pathcheck.rs`, `tptable.rs`, one test in `crates/mzdraw-wasm/tests/pathway_draw.rs`; no engine change, no golden moved.)

## What changed

The text of a formula from counts already in Hill order was written by three copies (`outline.rs`, `pathcheck.rs`, `tptable.rs`). It is now one function, `outline::hill_text`: each element, then its count when above one, a zero count left out. `outline::hill` (sort in Hill order, then the charge) calls it; the pathway check and the TP table call it directly. Output is identical on every input the three copies saw, so no golden moved.

## The check already uses the shared engine

`pathway-check/1` holds no mass or formula-mass code. The monoisotopic mass of a node comes from the `sel` engine of mzLab through `glue::measure` (the `values::Measurer` the bridge hands in); the formulas are element counts of the drawing and the label deltas are read with `mzlab_mol::delta`. `descriptors/1` (mzLab) gives the average weight, logP-type values and `analysis/1`, not the monoisotopic mass of `sel`, and lives in `mzlab-feat` (feature `check`), which the core crate does not depend on; so the check is not moved onto it.

## Test

`the_node_mass_of_the_check_is_the_mass_of_the_shared_engine_sel_for_the_same_molfile`: for three synthetic structures, the `monoisotopic_mass` of the check node equals `sel_json` of the same molfile rounded to 4 decimals, and its formula equals the formula of `sel`. The existing pathway tests are unchanged.

## PNG route

Not built here. mzLab already has `mol draw` (CDXML, SVG, PNG): the PNG route of mzDraw should reuse it, not add a renderer. The rdkit-compare chat owns that follow-up row.

## Not built

A PNG renderer; moving the check onto `descriptors/1`.
