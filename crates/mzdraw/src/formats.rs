//! Formats, first family: what the molfile and SMILES doors need around the reader and the writers of `mzlab-mol`. SMILES out with
//! the brackets a reader does not need taken away; the V3000 molfile with the wedge marks and the R groups of the drawing; the
//! parts of a molfile the record has no place for (R groups, query atoms and bonds, S-groups), taken out before the strict reader
//! sees them and reported, never dropped in silence.

use crate::doc::{Document, WEDGE_DOWN, WEDGE_UP};
use crate::io::{Imported, R_STAND_IN};
use crate::label::{self, Kind};
use mzlab_mol::{GroupKind, Mol, StereoGroup};

/// Atom symbols of a molfile that ask a question instead of naming an atom.
const QUERY_ATOMS: &[&str] = &["A", "AH", "Q", "QH", "X", "XH", "M", "MH", "*", "L"];

impl Document {
    /// The whole canvas as one SMILES written by `mzlab-mol` (pieces joined by `.`, Kekulé bonds, stereo from the wedges as drawn,
    /// abbreviations expanded). An atom of the organic subset with its usual hydrogens is written without brackets; an R group is
    /// the atom `[*]`, with its number as the atom class (`[*:1]`). The error is an i18n key.
    pub fn to_smiles(&self) -> Result<String, &'static str> {
        let m = self.to_mol_stereo();
        let (text, order) = m
            .to_smiles_with_order()
            .map_err(|_| "error.draw2.smilesOut")?;
        let mut used = vec![0u32; m.atoms.len()];
        for b in &m.bonds {
            used[b.a as usize] += u32::from(b.o);
            used[b.b as usize] += u32::from(b.o);
        }
        let mut out = String::with_capacity(text.len());
        // the writer puts every atom in brackets, in the order it gives: each is looked at in place
        let mut written = order.iter().map(|&i| i as usize);
        let mut rest = text.as_str();
        while let Some(open) = rest.find('[') {
            let close = rest[open..].find(']').map_or(rest.len(), |k| open + k + 1);
            let token = &rest[open..close];
            let i = written.next().unwrap_or(usize::MAX);
            out.push_str(&rest[..open]);
            match (
                self.atoms.get(i).and_then(|a| label::kind(&a.lab)),
                m.atoms.get(i),
            ) {
                (Some(Kind::R(0)), _) => out.push_str("[*]"),
                (Some(Kind::R(n)), _) => out.push_str(&format!("[*:{n}]")),
                (_, Some(a)) if !token.contains('@') && bare(a, used[i]) => out.push_str(&a.el),
                _ => out.push_str(token),
            }
            rest = &rest[close..];
        }
        out.push_str(rest);
        Ok(out)
    }

    /// The whole canvas as a V3000 molfile: as `to_molfile`, with the wedge marks as `CFG` of the bonds and the R groups as `R#`
    /// atoms with `RGROUPS`.
    pub fn to_molfile_v3000(&self) -> String {
        self.v3000(self.to_mol(), false)
    }

    /// The V3000 text of the record `m` of this document (`Document::to_mol`), written by `mzlab-mol` and marked line by line.
    pub(crate) fn v3000(&self, mut m: Mol, r_as_query: bool) -> String {
        // the writer of `mzlab-mol` chooses V3000 for a record with stereo groups: an empty one is lent and its block left out
        m.groups.push(StereoGroup {
            kind: GroupKind::Abs,
            atoms: Vec::new(),
        });
        let mut out = String::new();
        let (mut block, mut k) = (' ', 0);
        for line in m.to_molfile(crate::NAME).lines() {
            let body = line.strip_prefix("M  V30 ").unwrap_or("");
            let mut marked = None;
            match (body, block) {
                ("BEGIN ATOM", _) => (block, k) = ('a', 0),
                ("BEGIN BOND", _) => (block, k) = ('b', 0),
                ("BEGIN COLLECTION", _) => block = 'c',
                ("END COLLECTION", _) => {
                    block = ' ';
                    continue;
                }
                ("END ATOM" | "END BOND", _) => block = ' ',
                (_, 'a') => {
                    let r = self.atoms.get(k).and_then(|a| label::kind(&a.lab));
                    if let Some(Kind::R(n)) = r {
                        // index, stand-in, x, y, z: the symbol is replaced, what follows the coordinates is not about an R group
                        let f: Vec<&str> = body.split(' ').collect();
                        let (symbol, number) = match (r_as_query, n) {
                            (true, _) => ("A", String::new()),
                            (false, 0) => ("R#", String::new()),
                            (false, n) => ("R#", format!(" RGROUPS=(1 {n})")),
                        };
                        if let [i, _, x, y, z, ..] = f[..] {
                            marked = Some(format!("M  V30 {i} {symbol} {x} {y} {z} 0{number}"));
                        }
                    }
                    k += 1;
                }
                (_, 'b') => {
                    let ends = (self.bonds.get(k).filter(|b| b.s != 0))
                        .and_then(|b| Some((b.s, self.atom_index(b.a)?, self.atom_index(b.b)?)));
                    if let Some((s, i, j)) = ends {
                        let cfg = match s {
                            WEDGE_UP => 1,
                            WEDGE_DOWN => 3,
                            _ => 2,
                        };
                        marked = Some(format!("M  V30 {} 1 {} {} CFG={cfg}", k + 1, i + 1, j + 1));
                    }
                    k += 1;
                }
                _ => {}
            }
            if block != 'c' {
                out.push_str(marked.as_deref().unwrap_or(line));
                out.push('\n');
            }
        }
        out
    }

    /// Adds every atom and bond of `other` to the right of this document, with ids after the ones in use.
    pub(crate) fn append(&mut self, mut other: Document) {
        let shift = self.next.max(1) - 1;
        let dx = match (self.bounds(), other.bounds()) {
            (Some(mine), Some(new)) => mine.2 + 2.0 - new.0,
            _ => 0.0,
        };
        for a in &mut other.atoms {
            a.id += shift;
            a.x = crate::doc::round4(a.x + dx);
        }
        for b in &mut other.bonds {
            (b.id, b.a, b.b) = (b.id + shift, b.a + shift, b.b + shift);
        }
        self.next = shift + other.next.max(1);
        self.atoms.append(&mut other.atoms);
        self.bonds.append(&mut other.bonds);
    }
}

/// Whether a SMILES reader gives the atom, written without brackets, the same hydrogens: an element of the organic subset, no
/// charge, isotope or unpaired electron, and the hydrogens that fill its lowest usual valence.
fn bare(a: &mzlab_mol::Atom, used: u32) -> bool {
    let usual: &[u32] = match a.el.as_str() {
        "B" => &[3],
        "C" => &[4],
        "N" | "P" => &[3, 5],
        "O" => &[2],
        "S" => &[2, 4, 6],
        "F" | "Cl" | "Br" | "I" => &[1],
        _ => return false,
    };
    let free = usual.iter().find(|&&v| v >= used).map(|v| v - used);
    a.chg == 0 && a.iso == 0 && a.rad == 0 && free == Some(u32::from(a.h))
}

/// The label a molfile symbol stands for when the record has no place for it: an R group (`R#`, `R`, `R1` ...) or, counted in
/// `info`, a query atom, which is read as an unnumbered R group.
fn stand_in(symbol: &str, info: &mut Imported) -> Option<String> {
    if symbol == "R#" {
        return Some("R".into());
    }
    if matches!(label::kind(symbol), Some(Kind::R(_))) {
        return Some(symbol.into());
    }
    let list = symbol.starts_with('[') || symbol.starts_with("NOT[");
    (list || QUERY_ATOMS.contains(&symbol)).then(|| {
        info.query_atoms += 1;
        "R".into()
    })
}

/// One record of a molfile (V2000 or V3000) made readable for the strict reader of `mzlab-mol`: R groups and query atoms become
/// the stand-in element and come back as (atom index, label); query bonds become single bonds; S-groups are counted. What was
/// changed is counted in `info`. A text that is not a molfile comes back as it is (the reader says what is wrong).
pub(crate) fn prepare(record: &str, info: &mut Imported) -> (String, Vec<(usize, String)>) {
    // what is left of the `$$$$` line of the record before
    let record = (record.strip_prefix("\r\n"))
        .or(record.strip_prefix('\n'))
        .unwrap_or(record);
    let mut lines: Vec<String> = record.lines().map(str::to_string).collect();
    let mut labels: Vec<(usize, String)> = Vec::new();
    let number = |labels: &mut Vec<(usize, String)>, atom: usize, n: &str| {
        let lab = format!("R{}", n.trim_end_matches(')'));
        if let (Some(l), Some(Kind::R(_))) =
            (labels.iter_mut().find(|l| l.0 == atom), label::kind(&lab))
        {
            l.1 = lab;
        }
    };
    let count = |l: &str, k: usize| l.get(k..k + 3).and_then(|f| f.trim().parse::<usize>().ok());
    let counts = lines.get(3).cloned().unwrap_or_default();
    if counts.contains("V3000") {
        let (mut block, mut k) = (' ', 0);
        for line in &mut lines {
            let Some(body) = line.strip_prefix("M  V30 ") else {
                continue;
            };
            let mut f: Vec<&str> = body.split_whitespace().collect();
            match (body.trim(), block) {
                ("BEGIN ATOM", _) => (block, k) = ('a', 0),
                ("BEGIN BOND", _) => block = 'b',
                ("BEGIN SGROUP", _) => block = 's',
                (t, _) if t.starts_with("END ") => block = ' ',
                (_, 'a') => {
                    if let Some(lab) = f.get(1).and_then(|s| stand_in(s, info)) {
                        labels.push((k, lab));
                        if let Some(at) = f.iter().position(|t| t.starts_with("RGROUPS=(")) {
                            if let Some(n) = f.get(at + 1) {
                                number(&mut labels, k, n);
                            }
                            f.truncate(at);
                        }
                        f[1] = R_STAND_IN;
                        *line = format!("M  V30 {}", f.join(" "));
                    }
                    k += 1;
                }
                (_, 'b') => {
                    if matches!(f.get(1), Some(&("5" | "6" | "7" | "8"))) {
                        info.query_bonds += 1;
                        f[1] = "1";
                        *line = format!("M  V30 {}", f.join(" "));
                    }
                }
                (_, 's') => info.sgroups += 1,
                _ => {}
            }
        }
    } else if let (Some(atoms), Some(bonds)) = (count(&counts, 0), count(&counts, 3)) {
        for (i, line) in lines.iter_mut().enumerate().skip(4) {
            let k = i - 4;
            if k < atoms {
                let lab = line.get(31..34).and_then(|s| stand_in(s.trim(), info));
                if let Some(lab) = lab {
                    labels.push((k, lab));
                    line.replace_range(31..34, "He ");
                }
            } else if k < atoms + bonds {
                if matches!(line.get(6..9), Some("  5" | "  6" | "  7" | "  8")) {
                    info.query_bonds += 1;
                    line.replace_range(6..9, "  1");
                }
            } else if let Some(list) = line.strip_prefix("M  RGP") {
                let f: Vec<&str> = list.split_whitespace().skip(1).collect();
                for pair in f.chunks_exact(2) {
                    if let Some(atom) = pair[0].parse::<usize>().ok().and_then(|a| a.checked_sub(1))
                    {
                        number(&mut labels, atom, pair[1]);
                    }
                }
            } else if let Some(list) = line.strip_prefix("M  STY") {
                info.sgroups += count(list, 0).unwrap_or(1);
            }
        }
    }
    (lines.join("\n"), labels)
}

/// A SMILES with its R groups (`*`, `[*]`, `[*:1]`, as `Document::to_smiles` writes them) made readable for the reader of
/// `mzlab-mol`: each becomes the stand-in element and comes back as (atom index, label), the atoms counted in the order of the text.
pub(crate) fn r_atoms(smiles: &str) -> (String, Vec<(usize, String)>) {
    let (mut out, mut labels) = (String::new(), Vec::new());
    let b = smiles.as_bytes();
    let (mut i, mut copied, mut atoms) = (0, 0, 0);
    // up to the first blank: a CXSMILES tail or a name is not atoms
    while i < b.len() && !b[i].is_ascii_whitespace() {
        let mut end = i + 1;
        let star = match b[i] {
            b'[' => {
                end = smiles[i..].find(']').map_or(b.len(), |k| i + k + 1);
                // an atom left open is not repaired: the reader refuses it
                smiles[i..end].starts_with("[*") && smiles[i..end].ends_with(']')
            }
            b'C' | b'B' if matches!(b.get(end), Some(b'l' | b'r')) => {
                end += 1;
                false
            }
            b'*' => true,
            c if c.is_ascii_alphabetic() => false,
            _ => {
                i = end;
                continue;
            }
        };
        if star {
            let class = smiles[i..end].strip_prefix("[*:").unwrap_or("");
            let lab = format!("R{}", class.trim_end_matches(']'));
            let known = matches!(label::kind(&lab), Some(Kind::R(_)));
            labels.push((atoms, if known { lab } else { "R".into() }));
            out.push_str(&smiles[copied..i]);
            out.push_str("[He]");
            copied = end;
        }
        atoms += 1;
        i = end;
    }
    out.push_str(&smiles[copied..]);
    (out, labels)
}
