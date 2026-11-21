//! A drawn parent → arrow → product as a transformation record `trasf/1` (plan-12), and the way back.
//!
//! The record is built from the two `mol/1` records of the drawing and the atom maps (plan-11): a parent atom and a product atom
//! with the same map number are the same atom; an atom without a map on the parent side is lost, on the product side it is added
//! (the convention of mapped reaction SMILES). The writer is `mzlab_mol::Transformation` (`canonical` puts the map on the canonical
//! indices of the two records, `validate_against` checks it); this module only collects the inputs. A product whose map is not
//! complete is refused: the uncertain form of `trasf/1` needs the probability `p` of every site, and a drawing has none (a most
//! likely site is never written as a certain one).
//!
//! The file written by `Document::to_trasf` is three JSON lines: the record `trasf/1`, then the parent and the product as `mol/1` on
//! the canonical order the record's map is written on (so their hashes are the ones the record names). Read, the record may come with
//! its two records in any order; the records named by their hashes are found among the lines.

use crate::doc::{Document, ObjKind};
use crate::io::Imported;
use mzlab_mol::{delta, Mol, Transformation, TRANSFORMATION_FORMAT};

/// Not one arrow with one structure before it and one after it.
pub const SHAPE: &str = "error.draw2.trasfShape";
/// The map is not complete: the atoms (ids of the drawing) are named by `TrasfError::atoms`.
pub const MAP: &str = "error.draw2.trasfMap";
/// A group label or an R group stands in the drawing: its atoms carry no map.
pub const LABEL: &str = "error.draw2.trasfLabel";
/// `mzlab-mol` refused the record (an element changes outside the centre, a stereo centre copied as defined).
pub const RECORD: &str = "error.draw2.trasfRecord";
/// The file is not a `trasf/1` record with its two records.
pub const READ: &str = "error.draw2.trasf";
/// A record without a product (the uncertain form) has nothing to draw after the arrow.
pub const UNCERTAIN: &str = "error.draw2.trasfUncertain";
/// The record carries `rule_set`, `rule_version`, `stereo_rule`, `level` or `evidence`, which a drawing cannot keep.
pub const FIELDS: &str = "error.draw2.trasfFields";

/// Why a drawing is not a transformation: an i18n key and the atoms (ids of the drawing) the key is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrasfError {
    pub key: &'static str,
    pub atoms: Vec<u32>,
}

fn fail<T>(key: &'static str, atoms: Vec<u32>) -> Result<T, TrasfError> {
    Err(TrasfError { key, atoms })
}

/// The record and the two records it names, as `Document::trasf` builds them.
pub struct Trasf {
    pub record: Transformation,
    /// On the canonical order of the record.
    pub parent: Mol,
    pub product: Mol,
}

impl Trasf {
    /// The file: the record, the parent and the product, one JSON text per line.
    pub fn text(&self) -> String {
        format!(
            "{}\n{}\n{}\n",
            self.record.to_json(),
            self.parent.to_json(),
            self.product.to_json()
        )
    }
}

/// Element counts of a record, hydrogens included.
fn counts(m: &Mol) -> delta::Counts {
    let mut c: delta::Counts = Vec::new();
    let mut add = |el: &str, n: i64| match c.iter_mut().find(|e| e.0 == el) {
        Some(e) => e.1 += n,
        None => c.push((el.to_string(), n)),
    };
    for a in &m.atoms {
        add(&a.el, 1);
        add("H", a.h as i64);
    }
    c.retain(|e| e.1 != 0);
    c
}

/// The change of formula from `parent` to `product` in the spelling of `delta`.
fn formula_delta(parent: &Mol, product: &Mol) -> String {
    let mut d = counts(product);
    for (el, n) in counts(parent) {
        match d.iter_mut().find(|e| e.0 == el) {
            Some(e) => e.1 -= n,
            None => d.push((el, -n)),
        }
    }
    d.retain(|e| e.1 != 0);
    delta::print(&d)
}

/// `(atom index, other end, order)` of every bond of an atom.
fn bonds_of(m: &Mol, i: u32) -> Vec<(u32, u8)> {
    let mut v: Vec<(u32, u8)> = m
        .bonds
        .iter()
        .filter_map(|b| {
            if b.a == i {
                Some((b.b, b.o))
            } else if b.b == i {
                Some((b.a, b.o))
            } else {
                None
            }
        })
        .collect();
    v.sort_unstable();
    v
}

impl Document {
    /// The two structures of a drawing with one arrow: the one before its tail and the one past its head. Anything else (no arrow
    /// or two, several structures on a side, a structure alongside the arrow) is `SHAPE`; nothing is left out silently.
    fn trasf_sides(&self) -> Result<(Document, Document, String), TrasfError> {
        // a plus sign or a text has no place in the record: named, never left out silently
        let others: Vec<u32> = (self.objects.iter())
            .filter(|o| o.t != ObjKind::Arrow)
            .map(|o| o.id)
            .collect();
        if !others.is_empty() {
            return fail(SHAPE, others);
        }
        let mut arrows = self.objects.iter().filter(|o| o.t == ObjKind::Arrow);
        let (Some(arrow), None) = (arrows.next(), arrows.next()) else {
            return fail(SHAPE, Vec::new());
        };
        let (dx, dy) = (arrow.x2 - arrow.x, arrow.y2 - arrow.y);
        let (mut before, mut after) = (Vec::new(), Vec::new());
        for ids in crate::layout::pieces(self) {
            let piece = self.part(&ids);
            let n = piece.atoms.len() as f64;
            let (cx, cy) = piece
                .atoms
                .iter()
                .fold((0.0, 0.0), |c, a| (c.0 + a.x / n, c.1 + a.y / n));
            let t = ((cx - arrow.x) * dx + (cy - arrow.y) * dy) / (dx * dx + dy * dy);
            if t < 0.0 {
                before.push(piece);
            } else if t > 1.0 {
                after.push(piece);
            } else {
                return fail(SHAPE, ids);
            }
        }
        match (
            before.pop(),
            after.pop(),
            before.is_empty(),
            after.is_empty(),
        ) {
            (Some(p), Some(q), true, true) => Ok((p, q, arrow.text.clone())),
            _ => fail(SHAPE, Vec::new()),
        }
    }

    /// The transformation of the drawing: the record of `mzlab-mol` and its two records. The errors are `SHAPE`, `LABEL`, `MAP` (with
    /// the atoms that have no partner) and `RECORD`.
    pub fn trasf(&self) -> Result<Trasf, TrasfError> {
        let (p, q, rule) = self.trasf_sides()?;
        let labelled: Vec<u32> = (p.atoms.iter().chain(&q.atoms))
            .filter(|a| !a.lab.is_empty())
            .map(|a| a.id)
            .collect();
        if !labelled.is_empty() {
            return fail(LABEL, labelled);
        }
        let (pm, qm) = (p.to_mol_stereo(), q.to_mol_stereo());
        if pm.atoms.len() != p.atoms.len() || qm.atoms.len() != q.atoms.len() {
            return fail(RECORD, Vec::new());
        }
        let ok = self.pair_maps(&p, &q)?;
        let (np, nq) = (p.atoms.len(), q.atoms.len());
        let mut map: Vec<(u32, Option<u32>)> = (0..np as u32).map(|a| (a, None)).collect();
        let mut hit = vec![false; nq];
        for &(a, b) in &ok {
            map[a].1 = Some(b as u32);
            hit[b] = true;
        }
        let added: Vec<u32> = (0..nq as u32).filter(|&b| !hit[b as usize]).collect();
        // the centre: the mapped atoms whose element, charge, hydrogens or bonds are not the same in the product
        let image = |a: u32| map[a as usize].1;
        let centre: Vec<u32> = (0..np as u32)
            .filter(|&a| {
                let Some(b) = image(a) else { return false };
                let (x, y) = (&pm.atoms[a as usize], &qm.atoms[b as usize]);
                if x != y {
                    return true;
                }
                let theirs = bonds_of(&qm, b);
                let mine: Vec<(u32, u8)> = bonds_of(&pm, a)
                    .into_iter()
                    .filter_map(|(n, o)| image(n).map(|m| (m, o)))
                    .collect();
                let mut mine = mine;
                mine.sort_unstable();
                mine != theirs
            })
            .collect();
        let record = Transformation {
            format: TRANSFORMATION_FORMAT.into(),
            parent: pm.core_hash_hex(),
            product: Some(qm.core_hash_hex()),
            delta: formula_delta(&pm, &qm),
            rule,
            rule_set: String::new(),
            rule_version: String::new(),
            map,
            added,
            centre,
            sites: Vec::new(),
            region: None,
            stereo_rule: None,
            level: None,
            evidence: Vec::new(),
        };
        let record = record
            .canonical(&pm, Some(&qm))
            .or_else(|_| fail(RECORD, Vec::new()))?;
        let renumbered = |m: &Mol| m.renumbered(&m.canonical_order());
        match (renumbered(&pm), renumbered(&qm)) {
            (Ok(parent), Ok(product)) => Ok(Trasf {
                record,
                parent,
                product,
            }),
            _ => fail(RECORD, Vec::new()),
        }
    }

    /// Pairs the atoms of the two sides by map number: `(parent index, product index)`. A complete map has every number once on each
    /// side and on both, and covers every atom of the smaller structure (what is left on the larger one is lost or added).
    fn pair_maps(&self, p: &Document, q: &Document) -> Result<Vec<(usize, usize)>, TrasfError> {
        let numbers = |d: &Document| -> Vec<(u16, usize)> {
            d.atoms
                .iter()
                .enumerate()
                .filter(|(_, a)| a.map != 0)
                .map(|(i, a)| (a.map, i))
                .collect()
        };
        let (mp, mq) = (numbers(p), numbers(q));
        let mut bad: Vec<u32> = Vec::new();
        for (side, other, d) in [(&mp, &mq, p), (&mq, &mp, q)] {
            for &(n, i) in side {
                let twice = side.iter().filter(|x| x.0 == n).count() > 1;
                if twice || !other.iter().any(|x| x.0 == n) {
                    bad.push(d.atoms[i].id);
                }
            }
        }
        if !bad.is_empty() {
            return fail(MAP, bad);
        }
        let pairs: Vec<(usize, usize)> = mp
            .iter()
            .filter_map(|&(n, i)| mq.iter().find(|x| x.0 == n).map(|x| (i, x.1)))
            .collect();
        // the smaller structure must be covered; nothing mapped at all names every atom
        let smaller = p.atoms.len().min(q.atoms.len());
        if pairs.is_empty() || pairs.len() < smaller {
            let open: Vec<u32> = (p.atoms.iter().filter(|a| a.map == 0))
                .chain(q.atoms.iter().filter(|a| a.map == 0))
                .map(|a| a.id)
                .collect();
            return fail(MAP, open);
        }
        Ok(pairs)
    }

    /// The drawing as a `trasf/1` file (see the module): the record and the two records it names, one JSON text per line.
    pub fn to_trasf(&self) -> Result<String, TrasfError> {
        self.trasf().map(|t| t.text())
    }
}

/// A `trasf/1` file read as a drawing: the parent, an arrow and the product, the atoms the map pairs carry the same number
/// (1, 2, ... in the order of the parent). The error is an i18n key.
pub(crate) fn from_trasf(text: &str, info: &mut Imported) -> Result<Document, &'static str> {
    let mut record: Option<Transformation> = None;
    let mut mols: Vec<Mol> = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value = serde_json::from_str(line).map_err(|_| READ)?;
        if v["format"] == TRANSFORMATION_FORMAT {
            if record.is_some() {
                return Err(READ);
            }
            record = Some(Transformation::from_json(line).map_err(|_| READ)?);
        } else {
            mols.push(Mol::from_json(line).map_err(|_| READ)?);
        }
    }
    let t = record.ok_or(READ)?;
    // what a drawing cannot keep is refused, not dropped: only `rule` has a place (the text of the arrow)
    if t.product.is_none() {
        return Err(UNCERTAIN);
    }
    if !(t.rule_set.is_empty()
        && t.rule_version.is_empty()
        && t.stereo_rule.is_none()
        && t.level.is_none()
        && t.evidence.is_empty())
    {
        return Err(FIELDS);
    }
    let find = |hash: &str| mols.iter().find(|m| m.core_hash_hex() == hash);
    let parent = find(&t.parent).ok_or(READ)?;
    // an uncertain record has no product: there is nothing to draw after the arrow
    let product = find(t.product.as_deref().ok_or(READ)?).ok_or(READ)?;
    t.validate_against(parent, Some(product))
        .map_err(|_| READ)?;
    if t.map.len() > crate::doc::MAX_MAP as usize {
        return Err(READ);
    }
    let name = crate::NAME;
    let text = format!(
        "$RXN\n\n  {name}\n\n  1  1\n$MOL\n{}$MOL\n{}",
        parent.to_molfile(name),
        product.to_molfile(name)
    );
    let mut d = crate::rxn::from_rxn(&text, info)?;
    let np = parent.atoms.len();
    if d.atoms.len() != np + product.atoms.len() {
        return Err(READ);
    }
    let mut n = 0u16;
    for &(a, b) in &t.map {
        let Some(b) = b else { continue };
        n += 1;
        d.atoms[a as usize].map = n;
        d.atoms[np + b as usize].map = n;
    }
    if let Some(arrow) = d.objects.iter_mut().find(|o| o.t == ObjKind::Arrow) {
        arrow.text = t.rule.clone();
    }
    info.format = "trasf";
    Ok(d)
}
