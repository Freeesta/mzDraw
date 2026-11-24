//! Bridges between the document and the `mol/1` record of `mzlab-mol`, whose readers and writers do all the format work: SMILES and
//! molfile in, molfile out. No source file is parsed here. The wedge marks cross the bridge through the molfile `mzlab-mol` writes:
//! on the way out the marks as drawn are set on its bond lines (the record's reader then derives the stereo by its own rules), on the
//! way in the marks are read from the molfile the record writes for its own stereo on our coordinates (`wedges_from_record`).

use crate::doc::{round4, Atom, Bond, Document, WEDGE_DOWN, WEDGE_UP, WEDGE_WAVY};
use crate::formats;
use crate::label;
use crate::layout;
use crate::m;
use mzlab_mol::{Coords, Mol};

/// The element that stands in for an R group in the record, which has no place for one; the molfile writer of this module
/// replaces it on the atom line.
pub(crate) const R_STAND_IN: &str = "He";

/// What an import read, and what it could not keep (the page says so: nothing is dropped in silence).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Imported {
    /// `mzdraw`, `svg` or `png` (a picture carrying its document), `ket`, `cdxml`, `cdx` (binary CDX), `cml` (CML or a Marvin document), `rxn` (a reaction file or an RD file), `molfile` or `smiles`.
    pub format: &'static str,
    /// The positions are placeholders (the text had no coordinates).
    pub placeholder: bool,
    /// Records of a molfile or SDF that were read.
    pub records: usize,
    /// Records of an SDF beyond `MAX_RECORDS`, not read.
    pub skipped: usize,
    /// Query atoms (`A`, `Q`, `*`, atom lists ...), read as unnumbered R groups.
    pub query_atoms: usize,
    /// Query bonds (types 5 to 8), read as single bonds.
    pub query_bonds: usize,
    /// S-groups, not read.
    pub sgroups: usize,
    /// Data fields of an SDF a document cannot hold (too many, too long, a name with `<` or `>`), left out.
    pub data_left: usize,
    /// Things of a file mzDraw does not draw (shapes, images, R-group definitions ...), left out.
    pub other: usize,
    /// Enhanced-stereo groups "and" / "or": the centres are drawn with their wedges, the grouping is not kept.
    pub stereo_groups: usize,
    /// Fields that should hold an atom-to-atom map and hold no whole number 1..=999 (negative, above 999, not a number): read as no map.
    pub maps_left: usize,
    /// Structures with one map number on more than one atom: kept as read.
    pub maps_duplicate: usize,
    /// Binary CDX: text runs of a font whose code page is not decoded (read as Latin-1, so some characters may be wrong).
    pub code_page_runs: usize,
    /// Binary CDX: properties of tags the SDK pages read do not list (styles, colours, widths), skipped by their length. Counted,
    /// not said on the page: every file of ChemDraw has dozens and they are no content.
    pub props_skipped: usize,
    /// A TP table (tp-09) was read as a list: its computed columns (`tp_list::DERIVED`), not read; they are calculated again.
    pub table_ignored: usize,
    /// Text runs a file holds that a text cannot (a size outside 0.25 to 8 times the font, a colour or font name that is not one, a
    /// text too long): clamped or left out, counted.
    pub runs_adjusted: usize,
    /// Plain texts of a file longer than the 200 characters of a text line: cut, counted.
    pub text_cut: usize,
    /// SDF records that carry the name of a TP (`TP_ID`): an SDF has no arrows, so they were read without a pathway.
    pub no_pathway: usize,
    /// Lines of a SMILES file (counted from 1) that are no SMILES: said with their number, the other lines are read.
    pub smiles_lines: Vec<usize>,
    /// Names of a SMILES file longer than the 80 characters of a title line: cut, counted.
    pub names_cut: usize,
    /// Reaction SMILES lines whose CXSMILES tail (`|...|`) was not read: the reaction is drawn without it (`draw2.import.reactionTail`).
    pub reaction_tails: usize,
    /// CDXML: bonds that carried a display (`Dash`, `Hash`) or the side of a double bond, in a file where the reader refused a bond
    /// (so the bonds of the file and of the drawing cannot be paired): the display of every bond is left out, counted.
    pub bond_display_left: usize,
    /// The dative (type 9) and hydrogen (type 10) bonds of the record being read, as (donor atom, other atom, kind), atoms from 0:
    /// read as single bonds by the reader of `mzlab-mol`, which has no such bond; `labelled` gives them their kind and direction
    /// back. Empty between records.
    pub links: Vec<(usize, usize, &'static str)>,
}

impl Imported {
    /// What the page says after the import, as (i18n key, count): only what is worth saying.
    pub fn notes(&self) -> Vec<(&'static str, usize)> {
        let more = self.smiles_lines.len().saturating_sub(MAX_SAID);
        let lines = self.smiles_lines.iter().take(MAX_SAID);
        [
            ("draw2.import.placeholder", usize::from(self.placeholder)),
            (
                "draw2.import.records",
                if self.records > 1 { self.records } else { 0 },
            ),
            ("draw2.import.skipped", self.skipped),
            ("draw2.import.queryAtoms", self.query_atoms),
            ("draw2.import.queryBonds", self.query_bonds),
            ("draw2.import.sgroups", self.sgroups),
            ("draw2.import.stereoGroups", self.stereo_groups),
            ("draw2.import.runsAdjusted", self.runs_adjusted),
            ("draw2.import.textCut", self.text_cut),
            ("draw2.import.other", self.other),
            ("draw2.import.dataLeft", self.data_left),
            ("draw2.import.mapsLeft", self.maps_left),
            ("draw2.import.mapsDuplicate", self.maps_duplicate),
            ("draw2.import.codePage", self.code_page_runs),
            ("draw2.import.tpTableIgnored", self.table_ignored),
            ("draw2.import.tpNoPathway", self.no_pathway),
            ("draw2.import.bondDisplay", self.bond_display_left),
        ]
        .into_iter()
        .chain(lines.map(|&n| ("draw2.import.smilesLine", n)))
        .chain([
            ("draw2.import.smilesMore", more),
            ("draw2.import.nameCut", self.names_cut),
            ("draw2.import.reactionTail", self.reaction_tails),
        ])
        .filter(|n| n.1 > 0)
        .collect()
    }
}

/// The most lines of a SMILES file that are named one by one in the notes; the others are counted (`draw2.import.smilesMore`).
pub const MAX_SAID: usize = 10;

/// Records of one SDF that are read into a drawing.
pub const MAX_RECORDS: usize = 64;

impl Document {
    /// The document as a `mol/1` record: atoms in pool order, hydrogens as the labels show them, the drawn coordinates as a layer.
    /// An abbreviation is expanded: its attachment atom takes the place of the labelled atom and its other atoms follow the pool,
    /// with placeholder positions next to it. An R group has no place in the record: a helium stands in (see `to_molfile`).
    pub fn to_mol(&self) -> Mol {
        let mut atoms: Vec<mzlab_mol::Atom> = Vec::new();
        let mut xy: Vec<Vec<f64>> = Vec::new();
        let mut extra: Vec<(usize, Mol, Document)> = Vec::new();
        let (sums, free) = (self.order_sums(), self.all_hydrogens());
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
                let used = sums.get(&a.id).copied().unwrap_or(0).min(255) as u8;
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
                h: free[i] as u8,
            });
        }
        let index = self.index_map();
        let at = |id: u32| index.get(&id).copied().unwrap_or(0) as u32;
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
                map: 0,
                more: Vec::new(),
            });
        }
        for (i, b) in m.bonds.iter().enumerate() {
            d.bonds.push(Bond {
                id: n + i as u32 + 1,
                a: b.a + 1,
                b: b.b + 1,
                o: b.o,
                s: 0,
                more: Vec::new(),
            });
        }
        // keep a hydrogen count only where the valence rules would show another one
        let shown = d.all_hydrogens();
        for (i, a) in m.atoms.iter().enumerate() {
            if shown[i] != a.h as u32 {
                d.atoms[i].h = Some(a.h);
            }
        }
        let placeholder = m.coords.is_none() && !m.atoms.is_empty();
        if placeholder {
            layout::computed(&mut d, m);
        } else {
            d.normalise();
        }
        d.wedges_from_record(m);
        // one atom alone has no layout to speak of: its position is not a placeholder worth a word
        (d, placeholder && m.atoms.len() > 1)
    }

    /// The stereo of a record as wedge marks on this drawing: the record, with the coordinates of the drawing, is written as a
    /// molfile by `mzlab-mol` (one wedge per defined centre, a wavy mark per unknown one, chosen by its own parity rule) and the
    /// marks of its bond lines are taken. A record without stereo, or too large for V2000, leaves the drawing unmarked.
    fn wedges_from_record(&mut self, m: &Mol) {
        let mut rec = with_unknown_double_bonds(m);
        if rec.stereo.is_empty() {
            return;
        }
        // the stereo groups would make the writer choose V3000; the marks are the same without them
        rec.groups.clear();
        rec.coords = Some(Coords {
            dim: 2,
            xy: self.atoms.iter().map(|a| vec![a.x, a.y]).collect(),
            from: crate::NAME.into(),
        });
        let text = rec.to_molfile(crate::NAME);
        // the first single bond between two atoms, by the pair; and the first double bond
        let mut pairs: std::collections::HashMap<(u32, u32), usize> =
            std::collections::HashMap::new();
        let mut doubles: std::collections::HashMap<(u32, u32), usize> =
            std::collections::HashMap::new();
        for (k, x) in self.bonds.iter().enumerate().rev() {
            match x.o {
                1 => pairs.insert((x.a.min(x.b), x.a.max(x.b)), k),
                2 => doubles.insert((x.a.min(x.b), x.a.max(x.b)), k),
                _ => None,
            };
        }
        for (i, j, order, mark) in bond_lines(&text, self.atoms.len(), self.bonds.len()) {
            let (Some(a), Some(b)) = (self.atoms.get(i), self.atoms.get(j)) else {
                continue;
            };
            let (a, b) = (a.id, b.id);
            // a double bond whose E/Z the record does not know: "either"
            if order == 2 && mark == 3 {
                if let Some(&k) = doubles.get(&(a.min(b), a.max(b))) {
                    crate::bonddisp::ez_put(&mut self.bonds[k].more, true);
                }
                continue;
            }
            let s = match mark {
                1 => WEDGE_UP,
                6 => WEDGE_DOWN,
                4 => WEDGE_WAVY,
                _ => continue,
            };
            if let Some(&k) = pairs.get(&(a.min(b), a.max(b))) {
                let bond = &mut self.bonds[k];
                (bond.a, bond.b, bond.s) = (a, b, s);
            }
        }
    }
}

/// The smallest ring a double bond can be in and still carry E/Z (as `mzlab-mol` counts it).
const MIN_EZ_RING: usize = 8;

/// The record with an `unknown` element on each double bond that can carry E/Z and has none (a SMILES `CC=CC`, a molfile double
/// bond marked "either"): drawn, its geometry would say E or Z, so the drawing marks it "either". Tetrahedral centres are left as
/// they are. A double bond can carry E/Z when it is in no ring smaller than `MIN_EZ_RING`, neither end has another double bond
/// (an allene), and each end has one other neighbour (or, with none, one hydrogen: the =NH of an imine), or two of different
/// constitution (`classes`). Two substituents that differ
/// only through stereo elsewhere are not seen (no mark: the drawing says what it said before). Linear in practice, unlike a full
/// search of the stereo sites, which can try every assignment of the other sites.
fn with_unknown_double_bonds(m: &Mol) -> Mol {
    use mzlab_mol::{Status, Stereo};
    let has = |a: u32, b: u32| {
        m.stereo
            .iter()
            .any(|s| matches!(s, Stereo::Db { a: x, b: y, .. } if (*x, *y) == (a.min(b), a.max(b))))
    };
    let n = m.atoms.len();
    let mut adj: Vec<Vec<(usize, u8)>> = vec![Vec::new(); n];
    for b in &m.bonds {
        let (a, c) = (b.a as usize, b.b as usize);
        if a < n && c < n {
            adj[a].push((c, b.o));
            adj[c].push((a, b.o));
        }
    }
    // an end has a substituent besides the other end: an atom, or a lone hydrogen facing a lone pair (the =NH of an imine)
    let lone_h = |at: usize| m.atoms.get(at).is_some_and(|x| x.h == 1);
    let bare = |at: usize, other: usize| adj[at].iter().any(|x| x.0 != other) || lone_h(at);
    let mut rec = m.clone();
    let open: Vec<(usize, usize)> = (m.bonds.iter())
        .filter(|b| b.o == 2 && (b.a as usize) < n && (b.b as usize) < n && !has(b.a, b.b))
        .map(|b| (b.a as usize, b.b as usize))
        .filter(|&(a, b)| bare(a, b) && bare(b, a))
        .collect();
    if open.is_empty() {
        return rec;
    }
    let cls = classes(m, &adj);
    let end_fits = |at: usize, other: usize| {
        let subs: Vec<usize> = adj[at]
            .iter()
            .filter(|x| x.0 != other)
            .map(|x| x.0)
            .collect();
        let cumulated = adj[at].iter().any(|x| x.0 != other && x.1 == 2);
        !cumulated
            && match subs[..] {
                [] => lone_h(at),
                [_] => true,
                [x, y] => cls[x] != cls[y],
                _ => false,
            }
    };
    for (a, b) in open {
        if end_fits(a, b) && end_fits(b, a) && !in_small_ring(&adj, a, b) {
            rec.stereo.push(Stereo::Db {
                a: a.min(b) as u32,
                b: a.max(b) as u32,
                refs: None,
                cfg: None,
                status: Status::Unknown,
            });
        }
    }
    rec
}

/// The constitution class of each atom: element, charge, mass number, hydrogens and bonds, refined by the classes of the
/// neighbours until no class splits (stereo is not looked at).
fn classes(m: &Mol, adj: &[Vec<(usize, u8)>]) -> Vec<usize> {
    let rank = |keys: Vec<String>| -> Vec<usize> {
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        keys.iter()
            .map(|k| sorted.binary_search(k).unwrap_or(0))
            .collect()
    };
    let mut cls = rank(
        (m.atoms.iter().zip(adj))
            .map(|(a, nb)| {
                format!(
                    "{} {} {} {} {} {}",
                    a.el,
                    a.chg,
                    a.iso,
                    a.h,
                    a.rad,
                    nb.len()
                )
            })
            .collect(),
    );
    let mut count = 0;
    loop {
        let distinct = cls.iter().collect::<std::collections::BTreeSet<_>>().len();
        if distinct == count {
            return cls;
        }
        count = distinct;
        cls = rank(
            (0..cls.len())
                .map(|i| {
                    let mut nb: Vec<(u8, usize)> =
                        adj[i].iter().map(|&(j, o)| (o, cls[j])).collect();
                    nb.sort();
                    format!("{} {nb:?}", cls[i])
                })
                .collect(),
        );
    }
}

/// Whether the bond `a`-`b` is in a ring of fewer than `MIN_EZ_RING` atoms: a path from `a` to `b` without the bond, shorter
/// than the ring (breadth first, cut at the depth that matters).
fn in_small_ring(adj: &[Vec<(usize, u8)>], a: usize, b: usize) -> bool {
    let mut seen = vec![false; adj.len()];
    seen[a] = true;
    let mut front = vec![a];
    for _ in 1..MIN_EZ_RING - 1 {
        let mut next = Vec::new();
        for &x in &front {
            for &(y, _) in &adj[x] {
                if x == a && y == b {
                    continue;
                }
                if y == b {
                    return true;
                }
                if !seen[y] {
                    seen[y] = true;
                    next.push(y);
                }
            }
        }
        front = next;
    }
    false
}

impl Document {
    /// Sets the wedge marks as drawn on the bond lines of a V2000 molfile of this document (bond lines in pool order): the first
    /// atom of a marked line is the narrow end.
    pub(crate) fn mark_wedges(&self, molfile: String, record_atoms: usize) -> String {
        if self.bonds.iter().all(|b| b.s == 0 && !b.either()) || !molfile.contains("V2000") {
            return molfile;
        }
        let first = 4 + record_atoms;
        let mut lines: Vec<String> = molfile.split('\n').map(str::to_string).collect();
        let index = self.index_map();
        for (k, b) in (self.bonds.iter().enumerate()).filter(|(_, b)| b.s != 0 || b.either()) {
            let (Some(&i), Some(&j)) = (index.get(&b.a), index.get(&b.b)) else {
                continue;
            };
            let Some(line) = lines.get_mut(first + k).filter(|l| l.len() >= 12) else {
                continue;
            };
            let mark = match b.s {
                // a double bond whose E/Z is unknown: "either" is 3 on a double bond
                0 => 3,
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
        let m = self.to_mol();
        // without the R-group marks: the reader of the record refuses `R#`, the stand-in element carries the stereo through
        let text = self.mark_wedges(m.to_molfile(crate::NAME), m.atoms.len());
        Mol::from_molfile(&text).unwrap_or(m)
    }

    /// Scales the coordinates so that the median bond length is 1 (a drawing without bonds is left alone).
    fn normalise(&mut self) {
        let index = self.index_map();
        let mut len: Vec<f64> = self
            .bonds
            .iter()
            .filter_map(|b| {
                Some((
                    self.atoms.get(*index.get(&b.a)?)?,
                    self.atoms.get(*index.get(&b.b)?)?,
                ))
            })
            .map(|(p, q)| m::hypot(p.x - q.x, p.y - q.y))
            .collect();
        m::sort_by(&mut len, |a, b| a.total_cmp(b).is_lt());
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

    /// The whole canvas as one molfile (V2000, or V3000 above 999 atoms or bonds: see `to_molfile_v3000`), written by `mzlab-mol`,
    /// with the wedge marks as drawn, abbreviations expanded and R groups as `R#` atoms with their `M  RGP` numbers.
    pub fn to_molfile(&self) -> String {
        self.molfile(false)
    }

    /// The molfile for the structure check: as `to_molfile`, but an R group is the query atom `A` (any atom), which the check reads.
    pub fn to_molfile_for_check(&self) -> String {
        self.molfile(true)
    }

    /// The title line of the files this drawing is written as: the title of its first structure that has one (the name of a SMILES
    /// line, say), else the name of the program.
    pub fn file_title(&self) -> String {
        Some(self.title())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| crate::NAME.into())
    }

    fn molfile(&self, r_as_query: bool) -> String {
        let m = self.to_mol();
        if m.atoms.len() > 999 || m.bonds.len() > 999 {
            return self.v3000(m, r_as_query);
        }
        let text = self.mark_wedges(m.to_molfile(&self.file_title()), m.atoms.len());
        self.mark_maps(self.mark_r_groups(text, r_as_query))
    }

    /// Writes the R groups on the atom lines of a V2000 molfile of this document (the stand-in element becomes `R#`, or `A` for
    /// the check) and their numbers as `M  RGP` lines.
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

/// `import` for the bytes of a file: a PNG or a PDF saved by mzDraw gives the document it carries, a binary CDX file (found by
/// its magic, `VjCD0100`) is read by `cdx`, anything else is read as text.
pub fn import_bytes(bytes: &[u8]) -> Result<(Document, Imported), &'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        let d = crate::png::carried(bytes).ok_or("error.draw2.pngPlain")??;
        let info = Imported {
            format: "png",
            ..Imported::default()
        };
        return Ok((d, info));
    }
    if bytes.starts_with(b"%PDF-") {
        let d = crate::pdf::carried(bytes).ok_or("error.draw2.pdfPlain")??;
        let info = Imported {
            format: "pdf",
            ..Imported::default()
        };
        return Ok((d, info));
    }
    if crate::cdx::is_cdx(bytes) {
        let mut info = Imported {
            format: "cdx",
            ..Imported::default()
        };
        let tree = crate::cdx::tree(bytes).ok_or(crate::cdx::KEY)?;
        info.code_page_runs = tree.code_page_runs;
        info.props_skipped = tree.props_skipped;
        let d = crate::cdxml::from_tree(&tree.root, &mut info, crate::cdx::KEY)?;
        return checked(d, info, crate::cdx::KEY);
    }
    import(std::str::from_utf8(bytes).map_err(|_| "error.draw2.import")?)
}

/// Reads a structure text: an `mzdraw/1` document, a molfile or an SDF (V2000/V3000; every record, side by side) or SMILES (one
/// per line). The error is an i18n key.
pub fn import(text: &str) -> Result<(Document, Imported), &'static str> {
    let t = text.trim();
    let mut info = Imported::default();
    // a transformation: the record `trasf/1` with its two records, one JSON text per line; the first line says it (`trasf/1`, or `mol/1`
    // when the records come first), so a KET file or a document that mentions the text is not taken for one
    let first = t.lines().next().unwrap_or("");
    if t.lines().count() > 1
        && first.starts_with('{')
        && serde_json::from_str::<serde_json::Value>(first)
            .is_ok_and(|v| v["format"] == "trasf/1" || v["format"] == "mol/1")
    {
        let d = crate::trasf::from_trasf(t, &mut info)?;
        return checked(d, info, crate::trasf::READ);
    }
    if t.starts_with('{') {
        // a JSON text with a `root` and no `format` is a KET file
        let ket = serde_json::from_str::<serde_json::Value>(t).ok();
        if let Some(v) = ket.filter(|v| v["root"].is_object() && v.get("format").is_none()) {
            info.format = "ket";
            let d = crate::ket::from_ket(&v, &mut info)?;
            return checked(d, info, "error.draw2.ket");
        }
        info.format = crate::brand::SLUG;
        return Ok((Document::from_json(t)?, info));
    }
    if t.contains("<CDXML") {
        info.format = "cdxml";
        let d = crate::cdxml::from_cdxml(t, &mut info)?;
        return checked(d, info, "error.draw2.cdxml");
    }
    if t.contains("<cml") || t.contains("<molecule") || t.contains(":cml") {
        info.format = "cml";
        let d = crate::cml::from_cml(t, &mut info)?;
        return checked(d, info, "error.draw2.cml");
    }
    if t.starts_with('<') {
        // an SVG written by mzDraw carries its document; a picture of another program has no structure to read
        info.format = "svg";
        let d = crate::svg::carried(t).ok_or("error.draw2.svgPlain")??;
        return Ok((d, info));
    }
    if t.starts_with("$RXN") || t.starts_with("$RDFILE") {
        info.format = "rxn";
        let d = crate::rxn::from_rxn(t, &mut info)?;
        return checked(d, info, "error.draw2.rxn");
    }
    if text.contains("M  END") {
        info.format = "molfile";
        let mut all = Document::default();
        for (k, record) in text.split("$$$$").enumerate() {
            // what is left of the `$$$$` line of the record before (the first record keeps its empty title line)
            let record = match k {
                0 => extra_blank_line(record),
                _ => (record.strip_prefix("\r\n"))
                    .or(record.strip_prefix('\n'))
                    .unwrap_or(record),
            };
            if !record.contains("M  END") {
                continue;
            }
            if info.records == MAX_RECORDS {
                info.skipped += 1;
                continue;
            }
            let (mut d, placeholder) = record_of(record, &mut info)?;
            info.placeholder |= placeholder;
            // the data fields of the record are kept on its first atom
            let (data, ann, left) = formats::sdf_data(record);
            info.data_left += left;
            info.no_pathway += usize::from(
                data.iter()
                    .any(|(k, _)| k.trim().eq_ignore_ascii_case("TP_ID")),
            );
            if let Some(a) = d.atoms.first_mut() {
                a.set_data(&data);
                if !ann.is_empty() {
                    a.set_ann(&ann);
                }
            }
            info.records += 1;
            all.append(d);
        }
        return checked(all, info, "error.draw2.molfile");
    }
    if t.is_empty() {
        return Err("error.draw2.import");
    }
    info.format = "smiles";
    let mut all = Document::default();
    // OpenSMILES, "SMILES Files": one SMILES per line, what follows the first space or tab is the name of the record, LF and CR/LF
    // end a line, a blank line and a line that starts with a space or a tab are ignored; nothing is stripped into the SMILES
    let mut broken: Vec<usize> = Vec::new();
    for (n, line) in text
        .strip_prefix('\u{feff}')
        .unwrap_or(text)
        .lines()
        .enumerate()
    {
        if line.is_empty() || line.starts_with([' ', '\t']) || line.trim().is_empty() {
            continue;
        }
        // at most MAX_RECORDS lines are parsed, good or broken: the work is bounded by that many lines of the input, whatever its size
        if info.records + broken.len() == MAX_RECORDS {
            info.skipped += 1;
            continue;
        }
        let (smiles, name, cut) = smiles_record(line);
        // a reaction SMILES (`reactants>agents>products`) is drawn as a reaction file is; a molecule may have dative bonds
        let core = smiles.split([' ', '\t']).next().unwrap_or(smiles);
        let read = match crate::rsmiles::reaction(core) {
            Ok(Some([r, a, p])) => {
                let side = |part: &str, info: &mut Imported| -> Option<Vec<Document>> {
                    (crate::rsmiles::molecules(part).into_iter())
                        .map(|m| smiles_molecule(m, info))
                        .collect()
                };
                match (side(r, &mut info), side(a, &mut info), side(p, &mut info)) {
                    (Some(r), Some(a), Some(p)) if !(r.is_empty() && p.is_empty()) => {
                        // the CXSMILES tail of a reaction line (stereo groups, coordinates of its molecules) is not read: said
                        info.reaction_tails += usize::from(smiles.len() > core.len());
                        Some(crate::rxn::reaction_of(r, p, a))
                    }
                    _ => None,
                }
            }
            Ok(None) => smiles_molecule(smiles, &mut info),
            Err(()) => None,
        };
        let Some(mut d) = read else {
            broken.push(n + 1);
            continue;
        };
        if let Some(a) = d.atoms.first_mut() {
            a.set_title(&name);
        }
        info.records += 1;
        info.names_cut += usize::from(cut);
        all.append(d);
    }
    if info.records == 0 {
        return Err(if broken.is_empty() {
            "error.draw2.import"
        } else {
            "error.draw2.smiles"
        });
    }
    info.smiles_lines = broken;
    checked(all, info, "error.draw2.smiles")
}

/// One molecule of a SMILES (with its CXSMILES tail): its dative bonds (`->`, `<-`, `rsmiles::datives`) made bonds of the kind
/// `dative` from the donor to the acceptor, its R groups and its atom maps read. `None` when the reader refuses it.
fn smiles_molecule(smiles: &str, info: &mut Imported) -> Option<Document> {
    // the dative arrows of the SMILES itself: the CXSMILES tail after the first blank (labels may hold `->`) is kept as it is
    let at = smiles.find([' ', '\t']).unwrap_or(smiles.len());
    let (core, tail) = smiles.split_at(at);
    let (plain, pairs) = crate::rsmiles::datives(core).ok()?;
    let plain = plain + tail;
    let (clean, labels) = formats::r_atoms(&plain);
    let m = Mol::from_smiles(&clean).ok()?;
    let (mut d, placeholder) = labelled(&m, labels, info);
    d.set_maps(crate::maps::of_smiles(&plain), info);
    info.placeholder |= placeholder;
    for (donor, acceptor) in pairs {
        let (x, y) = (d.atoms.get(donor)?.id, d.atoms.get(acceptor)?.id);
        let b = (d.bonds.iter_mut()).find(|b| (b.a, b.b) == (x, y) || (b.a, b.b) == (y, x))?;
        if b.o != 1 {
            return None;
        }
        (b.a, b.b, b.s) = (x, y, 0);
        crate::bonddisp::k_put(&mut b.more, "dative");
    }
    Some(d)
}

/// A line of a SMILES file: the SMILES (with its CXSMILES tail, `|...|`, when the first word after it opens one), the name of the
/// record (the rest of the line: a control character, a tab included, is a space; at most `MAX_TITLE` characters) and whether the
/// name was cut to that length.
fn smiles_record(line: &str) -> (&str, String, bool) {
    let at = line.find([' ', '\t']).unwrap_or(line.len());
    let (mut smiles, mut rest) = line.split_at(at);
    let tail = rest.trim_start_matches([' ', '\t']);
    if let Some(body) = tail.strip_prefix('|')
        && let Some(close) = body.find('|')
    {
        let end = line.len() - rest.len() + (rest.len() - tail.len()) + close + 2;
        (smiles, rest) = (&line[..end], &line[end..]);
    }
    let spaced: String = (rest.trim_matches([' ', '\t']).chars())
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let cut = spaced.chars().count() > crate::doc::MAX_TITLE;
    let name: String = spaced.chars().take(crate::doc::MAX_TITLE).collect();
    (smiles, name.trim_end().to_string(), cut)
}

/// The drawing of one molfile record (V2000 or V3000), and whether its positions are placeholders; what the drawing does not
/// keep is counted in `info`.
pub(crate) fn record_of(
    record: &str,
    info: &mut Imported,
) -> Result<(Document, bool), &'static str> {
    let (clean, labels) = formats::prepare(record, info);
    let m = Mol::from_molfile(&clean).map_err(|_| "error.draw2.molfile")?;
    let (mut d, placeholder) = labelled(&m, labels, info);
    d.set_maps(crate::maps::of_molfile(&clean), info);
    Ok((d, placeholder))
}

/// A molfile pasted with one blank line too many before its title: the counts line stands one line late (line 5 and not line 4),
/// and the blank line is dropped. A molfile with an empty title line has its counts on line 4 and is left as it is.
fn extra_blank_line(record: &str) -> &str {
    let counts = |l: Option<&str>| l.is_some_and(|l| l.contains("V2000") || l.contains("V3000"));
    let mut lines = record.lines();
    let blank = lines.next().is_some_and(|l| l.trim().is_empty());
    if blank && !counts(record.lines().nth(3)) && counts(record.lines().nth(4)) {
        let at = record.find('\n').map_or(0, |i| i + 1);
        return &record[at..];
    }
    record
}

/// A file may hold what a document does not (a coordinate far outside the bound): it is refused with `key`, not half kept.
fn checked(
    d: Document,
    info: Imported,
    key: &'static str,
) -> Result<(Document, Imported), &'static str> {
    if d.consistent() {
        Ok((d, info))
    } else {
        Err(key)
    }
}

/// The drawing of a record whose atoms `labels` (index, label) are stand-ins of R groups: they get their label back. What the
/// drawing does not keep of the record is counted in `info`.
pub(crate) fn labelled(
    m: &Mol,
    labels: Vec<(usize, String)>,
    info: &mut Imported,
) -> (Document, bool) {
    let grouped = |g: &&mzlab_mol::StereoGroup| g.kind != mzlab_mol::GroupKind::Abs;
    info.stereo_groups += m.groups.iter().filter(grouped).count();
    let (mut d, placeholder) = Document::from_mol(m);
    // the bonds the file drew from one atom to the other: dative and hydrogen bonds (the donor is the first atom of the line)
    for (from, to, kind) in std::mem::take(&mut info.links) {
        let ids = (
            d.atoms.get(from).map(|a| a.id),
            d.atoms.get(to).map(|a| a.id),
        );
        // the reader may order the bonds as it likes: the bond is found by its two atoms
        if let (Some(f), Some(t)) = ids
            && let Some(b) = d
                .bonds
                .iter_mut()
                .find(|b| (b.a, b.b) == (f, t) || (b.a, b.b) == (t, f))
        {
            b.o = 1;
            crate::bonddisp::k_put(&mut b.more, kind);
            (b.a, b.b) = (f, t);
        }
    }
    for (i, lab) in labels {
        if let Some(a) = d.atoms.get_mut(i) {
            (a.el, a.h, a.lab) = ("C".into(), None, lab);
        }
    }
    (d, placeholder)
}
