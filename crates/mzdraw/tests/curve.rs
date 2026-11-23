//! Lone pairs and curved arrows: where the pairs go, how an anchored curve follows its atoms without an operation, what removing an
//! anchor does (and its exact undo), the tools with mouse, finger and pen, the bend handle and the keys. Synthetic structures only.

use mzdraw::curve::{self, Curve, CurveSet};
use mzdraw::doc::ObjKind;
use mzdraw::editor::{CHANGED, MOUSE, PEN, TOUCH};
use mzdraw::editor_touch::{DOWN, MOVE, UP};
use mzdraw::{svg, Document, Editor, Op, Session};

fn doc(text: &str) -> Document {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e.doc().clone()
}

fn set_lp(id: u32, n: u8) -> Op {
    Op::AtomSet {
        id,
        el: None,
        chg: None,
        iso: None,
        rad: None,
        h: None,
        lab: None,
        map: None,
        lp: Some(n),
    }
}

fn curve_add(from: u32, to: u32, cv: Curve) -> Op {
    Op::ObjAdd {
        id: None,
        t: ObjKind::Curve,
        x: 0.0,
        y: 0.0,
        x2: 1.0,
        y2: 0.0,
        text: String::new(),
        runs: Vec::new(),
        from,
        to,
        cv,
    }
}

/// Methylamine `CN` (atoms 1 and 2, bond 3) with a lone pair on the nitrogen and a curve from it to the carbon (object 4).
fn amine() -> Session {
    let mut s = Session::new(doc("CN"));
    s.run(&[set_lp(2, 1)]).unwrap();
    let cv = Curve {
        fl: true,
        bend: 0.4,
        ..Curve::default()
    };
    s.run(&[curve_add(2, 1, cv)]).unwrap();
    assert_eq!(s.doc.objects[0].id, 4);
    s
}

fn objects_json(d: &Document) -> String {
    let t = d.to_json();
    t[t.find("\"objects\"").unwrap()..].to_string()
}

fn angle_gap(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(2.0 * std::f64::consts::PI);
    d.min(2.0 * std::f64::consts::PI - d)
}

#[test]
fn pairs_sit_in_the_free_sides_of_the_atom() {
    // water drawn as a bent chain: the two pairs of the oxygen stay away from both bonds and from each other
    let d = doc(
        r#"{"format":"mzdraw/1","next":6,"atoms":[{"id":1,"el":"H","x":-0.5,"y":0.866},{"id":2,"el":"O","x":0,"y":0},{"id":3,"el":"H","x":0.5,"y":0.866}],"bonds":[{"id":4,"a":1,"b":2,"o":1},{"id":5,"a":2,"b":3,"o":1}]}"#,
    );
    let dirs = curve::lp_dirs(&d, 2, 2);
    assert_eq!(dirs.len(), 2);
    let bonds = [(-0.5f64).atan2(0.866).abs(), 0.0];
    let up = [(0.866f64).atan2(-0.5), (0.866f64).atan2(0.5)];
    let _ = bonds;
    for p in &dirs {
        for b in up {
            assert!(angle_gap(*p, b) > 0.8, "a pair on a bond: {p} vs {b}");
        }
    }
    assert!(angle_gap(dirs[0], dirs[1]) > 0.8, "the two pairs apart");
    // no bond: the pairs are spread round the atom, the first one up
    let lone =
        doc(r#"{"format":"mzdraw/1","next":2,"atoms":[{"id":1,"el":"O","x":0,"y":0}],"bonds":[]}"#);
    let four = curve::lp_dirs(&lone, 1, 4);
    assert_eq!(four.len(), 4);
    for i in 0..4 {
        for j in i + 1..4 {
            assert!(angle_gap(four[i], four[j]) > 1.0, "{four:?}");
        }
    }
    assert!(curve::lp_dirs(&lone, 99, 2).is_empty());
}

#[test]
fn the_valence_rules_give_the_lone_pairs() {
    let d = doc("CN.O.[Cl-].[CH3+].[CH3-].C");
    let n = |id: u32| curve::lp_from_valence(&d, d.atom(id).unwrap());
    let by_el = |el: &str, chg: i8| {
        d.atoms
            .iter()
            .find(|a| a.el == el && a.chg == chg)
            .map(|a| n(a.id))
    };
    assert_eq!(by_el("N", 0), Some(1), "amine nitrogen");
    assert_eq!(by_el("O", 0), Some(2), "water oxygen");
    assert_eq!(by_el("Cl", -1), Some(4), "chloride");
    let carbons: Vec<(i8, u8)> = d
        .atoms
        .iter()
        .filter(|a| a.el == "C")
        .map(|a| (a.chg, n(a.id)))
        .collect();
    assert!(carbons.contains(&(1, 0)), "carbocation {carbons:?}");
    assert!(carbons.contains(&(-1, 1)), "carbanion {carbons:?}");
    assert!(carbons.contains(&(0, 0)), "methane {carbons:?}");
    // the operations leave out what is already right and name only the others
    let mut s = Session::new(d.clone());
    let ops = curve::lone_pair_ops(&s.doc, &[]);
    assert!(!ops.is_empty());
    s.run(&ops).unwrap();
    assert!(curve::lone_pair_ops(&s.doc, &[]).is_empty());
    assert_eq!(
        s.doc.atoms.iter().map(|a| a.lp()).sum::<u8>(),
        1 + 2 + 4 + 1
    );
}

#[test]
fn a_curve_follows_its_atoms_without_an_operation() {
    let mut s = amine();
    let before_doc = objects_json(&s.doc);
    let before_svg = svg::to_svg(&s.doc);
    // the carbon moves: the curve object is not touched, its picture is
    s.run(&[Op::Move {
        atoms: vec![1],
        objects: vec![],
        dx: 0.0,
        dy: 2.0,
    }])
    .unwrap();
    assert_eq!(
        objects_json(&s.doc),
        before_doc,
        "the document of the curve does not change"
    );
    let after_svg = svg::to_svg(&s.doc);
    assert_ne!(after_svg, before_svg);
    let (p, _, q) = curve::shape(&s.doc, &s.doc.objects[0]);
    let c = s.doc.atom(1).unwrap();
    assert!(
        (q.1 - c.y).abs() < 0.5 && (q.0 - c.x).abs() < 0.5,
        "the head is at the carbon: {q:?} vs ({}, {})",
        c.x,
        c.y
    );
    let n = s.doc.atom(2).unwrap();
    assert!(
        (p.0 - n.x).hypot(p.1 - n.y) < 0.4,
        "the tail is at the lone pair"
    );
}

#[test]
fn removing_an_anchor_makes_the_end_a_point_and_undo_restores_the_anchor() {
    let mut s = amine();
    let before = s.doc.clone();
    let (_, _, q) = curve::shape(&s.doc, &s.doc.objects[0]);
    s.run(&[Op::Remove {
        atoms: vec![1],
        bonds: vec![],
        objects: vec![],
    }])
    .unwrap();
    let o = &s.doc.objects[0];
    assert_eq!(
        (o.from, o.to),
        (2, 0),
        "the end on the carbon is a point now"
    );
    assert!(o.curve().fl, "the other end keeps its lone pair");
    assert!(
        ((o.x2 - q.0).hypot(o.y2 - q.1)) < 1e-3,
        "the point is where the head was"
    );
    // removing the atom of the lone pair frees that end too (and its flag)
    s.run(&[Op::Remove {
        atoms: vec![2],
        bonds: vec![],
        objects: vec![],
    }])
    .unwrap();
    let o = &s.doc.objects[0];
    assert_eq!((o.from, o.to, o.curve().fl), (0, 0, false));
    assert!(
        s.doc.to_json().contains("\"curve\""),
        "the curve stays as two points"
    );
    assert!(s.undo() && s.undo());
    assert_eq!(s.doc, before);
    assert_eq!(s.doc.to_json(), before.to_json());
    // the bond: a curve from a bond goes with the bond
    let mut s = Session::new(doc("CC"));
    s.run(&[curve_add(3, 0, Curve::default())]).unwrap();
    let before = s.doc.clone();
    s.run(&[Op::Remove {
        atoms: vec![],
        bonds: vec![3],
        objects: vec![],
    }])
    .unwrap();
    assert_eq!(s.doc.objects[0].from, 0);
    assert!(s.undo());
    assert_eq!(s.doc, before);
}

#[test]
fn anchors_are_checked_and_a_pair_that_goes_leaves_the_end_on_its_atom() {
    let mut s = Session::new(doc("CN"));
    // the same end twice, unknown ids and a lone-pair end without an atom are refused whole
    assert_eq!(
        s.run(&[curve_add(2, 2, Curve::default())]).unwrap_err().key,
        "error.draw2.anchor"
    );
    assert_eq!(
        s.run(&[curve_add(99, 1, Curve::default())])
            .unwrap_err()
            .key,
        "error.draw2.anchor"
    );
    let lone_point = Curve {
        tl: true,
        ..Curve::default()
    };
    assert_eq!(
        s.run(&[curve_add(2, 0, lone_point)]).unwrap_err().key,
        "error.draw2.anchor"
    );
    let wild = Curve {
        bend: 9.0,
        ..Curve::default()
    };
    assert_eq!(
        s.run(&[curve_add(2, 1, wild)]).unwrap_err().key,
        "error.draw2.object"
    );
    assert_eq!(s.run(&[set_lp(2, 5)]).unwrap_err().key, "error.draw2.lp");
    assert!(s.doc.objects.is_empty());
    // a pair taken away: the curve stays, and its end is drawn at the atom
    let mut s = amine();
    let (p, _, _) = curve::shape(&s.doc, &s.doc.objects[0]);
    s.run(&[set_lp(2, 0)]).unwrap();
    let (p0, _, _) = curve::shape(&s.doc, &s.doc.objects[0]);
    assert!(
        (p0.0 - p.0).hypot(p0.1 - p.1) > 0.1,
        "the end moved from the pair to the atom"
    );
    let n = s.doc.atom(2).unwrap();
    assert!((p0.0 - n.x).hypot(p0.1 - n.y) < 1e-9);
    assert!(s.undo());
    // freeing the end keeps it where it stands, with the flag taken away
    let (here, _, _) = curve::shape(&s.doc, &s.doc.objects[0]);
    s.run(&[Op::ObjSet {
        id: 4,
        text: None,
        runs: None,
        x2: None,
        y2: None,
        from: Some(0),
        to: None,
        cv: CurveSet {
            fl: Some(false),
            half: Some(true),
            ..CurveSet::default()
        },
        pt: [None, None],
    }])
    .unwrap();
    let o = &s.doc.objects[0];
    assert_eq!(o.from, 0);
    assert!(o.curve().half && !o.curve().fl);
    assert!(
        (o.x - here.0).hypot(o.y - here.1) < 1e-3,
        "the stored point is the place the pair was"
    );
    // an anchored end stores no point: what is given for it is not kept; a point is set with x and y
    assert_eq!((o.x2, o.y2), (0.0, 0.0));
    // an object that is not a curve takes no curve field
    let mut s = Session::new(doc("CC"));
    let mut bad = curve_add(
        0,
        0,
        Curve {
            half: true,
            ..Curve::default()
        },
    );
    if let Op::ObjAdd { t, .. } = &mut bad {
        *t = ObjKind::Arrow;
    }
    assert_eq!(s.run(&[bad]).unwrap_err().key, "error.draw2.object");
}

#[test]
fn the_bend_handle_and_the_control_point_agree() {
    let (p, q) = ((0.0, 0.0), (2.0, 0.0));
    for b in [-1.5, -0.4, 0.0, 0.3, 1.0, 2.5] {
        let mid = curve::at(p, curve::control(p, q, b), q, 0.5);
        let back = curve::bend_through(p, q, mid);
        assert!((back - b).abs() < 1e-9, "{b} -> {back}");
        // positive is to the left of the way it goes (y up)
        assert_eq!(mid.1 > 1e-9, b > 0.0, "{b}");
    }
    assert_eq!(curve::bend_through(p, q, (1.0, 99.0)), curve::MAX_BEND);
    // the free side: atoms above the chord, the curve bows below it
    let d = doc("CC");
    let up = curve::free_side_bend(&d, (0.0, -3.0), (3.0, -3.0));
    assert!(
        up < 0.0,
        "the atoms are above the chord (on its left): the curve bows below, to the right: {up}"
    );
    let down = curve::free_side_bend(&d, (0.0, 3.0), (3.0, 3.0));
    assert!(
        down > 0.0,
        "the atoms are below the chord: the curve bows up, to the left: {down}"
    );
}

/// The page's way: the pixel of an atom, or of the place a pair is drawn at.
fn atom_px(e: &Editor, id: u32) -> (f64, f64) {
    let a = e.doc().atom(id).unwrap();
    e.view.to_px(a.x, a.y)
}

fn pair_px(e: &Editor, id: u32) -> (f64, f64) {
    let a = e.doc().atom(id).unwrap();
    let d = curve::lp_dirs(e.doc(), id, a.lp())[0];
    e.view.to_px(
        a.x + curve::LP_REACH * d.cos(),
        a.y + curve::LP_REACH * d.sin(),
    )
}

fn amine_editor() -> Editor {
    let mut e = Editor::default();
    e.import("CN").unwrap();
    e.fit(600.0, 400.0);
    e
}

/// A press, a move and a release with one kind of pointer (mouse, pen or finger).
fn gesture(e: &mut Editor, kind: u32, from: (f64, f64), to: (f64, f64)) -> u32 {
    if kind == MOUSE {
        e.pointer_down(from.0, from.1, MOUSE, 0);
        e.pointer_move(to.0, to.1, MOUSE);
        return e.pointer_up(to.0, to.1);
    }
    e.pointer_event(DOWN, 1, from.0, from.1, kind, 0, 1000.0);
    e.pointer_event(
        MOVE,
        1,
        (from.0 + to.0) / 2.0,
        (from.1 + to.1) / 2.0,
        kind,
        0,
        1050.0,
    );
    e.pointer_event(MOVE, 1, to.0, to.1, kind, 0, 1100.0);
    e.pointer_event(UP, 1, to.0, to.1, kind, 0, 1150.0)
}

#[test]
fn the_tools_draw_a_pair_arrow_from_a_lone_pair_with_mouse_pen_and_finger() {
    for kind in [MOUSE, PEN, TOUCH] {
        let mut e = amine_editor();
        // the lone-pair tool: a click adds a pair, up to four, then none
        e.set_tool("lonepair");
        let n = atom_px(&e, 2);
        for want in [1u8, 2, 3, 4, 0] {
            let f = gesture_click(&mut e, kind, n);
            assert_eq!(f & CHANGED, CHANGED, "{kind}");
            assert_eq!(e.doc().atom(2).unwrap().lp(), want, "{kind}");
        }
        gesture_click(&mut e, kind, n);
        assert_eq!(e.doc().atom(2).unwrap().lp(), 1);
        // the curve tool: press on the pair, release on the carbon
        e.set_tool("curve:full");
        let (from, to) = (pair_px(&e, 2), atom_px(&e, 1));
        let f = gesture(&mut e, kind, from, to);
        assert_eq!(f & CHANGED, CHANGED, "{kind}: {}", e.error);
        let o = e.doc().objects.last().unwrap();
        assert_eq!(o.t, ObjKind::Curve);
        assert_eq!((o.from, o.to), (2, 1), "{kind}");
        assert!(o.curve().fl && !o.curve().tl && !o.curve().half, "{kind}");
        assert!(o.curve().bend != 0.0);
        // one undo step takes the curve away, another the pair
        e.undo();
        assert!(e.doc().objects.is_empty(), "{kind}");
    }
}

fn gesture_click(e: &mut Editor, kind: u32, at: (f64, f64)) -> u32 {
    if kind == MOUSE {
        e.pointer_down(at.0, at.1, MOUSE, 0);
        return e.pointer_up(at.0, at.1);
    }
    e.pointer_event(DOWN, 1, at.0, at.1, kind, 0, 1000.0);
    e.pointer_event(UP, 1, at.0, at.1, kind, 0, 1060.0)
}

#[test]
fn a_half_curve_from_an_atom_to_a_bond_and_a_curve_into_empty_space() {
    let mut e = amine_editor();
    e.set_tool("curve:half");
    let (a, b) = (atom_px(&e, 2), atom_px(&e, 1));
    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    gesture(&mut e, MOUSE, a, (mid.0, mid.1 + 1.0));
    let o = e.doc().objects.last().unwrap().clone();
    assert_eq!((o.from, o.to), (2, 3), "the atom and the bond");
    assert!(o.curve().half);
    // into empty space: a point
    gesture(&mut e, MOUSE, b, (b.0 + 80.0, b.1 - 90.0));
    let o = e.doc().objects.last().unwrap();
    assert_eq!((o.from, o.to), (1, 0));
    // a click without a drag draws nothing
    let n = e.doc().objects.len();
    gesture_click(&mut e, MOUSE, b);
    assert_eq!(e.doc().objects.len(), n);
}

#[test]
fn the_middle_handle_and_the_keys_bend_the_selected_curve() {
    let mut e = amine_editor();
    e.set_tool("lonepair");
    let n = atom_px(&e, 2);
    gesture_click(&mut e, MOUSE, n);
    e.set_tool("curve:full");
    let (from, to) = (pair_px(&e, 2), atom_px(&e, 1));
    gesture(&mut e, MOUSE, from, to);
    let id = e.doc().objects[0].id;
    let b0 = e.doc().objects[0].curve().bend;
    e.set_tool("select");
    // pick the curve by a click on its middle, then drag that handle to the other side of the chord
    let (p, c, q) = curve::shape(e.doc(), &e.doc().objects[0]);
    let m = curve::at(p, c, q, 0.5);
    let mp = e.view.to_px(m.0, m.1);
    gesture_click(&mut e, MOUSE, mp);
    assert_eq!(e.over.sel_objs, vec![id]);
    let other = e
        .view
        .to_px(2.0 * (p.0 + q.0) / 2.0 - m.0, 2.0 * (p.1 + q.1) / 2.0 - m.1);
    let f = gesture(&mut e, MOUSE, mp, other);
    assert_eq!(f & CHANGED, CHANGED);
    let b1 = e.doc().objects[0].curve().bend;
    assert!((b1 + b0).abs() < 0.05, "mirrored: {b0} -> {b1}");
    assert_eq!(e.over.sel_objs, vec![id], "the selection stays");
    // the keys: a tenth of the chord each, one undo step each
    let steps = e.session.doc.objects[0].curve().bend;
    e.key(".", 0);
    assert!((e.doc().objects[0].curve().bend - (steps + 0.1)).abs() < 1e-9);
    e.key(",", 0);
    e.key(",", 0);
    assert!((e.doc().objects[0].curve().bend - (steps - 0.1)).abs() < 1e-9);
    e.undo();
    assert!((e.doc().objects[0].curve().bend - (steps - 0.0)).abs() < 1e-9);
    // with nothing selected the keys do nothing
    e.key("Escape", 0);
    assert_eq!(e.key(".", 0) & CHANGED, 0);
}

#[test]
fn keys_pick_the_tools_and_place_the_lone_pairs() {
    let mut e = amine_editor();
    e.key("k", 0);
    assert_eq!(e.tool.name(), "curve:full");
    e.key("K", mzdraw::editor::SHIFT);
    assert_eq!(e.tool.name(), "curve:half");
    e.key("u", 0);
    assert_eq!(e.tool.name(), "lonepair");
    // Shift+U places them from the valence rules, in one step
    let f = e.key("U", mzdraw::editor::SHIFT);
    assert_eq!(f & CHANGED, CHANGED);
    assert_eq!(e.doc().atom(2).unwrap().lp(), 1);
    assert_eq!(e.doc().atom(1).unwrap().lp(), 0);
    assert_eq!(
        e.key("U", mzdraw::editor::SHIFT) & CHANGED,
        0,
        "nothing left to place"
    );
    e.undo();
    assert_eq!(e.doc().atom(2).unwrap().lp(), 0);
    // on a focused atom U cycles the pairs
    e.key("Home", 0);
    for k in ["arrowright", "arrowleft", "arrowup", "arrowdown"] {
        if e.over.focus != Some(2) {
            e.key(k, 0);
        }
    }
    assert_eq!(e.over.focus, Some(2));
    e.key("u", 0);
    assert_eq!(e.doc().atom(2).unwrap().lp(), 1);
}

#[test]
fn curves_and_pairs_are_painted_and_listed_as_lost_by_the_files_that_cannot_hold_them() {
    let s = amine();
    let svg = svg::to_svg(&s.doc);
    let plain = svg::to_svg(&doc("CN"));
    assert_ne!(svg, plain);
    let pairs = Document::from_json(&s.doc.to_json()).unwrap();
    assert_eq!(
        pairs, s.doc,
        "the document round trips with its pairs and its curve"
    );
    for f in ["molfile", "cdxml", "ket"] {
        let lost = mzdraw::loss::loss(&s.doc, f).unwrap();
        let what: Vec<&str> = lost.iter().map(|l| l.what).collect();
        assert!(
            what.contains(&"atom.lp") && what.contains(&"object.curve"),
            "{f}: {what:?}"
        );
    }
    let lost = mzdraw::loss::loss(&s.doc, "mzdraw").unwrap();
    assert!(lost.is_empty(), "{lost:?}");
}
