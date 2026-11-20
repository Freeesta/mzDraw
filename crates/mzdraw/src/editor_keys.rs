//! Drawing with the keyboard alone: one atom has the focus (a ring on the canvas), the arrow keys walk along the bonds, Enter grows
//! a bond, a letter sets the element, a digit sets the order of the bond just walked. Every answer is a flag set with `FOCUS` when
//! the page should say where the focus is now.

use crate::build;
use crate::editor::{Editor, CHANGED, FOCUS, REPAINT};
use crate::m;
use crate::ops::Op;

impl Editor {
    /// Puts the focus on an atom, remembering where it came from (the bond walked is the one digits act on).
    pub(crate) fn focus_on(&mut self, atom: Option<u32>, from: Option<u32>) -> u32 {
        self.over.focus = atom.filter(|&a| self.doc().atom(a).is_some());
        self.focus_from = from.filter(|_| self.over.focus.is_some());
        REPAINT | FOCUS
    }

    /// The bond between the focused atom and the one the focus came from.
    fn walked_bond(&self) -> Option<u32> {
        let d = self.doc();
        d.bond_between(self.over.focus?, self.focus_from?)
            .map(|b| b.id)
    }

    /// The keys that act on the focus. `None`: not a focus key here, the other bindings apply.
    pub(crate) fn focus_key(&mut self, key: &str, shift: bool) -> Option<u32> {
        let focus = self.over.focus;
        Some(match (key, focus) {
            ("arrowright", _) => self.walk(1.0, 0.0),
            ("arrowleft", _) => self.walk(-1.0, 0.0),
            ("arrowup", _) => self.walk(0.0, 1.0),
            ("arrowdown", _) => self.walk(0.0, -1.0),
            ("home", _) => {
                let first = self.doc().atoms.first().map(|a| a.id);
                self.focus_on(first, None)
            }
            ("pagedown", _) => self.next_piece(1),
            ("pageup", _) => self.next_piece(-1),
            ("enter", _) => self.grow(if shift { 2 } else { 1 }),
            (" ", Some(a)) => {
                // the focused atom joins or leaves the selection
                match self.over.sel_atoms.iter().position(|&x| x == a) {
                    Some(i) => {
                        self.over.sel_atoms.remove(i);
                    }
                    None => self.over.sel_atoms.push(a),
                }
                REPAINT | CHANGED
            }
            ("1" | "2" | "3", Some(_)) => {
                let bond = self.walked_bond()?;
                let o = key.parse().unwrap_or(1);
                let set = Op::BondSet {
                    id: bond,
                    o: Some(o),
                    s: None,
                    rev: false,
                };
                self.run(&[set], "draw2.say.order", key)
            }
            ("+" | "-", Some(a)) => {
                let now = self.doc().atom(a).map_or(0, |x| x.chg);
                let chg = now + if key == "+" { 1 } else { -1 };
                let set = Op::AtomSet {
                    id: a,
                    el: None,
                    chg: Some(chg),
                    iso: None,
                    rad: None,
                    h: None,
                    lab: None,
                };
                self.run(&[set], "draw2.say.charge", &chg.to_string()) | FOCUS
            }
            ("delete" | "backspace", Some(a)) if self.over.sel_atoms.is_empty() => {
                let back = self.focus_from;
                let remove = Op::Remove {
                    atoms: vec![a],
                    bonds: Vec::new(),
                    objects: Vec::new(),
                };
                let f = self.run(&[remove], "draw2.say.erased", "");
                f | self.focus_on(back, None)
            }
            _ => return None,
        })
    }

    /// Moves the focus to the neighbour that lies most in the direction (`dx`, `dy`) (document axes, y up); with no focus yet it
    /// lands on the first atom. A direction with no bond within about 70 degrees leaves the focus where it is.
    fn walk(&mut self, dx: f64, dy: f64) -> u32 {
        let d = self.doc();
        let Some(here) = self.over.focus.and_then(|a| d.atom(a)) else {
            let first = d.atoms.first().map(|a| a.id);
            return self.focus_on(first, None);
        };
        let mut best: (f64, Option<u32>) = (0.34, None);
        for n in d.neighbours(here.id).iter().filter_map(|&n| d.atom(n)) {
            let (vx, vy) = (n.x - here.x, n.y - here.y);
            let cos = (vx * dx + vy * dy) / m::hypot(vx, vy).max(1e-9);
            if cos > best.0 {
                best = (cos, Some(n.id));
            }
        }
        match best.1 {
            Some(to) => {
                let from = here.id;
                self.focus_on(Some(to), Some(from))
            }
            None => FOCUS,
        }
    }

    /// Moves the focus to the first atom of the next (`step` 1) or previous (-1) connected piece.
    fn next_piece(&mut self, step: i32) -> u32 {
        let pieces = crate::layout::pieces(self.doc());
        if pieces.is_empty() {
            return self.focus_on(None, None);
        }
        let n = pieces.len() as i32;
        let at = self
            .over
            .focus
            .and_then(|a| pieces.iter().position(|p| p.contains(&a)));
        let next = match at {
            Some(i) => (i as i32 + step).rem_euclid(n),
            None => 0,
        };
        self.focus_on(pieces[next as usize].first().copied(), None)
    }

    /// Grows a bond of order `o` from the focused atom to a new carbon, which takes the focus; with no focus, a first carbon is
    /// placed in the middle of what is drawn (or at the origin) and takes it.
    fn grow(&mut self, o: u8) -> u32 {
        let id = self.doc().next;
        let Some(a) = self.over.focus.and_then(|a| self.doc().atom(a)).cloned() else {
            let (x, y) = match self.doc().bounds() {
                Some(b) => ((b.0 + b.2) / 2.0, b.1 - 1.5),
                None => (0.0, 0.0),
            };
            let add = Op::AtomAdd {
                id: Some(id),
                el: "C".into(),
                x,
                y,
            };
            let f = self.run(&[add], "draw2.say.atom", "C");
            return if f & CHANGED != 0 {
                f | self.focus_on(Some(id), None)
            } else {
                f
            };
        };
        let ang = build::sprout_angle(self.doc(), a.id);
        let ops = [
            Op::AtomAdd {
                id: Some(id),
                el: "C".into(),
                x: a.x + m::cos(ang),
                y: a.y + m::sin(ang),
            },
            Op::BondAdd {
                id: None,
                a: a.id,
                b: id,
                o,
                s: 0,
            },
        ];
        let f = self.run(&ops, "draw2.say.bond", &o.to_string());
        if f & CHANGED != 0 {
            f | self.focus_on(Some(id), Some(a.id))
        } else {
            f
        }
    }
}
