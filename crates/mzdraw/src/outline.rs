//! The drawing in words, for the list next to the canvas that a screen reader reads: one line per connected structure with its
//! molecular formula, and what the keyboard focus is on. Counted on the record (`Document::to_mol`), so abbreviations count as
//! the atoms they stand for.

use crate::doc::Document;
use crate::label;

/// One connected structure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    /// Hill formula (C, H, then the other elements by name; `R` for R groups), with the net charge when there is one.
    pub formula: String,
    pub atoms: usize,
    pub bonds: usize,
    /// The id of the first atom of the structure: the atom its data is set on.
    pub anchor: u32,
    /// The data of the structure (the data fields of an SDF record): the data of its atoms, in pool order.
    pub data: crate::doc::Data,
    /// The current entry of the annotation of the structure (the annotations of its atoms, in pool order), if it has one.
    pub ann: Option<crate::ann::Current>,
}

/// What the keyboard focus is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Focus {
    pub id: u32,
    /// The label of the atom if it has one, else its element.
    pub name: String,
    /// Position of the atom in the drawing, from 1.
    pub index: usize,
    pub of: usize,
    pub bonds: usize,
    pub hydrogens: u32,
    pub charge: i8,
}

fn hill(counts: &mut [(String, u32)], charge: i32) -> String {
    let rank = |el: &str| match el {
        "C" => 0,
        "H" => 1,
        _ => 2,
    };
    crate::m::sort_by(counts, |a, b| (rank(&a.0), &a.0) < (rank(&b.0), &b.0));
    let mut out = String::new();
    for (el, n) in counts.iter().filter(|c| c.1 > 0) {
        out.push_str(el);
        if *n > 1 {
            out.push_str(&n.to_string());
        }
    }
    if charge != 0 {
        if charge.abs() > 1 {
            out.push_str(&charge.abs().to_string());
        }
        out.push(if charge > 0 { '+' } else { '\u{2212}' });
    }
    out
}

/// The connected structures of a drawing, in the order of their first atom.
pub fn pieces(doc: &Document) -> Vec<Piece> {
    let m = doc.to_mol();
    let n = m.atoms.len();
    let r_groups: Vec<bool> = (0..n)
        .map(|i| {
            matches!(
                doc.atoms.get(i).map(|a| label::kind(&a.lab)),
                Some(Some(label::Kind::R(_)))
            )
        })
        .collect();
    let mut piece_of: Vec<usize> = (0..n).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for b in &m.bonds {
        let (a, z) = (
            root(&mut piece_of, b.a as usize),
            root(&mut piece_of, b.b as usize),
        );
        // the smaller index stays the root, so pieces come out in the order of their first atom
        piece_of[a.max(z)] = a.min(z);
    }
    let mut out = Vec::new();
    for first in 0..n {
        if root(&mut piece_of, first) != first {
            continue;
        }
        let (mut counts, mut charge, mut atoms): (Vec<(String, u32)>, i32, usize) =
            (Vec::new(), 0, 0);
        let mut add = |el: &str, k: u32| match counts.iter_mut().find(|c| c.0 == el) {
            Some(c) => c.1 += k,
            None => counts.push((el.to_string(), k)),
        };
        for (i, is_r) in r_groups.iter().enumerate() {
            if root(&mut piece_of, i) != first {
                continue;
            }
            atoms += 1;
            if *is_r {
                add("R", 1);
            } else {
                add(&m.atoms[i].el, 1);
                add("H", m.atoms[i].h as u32);
                charge += m.atoms[i].chg as i32;
            }
        }
        let bonds = (m.bonds.iter())
            .filter(|b| root(&mut piece_of, b.a as usize) == first)
            .count();
        out.push(Piece {
            formula: hill(&mut counts, charge),
            atoms,
            bonds,
            anchor: doc.atoms.get(first).map_or(0, |a| a.id),
            data: (0..doc.atoms.len().min(n))
                .filter(|&i| root(&mut piece_of, i) == first)
                .flat_map(|i| doc.atoms[i].data())
                .collect(),
            ann: {
                let ids: Vec<u32> = (0..doc.atoms.len().min(n))
                    .filter(|&i| root(&mut piece_of, i) == first)
                    .map(|i| doc.atoms[i].id)
                    .collect();
                crate::ann::current(&crate::doc::Atom::ann_of(&doc.atoms, &ids))
            },
        });
    }
    out
}

/// The English words of the text alternative of a saved picture. A small fixed table: the file has no i18n layer.
const WORDS: [&str; 6] = [
    "Empty drawing",
    "and",
    "with a reaction arrow",
    "Reaction arrow",
    "Drawing",
    "level",
];

/// The text alternative of a drawing, as `(title, desc)`: the title is the outline line (the Hill formulas of the structures
/// joined by "and", then the arrow if there is one), the desc lists each structure with its formula and, when it is annotated,
/// its level. Plain text, not escaped.
pub fn alternative(doc: &Document) -> (String, String) {
    let pieces = pieces(doc);
    let arrow = doc
        .objects
        .iter()
        .any(|o| o.t == crate::doc::ObjKind::Arrow);
    if pieces.is_empty() && !arrow {
        let t = if doc.is_empty() { WORDS[0] } else { WORDS[4] };
        return (t.into(), t.into());
    }
    let mut title = String::new();
    for (i, p) in pieces.iter().enumerate() {
        if i > 0 {
            title.push_str(if i + 1 == pieces.len() { " and " } else { ", " });
        }
        title.push_str(&p.formula);
    }
    let mut desc = String::new();
    for (i, p) in pieces.iter().enumerate() {
        if i > 0 {
            desc.push(' ');
        }
        desc.push_str(&p.formula);
        if let Some(a) = p.ann.as_ref().filter(|a| !a.level.is_empty()) {
            desc.push_str(&format!(" ({} {})", WORDS[5], a.level));
        }
        desc.push('.');
    }
    if arrow {
        if pieces.is_empty() {
            title.push_str(WORDS[3]);
            desc.push_str(WORDS[3]);
            desc.push('.');
        } else {
            title.push(' ');
            title.push_str(WORDS[2]);
            desc.push(' ');
            desc.push_str(WORDS[2]);
            desc.push('.');
        }
    }
    (title, desc)
}

/// What the atom `id` is, for the line that says where the keyboard focus is.
pub fn focus(doc: &Document, id: u32) -> Option<Focus> {
    let index = doc.atom_index(id)?;
    let a = &doc.atoms[index];
    Some(Focus {
        id,
        name: if a.lab.is_empty() {
            a.el.clone()
        } else {
            a.lab.clone()
        },
        index: index + 1,
        of: doc.atoms.len(),
        bonds: doc.neighbours(id).len(),
        hydrogens: doc.hydrogens(a),
        charge: if a.lab.is_empty() { a.chg } else { 0 },
    })
}
