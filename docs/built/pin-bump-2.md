# Pin bump 2 of `mzlab-mol` and `mzlab-feat` (pin-bump-2), built

(root `Cargo.toml` and `Cargo.lock` only; no code of the editor changed in this slice.)

## What moved

Both git dependencies moved together (rule 9), from `3783626345c7b79487218eb8242310eb2d5d68db` to `4529765d63d1eca2ac2d3fd19808617cc39710a1`, the tip of `main` of `Freeesta/mzlab` taken at the start of the work (a merge of mzLab #3675). In `Cargo.lock` only the git sources of the mzLab packages (`mzlab-core`, `mzlab-feat`, `mzlab-mol`, `mzlab-picchi`, `opentfraw`) changed. `wasm-bindgen` is as pinned: the version in `Cargo.lock` is the one `tools/install_tools.sh` reads and checks.

Between the two pins there are 11 mzLab commits. In the crates mzDraw uses, the diff is three one-line changes (`mzlab-feat/src/chem/rules_site.rs`, `mzlab-mol/src/transform.rs`, `mzlab-mol/tests/transform.rs`); nothing that mzDraw calls was renamed or removed.

## Goldens

No golden file changed: `git status` shows only `Cargo.toml` and `Cargo.lock`, and no `MZDRAW_BLESS=1` run was needed. Parity wasm == native: 6460/6460 checks.

## Proof

fmt, clippy (`-D warnings`), `cargo test --locked` (all tests pass), `tools/build_site.sh` (build_site ok, under the cap), the `replay` example and `tests/parity.mjs`, `tests/e2e.mjs` in Chromium (all steps, 0 page errors), `tests/velocity.mjs --check`, `tools/queue_check.sh`.

## Not built

Nothing else: this slice is the pin only.
