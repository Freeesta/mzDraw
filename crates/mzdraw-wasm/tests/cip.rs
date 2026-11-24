//! CIP labels with the real engine (`cip` of mzLab through the glue, rdkit-compare 6): the labels by document ids, the picture of
//! the glue door, the page, and the golden file `tests/data/log/cip.json` (section `values`: through the glue door; `MZDRAW_BLESS=1
//! cargo test -p mzdraw-wasm --test cip` rewrites it after a wanted change). Synthetic structures only.

use mzdraw::Editor;
use mzdraw_wasm::glue;
use serde_json::{json, Value};

#[path = "../../mzdraw/tests/support/timing.rs"]
mod timing;

fn labels(smiles: &str) -> Vec<(u32, u32, String)> {
    let mut e = Editor::default();
    e.import(smiles).unwrap();
    glue::cip_labels(e.doc())
        .unwrap()
        .into_iter()
        .map(|l| (l.atom, l.bond, l.text))
        .collect()
}

#[test]
fn the_engine_labels_by_document_ids() {
    // (S)-alanine drawn from SMILES: atom 2 is the centre (ids follow the SMILES order from 1)
    assert_eq!(labels("C[C@H](N)C(=O)O"), [(2, 0, "S".to_string())]);
    assert_eq!(labels("C[C@@H](N)C(=O)O"), [(2, 0, "R".to_string())]);
    // (E)-but-2-ene: the double bond is the label's (bond ids after the atoms: 5, 6, 7; the double bond is the second)
    let e = labels("C/C=C/C");
    assert_eq!(e.len(), 1);
    assert_eq!((e[0].0, e[0].2.as_str()), (0, "E"));
    let mut ed = Editor::default();
    ed.import("C/C=C/C").unwrap();
    let b = ed.doc().bonds.iter().find(|b| b.id == e[0].1).unwrap();
    assert_eq!(b.o, 2, "the label is on the double bond");
    assert_eq!(labels(r"C/C=C\C")[0].2, "Z");
    // no stereo, no label
    assert!(labels("CCO").is_empty());
}

#[test]
fn the_glue_door_draws_them_and_the_page_too() {
    let req = json!({"import": "C/C=C/C[C@H](N)C(=O)O", "set": ["display.cip=on"], "out": "svg"})
        .to_string();
    let svg = glue::replay(&req).unwrap();
    assert!(svg.contains(">(S)<") && svg.contains(">(E)<"), "{svg}");
    // the plain core replay draws none
    let plain = mzdraw::replay::replay(&req).unwrap();
    assert!(!plain.contains(">(S)<"));
    // an editor with the glue's labeller (what the page has) draws the same picture
    let mut e = Editor::default();
    e.import("C/C=C/C[C@H](N)C(=O)O").unwrap();
    e.labeller = Some(glue::cip_labels);
    e.set("display.cip=on").unwrap();
    assert_eq!(e.save("svg").unwrap(), svg);
    assert!(
        e.lost_on_save("svg").is_empty(),
        "the engine is there: nothing lost"
    );
}

#[test]
fn the_golden_requests_give_the_recorded_bytes() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/log/cip.json");
    let mut file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let bless = std::env::var("MZDRAW_BLESS").is_ok();
    for case in file["values"].as_array_mut().unwrap().iter_mut() {
        let got = glue::replay(&case["request"].to_string()).unwrap();
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
fn the_labels_of_many_structures_cost_their_sum_with_the_real_engine() {
    // the record of the review: a structure with two open double bonds and open ring centres, 8 and 40 times. Given whole, the
    // engine took 108 s for 8 (exponential in the open sites); given one structure at a time, 40 cost about five times 8. The least
    // CPU time of runs taken in turns, no wall-clock bound
    let one = "CC(C)C=CC(=O)NC1CCC(CC1)C=CC";
    let editor = |n: usize| {
        let mut e = Editor::default();
        e.import(&vec![one; n].join(".")).unwrap();
        e.labeller = Some(glue::cip_labels);
        e.set("display.cip=on").unwrap();
        e
    };
    let (a, b) = (editor(8), editor(40));
    let (small, large) = timing::best_of_pair(
        1,
        3,
        || drop(mzdraw::cip::label(a.doc(), glue::cip_labels)),
        || drop(mzdraw::cip::label(b.doc(), glue::cip_labels)),
    );
    assert!(
        large.as_nanos() <= 10 * small.as_nanos(),
        "8 structures {small:?}, 40 structures {large:?}"
    );
    // nothing refused, nothing capped: these structures are within the caps
    let done = b.cip_labelled();
    assert_eq!((done.capped, done.failed), (0, 0));
    // one structure joined four times over is past the cap: no label, said, and no call that could hang
    let mut e = Editor::default();
    // (each copy's last carbon bonds to the next copy's first: one chain)
    e.import(&format!("{}C", one.repeat(4))).unwrap();
    e.labeller = Some(glue::cip_labels);
    e.set("display.cip=on").unwrap();
    let done = e.cip_labelled();
    assert_eq!(mzdraw::layout::pieces(e.doc()).len(), 1);
    assert_eq!((done.capped, done.labels.len()), (1, 0));
    assert_eq!(e.cip_notes(), vec![(mzdraw::cip::CAPPED, 1)]);
    // the CLI (the glue door): the picture's result lists it in `lost`
    let req = json!({"import": e.doc().to_json(), "set": ["display.cip=on"], "out": "svg", "report": true});
    let r: Value = serde_json::from_str(&glue::replay(&req.to_string()).unwrap()).unwrap();
    assert_eq!(
        r["lost"],
        json!([{"key": "draw2.export.cipCapped", "n": 1}]),
        "{r}"
    );
}
