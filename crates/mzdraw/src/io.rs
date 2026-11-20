//! Bridges between the document and the `mol/1` record of `mzlab-mol`, whose readers and writers do all the format work: SMILES and
//! molfile in, molfile out. No source file is parsed here. The wedge marks cross the bridge through the molfile `mzlab-mol` writes:
//! on the way out the marks as drawn are set on its bond lines (the record's reader then derives the stereo by its own rules), on the
//! way in the marks are read from the molfile the record writes for its own stereo on our coordinates (`wedges_from_record`).

use crate::doc::{round4, Atom, Bond, Document, WEDGE_DOWN, WEDGE_UP, WEDGE_WAVY};
use crate::label;
use crate::layout;
use crate::m;
use mzlab_mol::{Coords, Mol};

/// The element that stands in for an R group in the record, which has no place for one; the molfile writer of this module
/// replaces it on the atom line.
const R_STAND_IN: &str = "He";

/// How an import went: the format read and whether the positions are placeholders (the source had no coordinates).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Imported {
    pub format: &'static str,
    pub placeholder: bool,
}

impl Document {
    /// The document as a `mol/1` record: atoms in pool order, hydrogens as the labels show them, the drawn coordinates as a layer.
    /// An abbreviation is expanded: its attachment atom takes the place of the labelled atom and its other atoms follow the pool,
    /// with placeholder positions next to it. An R group has no place in the record: a helium stands in (see `to_molfile`).
    pub fn to_mol(&self) -> Mol {
        let mut atoms: Vec<mzlab_mol::Atom> = Vec::new();
        let mut xy: Vec<Vec<f64>> = Vec::new();
        let mut extra: Vec<(usize, Mol, Document)> = Vec::new();
        for (i, a) in self.atoms.iter().enumerate() {
            xy.push(vec![a.x, a.y]);
            let group = match label::kind(&a.lab) {
                Some(label::Kind::Group(smiles)) => Mol::from_smiles(smiles).ok(),
                _ => None,
            };
            if let Some(g) = group.filter(|g| !g.atoms.is_empty()) {
                // the bonds drawn on the label use the free valence of the attachment atom: its hydrogens, then the unpaired
                // electrons a bracket atom of the table stands with on its own
                let mut first = g.atoms[0].clone();
                let used = self.order_sum(a.id).min(255) as u8;
                first.rad = first.rad.saturating_sub(used.saturating_sub(first.h));
                first.h = first.h.saturating_sub(used);
                atoms.push(first);
                let (placed, _) = Document::from_mol(&g);
                extra.push((i, g, placed));
                continue;
            }
            let r = !a.lab.is_empty();
            atoms.push(mzlab_mol::Atom {
                el: if r { R_STAND_IN.into() } else { a.el.clone() },
                iso: if r { 0 } else { a.iso },
                chg: if r { 0 } else { a.chg },
                rad: if r { 0 } else { a.rad },
                h: self.hydrogens(a) as u8,
            });
        }
        let at = |id: u32| self.atom_index(id).unwrap_or(0) as u32;
        let mut bonds: Vec<mzlab_mol::Bond> = self
            .bonds
            .iter()
            .map(|b| {
                let (i, j) = (at(b.a), at(b.b));
                mzlab_mol::Bond {
                    a: i.min(j),
                    b: i.max(j),
                    o: b.o,
                }
            })
            .collect();
        for (i, g, placed) in &extra {
            // atom 0 of the group is the labelled atom; the others are appended
            let base = atoms.len();
            let index = |k: u32| if k == 0 { *i } else { base + k as usize - 1 } as u32;
            let (ox, oy) = (
                self.atoms[*i].x - placed.atoms[0].x,
                self.atoms[*i].y - placed.atoms[0].y,
            );
            for (k, a) in g.atoms.iter().enumerate().skip(1) {
                atoms.push(a.clone());
                xy.push(vec![
                    round4(placed.atoms[k].x + ox),
                    round4(placed.atoms[k].y + oy),
                ]);
            }
            for b in &g.bonds {
                let (p, q) = (index(b.a), index(b.b));
                bonds.push(mzlab_mol::Bond {
                    a: p.min(q),
                    b: p.max(q),
                    o: b.o,
                });
            }
        }
        let mut m = Mol::new(atoms, bonds, Vec::new(), Vec::new());
        if !self.atoms.is_empty() {
            m.coords = Some(Coords {
                dim: 2,
                xy,
                from: crate::NAME.into(),
            });
        }
        m
    }

    /// A document from a record: ids 1.. in record order; coordinates scaled so that the median bond is 1. Returns whether the
    /// positions are placeholders.
    pub fn from_mol(m: &Mol) -> (Document, bool) {
        let n = m.atoms.len() as u32;
        let mut d = Document {
            next: n + m.bonds.len() as u32 + 1,
            ..Document::default()
        };
        for (i, a) in m.atoms.iter().enumerate() {
            let p = m.coords.as_ref().and_then(|c| c.xy.get(i));
            let (x, y) = p.map_or((0.0, 0.0), |p| (p[0], p[1]));
            d.atoms.push(Atom {
                id: i as u32 + 1,
                el: a.el.clone(),
                x,
                y,
                chg: a.chg,
                iso: a.iso,
                rad: a.rad,
                h: None,
                lab: String::new(),
            });
        }
        for (i, b) in m.bonds.iter().enumerate() {
            d.bonds.push(Bond {
                id: n + i as u32 + 1,
                a: b.a + 1,
                b: b.b + 1,
                o: b.o,
                s: 0,
            });
        }
        // keep a hydrogen count only where the valence rules would show another one
        for (i, a) in m.atoms.iter().enumerate() {
            if d.hydrogens(&d.atoms[i]) != a.h as u32 {
                d.atoms[i].h = Some(a.h);
            }
        }
        let placeholder = m.coords.is_none() && !m.atoms.is_empty();
        if placeholder {
            layout::placeholder(&mut d);
        } else {
            d.normalise();
        }
        d.wedges_from_record(m);
        (d, placeholder)
    }

    /// The stereo of a record as wedge marks on this drawing: the record, with the coordinates of the drawing, is written as a
    /// molfile by `mzlab-mol` (one wedge per defined centre, a wavy mark per unknown one, chosen by its own parity rule) and the
    /// marks of its bond lines are taken. A record without stereo, or too large for V2000, leaves the drawing unmarked.
    fn wedges_from_record(&mut self, m: &Mol) {
        if m.stereo.is_empty() {
            return;
        }
        let mut rec = m.clone();
        rec.coords = Some(Coords {
            dim: 2,
            xy: self.atoms.iter().map(|a| vec![a.x, a.y]).collect(),
            from: crate::NAME.into(),
        });
        let text = rec.to_molfile(crate::NAME);
        for (i, j, _, mark) in bond_lines(&text, self.atoms.len(), self.bonds.len()) {
            let (Some(a), Some(b)) = (self.atoms.get(i), self.atoms.get(j)) else {
                continue;
            };
            let (a, b) = (a.id, b.id);
            let s = match mark {
                1 => WEDGE_UP,
                6 => WEDGE_DOWN,
                4 => WEDGE_WAVY,
                _ => continue,
            };
            if let Some(bond) = self
                .bonds
                .iter_mut()
                .find(|x| x.o == 1 && ((x.a == a && x.b == b) || (x.a == b && x.b == a)))
            {
                (bond.a, bond.b, bond.s) = (a, b, s);
            }
        }
    }

    /// Sets the wedge marks as drawn on the bond lines of a V2000 molfile of this document (bond lines in pool order): the first
    /// atom of a marked line is the narrow end. A V3000 file (more than 999 atoms or bonds) is left without marks.
    fn mark_wedges(&self, molfile: String, record_atoms: usize) -> String {
        if self.bonds.iter().all(|b| b.s == 0) || !molfile.contains("V2000") {
            return molfile;
        }
        let first = 4 + record_atoms;
        let mut lines: Vec<String> = molfile.split('\n').map(str::to_string).collect();
        for (k, b) in self.bonds.iter().enumerate().filter(|(_, b)| b.s != 0) {
            let (Some(i), Some(j)) = (self.atom_index(b.a), self.atom_index(b.b)) else {
                continue;
            };
            let Some(line) = lines.get_mut(first + k).filter(|l| l.len() >= 12) else {
                continue;
            };
            let mark = match b.s {
                WEDGE_UP => 1,
                WEDGE_DOWN => 6,
                _ => 4,
            };
            *line = format!(
                "{:>3}{:>3}{:>3}{:>3}{}",
                i + 1,
                j + 1,
                b.o,
                mark,
                &line[12..]
            );
        }
        lines.join("\n")
    }

    /// The document as a record with its stereo: the molfile as drawn, read back by `mzlab-mol` (its rules decide what the wedges
    /// mean). A drawing its reader refuses gives the record without stereo.
    pub fn to_mol_stereo(&self) -> Mol {
        Mol::from_molfile(&self.to_molfile()).unwrap_or_else(|_| self.to_mol())
    }

    /// Scales the coordinates so that the median bond length is 1 (a drawing without bonds is left alone).
    fn normalise(&mut self) {
        let mut len: Vec<f64> = self
            .bonds
            .iter()
            .filter_map(|b| Some((self.atom(b.a)?, self.atom(b.b)?)))
            .map(|(p, q)| m::hypot(p.x - q.x, p.y - q.y))
            .collect();
        len.sort_by(f64::total_cmp);
        let k = len
            .get(len.len() / 2)
            .copied()
            .filter(|&m| m > 1e-6)
            .map_or(1.0, |m| 1.0 / m);
        for a in &mut self.atoms {
            a.x = round4(a.x * k);
            a.y = round4(a.y * k);
        }
    }

    /// The whole canvas as one molfile (V2000, or V3000 when it has to be), written by `mzlab-mol`, with the wedge marks as drawn,
    /// abbreviations expanded and R groups as `R#` atoms with their `M  RGP` numbers.
    pub fn to_molfile(&self) -> String {
        self.molfile(false)
    }

    /// The molfile for the structure check: as `to_molfile`, but an R group is the query atom `A` (any atom), which the check reads.
    pub fn to_molfile_for_check(&self) -> String {
        self.molfile(true)
    }

    fn molfile(&self, r_as_query: bool) -> String {
        let m = self.to_mol();
        let text = self.mark_wedges(m.to_molfile(crate::NAME), m.atoms.len());
        self.mark_r_groups(text, r_as_query)
    }

    /// Writes the R groups on the atom lines of a V2000 molfile of this document (the stand-in element becomes `R#`, or `A` for
    /// the check) and their numbers as `M  RGP` lines. A V3000 file keeps the stand-in.
    fn mark_r_groups(&self, molfile: String, r_as_query: bool) -> String {
        let groups: Vec<(usize, u8)> = self
            .atoms
            .iter()
            .enumerate()
            .filter_map(|(i, a)| match label::kind(&a.lab) {
                Some(label::Kind::R(n)) => Some((i, n)),
                _ => None,
            })
            .collect();
        if groups.is_empty() || !molfile.contains("V2000") {
            return molfile;
        }
        let mut lines: Vec<String> = molfile.split('\n').map(str::to_string).collect();
        for &(i, n) in &groups {
            let symbol = match (r_as_query, n) {
                (true, _) => "A  ",
                (false, 0) => "R  ",
                (false, _) => "R# ",
            };
            if let Some(line) = lines.get_mut(4 + i).filter(|l| l.len() >= 34) {
                line.replace_range(31..34, symbol);
            }
        }
        let numbered: Vec<String> = groups
            .iter()
            .filter(|g| g.1 > 0 && !r_as_query)
            .map(|&(i, n)| format!("M  RGP  1{:>4}{:>4}", i + 1, n))
            .collect();
        if let Some(end) = lines.iter().position(|l| l == "M  END") {
            lines.splice(end..end, numbered);
        }
        lines.join("\n")
    }
}

/// The bond lines of a V2000 molfile written by `mzlab-mol`, as (first atom, second atom, order, stereo mark), atoms from 0.
fn bond_lines(molfile: &str, atoms: usize, bonds: usize) -> Vec<(usize, usize, u8, u8)> {
    if !molfile.contains("V2000") {
        return Vec::new();
    }
    let field = |l: &str, k: usize| l.get(k..k + 3).and_then(|f| f.trim().parse::<usize>().ok());
    molfile
        .split('\n')
        .skip(4 + atoms)
        .take(bonds)
        .filter_map(|l| {
            Some((
                field(l, 0)?.checked_sub(1)?,
                field(l, 3)?.checked_sub(1)?,
                field(l, 6)? as u8,
                field(l, 9)? as u8,
            ))
        })
        .collect()
}

/// What a saved `mzdraw/1` document holds, as (atoms, bonds, objects), for the offer to restore it; `None` for a text that is
/// not a readable document or for an empty one (nothing worth offering).
pub fn saved_summary(text: &str) -> Option<(usize, usize, usize)> {
    let d = Document::from_json(text.trim()).ok()?;
    (!d.is_empty()).then_some((d.atoms.len(), d.bonds.len(), d.objects.len()))
}

/// Reads a structure text: an `mzdraw/1` document, a molfile (V2000/V3000) or a SMILES. The error is an i18n key.
pub fn import(text: &str) -> Result<(Document, Imported), &'static str> {
    let t = text.trim();
    if t.starts_with('{') {
        let d = Document::from_json(t)?;
        return Ok((
            d,
            Imported {
                format: "mzdraw",
                placeholder: false,
            },
        ));
    }
    if text.contains("M  END") {
        let m = Mol::from_molfile(text).map_err(|_| "error.draw2.molfile")?;
        let (d, placeholder) = Document::from_mol(&m);
        return Ok((
            d,
            Imported {
                format: "molfile",
                placeholder,
            },
        ));
    }
    if t.is_empty() || t.lines().count() > 1 {
        return Err("error.draw2.import");
    }
    let m = Mol::from_smiles(t).map_err(|_| "error.draw2.smiles")?;
    let (d, placeholder) = Document::from_mol(&m);
    Ok((
        d,
        Imported {
            format: "smiles",
            placeholder,
        },
    ))
}
