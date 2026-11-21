//! Keeps a shared core oriented the same way across a scheme (`scheme:orient`). For each arrow, the structure after it is turned (and
//! mirrored, only when it has no wedge) so that the core it shares with the structure before it lies the way the source's does.
//!
//! The core is found by the substructure search of `mzlab-mol` (`SubQuery::matches`): the smaller structure in the larger one, and
//! when that fails the ring systems of either in the other, the largest first. The match that needs the least error is kept (among
//! equals, the smallest turn). The fit is a 2D Kabsch fit, written here with `libm`: the angle is `atan2(sum(p x q), sum(p . q))` of
//! the centred matched points. The output is `flip` and `rotate` about the centre of the box of the target, never a `move`: this
//! slice turns structures, it does not place them (placing is the layout's), and the pieces keep their place in the scheme.
//!
//! A plan that would bring two atoms closer than `OVERLAP` bond lengths (that were not that close before) is not applied. No
//! operation is new: `rotate` and `flip` have their exact inverse already, so one undo step takes the whole command back.

use crate::doc::{Document, ObjKind};
use crate::m;
use crate::ops::{Axis, Op};
use mzlab_mol::{SubOptions, SubQuery};
use std::collections::{HashMap, HashSet};

/// Two atoms closer than this (bond lengths) make the result overlap itself.
pub const OVERLAP: f64 = 0.3;
/// A fit leaves a root-mean-square distance (bond lengths, after taking the shift away) at or under this.
pub const FIT: f64 = 0.05;
/// A core needs at least this many atoms to give an orientation.
const MIN_CORE: usize = 3;
/// A turn smaller than this (degrees) is no turn: the target is already oriented.
const SAME: f64 = 0.05;
/// Matches kept per search (symmetric cores have many).
const MATCHES: usize = 400;
/// A structure of more heavy atoms than this is not oriented (`TooLarge`): the cost of the search is not bounded by its size.
pub const MAX_HEAVY: usize = 500;
/// Search steps of one substructure search (the default of `mzlab-mol` is 2,000,000): a search that needs more is a refusal to
/// match, said as `TooLarge`, not a missing core.
const STEPS: u64 = 100_000;
/// Ring systems tried per structure when the whole smaller structure is not in the larger one.
const SYSTEMS: usize = 4;
/// Reactants tried for one product (the nearest first): a scheme has few, and a drawing of thousands of pieces must not make
/// the cost grow with all pairs.
const REACTANTS: usize = 8;
/// Product structures one command orients; the ones after these are reported as `too_large` (the budget is spent).
pub const MAX_PRODUCTS: usize = 200;
/// An arrow, plus sign or text whose box this near the box of the turned structure (bond lengths) counts as overlapped.
const REACH: f64 = 0.3;

/// What came of one pair of structures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Turned (and maybe mirrored).
    Oriented,
    /// Already oriented the same way: nothing to do.
    Already,
    /// No common core of three atoms or more: nothing done.
    NoCore,
    /// The result would overlap an atom, a structure, an arrow, a plus sign or a text: not applied.
    Overlap,
    /// A structure has more than `MAX_HEAVY` heavy atoms, or the search needed more than `STEPS` steps: nothing done.
    TooLarge,
}

impl Status {
    /// The i18n key a refusal with this status gives (empty for the two that are not refusals).
    pub fn key(self) -> &'static str {
        match self {
            Status::Oriented => "",
            Status::Already => "",
            Status::NoCore => "error.draw2.orientNone",
            Status::Overlap => "error.draw2.orientOverlap",
            Status::TooLarge => "error.draw2.orientSize",
        }
    }

    /// The word of the result document.
    pub fn name(self) -> &'static str {
        match self {
            Status::Oriented => "oriented",
            Status::Already => "already",
            Status::NoCore => "no_core",
            Status::Overlap => "overlap",
            Status::TooLarge => "too_large",
        }
    }
}

/// One pair, source before the arrow and target after it, and what was done about it.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub arrow: u32,
    /// An atom of the source and one of the target (the first of their pieces).
    pub from: u32,
    pub to: u32,
    pub status: Status,
    /// Atoms of the core (0 when there is none).
    pub core: usize,
    pub flip: bool,
    pub deg: f64,
    /// Root-mean-square distance of the core atoms (shift taken away) before and after, in bond lengths.
    pub before: f64,
    pub after: f64,
    /// For `Overlap`: the atom (of the target) or the object that would come too close; 0 otherwise.
    pub at: u32,
}

/// All the steps of a document and the operations of the ones that apply.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub ops: Vec<Op>,
    pub steps: Vec<Step>,
}

/// A fit of one match.
#[derive(Clone, Copy)]
struct Fit {
    flip: bool,
    deg: f64,
    rms: f64,
}

fn centre(pts: &[(f64, f64)]) -> (f64, f64) {
    let n = pts.len() as f64;
    let (x, y) = pts.iter().fold((0.0, 0.0), |c, p| (c.0 + p.0, c.1 + p.1));
    (x / n, y / n)
}

/// The turn that takes `p` onto `q` best (both centred first): degrees and the rms left.
fn kabsch(p: &[(f64, f64)], q: &[(f64, f64)]) -> (f64, f64) {
    let (pc, qc) = (centre(p), centre(q));
    let (mut s, mut c) = (0.0, 0.0);
    for (a, b) in p.iter().zip(q) {
        let (ax, ay, bx, by) = (a.0 - pc.0, a.1 - pc.1, b.0 - qc.0, b.1 - qc.1);
        s += ax * by - ay * bx;
        c += ax * bx + ay * by;
    }
    let t = if s == 0.0 && c == 0.0 {
        0.0
    } else {
        m::atan2(s, c)
    };
    let (cs, sn) = (m::cos(t), m::sin(t));
    let mut sum = 0.0;
    for (a, b) in p.iter().zip(q) {
        let (ax, ay) = (a.0 - pc.0, a.1 - pc.1);
        let (rx, ry) = (ax * cs - ay * sn, ax * sn + ay * cs);
        let (dx, dy) = (rx - (b.0 - qc.0), ry - (b.1 - qc.1));
        sum += dx * dx + dy * dy;
    }
    (
        t * 180.0 / std::f64::consts::PI,
        libm::sqrt(sum / p.len() as f64),
    )
}

/// The root-mean-square distance of matched points with the shift taken away, no turn (what is wrong before the command).
fn shape_rms(p: &[(f64, f64)], q: &[(f64, f64)]) -> f64 {
    let (pc, qc) = (centre(p), centre(q));
    let sum: f64 = (p.iter().zip(q))
        .map(|(a, b)| {
            let (dx, dy) = (a.0 - pc.0 - (b.0 - qc.0), a.1 - pc.1 - (b.1 - qc.1));
            dx * dx + dy * dy
        })
        .sum();
    libm::sqrt(sum / p.len() as f64)
}

/// The best fit of the target points `p` (about the box centre `cx`) on the source points `q`; a mirror only when `mirror`.
fn best_fit(p: &[(f64, f64)], q: &[(f64, f64)], cx: f64, mirror: bool) -> Fit {
    let (deg, rms) = kabsch(p, q);
    let mut best = Fit {
        flip: false,
        deg,
        rms,
    };
    if mirror {
        let r: Vec<(f64, f64)> = p.iter().map(|a| (2.0 * cx - a.0, a.1)).collect();
        let (deg, rms) = kabsch(&r, q);
        // a mirror only when it lowers the error clearly
        if rms + 1e-3 < best.rms {
            best = Fit {
                flip: true,
                deg,
                rms,
            };
        }
    }
    best
}

/// Whether a wedge, hash or wave mark is drawn on a bond inside `ids`.
fn wedged(doc: &Document, ids: &HashSet<u32>) -> bool {
    (doc.bonds.iter()).any(|b| b.s != 0 && ids.contains(&b.a) && ids.contains(&b.b))
}

/// The bonds of a graph (pairs of 0-based atom numbers) that lie on a ring: the bonds that are not bridges (lowlink search, linear).
fn ring_bonds(n: usize, edges: &[(usize, usize)]) -> Vec<bool> {
    struct Dfs<'a> {
        adj: &'a [Vec<(usize, usize)>],
        disc: Vec<usize>,
        low: Vec<usize>,
        bridge: Vec<bool>,
        time: usize,
    }
    fn go(d: &mut Dfs, u: usize, via: usize) {
        d.time += 1;
        (d.disc[u], d.low[u]) = (d.time, d.time);
        for k in 0..d.adj[u].len() {
            let (v, e) = d.adj[u][k];
            if e == via {
                continue;
            }
            if d.disc[v] == 0 {
                go(d, v, e);
                d.low[u] = d.low[u].min(d.low[v]);
                if d.low[v] > d.disc[u] {
                    d.bridge[e] = true;
                }
            } else {
                d.low[u] = d.low[u].min(d.disc[v]);
            }
        }
    }
    let mut adj = vec![Vec::new(); n];
    for (e, &(a, b)) in edges.iter().enumerate() {
        adj[a].push((b, e));
        adj[b].push((a, e));
    }
    let mut d = Dfs {
        adj: &adj,
        disc: vec![0; n],
        low: vec![0; n],
        bridge: vec![false; edges.len()],
        time: 0,
    };
    for u in 0..n {
        if d.disc[u] == 0 {
            go(&mut d, u, usize::MAX);
        }
    }
    d.bridge.iter().map(|b| !b).collect()
}

/// The ring systems of a document of heavy atoms, as lists of atom ids, the biggest first: the connected sets of ring bonds.
fn ring_systems(h: &Document) -> Vec<Vec<u32>> {
    let at: HashMap<u32, usize> = (h.atoms.iter().enumerate())
        .map(|(i, a)| (a.id, i))
        .collect();
    let edges: Vec<(usize, usize)> = (h.bonds.iter())
        .filter_map(|b| Some((*at.get(&b.a)?, *at.get(&b.b)?)))
        .collect();
    let ring = ring_bonds(h.atoms.len(), &edges);
    let mut root: Vec<usize> = (0..h.atoms.len()).collect();
    fn find(root: &mut [usize], mut x: usize) -> usize {
        while root[x] != x {
            root[x] = root[root[x]];
            x = root[x];
        }
        x
    }
    let mut on_ring = vec![false; h.atoms.len()];
    for (&(a, b), _) in edges.iter().zip(&ring).filter(|p| *p.1) {
        (on_ring[a], on_ring[b]) = (true, true);
        let (ra, rb) = (find(&mut root, a), find(&mut root, b));
        root[ra] = rb;
    }
    let mut groups: Vec<(usize, Vec<u32>)> = Vec::new();
    for i in (0..h.atoms.len()).filter(|&i| on_ring[i]) {
        let r = find(&mut root, i);
        match groups.iter_mut().find(|g| g.0 == r) {
            Some(g) => g.1.push(h.atoms[i].id),
            None => groups.push((r, vec![h.atoms[i].id])),
        }
    }
    let mut systems: Vec<Vec<u32>> = groups.into_iter().map(|g| g.1).collect();
    m::sort_by(&mut systems, |a, b| a.len() > b.len());
    systems
}

/// The atoms of `keep` (heavy atoms only) with the bonds between them, as a document of their own.
fn heavy(doc: &Document, keep: &HashSet<u32>) -> Document {
    let ok: HashSet<u32> = (doc.atoms.iter())
        .filter(|a| keep.contains(&a.id) && a.el != "H")
        .map(|a| a.id)
        .collect();
    Document {
        atoms: (doc.atoms.iter())
            .filter(|a| ok.contains(&a.id))
            .cloned()
            .collect(),
        bonds: (doc.bonds.iter())
            .filter(|b| ok.contains(&b.a) && ok.contains(&b.b))
            .cloned()
            .collect(),
        next: doc.next,
        ..Document::default()
    }
}

/// Rows of `(small atom id, big atom id)`; `Err` when the search needed more than `STEPS`.
type Rows = Vec<Vec<(u32, u32)>>;

/// The matches of the document of heavy atoms `q` in the one `t`.
fn find(q: &Document, t: &Document) -> Result<Rows, ()> {
    if q.atoms.len() < MIN_CORE || q.atoms.len() > t.atoms.len() {
        return Ok(Vec::new());
    }
    let query = SubQuery::exact(q.to_mol());
    let opts = SubOptions {
        max_matches: MATCHES,
        max_steps: STEPS,
        ..SubOptions::default()
    };
    let rows = query.matches(&t.to_mol(), &opts).map_err(|_| ())?;
    // a query that grew atoms (an abbreviation expanded) has columns the document has no atom for: such a row is not read
    let width = query.mol.atoms.len();
    Ok((rows.into_iter())
        .filter(|r| r.len() == width)
        .map(|r| {
            (q.atoms.iter().zip(&r))
                .filter_map(|(a, &j)| Some((a.id, t.atoms.get(j as usize)?.id)))
                .collect::<Vec<_>>()
        })
        .filter(|r: &Vec<(u32, u32)>| r.len() >= MIN_CORE)
        .collect())
}

fn flipped(rows: Rows) -> Rows {
    (rows.into_iter())
        .map(|r| r.into_iter().map(|(x, y)| (y, x)).collect())
        .collect()
}

/// The matches of the core of `a` and `b` (rows are `(atom of a, atom of b)`): the smaller in the larger, else the ring systems of
/// either in the other, the biggest first. The flag says that a search ran out of steps (a missing core is then `TooLarge`).
fn core(doc: &Document, a: &HashSet<u32>, b: &HashSet<u32>) -> (Rows, bool) {
    let (ha, hb) = (heavy(doc, a), heavy(doc, b));
    let mut limited = false;
    let first = if ha.atoms.len() <= hb.atoms.len() {
        find(&ha, &hb)
    } else {
        find(&hb, &ha).map(flipped)
    };
    match first {
        Ok(rows) if !rows.is_empty() => return (rows, false),
        Ok(_) => {}
        Err(()) => limited = true,
    }
    let mut tries: Vec<(Document, bool)> = Vec::new();
    for (h, from_a) in [(&ha, true), (&hb, false)] {
        for s in ring_systems(h).into_iter().take(SYSTEMS) {
            tries.push((heavy(h, &s.into_iter().collect()), from_a));
        }
    }
    m::sort_by(&mut tries, |x, y| x.0.atoms.len() > y.0.atoms.len());
    for (q, from_a) in tries {
        let found = if from_a {
            find(&q, &hb)
        } else {
            find(&q, &ha).map(flipped)
        };
        match found {
            Ok(rows) if !rows.is_empty() => return (rows, false),
            Ok(_) => {}
            Err(()) => limited = true,
        }
    }
    (Vec::new(), limited)
}

/// The connected pieces of a drawing and the piece of each atom: computed once for a plan (turning does not change who is joined).
struct Pieces {
    list: Vec<Vec<u32>>,
    of: HashMap<u32, usize>,
    /// Heavy atoms of each piece.
    heavy: Vec<usize>,
}

impl Pieces {
    fn of(doc: &Document) -> Pieces {
        let list = crate::layout::pieces(doc);
        let of: HashMap<u32, usize> = (list.iter().enumerate())
            .flat_map(|(i, p)| p.iter().map(move |&a| (a, i)))
            .collect();
        let mut heavy = vec![0; list.len()];
        for a in doc.atoms.iter().filter(|a| a.el != "H") {
            heavy[of[&a.id]] += 1;
        }
        Pieces { list, of, heavy }
    }
}

fn blank(from: u32, to: u32, status: Status) -> Step {
    Step {
        arrow: 0,
        from,
        to,
        status,
        core: 0,
        flip: false,
        deg: 0.0,
        before: 0.0,
        after: 0.0,
        at: 0,
    }
}

type Box4 = (f64, f64, f64, f64);

fn meet(a: Box4, b: Box4) -> bool {
    a.0 <= b.2 && b.0 <= a.2 && a.1 <= b.3 && b.1 <= a.3
}

fn grow(b: Box4, m: f64) -> Box4 {
    (b.0 - m, b.1 - m, b.2 + m, b.3 + m)
}

/// Orients the piece of atom `to` on the piece of atom `from`: the operations (`flip` then `rotate`, about the centre of the box
/// of the target); none when there is nothing to do or nothing safe to do (`solve` says which).
pub fn orient(doc: &Document, from: u32, to: u32) -> Vec<Op> {
    solve(doc, from, to).1
}

/// The step for one pair and its operations (empty unless the status is `Oriented`).
pub fn solve(doc: &Document, from: u32, to: u32) -> (Step, Vec<Op>) {
    let pieces = Pieces::of(doc);
    match (pieces.of.get(&from), pieces.of.get(&to)) {
        (Some(&s), Some(&t)) if s != t => solve_in(doc, &pieces, s, t, from, to),
        _ => (blank(from, to, Status::NoCore), Vec::new()),
    }
}

/// `solve` for the pieces `si` (source) and `ti` (target) of `pieces`.
fn solve_in(
    doc: &Document,
    pieces: &Pieces,
    si: usize,
    ti: usize,
    from: u32,
    to: u32,
) -> (Step, Vec<Op>) {
    let mut step = blank(from, to, Status::NoCore);
    let (src, tgt): (HashSet<u32>, HashSet<u32>) = (
        pieces.list[si].iter().copied().collect(),
        pieces.list[ti].iter().copied().collect(),
    );
    if pieces.heavy[si] > MAX_HEAVY || pieces.heavy[ti] > MAX_HEAVY {
        step.status = Status::TooLarge;
        return (step, Vec::new());
    }
    if pieces.heavy[si] < MIN_CORE || pieces.heavy[ti] < MIN_CORE {
        return (step, Vec::new());
    }
    let (rows, limited) = core(doc, &src, &tgt);
    if rows.is_empty() {
        if limited {
            step.status = Status::TooLarge;
        }
        return (step, Vec::new());
    }
    let pos: HashMap<u32, (f64, f64)> = (doc.atoms.iter())
        .filter(|a| src.contains(&a.id) || tgt.contains(&a.id))
        .map(|a| (a.id, (a.x, a.y)))
        .collect();
    // the box centre of the target: the mirror axis and the pivot
    let tbox = box_of(doc, &tgt);
    let (cx, cy) = (
        crate::doc::round4((tbox.0 + tbox.2) / 2.0),
        crate::doc::round4((tbox.1 + tbox.3) / 2.0),
    );
    let mirror = !wedged(doc, &tgt);
    let mut best: Option<(Fit, usize, f64)> = None;
    for r in &rows {
        let q: Vec<(f64, f64)> = r.iter().map(|&(s, _)| pos[&s]).collect();
        let p: Vec<(f64, f64)> = r.iter().map(|&(_, t)| pos[&t]).collect();
        let fit = best_fit(&p, &q, cx, mirror);
        let better = match &best {
            None => true,
            Some((b, _, _)) => {
                fit.rms + 1e-3 < b.rms
                    || ((fit.rms - b.rms).abs() <= 1e-3
                        && (fit.flip, fit.deg.abs()) < (b.flip, b.deg.abs()))
            }
        };
        if better {
            best = Some((fit, r.len(), shape_rms(&p, &q)));
        }
    }
    let Some((fit, n, before)) = best else {
        return (step, Vec::new());
    };
    step.core = n;
    step.before = before;
    step.flip = fit.flip;
    step.deg = crate::doc::round4(fit.deg);
    if fit.rms > FIT {
        // the core does not fit as a rigid piece (drawn bent, or only a mirror fits a target with a wedge): nothing is done
        step.core = 0;
        step.flip = false;
        step.deg = 0.0;
        return (step, Vec::new());
    }
    if !fit.flip && fit.deg.abs() < SAME {
        step.status = Status::Already;
        step.after = before;
        return (step, Vec::new());
    }
    let mut ids: Vec<u32> = tgt.iter().copied().collect();
    ids.sort_unstable();
    let mut ops = Vec::new();
    if fit.flip {
        ops.push(Op::Flip {
            atoms: ids.clone(),
            axis: Axis::H,
            at: cx,
        });
    }
    ops.push(Op::Rotate {
        atoms: ids,
        deg: step.deg,
        x: cx,
        y: cy,
    });
    // measured on the result, not on the fit: the target alone is turned on a copy
    let mut moved = heavy_all(doc, &tgt);
    if moved.apply(&ops).is_err() {
        return (blank(from, to, Status::NoCore), Vec::new());
    }
    let after: HashMap<u32, (f64, f64)> = moved.atoms.iter().map(|a| (a.id, (a.x, a.y))).collect();
    step.after = rows
        .iter()
        .map(|r| {
            let q: Vec<(f64, f64)> = r.iter().map(|&(s, _)| pos[&s]).collect();
            let p: Vec<(f64, f64)> = r.iter().map(|&(_, t)| after[&t]).collect();
            shape_rms(&p, &q)
        })
        .fold(f64::MAX, f64::min);
    if let Some(at) = overlap(doc, pieces, ti, &tgt, tbox, &after) {
        step.status = Status::Overlap;
        step.at = at;
        return (step, Vec::new());
    }
    step.status = Status::Oriented;
    (step, ops)
}

/// All atoms of `set` (hydrogens too) with the bonds between them, to apply operations on a copy of the piece alone.
fn heavy_all(doc: &Document, set: &HashSet<u32>) -> Document {
    Document {
        atoms: (doc.atoms.iter())
            .filter(|a| set.contains(&a.id))
            .cloned()
            .collect(),
        bonds: (doc.bonds.iter())
            .filter(|b| set.contains(&b.a) && set.contains(&b.b))
            .cloned()
            .collect(),
        next: doc.next,
        ..Document::default()
    }
}

fn box_of(doc: &Document, set: &HashSet<u32>) -> Box4 {
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for a in doc.atoms.iter().filter(|a| set.contains(&a.id)) {
        b = (b.0.min(a.x), b.1.min(a.y), b.2.max(a.x), b.3.max(a.y));
    }
    b
}

/// What the turned target would run into that it did not run into before: an atom of another structure closer than `OVERLAP`, the
/// box of another structure, or the box of an arrow, a plus sign or a text (an arrow anchored to the source or the target is
/// left out: its ends follow the structure). The id of the atom or the object, `None` when nothing.
fn overlap(
    doc: &Document,
    pieces: &Pieces,
    ti: usize,
    tgt: &HashSet<u32>,
    old: Box4,
    after: &HashMap<u32, (f64, f64)>,
) -> Option<u32> {
    let new = (after.values()).fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |b, p| {
        (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1))
    });
    // atoms: any pair now closer than OVERLAP that was not
    for a in doc.atoms.iter().filter(|a| !tgt.contains(&a.id)) {
        let near = |p: (f64, f64)| m::hypot(p.0 - a.x, p.1 - a.y) < OVERLAP;
        if a.x < new.0 - OVERLAP
            || a.x > new.2 + OVERLAP
            || a.y < new.1 - OVERLAP
            || a.y > new.3 + OVERLAP
        {
            continue;
        }
        for t in doc.atoms.iter().filter(|t| tgt.contains(&t.id)) {
            if near(after[&t.id]) && !near((t.x, t.y)) {
                return Some(t.id);
            }
        }
    }
    // the boxes of the other structures
    let mut boxes: HashMap<usize, Box4> = HashMap::new();
    for a in &doc.atoms {
        let i = pieces.of[&a.id];
        if i != ti {
            let b = boxes
                .entry(i)
                .or_insert((f64::MAX, f64::MAX, f64::MIN, f64::MIN));
            *b = (b.0.min(a.x), b.1.min(a.y), b.2.max(a.x), b.3.max(a.y));
        }
    }
    let mut ordered: Vec<(&usize, &Box4)> = boxes.iter().collect();
    ordered.sort_by_key(|(i, _)| **i);
    for (&i, &b) in ordered {
        if meet(new, b) && !meet(old, b) {
            return pieces.list[i].first().copied();
        }
    }
    // the objects
    let mine = |a: u32| tgt.contains(&a);
    for o in &doc.objects {
        // an arrow anchored to this structure or to its neighbour follows it (`route`)
        if o.t == ObjKind::Arrow
            && (o.from != 0 || o.to != 0)
            && (mine(o.from)
                || mine(o.to)
                || pieces.of.contains_key(&o.from)
                || pieces.of.contains_key(&o.to))
        {
            continue;
        }
        let b = if o.t == ObjKind::Arrow {
            (o.x.min(o.x2), o.y.min(o.y2), o.x.max(o.x2), o.y.max(o.y2))
        } else {
            (o.x, o.y, o.x, o.y)
        };
        let b = grow(b, REACH);
        if meet(new, b) && !meet(old, b) {
            return Some(o.id);
        }
    }
    None
}

/// The pairs a scheme asks for. Every structure has its roles by the one rule of `to_rxn` and `scheme:row` (`rxn::along`,
/// `rxn::role`, the middle of the structure along each arrow): it is a reactant of the arrow whose tail is nearest among those it
/// stands before, a product of the arrow whose head is nearest among those it stands past, and an agent (never turned) of an
/// arrow it stands alongside. An arrow with both anchors (`from`, `to`) uses its two structures instead. The result is, for each
/// product in the order of the drawing, `(arrow, product piece, reactant pieces nearest first)`.
fn pairs(doc: &Document, pieces: &Pieces) -> Vec<(u32, usize, Vec<usize>)> {
    let at: HashMap<u32, (f64, f64)> = doc.atoms.iter().map(|a| (a.id, (a.x, a.y))).collect();
    let mid: Vec<(f64, f64)> = pieces
        .list
        .iter()
        .map(|p| {
            let n = p.len() as f64;
            p.iter().fold((0.0, 0.0), |c, i| {
                let a = at[i];
                (c.0 + a.0 / n, c.1 + a.1 / n)
            })
        })
        .collect();
    let arrows: Vec<&crate::doc::Obj> = doc
        .objects
        .iter()
        .filter(|o| o.t == ObjKind::Arrow)
        .collect();
    let anchored = |o: &crate::doc::Obj| matches!((pieces.of.get(&o.from), pieces.of.get(&o.to)), (Some(a), Some(b)) if a != b);
    // for each piece: the nearest arrow it is a reactant of, and the nearest it is a product of (distance to tail, to head)
    let mut react: Vec<Option<(f64, usize)>> = vec![None; pieces.list.len()];
    let mut prod: Vec<Option<(f64, usize)>> = vec![None; pieces.list.len()];
    for (k, o) in arrows.iter().enumerate().filter(|(_, o)| !anchored(o)) {
        if (o.x2 - o.x).abs() + (o.y2 - o.y).abs() < 1e-9 {
            continue;
        }
        for (i, c) in mid.iter().enumerate() {
            let (slot, d) = match crate::rxn::role(crate::rxn::along(o, c.0, c.1)) {
                0 => (&mut react[i], m::hypot(c.0 - o.x, c.1 - o.y)),
                1 => (&mut prod[i], m::hypot(c.0 - o.x2, c.1 - o.y2)),
                _ => continue,
            };
            if slot.is_none_or(|(best, _)| d < best) {
                *slot = Some((d, k));
            }
        }
    }
    // the reactants of each arrow, the nearest to its tail first
    let mut reactants: Vec<Vec<(f64, usize)>> = vec![Vec::new(); arrows.len()];
    for (i, r) in react.iter().enumerate() {
        if let Some((d, k)) = r {
            reactants[*k].push((*d, i));
        }
    }
    for r in &mut reactants {
        r.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    let mut out = Vec::new();
    for (k, o) in arrows.iter().enumerate() {
        if anchored(o) {
            out.push((o.id, pieces.of[&o.to], vec![pieces.of[&o.from]]));
            continue;
        }
        let near: Vec<usize> = reactants[k].iter().take(REACTANTS).map(|r| r.1).collect();
        for (ti, p) in prod.iter().enumerate() {
            if p.is_some_and(|(_, a)| a == k) && !near.is_empty() {
                out.push((o.id, ti, near.clone()));
            }
        }
    }
    out
}

/// The plan for the whole scheme: one step per product structure, the operations of the ones oriented. A product is oriented on
/// its nearest reactant that has a core (a reactant with none is skipped); a reactant that is itself a product of an earlier
/// arrow is oriented first (a chain A -> B -> C turns B on A, then C on the turned B). When any step would overlap, no operation
/// is returned and the step says so (the command does not apply).
pub fn plan(doc: &Document) -> Plan {
    let pieces = Pieces::of(doc);
    let mut todo = pairs(doc, &pieces);
    let mut waiting: HashSet<usize> = todo.iter().map(|p| p.1).collect();
    let mut work = doc.clone();
    let mut plan = Plan::default();
    let mut spent = 0;
    while !todo.is_empty() {
        // the first product none of whose reactants is a product still waiting
        let at = (0..todo.len())
            .find(|&i| {
                !todo[i]
                    .2
                    .iter()
                    .any(|r| waiting.contains(r) && *r != todo[i].1)
            })
            .unwrap_or(0);
        let (arrow, ti, from) = todo.remove(at);
        waiting.remove(&ti);
        spent += 1;
        let first = pieces.list[ti][0];
        if spent > MAX_PRODUCTS {
            let mut s = blank(0, first, Status::TooLarge);
            s.arrow = arrow;
            plan.steps.push(s);
            continue;
        }
        let mut got: Option<(Step, Vec<Op>)> = None;
        let mut limited = false;
        for si in from.into_iter().filter(|&s| s != ti) {
            let (a, b) = (pieces.list[si][0], first);
            let (step, ops) = solve_in(&work, &pieces, si, ti, a, b);
            match step.status {
                Status::NoCore => {}
                Status::TooLarge => limited = true,
                _ => {
                    got = Some((step, ops));
                    break;
                }
            }
        }
        let (mut step, ops) = got.unwrap_or_else(|| {
            let s = if limited {
                Status::TooLarge
            } else {
                Status::NoCore
            };
            (blank(0, first, s), Vec::new())
        });
        step.arrow = arrow;
        if step.status == Status::Oriented {
            let _ = work.apply(&ops);
            plan.ops.extend(ops);
        }
        plan.steps.push(step);
    }
    if plan.steps.iter().any(|s| s.status == Status::Overlap) {
        plan.ops.clear();
    }
    plan
}
