//! Moving and turning without dragging (WCAG 2.2 SC 2.5.7): the keys that nudge or turn the selection, and the "move here" mode in
//! which a click on an empty spot brings the selection there. Both only emit the operations `move` and `rotate`.

use crate::editor::{Editor, CHANGED, REPAINT};
use crate::ops::Op;

/// A nudge of the selection by `Alt+Arrow`, in bond lengths.
pub const NUDGE_SMALL: f64 = 0.1;
/// A nudge with `Alt+Shift+Arrow`, in bond lengths.
pub const NUDGE_LARGE: f64 = 1.0;
/// A turn by `Alt+[` or `Alt+]`, in degrees.
pub const TURN_STEP: f64 = 15.0;
/// Repeated keys within this many milliseconds of each other are one undo step.
const PAUSE_MS: f64 = 1000.0;

/// What the last key step was, to join the next one to it.
pub(crate) struct Nudge {
    /// Time stamp of the last key press.
    at: f64,
    /// The history as that step left it: any other change breaks the chain.
    generation: u64,
    /// The selection it acted on.
    atoms: Vec<u32>,
    objects: Vec<u32>,
    /// A move (`false`) or a turn (`true`).
    turn: bool,
    /// What the chain did so far: the displacement in bond lengths, or the angle in degrees in `.0`.
    total: (f64, f64),
}

impl Editor {
    /// Moves the selection by (`dx`, `dy`) bond lengths (document axes, y up) as one `move` op; the step joins the one before when
    /// it was made by keys on the same selection less than a second ago (`t`, the time stamp of the key). A refusal or nothing
    /// selected changes nothing.
    pub fn nudge(&mut self, dx: f64, dy: f64, t: f64) -> u32 {
        let (atoms, objects) = (self.over.sel_atoms.clone(), self.over.sel_objs.clone());
        if atoms.is_empty() && objects.is_empty() {
            return 0;
        }
        let op = Op::Move {
            atoms: atoms.clone(),
            objects: objects.clone(),
            dx,
            dy,
        };
        let total = self.chain(&atoms, &objects, false, t, (dx, dy));
        let what = describe_move(total);
        self.step(
            op,
            atoms,
            objects,
            false,
            t,
            total,
            "draw2.announce.moved",
            &what,
        )
    }

    /// Turns the selected atoms by `deg` degrees counter-clockwise about the centre of their positions (the mean, which a turn
    /// about itself leaves where it is, so that 24 turns of 15 degrees come back to the start) as one `rotate` op, joined to the
    /// step before as `nudge` is. Nothing is turned when no atom is selected.
    pub fn turn(&mut self, deg: f64, t: f64) -> u32 {
        let atoms = self.over.sel_atoms.clone();
        let here: Vec<_> = atoms.iter().filter_map(|&i| self.doc().atom(i)).collect();
        if here.is_empty() {
            // objects alone cannot be turned: say so rather than do nothing in silence
            if self.over.sel_objs.is_empty() {
                return 0;
            }
            self.said = ("draw2.announce.noturn", String::new());
            return REPAINT | CHANGED;
        }
        let n = here.len() as f64;
        let x = here.iter().map(|a| a.x).sum::<f64>() / n;
        let y = here.iter().map(|a| a.y).sum::<f64>() / n;
        let op = Op::Rotate {
            atoms: atoms.clone(),
            deg,
            x,
            y,
        };
        let total = self.chain(&atoms, &[], true, t, (deg, 0.0));
        let what = describe_turn(total.0);
        self.step(
            op,
            atoms,
            Vec::new(),
            true,
            t,
            total,
            "draw2.announce.rotated",
            &what,
        )
    }

    /// What the chain has done in all if this key press joins the one before, else just this press.
    fn chain(
        &self,
        atoms: &[u32],
        objects: &[u32],
        turn: bool,
        t: f64,
        now: (f64, f64),
    ) -> (f64, f64) {
        match self.joins(atoms, objects, turn, t) {
            Some(n) => (n.total.0 + now.0, n.total.1 + now.1),
            None => now,
        }
    }

    /// The last key step, if this press (the same kind of step on the same selection, soon enough, nothing else in between) joins it.
    fn joins(&self, atoms: &[u32], objects: &[u32], turn: bool, t: f64) -> Option<&Nudge> {
        let n = self.nudge.as_ref()?;
        let fresh = n.generation == self.session.generation();
        let soon = t >= n.at && t - n.at < PAUSE_MS;
        (fresh && soon && n.turn == turn && n.atoms == atoms && n.objects == objects).then_some(n)
    }

    #[allow(clippy::too_many_arguments)]
    fn step(
        &mut self,
        op: Op,
        atoms: Vec<u32>,
        objects: Vec<u32>,
        turn: bool,
        t: f64,
        total: (f64, f64),
        say: &'static str,
        what: &str,
    ) -> u32 {
        let join = self.joins(&atoms, &objects, turn, t).is_some();
        let ops = self.following(&[op]);
        let done = if join {
            self.session.run_into_last(&ops)
        } else {
            self.session.run(&ops)
        };
        match done {
            Ok(()) => {
                self.said = (say, what.to_string());
                self.error = "";
                self.nudge = Some(Nudge {
                    at: t,
                    generation: self.session.generation(),
                    atoms,
                    objects,
                    turn,
                    total,
                });
                REPAINT | CHANGED
            }
            Err(e) => {
                self.error = e.key;
                REPAINT
            }
        }
    }

    /// The centre of the selected atoms and objects (the middle of their bounding box).
    fn selection_centre(&self) -> Option<(f64, f64)> {
        let d = self.doc();
        let mut points: Vec<(f64, f64)> = (self.over.sel_atoms.iter())
            .filter_map(|&i| d.atom(i))
            .map(|a| (a.x, a.y))
            .collect();
        for o in self.over.sel_objs.iter().filter_map(|&i| d.object(i)) {
            points.push((o.x, o.y));
            if o.t == crate::doc::ObjKind::Arrow {
                points.push((o.x2, o.y2));
            }
        }
        let first = *points.first()?;
        let b = points
            .iter()
            .fold((first.0, first.1, first.0, first.1), |b, p| {
                (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1))
            });
        Some(((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0))
    }

    /// Moves the selection so that its centre lands on the document point (`x`, `y`): one `move` op. Nothing selected: nothing.
    pub fn move_centre_to(&mut self, x: f64, y: f64) -> u32 {
        let Some((cx, cy)) = self.selection_centre() else {
            return 0;
        };
        let (atoms, objects) = (self.over.sel_atoms.clone(), self.over.sel_objs.clone());
        let n = (atoms.len() + objects.len()).to_string();
        let op = Op::Move {
            atoms,
            objects,
            dx: x - cx,
            dy: y - cy,
        };
        self.run(&[op], "draw2.say.moved", &n)
    }

    /// Switches the "move here" mode: with the select tool and a selection, a click on an empty spot brings the selection there.
    /// Switching it on brings the select tool, which is the tool it works with.
    pub fn set_move_here(&mut self, on: bool) -> u32 {
        self.move_here = on;
        if on && self.tool != crate::editor::Tool::Select {
            return self.set_tool("select");
        }
        REPAINT | crate::editor::TOOL
    }

    /// Whether a press on an empty spot is a "move here" click (the selection must not be cleared by it).
    pub(crate) fn move_here_ready(&self) -> bool {
        self.move_here
            && self.tool == crate::editor::Tool::Select
            && !(self.over.sel_atoms.is_empty() && self.over.sel_objs.is_empty())
    }
}

fn amount(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    crate::svg::num(r as f32)
}

/// "right 0.3 up 0.1" in bond lengths: the parts that are not zero.
fn describe_move(total: (f64, f64)) -> String {
    let mut parts = Vec::new();
    for (v, plus, minus) in [(total.0, "right", "left"), (total.1, "up", "down")] {
        if (v * 100.0).round() != 0.0 {
            parts.push(format!(
                "{} {}",
                if v > 0.0 { plus } else { minus },
                amount(v.abs())
            ));
        }
    }
    if parts.is_empty() {
        "0".into()
    } else {
        parts.join(" ")
    }
}

/// "30 counter-clockwise" or "45 clockwise" in degrees.
fn describe_turn(deg: f64) -> String {
    if (deg * 100.0).round() == 0.0 {
        return "0".into();
    }
    format!(
        "{} {}",
        amount(deg.abs()),
        if deg > 0.0 {
            "counter-clockwise"
        } else {
            "clockwise"
        }
    )
}
