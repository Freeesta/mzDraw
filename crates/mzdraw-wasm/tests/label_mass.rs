//! The mass labels of arrows (tp-07b) with the real engine (`sel` of mzLab through the glue): the numbers checked against the TP table of the
//! same drawing, against hand sums, the refusal of a structure the engine cannot measure, and the goldens of
//! `tests/data/log/pathway-label-mass.json`. Synthetic structures only.
//!
//! Hand sums (u): H 1.00782503207, C 12 (exact), O 15.99491462, electron 0.00054857991.

use mzdraw_wasm::glue;
use serde_json::{json, Value};

const TP12: &str = include_str!("../../../tests/data/tp/tp12.csv");

fn run(request: Value) -> String {
    glue::replay(&request.to_string()).unwrap_or_else(|e| panic!("{}", e.to_json()))
}

/// The texts of the arrows of the document a request gives, in object order.
fn arrow_texts(request: Value) -> Vec<String> {
    let mut r = request;
    r["out"] = json!("mzdraw");
    let doc: Value = serde_json::from_str(&run(r)).unwrap();
    doc["objects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["t"] == "arrow")
        .map(|o| o["text"].as_str().unwrap_or("").to_string())
        .collect()
}

fn list(args: Value, mode: &str) -> Value {
    json!({"set": [format!("pathway.label={mode}")],
        "commands": [{"command": "tp:import", "args": {"csv": TP12}}, {"command": "pathway:label", "args": args}]})
}

#[test]
fn every_label_is_the_delta_and_the_mass_diff_of_a_row_of_the_table() {
    // the 12 pairs of the 12-row list (11 products and one second parent): the label `form_diff mass_diff` of each arrow is a row of the table, the same text in the same digits
    let mut labels = arrow_texts(list(json!({}), "delta+mass"));
    let mut req = list(json!({}), "delta");
    req["out"] = json!("tptable");
    let t: Value = serde_json::from_str(&run(req)).unwrap();
    let mut rows: Vec<String> = t["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["parent_id"] != "")
        .map(|r| {
            format!(
                "{} {}",
                r["form_diff"].as_str().unwrap(),
                r["mass_diff"].as_str().unwrap()
            )
        })
        .collect();
    assert_eq!(labels.len(), 12);
    labels.sort();
    rows.sort();
    assert_eq!(labels, rows);
    // the mass mode alone is the mass_diff column
    let mut only = arrow_texts(list(json!({}), "mass"));
    only.sort();
    let mut diffs: Vec<String> = rows
        .iter()
        .map(|r| r.split(' ').nth(1).unwrap().to_string())
        .collect();
    diffs.sort();
    assert_eq!(only, diffs);
}

#[test]
fn the_hand_sums_of_a_shift_and_of_the_m_z_of_the_product() {
    let labels = arrow_texts(list(json!({"adduct": "[M+H]+"}), "delta+mass"));
    // P -> M1: +O is 15.99491462 -> +15.9949; M1, C10H14O, 150.10446 + 1.00727645 = 151.11174 -> 151.1117
    assert!(
        labels.contains(&"+O +15.9949 [M+H]+ 151.1117".to_string()),
        "{labels:?}"
    );
    // M1 -> TP1 and M2 -> TP2: C10H14O2, 166.09938 + 1.00727645 = 167.10666 -> 167.1067
    assert!(labels.contains(&"+O +15.9949 [M+H]+ 167.1067".to_string()));
    // the acid -> TP3 (-CH2): -(12 + 2 * 1.00782503) = -14.01565 -> -14.0157
    assert!(labels
        .iter()
        .any(|l| l.starts_with("-CH2 -14.0157 [M+H]+ ")));
    // ACID -> TP4 and every other arrow carry the adduct: no arrow of the 12 is left without an m/z
    assert!(labels.iter().all(|l| l.contains(" [M+H]+ ")), "{labels:?}");
}

/// A document of `Na+` (one atom) and ethanol with an arrow, and ethanol and ethanoxide with another.
fn mixed() -> Value {
    json!({"format": "mzdraw/1", "next": 900,
        "atoms": [
            {"id": 1, "el": "Na", "x": 0.0, "y": 0, "chg": 1},
            {"id": 11, "el": "C", "x": 6.0, "y": 0}, {"id": 12, "el": "C", "x": 7.0, "y": 0}, {"id": 13, "el": "O", "x": 8.0, "y": 0},
            {"id": 21, "el": "C", "x": 12.0, "y": 0}, {"id": 22, "el": "C", "x": 13.0, "y": 0}, {"id": 23, "el": "O", "x": 14.0, "y": 0, "chg": -1}],
        "bonds": [{"id": 111, "a": 11, "b": 12, "o": 1}, {"id": 112, "a": 12, "b": 13, "o": 1},
            {"id": 121, "a": 21, "b": 22, "o": 1}, {"id": 122, "a": 22, "b": 23, "o": 1}],
        "objects": [
            {"id": 501, "t": "arrow", "x": 2.0, "y": 0, "x2": 5.0, "y2": 0, "from": 1, "to": 11},
            {"id": 502, "t": "arrow", "x": 9.0, "y": 0, "x2": 11.0, "y2": 0, "from": 11, "to": 21}]})
}

#[test]
fn what_the_engine_cannot_measure_is_a_keyed_note_and_a_charged_product_has_no_m_z() {
    let req = json!({"import": mixed().to_string(), "set": ["pathway.label=delta+mass"],
        "commands": [{"command": "pathway:label", "args": {"adduct": "[M+H]+"}}], "report": true, "out": "mzdraw"});
    let v: Value = serde_json::from_str(&run(req)).unwrap();
    assert_eq!(v["ok"], true, "{v}");
    let lost: Vec<(String, u64)> = v["lost"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["key"].as_str().unwrap().to_string(),
                l["n"].as_u64().unwrap(),
            )
        })
        .collect();
    // Na+ has no valence rule in the engine: its arrow is skipped under the engine's own key; the ethoxide has a charge: no m/z
    assert!(
        lost.contains(&("error.grafo.element".to_string(), 1)),
        "{lost:?}"
    );
    assert!(
        lost.contains(&("draw2.label.mzCharged".to_string(), 1)),
        "{lost:?}"
    );
    let doc: Value = serde_json::from_str(v["out"].as_str().unwrap()).unwrap();
    let texts: Vec<&str> = doc["objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["text"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        texts[0], "",
        "the arrow of the structure the engine refused is unlabelled"
    );
    // ethanol -> ethanoxide: -H, the difference of the two masses 46.04186 - 45.03404 = 1.00783 (the ion is the sum of its atoms)
    assert_eq!(texts[1], "-H -1.0078", "{texts:?}");
}

#[test]
fn the_plain_replay_has_no_engine_and_refuses_the_mass_modes() {
    let req = json!({"import": mixed().to_string(), "set": ["pathway.label=mass"],
        "commands": [{"command": "pathway:label"}], "report": true});
    let v: Value =
        serde_json::from_str(&mzdraw::replay::replay(&req.to_string()).unwrap()).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["failed"]["key"], "error.draw2.labelMass");
}

#[test]
fn the_goldens_of_the_mass_labels_are_what_the_glue_door_writes() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/log/pathway-label-mass.json"
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = file["values"].as_array().unwrap();
    assert!(cases.len() >= 4);
    let bless = std::env::var("MZDRAW_BLESS").is_ok();
    let mut fresh = file.clone();
    for (i, c) in cases.iter().enumerate() {
        let got = run(c["request"].clone());
        if bless {
            fresh["values"][i]["out"] = Value::from(got);
        } else {
            assert_eq!(got, c["out"].as_str().unwrap(), "{}", c["name"]);
        }
    }
    if bless {
        std::fs::write(path, serde_json::to_string_pretty(&fresh).unwrap() + "\n").unwrap();
    }
}
