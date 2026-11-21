//! The display list: the document, a view and the editor's transient state turned into flat primitives that any surface paints
//! without knowing chemistry. The page paints it on a canvas; `svg` writes the same list as a file.
//!
//! Layout of `data` (f32): `LINE x1 y1 x2 y2 width colour` · `TEXT x y size colour string anchor` (anchor 0 middle, 1 start) ·
//! `RECT x y w h colour` · `DISC x y r colour` · `RING x y r colour` · `TRI x1 y1 x2 y2 x3 y3 colour` (a filled triangle).
//! Coordinates are in output pixels, y down.

use crate::doc::{Atom, Document, Obj, ObjKind, WEDGE_DOWN, WEDGE_UP};
use crate::m;
use crate::settings::Metrics;

pub const LINE: f32 = 1.0;
pub const TEXT: f32 = 2.0;
pub const RECT: f32 = 3.0;
pub const DISC: f32 = 4.0;
pub const RING: f32 = 5.0;
pub const TRI: f32 = 6.0;

/// Colour indices; the surface owns the actual colours (CSS variables on the page, the style palette in a file).
pub mod colour {
    pub const INK: f32 = 0.0;
    pub const SELECT: f32 = 1.0;
    pub const HOVER: f32 = 2.0;
    pub const WARN: f32 = 3.0;
    pub const PREVIEW: f32 = 4.0;
    pub const N: f32 = 5.0;
    pub const O: f32 = 6.0;
    pub const S: f32 = 7.0;
    pub const HALOGEN: f32 = 8.0;
    pub const P: f32 = 9.0;
    /// The ring of the keyboard focus.
    pub const FOCUS: f32 = 10.0;
}

/// Document units to output pixels: `x' = ox + x * scale`, `y' = oy - y * scale`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub scale: f64,
    pub ox: f64,
    pub oy: f64,
}

impl View {
    pub fn to_px(&self, x: f64, y: f64) -> (f64, f64) {
        (self.ox + x * self.scale, self.oy - y * self.scale)
    }

    pub fn to_doc(&self, px: f64, py: f64) -> (f64, f64) {
        ((px - self.ox) / self.scale, (self.oy - py) / self.scale)
    }
}

/// What the editor adds on top of the document: selection and hover (atoms, bonds, objects), flagged atoms, a moving selection, a preview bond, a rubber band.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    pub sel_atoms: Vec<u32>,
    pub sel_bonds: Vec<u32>,
    pub sel_objs: Vec<u32>,
    pub hover_obj: Option<u32>,
    /// The atom with the keyboard focus.
    pub focus: Option<u32>,
    pub hover_atom: Option<u32>,
    pub hover_bond: Option<u32>,
    pub warn_atoms: Vec<u32>,
    /// Offset (document units) drawn on the selected atoms while they are dragged.
    pub drag: (f64, f64),
    /// A bond being drawn, in document units.
    pub preview: Option<(f64, f64, f64, f64)>,
    /// A rubber band, in pixels (x, y, w, h).
    pub band: Option<(f64, f64, f64, f64)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub data: Vec<f32>,
    pub strings: Vec<String>,
}

/// Approximate advance width of a label character, in units of the font size (Arial-like; the same table everywhere).
fn advance(c: char) -> f64 {
    match c {
        'I' | 'i' | 'l' => 0.28,
        '+' | '-' | '\u{2212}' => 0.58,
        '0'..='9' => 0.56,
        'a'..='z' => 0.5,
        'M' | 'W' => 0.9,
        _ => 0.7,
    }
}

pub fn width(s: &str, size: f64) -> f64 {
    s.chars().map(advance).sum::<f64>() * size
}

fn element_colour(el: &str) -> f32 {
    match el {
        "N" => colour::N,
        "O" => colour::O,
        "S" => colour::S,
        "P" => colour::P,
        "F" | "Cl" | "Br" | "I" => colour::HALOGEN,
        _ => colour::INK,
    }
}

/// Whether the atom is drawn as text (a carbon inside a chain is a bare vertex).
pub fn has_label(doc: &Document, a: &Atom) -> bool {
    a.el != "C"
        || !a.lab.is_empty()
        || a.chg != 0
        || a.iso != 0
        || a.rad != 0
        || a.h.is_some()
        || doc.order_sum(a.id) == 0
}

/// The parts of an atom drawn as text (`Painter::parts`): widths in pixels.
struct Parts {
    /// Half the width of the element symbol.
    half: f64,
    /// The width of the mass number (0 without one).
    iso: f64,
    h: u32,
    /// The width of the hydrogens with their count (0 without any).
    hw: f64,
    /// Whether the hydrogens stand on the left.
    left: bool,
    /// Where the hydrogens stand when bonds leave on both sides: 1 under the symbol, -1 over it, 0 beside it.
    stack: f64,
    /// The charge and the radical dots.
    mark: String,
}

impl Parts {
    /// The width of the hydrogens when they stand beside the symbol.
    fn side(&self) -> f64 {
        if self.stack == 0.0 {
            self.hw
        } else {
            0.0
        }
    }
}

struct Painter<'a> {
    doc: &'a Document,
    view: View,
    over: &'a Overlay,
    m: Metrics,
    font: f64,
    out: Scene,
}

impl Painter<'_> {
    fn pos(&self, a: &Atom) -> (f64, f64) {
        let moving = self.over.drag != (0.0, 0.0) && self.over.sel_atoms.contains(&a.id);
        let (dx, dy) = if moving { self.over.drag } else { (0.0, 0.0) };
        self.view.to_px(a.x + dx, a.y + dy)
    }

    fn push(&mut self, v: &[f64]) {
        self.out.data.extend(v.iter().map(|&x| x as f32));
    }

    fn line(&mut self, p: (f64, f64), q: (f64, f64), w: f64, c: f32) {
        self.push(&[LINE as f64, p.0, p.1, q.0, q.1, w, c as f64]);
    }

    fn text(&mut self, x: f64, y: f64, size: f64, c: f32, s: &str, anchor: f64) {
        let i = self.out.strings.len();
        self.out.strings.push(s.to_string());
        self.push(&[TEXT as f64, x, y, size, c as f64, i as f64, anchor]);
    }

    fn bond(&mut self, b: &crate::doc::Bond) {
        let (Some(p), Some(q)) = (self.doc.atom(b.a), self.doc.atom(b.b)) else {
            return;
        };
        let (mut s, mut e) = (self.pos(p), self.pos(q));
        let (dx, dy) = (e.0 - s.0, e.1 - s.1);
        let len = m::hypot(dx, dy);
        if len < 1e-6 {
            return;
        }
        let (ux, uy) = (dx / len, dy / len);
        // a bond stops short of a label
        let gap = self.m.gap.min(len * 0.4);
        let (lp, lq) = (has_label(self.doc, p), has_label(self.doc, q));
        if lp {
            let g = (gap + self.clear(p, ux, uy)).min(len * 0.45);
            s = (s.0 + ux * g, s.1 + uy * g);
        }
        if lq {
            let g = (gap + self.clear(q, -ux, -uy)).min(len * 0.45);
            e = (e.0 - ux * g, e.1 - uy * g);
        }
        let w = self.m.line;
        if self.over.sel_bonds.contains(&b.id) || self.over.hover_bond == Some(b.id) {
            let c = if self.over.sel_bonds.contains(&b.id) {
                colour::SELECT
            } else {
                colour::HOVER
            };
            self.line(s, e, w * 5.0, c);
        }
        let off = self.m.spacing;
        let (nx, ny) = (-uy * off, ux * off);
        let shift = |p: (f64, f64), k: f64| (p.0 + nx * k, p.1 + ny * k);
        if b.s != 0 {
            if lq && b.s != crate::doc::WEDGE_WAVY {
                // the wide end of a wedge needs more room in front of a label than a line does
                let more = (self.m.wedge * 0.5).min(len * 0.1);
                e = (e.0 - ux * more, e.1 - uy * more);
            }
            return self.wedge(b.s, s, e, w);
        }
        match b.o {
            2 => {
                let side = self.double_side(b, (ux, uy));
                if side == 0.0 {
                    self.line(shift(s, 0.5), shift(e, 0.5), w, colour::INK);
                    self.line(shift(s, -0.5), shift(e, -0.5), w, colour::INK);
                } else {
                    self.line(s, e, w, colour::INK);
                    // the second line sits inside, a little shorter unless it ends at a label
                    let trim = len * 0.12;
                    let s2 = if lp {
                        s
                    } else {
                        (s.0 + ux * trim, s.1 + uy * trim)
                    };
                    let e2 = if lq {
                        e
                    } else {
                        (e.0 - ux * trim, e.1 - uy * trim)
                    };
                    self.line(shift(s2, side), shift(e2, side), w, colour::INK);
                }
            }
            3 => {
                for k in [-1.0, 0.0, 1.0] {
                    self.line(shift(s, k), shift(e, k), w, colour::INK);
                }
            }
            _ => self.line(s, e, w, colour::INK),
        }
    }

    /// A wedge mark from `s` (narrow end) to `e`: a filled triangle, a ladder of widening rungs, or a wave.
    fn wedge(&mut self, mark: u8, s: (f64, f64), e: (f64, f64), w: f64) {
        let (dx, dy) = (e.0 - s.0, e.1 - s.1);
        let len = m::hypot(dx, dy).max(1e-6);
        let half = self.m.wedge;
        let (nx, ny) = (-dy / len * half, dx / len * half);
        let at = |t: f64, k: f64| (s.0 + dx * t + nx * k, s.1 + dy * t + ny * k);
        match mark {
            WEDGE_UP => {
                let (p, q) = (at(1.0, 1.0), at(1.0, -1.0));
                self.push(&[TRI as f64, s.0, s.1, p.0, p.1, q.0, q.1, colour::INK as f64]);
            }
            WEDGE_DOWN => {
                let n = ((len / self.m.hash).round() as usize).clamp(3, 12);
                for i in 1..=n {
                    let t = i as f64 / n as f64;
                    self.line(at(t, t), at(t, -t), w, colour::INK);
                }
            }
            _ => {
                // four waves, eight straight pieces each
                let n = 32;
                let wave = |i: usize| {
                    let t = i as f64 / n as f64;
                    at(t, 0.8 * m::sin(t * 8.0 * std::f64::consts::PI))
                };
                for i in 0..n {
                    self.line(wave(i), wave(i + 1), w, colour::INK);
                }
            }
        }
    }

    /// Which side the second line of a double bond takes: the side with more substituents (the inside of a ring), 0 = centred
    /// (an end without other bonds, as in C=O).
    fn double_side(&self, b: &crate::doc::Bond, u: (f64, f64)) -> f64 {
        let (Some(p), Some(q)) = (self.doc.atom(b.a), self.doc.atom(b.b)) else {
            return 0.0;
        };
        let mut sum = 0.0;
        for (end, other) in [(p, q), (q, p)] {
            let subs: Vec<u32> = self
                .doc
                .neighbours(end.id)
                .into_iter()
                .filter(|&n| n != other.id)
                .collect();
            if subs.is_empty() {
                return 0.0;
            }
            let o = self.pos(end);
            for n in subs.iter().filter_map(|&n| self.doc.atom(n)) {
                let m = self.pos(n);
                sum += (u.0 * (m.1 - o.1) - u.1 * (m.0 - o.0)).signum();
            }
        }
        if sum < 0.0 {
            -1.0
        } else {
            1.0
        }
    }

    /// Whether the bonds of an atom leave to the right, so that its text grows to the left.
    fn grows_left(&self, a: &Atom) -> bool {
        let lean: f64 = self
            .doc
            .neighbours(a.id)
            .iter()
            .filter_map(|&n| self.doc.atom(n))
            .map(|n| n.x - a.x)
            .sum();
        lean > 0.01
    }

    /// A group label (an abbreviation or an R group) as it is drawn: the text of `label::arrange` for the side the bonds leave on,
    /// and the widths in pixels of what stands before its attachment atom, of the attachment atom and of what stands after it (the
    /// digits of a count are lowered, hence narrower).
    fn group_box(&self, a: &Atom) -> (String, [f64; 3]) {
        let (f, small) = (self.font, self.m.small);
        let arranged = crate::label::arrange(&a.lab, self.grows_left(a));
        let mut w = [0.0; 3];
        for (i, c) in arranged.text.chars().enumerate() {
            let k = if i < arranged.at.0 {
                0
            } else if i < arranged.at.1 {
                1
            } else {
                2
            };
            w[k] += width(&c.to_string(), if c.is_ascii_digit() { small } else { f });
        }
        (arranged.text, w)
    }

    /// An abbreviation or an R group, written with its digits lowered and its units reversed when the bonds leave to the right
    /// (`label::arrange`), so that its attachment atom is the one centred on the atom of the drawing.
    fn group_label(&mut self, a: &Atom) {
        let (x, y) = self.pos(a);
        let (f, small) = (self.font, self.m.small);
        let (text, [before, mid, _]) = self.group_box(a);
        let mut runs: Vec<(String, bool)> = Vec::new();
        for c in text.chars() {
            let digit = c.is_ascii_digit();
            match runs.last_mut() {
                Some((s, d)) if *d == digit => s.push(c),
                _ => runs.push((c.to_string(), digit)),
            }
        }
        let size = |digit: bool| if digit { small } else { f };
        let mut tx = x - before - mid / 2.0;
        for (s, digit) in runs {
            let dy = if digit { f * 0.3 } else { 0.0 };
            self.text(tx, y + dy, size(digit), colour::INK, &s, 1.0);
            tx += width(&s, size(digit));
        }
    }

    /// The label of an atom: symbol centred on the atom, hydrogens on the free side, isotope and charge raised.
    fn label(&mut self, a: &Atom) {
        if !a.lab.is_empty() {
            return self.group_label(a);
        }
        let (x, y) = self.pos(a);
        let f = self.font;
        let c = if self.m.mono {
            colour::INK
        } else {
            element_colour(&a.el)
        };
        let small = self.m.small;
        self.text(x, y, f, c, &a.el, 0.0);
        let p = self.parts(a);
        let half = p.half;
        if a.iso != 0 {
            let s = a.iso.to_string();
            self.text(x - half - p.iso, y - f * 0.35, small, c, &s, 1.0);
        }
        if p.h > 0 {
            // on the left the hydrogens stand before the mass number: H3 13C
            let hx = if p.stack != 0.0 {
                x - width("H", f) / 2.0
            } else if p.left {
                x - half - p.iso - p.hw
            } else {
                x + half
            };
            let hy = y + p.stack * f * 0.95;
            self.text(hx, hy, f, c, "H", 1.0);
            if p.h > 1 {
                let count = p.h.to_string();
                self.text(hx + width("H", f), hy + f * 0.3, small, c, &count, 1.0);
            }
        }
        if !p.mark.is_empty() {
            let right = x + half + if p.left { 0.0 } else { p.side() };
            self.text(right, y - f * 0.35, small, c, &p.mark, 1.0);
        }
    }

    /// What stands around the element symbol of an atom drawn as text, and how wide each part is.
    fn parts(&self, a: &Atom) -> Parts {
        let (f, small) = (self.font, self.m.small);
        let h = self.doc.hydrogens(a);
        // hydrogens go left when the bonds leave to the right; with bonds on both sides they go under the symbol, or over it
        // when the bonds leave downwards
        let (mut lean, mut rise, mut sides) = (0.0, 0.0, [false; 2]);
        for n in (self.doc.neighbours(a.id).iter()).filter_map(|&n| self.doc.atom(n)) {
            let (dx, dy) = (n.x - a.x, n.y - a.y);
            lean += dx;
            rise += dy;
            if dx.abs() > 0.3 * m::hypot(dx, dy) {
                sides[usize::from(dx > 0.0)] = true;
            }
        }
        let stack = match (h > 0 && sides == [true; 2], rise < -0.01) {
            (false, _) => 0.0,
            (true, false) => 1.0,
            (true, true) => -1.0,
        };
        let count = if h > 1 { h.to_string() } else { String::new() };
        let mut mark = String::new();
        if a.chg != 0 {
            if a.chg.abs() > 1 {
                mark.push_str(&a.chg.abs().to_string());
            }
            mark.push(if a.chg > 0 { '+' } else { '\u{2212}' });
        }
        mark.push_str(&"\u{2022}".repeat(a.rad as usize));
        Parts {
            half: width(&a.el, f) / 2.0,
            iso: if a.iso != 0 {
                width(&a.iso.to_string(), small)
            } else {
                0.0
            },
            h,
            hw: if h > 0 {
                width("H", f) + width(&count, small)
            } else {
                0.0
            },
            left: stack == 0.0 && lean > 0.01,
            stack,
            mark,
        }
    }

    /// How far a bond leaving the atom along (`ux`, `uy`) (pixels, y down) stops short of it beyond the usual gap, so that it
    /// does not run into the hydrogens, the mass number or the charge that stand on its side of the symbol.
    fn clear(&self, a: &Atom, ux: f64, uy: f64) -> f64 {
        if ux.abs() < 1e-6 || !has_label(self.doc, a) {
            return 0.0;
        }
        if !a.lab.is_empty() {
            // a group label: a bond stops short of what is drawn on its side of the attachment atom (the count of `R1`, the units
            // of a label that grows towards the bond)
            let [before, _, after] = self.group_box(a).1;
            let row = self.font * 0.6 / uy.abs().max(1e-6);
            return ((if ux > 0.0 { after } else { before }) / ux.abs()).min(row);
        }
        let p = self.parts(a);
        let marks = width(&p.mark, self.m.small);
        let (hw, beside) = (p.side(), p.h > 0 && p.stack == 0.0);
        let (reach, raised) = match (ux > 0.0, p.left) {
            (true, true) => (marks, true),
            (true, false) => (hw + marks, !beside),
            (false, true) => (p.iso + hw, !beside),
            (false, false) => (p.iso, true),
        };
        // a symbol of two letters is wider than the gap of the style allows for
        let wide = (p.half - self.font * 0.35).max(0.0);
        // a raised part (charge, mass number) is cleared by a bond that goes down; the row of the text by one steep enough
        let reach = wide + if raised && uy > 0.3 { 0.0 } else { reach };
        let row = self.font * 0.6 / uy.abs().max(1e-6) - p.half;
        (reach / ux.abs()).min(row.max(0.0))
    }

    /// A reaction arrow (shaft, head, text above), a plus sign or a text; selected or hovered ones get a mark at their anchor.
    fn object(&mut self, o: &Obj) {
        let moving = self.over.drag != (0.0, 0.0) && self.over.sel_objs.contains(&o.id);
        let (dx, dy) = if moving { self.over.drag } else { (0.0, 0.0) };
        let (x, y) = self.view.to_px(o.x + dx, o.y + dy);
        let f = self.font;
        let mark = if self.over.sel_objs.contains(&o.id) {
            Some(colour::SELECT)
        } else if self.over.hover_obj == Some(o.id) {
            Some(colour::HOVER)
        } else {
            None
        };
        let w = self.m.line;
        match o.t {
            ObjKind::Arrow => {
                let e = self.view.to_px(o.x2 + dx, o.y2 + dy);
                let len = m::hypot(e.0 - x, e.1 - y).max(1e-6);
                let (ux, uy) = ((e.0 - x) / len, (e.1 - y) / len);
                if let Some(c) = mark {
                    self.line((x, y), e, w * 6.0, c);
                }
                let head = (self.view.scale * 0.3).min(len * 0.5);
                let neck = (e.0 - ux * head, e.1 - uy * head);
                let (nx, ny) = (-uy * head * 0.4, ux * head * 0.4);
                self.line((x, y), neck, w * 1.3, colour::INK);
                self.push(&[
                    TRI as f64,
                    e.0,
                    e.1,
                    neck.0 + nx,
                    neck.1 + ny,
                    neck.0 - nx,
                    neck.1 - ny,
                    colour::INK as f64,
                ]);
                if !o.text.is_empty() {
                    // above the middle of the shaft, on the side that is up on the screen
                    let up = if ux >= 0.0 { 1.0 } else { -1.0 };
                    let (mx, my) = ((x + e.0) / 2.0, (y + e.1) / 2.0);
                    let off = f * 0.8 * up;
                    self.text(
                        mx + uy * off,
                        my - ux * off,
                        f * 0.85,
                        colour::INK,
                        &crate::steplabel::display(&o.text),
                        0.0,
                    );
                }
            }
            ObjKind::Plus => {
                if let Some(c) = mark {
                    self.push(&[DISC as f64, x, y, f * 0.8, c as f64]);
                }
                self.text(x, y, f * 1.3, colour::INK, "+", 0.0);
            }
            ObjKind::Text => {
                if let Some(c) = mark {
                    let tw = width(&o.text, f);
                    self.line((x, y + f * 0.75), (x + tw, y + f * 0.75), w * 3.0, c);
                }
                self.text(x, y, f, colour::INK, &o.text, 1.0);
            }
        }
    }

    /// The level of each annotated structure, as `L` and the level (`L2b`) centred under the structure.
    fn levels(&mut self) {
        if self.doc.atoms.iter().all(|a| a.ann().is_empty()) {
            return;
        }
        for ids in crate::layout::pieces(self.doc) {
            let of: Vec<&Atom> = (self.doc.atoms.iter())
                .filter(|a| ids.contains(&a.id))
                .collect();
            let Some(now) = crate::ann::current(&Atom::ann_of(&self.doc.atoms, &ids)) else {
                continue;
            };
            let (mut x0, mut x1, mut y1) = (f64::MAX, f64::MIN, f64::MIN);
            for a in of {
                let (x, y) = self.pos(a);
                (x0, x1, y1) = (x0.min(x), x1.max(x), y1.max(y));
            }
            let f = self.font * 0.8;
            let y = y1 + self.view.scale * 0.3 + f * 0.9;
            self.text(
                (x0 + x1) / 2.0,
                y,
                f,
                colour::INK,
                &format!("L{}", now.level),
                0.0,
            );
        }
    }

    fn atoms(&mut self) {
        let r = self.font * 0.7;
        for a in &self.doc.atoms {
            let (x, y) = self.pos(a);
            if self.over.sel_atoms.contains(&a.id) {
                self.push(&[DISC as f64, x, y, r, colour::SELECT as f64]);
            } else if self.over.hover_atom == Some(a.id) {
                self.push(&[DISC as f64, x, y, r, colour::HOVER as f64]);
            }
            if self.over.warn_atoms.contains(&a.id) {
                self.push(&[RING as f64, x, y, r * 1.15, colour::WARN as f64]);
            }
            if self.over.focus == Some(a.id) {
                // two rings: the focus is seen by its shape, not by its colour alone
                self.push(&[RING as f64, x, y, r * 1.4, colour::FOCUS as f64]);
                self.push(&[RING as f64, x, y, r * 1.75, colour::FOCUS as f64]);
            }
        }
        for a in &self.doc.atoms {
            if has_label(self.doc, a) {
                self.label(a);
            }
            if a.map != 0 && self.m.maps {
                self.map_number(a);
            }
        }
    }

    /// The atom-to-atom map number at the lower right of the atom, in the small size.
    fn map_number(&mut self, a: &Atom) {
        let (x, y) = self.pos(a);
        let (f, small) = (self.font, self.m.small);
        let side = if has_label(self.doc, a) {
            width(&a.el, f) / 2.0
        } else {
            0.0
        };
        self.text(
            x + side + small * 0.15,
            y + f * 0.62,
            small,
            colour::INK,
            &a.map.to_string(),
            1.0,
        );
    }
}

/// Builds the display list of a document under a view, with the metrics of a style at that scale.
pub fn build(doc: &Document, view: View, over: &Overlay, m: Metrics) -> Scene {
    let mut p = Painter {
        doc,
        view,
        over,
        m,
        font: m.font,
        out: Scene::default(),
    };
    for b in &doc.bonds {
        p.bond(b);
    }
    p.atoms();
    p.levels();
    for o in &doc.objects {
        p.object(o);
    }
    if let Some((x1, y1, x2, y2)) = over.preview {
        p.line(
            view.to_px(x1, y1),
            view.to_px(x2, y2),
            m.line,
            colour::PREVIEW,
        );
    }
    if let Some((x, y, w, h)) = over.band {
        p.push(&[RECT as f64, x, y, w, h, colour::SELECT as f64]);
    }
    p.out
}

/// How far the ink of the label of an atom reaches from the atom's point, in the units of `style` scaled so that a bond is 1:
/// `[left, right, below, above]`, the text laid out as `Painter::label` and `Painter::group_label` do (the attachment atom of a
/// group label on the point, the rest on the side away from the bonds; hydrogens, mass number and charge where the painter puts
/// them). All zero for an atom drawn as a bare vertex. The vertical reach is a line of text, not the glyph outlines.
pub fn label_extent(doc: &Document, a: &Atom, style: &crate::settings::Style) -> [f64; 4] {
    if !has_label(doc, a) {
        return [0.0; 4];
    }
    let view = View {
        scale: 1.0,
        ox: 0.0,
        oy: 0.0,
    };
    let over = Overlay::default();
    let m = style.metrics(1.0, false);
    let p = Painter {
        doc,
        view,
        over: &over,
        m,
        font: m.font,
        out: Scene::default(),
    };
    let (f, small) = (m.font, m.small);
    let line = f * 0.6;
    if !a.lab.is_empty() {
        let [before, mid, after] = p.group_box(a).1;
        // the digits of a count are lowered by 0.3 of the font
        let lowered = f * 0.3 + small * 0.6;
        return [
            before + mid / 2.0,
            mid / 2.0 + after,
            line.max(lowered),
            line,
        ];
    }
    let q = p.parts(a);
    let wh = width("H", f);
    let mut left = q.half + q.iso;
    let mut right = q.half;
    let (mut below, mut above) = (line, line);
    if q.h > 1 {
        below = below.max(f * 0.3 + small * 0.6);
    }
    if q.h > 0 {
        if q.stack != 0.0 {
            left = left.max(wh / 2.0);
            right = right.max(q.hw - wh / 2.0);
            if q.stack > 0.0 {
                below = below.max(f * 0.95 + line);
            } else {
                above = above.max(f * 0.95 + line);
            }
        } else if q.left {
            left += q.hw;
        } else {
            right += q.hw;
        }
    }
    if q.iso != 0.0 || !q.mark.is_empty() {
        above = above.max(f * 0.35 + small * 0.6);
    }
    if !q.mark.is_empty() {
        right = right.max(q.half + if q.left { 0.0 } else { q.side() } + width(&q.mark, small));
    }
    [left, right, below, above]
}
