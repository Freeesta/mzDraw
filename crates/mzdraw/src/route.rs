//! Arrows that know their structures: the routed ends of an anchored arrow, and the follow of a command.
//!
//! An arrow of `mzdraw/1` may carry `from` and `to`, the id of one atom of each structure (the connected piece that holds the atom).
//! `route` gives the operations that put such an arrow on its routed ends: the segment between the centres of the two bounding
//! boxes (labels included: the ink laid out as the painter does, `scene::label_extent`), clipped at each box inflated by the margin. `follow`
//! gives the same operations for every arrow whose structures a list of operations changes: the editor commands append them to
//! their own step, so that the log carries them and a replay gives the same bytes. A bare operation never follows.

use crate::doc::{round4, Document, ObjKind};
use crate::ops::{Op, OpError};
use crate::settings::Style;

/// An axis-aligned box: x0, y0, x1, y1 (bond lengths).
pub type Rect = [f64; 4];

/// The box of the structure that holds `atom`: its atoms and the ink of their labels (`scene::label_extent`, the layout the
/// painter uses). `None` when no atom has that id.
pub fn structure_box(doc: &Document, atom: u32, style: &Style) -> Option<Rect> {
    Structures::new(doc, style).box_of(atom)
}

/// The structures of a document and their boxes, computed once: the connected pieces (`layout::pieces`, the costly part) and the box of
/// each. Every function of this module that needs the box of a structure takes one of these, so that a command pays for the pieces once
/// per call, not once per arrow or per question. The boxes depend on the atoms and the style only: they stay true across operations
/// that change anchors (`obj.set`) or arrows.
pub struct Structures {
    pieces: Vec<Vec<u32>>,
    boxes: Vec<Rect>,
    of_atom: std::collections::HashMap<u32, usize>,
}

impl Structures {
    /// The pieces of `doc` and the box of each.
    pub fn new(doc: &Document, style: &Style) -> Structures {
        let pieces = crate::layout::pieces(doc);
        let of_atom: std::collections::HashMap<u32, usize> = (pieces.iter().enumerate())
            .flat_map(|(i, p)| p.iter().map(move |&a| (a, i)))
            .collect();
        // the ink of a label depends on the atom and its bonds only, so each box is computed on a document of its own structure: the
        // cost of the lookups is that of the structure, not of the drawing (one pass puts the atoms and the bonds in their piece)
        let mut subs: Vec<Document> = pieces
            .iter()
            .map(|_| Document {
                next: doc.next,
                ..Document::default()
            })
            .collect();
        for a in &doc.atoms {
            if let Some(&i) = of_atom.get(&a.id) {
                subs[i].atoms.push(a.clone());
            }
        }
        for b in &doc.bonds {
            if let Some(&i) = of_atom.get(&b.a) {
                subs[i].bonds.push(b.clone());
            }
        }
        let boxes = (pieces.iter().zip(&subs))
            .map(|(p, sub)| piece_box(sub, p, style))
            .collect();
        Structures {
            pieces,
            boxes,
            of_atom,
        }
    }

    /// The structures, as lists of atom ids (`layout::pieces`, in its order).
    pub fn pieces(&self) -> &[Vec<u32>] {
        &self.pieces
    }

    /// The box of the structure that holds `atom`; `None` when no atom has that id.
    pub fn box_of(&self, atom: u32) -> Option<Rect> {
        self.of_atom.get(&atom).map(|&i| self.boxes[i])
    }

    /// Whether the two atoms are in the same structure.
    pub fn same(&self, a: u32, b: u32) -> bool {
        matches!((self.of_atom.get(&a), self.of_atom.get(&b)), (Some(x), Some(y)) if x == y)
    }

    /// The smallest atom id of the structure whose box (`HIT_SLACK` more on every side) holds the point; see `structure_at`.
    pub fn at(&self, x: f64, y: f64) -> Option<u32> {
        self.nearest(x, y, HIT_SLACK).map(|(id, _)| id)
    }

    /// The structure nearest to the point by box distance within `reach`; see `nearest_structure`.
    pub fn nearest(&self, x: f64, y: f64, reach: f64) -> Option<(u32, f64)> {
        let mut best: Option<(u32, f64)> = None;
        for (piece, &b) in self.pieces.iter().zip(&self.boxes) {
            let id = smallest(piece);
            let d = box_distance(b, x, y);
            if d > reach + 1e-9 {
                continue;
            }
            // the distance is compared as the documents store numbers: to four decimals
            let better = match best {
                None => true,
                Some((bid, bd)) => round4(d) < round4(bd) || (round4(d) == round4(bd) && id < bid),
            };
            if better {
                best = Some((id, d));
            }
        }
        best
    }
}

/// The box of one structure, given as the ids of its atoms: the atoms and the ink of their labels.
pub fn piece_box(doc: &Document, piece: &[u32], style: &Style) -> Rect {
    let mut r = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for a in piece.iter().filter_map(|&i| doc.atom(i)) {
        let [l, rt, below, above] = crate::scene::label_extent(doc, a, style);
        r = [
            r[0].min(a.x - l),
            r[1].min(a.y - below),
            r[2].max(a.x + rt),
            r[3].max(a.y + above),
        ];
    }
    r
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
fn boxes(st: &Structures, from: u32, to: u32) -> Option<(Rect, Rect)> {
    if from == 0 || to == 0 {
        return None;
    }
    let a = st.box_of(from)?;
    let b = st.box_of(to)?;
    (!st.same(from, to)).then_some((a, b))
}

/// The operations that put the anchored arrow `id` on its routed ends: a `move` of the arrow for the tail, then an `obj.set` of
/// `x2`, `y2`; none when it is already there, when it is not an arrow or is not anchored at both ends. The gap is `Style::pathway_margin` (bond
/// lengths), the label ink comes from the style too.
pub fn route(doc: &Document, id: u32, style: &Style) -> Vec<Op> {
    if !doc.objects.iter().any(|o| o.id == id && o.from != 0) {
        return Vec::new();
    }
    route_in(doc, id, &Structures::new(doc, style), style)
}

/// `route` with the structures of the document already computed (`Structures::new`).
pub fn route_in(doc: &Document, id: u32, st: &Structures, style: &Style) -> Vec<Op> {
    let Some(o) = doc.object(id).filter(|o| o.t == ObjKind::Arrow) else {
        return Vec::new();
    };
    let Some((a, b)) = boxes(st, o.from, o.to) else {
        return Vec::new();
    };
    route_between(o, a, b, style.pathway_margin)
}

/// The operations that put the arrow `o` on the routed ends between two boxes (see `route`).
pub fn route_between(o: &crate::doc::Obj, a: Rect, b: Rect, margin: f64) -> Vec<Op> {
    let id = o.id;
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
            runs: None,
            x2: Some(hx),
            y2: Some(hy),
            from: None,
            to: None,
            cv: Default::default(),
            pt: [None, None],
        });
    }
    ops
}

/// The route operations for every anchored arrow whose structures `ops` change (a box that moved, grew, shrank or appeared),
/// computed on the document as `ops` leave it. Empty when `ops` is refused (the step reports that) or touches no anchored structure.
pub fn follow(doc: &Document, ops: &[Op], style: &Style) -> Vec<Op> {
    if !doc.objects.iter().any(|o| o.from != 0 || o.to != 0) {
        return Vec::new();
    }
    let mut after = doc.clone();
    if after.apply(ops).is_err() {
        return Vec::new();
    }
    let (before_st, after_st) = (Structures::new(doc, style), Structures::new(&after, style));
    let box_of = |s: &Structures, atom: u32| (atom != 0).then(|| s.box_of(atom)).flatten();
    let mut out = Vec::new();
    for o in doc.objects.iter().filter(|o| o.t == ObjKind::Arrow) {
        let changed = [o.from, o.to]
            .into_iter()
            .any(|a| box_of(&before_st, a) != box_of(&after_st, a));
        if changed {
            out.extend(route_in(&after, o.id, &after_st, style));
        }
    }
    out
}

/// The smallest atom id of the structure that holds `atom`: the id an anchor made by hand or by `pathway:anchor` carries.
fn smallest(piece: &[u32]) -> u32 {
    piece.iter().copied().min().unwrap_or(0)
}

/// How far (bond lengths) the point (`x`, `y`) is from a box; 0 inside it or on its edge.
pub fn box_distance(b: Rect, x: f64, y: f64) -> f64 {
    let dx = (b[0] - x).max(x - b[2]).max(0.0);
    let dy = (b[1] - y).max(y - b[3]).max(0.0);
    crate::m::hypot(dx, dy)
}

/// How far outside its box (bond lengths) a point still counts as inside a structure for the arrow tool: the box of a chain of
/// carbons is a line, and a press on a bond or an atom must count.
pub const HIT_SLACK: f64 = 0.25;

/// The smallest atom id of the structure whose box (labels included, `HIT_SLACK` more on every side) holds the point (`x`, `y`);
/// with several boxes over the point, the nearest, then the smallest id. `None` on empty canvas.
pub fn structure_at(doc: &Document, x: f64, y: f64, style: &Style) -> Option<u32> {
    nearest_structure(doc, x, y, HIT_SLACK, style).map(|(id, _)| id)
}

/// The structure nearest to the point (`x`, `y`) by box distance, within `reach` bond lengths: its smallest atom id and the
/// distance. Ties go to the smallest atom id. `None` when no structure is within reach.
pub fn nearest_structure(
    doc: &Document,
    x: f64,
    y: f64,
    reach: f64,
    style: &Style,
) -> Option<(u32, f64)> {
    Structures::new(doc, style).nearest(x, y, reach)
}

/// The operations that anchor the ends of the arrow `id` to the nearest structures, each within `reach` bond lengths, then route
/// it (`route`). The anchors come back as `[from, to]` after the operations (0 = free). An end with no structure in reach keeps
/// what it has; when both ends reach the same structure the end farther from it (the head on a tie) is left as it was. Empty when
/// nothing changes. The error is the keyed refusal when `id` is not an arrow or an operation is refused.
pub fn anchor(
    doc: &Document,
    id: u32,
    reach: f64,
    style: &Style,
) -> Result<(Vec<Op>, [u32; 2]), OpError> {
    anchor_in(doc, id, reach, &Structures::new(doc, style), style)
}

/// `anchor` with the structures of the document already computed (`Structures::new`).
pub fn anchor_in(
    doc: &Document,
    id: u32,
    reach: f64,
    st: &Structures,
    style: &Style,
) -> Result<(Vec<Op>, [u32; 2]), OpError> {
    let o = doc
        .object(id)
        .filter(|o| o.t == ObjKind::Arrow)
        .ok_or(OpError {
            key: "error.draw2.object",
            id,
        })?;
    let found = [st.nearest(o.x, o.y, reach), st.nearest(o.x2, o.y2, reach)];
    let mut now = [o.from, o.to];
    match found {
        [Some((a, da)), Some((b, db))] if a == b => {
            if round4(da) < round4(db) {
                now[0] = a;
            } else {
                now[1] = b;
            }
        }
        _ => {
            for (n, f) in now.iter_mut().zip(found) {
                *n = f.map_or(*n, |(s, _)| s);
            }
        }
    }
    let mut ops = Vec::new();
    if now != [o.from, o.to] {
        ops.push(Op::ObjSet {
            id,
            text: None,
            runs: None,
            x2: None,
            y2: None,
            from: Some(now[0]),
            to: Some(now[1]),
            cv: Default::default(),
            pt: [None, None],
        });
    }
    let mut after = doc.clone();
    after.apply(&ops)?;
    // the anchors change no atom: the boxes of the document before are the boxes after
    ops.extend(route_in(&after, id, st, style));
    Ok((ops, now))
}

/// The route operations of every anchored arrow of the document (none when nothing moves).
pub fn route_all(doc: &Document, style: &Style) -> Vec<Op> {
    if !doc
        .objects
        .iter()
        .any(|o| o.t == ObjKind::Arrow && o.from != 0)
    {
        return Vec::new();
    }
    let st = Structures::new(doc, style);
    (doc.objects.iter().filter(|o| o.t == ObjKind::Arrow))
        .flat_map(|o| route_in(doc, o.id, &st, style))
        .collect()
}

/// The Hill formula of the structure that holds `atom`, for a line in words.
pub fn formula_of(doc: &Document, atom: u32) -> String {
    let first = crate::layout::pieces(doc)
        .into_iter()
        .find(|p| p.contains(&atom))
        .and_then(|p| p.first().copied());
    crate::outline::pieces(doc)
        .into_iter()
        .find(|p| Some(p.anchor) == first)
        .map(|p| p.formula)
        .unwrap_or_default()
}
