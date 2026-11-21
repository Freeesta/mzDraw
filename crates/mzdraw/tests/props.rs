//! The lines kept under a molecule (struct-props), core side: the operation `atom.cap` (exact undo, canonical list, refusals, unknown
//! keys), the editor commands on a selection, the captions and what they write for a molecule that chose its lines, the values
//! copied as text, the request field `props`, SVG and CDXML, the loss notes, the catalog and the cost of the new code. Synthetic
//! structures only. The numbers that need the engine are tested in `crates/mzdraw-wasm/tests/props.rs`.

use mzdraw::doc::Document;
use mzdraw::editor::Editor;
use mzdraw::replay::replay;
use mzdraw::values::{self, Measured};
use mzdraw::{Op, Session};
use serde_json::{json, Value};

fn editor(smiles: &str) -> Editor {
    let mut e = Editor::default();
    e.import(smiles).unwrap();
    e
}

fn cap(id: u32, lines: &[&str]) -> Op {
    Op::AtomCap {
        id,
        lines: lines.iter().map(|s| s.to_string()).collect(),
    }
}

fn keys(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn the_list_is_canonical_and_a_list_that_cannot_be_kept_is_refused() {
    use mzdraw::props::canon;
    assert_eq!(
        canon(&keys(&[" formula ", "", "logp", "formula", "future"])),
        Some(keys(&["formula", "logp", "future"])),
        "the repeats and the empty keys go, the order stays, an unknown key stays"
    );
    assert_eq!(canon(&[]), Some(Vec::new()));
    assert_eq!(canon(&keys(&[&"k".repeat(41)])), None);
    assert_eq!(canon(&keys(&["a\nb"])), None);
    let many: Vec<String> = (0..25).map(|i| format!("k{i}")).collect();
    assert_eq!(canon(&many), None);
    assert_eq!(canon(&many[..24]).unwrap().len(), 24);
}

#[test]
fn the_operation_sets_clears_and_undoes_exactly_and_is_one_step() {
    let mut s = Session::new(doc_of("CCO"));
    let before = s.doc.to_json();
    s.run(&[cap(1, &["formula", "logp", "logp"])]).unwrap();
    assert_eq!(s.steps(), 1);
    assert_eq!(s.doc.atom(1).unwrap().cap(), keys(&["formula", "logp"]));
    assert!(s.doc.to_json().contains(r#""cap":["formula","logp"]"#));
    // the field round trips through the document text and through the log
    assert_eq!(Document::from_json(&s.doc.to_json()).unwrap(), s.doc);
    assert!(s
        .log_json()
        .contains(r#"{"op":"atom.cap","id":1,"lines":["formula","logp"]}"#));
    s.run(&[cap(1, &[])]).unwrap();
    assert!(s.doc.atom(1).unwrap().cap().is_empty());
    assert_eq!(
        s.doc.to_json(),
        before,
        "the empty list takes the field away"
    );
    assert!(s.undo() && s.undo());
    assert_eq!(s.doc.to_json(), before);
    assert!(s.redo());
    assert_eq!(s.doc.atom(1).unwrap().cap(), keys(&["formula", "logp"]));
    // refused, nothing changes
    let held = s.doc.to_json();
    for op in [cap(99, &["formula"]), cap(1, &[&"k".repeat(41)])] {
        assert!(s.run(&[op]).is_err());
        assert_eq!(s.doc.to_json(), held);
    }
}

fn doc_of(smiles: &str) -> Document {
    editor(smiles).doc().clone()
}

#[test]
fn an_unknown_key_and_a_field_of_another_shape_survive() {
    let text = r#"{"format":"mzdraw/1","next":4,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0,"cap":["formula","future-line"]},{"id":2,"el":"O","x":1.0,"y":0.0,"cap":{"not":"a list"}}],"bonds":[{"id":3,"a":1,"b":2,"o":1}]}"#;
    let d = Document::from_json(text).unwrap();
    assert_eq!(d.atom(1).unwrap().cap(), keys(&["formula", "future-line"]));
    assert!(
        d.atom(2).unwrap().cap().is_empty(),
        "not a list of texts: not a choice"
    );
    assert_eq!(d.to_json(), text, "both are written back as they were read");
    // a line this version does not know writes no line
    let mut e = Editor::default();
    e.open(text).unwrap();
    let c = values::captions(e.doc(), values::Mode::Off, &[], &[]);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].lines, ["CH4O"]);
}

#[test]
fn a_molecule_that_chose_its_lines_writes_them_and_the_others_follow_the_style() {
    let mut e = editor("CCO.CC(=O)Oc1ccccc1C(=O)O");
    // ids 1-3 ethanol, 4-16 aspirin
    e.session
        .run(&[cap(4, &["formula", "hbd", "hba"])])
        .unwrap();
    let c = values::captions(e.doc(), values::Mode::Off, &[], &[]);
    assert_eq!(
        c.len(),
        1,
        "only the molecule with a choice, the style is off"
    );
    assert_eq!(c[0].lines, ["C9H8O4", "HBD 1", "HBA 4"]);
    let c = values::captions(e.doc(), values::Mode::Formula, &[], &[]);
    assert_eq!(c.len(), 2);
    assert_eq!(c[0].lines, ["C2H6O"], "ethanol follows the style");
    assert_eq!(
        c[1].lines,
        ["C9H8O4", "HBD 1", "HBA 4"],
        "aspirin its choice"
    );
    // the descriptors, with the number of decimals the words say
    e.session
        .run(&[cap(4, &["logp", "tpsa", "rotb", "heavy"])])
        .unwrap();
    let c = values::captions(e.doc(), values::Mode::Off, &[], &[]);
    assert_eq!(
        c[0].lines,
        [
            "logP 1.31",
            "tPSA 63.60",
            "Rotatable bonds 2",
            "Heavy atoms 13"
        ]
    );
    // a value that rounds to zero has no sign
    e.session.run(&[cap(1, &["logp"])]).unwrap();
    let c = values::captions(e.doc(), values::Mode::Off, &[], &[]);
    assert_eq!(c[0].lines, ["logP 0.00"]);
}

/// A stand-in for the engine with the numbers of one molecule.
fn engine(_: &str, adducts: &[String]) -> Result<Measured, &'static str> {
    Ok(Measured {
        mass: 46.0419,
        average: Some(46.069),
        ion_mz: None,
        pct: vec![
            ("C".into(), 52.14),
            ("H".into(), 13.13),
            ("O".into(), 34.73),
        ],
        mz: adducts.iter().map(|_| Some(47.0491)).collect(),
    })
}

#[test]
fn the_lines_that_need_the_engine_write_nothing_without_it_and_their_numbers_with_it() {
    let mut e = editor("CCO");
    e.session
        .run(&[cap(1, &["formula", "mw", "exact", "ea", "mz"])])
        .unwrap();
    let adducts = keys(&["[M+H]+", "[M-H]-"]);
    let c = values::captions(e.doc(), values::Mode::Off, &adducts, &[]);
    assert_eq!(c[0].lines, ["C2H6O"]);
    e.measurer = Some(engine);
    e.refresh_values();
    let c = values::captions(e.doc(), values::Mode::Off, &adducts, e.measured_entries());
    assert_eq!(
        c[0].lines,
        [
            "C2H6O",
            "MW 46.0690",
            "Exact mass 46.0419",
            "%C 52.14  %H 13.13  %O 34.73",
            "[M+H]+ 47.0491",
            "[M-H]- 47.0491"
        ]
    );
}

#[test]
fn the_editor_chooses_the_lines_of_the_molecule_of_the_selection_in_one_step() {
    let mut e = editor("CCO.CC");
    // nothing selected, no focus: no molecule
    e.toggle_line("logp", true);
    assert_eq!(e.error, "error.draw2.linesNone");
    e.over.sel_atoms = vec![2];
    assert_eq!(
        e.selection_lines(),
        Some(Vec::new()),
        "the style is off: nothing is written"
    );
    let flags = e.toggle_line("hba", true);
    assert!(flags & mzdraw::editor::CHANGED != 0);
    e.toggle_line("formula", true);
    assert_eq!(
        e.selection_lines(),
        Some(keys(&["formula", "hba"])),
        "the order of the list of lines"
    );
    assert_eq!(
        e.doc().atom(1).unwrap().cap(),
        keys(&["formula", "hba"]),
        "kept on the first atom"
    );
    assert!(e.doc().atom(2).unwrap().cap().is_empty());
    assert!(
        e.doc().atom(4).unwrap().cap().is_empty(),
        "the other molecule is not touched"
    );
    // the same choice again changes nothing (no empty undo step)
    let steps = e.session.steps();
    e.toggle_line("formula", true);
    assert_eq!(e.session.steps(), steps);
    // a first choice on a molecule with none starts from what the style writes
    e.style.values = values::Mode::Mass;
    e.over.sel_atoms = vec![4];
    assert_eq!(e.selection_lines(), Some(keys(&["formula", "exact"])));
    e.toggle_line("exact", false);
    assert_eq!(e.doc().atom(4).unwrap().cap(), keys(&["formula"]));
    // both molecules at once: the choice is set for each
    e.over.sel_atoms = vec![1, 4];
    assert_eq!(e.selection_lines(), None, "they differ");
    e.toggle_line("logp", true);
    assert_eq!(e.error, "error.draw2.linesMixed");
    e.set_lines(&keys(&["tpsa"]));
    assert_eq!(e.doc().atom(1).unwrap().cap(), keys(&["tpsa"]));
    assert_eq!(e.doc().atom(4).unwrap().cap(), keys(&["tpsa"]));
    // one undo step for both; the empty list gives them back to the style
    assert!(e.session.undo());
    assert_eq!(e.doc().atom(1).unwrap().cap(), keys(&["formula", "hba"]));
    e.set_lines(&[]);
    assert!(e.doc().atoms.iter().all(|a| a.cap().is_empty()));
    // refusals
    e.toggle_line("nope", true);
    assert_eq!(e.error, "error.draw2.lines");
    e.set_lines_json("[1]");
    assert_eq!(e.error, "error.draw2.lines");
}

#[test]
fn the_choice_follows_the_molecule_when_it_joins_another_and_when_its_first_atom_goes() {
    let mut e = editor("CCO.CC");
    e.session
        .run(&[cap(1, &["hbd"]), cap(4, &["hba"])])
        .unwrap();
    // a bond between the two molecules: the first atom's choice is the choice of the whole
    e.session
        .run(&[Op::parse(&json!({"op": "bond.add", "a": 3, "b": 4, "o": 1})).unwrap()])
        .unwrap();
    let c = values::scan(e.doc());
    assert_eq!(c.structures.len(), 1);
    assert_eq!(c.structures[0].cap, ["hbd"]);
    // the molecule moves: its caption moves with it (it is derived from its atoms)
    let before = values::captions(e.doc(), values::Mode::Off, &[], &[]);
    e.session
        .run(&[
            Op::parse(&json!({"op": "move", "atoms": [1, 2, 3, 4, 5], "dx": 2.0, "dy": -1.0}))
                .unwrap(),
        ])
        .unwrap();
    let after = values::captions(e.doc(), values::Mode::Off, &[], &[]);
    assert_eq!(before[0].lines, after[0].lines);
    assert!(
        (after[0].x - before[0].x - 2.0).abs() < 1e-9
            && (after[0].y - before[0].y + 1.0).abs() < 1e-9
    );
    // copy and paste carry the choice
    e.over.sel_atoms = vec![1, 2, 3, 4, 5];
    e.copy();
    e.paste(None);
    let c = values::scan(e.doc());
    assert_eq!(c.structures.len(), 2);
    assert_eq!(c.structures[1].cap, ["hbd"]);
}

#[test]
fn the_values_are_copied_as_text_in_three_forms() {
    let mut e = editor("CCO.C");
    e.session
        .run(&[cap(1, &["formula", "hbd", "hba"])])
        .unwrap();
    // no selection: every molecule, each with its choice, else every line the numbers of which are there (no engine: the record's)
    let methane =
        "CH4\nlogP: 0.64\ntPSA: 0.00\nHBD: 0\nHBA: 0\nRotatable bonds: 0\nHeavy atoms: 1\n";
    assert_eq!(
        e.copy_values("text").unwrap(),
        format!("C2H6O\nHBD: 1\nHBA: 1\n\n{methane}")
    );
    assert_eq!(
        e.copy_values("line").unwrap(),
        "C2H6O; HBD 1; HBA 1\nCH4; logP 0.64; tPSA 0.00; HBD 0; HBA 0; Rotatable bonds 0; Heavy atoms 1\n"
    );
    assert_eq!(
        e.copy_values("tsv").unwrap(),
        "structure\tname\tvalue\nC2H6O\tHBD\t1\nC2H6O\tHBA\t1\nCH4\tlogP\t0.64\nCH4\ttPSA\t0.00\nCH4\tHBD\t0\nCH4\tHBA\t0\nCH4\tRotatable bonds\t0\nCH4\tHeavy atoms\t1\n"
    );
    assert_eq!(e.copy_values("html"), Err("error.draw2.propsCopy"));
    // a molecule with only the formula chosen is its formula and nothing else
    e.session.run(&[cap(1, &["formula"])]).unwrap();
    assert!(e.copy_values("tsv").unwrap().contains("C2H6O\t\t\n"));
}

#[test]
fn the_request_field_gives_the_json_and_the_text_and_stands_alone() {
    let out = replay(r#"{"import":"CCO","log":[{"op":"atom.cap","id":1,"lines":["formula","hbd"]}],"props":true}"#).unwrap();
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["format"], "props/1");
    assert_eq!(v["engine"], false);
    assert_eq!(v["keys"].as_array().unwrap().len(), 11);
    let s = &v["structures"][0];
    assert_eq!(s["cap"], json!(["formula", "hbd"]));
    assert_eq!(s["values"]["hbd"], json!([["HBD", "1"]]));
    assert_eq!(
        s["values"]["mw"],
        json!([]),
        "no engine: no number, never a made-up one"
    );
    assert_eq!(s["values"]["logp"], json!([["logP", "0.00"]]));
    // the selection, as one structure with its cut bonds closed with hydrogen (the methyl of ethanol is CH4)
    let out = replay(r#"{"import":"CCO","props":{"atoms":[1]}}"#).unwrap();
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["selection"]["formula"], "CH4");
    assert_eq!(v["selection"]["of"], json!([1]));
    // the text forms
    let t = replay(r#"{"import":"CCO","props":{"text":"line","lines":["hbd","heavy"]}}"#).unwrap();
    assert_eq!(t, "C2H6O; HBD 1; Heavy atoms 3\n");
    let t = replay(r#"{"import":"CCO","props":{"text":"text","atoms":[1],"lines":["heavy"]}}"#)
        .unwrap();
    assert_eq!(t, "CH4\nHeavy atoms: 1\n");
    // a selection of a whole molecule that chose its lines copies those lines; a part of it, or molecules that chose differently, every line
    let t = replay(r#"{"import":"CCO","log":[{"op":"atom.cap","id":1,"lines":["formula","hbd"]}],"props":{"text":"line","atoms":[1,2,3]}}"#).unwrap();
    assert_eq!(t, "C2H6O; HBD 1\n");
    let t = replay(r#"{"import":"CCO.CC","log":[{"op":"atom.cap","id":1,"lines":["hbd"]},{"op":"atom.cap","id":4,"lines":["hba"]}],"props":{"text":"line","atoms":[1,4]}}"#).unwrap();
    assert!(
        t.starts_with("C2H8; ") && t.contains("Heavy atoms 2"),
        "{t}"
    );
    // refusals: a form that is none, a key that is not allowed, an edit field beside it, both fields
    for bad in [
        r#"{"import":"CCO","props":{"text":"html"}}"#,
        r#"{"import":"CCO","props":{"x":1}}"#,
        r#"{"import":"CCO","props":{"atoms":["a"]}}"#,
        r#"{"import":"CCO","props":true,"out":"svg"}"#,
        r#"{"import":"CCO","props":true,"values":true}"#,
    ] {
        assert!(replay(bad).is_err(), "{bad}");
    }
    assert_eq!(
        replay(r#"{"props":{"text":"text"}}"#).unwrap_err().key,
        "error.draw2.propsCopy",
        "nothing to copy"
    );
}

#[test]
fn svg_and_cdxml_carry_the_lines_under_the_molecule() {
    let mut e = editor("CCO");
    let plain = e.save("svg").unwrap();
    let cdxml_plain = e.save("cdxml").unwrap();
    e.session.run(&[cap(1, &["formula", "hba"])]).unwrap();
    let svg = e.save("svg").unwrap();
    assert!(svg.contains(">C2H6O<") && svg.contains(">HBA 1<"), "{svg}");
    assert!(!plain.contains("HBA 1"));
    // the page grows to hold the lines
    let height = |s: &str| {
        let h = s
            .split("height=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        h.trim_end_matches("pt").parse::<f64>().unwrap()
    };
    assert!(height(&svg) > height(&plain));
    let cdxml = e.save("cdxml").unwrap();
    assert!(cdxml.contains("C2H6O&#xa;HBA 1"), "{cdxml}");
    assert!(!cdxml_plain.contains("<s font=\"3\" size=\"9\""));
    // a drawing with no choice writes the same CDXML bytes as before
    e.session.undo();
    assert_eq!(e.save("cdxml").unwrap(), cdxml_plain);
}

#[test]
fn a_format_without_a_place_for_the_lines_says_it_and_the_document_and_cdxml_do_not() {
    let mut e = editor("CCO");
    e.session.run(&[cap(1, &["formula"])]).unwrap();
    for kind in [
        "molfile",
        "molfile-v3000",
        "smiles",
        "sdf",
        "ket",
        "cml",
        "trasf",
    ] {
        assert_eq!(
            e.lost_on_save(kind),
            [("draw2.export.captionsLost", 1)],
            "{kind}"
        );
        let said = mzdraw::loss::loss(e.doc(), kind);
        if let Ok(said) = said {
            assert!(said.iter().any(|l| l.what == "caption"), "{kind}");
            assert!(!said.iter().any(|l| l.what == "unknown"), "{kind}");
        }
    }
    for kind in ["mzdraw", "cdxml", "svg", "pdf"] {
        assert!(e.lost_on_save(kind).is_empty(), "{kind}");
    }
    let said = mzdraw::loss::loss(e.doc(), "cdxml").unwrap();
    assert!(
        !said.iter().any(|l| l.what == "caption"),
        "CDXML carries it"
    );
}

#[test]
fn every_text_of_the_lines_exists_in_the_catalog() {
    let lang = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/lang/mzdraw.js"),
    )
    .unwrap();
    let catalog: std::collections::BTreeMap<String, String> = lang
        .lines()
        .filter_map(|l| {
            let l = l.trim().trim_end_matches(',');
            let (k, v) = l.split_once("\": \"")?;
            Some((
                k.strip_prefix('"')?.to_string(),
                v.strip_suffix('"')?.to_string(),
            ))
        })
        .collect();
    let mut want: Vec<String> = [
        "draw2.say.lines",
        "draw2.props.title",
        "draw2.props.none",
        "draw2.props.copy",
        "draw2.props.copied",
        "draw2.props.form.text",
        "draw2.props.form.tsv",
        "draw2.props.form.line",
        "draw2.export.captionsLost",
        "error.draw2.lines",
        "error.draw2.linesNone",
        "error.draw2.linesMixed",
        "error.draw2.props",
        "error.draw2.propsCopy",
    ]
    .map(String::from)
    .to_vec();
    for k in mzdraw::props::LINES {
        want.push(format!("draw2.props.line.{k}"));
        want.push(format!("draw2.props.method.{k}"));
    }
    for k in &want {
        assert!(catalog.get(k).is_some_and(|v| !v.trim().is_empty()), "{k}");
    }
}

/// `n` structures, each a chain of three atoms (a carbon, a nitrogen, an oxygen), far apart, each with a choice of descriptor lines.
fn many(n: usize) -> Document {
    let (mut atoms, mut bonds) = (Vec::new(), Vec::new());
    let mut id = 1usize;
    for k in 0..n {
        for (j, el) in ["C", "N", "O"].iter().enumerate() {
            let mut a = json!({"id": id + j, "el": el, "x": 3.0 * k as f64 + 0.5 * j as f64, "y": (j % 2) as f64 * 0.5});
            if j == 0 {
                a["cap"] = json!(["formula", "logp", "tpsa", "hbd", "hba", "rotb", "heavy"]);
            }
            atoms.push(a);
        }
        id += 3;
    }
    let mut bid = id;
    for k in 0..n {
        for j in 0..2 {
            bonds.push(json!({"id": bid, "a": 3 * k + j + 1, "b": 3 * k + j + 2, "o": 1}));
            bid += 1;
        }
    }
    let v = json!({"format": "mzdraw/1", "next": bid + 1, "atoms": atoms, "bonds": bonds});
    Document::from_json(&v.to_string()).unwrap()
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

/// Four times the size may cost about four times; a pass over all atoms for each molecule costs sixteen times. No floor: the sizes are
/// chosen so that the small run takes milliseconds in a release build too, and the bound 8 leaves a factor two for noise.
fn assert_linear(what: &str, small: u128, large: u128) {
    assert!(
        large <= 8 * small.max(1),
        "{what}: {small} us, four times the size {large} us"
    );
}

/// The code of this slice that runs for every molecule: the captions that write descriptors (the memo is emptied before each run, so
/// the library's descriptors are timed, not the memo), the copy as text and the values of a selection.
fn new_code(d: &Document) {
    mzdraw::props::forget();
    drop(values::captions(d, values::Mode::Off, &[], &[]));
    drop(values::props_text("tsv", d, &[], &[], &[], None));
    let ids: Vec<u32> = d.atoms.iter().map(|a| a.id).collect();
    drop(values::props_json(
        d,
        &[],
        false,
        "",
        &[],
        Some((&ids, None)),
    ));
}

#[test]
fn the_new_code_scales_linearly_in_the_number_of_molecules() {
    let (small, large) = (many(500), many(2000));
    assert_linear(
        "captions, copy and values of 500 molecules",
        best(|| new_code(&small)),
        best(|| new_code(&large)),
    );
}

/// The proof of the test above: the same function with one pass over all the atoms for every molecule (what a careless version of
/// the scan would do) is quadratic and the guard refuses it.
#[test]
#[should_panic(expected = "four times the size")]
fn a_planted_quadratic_version_fails_the_scaling_test() {
    let slow = |d: &Document| {
        new_code(d);
        let n = values::scan(d).structures.len();
        let mut seen = 0usize;
        // 20 passes over all the atoms for each molecule, so that the planted work is what the time is made of
        for _ in 0..20 {
            for k in 0..n {
                seen += d
                    .atoms
                    .iter()
                    .filter(|a| a.id as usize % n.max(1) == k)
                    .count();
            }
        }
        std::hint::black_box(seen);
    };
    let (small, large) = (many(500), many(2000));
    assert_linear("planted", best(|| slow(&small)), best(|| slow(&large)));
}

/// The numbers of the descriptor lines for a few synthetic molecules, pinned. They are the outputs of the engine at the pin of
/// `Cargo.toml` (`mzlab-mol` `Mol::descriptors()`, which says in its own doc comment that its definitions are its own and not another
/// toolkit's), recorded here so that a move of the pin shows as a change; they are NOT values of a reference toolkit and no claim of
/// agreement with one is made.
#[test]
fn the_descriptor_lines_of_a_few_molecules_are_the_pinned_engines_outputs() {
    // (smiles, logP, tPSA, donors, acceptors, rotatable bonds, heavy atoms)
    let rows = [
        ("C", "0.64", "0.00", "0", "0", "0", "1"),
        ("CCO", "0.00", "20.23", "1", "1", "0", "3"),
        (
            "CC(=O)Oc1ccccc1C(=O)O",
            "1.31",
            "63.60",
            "1",
            "4",
            "2",
            "13",
        ),
        (
            "Cn1cnc2c1c(=O)n(C)c(=O)n2C",
            "-1.03",
            "61.82",
            "0",
            "3",
            "0",
            "14",
        ),
    ];
    for (smiles, logp, tpsa, hbd, hba, rotb, heavy) in rows {
        let mut e = editor(smiles);
        e.session
            .run(&[cap(1, &["logp", "tpsa", "hbd", "hba", "rotb", "heavy"])])
            .unwrap();
        let c = values::captions(e.doc(), values::Mode::Off, &[], &[]);
        assert_eq!(
            c[0].lines,
            [
                format!("logP {logp}"),
                format!("tPSA {tpsa}"),
                format!("HBD {hbd}"),
                format!("HBA {hba}"),
                format!("Rotatable bonds {rotb}"),
                format!("Heavy atoms {heavy}")
            ],
            "{smiles}"
        );
    }
}
