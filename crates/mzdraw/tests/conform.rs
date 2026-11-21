//! The drawing checks `std/1` (stereo family): per check one synthetic drawing that triggers it and a close drawing that does not,
//! the bytes of the report, the replay door, the selection of a finding, and a property test on random documents.

use mzdraw::conform::{report, Finding, CHECKS};
use mzdraw::doc::{Atom, Bond, Document, WEDGE_DOWN, WEDGE_UP, WEDGE_WAVY};
use mzdraw::Editor;

const SOLID: u8 = WEDGE_UP;
const HASHED: u8 = WEDGE_DOWN;
const WAVY: u8 = WEDGE_WAVY;

/// A drawing from atoms `(element, x, y)` (ids 1..) and bonds `(a, b, order, mark)` (ids after the atoms); the narrow end of a mark
/// is `a`. Coordinates are in bond lengths, y up.
fn draw(atoms: &[(&str, f64, f64)], bonds: &[(u32, u32, u8, u8)]) -> Document {
    let n = atoms.len() as u32;
    Document {
        next: n + bonds.len() as u32 + 1,
        atoms: (atoms.iter().enumerate())
            .map(|(i, &(el, x, y))| Atom {
                id: i as u32 + 1,
                el: el.into(),
                x,
                y,
                chg: 0,
                iso: 0,
                rad: 0,
                h: None,
                lab: String::new(),
                map: 0,
                more: Vec::new(),
            })
            .collect(),
        bonds: (bonds.iter().enumerate())
            .map(|(i, &(a, b, o, s))| Bond {
                id: n + i as u32 + 1,
                a,
                b,
                o,
                s,
                more: Vec::new(),
            })
            .collect(),
        ..Document::default()
    }
}

/// A point at `deg` degrees and length 1 from (`x`, `y`).
fn at(x: f64, y: f64, deg: f64) -> (f64, f64) {
    let r = deg.to_radians();
    (x + r.cos(), y + r.sin())
}

fn keys(doc: &Document) -> Vec<&'static str> {
    report(doc).checks.iter().map(|f| f.key).collect()
}

fn has(doc: &Document, key: &str) -> bool {
    keys(doc).contains(&key)
}

fn find<'a>(r: &'a [Finding], key: &str) -> &'a Finding {
    r.iter().find(|f| f.key == key).expect(key)
}

/// Butan-2-ol: C1 (id 1), C2 (2, the centre), C3 (3), C4 (4), O (5); the bond ids are 6 (C1-C2), 7 (C2-C3), 8 (C3-C4) and 9 (C2-O).
/// `wedge` is the mark of the bond C2-O.
fn butanol(wedge: u8) -> Document {
    let (c3x, c3y) = (0.866, -0.5);
    draw(
        &[
            ("C", -0.866, -0.5),
            ("C", 0.0, 0.0),
            ("C", c3x, c3y),
            ("C", c3x + 0.866, c3y + 0.5),
            ("O", 0.0, 1.0),
        ],
        &[(1, 2, 1, 0), (2, 3, 1, 0), (3, 4, 1, 0), (2, 5, 1, wedge)],
    )
}

#[test]
fn a_wedge_from_the_centre_of_butan_2_ol_is_clean() {
    for wedge in [SOLID, HASHED] {
        assert_eq!(report(&butanol(wedge)).checks, vec![], "mark {wedge}");
    }
}

#[test]
fn no_wedge_leaves_the_centre_undefined() {
    let r = report(&butanol(0));
    assert_eq!(keys(&butanol(0)), ["std.stereo.undefined"]);
    assert_eq!(find(&r.checks, "std.stereo.undefined").atoms, [2]);
    assert_eq!(
        r.to_json(),
        r#"{"format":"std/1","checks":[{"key":"std.stereo.undefined","grade":"info","atoms":[2]}]}"#
    );
}

#[test]
fn a_wavy_bond_at_the_centre_says_unknown_and_is_not_a_gap() {
    assert_eq!(report(&butanol(WAVY)).checks, vec![]);
}

#[test]
fn a_wedge_reversed_onto_a_ch2_is_not_at_a_centre() {
    // the same bond C2-C3 drawn from the CH2: narrow end on C3
    let mut d = butanol(0);
    d.bonds[1].a = 3;
    d.bonds[1].b = 2;
    d.bonds[1].s = SOLID;
    let r = report(&d).checks;
    assert_eq!(find(&r, "std.stereo.notCentre").atoms, [3]);
    assert_eq!(find(&r, "std.stereo.notCentre").bonds, [7]);
    // its wide end is the centre, which the two conventions read differently, and the centre itself has no mark of its own
    assert!(has(&d, "std.stereo.wideEndCentre"));
    assert!(has(&d, "std.stereo.undefined"));
    // a wedge from a plain CH2 to a terminal methyl
    let mut e = butanol(SOLID);
    e.bonds[2].s = SOLID;
    assert_eq!(find(&report(&e).checks, "std.stereo.notCentre").atoms, [3]);
}

fn diol(marks: [u8; 3]) -> Document {
    // C1 C2(OH) C3(OH) C4 as a zigzag; bonds 7 (C1-C2), 8 (C2-C3), 9 (C3-C4), 10 (C2-O), 11 (C3-O)
    draw(
        &[
            ("C", 0.0, 0.0),
            ("C", 0.866, 0.5),
            ("C", 1.732, 0.0),
            ("C", 2.598, 0.5),
            ("O", 0.866, 1.5),
            ("O", 1.732, -1.0),
        ],
        &[
            (1, 2, 1, 0),
            (2, 3, 1, marks[1]),
            (3, 4, 1, 0),
            (2, 5, 1, marks[0]),
            (3, 6, 1, marks[2]),
        ],
    )
}

#[test]
fn a_wedge_that_joins_two_centres_is_read_differently_by_the_two_conventions() {
    // 2,3-diol with one wedge on the bond between the centres
    let joined = diol([0, SOLID, 0]);
    let r = report(&joined).checks;
    let f = find(&r, "std.stereo.wideEndCentre");
    assert_eq!((f.atoms.clone(), f.bonds.clone()), (vec![3], vec![8]));
    // the close drawing: one wedge on each OH, the narrow end on the centre
    assert_eq!(report(&diol([SOLID, 0, HASHED])).checks, vec![]);
    assert_eq!(report(&diol([SOLID, 0, SOLID])).checks, vec![]);
}

/// A centre at the origin (atom 1, carbon) with a carbon neighbour at each of `degrees` (atoms 2..), the marks `(index, mark)` on
/// the bonds from the centre. Four neighbours is a quaternary carbon: no hydrogen.
fn star(degrees: &[f64], marks: &[(usize, u8)]) -> Document {
    let mut atoms = vec![("C", 0.0, 0.0)];
    let mut bonds = Vec::new();
    for (i, &deg) in degrees.iter().enumerate() {
        let (x, y) = at(0.0, 0.0, deg);
        atoms.push((["F", "Cl", "Br", "I"][i % 4], x, y));
        let s = marks.iter().find(|m| m.0 == i).map_or(0, |m| m.1);
        bonds.push((1, i as u32 + 2, 1, s));
    }
    draw(&atoms, &bonds)
}

#[test]
fn two_marks_on_opposite_bonds_are_ambiguous() {
    // CFClBrI: four different neighbours, a possible centre
    let opposite = star(&[0.0, 100.0, 180.0, 280.0], &[(0, SOLID), (2, HASHED)]);
    let r = report(&opposite).checks;
    let f = find(&r, "std.stereo.opposite");
    assert_eq!((f.atoms.clone(), f.bonds.clone()), (vec![1], vec![6, 8]));
    // 15 degrees short of 180 is fine, and the marks of any kind count
    let near = star(&[0.0, 100.0, 165.0, 280.0], &[(0, SOLID), (2, HASHED)]);
    assert!(!has(&near, "std.stereo.opposite"));
    let same_kind = star(&[0.0, 100.0, 175.0, 280.0], &[(0, SOLID), (2, SOLID)]);
    assert!(has(&same_kind, "std.stereo.opposite"));
    // a mark and a plain bond straight across is a collinear pair, not an opposite one
    let one = star(&[0.0, 100.0, 180.0, 280.0], &[(0, SOLID)]);
    assert!(!has(&one, "std.stereo.opposite"));
    assert!(has(&one, "std.stereo.collinear"));
}

#[test]
fn two_bonds_on_one_line_through_the_centre_are_collinear() {
    // butan-2-ol with C1 and C3 on the line and the OH wedged
    let mut d = butanol(SOLID);
    d.atoms[0].x = -0.866;
    d.atoms[0].y = 0.0;
    d.atoms[2].x = 0.866;
    d.atoms[2].y = 0.0;
    d.atoms[3].x = 1.732;
    d.atoms[3].y = 0.5;
    let r = report(&d).checks;
    assert_eq!(find(&r, "std.stereo.collinear").bonds, [6, 7]);
    // 15 degrees off the line is not collinear
    d.atoms[0].y = 0.23;
    assert!(!has(&d, "std.stereo.collinear"));
    // the drawing of the first test has none
    assert!(!has(&butanol(SOLID), "std.stereo.collinear"));
}

#[test]
fn three_neighbours_in_a_sector_do_not_fix_the_configuration() {
    // the three bonds within 90 degrees, one wedged: the centre lies outside the tetrahedron of its neighbours
    let mut d = butanol(SOLID);
    let place = |d: &mut Document, i: usize, deg: f64| {
        let (x, y) = at(0.0, 0.0, deg);
        d.atoms[i].x = x;
        d.atoms[i].y = y;
    };
    place(&mut d, 0, 0.0);
    place(&mut d, 2, 45.0);
    place(&mut d, 4, 90.0);
    d.atoms[3].x = 3.0;
    d.atoms[3].y = 3.0;
    let r = report(&d).checks;
    let f = find(&r, "std.stereo.sameKindThree");
    assert_eq!((f.atoms.clone(), f.bonds.clone()), (vec![2], vec![9]));
    assert_eq!(f.grade.name(), "error");
    // spread over the plane (120 degrees each) it is fixed, for a solid and for a hashed wedge
    for wedge in [SOLID, HASHED] {
        assert!(!has(&butanol(wedge), "std.stereo.sameKindThree"));
    }
    // two marks of one kind on a spread centre are still fixed
    let mut two = butanol(SOLID);
    two.bonds[0].a = 2;
    two.bonds[0].b = 1;
    two.bonds[0].s = SOLID;
    assert!(!has(&two, "std.stereo.sameKindThree"));
}

#[test]
fn a_wavy_bond_away_from_a_centre_says_nothing_it_can_say() {
    let mut d = butanol(SOLID);
    d.bonds[2].s = WAVY; // C3 to C4, both plain carbons
    let r = report(&d).checks;
    let f = find(&r, "std.stereo.wavyNotCentre");
    assert_eq!(
        (f.atoms.clone(), f.bonds.clone(), f.grade.name()),
        (vec![3], vec![8], "info")
    );
    assert!(!has(&butanol(WAVY), "std.stereo.wavyNotCentre"));
}

/// CH3-C(=X)-CH2CH3 with X = NH (`oxime` false) or N-OH: atoms C1 (1), C2 (2), N (3), CH2 (4), CH3 (5), O (6, with `oxime`).
fn imine(oxime: bool, wavy: bool) -> Document {
    let mut atoms = vec![
        ("C", -0.866, -0.5),
        ("C", 0.0, 0.0),
        ("N", 0.0, 1.0),
        ("C", 0.866, -0.5),
        ("C", 1.732, 0.0),
    ];
    let mut bonds = vec![(1, 2, 1, 0), (2, 3, 2, 0), (2, 4, 1, 0), (4, 5, 1, 0)];
    if oxime {
        atoms.push(("O", 0.866, 1.5));
        bonds.push((3, 6, 1, if wavy { WAVY } else { 0 }));
    }
    draw(&atoms, &bonds)
}

#[test]
fn a_double_bond_with_nothing_drawn_on_one_end_is_undefined() {
    let r = report(&imine(false, false)).checks;
    let f = find(&r, "std.stereo.undefined");
    assert_eq!((f.atoms.clone(), f.bonds.clone()), (vec![2, 3], vec![7]));
    // the oxime has its OH drawn: the geometry says E or Z; with a wavy bond it is said to be unknown, which is a statement
    assert!(!has(&imine(true, false), "std.stereo.undefined"));
    assert!(!has(&imine(true, true), "std.stereo.undefined"));
    assert!(!has(&imine(true, true), "std.stereo.wavyNotCentre"));
    // an ordinary alkene with a drawn substituent on each end is defined by its geometry
    assert_eq!(report(&but_2_ene()).checks, vec![]);
}

/// E-but-2-ene as a zigzag: C1 (1), C2 (2), C3 (3), C4 (4). Bond ids 5 (C1-C2), 6 (C2=C3), 7 (C3-C4).
fn but_2_ene() -> Document {
    draw(
        &[
            ("C", 0.0, 0.0),
            ("C", 0.866, 0.5),
            ("C", 1.732, 0.0),
            ("C", 2.598, 0.5),
        ],
        &[(1, 2, 1, 0), (2, 3, 2, 0), (3, 4, 1, 0)],
    )
}

#[test]
fn a_substituent_on_the_line_of_a_double_bond_cannot_be_read() {
    // the zigzag: C4 is placed down-right of C3 (trans), so the E isomer is read
    let e = but_2_ene();
    assert!(!has(&e, "std.stereo.doubleCollinear"));
    // the methyl C4 continues the line of C2=C3: neither E nor Z
    let mut straight = e.clone();
    let (c2, c3) = (straight.atoms[1].clone(), straight.atoms[2].clone());
    straight.atoms[3].x = c3.x + (c3.x - c2.x);
    straight.atoms[3].y = c3.y + (c3.y - c2.y);
    let r = report(&straight).checks;
    let f = find(&r, "std.stereo.doubleCollinear");
    assert_eq!(
        (f.atoms.clone(), f.bonds.clone(), f.grade.name()),
        (vec![2, 3], vec![6, 7], "warn")
    );
    // 20 degrees off the line it is a (poor) cis or trans drawing, not a collinear one
    straight.atoms[3].y += 0.4;
    assert!(!has(&straight, "std.stereo.doubleCollinear"));
}

/// Penta-2,3-diene CH3-CH=C=CH-CH3 with the hydrogens of the first end drawn: C1 (1), C2 (2), C3 (3), C4 (4), C5 (5), H (6) on C2.
/// Bond ids 7 (C1-C2), 8 (C2=C3), 9 (C3=C4), 10 (C4-C5), 11 (C2-H).
fn allene(marks: [u8; 2]) -> Document {
    draw(
        &[
            ("C", -0.5, 0.866),
            ("C", 0.0, 0.0),
            ("C", 1.0, 0.0),
            ("C", 2.0, 0.0),
            ("C", 2.5, -0.866),
            ("H", -0.5, -0.866),
        ],
        &[
            (2, 1, 1, marks[0]),
            (2, 3, 2, 0),
            (3, 4, 2, 0),
            (4, 5, 1, 0),
            (2, 6, 1, marks[1]),
        ],
    )
}

#[test]
fn an_allene_is_shown_by_two_opposite_wedges_at_one_end() {
    // one wedge, or two of one kind: not the form the Technical Manual reads
    let one = allene([SOLID, 0]);
    let f = find(&report(&one).checks, "std.stereo.allene").clone();
    assert_eq!((f.atoms.clone(), f.bonds.clone()), (vec![3], vec![7]));
    assert!(has(&allene([SOLID, SOLID]), "std.stereo.allene"));
    // two opposite wedges at one end: nothing to say, and no complaint that the end of an allene is not a centre
    for marks in [[SOLID, HASHED], [HASHED, SOLID]] {
        assert_eq!(report(&allene(marks)).checks, vec![], "{marks:?}");
    }
    // an allene without marks is not this check
    assert!(!has(&allene([0, 0]), "std.stereo.allene"));
}

#[test]
fn the_report_is_ordered_by_key_then_by_the_smallest_id() {
    // no mark anywhere: two centres, listed by their ids
    let bare = report(&diol([0, 0, 0]));
    let order: Vec<(&str, u32)> = bare.checks.iter().map(|f| (f.key, f.atoms[0])).collect();
    assert_eq!(
        order,
        [("std.stereo.undefined", 2), ("std.stereo.undefined", 3)]
    );
    // the wedge between the centres acts on C2 (its narrow end), so C3 is still undefined; keys sort as text
    let r = report(&diol([0, SOLID, 0]));
    assert_eq!(
        r.to_json(),
        concat!(
            r#"{"format":"std/1","checks":["#,
            r#"{"key":"std.stereo.undefined","grade":"info","atoms":[3]},"#,
            r#"{"key":"std.stereo.wideEndCentre","grade":"warn","atoms":[3],"bonds":[8]}]}"#
        )
    );
}

#[test]
fn the_report_changes_nothing_and_gives_the_same_bytes() {
    let d = diol([0, SOLID, 0]);
    let before = d.to_json();
    let (a, b) = (report(&d).to_json(), report(&d).to_json());
    assert_eq!(a, b);
    assert_eq!(d.to_json(), before);
    // the door of the editor and the replay request give the same bytes
    let mut e = Editor::default();
    e.open(&before).unwrap();
    assert_eq!(e.save("std").unwrap(), a);
    let request = serde_json::json!({ "import": before, "out": "std" }).to_string();
    assert_eq!(mzdraw::replay::replay(&request).unwrap(), a);
    // and it is no step of the history
    assert!(!e.session.can_undo());
}

#[test]
fn an_empty_drawing_has_an_empty_report() {
    let e = Editor::default();
    assert_eq!(e.save("std").unwrap(), r#"{"format":"std/1","checks":[]}"#);
}

#[test]
fn a_structure_without_stereo_has_nothing_to_say_even_with_a_wedge_lying_about() {
    // a methyl drawn with a hashed wedge: the narrow end is no centre
    let d = draw(&[("C", 0.0, 0.0), ("C", 1.0, 0.0)], &[(1, 2, 1, HASHED)]);
    assert_eq!(keys(&d), ["std.stereo.notCentre"]);
}

#[test]
fn selecting_a_check_selects_what_its_findings_are_about_and_is_no_history() {
    let mut e = Editor::default();
    e.open(&diol([0, SOLID, 0]).to_json()).unwrap();
    assert_eq!(
        e.select_check("std.stereo.wideEndCentre"),
        mzdraw::editor::REPAINT
    );
    assert_eq!(
        (e.over.sel_atoms.clone(), e.over.sel_bonds.clone()),
        (vec![3], vec![8])
    );
    assert_eq!(
        e.select_check("std.stereo.undefined"),
        mzdraw::editor::REPAINT
    );
    assert_eq!(
        (e.over.sel_atoms.clone(), e.over.sel_bonds.clone()),
        (vec![3], vec![])
    );
    // a key with no finding, or no key at all, changes nothing
    e.over.sel_atoms = vec![2];
    assert_eq!(e.select_check("std.stereo.allene"), 0);
    assert_eq!(e.select_check("nonsense"), 0);
    assert_eq!(e.over.sel_atoms, [2]);
    assert!(!e.session.can_undo());
    // two findings of one key are selected together
    e.open(&diol([0, 0, 0]).to_json()).unwrap();
    e.select_check("std.stereo.undefined");
    assert_eq!(e.over.sel_atoms, [2, 3]);
}

#[test]
fn the_panel_lists_each_key_once_with_its_count() {
    let r = report(&diol([0, 0, 0]));
    assert_eq!(
        r.counts(),
        vec![("std.stereo.undefined", mzdraw::conform::Grade::Info, 2)]
    );
    assert_eq!(
        r.counts_json(),
        r#"[{"key":"std.stereo.undefined","grade":"info","n":2}]"#
    );
    assert_eq!(report(&butanol(SOLID)).counts_json(), "[]");
}

#[test]
fn every_key_has_its_text_on_the_page() {
    let texts = include_str!("../../../web/lang/mzdraw.js");
    for (key, grade) in CHECKS {
        let text_key = format!("\"draw2.std.{}\"", key.trim_start_matches("std."));
        assert!(texts.contains(&text_key), "{text_key} for {key}");
        assert!(["error", "warn", "info"].contains(&grade.name()));
    }
}

/// xorshift: repeatable pseudo-random choices, not a dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as usize
    }
}

#[test]
fn the_report_of_a_random_document_never_panics_and_names_only_ids_that_exist() {
    let els = ["C", "C", "C", "N", "O", "S", "H", "Cl", "P", "Si", "B"];
    let mut r = Rng(0x2545_F491_4F6C_DD1D);
    for round in 0..300 {
        let n = 1 + r.next(12);
        let atoms: Vec<(&str, f64, f64)> = (0..n)
            .map(|_| {
                (
                    els[r.next(els.len())],
                    r.next(7) as f64 * 0.5,
                    r.next(7) as f64 * 0.5,
                )
            })
            .collect();
        let mut bonds: Vec<(u32, u32, u8, u8)> = Vec::new();
        for _ in 0..r.next(2 * n + 1) {
            let (a, b) = (1 + r.next(n) as u32, 1 + r.next(n) as u32);
            if a == b
                || bonds
                    .iter()
                    .any(|x| (x.0, x.1) == (a, b) || (x.0, x.1) == (b, a))
            {
                continue;
            }
            let o = 1 + r.next(3) as u8;
            let s = if o == 1 { r.next(4) as u8 } else { 0 };
            bonds.push((a, b, o, s));
        }
        let d = draw(&atoms, &bonds);
        let rep = report(&d);
        for f in &rep.checks {
            assert!(
                f.atoms.iter().all(|&i| d.atom(i).is_some()),
                "round {round}: {f:?}"
            );
            assert!(
                f.bonds.iter().all(|&i| d.bond(i).is_some()),
                "round {round}: {f:?}"
            );
            assert!(CHECKS.iter().any(|c| c.0 == f.key));
            assert!(
                f.atoms.windows(2).all(|w| w[0] < w[1]) && f.bonds.windows(2).all(|w| w[0] < w[1])
            );
        }
        assert_eq!(rep.to_json(), report(&d).to_json(), "round {round}");
    }
}
