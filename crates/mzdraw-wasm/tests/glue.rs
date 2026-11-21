//! The glue between the drawing and the mzLab engines: the atom numbers of an answer map back to the right document ids (also when
//! deletions leave ids that are not 0..n), every engine answers exactly what the mzLab function gives on the same molfile, and an engine
//! this build does not carry is refused with `error.draw2.engineMissing`.

use mzdraw::Document;
use mzdraw_wasm::glue;

/// A synthetic drawing whose ids are not 0..n: a ring, three of its atoms deleted, then a hetero atom bonded to one that is left.
fn drawing() -> Document {
    let request = r#"{"log":[
        {"op":"ring","n":6,"arom":false,"x":0,"y":0},
        {"op":"remove","atoms":[2,3,5]},
        {"op":"atom.add","el":"O","x":3,"y":0},
        {"op":"atom.add","el":"S","x":4,"y":1},
        {"op":"atom.add","el":"Cl","x":5,"y":2},
        {"op":"bond.add","a":13,"b":14},
        {"op":"bond.add","a":14,"b":15}
    ],"out":"mzdraw"}"#;
    Document::from_json(&mzdraw::replay::replay(request).unwrap()).unwrap()
}

/// The element of each atom line of a V2000 molfile, in file order.
fn molfile_elements(molfile: &str) -> Vec<String> {
    let lines: Vec<&str> = molfile.lines().collect();
    let n: usize = lines[3][..3].trim().parse().unwrap();
    (lines[4..4 + n])
        .iter()
        .map(|l| l.split_whitespace().nth(3).unwrap().to_string())
        .collect()
}

#[test]
fn ids_are_not_line_numbers_after_deletions() {
    let doc = drawing();
    let ids = glue::ids(&doc);
    assert_eq!(ids.len(), doc.atoms.len());
    // the deleted ids leave holes: reading an engine atom number as an id would paint the wrong atom
    assert!(
        ids.iter().enumerate().any(|(i, &id)| id as usize != i),
        "{ids:?}"
    );
    assert!(!ids.contains(&2) && !ids.contains(&3) && !ids.contains(&5));
    // engine atom i is the i-th atom line of the molfile: its element is the element of document atom ids[i]
    let els = molfile_elements(&glue::molfile(&doc));
    for (i, id) in ids.iter().enumerate() {
        let atom = doc.atoms.iter().find(|a| a.id == *id).unwrap();
        assert_eq!(els[i], atom.el, "engine atom {i} is document atom {id}");
    }
    let by_el = |el: &str| ids[els.iter().position(|e| e == el).unwrap()];
    for el in ["O", "S", "Cl"] {
        assert_eq!(doc.atoms.iter().find(|a| a.el == el).unwrap().id, by_el(el));
    }
}

#[test]
fn an_engine_atom_of_an_answer_is_painted_on_the_right_document_atom() {
    let doc = drawing();
    let ids = glue::ids(&doc);
    // cut the bond O-S (engine atoms 3 and 4): the answer lists the two atoms it changes by engine number
    let answer = glue::run("cut", &doc, r#"{"bonds":[[3,4]]}"#).unwrap();
    let v: serde_json::Value = serde_json::from_str(&answer).unwrap();
    let painted: Vec<(u64, u32)> = v["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            let i = a["i"].as_u64().unwrap();
            (i, ids[i as usize])
        })
        .collect();
    assert_eq!(painted.len(), 2);
    let el = |id: u32| doc.atoms.iter().find(|a| a.id == id).unwrap().el.as_str();
    let mut els: Vec<&str> = painted.iter().map(|&(_, id)| el(id)).collect();
    els.sort();
    assert_eq!(els, ["O", "S"]);
    // read as ids directly (the wrong way) the numbers 3 and 4 name atoms that do not exist
    assert!(painted.iter().all(|&(i, id)| i as u32 != id));
}

#[test]
fn every_engine_answers_what_the_mzlab_function_gives_on_the_same_molfile() {
    let doc = drawing();
    let mf = serde_json::Value::from(glue::molfile(&doc));
    let cases: Vec<(&str, &str, String)> = vec![
        ("check", "{}", format!(r#"{{"molfile":{mf}}}"#)),
        ("cip", "{}", format!(r#"{{"molfile":{mf}}}"#)),
        (
            "cut",
            r#"{"bonds":[[3,4]]}"#,
            format!(r#"{{"bonds":[[3,4]],"molfile":{mf}}}"#),
        ),
        (
            "mech",
            r#"{"arrows":[]}"#,
            format!(r#"{{"arrows":[],"molfile":{mf}}}"#),
        ),
        (
            "hilite",
            r#"{"layers":[]}"#,
            format!(r#"{{"layers":[],"molfile":{mf}}}"#),
        ),
        (
            "sel",
            r#"{"part":false}"#,
            format!(r#"{{"part":false,"molfile":{mf}}}"#),
        ),
    ];
    use mzlab_feat::chem;
    for (engine, args, direct) in cases {
        let want = match engine {
            "check" => chem::check::check_json(&direct),
            "cip" => chem::cip::cip_json(&direct),
            "cut" => chem::cut::cut_json(&direct),
            "mech" => chem::mech::mech_json(&direct),
            "hilite" => chem::hilite::hilite_json(&direct),
            _ => chem::sel::sel_json(&direct),
        }
        .unwrap_or_else(|e| panic!("{engine}: {}", e.key));
        assert_eq!(glue::run(engine, &doc, args).unwrap(), want, "{engine}");
    }
    assert_eq!(
        glue::check(&doc).unwrap(),
        glue::run("check", &doc, "").unwrap()
    );
    assert_eq!(glue::ENGINES.len(), 6);
}

#[test]
fn an_unknown_engine_is_missing_and_bad_arguments_are_a_request_error() {
    let doc = drawing();
    assert_eq!(
        glue::run("nope", &doc, "").unwrap_err().key,
        "error.draw2.engineMissing"
    );
    assert_eq!(
        glue::run("check", &doc, "[1]").unwrap_err().key,
        "error.draw2.request"
    );
    assert_eq!(
        glue::run("check", &doc, "{").unwrap_err().key,
        "error.draw2.request"
    );
    assert!(glue::check(&Document::default()).is_none());
}

#[test]
fn replay_gives_the_answers_in_request_order() {
    let request = r#"{"log":[{"op":"ring","n":6,"arom":true,"x":0,"y":0}],"engines":[{"engine":"sel"},{"engine":"check"}]}"#;
    let out: serde_json::Value = serde_json::from_str(&glue::replay(request).unwrap()).unwrap();
    let e = out["engines"].as_array().unwrap();
    assert_eq!(e[0]["engine"], "sel");
    assert_eq!(e[0]["answer"]["schema"], "selection/1");
    assert_eq!(e[1]["answer"]["schema"], "check/1");
    let missing = r#"{"engines":[{"engine":"nope"}]}"#;
    assert_eq!(
        glue::replay(missing).unwrap_err().key,
        "error.draw2.engineMissing"
    );
    assert!(glue::replay("{").is_err());
}

/// The monoisotopic mass of a structure as `sel` (the mass of `label/1`) answers it.
fn mono(smiles: &str) -> f64 {
    let request = serde_json::json!({"import": smiles, "engines": [{"engine": "sel"}]}).to_string();
    let out: serde_json::Value = serde_json::from_str(&glue::replay(&request).unwrap()).unwrap();
    let label = &out["engines"][0]["answer"]["label"];
    label["mass"].as_f64().unwrap_or_else(|| panic!("{label}"))
}

/// tp-07: the mass shifts that the mass labels will write are the differences of the masses the glue returns, not typed numbers.
/// `pathway:label` writes the formula delta only for now (the mass modes are refused until arch-web-17); this pins that the numbers of
/// the study (`+O` 15.9949, `-CH2` -14.0157, `+C6H8O6` 176.0321, `+SO3` 79.9568, `-H2` -2.0157, `+H2O` 18.0106) hold on the engine.
#[test]
fn the_mass_shifts_of_the_deltas_come_from_the_masses_the_glue_returns() {
    for (from, to, shift) in [
        ("CC", "CCO", 15.9949),
        ("CCC", "CC", -14.0157),
        ("CC", "C=C", -2.0157),
        ("C=C", "CCO", 18.0106),
        ("CO", "COS(=O)(=O)O", 79.9568),
    ] {
        let d = mono(to) - mono(from);
        assert!((d - shift).abs() < 1e-4, "{from} -> {to}: {d} vs {shift}");
    }
    // +C6H8O6: the mass of the group itself (ascorbic acid has that formula)
    assert!((mono("OCC(O)C1OC(=O)C(O)=C1O") - 176.0321).abs() < 1e-4);
}
