//! The TP table (tp-09): the drawing of a transformation-product pathway as the table a paper and its SI carry, one source and two
//! outputs, `save("tptable")` (`tptable/1`, JSON) and `save("tptable-csv")` (the same rows as CSV). The table is a view of the
//! drawing: nothing is typed twice, so scheme and table cannot disagree.
//!
//! **Columns**, in this order (`COLUMNS`): `tp_id`, `parent_id`, `generation`, `smiles`, `formula`, `monoisotopic_mass`, `adduct`,
//! `mz_calc`, `mz_obs`, `ppm`, `rt`, `form_diff`, `mass_diff`, `level`, `identifier`. The names `form_diff` and `mass_diff` and the
//! one row per parent-TP pair follow the TP lists of NORMAN-SLE (`Form_Diff`, `Mass_Diff`, `ParentOrTP`: study `docs/study/tp-draw.md`
//! §1 T4); the CSV is called NORMAN-style, never NORMAN-ready: no NORMAN validator was run.
//!
//! **Rows.** One row per structure; a structure with several parents (a convergent TP) has one row per parent, the parent pairs in
//! the order of the table. A structure with no parent has one row with `parent_id`, `form_diff` and `mass_diff` empty. The rows are
//! in the order `tp:number` numbers the products in (`pathway.number`: by layer, then position; `mz`; `rt`), roots included; the
//! structures no anchored arrow touches come after them in the order of their first atom; a drawing with no anchored arrow has no
//! pathway and its rows are in the order of the first atom (`generation` is then empty: no layer is defined).
//!
//! **Where each cell comes from.** `tp_id`, `mz_obs`, `rt`, `adduct` are the reserved data of the structure as typed (`tp_list::FIELDS`
//! `TP_ID`, `MZ`, `RT`, `ADDUCT`); `level` and `identifier` the current entry of its annotation. `parent_id` is the `TP_ID` of the
//! structure an anchored arrow leaves, `generation` the layer of tp-05 (the longest path from a root). `smiles` is the isomeric SMILES
//! `save("smiles")` writes for that structure alone; `formula` the Hill formula (no charge: the SMILES has it). `form_diff` is the
//! formula delta `TP - parent` of `pathway:label` (tp-07), in the spelling of `mzlab_mol::delta::print`; it is empty for equal
//! formulas (an isomer; `mass_diff` is then `0.0000`) and where there is no honest delta (a free-text label, or equal atoms and
//! another net charge: a delta has no charges). **No mass is computed or typed here**: `monoisotopic_mass` and `mz_calc` come from
//! the engine `sel` of mzLab through `values::Measurer` (arch-web-17), which only the bridge crate holds. Without it those cells,
//! `ppm` and `mass_diff` are empty and every structure is listed in the note `draw2.export.tpEngine`.
//!
//! **Numbers and rounding.** Masses and m/z are written with 4 decimals by the formatter of Rust (`{:.4}`: the exact decimal value of
//! the binary number, ties to even); `mass_diff` is the difference of the two UNROUNDED masses, rounded once and signed (`+15.9949`,
//! `-14.0157`, `0.0000`), so it can differ by 0.0001 from the difference of the two printed masses (`-CH2` is `-14.0157`; the printed
//! masses of acid and homologue differ by `14.0156`). `ppm = (mz_obs - mz_calc) / mz_calc * 10^6` from the typed `mz_obs` and the
//! PRINTED `mz_calc` (the number the reader sees: the table can be recomputed), 1 decimal (same formatter, so ties to even), `-0.0`
//! written `0.0`; against the unrounded m/z it would differ by up to 0.4 ppm. `mz_calc` is that of
//! the typed adduct and only for the seven adducts of `values::ADDUCTS`; another adduct (`[M+H-NH3]+`, `[M+2H]2+`) or a charged
//! structure leaves `mz_calc` and `ppm` empty and is listed (`draw2.export.tpAdduct`, `draw2.export.tpCharged`).
//!
//! **Nothing is dropped in silence.** Every cell left empty for a reason that is not "the drawing has no such thing" is a keyed note
//! (`Table::notes`): the output of `save` is the table alone, the notes are in `lost` of the `draw-result/1` document and on the page
//! after the save. In the JSON the notes are also a key of the document.
//!
//! **CSV.** RFC 4180: a field with a comma, a quote, a line break or a space at either end is quoted, `"` is doubled, lines end in
//! LF, the header is line 1. A text cell (`tp_id`, `parent_id`, `adduct`, `level`, `identifier`) that starts with `=`, `+`, `-` or
//! `@` gets a leading `'` so that a spreadsheet does not read it as a formula (`draw2.export.tpGuard` counts them); the cells this
//! module writes itself (`form_diff`, `mass_diff`, numbers) are not changed: **a spreadsheet may still read `+O` or `-CH2` as a
//! formula**, so the columns are best imported as text.
//!
//! **Limits, before any work.** More than `MAX_ATOMS` atoms or `MAX_STRUCTURES` structures is `error.draw2.tpTableSize`; the
//! pathway layout refuses above 1000 anchored arrows (`error.draw2.schemeSize`) and a cycle (`error.draw2.schemeCycle`).
//!
//! **Reading it back.** `tp_list` reads a TP table as a list (`tp_id` the name, `parent_id` the parent, `mz_obs` the m/z; rows with
//! one `tp_id` are one product with several parents); the derived columns are not read.

use crate::doc::{Atom, Document, ObjKind};
use crate::settings::Style;
use crate::values::{self, Measured, Measurer};
use std::collections::{BTreeSet, HashMap};

pub const FORMAT: &str = "tptable/1";

/// The columns of the table, in the order they are written.
pub const COLUMNS: [&str; 15] = [
    "tp_id",
    "parent_id",
    "generation",
    "smiles",
    "formula",
    "monoisotopic_mass",
    "adduct",
    "mz_calc",
    "mz_obs",
    "ppm",
    "rt",
    "form_diff",
    "mass_diff",
    "level",
    "identifier",
];

/// The columns that are computed from the drawing and never read back by the importer.
pub const DERIVED: [&str; 7] = [
    "generation",
    "formula",
    "monoisotopic_mass",
    "mz_calc",
    "ppm",
    "form_diff",
    "mass_diff",
];

/// The largest drawing a table is written for, counted before any work (the cost is one engine call per structure).
pub const MAX_ATOMS: usize = 15_000;
pub const MAX_STRUCTURES: usize = 1_000;

const KEY_SIZE: &str = "error.draw2.tpTableSize";

/// The notes (i18n keys; the text of each is on the page in `web/lang/mzdraw.js`).
pub const NOTE_KEYS: [&str; 9] = [
    "draw2.export.tpEngine",
    "draw2.export.tpMass",
    "draw2.export.tpAdduct",
    "draw2.export.tpCharged",
    "draw2.export.tpSmiles",
    "draw2.export.tpParentName",
    "draw2.export.tpDelta",
    "draw2.export.tpNumber",
    "draw2.export.tpGuard",
];

/// One thing the table could not fill: its key, the structure (`tp_id`, or `#` and its first atom when it has none) and, where the
/// engine or the writer refused, its key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub key: &'static str,
    pub item: String,
    pub detail: &'static str,
}

/// The table: its rows (15 cells each, in `COLUMNS` order, empty for no value) and the notes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    pub rows: Vec<Vec<String>>,
    pub notes: Vec<Note>,
    /// Whether the engine of the masses was there.
    pub engine: bool,
}

impl Table {
    /// The notes as (key, count), one entry per key in the order the keys first occur.
    pub fn counts(&self) -> Vec<(&'static str, usize)> {
        let mut out: Vec<(&'static str, usize)> = Vec::new();
        for n in &self.notes {
            match out.iter_mut().find(|c| c.0 == n.key) {
                Some(c) => c.1 += 1,
                None => out.push((n.key, 1)),
            }
        }
        out
    }
}

/// The 4-decimal text of a number and its value in ten-thousandths (the rounding of `{:.4}`: ties to even on the exact value).
fn units(x: f64) -> i64 {
    let t = format!("{:.4}", x.abs());
    let n: i64 = t.replace('.', "").parse().unwrap_or(0);
    if x < 0.0 {
        -n
    } else {
        n
    }
}

/// `n` ten-thousandths as a decimal, `sign` writing `+` before a positive one.
fn show(n: i64, sign: bool) -> String {
    let (a, i) = (n.unsigned_abs(), n.unsigned_abs() / 10_000);
    let s = format!("{i}.{:04}", a % 10_000);
    match n {
        0 => s,
        n if n < 0 => format!("-{s}"),
        _ if sign => format!("+{s}"),
        _ => s,
    }
}

/// The ppm error of an observed m/z against the calculated one (in ten-thousandths), 1 decimal; `None` for a calculated m/z of 0.
fn ppm(obs: f64, calc: i64) -> Option<String> {
    let calc = calc as f64 / 10_000.0;
    if calc <= 0.0 || !obs.is_finite() {
        return None;
    }
    let t = format!("{:.1}", (obs - calc) / calc * 1e6);
    Some(if t == "-0.0" { "0.0".to_string() } else { t })
}

/// The first value of the data field `name` among the atoms (ascending ids), compared without case.
fn field(atoms: &[&Atom], name: &str) -> String {
    for a in atoms {
        for (k, v) in a.data() {
            if k.trim().eq_ignore_ascii_case(name) {
                return v.trim().to_string();
            }
        }
    }
    String::new()
}

fn hill(composition: &[(String, u32)]) -> String {
    let mut out = String::new();
    for (el, n) in composition {
        out.push_str(el);
        if *n > 1 {
            out.push_str(&n.to_string());
        }
    }
    out
}

/// Builds the table of the drawing. `engine` gives the masses (the bridge passes `glue::measure`); without it the mass cells are
/// empty and the notes say so. The error is an i18n key.
pub fn build(
    doc: &Document,
    style: &Style,
    engine: Option<Measurer>,
) -> Result<Table, &'static str> {
    if doc.atoms.len() > MAX_ATOMS {
        return Err(KEY_SIZE);
    }
    let scan = values::scan(doc);
    let n = scan.structures.len();
    if n > MAX_STRUCTURES {
        return Err(KEY_SIZE);
    }
    let mut table = Table {
        engine: engine.is_some(),
        ..Table::default()
    };
    let owner: HashMap<u32, usize> = (scan.structures.iter().enumerate())
        .flat_map(|(k, s)| s.ids.iter().map(move |&a| (a, k)))
        .collect();
    // the order and the layers of the pathway; the structures no arrow touches follow, in the order of their first atom
    let order = crate::tp_list::table_order(doc, style).map_err(|e| e.0)?;
    let mut seq: Vec<(usize, Option<usize>)> = Vec::with_capacity(n);
    let mut seen = vec![false; n];
    for (atoms, layer) in order.iter().flatten() {
        if let Some(&k) = atoms.first().and_then(|a| owner.get(a))
            && !seen[k]
        {
            seen[k] = true;
            seq.push((k, Some(*layer)));
        }
    }
    seq.extend((0..n).filter(|&k| !seen[k]).map(|k| (k, None)));
    let pos: HashMap<usize, usize> = seq.iter().enumerate().map(|(i, s)| (s.0, i)).collect();
    // the parents of each structure: the structures an anchored arrow leaves
    let mut parents: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for o in &doc.objects {
        if o.t == ObjKind::Arrow
            && o.from != 0
            && o.to != 0
            && let (Some(&p), Some(&c)) = (owner.get(&o.from), owner.get(&o.to))
            && p != c
        {
            parents[c].insert(p);
        }
    }
    let ix: HashMap<u32, &Atom> = doc.atoms.iter().map(|a| (a.id, a)).collect();
    struct Info {
        id: String,
        mz: String,
        rt: String,
        adduct: String,
        level: String,
        identifier: String,
        item: String,
    }
    let infos: Vec<Info> = scan
        .structures
        .iter()
        .map(|s| {
            let mut ids = s.ids.clone();
            ids.sort_unstable();
            let atoms: Vec<&Atom> = ids.iter().filter_map(|i| ix.get(i).copied()).collect();
            let all: Vec<&Atom> = s.ids.iter().filter_map(|i| ix.get(i).copied()).collect();
            let ann = crate::ann::join(
                all.iter()
                    .map(|a| a.ann())
                    .collect::<Vec<_>>()
                    .iter()
                    .map(String::as_str),
            );
            let cur = crate::ann::current(&ann);
            let id = field(&atoms, "TP_ID");
            Info {
                item: if id.is_empty() {
                    format!("#{}", s.anchor)
                } else {
                    id.clone()
                },
                id,
                mz: field(&atoms, "MZ"),
                rt: field(&atoms, "RT"),
                adduct: field(&atoms, "ADDUCT"),
                level: cur.as_ref().map_or(String::new(), |c| c.level.clone()),
                identifier: cur.map_or(String::new(), |c| c.identifier),
            }
        })
        .collect();
    // the masses: one engine run for the adducts that rows ask for and the engine knows (the seven of `values::ADDUCTS`)
    let mut asked: Vec<String> = Vec::new();
    for i in &infos {
        if values::ADDUCTS.contains(&i.adduct.as_str()) && !asked.contains(&i.adduct) {
            asked.push(i.adduct.clone());
        }
    }
    let measured: Option<Vec<Result<Measured, &'static str>>> = match engine {
        Some(e) => Some(
            scan.measure_within(&asked, e, &[], (MAX_ATOMS, MAX_STRUCTURES))?
                .into_iter()
                .map(|e| e.result)
                .collect(),
        ),
        None => None,
    };
    let note = |t: &mut Table, key: &'static str, k: usize, detail: &'static str| {
        t.notes.push(Note {
            key,
            item: infos[k].item.clone(),
            detail,
        });
    };
    // the formula deltas, counted as `pathway:label` counts them
    let anchors: Vec<u32> = scan.structures.iter().map(|s| s.anchor).collect();
    let formulas = crate::steplabel::Formulas::new(doc, &anchors);
    // the cells of each structure that do not depend on the parent: smiles, formula, masses, m/z, ppm
    let mut smiles: Vec<String> = Vec::with_capacity(n);
    let mut mass: Vec<Option<f64>> = vec![None; n];
    let mut mz: Vec<Option<i64>> = vec![None; n];
    // the atoms and bonds of each structure, grouped in one pass (`Document::part` scans the whole drawing for each structure)
    let mut bonds_of: Vec<Vec<&crate::doc::Bond>> = vec![Vec::new(); n];
    for b in &doc.bonds {
        if let Some(&k) = owner.get(&b.a) {
            bonds_of[k].push(b);
        }
    }
    for (k, s) in scan.structures.iter().enumerate() {
        let part = Document {
            atoms: s
                .ids
                .iter()
                .filter_map(|i| ix.get(i))
                .map(|a| (*a).clone())
                .collect(),
            bonds: bonds_of[k].iter().map(|b| (*b).clone()).collect(),
            next: doc.next,
            ..Document::default()
        };
        match part.to_smiles() {
            Ok(t) => smiles.push(t),
            Err(key) => {
                smiles.push(String::new());
                note(&mut table, "draw2.export.tpSmiles", k, key);
            }
        }
        match &measured {
            None => note(&mut table, "draw2.export.tpEngine", k, ""),
            Some(m) => match &m[k] {
                Err(key) => note(&mut table, "draw2.export.tpMass", k, key),
                Ok(r) => {
                    mass[k] = Some(r.mass);
                    let a = &infos[k].adduct;
                    if !a.is_empty() {
                        match asked.iter().position(|x| x == a) {
                            None => note(&mut table, "draw2.export.tpAdduct", k, ""),
                            Some(j) => match r.mz.get(j).copied().flatten() {
                                Some(v) => mz[k] = Some(units(v)),
                                None => note(&mut table, "draw2.export.tpCharged", k, ""),
                            },
                        }
                    }
                }
            },
        }
    }
    let ppm_of = |k: usize| -> Option<String> {
        let (calc, obs) = (mz[k]?, infos[k].mz.parse::<f64>().ok());
        obs.and_then(|o| ppm(o, calc))
    };
    for &(k, layer) in &seq {
        let (s, i) = (&scan.structures[k], &infos[k]);
        if !i.mz.is_empty() && mz[k].is_some() && ppm_of(k).is_none() {
            note(&mut table, "draw2.export.tpNumber", k, "");
        }
        let mut base = vec![String::new(); COLUMNS.len()];
        base[0] = i.id.clone();
        base[2] = layer.map_or(String::new(), |l| l.to_string());
        base[3] = smiles[k].clone();
        base[4] = hill(&s.composition);
        base[5] = mass[k].map_or(String::new(), |m| show(units(m), false));
        base[6] = i.adduct.clone();
        base[7] = mz[k].map_or(String::new(), |m| show(m, false));
        base[8] = i.mz.clone();
        base[9] = ppm_of(k).unwrap_or_default();
        base[10] = i.rt.clone();
        base[13] = i.level.clone();
        base[14] = i.identifier.clone();
        if parents[k].is_empty() {
            table.rows.push(base);
            continue;
        }
        let mut ps: Vec<usize> = parents[k].iter().copied().collect();
        ps.sort_by_key(|p| pos[p]);
        for p in ps {
            let mut row = base.clone();
            if infos[p].id.is_empty() {
                note(&mut table, "draw2.export.tpParentName", k, "");
            }
            row[1] = infos[p].id.clone();
            let (from, to) = (
                formulas
                    .root_of(scan.structures[p].anchor)
                    .and_then(|r| formulas.acc.get(&r)),
                formulas
                    .root_of(s.anchor)
                    .and_then(|r| formulas.acc.get(&r)),
            );
            match from
                .zip(to)
                .and_then(|(a, b)| crate::steplabel::pair_delta(a, b))
            {
                Some(d) => row[11] = d,
                None => note(&mut table, "draw2.export.tpDelta", k, ""),
            }
            if let Some((a, b)) = mass[p].zip(mass[k]) {
                row[12] = show(units(b - a), true);
            }
            table.rows.push(row);
        }
    }
    Ok(table)
}

/// A string as a JSON text.
fn q(s: &str) -> String {
    serde_json::Value::from(s).to_string()
}

/// `tptable/1`: `{"format","columns":[...],"engine":BOOL,"rows":[{"tp_id",...}],"notes":[{"key","count","items":[...]}]}`; the keys of a
/// row are the columns in order, every value is a string (empty for no value; numbers keep the 4 or 1 decimals of the table).
pub fn json(t: &Table) -> String {
    let cols: Vec<String> = COLUMNS.iter().map(|c| q(c)).collect();
    let rows: Vec<String> = t
        .rows
        .iter()
        .map(|r| {
            let cells: Vec<String> = (COLUMNS.iter().zip(r))
                .map(|(c, v)| format!("{}:{}", q(c), q(v)))
                .collect();
            format!("{{{}}}", cells.join(","))
        })
        .collect();
    let notes: Vec<String> = (t.counts().iter())
        .map(|(key, n)| {
            let items: Vec<String> = (t.notes.iter())
                .filter(|x| x.key == *key)
                .map(|x| q(&x.item))
                .collect();
            format!(
                "{{\"key\":{},\"count\":{n},\"items\":[{}]}}",
                q(key),
                items.join(",")
            )
        })
        .collect();
    format!(
        "{{\"format\":{},\"columns\":[{}],\"engine\":{},\"rows\":[{}],\"notes\":[{}]}}",
        q(FORMAT),
        cols.join(","),
        t.engine,
        rows.join(","),
        notes.join(",")
    )
}

/// A CSV field (RFC 4180).
fn field_csv(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) || s.starts_with(' ') || s.ends_with(' ') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The columns of free text, whose cells a spreadsheet must not read as a formula.
const TEXT: [usize; 5] = [0, 1, 6, 13, 14];

/// The table as CSV: the header, then a line per row, each ending in LF. The second value is how many text cells got the guard `'`
/// (`draw2.export.tpGuard`).
pub fn csv_counted(t: &Table) -> (String, usize) {
    let mut out = COLUMNS.join(",");
    out.push('\n');
    let mut guarded = 0;
    for r in &t.rows {
        let cells: Vec<String> = r
            .iter()
            .enumerate()
            .map(|(c, v)| {
                if TEXT.contains(&c) && v.starts_with(['=', '+', '-', '@']) {
                    guarded += 1;
                    field_csv(&format!("'{v}"))
                } else {
                    field_csv(v)
                }
            })
            .collect();
        out.push_str(&cells.join(","));
        out.push('\n');
    }
    (out, guarded)
}

/// The table as CSV.
pub fn csv(t: &Table) -> String {
    csv_counted(t).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_thousandths_round_to_even_on_the_exact_value() {
        // 0.125 and 0.375 are exact in binary: the tie goes to the even digit; 0.00005 is not exact (it is above the tie)
        assert_eq!(format!("{:.2}", 0.125), "0.12");
        assert_eq!(format!("{:.2}", 0.375), "0.38");
        assert_eq!(units(15.994_915), 159_949);
        assert_eq!(units(-14.015_65), -140_157);
        assert_eq!(show(159_949, true), "+15.9949");
        assert_eq!(show(-140_157, true), "-14.0157");
        assert_eq!(show(0, true), "0.0000");
        assert_eq!(show(1_350_168, false), "135.0168");
        assert_eq!(show(-5, true), "-0.0005");
    }

    #[test]
    fn ppm_is_one_decimal_and_never_minus_zero() {
        // (135.1169 - 135.1168) / 135.1168 * 1e6 = 0.74
        assert_eq!(ppm(135.1169, 1_351_168).as_deref(), Some("0.7"));
        assert_eq!(ppm(135.1168, 1_351_168).as_deref(), Some("0.0"));
        assert_eq!(ppm(135.116_799_9, 1_351_168).as_deref(), Some("0.0"));
        assert_eq!(ppm(135.1167, 1_351_168).as_deref(), Some("-0.7"));
        assert_eq!(ppm(1.0, 0), None);
    }

    #[test]
    fn csv_quotes_by_rfc_4180_and_guards_formulas() {
        let t = Table {
            rows: vec![{
                let mut r = vec![String::new(); COLUMNS.len()];
                r[0] = "Q, \"special\"".into();
                r[1] = "=cmd".into();
                r[11] = "+O".into();
                r
            }],
            ..Table::default()
        };
        let (text, guarded) = csv_counted(&t);
        let line = text.lines().nth(1).unwrap();
        assert!(line.starts_with("\"Q, \"\"special\"\"\",'=cmd,"), "{line}");
        assert_eq!(guarded, 1);
        // the delta column is the module's own text and is not changed
        assert!(line.contains(",+O,"), "{line}");
    }
}
