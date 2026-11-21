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

/// The table of the operations (`draw-edit-schema/1`, one line of bytes) cut into pieces, one per row: the header, every operation, the
/// join to the request rows, every request row, the closing `]}`. A row that starts with a comma keeps it, so a row added in the middle or at the end
/// of the table adds its own lines and changes no other. The pieces joined give the bytes back exactly.
fn schema_rows(text: &str) -> Vec<String> {
    const ROWS: [&str; 2] = [r#"{"op":""#, r#"{"section":""#];
    const JOIN: &str = r#"],"request":["#;
    let mut cuts = vec![0];
    for (i, _) in text.char_indices() {
        let rest = &text[i..];
        let after_comma = text[..i].ends_with(',');
        if (rest.starts_with(',') && ROWS.iter().any(|r| rest[1..].starts_with(r)))
            || (!after_comma && ROWS.iter().any(|r| rest.starts_with(r)))
            || rest.starts_with(JOIN)
        {
            cuts.push(i);
        }
    }
    // the closing `]}` is a piece of its own: a row appended at the end changes no line that exists
    if text.ends_with("]}") {
        cuts.push(text.len() - 2);
    }
    cuts.push(text.len());
    cuts.windows(2)
        .map(|w| text[w[0]..w[1]].to_string())
        .collect()
}

/// The recorded table as the bytes it stands for: the text of an old one-line file, or the rows of a new one joined.
fn schema_out(out: &Value) -> String {
    match out {
        Value::String(s) => s.clone(),
        Value::Array(rows) => rows.iter().map(|r| r.as_str().unwrap()).collect(),
        other => panic!("the recorded table is a string or its rows, not {other}"),
    }
}

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
        // the context menu of a right-click on each kind of target and what pointer and key events leave of it (the flags, the selection, the
        // anchor, the validity), as the `.wasm` gives them (tests/parity.mjs)
        for case in section_mut(fx, "contexts") {
            let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
            if bless {
                case["out"] = json!(got);
            }
            assert_eq!(case["out"], json!(got), "{}", case["name"]);
            let v: Value = serde_json::from_str(&got).unwrap();
            if case["request"]["out"]
                .as_str()
                .unwrap()
                .starts_with("context-state:")
            {
                assert!(v["flags"].is_array() && v["valid"].is_array() && v["anchor"].is_array());
            } else {
                assert!(v["target"].is_string() && v["items"].is_array());
            }
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
        // the report of a bug or an improvement (`feedback/1`): the same bytes on both sides, the issue form named by the request
        for case in section_mut(fx, "feedbacks") {
            let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
            if bless {
                case["out"] = json!(got);
            }
            assert_eq!(case["out"], json!(got), "{}", case["name"]);
            let v: Value = serde_json::from_str(&got).unwrap();
            assert_eq!(v["format"], "feedback/1");
            assert!(v["url"].as_str().unwrap().len() <= mzdraw::feedback::LIMIT);
        }
        // the table of the operations (`draw-edit-schema/1`): the CLI twin of the language reader
        for case in section_mut(fx, "schemas") {
            let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
            // the table is recorded one row per line (`out` is the list of its rows): two changes to different rows do not meet
            if bless {
                case["out"] = json!(schema_rows(&got));
            }
            assert!(
                case["out"].is_array(),
                "{}: the rows, not one line",
                case["name"]
            );
            assert_eq!(schema_out(&case["out"]), got, "{}", case["name"]);
            assert_eq!(case["out"], json!(schema_rows(&got)), "{}", case["name"]);
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
        // what a format would drop of the drawing (`out: loss:FORMAT`, `loss/1`): the same bytes on both sides, the target named by the request
        for case in section_mut(fx, "losses") {
            let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
            if bless {
                case["out"] = json!(got);
            }
            assert_eq!(case["out"], json!(got), "{}", case["name"]);
            let v: Value = serde_json::from_str(&got).unwrap();
            let target = case["request"]["out"]
                .as_str()
                .unwrap()
                .strip_prefix("loss:");
            assert_eq!(v["format"], "loss/1");
            assert_eq!(v["target"].as_str(), target, "{}", case["name"]);
            assert!(v["items"].is_array());
        }
        // a drawn parent and product as `trasf/1` with its two records (`out: trasf`): the same bytes on both sides; the record is read back
        // by `mzlab-mol` and its `delta` is the one written in the case
        for case in section_mut(fx, "trasf") {
            let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
            if bless {
                case["out"] = json!(got);
            }
            assert_eq!(case["out"], json!(got), "{}", case["name"]);
            let t = mzlab_mol::Transformation::from_json(got.lines().next().unwrap()).unwrap();
            assert_eq!(json!(t.delta), case["delta"], "{}", case["name"]);
        }
        // the semantic diff of two documents (`{"diff":{"a","b"}}`): the patch and its hunks, which applied in order give `b`
        for case in section_mut(fx, "diffs") {
            let got = mzdraw::replay::replay(&case["request"].to_string()).unwrap();
            if bless {
                case["out"] = json!(got);
            }
            assert_eq!(case["out"], json!(got), "{}", case["name"]);
            let v: Value = serde_json::from_str(&got).unwrap();
            let doc = |k: &str| match case["request"]["diff"][k].as_str() {
                Some(text) => text.to_string(),
                None => case["request"]["diff"][k].to_string(),
            };
            let mut s = mzdraw::Session::new(mzdraw::Document::from_json(&doc("a")).unwrap());
            s.replay(&v["ops"].to_string()).unwrap();
            let b = mzdraw::Document::from_json(&doc("b")).unwrap();
            assert_eq!(s.doc.atoms, b.atoms, "{}", case["name"]);
            assert_eq!(s.doc.bonds, b.bonds, "{}", case["name"]);
            assert_eq!(s.doc.objects, b.objects, "{}", case["name"]);
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
