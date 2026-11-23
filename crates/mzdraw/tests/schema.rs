//! The table of the operations (`schema::OPS`) against the readers, the writer, mzLab's list and the docs.
//! `MZDRAW_BLESS=1 cargo test -p mzdraw --test schema` rewrites the section "Edit language" of `docs/mzdraw.md`.

use mzdraw::schema::{self, OPS};
use mzdraw::{Axis, ObjKind, Op};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// An operation of every kind with every field set.
fn full_ops() -> Vec<Op> {
    vec![
        Op::AtomAdd {
            map: Some(3),
            id: Some(9),
            el: "C".into(),
            x: 1.5,
            y: -2.0,
        },
        Op::BondAdd {
            id: Some(8),
            a: 1,
            b: 2,
            o: 1,
            s: 1,
        },
        Op::AtomSet {
            map: Some(4),
            id: 1,
            el: Some("N".into()),
            chg: Some(-1),
            iso: Some(15),
            rad: Some(1),
            h: Some(2),
            lab: Some("Ph".into()),
        },
        Op::AtomData {
            id: 1,
            data: vec![("name".into(), "synthetic".into())],
        },
        Op::AtomAnn {
            id: 1,
            ann: r#"[{"level":"2b"}]"#.into(),
        },
        Op::DocLabels {
            mode: "mass".into(),
            adduct: "[M+H]+".into(),
        },
        Op::AtomCap {
            id: 1,
            lines: vec!["formula".into(), "logp".into()],
        },
        Op::Colour {
            atoms: vec![1, 2],
            bonds: vec![3],
            objects: vec![4],
            col: "#d00000".into(),
        },
        Op::BondSet {
            id: 3,
            o: Some(2),
            s: Some(1),
            pos: Some("l".into()),
            k: Some("dash".into()),
            rev: true,
        },
        Op::Rotate {
            atoms: vec![1, 2],
            deg: 30.0,
            x: 0.5,
            y: 0.5,
        },
        Op::Flip {
            atoms: vec![1],
            axis: Axis::V,
            at: 1.0,
        },
        Op::Move {
            atoms: vec![1],
            objects: vec![4],
            dx: 1.0,
            dy: 2.0,
        },
        Op::Remove {
            atoms: vec![1],
            bonds: vec![3],
            objects: vec![4],
        },
        Op::ObjAdd {
            id: Some(7),
            t: ObjKind::Arrow,
            x: 1.0,
            y: 1.0,
            x2: 3.0,
            y2: 1.0,
            text: "hv".into(),
            runs: vec![mzdraw::Run::plain("hv")],
            from: 1,
            to: 2,
        },
        Op::ObjSet {
            id: 7,
            // both keys are written here to hold every field of the row; an applied operation with both is refused
            text: Some("Pd/C".into()),
            runs: Some(vec![mzdraw::Run::sub("2")]),
            x2: Some(4.0),
            y2: Some(2.0),
            from: Some(2),
            to: Some(0),
        },
        Op::Ring {
            n: 6,
            arom: true,
            atom: Some(1),
            bond: Some(3),
            x: 1.0,
            y: 1.0,
        },
        Op::Chain {
            n: 5,
            atom: Some(1),
            x: 1.0,
            y: 1.0,
            deg: 30.0,
        },
    ]
}

fn keys(v: &Value) -> BTreeSet<String> {
    (v.as_object().unwrap().keys())
        .filter(|k| *k != "op")
        .cloned()
        .collect()
}

#[test]
fn every_operation_of_the_writer_is_a_row_with_exactly_its_fields() {
    let ops = full_ops();
    let name = |o: &Op| {
        let v: Value = serde_json::from_str(&o.to_json()).unwrap();
        v["op"].as_str().unwrap().to_string()
    };
    let names: BTreeSet<String> = ops.iter().map(name).collect();
    // the 17 rows read here; `merge` is not read yet
    let read: BTreeSet<String> = (OPS.iter().filter(|o| o.mzdraw))
        .map(|o| o.name.to_string())
        .collect();
    assert_eq!(names, read);
    assert_eq!(read.len(), 17);
    for op in ops {
        let v: Value = serde_json::from_str(&op.to_json()).unwrap();
        let spec = schema::spec(v["op"].as_str().unwrap()).unwrap();
        let table: BTreeSet<String> = spec.fields.iter().map(|f| f.key.to_string()).collect();
        assert_eq!(keys(&v), table, "{}", spec.name);
        assert_eq!(Op::parse(&v).unwrap(), op);
    }
}

/// A value of the type of a field.
fn sample(f: &schema::FieldSpec) -> Value {
    match (f.key, f.ty) {
        ("el", _) => json!("C"),
        ("axis", _) => json!("h"),
        ("pos", _) => json!("l"),
        ("k", _) => json!("dash"),
        ("t", _) => json!("text"),
        (_, "uint") | (_, "int") => json!(1),
        (_, "number") => json!(1.5),
        (_, "string") => json!("a"),
        (_, "bool") => json!(true),
        (_, "uint[]") => json!([1]),
        (_, "string[][2]") => json!([["k", "v"]]),
        (_, "json[]") => json!([]),
        (_, "array") => json!([{"t": "a", "sub": true}]),
        _ => panic!("{} {}", f.key, f.ty),
    }
}

#[test]
fn every_field_of_the_table_is_read_and_a_required_one_is_needed() {
    for spec in OPS.iter().filter(|o| o.mzdraw) {
        let mut base = serde_json::Map::new();
        base.insert("op".into(), json!(spec.name));
        for f in spec.fields.iter().filter(|f| f.required) {
            base.insert(f.key.into(), sample(f));
        }
        let ok = |m: &serde_json::Map<String, Value>| Op::parse(&Value::Object(m.clone())).is_ok();
        assert!(ok(&base), "{} with its required fields", spec.name);
        for f in spec.fields {
            let mut one = base.clone();
            one.insert(f.key.into(), sample(f));
            assert!(ok(&one), "{}.{}", spec.name, f.key);
            // a field of the wrong type is refused whole
            let mut bad = base.clone();
            bad.insert(f.key.into(), json!({"no": 1}));
            assert!(!ok(&bad), "{}.{} as an object", spec.name, f.key);
        }
        for f in spec.fields.iter().filter(|f| f.required) {
            let mut gone = base.clone();
            gone.remove(f.key);
            assert!(!ok(&gone), "{} without {}", spec.name, f.key);
        }
    }
}

#[test]
fn a_field_outside_the_table_is_refused_with_its_own_key() {
    for text in [
        r#"{"op":"atom.set","id":1,"bogus":1}"#,
        r#"{"op":"remove","atoms":[1],"id":1}"#,
        r#"{"op":"ring","n":6,"s":1}"#,
    ] {
        let v: Value = serde_json::from_str(text).unwrap();
        assert_eq!(Op::parse(&v), Err("error.draw2.opField"), "{text}");
        assert!(Op::from_value(&v).is_none());
        let request = format!(r#"{{"log":[{text}]}}"#);
        let err = mzdraw::replay::replay(&request).unwrap_err();
        assert_eq!(err.key, "error.draw2.opField");
    }
    // an unknown operation and a row not read here keep the existing key
    for text in [r#"{"op":"nope"}"#, r#"{"op":"merge","from":1,"into":2}"#] {
        let request = format!(r#"{{"log":[{text}]}}"#);
        let err = mzdraw::replay::replay(&request).unwrap_err();
        assert_eq!(err.key, "error.draw2.request");
    }
}

#[test]
fn the_schema_is_served_by_replay_and_save_with_the_same_bytes() {
    let a = mzdraw::replay::replay(r#"{"out":"schema"}"#).unwrap();
    assert_eq!(a, schema::json());
    assert_eq!(a, mzdraw::Editor::default().save("schema").unwrap());
    // deterministic: the same bytes on every call
    assert_eq!(a, schema::json());
    let v: Value = serde_json::from_str(&a).unwrap();
    assert_eq!(v["format"], "draw-edit-schema/1");
    assert_eq!(v["ops"].as_array().unwrap().len(), OPS.len());
    assert!(a.starts_with(
        r#"{"format":"draw-edit-schema/1","ops":[{"op":"atom.add","since":"mzdraw/1","mzdraw":true,"mzlab":true,"#
    ));
}

#[test]
fn the_table_and_the_list_of_mzlab_agree() {
    let text = std::fs::read_to_string(root().join("tests/data/mzlab_ops.txt")).unwrap();
    assert!(text.contains("76a4703e6d7c26adce67d77caee744242317a280"));
    let list: BTreeSet<&str> = (text.lines())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    assert_eq!(list.len(), 7);
    for name in &list {
        let row = schema::spec(name);
        assert!(row.is_some_and(|o| o.mzlab), "mzLab op {name} is a row");
    }
    for o in OPS.iter().filter(|o| o.mzlab) {
        assert!(list.contains(o.name), "{} is not read by mzLab", o.name);
    }
}

/// The section of the docs, rendered from the JSON the table serves.
fn markdown() -> String {
    let v: Value = serde_json::from_str(&schema::json()).unwrap();
    let cell = |x: &Value| x.as_str().unwrap_or("").replace('|', "\\|");
    let mut out = String::from(
        "Generated from `schema::json()` (`\"out\":\"schema\"`); the test `crates/mzdraw/tests/schema.rs` fails when this text drifts (`MZDRAW_BLESS=1` rewrites it). A field outside the row of its operation is an error (`error.draw2.opField`; before, unknown fields were ignored). The `mzlab` column is true by operation name only: mzLab reads some fields under other names (`label`, `wedge`, `x_a`) and addresses bonds by their atoms.\n\n| op | since | mzdraw | mzlab | inverse | fields (`key` type, `*` required, unit, bounds) |\n|---|---|---|---|---|---|\n",
    );
    for o in v["ops"].as_array().unwrap() {
        let fields: Vec<String> = (o["fields"].as_array().unwrap().iter())
            .map(|f| {
                let mut extra: Vec<&str> = ["unit", "bounds"]
                    .iter()
                    .filter_map(|k| f[k].as_str())
                    .collect();
                if f["mzlab"] == false {
                    extra.push("not read by mzLab");
                }
                format!(
                    "`{}` {}{}{}",
                    f["key"].as_str().unwrap(),
                    f["type"].as_str().unwrap(),
                    if f["required"] == true { "*" } else { "" },
                    if extra.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", extra.join("; "))
                    }
                )
            })
            .collect();
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} |\n",
            o["op"].as_str().unwrap(),
            cell(&o["since"]),
            o[mzdraw::brand::SCHEMA_KEY],
            o["mzlab"],
            cell(&o["inverse"]),
            fields.join("<br>").replace('|', "\\|")
        ));
    }
    out.push_str(
        "\nThe replay request (`mzlab-cli mol draw`) is one JSON object whose fields and `expect` arms are these rows (`request` section of the table); a field or an arm outside it is an invalid request (`error.draw2.requestField`, `error.draw2.expect`). With `report`, `dry_run` or a non-empty `expect` the answer is the document `draw-result/1` (section \"Replay result\"); the request `diff` answers its own document (sections `diff` and `hunk`, plan-15).\n\n| section | key | type | meaning |\n|---|---|---|---|\n",
    );
    for x in v["request"].as_array().unwrap() {
        out.push_str(&format!(
            "| {} | `{}` | {} | {} |\n",
            cell(&x["section"]),
            x["key"].as_str().unwrap(),
            cell(&x["type"]),
            cell(&x["meaning"])
        ));
    }
    out
}

#[test]
fn the_docs_section_is_the_table() {
    let path = root().join("docs/mzdraw.md");
    let doc = std::fs::read_to_string(&path).unwrap();
    let head = "## Edit language (`draw_edit/1`)\n\n";
    let start = doc.find(head).expect("section Edit language") + head.len();
    let end = start + doc[start..].find("\n## ").expect("a section follows");
    let want = markdown();
    if std::env::var("MZDRAW_BLESS").is_ok() {
        let new = format!("{}{}{}", &doc[..start], want, &doc[end..]);
        std::fs::write(&path, new).unwrap();
        return;
    }
    let got = doc[start..end].trim_end_matches('\n');
    assert_eq!(got, want.trim_end_matches('\n'));
}

#[test]
fn the_golden_table_is_one_row_per_line() {
    // two changes to different rows must not meet: the recorded table is the list of its rows, one per line of the file, never one long line
    let path = root().join("tests/data/log/edit-schema.json");
    let text = std::fs::read_to_string(path).unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    let rows = v["schemas"][0]["out"]
        .as_array()
        .expect("the rows, not one line of bytes");
    let table: Value = serde_json::from_str(&schema::json()).unwrap();
    let want = OPS.len() + table["request"].as_array().unwrap().len();
    assert!(
        rows.len() >= want + 2,
        "{} pieces for {want} rows",
        rows.len()
    );
    for needle in [r#"{\"op\":\""#, r#"{\"section\":\""#] {
        for line in text.lines() {
            assert!(line.matches(needle).count() <= 1, "two rows on one line");
        }
    }
    assert_eq!(text.matches(r#"{\"op\":\""#).count(), OPS.len());
    // the closing is a row of its own, after the last request row, so an appended row touches no existing line
    let n = rows.len();
    assert_eq!(rows[n - 1], "]}", "the closing must be its own row");
    assert!(rows[n - 2]
        .as_str()
        .unwrap()
        .starts_with(r#",{"section":""#));
    assert_eq!(
        rows.iter().map(|r| r.as_str().unwrap()).collect::<String>(),
        schema::json()
    );
}
