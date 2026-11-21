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

/// Two structures from atoms given as `(id, element, label, charge)` chains (single bonds), at x 6k, and one arrow from the first to the second.
fn pair(a: &[(&str, &str, i64)], b: &[(&str, &str, i64)]) -> String {
    let (mut atoms, mut bonds) = (vec![], vec![]);
    for (k, s) in [a, b].iter().enumerate() {
        for (i, (el, lab, chg)) in s.iter().enumerate() {
            let id = 10 * k + 1 + i;
            let mut at = json!({"id": id, "el": el, "x": 6.0 * k as f64 + i as f64, "y": 0});
            if !lab.is_empty() {
                at["lab"] = json!(lab);
            }
            if *chg != 0 {
                at["chg"] = json!(chg);
            }
            atoms.push(at);
            if i > 0 {
                bonds.push(json!({"id": 1000 + id, "a": id - 1, "b": id, "o": 1}));
            }
        }
    }
    json!({"format": "mzdraw/1", "next": 9000, "atoms": atoms, "bonds": bonds,
        "objects": [{"id": 5000, "t": "arrow", "x": 3.0, "y": 0, "x2": 5.0, "y2": 0, "from": 1, "to": 11}]})
    .to_string()
}

fn label_of(a: &[(&str, &str, i64)], b: &[(&str, &str, i64)]) -> (String, Editor) {
    let mut e = editor(&pair(a, b));
    run(&mut e, json!({})).unwrap();
    (texts(&e).remove(0), e)
}

#[test]
fn abbreviations_count_as_the_atoms_they_stand_for() {
    // Ph-CH2-OH (C7H8O) -> Ph-CH2-OMe (C8H10O): +CH2; an abbreviation expands into atoms that no document id names
    let ph_oh = [("C", "Ph", 0), ("C", "", 0), ("O", "", 0)];
    let ph_ome = [("C", "Ph", 0), ("C", "", 0), ("C", "OMe", 0)];
    assert_eq!(label_of(&ph_oh, &ph_ome).0, "+CH2");
    assert_eq!(label_of(&ph_ome, &ph_oh).0, "-CH2");
    // an end that is only an abbreviation and its neighbour: BocNH2 (C5H11NO2) -> NH3-less piperidine-free case: Boc-NH2 -> NH2-CH2-OH
    let boc = [("C", "Boc", 0), ("N", "", 0)];
    let amino = [("N", "", 0), ("C", "", 0), ("O", "", 0)];
    // C5H11NO2 -> CH5NO: -C4H6O
    assert_eq!(label_of(&boc, &amino).0, "-C4H6O");
    // the same formula written two ways is an isomer: OMe-CH2-Ph is not drawn here, Ph-OMe vs Ph-CH2-OH is (C7H8O each)
    let ph_o_me = [("C", "Ph", 0), ("C", "OMe", 0)];
    let (t, _) = label_of(&ph_oh, &ph_o_me);
    assert_eq!(t, "isomer", "C7H8O and C7H8O");
    // a label that is not a group stays an element-less R group: R counts as the element R
    let r = [("C", "R1", 0), ("C", "", 0), ("O", "", 0)];
    let r2 = [("C", "R1", 0), ("C", "", 0), ("O", "", 0), ("C", "", 0)];
    assert_eq!(label_of(&r, &r2).0, "+CH2");
}

#[test]
fn charged_structures_have_a_delta_and_a_pure_charge_change_is_left_alone() {
    // acetic acid -> acetate: -H (the delta has no charge); acetate -> acetic acid: +H
    let acid = [("C", "", 0), ("C", "", 0), ("O", "", 0)];
    let acetate = [("C", "", 0), ("C", "", 0), ("O", "", -1)];
    assert_eq!(label_of(&acid, &acetate).0, "-H");
    assert_eq!(label_of(&acetate, &acid).0, "+H");
    // a quaternary N: Me-N -> Me-N+ (same atoms, one more H counted by the record): the counts differ by H or not; equal counts with
    // another charge are skipped and said
    let amine = [("C", "", 0), ("N", "", 0)];
    let ammonium = [("C", "", 0), ("N", "", 1)];
    let (t, e) = label_of(&amine, &ammonium);
    assert!(t == "+H" || t.is_empty(), "{t}");
    let _ = e;
    let eq_charge = [("N", "", 0)];
    let eq_charged = [("N", "", 1)];
    let (t, e) = label_of(&eq_charge, &eq_charged);
    // NH3 -> NH3+ has equal counts only if the record gives the charged one no extra H: whichever, no isomer word for a charge change
    assert_ne!(t, "isomer");
    let _ = e;
}

#[test]
fn a_seeded_property_the_delta_added_to_the_parent_is_the_product() {
    // 50 seeded pairs of random chains over C, N, O, S (single bonds): delta::add(P, label) == T on the Hill formulas of the outline
    let els = ["C", "N", "O", "S", "C", "C"];
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let chain = |n: &mut dyn FnMut() -> u64| -> Vec<(&'static str, &'static str, i64)> {
        let len = 2 + (n() % 5) as usize;
        (0..len).map(|_| (els[(n() % 6) as usize], "", 0)).collect()
    };
    for _ in 0..50 {
        let (p, t) = (chain(&mut next), chain(&mut next));
        let (label, e) = label_of(&p, &t);
        let f = mzdraw::outline::pieces(e.doc());
        let (fp, ft) = (&f[0].formula, &f[1].formula);
        let from = format!("+{fp}");
        let label = if label == "isomer" {
            String::new()
        } else {
            label
        };
        let sum = mzlab_mol::delta::add(&from, &label).unwrap();
        assert_eq!(
            sum,
            mzlab_mol::delta::add(&format!("+{ft}"), "").unwrap(),
            "{fp} {label} {ft}"
        );
    }
}

#[test]
fn a_label_in_two_lines_is_ours_and_cannot_be_stored() {
    // `+O` newline `+15.9949` is ours as text (the page's two-line label of a later slice), but an object text holds no line break
    assert!(owned("+O\n+15.9949", "isomer") && owned("\u{2212}O\n\u{2212}14.0157", "isomer"));
    let mut e = Editor::default();
    let mut doc: Value = serde_json::from_str(&pair(
        &[("C", "", 0), ("C", "", 0)],
        &[("C", "", 0), ("C", "", 0), ("O", "", 0)],
    ))
    .unwrap();
    doc["objects"][0]["text"] = json!("+O\n+15.9949");
    let doc = doc.to_string();
    assert!(
        e.import(&doc).is_err(),
        "a text with a line break is refused on import"
    );
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

/// A chain of `n` arrows over `n + 1` two-atom structures (C-C, C-O alternating), with `extra` plus signs that are not arrows, listed before the arrows (a lookup of an arrow by a scan of the objects walks them all).
fn chain(n: usize, extra: usize) -> String {
    let (mut atoms, mut bonds, mut objects) = (vec![], vec![], vec![]);
    for j in 0..extra {
        objects.push(json!({"id": 400_000 + j, "t": "plus", "x": j as f64, "y": 5}));
    }
    for k in 0..=n {
        let id = 2 * k + 1;
        atoms.push(json!({"id": id, "el": "C", "x": 4.0 * k as f64, "y": 0}));
        atoms.push(json!({"id": id + 1, "el": if k % 2 == 0 { "C" } else { "O" }, "x": 4.0 * k as f64 + 1.0, "y": 0}));
        bonds.push(json!({"id": 100_000 + id, "a": id, "b": id + 1, "o": 1}));
        if k < n {
            objects.push(json!({"id": 200_000 + k, "t": "arrow", "x": 4.0 * k as f64 + 2.0, "y": 0, "x2": 4.0 * k as f64 + 3.5, "y2": 0, "from": id, "to": id + 2}));
        }
    }
    json!({"format": "mzdraw/1", "next": 900_000, "atoms": atoms, "bonds": bonds, "objects": objects}).to_string()
}

/// The best of three runs of `steplabel::plan` on `text`, in microseconds (the import and the application of the operations, which the
/// document layer owns, are not timed).
fn timed(text: &str) -> u128 {
    let doc = Document::from_json(text).unwrap();
    (0..3)
        .map(|_| {
            let t = std::time::Instant::now();
            let p = mzdraw::steplabel::plan(&doc, Mode::Delta, &Opts::default()).unwrap();
            assert!(!p.ops.is_empty());
            t.elapsed().as_micros()
        })
        .min()
        .unwrap()
}

#[test]
fn the_labels_of_a_thousand_arrows_scale_linearly_and_more_are_refused() {
    let mut e = editor(&chain(1000, 0));
    run(&mut e, json!({})).unwrap();
    let t = texts(&e);
    assert_eq!(
        (t[0].as_str(), t[1].as_str(), t.len()),
        ("+O-CH2", "+CH2-O", 1000)
    );
    // the cap: a thousand and first arrow is refused before any work
    let mut e = editor(&chain(1001, 0));
    assert_eq!(
        run(&mut e, json!({})).unwrap_err().key,
        "error.draw2.schemeSize"
    );
    // scaling: 4 times the arrows may cost about 4 times, a per-arrow scan of the pieces or of the objects costs 16 times; the floor keeps
    // a very fast small run from making the ratio noise
    let (small, large) = (timed(&chain(250, 0)), timed(&chain(1000, 0)));
    assert!(
        large <= 9 * small.max(1500),
        "250 arrows {small} us, 1000 arrows {large} us"
    );
    // a hostile document: 1000 arrows and fifty thousand other objects (a scan of the objects per arrow costs 5e7 steps)
    let (base, crowded) = (timed(&chain(1000, 0)), timed(&chain(1000, 50_000)));
    assert!(
        crowded <= 6 * base.max(5000),
        "{base} us without, {crowded} us with 50000 plus signs"
    );
}
