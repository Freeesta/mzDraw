# Lone pairs in the SVG and the PDF, built (rdkit-compare 1)

Found by the RDKit comparison (`docs/study/rdkit-vs-mzdraw.md` F1): the lone pairs of `atom.set lp` were painted on the canvas, but the SVG and PDF writers skipped the `DISC` primitive of the display list, so no dot reached a file, while `docs/built/curved-arrows.md` said the SVG has them. Now both writers draw it: the SVG as `<circle cx cy r fill>` (`crates/mzdraw/src/svg.rs` `drawn_with`), the PDF as a filled closed curve of four cubic Bezier quarters (`pdf.rs` `disc`). In a file the only dots are the lone pairs: the selection handle, the hover dot and the marks of a proposal are overlays an export does not have. A dot has the colour of its atom (`colour`), else the ink. The PNG is the SVG drawn by the page, so it has them too. The canvas did not change.

Tests: `crates/mzdraw/tests/curve.rs` `the_lone_pairs_are_dots_in_the_svg_and_the_pdf` (two dots per pair at `curve::LP_REACH` from the atom, the same count of filled curves in the PDF, the colour of a coloured atom, no dot without a pair; it fails on the writers before this change); golden cases in `tests/data/log/curve.json` (all formats of a coloured methanol with three pairs, and its PDF), wasm == native in `tests/parity.mjs`. The SVG of the existing case "lone pairs are placed from the valence by a command" now carries its dots (re-blessed).

The SVG 1.1 check (`crates/mzdraw/tests/svg_access.rs` `every_golden_svg_uses_only_svg_1_1`) lists `circle` with `cx`, `cy`, `r` among the elements the writer may use: all SVG 1.1 (basic shapes), so the check still refuses anything outside SVG 1.1.

Not built: the other primitives an export skips (`RING`, `RECT`) are overlay-only today; a check that every primitive of an export scene has a writer is not written.
