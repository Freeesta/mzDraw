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
    for k in [
        "arrow_box_crossings",
        "edge_crossings",
        "label_box_overlaps",
    ] {
        assert_eq!(c["counts"][k], 0, "{k}: {c}");
    }
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
        // words only: a label that is given and cannot be read is an error, never a pass
        ("hydroxylation", "unreadable", 1),
        ("+O-", "unreadable", 1),
        ("+O,", "unreadable", 1),
        ("N-oxidation", "unreadable", 1),
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
    // the example is built here (never skipped): the test would otherwise pass when the example is absent
    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let mut build =
        std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    build.args([
        "build",
        "--locked",
        "-q",
        "-p",
        "mzdraw",
        "--example",
        "replay",
    ]);
    if profile_dir.ends_with("release") {
        build.arg("--release");
    }
    assert!(
        build.status().expect("cargo runs").success(),
        "the example replay builds"
    );
    let exe = profile_dir.join("examples/replay");
    assert!(exe.exists(), "{} is built", exe.display());
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

/// The drawing as numbers: (element, x, y) of each atom in order, the bonds as (index, index, order), the arrows as their end points, the texts.
type Shape = (
    Vec<(String, f64, f64)>,
    Vec<(usize, usize, u64)>,
    Vec<[f64; 4]>,
    BTreeSet<String>,
);

fn shape(e: &Editor) -> Shape {
    let v: Value = serde_json::from_str(&e.save("mzdraw").unwrap()).unwrap();
    let atoms = v["atoms"].as_array().unwrap();
    let at = |id: &Value| atoms.iter().position(|a| a["id"] == *id).unwrap();
    (
        atoms
            .iter()
            .map(|a| {
                (
                    a["el"].as_str().unwrap().to_string(),
                    a["x"].as_f64().unwrap(),
                    a["y"].as_f64().unwrap(),
                )
            })
            .collect(),
        v["bonds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| (at(&b["a"]), at(&b["b"]), b["o"].as_u64().unwrap_or(1)))
            .collect(),
        v["objects"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["t"] == "arrow")
            .map(|o| {
                [
                    o["x"].as_f64().unwrap(),
                    o["y"].as_f64().unwrap(),
                    o["x2"].as_f64().unwrap(),
                    o["y2"].as_f64().unwrap(),
                ]
            })
            .collect(),
        v["objects"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["t"] == "text" || o["t"] == "arrow")
            .filter_map(|o| {
                o["text"]
                    .as_str()
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
            })
            .collect(),
    )
}

#[test]
fn cdxml_and_ket_read_back_the_same_atoms_bonds_positions_arrows_and_labels() {
    let e = built();
    let (nodes, edges) = graph(&e);
    assert_eq!((nodes.len(), edges.len()), (4, 3));
    let want = shape(&e);
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
        // the numbers, before the arrows are anchored again: counts, elements, positions, bond orders, arrow end points, label texts
        let got = shape(&back);
        assert_eq!(
            (got.0.len(), got.1.len(), got.2.len()),
            (want.0.len(), want.1.len(), want.2.len()),
            "{kind}: counts"
        );
        // CDXML puts the page on an origin of its own: positions are compared from the first atom
        let (ox, oy) = (got.0[0].1 - want.0[0].1, got.0[0].2 - want.0[0].2);
        for (i, (g, w)) in got.0.iter().zip(&want.0).enumerate() {
            assert_eq!(g.0, w.0, "{kind}: element of atom {i}");
            assert!(
                (g.1 - ox - w.1).abs() < 0.01 && (g.2 - oy - w.2).abs() < 0.01,
                "{kind}: atom {i} at {:?}, was {:?}",
                (g.1 - ox, g.2 - oy),
                (w.1, w.2)
            );
        }
        assert_eq!(got.1, want.1, "{kind}: bonds");
        for (g, w) in got.2.iter().zip(&want.2) {
            let moved = [g[0] - ox, g[1] - oy, g[2] - ox, g[3] - oy];
            assert!(
                moved.iter().zip(w).all(|(a, b)| (a - b).abs() < 0.01),
                "{kind}: arrow {moved:?}, was {w:?}"
            );
        }
        assert_eq!(
            got.3, want.3,
            "{kind}: the label texts (on the arrows or above them)"
        );
        // anchored again, the graph is the same
        back.run_command("pathway:route", &json!({}))
            .expect("pathway:route runs");
        for id in arrow_ids(&back) {
            back.run_command("pathway:anchor", &json!({"arrow": id}))
                .unwrap();
        }
        let (n2, e2) = graph(&back);
        assert_eq!(n2, nodes, "{kind}: the same structures");
        assert_eq!(e2, edges, "{kind}: the same graph");
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
fn the_status_is_drawn_by_the_painter_into_the_svg_and_the_pdf_not_into_cdxml_or_ket() {
    let e = built();
    let svg = e.save("svg").unwrap();
    assert!(svg.contains("Benzoic acid (tentative)"), "{svg}");
    assert!(svg.contains("p-Cresol (not detected)"));
    assert!(!svg.contains("Toluene ("));
    // the PDF is painted from the same scene: its text is text
    let pdf = String::from_utf8_lossy(&e.pdf()).into_owned();
    assert!(
        pdf.contains("Benzoic acid \\(tentative\\)"),
        "the PDF draws the status"
    );
    // CDXML and KET carry the structures, the arrows and their texts, not the data fields or the words under a structure: the loss
    // manifest lists the data, and neither file has the word
    for kind in ["cdxml", "ket"] {
        let text = e.save(kind).unwrap();
        assert!(
            !text.contains("tentative") && !text.contains("not detected"),
            "{kind}"
        );
        assert!(
            e.save(&format!("loss:{kind}"))
                .unwrap()
                .contains("\"data\""),
            "{kind}"
        );
    }
}

#[test]
fn the_words_of_the_status_are_the_callers_and_travel_with_the_document() {
    let mut r = build(&model());
    r["commands"][0]["args"]["words"] =
        json!({"tentative": "provvisorio", "not_detected": "non rilevato"});
    r["out"] = json!("svg");
    let svg = replay(&r.to_string()).unwrap();
    assert!(
        svg.contains("Benzoic acid (provvisorio)") && svg.contains("p-Cresol (non rilevato)"),
        "{svg}"
    );
    // the page route: the words are given to the editor, the document keeps them
    let mut e = Editor::default();
    e.set_status_words("provvisorio", "non rilevato");
    e.import(MODEL).unwrap();
    assert_eq!(e.save("svg").unwrap(), svg);
    // the words are checked: both, 1 to 32 characters, no control characters
    for words in [
        json!({"tentative": "x"}),
        json!({"tentative": "", "not_detected": "y"}),
        json!({"tentative": "a\nb", "not_detected": "y"}),
        json!(3),
    ] {
        let mut r = build(&model());
        r["commands"][0]["args"]["words"] = words;
        assert_eq!(
            replay(&r.to_string()).unwrap_err().key,
            "error.draw2.request"
        );
    }
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

// --- the notation of a label, isotopes, the unchecked state and the layout of a larger pathway -------------------------------------

/// The result of the one edge of a two-node model (`from` to `to`, both SMILES) with this label, and the whole check.
fn one_edge(from: &str, to: &str, label: &str, args: Option<Value>) -> (String, Value) {
    let m = json!({"format": "pathway/1",
        "nodes": [{"id": "a", "smiles": from}, {"id": "b", "smiles": to}],
        "edges": [{"from": "a", "to": "b", "label": label}]});
    let doc = replay(&with_out(&m, "mzdraw")).unwrap();
    let mut r = json!({"import": doc, "out": "pathway-check"});
    if let Some(a) = args {
        r["args"] = a;
    }
    let c: Value = serde_json::from_str(&replay(&r.to_string()).unwrap()).unwrap();
    (c["edges"][0]["result"].as_str().unwrap().to_string(), c)
}

#[test]
fn every_notation_a_mass_spectrometrist_writes_is_read_and_a_wrong_one_fails() {
    // (from, to, labels that are right)
    let right = [
        (
            "c1ccccc1",
            "C1=CCC=CC1",
            vec![
                "+2H",
                "+H2",
                "+2 H",
                "+ 2H",
                "+H2 (hydrogenation)",
                "hydrogenation +2H",
            ],
        ),
        ("C1=CCC=CC1", "c1ccccc1", vec!["-2H", "-H2", "\u{2212}2H"]),
        ("CCO", "CC(=O)O", vec!["+O-2H", "+O-H2", "-2H+O", "+O -2H"]),
        ("CC", "CC(=O)O", vec!["+O2-2H", "+2O-2H", "+2O -H2"]),
        (
            "Oc1ccccc1",
            "OC1C(O)C(O)C(Oc2ccccc2)OC1C(=O)O",
            vec![
                "+C6H8O6",
                "glucuronidation +C6H8O6",
                "+C6H8O6 (glucuronidation)",
                "+C6H8O6 glucuronide",
            ],
        ),
        ("CCO", "CCOC", vec!["+CH2", "+1CH2"]),
        ("CCCl", "CCO", vec!["-Cl+OH", "+OH-Cl", "-Cl +OH"]),
        ("CCO", "CC", vec!["-O"]),
    ];
    for (from, to, labels) in right {
        for l in labels {
            let (result, c) = one_edge(from, to, l, None);
            assert_eq!(
                (result.as_str(), c["ok"].clone()),
                ("ok", json!(true)),
                "{from} > {to} {l:?}: {c}"
            );
        }
    }
    // wrong labels fail, and with exit semantics of a failed check (ok false); `+O-2H` for a loss of H2 was passing before
    for (from, to, l) in [
        ("CC=O", "CC(=O)O", "+O-2H"),
        ("CCO", "CC=O", "+O-2H"),
        ("CCO", "CC(=O)O", "+O-H"),
        ("CCO", "CC(=O)O", "+2O-2H"),
        ("c1ccccc1", "C1=CCC=CC1", "+4H"),
        ("c1ccccc1", "C1=CCC=CC1", "-2H"),
        (
            "Oc1ccccc1",
            "OC1C(O)C(O)C(Oc2ccccc2)OC1C(=O)O",
            "glucuronidation +C6H8O5",
        ),
    ] {
        let (result, c) = one_edge(from, to, l, None);
        assert_eq!(
            (result.as_str(), c["ok"].clone()),
            ("mismatch", json!(false)),
            "{from} > {to} {l:?}: {c}"
        );
        let r = replay_at(&json!({"import": replay(&with_out(&json!({"format": "pathway/1", "nodes": [{"id": "a", "smiles": from}, {"id": "b", "smiles": to}], "edges": [{"from": "a", "to": "b", "label": l}]}), "mzdraw")).unwrap(), "out": "pathway-check"}).to_string(), 0).unwrap();
        assert!(r.check_failed, "{l:?}: exit code 3");
    }
}

#[test]
fn a_label_that_cannot_be_read_is_an_error_unless_the_caller_is_lenient() {
    for l in [
        "hydroxylation",
        "oxidation (CYP)",
        "N-oxidation",
        "+",
        "+O,",
        "+O;",
    ] {
        let (result, c) = one_edge("CC", "CCO", l, None);
        assert_eq!(
            (result.as_str(), c["ok"].clone()),
            ("unreadable", json!(false)),
            "{l:?}"
        );
        let f = c["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["key"] == "pathway.edgeUnparsed")
            .unwrap();
        assert_eq!(f["severity"], "error", "{l:?}");
        // lenient: still named, still visible, a warning
        let (result, c) = one_edge("CC", "CCO", l, Some(json!({"lenient_labels": true})));
        assert_eq!(
            (result.as_str(), c["ok"].clone()),
            ("unreadable", json!(true)),
            "{l:?}"
        );
        let f = c["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["key"] == "pathway.edgeUnparsed")
            .unwrap();
        assert_eq!(f["severity"], "warn", "{l:?}");
        assert_eq!(c["counts"]["warn"], 1);
    }
}

#[test]
fn isotopes_are_named_never_silently_ignored() {
    let (result, c) = one_edge("[13CH3]CO", "[13CH3]C=O", "-2H", None);
    assert_eq!(result, "ok", "{c}");
    let f: Vec<&Value> = c["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["key"] == "pathway.isotopesNotCompared")
        .collect();
    assert_eq!(f.len(), 2, "one for each structure: {c}");
    assert!(f[0]["detail"].as_str().unwrap().contains("13C"));
    assert_eq!(f[0]["severity"], "warn");
    // a drawing without isotopes has none
    let (_, c) = one_edge("CCO", "CC=O", "-2H", None);
    assert!(!keys(&c).contains(&"pathway.isotopesNotCompared".to_string()));
}

#[test]
fn without_the_engine_the_summary_says_unchecked_and_the_caller_may_require_it() {
    let c = check_of(&model());
    assert_eq!(
        (c["ok"].clone(), c["checked"].clone(), c["engine"].clone()),
        (json!(true), json!(false), json!(false))
    );
    assert_eq!(
        c["counts"]["unchecked"], 4,
        "every node's masses and ions: {c}"
    );
    // required: the same check is a failed one (an error finding, exit 3), the document written all the same
    let doc = replay(&with_out(&model(), "mzdraw")).unwrap();
    let r = json!({"import": doc, "out": "pathway-check", "args": {"require_engine": true}})
        .to_string();
    let c: Value = serde_json::from_str(&replay(&r).unwrap()).unwrap();
    assert_eq!(c["ok"], json!(false));
    let f = c["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == "pathway.engine")
        .unwrap();
    assert_eq!(f["severity"], "error");
    assert!(replay_at(&r, 0).unwrap().check_failed);
    // a numeric label without the engine is counted too
    let (result, c) = one_edge("CCO", "CC(=O)O", "+13.9793", None);
    assert_eq!(result, "unchecked");
    assert_eq!(c["counts"]["unchecked"], 3);
}

/// A pathway of 16 nodes and 16 edges (a tree of 15 and the second parent of one product): a root, five branches of different depth, a convergent product (n12 has two parents), a node without a generation, and the text of every arrow as given.
fn big_model(label: &str) -> Value {
    let smiles = [
        "CC", "CCC", "CCO", "CCN", "CCCl", "CCS", "CCCC", "CCCO", "CCCN", "CCCCl", "CCCS", "CCOC",
        "CCNC", "CCCCC", "CCCCO", "CCOCC",
    ];
    let nodes: Vec<Value> = smiles
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut n =
                json!({"id": format!("n{i}"), "smiles": s, "label": format!("Product number {i}")});
            if i != 5 {
                n["generation"] = json!([0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3][i]);
            }
            n
        })
        .collect();
    let pairs = [
        (0, 1),
        (0, 2),
        (0, 3),
        (0, 4),
        (0, 5),
        (1, 6),
        (2, 7),
        (3, 8),
        (4, 9),
        (5, 10),
        (2, 11),
        (8, 12),
        (6, 13),
        (7, 14),
        (11, 15),
    ];
    let mut edges: Vec<Value> = pairs
        .iter()
        .map(|(a, b)| json!({"from": format!("n{a}"), "to": format!("n{b}"), "label": label}))
        .collect();
    // two parents for n12 (n8 and n9, both in layer 2): the sixteenth edge
    edges.push(json!({"from": "n9", "to": "n12", "label": label}));
    json!({"format": "pathway/1", "nodes": nodes, "edges": edges})
}

/// The three measures of the layout of the check, and the finding keys behind them.
fn measures(c: &Value) -> ([u64; 3], Vec<String>) {
    let k = |n: &str| c["counts"][n].as_u64().unwrap();
    let list = c["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| {
            f["key"].as_str().unwrap().contains("rossing")
                || f["key"].as_str().unwrap().contains("Overlap")
        })
        .map(|f| format!("{} {}", f["key"], f["item"]))
        .collect();
    (
        [
            k("arrow_box_crossings"),
            k("edge_crossings"),
            k("label_box_overlaps"),
        ],
        list,
    )
}

fn no_two_structure_boxes_overlap(m: &Value) {
    let boxes: Value = serde_json::from_str(&replay(&with_out(m, "boxes")).unwrap()).unwrap();
    let list = boxes["structures"].as_array().unwrap();
    assert_eq!(list.len(), 16);
    let f = |v: &Value, k: &str| v[k].as_f64().unwrap();
    for (i, a) in list.iter().enumerate() {
        for b in &list[i + 1..] {
            let apart = f(a, "x1") <= f(b, "x0")
                || f(b, "x1") <= f(a, "x0")
                || f(a, "y1") <= f(b, "y0")
                || f(b, "y1") <= f(a, "y0");
            assert!(apart, "{a} overlaps {b}");
        }
    }
}

#[test]
fn a_pathway_of_sixteen_nodes_sixteen_edges_two_parents_many_branches_has_no_overlap_and_no_crossing(
) {
    // the labels of the arrows are as long as the delta of a real step (`+O`, `-CH2`, `+C6H8O6`) and the boxes are clear
    for label in ["+CH2", "+O-2H", "+C6H8O6"] {
        let m = big_model(label);
        assert_eq!(
            (
                m["nodes"].as_array().unwrap().len(),
                m["edges"].as_array().unwrap().len()
            ),
            (16, 16)
        );
        let c = check_of(&m);
        assert_eq!(
            (c["counts"]["nodes"].clone(), c["counts"]["edges"].clone()),
            (json!(16), json!(16))
        );
        let (counts, list) = measures(&c);
        assert_eq!(counts, [0, 0, 0], "{label}: {list:?}");
        no_two_structure_boxes_overlap(&m);
        // one column per layer: the generation of the model is the layer of the layout (a node that states none gets its layer)
        let nodes = c["nodes"].as_array().unwrap();
        let layer = |i: usize| nodes[i]["generation"].as_u64().unwrap();
        assert_eq!(
            [layer(0), layer(1), layer(6), layer(13)],
            [0, 1, 2, 3],
            "{label}"
        );
        assert_eq!(layer(5), 1, "the node without a generation has its layer");
        assert!(
            !keys(&c).contains(&"pathway.generationMismatch".to_string()),
            "{label}: {c}"
        );
    }
}

#[test]
fn a_very_long_label_on_a_steep_fan_is_flagged_by_the_check_because_the_layout_does_not_make_room_for_it(
) {
    // KNOWN LIMIT, not a pass: the layout makes room for the visible LENGTH of an arrow (tp-05), not for the horizontal text of a steep one,
    // so the text of an arrow of the five-arrow fan from the root lies on a neighbouring arrow (and the two arrows that converge on one product have texts that meet). The structures stay clear and the arrows
    // do not meet a box; `label_box_overlaps` counts what the eye sees, and a warning names each.
    let m = big_model("+C6H8O6 glucuronide conjugate of the parent");
    let c = check_of(&m);
    let (counts, list) = measures(&c);
    assert_eq!((counts[0], counts[1]), (0, 0), "{list:?}");
    assert!(counts[2] >= 1, "the overlap is measured: {list:?}");
    assert!(list.iter().all(|l| l.contains("labelOverlap")), "{list:?}");
    assert!(
        list.iter().filter(|l| l.contains("n0>")).count() >= 4,
        "the fan from the root: {list:?}"
    );
    no_two_structure_boxes_overlap(&m);
}
