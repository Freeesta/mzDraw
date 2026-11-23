//! KET (Ketcher / Indigo JSON): written and read from the schema. Synthetic structures only.

use mzdraw::io::import;
use mzdraw::{Document, Editor, ObjKind};
use mzlab_mol::Mol;
use serde_json::Value;

fn drawn(text: &str) -> Document {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e.doc().clone()
}

fn canonical(smiles: &str) -> String {
    Mol::from_smiles(smiles)
        .unwrap()
        .canonical()
        .to_smiles()
        .unwrap()
}

#[test]
fn a_drawing_goes_out_as_ket_by_the_schema() {
    let mut d = drawn("N[C@@H](C)C(=O)[O-]");
    d.atoms[2].lab = "R2".into();
    let v: Value = serde_json::from_str(&d.to_ket()).unwrap();
    // the only top-level keys the schema allows; the molecule is referenced from the root
    let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["mol0", "root"]);
    assert_eq!(v["root"]["nodes"], serde_json::json!([{ "$ref": "mol0" }]));
    let mol = &v["mol0"];
    assert_eq!(mol["type"], "molecule");
    let atoms = mol["atoms"].as_array().unwrap();
    assert_eq!(atoms.len(), 6);
    for a in atoms {
        // an atom has a label, an R site a type; both a location of three numbers
        assert!(a["label"].is_string() != (a["type"] == "rg-label"), "{a}");
        assert_eq!(a["location"].as_array().unwrap().len(), 3, "{a}");
    }
    assert_eq!(
        atoms[2],
        serde_json::json!({ "type": "rg-label", "$refs": ["rg-2"], "location": [d.atoms[2].x, d.atoms[2].y, 0] })
    );
    assert_eq!(
        (&atoms[5]["label"], &atoms[5]["charge"]),
        (&Value::from("O"), &Value::from(-1))
    );
    assert!(atoms[0].get("charge").is_none() && atoms[0].get("radical").is_none());
    let bonds = mol["bonds"].as_array().unwrap();
    assert_eq!(bonds.len(), 5);
    let wedge: Vec<&Value> = bonds.iter().filter(|b| b.get("stereo").is_some()).collect();
    assert_eq!(wedge.len(), 1, "one wedge, as drawn");
    // the wedge starts at the stereocentre (atom 1 of the file) and is one of the schema's codes
    assert_eq!(wedge[0]["atoms"][0], 1);
    assert!([1, 6].contains(&wedge[0]["stereo"].as_i64().unwrap()));
    assert!(bonds.iter().any(|b| b["type"] == 2));
    // nothing drawn: an empty root
    assert_eq!(Document::default().to_ket(), r#"{"root":{"nodes":[]}}"#);
}

#[test]
fn ket_comes_back_as_the_same_drawing() {
    for s in [
        "CC(=O)O",
        "c1ccccc1",
        "N[C@@H](C)C(=O)O",
        "C[C@H]1CC[C@@H](O)CC1",
        "[NH4+].[Cl-]",
        "C[13CH3]",
        "[CH2]C",
        "CC[*:5].[*]CO",
    ] {
        let d = drawn(s);
        let (back, info) = import(&d.to_ket()).unwrap();
        assert_eq!(info.format, "ket");
        assert!(info.notes().is_empty(), "{s}: {:?}", info.notes());
        assert_eq!(back.to_smiles().unwrap(), d.to_smiles().unwrap(), "{s}");
        assert_eq!(back.atoms.len(), d.atoms.len());
        if !s.contains('*') {
            assert_eq!(canonical(&back.to_smiles().unwrap()), canonical(s), "{s}");
        }
        // the positions are the drawn ones (both are scaled to a bond length of 1)
        for (a, b) in d.atoms.iter().zip(&back.atoms) {
            assert!((a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3, "{s}");
        }
    }
    // a reaction: arrow with its text, plus, text; the objects keep their place next to the atoms
    let mut e = Editor::default();
    e.import(r#"{"format":"mzdraw/1","next":10,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0},{"id":2,"el":"O","x":1.0,"y":0.0},{"id":3,"el":"C","x":6.0,"y":0.0},{"id":4,"el":"N","x":7.0,"y":0.0}],"bonds":[{"id":5,"a":1,"b":2,"o":1},{"id":6,"a":3,"b":4,"o":1}],"objects":[{"id":7,"t":"arrow","x":2.0,"y":0.0,"x2":5.0,"y2":0.0,"text":"NH3, \"heat\" <Δ>"},{"id":8,"t":"plus","x":3.5,"y":-1.0},{"id":9,"t":"text","x":3.0,"y":2.0,"text":"second text"}]}"#).unwrap();
    let d = e.doc().clone();
    assert_eq!(d.objects.len(), 3);
    let (back, info) = import(&e.save("ket").unwrap()).unwrap();
    assert!(info.notes().is_empty());
    assert_eq!(back.atoms.len(), 4);
    let kinds: Vec<ObjKind> = back.objects.iter().map(|o| o.t).collect();
    // the text of the arrow is a text of its own in KET
    assert_eq!(
        kinds,
        [ObjKind::Arrow, ObjKind::Text, ObjKind::Plus, ObjKind::Text]
    );
    // a text of two lines in a file becomes two texts, one under the other
    let two = e
        .save("ket")
        .unwrap()
        .replace("second text", "second text\\\\nthird");
    let (split, _) = import(&two).unwrap();
    assert_eq!(split.objects.len(), 5);
    let third = &split.objects[4];
    assert_eq!((third.text.as_str(), third.x, third.y), ("third", 3.0, 1.4));
    let arrow = &back.objects[0];
    assert_eq!((arrow.x, arrow.y, arrow.x2, arrow.y2), (2.0, 0.0, 5.0, 0.0));
    assert_eq!(
        (
            back.objects[1].text.as_str(),
            back.objects[1].x,
            back.objects[1].y
        ),
        ("NH3, \"heat\" <Δ>", 3.5, 0.5)
    );
    assert_eq!((back.objects[2].x, back.objects[2].y), (3.5, -1.0));
    assert_eq!(back.objects[3].text, d.objects[2].text);
    assert_eq!(
        Document::from_json(&back.to_json()).unwrap(),
        back,
        "a valid document"
    );
    // objects alone
    let mut only = Editor::default();
    only.set_tool("plus");
    only.pointer_down(100.0, 100.0, 0, 0);
    only.pointer_up(100.0, 100.0);
    let (back, _) = import(&only.save("ket").unwrap()).unwrap();
    assert_eq!((back.atoms.len(), back.objects.len()), (0, 1));
}

/// A file as another editor writes it (synthetic): two molecules, an aromatic ring, a plain-string text, things out of scope.
const FOREIGN: &str = r#"{"ket_version":"2.0.0","header":{"moleculeName":"synthetic"},"root":{"nodes":[{"$ref":"mol0"},{"$ref":"mol1"},{"$ref":"rg1"},
 {"type":"arrow","data":{"mode":"filled-triangle","pos":[{"x":4,"y":0},{"x":7,"y":0,"z":0}],"height":0}},
 {"type":"text","data":{"content":"plain words","pos":[{"x":4,"y":1}]}},
 {"type":"simpleObject","data":{"mode":"ellipse","pos":[{"x":0,"y":0},{"x":1,"y":1}]}},
 {"type":"image","format":"image/png","boundingBox":{"x":0,"y":0,"width":1,"height":1},"data":""}]},
 "mol0":{"type":"molecule","atoms":[{"label":"C","location":[0,0]},{"label":"C","location":[1.5,0,0]},{"label":"C","location":[2.25,1.3,0]},{"label":"C","location":[1.5,2.6,0]},{"label":"C","location":[0,2.6,0]},{"label":"C","location":[-0.75,1.3,0]},
   {"type":"atom-list","elements":["N","O"],"location":[3,0,0]},{"label":"N","location":[-1.5,-1.3,0],"charge":1,"implicitHCount":3}],
  "bonds":[{"type":4,"atoms":[0,1]},{"type":4,"atoms":[1,2]},{"type":4,"atoms":[2,3]},{"type":4,"atoms":[3,4]},{"type":4,"atoms":[4,5]},{"type":4,"atoms":[5,0]},{"type":1,"atoms":[1,6]},{"type":9,"atoms":[0,7]}],
  "sgroups":[{"type":"SUP","atoms":[0,1],"name":"x"},{"type":"DAT","atoms":[2]}]},
 "mol1":{"type":"molecule","atoms":[{"label":"O","location":[9,0,0]},{"type":"rg-label","$refs":["rg-1"],"location":[10.5,0,0]}],"bonds":[{"customQuery":"-,=","atoms":[0,1]}]},
 "rg1":{"type":"rgroup","rlogic":{"number":1},"fragments":[]}}"#;

#[test]
fn a_foreign_ket_opens_with_what_is_out_of_scope_listed() {
    let (d, info) = import(FOREIGN).unwrap();
    assert_eq!(info.format, "ket");
    assert_eq!((d.atoms.len(), d.bonds.len(), d.objects.len()), (10, 9, 2));
    // the aromatic ring is kekulised, the atom list and the R site are labels, the charge is kept
    assert_eq!(d.bonds.iter().filter(|b| b.o == 2).count(), 3);
    let labs: Vec<&str> = d.atoms.iter().map(|a| a.lab.as_str()).collect();
    assert_eq!(labs, ["", "", "", "", "", "", "R", "", "", "R1"]);
    assert_eq!((d.atoms[7].el.as_str(), d.atoms[7].chg), ("N", 1));
    assert_eq!(
        info.notes(),
        [
            ("draw2.import.queryAtoms", 1),
            ("draw2.import.queryBonds", 1),
            ("draw2.import.sgroups", 2),
            ("draw2.import.other", 3)
        ]
    );
    // the bond of type 9 is no query bond (arch-web-20): a dative bond from the first atom of the bond to the second
    let dative: Vec<_> = d.bonds.iter().filter(|b| b.kind() == "dative").collect();
    assert_eq!(dative.len(), 1);
    assert_eq!((dative[0].a, dative[0].b, dative[0].o), (1, 8, 1));
    // the bond length of the file was 1.5: everything is scaled together, the arrow stays between the molecules
    let (ring_right, other_left) = (d.atoms[6].x, d.atoms[8].x);
    // (the ring of the fixture is drawn with rounded coordinates: its bonds are 1.5 within a thousandth)
    assert!((ring_right - 2.0).abs() < 0.01 && (other_left - 6.0).abs() < 0.01);
    let arrow = &d.objects[0];
    assert!(arrow.t == ObjKind::Arrow && ring_right < arrow.x && arrow.x2 < other_left);
    assert_eq!(
        (d.objects[1].t, d.objects[1].text.as_str()),
        (ObjKind::Text, "plain words")
    );
    assert_eq!(Document::from_json(&d.to_json()).unwrap(), d);
}

#[test]
fn a_damaged_ket_is_refused_or_read_but_never_a_panic() {
    for bad in [
        r#"{"root":{}}"#,
        r#"{"root":{"nodes":7}}"#,
        r#"{"root":{"nodes":[{"$ref":"mol0"}]},"mol0":{"type":"molecule"}}"#,
        r#"{"root":{"nodes":[{"$ref":"mol0"}]},"mol0":{"type":"molecule","atoms":[{"label":"C"}],"bonds":[{"type":1,"atoms":[0]}]}}"#,
        r#"{"root":{"nodes":[{"$ref":"mol0"}]},"mol0":{"type":"molecule","atoms":[{"label":"C"}],"bonds":[{"type":1,"atoms":[0,5]}]}}"#,
        r#"{"root":{"nodes":[{"$ref":"mol0"}]},"mol0":{"type":"molecule","atoms":[{"label":"C","location":[1e300,0,0]},{"label":"C","location":[0,0,0]},{"label":"C","location":[1,0,0]}],"bonds":[{"type":1,"atoms":[1,2]}]}}"#,
    ] {
        assert_eq!(import(bad).unwrap_err(), "error.draw2.ket", "{bad}");
    }
    // labels that would break the line they are written on become query atoms; missing things take their defaults
    let odd = r#"{"root":{"nodes":[{"$ref":"mol0"},{"$ref":"nothing"},{"type":"plus"},{"type":"text","data":{}},{"type":"arrow","data":{"pos":"x"}}, 7, null]},"mol0":{"type":"molecule","atoms":[{"label":"C CHG=5\nM  END"},{"label":""},{"label":"Zz","charge":100000,"isotope":-4},{}]}}"#;
    let (d, info) = import(odd).unwrap();
    // the plus is kept; a reference to nothing, an empty text, an arrow without points, a number and a null are counted
    assert_eq!((d.atoms.len(), d.objects.len()), (4, 1));
    assert_eq!((info.query_atoms, info.other), (4, 5), "{info:?}");
    assert!(d.atoms.iter().all(|a| a.chg == 0));
    // damage at random: bytes changed, cut, repeated
    let good = drawn("N[C@@H](C)C(=O)O").to_ket();
    let mut x = 0xD6E8_FEB8_6659_FD93u64;
    let mut next = |n: usize| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % n as u64) as usize
    };
    let (mut read, mut refused) = (0, 0);
    for text in [good.as_str(), FOREIGN] {
        for _ in 0..2500 {
            let mut b = text.as_bytes().to_vec();
            for _ in 0..1 + next(3) {
                let at = next(b.len());
                match next(3) {
                    0 => b[at] = b"0123456789-.e[]{}\",:x"[next(21)],
                    1 => {
                        b.remove(at);
                    }
                    _ => b.insert(at, b"9e[{\"-"[next(6)]),
                }
            }
            match import(&String::from_utf8_lossy(&b)) {
                Ok((doc, _)) => {
                    assert_eq!(Document::from_json(&doc.to_json()).as_ref(), Ok(&doc));
                    read += 1;
                }
                Err(_) => refused += 1,
            }
        }
    }
    assert!(
        read > 200 && refused > 200,
        "{read} read, {refused} refused"
    );
}
