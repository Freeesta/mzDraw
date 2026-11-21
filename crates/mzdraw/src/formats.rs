//! Formats, first family: what the molfile and SMILES doors need around the reader and the writers of `mzlab-mol`. SMILES out with
//! the brackets a reader does not need taken away; the V3000 molfile with the wedge marks and the R groups of the drawing; the
//! parts of a molfile the record has no place for (R groups, query atoms and bonds, S-groups), taken out before the strict reader
//! sees them and reported, never dropped in silence.

use crate::doc::{Document, Obj, ObjKind, WEDGE_DOWN, WEDGE_UP};
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
            let map = self.atoms.get(i).map_or(0, |a| a.map);
            out.push_str(&rest[..open]);
            match (
                self.atoms.get(i).and_then(|a| label::kind(&a.lab)),
                m.atoms.get(i),
            ) {
                (Some(Kind::R(0)), _) => out.push_str("[*]"),
                (Some(Kind::R(n)), _) => out.push_str(&format!("[*:{n}]")),
                (_, Some(a)) if !token.contains('@') && bare(a, used[i]) && map == 0 => {
                    out.push_str(&a.el)
                }
                // the atom class of a bracket atom is its map
                _ if map != 0 && token.ends_with(']') => {
                    out.push_str(&format!("{}:{map}]", &token[..token.len() - 1]))
                }
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
        let index = self.index_map();
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
                    let map = self.atoms.get(k).map_or(0, |a| a.map);
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
                    // the fifth field after the symbol is the atom-to-atom map
                    if map != 0 {
                        marked = Some(crate::maps::v3000_line_with_map(
                            marked.as_deref().unwrap_or(line),
                            map,
                        ));
                    }
                    k += 1;
                }
                (_, 'b') => {
                    let ends = (self.bonds.get(k).filter(|b| b.s != 0))
                        .and_then(|b| Some((b.s, *index.get(&b.a)?, *index.get(&b.b)?)));
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
    pub(crate) fn append(&mut self, other: Document) {
        let dx = match (self.bounds(), other.bounds()) {
            (Some(mine), Some(new)) => mine.2 + 2.0 - new.0,
            _ => 0.0,
        };
        self.join(other, dx, 0.0);
    }

    /// Adds every atom, bond and object of `other`, moved by (`dx`, `dy`), with ids after the ones in use.
    pub(crate) fn join(&mut self, mut other: Document, dx: f64, dy: f64) {
        let shift = self.next.max(1) - 1;
        let moved = |x: f64, d: f64| crate::doc::round4(x + d);
        for a in &mut other.atoms {
            a.id += shift;
            (a.x, a.y) = (moved(a.x, dx), moved(a.y, dy));
        }
        for o in &mut other.objects {
            o.id += shift;
            (o.x, o.y) = (moved(o.x, dx), moved(o.y, dy));
            if o.t == ObjKind::Arrow {
                (o.x2, o.y2) = (moved(o.x2, dx), moved(o.y2, dy));
            }
        }
        self.objects.append(&mut other.objects);
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

/// An object of a file on its way into a document: its kind, x, y, x2, y2 in the units of the file, and its text.
pub(crate) type Placed = (ObjKind, [f64; 4], String);

/// The drawing of a file that holds atoms, bonds and objects on one canvas (KET, CDXML). `atoms` and `bonds` are the bodies of
/// V3000 lines (symbol, x, y, z, map and properties; order, atoms and `CFG`): they become one record for the strict reader of
/// `mzlab-mol`, through the same door as a molfile. `far` is the largest coordinate of an atom, its atom and whether it is a y:
/// the drawing is scaled to its own bond length and the objects follow by that atom. The error is `key`.
pub(crate) fn assemble(
    atoms: &[String],
    bonds: &[String],
    objects: Vec<Placed>,
    far: (f64, usize, bool),
    info: &mut Imported,
    key: &'static str,
) -> Result<Document, &'static str> {
    let mut d = Document::default();
    let mut k = 1.0;
    if !atoms.is_empty() {
        let mut text = format!(
            "file\n\n\n  0  0  0     0  0            999 V3000\nM  V30 BEGIN CTAB\nM  V30 COUNTS {} {} 0 0 0\nM  V30 BEGIN ATOM\n",
            atoms.len(),
            bonds.len()
        );
        for (i, a) in atoms.iter().enumerate() {
            text += &format!("M  V30 {} {a}\n", i + 1);
        }
        text += "M  V30 END ATOM\nM  V30 BEGIN BOND\n";
        for (i, b) in bonds.iter().enumerate() {
            text += &format!("M  V30 {} {b}\n", i + 1);
        }
        text += "M  V30 END BOND\nM  V30 END CTAB\nM  END\n";
        let (clean, labels) = prepare(&text, info);
        let m = Mol::from_molfile(&clean).map_err(|_| key)?;
        d = crate::io::labelled(&m, labels, info).0;
        d.set_maps(crate::maps::of_molfile(&clean), info);
        if let Some(a) = d.atoms.get(far.1).filter(|_| far.0.abs() > 1e-9) {
            k = if far.2 { a.y } else { a.x } / far.0;
        }
    }
    for (t, p, text) in objects {
        let [x, y, x2, y2] = p.map(|c| crate::doc::round4(c * k));
        // a document text is one line: a text of several lines becomes one text per line, each under the one before
        let lines: Vec<&str> = if t == ObjKind::Text {
            text.lines().filter(|l| !l.trim().is_empty()).collect()
        } else {
            vec![""]
        };
        let mut kept = false;
        for (i, line) in lines.iter().enumerate() {
            let o = Obj {
                id: d.next.max(1),
                t,
                x,
                y: crate::doc::round4(y - 0.6 * i as f64),
                x2,
                y2,
                text: line.chars().take(200).collect(),
                from: 0,
                to: 0,
                more: Vec::new(),
            };
            // what a document cannot hold (an arrow without length) is counted with the things left out
            if o.valid() {
                d.next = o.id + 1;
                d.objects.push(o);
                kept = true;
            }
        }
        info.other += usize::from(!kept);
    }
    Ok(d)
}

/// The data fields of one SDF record (what follows `M  END`): each `> <name>` header with the lines after it up to the empty
/// line that ends the field. Returns the fields a document can hold and how many it cannot (too many, too long, a name or a
/// value of a shape `doc::data_valid` refuses). The fields an annotation is written as (`ann::FIELDS`) are read back into the
/// annotation, the second value (`ann::canon` text, empty for none): the whole one of `MZLAB_ANN` when it is one, else an
/// entry of `MZLAB_LEVEL` with `MZLAB_IDENTIFIER` and `MZLAB_EVIDENCE`; those that give no annotation are counted as not held.
pub(crate) fn sdf_data(record: &str) -> (crate::doc::Data, String, usize) {
    let after = record.find("M  END").map_or("", |at| &record[at..]);
    let (mut data, mut left) = (crate::doc::Data::new(), 0);
    let mut held: [Option<String>; 4] = [None, None, None, None];
    let mut lines = after.lines().skip(1).map(|l| l.trim_end_matches('\r'));
    while let Some(line) = lines.next() {
        let Some(header) = line.strip_prefix('>') else {
            continue;
        };
        // the name stands between `<` and `>`; the other things a header may carry (a field number, a registry number) are not kept
        let name = match (header.find('<'), header.rfind('>')) {
            (Some(a), Some(b)) if a < b => header[a + 1..b].trim(),
            _ => header.trim(),
        };
        let mut value: Vec<&str> = Vec::new();
        for l in lines.by_ref() {
            if l.trim().is_empty() {
                break;
            }
            value.push(l);
        }
        if let Some(k) = crate::ann::FIELDS.iter().position(|f| *f == name) {
            left += usize::from(held[k].is_some());
            held[k] = Some(value.join("\n"));
            continue;
        }
        let field = [(name.to_string(), value.join("\n"))];
        if data.len() < crate::doc::MAX_DATA.0 && crate::doc::data_valid(&field) {
            data.extend(field);
        } else {
            left += 1;
        }
    }
    let [level, identifier, evidence, whole] = held;
    let part = |p: &Option<String>| p.clone().unwrap_or_default();
    let ann = (whole.as_deref().and_then(crate::ann::canon)).or_else(|| {
        crate::ann::entry(
            crate::ann::SCALE,
            &level.clone()?,
            &part(&identifier),
            &part(&evidence),
            "",
            "",
        )
    });
    if ann.is_none() {
        left += [&level, &identifier, &evidence, &whole]
            .iter()
            .filter(|p| p.is_some())
            .count();
    }
    (data, ann.filter(|t| t != "[]").unwrap_or_default(), left)
}

impl Document {
    /// The structures of the canvas as an SDF: one record per structure, in the order of their first atoms; each is the molfile
    /// of the structure (V2000; V3000 above 999 atoms), then its data (the data of its atoms in pool order) as `>  <name>`
    /// fields, then its annotation if it has one (`ann::FIELDS`: the level, the identifier and the evidence of the current
    /// entry, and the whole annotation as one line of JSON), then `$$$$`. Arrows, plus signs and texts have no place in an SDF.
    /// The same drawing gives the same bytes.
    pub fn to_sdf(&self) -> String {
        let mut out = String::new();
        for ids in crate::layout::pieces(self) {
            let piece = self.part(&ids);
            out += piece.to_molfile().trim_end_matches('\n');
            out.push('\n');
            for (name, value) in piece.atoms.iter().flat_map(|a| a.data()) {
                out += &format!(">  <{name}>\n{value}\n\n");
            }
            let whole = crate::doc::Atom::ann_of(&piece.atoms, &ids);
            if let Some(now) = crate::ann::current(&whole) {
                let fields = [now.level, now.identifier, now.evidence, whole];
                for (name, value) in crate::ann::FIELDS.iter().zip(fields) {
                    if !value.is_empty() {
                        out += &format!(">  <{name}>\n{value}\n\n");
                    }
                }
            }
            out += "$$$$\n";
        }
        out
    }
}
