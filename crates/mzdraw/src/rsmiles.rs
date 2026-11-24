//! Reaction SMILES and dative bonds in SMILES (rdkit-compare 7), read before the SMILES reader of `mzlab-mol`, which knows neither.
//!
//! - A reaction SMILES is `reactants>agents>products`, each side a list of molecules joined by `.` (Daylight; a side may be empty).
//!   `reaction` cuts a line at its two `>` (outside brackets, and not the `>` of a dative `->`), and the import draws the reaction
//!   as a reaction file is drawn (`rxn::reaction_of`): reactants with plus signs, the arrow, the agents above it, the products.
//! - A dative bond is `->` (from the donor before it to the acceptor after it) or `<-` (from the atom after it to the one before it),
//!   as RDKit writes them. `datives` writes each as a single bond for the reader and gives its ends, as indices of the atoms in the
//!   order of the text; the import then makes that bond `dative` (`bonddisp::KINDS`), from the donor to the acceptor, so it takes
//!   no part in the valence of its ends (the donor of `N->[Cu]` keeps its three hydrogens).

/// The three parts of a reaction SMILES (reactants, agents, products), or `None` for a line that is not one (no `>` that is a
/// reaction's). `Err(())` for a line with one `>` or more than two.
pub(crate) fn reaction(smiles: &str) -> Result<Option<[&str; 3]>, ()> {
    let b = smiles.as_bytes();
    let mut cuts = Vec::new();
    let mut inside = false;
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'[' => inside = true,
            b']' => inside = false,
            b'>' if !inside && (i == 0 || b[i - 1] != b'-') => cuts.push(i),
            _ => {}
        }
    }
    match cuts.as_slice() {
        [] => Ok(None),
        [a, z] => Ok(Some([&smiles[..*a], &smiles[a + 1..*z], &smiles[z + 1..]])),
        _ => Err(()),
    }
}

/// The molecules of one side of a reaction SMILES: its parts between `.`, the empty ones left out.
pub(crate) fn molecules(side: &str) -> Vec<&str> {
    side.split('.').filter(|m| !m.is_empty()).collect()
}

/// A SMILES with each dative bond (`->`, `<-`) written as a single bond (`-`), and the ends of each as (donor, acceptor), indices of
/// the atoms in the order of the text. `Err(())` for a dative bond that is not between two atoms written next to each other (one
/// to a ring closure, a branch or a dot).
pub(crate) fn datives(smiles: &str) -> Result<(String, Vec<(usize, usize)>), ()> {
    if !smiles.contains("->") && !smiles.contains("<-") {
        return Ok((smiles.to_string(), Vec::new()));
    }
    let b = smiles.as_bytes();
    let mut out = String::with_capacity(smiles.len());
    let mut pairs = Vec::new();
    let (mut i, mut atoms) = (0, 0usize);
    let mut prev: Option<usize> = None;
    let mut stack: Vec<Option<usize>> = Vec::new();
    // a dative bond read and waiting for the atom after it: whether the atom before it is the donor
    let mut pending: Option<bool> = None;
    while i < b.len() {
        let c = b[i];
        let mut end = i + 1;
        let atom = match c {
            b'[' => {
                end = smiles[i..].find(']').map_or(b.len(), |k| i + k + 1);
                true
            }
            b'C' | b'B' if matches!(b.get(end), Some(b'l' | b'r')) => {
                end += 1;
                true
            }
            b'*' => true,
            c if c.is_ascii_alphabetic() => true,
            _ => false,
        };
        if atom {
            if let Some(donor_first) = pending.take() {
                let before = prev.ok_or(())?;
                pairs.push(if donor_first {
                    (before, atoms)
                } else {
                    (atoms, before)
                });
            }
            prev = Some(atoms);
            atoms += 1;
            out.push_str(&smiles[i..end]);
            i = end;
            continue;
        }
        if pending.is_some() {
            // only the atom itself may follow the arrow
            return Err(());
        }
        match (c, b.get(i + 1)) {
            (b'-', Some(b'>')) => {
                pending = Some(true);
                out.push('-');
                i += 2;
                continue;
            }
            (b'<', Some(b'-')) => {
                pending = Some(false);
                out.push('-');
                i += 2;
                continue;
            }
            (b'(', _) => stack.push(prev),
            (b')', _) => prev = stack.pop().ok_or(())?,
            (b'.', _) => prev = None,
            _ => {}
        }
        // a character of more than one byte (outside a bracket) is copied whole
        let ch = smiles[i..].chars().next().unwrap_or(c as char);
        out.push(ch);
        i += ch.len_utf8();
    }
    if pending.is_some() {
        return Err(());
    }
    Ok((out, pairs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reaction_is_cut_at_its_two_signs_and_not_at_a_dative_arrow() {
        assert_eq!(reaction("CC>>CO"), Ok(Some(["CC", "", "CO"])));
        assert_eq!(
            reaction("CC(=O)O.OCC>[H+]>CC(=O)OCC.O"),
            Ok(Some(["CC(=O)O.OCC", "[H+]", "CC(=O)OCC.O"]))
        );
        assert_eq!(reaction("N->[Cu]>>N"), Ok(Some(["N->[Cu]", "", "N"])));
        assert_eq!(reaction("CCO"), Ok(None));
        assert_eq!(reaction("N->[Cu+2]"), Ok(None));
        assert_eq!(reaction("CC>CO"), Err(()));
        assert_eq!(reaction("C>C>C>C"), Err(()));
        assert_eq!(molecules("CC..O."), ["CC", "O"]);
        assert!(molecules("").is_empty());
    }

    #[test]
    fn a_dative_bond_is_a_single_bond_with_its_donor_and_acceptor() {
        assert_eq!(datives("N->[Cu+2]"), Ok(("N-[Cu+2]".into(), vec![(0, 1)])));
        assert_eq!(datives("[Cu+2]<-N"), Ok(("[Cu+2]-N".into(), vec![(1, 0)])));
        // through a branch and two-letter atoms: the N and the Br of the branch both give to the Fe
        assert_eq!(
            datives("ClC(N->[Fe]<-Br)O"),
            Ok(("ClC(N-[Fe]-Br)O".into(), vec![(2, 3), (4, 3)]))
        );
        assert_eq!(datives("CCO"), Ok(("CCO".into(), vec![])));
        // a character of more than one byte is copied whole (the reader judges it), never cut into bytes
        assert_eq!(
            datives("N->[Cu]\u{e9}\u{4e2d}"),
            Ok(("N-[Cu]\u{e9}\u{4e2d}".into(), vec![(0, 1)]))
        );
        // at the start of a branch the atom before is the branch's own
        assert_eq!(datives("C(->N)O"), Ok(("C(-N)O".into(), vec![(0, 1)])));
        // not between two atoms written next to each other
        for bad in ["N->1CC1", "N->(C)C", "N->.C", "N->", "->N", "N.->C"] {
            assert_eq!(datives(bad), Err(()), "{bad}");
        }
    }
}
