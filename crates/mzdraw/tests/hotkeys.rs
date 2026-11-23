//! Hover keys (arch-cd-03): a key acts on the atom or bond under the pointer (or in keyboard focus) without selecting it first, as one
//! undo step; over nothing the same keys pick their tools as ever; the table has no clash.

use mzdraw::editor::{ALT, CHANGED, FOCUS, SHIFT};
use mzdraw::hotkeys::{self, Cmd, Target, TABLE};
use mzdraw::Editor;

fn from(smiles: &str) -> Editor {
    let mut e = Editor::default();
    e.import(smiles).unwrap();
    e
}

fn saved(e: &Editor) -> String {
    e.doc().to_json()
}

/// One key over a bond or an atom: the document changes, and one undo gives back exactly what it was.
fn one_step(e: &mut Editor, key: &str, mods: u32, hover: (bool, u32)) -> (&'static str, String) {
    let before = saved(e);
    let f = e.key_over(key, mods, Some(hover), None);
    let said = e.said.clone();
    assert_ne!(f & CHANGED, 0, "{key} {mods}");
    assert_ne!(saved(e), before, "{key} {mods}");
    let after = saved(e);
    assert!(e.undo() > 0);
    assert_eq!(saved(e), before, "one undo step for {key}");
    assert!(e.redo() > 0);
    assert_eq!(saved(e), after);
    said
}

#[test]
fn the_digits_set_the_order_of_the_hovered_bond_in_one_step() {
    let mut e = from("CC");
    let b = e.doc().bonds[0].id;
    for (key, order) in [("2", 2), ("3", 3)] {
        let said = one_step(&mut e, key, 0, (false, b));
        assert_eq!(e.doc().bond(b).unwrap().o, order);
        assert_eq!(said, ("draw2.say.order", key.to_string()));
    }
    e.key_over("1", 0, Some((false, b)), None);
    assert_eq!(e.doc().bond(b).unwrap().o, 1);
    // the pointer stays where it was put by the page: the replay field takes it away again
    assert_eq!(e.over.hover_bond, None);
}

#[test]
fn the_wedge_keys_set_the_mark_and_the_same_key_turns_it_around() {
    let mut e = from("CC");
    let b = e.doc().bonds[0].id;
    let ends = |e: &Editor| {
        let x = e.doc().bond(b).unwrap();
        (x.a, x.b, x.s, x.o)
    };
    let (a0, b0, ..) = ends(&e);
    let said = one_step(&mut e, "w", 0, (false, b));
    assert_eq!(ends(&e), (a0, b0, 1, 1));
    assert_eq!(said.0, "draw2.say.wedge");
    // the second press: same mark, the narrow end on the other atom
    let said = one_step(&mut e, "w", 0, (false, b));
    assert_eq!(ends(&e), (b0, a0, 1, 1));
    assert_eq!(said.0, "draw2.say.wedgeTurn");
    // Shift+W hashed, Y wavy; another kind replaces the mark
    e.key_over("w", SHIFT, Some((false, b)), None);
    assert_eq!(ends(&e).2, 2);
    e.key_over("y", 0, Some((false, b)), None);
    assert_eq!(ends(&e).2, 3);
    e.key_over("y", 0, Some((false, b)), None);
    assert_eq!(ends(&e).2, 3, "a wavy bond is turned around, not changed");
}

#[test]
fn a_wedge_on_a_double_bond_makes_it_single_with_the_wedge_in_one_step() {
    let mut e = from("C=C");
    let b = e.doc().bonds[0].id;
    one_step(&mut e, "w", 0, (false, b));
    let x = e.doc().bond(b).unwrap();
    assert_eq!((x.o, x.s), (1, 1));
}

#[test]
fn the_group_keys_label_the_hovered_atom() {
    for (key, mods, label) in [
        ("m", 0, "Me"),
        ("e", 0, "Et"),
        ("f", SHIFT, "CF3"),
        ("o", SHIFT, "OH"),
    ] {
        let mut e = from("CC");
        let a = e.doc().atoms[0].id;
        let said = one_step(&mut e, key, mods, (true, a));
        assert_eq!(e.doc().atom(a).unwrap().lab, label);
        assert_eq!(said, ("draw2.say.label", label.to_string()));
    }
}

#[test]
fn alt_digit_sprouts_a_chain_and_shift_digit_fuses_a_ring() {
    let mut e = from("CC");
    let a = e.doc().atoms[0].id;
    for n in 1..=9usize {
        let mut c = from("CC");
        one_step(&mut c, &n.to_string(), ALT, (true, a));
        assert_eq!(c.doc().atoms.len(), 2 + n, "chain of {n}");
        assert_eq!(c.doc().bonds.len(), 1 + n);
    }
    for n in 3..=8usize {
        let mut r = from("CC");
        let said = one_step(&mut r, &n.to_string(), SHIFT, (true, a));
        assert_eq!(r.doc().atoms.len(), 1 + n, "ring of {n} on the atom");
        assert_eq!(r.doc().bonds.len(), 1 + n);
        assert_eq!(said, ("draw2.say.ring", n.to_string()));
    }
    // benzene: a corner has one valence left, a ring needs two
    let mut b = from("c1ccccc1");
    let corner = b.doc().atoms[0].id;
    refused(&mut b, "6", SHIFT, (true, corner), "error.draw2.hotValence");
    // a ring outside 3 to 8 is no key
    e.key_over("2", SHIFT, Some((true, a)), None);
    assert_eq!(e.doc().atoms.len(), 2);
}

#[test]
fn over_nothing_the_keys_keep_picking_their_tools() {
    for (key, mods, tool) in [
        ("1", 0, "bond:1"),
        ("2", 0, "bond:2"),
        ("3", 0, "bond:3"),
        ("w", 0, "bond:up"),
        ("w", SHIFT, "bond:down"),
        ("y", 0, "bond:wavy"),
        ("e", 0, "erase"),
        ("f", SHIFT, "atom:F"),
        ("o", SHIFT, "atom:O"),
    ] {
        let mut e = from("CC");
        let f = e.key_over(key, mods, None, None);
        assert_eq!(e.tool.name(), tool, "{key}");
        assert_eq!(f & CHANGED, 0, "{key} changes no drawing");
        assert_eq!(saved(&e), saved(&from("CC")), "{key}");
    }
    // M still toggles bringing a selection to a click; a digit with Alt or Shift over nothing does nothing
    let mut e = from("CC");
    e.key("m", 0);
    assert!(e.move_here);
    let before = saved(&e);
    assert_eq!(e.key("3", ALT) & CHANGED, 0);
    assert_eq!(e.key("6", SHIFT) & CHANGED, 0);
    assert_eq!(saved(&e), before);
}

#[test]
fn the_pointer_wins_over_the_focus() {
    let mut e = from("CCC");
    let (b1, b2) = (e.doc().bonds[0].id, e.doc().bonds[1].id);
    let (a1, a2) = (e.doc().atoms[0].id, e.doc().atoms[1].id);
    // focus walked a1 -> a2 is the bond b1; the pointer over b2 takes the key
    e.key_over("2", 0, Some((false, b2)), Some((a2, Some(a1))));
    assert_eq!(e.doc().bond(b2).unwrap().o, 2);
    assert_eq!(e.doc().bond(b1).unwrap().o, 1);
}

#[test]
fn the_keyboard_route_acts_on_the_walked_bond_and_the_focused_atom() {
    let mut e = from("CCC");
    let (b1, a1, a2, a3) = (
        e.doc().bonds[0].id,
        e.doc().atoms[0].id,
        e.doc().atoms[1].id,
        e.doc().atoms[2].id,
    );
    let f = e.key_over("2", 0, None, Some((a2, Some(a1))));
    assert_ne!(f & CHANGED, 0);
    assert_eq!(e.doc().bond(b1).unwrap().o, 2);
    // Alt+M, Alt+Shift+F on the focused atom (the plain letters keep their tools with the focus: E erases)
    e.key_over("m", ALT, None, Some((a3, None)));
    assert_eq!(e.doc().atom(a3).unwrap().lab, "Me");
    let f = e.key_over("f", ALT | SHIFT, None, Some((a3, None)));
    assert_ne!(f & FOCUS, 0, "the page says where the focus is");
    assert_eq!(e.doc().atom(a3).unwrap().lab, "CF3");
    e.key("e", 0);
    assert_eq!(e.tool.name(), "erase");
    assert_eq!(e.doc().atom(a3).unwrap().lab, "CF3");
}

#[test]
fn the_table_has_no_clash_and_every_row_is_valid() {
    let reserved = [
        "arrowright",
        "arrowleft",
        "arrowup",
        "arrowdown",
        "home",
        "pageup",
        "pagedown",
        "enter",
        " ",
        "+",
        "-",
        "u",
        "delete",
        "backspace",
        "escape",
        "v",
        "k",
        "l",
        "t",
        "a",
        "d",
        "\\",
        ",",
        ".",
        "c",
        "n",
        "s",
        "p",
        "h",
        "b",
        "i",
    ];
    for (i, h) in TABLE.iter().enumerate() {
        assert!(
            TABLE[..i]
                .iter()
                .all(|p| (p.target, p.key, p.shift, p.alt) != (h.target, h.key, h.shift, h.alt)),
            "{} twice",
            h.key
        );
        assert!(
            !reserved.contains(&h.key),
            "{} is taken by another binding",
            h.key
        );
        match h.cmd {
            Cmd::Group(l) => assert!(mzdraw::label::kind(l).is_some(), "{l}"),
            Cmd::Chain(n) => assert!((1..=9).contains(&n) && h.alt && h.target == Target::Atom),
            Cmd::Ring(n) => assert!((3..=8).contains(&n) && h.shift && !h.alt),
            Cmd::Order(n) => assert!((1..=3).contains(&n) && h.target == Target::Bond),
            Cmd::Wedge(n) => assert!((1..=3).contains(&n) && h.target == Target::Bond),
        }
        // a plain letter or Shift letter that picks a tool with the focus is pointer-only; its Alt twin takes the focus
        if matches!(h.cmd, Cmd::Group(_)) {
            assert_eq!(h.focus, h.alt);
            assert!(h.alt || hotkeys::find(Target::Atom, h.key, h.shift, true).is_some());
        }
    }
    // the table for the help is valid JSON with one row each
    let v: serde_json::Value = serde_json::from_str(&hotkeys::help_json()).unwrap();
    assert_eq!(v.as_array().unwrap().len(), TABLE.len());
}

#[test]
fn with_a_drawing_tool_active_the_keys_that_picked_tools_still_pick_them() {
    let mut e = from("CC");
    let b = e.doc().bonds[0].id;
    e.key("w", 0);
    assert_eq!(e.tool.name(), "bond:up");
    let before = saved(&e);
    for (key, mods) in [("w", 0), ("w", SHIFT), ("y", 0), ("2", 0)] {
        e.key_over(key, mods, Some((false, b)), None);
        assert_eq!(
            saved(&e),
            before,
            "{key} picks a tool, the pointer is over a bond"
        );
    }
    assert_eq!(e.tool.name(), "bond:2");
    // the Alt keys and the rings are new keys: they act with any tool
    let a = e.doc().atoms[0].id;
    e.key_over("3", ALT, Some((true, a)), None);
    assert_eq!(e.doc().atoms.len(), 5);
}

fn degree(e: &Editor, n: usize) -> u32 {
    let d = e.doc();
    d.atoms
        .iter()
        .find(|a| d.neighbours(a.id).len() == n)
        .map(|a| a.id)
        .unwrap()
}

fn set_kind(e: &mut Editor, bond: u32, kind: &str) {
    let op = mzdraw::ops::Op::BondSet {
        id: bond,
        o: None,
        s: None,
        pos: None,
        k: Some(kind.into()),
        rev: false,
    };
    e.session.run(&[op]).unwrap();
}

/// The key leaves the drawing and the history as they were; `error` or the say line tells why.
fn refused(e: &mut Editor, key: &str, mods: u32, hover: (bool, u32), error: &'static str) {
    let before = (saved(e), e.session.log_json());
    let f = e.key_over(key, mods, Some(hover), None);
    assert_eq!(f & CHANGED, 0, "{key}");
    assert_eq!(
        (saved(e), e.session.log_json()),
        before,
        "{key} changes nothing"
    );
    assert_eq!(e.error, error, "{key}");
}

#[test]
fn a_key_that_changes_nothing_takes_no_undo_step() {
    let mut e = from("CCC");
    let (b, a) = (e.doc().bonds[0].id, degree(&e, 1));
    // an edit that is real, then keys that change nothing: the undo after them undoes the real edit
    e.key_over("2", 0, Some((false, b)), None);
    let real = saved(&e);
    for (key, mods, hover) in [("2", 0, (false, b)), ("1", 0, (false, e.doc().bonds[1].id))] {
        let log = e.session.log_json();
        let f = e.key_over(key, mods, Some(hover), None);
        assert_eq!(f & CHANGED, 0, "{key} changes nothing");
        assert_eq!(e.said.0, "draw2.say.hotNoChange");
        assert_eq!((saved(&e), e.session.log_json()), (real.clone(), log));
    }
    e.key_over("m", 0, Some((true, a)), None);
    let log = e.session.log_json();
    assert_eq!(
        e.key_over("m", 0, Some((true, a)), None) & CHANGED,
        0,
        "Me twice"
    );
    assert_eq!(e.session.log_json(), log);
    assert!(e.undo() > 0);
    assert_eq!(
        saved(&e),
        real,
        "the undo takes back the label, not a phantom step"
    );
    assert!(e.undo() > 0);
    assert_ne!(saved(&e), real, "and the next one the order");
}

#[test]
fn the_pointer_over_an_atom_does_not_send_a_digit_to_a_walked_bond_far_away() {
    let mut e = from("CCC");
    let (a1, a2, a3) = (
        e.doc().atoms[0].id,
        e.doc().atoms[1].id,
        e.doc().atoms[2].id,
    );
    let before = saved(&e);
    // the focus walked a1 -> a2 (the bond between them), the pointer rests on a3
    e.key_over("2", 0, Some((true, a3)), Some((a2, Some(a1))));
    assert_eq!(saved(&e), before, "the walked bond is not edited from afar");
    assert_eq!(
        e.tool.name(),
        "bond:2",
        "the digit picks its tool as everywhere"
    );
    // an atom row does not use the focus when the pointer is over a bond
    let mut e = from("CCC");
    let (b, a1, a3) = (
        e.doc().bonds[0].id,
        e.doc().atoms[0].id,
        e.doc().atoms[2].id,
    );
    e.key_over("3", ALT, Some((false, b)), Some((a3, Some(a1))));
    assert_eq!(
        e.doc().atoms.len(),
        3,
        "the chain is not sprouted from the focused atom"
    );
}

#[test]
fn a_group_label_is_refused_where_it_would_lose_something() {
    // not a terminal atom
    let mut e = from("CCC");
    let mid = degree(&e, 2);
    refused(&mut e, "m", 0, (true, mid), "error.draw2.hotGroup");
    // charged, an isotope, a radical, not carbon
    for (what, op) in [
        (
            "charge",
            mzdraw::ops::Op::AtomSet {
                id: 0,
                el: None,
                chg: Some(1),
                iso: None,
                rad: None,
                h: None,
                lab: None,
                map: None,
                lp: None,
            },
        ),
        (
            "isotope",
            mzdraw::ops::Op::AtomSet {
                id: 0,
                el: None,
                chg: None,
                iso: Some(13),
                rad: None,
                h: None,
                lab: None,
                map: None,
                lp: None,
            },
        ),
        (
            "element",
            mzdraw::ops::Op::AtomSet {
                id: 0,
                el: Some("N".into()),
                chg: None,
                iso: None,
                rad: None,
                h: None,
                lab: None,
                map: None,
                lp: None,
            },
        ),
    ] {
        let mut e = from("CCC");
        let end = degree(&e, 1);
        let mut op = op;
        if let mzdraw::ops::Op::AtomSet { id, .. } = &mut op {
            *id = end;
        }
        e.session.run(&[op]).unwrap();
        for (key, mods) in [("o", SHIFT), ("f", SHIFT), ("e", 0)] {
            refused(&mut e, key, mods, (true, end), "error.draw2.hotAtom");
        }
        let _ = what;
    }
}

#[test]
fn a_chain_or_a_ring_is_refused_on_an_atom_with_no_free_valence() {
    let mut e = from("CC(C)(C)C");
    let quaternary = degree(&e, 4);
    refused(
        &mut e,
        "1",
        ALT,
        (true, quaternary),
        "error.draw2.hotValence",
    );
    refused(
        &mut e,
        "6",
        SHIFT,
        (true, quaternary),
        "error.draw2.hotValence",
    );
    // one valence left: a chain goes, a ring (two bonds on the atom) does not
    let mut e = from("CC(C)C");
    let branch = degree(&e, 3);
    refused(&mut e, "5", SHIFT, (true, branch), "error.draw2.hotValence");
    let n = e.doc().atoms.len();
    e.key_over("2", ALT, Some((true, branch)), None);
    assert_eq!(e.doc().atoms.len(), n + 2);
    // an end atom has room for both, a labelled one (a group) none
    let mut e = from("CC");
    let end = e.doc().atoms[0].id;
    e.key_over("m", 0, Some((true, end)), None);
    refused(&mut e, "1", ALT, (true, end), "error.draw2.hotValence");
    let mut e = from("CC");
    e.key_over("5", SHIFT, Some((true, end)), None);
    assert_eq!(e.doc().atoms.len(), 6);
}

#[test]
fn a_key_never_drops_the_kind_of_a_bond_silently() {
    for kind in ["dative", "hbond"] {
        let mut e = from("CC");
        let b = e.doc().bonds[0].id;
        set_kind(&mut e, b, kind);
        for (key, mods) in [("2", 0), ("3", 0), ("w", 0), ("w", SHIFT), ("y", 0)] {
            refused(&mut e, key, mods, (false, b), "error.draw2.hotKind");
        }
        // order 1 is what the bond is: nothing to change, and the kind stays
        assert_eq!(e.key_over("1", 0, Some((false, b)), None) & CHANGED, 0);
        assert_eq!(e.doc().bond(b).unwrap().kind(), kind);
    }
    // a bold or dashed line keeps its kind through an order key, and a wedge key refuses (a mark would replace the kind)
    let mut e = from("CC");
    let b = e.doc().bonds[0].id;
    set_kind(&mut e, b, "bold");
    refused(&mut e, "w", 0, (false, b), "error.draw2.hotKind");
    e.key_over("2", 0, Some((false, b)), None);
    assert_eq!(e.doc().bond(b).unwrap().kind(), "bold");
}
