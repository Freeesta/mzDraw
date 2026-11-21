<!-- Input document, not a study of this repository: the raw output of an external research chat (Gemini Deep Research), converted from RTF to Markdown without edits. Every claim in it is UNVERIFIED until an architect re-checks it against the primary source. Kept as input for docs/plan. -->

**mzDraw Feature Discovery and Architecture**

*User prompt: Role: senior research lead in chemical-structure software and human-computer interaction. Task: discover which capabilities a 2D molecule drawing app is still missing or does poorly, and how to build the best of them. Depth, not length: I want few ideas, each with an evidence trail. Project (brief). mzDraw is a 2D molecule editor in Rust compiled to WebAssembly: runs in the browser, embedded in a mass-spectrometry/chemistry platform (mzLab) and as a desktop app (macOS, Windows). Hard constraints: no server, deterministic output (same commands = same bytes), CLI-native so AI agents can draw with JSON edit operations, exact undo for every operation, light footprint, synthetic data only. Already built: bonds, rings, labels, wedges, undo, clipboard, SMILES/molfile/SDF/KET/CDXML/CML/RXN import-export, 2D layout of structures without coordinates, live structure check, annotations of identification confidence (Schymanski levels), floating dockable palettes, right-click menus. Do not propose these. Method, in this order: 1. Start from raw material, not summaries or vendor marketing: public issue trackers (Ketcher on GitHub, Marvin and ChemDraw community forums), Chemistry StackExchange and Reddit threads, schemes and supporting-information files of open-access papers (PMC, ChemRxiv), patent drawings (USPTO/EPO), usability studies of structure editors (J. Chem. Inf. Model., J. Chem. Educ.), open datasets (USPTO reactions, Open Reaction Database, PubChem, ChEMBL, MassBank, GNPS). Quote or link each source. Say what you could not open. 2. Extract real user problems and the jobs people do (reaction schemes for papers, mechanisms, SAR tables, annotating fragments of spectra, teaching, reporting). Count how often each problem appears. 3. Only then propose capabilities: 8 at most, each as a card: problem and evidence (links, counts) \| the idea \| why existing editors fail at it \| how to build it under our constraints (algorithm, data, Rust/wasm, scriptable JSON operation for agents) \| smallest prototype to try first \| falsifiable hypothesis and the measure that would refute it (task time, clicks, error rate) \| scientific risk \| prior art and patent check (links) \| effort S/M/L. 4. Add a "hyper-creative" section: 3 bold ideas that break the usual editor model (for example: an editor that reasons about chemistry while you draw, drawing from instruments or spectra, a drawing that is also an executable query or protocol, collaborative or agent-native drawing). Each still needs a cheap experiment that could prove it wrong. 5. Say what you would NOT build and why, including ideas already tried elsewhere that failed. 6. Challenge my brief: list 3 better questions I should have asked. Proof must come from outside you: for each idea, name the independent check that would show it is correct (round trip against RDKit, Open Babel or the InChI reference; success rate on a named public dataset; timed task comparison against ChemDraw and Ketcher with 5-8 chemists and a SUS score; citation to a standard or peer-reviewed paper). Do not invent sources, numbers or version dates: if unsure, write "unverified". Output in English, as a ranked table followed by the cards, then a one-page list of the three experiments to run first.*

**Response: 1. Raw-Material Audit & Access Boundaries**

This audit sampled **34 primary web artifacts** across issue trackers, practitioner forums, peer-reviewed literature, standards, documentation, and open chemical datasets.

**Raw material inspected:**

- **Public issue trackers (9 items):** Ketcher GitHub issues covering S-group/mixture bugs (#12775, \#12772, \#12713), canvas centering (#2078), curved/elliptical arrows (#1045), reaction layout breakage (#1617), enhanced stereochemistry modals (#527), dative-bond valence semantics (#10427), functional-group abbreviation mutation (#1988), multi-tail arrows (#4861), R-group bracket protrusion (#197), and text copy hotkeys (#7152); plus Streamlit-Ketcher live-selection/edit-event gaps.

- **Practitioner forums & Q&A (12 items):** Reddit r/chemistry and r/Chempros threads on ChemDraw sluggishness and snapping regressions, standalone alternatives after 2026 academic license changes, hotkey/alignment rituals, Mac-to-Windows Word/PowerPoint OLE blurriness and silent rasterization, group-alignment regressions in ChemDraw 25.5, PowerPoint workarounds for "Clean Up Reaction", and immediate user feedback on open-source ChemDraw clone Octet-Draw; Chemistry StackExchange reports on bicyclic/bridged CIP failures, axial allene/biphenyl stereodescriptor contradictions, and manual "box" reaction arrow hacks; IONiC/VIPEr organometallic clean-up complaints; Boisvert lab DPI/Word clipping troubleshooting; ResearchGate HRMS adduct/fragmentation Q&A.

- **Peer-reviewed literature, preprints & standards (9 items):** IUPAC Graphical Representation Recommendations (Brecher, *Pure Appl. Chem.* 2006/2008); Open CIP discrepancy study (Mayfield, Lowe & Sayle, ACS 2017); chemical structure database error audit (Fourches, Muratov & Tropsha, PMC5657146); pharmaceutical patent Markush review (PMC13218012) and Markush OCSR benchmark (*J. Chem. Inf. Model.* 2026); Moodle JSME automated grading (*J. Chem. Educ.* 2026); OrganicPad tablet sketching evaluation; tRNA MS/MS fragmentation workflow using ChemDraw (*bioRxiv* 2025); MyCompoundID MS/MS ChemDraw tutorial; MassSpec Calculator Pro manual cleavage documentation.

- **Open datasets & platform docs (4 items):** MassBank (119,845 spectra / 18,529 compounds, PMC12807615); MassSpecGym (231,104 spectra / 29K molecules); Open Reaction Database schema (*JACS* 2021) and USPTO/FlowER/Reactron mechanistic corpora (*Chem. Soc. Rev.* 2026); ChemAxon Markush Editor user guide and 2026 licensing shifts (Marvin academic discontinuation, Marvin JS EOL Dec 2027, ChemSketch Freeware closure).

**What could not be opened:**

- Closed/proprietary Revvity Signals (ChemDraw) and Certara (ChemAxon/Marvin) customer bug trackers and gated support portals.

- Paywalled full-text PDFs and raw CDXML/Word supporting-information archives behind ACS/Wiley/Elsevier/RSC walls (only open abstracts, PMC full texts, bioRxiv preprints, and open-access articles were inspected).

- Raw USPTO/EPO patent drawing XML/TIFF archives and raw MassBank/ORD/PubChem bulk dumps (evaluated via published peer-reviewed analyses and dataset metadata).

- Formal freedom-to-operate patent databases beyond public Google Patents keyword queries; patent clearance in every card below is therefore marked where **unverified**.

**2. Extracted User Problems & Jobs-to-be-Done**

Across the **34 sampled primary artifacts** (several sources report multiple distinct failure modes), recurring problems cluster into six high-frequency jobs:

|                                                             |                                                                                                  |                                                   |                                                                                                                                                                                                                    |
|-------------------------------------------------------------|--------------------------------------------------------------------------------------------------|---------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Problem Cluster**                                         | **Primary Job-to-be-Done**                                                                       | **Observed Frequency in Audited Corpus (*N*=34)** | **Representative Evidence**                                                                                                                                                                                        |
|                                                             |                                                                                                  |                                                   |                                                                                                                                                                                                                    |
| **1. Cross-app vector fidelity & lost editability**         | Embedding figures into papers, theses, slides (Word, PowerPoint, Google Docs, LaTeX/LibreOffice) | **7 / 34**                                        | Mac/Win Word PDF blurriness, OLE corruption, Word auto-recovery rasterization, DPI clipping, Octet-Draw raster copy & re-editability requests, Ketcher copy bugs                                                   |
|                                                             |                                                                                                  |                                                   |                                                                                                                                                                                                                    |
| **2. Reaction scheme, arrow & catalytic-cycle layout**      | Multi-step synthesis schemes, manuscripts, graphical abstracts                                   | **7 / 34**                                        | Group-align broken in CD 25.5, PowerPoint spacing workaround, missing catalytic cycles, L-shaped "box" arrow hacks, Ketcher reaction layout (#1617) & multi-tail arrows (#4861), scaffold orientation alignment    |
|                                                             |                                                                                                  |                                                   |                                                                                                                                                                                                                    |
| **3. Stereochemistry, CIP & graphical ambiguity**           | Compound registration, structural reporting, publication integrity                               | **6 / 34**                                        | Bridged bicycloheptane CIP bug, axial allene/biphenyl *M*/*P* contradiction, Open CIP cross-tool mismatch, Ketcher Enhanced Stereo UX friction (#527), 0.1–.4% DB structure errors, IUPAC 2006/2008 rules          |
|                                                             |                                                                                                  |                                                   |                                                                                                                                                                                                                    |
| **4. MS/MS fragmentation & exact-mass annotation**          | Annotating HRMS/MS spectra, structural elucidation, reporting transformation products            | **5 / 34**                                        | Manual proton/adduct/radical cleanup in ChemDraw Mass Fragmentation, bioRxiv HRMS fragmentation figures, HRMS adduct/neutral-loss confusion, legacy MassSpec Calculator Pro, MassBank/MassSpecGym scale            |
|                                                             |                                                                                                  |                                                   |                                                                                                                                                                                                                    |
| **5. Markush, R-groups & SAR table synchronization**        | Patent claims, medicinal chemistry SAR reporting                                                 | **5 / 34**                                        | Markush graphical ambiguity & OCR failure, Ketcher R-group bracket & S-group bugs (#197, \#12775, \#12713), ChemAxon/Revvity enterprise lock-in                                                                    |
|                                                             |                                                                                                  |                                                   |                                                                                                                                                                                                                    |
| **6. Mechanistic curved arrows & inorganic/dative valence** | Reaction mechanism figures, catalysis, undergraduate/graduate teaching                           | **6 / 34**                                        | Ketcher decorative-only curved arrows (#1045), dative-bond valence bug (#10427), organometallic Clean Up failure, carbene lone-pair/pen-tool gaps, Moodle/automated grading demand, balanced arrow-pushing corpora |
|                                                             |                                                                                                  |                                                   |                                                                                                                                                                                                                    |

*Note on market timing (2026):* With ChemAxon ending free individual academic Marvin licenses in July 2026, Marvin JS reaching EOL in December 2027, and ACD/ChemSketch Freeware closing to new users after Revvity's January 2026 acquisition, the open browser/desktop market is actively fracturing—raising the payoff of shipping a crisp, zero-server Rust/WASM editor.

**3. Ranked Capability Table**

Capabilities are ranked by **(evidence severity × fit with mzDraw/mzLab architecture) / implementation risk**.

|          |                                                               |                                                 |            |                                                                                            |                                                                                                                   |
|----------|---------------------------------------------------------------|-------------------------------------------------|------------|--------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------|
| **Rank** | **Capability**                                                | **Primary Job**                                 | **Effort** | **Strongest Competitive Edge**                                                             | **Core Independent Proof**                                                                                        |
|          |                                                               |                                                 |            |                                                                                            |                                                                                                                   |
| **1**    | **MS/MS Semantic Cleavage & Exact-Mass Annotator**            | Annotating spectra & transformation products    | **M**      | Uncontested niche for mzLab; replaces manual radical/H-shift arithmetic                    | 95%+ formula/mass match (≤1 mDa) on 200 MassBank/MassSpecGym annotated fragments + timed task vs. ChemDraw        |
|          |                                                               |                                                 |            |                                                                                            |                                                                                                                   |
| **2**    | **Zero-OLE "Living Vector" Export (SVG/PDF/PNG)**             | Paper/thesis/slide publishing across Mac & Win  | **S**      | Eliminates \#1 daily ChemDraw pain (Word blurriness & lost editability) without OS plugins | 100% lossless byte-exact round-trip across 6 host apps (Word/PPT/Docs/LibreOffice/LaTeX/Typst) on macOS & Windows |
|          |                                                               |                                                 |            |                                                                                            |                                                                                                                   |
| **3**    | **Constraint-Based Reaction Scheme & Catalytic Cycle Layout** | Publication reaction schemes & ORD reporting    | **M**      | Aligns by MCS scaffold orientation & ring baseline, not naive bounding boxes               | RDKit MCS alignment score + ORD protobuf/JSON schema validator + 5– chemist timed task vs. ChemDraw/Ketcher       |
|          |                                                               |                                                 |            |                                                                                            |                                                                                                                   |
| **4**    | **Explainable Open-CIP, Enhanced Stereo & IUPAC Linter**      | Structural reporting & DB/patent registration   | **M**      | Transparent priority-digraph UI + fixes bridged/axial bugs closed tools ignore             | 100% pass rate on CIPValidationSuite + round-trip against RDKit/InChI 1.07 + Brecher 2006/2008 test cases         |
|          |                                                               |                                                 |            |                                                                                            |                                                                                                                   |
| **5**    | **Live Bidirectional Markush / R-Group & SAR Sync**           | Medicinal chemistry SAR tables & patent figures | **L**      | Brings ChemAxon/Signals enterprise R-group sync to offline WASM + CXSMILES                 | RDKit R-group decomposition parity on ChEMBL congeneric series + WildMol-20k Markush round-trip                   |
|          |                                                               |                                                 |            |                                                                                            |                                                                                                                   |
| **6**    | **Electron-Conserving Curved Arrows & Dative Valence Engine** | Mechanisms, organometallics & teaching quizzes  | **L**      | Turns decorative arcs into verifiable Bond-Electron matrix transformations                 | Mass/charge/electron conservation on 1,000 FlowER elementary steps + Ketcher \#10427 dative-valence matrix        |
|          |                                                               |                                                 |            |                                                                                            |                                                                                                                   |

**4. Capability Cards**

**Card 1 —MS/MS Semantic Cleavage & Exact-Mass Annotator**

- **Problem and evidence:** Analytical chemists still draw cleavage lines in ChemDraw and manually fix ion chemistry. As documented in the MyCompoundID workflow, ChemDraw's Mass Fragmentation tool forces the user to *"add or remove a proton... before breaking any bonds"* and then manually remove radicals with the eraser tool, assign positive charges, and add hydrogens to match MS/MS peaks. Recent HRMS studies still rely on this brittle tool for sub-5 ppm peak assignment, while HRMS practitioners repeatedly struggle with adducts (\[M+H\]+, \[M+Na\]+, \[M−H\]−), neutral-loss ambiguity (e.g., 146 Da coumaroyl vs. rhamnosyl), and isotope clusters. Dedicated cleavage tools like MassSpec Calculator Pro remain legacy Windows-only freeware.

- **The idea:** A native **MS Cleavage & Fragment Callout tool** in mzDraw. Sweeping a dashed cleavage line across 1– bonds (single cleavage or ring-opening retro-Diels-Alder/cross-ring cut) automatically generates an attached, layout-aware fragment annotation showing monoisotopic exact mass (4– decimals), formula, ion polarity/adduct, configurable Δ shift (−H to +2H for McLafferty/H-rearrangements), and optional Δpm error against a target experimental *m*/*z* (including Schymanski-linked provenance).

- **Why existing editors fail at it:** ChemDraw computes naive homolytic radical cuts unaware of ESI even-electron rules, adducts, or experimental peak tolerances; Ketcher, JSME, and Octet-Draw have no native MS cleavage engine at all.

- **How to build it under our constraints:**

  - *Algorithm & data:* Graph cut via DFS/BFS connected components in pure Rust; IUPAC 2021 standard monoisotopic atomic masses (bundled static lookup table, less than 4 KB) plus electron mass correction (*me*​=0.00054858 Da). Ring cuts require a 2-edge cutset (identifying the two cut edges that disconnect the graph). Output sorted deterministically by component canonical index.

  - *Rust/WASM footprint:* ~15–5 KB compiled WASM; zero external dependencies.

  - *Scriptable JSON op for agents:* {"op":"add_ms_cleavage","mol_id":"m1","bond_ids":\["b4","b9"\],"ion_mode":"ESI+","h_shift":-1,"adduct":"H","observed_mz":181.0495,"label_side":"auto"} —fully reversible via inverse op (remove_ms_cleavage).

<!-- -->

- **Smallest prototype to try first:** Single-bond and 2-bond ring cleavage cutline on a drawn molecule that renders two deterministic mass/formula callout tags (\[M+H\]+ / \[M-H\]- + ±1H shift toggle) without needing an external spectrum loaded.

- **Falsifiable hypothesis & refutation measure:** Annotating a 6-fragment MS/MS elucidation figure in mzDraw takes at least 50% less time and incurs at least 70% fewer mass-calculation errors than ChemDraw's Mass Fragmentation tool. *Refuted if* median task time reduction is under 25% or error rate is not lower across 5– analytical chemists.

- **Independent external proof:** Exact-mass round-trip within 0.0001 Da against RDKit Descriptors.ExactMolWt and Open Babel across 500 synthetic & MassBank fragment ions; timed task comparison + SUS score (target ≥80) against ChemDraw with 5– mass spectrometrists using 3 real MassSpecGym spectra.

- **Scientific risk:** Gas-phase CID rearrangements (skeletal migrations, water losses across non-adjacent atoms) cannot be represented solely by 1– bond cuts; over-claiming "predicted fragmentation" would mislead novices. Must be framed strictly as *mass-verified graphical annotation*, not ab-initio CID prediction.

- **Prior art and patent check:** Mass Spec Calculator Pro (freeware, Quadtech/SIS), ChemDraw Mass Fragmentation tool, Thermo Mass Frontier, Bruker SmartFormula. Patent clearance for specific interactive MS cleavage UI claims is **unverified**; basic homolytic bond-cut mass calculation dates to the 1990s (prior art well past 20-year patent terms).

- **Effort:** **M**

**Card 2 —Zero-OLE "Living Vector" Export (SVG / PDF / PNG with Embedded Canonical Graph)**

- **Problem and evidence:** Cross-platform document embedding is the single most recurring practical failure in practitioner forums. Mac users pasting ChemDraw drawings into Word get blurry PDFs and broken rendering on Windows; Word auto-recovery silently turns embedded ChemDraw objects into non-editable bitmaps; Windows DPI scaling clips drawings in Word; and within 18 days of Octet-Draw's launch, Reddit users immediately complained about raster copying and asked *"will there be a way to edit the pasted selection afterwards?"*.

- **The idea:** Every clipboard copy and file export (SVG, PDF, PNG) from mzDraw produces a **deterministic, publication-crisp vector/high-DPI graphic** whose fonts are optionally subset/outlined to deterministic vector paths (eliminating font/DPI drift) AND embeds the exact canonical mzDraw JSON scene graph (plus compressed CXSMILES/CDXML) inside standard container metadata (SVG \<metadata\>, PNG iTXt chunk, PDF XMP/attachment stream). Pasting or dragging any exported figure back into mzDraw—even years later from a colleague's Word doc, Google Doc, Typst project, or PDF—reopens the live editable structure.

- **Why existing editors fail at it:** ChemDraw depends on legacy Windows OLE / macOS PICT-PDF pasteboard hacks that break across OS versions and cloud editors (Google Docs, Notion, Overleaf); Ketcher exports static SVG/PNG without standardized round-trip scene metadata.

- **How to build it under our constraints:**

  - *Algorithm & data:* Pure Rust deterministic SVG/PDF path generator (fixed IEEE-754 decimal formatting, e.g., 3 decimal places, sorted DOM attributes so identical state yields identical SHA-256 bytes). Embed gzipped canonical JSON scene inside \<metadata id="mzdraw-v1"\> (SVG) and CRC-verified iTXt chunk (PNG). For clipboard, write simultaneous MIME types (image/svg+xml, image/png at 600 DPI with iTXt, and text/plain SMILES/CXSMILES) so Word, PowerPoint, Google Docs, and LibreOffice each pick their crispest supported format.

  - *Rust/WASM footprint:* ~30 KB WASM (using flate2 / minimal PNG encoder, zero headless browser dependency).

  - *Scriptable JSON op for agents:* {"op":"export_living_artifact","format":"svg","dpi":600,"outline_text":true,"embed_graph":true} and {"op":"import_living_artifact","bytes_b64":"..."}.

<!-- -->

- **Smallest prototype to try first:** CLI + browser "Copy Living PNG/SVG" and drag-and-drop re-import verifying byte-identical round-trip of a 3-step reaction scheme pasted into Word, PowerPoint, and Google Docs on macOS and Windows.

- **Falsifiable hypothesis & refutation measure:** Zero visual degradation (DPI clipping/blurriness) and 100% lossless re-editability after pasting into Word, PowerPoint, Google Docs, and LibreOffice and exporting to PDF. *Refuted if* more than 1 host application strips both SVG metadata and PNG iTXt chunks on save/copy-back.

- **Independent external proof:** Automated cross-OS round-trip matrix (macOS + Windows × Word, PowerPoint, Google Docs, LibreOffice, LaTeX, Typst) checking SHA-256 parity of the recovered mzDraw JSON graph; visual diff (SSIM ≥0.999) against ACS Document 1996 reference geometry.

- **Scientific risk:** Low scientific risk; engineering risk is that certain cloud apps (e.g., Google Docs or Word web) re-encode pasted PNGs and strip iTXt chunks—mitigated by testing SVG preservation and offering a tiny optional QR/invisible-comment fallback or sidecar link only if needed.

- **Prior art and patent check:** Diagrams.net (draw.io) embeds XML in PNG tEXt/SVG metadata; ChemDoodle and CDXML-in-EPS have partial equivalents (**unverified** for active patents on PNG/SVG chemical-graph embedding; embedding source XML in SVG/PNG metadata is long-established public prior art since the 2000s).

- **Effort:** **S**

**Card 3 —Constraint-Based Semantic Reaction Scheme & Catalytic Cycle Layout Engine**

- **Problem and evidence:** Chemists spend disproportionate time nudging molecules and arrows pixel-by-pixel. Users report that ChemDraw 25.5 breaks alignment inside groups and lacks anchor-object alignment, that "Clean Up Reaction" is *"too annoying for me to want to deal with it"*—driving chemists to copy individual molecules and arrows into PowerPoint just to space them—and that ChemDraw still lacks native catalytic-cycle layout and L-shaped multi-step arrows. Ketcher has repeated bugs when laying out reaction arrows and plus signs (#1617) and needed heavy manual drag-handle specs for multi-tail arrows (#4861). Power users also stress keeping shared scaffolds oriented identically from reactant to product.

- **The idea:** A **declarative reaction-scheme layout engine** supporting three first-class topologies: **linear/wrapping multi-step**, **convergent/divergent (multi-tail/branched)**, and **catalytic cycle (circular/elliptical ring of intermediates with curved entry/exit arrows)**. Instead of aligning raw bounding boxes, the solver aligns molecules along their **shared Maximum Common Substructure (MCS) core orientation** and visual ring baseline, automatically routes elbow/curved arrows with collision-free reagent text placement, and exports valid Open Reaction Database (ORD) structured JSON.

- **Why existing editors fail at it:** Existing editors treat reaction arrows, text blocks, and molecules as uncoupled 2D drawing primitives; "Clean Up" runs isolated per-molecule layout that flips scaffolds and collides with condition labels.

- **How to build it under our constraints:**

  - *Algorithm & data:* (1) Pairwise MCS (McGregor/vf2-bounded heuristic with deterministic tie-breaking on atom indices) aligns product scaffold coordinates to reactant template via Kabsch 2D rigid transform (rotation + translation only, preserving bond lengths and stereocenters). (2) 1D/2D Cassowary-style linear constraint solver (or deterministic topological sweep + gap-equalization pass) spaces bounding hulls with fixed visual margin and centers arrow baselines on scaffold centroids rather than outlier substituent tails. (3) Catalytic cycles place *k* intermediates on an ellipse sized from the sum of chord widths plus arrow arcs.

  - *Rust/WASM footprint:* ~40–0 KB WASM; deterministic float rounding.

  - *Scriptable JSON op for agents:* {"op":"layout_reaction_scheme","scheme_id":"s1","topology":"catalytic_cycle","steps":\[{"from":"m1","to":"m2","reagents":\["Pd(OAc)2"\],"conditions":"80 °C"}\],"align_mcs":true,"style":"acs_1996"}.

<!-- -->

- **Smallest prototype to try first:** Linear 3-step scheme + 4-intermediate catalytic cycle auto-layout with MCS scaffold lock and collision-free above/below arrow condition text.

- **Falsifiable hypothesis & refutation measure:** Drawing and formatting a 4-step synthesis scheme and a 4-intermediate catalytic cycle takes at least 40% fewer clicks and 35% less time than in ChemDraw/Ketcher while achieving zero label-bond overlaps. *Refuted if* users revert to manual nudging on more than 30% of steps in a 5– chemist benchmark.

- **Independent external proof:** Zero overlap / MCS orientation preservation rate on 500 multi-step synthetic & USPTO/ORD reaction schemes (validating exported JSON against the official ORD schema); timed task + SUS comparison vs. ChemDraw and Ketcher with 5– chemists.

- **Scientific risk:** Rigid MCS alignment can create steric clashes when a newly added bulky substituent collides with the unchanged scaffold; must detect self-overlap post-Kabsch and locally rotate only the newly attached exocyclic bond.

- **Prior art and patent check:** RDKit GenerateDepictionMatching2DStructure (BSD-3 open source), Ketcher multi-tail arrows (#4861), ORD web editor. Patent coverage for specific automated catalytic-cycle layout algorithms is **unverified**.

- **Effort:** **M**

**Card 4 —Explainable Open-CIP, Enhanced Stereo & IUPAC Graphical Linter**

- **Problem and evidence:** Closed-source editors silently get stereochemistry wrong and cannot be audited. Chemistry StackExchange documents ChemDraw assigning erroneous (*R*)/(*S*) labels to bridged bicycloheptane / isoborneol from its own template (unresolved for years) and misclassifying identical cyclic allenes and atropisomeric biphenyls as diastereomers—contradicting its own Structure-to-Name tool and failing to emit IUPAC *M*/*P* descriptors. Mayfield, Lowe & Sayle's ACS study (CIPValidationSuite) proved systemic disagreement across proprietary CIP engines. Meanwhile, Fourches et al. found 0.1–.4% structural error rates in public/commercial databases that fabricate false SAR activity cliffs, and Ketcher's Enhanced Stereochemistry workflow requires multi-click modal dialogs (#527).

- **The idea:** A deterministic Rust implementation of **2013 IUPAC CIP rules** (tetrahedral *R*/*S*, *E*/*Z*, axial/atropisomer *M*/*P*, pseudoasymmetry *r*/*s*) coupled with (a) an **IUPAC 2006/2008 graphical ambiguity linter** that catches perspective-vs-wedge conflicts on bridged/Haworth/chair templates and wedges drawn on ring bonds, (b) one-click inline **Enhanced Stereochemistry** (abs, AND1, OR1) badges directly on atoms (no modal dialog), and (c) an **"Explain CIP" hover overlay** highlighting ligand priority ranks 1 →2 →3 →4 and the exact decision sphere where priority broke.

- **Why existing editors fail at it:** ChemDraw is closed source with no public bug tracker or traceable CIP explanation; Ketcher delegates CIP/layout to external Indigo WASM and buries Enhanced Stereo behind modal dialogs.

- **How to build it under our constraints:**

  - *Algorithm & data:* Implement John Mayfield's open-CIP hierarchical digraph expansion (Sequence Rules 1a, 1b, 2, 3, 4a–c, 5) in pure Rust with bounded graph-expansion guardrails for pathological polycycles. Graphical linter checks IUPAC 2006 rules (e.g., 3 collinear bonds around a stereocenter, hash/wedge directionality on bridged ring systems, ambiguous crossed double bonds).

  - *Rust/WASM footprint:* ~35–0 KB WASM; fully deterministic.

  - *Scriptable JSON op for agents:* {"op":"lint_stereochemistry","mol_id":"m1","standard":"iupac_2013","include_digraph_trace":true} and {"op":"set_enhanced_stereo","mol_id":"m1","atom_ids":\["a3","a7"\],"group":"AND","group_id":1}.

<!-- -->

- **Smallest prototype to try first:** Headless Rust CLI crate implementing Sequence Rules 1– + graphical wedge-conflict linter, outputting JSON priority explanations.

- **Falsifiable hypothesis & refutation measure:** Chemists resolve ambiguous/incorrect stereochemical drawings at least 2× faster and with ≥95% accuracy (vs. baseline unassisted drawing) when shown inline IUPAC conflict warnings and 1– CIP rank overlays. *Refuted if* false-positive linter noise exceeds 5% on valid ACS/PubChem structures or fails to cut stereochemical errors in a 5– chemist test.

- **Independent external proof:** ≥99% exact agreement on CIPValidationSuite and RDKit's new CIP implementation + InChI reference canonicalization on 10,000 stereochemically rich PubChem/ChEMBL synthetic derivatives; explicit regression test on the bicycloheptane and cyclic allene/biphenyl cases.

- **Scientific risk:** Perspective-implied 3D rings (norbornane, adamantane, bridged steroids) have genuinely ambiguous 2D-to-3D projection conventions; the linter must distinguish *definitely wrong* from *convention-dependent* rather than forcing ugly flat drawings.

- **Prior art and patent check:** Open-source Centres / CDK-CIP / RDKit AssignStereochemistry (BSD/LGPL); IUPAC 2006/2008 public standards. No known patent barrier on IUPAC CIP rules; specific UI patent clearance **unverified**.

- **Effort:** **M**

**Card 5 —Live Bidirectional Markush / R-Group Decomposition & SAR Matrix Sync**

- **Problem and evidence:** Medicinal chemists and patent attorneys live in Markush scaffolds and SAR tables, yet open editors break on them. A 2026 PMC review and JCIM paper document persistent graphical ambiguity in Markush scaffolds (variable attachment points, position-variation bonds crossing rings, disconnected R-group legends). Ketcher's tracker shows years-long bugs with attachment points protruding outside R-group brackets (#197), S-group mixture handling (#12775), and leaving-group attachment naming (#12713). Meanwhile, live scaffold-to-SAR synchronization is locked behind enterprise suites like ChemAxon Markush Editor and Revvity Signals One.

- **The idea:** A **bidirectional Markush/SAR panel** synced live to the 2D canvas. Users either (a) draw a scaffold with \$R1, \$R2 (or ring position-variation bonds) and fill an inline lightweight table of substituents/SMARTS + numeric columns (e.g., IC50​, retention time, mass shift), or (b) paste/drop 5–0 analogs (SMILES/SDF) and click **Decompose to Markush** to extract a single MCS core + substituent table. Editing the scaffold redraws every enumerated analog in identical core orientation; exporting emits standard CXSMILES, SDF with R-group properties, or a publication-ready SVG SAR table.

- **Why existing editors fail at it:** Free editors treat R-groups as dead text labels or fragile S-groups without bidirectional table sync; enterprise suites require server backends and costly per-seat licenses.

- **How to build it under our constraints:**

  - *Algorithm & data:* Deterministic MCS core extraction (reusing Card 3's MCS engine) + R-group attachment-point graph splicing. Bounded lazy enumeration (configurable cap, e.g., 1,000 compounds, matching ChemAxon's safety guard) with deterministic lexicographic ordering. Attachment-point rendering strictly clipped inside S-group brackets with IUPAC wavy-line conventions.

  - *Rust/WASM footprint:* ~45–0 KB incremental WASM (sharing MCS with Card 3).

  - *Scriptable JSON op for agents:* {"op":"decompose_r_groups","mol_ids":\["m1","m2","m3"\],"core_id":"auto","emit_table":true} and {"op":"enumerate_markush","scaffold_id":"s1","r_table":{"R1":\["F","Cl","OMe"\],"R2":\["H","Me"\]},"align_to_core":true}.

<!-- -->

- **Smallest prototype to try first:** Draw 1 scaffold with R1/R2 + 4-row JSON/CSV substituent table →deterministic aligned enumeration to SDF + CXSMILES + 2×2 SVG grid.

- **Falsifiable hypothesis & refutation measure:** Building a 12-compound publication SAR figure + SDF export takes at least 60% less time than drawing/aligning manually in ChemDraw/Ketcher, with 0% scaffold-to-table mismatch errors. *Refuted if* MCS decomposition fails on more than 15% of standard congeneric series or users need manual re-layout of enumerated depictions.

- **Independent external proof:** Parity check against RDKit rdRGroupDecomposition on 50 congeneric series from ChEMBL (public CC BY-SA) and round-trip CXSMILES parsing on 500 Markush structures from WildMol-20k; timed comparison vs. Ketcher/ChemDraw with 5– chemists.

- **Scientific risk:** Symmetrical cores yield ambiguous R-group numbering (R1 vs. R2 swaps) and ring-closure R-groups (fused cycles) break simple attachment-point tables; requires canonical core symmetry breaking and explicit "non-decomposable analog" flagging.

- **Prior art and patent check:** ChemAxon Markush Editor, RDKit rdRGroupDecomposition (BSD-3), DataWarrior (GPL). Patent clearance on specific interactive patent-claim tree validation UIs is **unverified** (keep scope to scaffold/table sync and enumeration, avoiding patent-claim legal hierarchy UI).

- **Effort:** **L**

**Card 6 —Electron-Conserving Curved Arrows & Dative-Bond Valence Engine**

- **Problem and evidence:** Curved arrows in web editors are merely decorative splines. Ketcher \#1045 added elliptical arc arrows so users could draw mechanisms, with zero tracking of electron flow or atom/bond attachment, and recent Octet-Draw testers flagged missing carbene lone-pair semantics and pen-tool curved arrows. Simultaneously, Ketcher \#10427 (July 2026) revealed that Ketcher and Indigo treat dative bonds as having zero impact on valence—producing wrong valences for donor-acceptor P, C, and F complexes—while inorganic chemists on IONiC/VIPEr note ChemDraw treats metal centers like carbon during structure cleanup. On the teaching side, *J. Chem. Educ.* (2026) documents surging adoption of browser-based molecular editor quizzes for automated misconception feedback.

- **The idea:** Anchor every curved arrow (2*e*− full head, 1*e*− fishhook) semantically to **electron source** (bond or lone pair) and **electron sink** (atom or bond). Using Bond-Electron (BE) matrix arithmetic, a live **"Verify & Step"** mode checks electron/charge/mass conservation in real time and can auto-generate the next mechanistic intermediate (updating bond orders, formal charges, and radicals deterministically). Pair this with an orbital-aware **dative/coordination valence model** (explicitly tracking donor/acceptor orbital occupancy for main-group and common transition-metal complexes) so organometallic mechanisms don't drown in false valence errors.

- **Why existing editors fail at it:** ChemDraw and Ketcher separate graphics (curved splines) from topology (connection tables); their valence checkers assume organic covalent octet rules and break on dative/organometallic bonds.

- **How to build it under our constraints:**

  - *Algorithm & data:* Represent each elementary step as a sparse integer delta matrix ΔE where every full arrow moves 2 electrons (subtract 2 from source diagonal/off-diagonal, add 2 to sink) and every fishhook moves 1 electron. Conservation holds iff ∑ΔE=0 and resulting atom electron counts satisfy the orbital-occupancy table (incorporating donor/acceptor valence shifts from Ketcher \#10427 and 18-electron / common oxidation-state ranges for *d*-block metals).

  - *Rust/WASM footprint:* ~30 KB WASM (integer sparse matrix ops + element orbital table).

  - *Scriptable JSON op for agents:* {"op":"add_electron_arrow","step_id":"st1","electrons":2,"source":{"kind":"lone_pair","atom":"a4"},"sink":{"kind":"bond","atoms":\["a4","a2"\]}} and {"op":"apply_mechanism_step","step_id":"st1"}.

<!-- -->

- **Smallest prototype to try first:** Organic 2*e*− arrow anchoring + live BE-matrix conservation check + one-click "Generate Next Intermediate" on 10 classic reactions (SN​2, E2, aldol, Diels-Alder, nucleophilic acyl substitution) plus main-group dative-bond valence validation.

- **Falsifiable hypothesis & refutation measure:** Undergraduate/graduate chemists drawing multi-step mechanisms catch 100% of valence/charge-conservation slips before export and build 3-step mechanisms at least 30% faster than redrawing intermediates manually. *Refuted if* arrow-anchoring snapping adds friction that increases total task time vs. freehand curved arrows in ChemDraw.

- **Independent external proof:** Automated execution against 1,000 balanced elementary steps from the open **FlowER** mechanistic dataset (verifying product InChI/SMILES match against RDKit) plus all donor-acceptor test cases in Ketcher \#10427; SUS + error-rate comparison with 5– chemistry students/instructors.

- **Scientific risk:** Concerted transition states, delocalized *η*5/*η*6 organometallic hapticity, and non-classical carbocations resist strict localized Lewis BE-matrices; the verifier must allow an explicit "non-classical / visual-only" override without blocking drawing.

- **Prior art and patent check:** Dugundji-Ugi BE-matrix formalism (1973, public domain); Reaction Explorer / "No electron left behind" (*J. Chem. Inf. Model.*); FlowER/MechFinder. Patent clearance on commercial educational auto-grading UIs (e.g., Sapling/Macmillan, MasteringChemistry) is **unverified**—avoid copying proprietary homework grading workflows.

- **Effort:** **L**

**5. Hyper-Creative Section: 3 Bold Ideas Beyond the Classic Editor Model**

**Bold Idea 1 —Live Spectrum-Driven Canvas ("Draw by Peak, Not by Bond")**

- **Break from the classic model:** Today, mzLab (spectra) and mzDraw (structures) are adjacent tools; here they become a **single coupled bidirectional instrument**. Load an experimental MS/MS spectrum on the side of the canvas. Clicking any unassigned peak highlights every bond-cut / substructure motif on the currently drawn candidate within ±5 ppm. Conversely, moving a substituent (e.g., shifting an −OH across three aromatic positions to resolve positional isomers in non-target screening) recalculates all explained vs. unexplained MS/MS peaks in under 10 ms in WASM, turning 2D drawing into live hypothesis testing.

- **Constraints fit:** Pure Rust/WASM combinatorial sub-graph mass matcher over the single active molecule (typically ≤60 heavy atoms →instant deterministic enumeration without a server).

- **Cheap experiment to prove it wrong (1 week):** Take 20 positional-isomer pairs from MassBank/MassSpecGym (blinded labels). Give 5 analytical chemists the precursor formula + MS/MS spectrum; measure whether interactive live peak-highlighting on mzDraw increases correct isomer ranking over static ChemDraw + spreadsheet mass calculator. *Falsified if* naive bond-cut matching produces so many coincidental mass hits (false-positive fragment matches) that chemists cannot distinguish positional isomers better than chance.

**Bold Idea 2 —Executable Drawings ("Canvas-as-Query & Transformation-Product Compiler")**

- **Break from the classic model:** A drawn reaction scheme or degradation pathway stops being dead ink and compiles deterministically into **two executable artifacts**: (1) a local **reaction-SMARTS generator** that runs inside mzLab to screen an LC-HRMS feature list for predicted transformation products (TPs) of the drawn parent compound, and (2) an **Open Reaction Database (ORD) JSON protocol** with stoichiometry and exact masses derived live from drawn reactants. Draw a photocatalytic hydroxylation or dealkylation arrow once, and the canvas immediately outputs the exact *m*/*z* watch-list and SMILES library for your sample.

- **Constraints fit:** Deterministic Rust reaction-transform engine (graph rewrite rules from drawn atom-mapped reactant →product) + JSON schema emission; zero server needed.

- **Cheap experiment to prove it wrong (1 week):** Have 5 chemists draw 10 environmental/metabolic transformation arrows (hydroxylation, *N*-dealkylation, *S*-oxidation, glucuronidation, hydrolysis) in a prototype without writing SMARTS by hand. Compile the drawn arrows into reaction rules and run them against RDKit ReactionFromSmarts on 50 synthetic test substrates. *Falsified if* automatic atom-mapping from drawn schemes produces broken or over-specific SMARTS on more than 25% of reactions.

**Bold Idea 3 —Semantic Chemical Git-Diff & Agent "Ghost-Layer" Co-Drawing**

- **Break from the classic model:** Because mzDraw is CLI-native with deterministic JSON edit ops, stop making AI agents overwrite the whole canvas with opaque SMILES blobs. Instead, render agent-generated edits (e.g., proposed transformation products, IUPAC stereochemistry fixes, or cleaner scheme layouts) as **interactive translucent "ghost diffs"** directly on top of the user's drawing—highlighting added bonds in green, cleaved bonds in red, shifted stereocenters in amber, and live Δ*m*/*z*—with one-click per-hunk Accept / Reject (like IDE code-review diffs, but over chemical ASTs).

- **Constraints fit:** Reuses mzDraw's exact JSON operation log + deterministic undo stack; diffing two op-streams or canonicalized graphs is trivial in Rust/WASM.

- **Cheap experiment to prove it wrong (1 week):** Present 6 chemists with 10 real chemical structure edits (5 valid TP proposals, 5 subtle traps containing a wrong stereocenter or 1-atom valence/formula error) displayed either as side-by-side static structures or as an overlaid semantic ghost-diff. *Falsified if* ghost overlays create visual clutter that *increases* review time or fails to improve trap-detection rate by at least 20%.

**6. What NOT to Build (and Why)**

1.  **Freehand Stylus / Gesture Handwriting Recognition on Desktop:** Academic systems like *OrganicPad* (Clemson, 2007) explored ink-stroke molecule recognition for tablet classrooms, yet mouse/trackpad chemistry users overwhelmingly prefer deterministic keyboard hotkeys (Reddit power users cite single-key atom/ring shortcuts like 1, a, Ctrl+RightArrow as the primary reason they stick with ChemDraw). Gesture recognition adds nondeterministic failure modes, hurts stereochemical precision, and bloats WASM.

2.  **Embedded 3D Conformer Generator & Split-Screen 3D Viewer:** Browser tools like MolDraw advertise live 2D+3D side-by-side views, and ChemDraw ships Chem3D, yet even their own reviewers point users to Avogadro for serious 3D work. Embedding a force field (MMFF94/UFF) + WebGL renderer wrecks mzDraw's light WASM footprint and solves zero high-frequency 2D authoring problems.

3.  **Full Macromolecule / HELM / Biologics Hybrid Mode:** Inspecting Ketcher's live GitHub tracker (#12680–12741) shows an enormous maintenance burden from unifying small-molecule and macromolecule modes (lazy-loading refactors, monomer rotation bugs, missing query/atom-mapping parity). That domain is orthogonal to mzLab and would consume years of engineering for a different user base.

4.  **In-Editor Natural-Language "Prompt-to-Structure" LLM Generator:** Violates three hard constraints at once (no server, deterministic output, light footprint) and risks injecting subtle connectivity/stereochemistry hallucinations into scientific records. Keep mzDraw a rock-solid, deterministic JSON-op execution engine that *external* agents drive and validate.

5.  **Windows OLE Server / Proprietary Binary Formats:** Legacy OLE integration is the root cause of decades of Mac/Windows Word blurriness, DPI clipping, and document corruption. Open Living Vector containers (Card 2) solve the same job cleanly across every OS and cloud editor.

**7. Challenging the Brief: 3 Better Questions You Should Have Asked**

1.  **"Before adding new capabilities, where does mzDraw lose to ChemDraw on raw drawing velocity (time-to-first-valid-structure and keystrokes-per-molecule)?"** Practitioner threads make clear that chemists reject free alternatives less because of missing advanced features and more because of missing muscle-memory hotkeys (1 for methyl, a for benzene, Ctrl+Arrow to duplicate across a reaction arrow, Ctrl+J to join) and awkward default aesthetics (*"gives your models the 'my first molecule' feeling"*). A 10-molecule timed hotkey benchmark against ChemDraw may reveal higher ROI than any new module.

2.  **"Should mzDraw fight a war of attrition against ChemDraw/Octet-Draw/Ketcher for synthetic organic chemists, or dominate the uncontested analytical & mass-spectrometry elucidation niche?"** Your brief bundles synthetic/medchem jobs (SAR tables, patent Markush, multi-page schemes) alongside mzLab's analytical identity (MS fragments, Schymanski levels). With open-source clones like Octet-Draw already targeting 1:1 ChemDraw parity, mzDraw's strongest moat is deep integration between chemical graphs and analytical data (Cards 1 + 2 + 4 + Bold Ideas 1–), not chasing every medchem/patent feature first.

3.  **"How does the 'synthetic data only' constraint avoid blinding you to real-world chemical file pathologies?"** Synthetic generators produce clean graphs, whereas real ChemDraw CDXMLs, patent Markush files, and public databases exhibit 0.1–.4% structural error rates and bizarre wedge/dative/layout conventions. Keeping training/unit fixtures synthetic is great for determinism, but **validation** must still test against open CC0/CC-BY real corpora (MassBank, MassSpecGym, ORD, CIPValidationSuite) so mzDraw doesn't break on the first real user file.

**8. One-Page Plan: The Three Experiments to Run First**

Run these three low-cost, high-signal falsification experiments in parallel over **2– weeks** before committing to full UI builds.

**Experiment 1 —Headless Open-CIP & Graphical Ambiguity Audit (Validates Card 4)**

- **Goal:** Prove that a compact Rust/WASM CIP + IUPAC 2006/2008 linter can outperform closed-source editors on stereochemical accuracy within budget.

- **Setup:** Build a standalone Rust CLI crate (no UI) implementing Open-CIP rules 1– plus a 2D wedge-conflict detector. Run it headlessly on: (a) the public CIPValidationSuite, (b) the documented ChemDraw bicyclic isoborneol/camphor and cyclic allene/biphenyl failure cases, and (c) 2,000 stereochemically diverse molecules cross-checked against RDKit + InChI.

- **Go / Kill Threshold:** **Go** if WASM size increment is ≤60 KB, agreement with CIPValidationSuite + RDKit is ≥99%, and 100% of the documented ChemDraw perspective/axial bugs are correctly flagged with ≤3% false positives on clean ACS-style structures. **Kill/Defer** if bridged-perspective heuristics generate excessive false alarms.

**Experiment 2 —Zero-OLE "Living Vector" Cross-App Torture Matrix (Validates Card 2)**

- **Goal:** Verify whether embedding canonical mzDraw JSON inside SVG \<metadata\> and PNG iTXt chunks survives real academic writing pipelines without OLE.

- **Setup:** Export 3 test figures (simple molecule, stereocentered natural product, 3-step reaction scheme) from mzDraw CLI as deterministic outlined SVG and 600-DPI Living PNG. Paste/insert each into **Word (macOS & Windows), PowerPoint (macOS & Windows), Google Docs, LibreOffice Writer, LaTeX, and Typst**, export each document to PDF, and test (a) visual crispness at 400% zoom, and (b) copy-back / extract-back into mzDraw for byte-exact JSON graph recovery.

- **Go / Kill Threshold:** **Go** if visual rendering is artifact-free in 6/6 tools AND lossless editable graph recovery succeeds in at least 4/6 tools (including Word desktop + PowerPoint + LibreOffice). **Pivot** (add lightweight sidecar/embedded-URL fallback) if Word/Google Docs strip both SVG metadata and PNG iTXt on save.

**Experiment 3 —MS/MS Cleavage & Live Peak-Matching Usability Sprint (Validates Card 1 + Bold Idea 1)**

- **Goal:** Test whether semantic bond-cleavage + live *m*/*z* matching actually beats ChemDraw for analytical chemists—and whether naive cut-matching produces too many false-positive peak hits on positional isomers.

- **Setup:** Prototype a minimal cutline tool in mzDraw (1– bond cuts, ESI+/ESI-, ±2H shift, exact mass + Δpm against an imported peak list). Benchmark two things:

  1.  *Algorithmic:* Run automatic 1– bond cut matching on 100 annotated MS/MS spectra (including 20 positional-isomer pairs) from MassBank / MassSpecGym to measure fragment recall and isomer discrimination.

  2.  *Human (5– analytical chemists):* Within-subjects timed task comparing mzDraw prototype vs. ChemDraw Mass Fragmentation tool on annotating 2 unknown HRMS spectra (measuring task time, mass/formula errors, click count, and SUS score).

<!-- -->

- **Go / Kill Threshold:** **Go** if human annotation time drops ≥40%, formula/mass errors drop to 0, and SUS ≥78—even if Bold Idea 1's positional-isomer auto-ranking needs tighter rearrangement rules. **Kill Bold Idea 1 (keep Card 1)** if automated peak-to-bond highlighting has false-positive match density too high to aid isomer ranking.

**Sources**
