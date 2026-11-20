//! Wedge marks as the drawing and the stereo of the record derived from them, rotate and flip, and the clipboard, driven as the page
//! drives them.

use mzdraw::editor::{CHANGED, CTRL, HANDLED, MOUSE, SHIFT, TOOL};
use mzdraw::scene::{LINE, TRI};
use mzdraw::{svg, Axis, Document, Editor};
use mzlab_mol::{Mol, Stereo};

fn click(e: &mut Editor, x: f64, y: f64) -> u32 {
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_up(x, y)
}

fn px(e: &Editor, id: u32) -> (f64, f64) {
    let a = e.doc().atom(id).unwrap();
    e.view.to_px(a.x, a.y)
}

fn mid(e: &Editor, bond: u32) -> (f64, f64) {
    let b = e.doc().bond(bond).unwrap();
    let (p, q) = (px(e, b.a), px(e, b.b));
    ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0)
}

/// The parity of the one tetrahedral centre of a record, as text.
fn centre(m: &Mol) -> String {
    let t: Vec<String> = m
        .stereo
        .iter()
        .filter(|s| matches!(s, Stereo::Tetra { .. }))
        .map(|s| serde_json::to_string(s).unwrap())
        .collect();
    assert_eq!(t.len(), 1, "{t:?}");
    t[0].clone()
}

/// Alanine without stereo, drawn by `mzlab-mol` placeholders: atom 2 is the centre.
fn alanine() -> Editor {
    let mut e = Editor::default();
    e.import("CC(N)C(=O)O").unwrap();
    e.fit(800.0, 600.0);
    e
}

#[test]
fn a_wedge_drawn_on_a_bond_is_a_defined_centre_of_the_record() {
    let mut e = alanine();
    assert!(e.doc().to_mol_stereo().stereo.is_empty());
    let cn = e.doc().bond_between(2, 3).unwrap().id;
    assert_eq!(e.set_tool("bond:up") & TOOL, TOOL);
    let p = mid(&e, cn);
    assert_eq!(click(&mut e, p.0, p.1) & CHANGED, CHANGED);
    let b = e.doc().bond(cn).unwrap().clone();
    assert_eq!((b.o, b.s), (1, 1));
    // the narrow end must be the centre for the mark to mean something: a second click turns the wedge around
    if b.a != 2 {
        click(&mut e, p.0, p.1);
    }
    assert_eq!(e.doc().bond(cn).unwrap().a, 2);
    let up = centre(&e.doc().to_mol_stereo());
    assert!(up.contains(r#""status":"defined""#), "{up}");
    // the molfile carries the mark on its bond line, narrow end first
    let line = format!("{:>3}{:>3}  1  1", 2, 3);
    assert!(
        e.doc().to_molfile().contains(&line),
        "{}",
        e.doc().to_molfile()
    );
    // the hashed wedge on the same bond is the other parity
    e.set_tool("bond:down");
    click(&mut e, p.0, p.1);
    assert_eq!(e.doc().bond(cn).unwrap().s, 2);
    let down = centre(&e.doc().to_mol_stereo());
    assert_ne!(up, down);
    // a wavy mark is an unknown centre
    e.set_tool("bond:wavy");
    click(&mut e, p.0, p.1);
    assert!(centre(&e.doc().to_mol_stereo()).contains(r#""status":"unknown""#));
    // the plain single-bond tool takes the mark away, then walks the orders
    e.set_tool("bond:1");
    click(&mut e, p.0, p.1);
    assert_eq!(
        (e.doc().bond(cn).unwrap().o, e.doc().bond(cn).unwrap().s),
        (1, 0)
    );
    click(&mut e, p.0, p.1);
    assert_eq!(e.doc().bond(cn).unwrap().o, 2);
    for _ in 0..6 {
        e.undo();
    }
    assert!(e.doc().bonds.iter().all(|b| b.s == 0 && b.a < b.b));
}

#[test]
fn stereo_of_a_smiles_or_a_molfile_arrives_as_wedges_and_goes_back_out_the_same() {
    for smiles in [
        "C[C@H](N)C(=O)O",
        "C[C@@H](N)C(=O)O",
        "N[C@@](F)(Cl)Br",
        "C[C@H]1CC[C@@H](O)CC1",
    ] {
        let rec = Mol::from_smiles(smiles).unwrap();
        let mut e = Editor::default();
        e.import(smiles).unwrap();
        let marks = e.doc().bonds.iter().filter(|b| b.s != 0).count();
        assert!(marks >= 1, "{smiles}: the centre is drawn with a wedge");
        // the drawing means the same stereo as the source, by the record's own rules
        assert_eq!(e.doc().to_mol_stereo().stereo, rec.stereo, "{smiles}");
        // molfile out and in again: the same stereo and the same marks
        let mol = e.doc().to_molfile();
        let mut again = Editor::default();
        again.import(&mol).unwrap();
        assert_eq!(
            again.doc().to_mol_stereo().stereo,
            rec.stereo,
            "{smiles} through a molfile"
        );
        // the document itself round-trips with its marks
        let json = e.doc().to_json();
        assert_eq!(Document::from_json(&json).unwrap().to_json(), json);
    }
}

#[test]
fn rotating_keeps_the_stereo_and_flipping_inverts_it() {
    let mut e = Editor::default();
    e.import("C[C@H](N)C(=O)O").unwrap();
    let before = centre(&e.doc().to_mol_stereo());
    assert_eq!(e.rotate(30.0) & CHANGED, CHANGED);
    // what the page announces: the angle as a person writes it
    assert_eq!(e.said, ("draw2.say.rotated", "30".to_string()));
    e.rotate(-22.5);
    assert_eq!(e.said.1, "-22.5");
    e.undo();
    assert_eq!(centre(&e.doc().to_mol_stereo()), before);
    assert_eq!(e.flip(Axis::H) & CHANGED, CHANGED);
    assert_ne!(
        centre(&e.doc().to_mol_stereo()),
        before,
        "a mirror image is the other enantiomer"
    );
    e.flip(Axis::V);
    assert_eq!(
        centre(&e.doc().to_mol_stereo()),
        before,
        "two mirrors are a rotation"
    );
    // with a selection only the selection turns
    let json = e.doc().to_json();
    e.set_tool("select");
    let p = px(&e, 1);
    click(&mut e, p.0, p.1);
    e.rotate(90.0);
    let moved: Vec<u32> = Document::from_json(&json)
        .unwrap()
        .atoms
        .iter()
        .zip(&e.doc().atoms)
        .filter(|(a, b)| a != b)
        .map(|(a, _)| a.id)
        .collect();
    assert!(
        moved.is_empty(),
        "one atom turns around itself: nothing moves"
    );
    e.select_all();
    e.rotate(90.0);
    assert_ne!(e.doc().to_json(), json);
    e.undo();
    assert_eq!(e.doc().to_json(), json);
    // nothing to turn on an empty canvas
    assert_eq!(Editor::default().rotate(15.0), 0);
    assert_eq!(Editor::default().flip(Axis::V), 0);
}

#[test]
fn wedges_are_painted_as_a_triangle_a_ladder_and_a_wave() {
    let mut e = Editor::default();
    e.set_tool("bond:up");
    click(&mut e, 200.0, 200.0);
    let tris = |e: &Editor| e.scene().data.iter().filter(|&&v| v == TRI).count();
    let lines = |e: &Editor| {
        let svg = svg::to_svg(e.doc());
        (
            svg.matches("<line").count(),
            svg.matches("<polygon").count(),
        )
    };
    assert_eq!(e.doc().bonds[0].s, 1);
    assert!(tris(&e) >= 1);
    assert_eq!(lines(&e), (0, 1));
    e.set_tool("bond:down");
    click(&mut e, 220.0, 190.0);
    let (ladder, poly) = lines(&e);
    assert!((3..=12).contains(&ladder) && poly == 0, "{ladder} rungs");
    e.set_tool("bond:wavy");
    click(&mut e, 220.0, 190.0);
    assert_eq!(lines(&e), (32, 0));
    // a wedge dragged from an atom starts narrow at that atom
    let p = px(&e, 2);
    e.set_tool("bond:up");
    e.pointer_down(p.0, p.1, MOUSE, 0);
    e.pointer_move(p.0 + 30.0, p.1 + 10.0, MOUSE);
    e.pointer_up(p.0 + 40.0, p.1);
    let b = e.doc().bonds.last().unwrap();
    assert_eq!((b.a, b.s), (2, 1));
    assert_eq!(LINE, 1.0);
}

#[test]
fn copy_cut_paste_and_duplicate() {
    let mut e = Editor::default();
    e.import("C[C@H](N)C(=O)O").unwrap();
    let rec = e.doc().to_mol_stereo();
    assert!(!e.copy(), "nothing selected, nothing copied");
    assert_eq!(e.clipboard("mzdraw"), "");
    assert_eq!(e.paste(None), 0);
    e.select_all();
    assert!(e.copy());
    let clip = Document::from_json(&e.clipboard("mzdraw")).unwrap();
    assert_eq!((clip.atoms.len(), clip.bonds.len()), (6, 5));
    assert_eq!(
        Mol::from_molfile(&e.clipboard("molfile")).unwrap().stereo,
        rec.stereo
    );
    let before = e.doc().to_json();
    // a paste is one undo step, lands half a bond away, becomes the selection and picks the Select tool
    e.set_tool("bond:1");
    let f = e.paste(None);
    assert_eq!(f & (CHANGED | TOOL), CHANGED | TOOL);
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (12, 10));
    assert_eq!(e.tool.name(), "select");
    assert_eq!((e.over.sel_atoms.len(), e.over.sel_bonds.len()), (6, 5));
    let (a, b) = (e.doc().atoms[0].clone(), e.doc().atoms[6].clone());
    assert_eq!((b.x - a.x, b.y - a.y), (0.5, -0.5));
    assert_eq!(
        e.doc().bonds.iter().filter(|b| b.s != 0).count(),
        2,
        "the copy keeps its wedge"
    );
    // the second paste lands a further half bond away
    e.paste(None);
    assert_eq!(e.doc().atoms[12].x - a.x, 1.0);
    e.undo();
    e.undo();
    assert_eq!(e.doc().to_json(), before);
    // the same text coming back from the system clipboard is still our own copy (the paste count is not rewound by undo); a
    // foreign text lands to the right
    e.paste(Some(&e.clipboard("mzdraw")));
    assert_eq!(e.doc().atoms[6].x - a.x, 1.5);
    e.undo();
    let right = e.doc().atoms.iter().map(|a| a.x).fold(f64::MIN, f64::max);
    e.paste(Some("CCO"));
    assert_eq!(e.doc().atoms.len(), 9);
    assert!(e.doc().atoms[6..].iter().all(|a| a.x >= right + 1.9));
    assert_eq!(e.over.sel_atoms.len(), 3);
    // a text that is no structure changes nothing and says why
    let n = e.doc().to_json();
    e.paste(Some("not a structure at all ((("));
    assert_eq!(e.doc().to_json(), n);
    assert_eq!(e.error, "error.draw2.smiles");
    // cut takes the selection away in one step; Ctrl+D duplicates
    assert_eq!(e.cut() & CHANGED, CHANGED);
    assert_eq!(e.doc().atoms.len(), 6);
    e.undo();
    assert_eq!(e.doc().atoms.len(), 9);
    e.select_all();
    assert_eq!(e.key("d", CTRL) & (CHANGED | HANDLED), CHANGED | HANDLED);
    assert_eq!(e.doc().atoms.len(), 18);
    // copy, cut and paste keys are left to the page (clipboard events)
    for k in ["c", "x", "v"] {
        assert_eq!(e.key(k, CTRL), 0);
    }
    // only bonds with both ends selected travel
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    e.over.sel_atoms = vec![1, 3];
    assert!(e.copy());
    let clip = Document::from_json(&e.clipboard("mzdraw")).unwrap();
    assert_eq!((clip.atoms.len(), clip.bonds.len(), clip.next), (2, 0, 3));
}

#[test]
fn keys_pick_the_wedge_tools() {
    let mut e = Editor::default();
    for (k, mods, tool) in [
        ("w", 0, "bond:up"),
        ("W", SHIFT, "bond:down"),
        ("y", 0, "bond:wavy"),
        ("1", 0, "bond:1"),
    ] {
        assert_eq!(e.key(k, mods) & (TOOL | HANDLED), TOOL | HANDLED);
        assert_eq!(e.tool.name(), tool);
    }
}
