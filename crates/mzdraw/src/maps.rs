//! Atom-to-atom maps. The record of `mzlab-mol` (`mol/1`) has no field for them, so they travel beside it: read from the text of
//! a molfile (the `mmm` field of a V2000 atom line, `AAMAP` of a V3000 one), of a SMILES (the atom class `[CH3:4]`) or of a KET
//! (`mapping`) by this layer, and written back into the same places after `mzlab-mol` has written the rest. A map is a whole
//! number 1..=`MAX_MAP`; 0 is none. Formats that have no place for it are listed by `Document::maps_lost`.

use crate::doc::{Document, MAX_MAP};

/// A map number from the text of a field: a whole number in range, else none.
fn number(s: &str) -> u16 {
    s.trim()
        .parse::<u16>()
        .ok()
        .filter(|n| *n <= MAX_MAP)
        .unwrap_or(0)
}

/// The maps of the atoms of a molfile record (V2000 or V3000) as (atom index, map), the atoms counted in the order of the text.
/// A text that is not a molfile gives none (the reader of `mzlab-mol` says what is wrong).
pub(crate) fn of_molfile(record: &str) -> Vec<(usize, u16)> {
    let lines: Vec<&str> = record.lines().collect();
    let counts = lines.get(3).copied().unwrap_or_default();
    let mut out = Vec::new();
    if counts.contains("V3000") {
        let mut inside = false;
        let mut k = 0;
        for line in &lines {
            let body = line.strip_prefix("M  V30 ").unwrap_or("").trim();
            match body {
                "BEGIN ATOM" => (inside, k) = (true, 0),
                "END ATOM" => inside = false,
                _ if inside => {
                    // index, type, x, y, z, then the map
                    let n = body.split_whitespace().nth(5).map_or(0, number);
                    if n != 0 {
                        out.push((k, n));
                    }
                    k += 1;
                }
                _ => {}
            }
        }
    } else if let Some(atoms) = counts
        .get(0..3)
        .and_then(|c| c.trim().parse::<usize>().ok())
    {
        for (k, line) in lines.iter().skip(4).take(atoms).enumerate() {
            let n = line.get(60..63).map_or(0, number);
            if n != 0 {
                out.push((k, n));
            }
        }
    }
    out
}

/// The maps of a SMILES as (atom index, map): the class of a bracket atom, `[CH3:4]`. The atoms are counted in the order of the
/// text, as `formats::r_atoms` counts them; the class of `[*:n]` is an R-group number, not a map.
pub(crate) fn of_smiles(smiles: &str) -> Vec<(usize, u16)> {
    let (mut out, b) = (Vec::new(), smiles.as_bytes());
    let (mut i, mut atoms) = (0, 0);
    while i < b.len() && !b[i].is_ascii_whitespace() {
        let mut end = i + 1;
        match b[i] {
            b'[' => {
                end = smiles[i..].find(']').map_or(b.len(), |k| i + k + 1);
                let token = &smiles[i..end];
                let class = (!token.starts_with("[*") && token.ends_with(']'))
                    .then(|| token[..token.len() - 1].rsplit_once(':'))
                    .flatten()
                    .map(|(_, class)| class)
                    .filter(|c| c.bytes().all(|b| b.is_ascii_digit()));
                if let Some(n) = class.map(number).filter(|&n| n != 0) {
                    out.push((atoms, n));
                }
            }
            b'C' | b'B' if matches!(b.get(end), Some(b'l' | b'r')) => end += 1,
            c if c.is_ascii_alphabetic() || c == b'*' => {}
            _ => {
                i = end;
                continue;
            }
        }
        atoms += 1;
        i = end;
    }
    out
}

impl Document {
    /// Gives the atoms the maps read from a file, (atom index, map).
    pub(crate) fn set_maps(&mut self, maps: &[(usize, u16)]) {
        for &(i, n) in maps {
            if let Some(a) = self.atoms.get_mut(i) {
                a.map = n;
            }
        }
    }

    /// The atoms that carry a map.
    pub fn mapped(&self) -> usize {
        self.atoms.iter().filter(|a| a.map != 0).count()
    }

    /// How many map numbers the file format `kind` (a name of `Editor::save`) cannot hold: all of them for a format without a
    /// place for them, else none. A page or a CLI says it: nothing is dropped in silence.
    pub fn maps_lost(&self, kind: &str) -> usize {
        match kind {
            "mzdraw" | "svg" | "molfile" | "molfile-v3000" | "smiles" | "ket" | "rxn" | "sdf" => 0,
            _ => self.mapped(),
        }
    }

    /// The map numbers written into the atom lines of a V2000 molfile of this document (the field `mmm`, columns 61 to 63). The
    /// atoms of the pool come first in the molfile, in pool order, so atom `i` stands on line `4 + i`.
    pub(crate) fn mark_maps(&self, molfile: String) -> String {
        if self.mapped() == 0 || !molfile.contains("V2000") {
            return molfile;
        }
        let mut lines: Vec<String> = molfile.split('\n').map(str::to_string).collect();
        for (i, a) in self.atoms.iter().enumerate().filter(|(_, a)| a.map != 0) {
            if let Some(line) = lines.get_mut(4 + i).filter(|l| l.len() >= 63) {
                line.replace_range(60..63, &format!("{:>3}", a.map));
            }
        }
        lines.join("\n")
    }
}
