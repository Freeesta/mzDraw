# Pin bump of `mzlab-mol` and `mzlab-feat` (pin-bump), built

(root `Cargo.toml` and `Cargo.lock`, `crates/mzdraw/tests/tp_import_equal.rs`, `crates/mzdraw-wasm/tests/props.rs`, `tests/data/log/struct-props.json`; no code of the editor changed.)

## What moved

Both git dependencies moved together (rule 9), from `ccd4680fc35b4ca6029cfead3a8e97f82042fdf7` to `3783626345c7b79487218eb8242310eb2d5d68db`, the merge of mzLab #3662 on its `main`. In `Cargo.lock` only the git sources of the mzLab packages changed; `wasm-bindgen` is as pinned. The pin stops at this commit on purpose: the later identifier rename of mzLab is a pin PR of its own.

What the new pin brings from mzLab:

- #3661: `Mol::canonical()` is polynomial in the number of equal components (the doubling per copy is gone; same bytes, same signature).
- #3657: V3000 is read in the engines `graph`, `sel`, `cut` and `mech`.
- #3662: `descriptors/1`, the shared function `mzlab_feat::chem::descriptors::descriptors_json` (CLI `mzlab-cli mol measure`). Not used by mzDraw in this PR.

No item of `mzlab-mol` or `mzlab-feat` that mzDraw calls was renamed or removed: nothing else in the code had to follow the pin.

## Golden lines that moved

One line, in `tests/data/log/struct-props.json`, section `values`, case "props: a chain of 1000 carbons ...": `out` was `C1000H2002\nnote: error.draw2.valuesZero\n` and is now `C1000H2002\nnote: error.grafo.tooBig\n`. Cause, in mzLab and not in mzDraw: a structure of 1000 atoms or more is written as V3000; `sel` used to read it as nothing (mass 0, no error), which mzDraw turned into `error.draw2.valuesZero`. Since #3657 `sel` reads it and refuses it itself with its size key `error.grafo.tooBig`. The formula line is unchanged. The line was checked by hand, then written with `MZDRAW_BLESS=1` (through the test of `mzdraw-wasm` that owns this section; `cargo test -p mzdraw --test replay` moves nothing). The `name` text of that case still says "the engine gives a mass of zero"; it was left as it is (the file is not edited by hand) and is a label only.

No canonical SMILES, InChI or molfile text moved. The two tests of `crates/mzdraw-wasm/tests/props.rs` that expected `error.draw2.valuesZero` for a chain of 1000 and of 4900 carbons now expect `error.grafo.tooBig`; they still assert that a keyed error is given and that nothing is written (not loosened). The zero-mass guard in `glue.rs` stays as a defence.

## V3000 bond types 9 and 10

Reported only, not wired in: no handling of V3000 bond types 9 and 10 was found in the source of `mzlab-mol` at this commit.

## Proof

- `tests/tp_import_equal.rs`: one drawing of 16 equal benzenes has its canonical form computed in a small multiple of the time of 8 (a ratio with `support/timing.rs`, no absolute bound). On the old pin the same ratio was about 50 and the test failed; it passes on the new one. The seven earlier tests of the file stay.
- The acceptance list passes: fmt, clippy, `cargo test --locked`, `tools/build_site.sh` (inside the wasm cap), parity wasm == native, e2e in Chromium, `tests/velocity.mjs --check`, `tools/queue_check.sh`.

## Not built

V3000 bond types 9 and 10; `descriptors/1` in the page; the identifier rename of mzLab (a later pin PR).
