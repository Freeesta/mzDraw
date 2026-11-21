# mzDraw: guidelines and standards (ARCH study `arch-std`)

Report only: no code depends on this file. It inventories the public guidelines and specifications a molecule editor that wants to be better than Ketcher must follow (structure-drawing rules, stereo drawing, journal artwork rules, file formats, identification-confidence levels, accessibility, colour), turns them into presets with numbers, a conformance checklist, the list of checks the page must run, and one feature table with priorities. The proposed DEV slices are the prompts `docs/prompts/arch-std-NN-*.md`.
Inputs in this repository: `CLAUDE.md`, `docs/mzdraw.md` ("Built state" = what exists), `docs/mzdraw-plan.md`, `docs/mzdraw-competitors.md`, `docs/mzdraw-formats.md`, `docs/mzdraw-annotations.md`.

Rules of this study: every fact carries its source (URL + version/date); everything is summarised in our own words, no text or artwork is copied; **`*` marks a fact that could not be read at a primary source** during this study (blocked host, login, paywall, request form): it comes from our general knowledge or from a secondary page and must be confirmed before code depends on it. A source behind a login, paywall or form is "not read", never guessed. Nothing here claims fidelity to any product. Read on 2026-11-20 from the cloud container of this study.

## 0. What could and could not be read
The network of this study allowed a fixed list of hosts. `www.iupac.org`, `pubs.acs.org` and `support.revvitysignals.com` are on the list but answer automated clients with a challenge page (HTTP 403), so they could not be read either. The IUPAC recommendations themselves are on IUPAC subdomains (`joomla.iupac.org`, `list.iupac.org`, `publications.iupac.org`) and on `iupac.qmul.ac.uk`, the publishers' author guidelines on their own hosts; those hosts were requested from the coordinator and were not open when this file was written. Everything that depends on them is marked `*` and is a task for a second pass, not a guess to build on.

## 1. Source register
| id | source | version / date | URL | read? |
|---|---|---|---|---|
| S1 | IUPAC, Brecher, "Graphical representation standards for chemical structure diagrams" (IUPAC Recommendations 2008), Pure Appl. Chem. 80(2), 277-410, DOI 10.1351/pac200880020277 | 2008 | https://www.iupac.org/publications/pac/pdf/2008/pdf/8002x0277.pdf ; HTML edition https://iupac.qmul.ac.uk/drawing/drawing.html | **not read** (403 / host not allowed) `*` |
| S2 | IUPAC, Brecher, "Graphical representation of stereochemical configuration" (IUPAC Recommendations 2006), Pure Appl. Chem. 78(10), 1897-1970, DOI 10.1351/pac200678101897 | 2006 | https://joomla.iupac.org/publications/pac/2006/pdf/7810x1897.pdf ; HTML edition https://iupac.qmul.ac.uk/drawing/stereo.html | **not read** `*` |
| S3 | IUPAC Blue Book 2013, chapter P-9 (configuration and conformation; points to S2 for drawings) | 2013 | https://iupac.qmul.ac.uk/BlueBook/P9.html | **not read** `*` |
| S4 | ACS "Document 1996" drawing settings (ChemDraw style sheet used by ACS journals) and ACS author guidelines for graphics | ACS pages undated* | https://pubs.acs.org/page/4authors/index.html (and each journal's "Author Guidelines") | **not read** (403) `*`; the numbers are already in `settings.rs` from RDKit and the format study |
| S5 | RSC author guidelines (artwork, structures) | - | https://www.rsc.org/journals-books-databases/author-and-reviewer-hub/ | **not read** (host not allowed) `*` |
| S6 | Elsevier artwork and media instructions | - | https://www.elsevier.com/about/policies-and-standards/author/artwork-and-media-instructions | **not read** `*` |
| S7 | Wiley author guidelines (graphics; Chemistry Europe and Angewandte style) | - | https://authorservices.wiley.com/author-resources/Journal-Authors/Prepare/manuscript-preparation-guidelines.html | **not read** `*` |
| S8 | Springer Nature artwork guidelines (Nature Portfolio and Springer journals) | - | https://www.nature.com/nature/for-authors/formatting-guide ; https://www.springernature.com/gp/authors/campaigns/how-to-submit-a-journal-article-manuscript/figures-and-tables | **not read** `*` |
| S9 | BIOVIA "CTfile Formats" (molfile, SDF, RXN, RDF) | 2020 edition* | https://discover.3ds.com/ctfile-documentation-request-form | **not read**: behind a request form (the earlier slices were built from it, see `mzdraw-formats.md` §1) `*` |
| S10 | CDX / CDXML SDK documentation (Revvity, formerly CambridgeSoft), public mirror | undated | https://chemapps.stolaf.edu/iupac/cdx/sdk/IntroCDXML.htm | **not read** in this study (host not allowed); slice 3d was built from it |
| S11 | KET JSON schema, `ketcher-core` (Apache-2.0), JSON Schema draft-07 | Ketcher master `7f69095`, release v3.21.0 | https://raw.githubusercontent.com/epam/ketcher/master/packages/ketcher-core/src/domain/serializers/ket/schema.json | read |
| S12 | Ketcher arrow modes (`RxnArrowMode`, value names only) | master `7f69095` | https://raw.githubusercontent.com/epam/ketcher/master/packages/ketcher-core/src/domain/entities/rxnArrow.ts | read |
| S13 | CML Schema 3 (BlueObelisk `cml-schema`, Artistic-2.0) | commit `577a61d`, 2014-03-26 | https://github.com/BlueObelisk/cml-schema | read (README, schema header) |
| S14 | Chemaxon documentation: CXSMILES/CXSMARTS, MRV, molfile pages | `docs.chemaxon.com/latest`, read 2026-11-20 | https://docs.chemaxon.com/latest/formats_chemaxon-extended-smiles-and-smarts-cxsmiles-and-cxsmarts.html ; https://docs.chemaxon.com/latest/formats_marvin-documents-mrv.html | read |
| S15 | OpenSMILES specification (GNU FDL 1.2) | v1.0, 2016-05-15; repository commit `9240a28` (2026-04-13) | https://github.com/opensmiles/OpenSMILES (`opensmiles.asciidoc`) | read |
| S16 | Daylight Theory Manual (SMILES, SMARTS) | 4.9* | https://www.daylight.com/dayhtml/doc/theory/ | **not read** (host not allowed) `*` |
| S17 | InChI Technical Manual, software 1.07 | revision 2024-01-19; latest release v1.07.5 (2026-02-17); `main` at `7f26903` (2026-11-10) carries the unreleased v2026.1 changelog | https://github.com/IUPAC-InChI/InChI (`INCHI-1-DOC/TechMan/InChI_TechMan.pdf`, `INCHI-1-DOC/CHANGELOG.md`) | read |
| S18 | W3C SVG 1.1 (Second Edition), Recommendation | 2011-08-16 | https://www.w3.org/TR/2011/REC-SVG11-20110816/ | read (status) |
| S19 | W3C SVG 2, Candidate Recommendation Snapshot | 2026-10-06 | https://www.w3.org/TR/2026/CR-SVG2-20261006/ | read (status) |
| S20 | W3C PNG Specification (Third Edition), Recommendation | 2025-06-24 | https://www.w3.org/TR/2025/REC-png-3-20250624/ | read (status, chunk list) |
| S21 | ISO 32000-2:2020 (PDF 2.0) | 2020 | https://www.pdfa.org/resource/iso-32000-2/ | **not read** (host not allowed) `*` |
| S22 | W3C WCAG 2.2, Recommendation | 2024-12-12 | https://www.w3.org/TR/2024/REC-WCAG22-20241212/ | read |
| S23 | W3C WAI-ARIA 1.2, Recommendation; WAI-ARIA Graphics Module, Recommendation | 2023-06-06; 2018-10-02 | https://www.w3.org/TR/2023/REC-wai-aria-1.2-20230606/ ; https://www.w3.org/TR/graphics-aria-1.0/ | read (status) |
| S24 | W3C SVG Accessibility API Mappings, Working Draft | 2026-10-21 | https://www.w3.org/TR/svg-aam-1.0/ | read (status) |
| S25 | W3C CSS Color Adjustment Level 1 (forced colours), CR Snapshot; Media Queries Level 5 (reduced motion), WD; Clipboard API and events, WD | 2025-12-16; 2026-02-19; 2026-06-24 | https://www.w3.org/TR/css-color-adjust-1/ ; https://www.w3.org/TR/mediaqueries-5/ ; https://www.w3.org/TR/clipboard-apis/ | read (status) |
| S26 | WAI-ARIA Authoring Practices Guide, toolbar pattern | living document | https://www.w3.org/WAI/ARIA/apg/patterns/toolbar/ | read (title) |
| S27 | Schymanski et al., "Identifying small molecules via high resolution mass spectrometry: communicating confidence", Environ. Sci. Technol. 48(4), 2097-2098, DOI 10.1021/es5002105 (PubMed 24476540) | 2014-02-18 | https://pubs.acs.org/doi/10.1021/es5002105 | **text not read** (403); bibliographic record read on PubMed; the level meanings used by mzDraw are the ones already in `mzdraw-annotations.md` `*` |
| S28 | Charbonnet et al., "Communicating confidence of per- and polyfluoroalkyl substance identification via high-resolution mass spectrometry", Environ. Sci. Technol. Lett. 9(6), 473-481, DOI 10.1021/acs.estlett.2c00206 (PMC9202347) | 2022-06-14 | https://pmc.ncbi.nlm.nih.gov/articles/PMC9202347/ | read (full text through PubMed Central; its Table 1 is an image and was not read) |
| S29 | PubChem periodic table (`CPKHexColor`, the Jmol element colours) | read 2026-11-20 | https://pubchem.ncbi.nlm.nih.gov/rest/pug/periodictable/JSON | read |
| S30 | Machado, Oliveira, Fernandes, "A physiologically-based model for simulation of color vision deficiency", IEEE TVCG 15(6), 2009 (simulation matrices, severity 1.0) | 2009 | https://www.inf.ufrgs.br/~oliveira/pubs_files/CVD_Simulation/CVD_Simulation.html | **not read**: matrices used as recalled `*` |
| S31 | Okabe & Ito, "Color Universal Design"; Tol, "Colour schemes" (colour-blind-safe qualitative palettes) | 2008; 2021* | https://jfly.uni-koeln.de/color/ ; https://personal.sron.nl/~pault/ | **not read** (hosts not allowed) `*` |

## 2. Guideline inventory

### 2.1 Structure diagrams (S1, IUPAC 2008) `*`
The recommendations are organised as rules with a grade (preferred, acceptable, not acceptable) and give principles more than numbers: know the audience, avoid styles that can be read two ways, be consistent inside one document. What an editor must apply, as we know the document (`*`, to confirm on S1):
- **Geometry**: one bond length throughout a diagram; chains as a zigzag with 120° between consecutive bonds; rings as regular polygons; a triple bond and the bonds of a cumulated (allene) carbon drawn collinear (180°); an atom with four plain neighbours drawn with the angles spread as evenly as the neighbours allow; no atom placed on a bond it does not belong to; no two atoms closer than a fraction of a bond length; crossing bonds avoided, and where a crossing cannot be avoided the bond behind is drawn with a gap.
- **Double bonds**: in a ring, the second line inside the ring, shorter at both ends; outside rings, centred on the atoms when one end has no other neighbour (C=O) and offset to the side of the substituents otherwise. (Built in `scene.rs`: `double_side`.)
- **Labels**: carbon atoms are not labelled except where needed (some styles label terminal methyls); heteroatoms are always labelled and carry their hydrogens; the hydrogens stand on the side away from the bonds; a charge is written at the upper right of its symbol (a circled charge is a variant `*`); a mass number is a left superscript; the bond meets the label at its first (attachment) atom, so a group written to the left of its bond is reversed (`HO2C-`, `MeO-`).
- **Aromatic rings**: a Kekulé structure is preferred; an inner circle is acceptable for a single benzene ring and discouraged for fused systems, where it hides the electron count `*`.
- **Fonts and lines**: a sans-serif face, line widths and label sizes consistent across a figure; numeric values are style-sheet matters (ACS 1996 below), not part of S1 `*`.

### 2.2 Stereo drawing (S2, IUPAC 2006 `*`; S17, InChI Technical Manual, read)
From S17 (read, §III.d and the tables of the stereo layer):
- InChI reads wedges by default as "the narrow end of a wedge is at the stereocentre": a wedge acts on one atom only. An option (`/NEWPSOFF`) reads them in "perspective", acting on both atoms of the bond. The two readings give different identifiers for the same picture when a wedge joins two stereocentres. **Consequence for mzDraw**: a wedge whose wide end is also a stereocentre is ambiguous between programs; the editor must warn about it.
- For the parity of a centre, InChI lifts the wide end of a solid wedge 45° above the plane and puts the wide end of a hashed wedge 45° below, after setting every bond to the same length; a centre outside the tetrahedron of its neighbours gives a warning. **Consequence**: the check can use the same construction to detect drawings whose configuration depends on bond lengths or crosses the centre.
- 3D coordinates override wedges; a wavy ("either") bond always means unknown.
- Atoms InChI treats as possible tetrahedral centres: C, Si, Ge, N+, P, P+, As, B-, Sn and others listed in its Table 8; possible stereo double bonds involve C, Si, Ge and N (with charged forms listed in its Table 7). A double bond in a system without a complete alternating pattern (radicals, some zwitterions) is not stereogenic for InChI.
- Allenes: the configuration must be shown by two wedges at the same end of the allene with opposite marks (a stated limitation of the InChI software).
- Absolute versus relative stereo: InChI's `/s1` (absolute) and `/s2` (relative); the molfile chiral flag selects between them under the option `/SUCF`. The unreleased v2026.1 adds an option reading the V3000 enhanced-stereo collections (`STEABS`, `STEREL`, `STERAC`) as a beta feature.
From S2 (`*`, as we know the document; to confirm):
- Plain wedge (solid) and hashed wedge are the preferred marks; the narrow end sits on the stereocentre; a hashed mark drawn as parallel lines of equal length is acceptable, a "bold" bond used as a stereo mark is not; a wavy bond marks unknown configuration.
- A stereocentre should be drawn with at least three neighbours spread so that the configuration cannot be misread; two stereo marks at one centre are drawn on bonds that are not opposite (not at 180°); a wedge should not join two stereocentres; at a ring fusion the hydrogen is drawn explicitly with its wedge.
- Unknown E/Z: a wavy bond to a substituent of the double bond is preferred; a crossed double bond is discouraged or not acceptable `*`.
- Perspective, Fischer, Haworth and chair drawings are acceptable in their classes of compounds but must not be mixed with wedge conventions, and are not suited to structures read by software.

### 2.3 Journal style sheets and artwork rules (S4-S8) `*`
- **ACS "Document 1996"** (the numbers the built preset `acs-1996` uses; S4 not read, numbers confirmed on RDKit and ChemDraw secondary pages, `mzdraw-formats.md` §4): fixed bond length 14.4 pt (0.508 cm), line width 0.6 pt, bold width 2.0 pt, bond spacing 18 % of the bond length, hash spacing 2.5 pt, margin around labels 1.6 pt, Arial (Helvetica) 10 pt, chain angle 120°.
- **ACS artwork rules** (`*`, S4 not read): one-column width about 3.25 in (8.25 cm), two-column about 7 in (17.8 cm); line art at high resolution (1200 dpi), halftones 300 dpi, combinations 600 dpi; a table-of-contents graphic about 3.25 in × 1.75 in; fonts restricted to a common list (Arial, Helvetica, Times, Symbol, Courier). These must be read on S4 before any export check uses them.
- **RSC, Elsevier, Wiley, Springer Nature**: not read. As remembered (`*`, do not build on them): column widths near 8.3 / 17.1 cm (RSC), 90 / 140 / 190 mm (Elsevier), 89 / 183 mm (Nature); minimum line widths around 0.25-0.5 pt; fonts Arial or Helvetica at 5-8 pt in the printed size; resolutions as ACS. Wiley chemistry journals ask for ChemDraw drawings close to the ACS settings `*`. Rule kept from `mzdraw-formats.md` §4: **no preset for these publishers ships until its numbers are read on the publisher's current page.**

### 2.4 File formats (latest versions)
| format | latest version found | what a conformant editor must do (beyond what is built) | source |
|---|---|---|---|
| molfile / SDF / RXN / RDF | CTfile Formats 2020* | V3000 enhanced-stereo collections (`MDLV30/STEABS`, `STERELn`, `STERACn`) and the chiral flag read and written; atom aliases (`A` lines) and atom values kept; coordinate (dative) bond type 9 read (InChI v2026.1 accepts it, S17); RXN V3000 and RD files written; atom-atom maps | S9 `*`, S17 |
| SMILES | OpenSMILES 1.0 (2016-05-15) | reader: one SMILES per line, name after whitespace, LF or CR/LF, blank lines and lines starting with whitespace ignored (OpenSMILES "SMILES Files"); writer "standard form": bare organic-subset atoms, `+`/`-` without digit, H count 1 without digit, property order chirality-H-charge, no reuse of ring digits, ring closures on single bonds, start on a terminal or hetero atom, aromatic form preferred, no stereo marks on non-stereo atoms; partial stereo allowed | S15 |
| CXSMILES | Chemaxon page (latest) | the extension block after the SMILES: coordinates, atom labels / aliases / values, radicals, coordinate and hydrogen bonds, absolute flag, enhanced stereo groups, wiggly and up/down bonds, ring cis/trans, R-groups, link nodes, S-groups; a writer should give coordinates and enhanced stereo so a round trip keeps the drawing | S14 |
| InChI / InChIKey | software v1.07.5 (2026-02-17); v2026.1 in development (calendar versioning, `1S`/`1B`/`1N` prefixes, molecular inorganics on by default) | an editor shows InChI and InChIKey and the warnings of the stereo perception; mzDraw computes them only outside the browser module (decision 5 of `mzdraw-plan.md`) | S17 |
| KET | schema draft-07 on Ketcher master (v3.21.0) | built. Still open: the 18 arrow modes (open-angle, filled-triangle, filled-bow, dashed-open-angle, failed, retrosynthetic, both-ends-filled-triangle, 3 equilibrium, 4 unbalanced-equilibrium, 4 elliptical-arc); text runs with font, size, colour, bold, italic, sub/superscript; simple objects (line, rectangle, circle, ellipse, polyline); enhanced stereo flags; `cip` on atoms and bonds | S11, S12 |
| CDXML / CDX | SDK pages (undated) | built for CDXML; CDX binary later | S10 |
| CML | Schema 3 (2014) | built (molecular convention) | S13 |
| MRV | Chemaxon page (latest) | read built; writing later | S14 |
| SVG | SVG 1.1 SE is the last Recommendation (2011); SVG 2 is a CR Snapshot (2026-10-06) | write SVG 1.1 features only (no SVG 2-only element) for the widest reader base; give each saved picture a `<title>` and `<desc>` (text alternative, WCAG 1.1.1) and `role="img"` (ARIA Graphics, S23) | S18, S19, S23, S24 |
| PNG | Third Edition (2025-06-24) | built (`iTXt`, `pHYs`); the Third Edition adds `cICP`, `mDCV`, `cLLI` and makes `eXIf` official: none needed; an `sRGB` chunk says the colours are sRGB | S20 |
| PDF | ISO 32000-2:2020 (PDF 2.0)* | vector, text as text, base-14 fonts or an embedded subset; the document embedded as an attached file (`mzdraw-formats.md` §2); decision 4 | S21 `*` |

### 2.5 Identification confidence (S27, S28)
- Schymanski 2014 levels, as already used by mzDraw (`mzdraw-annotations.md`; text of S27 not read in this study `*`): 1 confirmed structure (reference standard), 2a probable (library spectrum), 2b probable (diagnostic evidence), 3 tentative candidate(s), 4 unequivocal molecular formula, 5 exact mass of interest. Built: levels as strings, evidence, scores, history, `L2b` under the structure.
- Charbonnet 2022 (S28, read) is a PFAS-specific extension, not a replacement: 1a and 1b (level 1 split: confirmed by a standard / indistinguishable from it because isomers give the same spectrum), 2a, 2b, 2c (2c rests on homologue evidence), 3a-3d (level 3 split by the kind of evidence), 4 (unchanged), 5a and 5b (suspect-list mass / mass found by data filtering). It says that sub-levels are defined per study with the evidence for each structure shown, that at 3a every positional isomer is named, that branched and linear isomers are reported together at the highest level any of them reached, and that level 5 is reported only in exceptional cases.
- **Consequence for mzDraw**: (1) the `scale` field must name the scale (`schymanski-2014`, `charbonnet-2022`, any other word kept), and the page offers the levels of the chosen scale; the built rule "an unknown level word is kept" already accepts `1a` or `5b`; (2) a structure at level 3 is one candidate among several and should say so in the picture (`L3`, candidate count when known); at levels 4 and 5 there is no structure to draw, so the annotation needs a carrier that is not a structure (a formula or an m/z text object): this is a depiction rule we derive, not a rule of S27/S28.

### 2.6 Accessibility of the editor (S22 WCAG 2.2, S23-S26)
Success criteria that bind a canvas editor, with their numbers (S22, read) and the state in mzDraw:
| SC | level | requirement (our words) | mzDraw |
|---|---|---|---|
| 1.1.1 Non-text content | A | a text alternative for every picture | canvas has an outline ("What is drawn"); saved SVG has no `<title>`/`<desc>` yet |
| 1.3.4 Orientation | AA | works in portrait and landscape | to test |
| 1.4.1 Use of colour | A | colour is never the only carrier of meaning | element symbols are text (pass); check marks and selection need a non-colour cue: to test |
| 1.4.3 Contrast (minimum) | AA | text 4.5:1, large text (18 pt, or 14 pt bold) 3:1 | atom labels are text: the Jmol colours fail on white (table 2.7) |
| 1.4.10 Reflow | AA | usable at 320 CSS px width without two-way scrolling (canvas content is an allowed exception, the controls are not) | to test |
| 1.4.11 Non-text contrast | AA | 3:1 for controls, states and graphics needed to understand content (bonds, selection, focus ring, hover) | to test |
| 1.4.12 Text spacing | AA | no loss with line height 1.5, paragraph spacing 2, letter spacing 0.12, word spacing 0.16 (times the font size) | to test |
| 1.4.13 Content on hover or focus | AA | tooltips dismissible, hoverable, persistent | to test |
| 2.1.1 Keyboard | A | every function from the keyboard | atoms yes; bonds and objects not focusable yet; arrows and texts not placeable from the keyboard |
| 2.1.2 No keyboard trap | A | | built (Tab never taken) |
| 2.1.4 Character key shortcuts | A | a one-key shortcut can be turned off, remapped with a modifier, or is active only while its component has focus | one-key shortcuts act only while the canvas has focus (to confirm by test); no off switch or remap yet |
| 2.4.7 / 2.4.11 Focus visible / not obscured (min) | AA | the focus is visible and not wholly hidden by other content | focus rings built; popovers and the help must not cover it: to test |
| 2.4.13 Focus appearance | AAA | indicator at least the area of a 2 CSS px perimeter, 3:1 change | SHOULD |
| 2.5.1 Pointer gestures | A | multipoint or path gestures have a single-pointer alternative | pinch zoom has buttons (built) |
| 2.5.2 Pointer cancellation | A | action on the up event, abortable | built (`pointercancel` commits nothing) |
| 2.5.7 Dragging movements | AA | everything done by dragging can be done by single pointer actions without dragging | moving, rotating and band selection need a non-drag way (keys or clicks): partly missing |
| 2.5.8 Target size (minimum) | AA | targets at least 24 × 24 CSS px, or spaced so 24 px circles do not overlap | buttons to measure; plan says 40 px on touch |
| 3.2.6 Consistent help | A | help in the same place on every page | one page: pass |
| 3.3.1 Error identification | A | errors described in text | import and export errors are said in words (built) |
| 3.3.7 Redundant entry | A | do not ask twice for the same input | pass (settings remembered) |
| 4.1.2 Name, role, value | A | every control has a name and state | buttons with `aria-pressed` (built) |
| 4.1.3 Status messages | AA | changes announced without moving focus | `aria-live` line (built) |
Also: forced-colours mode (S25, CSS Color Adjust: the page must keep the canvas legible when the system forces colours) and `prefers-reduced-motion` (S25); the toolbar should follow the APG toolbar pattern (S26: one tab stop, arrow keys inside).

### 2.7 Element colours and colour vision (S29-S31)
WCAG contrast (S22 formula, relative luminance of sRGB) of the Jmol colours as PubChem publishes them (S29), computed for this study:
| element | Jmol colour | on white | on #1E1E1E |
|---|---|---|---|
| H | #FFFFFF | 1.00 | 16.67 |
| C | #909090 | 3.19 | 5.22 |
| N | #3050F8 | 5.79 | 2.88 |
| O | #FF0D0D | 3.95 | 4.22 |
| F | #90E050 | 1.62 | 10.29 |
| Cl | #1FF01F | 1.55 | 10.75 |
| Br | #A62929 | 7.07 | 2.36 |
| I | #940094 | 7.82 | 2.13 |
| S | #FFFF30 | 1.07 | 15.56 |
| P | #FF8000 | 2.52 | 6.62 |
| B | #FFB5B5 | 1.68 | 9.95 |
| Si | #F0C8A0 | 1.56 | 10.70 |
On white only N, Br and I reach the 4.5:1 that small label text needs (1.4.3); O, P, S, F, Cl fail; on a dark ground N, Br and I fail instead. **No single set of colours serves both a light and a dark background at 4.5:1** (a colour needs relative luminance ≤ 0.183 for white and ≥ 0.204 for #121212). So mzDraw needs one palette per background, chosen for contrast first; the hue only follows the usual convention (N blue, O red, S yellow-brown, P orange, halogens green, I violet).
A proposal, computed here (contrast against the background; closest pair of N, O, S, P, F, Cl, Br, I in CIE L*a*b* ΔE76 after simulating protanopia, deuteranopia and tritanopia with the matrices of S30 `*`):
| element | light (on #FFFFFF) | contrast | dark (on #121212) | contrast |
|---|---|---|---|---|
| C, default ink | #222222 | 15.91 | #E6E6E6 | 15.01 |
| H (in labels) | #444444 | 9.74 | #CCCCCC | 11.67 |
| N | #1F45C8 | 7.65 | #8CA8FF | 8.17 |
| O | #C41E1E | 5.91 | #FF7A7A | 7.42 |
| S | #7A6400 | 5.75 | #F0D44A | 12.69 |
| P | #B04A00 | 5.49 | #FFA45C | 9.56 |
| F | #1E7A3A | 5.38 | #8FE39A | 12.15 |
| Cl | #177217 | 6.08 | #6EDC6E | 10.82 |
| Br | #8E2323 | 8.70 | #E79A9A | 8.46 |
| I | #6E1F8F | 9.31 | #D3A0F0 | 8.97 |
| B | #9C4F2A | 5.89 | #FFB58C | 10.92 |
| Si | #5E5242 | 7.60 | #D8C8A8 | 11.37 |
| other metals | #505A6A | 6.97 | #A8B4C8 | 8.95 |
Closest pairs under simulation: light palette protan S/Cl ΔE 1.9, deutan O/S 3.3, tritan F/Cl 3.8 (normal vision F/Cl 13.3); dark palette protan N/I 6.7, deutan O/F 12.4, tritan P/Br 6.0. A random search with all eight colours at ≥ 4.5:1 on white reached a worst-case ΔE of only about 10 and needed near-black blues and browns: **on a light page, contrast and colour-blind distinctness pull against each other**. The way out is the one WCAG 1.4.1 already asks for: the element symbol (text) carries the identity and colour is a redundant cue, so the palette is chosen for contrast, `mono` stays one click away (built), and nothing else on the canvas (selection, warnings, evidence highlights) is told apart by hue alone. Colour-blind-safe qualitative palettes (S31 `*`) are the starting point for highlight colours, not for elements.

## 3. Presets with numbers
Units pt unless said; "bond spacing" is a fraction of the bond length. Built presets are in `crates/mzdraw/src/settings.rs`.
| preset | bond length | line | bold (wedge width) | hash spacing | bond spacing | margin | font | colour | state / source |
|---|---|---|---|---|---|---|---|---|---|
| `mzlab` | 30 | 1.35 | 6.0 | 3.6 | 0.13 | 2.04 | 12, sans | element | built, ours |
| `acs-1996` | 14.4 | 0.6 | 2.0 | 2.5 | 0.18 | 1.6 | 10, Arial/Helvetica | mono | built; S4 `*` |
| `slides` | 21.6 | 1.5 | 4.0 | 4.0 | 0.18 | 2.5 | 16 | mono | built, ours |
| `mzlab-dark` (proposed) | as `mzlab` | | | | | | | the dark palette of 2.7, background #121212 | prompt 03 |
| `rsc`, `elsevier`, `wiley`, `nature` | - | - | - | - | - | - | - | - | not shipped until S5-S8 are read |
Fields the standards ask for that the style does not have yet (each a key of `mzdraw-settings/1`, read through `settings::FIELDS`): `chain_angle` (120°, layout only), `carbon_labels` none|terminal|all, `hydrogens` hetero|all|none, `hash_style` wedge|lines, `background` (colour), `palette` light|dark|mono, `crossing_gap` (pt, 0 = off), `font_family` (a list). Every new key is optional, defaults to the current look and keeps old files valid.

## 4. Conformance checklist (what the page must validate)
A check never blocks (the user decides, `CLAUDE.md`); it gives a key, the ids it is about and a grade. The report is one JSON, `std/1` (`{"format":"std/1","checks":[{"key","grade","atoms","bonds","objects"}]}`, ordered by key then id, the same bytes in the page and from the replay request `"out":"std"`). Grades: `error` (a reader will get the chemistry wrong), `warn` (ambiguous or against a recommendation), `info`.
| key | check | grade | source | when |
|---|---|---|---|---|
| `std.stereo.wideEndCentre` | a solid or hashed wedge whose wide end is also a possible stereocentre (read differently by the two InChI conventions) | warn | S17, S2 `*` | live |
| `std.stereo.notCentre` | a wedge whose narrow end is not a possible stereocentre (fewer than 3 neighbours plus H, or an atom not in InChI's list) | warn | S17 | live |
| `std.stereo.opposite` | two stereo marks at one centre on bonds within 10° of opposite (180°) | warn | S2 `*` | live |
| `std.stereo.sameKindThree` | a centre with three drawn neighbours and marks that do not fix the configuration (same-kind marks with the centre outside the triangle of its neighbours) | error | S17 (parity construction) | live |
| `std.stereo.collinear` | a stereocentre with two neighbours drawn collinear through it | warn | S2 `*` | live |
| `std.stereo.wavyNotCentre` | a wavy bond at an atom that is not a possible stereocentre and not next to a stereo double bond | info | S2 `*`, S17 | live |
| `std.stereo.undefined` | a possible stereocentre or stereo double bond with no mark (configuration not given) | info | S17 | live |
| `std.stereo.doubleCollinear` | a possible stereo double bond with a substituent drawn collinear with it (E/Z unreadable) | warn | S17, S2 `*` | live |
| `std.stereo.allene` | an allene with stereo marks that are not two opposite wedges at one end | warn | S17 | live |
| `std.geom.atomClash` | two atoms closer than 0.3 bond length | error | S1 `*` | live |
| `std.geom.atomOnBond` | an atom within 0.15 bond length of a bond it is not on | error | S1 `*` | live |
| `std.geom.crossing` | two bonds that cross (no shared atom) | warn | S1 `*` | live |
| `std.geom.length` | a bond whose length differs from the median of its structure by more than 15 % | info | S1 `*` | export |
| `std.geom.labelClash` | two labels whose boxes overlap, or a label over a bond | warn | S1 `*` | export |
| `std.label.chargeHidden` | a charge that cannot take the upper right (crowded) | info | S1 `*` | export |
| `std.ann.noStructure` | an annotation at level 4 or 5 (or a sub-level of them) on a full structure | info | S27 `*`, S28 | live |
| `std.access.contrast` | a colour of the style below 4.5:1 (labels) or 3:1 (bonds, marks) on the background | warn | S22 1.4.3 / 1.4.11 | on style change |
| `std.export.font` | a font outside the preset's list | warn | S4-S8 `*` | export (later) |
| `std.export.lineWidth` | a line thinner than the preset's minimum at print size | warn | S4-S8 `*` | export (later) |
| `std.export.width` | a figure wider than the preset's column width | info | S4-S8 `*` | export (later) |
Built checks that stay as they are: valence and charge (`check/1`), import report (`Imported::notes`), annotation shape (`error.draw2.ann`), SDF field limits (`error.draw2.data`). The thresholds above (0.3, 0.15, 10°, 15 %) are ours, chosen to flag only clear cases; the DEV slices pin them in tests and may tune them with a reason.

## 5. Feature table
Priority for mzDraw: **MUST** (needed to be better than Ketcher for a chemist who publishes; or required by a standard the editor claims), **SHOULD** (competitive, a standard recommends it), **LATER** (valuable, depends on a decision or another slice), **OUT** (outside `CLAUDE.md` scope or outside the browser module). "Twin" = the scriptable form (rule 10: the page only emits commands; every function is producible from `mzlab-cli mol draw`). Size = Rust work in mzDraw: S one small PR, M 2-3, L 4+. "built" = in "Built state" of `mzdraw.md` (listed for completeness, no prompt).
| # | feature | source | priority (reason) | twin | size |
|---|---|---|---|---|---|
| 1 | one bond length, 120° zigzag chains, regular rings for structures without coordinates (built) | S1 `*` | MUST (built) | replay `import` | - |
| 2 | clean-up of an existing drawing (layout of `mzlab-mol` on chosen atoms, one undo step) | S1 `*` | MUST (a drawing drifts; Ketcher has it) | new op `atoms.xy` `{"op":"atoms.xy","atoms":[...],"xy":[[x,y],...]}`, inverse = old positions | M |
| 3 | double-bond placement inside rings, centred for terminal ends (built) | S1 `*` | MUST (built) | - | - |
| 4 | gap in the rear bond at a crossing | S1 `*` | SHOULD (only where a crossing cannot be avoided) | setting `style.crossing_gap` | S |
| 5 | label reversal on the left side (`HO2C`, `MeO`, `F3C`, `H2N`) | S1 `*` | MUST (a left-side `CO2H` reads as C-bonded-O wrong) | rendering of `lab`, no op | S |
| 6 | carbon labels none/terminal/all and hydrogen display modes | S1 `*` | SHOULD (journal styles differ) | settings `style.carbon_labels`, `style.hydrogens` | S |
| 7 | charges moved off a crowded upper right | S1 `*` | SHOULD | rendering | S |
| 8 | aromatic circle display | S1 `*` | LATER (discouraged for fused rings) | setting `style.aromatic=circle` | M |
| 9 | wedge, hash, wavy marks, narrow end at atom `a` (built) | S2 `*`, S17 | MUST (built) | `bond.set` `s` | - |
| 10 | hashed mark drawn as wedge or as parallel lines | S2 `*` | SHOULD | setting `style.hash_style` | S |
| 11 | stereo drawing check (`std.stereo.*`) | S2 `*`, S17 | MUST (no free editor warns about ambiguous wedges; InChI conventions differ) | replay `"out":"std"` | M |
| 12 | unknown E/Z: wavy bond to a substituent, optional crossed double bond, written to molfile (`3` on a double bond), SMILES (no `/` `\`), KET | S2 `*`, S9 `*`, S15 | SHOULD (metabolites with unknown geometry) | `bond.set` `s:3` on a double bond | M |
| 13 | enhanced stereo groups (absolute, and-n, or-n) and the chiral flag: kept, drawn (`&1`, `or1`), written (V3000, CXSMILES, KET) | S9 `*`, S14, S17 | SHOULD (built import loses the grouping) | new op `stereo.group` | M |
| 14 | CIP labels (R/S, E/Z) on the canvas | S3 `*` | SHOULD (needs a CIP engine in `mzlab-mol`; `cip/1` is in `mzlab-feat` only) | setting `style.cip`; `mzlab-cli mol cip` | M |
| 15 | allene and atropisomer stereo | S17 | LATER | `bond.set` | M |
| 16 | geometry check (`std.geom.*`) | S1 `*` | MUST (a picture with clashes is not publishable) | replay `"out":"std"` | M |
| 17 | coordinate (dative) bond, hydrogen bond, bold and dashed display bonds | S9 `*`, S14, S17 | SHOULD (organometallics; InChI v2026.1 reads type 9) | `bond.add`/`bond.set` new order codes | M |
| 18 | atom aliases (molfile `A`, CXSMILES `$...$`) kept and shown | S9 `*`, S14 | SHOULD (built import ignores them) | `atom.set` `lab` | S |
| 19 | OpenSMILES file rules in the reader (names, CR/LF, blank and indented lines) | S15 | MUST (stated by the spec; tests missing) | replay `import` | S |
| 20 | SMILES standard form option (aromatic output) | S15 | SHOULD (writer of `mzlab-mol`) | `out` `smiles-aromatic` | S (in `mzlab-mol`) |
| 21 | CXSMILES out (coordinates, enhanced stereo, R groups) | S14 | SHOULD (round trip with RDKit and Chemaxon keeps the drawing) | `out` `cxsmiles` | M |
| 22 | InChI and InChIKey with stereo warnings | S17 | OUT of the wasm (decision 5); LATER in app and CLI | `mzlab-cli` | L |
| 23 | KET arrow modes (18) and text runs with sub/superscript | S11, S12 | MUST for sub/superscript text (conditions, `H2O`), SHOULD for arrow kinds | `obj.add` `kind`, `runs` | M |
| 24 | KET shapes (line, rectangle, circle, ellipse, polyline) | S11 | LATER | `obj.add` | M |
| 25 | atom-atom maps in RXN, KET, SMILES | S9 `*`, S11, S15 | SHOULD (mechanisms, transformation products) | `atom.set` `map` | M |
| 26 | RXN V3000 and RD file out | S9 `*` | SHOULD | `out` `rxn-v3000`, `rdf` | S |
| 27 | SVG with `<title>`, `<desc>`, `role="img"`, language | S18, S22 1.1.1, S23 | MUST (figures in accessible reports) | `out` `svg` | S |
| 28 | SVG 1.1 features only in what we write | S18, S19 | MUST (wide reader base; already true, pin by test) | `out` `svg` | S |
| 29 | PNG `sRGB` chunk | S20 | SHOULD | `png::finish` | S |
| 30 | PDF out with the document attached | S21 `*` | SHOULD (decision 4) | `out` `pdf` | L |
| 31 | journal presets other than ACS | S5-S8 `*` | LATER (not read) | `style` | S each |
| 32 | export checks (font list, minimum line width, column width) | S4-S8 `*` | SHOULD after S4-S8 are read | `"out":"std"` | S |
| 33 | element palettes per background with tested contrast, dark theme | S22, S29 | MUST (Jmol colours fail 4.5:1) | settings `style.palette`, `style.background` | S |
| 34 | highlight colours safe for colour vision | S31 `*` | LATER (with evidence colours) | settings | S |
| 35 | identification scale field and the levels of `schymanski-2014` and `charbonnet-2022` on the page | S27 `*`, S28 | SHOULD (PFAS studies use 1a-5b) | `atom.ann` `scale` | S |
| 36 | level-aware depiction (`L3` candidate, formula or m/z carrier for levels 4 and 5) | S27 `*`, S28 | LATER (needs a text object that carries an annotation) | `obj.add` + `atom.ann` | M |
| 37 | caption line with evidence under the structure | `mzdraw-formats.md` §5 | SHOULD (built "not built") | setting `style.caption` | S |
| 38 | every function from the keyboard: focus on bonds and objects, place arrows and texts | S22 2.1.1 | MUST | existing ops | M |
| 39 | move, rotate, select without dragging (keys on the selection, click-to-place moves) | S22 2.5.7 | MUST (AA) | existing ops `move`, `rotate` | S |
| 40 | one-key shortcuts: off switch and remap, active only with canvas focus | S22 2.1.4 | MUST (A) | settings section `shortcuts` | M |
| 41 | page audit: target size 24 px, reflow 320 px, text spacing, focus not obscured, orientation, non-text contrast, forced colours, reduced motion | S22, S25 | MUST | e2e only | S |
| 42 | toolbar as the APG toolbar pattern (one tab stop, arrows inside) | S26 | SHOULD | page only | S |
| 43 | pass with a real screen reader | S22 | SHOULD (cannot run here) | - | S |
| 44 | conformance report `std/1` on the page (list, click to select the atoms) | this study | MUST (the checks must be seen) | `"out":"std"` | S |
| 45 | molfile, SDF, KET, CDXML, CML, MRV, RXN in; SVG, PNG out (built) | S9-S14, S18, S20 | MUST (built) | `import`, `out` | - |
| 46 | CDX binary read | S10 | LATER | `import` | L |
| 47 | Haworth, chair, Fischer templates | S2 `*` | LATER (class drawings, not for software reading) | templates | M |
| 48 | macromolecules, query features, S-groups other than abbreviations | `CLAUDE.md` | OUT (scope) | listed on import | - |
| 49 | EMF / OLE clipboard | `mzdraw-formats.md` §1 | OUT of the page; LATER in the desktop app | - | L |
Counts (rows 1-49, a row with two words counts under its first): MUST 18 (4 of them built: rows 1, 3, 9, 45; 14 to build), SHOULD 20, LATER 8, OUT 3.

## 6. Proposed DEV slices (prompts)
One small slice each, in the order a coordinator can run them; none repeats a built feature. Each prompt is self-contained (`docs/prompts/arch-std-NN-slug.md`).
| NN | slug | rows | priority |
|---|---|---|---|
| 01 | std-report-stereo | 11, 44 | MUST |
| 02 | std-report-geometry | 16 | MUST |
| 03 | palette-contrast | 33 | MUST |
| 04 | svg-accessible | 27, 28 | MUST |
| 05 | drag-free-editing | 39 | MUST |
| 06 | shortcut-settings | 40 | MUST |
| 07 | page-a11y-audit | 41 | MUST |
| 08 | label-reversal | 5 | MUST |
| 09 | opensmiles-files | 19 | MUST |
| 10 | label-display-modes | 6, 10 | SHOULD |
| 11 | unknown-double-bond | 12 | SHOULD |
| 12 | enhanced-stereo | 13 | SHOULD |
| 13 | crossing-gap | 4 | SHOULD |
| 14 | ann-scales | 35 | SHOULD |
Not prompted here: rows that belong to other studies (arrow kinds, text runs, clean-up wiring, PDF, CIP, InChI), rows waiting for a source (31, 32), and rows rated LATER.
