//! CIP labels drawn in the picture (`display.cip`, rdkit-compare 6): R/S at a stereocentre, E/Z beside a double bond. The labels are
//! computed by the engine `cip` of mzLab, which this crate does not have: the editor is given a `Labeller` (a plain function pointer,
//! as the masses are given a `values::Measurer`) by the bridge crate (`mzdraw-wasm`, the glue). Without one, `display.cip=on` draws
//! nothing and a `draw-result/1` says so in `lost` (`draw2.export.cipEngine`): no label is ever made up here.

use crate::doc::Document;

/// One label: on an atom (`atom` its id, `bond` 0) or on a bond (`bond` its id, `atom` 0), `text` the descriptor as the engine
/// gives it (`R`, `S`, `r`, `s`, `E`, `Z`, ...). The picture writes it in brackets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CipLabel {
    pub atom: u32,
    pub bond: u32,
    pub text: String,
}

/// The labels of a drawing, by document ids; the error is an i18n key.
pub type Labeller = fn(&Document) -> Result<Vec<CipLabel>, &'static str>;

/// The key of the `lost` item of a `draw-result/1` when `display.cip` is on and no labeller is attached (the plain core replay).
pub const NO_ENGINE: &str = "draw2.export.cipEngine";

/// The labels in a stable order: atoms by id, then bonds by id (the engine's order is the molfile's, which follows the pool).
pub fn sorted(mut labels: Vec<CipLabel>) -> Vec<CipLabel> {
    labels.sort_by_key(|a| (a.bond != 0, a.atom, a.bond));
    labels
}

/// The key of the `lost` item (and of the page's note) for the structures left without labels because they have more than
/// `MAX_OPEN` open stereo sites or more than `MAX_ATOMS` atoms: the engine's work grows exponentially with the open sites of what it
/// is given, so such a structure is not given to it (never a hang, never in silence).
pub const CAPPED: &str = "draw2.export.cipCapped";

/// The key of the `lost` item (and of the page's note) for the structures the engine refused (its error): they have no labels.
pub const FAILED: &str = "draw2.export.cipFailed";

/// The most open stereo sites a structure given to the engine may have (`open_sites`).
pub const MAX_OPEN: usize = 16;

/// The most atoms a structure given to the engine may have.
pub const MAX_ATOMS: usize = 500;

/// The labels of a drawing and what was not labelled: the structures capped (`CAPPED`) and refused (`FAILED`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Labelled {
    pub labels: Vec<CipLabel>,
    pub capped: usize,
    pub failed: usize,
}

impl Labelled {
    /// The notes of what was not labelled, as (i18n key, count), the ones that are not zero.
    pub fn notes(&self) -> Vec<(&'static str, usize)> {
        [(CAPPED, self.capped), (FAILED, self.failed)]
            .into_iter()
            .filter(|n| n.1 > 0)
            .collect()
    }
}

/// The open stereo sites of a structure, counted from its drawing alone (linear, no engine): an atom of C, N, P, S or Si with three or
/// more bonds, all single, and no wedge or hash starting at it (a centre whose configuration is not drawn), and a double bond drawn
/// "either" (`ez`, an E/Z the drawing leaves open). These are the sites the engine has to try both ways; the defined ones are cheap.
pub fn open_sites(doc: &Document, atoms: &[u32]) -> usize {
    use std::collections::{HashMap, HashSet};
    let inside: HashSet<u32> = atoms.iter().copied().collect();
    // per atom: its bonds, whether all are single, whether a wedge or a hash starts at it
    let mut at: HashMap<u32, (usize, bool, bool)> = HashMap::new();
    let mut either = 0;
    for b in doc.bonds.iter().filter(|b| inside.contains(&b.a)) {
        for (end, start) in [(b.a, true), (b.b, false)] {
            let e = at.entry(end).or_insert((0, true, false));
            e.0 += 1;
            e.1 &= b.o == 1;
            e.2 |= start && b.s != 0;
        }
        either += usize::from(b.either());
    }
    let centres = (atoms.iter())
        .filter_map(|id| doc.atom(*id))
        .filter(|a| matches!(a.el.as_str(), "C" | "N" | "P" | "S" | "Si"))
        .filter(|a| {
            at.get(&a.id)
                .is_some_and(|&(n, single, wedged)| n >= 3 && single && !wedged)
        })
        .count();
    centres + either
}

/// The labels of a drawing, structure by structure: the engine is given each connected structure on its own (a document of its atoms
/// and bonds, with their ids), so its work is the sum of the structures' and not that of their product; a structure over the caps
/// (`MAX_OPEN`, `MAX_ATOMS`) is not given to it and is counted `capped`; one the engine refuses is counted `failed`. Labels in the
/// order of `sorted`.
pub fn label(doc: &Document, engine: Labeller) -> Labelled {
    let pieces = crate::layout::pieces(doc);
    let of: std::collections::HashMap<u32, usize> = (pieces.iter().enumerate())
        .flat_map(|(k, p)| p.iter().map(move |&a| (a, k)))
        .collect();
    let mut subs: Vec<Document> = pieces
        .iter()
        .map(|_| Document {
            next: doc.next,
            ..Document::default()
        })
        .collect();
    for a in &doc.atoms {
        if let Some(&k) = of.get(&a.id) {
            subs[k].atoms.push(a.clone());
        }
    }
    for b in &doc.bonds {
        if let Some(&k) = of.get(&b.a) {
            subs[k].bonds.push(b.clone());
        }
    }
    let mut out = Labelled::default();
    for (piece, sub) in pieces.iter().zip(&subs) {
        if piece.len() > MAX_ATOMS || open_sites(sub, piece) > MAX_OPEN {
            out.capped += 1;
            continue;
        }
        match engine(sub) {
            Ok(l) => out.labels.extend(l),
            Err(_) => out.failed += 1,
        }
    }
    out.labels = sorted(out.labels);
    out
}
