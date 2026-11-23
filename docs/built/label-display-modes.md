# Carbon labels, hydrogen modes and the hash style (arch-std-10)

Three optional keys of `mzdraw-settings/1`, in the section `style` (`--set style.<key>=<value>`, the replay request `"set": [...]`, the fields of the page under "Your own values"). They are enumerations, so they sit in the `choices` of `settings::schema_json()` (a sibling table of `FIELDS`), with their words and their default; the page builds the three selects from it as it does for the pathway words. A word outside the schema is refused with `error.draw2.styleValue`, a key that is none with `error.draw2.styleKey`, and nothing changes. A key at its default is not written, so a file that never names them keeps its bytes (the presets, `acs-1996` included, keep their look).

| key | words (default first) | what it does |
|---|---|---|
| `style.carbon_labels` | `none`, `terminal`, `all` | which carbons are drawn as text. `terminal`: a carbon with one bond (`CH3`, `CH2`); also a carbon between two collinear bonds. `all`: every carbon. |
| `style.hydrogens` | `hetero`, `all` | `hetero`: the hydrogens are written on heteroatoms and on the carbons that carry them today. `all`: also on every carbon the style labels. There is no `none`: a label without hydrogens says the atom has none (IUPAC 2008 GR-2.1.1). |
| `style.hash_style` | `wedge`, `lines` | a hashed wedge as the widening ladder (today) or as parallel lines of one length across the bond. |

How it is drawn: the canvas, the SVG and every export that paints go through `scene::Painter`; `Metrics` carries the three values (`carbons`, `h_all`, `hash_lines`), `scene::labelled` decides which atoms are text (a bond stops short of a label by the same code as for a heteroatom), `shown_hydrogens` the hydrogens written. Rules the code keeps:

- A carbon that has a label, a charge, a mass number, radicals, a fixed hydrogen count or no bond is labelled whatever `carbon_labels` says (as before).
- A carbon with one bond always shows its hydrogens when it is labelled (`CH3`, never `C`): under `hetero` only a carbon labelled by `all` with two bonds or more is the bare symbol, and `hydrogens=all` completes it (`CH2`).
- `lines`: the number of lines is the one nearest to `length / hash_spacing` whose gap is 2 to 4 line widths (GR-1.3); never fewer than three, so a bond too short for that keeps three lines with a smaller gap.
- The drawing checks have a new key, `std.stereo.hashLines` (warn): with `hash_style=lines`, a hashed wedge whose wide end is on an atom that is not terminal (ST-0.3 allows parallel hashed lines only towards a terminal atom). `conform::report_with(doc, hash_lines)`; `conform::report(doc)` is the same with `false`.

CLI route: nothing new is needed beyond the settings: `mzlab-cli mol draw` replays a request with `"set": ["style.carbon_labels=terminal", "style.hash_style=lines"]` to the same bytes as the page. No edit operation is new (a setting is not in the undo history), so the property test "apply then undo" has nothing to add.

Tests: `crates/mzdraw/tests/label_display.rs` (words, refusals, file round trip, labels, bonds stopping short, hydrogens, equal rungs and gaps at four bond lengths, the check, the editor), the golden cases of `tests/data/log/label-display.json` (parity wasm == native), the schema row in `tests/pathway_page.rs`, and the e2e steps "display modes by keyboard / touch / pen" with screenshots.

Not built: explicit hydrogen atoms as atoms and aromatic circles (out of scope); the collinear rule (GR-2.1.2) under `carbon_labels=none`: the default look stays byte for byte, so a carbon between two collinear bonds is labelled only with `terminal` or `all` (deferred by decision of the coordinator: it moves four default golden files, `pathway-label`, `step-field`, `tp-lens` and `tp-steps`, so it is a follow-up in its own PR, queued in `docs/plan/queue.md`); `hash_style=lines` is one style for the whole drawing (no per-bond choice); the `hashLines` check does not look at "near-terminal" atoms; an `all` drawing under `hetero` can show a bare `C` for a carbon with hydrogens (the choice is the user's, `hydrogens=all` completes it).
