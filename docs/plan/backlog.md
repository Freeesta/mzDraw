# mzDraw backlog: one ordered plan (ARCH-PLAN)

DRAFT for agreement with the coordinator (COO-MZDRAW). Until the coordinator answers, the wave plan in section 6 is a proposal.

This file has five inputs:
- the external research output `docs/input/gemini-feature-discovery.md` (Gemini Deep Research, PR #39);
- the three studies and their prompts: arch-std on `main`, arch-web on PR #38 (branch `claude/arch-web` at `7d15a7b`), arch-cd on branch `claude/arch-cd` at `ef4542d`;
- the running work, as the coordinator listed it on 2026-11-21;
- the code of `main` at `222612f`;
- the engines of mzLab, read at the pinned commit `ccd4680` and at mzLab `main` `76a4703`.

Facts marked `*` are not verified at a primary source.

## 0. Two findings that change the plan

### F1. mzLab already has most of the engines Gemini proposes we build
These modules sit in `mzlab-feat` under `src/chem/` at the pinned commit `ccd4680`, behind the features `chem` and `check`. `mzdraw-wasm` already links `check`. Each one has a twin in `mzlab-cli`.

| engine | module | contract | CLI twin | what it does |
|---|---|---|---|---|
| CIP | `cip.rs`, `cip_rank.rs` | `cip/1` | `mol cip` | Hierarchical digraph, rules 1a to 5. Rule 4c is not done, nor rules 4b and 5 on double bonds, nor axial M/P. |
| cuts | `cut.rs` | `cut/1` | `mol cut` | Homolytic, heterolytic or "+H" cuts across chosen bonds, with the pieces and a shift. |
| fragment masses | `fragments.rs` | | | Pieces of one bond with their formula, neutral mass and [F+H]+. |
| MSn graph | `breaks.rs` | | `msn graph --breaks` | One- and two-bond breaks with H transfer that explain an `msn-graph/1`. |
| in-silico fragmentation | `fragmenter.rs` | | | Systematic cuts matched to MS/MS peaks within a ppm window. |
| curved arrows | `mech.rs` | `mechanism/1` | `mol mech` | Form check of curved arrows and lone pairs. It never says which mechanism is right. |
| highlighting | `hilite.rs` | `highlight/1` | `mol hilite` | Evidence layers on atoms and bonds. |
| selection info | `sel.rs`, feature `sel` | `selection/1` | `mol sel` | Formula and masses of a selection. |

`mzlab-core` holds the nuclide masses (`nuclides.rs`) and the adduct table (`molecules/ion.rs`). `mzlab-mol` holds `trasf/1`, a parent-to-product transformation with an atom map (`transform.rs`), and a substructure search (`substruct.rs`).

So Gemini cards 1, 4 and 6 and bold idea 1 become work in mzDraw only:
- an object in the document (a cut line, an anchored curved arrow, a lone pair);
- the glue that hands the molfile and the stable ids to the engine;
- painting the answer.

None of them needs a new engine. mzLab's own study already says so (`docs/agenti/mzdraw.md` on mzLab `main`): "every engine already takes a molfile ... mzDraw hands over the same molfile plus stable atom ids".

A second consequence: the glue runs only in wasm today, so the native replay of `crates/mzdraw` cannot give the same bytes as the page once a feature uses an engine. `plan-04` fixes that first.

### F2. The edit language has two dialects
`mzlab-feat/src/chem/edit.rs` (the reader of `mol draw --set edit=` on mzLab `main` `76a4703`) knows 7 operations: `atom.add`, `atom.set`, `bond.add`, `bond.set`, `merge`, `move`, `remove`.

The log of mzDraw (`crates/mzdraw/src/ops.rs`) has 12, adding `atom.ann`, `atom.data`, `obj.add`, `obj.set`, `rotate`, `flip` and `ring`. It does not have `merge`.

Rule 10 (`CLAUDE.md`) says the log is `draw_edit/1` of mzLab. A log that uses any of mzDraw's extra ops is refused by the CLI route of mzLab today. This backlog adds about 12 more ops (section 4, column "COO 16"). `plan-01` makes the language one table that both sides read, and is a COO 16 item.

## 1. Audit of the Gemini output

Verified on 2026-11-21; every URL was read on that day.
- Ketcher issues were read on github.com; the REST API refused access.
- Bot-walled pages (Reddit, ResearchGate, SlideShare) are "not read".

### 1.1 Claims checked
| claim in the input | verdict | source read | note |
|---|---|---|---|
| Ketcher #1045 elliptical arrows for mechanisms | verified | https://github.com/epam/ketcher/issues/1045 | Closed 2021-12-24, shipped in 2.5; decorative arcs. |
| Ketcher #10427 dative bonds and valence | verified | https://github.com/epam/ketcher/issues/10427 | Opened 2026-07-10, still open. |
| Ketcher #1617 reaction layout | verified | https://github.com/epam/ketcher/issues/1617 | Closed 2022-05-27. |
| Ketcher #4861 multi-tail arrows | verified | https://github.com/epam/ketcher/issues/4861 | Closed 2024-09-03. |
| Ketcher #527 enhanced stereo window | verified | https://github.com/epam/ketcher/issues/527 | Closed 2021-06-07. It is a feature request, not a complaint about a modal. |
| Ketcher #197 attachment points outside R-group brackets | verified | https://github.com/epam/ketcher/issues/197 | Closed 2023-07-18. |
| ChemAxon ends free individual academic licences; Marvin JS end of life Dec 2027 | verified | https://www.certara.com/announcement/modernizing-the-academic-program-and-retiring-chemicalize/ (2026-05-21); https://www.certara.com/cxn/blog/marvin-js-end-of-life-and-transition-notice (2025-09-16) | Existing licences stay active to 2026-07-31. Marvin JS: last sale 2026-12-31, end of life 2027-12-31. |
| ChemSketch Freeware closed after the Revvity acquisition | partly (stronger) | https://www.acdlabs.com/resources/freeware/chemsketch/download.php | The page reads "ChemSketch Freeware Discontinued". ACD/Labs is part of Revvity Signals (deal announced 2025-11-10*). |
| Octet-Draw exists | verified | https://github.com/SamChemMan/Octet-Draw | Apache-2.0, TypeScript, CDXML/CDX. The user feedback quoted in the input was not read (Reddit). |
| CIP validation suite; 2017 talk by Mayfield, Lowe, Sayle | partly | https://github.com/CIPValidationSuite/ValidationSuite (v1.0, 2018-05-20); doi:10.1021/acs.jcim.8b00324 | 300 compounds. **No licence is stated**, so it cannot be committed. The 2017 talk was not read (SlideShare). |
| RDKit new CIP labeller | verified | https://rdkit.org/docs/source/rdkit.Chem.rdCIPLabeler.html | Since 2020.09; a port of Mayfield's "centres". |
| MassSpecGym 231,104 spectra | verified | https://huggingface.co/datasets/roman-bushuiev/MassSpecGym | MIT licence. "29k molecules" comes from the paper, read only through the search index. |
| MassBank 119,845 spectra / 18,529 compounds | partly (outdated) | https://massbank.eu/MassBank-api/metadata (v2026.11) | Now 140,372 spectra and 20,925 compounds. Zenodo release CC BY 4.0. |
| FlowER mechanistic dataset | partly | https://github.com/FongMunHong/FlowER ; doi:10.1038/s41586-025-09426-9 | Code is MIT. The size and licence of the data were not found. |
| Brecher, IUPAC 2006 / 2008 | partly (topics swapped) | doi:10.1351/pac200678101897 (2006 = stereo configuration); doi:10.1351/pac200880020277 (2008 = structure diagrams) | The input gives each topic the wrong year. |
| exact masses, electron mass | partly | https://www.nist.gov/pml/atomic-weights-and-isotopic-compositions-relative-atomic-masses (AME2012); CODATA 2022 https://physics.nist.gov/cgi-bin/cuu/Value?meu | Electron mass 5.485799090441e-4 u (CODATA 2022). mzLab's `nuclides.rs` holds its own table; its source is mzLab's to state. |
| draw.io embeds its XML in PNG and SVG | verified | https://www.drawio.com/docs/manual/export/xml-in-png/ ; https://www.drawio.com/docs/manual/export/embed-svg/ | This is prior art for "pictures that open again", which mzDraw already built (slice 3b). |
| ChemDraw Mass Fragmentation tool | partly | ChemDraw 2004 help, through the search index only (the direct fetch answered 401) | It cuts chosen bonds and shows formula and exact mass. Current documentation: not read. |
| Open Reaction Database | verified | https://github.com/open-reaction-database/ord-data ; doi:10.1021/jacs.1c09820 | Protocol Buffers schema; data CC BY-SA 4.0. |
| InChI 1.07 | verified (newer) | https://github.com/IUPAC-InChI/InChI/releases | 1.07.5 is current; MIT licence since 1.07.1. |
| Word/PowerPoint keep SVG `<metadata>` | not found | MS-ODRAWXML "Pictures", read through the search index only | Office keeps the SVG in an `svgBlip` and adds a PNG fallback. Nothing says whether `<metadata>` survives, so it must be tested (`plan-10`). |
| "7/34" problem counts, Reddit quotes, ChemDraw 25.5 regressions, Octet-Draw feedback | not verified | Reddit, forums | Bot-walled or not given. These counts are not used anywhere below. |

### 1.2 Verdicts
Sizes: S is up to about 300 lines, M about 600 lines (one PR), L means several PRs.

| item | verdict | evidence and reason | depends on | size |
|---|---|---|---|---|
| Card 1: MS/MS cleavage and exact-mass annotator | **REFRAME** → plan-05, plan-06, plan-07 | `cut/1`, `fragments`, `breaks` and the adduct table exist in mzLab (F1). mzDraw adds the cut-line object, the callout and the peak match through the glue. It is framed as annotation, never as prediction; `cut.rs` already says "candidates to judge, never a ranking". This is our edge as the mass-spectrometry editor. The Gemini op `add_ms_cleavage` is replaced by `obj.add t:"cut"`. | plan-04 | 3 × M |
| Card 2: living vector export | **REFRAME** → plan-09, plan-10; the rest is built | SVG `<metadata>` and PNG `iTXt` carrying the document are built (slice 3b); the clipboard formats are arch-web-21. New here: PDF out with the document embedded, and a survival matrix over the hosts we can run headless, plus a checklist for Word and PowerPoint run by hand. Outlining fonts into paths is **DEFER**: a font file has to be bundled and licensed, and the screen text would differ from the file. "#1 daily pain" is not verified. | UI-2 R1 (`export` section) | M + S |
| Card 3: reaction scheme and catalytic-cycle layout | **REFRAME** → plan-13, plan-14; cycle **DEFER**; ORD export **DROP** | Ketcher #1617 is real but fixed. Layout of a one-line scheme and keeping the core oriented the same way on both sides of an arrow are cheap with `move`/`rotate`/`flip` and the substructure search of `mzlab-mol`; no MCS engine is needed for the first cut. A catalytic cycle needs curved arrows on paths first (arch-cd-25). ORD is a protocol-buffer data model of experiments, outside a drawing editor. | arch-web-13, arch-cd-15 | 2 × M |
| Card 4: open CIP, enhanced stereo, IUPAC linter | **REFRAME** → arch-web-16, arch-std-01/02/12; "Explain CIP" **DEFER** | The CIP engine exists (`cip/1`). The linter is arch-std-01/02 (01 running), enhanced stereo is arch-std-12, the labels are arch-web-16. "Explain CIP" needs `cip/1` to emit its digraph trace, a mzLab change (COO 16). The CIP validation suite has no licence, so it can only be a run-time benchmark in mzLab and is never committed. | plan-04 | covered |
| Card 5: Markush / R-group / SAR sync | **DEFER** | Markush enumeration and R-group definitions are query features, outside rule 13 of `CLAUDE.md`; it needs an owner decision. R labels are built. Ketcher #197 is closed. | owner decision | L |
| Card 6: electron-conserving curved arrows, dative valence | **REFRAME** → arch-cd-25 + plan-08; "next intermediate" **DEFER** | `mechanism/1` checks the form of the arrows (F1); arch-cd-25 draws anchored arrows and lone pairs. Generating the next intermediate from a bond-electron matrix would be a new mzLab engine (COO 16). Dative bonds are arch-web-20; their valence meaning belongs to `mzlab-mol` (COO 16; Ketcher #10427 shows the trap). | arch-cd-25, plan-04 | S |
| Bold 1: spectrum-driven canvas | **REFRAME** → plan-07 + arch-web-15 | Peak list in, cuts that explain each peak out (`breaks`, `fragmenter`), shown as highlight layers. Live re-ranking of isomers while dragging is **DEFER** until plan-07 measures the false-positive density Gemini warns about. | plan-06 | M |
| Bold 2: executable drawings, transformation-product compiler | **REFRAME** → plan-11, plan-12; SMARTS and ORD **DEFER** | `trasf/1` exists in `mzlab-mol` and mzLab screens transformation products. mzDraw needs atom maps and an export of a drawn parent → product as `trasf/1`. Generating SMARTS is query language (rule 13). | — | 2 × M |
| Bold 3: agent ghost-diff co-drawing | **KEEP** → plan-15, plan-16 | It fits rule 15 best: an agent proposes a `draw_edit/1` patch, the person sees it as a layer and accepts or rejects each hunk, and every accept is an undo step. No claim of novelty (prior art: code-review diffs; no chemistry editor with it was found, which proves nothing). | UI-2 PR 2 | 2 × M |
| Do-not 1: freehand gesture recognition | **DROP** (agree) | Non-deterministic. Touch and pen editing are built; recognition is not. | — | — |
| Do-not 2: 3D conformers and viewer | **DROP** (agree) | mzLab has 3D (`mzlab-chem` embed); a host page can show it. | — | — |
| Do-not 3: macromolecules / HELM | **DROP** (agree) | Rule 13. | — | — |
| Do-not 4: natural-language structure generator | **DROP** (agree) | No server, deterministic bytes; agents drive the editor from outside (rule 15). | — | — |
| Do-not 5: OLE and proprietary binary formats | **REFRAME** | OLE is dropped. Reading **binary CDX** is kept (plan-17, plan-18): it is a common file format, Octet-Draw reads it, and the owner asks for all common formats. Writing CDX is DEFER. | slice 3d (`cdxml.rs`) | 2 × M |
| Better question 1: where do we lose on drawing speed? | **KEEP** → plan-02 | Answered by measurement, below (section 2). | — | M |
| Better question 2: analytical niche vs synthetic chemists | answered | Both, in order: drawing speed and file fidelity first (waves 1 to 4), the mass-spectrometry edge from wave 3 (plan-05 to plan-07). | — | — |
| Better question 3: synthetic data vs real files | **KEEP** → plan-03 | Fixtures stay synthetic (rule 4). A tool fetches public corpora at run time, never commits them, and commits only the success rates. Open question Q5. | — | M |

Gemini counts: 1 kept, 8 reframed, 1 deferred, 4 dropped. The better questions add 2 kept and 1 answered.

## 2. Drawing speed: a measured answer (plan-02)
There is no number today; we do not know where we lose to ChemDraw. The plan measures it on a fixed task set. Speed has two parts, one per kind of user (rule 15).

1. **Person speed.**
   - **Tasks.** 12 fixed tasks in `tests/data/velocity/tasks.json`. Each gives a target as a SMILES (plus arrows and texts for schemes) and is checked by the canonical SMILES of `save("smiles")`. The set: benzene; ethanol; isobutanol; ibuprofen; caffeine; a fused tricycle (anthracene); a steroid skeleton; a wedged diol; a charged zwitterion (glycine); a molecule with an abbreviation (Boc-protected amine); a two-component reaction with an arrow and a condition text; three structures aligned in a row.
   - **Expert paths.** For each tool, the shortest known sequence of inputs written by hand: pointer presses, keys and drags.
   - **Metrics.** Input actions, a deterministic count from the script, and wall time to the first valid structure, informative only (median of 5 Playwright runs).
   - **Ketcher.** The same harness runs the same tasks on a pinned standalone Ketcher build (Apache-2.0), fetched at run time under `tools/velocity/` and never committed. This gives a comparison we can repeat.
   - **ChemDraw.** It cannot run here. A one-page protocol for a person with a licence: the same tasks, the counts, a screen recording. The result is entered by hand into `velocity/1` with `"measured_by": "person"`.
2. **Agent speed.** The same tasks as CLI patches: number of `draw_edit/1` ops, bytes, and native time of `replay` (median of 20).

The output is `velocity/1` JSON, with one row per task and tool. A test fails when the action count of an mzDraw task rises; that is the regression guard. The first report is expected to show where hover hotkeys (arch-cd-03), chains (arch-web-08), templates (arch-web-19) and labels pay off, and it orders wave 3 and later; the coordinator re-orders after it.

## 3. Duplicates and conflicts resolved
| topic | prompts | resolution |
|---|---|---|
| shortcuts section | arch-web-06 (none), arch-std-06, arch-cd-17 | **arch-std-06 runs.** Add from arch-cd-17: the per-target table (canvas, atom, bond), reserved keys, a printable table. Add from arch-web-06: nothing (it was never written). |
| keyboard move / rotate / nudge | arch-std-05 (running), arch-web-07, arch-cd-15 | **arch-std-05 owns the keys**: Alt+Arrow moves, Alt+[ ] rotates. arch-cd-15 keeps align, distribute and scale and **drops** its Alt+Arrow 1° and Shift+Alt+Arrow 15° rotation. arch-web-07 keeps lasso, piece selection, the two-click band and focus on bonds and objects; it must not bind Shift+Arrow (arch-std-05 does). |
| context menu | arch-web-12, CTX-MENU (PR #37), arch-cd-20 | **arch-web-12 is withdrawn** (it is PR #37). Its long-press and Shift+F10 / Menu key notes go to the CTX-MENU page PR. arch-cd-20 (refine) runs after that page PR. **Its file is missing on `claude/arch-cd` at `ef4542d`** although the study lists it; arch-cd must add it (edit E10). |
| either / crossed double bond | arch-web-20, arch-std-11, arch-cd-04 | **arch-std-11** (`bond.set s:3` on a double bond). arch-web-20 has no "either". |
| bond kinds | arch-web-20, arch-cd-04, arch-cd-23 | One field `k` on a bond (arch-web-20): `bold`, `dative`, `hbond`, plus arch-cd-04's dashed kinds. arch-cd-04 keeps the double-bond position `pos`. arch-cd-23 supplies V3000 types 9 and 10. Whether a dative bond counts for valence is decided in `mzlab-mol` (COO 16). |
| CIP labels | arch-web-16, arch-cd-02 | **arch-web-16 runs**, computing through `cip/1` in the glue (plan-04), so the native replay gives the same bytes. Add arch-cd-02's `"cip"` request field. |
| enhanced stereo | arch-std-12, arch-cd-24 | **arch-std-12 runs** (`stereo.group {atoms, g}`, atom field `stg`). arch-cd-24 becomes its additions. |
| abbreviations expand / contract / user | arch-web-18, arch-cd-10, arch-cd-11, arch-std-08 | **arch-web-18 runs**, with op names `abbr.expand` / `abbr.contract`. Composed labels (CO2Me) must reuse the label-unit tokenizer of arch-std-08, which runs first. |
| templates | arch-web-19, arch-cd-12, arch-cd-13 | **arch-web-19 runs** (op `tmpl`), with arch-cd-12's merge rules added. |
| chain | arch-web-08, arch-cd-05 | **arch-web-08 runs** (op `chain`), with arch-cd-05's count display added. |
| clean-up | arch-web-10, arch-cd-01, Gemini card 3 | **arch-web-10 runs** (op `place {atoms, xy}`), with arch-cd-01's acceptance added: isomeric SMILES identical before and after, wedges keep their meaning. |
| values | arch-web-17, arch-cd-14 | **arch-web-17 runs**, but takes its numbers from `selection/1` (`mzlab-feat` feature `sel`) through the glue: `mzlab-mol` has no mass table (arch-cd-14 is right). |
| copy as / drop / paste | arch-web-21, arch-cd-16 | Both run: web-21 for drop and copy-as, cd-16 for paste on a free spot. The key `Ctrl+Shift+K` goes to clean-up (arch-web-10); "copy as KET" moves into the copy-as chooser. |
| theme tokens | arch-web-04, arch-std-03, UI-2 PR 3 | Order: UI-2 PR 3 (tokens), then arch-std-03 (element palettes, `background`), then arch-web-04 (host tokens, no own palette). |
| palettes | arch-cd-18, arch-cd-19, UI-2 R1 (PR #35) | arch-cd-18 and arch-cd-19 run on top of R1, after UI-2 PR 2. |
| embedding API | arch-web-01..03, arch-cd-21 | **arch-web runs.** The element name stays `<mzdraw-editor>` until the product is renamed; it is the one literal, in one file (owner: the name will change). |

## 4. The backlog
Columns:
- **Zone** is the main write zone (section 5).
- **COO 16** is "yes" when the item needs COO 16's agreement first: a new `draw_edit/1` op or field, or a change in `mzlab-mol` / `mzlab-feat`.
- **Proof** is the independent check named in the prompt.

Status: RUN = running now, Q = queued, P = has a prompt, N = new prompt in this PR, W = withdrawn, D = deferred.

| # | id | item | status | size | depends on | zone | COO 16 | proof |
|---|---|---|---|---|---|---|---|---|
| 1 | UI-2 R1 | palette layout in Rust (PR #35) | RUN | M | — | `palettes.rs`, settings `layout`/`export` | no | property tests |
| 2 | UI-2 PR 2 | icons and page palettes | RUN | M | R1 | page | no | screenshots |
| 3 | UI-2 PR 3 | light / dark tokens | RUN | S | PR 2 | page css | no | contrast |
| 4 | CTX-MENU | menu model (PR #37) + page PR | RUN | M+S | — | `context.rs`, bridge `menu` | no | APG pattern |
| 5 | arch-std-01 | `std/1` stereo checks | RUN | M | — | `conform.rs` | no | InChI TechMan 1.07 |
| 6 | arch-std-04 | accessible SVG | RUN | S | — | `svg.rs`, `outline.rs` | no | SVG 1.1 element test |
| 7 | arch-std-05 | drag-free editing | RUN | S | — | input | no | 24×15° round trip |
| 8 | arch-std-08 | label reversal | RUN | S | — | `label.rs` | no | goldens |
| 9 | arch-std-14 | annotation scales | RUN | S | — | `ann.rs` | no | Charbonnet 2022 |
| 10 | arch-std-02 | geometry checks | Q | M | std-01 | `conform.rs` | no | flaw sheet |
| 11 | arch-std-13 | crossing gap | Q | S | std-02 | scene | no | goldens |
| 12 | plan-01 | one op table for `draw_edit/1` | N | S | — | new `schema.rs` | **yes** (ownership of the language) | mzLab `edit.rs` op set ⊆ table, by test |
| 13 | plan-02 | drawing-speed benchmark | N | M | — | `tools/velocity`, `tests/velocity.mjs` | no | Ketcher pinned build; ChemDraw protocol |
| 14 | plan-03 | format fidelity against RDKit / Open Babel / InChI | N | M | — | `tools/fidelity` | no | RDKit 2025.09*, Open Babel 3.1.1*, InChI 1.07.5 |
| 15 | plan-04 | engine glue callable natively | N | S | CTX-MENU bridge merged | bridge, new `glue.rs` | no (uses existing features) | parity wasm == native with engines |
| 16 | arch-std-09 | OpenSMILES file rules | P | S | — | readers (`io.rs`) | no | OpenSMILES 1.0 |
| 17 | plan-09 | PDF out with the document embedded | N | M | — | new `pdf.rs` | no | `qpdf --check`, `pdfdetach` (poppler) |
| 18 | plan-15 | semantic diff of two documents | N | M | plan-01 | new `diff.rs` | no | property: apply(diff(a,b),a) == b |
| 19 | plan-17 | binary CDX reader: structures | N | M | — | new `cdx.rs` | no | CDX ↔ CDXML of the same synthetic file, written by Open Babel 3.1.1* |
| 20 | arch-std-12 | enhanced stereo | P | M | std-01 | document atoms/bonds | **yes** (op `stereo.group`, field `stg`) | V3000 / CXSMILES against RDKit (plan-03) |
| 21 | arch-std-03 | palette contrast | P | S | UI-2 PR 3 | scene / style | no | WCAG 2.2 formula |
| 22 | arch-std-06 | shortcuts section (+ arch-cd-17 additions) | P | M | std-05 | input + settings | no | WCAG 2.1.4 |
| 23 | arch-web-01 | editor instances | P | M | plan-04 | bridge | no | two-editor test |
| 24 | plan-05 | cut line object | N | M | — | document objects | **yes** (`obj.add t:"cut"`) | `mol cut` of mzLab on the same molfile |
| 25 | plan-13 | one-line scheme layout | N | M | — | new `scheme.rs` | no (moves only) | geometry invariants; Ketcher comparison |
| 26 | plan-18 | binary CDX reader: objects | N | M | plan-17 | `cdx.rs` | no | as plan-17 |
| 27 | arch-std-11 | unknown double bond | P | M | std-12 | document atoms/bonds | **yes** (`s:3` on order 2) | round trips (plan-03) |
| 28 | arch-web-02 | `<mzdraw-editor>` element | P | M | web-01, UI-2 PR 2 | page (new file) | no (`apply` = log) | network log |
| 29 | plan-06 | fragment masses on the cut | N | M | plan-04, plan-05 | glue + objects | no | masses against RDKit `ExactMolWt` and mzLab `mol cut` |
| 30 | arch-web-10 | clean-up (+ arch-cd-01) | P | M | — | `layout.rs` | **yes** (op `place`) | isomeric SMILES unchanged |
| 31 | arch-cd-03 | hover hotkeys | P | S | std-06 | input | no | velocity/1 counts drop |
| 32 | arch-web-13 | arrow kinds (+ arch-cd-06) | P | S | UI flyout | document objects | **yes** (obj fields) | KET / CDXML round trip |
| 33 | plan-10 | survival matrix of pictures | N | S | plan-09 | `tools/survival` | no | rsvg-convert, Inkscape, LibreOffice headless, ImageMagick |
| 34 | arch-cd-04 | double-bond position, dashed display | P | S | web-20 field agreed | scene + bond field | **yes** (`pos`) | CDXML |
| 35 | plan-11 | atom maps | N | M | std-11 | document atoms | **yes** (`atom.set map`) | RXN / SMILES maps against RDKit |
| 36 | arch-std-10 | label display modes | P | S | std-03 | scene / style | no | goldens |
| 37 | arch-web-03 | host adapter | P | S | web-02 | page | no | — |
| 38 | arch-web-07 | selection (lasso, piece) | P | M | std-05 | input | no | — |
| 39 | plan-07 | peak list → explained cuts | N | M | plan-06 | glue | no | MassBank CC BY 4.0, fetched at run time, success rate |
| 40 | arch-web-14 | rich text (+ arch-cd-07) | P | M | — | document objects | **yes** (`runs`) | goldens, KET |
| 41 | plan-14 | keep the core oriented across a scheme | N | M | plan-13 | `scheme.rs` | no | RDKit `GenerateDepictionMatching2DStructure` |
| 42 | arch-std-07 | page accessibility audit | P | S | UI-2 PR 3 | page css + e2e | no | WCAG values |
| 43 | arch-web-16 | CIP labels (+ arch-cd-02) | P | M | plan-04 | scene labels | no | RDKit `rdCIPLabeler` on the plan-03 set |
| 44 | arch-web-08 | chain (+ arch-cd-05) | P | S | std-06 | input | **yes** (op `chain`) | velocity/1 |
| 45 | arch-web-05 | viewer and form | P | S | web-02 | page | no | WHATWG text |
| 46 | arch-web-15 | highlight marks | P | M | — | document atoms/bonds | **yes** (field `hl`) | KET |
| 47 | plan-12 | `trasf/1` out | N | M | plan-11 | io | no (`mzlab-mol` has it) | mzLab `trasf/1` reader; RDKit reaction |
| 48 | arch-cd-25 | curved arrows and lone pairs | P | M | web-13 | document objects | **yes** (`lp`, `t:"curve"`) | `mechanism/1` |
| 49 | arch-web-17 | values (+ arch-cd-14) | P | S | plan-04 | glue | no | RDKit masses |
| 50 | arch-cd-08 | radical, isotope, periodic table | P | S | web-07 | input | no | — |
| 51 | plan-16 | agent proposal layer | N | M | plan-15, UI-2 PR 2 | overlay + page | no (view only) | e2e accept/reject = log |
| 52 | plan-08 | curved-arrow check | N | S | arch-cd-25, plan-04 | glue | no | `mol mech` of mzLab |
| 53 | arch-web-09 | merge on drop | P | M | web-08 | input | **yes** (`merge` exists in mzLab, not in mzDraw) | — |
| 54 | arch-web-11 | atom popover (+ arch-cd-09) | P | M | cd-08, CTX-MENU page | bridge + page | no | — |
| 55 | arch-web-18 | abbreviations (+ arch-cd-10/11) | P | M | std-08 | `label.rs` | **yes** (`abbr.*`) | molfile of expanded = contracted |
| 56 | arch-web-19 | templates (+ arch-cd-12/13) | P | M | std-06 | new data + ops | **yes** (`tmpl`) | velocity/1 |
| 57 | arch-web-20 | bond kinds (+ arch-cd-23) | P | M | cd-04 | document bonds | **yes** (`k`; dative valence in `mzlab-mol`) | V3000, KET |
| 58 | arch-web-21 | drop and copy as | P | S | CTX-MENU page | page | no | — |
| 59 | arch-cd-16 | smart paste | P | S | web-21 | editor_edit | no | — |
| 60 | arch-cd-15 | align, distribute, scale (no Alt keys) | P | S | web-10 | ops via commands | no | geometry |
| 61 | arch-cd-18 | palette tear-off | P | M | UI-2 PR 2 | `palettes.rs` + page | no | — |
| 62 | arch-cd-19 | named layouts | P | S | cd-18 | `palettes.rs` | no | — |
| 63 | arch-cd-20 | context-menu refine | P (file missing) | S | CTX-MENU page | `context.rs` | no | — |
| 64 | arch-web-04 | host theme tokens | P | S | UI-2 PR 3, std-03 | scene screen palette | no | goldens unchanged |
| 65 | arch-cd-26 | history panel | P | S | — | editor + page | no | log |
| 66 | arch-cd-22 | desktop shell | P | L | owner's choice of shell, web-02 | new crate | no | headless smoke |
| — | arch-web-12 | context menu | W | — | = CTX-MENU | — | — | — |
| — | arch-cd-01/02/05/06/07/09/10/11/12/13/14/17/21/23/24 | superseded notes | W (additions only) | — | see section 3 | — | — | — |
| — | Markush / SAR (card 5) | | D | L | owner, rule 13 | — | — | — |
| — | catalytic-cycle layout | | D | M | arch-cd-25 on paths | — | — | — |
| — | Explain CIP trace | | D | M | `cip/1` trace (mzLab) | — | yes | — |
| — | next mechanistic intermediate | | D | L | new mzLab engine | — | yes | — |
| — | font outlining in files | | D | M | font licence | — | — | — |
| — | CDX writer | | D | M | plan-18 | — | — | — |

Totals: 66 rows to run (11 running or queued, 18 new prompts, 37 with existing prompts). Deferred: 6. Withdrawn: 1 prompt, plus 15 arch-cd prompts reduced to notes of additions.

## 5. File zones
Two chats never write the same zone at the same time.

| zone | files |
|---|---|
| **document atoms/bonds** | `doc.rs` (atom/bond fields), the atom/bond arms of `ops.rs`, the writers of those fields in `io.rs`, `ket.rs`, `cdxml.rs`, `cml.rs`, `rxn.rs` |
| **document objects** | `doc.rs` (objects), the `obj.*` arms of `ops.rs`, object painting in `scene.rs` |
| **scene / style** | atom, bond and label painting in `scene.rs`, `settings.rs` style fields, `svg.rs` |
| **input** | `editor_tools.rs`, `editor_keys.rs`, `editor_touch.rs`, `editor_edit.rs` |
| **readers** | `io.rs` read paths. A new reader module (`cdx.rs`) is its own zone. |
| **bridge** | `crates/mzdraw-wasm/src/*` |
| **page** | `web/*` (a new page file is its own zone) |
| **conform** | `conform.rs` |
| **label** | `label.rs` |
| **layout** | `layout.rs`, `scheme.rs` |
| **tools** | `tools/<own dir>`, a new `tests/*.mjs` |
| **new module** | one new file in `crates/mzdraw/src` |

Append-only lines that any chat may add next to another chat (merge = keep both):
- the Built state bullets of `docs/mzdraw.md`;
- `measured` in `tools/wasm-cap.toml`;
- keys in `web/lang/mzdraw.js`;
- cases in `tests/data/mzdraw_log.json`;
- `mod` lines in `lib.rs`;
- one match arm in `Op::from_value` / `Op::write`;
- one arm in `Editor::save`.

## 6. Wave plan (at most 8 chats each, Sonnet at medium effort)
Wave 0 is what runs now: UI-2 (R1, PR 2, PR 3), CTX-MENU, arch-std-01, -04, -05, -08, -14.

| wave | starts when | items | zones |
|---|---|---|---|
| **1** | now | plan-01, plan-02, plan-03, plan-04, plan-09, plan-15, plan-17, arch-std-09 | new `schema.rs` · tools/velocity · tools/fidelity · bridge new `glue.rs` · new `pdf.rs` · new `diff.rs` · new `cdx.rs` · readers |
| **2** | std-01, std-05, CTX-MENU Rust and UI-2 PR 2 merged; COO 16 answered on plan-01 | arch-std-02, arch-std-12, arch-std-03\*, arch-std-06, arch-web-01, plan-05, plan-13, plan-18 | conform · document atoms/bonds · scene/style · input · bridge · document objects · layout · `cdx.rs` |
| **3** | wave 2 merged; first `velocity/1` report in | arch-std-11, arch-std-13, arch-web-02, plan-06, arch-web-10, arch-cd-03, arch-web-13†, plan-10 | document atoms/bonds · scene · page (new file) · bridge glue · layout · input · document objects · tools |
| **4** | wave 3 merged | plan-11, arch-std-10, arch-web-03, arch-web-07, plan-07, arch-web-14, plan-14, arch-std-07 | document atoms/bonds · scene/style · page (element file) · input · bridge glue · document objects · `scheme.rs` · page css |
| **5** | wave 4 merged | arch-web-16, arch-web-08, arch-web-05, arch-web-15‡, plan-12, arch-cd-25, arch-web-17, arch-cd-04‡ | scene labels · input · page · document atoms/bonds · io · document objects · glue · bond display |
| **6** | wave 5 merged | plan-16, arch-web-09, arch-web-11, arch-web-18, arch-web-19, arch-web-20, arch-web-21, plan-08 | overlay+page · input · bridge · label · ops+data · document bonds · page clipboard · glue |
| **7** | wave 6 merged | arch-cd-08, arch-cd-15, arch-cd-16, arch-cd-18, arch-cd-19, arch-cd-20, arch-web-04, arch-cd-26 (+ arch-cd-22 once the owner picks a shell) | input · layout · editor_edit · palettes (18 then 19 in one chat) · context · scene palette · editor |

Notes on the waves:
- \* arch-std-03 waits for UI-2 PR 3 if PR 3 has not merged by then; if so, swap in arch-std-10.
- † arch-web-13 and plan-05 both write the object zone, so arch-web-13 starts only after plan-05 (wave 2) has merged.
- ‡ arch-web-15 (field `hl`) and arch-cd-04 (field `pos`) are both in the document atoms/bonds zone. Run them one after the other in one chat, or move arch-cd-04 to wave 6.
- Prompts whose COO 16 agreement has not come back by their wave start are swapped for the next prompt with "no" in that column.

## 7. Edits to existing prompts
These are listed here, not rewritten. Each chat applies its edit when it starts.

1. **E1 arch-std-06:** take arch-cd-17's per-target table (canvas, atom, bond), its reserved keys and its printable table; the command table is the one arch-cd-03 fills later.
2. **E2 arch-cd-15:** drop Alt+Arrow and Shift+Alt+Arrow rotation and plain-arrow nudging (arch-std-05 owns them); scaling uses arch-web-10's `place`.
3. **E3 arch-web-07:** do not bind Shift+Arrow (arch-std-05).
4. **E4 arch-web-10:** add arch-cd-01's acceptance (isomeric SMILES identical before and after; wedge meaning and E/Z kept, `ez_mismatches`). The key is `Ctrl+Shift+K`.
5. **E5 arch-web-16:** compute through `cip/1` in the glue of plan-04. The native replay calls the same glue, so parity holds. Add arch-cd-02's `"cip"` request field. Independent check: RDKit `rdCIPLabeler` on the plan-03 set.
6. **E6 arch-web-17:** numbers from `selection/1` (`mzlab-feat` feature `sel`) through the glue; there is no mass table in `mzlab-mol`. Remove "ask COO for masses in mzlab-mol". Check: RDKit `ExactMolWt` reference values from plan-03.
7. **E7 arch-web-21:** "copy as KET" moves into the chooser; `Ctrl+Shift+K` is clean-up.
8. **E8 arch-web-18:** reuse arch-std-08's label-unit tokenizer for composed labels; the op names are `abbr.expand` / `abbr.contract`.
9. **E9 arch-web-20:** the field `k` takes arch-cd-04's dashed kinds; V3000 types 9 and 10 come from arch-cd-23; valence meaning is in `mzlab-mol` (COO 16).
10. **E10 arch-cd:** add the missing `arch-cd-20-context-menu-refine.md` on `claude/arch-cd`.
11. **E11 every prompt** that adds a `draw_edit/1` op or field: add the op to the plan-01 table and wait for COO 16's agreement before its PR.
12. **E12 every prompt** whose feature calls an engine: go through `glue.rs` (plan-04), never call `mzlab_feat` from `lib.rs` directly.
13. **E13 handoff:** the new prompts hand off at about 150k tokens; the existing ones say 200k. Keep each as written.

## 8. Open questions for the coordinator
These are in the message sent before the PR; the answers will be recorded here.
