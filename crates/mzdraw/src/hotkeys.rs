//! Hover keys: with the pointer over an atom or a bond (or the keyboard focus on an atom, or on the bond just walked), a key acts on
//! that object without selecting it first. The one table `TABLE` says which key does what; `Editor::key` reads it before the tool
//! keys. Every command is made of operations that exist (`bond.set`, `atom.set`, `chain`, `ring`), so a session with hover keys
//! replays natively to the same bytes and each key is one undo step. With the pointer over nothing and no focus the keys pick
//! their tools as ever, and so they do with a drawing tool active: the keys that already picked a tool act on the object only with
//! the select tool or the plain single-bond tool, which is the tool the page starts with (the Alt keys and the rings work with any
//! tool).

use crate::build;
use crate::editor::{Editor, Tool, FOCUS, REPAINT};
use crate::ops::Op;
use std::f64::consts::PI;

/// What a hover key acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The atom under the pointer, else the atom with the keyboard focus.
    Atom,
    /// The bond under the pointer, else the bond just walked with the keyboard.
    Bond,
}

/// What a hover key does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// The bond takes this order (`bond.set o`).
    Order(u8),
    /// The bond takes this wedge mark (1 solid, 2 hashed, 3 wavy: `bond.set s`, with order 1); the same key again turns it around
    /// (`bond.set rev`).
    Wedge(u8),
    /// The atom takes this abbreviation (`atom.set lab`, a name of `label::ABBREVIATIONS`).
    Group(&'static str),
    /// A zigzag chain of this many bonds sprouts from the atom (`chain`, at the sprout angle).
    Chain(u8),
    /// A ring of this many atoms that shares the atom as one corner, a spiro or pendant ring (`ring`).
    Ring(u8),
}

/// What a hover key would do to its object.
enum Check {
    Go,
    Nothing,
    Refuse(&'static str),
}

/// One row of the table: the key (lower case, as `Editor::key` reads it), whether Shift or Alt is held, the command.
#[derive(Clone, Copy, Debug)]
pub struct Hot {
    pub target: Target,
    pub key: &'static str,
    pub shift: bool,
    pub alt: bool,
    /// Whether the focused atom counts when the pointer is over no atom. Not for a plain letter that picks a tool with the focus
    /// (`E` is the erase tool with an atom in focus: a tested meaning): those have an Alt twin that does.
    pub focus: bool,
    pub cmd: Cmd,
}

const fn row(target: Target, key: &'static str, shift: bool, alt: bool, cmd: Cmd) -> Hot {
    Hot {
        target,
        key,
        shift,
        alt,
        focus: !(matches!(cmd, Cmd::Group(_)) && !alt),
        cmd,
    }
}

use Target::{Atom, Bond};

/// The table of hover keys. Keys that stand for an element (C N O S P F H B I, Shift+C, Shift+B, Shift+S) are not here: they keep
/// setting the element. Over nothing, 1 2 3 W Shift+W Y M E Shift+F Shift+O keep picking their tools.
pub const TABLE: &[Hot] = &[
    row(Bond, "1", false, false, Cmd::Order(1)),
    row(Bond, "2", false, false, Cmd::Order(2)),
    row(Bond, "3", false, false, Cmd::Order(3)),
    row(Bond, "w", false, false, Cmd::Wedge(1)),
    row(Bond, "w", true, false, Cmd::Wedge(2)),
    row(Bond, "y", false, false, Cmd::Wedge(3)),
    row(Atom, "m", false, false, Cmd::Group("Me")),
    row(Atom, "e", false, false, Cmd::Group("Et")),
    row(Atom, "f", true, false, Cmd::Group("CF3")),
    row(Atom, "o", true, false, Cmd::Group("OH")),
    row(Atom, "m", false, true, Cmd::Group("Me")),
    row(Atom, "e", false, true, Cmd::Group("Et")),
    row(Atom, "f", true, true, Cmd::Group("CF3")),
    row(Atom, "o", true, true, Cmd::Group("OH")),
    row(Atom, "1", false, true, Cmd::Chain(1)),
    row(Atom, "2", false, true, Cmd::Chain(2)),
    row(Atom, "3", false, true, Cmd::Chain(3)),
    row(Atom, "4", false, true, Cmd::Chain(4)),
    row(Atom, "5", false, true, Cmd::Chain(5)),
    row(Atom, "6", false, true, Cmd::Chain(6)),
    row(Atom, "7", false, true, Cmd::Chain(7)),
    row(Atom, "8", false, true, Cmd::Chain(8)),
    row(Atom, "9", false, true, Cmd::Chain(9)),
    row(Atom, "3", true, false, Cmd::Ring(3)),
    row(Atom, "4", true, false, Cmd::Ring(4)),
    row(Atom, "5", true, false, Cmd::Ring(5)),
    row(Atom, "6", true, false, Cmd::Ring(6)),
    row(Atom, "7", true, false, Cmd::Ring(7)),
    row(Atom, "8", true, false, Cmd::Ring(8)),
];

/// The row for a key, if any (`key` lower case).
pub fn find(target: Target, key: &str, shift: bool, alt: bool) -> Option<&'static Hot> {
    TABLE
        .iter()
        .find(|h| h.target == target && h.key == key && h.shift == shift && h.alt == alt)
}

/// The table for the help of the page: a JSON list of `{"target","key","shift","alt","focus","cmd","n"|"label"}` (keys of the page texts
/// are `draw2.hot.<cmd>`; Rust returns the keys and parameters, the page words them).
pub fn help_json() -> String {
    let rows: Vec<String> = TABLE
        .iter()
        .map(|h| {
            let t = if h.target == Atom { "atom" } else { "bond" };
            let (cmd, param) = match h.cmd {
                Cmd::Order(n) => ("order", format!("\"n\":{n}")),
                Cmd::Wedge(n) => ("wedge", format!("\"n\":{n}")),
                Cmd::Group(l) => ("group", format!("\"label\":\"{l}\"")),
                Cmd::Chain(n) => ("chain", format!("\"n\":{n}")),
                Cmd::Ring(n) => ("ring", format!("\"n\":{n}")),
            };
            format!(
                "{{\"target\":\"{t}\",\"key\":\"{}\",\"shift\":{},\"alt\":{},\"focus\":{},\"cmd\":\"{cmd}\",{param}}}",
                h.key, h.shift, h.alt, h.focus
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

impl Editor {
    /// The atom a hover key acts on: under the pointer first; else, when the pointer is over nothing and the row takes it, the one with
    /// the focus (a pointer over a bond is over something: the focus is not used).
    fn hot_atom(&self, focus: bool) -> Option<u32> {
        let pointer = self.over.hover_atom;
        let rest = self
            .over
            .focus
            .filter(|_| focus && self.over.hover_bond.is_none());
        pointer.or(rest).filter(|&a| self.doc().atom(a).is_some())
    }

    /// The bond a hover key acts on: under the pointer first; else, when the pointer is over no atom, the bond just walked.
    fn hot_bond(&self) -> Option<u32> {
        let pointer = self.over.hover_bond;
        let walked = self
            .walked_bond()
            .filter(|_| self.over.hover_atom.is_none());
        pointer.or(walked).filter(|&b| self.doc().bond(b).is_some())
    }

    /// A hover key (`key` lower case): `None` when it is not one here (no row, or nothing under the pointer or in focus), so that the
    /// other bindings apply. One undo step, announced; a key that would change nothing takes no step and says so, and one that would
    /// lose something silently is refused with a keyed note and changes nothing.
    pub(crate) fn hot_key(&mut self, key: &str, shift: bool, alt: bool) -> Option<u32> {
        // a key that already picks a tool (digits, W, Y, M, E, Shift+F, Shift+O) acts on the object only with the select tool or the plain
        // single-bond tool (the tool the page starts with), so that the flow "pick a tool with the key, then click" of the drawing
        // tools is what it was; the Alt keys and the rings are new keys
        let contested = !alt && !(shift && key.len() == 1 && key.as_bytes()[0].is_ascii_digit());
        if contested && !matches!(self.tool, Tool::Select | Tool::Bond(1, 0)) {
            return None;
        }
        let bond = self.hot_bond();
        let (h, id) = match (find(Atom, key, shift, alt), find(Bond, key, shift, alt)) {
            (Some(h), _) if self.hot_atom(h.focus).is_some() => (h, self.hot_atom(h.focus)?),
            (_, Some(h)) if bond.is_some() => (h, bond?),
            _ => return None,
        };
        let focused = if self.over.hover_atom.is_none() && h.target == Atom {
            FOCUS
        } else {
            0
        };
        Some(match self.hot_check(h.cmd, id) {
            Check::Go => self.hot_run(h.cmd, id) | focused,
            Check::Nothing => {
                self.said = ("draw2.say.hotNoChange", String::new());
                self.error = "";
                REPAINT
            }
            Check::Refuse(key) => {
                self.error = key;
                REPAINT
            }
        })
    }

    /// Whether the key changes the object, would lose something, or changes nothing.
    fn hot_check(&self, cmd: Cmd, id: u32) -> Check {
        let d = self.doc();
        match cmd {
            Cmd::Order(o) => {
                let Some(b) = d.bond(id) else {
                    return Check::Nothing;
                };
                let kind = b.kind();
                if b.o == o {
                    Check::Nothing
                } else if o != 1 && crate::bonddisp::single_only(&kind) {
                    // a dative or hydrogen bond is a single bond: another order would take the kind away
                    Check::Refuse("error.draw2.hotKind")
                } else {
                    Check::Go
                }
            }
            Cmd::Wedge(_) => match d.bond(id) {
                // a mark replaces the display kind (dashed, bold, dative ...): not silently
                Some(b) if !b.kind().is_empty() => Check::Refuse("error.draw2.hotKind"),
                _ => Check::Go,
            },
            Cmd::Group(label) => {
                let Some(a) = d.atom(id) else {
                    return Check::Nothing;
                };
                // the label stands for the group the atom is the attachment of: one bond at most, nothing of the atom's own to lose
                if d.neighbours(id).len() > 1 {
                    Check::Refuse("error.draw2.hotGroup")
                } else if a.chg != 0
                    || a.iso != 0
                    || a.rad != 0
                    || (a.el != "C" && a.lab.is_empty())
                {
                    Check::Refuse("error.draw2.hotAtom")
                } else if a.lab == label {
                    Check::Nothing
                } else {
                    Check::Go
                }
            }
            Cmd::Chain(_) | Cmd::Ring(_) => {
                let new = if matches!(cmd, Cmd::Ring(_)) { 2 } else { 1 };
                match d.atom(id) {
                    Some(a) if self.free_valence(a) < new => {
                        Check::Refuse("error.draw2.hotValence")
                    }
                    _ => Check::Go,
                }
            }
        }
    }

    /// The valence an atom has left for new bonds: the highest valence its element and charge allow, less what its bonds and radical
    /// electrons use; none on a labelled atom. An element with no known valence has no limit.
    fn free_valence(&self, a: &crate::doc::Atom) -> u32 {
        let used = self.doc().order_sum(a.id) + u32::from(a.rad);
        // a labelled atom is a group of its own: nothing is added to it
        if !a.lab.is_empty() {
            return 0;
        }
        match mzlab_mol::allowed_valences(&a.el, i32::from(a.chg))
            .into_iter()
            .max()
        {
            Some(v) => v.saturating_sub(used),
            None => u32::MAX,
        }
    }

    fn hot_run(&mut self, cmd: Cmd, id: u32) -> u32 {
        match cmd {
            Cmd::Order(o) => {
                let set = Op::BondSet {
                    id,
                    o: Some(o),
                    s: None,
                    pos: None,
                    k: None,
                    rev: false,
                };
                self.run(&[set], "draw2.say.order", &o.to_string())
            }
            Cmd::Wedge(s) => {
                let Some(b) = self.doc().bond(id) else {
                    return 0;
                };
                // the same wedge again turns it around; another order becomes single with the wedge
                let again = b.o == 1 && b.s == s;
                let set = Op::BondSet {
                    id,
                    o: (b.o != 1).then_some(1),
                    s: (!again).then_some(s),
                    pos: None,
                    k: None,
                    rev: again,
                };
                let say = if again {
                    "draw2.say.wedgeTurn"
                } else {
                    "draw2.say.wedge"
                };
                self.run(&[set], say, "")
            }
            Cmd::Group(label) => {
                let set = Op::AtomSet {
                    map: None,
                    id,
                    el: None,
                    chg: None,
                    iso: None,
                    rad: None,
                    h: None,
                    lab: Some(label.into()),
                    lp: None,
                };
                self.run(&[set], "draw2.say.label", label)
            }
            Cmd::Chain(n) => {
                let Some(a) = self.doc().atom(id) else {
                    return 0;
                };
                let deg = build::sprout_angle(self.doc(), id) * 180.0 / PI;
                let op = Op::Chain {
                    n,
                    atom: Some(id),
                    x: a.x,
                    y: a.y,
                    deg,
                };
                self.run(&[op], "draw2.say.chain", &n.to_string())
            }
            Cmd::Ring(n) => {
                let op = Op::Ring {
                    n,
                    arom: false,
                    atom: Some(id),
                    bond: None,
                    x: 0.0,
                    y: 0.0,
                };
                self.run(&[op], "draw2.say.ring", &n.to_string())
            }
        }
    }

    /// Runs one key as if the pointer were over the atom (`(true, id)`) or the bond (`(false, id)`) `hover`, and the keyboard focus
    /// were on `focus` (an atom and the atom the focus came from): the replay field `keys`. The pointer is taken away again
    /// afterwards; the focus stays, as after a key of the page.
    pub fn key_over(
        &mut self,
        key: &str,
        mods: u32,
        hover: Option<(bool, u32)>,
        focus: Option<(u32, Option<u32>)>,
    ) -> u32 {
        let was = (self.over.hover_atom, self.over.hover_bond);
        match hover {
            Some((true, id)) => (self.over.hover_atom, self.over.hover_bond) = (Some(id), None),
            Some((false, id)) => (self.over.hover_atom, self.over.hover_bond) = (None, Some(id)),
            None => {}
        }
        if let Some((a, from)) = focus {
            self.focus_on(Some(a), from);
        }
        let f = self.key(key, mods);
        (self.over.hover_atom, self.over.hover_bond) = was;
        f
    }
}
