//! The lines kept under a molecule (struct-props): the owner chooses, per molecule, which calculated values stay written under it.
//!
//! **The choice** is the optional field `cap` of an atom of the molecule: a JSON list of line keys (`LINES`), in the order they are
//! written, e.g. `["formula","mw","logp"]`. It lives in the unknown-field list of the atom (`doc::More`), like `col`, so a document
//! without it reads and writes as before and an older reader keeps it. `mzdraw/1` is unchanged. A molecule's choice is the `cap` of
//! the first of its atoms (pool order) that has one; the operation `atom.cap` (`ops::Op::AtomCap`) sets it on the first atom of the
//! molecule and clears it on the others, as the annotation does. A molecule with no choice follows the style `display.values`.
//! A key this version does not know is kept in the list and writes no line.
//!
//! **The numbers.** Formula, average mass, exact mass, element composition in per cent and the adduct m/z come from the one shared
//! measure (`values`, the engine of mzLab through the bridge). The counts and the two sums below are the descriptors of `mzlab-mol`
//! (`Mol::descriptors`, tables of the library, no engine, no new dependency): `logp` the Crippen logP, `tpsa` the topological polar
//! surface area (A^2), `hbd` and `hba` the hydrogen-bond donors and acceptors, `rotb` the rotatable bonds; `heavy` is the count of atoms
//! that are not hydrogen, made here. Definitions are the ones of that library (`docs/built/struct-props.md` says them again).

use crate::doc::More;
use mzlab_mol::Mol;

/// The keys of the lines, in the order the panel lists them. `mz` writes one line per adduct of `display.adducts`.
pub const LINES: [&str; 11] = [
    "formula", "mw", "exact", "ea", "mz", "logp", "tpsa", "hbd", "hba", "rotb", "heavy",
];

/// At most this many keys of at most `MAX_KEY` characters are kept for a molecule.
pub const MAX_LINES: usize = 24;
pub const MAX_KEY: usize = 40;

/// The elements the descriptor numbers are written for: the organic set the Wildman-Crippen typing is written for (`H C N O F Cl Br I P S`).
/// The library also gives a default type to metals, semimetals and other elements (a bare `Na+` gets a logP), which is not a measure of the
/// structure: a structure with any other element has no descriptor number, only the key `error.draw2.propsElement`.
pub const DESCRIPTOR_ELEMENTS: [&str; 10] = ["H", "C", "N", "O", "F", "Cl", "Br", "I", "P", "S"];

/// The method of each line: a stable id and the English text (the page shows the words of its catalog, `draw2.props.method.<key>`).
pub fn method(key: &str) -> Option<(&'static str, &'static str)> {
    Some(match key {
        "formula" => ("hill-formula", "counted on the drawing, Hill order, abbreviations expanded"),
        "mw" => ("average-mass-conventional-weights", "average mass, the conventional atomic weights of the mzLab engine sel"),
        "exact" => ("monoisotopic-mass", "monoisotopic mass from the mzLab engine sel"),
        "ea" => ("mass-percent-analysis", "mass per cent of each element of the formula, from the mzLab engine analysis/1"),
        "mz" => ("adduct-shift", "monoisotopic mass plus the adduct shift of the mzLab engine"),
        "logp" => ("wildman-crippen-logp", "Wildman-Crippen logP, from the molecular descriptors of mzlab-mol (its own definitions, not another toolkit's)"),
        "tpsa" => ("tpsa-ertl-n-o-only", "topological polar surface area from N and O only (Ertl 2000), from the molecular descriptors of mzlab-mol (its own definitions)"),
        "hbd" => ("mzlab-mol-hbd", "hydrogen-bond donors by the molecular descriptors of mzlab-mol (its own definitions)"),
        "hba" => ("mzlab-mol-hba", "hydrogen-bond acceptors by the molecular descriptors of mzlab-mol (its own definitions)"),
        "rotb" => ("mzlab-mol-rotatable-bonds", "rotatable bonds by the molecular descriptors of mzlab-mol (its own definitions)"),
        "heavy" => ("heavy-atom-count", "atoms that are not hydrogen, counted on the drawing"),
        _ => return None,
    })
}

/// The lines that need the engine (the shared measure); the others come from the record alone.
pub fn needs_engine(key: &str) -> bool {
    matches!(key, "mw" | "exact" | "ea" | "mz")
}

/// The canonical list of a choice: the empty keys and the repeats dropped, the order kept. `None` for a list that is too long, or a key
/// that is too long or has a control character.
pub fn canon(lines: &[String]) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for k in lines.iter().map(|k| k.trim()).filter(|k| !k.is_empty()) {
        if k.chars().count() > MAX_KEY || k.chars().any(char::is_control) {
            return None;
        }
        if !out.iter().any(|o| o == k) {
            out.push(k.to_string());
        }
    }
    (out.len() <= MAX_LINES).then_some(out)
}

/// The choice kept in the fields of an atom, empty for none (also when the field is not a list of texts: it is left alone).
pub(crate) fn get(more: &More) -> Vec<String> {
    (more.iter().find(|m| m.0 == "cap"))
        .and_then(|m| serde_json::from_str::<Vec<String>>(&m.1).ok())
        .and_then(|l| canon(&l))
        .unwrap_or_default()
}

/// Replaces the choice kept in the fields (`lines` canonical); the empty list takes the field away.
pub(crate) fn put(more: &mut More, lines: &[String]) {
    more.retain(|m| m.0 != "cap");
    if !lines.is_empty() {
        more.push(("cap".into(), serde_json::json!(lines).to_string()));
        crate::m::sort(more);
    }
}

/// The descriptors of one structure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Props {
    pub logp: f64,
    pub tpsa: f64,
    pub hbd: u32,
    pub hba: u32,
    pub rotb: u32,
    pub heavy: u32,
}

/// A hash of what the descriptors depend on (the atoms and the bonds of the record).
fn sig(m: &Mol) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x100_0000_01b3);
    };
    for a in &m.atoms {
        a.el.bytes().for_each(|b| mix(u64::from(b)));
        mix(u64::from(a.iso) << 24 | u64::from(a.h) << 8 | u64::from(a.rad));
        mix(a.chg as u8 as u64);
    }
    for b in &m.bonds {
        mix(u64::from(b.a) << 32 | u64::from(b.b) << 8 | u64::from(b.o));
    }
    mix(m.atoms.len() as u64);
    h
}

thread_local! {
    /// The last results by record: a pure memo (the same record always gives the same numbers), so the painter does not ask the
    /// library again at every frame. Bounded.
    static MEMO: std::cell::RefCell<std::collections::HashMap<u64, Props>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// The descriptors of a record (hydrogens as counted on the atoms, bonds in Kekule form), memoised by content.
pub fn of(m: &Mol) -> Props {
    let key = sig(m);
    if let Some(p) = MEMO.with(|c| c.borrow().get(&key).copied()) {
        return p;
    }
    let d = m.descriptors();
    let p = Props {
        logp: d.logp,
        tpsa: d.tpsa,
        hbd: d.hbd,
        hba: d.hba,
        rotb: d.rotors,
        heavy: m.atoms.iter().filter(|a| a.el != "H").count() as u32,
    };
    MEMO.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= 512 {
            c.clear();
        }
        c.insert(key, p);
    });
    p
}

/// Per cent with two decimals, half away from zero (`mzlab_mol::fixed`).
/// A value that rounds to zero has no sign (`-0.00` is `0.00`).
pub fn two(v: f64) -> String {
    let t = mzlab_mol::fixed(v, 2);
    if t == "-0.00" {
        "0.00".to_string()
    } else {
        t
    }
}

/// Forgets the memo of `of` (the tests of cost time the descriptors, not the memo).
pub fn forget() {
    MEMO.with(|c| c.borrow_mut().clear());
}

/// The (name, value) of a descriptor line, `None` for a key that is not one of them.
pub fn line(key: &str, p: &Props) -> Option<(&'static str, String)> {
    Some(match key {
        "logp" => ("logP", two(p.logp)),
        "tpsa" => ("tPSA", two(p.tpsa)),
        "hbd" => ("HBD", p.hbd.to_string()),
        "hba" => ("HBA", p.hba.to_string()),
        "rotb" => ("Rotatable bonds", p.rotb.to_string()),
        "heavy" => ("Heavy atoms", p.heavy.to_string()),
        _ => return None,
    })
}

/// The formats of `Editor::save` that have no place for the lines of a molecule (they are written without them and the loss is said).
/// The document, SVG, PNG, PDF and CDXML carry them (CDXML as a text object under the molecule).
pub const LOST_IN: [&str; 8] = [
    "molfile",
    "molfile-v3000",
    "smiles",
    "sdf",
    "ket",
    "cml",
    "rxn",
    "trasf",
];

impl crate::doc::Document {
    /// How many atoms carry a choice of lines that the output `kind` of `Editor::save` cannot hold.
    pub fn caps_lost(&self, kind: &str) -> usize {
        if !LOST_IN.contains(&kind) {
            return 0;
        }
        self.atoms.iter().filter(|a| !a.cap().is_empty()).count()
    }
}
