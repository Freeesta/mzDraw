# Preferences, built

A gear button in the File palette (item "Preferences", a real button with an accessible name) opens the Preferences flyout. It is a panel beside the button, not a modal dialog: the canvas keeps its keyboard and pointer, Escape closes it and gives the focus back to the gear. It works by keyboard, touch and pen (rows of at least 36 px, native selects and number fields).

One source of truth, in Rust: `crates/mzdraw/src/prefs.rs` lists every option with its `group`, its `label` text key and its kind (words, number, adducts). `settings::schema_json()` carries it as `groups` and `prefs` (`{"setting", "group", "label", "kind", "default", "values" | "unit", "min", "max"}`; a word is `{"value", "label"}`, the label being a key `<label>.<word>`, `.doc` for the document format so that no key holds the product name). The page (`web/prefs.js`) builds the whole panel from it and decides nothing.

Groups, in order:
- **Display**: `style.colour` (by element, the CPK-like default, or one colour: `mono`), `display.maps` (the atom numbers), `display.values` (calculated values under structures), `display.adducts`, `display.lens`.
- **Style**: `style.preset` and the numbers of the style (bond length, line width, wedge width, hash spacing, double bond gap, label margin, font size, subscript size).
- **Pathway**: `pathway.dir`, `pathway.order`, `pathway.number`, `pathway.derive` and the five numbers `pathway.*`.
- **Files**: `export.default` (the default download, `svg` by default) and `export.background` (`white` by default).
- **Theme**: `ui.theme` (`auto`, `light`, `dark`; the theme tokens already follow `data-theme`). Language: English is the only language, said in the group.

Every option is a setting `<section>.<key>=<value>` (the `--set` shape), so the replay request field `set` gives the bytes the page gives (golden `tests/data/log/prefs.json`, wasm == native). One new setting: `ui.theme` (a row in `schema.rs`; the golden `edit-schema.json` moves by that row only). It is stored in the object `ui` of the settings file `mzdraw-settings/1`, written only when not `auto`; unknown keys of `ui` are kept like those of `export`. `Editor::get(setting)` gives the value of any setting as text.

A reset per group: `Editor::reset_commands(group)` is the list of settings that does it (the default of each option of the group, and, for the group Style whose preset replaces the whole style, the current values of the options of the other groups the preset would change), `Editor::reset_group` applies them one by one; a script sends the same list in the `set` field. The panel shows what the module holds after each change, the settings are kept in the browser (OPFS) like the last style and are saved and opened with "Save settings" / "Open settings" of the Style panel.

Not built: the Shortcuts group (arch-std-06 is not merged); label display modes and stereo labels (arch-std-10 and arch-cd-02 are not built, so there is no such setting); a second language. No standalone gear outside the File palette: when the File palette is collapsed the item is in its flyout.
