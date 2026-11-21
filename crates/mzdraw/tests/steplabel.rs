//! Arrow labels computed from the two structures (tp-07): `pathway:label`. Synthetic structures only (chains written here).

use mzdraw::editor::CHANGED;
use mzdraw::steplabel::{display, owned, Mode, Opts};
use mzdraw::{Document, Editor};
use serde_json::{json, Value};

/// A structure: elements of a chain joined by the orders of `bonds` (a bond between consecutive atoms: `orders[i]` joins i and i+1).
struct S(&'static [&'static str], &'static [u8]);

const ETHANE: S = S(&["C", "C"], &[1]);
const ETHENE: S = S(&["C", "C"], &[2]);
const ETHANOL: S = S(&["C", "C", "O"], &[1, 1]);
const ETHER: S = S(&["C", "O", "C"], &[1, 1]);
const PROPANE: S = S(&["C", "C", "C"], &[1, 1]);
const PROPANOL: S = S(&["C", "C", "C", "O"], &[1, 1, 1]);

/// The document of the structures in a row (6 bond lengths apart) and an arrow with the text `texts[k]` from structure `a` to `b`
/// for each `(a, b)` of `links`. The first atom of structure k has the id `10 * k + 1`.
fn scheme(structs: &[&S], links: &[(usize, usize, &str)]) -> String {
    let (mut atoms, mut bonds, mut objects) = (vec![], vec![], vec![]);
    for (k, s) in structs.iter().enumerate() {
        for (i, el) in s.0.iter().enumerate() {
            let id = 10 * k + 1 + i;
            atoms.push(json!({"id": id, "el": el, "x": 6.0 * k as f64 + i as f64, "y": 0}));
            if i > 0 {
                bonds.push(json!({"id": 1000 + id, "a": id - 1, "b": id, "o": s.1[i - 1]}));
            }
        }
    }
    for (n, (a, b, text)) in links.iter().enumerate() {
        let mut o = json!({"id": 5000 + n, "t": "arrow", "x": 6.0 * *a as f64 + 3.0, "y": 0, "x2": 6.0 * *b as f64 - 1.0, "y2": 0,
            "from": 10 * a + 1, "to": 10 * b + 1});
        if !text.is_empty() {
            o["text"] = json!(text);
        }
        objects.push(o);
    }
    json!({"format": "mzdraw/1", "next": 9000, "atoms": atoms, "bonds": bonds, "objects": objects})
        .to_string()
}

fn editor(text: &str) -> Editor {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e
}

fn texts(e: &Editor) -> Vec<String> {
    e.doc().objects.iter().map(|o| o.text.clone()).collect()
}

fn run(e: &mut Editor, args: Value) -> Result<(), mzdraw::ops::OpError> {
    e.run_command("pathway:label", &args)
}

#[test]
fn the_delta_of_each_arrow_is_written_in_the_canonical_spelling() {
    let s = scheme(
        &[&ETHANE, &ETHANOL, &PROPANE, &ETHENE, &ETHANOL],
        &[(0, 1, ""), (2, 0, ""), (3, 4, ""), (1, 0, ""), (4, 3, "")],
    );
    let mut e = editor(&s);
    run(&mut e, json!({})).unwrap();
    // ethane -> ethanol +O; propane -> ethane -CH2; ethene -> ethanol +H2O; ethanol -> ethane -O; ethanol -> ethene -H2O
    assert_eq!(texts(&e), ["+O", "-CH2", "+H2O", "-O", "-H2O"]);
    // ethane -> ethene is -H2
    let mut e = editor(&scheme(&[&ETHANE, &ETHENE], &[(0, 1, "")]));
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), ["-H2"]);
}

#[test]
fn equal_formulas_are_an_isomer_in_the_word_of_the_caller() {
    let s = scheme(&[&ETHANOL, &ETHER], &[(0, 1, "")]);
    let mut e = editor(&s);
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), ["isomer"]);
    let mut e = editor(&s);
    run(&mut e, json!({"isomer": "isomero"})).unwrap();
    assert_eq!(texts(&e), ["isomero"]);
    // the word is a token this command owns: another run rewrites it when the structures change
    assert!(owned("isomero", "isomero") && owned("isomer", "isomero"));
}

#[test]
fn a_label_that_is_ours_follows_the_structures_and_the_persons_label_stays() {
    // ours: ASCII, true minus, two lines of tokens; the person's: a name, a bare formula, an ozone step, a delta with a remark
    for ours in [
        "+O",
        "\u{2212}O",
        "-O",
        "+O +15.9949",
        "+Cl",
        "isomer",
        "+5.0000",
    ] {
        let mut e = editor(&scheme(&[&ETHANE, &PROPANE], &[(0, 1, ours)]));
        run(&mut e, json!({})).unwrap();
        assert_eq!(texts(&e), ["+CH2"], "{ours:?} is ours");
    }
    for theirs in [
        "hydroxylation (CYP)",
        "H2O",
        "O3",
        "NaBH4",
        "+O, 37 \u{b0}C",
        "then +O",
        "x",
    ] {
        let mut e = editor(&scheme(&[&ETHANE, &PROPANE], &[(0, 1, theirs)]));
        let before = e.doc().to_json();
        run(&mut e, json!({})).unwrap();
        assert_eq!(texts(&e), [theirs], "{theirs:?} is the person's");
        assert_eq!(e.doc().to_json(), before);
        assert_eq!(e.said.0, "draw2.say.labelled");
        assert_eq!(e.said_params[1], ("kept", 1));
        // force rewrites it
        run(&mut e, json!({"force": true})).unwrap();
        assert_eq!(texts(&e), ["+CH2"]);
    }
}

#[test]
fn a_label_on_an_arrow_whose_product_changed_is_rewritten_and_one_undo_step_restores_it() {
    let mut e = editor(&scheme(&[&ETHANE, &PROPANOL], &[(0, 1, "+O +15.9949")]));
    let before = e.doc().to_json();
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), ["+CH2O"]);
    assert_eq!(e.undo() & CHANGED, CHANGED);
    assert_eq!(e.doc().to_json(), before);
    assert_ne!(e.redo() & CHANGED, 0);
    assert_eq!(texts(&e), ["+CH2O"]);
}

#[test]
fn a_second_run_changes_nothing() {
    let mut e = editor(&scheme(&[&ETHANE, &ETHANOL], &[(0, 1, "")]));
    run(&mut e, json!({})).unwrap();
    let once = e.doc().to_json();
    let flags = e.pathway_label_page("");
    assert_eq!(
        flags & CHANGED,
        0,
        "no step when the text is already the label"
    );
    assert_eq!(e.doc().to_json(), once);
}

#[test]
fn modes_that_need_masses_or_names_are_refused_with_their_keys_and_change_nothing() {
    for (mode, key) in [
        ("mass", "error.draw2.labelMass"),
        ("delta+mass", "error.draw2.labelMass"),
        ("name+delta", "error.draw2.labelSteps"),
    ] {
        let mut e = editor(&scheme(&[&ETHANE, &ETHANOL], &[(0, 1, "")]));
        e.set(&format!("pathway.label={mode}")).unwrap();
        let before = e.doc().to_json();
        let err = run(&mut e, json!({})).unwrap_err();
        assert_eq!(err.key, key, "{mode}");
        assert_eq!(e.doc().to_json(), before);
    }
    let mut e = editor(&scheme(&[&ETHANE, &ETHANOL], &[(0, 1, "")]));
    assert!(e.set("pathway.label=nonsense").is_err());
    assert_eq!(Mode::parse("delta"), Some(Mode::Delta));
    e.set("pathway.label=delta+mass").unwrap();
    assert!(e
        .settings_json()
        .contains(r#""pathway.label":"delta+mass""#));
    e.set("pathway.label=delta").unwrap();
    assert!(!e.settings_json().contains("pathway.label"));
}

#[test]
fn charged_and_r_group_ends_are_left_alone_and_counted() {
    let s = json!({"format": "mzdraw/1", "next": 100,
        "atoms": [{"id": 1, "el": "C", "x": 0, "y": 0}, {"id": 2, "el": "C", "x": 1, "y": 0},
                  {"id": 11, "el": "N", "x": 6, "y": 0, "chg": 1}, {"id": 12, "el": "C", "x": 7, "y": 0}],
        "bonds": [{"id": 3, "a": 1, "b": 2, "o": 1}, {"id": 13, "a": 11, "b": 12, "o": 1}],
        "objects": [{"id": 50, "t": "arrow", "x": 3, "y": 0, "x2": 5, "y2": 0, "from": 1, "to": 11}]})
    .to_string();
    let mut e = editor(&s);
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), [""]);
    assert_eq!(e.said_params[2], ("skipped", 1));
}

#[test]
fn the_arguments_are_checked_before_anything_runs() {
    for bad in [
        json!({"force": 1}),
        json!({"isomer": ""}),
        json!({"isomer": "a b"}),
        json!({"x": 1}),
        json!([]),
    ] {
        assert_eq!(Opts::parse(&bad), Err("error.draw2.request"), "{bad}");
        let r = mzdraw::replay::replay(&json!({"import": scheme(&[&ETHANE, &ETHANOL], &[(0, 1, "")]), "commands": [{"command": "pathway:label", "args": bad}]}).to_string());
        assert!(r.is_err(), "{bad}");
    }
}

#[test]
fn the_drawing_shows_a_true_minus_and_the_data_keeps_the_ascii_one() {
    assert_eq!(display("-CH2 -Cl+OH"), "\u{2212}CH2 \u{2212}Cl+OH");
    assert_eq!(display("re-do -x"), "re-do -x");
    let mut e = editor(&scheme(&[&PROPANE, &ETHANE], &[(0, 1, "")]));
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), ["-CH2"]);
    let svg = e.save("svg").unwrap();
    assert!(
        svg.contains("\u{2212}CH2") && !svg.contains(">-CH2<"),
        "{svg}"
    );
}

#[test]
fn a_label_makes_room_the_gap_between_columns_grows_with_the_text() {
    let base = scheme(&[&ETHANE, &ETHANOL], &[(0, 1, "")]);
    let gap = |text: &str| {
        let mut d = Document::from_json(&base).unwrap();
        let _ = &mut d;
        let t = scheme(&[&ETHANE, &ETHANOL], &[(0, 1, text)]);
        let mut e = editor(&t);
        e.run_command("pathway:tree", &json!({})).unwrap();
        let o = &e.doc().objects[0];
        (o.x2 - o.x).abs()
    };
    let short = gap("");
    let long = gap("a long reagent text over the arrow of this step");
    assert!(long > short + 1.0, "{short} {long}");
    // the visible arrow is as long as the text: no column gap is wasted when there is no text
    assert!(short < 3.0, "{short}");
}

#[test]
fn a_thousand_arrows_are_labelled_quickly_and_more_are_refused() {
    // 1001 structures in a chain of arrows, built as a document: the cap is 1000 arrows
    let n = 1000usize;
    let (mut atoms, mut bonds, mut objects) = (vec![], vec![], vec![]);
    for k in 0..=n {
        let id = 2 * k + 1;
        atoms.push(json!({"id": id, "el": "C", "x": 4.0 * k as f64, "y": 0}));
        atoms.push(json!({"id": id + 1, "el": if k % 2 == 0 { "C" } else { "O" }, "x": 4.0 * k as f64 + 1.0, "y": 0}));
        bonds.push(json!({"id": 100_000 + id, "a": id, "b": id + 1, "o": 1}));
        if k < n {
            objects.push(json!({"id": 200_000 + k, "t": "arrow", "x": 4.0 * k as f64 + 2.0, "y": 0, "x2": 4.0 * k as f64 + 3.5, "y2": 0, "from": id, "to": id + 2}));
        }
    }
    let doc = |objs: &[Value]| {
        json!({"format": "mzdraw/1", "next": 900_000, "atoms": atoms, "bonds": bonds, "objects": objs}).to_string()
    };
    let mut e = editor(&doc(&objects));
    let t = std::time::Instant::now();
    run(&mut e, json!({})).unwrap();
    let dt = t.elapsed();
    let t = texts(&e);
    assert_eq!(
        (t[0].as_str(), t[1].as_str(), t.len()),
        ("+O-CH2", "+CH2-O", n)
    );
    // a quadratic version (a rescan of the pieces or of the objects per arrow) takes seconds here; debug builds get a wider bound
    let bound = if cfg!(debug_assertions) { 20_000 } else { 1500 };
    assert!(dt.as_millis() < bound, "{dt:?}");
    // doubling the arrows less than quadruples the time is not asserted (the cap is the guard): one more arrow is refused
    let mut more = objects.clone();
    more.push(json!({"id": 300_000, "t": "arrow", "x": 1.0, "y": 1, "x2": 2.5, "y2": 1, "from": 1, "to": 3}));
    let mut e = editor(&doc(&more));
    assert_eq!(
        run(&mut e, json!({})).unwrap_err().key,
        "error.draw2.schemeSize"
    );
}
