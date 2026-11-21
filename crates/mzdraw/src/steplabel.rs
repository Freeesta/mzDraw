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
    /// Arrows left alone because an end holds an R group or a charge (a delta has no charges).
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

/// The Hill counts of one structure: the elements with the implicit hydrogens, as the outline counts them. `None` when it holds an
/// R group or has a net charge (the delta has neither).
type Counts = Option<Vec<(String, i64)>>;

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

/// The label of every anchored arrow whose text may be rewritten, as `obj.set` operations of `text`, in one pass over the document:
/// the structures (`layout::pieces`) and the record (`Document::to_mol`) are built once, and the counts of a structure once, however
/// many arrows end in it.
pub fn plan(doc: &Document, mode: Mode, opts: &Opts) -> Result<Plan, OpError> {
    if let Some(key) = mode.refusal() {
        return Err(OpError { key, id: 0 });
    }
    let arrows: Vec<(u32, u32, u32)> = (doc.objects.iter())
        .filter(|o| o.t == ObjKind::Arrow && o.from != 0 && o.to != 0)
        .map(|o| (o.id, o.from, o.to))
        .collect();
    if arrows.len() > crate::pathway::MAX_ARROWS {
        return Err(OpError {
            key: "error.draw2.schemeSize",
            id: 0,
        });
    }
    let mut out = Plan::default();
    if arrows.is_empty() {
        return Ok(out);
    }
    let pieces = crate::layout::pieces(doc);
    let mut piece_of: HashMap<u32, usize> = HashMap::new();
    for (i, p) in pieces.iter().enumerate() {
        for &a in p {
            piece_of.insert(a, i);
        }
    }
    let index: HashMap<u32, usize> = doc
        .atoms
        .iter()
        .enumerate()
        .map(|(i, a)| (a.id, i))
        .collect();
    let mol = doc.to_mol();
    let mut memo: HashMap<usize, Counts> = HashMap::new();
    let mut counts_of = |p: usize| -> Counts {
        memo.entry(p)
            .or_insert_with(|| {
                let (mut c, mut charge) = (Vec::new(), 0i64);
                for id in &pieces[p] {
                    let i = *index.get(id)?;
                    let a = mol.atoms.get(i)?;
                    if matches!(
                        doc.atoms.get(i).map(|x| label::kind(&x.lab)),
                        Some(Some(label::Kind::R(_)))
                    ) {
                        return None;
                    }
                    bump(&mut c, &a.el, 1);
                    bump(&mut c, "H", i64::from(a.h));
                    charge += i64::from(a.chg);
                }
                (charge == 0).then_some(c)
            })
            .clone()
    };
    for (id, from, to) in arrows {
        let (Some(&pf), Some(&pt)) = (piece_of.get(&from), piece_of.get(&to)) else {
            continue;
        };
        if pf == pt {
            continue;
        }
        let (Some(a), Some(b)) = (counts_of(pf), counts_of(pt)) else {
            out.skipped += 1;
            continue;
        };
        let d = delta_text(&a, &b);
        let text = if d.is_empty() { opts.isomer.clone() } else { d };
        let Some(now) = doc.object(id).map(|o| o.text.as_str()) else {
            continue;
        };
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
