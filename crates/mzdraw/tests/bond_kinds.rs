//! Bond kinds (arch-web-20): `bold`, `dative` (an arrow from the donor `a` to the acceptor `b`) and `hbond` (dotted) on the field `k` of
//! bonds: the operation, the drawing, the tools, the menu, the round trips of V3000, KET and CDXML, and what the other formats list.
//! Synthetic structures only.

use mzdraw::editor::{CHANGED, MOUSE};
use mzdraw::io::import;
use mzdraw::ops::{Op, Session};
use mzdraw::{Document, Editor};

fn drawn(smiles: &str) -> Editor {
    let mut e = Editor::default();
    e.import(smiles).unwrap();
    e.fit(800.0, 600.0);
    e
}

fn kind(id: u32, k: &str) -> Op {
    Op::BondSet {
        id,
        o: None,
        s: None,
        pos: None,
        k: Some(k.into()),
        rev: false,
    }
}

fn session(d: &Document) -> Session {
    let mut s = Session::default();
    s.doc = d.clone();
    s
}

fn svg_of(d: &Document) -> String {
    let mut e = Editor::default();
    e.session.doc = d.clone();
    e.svg()
}

/// A complex drawn by hand: ammonia donating to boron (a dative bond from N), and a water molecule held by a hydrogen bond.
fn complex() -> Document {
    let mut e = drawn("N.B.O");
    let doc = e.doc().clone();
    let (n, b, o) = (doc.atoms[0].id, doc.atoms[1].id, doc.atoms[2].id);
    let mut s = session(&doc);
    s.run(&[
        Op::BondAdd {
            id: None,
            a: n,
            b,
            o: 1,
            s: 0,
        },
        Op::BondAdd {
            id: None,
            a: o,
            b: n,
            o: 1,
            s: 0,
        },
    ])
    .unwrap();
    let (d1, d2) = (s.doc.bonds[0].id, s.doc.bonds[1].id);
    s.run(&[kind(d1, "dative"), kind(d2, "hbond")]).unwrap();
    e.session.doc = s.doc;
    e.doc().clone()
}

fn kinds(d: &Document) -> Vec<(u32, u32, String)> {
    let at = |id: u32| d.atoms.iter().position(|a| a.id == id).unwrap() as u32;
    d.bonds
        .iter()
        .map(|b| (at(b.a), at(b.b), b.kind()))
        .collect()
}

#[test]
fn a_kind_comes_and_goes_exactly_and_a_drawing_without_one_is_the_same_bytes() {
    let e = drawn("CCO");
    let before = e.doc().to_json();
    assert!(!before.contains("\"k\""));
    let id = e.doc().bonds[0].id;
    for k in ["bold", "dative", "hbond"] {
        let mut s = session(e.doc());
        s.run(&[kind(id, k)]).unwrap();
        let after = s.doc.to_json();
        assert!(after.contains(&format!("\"k\":\"{k}\"")), "{after}");
        assert_eq!(Document::from_json(&after).unwrap().to_json(), after);
        s.run(&[kind(id, "solid")]).unwrap();
        assert_eq!(s.doc.to_json(), before, "{k} taken away");
        s.undo();
        assert_eq!(s.doc.to_json(), after);
        s.undo();
        assert_eq!(s.doc.to_json(), before);
    }
}

#[test]
fn a_dative_or_hydrogen_bond_is_single_a_new_order_takes_it_away_and_a_refusal_changes_nothing() {
    let e = drawn("CC=C");
    let (single, double) = (e.doc().bonds[0].id, e.doc().bonds[1].id);
    let mut s = session(e.doc());
    let before = s.doc.to_json();
    // refused on a double bond, nothing changes (the whole batch is undone)
    for k in ["dative", "hbond"] {
        let refused = s.run(&[kind(single, "bold"), kind(double, k)]);
        assert!(refused.is_err(), "{k}");
        assert_eq!(s.doc.to_json(), before);
    }
    // bold fits any order
    s.run(&[kind(double, "bold")]).unwrap();
    assert_eq!(s.doc.bond(double).unwrap().kind(), "bold");
    s.undo();
    s.run(&[kind(single, "dative")]).unwrap();
    let set_order = |id, o| Op::BondSet {
        id,
        o: Some(o),
        s: None,
        pos: None,
        k: None,
        rev: false,
    };
    // a new order other than 1 takes the kind away; order 1 keeps it
    s.run(&[set_order(single, 1)]).unwrap();
    assert_eq!(s.doc.bond(single).unwrap().kind(), "dative");
    s.run(&[set_order(single, 2)]).unwrap();
    assert_eq!(
        (
            s.doc.bond(single).unwrap().o,
            s.doc.bond(single).unwrap().kind()
        ),
        (2, String::new())
    );
    s.undo();
    assert_eq!(s.doc.bond(single).unwrap().kind(), "dative");
    // a wedge mark and a kind of its own exclude each other: a mark takes the kind away, both together are refused
    let wedge = Op::BondSet {
        id: single,
        o: None,
        s: Some(1),
        pos: None,
        k: None,
        rev: false,
    };
    s.run(&[wedge]).unwrap();
    assert_eq!(s.doc.bond(single).unwrap().kind(), "");
    let both = Op::BondSet {
        id: single,
        o: None,
        s: Some(1),
        pos: None,
        k: Some("hbond".into()),
        rev: false,
    };
    assert!(s.run(&[both]).is_err());
}

#[test]
fn the_log_says_it_and_a_value_outside_the_list_is_refused() {
    let op = kind(5, "dative");
    let text = op.to_json();
    assert_eq!(text, r#"{"op":"bond.set","id":5,"k":"dative"}"#);
    assert_eq!(
        Op::parse(&serde_json::from_str(&text).unwrap()).unwrap(),
        op
    );
    for k in ["bold", "hbond", "solid"] {
        let t = format!(r#"{{"op":"bond.set","id":5,"k":"{k}"}}"#);
        assert!(Op::parse(&serde_json::from_str(&t).unwrap()).is_ok(), "{k}");
    }
    for bad in ["wiggly", "Dative", "hydrogen", "coordinate"] {
        let t = format!(r#"{{"op":"bond.set","id":5,"k":"{bad}"}}"#);
        assert!(
            Op::parse(&serde_json::from_str(&t).unwrap()).is_err(),
            "{bad}"
        );
    }
}

fn lines(svg: &str) -> Vec<String> {
    svg.lines()
        .filter(|x| x.starts_with("<line"))
        .map(String::from)
        .collect()
}

fn width_of(line: &str) -> f64 {
    let at = line.find("stroke-width=\"").unwrap() + 14;
    line[at..].split('"').next().unwrap().parse().unwrap()
}

#[test]
fn each_kind_has_its_own_drawing() {
    let e = drawn("CCO");
    let id = e.doc().bonds[0].id;
    let svg = |k: &str| {
        let mut s = session(e.doc());
        s.run(&[kind(id, k)]).unwrap();
        svg_of(&s.doc)
    };
    let plain = svg("solid");
    let (bold, dative, hbond) = (svg("bold"), svg("dative"), svg("hbond"));
    // bold: the same lines, the one bond heavier (2.5 times the stroke)
    let (lp, lb) = (lines(&plain), lines(&bold));
    assert_eq!(lp.len(), lb.len());
    let wide = lb
        .iter()
        .zip(&lp)
        .filter(|(a, b)| a != b)
        .collect::<Vec<_>>();
    assert_eq!(wide.len(), 1, "{bold}");
    assert!((width_of(wide[0].0) / width_of(wide[0].1) - 2.5).abs() < 0.01);
    // dative: the line is shortened for a head, which is a filled triangle at the acceptor (the atom `b`)
    assert!(!plain.contains("<polygon") && dative.contains("<polygon"));
    assert_eq!(lines(&dative).len(), lp.len());
    // hydrogen bond: dots, many more pieces than the plain line, none of them as long as the plain bond
    assert!(lines(&hbond).len() >= lp.len() + 3, "{hbond}");
    // every kind is drawn differently from the others
    for (i, a) in [&plain, &bold, &dative, &hbond].iter().enumerate() {
        for b in [&plain, &bold, &dative, &hbond].iter().skip(i + 1) {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn the_arrow_points_at_the_acceptor_and_turns_with_the_reverse() {
    let e = drawn("CCO");
    let id = e.doc().bonds[0].id;
    let b = e.doc().bond(id).unwrap().clone();
    let tip = |doc: &Document| {
        // the first and the second corner of the triangle: the tip is the first point of the polygon
        let svg = svg_of(doc);
        let at = svg.find("<polygon points=\"").unwrap() + 17;
        let p: Vec<f64> = svg[at..]
            .split('"')
            .next()
            .unwrap()
            .split([' ', ','])
            .map(|n| n.parse().unwrap())
            .collect();
        (p[0], p[1])
    };
    let mut s = session(e.doc());
    s.run(&[kind(id, "dative")]).unwrap();
    let t1 = tip(&s.doc);
    let flip = Op::BondSet {
        id,
        o: None,
        s: None,
        pos: None,
        k: None,
        rev: true,
    };
    s.run(&[flip]).unwrap();
    assert_eq!(
        (s.doc.bond(id).unwrap().a, s.doc.bond(id).unwrap().b),
        (b.b, b.a)
    );
    let t2 = tip(&s.doc);
    // the head sits at the other end now
    assert!(
        ((t1.0 - t2.0).powi(2) + (t1.1 - t2.1).powi(2)).sqrt() > 10.0,
        "{t1:?} {t2:?}"
    );
}

#[test]
fn a_dative_and_a_hydrogen_bond_do_not_take_the_hydrogens_of_their_ends() {
    let c = complex();
    let plain = {
        let mut p = c.clone();
        p.bonds.clear();
        p
    };
    // the implicit hydrogens of N, B and O are the ones of the atoms alone
    let h = |d: &Document| d.atoms.iter().map(|a| d.hydrogens(a)).collect::<Vec<_>>();
    assert_eq!(h(&c), h(&plain));
    // (and a plain single bond in the place of the dative one does take a hydrogen of the nitrogen)
    let mut single = c.clone();
    let dative = single
        .bonds
        .iter()
        .position(|b| b.kind() == "dative")
        .unwrap();
    single.bonds[dative].more.clear();
    assert_ne!(h(&single), h(&plain));
}

#[test]
fn v3000_ket_and_cdxml_carry_both_with_their_direction() {
    let c = complex();
    let want = kinds(&c);
    assert_eq!(want.iter().filter(|k| k.2 == "dative").count(), 1);
    // the V3000 file has the types 9 and 10, the donor first
    let v3 = c.to_molfile_v3000();
    assert!(v3.contains(" 9 1 2") || v3.contains(" 9 "), "{v3}");
    assert!(v3.contains(" 10 "), "{v3}");
    for (name, text) in [("v3000", v3), ("ket", c.to_ket()), ("cdxml", c.to_cdxml())] {
        let (back, _) = import(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut got = kinds(&back);
        let mut want = want.clone();
        got.sort();
        want.sort();
        assert_eq!(got, want, "{name}");
        assert!(back.bonds.iter().all(|b| b.o == 1), "{name}");
    }
    let ket = c.to_ket();
    assert!(
        ket.contains(r#""type":9"#) && ket.contains(r#""type":10"#),
        "{ket}"
    );
    let cdxml = c.to_cdxml();
    assert!(cdxml.contains("Order=\"dative\"") && cdxml.contains("Order=\"hydrogen\""));
}

#[test]
fn bold_goes_to_cdxml_alone_and_is_listed_for_the_other_formats() {
    let e = drawn("CCO");
    let id = e.doc().bonds[0].id;
    let mut s = session(e.doc());
    s.run(&[kind(id, "bold")]).unwrap();
    let text = s.doc.to_cdxml();
    assert!(text.contains("Display=\"Bold\""));
    let (back, _) = import(&text).unwrap();
    assert_eq!(kinds(&back), kinds(&s.doc));
    let said = |f: &str| -> Vec<&'static str> {
        mzdraw::loss::loss(&s.doc, f)
            .unwrap()
            .iter()
            .map(|l| l.what)
            .collect()
    };
    assert!(said("cdxml").is_empty());
    for f in ["molfile", "molfile-v3000", "smiles", "sdf", "ket", "cml"] {
        assert_eq!(said(f), ["bond.bold"], "{f}");
    }
}

#[test]
fn a_file_without_a_place_for_the_kinds_lists_them_and_keeps_the_connectivity() {
    let c = complex();
    for f in ["molfile", "smiles", "sdf", "cml"] {
        let names: Vec<&str> = mzdraw::loss::loss(&c, f)
            .unwrap()
            .iter()
            .map(|l| l.what)
            .collect();
        assert!(
            names.contains(&"bond.dative") && names.contains(&"bond.hbond"),
            "{f} {names:?}"
        );
    }
    for f in ["molfile-v3000", "ket", "cdxml"] {
        let names: Vec<&str> = mzdraw::loss::loss(&c, f)
            .unwrap()
            .iter()
            .map(|l| l.what)
            .collect();
        assert!(
            !names.contains(&"bond.dative") && !names.contains(&"bond.hbond"),
            "{f} {names:?}"
        );
    }
    // the V2000 file keeps the three bonds as plain single bonds
    let (back, _) = import(&c.to_molfile()).unwrap();
    assert_eq!(back.bonds.len(), 2);
    assert!(back.bonds.iter().all(|b| b.o == 1 && b.kind().is_empty()));
}

#[test]
fn the_tools_draw_a_kind_toggle_it_and_make_a_double_bond_single_for_the_two_that_need_it() {
    let mut e = drawn("CC=C");
    let (id, dbl) = (e.doc().bonds[0].id, e.doc().bonds[1].id);
    let mid = |e: &Editor, id: u32| {
        let b = e.doc().bond(id).unwrap();
        let (p, q) = (e.doc().atom(b.a).unwrap(), e.doc().atom(b.b).unwrap());
        e.view.to_px((p.x + q.x) / 2.0, (p.y + q.y) / 2.0)
    };
    let click = |e: &mut Editor, (x, y): (f64, f64)| {
        e.pointer_down(x, y, MOUSE, 0);
        e.pointer_up(x, y)
    };
    for (tool, k) in [
        ("bond:bold", "bold"),
        ("bond:dative", "dative"),
        ("bond:hbond", "hbond"),
    ] {
        e.set_tool(tool);
        assert_eq!(e.tool.name(), tool);
        let at = mid(&e, id);
        assert_eq!(click(&mut e, at) & CHANGED, CHANGED);
        assert_eq!(e.doc().bond(id).unwrap().kind(), k);
        click(&mut e, at);
        assert_eq!(e.doc().bond(id).unwrap().kind(), "");
    }
    // on a double bond: bold keeps the order, a dative or hydrogen bond makes it single
    let at = mid(&e, dbl);
    e.set_tool("bond:bold");
    click(&mut e, at);
    assert_eq!(
        (
            e.doc().bond(dbl).unwrap().o,
            e.doc().bond(dbl).unwrap().kind()
        ),
        (2, "bold".to_string())
    );
    e.set_tool("bond:dative");
    click(&mut e, at);
    assert_eq!(
        (
            e.doc().bond(dbl).unwrap().o,
            e.doc().bond(dbl).unwrap().kind()
        ),
        (1, "dative".to_string())
    );
    // one undo gives the double bond with its bold line back
    e.undo();
    assert_eq!(
        (
            e.doc().bond(dbl).unwrap().o,
            e.doc().bond(dbl).unwrap().kind()
        ),
        (2, "bold".to_string())
    );
    // from an atom: a new dative bond that starts at the atom pressed on
    let a = e.doc().atoms[0].clone();
    let (x, y) = e.view.to_px(a.x, a.y);
    let n = e.doc().bonds.len();
    e.set_tool("bond:dative");
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_up(x, y);
    assert_eq!(e.doc().bonds.len(), n + 1);
    let new = e.doc().bonds.last().unwrap();
    assert_eq!((new.a, new.kind().as_str()), (a.id, "dative"));
}

#[test]
fn the_menu_offers_the_kinds_the_bond_can_take() {
    let mut e = drawn("CC=C");
    let (single, dbl) = (e.doc().bonds[0].id, e.doc().bonds[1].id);
    fn walk(v: &serde_json::Value, id: &str) -> Option<bool> {
        if let Some(a) = v.as_array() {
            return a.iter().find_map(|x| walk(x, id));
        }
        if v["id"] == id {
            return v["enabled"].as_bool();
        }
        v.as_object()?.values().find_map(|c| walk(c, id))
    }
    let open = |e: &mut Editor, bond: u32| -> serde_json::Value {
        let b = e.doc().bond(bond).unwrap();
        let (p, q) = (e.doc().atom(b.a).unwrap(), e.doc().atom(b.b).unwrap());
        let (x, y) = e.view.to_px((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
        serde_json::from_str(&e.context_menu(x, y)).unwrap()
    };
    let menu = open(&mut e, single);
    for k in ["bold", "dative", "hbond"] {
        assert_eq!(walk(&menu, &format!("bond.k.{k}")), Some(true), "{k}");
    }
    assert_eq!(walk(&menu, "bond.reverse"), Some(false));
    let menu = open(&mut e, dbl);
    assert_eq!(walk(&menu, "bond.k.bold"), Some(true));
    assert_eq!(walk(&menu, "bond.k.dative"), Some(false));
    assert_eq!(walk(&menu, "bond.k.hbond"), Some(false));
    // a disabled choice does nothing
    assert_eq!(e.context_run("bond.k.dative"), 0);
    assert_eq!(e.doc().bond(dbl).unwrap().kind(), "");
    // the choice sets the kind; the reverse is offered for a dative bond, and turns it
    open(&mut e, single);
    e.context_run("bond.k.dative");
    assert_eq!(e.doc().bond(single).unwrap().kind(), "dative");
    let before = e.doc().bond(single).unwrap().clone();
    let menu = open(&mut e, single);
    assert_eq!(walk(&menu, "bond.reverse"), Some(true));
    e.context_run("bond.reverse");
    let after = e.doc().bond(single).unwrap();
    assert_eq!((after.a, after.b), (before.b, before.a));
}

// ---- the chemistry of a dative and a hydrogen bond (review of #115) ----------------------------------------------------------------
//
// The rule: a dative bond and a hydrogen bond are NOT valence of their ends. Hydrogens are derived from the valence of the covalent
// bonds only, so the donor and the acceptor keep the hydrogens of the atoms alone (H3N->BH3 is H6BN, not H4BN with a bond that
// consumed one hydrogen on each side). RDKit counts a dative bond for the acceptor (its atoms carry explicit hydrogen counts); a
// drawing whose hydrogens are derived would give boron two hydrogens, BH2, so the choice here is "neither end".

/// `smiles` (two pieces, atoms 1 and 2 the ends) joined by a bond of the kind `k`, drawn from atom 1 to atom 2.
fn joined(smiles: &str, k: &str) -> Document {
    let mut e = Editor::default();
    e.import(smiles).unwrap();
    let mut s = session(e.doc());
    s.run(&[Op::BondAdd {
        id: None,
        a: 1,
        b: 2,
        o: 1,
        s: 0,
    }])
    .unwrap();
    let id = s.doc.bonds.last().unwrap().id;
    s.run(&[kind(id, k)]).unwrap();
    s.doc
}

fn refusing(_: &str, _: &[String]) -> Result<mzdraw::values::Measured, &'static str> {
    panic!("the engine must not be asked for a structure with a dative or hydrogen bond");
}

#[test]
fn ammonia_to_borane_keeps_the_hydrogens_of_both_and_the_formula_is_the_adduct() {
    let d = joined("N.B", "dative");
    // N has 3 and B has 3 hydrogens: NH3->BH3
    assert_eq!(
        d.atoms.iter().map(|a| d.hydrogens(a)).collect::<Vec<_>>(),
        [3, 3]
    );
    let scan = mzdraw::values::scan(&d);
    assert_eq!(scan.structures.len(), 1, "one connected structure");
    let s = &scan.structures[0];
    assert_eq!((s.formula.as_str(), s.charge, s.atoms), ("H6BN", 0, 2));
    let comp: Vec<(&str, u32)> = s
        .composition
        .iter()
        .map(|(e, n)| (e.as_str(), *n))
        .collect();
    // (the order is the one of `outline::hill`, which puts the hydrogens first when there is no carbon)
    assert_eq!(comp, [("H", 6), ("B", 1), ("N", 1)]);
    // the plain single bond of the same drawing is H3N-BH3 minus a hydrogen on each end: the dative bond is what keeps them
    let mut single = d.clone();
    single.bonds[0].more.clear();
    let s = &mzdraw::values::scan(&single).structures[0];
    assert_eq!(s.formula, "H4BN");
}

#[test]
fn water_and_ammonia_held_by_a_hydrogen_bond_keep_their_own_hydrogens() {
    let d = joined("O.N", "hbond");
    assert_eq!(
        d.atoms.iter().map(|a| d.hydrogens(a)).collect::<Vec<_>>(),
        [2, 3]
    );
    let s = &mzdraw::values::scan(&d).structures[0];
    assert_eq!((s.formula.as_str(), s.charge), ("H5NO", 0));
}

#[test]
fn no_mass_and_no_descriptor_is_made_of_a_structure_the_engine_would_read_as_a_single_bond() {
    // the engine reads the record, where the kinds are single bonds, and counts the hydrogens of that reading (H4BN, H3NO):
    // a number that disagrees with the formula is not written; the formula stays
    for (smiles, k, formula) in [("N.B", "dative", "H6BN"), ("O.N", "hbond", "H5NO")] {
        let d = joined(smiles, k);
        let scan = mzdraw::values::scan(&d);
        assert!(scan.structures[0].linked);
        let entries = scan.measure(&[], refusing).unwrap();
        assert_eq!(entries[0].formula, formula);
        assert_eq!(entries[0].result, Err("error.draw2.valuesLink"));
        assert_eq!(scan.props(0).unwrap_err(), "error.draw2.valuesLink");
        let sel = scan.selection(&d, &[1, 2]).unwrap();
        assert_eq!(
            sel.measure(&[], refusing).unwrap_err(),
            "error.draw2.valuesLink"
        );
        assert_eq!(sel.props().unwrap_err(), "error.draw2.valuesLink");
    }
    // a structure without one is measured as before (the engine is asked)
    let plain = drawn("CCO");
    let scan = mzdraw::values::scan(plain.doc());
    assert!(!scan.structures[0].linked);
}

#[test]
fn a_selection_cut_through_a_dative_or_hydrogen_bond_leaves_no_hydrogen_behind() {
    // N alone, taken from NH3->BH3: NH3, its own three hydrogens and none added for the cut bond (closing it with a hydrogen
    // would give NH4, which this assertion would catch); the plain single bond below is N with two hydrogens plus the one that closes the cut: also NH3
    let d = joined("N.B", "dative");
    let scan = mzdraw::values::scan(&d);
    assert_eq!(scan.selection(&d, &[1]).unwrap().formula, "H3N");
    assert_eq!(scan.selection(&d, &[2]).unwrap().formula, "H3B");
    let mut single = d.clone();
    single.bonds[0].more.clear();
    let scan = mzdraw::values::scan(&single);
    assert_eq!(scan.selection(&single, &[1]).unwrap().formula, "H3N");
}

#[test]
fn the_exact_files_of_the_two_fixtures() {
    let d = joined("N.B", "dative");
    assert_eq!(d.to_smiles().unwrap(), "[NH3][BH3]");
    assert_eq!(
        d.to_molfile(),
        "mzDraw\n  mzLab             2D\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 N   0  0  0  0  0  4  0  0  0  0  0  0\n    2.0000    0.0000    0.0000 B   0  0  0  0  0  4  0  0  0  0  0  0\n  1  2  1  0  0  0  0\nM  END\n"
    );
    assert!(d.to_molfile_v3000().contains("M  V30 1 9 1 2\n"));
    let h = joined("O.N", "hbond");
    assert_eq!(h.to_smiles().unwrap(), "[OH2][NH3]");
    assert_eq!(
        h.to_molfile(),
        "mzDraw\n  mzLab             2D\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 O   0  0  0  0  0  3  0  0  0  0  0  0\n    2.0000    0.0000    0.0000 N   0  0  0  0  0  4  0  0  0  0  0  0\n  1  2  1  0  0  0  0\nM  END\n"
    );
    assert!(h.to_molfile_v3000().contains("M  V30 1 10 1 2\n"));
}

/// The foreign CDXML of the tests holds `Order="dative"` from a ring carbon (an aromatic C of the ring) to a list atom: a nonsense
/// bond chemically, synthetic on purpose. Before arch-web-20 it was a query bond read as single: the ring carbon had no hydrogen
/// left (title formula C12H9NR3+). A dative bond is not the donor's valence, so the carbon keeps its hydrogen: C12H10NR3+.
#[test]
fn the_dative_bond_of_the_foreign_cdxml_leaves_its_donor_its_hydrogen() {
    let (d, info) = import(include_str!("../../../tests/data/foreign.cdxml")).unwrap();
    let dative: Vec<_> = d.bonds.iter().filter(|b| b.kind() == "dative").collect();
    assert_eq!(dative.len(), 1);
    let donor = d.atom(dative[0].a).unwrap();
    assert_eq!((donor.el.as_str(), d.hydrogens(donor)), ("C", 1));
    let scan = mzdraw::values::scan(&d);
    assert_eq!(scan.structures[0].formula, "C12H10NR3+");
    assert_eq!(scan.structures[1].formula, "C2H4ClFO");
    assert!(info
        .notes()
        .iter()
        .all(|n| n.0 != "draw2.import.queryBonds"));
    // the same bond read as a single bond: the ring carbon would have had none
    let mut single = d.clone();
    for b in &mut single.bonds {
        b.more.retain(|m| m.0 != "k");
    }
    assert_eq!(
        mzdraw::values::scan(&single).structures[0].formula,
        "C12H9NR3+"
    );
}

#[test]
fn a_cdxml_bond_that_is_dative_or_hydrogen_and_bold_stays_what_the_order_says() {
    let c = complex();
    for (order, want) in [("dative", "dative"), ("hydrogen", "hbond")] {
        let text = c.to_cdxml().replace(
            &format!("Order=\"{order}\""),
            &format!("Order=\"{order}\" Display=\"Bold\""),
        );
        assert!(text.contains("Display=\"Bold\""));
        let (back, _) = import(&text).unwrap();
        assert_eq!(
            back.bonds.iter().filter(|b| b.kind() == want).count(),
            1,
            "{order}: {:?}",
            kinds(&back)
        );
        assert_eq!(back.bonds.iter().filter(|b| b.kind() == "bold").count(), 0);
    }
}

#[test]
fn a_v3000_reaction_keeps_each_molecules_links_and_directions() {
    // reactant 1: B->N written as "9 2 1" (the donor is atom 2); reactant 2: water and ammonia, "10 3 1": the first atom first;
    // product: plain bonds. Every molecule is read through its own door: the links of one do not leak into the next.
    let mol = |atoms: &[(&str, f64)], bonds: &[&str]| {
        let mut t = format!(
            "M  V30 BEGIN CTAB\nM  V30 COUNTS {} {} 0 0 0\nM  V30 BEGIN ATOM\n",
            atoms.len(),
            bonds.len()
        );
        for (i, (el, x)) in atoms.iter().enumerate() {
            t += &format!("M  V30 {} {el} {x} 0 0 0\n", i + 1);
        }
        t += "M  V30 END ATOM\nM  V30 BEGIN BOND\n";
        for (i, b) in bonds.iter().enumerate() {
            t += &format!("M  V30 {} {b}\n", i + 1);
        }
        t + "M  V30 END BOND\nM  V30 END CTAB\n"
    };
    let first = mol(&[("N", 0.0), ("B", 1.5)], &["9 2 1"]);
    let second = mol(&[("O", 0.0), ("C", 1.5), ("N", 3.0)], &["1 1 2", "10 3 1"]);
    let product = mol(&[("C", 0.0), ("C", 1.5)], &["1 1 2"]);
    let text = format!(
        "$RXN V3000\n\n  synthetic\n\nM  V30 COUNTS 2 1\nM  V30 BEGIN REACTANT\n{first}{second}M  V30 END REACTANT\nM  V30 BEGIN PRODUCT\n{product}M  V30 END PRODUCT\nM  END\n"
    );
    let (d, _) = import(&text).unwrap();
    let at = |id: u32| d.atom(id).unwrap().el.clone();
    let got: Vec<(String, String, String)> = d
        .bonds
        .iter()
        .map(|b| (at(b.a), at(b.b), b.kind()))
        .collect();
    assert!(
        got.contains(&("B".into(), "N".into(), "dative".into())),
        "{got:?}"
    );
    assert!(
        got.contains(&("N".into(), "O".into(), "hbond".into())),
        "{got:?}"
    );
    assert_eq!(got.iter().filter(|g| !g.2.is_empty()).count(), 2, "{got:?}");
    assert!(d.bonds.iter().all(|b| b.o == 1));
}
