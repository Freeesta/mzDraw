# Equal molecules in a TP import (tp-import-equal), built

(`crates/mzdraw/tests/tp_import_equal.rs`; golden case `tests/data/log/tp-import.json` (the last case); e2e steps in `tests/e2e.mjs`; no code of the editor changed.)

## What was asked and what was found

The follow-up of `built/tp-import.md` said that a list with 6 or more equal molecules freezes the page (found by tp-03, `built/tp-derive.md`: "5 identical copies: seconds; 6: about a minute"). **It does not reproduce on the pinned `mzlab` revision (`ccd4680`).** What was run, on `main` at 44c0524, debug build for the native runs and the release site for the others:

- Native `tp:import` of 2 to 24 equal molecules (benzene, ethanol, isobutane, cyclohexane, alanine with a stereo centre): 2 to 17 ms. `tp:number`, `tp:derive` of one of them and a `move` of one atom among 16 equal benzenes (the stereo guard runs on that step): about 1 ms.
- The six engines of the glue (`check`, `cip`, `cut`, `mech`, `hilite`, `sel`) on the imported drawing with 6, 12 and 24 equal molecules: 1 to 3 ms each.
- The `.wasm` in Node: `add` of the CSV, of a dotted SMILES of equal parts, repeated adds, copy and paste up to 40 molecules, duplicate, rotate, flip, save, `pathway-tree`, `scheme-orient`, undo: nothing above 30 ms.
- The page in Chromium, the CSV in the Add field and Enter: 8 benzenes 144 ms, 24 ethanols 81 ms, 12 alanines 55 ms.

So there was no loop to fix and nothing to refuse: the import never compares structures with each other, its work follows the size of the list (bounded already by `tpListSize`, `tpRowSize` and `schemeSize`), and no refusal for equal molecules was added.

## What an import of equal molecules means (fixed by the tests)

Duplicates are legal. Each row keeps its own node, its own `TP_ID`, its own data and its own anchored arrow. Equal children of one parent stand in one column, stacked at an even step, and the stack is symmetric about the parent; the nameless rows are numbered `TP1..` from the top. A chain of equal molecules is a chain (one layer per row); with no parent link at all the structures stay in a column and the nameless rows are numbered in the order of the rows. Two rows that give the same `name` are still `tpRowDup`, whatever their structures.

## A limit of `mzlab-mol`, stated

`Mol::canonical()` of `mzlab-mol` on a whole drawing of equal components **doubles in time with each equal component**. Measured natively, debug build, benzene (one drawing of n equal benzenes):

| copies | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|
| time | 6 ms | 12 ms | 25 ms | 45 ms | 87 ms |

(cyclohexane: 9 ms at 4, 113 ms at 8; ethanol: 1 ms at 4, 9 ms at 8). That is about 1.4 s at 12 copies and 22 s at 16 by the same doubling. This is very likely what tp-03 met. mzDraw does not call it on a whole drawing: `Mol::canonical()` is taken only in `stereo_guard.rs` (the signature of one piece, or of the neighbourhood of one stereo element) and `canonical_order` only in `trasf.rs` (the parent and the product of one record). A follow-up row of `docs/plan/queue.md` asks mzLab (COO 16) to make it linear in the number of equal components.

## Proof

- `tests/tp_import_equal.rs` (7): 2 to 24 equal molecules for three structures (one node, one arrow, one number each, one column, an even and symmetric stack, `TP1..` from the top); equal molecules with no parent; a chain of 3, 8 and 24 equal molecules; equal and different molecules mixed; a property test on 60 seeded lists of 2 to 24 rows (deterministic bytes in a second editor, one node and one arrow per link, undo is the identical document, redo, a second import undoes alone: rule 11); the log replays to the same bytes; and the **pin of the callers of the canonical form**: it reads the sources of `mzdraw` and `mzdraw-wasm` and fails when a line of code that calls `.canonical()`, `canonical_order(` or `canonical_text(` appears anywhere else or more often than now, so that a new caller, which may meet equal molecules on a whole drawing, is looked at first. No test measures time: they assert completion and exact results.
- Golden parity wasm == native: the case "eight equal molecules" in `tests/data/log/tp-import.json` (all eleven outputs).
- e2e (Chromium): eight equal molecules imported by mouse (the Add button), by keyboard (Enter in the Add field) and by touch (a tablet: Open a file tapped); each ends at the golden document and picture inside the wait of the step; one undo step; select all and undo still answer after. No pen step: the import adds no pen gesture. Screenshot at deviceScaleFactor 2: `docs/shots/tp-import-equal.png` when the e2e is run with `MZDRAW_SHOTS`.

## Not built

A refusal of equal rows (nothing needs it). A faster `Mol::canonical()` (mzLab's, queue row).
