//! The display list: the document, a view and the editor's transient state turned into flat primitives that any surface paints
//! without knowing chemistry. The page paints it on a canvas; `svg` writes the same list as a file.
//!
//! Layout of `data` (f32): `LINE x1 y1 x2 y2 width colour` · `TEXT x y size colour string anchor` (anchor 0 middle, 1 start) ·
//! `TEXTX x y size colour string anchor face rgb font ul ulw w` (a text of one style: face 1 bold + 2 italic + 4 underline, rgb `0xrrggbb` or -1
//! for `colour`, font the index of a family name in the strings, empty for the font of the surface; `ul` the distance of the underline
//! below the centre line, `ulw` its thickness and `w` the width of the text, all in pixels, `ul` 0 for none; formatted texts only) ·
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
pub const TEXTX: f32 = 7.0;

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
    /// What a proposal adds or changes (the accent token); what it removes is struck in `WARN`.
    pub const ACCENT: f32 = 11.0;
    /// The muted ink: what the diff lens draws as a ghost (a bond the product no longer has).
    pub const MUTED: f32 = 12.0;
}

/// One mark of a proposal laid over the drawing (`proposal`), in document units. The surface owns the colours.
#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    /// A line; heavy for a struck bond.
    Line {
        a: (f64, f64),
        b: (f64, f64),
        c: f32,
        heavy: bool,
    },
    /// A small filled disc: an atom a proposal adds.
    Dot { p: (f64, f64), c: f32 },
    /// A ring of `r` font sizes: a thing a proposal changes or removes.
    Ring { p: (f64, f64), c: f32, r: f64 },
    /// The number of a hunk; the current one has a second ring (a shape, not a colour alone).
    Num {
        p: (f64, f64),
        n: u32,
        current: bool,
    },
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
/// The selection inside a text: the object, the two ends of the range in characters (`anchor` stays, `caret` moves), whether the text is
/// being edited (keys type into it) or only selected, the style the next typed character takes when it differs from the one before
/// the caret (`pending`, set by a style button with no range) and the history generation after the last typed character (the
/// next one joins that undo step while nothing came between).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSel {
    pub id: u32,
    pub anchor: usize,
    pub caret: usize,
    pub editing: bool,
    pub dragging: bool,
    pub pending: Option<crate::runs::Run>,
    pub join: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    pub sel_atoms: Vec<u32>,
    pub sel_bonds: Vec<u32>,
    pub sel_objs: Vec<u32>,
    pub hover_obj: Option<u32>,
    /// The text being edited, or the text the panel acts on (`editor_text.rs`): drawn with its selection and caret while editing.
    pub text: Option<TextSel>,
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
    /// The marks of an open proposal, laid over the drawing (filled by `Editor::scene`).
    pub marks: Vec<Mark>,
    /// The calculated values drawn under the structures (`display.values`), filled by the editor and by the exports.
    pub captions: Vec<crate::values::Caption>,
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
    /// The diff lens of the drawing (`display.lens` not off): what to tint and what to ghost.
    lens: LensPaint,
    /// The ink of the bond being painted: ink, or the accent when the lens tints it.
    ink: f32,
    /// The display of the line of the bond being painted: 0 plain, 1 dash, 2 bold dash, 3 hash, 4 bold, 5 dative (a plain line; its arrowhead is drawn by `bond`), 6 dotted (`bonddisp`).
    kind: u8,
}

/// What the diff lens paints: the atoms and bonds of the products to tint (accent) and the cleaved bonds, as the two atoms of the
/// product, drawn dashed and muted behind the bonds.
#[derive(Default)]
struct LensPaint {
    atoms: std::collections::HashSet<u32>,
    bonds: std::collections::HashSet<u32>,
    ghosts: Vec<(u32, u32)>,
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

    /// One run of a formatted text: a plain `TEXT` when it has no style, else a `TEXTX`.
    fn piece(&mut self, x: f64, y: f64, p: &crate::runs::Piece, ink: f32) {
        if p.face == 0 && p.font.is_empty() && p.rgb < 0 {
            self.text(x, y, p.size, ink, &p.text, 1.0);
            return;
        }
        let i = self.out.strings.len();
        self.out.strings.push(p.text.clone());
        self.out.strings.push(p.font.clone());
        self.push(&[
            TEXTX as f64,
            x,
            y,
            p.size,
            f64::from(ink),
            i as f64,
            1.0,
            f64::from(p.face),
            f64::from(p.rgb),
            (i + 1) as f64,
            if p.face & 4 != 0 { p.size * 0.45 } else { 0.0 },
            (p.size / 14.0).max(1.0),
            p.width,
        ]);
    }

    /// The size of a script relative to its run.
    fn script(&self) -> f64 {
        if self.m.font > 0.0 {
            self.m.small / self.m.font
        } else {
            0.7
        }
    }

    fn bond(&mut self, b: &crate::doc::Bond) {
        let (Some(p), Some(q)) = (self.doc.atom(b.a), self.doc.atom(b.b)) else {
            return;
        };
        // the lens (a display of a diff) wins; else an explicit colour; else the ink
        self.ink = if self.lens.bonds.contains(&b.id) {
            colour::ACCENT
        } else {
            crate::ink::index(&b.col()).unwrap_or(colour::INK)
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
        self.kind = match b.kind().as_str() {
            "dash" => 1,
            "bdash" => 2,
            "hash" => 3,
            "bold" => 4,
            "dative" if b.o == 1 => 5,
            "hbond" if b.o == 1 => 6,
            _ => 0,
        };
        if b.s != 0 {
            if lq && b.s != crate::doc::WEDGE_WAVY {
                // the wide end of a wedge needs more room in front of a label than a line does
                let more = (self.m.wedge * 0.5).min(len * 0.1);
                e = (e.0 - ux * more, e.1 - uy * more);
            }
            self.kind = 0;
            return self.wedge(b.s, s, e, w);
        }
        match b.o {
            2 => {
                let side = match b.pos().as_str() {
                    "c" => 0.0,
                    "l" => -1.0,
                    "r" => 1.0,
                    _ => self.double_side(b, (ux, uy)),
                };
                if side == 0.0 {
                    self.seg(shift(s, 0.5), shift(e, 0.5), w);
                    self.seg(shift(s, -0.5), shift(e, -0.5), w);
                } else {
                    self.seg(s, e, w);
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
                    self.seg(shift(s2, side), shift(e2, side), w);
                }
            }
            3 => {
                for k in [-1.0, 0.0, 1.0] {
                    self.seg(shift(s, k), shift(e, k), w);
                }
            }
            _ if self.kind == 5 => {
                // an arrow from the donor `a` to the acceptor `b`: the line stops at the base of the head, which is a solid triangle
                let head = (self.m.wedge * 1.8).min(len * 0.45);
                let half = head * 0.4;
                let base = (e.0 - ux * head, e.1 - uy * head);
                self.line(
                    s,
                    (e.0 - ux * head * 0.9, e.1 - uy * head * 0.9),
                    w,
                    self.ink,
                );
                let (p, q) = (
                    (base.0 - uy * half, base.1 + ux * half),
                    (base.0 + uy * half, base.1 - ux * half),
                );
                self.push(&[TRI as f64, e.0, e.1, p.0, p.1, q.0, q.1, self.ink as f64]);
            }
            _ => self.seg(s, e, w),
        }
    }

    /// A line of the bond being painted, in its display: plain, dashed (dashes as long as the hash spacing, bold ones heavier) or
    /// hashed (short ticks across the line).
    fn seg(&mut self, p: (f64, f64), q: (f64, f64), w: f64) {
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let len = m::hypot(dx, dy);
        if self.kind == 0 || len < 1e-6 {
            return self.line(p, q, w, self.ink);
        }
        let at = |t: f64| (p.0 + dx * t, p.1 + dy * t);
        let c = self.ink;
        if self.kind == 4 {
            // a heavy line
            return self.line(p, q, w * 2.5, c);
        }
        if self.kind == 6 {
            // dots a hash spacing apart (a round cap on a very short piece), at least two
            let n = ((len / (self.m.hash * 0.8)).round() as usize).clamp(2, 60);
            let tiny = (dx / len * w * 0.05, dy / len * w * 0.05);
            for i in 0..n {
                let m = at((i as f64 + 0.5) / n as f64);
                self.line(m, (m.0 + tiny.0, m.1 + tiny.1), w * 1.6, c);
            }
            return;
        }
        if self.kind == 3 {
            // ticks across the line, as long as the plain stroke of a hashed wedge is wide at its middle
            let n = ((len / self.m.hash).round() as usize).clamp(2, 40);
            let half = self.m.wedge * 0.5;
            let (nx, ny) = (-dy / len * half, dx / len * half);
            for i in 0..n {
                let t = (i as f64 + 0.5) / n as f64;
                let m = at(t);
                self.line((m.0 - nx, m.1 - ny), (m.0 + nx, m.1 + ny), w, c);
            }
            return;
        }
        // n segments, every other one drawn, starting and ending with a dash
        let n = ((len / self.m.hash).round() as usize).clamp(1, 80);
        let n = if n.is_multiple_of(2) { n + 1 } else { n };
        let w = if self.kind == 2 { w * 2.5 } else { w };
        for i in (0..n).step_by(2) {
            self.line(at(i as f64 / n as f64), at((i + 1) as f64 / n as f64), w, c);
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
                self.push(&[TRI as f64, s.0, s.1, p.0, p.1, q.0, q.1, self.ink as f64]);
            }
            WEDGE_DOWN => {
                let n = ((len / self.m.hash).round() as usize).clamp(3, 12);
                for i in 1..=n {
                    let t = i as f64 / n as f64;
                    self.line(at(t, t), at(t, -t), w, self.ink);
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
                    self.line(wave(i), wave(i + 1), w, self.ink);
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
            let ink = if self.lens.atoms.contains(&a.id) {
                colour::ACCENT
            } else {
                crate::ink::index(&a.col()).unwrap_or(colour::INK)
            };
            self.text(tx, y + dy, size(digit), ink, &s, 1.0);
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
        // an explicit colour overrides the element colour and the mono ink alike
        let c = if self.lens.atoms.contains(&a.id) {
            colour::ACCENT
        } else if let Some(c) = crate::ink::index(&a.col()) {
            c
        } else if self.m.mono {
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
        let ink = crate::ink::index(&o.col()).unwrap_or(colour::INK);
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
                self.line((x, y), neck, w * 1.3, ink);
                self.push(&[
                    TRI as f64,
                    e.0,
                    e.1,
                    neck.0 + nx,
                    neck.1 + ny,
                    neck.0 - nx,
                    neck.1 - ny,
                    ink as f64,
                ]);
                if !o.runs.is_empty() {
                    let up = if ux >= 0.0 { 1.0 } else { -1.0 };
                    let (mx, my) = (
                        (x + e.0) / 2.0 + uy * f * 0.8 * up,
                        (y + e.1) / 2.0 - ux * f * 0.8 * up,
                    );
                    let base = f * 0.85;
                    let laid = crate::runs::layout(&o.runs, base, self.script());
                    let lift = (laid.widths.len() - 1) as f64 * base * 1.25;
                    for p in &laid.pieces {
                        self.piece(
                            mx - laid.widths[p.line] / 2.0 + p.x,
                            my + p.dy - lift,
                            p,
                            ink,
                        );
                    }
                } else if !o.text.is_empty() {
                    // above the middle of the shaft, on the side that is up on the screen
                    let up = if ux >= 0.0 { 1.0 } else { -1.0 };
                    let (mx, my) = ((x + e.0) / 2.0, (y + e.1) / 2.0);
                    let off = f * 0.8 * up;
                    self.text(
                        mx + uy * off,
                        my - ux * off,
                        f * 0.85,
                        ink,
                        &crate::steplabel::display(&o.text),
                        0.0,
                    );
                }
            }
            ObjKind::Plus => {
                if let Some(c) = mark {
                    self.push(&[DISC as f64, x, y, f * 0.8, c as f64]);
                }
                self.text(x, y, f * 1.3, ink, "+", 0.0);
            }
            ObjKind::Text => {
                let edit = (self.over.text.clone())
                    .filter(|t| t.editing && t.id == o.id && self.over.sel_objs.contains(&o.id));
                let rs = crate::textedit::runs_of(o);
                let script = self.script();
                let laid = crate::runs::layout(&rs, f, script);
                let row = f * 1.25;
                let last = laid.widths.len() - 1;
                if let Some(t) = &edit {
                    // the selected range as a band behind the text, one per line
                    let (lo, hi) = (t.anchor.min(t.caret), t.anchor.max(t.caret));
                    let at = |p| crate::textedit::caret_at(&rs, f, script, p);
                    let ((l0, x0), (l1, x1)) = (at(lo), at(hi));
                    if lo < hi {
                        for l in l0..=l1 {
                            let a = if l == l0 { x0 } else { 0.0 };
                            let b = if l == l1 { x1 } else { laid.widths[l] };
                            let yy = y + l as f64 * row;
                            self.line(
                                (x + a, yy),
                                (x + b.max(a + 2.0), yy),
                                f * 1.15,
                                colour::SELECT,
                            );
                        }
                    }
                } else if let Some(c) = mark {
                    let bottom = y + f * 0.75 + last as f64 * row;
                    self.line((x, bottom), (x + laid.width(), bottom), w * 3.0, c);
                }
                if o.runs.is_empty() {
                    self.text(x, y, f, ink, &o.text, 1.0);
                } else {
                    for p in &laid.pieces {
                        self.piece(x + p.x, y + p.dy, p, ink);
                    }
                }
                if let Some(t) = &edit {
                    let (l, cx) = crate::textedit::caret_at(&rs, f, script, t.caret);
                    let yy = y + l as f64 * row;
                    self.line(
                        (x + cx, yy - f * 0.55),
                        (x + cx, yy + f * 0.55),
                        w * 1.6,
                        colour::FOCUS,
                    );
                }
            }
        }
    }

    /// The cleaved bonds of the lens: a thin dashed line between the two atoms of the product, in the muted ink, painted first so
    /// that it stands behind the bonds. The dashes are lines of their own (the display list has no dash primitive).
    fn ghosts(&mut self) {
        if self.lens.ghosts.is_empty() {
            return;
        }
        let w = (self.m.line * 0.6).max(0.5);
        let dash = self.font * 0.35;
        // the atoms by id, once: a scan per ghost would be quadratic
        let by_id: std::collections::HashMap<u32, &Atom> =
            self.doc.atoms.iter().map(|a| (a.id, a)).collect();
        for k in 0..self.lens.ghosts.len() {
            let (a, b) = self.lens.ghosts[k];
            let (Some(&p), Some(&q)) = (by_id.get(&a), by_id.get(&b)) else {
                continue;
            };
            let (s, e) = (self.pos(p), self.pos(q));
            let len = m::hypot(e.0 - s.0, e.1 - s.1);
            let n = ((len / dash).round() as usize).clamp(1, 200);
            // n segments, every other one drawn: starts and ends with a dash when n is odd
            let n = if n.is_multiple_of(2) { n + 1 } else { n };
            for i in (0..n).step_by(2) {
                let at = |t: f64| (s.0 + (e.0 - s.0) * t, s.1 + (e.1 - s.1) * t);
                self.line(
                    at(i as f64 / n as f64),
                    at((i + 1) as f64 / n as f64),
                    w,
                    colour::MUTED,
                );
            }
        }
    }

    /// The marks of an open proposal: added things in the accent colour, removed ones struck in the warning colour, changed ones
    /// outlined, each hunk numbered.
    fn marks(&mut self) {
        let (f, w) = (self.font, self.m.line);
        for mark in &self.over.marks {
            match *mark {
                Mark::Line { a, b, c, heavy } => {
                    let (a, b) = (self.view.to_px(a.0, a.1), self.view.to_px(b.0, b.1));
                    self.line(a, b, if heavy { w * 3.0 } else { w * 1.6 }, c);
                }
                Mark::Dot { p, c } => {
                    let (x, y) = self.view.to_px(p.0, p.1);
                    self.push(&[DISC as f64, x, y, f * 0.3, c as f64]);
                }
                Mark::Ring { p, c, r } => {
                    let (x, y) = self.view.to_px(p.0, p.1);
                    self.push(&[RING as f64, x, y, f * r, c as f64]);
                }
                Mark::Num { p, n, current } => {
                    let (x, y) = self.view.to_px(p.0, p.1);
                    self.push(&[RING as f64, x, y, f * 0.75, colour::ACCENT as f64]);
                    if current {
                        self.push(&[RING as f64, x, y, f * 1.0, colour::ACCENT as f64]);
                    }
                    self.text(x, y, f * 0.9, colour::ACCENT, &n.to_string(), 0.0);
                }
            }
        }
    }

    /// The calculated values under each structure (`display.values`), below the level when there is one.
    fn captions(&mut self) {
        let f = crate::values::caption_font(self.font);
        for c in &self.over.captions {
            let (x, y) = self.view.to_px(c.x, c.y);
            let first = y + crate::values::caption_first(self.view.scale, f, c.below);
            for (k, line) in c.lines.iter().enumerate() {
                let at = first + k as f64 * crate::values::caption_step(f);
                self.text(x, at, f, colour::INK, line, 0.0);
            }
        }
    }

    /// The name and the level of each structure that has them, centred under the structure: `TP3 \u{b7} L2b`, or `L2b` (a level
    /// alone), or `TP3` (a name alone). The name is the data field `TP_ID`; the level is the current entry of the annotation.
    fn levels(&mut self) {
        if self
            .doc
            .atoms
            .iter()
            .all(|a| a.ann().is_empty() && !a.more.iter().any(|m| m.0 == "data"))
        {
            return;
        }
        // one pass over the atoms gives each structure its atoms (the cost stays linear in the drawing)
        let pieces = crate::layout::pieces(self.doc);
        let mut owner: std::collections::HashMap<u32, usize> =
            std::collections::HashMap::with_capacity(self.doc.atoms.len());
        for (k, p) in pieces.iter().enumerate() {
            owner.extend(p.iter().map(|&i| (i, k)));
        }
        let mut members: Vec<Vec<&Atom>> = vec![Vec::new(); pieces.len()];
        for a in &self.doc.atoms {
            if let Some(&k) = owner.get(&a.id) {
                members[k].push(a);
            }
        }
        for of in members {
            let texts: Vec<String> = of.iter().map(|a| a.ann()).collect();
            let level = crate::ann::current(&crate::ann::join(texts.iter().map(String::as_str)))
                .map(|now| format!("L{}", now.level));
            let name = of.iter().find_map(|a| {
                a.data()
                    .into_iter()
                    .find(|(k, v)| k == "TP_ID" && !v.is_empty())
                    .map(|(_, v)| v)
            });
            let label = match (name, level) {
                (Some(n), Some(l)) => format!("{n} \u{b7} {l}"),
                (Some(n), None) => n,
                (None, Some(l)) => l,
                (None, None) => continue,
            };
            let (mut x0, mut x1, mut y1) = (f64::MAX, f64::MIN, f64::MIN);
            for a in of {
                let (x, y) = self.pos(a);
                (x0, x1, y1) = (x0.min(x), x1.max(x), y1.max(y));
            }
            let f = self.font * 0.8;
            let y = y1 + self.view.scale * 0.3 + f * 0.9;
            self.text((x0 + x1) / 2.0, y, f, colour::INK, &label, 0.0);
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

/// What the lens of the drawing paints under `mode`: nothing when it is off or when a pair cannot be compared (`tp::lenses`).
fn lens_paint(doc: &Document, mode: crate::settings::LensMode) -> LensPaint {
    use crate::settings::LensMode;
    let mut out = LensPaint::default();
    if mode == LensMode::Off {
        return out;
    }
    let Ok(all) = crate::tp::lenses(doc) else {
        return out;
    };
    for l in &all.lenses {
        out.atoms.extend(l.added.iter().chain(&l.changed));
        out.bonds
            .extend(l.added_bonds.iter().chain(&l.changed_bonds));
        if mode == LensMode::TintGhost {
            out.ghosts.extend(l.cleaved.iter().copied());
        }
    }
    out
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
        lens: lens_paint(doc, m.lens),
        ink: colour::INK,
        kind: 0,
    };
    p.ghosts();
    for b in &doc.bonds {
        p.bond(b);
    }
    p.atoms();
    p.levels();
    p.captions();
    for o in &doc.objects {
        p.object(o);
    }
    p.marks();
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
        lens: LensPaint::default(),
        ink: colour::INK,
        kind: 0,
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

#[cfg(test)]
#[path = "../tests/support/timing.rs"]
mod timing;

#[cfg(test)]
mod tests {
    use super::timing;
    use super::*;
    /// `n` pairs: a triangle (maps 1, 2, 3) and a chain with the same maps, which lacks the bond 3-1 (one ghost per pair).
    fn pairs(n: u32) -> Document {
        let (mut atoms, mut bonds, mut objects) = (Vec::new(), Vec::new(), Vec::new());
        let (mut a, mut b) = (0u32, 0u32);
        for k in 0..n {
            // a grid, so that the coordinates stay in range however many pairs
            let (x0, y0) = ((k % 100) as f64 * 20.0, (k / 100) as f64 * 10.0);
            let first = a + 1;
            for (dx, ring) in [(0.0, true), (8.0, false)] {
                let start = a;
                for i in 0..3u32 {
                    a += 1;
                    atoms.push(format!(
                        r#"{{"id":{a},"el":"C","x":{},"y":{},"map":{}}}"#,
                        x0 + dx + i as f64,
                        y0 + (i % 2) as f64,
                        i + 1
                    ));
                }
                for (i, j) in [(1u32, 2u32), (2, 3), (3, 1)] {
                    if ring || i != 3 {
                        b += 1;
                        bonds.push(format!(
                            r#"{{"id":{},"a":{},"b":{},"o":1}}"#,
                            1_000_000 + b,
                            start + i,
                            start + j
                        ));
                    }
                }
            }
            objects.push(format!(
                r#"{{"id":{},"t":"arrow","x":{},"y":{y0},"x2":{},"y2":{y0},"from":{first},"to":{}}}"#,
                2_000_000 + k,
                x0 + 4.0,
                x0 + 7.5,
                first + 3
            ));
        }
        Document::from_json(&format!(
            r#"{{"format":"{}","next":3000000,"atoms":[{}],"bonds":[{}],"objects":[{}]}}"#,
            crate::brand::DOCUMENT_ID,
            atoms.join(","),
            bonds.join(","),
            objects.join(",")
        ))
        .unwrap()
    }

    /// The time of `reps` paintings of the ghosts of `n` pairs; the drawing is built outside the clock.
    fn cost(n: u32, reps: u32) -> f64 {
        let doc = pairs(n);
        // the ghosts as the lens gives them (the product's atom 3 with its atom 1: the bond 3-1 it lacks), fed to the painter
        // directly: the drawing here is larger than the lens itself accepts (`tp::MAX_LENS_DOC`), and only the painting is timed
        timing::best_of(1, 5, || {
            for _ in 0..reps {
                let mut p = Painter {
                    doc: &doc,
                    view: View {
                        scale: 20.0,
                        ox: 0.0,
                        oy: 0.0,
                    },
                    over: &Overlay::default(),
                    m: crate::settings::Style::default().metrics(20.0, true),
                    font: 10.0,
                    out: Scene::default(),
                    lens: LensPaint {
                        ghosts: (0..n).map(|k| (6 * k + 4, 6 * k + 6)).collect(),
                        ..LensPaint::default()
                    },
                    ink: colour::INK,
                    kind: 0,
                };
                p.ghosts();
                assert!(!p.out.data.is_empty());
            }
        })
        .as_secs_f64()
    }

    #[test]
    fn the_lens_painting_is_linear_in_the_drawing() {
        // four times the pairs must cost about four times as much, not sixteen. A scan of the atoms per ghost (as `Document::atom`
        // does) takes this to a ratio of 16.
        let (small, big) = (cost(1000, 3), cost(4000, 3));
        assert!(
            big < 8.0 * small,
            "1000 pairs {small:.3} s, 4000 pairs {big:.3} s"
        );
    }
}
