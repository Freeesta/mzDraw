# Bond display: the side of the second line and dashed lines (arch-cd-04)

Study rows A03 (built) and A04 (built for `dash`, `bdash`, `hash`; `bold` stays with arch-web-20). Code: `crates/mzdraw/src/bonddisp.rs` (the fields), `ops.rs` (`Op::BondSet`), `scene.rs` (`Painter::seg`, the side in `Painter::bond`), `editor_tools.rs` (the tools), `context.rs` (the menu), `cdxml.rs` (the one file format that holds them), `loss.rs`.

## The fields

Both are optional keys of a bond in `mzdraw/1`, kept in the unknown-field list of the bond as the colour is (`ink.rs`): a document without them is the same bytes as before, and an older reader keeps them.

- `pos`: where the second line of a double bond goes, seen from the atom `a` to the atom `b` on the screen: `c` centred, `l` left, `r` right; absent is `auto`. Only a double bond has one.
- `k`: how the line is drawn: `dash` (a dashed line), `bdash` (bold dashed), `hash` (plain hashed: short ticks across the line, not a stereo mark); absent is solid. This is the one display field of COO decision 16. arch-web-20 was not on `main` when this slice was built, so only these three values exist; `bold`, `dative` and `hbond` come on the same field with the same reader and writer (`bonddisp::k_canon`, `KINDS`). A value this version does not know is kept as it is, drawn as a plain line and listed by the loss manifest as `unknown`.

**Auto rule** (unchanged drawing, written down): in a ring the second line goes to the side of the ring (the side with more of the neighbouring bonds); a double bond with a free end (C=O, =CH2) is centred; in a chain it goes to the side with more substituents (the signs of the neighbours of both ends are summed; a tie goes right).

## The operation (`draw_edit/1`)

`{"op":"bond.set","id":11,"pos":"l","k":"dash"}`: `pos` is `auto`, `c`, `l` or `r`; `k` is `solid`, `dash`, `bdash` or `hash`; both optional, defaults left out of the log. A new order other than 2 drops `pos`; a new wedge mark drops `k`; a `pos` on a bond that is not double, or a `k` on a bond that has a mark, is refused with `error.draw2.bond` and nothing changes; a value outside the lists is refused at reading (`error.draw2.request`). The two fields are `mzlab: false` in the schema (mzLab's reader does not know them yet). Inverse: `SetBond` (the whole bond), in the property test (`tests/ops.rs`, `random_step`) and in `diff` (a patch between two documents says them).

## The page

- Double-bond tool on a double bond: each click walks `auto -> l -> c -> r -> auto`.
- Bond palette: tools `bond:dash`, `bond:bdash`, `bond:hash` (in the flyout of the bonds palette); on a bond they set the kind (a second click takes it away); from an atom or on empty canvas they draw a single bond of that kind.
- Context menu of a bond: the group "Double bond side" (double bonds only: Automatic, Centred, Left, Right) and "Line style" (Solid, Dashed, Bold dashed, Hashed; off for a bond with a wedge mark).
- Hover keys (`L`, `C`, ...): not built; arch-cd-03 has not merged.

## Display and files

Screen and SVG are one display list: a dash is a run of lines as long as the hash spacing (a bold dash 2.5 times the stroke), a hash is ticks across the line. Every line of a double or triple bond takes the kind.

- CDXML out: `Display="Dash"` / `Display="Hash"` (CDX SDK page `Bond_Display`) and `DoublePosition="Center|Left|Right"` (page `Bond_DoublePosition`: the side is "viewing from the begin atom to the end atom"); the side is turned when the written begin atom is the other one. CDXML in reads them back (`Dash`, `Hash`, and `DoublePosition` of a double bond; an unlisted value is passed over). The SDK page says a double bond supports Solid, Dash and Bold only: a hashed double bond is written all the same. The SDK pages distinguish a position set by hand (256 + value) from an automatic one; CDXML names do not, so every `DoublePosition` read is taken as set by hand (gap).
- `bdash` has no CDXML value: written as a plain line and listed (`bond.bdash`).
- Molfile (both), SMILES, SDF, KET, CML, RXN: none of the four is written; the loss manifest lists `bond.pos`, `bond.dash`, `bond.bdash`, `bond.hash` (rows in `docs/mzdraw-formats.md`, `loss.rs`). KET has no dashed or side field in its bond schema as far as the schema read goes (not confirmed here).
- SVG, mzDraw document: carry all.

## Tests

`crates/mzdraw/tests/bond_display.rs`; golden parity `tests/data/log/bond-display.json` (cases, losses, refusals); e2e steps "bond display" in `tests/e2e.mjs`.

## Not built

`bold`, `dative`, `hbond` (arch-web-20); the double bond of unknown geometry (arch-std-11); hover keys (arch-cd-03); a bond-display preference; reading dashed/bold displays from molfile or KET.
