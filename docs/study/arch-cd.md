# arch-cd: what the desktop editors have, and what mzDraw must have (study)

A study: no code depends on this file. It lists, feature by feature, what the desktop structure editors ChemDraw, BIOVIA Draw, MarvinSketch (with Marvin JS and the new web Marvin) and ChemDoodle (2D desktop and Web Components) offer, as their public documentation describes it. For each feature it gives a priority for mzDraw, the scriptable twin it would have (an op of `draw_edit/1`, a setting or a CLI flag) and a guess of the Rust work. The owner's goal: mzDraw becomes a standalone molecule editor (browser, embedded in the mzLab page, desktop app for macOS and Windows) that is better than Ketcher. The wasm size limit is lifted for now, by the owner's decision (COO-MZDRAW, 2026-11-20).

What is already built is in `docs/mzdraw.md` ("Built state"). Rows that are built say so and get no prompt. One ready prompt per proposed DEV slice is in `docs/prompts/arch-cd-NN-slug.md` (list in §6).

## Rules of this study
- Public documentation only, read on 2026-11-20. A page behind a login, a licence or a lead form is "not read" and nothing is guessed from it.
- Everything is summarised in our own words. No text, artwork, icon, template or style sheet of any product is copied. Published journal numbers (ACS Document 1996) are journal guidance, not a vendor resource.
- `*` = not read at a primary source: taken from a search-result snippet, a third-party copy of an old manual, or a page that refused the fetch. Check before quoting it.
- Every fact carries a source key; §1 maps each key to its URL, version and date.
- No claim of fidelity to any product is made here or implied by a row.

Priority: **MUST** = the editor is not a credible ChemDraw/Ketcher replacement without it; **SHOULD** = expected by many users, competitive; **LATER** = valuable, after the MUST/SHOULD set; **OUT** = outside the scope of `CLAUDE.md` / `docs/mzdraw.md`, or against a rule (offline, no copied artwork). "built" = already in `main` (Built state of `docs/mzdraw.md`).
Size: S = one small PR (about half a day of an agent), M = 2-3, L = 4 or more. Twin: the `draw_edit/1` operation (`{"op": ...}`), the setting (`<section>.<key>`, the shape of `Editor::set` and `mzlab-cli mol draw --set`), or the CLI flag that does the same thing without the page (CLAUDE.md rule 10).
Who: **CD** ChemDraw, **BD** BIOVIA Draw, **MS** MarvinSketch, **MJ** Marvin JS, **M** the new web Marvin, **CDD** ChemDoodle 2D desktop, **CDW** ChemDoodle Web Components.

## 1. Sources, versions, dates
Latest versions found: ChemDraw desktop **26.1** (2026-09-28*), BIOVIA Draw **2026** (released 2025-11), MarvinSketch / Marvin desktop suite **26.1.5** (2026-11-19), Marvin JS **25.3.0** (2025-09-12; final major version, last sale 2026-12-31 and end of support 2027-12-31*), ChemDoodle 2D **12.12.1** (2026-09-19), ChemDoodle Web Components **11.0.0** (2025-05-17).

| key | URL | version / date | read |
|---|---|---|---|
| CD-26.1 | https://support.revvitysignals.com/hc/en-us/articles/54009539308052-Release-Announcement-ChemDraw-Desktop-26-1 | 26.1, 2026-09-28 | no: 403 (Cloudflare); snippet only * |
| CD-26.0 | https://support.revvitysignals.com/hc/en-us/articles/49632553670676-Release-Announcement-ChemDraw-26-0 | 26.0, about 2026-05 | snippet only * |
| CD-25.5 | https://support.revvitysignals.com/hc/en-us/articles/43934842634004-Release-Announcement-Signals-ChemDraw-25-5 | 25.5, page 2025-12-02 | snippet only * |
| CD-25.0 | https://support.revvitysignals.com/hc/en-us/articles/38808816928788-Release-Announcement-ChemDraw-25-0 | 25.0, 2025-07-01 | snippet only * |
| CD-JS25 | https://support.revvitysignals.com/hc/en-us/articles/38809304211220-Release-Announcement-ChemDraw-JS-v25-0-1 | ChemDraw JS 25.0.1 | snippet only * |
| CD-KB/n | https://support.revvitysignals.com/hc/en-us/articles/n (knowledge-base article number n) | undated | snippet only * |
| CD-DC26 | https://assets.revvitysignals.com/m/3b72f4344076941f/original/Full-Dot-Chart-ChemDraw-Suite-of-Products.pdf | feature chart, May 2026 (PDF 2026-05-18) | yes |
| CD-WR25 | https://assets.revvitysignals.com/m/6e4aae70c40d28af/original/Which-ChemDraw-is-right-for-me.pdf | June 2025 | yes |
| CD-HW26 | https://assets.revvitysignals.com/m/7162ca63191b56d0/original/ChemDraw-Desktop-Requirements-Hardware-Software-Guide.pdf | 26.0, 2026-04-23 | yes |
| CD-RN23 | https://www.mn.uio.no/kjemi/tjenester/it/hjelp2/chemdraw/chemdraw-suite-v23.0-release-notes.pdf | 23.0 release notes, 2024-01-18 (copy on a university host) | yes |
| CD-RN22 | https://www.mn.uio.no/kjemi/tjenester/it/hjelp2/chemdraw/chemoffice-v22.0-release-notes.pdf | 22.0 release notes, 2022-09 (university copy) | yes |
| CD-UG17 | https://uni-ulm.de/fileadmin/website_uni_ulm/nawi.chemie/Downloads/ChemOffice/ChemDraw.pdf | 17.1 user guide, 2018-01 (university copy; the newest full user guide on a public host; 26.x may differ) | yes, by section; facts marked `(17.1)` |
| CD-API | https://github.com/Revvity/ChemDraw-AddIns (Documentation/ChemDraw JavaScript API Reference Guide.pdf) | API guide 1.6, 2024-02-14; repo master 119dcb0 | yes |
| CD-JSX | https://github.com/Revvity/ChemDraw-JS-Public-Examples | c96cfbb, 2025-06-27 | yes |
| CD-JSF | https://revvitysignals.com/sites/default/files/2024-05/rs-flyer-chemdraw-javascript-sketcher.pdf | ChemDraw JS flyer, 2024-05 | snippet only * (host 403) |
| CD-WN | https://revvitysignals.com/signals-chemdraw-whats-new, https://revvitysignals.com/products/research/chemdraw | 2025-2026 | no: 403 (Akamai) |
| CD-UG26 | the ChemDraw 26 user guide (Download Center or in-app help) | 26.x | no: login |
| BD-PP | https://www.3ds.com/products/biovia/draw | page links "What's New 2026" | yes |
| BD-WN26 | https://www.3ds.com/assets/invest/2026-03/biovia-draw-what-s-new-2026.png | 2026 | yes (image) |
| BD-DS24 | https://www.3ds.com/assets/invest/2024-02/biovia-draw-ds.pdf | datasheet DS-5005-0224, 2024-02-15 | yes |
| BD-DS20 | https://www.3ds.com/fileadmin/PRODUCTS-SERVICES/BIOVIA/PDF/biovia-draw-ds.pdf | datasheet, 2020-06-30 | yes |
| BD-T49 | https://www.3ds.com/support/documentation/t49-2025-notification-regarding-biovia-draw-2026 | Draw 2026, 2025-11 | yes |
| BD-T30 | https://www.3ds.com/support/documentation/t30-2025-notification-regarding-biovia-draw-2025-sp1 | Draw 2025 SP1, 2025-06 | yes |
| BD-T104 | https://www.3ds.com/support/documentation/resource-library/t104-2024-notification-regarding-biovia-draw-2025 | Draw 2025, 2024-11 | yes |
| BD-T65 | https://www.3ds.com/support/news/single/t65-2023-notification-regarding-biovia-draw-2024/ | Draw 2024, 2023-12-12 | yes |
| BD-AC | https://discover.3ds.com/biovia-draw-academic | undated | yes |
| BD-CT26 | https://www.3ds.com/assets/invest/2026-01/biovia_chemistry_2026_ctfile_formats.pdf | BIOVIA Chemistry 2026 CTfile Formats, 2025-11-11 | yes (editor-relevant sections) |
| BD-PS | https://www.3ds.com/products/biovia/pipette-sketcher, https://www.3ds.com/assets/invest/2023-10/biovia-pipette-sketcher.pdf | datasheet DS-9921-0122, 2022-01-28 | yes |
| BD-UG | BIOVIA Draw user guide (documentation zip on software.3ds.com) | 2026 | no: login |
| MS/page | https://docs.chemaxon.com/latest/page (e.g. `marvin_draw-bonds.html`) | footer 2026-11-02; suite 26.1.5 of 2026-11-19 (`marvin_history-of-changes.html`) | yes |
| MS-IOD/page | https://docs.chemaxon.com/lts-iodine/page (`customizing-marvinsketch-gui.html`, `configurations-of-marvinsketch.html`, `toolbars-of-marvinsketch.html`, `where-are-the-settings-stored.html`) | LTS Iodine, 2022-12-21 | yes; `clean-2d.html` and `elemental-analysis-plugin.html` snippet only * |
| MJ/page | https://docs.chemaxon.com/latest/marvin-js_*.html (toolbars, keyboard-shortcuts, bond, abbreviations, stereochemistry, reaction-mechanism, how-to-embed, web-services, marvin-js-and-marvin-sketch, history-of-changes) | footer 2026-11-02; Marvin JS 25.3.0, 2025-09-12 | yes |
| M/page | https://docs.chemaxon.com/latest/marvin_shortcuts-and-tricks.html, marvin_sketch-js-comparison.html, marvin_installation-guide.html | 2026-11-02 | yes; the full documentation of the new Marvin needs a licence: not read |
| M-PP | https://chemaxon.com/marvin, https://chemaxon.com/blog/marvin-js-end-of-life-and-transition-notice | undated (redirect to certara.com) | snippet only * |
| CDD-G | https://www.chemdoodle.com/downloads/ChemDoodleUserGuide.pdf | ChemDoodle 12.12 user guide, compiled 2026-09-16 | yes, by section (§ numbers given) |
| CDD-K | https://www.chemdoodle.com/downloads/KeyboardShortcuts.pdf | shortcut sheet, 2014-08-19, no version | yes |
| CDD-VH | https://www.ichemlabs.com/support/version-history.php?product=ChemDoodle | 12.12.1 of 2026-09-19, 12.12.0 of 2026-09-15, 12.11.0 of 2026-06-22, 12.10.0 of 2026-02-03 | yes |
| CDD-HP | https://www.chemdoodle.com/ | 12.12.1 | yes |
| MS/pop-up | https://docs.chemaxon.com/latest/marvin_pop-up-menus-of-marvinsketch.html | MarvinSketch context menus, page 2026-11-02 | yes |
| MJ/ctx | https://docs.chemaxon.com/latest/marvin-js_context-menus.html, https://docs.chemaxon.com/latest/marvin-js_selection-context-menu.html | Marvin JS context menus, 2026-11-02 | yes |
| M-253 | https://www.certara.com/wp-content/uploads/2026/03/marvin_25.3.3.pdf | new web Marvin user documentation 25.3.3, 2025-09-01 | yes |
| KE-SRC | https://github.com/epam/ketcher (`packages/ketcher-react/src/script/ui/views/components/ContextMenu/`, `documentation/help.md`), releases and issues | 3.23.0-dev.1, commit 7f69095, 2026-11-20 (Apache-2.0; read, nothing copied) | yes |
| CDW/page | https://web.chemdoodle.com/ (tutorial/2d-structure-canvases/sketcher-canvas, demos/2d-sketcher, installation/changelog, docs/2d-shapes, tutorial/advanced/styles, docs/ichemlabs-cloud-services, tutorial/advanced/sketcher-application-window) | 11.0.0, 2025-05-17 | yes (documentation only; the GPL code was not opened) |

Hosts that refused the fetch (site-side bot walls, not bypassed): revvitysignals.com, www.revvitysignals.com, support.revvitysignals.com, www.revvity.com, pubs.acs.org, the ACS LabNotes PDF on www.acs.org. chemmazing.revvitysignals.com did not connect. Behind a login or licence: the ChemDraw 26 and BIOVIA Draw user guides, the new Marvin full documentation, the ChemDraw tips e-book (lead form).

## 2. What the four products are, in one paragraph each
- **ChemDraw** (Revvity Signals; tiers Prime < Professional < Signals ChemDraw, CD-DC26, CD-WR25): Windows 10/11 and macOS 15/26; on the Mac only ChemDraw itself runs (CD-HW26). Fastest-drawing reputation through hotkeys on the hovered atom or bond, nicknames, Clean Up, live chemical warnings, Analysis window, style sheets with ACS Document 1996; Professional adds name↔structure, NMR prediction, enhanced stereo, R-group tables (CD-DC26). Add-ins in JavaScript (WebView2) see the document and the selection as whole formats, with no per-atom object model and no undo (CD-API). ChemDraw JS is a licensed npm sketcher with an optional Docker service (CD-JSX).
- **BIOVIA Draw** 2026 (Windows only; free for academic use after registration, commercial otherwise; BD-DS24, BD-AC, BD-T49): the CTfile owner's editor. Strong on stereo (CIP labels incl. meso and pseudo, enhanced stereo on centres, double bonds, sulfoxides and axes), Markush, haptic bonds, polymers, mixtures and formulations, and biologics (HELM, BILN, SCSR, sequences); live status-bar validation; resolver lookups; add-in API, XML-configured UI, DrawBridge for web apps (BD-DS24, BD-T65, BD-T104, BD-T30, BD-T49).
- **MarvinSketch** 26.1.5 (Java 21 desktop since 26.1.0; MS/marvin_history-of-changes.html), **Marvin JS** 25.3.0 (iframe editor that needs JChem Web Services for nearly every format, clean, CIP; final major version*) and the **new web Marvin** (React, licensed; universal search bar*, results on the canvas, journal presets, WCAG 2.2 A/AA partial conformance*; M/marvin_sketch-js-comparison.html, M-PP). Deep configuration model (toolbars, keymaps, "configurations" as profiles; MS-IOD).
- **ChemDoodle 2D** 12.12.1 (Windows, macOS, Linux; CDD-HP): the broadest drawing toolset of the four (about 23 bond kinds, orbitals, glassware, TLC, tables, electron pushers with automatic layout, 12 journal style sheets, rebindable shortcuts, workspaces, a History widget, text descriptions of figures since 12.11.0; CDD-G, CDD-VH). **ChemDoodle Web** 11.0.0 is GPL or commercial; clean, naming and many formats need the iChemLabs cloud (CDW/docs/ichemlabs-cloud-services).

## 3. Feature inventory
Rows grouped by area; the ID is used by the prompts (§6). Source keys refer to §1.

### A. Drawing
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| A01 | bond tool: drag from an atom, fixed length, angle snapped | all (CD-UG17, MS/marvin_draw-bonds.html, CDD-G §2.3.1, BD-DS24) | press on an atom, drag, release; length fixed, angle in steps | MUST · built | `bond.add` | - |
| A02 | click a bond to raise its order 1→2→3→1 | CD-UG17, MS/marvin_draw-bonds.html, CDD-G §2.2.4.3 | the single-bond tool on a bond cycles the order | MUST · built | `bond.set o` | - |
| A03 | double bond placed centred, left or right of the line | CD-UG17 (keys l/r/c), MS/marvin_draw-bonds.html (cycles alignment), MJ/marvin-js_bond.html (middle/left/right/auto) | a per-bond position; auto puts ring doubles inside the ring | MUST: journal figures put ring doubles inside; auto is not enough for chains | `bond.set pos` (`auto`/`c`/`l`/`r`) | S (built: `docs/built/bond-display.md`) |
| A04 | bold, dashed, bold-dashed and plain hashed bonds (display, not stereo) | CD-UG17, MS/marvin_draw-bonds.html (bold style), CDD-G §2.2.4.3 | display kinds of a bond; chemistry unchanged | SHOULD: perspective and partial bonds in figures | `bond.set k` (`dash`/`bdash`/`hash` built in `docs/built/bond-display.md`; `bold` is arch-web-20) | S (partly built) |
| A05 | dative / coordinate bond, drawn as an arrow | CD-25.0* (one type with coordination), MS/marvin_draw-coordination-compounds.html, MJ/marvin-js_bond.html, CDD-G §2.2.4.3, CDW/changelog 9.2.0 | its own bond type; V3000 molfile type 9 | SHOULD: complexes, ylides, N→O written as arrows | `bond.add o:9` | S |
| A06 | hydrogen bond (dashed, non-covalent) | CD-RN22 (tool since 22.0), MS/formats CXSMILES `H:` field only, CDD-G (not a bond kind) | a bond that adds no valence; V3000 type 10 | SHOULD: mechanisms, base pairs, supramolecular sketches | `bond.add o:10` | S |
| A07 | quadruple and higher orders, zero-order and half bonds | CD-UG17 (quadruple), CDD-G §2.2.4.3 (to sextuple, half, zero-order) | extra orders | LATER: metal-metal bonds; outside organic scope today | `bond.add o` | S |
| A08 | "either" double bond (crossed or wavy at the end: unknown E/Z) | CD-KB/4408233624724*, MS/background_stereochemistry.html, CDD-G §14.6.3 (crossed double warned) | a double with unknown geometry | SHOULD: unknown E/Z is real chemistry (molfile stereo 3 on a double) | `bond.set s:3` on a double | S |
| A09 | aromatic bond display, ring circle | MS/marvin_draw-bonds.html (aromatic type), CDD-G §6.4 (aromatic circle) | Kekulé or a circle in the ring | SHOULD: settings study §4 already names `aromatic kekule\|circle` | setting `style.aromatic` | M |
| A10 | rings 3-8, fused on a bond or on an atom | all | ring tool on canvas, atom or bond | MUST · built | `ring` | - |
| A11 | ring of any size, macrocycles | CDD-G §2.2.4.4, CDW/demos/2d-sketcher | ring size typed or dragged | SHOULD: macrocycles; `ring` stops at 8 | `ring n` up to 30 | S |
| A12 | chair and conformer templates | CD-UG17 (keys 9/0), CDD-G §2.2.4.4 (two chairs), MS/marvin_templates-in-marvinsketch.html | ready conformers | SHOULD: sugars and cyclohexane stereo teaching | template entries (A14) | S |
| A13 | chain tool: drag sets the number of carbons, the count is shown | CD (chain tool in the ChemDraw JS flyer, CD-JSF*), MS/marvin_draw-chains.html, CDD-G §2.2.4.5 | zigzag follows the pointer; count shown live | MUST: in the scope of `docs/mzdraw.md` and still to build | `atom.add`/`bond.add` batch (one undo step); key `X` | S |
| A14 | template library in groups (rings, bicyclics, heterocycles, amino acids, sugars, protecting groups) | CD-DC26, BD-DS24, BD-T104 (161 named-reaction templates), MS/marvin_templates-in-marvinsketch.html, CDD-G §4.12 | a palette of fragments with an attachment point | MUST: speed of drawing; every product has one | settings `templates`; command `template:<name>` | M |
| A15 | user templates: save a selection, rename, remove, share a file | CD-KB/4408239673748*, MS/marvin_templates-in-marvinsketch.html (My Templates), CDD-G §4.12, BD-DS24 (drag onto the toolbar), BD-T65 (central libraries per user or all users) | the lab's own scaffolds | MUST: the lab's scaffolds; all four have it | settings `templates` entries; `--set templates=FILE` | M |
| A16 | directed merge of a fragment on chosen atom pairs | MS/marvin_merge-structures.html | pairs of atoms define the overlay | LATER: niche | - | M |
| A17 | auto-merge of overlapping atoms when a ring or template lands, a key to suppress it | CDD-G §2.3.1 (Space suppresses) | atoms within a radius become one | SHOULD: fusing by contact; part of A14 | in the template op batch | S |
| A18 | click-move-click drawing mode | CDD-G §2.3.1.1 ("Control" mode), CDD-G §2.4.5 | no drag needed | SHOULD: users with motor limits, trackpads | setting `editor.click_mode` | S |
| A19 | fixed length and fixed angle toggles, snap grid | CD-UG17 (Ctrl+L, Ctrl+E), CDD-G §2.3.1.1 (5 or 10 px grid) | free drawing when off | SHOULD: free figures | settings `editor.fixed_length`, `editor.snap_angle`, `editor.grid` | S |
| A20 | undo/redo without a fixed depth | CD-UG17 (memory-bound, since last save), MS/marvin_marvinsketch_edit-menu.html, CDD-G | | MUST · built | log | - |
| A21 | history panel: every step listed, jump back to any | CDD-G §3.7.2 (History widget) | the action log as a list | SHOULD · built: the command log exists; one view more | command `undo:<n>`; the log is the CLI twin | S |
| A22 | label entry with Alt+click; double-click adds a bond | CD-KB/44882776753300* (25.5) | | LATER: covered by the label field and keyboard focus | - | - |

### B. Atoms, labels, abbreviations, hotkeys
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| B01 | element by key over the hovered or focused atom | CD-UG17, CDD-K, MS/marvin_shortcuts-in-marvinsketch.html, BD-DS24 | type `N` over an atom | MUST · built | `atom.set el` | - |
| B02 | hotkeys over the hovered bond: order, wedge kinds, dashed, bold, turn the wedge | CD-UG17 (1/2/3, b, w, h, y, d), CDD-K (1-6, 7 wedge, 8 hash, 9 ambiguous, F flips), MJ/marvin-js_keyboard-shortcuts.html (digits) | a key acts on the bond under the pointer | MUST: the fastest edit there is; today digits pick the tool or act on the walked bond only | `bond.set` | S |
| B03 | group hotkeys over an atom: Me, Et, CF3, OH, chains of n, rings of n, fuse rings | CD-UG17 (m, e, Shift+f), CD-KB* (v, u: cyclopropyl, cyclobutyl; k: sulfonamide), CDD-K (digit = chain, Shift+digit = ring), M/marvin_shortcuts-and-tricks.html (atom-context ring and acyl keys) | a key sprouts a group at the atom | MUST: listed as "magic hotkeys" in every ChemDraw tier (CD-DC26); CDD and M have the same idea | existing ops in one step; key table in `shortcuts` | S |
| B04 | typed label parsed chemically (OMe, CO2H, NHBoc, CH2OH, counts and charges) | CD-UG17 (nicknames), CDD-G §14.4 ("Interpret Chemically"), MS/marvin_shortcuts-in-marvinsketch.html (type, Enter completes) | a free label becomes atoms | MUST: the 30-entry table is built; composed labels (CO2Me, NHBoc) are not parsed | `atom.set lab` | M |
| B05 | nickname list with pictures | CD-KB/4408239774996* (File > List Nicknames) | browse the abbreviations | SHOULD: discoverability | from the label table | S |
| B06 | user abbreviations (from a selection or from a SMILES with its attachment atom) | CD-UG17 (Define Nickname), MS/marvin_abbreviated-groups-superatom-group.html (`user.abbrevgroup`), CDD-G §14.4.1.1 (label, SMILES, valency 0/1/2), BD-DS24 | the user's groups behave like the built-in ones | MUST: protecting groups of the lab; all four | settings `abbreviations` (`LABEL SMILES` lines, `docs/mzdraw-formats.md` §4); `--set abbreviations=FILE` | M |
| B07 | expand / contract an abbreviation in place | CD-DC26, MS/marvin_marvinsketch_structure-menu.html, MJ/marvin-js_abbreviations.html (editing a contracted group expands it), CDD-G §14.4.2 (auto-collapse), BD-CT26 (state stored in the file) | the group turns into its atoms and back | MUST: edit inside a group, fold it back | ops `label.expand {id}`, `label.contract {atoms, lab}` | M |
| B08 | label reversed by side (OTBS / TBSO, CO2H / HO2C) | CD-KB/4408239978004*, MS/marvin_abbreviated-groups-superatom-group.html (orientation adapts) | text order follows the bond side | SHOULD · built (#43, arch-std-08) | scene only | S |
| B09 | charge tool and `+` `-` keys | all | | MUST · built | `atom.set chg` | - |
| B10 | radical and isotope editing (keys, a field) | MS/marvin_atom-menu.html, CDD-G §2.2.4.7 and §4.14, CDW/changelog 9.2.0 (isotope popover) | `atom.set` has `iso` and `rad`, the page has no way to set them | MUST: the plan's keys `.` and `Shift+I` are not built | `atom.set iso`, `atom.set rad` | S |
| B11 | lone pairs (dots or bar), placed automatically | CD-KB/4408233152148*, MS and MJ (calculated lone pairs), CDD-G §2.2.2 (Place Lone Pairs) | electron pairs drawn at free sides | SHOULD: mechanisms; needed by curved arrows (G05) | `atom.set lp` | S |
| B12 | other atom attributes: δ+/δ−, asterisk, numbered tags | CDD-G §2.2.4.7 | small marks attached to an atom | LATER | - | S |
| B13 | atom properties popover (element, charge, isotope, radical, H count, label) | MS/marvin_atom-menu.html, BD-DS24 (right-click properties), MJ/marvin-js_bond.html (bond dialog) | one place for every field | MUST: `docs/mzdraw.md` rule 4 (no modal dialog, a popover usable from the keyboard) | `atom.set` | M |
| B14 | alias: free text that stands for an atom | MS/marvin_draw-atoms.html (alias, calculations use the atom), CDD-G §14.6.3 (uninterpretable label warned) | text shown, chemistry kept | SHOULD: "free-text labels" are on the still-to-build list | `atom.set alias`; molfile `A` lines | M |
| B15 | atom numbers and map numbers shown | CD-26.0*, BD-T30, MS/marvin_marvinsketch_view-menu.html (index or IUPAC numbering) | numbers next to atoms | SHOULD: reports, NMR assignment | setting `display.atom_numbers` | S |
| B16 | hydrogen display (all / hetero and terminal / hetero / off), carbon labels | MS/marvin_display-options-for-implicit-and-explicit-hydrogens.html, MJ/marvin-js_marvin-js-and-marvin-sketch.html | display modes | SHOULD: settings study §4 `labels.hydrogens`, `labels.carbon` | settings `style.hydrogens`, `style.carbon` | M |
| B17 | add or remove explicit hydrogens on a selection | MS/marvin_display-options-for-implicit-and-explicit-hydrogens.html | H atoms made or removed | SHOULD | `atom.add`/`bond.add`/`remove` batch | S |
| B18 | periodic table picker | MS/marvin_shortcuts-in-marvinsketch.html (Ctrl+E), CD-DC26, CDW/demos/2d-sketcher | any element in two clicks | MUST: Se, B, metals have no key | command `atom:<el>` (exists) | S |
| B19 | generic atoms Q, X, A, M, atom lists | MS/marvin_atom-menu.html, CD-DC26 (atom lists), CDD-G §7.3 | query atoms | OUT: query features (`CLAUDE.md` rule 13); listed on import | - | - |

### C. Stereochemistry
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| C01 | wedge, hash, wavy bonds | all | | MUST · built | `bond.set s` | - |
| C02 | CIP labels on the canvas (R/S, r/s, E/Z, M/P), updated while drawing | CD-UG17 (Show Stereochemistry), MS/marvin_stereochemistry-in-marvinsketch.html (View > Stereo), BD-DS24 and BD-T104 (meso, pseudo, unknown; shown on request), CDD-G §16.3.1 (assign as labels), MJ/marvin-js_stereochemistry.html (web service) | labels next to centres and double bonds | MUST: the four desktop editors have it; `cip/1` exists in `mzlab-feat` | setting `display.cip`; request field `cip` | M |
| C03 | stereo warnings: wedge on a non-centre, missing wedge, wavy between centres | CD-DC26 (new in 2026), CD-UG17, CDD-G §14.6.3 (off by default), MS/structure-checker_checker-list.html | live marks like valence | SHOULD: `check/1` covers valence and charge today | `check/1` (mzLab) | M |
| C04 | enhanced stereo ABS/AND/OR: assign groups, show `&1`/`or1`, V3000 in and out | CD-UG17, CD-DC26 (Professional), MS/marvin_stereochemistry-in-marvinsketch.html (typed `abs`/`or1`/`and1`), BD-DS24, BD-T65 (double bonds), BD-T104 (sulfoxides), CDD-G §16.5, CDW/changelog 9.3.0 | each stereo centre belongs to one group | SHOULD (MUST for medicinal chemists): today the groups are listed as lost on import | op `stereo.group {atoms, k, n}` | M |
| C05 | chiral flag / "absolute" mark | MS/marvin_marvinsketch_structure-menu.html, BD-CT26, CD-DC26 (option to ignore the flag) | one flag per structure | SHOULD: molfile chiral flag round trip | op `mol.flag` | S |
| C06 | axial stereo (atropisomers, allenes) labelled M/P | CD-25.0*, MS/marvin_stereochemistry-in-marvinsketch.html, BD-T30, BD-T49, CDD-G §16 | | LATER: follows `cip/1` coverage in mzLab | - | M |
| C07 | flip that keeps the configuration (rotate 180° instead of mirroring) | CD-KB/4408233459348* | Flip mirrors, Rotate 180° keeps | SHOULD: on the still-to-build list | `rotate deg:180` | S |
| C08 | wedge convention modes (IUPAC vs BIOVIA) | BD-T104, CDD-G §16 | how ambiguous wedges are read | LATER | - | S |
| C09 | Fischer, Haworth, Newman, chair projection tools | CDD-G §6.5 and §2.2.4.5 | projection drawing helpers | LATER: teaching | - | M |

### D. Layout and clean-up
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| D01 | Clean Up Structure (selection or all), stereo meaning kept | CD-UG17 (Shift+Ctrl+K; bonds to the fixed length, 15° multiples, rings only if fully selected), MS/marvin_cleaning.html (Ctrl+2), MS-IOD/clean-2d.html* (partial clean keeps coordinates), CDD-G §14.7.1.1 (stochastic; keeps stereo), BD-DS20 | new coordinates from the 2D layout | MUST: `layout::clean_up` still returns nothing; `Mol::layout_2d_fixed` exists | op `atoms.xy` (arch-std-15, PR #45); command `clean` | M |
| D02 | Clean Up Reaction: spacing, plus signs, arrow length, wrap | CD-UG17, CDD-G §2.2.2.7 and §8.6, MS/marvin_creating-reactions.html (automatic plus) | arranges a reaction along its arrow | SHOULD: scheme figures | `atoms.xy` + `move` + `obj.add plus`; command `clean-reaction` | M |
| D03 | clean with ring templates | MS/marvin_cleaning.html (Clean and Arrange), MS-IOD/clean-2d.html* | templates guide the layout | LATER: layout quality is mzLab's (D1) | - | L |
| D04 | deterministic clean | CDD-G §14.7.1.1 (stochastic: repeated runs differ) | | MUST (ours): parity needs the same bytes; part of D01 | - | - |
| D05 | centre the drawing, zoom to fit | BD-T65 (centre button) | | MUST · built (`fit`) | - | - |
| D06 | 3D clean and 3D view | CD-UG17 (Ctrl+Shift+D), MS/marvin_cleaning.html, CDD-G §6.2 | | OUT: a 2D editor (`docs/mzdraw.md` scope) | - | - |

### E. Checks and warnings
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| E01 | live chemical warnings, never blocking | CD-UG17 (red box, screen only), BD-T65 (status bar), MS/marvin_error-highlighting-in-marvinsketch.html, CDD-G §14.6 | marks while drawing | MUST · built (`check/1`) | `mzlab-cli mol check` | - |
| E02 | explain a warning; ignore it per object or per document | CD-UG17 ("Explain This Warning"), CDD-G §14.6 (off per type or object) | a sentence per mark; an ignore that is saved | SHOULD: the mark says what is wrong, the user decides | `atom.set ignore`; setting `display.warnings` | S |
| E03 | geometry warnings: close atoms, atom on a bond, isolated bond, linear centre, unbalanced label parentheses | CD-UG17, CDD-G §14.6.3, MS/structure-checker_checker-list.html | drawing mistakes, not valence | SHOULD: geometry is mzDraw's own (Rust, no mzLab call) | in `state` | S |
| E04 | Check Structure walk-through (next, ignore, stop) | CD-UG17, MS/marvin_structure-checker-in-marvinsketch.html (Ctrl+R) | step through the problems | LATER | - | S |
| E05 | automatic fixers, standardizer | MS/structure-checker_checker-list.html | fix on request | LATER | - | M |

### F. Calculations, names, lookups
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| F01 | Analysis panel: formula, MW, exact mass, m/z, elemental analysis of the selection or of all | CD-DC26 (all tiers), CD-KB/4408239739668*, MS/marvin_analysis-box.html (Ctrl+I), CDD-G §3.7, BD-DS24 | a panel that follows the selection | MUST: the first question of a mass spectrometrist; engines exist in mzLab (`selection/1`, `label/1`) | read-only; `mzlab-cli mol` | S |
| F02 | live caption on the canvas that updates with its structure | CD-UG17 (Analysis values pasted as live captions), MS/marvin_analysis-box.html (box updates), M/marvin_sketch-js-comparison.html (results on the canvas), CDD-G §15.4 (auto-updating labels) | a text object bound to a structure | SHOULD: the "labels under structures" of the mzLab Drawing page | `obj.add t:"calc"` with `of` and `fields` | M |
| F03 | m/z of chosen adducts, isotope pattern | CD-DC26 (m/z), MS/marvin_analysis-box.html (m/z distribution), CDD-G §3.7 (isotopic distribution) | | SHOULD: mzLab engines (adducts, isotopes) | read-only | S |
| F04 | mass fragmentation tool (masses of the pieces of a cut bond) | CD-25.5*, CD-DC26 | cut a bond, see both masses | SHOULD: mzLab has `cut/1` and the fragmenter; a strength for this lab | read-only; `mzlab-cli` | M |
| F05 | logP, tPSA, H-bond donors and acceptors | CD-DC26, BD-DS24, MS/marvin_calculations-plugins-in-marvinsketch.html | descriptors | LATER: Crippen logP is in `mzlab-mol` | read-only | S |
| F06 | pKa, solubility, NMR prediction | CD-DC26 (Professional), MS plugins, CDD-G §3.7.6.5 | prediction models | OUT: models outside the editor | - | - |
| F07 | name to structure | CD-DC26 (Professional), MS/name-to-structure_user-guide.html, BD-DS24, CDD-G §2.2.2, CDW (cloud) | a typed name becomes a structure | LATER: no permissive engine in Rust; in mzLab the molecule database answers names | input bar, `mzlab-cli` | L |
| F08 | structure to name (IUPAC) | CD-DC26, MS/structure-to-name_user-guide.html, BD-DS24, CDD-G §15 | | OUT for now: a large rule engine; revisit with the owner | - | L |
| F09 | lookup by name, CAS, identifier (PubChem and others) | BD-DS24 (NCI, PubChem), CDD-HP (MolGrabber), CD-DC26 (ChemACX, SciFinder add-ins) | online search | LATER: network; in mzLab the molecule database, offline first | - | M |
| F10 | stoichiometry table, reaction balancing | CD-DC26 (Professional), CDD-G §8.9-8.10 | | LATER | - | M |
| F11 | InChI / InChIKey, SLN, ROSDAL out | CD-DC26 (InChI copy), BD-DS24, CDD-G §2.2.2 | line notations | LATER: InChI is decision 5 of `docs/mzdraw-plan.md` | `save` kind | M |

### G. Reactions and mechanisms
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| G01 | reaction arrow with text above | all | | MUST · built | `obj.add t:"arrow" text` | - |
| G02 | conditions below the arrow, several lines | CD-RN23 (reagents above or below), CDD-G §8.5 (rich text above and below), MS/marvin_creating-reactions.html (agents move with the arrow), CDW/docs/2d-shapes (top and bottom text) | text under the arrow | MUST: solvent, temperature, time are written there | `obj.set below` | S |
| G03 | arrow kinds: equilibrium, unbalanced equilibrium, retrosynthetic, resonance, failed (crossed), dashed, hollow | CD-UG17, MS/marvin_creating-reactions.html (chemical meaning for single, retro, equilibrium, resonance), M/marvin_sketch-js-comparison.html (unbalanced equilibrium, rearrangement), CDD-G §8.3.6, CDW/docs/2d-shapes | one arrow object, a kind field | MUST: the vocabulary of schemes; on the still-to-build list | `obj.add k`, `obj.set k` | S |
| G04 | arrow heads: full, half, none, at either end; curved (arc) arrows | CD-UG17, CDD-G §8.3 (start head, connector, end head; arcs 90-270°) | head choice per end | SHOULD | `obj.set h1 h2 arc` | S |
| G05 | electron-pushing curved arrows anchored to atoms, bonds and lone pairs (pair and single electron) | MS/marvin_electron-flow-arrows.html (snapping, counts per source), MJ/marvin-js_reaction-mechanism.html, CDD-G §10.13 (automatic layout, "show result"), CD-KB/4408233606804* | ends attached to things, the curve follows them | SHOULD: mechanisms in teaching; mzLab `mechanism/1` | object `curve` with anchors (`obj.add t:"curve"`) | M |
| G06 | plus signs added by the reaction clean-up | MS/marvin_creating-reactions.html, CD-UG17, CDD-G §8.5 | | SHOULD: part of D02 | - | - |
| G07 | roles of structures from their place along the arrow | CD-RN23, MS/marvin_creating-reactions.html | reactants, agents, products | MUST · built (RXN out) | `save rxn` | - |
| G08 | multi-step schemes (several arrows) saved as chemistry | MS/marvin_creating-reactions.html (several arrows = graphics only), CD | | LATER: RXN holds one step; KET/CDXML carry the picture | - | M |
| G09 | atom-to-atom mapping: manual tool, map numbers shown, RXN maps in and out | CD-DC26 (all tiers), MS/marvin_mapping-reactions.html (manual + automatic modes), CDD-G §8.8, MJ (manual; automatic needs the service) | numbers pair atoms across the arrow | SHOULD: reaction data; automatic mapping LATER | `atom.set map` | M |
| G10 | numbering of compounds in a scheme (1, 2, 3 under structures) | CD-UG17 (autonumbering) | numbered captions that renumber | SHOULD: papers and theses | `obj.add t:"calc" fields:["number"]` | M |
| G11 | reaction by keyboard: copy the selection past a new arrow | CD-UG17 (Ctrl+arrow) | | LATER | - | S |

### H. Graphics, text, tables
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| H01 | one-line text | all | | MUST · built | `obj.add t:"text"` | - |
| H02 | rich text: several lines, sub/superscript runs, bold, italic, "formula" style (digits lowered) | CD-UG17 (formula style), CDD-G §10.11, M/marvin_sketch-js-comparison.html (rich text) | text runs with formats | MUST: captions such as H2O, 25 °C, 10⁻³ M; on the still-to-build list | `obj.add text runs`; `obj.set runs` | M |
| H03 | shapes: line, rectangle, rounded rectangle, ellipse, arc | CD-UG17, MS/marvin_drawing-shapes.html, CDD-G §2.2.4.10, BD-T65 (rectangle) | framing, highlighting | SHOULD | `obj.add t:"shape" k` | M |
| H04 | brackets (round, square, curly) with a charge or count label | CD-UG17, MS/marvin_drawing-shapes.html, CDD-G §2.2.4.11 | graphics only | SHOULD: [ ]⁺, ‡ transition states; polymer meaning stays OUT | `obj.add t:"shape" k:"bracket"` | S |
| H05 | colour of atoms, bonds, objects; highlight | CD-DC26 (highlight, Professional), M/marvin_sketch-js-comparison.html, CDD-G §12.12 | per-object colour | SHOULD: figures; evidence colours of mzLab (`highlight/1`) | op `colour {atoms, bonds, objects, c}` | M |
| H06 | orbitals (s, p, d, hybrids) | CD-DC26, CD-UG17 (Shift+g), CDD-G §2.2.4.8 | | LATER | - | M |
| H07 | TLC plate and gel tools | CD-UG17, CD-DC26, CDD-G §3.7.12 | lanes, spots by Rf | LATER: lab notebooks | - | M |
| H08 | glassware, lab equipment and bio art | CD-DC26, CDD-G §11 | artwork library | OUT: artwork cannot be copied; our own set would be a separate project | - | L |
| H09 | tables | CD-UG17, CDD-G §10.10 | | LATER | - | M |
| H10 | symbol picker (‡, δ, ⊕, arrows) | CDD-G §3.7 (Unicode Symbols widget), CD-UG17 (character map) | | SHOULD: part of H02 | - | S |
| H11 | hand-drawn stroke styles | CDD-HP, CDD-G §13.5 | | OUT | - | - |
| H12 | group objects (move as one) | CD-UG17 (Ctrl+G, integral or not), CDD-G §12.7 | | SHOULD | op `group {atoms, objects}` / `ungroup` | M |
| H13 | z-order, which bond passes over | CD-UG17 (F2/F3), CDD-G §2.2.2 (layering from wedges) | | LATER | - | S |

### I. Document, styles, arranging
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| I01 | style presets, ACS Document 1996 (14.4 pt bond, 0.6 pt line, 2 pt bold, 2.5 pt hash, 1.6 pt margin, 18 % spacing, 10 pt labels, 120° chains) | CD-UG17 (p. 48), MS/marvin_drawing-styles.html, CDD-G §13, CDW/tutorial/advanced/styles | | MUST · built (`mzlab`, `acs-1996`, `slides`) | `style.preset` | - |
| I02 | more journal presets (RSC, Nature, Wiley, Elsevier, Angewandte, Synthesis) | MS/marvin_drawing-styles.html (12 presets), CDD-G §2.5.2 (12 sheets) | | SHOULD: only from numbers read in each journal's guidelines (`docs/mzdraw-formats.md` §4) | `style.preset` | S |
| I03 | style applied to a selection; style sheets merged into a document | CD-UG17, CDD-G §13.2 | | LATER | - | M |
| I04 | dark mode (interface and drawing) | CD-RN23 (dark style sheet), CDD-G §2.3.2 | | SHOULD: preset `dark` is planned | `style.preset=dark` | S |
| I05 | page setup, margins, print, several pages | CD-UG17, MS/marvin_creating-a-multipage-document.html, CDD-G §13.4.2 and §17.12 | a page model | LATER (print: SHOULD in the desktop app) | settings `page` | M |
| I06 | align (left, centre, right, top, middle, bottom), distribute, centre | CD-UG17 (Alt+Shift+Ctrl+letter), M/marvin_sketch-js-comparison.html (guides), CDD-G §12.4 | moves whole structures and objects | MUST: tidy schemes; all have it | `move` ops; commands `align:<edge>`, `distribute:h\|v` | S |
| I07 | rotate by a typed angle; 1° and 15° steps from the keyboard | CD-UG17 (Ctrl+R), M/marvin_shortcuts-and-tricks.html (Alt+arrow 1°, Shift+Alt+arrow 15°), CDD-G §12.4 | | SHOULD: 15° buttons are built | `rotate deg` | S |
| I08 | scale the selection | CD-UG17 (Ctrl+K), MS/marvin_geometric-transformation-of-structures-and-objects.html, CDD-G §12.4 | | SHOULD | `atoms.xy` (D01) | S |
| I09 | nudge the selection with the arrow keys | CDW/demos/2d-sketcher | | SHOULD: arrows walk the focus today | `move` | S |
| I10 | select inverse, reselect, select a whole piece | CDD-G §12.2 | | SHOULD | - | S |
| I11 | lasso selection | CDD-G §4.2.2 (key S), CDW/demos/2d-sketcher | free-form selection | SHOULD: a band exists | - | S |

### J. Files, clipboard
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| J01 | MOL V2000/V3000, SDF, SMILES, RXN, RDF, CML, MRV, KET, CDXML | CD-DC26, BD-DS24, MS/formats_index.html, CDD-G §17.6.5 | | MUST · built (see `docs/mzdraw-formats.md` §1a) | `save`, `add` | - |
| J02 | CDX (binary) read | CD-DC26, BD-DS24 (open only), CDD-G §17.6.5 | | SHOULD: ChemDraw users send CDX | `add` | M |
| J03 | PDF and EPS out, with the chemistry inside | CD-DC26 (EPS), CDD-G §17.9 (PDF, EPS, ODG, SVG carry chemical data) | vector files | SHOULD (MUST in the desktop app for print): decision 4 of `docs/mzdraw-plan.md` | `save pdf` | M |
| J04 | smart paste: lands at the view centre or the nearest free place, never over a drawing | CD-RN23, CD-DC26 | placement of pasted parts | MUST: a paste on top of a drawing is the first annoyance | `move` after the paste ops | S |
| J05 | paste text in any format, sniffed | CD-KB/4408268608660* (17.1), MS/marvin_copy-paste-and-draganddrop.html | | MUST · built (InChI not read) | `add` | - |
| J06 | copy as SMILES, molfile, CDXML, KET, InChI | CD-DC26, MS/marvin_shortcuts-in-marvinsketch.html (Ctrl+L / Ctrl+M), BD-DS24, CDD-G §17.2 | | MUST: copy writes molfile + `mzdraw/1` today; the rest is one command each | commands `copy:<kind>` | S |
| J07 | desktop clipboard in native formats (CDXML for ChemDraw, MDLCT on Windows, EMF/PDF pictures) | CD-DC26 (OLE), CDD-G §17.2 (MIME per kind; MDLCT/MDLSK), MS/marvin_copy-paste-and-draganddrop.html (EMF, OLE) | other programs paste chemistry | SHOULD (desktop app) | - | M |
| J08 | edit again inside Office (OLE, LibreOffice plugin) | CD-DC26, MS, CDD-G §17.7 | | LATER: our SVG/PNG carry their document (built) | - | L |
| J09 | drag and drop of files on the canvas | CD-26.0*, MS/marvin_copy-paste-and-draganddrop.html | | SHOULD | `add` | S |
| J10 | image or PDF to structure (OCR) | MS/marvin_opening-a-molecule-file.html (OSRA), CDD-G §17.11 | | OUT: not planned (`docs/mzdraw-plan.md`) | - | - |

### K. Interface, palettes, shortcuts, accessibility
(Palettes in detail: §4.)
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| K01 | tool groups with sub-palettes; the last tool picked becomes the face of the group | CD-UG17, CDD-G §2.2.4 | press a group, pick a variant | MUST · R1 (PR #35) builds it | `layout` (§4) | M |
| K02 | rebindable shortcuts with conflict report and a printable table | CD-UG17 (Hotkeys.xml per object type), MS-IOD/customizing-marvinsketch-gui.html (keymap schemes, conflicts warned), CDD-G §4.2 (Edit Shortcuts, Summarize to HTML) | key → command, per object type | MUST: settings `shortcuts` planned in `docs/mzdraw-formats.md` §4 | settings `shortcuts`; `--set shortcuts=FILE` | M |
| K03 | context menu on an atom, a bond, the selection, the canvas | BD-DS24, MS/marvin_atom-menu.html, CD-UG17 | the right commands where the pointer is | MUST: right-click and long press; the model is PR #37, refinement in §7 and arch-cd-20 | `context:<x>,<y>` replay output (PR #37) | M |
| K04 | floating context toolbar next to the selection | CD-26.0*, CD-DC26 (web) | | SHOULD | - | S |
| K05 | keyboard navigation of the toolbars (Tab between groups, arrows inside, Enter opens, Escape closes) | M/marvin_shortcuts-and-tricks.html | | MUST: WCAG 2.2; real buttons are built | part of §4 | S |
| K06 | WCAG 2.2 AA conformance statement and screen reader support | M-PP* (partial 2.2 A/AA), CD-26.0* (2.1 AA work) | | MUST: European Accessibility Act; the outline is built, a real screen-reader pass is not | - | M |
| K07 | text description of a figure | CDD-VH (12.11.0) | | MUST · built (outline "What is drawn"); SHOULD: write it into the SVG as `<desc>` | `save svg` | S |
| K08 | interface colours for colour vision deficiency, font size, left-handed mode | CDD-G §2.4 | | SHOULD | settings (e.g. `display.*`) | S |
| K09 | sharp icons at any display scale | complaint CD-KB/4408233178004* | | MUST: SVG icons only | - | S |
| K10 | status line with what a click will do | BD-T65 (status bar) | | SHOULD: the live line exists | - | S |
| K11 | toolbar presets for embedders (one molecule, reaction, teaching) | MJ (presets*), CDW/tutorial/2d-structure-canvases/sketcher-canvas (`oneMolecule`, `includeToolbar`, `useServices`) | the host picks the tools | SHOULD: the mzLab page embeds mzDraw | element attribute and `layout.preset` | S |

### L. Platform, embedding, scripting
| ID | feature | who has it | how it works (short) | priority for mzDraw | scriptable twin | size |
|---|---|---|---|---|---|---|
| L01 | desktop app for macOS and Windows | CD-HW26 (Win, Mac), BD-DS24 (Windows only), MS (Java), CDD-HP (Win, Mac, Linux) | | MUST: the owner's goal | the same Rust core | L |
| L02 | documents in tabs or windows, recent files, file associations, autosave per document | CDD-G (document tabs), CD-UG17 | | MUST (desktop) | - | M |
| L03 | auto-update with an administrator switch | CD-26.1* | | SHOULD (desktop) | - | M |
| L04 | embeddable editor with a small API: load, get SMILES / molfile / SVG, change event, read-only | CD-JSX (`attach`), MJ/marvin-js_how-to-embed.html (iframe, promise), CDW/tutorial/2d-structure-canvases/sketcher-canvas, BD-DS24 (DrawBridge) | the host page talks to the editor | MUST: embedding in the mzLab page; `TPDraw` of `docs/mzdraw.md` | the wasm exports | M |
| L05 | add-in API (document and selection in formats, change events) | CD-API, BD-DS24 | | SHOULD: our route is the CLI plus `draw_edit/1`; a JS API mirrors `Editor` | - | M |
| L06 | Python scripting | CD-DC26 (ChemScript) | | OUT here: the CLI of mzLab is the route | `mzlab-cli mol draw` | - |
| L07 | everything client-side, no server for formats, clean, CIP, names | MJ needs JChem Web Services (MJ/marvin-js_marvin-js-and-marvin-sketch.html), CDW needs the cloud (CDW/docs/ichemlabs-cloud-services) | | MUST · built (the principle) | - | - |
| L08 | cloud sharing and collaboration | CD-DC26 (Signals tier, ChemDraw+) | | OUT: offline first | - | - |

### M. Out of scope, held by competitors
| ID | feature | who has it | priority | reason |
|---|---|---|---|---|
| M01 | biopolymers: HELM, BILN, SCSR, sequences, monomer libraries | CD-DC26, BD-DS24, BD-T49, CDD-G §2.2.4.6 | OUT | macromolecules (`CLAUDE.md` rule 13); files list them on import |
| M02 | polymers, mixtures, formulations (S-groups) | BD-DS24, MS/marvin_draw-polymers.html, CDD-G §10.15, CD-DC26 | OUT | S-groups other than abbreviations |
| M03 | Markush: R-group definitions, R-logic, enumeration, variable attachment | BD-DS24, MS/marvin_r-groups-in-marvinsketch.html, CD-DC26 | OUT (R1/R2 placeholders are built) | query features |
| M04 | query atoms and bonds | all | OUT | query features |
| M05 | haptic / multicentre bonds | BD-DS24, BD-CT26, MS/marvin_draw-coordination-compounds.html, CDD-VH (12.12.0) | LATER | organometallics; a scope question for the owner |
| M06 | spectra (JCAMP-DX), NMR prediction | CDD-G §9, CD-DC26 | OUT | not an editor feature here |

## 4. Palettes: tear-off, docking, layouts (the owner likes ChemDraw's)
### What the products do
- **ChemDraw**: a tool with a small corner triangle opens a sub-palette, and the sub-palette can be dragged off by its title to float anywhere (CD-UG17, 17.1). Toolbars dock on a window edge or float; right-click > Dock Toolbar, or a double-click sends a toolbar back to its last or docked place (CD-UG17). Each toolbar is shown or hidden from the View menu or its close box (CD-UG17). The toolbars are defined in XML files per skin, editable by hand, then a restart (CD-UG17). The 2026 web editor shows context-sensitive floating toolbars (CD-DC26). The layout is stored in the Windows registry (CD-KB/4408232681876*, v23+), and no reset command was found in any page read: the vendor's fix is to delete the registry key.
- **MarvinSketch**: View > Toolbars shows or hides each toolbar; a toolbar is dragged by its separator and a coloured border previews where it lands (MS-IOD/customizing-marvinsketch-gui.html). Customize adds, removes, reorders and renames buttons and makes user toolbars; icon size large or small; text, icons or both. A "configuration" bundles menus, toolbars, pop-ups and shortcuts: activate, copy, rename, reset, import and export (MS-IOD/configurations-of-marvinsketch.html). The file is per user and per version and is carried forward on upgrade (MS-IOD/where-are-the-settings-stored.html). Ctrl+E and Ctrl+T open the periodic table and the templates (MS/marvin_shortcuts-in-marvinsketch.html).
- **Marvin JS / new Marvin**: a fixed layout (tools left, general top, atoms right, templates bottom); small windows get scroll buttons on toolbars (MJ/marvin-js_toolbars.html). The embedder picks a preset; end users cannot customise* (MJ presets). The new Marvin can be driven fully from the keyboard: Tab between toolbars, arrows inside, Enter or Space opens a sub-toolbar, Escape closes (M/marvin_shortcuts-and-tricks.html).
- **BIOVIA Draw**: users drag structures and abbreviations onto the toolbar to reuse them (BD-DS24). An organisation shapes the interface through an XML file, and add-ins appear as tools, buttons or menu items (BD-DS24, BD-PP). Template libraries can be installed for one user or all users (BD-T65). Floating, docking, reset: not described on any public page.
- **ChemDoodle 2D**: the Drawing toolbar on the left groups tools; the variant last chosen becomes the button's tool (CDD-G §2.2.4). The Drawing toolbar can be detached and moved by its drag anchor (CDD-G §2.2.4, Figure 2.10). Window positions, widgets, scale and view options are remembered automatically; Window > Default Workspace resets, Revert Workspace goes back to the last saved one, Save Workspace stores it (CDD-G §2.2.7). Every menu item and tool can be rebound and the table printed (CDD-G §4.2). Ctrl+Alt+W shows or hides the dialogs (CDD-K). **ChemDoodle Web**: `floatDrawTools` turns the drawing tools into a floating palette whose option trays follow it and flip near the edges (CDW/tutorial/advanced/sketcher-application-window, CDW/changelog 8.0.0); `isMobile` changes the layout on phones (CDW/tutorial/2d-structure-canvases/sketcher-canvas).

### What users complain about
- ChemDraw: the tool palette or the main toolbar disappears and the View menu does not bring it back (CD-KB/4408239737876*, CD-KB/4408232681876*); customisations are lost for users without administrator rights, who cannot write the registry (CD-KB/4408233143188*); on the Mac toolbars float on a second display but dock only on the main one (CD-KB/4408239540884*); a window lost off-screen after a monitor change (CD-KB/14100226145172*); tiny icons on 4K screens (CD-KB/4408233178004*); a toolbar missing after an upgrade (CD-KB/4408239538068*, 18.1).
- MarvinSketch: dockable toolbars that would not pin to a new place (fixed 20.14, 2020-06-18), windows opened at the wrong place at high display scaling (fixed 20.15), an error after closing the customisation dialog (fixed 21.3) (MS/marvin_history-of-changes.html).
- ChemDoodle Web: toolbars cut off at the screen edge (fixed 4.5.0), trays opening behind the canvas in Safari (fixed 4.3.1), page CSS stretching the toolbars (fixed 5.2.3) (CDW/installation/changelog).

### What mzDraw should do
The model is the one of `docs/mzdraw-ui.md` §4-§5, built as R1 (PR #35, `crates/mzdraw/src/palettes.rs`): the object `layout` of `mzdraw-settings/1` (preset, snap, and per palette `dock`, `order`, `x`, `y`, `orient`, `collapsed`, `hidden`; only what differs from the preset), decided in Rust, painted by the page, not part of the document and no undo step. The table below says which requirements R1 already meets and what the arch-cd prompts 18-19 add on top of it.
| ID | requirement | taken from | priority | twin |
|---|---|---|---|---|
| P01 | tool groups with sub-palettes; the last variant becomes the face of the group | CD-UG17, CDD-G §2.2.4 | MUST · R1 (flyouts with the kind in use, `docs/mzdraw-ui.md` §4) | `layout` |
| P02 | tear-off: a sub-palette dragged by its title becomes a floating palette above the canvas, with a close box | CD-UG17 | MUST (arch-cd-18) | `layout` entry with `from` |
| P03 | dock and undock toolbars on the four edges; a double-click on a palette's title returns it to its home | CD-UG17, MS-IOD/customizing-marvinsketch-gui.html (landing preview) | MUST · R1 docks and snaps; home on double-click: arch-cd-18 | `layout.palettes[].dock` |
| P04 | show / hide every palette from one menu with check marks, and by a key | CD-UG17, MS-IOD/customizing-marvinsketch-gui.html, CDD-K | MUST · R1 (Palettes menu, `\` hides all); check marks: arch-cd-18 | `layout.palettes[].hidden` |
| P05 | the layout survives a reload and an upgrade, per user, without administrator rights | CDD-G §2.2.7; complaints CD-KB/4408233143188* | MUST · R1 (settings file; unknown keys kept) | `layout` |
| P06 | Reset layout command, always reachable (also when every palette is hidden) | CDD-G §2.2.7 (Default Workspace); complaint CD-KB/4408232681876* | MUST · R1 (Reset layout) | `layout.reset` |
| P07 | a palette never ends outside the window: clamped after a resize, a zoom change or a monitor change | complaints CD-KB/14100226145172*, MS/marvin_history-of-changes.html (20.15), CDW/installation/changelog (4.5.0) | MUST · R1 (clamped on every resize, property test) | - |
| P08 | keyboard: Tab to a palette, arrows inside, Enter opens a group, Escape closes, focus visible | M/marvin_shortcuts-and-tricks.html | MUST · R1 (`docs/mzdraw-ui.md` §4 keyboard and flyouts) | - |
| P09 | named layouts (save, revert, export, import) | CDD-G §2.2.7, MS-IOD/configurations-of-marvinsketch.html | SHOULD (arch-cd-19) | `layout.save`, `layout.use` |
| P10 | customise palettes: add, remove, reorder tools, user palettes, a user-templates palette | MS-IOD/customizing-marvinsketch-gui.html, CD-UG17 (XML), BD-DS24 (drag onto toolbar) | SHOULD (later) | `layout.palettes[].tools` |
| P11 | icon size small / large; sharp at any scale | MS-IOD/customizing-marvinsketch-gui.html; complaint CD-KB/4408233178004* | SHOULD (arch-cd-19) | `layout.icons` |
| P12 | narrow screens and touch: palettes become a bottom bar, no floating windows | CDW/tutorial/2d-structure-canvases/sketcher-canvas (`isMobile`) | SHOULD · R1 (tablet and phone presets) | `layout.preset` |
| P13 | palettes on a second monitor (desktop app) | complaint CD-KB/4408239540884* | LATER | - |
| P14 | interface layout stored inside drawing files | MS/marvin_history-of-changes.html 15.3.2.0 (GUI options in MRV) | OUT: the document holds chemistry and drawing, not the user's interface | - |

## 5. Counts
- 173 rows (A-M, palettes P, context menus X): **MUST 68** (17 built on `main`, 11 more built by the open PRs #35 palettes R1 and #37 context menu, so 40 left), **SHOULD 63**, LATER 27, OUT 15. A row whose priority has a parenthesis ("SHOULD (MUST for medicinal chemists)") is counted by its first word.
- Every MUST row not built maps to a prompt to run (§6: an arch-cd prompt, or the arch-web / arch-std prompt that covers it), except K06; the context-menu entries of a feature (X09, X10) are part of that feature's prompt, and X16 is part of arch-web-01/02; prompts 23-26 take the SHOULD rows with the best value for cost. The rest of the SHOULD rows are left for a later study round.
- Where mzDraw is already ahead of all four (from the rows above): one Rust path for page, CLI and desktop with the same bytes (L07, `CLAUDE.md` rule 10); no server for any feature (L07, against MJ and CDW); pictures that reopen as chemistry (J09 is built for SVG and PNG; CDD does it for PDF/EPS/SVG); an import that lists what it could not read; per-structure identification annotations (no competitor has a slot for them); a text description of the figure for screen readers (only CDD 12.11 has one).

## 6. Proposed DEV slices, and which prompt to run
One prompt per slice in `docs/prompts/arch-cd-NN-slug.md`. The studies arch-std (`docs/study/arch-std.md`, on `main`) and arch-web (branch `claude/arch-web`) propose some of the same slices. Where theirs covers the same scope, **theirs is the one to run**, and the arch-cd file is a short "superseded" note that lists what the desktop editors add (to paste into that prompt or its review). Context menu and palettes are being built (PR #37 context menu model, PR #35 palettes R1): the arch-cd prompts 18-20 refine on top of them and add no second model.
| NN | slug | rows | priority | run |
|---|---|---|---|---|
| 01 | clean-up | D01, D04 | MUST | **arch-std-15** (PR #45; arch-web-10 is withdrawn); arch-cd-01 = additions |
| 02 | cip-labels | C02 | MUST | **arch-web-16**; arch-cd-02 = additions |
| 03 | hover-hotkeys | B02, B03 | MUST | **arch-cd-03** (keys into arch-std-06's command table when merged) |
| 04 | bond-display | A03, A04 | MUST (A03) + SHOULD (A04) | **arch-cd-04** (double-bond position; dashed kinds on the one bond field `k`, COO decision 16) |
| 05 | chain-tool | A13 | MUST | **arch-web-08**; arch-cd-05 = additions |
| 06 | arrow-kinds | G02, G03, G04 | MUST | **arch-web-13**; arch-cd-06 = additions (heads, arc) |
| 07 | rich-text | H02, H10 | MUST | **arch-web-14**; arch-cd-07 = additions (formula mode, symbols) |
| 08 | radical-isotope-table | B10, B18 | MUST | **arch-cd-08** (periodic table and keys; the popover is arch-web-11) |
| 09 | atom-popover | B13 | MUST | **arch-web-11**; arch-cd-09 = additions |
| 10 | abbrev-expand | B07 | MUST | **arch-web-18**; arch-cd-10 = additions |
| 11 | user-abbreviations | B06, B04 | MUST | **arch-web-18**; arch-cd-11 points to arch-cd-10's additions |
| 12 | templates | A14, A17, A12 | MUST | **arch-web-19**; arch-cd-12 = additions |
| 13 | user-templates | A15 | MUST | **arch-web-19**; arch-cd-13 = additions |
| 14 | analysis-panel | F01 | MUST | **arch-web-17**; arch-cd-14 = additions (where the numbers come from, parity) |
| 15 | align-distribute | I06, I07, I08 | MUST + SHOULD | **arch-cd-15** (scaling needs arch-std-15's `atoms.xy`; the 1°/15° keys and nudging are arch-std-05) |
| 16 | smart-paste | J04 | MUST | **arch-cd-16** (Copy as is arch-web-21 and PR #37) |
| 17 | shortcuts | K02 | MUST | **arch-std-06**; arch-cd-17 = additions |
| 18 | palette-tear-off | P02, P03, P04, K01 | MUST | **arch-cd-18**, on top of PR #35 (R1) |
| 19 | named-layouts | P09, P11 | SHOULD | **arch-cd-19**, on top of PR #35 (R1) |
| 20 | context-menu-refine | K03, §7 | MUST | **arch-cd-20**, on top of PR #37 (the model) and its page PR; arch-web-12 is the same first slice as PR #37 |
| 21 | embed-api | L04 | MUST | **arch-web-01, -02, -03**; arch-cd-21 = additions |
| 22 | desktop-shell | L01, L02 | MUST | **arch-cd-22** (asks the owner's choice of shell first) |
| 23 | dative-hbond | A05, A06 | SHOULD | **arch-web-20** (the one bond field `k`, COO decision 16); arch-cd-23 = additions |
| 24 | enhanced-stereo | C04 | SHOULD | **arch-std-12**; arch-cd-24 = additions |
| 25 | curved-arrows | G05, B11 | SHOULD | **arch-cd-25** |
| 26 | history-panel | A21 | SHOULD | **arch-cd-26** |

Other rows of this study that the other studies already carry as prompts: I07 keyboard rotation and I09 nudging = arch-std-05, A08 (either double bond) = arch-std-11, B08 (label reversal) = arch-std-08, B16 (hydrogen and carbon label display) = arch-std-10, I11 and I10 (lasso, select a piece) = arch-web-07.
Not given a prompt although MUST: K06 (the WCAG 2.2 audit with a real screen reader; arch-std-07 is the page audit).
Every prompt that is run cites `CLAUDE.md` rule 15: the feature is a `draw_edit/1` op or a `--set` setting first, with a CLI route and a native replay test, and the page is one client of it.

## 7. Context menus (by target)
The model is PR #37 (`crates/mzdraw/src/context.rs`, `Editor::context_menu(x, y)`, replay output `context:<x>,<y>`), with the page in its PR 2. This section is the input for refining it (prompt arch-cd-20).
### What the products put in each menu
Order as in the source; `>` = a submenu. ChemDraw's guide never lists a whole menu: its entries are the ones named in the task descriptions of CD-UG17 (17.1), order unknown. No product documents shortcut hints inside its menus; Ketcher's code shows none (KE-SRC).
| target | ChemDraw (CD-UG17, 17.1) | MarvinSketch (MS/pop-up) | Marvin JS (MJ/ctx) | new Marvin (M-253) | Ketcher (KE-SRC) | ChemDoodle (CDD-G) |
|---|---|---|---|---|---|---|
| atom | Show Atom Number, terminal carbon labels, generic labels >, Atom Properties >, query indicator, Display Warnings, Object Settings | Stereo, Charge, Valence, Radical, Isotope (for the element), Map, R-group items, Link Node, Delete, Group / Edit Group / Ungroup / Contract, Add > Data, Format, Edit properties, Label Editor | Duplicate, R-group attachment ±, Attached Data, Atom properties, Absolute stereo, R-logic, Paste | Charge, Radical >, Valence, enhanced stereo label, atom numbers >, Duplicate, front / back | Edit…, Enhanced stereochemistry…, Query properties >, Highlight >, — Delete | carbon label, label orientation >, attribute, Mass Number >, Implicit Hydrogens >, Stereochemistry >, Enhanced Stereochemistry >, Format, warnings on/off, Copy Text |
| bond | bond types >, Bond Position > (double-bond side), Bond Properties >, query indicator | Type, Bold, Hashed, Topology >, Reacting Center >, Arrange > (front, back), Align >, Group, Add > (H, maps, data, multicentre), Remove >, Transformation >, Format, Add to My Templates, Edit properties | Attached Data, Bond properties, Absolute stereo, Paste | Bond properties, Topology; arrows: change direction, arrow type | Edit…, — bond types, — Query bonds >, Change direction (wedge, hash, dative only), Attach S-Group, Highlight >, — Delete | Arrange > (forward, backward, top, bottom), Flip Bond Orientation, Scale Bond, Torsion Bond, Stereochemistry > |
| selection | Object Settings, Analysis (values into a caption), Group > (integral, ungroup) | not listed (some atom and bond entries appear only with a selection) | Duplicate, Cut, Copy, Attached Data, R-group attachment ±, Atom / Bond properties, Mirror, Flip, Absolute stereo, R-logic, Merge / Unmerge agents | Attach data, Clean up reaction, Copy as, Duplicate, Align and Distribute, Mirror & rotate >, Merge / Split components, Valence, IUPAC numbers, Check structure, properties | Edit selected bonds / atoms…, Bond type >, Enhanced stereochemistry…, Highlight >, — Delete | (the menu acts on the hovered object; several entries need an empty selection) |
| abbreviation | (expand / contract are commands, CD-DC26) | in the atom menu (Group, Ungroup, Contract) | Expand, Contract, Ungroup, Expand and Clean, Ungroup and Clean, Duplicate, Atom properties, … | Expand, Contract, Ungroup | Expand, Contract, Remove Abbreviation | Expand Labels (menu bar, CDD-G §14.4) |
| object | arrows: head shape, half heads, styles, fill; text: font, size, colour, Interpret Chemically; warning box: Explain This Warning; table: Borders, Align > | Bring to Front, Send to Back, Transformation > (flip, mirror, invert), Format, Add to My Templates | Duplicate, Set box colour, Absolute stereo | bracket properties; calculation box: Calculations >, Copy text; image: Crop; front / back | only the multi-tail arrow (remove tail, add tail); texts, arrows and shapes have no menu | Format…, Arrange >, arrow: Edit Reaction…; pusher: Format, Convert to Bézier; table and TLC entries |
| canvas | Paste, Paste Special > | Cut, Copy, Copy As, Paste, Select All, Group | Absolute stereo, R-logic | visibility settings: CPK colours, valence errors, display style | none (a right-click only clears the selection) | Paste text as an IUPAC name; tab menu (close, duplicate) |
Opening: Marvin JS opens menus with a long press since 6.2.0 (2013) and with Ctrl+click on macOS (MJ/marvin-js_history-of-changes.html); the new Marvin opens them with `Shift+Space`, moves with the arrow keys (Left / Right for submenus), Enter or Space chooses, Escape closes, and states screen-reader support (M-253). BIOVIA Draw: right-click reaches atom, bond and fragment properties and query options (BD-DS24); nothing more is public.

### What users ran into
- An item that does something else than it says: Ketcher #10377 (open, confirmed in 3.17.0-rc.1: "Single" raises the bond order); #4188 (fixed 3.11.0: wedge or hash direction from the menu did nothing).
- Selection rules: a right-click outside the selection must clear it the same way everywhere (Ketcher #4445, 3.21.0; #4446); items shown with nothing selected (#12151).
- Widget faults: a click on a separator closed the menu (Ketcher #3269, 3.19.0); missing separators (#7327, #7395); a menu wrong when the editor is embedded (#9671, 3.18.0); the editor still caught right-clicks after it was removed from the page (#5588).
- Touch: a long press on an iPad opened the wrong menu (Marvin JS 14.11.10); selection entries missing on macOS Ctrl+click (6.3.0) (MJ/marvin-js_history-of-changes.html).
- Missing entries asked for: reaction centre on bonds, S-group attach or edit on atoms (Ketcher #2221, #2347, #4677).
- MarvinSketch fixed a missing entry with several groups contracted at once (19.10) and a graphical-object menu that did not work (17.22) (MS/marvin_history-of-changes.html).
- Marvin JS repeats the same document-wide entries (Absolute stereo, R-logic) at the end of almost every menu (MJ/ctx), which makes every menu longer.

### What mzDraw's menu should do (on top of PR #37)
| ID | requirement | taken from | priority | twin |
|---|---|---|---|---|
| X01 | the menu is for what is under the pointer; a right-click outside the selection makes that thing the selection, the same rule for every target | KE-SRC (#4445) | MUST · built (PR #37) | `context:<x>,<y>` |
| X02 | a stale menu (the document, selection, clipboard or view changed) runs nothing | PR #37 | MUST · built | `error.draw2.contextStale` |
| X03 | every item does exactly what its label says; one test per item that checks the document after it | KE-SRC (#10377, #4188) | MUST | replay `context_run` (X13) |
| X04 | order: the most frequent actions first, kinds of the target next, clipboard, then Delete last after a separator; no document-wide tail repeated on every menu | KE-SRC (separators, Delete last); MJ/ctx (the repeated tail to avoid) | MUST | menu JSON order |
| X05 | the key of each item shown as a hint (no product documents this; ours does) | PR #37 `key` | MUST · built | `key` field |
| X06 | keyboard: `Shift+F10` and the Menu key open the menu of the focused atom or bond; arrows move, Left / Right open and close submenus, Enter / Space choose, Escape closes and gives the focus back | M-253 | MUST | page PR 2 |
| X07 | touch and pen: a long press opens the menu of what is under the finger (today a long press selects: open the menu on the same press once it has selected, or on a second long press; decide and test); the pen's barrel button acts as a right-click | MJ/marvin-js_history-of-changes.html (6.2.0, 14.11.10) | MUST | page PR 2 |
| X08 | an abbreviation is its own target: Expand, Contract, the label field, Delete | MJ/ctx, M-253, KE-SRC | MUST (needs arch-web-18) | `abbr.expand`, `abbr.contract` |
| X09 | the atom menu gains, as each feature lands: Properties… (the popover, arch-web-11), isotope and radical (arch-cd-08), enhanced stereo group (arch-std-12), show the atom number | MS/pop-up, M-253, CDD-G, CD-UG17 | MUST: each entry is part of its feature's prompt (arch-cd-08 scope; additions in arch-cd-09 for arch-web-11 and arch-cd-24 for arch-std-12) | existing ops |
| X10 | the bond menu gains: double-bond side (arch-cd-04), the bond kinds of arch-web-20, Change direction only for bonds that have one | CD-UG17 (Bond Position), KE-SRC | MUST: part of arch-cd-04 (scope) and of arch-web-20 (addition in arch-cd-23); "Change direction only where a bond has one" is in arch-cd-20 | `bond.set` |
| X11 | the selection menu gains: Clean (arch-std-15), Align and distribute (arch-cd-15), Expand / Contract when it holds labels | M-253 | SHOULD | existing commands |
| X12 | the object menu gains: arrow kind and heads (arch-web-13), text formula mode (arch-web-14), bring to front / send to back (later) | CD-UG17, M-253, MS/pop-up | SHOULD | `obj.set` |
| X13 | the CLI runs a menu item too: the replay request takes `context: [{x, y, item}]` and runs `context_menu` then `context_run`, so an agent and the parity test reach every item natively | `CLAUDE.md` rule 15 | MUST | replay request field |
| X14 | the canvas menu gains the display toggles (stereo labels of arch-web-16, warnings, atom numbers) | M-253 (visibility settings) | SHOULD | `--set display.*` |
| X15 | a warning mark has a menu: Explain (the sentence of the `check/1` warning), Ignore on this atom | CD-UG17 (Explain This Warning) | SHOULD | `check/1` keys |
| X16 | an embedded editor that is removed stops catching right-clicks | KE-SRC (#5588) | MUST: part of arch-web-01/02 (addition in arch-cd-21) | - |
| X17 | Paste text as a name | CDD-G | LATER (no name engine, F07) | - |
