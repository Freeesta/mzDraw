# mzDraw: a molecule editor of our own (study and build plan)

mzDraw is a 2D structure editor written in Rust and compiled to wasm. It began inside mzLab (`Freeesta/mzlab`, where this study was written and where the files `drawing.md`, `drawing-menu.md`, the competitor matrix and the CLI route `mzlab-cli mol draw` live) and now stands in its own repository, `Freeesta/mzdraw`. The name lives in one place per layer: the crate names `mzdraw` / `mzdraw-wasm`, the constant `mzdraw::NAME` and the i18n key `draw2.name`.

Companion studies, copied from mzLab (written there, so their paths are mzLab paths): `mzdraw-competitors.md` (competitor matrix), `mzdraw-formats.md` (format conformity, settings, annotations; the built field list of annotations is `mzdraw-annotations.md`), `mzdraw-plan.md` (priorities and open decisions). Guidelines and standards (IUPAC, journal styles, format specifications, WCAG 2.2, colour) with a conformance checklist and the feature table: `study/arch-std.md`; its DEV prompts are `prompts/arch-std-NN-*.md`. Written here: `study/arch-web.md` (web and embeddable editors, the complete feature list with priorities and scriptable twins, the `<mzdraw-editor>` embedding API for the two hosts; DEV prompts in `prompts/arch-web-NN-slug.md`). Also written here: `study/arch-cd.md` (desktop editors: ChemDraw, BIOVIA Draw, MarvinSketch, ChemDoodle; the feature list with priorities and scriptable twins, palettes and context menus by target; DEV prompts in `prompts/arch-cd-NN-slug.md`, with the prompt to run where another study covers the same slice).

What to build next, in order and in waves, is one list: `docs/plan/backlog.md` (it merges the studies arch-std, arch-web and arch-cd, the external research input and the running work; new prompts `docs/prompts/plan-NN-*.md`).

## Built state

What is built is told in `docs/built/`: one short file per slice or feature, so that two PRs never edit the same lines (see `docs/built/README.md`). There is no list, counter or size figure here to keep in step; the size of the module is printed by `tools/build_site.sh`.

## Scope
- **In**: organic molecules; atoms and elements, bonds (single, double, triple, wedge, hash, wavy, aromatic), charges, isotopes, radicals, labels, abbreviations (Ph, Me, Et, Boc, ...), R1/R2 placeholders, rings and templates (benzene, cyclohexane, rings 3-8), chains, select/move/rotate/copy/paste/delete, undo/redo, fragments, reaction arrows with text, text annotations, zoom/pan, mouse + touch + pen + keyboard.
  Also in: every exchange format of the conformity matrix, SVG/PNG/PDF out, clean-up (2D layout), live valence and charge check (`check/1`), CIP labels, styles and presets, per-structure annotations.
- **Out**: macromolecules (peptides, DNA, polymers), query features (atom lists, query bonds), S-groups other than simple abbreviations. A file carrying them opens with the unsupported parts listed in the import report, never silently dropped.

## What the Ketcher page does (to match, from `drawing.md` and `drawing-menu.md`)
| Drawing page feature | mzDraw |
|---|---|
| atoms, bonds, rings, chains, charge, radical, eraser, selection, arrows, text, shapes | slices 1-2 (shapes: later) |
| zoom in/out/reset, full canvas, tablet toolbars, 40 px touch targets, long-press names | slices 1-2 |
| "New" with confirmation, unsaved-changes guard | slice 2 (autosave in OPFS makes the guard a recovery prompt) |
| Quick draw: paste a SMILES, search a compound by name | SMILES slice 1; name search later (molecule database glue) |
| open/drop/paste molfile, KET, CDXML, CDX | matrix below; slice 1 molfile + SMILES |
| export images, `.ket`, MOL/SDF | slice 1 molfile + SVG, slice 3 the rest |
| right-click menu, touch copy/cut/paste | slice 2 (own clipboard, no Ketcher traps) |
| labels under structures, formula analysis, Information..., fragments, cuts, mechanism, partial structures, multicentre, evidence colours, substructure search, outside links, CIP card | later, one by one: every engine already takes a molfile (`label/1`, `analysis/1`, `frag/1`, `cut/1`, `mechanism/1`, `parz/1`, `highlight/1`, `substructure/1`, `link/1`, `cip/1`); mzDraw hands over the same molfile plus stable atom ids |
| `TPDraw` API for mzFinder and Tabui (`load`, `smiles`, `partial`, `mark`, `highlight`) | later; the wasm API is shaped for it (`Editor::export`, `select`, `overlay`) |

## Assets reused (nothing here is rewritten)
- `mzlab-mol` (deps: `serde`, `serde_json`, `sha2`): record `mol/1`, SMILES/CXSMILES and molfile V2000/V3000 readers and writers, `drawing_from_molfile` (wedges, query marks kept), valence table, Kekulé, aromaticity perception, stereo sites, canonical order, pieces.
- `mzlab-feat` `chem/` (features `check`, `sub`): `check/1`, `cip/1`, `highlight/1`, `sdf/1`, labels, analysis. mzDraw calls them only from the glue layer.
- Agent drawing (`agent-draw.md`, branch `claude/agent-draw`): the route "own Rust layout + SVG writer in `mzlab-mol`". Work in progress elsewhere and consumed here, never duplicated: `Mol::layout_2d` / `layout_2d_fixed` and `mol draw` (D1), the SVG writer (D2), the edit patches of `mol draw --set edit=` (D3), the KET and CDXML writers (D4), the CDXML reader (DRAW-CDXML).
- Until D1 exists, "Clean up" calls `mzdraw::layout::clean_up`, one function that returns the coordinates unchanged with `status: "unavailable"`; wiring D1 changes that function only.

## Architecture
```
mzlab-mol (record, readers, writers; later layout + SVG)      no mzLab dependency
   ^
mzdraw (document, scene, commands, undo, tools, hit-test,     depends on mzlab-mol, serde, serde_json only
        display list, settings, format registry)
   ^                          ^
mzdraw-wasm (wasm-bindgen)    mzlab-cli `mol draw` / mzlab-desktop
   + mzlab-feat[check]  = the thin mzLab glue: check/1, cip/1, i18n keys, OPFS autosave
```
- **Leaving the repo** = moving `crates/mzlab-mol` + `crates/mzdraw` (+ `mzdraw-wasm` without the `mzlab` cargo feature) and `mzlab/web/draw2/`. Guard: a test reads `cargo metadata` and fails if `mzdraw` gains a dependency outside `mzlab-mol`, `serde`, `serde_json`.
- **One wasm module** `mzdraw-wasm`, an entry of its own in `tools/moduli/` with its own cap. Targets: < 600,000 B raw, < 250,000 B gzip; the cap is the first measured size + 5 %. Owner lifted the limit on 2026-11-20; revisit before the first release (the cap is now a 4,000,000 B guard, see `tools/wasm-cap.toml`). Basis: `molkit` (record + readers + `check/1` + substructure) measures 352,069 B; the editor core is commands and geometry with no tables, estimated 80-120 KB on top of the readers.
- **Render path: Canvas2D for the live view, SVG from Rust for files.** Measured (scratch Playwright bench, Chromium, deviceScaleFactor 2, synthetic 500 atoms / 520 bonds / 150 labels, full repaint, median / p95): Canvas2D 2.4 / 3.2 ms; SVG DOM rebuild 3.6 / 7.5 ms script only, 814 nodes, plus style and layout before paint. With CPU throttled 4x (tablet class): Canvas2D 16.2 / 21.4 ms, SVG 17.7 / 28.9 ms script plus a frame at 27.5 ms.
  Canvas keeps a full repaint inside one 60 Hz frame on a desktop and close to it on a tablet with no DOM to keep in sync; a drag repaints only the display list, which Rust rebuilds in place. Hit-testing never needs DOM nodes (it is in Rust). To re-measure on WebKit on a Mac and on a real tablet in slice 2 (the cloud container has no WebKit build).
- **Display list**: Rust turns document + style + view state into a flat list of primitives (`line`, `poly`, `text`, `arc`, `rect`, each with a style index and the id of the object it paints), as one `Float32Array` plus a string table. JS paints it and nothing else; the same list serialised by Rust is the SVG, so the screen and the file cannot disagree. When the D2 SVG writer exists the list goes through it (one SVG writer in the workspace).
- **Input model**: JS forwards `pointerdown/move/up/cancel`, `wheel`, `keydown` as numbers (x, y in CSS px, pointer type, pressure, buttons, modifiers, key code); Rust owns the tool state machine, hit-testing (uniform grid over atoms and bond midpoints; radius 8 px mouse, 16 px pen, 22 px finger), snapping (30° steps, standard bond length), hover and the preview. Each event returns flags `{repaint, changed, cursor, announce}`. Target: event to display list < 1 ms for 500 atoms in wasm; pointer-to-paint within one frame.
- **Document model** `mzdraw/1` (below): objects with stable ids; a molecule object is a `mol/1` core plus what the record does not hold (2D coordinates as the user drew them, wedge marks as drawn, aromatic-circle display, abbreviation groups, R labels). Stereo follows the `mol.md` rule: the wedges are the drawing, the `stereo` elements are derived by the record's molfile rules at export, never the reverse.
- **Undo** = a command log in Rust. Every edit is a command with an exact inverse (the removed atoms and bonds are stored in the inverse); undo and redo are two stacks of commands, a drag is one command (coalesced). Property tested: any random command sequence then all its undos gives the identical document bytes, and redo gives the identical final bytes.
- **Command log = edit patch language.** The log is the JSON that `mol draw --set edit=` takes (D3). Until D3 is merged this is that language's draft: `[{"op":"atom.add","id":7,"el":"N","x":1.5,"y":0},{"op":"bond.add","id":9,"a":3,"b":7,"o":1},{"op":"atom.set","id":7,"chg":1},{"op":"move","atoms":[3,7],"dx":0.5,"dy":0},{"op":"remove","atoms":[7]},{"op":"ring","at":3,"n":6,"arom":true}]`. Ids are document ids, coordinates are in bond lengths (1.0 = one standard bond). D3 keeps or renames the ops; mzDraw follows, one language.
- **Determinism**: same document + same command log = same bytes from wasm, app and CLI (golden parity test). Floats are rounded to 4 decimals at the document boundary; no hash maps in serialised paths; no time, no randomness.
- **Accessibility**: the canvas is `role="application"` with a label; every tool is a real `<button>` with `aria-pressed`; full keyboard editing (a focus ring on one atom or bond, arrows move the focus along bonds, typing an element sets it, Enter/Shift+Enter grow a bond, digits set the order); every change is announced through an `aria-live="polite"` line built from i18n keys ("Atom N added, bonded to C3"); a text outline of the structure (formula, pieces, warnings) sits next to the canvas for screen readers; `prefers-reduced-motion` and forced-colours are honoured.
- **i18n**: Rust returns keys and parameters, never sentences; the page resolves them in `mzlab/web/lang/en/` (`draw2.*`). Tooltips are a name and a shortcut, nothing longer.

## Drawing checks (`std/1`)
A check never blocks and never changes the document. The list is `conform::CHECKS` (key, grade); the text of a key on the page is the i18n key `draw2.` + the key (`draw2.std.stereo.notCentre`). A possible stereocentre or stereo double bond is a site of `Mol::stereo_sites` of `mzlab-mol` on the record of the drawing (no second perception); sites on labelled atoms or on atoms that only the record has (an abbreviation's expansion) are left out. This is wider than the short atom list of the task text: it follows Table 8 of the InChI Technical Manual 1.07 as `mzlab-mol` encodes it (it also admits S and Se centres and strained N). Reading rules follow the Technical Manual: a wedge acts on its narrow end only; a wavy bond is an explicit "unknown". Thresholds are ours: 10 degrees.

| key | grade | what it flags |
|---|---|---|
| `std.stereo.wideEndCentre` | warn | solid or hashed wedge whose wide end is a possible stereocentre (the two InChI conventions read it differently) |
| `std.stereo.notCentre` | warn | wedge whose narrow end is not a possible stereocentre (not for the end of an allene) |
| `std.stereo.opposite` | warn | two solid or hashed marks at one centre on bonds within 10 degrees of 180 |
| `std.stereo.sameKindThree` | error | centre with three drawn neighbours and marks of one kind that do not fix the configuration: the Technical Manual construction (equal lengths, wide end of a solid wedge 45 degrees up, hashed 45 degrees down, implicit hydrogen on the other side) puts the centre outside the tetrahedron, which for same-kind marks means the centre is outside the triangle of its three neighbours |
| `std.stereo.collinear` | warn | two bonds of a centre within 10 degrees of one straight line, not both marked (two marks are `opposite`) |
| `std.stereo.wavyNotCentre` | info | wavy bond whose narrow end is no possible centre, and neither end is on a possible stereo double bond |
| `std.stereo.undefined` | info | possible centre with no mark at all (a wavy bond counts as a statement); possible stereo double bond with an end that has nothing drawn on it (a wavy bond counts as a statement; a double bond with substituents drawn is defined by its geometry) |
| `std.stereo.doubleCollinear` | warn | stereo double bond with a substituent within 10 degrees of its own line |
| `std.stereo.allene` | warn | allene (C with two double bonds and no other bond) with wedges at its ends that are not exactly two marks, one solid and one hashed, at one end |

Out of this slice: the five further stereo checks `fischerCross`, `reentrant`, `ringBond`, `hiddenH`, `relLabel` (follow-up prompt), the geometry checks (`std.geom.*`), export checks, CIP labels, automatic fixes. The older `check/1` of mzLab (valence, and a wedge from a non-stereocentre) stays as it is; the two lists overlap on that one case.

## Keyboard map
| keys | action |
|---|---|
| `C N O S P F H B I`, `Shift+C` Cl, `Shift+B` Br, `Shift+S` Si | element on the hovered/focused atom, or the atom tool |
| `1` `2` `3` | bond order (on a bond) or bond tool; `W` wedge, `Shift+W` hash, `Y` wavy, `A` aromatic |
| `+` `-` | charge; `.` radical; `Shift+I` isotope dialog |
| `R` then `3`-`8` | ring; `Shift+R` benzene; `X` chain |
| `V` / `Esc` select, `E` eraser, `T` text, `Shift+A` arrow, `L` label/abbreviation |
| arrows, `Enter`, `Shift+Enter`, `Tab` | move focus along bonds, grow a bond, grow a double bond, next piece |
| `Alt+Arrow`, `Alt+Shift+Arrow`, `Alt+[` `Alt+]`, `Shift+Arrow`, `M` | with a selection: move by 0.1 / 1 bond length, turn by 15°, extend the selection along the bonds, "Move here" mode |
| `Ctrl/Cmd+Z`, `Shift+Ctrl/Cmd+Z` or `Ctrl+Y` | undo, redo |
| `Ctrl/Cmd+A/C/X/V`, `Del`/`Backspace` | select all, copy, cut, paste, delete |
| `Ctrl/Cmd+0/+/-`, `Space`+drag, two-finger drag, pinch | zoom to fit, in, out, pan |
| `Ctrl/Cmd+L` | clean up; `Ctrl/Cmd+S` save, `Ctrl/Cmd+O` open |
Every binding comes from the settings file and can be changed there.

## Edit language (`draw_edit/1`)

Generated from `schema::json()` (`"out":"schema"`); the test `crates/mzdraw/tests/schema.rs` fails when this text drifts (`MZDRAW_BLESS=1` rewrites it). A field outside the row of its operation is an error (`error.draw2.opField`; before, unknown fields were ignored). The `mzlab` column is true by operation name only: mzLab reads some fields under other names (`label`, `wedge`, `x_a`) and addresses bonds by their atoms.

| op | since | mzdraw | mzlab | inverse | fields (`key` type, `*` required, unit, bounds) |
|---|---|---|---|---|---|
| `atom.add` | mzdraw/1 | true | true | DropAtom, Next | `id` uint (from the document counter when absent)<br>`el` string* (an element symbol)<br>`x` number* (bond length)<br>`y` number* (bond length) |
| `bond.add` | mzdraw/1 | true | true | DropBond, Next | `id` uint (from the document counter when absent)<br>`a` uint*<br>`b` uint*<br>`o` uint (0..=255, default 1)<br>`s` uint (0..=255, default 0) |
| `atom.set` | mzdraw/1 | true | true | SetAtom | `id` uint*<br>`el` string (an element symbol)<br>`chg` int (-128..=127)<br>`iso` uint (0..=65535)<br>`rad` uint (0..=255)<br>`h` uint (0..=255)<br>`lab` string (the empty text takes the label away) |
| `atom.data` | mzdraw/1 | true | false | SetAtom | `id` uint*<br>`data` string[][2]* (the empty list takes the data away) |
| `atom.ann` | mzdraw/1 | true | false | SetAtom | `id` uint*<br>`ann` json[]* (the empty list takes the annotation away) |
| `bond.set` | mzdraw/1 | true | true | SetBond | `id` uint*<br>`o` uint (0..=255)<br>`s` uint (0..=255)<br>`rev` bool (default false) |
| `rotate` | mzdraw/1 | true | false | SetAtom per atom | `atoms` uint[] (default empty)<br>`deg` number* (degree; counter-clockwise)<br>`x` number* (bond length)<br>`y` number* (bond length) |
| `flip` | mzdraw/1 | true | false | SetAtom per atom | `atoms` uint[] (default empty)<br>`axis` enum* (h, v)<br>`at` number* (bond length) |
| `move` | mzdraw/1 | true | true | SetAtom per atom, SetObj per object | `atoms` uint[] (default empty)<br>`objects` uint[] (default empty)<br>`dx` number* (bond length)<br>`dy` number* (bond length) |
| `remove` | mzdraw/1 | true | true | PutAtom, PutBond, PutObj, each back in place | `atoms` uint[] (default empty)<br>`bonds` uint[] (default empty)<br>`objects` uint[] (default empty) |
| `obj.add` | mzdraw/1 | true | false | DropObj, Next | `id` uint (from the document counter when absent)<br>`t` enum* (arrow, plus, text)<br>`x` number* (bond length)<br>`y` number* (bond length)<br>`x2` number (bond length; default 0)<br>`y2` number (bond length; default 0)<br>`text` string (default empty)<br>`from` uint (arrows: an atom of the structure the arrow leaves; 0 or absent is a free end)<br>`to` uint (arrows: an atom of the structure the arrow reaches; 0 or absent is a free end) |
| `obj.set` | mzdraw/1 | true | false | SetObj | `id` uint*<br>`text` string<br>`x2` number (bond length)<br>`y2` number (bond length)<br>`from` uint (an atom id; 0 frees the end)<br>`to` uint (an atom id; 0 frees the end) |
| `ring` | mzdraw/1 | true | false | DropAtom and DropBond per new item, Next | `n` uint* (0..=255 (the editor takes 3..=8))<br>`arom` bool (default false)<br>`atom` uint (a corner of the ring)<br>`bond` uint (the bond the ring is fused on)<br>`x` number (bond length; the centre, default 0)<br>`y` number (bond length; the centre, default 0) |
| `merge` |  | false | true |  | `from` uint*<br>`into` uint* |

The replay request (`mzlab-cli mol draw`) is one JSON object whose fields and `expect` arms are these rows (`request` section of the table); a field or an arm outside it is an invalid request (`error.draw2.requestField`, `error.draw2.expect`). With `report`, `dry_run` or a non-empty `expect` the answer is the document `draw-result/1` (section "Replay result").

| section | key | type | meaning |
|---|---|---|---|
| request | `import` | string | a structure text to start from: `mzdraw/1` JSON, a molfile or a SMILES |
| request | `style` | string\|object | the style of the SVG: a preset name or a settings file |
| request | `set` | string[] | settings applied after `style`, each `<section>.<key>=<value>` |
| request | `log` | json[] | the command log, operations of `draw_edit/1` |
| request | `out` | string | the output wanted (`mzdraw` by default): a format name, `log`, `std`, `schema` or `settings` |
| request | `commands` | object[] | editor commands `[{"command":NAME,"args":{...}}]` (move, rotate, flip, scheme:row), run after `log` and before `out`; the route of the anchored arrows follows; not with `command` |
| request | `command` | string | one editor command, with `args`: the same as a `commands` list of length one; not with `commands` |
| request | `args` | object | the fields of the operation of the same name as `command` (a rotate without x, y or a flip without at works about the centre of its atoms); `scheme:row` takes `gap` (bond lengths between neighbours, default 1.0, above 0), `wrap` (a width in bond lengths that breaks the row after an arrow, default 0: never) and `atoms`, `objects` (the selection, default: everything) and writes `move` and `obj.set` operations |
| request | `report` | bool | answer with one `draw-result/1` document instead of the bare output |
| request | `dry_run` | bool | run and report everything, write nothing: the document has no `out`; implies `report` |
| request | `expect` | object[] | postconditions on the final drawing, one arm each; a non-empty list implies `report` |
| request | `id` | string\|uint | echoed as the `id` of the document (default: the index of the request in its batch) |
| expect | `std` | string | `"none"`: the drawing has no `std/1` finding |
| expect | `std_absent` | string[] | none of these `std/1` keys is found |
| expect | `pieces` | uint | the atoms form exactly this many connected pieces |
| expect | `smiles` | string | the SMILES of the whole drawing equals this text |

## Replay result (`draw-result/1`)

A replay request with `"report":true`, `"dry_run":true` or a non-empty `"expect"` is answered by ONE JSON document `draw-result/1` (nothing else on stdout); without them the answer is the bare output as before. Code: `crates/mzdraw/src/outcome.rs` and `replay.rs` (`replay_at` also gives `ok` and takes the index `n` of the request in its batch); schema: `docs/schemas/draw-result-1.json`. Keys, always in this order; a key that does not apply is left out:

| key | type | when | meaning |
|---|---|---|---|
| `format` | string | always | `"draw-result/1"` |
| `id` | string or integer | always | the request's own `id`, else `n`, the 0-based index of the request in its batch (0 for a single request) |
| `ok` | bool | always | every op ran and every expectation holds |
| `dry_run` | bool | only when true | the request asked for a dry run |
| `patch` | array | with a goal (plan-27) | the canonical patch; not written by this slice |
| `out` | string | `ok` and not a dry run | what `Editor::save(out)` wrote (`out` of the request, default `mzdraw`) |
| `created` | object | always | `{"atoms":[ids],"bonds":[ids],"objects":[ids]}` = ids after minus before, ascending; empty lists left out, `{}` if none |
| `removed` | object | always | the same for ids before minus after |
| `failed` | object | the run stopped | `{"stage","op","key","id","candidates","rows"}`: `stage` is `log` (an op of the log), `select` (plan-24), `plan` (plan-27), `grant` (plan-28) or `command` (tp-01); `op` the 0-based index in `log` (or in `commands` for stage `command`); `key` the i18n key; `id` the id it is about (0 when none); `candidates` (stage `select`) the matched atom ids per match; `rows` (stage `command`, commands that read rows) `[{"line","key"}]`, last |
| `expect` | array | the request had `expect` | one `{"check":<the request object>,"ok","detail"}` per expectation, in request order; `detail` is empty when `ok`; after a failed log every entry is `ok:false`, "not checked" |

Exit codes (mzLab's table, no new code): 0 when `ok`; 1 when `ok` is false (an op or an editor command refused for this document (`failed.stage` `log` or `command`), or a false expectation; the document is on stdout; the example `replay` and `mzlab-cli` do this); 2 for an invalid request (not JSON, a field or an `expect` arm outside the table, an op outside the table, a malformed `id`, an unknown command, an unknown or missing argument of a command, a key of a `commands` item other than `command` and `args`; all checked before any command runs), which is the keyed error and no document; an unreadable `import` keeps its code. `created` and `removed` describe the attempted run even when `ok` is false; nothing is written then, and a dry run runs everything and writes nothing. `goal` (plan-27) will join the three fields that make the answer a document. Expectations: `{"std":"none"}`, `{"std_absent":[keys]}`, `{"pieces":N}`, `{"smiles":"..."}` (the request text is compared byte for byte with `Document::to_smiles()` of the whole drawing, which is what `Editor::save("smiles")` writes: it is not canonicalised, so another writing of the same molecule is false; `detail` gives the actual SMILES). An `out` that the drawing cannot give (`rxn` without an arrow, `smiles` of nothing) is the keyed error, exit 2, with or without `dry_run`, unless the run is already not ok (nothing is written then). The JSON Schema file is checked by hand in `tests/outcome.rs` (key order on the raw text, required keys, types, stages): no validator is a dependency.

## Own document format `mzdraw/1` (open, versioned, round-trips everything the editor draws)
JSON, compact, fixed field order, the `mol/1` conventions (empty fields omitted, same document = same bytes):
`{format:"mzdraw/1", style:<name or object>, objects:[...], meta:{...}}`. Object kinds: `mol` (`id`, `atoms` and `bonds` as in `mol/1` with a stable `id` each, `xy`, `marks` = wedge/hash/wavy/aromatic display per bond, `groups` = abbreviations and their expansion, `rlabels`, `ann`), `arrow` (kind, points, text above/below), `text` (position, runs with sub/superscript), `plus`.
- `ann` on a molecule is the per-structure annotation: `{level: "2b", evidence: [...], scores: [{name, v, from}], source}`. The level is a STRING of the Schymanski 2014 set `"1"`, `"2a"`, `"2b"`, `"3"`, `"4"`, `"5"` (owner decision; it contains the plain 1-5), never an integer; a string outside the set is kept as read and written back unchanged. Details of the fields: `mzdraw-formats.md` §5. The slot and its rendering place (a badge and caption under the structure) are reserved from the first version; the identification features fill it later.
- A format change bumps the number; the reader of version N reads every earlier version (fixtures kept in `crates/mzdraw/tests/data/`). Unknown fields are kept and written back, so a newer file survives an older editor.
- Autosave: the page writes the document to OPFS (`mzdraw/autosave.json`, debounced 1 s after the last change, through the glue) and offers it on the next visit.

## Format conformity matrix
R = read, W = write. "Slice" is when it lands; "who" names the code that owns it.
| format | R | W | lost or refused | slice / who |
|---|---|---|---|---|
| `mzdraw/1` | yes | yes | nothing | 1 / `mzdraw` |
| MOL V2000 | yes | yes | R: query atoms and bonds reported; W: arrows, text, styles, annotations; abbreviations expanded | 1 / `mzlab-mol` |
| MOL V3000 | yes | yes | as V2000; enhanced stereo groups kept | 1 / `mzlab-mol` |
| SDF | yes (every record, data fields into `meta`) | yes | as MOL; one record per structure | 3 / `sdf/1` |
| SMILES, CXSMILES | yes (laid out by D1) | yes | coordinates, wedge choice, arrows, text; `unknown` stereo not writable | 1 read with stub layout, 3 full |
| RXN, RDF | yes | yes | text and styles; arrow shape (always one straight arrow) | later / new reader on the molfile blocks |
| KET | yes | yes | macromolecule and query parts refused; our annotations not written | 3 / D4 writer, reader in `mzdraw` |
| CDXML | yes | yes | R: only structure, arrows, text, basic style (the open schema; no fidelity claim, `drawing-menu.md`); W: annotations as plain text | 3 / DRAW-CDXML reader, D4 writer |
| CDX (binary) | later | no | same subset as CDXML once the tag table is read; never written | later, to decide by measurement of the reader size |
| MRV (Marvin) | later | later | structure, arrows, text only | later |
| CML | later | later | structure only | later |
| XYZ, PDB | later | no | read as 3D atoms projected to 2D, bonds perceived by distance (PDB: `CONECT`); organic small molecules only | later, where sensible |
| InChI | no | later | no InChI reader is written (`mol.md`); writing needs an InChI engine outside the capped wasm | later / app and CLI only |
| SVG | no | yes | not re-readable; the `mzdraw/1` document can be embedded in `<metadata>` to make the SVG re-openable | 1 |
| PNG | no | yes | raster; same embedded document in a `tEXt` chunk | 3 (page: canvas raster of the SVG; CLI: `resvg` behind the `png` feature) |
| PDF | no | yes | vector, text as text; no embedded fonts beyond the base 14 | later / small own writer on the display list |
| clipboard | yes | yes | writes `text/plain` molfile, `image/svg+xml`, `web chemical/x-mdl-molfile`, own `web application/x-mzdraw+json`; reads the same plus SMILES text, KET and CDXML text | 2 |
Every reader returns an import report (`{read, lost: [{what, count}]}`) shown after opening: the user sees what did not come through.

## Customisation: one settings file `mzdraw-settings/1`
One JSON file shared by the editor, the page and the CLI (`mol draw --set style=FILE|NAME`): `{format, style, templates, shortcuts, layout, export}`.
- `style`: bond length (pt), line width, bold width, hash spacing, double-bond gap, margin around labels, font family and size, subscript scale, atom colours (per element or all black), background, label placement (auto/left/right), explicit-H policy, terminal-carbon labels, aromatic rings as circles or alternating bonds.
- Built-in presets: `mzlab` (screen default), `journal-1996` (the ACS Document 1996 numbers: bond length 14.4 pt, line width 0.6 pt, bold width 2 pt, hash spacing 2.5 pt, double-bond gap 18 % of the bond length, Arial/Helvetica 10 pt, fixed length), `slides` (thick lines, large labels), `dark`. A preset is a settings file like any other; the numbers are public journal guidelines, no resource of another program is copied (`drawing-menu.md`).
- `templates`: named `mzdraw/1` fragments with an attachment atom or bond (rings 3-8, benzene, chair, common protecting groups); `shortcuts`: key to command.
- `layout` (the palettes: preset, snap distance, the palettes that differ from the preset) and `export` (`default` format of Save, `background` `white` or `none`): `docs/mzdraw-ui.md` §5. The file is validated by a JSON schema generated from the Rust types.

## CLI integration (`mzlab-cli mol draw`, with D1-D3)
- Whatever the editor produces, the CLI produces with the same Rust functions and the same bytes: `mol draw IN|- --set format=svg|png|pdf|molfile|ket|cdxml|mzdraw --set style=... --set edit=LOG`, `--json` with stable keys, exit codes per `cli-agenti.md`, data on stdout, progress on stderr, `--dry-run` = parse + check without output. Batch: several inputs or a stream on stdin (one SMILES or one JSON document per line, one result per line), no browser anywhere in the path.
- Replaying an editor session: `mol draw DOC --set edit=LOG` applies the same command log the editor recorded; the golden parity test compares its bytes with the wasm editor's.
- Speed gates (native, release, one thread, criterion-style bench in `crates/mzdraw/benches`, numbers written to this file when measured): command apply < 5 µs; document to display list < 50 µs and display list to SVG < 50 µs for a 30-atom molecule; layout + SVG for a small molecule in the tens of µs to low hundreds (the reference engines measure 10-180 µs median for layout alone, `agent-draw.md`); 500 atoms to display list < 1 ms.

## Interaction design: what the other editors teach
| editor | worth taking | to avoid |
|---|---|---|
| ChemDraw | hotkeys on the hovered atom (type `N` over an atom), click a bond to cycle its order, drag from an atom to sprout a bond at 30° steps with fixed length, journal presets, chemistry-aware labels typed as text (`OMe`, `CO2H`) | no resource or wording is copied; CDX fidelity is not claimed |
| Ketcher | open KET format, template library, floating right-click menu | iframe + worker + 12 MB Indigo; clipboard traps, a paste that rezooms, `localStorage` without guards (`drawing.md`) |
| Marvin JS | abbreviation groups that expand/contract, clear valence-error underline, good import dialog | server-side services for clean-up and naming |
| JSME | one-click simplicity and tiny size (a single small script), works on touch | no undo depth, no reactions with text, dated look |
| Kekule.js, ChemDoodle Web | object model with separate render and IO layers; ChemDoodle's lasso and hover feedback | size, licences (GPL for ChemDoodle Web) |
| OpenChemLib editor | fast, compact clean-up; stereo recognition while drawing | it reorders atoms and drops explicit H (`mol.md` trap) |
| PubChem Sketcher, MolView | import by name, immediate SMILES/InChI round trip in the page | server round trips for every edit |
Rules taken: (1) everything on the canvas is reachable in one gesture and reversible with one undo; (2) hover shows what a click will do (preview in a lighter stroke); (3) a wrong valence is marked while drawing, never blocked ("the user decides"); (4) no modal dialog for atom properties: a small popover next to the atom, usable from the keyboard; (5) a tap target is 40 px on touch; pen = precise tool, finger = tool or pan by setting, palm rejected while the pen is near (as `touch.js`). The full competitor matrix comes from the architecture study and replaces this table when it exists.

## Build order and estimates (1 unit = one small PR, about half a day of an agent)
| slice | content | units |
|---|---|---|
| 1 | crate `mzdraw` (document, ids, atom/bond add/remove/move/set, ring templates, charge, undo/redo, command log, display list, hit-test, select/erase tools), `mzdraw-wasm` + cap, page `draw2/` (canvas, palette, SMILES/molfile in, molfile/SVG/`mzdraw/1` out, live `check/1` marks), unit + property + parity + e2e tests | 3 |
| 2 | copy/paste and clipboard formats, rotate/flip, wedge/hash/wavy with stereo through the record, labels and abbreviations, R1/R2, text, reaction arrow and plus, chains, touch and pen gestures, full keyboard editing, accessibility pass, settings file with presets, OPFS autosave | 4 |
| 3 | KET and CDXML in/out (D4, DRAW-CDXML), SDF, PNG, clean-up wired to D1, CIP marks, `mol draw` replay of the log (D3), speed bench in this file, comparison page next to Ketcher (test only) | 3 |
| later | RXN/RDF, MRV, CML, XYZ/PDB, CDX, PDF, shapes, template library editor, name search, annotations rendering with the identification features, the Drawing page panels (labels, analysis, fragments, cuts, mechanism, partials, evidence colours, substructure, links), `TPDraw` API, desktop window | 1-2 each |

## Limits stated
- The page rasterises PNG from the SVG with a canvas; the CLI uses `resvg`: parity is on SVG bytes, not PNG bytes.
- The OPFS autosave and the system clipboard exist only in the page; the CLI reads and writes files and stdin/stdout.
- Text metrics: label widths come from a fixed table of advance widths for the preset fonts in Rust, so the layout is identical in wasm, app and CLI; a font outside the table falls back to the table of the nearest generic family.
