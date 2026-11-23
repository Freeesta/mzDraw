//! An agent's patch shown as a layer over the drawing, accepted or rejected hunk by hunk. `Editor::propose` holds a `draw_edit/1`
//! patch apart from the document: it is not in the drawing and not in the log until a hunk is accepted. The hunks are those of
//! `diff::hunks`; accepting one is one undo step with exactly its operations (no route operations follow: the log carries what the
//! agent wrote), rejecting one drops it, and the others stay valid (plan-15). Any other change of the history (an edit, an undo)
//! makes the proposal stale: it is no longer shown or acted on, because its hunks were written against another drawing.
//! The keys, the tap on a hunk number and the replay request (`propose`, `accept`) all call the functions of this file.

use crate::diff::{self, touched, Hunk};
use crate::doc::{Obj, ObjKind};
use crate::editor::{Editor, CHANGED, HANDLED, MIDDLE, PROPOSAL, REPAINT, SECONDARY};
use crate::editor_touch::{CANCEL, DOWN, UP};
use crate::ops::{Op, OpError};
use crate::scene::{colour, Mark};
use std::collections::BTreeMap;

/// The most operations a proposal may hold: the patch is applied once to a copy to check it, and every hunk is marked on the screen.
pub const MAX_OPS: usize = 2000;
/// The most ids the operations of a proposal may name in all (one `remove` or `move` can carry a list): the cost of holding, marking
/// and saying a proposal grows with it.
pub const MAX_IDS: usize = 50_000;
/// The ids a line of the live text names before it says "and more".
const SAID_IDS: usize = 6;

/// The patch, its hunks and which of them are still open; `at` is the generation of the history it was written against.
#[derive(Clone, Debug)]
pub(crate) struct Proposal {
    hunks: Vec<Hunk>,
    open: Vec<bool>,
    /// The hunk (0-based) that has the keyboard.
    cur: usize,
    /// Whether Enter, Delete and Backspace are the proposal's: set when a hunk was given the keyboard (`]`, `[`, a tap, Show), cleared by
    /// Escape and by a press on the canvas that is not on a number; until then they edit the drawing as ever.
    review: bool,
    at: u64,
}

impl Proposal {
    fn next_open(&self, from: usize, step: isize) -> Option<usize> {
        let n = self.hunks.len() as isize;
        (1..=n)
            .map(|k| (from as isize + step * k).rem_euclid(n) as usize)
            .find(|&i| self.open[i])
    }
}

fn err(key: &'static str, id: u32) -> OpError {
    OpError { key, id }
}

/// What a hunk does, in words: `(i18n key, what)` pairs, the ids it names in `what`.
pub fn say(h: &Hunk) -> Vec<(&'static str, String)> {
    // atoms, bonds, objects: added, removed, changed, moved
    let mut c: [[Vec<u32>; 3]; 4] = Default::default();
    let mut other = 0;
    let mut labels = 0;
    for op in &h.ops {
        match op {
            Op::AtomAdd { id, .. } => c[0][0].extend(id),
            Op::BondAdd { id, .. } => c[0][1].extend(id),
            Op::ObjAdd { id, .. } => c[0][2].extend(id),
            Op::Remove {
                atoms,
                bonds,
                objects,
            } => {
                c[1][0].extend(atoms);
                c[1][1].extend(bonds);
                c[1][2].extend(objects);
            }
            Op::AtomSet { id, .. }
            | Op::AtomData { id, .. }
            | Op::AtomAnn { id, .. }
            | Op::AtomTitle { id, .. }
            | Op::AtomCap { id, .. } => c[2][0].push(*id),
            Op::BondSet { id, .. } => c[2][1].push(*id),
            Op::Colour {
                atoms,
                bonds,
                objects,
                ..
            } => {
                c[2][0].extend(atoms);
                c[2][1].extend(bonds);
                c[2][2].extend(objects);
            }
            Op::ObjSet { id, .. } => c[2][2].push(*id),
            Op::Move { atoms, objects, .. } => {
                c[3][0].extend(atoms);
                c[3][2].extend(objects);
            }
            Op::DocLabels { .. } => labels += 1,
            _ => other += 1,
        }
    }
    const KEYS: [[&str; 3]; 4] = [
        [
            "draw2.hunk.atomsAdded",
            "draw2.hunk.bondsAdded",
            "draw2.hunk.objectsAdded",
        ],
        [
            "draw2.hunk.atomsRemoved",
            "draw2.hunk.bondsRemoved",
            "draw2.hunk.objectsRemoved",
        ],
        [
            "draw2.hunk.atomsChanged",
            "draw2.hunk.bondsChanged",
            "draw2.hunk.objectsChanged",
        ],
        ["draw2.hunk.atomsMoved", "", "draw2.hunk.objectsMoved"],
    ];
    let mut out = Vec::new();
    for (row, keys) in c.iter_mut().zip(KEYS) {
        for (ids, key) in row.iter_mut().zip(keys) {
            if ids.is_empty() {
                continue;
            }
            crate::m::sort(ids);
            ids.dedup();
            let mut what: Vec<String> = ids.iter().take(SAID_IDS).map(u32::to_string).collect();
            if ids.len() > SAID_IDS {
                what.push("…".into());
            }
            out.push((key, what.join(", ")));
        }
    }
    if labels > 0 {
        out.push(("draw2.hunk.labels", String::new()));
    }
    if other > 0 {
        out.push(("draw2.hunk.other", other.to_string()));
    }
    out
}

impl Editor {
    /// The proposal, when the history is still the one it was written against.
    pub(crate) fn live_proposal(&self) -> Option<&Proposal> {
        (self.proposal.as_ref()).filter(|p| p.at == self.session.generation())
    }

    /// Holds `ops` as a proposal in place of any earlier one and returns how many hunks it has. The drawing and the log do not
    /// change. Refused, with nothing held: an empty patch (`error.draw2.proposalEmpty`), a patch of more than `MAX_OPS` operations
    /// (`error.draw2.proposalSize`) and a patch that does not apply to the drawing (the refusal of the operation).
    pub fn propose(&mut self, ops: Vec<Op>) -> Result<usize, OpError> {
        if ops.is_empty() {
            return Err(err("error.draw2.proposalEmpty", 0));
        }
        if ops.len() > MAX_OPS {
            return Err(err("error.draw2.proposalSize", 0));
        }
        let ids: usize = ops.iter().map(|o| touched(o).len()).sum();
        if ids > MAX_IDS {
            return Err(err("error.draw2.proposalIds", 0));
        }
        // the whole patch must apply to the drawing as it is: checked on a copy
        self.doc().clone().apply(&ops)?;
        let hunks = diff::hunks(self.doc(), &ops);
        let n = hunks.len();
        self.proposal = Some(Proposal {
            open: vec![true; n],
            hunks,
            cur: 0,
            review: false,
            at: self.session.generation(),
        });
        self.said = ("draw2.say.proposed", n.to_string());
        self.error = "";
        Ok(n)
    }

    /// The hunk `n` (1-based) of an open proposal.
    fn hunk_index(&self, n: usize) -> Result<usize, OpError> {
        let p = self
            .live_proposal()
            .ok_or(err("error.draw2.noProposal", 0))?;
        match n.checked_sub(1).filter(|&i| i < p.hunks.len() && p.open[i]) {
            Some(i) => Ok(i),
            None => Err(err(
                "error.draw2.proposalHunk",
                n.min(u32::MAX as usize) as u32,
            )),
        }
    }

    /// Closes the hunks in `done`; the proposal ends when none is open, else the keyboard moves to the next open one.
    fn close(&mut self, done: &[usize], at: u64) {
        let Some(p) = self.proposal.as_mut() else {
            return;
        };
        for &i in done {
            p.open[i] = false;
        }
        p.at = at;
        if p.open.iter().any(|&o| o) {
            if !p.open[p.cur] {
                p.cur = p.next_open(p.cur, 1).unwrap_or(p.cur);
            }
        } else {
            self.proposal = None;
        }
    }

    /// Accepts the hunk `n` (1-based) as one undo step with exactly its operations.
    pub fn accept_hunk(&mut self, n: usize) -> Result<(), OpError> {
        let i = self.hunk_index(n)?;
        let ops = self.proposal.as_ref().map(|p| p.hunks[i].ops.clone());
        self.session.run(&ops.unwrap_or_default())?;
        self.close(&[i], self.session.generation());
        self.said = ("draw2.say.accepted", n.to_string());
        self.error = "";
        self.prune();
        Ok(())
    }

    /// Accepts the hunks `ns` (1-based) in that order, one undo step each, all or none: the numbers must be open and different, and
    /// the hunks must apply in that order (checked on a copy first). The error carries the index in `ns` of the first that fails.
    pub fn accept_hunks(&mut self, ns: &[usize]) -> Result<(), (usize, OpError)> {
        let mut copy = self.doc().clone();
        let mut seen = std::collections::BTreeSet::new();
        for (k, &n) in ns.iter().enumerate() {
            let i = self.hunk_index(n).map_err(|e| (k, e))?;
            if !seen.insert(n) {
                return Err((
                    k,
                    err("error.draw2.proposalHunk", n.min(u32::MAX as usize) as u32),
                ));
            }
            let ops = self.proposal.as_ref().map(|p| &p.hunks[i].ops);
            copy.apply(ops.map_or(&[], Vec::as_slice))
                .map_err(|e| (k, e))?;
        }
        for (k, &n) in ns.iter().enumerate() {
            self.accept_hunk(n).map_err(|e| (k, e))?;
        }
        Ok(())
    }

    /// Rejects the hunk `n` (1-based): it is dropped, the drawing and the log do not change.
    pub fn reject_hunk(&mut self, n: usize) -> Result<(), OpError> {
        let i = self.hunk_index(n)?;
        self.close(&[i], self.session.generation());
        self.said = ("draw2.say.rejected", n.to_string());
        self.error = "";
        Ok(())
    }

    /// Accepts every open hunk, in order, as one undo step.
    pub fn accept_all(&mut self) -> Result<(), OpError> {
        let p = self
            .live_proposal()
            .ok_or(err("error.draw2.noProposal", 0))?;
        let open: Vec<usize> = (0..p.hunks.len()).filter(|&i| p.open[i]).collect();
        let ops: Vec<Op> = open.iter().flat_map(|&i| p.hunks[i].ops.clone()).collect();
        self.session.run(&ops)?;
        self.proposal = None;
        self.said = ("draw2.say.acceptedAll", open.len().to_string());
        self.error = "";
        self.prune();
        Ok(())
    }

    /// Drops what is left of the proposal; the drawing and the log do not change.
    pub fn reject_all(&mut self) -> Result<(), OpError> {
        let n = (self.live_proposal())
            .ok_or(err("error.draw2.noProposal", 0))?
            .open
            .iter()
            .filter(|&&o| o)
            .count();
        self.proposal = None;
        self.said = ("draw2.say.rejectedAll", n.to_string());
        self.error = "";
        Ok(())
    }

    /// The hunk (1-based) that has the keyboard, or 0.
    pub fn current_hunk(&self) -> usize {
        self.live_proposal().map_or(0, |p| p.cur + 1)
    }

    /// Gives the keyboard to the hunk `n` (1-based).
    pub fn select_hunk(&mut self, n: usize) -> Result<(), OpError> {
        let i = self.hunk_index(n)?;
        if let Some(p) = self.proposal.as_mut() {
            p.cur = i;
            p.review = true;
        }
        self.said = ("draw2.say.hunk", n.to_string());
        self.error = "";
        Ok(())
    }

    /// Moves the keyboard to the next (`step` 1) or previous (-1) open hunk, round the ring.
    pub fn step_hunk(&mut self, step: isize) -> Result<(), OpError> {
        let cur = self
            .live_proposal()
            .ok_or(err("error.draw2.noProposal", 0))?
            .cur;
        let to = self.proposal.as_ref().and_then(|p| p.next_open(cur, step));
        self.select_hunk(to.map_or(cur, |i| i) + 1)
    }

    /// An action on the proposal by name (what the page and the keys ask): `accept`, `reject` (the hunk with the keyboard),
    /// `accept-all`, `reject-all`, `next`, `prev`, `accept:N`, `reject:N`, `select:N`. Flags as the other commands, plus `PROPOSAL`;
    /// a refusal is kept in `error`.
    pub fn proposal_action(&mut self, what: &str) -> u32 {
        let cur = self.current_hunk();
        let (name, arg) = what.split_once(':').unwrap_or((what, ""));
        let n = || arg.parse::<usize>().unwrap_or(0);
        let done = match name {
            "accept" => self.accept_hunk(if arg.is_empty() { cur } else { n() }),
            "reject" => self.reject_hunk(if arg.is_empty() { cur } else { n() }),
            "accept-all" => self.accept_all(),
            "reject-all" => self.reject_all(),
            "next" => self.step_hunk(1),
            "prev" => self.step_hunk(-1),
            "select" => self.select_hunk(n()),
            _ => return 0,
        };
        match done {
            Ok(()) => {
                let changed = matches!(name, "accept" | "accept-all");
                REPAINT | PROPOSAL | if changed { CHANGED } else { 0 }
            }
            Err(e) => {
                self.error = e.key;
                REPAINT | PROPOSAL
            }
        }
    }

    /// The keys of an open proposal: `]` and `[` the next and previous hunk (nothing else on the keyboard uses them without Alt); once a
    /// hunk has the keyboard (`review`), Enter accepts it and Delete or Backspace rejects it, which edit the drawing otherwise (grow a
    /// bond, delete the selection); with Ctrl, which nothing else uses, they act on all hunks at any time. Escape ends the review
    /// (and goes on as it always does). `None`: not a key of the proposal.
    pub(crate) fn proposal_key(&mut self, key: &str, ctrl: bool) -> Option<u32> {
        let review = self.live_proposal().is_some_and(|p| p.review);
        let what = match (key, ctrl) {
            ("]" | "}", false) => "next",
            ("[" | "{", false) => "prev",
            ("enter", false) if review => "accept",
            ("enter", true) => "accept-all",
            ("delete" | "backspace", false) if review => "reject",
            ("delete" | "backspace", true) => "reject-all",
            ("escape", _) => {
                if let Some(p) = self.proposal.as_mut() {
                    p.review = false;
                }
                return None;
            }
            _ => return None,
        };
        Some(self.proposal_action(what))
    }

    /// An open proposal ended by an edit of the person (or an undo): said at once, so that nothing is left on screen that does not act.
    /// Returns `PROPOSAL | REPAINT` when it ended now.
    pub fn settle_proposal(&mut self) -> u32 {
        // the step field follows the drawing after every call too (its sites are found again, or it closes)
        let step = self.step_follow();
        if self.proposal.is_some() && self.live_proposal().is_none() {
            self.proposal = None;
            self.eat = None;
            self.said = ("draw2.say.proposalStale", String::new());
            // kept beside the counts until the next step: a focus announcement of the same key press must not hide it
            self.notice = self.said.clone();
            return PROPOSAL | REPAINT | step;
        }
        step
    }

    /// A press on the number of a hunk gives it the keyboard; the rest of that press (move, release) is swallowed, and only that
    /// pointer's. A press elsewhere ends the review. Returns the flags to answer with, or `None` when the event is not for the proposal.
    pub(crate) fn proposal_pointer(
        &mut self,
        phase: u32,
        id: u32,
        x: f64,
        y: f64,
        kind: u32,
        mods: u32,
    ) -> Option<u32> {
        if self.live_proposal().is_none() {
            self.eat = None;
            return None;
        }
        if let Some(own) = self.eat {
            if own != id {
                return None;
            }
            if phase == UP || phase == CANCEL {
                self.eat = None;
            }
            return Some(HANDLED);
        }
        if phase != DOWN || mods & (SECONDARY | MIDDLE) != 0 {
            return None;
        }
        let reach = Self::reach(kind).max(self.style.metrics(self.view.scale, true).font);
        let hit = (self.proposal_marks().into_iter())
            .filter_map(|m| match m {
                Mark::Num { p, n, .. } => {
                    let (px, py) = self.view.to_px(p.0, p.1);
                    let d = crate::m::hypot(px - x, py - y);
                    (d <= reach).then_some((d, n))
                }
                _ => None,
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let Some((_, n)) = hit else {
            if let Some(p) = self.proposal.as_mut() {
                p.review = false;
            }
            return None;
        };
        self.eat = Some(id);
        Some(self.proposal_action(&format!("select:{n}")) | HANDLED)
    }

    /// The marks of the open hunks over the drawing, in document units: added things in the accent colour, removed ones struck in
    /// the warning colour, changed ones outlined, a moved atom drawn where it will stand with a line from where it is; each hunk has
    /// its number. Built from the operations and one table of positions, not looked up per mark in the drawing.
    pub fn proposal_marks(&self) -> Vec<Mark> {
        let Some(p) = self.live_proposal() else {
            return Vec::new();
        };
        let d = self.doc();
        let mut at: BTreeMap<u32, (f64, f64)> =
            d.atoms.iter().map(|a| (a.id, (a.x, a.y))).collect();
        for (h, _) in p.hunks.iter().zip(&p.open).filter(|x| *x.1) {
            for op in &h.ops {
                if let Op::AtomAdd {
                    id: Some(i), x, y, ..
                } = op
                {
                    at.insert(*i, (*x, *y));
                }
            }
        }
        let objs: BTreeMap<u32, &Obj> = d.objects.iter().map(|o| (o.id, o)).collect();
        let bonds: BTreeMap<u32, (u32, u32)> = d.bonds.iter().map(|b| (b.id, (b.a, b.b))).collect();
        let mut marks = Vec::new();
        for (i, h) in p.hunks.iter().enumerate().filter(|(i, _)| p.open[*i]) {
            let mut pts: Vec<(f64, f64)> = Vec::new();
            let ring = |marks: &mut Vec<Mark>, pts: &mut Vec<(f64, f64)>, q: (f64, f64), c| {
                marks.push(Mark::Ring { p: q, c, r: 0.75 });
                pts.push(q);
            };
            for op in &h.ops {
                match op {
                    Op::AtomAdd { x, y, .. } => {
                        marks.push(Mark::Dot {
                            p: (*x, *y),
                            c: colour::ACCENT,
                        });
                        pts.push((*x, *y));
                    }
                    Op::BondAdd { a, b, .. } => {
                        if let (Some(&a), Some(&b)) = (at.get(a), at.get(b)) {
                            marks.push(Mark::Line {
                                a,
                                b,
                                c: colour::ACCENT,
                                heavy: false,
                            });
                            pts.push(((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0));
                        }
                    }
                    Op::ObjAdd {
                        t, x, y, x2, y2, ..
                    } => {
                        if *t == ObjKind::Arrow {
                            marks.push(Mark::Line {
                                a: (*x, *y),
                                b: (*x2, *y2),
                                c: colour::ACCENT,
                                heavy: false,
                            });
                        } else {
                            marks.push(Mark::Dot {
                                p: (*x, *y),
                                c: colour::ACCENT,
                            });
                        }
                        pts.push((*x, *y));
                    }
                    Op::Remove {
                        atoms,
                        bonds: bs,
                        objects,
                    } => {
                        for q in atoms.iter().filter_map(|a| at.get(a)) {
                            // a cross over the atom
                            for (u, v) in [((-1.0, -1.0), (1.0, 1.0)), ((-1.0, 1.0), (1.0, -1.0))] {
                                marks.push(Mark::Line {
                                    a: (q.0 + 0.12 * u.0, q.1 + 0.12 * u.1),
                                    b: (q.0 + 0.12 * v.0, q.1 + 0.12 * v.1),
                                    c: colour::WARN,
                                    heavy: true,
                                });
                            }
                            pts.push(*q);
                        }
                        for (a, b) in bs.iter().filter_map(|b| bonds.get(b)) {
                            if let (Some(&a), Some(&b)) = (at.get(a), at.get(b)) {
                                marks.push(Mark::Line {
                                    a,
                                    b,
                                    c: colour::WARN,
                                    heavy: true,
                                });
                                pts.push(((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0));
                            }
                        }
                        for o in objects.iter().filter_map(|o| objs.get(o)) {
                            ring(&mut marks, &mut pts, (o.x, o.y), colour::WARN);
                        }
                    }
                    Op::AtomSet { id, .. }
                    | Op::AtomData { id, .. }
                    | Op::AtomAnn { id, .. }
                    | Op::AtomTitle { id, .. }
                    | Op::AtomCap { id, .. } => {
                        if let Some(&q) = at.get(id) {
                            ring(&mut marks, &mut pts, q, colour::ACCENT);
                        }
                    }
                    Op::BondSet { id, .. } => {
                        if let Some((a, b)) = bonds
                            .get(id)
                            .and_then(|(a, b)| Some((at.get(a)?, at.get(b)?)))
                        {
                            let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                            ring(&mut marks, &mut pts, mid, colour::ACCENT);
                        }
                    }
                    Op::Colour {
                        atoms,
                        bonds: bs,
                        objects: os,
                        ..
                    } => {
                        for q in atoms.iter().filter_map(|i| at.get(i)) {
                            ring(&mut marks, &mut pts, *q, colour::ACCENT);
                        }
                        for o in os.iter().filter_map(|o| objs.get(o)) {
                            ring(&mut marks, &mut pts, (o.x, o.y), colour::ACCENT);
                        }
                        for (a, b) in bs
                            .iter()
                            .filter_map(|i| bonds.get(i))
                            .filter_map(|(a, b)| Some((at.get(a)?, at.get(b)?)))
                        {
                            let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                            ring(&mut marks, &mut pts, mid, colour::ACCENT);
                        }
                    }
                    Op::ObjSet { id, .. } => {
                        if let Some(o) = objs.get(id) {
                            ring(&mut marks, &mut pts, (o.x, o.y), colour::ACCENT);
                        }
                    }
                    Op::Move {
                        atoms,
                        objects,
                        dx,
                        dy,
                    } => {
                        for q in atoms.iter().filter_map(|a| at.get(a)) {
                            let to = (q.0 + dx, q.1 + dy);
                            marks.push(Mark::Line {
                                a: *q,
                                b: to,
                                c: colour::PREVIEW,
                                heavy: false,
                            });
                            marks.push(Mark::Dot {
                                p: to,
                                c: colour::ACCENT,
                            });
                            pts.push(to);
                        }
                        for o in objects.iter().filter_map(|o| objs.get(o)) {
                            ring(&mut marks, &mut pts, (o.x + dx, o.y + dy), colour::ACCENT);
                        }
                    }
                    // rotate, flip, ring: a ring on each atom they name
                    other => {
                        for q in touched(other).iter().filter_map(|a| at.get(a)) {
                            ring(&mut marks, &mut pts, *q, colour::ACCENT);
                        }
                    }
                }
            }
            // the number sits above the middle of what the hunk touches; a hunk that touches nothing the marks can place (an operation
            // that names no id) takes the middle of the drawing
            if pts.is_empty() {
                pts.push(
                    d.bounds()
                        .map_or((0.0, 0.0), |b| ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0)),
                );
            }
            let n = pts.len() as f64;
            let (cx, cy) = pts.iter().fold((0.0, 0.0), |s, q| (s.0 + q.0, s.1 + q.1));
            marks.push(Mark::Num {
                p: (cx / n, cy / n + 0.7),
                n: (i + 1) as u32,
                current: i == p.cur,
            });
        }
        marks
    }

    /// The proposal as the page shows it, JSON: `{"cur": N, "hunks": [{"n", "say": [[key, what]]}]}` for the open hunks, or `null`.
    pub fn proposal_json(&self) -> String {
        let Some(p) = self.live_proposal() else {
            return "null".into();
        };
        let hunks: Vec<serde_json::Value> = (p.hunks.iter().enumerate())
            .filter(|(i, _)| p.open[*i])
            .map(|(i, h)| serde_json::json!({ "n": i + 1, "say": say(h) }))
            .collect();
        // `speak`: the live line is about the hunk with the keyboard (a proposal was just made, or a hunk was given the keyboard)
        let speak = matches!(self.said.0, "draw2.say.hunk" | "draw2.say.proposed");
        serde_json::json!({ "cur": p.cur + 1, "hunks": hunks, "speak": speak }).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_of_nothing_are_nothing() {
        assert!(Editor::default().proposal_marks().is_empty());
        assert_eq!(Editor::default().proposal_json(), "null");
    }
}
