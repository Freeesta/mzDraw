# Pin bump 3 of `mzlab-mol` and `mzlab-feat` (pin-bump-3), built

(root `Cargo.toml` and `Cargo.lock`, one test, and this note; no code of the editor changed.)

## What moved

Both git dependencies moved together (rule 9), from `4529765d63d1eca2ac2d3fd19808617cc39710a1` to `8bcfdc3658b7abea31c135f7b0c0327b0175d1e9` (merge of mzLab #3694; it includes #3690 `stereo_sites` per piece and #3691 canonical linear for tied pieces). In `Cargo.lock` only the git sources of the mzLab packages changed. `wasm-bindgen` is untouched.

## Goldens

No golden file changed: `git status` shows no file of `tests/data/` touched and no `MZDRAW_BLESS=1` run was needed. Parity wasm == native passes.

## What the new `cip/1` does

`cip/1` now works per connected piece, tests as stereo sites only the elements the drawing defines, and refuses a piece past its own work limit (8,000,000 units per stage) with `error.cip.tooComplex` (params `stage`, `atoms`, `sites`, `work`, `limit`; CLI exit 1). Not silent in mzDraw: the glue turns any engine error into `Err`, and `cip::label` counts it `failed`, so it is the `lost` item `draw2.export.cipFailed` (and the page's note). No new key was added: the refusal is already a lost item. The glue does not read the engine's params.

## The per-piece workaround of `cip.rs`: kept

Measured with the real engine (release): records of 8 and 40 separate copies, and one chain of 2, 3 and 4 joined copies (up to 20 open sites), all with open (undefined) sites, cost between 0.2 and 2.3 ms given whole to the engine, with no label (the engine labels only defined elements). So open sites are no longer exponential for the engine. But no test could show a joined piece with many DEFINED sites over the engine's own limit being reported while within the caps, so the caps (16 open sites, 500 atoms, per connected piece) stay, and every test stays. The engine now also limits itself; the two limits stack. A new test, `the_engine_alone_is_cheap_on_open_sites_and_the_caps_stay` (`crates/mzdraw-wasm/tests/cip.rs`), pins the measurement (CPU-time ratio of 40 to 8 copies given whole, and the joined piece still capped).

## Proof

fmt, clippy (`-D warnings`), `cargo test --locked`, `tools/build_site.sh` (build_site ok, under the cap), the `replay` example and `tests/parity.mjs`, `tests/e2e.mjs`, `tests/velocity.mjs --check`, `tools/queue_check.sh`.

## Not built

Relaxing or removing the caps; reading the engine's `stage`/`atoms` params in the lost item; a test with a joined piece of many defined sites over the engine's limit.
