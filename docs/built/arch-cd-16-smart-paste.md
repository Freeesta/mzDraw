# Smart paste (arch-cd-16, study row J04)

A paste (the editor's own clipboard, a Ctrl+V of text from another program, or the context menu item Paste) lands in a free place and
never on the drawing. The view (zoom, pan) does not change.

## What it does

- `Editor::paste` reads the structure text and asks `Editor::free_offset` for the shift of the pasted part (`editor_edit.rs`).
- The place is the centre of the view (the canvas size is told by the page with `canvas(w, h)`, `Editor::set_canvas`; `fit` sets it too;
  default 400 x 400 px, which puts the centre at the origin of the default view) when no drawn point is within half a bond of the pasted box.
  The drawn points are the atoms, a sample every quarter bond along each bond, the anchors of the objects and a sample along each arrow.
- Otherwise the candidates are the cells of a grid of one bond length around the centre, in the quadrant to the right and below, by growing
  distance (ties: the nearer to the right, then the lower); when that quadrant (40 bonds) is full, the nearest free cell in any direction, and
  last to the right of the drawing. The result depends only on the document, the view and the canvas size: it is deterministic.
- The shift is part of the paste step: the `atom.add` operations of the step carry the final coordinates (no separate `move` is
  needed), so the log of the page replays natively to the same bytes. One undo step; the pasted part is the selection and the tool is Select.
- `duplicate` (Ctrl+D) still lands next to its source, half a bond further at each press.

## CLI route

No new operation. The replay request has two new fields (rows of `draw-edit-schema/1`): `paste` (a structure text pasted as the page pastes it,
after `commands` and before `events`) and `canvas` (`[w, h]`, the canvas size the view centre is taken from). A refused paste is the error of the
request (its i18n key). Golden cases: `tests/data/log/smart-paste.json`.

## Tests

`crates/mzdraw/tests/smart_paste.rs` (centre of an empty view, no overlap on repeated pastes, view unchanged, one undo step, determinism);
the golden file above (wasm == native, `tests/parity.mjs`); e2e: keyboard, context menu (mouse), touch and pen.

## Limits (tested)

- A text object is one anchor point for the placement: its drawn extent is not reserved, so a long text may reach a neighbour (`a_paste_with_an_arrow_and_a_text_keeps_clear_of_a_drawing`).
- A part wider than 80 bonds is placed by the same rule (box over the centre, then the free cells); `a_part_wider_than_eighty_bonds_...`.
- Fallbacks: quadrant right and below full, then any direction within 40 bonds, then two bonds right of the drawing (`when_the_quadrant_is_full_...`, `when_no_cell_is_free_...`).

## Not built

- Copy as SMILES, molfile, KET, image (arch-web-21); native clipboard formats of the desktop shell (J07).
- A paste of a part larger than the view still lands at the view centre (the view does not rezoom to show it).
