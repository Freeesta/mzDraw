//! The step field (tp-11): the filter, the keys that are the field's only while it is open, the overlay, the tap on a ring, one undo
//! step. Synthetic structures only.

use mzdraw::editor::{ALT, HANDLED, SHIFT, STEP};
use mzdraw::tp;
use mzdraw::Editor;
use serde_json::Value;

fn trimethylamine() -> Editor {
    let mut e = Editor::default();
    e.style.pathway_derive = true;
    e.import("CN(C)C").unwrap();
    e
}

fn field(e: &mut Editor) -> Value {
    serde_json::from_str(&e.step_json()).unwrap()
}

fn names(rows: &[&tp::Step]) -> Vec<&'static str> {
    rows.iter().map(|s| s.name).collect()
}

fn no_mass(_: &str) -> Option<f64> {
    None
}

#[test]
fn filter_by_delta_name_and_mass() {
    let all = tp::filter("", &no_mass);
    assert_eq!(all.len(), tp::STEPS.len());
    // the delta `+O` is exactly `+O`, not `+O3S` or `+C2H2O`; table order
    assert_eq!(
        names(&tp::filter("+O", &no_mass)),
        ["hydroxylation", "n-oxidation"]
    );
    assert_eq!(
        names(&tp::filter("-CH2", &no_mass)),
        ["n-demethylation", "o-demethylation"]
    );
    // while typing a longer delta the prefix selects
    assert!(names(&tp::filter("+C6", &no_mass)).contains(&"glucuronidation"));
    // part of a name, any case
    assert_eq!(
        names(&tp::filter("DEMETH", &no_mass)),
        ["n-demethylation", "o-demethylation"]
    );
    assert!(tp::filter("nothing", &no_mass).is_empty());
    // a mass shift: with no engine nothing matches, with masses the rows whose shift starts with the text
    assert!(tp::filter("+15.99", &no_mass).is_empty());
    let shift = |d: &str| match d {
        "+O" => Some(15.9949),
        "-CH2" => Some(-14.0157),
        _ => None,
    };
    assert_eq!(
        names(&tp::filter("+15.99", &shift)),
        ["hydroxylation", "n-oxidation"]
    );
    assert_eq!(
        names(&tp::filter("-14.0", &shift)),
        ["n-demethylation", "o-demethylation"]
    );
    assert!(tp::filter("+16.5", &shift).is_empty());
}

#[test]
fn bare_brackets_do_nothing_with_the_field_closed_and_alt_brackets_rotate() {
    let mut e = trimethylamine();
    assert_eq!(e.key("[", 0), 0);
    assert_eq!(e.key("]", 0), 0);
    e.select_all();
    let before = e.doc().to_json();
    assert_ne!(e.key("[", ALT) & HANDLED, 0);
    assert_ne!(e.doc().to_json(), before);
    // the field open: Alt+[ and Alt+] still turn the selection
    e.step_open(Some(1));
    let before = e.doc().to_json();
    assert_ne!(e.key("]", ALT) & HANDLED, 0);
    assert_ne!(e.doc().to_json(), before);
}

#[test]
fn the_open_field_has_its_keys_and_walks_the_sites() {
    let mut e = trimethylamine();
    e.over.focus = Some(2);
    assert_ne!(e.step_open(None) & STEP, 0);
    let f = field(&mut e);
    assert_eq!(f["open"], true);
    assert_eq!(f["rows"].as_array().unwrap().len(), tp::STEPS.len());
    e.step_text("+O");
    let f = field(&mut e);
    assert_eq!(f["step"], "hydroxylation");
    assert_eq!(f["sites"], serde_json::json!([1, 3, 4]));
    assert_eq!(f["site"], 0);
    // the overlay: a ring per site (and a second on the focused one), a dot and a line for the added O; the document is untouched
    let doc = e.doc().to_json();
    // three sites: a ring each (5 numbers) and a second on the focused one; the ghost adds a dot (5) and a line (7)
    let with = e.scene().data.len();
    e.step_text("zzz");
    let without = e.scene().data.len();
    assert_eq!(with - without, 3 * 5 + 5 + 5 + 7);
    e.step_text("+O");
    assert_ne!(e.key("]", 0) & STEP, 0);
    assert_ne!(e.key("]", 0) & STEP, 0);
    assert_eq!(field(&mut e)["site"], 2);
    assert_eq!(field(&mut e)["at"], "C4");
    e.key("]", 0);
    assert_eq!(field(&mut e)["site"], 0, "round");
    e.key("[", 0);
    assert_eq!(field(&mut e)["site"], 2);
    assert_eq!(e.doc().to_json(), doc, "no operation before Enter");
    // Enter derives: one undo step, the field closes
    let atoms = e.doc().atoms.len();
    assert_ne!(e.key("Enter", 0) & mzdraw::editor::CHANGED, 0);
    assert_eq!(field(&mut e)["open"], false);
    assert_eq!(e.doc().atoms.len(), 2 * atoms + 1);
    e.undo();
    assert_eq!(e.doc().to_json(), doc, "one undo step, exact");
}

#[test]
fn escape_closes_and_other_keys_are_not_the_fields() {
    let mut e = trimethylamine();
    e.step_open(Some(1));
    // a letter is typed into the field (not handled here), and does not edit the drawing
    let before = e.doc().to_json();
    assert_eq!(e.key("o", 0), 0);
    assert_eq!(e.doc().to_json(), before);
    assert_ne!(e.key("Escape", 0) & STEP, 0);
    assert_eq!(field(&mut e)["open"], false);
    // Shift+T has no step key (the kill test of tp-11 did not pass): it is still the text tool, with the pathway features on or off
    for on in [false, true] {
        let mut plain = Editor::default();
        plain.style.pathway_derive = on;
        plain.import("CCO").unwrap();
        assert_eq!(plain.key("T", SHIFT) & STEP, 0);
        assert_ne!(plain.key("T", SHIFT) & mzdraw::editor::ASK_TEXT, 0);
        assert_eq!(field(&mut plain)["open"], false);
    }
}

#[test]
fn a_tap_on_a_ring_focuses_the_site_and_derive_runs_it() {
    let mut e = trimethylamine();
    e.step_open(Some(1));
    e.step_text("hydroxylation");
    let target = e.doc().atom(3).map(|a| (a.x, a.y)).unwrap();
    let (px, py) = e.view.to_px(target.0, target.1);
    let (touch, down, up) = (2, 0, 2);
    let f = e.pointer_event(down, 7, px, py, touch, 0, 0.0);
    assert_ne!(f & HANDLED, 0);
    e.pointer_event(up, 7, px, py, touch, 0, 20.0);
    let f = field(&mut e);
    assert_eq!(f["site"], 1);
    assert_eq!(f["at"], "C3");
    let doc = e.doc().to_json();
    assert_eq!(doc, e.doc().to_json(), "a tap makes no operation");
    // the Derive button
    assert_ne!(e.step_derive() & mzdraw::editor::CHANGED, 0);
    assert_eq!(field(&mut e)["open"], false);
}

#[test]
fn a_change_of_the_history_finds_the_sites_again_and_losing_the_atom_closes_the_field() {
    let mut e = trimethylamine();
    e.step_open(Some(1));
    e.step_text("hydroxylation");
    e.key("]", 0);
    assert_eq!(field(&mut e)["sites"], serde_json::json!([1, 3, 4]));
    assert_eq!(field(&mut e)["site"], 1);
    // an atom added elsewhere: the field stays, the sites are those of the drawing now, the focused site stays on atom 3
    let add = mzdraw::ops::Op::AtomAdd {
        id: None,
        el: "C".into(),
        x: 9.0,
        y: 9.0,
        map: None,
    };
    e.session.run(&[add]).unwrap();
    let f = field(&mut e);
    assert_eq!(f["open"], true);
    assert_eq!(f["sites"], serde_json::json!([1, 3, 4]));
    assert_eq!(f["at"], "C3");
    // a turn of the selection (Alt+]) with the field open leaves it open
    e.over.sel_atoms = vec![1, 2, 3, 4];
    assert_ne!(e.key("]", ALT) & HANDLED, 0);
    assert_eq!(field(&mut e)["open"], true);
    // the atom it was opened on removed: closed
    let gone = mzdraw::ops::Op::Remove {
        atoms: vec![1],
        bonds: Vec::new(),
        objects: Vec::new(),
    };
    e.session.run(&[gone]).unwrap();
    assert_eq!(field(&mut e)["open"], false);
    assert!(!e.scene().data.is_empty());
}

#[test]
fn the_field_needs_an_atom() {
    let mut e = Editor::default();
    e.style.pathway_derive = true;
    assert_eq!(e.step_open(None) & STEP, 0);
    assert_eq!(field(&mut e)["open"], false);
}
