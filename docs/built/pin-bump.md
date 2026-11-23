# Pin bump of `mzlab-mol` and `mzlab-feat` (pin-bump), built

(root `Cargo.toml`, `Cargo.lock`, `crates/mzdraw/tests/tp_import_equal.rs`; no code of the editor changed unless the pin forced it.)

## What moved

DRAFT: the new `rev` is not set yet (waiting for the full SHA of the mzLab `main` commit that holds #3662). Both `rev` move together, from `ccd4680fc35b4ca6029cfead3a8e97f82042fdf7` to the new one.

What the new pin brings from mzLab: #3661 (`Mol::canonical()` is polynomial: no doubling per equal component), #3657 (V3000 read in graph, sel, cut, mech), #3662 (`descriptors/1`, shared function `mzlab_feat::chem::descriptors::descriptors_json`, CLI `mzlab-cli mol measure`).

## Golden lines that moved

To be filled after the run of the acceptance list.

## Proof

- `tests/tp_import_equal.rs`: the canonical form of one drawing of 16 equal benzenes costs a small multiple of the cost of 8 (a ratio, no absolute bound, with `support/timing.rs`). On the old pin this ratio was far above the bound (doubling per copy).
- The seven earlier tests of the file stay.
