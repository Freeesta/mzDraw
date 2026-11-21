# Mass-label choice on the page (mass-label-ui), built

The mass labels of the steps (`tp-07b-mass-labels.md`) had no control on the page: only the request and the settings could choose the mode. Now a scheme holds its own choice, the page sets it from the menu, and the Preferences hold the default for schemes with none.

## What is stored, and where

- **The document field `labels`** (`crates/mzdraw/src/doc.rs`: `labels`, `set_labels`, `labels_canon`): optional, `{"mode", "adduct"}`, written only when set, inside the unknown-field list of the document (like `cap` of an atom), so `mzdraw/1` stays `/1` and an older reader keeps it. `mode` is one of `delta`, `mass`, `delta+mass`, `name+delta`; `adduct` one of `display.adducts`, and only with `mass` or `delta+mass`. A word outside those is `error.draw2.labels` (never a fallback). Keys of `labels` this version does not know are kept. COO 16 (mzLab, owner of `draw_edit/1`) agreed the field and the operation name.
- **The operation `doc.labels`** (`Op::DocLabels { mode, adduct }`, inverse `Inv::SetMore`; row in `schema.rs`, `docs/mzdraw.md` and `tests/data/log/edit-schema.json` blessed): sets the field; the empty `mode` takes it away. Covered by the property test "apply then undo returns the identical document" (a generator case and a sample row), by the diff of two documents (`diff` writes it), and by a loss row (`labels`, kept by SVG and the document only; `docs/mzdraw-formats.md`).
- **Two settings, the defaults for new schemes**: `pathway.label` (already there) and the new `pathway.adduct` (`none` or one of the seven adducts; written to the settings file only when not `none`; a value that is none is `error.draw2.styleValue`). Both are rows of the Preferences panel, group Display.

## What the command uses

`pathway:label` runs with: an argument of the command (`adduct`), else the scheme's `labels`, else the settings. A default adduct is taken only by a mode that writes numbers (`Editor::label_choice`). The CLI route is the log `{"op":"doc.labels",...}` plus the command `pathway:label`; a native replay with the engine gives the page's bytes (`tests/data/log/mass-label-ui.json`, five cases, wasm == native == recorded in `tests/parity.mjs`; `crates/mzdraw-wasm/tests/mass_label_ui.rs` also holds that the menu route and the log route write the same scheme).

## On the page (no decision in JS)

- **Canvas menu and the menu of an anchored arrow**: the group "Label mode" (four items, the one in force ticked) and the group "Label adduct" (off while the mode has no numbers). Reached by mouse (right button), keyboard (Menu key), touch (long press) and pen (barrel button): the menu is the existing one. An item runs `Editor::set_label_choice`: the field and the labels of that mode, in ONE undo step. A mode that needs the engine where there is none, or a choice the field refuses, is an error key and changes nothing. A person's own label text is kept, as `pathway:label` keeps it. A mode with no numbers drops the adduct.
- **State JSON**: `state().labels` = `{mode, adduct, own, meaning, now, modes, adducts}` (`Editor::labels_json`): what is in force, whether the scheme holds its own choice, and the text key of what the mode means. The button "Label arrows" says it in its `aria-description` (the finished key `now`, one key per mode, the adduct its one parameter).
- **Preferences**: the rows "Labels under arrows" and "Adduct in mass labels" (Display), in the panel built from the schema.
- Texts are keyed in `web/lang/mzdraw.js` (the existing key tests cover the Preferences and the schema). The refusal `error.draw2.labels` has its text.

## Limits (stated, not hidden)

- **Import and paste drop `labels`.** `Editor::import` and paste add structures with new ids and carry no document-level field, so a `labels` choice of an imported or pasted document is lost; opening a file keeps it and a log carries it with `doc.labels`. A follow-up row is in `docs/plan/queue.md`.
- **The diff** writes `doc.labels` as a hunk of its own (names no id, applies alone, said as `draw2.hunk.labels`). When the later document holds a `labels` field of a shape this version cannot read (not an object, a bad word), `diff` sees "no choice" and writes the clearing op when the earlier one had a choice: the patch does not reproduce those bytes of the later document (the odd field is not a choice, and no patch of this version writes it).
- **The loss row** `labels` is one row for the choice; the unknown keys inside `labels` that a later version may add are not named by `loss/1` (they are kept in the file, listed nowhere).
- **A menu choice writes what is in force.** Choosing a mass mode writes the adduct that is in force into the document, which is the Preferences default (`pathway.adduct`) when the scheme had none: the scheme then keeps that adduct even if the Preferences change later.
- A free (unanchored) arrow has no label items. The adduct group is the list of the seven adducts of `display.adducts`.
- No scaling test: the new code does no work on the scheme (one field, one plan the existing tests time).

## Proof

Unit `crates/mzdraw/tests/mass_label_ui.rs` (field written only when set and exact undo; bad choices; unknown keys kept; precedence argument > scheme > settings; state; menu items and one-step choice; engine refusal; loss), the property test (`tests/ops.rs`), schema tests, `crates/mzdraw-wasm/tests/mass_label_ui.rs` (real engine: page route == log route, hand sums, golden), e2e `tests/e2e.mjs` (mouse, keyboard, touch, pen, Preferences), screenshots at deviceScaleFactor 2: [menu](../shots/mass-label-ui-menu.png), [touch](../shots/mass-label-ui-touch.png), [pen](../shots/mass-label-ui-pen.png), [arrow menu](../shots/mass-label-ui-arrow-menu.png), [Preferences](../shots/mass-label-ui-prefs.png).
