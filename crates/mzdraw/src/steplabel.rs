//! Arrow labels computed from the two structures of an anchored arrow (tp-07): the command `pathway:label`.
//!
//! For each arrow `P -> T` anchored at both ends to two different structures, the formula delta `T - P` is written as the text of the
//! arrow in the canonical spelling of `mzlab_mol::delta::print` (`+O`, `-CH2`, `-Cl+OH`), on the Hill counts that the outline counts
//! (the atoms of the record, implicit hydrogens included). A pair with the same formula is an isomer: the word of the page's language.
//!
//! **Whose label it is, with no stored flag.** The text of an arrow is split into tokens on white space. It is rewritten only when it
//! is empty or when every token is one that this command writes (a signed delta, a signed number with four decimals, the isomer word);
//! any other text is the person's and is kept, unless `force` is set. An unsigned formula such as `H2O` or `NaBH4` is never a token.
//!
//! The data holds the ASCII `-`; the drawing paints a true minus sign (U+2212) in the tokens that are signed deltas (`display`).
//! **Mass modes (tp-07b).** `mass` writes the mass shift of the two structures (`+15.9949`), `delta+mass` the delta and the shift
//! (`+O +15.9949`), and the argument `adduct` (one of `values::ADDUCTS`) adds the product's m/z for it (`[M+H]+ 151.1117`). Every number
//! comes from the engine through the `values::Measurer` the caller hands to `plan` (the bridge's `glue::measure`; this crate has no
//! mass table and adds no masses): the shift is the difference of the two UNROUNDED masses, and the text is `tptable::shift4` /
//! `tptable::plain4`, the formatter of the TP table, so a label and a cell are the same digits. Without a measurer the mass modes are
//! `error.draw2.labelMass`; a structure the engine refuses (an R group, `Na+`, over 200 atoms) leaves its arrow unlabelled and is a
//! keyed note (`Plan::notes`), never a missing number. The step names (tp-10) are written by the mode `name+delta`: the name of the one step of `tp::STEPS`
//! whose delta is the label, then the delta (`dehydrogenation -H2`); a delta that two steps share (`+O`) or none has is the delta alone.

use crate::doc::{Document, ObjKind};
use crate::label;
use crate::ops::{Op, OpError};
use crate::values::Measurer;
use mzlab_mol::delta;
use std::borrow::Cow;
use std::collections::HashMap;

/// The word an arrow carries when its two structures have one formula, when the caller gives none.
pub const ISOMER: &str = "isomer";

/// What `pathway.label` writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Delta,
    Mass,
    DeltaMass,
    NameDelta,
}

impl Mode {
    pub fn parse(v: &str) -> Option<Mode> {
        match v {
            "delta" => Some(Mode::Delta),
            "mass" => Some(Mode::Mass),
            "delta+mass" => Some(Mode::DeltaMass),
            "name+delta" => Some(Mode::NameDelta),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Delta => "delta",
            Mode::Mass => "mass",
            Mode::DeltaMass => "delta+mass",
            Mode::NameDelta => "name+delta",
        }
    }

    /// Whether the mode writes numbers of the engine.
    pub(crate) fn wants_mass(self) -> bool {
        matches!(self, Mode::Mass | Mode::DeltaMass)
    }
}

/// The arguments of `pathway:label`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opts {
    /// Rewrite every label, the person's as well.
    pub force: bool,
    /// The word for an isomer (the page's language); empty is `ISOMER`.
    pub isomer: String,
    /// The adduct whose m/z of the product is written after the label (a name of `values::ADDUCTS`); only for `mass` and `delta+mass`.
    pub adduct: Option<String>,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts {
            force: false,
            isomer: ISOMER.into(),
            adduct: None,
        }
    }
}

impl Opts {
    /// Reads `{"force": bool, "isomer": text, "adduct": name}`, all optional; anything else is an invalid request.
    pub fn parse(args: &serde_json::Value) -> Result<Opts, &'static str> {
        let bad = "error.draw2.request";
        let o = args.as_object().ok_or(bad)?;
        let mut r = Opts::default();
        for (k, v) in o {
            match k.as_str() {
                "force" => r.force = v.as_bool().ok_or(bad)?,
                "isomer" => {
                    let w = v.as_str().ok_or(bad)?;
                    if w.is_empty()
                        || w.chars().count() > 40
                        || w.contains(char::is_whitespace)
                        || w.contains(['\n', '\r'])
                    {
                        return Err(bad);
                    }
                    r.isomer = w.into();
                }
                "adduct" => {
                    let a = v.as_str().ok_or(bad)?;
                    if !crate::values::ADDUCTS.contains(&a) {
                        return Err(bad);
                    }
                    r.adduct = Some(a.into());
                }
                _ => return Err(bad),
            }
        }
        Ok(r)
    }
}

/// What a run did: the operations and the counts of the live line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub ops: Vec<Op>,
    /// Arrows whose label is now the computed one (written or already so).
    pub labelled: usize,
    /// Arrows whose text is the person's and was kept.
    pub kept: usize,
    /// Arrows left alone because the two structures have the same atoms and a different net charge (a delta has no charges).
    pub skipped: usize,
    /// What was left out for a reason, as (i18n key, arrows): the refusal key of the engine for a structure it cannot measure (the arrow is
    /// then in `skipped`), `draw2.label.mzCharged` for a product with a charge (it has no adduct: the shift is written, the m/z is not).
    pub notes: Vec<(&'static str, usize)>,
}

fn note(notes: &mut Vec<(&'static str, usize)>, key: &'static str) {
    match notes.iter_mut().find(|n| n.0 == key) {
        Some(n) => n.1 += 1,
        None => notes.push((key, 1)),
    }
}

/// A text that is a signed number with exactly four decimals (`+15.9949`, `-14.0157`; U+2212 or `-`).
fn is_mass(t: &str) -> bool {
    let mut c = t.chars();
    if !matches!(c.next(), Some('+' | '-' | '\u{2212}')) {
        return false;
    }
    let rest = c.as_str();
    match rest.split_once('.') {
        Some((i, d)) => {
            !i.is_empty()
                && i.bytes().all(|b| b.is_ascii_digit())
                && d.len() == 4
                && d.bytes().all(|b| b.is_ascii_digit())
        }
        None => false,
    }
}

/// A text that is an unsigned number with exactly four decimals (the m/z after an adduct).
fn is_plain4(t: &str) -> bool {
    match t.split_once('.') {
        Some((i, d)) => {
            !i.is_empty()
                && i.bytes().all(|b| b.is_ascii_digit())
                && d.len() == 4
                && d.bytes().all(|b| b.is_ascii_digit())
        }
        None => false,
    }
}

/// A text that is a signed delta: a sign, then what `delta::parse` reads after the sign is mapped to ASCII, with at least one element.
fn is_delta(t: &str) -> bool {
    let mapped = t.replace('\u{2212}', "-");
    let mut c = mapped.chars();
    if !matches!(c.next(), Some('+' | '-')) || !c.next().is_some_and(|x| x.is_ascii_uppercase()) {
        return false;
    }
    delta::parse(&mapped).is_ok_and(|n| !n.is_empty())
}

/// Whether `text` may be rewritten without `force`: empty, or only tokens this command writes.
pub fn owned(text: &str, isomer: &str) -> bool {
    // an m/z number is ours only right after the adduct token that introduces it: a bare `151.1117` is the person's
    let mut after_adduct = false;
    text.split_whitespace().all(|t| {
        let adduct = crate::values::ADDUCTS.contains(&t);
        let ours = t == isomer
            || t == ISOMER
            || is_delta(t)
            || is_mass(t)
            || adduct
            || (after_adduct && is_plain4(t))
            || crate::tp::step(t).is_some();
        after_adduct = adduct;
        ours
    })
}

/// The text as painted: the `-` of a token that is a signed delta becomes the true minus sign (U+2212). The data is not changed.
pub fn display(text: &str) -> Cow<'_, str> {
    if !text.contains('-') {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 4);
    let mut first = true;
    for t in text.split(' ') {
        if !first {
            out.push(' ');
        }
        first = false;
        if is_delta(t) || is_mass(t) {
            out.push_str(&t.replace('-', "\u{2212}"));
        } else {
            out.push_str(t);
        }
    }
    Cow::Owned(out)
}

/// The Hill counts of one structure and its net charge, as the outline counts them (every atom of the record, so an abbreviation counts
/// as the atoms it stands for; an R group is the element `R`).
#[derive(Default, Clone)]
pub(crate) struct Acc {
    pub(crate) counts: Vec<(String, i64)>,
    pub(crate) charge: i64,
    /// An atom with a free-text label (neither a table abbreviation nor an R group): the record has no atoms for it, only a stand-in.
    pub(crate) free_text: bool,
}

fn bump(c: &mut Vec<(String, i64)>, el: &str, n: i64) {
    if n == 0 {
        return;
    }
    match c.iter_mut().find(|e| e.0 == el) {
        Some(e) => e.1 += n,
        None => c.push((el.to_string(), n)),
    }
}

/// The text of the delta `to - from`, in the canonical spelling; empty for equal formulas.
pub(crate) fn delta_text(from: &[(String, i64)], to: &[(String, i64)]) -> String {
    let mut sum: Vec<(String, i64)> = Vec::new();
    for (el, n) in to {
        bump(&mut sum, el, *n);
    }
    for (el, n) in from {
        bump(&mut sum, el, -*n);
    }
    sum.retain(|e| e.1 != 0);
    sum.sort();
    delta::print(&sum)
}

/// The most atoms a drawing may have for `pathway:label` (a hostile document is refused before any work).
pub const MAX_ATOMS: usize = 100_000;

fn root(p: &mut [usize], mut i: usize) -> usize {
    while p[i] != i {
        p[i] = p[p[i]];
        i = p[i];
    }
    i
}

/// The structures of a drawing as the record sees them, and the counts of the ones asked for, in one linear pass: the record
/// (`Document::to_mol`, with the atoms abbreviations expand to) is built once, its structures are the connected pieces of its bonds
/// (union-find), and the counts of a structure are summed once however many arrows end in it. Shared by `pathway:label` and the diff
/// lens (`tp::lenses`), so the formula of a structure is counted in one place.
pub(crate) struct Formulas {
    index: HashMap<u32, usize>,
    /// The root (record index of the first atom) of the structure of each record atom.
    roots: Vec<usize>,
    pub(crate) acc: HashMap<usize, Acc>,
}

impl Formulas {
    /// The counts of the structures that hold the atoms `wanted` (atom ids that no atom carries are ignored).
    pub(crate) fn new(doc: &Document, wanted: &[u32]) -> Formulas {
        let index: HashMap<u32, usize> = doc
            .atoms
            .iter()
            .enumerate()
            .map(|(i, a)| (a.id, i))
            .collect();
        let mol = doc.to_mol();
        let n = mol.atoms.len();
        let mut up: Vec<usize> = (0..n).collect();
        for b in &mol.bonds {
            let (x, z) = (root(&mut up, b.a as usize), root(&mut up, b.b as usize));
            up[x.max(z)] = x.min(z);
        }
        let roots: Vec<usize> = (0..n).map(|i| root(&mut up, i)).collect();
        let mut acc: HashMap<usize, Acc> = HashMap::new();
        for w in wanted {
            if let Some(&i) = index.get(w) {
                acc.entry(roots[i]).or_default();
            }
        }
        for (i, &r) in roots.iter().enumerate() {
            let Some(a) = acc.get_mut(&r) else {
                continue;
            };
            let kind = doc
                .atoms
                .get(i)
                .map(|x| (x.lab.is_empty(), label::kind(&x.lab)));
            if matches!(kind, Some((false, None))) {
                a.free_text = true;
            } else if matches!(kind, Some((_, Some(label::Kind::R(_))))) {
                bump(&mut a.counts, "R", 1);
            } else {
                let m = &mol.atoms[i];
                bump(&mut a.counts, &m.el, 1);
                bump(&mut a.counts, "H", i64::from(m.h));
                a.charge += i64::from(m.chg);
            }
        }
        Formulas { index, roots, acc }
    }

    /// The root of the structure that holds the atom `id`.
    pub(crate) fn root_of(&self, id: u32) -> Option<usize> {
        self.index
            .get(&id)
            .and_then(|&i| self.roots.get(i).copied())
    }
}

/// The delta `to - from` as `pathway:label` writes it, or `None` where it has no honest text: a structure with a free-text label (the
/// record has only a stand-in for it), or two structures with the same counts and a different net charge (a delta has no charges).
/// `Some("")` is an isomer.
pub(crate) fn pair_delta(from: &Acc, to: &Acc) -> Option<String> {
    if from.free_text || to.free_text {
        return None;
    }
    let d = delta_text(&from.counts, &to.counts);
    (!d.is_empty() || from.charge == to.charge).then_some(d)
}

/// The label of every anchored arrow whose text may be rewritten, as `obj.set` operations of `text`, in one linear pass: the record
/// (`Document::to_mol`, with the atoms that abbreviations expand to) is built once, its structures are the connected pieces of its
/// bonds (union-find), and the counts of a structure are summed once however many arrows end in it. The charge is not part of a delta
/// (`delta::parse` reads elements only): the delta of `CC(=O)O` to `CC(=O)[O-]` is `-H`; two structures with the same counts and a
/// different net charge are left alone and counted as skipped.
///
/// The modes `mass` and `delta+mass` need `engine` (`error.draw2.labelMass` without one); `opts.adduct` is for them only.
pub fn plan(
    doc: &Document,
    mode: Mode,
    opts: &Opts,
    engine: Option<Measurer>,
) -> Result<Plan, OpError> {
    let request = OpError {
        key: "error.draw2.request",
        id: 0,
    };
    if opts.adduct.is_some() && !mode.wants_mass() {
        return Err(request);
    }
    let engine = match (mode.wants_mass(), engine) {
        (true, None) => {
            return Err(OpError {
                key: "error.draw2.labelMass",
                id: 0,
            })
        }
        (true, Some(e)) => Some(e),
        (false, _) => None,
    };
    let size = OpError {
        key: "error.draw2.schemeSize",
        id: 0,
    };
    if doc.atoms.len() > MAX_ATOMS {
        return Err(size);
    }
    let mut arrows: Vec<(u32, u32, u32, &str)> = Vec::new();
    for o in &doc.objects {
        if o.t == ObjKind::Arrow && o.from != 0 && o.to != 0 {
            if arrows.len() == crate::pathway::MAX_ARROWS {
                return Err(size);
            }
            arrows.push((o.id, o.from, o.to, o.text.as_str()));
        }
    }
    let mut out = Plan::default();
    if arrows.is_empty() {
        return Ok(out);
    }
    let ends_of: Vec<u32> = arrows.iter().flat_map(|a| [a.1, a.2]).collect();
    let fm = Formulas::new(doc, &ends_of);
    /// An arrow joining two structures: its id, the roots of the two structures, its text, the atoms it is anchored to.
    type End<'a> = (u32, usize, usize, &'a str, (u32, u32));
    let mut ends: Vec<End> = Vec::new();
    for &(id, from, to, text) in &arrows {
        if let (Some(rf), Some(rt)) = (fm.root_of(from), fm.root_of(to))
            && rf != rt
        {
            ends.push((id, rf, rt, text, (from, to)));
        }
    }
    let acc = &fm.acc;
    // the numbers of the engine, one run per structure (the same scan and caps as the TP table), asked for before any text is made
    let adducts: Vec<String> = opts.adduct.iter().cloned().collect();
    let measured = match engine {
        Some(e) => {
            let scan = crate::values::scan(doc);
            let owner: HashMap<u32, usize> = (scan.structures.iter().enumerate())
                .flat_map(|(k, s)| s.ids.iter().map(move |&a| (a, k)))
                .collect();
            let m = scan
                .measure_within(
                    &adducts,
                    e,
                    &[],
                    (crate::tptable::MAX_ATOMS, crate::tptable::MAX_STRUCTURES),
                )
                .map_err(|key| OpError { key, id: 0 })?;
            Some((owner, m))
        }
        None => None,
    };
    for (id, rf, rt, now, (af, at)) in ends {
        let Some(d) = pair_delta(&acc[&rf], &acc[&rt]) else {
            out.skipped += 1;
            continue;
        };
        let mut text = match (d.is_empty(), mode) {
            (true, _) => opts.isomer.clone(),
            (false, Mode::NameDelta) => match crate::tp::name_of(&d) {
                Some(name) => format!("{name} {d}"),
                None => d.clone(),
            },
            (false, Mode::Mass) => String::new(),
            (false, _) => d.clone(),
        };
        if let Some((owner, m)) = &measured {
            // the two structures the arrow joins, by the atoms it is anchored to
            let (Some(from), Some(to)) = (owner.get(&af), owner.get(&at)) else {
                // not reachable: every atom an anchored arrow ends in is in a structure of the scan (both read the same record); if it
                // ever is, the arrow is skipped and says so, never silently
                out.skipped += 1;
                note(&mut out.notes, "draw2.label.massMissing");
                continue;
            };
            let (a, b) = match (&m[*from].result, &m[*to].result) {
                (Ok(a), Ok(b)) => (a, b),
                (Err(key), _) | (_, Err(key)) => {
                    out.skipped += 1;
                    note(&mut out.notes, key);
                    continue;
                }
            };
            let mut parts: Vec<String> = Vec::new();
            if !text.is_empty() {
                parts.push(text);
            }
            if !d.is_empty() {
                // the difference of the unrounded masses, rounded once, as the `mass_diff` of the TP table
                parts.push(crate::tptable::shift4(b.mass - a.mass));
            }
            if let Some(adduct) = &opts.adduct {
                match b.mz.first().copied().flatten() {
                    Some(v) => {
                        parts.push(adduct.clone());
                        parts.push(crate::tptable::plain4(v));
                    }
                    None => note(&mut out.notes, "draw2.label.mzCharged"),
                }
            }
            text = parts.join(" ");
        }
        if now == text {
            out.labelled += 1;
        } else if opts.force || owned(now, &opts.isomer) {
            out.labelled += 1;
            out.ops.push(Op::ObjSet {
                id,
                text: Some(text),
                runs: None,
                x2: None,
                y2: None,
                from: None,
                to: None,
            });
        } else {
            out.kept += 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens() {
        for t in ["+O", "-CH2", "\u{2212}O", "-Cl+OH", "+C6H8O6"] {
            assert!(is_delta(t), "{t}");
        }
        for t in [
            "O", "H2O", "O3", "NaBH4", "+", "+o", "+O,", "(CYP)", "37", "+1.5",
        ] {
            assert!(!is_delta(t), "{t}");
        }
        assert!(is_mass("+15.9949") && is_mass("\u{2212}14.0157") && !is_mass("+15.99"));
        assert!(owned("", "isomer") && owned("+O\n+15.9949", "isomer"));
        assert!(!owned("+O, 37 \u{b0}C", "isomer") && !owned("hydroxylation (CYP)", "isomer"));
        assert_eq!(display("-H2 and -x"), "\u{2212}H2 and -x");
    }

    #[test]
    fn canonical_deltas() {
        let c = |s: &str| delta::parse(s).unwrap();
        assert_eq!(delta_text(&c("+C7H8O"), &c("+C7H8O2")), "+O");
        assert_eq!(delta_text(&c("+C7H8O2"), &c("+C6H6O2")), "-CH2");
        assert_eq!(delta_text(&c("+C2H6"), &c("+C2H6")), "");
    }
}
