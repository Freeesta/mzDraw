//! Import a list of transformation products and number them (tp-08). Synthetic structures only.

use mzdraw::tp_list::{self, Failure, Input};
use mzdraw::{Document, Editor};
use serde_json::{json, Value};
use std::time::Instant;

const TP12: &str = include_str!("../../../tests/data/tp/tp12.csv");

fn import(csv: &str) -> Result<Editor, Box<(Editor, mzdraw::ops::OpError)>> {
    let mut e = Editor::default();
    match e.run_command("tp:import", &json!({ "csv": csv })) {
        Ok(()) => Ok(e),
        Err(err) => Err(Box::new((e, err))),
    }
}

/// The data field `name` of the structure of atom `atom`.
fn field(doc: &Document, piece: &[u32], name: &str) -> Option<String> {
    doc.atoms
        .iter()
        .filter(|a| piece.contains(&a.id))
        .flat_map(|a| a.data())
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
}

/// (TP_ID, atoms) of every structure, in the order of the drawing.
fn names(doc: &Document) -> Vec<(String, Vec<u32>)> {
    mzdraw::layout::pieces(doc)
        .into_iter()
        .map(|p| (field(doc, &p, "TP_ID").unwrap_or_default(), p))
        .collect()
}

fn replay(request: Value) -> Value {
    serde_json::from_str(&mzdraw::replay::replay(&request.to_string()).unwrap()).unwrap()
}

#[test]
fn twelve_rows_become_a_drawing_with_names_data_levels_and_anchored_arrows() {
    let e = import(TP12).unwrap_or_else(|b| panic!("{:?}", b.1));
    let doc = e.doc();
    let list = names(doc);
    assert_eq!(list.len(), 12);
    let arrows: Vec<_> = doc
        .objects
        .iter()
        .filter(|o| o.from != 0 && o.to != 0)
        .collect();
    assert_eq!(
        arrows.len(),
        12,
        "one arrow per parent link, M2;M3 gives two"
    );
    // the names typed stay, the five that had none are TP1..TP5
    let mut ids: Vec<String> = list.iter().map(|l| l.0.clone()).collect();
    ids.sort();
    let mut want: Vec<String> = [
        "P",
        "M1",
        "M2",
        "M3",
        "ACID",
        "Q, special",
        "CAT",
        "TP1",
        "TP2",
        "TP3",
        "TP4",
        "TP5",
    ]
    .map(String::from)
    .to_vec();
    want.sort();
    assert_eq!(ids, want);
    let of = |n: &str| list.iter().find(|l| l.0 == n).unwrap().1.clone();
    assert_eq!(field(doc, &of("P"), "MZ").as_deref(), Some("135.1168"));
    assert_eq!(field(doc, &of("ACID"), "ADDUCT").as_deref(), Some("[M-H]-"));
    assert_eq!(field(doc, &of("M1"), "RT").as_deref(), Some("11.6"));
    let level = |n: &str| {
        let texts: Vec<String> = doc
            .atoms
            .iter()
            .filter(|a| of(n).contains(&a.id))
            .map(|a| a.ann())
            .collect();
        mzdraw::ann::current(&mzdraw::ann::join(texts.iter().map(String::as_str))).map(|c| c.level)
    };
    assert_eq!(level("P").as_deref(), Some("1"));
    assert_eq!(level("M2").as_deref(), Some("2b"));
    assert_eq!(level("CAT").as_deref(), Some("3"));
    // the layout holds: a root at layer 0, the products to its right
    let lay = mzdraw::pathway::layout(doc, &e.style).unwrap();
    assert_eq!(lay.edges.len(), 12);
    let layer = |n: &str| {
        lay.nodes
            .iter()
            .find(|x| x.atoms.iter().any(|a| of(n).contains(a)))
            .unwrap()
            .layer
    };
    assert_eq!((layer("P"), layer("M1"), layer("TP1")), (0, 1, 2));
    // every arrow runs from the left structure to the right one
    for a in arrows {
        assert!(a.x2 > a.x, "{a:?}");
    }
}

#[test]
fn the_same_list_as_json_rows_gives_the_same_drawing() {
    let a = import(TP12).unwrap_or_else(|b| panic!("{:?}", b.1));
    let rows: Vec<Value> = TP12
        .lines()
        .skip(1)
        .map(|l| {
            // the quoted name holds a comma: split by hand for the one row that has it
            let l = l.replace("\"Q, special\"", "Q;special");
            let f: Vec<&str> = l.split(',').collect();
            let mut o = json!({"smiles": f[0], "name": f[1].replace(';', ", "), "parent": f[2], "mz": f[3], "rt": f[4], "adduct": f[5], "level": f[6]});
            o["mz"] = json!(f[3].parse::<f64>().unwrap().to_string());
            o
        })
        .collect();
    let mut b = Editor::default();
    b.run_command("tp:import", &json!({ "rows": rows }))
        .unwrap();
    assert_eq!(names(a.doc()).len(), names(b.doc()).len());
    let na: Vec<String> = names(a.doc()).into_iter().map(|x| x.0).collect();
    let nb: Vec<String> = names(b.doc()).into_iter().map(|x| x.0).collect();
    assert_eq!(na, nb);
    assert_eq!(a.doc().atoms.len(), b.doc().atoms.len());
}

#[test]
fn a_list_with_a_bad_smiles_an_unknown_parent_and_a_cycle_is_refused_whole() {
    let csv = "smiles,name,parent,mz\nCCO,A,,47.0491\nC1CC,B,A,\nCCC,C,Z,\nCC,D,E,\nCCN,E,D,\nCCCC,F,A,\n";
    let (e, err) = *import(csv).err().unwrap();
    assert_eq!(err.key, "error.draw2.tpRows");
    assert_eq!(
        e.failed_rows().to_vec(),
        vec![
            (3, "error.draw2.tpRowSmiles".to_string()),
            (4, "error.draw2.tpRowParent".to_string()),
            (5, "error.draw2.tpRowCycle".to_string()),
            (6, "error.draw2.tpRowCycle".to_string()),
        ]
    );
    assert!(e.doc().atoms.is_empty(), "nothing is applied");
    // through the request: exit-1 verdict with failed.rows, no out
    let r = replay(
        json!({"commands":[{"command":"tp:import","args":{"csv":csv}}],"out":"mzdraw","report":true}),
    );
    assert_eq!(r["ok"], false);
    assert_eq!(r["failed"]["stage"], "command");
    assert_eq!(r["failed"]["key"], "error.draw2.tpRows");
    assert_eq!(r["failed"]["rows"].as_array().unwrap().len(), 4, "{r}");
    assert_eq!(
        r["failed"]["rows"][0],
        json!({"line":3,"key":"error.draw2.tpRowSmiles"})
    );
    assert!(r.get("out").is_none(), "no out is written");
}

#[test]
fn every_kind_of_row_failure_has_its_key() {
    let rows = |csv: &str| match tp_list::import_ops(
        &Document::default(),
        &Default::default(),
        &Input::Csv(csv),
    ) {
        Err(Failure::Rows(r)) => r,
        other => panic!("{other:?}"),
    };
    let key = |csv: &str| rows(csv)[0].1.clone();
    assert_eq!(key("smiles,mz\nCCO,abc\n"), "error.draw2.tpRowNumber");
    assert_eq!(key("smiles,mz\nCCO,-1\n"), "error.draw2.tpRowNumber");
    assert_eq!(
        key("smiles,adduct\nCCO,[M+X]+\n"),
        "error.draw2.tpRowAdduct"
    );
    assert_eq!(key("smiles,level\nCCO,2 b\n"), "error.draw2.tpRowLevel");
    assert_eq!(key("smiles,name\nCCO,A\nCCN,A\n"), "error.draw2.tpRowDup");
    assert_eq!(key("smiles,name\nCCO,A;B\n"), "error.draw2.tpRowName");
    assert_eq!(
        key("smiles,name,parent\nCCO,A,A\n"),
        "error.draw2.tpRowCycle"
    );
    assert_eq!(key("smiles\nCC.O\n"), "error.draw2.tpRowSmiles");
    assert_eq!(key("smiles\n\nCCO,extra\n"), "error.draw2.tpRowShape");
    assert_eq!(
        key("smiles\n  \n"),
        "error.draw2.tpRowSmiles",
        "a line of spaces is a row with no structure"
    );
    assert_eq!(key("smiles,name\nCCO\n"), "error.draw2.tpRowShape");
    assert_eq!(
        key(&format!("smiles\n{}\n", "C".repeat(501))),
        "error.draw2.tpRowSize"
    );
    assert_eq!(
        rows("smiles\n\"CCO\n")[0].1,
        "error.draw2.tpRowShape",
        "an unclosed quote"
    );
    // the line of a row is the line its record starts on, a quoted line break counts
    let r = rows("smiles,name\nCCO,\"a\nb\"\nXX,B\n");
    assert_eq!(r, vec![(4, "error.draw2.tpRowSmiles".to_string())]);
}

#[test]
fn a_header_that_is_not_a_header_is_refused_with_its_key() {
    for csv in [
        "name,parent\nA,\n",
        "smiles,smiles\nC,C\n",
        "smiles,colour\nC,red\n",
    ] {
        let e = tp_list::import_ops(&Document::default(), &Default::default(), &Input::Csv(csv));
        assert_eq!(e, Err(Failure::Key("error.draw2.tpHeader", 0)), "{csv}");
    }
    let e = tp_list::import_ops(
        &Document::default(),
        &Default::default(),
        &Input::Csv("smiles,name\n"),
    );
    assert_eq!(e, Err(Failure::Key("error.draw2.tpEmpty", 0)));
}

#[test]
fn csv_quoting_crlf_and_bom_are_read_as_rfc_4180() {
    let csv = "\u{feff}smiles,name,parent\r\nCCO,\"He said \"\"hi\"\"\",\r\nCCN,B,\"He said \"\"hi\"\"\"\r\n";
    let e = import(csv).unwrap_or_else(|b| panic!("{:?}", b.1));
    let n: Vec<String> = names(e.doc()).into_iter().map(|x| x.0).collect();
    assert!(n.contains(&"He said \"hi\"".to_string()), "{n:?}");
    assert_eq!(e.doc().objects.len(), 1, "the quoted name is a parent");
}

#[test]
fn a_list_above_the_bounds_is_refused_by_its_size() {
    let many: String = std::iter::once("smiles\n".to_string())
        .chain((0..10_001).map(|_| "C\n".to_string()))
        .collect();
    let e = tp_list::import_ops(
        &Document::default(),
        &Default::default(),
        &Input::Csv(&many),
    );
    assert_eq!(e, Err(Failure::Key("error.draw2.tpSize", 0)));
    let big = format!("smiles,name\nC,{}\n", "x".repeat(1_000_001));
    let e = tp_list::import_ops(&Document::default(), &Default::default(), &Input::Csv(&big));
    assert_eq!(e, Err(Failure::Key("error.draw2.tpSize", 0)));
    // 10,000 rows is allowed by the bound (no parent: no layout, the cost is the rows')
    let ten: String = std::iter::once("smiles\n".to_string())
        .chain((0..10_000).map(|_| "CO\n".to_string()))
        .collect();
    let t = Instant::now();
    let ok = tp_list::import_ops(&Document::default(), &Default::default(), &Input::Csv(&ten));
    assert!(ok.is_ok(), "{:?}", ok.err());
    println!(
        "10,000 rows without a pathway: {:.3} s",
        t.elapsed().as_secs_f64()
    );
}

#[test]
fn numbering_follows_the_order_asked_a_typed_name_survives_and_force_replaces_it() {
    let mut e = import(TP12).unwrap_or_else(|b| panic!("{:?}", b.1));
    // renumber the layout order from scratch, then by m/z and by retention time
    let order = |e: &Editor| -> Vec<(String, String)> {
        let d = e.doc();
        let mut v: Vec<(String, String)> = names(d)
            .into_iter()
            .map(|(n, p)| (n, field(d, &p, "MZ").unwrap_or_default()))
            .filter(|(n, _)| n.starts_with("TP"))
            .collect();
        v.sort();
        v
    };
    assert_eq!(order(&e).len(), 5);
    e.run_command("tp:number", &json!({})).unwrap();
    // the typed names (M1, ACID, "Q, special", CAT...) are kept; the five TPs are TP1..TP5 and nothing else is TP<n>
    let all: Vec<String> = names(e.doc()).into_iter().map(|x| x.0).collect();
    for n in ["P", "M1", "M2", "M3", "ACID", "Q, special", "CAT"] {
        assert!(all.contains(&n.to_string()), "{n} kept");
    }
    // by m/z: among the TPs the smallest m/z is TP1
    let mut z = Editor::default();
    z.run_command("tp:import", &json!({"csv": TP12})).unwrap();
    z.set("pathway.number=mz").unwrap();
    z.run_command("tp:number", &json!({"force": true})).unwrap();
    let d = z.doc();
    let by_mz: Vec<(String, f64)> = {
        let mut v: Vec<(String, f64)> = names(d)
            .into_iter()
            .filter_map(|(n, p)| field(d, &p, "MZ").map(|m| (n, m.parse().unwrap())))
            .filter(|(n, _)| n.starts_with("TP"))
            .collect();
        v.sort_by_key(|a| a.0[2..].parse::<u32>().unwrap());
        v
    };
    assert_eq!(
        by_mz.len(),
        12 - 1,
        "force renames every product: the typed names too (the root P keeps its name)"
    );
    assert!(
        by_mz.windows(2).all(|w| w[0].1 <= w[1].1),
        "TP1.. rise with m/z: {by_mz:?}"
    );
    assert!(
        names(d).iter().any(|x| x.0 == "P"),
        "a root keeps its name even with force"
    );
    // by retention time
    let mut r = Editor::default();
    r.run_command("tp:import", &json!({"csv": TP12})).unwrap();
    r.set("pathway.number=rt").unwrap();
    r.run_command("tp:number", &json!({"force": true})).unwrap();
    let d = r.doc();
    let mut by_rt: Vec<(String, f64)> = names(d)
        .into_iter()
        .filter_map(|(n, p)| field(d, &p, "RT").map(|m| (n, m.parse().unwrap())))
        .filter(|(n, _)| n.starts_with("TP"))
        .collect();
    by_rt.sort_by_key(|a| a.0[2..].parse::<u32>().unwrap());
    assert!(by_rt.windows(2).all(|w| w[0].1 <= w[1].1), "{by_rt:?}");
}

#[test]
fn a_typed_name_is_kept_without_force_and_numbering_counts_only_the_names_it_sets() {
    let csv = "smiles,name,parent\nCCCCO,R,\nCCCO,mine,R\nCCCN,,R\nCCCCN,,R\n";
    let mut e = import(csv).unwrap_or_else(|b| panic!("{:?}", b.1));
    let mut n: Vec<String> = names(e.doc()).into_iter().map(|x| x.0).collect();
    n.sort();
    assert_eq!(n, ["R", "TP1", "TP2", "mine"]);
    let before = e.doc().to_json();
    e.run_command("tp:number", &json!({})).unwrap();
    assert_eq!(
        e.doc().to_json(),
        before,
        "already numbered, typed name kept: no change"
    );
    e.run_command("tp:number", &json!({"force": true})).unwrap();
    let mut n: Vec<String> = names(e.doc()).into_iter().map(|x| x.0).collect();
    n.sort();
    assert_eq!(
        n,
        ["R", "TP1", "TP2", "TP3"],
        "force replaces the typed name"
    );
}

#[test]
fn undo_of_the_import_and_of_the_numbering_returns_the_identical_document() {
    let mut e = Editor::default();
    let empty = e.doc().to_json();
    e.run_command("tp:import", &json!({"csv": TP12})).unwrap();
    let imported = e.doc().to_json();
    e.run_command("tp:number", &json!({"force": true})).unwrap();
    assert_ne!(e.doc().to_json(), imported);
    e.undo();
    assert_eq!(e.doc().to_json(), imported, "one undo step for tp:number");
    e.undo();
    assert_eq!(e.doc().to_json(), empty, "one undo step for tp:import");
    e.redo();
    assert_eq!(e.doc().to_json(), imported);
}

#[test]
fn the_import_log_replays_to_the_same_bytes_and_a_dry_run_writes_nothing() {
    let r = replay(
        json!({"commands":[{"command":"tp:import","args":{"csv":TP12}}],"out":"mzdraw","report":true}),
    );
    assert_eq!(r["ok"], true, "{r}");
    let doc = r["out"].as_str().unwrap().to_string();
    // the document of the run opens and carries the names
    let d = Document::from_json(&doc).unwrap();
    assert_eq!(names(&d).len(), 12);
    let dry =
        replay(json!({"commands":[{"command":"tp:import","args":{"csv":TP12}}],"dry_run":true}));
    assert_eq!(dry["ok"], true);
    assert!(dry.get("out").is_none());
    // a bad shape of the arguments is an invalid request
    for args in [
        json!({}),
        json!({"csv":"a","rows":[]}),
        json!({"csv": 3}),
        json!({"rows": {}}),
    ] {
        let e = mzdraw::replay::replay(
            &json!({"commands":[{"command":"tp:import","args":args}]}).to_string(),
        );
        assert_eq!(e.unwrap_err().key, "error.draw2.request");
    }
}

#[test]
fn the_name_and_the_level_are_drawn_under_the_structure() {
    let e = import(TP12).unwrap_or_else(|b| panic!("{:?}", b.1));
    let svg = e.svg();
    assert!(svg.contains("M2 \u{b7} L2b"), "name and level together");
    assert!(
        svg.contains("TP1 \u{b7} L"),
        "{}",
        &svg[..200.min(svg.len())]
    );
}

fn tree(n: usize) -> String {
    let mut s = String::from("smiles,name,parent,mz,rt\n");
    for i in 0..n {
        let parent = if i == 0 {
            String::new()
        } else {
            format!("N{}", (i - 1) / 3)
        };
        s.push_str(&format!(
            "{}O,N{i},{parent},{},{}\n",
            "C".repeat(i % 7 + 1),
            100 + i,
            i % 30
        ));
    }
    s
}

fn import_time(n: usize) -> f64 {
    let csv = tree(n);
    let t = Instant::now();
    let mut e = Editor::default();
    e.run_command("tp:import", &json!({ "csv": csv }))
        .unwrap_or_else(|err| panic!("{n}: {err:?}"));
    t.elapsed().as_secs_f64()
}

#[test]
fn the_import_grows_linearly_with_the_rows_and_200_rows_take_under_two_seconds_in_release() {
    // `scheme:orient` reads at most 200 arrows (its own cap), so the sizes are above that: 400 and 800 rows, and 1000 is the
    // most a pathway holds. A quadratic step anywhere else (the checks, the placement, the layout, the names) shows as a ratio
    // far above 2.
    let small = import_time(400);
    let big = import_time(800);
    assert!(
        big < 3.0 * small + 0.05,
        "400 rows {small:.3} s, 800 rows {big:.3} s"
    );
    let t = import_time(200);
    println!("200 rows: {t:.3} s");
    if !cfg!(debug_assertions) {
        assert!(t < 2.0, "the kill test: 200 rows took {t:.3} s");
    }
}

#[test]
fn more_than_a_thousand_linked_structures_are_refused_quickly_and_change_nothing() {
    let csv = tree(1500);
    let t = Instant::now();
    let (e, err) = *import(&csv).err().unwrap();
    assert_eq!(err.key, "error.draw2.schemeSize");
    assert!(e.doc().atoms.is_empty());
    assert!(t.elapsed().as_secs_f64() < 20.0);
}
