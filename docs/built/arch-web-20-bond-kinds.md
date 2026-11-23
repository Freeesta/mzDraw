# Bond kinds: bold, dative, hydrogen bond (arch-web-20)

Study rows of `docs/study/arch-web.md` §4: "bond kinds: bold, dative / coordinate, hydrogen bond" (built). Code: `crates/mzdraw/src/bonddisp.rs` (`KINDS`, `single_only`), `ops.rs` (`Op::BondSet`), `scene.rs` (`Painter::bond`, `Painter::seg`), `formats.rs` (V3000 out, `prepare` in), `ket.rs`, `cdxml.rs`, `io.rs` (`Imported::links`, `labelled`), `loss.rs`, `editor.rs` / `editor_tools.rs` (the tools), `context.rs` (the menu). This note adds three values to the field `k` of `docs/built/bond-display.md`; where that note says "`bold`, `dative`, `hbond` (arch-web-20)" are not built, this one is true.

## The kinds (field `k` of a bond, same reader and writer as `dash`, `bdash`, `hash`)

- `bold`: a heavy line (2.5 times the stroke, as the bold dashed one). Any order; every line of a double or triple bond is heavy.
- `dative`: an arrow from the donor, the atom `a` of the bond, to the acceptor, the atom `b`: a plain line that stops at the base of a filled triangle whose tip is at `b` (at the edge of a label when `b` has one).
- `hbond`: a dotted line (round dots, about one hash spacing apart).

`dative` and `hbond` are bonds of order 1 only (`bonddisp::single_only`): `bond.set` with either on a bond of another order is refused with `error.draw2.bond` and nothing changes; a new order other than 1 takes them away (as a new order other than 2 takes `pos` away); a wedge mark takes any kind away. A dative or hydrogen bond takes no part in the valence of its ends: it does not count for the implicit hydrogens of N in `H3N->BH3` (`Document::order_sum`, `order_sums`), so the hydrogens drawn are those of the atoms alone. In the record handed to `mzlab-mol` (which has no such bond: `Bond{a,b,o}`) it is an order-1 bond, so SMILES, V2000 and the checks see a single bond (and a valence one above the usual on the donor: a limit, see below).

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

`crates/mzdraw/tests/bond_kinds.rs` (the field comes and goes exactly, refusals change nothing, the log form, each kind's drawing against the plain one, the arrow's tip and the reverse, hydrogens of the ends, round trips of V3000, KET and CDXML with their direction, the manifest per format, the tools, the menu); golden parity `tests/data/log/bond-kinds.json` (two cases, losses, refusals); e2e steps "bond kinds" (desktop: tools, keyboard, menu, files, undo) and "tablet with touch and a pen: bold, dative and hydrogen bonds" in `tests/e2e.mjs`. Screenshots at deviceScaleFactor 2: `docs/screenshots/bond-kinds.png` (desktop), `docs/screenshots/bond-kinds-touch.png` (tablet 768x1024). There are no dark-mode tokens on `main` to test.

## Not built

- A bond type 9 or 10 in `mzlab-mol` (its `Bond` has an order only): a dative or hydrogen bond is a single bond in the record, so a SMILES or V2000 file shows a plain single bond between the two molecules, listed in the manifest; the neutral donor then has one valence above the usual (`VAL=` in the V3000 atom line). Recommended for a later `mzlab` slice.
- Dative bond direction in SMILES (`->`, `<-`), CML, V2000: none.
- Bold, dative or hydrogen kinds from KET fields other than `type`, CDXML displays other than `Bold` (`Dot`, `Wedge...` are not read as kinds), "unknown" double bond (arch-std-11), haptic bonds, query bonds.
- A key to choose a kind (hover keys wait for arch-cd-03).
