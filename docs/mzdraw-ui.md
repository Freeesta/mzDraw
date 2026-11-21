# mzDraw: action hierarchy, palettes, keys, icons (UI study)

This is a study only; it adds no code. It sets what the UI slice builds: R1 is the palette layout in Rust (§5), PR 2 the icon set and the palettes of the page, PR 3 the light and dark tokens. The owner's feedback of 2026-11-20 found four faults: text-only buttons, no hierarchy, too much contrast, and no light and dark themes. On top of that, the palettes must float and be movable, as the tear-off palettes of ChemDraw are. `*` = a fact not read on a primary source.

## 1. What the other editors do
| editor | arrangement | flyouts | keys |
|---|---|---|---|
| Ketcher 3.x ([help.md](https://github.com/epam/ketcher/blob/master/documentation/help.md)) | Main toolbar on the top panel; tool palette; atoms toolbar; templates toolbar at the bottom | bond kinds, selection kinds, reaction arrows, mapping, R-group, shapes | `c n o` atoms, `1 2 3` bonds, `t` templates, `Mod+Z` / `Mod+Y`, `Mod+=` / `Mod+-` zoom, `Alt+H` / `Alt+V` flip, `Alt+S` check |
| Marvin JS | top file bar, left tools, right atoms, bottom templates* | bonds, charges, arrows* | atom letters on hover* |
| ChemDraw | General toolbar on top, Main Tools palette at the left, palettes torn off from small triangles and floated anywhere* | bonds, rings, arrows, templates* | hotkeys on the hovered atom or bond* |
| JSME ([help](https://jsme-editor.github.io/help.html)) | one row of icon buttons and a column of atoms* | functional-group popup | `C N O P S F L B I H R`, `-` `=` bonds, `3`-`8` rings, `1` phenyl, `a` COOH, `d` delete, `Esc` back to carbon |
| ChemDoodle Web ([sketcher](https://web.chemdoodle.com/tutorial/2d-structure-canvases/sketcher-canvas)) | toolbar rows; the drawing tools can float (`floatDrawTools`) | dropdown buttons* | not documented there |
| BIOVIA Draw | top toolbars, left tool palette, templates panel* | bonds, rings* | not read |

What these editors share:
- The drawing tools sit in a column at the side of the canvas, and the document actions on top.
- The many bond and ring kinds hide behind one button with a small corner triangle.
- The most used atoms are one click away, and the common tools have a key.

What none of them does well: a calm look in a dark theme, the key in every tooltip, flyouts that the keyboard can reach, and palettes that the keyboard can move. No artwork, wording or icon of theirs is used.

## 2. Hierarchy
- **PRIMARY** (12, always visible at full size): select, erase, bond ▸, ring ▸, C, N, O, more atoms ▸, undo, redo, open, save ▸.
- **SECONDARY** (one click in a flyout, or a smaller visible button):
  - bond kinds and wedges; rings of 3 to 8 atoms and benzene; S P F Cl Br I H and any element;
  - charge + and −, and the abbreviation or R-group label; arrow, plus, text;
  - cut, copy, paste, duplicate, rotate, flip, select all; zoom in, zoom out, fit;
  - the 12 save formats: the 11 that `Editor::save` writes, and PNG, which the page rasterises from the SVG.
- **TERTIARY** (the side groups, one toggle away): add a structure by text, style and the settings file, data of the structures, annotation, what is drawn and the check, keyboard help, clear all.

A collapsed palette shows only its head. The head shows the kind last used from it, so using it again takes one click; a click on the head of the tool in use opens the rest.

## 3. Every action, its key and its scriptable twin
- **Built key**: as `Editor::key` binds it today. Only these keys appear in tooltips; an action with no key names its gesture or button there.
- **Planned key**: the wished key, for a later Rust slice that binds it through `Editor::key` returning page flags (Q2).
- **Page command**: what the page sends (`ed.command(...)`, a tool name, or a page call).
- **Log op**: what `save("log")` writes in `draw_edit/1`, replayed by `tests/parity.mjs` and by `mzlab-cli mol draw`. A tool writes ops only when it is used on the canvas.

| action | level | built key | planned key | page command | log op |
|---|---|---|---|---|---|
| select / move | P | `V`, `Esc` | | `select` | `move` (drag) |
| erase | P | `E`; `Del` (selection) | | `erase`, `delete` | `remove` |
| single bond; on a bond it walks 1→2→3 (`editor_tools.rs`, `bond_release`, `Hit::Bond`) | P | `1` | | `bond:1` | `atom.add`, `bond.add`, `bond.set` |
| double, triple (each key picks its tool; only a click on a bond walks) | S | `2`, `3` | | `bond:2`, `bond:3` | same |
| solid wedge, hashed wedge, wavy | S | `W`, `Shift+W`, `Y` | | `bond:up`, `bond:down`, `bond:wavy` | `bond.add` with `s`; `bond.set` with `s`, `rev` |
| benzene | P | — (button) | `R` | `ring:6a` | `ring` is sent; the log keeps what it expands to: `atom.add`, `bond.add` (`ops.rs`, `Op::Ring`) |
| ring of 3 to 8 atoms | S | — (button) | `R`, then `3`-`8` | `ring:3` ... `ring:8` | same |
| carbon, nitrogen, oxygen | P | `C`, `N`, `O` | | `atom:C` ... | `atom.add`, or `atom.set` with `el` |
| S P F H B I; Cl, Br, Si | S | the letter; `Shift+C`, `Shift+B`, `Shift+S` | | `atom:S` ... | same |
| any element | S | — (type the symbol) | `X` | `atom:<symbol>` | same |
| charge + / − | S | `+`, `-` | | `charge:+`, `charge:-` | `atom.set` with `chg` |
| abbreviation or R group | S | `L` | | `label:<name>` | `atom.set` with `lab` |
| arrow | S | `Shift+A` | | `arrow` | `obj.add` |
| plus sign | S | — (button) | `K` | `plus` | `obj.add` |
| text | S | `T` | | `text:<line>` | `obj.add`, `obj.set` |
| undo, redo | P | `Ctrl+Z`; `Ctrl+Y`, `Ctrl+Shift+Z` | | `undo`, `redo` | none: the log holds only the ops still applied |
| cut, copy, paste | S | `Ctrl+X`, `Ctrl+C`, `Ctrl+V` | | `cut`, `copy`, `paste` | `remove` (cut); `atom.add`, `bond.add`, `obj.add` (paste) |
| duplicate | S | `Ctrl+D` | | `duplicate` | as paste |
| orient a scheme: each structure after an arrow turned onto the one before it | S | — (button) | — (Tab to the button, Enter) | `scheme-orient` | `scheme:orient` |
| rotate 15° left or right | S | — (button) | `[`, `]` | `rotate-l`, `rotate-r` | `rotate` |
| flip ↔ / ↕ | S | — (button) | `Alt+H`, `Alt+V` (once `key` reads Alt first) | `flip-h`, `flip-v` | `flip` |
| select all | S | `Ctrl+A` | | `all` | none |
| zoom in, out, fit | S | — (wheel, pinch, buttons) | `Ctrl+=`, `Ctrl+-`, `Ctrl+0` | `ed.zoom`, `ed.fit` | none (view) |
| open a file | P | — (button, drop) | `Ctrl+O` | `ed.add_bytes` | replay request `import` |
| save (the default format, §5) / in a format | P / S | — (button) | `Ctrl+S` | `ed.save(<format>)` | replay request `out` |
| show or hide every palette | S | — (button) | `\` (bound in R1) | layout call (§5) | settings `layout`, not logged |
| add a structure by text | T | none | | `ed.add` | request `import` |
| style, settings file | T | none | | `ed.set_style`, `ed.set` | request `style`, `set` |
| data of a structure | T | none | | `ed.set_data` | `atom.data` |
| annotation | T | none | | `ed.annotate` | `atom.ann` |
| check | T (live) | none | | `ed.state().check` | `check/1` of mzLab, not logged |
| clear all | T | none | | `clear` | `remove` |
| keyboard walk, grow, set | — | arrows, `Home`, `PgUp`, `PgDn`, `Enter`, `Shift+Enter`, `Space` | | `ed.key` | `atom.add`, `bond.add`, `atom.set`, `bond.set` |

Every button has a twin in the log, in the replay request or in the settings file.

A planned key must be free in `Editor::key` (`editor_tools.rs`, `key` and `element_key`), which lowercases the key and reads Shift only with `W`, `A`, `C`, `B` and `S`. So `Shift+E` is erase and `Shift+P` is phosphorus today, and neither is planned: `X` and `K` are bound to nothing, with or without Shift. `Editor::key_at` handles Alt before the letters (drag-free editing: `Alt+Arrow`, `Alt+[`, `Alt+]`, bit 8 of `mods`), so an Alt chord that is not one of those returns 0 and never reaches the element keys: `Alt+H` and `Alt+V` do nothing today.

## 4. Palettes
Every group of actions is a small palette with a drag handle (44 px on touch). The user drags a palette anywhere over the page, docks it on any of the four edges, collapses it to its head, or closes it. The Palettes menu of the File palette reopens a closed palette, and its Reset layout entry brings back the default below.
- **Orientation**: a palette docked at the top or bottom is laid out in a row; one docked left or right, in a column. A floating palette keeps its last orientation and can be flipped.
- **Snapping**: a palette dragged within the snap distance (12 px, a setting) of an edge or of another palette lines up with it, flush and in the same row or column. Dragging it away detaches it.
- **The canvas stays free**: `\` hides every palette and shows it again, and a palette never covers the canvas for good.
- **Keyboard**: on a focused handle, the arrow keys move the palette 8 px (32 px with Shift); `Enter` docks it at the nearest edge or floats it; `Esc` puts it back where the move began.
- **Flyouts** follow the disclosure pattern. The head is a `button` with `aria-expanded`. Enter, Space or the arrow key across the palette opens it, with the focus on the kind in use. The arrow keys move inside it, `Esc` closes it and gives the focus back to the head, and Tab leaves. Each palette is one tab stop; the arrow keys move along it.

| id | holds | icon | desktop default |
|---|---|---|---|
| `tools` | select, erase | select | left, 1, expanded |
| `bonds` | the 6 bond kinds | bond in use | left, 2, collapsed |
| `rings` | benzene, rings of 3 to 8 atoms | ring in use | left, 3, collapsed |
| `atoms` | C N O shown; S P F Cl Br I H B Si and any element in the flyout | elements | left, 4, C N O + head |
| `charges` | +, −, the label field | charge-plus | left, 5, collapsed |
| `objects` | arrow, plus, the text field | arrow | left, 6, collapsed |
| `file` | open, save (default format), the 12 formats, new, the Palettes menu | open | top, 1 |
| `history` | undo, redo | undo | top, 2 |
| `edit` | cut, copy, paste, duplicate, select all, rotate, flip | more | top, 3, collapsed |
| `view` | zoom out, zoom in, fit | fit | top, 4 (end) |
| `structure`, `style`, `data`, `annotation`, `check`, `keys` | the side groups of today | panel | right, 1-6 |

Presets, chosen by Rust from the width of the viewport:
- **desktop** (at least 900 px): the table above.
- **tablet** (600 to 899 px): the same docks, with the right-hand side groups hidden behind the panel button and opened as a drawer.
- **phone** (below 600 px): the left palettes dock at the top as a second row, and the side groups are hidden.

Sizes: an item is 44 × 44 px and every palette has 4 px of padding on each side, inside the size the page measures; Rust stacks the palettes flush, so a separator between two neighbours is 8 px (the padding of both). At 1280×720 the left column holds 6 palettes with 10 items (tools 2, bonds 1, rings 1, atoms 4, charges 1, objects 1): 10 × 44 + 5 separators × 8 + 4 + 4 at its two ends = 488 px. The top row is one item high, 44 + 4 + 4 = 52 px, which leaves 720 − 52 = 668 px for the column. At 768×1024 the column has 1024 − 52 = 972 px.

## 5. Layout and export in the settings file (decided in Rust: slice R1)
`mzdraw-settings/1` gains two optional objects. A version that does not know them keeps them, as it keeps every unknown key.
- `layout`: `{"preset": "desktop", "snap": 12, "palettes": [{"id": "bonds", "dock": "left", "order": 2, "collapsed": true}, {"id": "edit", "dock": "float", "x": 420, "y": 96, "orient": "h"}, {"id": "keys", "hidden": true}]}`.
  - Only what differs from the preset is written.
  - Positions are page pixels. They are clamped to the viewport on every resize.
  - Unknown ids are kept, and missing ids take the preset's place.
- `export`: `{"default": "svg", "background": "white"}`.
  - One click on Save writes the default: an image of the canvas on a white background, with the document embedded (the format awaits the architect's confirmation).
  - The other 11 formats stay in the flyout.

The page sends the measured size of each palette and the pointer or key events on a handle. Rust answers with the rectangle, dock, orientation and order of every palette; it handles snapping, clamping, ordering, the presets and the reset. The page paints that answer and decides nothing. The CLI twin is `--set layout=<object>` or `--set layout.snap=12`, and `--set export.default=svg --set export.background=white` (the default, written out), with no `draw_edit/1` op. Property tests cover the layout: every palette stays inside the viewport, a reset restores the preset, and the layout survives a round trip through the settings text.

## 6. Icons (24×24 grid, 1.5 px stroke, round caps and joins, `currentColor`; 2 px of padding)
| name | drawn as |
|---|---|
| select | dashed rectangle with a pointer at its lower right |
| erase | eraser block tilted 45°, with a base line |
| bond-1, bond-2, bond-3 | one, two, three parallel strokes at 30° |
| bond-up | solid wedge, narrow end at lower left |
| bond-down | hashed wedge: 5 strokes growing in length |
| bond-wavy | sine wave of 3 half periods along 30° |
| ring-3 ... ring-8 | regular polygon of n sides, flat bottom |
| benzene | hexagon with three inner double-bond strokes |
| atom C, N, O, ... | the symbol in the UI font, in its element colour |
| elements | a small periodic-table outline |
| charge-plus, charge-minus | `+` or `−` in a circle |
| label | `R` with a subscript `1` |
| arrow, plus, text | line with an open head; `+`; `T` |
| undo, redo | curved arrow left, curved arrow right |
| cut, copy, paste, duplicate | scissors; two offset sheets; clipboard; sheet with `+` |
| rotate-l, rotate-r | arc of 270° with a head |
| flip-h, flip-v | two triangles mirrored over a dashed axis |
| scheme-orient | a triangle and a short turning arrow |
| zoom-in, zoom-out, fit | lens with `+` or `−`; four corner brackets |
| open, save, clear | folder; tray with a down arrow; blank sheet |
| panel, more | rectangle with a right third ruled off; three dots |

## 7. Decisions (the first option of each was taken on 2026-11-20)
1. Reaction objects have their own palette (`objects`).
2. Keys: no new key now except `\` for the palettes, which comes with R1. Tooltips show only built keys, and the rest name their gesture or button. The wished keys are in the "planned key" column for a later Rust slice; binding them in the page was dropped (rule 2).
3. The atoms are in the tool column, not in a right column as in Ketcher and Marvin JS.
4. Save writes the default of `export` at once, and the formats are in its flyout (§5).
5. On a tablet, the side groups open as a drawer at the right.
6. Label, element and text are entered in a field inside the palette flyout; a popover at the clicked atom waits for a position from Rust.
