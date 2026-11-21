//! Arrow labels computed from the two structures (tp-07): `pathway:label`. Synthetic structures only (chains written here).

use mzdraw::editor::CHANGED;
use mzdraw::steplabel::{display, owned, Mode, Opts};
use mzdraw::{Document, Editor};
use serde_json::{json, Value};

#[path = "support/timing.rs"]
mod timing;

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
fn modes_that_need_masses_are_refused_with_their_key_and_change_nothing() {
    for (mode, key) in [
        ("mass", "error.draw2.labelMass"),
        ("delta+mass", "error.draw2.labelMass"),
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
    // methylamine -> methylammonium: exactly `+H`
    let amine = [("C", "", 0), ("N", "", 0)];
    let ammonium = [("C", "", 0), ("N", "", 1)];
    let (t, e) = label_of(&amine, &ammonium);
    assert_eq!(t, "+H", "the record gives the N+ one more hydrogen");
    let _ = e;
}

#[test]
fn the_same_atoms_with_another_charge_are_skipped_not_called_an_isomer() {
    // two single atoms of nitrogen with the same hydrogens (explicit) and charges 0 and +1: equal counts, another net charge
    let mut d = Document::from_json(&pair(&[("N", "", 0)], &[("N", "", 0)])).unwrap();
    for a in &mut d.atoms {
        a.h = Some(3);
    }
    d.atoms[1].chg = 1;
    let p = mzdraw::steplabel::plan(&d, Mode::Delta, &Opts::default(), None).unwrap();
    assert!(
        p.ops.is_empty() && p.skipped == 1,
        "{:?} {}",
        p.ops,
        p.skipped
    );
}

#[test]
fn a_free_text_label_is_not_an_element_and_its_arrow_is_skipped() {
    // a document holds only table abbreviations and R groups as labels, but a `Document` built in memory may hold any text: the record gives
    // such an atom a helium stand-in, which must never be read as an element (`+He`)
    let plain = [("C", "", 0), ("C", "", 0)];
    let mut d = Document::from_json(&pair(&plain, &plain)).unwrap();
    d.atoms[0].lab = "Xyz".into();
    let p = mzdraw::steplabel::plan(&d, Mode::Delta, &Opts::default(), None).unwrap();
    assert!(
        p.ops.is_empty() && (p.labelled, p.kept, p.skipped) == (0, 0, 1),
        "{:?}",
        p.ops
    );
    d.atoms[0].lab = "Ph".into();
    let p = mzdraw::steplabel::plan(&d, Mode::Delta, &Opts::default(), None).unwrap();
    assert_eq!(
        (p.skipped, p.ops.len()),
        (0, 1),
        "a table abbreviation is counted"
    );
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

/// The least CPU time of the thread over seven runs (after one warm-up) of `steplabel::plan` on `text`, in nanoseconds (the import and the application of the operations, which the
/// document layer owns, are not timed).
fn timed(text: &str) -> u128 {
    let doc = Document::from_json(text).unwrap();
    timing::best_of(1, 7, || {
        let p = mzdraw::steplabel::plan(&doc, Mode::Delta, &Opts::default(), None).unwrap();
        assert!(!p.ops.is_empty());
    })
    .as_nanos()
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
        large <= 9 * small,
        "250 arrows {small} ns, 1000 arrows {large} ns"
    );
    // a hostile document: 1000 arrows and fifty thousand other objects (a scan of the objects per arrow costs 5e7 steps)
    let (base, crowded) = (timed(&chain(1000, 0)), timed(&chain(1000, 50_000)));
    assert!(
        crowded <= 6 * base,
        "{base} ns without, {crowded} ns with 50000 plus signs"
    );
}

const PROPENE: S = S(&["C", "C", "C"], &[1, 2]);

#[test]
fn name_and_delta_writes_the_name_of_the_one_step_with_that_delta_else_the_delta_alone() {
    // -H2 is dehydrogenation only; +O is shared by hydroxylation and n-oxidation, so it has no name
    let mut e = editor(&scheme(
        &[&ETHANE, &ETHENE, &ETHANOL, &PROPANE, &PROPENE],
        &[(0, 1, ""), (0, 2, ""), (3, 4, "")],
    ));
    e.set("pathway.label=name+delta").unwrap();
    run(&mut e, json!({})).unwrap();
    assert_eq!(
        texts(&e),
        ["dehydrogenation -H2", "+O", "dehydrogenation -H2"]
    );
    // the written label is ours: a second run is a no-op, and after the structures change it follows them
    let once = e.doc().to_json();
    run(&mut e, json!({})).unwrap();
    assert_eq!(e.doc().to_json(), once);
    assert!(owned("dehydrogenation -H2", "isomer") && owned("hydroxylation", "isomer"));
    assert!(
        !owned("hydroxylation (CYP)", "isomer"),
        "a remark makes it the person's"
    );
    // a delta with two steps, and a delta that no step has, are not named
    let mut e = editor(&scheme(
        &[&PROPANE, &ETHANE, &ETHANOL],
        &[(0, 1, ""), (1, 2, "")],
    ));
    e.set("pathway.label=name+delta").unwrap();
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), ["-CH2", "+O"]);
    // the mode `delta` still writes the delta alone
    let mut e = editor(&scheme(&[&ETHANE, &ETHENE], &[(0, 1, "")]));
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), ["-H2"]);
}

// ---- tp-07b: the mass modes. A stand-in engine with made-up weights, so these tests are about the label, never about a mass.

use mzdraw::values::Measured;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The made-up weight of an element (u): not a real mass. `O` is a binary fraction on purpose: the exact tie `0.03125` has 4-decimal
/// roundings that differ between the two formatters of this repository.
fn weight(el: &str) -> f64 {
    match el {
        "C" => 12.0,
        "O" => 16.5,
        "F" => 0.03125,
        _ => 1.0,
    }
}

/// The stand-in: the sum of the made-up weights of the atom lines of the molfile; an adduct adds 1.5; a charged structure has no adduct;
/// nitrogen is an element the "engine" refuses (as `sel` refuses `Na+`).
fn engine(molfile: &str, adducts: &[String]) -> Result<Measured, &'static str> {
    let atoms: Vec<&str> = molfile
        .lines()
        .filter_map(|l| {
            let t: Vec<&str> = l.split_whitespace().collect();
            (t.len() >= 4 && t[0].contains('.') && t[3].chars().all(|c| c.is_ascii_alphabetic()))
                .then(|| t[3])
        })
        .collect();
    if atoms.contains(&"N") {
        return Err("error.grafo.element");
    }
    let mass: f64 = atoms.iter().map(|e| weight(e)).sum();
    let charged = molfile.contains("M  CHG");
    Ok(Measured {
        mass,
        average: None,
        ion_mz: None,
        pct: Vec::new(),
        mz: adducts
            .iter()
            .map(|_| (!charged).then_some(mass + 1.5))
            .collect(),
    })
}

fn mass_editor(text: &str, mode: &str) -> Editor {
    let mut e = editor(text);
    e.set(&format!("pathway.label={mode}")).unwrap();
    e.measurer = Some(engine);
    e
}

const FLUOROETHANE: S = S(&["C", "C", "F"], &[1, 1]);

#[test]
fn mass_modes_write_the_shift_of_the_engine_and_the_m_z_of_the_chosen_adduct() {
    let s = scheme(&[&ETHANE, &ETHANOL], &[(0, 1, "")]);
    // the structures weigh 24.0 and 40.5 here: shift +16.5, the product's [M+H]+ 40.5 + 1.5
    for (mode, args, want) in [
        ("mass", json!({}), "+16.5000"),
        ("delta+mass", json!({}), "+O +16.5000"),
        (
            "mass",
            json!({"adduct": "[M+H]+"}),
            "+16.5000 [M+H]+ 42.0000",
        ),
        (
            "delta+mass",
            json!({"adduct": "[M-H]-"}),
            "+O +16.5000 [M-H]- 42.0000",
        ),
    ] {
        let mut e = mass_editor(&s, mode);
        run(&mut e, args.clone()).unwrap();
        assert_eq!(texts(&e), [want], "{mode} {args}");
        // the label is ours: a second run is a no-op, and the text is owned (the adduct's number too)
        let once = e.doc().to_json();
        run(&mut e, args).unwrap();
        assert_eq!(e.doc().to_json(), once);
        assert!(owned(want, "isomer"), "{want}");
    }
    // the delta mode is unchanged and does not call the engine
    let mut e = mass_editor(&s, "delta");
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), ["+O"]);
    // one undo step restores the empty text
    e.undo();
    assert_eq!(texts(&e), [""]);
}

#[test]
fn an_isomer_is_its_word_with_no_shift_and_still_gets_the_m_z() {
    let s = scheme(&[&ETHANOL, &ETHER], &[(0, 1, "")]);
    let mut e = mass_editor(&s, "mass");
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), ["isomer"]);
    let mut e = mass_editor(&s, "delta+mass");
    run(&mut e, json!({"adduct": "[M+Na]+"})).unwrap();
    assert_eq!(texts(&e), ["isomer [M+Na]+ 42.0000"]);
}

#[test]
fn the_label_and_the_table_write_the_same_digits_and_the_panel_rounding_is_pinned() {
    // an exact binary tie: F weighs 0.03125, so the shift of ethane -> fluoroethane is 0.03125 (4 decimals: 0.0312 or 0.0313)
    let s = scheme(&[&ETHANE, &FLUOROETHANE], &[(0, 1, "")]);
    let mut e = mass_editor(&s, "mass");
    run(&mut e, json!({})).unwrap();
    // 12 + 12 = 24 and 24.03125: the label is the digits of the table (`{:.4}`, ties to even)
    assert_eq!(texts(&e), ["+0.0312"]);
    let t = mzdraw::tptable::build(e.doc(), &e.style, Some(engine)).unwrap();
    let t: Value = serde_json::from_str(&mzdraw::tptable::json(&t)).unwrap();
    let rows = t["rows"].as_array().unwrap();
    let with_parent: Vec<&Value> = rows.iter().filter(|r| r["parent_id"] != "").collect();
    // no TP_ID data: the parent has an empty name; the pair is the only row with a mass_diff
    let diffs: Vec<&str> = rows
        .iter()
        .filter_map(|r| r["mass_diff"].as_str().filter(|d| !d.is_empty()))
        .collect();
    assert_eq!(diffs, ["+0.0312"], "{with_parent:?}");
    // the shared formatters, and the one rule that differs from the panel of arch-web-17 (`values::four`, half away from zero on an
    // exact tie): pinned here so the difference is stated, not discovered
    assert_eq!(mzdraw::tptable::shift4(0.03125), "+0.0312");
    assert_eq!(mzdraw::tptable::shift4(-0.03125), "-0.0312");
    assert_eq!(mzdraw::tptable::plain4(0.03125), "0.0312");
    assert_eq!(mzdraw::values::four(0.03125), "0.0313");
    // away from an exact tie (every real mass) they agree
    for v in [15.9949146, 134.109550, 0.12346, 1e-6, 1234.5678499] {
        assert_eq!(mzdraw::tptable::plain4(v), mzdraw::values::four(v), "{v}");
    }
    // a zero and a tiny negative shift have no sign
    assert_eq!(mzdraw::tptable::shift4(0.0), "0.0000");
    assert_eq!(mzdraw::tptable::shift4(-0.00001), "0.0000");
    // the painter shows the true minus in a mass shift as in a delta; the data keeps `-`
    assert_eq!(
        display("-14.0157 [M-H]- 1.0000"),
        "\u{2212}14.0157 [M-H]- 1.0000"
    );
}

#[test]
fn a_negative_shift_keeps_its_sign_and_the_painter_shows_the_true_minus() {
    let s = scheme(&[&ETHANOL, &ETHANE], &[(0, 1, "")]);
    let mut e = mass_editor(&s, "delta+mass");
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), ["-O -16.5000"]);
    assert!(owned("-O -16.5000", "isomer"));
    assert_eq!(display("-O -16.5000"), "\u{2212}O \u{2212}16.5000");
}

#[test]
fn a_structure_the_engine_refuses_is_a_keyed_note_and_its_arrow_is_not_labelled() {
    // arrow 0: ethane -> ethanol (measurable); arrow 1: ethanol -> a chain with nitrogen (the stand-in refuses it)
    const AMINE: S = S(&["C", "N"], &[1]);
    let s = scheme(&[&ETHANE, &ETHANOL, &AMINE], &[(0, 1, ""), (1, 2, "")]);
    let mut e = mass_editor(&s, "mass");
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), ["+16.5000", ""]);
    assert_eq!(
        e.said_params,
        [("labelled", 1), ("kept", 0), ("skipped", 1)]
    );
    assert_eq!(e.label_notes, [("error.grafo.element", 1)]);
    // the note is in `lost` of a verdict, as a key and a count, and never silently dropped
    assert!(e
        .lost_on_save("mzdraw")
        .contains(&("error.grafo.element", 1)));
}

#[test]
fn a_charged_product_has_no_adduct_so_the_shift_is_written_and_the_m_z_is_a_note() {
    let acid = [("C", "", 0), ("C", "", 0), ("O", "", 0)];
    let ion = [("C", "", 0), ("C", "", 0), ("O", "", -1)];
    let mut e = mass_editor(&pair(&acid, &ion), "delta+mass");
    run(&mut e, json!({"adduct": "[M+H]+"})).unwrap();
    let t = texts(&e).remove(0);
    assert!(t.starts_with("-H "), "{t}");
    assert!(!t.contains("[M+H]+"), "{t}");
    assert_eq!(e.label_notes, [("draw2.label.mzCharged", 1)]);
}

#[test]
fn the_mass_modes_need_the_engine_and_the_adduct_is_checked_before_anything_runs() {
    let s = scheme(&[&ETHANE, &ETHANOL], &[(0, 1, "")]);
    // no engine: the key of the refusal, nothing changes
    let mut e = editor(&s);
    e.set("pathway.label=mass").unwrap();
    let before = e.doc().to_json();
    assert_eq!(
        run(&mut e, json!({})).unwrap_err().key,
        "error.draw2.labelMass"
    );
    assert_eq!(e.doc().to_json(), before);
    // an adduct outside the seven, a non-text adduct, or an adduct with the delta mode
    for args in [json!({"adduct": "[M+2H]2+"}), json!({"adduct": 1})] {
        let mut e = mass_editor(&s, "mass");
        assert_eq!(run(&mut e, args).unwrap_err().key, "error.draw2.request");
    }
    let mut e = mass_editor(&s, "delta");
    assert_eq!(
        run(&mut e, json!({"adduct": "[M+H]+"})).unwrap_err().key,
        "error.draw2.request"
    );
}

#[test]
fn a_bare_number_is_the_persons_and_only_the_one_after_an_adduct_is_ours() {
    for t in ["151.1117", "42.0000 [M+H]+", "+O 151.1117", "[M+H]+ 151.11"] {
        assert!(!owned(t, "isomer"), "{t}");
    }
    for t in [
        "[M+H]+ 151.1117",
        "+O +15.9949 [M-H]- 1.0000",
        "isomer [M+K]+ 9.5000",
    ] {
        assert!(owned(t, "isomer"), "{t}");
    }
    // a person's measured m/z survives the label run
    let s = scheme(&[&ETHANE, &ETHANOL], &[(0, 1, "151.1117")]);
    let mut e = mass_editor(&s, "mass");
    run(&mut e, json!({})).unwrap();
    assert_eq!(texts(&e), ["151.1117"]);
    assert_eq!(e.said_params[1], ("kept", 1));
}

static CALLS: AtomicUsize = AtomicUsize::new(0);

fn counting(molfile: &str, adducts: &[String]) -> Result<Measured, &'static str> {
    CALLS.fetch_add(1, Ordering::SeqCst);
    engine(molfile, adducts)
}

/// `n` arrows over `n + 1` structures of `k` carbons each (a chain; the last atom alternates C and O so every arrow has a delta), the arrow
/// anchored to the first atoms; `extra` plus signs that are not arrows are listed before the arrows. A scan per arrow of the atoms, of the
/// structures or of the objects walks `n * k` atoms, `n` structures or `extra` objects.
fn big_chain(n: usize, k: usize, extra: usize) -> String {
    let (mut atoms, mut bonds, mut objects) = (vec![], vec![], vec![]);
    for j in 0..extra {
        objects.push(json!({"id": 4_000_000 + j, "t": "plus", "x": j as f64, "y": 5}));
    }
    for s in 0..=n {
        let first = s * k + 1;
        for i in 0..k {
            let el = if i + 1 == k && s % 2 == 1 { "O" } else { "C" };
            atoms.push(json!({"id": first + i, "el": el, "x": 4.0 * s as f64 + i as f64 * 0.1, "y": i as f64}));
            if i > 0 {
                bonds.push(json!({"id": 1_000_000 + first + i, "a": first + i - 1, "b": first + i, "o": 1}));
            }
        }
        if s < n {
            objects.push(json!({"id": 2_000_000 + s, "t": "arrow", "x": 4.0 * s as f64 + 2.0, "y": 0, "x2": 4.0 * s as f64 + 3.5, "y2": 0, "from": first, "to": first + k}));
        }
    }
    json!({"format": "mzdraw/1", "next": 9_000_000, "atoms": atoms, "bonds": bonds, "objects": objects}).to_string()
}

/// The least CPU time of the thread over seven runs (after one warm-up) of `plan` in a mass mode, in nanoseconds, the stand-in engine included (the same cheap function for every size).
fn timed_mass(text: &str) -> u128 {
    let doc = Document::from_json(text).unwrap();
    timing::best_of(1, 7, || {
        let p = mzdraw::steplabel::plan(&doc, Mode::Mass, &Opts::default(), Some(engine)).unwrap();
        assert!(!p.ops.is_empty());
    })
    .as_nanos()
}

#[test]
fn the_mass_labels_ask_the_engine_once_per_structure() {
    // 999 arrows over 1000 structures: the engine is asked 1000 times, not twice per arrow
    let doc = Document::from_json(&chain(999, 0)).unwrap();
    CALLS.store(0, Ordering::SeqCst);
    let p = mzdraw::steplabel::plan(&doc, Mode::Mass, &Opts::default(), Some(counting)).unwrap();
    assert_eq!(p.labelled, 999);
    assert_eq!(CALLS.load(Ordering::SeqCst), 1000);
    assert_eq!(p.ops.len(), 999);
    // the cap of the TP table: 1001 structures are refused before the engine is called
    let doc = Document::from_json(&chain(1000, 0)).unwrap();
    CALLS.store(0, Ordering::SeqCst);
    let err =
        mzdraw::steplabel::plan(&doc, Mode::Mass, &Opts::default(), Some(counting)).unwrap_err();
    assert_eq!(err.key, "error.draw2.valuesSize");
    assert_eq!(CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn the_mass_labels_scale_linearly_in_arrows_atoms_and_objects() {
    // no floor: 4 times the arrows (and atoms) may cost about 4 times; a scan per arrow of the atoms (14 per structure here, so 14,000
    // atoms in the large run), of the structures or of the whole scan costs about 16 times
    let (small, large) = (
        timed_mass(&big_chain(250, 14, 0)),
        timed_mass(&big_chain(999, 14, 0)),
    );
    assert!(
        large <= 8 * small,
        "250 arrows {small} ns, 999 arrows {large} ns (14 atoms each)"
    );
    // a hostile document: 999 arrows and fifty thousand other objects (a scan of the objects per arrow costs 5e7 steps)
    let (base, crowded) = (
        timed_mass(&big_chain(999, 2, 0)),
        timed_mass(&big_chain(999, 2, 50_000)),
    );
    assert!(
        crowded <= 6 * base,
        "{base} ns without, {crowded} ns with 50000 plus signs"
    );
}

/// Two structures of 7,490 carbons each (14,980 atoms, under the cap of the table) and `n` arrows between them, all anchored to the last
/// atoms (an early exit of a lookup by id does not hide a scan): the atoms stay the same while the arrows grow, so only a cost per arrow shows.
fn fan(n: usize) -> String {
    let (mut atoms, mut bonds, mut objects) = (vec![], vec![], vec![]);
    let k = 7490;
    for s in 0..2 {
        for i in 0..k {
            let id = s * k + i + 1;
            let el = if s == 1 && i + 1 == k { "O" } else { "C" };
            atoms.push(json!({"id": id, "el": el, "x": 40.0 * s as f64 + i as f64 * 0.01, "y": i as f64 * 0.01}));
            if i > 0 {
                bonds.push(json!({"id": 1_000_000 + id, "a": id - 1, "b": id, "o": 1}));
            }
        }
    }
    for j in 0..n {
        objects.push(json!({"id": 2_000_000 + j, "t": "arrow", "x": 20.0, "y": j as f64, "x2": 25.0, "y2": j as f64, "from": k, "to": 2 * k}));
    }
    json!({"format": "mzdraw/1", "next": 9_000_000, "atoms": atoms, "bonds": bonds, "objects": objects}).to_string()
}

#[test]
fn the_cost_per_arrow_does_not_grow_with_the_atoms() {
    // the atoms are fixed, so the run is the same work plus the arrows: a scan of the atoms per arrow (999 x 14,980) adds a third or more
    let (small, large) = (timed_mass(&fan(250)), timed_mass(&fan(999)));
    assert!(
        4 * large <= 5 * small,
        "250 arrows {small} ns, 999 arrows {large} ns over 14,980 atoms"
    );
}

#[test]
fn a_note_belongs_to_the_drawing_it_was_made_for() {
    const AMINE: S = S(&["C", "N"], &[1]);
    let s = scheme(&[&ETHANE, &ETHANOL, &AMINE], &[(0, 1, ""), (1, 2, "")]);
    let mut e = mass_editor(&s, "mass");
    run(&mut e, json!({})).unwrap();
    let note = ("error.grafo.element", 1);
    assert!(e.lost_on_save("mzdraw").contains(&note));
    // an edit hides it (the arrow may have changed); undo brings the drawing, and so the note, back
    let id = e.doc().objects[1].id;
    e.session
        .run(&[mzdraw::ops::Op::ObjSet {
            id,
            text: Some("my own".into()),
            runs: None,
            x2: None,
            y2: None,
            from: None,
            to: None,
        }])
        .unwrap();
    assert!(!e.lost_on_save("mzdraw").contains(&note));
    e.undo();
    assert!(e.lost_on_save("mzdraw").contains(&note));
    // a run that fails reports nothing from the run before
    e.set("pathway.label=delta").unwrap();
    assert!(run(&mut e, json!({"adduct": "[M+H]+"})).is_err());
    assert!(e.label_notes.is_empty() && !e.lost_on_save("mzdraw").contains(&note));
    // a loaded document has none
    let mut e = mass_editor(&s, "mass");
    run(&mut e, json!({})).unwrap();
    e.import(&scheme(&[&ETHANE, &ETHANOL], &[(0, 1, "")]))
        .unwrap();
    assert!(e.lost_on_save("mzdraw").iter().all(|l| l.0 != note.0));
}

#[test]
fn the_texts_of_the_label_notes_are_on_the_page() {
    let texts = include_str!("../../../web/lang/mzdraw.js");
    for key in [
        "draw2.label.mzCharged",
        "draw2.label.massMissing",
        "error.draw2.labelMass",
        "error.grafo.element",
        "error.draw2.valuesOpen",
    ] {
        assert!(texts.contains(&format!("\"{key}\"")), "{key}");
    }
}
