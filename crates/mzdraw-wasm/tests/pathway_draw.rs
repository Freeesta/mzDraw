//! The pathway checks with the real engine (`sel` of mzLab through the glue): masses, expected ions per polarity, ppm and delta masses
//! against hand sums, the golden requests of `tests/data/log/pathway-draw.json` through the glue door (section `values`, or the plain
//! replay with `"plain": true`), and what the outputs hold. Synthetic structures only.
//!
//! The hand sums (u): H 1.00782503207, C 12 (exact), O 15.99491461956, electron 0.00054857991, so [M+H]+ adds H minus an electron
//! (1.00727645216) and [M-H]- takes it away.

use mzdraw_wasm::glue;
use serde_json::{json, Value};

const MODEL: &str = include_str!("../../../tests/data/pathway-draw/oxidation.json");

fn model() -> Value {
    serde_json::from_str(MODEL).unwrap()
}

fn check(m: &Value, args: Option<Value>) -> Value {
    let mut r = json!({"commands": [{"command": "pathway:build", "args": {"model": m}}], "out": "pathway-check"});
    if let Some(a) = args {
        // `args` belongs to `out` only without `commands`: the drawing goes through `import`
        let doc =
            glue::replay(&json!({"commands": r["commands"], "out": "mzdraw"}).to_string()).unwrap();
        r = json!({"import": doc, "out": "pathway-check", "args": a});
    }
    serde_json::from_str(&glue::replay(&r.to_string()).unwrap()).unwrap()
}

fn node<'a>(c: &'a Value, id: &str) -> &'a Value {
    c["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap()
}

#[test]
fn the_masses_ions_and_delta_masses_are_the_hand_sums_to_four_decimals() {
    let c = check(&model(), None);
    assert_eq!(
        (c["engine"].clone(), c["ok"].clone()),
        (json!(true), json!(true)),
        "{c}"
    );
    assert_eq!(c["findings"], json!([]));
    // toluene C7H8: 84 + 8 * 1.00782503207 = 92.0626003 ; [M+H]+ 93.0698767 ; [M-H]- 91.0553239
    let p = node(&c, "P");
    assert_eq!(p["monoisotopic_mass"], json!(92.0626));
    assert_eq!(
        p["ions"],
        json!([{"adduct": "[M+H]+", "mz": 93.0699}, {"adduct": "[M-H]-", "mz": 91.0553}])
    );
    // benzyl alcohol C7H8O: + 15.9949146 = 108.0575149
    assert_eq!(node(&c, "M1")["monoisotopic_mass"], json!(108.0575));
    // benzoic acid C7H6O2: 84 + 6 * 1.00782503 + 2 * 15.99491462 = 122.0367794 ; [M-H]- 121.0295
    let m2 = node(&c, "M2");
    assert_eq!(m2["monoisotopic_mass"], json!(122.0368));
    assert_eq!(
        m2["observed"],
        json!({"adduct": "[M-H]-", "mz": 121.0295, "expected": 121.0295, "ppm": 0.0})
    );
    // +O = 15.9949146 ; +O-H2 = 15.9949146 - 2.0156501 = 13.9792645
    let d: Vec<f64> = c["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["delta_mass"].as_f64().unwrap())
        .collect();
    assert_eq!(d, [15.9949, 13.9793, 15.9949]);
    // a node with no observed ion has none
    assert!(node(&c, "M3")["observed"].is_null());
}

#[test]
fn the_ppm_follows_the_unrounded_expected_value_and_the_printed_numbers_agree_within_their_rounding(
) {
    let c = check(&model(), None);
    for id in ["P", "M1", "M2"] {
        let o = &node(&c, id)["observed"];
        let (mz, ex) = (o["mz"].as_f64().unwrap(), o["expected"].as_f64().unwrap());
        // the expected value is printed to 4 decimals (an error of at most 0.00005), the ppm is the unrounded one: the two agree to that
        let from_printed = (mz - ex) / ex * 1e6;
        let slack = 0.00005 / ex * 1e6 + 0.06;
        assert!(
            (o["ppm"].as_f64().unwrap() - from_printed).abs() <= slack,
            "{id}: {o}"
        );
    }
}

/// One node of the given structure, observed m/z and adduct, checked with the engine.
fn one_ion(smiles: &str, ion: Option<&str>, mz: f64, args: Option<Value>) -> Value {
    let mut n = json!({"id": "x", "smiles": smiles, "mz": mz});
    if let Some(i) = ion {
        n["ion"] = json!(i);
    }
    check(&json!({"format": "pathway/1", "nodes": [n]}), args)
}

#[test]
fn the_verdict_uses_the_unrounded_expected_mz_only_the_print_is_rounded() {
    // acetic acid [M-H]-: 60.021129 - 1.007276 = 59.013853 (printed 59.0139). Observed 59.0133 is -9.4 ppm from the true value, inside the
    // default 10 ppm; against the rounded 59.0139 it was -10.2 and raised an error
    let c = one_ion("CC(=O)O", Some("[M-H]-"), 59.0133, None);
    assert_eq!(c["ok"], json!(true), "{c}");
    let o = &node(&c, "x")["observed"];
    assert_eq!(
        (o["expected"].clone(), o["ppm"].clone()),
        (json!(59.0139), json!(-9.4))
    );
    // the other side of the same line: 10.2 ppm from the true value is an error
    let c = one_ion("CC(=O)O", Some("[M-H]-"), 59.0132, None);
    assert_eq!(c["ok"], json!(false), "{c}");
    // a low m/z where the rounding goes the other way: acetamide [M-H]-, true 58.029837 (printed 58.0298, below it). Observed 58.03039
    // is +9.5 ppm from the true value (inside) and +10.2 from the rounded one (outside)
    let c = one_ion("CC(N)=O", Some("[M-H]-"), 58.03039, None);
    assert_eq!(c["ok"], json!(true), "{c}");
    assert_eq!(node(&c, "x")["observed"]["expected"], json!(58.0298));
    assert_eq!(node(&c, "x")["observed"]["ppm"], json!(9.5));
    // and without a typed adduct, the same line decides which ion matches
    assert_eq!(
        node(&one_ion("CC(=O)O", None, 59.0133, None), "x")["observed"]["adduct"],
        "[M-H]-"
    );
}

#[test]
fn a_charged_structure_is_its_own_ion_and_a_correct_observed_m_plus_matches() {
    // tetramethylammonium C4H12N+: 4 * 12 + 12 * 1.00782503 + 14.00307400 = 74.0969744 (atoms), minus one electron 0.0005486 = 74.0964258
    let c = one_ion("C[N+](C)(C)C", None, 74.0964, None);
    assert_eq!(c["ok"], json!(true), "{c}");
    assert!(
        !c["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["key"] == "pathway.ionUnmatched"),
        "{c}"
    );
    let x = node(&c, "x");
    assert_eq!(
        x["ions"],
        json!([{"adduct": "[M]+", "mz": 74.0964}]),
        "no electron counted twice, no adduct added"
    );
    assert_eq!(x["observed"]["adduct"], "[M]+");
    assert!(x["observed"]["ppm"].as_f64().unwrap().abs() < 1.0);
    // typed as its own ion it is checked too; a wrong value is an error
    assert_eq!(
        one_ion("C[N+](C)(C)C", Some("[M]+"), 74.0964, None)["ok"],
        json!(true)
    );
    assert_eq!(
        one_ion("C[N+](C)(C)C", Some("[M]+"), 74.1, None)["ok"],
        json!(false)
    );
    // an adduct typed on a charged structure is said, not computed
    let c = one_ion("C[N+](C)(C)C", Some("[M+H]+"), 75.1, None);
    assert!(
        c["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["key"] == "pathway.ionNotComputed"),
        "{c}"
    );
    // an anion is its own ion as well: acetate (59.0139 as a charged species, the same number as [M-H]- of the acid)
    let c = one_ion("CC(=O)[O-]", None, 59.0139, None);
    assert_eq!(node(&c, "x")["observed"]["adduct"], "[M]-", "{c}");
    // the polarity filter: the cation has no ion in negative polarity
    assert_eq!(
        node(
            &one_ion(
                "C[N+](C)(C)C",
                None,
                74.0964,
                Some(json!({"polarity": "negative"}))
            ),
            "x"
        )["ions"],
        json!([])
    );
}

#[test]
fn the_delta_mass_between_a_neutral_and_its_cation_is_the_atoms_not_the_electrons() {
    let m = json!({"format": "pathway/1",
        "nodes": [{"id": "a", "smiles": "CN(C)C"}, {"id": "b", "smiles": "C[N+](C)(C)C", "mz": 74.0964}],
        "edges": [{"from": "a", "to": "b", "label": "+CH3"}]});
    let c = check(&m, None);
    assert_eq!(c["ok"], json!(true), "{c}");
    // CH3 = 12 + 3 * 1.00782503 = 15.0234751; the same number as the formula delta, no electron in it
    assert_eq!(c["edges"][0]["delta_mass"], json!(15.0235));
    // a numeric label is compared with it
    let mut m2 = m.clone();
    m2["edges"][0]["label"] = json!("+15.0235");
    assert_eq!(check(&m2, None)["edges"][0]["result"], "ok");
}

#[test]
fn a_numeric_label_uses_the_masses_even_when_the_two_structures_have_no_honest_formula_delta() {
    // a methyl radical and the methyl cation have the same atoms and another net charge: no formula delta (`delta` null), but the masses exist
    let m = json!({"format": "pathway/1",
        "nodes": [{"id": "a", "smiles": "[CH3]"}, {"id": "b", "smiles": "[CH3+]"}],
        "edges": [{"from": "a", "to": "b", "label": "+0.0"}]});
    let c = check(&m, None);
    let e = &c["edges"][0];
    assert_eq!(
        (e["delta"].clone(), e["result"].clone()),
        (Value::Null, json!("ok")),
        "{c}"
    );
    // without a number the same pair stays unchecked (a formula label has nothing to be compared with)
    let mut m2 = m.clone();
    m2["edges"][0]["label"] = json!("+H");
    assert_eq!(check(&m2, None)["edges"][0]["result"], "unchecked");
}

#[test]
fn with_the_engine_the_summary_says_checked_and_nothing_is_unchecked() {
    let c = check(&model(), None);
    assert_eq!(
        (c["checked"].clone(), c["counts"]["unchecked"].clone()),
        (json!(true), json!(0)),
        "{c}"
    );
}

#[test]
fn a_wrong_observed_mz_and_a_wrong_label_fail_with_the_right_keys() {
    let mut m = model();
    m["edges"][2]["label"] = json!("-CH2");
    m["nodes"][2]["mz"] = json!(121.05); // 121.0295 expected: 168 ppm
    let c = check(&m, None);
    assert_eq!(c["ok"], json!(false));
    let keys: Vec<&str> = c["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["pathway.ionMismatch", "pathway.edgeMismatch"]);
    // the tolerance is the caller's: 200 ppm lets the ion pass, the label still fails
    let c = check(&m, Some(json!({"ppm": 200})));
    let keys: Vec<&str> = c["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["pathway.edgeMismatch"]);
}

#[test]
fn numeric_labels_are_compared_with_the_delta_mass_within_the_tolerance() {
    let mut m = model();
    m["edges"][0]["label"] = json!("+15.9949");
    m["edges"][1]["label"] = json!("+14.0"); // the true delta is 13.9793
    let c = check(&m, None);
    let r: Vec<&str> = c["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["result"].as_str().unwrap())
        .collect();
    assert_eq!(r, ["ok", "mismatch", "ok"]);
    // a wider tolerance accepts it
    let c = check(&m, Some(json!({"da": 0.05})));
    assert_eq!(c["edges"][1]["result"], "ok");
}

#[test]
fn polarity_filters_the_expected_ions() {
    for (pol, want) in [
        ("positive", vec!["[M+H]+"]),
        ("negative", vec!["[M-H]-"]),
        ("both", vec!["[M+H]+", "[M-H]-"]),
    ] {
        let c = check(&model(), Some(json!({"polarity": pol})));
        let got: Vec<&str> = node(&c, "P")["ions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["adduct"].as_str().unwrap())
            .collect();
        assert_eq!(got, want, "{pol}");
    }
}

#[test]
fn an_observed_mz_with_no_adduct_is_matched_to_an_expected_ion_of_the_polarity() {
    let mut m = model();
    m["nodes"][0].as_object_mut().unwrap().remove("ion");
    let c = check(&m, None);
    assert_eq!(node(&c, "P")["observed"]["adduct"], "[M+H]+");
    // and one that fits none is a warning, not an error
    m["nodes"][0]["mz"] = json!(500.0);
    let c = check(&m, None);
    assert_eq!(c["ok"], json!(true));
    assert!(c["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["key"] == "pathway.ionUnmatched"));
}

#[test]
fn an_adduct_the_engine_has_no_shift_for_is_an_info_not_an_error() {
    let mut m = model();
    m["nodes"][0]["ion"] = json!("[M+H-H2O]+");
    let c = check(&m, None);
    assert_eq!(c["ok"], json!(true));
    assert!(c["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["key"] == "pathway.ionNotComputed" && f["item"] == "P"));
    // a node whose ion could not be computed is counted as unchecked
    assert_eq!(c["counts"]["unchecked"], json!(1), "{c}");
}

#[test]
fn the_goldens_are_what_the_glue_door_writes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/log/pathway-draw.json");
    let mut file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let bless = std::env::var("MZDRAW_BLESS").is_ok();
    let cases = file["values"].as_array_mut().unwrap();
    assert!(cases.len() >= 8);
    for case in cases.iter_mut() {
        let request = case["request"].to_string();
        let got = if case["plain"] == true {
            mzdraw::replay::replay(&request).unwrap()
        } else {
            glue::replay(&request).unwrap()
        };
        if bless {
            case["out"] = json!(got);
        }
        assert_eq!(case["out"], json!(got), "{}", case["name"]);
    }
    // the engine was there in the glue cases of the check, and not in the plain one
    let engine = |i: usize| {
        serde_json::from_str::<Value>(cases[i]["out"].as_str().unwrap()).unwrap()["engine"].clone()
    };
    assert_eq!(
        (engine(0), engine(1), engine(7)),
        (json!(true), json!(true), json!(false))
    );
    if bless {
        std::fs::write(&path, serde_json::to_string_pretty(&file).unwrap() + "\n").unwrap();
    }
}

#[test]
fn the_node_mass_of_the_check_is_the_mass_of_the_shared_engine_sel_for_the_same_molfile() {
    // the check holds no mass code: its number is what `sel` of mzLab gives for the structure, rounded to 4 decimals
    for smiles in ["Cc1ccccc1", "OC(=O)c1ccccc1", "Cn1cnc2c1c(=O)n(C)c(=O)n2C"] {
        let molfile =
            glue::replay(&json!({"import": smiles, "out": "molfile"}).to_string()).unwrap();
        let sel: Value = serde_json::from_str(
            &mzlab_feat::chem::sel::sel_json(&json!({"molfile": molfile}).to_string()).unwrap(),
        )
        .unwrap();
        let want = sel["label"]["mass"].as_f64().unwrap();
        let c: Value = serde_json::from_str(
            &glue::replay(
                &json!({"commands": [{"command": "pathway:build", "args": {"model":
                    {"format": "pathway/1", "nodes": [{"id": "N", "smiles": smiles}], "edges": []}}}],
                    "out": "pathway-check"})
                .to_string(),
            )
            .unwrap(),
        )
        .unwrap();
        let got = c["nodes"][0]["monoisotopic_mass"].as_f64().unwrap();
        assert_eq!(got, (want * 1e4).round() / 1e4, "{smiles}: {c}");
        // the check's formula text is the Hill text of the one shared writer
        assert_eq!(
            c["nodes"][0]["formula"], sel["label"]["formula"],
            "{smiles}"
        );
    }
}
