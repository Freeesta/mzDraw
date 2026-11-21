//! Arrows that know their structures: the routed ends of an anchored arrow, and the follow of a command.
//!
//! An arrow of `mzdraw/1` may carry `from` and `to`, the id of one atom of each structure (the connected piece that holds the atom).
//! `route` gives the operations that put such an arrow on its routed ends: the segment between the centres of the two bounding
//! boxes (labels included, as an estimate from the width table of `scene`), clipped at each box inflated by the margin. `follow`
//! gives the same operations for every arrow whose structures a list of operations changes: the editor commands append them to
//! their own step, so that the log carries them and a replay gives the same bytes. A bare operation never follows.

use crate::doc::{round4, Atom, Document, ObjKind};
use crate::ops::Op;

/// An axis-aligned box: x0, y0, x1, y1 (bond lengths).
type Rect = [f64; 4];

/// The ink of an atom label that sticks out of its point, in bond lengths: half the width on each side and a little more than half
/// the font height above and below. An estimate that errs on the large side (the hydrogens may stand on either side).
fn label_reach(doc: &Document, a: &Atom, font: f64) -> (f64, f64) {
    if !crate::scene::has_label(doc, a) {
        return (0.0, 0.0);
    }
    let mut text = if a.lab.is_empty() {
        a.el.clone()
    } else {
        a.lab.clone()
    };
    let h = doc.hydrogens(a);
    if h > 0 {
        text.push('H');
        if h > 1 {
            text.push_str(&h.to_string());
        }
    }
    if a.iso != 0 {
        text.push_str(&a.iso.to_string());
    }
    if a.chg != 0 {
        text.push_str("2+");
    }
    (crate::scene::width(&text, font), font * 0.7)
}

/// The box of the structure that holds `atom`: its atoms and their labels. `None` when no atom has that id.
pub fn structure_box(doc: &Document, atom: u32, font: f64) -> Option<Rect> {
    doc.atom(atom)?;
    let piece = crate::layout::pieces(doc)
        .into_iter()
        .find(|p| p.contains(&atom))?;
    let mut r = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for a in piece.iter().filter_map(|&i| doc.atom(i)) {
        let (rx, ry) = label_reach(doc, a, font);
        r = [
            r[0].min(a.x - rx),
            r[1].min(a.y - ry),
            r[2].max(a.x + rx),
            r[3].max(a.y + ry),
        ];
    }
    Some(r)
}

/// How far from its centre a ray along the unit vector (`ux`, `uy`) leaves the box inflated by `margin`.
pub fn exit_distance(b: Rect, margin: f64, ux: f64, uy: f64) -> f64 {
    let (hx, hy) = ((b[2] - b[0]) / 2.0 + margin, (b[3] - b[1]) / 2.0 + margin);
    let tx = if ux.abs() > 1e-12 {
        hx / ux.abs()
    } else {
        f64::MAX
    };
    let ty = if uy.abs() > 1e-12 {
        hy / uy.abs()
    } else {
        f64::MAX
    };
    tx.min(ty)
}

/// The routed ends (tail, head) of the segment between the centres of two boxes, clipped at each box inflated by `margin`; `None`
/// when the inflated boxes touch along it (nothing is left to draw) or the centres coincide.
pub fn clip(a: Rect, b: Rect, margin: f64) -> Option<((f64, f64), (f64, f64))> {
    let ca = ((a[0] + a[2]) / 2.0, (a[1] + a[3]) / 2.0);
    let cb = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
    let len = crate::m::hypot(cb.0 - ca.0, cb.1 - ca.1);
    if len < 1e-9 {
        return None;
    }
    let (ux, uy) = ((cb.0 - ca.0) / len, (cb.1 - ca.1) / len);
    let (ta, tb) = (
        exit_distance(a, margin, ux, uy),
        exit_distance(b, margin, ux, uy),
    );
    // the head must come after the tail, with a length a valid arrow has
    if ta + tb + 1e-3 >= len {
        return None;
    }
    Some((
        (ca.0 + ux * ta, ca.1 + uy * ta),
        (cb.0 - ux * tb, cb.1 - uy * tb),
    ))
}

/// The boxes of the two structures an arrow is anchored to; `None` unless both ends are anchored to existing atoms of two
/// different structures (a dangling or a free end is used as free, and an arrow with a free end is not routed).
fn boxes(doc: &Document, from: u32, to: u32, font: f64) -> Option<(Rect, Rect)> {
    if from == 0 || to == 0 {
        return None;
    }
    let a = structure_box(doc, from, font)?;
    let b = structure_box(doc, to, font)?;
    let same = crate::layout::pieces(doc)
        .iter()
        .any(|p| p.contains(&from) && p.contains(&to));
    (!same).then_some((a, b))
}

/// The operations that put the anchored arrow `id` on its routed ends: a `move` of the arrow for the tail, then an `obj.set` of
/// `x2`, `y2`; none when it is already there, when it is not an arrow or is not anchored at both ends. `margin` and `font` are in
/// bond lengths (`Style::pathway_margin`, `font_size / bond_length`).
pub fn route(doc: &Document, id: u32, margin: f64, font: f64) -> Vec<Op> {
    let Some(o) = doc.object(id).filter(|o| o.t == ObjKind::Arrow) else {
        return Vec::new();
    };
    let Some((a, b)) = boxes(doc, o.from, o.to, font) else {
        return Vec::new();
    };
    let Some((tail, head)) = clip(a, b, margin) else {
        return Vec::new();
    };
    let (tx, ty) = (round4(tail.0), round4(tail.1));
    let (hx, hy) = (round4(head.0), round4(head.1));
    let mut ops = Vec::new();
    let (dx, dy) = (round4(tx - o.x), round4(ty - o.y));
    let (mut x2, mut y2) = (o.x2, o.y2);
    if dx != 0.0 || dy != 0.0 {
        ops.push(Op::Move {
            atoms: Vec::new(),
            objects: vec![id],
            dx,
            dy,
        });
        (x2, y2) = (round4(x2 + dx), round4(y2 + dy));
    }
    if x2 != hx || y2 != hy {
        ops.push(Op::ObjSet {
            id,
            text: None,
            x2: Some(hx),
            y2: Some(hy),
            from: None,
            to: None,
        });
    }
    ops
}

/// The route operations for every anchored arrow whose structures `ops` change (a box that moved, grew, shrank or appeared),
/// computed on the document as `ops` leave it. Empty when `ops` is refused (the step reports that) or touches no anchored structure.
pub fn follow(doc: &Document, ops: &[Op], margin: f64, font: f64) -> Vec<Op> {
    if !doc.objects.iter().any(|o| o.from != 0 || o.to != 0) {
        return Vec::new();
    }
    let mut after = doc.clone();
    if after.apply(ops).is_err() {
        return Vec::new();
    }
    let box_of =
        |d: &Document, atom: u32| (atom != 0).then(|| structure_box(d, atom, font)).flatten();
    let mut out = Vec::new();
    for o in doc.objects.iter().filter(|o| o.t == ObjKind::Arrow) {
        let changed = [o.from, o.to]
            .into_iter()
            .any(|a| box_of(doc, a) != box_of(&after, a));
        if changed {
            out.extend(route(&after, o.id, margin, font));
        }
    }
    out
}
