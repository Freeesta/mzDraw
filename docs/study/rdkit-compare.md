# What can be drawn from the command line: RDKit against mzDraw (study `rdkit-compare`)

Study and harness only, no product code. Owner goal: mzDraw must have more and better drawing features than RDKit's. This file answers with (1) a measured comparison of the two on one synthetic set, (2) one feature matrix of every RDKit drawing capability we found, with the mzDraw command that does it (or the gap) and the evidence, (3) the bugs the comparison found in mzDraw, (4) the gaps ranked, as rows of `docs/plan/queue.md`.

**How the evidence was made.** RDKit 2026.09.2 (`pip install rdkit` in a scratch virtual environment outside the repository, Python 3.13), mzDraw at `main` 05a0773 through its CLI route: the native `replay` example, which is the twin of the `.wasm` export and of `mzlab-cli mol draw` (`crates/mzdraw/examples/replay.rs`). Every row marked **M** was run: either by the harness `tools/rdkit_compare.py` (its committed output is `tools/rdkit_compare/reference.json`) or by a command written in the row. A row marked **D** is from RDKit's API as listed by the installed module (the `MolDrawOptions` fields, the functions of `rdDepictor`, `rdMolDraw2D`, `Draw`) and was not drawn and inspected. RDKit is a reference program here, never a dependency: nothing of it is linked, vendored or required by CI.

**Reproduce.**
```
cargo build --locked -p mzdraw --example replay
pip install rdkit                                   # optional; without it the script says so and exits 0
python3 tools/rdkit_compare.py --no-time > out.json # the same bytes as tools/rdkit_compare/reference.json
python3 tools/rdkit_compare.py --markdown           # the tables below, with timings; pictures in target/rdkit-compare/
```

## 1. The set and the ruler

`tools/rdkit_compare/set.tsv`: 32 synthetic inputs (invented or textbook-generic structures, rule 4): open and branched chains, two stereocentres, E and Z double bonds, cis ring substituents, 3-, 5-, 6- and 12-membered rings, two and three fused aromatic rings, a four-ring skeleton with six stereocentres, spiro, bridged, two cages, a zwitterion, a nitro N-oxide, a salt, dative bonds to a metal, 2H and 13C, an oxygen radical, a Boc-protected amine, two drug-like skeletons (five rings; amides and stereo), fused heteroaromatics, a sulfonamide, a triple bond, an allene, a phosphate, a polyene, a pyranose, and one reaction SMILES. A transformation-product scheme of four rows (a parent, two products, one convergent product) is the 33rd case.

Both layouts are read into RDKit (mzDraw's through its own molfile, so the coordinates and wedges are mzDraw's) and measured by one function, `layout_metrics`, in bond lengths (median bond = 1) and degrees:

| measure | meaning |
|---|---|
| `bond_dev_max` | worst relative deviation of a bond from the median length |
| `min_nonbonded`, `atom_clashes` | nearest two atoms that are not bonded; pairs closer than 0.5 |
| `bond_crossings` | pairs of bonds with no shared atom that cross |
| `label_crossings` | a labelled atom (not a plain carbon) with a bond that does not touch it closer than 0.35 |
| `ring_edge_dev_max`, `ring_angle_dev_max` | worst ring edge against the ring mean; worst ring corner against the regular polygon |
| `chain_angle_dev_mean`, `_max` | at atoms in no ring with 2 to 4 bonds, the gaps between bonds against 120 degrees (180 at a triple bond or an allene centre, 90 at four bonds) |
| stereo kept | the canonical isomeric SMILES of the input equals the one RDKit reads back from the tool's molfile (wedges), from mzDraw's SMILES and from mzDraw's CDXML |
| deterministic | two runs give the same SVG bytes (RDKit: two calls in one process; mzDraw: two processes) |

## 2. Results (measured, the whole set)

| measure | RDKit | mzDraw |
|---|---|---|
| inputs read (of 32) | 32 | 32 since rdkit-compare 7 (before: 30, the dative SMILES `[NH3]->[Pd]...` and the reaction SMILES refused, F4 and F5); `tools/rdkit_compare/reference.json` regenerated |
| SVG deterministic | 32 of 32 | 32 of 32 |
| stereo kept through the molfile | 31 of 31 molecules | 31 of 31 molecules (before the "either" bond of F3: 29 of 30, the polyene) |
| mzDraw stereo kept through its own SMILES / its CDXML read by RDKit | n.a. | 31 of 31 / 31 of 31 (before F3: 29 of 30 / 29 of 30, the same polyene) |
| atom clashes (pairs < 0.5) | 0 | 3 (cage 2, adamantane-like 1) |
| bond crossings | 3 (cage 2, adamantane-like 1) | 3 (the same two) |
| labelled atoms crossed by a bond | 0 | 0 |
| worst bond-length deviation | 2.0 (cage: one bond drawn 3 times the median) | 0.63 (adamantane-like) |
| nearest non-bonded atoms | 0.518 | 0.227 |
| mean chain-angle deviation over molecules | 0.0 degrees | 0.3 degrees |

Per molecule, 25 of the 30 inputs both tools drew give the same numbers for every measure (the layout of `mzlab-mol` and RDKit's default depiction agree on chains, fused rings, spiro, bridged bicycles, the macrocycle and the drug-like skeletons). The five that differ:

| molecule | RDKit | mzDraw | reading |
|---|---|---|---|
| cage (cubane skeleton) | bonds up to 3.0x the median, no clash | bonds within 0.18, 2 clashing pairs (0.43 apart) | each tool trades one fault for another; neither draws the usual cube projection |
| adamantane-like | bonds up to 2.19x, nearest 0.52, no clash | bonds within 0.63, nearest 0.23, 1 clash | RDKit better (no overlap) |
| salt | the two ions 0.67 apart | 1.73 apart | mzDraw better (ions do not touch) |
| branched (two quaternary carbons) | nearest 0.73, angles exact | nearest 1.0, one 30-degree deviation at a quaternary carbon | mzDraw roomier, RDKit more regular |
| phosphate | nearest 0.90 | nearest 1.34 | mzDraw better |

Timing (`--markdown`, five runs each, median over the set): RDKit 0.33 ms per molecule in one Python process (parse, layout, SVG); mzDraw 1.61 ms per request, of which 1.40 ms is the start of the `replay` process (measured alone on a one-atom request). The work itself is of the same order; a CLI that runs many requests in one process (mzlab-cli batch input, rule 15) removes the start. Not a benchmark: one machine, a cold cache, a small set.

Transformation-product scheme (four rows): mzDraw `tp:import` draws 4 structures and 4 anchored arrows with formula deltas, deterministic; RDKit has no scheme or pathway drawing, the nearest is a grid of 4 with legends and no arrows.

## 3. Bugs and losses found by the comparison

| id | finding | evidence (M) | consequence |
|---|---|---|---|
| F1 | **Lone pairs are not in the SVG or the PDF.** `atom.set lp` is drawn on the canvas, but the SVG and PDF writers skip the `DISC` primitive of the display list, so the dots never reach a file. `docs/built/curved-arrows.md` says "the SVG draws it and the lone pairs": not true at 05a0773. | `{"import":"CO","log":[{"op":"atom.set","id":2,"lp":2}],"out":"svg"}` gives one `<line>` and two `<text>`, no dot; `crates/mzdraw/src/svg.rs` `drawn_with` and `pdf.rs` handle `LINE`, `TEXT`, `TEXTX`, `TRI`, `RECT` only | a mechanism exported as SVG/PDF loses its electrons; fix first (small, high value) |
| F2 | **Enhanced stereo is dropped on import, and the CLI route does not say so.** `C[C@H](F)Cl \|&1:1\|` and the V3000 molfile RDKit writes for it (`MDLV30/STERAC1`) both come in as plain absolute stereo. The page says it: the import note `draw2.import.stereoGroups` (`crates/mzdraw/src/io.rs`, `web/lang/mzdraw.js`, test `crates/mzdraw/tests/formats.rs`). The CLI route does not: the `draw-result/1` of `report: true` has no `lost` entry and `out: loss:molfile-v3000` has no item. | `{"import":"C[C@H](F)Cl \|&1:1\|","report":true,"out":"smiles"}` gives `C[C@H](F)Cl` and no `lost`; same with the V3000 text | an agent on the CLI route reads a racemic or relative centre as absolute with no word of it (rule 15: what the page says, the CLI says too) |
| F3 | **An unspecified double bond is written as a defined E/Z.** RDKit writes an unspecified stereogenic double bond as "either" (crossed, bond stereo 3) in its molfile; mzDraw writes a plain double bond, so RDKit (and any reader that takes E/Z from 2D coordinates) reads the drawn geometry as specified. | polyene `C=CC=CC=CC=CC=C`: RDKit canonical SMILES of mzDraw's molfile, SMILES and CDXML differ from the input (E/Z added); RDKit's own round trip keeps it | a drawing says more than the input; `expect stereo kept` does not see it (it compares mzDraw with itself) |
| F4 | **A reaction SMILES is refused.** `CC(=O)O.OCC>>CC(=O)OCC` is `error.draw2.smiles`; the same reaction as an RXN file is drawn (reactants, plus, arrow, product). | harness row `reaction` | agents must build an RXN file to draw a reaction |
| F5 | **A dative SMILES is refused.** `[NH3]->[Pd](Cl)(Cl)<-[NH3]` is `error.draw2.smiles`, though the document has dative bonds (`bond.set k dative`). | harness row `dative` | coordination compounds cannot be drawn from SMILES |
| F6 | CXSMILES coordinates `\|(x,y,;...)\|` are ignored: the layout is computed. | `{"import":"CCO \|(0,0,;3,0,;3,3,)\|","out":"digest"}` gives the computed zigzag | the layout an agent gives is lost |
| F7 | A wedge is drawn on a centre that is not stereogenic. `COP(=O)(OC)O[C@@H]1CCCC1` (the ring carbon has two equal ring branches) gets 1 wedge; RDKit finds 0 stereocentres and draws none. | harness row `phosphate` (`wedges` 1, `stereo_centres` 0) | cosmetic: a mark with no meaning |

## 4. Feature matrix

Status: **built** (both do it), **better** (mzDraw does more, or RDKit cannot), **partial**, **missing** (RDKit does it, mzDraw cannot), **n.a.** (out of scope by CLAUDE.md rule 13, or not a drawing feature). Evidence: **M** measured (harness probe or the request in the row), **D** documented API only. "Test" names the golden area file of `tests/data/log/` (parity wasm == native) or the Rust test that proves the mzDraw side.

### 4.1 Input and coordinates

| # | capability | RDKit (API) | mzDraw (CLI route) | status | test | ev. |
|---|---|---|---|---|---|---|
| 1 | depict a SMILES | `MolFromSmiles` + `Compute2DCoords` | `{"import":SMILES,"out":"svg"}` | built | `smiles-files.json` | M |
| 2 | depict a molfile V2000/V3000 with its coordinates | `MolFromMolBlock` | `import` a molfile | built | `core.json` | M |
| 3 | depict every record of an SDF | `SDMolSupplier` (+ grid) | `import` an SDF: records side by side | built | `sdf-data.json` | M |
| 4 | coordinates given in a CXSMILES | read `\|(...)\|` | ignored, layout computed (F6) | missing | | M |
| 5 | enhanced stereo groups (`&1`, `o1`, V3000 `STERAC`) | kept and drawn | kept on their stereocentres (field `stg`, op `stereo.group`), read from and written to CXSMILES and V3000, drawn; KET and the V2000 chiral flag not yet (arch-std-19) (`built/enhanced-stereo-groups.md`) | built | `stereo-groups.json`, `crates/mzdraw/tests/stereo_groups.rs` | M |
| 6 | dative bonds in SMILES (`->`) | read | read: `->` and `<-` are bonds of the kind `dative` from the donor, which take no part in the valence (`N->[Cu+2]` keeps NH3); not written (a single bond, a `loss` item); an aromatic ring donor (`[Pd]<-[n]1ccccc1`) is still refused, see Not built (`built/reaction-smiles.md`) | built | `reaction-smiles.json`, `crates/mzdraw/tests/reaction_smiles.rs` | M |
| 7 | reaction SMILES (`A>>B`) | `ReactionFromSmarts(useSmiles=True)` | read: drawn as a reaction file is, reactants with plus signs, the arrow, the agents above it, the products, atom maps kept; not written (`built/reaction-smiles.md`) | built | `reaction-smiles.json`, `crates/mzdraw/tests/reaction_smiles.rs` | M |
| 8 | CDXML / CDX read | `MolsFromCDXML` (CDXML) | CDXML and binary CDX | better | `cdx.json` | M |
| 9 | 2D layout quality, ordinary molecules | default depictor | `mzlab-mol` layout D1: identical measures on 25 of 30 | built | harness | M |
| 10 | layout of cages and crowded polycycles | stretched bonds, no clash | uniform bonds, 3 clashes | partial | harness | M |
| 11 | spacing of separate ions and pieces | 0.67 bond lengths apart | 1.73 | better | harness | M |
| 12 | second layout engine | `SetPreferCoordGen(True)` | one layout | missing | | M |
| 13 | canonical orientation, straighten | `Compute2DCoords(canonOrient=True)`, `StraightenDepiction`, `NormalizeDepiction` | none | missing | | M |
| 14 | align to a template or scaffold | `GenerateDepictionMatching2DStructure` | none (`error.draw2.command`) | missing | | M |
| 15 | ring-system templates | `SetRingSystemTemplates` | none | missing | | M |
| 16 | re-layout of an existing drawing (clean up) | `Compute2DCoords` on any molecule | not wired (`layout::clean_up` returns the drawing) | missing | | M |
| 17 | fixed bond length | `MolDrawOptions.fixedBondLength` | `"set":["style.bond_length=20"]` | built | `prefs.json` | M |
| 18 | rotate | `MolDrawOptions.rotate` (whole picture, at draw time) | `rotate` op / command on any atoms, about any point, in the log with an exact inverse | better | `core.json` | M |
| 19 | mirror | none in MolDraw2D | `flip` (keeps the marks: the other enantiomer, said in the log) | better | `core.json`, `stereo-guard.json` | M |

### 4.2 What is drawn on a molecule

| # | capability | RDKit (API) | mzDraw (CLI route) | status | test | ev. |
|---|---|---|---|---|---|---|
| 20 | heteroatom labels with H, which carbons are labelled | `explicitMethyl`, `noAtomLabels` | `style.carbon_labels` none/terminal/all, `style.hydrogens` hetero/all | built | `label-display.json` | M |
| 21 | explicit hydrogen atoms | `AddHs` before drawing | `[H]` atoms of the input are kept and drawn | built | harness probe | M |
| 22 | charges, isotopes, radicals | drawn; `isotopeLabels`, `atomLabelDeuteriumTritium` (D, T) | drawn (`2H`, no D/T option) | built | harness (isotopes, radical, zwitterion) | M |
| 23 | wedges and hashes from stereo | `WedgeMolBonds` | wedges written by `mzlab-mol` on our coordinates; read back by RDKit 29 of 30 | built | `stereo-guard.json` | M |
| 24 | wavy bond | unknown stereo (`unspecifiedStereoIsUnknown`) | `bond.set s 3` | built | `core.json` | M |
| 25 | unspecified double bond marked "either" in the molfile | crossed bond, stereo 3 | built after F3: bond stereo 3, V3000 `CFG=2`, CDXML `Display="Wavy"`, read back (`built/either-double-bond.md`) | built | `either.json`, `crates/mzdraw/tests/either.rs` | M |
| 26 | hash drawn as wedge ladder or parallel lines | ladder (no option found among `MolDrawOptions`) | `style.hash_style` wedge/lines | better | `label-display.json` | M (mzDraw), D (RDKit) |
| 27 | bond kinds: bold, dashed, bold dashed, plain hash | none of these as display kinds (not found among `MolDrawOptions` or the bond types drawn) | `bond.set k` bold/dash/bdash/hash | better | `bond-display.json`, `bond-kinds.json` | M (mzDraw), D (RDKit) |
| 28 | dative bond drawn as an arrow; hydrogen bond drawn dashed or dotted | `BondType.DATIVE`; `BondType.HYDROGEN` (dashed) | `bond.set k dative`, `k hbond` (dotted) | built | `bond-kinds.json` | M |
| 29 | zero-order bond | `BondType.ZERO` drawn dashed | `bond.set o 0` refused (`error.draw2.bond`) | missing | | M |
| 30 | side of the second line of a double bond | automatic only | `bond.set pos` auto/c/l/r | better | `bond-display.json` | M |
| 31 | aromatic bonds drawn aromatic (dashed) | `PrepareMolForDrawing(kekulize=False)` | Kekulé only (circles named out of scope in `built/label-display-modes.md`) | missing | | M |
| 32 | abbreviation set by hand | `atomLabels`, `_displayLabel` | `atom.set lab` (31 groups, lowered digits, expanded in the molfile) | built | `labels.json` | M |
| 33 | abbreviations condensed automatically | `rdAbbreviations.CondenseMolAbbreviations` | none | missing | | M |
| 34 | R groups, attachment points | dummy atoms, `dummiesAreAttachments` (wavy attachment) | `R`, `R1`..`R32` labels; no attachment-point mark | partial | `labels.json` | M |
| 35 | atom map numbers | drawn by default | drawn, `display.maps` on/off | built | `atom-maps.json` | M |
| 36 | atom and bond indices | `addAtomIndices`, `addBondIndices` | `display.ids` off/atoms/bonds/both: the document ids the operations name, placed clear of lines and labels (`built/display-ids.md`) | built | `display-ids.json`, `crates/mzdraw/tests/display_ids.rs` | M |
| 37 | CIP labels R/S, E/Z | `addStereoAnnotation`, `showAllCIPCodes` | `display.cip`: the labels of the engine `cip` (glue) drawn in brackets, placed as the ids; the plain core replay says it has no engine (`built/display-cip.md`) | built | `cip.json`, `crates/mzdraw-wasm/tests/cip.rs` | M |
| 38 | enhanced-stereo labels, chiral flag | `addStereoGroupAnnotation`, `includeChiralFlagLabel` | the group labels `and1`, `or1` next to each centre (`abs` beside another kind); no chiral-flag label (arch-std-19) (`built/enhanced-stereo-groups.md`) | partial | `stereo-groups.json` | M (first), D (second) |
| 39 | atom notes, bond notes | `atomNote`, `bondNote` properties | `atom.data` kept in the document and SDF, not drawn | missing | | M |
| 40 | lone pairs | none | `atom.set lp` on the canvas, and in SVG and PDF since PR #127 (dfa7ddf) (F1; `built/lone-pairs-export.md`) | better | `curve.json`, `crates/mzdraw/tests/curve.rs` | M |

### 4.3 Highlighting, captions, grids

| # | capability | RDKit (API) | mzDraw (CLI route) | status | test | ev. |
|---|---|---|---|---|---|---|
| 41 | colour atoms and bonds by index | `highlightAtoms`, `highlightBonds`, `highlightAtomColors` (a halo behind) | `colour {atoms, bonds, objects, col}`: the ink itself, in the document, exact undo, CDXML colour table | partial | `colour.json` | M |
| 42 | halo highlight with radius, several colours per atom | `highlightRadius`, `fillHighlights`, `DrawMoleculeWithHighlights` | none | missing | | M |
| 43 | substructure-match highlight (SMARTS) | `GetSubstructMatches` + highlight | queries are out of scope (rule 13); not built | n.a. | | D |
| 44 | similarity maps, atom-weight contours | `Draw.SimilarityMaps` | none | missing | | M |
| 45 | legend (free text) under a molecule | `legend=` | `display.titles`: the title (`atom.title`) drawn under its structure, first caption line (`built/display-titles.md`) | built | `display-titles.json`, `crates/mzdraw/tests/display_titles.rs` | M |
| 46 | calculated values under a structure (formula, masses, m/z, logP, TPSA, HBD/HBA ...) | none (compute, then pass a legend string) | `display.values`, `atom.cap` | better | `values.json`, `struct-props.json` | M |
| 47 | identification level under a structure | none | `atom.ann` (`L2b`) | better | `annotations.json` | M |
| 48 | grid of molecules with legends | `MolsToGridImage(molsPerRow, legends, subImgSize, highlightAtomLists)` | `grid` (`cols`, `gap`, `order` drawing/reading/title with numbers as numbers, `atoms`): `move` operations, one undo step, cells as wide as the widest structure or caption, the legends (`display.titles`) of a row on one line; a picture of the drawing is the grid image (`built/structure-grid.md`). No fixed cell size: a cell fits its content | built | `grid.json`, `crates/mzdraw/tests/grid.rs` | M |
| 49 | molecules drawn at one scale | `drawMolsSameScale` | always: the bond length of the style is the scale | built | `style.json` | D |

### 4.4 Reactions and schemes

| # | capability | RDKit (API) | mzDraw (CLI route) | status | test | ev. |
|---|---|---|---|---|---|---|
| 50 | reaction from an RXN file | `ReactionFromRxnBlock` + `DrawReaction` | `import` RXN: reactants, plus signs, arrow, products, agents above | built | `scheme-row.json` | M |
| 51 | colour by reactant / map numbers in a reaction | `DrawReaction(highlightByReactant=True)` | none | missing | | D |
| 52 | arrows, plus signs, texts, curved arrows, formatted text | none | `obj.add` arrow/plus/text/curve, `runs` | better | `curve.json`, `text-runs.json` | M |
| 53 | a reaction or scheme laid out on one line, oriented | reaction layout only | `scheme:row`, `scheme:orient` | better | `scheme-row.json`, `scheme-orient.json` | M |
| 54 | transformation-product pathway from a table | none | `tp:import`, `pathway:tree`, `pathway:label`, `tp:number`, `out tptable` | better | `tp-import.json`, `tp-table.json` | M |

### 4.5 Style

| # | capability | RDKit (API) | mzDraw (CLI route) | status | test | ev. |
|---|---|---|---|---|---|---|
| 55 | black and white | `useBWAtomPalette` | `style.colour=mono` | built | `colour.json` | M |
| 56 | custom element palette | `updateAtomPalette`, `useCDKAtomPalette`, `useAvalonAtomPalette` | element or mono, plus colours per item (#41) | partial | `colour.json` | D |
| 57 | transparent background | `clearBackground = False` | the SVG has no background | built | | M |
| 58 | background colour | `setBackgroundColour` | none | missing | | D |
| 59 | font size, line width, bond spacing, margin | `baseFontSize`, `bondLineWidth`, `multipleBondOffset`, `padding` | `style.font_size`, `line_width`, `bond_spacing`, `margin` (points, true size) | built | `style.json` | M |
| 60 | ACS 1996 style | `SetACS1996Mode`, `MolToACS1996SVG` | `"style":"acs-1996"` | built | `style.json` | M |
| 61 | hand-drawn look | `comicMode` | none | missing | | D |
| 62 | accessible SVG: text as text, title, description | glyphs as paths, class names only | `<text>`, `<title>` (formula), `<desc>`, `role="img"` | better | `crates/mzdraw/tests/svg_access.rs` | M |

### 4.6 Output and machine use

| # | capability | RDKit (API) | mzDraw (CLI route) | status | test | ev. |
|---|---|---|---|---|---|---|
| 63 | SVG | `MolDraw2DSVG` | `out svg` | built | every area file | M |
| 64 | PNG from the command line | `MolDraw2DCairo`, `MolToFile(.png)` | page only: the core replay has no rasteriser (`out png` is `error.draw2.request`) | missing | | M |
| 65 | vector PDF | none (a PIL image saves as raster PDF) | `out pdf`, document and molfile attached | better | `pdf.json` | M |
| 66 | the drawing travels in the picture and opens again | PNG metadata (`MolFromPNGString`): molecules | SVG metadata, PDF attachment, PNG `iTXt`: the whole document (arrows, texts, annotations) | better | `pdf.json` | M |
| 67 | CDXML, CML, MRV out | `MolToCDXMLBlock`, `MolToCMLBlock`, `MolToMrvBlock` | `out cdxml`, `cml`; MRV read | built | `colour.json` (CDXML), `crates/mzdraw/tests/cml.rs` | M |
| 68 | KET out, RXN out | none / `ReactionToRxnBlock` | `out ket`, `out rxn` | better | `crates/mzdraw/tests/ket.rs`, `core.json` (RXN) | M |
| 69 | deterministic bytes | in one process (32 of 32) | across processes (31 of 31) and wasm == native (`tests/parity.mjs`) | better | `tests/parity.mjs` | M |
| 70 | result envelope, dry run, postconditions, keyed errors, exit codes | none (a library) | `report`, `dry_run`, `expect`, `error_key`, exit 1/2 | better | `result-envelope.json` | M |
| 71 | loss report before writing a format | none | `out loss:FORMAT` | better | `loss.json` | M |
| 72 | edit log with exact undo, history, diff, proposals | none (RWMol edits have no inverse) | `draw_edit/1`, `undo:N`, `out history`, `diff`, `propose`/`accept` | better | `history.json`, `diff.json`, `proposal.json` | M |
| 73 | drawing checks and a compact digest for agents | `flagCloseContactsDist` (a red mark on close atoms) | `out std`, `out digest`, `boxes` | better | `std.json`, `digest.json` | M |

### 4.7 Out of scope or not drawing

| # | capability | RDKit (API) | mzDraw | status | ev. |
|---|---|---|---|---|---|
| 74 | query atoms and bonds drawn | SMARTS molecules drawn, `queryColour` | rule 13: listed on import, not built | n.a. | M |
| 75 | polymer brackets, data S-groups | `SRU` brackets drawn | rule 13: abbreviations only | n.a. | M |
| 76 | MCS and R-group decomposition pictures | `rdFMCS`, `RGroupDecompose` | need substructure queries (rule 13); the in-scope part is #14 (align a series to a given core) | n.a. | M |
| 77 | 3D views, notebooks | IPythonConsole, py3Dmol | 2D editor | n.a. | D |

### 4.8 Counts

| status | rows |
|---|---|
| built | 29 |
| better | 22 |
| partial | 5 |
| missing | 16 |
| n.a. | 5 |
| total | 77 |

Built or better: 51 of 77 (#5 missing to built and #38 missing to partial, rdkit-compare 3 part 2; #36 missing to built, rdkit-compare 4; #45 partial to built, rdkit-compare 5; #37 partial to built, rdkit-compare 6; #48 missing to built, rdkit-compare 8; #6 and #7 missing to built, rdkit-compare 7; after F1, PR #127, and F3, the "either" PR: #40 partial to better (PR #127, merged), #25 missing to built; the review moved the hydrogen bond from #27, better, to #28, built: both rows keep their status, so the totals did not change). RDKit does something mzDraw cannot in the 16 missing rows and in part of the 5 partial ones. Evidence: 67 rows measured, 3 measured in part (#26, #27: the mzDraw side; #38: its first half), 7 documented only (#43, #49, #51, #56, #58, #61, #77).

## 5. Where mzDraw is already better, with evidence

- **It is an editor, not a renderer**: every change is a `draw_edit/1` operation with an exact inverse (`tests/ops.rs` property test), the session has a history, a diff between two drawings, proposals with hunks (#72). RDKit draws a molecule it is handed; it keeps no drawing.
- **Schemes**: arrows, plus signs, curved arrows with lone-pair ends, formatted text, one-line schemes, and the transformation-product pathway from a table with formula deltas and TP numbers (#52-54). RDKit draws one reaction at a time.
- **What is written under a structure** comes from the chemistry: formula, masses, m/z of adducts, descriptors, identification level (#46-47). RDKit has a free legend.
- **Files**: vector PDF, the whole document inside SVG/PDF/PNG, CDX read, KET and RXN written, a loss report before each write (#65-71).
- **Agents**: one request format, keyed errors and exit codes, dry run, postconditions, a digest and drawing checks; the same bytes from the page, the wasm and the CLI (#69-73).
- **Spacing of pieces**: separate ions and crowded substituents get more room on this set (#11, salt 1.73 against 0.67; phosphate 1.34 against 0.90).

Guards: the golden files named in the matrix replay these outputs on every PR (`tests/parity.mjs`). Not yet guarded: the spacing advantage of #11, which no golden measures; a test that keeps the nearest non-bonded distance of the salt and the phosphate at least as it is today is in the queue (row "rdkit-compare: guard").

## 6. Gaps, ranked

Rank by what agents and chemists need most from the command line, then by size. Each is a row of `docs/plan/queue.md` "Follow-ups without a prompt" (prefix `rdkit-compare:`); none is built by this PR.

| rank | gap | rows | why first | size |
|---|---|---|---|---|
| 1 | lone pairs in SVG and PDF (F1) | #40 | a bug in a built feature; the doc says it works | small |
| 2 | enhanced stereo: a loss item on the CLI route (the page already says it), then keep and draw it | #5, #38 | a change of meaning the CLI does not report; rule 15 | small (loss item), medium (keep) |
| 3 | "either" mark for an unspecified stereogenic double bond in molfile/CDXML out (F3) | #25 | other programs read a defined E/Z | small |
| 4 | atom and bond indices in the picture (`display.ids`) | #36 | agents name atoms by id in every op; seeing them closes the loop | small |
| 5 | CIP labels drawn (`display.cip`, from the engine `cip`) | #37 | chemists check R/S and E/Z at a glance; the engine exists | medium |
| 6 | reaction SMILES and dative SMILES in (F4, F5) | #6, #7 | the shortest machine form of a reaction or a complex | medium (mzlab-mol reader) |
| 7 | legend under a structure: draw `atom.title` | #45 | captions in every figure | small |
| 8 | grid of structures with legends (`grid` command: columns, gap, order) | #48 | series figures; with #7 | medium |
| 9 | PNG pixels from the CLI without a browser | #64 | agents that need a raster for a report or a chat | large (a rasteriser and fonts in Rust, or a host-side step) |
| 10 | align to a template or a given core; clean up of a drawn structure | #14, #16 | series in one orientation; layout of a hand-drawn part | medium |
| 11 | halo highlight with radius and several colours | #42 | the RDKit look agents ask for | medium |
| 12 | aromatic display (dashed or circle) as a style | #31 | some journals; needs the out-of-scope note revisited | medium |
| 13 | atom notes drawn (`atom.data` key shown at the atom) | #39 | per-atom values (shifts, charges) | small |
| 14 | zero-order bond | #29 | organometallics | small |
| 15 | cage layout without clashes (templates) | #10, #15 | rare skeletons | large (`mzlab-mol` layout) |
| 16 | CXSMILES coordinates kept | #4 | an agent's own layout | small (reader) |
| 17 | automatic abbreviation condensation | #33 | large protecting groups | medium |
| 18 | canonical orientation, second layout, background colour, palettes, D/T labels, chiral flag, comic mode, similarity maps | #12, #13, #56, #58, #61, #44 | rare or cosmetic | small each |

## 7. Not done by this study

- No product code: every gap stays a queue row; the feature PRs come one by one after the coordinator's answer.
- PNG pixels were not compared (mzDraw's are made by the page); the pictures RDKit and mzDraw write were not compared by eye or by a pixel metric, only by the layout ruler.
- The ruler measures coordinates, not label boxes: a label that overlaps a bond it does not touch is approximated by a disc of 0.35 bond lengths.
- Timings are one machine, one run of five repetitions; `mzlab-cli` (the batch CLI of mzLab) was not run.
- RDKit's options were listed and the main ones drawn; `DrawReaction(highlightByReactant)`, palettes, the chiral flag and the background colour were not drawn (rows marked D).
- No guard test was added (docs and tools only in this PR); the guard for #11 is a queue row.
