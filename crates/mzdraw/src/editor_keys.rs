//! Drawing with the keyboard alone: one atom has the focus (a ring on the canvas), the arrow keys walk along the bonds, Enter grows
//! a bond, a letter sets the element, a digit sets the order of the bond just walked. Every answer is a flag set with `FOCUS` when
//! the page should say where the focus is now.

use crate::build;
use crate::editor::{Editor, CHANGED, FOCUS, REPAINT};
use crate::m;
use crate::ops::Op;
use std::f64::consts::PI;

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
            ("arrowright" | "arrowleft" | "arrowup" | "arrowdown", _) => {
                let (dx, dy) = match key {
                    "arrowright" => (1.0, 0.0),
                    "arrowleft" => (-1.0, 0.0),
                    "arrowup" => (0.0, 1.0),
                    _ => (0.0, -1.0),
                };
                let f = self.walk(dx, dy);
                if shift {
                    self.extend_selection(focus) | f
                } else {
                    f
                }
            }
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
                    pos: None,
                    k: None,
                    rev: false,
                };
                self.run(&[set], "draw2.say.order", key)
            }
            ("+" | "-", Some(a)) => {
                let now = self.doc().atom(a).map_or(0, |x| x.chg);
                let chg = now + if key == "+" { 1 } else { -1 };
                let set = Op::AtomSet {
                    map: None,
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

    /// The chain the keyboard would draw now: from the focused atom at its sprout angle, or from the middle below what is drawn,
    /// with the typed count (2 when none is typed): `(atom, x, y, deg, n)`.
    fn chain_by_keys(&self) -> (Option<u32>, f64, f64, f64, u8) {
        let n = self
            .chain_keys
            .parse()
            .unwrap_or(2)
            .clamp(1, build::CHAIN_MAX);
        if let Some(a) = self.over.focus.and_then(|a| self.doc().atom(a)) {
            return (
                Some(a.id),
                a.x,
                a.y,
                build::sprout_angle(self.doc(), a.id) * 180.0 / PI,
                n,
            );
        }
        let (x, y) = match self.doc().bounds() {
            Some(b) => ((b.0 + b.2) / 2.0, b.1 - 1.5),
            None => (0.0, 0.0),
        };
        (None, x, y, 30.0, n)
    }

    /// The preview of the chain the keys would draw: shown while a count is typed.
    fn chain_key_preview(&mut self) {
        self.over.chain = (!self.chain_keys.is_empty()).then(|| {
            let (_, x, y, deg, n) = self.chain_by_keys();
            build::chain_points(x, y, deg, n as usize)
        });
    }

    /// The keys of the chain tool: digits type the count (1-50), Backspace takes the last digit, Enter draws the chain from the
    /// focused atom (else from the middle below the drawing), Escape forgets the count; the arrow keys move the focus as always.
    /// `None`: not a key of the chain tool.
    pub(crate) fn chain_key(&mut self, key: &str, shift: bool) -> Option<u32> {
        match key {
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                let mut typed = format!("{}{key}", self.chain_keys);
                if typed.len() > 2 || typed.parse::<u8>().is_ok_and(|n| n > build::CHAIN_MAX) {
                    typed = build::CHAIN_MAX.to_string();
                }
                if typed.parse::<u8>().is_ok_and(|n| n == 0) {
                    return Some(0);
                }
                self.chain_keys = typed;
                self.chain_key_preview();
                self.said = ("draw2.say.chainCount", self.chain_keys.clone());
                Some(REPAINT)
            }
            "backspace" if !self.chain_keys.is_empty() => {
                self.chain_keys.pop();
                self.chain_key_preview();
                Some(REPAINT)
            }
            "escape" => {
                let typed = !self.chain_keys.is_empty();
                self.chain_keys.clear();
                self.over.chain = None;
                // with a count typed, Escape forgets it and keeps the tool; else it leaves the tool as every tool
                typed.then_some(REPAINT)
            }
            "enter" => {
                let (atom, x, y, deg, n) = self.chain_by_keys();
                let last = {
                    let (px, py) = match atom.and_then(|a| self.doc().atom(a)) {
                        Some(a) => (a.x, a.y),
                        None => (x, y),
                    };
                    *build::chain_points(px, py, deg, n as usize)
                        .last()
                        .expect("points")
                };
                self.chain_keys.clear();
                self.over.chain = None;
                let f = self.run(
                    &[Op::Chain { n, atom, x, y, deg }],
                    "draw2.say.chain",
                    &n.to_string(),
                );
                if f & CHANGED == 0 {
                    return Some(f);
                }
                let end = self
                    .doc()
                    .atoms
                    .iter()
                    .find(|a| m::hypot(a.x - last.0, a.y - last.1) < 0.25)
                    .map(|a| a.id);
                let from = atom.filter(|_| n == 1);
                Some(f | self.focus_on(end, from))
            }
            "arrowright" | "arrowleft" | "arrowup" | "arrowdown" | "home" | "pagedown"
            | "pageup" => {
                let f = self.focus_key(key, shift)?;
                self.chain_key_preview();
                Some(f)
            }
            _ => None,
        }
    }

    /// After Shift+Arrow: the atom the focus walked to (and the one it left, when nothing was selected yet) joins the selection,
    /// with the bond between them. Does nothing when the focus did not move.
    fn extend_selection(&mut self, from: Option<u32>) -> u32 {
        let (Some(from), Some(to)) = (from, self.over.focus) else {
            return 0;
        };
        if from == to {
            return 0;
        }
        let bond = self.doc().bond_between(from, to).map(|b| b.id);
        let o = &mut self.over;
        for a in [from, to] {
            if !o.sel_atoms.contains(&a) {
                o.sel_atoms.push(a);
            }
        }
        if let Some(b) = bond.filter(|b| !o.sel_bonds.contains(b)) {
            o.sel_bonds.push(b);
        }
        REPAINT | CHANGED
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
                map: None,
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
                map: None,
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
