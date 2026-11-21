//! The stereo guard: a geometry edit (`move`, `rotate`, `flip`) can change a configuration without any stereo edit, because the
//! wedges are the drawing and the configuration is derived from them. This module writes the canonical isomeric SMILES of chosen
//! pieces as `mzlab-mol` writes it from the wedges as drawn (the path of `save("smiles")`), so a step can be compared before and
//! after. The editor says a change in the live line and never blocks it; the `expect` arm `{"stereo":"kept"}` of `draw-result/1`
//! compares the start of a request with its end.

use crate::doc::Document;
use crate::ops::Op;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A piece: the ids of its atoms, ascending (a connected set of atoms; an atom alone is a piece).
pub type Piece = Vec<u32>;

/// The signatures computed since the start of the process: lets a test prove that no signature is written during a drag.
static COMPUTED: AtomicUsize = AtomicUsize::new(0);

/// How many piece signatures were computed so far.
pub fn computed() -> usize {
    COMPUTED.load(Ordering::Relaxed)
}

/// The connected pieces of the atoms of a document, each with ascending ids, ordered by their smallest id.
pub fn pieces(doc: &Document) -> Vec<Piece> {
    let n = doc.atoms.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for b in &doc.bonds {
        if let (Some(i), Some(j)) = (doc.atom_index(b.a), doc.atom_index(b.b)) {
            let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
            parent[ri] = rj;
        }
    }
    let mut groups: Vec<Piece> = Vec::new();
    let mut slot = vec![usize::MAX; n];
    for i in 0..n {
        let r = root(&mut parent, i);
        if slot[r] == usize::MAX {
            slot[r] = groups.len();
            groups.push(Vec::new());
        }
        groups[slot[r]].push(doc.atoms[i].id);
    }
    for g in &mut groups {
        g.sort_unstable();
    }
    groups.sort_by_key(|g| g[0]);
    groups
}

/// The atoms at the narrow end of a wedge (solid, hashed or wavy) of the piece: the atoms whose configuration the marks draw.
pub fn centres(doc: &Document, piece: &[u32]) -> Vec<u32> {
    let set: BTreeSet<u32> = piece.iter().copied().collect();
    let mut out: BTreeSet<u32> = BTreeSet::new();
    for b in &doc.bonds {
        if b.s != 0 && set.contains(&b.a) && set.contains(&b.b) {
            out.insert(b.a);
        }
    }
    out.into_iter().collect()
}

/// Whether the piece holds a wedge mark.
fn has_wedge(doc: &Document, set: &BTreeSet<u32>) -> bool {
    doc.bonds.iter().any(|b| b.s != 0 && set.contains(&b.a))
}

/// The piece as a drawing of its own.
fn sub_document(doc: &Document, set: &BTreeSet<u32>) -> Document {
    let mut d = Document {
        next: doc.next,
        ..Document::default()
    };
    d.atoms = doc
        .atoms
        .iter()
        .filter(|a| set.contains(&a.id))
        .cloned()
        .collect();
    d.bonds = doc
        .bonds
        .iter()
        .filter(|b| set.contains(&b.a) && set.contains(&b.b))
        .cloned()
        .collect();
    d
}

/// The canonical isomeric SMILES of each of `pieces`, as written from the wedges as drawn. A piece with no wedge has no
/// configuration to guard and is left out; so is a piece the writer refuses. Order of `pieces` is kept.
pub fn signature(doc: &Document, pieces: &[Piece]) -> Vec<(Piece, String)> {
    let mut out = Vec::new();
    for p in pieces {
        let set: BTreeSet<u32> = p.iter().copied().collect();
        if !has_wedge(doc, &set) {
            continue;
        }
        COMPUTED.fetch_add(1, Ordering::Relaxed);
        // the canonical form, so that two drawings of one configuration (a mirrored meso compound) give the same text
        let mol = sub_document(doc, &set).to_mol_stereo().canonical();
        if let Ok((text, _)) = mol.to_smiles_with_order() {
            out.push((p.clone(), text));
        }
    }
    out
}

/// Whether every operation only moves things: `move`, `rotate` and `flip` (and nothing else).
pub fn geometry_only(ops: &[Op]) -> bool {
    !ops.is_empty()
        && ops
            .iter()
            .all(|o| matches!(o, Op::Move { .. } | Op::Rotate { .. } | Op::Flip { .. }))
}

/// The atoms the geometry operations act on.
fn touched(ops: &[Op]) -> BTreeSet<u32> {
    let mut s = BTreeSet::new();
    for o in ops {
        match o {
            Op::Move { atoms, .. } | Op::Rotate { atoms, .. } | Op::Flip { atoms, .. } => {
                s.extend(atoms.iter().copied());
            }
            _ => {}
        }
    }
    s
}

/// What to compare before a geometry step: the signatures of the pieces it touches that hold a wedge. `None` when the step is
/// not only geometry or touches no piece with a wedge (nothing is computed then).
pub fn before(doc: &Document, ops: &[Op]) -> Option<Vec<(Piece, String)>> {
    if !geometry_only(ops) {
        return None;
    }
    let hit = touched(ops);
    if hit.is_empty() {
        return None;
    }
    let chosen: Vec<Piece> = pieces(doc)
        .into_iter()
        .filter(|p| p.iter().any(|a| hit.contains(a)))
        .collect();
    let sig = signature(doc, &chosen);
    (!sig.is_empty()).then_some(sig)
}

/// The centres of the pieces whose signature in `then` differs from the one now, ascending; empty when nothing changed.
pub fn changed(doc: &Document, then: &[(Piece, String)]) -> Vec<u32> {
    let pcs: Vec<Piece> = then.iter().map(|(p, _)| p.clone()).collect();
    let now = signature(doc, &pcs);
    let mut atoms: BTreeSet<u32> = BTreeSet::new();
    for (p, was) in then {
        let same = now
            .iter()
            .find(|(q, _)| q == p)
            .is_some_and(|(_, s)| s == was);
        if !same {
            atoms.extend(centres(doc, p));
        }
    }
    atoms.into_iter().collect()
}

/// The pieces of the start that are still the same set of atoms at the end, with a signature that differs: the verdict of
/// `{"stereo":"kept"}`. `(centres, was, now)` per piece, in piece order. A piece whose atoms changed is another structure and is
/// not compared (an edit of the structure changes the SMILES legitimately).
pub fn differing(start: &[(Piece, String)], end: &Document) -> Vec<(Vec<u32>, String, String)> {
    let pcs: Vec<Piece> = start.iter().map(|(p, _)| p.clone()).collect();
    let now_pieces = pieces(end);
    let same_sets: Vec<Piece> = pcs.into_iter().filter(|p| now_pieces.contains(p)).collect();
    let now = signature(end, &same_sets);
    let mut out = Vec::new();
    for (p, was) in start {
        if !same_sets.contains(p) {
            continue;
        }
        let got = now
            .iter()
            .find(|(q, _)| q == p)
            .map_or("", |(_, s)| s.as_str());
        if got != was {
            out.push((centres(end, p), was.clone(), got.to_string()));
        }
    }
    out
}
