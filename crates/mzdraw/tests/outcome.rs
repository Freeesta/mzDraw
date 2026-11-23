//! The verdict of a replay request (`draw-result/1`): ids created and removed, the failing op, dry run, postconditions, the refusals
//! that are no document, the key table of the document and a property test of the id sets against the two documents.

mod common;

use common::{golden_files, section};
use mzdraw::outcome::{FAILED_KEYS, KEYS};
use mzdraw::replay::{replay, replay_at};
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn run(request: Value) -> Value {
    serde_json::from_str(&replay(&request.to_string()).unwrap()).unwrap()
}

fn atom(el: &str, x: f64, y: f64) -> Value {
    json!({"op":"atom.add","el":el,"x":x,"y":y})
}

fn keys(v: &Value) -> Vec<&str> {
    v.as_object().unwrap().keys().map(String::as_str).collect()
}

#[test]
fn a_log_that_adds_three_atoms_and_removes_one_reports_exactly_those_ids() {
    let r = run(
        json!({"import":"CC","log":[atom("C",0.0,2.0),atom("O",1.0,2.0),atom("N",2.0,2.0),{"op":"remove","atoms":[1]}],"report":true}),
    );
    assert_eq!(r["ok"], true);
    assert_eq!(r["created"], json!({"atoms":[4,5,6]}));
    assert_eq!(r["removed"], json!({"atoms":[1],"bonds":[3]}));
    assert!(r.get("failed").is_none() && r["out"].is_string());
}

#[test]
fn a_log_failing_at_op_4_reports_it_and_writes_nothing() {
    let log = json!([atom("C",0.0,0.0),atom("C",1.0,0.0),atom("O",2.0,0.0),{"op":"bond.add","a":1,"b":2},{"op":"bond.add","a":1,"b":99}]);
    let reply = replay_at(&json!({"log":log,"report":true}).to_string(), 0).unwrap();
    assert!(!reply.ok);
    let r: Value = serde_json::from_str(&reply.text).unwrap();
    assert_eq!(r["ok"], false);
    assert_eq!(
        r["failed"],
        json!({"stage":"log","op":4,"key":"error.draw2.noAtom","id":99})
    );
    assert!(r.get("out").is_none());
    // the attempted run is described: what was created before the refusal
    assert_eq!(r["created"], json!({"atoms":[1,2,3],"bonds":[4]}));
}

#[test]
fn a_dry_run_is_the_real_run_without_out_and_the_fields_alone_make_the_document() {
    let log = json!([atom("C", 0.0, 0.0), atom("O", 1.0, 0.0)]);
    let real = run(json!({"log":log,"report":true}));
    let dry = run(json!({"log":log,"dry_run":true}));
    assert_eq!(dry["dry_run"], true);
    assert!(dry.get("out").is_none() && real["out"].is_string());
    assert_eq!(dry["created"], real["created"]);
    assert_eq!(dry["ok"], real["ok"]);
    // without any of the three fields the answer is the bare output
    let bare = replay(&json!({"log":log,"out":"smiles"}).to_string()).unwrap();
    assert_eq!(bare, "C.O");
    // `expect` alone, `dry_run: false` and `report: false` alone
    assert_eq!(
        run(json!({"log":log,"expect":[{"pieces":2}]}))["format"],
        "draw-result/1"
    );
    assert_eq!(
        replay(
            &json!({"log":log,"out":"smiles","dry_run":false,"report":false,"expect":[]})
                .to_string()
        )
        .unwrap(),
        "C.O"
    );
}

#[test]
fn a_refused_op_without_the_fields_stays_an_error_and_with_them_is_a_verdict() {
    let log = json!([{"op":"atom.set","id":9,"el":"N"}]);
    let e = replay(&json!({"log":log}).to_string()).unwrap_err();
    assert_eq!((e.key, e.id), ("error.draw2.noAtom", 9));
    let r = run(json!({"log":log,"report":true,"id":"x"}));
    assert_eq!(
        (r["ok"].clone(), r["id"].clone()),
        (json!(false), json!("x"))
    );
    assert_eq!(r["failed"]["stage"], "log");
    assert_eq!(r["failed"]["op"], 0);
}

#[test]
fn an_invalid_request_is_an_error_whatever_the_fields() {
    for bad in [
        json!({"log":[{"op":"teleport"}],"report":true}),
        json!({"log":[],"report":true,"goal":{}}),
        json!({"expect":[{"stereo":"lost"}]}),
        json!({"expect":[{"std":"some"}]}),
        json!({"expect":[{"pieces":1,"smiles":"C"}]}),
        json!({"expect":[{"pieces":-1}]}),
        json!({"expect":{"pieces":1}}),
        json!({"report":"yes"}),
        json!({"report":true,"id":1.5}),
        json!({"report":true,"id":null}),
        json!({"report":true,"out":"nonsense"}),
        json!({"expect":[{"pieces":0}],"out":"nonsense"}),
    ] {
        assert!(replay(&bad.to_string()).is_err(), "{bad}");
    }
    let e = replay(&json!({"expect":[{"stereo":"lost"}]}).to_string()).unwrap_err();
    assert_eq!(e.key, "error.draw2.expect");
    let e = replay(&json!({"flavour":1}).to_string()).unwrap_err();
    assert_eq!(e.key, "error.draw2.requestField");
    assert!(replay("not json").is_err());
}

#[test]
fn each_expect_arm_true_and_false() {
    let check = |import: &str, e: Value| {
        let r = run(json!({"import":import,"expect":[e]}));
        let x = &r["expect"][0];
        (
            r["ok"].as_bool().unwrap(),
            x["ok"].as_bool().unwrap(),
            x["detail"].as_str().unwrap().to_string(),
            r.get("out").is_some(),
        )
    };
    assert_eq!(
        check("CCO", json!({"std":"none"})),
        (true, true, String::new(), true)
    );
    assert_eq!(
        check("CCO", json!({"std_absent":["std.stereo.opposite"]})),
        (true, true, String::new(), true)
    );
    assert_eq!(
        check("CCO", json!({"pieces":1})),
        (true, true, String::new(), true)
    );
    assert_eq!(
        check("CCO", json!({"smiles":"CCO"})),
        (true, true, String::new(), true)
    );
    assert_eq!(
        check("CC.O", json!({"pieces":1})),
        (false, false, "the drawing has 2 pieces".into(), false)
    );
    assert_eq!(
        check("CC.O", json!({"smiles":"CCO"})),
        (
            false,
            false,
            "the SMILES of the drawing is CC.O".into(),
            false
        )
    );
    // a wedge on an atom that is no stereocentre is a finding
    let wedged =
        json!({"log":[atom("C",0.0,0.0),atom("C",1.0,0.0),{"op":"bond.add","a":1,"b":2,"s":1}]});
    for (e, ok) in [
        (json!({"std":"none"}), false),
        (json!({"std_absent":["std.stereo.notCentre"]}), false),
        (json!({"std_absent":["std.stereo.allene"]}), true),
    ] {
        let mut req = wedged.clone();
        req["expect"] = json!([e.clone()]);
        let r = run(req);
        assert_eq!(r["expect"][0]["ok"], ok, "{e}");
        assert_eq!(r["expect"][0]["check"], e);
        assert_eq!(r["expect"][0]["detail"].as_str().unwrap().is_empty(), ok);
    }
    // the first false expectation does not stop the others: all are answered, in request order
    let r = run(json!({"import":"CC.O","expect":[{"pieces":1},{"pieces":2}]}));
    assert_eq!(
        (r["expect"][0]["ok"].clone(), r["expect"][1]["ok"].clone()),
        (json!(false), json!(true))
    );
}

#[test]
fn the_id_is_the_request_id_or_the_index_in_the_batch() {
    let rq = |extra: Value| {
        let mut r = json!({"report":true});
        r.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        r.to_string()
    };
    let id = |text: &str, n| {
        serde_json::from_str::<Value>(&replay_at(text, n).unwrap().text).unwrap()["id"].clone()
    };
    assert_eq!(id(&rq(json!({})), 0), json!(0));
    assert_eq!(id(&rq(json!({})), 5), json!(5));
    assert_eq!(id(&rq(json!({"id":"a-1"})), 5), json!("a-1"));
    assert_eq!(id(&rq(json!({"id":42})), 5), json!(42));
}

/// The members of a JSON object text in the order written: `(key, text of the value)`. (`serde_json::Value` sorts the keys.)
fn members(text: &str) -> Vec<(String, String)> {
    let (b, mut out) = (text.as_bytes(), Vec::new());
    let (mut i, mut depth) = (0, 0);
    let (mut start, mut key): (usize, Option<String>) = (0, None);
    while i < b.len() {
        match b[i] {
            b'"' => {
                let from = i;
                i += 1;
                while b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                if depth == 1 && key.is_none() && b[from - 1] != b':' {
                    key = Some(text[from + 1..i].to_string());
                }
            }
            b'{' | b'[' => {
                depth += 1;
                if depth == 2 {
                    start = i;
                }
            }
            b'}' | b']' => {
                depth -= 1;
                if depth == 1 {
                    out.push((key.take().unwrap(), text[start..=i].to_string()));
                }
            }
            b':' if depth == 1 => start = i + 1,
            _ => {}
        }
        if depth == 1 && (b[i] == b',' || i + 1 == b.len() - 1) && key.is_some() {
            let v = text[start..i + usize::from(b[i] != b',')].trim();
            out.push((key.take().unwrap(), v.to_string()));
        }
        i += 1;
    }
    out
}

fn in_order(got: &[String], table: &[&str]) {
    let at: Vec<usize> = got
        .iter()
        .map(|k| {
            table
                .iter()
                .position(|x| x == k)
                .unwrap_or_else(|| panic!("key {k}"))
        })
        .collect();
    assert!(
        at.windows(2).all(|w| w[0] < w[1]),
        "keys out of order: {got:?}"
    );
}

/// The key table, order included, on a golden envelope given as text; the schema file lists the same keys.
fn check_document(text: &str, schema: &Value) {
    let v: Value = serde_json::from_str(text).unwrap();
    let order: Vec<&str> = schema["x-key-order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k.as_str().unwrap())
        .collect();
    assert_eq!(order, KEYS);
    let top = members(text);
    in_order(
        &top.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
        &KEYS,
    );
    assert_eq!(top.len(), v.as_object().unwrap().len(), "{text}");
    for k in schema["required"].as_array().unwrap() {
        assert!(v.get(k.as_str().unwrap()).is_some(), "missing {k}");
    }
    assert_eq!(v["format"], "draw-result/1");
    assert!(v["id"].is_string() || v["id"].is_u64());
    assert!(v["ok"].is_boolean());
    assert_eq!(
        v.get("out").is_some(),
        v["ok"] == true && v.get("dry_run").is_none()
    );
    if v.get("dry_run").is_some() {
        assert_eq!(v["dry_run"], true);
    }
    for k in ["created", "removed"] {
        let text_of = &top.iter().find(|(key, _)| key == k).unwrap().1;
        in_order(
            &members(text_of)
                .into_iter()
                .map(|(k, _)| k)
                .collect::<Vec<_>>(),
            &["atoms", "bonds", "objects"],
        );
        for list in v[k].as_object().unwrap().values() {
            let ids: Vec<u64> = list
                .as_array()
                .unwrap()
                .iter()
                .map(|i| i.as_u64().unwrap())
                .collect();
            assert!(
                !ids.is_empty() && ids.windows(2).all(|w| w[0] < w[1]),
                "ascending, not empty: {ids:?}"
            );
        }
    }
    if let Some(f) = v.get("failed") {
        let text_of = &top.iter().find(|(key, _)| key == "failed").unwrap().1;
        let fk: Vec<String> = members(text_of).into_iter().map(|(k, _)| k).collect();
        in_order(&fk, &FAILED_KEYS);
        assert_eq!(fk[0], "stage");
        assert!(["log", "select", "plan", "grant", "command", "proposal"]
            .contains(&f["stage"].as_str().unwrap()));
        assert!(f["key"].is_string() && f["id"].is_u64());
        assert_eq!(v["ok"], false);
    }
    if let Some(list) = v.get("expect") {
        let text_of = &top.iter().find(|(key, _)| key == "expect").unwrap().1;
        assert!(
            text_of.contains(r#","ok":"#)
                && text_of.matches(r#"{"check":"#).count() == list.as_array().unwrap().len()
        );
        // the order of each item on the raw text: check, ok, detail
        for item in text_of.split(r#"{"check":"#).skip(1) {
            let (ok, detail) = (
                item.rfind(r#","ok":"#).unwrap(),
                item.rfind(r#","detail":"#).unwrap(),
            );
            assert!(ok < detail, "{item}");
        }
        for x in list.as_array().unwrap() {
            assert_eq!(keys(x), ["check", "detail", "ok"]);
            assert_eq!(x["check"].as_object().unwrap().len(), 1);
            assert!(!x["detail"].as_str().unwrap().is_empty() || x["ok"] == true);
        }
        let all = list.as_array().unwrap().iter().all(|x| x["ok"] == true);
        assert_eq!(v["ok"] == true, v.get("failed").is_none() && all);
    }
}

#[test]
fn every_golden_envelope_follows_the_key_table() {
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/schemas/draw-result-1.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        schema["properties"]["failed"]["x-key-order"],
        json!(FAILED_KEYS)
    );
    let mut n = 0;
    for (_, fx) in golden_files() {
        for case in section(&fx, "results") {
            check_document(case["out"].as_str().unwrap(), &schema);
            n += 1;
        }
    }
    assert!(n >= 7);
}

/// A small deterministic generator (no dependency).
struct Rng(u64);
impl Rng {
    fn next(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
}

fn ids_of(doc_json: &str) -> [BTreeSet<u64>; 3] {
    let v: Value = serde_json::from_str(doc_json).unwrap();
    ["atoms", "bonds", "objects"].map(|k| {
        v.get(k)
            .and_then(Value::as_array)
            .map_or_else(BTreeSet::new, |l| {
                l.iter().map(|x| x["id"].as_u64().unwrap()).collect()
            })
    })
}

#[test]
fn created_and_removed_are_the_differences_of_the_two_documents() {
    let mut rng = Rng(7);
    for case in 0..200 {
        let mut log: Vec<Value> = Vec::new();
        for _ in 0..(1 + rng.next(10)) {
            let a = 1 + rng.next(12);
            let b = 1 + rng.next(12);
            log.push(match rng.next(6) {
                0 | 1 => atom(
                    ["C", "N", "O"][rng.next(3) as usize],
                    rng.next(8) as f64,
                    rng.next(8) as f64,
                ),
                2 => json!({"op":"bond.add","a":a,"b":b}),
                3 => json!({"op":"remove","atoms":[a],"bonds":[b]}),
                4 => json!({"op":"move","atoms":[a],"dx":1.0,"dy":0.0}),
                _ => json!({"op":"obj.add","t":"plus","x":rng.next(8) as f64,"y":9.0}),
            });
        }
        let import = if case % 2 == 0 {
            json!("CCO")
        } else {
            json!("c1ccccc1")
        };
        let r = run(json!({"import":import,"log":log,"report":true}));
        // the run that was attempted: the log up to the op that failed
        let done = r["failed"]["op"].as_u64().map_or(log.len(), |i| i as usize);
        let before = ids_of(&replay(&json!({"import":import}).to_string()).unwrap());
        let after =
            ids_of(&replay(&json!({"import":import,"log":&log[..done]}).to_string()).unwrap());
        for (i, k) in ["atoms", "bonds", "objects"].iter().enumerate() {
            let want = |a: &BTreeSet<u64>, b: &BTreeSet<u64>| -> Value {
                let d: Vec<u64> = a.difference(b).copied().collect();
                json!(d)
            };
            let got = |key: &str| r[key].get(k).cloned().unwrap_or(json!([]));
            assert_eq!(
                got("created"),
                want(&after[i], &before[i]),
                "case {case} created {k}"
            );
            assert_eq!(
                got("removed"),
                want(&before[i], &after[i]),
                "case {case} removed {k}"
            );
        }
        if r["ok"] == true {
            assert_eq!(
                r["out"].as_str().unwrap(),
                replay(&json!({"import":import,"log":log}).to_string()).unwrap()
            );
        }
    }
}

#[test]
fn the_order_check_sees_a_swapped_document() {
    let m = members(r#"{"ok":true,"format":"draw-result/1","created":{"atoms":[1]}}"#);
    let k: Vec<&str> = m.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(k, ["ok", "format", "created"]);
    assert_eq!(m[2].1, r#"{"atoms":[1]}"#);
    let at =
        std::panic::catch_unwind(|| in_order(&["ok".to_string(), "format".to_string()], &KEYS));
    assert!(at.is_err());
}

#[test]
fn an_out_the_drawing_cannot_give_is_the_keyed_error_dry_run_or_not() {
    // exit 2 (an error and no document): the `out` cannot be written for this drawing, so the request cannot be answered
    for extra in [
        json!({"dry_run":true}),
        json!({"report":true}),
        json!({"expect":[{"pieces":0}]}),
    ] {
        let mut r = json!({"out":"rxn"});
        r.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert_eq!(
            replay(&r.to_string()).unwrap_err().key,
            "error.draw2.rxnArrow"
        );
    }
    // a verdict that is not ok writes nothing, so the same `out` is not asked for
    let r = run(json!({"out":"rxn","expect":[{"pieces":1}]}));
    assert_eq!(r["ok"], false);
}

#[test]
fn the_smiles_arm_compares_byte_for_byte_and_says_what_it_found() {
    // not canonicalised: another writing of the same molecule is a false expectation, and `detail` gives the actual SMILES
    let r = run(json!({"import":"CCO","expect":[{"smiles":"OCC"}]}));
    assert_eq!(r["expect"][0]["ok"], false);
    assert_eq!(r["expect"][0]["detail"], "the SMILES of the drawing is CCO");
}

#[test]
fn what_the_import_does_not_keep_is_in_imported_as_the_page_says_it() {
    let notes = |v: &Value| -> Vec<(String, u64)> {
        (v["imported"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter())
        .map(|i| {
            (
                i["key"].as_str().unwrap().to_string(),
                i["n"].as_u64().unwrap(),
            )
        })
        .collect()
    };
    let group = ("draw2.import.stereoGroups".to_string(), 1);
    // an "and" group of a CXSMILES: the drawing keeps the centre as drawn, the group is said
    let v = run(json!({"import": "C[C@H](F)Cl |&1:1|", "report": true, "out": "smiles"}));
    assert_eq!(notes(&v), [group.clone()], "{v}");
    // two "or" groups, with a dry run (nothing written, the notes all the same)
    let v = run(json!({"import": "C[C@H](O)[C@@H](N)C |o1:1,o2:3|", "dry_run": true}));
    assert_eq!(
        notes(&v),
        [("draw2.import.stereoGroups".to_string(), 2)],
        "{v}"
    );
    // the page's notes of the same import are the same items (`io::Imported::notes`, less the computed layout)
    let mut e = mzdraw::Editor::default();
    let page: Vec<(String, u64)> = (e.import("C[C@H](F)Cl |&1:1|").unwrap().notes().into_iter())
        .filter(|(k, n)| *n > 0 && *k != "draw2.import.placeholder")
        .map(|(k, n)| (k.to_string(), n as u64))
        .collect();
    assert_eq!(page, [group]);
    // a plain SMILES says nothing: no key at all (a computed layout is no loss)
    let v = run(json!({"import": "C[C@H](F)Cl", "report": true, "out": "smiles"}));
    assert!(v.get("imported").is_none(), "{v}");
    // the key sits between removed and lost
    let text = replay_at(
        &json!({"import": "C[C@H](F)Cl |&1:1|", "report": true, "out": "smiles"}).to_string(),
        0,
    )
    .unwrap()
    .text;
    assert!(
        text.contains(
            r#""removed":{},"imported":[{"key":"draw2.import.stereoGroups","n":1}],"dirty""#
        ),
        "{text}"
    );
}
