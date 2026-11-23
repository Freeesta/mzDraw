//! The lines kept under a molecule with the real engine (the glue of mzLab): elemental analysis against hand sums, the m/z lines, the
//! request field `props` through the glue door, and the golden file `tests/data/log/struct-props.json`. Synthetic structures only.

use mzdraw_wasm::glue;
use serde_json::{json, Value};

fn props(request: Value) -> Value {
    serde_json::from_str(&glue::replay(&request.to_string()).unwrap()).unwrap()
}

#[test]
fn the_elemental_analysis_is_the_mass_per_cent_of_the_conventional_weights() {
    // caffeine C8H10N4O2, weights of `sel` (C 12.011, H 1.008, N 14.007, O 15.999): MW 194.194
    //   C 8 * 12.011 = 96.088 -> 49.4804 %   H 10 * 1.008 = 10.08 -> 5.1909 %
    //   N 4 * 14.007 = 56.028 -> 28.8514 %   O 2 * 15.999 = 31.998 -> 16.4772 %
    let v = props(json!({"import": "Cn1cnc2c1c(=O)n(C)c(=O)n2C", "props": true}));
    assert_eq!(v["engine"], true);
    let s = &v["structures"][0]["values"];
    assert_eq!(
        s["ea"],
        json!([
            ["%C", "49.48"],
            ["%H", "5.19"],
            ["%N", "28.85"],
            ["%O", "16.48"]
        ])
    );
    assert_eq!(s["mw"], json!([["MW", "194.1940"]]));
    assert_eq!(s["exact"], json!([["Exact mass", "194.0804"]]));
    assert_eq!(
        s["mz"],
        json!([["[M+H]+", "195.0877"], ["[M-H]-", "193.0731"]])
    );
    // the descriptors of the library: caffeine has no donor, three acceptors, no rotatable bond (a methyl on a ring nitrogen is not one)
    assert_eq!(s["hbd"], json!([["HBD", "0"]]));
    assert_eq!(s["hba"], json!([["HBA", "3"]]));
    assert_eq!(s["rotb"], json!([["Rotatable bonds", "0"]]));
    assert_eq!(s["heavy"], json!([["Heavy atoms", "14"]]));
    assert_eq!(s["tpsa"], json!([["tPSA", "61.82"]]));
}

#[test]
fn the_per_cent_of_a_selection_is_that_of_its_closed_structure() {
    // the methyl of ethanol, closed with hydrogen, is CH4: C 12.011 / 16.043 = 74.8738 %, H 4.032 / 16.043 = 25.1325 %
    let v = props(json!({"import": "CCO", "props": {"atoms": [1]}}));
    let s = &v["selection"];
    assert_eq!(s["formula"], "CH4");
    assert_eq!(s["values"]["ea"], json!([["%C", "74.87"], ["%H", "25.13"]]));
    assert_eq!(s["values"]["exact"], json!([["Exact mass", "16.0313"]]));
}

#[test]
fn a_charged_structure_has_its_own_mz_and_no_adduct_and_a_structure_with_an_r_group_no_number() {
    let v = props(json!({"import": "C[NH3+]", "props": true}));
    let s = &v["structures"][0]["values"];
    assert_eq!(s["mz"], json!([["m/z", "32.0495"]]));
    let v = props(json!({"import": "C[*]", "props": true}));
    let b = &v["structures"][0];
    // no engine number and no descriptor: a structure with an R group is not a molecule yet
    assert_eq!(b["error"], "error.draw2.valuesOpen");
    assert_eq!(b["values"]["exact"], json!([]));
    assert_eq!(b["values"]["logp"], json!([]));
}

/// The golden file `tests/data/log/struct-props.json`, section `values`: each request through the glue door (or, with `"plain": true`, the
/// plain core replay) gives the recorded bytes; `tests/parity.mjs` holds the `.wasm` to the same bytes. `MZDRAW_BLESS=1` rewrites them.
#[test]
fn the_golden_requests_give_the_recorded_bytes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/log/struct-props.json");
    let mut file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let bless = std::env::var("MZDRAW_BLESS").is_ok();
    let cases = file["values"].as_array_mut().unwrap();
    assert!(cases.len() >= 6);
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
    if bless {
        std::fs::write(&path, serde_json::to_string_pretty(&file).unwrap() + "\n").unwrap();
    }
}

#[test]
fn an_element_the_engine_does_not_know_and_a_structure_above_its_limit_leave_no_number() {
    // dimethylselenide and a bare sodium ion: the engine refuses the element, so no engine line and no descriptor number either
    for smiles in ["C[Se]C", "[Na+]"] {
        let v = props(json!({"import": smiles, "props": true}));
        let b = &v["structures"][0];
        assert_eq!(b["error"], "error.grafo.element", "{smiles}");
        assert_eq!(b["propsError"], "error.grafo.element", "{smiles}");
        for k in [
            "mw", "exact", "ea", "mz", "logp", "tpsa", "hbd", "hba", "rotb",
        ] {
            assert_eq!(b["values"][k], json!([]), "{smiles} {k}");
        }
    }
    // a chain of 1000 carbons: the engine reads it (V3000 since mzLab #3657) and refuses it with its own size key `error.grafo.tooBig`,
    // nothing written (before that pin it read the V3000 as nothing and gave a mass of zero: `error.draw2.valuesZero`)
    for n in [1000usize, 4900] {
        let v = props(json!({"import": "C".repeat(n), "props": true}));
        let b = &v["structures"][0];
        assert_eq!(b["error"], "error.grafo.tooBig", "{n}");
        assert_eq!(b["values"]["exact"], json!([]), "{n}");
        assert_eq!(b["values"]["logp"], json!([]), "{n}");
        let t =
            glue::replay(&json!({"import": "C".repeat(n), "props": {"text": "line"}}).to_string())
                .unwrap();
        assert!(
            !t.contains("0.0000") && t.contains("note error.grafo.tooBig"),
            "{n}: {t}"
        );
        // the panel's values/1 (arch-web-17) no longer says 0.0000 either
        let v = props(json!({"import": "C".repeat(n), "values": true}));
        assert_eq!(v["structures"][0]["error"], "error.grafo.tooBig", "{n}");
        assert_eq!(v["structures"][0]["mass"], Value::Null, "{n}");
    }
}

#[test]
fn a_charged_structure_through_the_engine_has_its_engine_numbers_and_no_descriptor() {
    // chloride, methylammonium, acetate, glycine zwitterion (net charge zero, two charged atoms): the engine gives the formula, the mass
    // and the per cent (they are right for ions), the descriptors say `error.draw2.propsCharged` and are empty
    for smiles in ["[Cl-]", "C[NH3+]", "CC(=O)[O-]", "[NH3+]CC(=O)[O-]"] {
        let v = props(json!({"import": smiles, "props": true}));
        assert_eq!(v["engine"], true, "{smiles}");
        let b = &v["structures"][0];
        assert_eq!(b["error"], Value::Null, "{smiles}");
        assert_eq!(b["propsError"], "error.draw2.propsCharged", "{smiles}");
        for k in ["logp", "tpsa", "hbd", "hba", "rotb", "heavy"] {
            assert_eq!(b["values"][k], json!([]), "{smiles} {k}");
        }
        for k in ["formula", "exact", "ea"] {
            assert!(
                !b["values"][k].as_array().unwrap().is_empty(),
                "{smiles} {k}"
            );
        }
    }
    // the per cent of the zwitterion C2H5NO2 (conventional weights 12.011, 1.008, 14.007, 15.999; 75.067): C 24.0220 / 75.067 = 32.00 %
    let z = props(json!({"import": "[NH3+]CC(=O)[O-]", "props": true}));
    assert_eq!(
        z["structures"][0]["values"]["ea"][0],
        json!(["%C", "32.00"])
    );
    // the neutral control keeps every number
    let c = props(json!({"import": "CCO", "props": true}));
    let b = &c["structures"][0];
    assert_eq!(
        (&b["error"], &b["propsError"]),
        (&Value::Null, &Value::Null)
    );
    for k in ["logp", "tpsa", "hbd", "hba", "rotb", "heavy"] {
        assert!(!b["values"][k].as_array().unwrap().is_empty(), "{k}");
    }
    // sodium has no engine entry at all: the engine's key, not the core's element key
    let na = props(json!({"import": "[Na+]", "props": true}));
    assert_eq!(na["structures"][0]["propsError"], "error.grafo.element");
}
