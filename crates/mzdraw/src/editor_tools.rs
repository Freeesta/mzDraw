//! What the tools do with a press, a drag and a release, and the keyboard. Each completed gesture is one undo step.

use crate::build;
use crate::doc::ObjKind;
use crate::editor::{
    Editor, Hit, Press, Tool, ASK_LABEL, ASK_TEXT, CHANGED, CTRL, FOCUS, HANDLED, REPAINT, SHIFT,
    TOOL,
};
use crate::m;
use crate::ops::Op;
use std::f64::consts::PI;

/// A press becomes a drag after this many pixels.
const SLOP: f64 = 4.0;

impl Editor {
    pub fn pointer_down(&mut self, px: f64, py: f64, kind: u32, mods: u32) -> u32 {
        let hit = self.hit(px, py, kind);
        let shift = mods & SHIFT != 0;
        if self.tool == Tool::Select {
            self.select_press(hit, shift);
        }
        self.press = Some(Press {
            start: (px, py),
            now: (px, py),
            hit,
            moved: false,
            kind,
        });
        REPAINT
    }

    /// Makes the thing hit the selection (a long press with a finger).
    pub(crate) fn select_hit(&mut self, hit: Hit) {
        self.over.sel_atoms.clear();
        self.over.sel_bonds.clear();
        self.over.sel_objs.clear();
        self.select_press(hit, false);
    }

    /// Selection on press: a thing not yet selected becomes the selection (added with Shift); an empty spot clears it.
    fn select_press(&mut self, hit: Hit, shift: bool) {
        let o = &mut self.over;
        let inside = match hit {
            Hit::Atom(a) => o.sel_atoms.contains(&a),
            Hit::Bond(b) => o.sel_bonds.contains(&b),
            Hit::Obj(i) => o.sel_objs.contains(&i),
            Hit::None => false,
        };
        if !inside && !shift {
            o.sel_atoms.clear();
            o.sel_bonds.clear();
            o.sel_objs.clear();
        }
        match hit {
            Hit::Obj(i) if !inside => o.sel_objs.push(i),
            Hit::Atom(a) if !inside => o.sel_atoms.push(a),
            Hit::Bond(b) if !inside => {
                o.sel_bonds.push(b);
                if let Some(x) = self.session.doc.bond(b) {
                    for e in [x.a, x.b] {
                        if !o.sel_atoms.contains(&e) {
                            o.sel_atoms.push(e);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn pointer_move(&mut self, px: f64, py: f64, kind: u32) -> u32 {
        let Some(p) = self.press.as_mut() else {
            let hit = self.hit(px, py, kind);
            let (a, b, o) = match hit {
                Hit::Atom(a) => (Some(a), None, None),
                Hit::Bond(b) => (None, Some(b), None),
                Hit::Obj(o) => (None, None, Some(o)),
                Hit::None => (None, None, None),
            };
            let over = &mut self.over;
            let same = over.hover_atom == a && over.hover_bond == b && over.hover_obj == o;
            (over.hover_atom, over.hover_bond, over.hover_obj) = (a, b, o);
            return if same { 0 } else { REPAINT };
        };
        p.now = (px, py);
        if m::hypot(px - p.start.0, py - p.start.1) > SLOP {
            p.moved = true;
        }
        if !p.moved {
            return 0;
        }
        let (start, hit, kind) = (p.start, p.hit, p.kind);
        match (&self.tool, hit) {
            (Tool::Select, Hit::None) => {
                self.over.band = Some((
                    start.0.min(px),
                    start.1.min(py),
                    (px - start.0).abs(),
                    (py - start.1).abs(),
                ));
            }
            (Tool::Select, _) => {
                self.over.drag = (
                    (px - start.0) / self.view.scale,
                    (start.1 - py) / self.view.scale,
                );
            }
            (Tool::Arrow, _) => {
                let (from, to) = (
                    self.view.to_doc(start.0, start.1),
                    self.arrow_end(start, px, py),
                );
                self.over.preview = Some((from.0, from.1, to.0, to.1));
            }
            (Tool::Bond(..) | Tool::Atom(_), Hit::Atom(a)) => {
                let (to, _) = self.sprout_target(a, px, py, kind);
                if let Some(from) = self.session.doc.atom(a) {
                    self.over.preview = Some((from.x, from.y, to.0, to.1));
                }
            }
            _ => {}
        }
        REPAINT
    }

    /// Where an arrow dragged from `start` (pixels) to the pointer ends, in document units: level when it is within 10 degrees
    /// of the horizontal.
    fn arrow_end(&self, start: (f64, f64), px: f64, py: f64) -> (f64, f64) {
        let level = (py - start.1).abs() < (px - start.0).abs() * 0.176;
        self.view.to_doc(px, if level { start.1 } else { py })
    }

    /// Where a bond dragged from atom `a` ends: on the atom under the pointer, else one bond length away at a multiple of 30°.
    fn sprout_target(&self, a: u32, px: f64, py: f64, kind: u32) -> ((f64, f64), Option<u32>) {
        let d = &self.session.doc;
        if let Hit::Atom(b) = self.hit(px, py, kind)
            && b != a
            && let Some(t) = d.atom(b)
        {
            return ((t.x, t.y), Some(b));
        }
        let Some(from) = d.atom(a) else {
            return ((0.0, 0.0), None);
        };
        let (x, y) = self.view.to_doc(px, py);
        let step = PI / 6.0;
        let ang = (m::atan2(y - from.y, x - from.x) / step).round() * step;
        ((from.x + m::cos(ang), from.y + m::sin(ang)), None)
    }

    pub fn pointer_up(&mut self, px: f64, py: f64) -> u32 {
        let Some(p) = self.press.take() else {
            return 0;
        };
        let drag = self.over.drag;
        let band = self.over.band.take();
        self.over.drag = (0.0, 0.0);
        self.over.preview = None;
        let flags = match self.tool.clone() {
            Tool::Select => self.select_release(&p, drag, band),
            Tool::Atom(el) => self.atom_release(&p, &el, px, py),
            Tool::Bond(o, s) => self.bond_release(&p, (o, s), px, py),
            Tool::Erase => self.erase(p.hit),
            Tool::Ring(n, arom) => {
                let (x, y) = self.view.to_doc(px, py);
                let (atom, bond) = match p.hit {
                    Hit::Atom(a) => (Some(a), None),
                    Hit::Bond(b) => (None, Some(b)),
                    Hit::None | Hit::Obj(_) => (None, None),
                };
                self.run(
                    &[Op::Ring {
                        n,
                        arom,
                        atom,
                        bond,
                        x,
                        y,
                    }],
                    "draw2.say.ring",
                    &n.to_string(),
                )
            }
            Tool::Label(l) => self.label_release(&p, &l, px, py),
            Tool::Arrow => {
                let (x, y) = self.view.to_doc(p.start.0, p.start.1);
                // a click gives an arrow of two bond lengths to the right; a drag gives its own
                let (x2, y2) = if p.moved {
                    self.arrow_end(p.start, px, py)
                } else {
                    (x + 2.0, y)
                };
                let add = Op::ObjAdd {
                    id: None,
                    t: ObjKind::Arrow,
                    x,
                    y,
                    x2,
                    y2,
                    text: String::new(),
                };
                self.run(&[add], "draw2.say.arrow", "")
            }
            Tool::Plus => {
                let (x, y) = self.view.to_doc(px, py);
                let add = Op::ObjAdd {
                    id: None,
                    t: ObjKind::Plus,
                    x,
                    y,
                    x2: 0.0,
                    y2: 0.0,
                    text: String::new(),
                };
                self.run(&[add], "draw2.say.plus", "")
            }
            Tool::Text(text) => {
                let op = match p.hit {
                    // on a text or an arrow the tool sets its text
                    Hit::Obj(id) if self.doc().object(id).is_some_and(|o| o.t != ObjKind::Plus) => {
                        Op::ObjSet {
                            id,
                            text: Some(text.clone()),
                            x2: None,
                            y2: None,
                        }
                    }
                    _ => {
                        let (x, y) = self.view.to_doc(px, py);
                        Op::ObjAdd {
                            id: None,
                            t: ObjKind::Text,
                            x,
                            y,
                            x2: 0.0,
                            y2: 0.0,
                            text: text.clone(),
                        }
                    }
                };
                self.run(&[op], "draw2.say.text", &text)
            }
            Tool::Charge(c) => match p.hit {
                Hit::Atom(a) => {
                    let now = self.session.doc.atom(a).map_or(0, |x| x.chg);
                    let set = Op::AtomSet {
                        id: a,
                        el: None,
                        chg: Some(now + c),
                        iso: None,
                        rad: None,
                        h: None,
                        lab: None,
                    };
                    self.run(&[set], "draw2.say.charge", &(now + c).to_string())
                }
                _ => 0,
            },
        };
        flags | REPAINT
    }

    fn select_release(
        &mut self,
        p: &Press,
        drag: (f64, f64),
        band: Option<(f64, f64, f64, f64)>,
    ) -> u32 {
        if let Some((x, y, w, h)) = band {
            let d = &self.session.doc;
            let inside = |id: u32| {
                d.atom(id).is_some_and(|a| {
                    let (ax, ay) = self.view.to_px(a.x, a.y);
                    ax >= x && ax <= x + w && ay >= y && ay <= y + h
                })
            };
            let atoms: Vec<u32> = d
                .atoms
                .iter()
                .map(|a| a.id)
                .filter(|&i| inside(i))
                .collect();
            let bonds: Vec<u32> = d
                .bonds
                .iter()
                .filter(|b| inside(b.a) && inside(b.b))
                .map(|b| b.id)
                .collect();
            for a in atoms {
                if !self.over.sel_atoms.contains(&a) {
                    self.over.sel_atoms.push(a);
                }
            }
            for b in bonds {
                if !self.over.sel_bonds.contains(&b) {
                    self.over.sel_bonds.push(b);
                }
            }
            let objects: Vec<u32> = (self.session.doc.objects.iter())
                .filter(|o| {
                    let (ox, oy) = self.view.to_px(o.x, o.y);
                    ox >= x && ox <= x + w && oy >= y && oy <= y + h
                })
                .map(|o| o.id)
                .collect();
            for o in objects {
                if !self.over.sel_objs.contains(&o) {
                    self.over.sel_objs.push(o);
                }
            }
            return REPAINT;
        }
        if p.moved && p.hit != Hit::None && drag != (0.0, 0.0) {
            let (atoms, objects) = (self.over.sel_atoms.clone(), self.over.sel_objs.clone());
            let n = (atoms.len() + objects.len()).to_string();
            return self.run(
                &[Op::Move {
                    atoms,
                    objects,
                    dx: drag.0,
                    dy: drag.1,
                }],
                "draw2.say.moved",
                &n,
            );
        }
        REPAINT
    }

    fn atom_release(&mut self, p: &Press, el: &str, px: f64, py: f64) -> u32 {
        match p.hit {
            Hit::Atom(a) if p.moved => self.sprout(a, el, (1, 0), px, py, p.kind),
            Hit::Atom(a) => {
                let set = Op::AtomSet {
                    id: a,
                    el: Some(el.into()),
                    chg: None,
                    iso: None,
                    rad: None,
                    h: None,
                    lab: None,
                };
                self.run(&[set], "draw2.say.element", el)
            }
            Hit::Bond(_) => 0,
            Hit::None | Hit::Obj(_) => {
                let (x, y) = self.view.to_doc(px, py);
                self.run(
                    &[Op::AtomAdd {
                        id: None,
                        el: el.into(),
                        x,
                        y,
                    }],
                    "draw2.say.atom",
                    el,
                )
            }
        }
    }

    /// The label tool: on an atom it sets the label, on an empty spot it adds a labelled atom.
    fn label_release(&mut self, p: &Press, label: &str, px: f64, py: f64) -> u32 {
        let set = |id| Op::AtomSet {
            id,
            el: None,
            chg: None,
            iso: None,
            rad: None,
            h: None,
            lab: Some(label.to_string()),
        };
        match p.hit {
            Hit::Atom(a) => self.run(&[set(a)], "draw2.say.label", label),
            Hit::Bond(_) => 0,
            Hit::None | Hit::Obj(_) => {
                let (x, y) = self.view.to_doc(px, py);
                let id = self.session.doc.next;
                let add = Op::AtomAdd {
                    id: Some(id),
                    el: "C".into(),
                    x,
                    y,
                };
                self.run(&[add, set(id)], "draw2.say.label", label)
            }
        }
    }

    /// A bond from atom `a` to the atom under the pointer or to a new atom `el`.
    fn sprout(&mut self, a: u32, el: &str, os: (u8, u8), px: f64, py: f64, kind: u32) -> u32 {
        let (to, onto) = self.sprout_target(a, px, py, kind);
        self.join(a, onto, to, el, os)
    }

    /// Bonds `a` to the existing atom `onto` (setting the order and mark of a bond already there) or to a new atom at `to`. A wedge
    /// mark has its narrow end at `a`.
    fn join(
        &mut self,
        a: u32,
        onto: Option<u32>,
        to: (f64, f64),
        el: &str,
        (o, s): (u8, u8),
    ) -> u32 {
        let d = &self.session.doc;
        let ops = match onto {
            Some(b) => match d.bond_between(a, b) {
                Some(x) => vec![Op::BondSet {
                    id: x.id,
                    o: Some(o),
                    s: (s != 0).then_some(s),
                    rev: s != 0 && x.a != a,
                }],
                None => vec![Op::BondAdd {
                    id: None,
                    a,
                    b,
                    o,
                    s,
                }],
            },
            None => {
                let id = d.next;
                vec![
                    Op::AtomAdd {
                        id: Some(id),
                        el: el.into(),
                        x: to.0,
                        y: to.1,
                    },
                    Op::BondAdd {
                        id: None,
                        a,
                        b: id,
                        o,
                        s,
                    },
                ]
            }
        };
        self.run(&ops, "draw2.say.bond", &o.to_string())
    }

    fn bond_release(&mut self, p: &Press, (o, s): (u8, u8), px: f64, py: f64) -> u32 {
        match p.hit {
            Hit::Atom(a) if p.moved => self.sprout(a, "C", (o, s), px, py, p.kind),
            Hit::Atom(a) => {
                let ang = build::sprout_angle(&self.session.doc, a);
                let to = self
                    .session
                    .doc
                    .atom(a)
                    .map_or((0.0, 0.0), |f| (f.x + m::cos(ang), f.y + m::sin(ang)));
                self.join(a, None, to, "C", (o, s))
            }
            Hit::Bond(b) if s != 0 => {
                // a wedge tool on a bond sets its mark; on a bond that has it already it turns the wedge around
                let same = self.session.doc.bond(b).is_some_and(|x| x.s == s);
                let set = Op::BondSet {
                    id: b,
                    o: Some(1),
                    s: Some(s),
                    rev: same,
                };
                self.run(&[set], "draw2.say.wedge", "")
            }
            Hit::Bond(b) => {
                // the single-bond tool on a bond walks 1 -> 2 -> 3 -> 1 (a marked bond first loses its mark); the others set their order
                let (now, mark) = self.session.doc.bond(b).map_or((1, 0), |x| (x.o, x.s));
                let next = match (o, mark) {
                    (1, 0) => now % 3 + 1,
                    _ => o,
                };
                let set = Op::BondSet {
                    id: b,
                    o: Some(next),
                    s: Some(0),
                    rev: false,
                };
                self.run(&[set], "draw2.say.order", &next.to_string())
            }
            Hit::None | Hit::Obj(_) => {
                let (x, y) = self.view.to_doc(px, py);
                let id = self.session.doc.next;
                let (cx, sy) = (m::cos(PI / 6.0), m::sin(PI / 6.0));
                let ops = [
                    Op::AtomAdd {
                        id: Some(id),
                        el: "C".into(),
                        x,
                        y,
                    },
                    Op::AtomAdd {
                        id: Some(id + 1),
                        el: "C".into(),
                        x: x + cx,
                        y: y + sy,
                    },
                    Op::BondAdd {
                        id: None,
                        a: id,
                        b: id + 1,
                        o,
                        s,
                    },
                ];
                self.run(&ops, "draw2.say.bond", &o.to_string())
            }
        }
    }

    /// Erases the thing hit, or the whole selection when the thing is part of it.
    fn erase(&mut self, hit: Hit) -> u32 {
        let o = &self.over;
        let selected = match hit {
            Hit::Atom(a) => o.sel_atoms.contains(&a),
            Hit::Obj(i) => o.sel_objs.contains(&i),
            _ => false,
        };
        let (mut atoms, mut bonds, mut objects) = (Vec::new(), Vec::new(), Vec::new());
        match hit {
            _ if selected => (atoms, objects) = (o.sel_atoms.clone(), o.sel_objs.clone()),
            Hit::Atom(a) => atoms.push(a),
            Hit::Bond(b) => bonds.push(b),
            Hit::Obj(i) => objects.push(i),
            Hit::None => return 0,
        }
        let op = Op::Remove {
            atoms,
            bonds,
            objects,
        };
        self.run(&[op], "draw2.say.erased", "")
    }

    pub fn delete_selection(&mut self) -> u32 {
        let (atoms, bonds) = (self.over.sel_atoms.clone(), self.over.sel_bonds.clone());
        let objects = self.over.sel_objs.clone();
        if atoms.is_empty() && bonds.is_empty() && objects.is_empty() {
            return 0;
        }
        // a selected bond goes with its atoms only when they are selected too
        let bonds = bonds
            .into_iter()
            .filter(|&b| {
                self.session
                    .doc
                    .bond(b)
                    .is_some_and(|x| !atoms.contains(&x.a) && !atoms.contains(&x.b))
            })
            .collect();
        let op = Op::Remove {
            atoms,
            bonds,
            objects,
        };
        self.run(&[op], "draw2.say.erased", "")
    }

    pub fn select_all(&mut self) -> u32 {
        self.over.sel_atoms = self.session.doc.atoms.iter().map(|a| a.id).collect();
        self.over.sel_bonds = self.session.doc.bonds.iter().map(|b| b.id).collect();
        self.over.sel_objs = self.session.doc.objects.iter().map(|o| o.id).collect();
        REPAINT
    }

    /// Selects what the findings of one check (`key`, as the report names it) are about: their atoms, bonds and objects together,
    /// as the select tool would. The selection is the user's view and no step of the history. A key with no finding changes nothing.
    pub fn select_check(&mut self, key: &str) -> u32 {
        let report = crate::conform::report(&self.session.doc);
        let found = report.checks.iter().filter(|f| f.key == key);
        let (mut atoms, mut bonds, mut objects) = (Vec::new(), Vec::new(), Vec::new());
        let mut any = false;
        for f in found {
            any = true;
            atoms.extend(&f.atoms);
            bonds.extend(&f.bonds);
            objects.extend(&f.objects);
        }
        if !any {
            return 0;
        }
        for ids in [&mut atoms, &mut bonds, &mut objects] {
            crate::m::sort(ids);
            ids.dedup();
        }
        (self.over.sel_atoms, self.over.sel_bonds, self.over.sel_objs) = (atoms, bonds, objects);
        REPAINT
    }

    /// A key press (`key` as the browser names it). Returns `HANDLED` when the page must not let the browser act on it.
    pub fn key(&mut self, key: &str, mods: u32) -> u32 {
        let ctrl = mods & CTRL != 0;
        let shift = mods & SHIFT != 0;
        let lower = key.to_ascii_lowercase();
        if !ctrl && let Some(f) = self.focus_key(&lower, shift) {
            return f | HANDLED;
        }
        let f = match (ctrl, lower.as_str()) {
            (true, "z") if shift => self.redo(),
            (true, "z") => self.undo(),
            (true, "y") => self.redo(),
            (true, "a") => self.select_all(),
            (true, "d") => self.duplicate(),
            // copy, cut and paste arrive from the page as clipboard events
            (true, _) => return 0,
            (false, "delete" | "backspace") => self.delete_selection(),
            (false, "escape") => {
                self.over.sel_atoms.clear();
                self.over.sel_bonds.clear();
                self.over.sel_objs.clear();
                // leaving the focus is said only when there was one
                let left = match self.over.focus {
                    Some(_) => self.focus_on(None, None),
                    None => 0,
                };
                left | self.set_tool("select")
            }
            (false, "1" | "2" | "3") => self.set_tool(&format!("bond:{lower}")),
            (false, "w") if shift => self.set_tool("bond:down"),
            (false, "w") => self.set_tool("bond:up"),
            (false, "y") => self.set_tool("bond:wavy"),
            (false, "+" | "-") => self.set_tool(&format!("charge:{lower}")),
            (false, "v") => self.set_tool("select"),
            (false, "e") => self.set_tool("erase"),
            (false, "l") => ASK_LABEL,
            (false, "t") => ASK_TEXT,
            (false, "a") if shift => self.set_tool("arrow"),
            (false, k) => return self.element_key(k, shift),
        };
        f | HANDLED
    }

    /// A letter sets the element of the focused atom, else of the atom under the pointer, or picks the atom tool.
    fn element_key(&mut self, k: &str, shift: bool) -> u32 {
        let el = match (k, shift) {
            ("c", true) => "Cl",
            ("b", true) => "Br",
            ("s", true) => "Si",
            ("c", _) => "C",
            ("n", _) => "N",
            ("o", _) => "O",
            ("s", _) => "S",
            ("p", _) => "P",
            ("f", _) => "F",
            ("h", _) => "H",
            ("b", _) => "B",
            ("i", _) => "I",
            _ => return 0,
        };
        // the atom with the keyboard focus first, else the one under the pointer
        let said = if self.over.focus.is_some() { FOCUS } else { 0 };
        let f = match self.over.focus.or(self.over.hover_atom) {
            Some(a) => {
                let set = Op::AtomSet {
                    id: a,
                    el: Some(el.into()),
                    chg: None,
                    iso: None,
                    rad: None,
                    h: None,
                    lab: None,
                };
                self.run(&[set], "draw2.say.element", el) | said
            }
            None => self.set_tool(&format!("atom:{el}")),
        };
        f | HANDLED | (f & (CHANGED | TOOL))
    }
}
