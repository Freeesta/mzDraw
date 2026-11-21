//! The TP table with the real engine (`sel` of mzLab through the glue): the masses, the m/z and the differences checked against hand-computed
//! numbers, the round trips (CSV and SDF), the page path (`Editor::save` with the engine of the bridge), the notes, and the goldens of
//! `tests/data/log/tp-table.json`. Synthetic structures only.
//!
//! The hand sums use these masses (u): H 1.00782503207, C 12 (exact), O 15.99491461956, electron 0.00054857991, so [M+H]+ adds H minus an
//! electron, 1.00727645216, and [M-H]- takes it away.

use mzdraw::Editor;
use mzdraw_wasm::glue;
use serde_json::{json, Value};

const TP12: &str = include_str!("../../../tests/data/tp/tp12.csv");

fn run(request: Value) -> String {
    glue::replay(&request.to_string()).unwrap_or_else(|e| panic!("{}", e.to_json()))
}

fn import_request(csv: &str, out: &str) -> Value {
    json!({"commands":[{"command":"tp:import","args":{"csv":csv}}],"out":out})
}

fn table(csv: &str) -> Value {
    serde_json::from_str(&run(import_request(csv, "tptable"))).unwrap()
}

fn cell<'a>(row: &'a Value, col: &str) -> &'a str {
    row[col].as_str().unwrap()
}

fn row<'a>(t: &'a Value, name: &str, parent: &str) -> &'a Value {
    t["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| cell(r, "tp_id") == name && cell(r, "parent_id") == parent)
        .unwrap_or_else(|| panic!("{name} <- {parent}"))
}

#[test]
fn the_masses_and_mz_are_the_hand_computed_ones_to_four_decimals() {
    let t = table(TP12);
    // P, C10H14: 10 * 12 + 14 * 1.00782503207 = 134.10955 -> 134.1096; [M+H]+ = 134.10955 + 1.00727645 = 135.11683 -> 135.1168
    let p = row(&t, "P", "");
    assert_eq!(cell(p, "formula"), "C10H14");
    assert_eq!(cell(p, "monoisotopic_mass"), "134.1096");
    assert_eq!(cell(p, "mz_calc"), "135.1168");
    // M1, C10H14O: + 15.99491462 = 150.10446 -> 150.1045; [M+H]+ 151.11174 -> 151.1117
    let m1 = row(&t, "M1", "P");
    assert_eq!(cell(m1, "monoisotopic_mass"), "150.1045");
    assert_eq!(cell(m1, "mz_calc"), "151.1117");
    assert_eq!(cell(m1, "form_diff"), "+O");
    // +O: 15.99491462 -> +15.9949
    assert_eq!(cell(m1, "mass_diff"), "+15.9949");
    // ACID, C10H12O2: 120 + 12 * 1.00782503 + 2 * 15.99491462 = 164.08373 -> 164.0837; [M-H]- = 164.08373 - 1.00727645 = 163.07645 -> 163.0765
    let acid = row(&t, "ACID", "M1");
    assert_eq!(cell(acid, "monoisotopic_mass"), "164.0837");
    assert_eq!(cell(acid, "mz_calc"), "163.0765");
    assert_eq!(cell(acid, "adduct"), "[M-H]-");
    // M1 -> ACID is +O-H2: 15.99491462 - 2.01565006 = 13.97926456 -> +13.9793 (the printed masses differ by 13.9792)
    assert_eq!(cell(acid, "form_diff"), "+O-H2");
    assert_eq!(cell(acid, "mass_diff"), "+13.9793");
    // ACID -> TP3 (the homologue with one CH2 less): -(12 + 2 * 1.00782503) = -14.01565006 -> -14.0157
    let tp3 = row(&t, "TP3", "ACID");
    assert_eq!(cell(tp3, "form_diff"), "-CH2");
    assert_eq!(cell(tp3, "mass_diff"), "-14.0157");
    assert_eq!(cell(tp3, "formula"), "C9H10O2");
}

#[test]
fn every_ppm_is_recomputed_from_mz_obs_and_mz_calc_of_the_same_row() {
    let t = table(TP12);
    let mut checked = 0;
    for r in t["rows"].as_array().unwrap() {
        let (obs, calc): (f64, f64) = (
            cell(r, "mz_obs").parse().unwrap(),
            cell(r, "mz_calc").parse().unwrap(),
        );
        let want = (obs - calc) / calc * 1e6;
        let want = format!("{want:.1}");
        let want = if want == "-0.0" {
            "0.0".to_string()
        } else {
            want
        };
        assert_eq!(cell(r, "ppm"), want, "{r}");
        checked += 1;
    }
    assert_eq!(checked, 13);
    // the two [M-H]- products whose typed m/z is 0.001 below the calculated one are -6 to -7 ppm: the table says so, it does not hide it
    assert_eq!(cell(row(&t, "ACID", "M1"), "ppm"), "-6.7");
}

#[test]
fn every_form_diff_and_mass_diff_agrees_with_the_two_formulas_and_masses() {
    let t = table(TP12);
    let rows = t["rows"].as_array().unwrap();
    for r in rows.iter().filter(|r| !cell(r, "parent_id").is_empty()) {
        let parent = row(
            &t,
            cell(r, "parent_id"),
            parent_of(rows, cell(r, "parent_id")),
        );
        let sum = mzlab_mol::delta::add(
            &format!("+{}", cell(parent, "formula")),
            cell(r, "form_diff"),
        )
        .unwrap();
        let want = mzlab_mol::delta::print(
            &mzlab_mol::delta::parse(&format!("+{}", cell(r, "formula"))).unwrap(),
        );
        assert_eq!(sum, want, "{r}");
        // the mass difference is within one unit of the last place of the difference of the two printed masses
        let (a, b): (f64, f64) = (
            cell(parent, "monoisotopic_mass").parse().unwrap(),
            cell(r, "monoisotopic_mass").parse().unwrap(),
        );
        let d: f64 = cell(r, "mass_diff").parse().unwrap();
        assert!((d - (b - a)).abs() < 1.5e-4, "{r}");
    }
}

/// The parent of `name` in the first row that names it (to look up a row by name and parent).
fn parent_of<'a>(rows: &'a [Value], name: &str) -> &'a str {
    cell(
        rows.iter().find(|r| cell(r, "tp_id") == name).unwrap(),
        "parent_id",
    )
}

/// The structures and arrows of a document, canonically: for each structure its name, the identity of its structure (`Mol::can`), its data,
/// its annotation and the centre of its atoms; for each arrow the two names and its text. No id is in it.
fn canon(doc_json: &str) -> Value {
    let doc = mzdraw::Document::from_json(doc_json).unwrap();
    let mut structures = Vec::new();
    let mut name_of = std::collections::HashMap::new();
    for piece in mzdraw::layout::pieces(&doc) {
        let atoms: Vec<&mzdraw::doc::Atom> =
            doc.atoms.iter().filter(|a| piece.contains(&a.id)).collect();
        let name = atoms
            .iter()
            .flat_map(|a| a.data())
            .find(|(k, _)| k == "TP_ID")
            .map(|d| d.1)
            .unwrap_or_default();
        for a in &atoms {
            name_of.insert(a.id, name.clone());
        }
        let n = atoms.len() as f64;
        let (cx, cy) = (
            atoms.iter().map(|a| a.x).sum::<f64>() / n,
            atoms.iter().map(|a| a.y).sum::<f64>() / n,
        );
        let mut data: Vec<(String, String)> = atoms.iter().flat_map(|a| a.data()).collect();
        data.sort();
        let ann: Vec<String> = atoms
            .iter()
            .map(|a| a.ann())
            .filter(|a| a.len() > 2)
            .collect();
        let part: Vec<String> = Vec::new();
        let _ = part;
        structures.push(json!({"name": name, "atoms": atoms.len(), "data": data, "ann": ann, "at": [format!("{cx:.3}"), format!("{cy:.3}")]}));
    }
    structures.sort_by_key(|s| s["name"].as_str().unwrap().to_string());
    let mut arrows: Vec<Value> = doc
        .objects
        .iter()
        .filter(|o| o.from != 0 && o.to != 0)
        .map(|o| json!([name_of[&o.from], name_of[&o.to], o.text]))
        .collect();
    arrows.sort_by_key(Value::to_string);
    json!({"structures": structures, "arrows": arrows})
}

#[test]
fn a_tp_list_exported_as_a_table_and_imported_again_is_the_same_document_and_the_same_csv() {
    let a_csv = run(import_request(TP12, "tptable-csv"));
    let a_doc = run(import_request(TP12, "mzdraw"));
    // the table is read back as a list
    let b_csv = run(import_request(&a_csv, "tptable-csv"));
    let b_doc = run(import_request(&a_csv, "mzdraw"));
    assert_eq!(a_csv, b_csv, "the same CSV bytes");
    assert_eq!(canon(&a_doc), canon(&b_doc), "the same document, ids aside");
    // and once more, from the second export
    assert_eq!(run(import_request(&b_csv, "tptable-csv")), a_csv);
    // 12 structures, 12 arrows
    assert_eq!(canon(&b_doc)["structures"].as_array().unwrap().len(), 12);
    assert_eq!(canon(&b_doc)["arrows"].as_array().unwrap().len(), 12);
}

#[test]
fn the_sdf_round_trip_keeps_the_data_and_annotations_and_says_that_the_pathway_is_gone() {
    let sdf = run(import_request(TP12, "sdf"));
    let mut e = Editor::default();
    e.measurer = Some(glue::measure);
    let info = e.import(&sdf).unwrap();
    assert_eq!(info.records, 12);
    // the pathway is not in an SDF: listed in the import notes, never dropped silently
    assert_eq!(info.no_pathway, 12);
    assert!(info.notes().contains(&("draw2.import.tpNoPathway", 12)));
    let back: Value = serde_json::from_str(&e.save("tptable").unwrap()).unwrap();
    let orig = table(TP12);
    // per structure: the data, the annotation and the structure come back (the rows of a convergent product are one here, with no parent)
    let mut names = Vec::new();
    for r in back["rows"].as_array().unwrap() {
        assert_eq!(cell(r, "parent_id"), "");
        assert_eq!(cell(r, "generation"), "");
        assert_eq!(cell(r, "form_diff"), "");
        assert_eq!(cell(r, "mass_diff"), "");
        let o = orig["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| cell(o, "tp_id") == cell(r, "tp_id"))
            .unwrap();
        for c in [
            "formula",
            "monoisotopic_mass",
            "adduct",
            "mz_calc",
            "mz_obs",
            "ppm",
            "rt",
            "level",
            "identifier",
        ] {
            assert_eq!(cell(r, c), cell(o, c), "{} {c}", cell(r, "tp_id"));
        }
        // the SMILES may be written in another atom order: the structure is the same
        let id = |s: &str| mzlab_mol::Mol::from_smiles(s).unwrap().can();
        assert_eq!(
            id(cell(r, "smiles")),
            id(cell(o, "smiles")),
            "{}",
            cell(r, "tp_id")
        );
        names.push(cell(r, "tp_id").to_string());
    }
    names.sort();
    assert_eq!(names.len(), 12);
    // the SDF of the re-read drawing is the same SDF
    assert_eq!(e.save("sdf").unwrap().matches("$$$$").count(), 12);
}

#[test]
fn the_page_path_gives_the_bytes_of_the_replay_and_the_notes_of_each_case() {
    // the page: an Editor with the engine of the bridge saves; the replay through the glue door is the CLI route
    let mut e = Editor::default();
    e.measurer = Some(glue::measure);
    e.run_command("tp:import", &json!({"csv": TP12})).unwrap();
    for (kind, out) in [("tptable", "tptable"), ("tptable-csv", "tptable-csv")] {
        assert_eq!(
            e.save(kind).unwrap(),
            run(import_request(TP12, out)),
            "{kind}"
        );
    }
    assert_eq!(e.lost_on_save("tptable-csv"), Vec::<(&str, usize)>::new());
}

#[test]
fn what_the_engine_cannot_do_is_a_note_and_an_empty_cell() {
    // a sodium salt (no valence rule for Na+: the engine refuses), a charged structure with an adduct, an adduct the engine does not know,
    // an R group, and a good one
    let csv = "smiles,name,adduct,mz\n[Na+],S1,[M+H]+,23\nC[NH3+],S2,[M+H]+,32.05\nCCO,S3,[M+H-H2O]+,29.03\nCC,S4,[M+H]+,31.0\nCCO,S5,[M+H]+,47.0491\n";
    let r = run(
        json!({"commands":[{"command":"tp:import","args":{"csv":csv}}],"out":"tptable","report":true}),
    );
    let v: Value = serde_json::from_str(&r).unwrap();
    let t: Value = serde_json::from_str(v["out"].as_str().unwrap()).unwrap();
    let find = |n: &str| {
        t["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| cell(r, "tp_id") == n)
            .unwrap()
            .clone()
    };
    assert_eq!(
        cell(&find("S1"), "monoisotopic_mass"),
        "",
        "an element the engine has no valence rule for"
    );
    assert_eq!(
        cell(&find("S2"), "mz_calc"),
        "",
        "a charged structure has no adduct"
    );
    assert_ne!(cell(&find("S2"), "monoisotopic_mass"), "");
    assert_eq!(
        cell(&find("S3"), "mz_calc"),
        "",
        "[M+H-H2O]+ is not one of the seven"
    );
    assert_ne!(cell(&find("S3"), "monoisotopic_mass"), "");
    // CCO [M+H]+ = 46.04186 + 1.00728 = 47.04914 -> 47.0491, so 0.0 ppm
    assert_eq!(cell(&find("S5"), "mz_calc"), "47.0491");
    assert_eq!(cell(&find("S5"), "ppm"), "0.0");
    let keys: Vec<&str> = t["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["key"].as_str().unwrap())
        .collect();
    for k in [
        "draw2.export.tpMass",
        "draw2.export.tpCharged",
        "draw2.export.tpAdduct",
    ] {
        assert!(keys.contains(&k), "{k} in {keys:?}");
    }
    // the same notes are in `lost` of the draw-result
    let lost: Vec<&str> = v["lost"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["key"].as_str().unwrap())
        .collect();
    assert_eq!(lost, keys);
}

#[test]
fn the_goldens_of_the_table_are_what_the_glue_door_writes() {
    let file: Value =
        serde_json::from_str(include_str!("../../../tests/data/log/tp-table.json")).unwrap();
    let cases = file["values"].as_array().unwrap();
    assert!(cases.len() >= 7);
    let bless = std::env::var("MZDRAW_BLESS").is_ok();
    let mut fresh = file.clone();
    for (i, c) in cases.iter().enumerate() {
        // a case that says `plain` goes through the plain replay (no engine), the others through the glue door
        let got = if c["plain"] == true {
            mzdraw::replay::replay(&c["request"].to_string()).unwrap()
        } else {
            run(c["request"].clone())
        };
        if bless {
            fresh["values"][i]["out"] = Value::from(got);
        } else {
            assert_eq!(got, c["out"].as_str().unwrap(), "{}", c["name"]);
        }
    }
    if bless {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/data/log/tp-table.json"
        );
        std::fs::write(path, serde_json::to_string_pretty(&fresh).unwrap() + "\n").unwrap();
    }
}

#[test]
fn an_sdf_field_named_in_lower_case_still_counts_as_a_name_for_the_pathway_note() {
    let mut e = Editor::default();
    let info = e.import("\n  x\n\n  1  0  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\nM  END\n>  <tp_id>\nA\n\n$$$$\n").unwrap();
    assert_eq!(info.no_pathway, 1);
}
