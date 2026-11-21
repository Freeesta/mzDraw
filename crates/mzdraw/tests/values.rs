//! The calculated values (arch-web-17), core side: what needs no engine (formula, composition, charge, the selection with its cut bonds
//! closed with hydrogen, the captions, the settings, the request field) and what the editor does with an engine it is given (here a
//! counting stand-in: the real one, with the masses to 4 decimals, is tested in `crates/mzdraw-wasm/tests/values.rs`). Synthetic
//! structures only.

use mzdraw::doc::Document;
use mzdraw::editor::Editor;
use mzdraw::replay::{replay, replay_with};
use mzdraw::values::{self, Measured};
use serde_json::{json, Value};
use std::cell::Cell;

fn doc_of(smiles: &str) -> Document {
    let mut e = Editor::default();
    e.import(smiles).unwrap();
    e.doc().clone()
}

thread_local! {
    /// The calls the stand-in got on this thread (every test has its own).
    static CALLS: Cell<usize> = const { Cell::new(0) };
}

fn calls() -> usize {
    CALLS.with(Cell::get)
}

/// A stand-in for the engine: the mass is the number of atom lines of the molfile, and every adduct gets that plus its position.
fn stand_in(molfile: &str, adducts: &[String]) -> Result<Measured, &'static str> {
    CALLS.with(|c| c.set(c.get() + 1));
    let atoms: f64 = molfile.lines().nth(3).unwrap()[..3].trim().parse().unwrap();
    Ok(Measured {
        mass: atoms,
        average: Some(atoms + 0.5),
        ion_mz: None,
        mz: (0..adducts.len()).map(|k| Some(atoms + k as f64)).collect(),
    })
}

fn refusing(_: &str, _: &[String]) -> Result<Measured, &'static str> {
    panic!("the engine must not be asked");
}

#[test]
fn the_formula_and_the_composition_are_counted_on_the_record_and_agree_with_the_outline() {
    for smiles in [
        "Cn1cnc2c1c(=O)n(C)c(=O)n2C",
        "OCC(O)C(O)C(O)C(O)C=O",
        "C[NH3+].[Cl-]",
        "[O-]S(=O)(=O)[O-].[Na+].[Na+]",
        "CC(=O)Oc1ccccc1C(=O)O",
        "[13CH3]O.CCO",
    ] {
        let doc = doc_of(smiles);
        let scan = values::scan(&doc);
        let outline = mzdraw::outline::pieces(&doc);
        assert_eq!(scan.structures.len(), outline.len(), "{smiles}");
        for (s, o) in scan.structures.iter().zip(&outline) {
            assert_eq!(
                (&s.formula, s.atoms, s.anchor),
                (&o.formula, o.atoms, o.anchor)
            );
        }
    }
    let doc = doc_of("Cn1cnc2c1c(=O)n(C)c(=O)n2C");
    let s = &values::scan(&doc).structures[0];
    let c: Vec<(&str, u32)> = s
        .composition
        .iter()
        .map(|(e, n)| (e.as_str(), *n))
        .collect();
    assert_eq!(c, [("C", 8), ("H", 10), ("N", 4), ("O", 2)]);
    assert_eq!(
        (s.formula.as_str(), s.charge, s.open),
        ("C8H10N4O2", 0, false)
    );
    // charge: the salt is two structures with their own charge each
    let doc = doc_of("C[NH3+].[Cl-]");
    let (a, b) = (
        &values::scan(&doc).structures[0],
        &values::scan(&doc).structures[1],
    );
    assert_eq!((a.formula.as_str(), a.charge), ("CH6N+", 1));
    assert_eq!((b.formula.as_str(), b.charge), ("Cl\u{2212}", -1));
}

#[test]
fn a_selection_is_one_structure_and_a_cut_bond_is_closed_with_hydrogen() {
    let doc = doc_of("CCO");
    let scan = values::scan(&doc);
    let sel = |ids: &[u32]| scan.selection(&doc, ids).unwrap();
    // ids 1, 2, 3 are C, C, O
    assert_eq!(sel(&[1]).formula, "CH4");
    assert_eq!(sel(&[2, 3]).formula, "CH4O");
    assert_eq!(sel(&[1, 2, 3]).formula, "C2H6O");
    // two pieces of one selection are one unit: CH4 and H2O
    assert_eq!(sel(&[3, 1, 1]).formula, "CH6O");
    // no atom of the document is no selection
    assert!(scan.selection(&doc, &[]).is_none());
    assert!(scan.selection(&doc, &[999]).is_none());
    // a double bond cut counts twice: the carbonyl carbon of acetic acid without its two neighbours is formaldehyde
    let acid = doc_of("CC(=O)O");
    let s = values::scan(&acid);
    assert_eq!(s.selection(&acid, &[2, 3]).unwrap().formula, "CH2O");
}

#[test]
fn an_abbreviation_counts_as_the_atoms_it_stands_for_also_in_a_selection() {
    let doc = Document::from_json(
        r#"{"format":"mzdraw/1","next":10,"atoms":[{"id":1,"el":"C","x":0,"y":0},{"id":2,"el":"C","x":1,"y":0,"lab":"Ph"}],"bonds":[{"id":3,"a":1,"b":2,"o":1}]}"#,
    )
    .unwrap();
    let scan = values::scan(&doc);
    assert_eq!(scan.structures[0].formula, "C7H8");
    assert_eq!(scan.structures[0].atoms, 7);
    // the label atom alone: its six atoms, the bond to the methyl closed with hydrogen
    let s = scan.selection(&doc, &[2]).unwrap();
    assert_eq!((s.formula.as_str(), s.atoms), ("C6H6", 6));
}

#[test]
fn an_r_group_is_open_has_no_mass_and_never_reaches_the_engine() {
    let doc = Document::from_json(
        r#"{"format":"mzdraw/1","next":10,"atoms":[{"id":1,"el":"C","x":0,"y":0},{"id":2,"el":"C","x":1,"y":0,"lab":"R1"}],"bonds":[{"id":3,"a":1,"b":2,"o":1}]}"#,
    )
    .unwrap();
    let scan = values::scan(&doc);
    assert!(scan.structures[0].open);
    assert!(scan.structures[0].formula.contains('R'));
    let entries = scan.measure(&[], refusing).unwrap();
    assert_eq!(entries[0].result, Err("error.draw2.valuesOpen"));
    assert_eq!(
        scan.selection(&doc, &[1, 2])
            .unwrap()
            .measure(&[], refusing),
        Err("error.draw2.valuesOpen")
    );
}

#[test]
fn the_engine_is_asked_once_per_structure_and_above_the_caps_not_at_all() {
    let doc = doc_of("CCO.CC.C");
    let before = calls();
    let entries = values::scan(&doc)
        .measure(&["[M+H]+".to_string(), "[M-H]-".to_string()], stand_in)
        .unwrap();
    assert_eq!(calls() - before, 3);
    assert_eq!(entries.len(), 3);
    let r = entries[0].result.as_ref().unwrap();
    assert_eq!((r.mass, r.mz.clone()), (3.0, vec![Some(3.0), Some(4.0)]));
    // more structures than the cap: refused before the first call
    let many = grid(values::MAX_STRUCTURES + 1, 1);
    let before = calls();
    assert_eq!(
        values::scan(&many).measure(&[], stand_in).unwrap_err(),
        "error.draw2.valuesSize"
    );
    // more atoms than the cap, in one structure
    let long = grid(1, values::MAX_ATOMS + 1);
    assert_eq!(
        values::scan(&long).measure(&[], stand_in).unwrap_err(),
        "error.draw2.valuesSize"
    );
    assert_eq!(calls(), before);
    // the formulas stay, whatever the size
    assert_eq!(
        values::scan(&many).structures.len(),
        values::MAX_STRUCTURES + 1
    );
}

/// `n` structures, each a chain of `len` carbon atoms (ids by structure), far apart.
fn grid(n: usize, len: usize) -> Document {
    let (mut atoms, mut bonds) = (Vec::new(), Vec::new());
    let mut id = 1usize;
    let mut bond_ids = Vec::new();
    for k in 0..n {
        let first = id;
        for j in 0..len {
            atoms.push(json!({"id": id, "el": "C", "x": 3.0 * k as f64 + 0.1 * j as f64, "y": (j % 2) as f64 * 0.5}));
            id += 1;
        }
        bond_ids.push((first, len));
    }
    let mut bid = id;
    for (first, len) in bond_ids {
        for j in 1..len {
            bonds.push(json!({"id": bid, "a": first + j - 1, "b": first + j, "o": 1}));
            bid += 1;
        }
    }
    let v = json!({"format": "mzdraw/1", "next": bid + 1, "atoms": atoms, "bonds": bonds});
    Document::from_json(&v.to_string()).unwrap()
}

#[test]
fn without_an_engine_the_values_have_the_formulas_and_no_number() {
    let out = replay(r#"{"import":"CCO","values":true}"#).unwrap();
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["format"], "values/1");
    assert_eq!(
        (v["engine"].clone(), v["error"].clone()),
        (json!(false), Value::Null)
    );
    let s = &v["structures"][0];
    assert_eq!(s["formula"], "C2H6O");
    assert_eq!(s["composition"], json!({"C": 2, "H": 6, "O": 1}));
    assert_eq!(
        (s["mass"].clone(), s["average"].clone(), s["error"].clone()),
        (Value::Null, Value::Null, Value::Null)
    );
    assert_eq!(v["adducts"], json!(["[M+H]+", "[M-H]-"]));
    // a selection is asked by atom ids
    let out = replay(r#"{"import":"CCO","values":{"atoms":[1]}}"#).unwrap();
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["selection"]["formula"], "CH4");
    assert_eq!(v["selection"]["of"], json!([1]));
    assert_eq!(v["selection"]["atoms"], 1);
}

#[test]
fn with_an_engine_the_numbers_are_the_strings_of_four_decimals() {
    let r = replay_with(
        r#"{"import":"CCO","values":{"atoms":[2,3]},"set":["display.adducts=[M+Na]+"]}"#,
        0,
        Some(stand_in),
    )
    .unwrap();
    let v: Value = serde_json::from_str(&r.text).unwrap();
    assert_eq!(v["engine"], true);
    assert_eq!(v["structures"][0]["mass"], "3.0000");
    assert_eq!(v["structures"][0]["average"], "3.5000");
    assert_eq!(v["structures"][0]["mz"], json!({"[M+Na]+": "3.0000"}));
    assert_eq!(v["selection"]["mass"], "2.0000");
}

#[test]
fn the_field_values_stands_alone_and_a_bad_form_is_a_bad_request() {
    for extra in [
        r#""out":"svg""#,
        r#""report":true"#,
        r#""dry_run":true"#,
        r#""expect":[]"#,
    ] {
        let request = format!(r#"{{"import":"CCO","values":true,{extra}}}"#);
        assert_eq!(
            replay(&request).unwrap_err().key,
            "error.draw2.valuesAlone",
            "{extra}"
        );
    }
    for bad in [
        r#""values":1"#,
        r#""values":{"atoms":"1"}"#,
        r#""values":{"atoms":[-1]}"#,
        r#""values":{"x":[1]}"#,
    ] {
        let request = format!(r#"{{"import":"CCO",{bad}}}"#);
        assert_eq!(
            replay(&request).unwrap_err().key,
            "error.draw2.request",
            "{bad}"
        );
    }
    // false is the field left out
    assert!(replay(r#"{"import":"CCO","values":false}"#)
        .unwrap()
        .contains("\"format\":\"mzdraw/1\""));
}

#[test]
fn the_settings_take_the_modes_and_the_adducts_and_a_wrong_word_changes_nothing() {
    let mut e = Editor::default();
    assert_eq!(e.style.values, values::Mode::Off);
    for mode in ["formula", "mass", "mz", "off"] {
        e.set(&format!("display.values={mode}")).unwrap();
        assert_eq!(e.style.values.name(), mode);
    }
    assert_eq!(
        e.set("display.values=loud").unwrap_err(),
        "error.draw2.styleValue"
    );
    e.set("display.adducts=[M+Na]+, [M-H]-").unwrap();
    assert_eq!(e.style.adducts, ["[M+Na]+", "[M-H]-"]);
    e.set("display.adducts=").unwrap();
    assert!(e.style.adducts.is_empty());
    for bad in ["[M+Li]+", "[M+H]+,[M+H]+", "M+H"] {
        assert_eq!(
            e.set(&format!("display.adducts={bad}")).unwrap_err(),
            "error.draw2.styleValue",
            "{bad}"
        );
    }
    assert!(e.style.adducts.is_empty());
    // the settings file keeps them, and a file that says nothing keeps its bytes
    let plain = Editor::default().settings_json();
    assert!(!plain.contains("display."));
    e.set("display.values=mz").unwrap();
    e.set("display.adducts=[M+K]+").unwrap();
    let file = e.settings_json();
    assert!(
        file.contains(r#""display.values":"mz""#) && file.contains(r#""display.adducts":"[M+K]+""#)
    );
    let mut again = Editor::default();
    again.set_style(&file).unwrap();
    assert_eq!(
        (again.style.values, again.style.adducts.clone()),
        (values::Mode::Mz, vec!["[M+K]+".to_string()])
    );
    assert_eq!(again.settings_json(), file);
    // the schema lists the words and the names
    let schema: Value = serde_json::from_str(&mzdraw::settings::schema_json()).unwrap();
    assert_eq!(
        schema["display"]["values"],
        json!(["off", "formula", "mass", "mz"])
    );
    assert_eq!(schema["display"]["adducts"].as_array().unwrap().len(), 7);
}

fn svg_of(smiles: &str, set: &[&str], engine: bool) -> String {
    let request = json!({"import": smiles, "set": set, "out": "svg"}).to_string();
    replay_with(&request, 0, engine.then_some(stand_in as values::Measurer))
        .unwrap()
        .text
}

#[test]
fn the_caption_under_a_structure_follows_the_mode_and_the_page_grows_to_hold_it() {
    let plain = svg_of("CCO", &[], false);
    assert!(!plain.contains("C2H6O</text>"));
    let formula = svg_of("CCO", &["display.values=formula"], false);
    assert!(formula.contains(">C2H6O</text>"));
    let height = |svg: &str| -> f64 {
        let h = svg.split("height=\"").nth(1).unwrap();
        h[..h.find("pt").unwrap()].parse().unwrap()
    };
    assert!(height(&formula) > height(&plain));
    // a mass or an m/z needs the engine; without it the formula stands in, never a made-up number
    assert!(svg_of("CCO", &["display.values=mass"], false).contains(">C2H6O</text>"));
    assert!(svg_of("CCO", &["display.values=mz"], false).contains(">C2H6O</text>"));
    // with it: the formula and the mass, or a line per adduct (the stand-in gives 9 atoms' worth: 3 here)
    assert!(svg_of("CCO", &["display.values=mass"], true).contains(">C2H6O  3.0000</text>"));
    let mz = svg_of(
        "CCO",
        &["display.values=mz", "display.adducts=[M+H]+,[M+Na]+"],
        true,
    );
    assert!(mz.contains(">[M+H]+ 3.0000</text>") && mz.contains(">[M+Na]+ 4.0000</text>"));
    // a structure with a charge shows its own m/z, not an adduct
    // (the stand-in has none: the formula stands in)
    // one caption per structure
    assert_eq!(
        svg_of("CC.CCO", &["display.values=formula"], false)
            .matches("</text>")
            .count()
            - svg_of("CC.CCO", &[], false).matches("</text>").count(),
        2
    );
}

#[test]
fn a_result_a_later_edit_left_behind_is_never_drawn() {
    let mut e = Editor::default();
    e.measurer = Some(stand_in);
    e.import("CCO").unwrap();
    e.set("display.values=mass").unwrap();
    e.refresh_values();
    assert!(e.svg().contains("3.0000"));
    // the oxygen becomes sulfur: another formula, and the held result is for the old one until the next refresh
    let set = mzdraw::ops::Op::parse(&json!({"op": "atom.set", "id": 3, "el": "S"})).unwrap();
    e.session.run(&[set]).unwrap();
    let svg = e.svg();
    assert!(
        !svg.contains("3.0000") && svg.contains(">C2H6S</text>"),
        "{svg}"
    );
    e.refresh_values();
    assert!(e.svg().contains("C2H6S  3.0000"));
    // a mass number the formula does not show is a change too
    let set = mzdraw::ops::Op::parse(&json!({"op": "atom.set", "id": 1, "iso": 13})).unwrap();
    e.session.run(&[set]).unwrap();
    let before = calls();
    e.refresh_values();
    assert_eq!(calls() - before, 1, "a labelled atom is asked again");
    // moving an atom keeps the numbers without asking the engine again
    let before = calls();
    e.refresh_values();
    assert_eq!(calls(), before, "nothing changed: no call");
}

/// The best of five runs of `f`, in microseconds.
fn best(f: impl Fn()) -> u128 {
    (0..5)
        .map(|_| {
            let t = std::time::Instant::now();
            f();
            t.elapsed().as_micros()
        })
        .min()
        .unwrap()
}

/// Four times the size may cost about four times; a scan of all atoms or all bonds for each structure costs sixteen times. There is
/// no floor: the sizes are chosen so that the small run takes milliseconds in a release build too, and the bound 8 leaves a factor two
/// over the linear ratio for noise.
fn assert_linear(what: &str, small: u128, large: u128) {
    assert!(
        large <= 8 * small.max(1),
        "{what}: {small} us, four times the size {large} us"
    );
}

#[test]
fn the_scan_the_captions_and_a_selection_scale_linearly() {
    let (small, large) = (grid(4000, 3), grid(16000, 3));
    assert_linear(
        "scan of 4000 structures",
        best(|| drop(values::scan(&small))),
        best(|| drop(values::scan(&large))),
    );
    let adducts: Vec<String> = Vec::new();
    assert_linear(
        "captions of 4000 structures",
        best(|| {
            drop(values::captions(
                &small,
                values::Mode::Formula,
                &adducts,
                &[],
            ))
        }),
        best(|| {
            drop(values::captions(
                &large,
                values::Mode::Formula,
                &adducts,
                &[],
            ))
        }),
    );
    // a selection of one long chain: all its atoms, cut bonds counted once per bond
    let (small, large) = (grid(1, 4000), grid(1, 16000));
    let sel = |d: &Document, n: u32| {
        let s = values::scan(d);
        drop(s.selection(d, &(1..=n).collect::<Vec<u32>>()).unwrap());
    };
    assert_linear(
        "selection of 4000 atoms",
        best(|| sel(&small, 4000)),
        best(|| sel(&large, 16000)),
    );
    // the sub-records of the structures, up to the cap of 5000 atoms (the stand-in engine: the glue path with the real engine is
    // timed in crates/mzdraw-wasm/tests/values.rs)
    let (small, large) = (grid(25, 50), grid(100, 50));
    // each timed run repeats the work 100 times, so that the small one takes milliseconds in a release build
    let measure = |d: &Document| {
        for _ in 0..100 {
            drop(values::scan(d).measure(&adducts, stand_in).unwrap());
        }
    };
    assert_linear(
        "sub-records of 25 structures of 50 atoms, 100 runs",
        best(|| measure(&small)),
        best(|| measure(&large)),
    );
}

#[test]
fn an_edit_of_one_structure_asks_the_engine_again_for_that_one_only() {
    let mut e = Editor::default();
    e.measurer = Some(stand_in);
    e.import("CCO.CC.C").unwrap();
    let before = calls();
    e.refresh_values();
    assert_eq!(calls() - before, 3);
    // ids 1-3, 4-5, 6: the lone carbon becomes nitrogen
    let set = mzdraw::ops::Op::parse(&json!({"op": "atom.set", "id": 6, "el": "N"})).unwrap();
    e.session.run(&[set]).unwrap();
    let before = calls();
    e.refresh_values();
    assert_eq!(calls() - before, 1, "only the changed structure is asked");
    // a change of the adducts asks for all of them
    e.set("display.adducts=[M+K]+").unwrap();
    let before = calls();
    e.refresh_values();
    assert_eq!(calls() - before, 3);
}
