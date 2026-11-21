//! Group labels written the other way round when the bonds leave to the right, so that the attachment atom of the label faces the
//! bond (`HO2C-`, `MeO-`): the tokenizer and the reversal of `label.rs`, the drawing of them in `scene.rs` (screen and SVG are the
//! one display list), the bond that stops short of what is written beyond the attachment atom, and the replay request that gives the
//! same bytes as the page. Synthetic drawings only.

use mzdraw::label::{arrange, kind, ABBREVIATIONS};
use mzdraw::scene::{self, Overlay, View, TEXT};
use mzdraw::{replay, Document, Style};

/// Labels and the way they are drawn when they grow to the left; every one of the table of `label.rs`, and some that only the
/// tokenizer reads (parentheses, a hydrogen count, other butyls).
const REVERSED: &[(&str, &str)] = &[
    // the table of abbreviations
    ("Me", "Me"),
    ("Et", "Et"),
    ("Pr", "Pr"),
    ("iPr", "iPr"),
    ("Bu", "Bu"),
    ("tBu", "tBu"),
    ("Ph", "Ph"),
    ("Bn", "Bn"),
    ("Ac", "Ac"),
    ("Bz", "Bz"),
    ("Boc", "Boc"),
    ("Cbz", "Cbz"),
    ("Fmoc", "Fmoc"),
    ("Ts", "Ts"),
    ("Ms", "Ms"),
    ("Tf", "Tf"),
    ("OMe", "MeO"),
    ("OEt", "EtO"),
    ("OAc", "AcO"),
    ("OTs", "TsO"),
    ("NMe2", "Me2N"),
    ("NO2", "O2N"),
    ("CN", "NC"),
    ("CF3", "F3C"),
    ("COOH", "HOOC"),
    ("CO2H", "HO2C"),
    ("CO2Me", "MeO2C"),
    ("CO2Et", "EtO2C"),
    ("SO3H", "HO3S"),
    ("TMS", "TMS"),
    ("TBS", "TBS"),
    ("GlcA", "GlcA"),
    // R groups and a bare count stay
    ("R", "R"),
    ("R1", "R1"),
    ("R32", "R32"),
    ("N3", "N3"),
    // what only the tokenizer reads
    ("OH", "HO"),
    ("SH", "HS"),
    ("CH3", "H3C"),
    ("CH2OH", "HOH2C"),
    ("CH2CH3", "H3CH2C"),
    ("NHBoc", "BocHN"),
    ("OBn", "BnO"),
    ("nBu", "nBu"),
    ("N(CH3)2", "(H3C)2N"),
    ("CH(CH3)2", "(H3C)2HC"),
    ("SiMe3", "Me3Si"),
    ("CCl3", "Cl3C"),
    ("ClCH2", "H2CCl"),
];

#[test]
fn a_label_is_reversed_by_units_and_each_unit_keeps_its_order() {
    assert!(REVERSED.len() >= 20);
    for (label, reversed) in REVERSED {
        assert_eq!(arrange(label, true).text, *reversed, "{label} to the left");
        assert_eq!(arrange(label, false).text, *label, "{label} to the right");
    }
    // the table is covered whole, so that a new abbreviation cannot be added without a decision about its reversal
    for (name, _) in ABBREVIATIONS {
        assert!(
            REVERSED.iter().any(|(l, _)| l == name),
            "{name} is not in the table of this test"
        );
    }
    // an abbreviation is one unit whatever its letters spell: Ac is not A-c, Boc is not B-o-c, Ph does not end in an H
    for unit in ["Boc", "Ph", "Ac", "Me", "Ts", "Cbz", "Fmoc", "GlcA", "TMS"] {
        assert_eq!(arrange(unit, true).text, unit);
        assert_eq!(arrange(&format!("O{unit}"), true).text, format!("{unit}O"));
    }
}

#[test]
fn a_label_that_is_not_made_of_units_is_drawn_as_written() {
    for label in ["", "x", "n-Bu", "CO2H+", "C O", "(CH3", "()", "ph", "Q"] {
        let (l, r) = (arrange(label, true), arrange(label, false));
        assert_eq!(
            (l.text.as_str(), r.text.as_str()),
            (label, label),
            "{label:?}"
        );
    }
}

#[test]
fn the_attachment_atom_is_the_one_that_faces_the_bond() {
    // the range of characters (indices into the text drawn) that stands on the atom of the drawing
    for (label, left, text, at) in [
        ("CO2H", false, "CO2H", (0, 1)),
        ("CO2H", true, "HO2C", (3, 4)),
        ("OAc", true, "AcO", (2, 3)),
        ("NMe2", true, "Me2N", (3, 4)),
        ("NO2", true, "O2N", (2, 3)),
        ("OMe", true, "MeO", (2, 3)),
        ("N(CH3)2", true, "(H3C)2N", (6, 7)),
        ("N(CH3)2", false, "N(CH3)2", (0, 1)),
        ("(CH3)2N", false, "(CH3)2N", (1, 2)),
        ("(CH3)2N", true, "N(H3C)2", (4, 5)),
        // an abbreviation alone keeps the end it is read from: the first letter, the last one on the left
        ("Ph", false, "Ph", (0, 1)),
        ("Ph", true, "Ph", (1, 2)),
        ("Boc", true, "Boc", (2, 3)),
        // the symbol of an R group, not its number; the symbol of an azide, not its count
        ("R1", true, "R1", (0, 1)),
        ("R12", true, "R12", (0, 1)),
        ("N3", true, "N3", (0, 1)),
        ("Cl", true, "Cl", (0, 2)),
    ] {
        let a = arrange(label, left);
        assert_eq!((a.text.as_str(), a.at), (text, at), "{label} left={left}");
    }
}

fn doc(atoms: &[(&str, f64, f64)], bonds: &[(u32, u32)]) -> Document {
    let a: Vec<String> = atoms
        .iter()
        .enumerate()
        .map(|(i, (lab, x, y))| {
            let lab = if lab.is_empty() {
                String::new()
            } else {
                format!(r#","lab":"{lab}""#)
            };
            format!(r#"{{"id":{},"el":"C","x":{x:?},"y":{y:?}{lab}}}"#, i + 1)
        })
        .collect();
    let b: Vec<String> = bonds
        .iter()
        .enumerate()
        .map(|(i, (p, q))| format!(r#"{{"id":{},"a":{p},"b":{q},"o":1}}"#, atoms.len() + i + 1))
        .collect();
    Document::from_json(&format!(
        r#"{{"format":"mzdraw/1","next":{},"atoms":[{}],"bonds":[{}]}}"#,
        atoms.len() + bonds.len() + 1,
        a.join(","),
        b.join(",")
    ))
    .unwrap()
}

/// The display list of a document at 50 pixels to the bond with the atom 1 at (100, 100): the texts as (string, x, size) and the
/// first bond line, if there is one, as (x1, y1, x2, y2).
type Texts = Vec<(String, f32, f32)>;

fn drawn(d: &Document) -> (Texts, Option<[f32; 4]>) {
    let view = View {
        scale: 50.0,
        ox: 100.0,
        oy: 100.0,
    };
    let m = Style::preset("mzlab").unwrap().metrics(50.0, false);
    let s = scene::build(d, view, &Overlay::default(), m);
    let (mut texts, mut line, mut i) = (Vec::new(), None, 0);
    while i < s.data.len() {
        let op = s.data[i];
        if op == TEXT {
            let at = s.data[i + 5] as usize;
            texts.push((s.strings[at].clone(), s.data[i + 1], s.data[i + 3]));
            i += 7;
        } else if op == scene::LINE {
            line.get_or_insert([s.data[i + 1], s.data[i + 2], s.data[i + 3], s.data[i + 4]]);
            i += 7;
        } else {
            panic!("an unexpected item of the display list: {op}");
        }
    }
    (texts, line)
}

fn bond(l: Option<[f32; 4]>) -> [f32; 4] {
    l.expect("a bond")
}

fn names(t: &[(String, f32, f32)]) -> Vec<&str> {
    t.iter().map(|x| x.0.as_str()).collect()
}

#[test]
fn a_group_to_the_left_of_its_bond_is_written_reversed_with_its_attachment_atom_on_the_atom() {
    // atom 1 (the group) at the origin, a carbon to its right: the bond leaves to the right
    let (t, line) = drawn(&doc(&[("CO2H", 0.0, 0.0), ("", 1.0, 0.0)], &[(1, 2)]));
    assert_eq!(names(&t), ["HO", "2", "C"]);
    // the C is the last text and is centred on the atom (x = 100); the digit is lowered and smaller
    let (c, h) = (&t[2], &t[0]);
    let f = c.2;
    let half = 0.7 * f / 2.0; // the advance of a capital in the display list is 0.7 of the font size
    assert!((c.1 + half - 100.0).abs() < 0.01, "{t:?}");
    assert!(t[1].2 < f && h.1 < c.1, "{t:?}");
    // the bond starts after the atom, towards the carbon: nothing of the label is on its way
    let line = bond(line);
    assert!(line[0] > 100.0 + half && line[0] < line[2], "{line:?}");
    // the same group with the bond leaving to the left keeps its written form, the C first and on the atom
    let (t, line) = drawn(&doc(&[("CO2H", 0.0, 0.0), ("", -1.0, 0.0)], &[(1, 2)]));
    assert_eq!(names(&t), ["CO", "2", "H"]);
    assert!((t[0].1 + half - 100.0).abs() < 0.01, "{t:?}");
    let line = bond(line);
    assert!(line[2] < 100.0 - half, "{line:?}");
}

#[test]
fn without_a_bond_or_with_a_bond_straight_up_or_down_the_written_form_stays() {
    for d in [
        doc(&[("CO2H", 0.0, 0.0)], &[]),
        doc(&[("CO2H", 0.0, 0.0), ("", 0.0, 1.0)], &[(1, 2)]),
        doc(&[("CO2H", 0.0, 0.0), ("", 0.0, -1.0)], &[(1, 2)]),
        // two bonds that lean opposite ways cancel out
        doc(
            &[("CO2H", 0.0, 0.0), ("", 1.0, 0.5), ("", -1.0, 0.5)],
            &[(1, 2), (1, 3)],
        ),
    ] {
        let (t, _) = drawn(&d);
        assert_eq!(names(&t)[..3], ["CO", "2", "H"], "{t:?}");
    }
}

#[test]
fn an_abbreviation_that_is_one_unit_is_never_split() {
    for (label, pieces) in [
        ("Boc", vec!["Boc"]),
        ("Ph", vec!["Ph"]),
        ("OMe", vec!["MeO"]),
    ] {
        let (t, _) = drawn(&doc(&[(label, 0.0, 0.0), ("", 1.0, 0.0)], &[(1, 2)]));
        assert_eq!(names(&t)[..pieces.len()], pieces[..], "{label}: {t:?}");
    }
}

#[test]
fn a_bond_stops_short_of_the_count_that_stands_beyond_the_attachment_atom() {
    // R1 to the left of its bond: the R is on the atom, the lowered 1 stands to its right, and the bond starts after the 1
    let (t, line) = drawn(&doc(&[("R1", 0.0, 0.0), ("", 1.0, 0.0)], &[(1, 2)]));
    assert_eq!(names(&t), ["R", "1"]);
    let line = bond(line);
    assert!(line[0] >= t[1].1 + 0.56 * t[1].2 - 0.01, "{t:?} {line:?}");
    // the written form of the other side, and a label without a trailing count, are not moved by it
    let bare = bond(drawn(&doc(&[("OMe", 0.0, 0.0), ("", 1.0, 0.0)], &[(1, 2)])).1);
    let plain = bond(drawn(&doc(&[("R1", 0.0, 0.0), ("", -1.0, 0.0)], &[(1, 2)])).1);
    assert!(
        (bare[0] - 100.0 - (100.0 - plain[0])).abs() < 0.01,
        "{bare:?} {plain:?}"
    );
}

#[test]
fn the_acid_and_the_ester_on_the_left_of_a_ring_read_the_right_way_round() {
    // methyl 4-methoxybenzoate-like drawing: CO2Me on the left of the ring, OMe on the right of it
    let sheet = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/data/flaw_sheet.smi"),
    )
    .unwrap();
    let svgs: Vec<String> = sheet
        .lines()
        .filter(|l| l.contains("\"lab\""))
        .map(|l| mzdraw::svg::to_svg(&mzdraw::io::import(l).unwrap().0))
        .collect();
    assert_eq!(svgs.len(), 2, "two labelled structures on the flaw sheet");
    let order = |svg: &str, a: &str, b: &str| svg.find(a).unwrap() < svg.find(b).unwrap();
    // CO2H on the left of its ring bond: H, O, 2, C in this order
    assert!(
        order(&svgs[0], ">HO<", ">2<") && order(&svgs[0], ">2<", ">C<"),
        "{}",
        svgs[0]
    );
    assert!(!svgs[0].contains(">CO<"));
    // CO2Me on the left: MeO, 2, C; OMe on the right of its ring: the written form
    assert!(
        order(&svgs[1], ">MeO<", ">2<") && order(&svgs[1], ">2<", ">C<"),
        "{}",
        svgs[1]
    );
    assert!(svgs[1].contains(">OMe<"), "{}", svgs[1]);
    assert!(!svgs[1].contains(">CO<"));
}

#[test]
fn a_replay_request_gives_the_reversed_label_in_the_svg_and_the_same_bytes_every_time() {
    // the route of the CLI: the commands only, the SVG out
    let request = |out: &str| {
        format!(
            r#"{{"log":[{{"op":"atom.add","el":"C","x":0,"y":0}},{{"op":"atom.add","el":"C","x":1,"y":0}},{{"op":"bond.add","a":1,"b":2}},{{"op":"atom.set","id":1,"lab":"CO2H"}}],"out":"{out}"}}"#
        )
    };
    let svg = replay::replay(&request("svg")).unwrap();
    for piece in [">HO<", ">2<", ">C<"] {
        assert!(svg.contains(piece), "{piece} in {svg}");
    }
    assert!(!svg.contains(">CO<"));
    assert_eq!(svg, replay::replay(&request("svg")).unwrap());
    // the document and the molfile do not know the way a label is drawn: they carry CO2H as written
    let doc = replay::replay(&request("mzdraw")).unwrap();
    assert!(doc.contains(r#""lab":"CO2H""#), "{doc}");
    assert!(kind("CO2H").is_some());
}
