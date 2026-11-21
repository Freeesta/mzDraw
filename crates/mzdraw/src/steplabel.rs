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
//! Not built here: the mass modes (no mass table in `mzlab-mol`: they need the glue of arch-web-17, so they are refused with
//! `error.draw2.labelMass`) and the step names (tp-10: refused with `error.draw2.labelSteps`).

use crate::doc::{Document, ObjKind};
use crate::label;
use crate::ops::{Op, OpError};
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

    /// The refusal of a mode this build cannot write, `None` for `delta`.
    fn refusal(self) -> Option<&'static str> {
        match self {
            Mode::Delta => None,
            Mode::Mass | Mode::DeltaMass => Some("error.draw2.labelMass"),
            Mode::NameDelta => Some("error.draw2.labelSteps"),
        }
    }
}

/// The arguments of `pathway:label`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opts {
    /// Rewrite every label, the person's as well.
    pub force: bool,
    /// The word for an isomer (the page's language); empty is `ISOMER`.
    pub isomer: String,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts {
            force: false,
            isomer: ISOMER.into(),
        }
    }
}

impl Opts {
    /// Reads `{"force": bool, "isomer": text}`, both optional; anything else is an invalid request.
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
    text.split_whitespace()
        .all(|t| t == isomer || t == ISOMER || is_delta(t) || is_mass(t))
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
        if is_delta(t) {
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
struct Acc {
    counts: Vec<(String, i64)>,
    charge: i64,
    /// An atom with a free-text label (neither a table abbreviation nor an R group): the record has no atoms for it, only a stand-in.
    free_text: bool,
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
fn delta_text(from: &[(String, i64)], to: &[(String, i64)]) -> String {
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

/// The label of every anchored arrow whose text may be rewritten, as `obj.set` operations of `text`, in one linear pass: the record
/// (`Document::to_mol`, with the atoms that abbreviations expand to) is built once, its structures are the connected pieces of its
/// bonds (union-find), and the counts of a structure are summed once however many arrows end in it. The charge is not part of a delta
/// (`delta::parse` reads elements only): the delta of `CC(=O)O` to `CC(=O)[O-]` is `-H`; two structures with the same counts and a
/// different net charge are left alone and counted as skipped.
pub fn plan(doc: &Document, mode: Mode, opts: &Opts) -> Result<Plan, OpError> {
    if let Some(key) = mode.refusal() {
        return Err(OpError { key, id: 0 });
    }
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
    // the two ends of each arrow as roots of structures; the roots that are needed
    let mut ends: Vec<(u32, usize, usize, &str)> = Vec::new();
    let mut acc: HashMap<usize, Acc> = HashMap::new();
    for &(id, from, to, text) in &arrows {
        let (Some(&f), Some(&t)) = (index.get(&from), index.get(&to)) else {
            continue;
        };
        let (rf, rt) = (root(&mut up, f), root(&mut up, t));
        if rf == rt {
            continue;
        }
        acc.entry(rf).or_default();
        acc.entry(rt).or_default();
        ends.push((id, rf, rt, text));
    }
    for i in 0..n {
        let r = root(&mut up, i);
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
    for (id, rf, rt, now) in ends {
        let (a, b) = (&acc[&rf], &acc[&rt]);
        if a.free_text || b.free_text {
            out.skipped += 1;
            continue;
        }
        let d = delta_text(&a.counts, &b.counts);
        if d.is_empty() && a.charge != b.charge {
            out.skipped += 1;
            continue;
        }
        let text = if d.is_empty() { opts.isomer.clone() } else { d };
        if now == text {
            out.labelled += 1;
        } else if opts.force || owned(now, &opts.isomer) {
            out.labelled += 1;
            out.ops.push(Op::ObjSet {
                id,
                text: Some(text),
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
