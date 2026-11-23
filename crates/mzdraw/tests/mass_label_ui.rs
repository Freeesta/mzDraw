//! The mass-label choice of a scheme (mass-label-ui): the document field `labels`, the operation `doc.labels`, the defaults `pathway.label`
//! and `pathway.adduct`, and the menu items that choose them. Synthetic structures and a stand-in engine with made-up weights: these tests are
//! about the choice and the label, never about a mass.

use mzdraw::editor::CHANGED;
use mzdraw::values::Measured;
use mzdraw::{Document, Editor, Op};
use serde_json::{json, Value};

/// ethane -> ethanol, one anchored arrow (ids: atoms 1-2 and 11-13, arrow 5000).
fn scheme() -> String {
    let atoms = [
        (1, "C", 0.0),
        (2, "C", 1.0),
        (11, "C", 6.0),
        (12, "C", 7.0),
        (13, "O", 8.0),
    ];
    let atoms: Vec<Value> = atoms
        .iter()
        .map(|(id, el, x)| json!({"id": id, "el": el, "x": x, "y": 0}))
        .collect();
    json!({"format": "mzdraw/1", "next": 9000, "atoms": atoms,
        "bonds": [{"id": 1001, "a": 1, "b": 2, "o": 1}, {"id": 1012, "a": 11, "b": 12, "o": 1}, {"id": 1013, "a": 12, "b": 13, "o": 1}],
        "objects": [{"id": 5000, "t": "arrow", "x": 3.0, "y": 0, "x2": 5.0, "y2": 0, "from": 1, "to": 11}]})
    .to_string()
}

/// The stand-in engine: made-up weights, an adduct adds 1.5.
fn engine(molfile: &str, adducts: &[String]) -> Result<Measured, &'static str> {
    let mass: f64 = molfile
        .lines()
        .filter_map(|l| {
            let t: Vec<&str> = l.split_whitespace().collect();
            (t.len() >= 4 && t[0].contains('.') && t[3].chars().all(|c| c.is_ascii_alphabetic()))
                .then(|| if t[3] == "O" { 16.5 } else { 12.0 })
        })
        .sum();
    Ok(Measured {
        mass,
        average: None,
        ion_mz: None,
        pct: Vec::new(),
        mz: adducts.iter().map(|_| Some(mass + 1.5)).collect(),
    })
}

fn editor(engine_on: bool) -> Editor {
    let mut e = Editor::default();
    e.import(&scheme()).unwrap();
    if engine_on {
        e.measurer = Some(engine);
    }
    e
}

fn text(e: &Editor) -> String {
    e.doc().object(5000).unwrap().text.clone()
}

fn labels(e: &Editor) -> Value {
    serde_json::from_str(&e.labels_json()).unwrap()
}

fn doc_op(mode: &str, adduct: &str) -> Op {
    Op::DocLabels {
        mode: mode.into(),
        adduct: adduct.into(),
    }
}

#[test]
fn the_choice_is_a_document_field_written_only_when_set_and_undone_exactly() {
    let mut e = editor(false);
    let before = e.doc().to_json();
    assert!(
        !before.contains("labels"),
        "a scheme with no choice writes no field"
    );
    e.session.run(&[doc_op("delta+mass", "[M+H]+")]).unwrap();
    let after = e.doc().to_json();
    assert!(
        after.contains(r#""labels":{"adduct":"[M+H]+","mode":"delta+mass"}"#),
        "{after}"
    );
    // the file opens with the same choice
    let again = Document::from_json(&after).unwrap();
    assert_eq!(again.labels(), ("delta+mass".into(), "[M+H]+".into()));
    assert_eq!(again.to_json(), after);
    assert!(e.undo() & CHANGED != 0);
    assert_eq!(e.doc().to_json(), before);
    // an empty mode takes the choice away again
    e.session.run(&[doc_op("mass", "")]).unwrap();
    e.session.run(&[doc_op("", "")]).unwrap();
    assert_eq!(e.doc().to_json(), before);
}

#[test]
fn a_bad_choice_is_a_keyed_error_and_changes_nothing_never_a_fallback() {
    for (mode, adduct) in [
        ("bogus", ""),
        ("Delta", ""),
        ("delta", "[M+H]+"),
        ("name+delta", "[M+H]+"),
        ("mass", "Na"),
        ("mass", "[M+2H]2+"),
        ("", "[M+H]+"),
    ] {
        let mut e = editor(false);
        let before = e.doc().to_json();
        let err = e.session.run(&[doc_op(mode, adduct)]).unwrap_err();
        assert_eq!(err.key, "error.draw2.labels", "{mode:?} {adduct:?}");
        assert_eq!(e.doc().to_json(), before);
    }
    // in a log it is the same refusal
    let r = mzdraw::replay::replay(&format!(
        r#"{{"import":{},"log":[{{"op":"doc.labels","mode":"bogus"}}]}}"#,
        serde_json::to_string(&scheme()).unwrap()
    ));
    assert_eq!(r.unwrap_err().key, "error.draw2.labels");
}

#[test]
fn the_keys_of_the_field_this_version_does_not_know_are_kept() {
    let mut doc: Value = serde_json::from_str(&scheme()).unwrap();
    doc["labels"] = json!({"mode": "mass", "future": [1, 2]});
    let mut e = Editor::default();
    e.open(&doc.to_string()).unwrap();
    assert_eq!(e.doc().labels().0, "mass");
    e.session.run(&[doc_op("delta+mass", "[M-H]-")]).unwrap();
    let out: Value = serde_json::from_str(&e.doc().to_json()).unwrap();
    assert_eq!(out["labels"]["future"], json!([1, 2]));
    assert_eq!(out["labels"]["mode"], "delta+mass");
    // a field of another shape is no choice, and is written back as it came
    let mut doc: Value = serde_json::from_str(&scheme()).unwrap();
    doc["labels"] = json!("later");
    let mut e = Editor::default();
    e.open(&doc.to_string()).unwrap();
    assert_eq!(e.doc().labels(), (String::new(), String::new()));
    assert!(e.doc().to_json().contains(r#""labels":"later""#));
}

#[test]
fn pathway_label_uses_the_argument_then_the_scheme_then_the_preferences() {
    // the preferences: mass with an adduct, for a scheme with no choice
    let mut e = editor(true);
    e.set("pathway.label=mass").unwrap();
    e.set("pathway.adduct=[M+Na]+").unwrap();
    e.run_command("pathway:label", &json!({})).unwrap();
    assert_eq!(text(&e), "+16.5000 [M+Na]+ 42.0000");
    // the scheme's own choice wins over the preferences
    let mut e = editor(true);
    e.set("pathway.label=mass").unwrap();
    e.set("pathway.adduct=[M+Na]+").unwrap();
    e.session.run(&[doc_op("delta", "")]).unwrap();
    e.run_command("pathway:label", &json!({})).unwrap();
    assert_eq!(text(&e), "+O");
    // an explicit argument wins over both
    let mut e = editor(true);
    e.session.run(&[doc_op("mass", "[M+H]+")]).unwrap();
    e.run_command("pathway:label", &json!({"adduct": "[M-H]-"}))
        .unwrap();
    assert_eq!(text(&e), "+16.5000 [M-H]- 42.0000");
    // the scheme's choice is what a replay of the log runs with: the CLI route, no page
    let reply = mzdraw::replay::replay_with(
        &json!({"import": scheme(), "log": [{"op": "doc.labels", "mode": "delta+mass", "adduct": "[M+H]+"}],
            "commands": [{"command": "pathway:label"}]})
        .to_string(),
        0,
        Some(engine),
    )
    .unwrap();
    assert!(
        reply.text.contains("+O +16.5000 [M+H]+ 42.0000"),
        "{}",
        reply.text
    );
}

#[test]
fn the_preferences_hold_the_defaults_for_new_schemes_and_say_when_a_value_is_none() {
    let mut e = Editor::default();
    assert_eq!(e.get("pathway.adduct").unwrap(), "none");
    e.set("pathway.adduct=[M-H]-").unwrap();
    assert_eq!(e.get("pathway.adduct").unwrap(), "[M-H]-");
    for bad in [
        "pathway.adduct=Na",
        "pathway.adduct=",
        "pathway.label=bogus",
    ] {
        assert_eq!(e.set(bad).unwrap_err(), "error.draw2.styleValue", "{bad}");
    }
    assert_eq!(e.get("pathway.adduct").unwrap(), "[M-H]-");
    // the settings file keeps it, a file of another adduct is read, an unknown one is not a style
    let file = e.style.to_json();
    assert!(file.contains(r#""pathway.adduct":"[M-H]-""#), "{file}");
    let back = mzdraw::settings::Style::from_text(&file).unwrap();
    assert_eq!(back.value("pathway.adduct").unwrap(), "[M-H]-");
    let bad = file.replace("[M-H]-", "[M+2H]2+");
    assert!(mzdraw::settings::Style::from_text(&bad).is_err());
    // the default (none) is not written
    assert!(!mzdraw::settings::Style::default()
        .to_json()
        .contains("pathway.adduct"));
}

#[test]
fn the_state_says_what_is_in_force_and_what_it_means() {
    let mut e = editor(true);
    let l = labels(&e);
    assert_eq!(
        (l["mode"].as_str(), l["own"].as_bool()),
        (Some("delta"), Some(false))
    );
    assert_eq!(l["meaning"], "draw2.labels.meaning.delta");
    assert_eq!(l["modes"].as_array().unwrap().len(), 4);
    assert_eq!(l["adducts"].as_array().unwrap().len(), 7);
    e.set("pathway.label=mass").unwrap();
    assert_eq!(labels(&e)["mode"], "mass");
    e.session.run(&[doc_op("name+delta", "")]).unwrap();
    let l = labels(&e);
    assert_eq!(
        (l["mode"].as_str(), l["own"].as_bool()),
        (Some("name+delta"), Some(true))
    );
}

// ---- the menu: the items an arrow and the canvas carry

fn items(menu: &Value) -> Vec<Value> {
    fn walk(list: &Value, out: &mut Vec<Value>) {
        for i in list.as_array().unwrap() {
            out.push(i.clone());
            if i.get("children").is_some() {
                walk(&i["children"], out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&menu["items"], &mut out);
    out
}

fn open(e: &mut Editor, canvas: bool) -> Vec<Value> {
    let (x, y) = if canvas {
        (900.0, 900.0)
    } else {
        let o = e.doc().objects[0].clone();
        e.view.to_px((o.x + o.x2) / 2.0, o.y)
    };
    items(&serde_json::from_str(&e.context_menu(x, y)).unwrap())
}

fn item<'a>(list: &'a [Value], id: &str) -> &'a Value {
    list.iter()
        .find(|i| i["id"] == id)
        .unwrap_or_else(|| panic!("no item {id}"))
}

#[test]
fn an_arrow_and_the_canvas_offer_the_modes_and_tick_the_one_in_force() {
    let mut e = editor(true);
    for canvas in [false, true] {
        let m = open(&mut e, canvas);
        assert_eq!(item(&m, "labelmode.delta")["checked"], true);
        assert_eq!(item(&m, "labelmode.mass")["checked"], false);
        // the adduct has no meaning for a mode with no numbers: its group is off
        assert_eq!(item(&m, "pathway.labeladduct")["enabled"], false);
    }
    e.set("pathway.label=delta+mass").unwrap();
    let m = open(&mut e, false);
    assert_eq!(item(&m, "labelmode.delta+mass")["checked"], true);
    assert_eq!(item(&m, "pathway.labeladduct")["enabled"], true);
    assert_eq!(item(&m, "labeladduct.none")["checked"], true);
    // a free arrow has no scheme to label: no items
    let mut free = Editor::default();
    free.import(
        r#"{"format":"mzdraw/1","next":3,"atoms":[],"bonds":[],"objects":[{"id":1,"t":"arrow","x":0,"y":0,"x2":2,"y2":0}]}"#,
    )
    .unwrap();
    let m = open(&mut free, false);
    assert!(m.iter().all(|i| i["id"] != "pathway.labelmode"));
}

#[test]
fn choosing_a_mode_sets_the_field_and_writes_the_labels_in_one_undo_step() {
    let mut e = editor(true);
    let before = e.doc().to_json();
    open(&mut e, false);
    assert!(e.context_run("labelmode.delta+mass") & CHANGED != 0);
    assert_eq!(text(&e), "+O +16.5000");
    assert_eq!(e.doc().labels().0, "delta+mass");
    open(&mut e, false);
    e.context_run("labeladduct.[M+H]+");
    assert_eq!(text(&e), "+O +16.5000 [M+H]+ 42.0000");
    assert_eq!(e.doc().labels(), ("delta+mass".into(), "[M+H]+".into()));
    // a mode with no numbers drops the adduct (never an adduct beside a mode that has none)
    open(&mut e, false);
    e.context_run("labelmode.delta");
    assert_eq!(text(&e), "+O");
    assert_eq!(e.doc().labels(), ("delta".into(), String::new()));
    // each choice is one step: three undos bring the first document back
    for _ in 0..3 {
        assert!(e.undo() & CHANGED != 0);
    }
    assert_eq!(e.doc().to_json(), before);
}

#[test]
fn a_mode_that_needs_the_engine_is_refused_with_its_key_and_changes_nothing() {
    let mut e = editor(false);
    let before = e.doc().to_json();
    let steps = e.session.can_undo();
    open(&mut e, false);
    e.context_run("labelmode.mass");
    assert_eq!(e.doc().to_json(), before);
    assert_eq!(e.error, "error.draw2.labelMass");
    assert_eq!(e.session.can_undo(), steps);
    // a person's own label is kept by a mode change, as `pathway:label` keeps it
    let mut e = editor(true);
    e.run_command("pathway:label", &json!({})).unwrap();
    e.session
        .run(&[Op::ObjSet {
            id: 5000,
            text: Some("hydroxylation (CYP)".into()),
            runs: None,
            x2: None,
            y2: None,
            from: None,
            to: None,
            cv: Default::default(),
            pt: [None, None],
        }])
        .unwrap();
    open(&mut e, false);
    e.context_run("labelmode.mass");
    assert_eq!(text(&e), "hydroxylation (CYP)");
    assert_eq!(e.doc().labels().0, "mass");
}

#[test]
fn the_choice_is_a_loss_for_a_format_that_cannot_hold_it() {
    let mut e = editor(false);
    e.session.run(&[doc_op("mass", "")]).unwrap();
    let doc = e.doc().clone();
    let lost = |f: &str| {
        mzdraw::loss::loss(&doc, f)
            .unwrap()
            .iter()
            .any(|l| l.what == "labels")
    };
    assert!(lost("molfile") && lost("cdxml") && lost("ket"));
    assert!(!lost("svg") && !lost(mzdraw::brand::SLUG));
    // and it is not an "unknown" field: it has its own row
    assert!(!mzdraw::loss::loss(&doc, "molfile")
        .unwrap()
        .iter()
        .any(|l| l.what == "unknown"));
}

#[test]
fn the_preferences_words_are_the_words_of_the_field_and_the_adduct_list() {
    let schema: Value = serde_json::from_str(&format!("{{{}}}", mzdraw::prefs::json())).unwrap();
    let words = |setting: &str| -> Vec<String> {
        schema["prefs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["setting"] == setting)
            .unwrap_or_else(|| panic!("no pref {setting}"))["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["value"].as_str().unwrap().to_string())
            .collect()
    };
    let modes: Vec<String> = mzdraw::doc::LABEL_MODES
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(words("pathway.label"), modes);
    let mut adducts = vec!["none".to_string()];
    adducts.extend(mzdraw::values::ADDUCTS.iter().map(|s| s.to_string()));
    assert_eq!(words("pathway.adduct"), adducts);
}

#[test]
fn the_sentence_of_the_button_is_a_finished_key_in_the_catalogue() {
    let lang = include_str!("../../../web/lang/mzdraw.js");
    let mut e = editor(true);
    for (mode, adduct) in [
        ("delta", ""),
        ("mass", ""),
        ("delta+mass", "[M+H]+"),
        ("name+delta", ""),
    ] {
        e.session.run(&[doc_op(mode, adduct)]).unwrap();
        let l = labels(&e);
        let key = l["now"].as_str().unwrap();
        assert!(key.ends_with(&format!(".{mode}")), "{key}");
        assert_eq!(key.contains("nowAdduct"), !adduct.is_empty(), "{key}");
        for k in [key, l["meaning"].as_str().unwrap(), "draw2.hunk.labels"] {
            assert!(lang.contains(&format!("\"{k}\"")), "{k}");
        }
    }
}

#[test]
fn a_patch_with_the_choice_has_a_hunk_of_its_own_and_says_it() {
    let a = Document::from_json(&scheme()).unwrap();
    let mut e = editor(false);
    e.session.run(&[doc_op("mass", "")]).unwrap();
    let mut b = e.doc().clone();
    // the choice and a change of the drawing: two hunks that apply alone
    b.objects[0].text = "x".into();
    let ops = mzdraw::diff::diff(&a, &b);
    assert!(
        ops.iter().any(|o| matches!(o, Op::DocLabels { .. })),
        "{ops:?}"
    );
    let hunks = mzdraw::diff::hunks(&a, &ops);
    assert_eq!(hunks.len(), 2, "{hunks:?}");
    let own = hunks
        .iter()
        .find(|h| h.ops.iter().any(|o| matches!(o, Op::DocLabels { .. })))
        .unwrap();
    assert_eq!(own.ops.len(), 1);
    assert_eq!(
        mzdraw::proposal::say(own),
        [("draw2.hunk.labels", String::new())]
    );
}
