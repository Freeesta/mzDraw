# Brand: the product name in one place

"mzDraw" is a codename; the product will be renamed before the first release, so nothing depends on the literal.

## What is built
- `brand.toml` (root): product name, slug, crate prefix, component tag, CSS prefix, app id, the ids of the document (`mzdraw/1`), of the settings (`mzdraw-settings/1`), of the command log (`draw_edit/1`) and of the page-module bridge (`draw-bridge/1`), the first document id (`first_document`), `legacy_ids`, repository, Pages URL and base path, citation title.
- `crates/mzdraw/src/brand.rs`: the same values as constants (`brand::NAME`, `SLUG`, `DOCUMENT_ID`, ...), `slugs()` (the slug in use, then every legacy one) and the version rule of the document and settings ids. `mzdraw::NAME` is `brand::NAME`.
- Readers accept every slug of `legacy_ids`: the version of the document and settings ids, the PNG `iTXt` keyword, the PDF attachment names, the SVG `metadata` id the `out` kind `<slug>` of `save` and `clipboard`, and `export.default` of a settings file or of `--set export.default=<slug>` (kept as the current slug); a legacy document is read and written again under the current id. Writers use the current slug only. `legacy_ids` is empty while the codename is the name.
- The page: `web/names.js` (`NAMES`, a module) and `web/appname.js` (`window.APP_NAME`, a classic script read by `i18n.js`) are written from `brand.rs` and carry no literal of their own; the texts of the catalog say `{app}`; the autosave key, the settings file, the clipboard MIME type, the download name, the window global and the module path read `NAMES`; the autosave, settings and clipboard of an earlier name are read from `NAMES.autosaveOld`, `settingsOld` and `clipOld`.
- Two ids do not follow the name, on purpose. `bridge` (`draw-bridge/1`) is what `version()` of the wasm module answers and the page checks: a contract between two files of one build, so it is its own constant like `draw_edit/1` (it was the document id before). `first_document` (`mzdraw/1`) is history: the edit schema names it as `since` of the operations that existed in the first version, so `out: schema` and `edit-schema.json` keep their bytes under any name; an operation added later says `brand::DOCUMENT_ID`, the version of the day.
- `appname.js` is a classic script in the head that also sets `document.title`, so the tab is never blank before `i18n.js` runs.
- The element id of the document export button is `d2-ex-doc` (it was named after the product).
- Written files carry the slug as before (CML namespace prefix and URL from `brand::SLUG` and `PAGES_URL`).

## Tests
- `crates/mzdraw/tests/brand.rs`: `brand.toml` equals `brand.rs` key by key; the ids follow the slug; the crate manifests, `CITATION.cff` (title, URLs, version) and the CSS tokens follow `brand.toml`; `names.js` and `appname.js` equal what `brand.rs` writes (`MZDRAW_BLESS=1 cargo test -p mzdraw --test brand` rewrites them); `brand_no_literals` fails when the codename is written in a string of the Rust sources of the crates or in a text or string of `web/` (allow-list `ALLOWED`, each entry with its reason, stale entries fail); the scanner has its own test.
- Unit tests in `brand.rs` and `png.rs` run as if one earlier name existed (`#[cfg(test)]` `LEGACY_IDS = ["zzzzzz"]`) and prove that every reader above accepts it. `crates/mzdraw/tests/legacy/` holds real files of that earlier name (document, settings with `export.default`, SVG, PNG, PDF): each is opened through `import_bytes`, and what is written back carries the current name and no trace of the old one. The same file proves the generated `names.js` lists the earlier names (`autosaveOld`, `settingsOld`, `clipOld`) when there are some.
- `brand_no_literals` also scans every `web/**/*.html`, `web/**/*.json` and `web/**/*.css`. Rust comments are not scanned: they name the product in hundreds of places and a rename leaves them to the rename tool.

## Not built
- Not covered by `brand_no_literals`: Rust identifiers and paths (the crate names), the manifests, the tests, `tools/`, `docs/` and file names (`web/mzdraw.js`, `web/mzdraw.css`, `web/lang/mzdraw.js`). The rename tool moves them.
- The key `mzdraw` of the schema JSON (`out: schema`, per operation) stays: it is a stable key and its rename changes the bytes of a golden.
