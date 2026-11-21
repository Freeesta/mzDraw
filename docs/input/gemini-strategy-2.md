<!-- INPUT, NOT VERIFIED. Output of an external research chat (Gemini Deep Research) asked by the owner on 21 Nov 2026 with a prompt written by COO-MZDRAW. Converted from DOCX to Markdown without edits. Every claim, number, issue number and URL is unverified until re-checked at the source. The author says itself that the GitHub reaction counts, test-suite internals, the ACS 2026 column width, ChemDraw 25.5 notes and patent clearance were NOT verified. -->

# Chemical Editor Strategy and Design

<https://gemini.google.com/app/b0657099ff7e71a0>

*User prompt: You are a research strategist with deep knowledge of chemistry software, developer tools and human-computer interaction. I want a deep, evidence-based result, not a long one. Fewer ideas, each one defended. PROJECT (all facts below are true today; do not assume anything beyond them) We are building a 2D chemical structure and reaction editor, written in Rust, compiled to WebAssembly. It runs in a browser, as a desktop app (macOS, Windows), and is embedded in a larger scientific platform (mzLab, a mass-spectrometry and cheminformatics suite) that has its own Rust chemistry engines (CIP labels, fragmentation, structure checks, reaction transformations, substructure search). Design rules: - All logic is in Rust. The page only forwards events and paints a display list. No chemistry in JavaScript. - CLI-native and agent-first: every feature is first a command with a machine-readable form (a JSON patch language of 12 operations, \`--json\` with stable keys, distinct exit codes, dry-run, batch and stdin streaming, byte-for-byte deterministic output, no browser needed). The page is only one client of the same commands. AI agents are first-class users: they draw canvases through the command line. - Every operation has an exact inverse; a replayed command log produces identical bytes in the app, the browser and the CLI (we check parity automatically). - Formats: reads and writes molfile V2000/V3000, SDF, SMILES, KET, CDXML, SVG (with the document embedded), PNG; reading ChemDraw CDX is planned. - Built already: drawing and editing, undo, abbreviations, reactions, annotations of identification levels, a standards-conformance report, accessible SVG, keyboard-only editing, a Rust-decided right-click menu, movable palettes. Measured facts: wasm module about 585 kB; a saved SVG is 3 to 15 kB, a PNG at 300 dpi 20 to 200 kB. In our sister CLI, 901 separate process calls took 77 s versus 0.8 s for one batched call (about 95x): per-call cost dominates for agents. Our drawing-speed benchmark against other editors is not finished: no numbers yet, so do not invent any. ALREADY CONSIDERED (do not repeat; go beyond): MS/MS fragment annotation on the canvas; a living vector file with an embedded document; constraint-based reaction-scheme layout; explainable CIP and a drawing linter; Markush and R-group tables; electron-conserving curved arrows; spectrum-driven canvas; executable drawings; a ghost-diff mode where an agent proposes edits as a reviewable diff. TASK Find things that are missing in this field, or that exist in other fields and would transfer here. Then invent something new. 1. Start from raw sources, not from summaries: issue trackers and test suites of RDKit, Open Babel, Indigo, CDK, Ketcher (use the GitHub API, sort by reactions and frequency), release notes and user forums of ChemDraw and Marvin, file-format specifications, IUPAC and ACS drawing guidelines, open-licence drawings (PubChem, ChEMBL, Wikimedia Commons). Name each source with its URL. Mark every claim you could not read at the source as UNVERIFIED. Never invent a reference, a number or a URL. 2. Study how an agent fails when it draws (addressing atoms by index, no undo, too many calls, ambiguous stereo) and what tools from other fields solve the same shape of problem: CAD constraint solvers, git patch/diff and merge, compilers (intermediate representation, linters, language servers), Figma-style multiplayer, text-to-score systems (LilyPond), TikZ/ChemFig, Mermaid. Use at least two methods (SCAMPER, morphological analysis, analogy transfer) and show the working briefly. 3. Deliver at most 5 ideas, at least 2 of them genuinely new (no prior art found). For each idea give: (a) the problem in one sentence, with the raw source that shows it exists; (b) the analogy from another field; (c) how it maps to our architecture (new JSON operation or setting, Rust module, CLI route, what mzLab engine it reuses); (d) a prior-art and patent check (Google Patents, Google Scholar), with the closest hit; (e) the cheapest experiment that could KILL the idea, with a pass/fail threshold; (f) effort (S/M/L) and what to cut if it grows. 4. Say which 3 popular features we should NOT build, and why. 5. Say how we should prove the result correct from outside ourselves: a public corpus, an independent toolkit round trip (RDKit, Open Babel, Indigo, InChI), a human time-and-motion test with at least 5 chemists against ChemDraw and Ketcher (keystroke-level model for the estimate), and a fixed agent task suite run on several models that do not know our system. Specify the exact metrics and the confidence intervals. OUTPUT: Markdown, English, under 2,500 words, one table at the top (idea, novelty, evidence strength, effort, kill test). End with a list of everything you could not verify.*

Response:

| **Idea**                                         | **Novelty**                | **Evidence Strength**                                   | **Effort** | **Kill Test**                                                                                |
|--------------------------------------------------|----------------------------|---------------------------------------------------------|------------|----------------------------------------------------------------------------------------------|
| **1. ChemSel: Chemical-Spatial Patch Selectors** | Transfer + novel synthesis | High (Ketcher \#5873, ChemDraw 23, RDKit \#4041)        | M          | Agent surgical-edit accuracy on 30 tasks fails to beat index addressing by ≥20 pp.           |
| **2. Stereo-Invariant 2D Geometry Kernel**       | **Genuinely new**          | High (RDKit \#4041/#5067, Indigo \#2739, IUPAC 2006)    | M          | Re-solved wedges fail IUPAC ST-1.1 / InChI parity on \>0.5% of 1,000 drag/flip transforms.   |
| **3. Parametric Core-Linked Analog Families**    | Transfer (Figma + Git)     | High (ChemDraw 25.5 forums, RDKit \#1286/#8124)         | M          | Core RMSD \>0.0 px or clash-free rate \<90% on 50 ChEMBL SAR series.                         |
| **4. Knuth-Plass Adaptive Abbreviation Folding** | **Genuinely new**          | High (IUPAC 2008 GR-0.3/GR-2.2, RDKit \#6626/#4508)     | L          | Blind chemist panel prefers naive scaling or greedy folding in \>40% of 30 column-fit pairs. |
| **5. Headless Transactional Canvas-LSP**         | **Genuinely new**          | High (95× CLI overhead, RDKit \#4504/#6626, arXiv 2026) | S–M        | Blind agent pass@1 on 30 layout tasks improves \<15 pp or needs \>2 CLI calls vs. VLM loop.  |

## 1. Raw-Source Diagnosis & Agent Failure Modes

Primary-source inspection across issue trackers, format specifications, drawing standards, and user forums exposes five structural failure modes that existing 2D editors—and AI agents driving them—hit repeatedly:

- **Volatile, opaque addressing:** Ketcher's getKET incremented internal atom IDs on repeated exports ([<u>Ketcher \#5873</u>](https://github.com/epam/ketcher/issues/5873)), and Ketcher's API silently dropped selections in KET fragments ([<u>Ketcher \#8898</u>](https://github.com/epam/ketcher/issues/8898)). ChemDraw 23.0 introduced opaque canvas atom IDs ([<u>ChemDraw 23.0 Release Notes</u>](https://revvitysignals.com/sites/default/files/2024-02/rs-release-notes-DT-23.0-chemdraw.pdf)), which help scripts only after an extra read round-trip. In typed-smiles (Nov 2026), abbreviating a group creates index gaps that break numeric addressing ([<u>typed-smiles</u>](https://typst.app/universe/package/typed-smiles/)). For an LLM agent, integer indices shift after every deletion or canonicalization.

- **Coordinate-coupled, brittle stereochemistry:** RDKit users routinely trigger random (R)/(S) inversion because wedge direction depends on 2D coordinates while chiral tags depend on bond-list order ([<u>RDKit \#4041</u>](https://github.com/rdkit/rdkit/issues/4041)); single-wedge defaults also deviate from IUPAC clarity for quaternary centers ([<u>RDKit \#5067</u>](https://github.com/rdkit/rdkit/issues/5067)). Indigo leaked stereocenter maps across CDXML reaction products ([<u>Indigo \#2739</u>](https://github.com/epam/Indigo/issues/2739)). IUPAC 2006 (ST-1.1.9–ST-1.1.10) explicitly classifies common drag-induced wedge geometries as formally ambiguous ([<u>Brecher 2006</u>](https://iupac.qmul.ac.uk/drawing/stereo.html)). Meanwhile, V2000 drops OR/MIXED enhanced stereo collections ([<u>Ketcher \#64</u>](https://github.com/epam/ketcher/issues/64), [<u>BIOVIA CTfile 2026</u>](https://www.3ds.com/assets/invest/2026-01/biovia_chemistry_2026_ctfile_formats.pdf)).

- **Broken multi-structure alignment:** ChemDraw 25.5 broke partial-molecule and grouped-object anchoring, prompting chemists to downgrade ([<u>r/chemistry May 2026</u>](https://www.reddit.com/r/chemistry/comments/1to4yp5/chemdraw_objects_in_groups_and_how_to_align_them/), [<u>r/OrganicChemistry Aug 2026</u>](https://www.reddit.com/r/OrganicChemistry/comments/1vswtuo/anyone_struggling_with_the_newer_version_of/)). Programmatically, RDKit's GenerateDepictionMatching2DStructure mutates core coordinates during clash cleanup ([<u>RDKit \#1286</u>](https://github.com/rdkit/rdkit/issues/1286)) and flips scaffold orientation ([<u>RDKit \#8124</u>](https://github.com/rdkit/rdkit/issues/8124)).

- **2D overlaps and scale-versus-legibility collapse:** Bridged bicyclics, 1,1-disubstituted cyclobutanes, and data S-group labels still generate overlapping atoms/text in RDKit ([<u>RDKit \#6626</u>](https://github.com/rdkit/rdkit/issues/6626), [<u>\#4504</u>](https://github.com/rdkit/rdkit/issues/4504), [<u>\#4508</u>](https://github.com/rdkit/rdkit/issues/4508)), CDK ([<u>CDK \#1282</u>](https://github.com/cdk/cdk/issues), [<u>\#1292</u>](https://github.com/cdk/cdk/issues/1292)), Open Babel ([<u>Open Babel \#2015</u>](https://github.com/openbabel/openbabel/issues/2015)), and Indigo CDXML imports ([<u>Indigo \#2810</u>](https://github.com/epam/Indigo/issues/2810); cf. [<u>RDKit \#9354</u>](https://github.com/rdkit/rdkit/issues/9354)). IUPAC 2008 (GR-0.3, GR-2.2) forbids shrinking diagrams to illegible font sizes and recommends replacing fragments with abbreviations instead ([<u>Brecher 2008</u>](https://iupac.qmul.ac.uk/drawing/drawing.html)); ACS Document 1996 fixes bond length at 0.508 cm / 14.4 pt ([<u>Wikipedia MoS</u>](https://en.wikipedia.org/wiki/Wikipedia:Manual_of_Style/Chemistry/Structure_drawing), [<u>Ketcher \#5152/#5156</u>](https://github.com/epam/ketcher/issues/5152)).

- **Agent round-trip & blindness penalty:** Given our measured ~95× process-spawn penalty (77 s vs. 0.8 s for 901 calls), chatty per-atom agent loops are unusable; meanwhile, multimodal agents rendering via SVG/VLM loops suffer high token costs and geometric hallucinations ([<u>LiveFigure, arXiv 2026</u>](https://arxiv.org/html/2605.23527v1); [<u>IChO 2025 Multi-Agent, arXiv 2025</u>](https://arxiv.org/html/2511.16205v2)).

*Note:* GitHub API reaction-count sorting across all five repositories could not be executed directly via web search; specific issue frequencies are marked **UNVERIFIED** below.

## 2. Synthesis Methods (Working Shown)

### Method A: Morphological Analysis (5-Axis Design Space)

| **Axis**                     | **Legacy Editor State**                            | **Cross-Domain Target**                      | **Selected Synthesis**        |
|------------------------------|----------------------------------------------------|----------------------------------------------|-------------------------------|
| **Target Addressing**        | Pixel coords / integer indices / random UUID       | CSS selectors + Tree-sitter AST queries      | **Idea 1 (ChemSel)**          |
| **Stereo-Geometry Coupling** | Wedges define stereo; drags silently invert        | Parametric CAD geometric invariant solver    | **Idea 2 (Stereo Kernel)**    |
| **Multi-Structure Series**   | Copy-paste + manual align / lossy MCS refit        | Figma Master Components + Git delta replay   | **Idea 3 (Analog Families)**  |
| **Print-Box Overflow**       | Global shrink (breaks ACS 14.4 pt) or greedy abbr. | TeX Knuth-Plass box-and-glue DP / LilyPond   | **Idea 4 (Adaptive Folding)** |
| **Agent Feedback Loop**      | N process calls + PNG screenshot VLM loop          | rustc JSON LSP diagnostics + atomic rollback | **Idea 5 (Canvas-LSP)**       |

### Method B: SCAMPER + Analogy Transfer (Applied to the 12-Op Patch Engine)

- **Substitute (Addressing):** Replace brittle atom_id: 14 in patch ops with a chemical-spatial selector resolved inside Rust before execution (DOM querySelector → **Idea 1**).

- **Reverse (Stereo Dataflow):** Invert 2D wedge -\> chirality into locked chirality -\> solver updates 2D wedge during every geometric op (CAD constraint solver → **Idea 2**).

- **Combine (Delta Log + Instances):** Combine our invertible 12-op patch log with Figma's component-override tree so analogs store only deterministic patches over a pinned master scaffold (**Idea 3**).

- **Adapt (TeX Line-Breaking):** Adapt Knuth-Plass badness minimization over reversible collapse_sgroup and IUPAC-ranked bond rotations to satisfy fixed print width at 100% scale (**Idea 4**).

- **Eliminate (Vision Loop):** Eliminate PNG round-trips for agents by compiling layout collisions into structured rustc-style fix-it diagnostics in one stdin stream (**Idea 5**).

## 3. Five Defended Ideas

### Idea 1: ChemSel — Deterministic Chemical-Spatial Selectors for Patch Addressing

- **(a) Problem:** Agents and scripts corrupt drawings when atom/bond indices shift across edits or exports ([<u>Ketcher \#5873</u>](https://github.com/epam/ketcher/issues/5873)) or vanish under abbreviations ([<u>typed-smiles</u>](https://typst.app/universe/package/typed-smiles/)), while opaque IDs ([<u>ChemDraw 23.0</u>](https://revvitysignals.com/sites/default/files/2024-02/rs-release-notes-DT-23.0-chemdraw.pdf)) force extra read calls.

- **(b) Analogy:** CSS Selectors / DOM querySelectorAll paired with Tree-sitter AST predicates.

- **(c) Architecture mapping:** Extend the 12-op JSON patch schema so any target field accepts either an explicit ID or a selector object ({"select": "mol\[0\] // smarts(\[c;H1\]1ccccc1Cl):para & spatial(rightmost)", "expect": 1}). Add crate::selector in Rust, reusing mzLab's substructure search and CIP engines for semantic predicates (smarts(...), cip(R), sgroup(Boc)), composed with deterministic geometric tie-breakers (leftmost, outermost). CLI route: mzed select --query ... --json plus inline resolution in mzed patch --batch. Crucially, resolution compiles down to concrete IDs before recording the command log, preserving exact inverses and byte parity.

- **(d) Prior-art & patent check:** Daylight SMARTS and PyMOL/VMD 3D selection algebras query molecules; Molsoft ICM supports stereospecific SMARTS queries like \[C;S1;\$(C(N)C=O)\] ([<u>ICM Release Notes</u>](https://molsoft.com/~eugene/icm/release-notes.html)). No patent or paper was found combining SMARTS + CIP + 2D visual-spatial predicates as an invertible patch-addressing layer in a 2D chemical editor (Google Patents/Scholar searches returned only sequence/docking hits).

- **(e) Cheapest kill test:** Build a 150-LOC prototype supporting 5 predicates (smarts, cip, sgroup, reaction_role, spatial) and run 3 frontier LLMs on 30 surgical-edit tasks (e.g., "fluorinate *para* to chlorine on product 2 without touching reactant 1"). **Kill threshold:** fails to beat numeric-ID addressing by ≥20 percentage points on first-try exact edit accuracy, or selector ambiguity errors exceed 10%.

- **(f) Effort & scope cut:** **M**. If it grows, cut recursive/nested spatial combinators and ship only smarts + cip + reaction_role + expect: N cardinality guard.

### Idea 2: Stereo-Invariant 2D Geometry Kernel (Genuinely New)

- **(a) Problem:** Dragging atoms, mirroring rings, or running 2D cleanup silently inverts (R)/(S) or produces formally ambiguous wedges because editors treat wedges as passive strokes rather than topological invariants ([<u>RDKit \#4041</u>](https://github.com/rdkit/rdkit/issues/4041), [<u>Indigo \#2739</u>](https://github.com/epam/Indigo/issues/2739), [<u>Brecher 2006 ST-1.1.10</u>](https://iupac.qmul.ac.uk/drawing/stereo.html)).

- **(b) Analogy:** Parametric CAD geometric constraint preservation (SolidWorks/FreeCAD) and Rust type-state invariants.

- **(c) Architecture mapping:** Add an optional document/stereocenter setting stereo_lock: "strict" \| "warn" \| "off" (default "strict" for agent batches) and a lock_stereo op storing canonical local parity + enhanced stereo group (ABS/AND1/OR1) + E/Z bond invariant. Add crate::stereo_kernel in Rust, reusing mzLab's CIP and structure-check engines. After *any* geometric mutation (move, rotate, flip, cleanup), the kernel tests parity against the 2D coordinates and deterministically re-solves IUPAC-2006-preferred wedge placement (ST-1.1.1/ST-1.1.2, supporting dual wedge/dash on quaternary centers per [<u>RDKit \#5067</u>](https://github.com/rdkit/rdkit/issues/5067)), emitting synthetic wedge-update sub-ops into the same inverse group. If an edit forces inversion without --allow-stereo-inversion, the CLI returns a distinct exit code (e.g., EXIT_STEREO_VIOLATION = 12).

- **(d) Prior-art & patent check:** Schrodinger's coordgenlibs ([<u>CoordgenMinimizer.h</u>](https://github.com/schrodinger/coordgenlibs/blob/master/CoordgenMinimizer.h)), Clark et al. 2006 ([<u>JCIM 46:1107</u>](https://pubs.acs.org/jcisd8/article/46/3/1107/3465149/2D-Structure-Depiction)), and RDKit's WedgeMolBonds assign wedges during *initial* layout from SMILES/3D, while linters flag errors *post hoc*. No patent or literature hit was found for an interactive/patch-level active stereo invariant kernel that automatically re-solves IUPAC wedges across arbitrary 2D edits and aborts silent inversions.

- **(e) Cheapest kill test:** Take 1,000 chiral ChEMBL molecules, apply 5 random geometric ops per molecule (drag neighbor across bond axis, horizontal mirror, 45° partial rotation, ring flip, cleanup), and verify round-trip Standard InChI stereo layer + IUPAC ST-1.1 legality. **Kill threshold:** \>0.5% parity mismatch or ambiguous-wedge failure, or \>15% increase in user drag rejection frustration in a 5-chemist test.

- **(f) Effort & scope cut:** **M**. If it grows, restrict v1 to tetrahedral centers and E/Z alkenes; defer atropisomers and bridged cage perspective rings.

### Idea 3: Parametric Core-Linked Analog Families ("Chemical Components")

- **(a) Problem:** Keeping a common core pixel-aligned across SAR tables, reaction series, or transformation-product (TP) pathways is broken in ChemDraw 25.5 ([<u>r/chemistry</u>](https://www.reddit.com/r/chemistry/comments/1to4yp5/chemdraw_objects_in_groups_and_how_to_align_them/), [<u>r/OrganicChemistry</u>](https://www.reddit.com/r/OrganicChemistry/comments/1vswtuo/anyone_struggling_with_the_newer_version_of/)) and fails in RDKit's GenerateDepictionMatching2DStructure, which clobbers core coordinates ([<u>RDKit \#1286</u>](https://github.com/rdkit/rdkit/issues/1286)) and flips orientation ([<u>RDKit \#8124</u>](https://github.com/rdkit/rdkit/issues/8124)).

- **(b) Analogy:** Figma Master Components with instance overrides + Git rebase over our deterministic patch log.

- **(c) Architecture mapping:** Unlike Markush/R-group tables (restricted to single-scaffold attachment slots), any canvas molecule can be declared a MasterCore, and derived analogs/TPs—including heteroatom swaps, oxidations, and ring cleavages—are stored as Instance { base: core_id, delta: \[PatchOp\], pin_core: true }. Add crate::component_graph in Rust and CLI routes mzed component define / mzed component derive --from core.svg --patch delta.json (or auto-derived via mzLab's substructure/reaction transformation engines). Editing geometry or style on the Master replays every instance's delta with strictly pinned core coordinates and local peripheral clash relaxation only. Exporting to SVG/V3000/CDXML flattens deterministically while retaining component metadata inside the embedded SVG document.

- **(d) Prior-art & patent check:** RDKit GenerateDepictionMatching2DStructure and template-based MCS aligners (e.g., [<u>Clark et al. 2006</u>](https://pubs.acs.org/jcisd8/article/46/3/1107/3465149/2D-Structure-Depiction)) perform one-shot static coordinate projection; generative scaffold patents like [<u>US12159693B2</u>](https://patents.google.com/patent/US12159693B2/fr) target ML molecule generation, not 2D visual inheritance. Live reactive master-core inheritance via replayable edit deltas appears absent from 2D chemical editors.

- **(e) Cheapest kill test:** Run on 50 real ChEMBL congeneric series (10–25 analogs each) plus 10 environmental TP degradation networks; rotate/restyle each master core and verify all derived structures. **Kill threshold:** core-atom RMSD \>0.0 px after propagation, or peripheral overlap-free rate \<90% without manual cleanup.

- **(f) Effort & scope cut:** **M**. If it grows, disallow multi-level nesting (master → instance only, depth 1) and require explicit re-basing when a core atom referenced by a delta is deleted.

### Idea 4: Budget-Constrained Adaptive Abbreviation Folding — Knuth-Plass Chemical Reflow (Genuinely New)

- **(a) Problem:** When large molecules or multi-step schemes exceed a journal column width or self-collide ([<u>RDKit \#6626</u>](https://github.com/rdkit/rdkit/issues/6626), [<u>\#4508</u>](https://github.com/rdkit/rdkit/issues/4508)), editors either globally shrink drawings—violating IUPAC GR-0.3 legibility and ACS 14.4 pt fixed bond length ([<u>Brecher 2008</u>](https://iupac.qmul.ac.uk/drawing/drawing.html), [<u>Wikipedia MoS</u>](https://en.wikipedia.org/wiki/Wikipedia:Manual_of_Style/Chemistry/Structure_drawing))—or apply indiscriminate all-or-nothing abbreviation catalogues.

- **(b) Analogy:** TeX's Knuth-Plass dynamic-programming line-breaking (minimizing visual badness under an exact box width) and LilyPond horizontal spacing.

- **(c) Architecture mapping:** Add crate::reflow_budget in Rust and CLI route mzed fit --box 82.5mmx60mm --style acs1996 --protect \<ChemSel\>. Using mzLab's substructure search to enumerate candidate collapsible substituents (already supported via abbreviations/S-groups) and single-bond hinges, a deterministic branch-and-bound solver minimizes a weighted badness score: (1) zero overlaps, (2) zero scale reduction below 100%, (3) IUPAC GR-1 rotation preference (ring-adjacent single bonds first, mid-chain next), (4) minimal semantic folding cost (e.g., fold t-Bu/Ph/Boc before core heterocycles; never fold --protect or reaction-center atoms). Output is a standard invertible sequence of rotate_bond and collapse_sgroup JSON ops—reversible with one undo or --expand-all.

- **(d) Prior-art & patent check:** RDKit's CondenseMolAbbreviations ([<u>MolMiner, PMC9710516</u>](https://pmc.ncbi.nlm.nih.gov/articles/PMC9710516/)) and typed-smiles ([<u>Typst Universe, Nov 2026</u>](https://typst.app/universe/package/typed-smiles/)) perform static greedy catalogue matching independent of canvas width or collisions. PoseView ([<u>Bioinformatics 2006</u>](https://academic.oup.com/bioinformatics/article/22/14/1710/226866)) uses combinatorial bond flips for protein-ligand complexes, not chemical abbreviation folding. No patent or paper was found optimizing selective superatom folding + bond rotations under a fixed physical print-box budget.

- **(e) Cheapest kill test:** Take 30 real PubChem/ChEMBL drug molecules and 2-step schemes exceeding 82.5 mm at 14.4 pt bond length; generate (i) globally shrunk ACS output, (ii) greedy all-abbreviations output, and (iii) Knuth-Plass folded output. Have 5 chemists blind-rank publication readiness. **Kill threshold:** naive scaling or greedy folding is preferred in \>40% of pairs, or solver runtime exceeds 200 ms in WASM on 80-atom molecules.

- **(f) Effort & scope cut:** **L**. If it grows, drop continuous multi-step scheme wrapping and ship single-molecule + single-row reaction folding with a curated 40-group ACS/IUPAC abbreviation cost table.

### Idea 5: Headless Transactional Canvas-LSP — Compiler Diagnostics for Blind Agents (Genuinely New)

- **(a) Problem:** Spawning one CLI process per edit incurs a ~95× latency tax (measured: 77 s vs. 0.8 s for 901 calls), yet blind single-shot agents cannot detect 2D overlaps, clipped labels, or stereo clashes without costly PNG-to-VLM loops ([<u>LiveFigure, arXiv 2026</u>](https://arxiv.org/html/2605.23527v1); [<u>RDKit \#4504/#4508</u>](https://github.com/rdkit/rdkit/issues/4504)).

- **(b) Analogy:** rustc structured JSON compiler diagnostics with machine-applicable fix-its + Language Server Protocol (LSP) persistent stdio sessions.

- **(c) Architecture mapping:** Add crate::diagnostics_lsp and a persistent NDJSON stdio route mzed serve --stdio (plus single-call mzed patch --batch --atomic --assert ...). An agent sends a batched transaction with declarative postconditions (no_clash, iupac_stereo, fit_box, valence_ok, lossless_target: "v3000"). Rust executes in memory, runs exact 2D bounding-box/clearance geometry checks + mzLab structure checks, rolls back via exact inverses if --atomic fails, and returns: (i) stable JSON diagnostics with **concrete patch fix-its** (e.g., CLASH_LABEL_ATOM → {"op":"rotate_bond","target":"b4","deg":60}), (ii) a **Format Loss Manifest** before lossy export (e.g., flagging OR stereo dropped by V2000 per [<u>Ketcher \#64</u>](https://github.com/epam/ketcher/issues/64)), and (iii) a \< 300-token semantic-spatial digest (component boxes, clearance min, chiral audit)—replacing 20–200 kB PNG vision calls.

- **(d) Prior-art & patent check:** Existing chemical agent frameworks (ChemCrow, reviewed in [<u>arXiv:2508.17692</u>](https://arxiv.org/html/2508.17692v2)) and scientific illustration agents ([<u>LiveFigure, arXiv:2605.23527</u>](https://arxiv.org/html/2605.23527v1)) rely on SMILES strings or vision-language screenshot feedback loops; CDK issue [<u>\#951</u>](https://github.com/cdk/cdk/issues) only requests warn/error/fatal IO parsing levels. No prior art found for a transactional chemical-canvas stdio server emitting machine-applicable 2D spatial fix-it patches and compact spatial digests for non-visual LLM agents.

- **(e) Cheapest kill test:** Run 3 models (text-only mode with Canvas-LSP vs. vision-enabled mode receiving 300 dpi PNGs) on 30 drawing/editing tasks. **Kill threshold:** Canvas-LSP fails to beat raw batched CLI by ≥15 pp pass@1, fails to match VLM-loop accuracy within 5 pp at \<0.25× token cost, or averages \>2.5 transactions per passing task.

- **(f) Effort & scope cut:** **S–M**. If it grows, cut persistent serve --stdio first and ship single-call --batch --atomic --diagnostics-json with clash/overlap + stereo + loss-manifest checks (already eliminating the 95× per-op spawn overhead).

## 4. Three Popular Features We Should NOT Build

1.  **Full HELM / Cartoon Macromolecule & Biologics Editor:** Ketcher's Flex/Macro mode is a disproportionate source of undo corruption and snapping regressions ([<u>Ketcher \#6377</u>](https://github.com/epam/ketcher/issues/6377), [<u>\#7649</u>](https://github.com/epam/ketcher/issues/7649)), and ChemDraw 23.0's monomer cartoon engine drags in an external monomer database ([<u>ChemDraw 23.0 Release Notes</u>](https://revvitysignals.com/sites/default/files/2024-02/rs-release-notes-DT-23.0-chemdraw.pdf)). Supporting HELM blows past our ~585 kB WASM budget, fragments our 12-op inverse model, and serves a different user base than small-molecule / LC-MS workflows.

2.  **Embedded 3D Conformer / Force-Field / NMR Prediction Suite:** ChemDraw 23.0 cannot even maintain macOS parity for Chem3D and recently removed its Enhanced Retrosynthesis tool ([<u>ChemDraw 23.0 Release Notes</u>](https://revvitysignals.com/sites/default/files/2024-02/rs-release-notes-DT-23.0-chemdraw.pdf)). Iterative 3D minimizers introduce floating-point drift across WASM/x86/ARM64 that breaks byte-for-byte parity, while bloating binary size; keep 3D/spectroscopy in external mzLab modules.

3.  **Proprietary Cloud Sync & Real-Time CRDT Multiplayer:** Revvity's push to cloud-tethered Signals ChemDraw adds login friction and vendor lock-in ([<u>ChemDraw 23.0 Release Notes</u>](https://revvitysignals.com/sites/default/files/2024-02/rs-release-notes-DT-23.0-chemdraw.pdf)). Because our SVG-embedded files are 3–15 kB and command logs are byte-deterministic, standard Git workflows already cover async review and provenance without CRDT state-machine complexity or server dependencies.

## 5. External Validation Protocol

To prove correctness and usability from outside our own codebase:

- **Public Stratified Corpus (*N*=10,000):** Draw 4,000 PubChem compounds ([<u>PubChem</u>](https://academic.oup.com/nar/article/44/D1/D1202/2503131)) stratified across macrocycles, bridged polycyclics, enhanced stereo, and salts; 4,000 ChEMBL drug-like structures pre-checked with [<u>ChEMBL Structure Pipeline checker.py</u>](https://github.com/chembl/ChEMBL_Structure_Pipeline/blob/master/chembl_structure_pipeline/checker.py); and 2,000 Wikimedia Commons chemistry SVGs ([<u>Wikimedia Commons</u>](https://commons.wikimedia.org/wiki/File:Chemical_structural_formulas_of_acyl_group.svg)) plus every positive/negative test case from IUPAC 2006/2008 ([<u>Brecher 2006</u>](https://iupac.qmul.ac.uk/drawing/stereo.html), [<u>Brecher 2008</u>](https://iupac.qmul.ac.uk/drawing/drawing.html)).

- **Independent Toolkit Round-Trip:** Export every corpus item via CLI to V2000, V3000, SMILES, KET, CDXML, and embedded SVG, then parse independently in pinned Docker builds of **RDKit, Open Babel, Indigo, and IUPAC InChI**.

  - *Metrics & CIs:* (1) Exact Standard InChI + stereo-layer agreement ≥99.9% on lossless formats (Wilson 95% CI \[99.82%,99.95%\] at *N*=10,000); (2) zero silent semantic loss (100% of V2000/SMILES mismatches predicted by our Loss Manifest, Clopper-Pearson lower 95% bound ≥99.96%); (3) 0-byte diff across CLI, WASM, and desktop replays.

- **Human Time-and-Motion Test (*n*≥6 Chemists, Balanced 3×3 Latin Square vs. ChemDraw & Ketcher):** 6 realistic tasks (edit stereocenter + preserve chirality, draw 6 aligned analogs, fit a 3-step scheme to ACS single column, fix overlapping S-group, export publication SVG, revise core orientation across series).

  - *KLM Prior Estimate:* Using Card, Moran & Newell (1980) canonical times (*K*=0.20 s, *P*=1.10 s, *H*=0.40 s, *B*=0.10 s, *M*=1.35 s; [<u>Textbook of Usability</u>](https://www.textbookofusability.com/chapter/goms-and-keystroke-level-model.html)), updating a core ring orientation across 6 analogs requires ~6 manual align/group sequences in ChemDraw/Ketcher (≈6×(2*M*+*H*+3*P*+3*B*)=40.2 s minimum) versus 1 Master Core edit in Idea 3 (≈2*M*+4*K*=3.5 s keyboard-only)—an ~11× KLM speedup on series revision.

  - *Metrics & CIs:* Geometric mean completion-time ratio with 95% paired *t*-CI on log(time) (pass: ≥1.3× overall speedup, lower 95% CI \>1.0×); stereochemical/IUPAC error rate (Clopper-Pearson 95% CI); NASA-TLX workload difference.

- **Blind Multi-Model Agent Task Suite:** 60 fixed tasks (20 build-from-text, 20 surgical edits, 20 multi-structure/column-fit) run zero-shot—giving models *only* --help and JSON schema—across 4 distinct external models (e.g., Claude, GPT, Gemini, and an open-weights coding model) against RDKit-Python and Ketcher-API baselines.

  - *Metrics & CIs:* Pooled pass@1 rate (*N*=240 model-task runs; Wilson 95% CI width ≤±6.3 pp; pass target ≥85% overall and ≥+15 pp over baselines), median wall-clock time, median CLI calls per task (target ≤2.0, bootstrapped 95% CI), and median total tokens per passing figure (target ≤0.25× PNG-VLM baseline).

## 6. Unverified Claims & Verification Gaps

- **GitHub API reaction-count sorting (UNVERIFIED):** Web search retrieved individual issues from Ketcher, RDKit, Indigo, CDK, and Open Babel, not live GitHub API queries sorted globally by reaction count or total issue frequency across each tracker.

- **Test-suite internals of RDKit, Open Babel, Indigo, CDK, and Ketcher (UNVERIFIED):** Source test files were not read end-to-end; claims rely on linked GitHub issue threads and commit references.

- **ACS exact 2026 single-column artwork width (UNVERIFIED):** The 82.5 mm (3.25 in) single-column figure width used in Idea 4's CLI example reflects standard ACS artwork conventions but was not read directly from a current publish.acs.org page in this session; ACS Document 1996 bond length (0.508 cm / 14.4 pt) was verified via Wikipedia's Chemistry MoS rather than Revvity's proprietary .cds file.

- **Full CDXML, KET, and ChemDraw 25.5 release-note specifications (UNVERIFIED):** ChemDraw 25.5 alignment regressions were read from May/August 2026 user forum reports, not official Revvity 25.5 release notes (only ChemDraw 23.0 release notes were read directly).

- **Exhaustive patent clearance (UNVERIFIED):** Google Patents and Google Scholar searches returned no blocking hits for Ideas 2, 4, or 5, but keyword web search is not a formal freedom-to-operate (FTO) search across CPC classes G16C20/80 and G06F3/048.

**Sources**

1.  [SAVE and OPEN STRUCTURE with STEREO MARKS \#64 - GitHub](https://github.com/epam/ketcher/issues/64)

2.  [AllChem.GenerateDepictionMatching2DStructure gives flipped](https://github.com/rdkit/rdkit/issues/8124)

3.  [Wikipedia:Manual of Style/Chemistry/Structure drawing](https://en.wikipedia.org/wiki/Wikipedia:Manual_of_Style/Chemistry/Structure_drawing)

4.  [A Multi-Agent System for Multimodal Reasoning on IChO 2025 - arXiv](https://arxiv.org/html/2511.16205v2)

5.  [Graphical Representation of Stereochemical Configuration](https://iupac.qmul.ac.uk/drawing/stereo.html)

6.  [US12159693B2 - Computational platform for generating molecules](https://patents.google.com/patent/US12159693B2/fr)

7.  [Molecular complexes at a glance: automated generation of two](https://academic.oup.com/bioinformatics/article/22/14/1710/226866)

8.  [Issues · cdk/cdk - GitHub](https://github.com/cdk/cdk/issues)
