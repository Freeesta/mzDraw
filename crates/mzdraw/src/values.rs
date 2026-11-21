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
    /// Whether the painter writes a name or a level under the structure (an annotation, or the data field `TP_ID`): the caption goes below it.
    pub annotated: bool,
    /// Its box in the document, `[x0, y0, x1, y1]`, y up.
    pub bbox: [f64; 4],
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
        .map(|members| {
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
            for &i in &members {
                if let Some(a) = doc.atoms.get(i) {
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
                annotated,
                bbox,
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
        if self.record.atoms.len() > MAX_ATOMS || self.structures.len() > MAX_STRUCTURES {
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
        for b in &self.record.bonds {
            let (a, z) = (b.a as usize, b.b as usize);
            if in_set[a] && !in_set[z] {
                cut[at[a]] += u32::from(b.o);
            } else if in_set[z] && !in_set[a] {
                cut[at[z]] += u32::from(b.o);
            }
        }
        let (formula, composition, charge, open) = count(doc, &self.record, &members, &cut);
        Some(Selected {
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
    molfile: String,
}

impl Selected {
    /// The numbers of the selection, as for a structure.
    pub fn measure(&self, adducts: &[String], engine: Measurer) -> Result<Measured, &'static str> {
        if self.atoms > MAX_ATOMS {
            return Err("error.draw2.valuesSize");
        }
        if self.open {
            return Err("error.draw2.valuesOpen");
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

/// The captions of a drawing in a mode (none for `Off` or an empty drawing); `measured` are the results kept by the editor.
pub fn captions(
    doc: &Document,
    mode: Mode,
    adducts: &[String],
    measured: &[Entry],
) -> Vec<Caption> {
    if mode == Mode::Off || doc.atoms.is_empty() {
        return Vec::new();
    }
    (scan(doc).structures.iter())
        .filter(|s| !s.ids.is_empty())
        .map(|s| {
            let now = (measured.iter())
                .find(|e| e.matches(s))
                .and_then(|e| e.result.as_ref().ok());
            Caption {
                x: (s.bbox[0] + s.bbox[2]) / 2.0,
                y: s.bbox[1],
                lines: lines(s, mode, adducts, now),
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
