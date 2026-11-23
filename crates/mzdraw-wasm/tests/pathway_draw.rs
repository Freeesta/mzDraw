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
fn the_ppm_is_recomputed_from_the_printed_numbers() {
    let c = check(&model(), None);
    for id in ["P", "M1", "M2"] {
        let o = &node(&c, id)["observed"];
        let (mz, ex) = (o["mz"].as_f64().unwrap(), o["expected"].as_f64().unwrap());
        let ppm = ((mz - ex) / ex * 1e6 * 10.0).round() / 10.0;
        assert_eq!(o["ppm"].as_f64().unwrap(), ppm + 0.0, "{id}");
    }
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
