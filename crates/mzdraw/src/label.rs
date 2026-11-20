//! Atom labels: abbreviations of common groups (drawn as their name, expanded to their atoms at export) and R-group placeholders
//! (`R`, `R1` ... `R32`). The table is the one place that says what an abbreviation stands for.

/// Abbreviation and its expansion as a SMILES whose first atom is the attachment point.
pub const ABBREVIATIONS: &[(&str, &str)] = &[
    ("Me", "C"),
    ("Et", "CC"),
    ("Pr", "CCC"),
    ("iPr", "C(C)C"),
    ("Bu", "CCCC"),
    ("tBu", "C(C)(C)C"),
    ("Ph", "c1ccccc1"),
    ("Bn", "Cc1ccccc1"),
    ("Ac", "C(C)=O"),
    ("Bz", "C(=O)c1ccccc1"),
    ("Boc", "C(=O)OC(C)(C)C"),
    ("Cbz", "C(=O)OCc1ccccc1"),
    ("Fmoc", "C(=O)OCC1c2ccccc2-c2ccccc12"),
    ("Ts", "S(=O)(=O)c1ccc(C)cc1"),
    ("Ms", "S(C)(=O)=O"),
    ("Tf", "S(=O)(=O)C(F)(F)F"),
    ("OMe", "OC"),
    ("OEt", "OCC"),
    ("OAc", "OC(C)=O"),
    ("OTs", "OS(=O)(=O)c1ccc(C)cc1"),
    ("NMe2", "N(C)C"),
    ("NO2", "[N+](=O)[O-]"),
    ("CN", "C#N"),
    ("CF3", "C(F)(F)F"),
    ("COOH", "C(=O)O"),
    ("CO2Me", "C(=O)OC"),
    ("CO2Et", "C(=O)OCC"),
    ("SO3H", "S(=O)(=O)O"),
    ("TMS", "[Si](C)(C)C"),
    ("TBS", "[Si](C)(C)C(C)(C)C"),
];

/// What a label is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// An abbreviation, with its expansion.
    Group(&'static str),
    /// An R-group placeholder; 0 for a bare `R`.
    R(u8),
}

/// The kind of a label, `None` for a text that is neither an abbreviation of the table nor `R`, `R1` ... `R32`.
pub fn kind(label: &str) -> Option<Kind> {
    if let Some(&(_, smiles)) = ABBREVIATIONS.iter().find(|(name, _)| *name == label) {
        return Some(Kind::Group(smiles));
    }
    let n = label.strip_prefix('R')?;
    if n.is_empty() {
        return Some(Kind::R(0));
    }
    let ok = !n.starts_with('0') && n.bytes().all(|b| b.is_ascii_digit());
    n.parse()
        .ok()
        .filter(|v| ok && (1..=32).contains(v))
        .map(Kind::R)
}
