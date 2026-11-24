//! Formats, first family: SMILES and molfile V2000 / V3000 (and SDF), in and out. Synthetic structures only.

use mzdraw::io::{import, Imported, MAX_RECORDS};
use mzdraw::{Document, Editor};
use mzlab_mol::Mol;

fn drawn(text: &str) -> Document {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e.doc().clone()
}

/// The canonical SMILES of a SMILES or of a molfile, to compare structures whatever the order and the spelling.
fn canonical(text: &str) -> String {
    let m = if text.contains("M  END") {
        Mol::from_molfile(text).unwrap()
    } else {
        Mol::from_smiles(text).unwrap()
    };
    m.canonical().to_smiles().unwrap()
}

fn labels(d: &Document) -> Vec<&str> {
    d.atoms.iter().map(|a| a.lab.as_str()).collect()
}

fn marks(d: &Document) -> Vec<u8> {
    d.bonds.iter().map(|b| b.s).collect()
}

const STRUCTURES: [&str; 12] = [
    "CC(=O)O",
    "c1ccccc1",
    "N[C@@H](C)C(=O)O",
    "N[C@H](C)C(=O)O",
    "C[C@H]1CC[C@@H](O)CC1",
    "[NH4+].[Cl-]",
    "CC(=O)[O-].[Na+]",
    "C[13CH3]",
    "O=S(=O)(O)c1ccc(C)cc1",
    "CP(C)(C)=O",
    "C#N",
    "FC(Cl)(Br)I",
];

#[test]
fn smiles_goes_out_as_a_person_writes_it_and_comes_back_the_same_structure() {
    for s in STRUCTURES {
        let out = drawn(s).to_smiles().unwrap();
        assert_eq!(canonical(&out), canonical(s), "{s} went out as {out}");
        // and once more through the drawing: nothing drifts
        assert_eq!(drawn(&out).to_smiles().unwrap(), out, "{s}");
    }
    // brackets only where a reader needs them: charge, isotope, stereo, an unusual hydrogen count, an element outside the subset
    for (s, want) in [
        ("CC(=O)O", "CC(=O)O"),
        ("c1ccccc1", "C=1C=CC=CC1"),
        ("[NH4+].[Cl-]", "[NH4+].[Cl-]"),
        ("C[13CH3]", "C[13CH3]"),
        ("CP(C)(C)=O", "CP(C)(C)=O"),
        ("C[Si](C)(C)C", "C[Si](C)(C)C"),
        ("[CH2]C", "[CH2]C"),
        ("ClCBr", "ClCBr"),
    ] {
        assert_eq!(drawn(s).to_smiles().unwrap(), want, "{s}");
    }
    assert!(drawn("N[C@@H](C)C(=O)O")
        .to_smiles()
        .unwrap()
        .contains("[C@"));
    assert_eq!(Document::default().to_smiles().unwrap(), "");
}

#[test]
fn smiles_arrive_one_per_line_with_their_names() {
    let (d, info) = import("CCO ethanol\nc1ccccc1\tbenzene\n\nC#N\n").unwrap();
    assert_eq!((info.format, info.placeholder), ("smiles", true));
    assert_eq!(
        canonical(&d.to_smiles().unwrap()),
        canonical("CCO.c1ccccc1.C#N")
    );
    // one SMILES with a name after it
    let (d, _) = import("CC(=O)O acetic acid").unwrap();
    assert_eq!(d.to_smiles().unwrap(), "CC(=O)O");
    // the R groups this module writes are read back, with their numbers, wherever they stand
    for s in [
        "[*:1]C(C=1C=CC=CC1)[*]",
        "CCO.CC[*:5].[*]CO",
        "*c1ccccc1",
        "Cl[*:32]Br",
        "[*:12]",
    ] {
        let (d, _) = import(s).unwrap();
        let want = if s.starts_with('*') {
            "[*]C=1C=CC=CC1"
        } else {
            s
        };
        assert_eq!(d.to_smiles().unwrap(), want, "{s}");
        assert!(d.atoms.iter().all(|a| a.el != "He"), "{s}");
    }
    let (d, _) = import("N[C@@H]([*:2])C(=O)O |a:1|").unwrap();
    assert_eq!(d.atoms[2].lab, "R2");
    assert!(
        d.to_smiles().unwrap().contains('@'),
        "the stereo passes an R group"
    );
    // a helium is a helium, a class beyond R32 is a bare R
    assert_eq!(import("[He]").unwrap().0.atoms[0].lab, "");
    assert_eq!(import("C[*:99]").unwrap().0.atoms[1].lab, "R");
    for bad in ["", "   \n", "not a smiles at all", "CC(\nC(", "C[*", "*("] {
        assert!(import(bad).is_err(), "{bad:?}");
    }
}

/// The "SMILES Files" rules of OpenSMILES 1.0, written as tests with SMILES of our own.
#[test]
fn a_smiles_file_follows_the_rules_of_the_format() {
    // CR/LF and LF end a line, a blank line and an indented line are ignored, a tab or a space ends the SMILES, the name keeps its spaces
    let text =
        "CCO\tethanol sample\r\n\r\n  CC(=O)O ignored\r\n\tN#N\nc1ccccc1 benzene ring  \nC#N\r\n";
    let (d, info) = import(text).unwrap();
    assert_eq!((info.format, info.records), ("smiles", 3));
    assert!(
        info.smiles_lines.is_empty()
            && info
                .notes()
                .iter()
                .all(|n| n.0 != "draw2.import.smilesLine")
    );
    assert_eq!(
        canonical(&d.to_smiles().unwrap()),
        canonical("CCO.c1ccccc1.C#N")
    );
    // the name of each record is the title of its structure, and the title line of the file it is written as
    let titles: Vec<String> = d
        .atoms
        .iter()
        .map(|a| a.title())
        .filter(|t| !t.is_empty())
        .collect();
    assert_eq!(titles, ["ethanol sample", "benzene ring"]);
    assert!(d.to_molfile().starts_with("ethanol sample\n"));
    let sdf = d.to_sdf();
    let heads: Vec<&str> = sdf
        .split("$$$$\n")
        .filter(|r| !r.is_empty())
        .map(|r| r.lines().next().unwrap())
        .collect();
    assert_eq!(heads, ["ethanol sample", "benzene ring", mzdraw::NAME]);
    // a drawing with no name is written as before
    assert!(import("CCO")
        .unwrap()
        .0
        .to_molfile()
        .starts_with(&format!("{}\n", mzdraw::NAME)));
    // the title is part of the document and survives its text
    let back = Document::from_json(&d.to_json()).unwrap();
    assert_eq!(back.title(), "ethanol sample");
    // a broken line between two good ones: both open, the line is said with its number (counting blank lines)
    let (d, info) = import("CCO\n\nC(C\nCC=O\n").unwrap();
    assert_eq!(
        (d.atoms.len(), info.records, info.smiles_lines.clone()),
        (6, 2, vec![3])
    );
    assert!(info.notes().contains(&("draw2.import.smilesLine", 3)));
    // no leading whitespace is stripped into the SMILES: an indented single line is not read
    assert_eq!(import("  CCO").unwrap_err(), "error.draw2.import");
    assert_eq!(import("CCO\n C(").unwrap().1.records, 1);
    // only broken lines: the key of a SMILES that does not read
    assert_eq!(import("C(\nC)").unwrap_err(), "error.draw2.smiles");
    // a CXSMILES tail stays with its SMILES and the name follows it
    let (d, _) = import("N[C@@H](C)C(=O)O |&1:1| alanine\nCC").unwrap();
    assert_eq!(d.title(), "alanine");
    // the 64-record window holds for a SMILES file, and says what it left out
    let many = "C\n".repeat(MAX_RECORDS + 3);
    let (_, info) = import(&many).unwrap();
    assert_eq!((info.records, info.skipped), (MAX_RECORDS, 3));
    // many broken lines: the first are named, the rest counted
    let bad = "C(\n".repeat(14);
    let (_, info) = import(&format!("C\n{bad}")).unwrap();
    let notes = info.notes();
    assert_eq!(
        notes
            .iter()
            .filter(|n| n.0 == "draw2.import.smilesLine")
            .count(),
        10
    );
    assert!(notes.contains(&("draw2.import.smilesMore", 4)));
}

#[test]
fn r_groups_and_abbreviations_go_out_in_every_form_and_r_groups_come_back() {
    // R1-CH(Ph)-R, drawn: the phenyl is an abbreviation, the ends are R groups
    let mut d = drawn("CC(C)C");
    d.atoms[0].lab = "R1".into();
    d.atoms[2].lab = "Ph".into();
    d.atoms[3].lab = "R".into();
    assert_eq!(d.to_smiles().unwrap(), "[*:1]C(C=1C=CC=CC1)[*]");
    let v2 = d.to_molfile();
    assert!(
        v2.contains("V2000") && v2.contains("M  RGP  1   1   1"),
        "{v2}"
    );
    let v3 = d.to_molfile_v3000();
    assert!(
        v3.contains("V3000") && v3.contains(" R# ") && v3.contains("RGROUPS=(1 1)"),
        "{v3}"
    );
    assert!(!v3.contains("COLLECTION") && !v3.contains("He"), "{v3}");
    assert!(v3.ends_with("M  V30 END CTAB\nM  END\n"), "{v3}");
    for text in [&v2, &v3] {
        let (back, info) = import(text).unwrap();
        assert_eq!(
            info,
            Imported {
                format: "molfile",
                records: 1,
                ..Imported::default()
            }
        );
        // the abbreviation went out as its atoms, the R groups came back as labels on the same atoms
        assert_eq!(back.atoms.len(), 9);
        assert_eq!(
            &labels(&back)[..4],
            ["R1", "", "C", "R"].map(|l| if l == "C" { "" } else { l })
        );
        assert_eq!(back.to_smiles().unwrap(), "[*:1]C(C=1C=CC=CC1)[*]");
        assert!(back.atoms.iter().all(|a| a.el != "He"));
    }
    // the document form of what came back is a valid document
    let (back, _) = import(&v3).unwrap();
    assert_eq!(Document::from_json(&back.to_json()).unwrap(), back);
}

#[test]
fn v3000_carries_the_wedges_as_drawn() {
    for s in [
        "N[C@@H](C)C(=O)O",
        "C[C@H]1CC[C@@H](O)CC1",
        "C[C@H](O)[C@@H](N)C",
    ] {
        let d = drawn(s);
        assert!(
            marks(&d).iter().any(|&m| m != 0),
            "{s} is drawn with wedges"
        );
        let v3 = d.to_molfile_v3000();
        assert!(v3.contains("V3000") && v3.contains("CFG="), "{v3}");
        // the same stereo as the V2000 file and as the SMILES it came from
        assert_eq!(canonical(&v3), canonical(s), "{s}");
        assert_eq!(canonical(&v3), canonical(&d.to_molfile()), "{s}");
        // read back into a drawing: the same structure, with wedges
        let (back, _) = import(&v3).unwrap();
        assert_eq!(canonical(&back.to_smiles().unwrap()), canonical(s), "{s}");
        assert_eq!(back.atoms.len(), d.atoms.len());
    }
    // a wavy bond is CFG=2 and a hashed one CFG=3, from the narrow end
    let mut d = drawn("CC(N)O");
    (d.bonds[1].s, d.bonds[2].s) = (3, 2);
    (d.bonds[2].a, d.bonds[2].b) = (d.bonds[2].b, d.bonds[2].a);
    let v3 = d.to_molfile_v3000();
    assert!(
        v3.contains("M  V30 2 1 2 3 CFG=2\n") && v3.contains("M  V30 3 1 4 2 CFG=3\n"),
        "{v3}"
    );
    // above 999 atoms the molfile is V3000 by itself, and keeps the marks and the R groups
    let mut big = drawn(&"C".repeat(1001));
    big.bonds[4].s = 1;
    big.atoms[0].lab = "R7".into();
    let text = big.to_molfile();
    assert!(text.contains("V3000") && text.contains("CFG=1") && text.contains("RGROUPS=(1 7)"));
    let (back, _) = import(&text).unwrap();
    assert_eq!((back.atoms.len(), back.atoms[0].lab.as_str()), (1001, "R7"));
}

#[test]
fn every_record_of_an_sdf_is_read_side_by_side() {
    let record = |s: &str| drawn(s).to_molfile() + "\n>  <name>\nsynthetic\n\n$$$$\n";
    let sdf = record("CCO") + &record("c1ccccc1") + &record("N[C@@H](C)C(=O)O");
    let (d, info) = import(&sdf).unwrap();
    assert_eq!(
        (info.records, info.skipped, info.notes()),
        (3, 0, vec![("draw2.import.records", 3)])
    );
    assert_eq!(
        canonical(&d.to_smiles().unwrap()),
        canonical("CCO.c1ccccc1.N[C@@H](C)C(=O)O")
    );
    // valid as a document: ids unique, bonds inside their own record; and no record sits on another
    assert_eq!(Document::from_json(&d.to_json()).unwrap(), d);
    let x = |range: std::ops::Range<usize>| {
        let xs = d.atoms[range].iter().map(|a| a.x);
        (
            xs.clone().fold(f64::MAX, f64::min),
            xs.fold(f64::MIN, f64::max),
        )
    };
    assert!(x(0..3).1 + 1.0 < x(3..9).0 && x(3..9).1 + 1.0 < x(9..15).0);
    // Windows line ends
    let (w, _) = import(&sdf.replace('\n', "\r\n")).unwrap();
    assert_eq!(w.to_smiles().unwrap(), d.to_smiles().unwrap());
    // a long SDF is read up to the bound and the rest is said, not dropped in silence
    let long = record("CO").repeat(MAX_RECORDS + 5);
    let (d, info) = import(&long).unwrap();
    assert_eq!(
        (d.atoms.len(), info.records, info.skipped),
        (2 * MAX_RECORDS, MAX_RECORDS, 5)
    );
    assert!(info.notes().contains(&("draw2.import.skipped", 5)));
    // one bad record refuses the file: nothing is half read
    assert_eq!(
        import(&(record("CCO") + "x\n\n\nnot counts\nM  END\n$$$$\n")).unwrap_err(),
        "error.draw2.molfile"
    );
    // in the editor an SDF is one undo step
    let mut e = Editor::default();
    e.import(&sdf).unwrap();
    assert_eq!(e.doc().atoms.len(), 15);
    e.undo();
    assert!(e.doc().is_empty());
}

/// Benzene-like ring with a query atom, a query bond and an S-group, as other editors write them (synthetic).
const QUERY_V2000: &str = "synthetic\n  test\n\n  4  3  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 A   0  0  0  0  0  0  0  0  0  0  0  0\n    2.0000    0.0000    0.0000 Q   0  0  0  0  0  0  0  0  0  0  0  0\n    3.0000    0.0000    0.0000 L   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\n  2  3  6  0  0  0  0\n  3  4  8  0  0  0  0\nM  ALS   4  2 F N   O   \nM  STY  2   1 SUP   2 DAT\nM  END\n";

const QUERY_V3000: &str = "synthetic\n  test\n\n  0  0  0     0  0            999 V3000\nM  V30 BEGIN CTAB\nM  V30 COUNTS 4 3 1 0 0\nM  V30 BEGIN ATOM\nM  V30 1 C 0 0 0 0\nM  V30 2 A 1 0 0 0\nM  V30 3 [N,O] 2 0 0 0\nM  V30 4 R# 3 0 0 0 RGROUPS=(1 4)\nM  V30 END ATOM\nM  V30 BEGIN BOND\nM  V30 1 1 1 2\nM  V30 2 5 2 3\nM  V30 3 1 3 4\nM  V30 END BOND\nM  V30 BEGIN SGROUP\nM  V30 1 SUP 0 ATOMS=(1 1) LABEL=x\nM  V30 END SGROUP\nM  V30 END CTAB\nM  END\n";

#[test]
fn what_is_out_of_scope_opens_and_is_listed() {
    let (d, info) = import(QUERY_V2000).unwrap();
    assert_eq!(labels(&d), ["", "R", "R", "R"]);
    assert!(d.bonds.iter().all(|b| b.o == 1));
    assert_eq!(
        (info.query_atoms, info.query_bonds, info.sgroups),
        (3, 2, 2)
    );
    assert_eq!(
        info.notes(),
        [
            ("draw2.import.queryAtoms", 3),
            ("draw2.import.queryBonds", 2),
            ("draw2.import.sgroups", 2)
        ]
    );
    let (d, info) = import(QUERY_V3000).unwrap();
    assert_eq!(labels(&d), ["", "R", "R", "R4"]);
    assert_eq!(
        (info.query_atoms, info.query_bonds, info.sgroups),
        (2, 1, 1)
    );
    // enhanced stereo: the centres keep their wedges and the groups are kept on their atoms (`atom.sg`, rdkit-compare 3 part 2):
    // nothing is said lost, and the SMILES written back carries them in its CXSMILES tail
    let sgs = |d: &mzdraw::Document| -> Vec<String> {
        d.atoms
            .iter()
            .map(|a| a.sg())
            .filter(|g| !g.is_empty())
            .collect()
    };
    for (text, groups) in [
        ("N[C@@H](C)C(=O)O |a:1|", vec!["abs"]),
        ("N[C@@H](C)C(=O)O |&1:1|", vec!["and1"]),
        ("C[C@H](O)[C@@H](N)C |o1:1,o2:3|", vec!["or1", "or2"]),
    ] {
        let (d, info) = import(text).unwrap();
        assert!(
            marks(&d).iter().any(|&m| m != 0),
            "{text} is drawn with wedges"
        );
        let back = d.to_smiles().unwrap();
        let (core, tail) = back.split_once(' ').unwrap();
        assert_eq!(
            canonical(core),
            canonical(text.split(' ').next().unwrap()),
            "{text}"
        );
        assert_eq!(
            tail,
            text.split_once(' ').unwrap().1,
            "{text}: the tail written back"
        );
        assert_eq!(sgs(&d), groups, "{text}");
        assert_eq!(info.stereo_groups, 0, "{text}");
        assert!(
            !info
                .notes()
                .iter()
                .any(|n| n.0 == "draw2.import.stereoGroups"),
            "{text}"
        );
    }
    // the same through a V3000 file with its collections, and back to one
    let racemic = Mol::from_smiles("N[C@@H](C)C(=O)O |&1:1|").unwrap();
    let mut placed = racemic.clone();
    placed.coords = drawn("NC(C)C(=O)O").to_mol().coords;
    let v3 = placed.to_molfile("synthetic");
    assert!(v3.contains("V3000") && v3.contains("STERAC1"), "{v3}");
    let (d, info) = import(&v3).unwrap();
    assert!(marks(&d).iter().any(|&m| m != 0) && info.stereo_groups == 0);
    assert_eq!(sgs(&d), ["and1"]);
    assert!(d.to_molfile_v3000().contains("MDLV30/STERAC1 ATOMS=(1 2)"));
    assert_eq!(
        canonical(d.to_smiles().unwrap().split(' ').next().unwrap()),
        canonical("N[C@@H](C)C(=O)O")
    );
    // a plain file has nothing to say
    let (_, info) = import(&drawn("CCO").to_molfile()).unwrap();
    assert!(info.notes().is_empty());
    let (_, info) = import("CCO").unwrap();
    assert_eq!(info.notes(), [("draw2.import.placeholder", 1)]);
}

#[test]
fn a_damaged_molfile_is_refused_or_read_but_never_a_panic() {
    let mut d = drawn("N[C@@H](C)C(=O)O");
    d.atoms[0].lab = "R3".into();
    let mut x = 0x2545_F491_4F6C_DD1Du64;
    let mut next = |n: usize| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % n as u64) as usize
    };
    let mut read = 0;
    for text in [
        d.to_molfile(),
        d.to_molfile_v3000(),
        QUERY_V2000.into(),
        QUERY_V3000.into(),
    ] {
        let bytes = text.as_bytes();
        for _ in 0..1500 {
            let mut b = bytes.to_vec();
            match next(4) {
                0 => b.truncate(next(b.len())),
                1 => {
                    let at = next(b.len());
                    b[at] = b" 9R#-\n\xC3A(=)"[next(11)];
                }
                2 => {
                    let at = next(b.len());
                    b.insert(at, b"\n 0R\xE2\x82\xAC"[next(7)]);
                }
                _ => {
                    let (from, len) = (next(b.len()), next(40));
                    let piece: Vec<u8> = b[from..(from + len).min(b.len())].to_vec();
                    let at = next(b.len());
                    b.splice(at..at, piece);
                }
            }
            let text = String::from_utf8_lossy(&b).into_owned() + "\nM  END\n";
            if let Ok((doc, _)) = import(&text) {
                // what is read is a valid document
                assert_eq!(Document::from_json(&doc.to_json()).as_ref(), Ok(&doc));
                read += 1;
            }
        }
    }
    assert!(read > 100, "the damage leaves some files readable ({read})");
}

#[test]
fn a_title_is_an_operation_with_an_exact_inverse_and_the_import_makes_it() {
    use mzdraw::ops::Op;
    let mut e = Editor::default();
    e.import("CCO ethanol\nCC=O\n").unwrap();
    let titles = |e: &Editor| -> Vec<String> {
        (e.doc().atoms.iter().map(|a| a.title()))
            .filter(|t| !t.is_empty())
            .collect()
    };
    assert_eq!(titles(&e), ["ethanol"]);
    let before = e.doc().to_json();
    let mut s = mzdraw::Session::new(e.doc().clone());
    for bad in ["two\nlines", "tab\there", &"x".repeat(81)] {
        let err = s
            .run(&[Op::AtomTitle {
                id: 1,
                title: bad.into(),
            }])
            .unwrap_err();
        assert_eq!(err.key, "error.draw2.title", "{bad:?}");
    }
    let second = e.doc().atoms[3].id;
    s.run(&[Op::AtomTitle {
        id: second,
        title: "acetaldehyde".into(),
    }])
    .unwrap();
    assert_eq!(s.doc.file_title(), "ethanol");
    s.undo();
    assert_eq!(s.doc.to_json(), before);
    // the operation's text form
    let op = Op::AtomTitle {
        id: 1,
        title: "a \"b\"".into(),
    };
    assert_eq!(
        op.to_json(),
        r#"{"op":"atom.title","id":1,"title":"a \"b\""}"#
    );
}

#[test]
fn names_are_cleaned_and_cut_in_the_open_and_the_work_is_bounded() {
    // a control character inside a name is a space (nothing glued, nothing dropped); a name over 80 characters is cut and counted
    let long = "n".repeat(100);
    let (d, info) = import(&format!("CCO a\u{1}b\tc\nCC {long}\n")).unwrap();
    let titles: Vec<String> = (d.atoms.iter().map(|a| a.title()))
        .filter(|t| !t.is_empty())
        .collect();
    assert_eq!(titles, ["a b c".to_string(), "n".repeat(80)]);
    assert!(info.notes().contains(&("draw2.import.nameCut", 1)));
    // at most MAX_RECORDS lines are parsed, good or broken; the rest is counted as not read, whatever the size of the input
    let text = format!("C\n{}", "C(\n".repeat(200));
    let (_, info) = import(&text).unwrap();
    assert_eq!(info.records + info.smiles_lines.len(), MAX_RECORDS);
    assert_eq!(info.skipped, 201 - MAX_RECORDS);
    let notes = info.notes();
    assert_eq!(
        notes
            .iter()
            .filter(|n| n.0 == "draw2.import.smilesLine")
            .count(),
        10
    );
    assert!(notes.contains(&("draw2.import.smilesMore", MAX_RECORDS - 1 - 10)));
    // only broken lines: refused, and the work is the same bound
    assert_eq!(
        import(&"C(\n".repeat(500)).unwrap_err(),
        "error.draw2.smiles"
    );
}
