//! The automatic checks of a pathway drawing (pathway-draw): `{"out":"pathway-check"}` gives `pathway-check/1`.
//!
//! It reads the DRAWING, not the model that may have made it (`pathmodel`): the structures, the anchored arrows and the data fields
//! `NODE_ID`, `FORMULA`, `STATUS`, `GENERATION`, `MZ`, `ADDUCT` (and `TP_ID`, `RT`, `BRANCH`), so it works on a drawing built by hand,
//! by `tp:import` or opened from a file that keeps those fields.
//!
//! **What is checked** (finding keys, `severity`; the texts are `pathway.*` in `web/lang/mzdraw.js`):
//! - `pathway.edgeMismatch` (error): the label of an arrow is a formula delta and the delta of the two structures is another. The label is
//!   read as a mass spectrometrist writes it (`read_label`): signed terms with an optional leading count and spaces (`+O`, `-CH2`, `+2H`,
//!   `+O-2H`, `+O2-2H`, `+2 H`, `-Cl+OH`), a free word before the first term (`glucuronidation +C6H8O6`), a comment in parentheses after a term (`+O (hydroxylation)`), never a bare token after a term (`+O 2H` is unreadable, not `+O`), the word of an
//!   isomer; the comparison is on element counts, so it is exact. A signed number (`+15.9949`) is compared with the difference of the two
//!   monoisotopic masses within `da` (needs the engine), also where the two structures have no honest formula delta.
//! - `pathway.edgeUnparsed` (error, or warn with `lenient_labels`): a label that is given and cannot be read; it never passes in silence.
//!   `pathway.edgeUnlabelled` (info). `pathway.edgeUnchecked` (info): a structure with a free-text label, two structures with the same
//!   atoms and another net charge (a formula label), or a numeric label with no engine.
//! - `pathway.mzUnreadable` (error): the observed `MZ` of a node is not a finite number above 0 (`NaN`, `0x10`, `59,0139`): it never passes.
//! - `pathway.formulaMismatch` (error): the `FORMULA` a node states is not the formula of its structure (counts; a trailing charge is
//!   ignored). `pathway.isotopesNotCompared` (warn): a structure with isotopes: formulas and deltas count elements, the masses count isotopes.
//! - `pathway.ionMismatch` (error): the observed `MZ` differs from the m/z of its `ADDUCT` by more than `ppm`; with no adduct typed, the
//!   observed m/z is matched against the expected ions of the polarity and `pathway.ionUnmatched` (warn) says none fits. The UNROUNDED
//!   expected value decides; only the printed number is rounded. A charged structure is its own ion (`[M]+`, `[M]-`, the engine's ion m/z:
//!   no adduct, no electron added twice). `pathway.ionNotComputed` (info): an adduct the engine has no shift for. Needs the engine.
//! - `pathway.generationMismatch` (warn): the `GENERATION` a node states is not its layer (the longest path from a root).
//! - The layout, three measures (`counts`): `arrow_box_crossings` (`pathway.crossing`: an arrow whose visible segment meets the box of another
//!   structure, the finding `std.scheme.crossing` of tp-05), `edge_crossings` (`pathway.edgeCrossing`: two arrows cross) and
//!   `label_box_overlaps` (`pathway.labelOverlap`: the box of an arrow's text, ESTIMATED as the width the painter sets above the middle of the
//!   shaft, meets a structure, another text or another arrow's shaft). `pathway.layout` (error): the layout is refused (a cycle, too many).
//! - `pathway.engine`: no engine, so no mass, no ion and no numeric label was checked. The summary says `checked: false` and
//!   `counts.unchecked`; the finding is info, and an error with `require_engine`.
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
    /// A label that cannot be read is a warning instead of an error.
    pub lenient_labels: bool,
    /// No engine is an error (the check could not do its job), not only a note.
    pub require_engine: bool,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts {
            polarity: 0,
            ppm: PPM,
            da: DA,
            lenient_labels: false,
            require_engine: false,
        }
    }
}

impl Opts {
    /// `{"polarity":"both"|"positive"|"negative","ppm":N,"da":N,"lenient_labels":BOOL,"require_engine":BOOL}`, every key optional;
    /// anything else is an invalid request.
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
                "lenient_labels" | "require_engine" => {
                    let b = val.as_bool().ok_or(BAD)?;
                    if k == "lenient_labels" {
                        o.lenient_labels = b;
                    } else {
                        o.require_engine = b;
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
    /// Neither a formula delta nor a number: words, or a notation that is not understood.
    Unreadable,
}

/// Whether what follows a sign starts a term of a formula delta: a count or an element.
fn term_start(rest: &str) -> bool {
    rest.trim_start()
        .starts_with(|c: char| c.is_ascii_digit() || c.is_ascii_uppercase())
}

/// Reads a label as a mass spectrometrist writes it: signed terms with an optional leading count and spaces (`+O`, `-CH2`, `+2H`,
/// `+O-2H`, `+O2-2H`, `-Cl+OH`, `+2 H`), after an optional free-text word (`glucuronidation +C6H8O6`) and before an optional one
/// (`+O (hydroxylation)`); `2H` is `H2` and the count of a term multiplies all of its elements (`+2CH2` is `+C2H4`). A signed number
/// (`+15.9949`) is a mass. The isomer word. Anything else cannot be read.
fn read_label(text: &str) -> Label {
    let t = text.trim().replace('\u{2212}', "-");
    if t.is_empty() {
        return Label::Empty;
    }
    if t.eq_ignore_ascii_case("isomer") {
        return Label::Isomer;
    }
    let starts: Vec<usize> = t
        .char_indices()
        .filter(|&(i, c)| matches!(c, '+' | '-') && term_start(&t[i + 1..]))
        .map(|(i, _)| i)
        .collect();
    let Some(&first) = starts.first() else {
        return Label::Unreadable;
    };
    let body = &t[first..];
    if let Ok(x) = body.trim().parse::<f64>()
        && x.is_finite()
    {
        return Label::Mass(x);
    }
    let mut sum: Vec<(String, i64)> = Vec::new();
    for (k, &at) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(t.len());
        let sign: i64 = if t[at..].starts_with('+') { 1 } else { -1 };
        let term = t[at + 1..end].trim_start();
        let digits = term.bytes().take_while(u8::is_ascii_digit).count();
        let count: i64 = if digits == 0 {
            1
        } else {
            match term[..digits].parse() {
                Ok(n) if n <= 1000 => n,
                _ => return Label::Unreadable,
            }
        };
        let rest = term[digits..].trim_start();
        let token: String = rest
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        // what follows the formula is a comment only in parentheses (`+O (hydroxylation)`): a bare token after a term is never skipped (`+O 2H`
        // is not read as `+O`), so a wrong label cannot pass as a right one with a comment that is really more of the delta; it is unreadable
        let after = rest[token.len()..].trim();
        // exactly ONE parenthesis group, with no parenthesis inside it and nothing after it (`+O (a) 2H (b)` and `+O (a)b)` are unreadable)
        let comment = after.len() >= 2
            && after.starts_with('(')
            && after.ends_with(')')
            && !after[1..after.len() - 1].contains(['(', ')']);
        if token.is_empty() || !(after.is_empty() || comment) {
            return Label::Unreadable;
        }
        let Ok(counts) = mzlab_mol::delta::parse(&format!("+{token}")) else {
            return Label::Unreadable;
        };
        for (el, n) in counts {
            match sum.iter_mut().find(|e| e.0 == el) {
                Some(e) => e.1 += sign * count * n,
                None => sum.push((el, sign * count * n)),
            }
        }
    }
    sum.retain(|e| e.1 != 0);
    sum.sort();
    Label::Delta(sum)
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
        // a check that could not look at the masses says so in its summary (`checked`), and an error when the caller requires them
        find(
            &mut findings,
            if opts.require_engine { "error" } else { "info" },
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
        if !s.iso.is_empty() {
            // the formula and the delta count elements, not isotopes; the masses do count them
            find(
                &mut findings,
                "warn",
                "pathway.isotopesNotCompared",
                &node_id,
                format!(
                    "isotopes ({}) are not compared in formulas and deltas",
                    s.iso
                ),
            );
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
        // the ions: the adducts of a neutral structure, the structure itself for a charged one (its own ion, no adduct, no electrons added)
        let entry = measured.as_ref().and_then(|m| m.get(k));
        let mut ions = Vec::new();
        let mut mass = Value::Null;
        let meas = entry.and_then(|e| e.result.as_ref().ok());
        let own = (s.charge != 0).then(|| {
            let n = s.charge.unsigned_abs();
            let sign = if s.charge > 0 { '+' } else { '-' };
            format!(
                "[M]{}{sign}",
                if n > 1 { n.to_string() } else { String::new() }
            )
        });
        let polar = |sign_plus: bool| opts.polarity == 0 || (opts.polarity > 0) == sign_plus;
        if let Some(m) = meas {
            mass = four(m.mass);
            if let (Some(name), Some(mz)) = (&own, m.ion_mz) {
                if polar(s.charge > 0) {
                    ions.push(json!({"adduct": name, "mz": four(mz)}));
                }
            } else {
                for (name, mz) in adducts.iter().zip(&m.mz) {
                    if polar(positive(name))
                        && let Some(mz) = mz
                    {
                        ions.push(json!({"adduct": name, "mz": four(*mz)}));
                    }
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
        // an observed m/z that is not a finite number above 0 (`NaN`, `0x10`, `59,0139`, `-5`) is an error, whatever the engine: it must not pass
        let obs_ok = obs_text
            .parse::<f64>()
            .ok()
            .filter(|x| x.is_finite() && *x > 0.0);
        if !obs_text.is_empty() && obs_ok.is_none() {
            find(
                &mut findings,
                "error",
                "pathway.mzUnreadable",
                &node_id,
                format!("the observed m/z {obs_text:?} is not a number above 0"),
            );
            observed = json!({"adduct": ion_text, "mz": null, "expected": null, "ppm": null});
        }
        if let (Some(obs), Some(m)) = (obs_ok, meas) {
            // the UNROUNDED expected m/z decides; only the printed number is rounded
            let calc_of = |name: &str| -> Option<f64> {
                if let Some(own) = &own {
                    return (name == own).then_some(m.ion_mz).flatten();
                }
                adducts
                    .iter()
                    .position(|a| a == name)
                    .and_then(|i| m.mz.get(i).copied().flatten())
            };
            let ppm_of = |calc: f64| (obs - calc) / calc * 1e6;
            let candidates: Vec<String> = match &own {
                Some(o) => vec![o.clone()],
                None => adducts
                    .iter()
                    .filter(|a| polar(positive(a)))
                    .cloned()
                    .collect(),
            };
            let known = |a: &str| own.as_deref() == Some(a) || values::ADDUCTS.contains(&a);
            let (adduct, calc) = if ion_text.is_empty() {
                // no adduct typed: the expected ion of the polarity that fits, if any
                let hit = (candidates.iter())
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
            } else if known(&ion_text)
                && (own.is_none() || own.as_deref() == Some(ion_text.as_str()))
            {
                (ion_text.clone(), calc_of(&ion_text))
            } else {
                find(
                    &mut findings,
                    "info",
                    "pathway.ionNotComputed",
                    &node_id,
                    if own.is_some() {
                        format!("{ion_text}: a charged structure is its own ion")
                    } else {
                        format!("no shift for {ion_text}")
                    },
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
        } else if let Some(obs) = obs_ok {
            observed = json!({"adduct": ion_text, "mz": obs, "expected": null, "ppm": null});
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
        let unreadable_severity = if opts.lenient_labels { "warn" } else { "error" };
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
            // a label that is given and cannot be read must not pass in silence
            (_, Label::Unreadable) => {
                find(
                    &mut findings,
                    unreadable_severity,
                    "pathway.edgeUnparsed",
                    &item,
                    o.text.clone(),
                );
                "unreadable"
            }
            // a number is compared with the masses, whether or not the two structures have an honest formula delta
            (_, Label::Mass(x)) => match delta_mass {
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
        };
        edges.push(json!({
            "from": from, "to": to,
            "label": o.text.trim(),
            "delta": computed,
            "delta_mass": delta_mass.map(|d| Value::from(crate::tptable::shift4(d).parse::<f64>().unwrap_or(0.0))),
            "result": result,
        }));
    }
    // the layout, three measures: arrows that meet the box of another structure (`arrow_box_crossings`), arrows that cross each other
    // (`edge_crossings`), and the box of an arrow's text (estimated: the width the painter sets, above the middle of the shaft) that meets a
    // structure's box or another text (`label_box_overlaps`)
    let name_of = |a: u32| {
        owner
            .get(&a)
            .and_then(|k| ids.get(k))
            .cloned()
            .unwrap_or_default()
    };
    let pair = |o: &crate::doc::Obj| format!("{}>{}", name_of(o.from), name_of(o.to));
    let mut arrow_box = 0usize;
    if !arrows.is_empty() {
        for f in crate::pathway::report_with(doc, style)
            .checks
            .iter()
            .filter(|f| f.key == "std.scheme.crossing")
        {
            arrow_box += 1;
            let item = (f.objects.iter())
                .filter_map(|id| doc.objects.iter().find(|o| o.id == *id))
                .map(&pair)
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
    let mut edge_cross = 0usize;
    for (i, p) in arrows.iter().enumerate() {
        for q in &arrows[i + 1..] {
            if segments_cross((p.x, p.y, p.x2, p.y2), (q.x, q.y, q.x2, q.y2)) {
                edge_cross += 1;
                find(
                    &mut findings,
                    "warn",
                    "pathway.edgeCrossing",
                    &format!("{},{}", pair(p), pair(q)),
                    String::new(),
                );
            }
        }
    }
    let boxes: Vec<(usize, [f64; 4])> = wanted
        .iter()
        .map(|&k| {
            (
                k,
                crate::route::piece_box(doc, &scan.structures[k].ids, style),
            )
        })
        .collect();
    let labels: Vec<(&crate::doc::Obj, [f64; 4])> = arrows
        .iter()
        .filter(|o| !o.text.trim().is_empty())
        .map(|o| (*o, label_box(o, style)))
        .collect();
    let mut label_overlap = 0usize;
    for (i, (o, b)) in labels.iter().enumerate() {
        for (k, sb) in &boxes {
            if boxes_meet(b, sb) {
                label_overlap += 1;
                find(
                    &mut findings,
                    "warn",
                    "pathway.labelOverlap",
                    &format!("{},{}", pair(o), ids[k]),
                    String::new(),
                );
            }
        }
        // the box of the text on the shaft of another arrow
        for q in &arrows {
            if std::ptr::eq(*q, *o) {
                continue;
            }
            if segment_meets_box((q.x, q.y, q.x2, q.y2), b) {
                label_overlap += 1;
                find(
                    &mut findings,
                    "warn",
                    "pathway.labelOverlap",
                    &format!("{},{}", pair(o), pair(q)),
                    String::new(),
                );
            }
        }
        for (q, qb) in &labels[i + 1..] {
            if boxes_meet(b, qb) {
                label_overlap += 1;
                find(
                    &mut findings,
                    "warn",
                    "pathway.labelOverlap",
                    &format!("{},{}", pair(o), pair(q)),
                    String::new(),
                );
            }
        }
    }
    let count = |s: &str| findings.iter().filter(|f| f.severity == s).count();
    let ok = count("error") == 0;
    // what could not be looked at: every node's masses and ions without the engine, and each edge left unchecked
    let unchecked = if engine.is_none() { nodes.len() } else { 0 }
        + edges.iter().filter(|e| e["result"] == "unchecked").count()
        + findings
            .iter()
            .filter(|f| f.key == "pathway.ionNotComputed")
            .count();
    let polarity = ["negative", "both", "positive"][(opts.polarity + 1) as usize];
    let doc_json = json!({
        "format": FORMAT,
        "ok": ok,
        "engine": engine.is_some(),
        "checked": engine.is_some(),
        "polarity": polarity,
        "tolerance": {"ppm": opts.ppm, "da": opts.da},
        "counts": {"nodes": nodes.len(), "edges": edges.len(), "error": count("error"), "warn": count("warn"), "info": count("info"), "unchecked": unchecked, "arrow_box_crossings": arrow_box, "edge_crossings": edge_cross, "label_box_overlaps": label_overlap},
        "nodes": nodes,
        "edges": edges,
        "findings": findings.iter().map(|f| json!({"severity": f.severity, "key": f.key, "item": f.item, "detail": f.detail})).collect::<Vec<_>>(),
    });
    Ok(Report {
        json: serde_json::to_string(&doc_json).unwrap_or_default(),
        ok,
    })
}

/// Whether two segments (x1, y1, x2, y2) cross properly (an interior point of both).
fn segments_cross(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    let side = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| {
        (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
    };
    let (a1, a2, b1, b2) = ((a.0, a.1), (a.2, a.3), (b.0, b.1), (b.2, b.3));
    let (d1, d2, d3, d4) = (
        side(b1, b2, a1),
        side(b1, b2, a2),
        side(a1, a2, b1),
        side(a1, a2, b2),
    );
    d1 * d2 < -1e-12 && d3 * d4 < -1e-12
}

/// The estimated box of the text of an arrow (bond lengths, y up): as wide as the painter sets it, centred above the middle of the shaft.
fn label_box(o: &crate::doc::Obj, style: &Style) -> [f64; 4] {
    let w = crate::pathway::label_width(&o.text, style);
    let font = style.metrics(1.0, false).font;
    let (dx, dy) = (o.x2 - o.x, o.y2 - o.y);
    let len = dx.hypot(dy).max(1e-9);
    // the normal that points up
    let (mut nx, mut ny) = (-dy / len, dx / len);
    if ny < 0.0 {
        (nx, ny) = (-nx, -ny);
    }
    let off = font * 0.8;
    let (cx, cy) = ((o.x + o.x2) / 2.0 + nx * off, (o.y + o.y2) / 2.0 + ny * off);
    let h = font * 0.85;
    [cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0]
}

/// Whether two boxes [x0, y0, x1, y1] share an area.
fn boxes_meet(a: &[f64; 4], b: &[f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// Whether a segment (x1, y1, x2, y2) passes through the inside of a box [x0, y0, x1, y1] (Liang-Barsky).
fn segment_meets_box(sg: (f64, f64, f64, f64), b: &[f64; 4]) -> bool {
    let (dx, dy) = (sg.2 - sg.0, sg.3 - sg.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, sg.0 - b[0]),
        (dx, b[2] - sg.0),
        (-dy, sg.1 - b[1]),
        (dy, b[3] - sg.1),
    ] {
        if p.abs() < 1e-12 {
            if q <= 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    t0 + 1e-9 < t1
}
