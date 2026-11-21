//! The TP table (tp-09): `save("tptable")` and `save("tptable-csv")`, the rows of a pathway as a table. Synthetic structures only.
//!
//! The engine of the masses is not in this crate: the tests that need one use the stand-in `engine` below (the numbers are fixed and
//! made up, so what is tested is the table, never a mass); the real engine is in `crates/mzdraw-wasm/tests/tptable.rs`.

use mzdraw::tp_list;
use mzdraw::tptable::{self, COLUMNS};
use mzdraw::values::Measured;
use mzdraw::Editor;
use serde_json::{json, Value};
use std::time::Instant;

#[path = "support/timing.rs"]
mod timing;

const TP12: &str = include_str!("../../../tests/data/tp/tp12.csv");

/// A stand-in for the engine: every structure weighs 100 and every adduct adds 1 (a made-up value; a charged structure is not asked).
fn engine(_molfile: &str, adducts: &[String]) -> Result<Measured, &'static str> {
    Ok(Measured {
        mass: 100.0,
        average: None,
        ion_mz: None,
        pct: Vec::new(),
        mz: adducts.iter().map(|_| Some(101.0)).collect(),
    })
}

fn editor(csv: &str) -> Editor {
    let mut e = Editor::default();
    e.run_command("tp:import", &json!({ "csv": csv }))
        .unwrap_or_else(|err| panic!("{err:?}"));
    e.measurer = Some(engine);
    e
}

fn table(e: &Editor) -> Value {
    serde_json::from_str(&e.save("tptable").unwrap()).unwrap()
}

fn cell<'a>(row: &'a Value, col: &str) -> &'a str {
    row[col].as_str().unwrap()
}

#[test]
fn the_columns_are_in_the_order_of_the_contract_and_every_row_has_them_all() {
    assert_eq!(
        COLUMNS,
        [
            "tp_id",
            "parent_id",
            "generation",
            "smiles",
            "formula",
            "monoisotopic_mass",
            "adduct",
            "mz_calc",
            "mz_obs",
            "ppm",
            "rt",
            "form_diff",
            "mass_diff",
            "level",
            "identifier"
        ]
    );
    let e_text = || editor(TP12).save("tptable").unwrap();
    let t = table(&editor(TP12));
    assert_eq!(t["format"], "tptable/1");
    assert_eq!(t["columns"], json!(COLUMNS));
    // the keys of a row are written in the order of the columns (a parsed `Value` sorts its keys, so the text is read)
    let text = e_text();
    let first = &text[text.find("\"rows\":[{").unwrap()..];
    let at: Vec<usize> = COLUMNS
        .iter()
        .map(|c| first.find(&format!("\"{c}\":")).unwrap())
        .collect();
    assert!(at.windows(2).all(|w| w[0] < w[1]), "{at:?}");
    for r in t["rows"].as_array().unwrap() {
        assert_eq!(r.as_object().unwrap().len(), COLUMNS.len());
        assert!(r.as_object().unwrap().values().all(Value::is_string));
    }
}

#[test]
fn a_pathway_has_one_row_per_parent_pair_in_the_order_of_the_numbering() {
    let e = editor(TP12);
    let t = table(&e);
    let rows = t["rows"].as_array().unwrap();
    // 12 structures, one of them (TP2) with two parents: 13 rows
    assert_eq!(rows.len(), 13);
    let pairs: Vec<(&str, &str)> = rows
        .iter()
        .map(|r| (cell(r, "tp_id"), cell(r, "parent_id")))
        .collect();
    assert!(pairs.contains(&("TP2", "M2")) && pairs.contains(&("TP2", "M3")));
    // by layer (generation) first, then the position in the layer; roots come first
    let layers: Vec<u32> = rows
        .iter()
        .map(|r| cell(r, "generation").parse().unwrap())
        .collect();
    assert!(layers.windows(2).all(|w| w[0] <= w[1]), "{layers:?}");
    assert_eq!(layers[0], 0);
    assert_eq!(cell(&rows[0], "tp_id"), "P");
    assert_eq!(cell(&rows[0], "parent_id"), "");
    assert_eq!(cell(&rows[0], "form_diff"), "");
    assert_eq!(cell(&rows[0], "mass_diff"), "");
    // a product is one layer below its parent
    for r in rows.iter().filter(|r| !cell(r, "parent_id").is_empty()) {
        let parent = rows
            .iter()
            .find(|p| cell(p, "tp_id") == cell(r, "parent_id"))
            .unwrap();
        let g = |x: &Value| cell(x, "generation").parse::<u32>().unwrap();
        assert!(g(r) > g(parent), "{r}");
    }
    // the same order tp:number numbered the products in: TP1..TP5 appear in this order among the nameless ones
    let numbered: Vec<&str> = pairs
        .iter()
        .map(|p| p.0)
        .filter(|n| n.starts_with("TP"))
        .collect();
    let mut seen = Vec::new();
    for n in numbered {
        if !seen.contains(&n) {
            seen.push(n);
        }
    }
    assert_eq!(seen, ["TP1", "TP2", "TP3", "TP4", "TP5"]);
}

#[test]
fn every_row_carries_the_data_the_structure_holds_as_typed() {
    let t = table(&editor(TP12));
    let rows = t["rows"].as_array().unwrap();
    let find = |name: &str| rows.iter().find(|r| cell(r, "tp_id") == name).unwrap();
    let m1 = find("M1");
    assert_eq!(cell(m1, "mz_obs"), "151.1117");
    assert_eq!(cell(m1, "rt"), "11.6");
    assert_eq!(cell(m1, "adduct"), "[M+H]+");
    assert_eq!(cell(m1, "level"), "2a");
    assert_eq!(cell(m1, "formula"), "C10H14O");
    assert_eq!(cell(m1, "parent_id"), "P");
    assert_eq!(cell(m1, "form_diff"), "+O");
    // the SMILES is the one `save("smiles")` writes for that structure alone
    assert_eq!(cell(m1, "smiles"), "OCCCCC=1C=CC=CC1");
    let q = find("Q, special");
    assert_eq!(cell(q, "level"), "4");
    // an empty cell is empty, never 0
    assert_eq!(cell(find("P"), "identifier"), "");
}

#[test]
fn without_the_engine_the_mass_cells_are_empty_and_every_structure_is_listed() {
    let mut e = editor(TP12);
    e.measurer = None;
    let t = table(&e);
    for r in t["rows"].as_array().unwrap() {
        for c in ["monoisotopic_mass", "mz_calc", "ppm", "mass_diff"] {
            assert_eq!(cell(r, c), "", "{c} must not be made up");
        }
    }
    assert_eq!(t["engine"], false);
    let note = &t["notes"][0];
    assert_eq!(note["key"], "draw2.export.tpEngine");
    assert_eq!(note["count"], 12);
    assert_eq!(note["items"].as_array().unwrap().len(), 12);
    // the formulas and the formula deltas do not need the engine
    let rows = t["rows"].as_array().unwrap();
    assert_eq!(cell(&rows[1], "form_diff"), "+O");
    // and the draw-result lists the same note in `lost`
    assert_eq!(
        e.lost_on_save("tptable"),
        vec![("draw2.export.tpEngine", 12)]
    );
}

#[test]
fn the_masses_are_the_engines_and_ppm_and_mass_diff_are_computed_from_them() {
    let e = editor(TP12);
    let t = table(&e);
    for r in t["rows"].as_array().unwrap() {
        assert_eq!(cell(r, "monoisotopic_mass"), "100.0000");
        assert_eq!(cell(r, "mz_calc"), "101.0000");
        let obs: f64 = cell(r, "mz_obs").parse().unwrap();
        // (obs - calc) / calc * 10^6, one decimal, computed here from the cells of the row
        let want = format!("{:.1}", (obs - 101.0) / 101.0 * 1e6);
        assert_eq!(cell(r, "ppm"), want, "{r}");
        if !cell(r, "parent_id").is_empty() {
            assert_eq!(cell(r, "mass_diff"), "0.0000", "two stand-in masses of 100");
        }
    }
}

#[test]
fn form_diff_plus_the_parent_formula_is_the_formula_of_the_product() {
    let t = table(&editor(TP12));
    let rows = t["rows"].as_array().unwrap();
    let canon = |s: &str| mzlab_mol::delta::print(&mzlab_mol::delta::parse(s).unwrap());
    let mut checked = 0;
    for r in rows.iter().filter(|r| !cell(r, "parent_id").is_empty()) {
        let parent = rows
            .iter()
            .find(|p| cell(p, "tp_id") == cell(r, "parent_id"))
            .unwrap();
        let sum = mzlab_mol::delta::add(
            &format!("+{}", cell(parent, "formula")),
            cell(r, "form_diff"),
        )
        .unwrap();
        assert_eq!(sum, canon(&format!("+{}", cell(r, "formula"))), "{r}");
        checked += 1;
    }
    assert_eq!(checked, 12);
}

#[test]
fn csv_is_the_same_rows_with_quoting_and_a_header() {
    let e = editor(TP12);
    let csv = e.save("tptable-csv").unwrap();
    let mut lines = csv.lines();
    assert_eq!(lines.next().unwrap(), COLUMNS.join(","));
    assert_eq!(csv.lines().count(), 14);
    assert!(csv.ends_with('\n') && !csv.contains('\r'));
    // the name with a comma is quoted; the same bytes twice
    assert!(csv.contains("\"Q, special\",M3,2,"), "{csv}");
    assert_eq!(csv, e.save("tptable-csv").unwrap());
    // read back with the reader of the list: every field of every line, in order
    let rows: Vec<Vec<String>> = csv
        .lines()
        .skip(1)
        .map(|l| l.split(',').map(str::to_string).collect())
        .collect();
    assert!(rows.iter().all(|r| r.len() >= 15));
}

#[test]
fn a_text_cell_that_a_spreadsheet_would_read_as_a_formula_is_guarded_and_counted() {
    let mut e = editor("smiles,name,parent\nCCO,=cmd,\nCCCO,@x,=cmd\n");
    e.measurer = Some(engine);
    let csv = e.save("tptable-csv").unwrap();
    assert!(csv.contains("\n'=cmd,"), "{csv}");
    assert!(csv.contains("\n'@x,'=cmd,"), "{csv}");
    assert_eq!(
        e.lost_on_save("tptable-csv"),
        vec![("draw2.export.tpGuard", 3)]
    );
    // the JSON is not opened in a spreadsheet and is not changed
    let t = table(&e);
    assert_eq!(cell(&t["rows"][0], "tp_id"), "=cmd");
}

#[test]
fn a_drawing_with_no_arrow_has_no_pathway_and_no_generation() {
    let mut e = Editor::default();
    e.import("CCO.CCCO").unwrap();
    e.measurer = Some(engine);
    let t = table(&e);
    let rows = t["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for r in rows {
        assert_eq!(cell(r, "generation"), "");
        assert_eq!(cell(r, "parent_id"), "");
        assert_eq!(cell(r, "tp_id"), "");
    }
    assert_eq!(cell(&rows[0], "formula"), "C2H6O");
    assert_eq!(cell(&rows[1], "formula"), "C3H8O");
    // an empty drawing is a table with only its header
    let empty = Editor::default();
    assert_eq!(empty.save("tptable-csv").unwrap(), COLUMNS.join(",") + "\n");
    assert_eq!(table(&empty)["rows"], json!([]));
}

#[test]
fn a_structure_no_arrow_touches_follows_the_pathway() {
    // a list with a pathway, then a structure no arrow touches: it comes last and is in no generation
    let mut e = editor("smiles,name,parent\nCCO,A,\nCCCO,B,A\n");
    e.import("CCCC").unwrap();
    e.measurer = Some(engine);
    let t = table(&e);
    let rows = t["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(cell(&rows[2], "formula"), "C4H10");
    assert_eq!(cell(&rows[2], "generation"), "");
    assert_eq!(cell(&rows[2], "tp_id"), "");
    assert_eq!(cell(&rows[1], "generation"), "1");
}

#[test]
fn a_parent_without_a_name_is_listed_and_its_product_has_an_empty_parent_cell() {
    // an anchored arrow from an unnamed structure: the product cannot name its parent
    let mut e = editor("smiles,name,parent\nCCO,A,\nCCCO,B,A\n");
    let anchor = e
        .doc()
        .atoms
        .iter()
        .find(|a| a.data().iter().any(|(k, v)| k == "TP_ID" && v == "A"))
        .unwrap()
        .id;
    e.set_data(anchor, Vec::new());
    e.measurer = Some(engine);
    let t = table(&e);
    let rows = t["rows"].as_array().unwrap();
    let b = rows.iter().find(|r| cell(r, "tp_id") == "B").unwrap();
    assert_eq!(cell(b, "parent_id"), "");
    assert_eq!(cell(b, "form_diff"), "+CH2", "the delta is still there");
    assert!(t["notes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["key"] == "draw2.export.tpParentName" && n["items"] == json!(["B"])));
}

#[test]
fn a_big_drawing_is_refused_before_any_work() {
    // the cap is on the count of atoms, read from the document before the scan
    let atoms: Vec<Value> = (1..=tptable::MAX_ATOMS + 1)
        .map(|i| json!({"id": i, "el": "C", "x": i, "y": 0}))
        .collect();
    let doc =
        json!({"format": "mzdraw/1", "next": tptable::MAX_ATOMS + 2, "atoms": atoms, "bonds": []});
    let mut e = Editor::default();
    e.open(&doc.to_string()).unwrap();
    let t = Instant::now();
    assert_eq!(e.save("tptable"), Err("error.draw2.tpTableSize"));
    assert_eq!(e.save("tptable-csv"), Err("error.draw2.tpTableSize"));
    assert!(t.elapsed().as_secs_f64() < 1.0, "refused before the work");
}

#[test]
fn the_output_is_unknown_on_the_plain_replay_of_a_request_without_the_module() {
    // the kill test of the slice: `tptable-csv` is an output; before the slice it was `error.draw2.request`
    let r = mzdraw::replay::replay(
        &json!({"commands":[{"command":"tp:import","args":{"csv":TP12}}],"out":"tptable-csv"})
            .to_string(),
    )
    .unwrap();
    assert!(r.starts_with("tp_id,parent_id,generation,smiles,"), "{r}");
    // the plain replay has no engine: the mass cells are empty (never made up), the formulas are there
    assert!(r.lines().nth(2).unwrap().contains(",C10H14O,,[M+H]+,,"));
    let bad = mzdraw::replay::replay(&json!({"out":"tptable-xlsx"}).to_string()).unwrap_err();
    assert_eq!(bad.key, "error.draw2.request");
}

#[test]
fn the_report_lists_what_the_table_left_empty() {
    let r = mzdraw::replay::replay(
        &json!({"commands":[{"command":"tp:import","args":{"csv":TP12}}],"out":"tptable","report":true}).to_string(),
    )
    .unwrap();
    let v: Value = serde_json::from_str(&r).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["lost"][0]["key"], "draw2.export.tpEngine");
    assert_eq!(v["lost"][0]["n"], 12);
}

#[test]
fn the_texts_of_the_notes_and_errors_are_on_the_page() {
    let texts = include_str!("../../../web/lang/mzdraw.js");
    for key in tptable::NOTE_KEYS.iter().chain(
        [
            "draw2.import.tpTableIgnored",
            "draw2.import.tpNoPathway",
            "error.draw2.tpTableSize",
            "draw2.export.tptable",
            "draw2.export.tptableCsv",
        ]
        .iter(),
    ) {
        assert!(texts.contains(&format!("\"{key}\"")), "no text for {key}");
    }
}

// --- reading a table back ----------------------------------------------------------------------------------------------

#[test]
fn a_table_is_read_back_as_a_list_and_the_derived_columns_are_not_read() {
    let e = editor(TP12);
    let csv = e.save("tptable-csv").unwrap();
    assert!(tp_list::looks_like(&csv));
    assert_eq!(tp_list::table_ignored(&csv), 7);
    assert_eq!(tp_list::table_ignored(TP12), 0);
    let mut again = Editor::default();
    let info = again.import(&csv).unwrap();
    assert_eq!(info.format, "tp-list");
    again.measurer = Some(engine);
    // the same rows, the same bytes
    assert_eq!(again.save("tptable-csv").unwrap(), csv);
    let (a, b) = (e.save("tptable").unwrap(), again.save("tptable").unwrap());
    assert_eq!(a, b);
}

#[test]
fn a_table_with_two_rows_for_one_product_with_different_structures_is_refused() {
    let csv = "tp_id,parent_id,smiles\nA,,CCO\nB,A,CCCO\nB,A,CCCCO\n";
    let mut e = Editor::default();
    let err = e.import(csv).unwrap_err();
    assert_eq!(err, "error.draw2.tpRows");
    assert_eq!(
        e.failed_rows(),
        vec![(4, "error.draw2.tpRowDup".to_string())]
    );
    // a header that mixes the two kinds of column is refused whole
    let mut e = Editor::default();
    assert!(e.import("tp_id,name,smiles\nA,A,CCO\n").is_err());
}

#[test]
fn an_identifier_comes_back_with_the_level_and_one_without_a_level_is_refused() {
    let mut e = Editor::default();
    e.import("tp_id,smiles,level,identifier\nA,CCO,2a,CHEMBL1\n")
        .unwrap();
    e.measurer = Some(engine);
    let t = table(&e);
    assert_eq!(cell(&t["rows"][0], "identifier"), "CHEMBL1");
    assert_eq!(cell(&t["rows"][0], "level"), "2a");
    let mut e = Editor::default();
    assert!(e
        .import("tp_id,smiles,identifier\nA,CCO,CHEMBL1\n")
        .is_err());
    assert_eq!(
        e.failed_rows(),
        vec![(2, "error.draw2.tpRowLevel".to_string())]
    );
}

// --- scaling -----------------------------------------------------------------------------------------------------------

/// `n` structures of `CC...O`, optionally linked in a tree (every structure the child of a structure three times fewer).
fn big(n: usize, linked: bool) -> Editor {
    let mut csv = String::from("smiles,name,parent,mz,rt,adduct,level\n");
    for i in 0..n {
        let parent = if linked && i > 0 {
            format!("N{}", (i - 1) / 3)
        } else {
            String::new()
        };
        csv.push_str(&format!(
            "{}O,N{i},{parent},{},{},[M+H]+,3\n",
            "C".repeat(i % 7 + 1),
            100 + i,
            i % 30
        ));
    }
    let mut e = editor(&csv);
    e.measurer = Some(engine);
    e
}

/// The least CPU time of the thread over `runs` builds (after a warm-up) of the table of `e`, with the layout of the pathway made before and outside the timing (the document and the
/// stand-in engine too): the work of `tptable` alone.
fn best(e: &Editor, runs: usize) -> f64 {
    let order = mzdraw::tp_list::table_order(e.doc(), &e.style).unwrap();
    timing::best_of(1, runs, || {
        let table = tptable::build_with(e.doc(), order.as_ref(), e.measurer).unwrap();
        assert!(!table.rows.is_empty());
    })
    .as_secs_f64()
}

#[test]
fn the_table_of_unlinked_structures_grows_linearly() {
    // 1000 structures against 250 (the cap): the work of this module alone (no pathway: no layout), the document and the stand-in engine
    // outside the timing. A scan of the atoms, the objects or the structures for each structure shows as a ratio far above 4.
    let (small, large) = (big(250, false), big(1000, false));
    let (a, b) = (best(&small, 7), best(&large, 7));
    println!(
        "250 structures {:.1} ms, 1000 structures {:.1} ms",
        a * 1e3,
        b * 1e3
    );
    assert!(b < 6.0 * a, "250: {a:.4} s, 1000: {b:.4} s");
}

#[test]
fn the_table_of_a_pathway_grows_linearly() {
    // the pathway is laid out before the timing (that layout is tp-05's, with its own tests); 100 against 400 rows
    let (small, large) = (big(100, true), big(400, true));
    let (a, b) = (best(&small, 7), best(&large, 7));
    println!("100 linked {:.1} ms, 400 linked {:.1} ms", a * 1e3, b * 1e3);
    assert!(b < 6.0 * a, "100: {a:.4} s, 400: {b:.4} s");
}

#[test]
fn the_key_table_file_says_what_the_module_writes() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../docs/schemas/tptable-1.json")).unwrap();
    // the columns and their order, the notes and the top-level key order are the module's
    assert_eq!(schema["properties"]["columns"]["const"], json!(COLUMNS));
    assert_eq!(
        schema["properties"]["rows"]["items"]["x-key-order"],
        json!(COLUMNS)
    );
    let props = schema["properties"]["rows"]["items"]["properties"]
        .as_object()
        .unwrap();
    assert!(COLUMNS.iter().all(|c| props.contains_key(*c)) && props.len() == COLUMNS.len());
    assert_eq!(
        schema["properties"]["notes"]["items"]["properties"]["key"]["enum"],
        json!(&tptable::NOTE_KEYS[..8])
    );
    let text = editor(TP12).save("tptable").unwrap();
    let order: Vec<usize> = schema["x-key-order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| text.find(&format!("\"{}\":", k.as_str().unwrap())).unwrap())
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
}

/// A structure whose data `MZ` and `RT` are the given texts (as an SDF may carry any text there).
fn with_fields(name: &str, mz: &str, rt: &str, adduct: &str) -> Editor {
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    let mut data = vec![
        ("TP_ID".to_string(), name.to_string()),
        ("MZ".into(), mz.into()),
        ("RT".into(), rt.into()),
    ];
    if !adduct.is_empty() {
        data.push(("ADDUCT".into(), adduct.into()));
    }
    e.set_data(1, data);
    e.measurer = Some(engine);
    e
}

#[test]
fn a_typed_number_that_is_text_is_guarded_and_listed_with_or_without_an_adduct() {
    let e = with_fields("A", "=HYPERLINK(\"x\")", "@rt", "");
    let csv = e.save("tptable-csv").unwrap();
    assert!(csv.contains(",\"'=HYPERLINK(\"\"x\"\")\","), "{csv}");
    assert!(csv.contains(",'@rt,"), "{csv}");
    assert_eq!(
        e.lost_on_save("tptable-csv"),
        vec![("draw2.export.tpNumber", 1), ("draw2.export.tpGuard", 2)]
    );
    // junk with no adduct is a note too, and a real number is neither guarded nor listed
    let e = with_fields("B", "n/a", "x", "");
    e.save("tptable").unwrap();
    assert_eq!(
        e.lost_on_save("tptable"),
        vec![("draw2.export.tpNumber", 1)]
    );
    let e = with_fields("C", "151.1117", "-1", "[M+H]+");
    let csv = e.save("tptable-csv").unwrap();
    assert!(
        csv.contains(",151.1117,") && csv.contains(",-1,") && !csv.contains("'-1"),
        "{csv}"
    );
    assert!(e.lost_on_save("tptable-csv").is_empty());
}

#[test]
fn a_guarded_text_cell_round_trips_and_a_plain_quote_stays() {
    let e = with_fields("=x", "100", "1", "");
    let csv = e.save("tptable-csv").unwrap();
    assert!(csv.contains("\n'=x,"), "{csv}");
    let mut again = Editor::default();
    again.import(&csv).unwrap();
    again.measurer = Some(engine);
    assert_eq!(again.save("tptable-csv").unwrap(), csv);
    assert_eq!(cell(&table(&again)["rows"][0], "tp_id"), "=x");
    // a name that merely starts with a quote is kept as written
    let mut q = Editor::default();
    q.import("tp_id,smiles\n'a,CCO\n").unwrap();
    assert_eq!(cell(&table(&q)["rows"][0], "tp_id"), "'a");
}

#[test]
fn a_structure_above_the_smiles_cap_has_an_empty_cell_and_a_keyed_note() {
    // 501 atoms in one chain: above `formats::MAX_SMILES_ATOMS`, so no SMILES is written for it; the small one beside it keeps its own
    let mut e = Editor::default();
    e.import(&format!(
        "{}.CCO",
        "C".repeat(mzdraw::formats::MAX_SMILES_ATOMS + 1)
    ))
    .unwrap();
    e.measurer = Some(engine);
    for kind in ["tptable", "tptable-csv"] {
        let text = e.save(kind).unwrap();
        // the cell is empty and the reason is said, not left to be guessed: one note, for the big structure only
        assert_eq!(
            e.lost_on_save(kind),
            vec![("draw2.export.tpSmiles", 1)],
            "{kind}"
        );
        if kind == "tptable" {
            let t: Value = serde_json::from_str(&text).unwrap();
            let rows = t["rows"].as_array().unwrap();
            let smiles: Vec<&str> = rows.iter().map(|r| cell(r, "smiles")).collect();
            assert_eq!(
                smiles.iter().filter(|s| s.is_empty()).count(),
                1,
                "{smiles:?}"
            );
            assert!(smiles.contains(&"CCO"), "{smiles:?}");
            let note = &t["notes"][0];
            assert_eq!(note["key"], "draw2.export.tpSmiles");
            assert_eq!(note["count"], 1);
            assert_eq!(note["items"].as_array().unwrap().len(), 1);
        }
    }
    // the table itself says why: the detail key of the writer is the refusal of `save("smiles")` for that structure
    let table = tptable::build(e.doc(), &e.style, e.measurer).unwrap();
    let n = table
        .notes
        .iter()
        .find(|n| n.key == "draw2.export.tpSmiles");
    assert_eq!(n.map(|n| n.detail), Some("error.draw2.smilesSize"));
}
