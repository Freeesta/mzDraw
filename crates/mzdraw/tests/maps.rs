//! Atom-to-atom maps: the field, the operation and its inverse, the readers and writers of the formats that hold maps, the
//! display. Synthetic structures only (esterification, written here).

use mzdraw::io::import;
use mzdraw::{Document, Editor, Op, Session};

/// One V2000 atom line with a map number (columns 61 to 63).
fn atom(x: f64, y: f64, el: &str, map: u16) -> String {
    format!(
        "{x:>10.4}{y:>10.4}{:>10.4} {el:<3} 0  0  0  0  0  0  0  0  0{map:>3}  0  0\n",
        0.0
    )
}

/// One molfile of a reaction file: atoms (element, map) on a line with bonds (a, b, order), from 1.
fn mol(atoms: &[(&str, u16)], bonds: &[(usize, usize, u8)]) -> String {
    let mut t = format!(
        "$MOL\nsynthetic\n  mzdraw\n\n{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
        atoms.len(),
        bonds.len()
    );
    for (i, (el, m)) in atoms.iter().enumerate() {
        t += &atom(i as f64 * 1.5, 0.0, el, *m);
    }
    for (a, b, o) in bonds {
        t += &format!("{a:>3}{b:>3}{o:>3}  0\n");
    }
    t + "M  END\n"
}

/// Acetic acid + methanol -> methyl acetate + water, maps 1..6: C1 C2 O3 O4 (acid), C5 O6 (methanol); the ester keeps 1 2 3 6 5,
/// the water takes 4.
fn esterification() -> String {
    let acid = mol(
        &[("C", 1), ("C", 2), ("O", 3), ("O", 4)],
        &[(1, 2, 1), (2, 3, 2), (2, 4, 1)],
    );
    let methanol = mol(&[("C", 5), ("O", 6)], &[(1, 2, 1)]);
    let ester = mol(
        &[("C", 1), ("C", 2), ("O", 3), ("O", 6), ("C", 5)],
        &[(1, 2, 1), (2, 3, 2), (2, 4, 1), (4, 5, 1)],
    );
    let water = mol(&[("O", 4)], &[]);
    format!("$RXN\n\n  synthetic\n\n  2  2\n{acid}{methanol}{ester}{water}")
}

/// The sorted (element, map) pairs of the mapped atoms of a document.
fn maps(d: &Document) -> Vec<(String, u16)> {
    let mut v: Vec<(String, u16)> = d
        .atoms
        .iter()
        .filter(|a| a.map != 0)
        .map(|a| (a.el.clone(), a.map))
        .collect();
    v.sort();
    v
}

fn expected() -> Vec<(String, u16)> {
    let mut v: Vec<(String, u16)> = [("C", 1), ("C", 2), ("O", 3), ("O", 4), ("C", 5), ("O", 6)]
        .iter()
        .map(|(e, m)| (e.to_string(), *m))
        .collect();
    // the products repeat the maps of the reactants
    v.extend(v.clone().into_iter().filter(|(_, m)| *m != 4));
    v.push(("O".into(), 4));
    v.sort();
    v
}

#[test]
fn a_reaction_file_keeps_its_maps_through_a_round_trip() {
    let (d, _) = import(&esterification()).unwrap();
    assert_eq!(maps(&d), expected());
    let rxn = d.to_rxn().unwrap();
    let (back, _) = import(&rxn).unwrap();
    assert_eq!(maps(&back), expected());
    // the same numbers on the same atoms: the atom lines of the written file carry them
    let (again, _) = import(&back.to_rxn().unwrap()).unwrap();
    assert_eq!(back.to_rxn().unwrap(), again.to_rxn().unwrap());
}

#[test]
fn a_v3000_molfile_carries_the_maps_as_aamap() {
    let (d, _) = import(&esterification()).unwrap();
    let text = d.to_molfile_v3000();
    assert!(
        text.contains(" AAMAP")
            || text
                .lines()
                .any(|l| l.starts_with("M  V30 ") && l.split(' ').nth(8).is_some_and(|t| t != "0"))
    );
    let (back, _) = import(&text).unwrap();
    assert_eq!(maps(&back), maps(&d));
}

#[test]
fn a_v3000_reaction_file_is_read_with_its_maps() {
    let ctab = |atoms: &[(&str, u16)], bonds: &[(usize, usize)]| {
        let mut t = format!(
            "M  V30 BEGIN CTAB\nM  V30 COUNTS {} {} 0 0 0\nM  V30 BEGIN ATOM\n",
            atoms.len(),
            bonds.len()
        );
        for (i, (el, m)) in atoms.iter().enumerate() {
            t += &format!("M  V30 {} {el} {}.0 0 0 {m}\n", i + 1, i);
        }
        t += "M  V30 END ATOM\nM  V30 BEGIN BOND\n";
        for (i, (a, b)) in bonds.iter().enumerate() {
            t += &format!("M  V30 {} 1 {a} {b}\n", i + 1);
        }
        t + "M  V30 END BOND\nM  V30 END CTAB\n"
    };
    let text = format!(
        "$RXN V3000\n\n  synthetic\n\nM  V30 COUNTS 1 1\nM  V30 BEGIN REACTANT\n{}M  V30 END REACTANT\nM  V30 BEGIN PRODUCT\n{}M  V30 END PRODUCT\nM  END\n",
        ctab(&[("C", 1), ("O", 2)], &[(1, 2)]),
        ctab(&[("O", 2), ("C", 1)], &[(1, 2)]),
    );
    let (d, _) = import(&text).unwrap();
    assert_eq!(
        maps(&d),
        vec![
            ("C".into(), 1),
            ("C".into(), 1),
            ("O".into(), 2),
            ("O".into(), 2)
        ]
    );
}

#[test]
fn smiles_atom_classes_are_maps_in_and_out() {
    let (d, _) = import("[CH3:1][OH:2]").unwrap();
    assert_eq!(maps(&d), vec![("C".into(), 1), ("O".into(), 2)]);
    assert_eq!(d.to_smiles().unwrap(), "[CH3:1][OH:2]");
    // an unmapped atom stays bare; an R-group class is not a map
    let (e, _) = import("CC[*:1]").unwrap();
    assert_eq!(e.mapped(), 0);
    let (f, _) = import("C[CH2:12]O").unwrap();
    assert_eq!(f.to_smiles().unwrap(), "C[CH2:12]O");
}

#[test]
fn ket_mapping_is_read_and_written() {
    let (d, _) = import("[CH3:3][OH:4]").unwrap();
    let ket = d.to_ket();
    assert!(ket.contains("\"mapping\":3") && ket.contains("\"mapping\":4"));
    let (back, _) = import(&ket).unwrap();
    assert_eq!(maps(&back), maps(&d));
}

#[test]
fn atom_set_map_has_an_exact_inverse() {
    let (d, _) = import("CO").unwrap();
    let before = d.to_json();
    let mut s = Session::new(d);
    let set = |map| Op::AtomSet {
        id: 1,
        el: None,
        chg: None,
        iso: None,
        rad: None,
        h: None,
        lab: None,
        map: Some(map),
    };
    s.run(&[set(4)]).unwrap();
    assert_eq!(s.doc.atoms[0].map, 4);
    assert!(s.doc.to_json().contains("\"map\":4"));
    s.run(&[set(0)]).unwrap();
    assert_eq!(s.doc.atoms[0].map, 0);
    assert!(!s.doc.to_json().contains("\"map\""));
    assert!(s.undo() && s.undo());
    assert_eq!(s.doc.to_json(), before);
    // out of range is refused and changes nothing
    assert!(s.run(&[set(1000)]).is_err());
    assert_eq!(s.doc.to_json(), before);
    assert!(Op::parse(&serde_json::json!({"op": "atom.set", "id": 1, "map": 4})).is_ok());
}

#[test]
fn a_document_with_maps_round_trips_through_json() {
    let (d, _) = import("[CH3:1][OH:2]").unwrap();
    let back = Document::from_json(&d.to_json()).unwrap();
    assert_eq!(back.to_json(), d.to_json());
    assert_eq!(maps(&back), maps(&d));
}

#[test]
fn the_number_is_drawn_unless_display_maps_is_off() {
    let mut e = Editor::default();
    e.import("[CH3:7][OH:2]").unwrap();
    let on = e.svg();
    assert!(on.contains(">7</text>") && on.contains(">2</text>"));
    e.set("display.maps=off").unwrap();
    let off = e.svg();
    assert!(!off.contains(">7</text>"));
    assert!(e.set("display.maps=maybe").is_err());
    assert!(e.set("display.nope=on").is_err());
}

#[test]
fn a_format_without_a_place_for_maps_lists_them_as_lost() {
    let (d, _) = import("[CH3:1][OH:2]").unwrap();
    assert_eq!(d.maps_lost("rxn"), 0);
    assert_eq!(d.maps_lost("smiles"), 0);
    assert_eq!(d.maps_lost("cdxml"), 2);
    assert_eq!(d.maps_lost("cml"), 2);
}
