//! Derive a transformation product from its parent (tp-03): the copy of one structure with its exact coordinates and atom maps, a
//! patch written on the parent's ids applied to the copy, and an anchored, routed arrow from the parent to the copy. Nothing is
//! predicted: the caller names the edit. The result is one list of plain operations (`draw_edit/1`), so a log of it replays to the same
//! bytes; the command `tp:derive` runs it as one undo step.
//!
//! The patch is written on the parent: its atom and bond ids are the parent's and its coordinates are the parent's (the copy is a
//! translation of the parent, and the patch is translated with it). A new atom is named `-1`, `-2`, ... in the request (the k-th
//! `atom.add` of the patch); the command layer turns those names into `NEW + k` before the operations are read.

use crate::doc::{Atom, Document, ObjKind, MAX_MAP};
use crate::ops::{Axis, Op};
use crate::route::{self, Rect, Structures};
use crate::settings::Style;
use std::collections::{BTreeSet, HashMap};

/// Ids at or above this name a new atom of the patch (`NEW + k` is the k-th `atom.add`, k from 1): a real id never reaches it
/// (`doc::MAX_ID` is `1 << 31`).
pub const NEW: u32 = 1 << 31;

/// The most operations a patch may hold, and the most atoms a parent may have (a hostile size is refused, never slow).
pub const MAX_PATCH: usize = 10_000;
pub const MAX_ATOMS: usize = 5_000;

/// What the name of a new atom `-k` becomes in an operation read from JSON.
pub fn new_ref(k: u32) -> u32 {
    NEW + k
}

const PATCH: &str = "error.draw2.tpPatch";
const MAPS: &str = "error.draw2.tpMaps";
const SIZE: &str = "error.draw2.tpSize";
const MAP_DUP: &str = "error.draw2.tpMapDup";

/// The operations that derive a transformation product of the structure that holds `of`: the maps given to the parent when it has
/// none, the copy (atoms, bonds, labels, wedges; not data or annotation), the `patch` on the copy, and the routed arrow. `at` is the
/// centre of the copy's box; without it the copy stands one column to the right of the parent (`pathway.col_gap`), or below the
/// last structure in that place (`pathway.row_gap`).
pub fn derive(
    doc: &Document,
    style: &Style,
    of: u32,
    patch: &[Op],
    at: Option<(f64, f64)>,
) -> Result<Vec<Op>, &'static str> {
    if doc.atom(of).is_none() {
        return Err("error.draw2.noAtom");
    }
    if patch.len() > MAX_PATCH {
        return Err(PATCH);
    }
    let st = Structures::new(doc, style);
    let piece: Vec<u32> = (st.pieces().iter())
        .find(|p| p.contains(&of))
        .cloned()
        .ok_or("error.draw2.noAtom")?;
    if piece.len() > MAX_ATOMS {
        return Err(SIZE);
    }
    let member: BTreeSet<u32> = piece.iter().copied().collect();
    let parent_box = st.box_of(of).ok_or("error.draw2.noAtom")?;
    let mut ops: Vec<Op> = Vec::new();

    // atoms of the parent in pool order, its bonds, and the maps the parent still lacks (in id order, after the largest map)
    let atoms: Vec<&Atom> = doc
        .atoms
        .iter()
        .filter(|a| member.contains(&a.id))
        .collect();
    let bonds: Vec<&crate::doc::Bond> = (doc.bonds.iter())
        .filter(|b| member.contains(&b.a) && member.contains(&b.b))
        .collect();
    let mut next_map = doc.atoms.iter().map(|a| a.map).max().unwrap_or(0);
    let mut maps: HashMap<u32, u16> = atoms.iter().map(|a| (a.id, a.map)).collect();
    let mut lacking: Vec<u32> = atoms.iter().filter(|a| a.map == 0).map(|a| a.id).collect();
    lacking.sort_unstable();
    for id in lacking {
        next_map += 1;
        if next_map > MAX_MAP {
            return Err(MAPS);
        }
        maps.insert(id, next_map);
        ops.push(Op::AtomSet {
            id,
            el: None,
            chg: None,
            iso: None,
            rad: None,
            h: None,
            lab: None,
            map: Some(next_map),
        });
    }

    // where the copy goes
    let (dx, dy) = place(style, &st, parent_box, at);

    // ids: the copy's atoms, its bonds, the new atoms of the patch; then the patch's new bonds and the arrow as they come
    let mut next = doc.next;
    let mut take = || {
        let id = next;
        next += 1;
        id
    };
    let to_copy: HashMap<u32, u32> = atoms.iter().map(|a| (a.id, take())).collect();
    let bond_copy: HashMap<u32, u32> = bonds.iter().map(|b| (b.id, take())).collect();
    let added = patch
        .iter()
        .filter(|o| matches!(o, Op::AtomAdd { .. }))
        .count();
    let new_ids: Vec<u32> = (0..added).map(|_| take()).collect();

    for a in &atoms {
        ops.push(Op::AtomAdd {
            id: Some(to_copy[&a.id]),
            el: a.el.clone(),
            x: a.x + dx,
            y: a.y + dy,
            map: Some(maps[&a.id]).filter(|&m| m != 0),
        });
        let (chg, iso, rad) = (a.chg, a.iso, a.rad);
        if chg != 0 || iso != 0 || rad != 0 || a.h.is_some() || !a.lab.is_empty() {
            ops.push(Op::AtomSet {
                id: to_copy[&a.id],
                el: None,
                chg: (chg != 0).then_some(chg),
                iso: (iso != 0).then_some(iso),
                rad: (rad != 0).then_some(rad),
                h: a.h,
                lab: Some(a.lab.clone()).filter(|l| !l.is_empty()),
                map: None,
            });
        }
    }
    for b in &bonds {
        ops.push(Op::BondAdd {
            id: Some(bond_copy[&b.id]),
            a: to_copy[&b.a],
            b: to_copy[&b.b],
            o: b.o,
            s: b.s,
        });
    }

    // the patch: parent ids to copy ids, `NEW + k` to the new atoms, coordinates translated, new atoms mapped after the others
    let atom_ref = |id: u32| -> Result<u32, &'static str> {
        if id >= NEW {
            let k = (id - NEW) as usize;
            return (k >= 1 && k <= new_ids.len())
                .then(|| new_ids[k - 1])
                .ok_or(PATCH);
        }
        to_copy.get(&id).copied().ok_or(PATCH)
    };
    let atom_refs = |l: &[u32]| {
        l.iter()
            .map(|&i| atom_ref(i))
            .collect::<Result<Vec<_>, _>>()
    };
    let mut made = 0usize;
    // a map names one atom across the pathway: a number the patch gives must be free in the drawing (and among the patch's own)
    let mut used: BTreeSet<u16> = doc
        .atoms
        .iter()
        .map(|a| a.map)
        .filter(|&m| m != 0)
        .collect();
    used.extend(maps.values().copied().filter(|&m| m != 0));
    let own: HashMap<u32, u16> = to_copy.iter().map(|(p, c)| (*c, maps[p])).collect();
    let give = |used: &mut BTreeSet<u16>, id: u32, m: u16| -> Result<(), &'static str> {
        if m == 0 || own.get(&id) == Some(&m) || used.insert(m) {
            Ok(())
        } else {
            Err(MAP_DUP)
        }
    };
    for op in patch {
        ops.push(match op {
            Op::AtomAdd {
                id: None,
                el,
                x,
                y,
                map,
            } => {
                let id = new_ids[made];
                made += 1;
                let map = match map {
                    Some(m) => {
                        give(&mut used, id, *m)?;
                        Some(*m)
                    }
                    None => {
                        next_map += 1;
                        while used.contains(&next_map) {
                            next_map += 1;
                        }
                        if next_map > MAX_MAP {
                            return Err(MAPS);
                        }
                        used.insert(next_map);
                        Some(next_map)
                    }
                };
                Op::AtomAdd {
                    id: Some(id),
                    el: el.clone(),
                    x: x + dx,
                    y: y + dy,
                    map,
                }
            }
            Op::BondAdd {
                id: None,
                a,
                b,
                o,
                s,
            } => Op::BondAdd {
                id: Some(take()),
                a: atom_ref(*a)?,
                b: atom_ref(*b)?,
                o: *o,
                s: *s,
            },
            Op::AtomSet {
                id,
                el,
                chg,
                iso,
                rad,
                h,
                lab,
                map,
            } => {
                let id = atom_ref(*id)?;
                if let Some(m) = map {
                    give(&mut used, id, *m)?;
                }
                Op::AtomSet {
                    id,
                    el: el.clone(),
                    chg: *chg,
                    iso: *iso,
                    rad: *rad,
                    h: *h,
                    lab: lab.clone(),
                    map: *map,
                }
            }
            Op::BondSet { id, o, s, rev } => Op::BondSet {
                id: *bond_copy.get(id).ok_or(PATCH)?,
                o: *o,
                s: *s,
                rev: *rev,
            },
            Op::AtomData { id, data } => Op::AtomData {
                id: atom_ref(*id)?,
                data: data.clone(),
            },
            Op::AtomAnn { id, ann } => Op::AtomAnn {
                id: atom_ref(*id)?,
                ann: ann.clone(),
            },
            Op::Move {
                atoms,
                objects,
                dx,
                dy,
            } if objects.is_empty() => Op::Move {
                atoms: atom_refs(atoms)?,
                objects: Vec::new(),
                dx: *dx,
                dy: *dy,
            },
            Op::Rotate { atoms, deg, x, y } => Op::Rotate {
                atoms: atom_refs(atoms)?,
                deg: *deg,
                x: x + dx,
                y: y + dy,
            },
            Op::Flip { atoms, axis, at } => Op::Flip {
                atoms: atom_refs(atoms)?,
                axis: *axis,
                at: at + if *axis == Axis::H { dx } else { dy },
            },
            Op::Remove {
                atoms,
                bonds,
                objects,
            } if objects.is_empty() => Op::Remove {
                atoms: atom_refs(atoms)?,
                bonds: (bonds.iter())
                    .map(|b| bond_copy.get(b).copied().ok_or(PATCH))
                    .collect::<Result<_, _>>()?,
                objects: Vec::new(),
            },
            _ => return Err(PATCH),
        });
    }

    // the copy as the patch leaves it must be valid for the chemistry layer. Everything is checked on a document of the parent
    // alone (the cost does not grow with the drawing): the copy, the patch and the arrow are applied to it
    let parent = Document {
        atoms: atoms.iter().map(|a| (*a).clone()).collect(),
        bonds: bonds.iter().map(|b| (*b).clone()).collect(),
        next: doc.next,
        ..Document::default()
    };
    let mut after = parent.clone();
    after.apply(&ops).map_err(|_| PATCH)?;
    let copied: BTreeSet<u32> = to_copy.values().chain(&new_ids).copied().collect();
    let mine: Vec<u32> = (after.atoms.iter())
        .map(|a| a.id)
        .filter(|i| copied.contains(i))
        .collect();
    let anchor = (to_copy
        .values()
        .copied()
        .filter(|&i| after.atom(i).is_some()))
    .min()
    .ok_or(PATCH)?;
    if overvalent(&after, &mine, &parent, &to_copy).is_some() || !record_ok(&after, &copied) {
        return Err(PATCH);
    }

    // the arrow, from the smallest atom of the parent to the smallest of the copy, then routed (between the two boxes, with the
    // structures of the small document: the parent and the copy)
    let id = take();
    let from = *piece.iter().min().unwrap_or(&of);
    let pc = centre(parent_box);
    let arrow = Op::ObjAdd {
        id: Some(id),
        t: ObjKind::Arrow,
        x: pc.0,
        y: pc.1,
        x2: pc.0 + dx,
        y2: pc.1 + dy,
        text: String::new(),
        from,
        to: anchor,
    };
    after
        .apply(std::slice::from_ref(&arrow))
        .map_err(|_| PATCH)?;
    let st_after = Structures::new(&after, style);
    ops.push(arrow);
    ops.extend(route::route_in(&after, id, &st_after, style));
    Ok(ops)
}

/// Whether the copy (the atoms `ids` of `d` and the bonds between them) is a record `mzlab-mol` accepts (`Mol::validate`).
fn record_ok(d: &Document, ids: &BTreeSet<u32>) -> bool {
    let part = Document {
        atoms: (d.atoms.iter())
            .filter(|a| ids.contains(&a.id))
            .cloned()
            .collect(),
        bonds: (d.bonds.iter())
            .filter(|b| ids.contains(&b.a) && ids.contains(&b.b))
            .cloned()
            .collect(),
        next: d.next,
        ..Document::default()
    };
    part.to_mol_stereo().validate().is_ok()
}

fn centre(b: Rect) -> (f64, f64) {
    ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0)
}

/// The translation of the copy. With `at`, the centre of its box goes there. Without: one column to the right of the parent, at the
/// parent's height; where that is taken, below the lowest box in the way (the TPs of the parent stand in that column), `row_gap` apart.
fn place(style: &Style, st: &Structures, b: Rect, at: Option<(f64, f64)>) -> (f64, f64) {
    let (w, h) = (b[2] - b[0], b[3] - b[1]);
    if let Some((x, y)) = at {
        let c = centre(b);
        return (x - c.0, y - c.1);
    }
    let x0 = b[2] + style.pathway_col_gap;
    let mut y0 = b[1];
    // the boxes in the way of the column, highest first: one pass moves the copy below each box it meets, so the cost is that of
    // the sort (a box passed stays above the copy, and one not met yet is lower than the window or in it)
    let eps = 1e-9;
    let mut column: Vec<Rect> = (st.pieces().iter())
        .filter_map(|p| p.first().and_then(|&a| st.box_of(a)))
        .filter(|r| r[0] < x0 + w - eps && x0 < r[2] - eps)
        .collect();
    column.sort_by(|p, q| q[3].total_cmp(&p[3]));
    for r in column {
        if r[1] < y0 + h - eps && y0 < r[3] - eps {
            y0 = r[1] - style.pathway_row_gap - h;
        }
    }
    (x0 - b[0], y0 - b[1])
}

/// The first atom of `ids` whose bond orders (and radical, explicit hydrogens) exceed the largest allowed valence of its element and
/// charge, where its parent atom (if any) was not already over: a parent that came over from a file is not blamed on the patch.
fn overvalent(
    after: &Document,
    ids: &[u32],
    before: &Document,
    to_copy: &HashMap<u32, u32>,
) -> Option<u32> {
    let mut sum: HashMap<u32, u32> = HashMap::new();
    for b in &after.bonds {
        *sum.entry(b.a).or_default() += b.o as u32;
        *sum.entry(b.b).or_default() += b.o as u32;
    }
    let used = |a: &Atom, s: u32| s + a.rad as u32 + u32::from(a.h.unwrap_or(0));
    let top = |a: &Atom| {
        mzlab_mol::allowed_valences(&a.el, a.chg as i32)
            .into_iter()
            .max()
    };
    let parent_of: HashMap<u32, u32> = to_copy.iter().map(|(&p, &c)| (c, p)).collect();
    ids.iter().copied().find(|&i| {
        let Some(a) = after.atom(i).filter(|a| a.lab.is_empty()) else {
            return false;
        };
        let Some(max) = top(a) else { return false };
        if used(a, sum.get(&i).copied().unwrap_or(0)) <= max {
            return false;
        }
        // over now: blame the patch unless the parent atom was over by the same count
        match parent_of.get(&i).and_then(|&p| before.atom(p)) {
            Some(p) => used(p, before.order_sum(p.id)) <= top(p).unwrap_or(u32::MAX),
            None => true,
        }
    })
}

// ---------------------------------------------------------------------------------------------------------------------------
// The diff lens (tp-04): what changed from the parent, computed from the atom maps of the pathway.

/// The most atoms a structure may have to be compared by the lens, the most atoms (both ends, summed over the arrows) one request
/// compares (an arrow past either is skipped, `over cap`), and the most atoms a drawing may have for the lens at all (above it the
/// whole answer is `error.draw2.lensSize`): a hostile size is refused, never slow.
pub const MAX_LENS_ATOMS: usize = 5_000;
pub const MAX_LENS_WORK: usize = 2_000_000;
pub const MAX_LENS_DOC: usize = 6_000;

const LENS_SIZE: &str = "error.draw2.lensSize";

/// The lens of one anchored arrow, from structure P (where it leaves) to structure T (where it arrives), compared by map number.
/// Every list is ascending. `added` and `changed` hold atom ids of T, `added_bonds` and `changed_bonds` bond ids of T, `cleaved`
/// holds the two T atom ids (smaller first) of each bond of P that T no longer has, `lost` the maps of P that T lacks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lens {
    pub arrow: u32,
    /// The atoms of T whose map P does not have.
    pub added: Vec<u32>,
    /// The bonds of T with an added end, and the bonds between two atoms both structures have that P did not bond.
    pub added_bonds: Vec<u32>,
    /// The atoms in both whose element, label, charge, isotope, radical or hydrogen count differ.
    pub changed: Vec<u32>,
    /// The bonds in both (the same two maps) whose order differs.
    pub changed_bonds: Vec<u32>,
    /// The bonds of P between two maps that T has but does not bond, as T atom ids.
    pub cleaved: Vec<(u32, u32)>,
    /// The maps of P that T lacks.
    pub lost: Vec<u16>,
    /// The Hill formula of P (with its net charge, as the outline writes it) and the signed element counts from P to T (`delta`,
    /// empty for equal formulas). `delta` is `None` where `steplabel::pair_delta` has no honest text (a free-text label, or equal
    /// counts with a different net charge): the same rule as `pathway:label`, one implementation.
    pub from: String,
    pub delta: Option<String>,
    /// The first atom (pool order) of T: the `anchor` the outline names that structure by.
    pub anchor: u32,
}

/// A pair the lens refuses: the atoms of P and T that have no map, or share one with another atom of the same structure, in a pair
/// where some atom does have a map (a pair with no map at all is not an inconsistency: it has no lens and no finding).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unmapped {
    pub arrow: u32,
    pub atoms: Vec<u32>,
}

/// An anchored arrow the lens did not compare, and why: `no maps` (neither structure has a map), `unmapped` (a partly mapped pair,
/// also the finding `std.tp.unmapped`) or `over cap` (a structure above `MAX_LENS_ATOMS`, or `MAX_LENS_WORK` already spent).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skipped {
    pub arrow: u32,
    pub why: &'static str,
}

/// The lens of every anchored arrow of a drawing: the lenses by arrow id, the arrows left out with the reason (nothing is dropped
/// in silence), and the pairs that cannot be compared (never a partial lens: such an arrow has no entry in `lenses`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lenses {
    pub lenses: Vec<Lens>,
    pub skipped: Vec<Skipped>,
    pub unmapped: Vec<Unmapped>,
}

/// Why the lens of one arrow is not there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoLens {
    /// Not an arrow, or an end is free, or both ends are in one structure.
    NotAnchored,
    /// Neither structure has a map at all: nothing to compare, and nothing wrong with the drawing (no finding).
    NoMaps,
    /// A pair that is partly mapped: atoms without a map or with a map another atom of their structure has (`std.tp.unmapped`).
    Unmapped(Vec<u32>),
    /// A structure above `MAX_LENS_ATOMS`.
    Size,
}

struct Ctx<'a> {
    doc: &'a Document,
    pieces: Vec<Vec<u32>>,
    piece_of: HashMap<u32, usize>,
    pos: HashMap<u32, usize>,
    hydrogens: Vec<u32>,
    bonds_of: Vec<Vec<usize>>,
    /// The element counts of the structures the arrows end in (`steplabel::Formulas`, shared with `pathway:label`).
    formulas: crate::steplabel::Formulas,
}

impl<'a> Ctx<'a> {
    fn new(doc: &'a Document, wanted: &[u32]) -> Ctx<'a> {
        let pieces = crate::layout::pieces(doc);
        let mut piece_of = HashMap::with_capacity(doc.atoms.len());
        for (k, p) in pieces.iter().enumerate() {
            for &a in p {
                piece_of.insert(a, k);
            }
        }
        let pos: HashMap<u32, usize> = (doc.atoms.iter().enumerate())
            .map(|(i, a)| (a.id, i))
            .collect();
        let mut bonds_of = vec![Vec::new(); pieces.len()];
        for (i, b) in doc.bonds.iter().enumerate() {
            if let (Some(&k), Some(_)) = (piece_of.get(&b.a), piece_of.get(&b.b)) {
                bonds_of[k].push(i);
            }
        }
        Ctx {
            doc,
            pieces,
            piece_of,
            pos,
            hydrogens: doc.all_hydrogens(),
            bonds_of,
            formulas: crate::steplabel::Formulas::new(doc, wanted),
        }
    }

    fn atom(&self, id: u32) -> &Atom {
        &self.doc.atoms[self.pos[&id]]
    }

    /// The map table of a structure, or the atoms that stop it from being one: without a map, or sharing one.
    fn maps(&self, piece: usize) -> Result<HashMap<u16, u32>, Vec<u32>> {
        let mut table: HashMap<u16, u32> = HashMap::new();
        let mut bad: Vec<u32> = Vec::new();
        for &id in &self.pieces[piece] {
            let a = self.atom(id);
            if a.map == 0 {
                bad.push(id);
            } else if let Some(&first) = table.get(&a.map) {
                bad.push(id);
                bad.push(first);
            } else {
                table.insert(a.map, id);
            }
        }
        if bad.is_empty() {
            Ok(table)
        } else {
            Err(bad)
        }
    }

    fn lens(&self, arrow: &crate::doc::Obj) -> Result<Lens, NoLens> {
        if arrow.t != ObjKind::Arrow || arrow.from == 0 || arrow.to == 0 {
            return Err(NoLens::NotAnchored);
        }
        let (Some(&p), Some(&t)) = (self.piece_of.get(&arrow.from), self.piece_of.get(&arrow.to))
        else {
            return Err(NoLens::NotAnchored);
        };
        if p == t {
            return Err(NoLens::NotAnchored);
        }
        if self.pieces[p].len() > MAX_LENS_ATOMS || self.pieces[t].len() > MAX_LENS_ATOMS {
            return Err(NoLens::Size);
        }
        if (self.pieces[p].iter().chain(&self.pieces[t])).all(|&a| self.atom(a).map == 0) {
            return Err(NoLens::NoMaps);
        }
        let (pm, tm) = (self.maps(p), self.maps(t));
        let (pm, tm) = match (pm, tm) {
            (Ok(a), Ok(b)) => (a, b),
            (a, b) => {
                let mut bad: Vec<u32> = a.err().into_iter().chain(b.err()).flatten().collect();
                bad.sort_unstable();
                bad.dedup();
                return Err(NoLens::Unmapped(bad));
            }
        };
        let mut l = Lens {
            arrow: arrow.id,
            ..Lens::default()
        };
        for &id in &self.pieces[t] {
            let a = self.atom(id);
            match pm.get(&a.map) {
                None => l.added.push(id),
                Some(&q) => {
                    let o = self.atom(q);
                    let h = |x: u32| self.hydrogens[self.pos[&x]];
                    if (o.el.as_str(), o.lab.as_str(), o.chg, o.iso, o.rad, h(q))
                        != (a.el.as_str(), a.lab.as_str(), a.chg, a.iso, a.rad, h(id))
                    {
                        l.changed.push(id);
                    }
                }
            }
        }
        let key = |x: u16, y: u16| (x.min(y), x.max(y));
        let map_of = |id: u32| self.atom(id).map;
        let mut p_bonds: HashMap<(u16, u16), u8> = HashMap::new();
        for &i in &self.bonds_of[p] {
            let b = &self.doc.bonds[i];
            p_bonds.insert(key(map_of(b.a), map_of(b.b)), b.o);
        }
        let mut t_bonds: std::collections::HashSet<(u16, u16)> = Default::default();
        for &i in &self.bonds_of[t] {
            let b = &self.doc.bonds[i];
            let k = key(map_of(b.a), map_of(b.b));
            t_bonds.insert(k);
            match p_bonds.get(&k) {
                None => l.added_bonds.push(b.id),
                Some(&o) if o != b.o => l.changed_bonds.push(b.id),
                Some(_) => {}
            }
        }
        for &(x, y) in p_bonds.keys() {
            if t_bonds.contains(&(x, y)) {
                continue;
            }
            if let (Some(&a), Some(&b)) = (tm.get(&x), tm.get(&y)) {
                l.cleaved.push((a.min(b), a.max(b)));
            }
        }
        l.lost = pm.keys().copied().filter(|m| !tm.contains_key(m)).collect();
        for v in [
            &mut l.added,
            &mut l.added_bonds,
            &mut l.changed,
            &mut l.changed_bonds,
        ] {
            v.sort_unstable();
        }
        l.cleaved.sort_unstable();
        l.cleaved.dedup();
        l.lost.sort_unstable();
        // the formula of P and the change of the counts from P to T: the counts `pathway:label` uses
        l.anchor = self.pieces[t][0];
        let (Some(rp), Some(rt)) = (
            self.formulas.root_of(arrow.from),
            self.formulas.root_of(arrow.to),
        ) else {
            return Ok(l);
        };
        let (pa, ta) = (&self.formulas.acc[&rp], &self.formulas.acc[&rt]);
        l.delta = crate::steplabel::pair_delta(pa, ta);
        if l.delta.is_some() {
            let mut f: Vec<(String, u32)> = pa
                .counts
                .iter()
                .map(|(k, v)| (k.clone(), *v as u32))
                .collect();
            l.from = crate::outline::hill(&mut f, pa.charge as i32);
        }
        Ok(l)
    }
}

/// The lens of the arrow `arrow` (an object id). Builds the tables of the whole drawing: for every arrow use `lenses`.
pub fn lens(doc: &Document, arrow: u32) -> Result<Lens, NoLens> {
    let Some(o) = doc.objects.iter().find(|o| o.id == arrow) else {
        return Err(NoLens::NotAnchored);
    };
    Ctx::new(doc, &[o.from, o.to]).lens(o)
}

/// The lens of every anchored arrow, arrows by id; a pair that cannot be compared is listed in `unmapped`, not drawn. A structure
/// above `MAX_LENS_ATOMS`, or more than `MAX_LENS_WORK` atoms compared in all, is `error.draw2.lensSize`. A drawing without an
/// anchored arrow costs one pass over its arrows.
pub fn lenses(doc: &Document) -> Result<Lenses, &'static str> {
    let mut arrows: Vec<&crate::doc::Obj> = (doc.objects.iter())
        .filter(|o| o.t == ObjKind::Arrow && o.from != 0 && o.to != 0)
        .collect();
    let mut out = Lenses::default();
    if doc.atoms.len() > MAX_LENS_DOC {
        return Err(LENS_SIZE);
    }
    if arrows.is_empty() {
        return Ok(out);
    }
    arrows.sort_by_key(|o| o.id);
    let wanted: Vec<u32> = arrows.iter().flat_map(|o| [o.from, o.to]).collect();
    let cx = Ctx::new(doc, &wanted);
    let mut work = 0usize;
    for o in arrows {
        let sizes = match (cx.piece_of.get(&o.from), cx.piece_of.get(&o.to)) {
            (Some(&p), Some(&t)) if p != t => Some(cx.pieces[p].len() + cx.pieces[t].len()),
            _ => None,
        };
        let spent = sizes.is_some_and(|n| {
            work += n;
            work > MAX_LENS_WORK
        });
        let why = |why| out_skip(o.id, why);
        match if spent { Err(NoLens::Size) } else { cx.lens(o) } {
            Ok(l) => out.lenses.push(l),
            Err(NoLens::NotAnchored) => {}
            Err(NoLens::NoMaps) => out.skipped.push(why("no maps")),
            Err(NoLens::Size) => out.skipped.push(why("over cap")),
            Err(NoLens::Unmapped(atoms)) => {
                out.skipped.push(why("unmapped"));
                out.unmapped.push(Unmapped { arrow: o.id, atoms });
            }
        }
    }
    Ok(out)
}

fn out_skip(arrow: u32, why: &'static str) -> Skipped {
    Skipped { arrow, why }
}

impl Lenses {
    /// `lens/1`: the machine form, keys in a fixed order, ids ascending, arrows by id.
    pub fn to_json(&self) -> String {
        let list = |v: &[u32]| {
            let s: Vec<String> = v.iter().map(|x| x.to_string()).collect();
            format!("[{}]", s.join(","))
        };
        let arrows: Vec<String> = (self.lenses.iter())
            .map(|l| {
                let cleaved: Vec<String> = (l.cleaved.iter())
                    .map(|(a, b)| format!("[{a},{b}]"))
                    .collect();
                let lost: Vec<String> = l.lost.iter().map(|m| m.to_string()).collect();
                format!(
                    r#"{{"arrow":{},"added":{},"added_bonds":{},"changed":{},"changed_bonds":{},"cleaved":[{}],"lost":[{}],"delta":{}}}"#,
                    l.arrow,
                    list(&l.added),
                    list(&l.added_bonds),
                    list(&l.changed),
                    list(&l.changed_bonds),
                    cleaved.join(","),
                    lost.join(","),
                    l.delta
                        .as_deref()
                        .map_or("null".to_string(), |d| serde_json::Value::from(d).to_string())
                )
            })
            .collect();
        let skipped: Vec<String> = (self.skipped.iter())
            .map(|k| format!(r#"{{"arrow":{},"why":"{}"}}"#, k.arrow, k.why))
            .collect();
        format!(
            r#"{{"format":"lens/1","arrows":[{}],"skipped":[{}]}}"#,
            arrows.join(","),
            skipped.join(",")
        )
    }

    /// The `std/1` findings of the pairs the lens refuses: `std.tp.unmapped`, one per arrow (the arrow, and the atoms of its two
    /// structures without a map or sharing one).
    pub fn findings(&self) -> Vec<crate::conform::Finding> {
        (self.unmapped.iter())
            .map(|u| {
                let mut f = crate::conform::Finding::new("std.tp.unmapped", &u.atoms, &[]);
                f.objects = vec![u.arrow];
                f
            })
            .collect()
    }
}

/// What the page needs of the lens when it is on, from one pass: the words of each product by its anchor (the first atom of the
/// structure, as the outline names it) in a table, and the note when the lens is not showing everything.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LensState {
    lines: HashMap<u32, Line>,
    /// The i18n key of the note the page shows, and the number it is about: `draw2.notice.lensSize` (the drawing is over the cap),
    /// `draw2.notice.lensSkipped` (arrows left out).
    pub note: Option<(&'static str, usize)>,
}

/// The outline words of one product: the formula it comes from, the change, the bonds cleaved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub from: String,
    pub delta: String,
    pub cleaved: usize,
}

impl LensState {
    /// The lens of a drawing for the page: one pass. Not called when the lens is off.
    pub fn of(doc: &Document) -> LensState {
        match lenses(doc) {
            Err(_) => LensState {
                note: Some(("draw2.notice.lensSize", doc.atoms.len())),
                ..LensState::default()
            },
            Ok(l) => LensState::from_lenses(&l),
        }
    }

    /// The table of the words of each product, from the lenses of a drawing.
    pub fn from_lenses(l: &Lenses) -> LensState {
        let mut st = LensState::default();
        for x in &l.lenses {
            if let Some(d) = &x.delta {
                st.lines.insert(
                    x.anchor,
                    Line {
                        from: x.from.clone(),
                        delta: d.clone(),
                        cleaved: x.cleaved.len(),
                    },
                );
            }
        }
        if !l.skipped.is_empty() {
            st.note = Some(("draw2.notice.lensSkipped", l.skipped.len()));
        }
        st
    }

    /// The words of the product whose first atom is `anchor`.
    pub fn line(&self, anchor: u32) -> Option<&Line> {
        self.lines.get(&anchor)
    }
}
