//! The semantic diff of two documents as a `draw_edit/1` patch, and its hunks. Things are matched by id (ids are stable across
//! edits); the patch uses only operations of the table of `schema`, in a fixed order: removes (bonds, atoms, objects), adds (atoms,
//! bonds, objects), sets (atoms, bonds, objects), moves (grouped by equal displacement). Applying `diff(a, b)` to `a` gives `b`,
//! except what no operation can write: the counter `next` (applying raises it only to the ids added), the order of a pool after a
//! bond or object is re-added, and fields of an atom, bond, object or document that this version does not know (`more`, other than
//! `data` and `ann`, which have operations).

use crate::doc::{round4, Atom, Bond, Document, Obj, ObjKind};
use crate::ops::{log_text, Op};
use std::collections::{BTreeMap, HashMap};

/// The ends of a bond as an unordered pair.
fn ends(b: &Bond) -> (u32, u32) {
    (b.a.min(b.b), b.a.max(b.b))
}

/// The operation that turns atom `a` into atom `b` (same id), without the position (that is a move), or `None`. A new element or
/// `el` given again forgets the explicit hydrogen count and the label: that is how a count is taken away, and the label is then
/// said again.
fn atom_set(a: &Atom, b: &Atom) -> Option<Op> {
    let reset = a.el != b.el || (a.h.is_some() && b.h.is_none());
    let lab = if reset {
        (!b.lab.is_empty()).then(|| b.lab.clone())
    } else {
        (a.lab != b.lab).then(|| b.lab.clone())
    };
    let op = Op::AtomSet {
        id: b.id,
        el: reset.then(|| b.el.clone()),
        chg: (a.chg != b.chg).then_some(b.chg),
        iso: (a.iso != b.iso).then_some(b.iso),
        rad: (a.rad != b.rad).then_some(b.rad),
        h: if reset || a.h != b.h { b.h } else { None },
        lab,
        map: (a.map != b.map).then_some(b.map),
    };
    let empty = matches!(
        &op,
        Op::AtomSet {
            el: None,
            chg: None,
            iso: None,
            rad: None,
            h: None,
            lab: None,
            map: None,
            ..
        }
    );
    (!empty).then_some(op)
}

fn obj_add(o: &Obj) -> Op {
    Op::ObjAdd {
        id: Some(o.id),
        t: o.t,
        x: o.x,
        y: o.y,
        x2: o.x2,
        y2: o.y2,
        text: if o.runs.is_empty() {
            o.text.clone()
        } else {
            String::new()
        },
        runs: o.runs.clone(),
        from: o.from,
        to: o.to,
    }
}

/// Atoms and objects that move by the same displacement, in the order they were met.
#[derive(Default)]
struct Moves(Vec<Group>, HashMap<(u64, u64), usize>);

/// The bits of a displacement, the atoms and the objects that move by it.
type Group = ((u64, u64), Vec<u32>, Vec<u32>);

impl Moves {
    fn put(&mut self, d: (f64, f64), atom: Option<u32>, object: Option<u32>) {
        let key = (d.0.to_bits(), d.1.to_bits());
        let groups = &mut self.0;
        let i = *self.1.entry(key).or_insert_with(|| {
            groups.push((key, Vec::new(), Vec::new()));
            groups.len() - 1
        });
        self.0[i].1.extend(atom);
        self.0[i].2.extend(object);
    }
}

/// The atoms, bonds and objects of a document by id.
type Pools<'a> = (
    HashMap<u32, &'a Atom>,
    HashMap<u32, &'a Bond>,
    HashMap<u32, &'a Obj>,
);

fn by_id(d: &Document) -> Pools<'_> {
    (
        d.atoms.iter().map(|x| (x.id, x)).collect(),
        d.bonds.iter().map(|x| (x.id, x)).collect(),
        d.objects.iter().map(|x| (x.id, x)).collect(),
    )
}

/// The patch that turns `a` into `b`; empty when they are the same drawing.
pub fn diff(a: &Document, b: &Document) -> Vec<Op> {
    let mut out = Vec::new();
    // one index per pool, so that the lookups below cost one each on a big document
    let ((a_atoms, a_bonds, a_objects), (b_atoms, b_bonds, b_objects)) = (by_id(a), by_id(b));
    let (a_atom, a_bond, a_object) = (
        |id: u32| a_atoms.get(&id).copied(),
        |id: u32| a_bonds.get(&id).copied(),
        |id: u32| a_objects.get(&id).copied(),
    );
    let (b_atom, b_bond, b_object) = (
        |id: u32| b_atoms.get(&id).copied(),
        |id: u32| b_bonds.get(&id).copied(),
        |id: u32| b_objects.get(&id).copied(),
    );
    let (kept_a, kept_b) = (|id| b_atom(id).is_some(), |id| a_atom(id).is_some());
    // a bond is the same bond when it has the same id and the same two atoms (the ends may be swapped); another pair is another bond
    let same_bond = |x: &Bond, y: &Bond| ends(x) == ends(y);
    let bond_kept = |x: &Bond| b_bond(x.id).is_some_and(|y| same_bond(x, y));
    let bond_new = |y: &Bond| a_bond(y.id).is_none_or(|x| !same_bond(x, y));
    let obj_kept = |o: &Obj| b_object(o.id).is_some_and(|p| p.t == o.t);
    let obj_new = |p: &Obj| a_object(p.id).is_none_or(|o| o.t != p.t);

    // removes
    let bonds = a
        .bonds
        .iter()
        .filter(|x| !bond_kept(x))
        .map(|x| x.id)
        .collect();
    let atoms = a
        .atoms
        .iter()
        .filter(|x| !kept_a(x.id))
        .map(|x| x.id)
        .collect();
    let objects = a
        .objects
        .iter()
        .filter(|o| !obj_kept(o))
        .map(|o| o.id)
        .collect();
    for (bonds, atoms, objects) in [
        (bonds, vec![], vec![]),
        (vec![], atoms, vec![]),
        (vec![], vec![], objects),
    ] {
        if !(bonds.is_empty() && atoms.is_empty() && objects.is_empty()) {
            out.push(Op::Remove {
                atoms,
                bonds,
                objects,
            });
        }
    }

    // adds
    for y in b.atoms.iter().filter(|y| !kept_b(y.id)) {
        out.push(Op::AtomAdd {
            id: Some(y.id),
            el: y.el.clone(),
            x: y.x,
            y: y.y,
            map: (y.map != 0).then_some(y.map),
        });
    }
    for y in b.bonds.iter().filter(|y| bond_new(y)) {
        out.push(Op::BondAdd {
            id: Some(y.id),
            a: y.a,
            b: y.b,
            o: y.o,
            s: y.s,
        });
    }
    for p in b.objects.iter().filter(|p| obj_new(p)) {
        out.push(obj_add(p));
    }

    // sets: atoms (what the plain add could not say too), bonds, objects
    let blank = |id| Atom {
        id,
        el: String::new(),
        x: 0.0,
        y: 0.0,
        chg: 0,
        iso: 0,
        rad: 0,
        h: None,
        lab: String::new(),
        map: 0,
        more: Vec::new(),
    };
    for y in &b.atoms {
        // a new atom was added with its element and map: the rest is said here
        let mut base = blank(y.id);
        base.el = y.el.clone();
        base.map = y.map;
        let x = a_atom(y.id).unwrap_or(&base);
        out.extend(atom_set(x, y));
        if x.data() != y.data() {
            out.push(Op::AtomData {
                id: y.id,
                data: y.data(),
            });
        }
        if x.ann() != y.ann() {
            let ann = y.ann();
            out.push(Op::AtomAnn {
                id: y.id,
                ann: if ann.is_empty() { "[]".into() } else { ann },
            });
        }
        if x.cap() != y.cap() {
            out.push(Op::AtomCap {
                id: y.id,
                lines: y.cap(),
            });
        }
        if x.col() != y.col() {
            out.push(Op::Colour {
                atoms: vec![y.id],
                bonds: Vec::new(),
                objects: Vec::new(),
                col: y.col(),
            });
        }
    }
    for y in &b.bonds {
        let Some(x) = a_bond(y.id).filter(|x| same_bond(x, y)) else {
            // a bond added now: its colour is said with it
            if !y.col().is_empty() {
                out.push(Op::Colour {
                    atoms: Vec::new(),
                    bonds: vec![y.id],
                    objects: Vec::new(),
                    col: y.col(),
                });
            }
            continue;
        };
        let rev = x.a != y.a;
        if x.col() != y.col() {
            out.push(Op::Colour {
                atoms: Vec::new(),
                bonds: vec![y.id],
                objects: Vec::new(),
                col: y.col(),
            });
        }
        if x.o != y.o || x.s != y.s || rev {
            out.push(Op::BondSet {
                id: y.id,
                o: (x.o != y.o).then_some(y.o),
                s: (x.s != y.s).then_some(y.s),
                rev,
            });
        }
    }
    let mut moves = Moves::default();
    for y in &b.atoms {
        if let Some(x) = a_atom(y.id) {
            let d = (round4(y.x - x.x), round4(y.y - x.y));
            if d != (0.0, 0.0) {
                moves.put(d, Some(y.id), None);
            }
        }
    }
    for p in &b.objects {
        let Some(o) = a_object(p.id).filter(|o| o.t == p.t) else {
            // an object added now: its colour is said with it
            if !p.col().is_empty() {
                out.push(Op::Colour {
                    atoms: Vec::new(),
                    bonds: Vec::new(),
                    objects: vec![p.id],
                    col: p.col(),
                });
            }
            continue;
        };
        let d = (round4(p.x - o.x), round4(p.y - o.y));
        // an arrow moves with its two ends; a different end point is set in the frame before the move
        let end = |v: f64, d: f64, was: f64| {
            let want = round4(v - d);
            (p.t == ObjKind::Arrow && want != was).then_some(want)
        };
        let (x2, y2) = (end(p.x2, d.0, o.x2), end(p.y2, d.1, o.y2));
        let (from, to) = (
            (o.from != p.from).then_some(p.from),
            (o.to != p.to).then_some(p.to),
        );
        let changed = o.text != p.text || o.runs != p.runs;
        let runs = (changed && !p.runs.is_empty()).then(|| p.runs.clone());
        let text = (changed && p.runs.is_empty()).then(|| p.text.clone());
        if x2.is_some() || y2.is_some() || from.is_some() || to.is_some() || changed {
            out.push(Op::ObjSet {
                id: p.id,
                text,
                runs,
                x2,
                y2,
                from,
                to,
            });
        }
        if o.col() != p.col() {
            out.push(Op::Colour {
                atoms: Vec::new(),
                bonds: Vec::new(),
                objects: vec![p.id],
                col: p.col(),
            });
        }
        if d != (0.0, 0.0) {
            moves.put(d, None, Some(p.id));
        }
    }
    // moves
    for (key, atoms, objects) in moves.0 {
        out.push(Op::Move {
            atoms,
            objects,
            dx: f64::from_bits(key.0),
            dy: f64::from_bits(key.1),
        });
    }
    // the mass-label choice of the scheme
    if a.labels() != b.labels() {
        let (mode, adduct) = b.labels();
        out.push(Op::DocLabels { mode, adduct });
    }
    out
}

/// A part of the patch a person can accept or reject on its own: the operations that touch one connected piece of the drawing or
/// one object. Every hunk is a patch that applies to the first document on its own.
#[derive(Clone, Debug, PartialEq)]
pub struct Hunk {
    /// The ids the hunk touches, ascending.
    pub ids: Vec<u32>,
    pub ops: Vec<Op>,
}

/// A union-find over ids: iterative (no recursion on a long chain), with the smaller set joined under the larger one and the path
/// compressed, so a call is near constant; the representative of a set is its smallest id, so the hunks do not depend on the order.
#[derive(Default)]
struct Sets {
    parent: BTreeMap<u32, u32>,
    size: BTreeMap<u32, u32>,
    least: BTreeMap<u32, u32>,
}

impl Sets {
    fn root(&mut self, id: u32) -> u32 {
        let mut r = id;
        while let Some(&p) = self.parent.get(&r) {
            if p == r {
                break;
            }
            r = p;
        }
        let mut at = id;
        while at != r {
            let next = self.parent.insert(at, r).unwrap_or(r);
            at = next;
        }
        self.parent.entry(r).or_insert(r);
        r
    }
    /// The smallest id of the set of `id`.
    fn find(&mut self, id: u32) -> u32 {
        let r = self.root(id);
        *self.least.get(&r).unwrap_or(&r)
    }
    fn join(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.root(a), self.root(b));
        if ra == rb {
            return;
        }
        let (la, lb) = (self.find(a), self.find(b));
        let (sa, sb) = (
            *self.size.get(&ra).unwrap_or(&1),
            *self.size.get(&rb).unwrap_or(&1),
        );
        let (big, small) = if sa >= sb { (ra, rb) } else { (rb, ra) };
        self.parent.insert(small, big);
        self.size.insert(big, sa + sb);
        self.least.insert(big, la.min(lb));
    }
}

/// Splits a list of ids by the piece each belongs to, in the order of first appearance.
fn split(sets: &mut Sets, lists: &[&[u32]]) -> Vec<(u32, Vec<Vec<u32>>)> {
    let mut out: Vec<(u32, Vec<Vec<u32>>)> = Vec::new();
    let mut at: BTreeMap<u32, usize> = BTreeMap::new();
    for (k, list) in lists.iter().enumerate() {
        for &id in *list {
            let r = sets.find(id);
            let i = *at.entry(r).or_insert_with(|| {
                out.push((r, vec![Vec::new(); lists.len()]));
                out.len() - 1
            });
            out[i].1[k].push(id);
        }
    }
    out
}

/// The hunks of a patch of `diff` (or any patch of `a` made of the operations `diff` writes). A piece is a connected piece of `a`
/// joined by the bonds the patch adds; an id used by several operations (an atom and a bond that swap ids, an object moved and
/// reshaped) keeps them together, so the hunks do not depend on each other and any subset applies in order. An operation that
/// touches several pieces (`remove`, `move`) is split. A patch with an operation `diff` never writes (`rotate`, `flip`, `ring`), an
/// add that names no id (its id comes from the counter) or an operation that names nothing is one hunk. The hunks come in the order of their first operation.
pub fn hunks(a: &Document, ops: &[Op]) -> Vec<Hunk> {
    // the mass-label choice of the scheme names no id and depends on nothing in the drawing: it is a hunk of its own, last
    if ops.iter().any(|o| matches!(o, Op::DocLabels { .. })) {
        let (doc, rest): (Vec<Op>, Vec<Op>) = (ops.iter().cloned())
            .partition(|o| matches!(o, Op::DocLabels { .. }));
        let mut out = if rest.is_empty() {
            Vec::new()
        } else {
            hunks(a, &rest)
        };
        out.push(Hunk {
            ids: Vec::new(),
            ops: doc,
        });
        return out;
    }
    if !ops.iter().all(splittable) {
        let mut ids = Vec::new();
        for op in ops {
            ids.extend(touched(op));
        }
        crate::m::sort(&mut ids);
        ids.dedup();
        return vec![Hunk {
            ids,
            ops: ops.to_vec(),
        }];
    }
    let mut sets = Sets::default();
    for x in &a.bonds {
        sets.join(x.id, x.a);
        sets.join(x.id, x.b);
    }
    for op in ops {
        match op {
            Op::BondAdd { id, a, b, .. } => {
                sets.join(id.unwrap_or_default(), *a);
                sets.join(*a, *b);
            }
            _ => {
                let t = touched(op);
                for &i in &t {
                    sets.join(t[0], i);
                }
            }
        }
    }
    let mut out: Vec<(u32, Hunk)> = Vec::new();
    let mut at: BTreeMap<u32, usize> = BTreeMap::new();
    let mut put = |root: u32, op: Op| {
        let i = *at.entry(root).or_insert_with(|| {
            out.push((
                root,
                Hunk {
                    ids: Vec::new(),
                    ops: Vec::new(),
                },
            ));
            out.len() - 1
        });
        out[i].1.ids.extend(touched(&op));
        out[i].1.ops.push(op);
    };
    for op in ops {
        match op {
            Op::Remove {
                atoms,
                bonds,
                objects,
            } => {
                for (r, p) in split(&mut sets, &[atoms, bonds, objects]) {
                    let (atoms, bonds, objects) = (p[0].clone(), p[1].clone(), p[2].clone());
                    put(
                        r,
                        Op::Remove {
                            atoms,
                            bonds,
                            objects,
                        },
                    );
                }
            }
            Op::Move {
                atoms,
                objects,
                dx,
                dy,
            } => {
                for (r, p) in split(&mut sets, &[atoms, objects]) {
                    put(
                        r,
                        Op::Move {
                            atoms: p[0].clone(),
                            objects: p[1].clone(),
                            dx: *dx,
                            dy: *dy,
                        },
                    );
                }
            }
            _ => {
                let r = sets.find(touched(op)[0]);
                put(r, op.clone());
            }
        }
    }
    out.into_iter()
        .map(|(_, mut h)| {
            crate::m::sort(&mut h.ids);
            h.ids.dedup();
            h
        })
        .collect()
}

/// Whether the hunks can tell what an operation touches: an operation `diff` writes, with its ids named.
fn splittable(op: &Op) -> bool {
    match op {
        Op::Rotate { .. } | Op::Flip { .. } | Op::Ring { .. } => false,
        Op::AtomAdd { id, .. } | Op::BondAdd { id, .. } | Op::ObjAdd { id, .. } => id.is_some(),
        _ => !touched(op).is_empty(),
    }
}

/// The ids an operation of `diff` names (for a bond add: its own id and its two atoms).
pub(crate) fn touched(op: &Op) -> Vec<u32> {
    match op {
        Op::AtomAdd { id, .. } | Op::ObjAdd { id, .. } => id.iter().copied().collect(),
        Op::BondAdd { id, a, b, .. } => id.iter().copied().chain([*a, *b]).collect(),
        Op::AtomSet { id, .. }
        | Op::AtomData { id, .. }
        | Op::AtomAnn { id, .. }
        | Op::AtomCap { id, .. }
        | Op::BondSet { id, .. }
        | Op::ObjSet { id, .. } => vec![*id],
        Op::Colour {
            atoms,
            bonds,
            objects,
            ..
        } => atoms.iter().chain(bonds).chain(objects).copied().collect(),
        Op::DocLabels { .. } => Vec::new(),
        Op::Rotate { atoms, .. } | Op::Flip { atoms, .. } => atoms.clone(),
        Op::Move { atoms, objects, .. } => atoms.iter().chain(objects).copied().collect(),
        Op::Remove {
            atoms,
            bonds,
            objects,
        } => atoms.iter().chain(bonds).chain(objects).copied().collect(),
        Op::Ring { atom, bond, .. } => atom.iter().chain(bond).copied().collect(),
    }
}

/// The answer of the replay request `diff`: `{"ops":[...],"hunks":[{"ids":[...],"ops":[...]}]}`.
pub fn json(a: &Document, ops: &[Op]) -> String {
    let hunks: Vec<String> = (hunks(a, ops).iter())
        .map(|h| {
            let ids: Vec<String> = h.ids.iter().map(u32::to_string).collect();
            format!(
                "{{\"ids\":[{}],\"ops\":{}}}",
                ids.join(","),
                log_text(&h.ops)
            )
        })
        .collect();
    format!(
        "{{\"ops\":{},\"hunks\":[{}]}}",
        log_text(ops),
        hunks.join(",")
    )
}
