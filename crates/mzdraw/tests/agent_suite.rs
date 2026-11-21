//! The agent task suite (`tests/data/agent-suite`): 30 fixed tasks, one hand-written reference solution each, and the checker that
//! the examples and the runner use. No Python and no model: the checker passes every reference and fails a broken copy of each.

#[path = "suite/check.rs"]
mod check;

use serde_json::{json, Value};
use std::path::PathBuf;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/agent-suite")
}

fn tasks() -> Vec<Value> {
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(dir().join("tasks.json")).unwrap()).unwrap();
    assert_eq!(v["format"], "agent-suite-tasks/1");
    v["tasks"].as_array().unwrap().clone()
}

fn reference(task: &Value) -> Value {
    let path = dir()
        .join("ref")
        .join(format!("{}.json", task["id"].as_str().unwrap()));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The final drawing of a request as `mzdraw/1` text.
fn drawing(request: &Value) -> mzdraw::Document {
    let mut r = request.clone();
    r["out"] = "mzdraw".into();
    mzdraw::Document::from_json(&mzdraw::replay::replay(&r.to_string()).unwrap()).unwrap()
}

/// The request with one more op at its end.
fn with_op(request: &Value, op: Value) -> Value {
    let mut r = request.clone();
    let mut log = r["log"].as_array().cloned().unwrap_or_default();
    log.push(op);
    r["log"] = Value::Array(log);
    r
}

#[test]
fn the_suite_has_thirty_tasks_ten_of_each_kind_with_unique_ids_and_a_reference_each() {
    let ts = tasks();
    assert_eq!(ts.len(), 30);
    let mut ids = std::collections::BTreeSet::new();
    for kind in ["build", "edit", "several"] {
        assert_eq!(
            ts.iter().filter(|t| t["kind"] == kind).count(),
            10,
            "{kind}"
        );
    }
    for t in &ts {
        assert!(ids.insert(t["id"].as_str().unwrap().to_string()));
        assert!(t["words"].as_str().is_some_and(|w| !w.is_empty()));
        assert!(t["check"]["pieces"]
            .as_array()
            .is_some_and(|p| !p.is_empty()));
        assert_eq!(
            t["kind"] == "edit",
            t.get("import").is_some(),
            "{}",
            t["id"]
        );
        assert_eq!(t["kind"] == "edit", t["check"].get("edit_zone").is_some());
        assert!(dir()
            .join("ref")
            .join(format!("{}.json", t["id"].as_str().unwrap()))
            .exists());
    }
    let files = std::fs::read_dir(dir().join("ref")).unwrap().count();
    assert_eq!(files, 30, "a reference without a task");
}

#[test]
fn every_reference_solution_passes() {
    for t in tasks() {
        let r = check::check(&t, &reference(&t).to_string());
        assert_eq!(r, Ok(()), "{}", t["id"]);
    }
}

#[test]
fn a_broken_copy_of_each_reference_fails_with_a_reason() {
    for t in tasks() {
        let good = reference(&t);
        let first = drawing(&good).atoms[0].id;
        // one atom changed: its element becomes silicon
        let broken = with_op(&good, json!({"op":"atom.set","id":first,"el":"Si"}));
        let r = check::check(&t, &broken.to_string());
        let why = r.expect_err(&format!("{}: the broken copy passed", t["id"]));
        assert!(!why.is_empty());
    }
}

/// Two atoms of different elements swapped: the formula is the same, the structure is not (unless the swap is a symmetry, which is skipped).
#[test]
fn a_swap_that_keeps_the_formula_still_fails() {
    let mut tried = 0;
    for t in tasks() {
        let good = reference(&t);
        let doc = drawing(&good);
        let swap = doc.atoms.iter().enumerate().find_map(|(i, a)| {
            (doc.atoms[i + 1..].iter())
                .find(|b| b.el != a.el && b.el != "H" && a.el != "H")
                .map(|b| (a.id, a.el.clone(), b.id, b.el.clone()))
        });
        let Some((ia, ea, ib, eb)) = swap else {
            continue;
        };
        let mut broken = with_op(&good, json!({"op":"atom.set","id":ia,"el":eb}));
        broken = with_op(&broken, json!({"op":"atom.set","id":ib,"el":ea}));
        // a swap that is a symmetry of the target draws the target again and passes: it is not counted
        if check::check(&t, &broken.to_string()).is_err() {
            tried += 1;
        }
    }
    assert!(tried >= 20, "only {tried} swaps were told apart");
}

#[test]
fn a_surgical_edit_fails_when_an_untouched_atom_moves_or_the_drawing_is_rebuilt() {
    for t in tasks().iter().filter(|t| t["kind"] == "edit") {
        let good = reference(t);
        let zone: Vec<u64> = t["check"]["edit_zone"]["atoms"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_u64)
            .collect();
        let start = drawing(&json!({"import": check::import_text(t).unwrap()}));
        let untouched = start
            .atoms
            .iter()
            .find(|a| !zone.contains(&u64::from(a.id)))
            .unwrap()
            .id;
        let moved = with_op(
            &good,
            json!({"op":"move","atoms":[untouched],"dx":0.1,"dy":0.0}),
        );
        let why = check::check(t, &moved.to_string()).expect_err("a moved atom passed");
        assert!(
            why.contains("untouched atom") && why.contains("moved"),
            "{}: {why}",
            t["id"]
        );
        // the right structure, drawn again from its SMILES: ids and positions are not those of the start
        let target = t["check"]["pieces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect::<Vec<_>>()
            .join(".");
        let rebuilt = json!({"import": target, "out": "smiles"});
        let r = check::check(t, &rebuilt.to_string());
        // a rebuild that is the very drawing of the reference (same ids, same positions) is the edit itself
        let same = drawing(&rebuilt) == drawing(&good);
        assert!(r.is_err() || same, "{}: a rebuilt drawing passed", t["id"]);
    }
}

#[test]
fn the_report_line_is_the_same_bytes_every_time_and_refusals_are_failures() {
    let t = &tasks()[0];
    let good = reference(t).to_string();
    let a = check::report("x", &check::check(t, &good));
    assert_eq!(a, check::report("x", &check::check(t, &good)));
    assert_eq!(a, r#"{"task":"x","pass":true,"reason":""}"#);
    let bad = check::report(
        "x",
        &check::check(t, r#"{"log":[{"op":"atom.set","id":9,"el":"N"}]}"#),
    );
    assert!(
        bad.starts_with(r#"{"task":"x","pass":false,"reason":"the request is refused: "#),
        "{bad}"
    );
    assert!(check::check(t, "not json").is_err());
}
