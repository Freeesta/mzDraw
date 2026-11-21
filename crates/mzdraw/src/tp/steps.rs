//! Named transformation steps (tp-10): a fixed table of standard steps, each with the kind of site it acts on, a site test on the
//! record of `mzlab-mol`, and the edit it expands to. The person or the agent chooses the step AND the site; nothing is ranked and
//! nothing is predicted. The editor only refuses a step that does not fit the site (`error.draw2.tpSite`). A step expands to a
//! patch of plain `draw_edit/1` operations written on the parent's ids (the patch language of `tp:derive`), and the delta of the
//! derived copy is checked against the table (`tp::derive`), so a wrong edit cannot ship.
//!
//! Hydrogen counts come from the record (`Document::to_mol`), not from the drawing. Limits, said plainly: an atom with a label
//! (an abbreviation or an R group) is never a site and is an unknown neighbour (it is neither N nor O); an explicit hydrogen drawn
//! as an atom of element H is a neighbour like any other and is not looked through; aromaticity is perceived by `mzlab-mol` from the
//! Kekule form, so a bond of an aromatic ring is never a site of a bond step; no stereo-chemistry is created.

use super::NEW;
use crate::doc::Document;
use crate::ops::Op;
use mzlab_mol::{delta, Mol};
use std::cell::OnceCell;
use std::collections::BTreeMap;

/// What a step acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Atom,
    Bond,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Atom => "atom",
            Kind::Bond => "bond",
        }
    }
}

/// One row of the table.
pub struct Step {
    pub name: &'static str,
    /// The formula delta in the canonical spelling of `mzlab_mol::delta::print` (gains first, Hill order).
    pub delta: &'static str,
    pub kind: Kind,
    test: fn(&View, usize) -> bool,
    edit: fn(&View, usize) -> Vec<Op>,
}

/// The table. Deltas: `-CH2`, `-H2`, `+O`, `+H2O`, `+C6H8O6` are listed in the mass-shift table of Molecules 2019, 24:1203, Table 4
/// (https://pmc.ncbi.nlm.nih.gov/articles/PMC6480032/table/molecules-24-01203-t004/, study `docs/study/tp-draw.md` section 1 T11);
/// `+O3S` (SO3 in Hill order) and `+C2H2O` are computed from the formulas (not read in that source); `+H-Cl`, `+H-Br` and `-CO2` are computed. The
/// delta of every row is also checked on every derive against the counts of the record, so a row cannot say more than it does.
pub const STEPS: &[Step] = &[
    // T11: +O
    Step {
        name: "hydroxylation",
        delta: "+O",
        kind: Kind::Atom,
        test: |v, i| v.neutral("C", i) && v.h[i] >= 1,
        edit: hydroxylation,
    },
    // T11: -CH2
    Step {
        name: "n-demethylation",
        delta: "-CH2",
        kind: Kind::Atom,
        test: |v, i| v.methyl(i, "N"),
        edit: demethylation,
    },
    // T11: -CH2
    Step {
        name: "o-demethylation",
        delta: "-CH2",
        kind: Kind::Atom,
        test: |v, i| v.methyl(i, "O"),
        edit: demethylation,
    },
    // T11: +O
    Step {
        name: "n-oxidation",
        delta: "+O",
        kind: Kind::Atom,
        test: n_oxide_site,
        edit: n_oxidation,
    },
    // T11: -H2
    Step {
        name: "dehydrogenation",
        delta: "-H2",
        kind: Kind::Bond,
        test: |v, b| {
            let (p, q) = v.ends(b);
            v.bond(b).o == 1
                && v.neutral("C", p)
                && v.neutral("C", q)
                && v.h[p] >= 1
                && v.h[q] >= 1
                && !v.aromatic_bond(b)
        },
        edit: dehydrogenation,
    },
    // T11: +H2O
    Step {
        name: "hydration",
        delta: "+H2O",
        kind: Kind::Bond,
        test: |v, b| {
            let (p, q) = v.ends(b);
            v.bond(b).o == 2 && v.neutral("C", p) && v.neutral("C", q) && !v.aromatic_bond(b)
        },
        edit: hydration,
    },
    // computed: Cl lost, H gained
    Step {
        name: "dechlorination",
        delta: "+H-Cl",
        kind: Kind::Atom,
        test: |v, i| v.halogen("Cl", i),
        edit: dehalogenation,
    },
    // computed: Br lost, H gained
    Step {
        name: "debromination",
        delta: "+H-Br",
        kind: Kind::Atom,
        test: |v, i| v.halogen("Br", i),
        edit: dehalogenation,
    },
    // T11: +H2O
    Step {
        name: "hydrolysis",
        delta: "+H2O",
        kind: Kind::Bond,
        test: |v, b| v.acyl_bond(b).is_some(),
        edit: hydrolysis,
    },
    // computed: +C2H2O
    Step {
        name: "acetylation",
        delta: "+C2H2O",
        kind: Kind::Atom,
        test: |v, i| v.donor(i),
        edit: |v, i| attach(v, i, "Ac"),
    },
    // computed: +SO3
    Step {
        name: "sulfation",
        delta: "+O3S",
        kind: Kind::Atom,
        test: |v, i| v.neutral("O", i) && v.h[i] >= 1,
        edit: |v, i| attach(v, i, "SO3H"),
    },
    // T11: +C6H8O6
    Step {
        name: "glucuronidation",
        delta: "+C6H8O6",
        kind: Kind::Atom,
        test: |v, i| v.donor(i),
        edit: |v, i| attach(v, i, "GlcA"),
    },
    // computed: -CO2
    Step {
        name: "decarboxylation",
        delta: "-CO2",
        kind: Kind::Atom,
        test: |v, i| v.carboxyl(i).is_some(),
        edit: decarboxylation,
    },
];

/// The row of a step name.
pub fn step(name: &str) -> Option<&'static Step> {
    STEPS.iter().find(|s| s.name == name)
}

/// The name of the one step whose delta is `delta`; `None` when no step or several steps have it (a delta shared by two steps
/// names none: no guess).
pub fn name_of(delta: &str) -> Option<&'static str> {
    let mut it = STEPS.iter().filter(|s| s.delta == delta);
    match (it.next(), it.next()) {
        (Some(s), None) => Some(s.name),
        _ => None,
    }
}

/// The most atoms of a structure a step is applied to (a hostile size is refused, never slow).
pub const MAX_ATOMS: usize = super::MAX_ATOMS;

pub(crate) const STEP: &str = "error.draw2.tpStep";
pub(crate) const SITE: &str = "error.draw2.tpSite";
const NO_ATOM: &str = "error.draw2.noAtom";
const SIZE: &str = "error.draw2.tpSize";

/// The sites of a step in a structure: ascending ids of atoms or of bonds.
#[derive(Clone, Debug)]
pub struct Sites {
    pub step: &'static Step,
    pub ids: Vec<u32>,
}

impl std::fmt::Debug for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Step({})", self.name)
    }
}

impl Sites {
    /// `sites/1`.
    pub fn json(&self) -> String {
        serde_json::json!({
            "format": "sites/1",
            "step": self.step.name,
            "delta": self.step.delta,
            "kind": self.step.kind.name(),
            "ids": self.ids,
        })
        .to_string()
    }
}

/// `steps/1`: the table.
pub fn steps_json() -> String {
    let rows: Vec<serde_json::Value> = STEPS
        .iter()
        .map(|s| serde_json::json!({"name": s.name, "delta": s.delta, "kind": s.kind.name()}))
        .collect();
    serde_json::json!({"format": "steps/1", "steps": rows}).to_string()
}

/// Every atom id (kind atom) or bond id (kind bond) of the structure that holds the atom `of` where the site test of `step` holds,
/// ascending. A list to choose from, never a ranking.
pub fn sites(doc: &Document, of: u32, step_name: &str) -> Result<Sites, &'static str> {
    let step = step(step_name).ok_or(STEP)?;
    let part = structure_of(doc, of)?;
    let v = View::new(&part);
    let mut ids: Vec<u32> = match step.kind {
        Kind::Atom => (0..part.atoms.len())
            .filter(|&i| (step.test)(&v, i))
            .map(|i| part.atoms[i].id)
            .collect(),
        Kind::Bond => (0..part.bonds.len())
            .filter(|&b| (step.test)(&v, b))
            .map(|b| part.bonds[b].id)
            .collect(),
    };
    ids.sort_unstable();
    Ok(Sites { step, ids })
}

/// The patch (operations on the parent's ids, a new atom named `NEW + k`) of `step` at `site`, with the row. A site where the test
/// fails (or that is not an atom / bond of the structure of `of`, as the step's kind says) is `error.draw2.tpSite`.
pub fn patch(
    doc: &Document,
    of: u32,
    step_name: &str,
    site: u32,
) -> Result<(&'static Step, Vec<Op>), &'static str> {
    let step = step(step_name).ok_or(STEP)?;
    let part = structure_of(doc, of)?;
    let v = View::new(&part);
    let at = match step.kind {
        Kind::Atom => part.atoms.iter().position(|a| a.id == site),
        Kind::Bond => part.bonds.iter().position(|b| b.id == site),
    };
    match at {
        Some(i) if (step.test)(&v, i) => Ok((step, (step.edit)(&v, i))),
        _ => Err(SITE),
    }
}

/// The formula counts of a document as the record has them (every atom with its hydrogens; an abbreviation counts as the atoms it
/// stands for), by element.
fn counts(doc: &Document) -> BTreeMap<String, i64> {
    let mut c: BTreeMap<String, i64> = BTreeMap::new();
    for a in &doc.to_mol().atoms {
        *c.entry(a.el.clone()).or_default() += 1;
        *c.entry("H".into()).or_default() += i64::from(a.h);
    }
    c
}

/// The delta `after - before` of two documents in the canonical spelling.
pub(crate) fn delta_between(before: &Document, after: &Document) -> String {
    let mut sum = counts(after);
    for (el, n) in counts(before) {
        *sum.entry(el).or_default() -= n;
    }
    let list: delta::Counts = sum.into_iter().filter(|e| e.1 != 0).collect();
    delta::print(&list)
}

/// The connected structure that holds the atom `of`, as a document of its own (atoms and bonds in the order of the drawing).
fn structure_of(doc: &Document, of: u32) -> Result<Document, &'static str> {
    let index = doc.index_map();
    let start = *index.get(&of).ok_or(NO_ATOM)?;
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); doc.atoms.len()];
    for b in &doc.bonds {
        if let (Some(&p), Some(&q)) = (index.get(&b.a), index.get(&b.b)) {
            adj[p].push(q);
            adj[q].push(p);
        }
    }
    let mut seen = vec![false; doc.atoms.len()];
    seen[start] = true;
    let mut stack = vec![start];
    let mut count = 1usize;
    while let Some(i) = stack.pop() {
        for &j in &adj[i] {
            if !seen[j] {
                seen[j] = true;
                count += 1;
                if count > MAX_ATOMS {
                    return Err(SIZE);
                }
                stack.push(j);
            }
        }
    }
    Ok(Document {
        atoms: (doc.atoms.iter().enumerate())
            .filter(|(i, _)| seen[*i])
            .map(|(_, a)| a.clone())
            .collect(),
        bonds: (doc.bonds.iter())
            .filter(|b| index.get(&b.a).is_some_and(|&i| seen[i]))
            .cloned()
            .collect(),
        next: doc.next,
        ..Document::default()
    })
}

/// What the site tests and the edits read: one structure with the hydrogen counts of its record, its neighbours and (when a
/// step asks) its aromatic flags.
pub struct View<'a> {
    doc: &'a Document,
    mol: Mol,
    /// Hydrogens of every atom as the record counts them.
    h: Vec<u32>,
    /// Per atom: (neighbour atom index, bond index).
    adj: Vec<Vec<(usize, usize)>>,
    /// Per bond: its two atom indices.
    ends: Vec<(usize, usize)>,
    aromatic: OnceCell<(Vec<bool>, Vec<bool>)>,
}

impl<'a> View<'a> {
    fn new(doc: &'a Document) -> View<'a> {
        let mol = doc.to_mol();
        let h = (0..doc.atoms.len())
            .map(|i| u32::from(mol.atoms[i].h))
            .collect();
        let index = doc.index_map();
        let mut adj = vec![Vec::new(); doc.atoms.len()];
        let mut ends = Vec::with_capacity(doc.bonds.len());
        for (k, b) in doc.bonds.iter().enumerate() {
            let (p, q) = (index[&b.a], index[&b.b]);
            adj[p].push((q, k));
            adj[q].push((p, k));
            ends.push((p, q));
        }
        View {
            doc,
            mol,
            h,
            adj,
            ends,
            aromatic: OnceCell::new(),
        }
    }

    fn ends(&self, b: usize) -> (usize, usize) {
        self.ends[b]
    }

    fn bond(&self, b: usize) -> &crate::doc::Bond {
        &self.doc.bonds[b]
    }

    fn id(&self, i: usize) -> u32 {
        self.doc.atoms[i].id
    }

    /// The aromatic flags of the atoms and bonds of the structure, perceived once and only when a test asks. A structure is connected,
    /// so one with fewer bonds than atoms has no ring and nothing in it is aromatic (the perception is not run: it is not linear on
    /// a long chain).
    fn flags(&self) -> &(Vec<bool>, Vec<bool>) {
        self.aromatic.get_or_init(|| {
            if self.doc.bonds.len() < self.doc.atoms.len() {
                (
                    vec![false; self.doc.atoms.len()],
                    vec![false; self.doc.bonds.len()],
                )
            } else {
                self.mol.aromatic_flags()
            }
        })
    }

    fn aromatic_bond(&self, b: usize) -> bool {
        self.flags().1[b]
    }

    fn aromatic_atom(&self, i: usize) -> bool {
        self.flags().0[i]
    }

    /// Whether the atom has no label and the element `el`.
    fn is(&self, el: &str, i: usize) -> bool {
        let a = &self.doc.atoms[i];
        a.lab.is_empty() && a.el == el
    }

    /// `is`, with no charge and no radical.
    fn neutral(&self, el: &str, i: usize) -> bool {
        let a = &self.doc.atoms[i];
        self.is(el, i) && a.chg == 0 && a.rad == 0
    }

    /// A methyl: a carbon with three hydrogens and one neighbour, of element `x`.
    fn methyl(&self, i: usize, x: &str) -> bool {
        self.neutral("C", i)
            && self.h[i] == 3
            && matches!(self.adj[i].as_slice(), [(n, _)] if self.is(x, *n))
    }

    /// A terminal halogen `el`.
    fn halogen(&self, el: &str, i: usize) -> bool {
        self.neutral(el, i) && self.adj[i].len() == 1
    }

    /// An N or an O with a hydrogen and no charge (an N-H, an O-H).
    fn donor(&self, i: usize) -> bool {
        (self.neutral("N", i) || self.neutral("O", i)) && self.h[i] >= 1
    }

    /// The carbonyl oxygen of a carbon: a terminal O doubly bonded to it.
    fn carbonyl_o(&self, c: usize) -> Option<(usize, usize)> {
        self.adj[c]
            .iter()
            .copied()
            .find(|&(n, b)| self.bond(b).o == 2 && self.neutral("O", n) && self.adj[n].len() == 1)
    }

    /// The ends (carbonyl carbon, hetero atom) of the single bond `b` of an ester or an amide: `C(=O)-O-R` or `C(=O)-N`, the hetero
    /// atom O or N with another heavy neighbour, the bond not in an aromatic ring.
    fn acyl_bond(&self, b: usize) -> Option<(usize, usize)> {
        if self.bond(b).o != 1 || self.aromatic_bond(b) {
            return None;
        }
        let (p, q) = self.ends[b];
        [(p, q), (q, p)].into_iter().find(|&(c, x)| {
            self.neutral("C", c)
                && self.carbonyl_o(c).is_some()
                && (self.is("O", x) || self.is("N", x))
                && self.adj[x].len() >= 2
        })
    }

    /// The carbon of a carboxylic acid `R-C(=O)-OH` with its carbonyl oxygen, its hydroxyl oxygen, the third neighbour and the three
    /// bonds (to the carbonyl O, to the hydroxyl O, to the third neighbour).
    fn carboxyl(&self, c: usize) -> Option<[(usize, usize); 3]> {
        if !self.neutral("C", c) || self.adj[c].len() != 3 {
            return None;
        }
        let (co, cb) = self.carbonyl_o(c)?;
        let hydroxyl = |&(n, b): &(usize, usize)| {
            self.bond(b).o == 1 && self.neutral("O", n) && self.adj[n].len() == 1 && self.h[n] >= 1
        };
        let mut it = self.adj[c].iter().filter(|e| hydroxyl(e));
        let oh = *it.next()?;
        if it.next().is_some() {
            return None;
        }
        let other = *self.adj[c].iter().find(|&&(n, _)| n != co && n != oh.0)?;
        // the third neighbour is not itself a terminal oxygen (carbonic acid is not a carboxylic acid of a carbon chain)
        if self.adj[other.0].len() == 1 && self.is("O", other.0) {
            return None;
        }
        Some([(co, cb), oh, other])
    }

    /// The typical bond length of the structure (the median), 1 when there is none.
    fn length(&self) -> f64 {
        let mut l: Vec<f64> = (self.doc.bonds.iter().enumerate())
            .map(|(k, _)| {
                let (p, q) = self.ends[k];
                let (a, b) = (&self.doc.atoms[p], &self.doc.atoms[q]);
                libm::hypot(a.x - b.x, a.y - b.y)
            })
            .collect();
        l.sort_by(f64::total_cmp);
        match l.get(l.len() / 2) {
            Some(&m) if m > 1e-6 => m,
            _ => 1.0,
        }
    }

    /// Where a new atom bonded to the atom `i` goes: one bond length away, along the bisector of the widest angle between the
    /// neighbours of `i` (the neighbour `skip` is not counted: its bond is being cut). Nothing else of the drawing is looked at.
    fn free(&self, i: usize, skip: Option<usize>) -> (f64, f64) {
        let a = &self.doc.atoms[i];
        let mut angles: Vec<f64> = (self.adj[i].iter())
            .filter(|(n, _)| Some(*n) != skip)
            .map(|&(n, _)| {
                let o = &self.doc.atoms[n];
                libm::atan2(o.y - a.y, o.x - a.x)
            })
            .collect();
        angles.sort_by(f64::total_cmp);
        let mut best = (0.0, 0.0);
        for (k, &start) in angles.iter().enumerate() {
            let end = angles
                .get(k + 1)
                .copied()
                .unwrap_or(angles[0] + 2.0 * core::f64::consts::PI);
            if end - start > best.0 {
                best = (end - start, start + (end - start) / 2.0);
            }
        }
        let l = self.length();
        (a.x + l * libm::cos(best.1), a.y + l * libm::sin(best.1))
    }

    /// The operation that moves an explicit hydrogen count of the atom `i` by `d` (nothing for an atom whose hydrogens follow the
    /// valence).
    fn hydrogens(&self, i: usize, d: i32) -> Option<Op> {
        let h = self.doc.atoms[i].h?;
        Some(set(self.id(i), |s| {
            s.h = Some((i32::from(h) + d).clamp(0, 255) as u8)
        }))
    }
}

/// Fields of an `atom.set` to fill in.
struct Setting {
    chg: Option<i8>,
    h: Option<u8>,
    lab: Option<String>,
}

fn set(id: u32, fill: impl FnOnce(&mut Setting)) -> Op {
    let mut s = Setting {
        chg: None,
        h: None,
        lab: None,
    };
    fill(&mut s);
    Op::AtomSet {
        id,
        el: None,
        chg: s.chg,
        iso: None,
        rad: None,
        h: s.h,
        lab: s.lab,
        map: None,
    }
}

fn add_atom(el: &str, at: (f64, f64)) -> Op {
    Op::AtomAdd {
        id: None,
        el: el.into(),
        x: at.0,
        y: at.1,
        map: None,
    }
}

fn bond(a: u32, b: u32) -> Op {
    Op::BondAdd {
        id: None,
        a,
        b,
        o: 1,
        s: 0,
    }
}

fn hydroxylation(v: &View, i: usize) -> Vec<Op> {
    let mut ops = vec![add_atom("O", v.free(i, None)), bond(v.id(i), NEW + 1)];
    ops.extend(v.hydrogens(i, -1));
    ops
}

fn demethylation(v: &View, i: usize) -> Vec<Op> {
    let (n, b) = v.adj[i][0];
    let mut ops = vec![Op::Remove {
        atoms: vec![v.id(i)],
        bonds: vec![v.bond(b).id],
        objects: Vec::new(),
    }];
    ops.extend(v.hydrogens(n, 1));
    ops
}

/// A tertiary amine (three single bonds, not aromatic) or a pyridine-type N (aromatic, two neighbours, no hydrogen), neutral.
fn n_oxide_site(v: &View, i: usize) -> bool {
    if !v.neutral("N", i) || v.h[i] != 0 {
        return false;
    }
    let orders: Vec<u8> = v.adj[i].iter().map(|&(_, b)| v.bond(b).o).collect();
    if v.aromatic_atom(i) {
        orders.len() == 2 && orders.iter().map(|&o| u32::from(o)).sum::<u32>() == 3
    } else {
        orders == [1, 1, 1]
    }
}

fn n_oxidation(v: &View, i: usize) -> Vec<Op> {
    vec![
        add_atom("O", v.free(i, None)),
        bond(v.id(i), NEW + 1),
        set(v.id(i), |s| s.chg = Some(1)),
        set(NEW + 1, |s| s.chg = Some(-1)),
    ]
}

fn dehydrogenation(v: &View, b: usize) -> Vec<Op> {
    let (p, q) = v.ends(b);
    let mut ops = vec![Op::BondSet {
        id: v.bond(b).id,
        o: Some(2),
        s: None,
        rev: false,
    }];
    ops.extend(v.hydrogens(p, -1));
    ops.extend(v.hydrogens(q, -1));
    ops
}

/// The end with fewer hydrogens takes the OH (ties by id), the other end takes the hydrogen.
fn hydration(v: &View, b: usize) -> Vec<Op> {
    let (p, q) = v.ends(b);
    let (o_end, h_end) = if (v.h[p], v.id(p)) <= (v.h[q], v.id(q)) {
        (p, q)
    } else {
        (q, p)
    };
    let mut ops = vec![
        Op::BondSet {
            id: v.bond(b).id,
            o: Some(1),
            s: None,
            rev: false,
        },
        add_atom("O", v.free(o_end, None)),
        bond(v.id(o_end), NEW + 1),
    ];
    ops.extend(v.hydrogens(h_end, 1));
    ops
}

fn dehalogenation(v: &View, i: usize) -> Vec<Op> {
    let (n, b) = v.adj[i][0];
    let mut ops = vec![Op::Remove {
        atoms: vec![v.id(i)],
        bonds: vec![v.bond(b).id],
        objects: Vec::new(),
    }];
    ops.extend(v.hydrogens(n, 1));
    ops
}

fn hydrolysis(v: &View, b: usize) -> Vec<Op> {
    let Some((c, x)) = v.acyl_bond(b) else {
        return Vec::new();
    };
    let mut ops = vec![
        Op::Remove {
            atoms: Vec::new(),
            bonds: vec![v.bond(b).id],
            objects: Vec::new(),
        },
        add_atom("O", v.free(c, Some(x))),
        bond(v.id(c), NEW + 1),
    ];
    ops.extend(v.hydrogens(x, 1));
    ops
}

/// A new atom labelled `label` bonded to the atom `i`, which loses one hydrogen.
fn attach(v: &View, i: usize, label: &str) -> Vec<Op> {
    let mut ops = vec![
        add_atom("C", v.free(i, None)),
        set(NEW + 1, |s| s.lab = Some(label.into())),
        bond(v.id(i), NEW + 1),
    ];
    ops.extend(v.hydrogens(i, -1));
    ops
}

fn decarboxylation(v: &View, i: usize) -> Vec<Op> {
    let Some([(co, cb), (oh, hb), (x, xb)]) = v.carboxyl(i) else {
        return Vec::new();
    };
    let mut ops = vec![Op::Remove {
        atoms: vec![v.id(i), v.id(co), v.id(oh)],
        bonds: vec![v.bond(cb).id, v.bond(hb).id, v.bond(xb).id],
        objects: Vec::new(),
    }];
    ops.extend(v.hydrogens(x, 1));
    ops
}
