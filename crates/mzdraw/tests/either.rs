//! A double bond whose E/Z is unknown ("either", rdkit-compare 2): which double bonds an import marks, what each file writes for the
//! mark and reads back, the operation that sets and clears it with its exact undo, and the diff that says it. Synthetic structures only.

use mzdraw::{Document, Editor, Op, Session};
use mzlab_mol::{Mol, Site};

fn doc(text: &str) -> Document {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e.doc().clone()
}

/// The double bonds marked "either", as pairs of atom ids (lower first).
fn either(d: &Document) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = (d.bonds.iter())
        .filter(|b| b.either())
        .map(|b| (b.a.min(b.b), b.a.max(b.b)))
        .collect();
    out.sort();
    out
}

fn set_mark(id: u32, o: Option<u8>, s: Option<u8>) -> Op {
    Op::BondSet {
        id,
        o,
        s,
        pos: None,
        k: None,
        rev: false,
    }
}

#[test]
fn an_import_marks_exactly_the_double_bonds_that_can_carry_e_z_and_have_none() {
    // the full search of `mzlab-mol` (`Mol::stereo_sites`) is the reference: every double-bond site it finds in a SMILES without E/Z
    // is marked, and no other bond (atom ids are the SMILES order from 1)
    for smiles in [
        "CC=CC",
        "C=CC",
        "CC(C)=CC",
        "CC(C)=C(C)C",
        "CC=C=CC",
        "C1CCCCC=C1",
        "C1CCCCCCC=C1",
        "CC=NO",
        "CC(CC)=C(C)CCC",
        "OC(=O)C=CC(=O)O",
        "c1ccccc1C=CC(=O)O",
        "C=CC=CC=CC=CC=C",
        "CC=CC=CC.C=C",
        "FC(Cl)=C(Br)I",
    ] {
        let d = doc(smiles);
        let mut sites: Vec<(u32, u32)> =
            (Mol::from_smiles(smiles).unwrap().stereo_sites().into_iter())
                .filter_map(|s| match s {
                    Site::Db(a, b) => Some((a + 1, b + 1)),
                    Site::Tetra(_) => None,
                })
                .collect();
        sites.sort();
        assert_eq!(either(&d), sites, "{smiles}");
    }
    // a double bond whose E/Z is written is drawn as it is, unmarked
    assert!(either(&doc("C/C=C/C")).is_empty());
    assert!(either(&doc(r"OC(=O)/C=C\C(=O)O")).is_empty());
}

#[test]
fn the_mark_is_written_as_either_and_read_back_from_every_file_that_holds_it() {
    let d = doc("CC=CC");
    assert_eq!(either(&d), vec![(2, 3)]);
    // V2000: bond stereo 3 on the double bond; V3000: CFG=2; CDXML: the wavy display of a double bond
    let v2000 = d.to_molfile();
    assert!(v2000.contains("  2  3  2  3"), "{v2000}");
    let v3000 = d.to_molfile_v3000();
    assert!(v3000.contains("M  V30 2 2 2 3 CFG=2"), "{v3000}");
    let cdxml = d.to_cdxml();
    assert!(cdxml.contains("Order=\"2\" Display=\"Wavy\""), "{cdxml}");
    // the SMILES says nothing of E/Z, so it reads back unmarked and is marked again
    assert_eq!(d.to_smiles().unwrap(), "CC=CC");
    for text in [v2000, v3000, cdxml, d.to_smiles().unwrap(), d.to_json()] {
        assert_eq!(either(&doc(&text)), vec![(2, 3)], "{text}");
    }
    // the record keeps the double bond unknown: no E/Z appears from the drawn geometry
    let m = d.to_mol_stereo();
    assert!(m.stereo.iter().all(|s| matches!(s, mzlab_mol::Stereo::Db { status, .. } if *status == mzlab_mol::Status::Unknown)));
    // a defined double bond writes none of these
    let e = doc("C/C=C/C");
    assert!(e.to_molfile().contains("  2  3  2  0") && !e.to_cdxml().contains("Wavy"));
}

#[test]
fn the_files_that_cannot_hold_the_mark_say_so() {
    let d = doc("CC=CC");
    for (format, lost) in [
        ("ket", true),
        ("cml", true),
        ("molfile", false),
        ("molfile-v3000", false),
        ("cdxml", false),
        ("smiles", false),
    ] {
        let items = mzdraw::loss::loss(&d, format).unwrap();
        assert_eq!(
            items.iter().any(|l| l.what == "bond.either"),
            lost,
            "{format}: {items:?}"
        );
    }
}

#[test]
fn the_operation_sets_and_clears_the_mark_with_an_exact_undo() {
    // but-2-ene drawn E (atoms 1-4, bonds 5, 6, 7; the double bond is 6)
    let mut s = Session::new(doc("C/C=C/C"));
    let start = s.doc.to_json();
    s.run(&[set_mark(6, None, Some(3))]).unwrap();
    assert_eq!(either(&s.doc), vec![(2, 3)]);
    assert_eq!(
        s.doc.bonds[1].s, 0,
        "the mark of a double bond is not a wedge"
    );
    s.run(&[set_mark(6, None, Some(0))]).unwrap();
    assert!(either(&s.doc).is_empty());
    s.run(&[set_mark(6, None, Some(3))]).unwrap();
    // a new order other than double takes it away
    s.run(&[set_mark(6, Some(1), None)]).unwrap();
    assert!(either(&s.doc).is_empty());
    s.run(&[set_mark(6, Some(2), None)]).unwrap();
    assert!(
        either(&s.doc).is_empty(),
        "it does not come back with the order"
    );
    // a wedge mark on a double bond is refused, and changes nothing
    let before = s.doc.to_json();
    assert!(s.run(&[set_mark(6, None, Some(1))]).is_err());
    assert_eq!(s.doc.to_json(), before);
    // on a single bond 3 is still the wavy wedge
    s.run(&[set_mark(5, None, Some(3))]).unwrap();
    assert_eq!(s.doc.bonds[0].s, 3);
    while s.undo() {}
    assert_eq!(
        s.doc.to_json(),
        start,
        "every step undone gives the identical document"
    );
}

#[test]
fn a_diff_says_the_mark_and_its_patch_gives_the_drawing() {
    let a = doc("C/C=C/C");
    let b = doc("CC=CC");
    // the same atoms and bonds; only the mark differs (and the coordinates, which are the same layout)
    let ops = mzdraw::diff::diff(&a, &b);
    let mut s = Session::new(a.clone());
    s.run(&ops).unwrap();
    assert_eq!(either(&s.doc), vec![(2, 3)]);
    let back = mzdraw::diff::diff(&s.doc, &a);
    s.run(&back).unwrap();
    assert!(either(&s.doc).is_empty());
}

#[test]
fn many_pieces_with_ring_centres_are_marked_without_a_search_of_the_sites() {
    // forty copies of a piece with a ring cis/trans pair and two open double bonds: a full search of the sites tries assignments of
    // the other sites and does not finish at this size; the mark comes from the constitution alone and does
    let one = "CC(C)C=CC(=O)NC1CCC(CC1)C=CC";
    let many = vec![one; 40].join(".");
    let d = doc(&many);
    assert_eq!(either(&d).len(), 80);
}

#[test]
fn a_mark_no_file_can_keep_is_listed_as_lost() {
    // a dashed "either" double bond: CDXML has one display per bond, so the dash stays and the mark is lost (and said)
    let mut s = Session::new(doc("CC=CC"));
    s.run(&[Op::parse(&serde_json::json!({"op": "bond.set", "id": 6, "k": "dash"})).unwrap()])
        .unwrap();
    assert_eq!(either(&s.doc), vec![(2, 3)]);
    let lost = |d: &Document, f: &str| {
        (mzdraw::loss::loss(d, f).unwrap().iter()).any(|l| l.what == "bond.either")
    };
    assert!(lost(&s.doc, "cdxml") && !lost(&s.doc, "molfile"));
    assert!(doc(&s.doc.to_cdxml())
        .bonds
        .iter()
        .any(|b| b.kind() == "dash"));
    // a mark set by hand on a double bond that cannot carry E/Z (a terminal =CH2) is lost by every file but the document
    let mut t = Session::new(doc("C=CC"));
    t.run(&[set_mark(4, None, Some(3))]).unwrap();
    assert_eq!(either(&t.doc), vec![(1, 2)]);
    for f in ["molfile", "molfile-v3000", "smiles", "cdxml", "ket", "cml"] {
        assert!(lost(&t.doc, f), "{f}");
    }
    assert!(!lost(&t.doc, "svg"));
}
