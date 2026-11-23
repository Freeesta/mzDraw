# `pathway/1`: a pathway drawn from a declarative model, with checks (pathway-draw), built

(`crates/mzdraw/src/pathmodel.rs` (the model, `pathway:build`), `pathcheck.rs` (`pathway-check/1`), `commands.rs`, `editor.rs` (`import_model`, `save("pathway-check")`), `replay.rs` (`Reply::check_failed`, the `args` of the check), `scene.rs` (the status next to the name), `schema.rs` (two request rows), `web/lang/mzdraw.js` (13 error texts); tests `crates/mzdraw/tests/pathway_draw.rs` (22), `crates/mzdraw-wasm/tests/pathway_draw.rs` (8, the real engine); golden cases `tests/data/log/pathway-draw.json`; the synthetic model `tests/data/pathway-draw/oxidation.json`; three e2e steps in `tests/e2e.mjs`; screenshots at deviceScaleFactor 2 `docs/shots/pathway-draw.png`, `pathway-draw-tablet.png`.) No `draw_edit/1` operation or field is new: the model becomes the plain operations of `tp:import` plus `atom.data` and `obj.set`, so no property test of inverses is added (the existing one covers the operations) and a log of the result replays to the same bytes natively and in the `.wasm`.

The request came from an agent that wrote a CDXML writer by hand and pasted RDKit images into matplotlib. What was already in mzDraw, and what was not:

## Gap table: wanted against built

| wanted | before this slice | now |
|---|---|---|
| A declarative JSON model `pathway/1` (nodes: id, smiles and/or formula, label, ion, rt, status, generation, branch; edges: from, to, label) | no: a CSV/JSON *list* of rows (`tp:import`, tp-08) had smiles, name, parent, mz, rt, adduct, level, no edge labels, no status, no generation, no branch | **built**: `pathway:build` (`args` `{"model":...}`), also the Add field and the file button of the page. A node with a formula alone (no SMILES) is **refused with its own key** (`pathNodeSmiles`): there is no structure to draw (follow-up) |
| Formula and monoisotopic mass of each SMILES | formula by `Document::to_mol`, masses by the engine `sel` of mzLab through `values::Measurer` (tp-09, arch-web-17) | reused; shown by `pathway-check/1` per node |
| Delta-m on each edge against the edge label, flagged on a mismatch | the formula delta is *written* on each arrow (`pathway:label`, tp-07); nothing compared a label that was typed | **built**: `pathway.edgeMismatch` (error). A formula label (`+O`, `-CH2`, `+C6H8O6`, `-Cl+OH`, the isomer word) is compared on element counts, so exactly; a signed number (`+15.9949`) is compared with the delta of the monoisotopic masses within `da` (default 0.005 Da, needs the engine) |
| Expected ions per polarity, next to the observed ion | m/z of the adducts of `display.adducts` under a structure (arch-web-17); `mz_calc` and `ppm` of a typed adduct in the TP table | **built** in the check: `ions` (the expected `[M+H]+` / `[M-H]-` per polarity), `observed` (`adduct`, `mz`, `expected`, `ppm`), `pathway.ionMismatch` (error, beyond `ppm`, default 10), `pathway.ionUnmatched` (warn: no adduct typed and none fits), `pathway.ionNotComputed` (info). Drawn under a structure by the existing `display.values=mz` |
| Automatic layout: columns per generation, rows per branch, no overlap | the tidy tree of `pathway:tree` (tp-05, tp-05b): a column per layer, a subtree contiguous (a branch is a block of rows), clear of boxes by property tests | reused; the check reports the layer against the stated `generation` (`pathway.generationMismatch`, warn) and the arrows that meet another box (`pathway.crossing`, warn); the test of this slice checks one x per generation and no overlap of boxes on the model. `branch` is kept as data and shown by the check; it does **not** reorder the layout (the order is `pathway.order`) |
| Status observed / tentative / not detected | no | **built**: data `STATUS` (`observed` is the default and not written), drawn next to the name as `Name (tentative)`, `Name (not detected)` by the painter of the editor (the page, the SVG and the PDF are painted from one scene; tested on the SVG) |
| SVG (vector, structures included) | yes (`out svg`, the document embedded) | reused; **re-read test**: the SVG opened again is the same document (bytes of `mzdraw` and of the check) |
| PDF | yes (arch-std-18, vector, the document attached) | reused; **re-read test**: the PDF opened again is the same document |
| CDXML | yes (slice-3d) | reused; **re-read test**: same structures (SMILES of each) and, once `pathway:anchor` ran on each arrow, the same graph, and the labels are back as texts. CDXML keeps atoms, bonds, arrows and the text above an arrow, **not** the anchors or the data fields: `loss:cdxml` says so (`arrow.anchor`, `arrow.text`, `data`) |
| `.ket` (Ketcher) | yes (slice-3c) | reused; same re-read test and the same loss as CDXML |
| PNG | the page rasterises the SVG of the editor on a canvas and writes the document into the file (`png.rs`, e2e); no native rasteriser | **not built natively**: a CLI PNG needs a rasteriser in Rust (a new dependency or a hand-written one), see the follow-ups. `out png` natively is still `error.draw2.request` |
| CLI-first: command route, `--json` summary, `--check`, `--dry-run`, deterministic bytes | commands in `commands`, `dry_run`, `report`, `expect`, `draw-result/1` | **built** (below) |
| `--from-dig RUN/dig.json` | no | **not in this slice** (the request says so): a row in `docs/plan/queue.md` |

## The model

```json
{"format":"pathway/1",
 "nodes":[{"id":"P","smiles":"Cc1ccccc1","label":"Toluene","ion":"[M+H]+","mz":93.0699,"rt":6.1,
           "status":"observed","generation":0,"branch":"A","formula":"C7H8"}],
 "edges":[{"from":"P","to":"M1","label":"+O"}]}
```

`id` and `smiles` are required; 1 to 500 nodes, up to 1000 edges, no other key anywhere. A number is a JSON number or a text that reads as one (a text is kept as typed; a JSON number is written the same whatever wrote the JSON: `4.0` and `4` are one). `label` is the shown name (else the id), and must be unique among the shown names; `ion` is an adduct word (`tp_list::adduct_ok`); `mz` the observed m/z; `formula` the formula the model states (checked, see below); `status` `observed`, `tentative` or `not_detected`; `generation` a whole number 0 to 1000.

**What it makes (one undo step).** The nodes are the rows of `tp:import` (structures, one anchored arrow per edge, `scheme:orient`, the layout of `pathway:tree`, the formula delta on each arrow); on the same copy of the drawing the data of each structure are set (`TP_ID` = label else id, then `MZ`, `RT`, `ADDUCT`, then `NODE_ID`, `FORMULA`, `STATUS`, `GENERATION`, `BRANCH`) and the text of each edge that has a `label` (a label of the model replaces the delta that was written; an edge without one keeps the computed delta). A drawing that already has structures keeps them: the new ones go to the right.

**All or nothing, every problem listed.** `error.draw2.pathModel` (not an object of this format, a key that is not allowed, no node, above the caps; under `report` a verdict, exit 1, as every refused command); `error.draw2.pathRows` with `failed.rows` `[{"line","key"}]`: the line is the place in `nodes`, then in `edges` (the edge `j` has the line `nodes + j`), the keys `pathNodeShape`, `pathNodeId`, `pathNodeDup`, `pathNodeSmiles`, `pathNodeFormula`, `pathNodeStatus`, `pathNodeNumber`, `pathEdgeShape`, `pathEdgeEnd`, `pathEdgeLoop`, `pathEdgeDup`, then the row keys of `tp:import` (`tpRowSmiles`, `tpRowAdduct`, `tpRowCycle`, ...) on the line of the node, and the layout's own keys (`schemeSize`, `schemeCycle`). All have texts in `web/lang/mzdraw.js` (`tests/text_keys.rs`).

## The checks: `{"out":"pathway-check"}` (`pathway-check/1`)

It reads the **drawing**, not the model: any drawing with anchored arrows (from `tp:import`, by hand, from an SVG or PDF of mzDraw) has the structural checks; the data fields add the rest. `args` (optional, with `out` and no `commands`): `{"polarity":"both"|"positive"|"negative","ppm":N,"da":N}`.

Keys: `format`, `ok` (no finding of severity `error`), `engine`, `polarity`, `tolerance`, `counts`, `nodes`, `edges`, `findings` (the full row is in `schema::REQUEST`, section `out`). Findings: errors `pathway.edgeMismatch`, `pathway.formulaMismatch` (the `FORMULA` a node states is not that of its structure: counts, a trailing charge ignored), `pathway.ionMismatch`, `pathway.layout` (a cycle, too many structures); warnings `pathway.generationMismatch`, `pathway.ionUnmatched`, `pathway.edgeUnparsed` (a label that is words), `pathway.crossing`; info `pathway.edgeUnlabelled`, `pathway.edgeUnchecked` (a structure with a free-text label, two structures with the same atoms and another net charge, or a numeric label with no engine), `pathway.ionNotComputed`, `pathway.engine` (no engine: masses, ions and numeric labels were not checked). Masses and m/z are the engine's (4 decimals, the rounding of the TP table); `ppm` is computed from the PRINTED expected m/z, so a reader recomputes it from the output. **Where `descriptors/1` would plug in:** it is not in the pinned `mzlab` revision and the pin is not moved here; the per-node numbers come from `values::Scan::measure_within` (`Measurer`) in `pathcheck::report`, and a descriptor column would be added there, to `nodes`, behind the same engine.

## The CLI route (rule 15)

| need | how |
|---|---|
| draw | `{"commands":[{"command":"pathway:build","args":{"model":{...}}}],"out":"svg"}` (or `cdxml`, `ket`, `pdf`, `mzdraw`, `log`, `sdf`, ...), data on stdout |
| `--json` summary | `"report":true` (a `draw-result/1`, stable keys: `ok`, `created`, `removed`, `failed` with `rows`); the check itself is JSON with stable keys and a fixed key order |
| `--check` (validate, no output file) | `"out":"pathway-check"`: the answer is the check, nothing else is written; with the engine (`replay_with(request, n, Some(measure))`, as the TP table) the masses and ions are there |
| distinct exit code on a failed check | the example `replay` exits **3** when the check has an error (the document is written all the same), **1** for a refusal or a failed verdict, 0 otherwise; `Reply::check_failed` carries it for `mzlab-cli mol draw`, which has to map it. The glue door returns the text only (exit 0 on the page and in `replay_glue`); `tests/parity.mjs` therefore holds the glue cases to the text and the exit code 3 is tested natively (`the_native_example_exits_with_3_...`) |
| `--dry-run` | `"dry_run":true`: the model is validated and built, nothing is written; a bad model is the verdict with exit 1 |
| deterministic bytes | tested for every output (`the_same_model_gives_the_same_bytes_every_time`); wasm == native == recorded on `tests/data/log/pathway-draw.json` (parity: the glue cases with the engine, the vector outputs and the dry run and refusal through the plain replay) |
| replay | the log of a built pathway has no command name, only operations: `{"log":...}` replays to the same bytes (tested) |

The page is one client: the Add field and the file button take a model as they take a TP list (`pathmodel::looks_like`: a JSON object whose `format` is `pathway/1`); a refusal adds nothing and the live line names the lines (`draw2.say.tpRows`).

## Proof

- `crates/mzdraw/tests/pathway_draw.rs` (22): the built drawing (four structures, three anchored arrows with the labels, the data in the SDF, `observed` not written), one undo and one redo; one x per generation and no overlapping boxes; the same bytes every time for seven outputs; every problem listed with its line and key (seven problems of seven kinds in one model), the duplicate edge, the cycle, a label clash, a SMILES that does not read, an adduct that is not one; the whole-model refusals, the caps; the dry run; the log replay; the checks without an engine (labels `+O` right, `-CH2`, `+O2`, `+C6H8O6`, `-Cl+OH`, the isomer word wrong, words only a warning, an isomer right for equal formulas, a stated formula, a stated generation); a drawing made by `tp:import` is checked, a hand-edited arrow is caught; the options validated; `Reply::check_failed` and the exit codes of the example; the re-read tests (SVG and PDF give the same document, CDXML and KET the same structures and graph); the status drawn; the Add field and the file button; the number text.
- `crates/mzdraw-wasm/tests/pathway_draw.rs` (8; the real engine): hand sums to four decimals (toluene 92.0626, `[M+H]+` 93.0699, `[M-H]-` 91.0553; benzyl alcohol 108.0575; benzoic acid 122.0368, `[M-H]-` 121.0295; `+O` 15.9949, `+O-H2` 13.9793), ppm recomputed from the printed numbers, a wrong m/z and a wrong label, numeric labels and their tolerance, polarity, no adduct typed, an adduct with no shift, and the goldens through the glue door. It **fails without the change**: `pathway:build` is an unknown command (`error.draw2.command`) on `main`.
- e2e (three steps): by keyboard (model in the Add field, Enter; SVG, check and KET equal the goldens; undo and redo; a refused model changes nothing and the live line says `2, 5`), by touch on a tablet (Open a file tapped), by a pen (the Add button).
- Independent proof: **none run.** The masses and ions are the engine's, hand-checked above; no RDKit or other program compared the checks (the TP table slice has `tools/fidelity/tptable.py` for the same numbers; it was not extended). The layout is tp-05's.

## Not built (also in `docs/plan/queue.md`)

- A node with a formula alone (no SMILES): no structure to draw; refused with `pathNodeSmiles`.
- **PNG natively** (the CLI): needs a rasteriser; the page's PNG exists.
- `--from-dig RUN/dig.json`: the model made from a run of the dig of mzLab (the format is not in this repository).
- The flags `--check`, `--json`, `--dry-run` of `mzlab-cli mol draw` are the request fields above; the mapping (and exit code 3) is mzLab's.
- `branch` does not order the layout; `generation` is checked, not imposed (a layer is the longest path from a root).
- The check does not compare a `FORMULA` charge, an observed `rt` against anything, or a structure's isotopes against the ion.
- CDXML and KET lose the anchors and the data fields (the loss manifest lists them); a `.cdxml` re-read is anchored again by `pathway:anchor`, which can pick the wrong structure if the file was moved by hand.
- Finding keys (`pathway.*`) have no page text: the page does not list them yet.
