//! Golden replay: every case of `tests/data/mzdraw_log.json` (a start text, a command log) gives the recorded document, molfile (V2000 and
//! V3000), SMILES, SDF, SVG and log byte for byte, and every refused request its key. The `.wasm` is held to the same file by `tests/parity.mjs`.
//! `MZDRAW_BLESS=1 cargo test -p mzdraw --test replay` rewrites the recorded outputs after a wanted change.

use serde_json::{json, Value};
use std::path::PathBuf;

const OUTS: [&str; 10] = [
    "mzdraw",
    "molfile",
    "molfile-v3000",
    "smiles",
    "ket",
    "cdxml",
    "cml",
    "sdf",
    "svg",
    "log",
];

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/mzdraw_log.json")
}

fn request(case: &Value, out: &str) -> String {
    let mut r = case["request"].clone();
    r["out"] = json!(out);
    r.to_string()
}

#[test]
fn every_case_replays_to_the_recorded_bytes() {
    let mut fx: Value = serde_json::from_str(&std::fs::read_to_string(fixture()).unwrap()).unwrap();
    let bless = std::env::var("MZDRAW_BLESS").is_ok();
    for case in fx["cases"].as_array_mut().unwrap() {
        for out in OUTS {
            let got = mzdraw::replay::replay(&request(case, out)).unwrap();
            if bless {
                case["out"][out] = json!(got);
            }
            assert_eq!(case["out"][out], json!(got), "{} as {out}", case["name"]);
        }
        // a reaction file is written by the drawings that are a reaction (one arrow): recorded where it is, refused elsewhere
        match mzdraw::replay::replay(&request(case, "rxn")) {
            Ok(got) => {
                if bless {
                    case["out"]["rxn"] = json!(got);
                }
                assert_eq!(case["out"]["rxn"], json!(got), "{} as rxn", case["name"]);
            }
            Err(e) => {
                assert_eq!(e.key, "error.draw2.rxnArrow", "{}", case["name"]);
                assert!(case["out"].get("rxn").is_none(), "{}", case["name"]);
            }
        }
    }
    // the context menu of a right-click on each kind of target, as the `.wasm` gives it (tests/parity.mjs)
    for case in fx["contexts"].as_array_mut().unwrap() {
        let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
        if bless {
            case["out"] = json!(got);
        }
        assert_eq!(case["out"], json!(got), "{}", case["name"]);
        let menu: Value = serde_json::from_str(&got).unwrap();
        assert!(menu["target"].is_string() && menu["items"].is_array());
    }
    // the settings file (`out: settings`) after settings set through `Editor::set` (the `--set` of the CLI): the layout and the
    // export defaults are covered by the same parity as the drawing
    for case in fx["settings"].as_array_mut().unwrap() {
        let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
        if bless {
            case["out"] = json!(got);
        }
        assert_eq!(case["out"], json!(got), "{}", case["name"]);
        let file: Value = serde_json::from_str(&got).unwrap();
        assert!(file["layout"].is_object() && file["export"].is_object());
    }
    if bless {
        std::fs::write(fixture(), serde_json::to_string_pretty(&fx).unwrap() + "\n").unwrap();
    }
    assert!(fx["cases"].as_array().unwrap().len() >= 3);
}

#[test]
fn the_recorded_log_replays_to_the_same_document() {
    let fx: Value = serde_json::from_str(&std::fs::read_to_string(fixture()).unwrap()).unwrap();
    for case in fx["cases"].as_array().unwrap() {
        // the log of a session is itself a command log: replayed on the same start it gives the same document, and logs itself again
        let again = json!({ "log": serde_json::from_str::<Value>(case["out"]["log"].as_str().unwrap()).unwrap(), "out": "log" });
        assert_eq!(
            mzdraw::replay::replay(&again.to_string()).unwrap(),
            case["out"]["log"].as_str().unwrap(),
            "{}",
            case["name"]
        );
    }
}

#[test]
fn a_refused_request_gives_its_key() {
    let fx: Value = serde_json::from_str(&std::fs::read_to_string(fixture()).unwrap()).unwrap();
    for case in fx["refused"].as_array().unwrap() {
        let e = mzdraw::replay::replay(&case["request"].to_string()).unwrap_err();
        assert_eq!(e.key, case["error_key"].as_str().unwrap());
    }
}
