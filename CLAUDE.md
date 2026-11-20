# mzDraw: rules for agents

mzDraw is a 2D molecule editor in Rust compiled to wasm, published at https://freeesta.github.io/mzDraw/ (capital D: the path is the repository name and is case-sensitive; the lowercase address answers 404). Plan and built state: `docs/mzdraw.md` (keep its "Built state" section true in every PR).

## Layout
- `crates/mzdraw`: the editor core (document `mzdraw/1`, operations with exact undo, tools, hit-test, display list, SVG, import/export). Depends only on `mzlab-mol`, `libm`, `serde`, `serde_json`.
- `crates/mzdraw-wasm`: the wasm-bindgen bridge (events in, display list out); its feature `mzlab` adds the live structure check `check/1` of mzLab.
- `web/`: the page (site root). `tools/build_site.sh` builds `site/`; `tools/wasm-cap.toml` + `tools/wasm_cap.sh` are the size cap.
- `tests/`: `parity.mjs` (golden parity wasm == native on `tests/data/mzdraw_log.json`), `e2e.mjs` (Playwright, Chromium: mouse, keyboard, touch).

## Rules
1. English only, everywhere in the repository (code, comments, texts, commits, PRs).
2. All logic is in Rust. JavaScript is interface only: it forwards pointer, wheel and key events and paints the display list. No chemistry and no decision in JS.
3. No bundler, no framework, no heavy dependency. The page is plain files; a new crate or npm package needs a reason stated in the PR.
4. Synthetic data only in tests and examples.
5. Never skip, disable or loosen a test. A test that fails is fixed, or the code is.
6. Never push to `main`. Never rebase, never force-push. Merge `main` into a branch only on a real conflict.
7. PRs are opened ready for review and are never merged by their author: the coordinator (COO-DRAW) merges with the full head SHA.
8. The wasm cap (`tools/wasm-cap.toml`) is respected: update `measured` when the size moves; raise `cap` only with a reason in the PR.
9. `mzlab-mol` (and `mzlab-feat` of the `mzlab` feature) are git dependencies pinned to ONE commit of `main` of `Freeesta/mzlab` in the root `Cargo.toml`; move both `rev` together, in a PR of its own, with the golden files re-checked. `wasm-bindgen` stays at the version whose CLI `tools/install_tools.sh` pins.
10. Every function of the page must also be producible from the CLI route of mzLab (`mzlab-cli mol draw`): the page only emits commands, the command log format is the JSON patch language `draw_edit/1` of mzLab, and replaying a log natively gives the same bytes as the page (`tests/parity.mjs`).
11. Every edit operation has an exact inverse: the property test "apply then undo returns the identical document" covers every new operation.
12. Each PR is one small slice with unit tests, golden parity wasm == native, and e2e in Chromium (keep touch/pen and keyboard in the tests). UI changes come with screenshots at deviceScaleFactor 2.
13. Scope out: macromolecules, query features, S-groups other than abbreviations. A file carrying them opens with the unsupported parts listed, never silently dropped.
14. Cite mzDraw / mzLab when published (`CITATION.cff`; keep its `version` equal to the workspace version and add `date-released` and `doi` with the first tagged release).

## Commands
```
cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
tools/build_site.sh                                  # site/ + target/pkg-node, with the size cap
cargo build --locked -p mzdraw --example replay && node tests/parity.mjs
node tests/e2e.mjs                                   # serves site/ itself; `node tests/e2e.mjs <URL>` tests a published site
MZDRAW_BLESS=1 cargo test -p mzdraw --test replay    # rewrites the golden outputs after a WANTED change
```
