//! wasm-bindgen bridge of the mzDraw editor: one editor per page, driven by pointer, wheel and key events as numbers; the page paints
//! the display list (`scene` + `strings`) and shows what `state` says. All decisions are in the crate `mzdraw`; `replay` is the same
//! function the native side runs, for the parity test. With the `mzlab` feature `state` carries the `check/1` warnings of the drawing
//! (`mzlab_feat::chem::check`, the bytes of `mzlab-cli mol check`). Errors are JSON `{"error_key", "params"}` strings.
use mzdraw::editor::{Editor, CHANGED};
use mzdraw::scene::Scene;
use mzdraw::Axis;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "mzdraw/1";

#[derive(Default)]
struct Page {
    editor: Editor,
    scene: Scene,
    /// `check/1` of the drawing as last computed, or empty.
    check: String,
}

thread_local! {
    static PAGE: RefCell<Page> = RefCell::new(Page::default());
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
        if flags & CHANGED != 0 {
            recheck(p);
        }
        flags
    })
}

#[cfg(feature = "mzlab")]
fn recheck(p: &mut Page) {
    let doc = p.editor.doc();
    p.check.clear();
    let mut flagged = Vec::new();
    if !doc.atoms.is_empty() {
        let molfile = serde_json::Value::from(doc.to_molfile_for_check());
        let request = format!(r#"{{"molfile":{molfile}}}"#);
        if let Ok(j) = mzlab_feat::chem::check::check_json(&request) {
            let v: serde_json::Value = serde_json::from_str(&j).unwrap_or_default();
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
                    if let Some(a) = doc.atoms.get(i as usize) {
                        flagged.push(a.id);
                    }
                }
            }
            p.check = j;
        }
    }
    p.editor.set_warnings(flagged);
}

#[cfg(not(feature = "mzlab"))]
fn recheck(_p: &mut Page) {}

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// The product name, from the one place that holds it.
#[wasm_bindgen]
pub fn name() -> String {
    mzdraw::NAME.to_string()
}

/// A pointer event: `phase` 0 down, 1 move, 2 up, 3 cancel; `kind` 0 mouse, 1 pen, 2 finger; `mods` bit 1 Shift, bit 2 Ctrl/Cmd,
/// bit 4 the eraser end of a pen; `id` the pointer id and `t` its time stamp in milliseconds (several fingers, a resting palm and a
/// long press are told apart in Rust). Returns flags (1 repaint, 2 changed, 4 tool changed, 8 handled).
#[wasm_bindgen]
pub fn pointer(phase: u32, x: f64, y: f64, kind: u32, mods: u32, id: u32, t: f64) -> u32 {
    act(|e| e.pointer_event(phase, id, x, y, kind, mods, t))
}

#[wasm_bindgen]
pub fn key(key: &str, mods: u32) -> u32 {
    act(|e| e.key(key, mods))
}

/// `what`: `undo`, `redo`, `clear`, `all` (select all), `delete` (the selection), `copy`, `cut`, `paste` (the editor's own
/// clipboard), `duplicate`, `rotate-l` / `rotate-r` (15 degrees), `flip-h`, `flip-v`, or a tool name (`select`, `atom:N`, `bond:2`,
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

/// Opens a saved `mzdraw/1` document in place of the drawing (same bytes, empty history).
#[wasm_bindgen]
pub fn open(text: &str) -> Result<u32, JsValue> {
    with(|p| {
        p.editor.open(text).map_err(js_err)?;
        recheck(p);
        Ok(mzdraw::editor::REPAINT | CHANGED)
    })
}

/// The drawing as `mzdraw` (the document), `molfile`, `molfile-v3000`, `smiles`, `ket`, `cdxml`, `cml`, `rxn`, `svg` or `log` (the command log).
#[wasm_bindgen]
pub fn save(kind: &str) -> Result<String, JsValue> {
    with(|p| p.editor.save(kind).map_err(js_err))
}

/// What the page draws to save a PNG: `{"svg", "width", "height", "paper"}` (pixels; the SVG is rasterised on a canvas of that
/// size filled with the paper colour). The bytes of the canvas then go through `png_finish`.
#[wasm_bindgen]
pub fn png_plan() -> Result<String, JsValue> {
    with(|p| {
        let plan = p.editor.png_plan();
        Ok(format!(
            r#"{{"svg":{},"width":{},"height":{},"paper":"{}"}}"#,
            serde_json::Value::from(plan.svg),
            plan.width,
            plan.height,
            plan.paper
        ))
    })
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
    with(|p| p.editor.set_style(text).map_err(js_err))
}

/// The current style as a settings file.
#[wasm_bindgen]
pub fn style() -> String {
    with(|p| p.editor.style.to_json())
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

/// What the page shows around the canvas: `{tool, style, outline: [{formula, atoms, bonds}], focus: {id, name, index, of, bonds, hydrogens, charge} | null, atoms, bonds, objects, undo, redo, selected, clip, say: {key, what}, error, check}` (`check` is
/// the `check/1` document or null).
#[wasm_bindgen]
pub fn state() -> String {
    with(|p| {
        let e = &p.editor;
        // written by hand: building the state as a JSON value cost about 2 kB of module more
        let q = |s: &str| serde_json::Value::from(s).to_string();
        let outline: Vec<String> = mzdraw::outline::pieces(e.doc())
            .into_iter()
            .map(|s| {
                format!(
                    r#"{{"formula":{},"atoms":{},"bonds":{}}}"#,
                    q(&s.formula),
                    s.atoms,
                    s.bonds
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
        format!(
            r#"{{"tool":{},"outline":[{}],"focus":{focus},"style":{},"atoms":{},"bonds":{},"objects":{},"undo":{},"redo":{},"selected":{selected},"clip":{},"say":{{"key":{},"what":{}}},"error":{},"check":{}}}"#,
            q(&e.tool.name()),
            outline.join(","),
            q(&e.style.name),
            e.doc().atoms.len(),
            e.doc().bonds.len(),
            e.doc().objects.len(),
            e.session.can_undo(),
            e.session.can_redo(),
            !e.clipboard("mzdraw").is_empty(),
            q(e.said.0),
            q(&e.said.1),
            q(e.error),
            if p.check.is_empty() { "null" } else { &p.check },
        )
    })
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
