//! The automatic checks of a pathway drawing (pathway-draw): `{"out":"pathway-check"}` gives `pathway-check/1`.
//!
//! It reads the DRAWING, not the model that may have made it (`pathmodel`): the structures, the anchored arrows and the data fields
//! `NODE_ID`, `FORMULA`, `STATUS`, `GENERATION`, `MZ`, `ADDUCT` (and `TP_ID`, `RT`, `BRANCH`), so it works on a drawing built by hand,
//! by `tp:import` or opened from a file that keeps those fields.
//!
//! **What is checked** (finding keys, `severity`):
//! - `pathway.edgeMismatch` (error): the label of an arrow is a formula delta (`+O`, `-CH2`, `+C6H8O6`, `-Cl+OH`, the word of an
//!   isomer) and the delta of the two structures is another. The comparison is on element counts, so it is exact; the label is read
//!   by the delta reader of mzLab. A label that is a signed number (`+15.9949`) is compared with the difference of the two monoisotopic
//!   masses within `da` (needs the engine). `pathway.edgeUnparsed` (warn): a label that is neither (words). `pathway.edgeUnlabelled`
//!   (info). `pathway.edgeUnchecked` (info): a structure with a free-text label, two structures with the same atoms and another net
//!   charge, or a numeric label with no engine.
//! - `pathway.formulaMismatch` (error): the `FORMULA` a node states is not the formula of its structure (counts; a trailing charge is
//!   ignored).
//! - `pathway.ionMismatch` (error): the observed `MZ` differs from the m/z of its `ADDUCT` by more than `ppm`; with no adduct typed, the
//!   observed m/z is matched against the expected ions of the polarity and `pathway.ionUnmatched` (warn) says none fits.
//!   `pathway.ionNotComputed` (info): an adduct the engine has no shift for. Needs the engine.
//! - `pathway.generationMismatch` (warn): the `GENERATION` a node states is not its layer (the longest path from a root).
//! - `pathway.crossing` (warn): an arrow whose visible segment meets the box of another structure (the finding `std.scheme.crossing`
//!   of tp-05); `pathway.layout` (error): the layout is refused (a cycle, too many structures).
//! - `pathway.engine` (info): no engine, so no mass, no ion and no numeric label was checked.
//!
//! **Expected ions.** The adducts are `display.adducts` (default `[M+H]+`, `[M-H]-`), filtered by `polarity` (`positive`: charge `+`,
//! `negative`: charge `-`, `both`), plus the observed adduct of a node when the engine knows it. The m/z come from the engine
//! (`values::Measurer`): no mass is computed or typed in this crate. Without the engine every mass field is `null`.
//!
//! `ok` is true when there is no finding of severity `error`. The bytes are deterministic (fixed key order, 4 decimals for masses and
//! m/z, 1 for ppm, the same rounding as the TP table).

use crate::doc::Document;
use crate::settings::Style;
use crate::steplabel::Formulas;
use crate::values::{self, Measurer};
use serde_json::{json, Value};
use std::collections::HashMap;

/// The value of the key `format`.
pub const FORMAT: &str = "pathway-check/1";

/// Default tolerances: ppm for an ion, daltons for a numeric label.
pub const PPM: f64 = 10.0;
pub const DA: f64 = 0.005;

/// What the caller may choose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opts {
    /// 1 positive, -1 negative, 0 both.
    pub polarity: i8,
    pub ppm: f64,
    pub da: f64,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts {
            polarity: 0,
            ppm: PPM,
            da: DA,
        }
    }
}

impl Opts {
    /// `{"polarity":"both"|"positive"|"negative","ppm":N,"da":N}`, every key optional; anything else is an invalid request.
    pub fn parse(args: Option<&Value>) -> Result<Opts, &'static str> {
        const BAD: &str = "error.draw2.request";
        let mut o = Opts::default();
        let Some(v) = args else {
            return Ok(o);
        };
        for (k, val) in v.as_object().ok_or(BAD)? {
            match k.as_str() {
                "polarity" => {
                    o.polarity = match val.as_str() {
                        Some("both") => 0,
                        Some("positive") => 1,
                        Some("negative") => -1,
                        _ => return Err(BAD),
                    }
                }
                "ppm" | "da" => {
                    let x = val
                        .as_f64()
                        .filter(|x| x.is_finite() && *x > 0.0 && *x <= 1e6);
                    let x = x.ok_or(BAD)?;
                    if k == "ppm" {
                        o.ppm = x;
                    } else {
                        o.da = x;
                    }
                }
                _ => return Err(BAD),
            }
        }
        Ok(o)
    }
}

/// The answer: the document and whether it holds no error.
pub struct Report {
    pub json: String,
    pub ok: bool,
}

struct Finding {
    severity: &'static str,
    key: &'static str,
    item: String,
    detail: String,
}

fn find(
    f: &mut Vec<Finding>,
    severity: &'static str,
    key: &'static str,
    item: &str,
    detail: String,
) {
    f.push(Finding {
        severity,
        key,
        item: item.to_string(),
        detail,
    });
}

fn four(x: f64) -> Value {
    // the rounding of the TP table (`{:.4}`, ties to even on the exact binary value), kept as a number
    crate::tptable::plain4(x)
        .parse::<f64>()
        .map_or(Value::Null, Value::from)
}

fn hill(c: &[(String, u32)]) -> String {
    c.iter()
        .map(|(el, n)| {
            if *n > 1 {
                format!("{el}{n}")
            } else {
                el.clone()
            }
        })
        .collect()
}

fn data(ids: &[u32], index: &HashMap<u32, &crate::doc::Atom>, name: &str) -> String {
    for id in ids {
        if let Some(a) = index.get(id) {
            for (k, v) in a.data() {
                if k.trim().eq_ignore_ascii_case(name) && !v.trim().is_empty() {
                    return v.trim().to_string();
                }
            }
        }
    }
    String::new()
}

/// How a label reads.
enum Label {
    Empty,
    Isomer,
    Delta(Vec<(String, i64)>),
    Mass(f64),
    Words,
}

fn read_label(text: &str) -> Label {
    let t = text.trim().replace('\u{2212}', "-");
    if t.is_empty() {
        return Label::Empty;
    }
    if t.eq_ignore_ascii_case("isomer") {
        return Label::Isomer;
    }
    if t.starts_with(['+', '-']) {
        if let Ok(x) = t.parse::<f64>()
            && x.is_finite()
        {
            return Label::Mass(x);
        }
        if let Ok(mut c) = mzlab_mol::delta::parse(&t) {
            c.sort();
            return Label::Delta(c);
        }
    }
    Label::Words
}

/// The report of a drawing. `engine` gives the masses and the ions; without it they are `null` and the finding `pathway.engine` says so.
pub fn report(
    doc: &Document,
    style: &Style,
    engine: Option<Measurer>,
    opts: &Opts,
) -> Result<Report, &'static str> {
    let scan = values::scan(doc);
    let index: HashMap<u32, &crate::doc::Atom> = doc.atoms.iter().map(|a| (a.id, a)).collect();
    let owner: HashMap<u32, usize> = (scan.structures.iter().enumerate())
        .flat_map(|(k, s)| s.ids.iter().map(move |&a| (a, k)))
        .collect();
    // the structures that are nodes: a name, or an end of an anchored arrow
    let arrows: Vec<&crate::doc::Obj> = (doc.objects.iter())
        .filter(|o| o.t == crate::doc::ObjKind::Arrow && o.from != 0 && o.to != 0)
        .filter(|o| owner.get(&o.from) != owner.get(&o.to) && owner.contains_key(&o.from))
        .collect();
    let mut linked = vec![false; scan.structures.len()];
    for o in &arrows {
        for end in [o.from, o.to] {
            if let Some(&k) = owner.get(&end) {
                linked[k] = true;
            }
        }
    }
    let wanted: Vec<usize> = (0..scan.structures.len())
        .filter(|&k| linked[k] || !data(&scan.structures[k].ids, &index, "TP_ID").is_empty())
        .collect();
    if wanted.len() > crate::tptable::MAX_STRUCTURES {
        return Err("error.draw2.tpTableSize");
    }
    // the adducts: those of the style in the polarity, then the observed ones the engine knows
    let positive = |a: &str| a.ends_with('+');
    let mut adducts: Vec<String> = (style.adducts.iter())
        .filter(|a| opts.polarity == 0 || (opts.polarity > 0) == positive(a))
        .cloned()
        .collect();
    for &k in &wanted {
        let a = data(&scan.structures[k].ids, &index, "ADDUCT");
        if values::ADDUCTS.contains(&a.as_str()) && !adducts.contains(&a) {
            adducts.push(a);
        }
    }
    let measured = match engine {
        Some(m) => Some(scan.measure_within(
            &adducts,
            m,
            &[],
            (crate::tptable::MAX_ATOMS, crate::tptable::MAX_STRUCTURES),
        )?),
        None => None,
    };
    let mut findings: Vec<Finding> = Vec::new();
    if engine.is_none() {
        find(
            &mut findings,
            "info",
            "pathway.engine",
            "",
            "no engine: masses, ions and numeric labels were not checked".into(),
        );
    }
    let order = crate::tp_list::table_order(doc, style);
    let layers: HashMap<u32, usize> = match &order {
        Ok(Some(o)) => o
            .iter()
            .flat_map(|(atoms, l)| atoms.iter().map(move |&a| (a, *l)))
            .collect(),
        _ => HashMap::new(),
    };
    if let Err((key, _)) = &order {
        find(
            &mut findings,
            "error",
            "pathway.layout",
            "",
            (*key).to_string(),
        );
    }
    let mut ids: HashMap<usize, String> = HashMap::new();
    let mut nodes = Vec::new();
    for &k in &wanted {
        let s = &scan.structures[k];
        let node_id = {
            let n = data(&s.ids, &index, "NODE_ID");
            let t = data(&s.ids, &index, "TP_ID");
            if !n.is_empty() {
                n
            } else if !t.is_empty() {
                t
            } else {
                format!("#{}", s.anchor)
            }
        };
        ids.insert(k, node_id.clone());
        let label = data(&s.ids, &index, "TP_ID");
        let status = data(&s.ids, &index, "STATUS");
        let branch = data(&s.ids, &index, "BRANCH");
        let given = data(&s.ids, &index, "FORMULA");
        let gen_given = data(&s.ids, &index, "GENERATION");
        let layer = s.ids.iter().find_map(|a| layers.get(a)).copied();
        if !given.is_empty() {
            let mut have: Vec<(String, i64)> = s
                .composition
                .iter()
                .map(|(e, n)| (e.clone(), i64::from(*n)))
                .collect();
            have.sort();
            if crate::pathmodel::formula_counts(&given).as_ref() != Some(&have) {
                find(
                    &mut findings,
                    "error",
                    "pathway.formulaMismatch",
                    &node_id,
                    format!("states {given}, the structure is {}", hill(&s.composition)),
                );
            }
        }
        if let (Ok(g), Some(l)) = (gen_given.parse::<usize>(), layer)
            && g != l
        {
            find(
                &mut findings,
                "warn",
                "pathway.generationMismatch",
                &node_id,
                format!("states generation {g}, the layer is {l}"),
            );
        }
        // the ions
        let entry = measured.as_ref().and_then(|m| m.get(k));
        let mut ions = Vec::new();
        let mut mass = Value::Null;
        let meas = entry.and_then(|e| e.result.as_ref().ok());
        if let Some(m) = meas {
            mass = four(m.mass);
            for (name, mz) in adducts.iter().zip(&m.mz) {
                let on = opts.polarity == 0 || (opts.polarity > 0) == positive(name);
                if on && let Some(mz) = mz {
                    ions.push(json!({"adduct": name, "mz": four(*mz)}));
                }
            }
        } else if let Some(e) = entry
            && let Err(key) = &e.result
        {
            find(
                &mut findings,
                "info",
                "pathway.engine",
                &node_id,
                (*key).to_string(),
            );
        }
        let obs_text = data(&s.ids, &index, "MZ");
        let ion_text = data(&s.ids, &index, "ADDUCT");
        let mut observed = Value::Null;
        if let (Ok(obs), Some(m)) = (obs_text.parse::<f64>(), meas) {
            let calc_of = |name: &str| {
                adducts
                    .iter()
                    .position(|a| a == name)
                    .and_then(|i| m.mz.get(i).copied().flatten())
                    // the PRINTED expected m/z (4 decimals), as the TP table does, so that a reader recomputes the ppm from the output
                    .and_then(|c| crate::tptable::plain4(c).parse::<f64>().ok())
            };
            let ppm_of = |calc: f64| (obs - calc) / calc * 1e6;
            let (adduct, calc) = if ion_text.is_empty() {
                // no adduct typed: the expected ion of the polarity that fits, if any
                let hit = (adducts.iter())
                    .filter(|a| opts.polarity == 0 || (opts.polarity > 0) == positive(a))
                    .filter_map(|a| calc_of(a).map(|c| (a.clone(), c)))
                    .filter(|(_, c)| *c > 0.0 && ppm_of(*c).abs() <= opts.ppm)
                    .min_by(|a, b| ppm_of(a.1).abs().total_cmp(&ppm_of(b.1).abs()));
                if hit.is_none() {
                    find(
                        &mut findings,
                        "warn",
                        "pathway.ionUnmatched",
                        &node_id,
                        format!(
                            "m/z {obs_text} matches no expected ion within {} ppm",
                            opts.ppm
                        ),
                    );
                }
                match hit {
                    Some((a, c)) => (a, Some(c)),
                    None => (String::new(), None),
                }
            } else if values::ADDUCTS.contains(&ion_text.as_str()) {
                (ion_text.clone(), calc_of(&ion_text))
            } else {
                find(
                    &mut findings,
                    "info",
                    "pathway.ionNotComputed",
                    &node_id,
                    format!("no shift for {ion_text}"),
                );
                (ion_text.clone(), None)
            };
            if let Some(c) = calc.filter(|c| *c > 0.0) {
                let ppm = ppm_of(c);
                if !ion_text.is_empty() && ppm.abs() > opts.ppm {
                    find(
                        &mut findings,
                        "error",
                        "pathway.ionMismatch",
                        &node_id,
                        format!(
                            "{adduct}: observed {obs_text}, expected {}, {ppm:.1} ppm",
                            crate::tptable::plain4(c)
                        ),
                    );
                }
                let t = format!("{ppm:.1}");
                observed = json!({
                    "adduct": adduct, "mz": obs, "expected": four(c),
                    "ppm": if t == "-0.0" { 0.0 } else { t.parse::<f64>().unwrap_or(0.0) },
                });
            } else {
                observed = json!({"adduct": adduct, "mz": obs, "expected": null, "ppm": null});
            }
        } else if !obs_text.is_empty() && obs_text.parse::<f64>().is_ok() {
            observed = json!({"adduct": ion_text, "mz": obs_text.parse::<f64>().unwrap_or(0.0), "expected": null, "ppm": null});
        }
        nodes.push(json!({
            "id": node_id,
            "label": label,
            "formula": s.formula,
            "monoisotopic_mass": mass,
            "status": if status.is_empty() { "observed".to_string() } else { status },
            "generation": layer,
            "branch": branch,
            "ions": ions,
            "observed": observed,
        }));
    }
    // the edges
    let ends: Vec<u32> = arrows.iter().flat_map(|o| [o.from, o.to]).collect();
    let formulas = Formulas::new(doc, &ends);
    let mut edges = Vec::new();
    for o in &arrows {
        let (Some(&a), Some(&b)) = (owner.get(&o.from), owner.get(&o.to)) else {
            continue;
        };
        let (from, to) = (ids[&a].clone(), ids[&b].clone());
        let item = format!("{from}>{to}");
        let delta_mass = measured.as_ref().and_then(|m| {
            let (x, y) = (
                m.get(a)?.result.as_ref().ok()?,
                m.get(b)?.result.as_ref().ok()?,
            );
            Some(y.mass - x.mass)
        });
        let computed = match (formulas.root_of(o.from), formulas.root_of(o.to)) {
            (Some(x), Some(y)) => match (formulas.acc.get(&x), formulas.acc.get(&y)) {
                (Some(p), Some(q)) => crate::steplabel::pair_delta(p, q),
                _ => None,
            },
            _ => None,
        };
        let label = read_label(&o.text);
        let result = match (&computed, &label) {
            (_, Label::Empty) => {
                find(
                    &mut findings,
                    "info",
                    "pathway.edgeUnlabelled",
                    &item,
                    String::new(),
                );
                "unlabelled"
            }
            (_, Label::Words) => {
                find(
                    &mut findings,
                    "warn",
                    "pathway.edgeUnparsed",
                    &item,
                    o.text.clone(),
                );
                "unparsed"
            }
            (None, _) => {
                find(
                    &mut findings,
                    "info",
                    "pathway.edgeUnchecked",
                    &item,
                    "no honest formula delta for these two structures".into(),
                );
                "unchecked"
            }
            (Some(c), Label::Isomer) => {
                if c.is_empty() {
                    "ok"
                } else {
                    find(
                        &mut findings,
                        "error",
                        "pathway.edgeMismatch",
                        &item,
                        format!("labelled as an isomer, the formulas differ by {c}"),
                    );
                    "mismatch"
                }
            }
            (Some(c), Label::Delta(l)) => {
                let mut want = mzlab_mol::delta::parse(c).unwrap_or_default();
                want.sort();
                if &want == l {
                    "ok"
                } else {
                    find(
                        &mut findings,
                        "error",
                        "pathway.edgeMismatch",
                        &item,
                        format!(
                            "labelled {}, the formulas differ by {}",
                            o.text.trim(),
                            if c.is_empty() { "nothing" } else { c }
                        ),
                    );
                    "mismatch"
                }
            }
            (Some(_), Label::Mass(x)) => match delta_mass {
                Some(d) if (d - x).abs() <= opts.da => "ok",
                Some(d) => {
                    find(
                        &mut findings,
                        "error",
                        "pathway.edgeMismatch",
                        &item,
                        format!(
                            "labelled {}, the masses differ by {}",
                            o.text.trim(),
                            crate::tptable::shift4(d)
                        ),
                    );
                    "mismatch"
                }
                None => {
                    find(
                        &mut findings,
                        "info",
                        "pathway.edgeUnchecked",
                        &item,
                        "a numeric label needs the engine".into(),
                    );
                    "unchecked"
                }
            },
        };
        edges.push(json!({
            "from": from, "to": to,
            "label": o.text.trim(),
            "delta": computed,
            "delta_mass": delta_mass.map(|d| Value::from(crate::tptable::shift4(d).parse::<f64>().unwrap_or(0.0))),
            "result": result,
        }));
    }
    // the layout: arrows that meet the box of another structure
    let mut crossings = 0usize;
    if !arrows.is_empty() {
        for f in crate::pathway::report_with(doc, style)
            .checks
            .iter()
            .filter(|f| f.key == "std.scheme.crossing")
        {
            crossings += 1;
            let item = (f.objects.iter())
                .filter_map(|id| doc.objects.iter().find(|o| o.id == *id))
                .filter_map(|o| {
                    Some(format!(
                        "{}>{}",
                        ids.get(owner.get(&o.from)?)?,
                        ids.get(owner.get(&o.to)?)?
                    ))
                })
                .collect::<Vec<_>>()
                .join(",");
            find(
                &mut findings,
                "warn",
                "pathway.crossing",
                &item,
                String::new(),
            );
        }
    }
    let count = |s: &str| findings.iter().filter(|f| f.severity == s).count();
    let ok = count("error") == 0;
    let polarity = ["negative", "both", "positive"][(opts.polarity + 1) as usize];
    let doc_json = json!({
        "format": FORMAT,
        "ok": ok,
        "engine": engine.is_some(),
        "polarity": polarity,
        "tolerance": {"ppm": opts.ppm, "da": opts.da},
        "counts": {"nodes": nodes.len(), "edges": edges.len(), "error": count("error"), "warn": count("warn"), "info": count("info"), "crossings": crossings},
        "nodes": nodes,
        "edges": edges,
        "findings": findings.iter().map(|f| json!({"severity": f.severity, "key": f.key, "item": f.item, "detail": f.detail})).collect::<Vec<_>>(),
    });
    Ok(Report {
        json: serde_json::to_string(&doc_json).unwrap_or_default(),
        ok,
    })
}
