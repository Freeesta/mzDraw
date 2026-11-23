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
