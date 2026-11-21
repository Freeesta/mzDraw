//! What a format loses, said before the file is written (plan-22). One table says, per feature of the document and per output
//! format, whether the writer carries it; `loss(doc, format)` lists the things of THIS document that the format drops or changes,
//! with their ids. The table is not trusted: `observed` compares two documents feature by feature without looking at the table, and
//! the property test of `tests/loss.rs` writes random documents in every format, reads them back and holds the two to each other
//! (what the round trip lost is in `loss`, and every item of `loss` is a real difference).
//!
//! The table cannot decide some cells, because they depend on the drawing:
//! - a wedge mark: the readers derive the marks again from the stereo of the record, so whether a drawn mark comes back depends on the
//!   chemistry of the centre;
//! - the radical in a SMILES: it is kept for some atoms and not for others;
//! - the plus signs of a reaction file: it makes its own between the structures;
//! - the shape: a SMILES has no coordinates, an SDF puts its records side by side again, a reaction file places the structures about
//!   its arrow, unless the layout they give is the one drawn.
//!
//! Those cells are `Carry::Probe`: `loss` writes the document, reads it back and compares (the round trip itself is the answer, a limit
//! stated in `docs/built/loss-manifest.md`). The answer is per kind: when some of a kind do not come back, the reader does not say
//! which, so every occurrence of that kind is listed.
//!
//! The shape (`geometry`) is compared only for those formats, and in bond lengths. A molfile, a V3000 file, KET and CDXML rescale a
//! drawing to a bond length of 1, and CDXML rounds to a hundredth of a point (about 0.0005 of a bond): neither is checked or a loss here.

use crate::doc::{Document, ObjKind};
use crate::label::{self, Kind};
use std::collections::BTreeMap;

/// The output formats, in the order of the columns of `TABLE`.
pub const FORMATS: [&str; 10] = [
    "molfile",
    "molfile-v3000",
    "smiles",
    "sdf",
    "ket",
    "cdxml",
    "cml",
    "rxn",
    "svg",
    crate::brand::SLUG,
];

/// The format field of the document `loss/1`.
pub const FORMAT: &str = "loss/1";

/// Documents above this many atoms, bonds and objects together are refused (`error.draw2.lossSize`): a round trip writes and reads the
/// whole drawing (about a second in the worst case at this size, in a release build).
pub const MAX_THINGS: usize = 50_000;

/// A round trip writes the atoms of the drawing and, for each abbreviation, the atoms it stands for (seven for `Boc`), and reads them all
/// back; the stereo perception of `mzlab-mol` that follows is not linear. A drawing that would be written as more atoms than this is refused
/// (`error.draw2.lossSize`) wherever a round trip is needed (about a second in a release build at this size).
pub const MAX_WRITTEN_ATOMS: usize = 40_000;

/// The number of atoms the writers make of a drawing: the atoms drawn, an abbreviation as its own atoms.
fn written_atoms(d: &Document) -> usize {
    let mut size: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut total = d.atoms.len();
    for a in &d.atoms {
        if let Some(Kind::Group(smiles)) = label::kind(&a.lab) {
            let n = *size.entry(smiles).or_insert_with(|| {
                mzlab_mol::Mol::from_smiles(smiles).map_or(1, |m| m.atoms.len())
            });
            total += n.saturating_sub(1);
        }
    }
    total
}

/// The shape of a drawing is compared by the distances between all its atoms, which grow with the square of the atom count: where the
/// shape is part of the answer (SMILES, SDF, reaction file) a drawing above this many atoms is refused, and so is a file that reads
/// back as more than `MAX_BACK_ATOMS` (abbreviations come back as their atoms).
pub const MAX_SHAPE_ATOMS: usize = 600;

/// See `MAX_SHAPE_ATOMS`.
pub const MAX_BACK_ATOMS: usize = 3_000;

/// What a writer does with a feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carry {
    /// Written and read back as it was (for `geometry`: the positions are written; the scale is not checked).
    Yes,
    /// Not written, or written in a form the reader does not turn back into the feature.
    No,
    /// Depends on the drawing: decided by writing and reading it (wedge marks).
    Probe,
}

const Y: Carry = Carry::Yes;
const N: Carry = Carry::No;
const P: Carry = Carry::Probe;

/// One row per feature (`what`), one column per format of `FORMATS`. Every `what` of `instances` has a row.
pub const TABLE: &[(&str, [Carry; 10])] = &[
    //                      mol v3k smi sdf ket cdx cml rxn svg doc
    ("abbreviation", [N, N, N, N, N, N, N, N, Y, Y]),
    ("annotation", [N, N, N, Y, N, N, N, N, Y, Y]),
    ("arrow.anchor", [N, N, N, N, N, N, N, N, Y, Y]),
    ("arrow.text", [N, N, N, N, N, N, N, N, Y, Y]),
    ("atom.charge", [Y, Y, Y, Y, Y, Y, Y, Y, Y, Y]),
    ("atom.isotope", [Y, Y, Y, Y, Y, Y, Y, Y, Y, Y]),
    ("atom.map", [Y, Y, Y, Y, Y, N, N, Y, Y, Y]),
    ("atom.radical", [Y, Y, P, Y, Y, Y, Y, Y, Y, Y]),
    ("colour", [N, N, N, N, N, Y, N, N, Y, Y]),
    ("data", [N, N, N, Y, N, N, N, N, Y, Y]),
    ("geometry", [Y, Y, P, P, Y, Y, Y, P, Y, Y]),
    ("object.arrow", [N, N, N, N, Y, Y, N, Y, Y, Y]),
    ("object.plus", [N, N, N, N, Y, Y, N, P, Y, Y]),
    ("object.text", [N, N, N, N, Y, Y, N, N, Y, Y]),
    ("rlabel", [Y, Y, Y, Y, Y, Y, Y, Y, Y, Y]),
    ("unknown", [N, N, N, N, N, N, N, N, Y, Y]),
    ("wedge.hashed", [P, P, P, P, P, P, P, P, Y, Y]),
    ("wedge.solid", [P, P, P, P, P, P, P, P, Y, Y]),
    ("wedge.wavy", [P, P, P, P, P, P, P, P, Y, Y]),
];

/// The cell of a feature and a format; `None` for a name outside the table.
pub fn carry(what: &str, format: &str) -> Option<Carry> {
    let col = FORMATS.iter().position(|f| *f == format)?;
    TABLE.iter().find(|r| r.0 == what).map(|r| r.1[col])
}

/// One thing of the document the format drops or changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loss {
    pub what: &'static str,
    pub atoms: Vec<u32>,
    pub bonds: Vec<u32>,
    pub objects: Vec<u32>,
}

/// One occurrence of a feature in a document: its kind, a value that tells two of a kind apart (never an id), and the id it is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instance {
    pub what: &'static str,
    pub value: String,
    pub atom: u32,
    pub bond: u32,
    pub object: u32,
}

/// Every occurrence of every feature of the table, except `geometry` (a property of the whole drawing, see `geometry_of`).
pub fn instances(d: &Document) -> Vec<Instance> {
    let mut out = Vec::new();
    let mut push = |what, value: String, atom, bond, object| {
        out.push(Instance {
            what,
            value,
            atom,
            bond,
            object,
        })
    };
    for a in &d.atoms {
        match label::kind(&a.lab) {
            Some(Kind::R(_)) => push("rlabel", a.lab.clone(), a.id, 0, 0),
            Some(Kind::Group(_)) => push("abbreviation", a.lab.clone(), a.id, 0, 0),
            // a label stands alone: the charge, isotope, radical and map of its own are not drawn
            None => {
                if a.chg != 0 {
                    push("atom.charge", a.chg.to_string(), a.id, 0, 0);
                }
                if a.iso != 0 {
                    push("atom.isotope", a.iso.to_string(), a.id, 0, 0);
                }
                if a.rad != 0 {
                    push("atom.radical", a.rad.to_string(), a.id, 0, 0);
                }
                if a.map != 0 {
                    push("atom.map", a.map.to_string(), a.id, 0, 0);
                }
            }
        }
        if !a.col().is_empty() {
            push("colour", a.col(), a.id, 0, 0);
        }
        for (name, value) in a.data() {
            push("data", format!("{name}={value}"), a.id, 0, 0);
        }
        if !a.ann().is_empty() {
            push("annotation", a.ann(), a.id, 0, 0);
        }
        for (key, _) in (a.more.iter()).filter(|m| !["data", "ann", "col"].contains(&m.0.as_str()))
        {
            push("unknown", format!("atom.{key}"), a.id, 0, 0);
        }
    }
    for b in &d.bonds {
        let what = match b.s {
            crate::doc::WEDGE_UP => Some("wedge.solid"),
            crate::doc::WEDGE_DOWN => Some("wedge.hashed"),
            crate::doc::WEDGE_WAVY => Some("wedge.wavy"),
            _ => None,
        };
        if let Some(what) = what {
            push(what, String::new(), 0, b.id, 0);
        }
        if !b.col().is_empty() {
            push("colour", b.col(), 0, b.id, 0);
        }
        for (key, _) in b.more.iter().filter(|m| m.0 != "col") {
            push("unknown", format!("bond.{key}"), 0, b.id, 0);
        }
    }
    for o in &d.objects {
        match o.t {
            ObjKind::Arrow => {
                push("object.arrow", String::new(), 0, 0, o.id);
                if !o.text.is_empty() {
                    push("arrow.text", o.text.clone(), 0, 0, o.id);
                }
                if o.from != 0 || o.to != 0 {
                    let ends = format!("{}{}", u8::from(o.from != 0), u8::from(o.to != 0));
                    push("arrow.anchor", ends, 0, 0, o.id);
                }
            }
            ObjKind::Plus => push("object.plus", String::new(), 0, 0, o.id),
            ObjKind::Text => push("object.text", o.text.clone(), 0, 0, o.id),
        }
        for (key, _) in &o.more {
            push("unknown", format!("object.{key}"), 0, 0, o.id);
        }
    }
    for (key, _) in &d.more {
        push("unknown", format!("document.{key}"), 0, 0, 0);
    }
    out
}

/// The distances between the atoms of a drawing, to two decimals, sorted: what does not change when the drawing is moved, turned over
/// or written again. `None` for fewer than two atoms (nothing to lay out) and above `max` atoms.
fn distances(d: &Document, max: usize, unit: f64) -> Option<Vec<i64>> {
    if d.atoms.len() < 2 || d.atoms.len() > max {
        return None;
    }
    let mut dist = Vec::with_capacity(d.atoms.len() * d.atoms.len() / 2);
    for (i, p) in d.atoms.iter().enumerate() {
        for q in &d.atoms[i + 1..] {
            dist.push((crate::m::hypot(p.x - q.x, p.y - q.y) / unit * 100.0).round() as i64);
        }
    }
    dist.sort_unstable();
    Some(dist)
}

/// The median length of the bonds of a drawing, or 1 when it has none (or they have no length): the unit of its scale. The readers of
/// a molfile, a V3000 file, KET and CDXML rescale a drawing to a bond length of 1, so the shape is compared in bond lengths.
fn unit_of(d: &Document) -> f64 {
    let index = d.index_map();
    let mut len: Vec<f64> = (d.bonds.iter())
        .filter_map(|b| {
            Some((
                d.atoms.get(*index.get(&b.a)?)?,
                d.atoms.get(*index.get(&b.b)?)?,
            ))
        })
        .map(|(p, q)| crate::m::hypot(p.x - q.x, p.y - q.y))
        .collect();
    len.sort_by(f64::total_cmp);
    len.get(len.len() / 2)
        .copied()
        .filter(|m| *m > 1e-6)
        .unwrap_or(1.0)
}

/// Whether a drawing has a shape worth keeping: two atoms or more.
pub fn has_shape(d: &Document) -> bool {
    d.atoms.len() >= 2
}

/// Whether the shape of `a` is in `b`, in bond lengths (the median bond of each is the unit, so a change of scale is not a change of
/// shape): every distance of `a` is a distance of `b` (a reader may add atoms, an abbreviation comes back
/// as its atoms, but the atoms drawn stay where they were).
fn shape_kept(a: &Document, b: &Document) -> bool {
    let (Some(da), Some(db)) = (
        distances(a, MAX_SHAPE_ATOMS, unit_of(a)),
        distances(b, MAX_BACK_ATOMS, unit_of(b)),
    ) else {
        return true;
    };
    // the writers round to a hundredth of a point, which is about 0.0005 of a bond: a distance matches within 0.02 of a bond
    let mut j = 0;
    da.iter().all(|x| {
        while j < db.len() && db[j] < *x - 2 {
            j += 1;
        }
        let found = j < db.len() && db[j] <= *x + 2;
        j += usize::from(found);
        found
    })
}

/// The kinds of feature (`what`) that `a` has and `b` does not: for every kind, the occurrences of `a` (kind and value) that `b` has
/// fewer of. Matched by value, never by id: a reader gives new ids and may put more things than were written (an abbreviation comes
/// back as its atoms). It does not look at the table. `geometry` is named when the shape of the drawing differs.
pub fn observed(a: &Document, b: &Document) -> Vec<&'static str> {
    observed_with(a, b, true)
}

/// `observed`, with the shape compared only when `shape` (it is the one comparison that costs more than the size of the drawing).
fn observed_with(a: &Document, b: &Document, shape: bool) -> Vec<&'static str> {
    let count = |d: &Document| {
        let mut m: BTreeMap<(&'static str, String), usize> = BTreeMap::new();
        for i in instances(d) {
            *m.entry((i.what, i.value)).or_default() += 1;
        }
        m
    };
    let (ca, cb) = (count(a), count(b));
    let mut out: Vec<&'static str> = Vec::new();
    for (k, n) in &ca {
        if cb.get(k).copied().unwrap_or(0) < *n && !out.contains(&k.0) {
            out.push(k.0);
        }
    }
    if shape && has_shape(a) && !shape_kept(a, b) {
        out.push("geometry");
    }
    out.sort_unstable();
    out
}

/// The document as the text of the format; a writer's refusal is its key (`rxn` needs exactly one arrow, `smiles` a drawing that has
/// a SMILES). Not for `svg` (it needs the style of the editor) or the document: both carry everything.
fn write(d: &Document, format: &str) -> Result<String, &'static str> {
    Ok(match format {
        "molfile" => d.to_molfile(),
        "molfile-v3000" => d.to_molfile_v3000(),
        "smiles" => d.to_smiles()?,
        "sdf" => d.to_sdf(),
        "ket" => d.to_ket(),
        "cdxml" => d.to_cdxml(),
        "cml" => d.to_cml(),
        "rxn" => d.to_rxn()?,
        "svg" => {
            let mut e = crate::editor::Editor::default();
            e.session.doc = d.clone();
            e.svg()
        }
        f if f == crate::brand::SLUG => d.to_json(),
        _ => return Err("error.draw2.request"),
    })
}

/// What `format` drops or changes of this document, ordered by `what` then by the smallest id. An unknown format is
/// `error.draw2.request`; a document above `MAX_THINGS` (or the limits of `formats::MAX_SMILES_ATOMS` (a piece), `MAX_SHAPE_ATOMS`, `MAX_BACK_ATOMS`, `MAX_WRITTEN_ATOMS`) is `error.draw2.lossSize`; a document the writer refuses gives the writer's
/// key (there is no file, so no loss to list).
pub fn loss(d: &Document, format: &str) -> Result<Vec<Loss>, &'static str> {
    if !FORMATS.contains(&format) {
        return Err("error.draw2.request");
    }
    if d.atoms.len() + d.bonds.len() + d.objects.len() > MAX_THINGS {
        return Err("error.draw2.lossSize");
    }
    if (format == "smiles" && crate::formats::smiles_too_large(d))
        || (d.atoms.len() > MAX_SHAPE_ATOMS && carry("geometry", format) == Some(Carry::Probe))
    {
        return Err("error.draw2.lossSize");
    }
    // the writers that refuse some drawings say so before anything is listed
    if matches!(format, "smiles" | "rxn") {
        write(d, format)?;
    }
    let all = instances(d);
    // the cells that depend on the drawing: write it, read it back, see which kinds of mark do not return
    let probe = |what| carry(what, format) == Some(Carry::Probe);
    // the shape is compared only where its cell says `drawing`; where it says `yes` it is not looked at (see `Carry`)
    let shape = has_shape(d) && probe("geometry");
    let probed: Vec<&'static str> = if all.iter().any(|i| probe(i.what)) || shape {
        if written_atoms(d) > MAX_WRITTEN_ATOMS {
            return Err("error.draw2.lossSize");
        }
        // a file the reader refuses (its own bounds on elements and depth) is too large to check: the key is `error.draw2.lossSize`,
        // not the reader's, and the schema row says so
        let back = crate::io::import(&write(d, format)?)
            .map_err(|_| "error.draw2.lossSize")?
            .0;
        if shape && back.atoms.len() > MAX_BACK_ATOMS {
            return Err("error.draw2.lossSize");
        }
        observed_with(d, &back, shape)
    } else {
        Vec::new()
    };
    let mut out: Vec<Loss> = Vec::new();
    for i in &all {
        let gone = match carry(i.what, format) {
            Some(Carry::No) => true,
            Some(Carry::Probe) => probed.contains(&i.what),
            _ => false,
        };
        if gone {
            out.push(Loss {
                what: i.what,
                atoms: Some(i.atom).filter(|&x| x != 0).into_iter().collect(),
                bonds: Some(i.bond).filter(|&x| x != 0).into_iter().collect(),
                objects: Some(i.object).filter(|&x| x != 0).into_iter().collect(),
            });
        }
    }
    if probed.contains(&"geometry") {
        out.push(Loss {
            what: "geometry",
            atoms: d.atoms.iter().map(|a| a.id).collect(),
            bonds: Vec::new(),
            objects: Vec::new(),
        });
    }
    let first = |l: &Loss| {
        let ids = l.atoms.iter().chain(&l.bonds).chain(&l.objects);
        ids.min().copied().unwrap_or(0)
    };
    out.sort_by_key(|l| (l.what, first(l)));
    Ok(out)
}

/// The document `loss/1`, written by hand with the keys in a fixed order: `format`, `target`, `items` (`what`, `atoms`, `bonds`,
/// `objects`, each list ascending and always written).
pub fn json(target: &str, items: &[Loss]) -> String {
    let list = |v: &[u32]| {
        let ids: Vec<String> = v.iter().map(u32::to_string).collect();
        format!("[{}]", ids.join(","))
    };
    let rows: Vec<String> = items
        .iter()
        .map(|l| {
            format!(
                "{{\"what\":{},\"atoms\":{},\"bonds\":{},\"objects\":{}}}",
                serde_json::Value::from(l.what),
                list(&l.atoms),
                list(&l.bonds),
                list(&l.objects)
            )
        })
        .collect();
    format!(
        "{{\"format\":\"{FORMAT}\",\"target\":{},\"items\":[{}]}}",
        serde_json::Value::from(target),
        rows.join(",")
    )
}

/// The table as the markdown of `docs/mzdraw-formats.md` (section "What a format loses"): a row per feature, a column per format;
/// `yes` carried, `no` dropped or changed, `drawing` decided by the drawing. A test holds the file to this text.
pub fn table_markdown() -> String {
    let mut s = format!("| feature | {} |\n|---|", FORMATS.join(" | "));
    s.push_str(&"---|".repeat(FORMATS.len()));
    s.push('\n');
    for (what, row) in TABLE {
        let cells: Vec<&str> = row
            .iter()
            .map(|c| match c {
                Carry::Yes => "yes",
                Carry::No => "no",
                Carry::Probe => "drawing",
            })
            .collect();
        s.push_str(&format!("| `{what}` | {} |\n", cells.join(" | ")));
    }
    s
}
