//! CIP labels in the picture (`display.cip`, rdkit-compare 6), in the core, with a labeller made here (the real engine is the glue's,
//! tested in `crates/mzdraw-wasm/tests/cip.rs`): off by default; on, every label of the labeller is drawn in brackets at its atom or
//! bond, in the canvas, the SVG, the PDF and the PNG; the labeller runs once per change of the drawing; without a labeller nothing is
//! drawn and the result says so. Synthetic structures only.

use mzdraw::cip::CipLabel;
use mzdraw::{Document, Editor, Op};
use serde_json::json;
use std::cell::Cell;

thread_local! {
    /// How many times the labeller ran on this test's thread (each test has its own thread).
    static CALLS: Cell<usize> = const { Cell::new(0) };
}

fn calls() -> usize {
    CALLS.with(Cell::get)
}

/// A labeller for the tests: atom 2 is S and the bond 7 (when there is one) is E, whatever the drawing.
fn fake(d: &Document) -> Result<Vec<CipLabel>, &'static str> {
    CALLS.with(|c| c.set(c.get() + 1));
    let mut out = vec![CipLabel {
        atom: 2,
        bond: 0,
        text: "S".into(),
    }];
    if d.bonds.iter().any(|b| b.id == 7) {
        out.push(CipLabel {
            atom: 0,
            bond: 7,
            text: "E".into(),
        });
    }
    Ok(out)
}

fn editor(text: &str) -> Editor {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e.labeller = Some(fake);
    e
}

#[test]
fn the_labels_are_off_by_default_and_drawn_in_brackets_when_on() {
    let mut e = editor("C[C@H](N)C(=O)O");
    assert!(!e.svg().contains(">(S)<"), "off by default");
    assert_eq!(e.get("display.cip").as_deref(), Some("off"));
    e.set("display.cip=on").unwrap();
    let svg = e.svg();
    assert_eq!(
        svg.matches(">(S)<").count(),
        1,
        "the label of the labeller, once"
    );
    // the PDF and the PNG (the SVG the page draws) carry it too
    let pdf: String = e
        .pdf()
        .iter()
        .map(|&c| if c < 128 { c as char } else { '?' })
        .collect();
    assert!(pdf.contains("(\\(S\\)) Tj"), "the PDF writes the label");
    assert!(e.png_plan().svg.contains(">(S)<"));
    // off again: gone; a word that is none is refused and changes nothing
    e.set("display.cip=off").unwrap();
    assert!(!e.svg().contains(">(S)<"));
    assert_eq!(e.set("display.cip=yes"), Err("error.draw2.styleValue"));
}

#[test]
fn a_label_sits_by_its_atom_or_its_bond() {
    // but-2-ene with a centre: atoms 1..=4 the alkene, bonds after; the fake labels atom 2 and bond 7
    let mut e = editor("C/C=C/C");
    e.set("display.cip=on").unwrap();
    let svg = e.svg();
    let at = |t: &str| -> (f64, f64) {
        let el = svg
            .split("<text")
            .find(|x| x.contains(&format!(">{t}<")))
            .unwrap();
        let attr = |k: &str| -> f64 {
            el.split(&format!(" {k}=\""))
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
                .parse()
                .unwrap()
        };
        (attr("x"), attr("y"))
    };
    let (s, e_bond) = (at("(S)"), at("(E)"));
    // the labels are near the drawing: inside the page, apart from each other
    let w: f64 = svg
        .split("width=\"")
        .nth(1)
        .unwrap()
        .split("pt")
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(s.0 > 0.0 && s.0 < w && e_bond.0 > 0.0 && e_bond.0 < w);
    assert!(
        ((s.0 - e_bond.0).powi(2) + (s.1 - e_bond.1).powi(2)).sqrt() > 5.0,
        "{s:?} {e_bond:?}"
    );
}

#[test]
fn the_labeller_runs_once_per_change_of_the_drawing() {
    let mut e = editor("C[C@H](N)C(=O)O");
    e.set("display.cip=on").unwrap();
    let before = calls();
    for _ in 0..5 {
        e.svg();
        e.scene();
    }
    let after = calls();
    assert_eq!(after - before, 1, "five pictures of one drawing, one call");
    e.session
        .run(&[Op::parse(&json!({"op": "atom.set", "id": 3, "el": "O"})).unwrap()])
        .unwrap();
    e.svg();
    e.svg();
    assert_eq!(calls() - after, 1, "a change, one more call");
}

#[test]
fn without_a_labeller_nothing_is_drawn_and_the_result_says_so() {
    let mut e = Editor::default();
    e.import("C[C@H](N)C(=O)O").unwrap();
    e.set("display.cip=on").unwrap();
    assert!(!e.svg().contains(">(S)<"));
    assert!(e.lost_on_save("svg").contains(&(mzdraw::cip::NO_ENGINE, 1)));
    assert!(
        !e.lost_on_save("molfile")
            .iter()
            .any(|l| l.0 == mzdraw::cip::NO_ENGINE),
        "not a picture: nothing to say"
    );
    // the replay request says it in lost
    let v: serde_json::Value = serde_json::from_str(
        &mzdraw::replay::replay(&json!({"import": "C[C@H](N)C(=O)O", "set": ["display.cip=on"], "out": "svg", "report": true}).to_string())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        v["lost"],
        json!([{"key": "draw2.export.cipEngine", "n": 1}])
    );
    // the settings file keeps the choice
    let file = e.save("settings").unwrap();
    assert!(file.contains("\"cip\":\"on\""), "{file}");
    assert!(mzdraw::Style::from_text(&file).unwrap().cip);
}

thread_local! {
    /// The number of atoms of each document the piece labeller was given, in order.
    static GIVEN: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A labeller that records what it is given, labels the first atom of each structure `R`, and refuses a structure with a nitrogen.
fn by_piece(d: &Document) -> Result<Vec<CipLabel>, &'static str> {
    GIVEN.with(|g| g.borrow_mut().push(d.atoms.len()));
    if d.atoms.iter().any(|a| a.el == "N") {
        return Err("error.draw2.cipEngine");
    }
    Ok(vec![CipLabel {
        atom: d.atoms[0].id,
        bond: 0,
        text: "R".into(),
    }])
}

#[test]
fn the_engine_is_given_each_structure_on_its_own_and_what_it_refuses_or_is_not_given_is_said() {
    // three structures: one labelled, one refused (a nitrogen), one with too many open sites (a chain of 17 undrawn centres)
    let open = format!("C{}C", "C(C)".repeat(mzdraw::cip::MAX_OPEN + 1));
    let text = format!("C[C@H](O)CC.CCN.{open}");
    let mut e = Editor::default();
    e.import(&text).unwrap();
    let pieces = mzdraw::layout::pieces(e.doc());
    assert_eq!(pieces.len(), 3);
    assert!(mzdraw::cip::open_sites(e.doc(), &pieces[2]) > mzdraw::cip::MAX_OPEN);
    assert!(mzdraw::cip::open_sites(e.doc(), &pieces[0]) <= 1);
    e.labeller = Some(by_piece);
    e.set("display.cip=on").unwrap();
    GIVEN.with(|g| g.borrow_mut().clear());
    let done = e.cip_labelled();
    // one call per structure given, each with that structure alone; the capped one is not given
    let given = GIVEN.with(|g| g.borrow().clone());
    assert_eq!(given, [pieces[0].len(), pieces[1].len()]);
    assert_eq!(done.labels.len(), 1);
    assert_eq!((done.capped, done.failed), (1, 1));
    // the notes: on the page (`cip_notes`) and in `lost` of a picture
    let notes = vec![(mzdraw::cip::CAPPED, 1), (mzdraw::cip::FAILED, 1)];
    assert_eq!(e.cip_notes(), notes);
    for kind in ["svg", "pdf", "png"] {
        let lost = e.lost_on_save(kind);
        assert!(notes.iter().all(|n| lost.contains(n)), "{kind}: {lost:?}");
    }
    assert!(!e
        .lost_on_save("molfile")
        .iter()
        .any(|l| l.0 == mzdraw::cip::CAPPED));
    // the label of the labelled structure is drawn, the others draw none
    assert_eq!(e.svg().matches(">(R)<").count(), 1);
    // off: nothing is asked and nothing is said
    e.set("display.cip=off").unwrap();
    assert!(e.cip_notes().is_empty());
}

#[test]
fn one_structure_per_call_whatever_the_number_of_structures() {
    // the work of the engine grows exponentially with the open sites of what it is given: the drawing is never given whole. The
    // structure of the review (two open double bonds, open ring centres), 8 and 40 times: every call has one structure's atoms
    let one = "CC(C)C=CC(=O)NC1CCC(CC1)C=CC";
    for n in [8, 40] {
        let mut e = Editor::default();
        e.import(&vec![one; n].join(".")).unwrap();
        let size = e.doc().atoms.len() / n;
        e.labeller = Some(by_piece);
        e.set("display.cip=on").unwrap();
        GIVEN.with(|g| g.borrow_mut().clear());
        e.cip_labelled();
        let given = GIVEN.with(|g| g.borrow().clone());
        assert_eq!(given, vec![size; n], "{n} structures");
    }
}
