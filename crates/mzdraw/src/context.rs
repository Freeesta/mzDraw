//! The context menu: the most important options for what is under the pointer. `Editor::context_menu(x, y)` hit-tests, settles the
//! selection (a thing that is not part of a larger selection becomes the selection, as a click would make it) and returns the
//! menu as JSON `{"target", "items": [{"id", "label", "key"?, "enabled", "checked"?, "children"?}], "notice"?}`. `Editor::context_run(id)`
//! runs one item through the commands the page already has (one undo step), and closes the menu. Every item is an existing
//! command (an `Op` of the `draw_edit/1` log, the clipboard, the selection, the view), so the CLI route exists for all of them.
//!
//! `key` is the key the page binds to the same function, shown as a hint. It is the key of the item's action for Ctrl+Z / Y / A / D
//! and Del; Ctrl+C / X / V are clipboard events of the page, and the item does the same through `context_run` (the page mirrors the
//! editor's clipboard to the system one on the answer `clipboard`). For the keys that only pick a tool (1-3, W, Shift+W, Y, + and -
//! with no focused atom, the element letters) the menu item applies the choice to the atom or bond under the pointer at once: the key
//! is the hint of the same choice, not of the same gesture.
//!
//! Items the editor cannot finish alone (a field to fill, a file to write, the size of the canvas) answer with the flag `ASK` and
//! leave what the page has to do in `context_ask()`: `element:<atom>`, `label:<atom>`, `data:<anchor>`, `annotation:<anchor>` (the
//! anchor is the first atom of the structure, as `outline` gives it),
//! `text:<object>`, `import`, `fit`, `save-image`, `clipboard` (mirror the editor's clipboard to the system one) and
//! `copy-as:<molfile|smiles|svg|png>` (the text comes from `context_copy_as`, the PNG plan from `context_png_plan`).
//!
//! A menu is a picture of the state at the click: it remembers a fingerprint of the document, the selection, the clipboard and the
//! view, and an item run after any of them changed is refused (`error.draw2.contextStale`), so a stale menu cannot act on an old target.
//! The view is part of the fingerprint: a pan or a zoom between opening and running closes the menu, as a change of the document does.

use crate::doc::{ObjKind, WEDGE_DOWN, WEDGE_UP, WEDGE_WAVY};
use crate::editor::{Editor, Hit, ASK, CHANGED, MOUSE, REPAINT};
use crate::ops::{Axis, Op};
use serde_json::{json, Value};
use std::hash::{Hash, Hasher};

/// The i18n key of the line shown instead of an export on an empty canvas.
pub const EMPTY_NOTICE: &str = "draw2.ctx.empty";

/// The charge of an atom is kept within +-4 (`Op::AtomSet`): the charge items are disabled at the limit.
const CHARGE_MAX: i8 = 4;

/// The elements of the quick list, with their keys.
const ELEMENTS: [(&str, &str); 9] = [
    ("C", "C"),
    ("N", "N"),
    ("O", "O"),
    ("S", "S"),
    ("P", "P"),
    ("F", "F"),
    ("Cl", "Shift+C"),
    ("Br", "Shift+B"),
    ("I", "I"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Atom(u32),
    Bond(u32),
    Selection,
    Object(u32),
    Canvas,
}

impl Target {
    fn name(self) -> &'static str {
        match self {
            Target::Atom(_) => "atom",
            Target::Bond(_) => "bond",
            Target::Selection => "selection",
            Target::Object(_) => "object",
            Target::Canvas => "canvas",
        }
    }
}

/// A menu that is open: what it is about, where it was opened (pixels), the ids it offers enabled, the state it was built from.
struct Open {
    target: Target,
    at: (f64, f64),
    ids: Vec<String>,
    print: u64,
}

#[derive(Default)]
pub(crate) struct Context {
    open: Option<Open>,
    ask: String,
}

fn finite(v: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

fn item(id: &str, label: &str, key: &str, enabled: bool) -> Value {
    let mut v = json!({"id": id, "label": label, "enabled": enabled});
    if !key.is_empty() {
        v["key"] = json!(key);
    }
    v
}

fn checked(mut v: Value, on: bool) -> Value {
    v["checked"] = json!(on);
    v
}

fn group(id: &str, label: &str, children: Vec<Value>) -> Value {
    json!({"id": id, "label": label, "enabled": true, "children": children})
}

/// The ids of the items that can be run: enabled leaves.
fn runnable(items: &[Value], out: &mut Vec<String>) {
    for i in items.iter().filter(|i| i["enabled"] == true) {
        match i["children"].as_array() {
            Some(kids) => runnable(kids, out),
            None => out.extend(i["id"].as_str().map(str::to_string)),
        }
    }
}

impl Editor {
    /// The menu for a mouse pointer at (`x`, `y`) pixels; see the module text.
    pub fn context_menu(&mut self, x: f64, y: f64) -> String {
        self.context_menu_at(x, y, MOUSE)
    }

    /// `context_menu` for a pointer of the given kind (a finger reaches further).
    pub fn context_menu_at(&mut self, x: f64, y: f64, kind: u32) -> String {
        // a number the page could not parse must not reach the view (the zoom item keeps this point)
        let (x, y) = (finite(x), finite(y));
        self.cancel_press();
        let hit = self.hit(x, y, kind);
        let target = self.context_target(hit);
        let empty = self.doc().is_empty();
        let items = match target {
            Target::Atom(a) => self.atom_items(a),
            Target::Bond(b) => self.bond_items(b),
            Target::Selection => self.selection_items(),
            Target::Object(o) => self.object_items(o),
            Target::Canvas => self.canvas_items(),
        };
        let mut ids = Vec::new();
        runnable(&items, &mut ids);
        self.ctx.open = Some(Open {
            target,
            at: (x, y),
            ids,
            print: self.print(),
        });
        self.ctx.ask.clear();
        let mut menu = json!({"target": target.name(), "items": items});
        if target == Target::Canvas && empty {
            menu["notice"] = json!(EMPTY_NOTICE);
        }
        menu.to_string()
    }

    /// Whether the open menu still describes the state: false when none is open or when the document, the selection, the clipboard
    /// or the view changed since it was built. The page closes its menu when this turns false.
    pub fn context_valid(&self) -> bool {
        self.ctx
            .open
            .as_ref()
            .is_some_and(|o| o.print == self.print())
    }

    /// Where a menu opened from the keyboard goes, in pixels of a canvas `w` x `h`: on the atom with the keyboard focus, else on
    /// the first selected atom or object, else in the middle of the canvas.
    pub fn context_anchor(&self, w: f64, h: f64) -> (f64, f64) {
        let d = self.doc();
        let atom = (self
            .over
            .focus
            .into_iter()
            .chain(self.over.sel_atoms.first().copied()))
        .find_map(|a| d.atom(a));
        if let Some(a) = atom {
            return self.view.to_px(a.x, a.y);
        }
        match self.over.sel_objs.first().and_then(|&i| d.object(i)) {
            Some(o) => self.view.to_px(o.x, o.y),
            None => (finite(w) / 2.0, finite(h) / 2.0),
        }
    }

    /// The first atom of the structure `atom` belongs to: the anchor its data and annotation are kept on.
    fn anchor_of(&self, atom: u32) -> u32 {
        (crate::layout::pieces(self.doc()).into_iter())
            .find(|p| p.contains(&atom))
            .and_then(|p| p.first().copied())
            .unwrap_or(atom)
    }

    /// Closes the menu without running anything.
    pub fn context_close(&mut self) {
        self.ctx.open = None;
    }

    /// What the page has to do after an item answered `ASK` (see the module text); read once.
    pub fn context_ask(&mut self) -> String {
        std::mem::take(&mut self.ctx.ask)
    }

    /// The selection as a text for the clipboard of another program: `molfile`, `smiles` or `svg`.
    pub fn context_copy_as(&self, kind: &str) -> Result<String, &'static str> {
        let d = self.selection_doc();
        if d.atoms.is_empty() {
            return Err("error.draw2.request");
        }
        Ok(match kind {
            "molfile" => d.to_molfile(),
            "smiles" => d.to_smiles()?,
            "svg" => crate::svg::carrying(crate::svg::to_svg_styled(&d, &self.style), &d),
            _ => return Err("error.draw2.request"),
        })
    }

    /// The PNG plan (`png::plan`) of the selection; the document it carries is `selection_doc`.
    pub fn context_png_plan(&self) -> crate::png::Plan {
        crate::png::plan(&self.selection_doc(), &self.style)
    }

    /// The selection as a document of its own, as `context_png_plan` draws it.
    pub fn context_selection_doc(&self) -> crate::doc::Document {
        self.selection_doc()
    }

    /// Runs an item of the open menu as one undo step; the menu closes. Returns the flags of the editor, with `ASK` when the page
    /// has to finish the job. An item that is not offered enabled does nothing; a menu older than the last change is refused.
    pub fn context_run(&mut self, id: &str) -> u32 {
        let Some(open) = self.ctx.open.take() else {
            return 0;
        };
        self.ctx.ask.clear();
        if open.print != self.print() {
            self.error = "error.draw2.contextStale";
            return REPAINT;
        }
        if !open.ids.iter().any(|i| i == id) {
            return 0;
        }
        let atom = match open.target {
            Target::Atom(a) => a,
            _ => 0,
        };
        let f = match id {
            "edit.undo" => self.undo(),
            "edit.redo" => self.redo(),
            "edit.copy" => match self.copy() {
                true => self.ask("clipboard".into()),
                false => 0,
            },
            "edit.cut" => match self.cut() {
                f if f & CHANGED != 0 => f | self.ask("clipboard".into()),
                f => f,
            },
            "edit.paste" => self.paste(None),
            "edit.duplicate" => self.duplicate(),
            "edit.delete" => self.delete_selection(),
            "edit.all" => self.select_all(),
            "bond.delete" => match open.target {
                Target::Bond(b) => self.run(
                    &[Op::Remove {
                        atoms: Vec::new(),
                        bonds: vec![b],
                        objects: Vec::new(),
                    }],
                    "draw2.say.erased",
                    "",
                ),
                _ => 0,
            },
            "bond.reverse" => match open.target {
                Target::Bond(b) => self.run(
                    &[Op::BondSet {
                        id: b,
                        o: None,
                        s: None,
                        rev: true,
                    }],
                    "draw2.say.wedge",
                    "",
                ),
                _ => 0,
            },
            "atom.connected" => self.select_piece(atom),
            "atom.chg.plus" => self.add_charge(atom, 1),
            "atom.chg.minus" => self.add_charge(atom, -1),
            "atom.el.more" => self.ask(format!("element:{atom}")),
            "atom.label" => self.ask(format!("label:{atom}")),
            "atom.data" => self.ask(format!("data:{}", self.anchor_of(atom))),
            "atom.ann" => self.ask(format!("annotation:{}", self.anchor_of(atom))),
            "obj.edit" => match open.target {
                Target::Object(o) => self.ask(format!("text:{o}")),
                _ => 0,
            },
            "pathway.anchor" | "pathway.free" => match open.target {
                Target::Object(o) => {
                    let r = if id == "pathway.free" {
                        self.pathway_free(o)
                    } else {
                        self.pathway_anchor(o)
                    };
                    self.pathway_flags(r)
                }
                _ => 0,
            },
            "tp.derive" => self.tp_derive(if atom != 0 { Some(atom) } else { None }),
            "pathway.route" => {
                let r = self.pathway_route();
                self.pathway_flags(r)
            }
            "sel.rotate.l" => self.rotate(15.0),
            "sel.rotate.r" => self.rotate(-15.0),
            "sel.flip.h" => self.flip(Axis::H),
            "sel.flip.v" => self.flip(Axis::V),
            "pathway.tree" => self.pathway_tree(),
            "canvas.add" => self.ask("import".into()),
            "canvas.fit" => self.ask("fit".into()),
            "canvas.save" => self.ask("save-image".into()),
            "canvas.zoom.in" => self.zoom(open.at.0, open.at.1, 1.25),
            "canvas.zoom.out" => self.zoom(open.at.0, open.at.1, 0.8),
            "canvas.lens" => self.cycle_lens(),
            "canvas.clear" => self.clear(),
            _ => self.run_choice(open.target, id),
        };
        f | REPAINT
    }

    /// The ids of the families `atom.el.<symbol>`, `bond.o.<order>`, `bond.s.<mark>` and `sel.copyas.<kind>`.
    fn run_choice(&mut self, target: Target, id: &str) -> u32 {
        if let Target::Atom(a) = target
            && let Some(el) = id.strip_prefix("atom.el.")
            && mzlab_mol::is_element(el)
        {
            let same = self.doc().atom(a).is_some_and(|x| x.el == el);
            let set = Op::AtomSet {
                map: None,
                id: a,
                el: Some(el.into()),
                chg: None,
                iso: None,
                rad: None,
                h: None,
                lab: None,
            };
            return if same {
                0
            } else {
                self.run(&[set], "draw2.say.element", el)
            };
        }
        if let Target::Bond(b) = target {
            let (now, mark) = self.doc().bond(b).map_or((1, 0), |x| (x.o, x.s));
            if let Some(o) = id
                .strip_prefix("bond.o.")
                .and_then(|o| o.parse::<u8>().ok())
            {
                if (now, mark) == (o, 0) {
                    return 0;
                }
                let set = Op::BondSet {
                    id: b,
                    o: Some(o),
                    s: Some(0),
                    rev: false,
                };
                return self.run(&[set], "draw2.say.order", &o.to_string());
            }
            let s = match id.strip_prefix("bond.s.") {
                Some("up") => WEDGE_UP,
                Some("down") => WEDGE_DOWN,
                Some("wavy") => WEDGE_WAVY,
                _ => 0,
            };
            if s != 0 && mark != s {
                let set = Op::BondSet {
                    id: b,
                    o: Some(1),
                    s: Some(s),
                    rev: false,
                };
                return self.run(&[set], "draw2.say.wedge", "");
            }
        }
        match id.strip_prefix("sel.copyas.") {
            Some(kind @ ("molfile" | "smiles" | "svg" | "png")) => {
                self.ask(format!("copy-as:{kind}"))
            }
            _ => 0,
        }
    }

    fn ask(&mut self, what: String) -> u32 {
        self.ctx.ask = what;
        ASK
    }

    fn add_charge(&mut self, atom: u32, by: i8) -> u32 {
        let chg = self.doc().atom(atom).map_or(0, |a| a.chg) + by;
        let set = Op::AtomSet {
            map: None,
            id: atom,
            el: None,
            chg: Some(chg),
            iso: None,
            rad: None,
            h: None,
            lab: None,
        };
        self.run(&[set], "draw2.say.charge", &chg.to_string())
    }

    /// Selects the connected piece the atom belongs to.
    fn select_piece(&mut self, atom: u32) -> u32 {
        let piece = (crate::layout::pieces(self.doc()).into_iter())
            .find(|p| p.contains(&atom))
            .unwrap_or_default();
        let bonds = (self.doc().bonds.iter())
            .filter(|b| piece.contains(&b.a) && piece.contains(&b.b))
            .map(|b| b.id)
            .collect();
        self.over.sel_bonds = bonds;
        self.over.sel_atoms = piece;
        self.over.sel_objs.clear();
        self.said = ("draw2.say.selected", String::new());
        REPAINT
    }

    /// What the menu is about. A thing that is part of a larger selection keeps that selection (`Selection`); any other thing
    /// becomes the selection, as a click makes it, and an empty spot clears the selection.
    fn context_target(&mut self, hit: Hit) -> Target {
        let sorted = |o: &crate::scene::Overlay| {
            let mut s = (o.sel_atoms.clone(), o.sel_bonds.clone(), o.sel_objs.clone());
            s.0.sort_unstable();
            s.1.sort_unstable();
            s.2.sort_unstable();
            s
        };
        let o = &self.over;
        let before = (o.sel_atoms.clone(), o.sel_bonds.clone(), o.sel_objs.clone());
        let inside = match hit {
            Hit::Atom(a) => o.sel_atoms.contains(&a),
            Hit::Bond(b) => o.sel_bonds.contains(&b),
            Hit::Obj(i) => o.sel_objs.contains(&i),
            Hit::None => false,
        };
        let was = sorted(o);
        self.select_hit(hit);
        if inside && sorted(&self.over) != was {
            (self.over.sel_atoms, self.over.sel_bonds, self.over.sel_objs) = before;
            return Target::Selection;
        }
        match hit {
            Hit::Atom(a) => Target::Atom(a),
            Hit::Bond(b) => Target::Bond(b),
            Hit::Obj(i) => Target::Object(i),
            Hit::None => Target::Canvas,
        }
    }

    /// A fingerprint of what a menu depends on: the document, the selection, the clipboard and the view.
    fn print(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.doc().to_json().hash(&mut h);
        let o = &self.over;
        (&o.sel_atoms, &o.sel_bonds, &o.sel_objs, &self.clip).hash(&mut h);
        [self.view.scale, self.view.ox, self.view.oy]
            .map(f64::to_bits)
            .hash(&mut h);
        h.finish()
    }

    fn atom_items(&self, a: u32) -> Vec<Value> {
        let now = self.doc().atom(a).map_or("", |x| x.el.as_str());
        let chg = self.doc().atom(a).map_or(0, |x| x.chg);
        let piece = (crate::layout::pieces(self.doc()).into_iter())
            .find(|p| p.contains(&a))
            .map_or(0, |p| p.len());
        let mut elements: Vec<Value> = ELEMENTS
            .iter()
            .map(|&(el, key)| checked(item(&format!("atom.el.{el}"), el, key, true), el == now))
            .collect();
        elements.push(item("atom.el.more", "More...", "", true));
        let mut items = vec![
            group("atom.element", "Element", elements),
            item("atom.chg.plus", "Charge +", "+", chg < CHARGE_MAX),
            item("atom.chg.minus", "Charge -", "-", chg > -CHARGE_MAX),
            item("atom.label", "Label or abbreviation...", "L", true),
            item("atom.data", "Data...", "", true),
            item("atom.ann", "Annotation...", "", true),
            item("edit.copy", "Copy", "Ctrl+C", true),
            item("edit.cut", "Cut", "Ctrl+X", true),
            item("edit.delete", "Delete", "Del", true),
            item("atom.connected", "Select connected", "", piece > 1),
        ];
        if self.style.pathway_derive {
            items.insert(6, item("tp.derive", "Derive TP", "Shift+D", true));
        }
        items
    }

    fn bond_items(&self, b: u32) -> Vec<Value> {
        let (o, s) = self.doc().bond(b).map_or((1, 0), |x| (x.o, x.s));
        vec![
            checked(item("bond.o.1", "Single", "1", true), (o, s) == (1, 0)),
            checked(item("bond.o.2", "Double", "2", true), (o, s) == (2, 0)),
            checked(item("bond.o.3", "Triple", "3", true), (o, s) == (3, 0)),
            checked(item("bond.s.up", "Wedge", "W", true), s == WEDGE_UP),
            checked(
                item("bond.s.down", "Hash", "Shift+W", true),
                s == WEDGE_DOWN,
            ),
            checked(item("bond.s.wavy", "Wavy", "Y", true), s == WEDGE_WAVY),
            item("bond.reverse", "Reverse direction", "", s != 0),
            item("bond.delete", "Delete", "Del", true),
        ]
    }

    fn selection_items(&self) -> Vec<Value> {
        let atoms = !self.over.sel_atoms.is_empty();
        let doc = self.selection_doc();
        // a selection of bonds alone holds nothing to copy: atoms and objects are what the clipboard keeps
        let copyable = !doc.is_empty();
        let smiles = atoms && doc.to_smiles().is_ok();
        let copy_as = vec![
            item("sel.copyas.molfile", "Molfile", "", atoms),
            item("sel.copyas.smiles", "SMILES", "", smiles),
            item("sel.copyas.svg", "SVG", "", atoms),
            item("sel.copyas.png", "PNG", "", atoms),
        ];
        let mut items = vec![
            item("edit.cut", "Cut", "Ctrl+X", copyable),
            item("edit.copy", "Copy", "Ctrl+C", copyable),
            item("edit.paste", "Paste", "Ctrl+V", !self.clip.is_empty()),
            item("edit.duplicate", "Duplicate", "Ctrl+D", copyable),
            item("sel.rotate.l", "Rotate left", "", atoms),
            item("sel.rotate.r", "Rotate right", "", atoms),
            item("sel.flip.h", "Flip horizontal", "", atoms),
            item("sel.flip.v", "Flip vertical", "", atoms),
            item("edit.delete", "Delete", "Del", true),
            group("sel.copyas", "Copy as", copy_as),
        ];
        if self.style.pathway_derive {
            items.insert(5, item("tp.derive", "Derive TP", "Shift+D", atoms));
        }
        items
    }

    fn object_items(&self, o: u32) -> Vec<Value> {
        let text = self.doc().object(o).is_some_and(|x| x.t != ObjKind::Plus);
        let arrow = self.doc().object(o).filter(|x| x.t == ObjKind::Arrow);
        let mut items = vec![item("obj.edit", "Edit", "", text)];
        if let Some(a) = arrow {
            items.push(item(
                "pathway.anchor",
                "Anchor to structures",
                "",
                !self.doc().atoms.is_empty(),
            ));
            items.push(item(
                "pathway.free",
                "Free ends",
                "",
                a.from != 0 || a.to != 0,
            ));
        }
        items.push(item("edit.delete", "Delete", "Del", true));
        items
    }

    fn canvas_items(&self) -> Vec<Value> {
        let any = !self.doc().is_empty();
        // an arrow that is anchored at both ends: what a pathway is made of (a refusal of the layout, such as a cycle, is said in words)
        let anchored =
            (self.doc().objects.iter()).any(|o| o.t == ObjKind::Arrow && o.from != 0 && o.to != 0);
        let mut items = vec![
            item("edit.paste", "Paste", "Ctrl+V", !self.clip.is_empty()),
            item("edit.all", "Select all", "Ctrl+A", any),
            item("pathway.tree", "Arrange pathway", "", anchored),
            item("canvas.add", "Add structure from text...", "", true),
            item("edit.undo", "Undo", "Ctrl+Z", self.session.can_undo()),
            item("edit.redo", "Redo", "Ctrl+Shift+Z", self.session.can_redo()),
            item("canvas.fit", "Fit", "", true),
            item("canvas.zoom.in", "Zoom in", "", true),
            item("canvas.zoom.out", "Zoom out", "", true),
        ];
        // nothing to export on an empty canvas: the menu carries a notice instead (`context_menu`)
        if any {
            items.push(item("canvas.save", "Save image", "", true));
        }
        let anchored =
            (self.doc().objects.iter()).any(|o| o.t == ObjKind::Arrow && o.from != 0 && o.to != 0);
        items.push(item("pathway.route", "Route arrows", "", anchored));
        // the diff lens has something to show only where an arrow joins a parent to a product; each press goes to the next mode
        if anchored {
            items.push(checked(
                item("canvas.lens", "Diff lens", "Shift+L", true),
                self.style.lens != crate::settings::LensMode::Off,
            ));
        }
        items.push(item("canvas.clear", "Clear all", "", any));
        items
    }
}
