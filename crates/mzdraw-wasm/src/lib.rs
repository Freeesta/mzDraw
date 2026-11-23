//! wasm-bindgen bridge of the mzDraw editor: one editor per page, driven by pointer, wheel and key events as numbers; the page paints
//! the display list (`scene` + `strings`) and shows what `state` says. All decisions are in the crate `mzdraw`; `replay` is the same
//! function the native side runs, for the parity test. With the `mzlab` feature `state` carries the `check/1` warnings of the drawing
//! (`mzlab_feat::chem::check`, the bytes of `mzlab-cli mol check`). Errors are JSON `{"error_key", "params"}` strings.
#[cfg(feature = "mzlab")]
pub mod glue;

use mzdraw::editor::{Editor, CHANGED};
use mzdraw::scene::Scene;
use mzdraw::Axis;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = mzdraw::brand::BRIDGE_ID;

#[derive(Default)]
struct Page {
    editor: Editor,
    scene: Scene,
    /// `check/1` of the drawing as last computed, or empty.
    check: String,
    /// The drawing checks as the panel lists them (`Report::counts_json`), recomputed after every change.
    std: String,
    /// Whether something is drawn and the checks found nothing (the page says so in words).
    std_none: bool,
}

/// A page with the editor connected to the engine that gives the masses (`glue::measure`, the engine `sel` of mzLab).
fn new_page() -> Page {
    let mut page = Page::default();
    #[cfg(feature = "sel")]
    {
        page.editor.measurer = Some(glue::measure);
    }
    page
}

thread_local! {
    static PAGE: RefCell<Page> = RefCell::new(new_page());
}

fn with<T>(f: impl FnOnce(&mut Page) -> T) -> T {
    PAGE.with(|p| f(&mut p.borrow_mut()))
}

fn js_err(key: &str) -> JsValue {
    JsValue::from_str(&format!("{{\"error_key\":\"{key}\",\"params\":{{}}}}"))
}

/// Runs an editor action; after a change the warnings are recomputed.
fn act(f: impl FnOnce(&mut Editor) -> u32) -> u32 {
    with(|p| {
        let flags = f(&mut p.editor);
        // an open proposal that an edit, an undo or a redo made stale is ended and said at once
        let flags = flags | p.editor.settle_proposal();
        if flags & CHANGED != 0 {
            recheck(p);
        }
        flags
    })
}

/// After a change: the drawing checks `std/1` (not the pathway findings: a full layout of 1000 nodes takes 73 ms in release, more than a
/// live check should; they are in `out: std`, `report` and `expect`) and, with the `mzlab` feature, the structure check.
fn recheck(p: &mut Page) {
    let report = mzdraw::conform::report(p.editor.doc());
    p.std = report.counts_json();
    p.std_none = !p.editor.doc().atoms.is_empty() && report.checks.is_empty();
    live_check(p);
    // the calculated values (formula, masses, m/z): the formulas come from the record, the numbers from the engine
    p.editor.refresh_values();
}

#[cfg(feature = "mzlab")]
fn live_check(p: &mut Page) {
    let doc = p.editor.doc();
    p.check.clear();
    let mut flagged = Vec::new();
    if let Some(j) = glue::check(doc) {
        let v: serde_json::Value = serde_json::from_str(&j).unwrap_or_default();
        let ids = glue::ids(doc);
        for w in v["warnings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|w| w["level"] == "warn")
        {
            for i in w["atoms"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|i| i.as_u64())
            {
                // check/1 counts atoms from 0 in molfile order, which is the pool order
                if let Some(id) = ids.get(i as usize) {
                    flagged.push(*id);
                }
            }
        }
        p.check = j;
    }
    p.editor.set_warnings(flagged);
}

#[cfg(not(feature = "mzlab"))]
fn live_check(_p: &mut Page) {}

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// The report of a bug or the suggestion of an improvement (`feedback/1`, made in Rust): `kind` is `bug` or `improvement`, `with_log`
/// puts the command log of the drawing in the issue, `agent` and `lang` are the browser's. The page only opens the `url` in a new tab.
#[wasm_bindgen]
pub fn feedback(kind: &str, with_log: bool, agent: &str, lang: &str) -> Result<String, JsValue> {
    with(|p| {
        let log = p.editor.save("log").map_err(js_err)?;
        let params = mzdraw::feedback::Params {
            kind,
            with_log,
            log: &log,
            agent,
            lang,
        };
        mzdraw::feedback::build(&params)
            .map(|f| f.json())
            .map_err(js_err)
    })
}

/// What the About box shows (`about/1`): the name, the version, the repository and the licence.
#[wasm_bindgen]
pub fn about() -> String {
    mzdraw::feedback::about_json()
}

/// The product name, from the one place that holds it.
#[wasm_bindgen]
pub fn name() -> String {
    mzdraw::NAME.to_string()
}

/// A pointer event: `phase` 0 down, 1 move, 2 up, 3 cancel; `kind` 0 mouse, 1 pen, 2 finger; `mods` bit 1 Shift, bit 2 Ctrl/Cmd,
/// bit 4 the eraser end of a pen, bit 8 Alt, bit 16 the secondary button (right button of a mouse, barrel button of a pen), bit 32 the middle
/// button; `id` the pointer id and `t` its time stamp in milliseconds (several fingers, a resting palm and a
/// long press are told apart in Rust). Returns flags (1 repaint, 2 changed, 4 tool changed, 8 handled, 512 open the context menu: a finger or a pen held still, the secondary button released without a drag, the barrel button of a pen; a drag with the
/// secondary or the middle button pans the view in Rust).
#[wasm_bindgen]
pub fn pointer(phase: u32, x: f64, y: f64, kind: u32, mods: u32, id: u32, t: f64) -> u32 {
    act(|e| e.pointer_event(phase, id, x, y, kind, mods, t))
}

/// A key press: `mods` bit 1 Shift, bit 2 Ctrl/Cmd, bit 8 Alt/Option; `t` the time stamp of the event in milliseconds (repeated
/// nudges within a second are one undo step).
#[wasm_bindgen]
pub fn key(key: &str, mods: u32, t: f64) -> u32 {
    act(|e| e.key_at(key, mods, t))
}

/// `what`: `undo`, `redo`, `clear`, `all` (select all), `delete` (the selection), `copy`, `cut`, `paste` (the editor's own
/// clipboard), `duplicate`, `rotate-l` / `rotate-r` (15 degrees), `flip-h`, `flip-v`, `scheme-row` (the one-line scheme layout), `pathway-tree` (arranges the pathway of anchored arrows as a tree), `align:left|centre|right|top|middle|bottom`, `distribute:h|v`, `rotate:<deg>`, `nudge:<dx>,<dy>` (arrange the selection, or everything), `scheme-orient` (each structure after an arrow turned onto the one before it), `move-here` (toggles the mode in which a click brings the selection there), or a tool name (`select`, `atom:N`, `bond:2`,
/// `bond:up`, `label:Ph`, `arrow`, `plus`, `text:<one line>`, ...).
#[wasm_bindgen]
pub fn command(what: &str) -> u32 {
    act(|e| match what {
        "undo" => e.undo(),
        "redo" => e.redo(),
        "clear" => e.clear(),
        "all" => e.select_all(),
        "delete" => e.delete_selection(),
        "copy" => e.copy() as u32,
        "cut" => e.cut(),
        "paste" => e.paste(None),
        "duplicate" => e.duplicate(),
        "rotate-l" => e.rotate(15.0),
        "rotate-r" => e.rotate(-15.0),
        "flip-h" => e.flip(Axis::H),
        "flip-v" => e.flip(Axis::V),
        "scheme-row" => e.scheme_row(),
        "pathway-tree" => e.pathway_tree(),
        "scheme-orient" => e.scheme_orient(),
        "pathway-label" => e.pathway_label_page(""),
        w if mzdraw::arrange::is_arrange(w) => e.arrange(w),
        w if w.starts_with("pathway-label:") => e.pathway_label_page(&w["pathway-label:".len()..]),
        "move-here" => e.set_move_here(!e.move_here),
        tool => e.set_tool(tool),
    })
}

/// The editor's clipboard as `mzdraw` (its document) or `molfile` (for other programs), for the system clipboard; empty when
/// nothing was copied.
#[wasm_bindgen]
pub fn clipboard(kind: &str) -> String {
    with(|p| p.editor.clipboard(kind))
}

/// Pastes a structure text that came from the system clipboard; a refusal is in `state().error`.
#[wasm_bindgen]
pub fn paste(text: &str) -> u32 {
    act(|e| e.paste(Some(text)))
}

#[wasm_bindgen]
pub fn zoom(x: f64, y: f64, factor: f64) -> u32 {
    act(|e| e.zoom(x, y, factor))
}

#[wasm_bindgen]
pub fn pan(dx: f64, dy: f64) -> u32 {
    act(|e| e.pan(dx, dy))
}

/// Tells the editor the size of the canvas in pixels, so that a paste lands at the view centre; the view does not change.
#[wasm_bindgen]
pub fn canvas(w: f64, h: f64) -> u32 {
    act(|e| {
        e.set_canvas(w, h);
        0
    })
}

#[wasm_bindgen]
pub fn fit(w: f64, h: f64) -> u32 {
    act(|e| e.fit(w, h))
}

/// Adds a structure text (SMILES, molfile or SDF, `mzdraw/1`) to the drawing; returns `{"format", "placeholder", "notes"}`, the
/// notes being what the page has to say about the import, as `[i18n key, count]` pairs.
#[wasm_bindgen]
pub fn add(text: &str) -> Result<String, JsValue> {
    add_bytes(text.as_bytes())
}

/// `add` for the bytes of a file: a text as above, or a PNG saved by mzDraw (it carries its document).
#[wasm_bindgen]
pub fn add_bytes(bytes: &[u8]) -> Result<String, JsValue> {
    with(|p| {
        let info = p.editor.import_bytes(bytes).map_err(js_err)?;
        recheck(p);
        let notes: Vec<String> = (info.notes().iter())
            .map(|(key, n)| format!("[\"{key}\",{n}]"))
            .collect();
        Ok(format!(
            "{{\"format\":\"{}\",\"placeholder\":{},\"notes\":[{}]}}",
            info.format,
            info.placeholder,
            notes.join(",")
        ))
    })
}

/// Selects what the findings of one drawing check are about (`key` of an entry of `state().std`); returns flags.
#[wasm_bindgen]
pub fn select_check(key: &str) -> u32 {
    act(|e| e.select_check(key))
}

/// What an autosaved document holds, as `{"atoms", "bonds", "objects"}`, or the empty text when it is not worth offering (not a
/// readable document, or an empty one). The page asks this before it offers to restore the drawing of the last visit.
#[wasm_bindgen]
pub fn saved_summary(text: &str) -> String {
    match mzdraw::io::saved_summary(text) {
        Some((atoms, bonds, objects)) => {
            format!("{{\"atoms\":{atoms},\"bonds\":{bonds},\"objects\":{objects}}}")
        }
        None => String::new(),
    }
}

/// The context menu of a right-click, a long press or the Menu key at (`x`, `y`) pixels, as JSON `{"target", "items": [{"id", "label",
/// "key"?, "enabled", "checked"?, "children"?}], "notice"?}` (`target`: atom, bond, selection, object or canvas). `kind` is the pointer
/// kind (0 mouse, 1 pen, 2 finger). The selection follows what is under the pointer, as a click makes it.
#[wasm_bindgen]
pub fn context_menu(x: f64, y: f64, kind: u32) -> String {
    with(|p| p.editor.context_menu_at(x, y, kind))
}

/// Runs an item of the open menu (one undo step) and closes the menu. Flags as `command`, plus 256 when the page has to finish the
/// job: read `context_ask`. A menu older than the last change refuses (`error.draw2.contextStale` in `state().error`).
#[wasm_bindgen]
pub fn context_run(id: &str) -> u32 {
    act(|e| e.context_run(id))
}

/// Whether the open menu still describes the state: false when none is open or when the document, the selection, the clipboard
/// or the view changed since it was built. The page closes its menu when this turns false.
#[wasm_bindgen]
pub fn context_valid() -> bool {
    with(|p| p.editor.context_valid())
}

/// Where a menu opened from the keyboard (flag 512 of `key`: the Menu key or Shift+F10) goes in a canvas `w` x `h`: `[x, y]` in
/// pixels, on the atom with the keyboard focus, else on the first selected atom or object, else in the middle.
#[wasm_bindgen]
pub fn context_anchor(w: f64, h: f64) -> Vec<f64> {
    with(|p| {
        let (x, y) = p.editor.context_anchor(w, h);
        vec![x, y]
    })
}

/// The step field (tp-11), as JSON `{"open", "text", "pick", "rows": [{"name", "delta", "kind", "mass"}], "step", "delta", "kind",
/// "sites", "site", "at", "error"}`. Opened by the context item `tp.step` (flag 2048 of `context_run`); the rest of
/// the field (the filter, the sites, `[` `]` `Enter` `Escape` while it is open) is decided in Rust.
#[wasm_bindgen]
pub fn step_field() -> String {
    with(|p| p.editor.step_json())
}

/// The text typed into the step field; the rows are filtered again (by name, delta and, with the engine, mass shift).
#[wasm_bindgen]
pub fn step_text(text: &str) -> u32 {
    act(|e| e.step_text(text))
}

/// A tap on row `i` of the list.
#[wasm_bindgen]
pub fn step_pick(i: u32) -> u32 {
    act(|e| e.step_pick(i as usize))
}

/// The "Derive" button: the chosen step at the focused site, one undo step.
#[wasm_bindgen]
pub fn step_derive() -> u32 {
    act(|e| e.step_derive())
}

/// Closes the step field.
#[wasm_bindgen]
pub fn step_close() -> u32 {
    act(|e| e.step_close())
}

/// Closes the menu without running anything.
#[wasm_bindgen]
pub fn context_close() {
    with(|p| p.editor.context_close())
}

/// What the page has to do after `context_run` answered 256: `element:<atom>`, `label:<atom>`, `data:<anchor>`, `annotation:<anchor>`,
/// `text:<object>`, `import`, `fit`, `save-image`, `clipboard` or `copy-as:<molfile|smiles|svg|png>`; read once.
#[wasm_bindgen]
pub fn context_ask() -> String {
    with(|p| p.editor.context_ask())
}

/// The selection as a text for the clipboard of another program: `molfile`, `smiles` or `svg`.
#[wasm_bindgen]
pub fn context_copy_as(kind: &str) -> Result<String, JsValue> {
    with(|p| p.editor.context_copy_as(kind).map_err(js_err))
}

/// `png_plan` of the selection; the bytes the page draws go through `context_png_finish`.
#[wasm_bindgen]
pub fn context_png_plan() -> String {
    with(|p| plan_json(p.editor.context_png_plan()))
}

/// `png_finish` for the selection: the PNG carries the selection as its document.
#[wasm_bindgen]
pub fn context_png_finish(png: &[u8]) -> Result<Vec<u8>, JsValue> {
    with(|p| {
        let e = &p.editor;
        mzdraw::png::finish(
            png,
            &e.context_selection_doc(),
            e.context_png_plan().density,
        )
        .ok_or_else(|| js_err("error.draw2.pngPlain"))
    })
}

/// Opens a saved `mzdraw/1` document in place of the drawing (same bytes, empty history).
#[wasm_bindgen]
pub fn open(text: &str) -> Result<u32, JsValue> {
    with(|p| {
        p.editor.open(text).map_err(js_err)?;
        recheck(p);
        Ok(mzdraw::editor::REPAINT | CHANGED)
    })
}

/// The drawing as `mzdraw` (the document), `molfile`, `molfile-v3000`, `smiles`, `ket`, `cdxml`, `cml`, `rxn`, `trasf`, `sdf`, `svg`, `log` (the command log), `std` (the drawing checks), `tptable` or `tptable-csv` (the TP table, with the masses of the engine) or `settings`.
#[wasm_bindgen]
pub fn save(kind: &str) -> Result<String, JsValue> {
    with(|p| {
        p.editor.save(kind).map_err(|key| {
            // a refused transformation names its atoms (the ids of the drawing) for the message
            match p.editor.trasf_problem().filter(|_| kind == "trasf") {
                Some((_, atoms)) if !atoms.is_empty() => {
                    let ids: Vec<String> = atoms.iter().map(u32::to_string).collect();
                    let params = serde_json::json!({ "atoms": ids.join(", ") });
                    JsValue::from_str(&format!(r#"{{"error_key":"{key}","params":{params}}}"#))
                }
                _ => js_err(key),
            }
        })
    })
}

/// What the last `save("tptable")` or `save("tptable-csv")` left empty or changed, as JSON `[[key, n], ...]` (the page says each with its text:
/// nothing is dropped in silence).
#[wasm_bindgen]
pub fn save_notes() -> String {
    with(|p| serde_json::to_string(&*p.editor.table_notes.borrow()).unwrap_or_else(|_| "[]".into()))
}

/// The drawing as a PDF file (vector, text as text, the document inside so that it opens again).
#[wasm_bindgen]
pub fn save_pdf() -> Vec<u8> {
    with(|p| p.editor.pdf())
}

/// What the page draws to save a PNG: `{"svg", "width", "height", "paper"}` (pixels; the SVG is rasterised on a canvas of that
/// size filled with the paper colour). The bytes of the canvas then go through `png_finish`.
#[wasm_bindgen]
pub fn png_plan() -> Result<String, JsValue> {
    with(|p| Ok(plan_json(p.editor.png_plan())))
}

fn plan_json(plan: mzdraw::png::Plan) -> String {
    format!(
        r#"{{"svg":{},"width":{},"height":{},"paper":"{}"}}"#,
        serde_json::Value::from(plan.svg),
        plan.width,
        plan.height,
        plan.paper
    )
}

/// The PNG the page drew, with the document inside (it opens again as a drawing) and its resolution.
#[wasm_bindgen]
pub fn png_finish(png: &[u8]) -> Result<Vec<u8>, JsValue> {
    with(|p| {
        let e = &p.editor;
        mzdraw::png::finish(png, e.doc(), e.png_plan().density)
            .ok_or_else(|| js_err("error.draw2.pngPlain"))
    })
}

/// The names of the built-in drawing styles.
#[wasm_bindgen]
pub fn styles() -> Vec<String> {
    mzdraw::settings::PRESETS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Sets the drawing style of the screen and of the saved SVG: a preset name or the text of a settings file `mzdraw-settings/1`.
#[wasm_bindgen]
pub fn set_style(text: &str) -> Result<u32, JsValue> {
    with(|p| {
        let flags = p.editor.set_style(text).map_err(js_err)?;
        p.editor.refresh_values();
        Ok(flags)
    })
}

/// How many atom maps `save(kind)` leaves out of the file because the format has none (0 for the formats that hold them).
#[wasm_bindgen]
pub fn maps_lost(kind: &str) -> u32 {
    with(|p| p.editor.doc().maps_lost(kind) as u32)
}

/// How many atoms and bonds with an explicit colour `save(kind)` leaves out of the file because the format has no place for it.
#[wasm_bindgen]
pub fn colours_lost(kind: &str) -> u32 {
    with(|p| p.editor.doc().colours_lost(kind) as u32)
}

/// How many atoms carry a choice of lines under their molecule that `save(kind)` leaves out because the format has no place for it.
#[wasm_bindgen]
pub fn captions_lost(kind: &str) -> u32 {
    with(|p| p.editor.doc().caps_lost(kind) as u32)
}

/// Adds (`on`) or takes away one line key of `props::LINES` in the choice kept under the molecule of the selection (or the focused
/// atom); one undo step. A refusal changes nothing (`error` in `state`).
#[wasm_bindgen]
pub fn toggle_line(key: &str, on: bool) -> u32 {
    act(|e| e.toggle_line(key, on))
}

/// Sets the whole choice from a JSON list of keys; the empty list gives the molecule back to `display.values`.
#[wasm_bindgen]
pub fn set_lines(keys: &str) -> u32 {
    act(|e| e.set_lines_json(keys))
}

/// `props/1` of the drawing as the editor holds it: the values and the choice of lines of every molecule and of the selection.
#[wasm_bindgen]
pub fn props() -> String {
    with(|p| p.editor.props_json())
}

/// The values copied as text in a form of `values::FORMS` (`text`, `tsv`, `line`); `""` when there is nothing to copy (see `error`).
#[wasm_bindgen]
pub fn copy_values(form: &str) -> String {
    with(|p| p.editor.copy_values(form).unwrap_or_default())
}

/// Sets the explicit colour of the selected atoms and bonds (or the focused atom): `#rgb` or `#rrggbb`, or the empty text for
/// automatic; one undo step. A text that is not a colour, or nothing to colour, changes nothing (`error` in `state`).
#[wasm_bindgen]
pub fn set_colour(text: &str) -> u32 {
    act(|e| e.colour(text))
}

/// A style button of the text panel: `kind` is `b`, `i`, `u`, `sub`, `sup` (toggles), `font` (a key of `state().text.fonts`), `size` (a
/// percent of `state().text.sizes`) or `colour`; it acts on the selected characters of a text (the whole text when it is selected and
/// not edited; the next typed characters when there is a caret and no range). One undo step; a refusal is in `state().error`.
#[wasm_bindgen]
pub fn text_style(kind: &str, value: &str) -> u32 {
    act(|e| e.text_style(kind, value))
}

/// Starts editing the selected text (Enter, F2 or the panel's Edit button), or ends it (`false`).
#[wasm_bindgen]
pub fn text_edit(on: bool) -> u32 {
    act(|e| if on { e.text_open() } else { e.text_close() })
}

/// Typed text, a paste or the result of an input method, put in place of the selected characters of the text being edited.
#[wasm_bindgen]
pub fn text_input(text: &str) -> u32 {
    act(|e| e.text_input(text))
}

/// Whether keys and the clipboard act on the characters of a text being edited.
#[wasm_bindgen]
pub fn text_editing() -> bool {
    with(|p| p.editor.text_editing())
}

/// A double click or a double tap at a point: a text under it is selected, edited and its word under the point selected.
#[wasm_bindgen]
pub fn text_double(x: f64, y: f64) -> u32 {
    act(|e| e.text_double(x, y))
}

/// The selected characters of the text being edited, for the clipboard (empty when none); `text_cut` removes them.
#[wasm_bindgen]
pub fn text_selected() -> String {
    with(|p| p.editor.text_selected())
}

/// Removes the selected characters of the text being edited (a cut).
#[wasm_bindgen]
pub fn text_cut() -> u32 {
    act(|e| e.text_delete(true))
}

/// The colours of the picker as JSON `[{"key", "name", "hex"}]`; the automatic colour is the empty text and is the page's first entry.
#[wasm_bindgen]
pub fn colour_palette() -> String {
    let rows: Vec<String> = (mzdraw::ink::PALETTE.iter())
        .map(|(k, n, h)| format!(r#"{{"key":"{k}","name":"{n}","hex":"{h}"}}"#))
        .collect();
    format!("[{}]", rows.join(","))
}

/// Sets the atom-to-atom map (1..=999, 0 for none) of the focused atom, or of the only selected one; a refusal changes nothing
/// (`error` in `state`).
#[wasm_bindgen]
pub fn set_map(text: &str) -> u32 {
    with(|p| p.editor.set_map_text(text))
}

/// Sets the data of the structure of the atom `anchor` (an `anchor` of the outline in `state`) from a JSON list of
/// `[name, value]` pairs; a list that is not of that shape, or that an SDF could not hold, changes nothing (`error` in `state`).
#[wasm_bindgen]
pub fn set_data(anchor: u32, json: &str) -> u32 {
    with(|p| {
        let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
        let rows = v.as_array().map(|rows| {
            rows.iter()
                .map(|r| {
                    Some((
                        r.get(0)?.as_str()?.to_string(),
                        r.get(1)?.as_str()?.to_string(),
                    ))
                })
                .collect::<Option<Vec<(String, String)>>>()
        });
        match rows.flatten() {
            Some(data) => p.editor.set_data(anchor, data),
            None => {
                p.editor.error = "error.draw2.data";
                mzdraw::editor::REPAINT
            }
        }
    })
}

/// Annotates the structure of the atom `anchor` with a new entry: a scale (`ann_schema` lists them; empty: the default) and a
/// confidence level (`ann_schema` lists those of each scale;
/// an empty one takes the annotation away), an identifier, the evidence as lines `kind: reference`, a note. An entry that is not
/// of its shape changes nothing (`error` in `state`).
#[wasm_bindgen]
pub fn annotate(
    anchor: u32,
    scale: &str,
    level: &str,
    identifier: &str,
    evidence: &str,
    note: &str,
) -> u32 {
    with(|p| {
        p.editor
            .annotate(anchor, scale, level, identifier, evidence, note)
    })
}

/// What an annotation can say, for a surface that builds its fields: `{"scale", "levels": [...], "scales": [{"name",
/// "levels"}], "kinds": [...]}` (`scale` and `levels` are the default scale and its levels). The levels to list for a chosen
/// scale are `ann_levels`.
#[wasm_bindgen]
pub fn ann_schema() -> String {
    let list = |l: &[&str]| format!(r#"["{}"]"#, l.join(r#"",""#));
    format!(
        r#"{{"scale":"{}","levels":{},"scales":[{}],"kinds":{}}}"#,
        mzdraw::ann::SCALE,
        list(&mzdraw::ann::LEVELS),
        (mzdraw::ann::SCALES.iter())
            .map(|s| format!(r#"{{"name":"{}","levels":{}}}"#, s.0, list(s.1)))
            .collect::<Vec<_>>()
            .join(","),
        list(&mzdraw::ann::KINDS)
    )
}

/// The levels to list for the scale `scale` and the current level `file_level` of a structure: `[{"level", "ofFile"}]`,
/// those of the scale first, then `file_level` flagged `ofFile` when it is not in the scale.
#[wasm_bindgen]
pub fn ann_levels(scale: &str, file_level: &str) -> String {
    let items: Vec<String> = (mzdraw::ann::level_list(scale, file_level).into_iter())
        .map(|(l, f)| {
            let level = serde_json::Value::from(l);
            format!(r#"{{"level":{level},"ofFile":{f}}}"#)
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// Sets one setting, `<section>.<key>=<value>` (for the style: `style.line_width=2`, `style.colour=mono`).
#[wasm_bindgen]
pub fn set(setting: &str) -> Result<u32, JsValue> {
    with(|p| {
        let flags = p.editor.set(setting).map_err(js_err)?;
        // the adducts and the mode of `display.values` change what is asked of the engine
        p.editor.refresh_values();
        Ok(flags)
    })
}

/// The schema of the settings file: presets, colours and the numeric fields with unit, bounds and default.
#[wasm_bindgen]
pub fn settings_schema() -> String {
    mzdraw::settings::schema_json()
}

/// The value of a setting as text (`<section>.<key>`), empty for one that is none: what the Preferences panel shows.
#[wasm_bindgen]
pub fn get(setting: &str) -> String {
    with(|p| p.editor.get(setting).unwrap_or_default())
}

/// The value for `data-theme` of the theme preference (`light`, `dark`), empty for `auto`: the page applies what the module says.
#[wasm_bindgen]
pub fn theme_attr() -> String {
    with(|p| p.editor.ui.attr().to_string())
}

/// `set_style` for the settings file kept in the browser: a bad `ui` object is dropped with a note and the rest is taken. The answer is
/// `{"flags": N, "note": KEY}` (`note` empty for none).
#[wasm_bindgen]
pub fn set_style_stored(text: &str) -> Result<String, JsValue> {
    with(|p| {
        let (flags, note) = p.editor.set_style_stored(text).map_err(js_err)?;
        Ok(format!(r#"{{"flags":{flags},"note":"{note}"}}"#))
    })
}

/// Puts a group of the preferences back to its defaults; the flags like `set`.
#[wasm_bindgen]
pub fn reset_group(group: &str) -> Result<u32, JsValue> {
    with(|p| {
        let flags = p.editor.reset_group(group).map_err(js_err)?;
        p.editor.refresh_values();
        Ok(flags)
    })
}

/// The current settings file: the style, the layout of the palettes and the export defaults.
#[wasm_bindgen]
pub fn style() -> String {
    with(|p| p.editor.settings_json())
}

/// The effective word of a setting that takes a word (`choices` of `settings_schema`): `right` for `pathway.dir` when the file says nothing.
#[wasm_bindgen]
pub fn choice(key: &str) -> String {
    with(|p| p.editor.style.choice(key).unwrap_or_default().to_string())
}

/// The palettes laid out in a viewport of `w` x `h` pixels, from the sizes the page measured (`sizes`: a JSON list of
/// `[id, width, height]`, or empty to keep the last ones). JSON as `Layout::arranged_json` writes it.
#[wasm_bindgen]
pub fn palettes(w: f64, h: f64, sizes: &str) -> String {
    let sizes = mzdraw::palettes::sizes_of(sizes);
    with(|p| p.editor.layout.arranged_json(w, h, &sizes))
}

/// An action on the palette `id`: `press`, `drag`, `release` at the page point (`x`, `y`) of its handle, `cancel`, a key on
/// its handle (`key:<name as the browser names it>`, Shift in `shift`), `hidden`, `collapsed`, `orient` (toggles), `reset`
/// (every palette), `all` (hide or show every palette). Returns `PALETTES` (128) when the layout changed, else 0.
#[wasm_bindgen]
pub fn palette(action: &str, id: &str, x: f64, y: f64, shift: bool) -> u32 {
    with(|p| {
        let l = &mut p.editor.layout;
        let changed = match action {
            "press" => l.press(id, x, y),
            "drag" => l.drag(x, y),
            "release" => l.release(x, y),
            "cancel" => l.cancel(),
            "reset" => {
                l.reset();
                true
            }
            "all" => {
                l.all_hidden = !l.all_hidden;
                true
            }
            a => match a.strip_prefix("key:") {
                Some(k) => l.key(id, &k.to_ascii_lowercase(), shift),
                None => l.toggle(id, a),
            },
        };
        if changed {
            mzdraw::editor::PALETTES
        } else {
            0
        }
    })
}

/// What one click on Save writes: `{"default": <format>, "background": "white" | "none"}` of the settings.
#[wasm_bindgen]
pub fn export_settings() -> String {
    with(|p| p.editor.export.to_json())
}

/// The labels the label tool takes: the abbreviations of the table, then `R` and `R1` ... `R9` (up to `R32` can be typed).
#[wasm_bindgen]
pub fn labels() -> Vec<String> {
    let groups = mzdraw::label::ABBREVIATIONS.iter().map(|g| g.0.to_string());
    let r = std::iter::once("R".to_string()).chain((1..=9).map(|n| format!("R{n}")));
    groups.chain(r).collect()
}

/// The display list numbers (layout in `mzdraw::scene`); call `strings` after it for the texts.
#[wasm_bindgen]
pub fn scene() -> Vec<f32> {
    with(|p| {
        p.scene = p.editor.scene();
        p.scene.data.clone()
    })
}

#[wasm_bindgen]
pub fn strings() -> Vec<String> {
    with(|p| p.scene.strings.clone())
}

/// What the page shows around the canvas: `{tool, style, outline: [{formula, atoms, bonds, anchor, data, ann: {level, known, identifier, evidence, note, entries} | null}], focus: {id, name, index, of, bonds, hydrogens, charge} | null, atoms, bonds, objects, undo, redo, selected, moveHere, clip, say: {key, what}, notice: {key, what}, error, check, std, proposal, values: {adducts, error, structures: [{anchor, atoms, formula, charge, composition, mass, average, ionMz, mz, error}], selection: {of, atoms, formula, ...} | null}}` (`check` is
/// the `check/1` document or null; `std` is `{none, lines}`: `lines` lists the drawing checks that have findings, `[{key, grade, n}]`, as `Editor::save("std")` reports them, and `none` is true when something is drawn and there are none).
#[wasm_bindgen]
pub fn state() -> String {
    with(|p| {
        // a selection is not a change of the drawing: its numbers are asked for here
        p.editor.refresh_selection_values();
        let e = &p.editor;
        // written by hand: building the state as a JSON value cost about 2 kB of module more
        let q = |s: &str| serde_json::Value::from(s).to_string();
        // the diff lens, one pass and only when it is on: the words of each product by its anchor, and the note when it skips arrows
        let lens = (e.style.lens != mzdraw::settings::LensMode::Off)
            .then(|| mzdraw::tp::LensState::of(e.doc()));
        let lens_note = lens
            .as_ref()
            .and_then(|l| l.note)
            .map_or("null".to_string(), |(k, n)| {
                format!(r#"{{"key":{},"n":{n}}}"#, q(k))
            });
        let outline: Vec<String> = mzdraw::outline::pieces(e.doc())
            .into_iter()
            .map(|s| {
                let data: Vec<String> = (s.data.iter())
                    .map(|(k, v)| format!("[{},{}]", q(k), q(v)))
                    .collect();
                let ann = s.ann.map_or("null".to_string(), |a| {
                    format!(
                        r#"{{"scale":{},"level":{},"known":{},"identifier":{},"evidence":{},"note":{},"entries":{}}}"#,
                        q(&a.scale),
                        q(&a.level),
                        a.known,
                        q(&a.identifier),
                        q(&a.evidence),
                        q(&a.note),
                        a.entries
                    )
                });
                let lens = lens
                    .as_ref()
                    .and_then(|l| l.line(s.anchor))
                    .map_or("null".to_string(), |x| {
                        format!(
                            r#"{{"from":{},"delta":{},"cleaved":{}}}"#,
                            q(&x.from),
                            q(&x.delta),
                            x.cleaved
                        )
                    });
                format!(
                    r#"{{"formula":{},"atoms":{},"bonds":{},"anchor":{},"data":[{}],"ann":{ann},"lens":{lens}}}"#,
                    q(&s.formula),
                    s.atoms,
                    s.bonds,
                    s.anchor,
                    data.join(",")
                )
            })
            .collect();
        let focus = e.over.focus.and_then(|a| mzdraw::outline::focus(e.doc(), a)).map_or("null".into(), |f| {
            format!(
                r#"{{"id":{},"name":{},"index":{},"of":{},"bonds":{},"hydrogens":{},"charge":{}}}"#,
                f.id,
                q(&f.name),
                f.index,
                f.of,
                f.bonds,
                f.hydrogens,
                f.charge
            )
        });
        let selected = e.over.sel_atoms.len() + e.over.sel_bonds.len() + e.over.sel_objs.len();
        let colour = e.selection_colour().map_or("null".to_string(), |c| q(&c));
        let lines = e.selection_lines().map_or("null".to_string(), |l| {
            format!("[{}]", l.iter().map(|k| q(k)).collect::<Vec<_>>().join(","))
        });
        format!(
            r#"{{"tool":{},"outline":[{}],"focus":{focus},"style":{},"lens":{},"lensNote":{lens_note},"atoms":{},"bonds":{},"objects":{},"undo":{},"redo":{},"selected":{selected},"colour":{colour},"lines":{lines},"text":{},"moveHere":{},"clip":{},"say":{{"key":{},"what":{},"params":{{{}}}}},"notice":{{"key":{},"what":{}}},"error":{},"check":{},"std":{{"none":{},"lines":{}}},"proposal":{},"values":{},"labels":{}}}"#,
            q(&e.tool.name()),
            outline.join(","),
            q(&e.style.name),
            q(e.style.lens.name()),
            e.doc().atoms.len(),
            e.doc().bonds.len(),
            e.doc().objects.len(),
            e.session.can_undo(),
            e.session.can_redo(),
            e.text_json(),
            e.move_here,
            !e.clipboard(mzdraw::brand::SLUG).is_empty(),
            q(e.said.0),
            q(&e.said.1),
            (e.said_params.iter())
                .filter(|_| matches!(e.said.0, "draw2.say.pathway" | "draw2.say.labelled"))
                .map(|(k, n)| format!("\"{k}\":{n}"))
                .collect::<Vec<_>>()
                .join(","),
            q(e.notice.0),
            q(&e.notice.1),
            q(e.error),
            if p.check.is_empty() { "null" } else { &p.check },
            p.std_none,
            if p.std.is_empty() { "[]" } else { &p.std },
            e.proposal_json(),
            e.values_json(),
            e.labels_json(),
        )
    })
}

/// Holds a patch (a JSON list of `draw_edit/1` operations, from an agent or a file) as a proposal over the drawing: it is not in the
/// drawing and not in the log until a hunk is accepted. A text that is not a list of operations is `error.draw2.proposalPatch`; the
/// other refusals are the keys of `Editor::propose`. Returns flags (repaint, proposal). `state().proposal` lists the hunks.
#[wasm_bindgen]
pub fn propose(patch: &str) -> Result<u32, JsValue> {
    let bad = || js_err("error.draw2.proposalPatch");
    let v: serde_json::Value = serde_json::from_str(patch).map_err(|_| bad())?;
    let ops = (v.as_array().ok_or_else(bad)?.iter())
        .map(mzdraw::Op::parse)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| bad())?;
    with(|p| {
        p.editor.propose(ops).map_err(|e| js_err(e.key))?;
        Ok(mzdraw::editor::REPAINT | mzdraw::editor::PROPOSAL)
    })
}

/// An action on the proposal: `accept`, `reject` (the hunk with the keyboard), `accept-all`, `reject-all`, `next`, `prev`, `accept:N`,
/// `reject:N`, `select:N` (hunk numbers from 1). Flags as `command` (an accept is a change), plus 1024 for the proposal; a refusal is
/// in `state().error`.
#[wasm_bindgen]
pub fn proposal(what: &str) -> u32 {
    act(|e| e.proposal_action(what))
}

/// Headless with engines: a replay request plus `"engines": [{"engine", "args"?}]` gives `{"engines": [...]}` (`glue::replay`), the same bytes
/// as the native example `replay_glue`.
#[cfg(feature = "mzlab")]
#[wasm_bindgen]
pub fn replay_glue(request: &str) -> Result<String, JsValue> {
    glue::replay(request).map_err(|e| JsValue::from_str(&e.to_json()))
}

/// Headless: `{"import", "log", "out"}` gives the output text, the same bytes as the native `mzdraw::replay::replay`.
#[wasm_bindgen]
pub fn replay(request: &str) -> Result<String, JsValue> {
    mzdraw::replay::replay(request).map_err(|e| {
        JsValue::from_str(&format!(
            "{{\"error_key\":\"{}\",\"params\":{{\"id\":{}}}}}",
            e.key, e.id
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn the_annotation_schema_lists_the_scales_and_their_levels() {
        let v: Value = serde_json::from_str(&ann_schema()).unwrap();
        assert_eq!(v["scale"], "schymanski-2014");
        assert_eq!(v["levels"], json!(["1", "2a", "2b", "3", "4", "5"]));
        assert_eq!(v["scales"][0]["name"], "schymanski-2014");
        assert_eq!(v["scales"][0]["levels"], v["levels"]);
        assert_eq!(v["scales"][1]["name"], "charbonnet-2022");
        assert_eq!(
            v["scales"][1]["levels"],
            json!(["1a", "1b", "2a", "2b", "2c", "3a", "3b", "3c", "3d", "4", "5a", "5b"])
        );
        assert_eq!(v["scales"].as_array().unwrap().len(), 2);
        assert!(v["kinds"].as_array().unwrap().contains(&json!("formula")));
    }

    #[test]
    fn the_levels_to_list_follow_the_scale_and_flag_the_level_of_the_file() {
        let list = |s: &str, l: &str| serde_json::from_str::<Value>(&ann_levels(s, l)).unwrap();
        let pfas = list("charbonnet-2022", "");
        assert_eq!(pfas.as_array().unwrap().len(), 12);
        assert!(pfas
            .as_array()
            .unwrap()
            .iter()
            .all(|x| x["ofFile"] == false));
        // a level of the chosen scale is not "of the file"; one that is not in it is, once, last
        assert_eq!(list("charbonnet-2022", "3b"), pfas);
        let other = list("schymanski-2014", "3b");
        assert_eq!(other.as_array().unwrap().len(), 7);
        assert_eq!(other[6], json!({"level": "3b", "ofFile": true}));
        assert_eq!(other[0], json!({"level": "1", "ofFile": false}));
        // a scale of no table has only the level of the file
        assert_eq!(list("mine", "x"), json!([{"level": "x", "ofFile": true}]));
    }
}
