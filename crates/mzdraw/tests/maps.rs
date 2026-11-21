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

/// A V2000 molfile of two atoms with the given text in the map field (3 characters, as the format has it).
fn two_atoms(f1: &str, f2: &str) -> String {
    let line = |x: f64, f: &str| {
        format!(
            "{x:>10.4}{:>10.4}{:>10.4} C   0  0  0  0  0  0  0  0  0{f:>3}  0  0\n",
            0.0, 0.0
        )
    };
    format!(
        "m\n  mzdraw\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n{}{}  1  2  1  0\nM  END\n",
        line(0.0, f1),
        line(1.5, f2)
    )
}

#[test]
fn a_field_that_is_no_map_is_counted_not_dropped_in_silence() {
    // 0 and blank are none, 1 and 999 are maps; negative, non-numeric and 1.5 are counted as left out
    for (a, b, mapped, left) in [
        ("  0", "   ", 0, 0),
        ("  1", "999", 2, 0),
        (" -5", "  7", 1, 1),
        ("abc", "  7", 1, 1),
        ("1.5", " -1", 0, 2),
    ] {
        let (d, info) = import(&two_atoms(a, b)).unwrap();
        assert_eq!((d.mapped(), info.maps_left), (mapped, left), "{a:?} {b:?}");
        assert_eq!(
            info.notes().iter().any(|n| n.0 == "draw2.import.mapsLeft"),
            left > 0
        );
    }
    // V3000: AAMAP above the limit, huge, negative and not a number
    let v3 = |aamap: &str| {
        format!("m\n  mzdraw\n\n  0  0  0     0  0            999 V3000\nM  V30 BEGIN CTAB\nM  V30 COUNTS 1 0 0 0 0\nM  V30 BEGIN ATOM\nM  V30 1 C 0 0 0 {aamap}\nM  V30 END ATOM\nM  V30 END CTAB\nM  END\n")
    };
    for (aamap, mapped, left) in [
        ("0", 0, 0),
        ("999", 1, 0),
        ("1000", 0, 1),
        ("99999999999", 0, 1),
        ("-3", 0, 1),
        ("x", 0, 1),
    ] {
        let (d, info) = import(&v3(aamap)).unwrap_or_else(|e| panic!("AAMAP {aamap}: {e}"));
        assert_eq!(
            (d.mapped(), info.maps_left),
            (mapped, left),
            "AAMAP {aamap}"
        );
    }
    // SMILES: a class that is no map is counted
    for (smiles, mapped, left) in [
        ("[CH3:0]C", 0, 0),
        ("[CH3:999]C", 1, 0),
        ("[CH3:1000]C", 0, 1),
    ] {
        let (d, info) = import(smiles).unwrap();
        assert_eq!((d.mapped(), info.maps_left), (mapped, left), "{smiles}");
    }
    // a class the SMILES reader of mzlab-mol refuses (negative, not a number, a fraction) is its keyed error, never a map
    for smiles in ["[CH3:-1]C", "[CH3:x]C", "[CH3:1.5]C"] {
        assert_eq!(
            import(smiles).unwrap_err(),
            "error.draw2.smiles",
            "{smiles}"
        );
    }
    // an empty class is none
    let (d, info) = import("[CH3:]C").unwrap();
    assert_eq!((d.mapped(), info.maps_left), (0, 0));
    // KET: mapping 0 is none; negative, above the limit, a string and a fraction are counted
    let ket = |mapping: &str| {
        format!(
            r#"{{"root":{{"nodes":[{{"$ref":"mol0"}}]}},"mol0":{{"type":"molecule","atoms":[{{"label":"C","location":[0,0,0],"mapping":{mapping}}}],"bonds":[]}}}}"#
        )
    };
    for (mapping, mapped, left) in [
        ("0", 0, 0),
        ("5", 1, 0),
        ("1000", 0, 1),
        ("-2", 0, 1),
        ("\"a\"", 0, 1),
        ("1.5", 0, 1),
    ] {
        let (d, info) = import(&ket(mapping)).unwrap();
        assert_eq!(
            (d.mapped(), info.maps_left),
            (mapped, left),
            "mapping {mapping}"
        );
    }
}

#[test]
fn one_number_on_two_atoms_of_a_structure_is_reported() {
    let (d, info) = import("[CH3:1][CH2:1]O").unwrap();
    assert_eq!((d.duplicate_maps(), info.maps_duplicate), (1, 1));
    assert!(info
        .notes()
        .iter()
        .any(|n| n.0 == "draw2.import.mapsDuplicate"));
    // the same number in the reactant and in the product is the point of a map: no report
    let (_, info) = import(&esterification()).unwrap();
    assert_eq!(info.maps_duplicate, 0);
}

#[test]
fn the_sdf_and_the_other_formats_keep_the_maps_and_cdxml_and_cml_lose_them_aloud() {
    let (d, _) = import("[CH3:1][OH:2]").unwrap();
    let sdf = d.to_sdf();
    let (back, _) = import(&sdf).unwrap();
    assert_eq!(maps(&back), maps(&d));
    for kind in [
        "mzdraw",
        "svg",
        "molfile",
        "molfile-v3000",
        "smiles",
        "ket",
        "sdf",
    ] {
        assert_eq!(d.maps_lost(kind), 0, "{kind}");
    }
    let mut e = Editor::default();
    e.import("[CH3:1][OH:2]").unwrap();
    assert_eq!(e.lost_on_save("cdxml"), vec![("draw2.export.mapsLost", 2)]);
    assert_eq!(e.lost_on_save("cml"), vec![("draw2.export.mapsLost", 2)]);
    assert!(e.lost_on_save("molfile").is_empty());
    // the replay report carries it
    let r = mzdraw::replay::replay(r#"{"import":"[CH3:1][OH:2]","out":"cdxml","report":true}"#)
        .unwrap();
    assert!(
        r.contains(r#""lost":[{"key":"draw2.export.mapsLost","n":2}]"#),
        "{r}"
    );
    let r = mzdraw::replay::replay(r#"{"import":"[CH3:1][OH:2]","out":"molfile","report":true}"#)
        .unwrap();
    assert!(!r.contains("lost"));
}

#[test]
fn atom_add_takes_a_map_and_the_writers_never_skip_a_number() {
    let mut s = Session::new(Document::default());
    let add = |map| Op::AtomAdd {
        id: None,
        el: "C".into(),
        x: 0.0,
        y: 0.0,
        map: Some(map),
    };
    s.run(&[add(12)]).unwrap();
    assert_eq!(s.doc.atoms[0].map, 12);
    assert!(s.run(&[add(1000)]).is_err());
    assert!(s.undo());
    assert!(s.doc.atoms.is_empty());
    // the numbers are written in the V3000 atom line even for a bare atom, and in the V2000 field
    let (d, _) = import("[CH3:5]").unwrap();
    assert!(d.to_molfile().contains("  0  5  0  0"));
    assert!(d
        .to_molfile_v3000()
        .lines()
        .any(|l| l.starts_with("M  V30 1 C ") && l.contains(" 5")));
}

/// The same hash as `fnv1a` of `tools/fidelity/maps.py`: the product name and slug are masked (see `tests/fidelity.rs`).
fn fnv1a(text: &str) -> String {
    let text = text
        .replace(mzdraw::brand::PAGES_URL, "\u{3}")
        .replace(mzdraw::brand::NAME, "\u{1}")
        .replace(mzdraw::brand::SLUG, "\u{2}");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn data(name: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/fidelity")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The reference of `tools/fidelity/maps.py` (RDKit reading our reaction files): needs no Python or RDKit here. Our writer still gives
/// the very bytes RDKit read (hash), RDKit read the maps of the input, and they are the maps our reader reads from the same file.
#[test]
fn rdkit_reads_the_maps_we_write_as_the_reference_says() {
    let reference: serde_json::Value = serde_json::from_str(&data("maps.json")).unwrap();
    assert!(reference["programs"]["rdkit"]
        .as_str()
        .is_some_and(|v| !v.is_empty()));
    let set: Vec<String> = data("maps-set.txt")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect();
    let results = reference["results"].as_array().unwrap();
    assert_eq!(results.len(), set.len());
    assert_eq!(set.len(), 20);
    let replay = |req: serde_json::Value| mzdraw::replay::replay(&req.to_string()).unwrap();
    for (reaction, r) in set.iter().zip(results) {
        assert_eq!(r["reaction"].as_str().unwrap(), reaction);
        assert_eq!(r["same_as_input"], true, "{reaction}");
        let (left, right) = reaction.split_once(">>").unwrap();
        let sides: Vec<Vec<&str>> = [left, right]
            .iter()
            .map(|s| s.split('.').collect())
            .collect();
        let mols: Vec<Vec<String>> = sides
            .iter()
            .map(|side| {
                side.iter()
                    .map(|s| replay(serde_json::json!({"import": s, "out": "molfile"})))
                    .collect()
            })
            .collect();
        let block = format!(
            "$RXN\n\n  synthetic\n\n{:>3}{:>3}\n{}",
            mols[0].len(),
            mols[1].len(),
            mols.iter()
                .flatten()
                .map(|m| format!("$MOL\n{m}"))
                .collect::<String>()
        );
        let ours = replay(serde_json::json!({"import": block, "out": "rxn"}));
        assert_eq!(
            fnv1a(&ours),
            r["hash"].as_str().unwrap(),
            "{reaction}: our file changed: rerun tools/fidelity/maps.py"
        );
        // the maps RDKit read, flattened, are the maps our reader reads from the same file
        let mut theirs: Vec<u64> = r["rdkit_rxn"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|s| s.as_array().unwrap())
            .flat_map(|m| m.as_array().unwrap())
            .map(|n| n.as_u64().unwrap())
            .collect();
        let (d, _) = import(&ours).unwrap();
        let mut mine: Vec<u64> = d
            .atoms
            .iter()
            .filter(|a| a.map != 0)
            .map(|a| u64::from(a.map))
            .collect();
        theirs.sort_unstable();
        mine.sort_unstable();
        assert_eq!(theirs, mine, "{reaction}");
    }
}

#[test]
fn duplicates_are_reported_in_molfile_rxn_and_ket_too() {
    // molfile: one number on both atoms of a structure
    let (d, info) = import(&two_atoms("  3", "  3")).unwrap();
    assert_eq!((d.duplicate_maps(), info.maps_duplicate), (1, 1));
    // two different numbers: none
    let (_, info) = import(&two_atoms("  3", "  4")).unwrap();
    assert_eq!(info.maps_duplicate, 0);
    // RXN: the reactant carries 1 twice, the product 1 once: one structure is reported
    let twice = mol(&[("C", 1), ("O", 1)], &[(1, 2, 1)]);
    let once = mol(&[("C", 1), ("O", 2)], &[(1, 2, 1)]);
    let (d, info) = import(&format!("$RXN\n\n  synthetic\n\n  1  1\n{twice}{once}")).unwrap();
    assert_eq!((d.duplicate_maps(), info.maps_duplicate), (1, 1));
    // KET: two bonded atoms with the same mapping
    let ket = r#"{"root":{"nodes":[{"$ref":"mol0"}]},"mol0":{"type":"molecule","atoms":[{"label":"C","location":[0,0,0],"mapping":4},{"label":"O","location":[1,0,0],"mapping":4}],"bonds":[{"type":1,"atoms":[0,1]}]}}"#;
    let (d, info) = import(ket).unwrap();
    assert_eq!((d.duplicate_maps(), info.maps_duplicate), (1, 1));
}

#[test]
fn map_zero_means_none_on_read_and_is_not_counted() {
    for text in [two_atoms("  0", "  0"), "[CH3:0]C".to_string()] {
        let (d, info) = import(&text).unwrap();
        assert_eq!((d.mapped(), info.maps_left, info.maps_duplicate), (0, 0, 0));
    }
}

#[test]
fn the_field_text_must_be_a_whole_number() {
    let mut e = Editor::default();
    e.import("CO").unwrap();
    e.key("home", 0);
    for bad in ["", " ", "1.5", "-1", "+3", "x", "1000", "99999999999"] {
        e.set_map_text(bad);
        assert_eq!(e.error, "error.draw2.map", "{bad:?}");
        assert_eq!(e.doc().mapped(), 0, "{bad:?}");
        e.error = "";
    }
    e.set_map_text(" 12 ");
    assert_eq!(e.doc().mapped(), 1);
    e.set_map_text("0");
    assert_eq!(e.doc().mapped(), 0);
}
