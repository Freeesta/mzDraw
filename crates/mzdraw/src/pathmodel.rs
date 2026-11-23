//! The declarative pathway model `pathway/1` (pathway-draw): a list of nodes and edges that becomes the drawing in one undo step.
//!
//! ```json
//! {"format":"pathway/1",
//!  "nodes":[{"id":"P","smiles":"c1ccccc1C","label":"Parent","ion":"[M+H]+","mz":93.07,"rt":4.2,
//!            "status":"observed","generation":0,"branch":"A","formula":"C7H8"}],
//!  "edges":[{"from":"P","to":"M1","label":"+O"}]}
//! ```
//!
//! The model is no new drawing code: the nodes become the rows of a TP list (`tp_list`: structures, anchored arrows, orientation,
//! the tidy-tree layout of `pathway:tree`, formula deltas on the arrows) and a few plain operations are added on the same copy of the
//! drawing: the data fields of each structure and the text of the arrows that carry a label of the model. No `draw_edit/1` operation or
//! field is new, so a log of the result replays to the same bytes natively and in the `.wasm`.
//!
//! **Data kept on a structure** (after the reserved fields of `tp_list::FIELDS`): `NODE_ID` (the `id` of the model), `FORMULA` (the
//! formula the model states), `STATUS` (`observed`, `tentative`, `not_detected`; `observed` is the default and is not written) and
//! `STATUS_TEXT` (the words shown next to the name, in the language of the caller; English when the caller gave none),
//! `GENERATION`, `BRANCH`. The shown name `TP_ID` is the `label` (else the `id`); the checks (`pathcheck`) read the drawing, not the
//! model, so they work on any file that carries these fields.
//!
//! **Atomic.** Every problem of the model is listed, each with its key; a line is the 1-based place in `nodes`, then in `edges`
//! (the edge `j` has the line `nodes + j`). The problems of the structures themselves (a SMILES that does not read, an adduct that is
//! not of the form `[M+H]+`) are the rows keys of `tp:import`, on the line of the node.

use crate::doc::Document;
use crate::ops::Op;
use crate::settings::Style;
use crate::tp_list::Failure;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

/// The value of the key `format`.
pub const FORMAT: &str = "pathway/1";

/// At most this many nodes and edges in one model (the layout refuses above 1000 linked structures; the nodes are structures).
pub const MAX_NODES: usize = 500;
pub const MAX_EDGES: usize = 1000;

/// The statuses of a node.
pub const STATUSES: [&str; 3] = ["observed", "tentative", "not_detected"];

/// The key of a model refused as a whole (shape, format, size) and of a model with problems listed in `failed.rows`.
pub const KEY_MODEL: &str = "error.draw2.pathModel";
pub const KEY_ROWS: &str = "error.draw2.pathRows";

const NODE_SHAPE: &str = "error.draw2.pathNodeShape";
const NODE_ID: &str = "error.draw2.pathNodeId";
const NODE_DUP: &str = "error.draw2.pathNodeDup";
const NODE_SMILES: &str = "error.draw2.pathNodeSmiles";
const NODE_FORMULA: &str = "error.draw2.pathNodeFormula";
const NODE_STATUS: &str = "error.draw2.pathNodeStatus";
const NODE_NUMBER: &str = "error.draw2.pathNodeNumber";
const EDGE_SHAPE: &str = "error.draw2.pathEdgeShape";
const EDGE_END: &str = "error.draw2.pathEdgeEnd";
const EDGE_LOOP: &str = "error.draw2.pathEdgeLoop";
const EDGE_DUP: &str = "error.draw2.pathEdgeDup";

/// One node of a model.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Node {
    pub id: String,
    pub smiles: String,
    pub formula: String,
    pub label: String,
    pub ion: String,
    pub mz: String,
    pub rt: String,
    pub status: String,
    pub generation: Option<u32>,
    pub branch: String,
}

/// One edge of a model.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: String,
}

/// A model that passed its own checks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

/// Whether a text is a pathway model: a JSON object that names its format (the page takes it in its Add field and its file button, as it
/// takes a TP list). A cheap look first, so that a SMILES or a molfile is never parsed as JSON.
pub fn looks_like(text: &str) -> bool {
    let t = text.trim_start();
    t.starts_with('{')
        && t.contains("\"pathway/1\"")
        && serde_json::from_str::<Value>(t)
            .ok()
            .is_some_and(|v| v.get("format").and_then(Value::as_str) == Some(FORMAT))
}

/// The element counts of a formula text such as `C6H8O6`, with an optional trailing charge (`C2H3O2-`, ignored); `None` for any text
/// that is not elements and counts. The reader is the delta reader of mzLab (`mzlab_mol::delta`), so a formula and a label are read
/// by one code.
pub fn formula_counts(text: &str) -> Option<Vec<(String, i64)>> {
    let body = text.trim().trim_end_matches(['+', '-']);
    if body.is_empty() || !body.starts_with(|c: char| c.is_ascii_uppercase()) {
        return None;
    }
    let mut c = mzlab_mol::delta::parse(&format!("+{body}")).ok()?;
    c.retain(|e| e.1 != 0);
    c.sort();
    Some(c)
}

fn text_of(v: &Value, max: usize) -> Option<String> {
    let s = v.as_str()?;
    (s.chars().count() <= max && !s.chars().any(char::is_control)).then(|| s.to_string())
}

/// A number given as a JSON number or as a text that reads as one, kept as written (the list keeps what was typed).
fn number_of(v: &Value) -> Option<String> {
    match v {
        // a number is written the same whatever wrote the JSON (`4.0` and `4` are one number: JavaScript drops the `.0`)
        Value::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => Some(i.to_string()),
            (None, Some(x)) if x.is_finite() => Some(format!("{x}")),
            _ => None,
        },
        Value::String(s) if s.trim().parse::<f64>().is_ok_and(f64::is_finite) => {
            Some(s.trim().to_string())
        }
        _ => None,
    }
}

/// Reads and checks a model. The failure is a key alone (the model as a whole) or the list of every problem.
pub fn parse(v: &Value) -> Result<Model, Failure> {
    let top = v.as_object().ok_or(Failure::Key(KEY_MODEL, 0))?;
    if top.get("format").and_then(Value::as_str) != Some(FORMAT)
        || top
            .keys()
            .any(|k| !matches!(k.as_str(), "format" | "nodes" | "edges"))
    {
        return Err(Failure::Key(KEY_MODEL, 0));
    }
    let nodes = top.get("nodes").and_then(Value::as_array);
    let empty = Vec::new();
    let edges = match top.get("edges") {
        None => &empty,
        Some(e) => e.as_array().ok_or(Failure::Key(KEY_MODEL, 0))?,
    };
    let nodes = nodes.ok_or(Failure::Key(KEY_MODEL, 0))?;
    if nodes.is_empty() || nodes.len() > MAX_NODES || edges.len() > MAX_EDGES {
        return Err(Failure::Key(KEY_MODEL, 0));
    }
    let mut bad: BTreeMap<u32, &'static str> = BTreeMap::new();
    let mut model = Model::default();
    let mut names: HashSet<String> = HashSet::new();
    for (i, item) in nodes.iter().enumerate() {
        let line = i as u32 + 1;
        let mut n = Node::default();
        let Some(o) = item.as_object() else {
            bad.insert(line, NODE_SHAPE);
            model.nodes.push(n);
            continue;
        };
        let mut key = None;
        for (k, val) in o {
            let r = match k.as_str() {
                "id" => text_of(val, 64)
                    .filter(|s| !s.is_empty() && s.trim() == s && !s.contains(';'))
                    .map(|s| n.id = s)
                    .ok_or(NODE_ID),
                "smiles" => text_of(val, crate::tp_list::MAX_SMILES)
                    .map(|s| n.smiles = s.trim().to_string())
                    .ok_or(NODE_SMILES),
                "formula" => text_of(val, 64)
                    .filter(|s| formula_counts(s).is_some())
                    .map(|s| n.formula = s.trim().to_string())
                    .ok_or(NODE_FORMULA),
                "label" => text_of(val, 64)
                    .filter(|s| s.trim() == s && !s.contains(';'))
                    .map(|s| n.label = s)
                    .ok_or(NODE_SHAPE),
                "ion" => text_of(val, 32)
                    .map(|s| n.ion = s.trim().to_string())
                    .ok_or(NODE_SHAPE),
                "mz" => number_of(val).map(|s| n.mz = s).ok_or(NODE_NUMBER),
                "rt" => number_of(val).map(|s| n.rt = s).ok_or(NODE_NUMBER),
                "status" => text_of(val, 16)
                    .filter(|s| STATUSES.contains(&s.as_str()))
                    .map(|s| n.status = s)
                    .ok_or(NODE_STATUS),
                "generation" => val
                    .as_u64()
                    .and_then(|g| u32::try_from(g).ok())
                    .filter(|&g| g <= 1000)
                    .map(|g| n.generation = Some(g))
                    .ok_or(NODE_NUMBER),
                "branch" => text_of(val, 64)
                    .filter(|s| !s.is_empty())
                    .map(|s| n.branch = s)
                    .ok_or(NODE_SHAPE),
                _ => Err(NODE_SHAPE),
            };
            if let Err(e) = r {
                key.get_or_insert(e);
            }
        }
        if n.id.is_empty() && key.is_none() {
            key = Some(NODE_ID);
        }
        if key.is_none() && n.smiles.is_empty() {
            // a node with a formula alone has no structure to draw (a follow-up): said, never silently dropped
            key = Some(NODE_SMILES);
        }
        if key.is_none() {
            let name = if n.label.is_empty() { &n.id } else { &n.label };
            if !names.insert(name.clone()) {
                key = Some(NODE_DUP);
            }
        }
        if let Some(k) = key {
            bad.insert(line, k);
        }
        model.nodes.push(n);
    }
    // an id and a shown name that two nodes share would make a link ambiguous
    let mut ids: HashMap<&str, usize> = HashMap::new();
    for (i, n) in model.nodes.iter().enumerate() {
        if !n.id.is_empty() && ids.insert(&n.id, i).is_some() {
            bad.entry(i as u32 + 1).or_insert(NODE_DUP);
        }
    }
    let base = nodes.len() as u32;
    let mut pairs: HashSet<(String, String)> = HashSet::new();
    for (j, item) in edges.iter().enumerate() {
        let line = base + j as u32 + 1;
        let mut e = Edge::default();
        let Some(o) = item.as_object() else {
            bad.insert(line, EDGE_SHAPE);
            continue;
        };
        let mut key = None;
        for (k, val) in o {
            let r = match k.as_str() {
                "from" => text_of(val, 64).map(|s| e.from = s).ok_or(EDGE_SHAPE),
                "to" => text_of(val, 64).map(|s| e.to = s).ok_or(EDGE_SHAPE),
                "label" => text_of(val, 64)
                    .filter(|s| s.trim() == s)
                    .map(|s| e.label = s)
                    .ok_or(EDGE_SHAPE),
                _ => Err(EDGE_SHAPE),
            };
            if let Err(k) = r {
                key.get_or_insert(k);
            }
        }
        if key.is_none() && (e.from.is_empty() || e.to.is_empty()) {
            key = Some(EDGE_SHAPE);
        }
        if key.is_none() && (!ids.contains_key(e.from.as_str()) || !ids.contains_key(e.to.as_str()))
        {
            key = Some(EDGE_END);
        }
        if key.is_none() && e.from == e.to {
            key = Some(EDGE_LOOP);
        }
        if key.is_none() && !pairs.insert((e.from.clone(), e.to.clone())) {
            key = Some(EDGE_DUP);
        }
        if let Some(k) = key {
            bad.insert(line, k);
        }
        model.edges.push(e);
    }
    if !bad.is_empty() {
        return Err(Failure::Rows(
            bad.into_iter().map(|(l, k)| (l, k.to_string())).collect(),
        ));
    }
    Ok(model)
}

/// The operations of `pathway:build`: the list of `tp:import` made from the model, then the data of each structure and the labels the
/// model gives to its arrows, all on one copy of the drawing so that the caller runs them as one undo step.
pub fn build_ops(
    doc: &Document,
    style: &Style,
    v: &Value,
    words: Option<&(String, String)>,
) -> Result<Vec<Op>, Failure> {
    let model = parse(v)?;
    let rows: Vec<Value> = model
        .nodes
        .iter()
        .map(|n| {
            let parents: Vec<&str> = (model.edges.iter())
                .filter(|e| e.to == n.id)
                .map(|e| e.from.as_str())
                .collect();
            let mut r = serde_json::Map::new();
            r.insert("smiles".into(), json!(n.smiles));
            r.insert("name".into(), json!(n.id));
            r.insert("parent".into(), json!(parents));
            for (k, val) in [("mz", &n.mz), ("rt", &n.rt), ("adduct", &n.ion)] {
                if !val.is_empty() {
                    r.insert(k.into(), json!(val));
                }
            }
            Value::Object(r)
        })
        .collect();
    let mut ops = crate::tp_list::import_ops(
        doc,
        style,
        &crate::tp_list::Input::Rows(&Value::Array(rows)),
    )?;
    let key = |e: crate::ops::OpError| Failure::Key(e.key, e.id);
    let mut scratch = doc.clone();
    scratch.apply(&ops).map_err(key)?;
    // the new structures: the smallest atom of each, found by the name the list gave it (`TP_ID` = the id)
    let mut anchor: HashMap<String, u32> = HashMap::new();
    for a in scratch.atoms.iter().filter(|a| a.id >= doc.next) {
        if let Some((_, v)) = a.data().into_iter().find(|(k, _)| k == "TP_ID") {
            anchor.entry(v).or_insert(a.id);
        }
    }
    for n in &model.nodes {
        let Some(&at) = anchor.get(&n.id) else {
            continue;
        };
        let Some(atom) = scratch.atom(at) else {
            continue;
        };
        let mut data = atom.data();
        data.retain(|(k, _)| k != "TP_ID");
        let shown = if n.label.is_empty() { &n.id } else { &n.label };
        let mut head = vec![("TP_ID".to_string(), shown.clone())];
        head.append(&mut data);
        let mut extra: Vec<(&str, String)> = vec![("NODE_ID", n.id.clone())];
        if !n.formula.is_empty() {
            extra.push(("FORMULA", n.formula.clone()));
        }
        if !n.status.is_empty() && n.status != "observed" {
            extra.push(("STATUS", n.status.clone()));
            // the words the drawing shows, in the language of the caller; without them the painter's English
            if let Some((tentative, absent)) = words {
                let w = if n.status == "tentative" {
                    tentative
                } else {
                    absent
                };
                extra.push(("STATUS_TEXT", w.clone()));
            }
        }
        if let Some(g) = n.generation {
            extra.push(("GENERATION", g.to_string()));
        }
        if !n.branch.is_empty() {
            extra.push(("BRANCH", n.branch.clone()));
        }
        head.extend(extra.into_iter().map(|(k, v)| (k.to_string(), v)));
        ops.push(Op::AtomData { id: at, data: head });
    }
    for e in model.edges.iter().filter(|e| !e.label.is_empty()) {
        let (Some(&f), Some(&t)) = (anchor.get(&e.from), anchor.get(&e.to)) else {
            continue;
        };
        if let Some(o) = scratch.objects.iter().find(|o| o.from == f && o.to == t) {
            ops.push(Op::ObjSet {
                id: o.id,
                text: Some(e.label.clone()),
                runs: None,
                x2: None,
                y2: None,
                from: None,
                to: None,
                cv: Default::default(),
                pt: [None, None],
            });
        }
    }
    // the layout ran before the labels of the model were set: run it again on the drawing as it now is, so that the room for the text of
    // each arrow is the room for the text it really has (the rule `label` of tp-05), and the arrows are routed to the moved structures
    if model.edges.iter().any(|e| !e.label.is_empty()) {
        let mut again = doc.clone();
        again.apply(&ops).map_err(key)?;
        let tree =
            crate::pathway::command(&again, style).map_err(|r| Failure::Key(r.key(), r.id()))?;
        ops.extend(tree);
    }
    Ok(ops)
}
