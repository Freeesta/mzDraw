# mzDraw for transformation products (ARCH study `tp-draw`)

This is a report only: no code depends on this file. It reviews six feature ideas from an external LLM (Gemini). It checks each idea against the owner's three priorities and against what mzDraw has built or planned, and picks what to build first. The prompts it adds are `docs/prompts/tp-01` to `tp-11`.

Owner priorities (a mass spectrometrist):
1. draw the transformation products (TPs) of a parent fast;
2. arrows that organise themselves, symmetric and well routed;
3. product tables.

Every feature is CLI-native (CLAUDE.md rule 15) and its logic is in Rust.

How to read it:
- The six ideas are **unverified hypotheses**. A claim of novelty in them is not evidence.
- `*` marks a fact this study could not read at a primary source.
- Sources were read on 2026-11-21.

## 1. Source register (what was checked)
| id | source | read? | fact used |
|---|---|---|---|
| T1 | Schymanski et al. 2014, ES&T 48(4):2097-2098, doi:10.1021/es5002105 | **not read** (paywall, 403) `*` | Level 3 "tentative candidate(s)". This comes from secondary sources. The 2014 wording on positional isomers was not seen. |
| T2 | Charbonnet et al. 2022, ES&T Lett. 9(6):473-481, doi:10.1021/acs.estlett.2c00206, https://pmc.ncbi.nlm.nih.gov/articles/PMC9202347/ | read | Level 3a "positional isomer candidates" (the position of a group is ambiguous). This is a PFAS scale; mzDraw already supports it as a `scale` (arch-std-14). |
| T3 | enviPath, https://envipath.org ; https://wiki.envipath.org/doku.php?id=pathways | read | A pathway is compounds as nodes and reactions as connections, and it can be edited. The pages do not name a layout method or say whether edges are labelled. |
| T4 | NORMAN-SLE S66 EAWAGTPS, https://zenodo.org/records/3829088 (v S66.0.1.4, 2024-02-07); S68 HSDBTPS, https://zenodo.org/records/14836807 | read (CSV headers) | S66 has the columns `SMILES`, `InChIKey`, `MolecularFormula`, `ExactMass`, `ParentOrTP`, `Eawag_ID_of_Pair`, `Mass_Diff`, `Form_Diff`, `Transformation`. S68 has `Predecessor_*`, `Successor_*`, `Transformation`, `Evidence_*`. |
| T5 | ChemDraw "Reaction Clean-up", Align / Distribute | **not read** (Revvity PDFs 403; old help 401) `*` | Search snippets list a reaction clean-up and Align/Distribute. No feature was found that orients a product on its reactant or lays out a multi-step tree. The owner's "ChemDraw does arrows badly" stays the owner's experience; this study did not test it. |
| T6 | Marvin, https://docs.chemaxon.com/latest/marvin_cleaning.html | read (Pro page 404, snippet only) | "Clean and Arrange in 2D" uses templates. "Clean up reaction" (snippet `*`) aligns one reaction and needs exactly one reaction selected. |
| T7 | Ketcher help, https://raw.githubusercontent.com/epam/ketcher/master/documentation/help.md ; Indigo #2251 | read | Layout aligns elements relative to the arrow. #2251 (pluses added by Layout) was fixed in Indigo 1.50.0-rc.1, closed 2026-10-24. |
| T8 | RDKit `rdDepictor.GenerateDepictionMatching2DStructure`, https://www.rdkit.org/docs/source/rdkit.Chem.rdDepictor.html | read | It exists, and `alignOnly` gives a rigid alignment. It is the reference of plan-14. |
| T9 | GNPS networking views, https://ccms-ucsd.github.io/GNPSDocumentation/networkingviews/ ; CHaMP, https://zenodo.org/records/7912853 | read | GNPS can label an edge with "DeltaMZ", so Δm/z on edges is **prior art**. CHaMP infers pathways from mass-difference networks; its layout is not described. No tool was found that places TPs by generation on x and Δm/z on y. That proves nothing. |
| T10 | CXSMILES `m:` (multicentre S-groups), https://docs.chemaxon.com/lts-iodine/chemaxon-extended-smiles-and-smarts-cxsmiles-and-cxsmarts.html ; V3000 `ENDPTS` / `ATTACH=ANY` (secondary: depth-first.com 2021) | read; the BIOVIA spec `*` | Position variation is a multicentre S-group or a generic bond, which is query or Markush territory. |
| T11 | Mass-shift table, Molecules 2019, 24:1203, Table 4, https://pmc.ncbi.nlm.nih.gov/articles/PMC6480032/table/molecules-24-01203-t004/ | read | −CH2 −14.0157, −H2 −2.0157, +O 15.9949, +H2O 18.0106, +C6H8O6 176.0321. +SO3 79.9568 and +C2H2O 42.0106 were computed from IUPAC masses and are not read in a source; tp-05 tests them against the glue's nuclides. |
| T12 | Elsevier artwork sizing, https://www.elsevier.com/authors/policies-and-guidelines/artwork-and-media-instructions/artwork-sizing ; ACS JACS/ES&T/AC guidelines PDFs | read | These numbers already sit in arch-std-16. The "ACS Document 1996" values (bond 14.4 pt = 0.200 in, line 0.6 pt, Arial 10 pt) come from a secondary source (Wikipedia) `*`. |
| T13 | mzLab `mzlab-mol` at the pin `ccd4680`: `transform.rs` (`trasf/1`), `delta.rs`, `substruct.rs` | read (code) | `trasf/1` already has an **uncertain product**: `delta`, `sites` (parent atoms with support `p`) and a `region` (`zone` / `multicenter` / `whole`), with no product structure. `delta.rs` parses, adds and prints formula deltas. No mass table was found in `mzlab-mol` (grep at the pin), which agrees with backlog E6. That the masses are in `mzlab-core` (`nuclides.rs`) is taken from backlog §0 F1 and was not re-read here `*`. The TP prompts take masses through arch-web-17 (`selection/1` through the glue, E6). |

## 2. What exists or is planned (the overlap map)
| need | built | planned | gap |
|---|---|---|---|
| arrows tied to structures | no: an arrow is two free points | none | **tp-01** (fields, follow), **tp-02** (tool, commands) |
| copy a parent and keep its coordinates | copy/paste (new ids, no correspondence) | plan-11 atom maps; plan-14 orientation by fitting | **tp-03**: derive with maps; no fitting needed, because the coordinates are inherited |
| show what changed | none | plan-15 id diff and plan-16 proposal layer (for agent patches, not parent vs TP) | **tp-04**: a lens on the maps |
| lay out a scheme | none | plan-13 one row; plan-14 orientation | **tp-05** (core, properties), **tp-06** (page, Graphviz proof) |
| arrow labels | a typed text above the arrow | arch-web-13 adds `below` | **tp-07**: computed from the structures |
| masses, adduct m/z | none | arch-web-17 values (glue) | reused by tp-07 and tp-09 |
| confidence level | `ann` with Schymanski and other scales (built) | — | none |
| per-TP data, SDF | `atom.data`, SDF out (built) | — | **tp-08** (import, numbering) names the fields; **tp-09** writes the table |
| journal sizes | presets `acs-1996`, `slides` | arch-std-16 export profiles | none: idea 6's "preset compiler" duplicates arch-std-16 |
| uncertain TP (L3) | — | plan-12 writes `trasf/1`, including the uncertain form | idea 3, reframed (V3) |

## 3. Verdicts
| id | idea | verdict | why |
|---|---|---|---|
| V1 | ghost-parent scaffold lock + diff lens | **WIN, build first** (tp-03, tp-04) | Priority 1. It is the fastest path from parent to TP. It needs no fitting, because the TP *is* the parent's coordinates plus an edit. The atom maps (plan-11) make the lens exact, and they feed plan-12's `trasf/1`. **Dropped part:** the 0-100 % "rigidity slider", which has no measurable meaning; a clean-up of the changed atoms is arch-std-15's. |
| V2 | Δm/z metro-map auto-layout | **WIN, build first, split** (tp-01, tp-02, tp-05, tp-06, tp-07; later V2b) | Priority 2. The core is a layered tree/DAG layout of anchored arrows with a measurable symmetry contract (§4). Δm/z labels on edges are prior art (GNPS, T9), so we claim no novelty. **Later (V2b):** y by Δm/z or RT, lane pins for convergent pathways, and 45°/90° polyline routing. These need a polyline arrow and real use of tp-04 first. |
| V3 | living Markush cloud for L3 positional isomers | **KEEP, reframed, after tp-03; no prompt yet** | Its value is real: L3 hydroxylation isomers are common (T2). As Markush or position-variation bonds it breaks rule 13 (T10: query/multicentre S-groups). How it stays in scope: draw the **parent** with a **site mark**, a document object listing parent atom ids and a delta (`+O`). It is an annotation, not a query, and it maps one to one onto `trasf/1`'s uncertain form (`delta`, `sites`, `region kind multicenter`, T13). The formula is parent + delta (`delta.rs`). "Fan out" = one `tp:derive` per atom listed in the mark: an explicit patch per site, *not* tp-10's `sites`, which lists where a named step's test holds and is a different thing. The two can agree for hydroxylation, but the mark's list is the analyst's, from the evidence. "Collapse" = keep one of them and move the annotation to L2/L1. Export goes through `trasf/1` (plan-12) and enumerated SMILES. CXSMILES `m:` and V3000 `ENDPTS` are **not written**; they are listed as not carried. A prompt waits for plan-12. |
| V4 | data-bound arrows (width from peak area) | **DROP** | Peak areas of different TPs are not comparable without standards, because ionisation efficiency differs. A width suggests a flux that was not measured. Even with a "relative uncalibrated" legend, it invites a wrong reading in a figure. `AREA` can be kept as data (tp-06) and is not drawn. |
| V5 | arrow-pull mass-shift autocomplete | **KEEP, reduced** (tp-10 table and CLI, tp-11 page field; second round) | Priority 1. The reduced form is a fixed table of standard steps, applied at a site the person or agent chooses, with the delta checked. It has no ranking and no rule engine (SMARTS stays out, rule 13). `Tab` is not used (WCAG 2.1.2, Built state), so `[` / `]` cycle the sites while the step field is open (`Alt+[` / `Alt+]` still rotate). |
| V6 | live-linked SI table + journal preset compiler | **WIN, build first, reduced** (tp-08, tp-09) | Priority 3. The table is a view of the drawing: import a list, number, export CSV/JSON with S66-style `Form_Diff` / `Mass_Diff` columns (T4). **Dropped part:** the preset compiler, because arch-std-16 covers it. It is called "NORMAN-style", never "NORMAN-ready", because no validator was run. A table drawn on the canvas comes later. |
| — | rejected by the source: 3D viewer, freeform graphics, MS2/reaction prediction | **agree** | Already dropped (backlog §4) or out of the editor's role. Prediction stays in mzLab's engines, framed as candidates. |

**Build first:** V2 (tp-01, tp-02, tp-05, tp-06, tp-07), V1 (tp-03, tp-04) and V6 (tp-08, tp-09).
- Order: tp-01, tp-02, tp-05, tp-06, tp-03, tp-04, tp-07, tp-08, tp-09, then tp-10 and tp-11.
- tp-01, tp-02, tp-05 and tp-06 come first: they answer the owner's top pain and need nothing new from mzLab beyond COO 16's yes on two fields.
- tp-03 waits for plan-11.
- tp-07 and tp-09 wait for arch-web-17.

## 4. Arrow organisation: what "symmetric" means (summary)
The normative text is in `docs/prompts/tp-05-pathway-layout.md`. In short:
- an anchored arrow joins two structures by id (tp-01). Its ends follow the structures when an editor *command* moves them; a bare `move` op does not re-route, and `pathway:route` does;
- the pathway is laid out as layers by generation (longest path), a primary-parent forest, and a tidy tree in y (Walker, made linear by Buchheim–Jünger–Leipert `*`);
- the columns are equally gapped, and a gap is widened only to keep slopes ≤ `max_angle` and arrows ≥ `min_arrow`;
- arrows are straight lines between box centres, clipped at the boxes.

The 14 properties are tests:
- determinism, order independence, translation, idempotence;
- columns, centring, fan symmetry `y_i + y_{m+1−i} = 2·y_parent`, mirror;
- arrow vectors (dx, ±dy), even spacing;
- bounded slope and length, no collision and no crossing of primary arrows;
- compactness and locality.

Each property is checked twice: on the unrounded layout with ε = 1e-9, and on the saved document (4 decimals) with ε = 3e-4. Translation is claimed only for shifts on the document's 1e-4 grid. Mirror is claimed only for single-root components. Only the first component's first root is a fixed point; later components are stacked below it.

Graphviz `dot` (tp-06) is the independent check of layers and crossings. It is optional: without it, the script says "skipped".

## 5. Backlog lines
They are in `docs/plan/backlog.md` §11:
- **rows 81–91**, one for each of tp-01..11;
- **the waves**;
- **the edits to existing prompts**: E27 (plan-13: the two command shapes, anchored arrows routed by tp-01), E28 (plan-14: amends E18, and E18 stays) and E29 (plan-20: stage `command`, exit 1 for a refused command, `failed.rows`, atomic, `id` second as E26);
- **the decisions** D31 (atomic TP import) and D32 (E18 stays).

The zones `tp`, `pathway`, `tp-list` and `tp-table` are in §5.

## 6. Risks and limits stated
- tp-03 and tp-04 depend on plan-11 (atom maps, wave 4). Without maps the lens refuses and never guesses.
- Masses (tp-07, tp-09) come from arch-web-17 (`selection/1` through the glue, E6). They are never typed in `mzdraw`. The electron mass of tp-09's proof (CODATA 2022) is taken from backlog §1.1 and was not re-read here `*`.
- No InChIKey column until `mzlab-mol` has InChI (a COO 16 question).
- tp-01 adds two `draw_edit/1` fields (`from`, `to`) and needs COO 16 agreement first. The other prompts add no op or field. They add request rows (D13): commands, `out` values, settings, `draw-result/1` keys (`failed.stage` `command`, `failed.rows`, E29), and reserved data names. tp-07 marks its labels by a rule on their text (signed deltas only), not by a stored flag.
- Novelty: none is claimed. T9 is prior art for Δm/z edge labels. The absence of a generation × Δm/z layout in the tools searched proves nothing.
