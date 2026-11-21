# Rename tool

`tools/rename_product.sh NEW_NAME` renames the product from the values of `brand.toml` (see `brand.md`); it is the second half of "the name lives in one place".

## What it does
- Rewrites every text file of the tree (except the exclusions below; any letter case of the old slug that is not one of the four, such as `mzDRAW`, becomes the slug): the name (`mzDraw` -> `NEW_NAME`), the slug (`mzdraw` -> lower case of the name: crates, Rust paths, ids, formats, file names), the app id (`MZDRAW` -> upper case, also `MZDRAW_BLESS`), the PascalCase slug (`Mzdraw`) and the colour-token prefix (`--mzd-` -> `--<first three letters of the slug>-`, or `--css-prefix`).
- Moves the paths that carry the slug (`crates/mzdraw`, `crates/mzdraw-wasm`, `web/mzdraw.js`, `web/mzdraw.css`, `web/lang/mzdraw.js`, `docs/mzdraw*.md`).
- Keeps the old slug readable: it is appended to `legacy_ids` in `brand.toml` and in `LEGACY_IDS` of `brand.rs` (the older ones stay), so documents, settings, SVG, PNG and PDF files, autosaves and clipboards written under the old name still open; writers use the new one.
- Updates `Cargo.lock` (package names) and runs `cargo update --workspace --offline` and `cargo fmt` (imports are sorted by name) when cargo is there.
- `--dry-run` applies the rename to a temporary copy and prints the files that would change and the diff of `brand.toml`; `--copy-to DIR` writes a renamed copy of the tree and leaves the tree as it is; `--dir DIR` renames another tree. The name is letters and digits. It is refused (exit 2, nothing touched) when it is the name in use, an earlier name (renaming back is the owner's decision), or contains the name in use or an earlier slug in any case (`mzdrawx`, `xmzdraw`). Cargo is required, and a failure of `cargo update --workspace --offline` or `cargo fmt` is fatal (run `cargo fetch --locked` first on a fresh machine).

## What the tests do so that a rename does not break them
- The `id` of an SVG's `title` and `desc` hashes the document under the id of its first version (`FIRST_DOCUMENT_ID`), so the ids in the goldens do not change with the name.
- The fidelity references (`tests/data/fidelity/reference.json`, `maps.json`) hash our files with the product name and slug masked (`tools/fidelity/run.py` and `maps.py` do the same, reading `brand.toml`): the writers put the name in the header of a molfile, an RXN and a CDXML, and a rename must not look like a change of a writer. The hashes were recomputed once for this (each recorded unmasked hash was first reproduced from our files; the new values come from the Rust tests' output and, for `maps.json`, from a replica of the maps test, not from a run of the Python tools); what RDKit and Open Babel read is untouched. `fnv1a(mask(..))` of `run.py` and `fnv1a` of `maps.py` were loaded alone (the modules need Open Babel or run argparse on import) and reproduce all 1420 hashes of `reference.json` and the 20 of `maps.json`, so they agree with the Rust mask; the two tools were not run as a whole (no RDKit or Open Babel here).
- `web/names.js` has its comments above the lines, so it does not depend on the length of a name; tests do not assume a slug length.

## What it does not change
- The repository and the Pages address (`Freeesta/mzDraw`, `/mzDraw/`): they follow the GitHub repository name, which the owner renames. `brand.toml` keeps `repository`, `pages_url` and `pages_base`; change them with the repository.
- Ids that are not made from the name (`draw_edit/1`, `draw-bridge/1`).
- The key of the edit schema (`out: schema`) that says an operation is read by this editor, `"mzdraw":true` (`schema_key` in `brand.toml`, `brand::SCHEMA_KEY`): a stable machine key for agents, kept like the first document id (owner's decision). Its occurrences in the goldens and in the test are protected by the pattern `"<key>":true|false`; the writer and the test read the constant.
- History: a line marked `rename-keep` (`first_document` in `brand.toml` and `FIRST_DOCUMENT_ID` in `brand.rs`, and the `since` column of the edit schema in the docs and the goldens), `docs/prompts`, `docs/input`, `crates/*/tests/legacy` (files of an earlier name) and the coordinator's name `COO-MZDRAW`.
- A line marked `rename-legacy` is the only place `brand.rs` receives the earlier names.

## The old name left behind
`tools/check_old_name_left.sh DIR OLD_SLUG` looks for the old slug in any letter case in every text file and every path of DIR and fails with the places listed, except the keep-set: the build and history directories (`target`, `site`, `node_modules`, `tools/.bin`, `.git`, `docs/prompts`, `docs/input`, `crates/*/tests/legacy`), the lines marked `rename-keep` or `rename-legacy`, the `legacy_ids` line of `brand.toml`, the `*Old` lists of `web/names.js`, `Cargo.lock`, and inside a line the repository and Pages address, `COO-MZDRAW`, the first document id as `since` and the schema key. `tools/test_rename_product.sh COPY OLD_SLUG` runs it on a renamed copy, shows that it is not blind (a leftover planted in a Rust source, a page, a document under other letter cases and in a file name is found, and the copy is clean again without them) and that the tool refuses `mzdrawx`, `xmzdraw`, the current slug plus a letter, an earlier name and the name in use without touching the tree.

## CI
The job `rename` of `.github/workflows/ci.yml` runs the tool on a copy with the name `Zzz` (after `cargo fetch --locked`), checks that the real tree has no change, runs `tools/test_rename_product.sh` on the copy, then runs on the copy `cargo fmt --check`, clippy, `cargo test --locked`, `tools/build_site.sh` (size cap), golden parity wasm == native and `node tests/e2e.mjs`.

## Not proven
- The tool is GNU sed and find only (Linux, as `tools/install_tools.sh`).
- Comments in `brand.toml` lose their column alignment after a rename.
- Text in the docs that names the old product in prose is renamed too (the plan documents and `README.md`); a sentence that was about the history of the codename becomes about the new name.
- It was run with `Zzz`; another name changes the same places, but a name that contains the old slug or an English word the old name stood in (for example "draw") is not tried.
