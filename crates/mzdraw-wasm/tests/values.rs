//! The calculated values with the real engine (`sel` of mzLab through the glue): masses and m/z checked against hand-computed numbers
//! to 4 decimals, the keyed refusals of the engine, the page state, and the request field through the glue door. Synthetic structures only.
//!
//! The hand sums use these masses (u): H 1.00782503207, C 12 (exact), N 14.0030740048, O 15.99491461956, Cl 34.96885268, electron
//! 0.00054857991, so [M+H]+ adds H minus an electron, 1.00727645216, and [M-H]- takes it away. Each test shows its sum.

use mzdraw_wasm::glue;
use serde_json::{json, Value};

fn values(request: Value) -> Value {
    serde_json::from_str(&glue::replay(&request.to_string()).unwrap()).unwrap()
}

fn mass_of(smiles: &str) -> (Value, Value) {
    let v = values(json!({"import": smiles, "values": true}));
    (v["structures"][0].clone(), v)
}

#[test]
fn caffeine_and_glucose_have_the_hand_computed_masses_to_four_decimals() {
    // C8H10N4O2: 8 * 12 + 10 * 1.00782503207 + 4 * 14.0030740048 + 2 * 15.99491461956 = 194.080375... -> 194.0804
    let (s, v) = mass_of("Cn1cnc2c1c(=O)n(C)c(=O)n2C");
    assert_eq!(s["formula"], "C8H10N4O2");
    assert_eq!(s["mass"], "194.0804");
    // average with the conventional weights of `sel`: 8 * 12.011 + 10 * 1.008 + 4 * 14.007 + 2 * 15.999 = 194.194
    assert_eq!(s["average"], "194.1940");
    // [M+H]+ = 194.080375 + 1.007276 = 195.087652; [M-H]- = 194.080375 - 1.007276 = 193.073099
    assert_eq!(s["mz"], json!({"[M+H]+": "195.0877", "[M-H]-": "193.0731"}));
    assert_eq!(
        (s["ionMz"].clone(), s["error"].clone()),
        (Value::Null, Value::Null)
    );
    assert_eq!(
        (v["engine"].clone(), v["error"].clone()),
        (json!(true), Value::Null)
    );
    // glucose C6H12O6: 72 + 12 * 1.00782503207 + 6 * 15.99491461956 = 180.063388 -> 180.0634
    let v = values(json!({"import": "OCC(O)C(O)C(O)C(O)C=O", "values": true,
        "set": ["display.adducts=[M+H]+,[M+Na]+,[M-H]-,[M+NH4]+,[M+K]+,[M+Cl]-,[M+HCOO]-"]}));
    let s = &v["structures"][0];
    assert_eq!(
        (s["formula"].clone(), s["mass"].clone()),
        (json!("C6H12O6"), json!("180.0634"))
    );
    assert_eq!(s["average"], "180.1560");
    // [M+H]+ 181.070665; [M+Na]+ = 180.063388 + 22.98976928 - 0.000549 = 203.052609; [M-H]- 179.056112;
    // [M+NH4]+ = 180.063388 + (14.003074 + 4 * 1.007825 = 18.034374) - 0.000549 = 198.097214;
    // [M+K]+ = 180.063388 + 38.96370668 - 0.000549 = 219.026546; [M+Cl]- = 180.063388 + 34.96885268 + 0.000549 = 215.032790;
    // [M+HCOO]- = 180.063388 + (12 + 1.007825 + 31.989829) + 0.000549 = 225.061591
    assert_eq!(
        s["mz"],
        json!({"[M+H]+": "181.0707", "[M+Na]+": "203.0526", "[M-H]-": "179.0561", "[M+NH4]+": "198.0972",
               "[M+K]+": "219.0265", "[M+Cl]-": "215.0328", "[M+HCOO]-": "225.0616"})
    );
}

#[test]
fn a_chloride_salt_and_a_charged_ammonium_give_their_own_mz_and_no_adduct() {
    // CH6N+: 12 + 6 * 1.00782503207 + 14.0030740048 = 32.050024 (atoms); its m/z takes an electron off: 32.049476
    let (s, v) = mass_of("C[NH3+].[Cl-]");
    assert_eq!(v["structures"].as_array().unwrap().len(), 2);
    assert_eq!(
        (s["formula"].clone(), s["charge"].clone()),
        (json!("CH6N+"), json!(1))
    );
    assert_eq!(
        (s["mass"].clone(), s["ionMz"].clone()),
        (json!("32.0500"), json!("32.0495"))
    );
    assert_eq!(s["mz"], json!({"[M+H]+": null, "[M-H]-": null}));
    // Cl-: 34.96885268 as drawn; the m/z of the anion adds the electron: 34.969401
    let c = &v["structures"][1];
    assert_eq!(
        (c["formula"].clone(), c["charge"].clone()),
        (json!("Cl\u{2212}"), json!(-1))
    );
    assert_eq!(
        (c["mass"].clone(), c["ionMz"].clone()),
        (json!("34.9689"), json!("34.9694"))
    );
    // tetramethylammonium C4H12N+: 48 + 12 * 1.00782503207 + 14.0030740048 = 74.096974; m/z 74.096426
    let (t, _) = mass_of("C[N+](C)(C)C");
    assert_eq!(
        (t["formula"].clone(), t["mass"].clone(), t["ionMz"].clone()),
        (json!("C4H12N+"), json!("74.0970"), json!("74.0964"))
    );
    // a doubly charged structure divides by the charge: sulfate O4S 2-: 4 * 15.99491461956 + 31.97207117 = 95.951630 atoms; m/z (95.951630 + 2 * 0.000549) / 2
    let (u, _) = mass_of("[O-]S(=O)(=O)[O-]");
    assert_eq!(
        (u["mass"].clone(), u["ionMz"].clone()),
        (json!("95.9517"), json!("47.9764"))
    );
}

#[test]
fn a_selection_is_one_structure_with_its_cut_bonds_closed_with_hydrogen() {
    // the methyl of ethanol with its bond closed: CH4 = 12 + 4 * 1.00782503207 = 16.031300
    let v = values(json!({"import": "CCO", "values": {"atoms": [1]}}));
    let s = &v["selection"];
    assert_eq!(
        (s["formula"].clone(), s["mass"].clone()),
        (json!("CH4"), json!("16.0313"))
    );
    // the whole molecule selected has no cut: C2H6O 46.041865 (24 + 6.046950 + 15.994915)
    let v = values(json!({"import": "CCO", "values": {"atoms": [1, 2, 3]}}));
    assert_eq!(v["selection"]["mass"], "46.0419");
    assert_eq!(v["selection"]["mass"], v["structures"][0]["mass"]);
    // two atoms that are not joined are one unit: CH4 and H2O, 16.031300 + 18.010565 = 34.041865
    let v = values(json!({"import": "CCO", "values": {"atoms": [1, 3]}}));
    assert_eq!(
        (
            v["selection"]["formula"].clone(),
            v["selection"]["mass"].clone()
        ),
        (json!("CH6O"), json!("34.0419"))
    );
}

#[test]
fn what_the_engine_cannot_read_is_a_key_and_no_number_and_the_rest_is_kept() {
    // sodium acetate: the engine has no valence rule for Na, so the ion has a key; the acetate keeps its numbers
    let v = values(json!({"import": "CC(=O)[O-].[Na+]", "values": true}));
    let (acetate, sodium) = (&v["structures"][0], &v["structures"][1]);
    assert_eq!(acetate["mass"], "59.0133");
    assert_eq!(sodium["formula"], "Na+");
    assert_eq!(sodium["error"], "error.grafo.element");
    assert_eq!(
        (
            sodium["mass"].clone(),
            sodium["ionMz"].clone(),
            sodium["mz"].clone()
        ),
        (Value::Null, Value::Null, json!({}))
    );
    // an R group is open: no mass, and its own key
    let v = values(
        json!({"import": r#"{"format":"mzdraw/1","next":10,"atoms":[{"id":1,"el":"C","x":0,"y":0},{"id":2,"el":"C","x":1,"y":0,"lab":"R1"}],"bonds":[{"id":3,"a":1,"b":2,"o":1}]}"#, "values": true}),
    );
    assert_eq!(v["structures"][0]["error"], "error.draw2.valuesOpen");
}

#[test]
fn the_caps_leave_the_formulas_and_say_why() {
    // 201 one-atom structures: above the cap of 200, no engine call, every formula there, the key on the document
    let atoms: Vec<Value> = (1..=201)
        .map(|i| json!({"id": i, "el": "C", "x": 2 * i, "y": 0}))
        .collect();
    let doc = json!({"format": "mzdraw/1", "next": 300, "atoms": atoms, "bonds": []});
    let v = values(json!({"import": doc.to_string(), "values": true}));
    assert_eq!(v["error"], "error.draw2.valuesSize");
    assert_eq!(v["structures"].as_array().unwrap().len(), 201);
    assert_eq!(v["structures"][0]["formula"], "CH4");
    assert_eq!(v["structures"][0]["mass"], Value::Null);
}

#[test]
fn an_export_through_the_glue_door_draws_the_masses_and_the_plain_door_does_not() {
    let request =
        json!({"import": "OCC(O)C(O)C(O)C(O)C=O", "set": ["display.values=mass"], "out": "svg"})
            .to_string();
    // no `engines` key: the plain replay with the engine attached
    let with = glue::replay(&request).unwrap();
    assert!(with.contains(">C6H12O6  180.0634</text>"));
    let without = mzdraw::replay::replay(&request).unwrap();
    assert!(without.contains(">C6H12O6</text>") && !without.contains("180.0634"));
    // with `engines` the old answer shape is kept
    let both = glue::replay(&json!({"import": "CCO", "engines": [{"engine": "sel"}]}).to_string())
        .unwrap();
    assert!(both.starts_with(r#"{"engines":[{"engine":"sel","answer":"#));
}

#[test]
fn the_page_state_carries_the_values_of_each_structure_and_of_the_selection() {
    mzdraw_wasm::add("Cn1cnc2c1c(=O)n(C)c(=O)n2C.CCO").unwrap();
    let st: Value = serde_json::from_str(&mzdraw_wasm::state()).unwrap();
    let vs = &st["values"];
    assert_eq!(vs["adducts"], json!(["[M+H]+", "[M-H]-"]));
    assert_eq!(
        vs["structures"].as_array().unwrap().len(),
        st["outline"].as_array().unwrap().len()
    );
    assert_eq!(vs["structures"][0]["mass"], "194.0804");
    assert_eq!(vs["structures"][1]["mass"], "46.0419");
    assert_eq!(vs["selection"], Value::Null);
    // select all: the whole drawing is the selection, one unit
    mzdraw_wasm::command("all");
    let st: Value = serde_json::from_str(&mzdraw_wasm::state()).unwrap();
    let sel = &st["values"]["selection"];
    assert_eq!(sel["formula"], "C10H16N4O3");
    assert_eq!(sel["mass"], "240.1222");
    // a new adduct list changes what the engine is asked, at once
    mzdraw_wasm::set("display.adducts=[M+Na]+").unwrap();
    let st: Value = serde_json::from_str(&mzdraw_wasm::state()).unwrap();
    assert_eq!(
        st["values"]["structures"][1]["mz"],
        json!({"[M+Na]+": "69.0311"})
    );
    // another selection is asked for without a change of the drawing
    mzdraw_wasm::key("Escape", 0, 0.0);
    let st: Value = serde_json::from_str(&mzdraw_wasm::state()).unwrap();
    assert_eq!(st["values"]["selection"], Value::Null);
}

/// The golden file `tests/data/log/values.json`: each request through the glue door (or, with `"plain": true`, the plain core replay) gives
/// the recorded bytes. `tests/parity.mjs` holds the `.wasm` to the same bytes. `MZDRAW_BLESS=1 cargo test -p mzdraw-wasm --test values`
/// rewrites them after a wanted change.
#[test]
fn the_golden_requests_give_the_recorded_bytes() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/log/values.json");
    let mut file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let bless = std::env::var("MZDRAW_BLESS").is_ok();
    let cases = file["values"].as_array_mut().unwrap();
    assert!(cases.len() >= 9);
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
fn a_structure_of_more_than_200_atoms_is_a_key_and_no_number_and_200_has_its_mass() {
    let chain = |n: usize| "C".repeat(n);
    for (n, ok) in [(200, true), (201, false), (250, false)] {
        let v = values(json!({"import": chain(n), "values": true}));
        let s = &v["structures"][0];
        assert_eq!(s["formula"], format!("C{n}H{}", 2 * n + 2), "{n}");
        if ok {
            // 2400 + 402 * 1.00782503207 (405.145663) = 2805.145663 (a chain of 200 carbons, C200H402)
            assert_eq!(s["mass"], "2805.1457", "{n}");
            assert_eq!(s["error"], Value::Null);
        } else {
            assert_eq!(s["error"], "error.grafo.tooBig", "{n}");
            assert_eq!(
                (s["mass"].clone(), s["average"].clone(), s["mz"].clone()),
                (Value::Null, Value::Null, json!({})),
                "{n}"
            );
        }
    }
}

#[test]
fn every_key_the_engine_can_refuse_with_has_a_text_in_the_catalog() {
    // an explicit hydrogen atom (deuterium) and a metal ion: the engine's keys reach the panel, which shows the catalog text of the key
    let v = values(json!({"import": "[2H]C([2H])([2H])O.CC(=O)[O-].[Na+]", "values": true}));
    let keys: Vec<&str> = v["structures"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["error"].as_str())
        .collect();
    assert_eq!(keys, ["error.grafo.element", "error.grafo.element"]);
    let lang = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/lang/mzdraw.js"),
    )
    .unwrap();
    // The engine keys: every `error.grafo.*`, `error.sel.*` and `error.label.*` constant of the pinned mzlab-feat sources that the `sel` path
    // reads (chem/graph.rs, chem/sel.rs, chem/labels.rs, chem/smiles.rs), found with
    //   grep -ho '"error\.\(grafo\|sel\|label\)\.[A-Za-z.]*"' graph.rs graph_record.rs sel.rs labels.rs smiles.rs structures.rs
    // in the cargo git checkout of rev ccd4680 (11 keys), then ours. Some cannot be reached from a molfile (`mixture`, `smiles`): they are
    // kept so that a later pin that reaches them still shows words. REDO THE GREP WHEN THE PIN MOVES and add what is new.
    for key in [
        "error.grafo.delta",
        "error.grafo.delta.neg",
        "error.grafo.element",
        "error.grafo.mixture",
        "error.grafo.molfile",
        "error.grafo.smiles",
        "error.grafo.tooBig",
        "error.label.element",
        "error.label.isotope",
        "error.label.request",
        "error.sel.request",
        "error.draw2.valuesOpen",
        "error.draw2.valuesSize",
        "error.draw2.valuesAlone",
        "error.draw2.valuesEngine",
        "error.draw2.valuesAdduct",
    ] {
        let line = lang
            .lines()
            .find(|l| l.trim_start().starts_with(&format!("\"{key}\":")))
            .unwrap_or_else(|| panic!("no text for {key}"));
        assert!(
            line.len() > key.len() + 20 && !line.contains(": \"error."),
            "{key}: {line}"
        );
    }
}

/// The best of five runs of `f`, in microseconds.
fn best(f: impl Fn()) -> u128 {
    (0..5)
        .map(|_| {
            let t = std::time::Instant::now();
            f();
            t.elapsed().as_micros()
        })
        .min()
        .unwrap()
}

#[test]
fn the_glue_path_with_the_real_engine_scales_linearly_in_the_structures() {
    // A sanity check of the glue path, not the proof of linearity: the real `sel` calls dominate the time here, and the scan mutations are
    // caught by the unit tests of crates/mzdraw/tests/values.rs. 25 structures against 100 (four times): import, scan, one `sel` call per
    // structure and the answer.
    let ask = |n: usize| json!({"import": vec!["CCO"; n].join("."), "values": true}).to_string();
    let (small, large) = (ask(25), ask(100));
    let run = |r: &str| {
        let v: Value = serde_json::from_str(&glue::replay(r).unwrap()).unwrap();
        assert_eq!(v["error"], Value::Null);
    };
    let (a, b) = (best(|| run(&small)), best(|| run(&large)));
    assert!(
        b <= 8 * a.max(1),
        "25 structures {a} us, 100 structures {b} us"
    );
}

#[test]
fn the_mass_shift_of_a_step_delta_is_solved_from_the_engine_alone() {
    // tp-11: the step field matches a mass shift. The core has no table: `element_masses` asks the engine for CH4, C2H6 and five
    // hydrides and solves them. Hand sums with the masses above: +O 15.9949, -CH2 -(12 + 2 * 1.00782503207) = -14.0157,
    // +H2O 18.0106, -CO2 -(12 + 2 * 15.99491461956) = -43.9898.
    let m = mzdraw::values::element_masses(glue::measure).expect("the engine answers");
    let four = |d: &str| mzdraw::values::four(mzdraw::values::delta_mass(d, &m).unwrap());
    assert_eq!(four("+O"), "15.9949");
    assert_eq!(four("-CH2"), "-14.0157");
    assert_eq!(four("+H2O"), "18.0106");
    assert_eq!(four("-CO2"), "-43.9898");
    assert_eq!(four("+H-Cl"), "-33.9610");
}
