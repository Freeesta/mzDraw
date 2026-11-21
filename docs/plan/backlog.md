# mzDraw backlog: one ordered plan (ARCH-PLAN)

Agreed with the coordinator (COO-MZDRAW) and COO 16 on 2026-11-21; the decisions are in section 8.

This file has five inputs (a sixth, the second Gemini output, is section 9):
- the external research output `docs/input/gemini-feature-discovery.md` (Gemini Deep Research, PR #39);
- the three studies and their prompts:
  - arch-std 01–14 on `main`;
  - arch-std second pass 15–22 on PR #45 (branch `claude/arch-std-2` at `0cad22a`; on `main` after #45 merges);
  - arch-web on `main` (PR #38);
  - arch-cd on PR #41 (branch `claude/arch-cd` at `87520eb`; on `main` after #41 merges, not final until then);
- the running work, as the coordinator listed it on 2026-11-21;
- the code of `main` at `222612f`;
- the engines of mzLab, read at the pinned commit `ccd4680` and at mzLab `main` `76a4703`.

Facts marked `*` are not verified at a primary source.

## 0. Two findings that change the plan

### F1. mzLab already has most of the engines Gemini proposes we build
These modules sit in `mzlab-feat` under `src/chem/` at the pinned commit `ccd4680`, behind the features `chem` and `check`. `mzdraw-wasm` already links `check`. Most have a twin in `mzlab-cli`; `fragments.rs` and `fragmenter.rs` are reached through the flow steps (`spectra.insilico@1`), not a `mol` subcommand, so their twin column is empty. Table verified at pin `ccd4680`.

| engine (verified at pin `ccd4680`) | module | contract | CLI twin | what it does |
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
| Card 2: living vector export | **REFRAME** → arch-std-18 (PDF), arch-std-17 (export default and clipboard), plan-10; the rest is built | SVG `<metadata>` and PNG `iTXt` carrying the document are built (slice 3b); the clipboard formats are arch-web-21. New: PDF out with the document embedded (arch-std-18, written first; plan-09 was withdrawn for it), and a survival matrix over the hosts we can run headless, plus a checklist for Word and PowerPoint run by hand. Outlining fonts into paths is **DEFER**: a font file has to be bundled and licensed, and the screen text would differ from the file. "#1 daily pain" is not verified. | UI-2 R1 (`export` section) | M + S |
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
| Do-not 5: OLE and proprietary binary formats | **REFRAME** | OLE is dropped. Reading **binary CDX** is kept (arch-std-22, written first; plan-17 and plan-18 were withdrawn for it): it is a common file format, Octet-Draw reads it, and the owner asks for all common formats. Writing CDX is DEFER. | slice 3d (`cdxml.rs`) | M |
| Better question 1: where do we lose on drawing speed? | **KEEP** → plan-02 | Answered by measurement, below (section 2). | — | M |
| Better question 2: analytical niche vs synthetic chemists | answered | Both, in order: drawing speed and file fidelity first (waves 1 to 4), the mass-spectrometry edge from wave 3 (plan-05 to plan-07). | — | — |
| Better question 3: synthetic data vs real files | **KEEP** → plan-03 | Fixtures stay synthetic (rule 4). A tool fetches public corpora at run time, never commits them, and commits only the success rates. Decided in D5 (section 8). | — | M |

Gemini counts: 1 kept, 8 reframed, 1 deferred, 4 dropped (14 items). The better questions add 2 kept and 1 answered.

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
"Runs" names the one prompt to launch; the others become notes of additions that the running chat reads at its start (section 7).

| topic | prompts | resolution |
|---|---|---|
| shortcuts section | arch-web-06 (never written), arch-std-06, arch-cd-17 | **arch-std-06 runs**, with arch-cd-17's additions (E1). |
| keyboard move / rotate / nudge | arch-std-05 (built or running), arch-web-07, arch-cd-15 | **arch-std-05 owns the keys**: Alt+Arrow moves, Alt+[ ] rotates. arch-cd-15 keeps align, distribute and scale and drops its Alt keys (E2). arch-web-07 keeps lasso, piece selection, the band and bond/object focus, and does not bind Shift+Arrow (E3). |
| context menu | arch-web-12, CTX-MENU (PR #37 merged; page PR waits), arch-cd-20 | **arch-web-12 is withdrawn.** Its long-press and Shift+F10 / Menu key notes go to the CTX-MENU page PR. arch-cd-20 (refine) runs after that page PR (E10). |
| clean-up | arch-std-15 (`atoms.xy`), arch-web-10 (`place`), arch-cd-01 | **arch-std-15 runs; ONE op: `atoms.xy`.** arch-web-10 is withdrawn. arch-cd-01's acceptance (isomeric SMILES unchanged, wedge meaning and E/Z kept) and arch-web-10's "clean the selection only" are added to std-15 (E4). |
| text runs | arch-std-21, arch-web-14, arch-cd-07 | **arch-std-21 runs** (runs, `_`/`^` parser, Formula button, KET and CDXML). arch-web-14 is withdrawn. arch-cd-07's symbol picker stays as an addition (E15). |
| export default, clipboard, copy as | arch-std-17, arch-web-21, arch-cd-16 | **arch-std-17 runs first**: background, the Download default, and the clipboard payload list decided in Rust. **arch-web-21 runs after it** (drop files, a "Copy as" chooser built on std-17's payload list). arch-cd-16 (paste on a free spot) runs after that. The key `Ctrl+Shift+K` is not taken by either (E7). |
| PDF | arch-std-18, plan-09 (withdrawn) | **arch-std-18 runs**; plan-09's determinism and independent checks are added (E16). |
| CDX read | arch-std-22, plan-17 + plan-18 (withdrawn) | **arch-std-22 runs** (one slice: the container reader feeds the CDXML element tree, so objects come with it); plan-17's proof (Open Babel) is added (E17). |
| either / crossed double bond | arch-std-11, arch-web-20, arch-cd-04 | **arch-std-11.** IUPAC ST-4.4 says the crossed form is not acceptable (arch-std second pass), so its default is the wavy form and the crossed form is only an option. |
| bond kinds (dative, hydrogen, bold, dashed) | arch-std row 17 ("new order codes"), arch-web-20 (`k`), arch-cd-04, arch-cd-23 | **arch-web-20 runs with ONE field `k`**, written as V3000 types 9 and 10 (COO 16, D4). arch-std row 17 is realised by arch-web-20, with no new order codes. arch-cd-04 keeps the double-bond position `pos` and puts its dashed kinds on `k`. arch-cd-23 becomes additions. Valence meaning of a dative bond is `mzlab-mol`'s (E9). |
| CIP labels | arch-web-16, arch-cd-02 | **arch-web-16 runs**, through `cip/1` in the glue (plan-04), with arch-cd-02's `"cip"` request field (E5). |
| enhanced stereo | arch-std-12, -19, -20; arch-cd-24 | **arch-std-12** (document, op, drawing), **then 19** (V3000, KET), **then 20** (CXSMILES). arch-cd-24 becomes additions. |
| abbreviations | arch-web-18, arch-cd-10, arch-cd-11, arch-std-08 | **arch-web-18 runs** (`abbr.expand` / `abbr.contract`, section `abbreviations`), after arch-std-08, reusing its label-unit tokenizer (E8). |
| templates | arch-web-19, arch-cd-12, arch-cd-13 | **arch-web-19 runs** (op `tmpl`, COO 16, D3), with arch-cd-12's merge rules. |
| chain | arch-web-08, arch-cd-05 | **arch-web-08 runs** (op `chain`, COO 16, D3), with arch-cd-05's count display. |
| values | arch-web-17, arch-cd-14 | **arch-web-17 runs**, taking its numbers from `selection/1` through the glue (E6). |
| theme tokens | arch-web-04, arch-std-03, UI-2 PR 3 | Order: UI-2 PR 3, then arch-std-03, then arch-web-04. |
| palettes | arch-cd-18, arch-cd-19, UI-2 (R1 PR #35, page PR 2) | arch-cd-18 then arch-cd-19, two prompts in order, on top of R1 and after UI-2 PR 2; final scope as arch-cd fixes PR #41. |
| embedding API | arch-web-01..05, arch-cd-21 | **arch-web runs** (web-01 first: it removes the one global editor of the bridge and blocks web-02..05). The element name `<mzdraw-editor>` is the one literal, in one file. |
| desktop app | arch-cd-22 | Owner: one component, two distributions (standalone app macOS + Windows; bundled by default in mzLab site and app through Tauri). COO 16: a separate shell for now, the shared `release.yml` pattern, signing later by the owner. arch-cd-22 runs last (wave 8). |

## 4. The backlog
Columns:
- **Zone** is the main write zone (section 5).
- **COO 16** is "yes" when the item adds a `draw_edit/1` op or field (agreement before its PR) or needs a change in `mzlab-mol` / `mzlab-feat`. Ops already agreed in section 8 say "agreed".
- **Proof** is the independent check the prompt names.

Status: RUN = running now or waiting for merge, Q = queued, P = has a prompt, N = new prompt in this PR, W = withdrawn. The prompts arch-std-15..22 are on PR #45 and all `arch-cd-*` on PR #41: they are on `main` after #45 and #41 merge; until then read them from those branches.

| # | id | item | status | size | depends on | zone | COO 16 | proof |
|---|---|---|---|---|---|---|---|---|
| 1 | UI-2 R1 | palette layout in Rust (PR #35) | RUN | M | — | `palettes.rs`; settings `layout`, `export` | no | property tests |
| 2 | UI-2 PR 2 | icons and page palettes | RUN | M | R1 | page | no | screenshots |
| 3 | UI-2 PR 3 | light / dark tokens | RUN | S | PR 2 | page css | no | contrast |
| 4 | CTX-MENU page | menu on the page (PR 1 #37 merged) | RUN | S | UI-2 PR 2 | page | no | APG menu pattern |
| 5 | arch-std-01 | `std/1` stereo checks | RUN | M | — | conform | no | InChI TechMan 1.07 |
| 6 | arch-std-05 | drag-free editing | RUN | S | — | input | no | 24×15° round trip |
| 7 | arch-std-08 | label reversal | RUN | S | — | label | no | goldens, IUPAC GR-2.1.6 |
| 8 | arch-std-14 | annotation scales | RUN | S | — | `ann.rs` | no | Charbonnet 2022 |
| 9 | plan-01 | one table of `draw_edit/1` ops | N | S | — | new `schema.rs` | agreed (D1) | mzLab op list ⊆ table |
| 10 | plan-02 | drawing-speed benchmark | N | M | — | `tools/velocity`, `tests/velocity.mjs` | no | pinned Ketcher; person protocol |
| 11 | plan-03 | fidelity against RDKit / Open Babel / InChI | N | M | — | `tools/fidelity` | no | InChIKey by RDKit, Open Babel |
| 12 | plan-04 | engine glue callable natively | N | S | — | bridge, new `glue.rs` | no (D2) | parity with engines; bytes = mzLab CLI |
| 13 | plan-15 | semantic diff | N | M | — | new `diff.rs` | no | property apply(diff(a,b),a) == b |
| 14 | arch-std-09 | OpenSMILES file rules | P | S | — | readers (`io.rs`) | no | OpenSMILES 1.0 |
| 15 | arch-std-18 | PDF writer (+ E16) | P | M | — | new `pdf.rs` | no (D7) | `qpdf --check`, `pdfdetach` |
| 16 | arch-std-22 | CDX reader (+ E17) | P | M | — | new `cdx.rs` | no | Open Babel reads the same files |
| 17 | arch-std-02 | geometry checks | Q | M | std-01 | conform | no | flaw sheet |
| 18 | arch-std-12 | enhanced stereo: document, op, drawing | P | M | std-01 | document atoms/bonds | yes (`stereo.group`, `stg`) | IUPAC ST rules; RDKit via plan-03 |
| 19 | arch-std-03 | palette contrast | P | S | UI-2 PR 3 | scene/style | no | WCAG 2.2 formula |
| 20 | arch-std-06 | shortcuts (+ E1) | P | M | std-05 | input + settings section | no | WCAG 2.1.4 |
| 21 | arch-web-01 | editor instances | P | M | plan-04 | bridge | no | two-editor test |
| 22 | plan-05 | cut line object | N | M | plan-01 | document objects | yes (`t:"cut"`) | `mol cut` of mzLab |
| 23 | plan-13 | one-line scheme layout | N | M | — | new `scheme.rs` | no (moves only) | geometry invariants; RXN roles by RDKit |
| 24 | arch-std-15 | clean-up (+ E4) | P | M | — | `layout.rs` + op | yes (`atoms.xy`) | isomeric SMILES unchanged |
| 25 | arch-std-11 | unknown double bond | P | M | std-12 | document atoms/bonds | yes (`s:3` on order 2) | round trips (plan-03) |
| 26 | arch-std-13 | crossing gap | Q | S | std-02 | scene | no | IUPAC ST-3.1; goldens |
| 27 | arch-web-02 | `<mzdraw-editor>` element | P | M | web-01, UI-2 PR 2 | page (new file) | no | network log |
| 28 | plan-06 | fragment masses on a cut | N | M | plan-04, plan-05 | glue + objects | yes (fields `h`, `adduct`) | RDKit `ExactMolWt` |
| 29 | arch-std-17 | export default, Download, clipboard list | P | S | std-03 | `svg.rs` + page Download | no | Office SVG notes |
| 30 | arch-cd-03 | hover hotkeys | P | S | std-06 | input | no | velocity/1 |
| 31 | plan-14 | keep the core oriented | N | M | plan-13 | `scheme.rs` | no | RDKit `GenerateDepictionMatching2DStructure` |
| 32 | plan-10 | survival matrix of pictures | N | S | std-18 | `tools/survival` | no | independent hosts reopen |
| 33 | plan-11 | atom maps | N | M | std-11 | document atoms/bonds | yes (`atom.set map`) | RDKit maps |
| 34 | arch-std-10 | label display modes | P | S | std-03 | scene/style | no | goldens |
| 35 | arch-web-03 | host adapter | P | S | web-02 | page (element file) | no | — |
| 36 | arch-web-07 | selection (+ E3) | P | M | std-05 | input | no | — |
| 37 | plan-07 | peak list → explained cuts | N | M | plan-06 | glue | maybe (engine entry) | MassBank run-time rate |
| 38 | arch-web-13 | arrow kinds (+ cd-06) | P | S | — | document objects | yes (obj fields) | KET / CDXML |
| 39 | arch-std-16 | export profiles | P | M | std-01 | conform + svg sizes | no | journal pages read |
| 40 | arch-std-07 | page accessibility audit | P | S | UI-2 PR 3 | page css + e2e | no | WCAG values |
| 41 | arch-web-16 | CIP labels (+ E5) | P | M | plan-04 | scene labels + glue arm | no | RDKit `rdCIPLabeler` |
| 42 | arch-web-08 | chain (+ cd-05) | P | S | std-06 | input + op | agreed (D3) | velocity/1 |
| 43 | arch-web-05 | viewer and form | P | S | web-03 | page (element file) | no | WHATWG text |
| 44 | arch-std-19 | enhanced stereo in V3000, KET | P | M | std-12 | readers + writers | maybe (`mzlab-mol` reads them) | RDKit via plan-03 |
| 45 | plan-12 | `trasf/1` out | N | M | plan-11 | io (save arm) | maybe (constructor) | RDKit reaction; mzLab reader |
| 46 | arch-cd-25 | curved arrows, lone pairs | P | M | web-13 | document objects | yes (`lp`, `t:"curve"`) | `mechanism/1` |
| 47 | arch-web-17 | values (+ E6) | P | S | plan-04 | glue arm + outline | no | RDKit masses |
| 48 | plan-16 | agent proposal layer | N | M | plan-15, UI-2 PR 2 | overlay + page panel | no | bytes = apply of hunks |
| 49 | arch-web-09 | merge on drop | P | M | web-08 | input + op | yes (`merge`: mzLab has it, D1 table) | — |
| 50 | arch-web-11 | atom popover (+ cd-09) | P | M | CTX-MENU page | bridge + page | no | — |
| 51 | arch-web-18 | abbreviations (+ E8) | P | M | std-08 | label | yes (`abbr.*`) | molfile expanded = contracted |
| 52 | arch-web-19 | templates (+ cd-12/13) | P | M | std-06 | data + op | agreed (D3) | velocity/1 |
| 53 | arch-web-15 | highlight marks | P | M | — | document atoms/bonds | yes (`hl`) | KET |
| 54 | arch-std-21 | text runs (+ E15) | P | M | — | document objects | yes (`runs`) | KET, CDXML goldens |
| 55 | plan-08 | curved-arrow check | N | S | cd-25, plan-04 | glue arm + conform panel | no | `mol mech` of mzLab |
| 56 | arch-web-20 | bond kinds (+ E9) | P | M | web-15 | document bonds | agreed (D4); valence in `mzlab-mol` maybe | V3000 9/10, KET |
| 57 | arch-std-20 | enhanced stereo in CXSMILES | P | S | std-19 | readers | maybe (CXSMILES writer) | RDKit via plan-03 |
| 58 | arch-cd-08 | radical, isotope, periodic table | P | S | web-07 | input | no | — |
| 59 | arch-cd-15 | align, distribute, scale (+ E2) | P | S | std-15 | commands | no | geometry |
| 60 | arch-web-21 | drop and copy as (+ E7) | P | S | std-17, CTX-MENU page | page clipboard | no | — |
| 61 | arch-cd-18, arch-cd-19 | palette tear-off, then named layouts (two prompts, in order) | P | M + S | UI-2 PR 2 | `palettes.rs` + page | no | — |
| 62 | arch-cd-20 | context-menu refine | P | S | CTX-MENU page | `context.rs` | no | — |
| 63 | arch-cd-26 | history panel | P | S | — | editor + page | no | log |
| 64 | arch-cd-04 | double-bond position, dashed on `k` | P | S | web-20 | document bonds | yes (`pos`) | CDXML |
| 65 | arch-cd-16 | smart paste | P | S | web-21 | `editor_edit.rs` | no | — |
| 66 | arch-web-04 | host theme tokens | P | S | UI-2 PR 3, std-03 | scene screen palette | no | goldens unchanged |
| 67 | arch-cd-22 | desktop shell | P | L | web-02; owner's Tauri choice | new crate | no | headless smoke |
| — | arch-web-10, arch-web-12, arch-web-14 | clean-up, context menu, rich text | W | — | = std-15, CTX-MENU, std-21 | — | — | — |
| — | plan-09, plan-17, plan-18 | PDF, CDX core, CDX objects | W | — | = std-18, std-22 | — | — | — |
| — | arch-cd-01/02/05/06/07/09/10/11/12/13/14/17/21/23/24 | superseded | additions only | — | section 3 | — | — | — |

Deferred, no prompt: Markush / SAR (card 5, rule 13, owner); catalytic-cycle layout (after arch-cd-25 on paths); Explain-CIP trace (a `cip/1` change, COO 16); next mechanistic intermediate (a new mzLab engine, COO 16); font outlining in files (font licence); CDX writer (after std-22); live isomer re-ranking (after plan-07's numbers); automatic atom mapping.
Dropped: Open Reaction Database export; freehand gesture recognition; 3D conformers and viewer; macromolecules; natural-language structure generation; OLE.

Totals:
- 67 rows to run: 8 running or waiting, 15 new prompts, 44 with existing prompts.
- Withdrawn: 6 prompts (arch-web-10, -12, -14; plan-09, -17, -18).
- 15 arch-cd prompts are reduced to notes of additions.
- Deferred: 8.

## 5. File zones
Two chats never write the same zone at the same time.

| zone | files |
|---|---|
| **document atoms/bonds** | `doc.rs` atom/bond fields; the atom/bond arms of `ops.rs`; the writers of those fields in `io.rs`, `ket.rs`, `cdxml.rs`, `cml.rs`, `rxn.rs` |
| **document objects** | `doc.rs` objects; the `obj.*` arms of `ops.rs`; object painting in `scene.rs` |
| **scene/style** | atom, bond and label painting in `scene.rs`; `settings.rs` style fields; `svg.rs` |
| **input** | `editor_tools.rs`, `editor_keys.rs`, `editor_touch.rs`, `editor_edit.rs` |
| **readers** | `io.rs` read paths (a new reader module is its own zone) |
| **bridge** | `crates/mzdraw-wasm/src/lib.rs` and other bridge files except `glue.rs` |
| **glue** | `crates/mzdraw-wasm/src/glue.rs`: one engine arm per feature is append-only |
| **page** | `web/index.html`, `web/mzdraw.js`, `web/mzdraw.css` (the element file `web/mzdraw-editor.js` is its own zone) |
| **conform** | `conform.rs` |
| **label** | `label.rs` |
| **layout** | `layout.rs` (`scheme.rs` is its own zone) |
| **tp** | `crates/mzdraw/src/tp.rs` (tp-03, tp-04, tp-10, in that order; §11) |
| **pathway** | `crates/mzdraw/src/pathway.rs` (tp-05, then tp-07 for the label room; §11) |
| **tp-list** | `crates/mzdraw/src/tp_list.rs`: `tp:import`, `tp:number` (tp-08; §11) |
| **tp-table** | `crates/mzdraw/src/tptable.rs` (tp-09; §11) |
| **tools** | `tools/<own dir>`, a new `tests/*.mjs` |
| **new module** | one new file in `crates/mzdraw/src` |
| **new modules of §9–§10** | each its own zone: `outcome.rs` (plan-20), `stereo_guard.rs` (plan-21), `loss.rs` (plan-22), `digest.rs` (plan-23), `select.rs` (plan-24), `plan.rs` (plan-27), `grant.rs` (plan-28), `chain.rs` (plan-30); `tools/agent-suite` (plan-19) |

Append-only lines any chat may add next to another chat (merge = keep both, then rerun):
- the Built state bullets of `docs/mzdraw.md`;
- `measured` in `tools/wasm-cap.toml`;
- keys in `web/lang/mzdraw.js`;
- cases in `tests/data/mzdraw_log.json`;
- `mod` lines;
- one match arm in `Op::from_value` / `Op::write` / `Editor::save` / `glue::run`;
- one row in the plan-01 table.
- one `touches` arm in `ops.rs` (from plan-28, E25).

The e2e step count is counted from the file and is never edited by hand.

## 6. Wave plan
Waves are ordered batches of at most 8 prompts whose zones do not collide; they are not launched as whole batches.

Launch rules (owner and coordinator, 2026-11-21):
- At most 10 chats open in total, including the running ones.
- Launch 2–3 prompts at a time as slots free up, in the order listed inside the wave.
- No new chat while 6 green PRs wait for review.
- Chats run Sonnet at medium effort and hand off at about 150k tokens.
- A prompt marked "yes" for COO 16 waits for that agreement; the next prompt marked "no" in the same wave takes its slot.

Wave 0 (running now): UI-2 (R1, PR 2, PR 3), CTX-MENU page, arch-std-01, -05, -08, -14.

| wave | starts when | items, in launch order |
|---|---|---|
| **1** | now | plan-01, plan-04, plan-02, plan-03, arch-std-22, arch-std-18, plan-15, arch-std-09, plan-19, plan-20 |
| **2** | std-01, std-05, UI-2 PR 2 merged | arch-std-02, arch-std-15, plan-05, arch-std-12, arch-std-06, arch-web-01, plan-13, arch-std-03 (after UI-2 PR 3), plan-23 |
| **3** | the wave's dependencies merged; first `velocity/1` report in | plan-06, arch-std-11, arch-web-02, arch-std-17, arch-cd-03, plan-14, arch-std-13, plan-10, plan-21, plan-22, plan-30, plan-26 |
| **4** | as above | plan-11, plan-07, arch-web-07, arch-web-13, arch-std-16, arch-std-10, arch-web-03, arch-std-07, plan-24, plan-25, plan-27, plan-28, plan-31 |
| **5** | as above | arch-web-16, arch-web-17, arch-web-08, arch-std-19, plan-12, arch-cd-25, arch-web-05, plan-29 |
| **6** | as above | plan-16, arch-std-21, arch-web-15, arch-web-18, arch-web-19, arch-web-09, arch-web-11, plan-08 |
| **7** | as above | arch-web-20, arch-std-20, arch-web-21, arch-cd-15, arch-cd-08, arch-cd-18, arch-cd-19, arch-cd-20 |
| **8** | as above | arch-cd-26, arch-cd-04, arch-cd-16, arch-web-04, arch-cd-22 |

Waves 1, 3 and 4 hold more than 8 items since §9 and §10: they are ordered lists, launched 2–3 at a time under the launch rules, not batches.

Same zone, serialize: inside a wave, launch one prompt per zone at a time.
- Wave 4: arch-std-16 and arch-std-10 both touch `svg.rs` / scene.
- Wave 5: arch-std-19 and plan-12 both touch `io.rs`.
- Wave 6: plan-16, arch-web-11 and plan-08 all touch the page (`web/index.html`, `web/mzdraw.js`).
- Wave 7: arch-web-21, arch-cd-18 and arch-cd-19 all touch the page; arch-cd-19 also follows arch-cd-18 in `palettes.rs`.

Order inside a wave puts the analytical edge and the agent route early (plan-04, plan-05/06, plan-15) without starving drawing speed and file fidelity (std-22, std-18, std-15, cd-03). After the first `velocity/1` report, the coordinator may move the largest-gap items (hover hotkeys, chain, templates) one wave earlier; their zones allow it.

## 7. Edits to existing prompts
These are listed here, not rewritten. Each chat applies its edit when it starts.

1. **E1 arch-std-06:** take arch-cd-17's per-target table (canvas, atom, bond), its reserved keys and its printable table. arch-cd-03 fills the command table later.
2. **E2 arch-cd-15:** drop Alt+Arrow, Shift+Alt+Arrow and plain-arrow nudging (arch-std-05 owns them). Scaling uses `atoms.xy` (arch-std-15).
3. **E3 arch-web-07:** do not bind Shift+Arrow (arch-std-05).
4. **E4 arch-std-15:** add arch-cd-01's acceptance (isomeric SMILES identical before and after; wedge meaning and E/Z kept, `ez_mismatches`) and arch-web-10's "clean the selection only". The op is `atoms.xy` (the one name; `place` is not used). It needs COO 16's agreement for the op.
5. **E5 arch-web-16:** compute through `cip/1` in `glue.rs` (plan-04). The native example `replay_glue` gives the same bytes. Add arch-cd-02's `"cip"` request field. Independent check: RDKit `rdCIPLabeler` on the plan-03 set.
6. **E6 arch-web-17:** numbers from `selection/1` (`mzlab-feat` feature `sel`) through the glue; there is no mass table in `mzlab-mol`. Remove "ask COO for masses in mzlab-mol". Check: RDKit `ExactMolWt` reference values from plan-03.
7. **E7 arch-web-21:** build "Copy as" on arch-std-17's payload list in Rust; "copy as KET" is in the chooser, with no `Ctrl+Shift+K`.
8. **E8 arch-web-18:** reuse arch-std-08's label-unit tokenizer for composed labels; the op names are `abbr.expand` / `abbr.contract`.
9. **E9 arch-web-20:** one field `k` with arch-cd-04's dashed kinds; V3000 types 9 and 10 from arch-cd-23. A dative bond's valence meaning is `mzlab-mol`'s: if it is missing at the pin, list the gap for COO 16.
10. **E10 arch-cd:** done. `arch-cd-20-context-menu-refine.md` exists on `claude/arch-cd` (added at `0733735`; PR #41 head `87520eb`), and arch-cd applied E2, E4 and E9 to its own prompts.
11. **E11 every prompt that adds a `draw_edit/1` op or field:** add its row to the plan-01 table and get COO 16's agreement before the PR (the ones in section 8 are already agreed).
12. **E12 every prompt whose feature calls an engine:** go through `glue.rs` (plan-04), never call `mzlab_feat` from `lib.rs` directly.
13. **E13 every prompt:** hot files (`docs/mzdraw.md` Built state, `measured` in `tools/wasm-cap.toml`) conflict in every PR. Merge `origin/main` only on a real conflict, rerun, push. Do not edit the e2e step count, which is counted from the file now. Hand off at about 150k tokens with a pushed WIP branch and a one-line state note (replaces "200k").
14. **E14 arch-std-15, -16, -17, -18: name the CLI route** (coordinator's review of PR #45). Each must state its replay request and its `mzlab-cli mol draw` route, and add a native replay test in `crates/mzdraw/tests/replay.rs` that gives the page's bytes:
    - std-15: `{"command":"clean"}`, with `"select":[ids]` for a part;
    - std-16: `"set":["export.profile=elsevier","export.width=single"]` with `"out":"std"` and `"out":"svg"`;
    - std-17: `"set":["export.background=white"]` with `"out":"svg"`, and `"out":"clipboard"` for the payload list (MIME and base64);
    - std-18: `"out":"pdf"` (base64, `--raw` writes the file).
15. **E15 arch-std-21:** add arch-cd-07's symbol picker (a Rust list, the page paints it) if it fits in the slice; otherwise list it as not built.
16. **E16 arch-std-18:** add plan-09's points:
    - deterministic bytes (no dates; `/ID` a hash of the content; numbers through `mzlab_mol::fixed`);
    - the V3000 molfile as a second embedded file;
    - `qpdf --check` and `pdfdetach -list` (poppler) on the goldens, from a `tools/` script;
    - a fuzz-style test of the reader.
17. **E17 arch-std-22:** add plan-17's proof. `tools/fidelity/cdx.py` has Open Babel (`obabel -icdx`) read the same synthetic CDX files, and the committed reference compares InChIKeys with ours. Synthetic CDX files are written by a small writer in the test from the documented layout, never by another program. If one slice cannot carry the text and graphic objects, split them into a second PR.

## 8. Decisions (21 Nov 2026)
Answers of the coordinator (COO-MZDRAW) and of COO 16 to the questions of the draft:

| # | question | decision | by |
|---|---|---|---|
| D1 | who owns `draw_edit/1` | One op table in `crates/mzdraw/src/schema.rs`, served as JSON. A test checks that mzLab's 7 ops are in it. mzLab adopts the table in its integration PR, and mzLab's `edit.rs` does not change before then. | COO 16 |
| D2 | engines in the native replay | The glue lives in `crates/mzdraw-wasm/src/glue.rs`, with a native example there, so parity covers the engines and `mzdraw` stays free of `mzlab-feat`. | COO-MZDRAW |
| D3 | chain and template in the log | Semantic ops `chain` and `tmpl`. | COO 16 |
| D4 | bond kinds | One field `k` (bold, dashed, dative, hbond), written as V3000 types 9 and 10. | COO 16 |
| D5 | real public corpora | A `tools/` script fetches them at run time and commits only rates and reference outputs for our synthetic inputs. Fixtures stay synthetic (rule 4). | COO-MZDRAW |
| D6 | binary CDX reader | Kept now; the owner wants mzDraw to read all files. It runs as arch-std-22. | COO-MZDRAW |
| D7 | PDF writer | In `mzdraw`, from its display list. It runs as arch-std-18. | COO 16 |
| D8 | Ketcher in the benchmark | A pinned standalone Ketcher, fetched at run time in `tools/velocity`, outside CI. | COO-MZDRAW |
| D9 | integration with mzLab | `mzlab-cli` calls `mzdraw::replay`, so the KET and CDXML writers exist once. This is done in the integration PR, with site (wasm) and app/CLI parity tests. If adoption would raise any other mzLab module cap or the catalog guards (33,000 B / 25,600 B), stop and tell COO 16 first. | COO 16 |
| D10 | launch limits | At most 10 open chats; launch 2–3 at a time; no new chat while 6 green PRs wait; Sonnet medium; handoff at about 150k tokens. | owner, via COO-MZDRAW |
| D11 | withdrawals | arch-web-12 is withdrawn; arch-cd-15 drops its Alt keys; 15 arch-cd prompts are reduced to "additions" notes; the defer and drop lists of section 4 are agreed. | COO-MZDRAW |
| D12 | desktop app | One component, two distributions (standalone macOS + Windows app; bundled by default in mzLab site and app via Tauri). The shell is separate for now, uses the shared `release.yml` pattern, and signing is a later step of the owner's. | owner, COO 16 |

## 9. Second Gemini input (ARCH PLAN-2, 2026-11-21)
Input: `docs/input/gemini-strategy-2.md` (Gemini Deep Research, PR #49). It proposes five ideas, three features not to build and an external validation protocol. Every claim was re-checked at its source on 2026-11-21. Pages read on github.com, iupac.qmul.ac.uk, arxiv.org, typst.app, patents.google.com, molsoft.com and en.wikipedia.org. Pages that refused access (HTTP 403) or are bot-walled are "not read".

### 9.1 Claims checked
| claim in the input | verdict | source read | note |
|---|---|---|---|
| Ketcher #5873: KET export increments internal ids on every call | verified | https://github.com/epam/ketcher/issues/5873 | Opened 2024, fixed by Ketcher PR #5916. |
| Ketcher #8898: the API drops selections in KET fragments | partly | https://github.com/epam/ketcher/issues/8898 | Selection is ignored by `setMolecule` / `addFragment`; "silently" is the input's word. Closed Jan 2026. |
| Ketcher #64: V2000 drops OR / AND enhanced stereo | partly | https://github.com/epam/ketcher/issues/64 | A 2021 comment says AND/OR centres are "not supported in V2000"; nothing about silent loss. |
| Ketcher #5152, #5156: an ACS-style setting with bond length | verified (partly on "1996") | https://github.com/epam/ketcher/issues/5152 ; https://github.com/epam/ketcher/issues/5156 | "ACS style"; the year 1996 is not in the text read. |
| Ketcher #6377, #7649: undo bugs in macromolecule mode | verified (undo), not "regression" | https://github.com/epam/ketcher/issues/6377 ; https://github.com/epam/ketcher/issues/7649 | Both closed (2026-04, 2025-10). |
| RDKit #4041: random R/S because wedges need coordinates and tags depend on bond order | partly | https://github.com/rdkit/rdkit/issues/4041 | A question closed in 2021; the comments explain the dependence; no bug confirmed. |
| RDKit #5067: one wedge only on quaternary centres | verified | https://github.com/rdkit/rdkit/issues/5067 | An opt-in option was added (RDKit PR #6108), not the default. |
| RDKit #1286: depiction matching moves core coordinates in clean-up | verified | https://github.com/rdkit/rdkit/issues/1286 | Confirmed by the maintainer; closed by the stale bot (2025), not fixed. |
| RDKit #8124: depiction matching flips the orientation | verified | https://github.com/rdkit/rdkit/issues/8124 | Closed by the stale bot (2026), not fixed. |
| RDKit #6626: overlapping atoms in bridged bicyclics | partly | https://github.com/rdkit/rdkit/issues/6626 | A hard-to-read depiction; overlap is not stated. |
| RDKit #4504: overlapping coordinates | partly | https://github.com/rdkit/rdkit/issues/4504 | Atoms of 1,1-disubstituted cyclobutanes; labels not mentioned. Fixed. |
| RDKit #4508: S-group labels overlap atoms | verified | https://github.com/rdkit/rdkit/issues/4508 | Closed 2021. |
| RDKit #9354: CDXML overlap | **wrong** | https://github.com/rdkit/rdkit/issues/9354 | It is about NeedsClean atoms in CDXML reading. |
| Indigo #2739: stereo data leaks across CDXML products | verified | https://github.com/epam/Indigo/issues/2739 | Fixed by Indigo PR #2740. |
| Indigo #2810: overlaps after CDXML import | partly | https://github.com/epam/Indigo/issues/2810 | Rearrangement causing overlap, from hidden superatom atoms. |
| CDK #1292: depiction overlap | **wrong** | https://github.com/cdk/cdk/issues/1292 | SMARTS ring-size matching. |
| CDK #1282, #951 | not read | https://github.com/cdk/cdk/issues | The input links the issue list, not the issues. |
| Open Babel #2015: 2D overlap | **wrong** | https://github.com/openbabel/openbabel/issues/2015 | A request to adopt coordgen. |
| IUPAC 2006 rules ST-1.1.1, ST-1.1.2 | verified | https://iupac.qmul.ac.uk/drawing/stereo.html | Numbering ST-1.1.1 to ST-1.1.20. |
| IUPAC 2006 ST-1.1.9 / ST-1.1.10 call some wedge sets ambiguous | not read | same page | The page was read only in part; seen only in a summary. |
| IUPAC 2008 GR-0.3 forbids shrinking to illegible size | **wrong rule** | https://iupac.qmul.ac.uk/drawing/drawing.html | GR-0.3 is "Text" (under 5 pt not acceptable). The no-shrink rule is **GR-0.6**, which points to abbreviations (GR-2.2, GR-2.3) as the alternative. |
| ACS Document 1996 bond length 0.508 cm (14.4 pt) | verified | https://en.wikipedia.org/wiki/Wikipedia:Manual_of_Style/Chemistry/Structure_drawing | An essay page, not a guideline. |
| ACS single column 82.5 mm | not read | https://pubs.acs.org/page/4authors/submission/graphics_prep.html | HTTP 403. arch-std-16 has the ACS widths it read. |
| ChemDraw 23.0 release notes (canvas atom ids, monomers, retrosynthesis removed) | not read | the input's PDF link | HTTP 403. |
| ChemDraw 25.5 alignment regressions | not read | Reddit | Bot wall. |
| LiveFigure (arXiv 2605.23527): VLM agents for figures | partly | https://arxiv.org/abs/2605.23527 | Real (Shao et al., 22 May 2026). Screenshot loops and token cost are not in the abstract. |
| IChO 2025 multi-agent (arXiv 2511.16205) | verified | https://arxiv.org/abs/2511.16205 | "ChemLabs on ChemO", Nov 2025. |
| arXiv 2508.17692 reviews ChemCrow | partly | https://arxiv.org/abs/2508.17692 | An agent survey; ChemCrow is not in the abstract. |
| typed-smiles: abbreviations leave index gaps | verified | https://typst.app/universe/package/typed-smiles/ | Its own "Index gaps" note (v0.14.0). |
| ICM: stereo-specific SMARTS | verified | https://molsoft.com/~eugene/icm/release-notes.html | "S[123]" attribute, version 3.9-2d. |
| US12159693B2 | verified, not relevant | https://patents.google.com/patent/US12159693B2 | Genesis Therapeutics, generative ML of molecules. |
| Wilson 95% CI [99.82, 99.95] at N = 10,000 and 99.9%; ±6.3 pp at N = 240 | verified (computed) | — | Recomputed; ±6.3 pp is the worst case (p = 0.5). |
| KLM 40.2 s vs 3.5 s for a series revision | arithmetic correct | — | An estimate from assumed steps, not a measurement. |
| Novelty ("genuinely new") and patent clearance | not used | — | Keyword searches are not a prior-art or FTO search. |
| The batching penalty (77 s / 0.8 s for 901 calls) | ours, ratio corrected | — | Measured in mzLab. 77 / 0.8 is about 96×, not the 95× of the prompt. |

### 9.2 Findings of this pass
1. **Ids are already stable** in mzDraw (`atom.add` takes an `id`; ids never shift), so the input's main failure (indices that move after an edit) does not apply to us as stated. Two gaps remain:
   - the result of a replay does not report the ids it created and removed or which op failed, although every arch-std prompt states that rule;
   - an agent that imports a SMILES must read the whole document to learn the ids.
   plan-20 and plan-23 close these gaps.
2. **Exit codes** are mzLab's one table (`docs/agenti/cli-agenti.md` of mzLab, read at the pin `ccd4680`). The input's "exit code 12" is not used.
3. **Substructure** search in `mzlab-mol` (`substruct.rs`, `query.rs`) takes SMILES and molfile queries, not SMARTS. Selectors use SMILES only (rule 13).
4. **Flip** mirrors the configuration today. "Flip that keeps the configuration" is already listed as still to build (`docs/mzdraw.md`). plan-26 builds it with existing ops.

### 9.3 Verdicts
| idea | verdict | where | reason |
|---|---|---|---|
| 1 ChemSel selectors | **KEEP, reduced** → plan-24 | read-only `select/1` | Resolved to ids before the log, so inverses and parity are untouched. SMILES substructure, not SMARTS. Selectors inside ops are DEFER: they would put a field on every op. It starts only if plan-19 shows addressing errors. |
| 2 stereo-invariant geometry kernel | **REFRAME** → plan-21 (guard), plan-26 (`flip:keep`) | two slices | Report-only in the page, enforced only with `{"stereo":"kept"}`. Automatic wedge re-solving is DEFER. |
| 3 core-linked analog families | **DEFER**; part → E18 (plan-14) | — | A live master/instance link is a new document concept. Orienting a series on one core needs no new concept. Revisit after plan-14 numbers. |
| 4 Knuth-Plass abbreviation folding to fit a box | **DEFER**, no prompt | — | It depends on arch-std-16 (profiles), arch-web-18 (`abbr.contract`) and arch-std-15. The real basis is IUPAC GR-0.6. **Kill test** when revisited: on 30 synthetic large structures that do not fit an arch-std-16 width class at the minimum text size, contracting abbreviations (greedy) must make at least 50% fit; otherwise drop. |
| 5 transactional canvas | **KEEP, split** → plan-20 (envelope, dry run, postconditions), plan-22 (loss manifest), plan-23 (digest), plan-25 (fix-its) | replay request | The stdio server is DEFER: it would be a second server next to `mzlab-cli`. Batch and stdin streaming are already mzLab's contract. |
| Do-not 1 macromolecules / HELM | DROP (agree; already dropped) | — | Rule 13. |
| Do-not 2 3D, force field, NMR | DROP (agree; already dropped) | — | mzLab has 3D. |
| Do-not 3 cloud sync, CRDT multiplayer | **DROP** (agree, new) | — | No server; files and logs are deterministic, so git review covers async work. |
| Validation: public corpus round trip | **REFRAME** → E19 (plan-03) | — | Run-time fetch only (D5). |
| Validation: time-and-motion with chemists | **DEFER** (owner) | plan-02 person protocol | It needs at least 6 people with ChemDraw licences. |
| Validation: blind multi-model agent suite | **KEEP** → plan-19 | `tools/agent-suite` | It is the kill test of plan-20, plan-23 and plan-24. |

Counts, each idea once: ideas 1 and 5 kept (1 reduced to plan-24; 5 split into plan-20, -22, -23, -25), idea 2 reframed (plan-21, plan-26), ideas 3 and 4 deferred. Do-not list: 1 new drop, 2 drops already made. Validation: 1 kept (plan-19), 1 reframed (E19), 1 deferred.

### 9.4 Rows
Same columns as §4. "request row" in the COO 16 column means the item adds request fields or outputs of the `mol draw` contract: one row each in the `request` section of plan-01's table, which COO 16 reviews (D13); no PR-by-PR agreement is needed as long as no `draw_edit/1` op is added.

| # | id | item | status | size | depends on | zone | COO 16 | proof |
|---|---|---|---|---|---|---|---|---|
| 68 | plan-19 | agent task suite | N | M | plan-01 | `tools/agent-suite`, `tests/data/agent-suite`, new example | no | RDKit InChIKey of targets |
| 69 | plan-20 | replay result: ids, failing op, dry run, `expect` | N | M | plan-01 | `replay.rs` + new `outcome.rs` | request row | property on id sets |
| 70 | plan-21 | stereo guard (signature, live line, `expect` arm `stereo`) | N | M | plan-20 | new `stereo_guard.rs` | request row (`expect` arm `stereo`) | InChI stereo layer by RDKit |
| 71 | plan-22 | loss manifest `loss/1` | N | M | plan-20, plan-15 | new `loss.rs` | request row | plan-03 differences explained |
| 72 | plan-23 | digest `digest/1` | N | S–M | plan-20 | new `digest.rs` | request row | RDKit formula |
| 73 | plan-24 | selectors `select/1` | N | M | plan-20, plan-23, first plan-19 run | new `select.rs` | request row | RDKit match counts |
| 74 | plan-25 | exact fix-its on `std/1` | N | S | std-02, std-15, plan-20 | conform | request row (`std/1` field `fix`; `std/1` goldens move) | isomeric SMILES unchanged (RDKit) |
| 75 | plan-26 | `flip:keep`, with the rule for a selection that cuts a stereocentre | N | S | plan-21 | input (one command) + `context.rs` arm | no | InChI stereo layer by RDKit |

Deferred, no prompt, added to §4's list:
- live analog families (master/instance);
- abbreviation folding to fit a box (its kill test is in §9.3);
- automatic wedge re-solving;
- selectors inside ops;
- the stdio server;
- the time-and-motion study with chemists.

Dropped, added: cloud sync and CRDT multiplayer.

Totals after this section: 75 rows to run, 23 new prompts in all (8 here).

### 9.5 Edits to existing prompts
18. **E18 plan-14:** besides "each structure after an arrow on the one before it", add `scheme:orient-series`: every structure of a selection is oriented on ONE chosen core (the first selected), with the same rules (rigid fit, flip only without wedges, no clean-up, refuse on overlap). It is one undo step and has the CLI twin `{"command":"scheme:orient-series"}`. This is Gemini idea 3 without live links. Before applying it, the plan-14 chat asks the coordinator whether ARCH TP-DRAW (transformation products, arrow auto-layout, tables; prompts `tp-NN`) already covers series orientation; if it does, E18 is dropped in favour of that prompt. Check it against RDKit `GenerateDepictionMatching2DStructure` on the same synthetic series: core atoms must keep their exact coordinates (RDKit #1286 and #8124 are the cases to beat).
19. **E19 plan-03:** an optional `--corpus` run of `tools/fidelity/run.py` fetches a fixed public subset at run time (PubChem or ChEMBL ids listed in the script, never the structures). It commits only the success rates with a Wilson 95% interval, the source, the version and the date (D5). The synthetic set stays the test.

### 9.6 Waves
- **Wave 1 tail** (after plan-01): plan-19, plan-20 (coordinator, 2026-11-21).
- **Wave 2:** plan-23.
- **Wave 3:** plan-21, plan-22 (after plan-15), then plan-26 (after plan-21).
- **Wave 4:** plan-24 (only if its start condition holds), plan-25 (after std-02, std-15).

None of these touch the page except plan-21's live-line key and plan-26's one command (the `input` zone). They do not collide with the running zones; serialize plan-20 with any other prompt that edits `replay.rs`.

### 9.7 Decisions (21 Nov 2026)
| # | question | decision | by |
|---|---|---|---|
| D13 | replay fields and outputs as `mol draw` contract items; exit code of a failed postcondition; does AD0a–AD6 define an envelope | Yes: each replay field or output is ONE row in the `request` section of plan-01's table; COO 16 reviews the table, not each PR, while no `draw_edit/1` op is added. A false postcondition is exit 1 (verdict) with the `draw-result/1` document on stdout; exit codes stay 0..8. mzLab has no envelope yet: `draw-result/1` of plan-20 is the source of truth and `mzlab-cli` follows it. The caps of D9 still apply: no mzLab module cap or catalog guard (33,000 B / 25,600 B) rises without telling COO 16 first. | COO 16, via COO-MZDRAW |
| D14 | external models on the agent suite | The owner may run them outside CI; keys stay outside the repo; only rates are committed. | COO-MZDRAW |
| D15 | selector syntax | A JSON object, no mini-language. | COO-MZDRAW |
| D16 | stereo guard | Report-only in the page; enforced only when a request lists `{"stereo":"kept"}`. | COO-MZDRAW |
| D17 | abbreviation folding | A deferred row with its kill test, no prompt. | COO-MZDRAW |
| D18 | waves | plan-19 and plan-20 go in the wave 1 tail, after plan-01. | COO-MZDRAW |

## 10. Third and fourth external reports (ARCH PLAN-2, second pass, 2026-11-21)
Input: two more external research outputs. They are **not in the repository**: this section works from the coordinator's summaries (COO-MZDRAW, 2026-11-21) and does not quote them. Their novelty and patent claims are not evidence. Their figures (among them an "Open Babel 37%") are not verified and are used nowhere.

### 10.1 Sources checked for this pass
| claim or fact used | verdict | source read | note |
|---|---|---|---|
| V2000 chiral flag: on = a single isomer of known absolute configuration at every wedged centre; off = the drawn isomer and its enantiomer | verified | BIOVIA, CTfile Formats, Chemistry 2026, https://www.3ds.com/assets/invest/2026-01/biovia_chemistry_2026_ctfile_formats.pdf | Read on 2026-11-21 as text extracted locally (`pdftotext`); the fetch tool could not render the PDF. Counts line: "0=not chiral, 1=chiral". This public copy needs no request form: arch-std-19's note "behind a request form, not read" is corrected in that prompt. |
| V3000 collections `MDLV30/STEABS` (absolute), `STERACn` (relative configuration known, mixture of both enantiomers), `STERELn` (relative known, one enantiomer, unknown which) | verified | same | Collections take precedence over the chiral flag; wedged centres in no collection take the flag "globally". |
| IUPAC 2006 ST-6: no marks = unknown; mixtures said in words; the chiral-flag behaviour is not followed | verified (earlier) | `docs/study/arch-std.md` §2.2 (second pass) | Read for arch-std-12. |
| FIPS 180-4 Secure Hash Standard | partly | https://csrc.nist.gov/pubs/fips/180-4/upd1/final | Published August 2015. The landing page does not name SHA-256 (the standard does). The example vectors were not read: plan-30's DEV reads them. |
| `mzlab-mol` links `sha2` 0.11 and exposes only `Mol::core_hash` (core JSON, without coordinates or ids) | verified | `crates/mzlab-mol/Cargo.toml`, `src/codec.rs` at the pin `ccd4680` | So SHA-256 is already in the module. There is no public hash of bytes. |
| mzLab records runs with SHA-256 fields (`prov/1`) | verified | mzLab `docs/schemi/prov-1.json` at the pin | — |
| InChI `/s1`, `/s2`, `/s3` and the "absolute" flag | **not verified** | https://www.inchi-trust.org/technical-faq-2/ (301, then 404) | `docs/study/arch-std.md` has `/s1` and `/s2` only. `/s3` and "absolute" are not in `docs/study`; plan-29's proof says so. |
| mzDraw has FNV-1a only (SVG element names) | verified | `crates/mzdraw/src/svg.rs` | Not a hash for provenance. |

### 10.2 Verdicts
| idea | report | verdict | where | reason |
|---|---|---|---|---|
| `plan`: one goal in, a canonical reversible patch out, under a step budget | third | **KEEP** → plan-27 | `goal`, `budget` | Fewer calls for agents. It uses table ops only, so inverses and parity hold. Goal kinds are gated on their engines (templates, scheme, selectors); an unsupported kind is an explicit error. |
| `grant`: scope envelope checked per op, one transaction | third | **KEEP** → plan-28 | `grant` | Not covered: plan-20 checks the end of a log, not what each op touches. The page side waits for plan-16 (wave 6). |
| per-op drawing contracts | fourth | **MERGE** into plan-28 (`grant.each`) | — | These are plan-20's `expect` arms run after each op. No new op. |
| stereo tier per centre, never defaulted silently | fourth | **KEEP** → plan-29 | derived from `stg` | arch-std-12 stores groups. The gaps are a wedged centre with no group and a reader that would assume a tier from the chiral flag. The rule is "noted, never assumed" (D19). No new field, no new loss kind. |
| provenance ledger | fourth | **KEEP, two slices** → plan-30 (chain, `expect` arm), plan-31 (head in pictures) | `chain/1` | Cheap given byte parity. SHA-256 is already linked through `mzlab-mol`. |
| stable handles / selectors | third, fourth | = plan-24 | — | Already planned. |
| loss ledger | third | = plan-22 | — | Already planned. |
| public conformance suite, expert-verified corpus | third, fourth | = plan-03 (+ E19) and plan-19 | — | Only gold files and invariants, fetched at run time (D5). |

### 10.3 Rows
Same columns as §4 and §9.4.

| # | id | item | status | size | depends on | zone | COO 16 | proof |
|---|---|---|---|---|---|---|---|---|
| 76 | plan-27 | goal → canonical patch (`goal`, `budget`) | N | M | plan-01, plan-20 (E21); kinds gated (plan-13, web-19/18, plan-24) | new `plan.rs` | request row (`goal`, `budget`; key `patch` and stage `plan` via E21) | RDKit canonical SMILES |
| 77 | plan-28 | scope grant + per-op contracts | N | M | plan-20; plan-24 optional | new `grant.rs` + `touches` arms in `ops.rs` + `class` column in `schema.rs` | request row (`grant`, class column, stage `grant`) | property: bytes outside the scope unchanged; `touches` checked against real differences |
| 78 | plan-29 | stereo tier: noted never assumed, `tierUndefined`, digest `centres` | N | S–M | std-12, std-19 (E20), plan-22, plan-23 | readers (`io.rs`), conform, `digest.rs` | request row (`expect` arm, digest key `centres`) | InChI stereo flags by RDKit (meaning not verified) |
| 79 | plan-30 | hash chain of the log, `expect` arm `chain` | N | S | plan-20 | new `chain.rs` + one replay arm | request row (`out:chain`, arm `chain`); `sha2` direct dependency decided YES (D25; reason and wasm cost in the PR, guard in `docs/mzdraw.md` updated) | FIPS 180-4 examples; Python `hashlib` |
| 80 | plan-31 | chain head in saved SVG and PNG, `export.chain` | N | S | plan-30, std-16, std-17 | `svg.rs`, `png.rs`, settings table | request row (setting and output element) | head recomputed from the goldens |

The page part of plan-28 (a proposal held under a grant) is not built in row 77. It becomes a follow-up row once plan-16 (wave 6) merges.

Totals after this section: 80 rows to run, 28 new prompts in all (13 from PLAN-2).

### 10.4 Waves
- **Wave 3:** plan-30. It only touches a new module and one replay arm, so it does not collide with its wave-3 neighbours arch-std-17 (`svg.rs`) and arch-std-13 (scene).
- **Wave 4:** plan-27, plan-28 (both after plan-20; plan-28 after plan-24 if its selector scope is wanted), plan-31. plan-31 touches `svg.rs`: it runs after arch-std-17 (wave 3), and arch-std-16 and arch-std-10 (also `svg.rs`, wave 4) are serialized with it.
- **Wave 5:** plan-29 (after arch-std-19 and plan-22). It touches `io.rs`, so serialize it with arch-std-19 and plan-12.

### 10.5 Edits to existing prompts
These are listed here, not rewritten. Each chat applies its edit when it starts (as §7).

20. **E20 arch-std-19:**
    - The readers never set `stg` from a default. A V2000 chiral flag (0 or 1) and a V3000 wedged centre outside every collection leave the centre without a group; the import notes list the centres, the flag and its CTfile meaning (D19).
    - The V2000 write rule of arch-std-19 is unchanged, and plan-29 adds none.
    - The CTfile 2026 edition is readable at the public URL of §10.1; the prompt's source line now says so (corrected in the prompt file in this PR).
21. **E21 plan-20:** `draw-result/1` gains:
    - the key `patch` (plan-27), placed before `out`;
    - the `failed.stage` values `plan` (plan-27) and `grant` (plan-28);
    - `goal` joins `report`, `dry_run` and `expect` in making the answer an envelope;
    - the exit-2 list (invalid request) gains "an unsupported goal kind", "a malformed grant" and "`style`/`set` under a grant without `settings`".

    It also gains the request `id` as its second key: see E26 (D27).

    There is no stage `verify`: the chain is checked with the `expect` arm `chain` (plan-30). If plan-20 has merged before these prompts run, each of plan-27 and plan-28 makes its own part of E21 in its PR.
22. **E22 plan-19:** the `--arm` feature list gains `plan` (plan-27) and `grant` (plan-28). The adversarial set `tests/data/agent-suite/adversarial.json` comes with plan-28.
23. **E23 plan-01:** the op table gains a `class` column (`geometry`, `atoms`, `bonds`, `objects`, `structure`). plan-28 adds it if plan-01 has merged; otherwise plan-01 includes it.
24. **E24 plan-23:** the digest gains the per-centre key `centres` (`[[atom id, tier]]`) with plan-29. The per-bond `stereo` entries are unchanged.
25. **E25 every prompt that adds a `draw_edit/1` op after plan-28:** add its `touches` arm in `ops.rs` and its `class` in the table, next to its inverse and its property-test case.
26. **E26 plan-20 (request id, D27):** the key table of `draw-result/1` gains `id`.
    - Position: second, right after `format`.
    - When present: always.
    - Value: the request's own `id` echoed as given (a string or an integer; any other type is an invalid request, exit 2). When the request has none, it is `n`, the 0-based index of the request in its batch. The caller passes `n` (`mzlab-cli` counts the lines of a JSONL stream); a single request is `n` = 0.
    - Effect: a batch of requests maps answer to request, line to line.
    - The JSON Schema file `docs/schemas/draw-result-1.json` lists it, and the golden key-order test covers it (`format`, `id`, `ok`, ...).
    - The request field `id` is one row in the `request` section of plan-01's table.
    - If plan-20 has merged before this edit is applied, the next prompt to touch `outcome.rs` (plan-27 or plan-28) makes it in its PR.

### 10.6 Decisions (21 Nov 2026)
| # | question | decision | by |
|---|---|---|---|
| D19 | stereo tier on import | Noted, never assumed: no reader sets a group from the chiral flag or for an uncollected centre; the import notes say what the file said. | COO-MZDRAW (review of #55) |
| D20 | new request fields and stages for plan-27, -28, -30 | `patch`, stages `plan` and `grant` go into plan-20's key table (E21). The chain is an `expect` arm, not a field or a stage. | COO-MZDRAW (review of #55) |
| D21 | agent-suite features | `plan` and `grant` join the `--arm` list (E22). | COO-MZDRAW |
| D22 | op classes | A `class` column in plan-01's table (E23); `geometry` = `move`, `rotate`, `flip` today. Each later op names its class when it is added: `atoms.xy` (arch-std-15) `geometry`, `merge` (arch-web-09) `structure` (E25). | COO-MZDRAW |
| D23 | `touches` | One arm per op, append-only; every later op-adding prompt adds it (E25). | COO-MZDRAW |
| D24 | digest key for tiers | A new per-centre key `centres` (E24), not a change to the per-bond `stereo` entries. | COO-MZDRAW |
| D25 | SHA-256 | `sha2` becomes a direct dependency of `mzdraw` (the version `mzlab-mol` already links, already in `Cargo.lock`), added to the dependency guard. plan-30 states the reason and measures the module. Never a second implementation; no request to COO 16. | COO-MZDRAW |
| D26 | rows of other ARCH chats | ARCH TP-DRAW (PR #54) adds its rows (tp-01..07) after #55 merges; PLAN-2 owns rows 76–80. | COO-MZDRAW |
| D27 | request id in `draw-result/1` | `draw-result/1` carries the request's `id`, or its index `n` in the batch when it has none, so a batch or JSONL of requests maps line to line (E26). | COO 16 (decision 4), via COO-MZDRAW |
| D28 | who owns the writers | mzDraw owns every writer (KET, CDXML, SVG, PNG, PDF); mzLab deletes its copies in the integration PR (refines D9). | COO 16, via COO-MZDRAW |
| D29 | how mzLab links mzDraw | `mzlab-cli` depends on `mzdraw` as a git dependency with a pinned `rev`, and a `[patch]` section keeps one copy of `mzlab-mol` in its build. | COO 16, via COO-MZDRAW |
| D30 | mzLab site | The site runs `mzdraw::replay` in mzDraw's own wasm module, under its guard of 4,000,000 B (`tools/wasm-cap.toml`). | COO 16, via COO-MZDRAW |

## 11. Transformation products (ARCH TP-DRAW, 2026-11-21)
Input: six ideas of an external LLM (Gemini) on drawing transformation products (TPs), given by the coordinator and treated as unverified. The owner's priorities are: (1) draw the TPs of a parent fast; (2) arrows that organise themselves, symmetric and well routed; (3) product tables. The sources, the overlap with this backlog and the verdicts are in `docs/study/tp-draw.md`. There are 11 prompts, `docs/prompts/tp-01..11-*.md`.

### 11.1 Rows
Same columns as §4, §9.4 and §10.3. "request row" has the meaning of §9.4 (D13).

| # | id | item | status | size | depends on | zone | COO 16 | proof |
|---|---|---|---|---|---|---|---|---|
| 81 | tp-01 | arrow anchors `from`/`to`; follow on commands; `commands` in the request | N | M | plan-01, plan-20 (E29) | document objects + `replay.rs` (`commands`) | **yes** (fields `from`, `to` on `obj.add`/`obj.set`, `mzlab:false`); request rows | brute-force clip sampler; parity |
| 82 | tp-02 | anchoring by hand: arrow tool, `pathway:anchor`, `pathway:route` | N | S | tp-01; CTX-MENU page | input + `context.rs` arms | request row | velocity/1 (three new arrow tasks) |
| 83 | tp-03 | `tp:derive`: copy the parent, keep its scaffold, maps and patch | N | M | tp-01, tp-02, tp-05 (settings), plan-11, plan-20; CTX-MENU page | new `tp.rs` | request row | RDKit mapped RXN, formula delta |
| 84 | tp-04 | diff lens, `lens/1`, `display.lens` | N | S | tp-03 | `tp.rs` + scene | request row (`lens`, `std.tp.unmapped`) | RDKit mapped sets |
| 85 | tp-05 | pathway layout core `pathway:tree`, properties P1–P14 | N | M | tp-01, plan-20 | new `pathway.rs` | request row (`pathway:tree`, `pathway.*`, `std.scheme.*`) | independent Python re-check of the properties |
| 86 | tp-06 | pathway layout on the page; Graphviz proof | N | S | tp-05, tp-02; CTX-MENU page | page + `palettes.rs`/`context.rs` arms + `tools/fidelity` | no | Graphviz `dot` ranks and crossings (optional) |
| 87 | tp-07 | arrow labels computed: Δ formula, Δ mass | N | S | tp-01, tp-05, arch-web-17; arch-web-13 optional; tp-10 for names | `pathway.rs` | request row | RDKit formula and `ExactMolWt` |
| 88 | tp-08 | `tp:import` of a TP list, `tp:number` | N | M | tp-01, tp-05, plan-20 (E29); tp-07 and plan-14 optional | new `tp_list.rs` | request row (`failed.rows`, reserved data names) | Python `csv` + RDKit canonical SMILES |
| 89 | tp-09 | TP table out (`tptable/1`, CSV), SDF round trip | N | S | tp-08, tp-07, arch-web-17 | new `tptable.rs` | request row | RDKit masses; S66 column names (run-time) |
| 90 | tp-10 | named steps, `sites/1`, derive by step | N | M | tp-03 | `tp.rs` + `label.rs` row | request row (`step`, `site`, `sites`, `steps`) | RDKit reaction SMARTS in the tool only |
| 91 | tp-11 | step field on the page (`Shift+T`, `[` `]` while open) | N | S | tp-10 (its kill test passed); CTX-MENU page | input + page + overlay | no | velocity/1 |

Deferred, no prompt: V3, the uncertain L3 TP drawn as a site mark mapped onto `trasf/1`'s uncertain form (after plan-12); the "metro" mode of tp-05 (polyline routing, lane pins, y by Δm/z or RT, after tp-06); a TP table drawn on the canvas (after tp-09).
Dropped: arrow width or dash from peak areas (V4); the rigidity slider (V1); the journal preset compiler (V6, covered by arch-std-16).

Totals after this section: 91 rows to run, 39 new prompts in all (11 from TP-DRAW).

### 11.2 Waves
- **Wave 2 tail** (after plan-01 and plan-20): tp-01.
- **Wave 3:** tp-02, then tp-05 (both after tp-01), then tp-06. tp-02 and tp-06 touch the page and `context.rs`: serialize them with each other and with the CTX-MENU page PR.
- **Wave 4:** tp-03 (after plan-11), then tp-04; tp-07 (after arch-web-17 and tp-05).
- **Wave 5:** tp-08, then tp-09.
- **Wave 6:** tp-10, then tp-11 (only if tp-10's kill test passes). tp-11 touches the page: serialize it with plan-16, arch-web-11 and plan-08.

tp-01 is the only TP prompt that needs COO 16's agreement before its PR (two op fields). The others add request rows only (D13).

### 11.3 Edits to existing prompts
These are listed here, not rewritten. Each chat applies its edit when it starts (as §7).

27. **E27 plan-13:** the request has two shapes for commands, with one meaning: the top-level singular `"command"` + `"args"` (plan-13's shape) is a `"commands"` list of length one (tp-01), and a request with both is invalid (exit 2). Rules for `scheme:row`:
    - it sets no end of an arrow that has `from`/`to` (tp-01), and calls tp-01's `route` for it; free arrows keep plan-13's own rule;
    - plan-13's argument `gap` (1.0) is not tp-05's setting `pathway.col_gap` (2.5), and the names stay apart;
    - if plan-13 merges before tp-01, tp-01 makes the `route` change and adds `commands` next to plan-13's rows.
28. **E28 plan-14 (amends E18; E18 stays, D32):** series orientation is not covered by the TP prompts.
    - tp-03 inherits the parent's coordinates, so a TP made by `tp:derive` needs no orientation.
    - tp-05 only moves whole structures.
    - E18 (`scheme:orient-series`) stays for structures drawn or imported separately; tp-08 imports SMILES, whose computed coordinates are not oriented, and runs `scheme:orient` on each parent → TP pair once plan-14 exists.
    - plan-14 rotates and flips; it never places structures (that is tp-05's).
29. **E29 plan-20:** `draw-result/1` gains the following, with the key-table rows and the JSON Schema `docs/schemas/draw-result-1.json` updated with them:
    - **stage `command`**: `failed.stage` takes `command` next to `log`, `select` (plan-24), `plan` and `grant` (E21). With stage `command`:
      - `failed.op` is the 0-based index in the request's `commands` (or 0 for the singular `command`), not an index in `log`;
      - `failed.key` is the command's refusal key;
      - `failed.id` is the id it is about (0 when none).
    - **exit 1** (a verdict, with `draw-result/1` on stdout) for a command that is refused for this document: a cycle, a site that does not fit, a bad row. An unknown command or an unknown argument is an invalid request (exit 2).
    - **key `failed.rows`** (tp-08): an array `[{"line","key"}]` in line order, listing every failing row of a `tp:import`. It is placed last among the keys of `failed`, after `candidates`. It is present only for stage `command` when the command reads rows.
    - **atomic** (D31): a refused command writes nothing and gives no `out`, as plan-20 states for the log.
    - **`id`**: it stays the second key of the envelope (E26). These keys change nothing in its position.

    If plan-20 has merged before tp-01 runs, tp-01 makes E29 in its PR; else plan-20 includes it.

### 11.4 Decisions (21 Nov 2026)
| # | question | decision | by |
|---|---|---|---|
| D31 | TP import with failing rows | Atomic: nothing is applied and there is no `out`; `failed.rows` lists every failing row. There is no partial import. | COO-MZDRAW (review of #54) |
| D32 | does TP-DRAW cover plan-14's series orientation (E18) | No. TP-DRAW covers TPs made by derive. E18 stays for imported structures and series, and tp-08 calls `scheme:orient` once plan-14 exists (E28). | COO-MZDRAW (review of #54) |
