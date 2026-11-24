//! Calculated values (arch-web-17): per connected structure and for a selection, the Hill formula, the elemental composition, the
//! monoisotopic mass, the average mass and the m/z of the adducts chosen in `display.adducts`; shown in the panel, optionally under
//! each structure (`display.values`), and written by the replay request field `values` as `values/1`.
//!
//! What is here and what is not. The core counts atoms on the record (`Document::to_mol`, abbreviations expanded): formula,
//! composition and charge need no table. A MASS needs the element masses, and this crate has none and must not get one: they come
//! from the engine `sel` of mzLab (`mzlab_feat::chem::sel`), which the bridge crate calls (`mzdraw-wasm/src/glue.rs`) and hands to
//! the editor as a `Measurer` (a plain function pointer: molfile text and adduct names in, numbers out). Without a measurer (the
//! native `mzdraw` replay, a test) the formulas are there and every mass is absent, said with `engine: false`, never made up.
//!
//! Cost: one union-find pass over the record (`scan`), the sub-record of each structure built once from per-structure lists, one
//! engine call per structure; never a scan of all atoms per structure. The work is capped (`MAX_ATOMS`, `MAX_STRUCTURES`):
//! above it the answer is `error.draw2.valuesSize`, while the formulas stay (they are linear and uncapped).

use crate::doc::Document;
use crate::label;
use mzlab_mol::{fixed, Mol};

/// The value of the key `format` of the answer.
pub const FORMAT: &str = "values/1";

/// Atoms of the record (abbreviations expanded) the engine is asked about in one call, and structures it is asked about.
pub const MAX_ATOMS: usize = 5_000;
pub const MAX_STRUCTURES: usize = 200;

/// The adducts `display.adducts` accepts, by name (the shifts come from the engine, never from this crate).
pub const ADDUCTS: [&str; 7] = [
    "[M+H]+",
    "[M+NH4]+",
    "[M+Na]+",
    "[M+K]+",
    "[M-H]-",
    "[M+Cl]-",
    "[M+HCOO]-",
];

/// The adducts a style starts with.
pub const DEFAULT_ADDUCTS: [&str; 2] = ["[M+H]+", "[M-H]-"];

/// What `display.values` draws under each structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// Nothing.
    #[default]
    Off,
    /// The Hill formula.
    Formula,
    /// The formula and the monoisotopic mass.
    Mass,
    /// One line per adduct: its name and its m/z.
    Mz,
}

impl Mode {
    pub fn parse(v: &str) -> Option<Mode> {
        match v {
            "off" => Some(Mode::Off),
            "formula" => Some(Mode::Formula),
            "mass" => Some(Mode::Mass),
            "mz" => Some(Mode::Mz),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Formula => "formula",
            Mode::Mass => "mass",
            Mode::Mz => "mz",
        }
    }
}

/// The adducts of a `display.adducts` text: names from `ADDUCTS` separated by commas, each at most once; an empty text is none.
pub fn parse_adducts(text: &str) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for name in text.split(',').map(str::trim).filter(|n| !n.is_empty()) {
        if !ADDUCTS.contains(&name) || out.iter().any(|o| o == name) {
            return None;
        }
        out.push(name.to_string());
    }
    Some(out)
}

/// The numbers the engine gives for one structure.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    /// Monoisotopic mass of the structure as drawn (neutral mass for a neutral one; the ion mass, without the electrons' correction,
    /// for a charged one).
    pub mass: f64,
    /// Average mass (conventional weights), `None` when an element has none.
    pub average: Option<f64>,
    /// m/z of a charged structure itself (mass minus the electrons, over the charge), `None` for a neutral one.
    pub ion_mz: Option<f64>,
    /// m/z of each adduct asked for, in the order asked; `None` where an adduct does not apply (a structure that has a charge).
    pub mz: Vec<Option<f64>>,
    /// Mass per cent of each element of the formula, in Hill order (elemental analysis, from the engine `analysis/1`); empty when the
    /// engine did not give it.
    pub pct: Vec<(String, f64)>,
}

/// The engine, seen from the core: a molfile (the structure alone) and the adduct names to give the numbers of the structure. The
/// error is an i18n key.
pub type Measurer = fn(&str, &[String]) -> Result<Measured, &'static str>;

/// The result for one structure, kept by the editor between changes. It is used only for the structure with this anchor and
/// formula, so a result a later edit left behind is never shown.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub anchor: u32,
    pub formula: String,
    /// The isotopes (`13C`, sorted) that the formula does not show but the mass does.
    pub iso: String,
    pub result: Result<Measured, &'static str>,
}

impl Entry {
    /// Whether the entry is the result for this structure as it is now.
    pub fn matches(&self, s: &Structure) -> bool {
        self.anchor == s.anchor && self.formula == s.formula && self.iso == s.iso
    }
}

/// One connected structure, counted on the record.
#[derive(Clone, Debug, PartialEq)]
pub struct Structure {
    /// The id of its first atom.
    pub anchor: u32,
    /// The ids of its atoms of the document, in pool order.
    pub ids: Vec<u32>,
    /// Its atoms on the record (an abbreviation counts as the atoms it stands for).
    pub atoms: usize,
    /// Hill formula with the net charge, as `outline` writes it.
    pub formula: String,
    /// The Hill order, hydrogens included, only elements that occur; `R` for an R group.
    pub composition: Vec<(String, u32)>,
    pub charge: i32,
    /// The atoms with a mass number, `13C15N` (sorted): the formula does not say them and the mass does.
    pub iso: String,
    /// Whether an atom has no element (an R group or another label): the formula is then not a mass.
    pub open: bool,
    /// Whether the structure holds a dative or a hydrogen bond: the engine reads the record, where it is a single bond, and
    /// counts the hydrogens of the single-bond reading, so no mass or descriptor is made of it (`error.draw2.valuesLink`).
    pub linked: bool,
    /// Whether the painter writes a name or a level under the structure (an annotation, or the data field `TP_ID`): the caption goes below it.
    pub annotated: bool,
    /// Its box in the document, `[x0, y0, x1, y1]`, y up.
    pub bbox: [f64; 4],
    /// The lines the owner chose to keep under it (`props`), empty when it follows `display.values`.
    pub cap: Vec<String>,
    members: Vec<usize>,
}

/// The structures of a document and the record they are counted on.
pub struct Scan {
    pub structures: Vec<Structure>,
    record: Mol,
    bonds_of: Vec<Vec<usize>>,
}

/// Counts, Hill formula and charge of the atoms `members` of the record; the flags say whether one is an R group or another label.
fn count(
    doc: &Document,
    m: &Mol,
    members: &[usize],
    cut: &[u32],
) -> (String, Vec<(String, u32)>, i32, bool) {
    let mut counts: Vec<(String, u32)> = Vec::new();
    let mut charge = 0i32;
    let mut open = false;
    let add = |counts: &mut Vec<(String, u32)>, el: &str, k: u32| match counts
        .iter_mut()
        .find(|c| c.0 == el)
    {
        Some(c) => c.1 += k,
        None => counts.push((el.to_string(), k)),
    };
    for (k, &i) in members.iter().enumerate() {
        // an atom of the document with a label that is not an abbreviation: an R group stands in for it, as `outline` counts
        let r = doc.atoms.get(i).is_some_and(|a| {
            !a.lab.is_empty() && !matches!(label::kind(&a.lab), Some(label::Kind::Group(_)))
        });
        if r {
            open = true;
            add(&mut counts, "R", 1);
        } else {
            add(&mut counts, &m.atoms[i].el, 1);
            add(
                &mut counts,
                "H",
                u32::from(m.atoms[i].h) + cut.get(k).copied().unwrap_or(0),
            );
            charge += i32::from(m.atoms[i].chg);
        }
    }
    let mut sorted = counts;
    let formula = crate::outline::hill(&mut sorted, charge);
    sorted.retain(|c| c.1 > 0);
    (formula, sorted, charge, open)
}

/// The sub-record of the atoms `members` (record indices, ascending) with the bonds `bonds` (indices into the record): atoms keep
/// their order, so a bond keeps `a < b`.
fn sub_record(m: &Mol, members: &[usize], bonds: &[usize], cut: &[u32]) -> Mol {
    let mut new_of = std::collections::HashMap::with_capacity(members.len());
    for (k, &i) in members.iter().enumerate() {
        new_of.insert(i as u32, k as u32);
    }
    let atoms = (members.iter().enumerate())
        .map(|(k, &i)| {
            let mut a = m.atoms[i].clone();
            // a bond cut by the selection is closed with hydrogen
            a.h =
                a.h.saturating_add(cut.get(k).copied().unwrap_or(0).min(255) as u8);
            a
        })
        .collect();
    let bonds = bonds
        .iter()
        .map(|&k| {
            let b = &m.bonds[k];
            mzlab_mol::Bond {
                a: new_of[&b.a],
                b: new_of[&b.b],
                o: b.o,
            }
        })
        .collect();
    Mol::new(atoms, bonds, Vec::new(), Vec::new())
}

/// Whether every element of a composition is one the descriptors are written for.
fn covered(c: &[(String, u32)]) -> bool {
    c.iter()
        .all(|(el, _)| crate::props::DESCRIPTOR_ELEMENTS.contains(&el.as_str()))
}

/// Splits the drawing into its connected structures, in the order of their first atom: one pass.
pub fn scan(doc: &Document) -> Scan {
    let m = doc.to_mol();
    let n = m.atoms.len();
    let mut up: Vec<usize> = (0..n).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for b in &m.bonds {
        let (a, z) = (root(&mut up, b.a as usize), root(&mut up, b.b as usize));
        // the smaller index stays the root, so structures come out in the order of their first atom
        up[a.max(z)] = a.min(z);
    }
    let mut slot = vec![usize::MAX; n];
    let mut lists: Vec<Vec<usize>> = Vec::new();
    for i in 0..n {
        let r = root(&mut up, i);
        if slot[r] == usize::MAX {
            slot[r] = lists.len();
            lists.push(Vec::new());
        }
        lists[slot[r]].push(i);
    }
    let mut bonds_of: Vec<Vec<usize>> = vec![Vec::new(); lists.len()];
    for (k, b) in m.bonds.iter().enumerate() {
        bonds_of[slot[root(&mut up, b.a as usize)]].push(k);
    }
    let structures = lists
        .into_iter()
        .enumerate()
        .map(|(si, members)| {
            let linked = bonds_of[si]
                .iter()
                .any(|&k| doc.bonds.get(k).is_some_and(crate::doc::Bond::is_link));
            let (formula, composition, charge, open) = count(doc, &m, &members, &[]);
            let mut isos: Vec<String> = (members.iter())
                .filter(|&&i| m.atoms[i].iso > 0)
                .map(|&i| format!("{}{}", m.atoms[i].iso, m.atoms[i].el))
                .collect();
            isos.sort();
            let ids: Vec<u32> = (members.iter())
                .filter_map(|&i| doc.atoms.get(i).map(|a| a.id))
                .collect();
            let mut bbox = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
            let mut annotated = false;
            let mut cap: Vec<String> = Vec::new();
            for &i in &members {
                if let Some(a) = doc.atoms.get(i) {
                    if cap.is_empty() {
                        cap = a.cap();
                    }
                    bbox = [
                        bbox[0].min(a.x),
                        bbox[1].min(a.y),
                        bbox[2].max(a.x),
                        bbox[3].max(a.y),
                    ];
                    annotated |= !a.ann().is_empty()
                        || a.data().iter().any(|(k, v)| k == "TP_ID" && !v.is_empty());
                }
            }
            Structure {
                anchor: ids.first().copied().unwrap_or(0),
                ids,
                atoms: members.len(),
                formula,
                composition,
                charge,
                iso: isos.concat(),
                open,
                linked,
                annotated,
                bbox,
                cap,
                members,
            }
        })
        .collect();
    Scan {
        structures,
        record: m,
        bonds_of,
    }
}

/// What the engine is given for one set of atoms: the molfile of their sub-record.
fn molfile(m: &Mol, members: &[usize], bonds: &[usize], cut: &[u32]) -> String {
    sub_record(m, members, bonds, cut).to_molfile(crate::NAME)
}

impl Scan {
    /// The descriptors of structure `k` (`props`), or the key that says why there are none: `error.draw2.valuesSize` (above the caps, checked
    /// before any work), `error.draw2.valuesOpen` (an R group), `error.draw2.propsElement` (an element outside
    /// `props::DESCRIPTOR_ELEMENTS`: the library would give it a default type, which is not a measure) or `error.draw2.propsCharged` (an atom
    /// with a formal charge: the descriptors are those of neutral molecules).
    pub fn props(&self, k: usize) -> Result<crate::props::Props, &'static str> {
        if self.record.atoms.len() > MAX_ATOMS || self.structures.len() > MAX_STRUCTURES {
            return Err("error.draw2.valuesSize");
        }
        let s = self.structures.get(k).ok_or("error.draw2.valuesOpen")?;
        if s.open {
            return Err("error.draw2.valuesOpen");
        }
        if s.linked {
            return Err("error.draw2.valuesLink");
        }
        if !covered(&s.composition) {
            return Err("error.draw2.propsElement");
        }
        let sub = sub_record(&self.record, &s.members, &self.bonds_of[k], &[]);
        if sub.atoms.iter().any(|a| a.chg != 0) {
            return Err("error.draw2.propsCharged");
        }
        Ok(crate::props::of(&sub))
    }

    /// The numbers of every structure, in order. `Err` when the work is above the caps; an `open` structure (an R group) has the key
    /// `error.draw2.valuesOpen`, a refusal of the engine its own key.
    pub fn measure(
        &self,
        adducts: &[String],
        engine: Measurer,
    ) -> Result<Vec<Entry>, &'static str> {
        self.measure_cached(adducts, engine, &[])
    }

    /// Like `measure`, asking the engine only for the structures `prev` (results for the same adducts) does not hold as they are now:
    /// a result is reused when its anchor, formula and isotopes are the structure's. The engine is asked once for each other one.
    pub fn measure_cached(
        &self,
        adducts: &[String],
        engine: Measurer,
        prev: &[Entry],
    ) -> Result<Vec<Entry>, &'static str> {
        self.measure_within(adducts, engine, prev, (MAX_ATOMS, MAX_STRUCTURES))
    }

    /// Like `measure_cached` with the caps `(atoms, structures)` of the caller (the live panel keeps `MAX_ATOMS` and `MAX_STRUCTURES`;
    /// the TP table, an export asked for once, has its own, `tptable::MAX_ATOMS` and `tptable::MAX_STRUCTURES`). Above them the answer
    /// is `error.draw2.valuesSize` and the engine is not called.
    pub fn measure_within(
        &self,
        adducts: &[String],
        engine: Measurer,
        prev: &[Entry],
        caps: (usize, usize),
    ) -> Result<Vec<Entry>, &'static str> {
        if self.record.atoms.len() > caps.0 || self.structures.len() > caps.1 {
            return Err("error.draw2.valuesSize");
        }
        let held: std::collections::HashMap<u32, &Entry> =
            prev.iter().map(|e| (e.anchor, e)).collect();
        Ok(self
            .structures
            .iter()
            .enumerate()
            .map(|(k, s)| {
                if let Some(e) = held.get(&s.anchor).filter(|e| e.matches(s)) {
                    return (*e).clone();
                }
                Entry {
                    anchor: s.anchor,
                    formula: s.formula.clone(),
                    iso: s.iso.clone(),
                    result: if s.open {
                        Err("error.draw2.valuesOpen")
                    } else if s.linked {
                        Err("error.draw2.valuesLink")
                    } else {
                        engine(
                            &molfile(&self.record, &s.members, &self.bonds_of[k], &[]),
                            adducts,
                        )
                    },
                }
            })
            .collect())
    }

    /// The structure of the atoms `ids` of the document as one unit (the selection): the atoms with their hydrogens as drawn, the
    /// bonds between them, and the atoms an abbreviation stands for. A bond to an atom outside is cut and closed with hydrogen (one
    /// per order of the bond, in the formula and for the engine alike). `None` for no atom of the document.
    pub fn selection(&self, doc: &Document, ids: &[u32]) -> Option<Selected> {
        let index = doc.index_map();
        let n = doc.atoms.len().min(self.record.atoms.len());
        let mut chosen: Vec<usize> = ids.iter().filter_map(|id| index.get(id).copied()).collect();
        chosen.sort_unstable();
        chosen.dedup();
        chosen.retain(|&i| i < n);
        if chosen.is_empty() {
            return None;
        }
        // the atoms an abbreviation stands for follow their labelled atom (they are the record atoms after the document's)
        let mut next: Vec<Vec<usize>> = vec![Vec::new(); self.record.atoms.len()];
        for b in &self.record.bonds {
            next[b.a as usize].push(b.b as usize);
            next[b.b as usize].push(b.a as usize);
        }
        let mut in_set = vec![false; self.record.atoms.len()];
        let mut stack: Vec<usize> = Vec::new();
        for &i in &chosen {
            in_set[i] = true;
            stack.push(i);
        }
        while let Some(i) = stack.pop() {
            for &j in &next[i] {
                if j >= n && !in_set[j] {
                    in_set[j] = true;
                    stack.push(j);
                }
            }
        }
        let members: Vec<usize> = (0..in_set.len()).filter(|&i| in_set[i]).collect();
        let bonds: Vec<usize> = (self.record.bonds.iter().enumerate())
            .filter(|(_, b)| in_set[b.a as usize] && in_set[b.b as usize])
            .map(|(k, _)| k)
            .collect();
        // the order of the bonds each chosen atom loses to atoms outside the selection
        let mut at = vec![usize::MAX; in_set.len()];
        for (k, &i) in members.iter().enumerate() {
            at[i] = k;
        }
        let mut cut = vec![0u32; members.len()];
        for (k, b) in self.record.bonds.iter().enumerate() {
            let (a, z) = (b.a as usize, b.b as usize);
            // a dative or hydrogen bond is no valence of its ends: cut, it leaves no hydrogen behind
            if doc.bonds.get(k).is_some_and(crate::doc::Bond::is_link) {
                continue;
            }
            if in_set[a] && !in_set[z] {
                cut[at[a]] += u32::from(b.o);
            } else if in_set[z] && !in_set[a] {
                cut[at[z]] += u32::from(b.o);
            }
        }
        let (formula, composition, charge, open) = count(doc, &self.record, &members, &cut);
        let linked =
            (bonds.iter()).any(|&k| doc.bonds.get(k).is_some_and(crate::doc::Bond::is_link));
        Some(Selected {
            linked,
            sub: sub_record(&self.record, &members, &bonds, &cut),
            atoms: members.len(),
            formula,
            composition,
            charge,
            open,
            molfile: molfile(&self.record, &members, &bonds, &cut),
        })
    }
}

/// A selection as one structure.
#[derive(Clone, Debug, PartialEq)]
pub struct Selected {
    pub atoms: usize,
    pub formula: String,
    pub composition: Vec<(String, u32)>,
    pub charge: i32,
    pub open: bool,
    /// As `Structure::linked`.
    pub linked: bool,
    molfile: String,
    sub: Mol,
}

impl Selected {
    /// The descriptors of the selection, or the key that says why there are none (as `Scan::props`).
    pub fn props(&self) -> Result<crate::props::Props, &'static str> {
        if self.atoms > MAX_ATOMS {
            return Err("error.draw2.valuesSize");
        }
        if self.open {
            return Err("error.draw2.valuesOpen");
        }
        if self.linked {
            return Err("error.draw2.valuesLink");
        }
        if !covered(&self.composition) {
            return Err("error.draw2.propsElement");
        }
        if self.sub.atoms.iter().any(|a| a.chg != 0) {
            return Err("error.draw2.propsCharged");
        }
        Ok(crate::props::of(&self.sub))
    }

    /// The numbers of the selection, as for a structure.
    pub fn measure(&self, adducts: &[String], engine: Measurer) -> Result<Measured, &'static str> {
        if self.atoms > MAX_ATOMS {
            return Err("error.draw2.valuesSize");
        }
        if self.open {
            return Err("error.draw2.valuesOpen");
        }
        if self.linked {
            return Err("error.draw2.valuesLink");
        }
        engine(&self.molfile, adducts)
    }
}

/// A mass or an m/z with four decimals, rounded half away from zero (`mzlab_mol::fixed`, never the float formatter).
pub fn four(v: f64) -> String {
    fixed(v, 4)
}

fn quote(s: &str) -> String {
    serde_json::Value::from(s).to_string()
}

/// The numbers of one structure as the keys `mass`, `average`, `ionMz`, `mz` and `error` of a `values/1` entry (JSON members, no braces).
fn numbers_json(r: Option<&Result<Measured, &'static str>>, adducts: &[String]) -> String {
    let none =
        |why: &str| format!(r#""mass":null,"average":null,"ionMz":null,"mz":{{}},"error":{why}"#);
    match r {
        None => none("null"),
        Some(Err(key)) => none(&quote(key)),
        Some(Ok(v)) => {
            let mz: Vec<String> = (adducts.iter().zip(&v.mz))
                .map(|(a, x)| {
                    format!(
                        "{}:{}",
                        quote(a),
                        x.map_or("null".into(), |x| quote(&four(x)))
                    )
                })
                .collect();
            format!(
                r#""mass":{},"average":{},"ionMz":{},"mz":{{{}}},"error":null"#,
                quote(&four(v.mass)),
                v.average.map_or("null".into(), |x| quote(&four(x))),
                v.ion_mz.map_or("null".into(), |x| quote(&four(x))),
                mz.join(",")
            )
        }
    }
}

fn composition_json(c: &[(String, u32)]) -> String {
    let body: Vec<String> = c
        .iter()
        .map(|(el, n)| format!("{}:{n}", quote(el)))
        .collect();
    format!("{{{}}}", body.join(","))
}

/// The entry of a structure, `{"anchor", "atoms", "formula", "charge", "composition", "mass", "average", "ionMz", "mz", "error"}`;
/// the numbers are strings with four decimals (a JSON number would be formatted by the float printer, which the module does not carry).
pub fn structure_json(
    s: &Structure,
    r: Option<&Result<Measured, &'static str>>,
    adducts: &[String],
) -> String {
    format!(
        r#"{{"anchor":{},"atoms":{},"formula":{},"charge":{},"composition":{},{}}}"#,
        s.anchor,
        s.atoms,
        quote(&s.formula),
        s.charge,
        composition_json(&s.composition),
        numbers_json(r, adducts)
    )
}

/// The entry of a selection: as `structure_json`, with `atoms` the number of record atoms and the ids asked for in `of`.
pub fn selection_json(
    s: &Selected,
    of: &[u32],
    r: Option<&Result<Measured, &'static str>>,
    adducts: &[String],
) -> String {
    let ids: Vec<String> = of.iter().map(ToString::to_string).collect();
    format!(
        r#"{{"of":[{}],"atoms":{},"formula":{},"charge":{},"composition":{},{}}}"#,
        ids.join(","),
        s.atoms,
        quote(&s.formula),
        s.charge,
        composition_json(&s.composition),
        numbers_json(r, adducts)
    )
}

/// The whole answer `values/1`: `{"format", "adducts", "engine", "error", "structures": [...], "selection": {...} | null}`. `engine` says
/// whether numbers were asked of an engine; `error` is the key that stopped all numbers (the caps), else null.
pub fn document_json(
    doc: &Document,
    adducts: &[String],
    select: Option<&[u32]>,
    engine: Option<Measurer>,
) -> String {
    let scan = scan(doc);
    let (entries, error) = match engine.map(|f| scan.measure(adducts, f)) {
        Some(Ok(e)) => (Some(e), None),
        Some(Err(k)) => (None, Some(k)),
        None => (None, None),
    };
    let rows: Vec<String> = (scan.structures.iter().enumerate())
        .map(|(k, s)| structure_json(s, entries.as_ref().map(|e| &e[k].result), adducts))
        .collect();
    let selection = match (select, engine) {
        (Some(ids), _) => scan.selection(doc, ids).map_or("null".to_string(), |s| {
            let r = engine.map(|f| s.measure(adducts, f));
            selection_json(&s, ids, r.as_ref(), adducts)
        }),
        _ => "null".to_string(),
    };
    let names: Vec<String> = adducts.iter().map(|a| quote(a)).collect();
    format!(
        r#"{{"format":"{FORMAT}","adducts":[{}],"engine":{},"error":{},"structures":[{}],"selection":{selection}}}"#,
        names.join(","),
        engine.is_some(),
        error.map_or("null".into(), quote),
        rows.join(",")
    )
}

/// A selection as the props functions take it: the ids it was asked for and its result, when there is one.
pub type SelRef<'a> = (&'a [u32], Option<&'a Result<Measured, &'static str>>);

/// The value of the key `format` of the answer of `props_json`.
pub const PROPS_FORMAT: &str = "props/1";

/// The forms the values are copied as (`props_text`): `text` (a block of `name: value` lines per structure, headed by its formula),
/// `tsv` (tab-separated rows `structure`, `name`, `value`, with a header row) and `line` (one line per structure, `; ` between values).
pub const FORMS: [&str; 3] = ["text", "tsv", "line"];

/// One structure or the selection, with every line key's entries.
struct Block {
    /// `Some` for the selection: the ids it was asked for.
    of: Option<Vec<u32>>,
    anchor: u32,
    atoms: usize,
    formula: String,
    charge: i32,
    cap: Vec<String>,
    error: Option<&'static str>,
    /// Why there is no descriptor number (an element outside `props::DESCRIPTOR_ELEMENTS`, an R group, above the caps), when it is so.
    props_error: Option<&'static str>,
    values: Vec<(&'static str, Vec<(String, String)>)>,
}

fn block<'a>(
    adducts: &[String],
    mut x: Subject<'a>,
    props: Result<crate::props::Props, &'static str>,
    r: Option<&'a Result<Measured, &'static str>>,
    mut b: Block,
) -> Block {
    match r {
        Some(Ok(m)) => x.now = Some(m),
        Some(Err(k)) => b.error = Some(k),
        None => {}
    }
    // an engine that refused the structure (an element it does not know, a size it does not take) leaves no descriptor number either
    match props {
        Ok(p) if b.error.is_none() => x.props = Some(p),
        Ok(_) => b.props_error = b.error,
        Err(k) => b.props_error = b.error.or(Some(k)),
    }
    b.values = crate::props::LINES
        .iter()
        .map(|k| (*k, entries(k, adducts, &x)))
        .collect();
    b
}

/// The blocks of the structures, and of the selection when there is one. `measured` are the results for the structures, `sel` the
/// selection with the ids it was asked for and its result; both may be absent (no engine: only the numbers of the record).
fn blocks(
    doc: &Document,
    adducts: &[String],
    measured: &[Entry],
    sel: Option<SelRef<'_>>,
) -> (Vec<Block>, Option<Block>) {
    let sc = scan(doc);
    let rows = (sc.structures.iter().enumerate())
        .map(|(k, s)| {
            let r = (measured.iter()).find(|e| e.matches(s)).map(|e| &e.result);
            let b = Block {
                of: None,
                anchor: s.anchor,
                atoms: s.atoms,
                formula: s.formula.clone(),
                charge: s.charge,
                cap: s.cap.clone(),
                error: None,
                props_error: None,
                values: Vec::new(),
            };
            let x = Subject {
                formula: &s.formula,
                now: None,
                props: None,
            };
            block(adducts, x, sc.props(k), r, b)
        })
        .collect();
    let selection = sel.and_then(|(ids, r)| {
        let s = sc.selection(doc, ids)?;
        // the choice of the molecules the selection touches, when they all have the same one (else none: every line)
        let mut caps = (sc.structures.iter())
            .filter(|t| ids.iter().any(|i| t.ids.contains(i)))
            .map(|t| &t.cap);
        let cap = caps
            .next()
            .filter(|first| caps.all(|c| c == *first))
            .cloned()
            .unwrap_or_default();
        let b = Block {
            of: Some(ids.to_vec()),
            anchor: 0,
            atoms: s.atoms,
            formula: s.formula.clone(),
            charge: s.charge,
            cap,
            error: None,
            props_error: None,
            values: Vec::new(),
        };
        let x = Subject {
            formula: &s.formula,
            now: None,
            props: None,
        };
        Some(block(adducts, x, s.props(), r, b))
    });
    (rows, selection)
}

fn block_json(b: &Block) -> String {
    let head = match &b.of {
        Some(of) => {
            let ids: Vec<String> = of.iter().map(ToString::to_string).collect();
            format!(r#""of":[{}],"atoms":{}"#, ids.join(","), b.atoms)
        }
        None => format!(r#""anchor":{},"atoms":{}"#, b.anchor, b.atoms),
    };
    let cap: Vec<String> = b.cap.iter().map(|c| quote(c)).collect();
    let values: Vec<String> = (b.values.iter())
        .map(|(k, e)| {
            let pairs: Vec<String> = (e.iter())
                .map(|(n, v)| format!("[{},{}]", quote(n), quote(v)))
                .collect();
            format!("{}:[{}]", quote(k), pairs.join(","))
        })
        .collect();
    format!(
        r#"{{{head},"formula":{},"charge":{},"cap":[{}],"error":{},"propsError":{},"values":{{{}}}}}"#,
        quote(&b.formula),
        b.charge,
        cap.join(","),
        b.error.map_or("null".into(), quote),
        b.props_error.map_or("null".into(), quote),
        values.join(",")
    )
}

/// The answer `props/1`: `{"format", "keys", "adducts", "engine", "error", "structures": [...], "selection": {...} | null}`. A structure is
/// `{"anchor", "atoms", "formula", "charge", "cap", "error", "values"}` where `cap` is the choice of lines kept under it (empty: it
/// follows `display.values`) and `values` has one key per line key of `props::LINES` whose value is the list of `[name, value]` pairs
/// (strings; empty when the number is not there). The selection has `of` (the ids asked for) in place of `anchor`.
pub fn props_json(
    doc: &Document,
    adducts: &[String],
    engine: bool,
    error: &str,
    measured: &[Entry],
    sel: Option<SelRef<'_>>,
) -> String {
    let (rows, selection) = blocks(doc, adducts, measured, sel);
    let keys: Vec<String> = crate::props::LINES.iter().map(|k| quote(k)).collect();
    let names: Vec<String> = adducts.iter().map(|a| quote(a)).collect();
    let rows: Vec<String> = rows.iter().map(block_json).collect();
    let methods: Vec<String> = (crate::props::LINES.iter())
        .filter_map(|k| {
            let m = crate::props::method(k)?;
            Some(format!(
                r#"{}:{{"id":{},"text":{}}}"#,
                quote(k),
                quote(m.0),
                quote(m.1)
            ))
        })
        .collect();
    format!(
        r#"{{"format":"{PROPS_FORMAT}","keys":[{}],"adducts":[{}],"methods":{{{}}},"engine":{engine},"error":{},"structures":[{}],"selection":{}}}"#,
        keys.join(","),
        names.join(","),
        methods.join(","),
        if error.is_empty() {
            "null".into()
        } else {
            quote(error)
        },
        rows.join(","),
        selection.as_ref().map_or("null".into(), block_json)
    )
}

/// The values copied as text in a form of `FORMS`: of the selection when `sel` is given, else of every structure. `keys` is the list of
/// lines to copy; empty means the choice kept under each molecule, or every line for a molecule with none. The formula heads each block
/// (the key `formula` adds nothing more). `None` for a form that is none, or for nothing to copy.
pub fn props_text(
    form: &str,
    doc: &Document,
    adducts: &[String],
    keys: &[String],
    measured: &[Entry],
    sel: Option<SelRef<'_>>,
) -> Option<String> {
    if !FORMS.contains(&form) {
        return None;
    }
    let (rows, selection) = blocks(doc, adducts, measured, sel);
    let used: Vec<Block> = match (sel, selection) {
        (Some(_), s) => s.into_iter().collect(),
        (None, _) => rows,
    };
    if used.is_empty() {
        return None;
    }
    // (name, value, method id); a block with no number for a reason ends with the note, the key of that reason
    let pairs = |b: &Block| -> Vec<(String, String, String)> {
        let wanted: Vec<&str> = if !keys.is_empty() {
            keys.iter().map(String::as_str).collect()
        } else if !b.cap.is_empty() {
            b.cap.iter().map(String::as_str).collect()
        } else {
            crate::props::LINES.to_vec()
        };
        let mut rows: Vec<(String, String, String)> = (wanted.iter())
            .filter(|k| **k != "formula")
            .filter_map(|k| b.values.iter().find(|v| v.0 == *k))
            .flat_map(|v| {
                let id = crate::props::method(v.0).map_or("", |m| m.0);
                v.1.iter()
                    .map(move |(n, x)| (n.clone(), x.clone(), id.to_string()))
            })
            .collect();
        if let Some(why) = b.error.or(b.props_error) {
            rows.push(("note".into(), why.into(), String::new()));
        }
        rows
    };
    let mut out = String::new();
    match form {
        "tsv" => {
            out += "structure\tname\tvalue\tmethod\n";
            for b in &used {
                let rows = pairs(b);
                if rows.is_empty() {
                    out += &format!("{}\t\t\t\n", b.formula);
                }
                for (n, v, m) in rows {
                    out += &format!("{}\t{n}\t{v}\t{m}\n", b.formula);
                }
            }
        }
        "line" => {
            for b in &used {
                let mut parts = vec![b.formula.clone()];
                // the one-line form stays short: no method (the table and the text forms have it)
                parts.extend(pairs(b).into_iter().map(|(n, v, _)| format!("{n} {v}")));
                out += &(parts.join("; ") + "\n");
            }
        }
        _ => {
            for (k, b) in used.iter().enumerate() {
                if k > 0 {
                    out += "\n";
                }
                out += &format!("{}\n", b.formula);
                for (n, v, m) in pairs(b) {
                    if m.is_empty() {
                        out += &format!("{n}: {v}\n");
                    } else {
                        out += &format!("{n}: {v} [{m}]\n");
                    }
                }
            }
        }
    }
    Some(out)
}

/// `props_json` or `props_text` asked of an engine (the replay request field `props`, with no editor): the numbers of every structure
/// and of the selection are measured here; without an engine only the numbers of the record are there (`engine: false`).
pub fn document_props(
    doc: &Document,
    adducts: &[String],
    select: Option<&[u32]>,
    engine: Option<Measurer>,
    form: Option<(&str, &[String])>,
) -> Result<String, &'static str> {
    // an id the drawing lacks and a line key that is none are refused, never answered with an empty selection or a dropped key
    if select.is_some_and(|ids| ids.iter().any(|i| doc.atom(*i).is_none())) {
        return Err("error.draw2.noAtom");
    }
    if form.is_some_and(|(_, keys)| {
        keys.iter()
            .any(|k| !crate::props::LINES.contains(&k.as_str()))
    }) {
        return Err("error.draw2.propsLines");
    }
    let sc = scan(doc);
    let (entries, error) = match engine.map(|f| sc.measure(adducts, f)) {
        Some(Ok(e)) => (e, ""),
        Some(Err(k)) => (Vec::new(), k),
        None => (Vec::new(), ""),
    };
    let sel_r = match (select, engine) {
        (Some(ids), Some(f)) => sc.selection(doc, ids).map(|s| s.measure(adducts, f)),
        _ => None,
    };
    let sel = select.map(|ids| (ids, sel_r.as_ref()));
    match form {
        Some((f, keys)) => {
            props_text(f, doc, adducts, keys, &entries, sel).ok_or("error.draw2.propsCopy")
        }
        None => Ok(props_json(
            doc,
            adducts,
            engine.is_some(),
            error,
            &entries,
            sel,
        )),
    }
}

/// The text drawn under one structure.
#[derive(Clone, Debug, PartialEq)]
pub struct Caption {
    /// The middle of the structure and its lowest atom, in document units.
    pub x: f64,
    pub y: f64,
    pub lines: Vec<String>,
    /// Whether the level of an annotation is drawn first, under the structure.
    pub below: bool,
}

/// The lines of the caption of a structure in a mode, from what is known of it.
fn lines(s: &Structure, mode: Mode, adducts: &[String], now: Option<&Measured>) -> Vec<String> {
    match (mode, now) {
        (Mode::Off, _) => Vec::new(),
        (Mode::Mass, Some(v)) => vec![format!("{}  {}", s.formula, four(v.mass))],
        (Mode::Mz, Some(v)) => {
            let mut out: Vec<String> = Vec::new();
            if let Some(x) = v.ion_mz {
                out.push(format!("m/z {}", four(x)));
            }
            for (name, x) in adducts.iter().zip(&v.mz) {
                if let Some(x) = x {
                    out.push(format!("{name} {}", four(*x)));
                }
            }
            if out.is_empty() {
                out.push(s.formula.clone());
            }
            out
        }
        // the formula, and what stands in for a number that is not there
        _ => vec![s.formula.clone()],
    }
}

/// What one structure (or a selection) is known to be: the pieces the lines of a molecule are written from.
pub struct Subject<'a> {
    pub formula: &'a str,
    pub now: Option<&'a Measured>,
    pub props: Option<crate::props::Props>,
}

/// The (name, value) pairs of one line key of `props::LINES`, as the copy and the caption write them. Nothing for a key this version
/// does not know, and for a number that is not there (no engine, an R group): the formula is always there.
pub fn entries(key: &str, adducts: &[String], x: &Subject) -> Vec<(String, String)> {
    let one = |n: &str, v: String| vec![(n.to_string(), v)];
    match key {
        "formula" => one("Formula", x.formula.to_string()),
        "mw" => (x.now.and_then(|m| m.average))
            .map(|v| one("MW", four(v)))
            .unwrap_or_default(),
        "exact" => (x.now)
            .map(|m| one("Exact mass", four(m.mass)))
            .unwrap_or_default(),
        "ea" => (x.now)
            .map(|m| {
                (m.pct.iter())
                    .map(|(el, v)| (format!("%{el}"), crate::props::two(*v)))
                    .collect()
            })
            .unwrap_or_default(),
        "mz" => {
            let mut out = Vec::new();
            if let Some(m) = x.now {
                if let Some(v) = m.ion_mz {
                    out.push(("m/z".to_string(), four(v)));
                }
                for (name, v) in adducts.iter().zip(&m.mz) {
                    if let Some(v) = v {
                        out.push((name.clone(), four(*v)));
                    }
                }
            }
            out
        }
        _ => match x.props.as_ref().and_then(|p| crate::props::line(key, p)) {
            Some((n, v)) => one(n, v),
            None => Vec::new(),
        },
    }
}

/// The caption lines of a list of keys: one line per entry, except the formula (alone) and the elemental analysis (one line).
pub fn chosen_lines(keys: &[String], adducts: &[String], x: &Subject) -> Vec<String> {
    let mut out = Vec::new();
    for k in keys {
        let e = entries(k, adducts, x);
        match k.as_str() {
            "formula" => out.extend(e.into_iter().map(|p| p.1)),
            "ea" if !e.is_empty() => out.push(
                (e.iter())
                    .map(|(n, v)| format!("{n} {v}"))
                    .collect::<Vec<_>>()
                    .join("  "),
            ),
            _ => out.extend(e.into_iter().map(|(n, v)| format!("{n} {v}"))),
        }
    }
    out
}

/// The captions of a drawing: a molecule with a choice of lines (`props`) writes them; the others follow the mode (none for `Off`);
/// `measured` are the results kept by the editor. With `titles` (`display.titles`) the title of a structure (`atom.title`) is its
/// first line, its legend.
pub fn captions(
    doc: &Document,
    mode: Mode,
    adducts: &[String],
    measured: &[Entry],
    titles: bool,
) -> Vec<Caption> {
    let titled = titles && doc.atoms.iter().any(|a| !a.title().is_empty());
    if doc.atoms.is_empty()
        || (mode == Mode::Off && !titled && doc.atoms.iter().all(|a| a.cap().is_empty()))
    {
        return Vec::new();
    }
    // the title of a structure: the first of its atoms (in pool order) that has one
    let title_of = |s: &Structure| -> String {
        if !titled {
            return String::new();
        }
        (s.ids.iter())
            .filter_map(|id| doc.atom(*id))
            .map(|a| a.title())
            .find(|t| !t.is_empty())
            .unwrap_or_default()
    };
    let sc = scan(doc);
    (sc.structures.iter().enumerate())
        .filter(|(_, s)| !s.ids.is_empty())
        .filter(|(_, s)| mode != Mode::Off || !s.cap.is_empty() || !title_of(s).is_empty())
        .map(|(k, s)| {
            let held = (measured.iter()).find(|e| e.matches(s));
            let now = held.and_then(|e| e.result.as_ref().ok());
            let lines = if s.cap.is_empty() {
                lines(s, mode, adducts, now)
            } else {
                let props = s
                    .cap
                    .iter()
                    .any(|c| !crate::props::needs_engine(c) && c != "formula")
                    // no descriptor number where the engine refused the structure or the element is not covered
                    .then(|| sc.props(k).ok())
                    .flatten()
                    .filter(|_| !held.is_some_and(|e| e.result.is_err()));
                chosen_lines(
                    &s.cap,
                    adducts,
                    &Subject {
                        formula: &s.formula,
                        now,
                        props,
                    },
                )
            };
            let title = title_of(s);
            let lines = if title.is_empty() {
                lines
            } else {
                std::iter::once(title).chain(lines).collect()
            };
            Caption {
                x: (s.bbox[0] + s.bbox[2]) / 2.0,
                y: s.bbox[1],
                lines,
                below: s.annotated,
            }
        })
        .collect()
}

/// Where the lines of a caption stand, in pixels below the lowest atom of the structure and the height they take, for a font size
/// `f` (the painter and the page size of an export use the same numbers).
pub fn caption_first(scale: f64, f: f64, below: bool) -> f64 {
    scale * 0.3 + f * 0.9 + if below { f } else { 0.0 }
}

pub fn caption_step(f: f64) -> f64 {
    f * 1.1
}

/// The font of a caption, from the font of the drawing.
pub fn caption_font(font: f64) -> f64 {
    font * 0.8
}

/// The monoisotopic mass of each element a step delta can name (`C H N O S Cl Br`), got from the engine alone, so the core keeps no
/// mass table: the engine is asked for seven one-structure molfiles with implicit hydrogens (methane, ethane, water, ammonia,
/// hydrogen sulfide, hydrogen chloride, hydrogen bromide) and the masses are solved from them (H2 = 2 CH4 - C2H6, then each element
/// is its hydride minus its hydrogens). `None` when there is no engine or it refuses one of them.
pub fn element_masses(engine: Measurer) -> Option<std::collections::BTreeMap<String, f64>> {
    let mass = |atoms: &[&str]| -> Option<f64> {
        let mut m = format!("\n  {}\n\n", crate::NAME);
        m.push_str(&format!(
            "{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
            atoms.len(),
            atoms.len().saturating_sub(1)
        ));
        for (i, el) in atoms.iter().enumerate() {
            m.push_str(&format!(
                "{:>10.4}{:>10.4}{:>10.4} {:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n",
                i as f64 * 1.5,
                0.0,
                0.0,
                el
            ));
        }
        if atoms.len() == 2 {
            m.push_str("  1  2  1  0\n");
        }
        m.push_str("M  END\n");
        engine(&m, &[]).ok().map(|r| r.mass)
    };
    let (ch4, c2h6) = (mass(&["C"])?, mass(&["C", "C"])?);
    let h = (2.0 * ch4 - c2h6) / 2.0;
    let mut out = std::collections::BTreeMap::new();
    out.insert("H".to_string(), h);
    out.insert("C".to_string(), ch4 - 4.0 * h);
    for (el, hydride, n) in [
        ("O", "O", 2.0),
        ("N", "N", 3.0),
        ("S", "S", 2.0),
        ("Cl", "Cl", 1.0),
        ("Br", "Br", 1.0),
    ] {
        out.insert(el.to_string(), mass(&[hydride])? - n * h);
    }
    Some(out)
}

/// The mass shift of a delta in the canonical spelling (`+O`, `-CH2`, `+H-Cl`), from the element masses of `element_masses`.
pub fn delta_mass(delta: &str, masses: &std::collections::BTreeMap<String, f64>) -> Option<f64> {
    let counts = mzlab_mol::delta::parse(delta).ok()?;
    let mut sum = 0.0;
    for (el, n) in counts {
        sum += *masses.get(&el)? * n as f64;
    }
    Some(sum)
}
