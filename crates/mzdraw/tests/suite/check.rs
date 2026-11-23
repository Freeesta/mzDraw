//! The check of the agent task suite (`tests/data/agent-suite/tasks.json`, `agent-suite-tasks/1`): a task and the final replay request
//! of a model give pass or fail with a reason. Shared by the example `suite_check` and the test `agent_suite` (`#[path]`), so the
//! library carries no suite code. The identity of a piece is the canonical SMILES of `mzlab-mol` (`Mol::can`): atom order and Kekule
//! form do not matter, charge, isotope and stereo do. Formulas are never compared.

use mzdraw::replay::replay;
use mzdraw::{Document, ObjKind};
use mzlab_mol::Mol;
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// The final drawing of a request: the request runs with `out` forced to `mzdraw` and without the verdict fields.
fn final_document(request: &str) -> Result<Document, String> {
    let mut v: Value = serde_json::from_str(request).map_err(|_| "the request is not JSON")?;
    let obj = v.as_object_mut().ok_or("the request is not an object")?;
    for k in ["report", "dry_run", "expect"] {
        obj.remove(k);
    }
    obj.insert("out".into(), "mzdraw".into());
    let text = replay(&v.to_string()).map_err(|e| format!("the request is refused: {}", e.key))?;
    Document::from_json(&text).map_err(|e| format!("the drawing is not read: {e}"))
}

/// The start document of a task: `import` is a text (SMILES, molfile, `mzdraw/1`) or a JSON object.
pub fn import_text(task: &Value) -> Option<String> {
    match task.get("import")? {
        Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

fn canon(smiles: &str) -> Result<String, String> {
    Mol::from_smiles(smiles)
        .map(|m| m.can())
        .map_err(|e| format!("{smiles}: {e:?}"))
}

fn kind(k: ObjKind) -> &'static str {
    match k {
        ObjKind::Arrow => "arrow",
        ObjKind::Plus => "plus",
        ObjKind::Text => "text",
        ObjKind::Curve => "curve",
    }
}

/// A connected piece: its atom ids and its horizontal extent.
struct Piece {
    atoms: BTreeSet<u32>,
    left: f64,
    right: f64,
}

/// The pieces of a drawing, left to right (by the middle of the extent, then by the first atom id).
fn pieces(doc: &Document) -> Vec<Piece> {
    let mut group: Vec<usize> = (0..doc.atoms.len()).collect();
    fn find(g: &mut [usize], i: usize) -> usize {
        let mut r = i;
        while g[r] != r {
            r = g[r];
        }
        let mut i = i;
        while g[i] != r {
            let n = g[i];
            g[i] = r;
            i = n;
        }
        r
    }
    for b in &doc.bonds {
        if let (Some(a), Some(c)) = (doc.atom_index(b.a), doc.atom_index(b.b)) {
            let (ra, rc) = (find(&mut group, a), find(&mut group, c));
            group[ra] = rc;
        }
    }
    let mut roots: Vec<usize> = Vec::new();
    for i in 0..doc.atoms.len() {
        let r = find(&mut group, i);
        if !roots.contains(&r) {
            roots.push(r);
        }
    }
    let mut out: Vec<Piece> = roots
        .iter()
        .map(|&r| {
            let members: Vec<_> = (0..doc.atoms.len())
                .filter(|&i| find(&mut group, i) == r)
                .map(|i| &doc.atoms[i])
                .collect();
            Piece {
                atoms: members.iter().map(|a| a.id).collect(),
                left: members.iter().map(|a| a.x).fold(f64::MAX, f64::min),
                right: members.iter().map(|a| a.x).fold(f64::MIN, f64::max),
            }
        })
        .collect();
    out.sort_by(|a, b| {
        ((a.left + a.right) / 2.0)
            .total_cmp(&((b.left + b.right) / 2.0))
            .then(a.atoms.first().cmp(&b.atoms.first()))
    });
    out
}

/// One piece as the identity of `Mol::can` and as the SMILES the editor writes (for the reason of a failure).
type Identity = (String, String);

/// The identity of one piece: the piece alone, opened as a document, written as SMILES.
fn piece_identity(doc: &Document, p: &Piece) -> Result<Identity, String> {
    let mut one = doc.clone();
    one.atoms.retain(|a| p.atoms.contains(&a.id));
    one.bonds
        .retain(|b| p.atoms.contains(&b.a) && p.atoms.contains(&b.b));
    one.objects.clear();
    let request = json!({"import": one.to_json(), "out": "smiles"});
    let smiles =
        replay(&request.to_string()).map_err(|e| format!("a piece is refused: {}", e.key))?;
    Ok((canon(&smiles)?, smiles))
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    (v.get(key).and_then(Value::as_array))
        .map(|l| {
            l.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn ids(zone: &Value, key: &str) -> BTreeSet<u32> {
    (zone.get(key).and_then(Value::as_array))
        .map(|l| {
            l.iter()
                .filter_map(|n| n.as_u64().map(|n| n as u32))
                .collect()
        })
        .unwrap_or_default()
}

/// What the check of the task asks of the objects; every object of the drawing must be matched by one spec and every spec by one object.
fn check_objects(task_check: &Value, doc: &Document, ps: &[Piece]) -> Result<(), String> {
    let specs: Vec<Value> = (task_check.get("objects").and_then(Value::as_array))
        .cloned()
        .unwrap_or_default();
    if specs.len() != doc.objects.len() {
        let got: Vec<&str> = doc.objects.iter().map(|o| kind(o.t)).collect();
        return Err(format!(
            "objects: expected {} ({}), the drawing has {} ({})",
            specs.len(),
            (specs.iter())
                .map(|s| s["t"].as_str().unwrap_or("?"))
                .collect::<Vec<_>>()
                .join(", "),
            got.len(),
            got.join(", ")
        ));
    }
    let mut used = vec![false; doc.objects.len()];
    for s in &specs {
        let (want, text) = (s["t"].as_str().unwrap_or(""), s["text"].as_str());
        let spot = |i: usize| ps.get(i).map(|p| (p.left, p.right));
        let found = doc.objects.iter().enumerate().find(|(i, o)| {
            let end = if o.t == ObjKind::Arrow { o.x2 } else { o.x };
            let (lo, hi) = (o.x.min(end), o.x.max(end));
            let place = match (
                s.get("between").and_then(Value::as_array),
                s.get("after").and_then(Value::as_u64),
            ) {
                (Some(b), _) => {
                    let (a, c) = (
                        b[0].as_u64().unwrap_or(0) as usize,
                        b[1].as_u64().unwrap_or(0) as usize,
                    );
                    matches!((spot(a), spot(c)), (Some(l), Some(r)) if lo >= l.1 && hi <= r.0)
                }
                (None, Some(a)) => matches!(spot(a as usize), Some(l) if lo >= l.1),
                (None, None) => true,
            };
            !used[*i] && kind(o.t) == want && text.is_none_or(|t| t == o.text) && place
        });
        match found {
            Some((i, _)) => used[i] = true,
            None => {
                return Err(format!(
                    "no {want} {}{} in the drawing",
                    text.map_or(String::new(), |t| format!("with the text {t:?} ")),
                    s.get("between")
                        .map(|b| format!("between pieces {} and {}", b[0], b[1]))
                        .or_else(|| s.get("after").map(|a| format!("right of piece {a}")))
                        .unwrap_or_default()
                ))
            }
        }
    }
    Ok(())
}

/// Everything of the start document outside the edit zone keeps its id, element and position (the whole atom and bond).
fn check_untouched(start: &Document, doc: &Document, zone: &Value) -> Result<(), String> {
    let (za, zb, zo) = (ids(zone, "atoms"), ids(zone, "bonds"), ids(zone, "objects"));
    for a in start.atoms.iter().filter(|a| !za.contains(&a.id)) {
        match doc.atom(a.id) {
            None => {
                return Err(format!(
                    "untouched atom {} ({}) is gone or has a new id",
                    a.id, a.el
                ))
            }
            Some(b) if b.el != a.el => {
                return Err(format!(
                    "untouched atom {} changed its element from {} to {}",
                    a.id, a.el, b.el
                ))
            }
            Some(b) if (b.x, b.y) != (a.x, a.y) => {
                return Err(format!(
                    "untouched atom {} moved from ({}, {}) to ({}, {})",
                    a.id, a.x, a.y, b.x, b.y
                ))
            }
            Some(b) if b != a => {
                return Err(format!(
                    "untouched atom {} changed (charge, isotope, label or data)",
                    a.id
                ))
            }
            Some(_) => {}
        }
    }
    for b in start.bonds.iter().filter(|b| !zb.contains(&b.id)) {
        if doc.bond(b.id) != Some(b) {
            return Err(format!("untouched bond {} is gone or changed", b.id));
        }
    }
    for o in start.objects.iter().filter(|o| !zo.contains(&o.id)) {
        if doc.object(o.id) != Some(o) {
            return Err(format!("untouched object {} is gone or changed", o.id));
        }
    }
    Ok(())
}

/// The `std/1` findings of the drawing (their keys).
fn std_keys(doc: &Document) -> Result<Vec<String>, String> {
    let request = json!({"import": doc.to_json(), "out": "std"});
    let text = replay(&request.to_string()).map_err(|e| format!("std/1 is refused: {}", e.key))?;
    let v: Value = serde_json::from_str(&text).map_err(|_| "std/1 is not JSON")?;
    Ok((v["checks"].as_array().into_iter().flatten())
        .filter_map(|c| c["key"].as_str().map(str::to_string))
        .collect())
}

/// Pass (`Ok`) or fail with the reason.
pub fn check(task: &Value, request: &str) -> Result<(), String> {
    let spec = task.get("check").ok_or("the task has no check")?;
    let doc = final_document(request)?;
    let ps = pieces(&doc);
    let mut got = Vec::new();
    for p in &ps {
        got.push(piece_identity(&doc, p)?);
    }
    let mut want: Vec<Identity> = Vec::new();
    for s in strings(spec, "pieces") {
        want.push((canon(&s)?, s));
    }
    let ordered = spec.get("order").and_then(Value::as_bool).unwrap_or(false);
    let ids = |l: &[Identity]| -> Vec<String> {
        let mut v: Vec<String> = l.iter().map(|i| i.0.clone()).collect();
        if !ordered {
            v.sort();
        }
        v
    };
    if ids(&got) != ids(&want) {
        let texts = |l: &[Identity]| -> Vec<String> { l.iter().map(|i| i.1.clone()).collect() };
        return Err(format!(
            "pieces{}: expected {:?}, got {:?}",
            if ordered { " left to right" } else { "" },
            texts(&want),
            texts(&got)
        ));
    }
    check_objects(spec, &doc, &ps)?;
    if let (Some(zone), Some(text)) = (spec.get("edit_zone"), import_text(task)) {
        let start = Document::from_json(
            &replay(&json!({"import": text, "out": "mzdraw"}).to_string())
                .map_err(|e| format!("the start document is refused: {}", e.key))?,
        )?;
        check_untouched(&start, &doc, zone)?;
    }
    if spec.get("std").and_then(Value::as_str) == Some("none") {
        let keys = std_keys(&doc)?;
        if !keys.is_empty() {
            return Err(format!("std/1 reports {}", keys.join(", ")));
        }
    }
    let absent = strings(spec, "std_absent");
    if !absent.is_empty() {
        let keys = std_keys(&doc)?;
        if let Some(k) = keys.iter().find(|k| absent.contains(k)) {
            return Err(format!("std/1 reports {k}"));
        }
    }
    Ok(())
}

/// The line `suite_check` prints: fixed key order, so two runs are the same bytes.
pub fn report(task_id: &str, result: &Result<(), String>) -> String {
    let (pass, reason) = match result {
        Ok(()) => (true, String::new()),
        Err(r) => (false, r.clone()),
    };
    format!(
        "{{\"task\":{},\"pass\":{pass},\"reason\":{}}}",
        Value::from(task_id),
        Value::from(reason)
    )
}
