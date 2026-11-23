//! The side of the second line of a double bond (`pos`) and the dashed displays (`k`): the field, the operation `bond.set`, the
//! drawing, the CDXML round trip and what a format cannot hold. Synthetic structures only.

use mzdraw::editor::{CHANGED, MOUSE};
use mzdraw::ops::{Op, Session};
use mzdraw::{Document, Editor};

fn drawn(smiles: &str) -> Editor {
    let mut e = Editor::default();
    e.import(smiles).unwrap();
    e.fit(800.0, 600.0);
    e
}

fn set(id: u32, pos: Option<&str>, k: Option<&str>) -> Op {
    Op::BondSet {
        id,
        o: None,
        s: None,
        pos: pos.map(Into::into),
        k: k.map(Into::into),
        rev: false,
    }
}

/// The id of the first double bond of a drawing.
fn double(d: &Document) -> u32 {
    d.bonds.iter().find(|b| b.o == 2).unwrap().id
}

fn svg_of(d: &Document) -> String {
    let mut e = Editor::default();
    e.session.doc = d.clone();
    e.svg()
}

fn session(d: &Document) -> Session {
    let mut s = Session::default();
    s.doc = d.clone();
    s
}

#[test]
fn a_drawing_without_them_is_the_same_bytes_and_a_value_comes_and_goes_exactly() {
    let e = drawn("C1CCC=CC1");
    let before = e.doc().to_json();
    assert!(!before.contains("\"pos\"") && !before.contains("\"k\""));
    let id = double(e.doc());
    let mut s = session(e.doc());
    s.run(&[set(id, Some("l"), Some("dash"))]).unwrap();
    let after = s.doc.to_json();
    assert!(after.contains("\"pos\":\"l\"") && after.contains("\"k\":\"dash\""));
    // the document reads back as it was written
    assert_eq!(Document::from_json(&after).unwrap().to_json(), after);
    // auto and solid take them away: the same bytes as before
    s.run(&[set(id, Some("auto"), Some("solid"))]).unwrap();
    assert_eq!(s.doc.to_json(), before);
    // undo gives the exact document at each step
    s.undo();
    assert_eq!(s.doc.to_json(), after);
    s.undo();
    assert_eq!(s.doc.to_json(), before);
}

#[test]
fn the_log_says_it_and_a_malformed_op_is_refused_changing_nothing() {
    let op = set(5, Some("l"), None);
    let text = op.to_json();
    assert_eq!(text, r#"{"op":"bond.set","id":5,"pos":"l"}"#);
    assert_eq!(
        Op::parse(&serde_json::from_str(&text).unwrap()).unwrap(),
        op
    );
    for bad in [
        r#"{"op":"bond.set","id":5,"pos":"up"}"#,
        r#"{"op":"bond.set","id":5,"k":"wiggly"}"#,
        r#"{"op":"bond.set","id":5,"pos":3}"#,
    ] {
        assert!(
            Op::parse(&serde_json::from_str(bad).unwrap()).is_err(),
            "{bad}"
        );
    }
    // a side on a bond that is not double, a kind on a wedge: refused with the error key, nothing changes
    let e = drawn("CCC=C");
    let mut s = session(e.doc());
    let before = s.doc.to_json();
    let single = s.doc.bonds.iter().find(|b| b.o == 1).unwrap().id;
    assert_eq!(
        s.run(&[set(single, Some("r"), None)]).unwrap_err().key,
        "error.draw2.bond"
    );
    let wedge = Op::BondSet {
        id: single,
        o: None,
        s: Some(1),
        pos: None,
        k: Some("dash".into()),
        rev: false,
    };
    assert_eq!(s.run(&[wedge]).unwrap_err().key, "error.draw2.bond");
    assert_eq!(s.doc.to_json(), before);
}

#[test]
fn a_new_order_drops_the_side_and_a_new_mark_drops_the_kind() {
    let e = drawn("CCC=C");
    let id = double(e.doc());
    let mut s = session(e.doc());
    s.run(&[set(id, Some("r"), Some("hash"))]).unwrap();
    let order = |o| Op::BondSet {
        id,
        o: Some(o),
        s: None,
        pos: None,
        k: None,
        rev: false,
    };
    s.run(&[order(3)]).unwrap();
    let b = s.doc.bond(id).unwrap();
    assert_eq!((b.pos(), b.kind()), (String::new(), "hash".into()));
    s.run(&[order(2)]).unwrap();
    assert_eq!(s.doc.bond(id).unwrap().pos(), "");
    s.run(&[order(1)]).unwrap();
    s.run(&[Op::BondSet {
        id,
        o: None,
        s: Some(1),
        pos: None,
        k: None,
        rev: false,
    }])
    .unwrap();
    assert_eq!(s.doc.bond(id).unwrap().kind(), "");
}

#[test]
fn the_two_sides_of_a_ring_double_differ_in_the_second_line_only() {
    let e = drawn("C1CCC=CC1");
    let id = double(e.doc());
    let draw = |pos: &str| {
        let mut s = session(e.doc());
        s.run(&[set(id, Some(pos), None)]).unwrap();
        svg_of(&s.doc)
    };
    let (l, r, c) = (draw("l"), draw("r"), draw("c"));
    // the metadata holds the document, which differs by the field itself: the drawing is the lines
    let lines = |t: &str| -> Vec<String> {
        t.lines()
            .filter(|x| x.starts_with("<line"))
            .map(String::from)
            .collect()
    };
    let (ll, rl) = (lines(&l), lines(&r));
    assert_eq!(ll.len(), rl.len());
    let diff = ll.iter().zip(&rl).filter(|(a, b)| a != b).count();
    // the one second line: the plain line of the bond stays where it is
    assert_eq!(diff, 1, "{l}\n{r}");
    assert_ne!(lines(&c), ll);
    // the automatic drawing is one of the two sides inside the ring
    let auto = lines(&svg_of(e.doc()));
    assert!(auto == ll || auto == rl);
}

#[test]
fn a_dashed_line_is_dashes_and_a_hashed_line_is_ticks() {
    let e = drawn("CC=C");
    let id = double(e.doc());
    let count = |k: &str| {
        let mut s = session(e.doc());
        s.run(&[set(id, None, Some(k))]).unwrap();
        svg_of(&s.doc).matches("<line").count()
    };
    let plain = count("solid");
    assert!(count("dash") > plain);
    assert!(count("bdash") > plain);
    assert!(count("hash") > plain);
    // a bold dashed line is heavier than a dashed one
    let width = |k: &str| {
        let mut s = session(e.doc());
        s.run(&[set(id, None, Some(k))]).unwrap();
        svg_of(&s.doc)
    };
    assert_ne!(width("dash"), width("bdash"));
}

#[test]
fn the_double_bond_tool_walks_the_side_and_the_display_tools_set_the_kind() {
    let mut e = drawn("CCC=C");
    let id = double(e.doc());
    let b = e.doc().bond(id).unwrap().clone();
    let (p, q) = (e.doc().atom(b.a).unwrap(), e.doc().atom(b.b).unwrap());
    let (x, y) = e.view.to_px((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
    let click = |e: &mut Editor| {
        e.pointer_down(x, y, MOUSE, 0);
        e.pointer_up(x, y)
    };
    e.set_tool("bond:2");
    let mut seen = Vec::new();
    for _ in 0..5 {
        assert_eq!(click(&mut e) & CHANGED, CHANGED);
        seen.push(e.doc().bond(id).unwrap().pos());
    }
    assert_eq!(seen, ["l", "c", "r", "", "l"]);
    // the dashed tool sets the kind on the bond and takes it away on a second click
    e.set_tool("bond:dash");
    click(&mut e);
    assert_eq!(e.doc().bond(id).unwrap().kind(), "dash");
    click(&mut e);
    assert_eq!(e.doc().bond(id).unwrap().kind(), "");
    // and draws a new dashed bond from an atom
    e.set_tool("bond:hash");
    let a = e.doc().atom(b.a).unwrap();
    let (ax, ay) = e.view.to_px(a.x, a.y);
    let n = e.doc().bonds.len();
    e.pointer_down(ax, ay, MOUSE, 0);
    e.pointer_up(ax, ay);
    assert_eq!(e.doc().bonds.len(), n + 1);
    assert_eq!(e.doc().bonds.last().unwrap().kind(), "hash");
}

#[test]
fn the_menu_of_a_double_bond_offers_the_side_and_the_line_style() {
    let mut e = drawn("CCC=C");
    let id = double(e.doc());
    let b = e.doc().bond(id).unwrap().clone();
    let (p, q) = (e.doc().atom(b.a).unwrap(), e.doc().atom(b.b).unwrap());
    let (x, y) = e.view.to_px((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
    let menu = e.context_menu(x, y);
    assert!(
        menu.contains("\"bond.pos.l\"") && menu.contains("\"bond.k.dash\""),
        "{menu}"
    );
    e.context_run("bond.pos.r");
    assert_eq!(e.doc().bond(id).unwrap().pos(), "r");
    let menu = e.context_menu(x, y);
    e.context_run("bond.k.bdash");
    assert_eq!(e.doc().bond(id).unwrap().kind(), "bdash");
    assert!(menu.contains("\"bond.k.hash\""));
    // a single bond has no side to choose
    let single = e.doc().bonds.iter().find(|b| b.o == 1).unwrap().clone();
    let (p, q) = (
        e.doc().atom(single.a).unwrap(),
        e.doc().atom(single.b).unwrap(),
    );
    let (x, y) = e.view.to_px((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
    assert!(!e.context_menu(x, y).contains("bond.pos."));
}

#[test]
fn cdxml_carries_the_side_and_the_dashes_and_the_loss_says_the_rest() {
    let e = drawn("CC(C)=CC(=C)C#C");
    let ids: Vec<u32> = e
        .doc()
        .bonds
        .iter()
        .filter(|b| b.o == 2)
        .map(|b| b.id)
        .collect();
    let mut s = session(e.doc());
    s.run(&[
        set(ids[0], Some("l"), Some("dash")),
        set(ids[1], Some("c"), Some("hash")),
    ])
    .unwrap();
    let plain = s.doc.bonds.iter().find(|b| b.o == 1).unwrap().id;
    s.run(&[set(plain, None, Some("bdash"))]).unwrap();
    let text = s.doc.to_cdxml();
    assert!(text.contains("DoublePosition=\"Left\"") && text.contains("DoublePosition=\"Center\""));
    assert!(text.contains("Display=\"Dash\"") && text.contains("Display=\"Hash\""));
    // read back: the same sides and kinds on the same bonds; the bold dashed line is the one that is lost
    let mut back = Editor::default();
    back.import(&text).unwrap();
    let kinds = |d: &Document| -> Vec<(String, String)> {
        d.bonds.iter().map(|b| (b.pos(), b.kind())).collect()
    };
    let mut want = kinds(&s.doc);
    for w in &mut want {
        if w.1 == "bdash" {
            w.1.clear();
        }
    }
    assert_eq!(kinds(back.doc()), want);
    let lost = mzdraw::loss::loss(&s.doc, "cdxml").unwrap();
    let names: Vec<&str> = lost.iter().map(|l| l.what).collect();
    assert_eq!(names, ["bond.bdash"]);
    // a molfile holds none of them
    let names: Vec<&str> = mzdraw::loss::loss(&s.doc, "molfile")
        .unwrap()
        .iter()
        .map(|l| l.what)
        .collect();
    for w in ["bond.bdash", "bond.dash", "bond.hash", "bond.pos"] {
        assert!(names.contains(&w), "{w} {names:?}");
    }
}
