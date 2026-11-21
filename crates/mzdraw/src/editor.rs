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
/// The layout of the palettes changed (`Layout`): the page should lay them out again.
pub const PALETTES: u32 = 128;
/// An item of the context menu needs the page to finish it: read `context_ask` (`context`).
pub const ASK: u32 = 256;
/// The page should open the context menu: a finger or a pen held still, the secondary button of a mouse or the barrel button of a
/// pen, the Menu key or Shift+F10. The page calls `context_menu` at the point of the event, or at `context_anchor` for a key.
pub const MENU: u32 = 512;

/// Modifier bits of an event.
pub const SHIFT: u32 = 1;
pub const CTRL: u32 = 2;
/// Bit 4 of a pointer event is the eraser end of a pen; bit 8 is Alt (Option) on a key.
pub const ALT: u32 = 8;
/// The secondary button of a mouse (right) or the barrel button of a pen: the context menu, not the tool.
pub const SECONDARY: u32 = 16;
/// The middle button of a mouse: it pans the view, with the secondary one when it is dragged.
pub const MIDDLE: u32 = 32;

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
    /// The steps of the last `scheme:orient` command (for the result document), empty before the first.
    pub(crate) orient_steps: Vec<crate::orient::Step>,
    pub view: View,
    pub tool: Tool,
    pub over: Overlay,
    pub(crate) press: Option<Press>,
    /// What the last change did, as an i18n key with one parameter, for the screen reader line.
    pub said: (&'static str, String),
    /// The last refused operation, an i18n key.
    pub error: &'static str,
    /// The fingers and the pen at work (pinch, palm rejection, long press).
    pub(crate) gesture: crate::editor_touch::Gesture,
    /// The atom the keyboard focus came from (the bond walked last).
    pub(crate) focus_from: Option<u32>,
    /// The drawing style: what the screen and the saved SVG look like. Not part of the document.
    pub style: Style,
    /// Where the palettes of the page are, and what Save writes: the `layout` and `export` objects of the settings file.
    pub layout: crate::palettes::Layout,
    pub export: crate::palettes::Export,
    /// The editor's own clipboard: an `mzdraw/1` document of what was copied, and how many times it was pasted since.
    pub(crate) clip: String,
    pub(crate) pasted: u32,
    /// The last nudge or turn by keys, to join the next press to it as one undo step.
    pub(crate) nudge: Option<crate::editor_nudge::Nudge>,
    /// The "move here" mode: with a selection, a click on an empty spot brings the selection there.
    pub move_here: bool,
    /// The open context menu and what the page was asked to do after its last item (`context`).
    pub(crate) ctx: crate::context::Context,
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
            gesture: Default::default(),
            focus_from: None,
            style: Style::default(),
            layout: Default::default(),
            export: Default::default(),
            clip: String::new(),
            pasted: 0,
            nudge: None,
            move_here: false,
            orient_steps: Vec::new(),
            ctx: Default::default(),
        }
    }
}

impl Editor {
    pub fn doc(&self) -> &Document {
        &self.session.doc
    }

    /// Runs an editor command of a request (`commands::run`: `move`, `rotate`, `flip`, `pathway:tree`) as one undo step; the refusal is
    /// returned and changes nothing.
    pub fn run_command(
        &mut self,
        name: &str,
        args: &serde_json::Value,
    ) -> Result<(), crate::ops::OpError> {
        crate::commands::run(self, name, args)
    }

    /// The patch (`draw_edit/1`) that turns the drawing in the editor into `other`; see `diff::diff`.
    pub fn diff_with(&self, other: &Document) -> Vec<crate::ops::Op> {
        crate::diff::diff(self.doc(), other)
    }

    /// Runs one undo step; a refusal is kept in `error` and changes nothing.
    pub(crate) fn run(&mut self, ops: &[Op], say: &'static str, what: &str) -> u32 {
        let ops = self.following(ops);
        match self.session.run(&ops) {
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

    /// The operations of a command and, after them, the route operations of the anchored arrows whose structures they change
    /// (`route::follow`): the one place where a command makes the arrows follow. The log carries the route operations, so a
    /// replay of the log gives the same bytes; an operation sent bare never follows.
    pub(crate) fn following(&self, ops: &[Op]) -> Vec<Op> {
        let s = &self.session.doc;
        let mut all = ops.to_vec();
        all.extend(crate::route::follow(s, ops, &self.style));
        all
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

    /// What is under the pointer: the nearest atom within reach, unless the pointer is over the middle of a bond and nearer to
    /// the bond than to that atom (a finger reaches further than half a bond, and the middle of a bond must still be the
    /// bond); else the nearest bond; else an object.
    pub fn hit(&self, px: f64, py: f64, kind: u32) -> Hit {
        let r = Self::reach(kind);
        let d = self.doc();
        let mut atom = (r, Hit::None);
        for a in &d.atoms {
            let (x, y) = self.view.to_px(a.x, a.y);
            let dist = m::hypot(x - px, y - py);
            if dist < atom.0 {
                atom = (dist, Hit::Atom(a.id));
            }
        }
        // the nearest bond, and the nearest one whose middle part (30 % to 70 % of its length) the pointer is over
        let (mut bond, mut middle) = ((r * 0.7, Hit::None), (r * 0.7, Hit::None));
        for b in &d.bonds {
            let (Some(p), Some(q)) = (d.atom(b.a), d.atom(b.b)) else {
                continue;
            };
            let (s, e) = (self.view.to_px(p.x, p.y), self.view.to_px(q.x, q.y));
            let (dx, dy) = (e.0 - s.0, e.1 - s.1);
            let along = ((px - s.0) * dx + (py - s.1) * dy) / (dx * dx + dy * dy).max(1e-9);
            let t = along.clamp(0.0, 1.0);
            let dist = m::hypot(s.0 + t * dx - px, s.1 + t * dy - py);
            if dist < bond.0 {
                bond = (dist, Hit::Bond(b.id));
            }
            if (0.3..=0.7).contains(&along) && dist < middle.0 {
                middle = (dist, Hit::Bond(b.id));
            }
        }
        if atom.1 != Hit::None {
            return if middle.1 != Hit::None && middle.0 < atom.0 {
                middle.1
            } else {
                atom.1
            };
        }
        if bond.1 != Hit::None {
            return bond.1;
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
        // a number that is not finite (or a factor that is not positive) would poison the view for good
        if ![px, py, factor].iter().all(|v| v.is_finite()) || factor <= 0.0 {
            return 0;
        }
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

    /// The drawing as a text: `mzdraw` (the document), `molfile` (V2000, V3000 when it has to be), `molfile-v3000`, `smiles`, `ket`, `cdxml`, `cml`, `rxn` (a drawing with one arrow), `trasf` (the record `trasf/1` of one structure before and one after the arrow, with its two `mol/1` records), `sdf` (one record per structure, with its data),
    /// `svg` (in the style of the editor), `pdf` (base64 of the PDF file), `log` (the command log) or `std` (the drawing checks, `conform::report`, read-only). The error is an i18n key.
    pub fn save(&self, kind: &str) -> Result<String, &'static str> {
        Ok(match kind {
            k if crate::brand::is_doc_kind(k) => self.doc().to_json(),
            "molfile" => self.doc().to_molfile(),
            "molfile-v3000" => self.doc().to_molfile_v3000(),
            "smiles" => self.doc().to_smiles()?,
            "ket" => self.doc().to_ket(),
            "cdxml" => self.doc().to_cdxml(),
            "cml" => self.doc().to_cml(),
            "rxn" => self.doc().to_rxn()?,
            "trasf" => self.doc().to_trasf().map_err(|e| e.key)?,
            "sdf" => self.doc().to_sdf(),
            "svg" => self.svg(),
            "pdf" => crate::pdf::base64(&self.pdf()),
            "log" => self.session.log_json(),
            // the settings file: the style, the layout of the palettes and the export defaults
            "settings" => self.settings_json(),
            "std" => crate::pathway::report_with(self.doc(), &self.style).to_json(),
            "boxes" => crate::pathway::boxes_json(self.doc(), &self.style),
            "schema" => crate::schema::json(),
            _ => return Err("error.draw2.request"),
        })
    }

    /// Why `save("trasf")` refuses the drawing, if it does: the i18n key and the ids of the atoms it is about (the unmapped ones for
    /// `error.draw2.trasfMap`).
    pub fn trasf_problem(&self) -> Option<(&'static str, Vec<u32>)> {
        self.doc().trasf().err().map(|e| (e.key, e.atoms))
    }

    /// What `save(kind)` leaves out because the format has no place for it, as (i18n key, count): atom maps for CDXML and CML.
    pub fn lost_on_save(&self, kind: &str) -> Vec<(&'static str, usize)> {
        Some(("draw2.export.mapsLost", self.doc().maps_lost(kind)))
            .filter(|l| l.1 > 0)
            .into_iter()
            .collect()
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
                map: None,
                id: Some(base + a.id),
                el: a.el.clone(),
                x: a.x + dx,
                y: a.y + dy,
            });
            if a.chg != 0
                || a.iso != 0
                || a.rad != 0
                || a.map != 0
                || a.h.is_some()
                || !a.lab.is_empty()
            {
                let some = |v: bool| v.then_some(());
                ops.push(Op::AtomSet {
                    id: base + a.id,
                    el: None,
                    chg: some(a.chg != 0).map(|_| a.chg),
                    iso: some(a.iso != 0).map(|_| a.iso),
                    rad: some(a.rad != 0).map(|_| a.rad),
                    h: a.h,
                    lab: (!a.lab.is_empty()).then(|| a.lab.clone()),
                    map: (a.map != 0).then_some(a.map),
                });
            }
            // the data of a structure (the fields of an SDF record) comes along with the atom that keeps it
            let data = a.data();
            if !data.is_empty() {
                ops.push(Op::AtomData {
                    id: base + a.id,
                    data,
                });
            }
            let ann = a.ann();
            if !ann.is_empty() {
                ops.push(Op::AtomAnn {
                    id: base + a.id,
                    ann,
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
                from: if o.from == 0 { 0 } else { base + o.from },
                to: if o.to == 0 { 0 } else { base + o.to },
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
        self.nudge = None;
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

    /// Sets the drawing style from a preset name, or the whole settings from the text of a settings file (its `layout` and
    /// `export` too: a file without them gives their defaults); a refusal is an i18n key and changes nothing.
    pub fn set_style(&mut self, text: &str) -> Result<u32, &'static str> {
        let mut style = Style::from_text(text)?;
        let mut flags = REPAINT;
        if text.trim_start().starts_with('{') {
            let mut take = |key: &str| {
                let file = &mut style.unknown.file;
                let i = file.iter().position(|(k, _)| k == key)?;
                Some(file.remove(i).1)
            };
            let value = |t: String, e| serde_json::from_str::<serde_json::Value>(&t).map_err(|_| e);
            let layout = match take("layout") {
                Some(t) => crate::palettes::Layout::read(&value(t, "error.draw2.layout")?)?,
                None => Default::default(),
            };
            let export = match take("export") {
                Some(t) => crate::palettes::Export::read(&value(t, "error.draw2.export")?)?,
                None => Default::default(),
            };
            self.layout.replace(layout);
            self.export = export;
            flags |= PALETTES;
        }
        self.style = style;
        self.said = ("draw2.say.style", self.style.name.clone());
        Ok(flags)
    }

    /// The settings file of the editor: the style, the layout of the palettes and the export defaults (the last two only
    /// when they differ from their defaults).
    pub fn settings_json(&self) -> String {
        let mut s = self.style.clone();
        for (key, v) in [
            ("layout", self.layout.to_text()),
            ("export", self.export.to_text()),
        ] {
            if let Some(v) = v {
                s.unknown.file.retain(|(k, _)| k != key);
                s.unknown.file.push((key.to_string(), v));
            }
        }
        s.to_json()
    }

    /// `set_map` from the text of a field: it must be a whole number (digits only); anything else (empty, a fraction, a sign, a word) is
    /// refused with `error.draw2.map` and changes nothing.
    pub fn set_map_text(&mut self, text: &str) -> u32 {
        let t = text.trim();
        match t.parse::<u16>() {
            Ok(n) if t.bytes().all(|b| b.is_ascii_digit()) => self.set_map(n),
            _ => {
                self.error = "error.draw2.map";
                REPAINT
            }
        }
    }

    /// Sets the atom-to-atom map of the atom that has the focus, or else of the only selected atom, as one undo step; 0 takes it
    /// away. A number above `doc::MAX_MAP`, or no atom to act on, changes nothing (`error.draw2.map` in `error`).
    pub fn set_map(&mut self, map: u16) -> u32 {
        let target = (self.over.focus)
            .or_else(|| match self.over.sel_atoms[..] {
                [only] => Some(only),
                _ => None,
            })
            .filter(|&a| self.doc().atom(a).is_some());
        let Some(id) = target.filter(|_| map <= crate::doc::MAX_MAP) else {
            self.error = "error.draw2.map";
            return REPAINT;
        };
        let set = Op::AtomSet {
            id,
            el: None,
            chg: None,
            iso: None,
            rad: None,
            h: None,
            lab: None,
            map: Some(map),
        };
        self.run(&[set], "draw2.say.map", &map.to_string())
    }

    /// Sets the data of the structure the atom `anchor` belongs to (the data fields of its SDF record), as one undo step: the
    /// list is kept on `anchor` and taken off the other atoms of the structure. A list an SDF could not hold is refused
    /// (`error.draw2.data` in `error`) and changes nothing.
    pub fn set_data(&mut self, anchor: u32, data: Vec<(String, String)>) -> u32 {
        let piece = crate::layout::pieces(self.doc())
            .into_iter()
            .find(|p| p.contains(&anchor));
        let mut ops: Vec<Op> = (self.doc().atoms.iter())
            .filter(|a| {
                a.id != anchor
                    && piece.as_ref().is_some_and(|p| p.contains(&a.id))
                    && !a.data().is_empty()
            })
            .map(|a| Op::AtomData {
                id: a.id,
                data: Vec::new(),
            })
            .collect();
        let n = data.len().to_string();
        ops.push(Op::AtomData { id: anchor, data });
        self.run(&ops, "draw2.say.data", &n)
    }

    /// Annotates the structure the atom `anchor` belongs to, as one undo step: a new entry (`ann::entry`: the level, an
    /// identifier, the evidence as lines `kind: ref`, a note; decided by the chemist) is added after the entries the structure
    /// has, and the whole annotation is kept on `anchor`. The `scale` the level is read on (`ann::SCALES`; empty: the default) is
    /// written with the entry. An empty `level` takes the annotation of the structure away. An entry
    /// that is not of its shape is refused (`error.draw2.ann` in `error`) and changes nothing.
    pub fn annotate(
        &mut self,
        anchor: u32,
        scale: &str,
        level: &str,
        identifier: &str,
        evidence: &str,
        note: &str,
    ) -> u32 {
        let scale = Some(scale.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or(crate::ann::SCALE);
        let piece = crate::layout::pieces(self.doc())
            .into_iter()
            .find(|p| p.contains(&anchor))
            .unwrap_or_default();
        let held: Vec<(u32, String)> = (self.doc().atoms.iter())
            .filter(|a| piece.contains(&a.id))
            .map(|a| (a.id, a.ann()))
            .filter(|a| !a.1.is_empty())
            .collect();
        let level = level.trim();
        let new = crate::ann::entry(scale, level, identifier, evidence, note, "chemist");
        let list = if level.is_empty() {
            "[]".to_string()
        } else {
            let old = held.iter().map(|a| a.1.as_str());
            crate::ann::join(old.chain(new.as_deref()))
        };
        // refused: an entry that is not of its shape, a history over its limits, an atom that is none
        let fine = (level.is_empty() || new.is_some()) && !piece.is_empty();
        let Some(ann) = crate::ann::canon(&list).filter(|_| fine) else {
            self.error = "error.draw2.ann";
            return REPAINT;
        };
        let mut ops: Vec<Op> = (held.iter().filter(|a| a.0 != anchor))
            .map(|a| Op::AtomAnn {
                id: a.0,
                ann: "[]".into(),
            })
            .collect();
        ops.push(Op::AtomAnn { id: anchor, ann });
        self.run(&ops, "draw2.say.ann", level)
    }

    /// Sets one setting from its text, `<section>.<key>=<value>` as the `--set` of the CLI writes it. The sections are `style`
    /// (`Style::set`), `export` (`Export::set`) and `layout` (`Layout::set`; `layout=<object>` sets the whole object). The
    /// error is an i18n key and changes nothing.
    pub fn set(&mut self, setting: &str) -> Result<u32, &'static str> {
        let (name, value) = setting.split_once('=').ok_or("error.draw2.set")?;
        let name = name.trim();
        if name == "layout" {
            let v = serde_json::from_str(value).map_err(|_| "error.draw2.layout")?;
            self.layout.replace(crate::palettes::Layout::read(&v)?);
            return Ok(PALETTES);
        }
        let (section, key) = name.split_once('.').ok_or("error.draw2.set")?;
        match section {
            "style" => {
                self.style.set(key, value)?;
                self.said = ("draw2.say.style", self.style.name.clone());
                Ok(REPAINT)
            }
            "display" => self.style.set_display(key, value).map(|_| REPAINT),
            "pathway" => self.style.set_pathway(key, value).map(|_| 0),
            "export" => self.export.set(key, value).map(|_| 0),
            "layout" => self.layout.set(key, value).map(|_| PALETTES),
            _ => Err("error.draw2.set"),
        }
    }

    /// The drawing as an SVG in the editor's style, carrying its document (`svg::carrying`).
    pub fn svg(&self) -> String {
        crate::svg::carrying(
            crate::svg::to_svg_styled(self.doc(), &self.style),
            self.doc(),
        )
    }

    /// The drawing as a PDF file in the editor's style, carrying its document (`pdf::to_pdf`).
    pub fn pdf(&self) -> Vec<u8> {
        crate::pdf::to_pdf(self.doc(), &self.style)
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
