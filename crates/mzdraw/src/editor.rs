//! The interactive editor: a session, a view, a tool and the pointer state machine. The page forwards pointer, wheel and key events
//! as numbers and paints the display list; every decision (what is under the pointer, what a click does) is taken here.

use crate::doc::{Document, ObjKind, WEDGE_DOWN, WEDGE_UP, WEDGE_WAVY};
use crate::io;
use crate::m;
use crate::ops::{Op, Session};
use crate::scene::{self, Overlay, Scene, View};
use crate::settings::Style;

/// Flags an event returns.
pub const REPAINT: u32 = 1;
pub const CHANGED: u32 = 2;
pub const TOOL: u32 = 4;
pub const HANDLED: u32 = 8;
/// The page should move the keyboard to its label field.
pub const ASK_LABEL: u32 = 16;
/// The page should move the keyboard to its text field.
pub const ASK_TEXT: u32 = 32;
/// The keyboard focus moved or what it is on changed: the page should say where it is.
pub const FOCUS: u32 = 64;

/// Modifier bits of an event.
pub const SHIFT: u32 = 1;
pub const CTRL: u32 = 2;

/// Pointer kinds: mouse, pen, finger. A finger needs a wider reach.
pub const MOUSE: u32 = 0;
pub const PEN: u32 = 1;
pub const TOUCH: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    Atom(String),
    /// Order 1-3 and wedge mark 0-3 (a mark only on order 1).
    Bond(u8, u8),
    Erase,
    Ring(u8, bool),
    Charge(i8),
    /// An abbreviation or an R group of `label`.
    Label(String),
    /// A reaction arrow, dragged or of a standard length.
    Arrow,
    Plus,
    /// A line of text to place, or to set on a text or an arrow.
    Text(String),
}

impl Tool {
    /// `select`, `atom:N`, `bond:2`, `bond:up` / `bond:down` / `bond:wavy` (wedge marks), `erase`, `ring:6`, `ring:6a` (alternating
    /// double bonds), `charge:+`, `charge:-`, `label:Ph` / `label:R1` (an abbreviation or an R group of `label`), `arrow`, `plus`,
    /// `text:<one line>`.
    pub fn parse(s: &str) -> Option<Tool> {
        let (name, arg) = s.split_once(':').unwrap_or((s, ""));
        Some(match name {
            "select" => Tool::Select,
            "erase" => Tool::Erase,
            "atom" if mzlab_mol::is_element(arg) => Tool::Atom(arg.into()),
            "bond" => match arg {
                "up" => Tool::Bond(1, WEDGE_UP),
                "down" => Tool::Bond(1, WEDGE_DOWN),
                "wavy" => Tool::Bond(1, WEDGE_WAVY),
                _ => Tool::Bond(arg.parse().ok().filter(|o| (1..=3).contains(o))?, 0),
            },
            "ring" => {
                let n = arg
                    .trim_end_matches('a')
                    .parse()
                    .ok()
                    .filter(|n| (3..=8).contains(n))?;
                Tool::Ring(n, arg.ends_with('a'))
            }
            "charge" => Tool::Charge(if arg == "-" { -1 } else { 1 }),
            "label" if crate::label::kind(arg).is_some() => Tool::Label(arg.into()),
            "arrow" => Tool::Arrow,
            "plus" => Tool::Plus,
            "text"
                if !arg.trim().is_empty() && arg.chars().count() <= 200 && !arg.contains('\n') =>
            {
                Tool::Text(arg.into())
            }
            _ => return None,
        })
    }

    pub fn name(&self) -> String {
        match self {
            Tool::Select => "select".into(),
            Tool::Erase => "erase".into(),
            Tool::Atom(e) => format!("atom:{e}"),
            Tool::Bond(_, WEDGE_UP) => "bond:up".into(),
            Tool::Bond(_, WEDGE_DOWN) => "bond:down".into(),
            Tool::Bond(_, WEDGE_WAVY) => "bond:wavy".into(),
            Tool::Bond(o, _) => format!("bond:{o}"),
            Tool::Ring(n, a) => format!("ring:{n}{}", if *a { "a" } else { "" }),
            Tool::Charge(c) => format!("charge:{}", if *c < 0 { "-" } else { "+" }),
            Tool::Label(l) => format!("label:{l}"),
            Tool::Arrow => "arrow".into(),
            Tool::Plus => "plus".into(),
            Tool::Text(t) => format!("text:{t}"),
        }
    }
}

/// What is under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    None,
    Atom(u32),
    Bond(u32),
    /// An arrow, a plus sign or a text.
    Obj(u32),
}

/// The ids of what an import or a paste added.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Placed {
    pub atoms: Vec<u32>,
    pub bonds: Vec<u32>,
    pub objects: Vec<u32>,
}

/// A press in progress: where it started (pixels), what it hit, whether it became a drag.
pub(crate) struct Press {
    pub start: (f64, f64),
    pub now: (f64, f64),
    pub hit: Hit,
    pub moved: bool,
    pub kind: u32,
}

pub struct Editor {
    pub session: Session,
    pub view: View,
    pub tool: Tool,
    pub over: Overlay,
    pub(crate) press: Option<Press>,
    /// What the last change did, as an i18n key with one parameter, for the screen reader line.
    pub said: (&'static str, String),
    /// The last refused operation, an i18n key.
    pub error: &'static str,
    /// The atom the keyboard focus came from (the bond walked last).
    pub(crate) focus_from: Option<u32>,
    /// The drawing style: what the screen and the saved SVG look like. Not part of the document.
    pub style: Style,
    /// The editor's own clipboard: an `mzdraw/1` document of what was copied, and how many times it was pasted since.
    pub(crate) clip: String,
    pub(crate) pasted: u32,
}

impl Default for Editor {
    fn default() -> Self {
        Editor {
            session: Session::default(),
            view: View {
                scale: 40.0,
                ox: 200.0,
                oy: 200.0,
            },
            tool: Tool::Bond(1, 0),
            over: Overlay::default(),
            press: None,
            said: ("", String::new()),
            error: "",
            focus_from: None,
            style: Style::default(),
            clip: String::new(),
            pasted: 0,
        }
    }
}

impl Editor {
    pub fn doc(&self) -> &Document {
        &self.session.doc
    }

    /// Runs one undo step; a refusal is kept in `error` and changes nothing.
    pub(crate) fn run(&mut self, ops: &[Op], say: &'static str, what: &str) -> u32 {
        match self.session.run(ops) {
            Ok(()) => {
                self.said = (say, what.to_string());
                self.error = "";
                self.prune();
                REPAINT | CHANGED
            }
            Err(e) => {
                self.error = e.key;
                REPAINT
            }
        }
    }

    /// Drops selection, hover and flags that point at things that no longer exist.
    fn prune(&mut self) {
        let d = &self.session.doc;
        self.over.sel_atoms.retain(|&i| d.atom(i).is_some());
        self.over.sel_bonds.retain(|&i| d.bond(i).is_some());
        self.over.sel_objs.retain(|&i| d.object(i).is_some());
        self.over.focus = self.over.focus.filter(|&i| d.atom(i).is_some());
        self.focus_from = self.focus_from.filter(|&i| d.atom(i).is_some());
        self.over.hover_obj = self.over.hover_obj.filter(|&i| d.object(i).is_some());
        self.over.warn_atoms.retain(|&i| d.atom(i).is_some());
        self.over.hover_atom = self.over.hover_atom.filter(|&i| d.atom(i).is_some());
        self.over.hover_bond = self.over.hover_bond.filter(|&i| d.bond(i).is_some());
    }

    pub fn set_tool(&mut self, name: &str) -> u32 {
        match Tool::parse(name) {
            Some(t) => {
                self.tool = t;
                self.press = None;
                self.over.preview = None;
                self.over.band = None;
                REPAINT | TOOL
            }
            None => 0,
        }
    }

    pub fn undo(&mut self) -> u32 {
        if !self.session.undo() {
            return 0;
        }
        self.said = ("draw2.say.undo", String::new());
        self.prune();
        REPAINT | CHANGED
    }

    pub fn redo(&mut self) -> u32 {
        if !self.session.redo() {
            return 0;
        }
        self.said = ("draw2.say.redo", String::new());
        self.prune();
        REPAINT | CHANGED
    }

    /// Reach of the pointer in pixels.
    fn reach(kind: u32) -> f64 {
        match kind {
            TOUCH => 22.0,
            PEN => 16.0,
            _ => 10.0,
        }
    }

    /// The nearest atom within reach, else the nearest bond.
    pub fn hit(&self, px: f64, py: f64, kind: u32) -> Hit {
        let r = Self::reach(kind);
        let d = self.doc();
        let mut best = (r, Hit::None);
        for a in &d.atoms {
            let (x, y) = self.view.to_px(a.x, a.y);
            let dist = m::hypot(x - px, y - py);
            if dist < best.0 {
                best = (dist, Hit::Atom(a.id));
            }
        }
        if best.1 != Hit::None {
            return best.1;
        }
        best.0 = r * 0.7;
        for b in &d.bonds {
            let (Some(p), Some(q)) = (d.atom(b.a), d.atom(b.b)) else {
                continue;
            };
            let (s, e) = (self.view.to_px(p.x, p.y), self.view.to_px(q.x, q.y));
            let (dx, dy) = (e.0 - s.0, e.1 - s.1);
            let t = (((px - s.0) * dx + (py - s.1) * dy) / (dx * dx + dy * dy).max(1e-9))
                .clamp(0.0, 1.0);
            let dist = m::hypot(s.0 + t * dx - px, s.1 + t * dy - py);
            if dist < best.0 {
                best = (dist, Hit::Bond(b.id));
            }
        }
        if best.1 != Hit::None {
            return best.1;
        }
        self.hit_object(px, py, r)
    }

    /// The object under the pointer: near the shaft of an arrow, on a plus sign, or on the line of a text.
    fn hit_object(&self, px: f64, py: f64, r: f64) -> Hit {
        let font = self.style.metrics(self.view.scale, true).font;
        for o in self.doc().objects.iter().rev() {
            let (x, y) = self.view.to_px(o.x, o.y);
            let on = match o.t {
                ObjKind::Arrow => {
                    let e = self.view.to_px(o.x2, o.y2);
                    let (dx, dy) = (e.0 - x, e.1 - y);
                    let t = (((px - x) * dx + (py - y) * dy) / (dx * dx + dy * dy).max(1e-9))
                        .clamp(0.0, 1.0);
                    m::hypot(x + t * dx - px, y + t * dy - py) < r
                }
                ObjKind::Plus => m::hypot(x - px, y - py) < r + font * 0.4,
                ObjKind::Text => {
                    px >= x - r
                        && px <= x + scene::width(&o.text, font) + r
                        && (py - y).abs() <= font * 0.6 + r * 0.5
                }
            };
            if on {
                return Hit::Obj(o.id);
            }
        }
        Hit::None
    }

    /// Zooms by `factor` keeping the point under the pointer still (scale 8-240 px per bond).
    pub fn zoom(&mut self, px: f64, py: f64, factor: f64) -> u32 {
        let (x, y) = self.view.to_doc(px, py);
        self.view.scale = (self.view.scale * factor).clamp(8.0, 240.0);
        self.view.ox = px - x * self.view.scale;
        self.view.oy = py + y * self.view.scale;
        REPAINT
    }

    pub fn pan(&mut self, dx: f64, dy: f64) -> u32 {
        self.view.ox += dx;
        self.view.oy += dy;
        REPAINT
    }

    /// Centres the drawing in a `w` x `h` canvas, at most 40 px per bond.
    pub fn fit(&mut self, w: f64, h: f64) -> u32 {
        let Some((x0, y0, x1, y1)) = self.doc().bounds() else {
            self.view = View {
                scale: 40.0,
                ox: w / 2.0,
                oy: h / 2.0,
            };
            return REPAINT;
        };
        let scale = ((w - 80.0) / (x1 - x0).max(0.1))
            .min((h - 80.0) / (y1 - y0).max(0.1))
            .clamp(8.0, 40.0);
        self.view = View {
            scale,
            ox: w / 2.0 - (x0 + x1) / 2.0 * scale,
            oy: h / 2.0 + (y0 + y1) / 2.0 * scale,
        };
        REPAINT
    }

    /// The drawing as a text: `mzdraw` (the document), `molfile` (V2000, V3000 when it has to be), `molfile-v3000`, `smiles`, `ket`,
    /// `svg` (in the style of the editor) or `log` (the command log). The error is an i18n key.
    pub fn save(&self, kind: &str) -> Result<String, &'static str> {
        Ok(match kind {
            "mzdraw" => self.doc().to_json(),
            "molfile" => self.doc().to_molfile(),
            "molfile-v3000" => self.doc().to_molfile_v3000(),
            "smiles" => self.doc().to_smiles()?,
            "ket" => self.doc().to_ket(),
            "svg" => self.svg(),
            "log" => self.session.log_json(),
            _ => return Err("error.draw2.request"),
        })
    }

    /// Adds the structure of a text (SMILES, molfile or SDF, `mzdraw/1`) to the right of the drawing, as one undo step.
    pub fn import(&mut self, text: &str) -> Result<io::Imported, &'static str> {
        self.import_placed(text).map(|r| r.0)
    }

    /// `import` for the bytes of a file: also a PNG saved by mzDraw, which carries its document.
    pub fn import_bytes(&mut self, bytes: &[u8]) -> Result<io::Imported, &'static str> {
        let read = io::import_bytes(bytes)?;
        self.add_read(read).map(|r| r.0)
    }

    /// The PNG of the drawing in the editor's style, for the surface that has a rasteriser (`png::plan`).
    pub fn png_plan(&self) -> crate::png::Plan {
        crate::png::plan(self.doc(), &self.style)
    }

    /// `import`, also giving the ids of what was added.
    pub(crate) fn import_placed(
        &mut self,
        text: &str,
    ) -> Result<(io::Imported, Placed), &'static str> {
        let read = io::import(text)?;
        self.add_read(read)
    }

    fn add_read(
        &mut self,
        (new, info): (Document, io::Imported),
    ) -> Result<(io::Imported, Placed), &'static str> {
        let right = self.doc().bounds().map(|b| b.2);
        let left = new.bounds().map(|b| b.0);
        let dx = match (right, left) {
            (Some(r), Some(l)) => r + 2.0 - l,
            _ => 0.0,
        };
        let placed = self.place(&new, dx, 0.0)?;
        self.said = ("draw2.say.imported", new.atoms.len().to_string());
        Ok((info, placed))
    }

    /// Adds every atom, bond and object of `new`, moved by (`dx`, `dy`), with fresh ids, as one undo step; returns the new ids.
    pub(crate) fn place(
        &mut self,
        new: &Document,
        dx: f64,
        dy: f64,
    ) -> Result<Placed, &'static str> {
        let base = self.doc().next - 1;
        let mut ops = Vec::new();
        for a in &new.atoms {
            ops.push(Op::AtomAdd {
                id: Some(base + a.id),
                el: a.el.clone(),
                x: a.x + dx,
                y: a.y + dy,
            });
            if a.chg != 0 || a.iso != 0 || a.rad != 0 || a.h.is_some() || !a.lab.is_empty() {
                let some = |v: bool| v.then_some(());
                ops.push(Op::AtomSet {
                    id: base + a.id,
                    el: None,
                    chg: some(a.chg != 0).map(|_| a.chg),
                    iso: some(a.iso != 0).map(|_| a.iso),
                    rad: some(a.rad != 0).map(|_| a.rad),
                    h: a.h,
                    lab: (!a.lab.is_empty()).then(|| a.lab.clone()),
                });
            }
        }
        for b in &new.bonds {
            ops.push(Op::BondAdd {
                id: Some(base + b.id),
                a: base + b.a,
                b: base + b.b,
                o: b.o,
                s: b.s,
            });
        }
        for o in &new.objects {
            let arrow = o.t == ObjKind::Arrow;
            ops.push(Op::ObjAdd {
                id: Some(base + o.id),
                t: o.t,
                x: o.x + dx,
                y: o.y + dy,
                x2: if arrow { o.x2 + dx } else { 0.0 },
                y2: if arrow { o.y2 + dy } else { 0.0 },
                text: o.text.clone(),
            });
        }
        self.session.run(&ops).map_err(|e| e.key)?;
        Ok(Placed {
            atoms: new.atoms.iter().map(|a| base + a.id).collect(),
            bonds: new.bonds.iter().map(|b| base + b.id).collect(),
            objects: new.objects.iter().map(|o| base + o.id).collect(),
        })
    }

    /// Opens a saved `mzdraw/1` document in place of the drawing: the same bytes, ids included, with an empty history (there is
    /// nothing to undo back to). The view, the tool and the clipboard stay. The error is an i18n key and changes nothing.
    pub fn open(&mut self, text: &str) -> Result<(), &'static str> {
        let doc = Document::from_json(text.trim())?;
        self.session = Session::new(doc);
        self.over = Overlay::default();
        self.press = None;
        self.said = ("draw2.say.opened", self.doc().atoms.len().to_string());
        self.error = "";
        Ok(())
    }

    /// Empties the canvas as one undo step.
    pub fn clear(&mut self) -> u32 {
        let atoms: Vec<u32> = self.doc().atoms.iter().map(|a| a.id).collect();
        let objects: Vec<u32> = self.doc().objects.iter().map(|o| o.id).collect();
        if atoms.is_empty() && objects.is_empty() {
            return 0;
        }
        self.run(
            &[Op::Remove {
                atoms,
                bonds: Vec::new(),
                objects,
            }],
            "draw2.say.cleared",
            "",
        )
    }

    /// Sets the drawing style from a preset name or the text of a settings file; a refusal is an i18n key and changes nothing.
    pub fn set_style(&mut self, text: &str) -> Result<u32, &'static str> {
        self.style = Style::from_text(text)?;
        self.said = ("draw2.say.style", self.style.name.clone());
        Ok(REPAINT)
    }

    /// The drawing as an SVG in the editor's style, carrying its document (`svg::carrying`).
    pub fn svg(&self) -> String {
        crate::svg::carrying(
            crate::svg::to_svg_styled(self.doc(), &self.style),
            self.doc(),
        )
    }

    /// Marks the atoms a check flagged (ids), replacing the previous marks.
    pub fn set_warnings(&mut self, ids: Vec<u32>) {
        self.over.warn_atoms = ids;
    }

    pub fn scene(&self) -> Scene {
        scene::build(
            self.doc(),
            self.view,
            &self.over,
            self.style.metrics(self.view.scale, true),
        )
    }
}
