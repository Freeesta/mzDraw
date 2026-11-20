//! Edit operations: the command log language (the draft of the `mol draw --set edit=` patches). Every operation applied to a document
//! records an exact inverse, so undo restores the document byte for byte; a `Session` is a document with its undo and redo stacks.

use crate::build;
use crate::doc::{round4, Atom, Bond, Document, Obj, ObjKind};
use crate::m;
use serde::{Deserialize, Serialize};

fn one() -> u8 {
    1
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

fn is_zero_f(v: &f64) -> bool {
    *v == 0.0
}

/// The direction of a flip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    H,
    V,
}

/// One operation. Ids left out are taken from the document counter; `ring` is shorthand that expands to atoms and bonds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum Op {
    #[serde(rename = "atom.add")]
    AtomAdd {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<u32>,
        el: String,
        x: f64,
        y: f64,
    },
    #[serde(rename = "bond.add")]
    BondAdd {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<u32>,
        a: u32,
        b: u32,
        #[serde(default = "one")]
        o: u8,
        /// Wedge mark of a single bond (1 solid, 2 hashed, 3 wavy), narrow end at `a`.
        #[serde(default, skip_serializing_if = "is_zero")]
        s: u8,
    },
    #[serde(rename = "atom.set")]
    AtomSet {
        id: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        el: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        chg: Option<i8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        iso: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rad: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        h: Option<u8>,
        /// A label (an abbreviation or an R group); the empty text takes the label away.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lab: Option<String>,
    },
    /// Sets the order and/or the wedge mark of a bond; a new order other than 1 drops the mark. `rev` swaps the two ends (the narrow
    /// end of a wedge moves to the other atom).
    #[serde(rename = "bond.set")]
    BondSet {
        id: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        o: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        s: Option<u8>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        rev: bool,
    },
    /// Turns the atoms by `deg` degrees counter-clockwise around (`x`, `y`).
    #[serde(rename = "rotate")]
    Rotate {
        atoms: Vec<u32>,
        deg: f64,
        x: f64,
        y: f64,
    },
    /// Mirrors the atoms: axis `h` left to right across the vertical line x = `at`, axis `v` top to bottom across y = `at`. The wedge
    /// marks stay as drawn, so the mirror image is the other enantiomer.
    #[serde(rename = "flip")]
    Flip {
        atoms: Vec<u32>,
        axis: Axis,
        at: f64,
    },
    #[serde(rename = "move")]
    Move {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        atoms: Vec<u32>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        objects: Vec<u32>,
        dx: f64,
        dy: f64,
    },
    #[serde(rename = "remove")]
    Remove {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        atoms: Vec<u32>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        bonds: Vec<u32>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        objects: Vec<u32>,
    },
    /// A reaction arrow (`t` "arrow", from `x`,`y` to `x2`,`y2`, optional `text` above), a plus sign (`t` "plus") or a text
    /// (`t` "text").
    #[serde(rename = "obj.add")]
    ObjAdd {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<u32>,
        t: ObjKind,
        x: f64,
        y: f64,
        #[serde(default, skip_serializing_if = "is_zero_f")]
        x2: f64,
        #[serde(default, skip_serializing_if = "is_zero_f")]
        y2: f64,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        text: String,
    },
    /// Sets the text and/or the end point of an object.
    #[serde(rename = "obj.set")]
    ObjSet {
        id: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x2: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y2: Option<f64>,
    },
    /// A ring of `n` atoms (3-8), `arom` = alternating double bonds: fused on the bond `bond`, or with the atom `atom` as a corner, or
    /// centred on (`x`, `y`).
    #[serde(rename = "ring")]
    Ring {
        n: u8,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        arom: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        atom: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bond: Option<u32>,
        #[serde(default)]
        x: f64,
        #[serde(default)]
        y: f64,
    },
}

/// Why an operation was refused: an i18n key and the id it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpError {
    pub key: &'static str,
    pub id: u32,
}

fn err<T>(key: &'static str, id: u32) -> Result<T, OpError> {
    Err(OpError { key, id })
}

/// A wedge mark (1-3) sits on a single bond only.
fn wedge_fits(o: u8, s: u8) -> bool {
    s == 0 || (s <= 3 && o == 1)
}

/// What undoes one change; applied in reverse order of recording.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Inv {
    DropAtom(u32),
    DropBond(u32),
    PutAtom(usize, Atom),
    PutBond(usize, Bond),
    SetAtom(Atom),
    SetBond(Bond),
    DropObj(u32),
    PutObj(usize, Obj),
    SetObj(Obj),
    Next(u32),
}

impl Document {
    fn take_id(&mut self, id: Option<u32>, inv: &mut Vec<Inv>) -> Result<u32, OpError> {
        let id = id.unwrap_or(self.next);
        if id == 0
            || self.atom(id).is_some()
            || self.bond(id).is_some()
            || self.object(id).is_some()
        {
            return err("error.draw2.idTaken", id);
        }
        inv.push(Inv::Next(self.next));
        self.next = self.next.max(id + 1);
        Ok(id)
    }

    fn undo_one(&mut self, inv: Inv) {
        match inv {
            Inv::DropAtom(id) => self.atoms.retain(|a| a.id != id),
            Inv::DropBond(id) => self.bonds.retain(|b| b.id != id),
            Inv::PutAtom(i, a) => self.atoms.insert(i, a),
            Inv::PutBond(i, b) => self.bonds.insert(i, b),
            Inv::SetAtom(a) => {
                if let Some(i) = self.atom_index(a.id) {
                    self.atoms[i] = a;
                }
            }
            Inv::SetBond(b) => {
                if let Some(i) = self.bond_index(b.id) {
                    self.bonds[i] = b;
                }
            }
            Inv::DropObj(id) => self.objects.retain(|o| o.id != id),
            Inv::PutObj(i, o) => self.objects.insert(i, o),
            Inv::SetObj(o) => {
                if let Some(i) = self.objects.iter().position(|x| x.id == o.id) {
                    self.objects[i] = o;
                }
            }
            Inv::Next(n) => self.next = n,
        }
    }

    pub(crate) fn revert(&mut self, inv: Vec<Inv>) {
        for i in inv.into_iter().rev() {
            self.undo_one(i);
        }
    }

    fn add_atom(
        &mut self,
        id: Option<u32>,
        el: &str,
        x: f64,
        y: f64,
        done: &mut Vec<Op>,
        inv: &mut Vec<Inv>,
    ) -> Result<(), OpError> {
        if !mzlab_mol::is_element(el) {
            return err("error.draw2.element", id.unwrap_or(0));
        }
        let id = self.take_id(id, inv)?;
        let (x, y, el) = (round4(x), round4(y), el.to_string());
        self.atoms.push(Atom {
            id,
            el: el.clone(),
            x,
            y,
            chg: 0,
            iso: 0,
            rad: 0,
            h: None,
            lab: String::new(),
        });
        inv.push(Inv::DropAtom(id));
        done.push(Op::AtomAdd {
            id: Some(id),
            el,
            x,
            y,
        });
        Ok(())
    }

    fn add_bond(
        &mut self,
        id: Option<u32>,
        a: u32,
        b: u32,
        (o, s): (u8, u8),
        done: &mut Vec<Op>,
        inv: &mut Vec<Inv>,
    ) -> Result<(), OpError> {
        if self.atom(a).is_none() || a == b {
            return err("error.draw2.noAtom", a);
        }
        if self.atom(b).is_none() {
            return err("error.draw2.noAtom", b);
        }
        if !(1..=3).contains(&o) || !wedge_fits(o, s) || self.bond_between(a, b).is_some() {
            return err("error.draw2.bond", a);
        }
        let id = self.take_id(id, inv)?;
        self.bonds.push(Bond { id, a, b, o, s });
        inv.push(Inv::DropBond(id));
        done.push(Op::BondAdd {
            id: Some(id),
            a,
            b,
            o,
            s,
        });
        Ok(())
    }

    /// Sets the given properties of an atom; a new element forgets the explicit hydrogen count and the label.
    fn set_atom(&mut self, op: &Op, inv: &mut Vec<Inv>) -> Result<(), OpError> {
        let Op::AtomSet {
            id,
            el,
            chg,
            iso,
            rad,
            h,
            lab,
        } = op
        else {
            return Ok(());
        };
        let Some(i) = self.atom_index(*id) else {
            return err("error.draw2.noAtom", *id);
        };
        if el.as_ref().is_some_and(|e| !mzlab_mol::is_element(e)) {
            return err("error.draw2.element", *id);
        }
        if lab
            .as_ref()
            .is_some_and(|l| !l.is_empty() && crate::label::kind(l).is_none())
        {
            return err("error.draw2.label", *id);
        }
        inv.push(Inv::SetAtom(self.atoms[i].clone()));
        let a = &mut self.atoms[i];
        if let Some(e) = el {
            a.el = e.clone();
            a.h = None;
            a.lab.clear();
        }
        if let Some(l) = lab {
            a.lab = l.clone();
        }
        a.chg = chg.unwrap_or(a.chg).clamp(-4, 4);
        a.iso = iso.unwrap_or(a.iso);
        a.rad = rad.unwrap_or(a.rad).min(3);
        a.h = h.or(a.h);
        Ok(())
    }

    /// Applies one primitive operation, pushing its inverse and its resolved form (ids filled in, numbers rounded).
    fn apply_one(
        &mut self,
        op: &Op,
        done: &mut Vec<Op>,
        inv: &mut Vec<Inv>,
    ) -> Result<(), OpError> {
        match op {
            Op::AtomAdd { id, el, x, y } => self.add_atom(*id, el, *x, *y, done, inv)?,
            Op::BondAdd { id, a, b, o, s } => self.add_bond(*id, *a, *b, (*o, *s), done, inv)?,
            Op::AtomSet { .. } => {
                self.set_atom(op, inv)?;
                done.push(op.clone());
            }
            Op::BondSet { id, o, s, rev } => {
                let Some(i) = self.bond_index(*id) else {
                    return err("error.draw2.noBond", *id);
                };
                let now = &self.bonds[i];
                let order = o.unwrap_or(now.o);
                // a new order other than single drops the wedge mark
                let mark = s.unwrap_or(if order == 1 { now.s } else { 0 });
                if !(1..=3).contains(&order) || !wedge_fits(order, mark) {
                    return err("error.draw2.bond", *id);
                }
                inv.push(Inv::SetBond(now.clone()));
                let b = &mut self.bonds[i];
                (b.o, b.s) = (order, mark);
                if *rev {
                    (b.a, b.b) = (b.b, b.a);
                }
                done.push(op.clone());
            }
            Op::Rotate { atoms, deg, x, y } => {
                let (deg, x, y) = (round4(*deg), round4(*x), round4(*y));
                let t = deg * std::f64::consts::PI / 180.0;
                let (c, s) = (m::cos(t), m::sin(t));
                for id in atoms {
                    let Some(i) = self.atom_index(*id) else {
                        return err("error.draw2.noAtom", *id);
                    };
                    inv.push(Inv::SetAtom(self.atoms[i].clone()));
                    let a = &mut self.atoms[i];
                    let (dx, dy) = (a.x - x, a.y - y);
                    a.x = round4(x + dx * c - dy * s);
                    a.y = round4(y + dx * s + dy * c);
                }
                done.push(Op::Rotate {
                    atoms: atoms.clone(),
                    deg,
                    x,
                    y,
                });
            }
            Op::Flip { atoms, axis, at } => {
                let at = round4(*at);
                for id in atoms {
                    let Some(i) = self.atom_index(*id) else {
                        return err("error.draw2.noAtom", *id);
                    };
                    inv.push(Inv::SetAtom(self.atoms[i].clone()));
                    let a = &mut self.atoms[i];
                    match axis {
                        Axis::H => a.x = round4(2.0 * at - a.x),
                        Axis::V => a.y = round4(2.0 * at - a.y),
                    }
                }
                done.push(Op::Flip {
                    atoms: atoms.clone(),
                    axis: *axis,
                    at,
                });
            }
            Op::ObjAdd { .. } | Op::ObjSet { .. } => self.object_op(op, done, inv)?,
            Op::Move {
                atoms,
                objects,
                dx,
                dy,
            } => {
                let (dx, dy) = (round4(*dx), round4(*dy));
                for id in objects {
                    let Some(i) = self.objects.iter().position(|o| o.id == *id) else {
                        return err("error.draw2.noObject", *id);
                    };
                    inv.push(Inv::SetObj(self.objects[i].clone()));
                    let o = &mut self.objects[i];
                    (o.x, o.y) = (round4(o.x + dx), round4(o.y + dy));
                    if o.t == ObjKind::Arrow {
                        (o.x2, o.y2) = (round4(o.x2 + dx), round4(o.y2 + dy));
                    }
                }
                for id in atoms {
                    let Some(i) = self.atom_index(*id) else {
                        return err("error.draw2.noAtom", *id);
                    };
                    inv.push(Inv::SetAtom(self.atoms[i].clone()));
                    self.atoms[i].x = round4(self.atoms[i].x + dx);
                    self.atoms[i].y = round4(self.atoms[i].y + dy);
                }
                done.push(Op::Move {
                    atoms: atoms.clone(),
                    objects: objects.clone(),
                    dx,
                    dy,
                });
            }
            Op::Remove {
                atoms,
                bonds,
                objects,
            } => {
                if let Some(&id) = objects.iter().find(|&&id| self.object(id).is_none()) {
                    return err("error.draw2.noObject", id);
                }
                self.remove(atoms, bonds, done, inv)?;
                for i in (0..self.objects.len()).rev() {
                    if objects.contains(&self.objects[i].id) {
                        inv.push(Inv::PutObj(i, self.objects.remove(i)));
                    }
                }
                if let Some(Op::Remove { objects: o, .. }) = done.last_mut() {
                    *o = objects.clone();
                }
            }
            Op::Ring { .. } => {
                for p in build::ring(self, op)? {
                    self.apply_one(&p, done, inv)?;
                }
            }
        }
        Ok(())
    }

    /// Adds an object or sets its text or end point; an object that would not be valid (`Obj::valid`) is refused.
    fn object_op(
        &mut self,
        op: &Op,
        done: &mut Vec<Op>,
        inv: &mut Vec<Inv>,
    ) -> Result<(), OpError> {
        match op {
            Op::ObjAdd {
                id,
                t,
                x,
                y,
                x2,
                y2,
                text,
            } => {
                let mut o = Obj {
                    id: id.unwrap_or(self.next),
                    t: *t,
                    x: round4(*x),
                    y: round4(*y),
                    x2: round4(*x2),
                    y2: round4(*y2),
                    text: text.clone(),
                };
                if !o.valid() {
                    return err("error.draw2.object", o.id);
                }
                o.id = self.take_id(*id, inv)?;
                inv.push(Inv::DropObj(o.id));
                done.push(Op::ObjAdd {
                    id: Some(o.id),
                    t: o.t,
                    x: o.x,
                    y: o.y,
                    x2: o.x2,
                    y2: o.y2,
                    text: o.text.clone(),
                });
                self.objects.push(o);
            }
            Op::ObjSet { id, text, x2, y2 } => {
                let Some(i) = self.objects.iter().position(|o| o.id == *id) else {
                    return err("error.draw2.noObject", *id);
                };
                let mut o = self.objects[i].clone();
                o.text = text.clone().unwrap_or(o.text);
                o.x2 = x2.map_or(o.x2, round4);
                o.y2 = y2.map_or(o.y2, round4);
                if !o.valid() {
                    return err("error.draw2.object", *id);
                }
                inv.push(Inv::SetObj(std::mem::replace(&mut self.objects[i], o)));
                done.push(Op::ObjSet {
                    id: *id,
                    text: text.clone(),
                    x2: x2.map(round4),
                    y2: y2.map(round4),
                });
            }
            _ => {}
        }
        Ok(())
    }

    /// Removes atoms (with every bond on them) and bonds; positions in the pools are recorded so that undo puts each one back in place.
    fn remove(
        &mut self,
        atoms: &[u32],
        bonds: &[u32],
        done: &mut Vec<Op>,
        inv: &mut Vec<Inv>,
    ) -> Result<(), OpError> {
        if let Some(&id) = atoms.iter().find(|&&id| self.atom(id).is_none()) {
            return err("error.draw2.noAtom", id);
        }
        if let Some(&id) = bonds.iter().find(|&&id| self.bond(id).is_none()) {
            return err("error.draw2.noBond", id);
        }
        for i in (0..self.bonds.len()).rev() {
            let b = &self.bonds[i];
            if bonds.contains(&b.id) || atoms.contains(&b.a) || atoms.contains(&b.b) {
                inv.push(Inv::PutBond(i, self.bonds.remove(i)));
            }
        }
        for i in (0..self.atoms.len()).rev() {
            if atoms.contains(&self.atoms[i].id) {
                inv.push(Inv::PutAtom(i, self.atoms.remove(i)));
            }
        }
        done.push(Op::Remove {
            atoms: atoms.to_vec(),
            bonds: bonds.to_vec(),
            objects: Vec::new(),
        });
        Ok(())
    }

    /// Applies a group of operations as one step: all or nothing. Returns the resolved primitive operations and the inverse.
    pub(crate) fn apply(&mut self, ops: &[Op]) -> Result<(Vec<Op>, Vec<Inv>), OpError> {
        let (mut done, mut inv) = (Vec::new(), Vec::new());
        for op in ops {
            if let Err(e) = self.apply_one(op, &mut done, &mut inv) {
                self.revert(inv);
                return Err(e);
            }
        }
        Ok((done, inv))
    }
}

struct Step {
    ops: Vec<Op>,
    inv: Vec<Inv>,
}

/// A document with its history. `run` is one undo step.
#[derive(Default)]
pub struct Session {
    pub doc: Document,
    undo: Vec<Step>,
    redo: Vec<Vec<Op>>,
}

impl Session {
    pub fn new(doc: Document) -> Self {
        Session {
            doc,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Applies the operations as one step and forgets the redo stack. An empty group is not a step.
    pub fn run(&mut self, ops: &[Op]) -> Result<(), OpError> {
        let (done, inv) = self.doc.apply(ops)?;
        if !done.is_empty() {
            self.undo.push(Step { ops: done, inv });
            self.redo.clear();
        }
        Ok(())
    }

    /// Number of steps that can be undone.
    pub fn steps(&self) -> usize {
        self.undo.len()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(s) = self.undo.pop() else {
            return false;
        };
        self.doc.revert(s.inv);
        self.redo.push(s.ops);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(ops) = self.redo.pop() else {
            return false;
        };
        // resolved operations of a step that applied before on this same document: they apply again
        if let Ok((done, inv)) = self.doc.apply(&ops) {
            self.undo.push(Step { ops: done, inv });
        }
        true
    }

    /// The command log: every operation still applied, in order, as primitive operations with their ids.
    pub fn log(&self) -> Vec<Op> {
        self.undo
            .iter()
            .flat_map(|s| s.ops.iter().cloned())
            .collect()
    }

    pub fn log_json(&self) -> String {
        serde_json::to_string(&self.log()).unwrap_or_default()
    }

    /// Replays a JSON command log, one step per operation; stops at the first refused operation.
    pub fn replay(&mut self, log_json: &str) -> Result<(), OpError> {
        let ops: Vec<Op> = serde_json::from_str(log_json).map_err(|_| OpError {
            key: "error.draw2.log",
            id: 0,
        })?;
        for op in &ops {
            self.run(std::slice::from_ref(op))?;
        }
        Ok(())
    }
}
