//! Drawing with the keyboard alone: the focus walks along the bonds, Enter grows bonds, letters and digits set what the focus is
//! on; and the drawing in words (the outline a screen reader reads).

use mzdraw::editor::{CHANGED, FOCUS, HANDLED, SHIFT};
use mzdraw::outline::{focus, pieces};
use mzdraw::scene::{colour, RING};
use mzdraw::Editor;
use mzlab_mol::Mol;

fn key(e: &mut Editor, k: &str) -> u32 {
    e.key(k, 0)
}

fn rings(e: &Editor) -> usize {
    let d = e.scene().data;
    let (mut i, mut n) = (0, 0);
    while i < d.len() {
        let op = d[i];
        if op == RING && d[i + 4] == colour::FOCUS {
            n += 1;
        }
        i += if op == 1.0 || op == 2.0 {
            7
        } else if op == 3.0 {
            6
        } else if op == 6.0 {
            8
        } else {
            5
        };
    }
    n
}

#[test]
fn a_molecule_is_drawn_with_the_keyboard_alone() {
    let mut e = Editor::default();
    assert_eq!(e.over.focus, None);
    assert_eq!(rings(&e), 0);
    // Enter on an empty canvas places a carbon and focuses it; each Enter grows the chain and moves the focus on
    assert_eq!(
        key(&mut e, "Enter") & (CHANGED | FOCUS | HANDLED),
        CHANGED | FOCUS | HANDLED
    );
    assert_eq!((e.doc().atoms.len(), e.over.focus), (1, Some(1)));
    assert_eq!(rings(&e), 2, "the focus is drawn as two rings");
    key(&mut e, "Enter");
    key(&mut e, "Enter");
    assert_eq!(
        (e.doc().atoms.len(), e.doc().bonds.len(), e.over.focus),
        (3, 2, Some(4))
    );
    // the letter sets the element of the focused atom; the digit sets the order of the bond just walked
    assert_eq!(key(&mut e, "o") & (CHANGED | FOCUS), CHANGED | FOCUS);
    assert_eq!(e.doc().atom(4).unwrap().el, "O");
    assert_eq!(e.tool.name(), "bond:1", "no tool was picked");
    key(&mut e, "2");
    assert_eq!(e.doc().bond_between(2, 4).unwrap().o, 2);
    // walk back to the middle carbon, grow a branch with Shift+Enter (a double bond), make it single, make it N with a charge
    let back = e.doc().atom(2).unwrap().clone();
    let here = e.doc().atom(4).unwrap().clone();
    let towards = if back.x < here.x {
        "ArrowLeft"
    } else {
        "ArrowRight"
    };
    assert_eq!(key(&mut e, towards) & FOCUS, FOCUS);
    assert_eq!(e.over.focus, Some(2));
    e.key("Enter", SHIFT);
    assert_eq!(e.doc().bonds.last().unwrap().o, 2);
    key(&mut e, "1");
    key(&mut e, "n");
    key(&mut e, "+");
    let n = e.doc().atoms.last().unwrap().clone();
    assert_eq!(
        (n.el.as_str(), n.chg, e.doc().bonds.last().unwrap().o),
        ("N", 1, 1)
    );
    key(&mut e, "-");
    // acetamide by keyboard: C2H5NO
    let m = Mol::from_molfile(&e.doc().to_molfile()).unwrap();
    assert_eq!(pieces(e.doc())[0].formula, "C2H5NO");
    assert_eq!(m.atoms.len(), 4);
    // every key press was one undo step
    let steps = e.session.steps();
    // Enter x3, o, 2, Shift+Enter, 1, n, +, - : ten changes (the arrow key that moved the focus is not one)
    assert_eq!(steps, 10);
    for _ in 0..steps {
        e.undo();
    }
    assert!(e.doc().is_empty());
    assert_eq!(
        e.over.focus, None,
        "the focus does not point at an atom that is gone"
    );
    assert_eq!(rings(&e), 0);
}

#[test]
fn the_focus_walks_along_bonds_between_pieces_and_leaves() {
    let mut e = Editor::default();
    // a cross: atom 1 in the middle, 2 right, 3 up, 4 left, 5 down; and a second piece
    e.import(r#"{"format":"mzdraw/1","next":12,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0},{"id":2,"el":"N","x":1.0,"y":0.0},{"id":3,"el":"O","x":0.0,"y":1.0},{"id":4,"el":"S","x":-1.0,"y":0.0},{"id":5,"el":"F","x":0.0,"y":-1.0},{"id":10,"el":"Cl","x":5.0,"y":0.0}],"bonds":[{"id":6,"a":1,"b":2,"o":1},{"id":7,"a":1,"b":3,"o":1},{"id":8,"a":1,"b":4,"o":1},{"id":9,"a":1,"b":5,"o":1}]}"#)
        .unwrap();
    let el = |e: &Editor| e.over.focus.map(|a| e.doc().atom(a).unwrap().el.clone());
    // the first arrow lands on the first atom
    assert_eq!(
        key(&mut e, "ArrowRight") & (FOCUS | HANDLED),
        FOCUS | HANDLED
    );
    assert_eq!(el(&e).as_deref(), Some("C"));
    for (out, back, want) in [
        ("ArrowRight", "ArrowLeft", "N"),
        ("ArrowUp", "ArrowDown", "O"),
        ("ArrowLeft", "ArrowRight", "S"),
        ("ArrowDown", "ArrowUp", "F"),
    ] {
        key(&mut e, out);
        assert_eq!(el(&e).as_deref(), Some(want), "{out}");
        // no bond in that direction: the focus stays
        key(&mut e, out);
        assert_eq!(el(&e).as_deref(), Some(want), "{out} again");
        key(&mut e, back);
        assert_eq!(el(&e).as_deref(), Some("C"), "{back}");
    }
    // Page Down and Page Up go round the pieces; Home goes to the first atom
    key(&mut e, "PageDown");
    assert_eq!(el(&e).as_deref(), Some("Cl"));
    key(&mut e, "PageDown");
    assert_eq!(el(&e).as_deref(), Some("C"));
    key(&mut e, "PageUp");
    assert_eq!(el(&e).as_deref(), Some("Cl"));
    key(&mut e, "Home");
    assert_eq!(el(&e).as_deref(), Some("C"));
    // what the page says about the focus
    let f = focus(e.doc(), 1).unwrap();
    assert_eq!(
        (
            f.name.as_str(),
            f.index,
            f.of,
            f.bonds,
            f.hydrogens,
            f.charge
        ),
        ("C", 1, 6, 4, 0, 0)
    );
    // Space selects the focused atom, so the clipboard keys work without a pointer; Space again takes it out
    key(&mut e, " ");
    assert_eq!(e.over.sel_atoms, vec![1]);
    key(&mut e, " ");
    assert!(e.over.sel_atoms.is_empty());
    // Delete removes the focused atom and the focus goes back along the bond it came by
    key(&mut e, "ArrowRight");
    assert_eq!(key(&mut e, "Delete") & CHANGED, CHANGED);
    assert_eq!((e.doc().atoms.len(), el(&e).as_deref()), (5, Some("C")));
    // a digit with no bond walked, and keys the focus has no use for, keep their old meaning
    key(&mut e, "Home");
    key(&mut e, "3");
    assert_eq!(e.tool.name(), "bond:3");
    key(&mut e, "e");
    assert_eq!(e.tool.name(), "erase");
    // Escape leaves the focus; Tab is never taken (it must leave the canvas)
    assert_eq!(
        key(&mut e, "Escape") & FOCUS,
        FOCUS,
        "leaving the focus is said"
    );
    assert_eq!(e.over.focus, None);
    assert_eq!(
        key(&mut e, "Escape") & FOCUS,
        0,
        "with no focus there is nothing to say"
    );
    assert_eq!(e.key("Tab", 0), 0);
    assert_eq!(e.key("Tab", SHIFT), 0);
    // with a selection, Delete removes the selection, not the focus
    key(&mut e, "Home");
    e.over.sel_atoms = vec![10];
    key(&mut e, "Delete");
    assert_eq!((e.doc().atoms.len(), el(&e).as_deref()), (4, Some("C")));
}

#[test]
fn the_outline_says_what_is_drawn() {
    let mut e = Editor::default();
    assert!(pieces(e.doc()).is_empty());
    for smiles in ["CC(=O)O", "c1ccccc1", "[NH4+]", "CC(=O)[O-]"] {
        e.import(smiles).unwrap();
    }
    let got: Vec<(String, usize, usize)> = pieces(e.doc())
        .into_iter()
        .map(|p| (p.formula, p.atoms, p.bonds))
        .collect();
    assert_eq!(
        got,
        [
            ("C2H4O2".to_string(), 4, 3),
            ("C6H6".to_string(), 6, 6),
            ("H4N+".to_string(), 1, 0),
            ("C2H3O2\u{2212}".to_string(), 4, 3),
        ]
    );
    // an abbreviation counts as its atoms, an R group as R
    let mut e = Editor::default();
    e.import("CC").unwrap();
    e.set_tool("label:Ph");
    let a = e.doc().atom(1).unwrap().clone();
    let p = e.view.to_px(a.x, a.y);
    e.pointer_down(p.0, p.1, 0, 0);
    e.pointer_up(p.0, p.1);
    assert_eq!(pieces(e.doc())[0].formula, "C7H8");
    assert_eq!(pieces(e.doc())[0].atoms, 7);
    e.set_tool("label:R1");
    e.pointer_down(p.0, p.1, 0, 0);
    e.pointer_up(p.0, p.1);
    assert_eq!(pieces(e.doc())[0].formula, "CH3R");
    let f = focus(e.doc(), 1).unwrap();
    assert_eq!((f.name.as_str(), f.hydrogens), ("R1", 0));
    assert!(focus(e.doc(), 99).is_none());
}
