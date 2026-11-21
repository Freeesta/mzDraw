# Text runs: formatted texts, built

(arch-std-21; `crates/mzdraw/src/runs.rs`, `doc.rs`, `ops.rs`, `scene.rs`, `svg.rs`, `pdf.rs`, `ket.rs`, `cdxml.rs`, `loss.rs`; tests `crates/mzdraw/tests/text_runs.rs`, the unit tests of `runs.rs`, `ops.rs` (the random step), `loss.rs`; golden cases `tests/data/log/text-runs.json`; e2e steps "text runs" in `tests/e2e.mjs`.)

## The model (decided once, for the later panel)

A text object, and the text of an arrow, may carry `runs`: a list of pieces of text with one style each, or line breaks. Keys of a run, written in this order: `t` (the text), `br` (a line break, instead of `t`), `sub` or `sup` (never both), `b` bold, `i` italic, `u` underline, `font` (a family name, at most 64 characters; absent is the font of the style), `sz` (the size as a multiple of the style's font, 0.25 to 8, two decimals; absent is 1), `color` (`#rrggbb`, lowercase; absent is the ink of the drawing). Limits: 400 runs, 1000 characters, 20 lines, 200 characters in a run. A key this version does not know makes the document or the operation unreadable (an error, never a silent drop), so a later key is a version step, not a quiet loss. The list is kept canonical (no empty run, neighbours of one style merged, `sz` to two decimals): equal texts have equal bytes.

`text` stays the plain text, and for a text with `runs` it is the text of the runs (a line break is `\n`), kept in step by every operation and written next to `runs` for readers that know no runs. A plain one-line text has no `runs`, and a document of an earlier version opens and is written unchanged (byte for byte: tested).

The panel of the next slice (bold, italic, font, size, colour, sub/superscript on a selected text) needs no new model: every control is an `obj.set` with `runs`.

## The operations

`obj.add` and `obj.set` take `runs` (one row each in `schema.rs`; `edit-schema.json` moves by those two rows only; `mzlab: false` until mzLab adopts the field, COO 16, approved by the coordinator for this slice). `obj.set` `runs` replaces the text (the plain text of the runs becomes `text`); `runs` and `text` together are `error.draw2.object`; `runs: []` leaves no text (an arrow) or is refused (a text); a plain `text` in `obj.set` takes the formatting away. The inverse is the whole old object (`SetObj`), so undo restores the runs exactly; the random step of the property test in `tests/ops.rs` now adds and sets runs. `diff` writes `runs` where two documents differ in them. An invalid list (a run with `sub` and `sup`, a colour that is not `#rrggbb`, a size outside 0.25 to 8, a break with a style, too many lines) refuses the whole operation (`error.draw2.object`), nothing applied.

## The markup of the text field

`_x` or `_{..}` is a subscript, `^x` or `^{..}` a superscript, `\n` (or a new line) a line break, a backslash makes the next character literal; a `_` or `^` with nothing to take (the end, a space) is a letter. It is read in Rust (`runs::parse`; `runs::to_markup` is the exact inverse for runs that use only scripts and lines, `runs::markup_exact` says so); the text tool turns the typed text into `obj.add` / `obj.set` `runs` (a typed text without markup stays a plain `text`, so every earlier log is unchanged). The page and the replay give the same runs because the log carries the runs, not the markup. `runs::formula` lowers the digits after an element symbol or a closing parenthesis (`CH3CO2H` as `CH`, `3`, `CO`, `2`, `H`); in CDXML the formula face (96) is read through it.

## Drawing

The display list has a new primitive `TEXTX x y size colour string anchor face rgb font` (stride 10) for a run with a style (bold 1, italic 2, underline 4; a colour or a font); a run with no style is the plain `TEXT` as before, so the display list of an unstyled drawing is byte for byte what it was. Scripts are at the style's `small` size over its font (the same ratio as the isotope digits), lowered by 0.25 and raised by 0.4 of their run's size; lines are 1.25 of the font apart (an arrow's text grows upward from the shaft); widths come from the one advance-width table (bold is 7 percent wider), so the page, the SVG, the PDF and the CLI lay a text out alike. SVG: each run is its own `<text>` with `font-weight`, `font-style`, `text-decoration`, `font-family` and `fill` (the display list is flat; absolute positions give the same bytes everywhere, where a `tspan` baseline shift would depend on the viewer's font metrics). PDF: Helvetica, Helvetica-Bold, -Oblique and -BoldOblique (named in the page resources only when a text uses them, so a PDF of an unstyled drawing is unchanged); the fonts of runs other than Helvetica and the underline are drawn in Helvetica and as a line; the page canvas paints the same primitive. The bounds of an SVG include the width and the lines of a formatted text.

## What each format carries (keyed notes of the loss manifest, `loss/1`)

| key | KET | CDXML | SVG, PDF, page | mzdraw/1 | MOL, SMILES, SDF, RXN, CML |
|---|---|---|---|---|---|
| `text.script` (sub, sup) | yes | yes (`face` 32, 64; 96 read as the formula face) | yes | yes | no |
| `text.style` (bold, italic) | yes (`inlineStyleRanges`) | yes (`face` 1, 2) | yes | yes | no |
| `text.lines` | yes (one block per line) | yes (a new line in the run text) | yes | yes | no |
| `text.underline` | no | yes (`face` 4) | yes | yes | no |
| `text.font` (family, size) | no | yes (`fonttable`, `size`) | yes (the PDF: Helvetica) | yes | no |
| `text.colour` | no | yes (`colortable`) | yes | yes | no |

A format that cannot carry a key says so: `loss` and `expect` `lossless` list it (`text.underline`, `text.font`, `text.colour` for KET; every key for the molfile family), never a silent drop. In KET a style range it has no name for is not read (the KET reader ignores unknown style names; this is not counted). A KET or CDXML text with a style or a line break is now one text object with runs; a plain several-line text still becomes one text per line, as before. The CDXML reader takes a run's size as a multiple of 10 pt (the size the writer gives) and the font named Arial as the style's font.

## Relation to arch-web-14 (rich text on the page)

arch-web-14 asks for the page side of the same model: lines of runs `{t, sub|sup, b, i}` typed with markup (`_2`, `^+`, `**bold**`, `*italic*`) or a toolbar, in KET and CDXML, with the display list and SVG laid out from fixed advance widths. This slice is its core: the model (a superset: `u`, `font`, `sz`, `color` too), `obj.set runs`, the parser of `_` and `^`, the layout and widths, KET and CDXML in and out, SVG, PDF and the canvas. What arch-web-14 still has to add is the page: a toolbar (and `**`/`*` markup if wanted) that emits `obj.set` `runs`, on top of this model with no new field. The fixed table of advance widths is the one `scene::width` had; no table was added.

## Not built

- The toolbar and panel of the page (bold, italic, underline, sub, sup, font, size, colour on a selected text): the next slice, on this model; a "Formula" button likewise (`runs::formula` is built and tested; the page and the CLI use it through `obj.set runs` today).
- Markup for bold and italic (`**`, `*`) and for the other styles: the panel sets them.
- Runs in the text of an atom label, an annotation or a caption; rich-text paste from other programs; lists.
- Exact fonts in the PDF (Helvetica faces only); an underline in KET (the format's style names have none we use).
- The markup is not an operation of the log: a script of an agent writes `runs` directly (`obj.add`, `obj.set`).
