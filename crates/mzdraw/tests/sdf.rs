//! SDF out and the data of a structure (the data fields of an SDF record): read, kept through edits and undo, written.
//! Synthetic structures and data only.

use mzdraw::doc::{data_valid, MAX_DATA};
use mzdraw::io::import;
use mzdraw::{Document, Editor, Op, Session};
use mzlab_mol::Mol;

fn drawn(text: &str) -> Document {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e.doc().clone()
}

fn canonical(mol: &str) -> String {
    Mol::from_molfile(mol)
        .unwrap()
        .canonical()
        .to_smiles()
        .unwrap()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// An SDF as other programs write it: records with data headers of several shapes, a multi-line value, an empty value.
fn sdf() -> String {
    let a = drawn("CCO").to_molfile();
    let b = drawn("c1ccccc1").to_molfile();
    let c = drawn("N[C@@H](C)C(=O)O").to_molfile();
    format!(
        "{a}\n>  <name>\nethanol (synthetic)\n\n> <note> (1)\nfirst line\nsecond line\n\n>  <empty>\n\n$$$$\n{b}\n$$$$\n{c}\n> 25 <MELTING.POINT>\n297 & more <units?>\n\n>  <bad<name>\nx\n\n$$$$\n"
    )
}

#[test]
fn the_data_fields_of_an_sdf_are_read_onto_their_structures() {
    let (d, info) = import(&sdf()).unwrap();
    assert_eq!((info.records, info.data_left), (3, 1));
    assert_eq!(
        info.notes(),
        [("draw2.import.records", 3), ("draw2.import.dataLeft", 1)]
    );
    // the data of a record is kept on its first atom
    assert_eq!(
        d.atoms[0].data(),
        pairs(&[
            ("name", "ethanol (synthetic)"),
            ("note", "first line\nsecond line"),
            ("empty", "")
        ])
    );
    assert!(
        d.atoms[1].data().is_empty() && d.atoms[3].data().is_empty(),
        "no data on the benzene record"
    );
    assert_eq!(
        d.atoms[9].data(),
        pairs(&[("MELTING.POINT", "297 & more <units?>")])
    );
    // it is in the document, written and read back to the same bytes
    let json = d.to_json();
    assert!(json.contains(r#""data":[["name","ethanol (synthetic)"],["note","first line\nsecond line"],["empty",""]]"#), "{json}");
    assert_eq!(Document::from_json(&json).unwrap().to_json(), json);
    // through the editor (one undo step) the data comes along; the outline of the drawing is unchanged by it
    let mut e = Editor::default();
    e.import(&sdf()).unwrap();
    assert_eq!(e.doc().atoms[0].data().len(), 3);
    assert_eq!(e.doc().atoms[9].data().len(), 1);
    e.undo();
    assert!(e.doc().is_empty());
}

#[test]
fn an_sdf_is_written_one_record_per_structure_with_its_data() {
    let (d, _) = import(&sdf()).unwrap();
    let out = d.to_sdf();
    let records: Vec<&str> = out.split("$$$$\n").collect();
    assert_eq!(
        records.len(),
        4,
        "three records and nothing after the last end"
    );
    assert_eq!(records[3], "");
    let want = ["CCO", "c1ccccc1", "N[C@@H](C)C(=O)O"];
    for (r, s) in records.iter().zip(want) {
        assert_eq!(
            canonical(r),
            Mol::from_smiles(s)
                .unwrap()
                .canonical()
                .to_smiles()
                .unwrap(),
            "{s}"
        );
        assert!(r.contains("M  END\n"));
    }
    // data: after the molfile, each field a header, its value and an empty line
    assert!(records[0].ends_with("M  END\n>  <name>\nethanol (synthetic)\n\n>  <note>\nfirst line\nsecond line\n\n>  <empty>\n\n\n"), "{:?}", records[0]);
    assert!(records[1].ends_with("M  END\n"));
    assert!(records[2].ends_with("M  END\n>  <MELTING.POINT>\n297 & more <units?>\n\n"));
    // the same drawing gives the same bytes; read again, the same structures and the same data, and then the same file
    assert_eq!(d.to_sdf(), out);
    let (back, info) = import(&out).unwrap();
    assert_eq!(info.data_left, 0);
    assert_eq!(back.atoms[0].data(), d.atoms[0].data());
    assert_eq!(back.atoms[9].data(), d.atoms[9].data());
    let again = back.to_sdf();
    let data_of = |t: &str| -> Vec<String> {
        t.split("$$$$\n")
            .map(|r| r.split("M  END\n").nth(1).unwrap_or("").to_string())
            .collect()
    };
    assert_eq!(data_of(&again), data_of(&out));
    // two structures joined by a bond are one record with the data of both, in the order of their atoms
    let mut s = Session::default();
    s.doc = d.clone();
    let (first, last) = (d.atoms[0].id, d.atoms[9].id);
    s.run(&[Op::BondAdd {
        id: None,
        a: first,
        b: last,
        o: 1,
        s: 0,
    }])
    .unwrap();
    let joined = s.doc.to_sdf();
    assert_eq!(joined.matches("$$$$\n").count(), 2);
    let first = joined.split("$$$$\n").next().unwrap();
    assert!(first.find(">  <name>").unwrap() < first.find(">  <MELTING.POINT>").unwrap());
    // arrows and texts are not in an SDF; nothing drawn gives an empty file; the editor and the replay door write it
    assert_eq!(Document::default().to_sdf(), "");
    let mut e = Editor::default();
    e.import(&sdf()).unwrap();
    assert_eq!(e.save("sdf").unwrap(), e.doc().to_sdf());
    let request = serde_json::json!({ "import": sdf(), "out": "sdf" }).to_string();
    assert_eq!(mzdraw::replay::replay(&request).unwrap(), e.doc().to_sdf());
}

#[test]
fn data_is_set_by_an_operation_with_an_exact_inverse() {
    let mut s = Session::default();
    s.run(&[
        Op::AtomAdd {
            map: None,
            id: None,
            el: "C".into(),
            x: 0.0,
            y: 0.0,
        },
        Op::AtomAdd {
            map: None,
            id: None,
            el: "O".into(),
            x: 1.0,
            y: 0.0,
        },
        Op::BondAdd {
            id: None,
            a: 1,
            b: 2,
            o: 1,
            s: 0,
        },
    ])
    .unwrap();
    let plain = s.doc.to_json();
    let set = Op::AtomData {
        id: 1,
        data: pairs(&[("name", "methanol"), ("note", "a\nb")]),
    };
    s.run(std::slice::from_ref(&set)).unwrap();
    assert_eq!(
        s.doc.atoms[0].data(),
        pairs(&[("name", "methanol"), ("note", "a\nb")])
    );
    let with = s.doc.to_json();
    // the operation is in the log as JSON and reads back as itself
    assert_eq!(
        set.to_json(),
        r#"{"op":"atom.data","id":1,"data":[["name","methanol"],["note","a\nb"]]}"#
    );
    assert_eq!(serde_json::from_str::<Op>(&set.to_json()).unwrap(), set);
    // replaced, then taken away; each step undone to the exact bytes before it
    s.run(&[Op::AtomData {
        id: 1,
        data: pairs(&[("name", "MeOH")]),
    }])
    .unwrap();
    s.run(&[Op::AtomData {
        id: 1,
        data: vec![],
    }])
    .unwrap();
    assert_eq!(s.doc.to_json(), plain, "no data, no field");
    assert!(s.undo());
    assert!(s.undo());
    assert_eq!(s.doc.to_json(), with);
    // the data follows its atom: moved, changed, erased with it and back with undo
    s.run(&[Op::Move {
        atoms: vec![1],
        objects: vec![],
        dx: 1.0,
        dy: 1.0,
    }])
    .unwrap();
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
    assert_eq!(s.doc.atoms[0].data().len(), 2);
    s.run(&[Op::Remove {
        atoms: vec![1],
        bonds: vec![],
        objects: vec![],
    }])
    .unwrap();
    assert!(s.doc.atoms.iter().all(|a| a.data().is_empty()));
    assert!(s.undo());
    assert_eq!(s.doc.atoms[0].data().len(), 2);
    for _ in 0..3 {
        assert!(s.undo());
    }
    assert_eq!(s.doc.to_json(), plain);
    // what an SDF could not hold is refused and changes nothing
    let before = s.doc.to_json();
    let long = "x".repeat(MAX_DATA.2 + 1);
    for bad in [
        pairs(&[("", "v")]),
        pairs(&[("a<b", "v")]),
        pairs(&[("a>b", "v")]),
        pairs(&[(" padded", "v")]),
        pairs(&[("two\nlines", "v")]),
        pairs(&[("n", "an\n\nempty line")]),
        pairs(&[("n", "line\n$$$$")]),
        pairs(&[("n", "line\n> <header>")]),
        pairs(&[("n", "tab\u{1}control")]),
        pairs(&[("n", long.as_str())]),
        pairs(&[(&"n".repeat(MAX_DATA.1 + 1), "v")]),
        vec![("n".to_string(), "v".to_string()); MAX_DATA.0 + 1],
    ] {
        assert!(!data_valid(&bad));
        let e = s.run(&[Op::AtomData { id: 1, data: bad }]).unwrap_err();
        assert_eq!(e.key, "error.draw2.data");
        assert_eq!(s.doc.to_json(), before);
    }
    assert_eq!(
        s.run(&[Op::AtomData {
            id: 99,
            data: vec![]
        }])
        .unwrap_err()
        .key,
        "error.draw2.noAtom"
    );
    // a field `data` of another shape in a document is not data: it is kept as it is and written back
    let odd = r#"{"format":"mzdraw/1","next":2,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0,"data":{"later":1}}],"bonds":[]}"#;
    let doc = Document::from_json(odd).unwrap();
    assert!(doc.atoms[0].data().is_empty());
    assert!(
        doc.to_json().contains(r#""data":{"later":1}"#),
        "{}",
        doc.to_json()
    );
    assert_eq!(Document::from_json(&doc.to_json()).unwrap(), doc);
}

#[test]
fn a_damaged_sdf_with_data_is_refused_or_read_but_never_a_panic() {
    let good = sdf();
    let mut x = 0x2545_F491_4F6C_DD1Du64;
    let mut next = |n: usize| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % n as u64) as usize
    };
    let mut read = 0;
    for _ in 0..3000 {
        let mut b = good.as_bytes().to_vec();
        for _ in 0..1 + next(3) {
            let at = next(b.len());
            match next(3) {
                0 => b[at] = b" 9>$<\n\xC3M"[next(8)],
                1 => {
                    b.remove(at);
                }
                _ => b.insert(at, b"\n><$"[next(4)]),
            }
        }
        if let Ok((doc, _)) = import(&String::from_utf8_lossy(&b)) {
            // what is read is a valid document, and what it writes as SDF is read again with the same data
            assert_eq!(Document::from_json(&doc.to_json()).as_ref(), Ok(&doc));
            let data: Vec<_> = doc.atoms.iter().flat_map(|a| a.data()).collect();
            assert!(data_valid(&data) || data.len() > MAX_DATA.0);
            if let Ok((back, info)) = import(&doc.to_sdf()) {
                assert_eq!(
                    back.atoms.iter().flat_map(|a| a.data()).collect::<Vec<_>>(),
                    data
                );
                assert_eq!(info.data_left, 0);
            }
            read += 1;
        }
    }
    assert!(read > 300, "{read}");
}
