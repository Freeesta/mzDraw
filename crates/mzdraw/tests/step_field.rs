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

#[test]
fn opening_again_resets_the_text_and_altgr_brackets_walk_the_sites() {
    let mut e = trimethylamine();
    e.step_open(Some(1));
    e.step_text("+O");
    assert_eq!(field(&mut e)["text"], "+O");
    // "Apply step..." chosen while the field is open: a fresh field, and the page reads the text from here
    e.step_open(Some(1));
    let f = field(&mut e);
    assert_eq!(f["text"], "");
    assert_eq!(f["rows"].as_array().unwrap().len(), tp::STEPS.len());
    // AltGr arrives as Ctrl and Alt together: `[` and `]` still walk the sites, and the drawing is not turned
    e.step_text("hydroxylation");
    let doc = e.doc().to_json();
    assert_ne!(e.key("]", mzdraw::editor::CTRL | ALT) & STEP, 0);
    assert_eq!(field(&mut e)["site"], 1);
    assert_ne!(e.key("[", mzdraw::editor::CTRL | ALT) & STEP, 0);
    assert_eq!(field(&mut e)["site"], 0);
    assert_eq!(e.doc().to_json(), doc);
    // another Ctrl+Alt key is not the field's
    assert_eq!(e.key("q", mzdraw::editor::CTRL | ALT), 0);
}

#[test]
fn the_text_of_the_field_is_cut_at_64_characters_not_bytes() {
    let mut e = trimethylamine();
    e.step_open(Some(1));
    // 100 characters of two bytes each: the cut is on characters, so it never splits one
    let long: String = "é".repeat(100);
    e.step_text(&long);
    let text = field(&mut e)["text"].as_str().unwrap().to_string();
    assert_eq!(text.chars().count(), 64);
    assert_eq!(text, "é".repeat(64));
    // exactly 64 is kept whole, 65 loses the last one
    e.step_text(&"a".repeat(64));
    assert_eq!(field(&mut e)["text"], "a".repeat(64));
    e.step_text(&format!("{}b", "a".repeat(64)));
    assert_eq!(field(&mut e)["text"], "a".repeat(64));
    // the rows follow the cut text (a name that is not there lists nothing) and the first is chosen again
    let f = field(&mut e);
    assert_eq!(f["rows"].as_array().unwrap().len(), 0);
}

#[test]
fn ctrl_alt_with_a_letter_typed_in_the_open_field_is_not_the_drawings() {
    let mut e = trimethylamine();
    // with the field closed Ctrl+Alt+Z is the undo, as it always was (the change is scoped to the open field)
    let imported = e.doc().to_json();
    e.key("z", mzdraw::editor::CTRL | ALT);
    assert_ne!(e.doc().to_json(), imported);
    e.key("y", mzdraw::editor::CTRL | ALT);
    assert_eq!(e.doc().to_json(), imported);
    // with it open the keys typed into its input (AltGr arrives as Ctrl and Alt) are no undo, redo, select-all or duplicate
    e.step_open(Some(1));
    for k in ["z", "Z", "a", "d", "y"] {
        assert_eq!(e.key(k, mzdraw::editor::CTRL | ALT), 0, "{k}");
        assert_eq!(e.doc().to_json(), imported, "{k}");
    }
    // and with Shift as well (Ctrl+Alt+Shift+Z would be the redo)
    assert_eq!(e.key("z", mzdraw::editor::CTRL | ALT | SHIFT), 0);
    assert_eq!(e.doc().to_json(), imported);
    // the field is still open and still takes its four bracket keys
    assert_eq!(field(&mut e)["open"], true);
    assert_ne!(e.key("}", mzdraw::editor::CTRL | ALT) & STEP, 0);
}

static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A stand-in engine with made-up round masses: the sum of the atoms of the one-structure molfile and its implicit hydrogens.
fn counting_engine(
    molfile: &str,
    _adducts: &[String],
) -> Result<mzdraw::values::Measured, &'static str> {
    CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let table = |el: &str| match el {
        "H" => (1.0, 0),
        "C" => (12.0, 4),
        "N" => (14.5, 3),
        "O" => (16.25, 2),
        "S" => (32.0, 2),
        "Cl" => (35.5, 1),
        "Br" => (79.0, 1),
        _ => (0.0, 0),
    };
    let atoms: Vec<&str> = molfile
        .lines()
        .skip(4)
        .take_while(|l| !l.starts_with("M  END") && l.len() > 31)
        .filter(|l| {
            l[31..34]
                .trim()
                .chars()
                .next()
                .is_some_and(char::is_alphabetic)
        })
        .map(|l| l[31..34].trim())
        .collect();
    // two carbons joined (ethane) have three hydrogens each, a lone atom has its full valence
    let hydrogens: u32 = if atoms.len() == 2 {
        6
    } else {
        atoms.iter().map(|a| table(a).1).sum()
    };
    let mass: f64 = atoms.iter().map(|a| table(a).0).sum::<f64>() + f64::from(hydrogens);
    Ok(mzdraw::values::Measured {
        mass,
        average: None,
        ion_mz: None,
        pct: Vec::new(),
        mz: Vec::new(),
    })
}

#[test]
fn the_element_masses_are_solved_from_seven_engine_calls_and_kept() {
    // H = (2 CH4 - C2H6) / 2, C = CH4 - 4 H, and each other element is its hydride minus its hydrogens: O = H2O - 2 H, N = NH3 - 3 H,
    // S = H2S - 2 H, Cl = HCl - H, Br = HBr - H
    CALLS.store(0, std::sync::atomic::Ordering::SeqCst);
    let m = mzdraw::values::element_masses(counting_engine).unwrap();
    assert_eq!(CALLS.load(std::sync::atomic::Ordering::SeqCst), 7);
    for (el, mass) in [
        ("H", 1.0),
        ("C", 12.0),
        ("N", 14.5),
        ("O", 16.25),
        ("S", 32.0),
        ("Cl", 35.5),
        ("Br", 79.0),
    ] {
        assert!((m[el] - mass).abs() < 1e-9, "{el}: {}", m[el]);
    }
    assert_eq!(m.len(), 7);
    // the field asks the first time it opens and keeps the answer for the session: opening again calls the engine no more
    let mut e = trimethylamine();
    e.measurer = Some(counting_engine);
    CALLS.store(0, std::sync::atomic::Ordering::SeqCst);
    e.step_open(Some(1));
    assert_eq!(CALLS.load(std::sync::atomic::Ordering::SeqCst), 7);
    e.step_close();
    e.step_open(Some(1));
    assert_eq!(CALLS.load(std::sync::atomic::Ordering::SeqCst), 7);
}
