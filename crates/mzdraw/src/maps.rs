//! Atom-to-atom maps. The record of `mzlab-mol` (`mol/1`) has no field for them, so they travel beside it: read from the text of
//! a molfile (the `mmm` field of a V2000 atom line, `AAMAP` of a V3000 one), of a SMILES (the atom class `[CH3:4]`) or of a KET
//! (`mapping`) by this layer, and written back into the same places after `mzlab-mol` has written the rest. A map is a whole
//! number 1..=`MAX_MAP`; 0 is none. Formats that have no place for it are listed by `Document::maps_lost`.

use crate::doc::{Document, MAX_MAP};
use crate::io::Imported;

/// A map number from the text of a field: `Ok(0)` for an empty field or 0 (none), `Ok(n)` for a whole number 1..=`MAX_MAP`,
/// `Err` for anything else (negative, above `MAX_MAP`, not a number): the caller counts it (`Imported::maps_left`).
fn number(s: &str) -> Result<u16, ()> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(0);
    }
    s.parse::<u16>().ok().filter(|n| *n <= MAX_MAP).ok_or(())
}

/// The maps of the atoms of a molfile record (V2000 or V3000) as (atom index, map), the atoms counted in the order of the text.
/// A text that is not a molfile gives none (the reader of `mzlab-mol` says what is wrong).
pub(crate) fn of_molfile(record: &str) -> (Vec<(usize, u16)>, usize) {
    let lines: Vec<&str> = record.lines().collect();
    let counts = lines.get(3).copied().unwrap_or_default();
    let (mut out, mut bad) = (Vec::new(), 0);
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
                    match body.split_whitespace().nth(5).map_or(Ok(0), number) {
                        Ok(0) => {}
                        Ok(n) => out.push((k, n)),
                        Err(()) => bad += 1,
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
            match line.get(60..63).map_or(Ok(0), number) {
                Ok(0) => {}
                Ok(n) => out.push((k, n)),
                Err(()) => bad += 1,
            }
        }
    }
    (out, bad)
}

/// The maps of a SMILES as (atom index, map): the class of a bracket atom, `[CH3:4]`. The atoms are counted in the order of the
/// text, as `formats::r_atoms` counts them; the class of `[*:n]` is an R-group number, not a map.
pub(crate) fn of_smiles(smiles: &str) -> (Vec<(usize, u16)>, usize) {
    let (mut out, mut bad, b) = (Vec::new(), 0, smiles.as_bytes());
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
                    .map(|(_, class)| class);
                match class.map(|c| {
                    if c.bytes().all(|b| b.is_ascii_digit()) {
                        number(c)
                    } else {
                        Err(())
                    }
                }) {
                    None | Some(Ok(0)) => {}
                    Some(Ok(n)) => out.push((atoms, n)),
                    Some(Err(())) => bad += 1,
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
    (out, bad)
}

/// The outputs of `Editor::save` that have no place for a map number.
pub const MAPS_LOST_IN: [&str; 2] = ["cdxml", "cml"];

/// A V3000 atom line (`M  V30 index symbol x y z aamap ...`) with the map number in the `aamap` field; a line that stops before that
/// field is completed with zeros first, so the number is never left out.
pub(crate) fn v3000_line_with_map(line: &str, map: u16) -> String {
    let mut f: Vec<&str> = line.split(' ').collect();
    while f.len() <= 8 {
        f.push("0");
    }
    let map = map.to_string();
    f[8] = &map;
    f.join(" ")
}

impl Document {
    /// Gives the atoms the maps read from a file, (atom index, map), and counts in `info` what was refused: `bad` fields that
    /// are no map number (`maps_left`) and the structures that carry one number twice (`maps_duplicate`).
    pub(crate) fn set_maps(
        &mut self,
        (maps, bad): (Vec<(usize, u16)>, usize),
        info: &mut Imported,
    ) {
        info.maps_left += bad;
        for (i, n) in maps {
            if let Some(a) = self.atoms.get_mut(i) {
                a.map = n;
            }
        }
        info.maps_duplicate += self.duplicate_maps();
    }

    /// How many structures (connected pieces) carry one map number on more than one atom.
    pub fn duplicate_maps(&self) -> usize {
        let pieces = crate::layout::pieces(self);
        let mut piece_of = std::collections::HashMap::with_capacity(self.atoms.len());
        for (k, ids) in pieces.iter().enumerate() {
            for id in ids {
                piece_of.insert(*id, k);
            }
        }
        let mut seen: Vec<Vec<u16>> = vec![Vec::new(); pieces.len()];
        for a in self.atoms.iter().filter(|a| a.map != 0) {
            if let Some(&k) = piece_of.get(&a.id) {
                seen[k].push(a.map);
            }
        }
        seen.into_iter()
            .filter(|maps| {
                let mut maps = maps.clone();
                let n = maps.len();
                maps.sort_unstable();
                maps.dedup();
                maps.len() < n
            })
            .count()
    }

    /// The atoms that carry a map.
    pub fn mapped(&self) -> usize {
        self.atoms.iter().filter(|a| a.map != 0).count()
    }

    /// How many map numbers the output `kind` of `Editor::save` cannot hold: all of them for a format without a place for them
    /// (`MAPS_LOST_IN`), else none. `replay` reports it in `lost` and the page says it after a save: nothing is dropped in silence.
    pub fn maps_lost(&self, kind: &str) -> usize {
        if MAPS_LOST_IN.contains(&kind) {
            self.mapped()
        } else {
            0
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
            if let Some(line) = lines.get_mut(4 + i) {
                // the writer of `mzlab-mol` gives full atom lines; a short one is padded, never left without the number
                while line.len() < 63 {
                    line.push(' ');
                }
                line.replace_range(60..63, &format!("{:>3}", a.map));
            }
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::v3000_line_with_map;

    #[test]
    fn the_number_goes_into_the_aamap_field_of_a_full_or_a_short_line() {
        assert_eq!(
            v3000_line_with_map("M  V30 1 C 0 0 0 0", 5),
            "M  V30 1 C 0 0 0 5"
        );
        assert_eq!(
            v3000_line_with_map("M  V30 1 C 0 0 0 0 CHG=1", 12),
            "M  V30 1 C 0 0 0 12 CHG=1"
        );
        // short lines: completed, never left without the number
        assert_eq!(
            v3000_line_with_map("M  V30 1 C 0 0 0", 5),
            "M  V30 1 C 0 0 0 5"
        );
        assert_eq!(v3000_line_with_map("M  V30 1 C", 7), "M  V30 1 C 0 0 0 7");
    }
}
