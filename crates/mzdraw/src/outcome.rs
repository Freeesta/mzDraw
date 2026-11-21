//! The answer of a replay request that asks for a verdict: `draw-result/1` (docs/mzdraw.md, section "Replay result").
//!
//! A request with `"report":true`, `"dry_run":true` or a non-empty `"expect"` gets one `draw-result/1` document instead of the bare
//! output: whether every op ran and every postcondition holds, the ids the run created and removed, the op that failed, and the
//! `out` text only when the run is good and not a dry run. The document is written by hand, keys in the order of `KEYS`, so the
//! same request gives the same bytes on every surface; `docs/schemas/draw-result-1.json` describes it.

use crate::doc::Document;
use serde_json::Value;
use std::collections::BTreeSet;

/// The keys of the envelope, in the order they are written. A key that does not apply is left out.
pub const KEYS: [&str; 13] = [
    "format", "id", "ok", "dry_run", "patch", "out", "created", "removed", "lost", "failed",
    "changed", "orient", "expect",
];

/// The keys of an item of `orient`, in the order they are written (`at` only for status `overlap`).
pub const ORIENT_KEYS: [&str; 10] = [
    "arrow", "from", "to", "status", "core", "flip", "deg", "before", "after", "at",
];

/// The keys of `failed`, in the order they are written.
pub const FAILED_KEYS: [&str; 6] = ["stage", "op", "key", "id", "candidates", "rows"];

/// The format field.
pub const FORMAT: &str = "draw-result/1";

/// The ids of a document, per kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IdSets {
    pub atoms: BTreeSet<u32>,
    pub bonds: BTreeSet<u32>,
    pub objects: BTreeSet<u32>,
}

impl IdSets {
    pub fn of(doc: &Document) -> IdSets {
        IdSets {
            atoms: doc.atoms.iter().map(|a| a.id).collect(),
            bonds: doc.bonds.iter().map(|b| b.id).collect(),
            objects: doc.objects.iter().map(|o| o.id).collect(),
        }
    }

    /// The ids of `self` that `other` does not have, per kind.
    pub fn minus(&self, other: &IdSets) -> IdSets {
        let diff = |a: &BTreeSet<u32>, b: &BTreeSet<u32>| a.difference(b).copied().collect();
        IdSets {
            atoms: diff(&self.atoms, &other.atoms),
            bonds: diff(&self.bonds, &other.bonds),
            objects: diff(&self.objects, &other.objects),
        }
    }

    /// `{"atoms":[..],"bonds":[..],"objects":[..]}`, ascending, an empty list left out (`{}` when there is none).
    fn json(&self) -> String {
        let parts: Vec<String> = [
            ("atoms", &self.atoms),
            ("bonds", &self.bonds),
            ("objects", &self.objects),
        ]
        .iter()
        .filter(|(_, s)| !s.is_empty())
        .map(|(k, s)| {
            let ids: Vec<String> = s.iter().map(u32::to_string).collect();
            format!("\"{k}\":[{}]", ids.join(","))
        })
        .collect();
        format!("{{{}}}", parts.join(","))
    }
}

/// One postcondition of the request's `expect`.
#[derive(Clone, Debug, PartialEq)]
pub enum Check {
    /// `{"std":"none"}`: the drawing has no `std/1` finding.
    StdNone,
    /// `{"std_absent":[keys]}`: none of these `std/1` keys is found.
    StdAbsent(Vec<String>),
    /// `{"pieces":N}`: the atoms form N connected pieces.
    Pieces(u32),
    /// `{"smiles":"..."}`: the SMILES of the whole drawing (`Editor::save("smiles")`) is this text.
    Smiles(String),
    /// `{"lossless":"FORMAT"}`: the format drops and changes nothing of the drawing (`loss::loss` is empty).
    Lossless(String),
}

/// A postcondition with the request object it came from (written back as `check`).
#[derive(Clone, Debug, PartialEq)]
pub struct Expectation {
    pub request: Value,
    pub check: Check,
}

const BAD_EXPECT: &str = "error.draw2.expect";
const LOSS_SIZE: &str = "error.draw2.lossSize";

/// Reads the value of `expect`: a list of objects of one key each. An unknown arm or a value of the wrong kind is refused.
pub fn read_expect(v: &Value) -> Result<Vec<Expectation>, &'static str> {
    let list = v.as_array().ok_or("error.draw2.request")?;
    let mut out = Vec::new();
    for item in list {
        let obj = item
            .as_object()
            .filter(|o| o.len() == 1)
            .ok_or(BAD_EXPECT)?;
        let (arm, val) = obj.iter().next().ok_or(BAD_EXPECT)?;
        let check = match arm.as_str() {
            "std" if val.as_str() == Some("none") => Check::StdNone,
            "std_absent" => Check::StdAbsent(
                (val.as_array().ok_or(BAD_EXPECT)?.iter())
                    .map(|k| k.as_str().map(str::to_string))
                    .collect::<Option<Vec<String>>>()
                    .ok_or(BAD_EXPECT)?,
            ),
            "pieces" => Check::Pieces(
                val.as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or(BAD_EXPECT)?,
            ),
            "smiles" => Check::Smiles(val.as_str().ok_or(BAD_EXPECT)?.to_string()),
            "lossless" => Check::Lossless(
                val.as_str()
                    .filter(|f| crate::loss::FORMATS.contains(f))
                    .ok_or(BAD_EXPECT)?
                    .to_string(),
            ),
            _ => return Err(BAD_EXPECT),
        };
        out.push(Expectation {
            request: item.clone(),
            check,
        });
    }
    Ok(out)
}

/// The number of connected pieces of the atoms of a drawing (an atom alone is a piece).
pub fn pieces(doc: &Document) -> u32 {
    let mut parent: Vec<usize> = (0..doc.atoms.len()).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for b in &doc.bonds {
        if let (Some(i), Some(j)) = (doc.atom_index(b.a), doc.atom_index(b.b)) {
            let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
            parent[ri] = rj;
        }
    }
    (0..parent.len())
        .filter(|&i| root(&mut parent, i) == i)
        .count() as u32
}

impl Check {
    /// Whether the drawing satisfies it, and when it does not, why (English text for a reader; empty when it holds).
    pub fn run(&self, doc: &Document, style: &crate::settings::Style) -> (bool, String) {
        match self {
            Check::StdNone => {
                let keys = found(doc, style, None);
                if keys.is_empty() {
                    (true, String::new())
                } else {
                    (false, format!("std/1 reports {}", keys.join(", ")))
                }
            }
            Check::StdAbsent(wanted) => {
                let keys = found(doc, style, Some(wanted));
                if keys.is_empty() {
                    (true, String::new())
                } else {
                    (false, format!("std/1 reports {}", keys.join(", ")))
                }
            }
            Check::Pieces(n) => {
                let got = pieces(doc);
                if got == *n {
                    (true, String::new())
                } else {
                    (false, format!("the drawing has {got} pieces"))
                }
            }
            Check::Smiles(want) => match doc.to_smiles() {
                Ok(got) if got == *want => (true, String::new()),
                Ok(got) => (false, format!("the SMILES of the drawing is {got}")),
                Err(_) => (false, "the drawing has no SMILES".to_string()),
            },
            Check::Lossless(format) => match crate::loss::loss(doc, format) {
                Ok(items) if items.is_empty() => (true, String::new()),
                Ok(items) => {
                    let mut kinds: Vec<&str> = items.iter().map(|l| l.what).collect();
                    kinds.dedup();
                    (false, format!("{format} loses {}", kinds.join(", ")))
                }
                Err("error.draw2.lossSize") => (false, format!("too large to check ({LOSS_SIZE})")),
                Err(key) => (false, format!("the drawing has no {format} file ({key})")),
            },
        }
    }
}

/// The distinct `std/1` keys the drawing is reported for (all of them, or only those in `only`), in report order.
fn found(
    doc: &Document,
    style: &crate::settings::Style,
    only: Option<&Vec<String>>,
) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for f in &crate::pathway::report_with(doc, style).checks {
        let wanted = only.is_none_or(|l| l.iter().any(|k| k == f.key));
        if wanted && !keys.iter().any(|k| k == f.key) {
            keys.push(f.key.to_string());
        }
    }
    keys
}

/// What stopped the run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failed {
    /// `log` (this slice); later stages are `select`, `plan`, `grant` and `command`.
    pub stage: &'static str,
    /// The 0-based index in `log` (stage `log`) or in `commands` (stage `command`); `None` for the other stages.
    pub op: Option<usize>,
    /// The i18n key of the refusal.
    pub key: &'static str,
    /// The id it is about, 0 when none.
    pub id: u32,
    /// Stage `select`: the atom ids each match covered.
    pub candidates: Vec<Vec<u32>>,
    /// Stage `command` on a command that reads rows: `(line, key)` of every failing row, in line order.
    pub rows: Vec<(u32, String)>,
}

impl Failed {
    /// A refused op of the log.
    pub fn log(op: usize, key: &'static str, id: u32) -> Failed {
        Failed {
            stage: "log",
            op: Some(op),
            key,
            id,
            candidates: Vec::new(),
            rows: Vec::new(),
        }
    }

    /// A refused editor command of the request, by its index in `commands`.
    pub fn command(op: usize, key: &'static str, id: u32) -> Failed {
        Failed {
            stage: "command",
            ..Failed::log(op, key, id)
        }
    }

    fn json(&self) -> String {
        let mut s = format!("{{\"stage\":{}", Value::from(self.stage));
        if let Some(op) = self.op {
            s.push_str(&format!(",\"op\":{op}"));
        }
        s.push_str(&format!(
            ",\"key\":{},\"id\":{}",
            Value::from(self.key),
            self.id
        ));
        if !self.candidates.is_empty() {
            let lists: Vec<String> = (self.candidates.iter())
                .map(|l| {
                    let ids: Vec<String> = l.iter().map(u32::to_string).collect();
                    format!("[{}]", ids.join(","))
                })
                .collect();
            s.push_str(&format!(",\"candidates\":[{}]", lists.join(",")));
        }
        if !self.rows.is_empty() {
            let rows: Vec<String> = (self.rows.iter())
                .map(|(line, key)| {
                    format!("{{\"line\":{line},\"key\":{}}}", Value::from(key.as_str()))
                })
                .collect();
            s.push_str(&format!(",\"rows\":[{}]", rows.join(",")));
        }
        s.push('}');
        s
    }
}

/// The verdict on a run, ready to be written.
pub struct Outcome {
    /// The request's own `id` as a JSON text, or the index of the request in its batch.
    pub id: String,
    pub dry_run: bool,
    /// What `Editor::save(out)` wrote: only for a good run that is not a dry run.
    pub out: Option<String>,
    pub created: IdSets,
    pub removed: IdSets,
    /// What the output format `out` cannot hold, as (i18n key, count): `draw2.export.mapsLost` for atom maps. Empty: nothing lost.
    pub lost: Vec<(&'static str, usize)>,
    pub failed: Option<Failed>,
    /// Only when a command of the request reports whether it changed the drawing (`commands::reports_change`): whether the commands
    /// changed it.
    pub changed: Option<bool>,
    /// Only when a command was `scheme:orient`: the result of each product structure of its last run (`orient::Step`).
    pub orient: Option<Vec<crate::orient::Step>>,
    /// `(request object, ok, detail)` per expectation, in request order; `None` when the request had no `expect`.
    pub expect: Option<Vec<(Value, bool, String)>>,
}

impl Outcome {
    /// Every op ran and every expectation holds.
    pub fn ok(&self) -> bool {
        self.failed.is_none() && self.expect.iter().flatten().all(|(_, ok, _)| *ok)
    }

    /// The `draw-result/1` document, keys in the order of `KEYS`.
    pub fn json(&self) -> String {
        let mut s = format!(
            "{{\"format\":\"{FORMAT}\",\"id\":{},\"ok\":{}",
            self.id,
            self.ok()
        );
        if self.dry_run {
            s.push_str(",\"dry_run\":true");
        }
        if let Some(out) = &self.out {
            s.push_str(&format!(",\"out\":{}", Value::from(out.as_str())));
        }
        s.push_str(&format!(
            ",\"created\":{},\"removed\":{}",
            self.created.json(),
            self.removed.json()
        ));
        if !self.lost.is_empty() {
            let items: Vec<String> = (self.lost.iter())
                .map(|(key, n)| format!("{{\"key\":{},\"n\":{n}}}", Value::from(*key)))
                .collect();
            s.push_str(&format!(",\"lost\":[{}]", items.join(",")));
        }
        if let Some(f) = &self.failed {
            s.push_str(&format!(",\"failed\":{}", f.json()));
        }
        if let Some(c) = self.changed {
            s.push_str(&format!(",\"changed\":{c}"));
        }
        if let Some(steps) = &self.orient {
            let items: Vec<String> = steps.iter().map(orient_json).collect();
            s.push_str(&format!(",\"orient\":[{}]", items.join(",")));
        }
        if let Some(list) = &self.expect {
            let items: Vec<String> = (list.iter())
                .map(|(check, ok, detail)| {
                    format!(
                        "{{\"check\":{check},\"ok\":{ok},\"detail\":{}}}",
                        Value::from(detail.as_str())
                    )
                })
                .collect();
            s.push_str(&format!(",\"expect\":[{}]", items.join(",")));
        }
        s.push('}');
        s
    }
}

/// One item of `orient`: keys in the order of `ORIENT_KEYS`; numbers are the shortest text of the value, as everywhere else.
fn orient_json(s: &crate::orient::Step) -> String {
    let n = |v: f64| Value::from(crate::doc::round4(v)).to_string();
    let mut out = format!(
        "{{\"arrow\":{},\"from\":{},\"to\":{},\"status\":\"{}\",\"core\":{},\"flip\":{},\"deg\":{},\"before\":{},\"after\":{}",
        s.arrow,
        s.from,
        s.to,
        s.status.name(),
        s.core,
        s.flip,
        n(s.deg),
        n(s.before),
        n(s.after)
    );
    if s.status == crate::orient::Status::Overlap {
        out.push_str(&format!(",\"at\":{}", s.at));
    }
    out.push('}');
    out
}
