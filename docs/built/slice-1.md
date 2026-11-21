# Slice 1, built

crate `crates/mzdraw` (document `mzdraw/1`, ids, atom/bond add/remove/move/set, ring templates, charge, exact undo/redo, command log, display list, hit-test, select/erase tools, SMILES/molfile/document in, molfile/SVG/document out), bridge `crates/mzdraw-wasm` with the live `check/1` of mzLab, page `web/` (published at the root of https://freeesta.github.io/mzDraw/), size cap `tools/wasm-cap.toml`, unit + property tests (`crates/mzdraw/tests/`), golden parity wasm == native (`tests/parity.mjs`), Playwright e2e of 8 steps with mouse, keyboard and touch (`tests/e2e.mjs`).
