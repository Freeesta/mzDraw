//! Golden replay: every case of the files of `tests/data/log/` (one file per area; a start text, a command log) gives the recorded document, molfile (V2000 and
//! V3000), SMILES, SDF, SVG, log and drawing checks (`std`) byte for byte, and every refused request its key. The `.wasm` is held to the same file by `tests/parity.mjs`.
//! `MZDRAW_BLESS=1 cargo test -p mzdraw --test replay` rewrites the recorded outputs after a wanted change.

mod common;

use common::{golden_files, section};
use serde_json::{json, Value};

const OUTS: [&str; 11] = [
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
    "std",
];

fn section_mut<'a>(fx: &'a mut Value, key: &str) -> &'a mut [Value] {
    fx.get_mut(key)
        .and_then(Value::as_array_mut)
        .map_or(&mut [], Vec::as_mut_slice)
}

fn request(case: &Value, out: &str) -> String {
    let mut r = case["request"].clone();
    r["out"] = json!(out);
    r.to_string()
}

#[test]
fn every_case_replays_to_the_recorded_bytes() {
    let mut files = golden_files();
    let bless = std::env::var("MZDRAW_BLESS").is_ok();
    let mut cases = 0;
    for (path, fx) in files.iter_mut() {
        cases += section(fx, "cases").len();
        for case in section_mut(fx, "cases") {
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
        for case in section_mut(fx, "contexts") {
            let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
            if bless {
                case["out"] = json!(got);
            }
            assert_eq!(case["out"], json!(got), "{}", case["name"]);
            let menu: Value = serde_json::from_str(&got).unwrap();
            assert!(menu["target"].is_string() && menu["items"].is_array());
        }
        // the verdict of a request that asks for one (`draw-result/1`): recorded text and `ok`, the request being number `n` of its batch
        for case in section_mut(fx, "results") {
            let n = case.get("n").and_then(Value::as_u64).unwrap_or(0);
            let got = mzdraw::replay::replay_at(&case["request"].to_string(), n).unwrap();
            if bless {
                case["out"] = json!(got.text);
            }
            assert_eq!(case["out"], json!(got.text), "{}", case["name"]);
            let doc: Value = serde_json::from_str(&got.text).unwrap();
            assert_eq!(doc["ok"], json!(got.ok), "{}", case["name"]);
        }
        // the table of the operations (`draw-edit-schema/1`): the CLI twin of the language reader
        for case in section_mut(fx, "schemas") {
            let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
            if bless {
                case["out"] = json!(got);
            }
            assert_eq!(case["out"], json!(got), "{}", case["name"]);
            assert_eq!(got, mzdraw::schema::json());
        }
        // the settings file (`out: settings`) after settings set through `Editor::set` (the `--set` of the CLI): the layout and the
        // export defaults are covered by the same parity as the drawing
        for case in section_mut(fx, "settings") {
            let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
            if bless {
                case["out"] = json!(got);
            }
            assert_eq!(case["out"], json!(got), "{}", case["name"]);
            let file: Value = serde_json::from_str(&got).unwrap();
            assert!(file["layout"].is_object() && file["export"].is_object());
        }
        // the PDF file as base64 (`out: pdf`): the same bytes on both sides, a PDF that opens again as the same drawing
        for case in section_mut(fx, "pdf") {
            let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
            if bless {
                case["out"] = json!(got);
            }
            assert_eq!(case["out"], json!(got), "{}", case["name"]);
            assert!(got.starts_with("JVBERi0xLjc"), "{}: %PDF-1.7", case["name"]);
        }
        if bless {
            std::fs::write(&*path, serde_json::to_string_pretty(&*fx).unwrap() + "\n").unwrap();
        }
    }
    assert!(cases >= 3);
}

#[test]
fn the_recorded_log_replays_to_the_same_document() {
    for (_, fx) in golden_files() {
        for case in section(&fx, "cases") {
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
}

#[test]
fn a_refused_request_gives_its_key() {
    for (_, fx) in golden_files() {
        for case in section(&fx, "refused") {
            let e = mzdraw::replay::replay(&case["request"].to_string()).unwrap_err();
            assert_eq!(e.key, case["error_key"].as_str().unwrap());
        }
    }
}
