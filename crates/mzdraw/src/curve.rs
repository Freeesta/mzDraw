//! Lone pairs and electron-pushing curved arrows. A lone-pair count sits on an atom (`lp`, 0-4 pairs, drawn as dot pairs at the free
//! sides); a curve object (`t: "curve"`) has two ends, each a point or an anchor: an atom, a bond, or the lone pair of an atom
//! (the end flag `fl` / `tl`). The geometry of an anchored end is computed from the document whenever it is drawn, so an atom that
//! moves moves the curve without an operation. An anchor that is removed turns the end into the point it was at (`Remove` records
//! the object it changes, so the undo is exact). The head is `full` (a pair) or `half` (one electron); `bend` is the bow of the
//! curve as a fraction of its chord, positive to the left of the way it goes (y up).

use crate::doc::{Atom, Document, Obj, ObjKind};
use crate::m;
use serde_json::Value;
use std::f64::consts::PI;

/// The most lone pairs one atom carries.
pub const MAX_LP: u8 = 4;
/// The largest bow of a curve, as a fraction of its chord.
pub const MAX_BEND: f64 = 3.0;
/// The bow a new curve gets (its sign is chosen on the free side).
pub const DEFAULT_BEND: f64 = 0.4;
/// How far from its atom a lone pair is drawn, and the gap an arrow leaves at an atom, in bond lengths.
pub const LP_REACH: f64 = 0.3;
const ATOM_GAP: f64 = 0.28;

/// The attributes of a curve beyond its four coordinates and two anchors.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Curve {
    /// One electron (a fish-hook head) instead of a pair.
    pub half: bool,
    pub bend: f64,
    /// The start (`fl`) or the end (`tl`) is the lone pair of the atom `from` / `to`.
    pub fl: bool,
    pub tl: bool,
}

/// A change of a curve: what an `obj.set` carries.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CurveSet {
    pub half: Option<bool>,
    pub bend: Option<f64>,
    pub fl: Option<bool>,
    pub tl: Option<bool>,
}

impl Curve {
    pub fn valid(&self) -> bool {
        self.bend.is_finite() && self.bend.abs() <= MAX_BEND
    }
    pub(crate) fn apply(&self, s: &CurveSet) -> Curve {
        Curve {
            half: s.half.unwrap_or(self.half),
            bend: s.bend.map_or(self.bend, crate::doc::round4),
            fl: s.fl.unwrap_or(self.fl),
            tl: s.tl.unwrap_or(self.tl),
        }
    }
}

impl From<Curve> for CurveSet {
    /// What an `obj.add` writes: only what is not the default.
    fn from(c: Curve) -> CurveSet {
        CurveSet {
            half: c.half.then_some(true),
            bend: (c.bend != 0.0).then_some(c.bend),
            fl: c.fl.then_some(true),
            tl: c.tl.then_some(true),
        }
    }
}

impl CurveSet {
    pub fn is_empty(&self) -> bool {
        *self == CurveSet::default()
    }
}

fn field<'a>(more: &'a [(String, String)], k: &str) -> Option<&'a str> {
    more.iter().find(|m| m.0 == k).map(|m| m.1.as_str())
}

impl Atom {
    /// The lone pairs drawn on this atom (0-4).
    pub fn lp(&self) -> u8 {
        field(&self.more, "lp")
            .and_then(|t| t.parse::<u8>().ok())
            .filter(|&n| n <= MAX_LP)
            .unwrap_or(0)
    }

    pub(crate) fn set_lp(&mut self, n: u8) {
        self.more.retain(|m| m.0 != "lp");
        if n > 0 {
            self.more.push(("lp".into(), n.min(MAX_LP).to_string()));
            m::sort(&mut self.more);
        }
    }
}

impl Obj {
    /// The attributes of a curve (the default for any other object).
    pub fn curve(&self) -> Curve {
        let num = field(&self.more, "bend")
            .and_then(|t| t.parse::<f64>().ok())
            .unwrap_or(0.0);
        let flag = |k: &str| field(&self.more, k) == Some("true");
        Curve {
            half: field(&self.more, "head")
                .and_then(|t| serde_json::from_str::<Value>(t).ok())
                .is_some_and(|v| v.as_str() == Some("half")),
            bend: num,
            fl: flag("fl"),
            tl: flag("tl"),
        }
    }

    pub(crate) fn set_curve(&mut self, c: &Curve) {
        self.more
            .retain(|m| !["head", "bend", "fl", "tl"].contains(&m.0.as_str()));
        if c.half {
            self.more.push(("head".into(), "\"half\"".into()));
        }
        if c.bend != 0.0 {
            self.more
                .push(("bend".into(), Value::from(c.bend).to_string()));
        }
        for (k, on) in [("fl", c.fl), ("tl", c.tl)] {
            if on {
                self.more.push((k.into(), "true".into()));
            }
        }
        m::sort(&mut self.more);
    }
}

/// Whether the anchors of an object exist in the document (any object but a curve: always).
pub(crate) fn anchors_exist(doc: &Document, o: &Obj) -> bool {
    o.t != ObjKind::Curve
        || [(o.from, o.curve().fl), (o.to, o.curve().tl)]
            .iter()
            .all(|&(id, lone)| anchor_ok(doc, id, lone))
}

/// An end is a point (`id` 0), or an atom, a bond, or (`lone`) the lone pair of an atom (with no pair left it is drawn at the atom).
pub(crate) fn anchor_ok(doc: &Document, id: u32, lone: bool) -> bool {
    match (id, lone) {
        (0, false) => true,
        (0, true) => false,
        (_, true) => doc.atom(id).is_some(),
        _ => doc.atom(id).is_some() || doc.bond(id).is_some(),
    }
}

/// The number of segments a curve is drawn and hit-tested as.
pub const CURVE_STEPS: u32 = 24;

/// The directions (radians, y up) of the `n` lone pairs of an atom: in the gaps between its bonds, the widest first, spread evenly
/// when a gap takes several.
pub fn lp_dirs(doc: &Document, id: u32, n: u8) -> Vec<f64> {
    let Some(a) = doc.atom(id) else {
        return Vec::new();
    };
    let mut ang: Vec<f64> = doc
        .neighbours(id)
        .iter()
        .filter_map(|&b| doc.atom(b))
        .map(|b| m::atan2(b.y - a.y, b.x - a.x))
        .collect();
    m::sort_by(&mut ang, |p, q| p.total_cmp(q).is_lt());
    // gaps as (start angle, size)
    let gaps: Vec<(f64, f64)> = match ang.len() {
        0 => vec![(-PI / 2.0 + 0.0, 2.0 * PI)],
        k => (0..k)
            .map(|i| {
                let (s, e) = (ang[i], ang[(i + 1) % k]);
                let size = if k == 1 {
                    2.0 * PI
                } else {
                    (e - s + 2.0 * PI) % (2.0 * PI)
                };
                (s, size)
            })
            .collect(),
    };
    let mut take = vec![0u32; gaps.len()];
    let mut order: Vec<usize> = Vec::new();
    for _ in 0..n {
        let mut best = 0;
        for i in 1..gaps.len() {
            let (a, b) = (
                gaps[i].1 / f64::from(take[i] + 1),
                gaps[best].1 / f64::from(take[best] + 1),
            );
            if a > b + 1e-9 {
                best = i;
            }
        }
        take[best] += 1;
        order.push(best);
    }
    // the first pair sits in the widest gap, at its middle when alone there
    let mut out = Vec::new();
    for &g in &order {
        if out.len() >= usize::from(n) {
            break;
        }
        let (s, size) = gaps[g];
        let c = take[g];
        let seen = order[..out.len()].iter().filter(|&&x| x == g).count() as u32;
        out.push(s + size * f64::from(seen + 1) / f64::from(c + 1));
    }
    out
}

/// The lone pairs an atom has by the valence rules: the electrons of its group (less its charge) minus those its bonds, hydrogens
/// and radical electrons use, in pairs. Atoms of the groups 13-17 and a labelled atom's element only; 0 for any other.
pub fn lp_from_valence(doc: &Document, a: &Atom) -> u8 {
    let ve: i32 = match a.el.as_str() {
        "B" | "Al" => 3,
        "C" | "Si" => 4,
        "N" | "P" | "As" => 5,
        "O" | "S" | "Se" | "Te" => 6,
        "F" | "Cl" | "Br" | "I" => 7,
        _ => return 0,
    };
    let used = doc.order_sum(a.id) as i32 + doc.hydrogens(a) as i32 + i32::from(a.rad);
    let pairs = (ve - i32::from(a.chg) - used).max(0) / 2;
    pairs.min(i32::from(MAX_LP)) as u8
}

fn mid(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

/// The place an anchor stands at, before the gap at an atom is taken: an atom's point, the middle of a bond, the point a lone pair
/// is drawn at.
fn anchor_point(doc: &Document, id: u32, lone: bool, fallback: (f64, f64)) -> (f64, f64) {
    if let Some(a) = doc.atom(id) {
        if lone && let Some(d) = lp_dirs(doc, id, a.lp()).first() {
            return (a.x + LP_REACH * m::cos(*d), a.y + LP_REACH * m::sin(*d));
        }
        return (a.x, a.y);
    }
    if let Some(b) = doc.bond(id)
        && let (Some(p), Some(q)) = (doc.atom(b.a), doc.atom(b.b))
    {
        return mid((p.x, p.y), (q.x, q.y));
    }
    fallback
}

/// The point a curve end anchored on the bond `id` stands at (the middle of the bond).
pub(crate) fn shape_anchor(doc: &Document, id: u32) -> (f64, f64) {
    anchor_point(doc, id, false, (0.0, 0.0))
}

/// The `atom.set lp` operations that give each of `atoms` (all atoms when empty) the lone pairs of the valence rules; the ones
/// that already have them are left out.
pub fn lone_pair_ops(doc: &Document, atoms: &[u32]) -> Vec<crate::ops::Op> {
    (doc.atoms.iter())
        .filter(|a| atoms.is_empty() || atoms.contains(&a.id))
        .filter_map(|a| {
            let n = lp_from_valence(doc, a);
            (n != a.lp()).then_some({
                crate::ops::Op::AtomSet {
                    id: a.id,
                    el: None,
                    chg: None,
                    iso: None,
                    rad: None,
                    h: None,
                    lab: None,
                    map: None,
                    lp: Some(n),
                }
            })
        })
        .collect()
}

/// The two ends of a curve in document units (y up), anchors resolved; `shift(id)` is the displacement a drag gives an anchor
/// (an atom id, or a bond id given as the one of its atoms), `free` the one of the free ends.
pub fn ends(
    doc: &Document,
    o: &Obj,
    shift: &dyn Fn(u32) -> (f64, f64),
    free: (f64, f64),
) -> ((f64, f64), (f64, f64)) {
    let c = o.curve();
    let (p0, p1) = ((o.x, o.y), (o.x2, o.y2));
    let one = |id: u32, lone: bool, fb: (f64, f64)| {
        let s = if id == 0 {
            free
        } else if doc.atom(id).is_some() {
            shift(id)
        } else if let Some(b) = doc.bond(id) {
            let (u, v) = (shift(b.a), shift(b.b));
            ((u.0 + v.0) / 2.0, (u.1 + v.1) / 2.0)
        } else {
            free
        };
        let p = anchor_point(doc, id, lone, fb);
        (p.0 + s.0, p.1 + s.1)
    };
    let (raw_a, raw_b) = (one(o.from, c.fl, p0), one(o.to, c.tl, p1));
    let (mut a, mut b) = (raw_a, raw_b);
    // an atom end (not a lone pair) keeps a gap from the atom, towards the other end as it stands now
    for (end, other, id, lone) in [(&mut a, raw_b, o.from, c.fl), (&mut b, raw_a, o.to, c.tl)] {
        if id != 0 && !lone && doc.atom(id).is_some() {
            let (dx, dy) = (other.0 - end.0, other.1 - end.1);
            let len = m::hypot(dx, dy);
            if len > ATOM_GAP * 2.0 {
                *end = (end.0 + dx / len * ATOM_GAP, end.1 + dy / len * ATOM_GAP);
            }
        }
    }
    (a, b)
}

/// The control point of the quadratic curve from `p` to `q` bowed by `bend`.
pub fn control(p: (f64, f64), q: (f64, f64), bend: f64) -> (f64, f64) {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let mid = mid(p, q);
    // the middle of the curve is half way from the chord to the control point
    (mid.0 - dy * bend, mid.1 + dx * bend)
}

/// The point of the curve at `t` (0-1).
pub fn at(p: (f64, f64), c: (f64, f64), q: (f64, f64), t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    (
        u * u * p.0 + 2.0 * u * t * c.0 + t * t * q.0,
        u * u * p.1 + 2.0 * u * t * c.1 + t * t * q.1,
    )
}

/// The curve as seen: ends and control point, no drag.
pub fn shape(doc: &Document, o: &Obj) -> ((f64, f64), (f64, f64), (f64, f64)) {
    let (p, q) = ends(doc, o, &|_| (0.0, 0.0), (0.0, 0.0));
    (p, control(p, q, o.curve().bend), q)
}

/// The bow that puts the middle of a curve from `p` to `q` where `pt` is (the drag of its middle handle), kept within `MAX_BEND`.
pub fn bend_through(p: (f64, f64), q: (f64, f64), pt: (f64, f64)) -> f64 {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let len2 = (dx * dx + dy * dy).max(1e-9);
    let m0 = mid(p, q);
    // the middle of the curve is mid + n * bend / 2 with n = (-dy, dx) (see `control`)
    let b = ((pt.0 - m0.0) * -dy + (pt.1 - m0.1) * dx) / len2;
    (b * 2.0).clamp(-MAX_BEND, MAX_BEND)
}

/// The bow for a new curve from `p` to `q`: towards the side of the chord with fewer atoms ("the free side"), `DEFAULT_BEND` large.
pub fn free_side_bend(doc: &Document, p: (f64, f64), q: (f64, f64)) -> f64 {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let side: f64 = doc
        .atoms
        .iter()
        .map(|a| (a.x - p.0) * -dy + (a.y - p.1) * dx)
        .map(|s| {
            if s > 1e-9 {
                1.0
            } else if s < -1e-9 {
                -1.0
            } else {
                0.0
            }
        })
        .sum();
    // more atoms on the left: bow to the right
    if side > 0.0 {
        -DEFAULT_BEND
    } else {
        DEFAULT_BEND
    }
}

/// The anchors the removal of atoms and bonds takes away from curves, as the curves after the change: an end whose anchor goes
/// becomes the point it stood at. `(index in the pool, object)` for each curve that changes.
pub(crate) fn detach(
    doc: &Document,
    gone_atoms: &std::collections::HashSet<u32>,
    gone_bonds: &std::collections::HashSet<u32>,
) -> Vec<(usize, Obj)> {
    let mut out = Vec::new();
    let gone = |id: u32| {
        id != 0
            && (gone_atoms.contains(&id)
                || gone_bonds.contains(&id)
                || doc
                    .bond(id)
                    .is_some_and(|b| gone_atoms.contains(&b.a) || gone_atoms.contains(&b.b)))
    };
    for (i, o) in doc.objects.iter().enumerate() {
        if o.t != ObjKind::Curve {
            continue;
        }
        let c = o.curve();
        let (fa, ta) = (gone(o.from), gone(o.to));
        if !fa && !ta {
            continue;
        }
        let (p, q) = ends(doc, o, &|_| (0.0, 0.0), (0.0, 0.0));
        let mut n = o.clone();
        if fa {
            (n.x, n.y, n.from) = (crate::doc::round4(p.0), crate::doc::round4(p.1), 0);
        }
        if ta {
            (n.x2, n.y2, n.to) = (crate::doc::round4(q.0), crate::doc::round4(q.1), 0);
        }
        n.set_curve(&Curve {
            fl: c.fl && !fa,
            tl: c.tl && !ta,
            ..c
        });
        out.push((i, n));
    }
    out
}
