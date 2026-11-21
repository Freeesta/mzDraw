//! The stereo guard: a geometry edit (`move`, `rotate`, `flip`) can change a configuration without any stereo edit, because the
//! wedges are the drawing and the configuration is derived from them. A configuration is kept per stereo element, not per piece:
//! a stereocentre (the narrow end of a wedge) with its neighbours, or a double bond with the neighbours of its two ends. The
//! signature of an element is the canonical isomeric SMILES `mzlab-mol` writes from the wedges and the coordinates as drawn, of the
//! element alone with its neighbours made distinct, so it says "this centre is mirrored" whatever else happens to the piece (a
//! substituent added, a piece merged, an atom removed).
//!
//! The editor compares the elements near a geometry step before and after it and says the change in the live line (never blocks
//! it); the `expect` arm `{"stereo":"kept"}` of `draw-result/1` compares every element at the start of a request with the end. A
//! change that leaves the piece the same molecule (the mirror image of a meso compound) is not a change.

use crate::doc::{Atom, Bond, Document};
use crate::ops::Op;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A piece: the ids of its atoms, ascending (a connected set of atoms; an atom alone is a piece).
pub type Piece = Vec<u32>;

/// The element and piece signatures computed since the start of the process (one relaxed atomic add each, nothing else): lets a
/// test prove that no signature is written while a drag is under way and that a rigid edit computes none.
static COMPUTED: AtomicUsize = AtomicUsize::new(0);

/// How many signatures were computed so far.
pub fn computed() -> usize {
    COMPUTED.load(Ordering::Relaxed)
}

/// What is looked at: a stereocentre (the narrow end of a wedge) or a double bond (its two atoms, ascending).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Centre(u32),
    Double(u32, u32),
}

impl Key {
    fn core(self) -> Vec<u32> {
        match self {
            Key::Centre(c) => vec![c],
            Key::Double(a, b) => vec![a, b],
        }
    }
}

/// The document seen by id: the position of each atom (ids sorted, searched by bisection) and the bonds at each atom.
struct Index<'a> {
    doc: &'a Document,
    ids: Vec<(u32, usize)>,
    bonds_at: Vec<Vec<usize>>,
}

impl<'a> Index<'a> {
    fn new(doc: &'a Document) -> Index<'a> {
        let mut ids: Vec<(u32, usize)> = doc
            .atoms
            .iter()
            .enumerate()
            .map(|(i, a)| (a.id, i))
            .collect();
        ids.sort_unstable();
        let mut ix = Index {
            doc,
            ids,
            bonds_at: vec![Vec::new(); doc.atoms.len()],
        };
        for (i, b) in doc.bonds.iter().enumerate() {
            if let (Some(p), Some(q)) = (ix.pos(b.a), ix.pos(b.b)) {
                ix.bonds_at[p].push(i);
                ix.bonds_at[q].push(i);
            }
        }
        ix
    }

    /// The position of an atom in the document.
    fn pos(&self, id: u32) -> Option<usize> {
        self.ids
            .binary_search_by_key(&id, |&(i, _)| i)
            .ok()
            .map(|k| self.ids[k].1)
    }

    fn atom(&self, id: u32) -> Option<&'a Atom> {
        self.pos(id).map(|i| &self.doc.atoms[i])
    }

    fn bonds(&self, id: u32) -> &[usize] {
        self.pos(id).map_or(&[], |p| self.bonds_at[p].as_slice())
    }

    /// The neighbours of an atom, ascending.
    fn neighbours(&self, id: u32) -> BTreeSet<u32> {
        let mut out = BTreeSet::new();
        for &i in self.bonds(id) {
            let b = &self.doc.bonds[i];
            out.insert(if b.a == id { b.b } else { b.a });
        }
        out
    }

    /// The connected pieces, each with ascending ids, ordered by the smallest id, and the piece of each atom (by its position).
    fn piece_index(&self) -> (Vec<Piece>, Vec<usize>) {
        let n = self.doc.atoms.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn root(p: &mut [usize], mut i: usize) -> usize {
            while p[i] != i {
                p[i] = p[p[i]];
                i = p[i];
            }
            i
        }
        for b in &self.doc.bonds {
            if let (Some(i), Some(j)) = (self.pos(b.a), self.pos(b.b)) {
                let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                parent[ri] = rj;
            }
        }
        let mut groups: Vec<Piece> = Vec::new();
        let mut slot = vec![usize::MAX; n];
        let mut of = vec![0usize; n];
        for (i, g) in of.iter_mut().enumerate() {
            let r = root(&mut parent, i);
            if slot[r] == usize::MAX {
                slot[r] = groups.len();
                groups.push(Vec::new());
            }
            *g = slot[r];
            groups[slot[r]].push(self.doc.atoms[i].id);
        }
        for g in &mut groups {
            g.sort_unstable();
        }
        // ordered by the smallest id: the discovery order is by position, so the groups are renumbered
        let mut order: Vec<usize> = (0..groups.len()).collect();
        order.sort_by_key(|&g| groups[g][0]);
        let mut renumber = vec![0usize; groups.len()];
        for (new, &old) in order.iter().enumerate() {
            renumber[old] = new;
        }
        for g in &mut of {
            *g = renumber[*g];
        }
        let sorted = order
            .iter()
            .map(|&g| std::mem::take(&mut groups[g]))
            .collect();
        (sorted, of)
    }

    /// The pieces wanted as drawings of their own, all built in one pass over the atoms and the bonds (`None` for the others).
    fn documents(&self, of: &[usize], wanted: &[bool]) -> Vec<Option<Document>> {
        let mut docs: Vec<Option<Document>> = wanted
            .iter()
            .map(|&w| {
                w.then(|| Document {
                    next: self.doc.next,
                    ..Document::default()
                })
            })
            .collect();
        for (i, a) in self.doc.atoms.iter().enumerate() {
            if let Some(d) = &mut docs[of[i]] {
                d.atoms.push(a.clone());
            }
        }
        for b in &self.doc.bonds {
            if let (Some(p), Some(_)) = (self.pos(b.a), self.pos(b.b)) {
                docs[of[p]].iter_mut().for_each(|d| d.bonds.push(b.clone()));
            }
        }
        docs
    }
}

/// The connected pieces of the atoms of a document, each with ascending ids, ordered by their smallest id.
pub fn pieces(doc: &Document) -> Vec<Piece> {
    Index::new(doc).piece_index().0
}

/// A piece as a drawing of its own.
fn sub_document(doc: &Document, set: &BTreeSet<u32>) -> Document {
    Document {
        next: doc.next,
        atoms: doc
            .atoms
            .iter()
            .filter(|a| set.contains(&a.id))
            .cloned()
            .collect(),
        bonds: doc
            .bonds
            .iter()
            .filter(|b| set.contains(&b.a) && set.contains(&b.b))
            .cloned()
            .collect(),
        ..Document::default()
    }
}

/// The canonical isomeric SMILES of a drawing: `Mol::canonical` of the record `mzlab-mol` reads from the wedges and the coordinates
/// as drawn (the path of `save("smiles")`, before its text is made).
fn canonical_text(doc: &Document) -> Option<String> {
    COMPUTED.fetch_add(1, Ordering::Relaxed);
    // the writer recurses along a chain: a piece above the limit is refused as the other ways to a SMILES are (`Document::to_smiles`)
    if doc.atoms.len() > crate::formats::MAX_SMILES_ATOMS {
        return None;
    }
    let mol = doc.to_mol_stereo().canonical();
    mol.to_smiles_with_order().ok().map(|(text, _)| text)
}

/// The canonical isomeric SMILES of each of `pieces`, as `mzlab-mol` writes it from the wedges as drawn. A piece with no wedge and
/// no double bond has no configuration to guard and is left out; so is a piece the writer refuses. Order of `pieces` is kept. One
/// drawing is built per piece asked (a scan of the document each): for tests, the bench and diagnostics, not for the guard.
pub fn signature(doc: &Document, pieces: &[Piece]) -> Vec<(Piece, String)> {
    let mut out = Vec::new();
    for p in pieces {
        let set: BTreeSet<u32> = p.iter().copied().collect();
        let marked = doc
            .bonds
            .iter()
            .any(|b| (b.s != 0 || b.o == 2) && set.contains(&b.a) && set.contains(&b.b));
        if !marked {
            continue;
        }
        if let Some(text) = canonical_text(&sub_document(doc, &set)) {
            out.push((p.clone(), text));
        }
    }
    out
}

/// One stereo element with the atoms and bonds around it, as they were (a copy: the document moves on).
#[derive(Clone)]
struct Elem {
    key: Key,
    /// The core atoms first, then the neighbours.
    atoms: Vec<Atom>,
    bonds: Vec<Bond>,
    core: usize,
}

impl Elem {
    /// The element of `key` in a document; `None` when it is not there (an atom gone, no wedge at the centre, the bond not double).
    fn read(ix: &Index, key: Key) -> Option<Elem> {
        let core = key.core();
        let mut ids = core.clone();
        let mut bonds: Vec<Bond> = Vec::new();
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        for &c in &core {
            ix.atom(c)?;
            for &i in ix.bonds(c) {
                let b = &ix.doc.bonds[i];
                let other = if b.a == c { b.b } else { b.a };
                if ix.atom(other).is_none() || !seen.insert(b.id) {
                    continue;
                }
                bonds.push(b.clone());
                if !ids.contains(&other) {
                    ids.push(other);
                }
            }
        }
        let present = match key {
            Key::Centre(c) => bonds.iter().any(|b| b.a == c && b.s != 0),
            Key::Double(a, b) => bonds
                .iter()
                .any(|x| x.o == 2 && x.a.min(x.b) == a && x.a.max(x.b) == b),
        };
        present.then(|| Elem {
            key,
            atoms: ids.iter().filter_map(|&i| ix.atom(i).cloned()).collect(),
            bonds,
            core: core.len(),
        })
    }

    fn neighbours(&self) -> BTreeSet<u32> {
        self.atoms[self.core..].iter().map(|a| a.id).collect()
    }

    /// The signature of the element with only the neighbours in `keep`, each made distinct (its isotope is its rank), so the text
    /// says how those neighbours are arranged around the core.
    fn text(&self, keep: &BTreeSet<u32>) -> Option<String> {
        let mut d = Document::default();
        let mut rank = 0u16;
        for (i, a) in self.atoms.iter().enumerate() {
            d.next = d.next.max(a.id + 1);
            if i < self.core {
                d.atoms.push(a.clone());
            } else if keep.contains(&a.id) {
                rank += 1;
                let mut n = a.clone();
                (n.iso, n.chg, n.rad, n.map) = (rank, 0, 0, 0);
                n.h = None;
                n.lab = String::new();
                d.atoms.push(n);
            }
        }
        let has = |id: u32| d.atoms.iter().any(|a| a.id == id);
        d.bonds = (self.bonds.iter())
            .filter(|b| has(b.a) && has(b.b))
            .cloned()
            .collect();
        canonical_text(&d)
    }
}

/// The elements of a document: every stereocentre and every double bond with something on each end.
fn keys(ix: &Index) -> Vec<Key> {
    let mut out = Vec::new();
    for b in &ix.doc.bonds {
        if b.s != 0 {
            out.push(Key::Centre(b.a));
        } else if b.o == 2 && ix.neighbours(b.a).len() > 1 && ix.neighbours(b.b).len() > 1 {
            out.push(Key::Double(b.a.min(b.b), b.a.max(b.b)));
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// What a step does to the shape of things: the atoms each geometry operation acts on, and whether it may change the arrangement of
/// what it acts on (a mirror) or only its place (a move, a turn).
struct Geometry {
    sets: Vec<(BTreeSet<u32>, bool)>,
}

impl Geometry {
    fn of(ops: &[Op]) -> Option<Geometry> {
        let mut sets = Vec::new();
        for o in ops {
            match o {
                Op::Move { atoms, .. } | Op::Rotate { atoms, .. } if !atoms.is_empty() => {
                    sets.push((atoms.iter().copied().collect(), false));
                }
                Op::Flip { atoms, .. } if !atoms.is_empty() => {
                    sets.push((atoms.iter().copied().collect(), true));
                }
                _ => {}
            }
        }
        (!sets.is_empty()).then_some(Geometry { sets })
    }

    /// Whether the element with these atoms around it may have changed: an operation acts on part of them, or mirrors any of them. A
    /// move or turn of all of them (a rigid piece) cannot.
    fn may_change(&self, around: &BTreeSet<u32>) -> bool {
        self.sets.iter().any(|(set, mirror)| {
            let hit = around.iter().filter(|a| set.contains(a)).count();
            hit > 0 && (*mirror || hit < around.len())
        })
    }
}

/// What to compare after a step: the elements near its geometry operations, and the pieces around them (to see a mirror image of
/// the same molecule).
pub struct Snapshot {
    elems: Vec<Elem>,
    /// The piece of each element, as an index into `pieces`.
    piece_of: Vec<usize>,
    /// The pieces that hold an element: their atoms, ascending, and the drawing of each.
    pieces: Vec<(Piece, Document)>,
}

impl Snapshot {
    fn build(ix: &Index, elems: Vec<Elem>) -> Snapshot {
        let (all, of) = ix.piece_index();
        let mut wanted = vec![false; all.len()];
        let mut first: Vec<usize> = Vec::new();
        for e in &elems {
            let p = ix.pos(e.atoms[0].id).map_or(0, |i| of[i]);
            wanted[p] = true;
            first.push(p);
        }
        let mut docs = ix.documents(&of, &wanted);
        let mut slot = vec![usize::MAX; all.len()];
        let mut pieces = Vec::new();
        for (g, ids) in all.into_iter().enumerate() {
            if let Some(d) = docs[g].take() {
                slot[g] = pieces.len();
                pieces.push((ids, d));
            }
        }
        let piece_of = first.iter().map(|&g| slot[g]).collect();
        Snapshot {
            elems,
            piece_of,
            pieces,
        }
    }

    /// Every element of a document: the start of a request, for the arm `{"stereo":"kept"}`. `None` when there is none.
    pub fn all(doc: &Document) -> Option<Snapshot> {
        let ix = Index::new(doc);
        let elems: Vec<Elem> = keys(&ix)
            .into_iter()
            .filter_map(|k| Elem::read(&ix, k))
            .collect();
        (!elems.is_empty()).then(|| Snapshot::build(&ix, elems))
    }
}

/// What to compare before a step: the elements its geometry operations may change (`move`, `rotate`, `flip`, with or without other
/// operations in the same step). `None` when the step has no geometry operation, when the document has no wedge and no double bond
/// (checked first, linear in the bonds), or when no element is near what moves: nothing is computed then, and a rigid move or turn
/// of a whole piece computes nothing either.
pub fn before(doc: &Document, ops: &[Op]) -> Option<Snapshot> {
    let geo = Geometry::of(ops)?;
    if !doc.bonds.iter().any(|b| b.s != 0 || b.o == 2) {
        return None;
    }
    let ix = Index::new(doc);
    let mut elems = Vec::new();
    for key in keys(&ix) {
        let mut around: BTreeSet<u32> = key.core().into_iter().collect();
        for c in key.core() {
            around.extend(ix.neighbours(c));
        }
        if geo.may_change(&around) {
            elems.extend(Elem::read(&ix, key));
        }
    }
    (!elems.is_empty()).then(|| Snapshot::build(&ix, elems))
}

/// The atoms of the elements of `then` that are arranged otherwise in `doc`, ascending; empty when nothing changed. An element is
/// compared on the neighbours it still has (at least three for a centre). With `lost` (the arm `{"stereo":"kept"}`) an element that
/// is gone (an atom removed, fewer than three neighbours left, no wedge, no double bond) counts as changed: a configuration that is
/// not there is not kept; without it (the live line after a geometry step) only a centre that lost its wedge is said. A change that
/// leaves the piece the same molecule with the same atoms (the mirror image of a meso compound) is no change.
pub fn changed(doc: &Document, then: &Snapshot, lost: bool) -> Vec<u32> {
    let ix = Index::new(doc);
    let mut flagged: Vec<(usize, &Elem)> = Vec::new();
    for (k, e) in then.elems.iter().enumerate() {
        match Elem::read(&ix, e.key) {
            Some(now) => {
                let common: BTreeSet<u32> = (e.neighbours())
                    .intersection(&now.neighbours())
                    .copied()
                    .collect();
                if matches!(e.key, Key::Centre(_)) && common.len() < 3 {
                    if lost {
                        flagged.push((k, e));
                    }
                    continue;
                }
                if e.text(&common) != now.text(&common) {
                    flagged.push((k, e));
                }
            }
            // gone: a step of geometry never does it, a structure edit does (said only when `lost`); a centre that lost its wedge
            // but is still a centre is a configuration lost
            None => {
                let still = matches!(e.key, Key::Centre(c) if ix.neighbours(c).len() >= 3);
                if lost || still {
                    flagged.push((k, e));
                }
            }
        }
    }
    if flagged.is_empty() {
        return Vec::new();
    }
    // the same molecule drawn mirrored (meso): the piece has the same atoms and the same canonical text. Each piece is looked at once.
    let (now, of) = ix.piece_index();
    let mut end: Vec<Option<usize>> = Vec::new();
    let mut wanted = vec![false; now.len()];
    for &(k, _) in &flagged {
        let p = ix.pos(then.elems[k].atoms[0].id).map(|i| of[i]);
        if let Some(p) = p.filter(|&p| now[p] == then.pieces[then.piece_of[k]].0) {
            wanted[p] = true;
        }
        end.push(p);
    }
    let mut docs = ix.documents(&of, &wanted);
    let mut same: Vec<Option<bool>> = vec![None; then.pieces.len()];
    let mut out = BTreeSet::new();
    for (n, &(k, _)) in flagged.iter().enumerate() {
        let (e, w) = (&then.elems[k], then.piece_of[k]);
        let unchanged = match end[n].and_then(|p| docs[p].take().map(|d| (p, d))) {
            Some((_, d)) => *same[w].get_or_insert_with(|| {
                let a = canonical_text(&then.pieces[w].1);
                a.is_some() && a == canonical_text(&d)
            }),
            None => same[w].unwrap_or(false),
        };
        if !unchanged {
            out.extend(e.atoms[..e.core].iter().map(|a| a.id));
        }
    }
    out.into_iter().collect()
}
