//! The editor side of lone pairs and curved arrows: the two tools, the middle handle that bends a curve, the keys that bend it and
//! the placing of lone pairs by the valence rules. The operations are `atom.set lp`, `obj.add t:"curve"` and `obj.set bend`.

use crate::curve::{self, Curve};
use crate::doc::ObjKind;
use crate::editor::{Editor, Hit, Press, Tool, FOCUS, REPAINT};
use crate::m;
use crate::ops::Op;

/// A pointer reaches a lone pair within this many pixels (at least; a fraction of the scale for a large view).
const PAIR_REACH: f64 = 10.0;

impl Editor {
    /// An end of a curve under the pointer: `(id, lone pair, point in the document)`. A lone pair of an atom wins over the atom, an
    /// atom over a bond, anything else is a free point.
    pub(crate) fn curve_end_at(&self, px: f64, py: f64, kind: u32) -> (u32, bool, (f64, f64)) {
        let d = self.doc();
        let reach = PAIR_REACH.max(self.view.scale * 0.14)
            * if kind == crate::editor::TOUCH {
                1.6
            } else {
                1.0
            };
        let mut best: Option<(f64, u32, (f64, f64))> = None;
        for a in d.atoms.iter().filter(|a| a.lp() > 0) {
            for dir in curve::lp_dirs(d, a.id, a.lp()) {
                let p = (
                    a.x + curve::LP_REACH * m::cos(dir),
                    a.y + curve::LP_REACH * m::sin(dir),
                );
                let s = self.view.to_px(p.0, p.1);
                let dist = m::hypot(s.0 - px, s.1 - py);
                if dist < reach && best.is_none_or(|b| dist < b.0) {
                    best = Some((dist, a.id, p));
                }
            }
        }
        if let Some((_, id, p)) = best {
            return (id, true, p);
        }
        match self.hit(px, py, kind) {
            Hit::Atom(a) => {
                let at = d.atom(a).map_or((0.0, 0.0), |a| (a.x, a.y));
                (a, false, at)
            }
            Hit::Bond(b) => {
                let at = curve::shape_anchor(d, b);
                (b, false, at)
            }
            _ => (0, false, self.view.to_doc(px, py)),
        }
    }

    /// The release of the curve tool: a curve from where the press was to where the pointer is, bowed to the free side.
    pub(crate) fn curve_release(&mut self, p: &Press, half: bool, px: f64, py: f64) -> u32 {
        if !p.moved {
            self.said = ("draw2.say.curveDrag", String::new());
            return REPAINT;
        }
        let (from, fl, a) = self.curve_end_at(p.start.0, p.start.1, p.kind);
        let (to, tl, b) = self.curve_end_at(px, py, p.kind);
        let bend = curve::free_side_bend(self.doc(), a, b);
        let add = Op::ObjAdd {
            id: None,
            t: ObjKind::Curve,
            x: a.0,
            y: a.1,
            x2: b.0,
            y2: b.1,
            text: String::new(),
            runs: Vec::new(),
            from,
            to,
            cv: Curve { half, bend, fl, tl },
        };
        let what = if half { "1" } else { "2" };
        self.run(&[add], "draw2.say.curve", what)
    }

    /// Enter with a curve tool, on the focused atom `a`: the first one names the source (its lone pair when it has one, the atom with
    /// Shift), the second the target (the atom, or with Shift the bond walked to reach it) and draws the curve; Escape cancels.
    pub(crate) fn curve_key(&mut self, a: u32, shift: bool) -> u32 {
        let Tool::Curve(half) = self.tool else {
            return 0;
        };
        let Some((from, fl)) = self.curve_from else {
            let lone = !shift && self.doc().atom(a).is_some_and(|x| x.lp() > 0);
            self.curve_from = Some((a, lone));
            self.said = (
                if lone {
                    "draw2.say.curveFromPair"
                } else {
                    "draw2.say.curveFrom"
                },
                a.to_string(),
            );
            return REPAINT | FOCUS;
        };
        let to = match (shift, self.walked_bond()) {
            (true, Some(b)) => b,
            (true, None) => {
                self.error = "error.draw2.anchor";
                return REPAINT | FOCUS;
            }
            _ => a,
        };
        let d = self.doc();
        let a0 = curve::anchor_point_of(d, from, fl);
        let b0 = curve::anchor_point_of(d, to, false);
        let bend = curve::free_side_bend(d, a0, b0);
        let add = Op::ObjAdd {
            id: None,
            t: ObjKind::Curve,
            x: 0.0,
            y: 0.0,
            x2: b0.0,
            y2: b0.1,
            text: String::new(),
            runs: Vec::new(),
            from,
            to,
            cv: Curve {
                half,
                bend,
                fl,
                tl: false,
            },
        };
        self.curve_from = None;
        self.run(&[add], "draw2.say.curve", if half { "1" } else { "2" }) | FOCUS
    }

    /// A click of the lone-pair tool: one more pair on the atom, none after four.
    pub(crate) fn lonepair_click(&mut self, hit: Hit) -> u32 {
        let Hit::Atom(a) = hit else {
            return 0;
        };
        let now = self.doc().atom(a).map_or(0, |a| a.lp());
        self.set_lp(a, (now + 1) % (curve::MAX_LP + 1))
    }

    pub(crate) fn set_lp(&mut self, atom: u32, n: u8) -> u32 {
        let set = Op::AtomSet {
            id: atom,
            el: None,
            chg: None,
            iso: None,
            rad: None,
            h: None,
            lab: None,
            map: None,
            lp: Some(n),
        };
        self.run(&[set], "draw2.say.lonepairs", &n.to_string())
    }

    /// Fills the lone pairs of the selected atoms (all of them with no selection) from the valence rules, in one step.
    pub fn place_lone_pairs(&mut self) -> u32 {
        let atoms = self.over.sel_atoms.clone();
        let ops = curve::lone_pair_ops(self.doc(), &atoms);
        if ops.is_empty() {
            self.said = ("draw2.say.lonepairsNone", String::new());
            return REPAINT;
        }
        let n = ops.len().to_string();
        self.run(&ops, "draw2.say.lonepairsPlaced", &n)
    }

    /// The middle handle of the selected curve under a press: the press starts a bend drag and the selection is left alone.
    pub(crate) fn bend_press(&mut self, px: f64, py: f64) -> bool {
        let [id] = self.over.sel_objs[..] else {
            return false;
        };
        let Some(o) = self.doc().object(id).filter(|o| o.t == ObjKind::Curve) else {
            return false;
        };
        let (p, c, q) = curve::shape(self.doc(), o);
        let mid = curve::at(p, c, q, 0.5);
        let s = self.view.to_px(mid.0, mid.1);
        if m::hypot(s.0 - px, s.1 - py) < PAIR_REACH.max(self.view.scale * 0.14) {
            self.bend_drag = Some(id);
            return true;
        }
        false
    }

    /// The bow that puts the middle of the dragged curve under the pointer, drawn as a preview.
    pub(crate) fn bend_move(&mut self, px: f64, py: f64) {
        let Some(id) = self.bend_drag else {
            return;
        };
        let Some(o) = self.doc().object(id) else {
            return;
        };
        let (p, _, q) = curve::shape(self.doc(), o);
        let at = self.view.to_doc(px, py);
        let b = crate::doc::round4(curve::bend_through(p, q, at));
        self.over.bend = Some((id, b));
    }

    /// The end of a bend drag: one `obj.set bend`.
    pub(crate) fn bend_release(&mut self) -> u32 {
        let Some(id) = self.bend_drag.take() else {
            return 0;
        };
        let Some((_, b)) = self.over.bend.take() else {
            return REPAINT;
        };
        self.set_bend(id, b)
    }

    pub(crate) fn set_bend(&mut self, id: u32, bend: f64) -> u32 {
        let set = Op::ObjSet {
            id,
            text: None,
            runs: None,
            x2: None,
            y2: None,
            from: None,
            to: None,
            cv: curve::CurveSet {
                bend: Some(bend),
                ..Default::default()
            },
            pt: [None, None],
        };
        self.run(&[set], "draw2.say.bend", &format!("{bend}"))
    }

    /// `,` and `.` bow the selected curve by a tenth of its chord to either side. `None`: no single curve is selected.
    pub(crate) fn bend_key(&mut self, step: f64) -> Option<u32> {
        let [id] = self.over.sel_objs[..] else {
            return None;
        };
        let o = self.doc().object(id).filter(|o| o.t == ObjKind::Curve)?;
        let b = (o.curve().bend + step).clamp(-curve::MAX_BEND, curve::MAX_BEND);
        Some(self.set_bend(id, crate::doc::round4(b)))
    }
}
