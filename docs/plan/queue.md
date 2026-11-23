# Prompt queue: one row per prompt

This file is the single source of truth for the state of every file of `docs/prompts/`. `tools/queue_check.sh` (a CI step) fails if a prompt file has no row here, or a row names a prompt file that does not exist. A PR that merges a prompt, supersedes or withdraws one, adds a prompt file or changes a gate edits its row in the same PR.

Status:
- **MERGED**: a commit on `main` (PR number and short commit given) and evidence in the code or in a `docs/built/` note. Never a guess.
- **RUNNING**: a PR is open for it. A running prompt shows no merge evidence yet.
- **QUEUED**: has a prompt, not built, no gate. "Evidence" says only that no merged commit or built note names it.
- **SUPERSEDED**: another prompt runs instead (the top of the file says "superseded: do not run").
- **WITHDRAWN**: dropped by the backlog (`docs/plan/backlog.md` section 3).
- **GATED**: waits for a decision or a run outside the repository.

Checked against `main` at `4d39021` (2026-11-21). Prompt files: 107. MERGED 34, RUNNING 3, QUEUED 51, SUPERSEDED 15, WITHDRAWN 3, GATED 1. The order and the reasons of the plan are in `docs/plan/backlog.md`; "depends/gate" is the "Depends on" line of the prompt, cut.

| prompt id | title | status | evidence | depends/gate | note |
|---|---|---|---|---|---|
| `arch-cd-01-clean-up` | Clean Up Structure: the 2D layout of mzlab-mol on a drawing, stereo kept | SUPERSEDED | by arch-std-15 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-02-cip-labels` | CIP labels on the canvas (R/S, r/s, E/Z) from cip/1, live | SUPERSEDED | by arch-web-16 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-03-hover-hotkeys` | Hotkeys over the hovered bond and group hotkeys over an atom | QUEUED | no merged commit or built note names it | arch-std-06 (the command table of shortcuts) if it is on `main`: put the hover keys into that table | - |
| `arch-cd-04-bond-display` | Double-bond position (centre, left, right) and dashed bond displays | RUNNING | PR #108 (docs/built/bond-display.md) | the bond field `k` (one field for bold, dashed, dative, hbond; V3000 types 9 and 10: COO decision 16), from... | - |
| `arch-cd-05-chain-tool` | Chain tool: drag out a zigzag, the carbon count shown while dragging | SUPERSEDED | by arch-web-08 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-06-arrow-kinds` | Arrow kinds, heads and conditions under the arrow | SUPERSEDED | by arch-web-13 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-07-rich-text` | Rich text: several lines, sub- and superscript runs, bold, italic, formula style | SUPERSEDED | by arch-web-14 (itself withdrawn for arch-std-21) (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-08-radical-isotope-table` | Periodic table picker, and keys for radicals and isotopes | QUEUED | no merged commit or built note names it | nothing; uses arch-web-11's popover when it is on `main` | - |
| `arch-cd-09-atom-popover` | Atom popover: element, charge, isotope, radical, hydrogens, label in one place | SUPERSEDED | by arch-web-11 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-10-abbrev-expand` | Expand and contract an abbreviation in place | SUPERSEDED | by arch-web-18 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-11-user-abbreviations` | User abbreviations in the settings file, and composed labels | SUPERSEDED | by arch-web-18 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-12-templates` | Template library: groups of ready fragments placed like rings | SUPERSEDED | by arch-web-19 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-13-user-templates` | User templates: save the selection, rename, remove, share as a file | SUPERSEDED | by arch-web-19 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-14-analysis-panel` | Analysis panel: formula, MW, exact mass, m/z, elemental analysis of the selection | SUPERSEDED | by arch-web-17 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-15-align-distribute` | Align, distribute, rotate by a typed angle and scale the selection | QUEUED | no merged commit or built note names it | arch-std-15 (op `atoms.xy`, PR #45) for scaling | - |
| `arch-cd-16-smart-paste` | Smart paste: pasted parts land in a free place, never on the drawing, without a rezoom | QUEUED | no merged commit or built note names it | nothing | - |
| `arch-cd-17-shortcuts` | Rebindable shortcuts in the settings file, conflicts reported, a printable table | SUPERSEDED | by arch-std-06 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-18-palette-tear-off` | Palettes, refinement 1 on top of R1 (PR #35): tear-off flyouts, home on double-click, t... | QUEUED | no merged commit or built note names it | PR #35 (R1, `crates/mzdraw/src/palettes.rs`, `docs/mzdraw-ui.md` §4-§5) merged on `main` | built/palettes-page.md has floating and dockable palettes but no tear-off flyouts or named layouts |
| `arch-cd-19-named-layouts` | Palettes, refinement 2 on top of R1 (PR #35): named layouts and icon size | QUEUED | no merged commit or built note names it | PR #35 merged; arch-cd-18 if it touches the same code | built/palettes-page.md has floating and dockable palettes but no tear-off flyouts or named layouts |
| `arch-cd-20-context-menu-refine` | Context menu, refinement on top of PR #37: order, abbreviation target, touch and keyboa... | QUEUED | no merged commit or built note names it | PR #37 (context menu model) and its page PR 2 merged on `main` | CTX-MENU page PR #67 is merged, so its dependency is met |
| `arch-cd-21-embed-api` | Embedding API: the editor in another page with a small, stable interface | SUPERSEDED | by arch-web-01, arch-web-02, arch-web-03 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-22-desktop-shell` | Desktop app, first slice: one window for macOS and Windows around the same module | QUEUED | no merged commit or built note names it | a decision of the owner on the shell (see work 1) | - |
| `arch-cd-23-dative-hbond` | Dative (coordinate) and hydrogen bonds | SUPERSEDED | by arch-web-20 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-24-enhanced-stereo` | Enhanced stereo groups (ABS, AND, OR): assign, show, V3000 in and out | SUPERSEDED | by arch-std-12 (top of the prompt: "superseded: do not run") | - | - |
| `arch-cd-25-curved-arrows` | Electron-pushing curved arrows anchored to atoms, bonds and lone pairs | QUEUED | no merged commit or built note names it | arch-web-13 (arrow kinds) if merged | - |
| `arch-cd-26-history-panel` | History panel: every step of the session, jump back to any | QUEUED | no merged commit or built note names it | nothing | - |
| `arch-std-01-std-report-stereo` | conformance report `std/1`, stereo checks | MERGED | PR #46, `89011cc`; built/std-stereo.md; `conform.rs` | - | - |
| `arch-std-02-std-report-geometry` | conformance report, geometry and label checks | QUEUED | no merged commit or built note names it | - | - |
| `arch-std-03-palette-contrast` | element palettes with tested contrast, dark background | QUEUED | no merged commit or built note names it | - | UI-2 PR 3 (#64, built/tokens.md) is the prerequisite and is not this prompt |
| `arch-std-04-svg-accessible` | saved SVG with a text alternative | MERGED | PR #40, `fb46a0b`; built/svg-text-alternative.md | - | - |
| `arch-std-05-drag-free-editing` | move, rotate and select without dragging | MERGED | PR #44, `37369ca`; built/drag-free-editing.md | - | - |
| `arch-std-06-shortcut-settings` | one-key shortcuts that can be turned off and remapped | QUEUED | no merged commit or built note names it | - | built/proposal-layer.md and built/tp-06-pathway-page.md say its command table is not on main |
| `arch-std-07-page-a11y-audit` | page audit against WCAG 2.2 AA, fixed where it fails | QUEUED | no merged commit or built note names it | - | - |
| `arch-std-08-label-reversal` | group labels reversed on the left side | MERGED | PR #43, `5a3e3b9`; built/group-labels-reversed.md | - | - |
| `arch-std-09-opensmiles-files` | SMILES files read as the OpenSMILES file rules say | RUNNING | no merge evidence yet | PR #107 | - |
| `arch-std-10-label-display-modes` | carbon labels, hydrogen modes and the hash style | QUEUED | no merged commit or built note names it | - | - |
| `arch-std-11-unknown-double-bond` | double bonds of unknown geometry | QUEUED | no merged commit or built note names it | - | - |
| `arch-std-12-enhanced-stereo` | enhanced stereo groups: document, operation, drawing | QUEUED | no merged commit or built note names it | - | - |
| `arch-std-13-crossing-gap` | a gap in the rear bond where two bonds cross | QUEUED | no merged commit or built note names it | - | - |
| `arch-std-14-ann-scales` | identification scales on the page | MERGED | PR #42, `8d5748f`; built/annotations.md; `ann.rs` | - | - |
| `arch-std-15-clean-up` | Clean up a drawing with the layout of `mzlab-mol` | QUEUED | no merged commit or built note names it | - | op `atoms.xy` is not in the table yet (built/stereo-guard.md) |
| `arch-std-16-export-profiles` | export profiles of the journals that could be read | QUEUED | no merged commit or built note names it | - | - |
| `arch-std-17-export-default` | downloads in every format, SVG on white by default | QUEUED | no merged commit or built note names it | - | built/loss-manifest.md lists it as not built |
| `arch-std-18-pdf-writer` | PDF out: vector, text as text, the document attached | MERGED | PR #63, `fb8832d`; built/arch-std-18-pdf-writer.md; `pdf.rs` | - | - |
| `arch-std-19-enhanced-stereo-files` | enhanced stereo groups in molfile V3000 and KET | QUEUED | no merged commit or built note names it | - | - |
| `arch-std-20-enhanced-stereo-cxsmiles` | enhanced stereo groups from and to CXSMILES | QUEUED | no merged commit or built note names it | - | - |
| `arch-std-21-text-runs` | texts with subscripts, superscripts and several lines | QUEUED | no merged commit or built note names it | - | - |
| `arch-std-22-cdx-read` | read ChemDraw binary CDX files | MERGED | PR #62, `27faebf`; built/std-22-cdx-read.md; `cdx.rs` | - | - |
| `arch-web-01-instances` | Editor instances in the wasm bridge | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-02-element` | The custom element `<mzdraw-editor>` | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-03-host-adapter` | Host adapter: files and storage delegated to the host | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-04-theme-i18n` | Host text and colours: the `t` hook, host theme tokens into the palette, reduced motion | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-05-viewer-form` | Read-only viewer and form association | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-07-selection` | Lasso, fragment selection, a band without dragging, focus on bonds and objects | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-08-chain` | Chain tool and the op `chain` | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-09-merge` | Merge atoms on drop and join pieces: the op `merge` | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-10-clean-up` | Clean up the drawing or the selection: the op `place` | WITHDRAWN | = arch-std-15 (one op `atoms.xy`) (backlog section 3) | - | - |
| `arch-web-11-atom-popover` | Atom properties popover (any element, isotope, radical, hydrogens, charge, label); doub... | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-12-context-menu` | Context menu decided in Rust | WITHDRAWN | = CTX-MENU (PR #37 model, #67 page) (backlog section 3) | - | - |
| `arch-web-13-arrow-kinds` | Arrow kinds and the text under the arrow | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-14-rich-text` | Multi-line text with runs: subscript, superscript, bold, italic | WITHDRAWN | = arch-std-21 (text runs) (backlog section 3) | - | - |
| `arch-web-15-highlight-marks` | Highlight colours in the document and host marks over it | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-16-cip` | CIP labels drawn next to the stereocentres | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-17-values` | Calculated values: formula, masses, m/z of adducts | MERGED | PR #88, `c713c2b`; built/arch-web-17-values.md; `values.rs` | - | - |
| `arch-web-18-abbreviations` | Abbreviations: expand and contract in place, user abbreviations | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-19-templates` | Template library and user templates: the op `tmpl` | QUEUED | no merged commit or built note names it | - | - |
| `arch-web-20-bond-kinds` | Bond kinds: bold, dative, hydrogen bond | RUNNING | PR #115 (docs/built/arch-web-20-bond-kinds.md) | - | the field `k` and `bonddisp.rs` (`k_canon`, `KINDS`) already exist from arch-cd-04 (PR #108) with `dash`, `bdash`, `hash`: extend them with `bold`, `dative`, `hbond`, do not add a second field |
| `arch-web-21-drop-copy-as` | Drop a file on the canvas; copy as MOL / KET / SMILES / image; PNG and HTML on the clip... | QUEUED | no merged commit or built note names it | - | - |
| `plan-01-op-table` | one table of the `draw_edit/1` operations, served as JSON | MERGED | PR #52, `fd41029`; `schema.rs`; built/edit-schema.md | none (COO 16 agreement on ownership, backlog D1) | - |
| `plan-02-velocity-bench` | drawing-speed benchmark `velocity/1` on a fixed task set | MERGED | PR #50, `2510e86`; built/velocity.md; `tools/velocity` | none | - |
| `plan-03-fidelity-ref` | format fidelity checked by RDKit, Open Babel and InChI | MERGED | PR #61, `df133d7`; built/fidelity-ref.md; `tools/fidelity` | none (backlog D5 on fetched corpora) | - |
| `plan-04-glue-native` | engine glue callable natively, so parity covers mzLab engines | MERGED | PR #48, `90c4179`; built/engine-glue.md; `glue.rs` | none (CTX-MENU PR #37, which also touched the bridge, is merged) | - |
| `plan-05-cut-object` | a cut line across bonds, as an object of the document | QUEUED | no merged commit or built note names it | plan-01 (table entry) | - |
| `plan-06-cut-masses` | formula and exact m/z of the pieces of a cut | QUEUED | no merged commit or built note names it | plan-04, plan-05 | - |
| `plan-07-cut-peaks` | a peak list in, the cuts that explain each peak out | QUEUED | no merged commit or built note names it | plan-06 | - |
| `plan-08-mech-check` | live form check of curved arrows through `mechanism/1` | QUEUED | no merged commit or built note names it | arch-cd-25 (anchored curved arrows and lone pairs), plan-04 | - |
| `plan-10-survival-matrix` | which hosts keep the document inside our pictures | QUEUED | no merged commit or built note names it | arch-std-18 (PDF writer) | - |
| `plan-11-atom-maps` | atom-to-atom maps on atoms, read and written in reaction files and SMILES | MERGED | PR #66, `6889ebf`; built/atom-maps.md | arch-std-11 (same zone, runs before) | - |
| `plan-12-trasf-out` | a drawn parent → product saved as a `trasf/1` transformation record | MERGED | PR #76, `ef43add`; built/trasf-out.md; `trasf.rs`; follow-up #77 d54fd0a | plan-11 | - |
| `plan-13-scheme-row` | lay out a one-line reaction scheme | MERGED | PR #68, `04082a1`; built/scheme-row.md; `scheme.rs` | none | - |
| `plan-14-scheme-orient` | keep a shared core oriented the same way across a scheme | MERGED | PR #75, `b11c92a`; built/scheme-orient.md | plan-13 | - |
| `plan-15-diff-core` | semantic diff of two documents as a `draw_edit/1` patch | MERGED | PR #73, `2c60412`; built/diff-core.md; `diff.rs` | plan-01 (can start in parallel; rebase nothing, add ops to its table when it merges) | - |
| `plan-16-proposal-layer` | an agent's proposal shown as a layer, accepted or rejected hunk by hunk | MERGED | PR #81, `9546a3b`; built/proposal-layer.md; `proposal.rs` | plan-15, UI-2 PR 2 | - |
| `plan-19-agent-suite` | a fixed agent task suite, the external test of the agent route | MERGED | PR #65, `55267a7`; built/agent-suite.md | plan-01 (the `"out":"schema"` text given to the models; until it merges, give the op list of `ops.rs` by hand) | - |
| `plan-20-result-envelope` | the result of a replay request: created and removed ids, the failing op, dry run and po... | MERGED | PR #59, `246669a`; built/result-envelope.md; `outcome.rs` | plan-01 (the request rows of the table) | - |
| `plan-21-stereo-guard` | stereo guard: a geometry edit that changes a configuration is reported | MERGED | PR #74, `2f58c3a`; built/stereo-guard.md; `stereo_guard.rs` | plan-20 (the `expect` arm), arch-std-01 (built) | - |
| `plan-22-loss-manifest` | what a format loses, listed before the file is written | MERGED | PR #78, `3180850`; built/loss-manifest.md; `loss.rs` | plan-20 (the `expect` arm), plan-15 (diff, to compare a round trip) | - |
| `plan-23-digest` | a compact text digest of the drawing for agents that cannot see | MERGED | PR #82, `920679d`; built/plan-23-digest.md; `digest.rs` | plan-20 (an arm of the same runner); arch-std-02 helps (clearance findings) but is not needed | - |
| `plan-24-selectors` | chemical selectors that resolve to ids before the log | GATED | - | plan-20, plan-23, and the first plan-19 run (see the start condition) | start condition in the prompt: the first plan-19 `--arm report+digest` model run (the owner runs it; none exists, see built/agent-suite.md) must show addressing errors in at least 3 of 10 surgical-edit tasks, else the row is dropped |
| `plan-25-fix-its` | exact fix-its on drawing-check findings | QUEUED | no merged commit or built note names it | arch-std-02 (geometry checks), arch-std-15 (`atoms.xy`), plan-20 | not model-gated; waits for arch-std-02 and arch-std-15 |
| `plan-26-flip-keep` | a flip that keeps the configuration | QUEUED | no merged commit or built note names it | plan-21 (the signature, for the tests) | plan-21 (its dependency) is merged; no model-run gate in the prompt |
| `plan-27-goal-plan` | one goal in, one canonical reversible patch out (`goal`) | QUEUED | no merged commit or built note names it | plan-01 (op table), plan-20 (`draw-result/1`; edit E21 of §10.5) | - |
| `plan-28-grant` | a scope grant checked on every op, and per-op contracts | QUEUED | no merged commit or built note names it | plan-20; plan-24 for a scope given by selector (optional) | - |
| `plan-29-stereo-tier` | the stereo tier of each centre, never assumed | QUEUED | no merged commit or built note names it | arch-std-12 (`stg`), arch-std-19 with edit E20 (§10.5), plan-22 (loss manifest), plan-23 (digest) | - |
| `plan-30-log-chain` | a hash chain over the command log | QUEUED | no merged commit or built note names it | plan-20 (the `expect` arm) | - |
| `plan-31-chain-in-pictures` | the chain head carried in saved SVG and PNG | QUEUED | no merged commit or built note names it | plan-30, arch-std-16 (export profiles), arch-std-17 (export settings) | - |
| `tp-01-arrow-anchors` | arrows that know their structures (anchors and follow) | MERGED | PR #60, `97ff8f3`; built/arrow-anchors.md; `route.rs` | plan-01 (op table and its `request` section), plan-20 (`draw-result/1`; this slice applies E29 to it if pla... | - |
| `tp-02-anchor-tools` | anchoring by hand: the arrow tool, `pathway:anchor`, `pathway:route` | MERGED | PR #70, `2d63d4d`; built/pathway-tools.md | tp-01; the CTX-MENU page PR (the context-menu items; without it they are model-only and reached from the ke... | - |
| `tp-03-derive` | derive a transformation product from its parent (scaffold kept) | MERGED | PR #80, `f9c347d`; built/tp-derive.md; `tp.rs` | tp-01, tp-02, tp-05 (the settings `pathway.col_gap` and `pathway.row_gap`), plan-11 (atom maps), plan-20, t... | - |
| `tp-04-diff-lens` | the diff lens (what changed from the parent, shown on the TP) | MERGED | PR #86, `8e58da5`; built/tp-lens.md | tp-03 | - |
| `tp-05-pathway-layout` | pathway layout core: arrows that organise themselves (deterministic, symmetric) | MERGED | PR #71, `86db11f`; built/pathway-layout.md; `pathway.rs` | tp-01 (anchors, routing, `commands`), plan-20 (`draw-result/1`) | - |
| `tp-06-pathway-page` | pathway layout on the page, and the Graphviz proof | MERGED | PR #79, `fcf2478`; built/tp-06-pathway-page.md | tp-05, tp-02 | - |
| `tp-07-delta-labels` | arrow labels computed from the two structures (+O, +15.9949) | MERGED | PR #83, `246fa6d`; built/tp-07-delta-labels.md; `steplabel.rs` | - | - |
| `tp-08-tp-import` | import a TP list and number the TPs | MERGED | PR #85, `f7962c6`; built/tp-import.md; `tp_list.rs` | tp-01, tp-05 (layout after import), plan-20 | - |
| `tp-09-tp-table` | the TP table out (JSON, CSV) and the SDF round trip | MERGED | PR #89, `1955d5b`; built/tp-09-tp-table.md; `tptable.rs` | tp-08, tp-07 (deltas), arch-web-17 (masses and adduct m/z through the glue, E6) | - |
| `tp-10-named-steps` | named transformation steps at a chosen site (table, sites, derive by step) | MERGED | PR #87, `9c4eea4`; built/tp-10-named-steps.md; `tp/steps.rs` | tp-03 (derive) | - |
| `tp-11-step-field` | the step field on the page (type a step, pick a site, derive) | MERGED | PR #90, `4d39021`; built/tp-11-step-field.md; `editor_step.rs` | tp-10, and tp-10's kill test passed; the CTX-MENU page PR (the item that opens the field by touch and pen) | - |

## Follow-ups without a prompt

Work that is named in `docs/plan/backlog.md` or in a "Not built" line of `docs/built/*.md` and has no file in `docs/prompts/`. A row here moves to the first table when a prompt is written for it. Status RUNNING means a PR is open.

| follow-up | source | status | note |
|---|---|---|---|
| struct-props: an isotope-pattern line under the molecule | `docs/built/struct-props.md` Not built (`analysis/1` has the pattern) | QUEUED | no prompt file |
| struct-props: values of a lasso part of a structure pinned on the canvas | `docs/built/struct-props.md` Not built | QUEUED | no prompt file |
| struct-props: a dedicated values dialog | `docs/built/struct-props.md` Not built | QUEUED | no prompt file |
| struct-props / mzLab: `sel` reads V2000 only, so a structure of 1000 atoms or more gets mass 0.0000 with no error (mzDraw now says `error.draw2.valuesZero`); the engine should take V3000 or refuse with its own key | `docs/built/struct-props.md` | QUEUED | for mzLab, not fixed here |
| arch-web-17 values: `values:{"atoms":[99]}` (an atom the drawing lacks) answers `selection: null` with `error: null`; should be a keyed error like `props` (`error.draw2.noAtom`) | `docs/built/struct-props.md` review notes | QUEUED | no prompt file |
| Five further stereo checks: `std.stereo.fischerCross`, `reentrant`, `ringBond`, `hiddenH`, `relLabel` | built/std-stereo.md | QUEUED | not defined in the study yet |
| Geometry, label and export checks beyond arch-std-02, annotation and contrast checks, any automatic fix | built/std-stereo.md | QUEUED | partly the prompts arch-std-02, plan-25 |
| Automatic atom mapping `map:auto`; maps in CDXML and CML | built/atom-maps.md; backlog (deferred) | QUEUED | needs an engine decision |
| Re-solving wedge placement (IUPAC ST-1.1), atropisomers, allenes, cumulenes in the stereo guard | built/stereo-guard.md; backlog (automatic wedge re-solving) | QUEUED | |
| `scheme:orient-series` (E18) and a maximum-common-substructure engine (cases: propylbenzene to propylcyclohexane, indane to tetralin) | built/scheme-orient.md | QUEUED | needs mzLab (COO 16) |
| Matching by structure when ids differ in the semantic diff; merging two proposals; bringing a hunk back after an undo | built/diff-core.md, built/proposal-layer.md | QUEUED | |
| PDF: embedded TrueType fonts, PDF/A, several pages, characters outside WinAnsi, compressed streams | built/arch-std-18-pdf-writer.md | QUEUED | |
| CDX writer (CDX inside Office documents, Windows clipboard) | built/std-22-cdx-read.md; backlog (deferred, after std-22) | QUEUED | |
| Fidelity: InChI 1.07.5 not measured, no KET reader in RDKit or Open Babel | built/fidelity-ref.md | QUEUED | tool work |
| Values: an `Alt+C` key for the values dialog, copy of a selection with numbers (the isotope pattern, the lasso values and the dialog itself are the struct-props rows above) | built/arch-web-17-values.md | QUEUED | |
| Equal molecules in a TP import freeze the page (6 or more equal molecules): does not reproduce; regression tests and a pin of the callers of `Mol::canonical()` added | built/tp-import-equal.md | RUNNING | PR #104 |
| `mzlab-mol` `Mol::canonical()` doubles in time per equal component on a whole drawing (benzene, debug: 6 ms at 4 copies, 87 ms at 8) | built/tp-import-equal.md | QUEUED | for mzLab (COO 16); mzDraw does not call it on a whole drawing |
| TP table: adducts beyond the seven of `display.adducts`; the table drawn on the canvas | built/tp-09-tp-table.md; backlog 11.1 (deferred) | QUEUED | |
| TP table: `InChIKey` is NOT wanted now (owner decision; there is no InChI in `mzlab-mol`): do not build it, do not fake it | built/tp-09-tp-table.md ("Not built") | GATED | reopens only if the owner asks |
| TP: uncertain L3 TP as a site mark mapped onto `trasf/1` uncertain form; "metro" mode of `pathway:tree` | backlog 11.1 (deferred) | QUEUED | after plan-12, tp-06 |
| `trasf/1`: uncertain form, several products, `evidence`, `stereo_rule`, `level` | built/trasf-out.md | QUEUED | |
| tp-06 and tp-11 kill tests (the owner's judgement); the key binding for the pathway command and the step field | built/tp-06-pathway-page.md, built/tp-11-step-field.md | GATED | waits for arch-std-06 and the owner |
| Agent suite: a first model run by the owner; arm texts `select`, `plan`, `grant`; adversarial set of plan-28 | built/agent-suite.md | GATED | the owner runs external models outside CI |
| Rename: literals in crate names, manifests, tests, `tools/`, `docs/` and file names are moved by the rename tool, not covered by the Rust guard | built/brand.md | QUEUED | |
| Slice 2 leftovers: forced-colours check, a pass with a real screen reader, RXN export, import of abbreviations from files | built/still-to-build-slice-2.md (this note is itself partly stale: CDXML, RXN and CML are built) | QUEUED | |
| Markush / SAR | backlog deferred; CLAUDE.md rule 13 | GATED | out of scope by rule 13, the owner decides |
| Catalytic-cycle layout (after arch-cd-25); Explain-CIP trace and the next mechanistic intermediate (mzLab engine, COO 16) | backlog deferred | GATED | needs mzLab |
| Font outlining in files | backlog deferred | GATED | font licence |
| Live isomer re-ranking (after plan-07); live analog families (master/instance); abbreviation folding to fit a box; selectors inside ops; the stdio server; time-and-motion study with chemists | backlog deferred (sections 4, 9) | QUEUED | after the named prompts |
| tp-09 TP table: the guard `'` for cells that start with `=`, `+`, `-`, `@` is not injective on import (a cell that really starts with `'` cannot be told from a guarded one) | the CSV writer of `tptable.rs` (the function that adds the guard) and the CSV reader of `tp_list.rs` (by name; no line numbers) | RUNNING | PR #105 |
| tp-09 TP table: NORMAN-SLE spellings (`form_diff` as `H-2O1`, counts of 1 in `formula`, no plus sign in `mass_diff`, one row per parent and per TP, S66 column names) | built/tp-09-tp-table.md, "The kill test" items 1-5 | QUEUED | a change of the column table, only if the owner's workflow needs it |
| tp-09 TP table: doc and header comments drift from the code | built/tp-09-tp-table.md; `crates/mzdraw/src/tptable.rs` | RUNNING | PR #105 |
| tp-09 TP table: the kill test is the owner's plan-19 run (paste the CSV into the SI table workflow once); not run | built/tp-09-tp-table.md, "The kill test" | GATED | the owner runs it outside CI |
| tp-11 step field: the e2e "step field by touch" long-press flaked once on CI (a rerun passed) | `tests/e2e.mjs`; built/tp-11-step-field.md | QUEUED | find the timing cause; never loosen the test (rule 5) |
| tp-11 step field: Ctrl+Alt+Z / A / D / Y typed inside the step input has no test (the field takes Ctrl+Alt only for `[` `]` `{` `}`) | built/tp-11-step-field.md, "Not built"; `crates/mzdraw/src/editor_step.rs` | RUNNING | PR #105 |
| tp-11 step field: the 64-character truncation of `step_text` is untested | `crates/mzdraw/src/editor_step.rs` (`step_text`); `tests/step_field.rs` | RUNNING | PR #105 |
| tp-11 step field: the step masses are derived by element arithmetic from the `Measurer` (seven engine calls the first time, then cached): document it or simplify | `crates/mzdraw/src/editor_step.rs` (`step_masses`); `values::element_masses` | RUNNING | PR #105 |
| `tests/feedback.rs` writes the repository URL as a literal: derive it from `brand::REPOSITORY` | `crates/mzdraw/tests/feedback.rs:63`; `brand::REPOSITORY` | RUNNING | PR #105 |
| tp-05b gap rule: crossings inside a column are not addressed; the step 0.05 and the bound 400 are decisions of the slice (secondary and skipping arrows: done in PR #106, built/tp-followups-2.md) | built/tp-05b-gap-rule.md, "Not built" (PR #92, merged `6a16c67`) | RUNNING | PR #106 |
| mzlab-mol: an optional `map` on atoms, so atom maps do not live only in mzDraw | built/atom-maps.md | QUEUED | to be specified: needs COO 16; the note names no request |
| BRAND: Rust identifiers and paths, manifests, tests, `tools/`, `docs/` and file names (`web/mzdraw.js`, `web/mzdraw.css`, `web/lang/mzdraw.js`) are not covered by `brand_no_literals`; the rename tool moves them | built/brand.md, "Not built"; built/rename-tool.md | QUEUED | see also the Rename row above |
| to_smiles keeps its cap of 500 atoms per structure (a larger structure has no SMILES cell in the TP table) | built/tp-09-tp-table.md, "Limits" | RUNNING | PR #105 |
| `proposal.rs` `one_remove_naming_every_atom...`: absolute wall-clock bound failed once on a loaded CI runner (21290 ms, run 44930069336); move it to the thread-CPU clock / work counter like #102, never loosen (rule 5) | `crates/mzdraw/tests/proposal.rs` | RUNNING | PR #111; built/flaky-tests-2.md |
| `Editor::scene()` of a plain 20,000-atom chain is not linear in a debug build (about 0.8 s for 5,000, 10.7 s for 20,000 atoms, no proposal needed); `replay` import of a 20,000-atom document is superlinear too | built/flaky-tests-2.md, "A finding" | QUEUED | to be profiled: the scene build or the captions; not fixed in PR #111 |
| mass-label-ui: `Editor::import` and paste add structures with new ids and drop the document-level fields, so a `labels` choice in an imported or pasted document is lost (opening a file keeps it; a log carries it with `doc.labels`) | `docs/built/mass-label-ui.md` Limits | QUEUED | no prompt file |
| plan-22 loss manifest: the independent check against the InChI of plan-03 | built/loss-manifest.md, "Not built" | QUEUED | a numbered follow-up in the PR of plan-22 (#78); not read here |
| plan-16 proposal layer: a parity proof of the cases that have `commands` (the independent check skips them) | built/proposal-layer.md; `tests/parity.mjs:130` | QUEUED | the commands run before the proposal, so a plain log cannot say the same |
| `Op::ObjSet`: a possible per-op scan in the proposal code | `crates/mzdraw/src/proposal.rs` (the `Op::ObjSet` match arms of the hunk marks and of the grouping) | QUEUED | to be measured: the cited places are match arms, no scan is proven and no built note measures it |
| tp-04 diff lens: `lens/1` has no JSON Schema file (`docs/schemas/` holds `draw-result-1.json` and `tptable-1.json` only) | built/tp-lens.md; `docs/schemas/` | RUNNING | PR #106: `docs/schemas/lens-1.json` and its key-order test |
| PFAS level texts (`draw2.ann.level.charbonnet-2022.*`) are not checked against Charbonnet et al. 2022 (Environ. Sci. Technol. Lett. 9, 473) | `crates/mzdraw/src/ann.rs` (`PFAS_LEVELS`); built/annotations.md | QUEUED | verify at the primary source before any claim; none made here |
| #44 drag-free editing: `describe_move` writes English words ("right", "left", "up", "down") in Rust instead of i18n keys | `crates/mzdraw/src/editor_nudge.rs:224`; built/drag-free-editing.md | QUEUED | to be specified: which texts reach the page unkeyed |
| plan-03 fidelity: the reverse direction is not byte-checked by the Rust test; Open Babel makes no InChI of a structure with an R group (14 not compared); public-corpus rates were run once by hand and not committed (E19) | built/fidelity-ref.md, "Not built" | QUEUED | tool work; extends the Fidelity row above |
| Text panel: the last assertion of `a_toggle_that_would_pass_the_run_cap...` compares a document with itself; make it compare with the document before the toggle | `crates/mzdraw/tests/text_panel.rs`; built/text-panel.md (#99) | QUEUED | test-only |
| Text panel: tests for 1000 combining marks, emoji skin tones and a lone ZWJ at the start of a text | built/text-panel.md ("Visible characters"); `crates/mzdraw/tests/text_panel.rs` | QUEUED | test-only |
| Text panel: the pointer snap to a cluster is tested only at `snap` level, not through the page | built/text-panel.md; `crates/mzdraw/src/textedit.rs` | QUEUED | an e2e step |
| Text panel: `textedit::FONT_KEYS` is a hand-written copy of the keys of `FONTS` | `crates/mzdraw/src/textedit.rs`; built/text-panel.md | QUEUED | derive one from the other |
| Text panel: it acts on text objects only, not on the text of an arrow (F2 on an arrow gives `error.draw2.textArrow`) | built/text-panel.md, "Not built" | QUEUED |  |
| Text panel: grapheme limits (flags, Hangul, Indic scripts) are outside `textedit::extends` | built/text-panel.md ("Visible characters") | QUEUED | to be specified: the exact clusters now handled are the ZWJ, variation selectors, combining marks and skin tones |
| CDXML: a committed test that round-trips text-run colours TOGETHER with atom and bond colours (one colour table: white, black, then atom and bond colours, then run colours) | built/text-runs.md; built/colour-selection.md; `crates/mzdraw/tests/cdxml.rs` | QUEUED | test-only |
| Flaky timing tests: `grouping_and_diffing_scale_with_the_operations_and_the_ids` (83 ms vs 720 ms) and `the_cost_per_arrow_does_not_grow_with_the_atoms` (steplabel) failed once on CI and passed on rerun | `grouping_and_diffing_scale_with_the_operations_and_the_ids` (in the repository: `crates/mzdraw/tests/proposal.rs`): CI run 44904399210, job 136846775258 of PR #98 (reported as `crates/zzz/tests/proposal.rs:340`), 83.4 ms for 20000 vs 720.6 ms for 80000, passed on one rerun. `the_cost_per_arrow_does_not_grow_with_the_atoms` (`crates/mzdraw/tests/steplabel.rs`): reported by the PR authors as failing once under load on PR #96 and PR #99, passed on rerun | QUEUED | count work units or best of N WITHOUT loosening the ratio; prove with a planted slow mutation (rule 5) |
| `mzlab-cli` calls `replay_with(..., Some(measure))` through one shared function (COO 16 agreement); the mzLab pin move comes last | COO 16 agreement; CLAUDE.md rule 9 | GATED | needs mzLab; `mzlab mol descriptors --json` (`descriptors/1`) is COO 16's small CLI task, not ours |
| e2e flaky: "atom map by touch" timed out once (waiting for the document state after a tap), passed on rerun | CI run 44912250675, job 136870404702 (the `rename` job of PR #101); `tests/e2e.mjs` | QUEUED | find the timing cause; never loosen the test (rule 5); same family as the step-field touch row |
