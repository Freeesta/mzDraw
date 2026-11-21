# mzDraw: guidelines and standards (ARCH study `arch-std`)

Report only: no code depends on this file. It inventories the public guidelines and specifications a molecule editor that wants to be better than Ketcher must follow (structure-drawing rules, stereo drawing, journal artwork rules, file formats, identification-confidence levels, accessibility, colour), turns them into presets with numbers, a conformance checklist, the list of checks the page must run, and one feature table with priorities. The proposed DEV slices are the prompts `docs/prompts/arch-std-NN-*.md`.
Inputs in this repository: `CLAUDE.md`, `docs/mzdraw.md` ("Built state" = what exists), `docs/mzdraw-plan.md`, `docs/mzdraw-competitors.md`, `docs/mzdraw-formats.md`, `docs/mzdraw-annotations.md`.

Rules of this study: every fact carries its source (URL + version/date); everything is summarised in our own words, no text or artwork is copied; **`*` marks a fact that could not be read at a primary source** during this study (blocked host, login, paywall, request form): it comes from our general knowledge or from a secondary page and must be confirmed before code depends on it. A source behind a login, paywall or form is "not read", never guessed. Nothing here claims fidelity to any product. Read on 2026-11-20 from the cloud container of this study.

## 0. What could and could not be read
First pass (2026-11-20, morning): the network allowed a fixed list of hosts. Second pass (2026-11-20, evening): the environment was opened by the owner. Hosts that answer automated clients with a challenge page (HTTP 403 "Just a moment") stay **not read**: no challenge was bypassed. Not read for that reason: `www.iupac.org`, `pubs.acs.org` (the ACS guidelines were read on `researcher-resources.acs.org` instead), `support.revvitysignals.com`, `www.rsc.org`, `onlinelibrary.wiley.com/page/...` (Angewandte) and `advanced.onlinelibrary.wiley.com`, `help.benchling.com`. Read in the second pass: the two IUPAC recommendations (HTML editions on `iupac.qmul.ac.uk`, which state they are identical to the published papers), the ACS JOC author guidelines, the Elsevier artwork pages, the Nature submission pages, the Wiley figure guide, Microsoft and WebKit pages on SVG. Facts that came only from search-engine extracts of a page are marked `*` with the note "extract".

## 1. Source register
| id | source | version / date | URL | read? |
|---|---|---|---|---|
| S1 | IUPAC, Brecher, "Graphical representation standards for chemical structure diagrams" (IUPAC Recommendations 2008), Pure Appl. Chem. 80(2), 277-410, DOI 10.1351/pac200880020277 | 2008 | HTML edition https://iupac.qmul.ac.uk/drawing/drawing.html (GR-0 to GR-13, Tables I-III) | read (second pass); figures are images, so grades are taken from the text only |
| S2 | IUPAC, Brecher, "Graphical representation of stereochemical configuration" (IUPAC Recommendations 2006), Pure Appl. Chem. 78(10), 1897-1970, DOI 10.1351/pac200678101897 | 2006 | HTML edition https://iupac.qmul.ac.uk/drawing/stereo.html (ST-0 to ST-6) | read (second pass); figures are images |
| S3 | IUPAC Blue Book 2013, chapter P-9 (configuration and conformation; points to S2 for drawings) | 2013 | https://iupac.qmul.ac.uk/BlueBook/P9.html | **not read** `*` |
| S4 | ACS author guidelines, J. Org. Chem., "Appendix 2: Preparing Graphics"; the "ACS Document 1996" ChemDraw settings | JOC PDF "Last updated October 08, 2026"; Document 1996 numbers undated | https://researcher-resources.acs.org/publish/author_guidelines/pdf?coden=joceah | JOC graphics appendix read; the Document 1996 numbers are not in it and stay `*` (pubs.acs.org is a bot wall) |
| S5 | RSC journal pages (Chemical Science, RSC Advances, ChemComm) and book guide | undated | https://www.rsc.org/publishing/publish-with-us/publish-a-journal-article/chemical-science | **not read** (bot wall); numbers below are search extracts `*` |
| S6 | Elsevier artwork and media instructions: overview, types, sizing | undated, read 2026-11-20 | https://www.elsevier.com/about/policies-and-standards/author/artwork-and-media-instructions/artwork-sizing (and `/artwork-types`, `/artwork-overview`) | read |
| S7 | Wiley "Guidelines for the Preparation of Figures"; Angewandte graphics guidelines; Advanced Materials ChemDraw settings | figure guide "Updated March 2025"; journal pages undated | https://onlinelibrary.wiley.com/pb-assets/assets/20500904/AG/Electronic-Artwork-Guidelines-CCR3-1741401216880.pdf ; https://onlinelibrary.wiley.com/page/journal/15213773/homepage/graphics-guidelines | figure guide read; journal pages **not read** (bot wall), extracts `*` |
| S8 | Nature final submission and initial submission pages; Springer journal instructions | undated, read 2026-11-20 | https://www.nature.com/nature/for-authors/final-submission ; https://www.nature.com/nature/for-authors/initial-submission | Nature pages read; Springer journal PDFs are search extracts `*` |
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
| S32 | Microsoft 365 "Edit SVG images in Microsoft 365" (applies to Word, PowerPoint, Excel, Outlook for Microsoft 365) | undated, read 2026-11-20 | https://support.microsoft.com/en-us/office/edit-svg-images-in-microsoft-365-69f29d39-194a-4072-8c35-dbe5e7ea528c | read |
| S33 | Microsoft Edge blog, SVG in the Async Clipboard API from Edge 124 | 2024-07-11 | https://blogs.windows.com/msedgedev/2024/07/11/seamless-svg-copy-paste-on-the-web/ | read |
| S34 | WebKit blog, Safari Technology Preview 250 (SVG clipboard read/write, sanitised) | 2026* | https://webkit.org/blog/18191/ | read |
| S35 | mzLab `main` at `76a4703` (2026-11-21): the draw route `mzlab-feat/src/chem/draw*.rs` (SVG, PNG, KET and CDXML writers, CIP marks) and `mzlab-mol/src/layout/` (`layout_2d`, `layout_2d_fixed`, `clean.rs`) | 2026-11-21 | https://github.com/Freeesta/mzlab | read (module headers and public functions) |

## 2. Guideline inventory

### 2.1 Structure diagrams (S1, IUPAC 2008, read)
Rule numbers are those of S1 (GR-x). Grades are given where the text attaches one; the example drawings are images and were not read.
- **Tolerances**: an angle stated in the rules means about ±10°; "collinear" means 170-190° (GR-0.1). Substituents snap to horizontal, vertical or multiples of 30° when within about 5° (GR-4.2.1; GR-4.2.2 says 6°).
- **Text and lines**: printed labels are usually 8-14 pt and below 5 pt is not acceptable (GR-0.3); lines thinner than 0.5 pt are to be avoided and should be close to the stroke width of the label font (GR-0.4); a bond is about as wide as the stroke of a capital H of the label font, never more than 4 times or less than a quarter of it (GR-1.2); labels scale with the drawing (GR-0.6). One plain roman font and style per diagram, black on white by default; colour by element follows the CPK convention (O red, N blue); red is for emphasis and red with green is to be avoided (GR-0.3, GR-0.5).
- **Bonds**: one consistent bond length; atoms closer than twice the label height are to be avoided except in bridged or crowded systems (GR-1.1). Hashed, dashed and wavy bonds: gaps of 2-4 bond widths, at least 3 hashes or half-waves per bond, one pattern per diagram (GR-1.3). The lines of a multiple bond are not further apart than 33 % of the bond length (GR-1.6). A dotted partial bond has at least 3 dots and is at least twice the label height long (GR-1.8).
- **Double bonds** (GR-1.10): one line on the atom-atom axis, the second offset and shortened at both ends to the bisector of the angle with the neighbouring bond (not shortened at an end without substituents). Offset towards the side with more substituents; centred for a terminal `C=X` (C=O, CH2=); a trans bond with one substituent per end is offset (towards the ring centre in a ring); in an alternating six-membered ring all three double bonds lie inside the ring (preferred). Isolated ring double bonds go left, then top; a symmetric molecule gets a symmetric Kekulé pattern (GR-3.5).
- **Geometry**: chains are a 120° trans zigzag, horizontal as a whole, a doubly bonded branch pointing up, branches bending away from the centre (GR-3.2, GR-4.1). Atoms with three bonds: 120°. Atoms with four bonds (GR-4.1.3): 60/90/120/90 with the wedge and hash on the two bonds 60° apart when two neighbours are terminal; four 90° angles when every or no neighbour is terminal or for two double bonds (sulfonyl); **four bonds on the axes at a stereocentre are not acceptable** (they read as a Fischer projection). Five or more bonds: one vertical, the others evenly spread (GR-4.1.4). Cumulated double bonds and a single bond next to a triple bond are drawn at 180°, and such a carbon is labelled (GR-2.1.2, GR-4.1). Rings of up to 8 atoms are regular polygons; larger rings are puckered and keep the cis/trans geometry of their double bonds (GR-3.3). Orientation: main axis horizontal, principal groups and acyclic heteroatoms to the right, principal ring towards the lower left (GR-3.1); ring orientations by size (GR-3.4.2). A single ring substituent bisects the larger outer angle; two on one atom are 60° apart (90° on a four-membered ring); spiro ring bisectors are collinear (GR-4.2).
- **Overlap** (GR-4.3): exact overlap of atoms or bonds is never acceptable. The rule lists the fixes in order (flip a small substituent, rotate a chain bond but never a stereogenic double bond, swap neighbours without changing the configuration, bend or stretch at spiro centres, change ring perspective, lengthen bonds, change a substituent angle, move a small substituent between large ones).
- **Labels** (GR-2): proper case; H counts as subscripts after the symbol, before it when the bond attaches on the right of the label; a label without H means no H (implicit H only on unlabelled carbon). Carbon is unlabelled with two or more bonds and must be labelled when isolated, with collinear bonds, or in one-bond drawings; CH3 and Me are not mixed in one diagram; "C" alone for a methyl is wrong; terminal carbons are labelled in fragments of larger structures (GR-2.1.2). Isotopes as a left superscript mass number, never on an unlabelled carbon; D and T allowed but not mixed with other isotopes (GR-2.1.3). Bonds aim at the centre of the bonded atom's symbol (its first letter, or last letter of a reversed label), and the symbol centre stands where the unlabelled bond would end (GR-2.1.5). **A label of several symbols with bonds only on its right is reversed** (`HO`, `HO2C`); counts and charges stay right of their own symbol (GR-2.1.6). Multi-line labels stack away from a vertical bond (GR-2.1.7). Abbreviations usable without definition: Me, Et, Pr, iPr, Bu, iBu, s-Bu, t-Bu, Ac, Ph, Ms, Ts, Cp (Cp only on a metal); others are defined in the figure; new abbreviations that look like element symbols are not acceptable; Bz is to be avoided (GR-2.2). Contracted labels: at most two bonds, on the first and last character, branches in parentheses, H first then elements alphabetically then abbreviations (GR-2.3).
- **Charges and radicals** (GR-5): the charge after the symbol and its hydrogens, top right preferred, magnitude before the sign (`2+`) only above 1, an en dash for minus; circled charges acceptable for single charges only; never in the middle of a label; atoms are not moved to make room for a charge. A radical is a superscript dot after the symbol and its H; several radicals by a count or well-separated dots; lone pairs exactly above, below, left or right of the label, never as a bar. Ions of a salt are separate fragments about one bond length apart, never joined by a bond (GR-7).
- **Aromatic circle** (GR-6): centred, solid, one bond width thick, at least one bond width from the bonds and at most twice the double-bond spacing; only for delocalisation being emphasised; Kekulé bonds preferred for furan and pyrrole; circles in two adjacent fused rings are not acceptable.
- **Annotations** (GR-11): atom annotations about half a capital height from their atom and nearer to it than to any other; structure annotations 0.5 to 2 bond lengths away, usually below.
- **Crossings**: a crossing shows the front bond by a gap in the rear bond of at least twice the bond width (S2 ST-3.1); without the gap it reads as an atom.

### 2.2 Stereo drawing (S2, IUPAC 2006, read; S17, InChI Technical Manual, read)
From S17 (read, §III.d and the tables of the stereo layer):
- InChI reads wedges by default as "the narrow end of a wedge is at the stereocentre": a wedge acts on one atom only. An option (`/NEWPSOFF`) reads them in "perspective", acting on both atoms of the bond. The two readings give different identifiers for the same picture when a wedge joins two stereocentres. **Consequence for mzDraw**: a wedge whose wide end is also a stereocentre is ambiguous between programs; the editor must warn about it.
- For the parity of a centre, InChI lifts the wide end of a solid wedge 45° above the plane and puts the wide end of a hashed wedge 45° below, after setting every bond to the same length; a centre outside the tetrahedron of its neighbours gives a warning. **Consequence**: the check can use the same construction to detect drawings whose configuration depends on bond lengths or crosses the centre.
- 3D coordinates override wedges; a wavy ("either") bond always means unknown.
- Atoms InChI treats as possible tetrahedral centres: C, Si, Ge, N+, P, P+, As, B-, Sn and others listed in its Table 8; possible stereo double bonds involve C, Si, Ge and N (with charged forms listed in its Table 7). A double bond in a system without a complete alternating pattern (radicals, some zwitterions) is not stereogenic for InChI.
- Allenes: the configuration must be shown by two wedges at the same end of the allene with opposite marks (a stated limitation of the InChI software).
- Absolute versus relative stereo: InChI's `/s1` (absolute) and `/s2` (relative); the molfile chiral flag selects between them under the option `/SUCF`. The unreleased v2026.1 adds an option reading the V3000 enhanced-stereo collections (`STEABS`, `STEREL`, `STERAC`) as a beta feature.
From S2 (read; ST-x rule numbers):
- **Marks** (ST-0): plain bonds wherever possible, ring bonds plain and the wedge on an acyclic substituent (except spiro centres, or when every acyclic neighbour is itself a stereocentre). The narrow end is always at the stereocentre; a solid wedge comes towards the viewer, a hashed wedge goes behind; the hashed wedge is the only preferred "down" mark; parallel hashed lines (or a reversed hashed wedge) are acceptable only towards a terminal or near-terminal atom and never between two stereocentres; one hash style per diagram; a bold bond is not acceptable for configuration (it is for perspective); dashed lines are for partial bonds only. A wavy bond means unknown configuration; usually a plain bond is preferred and the wavy bond is for emphasis; it also marks an unknown configuration at a neighbouring double bond. **Wedges between two stereocentres are to be avoided**; if unavoidable, solid wedges only (they act on the narrow end). Asterisks are to be avoided; CIP labels never replace wedges (ST-0.6, ST-0.7).
- **Four explicit bonds** (ST-1.1, twenty patterns graded): preferred are two plain bonds about 120° apart with the wedge and hash between them about 60° apart, or three plain bonds about 120° apart with one wedge (best on ring atoms); alternating bonds at 90° are acceptable; a wedge inside a gap of 180° or more is not acceptable except in small fused rings; patterns that depict see-saw, square-pyramidal or square-planar geometry are wrong; the graphically larger substituent goes on the right.
- **Three explicit bonds** (ST-1.2): an implicit H is allowed only when the atom at the wide end has at most two non-H neighbours (draw the H when the wide-end atom is a stereocentre); preferred: two plain bonds about 120° apart and the wedge bisecting the angle opposite them; a wedge in a re-entrant ring angle is not acceptable (ST-1.2.3, ST-1.4); trigonal or T-shaped patterns are not acceptable.
- **Rings, bridges, spiro** (ST-1.3): one exocyclic substituent as a wedge bisecting the larger angle; two as wedge plus hash about 60° apart; at fusion atoms the wedge goes on the exocyclic bond (explicit H if needed); bridgeheads wedge the bridge bonds; spiro centres put wedge and hash in one ring, preferably the smaller one.
- **Allenes** (ST-1.6): stereo bonds at one end only, all four substituents drawn (implicit H acceptable). **Atropisomers** (ST-1.7): one ring all plain, the other with a wedge at the axis bond and on the opposite bond, the bond between them bold; one wedge at the axis is acceptable; wedges far from the axis are not.
- **Projections** (ST-1.8 to ST-1.11, ST-3): Fischer projections only for carbohydrates, never horizontal; Haworth projections with a bold bottom ring bond; both are to be avoided for software; flat (Mills) drawings are preferred. Perspective drawings only for bridged rings, viewed from above right, rear bonds gapped at crossings, depth emphasised by solid or bold bonds, never by hashes.
- **Double bonds** (ST-4): plain bonds at about 120°, geometry as drawn; double bonds in rings of 6 or fewer atoms are cis. **Unknown E/Z**: drawn linear (preferred, only with one substituent per end) or with a wavy bond to a substituent (preferred, never towards another stereocentre); **the crossed double bond is not acceptable** except where software requires it (ST-4.4).
- **Absolute and relative** (ST-6): no marks at all means unknown configuration; wedges at every centre mean one stereoisomer; mixtures are said in words under the drawing ("and enantiomer", ratios left to right); a "+" separator and the words "rac", "racemate", "rel", "relative" as labels on drawings are not acceptable; the chiral-flag behaviour of MDL files is explicitly not followed. The documents do not mention group labels such as `&1` or `or1`; they note that there is no consensus on depicting relative stereo groups (ST-0.8).
- **Non-tetrahedral centres** (ST-2): recipes for each geometry (square planar, trigonal bipyramidal, octahedral, ...); solid wedges may be up to 150 % of the length of hashed ones (ST-2.7, ST-2.12).

### 2.3 Journal style sheets and artwork rules (S4-S8)
| publisher | widths | depth | text at print size | lines | resolution (line / grey / colour) | formats | structures | source, read? |
|---|---|---|---|---|---|---|---|---|
| ACS (J. Org. Chem.) | one column up to 240 pt (3.33 in); two columns 300-504 pt (4.167-7 in) | 660 pt with caption (12 pt per caption line) | lettering at least 4.5 pt; Helvetica or Arial | at least 0.5 pt | 1200 / 600 / 300 dpi | per journal* | drawn with a program "such as ChemDraw"; TOC graphic at most 8.25 × 4.45 cm | S4 JOC PDF of 2026-10-08, read |
| ACS colour rule | - | - | - | - | - | - | do not rely on colour alone; contrast 4.5:1 for text and 3:1 for other elements, citing WCAG | same, read |
| ACS "Document 1996" | - | - | Arial 10 pt | line 0.6 pt, bold 2.0 pt | - | - | fixed bond 14.4 pt, bond spacing 18 %, margin 1.6 pt, hash 2.5 pt | `*` (secondary: RDKit, ChemDraw help; extract of an ACS guide repeats line, bold, length, spacing, margin) |
| Elsevier | minimum 30 mm; single 90 mm; 1.5 columns 140 mm; double 190 mm | - | 7 pt normal text, at least 6 pt for sub- and superscripts; Arial/Helvetica, Courier, Symbol, Times; fonts embedded | 0.10 to 1.5 pt (vector artwork) | line art 1000 dpi (1200 for fine lines) / halftone 300 / combination 500 | EPS (preferred for vector), PDF, TIFF, JPEG; Office files accepted | nothing specific | S6, read |
| Wiley (general figure guide) | small 80 mm, large 180 mm, or at least 1800 px wide | - | - | - | line art 600 dpi / images 300 dpi | EPS, PDF, TIFF, PNG | - | S7 PDF "Updated March 2025", read |
| Wiley journals | Angewandte: 8.4 / 17.8 cm*; Chemistry Europe asks structures as `.cdx` with the journal template* | - | Chemistry Europe checklist 2018: 1.8 mm text height* | - | - | - | Advanced Materials ChemDraw settings: bond 17 pt, line 0.75 pt, bold 2.6 pt, hash 2.6 pt, spacing 18 %, margin 2 pt, Arial 12 pt* | extracts `*` (bot wall) |
| Nature | single 89 mm; 1.5 columns 120-136 mm; double 183 mm | 247 mm | sans-serif (Helvetica or Arial), one font; initial submission 5-7 pt | 0.25 to 1 pt at final size | photos 300-600 dpi | final: AI, EPS, PDF, PostScript for line art; initial: `.ai .eps .pdf .ps .svg` as editable vector; no raw ChemDraw at final | ChemDraw converted to vector | S8, read |
| RSC | single 8.3 cm, double 17.1 cm, depth 23.3 cm* | - | - | - | TIFF at 600 dpi or more* | TIFF; EPS or PDF converted to TIFF* | Chemical Science ChemDraw settings: chain 120°, bond 0.43 cm (12.2 pt), line 0.5 pt, bold 1.6 pt, spacing 20 %; book guide adds hash 1.8 pt* | extracts `*` (bot wall) |
| Springer journals | 39 / 84 / 129 / 174 mm, depth 234 mm* | - | Helvetica or Arial 8-12 pt* | at least 0.1 mm (0.3 pt)* | 1200 / 300 / 600 dpi* | EPS, TIFF* | Monatshefte offers ChemDraw templates* | extracts `*` |
Rules for mzDraw from this table:
- The built `acs-1996` preset satisfies the read ACS JOC rules (lines 0.6 pt ≥ 0.5 pt; text 10 pt ≥ 4.5 pt) and the read IUPAC limits (spacing 18 % ≤ 33 %; lines ≥ 0.5 pt).
- No preset is shipped from an extract. RSC, Wiley-journal and Springer settings wait for a page that can be read; the coordinator can ask the owner to save the pages by hand.
- **Export profiles** can be built now from the read rows (ACS JOC, Elsevier, Nature, Wiley general): width classes, maximum depth, minimum text and line at print size, accepted formats. The export check reports a drawing that, scaled to the chosen width, puts text or lines under the minimum (prompt 16).
- **SVG is accepted only by Nature's initial submission** among the pages read; every publisher read accepts PDF or EPS for vector art. So a journal route needs PDF (prompt 18); EPS is not proposed (Office turned EPS off, and PDF is accepted everywhere EPS is).

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
IUPAC 2008 sets the frame (S1 GR-0.5, read): black on white by default, colour by element following the CPK convention, red for emphasis, red with green avoided; the ACS JOC guide (S4, read) asks authors not to rely on colour alone and to meet 4.5:1 for text and 3:1 for other elements. On white only N, Br and I reach the 4.5:1 that small label text needs (1.4.3); O, P, S, F, Cl fail; on a dark ground N, Br and I fail instead. **No single set of colours serves both a light and a dark background at 4.5:1** (a colour needs relative luminance ≤ 0.183 for white and ≥ 0.204 for #121212). So mzDraw needs one palette per background, chosen for contrast first; the hue only follows the usual convention (N blue, O red, S yellow-brown, P orange, halogens green, I violet).
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

### 2.9 Export: the default download and its fallbacks
Owner requirement (via the coordinator): the page downloads in every format it writes, and the default is the canvas image on a **white background**.
Evidence (read unless marked):
- **Office desktop**: Word, PowerPoint, Excel and Outlook for Microsoft 365 insert and edit SVG and can turn it into Office shapes (S32, read). PowerPoint for the web lists BMP, GIF, JPEG, PNG, TIFF and WMF/EMF and not SVG (Microsoft support page, extract `*`). EPS insertion is turned off in Office (Microsoft lifecycle page, extract `*`). Text inside an SVG in Office is reported fragile (CSS and `@font-face` ignored, a misalignment with `text-anchor="end"`): community answers only `*`.
- **Google Docs and Slides**: images inserted through their APIs must be PNG, JPEG or GIF, under 50 MB and 25 megapixels (Google developer pages, extract `*`); SVG is not insertable (community answer `*`).
- **LibreOffice** 24.2 writes SVG with a PNG fallback into OOXML (release notes, extract `*`); **Inkscape** opens SVG natively and imports PDF with text kept as text by default; **Illustrator** opens PDF and SVG (vendor pages, extracts `*`).
- **Publishers** (§2.3, read): vector art as PDF or EPS everywhere; SVG only at Nature's initial submission; raster TIFF/PNG/JPEG at 300-1200 dpi.
- **ELNs**: they import molfile/SDF (Benchling, LabArchives, Signals) and CDX/CDXML (Signals, LabArchives); images as PNG, JPEG, TIFF, PDF attachments; none was found that imports a structure from an image's metadata (vendor pages, extracts `*`; help.benchling.com is a bot wall). eLabFTW and Chemotion embed Ketcher, so KET pastes there.
- **Browser clipboard**: `image/png` is one of the mandatory types of the Clipboard API (S25); `image/svg+xml` in `write()` and `read()` from Edge 124 (S33, read; same Chromium code in Chrome*), in Safari only in Technology Preview 250 (S34, read), not in Firefox `*`.
- **Sizes measured for this study** (the built writers, release build of `examples/replay` on `main` at `e16bdb1`; PNG drawn by Chromium from the SVG at 300 pixels per inch on white, as the page does; structures laid out by `mzlab-mol`):
| structure | SVG `acs-1996` (gzip) | SVG `mzlab` (gzip) | PNG 300 dpi `acs-1996` | PNG 300 dpi `mzlab` | `mzdraw/1` | molfile | KET | CDXML |
|---|---|---|---|---|---|---|---|---|
| caffeine (14 heavy atoms) | 3,602 B (991) | 3,675 B (1,009) | 19,784 B, 448 × 447 px | 47,653 B, 815 × 814 px | 1,049 B | 1,388 B | 1,072 B | 1,566 B |
| ibuprofen (15) | 3,236 B (817) | 3,299 B (803) | 19,533 B | 46,622 B | 1,043 B | 1,458 B | 1,067 B | 1,300 B |
| paclitaxel (62) | 14,491 B (2,872) | 14,738 B (2,921) | 82,598 B, 993 × 674 px | 205,514 B, 1952 × 1287 px | 4,646 B | 5,914 B | 4,572 B | 5,301 B |
The SVG sizes include the carried `mzdraw/1` document; the PNG sizes do not include its `iTXt` chunk (about the size of the document). The saved SVG has **no background**: today it is transparent, so a white background needs a `<rect>` first (prompt 17).
**Recommendation**:
1. **Default download = SVG on white**: a white `<rect>` under the drawing, text as text in `Arial, Helvetica, sans-serif`, labels placed with explicit `x` (no `text-anchor="end"`), the `mzdraw/1` document in `metadata` (built) so the file reopens in mzDraw. It is vector at any size, 5 to 14 times smaller than the PNG at 300 dpi, opens in Office desktop, LibreOffice, Inkscape, Illustrator and browsers, and is the one picture the page and the CLI give with the same bytes (rule 10).
2. **PNG at 300 dpi on white** (built route, plus the white ground made explicit) is the fallback for Google Docs/Slides, PowerPoint for the web, ELN attachments, chat and email, and is always on the clipboard (`image/png`).
3. **PDF** (vector, base-14 Helvetica, text as text, the document attached) is the journal route: every publisher read accepts PDF for vector art (prompt 18).
4. **Clipboard on Copy**: `image/png` + `text/plain` molfile always; `image/svg+xml` where `ClipboardItem.supports()` says yes; the web custom formats in Chromium (built for molfile and the document). The payloads are made in Rust; JS hands them to the browser.
5. **Structure files for ELNs**: one click to molfile (V2000; V3000 above 999 atoms), SDF and CDXML, because ELNs import those and not pictures.
Not proposed: EPS (turned off in Office, PDF covers the publishers), EMF (Windows only; its better text handling in Office is reported only by community answers `*`; LATER in the desktop app).

### 2.10 Where each format lives, and what overlaps with mzLab today
Rule asked by the coordinator (COO 16): the draw route of mzLab must be pinned and called, not written twice. Read for this study (S35): on mzLab `main` at `76a4703` the draw route is in **`mzlab-feat`** (`src/chem/draw.rs`, `draw_svg.rs`, `draw_png.rs`, `draw_file.rs` with `ket()` and `cdxml()` writers, `draw_cip.rs`, `draw_label.rs`, `draw_font.rs`), not in `mzlab-mol`; it takes its own `Drawn` / `Figure` types (a structure laid out from a record), and it landed after the pin of mzDraw (`ccd4680`), so calling it needs a pin move (rule 9: a PR of its own). `mzlab-mol` holds the layout (`layout_2d`, `layout_2d_fixed`, `layout/clean.rs` with overlap removal), already at the pin.
Proposed placement:
- **(a) `mzlab-mol`** (shared with the CLI; PR through COO 16 / COO-DRAW): every format whose content is the chemistry of a record: molfile V2000/V3000, SDF, RXN/RDF, SMILES (with an aromatic "standard form" option), CXSMILES (a writer: today only the stereo-group tail is read), SMARTS (if ever), PDB/mmCIF ligands, XYZ, MOL2, CIF, InChI (outside the wasm); the 2D layout and clean-up; CIP perception.
- **(b) `crates/mzdraw` only**: what carries the drawing as drawn (positions as the user left them, wedges as drawn, arrows, plus signs, texts, objects, annotations, unknown fields): `mzdraw/1`, the editor's SVG/PNG/PDF from its display list, KET, CDXML/CDX, MRV, CML documents, the clipboard, the import report, the conformance report `std/1`.
- **Decision needed (COO 16 / owner)** where (a) and (b) meet: KET and CDXML exist twice and the depiction rules (label placement, double-bond side, wedges) exist twice. The mzDraw writers carry more (arrows, texts, R groups, drawn wedges, unknown fields, read back by its own readers); the mzLab writers carry one laid-out molecule. Two ways to have one writer per format: (1) `mzlab-cli mol draw` builds a `mzdraw/1` document and calls `mzdraw::replay` for KET, CDXML, SVG and PNG (mzLab depends on `mzdraw`; one writer, the richer one); (2) the mzDraw writers move into `mzlab-feat`/`mzlab-mol` and mzDraw calls them (`mzdraw` may then depend on `mzlab-feat`, which `CLAUDE.md` does not allow today). This study recommends (1): rule 10 of `CLAUDE.md` already makes the CLI replay the page's log.
Overlaps today, one line each (crates/mzdraw on `main` vs mzLab `main` `76a4703`):
1. SVG: `crates/mzdraw/src/svg.rs` + `scene.rs` (display list to SVG, carries the document) vs `mzlab-feat/src/chem/draw_svg.rs` (figure to SVG).
2. PNG: `crates/mzdraw/src/png.rs` (`finish`, `plan`: chunks, `iTXt`, `pHYs`; pixels from the browser canvas) vs `mzlab-feat/src/chem/draw_png.rs` (own rasteriser with a stroke font, stored zlib blocks).
3. KET writer: `crates/mzdraw/src/ket.rs` (document, arrows, texts, R groups; plus a reader) vs `mzlab-feat/src/chem/draw_file.rs::ket` (one molecule, writer only).
4. CDXML writer: `crates/mzdraw/src/cdxml.rs` + `xml.rs` (plus a reader) vs `mzlab-feat/src/chem/draw_file.rs::cdxml` (writer only).
5. Label rules (hydrogens left or right, `H2N`, charges): `crates/mzdraw/src/scene.rs` vs `mzlab-feat/src/chem/draw_svg.rs` / `draw_label.rs`.
6. Double-bond side and wedge drawing: `scene.rs::double_side` and wedge painter vs `draw_svg.rs`.
7. Text metrics: the advance-width tables of mzDraw vs `draw_font.rs` (stroke font, PNG only).
8. Clean-up: `crates/mzdraw/src/layout.rs::clean_up` (stub, unchanged drawing) vs `mzlab-mol::layout::{layout_2d_fixed, clean}` (at the pin): wire it, write nothing (prompt 15).
9. SDF data fields: `crates/mzdraw/src/io.rs` (fields on the first atom, writer and reader) vs `mzlab-feat` `sdf/1` (feature `check`): two SDF writers.
10. SMILES tidy-up after `mzlab-mol` writes (brackets a reader does not need removed, `CC(=O)O`): in `crates/mzdraw`; belongs in the SMILES writer of `mzlab-mol`.
11. CIP: none in `crates/mzdraw`; `mzlab-feat/src/chem/draw_cip.rs` places R/S, E/Z marks: call it through the decision above, do not write a third.
12. RXN/RDF, CML/MRV readers and the XML reader: only in `crates/mzdraw` (`rxn.rs`, `cml.rs`, `xml.rs`); by the rule above RXN/RDF belong in `mzlab-mol` (move, not copy), CML/MRV stay.

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
| `std.stereo.allene` | an allene with stereo marks that are not two opposite wedges at one end | warn | S17, S2 ST-1.6 | live |
| `std.stereo.fischerCross` | four bonds within 10° of the axes at a stereocentre (reads as a Fischer projection) | warn | S1 GR-4.1.3 | live |
| `std.stereo.reentrant` | a wedge inside a re-entrant ring angle | warn | S2 ST-1.2.3, ST-1.4 | live |
| `std.stereo.ringBond` | a wedge on a ring bond while an acyclic neighbour could carry it | info | S2 ST-0.2 | live |
| `std.stereo.hiddenH` | an implicit H at a centre whose wide-end atom is a stereocentre | warn | S2 ST-1.2 | live |
| `std.stereo.relLabel` | a text next to a structure reading rac, rel, racemate or relative | info | S2 ST-6.5 | live |
| `std.stereo.crossedDouble` | a double bond of unknown geometry drawn crossed | info | S2 ST-4.4 | live |
| `std.geom.atomClash` | two atoms closer than 0.3 bond length (exact overlap is never acceptable, GR-4.3) | error | S1 GR-4.3 | live |
| `std.geom.atomOnBond` | an atom within 0.15 bond length of a bond it is not on | error | S1 GR-4.3 | live |
| `std.geom.crossing` | two bonds that cross (no shared atom), unless the rear one is gapped | warn | S2 ST-3.1 | live |
| `std.geom.length` | a bond whose length differs from the median of its structure by more than 15 % | info | S1 GR-1.1 | export |
| `std.geom.labelClash` | two labels whose boxes overlap, or a label over a bond | warn | S1 GR-2.1.5, GR-4.3 | export |
| `std.label.mixedMethyl` | `CH3` and `Me` both used in one drawing | info | S1 GR-2.1.2 | export |
| `std.label.lookalike` | a user abbreviation that is also an element symbol | warn | S1 GR-2.2 | live |
| `std.label.chargeHidden` | a charge that cannot take the upper right (crowded) | info | S1 GR-5.1 | export |
| `std.style.limits` | a style with lines under 0.5 pt, text under 5 pt, multiple-bond spacing over 33 % or fewer than 3 hashes per bond | warn | S1 GR-0.3, GR-0.4, GR-1.3, GR-1.6 | on style change |
| `std.ann.noStructure` | an annotation at level 4 or 5 (or a sub-level of them) on a full structure | info | S27 `*`, S28 | live |
| `std.access.contrast` | a colour of the style below 4.5:1 (labels) or 3:1 (bonds, marks) on the background (the ACS JOC guide asks the same, citing WCAG) | warn | S22 1.4.3 / 1.4.11, S4 | on style change |
| `std.export.font` | a font outside the profile's list | warn | S4, S6, S8 (read) | export |
| `std.export.text` | text or sub/superscripts under the profile's minimum at print size | warn | S4, S6, S8 | export |
| `std.export.lineWidth` | a line thinner than the profile's minimum at print size | warn | S4, S6, S8 | export |
| `std.export.size` | a figure wider than the width class or deeper than the page | info | S4, S6, S7, S8 | export |
Built checks that stay as they are: valence and charge (`check/1`), import report (`Imported::notes`), annotation shape (`error.draw2.ann`), SDF field limits (`error.draw2.data`). The 10° and 170-190° tolerances are IUPAC's (GR-0.1, ST-0.1); the other thresholds (0.3, 0.15, 15 %) are ours, chosen to flag only clear cases; the DEV slices pin them in tests and may tune them with a reason.

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
