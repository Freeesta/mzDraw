//! Atom labels: abbreviations drawn as their name and expanded to their atoms at export, and R-group placeholders written as `R#`
//! atoms of a molfile.

use mzdraw::editor::{ASK_LABEL, CHANGED, HANDLED, MOUSE, TOOL};
use mzdraw::label::{kind, Kind, ABBREVIATIONS};
use mzdraw::{svg, Document, Editor, Op, Session};
use mzlab_mol::Mol;

fn click(e: &mut Editor, x: f64, y: f64) -> u32 {
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_up(x, y)
}

fn px(e: &Editor, id: u32) -> (f64, f64) {
    let a = e.doc().atom(id).unwrap();
    e.view.to_px(a.x, a.y)
}

fn set_label(id: u32, lab: &str) -> Op {
    Op::AtomSet {
        map: None,
        id,
        el: None,
        chg: None,
        iso: None,
        rad: None,
        h: None,
        lab: Some(lab.into()),
    }
}

/// The molecular formula of a record as (element, count) pairs with the hydrogens, sorted.
fn formula(m: &Mol) -> Vec<(String, u32)> {
    let mut f: Vec<(String, u32)> = Vec::new();
    let mut add = |el: &str, n: u32| match f.iter_mut().find(|p| p.0 == el) {
        Some(p) => p.1 += n,
        None => f.push((el.to_string(), n)),
    };
    for a in &m.atoms {
        add(&a.el, 1);
        add("H", a.h as u32);
    }
    f.retain(|p| p.1 > 0);
    f.sort();
    f
}

#[test]
fn every_abbreviation_of_the_table_reads_and_names_are_unique() {
    for (i, (name, smiles)) in ABBREVIATIONS.iter().enumerate() {
        assert!(Mol::from_smiles(smiles).is_ok(), "{name}: {smiles}");
        assert_eq!(kind(name), Some(Kind::Group(smiles)));
        assert!(
            ABBREVIATIONS[..i].iter().all(|p| p.0 != *name),
            "{name} twice"
        );
        // on a methyl: the expansion is exactly the molecule CH3-group, with every valence right
        let mut e = Editor::default();
        e.import("CC").unwrap();
        e.session.run(&[set_label(2, name)]).unwrap();
        let whole = Mol::from_smiles(&format!("C{smiles}")).unwrap();
        let back =
            Mol::from_molfile(&e.doc().to_molfile()).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(formula(&back), formula(&whole), "{name}");
        assert_eq!(back.atoms.len(), whole.atoms.len(), "{name}");
        assert!(
            back.atoms.iter().all(|a| a.rad == 0),
            "{name}: no unpaired electron left"
        );
        let charge = |m: &Mol| m.atoms.iter().map(|a| a.chg as i32).sum::<i32>();
        assert_eq!(charge(&back), charge(&whole), "{name}");
    }
    for (label, n) in [("R", 0), ("R1", 1), ("R32", 32)] {
        assert_eq!(kind(label), Some(Kind::R(n)));
    }
    for bad in ["", "R0", "R01", "R33", "Rx", "ph", "Xyz", "R1 "] {
        assert_eq!(kind(bad), None, "{bad:?}");
    }
}

#[test]
fn an_abbreviation_expands_to_its_atoms_at_export() {
    // toluene drawn as benzene + Me, and as Ph + CH3: the same formula C7H8 either way
    for (smiles, atom, lab) in [
        ("c1ccccc1C", 7, "Me"),
        ("CC", 2, "Ph"),
        ("c1ccccc1C", 7, "Et"),
    ] {
        let mut e = Editor::default();
        e.import(smiles).unwrap();
        e.session.run(&[set_label(atom, lab)]).unwrap();
        let direct = Mol::from_smiles(match lab {
            "Et" => "c1ccccc1CC",
            _ => "c1ccccc1C",
        })
        .unwrap();
        let m = e.doc().to_mol();
        assert_eq!(formula(&m), formula(&direct), "{smiles} with {lab}");
        // the molfile is read back by the record's reader to the same formula: nothing dangling, valences right
        let back = Mol::from_molfile(&e.doc().to_molfile()).unwrap();
        assert_eq!(
            formula(&back),
            formula(&direct),
            "{smiles} with {lab} through a molfile"
        );
        assert_eq!(back.atoms.len(), direct.atoms.len());
    }
    // a group with a charge inside (nitro) on a ring carbon... the labelled atom is the nitrogen
    let mut e = Editor::default();
    e.import("c1ccccc1C").unwrap();
    e.session.run(&[set_label(7, "NO2")]).unwrap();
    let m = Mol::from_molfile(&e.doc().to_molfile()).unwrap();
    assert_eq!(
        formula(&m),
        formula(&Mol::from_smiles("c1ccccc1[N+](=O)[O-]").unwrap())
    );
    // a wedge mark still lands on its own bond line when the expansion adds atoms before the bond block
    let mut e = Editor::default();
    e.import("C[C@H](N)C(=O)O").unwrap();
    let before = e.doc().to_mol_stereo().stereo;
    e.session.run(&[set_label(1, "Ph")]).unwrap();
    let after = Mol::from_molfile(&e.doc().to_molfile()).unwrap();
    assert_eq!(after.atoms.len(), 11);
    assert_eq!(after.stereo.len(), before.len());
}

#[test]
fn an_r_group_is_an_r_atom_of_the_molfile_and_a_query_atom_for_the_check() {
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    e.session
        .run(&[set_label(1, "R2"), set_label(3, "R")])
        .unwrap();
    let mol = e.doc().to_molfile();
    let lines: Vec<&str> = mol.lines().collect();
    assert_eq!(&lines[4][31..34], "R# ");
    assert_eq!(&lines[5][31..34], "C  ");
    assert_eq!(&lines[6][31..34], "R  ");
    assert!(mol.contains("M  RGP  1   1   2\nM  END"), "{mol}");
    let check = e.doc().to_molfile_for_check();
    assert_eq!(&check.lines().nth(4).unwrap()[31..34], "A  ");
    assert!(!check.contains("RGP") && !check.contains("He"));
    // the carbon between two R groups keeps its two hydrogens
    assert_eq!(e.doc().hydrogens(e.doc().atom(2).unwrap()), 2);
    assert_eq!(e.doc().hydrogens(e.doc().atom(1).unwrap()), 0);
}

#[test]
fn labels_are_set_refused_cleared_and_undone() {
    let mut s = Session::default();
    s.run(&[Op::AtomAdd {
        id: None,
        el: "C".into(),
        x: 0.0,
        y: 0.0,
    }])
    .unwrap();
    let plain = s.doc.to_json();
    s.run(&[set_label(1, "Boc")]).unwrap();
    assert_eq!(
        s.doc.to_json(),
        r#"{"format":"mzdraw/1","next":2,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0,"lab":"Boc"}],"bonds":[]}"#
    );
    assert_eq!(Document::from_json(&s.doc.to_json()).unwrap(), s.doc);
    assert_eq!(
        s.log_json().split("},{").last().unwrap(),
        r#""op":"atom.set","id":1,"lab":"Boc"}]"#
    );
    let before = s.doc.to_json();
    assert_eq!(
        s.run(&[set_label(1, "Nope")]).unwrap_err().key,
        "error.draw2.label"
    );
    assert_eq!(s.doc.to_json(), before);
    assert!(Document::from_json(&before.replace("Boc", "Nope")).is_err());
    // the empty label takes it away; so does a new element
    s.run(&[set_label(1, "")]).unwrap();
    assert_eq!(s.doc.to_json(), plain);
    s.undo();
    s.run(&[Op::AtomSet {
        map: None,
        id: 1,
        el: Some("N".into()),
        chg: None,
        iso: None,
        rad: None,
        h: None,
        lab: None,
    }])
    .unwrap();
    assert_eq!(
        (s.doc.atoms[0].el.as_str(), s.doc.atoms[0].lab.as_str()),
        ("N", "")
    );
    s.undo();
    s.undo();
    assert_eq!(s.doc.to_json(), plain);
}

#[test]
fn the_label_tool_labels_an_atom_or_adds_a_labelled_one_and_the_copy_keeps_it() {
    let mut e = Editor::default();
    assert_eq!(e.set_tool("label:Nope"), 0);
    assert_eq!(e.tool.name(), "bond:1");
    assert_eq!(e.key("l", 0), ASK_LABEL | HANDLED);
    assert_eq!(e.set_tool("label:Ph") & TOOL, TOOL);
    assert_eq!(e.tool.name(), "label:Ph");
    assert_eq!(click(&mut e, 300.0, 200.0) & CHANGED, CHANGED);
    assert_eq!(
        (e.doc().atoms.len(), e.doc().atoms[0].lab.as_str()),
        (1, "Ph")
    );
    assert_eq!(e.session.steps(), 1, "one undo step");
    e.set_tool("bond:1");
    click(&mut e, 300.0, 200.0);
    e.set_tool("label:CO2Me");
    let p = px(&e, 2);
    click(&mut e, p.0, p.1);
    assert_eq!(e.doc().atom(2).unwrap().lab, "CO2Me");
    // methyl benzoate, C8H8O2
    let m = Mol::from_molfile(&e.doc().to_molfile()).unwrap();
    assert_eq!(
        formula(&m),
        formula(&Mol::from_smiles("COC(=O)c1ccccc1").unwrap())
    );
    // drawn as text: the name with its digits lowered, no hydrogens
    let svg = svg::to_svg(e.doc());
    for piece in [">Ph<", ">CO<", ">2<", ">Me<"] {
        assert!(svg.contains(piece), "{piece} in {svg}");
    }
    assert!(!svg.contains(">H<"));
    // copy and paste keep the labels
    e.select_all();
    e.duplicate();
    let labs: Vec<&str> = e.doc().atoms.iter().map(|a| a.lab.as_str()).collect();
    assert_eq!(labs, ["Ph", "CO2Me", "Ph", "CO2Me"]);
}
