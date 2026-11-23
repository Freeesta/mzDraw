//! The declarative pathway `pathway/1` (`pathway:build`) and the checks `pathway-check/1`, without the engine of the masses (the engine's
//! part is `crates/mzdraw-wasm/tests/pathway_draw.rs`). Synthetic structures only (`tests/data/pathway-draw/`).

use mzdraw::replay::{replay, replay_at};
use mzdraw::Editor;
use serde_json::{json, Value};
use std::collections::BTreeSet;

const MODEL: &str = include_str!("../../../tests/data/pathway-draw/oxidation.json");

fn model() -> Value {
    serde_json::from_str(MODEL).unwrap()
}

fn build(m: &Value) -> Value {
    json!({"commands": [{"command": "pathway:build", "args": {"model": m}}]})
}

fn with_out(m: &Value, out: &str) -> String {
    let mut r = build(m);
    r["out"] = json!(out);
    r.to_string()
}

fn check_of(m: &Value) -> Value {
    serde_json::from_str(&replay(&with_out(m, "pathway-check")).unwrap()).unwrap()
}

fn keys(v: &Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].as_str().unwrap().to_string())
        .collect()
}

/// A model refused with its problems listed: (line, key) of `failed.rows`, and the key of the refusal.
fn refused(m: &Value) -> (String, Vec<(u64, String)>) {
    let mut r = build(m);
    r["report"] = json!(true);
    let v: Value = serde_json::from_str(&replay(&r.to_string()).unwrap()).unwrap();
    assert_eq!(v["ok"], json!(false), "{v}");
    assert_eq!(v["failed"]["stage"], "command");
    let rows = v["failed"]["rows"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|x| {
                    (
                        x["line"].as_u64().unwrap(),
                        x["key"].as_str().unwrap().to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    (v["failed"]["key"].as_str().unwrap().to_string(), rows)
}

#[test]
fn a_model_becomes_a_laid_out_drawing_with_its_data_in_one_step() {
    let mut e = Editor::default();
    let before = e.save("mzdraw").unwrap();
    e.run_command("pathway:build", &json!({"model": model()}))
        .unwrap();
    let doc = e.save("mzdraw").unwrap();
    let v: Value = serde_json::from_str(&doc).unwrap();
    // four structures, three anchored arrows with the labels of the model
    let arrows: Vec<&Value> = v["objects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["t"] == "arrow")
        .collect();
    assert_eq!(arrows.len(), 3);
    assert!(arrows
        .iter()
        .all(|a| a["from"].as_u64().unwrap() > 0 && a["to"].as_u64().unwrap() > 0));
    let texts: Vec<&str> = arrows.iter().map(|a| a["text"].as_str().unwrap()).collect();
    assert_eq!(texts, ["+O", "+O-H2", "+O"]);
    // the data of the structures
    let sdf = e.save("sdf").unwrap();
    for want in [
        "TP_ID",
        "NODE_ID",
        "FORMULA",
        "STATUS",
        "GENERATION",
        "BRANCH",
        "tentative",
        "not_detected",
        "Benzyl alcohol",
    ] {
        assert!(sdf.contains(want), "{want} in the SDF");
    }
    // `observed` is the default and is not written: exactly two STATUS fields
    assert_eq!(sdf.matches("<STATUS>").count(), 2);
    // one undo step brings back the identical document, redo the same bytes
    e.undo();
    assert_eq!(e.save("mzdraw").unwrap(), before);
    e.redo();
    assert_eq!(e.save("mzdraw").unwrap(), doc);
}

#[test]
fn the_layout_has_one_column_per_generation_and_nothing_overlaps() {
    let c = check_of(&model());
    assert_eq!(c["counts"]["crossings"], 0, "{c}");
    let boxes: Value =
        serde_json::from_str(&replay(&with_out(&model(), "boxes")).unwrap()).unwrap();
    let list = boxes["structures"].as_array().unwrap();
    let x = |s: &Value| (s["x0"].as_f64().unwrap() + s["x1"].as_f64().unwrap()) / 2.0;
    // the generations of the model are the layers of the layout, and a layer is a column
    let nodes = c["nodes"].as_array().unwrap();
    let mut col: std::collections::BTreeMap<u64, Vec<f64>> = Default::default();
    // the structures of the boxes are in the order of their smallest atom, which is the order of the nodes
    for (n, s) in nodes.iter().zip(list) {
        col.entry(n["generation"].as_u64().unwrap())
            .or_default()
            .push(x(s));
    }
    assert_eq!(col.len(), 3, "{col:?}");
    for xs in col.values() {
        assert!(
            xs.iter().all(|v| (v - xs[0]).abs() < 1e-3),
            "one x per generation: {xs:?}"
        );
    }
    // no two boxes overlap
    for (i, a) in list.iter().enumerate() {
        for b in &list[i + 1..] {
            let f = |v: &Value, k: &str| v[k].as_f64().unwrap();
            let apart = f(a, "x1") <= f(b, "x0")
                || f(b, "x1") <= f(a, "x0")
                || f(a, "y1") <= f(b, "y0")
                || f(b, "y1") <= f(a, "y0");
            assert!(apart, "{a} overlaps {b}");
        }
    }
}

#[test]
fn the_same_model_gives_the_same_bytes_every_time() {
    for out in [
        "mzdraw",
        "svg",
        "cdxml",
        "ket",
        "pdf",
        "log",
        "pathway-check",
    ] {
        let a = replay(&with_out(&model(), out)).unwrap();
        let b = replay(&with_out(&model(), out)).unwrap();
        assert_eq!(a, b, "{out}");
    }
}

#[test]
fn every_problem_of_a_model_is_listed_with_its_line_and_key() {
    let mut m = model();
    m["nodes"][1]["status"] = json!("maybe"); // line 2
    m["nodes"][2]["smiles"] = json!(""); // line 3: no structure
    m["nodes"][3]["id"] = json!("P"); // line 4: duplicate id
    m["nodes"][0]["formula"] = json!("not a formula"); // line 1
    m["edges"][0]["to"] = json!("nowhere"); // line 5
    m["edges"][1]["to"] = json!("M1"); // line 6: a loop
    m["edges"][2]["label"] = json!(7); // line 7: not a text
    let (key, rows) = refused(&m);
    assert_eq!(key, "error.draw2.pathRows");
    let want: Vec<(u64, &str)> = vec![
        (1, "error.draw2.pathNodeFormula"),
        (2, "error.draw2.pathNodeStatus"),
        (3, "error.draw2.pathNodeSmiles"),
        (4, "error.draw2.pathNodeDup"),
        (5, "error.draw2.pathEdgeEnd"),
        (6, "error.draw2.pathEdgeLoop"),
        (7, "error.draw2.pathEdgeShape"),
    ];
    let got: Vec<(u64, &str)> = rows.iter().map(|(l, k)| (*l, k.as_str())).collect();
    assert_eq!(got, want);
    // nothing was drawn
    let mut r = build(&m);
    r["out"] = json!("mzdraw");
    assert!(replay(&r.to_string()).is_err());
}

#[test]
fn a_duplicate_edge_a_cycle_a_label_clash_and_a_bad_structure_are_refused() {
    let mut m = model();
    m["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"from": "P", "to": "M1"}));
    assert_eq!(refused(&m).1, [(8, "error.draw2.pathEdgeDup".to_string())]);
    let mut m = model();
    m["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({"from": "M2", "to": "P"}));
    let (key, rows) = refused(&m);
    assert_eq!(key, "error.draw2.pathRows"); // the layout's refusal of a cycle, on the lines of the nodes
    assert!(
        rows.iter().all(|r| r.1 == "error.draw2.tpRowCycle") && !rows.is_empty(),
        "{rows:?}"
    );
    let mut m = model();
    m["nodes"][1]["label"] = json!("Toluene");
    assert_eq!(refused(&m).1, [(2, "error.draw2.pathNodeDup".to_string())]);
    let mut m = model();
    m["nodes"][0]["smiles"] = json!("C(C");
    let (key, rows) = refused(&m);
    assert_eq!((key.as_str(), rows[0].0), ("error.draw2.pathRows", 1));
    m = model();
    m["nodes"][0]["ion"] = json!("H+");
    assert_eq!(refused(&m).1[0], (1, "error.draw2.tpRowAdduct".to_string()));
}

#[test]
fn a_model_refused_as_a_whole_has_its_own_key() {
    for bad in [
        json!({"format": "pathway/2", "nodes": [{"id": "a", "smiles": "C"}]}),
        json!({"format": "pathway/1"}),
        json!({"format": "pathway/1", "nodes": []}),
        json!({"format": "pathway/1", "nodes": [{"id": "a", "smiles": "C"}], "colour": 1}),
        json!({"format": "pathway/1", "nodes": "x"}),
    ] {
        let (key, rows) = refused(&bad);
        assert_eq!(
            (key.as_str(), rows.len()),
            ("error.draw2.pathModel", 0),
            "{bad}"
        );
    }
    // `args` must be exactly `{"model": object}`
    for args in [
        json!({}),
        json!({"model": 3}),
        json!({"model": {}, "extra": 1}),
    ] {
        let r = json!({"commands": [{"command": "pathway:build", "args": args}]});
        assert_eq!(
            replay(&r.to_string()).unwrap_err().key,
            "error.draw2.request"
        );
    }
    // the size caps are checked before any work
    let nodes: Vec<Value> = (0..501)
        .map(|i| json!({"id": format!("n{i}"), "smiles": "C"}))
        .collect();
    let (key, _) = refused(&json!({"format": "pathway/1", "nodes": nodes}));
    assert_eq!(key, "error.draw2.pathModel");
}

#[test]
fn dry_run_validates_and_writes_nothing() {
    let mut r = build(&model());
    r["dry_run"] = json!(true);
    let v: Value = serde_json::from_str(&replay(&r.to_string()).unwrap()).unwrap();
    assert_eq!(
        (v["ok"].clone(), v["dry_run"].clone()),
        (json!(true), json!(true))
    );
    assert!(v.get("out").is_none());
    assert!(
        v["created"]["atoms"]
            .as_array()
            .is_some_and(|a| !a.is_empty()),
        "{v}"
    );
    // a bad model is a verdict with exit-1 meaning
    let mut m = model();
    m["edges"][0]["to"] = json!("nowhere");
    let mut r = build(&m);
    r["dry_run"] = json!(true);
    let reply = replay_at(&r.to_string(), 0).unwrap();
    assert!(!reply.ok);
}

#[test]
fn the_log_of_a_built_pathway_replays_to_the_same_bytes() {
    let log = replay(&with_out(&model(), "log")).unwrap();
    // the log holds plain operations of `draw_edit/1`: no operation is new
    assert!(!log.contains("pathway"), "no command name in a log");
    for out in ["mzdraw", "svg", "cdxml"] {
        let direct = replay(&with_out(&model(), out)).unwrap();
        let replayed = replay(
            &json!({"log": serde_json::from_str::<Value>(&log).unwrap(), "out": out}).to_string(),
        )
        .unwrap();
        assert_eq!(direct, replayed, "{out}");
    }
}

// --- the checks without an engine ------------------------------------------------------------------------------------------------

#[test]
fn a_good_model_passes_the_checks_that_need_no_engine() {
    let c = check_of(&model());
    assert_eq!(c["format"], "pathway-check/1");
    assert_eq!(c["ok"], json!(true), "{c}");
    assert_eq!(c["engine"], json!(false));
    assert_eq!(keys(&c), ["pathway.engine"]);
    let edges: Vec<&str> = c["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["result"].as_str().unwrap())
        .collect();
    assert_eq!(edges, ["ok", "ok", "ok"]);
    assert!(c["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|n| n["monoisotopic_mass"].is_null()));
    let formulas: Vec<&str> = c["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["formula"].as_str().unwrap())
        .collect();
    assert_eq!(formulas, ["C7H8", "C7H8O", "C7H6O2", "C7H8O"]);
    let status: Vec<&str> = c["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["status"].as_str().unwrap())
        .collect();
    assert_eq!(
        status,
        ["observed", "observed", "tentative", "not_detected"]
    );
}

#[test]
fn a_label_that_does_not_match_the_structures_is_flagged_and_the_check_fails() {
    // each label against the truth: the delta P > M3 is +O
    for (label, result, errors) in [
        ("-CH2", "mismatch", 1),
        ("+O2", "mismatch", 1),
        ("+C6H8O6", "mismatch", 1),
        ("-Cl+OH", "mismatch", 1),
        ("isomer", "mismatch", 1),
        ("+O", "ok", 0),
        ("hydroxylation", "unparsed", 0),
    ] {
        let mut m = model();
        m["edges"][2]["label"] = json!(label);
        let c = check_of(&m);
        assert_eq!(c["edges"][2]["result"], result, "{label}: {c}");
        assert_eq!(c["counts"]["error"], errors, "{label}");
        assert_eq!(c["ok"], json!(errors == 0), "{label}");
    }
}

#[test]
fn an_isomer_label_is_right_only_for_equal_formulas() {
    let mut m = model();
    m["nodes"][3]["smiles"] = json!("COc1ccccc1"); // anisole C7H8O: an isomer of benzyl alcohol, not of toluene
    m["nodes"][3]["formula"] = json!("C7H8O");
    m["edges"].as_array_mut().unwrap()[2] = json!({"from": "M1", "to": "M3", "label": "isomer"});
    let c = check_of(&m);
    assert_eq!(c["edges"][2]["result"], "ok", "{c}");
    assert_eq!(c["edges"][2]["delta"], "");
}

#[test]
fn a_formula_the_model_states_must_be_the_formula_of_the_structure() {
    let mut m = model();
    m["nodes"][1]["formula"] = json!("C7H8O2");
    let c = check_of(&m);
    assert_eq!(c["ok"], json!(false));
    assert!(
        keys(&c).contains(&"pathway.formulaMismatch".to_string()),
        "{c}"
    );
    let f = c["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == "pathway.formulaMismatch")
        .unwrap();
    assert_eq!(f["item"], "M1");
    // a trailing charge is ignored, so the anion of benzoic acid passes with its charge written
    let mut m = model();
    m["nodes"][2]["formula"] = json!("C7H6O2-");
    assert_eq!(check_of(&m)["ok"], json!(true));
}

#[test]
fn a_stated_generation_that_is_not_the_layer_is_a_warning() {
    let mut m = model();
    m["nodes"][2]["generation"] = json!(1);
    let c = check_of(&m);
    assert_eq!(c["ok"], json!(true));
    assert!(
        keys(&c).contains(&"pathway.generationMismatch".to_string()),
        "{c}"
    );
}

#[test]
fn a_check_on_a_drawing_that_no_model_made_reads_the_drawing() {
    // a pair of structures joined by an arrow drawn by `tp:import`, no model
    let csv = "smiles,name,parent\nCc1ccccc1,P,\nOCc1ccccc1,M,P\n";
    let r = json!({"commands": [{"command": "tp:import", "args": {"csv": csv}}], "out": "pathway-check"});
    let c: Value = serde_json::from_str(&replay(&r.to_string()).unwrap()).unwrap();
    assert_eq!(c["ok"], json!(true), "{c}");
    assert_eq!(c["edges"][0]["delta"], "+O");
    assert_eq!(c["edges"][0]["result"], "ok");
    // an arrow whose text was changed afterwards by hand is caught, and an arrow with no text is only a note
    let doc = replay(
        &json!({"commands": [{"command": "tp:import", "args": {"csv": csv}}], "out": "mzdraw"})
            .to_string(),
    )
    .unwrap();
    let id = serde_json::from_str::<Value>(&doc).unwrap()["objects"][0]["id"]
        .as_u64()
        .unwrap();
    for (text, ok, result) in [("-CH2", false, "mismatch"), ("", true, "unlabelled")] {
        let r = json!({"import": doc, "log": [{"op": "obj.set", "id": id, "text": text}], "out": "pathway-check"});
        let c: Value = serde_json::from_str(&replay(&r.to_string()).unwrap()).unwrap();
        assert_eq!(
            (c["ok"].clone(), c["edges"][0]["result"].clone()),
            (json!(ok), json!(result)),
            "{text}: {c}"
        );
    }
}

#[test]
fn the_options_of_the_check_are_validated() {
    let doc = replay(&with_out(&model(), "mzdraw")).unwrap();
    for (args, ok) in [
        (json!({}), true),
        (json!({"polarity": "negative", "ppm": 5, "da": 0.01}), true),
        (json!({"polarity": "up"}), false),
        (json!({"ppm": 0}), false),
        (json!({"ppm": "x"}), false),
        (json!({"tol": 1}), false),
    ] {
        let r = json!({"import": doc, "out": "pathway-check", "args": args});
        assert_eq!(replay(&r.to_string()).is_ok(), ok, "{args}");
    }
}

#[test]
fn a_failed_check_has_its_own_exit_flag_apart_from_a_refusal() {
    let mut m = model();
    m["edges"][0]["label"] = json!("+O2");
    let r = replay_at(&with_out(&m, "pathway-check"), 0).unwrap();
    assert!(r.ok && r.check_failed);
    let r = replay_at(&with_out(&model(), "pathway-check"), 0).unwrap();
    assert!(r.ok && !r.check_failed);
    // another output never sets it
    let r = replay_at(&with_out(&m, "svg"), 0).unwrap();
    assert!(r.ok && !r.check_failed);
}

/// The example `replay` (the native twin of the page's replay): exit 0, 3 for a failed check, 1 for a refusal. `cargo test` builds the
/// examples next to the test executables.
#[test]
fn the_native_example_exits_with_3_for_a_failed_check_and_1_for_a_refusal() {
    use std::io::Write;
    let exe = std::env::current_exe().unwrap();
    let exe = exe
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/replay");
    if !exe.exists() {
        eprintln!("skipped: {} is not built", exe.display());
        return;
    }
    let run = |request: String| {
        let mut p = std::process::Command::new(&exe)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        p.stdin
            .take()
            .unwrap()
            .write_all(request.as_bytes())
            .unwrap();
        let o = p.wait_with_output().unwrap();
        (
            o.status.code().unwrap(),
            String::from_utf8(o.stdout).unwrap(),
        )
    };
    let mut bad = model();
    bad["edges"][0]["label"] = json!("+O2");
    let (code, out) = run(with_out(&bad, "pathway-check"));
    assert_eq!(code, 3);
    assert!(
        out.contains("\"ok\":false"),
        "the answer is written all the same"
    );
    assert_eq!(run(with_out(&model(), "pathway-check")).0, 0);
    let mut broken = model();
    broken["edges"][0]["to"] = json!("nowhere");
    assert_eq!(run(with_out(&broken, "pathway-check")).0, 1);
}

// --- re-read: write, read back, the same graph and structures --------------------------------------------------------------------

/// The graph of a drawing: the SMILES of each structure, and each anchored arrow as (SMILES of the start, SMILES of the end).
fn graph(e: &Editor) -> (BTreeSet<String>, BTreeSet<(String, String)>) {
    let v: Value = serde_json::from_str(&e.save("mzdraw").unwrap()).unwrap();
    let doc = mzdraw::doc::Document::from_json(&e.save("mzdraw").unwrap()).unwrap();
    let pieces = mzdraw::layout::pieces(&doc);
    let smiles = |atoms: &Vec<u32>| {
        let mut d = doc.clone();
        d.atoms.retain(|a| atoms.contains(&a.id));
        d.bonds
            .retain(|b| atoms.contains(&b.a) && atoms.contains(&b.b));
        d.objects.clear();
        d.to_smiles().unwrap()
    };
    let of = |atom: u64| {
        pieces
            .iter()
            .find(|p| p.contains(&(atom as u32)))
            .map(&smiles)
    };
    let nodes = pieces.iter().map(&smiles).collect();
    let edges = v["objects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["t"] == "arrow")
        .filter_map(|o| Some((of(o["from"].as_u64()?)?, of(o["to"].as_u64()?)?)))
        .collect();
    (nodes, edges)
}

fn built() -> Editor {
    let mut e = Editor::default();
    e.run_command("pathway:build", &json!({"model": model()}))
        .unwrap();
    e
}

#[test]
fn svg_pdf_and_the_document_read_back_as_the_same_drawing() {
    let e = built();
    let want = e.save("mzdraw").unwrap();
    let svg = e.save("svg").unwrap();
    assert!(svg.contains("<svg"));
    // the SVG carries its document: structures, arrows, labels, data
    let mut back = Editor::default();
    back.import(&svg).unwrap();
    assert_eq!(graph(&back), graph(&e));
    assert_eq!(
        back.save("pathway-check").unwrap(),
        e.save("pathway-check").unwrap()
    );
    assert_eq!(back.save("mzdraw").unwrap(), want);
    // the PDF too (the document is attached)
    let pdf = mzdraw::pdf::base64(&e.pdf());
    let again = replay(&json!({"import_base64": pdf, "out": "mzdraw"}).to_string()).unwrap();
    assert_eq!(again, want);
    let c = replay(&json!({"import_base64": pdf, "out": "pathway-check"}).to_string()).unwrap();
    assert_eq!(c, e.save("pathway-check").unwrap());
}

#[test]
fn cdxml_and_ket_read_back_the_same_structures_and_the_same_graph_once_the_arrows_are_anchored() {
    let e = built();
    let (nodes, edges) = graph(&e);
    assert_eq!((nodes.len(), edges.len()), (4, 3));
    for kind in ["cdxml", "ket"] {
        let text = e.save(kind).unwrap();
        let mut back = Editor::default();
        back.import(&text).unwrap();
        // the formats keep atoms, bonds, arrows and the text above an arrow, not the anchors and the data (the loss manifest says so)
        let lost = e.save(&format!("loss:{kind}")).unwrap();
        assert!(
            lost.contains("arrow.anchor") && lost.contains("\"data\""),
            "{kind}: {lost}"
        );
        assert!(graph(&back).1.is_empty(), "{kind}: free arrows");
        back.run_command("pathway:route", &json!({})).ok();
        for id in arrow_ids(&back) {
            back.run_command("pathway:anchor", &json!({"arrow": id}))
                .unwrap();
        }
        let (n2, e2) = graph(&back);
        assert_eq!(n2, nodes, "{kind}: the same structures");
        assert_eq!(e2, edges, "{kind}: the same graph");
        // the labels come back as texts
        let texts: BTreeSet<String> = serde_json::from_str::<Value>(&back.save("mzdraw").unwrap())
            .unwrap()["objects"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["t"] == "text")
            .map(|o| o["text"].as_str().unwrap().to_string())
            .collect();
        assert!(
            ["+O", "+O-H2"].iter().all(|t| texts.contains(*t)),
            "{kind}: {texts:?}"
        );
    }
}

fn arrow_ids(e: &Editor) -> Vec<u64> {
    serde_json::from_str::<Value>(&e.save("mzdraw").unwrap()).unwrap()["objects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["t"] == "arrow")
        .map(|o| o["id"].as_u64().unwrap())
        .collect()
}

#[test]
fn the_status_is_drawn_under_the_structure_in_every_vector_output() {
    let e = built();
    let svg = e.save("svg").unwrap();
    assert!(svg.contains("Benzoic acid (tentative)"), "{svg}");
    assert!(svg.contains("p-Cresol (not detected)"));
    assert!(!svg.contains("Toluene ("));
}

// --- the page's way in: the Add field and the file button take the model -----------------------------------------------------------

#[test]
fn the_add_field_and_the_file_button_take_a_model_and_give_the_bytes_of_the_command() {
    let want = replay(&with_out(&model(), "mzdraw")).unwrap();
    let mut e = Editor::default();
    let info = e.import(MODEL).unwrap();
    assert_eq!((info.format, info.records), ("pathway", 4));
    assert_eq!(e.save("mzdraw").unwrap(), want);
    e.undo();
    assert_eq!(
        e.save("mzdraw").unwrap(),
        Editor::default().save("mzdraw").unwrap(),
        "one undo step"
    );
    let mut e = Editor::default();
    e.import_bytes(MODEL.as_bytes()).unwrap();
    assert_eq!(e.save("mzdraw").unwrap(), want);
    // a refused model adds nothing, names its lines and says so in the live line
    let mut bad = model();
    bad["nodes"][1]["status"] = json!("maybe");
    let mut e = Editor::default();
    assert_eq!(
        e.import(&bad.to_string()).unwrap_err(),
        "error.draw2.pathRows"
    );
    assert_eq!(
        e.save("mzdraw").unwrap(),
        Editor::default().save("mzdraw").unwrap()
    );
    assert_eq!(
        e.failed_rows(),
        [(2, "error.draw2.pathNodeStatus".to_string())]
    );
    // a SMILES and a JSON that is not a model are not taken for one
    assert!(!mzdraw::pathmodel::looks_like("CCO"));
    assert!(!mzdraw::pathmodel::looks_like(
        r#"{"format":"mzdraw/1","note":"pathway/1"}"#
    ));
}

#[test]
fn a_number_is_the_same_text_whatever_wrote_the_json() {
    // JavaScript writes 4.0 as 4: the data field must not depend on the writer
    for (a, b) in [("4.0", "4"), ("6.10", "6.1"), ("1e2", "100")] {
        let (mut x, mut y) = (model(), model());
        x["nodes"][0]["rt"] = serde_json::from_str(a).unwrap();
        y["nodes"][0]["rt"] = serde_json::from_str(b).unwrap();
        assert_eq!(
            replay(&with_out(&x, "mzdraw")).unwrap(),
            replay(&with_out(&y, "mzdraw")).unwrap(),
            "{a} {b}"
        );
    }
    // a number typed as a text is kept as typed
    let mut m = model();
    m["nodes"][0]["rt"] = json!("6.10");
    assert!(replay(&with_out(&m, "mzdraw"))
        .unwrap()
        .contains(r#"["RT","6.10"]"#));
}
