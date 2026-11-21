//! The mass-label choice of a scheme (mass-label-ui) with the real engine: the page's route (the menu items of an arrow) and the CLI route (the log
//! `doc.labels` and the command `pathway:label`) write the same document, and the goldens of `tests/data/log/mass-label-ui.json`.
//! Synthetic structures only.

use mzdraw::{Document, Editor};
use mzdraw_wasm::glue;
use serde_json::{json, Value};

const SCHEME: &str = r#"{"format":"mzdraw/1","next":900,"atoms":[{"id":1,"el":"C","x":0,"y":0},{"id":2,"el":"C","x":1,"y":0},{"id":11,"el":"C","x":6,"y":0},{"id":12,"el":"C","x":7,"y":0},{"id":13,"el":"O","x":8,"y":0},{"id":21,"el":"C","x":12,"y":0},{"id":22,"el":"C","x":13,"y":0},{"id":23,"el":"O","x":14,"y":0},{"id":24,"el":"O","x":15,"y":0}],"bonds":[{"id":101,"a":1,"b":2,"o":1},{"id":111,"a":11,"b":12,"o":1},{"id":112,"a":12,"b":13,"o":1},{"id":121,"a":21,"b":22,"o":1},{"id":122,"a":22,"b":23,"o":1},{"id":123,"a":23,"b":24,"o":1}],"objects":[{"id":501,"t":"arrow","x":2.5,"y":0,"x2":5,"y2":0,"from":1,"to":11},{"id":502,"t":"arrow","x":9,"y":0,"x2":11,"y2":0,"from":11,"to":21}]}"#;

fn run(request: Value) -> String {
    glue::replay(&request.to_string()).unwrap_or_else(|e| panic!("{}", e.to_json()))
}

/// The document of the CLI route: the log operation and the command, replayed through the glue door.
fn by_log(mode: &str, adduct: &str) -> Document {
    let mut op = json!({"op": "doc.labels", "mode": mode});
    if !adduct.is_empty() {
        op["adduct"] = json!(adduct);
    }
    let out = run(
        json!({"import": SCHEME, "log": [op], "commands": [{"command": "pathway:label"}], "out": "mzdraw"}),
    );
    Document::from_json(&out).unwrap()
}

/// The document of the page's route: the menu items of the first arrow, in the page's editor with the engine of the bridge.
fn by_menu(items: &[&str]) -> Document {
    let mut e = Editor::default();
    e.import(SCHEME).unwrap();
    e.measurer = Some(glue::measure);
    for id in items {
        let a = e.doc().objects[0].clone();
        let (x, y) = e.view.to_px((a.x + a.x2) / 2.0, a.y);
        e.context_menu(x, y);
        e.context_run(id);
        assert_eq!(e.error, "", "{id}");
    }
    e.doc().clone()
}

fn texts(d: &Document) -> Vec<String> {
    d.objects.iter().map(|o| o.text.clone()).collect()
}

#[test]
fn the_page_route_and_the_cli_route_write_the_same_scheme() {
    for (items, mode, adduct) in [
        (vec!["labelmode.mass"], "mass", ""),
        (
            vec!["labelmode.delta+mass", "labeladduct.[M+H]+"],
            "delta+mass",
            "[M+H]+",
        ),
        (
            vec!["labelmode.mass", "labeladduct.[M-H]-"],
            "mass",
            "[M-H]-",
        ),
        (vec!["labelmode.name+delta"], "name+delta", ""),
    ] {
        let page = by_menu(&items);
        let cli = by_log(mode, adduct);
        // the labels are the same text and the field is the same: the data of the document is one value, whatever route wrote it
        assert_eq!(texts(&page), texts(&cli), "{items:?}");
        assert_eq!(page.labels(), cli.labels(), "{items:?}");
        assert_eq!(page.labels().0, mode);
    }
}

#[test]
fn the_labels_are_the_numbers_of_the_engine_and_the_menu_says_them() {
    let d = by_menu(&["labelmode.delta+mass", "labeladduct.[M+H]+"]);
    // ethane -> ethanol: +O, +15.9949, the m/z of ethanol [M+H]+ 47.0491 (the hand sums of tp-07b); ethanol -> ethyl hydroperoxide: +O again
    assert_eq!(texts(&d)[0], "+O +15.9949 [M+H]+ 47.0491");
    assert_eq!(d.labels(), ("delta+mass".into(), "[M+H]+".into()));
}

#[test]
fn the_goldens_of_the_choice_are_what_the_glue_door_writes() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/log/mass-label-ui.json"
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = file["values"].as_array().unwrap();
    assert!(cases.len() >= 5);
    let bless = std::env::var("MZDRAW_BLESS").is_ok();
    let mut fresh = file.clone();
    for (i, c) in cases.iter().enumerate() {
        let got = glue::replay(&c["request"].to_string()).unwrap_or_else(|e| e.to_json());
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
