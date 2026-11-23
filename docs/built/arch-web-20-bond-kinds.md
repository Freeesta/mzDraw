# Bond kinds: bold, dative, hydrogen bond (arch-web-20)

Study rows of `docs/study/arch-web.md` §4: "bond kinds: bold, dative / coordinate, hydrogen bond" (built). Code: `crates/mzdraw/src/bonddisp.rs` (`KINDS`, `single_only`), `ops.rs` (`Op::BondSet`), `scene.rs` (`Painter::bond`, `Painter::seg`), `formats.rs` (V3000 out, `prepare` in), `ket.rs`, `cdxml.rs`, `io.rs` (`Imported::links`, `labelled`), `loss.rs`, `editor.rs` / `editor_tools.rs` (the tools), `context.rs` (the menu). This note adds three values to the field `k` of `docs/built/bond-display.md`; where that note says "`bold`, `dative`, `hbond` (arch-web-20)" are not built, this one is true.

## The kinds (field `k` of a bond, same reader and writer as `dash`, `bdash`, `hash`)

- `bold`: a heavy line (2.5 times the stroke, as the bold dashed one). Any order; every line of a double or triple bond is heavy.
- `dative`: an arrow from the donor, the atom `a` of the bond, to the acceptor, the atom `b`: a plain line that stops at the base of a filled triangle whose tip is at `b` (at the edge of a label when `b` has one).
- `hbond`: a dotted line (round dots, about one hash spacing apart).

`dative` and `hbond` are bonds of order 1 only (`bonddisp::single_only`): `bond.set` with either on a bond of another order is refused with `error.draw2.bond` and nothing changes; a new order other than 1 takes them away (as a new order other than 2 takes `pos` away); a wedge mark takes any kind away.

## The chemistry (read this before using the numbers)

- **Rule.** A dative or hydrogen bond is not valence of its ends. The hydrogens of an atom are derived from the covalent bonds alone (`Document::order_sum`, `order_sums`), so the donor and the acceptor keep the hydrogens of the atoms alone: `H3N->BH3` is `H6BN` (N 3 H, B 3 H), water joined to ammonia by a hydrogen bond is `H5NO`. The same bond drawn as a plain single bond would be `H4BN` (one hydrogen less on each end). Cutting such a bond in a selection closes nothing with hydrogen.
- **Not RDKit's convention.** RDKit counts a dative bond for the acceptor, not the donor, because its atoms carry explicit hydrogen counts. Here hydrogens are derived from valence, and counting the acceptor would give boron two hydrogens (`BH2`) for a drawn `N->B`; "neither end" gives the Lewis adduct. The cost: a dative arrow to a heteroatom with no other substituent (`P->O`) leaves it its implicit hydrogens (`OH2`): set the atom's hydrogens by hand, or draw it as it is drawn without the arrow.
- **The record.** `mzlab-mol` has no such bond, so in the `mol/1` record both are order-1 bonds (bond indices stay aligned with the document), with the hydrogens of the rule above on the atoms.
- **No mass, no descriptor for a structure that holds one.** The engine (`sel`) reads that record as a molfile and counts the hydrogens of the single-bond reading (water-ammonia: `H3NO`, 33.0215, where the drawing says `H5NO`). A number that disagrees with the formula is not written: the structure has its formula and the key `error.draw2.valuesLink` in `values/1` and in the panel, and the descriptors (`props`) give the same key; the other structures of the drawing are measured as before (`crates/mzdraw-wasm/tests/bond_kinds.rs`).
- **What the formats that cannot hold the kinds write** (V2000 molfile, SMILES, SDF, RXN, CML): a covalent single bond between the two things, with the hydrogens of the drawing and so one valence above the usual on the donor (`[NH3][BH3]`, V2000 atom lines with valence 4); a reader that derives hydrogens from bonds reads fewer hydrogens. The loss manifest says "no" for the kind; the text under its table in `docs/mzdraw-formats.md` says this.
- **Layout and records.** A dative or hydrogen bond joins its two pieces into one connected structure: per-structure values, the outline, the layout pieces and the records of an SDF treat the complex, or the two molecules held together, as one structure. That is a limit, stated here; the kinds do not split a structure.
- **A foreign file moved.** The fixture `tests/data/foreign.cdxml` holds `Order="dative"` from a ring carbon to a list atom (synthetic, chemically odd). It was read as a query bond (single): the ring carbon had no hydrogen left, formula `C12H9NR3+`. Now it is a dative bond that does not consume the carbon's valence: the carbon keeps its hydrogen, `C12H10NR3+`, the SMILES gains `=[CH](`, the V2000, SDF and RXN atom lines carry valence 5 on that carbon, the SVG shows the new formula and an arrowhead (`tests/data/log/cdx.json` re-blessed; the test `the_dative_bond_of_the_foreign_cdxml_leaves_its_donor_its_hydrogen` pins the formulas and the reason). This is the right chemistry for the rule above.

## The operation (`draw_edit/1`)

Unchanged in shape: `{"op":"bond.set","id":4,"k":"dative"}`, values `solid`, `dash`, `bdash`, `hash`, `bold`, `dative`, `hbond` (a value outside the list is refused at reading, `error.draw2.request`); `{"op":"bond.set","id":4,"rev":true}` turns a dative bond around (donor and acceptor swap). Inverse: `SetBond`, in the property test (`tests/ops.rs`, `random_step`: `k` now draws from all seven values and the empty one) and in `diff`. The schema text of `k` and its golden (`tests/data/log/edit-schema.json`) say the three values.

## The page

- Tools `bond:bold`, `bond:dative`, `bond:hbond` in the flyout of the bonds palette (icons, names and one-sentence titles in `web/`); on a bond they set the kind (a second click takes it away, a bond of another order becomes single for the last two), from an atom or on empty canvas they draw a single bond of that kind; a dative bond drawn from an atom or onto an existing bond starts at the atom pressed on. Mouse, keyboard (focus the palette entry, Enter, then the canvas), touch and pen run the same code.
- Context menu of a bond, group "Line style": Bold, Dative (arrow), Hydrogen bond (dotted); the last two are off for a bond that is not single; "Reverse direction" is also on for a dative bond.

## Files

Type numbers read from the Ketcher 3.21.0 source (`packages/ketcher-core/src/domain/entities/bond.ts`, `Bond.PATTERN.TYPE`: `DATIVE: 9`, `HYDROGEN: 10`; its `schema.json` allows bond `type` 1 to 12) and from the CTfile formats documentation (V3000 BOND block, `TYPE`: 9 coordination, 10 hydrogen; V2000 has types 1 to 8 only).

| format | dative | hydrogen | bold |
|---|---|---|---|
| V3000 molfile | type 9, donor first | type 10, first atom first | listed |
| V2000 molfile, SMILES, SDF, RXN, CML | listed (a single bond is written) | listed (a single bond is written) | listed |
| KET | bond `type` 9, `atoms` donor first | `type` 10 | listed |
| CDXML | `Order="dative"` begin atom = donor | `Order="hydrogen"` | `Display="Bold"` |
| SVG, mzDraw document | carried | carried | carried |

"Listed" is the loss manifest (`loss.rs`, rows `bond.bold`, `bond.dative`, `bond.hbond`, `docs/mzdraw-formats.md`). Reading: V3000 type 9 and 10, KET type 9 and 10 (they were query bonds before), CDXML `Order="dative"` / `"hydrogen"` (also from binary CDX, which has the names) and `Display="Bold"`; the first atom of the file's bond line is the donor, the bond is found by its two atoms (the reader of `mzlab-mol` may order bonds as it likes). The foreign CDXML and KET fixtures held a dative bond that was counted as a query bond: their expected notes changed (one query bond less) and the golden of the CDX file (`tests/data/log/cdx.json`) was blessed again for the same reason.

## Tests

`crates/mzdraw/tests/bond_kinds.rs` (the field comes and goes exactly, the hydrogens, formula, no-mass key and exact SMILES / V2000 of the two fixtures, the foreign file's formula, CDXML bold over a link, a V3000 reaction with links in two molecules, refusals change nothing, the log form, each kind's drawing against the plain one, the arrow's tip and the reverse, hydrogens of the ends, round trips of V3000, KET and CDXML with their direction, the manifest per format, the tools, the menu); golden parity `tests/data/log/bond-kinds.json` (two cases, losses, refusals); e2e steps "bond kinds" (desktop: tools, keyboard, menu, files, undo) and "tablet with touch and a pen: bold, dative and hydrogen bonds" in `tests/e2e.mjs`. Screenshots at deviceScaleFactor 2: `docs/screenshots/bond-kinds.png` (desktop), `docs/screenshots/bond-kinds-touch.png` (tablet 768x1024). There are no dark-mode tokens on `main` to test.

## Not built

- A bond type 9 or 10 in `mzlab-mol` (its `Bond` has an order only): see the chemistry section. With it the engine could count the hydrogens and the masses of these structures. Recommended for a later `mzlab` slice.
- Dative bond direction in SMILES (`->`, `<-`), CML, V2000: none.
- Bold, dative or hydrogen kinds from KET fields other than `type`, CDXML displays other than `Bold` (`Dot`, `Wedge...` are not read as kinds), "unknown" double bond (arch-std-11), haptic bonds, query bonds.
- A key to choose a kind (hover keys wait for arch-cd-03).
