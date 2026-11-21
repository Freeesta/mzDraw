//! The fixed task set of the velocity benchmark (`tests/data/velocity`, `tests/velocity.mjs`): every target is a SMILES `mzlab-mol` reads,
//! every task has a path for the page and a `draw_edit/1` patch, and every patch replays natively to its target (the CLI twin of the benchmark).

use mzlab_mol::Mol;
use serde_json::Value;

fn read(name: &str) -> Value {
    let path = format!(
        "{}/../../tests/data/velocity/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The objects of a document as sorted "kind|text" strings, as `tests/velocity.mjs` writes them: an anchored arrow adds "|i>j", the
/// creation-order index of the structure (rank of its smallest atom id among the structures) at its tail and at its head.
fn objects_of(doc: &Value) -> Vec<String> {
    let ids: Vec<u64> = doc["atoms"]
        .as_array()
        .map(|a| a.iter().map(|a| a["id"].as_u64().unwrap()).collect())
        .unwrap_or_default();
    let mut parent: std::collections::BTreeMap<u64, u64> = ids.iter().map(|&i| (i, i)).collect();
    fn find(p: &mut std::collections::BTreeMap<u64, u64>, mut x: u64) -> u64 {
        while p[&x] != x {
            let g = p[&p[&x]];
            p.insert(x, g);
            x = p[&x];
        }
        x
    }
    for b in doc["bonds"].as_array().into_iter().flatten() {
        let (ra, rb) = (
            find(&mut parent, b["a"].as_u64().unwrap()),
            find(&mut parent, b["b"].as_u64().unwrap()),
        );
        parent.insert(ra, rb);
    }
    let mut smallest: std::collections::BTreeMap<u64, u64> = Default::default();
    for &i in &ids {
        let r = find(&mut parent, i);
        let m = smallest.entry(r).or_insert(i);
        *m = (*m).min(i);
    }
    let mut order: Vec<u64> = smallest.values().copied().collect();
    order.sort();
    let mut index = |id: u64| -> String {
        if id == 0 {
            return "-".into();
        }
        let r = find(&mut parent, id);
        order
            .iter()
            .position(|&m| m == smallest[&r])
            .unwrap()
            .to_string()
    };
    let mut out: Vec<String> = doc["objects"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|o| {
            let (f, t) = (
                o["from"].as_u64().unwrap_or(0),
                o["to"].as_u64().unwrap_or(0),
            );
            let mut s = format!(
                "{}|{}",
                o["t"].as_str().unwrap_or(""),
                o["text"].as_str().unwrap_or("")
            );
            if f != 0 || t != 0 {
                s += &format!("|{}>{}", index(f), index(t));
            }
            s
        })
        .collect();
    out.sort();
    out
}

/// The identity of a structure as the editor holds it: opened from the text and written back as SMILES, then `Mol::can`.
fn identity_of(request: Value) -> (String, Vec<String>) {
    let smiles = mzdraw::replay::replay(&request.to_string()).expect("replays");
    let can = Mol::from_smiles(&smiles).expect("the SMILES is read").can();
    let mut doc = request.clone();
    doc["out"] = "mzdraw".into();
    let doc: Value =
        serde_json::from_str(&mzdraw::replay::replay(&doc.to_string()).unwrap()).unwrap();
    let mut objects = objects_of(&doc);
    objects.sort();
    (can, objects)
}

#[test]
fn every_target_is_a_smiles_mzlab_mol_reads() {
    let tasks = read("tasks.json");
    let tasks = tasks["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 16);
    let mut ids = std::collections::BTreeSet::new();
    for t in tasks {
        let (id, smiles) = (
            t["id"].as_str().unwrap(),
            t["target_smiles"].as_str().unwrap(),
        );
        assert!(ids.insert(id), "{id} twice");
        assert!(
            Mol::from_smiles(smiles).is_ok(),
            "{id}: {smiles} is not read"
        );
        // and it survives the editor's door: opened, written, read again
        let request = serde_json::json!({"import": smiles, "out": "smiles"});
        assert!(
            mzdraw::replay::replay(&request.to_string()).is_ok(),
            "{id}: the editor does not open it"
        );
    }
}

#[test]
fn every_task_has_a_page_path_and_a_patch_that_replays_to_the_target() {
    let tasks = read("tasks.json");
    let paths = read("mzdraw.json");
    let agent = read("agent.json");
    for t in tasks["tasks"].as_array().unwrap() {
        let id = t["id"].as_str().unwrap();
        assert!(
            paths["tasks"][id]["steps"]
                .as_array()
                .is_some_and(|s| !s.is_empty()),
            "{id}: no page path"
        );
        let patch = agent["tasks"][id].clone();
        assert!(
            patch.as_array().is_some_and(|p| !p.is_empty()),
            "{id}: no patch"
        );
        let want = identity_of(serde_json::json!({"import": t["target_smiles"], "out": "smiles"}));
        let mut want_objects: Vec<String> = t["objects"]
            .as_array()
            .map(|o| {
                o.iter()
                    .map(|o| {
                        let mut s = format!(
                            "{}|{}",
                            o["t"].as_str().unwrap(),
                            o["text"].as_str().unwrap_or("")
                        );
                        if o.get("from").is_some() || o.get("to").is_some() {
                            let end = |k: &str| o.get(k).map_or("-".to_string(), |v| v.to_string());
                            s += &format!("|{}>{}", end("from"), end("to"));
                        }
                        s
                    })
                    .collect()
            })
            .unwrap_or_default();
        want_objects.sort();
        let got = identity_of(serde_json::json!({"log": patch, "out": "smiles"}));
        assert_eq!(got.0, want.0, "{id}: the patch draws another structure");
        assert_eq!(got.1, want_objects, "{id}: the patch has other objects");
    }
}
