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

/// What came of one pair of structures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Turned (and maybe mirrored).
    Oriented,
    /// Already oriented the same way: nothing to do.
    Already,
    /// No common core of three atoms or more: nothing done.
    NoCore,
    /// The result would overlap itself: not applied.
    Overlap,
}

impl Status {
    /// The i18n key a refusal of a command with only this status gives.
    pub fn key(self) -> &'static str {
        match self {
            Status::Oriented => "",
            Status::Already => "error.draw2.orientDone",
            Status::NoCore => "error.draw2.orientNone",
            Status::Overlap => "error.draw2.orientOverlap",
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
    /// For `Overlap`: one of the atoms that would come too close.
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

/// Whether atom `id`'s bond list carries a wedge, hash or wave mark anywhere in `ids`.
fn wedged(doc: &Document, ids: &[u32]) -> bool {
    (doc.bonds.iter()).any(|b| b.s != 0 && ids.contains(&b.a) && ids.contains(&b.b))
}

/// The ring systems of a piece, as lists of atom ids: the connected sets of the bonds that lie on a ring (a bond is on a ring when
/// its ends stay connected without it), the biggest first.
fn ring_systems(doc: &Document, ids: &[u32]) -> Vec<Vec<u32>> {
    let bonds: Vec<(u32, u32)> = (doc.bonds.iter())
        .filter(|b| ids.contains(&b.a) && ids.contains(&b.b))
        .map(|b| (b.a, b.b))
        .collect();
    let connected = |skip: usize, from: u32, to: u32| {
        let mut seen = vec![from];
        let mut k = 0;
        while k < seen.len() {
            let at = seen[k];
            k += 1;
            for (i, &(a, b)) in bonds.iter().enumerate() {
                if i == skip {
                    continue;
                }
                let next = if a == at {
                    b
                } else if b == at {
                    a
                } else {
                    continue;
                };
                if !seen.contains(&next) {
                    seen.push(next);
                }
            }
        }
        seen.contains(&to)
    };
    let ring: Vec<(u32, u32)> = (bonds.iter().enumerate())
        .filter(|&(i, &(a, b))| connected(i, a, b))
        .map(|(_, &p)| p)
        .collect();
    let mut systems: Vec<Vec<u32>> = Vec::new();
    for &(a, b) in &ring {
        let (ia, ib) = (
            systems.iter().position(|s| s.contains(&a)),
            systems.iter().position(|s| s.contains(&b)),
        );
        match (ia, ib) {
            (None, None) => systems.push(vec![a, b]),
            (Some(i), None) => systems[i].push(b),
            (None, Some(j)) => systems[j].push(a),
            (Some(i), Some(j)) if i != j => {
                let moved = systems.remove(i.max(j));
                systems[i.min(j)].extend(moved);
            }
            _ => {}
        }
    }
    m::sort_by(&mut systems, |a, b| a.len() > b.len());
    systems
}

/// The heavy atoms of `ids`, as a document of their own with no abbreviation label dropped.
fn heavy(doc: &Document, ids: &[u32]) -> Document {
    let keep: Vec<u32> = (doc.atoms.iter())
        .filter(|a| ids.contains(&a.id) && a.el != "H")
        .map(|a| a.id)
        .collect();
    doc.part(&keep)
}

/// The matches of the atoms `small` of the document in the atoms `big`: rows of `(small atom id, big atom id)`.
fn find(doc: &Document, small: &[u32], big: &[u32]) -> Vec<Vec<(u32, u32)>> {
    let (q, t) = (heavy(doc, small), heavy(doc, big));
    if q.atoms.len() < MIN_CORE || q.atoms.len() > t.atoms.len() {
        return Vec::new();
    }
    let (qm, tm) = (q.to_mol(), t.to_mol());
    let query = SubQuery::exact(qm);
    let opts = SubOptions {
        max_matches: MATCHES,
        ..SubOptions::default()
    };
    // a query that grew atoms (an abbreviation expanded) has columns the document has no atom for: such a row is not read
    let Ok(rows) = query.matches(&tm, &opts) else {
        return Vec::new();
    };
    let width = query.mol.atoms.len();
    (rows.into_iter())
        .filter(|r| r.len() == width)
        .map(|r| {
            (q.atoms.iter().zip(&r))
                .filter_map(|(a, &j)| Some((a.id, t.atoms.get(j as usize)?.id)))
                .collect::<Vec<_>>()
        })
        .filter(|r: &Vec<(u32, u32)>| r.len() >= MIN_CORE)
        .collect()
}

/// The matches of the core of `a` and `b`: the smaller in the larger, else the ring systems of one in the other, the biggest
/// first. Rows are `(atom of a, atom of b)`.
fn core(doc: &Document, a: &[u32], b: &[u32]) -> Vec<Vec<(u32, u32)>> {
    let flipped = |rows: Vec<Vec<(u32, u32)>>| -> Vec<Vec<(u32, u32)>> {
        rows.into_iter()
            .map(|r| r.into_iter().map(|(x, y)| (y, x)).collect())
            .collect()
    };
    let n = |ids: &[u32]| heavy(doc, ids).atoms.len();
    let rows = if n(a) <= n(b) {
        find(doc, a, b)
    } else {
        flipped(find(doc, b, a))
    };
    if !rows.is_empty() {
        return rows;
    }
    // ring systems of either, the biggest that occurs in the other
    let mut tries: Vec<(Vec<u32>, bool)> = Vec::new();
    for (ids, from_a) in [(a, true), (b, false)] {
        for s in ring_systems(doc, ids) {
            tries.push((s, from_a));
        }
    }
    m::sort_by(&mut tries, |x, y| x.0.len() > y.0.len());
    for (s, from_a) in tries {
        let rows = if from_a {
            find(doc, &s, b)
        } else {
            flipped(find(doc, &s, a))
        };
        if !rows.is_empty() {
            return rows;
        }
    }
    Vec::new()
}

/// The pieces of a document that hold the atoms `from` and `to`; `None` when one is missing or both are in one piece.
fn pair_of(doc: &Document, from: u32, to: u32) -> Option<(Vec<u32>, Vec<u32>)> {
    let pieces = crate::layout::pieces(doc);
    let a = pieces.iter().find(|p| p.contains(&from))?;
    let b = pieces.iter().find(|p| p.contains(&to))?;
    (a != b).then(|| (a.clone(), b.clone()))
}

fn xy(doc: &Document, id: u32) -> (f64, f64) {
    doc.atom(id).map_or((0.0, 0.0), |a| (a.x, a.y))
}

/// Orients the piece of atom `to` on the piece of atom `from`: the operations (`flip` then `rotate`, about the centre of the box
/// of the target); none when there is nothing to do or nothing safe to do (`solve` says which).
pub fn orient(doc: &Document, from: u32, to: u32) -> Vec<Op> {
    solve(doc, from, to).1
}

/// The step for one pair and its operations (empty unless the status is `Oriented`).
pub fn solve(doc: &Document, from: u32, to: u32) -> (Step, Vec<Op>) {
    match solve_in(doc, from, to) {
        Ok(ops) => (ops.0, ops.1),
        Err(s) => (s, Vec::new()),
    }
}

fn solve_in(doc: &Document, from: u32, to: u32) -> Result<(Step, Vec<Op>), Step> {
    let mut step = Step {
        arrow: 0,
        from,
        to,
        status: Status::NoCore,
        core: 0,
        flip: false,
        deg: 0.0,
        before: 0.0,
        after: 0.0,
        at: 0,
    };
    let Some((src, mut tgt)) = pair_of(doc, from, to) else {
        return Err(step);
    };
    tgt.sort_unstable();
    let rows = core(doc, &src, &tgt);
    if rows.is_empty() {
        return Err(step);
    }
    // the box centre of the target: the mirror axis and the pivot
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &id in &tgt {
        let (x, y) = xy(doc, id);
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    let (cx, cy) = (
        crate::doc::round4((x0 + x1) / 2.0),
        crate::doc::round4((y0 + y1) / 2.0),
    );
    let mirror = !wedged(doc, &tgt);
    let mut best: Option<(Fit, usize, f64)> = None;
    for r in &rows {
        let q: Vec<(f64, f64)> = r.iter().map(|&(s, _)| xy(doc, s)).collect();
        let p: Vec<(f64, f64)> = r.iter().map(|&(_, t)| xy(doc, t)).collect();
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
        return Err(step);
    };
    step.core = n;
    step.before = before;
    step.flip = fit.flip;
    step.deg = crate::doc::round4(fit.deg);
    if fit.rms > FIT {
        // the core does not fit as a rigid piece (drawn bent): nothing is done
        step.status = Status::NoCore;
        step.core = 0;
        return Err(step);
    }
    if !fit.flip && fit.deg.abs() < SAME {
        step.status = Status::Already;
        step.after = before;
        return Err(step);
    }
    let mut ops = Vec::new();
    if fit.flip {
        ops.push(Op::Flip {
            atoms: tgt.clone(),
            axis: Axis::H,
            at: cx,
        });
    }
    ops.push(Op::Rotate {
        atoms: tgt.clone(),
        deg: step.deg,
        x: cx,
        y: cy,
    });
    // measured on the result, not on the fit
    let mut after = doc.clone();
    if after.apply(&ops).is_err() {
        return Err(step);
    }
    let mut all = rows.iter().map(|r| {
        let q: Vec<(f64, f64)> = r.iter().map(|&(s, _)| xy(&after, s)).collect();
        let p: Vec<(f64, f64)> = r.iter().map(|&(_, t)| xy(&after, t)).collect();
        shape_rms(&p, &q)
    });
    step.after = all.next().unwrap_or(0.0);
    for v in all {
        step.after = step.after.min(v);
    }
    // overlap: a pair of atoms that is now too close and was not before
    let close = |d: &Document, a: u32, b: u32| {
        let (p, q) = (xy(d, a), xy(d, b));
        m::hypot(p.0 - q.0, p.1 - q.1) < OVERLAP
    };
    for &a in &tgt {
        for o in &doc.atoms {
            if o.id != a && close(&after, a, o.id) && !close(doc, a, o.id) {
                step.status = Status::Overlap;
                step.at = a;
                return Err(step);
            }
        }
    }
    step.status = Status::Oriented;
    Ok((step, ops))
}

/// The pairs of structures a scheme asks for: for each arrow, the atom of the structure it leaves and the one it reaches. An arrow
/// with both anchors (`from`, `to`) uses them; a free arrow uses the nearest structure before its tail and the nearest past its
/// head (a structure beside the arrow is an agent, and is left out).
fn pairs(doc: &Document) -> Vec<(u32, u32, u32)> {
    let pieces = crate::layout::pieces(doc);
    let mid: Vec<(f64, f64)> = pieces
        .iter()
        .map(|p| centre(&p.iter().map(|&i| xy(doc, i)).collect::<Vec<_>>()))
        .collect();
    let mut out = Vec::new();
    for o in doc.objects.iter().filter(|o| o.t == ObjKind::Arrow) {
        if doc.atom(o.from).is_some() && doc.atom(o.to).is_some() {
            out.push((o.id, o.from, o.to));
            continue;
        }
        let (dx, dy) = (o.x2 - o.x, o.y2 - o.y);
        let len2 = dx * dx + dy * dy;
        if len2 == 0.0 {
            continue;
        }
        // the nearest piece before the tail and the nearest past the head, as (distance, piece)
        let mut near: [Option<(f64, usize)>; 2] = [None, None];
        for (i, c) in mid.iter().enumerate() {
            let t = ((c.0 - o.x) * dx + (c.1 - o.y) * dy) / len2;
            let (k, d) = if t < 0.0 {
                (0, m::hypot(c.0 - o.x, c.1 - o.y))
            } else if t > 1.0 {
                (1, m::hypot(c.0 - o.x2, c.1 - o.y2))
            } else {
                continue;
            };
            if near[k].is_none_or(|(best, _)| d < best) {
                near[k] = Some((d, i));
            }
        }
        if let [Some((_, s)), Some((_, t))] = near {
            out.push((o.id, pieces[s][0], pieces[t][0]));
        }
    }
    out
}

/// The plan for the whole scheme. A pair runs after the pair that gives its source a place (a chain A -> B -> C orients B on A and
/// then C on the turned B); a structure that is the target of two arrows is oriented by the first only. When any pair would
/// overlap, no operation is returned and the step says so (the command does not apply).
pub fn plan(doc: &Document) -> Plan {
    let mut todo = pairs(doc);
    let mut work = doc.clone();
    let mut plan = Plan::default();
    let mut done_targets: Vec<Vec<u32>> = Vec::new();
    let piece_of = |d: &Document, a: u32| {
        crate::layout::pieces(d)
            .into_iter()
            .find(|p| p.contains(&a))
            .unwrap_or_default()
    };
    while !todo.is_empty() {
        // the first pair whose source is not the target of another pair still waiting
        let at = (0..todo.len())
            .find(|&i| {
                let src = piece_of(&work, todo[i].1);
                !(todo.iter().enumerate())
                    .any(|(j, p)| j != i && piece_of(&work, p.2) == src && p.2 != todo[i].1)
            })
            .unwrap_or(0);
        let (arrow, from, to) = todo.remove(at);
        let target = piece_of(&work, to);
        if done_targets.contains(&target) || piece_of(&work, from) == target {
            continue;
        }
        done_targets.push(target);
        let (mut step, ops) = solve(&work, from, to);
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
