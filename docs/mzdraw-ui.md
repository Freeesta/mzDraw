# mzDraw: action hierarchy, toolbar layout, icons (UI study)

Study only, no code. It sets what PR 2 (icon set, toolbar) and PR 3 (light/dark tokens) of the UI slice build. Owner feedback of 2026-11-20: text-only buttons, no hierarchy, too much contrast, must work in a light and a dark theme. `*` = not read on a primary source.

## 1. What the other editors do
| editor | arrangement | flyouts | keys |
|---|---|---|---|
| Ketcher 3.x ([help.md](https://github.com/epam/ketcher/blob/master/documentation/help.md)) | Main toolbar on the top panel; tool palette; atoms toolbar; templates toolbar at the bottom | bond kinds, selection kinds, reaction arrows, mapping, R-group, shapes | `c n o` atoms, `1 2 3` bonds, `t` templates, `Mod+Z` / `Mod+Y`, `Mod+=` / `Mod+-` zoom, `Alt+H` / `Alt+V` flip, `Alt+S` check |
| Marvin JS | top file bar, left tools, right atoms, bottom templates* | bonds, charges, arrows* | atom letters on hover* |
| ChemDraw | General toolbar on top, Main Tools palette at the left, palettes torn off from small triangles* | bonds, rings, arrows, templates* | hotkeys on the hovered atom or bond* |
| JSME ([help](https://jsme-editor.github.io/help.html)) | one row of icon buttons and a column of atoms* | functional-group popup | `C N O P S F L B I H R`, `-` `=` bonds, `3`-`8` rings, `1` phenyl, `a` COOH, `d` delete, `Esc` back to carbon |
| ChemDoodle Web ([sketcher](https://web.chemdoodle.com/tutorial/2d-structure-canvases/sketcher-canvas)) | toolbar rows, drawing tools in the bottom row | dropdown buttons* | not documented there |
| BIOVIA Draw | top toolbars, left tool palette, templates panel* | bonds, rings* | not read |

What they share: drawing tools in a column at the side of the canvas, document actions on top, the many bond and ring kinds behind one button with a small corner triangle, the most used atoms one click away, and a key for every common tool. What none does well: a calm look in a dark theme, the key in every tooltip, keyboard-reachable flyouts. No artwork, wording or icon of theirs is used.

## 2. Hierarchy
- **PRIMARY** (12, always visible, full size): select, erase, bond ▸, ring ▸, C, N, O, more atoms ▸, undo, redo, open, save ▸.
- **SECONDARY** (one click in a flyout, or a smaller visible button): bond kinds and wedges; rings 3-8 and benzene; S P F Cl Br I H and any element; charge + / −; abbreviation and R-group label; arrow, plus, text; cut, copy, paste, duplicate, rotate, flip; zoom in, zoom out, fit; the 11 save formats.
- **TERTIARY** (the side panel, one toggle away): add a structure by text, style and settings file, data of the structures, annotation, what is drawn, check details, keyboard help, clear all.

A flyout head shows the last kind chosen from it (bond ▸ shows the wedge after a wedge was used), so a repeat is one click.

## 3. Every action, its key and its scriptable twin
Page command = what the page sends (`ed.command(...)`, a tool name or a page call). Log op = what `save("log")` writes in `draw_edit/1`, replayed by `tests/parity.mjs` and by `mzlab-cli mol draw`. A tool writes ops only when it is used on the canvas. Keys: as built in `Editor::key`; `K:` = proposed, not built (needs a Rust slice, §6 Q2).
| action | level | key | page command | log op |
|---|---|---|---|---|
| select / move | P | `V`, `Esc` | `select` | `move` (drag) |
| erase | P | `E`, `Del` (selection) | `erase`, `delete` | `remove` |
| single bond (click a bond: 1→2→3) | P | `1` | `bond:1` | `atom.add`, `bond.add`, `bond.set` |
| double, triple | S | `2`, `3` | `bond:2`, `bond:3` | same |
| solid wedge, hashed wedge, wavy | S | `W`, `Shift+W`, `Y` | `bond:up`, `bond:down`, `bond:wavy` | `bond.add` with `s`, `bond.set` with `s` / `rev` |
| benzene | P | `K: R` | `ring:6a` | `atom.add`, `bond.add` |
| ring 3 to 8 | S | `K: R` then `3`-`8` | `ring:3` ... `ring:8` | same |
| carbon, nitrogen, oxygen | P | `C`, `N`, `O` | `atom:C` ... | `atom.add` / `atom.set` with `el` |
| S P F H B I, Cl Br Si | S | letter; `Shift+C B S` | `atom:S` ... | same |
| any element | S | `K: Shift+E` | `atom:<symbol>` | same |
| charge + / − | S | `+`, `-` | `charge:+`, `charge:-` | `atom.set` with `chg` |
| abbreviation or R group | S | `L` | `label:<name>` | `atom.set` with `lab` |
| arrow | S | `Shift+A` | `arrow` | `obj.add` |
| plus sign | S | `K: Shift+P` | `plus` | `obj.add` |
| text | S | `T` | `text:<line>` | `obj.add`, `obj.set` |
| undo, redo | P | `Ctrl+Z`; `Ctrl+Y`, `Ctrl+Shift+Z` | `undo`, `redo` | none: the log holds only the ops still applied |
| cut, copy, paste | S | `Ctrl+X`, `Ctrl+C`, `Ctrl+V` | `cut`, `copy`, `paste` | `remove` (cut); `atom.add`, `bond.add`, `obj.add` (paste) |
| duplicate | S | `Ctrl+D` | `duplicate` | as paste |
| rotate 15° left / right | S | `K: [`, `]` | `rotate-l`, `rotate-r` | `rotate` |
| flip ↔ / ↕ | S | `K: Alt+H`, `Alt+V` | `flip-h`, `flip-v` | `flip` |
| select all | S | `Ctrl+A` | `all` | none |
| zoom in, out, fit | S | `K: Ctrl+=`, `Ctrl+-`, `Ctrl+0`; wheel, pinch | `ed.zoom`, `ed.fit` | none (view) |
| open a file | P | `K: Ctrl+O` | `ed.add_bytes` | replay request `import` |
| save in a format | P / S | `K: Ctrl+S` (document) | `ed.save(<format>)` | replay request `out` |
| add structure by text | T | none | `ed.add` | request `import` |
| style, settings file | T | none | `ed.set_style`, `ed.set` | request `style`, `set` |
| data of a structure | T | none | `ed.set_data` | `atom.data` |
| annotation | T | none | `ed.annotate` | `atom.ann` |
| check | T (live) | none | `ed.state().check` | `check/1` of mzLab, not logged |
| clear all | T | none | `clear` | `remove` |
| keyboard walk, grow, set | — | arrows, `Home`, `PgUp/PgDn`, `Enter`, `Shift+Enter`, `Space` | `ed.key` | `atom.add`, `bond.add`, `atom.set`, `bond.set` |

Every button therefore has a twin in the log or in the replay request, and the tooltip of every button names its key (a `K:` key appears in a tooltip only once it is built).

## 4. Layout
Desktop (1280×720 and wider):
```
┌ top bar ─────────────────────────────────────────────────────────────────┐
│ mzDraw  [open][save▸] │ [undo][redo] │ [edit▸]      status   [−][+][fit] [panel] │
├──────┬──────────────────────────────────────────────────────┬────────────┤
│select│                                                      │ side panel │
│erase │                     canvas (paper)                   │ (tertiary) │
│bond▸ │                                                      │            │
│ring▸ │                                                      │            │
│ ──── │                                                      │            │
│  C   │                                                      │            │
│  N   │                                                      │            │
│  O   │                                                      │            │
│atom▸ │                                                      │            │
│ ──── │                                                      │            │
│chg▸  │                                                      │            │
│obj▸  │                                                      │            │
└──────┴──────────────────────────────────────────────────────┴────────────┘
```
- One tool column at the left, as the side palettes of ChemDraw and Ketcher: the hand that draws reaches every tool on one line. The atoms sit in the same column under the tools (JSME), not in a second column at the right, which is where the side panel already is; three columns round the canvas would leave the canvas narrow at 768 px.
- Flyouts open to the right of their head, over the canvas. `edit ▸` holds cut, copy, paste, duplicate, rotate, flip, select all; `obj ▸` holds arrow, plus, text; `atom ▸` holds the other elements, any element and the label field; `chg ▸` holds + and −.
- 12 column items × 44 px + 2 separators fit in 720 − 48 px of top bar; nothing scrolls.

Tablet (768×1024, touch): the same column with targets of 44 px; the side panel becomes a drawer over the right of the canvas, opened by `panel`, closed by Esc or a tap outside; flyouts open next to the head and close after a choice. Phone width (< 600 px): the column becomes a row under the top bar, the flyouts open downwards.

Flyouts follow the WAI-ARIA menu button pattern: the head is a `button` with `aria-haspopup` and `aria-expanded`; Enter, Space or `ArrowRight` opens it with the focus on the current kind; arrow keys move; Enter picks; Esc closes and gives the focus back to the head; Tab leaves. Inside the column, arrow keys move between buttons (one tab stop for the toolbar).

## 5. Icons (24×24 grid, 1.5 px stroke, round caps and joins, `currentColor`; 2 px of padding)
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
| atom C, N, O, ... | the symbol in the UI font, no stroke icon; an optional element colour dot |
| atom-more | three small circles (periodic grid hint) |
| charge-plus, charge-minus | `+` or `−` in a circle |
| label | `R` with a small subscript bar |
| arrow | horizontal line with an open head |
| plus | `+` without circle |
| text | serif-free `T` with a short base |
| undo, redo | curved arrow left, curved arrow right |
| cut, copy, paste, duplicate | scissors; two offset sheets; clipboard; sheet with `+` |
| rotate-l, rotate-r | arc of 270° with a head |
| flip-h, flip-v | two triangles mirrored over a dashed axis |
| zoom-in, zoom-out, fit | lens with `+` or `−`; four corner brackets |
| open, save | folder; tray with a down arrow |
| panel | rectangle with a right third ruled off |
| edit, more | three dots in a row |

The set is drawn for mzDraw (PR 2); no icon pack, no tracing.

## 6. Open questions (two options each; the first is what PR 2 builds unless told otherwise)
1. Reaction objects: (a) their own `obj ▸` head in the column; (b) inside `atom ▸` with the other rare tools.
2. Keys not built yet (rings, plus, rotate, flip, zoom, fit, open, save, any element): (a) bound in Rust (`Editor::key` gains page flags such as open and save, as it has `ASK_LABEL`), one small slice of `crates/mzdraw`; (b) bound in the page as forwarded clicks, against rule 2.
3. Atoms: (a) in the tool column; (b) a right column as Ketcher and Marvin JS.
4. Save: (a) one `save ▸` flyout with the document first and the 10 other formats; (b) `save` writes the document at once and a separate `export ▸` holds the formats.
5. Side panel on a tablet: (a) a drawer at the right; (b) a bottom sheet.
6. Label and text entry: (a) a field inside the flyout; (b) a popover at the clicked atom or spot (rule 4 of `mzdraw.md`, needs the position from Rust).
