//! The single door to 2D layout. The layout is `Mol::layout_2d` of `mzlab-mol`: structures that arrive without coordinates (a
//! SMILES, a molfile without them) are placed by it (`computed`), and the import says so. `clean_up` of a drawing is not wired
//! yet: it leaves the drawing as it is and says so.

use crate::doc::{round4, Document};

/// What `clean_up` did: `unavailable` until the layout engine is linked.
pub const CLEAN_UP_STATUS: &str = "unavailable";

/// New coordinates for the atoms (id, x, y); empty when nothing moves.
pub fn clean_up(_doc: &Document) -> Vec<(u32, f64, f64)> {
    Vec::new()
}

/// The connected pieces of a document, as lists of atom ids in pool order.
pub fn pieces(doc: &Document) -> Vec<Vec<u32>> {
    let mut seen: Vec<u32> = Vec::new();
    let mut out = Vec::new();
    for a in &doc.atoms {
        if seen.contains(&a.id) {
            continue;
        }
        let mut piece = vec![a.id];
        let mut k = 0;
        while k < piece.len() {
            for n in doc.neighbours(piece[k]) {
                if !piece.contains(&n) {
                    piece.push(n);
                }
            }
            k += 1;
        }
        seen.extend(&piece);
        out.push(piece);
    }
    out
}

/// The bond length of `Mol::layout_2d`, in angstrom (its documentation: "bond length 1.5").
const LAYOUT_BOND: f64 = 1.5;

/// Positions for a document made from a record without coordinates (atom `i` of the document is atom `i` of the record): the 2D
/// layout of `mzlab-mol` (rings as regular polygons fused along their shared bonds, chains as a zigzag, pieces left to right), in
/// bond lengths of the drawing.
pub fn computed(doc: &mut Document, m: &mzlab_mol::Mol) {
    let xy = m.layout_2d().xy;
    for (a, p) in doc.atoms.iter_mut().zip(&xy) {
        if let [x, y, ..] = p[..] {
            (a.x, a.y) = (round4(x / LAYOUT_BOND), round4(y / LAYOUT_BOND));
        }
    }
}
