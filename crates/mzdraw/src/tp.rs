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
                    Some(m) => Some(*m),
                    None => {
                        next_map += 1;
                        if next_map > MAX_MAP {
                            return Err(MAPS);
                        }
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
            } => Op::AtomSet {
                id: atom_ref(*id)?,
                el: el.clone(),
                chg: *chg,
                iso: *iso,
                rad: *rad,
                h: *h,
                lab: lab.clone(),
                map: *map,
            },
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

    // the copy as the patch leaves it must be valid for the chemistry layer
    let mut after = doc.clone();
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
    if overvalent(&after, &mine, doc, &to_copy).is_some() {
        return Err(PATCH);
    }

    // the arrow, from the smallest atom of the parent to the smallest of the copy, then routed
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
    let boxes: Vec<Rect> = (st.pieces().iter())
        .filter_map(|p| p.first().and_then(|&a| st.box_of(a)))
        .collect();
    let eps = 1e-9;
    let taken = |y0: f64| -> Option<Rect> {
        (boxes.iter().copied())
            .filter(|r| {
                r[0] < x0 + w - eps && x0 < r[2] - eps && r[1] < y0 + h - eps && y0 < r[3] - eps
            })
            .min_by(|p, q| p[1].total_cmp(&q[1]))
    };
    for _ in 0..=boxes.len() {
        match taken(y0) {
            None => break,
            // below the lowest box in the way
            Some(r) => y0 = r[1] - style.pathway_row_gap - h,
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
