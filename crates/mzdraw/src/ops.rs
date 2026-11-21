//! Edit operations: the command log language (the draft of the `mol draw --set edit=` patches). Every operation applied to a document
//! records an exact inverse, so undo restores the document byte for byte; a `Session` is a document with its undo and redo stacks.

use crate::build;
use crate::doc::{round4, Atom, Bond, Document, Obj, ObjKind, Writer, MAX_ID};
use crate::m;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

/// The direction of a flip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    H,
    V,
}

/// One operation. Ids left out are taken from the document counter; `ring` is shorthand that expands to atoms and bonds.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    AtomAdd {
        id: Option<u32>,
        el: String,
        x: f64,
        y: f64,
        /// The atom-to-atom map number, 1..=999 (absent: none).
        map: Option<u16>,
    },
    BondAdd {
        id: Option<u32>,
        a: u32,
        b: u32,
        o: u8,
        /// Wedge mark of a single bond (1 solid, 2 hashed, 3 wavy), narrow end at `a`.
        s: u8,
    },
    AtomSet {
        id: u32,
        el: Option<String>,
        chg: Option<i8>,
        iso: Option<u16>,
        rad: Option<u8>,
        h: Option<u8>,
        /// A label (an abbreviation or an R group); the empty text takes the label away.
        lab: Option<String>,
        /// The atom-to-atom map number, 1..=999; 0 takes it away.
        map: Option<u16>,
    },
    /// Replaces the data kept on an atom (the data fields of its structure: name and value pairs, `doc::data_valid`); an empty
    /// list takes them away.
    AtomData {
        id: u32,
        data: Vec<(String, String)>,
    },
    /// Replaces the annotation kept on an atom (the annotation of its structure: `ann` is the JSON text of a list of entries,
    /// `ann::canon`); the empty list takes it away.
    AtomAnn { id: u32, ann: String },
    /// Sets the explicit colour of atoms and bonds (`col` is `#rgb` or `#rrggbb`; the empty text is automatic and takes the colour
    /// away: the element colour or the mono ink shows again). The inverse restores each atom and bond as it was.
    Colour {
        atoms: Vec<u32>,
        bonds: Vec<u32>,
        objects: Vec<u32>,
        col: String,
    },
    /// Sets the order and/or the wedge mark of a bond; a new order other than 1 drops the mark. `rev` swaps the two ends (the narrow
    /// end of a wedge moves to the other atom).
    BondSet {
        id: u32,
        o: Option<u8>,
        s: Option<u8>,
        rev: bool,
    },
    /// Turns the atoms by `deg` degrees counter-clockwise around (`x`, `y`).
    Rotate {
        atoms: Vec<u32>,
        deg: f64,
        x: f64,
        y: f64,
    },
    /// Mirrors the atoms: axis `h` left to right across the vertical line x = `at`, axis `v` top to bottom across y = `at`. The wedge
    /// marks stay as drawn, so the mirror image is the other enantiomer.
    Flip {
        atoms: Vec<u32>,
        axis: Axis,
        at: f64,
    },
    Move {
        atoms: Vec<u32>,
        objects: Vec<u32>,
        dx: f64,
        dy: f64,
    },
    Remove {
        atoms: Vec<u32>,
        bonds: Vec<u32>,
        objects: Vec<u32>,
    },
    /// A reaction arrow (`t` "arrow", from `x`,`y` to `x2`,`y2`, optional `text` above), a plus sign (`t` "plus") or a text
    /// (`t` "text").
    ObjAdd {
        id: Option<u32>,
        t: ObjKind,
        x: f64,
        y: f64,
        x2: f64,
        y2: f64,
        text: String,
        /// Arrows: an atom of the structure the arrow leaves and of the one it reaches (0: a free end).
        from: u32,
        to: u32,
    },
    /// Sets the text, the end point and/or the anchors of an object (`from`, `to`: an atom id, or 0 to free the end).
    ObjSet {
        id: u32,
        text: Option<String>,
        x2: Option<f64>,
        y2: Option<f64>,
        from: Option<u32>,
        to: Option<u32>,
    },
    /// A ring of `n` atoms (3-8), `arom` = alternating double bonds: fused on the bond `bond`, or with the atom `atom` as a corner, or
    /// centred on (`x`, `y`).
    Ring {
        n: u8,
        arom: bool,
        atom: Option<u32>,
        bond: Option<u32>,
        x: f64,
        y: f64,
    },
}

/// The operations as the JSON text of a command log. Written by hand (`Op::write`), as the reader is: the derived writer of the
/// enum cost about 3 kB of `.wasm`.
pub fn log_text(ops: &[Op]) -> String {
    let mut out = String::from("[");
    for (i, op) in ops.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        op.write(&mut out);
    }
    out.push(']');
    out
}

impl Op {
    /// The operation as a JSON object (see `write`).
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    /// Appends the operation as a JSON object: `op` first, then the fields in the order of the enum, defaults left out.
    pub fn write(&self, out: &mut String) {
        out.push('{');
        let mut w = Writer(std::mem::take(out));
        let ids = |w: &mut Writer, k: &str, v: &[u32], always: bool| {
            if always || !v.is_empty() {
                let list: Vec<String> = v.iter().map(u32::to_string).collect();
                w.raw(k, &format!("[{}]", list.join(",")));
            }
        };
        let int = |w: &mut Writer, k: &str, v: Option<i64>| {
            if let Some(v) = v {
                w.int(k, v);
            }
        };
        let flag = |w: &mut Writer, k: &str, v: bool| {
            if v {
                w.raw(k, "true");
            }
        };
        let id = |v: &Option<u32>| v.map(i64::from);
        match self {
            Op::AtomAdd {
                id: i,
                el,
                x,
                y,
                map,
            } => {
                w.text("op", "atom.add");
                int(&mut w, "id", id(i));
                w.text("el", el);
                w.num("x", *x);
                w.num("y", *y);
                int(&mut w, "map", map.map(i64::from));
            }
            Op::BondAdd { id: i, a, b, o, s } => {
                w.text("op", "bond.add");
                int(&mut w, "id", id(i));
                w.int("a", i64::from(*a));
                w.int("b", i64::from(*b));
                w.int("o", i64::from(*o));
                int(&mut w, "s", (*s != 0).then_some(i64::from(*s)));
            }
            Op::AtomSet {
                id: i,
                el,
                chg,
                iso,
                rad,
                h,
                lab,
                map,
            } => {
                w.text("op", "atom.set");
                w.int("id", i64::from(*i));
                if let Some(el) = el {
                    w.text("el", el);
                }
                int(&mut w, "chg", chg.map(i64::from));
                int(&mut w, "iso", iso.map(i64::from));
                int(&mut w, "rad", rad.map(i64::from));
                int(&mut w, "h", h.map(i64::from));
                if let Some(lab) = lab {
                    w.text("lab", lab);
                }
                int(&mut w, "map", map.map(i64::from));
            }
            Op::AtomData { id: i, data } => {
                w.text("op", "atom.data");
                w.int("id", i64::from(*i));
                let rows: Vec<String> = data
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "[{},{}]",
                            serde_json::Value::from(k.as_str()),
                            serde_json::Value::from(v.as_str())
                        )
                    })
                    .collect();
                w.raw("data", &format!("[{}]", rows.join(",")));
            }
            Op::AtomAnn { id: i, ann } => {
                w.text("op", "atom.ann");
                w.int("id", i64::from(*i));
                w.raw("ann", ann);
            }
            Op::Colour {
                atoms,
                bonds,
                objects,
                col,
            } => {
                w.text("op", "colour");
                ids(&mut w, "atoms", atoms, false);
                ids(&mut w, "bonds", bonds, false);
                ids(&mut w, "objects", objects, false);
                w.text("col", col);
            }
            Op::BondSet { id: i, o, s, rev } => {
                w.text("op", "bond.set");
                w.int("id", i64::from(*i));
                int(&mut w, "o", o.map(i64::from));
                int(&mut w, "s", s.map(i64::from));
                flag(&mut w, "rev", *rev);
            }
            Op::Rotate { atoms, deg, x, y } => {
                w.text("op", "rotate");
                ids(&mut w, "atoms", atoms, true);
                w.num("deg", *deg);
                w.num("x", *x);
                w.num("y", *y);
            }
            Op::Flip { atoms, axis, at } => {
                w.text("op", "flip");
                ids(&mut w, "atoms", atoms, true);
                w.text("axis", if *axis == Axis::H { "h" } else { "v" });
                w.num("at", *at);
            }
            Op::Move {
                atoms,
                objects,
                dx,
                dy,
            } => {
                w.text("op", "move");
                ids(&mut w, "atoms", atoms, false);
                ids(&mut w, "objects", objects, false);
                w.num("dx", *dx);
                w.num("dy", *dy);
            }
            Op::Remove {
                atoms,
                bonds,
                objects,
            } => {
                w.text("op", "remove");
                ids(&mut w, "atoms", atoms, false);
                ids(&mut w, "bonds", bonds, false);
                ids(&mut w, "objects", objects, false);
            }
            Op::ObjAdd {
                id: i,
                t,
                x,
                y,
                x2,
                y2,
                text,
                from,
                to,
            } => {
                w.text("op", "obj.add");
                int(&mut w, "id", id(i));
                w.text("t", t.name());
                w.num("x", *x);
                w.num("y", *y);
                for (k, v) in [("x2", *x2), ("y2", *y2)] {
                    if v != 0.0 {
                        w.num(k, v);
                    }
                }
                if !text.is_empty() {
                    w.text("text", text);
                }
                for (k, v) in [("from", *from), ("to", *to)] {
                    if v != 0 {
                        w.int(k, i64::from(v));
                    }
                }
            }
            Op::ObjSet {
                id: i,
                text,
                x2,
                y2,
                from,
                to,
            } => {
                w.text("op", "obj.set");
                w.int("id", i64::from(*i));
                if let Some(text) = text {
                    w.text("text", text);
                }
                for (k, v) in [("x2", x2), ("y2", y2)] {
                    if let Some(v) = v {
                        w.num(k, *v);
                    }
                }
                for (k, v) in [("from", from), ("to", to)] {
                    if let Some(v) = v {
                        w.int(k, i64::from(*v));
                    }
                }
            }
            Op::Ring {
                n,
                arom,
                atom,
                bond,
                x,
                y,
            } => {
                w.text("op", "ring");
                w.int("n", i64::from(*n));
                flag(&mut w, "arom", *arom);
                int(&mut w, "atom", id(atom));
                int(&mut w, "bond", id(bond));
                w.num("x", *x);
                w.num("y", *y);
            }
        }
        *out = w.0;
        out.push('}');
    }
}

/// Read by hand from a JSON value (`Op::from_value`): the derived reader of a tagged enum costs about 2 kB of `.wasm` per variant.
impl<'de> Deserialize<'de> for Op {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        Op::from_value(&v).ok_or_else(|| serde::de::Error::custom("not an operation"))
    }
}

/// Field readers of `Op::from_value`: `None` when a field is absent, `Some(None)`-free by design: a field of the wrong type makes
/// the whole operation unreadable.
pub(crate) struct Fields<'a>(pub(crate) &'a serde_json::Value);

impl Fields<'_> {
    /// An optional field read by `f`: `Ok(None)` when absent or null, `Err` when present with another type.
    pub(crate) fn opt<T>(
        &self,
        k: &str,
        f: impl Fn(&serde_json::Value) -> Option<T>,
    ) -> Result<Option<T>, ()> {
        match self.0.get(k) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(v) => f(v).map(Some).ok_or(()),
        }
    }
    pub(crate) fn f(&self, k: &str) -> Result<Option<f64>, ()> {
        self.opt(k, serde_json::Value::as_f64)
    }
    pub(crate) fn u<T: TryFrom<u64>>(&self, k: &str) -> Result<Option<T>, ()> {
        self.opt(k, |v| v.as_u64().and_then(|n| T::try_from(n).ok()))
    }
    pub(crate) fn i8(&self, k: &str) -> Result<Option<i8>, ()> {
        self.opt(k, |v| v.as_i64().and_then(|n| i8::try_from(n).ok()))
    }
    pub(crate) fn s(&self, k: &str) -> Result<Option<String>, ()> {
        self.opt(k, |v| v.as_str().map(str::to_string))
    }
    pub(crate) fn b(&self, k: &str) -> Result<bool, ()> {
        Ok(self.opt(k, serde_json::Value::as_bool)?.unwrap_or(false))
    }
    pub(crate) fn ids(&self, k: &str) -> Result<Vec<u32>, ()> {
        let list = self.opt(k, |v| {
            v.as_array()?
                .iter()
                .map(|x| x.as_u64().and_then(|n| u32::try_from(n).ok()))
                .collect::<Option<Vec<u32>>>()
        })?;
        Ok(list.unwrap_or_default())
    }
}

impl Op {
    /// An operation from its JSON object; `None` for an unknown `op`, a field the table of the operation does not list
    /// (`schema::OPS`), a missing required field or a field of the wrong type.
    pub fn from_value(v: &serde_json::Value) -> Option<Op> {
        Self::parse(v).ok()
    }

    /// `from_value` with the reason as an i18n key: `error.draw2.opField` for a field outside the table of the operation,
    /// `error.draw2.request` for anything else.
    pub fn parse(v: &serde_json::Value) -> Result<Op, &'static str> {
        const BAD: &str = "error.draw2.request";
        let name = v.get("op").and_then(|n| n.as_str()).ok_or(BAD)?;
        let spec = crate::schema::spec(name).filter(|s| s.mzdraw).ok_or(BAD)?;
        if let Some(obj) = v.as_object() {
            let known = |k: &str| k == "op" || spec.fields.iter().any(|f| f.key == k);
            if !obj.keys().all(|k| known(k)) {
                return Err("error.draw2.opField");
            }
        }
        Self::read(&Fields(v), name).map_err(|()| BAD)
    }

    fn read(g: &Fields, name: &str) -> Result<Op, ()> {
        let need = |v: Option<f64>| v.ok_or(());
        Ok(match name {
            "atom.add" => Op::AtomAdd {
                id: g.u("id")?,
                el: g.s("el")?.ok_or(())?,
                x: need(g.f("x")?)?,
                y: need(g.f("y")?)?,
                map: g.u("map")?,
            },
            "bond.add" => Op::BondAdd {
                id: g.u("id")?,
                a: g.u("a")?.ok_or(())?,
                b: g.u("b")?.ok_or(())?,
                o: g.u("o")?.unwrap_or(1),
                s: g.u("s")?.unwrap_or(0),
            },
            "atom.set" => Op::AtomSet {
                id: g.u("id")?.ok_or(())?,
                el: g.s("el")?,
                chg: g.i8("chg")?,
                iso: g.u("iso")?,
                rad: g.u("rad")?,
                h: g.u("h")?,
                lab: g.s("lab")?,
                map: g.u("map")?,
            },
            "atom.data" => Op::AtomData {
                id: g.u("id")?.ok_or(())?,
                data: (g.0.get("data").and_then(|d| d.as_array()).ok_or(())?.iter())
                    .map(|r| {
                        Some((
                            r.get(0)?.as_str()?.to_string(),
                            r.get(1)?.as_str()?.to_string(),
                        ))
                    })
                    .collect::<Option<Vec<(String, String)>>>()
                    .ok_or(())?,
            },
            "atom.ann" => Op::AtomAnn {
                id: g.u("id")?.ok_or(())?,
                ann: (g.0.get("ann").filter(|a| a.is_array()).ok_or(())?).to_string(),
            },
            "colour" => Op::Colour {
                atoms: g.ids("atoms")?,
                bonds: g.ids("bonds")?,
                objects: g.ids("objects")?,
                col: g.s("col")?.ok_or(())?,
            },
            "bond.set" => Op::BondSet {
                id: g.u("id")?.ok_or(())?,
                o: g.u("o")?,
                s: g.u("s")?,
                rev: g.b("rev")?,
            },
            "rotate" => Op::Rotate {
                atoms: g.ids("atoms")?,
                deg: need(g.f("deg")?)?,
                x: need(g.f("x")?)?,
                y: need(g.f("y")?)?,
            },
            "flip" => Op::Flip {
                atoms: g.ids("atoms")?,
                axis: match g.s("axis")?.as_deref() {
                    Some("h") => Axis::H,
                    Some("v") => Axis::V,
                    _ => return Err(()),
                },
                at: need(g.f("at")?)?,
            },
            "move" => Op::Move {
                atoms: g.ids("atoms")?,
                objects: g.ids("objects")?,
                dx: need(g.f("dx")?)?,
                dy: need(g.f("dy")?)?,
            },
            "remove" => Op::Remove {
                atoms: g.ids("atoms")?,
                bonds: g.ids("bonds")?,
                objects: g.ids("objects")?,
            },
            "obj.add" => Op::ObjAdd {
                id: g.u("id")?,
                t: ObjKind::read(g)?,
                x: need(g.f("x")?)?,
                y: need(g.f("y")?)?,
                x2: g.f("x2")?.unwrap_or(0.0),
                y2: g.f("y2")?.unwrap_or(0.0),
                text: g.s("text")?.unwrap_or_default(),
                from: g.u("from")?.unwrap_or(0),
                to: g.u("to")?.unwrap_or(0),
            },
            "obj.set" => Op::ObjSet {
                id: g.u("id")?.ok_or(())?,
                text: g.s("text")?,
                x2: g.f("x2")?,
                y2: g.f("y2")?,
                from: g.u("from")?,
                to: g.u("to")?,
            },
            "ring" => Op::Ring {
                n: g.u("n")?.ok_or(())?,
                arom: g.b("arom")?,
                atom: g.u("atom")?,
                bond: g.u("bond")?,
                x: g.f("x")?.unwrap_or(0.0),
                y: g.f("y")?.unwrap_or(0.0),
            },
            _ => return Err(()),
        })
    }
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
            || id >= MAX_ID
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

    /// The pool position of atom `id`: through the index when the operation made one (a long list), else by search (a short list
    /// must not pay for an index of the whole pool).
    fn position(&self, index: &Option<HashMap<u32, usize>>, id: u32) -> Option<usize> {
        match index {
            Some(m) => m.get(&id).copied(),
            None => self.atom_index(id),
        }
    }

    /// The position of every atom in the pool, by id: for an operation that names a long list of atoms and does not change the pool.
    fn atom_positions(&self) -> HashMap<u32, usize> {
        (self.atoms.iter().enumerate())
            .map(|(i, a)| (a.id, i))
            .collect()
    }

    pub(crate) fn revert(&mut self, inv: Vec<Inv>) {
        // a step that only set things (a move, a turn, a mirror) leaves the pools as they are: one index each, not one search per entry
        if inv.len() > 32
            && inv.iter().all(|i| {
                matches!(
                    i,
                    Inv::SetAtom(_) | Inv::SetObj(_) | Inv::SetBond(_) | Inv::Next(_)
                )
            })
        {
            let atoms = self.atom_positions();
            let objects: HashMap<u32, usize> = (self.objects.iter().enumerate())
                .map(|(i, o)| (o.id, i))
                .collect();
            let bonds: HashMap<u32, usize> = (self.bonds.iter().enumerate())
                .map(|(i, b)| (b.id, i))
                .collect();
            for i in inv.into_iter().rev() {
                match i {
                    Inv::SetAtom(a) => {
                        if let Some(&k) = atoms.get(&a.id) {
                            self.atoms[k] = a;
                        }
                    }
                    Inv::SetObj(o) => {
                        if let Some(&k) = objects.get(&o.id) {
                            self.objects[k] = o;
                        }
                    }
                    Inv::SetBond(b) => {
                        if let Some(&k) = bonds.get(&b.id) {
                            self.bonds[k] = b;
                        }
                    }
                    other => self.undo_one(other),
                }
            }
            return;
        }
        for i in inv.into_iter().rev() {
            self.undo_one(i);
        }
    }

    fn add_atom(
        &mut self,
        id: Option<u32>,
        el: &str,
        (x, y): (f64, f64),
        map: Option<u16>,
        done: &mut Vec<Op>,
        inv: &mut Vec<Inv>,
    ) -> Result<(), OpError> {
        if !mzlab_mol::is_element(el) {
            return err("error.draw2.element", id.unwrap_or(0));
        }
        if map.is_some_and(|m| m > crate::doc::MAX_MAP) {
            return err("error.draw2.map", id.unwrap_or(0));
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
            map: map.unwrap_or(0),
            h: None,
            lab: String::new(),
            more: Vec::new(),
        });
        inv.push(Inv::DropAtom(id));
        done.push(Op::AtomAdd {
            id: Some(id),
            el,
            x,
            y,
            map: map.filter(|&m| m != 0),
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
        self.bonds.push(Bond {
            id,
            a,
            b,
            o,
            s,
            more: Vec::new(),
        });
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
            map,
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
        if map.is_some_and(|m| m > crate::doc::MAX_MAP) {
            return err("error.draw2.map", *id);
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
        a.map = map.unwrap_or(a.map);
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
            Op::AtomAdd { id, el, x, y, map } => {
                self.add_atom(*id, el, (*x, *y), *map, done, inv)?
            }
            Op::BondAdd { id, a, b, o, s } => self.add_bond(*id, *a, *b, (*o, *s), done, inv)?,
            Op::AtomSet { .. } => {
                self.set_atom(op, inv)?;
                done.push(op.clone());
            }
            Op::AtomData { id, data } => {
                let Some(i) = self.atom_index(*id) else {
                    return err("error.draw2.noAtom", *id);
                };
                if !crate::doc::data_valid(data) {
                    return err("error.draw2.data", *id);
                }
                inv.push(Inv::SetAtom(self.atoms[i].clone()));
                self.atoms[i].set_data(data);
                done.push(op.clone());
            }
            Op::AtomAnn { id, ann } => {
                let Some(i) = self.atom_index(*id) else {
                    return err("error.draw2.noAtom", *id);
                };
                let Some(text) = crate::ann::canon(ann) else {
                    return err("error.draw2.ann", *id);
                };
                inv.push(Inv::SetAtom(self.atoms[i].clone()));
                self.atoms[i].set_ann(&text);
                done.push(Op::AtomAnn { id: *id, ann: text });
            }
            Op::Colour {
                atoms,
                bonds,
                objects,
                col,
            } => {
                let Some(col) = crate::ink::canon(col) else {
                    return err("error.draw2.colour", 0);
                };
                let index = (atoms.len() > 16).then(|| self.atom_positions());
                for id in atoms {
                    let Some(i) = Self::position(self, &index, *id) else {
                        return err("error.draw2.noAtom", *id);
                    };
                    inv.push(Inv::SetAtom(self.atoms[i].clone()));
                    crate::ink::put(&mut self.atoms[i].more, &col);
                }
                // a long list of bonds: one index of the pool, not one search per bond
                let by_id: Option<HashMap<u32, usize>> = (bonds.len() > 16).then(|| {
                    (self.bonds.iter().enumerate())
                        .map(|(i, b)| (b.id, i))
                        .collect()
                });
                for id in bonds {
                    let at = match &by_id {
                        Some(m) => m.get(id).copied(),
                        None => self.bond_index(*id),
                    };
                    let Some(i) = at else {
                        return err("error.draw2.noBond", *id);
                    };
                    inv.push(Inv::SetBond(self.bonds[i].clone()));
                    crate::ink::put(&mut self.bonds[i].more, &col);
                }
                for id in objects {
                    let Some(i) = self.objects.iter().position(|o| o.id == *id) else {
                        return err("error.draw2.noObject", *id);
                    };
                    inv.push(Inv::SetObj(self.objects[i].clone()));
                    crate::ink::put(&mut self.objects[i].more, &col);
                }
                done.push(Op::Colour {
                    atoms: atoms.clone(),
                    bonds: bonds.clone(),
                    objects: objects.clone(),
                    col,
                });
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
                // the pool does not change while atoms move: one index for the whole list
                let index = (atoms.len() > 16).then(|| self.atom_positions());
                for id in atoms {
                    let Some(i) = Self::position(self, &index, *id) else {
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
                let index = (atoms.len() > 16).then(|| self.atom_positions());
                for id in atoms {
                    let Some(i) = Self::position(self, &index, *id) else {
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
                let positions: Option<HashMap<u32, usize>> = (objects.len() > 16).then(|| {
                    (self.objects.iter().enumerate())
                        .map(|(i, o)| (o.id, i))
                        .collect()
                });
                for id in objects {
                    let found = match &positions {
                        Some(m) => m.get(id).copied(),
                        None => self.objects.iter().position(|o| o.id == *id),
                    };
                    let Some(i) = found else {
                        return err("error.draw2.noObject", *id);
                    };
                    inv.push(Inv::SetObj(self.objects[i].clone()));
                    let o = &mut self.objects[i];
                    (o.x, o.y) = (round4(o.x + dx), round4(o.y + dy));
                    if o.t == ObjKind::Arrow {
                        (o.x2, o.y2) = (round4(o.x2 + dx), round4(o.y2 + dy));
                    }
                }
                let index = (atoms.len() > 16).then(|| self.atom_positions());
                for id in atoms {
                    let Some(i) = Self::position(self, &index, *id) else {
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
                let have: HashSet<u32> = self.objects.iter().map(|o| o.id).collect();
                if let Some(&id) = objects.iter().find(|id| !have.contains(id)) {
                    return err("error.draw2.noObject", id);
                }
                self.remove(atoms, bonds, done, inv)?;
                let gone: HashSet<u32> = objects.iter().copied().collect();
                for i in (0..self.objects.len()).rev() {
                    if gone.contains(&self.objects[i].id) {
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
                from,
                to,
            } => {
                let mut o = Obj {
                    id: id.unwrap_or(self.next),
                    t: *t,
                    x: round4(*x),
                    y: round4(*y),
                    x2: round4(*x2),
                    y2: round4(*y2),
                    text: text.clone(),
                    from: *from,
                    to: *to,
                    more: Vec::new(),
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
                    from: o.from,
                    to: o.to,
                });
                self.objects.push(o);
            }
            Op::ObjSet {
                id,
                text,
                x2,
                y2,
                from,
                to,
            } => {
                let Some(i) = self.objects.iter().position(|o| o.id == *id) else {
                    return err("error.draw2.noObject", *id);
                };
                let mut o = self.objects[i].clone();
                o.text = text.clone().unwrap_or(o.text);
                o.x2 = x2.map_or(o.x2, round4);
                o.y2 = y2.map_or(o.y2, round4);
                o.from = from.unwrap_or(o.from);
                o.to = to.unwrap_or(o.to);
                if !o.valid() {
                    return err("error.draw2.object", *id);
                }
                inv.push(Inv::SetObj(std::mem::replace(&mut self.objects[i], o)));
                done.push(Op::ObjSet {
                    id: *id,
                    text: text.clone(),
                    x2: x2.map(round4),
                    y2: y2.map(round4),
                    from: *from,
                    to: *to,
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
        // sets, so that a long list costs its length and not its length times the pool
        let have: HashSet<u32> = self.atoms.iter().map(|a| a.id).collect();
        if let Some(&id) = atoms.iter().find(|id| !have.contains(id)) {
            return err("error.draw2.noAtom", id);
        }
        let have: HashSet<u32> = self.bonds.iter().map(|b| b.id).collect();
        if let Some(&id) = bonds.iter().find(|id| !have.contains(id)) {
            return err("error.draw2.noBond", id);
        }
        let (gone_atoms, gone_bonds): (HashSet<u32>, HashSet<u32>) = (
            atoms.iter().copied().collect(),
            bonds.iter().copied().collect(),
        );
        for i in (0..self.bonds.len()).rev() {
            let b = &self.bonds[i];
            if gone_bonds.contains(&b.id) || gone_atoms.contains(&b.a) || gone_atoms.contains(&b.b)
            {
                inv.push(Inv::PutBond(i, self.bonds.remove(i)));
            }
        }
        for i in (0..self.atoms.len()).rev() {
            if gone_atoms.contains(&self.atoms[i].id) {
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
        // a step that would put anything out of the range of coordinates is refused whole
        if !self.coords_ok() {
            self.revert(inv);
            return err("error.draw2.coordinate", 0);
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
    /// Counts every change of the history (a step, an undo, a redo): lets a caller know that nothing happened in between.
    generation: u64,
}

impl Session {
    pub fn new(doc: Document) -> Self {
        Session {
            doc,
            undo: Vec::new(),
            redo: Vec::new(),
            generation: 0,
        }
    }

    /// Applies the operations as one step and forgets the redo stack. An empty group is not a step.
    pub fn run(&mut self, ops: &[Op]) -> Result<(), OpError> {
        let (done, inv) = self.doc.apply(ops)?;
        if !done.is_empty() {
            self.undo.push(Step { ops: done, inv });
            self.redo.clear();
            self.generation += 1;
        }
        Ok(())
    }

    /// Like `run`, but the operations join the last undo step (one undo reverts both) when there is one; the log keeps every
    /// operation. Used to coalesce repeated nudges the way one drag is one step.
    pub fn run_into_last(&mut self, ops: &[Op]) -> Result<(), OpError> {
        let (done, inv) = self.doc.apply(ops)?;
        if done.is_empty() {
            return Ok(());
        }
        match self.undo.last_mut() {
            Some(last) => {
                last.ops.extend(done);
                last.inv.extend(inv);
            }
            None => self.undo.push(Step { ops: done, inv }),
        }
        self.redo.clear();
        self.generation += 1;
        Ok(())
    }

    /// The count of changes of the history so far.
    pub fn generation(&self) -> u64 {
        self.generation
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
        self.generation += 1;
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
        self.generation += 1;
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
        log_text(&self.log())
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
