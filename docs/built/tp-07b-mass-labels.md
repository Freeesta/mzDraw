# Mass labels on arrows (tp-07b), built

(`crates/mzdraw/src/steplabel.rs` (modes, `Opts::adduct`, `plan` with a `Measurer`, `Plan::notes`), `tptable.rs` (`shift4`, `plain4`: the one formatter), `editor.rs` / `editor_pathway.rs` (`label_notes`, the editor's `measurer` handed to `plan`), `replay.rs` (the engine is attached for a log that runs `pathway:label`), `tp_list.rs`; text `web/lang/mzdraw.js`; tests `crates/mzdraw/tests/steplabel.rs` (13 new), `crates/mzdraw-wasm/tests/label_mass.rs` (5, real engine); golden `tests/data/log/pathway-label-mass.json` (4, through the glue door).) No `draw_edit/1` op and no request row is new: the command still writes `obj.set` of `text`; the row of `pathway:label` gets the argument `adduct` and its text the mass modes, the row of `pathway.label` its text (`docs/mzdraw.md` and `tests/data/log/edit-schema.json` moved by that, blessed).

The modes `mass` and `delta+mass` of the setting `pathway.label` (tp-07 refused them with `error.draw2.labelMass`) now write numbers, and the optional argument `adduct` adds the m/z of the product.

| mode | an arrow `P -> T` with delta `+O` | an isomer |
|---|---|---|
| `delta` (default) | `+O` | `isomer` |
| `mass` | `+15.9949` | `isomer` |
| `delta+mass` | `+O +15.9949` | `isomer` |
| `mass` / `delta+mass` with `{"adduct":"[M+H]+"}` | `... [M+H]+ 151.1117` | `isomer [M+H]+ 151.1117` |

An isomer has equal formulas (the rule of tp-07) and gets its word with no shift: the shift of equal formulas is 0 by definition (isotopologues, which have equal counts and different masses, are not told apart: not built). The label is the plain tokens joined by a space (an object text has no line break; `below` of arch-web-13 is not on `main`).

## Where the numbers come from

Only the engine, through the one arm `glue::measure` of arch-web-17 (`values::Measurer`, a function pointer held by `Editor::measurer`; the bridge sets it, and the replay of the glue door passes it). This crate has no mass table and does no mass arithmetic: the shift is the difference of the two UNROUNDED masses the engine gave for the two structures, the m/z is the engine's `mz` for the adduct (its mass plus the engine's own shift). The page and the native twin (`mzlab-cli mol draw` calling `replay_with(request, n, Some(measure))` with its own copy of `glue::measure`, as `values` and the TP table already need) therefore write the same bytes; until that twin exists the native route is `cargo run -p mzdraw-wasm --example replay_glue`, which `tests/parity.mjs` compares with the page (wasm == native == recorded, 4 cases of the new golden file). A log that runs `pathway:label` gets the engine attached in `replay` (no other command or output changes). Without an engine (the plain replay of `mzdraw`) `mass` and `delta+mass` are `error.draw2.labelMass`, exit 1, `failed.stage` `command`, nothing changes (the case of `pathway-label.json` still holds). `tp:import` writes the delta when the mode has numbers (it has no engine; run `pathway:label` after it).

## Rounding, decided once

A label and the TP table (tp-09) are **the same digits**: both call `tptable::shift4` (a shift, signed with `+`/`-`, `0.0000` unsigned) and `tptable::plain4` (a mass or m/z), which are Rust's `{:.4}`: the exact decimal value of the binary number, ties to even. The label's shift equals the `mass_diff` cell and its m/z the `mz_calc` cell of the same drawing (tested on the 12-row list with the real engine: every label `form_diff mass_diff` is a row of the table).

The panel of arch-web-17 (`values::four`, `mzlab_mol::fixed`) rounds half away from zero on the value already rounded to 9 decimals. The two agree everywhere except on an exact tie at the fifth decimal of a binary fraction (0.03125: `0.0312` against `0.0313`), which no real mass reaches. That difference is **kept, not hidden**: a test pins both (`the_label_and_the_table_write_the_same_digits_and_the_panel_rounding_is_pinned`: `shift4(0.03125) == "+0.0312"`, `four(0.03125) == "0.0313"`, and equal on five ordinary values). The reason not to move `four`: its bytes are what `mzlab-cli` and the goldens of arch-web-17 have written since that slice, and the TP table (a table people paste into an SI) already uses `{:.4}`.

The shift of a charged product is the plain difference of the engine masses, with no electron mass (the engine's mass of an ion is the sum of its atoms, `values`): ethanol to ethanoxide is `-H -1.0078`, the mass of a hydrogen atom, in the golden file and in a test.

The painter shows the true minus (U+2212) in a mass shift as in a delta (`steplabel::display`); the data, the log and every export keep `-`.

## Whose label it is

`owned` knows the new tokens: a signed number with 4 decimals (tp-07), an adduct name of `values::ADDUCTS`, and an unsigned number with 4 decimals **only right after an adduct token**. A bare `151.1117` (a person's measured m/z) is the person's and is kept (tested); `force` rewrites all. A second run changes nothing.

## Nothing is dropped in silence

- A structure the engine refuses (an R group `error.draw2.valuesOpen`, a metal ion or explicit hydrogen `error.grafo.element`, over 200 atoms `error.grafo.tooBig`): its arrow keeps its text, counts as `skipped` in the live line (`draw2.say.labelled`, whose English now says so), and the engine's own key is a note.
- A product with a charge has no adduct (`values`): the shift is written, the m/z is not, note `draw2.label.mzCharged`.
- A note is reported only while the drawing is the document the run left (`Editor::label_notes_doc`): an edit, a load, or a run that fails hides or clears it, an undo back to that document brings it back (tested). A second key, `draw2.label.massMissing`, covers an arrow whose end is not in the scan: it cannot happen (both read the same record), so it is a note and not a panic, and it has no test beyond its text.
- Notes are `Plan::notes` -> `Editor::label_notes` -> `lost` of a `draw-result/1` (`[{"key","n"}]`, the mechanism of `draw2.export.mapsLost` and the TP table). Each key has its English text (`web/lang/mzdraw.js`).
- Above 15,000 atoms or 1,000 structures (the caps of the TP table, the same scan): `error.draw2.valuesSize`, the engine not called.
- `adduct` must be one of the seven of `display.adducts`, and only with `mass` or `delta+mass`: else `error.draw2.request`, before any work.

## Cost

One engine run per structure (`Scan::measure_within`, as the table), however many arrows end in it. Tests, with a stand-in engine (a cheap fixed function of the molfile, so only this crate's code is timed; best of five, no floor): `the_mass_labels_ask_the_engine_once_per_structure` counts the engine's calls (999 arrows over 1000 structures: 1000 calls, never 2 per arrow) and checks the cap is refused before any call; `the_mass_labels_scale_linearly_in_arrows_atoms_and_objects` (250 against 999 arrows over structures of 14 atoms, bound 8; and 999 arrows with 50,000 other objects, bound 6); `the_cost_per_arrow_does_not_grow_with_the_atoms` (14,980 atoms fixed, 250 against 999 arrows anchored to the last atoms, bound 1.25). Planted slow variants, run here in debug: a `values::scan` per arrow fails the first (about 17 times the small run), a scan of `doc.objects` per arrow fails the second (315 ms against 41 ms), a full scan of `doc.atoms` per arrow fails the third in 3 of 3 runs (ratio about 1.45; the unmutated code is at 0.95 to 1.1 and passed 3 of 3), and an early-exit `position` lookup of an atom per arrow fails it in 2 of 3 runs (ratio about 1.15 to 1.35, so it is at the edge of what a time can show; the call count and the structure of the code are the other guards).

## Proof

- Unit (`crates/mzdraw/tests/steplabel.rs`, stand-in engine with made-up weights: these test the label, never a mass): the four label forms and their idempotence and one undo step; isomers; the rounding pin; a negative shift and the true minus; an engine refusal as a note with the other arrow still labelled; a charged product; no engine; the notes follow the drawing (below); the checks of `adduct`; the ownership of numbers.
- Real engine (`crates/mzdraw-wasm/tests/label_mass.rs`): hand sums to four decimals (`+O` +15.9949, M1 `[M+H]+` 151.1117, `-CH2` -14.0157, ethanol -> ethanoxide `-H -1.0078`); label == table cell for the 12 arrows of the list; `Na+` and a charged product as two keyed notes in `lost`; the plain replay refuses; the golden through the glue door.
- Goldens `tests/data/log/pathway-label-mass.json` (new file, section `values`): shift, `delta+mass` with `[M+H]+`, a negative shift with `[M-H]-` and the true minus, the notes in `lost`. They fail without the change: `adduct` is `error.draw2.request` and the mass modes `error.draw2.labelMass` on `main`. Their numbers were hand-checked (ethanol 46.0419 + 1.0073 = 47.0491; ethane 30.0470 - 1.0073 = 29.0397).

## Not built

- The page control (a way for a person to choose mass/adduct mode on the page) is NOT in this PR: it is queued as a follow-up after the Preferences slice merges. The setting `pathway.label` and the argument `adduct` are on the CLI and the settings file. No page file but the texts changed, so no screenshots and no e2e step were added.
- Isotopologues (equal counts, different masses) are `isomer`; the `below` slot; the m/z of an adduct other than the seven; the native `mzlab-cli` twin of `glue::measure` (mzLab's side). The old notes of `tp-07-delta-labels.md` and `arch-web-17-values.md` that say the mass modes are not built are superseded by this file.
