//! `scheme:row`: lays out a one-line reaction scheme. The structures (connected pieces), the arrows, the plus signs and the texts of
//! a drawing, or of the selection, are put on one baseline along the main direction (the way the first arrow points, left to right
//! when there is none): equal gaps of `gap` bond lengths, each structure's box centred on the line, each arrow as long as its notes
//! (the texts and the agent structures above and below it, plus its own text) and a margin ask, each plus sign in a slot of its
//! own between its neighbours. `wrap` breaks the row after an arrow that ends past that width and starts the next line under it.
//!
//! The result is a list of `move` operations (and one `obj.set` of the head of each free arrow): the log carries them, so a native
//! replay gives the page's bytes, and one undo step reverts them. An arrow anchored at both ends (tp-01) gets no end set here:
//! its structures move and `route::route` gives its ends. The roles of the structures in a reaction file (`rxn::to_rxn`: before
//! the tail reactant, past the head product, alongside agent) are kept for a one-arrow row.

use crate::doc::{round4, Document, ObjKind};
use crate::ops::Op;
use crate::route::structure_box;
use crate::scene::width;
use crate::settings::Style;
use serde_json::Value;

/// The width of the slot of a plus sign, bond lengths.
pub const PLUS_WIDTH: f64 = 0.5;
/// The shortest arrow, bond lengths.
pub const MIN_ARROW: f64 = 2.0;
/// Room kept at each end of an arrow beside its widest note, bond lengths.
pub const NOTE_MARGIN: f64 = 0.5;
/// Between the arrow line and the first note, and between two notes, bond lengths.
const NOTE_CLEAR: f64 = 0.3;
const NOTE_GAP: f64 = 0.2;
/// How far from the arrow line (bond lengths) a text still belongs to it as a note (a structure alongside the one arrow of a one-line
/// row is an agent by the rule of `rxn::to_rxn`, at any distance).
pub const NOTE_REACH: f64 = 3.0;

type Rect = [f64; 4];

/// What `scheme:row` is asked: the gap between neighbours (bond lengths), the width that breaks the row (`0`: never), and the
/// selection (both empty: the whole drawing).
#[derive(Clone, Debug, PartialEq)]
pub struct Opts {
    pub gap: f64,
    pub wrap: f64,
    pub atoms: Vec<u32>,
    pub objects: Vec<u32>,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            gap: 1.0,
            wrap: 0.0,
            atoms: Vec::new(),
            objects: Vec::new(),
        }
    }
}

impl Opts {
    /// Reads the `args` of the command (`gap`, `wrap`, `atoms`, `objects`, each optional). The error is an i18n key.
    pub fn parse(args: &Value) -> Result<Opts, &'static str> {
        const BAD: &str = "error.draw2.request";
        let obj = args.as_object().ok_or(BAD)?;
        let mut o = Opts::default();
        for (k, v) in obj {
            match k.as_str() {
                "gap" => {
                    o.gap = v
                        .as_f64()
                        .filter(|g| g.is_finite() && *g > 0.0)
                        .ok_or(BAD)?
                }
                "wrap" => {
                    o.wrap = v
                        .as_f64()
                        .filter(|w| w.is_finite() && *w >= 0.0)
                        .ok_or(BAD)?
                }
                "atoms" | "objects" => {
                    let ids = (v.as_array().ok_or(BAD)?.iter())
                        .map(|i| i.as_u64().and_then(|i| u32::try_from(i).ok()).ok_or(BAD))
                        .collect::<Result<Vec<u32>, _>>()?;
                    if k == "atoms" {
                        o.atoms = ids;
                    } else {
                        o.objects = ids;
                    }
                }
                _ => return Err(BAD),
            }
        }
        Ok(o)
    }
}

/// The height of a line of text, bond lengths.
fn line_height(style: &Style) -> f64 {
    1.2 * style.font_size / style.bond_length
}

/// The box of a text object: from its left end and its middle line.
pub fn text_box(doc: &Document, id: u32, style: &Style) -> Option<Rect> {
    let o = doc.object(id).filter(|o| o.t == ObjKind::Text)?;
    let f = style.font_size / style.bond_length;
    let (w, h) = (width(&o.text, f), line_height(style));
    Some([o.x, o.y - h / 2.0, o.x + w, o.y + h / 2.0])
}

/// The box of a plus sign.
pub fn plus_box(doc: &Document, id: u32) -> Option<Rect> {
    let o = doc.object(id).filter(|o| o.t == ObjKind::Plus)?;
    let h = PLUS_WIDTH / 2.0;
    Some([o.x - h, o.y - h, o.x + h, o.y + h])
}

/// A text or a structure that stands beside an arrow, above or below it.
struct Note {
    atoms: Vec<u32>,
    text: u32,
    /// the current centre and the size of its box
    cx: f64,
    cy: f64,
    w: f64,
    h: f64,
    up: bool,
}

enum Kind {
    Mol(Vec<u32>),
    Plus(u32),
    Text(u32),
    Arrow { id: u32, notes: Vec<Note>, own: f64 },
}

/// One thing of the row: where it stands now, the width it asks along the row and how far it reaches above and below its line.
struct Unit {
    kind: Kind,
    cx: f64,
    cy: f64,
    w: f64,
    up: f64,
    down: f64,
}

fn centre(r: Rect) -> (f64, f64) {
    ((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0)
}

/// Where the notes of an arrow stand relative to its line: the index of each note and the offset of its centre (up is positive),
/// then how far the stacks reach above and below.
fn stack(notes: &[Note], own: f64) -> (Vec<(usize, f64)>, f64, f64) {
    let mut out = Vec::new();
    let mut reach = [0.0, 0.0];
    for (side, up) in [(0, true), (1, false)] {
        let mut idx: Vec<usize> = (0..notes.len()).filter(|&i| notes[i].up == up).collect();
        // the nearest to the line first, ties in order of appearance
        let key = |i: usize| if up { notes[i].cy } else { -notes[i].cy };
        crate::m::sort_by(&mut idx, |&a, &b| (key(a), a) < (key(b), b));
        // the text of the arrow itself takes the first place above
        let taken = if up && own > 0.0 { own + NOTE_GAP } else { 0.0 };
        let mut cursor = NOTE_CLEAR + taken;
        reach[side] = if taken > 0.0 { NOTE_CLEAR + own } else { 0.0 };
        for i in idx {
            let h = notes[i].h;
            let off = cursor + h / 2.0;
            out.push((i, if up { off } else { -off }));
            cursor += h;
            reach[side] = cursor;
            cursor += NOTE_GAP;
        }
    }
    (out, reach[0], reach[1])
}

/// The gathering of the things of the drawing (or of the selection) into units, in no order yet.
fn gather(doc: &Document, style: &Style, o: &Opts) -> Vec<Unit> {
    let all = o.atoms.is_empty() && o.objects.is_empty();
    let f = style.font_size / style.bond_length;
    let arrows: Vec<_> = (doc.objects.iter())
        .filter(|a| a.t == ObjKind::Arrow && (all || o.objects.contains(&a.id)))
        .collect();
    // the distance of a point from the shaft of an arrow, across it; an arrow without length is a point (`rxn::along` puts every
    // structure at its tail, an agent, as `to_rxn` does) and the distance is from it
    let across = |a: &crate::doc::Obj, x: f64, y: f64| {
        let (dx, dy) = (a.x2 - a.x, a.y2 - a.y);
        let l = crate::m::hypot(dx, dy);
        if l < 1e-9 {
            crate::m::hypot(x - a.x, y - a.y)
        } else {
            ((x - a.x) * dy - (y - a.y) * dx).abs() / l
        }
    };
    // the nearest arrow a point is beside: along the shaft where `ok` says and, not further off than that
    let beside = |x: f64, y: f64, ok: &dyn Fn(f64) -> bool, reach: f64| {
        let mut best: Option<(usize, f64)> = None;
        for (k, a) in arrows.iter().enumerate() {
            let (t, d) = (crate::rxn::along(a, x, y), across(a, x, y));
            if d < f64::MAX && ok(t) && d <= reach && best.is_none_or(|b| d < b.1) {
                best = Some((k, d));
            }
        }
        best.map(|b| b.0)
    };
    // a structure alongside the only arrow of a one-line row is an agent at any distance (the rule of `rxn::to_rxn`, the one file
    // the roles are written to); where there are several arrows or a wrap, a structure under or over an arrow is not an agent
    // of it unless it is as near as a text would be
    let structure_reach = if arrows.len() == 1 && o.wrap == 0.0 {
        f64::MAX
    } else {
        NOTE_REACH
    };
    let mut units: Vec<Unit> = Vec::new();
    let mut notes: Vec<Vec<Note>> = arrows.iter().map(|_| Vec::new()).collect();
    for ids in crate::layout::pieces(doc) {
        if !all && !ids.iter().any(|i| o.atoms.contains(i)) {
            continue;
        }
        let Some(r) = structure_box(doc, ids[0], style) else {
            continue;
        };
        let n = ids.len() as f64;
        let (mut mx, mut my) = (0.0, 0.0);
        for a in ids.iter().filter_map(|&i| doc.atom(i)) {
            (mx, my) = (mx + a.x / n, my + a.y / n);
        }
        let (cx, cy) = centre(r);
        let (w, h) = (r[2] - r[0], r[3] - r[1]);
        // the role of `rxn::to_rxn` (`rxn::role`, no cutoff): an agent stands between the tail and the head, however far off
        match beside(mx, my, &|t| crate::rxn::role(t) == 2, structure_reach) {
            Some(k) => notes[k].push(Note {
                atoms: ids,
                text: 0,
                cx,
                cy,
                w,
                h,
                up: cy >= (arrows[k].y + arrows[k].y2) / 2.0,
            }),
            None => units.push(Unit {
                kind: Kind::Mol(ids),
                cx,
                cy,
                w,
                up: h / 2.0,
                down: h / 2.0,
            }),
        }
    }
    for t in doc
        .objects
        .iter()
        .filter(|t| all || o.objects.contains(&t.id))
    {
        match t.t {
            ObjKind::Plus => {
                let r = plus_box(doc, t.id).unwrap_or_default();
                units.push(Unit {
                    kind: Kind::Plus(t.id),
                    cx: t.x,
                    cy: t.y,
                    w: r[2] - r[0],
                    up: PLUS_WIDTH / 2.0,
                    down: PLUS_WIDTH / 2.0,
                });
            }
            ObjKind::Text => {
                let r = text_box(doc, t.id, style).unwrap_or_default();
                let (cx, cy) = centre(r);
                // slightly past each end still counts as beside the arrow
                match beside(cx, cy, &|t| (-0.1..=1.1).contains(&t), NOTE_REACH) {
                    Some(k) => notes[k].push(Note {
                        atoms: Vec::new(),
                        text: t.id,
                        cx,
                        cy,
                        w: r[2] - r[0],
                        h: r[3] - r[1],
                        up: cy >= (arrows[k].y + arrows[k].y2) / 2.0,
                    }),
                    None => units.push(Unit {
                        kind: Kind::Text(t.id),
                        cx,
                        cy,
                        w: r[2] - r[0],
                        up: (r[3] - r[1]) / 2.0,
                        down: (r[3] - r[1]) / 2.0,
                    }),
                }
            }
            ObjKind::Arrow => {}
        }
    }
    for (a, notes) in arrows.iter().zip(notes) {
        let own = if a.text.is_empty() {
            0.0
        } else {
            line_height(style)
        };
        let wide = (notes.iter().map(|n| n.w).fold(0.0, f64::max)).max(width(&a.text, f * 0.85));
        let w = if wide > 0.0 {
            MIN_ARROW.max(wide + 2.0 * NOTE_MARGIN)
        } else {
            MIN_ARROW
        };
        // the arrow has the thickness of a plus sign at least: its line counts as part of the row it stands in
        let (_, up, down) = stack(&notes, own);
        let (up, down) = (up.max(PLUS_WIDTH / 2.0), down.max(PLUS_WIDTH / 2.0));
        units.push(Unit {
            cx: (a.x + a.x2) / 2.0,
            cy: (a.y + a.y2) / 2.0,
            w,
            up,
            down,
            kind: Kind::Arrow {
                id: a.id,
                notes,
                own,
            },
        });
    }
    units
}

/// `round4` with a tie always going up: the heights and widths of the things are sums of four-decimal numbers, so halves of them
/// often end in an exact 5 at the fifth decimal, and float noise would send the same tie either way in two runs of the layout.
fn snap(v: f64) -> f64 {
    ((v * 1e4 + 0.5 + 1e-6).floor()) / 1e4
}

/// A `move` of `dx`, `dy`, each rounded to the four decimals the document keeps; nothing when both are within what a stored
/// coordinate can show (5e-5: half the last decimal, so a thing already as near as it can be is not moved again and a second row
/// gives no operation).
fn shift(ops: &mut Vec<Op>, atoms: Vec<u32>, objects: Vec<u32>, dx: f64, dy: f64) {
    let still = |v: f64| {
        if v.abs() <= 5e-5 + 1e-9 {
            0.0
        } else {
            round4(v)
        }
    };
    let (dx, dy) = (still(dx), still(dy));
    if dx != 0.0 || dy != 0.0 {
        ops.push(Op::Move {
            atoms,
            objects,
            dx,
            dy,
        });
    }
}

/// The operations that put the structures, arrows, plus signs and texts on one row. Empty when there is nothing to lay out or
/// everything stands where the row puts it. The error is an i18n key (a bad `gap` or `wrap`, or a selection that names a thing
/// the drawing does not have).
pub fn row(doc: &Document, style: &Style, o: &Opts) -> Result<Vec<Op>, &'static str> {
    if !(o.gap.is_finite() && o.gap > 0.0 && o.wrap.is_finite() && o.wrap >= 0.0) {
        return Err("error.draw2.request");
    }
    // a selection names things the drawing has
    if o.atoms.iter().any(|&a| doc.atom(a).is_none()) {
        return Err("error.draw2.noAtom");
    }
    if o.objects.iter().any(|&i| doc.object(i).is_none()) {
        return Err("error.draw2.noObject");
    }
    let mut units = gather(doc, style, o);
    if units.is_empty() {
        return Ok(Vec::new());
    }
    // the main direction: the way the first arrow (in the order of the drawing) points
    let right = (doc.objects.iter())
        .find(|a| {
            a.t == ObjKind::Arrow
                && units
                    .iter()
                    .any(|u| matches!(u.kind, Kind::Arrow { id, .. } if id == a.id))
        })
        .is_none_or(|a| a.x2 >= a.x);
    let s = if right { 1.0 } else { -1.0 };
    // the order of reading: the lines of the drawing from the top (units whose vertical reaches overlap are one line, so a row that
    // was wrapped before is read line after line, and a second row finds the same order), each along the direction
    let mut top: Vec<usize> = (0..units.len()).collect();
    crate::m::sort_by(&mut top, |&a, &b| {
        (-(units[a].cy + units[a].up), a) < (-(units[b].cy + units[b].up), b)
    });
    let mut order: Vec<usize> = Vec::new();
    let mut line: Vec<usize> = Vec::new();
    let mut floor = f64::MAX;
    for i in top {
        let (hi, lo) = (units[i].cy + units[i].up, units[i].cy - units[i].down);
        if !line.is_empty() && hi < floor {
            crate::m::sort_by(&mut line, |&a, &b| {
                (s * units[a].cx, a) < (s * units[b].cx, b)
            });
            order.append(&mut line);
            floor = f64::MAX;
        }
        floor = floor.min(lo);
        line.push(i);
    }
    crate::m::sort_by(&mut line, |&a, &b| {
        (s * units[a].cx, a) < (s * units[b].cx, b)
    });
    order.append(&mut line);
    // the baseline: the first arrow's line, or the mean of the lines
    let base = match order
        .iter()
        .find(|&&i| matches!(units[i].kind, Kind::Arrow { .. }))
    {
        Some(&i) => units[i].cy,
        None => units.iter().map(|u| u.cy).sum::<f64>() / units.len() as f64,
    };
    let base = snap(base);
    // the start: the extreme end of the row as it stands
    let start = if right {
        units
            .iter()
            .map(|u| u.cx - u.w / 2.0)
            .fold(f64::MAX, f64::min)
    } else {
        units
            .iter()
            .map(|u| u.cx + u.w / 2.0)
            .fold(f64::MIN, f64::max)
    };
    // lines: a break after an arrow that ends past `wrap`
    let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
    let mut run = 0.0;
    for &i in &order {
        let line = lines.last_mut().expect("one line at least");
        run += if line.is_empty() { 0.0 } else { o.gap } + units[i].w;
        line.push(i);
        if o.wrap > 0.0 && run > o.wrap && matches!(units[i].kind, Kind::Arrow { .. }) {
            lines.push(Vec::new());
            run = 0.0;
        }
    }
    lines.retain(|l| !l.is_empty());
    let mut ops = Vec::new();
    let mut y = base;
    let mut below_prev = 0.0;
    for (k, line) in lines.iter().enumerate() {
        let above = line.iter().map(|&i| units[i].up).fold(0.0, f64::max);
        if k > 0 {
            // lines are further apart than the reach of a note, so that what stands by an arrow of one line is never read as a note
            // of another when the row is laid out again
            y = snap(y - (below_prev + o.gap + above).max(below_prev + NOTE_REACH + 0.1 + above));
        }
        below_prev = line.iter().map(|&i| units[i].down).fold(0.0, f64::max);
        let mut u = 0.0;
        for &i in line {
            let (cx, w) = (start + s * (u + units[i].w / 2.0), units[i].w);
            u += w + o.gap;
            let unit = &mut units[i];
            match &mut unit.kind {
                Kind::Mol(atoms) => {
                    shift(&mut ops, atoms.clone(), vec![], cx - unit.cx, y - unit.cy)
                }
                Kind::Plus(id) => shift(&mut ops, vec![], vec![*id], cx - unit.cx, y - unit.cy),
                Kind::Text(id) => shift(&mut ops, vec![], vec![*id], cx - unit.cx, y - unit.cy),
                Kind::Arrow { id, notes, own } => {
                    let a = doc.object(*id).expect("an arrow of the drawing");
                    let anchored = a.from != 0 && a.to != 0;
                    if !anchored {
                        let (tx, hx) = (cx - s * w / 2.0, cx + s * w / 2.0);
                        shift(&mut ops, vec![], vec![*id], tx - a.x, y - a.y);
                        let (hx, hy) = (snap(hx), round4(y));
                        let (x2, y2) = (
                            round4(a.x2 + round4(tx - a.x)),
                            round4(a.y2 + round4(y - a.y)),
                        );
                        if (x2, y2) != (hx, hy) {
                            ops.push(Op::ObjSet {
                                id: *id,
                                text: None,
                                x2: Some(hx),
                                y2: Some(hy),
                                from: None,
                                to: None,
                            });
                        }
                    }
                    let (place, _, _) = stack(notes, *own);
                    for (n, off) in place {
                        let n = &notes[n];
                        let objs = if n.text != 0 { vec![n.text] } else { vec![] };
                        shift(&mut ops, n.atoms.clone(), objs, cx - n.cx, y + off - n.cy);
                    }
                }
            }
        }
    }
    // an arrow anchored at both ends: its ends are routed (tp-01), on the document the moves leave
    let mut after = doc.clone();
    after.apply(&ops).map_err(|e| e.key)?;
    for u in &units {
        if let Kind::Arrow { id, .. } = u.kind {
            let a = doc.object(id).expect("an arrow of the drawing");
            if a.from != 0 && a.to != 0 {
                let routed = crate::route::route(&after, id, style);
                after.apply(&routed).map_err(|e| e.key)?;
                ops.extend(routed);
            }
        }
    }
    Ok(ops)
}
