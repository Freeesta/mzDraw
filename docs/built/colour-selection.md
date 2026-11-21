# Colour of the selection, built

Select atoms and bonds (click, rubber band, select all, the keyboard focus), choose a colour, and that part is coloured, as the Color menu of a drawing program does. The colour is part of the document, one undo step per choice, and the same operation runs from the CLI.

## The operation and the field

- **Document**: the optional field `col` of an atom and of a bond, the text `#rrggbb` (lower case). Absent means automatic: the element colour, or the ink of the mono style. It lives in the unknown-field list of the atom and the bond (`doc::More`), so a document without it reads and writes exactly as before, and a reader that does not know it keeps it. A value that is not a colour is not one (`Atom::col()` is empty) and is kept as it was read. `mzdraw/1` is unchanged (a forward-compatible field, not a new version). The code is `crates/mzdraw/src/ink.rs` (palette, `canon`, the display-list index).
- **Operation** `colour {atoms, bonds, objects, col}` of `draw_edit/1` (row in `schema.rs`; `mzlab` false: mzLab does not read it yet). `col` is `#rgb` or `#rrggbb` in any case, written back as `#rrggbb`; the empty text is automatic and takes the field away. All or nothing: `error.draw2.colour` (not a colour), `error.draw2.noAtom`, `error.draw2.noBond`. The inverse is the recorded `SetAtom` and `SetBond` of each atom and bond (exact: the property test `undo_restores_the_identical_document_and_redo_the_identical_result` has the operation in its random steps, with refused values too), one undo step per choice. A long list of ids uses one index of the pool, so the cost is linear (test `colouring_grows_with_the_size_not_with_its_square`, which times only this operation, its undo and the writer of the field: 20,000 against 40,000 atoms).
- **Editor** (`Editor::colour(col)`, wasm `set_colour`): colours the selected atoms and bonds, plus the bonds between two selected atoms; with no selection it colours the atom with the keyboard focus; with neither it says `error.draw2.colourNone`. The same colour again changes nothing (no empty undo step). The colour of the selection (`selection_colour`, in `state().colour`: the common colour, `""` automatic, `null` none or mixed) is what the page shows pressed.
- **One operation for atoms, bonds and objects**: `objects` names arrows (shaft, head and the text above), plus signs and texts; they get the same field `col` and the same inverse. A selected object is coloured by the same click.

## What draws it

- **Display list** (`scene.rs`): an explicit colour is a **negative colour index**, `-(rgb + 1)` (exact in an `f32`); an index of 0 or more is a token of the surface as before, so no surface needs a table. The page paints a negative index as `#rrggbb` (`web/mzdraw.js`, `paint`); `svg::paint` and `pdf::rgb` decode it for the SVG, the PNG (which rasterises the SVG) and the PDF. The label, its isotope, charge, hydrogens and radical marks, the group labels and every stroke of a bond (both lines of a double bond, wedges and hashes) use it.
- **Interaction with the global setting**: an explicit colour overrides the element colour of that atom and bond; the `mono` style does **not** erase explicit colours (it only turns the element colours to ink); `automatic` clears the override and the element colours or the mono ink show again. Test `an_explicit_colour_overrides_the_element_colour_and_mono_does_not_erase_it`, the golden case "colour: an explicit colour wins over the mono style", and an e2e step. The diff lens (`display.lens`, a display of a diff) keeps priority over an explicit colour on the atoms and bonds it tints, as it is a signal about the change.
- The selection halo, hover and focus ring keep their own tokens (they are not colours of the drawing).

## Files

| format | colour |
|---|---|
| `mzdraw` document, SVG (carries the document), PNG, PDF | carried (PDF as RGB operators) |
| CDXML | atoms and bonds: written as `colortable` and the `color` attribute of the `n` and `b` elements, and read back (the colour of an object is not written: reported, row `colour.object`); the table starts at index 2 (0 and 1 are black and white), as the public CDX pages say: **not checked against a file of ChemDraw** |
| molfile, V3000, SMILES, SDF, KET, CML, RXN, `trasf` | not carried: the save says it (`lost`: key `draw2.export.coloursLost` with the count; the page note; the replay report; `loss:FORMAT` lists the coloured atoms and bonds, row `colour` of the loss table) |

The binary CDX reader ignores colours as before. MRV is not written by this program.

## Page

- **Toolbar**: the opener `Colour` in the Edit palette opens a flyout: Automatic, the eleven colours of the module (`ink::PALETTE`), each with its **name as text** (swatch and name; the swatch is not the only signal), and a field for a colour typed as `#rgb` / `#rrggbb` with Apply (Enter in the field applies; a word that is not a colour is refused with its text and `aria-invalid`). The colour in use on the selection is `aria-pressed`. Buttons are 40 px or more; arrows walk the flyout, Escape closes it, a tap or a pen tap works as a click.
- **Context menu**: the group `Colour` (Automatic, the palette, `Custom...`) on an atom, a bond and a selection, before Copy / Delete; the colour in use is checked; `Custom...` is `ASK` `colour` and puts the keyboard on the field of the flyout. Ids `edit.colour`, `colour.auto`, `colour.<key>`, `colour.custom`; texts `draw2.ctx.*`, `draw2.colour.*` of `web/lang/mzdraw.js` (no literal product name).
- Palette contrast: the colours are chosen to read on white paper (the picker also says the name); explicit colours do not follow the dark theme, as they are the document's.

## CLI route (rules 10 and 15)

The operation is `draw_edit/1`: `{"op":"colour","atoms":[1,2],"bonds":[5],"col":"#d00000"}` in the `log` of a replay request (`mzlab-cli mol draw`), `{"op":"colour",...,"col":""}` for automatic; `out` `svg`, `pdf`, `cdxml`, `mzdraw` give the coloured file; `out` `loss:ket` and `report` give what a format left out. `out: "schema"` lists the row. The page's log of a colour click replayed natively gives the same bytes (`tests/data/log/colour.json`: seven cases, two results, two losses, four refusals; held wasm == native by `tests/parity.mjs`). `diff::diff` says a changed colour with the same operation, and paste, duplicate and the clipboard carry it.

## Digest and loss manifest (coordinator conditions)

- **Digest** (`out: digest`, plan-23): a structure with explicit colours gets the key `colours` after `rings`, `[[colour, atom ranges, bond ranges], ...]` (at most 20; a cut is `coloursCut`), and an object with a colour a fourth entry; a drawing without colours gives the same bytes as before (the digest goldens did not move).
- **Loss manifest** (plan-22): rows `colour` (atoms and bonds) and `colour.object`; every format that cannot carry them lists them as lost, and the property test holds the table to the real round trip.
- **Unknown colour fields**: a `col` that is not a colour (a newer file) is read as automatic, kept and written back, never an error (test `a_colour_field_of_a_newer_file_is_kept_and_never_an_error`).

## Relation to `arch-web-15` (highlight marks)

`docs/prompts/arch-web-15-highlight-marks.md` is **not merged** and has no branch at the time of this slice, and it is another feature: a highlight is a background mark of 8 named colours (`hl`, drawn behind the structure, KET `highlight`), host marks over the drawing are an overlay. This slice colours the ink of the atom and the bond itself. The two do not clash (different fields, `col` and `hl`, different layers); the operation of that slice (`atom.set hl`, `bond.set hl`) is not invented here. If it lands first, nothing here changes; the colour of a highlighted atom stays its own.

## Not built

- Colour of objects in CDXML (reported as left out); a drawing colour for new objects (ChemDraw's "current color"); a colour of the background.
- Colours in molfile, KET, CML, RXN (no place in those formats here; said on save). MRV and binary CDX.
- A keyboard shortcut of its own (the toolbar flyout and the context menu are reached by keys: Menu key or Shift+F10, then `C` of Colour).

## Tests

`crates/mzdraw/tests/colour.rs` (14: canonical text, index, set/clear/undo, refusals, forward compatibility, the editor command, focus, display list and mono, strokes, PDF and CDXML round trip and loss notes, SVG round trip, paste and diff, context menu, scaling); `ops.rs` (property test with random colour steps, the diff tests with the operation); `schema.rs` and `loss.rs` (one row each, 14 operations, 19 features); `tests/data/log/colour.json` (parity); six steps of `tests/e2e.mjs` (keyboard, typed colour, mono and file notes, mouse, touch, pen) with the page equal to the goldens, screenshots at deviceScaleFactor 2 in `docs/shots/colour-*.png`. Goldens that moved: `edit-schema.json` (the row `colour`), `context-menu.json` (the group `Colour` on the atom, bond and selection menus), and the generated tables of `docs/mzdraw.md` and `docs/mzdraw-formats.md`.
