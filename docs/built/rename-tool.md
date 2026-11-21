# Rename tool

`tools/rename_product.sh NEW_NAME` renames the product from the values of `brand.toml` (see `brand.md`); it is the second half of "the name lives in one place".

## What it does
- Rewrites every text file of the tree (except the exclusions below): the name (`mzDraw` -> `NEW_NAME`), the slug (`mzdraw` -> lower case of the name: crates, Rust paths, ids, formats, file names), the app id (`MZDRAW` -> upper case, also `MZDRAW_BLESS`), the PascalCase slug (`Mzdraw`) and the colour-token prefix (`--mzd-` -> `--<first three letters of the slug>-`, or `--css-prefix`).
- Moves the paths that carry the slug (`crates/mzdraw`, `crates/mzdraw-wasm`, `web/mzdraw.js`, `web/mzdraw.css`, `web/lang/mzdraw.js`, `docs/mzdraw*.md`).
- Keeps the old slug readable: it is appended to `legacy_ids` in `brand.toml` and in `LEGACY_IDS` of `brand.rs` (the older ones stay), so documents, settings, SVG, PNG and PDF files, autosaves and clipboards written under the old name still open; writers use the new one.
- Updates `Cargo.lock` (package names) and runs `cargo update --workspace --offline` and `cargo fmt` (imports are sorted by name) when cargo is there.
- `--dry-run` applies the rename to a temporary copy and prints the files that would change and the diff of `brand.toml`; `--copy-to DIR` writes a renamed copy of the tree and leaves the tree as it is; `--dir DIR` renames another tree. The name is letters and digits and must not be the name in use or an earlier one.

## What it does not change
- The repository and the Pages address (`Freeesta/mzDraw`, `/mzDraw/`): they follow the GitHub repository name, which the owner renames. `brand.toml` keeps `repository`, `pages_url` and `pages_base`; change them with the repository.
- Ids that are not made from the name (`draw_edit/1`, `draw-bridge/1`).
- History: a line marked `rename-keep` (`first_document` in `brand.toml` and `FIRST_DOCUMENT_ID` in `brand.rs`, and the `since` column of the edit schema in the docs and the goldens), `docs/prompts`, `docs/input`, `crates/*/tests/legacy` (files of an earlier name) and the coordinator's name `COO-MZDRAW`.
- A line marked `rename-legacy` is the only place `brand.rs` receives the earlier names.

## CI
The job `rename` of `.github/workflows/ci.yml` runs the tool on a copy with the name `Zzz`, checks that the real tree has no change, then runs on the copy `cargo fmt --check`, clippy, `cargo test --locked`, `tools/build_site.sh` (size cap), golden parity wasm == native and `node tests/e2e.mjs`.

## Not proven
- The tool is GNU sed and find only (Linux, as `tools/install_tools.sh`).
- Comments in `brand.toml` lose their column alignment after a rename.
- Text in the docs that names the old product in prose is renamed too (the plan documents and `README.md`); a sentence that was about the history of the codename becomes about the new name.
- It was run with `Zzz`; another name changes the same places, but a name that contains the old slug or an English word the old name stood in (for example "draw") is not tried.
