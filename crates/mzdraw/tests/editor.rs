//! The tools driven as the page drives them: pointer and key events in pixels, the document and the display list out.

use mzdraw::editor::{CHANGED, CTRL, HANDLED, MOUSE, PEN, SHIFT, TOOL, TOUCH};
use mzdraw::scene::{DISC, LINE, RING, TEXT, TRI};
use mzdraw::{svg, Editor};

fn click(e: &mut Editor, x: f64, y: f64) -> u32 {
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_up(x, y)
}

fn drag(e: &mut Editor, from: (f64, f64), to: (f64, f64)) -> u32 {
    e.pointer_down(from.0, from.1, MOUSE, 0);
    e.pointer_move((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0, MOUSE);
    e.pointer_move(to.0, to.1, MOUSE);
    e.pointer_up(to.0, to.1)
}

fn px(e: &Editor, id: u32) -> (f64, f64) {
    let a = e.doc().atom(id).unwrap();
    e.view.to_px(a.x, a.y)
}

fn count(e: &Editor, op: f32) -> usize {
    let d = e.scene().data;
    let mut i = 0;
    let mut n = 0;
    while i < d.len() {
        n += (d[i] == op) as usize;
        i += if d[i] == LINE || d[i] == TEXT {
            7
        } else if d[i] == 3.0 {
            6
        } else if d[i] == TRI {
            8
        } else {
            5
        };
    }
    n
}

#[test]
fn the_bond_tool_draws_chains_and_closes_rings() {
    let mut e = Editor::default();
    assert_eq!(click(&mut e, 200.0, 200.0) & CHANGED, CHANGED);
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (2, 1));
    // a click on an end atom grows the chain in a zigzag
    let p = px(&e, 2);
    click(&mut e, p.0, p.1);
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (3, 2));
    let (a, c) = (
        e.doc().atom(1).unwrap().clone(),
        e.doc().atom(4).unwrap().clone(),
    );
    assert!(
        ((a.x - c.x).hypot(a.y - c.y) - 3f64.sqrt()).abs() < 1e-3,
        "120 degrees between the two bonds"
    );
    // a drag from one end to the other closes the triangle; no new atom
    let (p1, p3) = (px(&e, 1), px(&e, 4));
    drag(&mut e, p3, p1);
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (3, 3));
    // a drag into empty space snaps to 30 degrees and one bond length
    drag(&mut e, p1, (p1.0 - 90.0, p1.1 + 7.0));
    let n = e.doc().atoms.last().unwrap().clone();
    assert!((n.x - (a.x - 1.0)).abs() < 1e-3 && (n.y - a.y).abs() < 1e-3);
    // a click on a bond walks its order; undo walks back
    let b = e.doc().bonds[0].clone();
    let mid = (
        (px(&e, b.a).0 + px(&e, b.b).0) / 2.0,
        (px(&e, b.a).1 + px(&e, b.b).1) / 2.0,
    );
    click(&mut e, mid.0, mid.1);
    assert_eq!(e.doc().bond(b.id).unwrap().o, 2);
    e.key("z", CTRL);
    assert_eq!(e.doc().bond(b.id).unwrap().o, 1);
    e.key("Z", CTRL | SHIFT);
    assert_eq!(e.doc().bond(b.id).unwrap().o, 2);
}

#[test]
fn atom_charge_erase_and_ring_tools() {
    let mut e = Editor::default();
    e.set_tool("ring:6a");
    click(&mut e, 300.0, 300.0);
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (6, 6));
    e.set_tool("atom:N");
    let p = px(&e, 1);
    click(&mut e, p.0, p.1);
    assert_eq!(e.doc().atom(1).unwrap().el, "N");
    assert_eq!(e.said, ("draw2.say.element", "N".to_string()));
    e.set_tool("charge:+");
    click(&mut e, p.0, p.1);
    assert_eq!(e.doc().atom(1).unwrap().chg, 1);
    // pyridinium: the label reads N, H and the charge
    let strings = e.scene().strings;
    assert_eq!(strings, ["N", "H", "+"]);
    e.set_tool("erase");
    click(&mut e, p.0, p.1);
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (5, 4));
    assert_eq!(
        click(&mut e, 5.0, 5.0) & CHANGED,
        0,
        "erasing nothing is not a step"
    );
    assert!(e.set_tool("ring:9") == 0 && e.set_tool("atom:Xx") == 0 && e.tool.name() == "erase");
}

#[test]
fn selection_moves_bands_and_deletes() {
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    e.fit(600.0, 400.0);
    e.set_tool("select");
    let p = px(&e, 1);
    let before = e.doc().atom(1).unwrap().clone();
    drag(&mut e, p, (p.0 + 80.0, p.1 - 40.0));
    let after = e.doc().atom(1).unwrap().clone();
    assert!((after.x - before.x - 2.0).abs() < 1e-3 && (after.y - before.y - 1.0).abs() < 1e-3);
    assert_eq!(e.over.sel_atoms, [1]);
    // a rubber band over everything, then Delete
    click(&mut e, 5.0, 5.0);
    assert!(e.over.sel_atoms.is_empty());
    drag(&mut e, (1.0, 1.0), (599.0, 399.0));
    assert_eq!((e.over.sel_atoms.len(), e.over.sel_bonds.len()), (3, 2));
    assert_eq!(count(&e, DISC), 3);
    assert_eq!(e.key("Delete", 0) & (CHANGED | HANDLED), CHANGED | HANDLED);
    assert!(e.doc().atoms.is_empty() && e.over.sel_atoms.is_empty());
    e.undo();
    assert_eq!(e.doc().atoms.len(), 3);
    assert_eq!(e.key("a", CTRL) & HANDLED, HANDLED);
    assert_eq!(e.over.sel_atoms.len(), 3);
}

#[test]
fn keys_pick_tools_and_set_the_element_under_the_pointer() {
    let mut e = Editor::default();
    assert_eq!(e.key("n", 0) & (TOOL | HANDLED), TOOL | HANDLED);
    assert_eq!(e.tool.name(), "atom:N");
    e.key("C", SHIFT);
    assert_eq!(e.tool.name(), "atom:Cl");
    e.key("2", 0);
    assert_eq!(e.tool.name(), "bond:2");
    click(&mut e, 100.0, 100.0);
    let p = px(&e, 1);
    e.pointer_move(p.0 + 2.0, p.1, MOUSE);
    assert_eq!(e.over.hover_atom, Some(1));
    assert_eq!(e.key("o", 0) & CHANGED, CHANGED);
    assert_eq!(e.doc().atom(1).unwrap().el, "O");
    assert_eq!(
        e.tool.name(),
        "bond:2",
        "typing over an atom keeps the tool"
    );
    assert_eq!(e.key("q", 0), 0);
    assert_eq!(e.key("r", CTRL), 0, "browser shortcuts are left alone");
    e.key("Escape", 0);
    assert_eq!(e.tool.name(), "select");
}

#[test]
fn a_finger_reaches_further_than_a_mouse() {
    let mut e = Editor::default();
    e.import("C").unwrap();
    let p = px(&e, 1);
    assert_eq!(e.hit(p.0 + 18.0, p.1, MOUSE), mzdraw::editor::Hit::None);
    assert_eq!(e.hit(p.0 + 18.0, p.1, TOUCH), mzdraw::editor::Hit::Atom(1));
}

#[test]
fn zoom_keeps_the_point_under_the_pointer() {
    let mut e = Editor::default();
    let before = e.view.to_doc(321.0, 123.0);
    e.zoom(321.0, 123.0, 1.25);
    let after = e.view.to_doc(321.0, 123.0);
    assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
    assert_eq!(e.view.scale, 50.0);
}

#[test]
fn import_and_export_go_through_the_record() {
    let mut e = Editor::default();
    let info = e.import("C[NH3+]").unwrap();
    assert!(info.placeholder && info.format == "smiles");
    let mf = e.doc().to_molfile();
    assert!(
        mf.contains("M  CHG  1   2   1") && mf.contains("M  END"),
        "{mf}"
    );
    // a molfile comes back with its coordinates, next to what is there, as one undo step
    let mut again = Editor::default();
    again.import("O").unwrap();
    let info = again.import(&mf).unwrap();
    assert!(!info.placeholder && info.format == "molfile");
    assert_eq!(again.doc().atoms.len(), 3);
    assert_eq!(again.doc().atom(again.doc().atoms[2].id).unwrap().chg, 1);
    assert!(again.doc().atoms[1].x > again.doc().atoms[0].x + 1.9);
    again.undo();
    assert_eq!(again.doc().atoms.len(), 1);
    assert_eq!(
        e.import("not a smiles ((").unwrap_err(),
        "error.draw2.smiles"
    );
    assert_eq!(e.import("").unwrap_err(), "error.draw2.import");
    // the document format reads itself
    let json = e.doc().to_json();
    let mut third = Editor::default();
    third.import(&json).unwrap();
    assert_eq!(third.doc().atoms.len(), 2);
}

#[test]
fn the_svg_is_the_display_list_without_the_overlay() {
    let mut e = Editor::default();
    e.import("C=O").unwrap();
    e.select_all();
    e.set_warnings(vec![1]);
    assert_eq!((count(&e, DISC), count(&e, RING)), (2, 1));
    let s = svg::to_svg(e.doc());
    assert!(
        s.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\""),
        "{s}"
    );
    assert_eq!(
        s.matches("<line ").count(),
        2,
        "a C=O double bond is two centred lines"
    );
    assert!(
        s.contains(">O</text>") && s.contains("fill=\"#c02020\""),
        "{s}"
    );
    assert!(!s.contains("circle"));
    assert_eq!(s, svg::to_svg(e.doc()), "same document, same bytes");
    assert!(svg::to_svg(Editor::default().doc()).contains("width=\"1\""));
}

#[test]
fn replay_gives_every_output_and_refuses_with_a_key() {
    let req = |out: &str| {
        format!(
            r#"{{"import":"c1ccccc1","log":[{{"op":"atom.set","id":1,"el":"N"}},{{"op":"ring","n":5,"bond":12}}],"out":"{out}"}}"#
        )
    };
    let doc = mzdraw::replay::replay(&req("mzdraw")).unwrap();
    assert!(doc.starts_with(r#"{"format":"mzdraw/1","next":"#));
    assert!(mzdraw::replay::replay(&req("molfile"))
        .unwrap()
        .contains("V2000"));
    assert!(mzdraw::replay::replay(&req("svg"))
        .unwrap()
        .ends_with("</svg>\n"));
    assert!(mzdraw::replay::replay(&req("log"))
        .unwrap()
        .starts_with(r#"[{"op":"atom.add","id":1,"#));
    assert_eq!(
        mzdraw::replay::replay(&req("docx")).unwrap_err().key,
        "error.draw2.request"
    );
    assert_eq!(
        mzdraw::replay::replay("{").unwrap_err().key,
        "error.draw2.request"
    );
    let e = mzdraw::replay::replay(r#"{"log":[{"op":"bond.set","id":5,"o":2}]}"#).unwrap_err();
    assert_eq!((e.key, e.id), ("error.draw2.noBond", 5));
}

#[test]
fn a_saved_document_opens_with_the_same_bytes_and_an_empty_history() {
    let mut e = Editor::default();
    e.import("C[C@H](N)C(=O)O").unwrap();
    e.set_tool("label:Boc");
    click(&mut e, 600.0, 400.0);
    e.set_tool("arrow");
    click(&mut e, 300.0, 500.0);
    let saved = e.doc().to_json();
    assert_eq!(
        mzdraw::io::saved_summary(&saved),
        Some((7, 5, 1)),
        "atoms, bonds, objects"
    );
    // opened in another editor: the same bytes, ids and all, nothing to undo, and work goes on from the same counter
    let mut again = Editor::default();
    again.import("CCO").unwrap();
    again.open(&saved).unwrap();
    assert_eq!(again.doc().to_json(), saved);
    assert!(!again.session.can_undo() && !again.session.can_redo());
    assert_eq!(again.undo(), 0);
    let next = again.doc().next;
    again.set_tool("atom:N");
    click(&mut again, 50.0, 50.0);
    assert_eq!(again.doc().atoms.last().unwrap().id, next);
    again.undo();
    assert_eq!(again.doc().to_json(), saved);
    // what is not a document, or an empty one, is not offered; a bad text opens nothing
    // (a file of a later version is refused too, and said to be one: the version rule)
    for (bad, key) in [
        ("", "error.draw2.document"),
        ("CCO", "error.draw2.document"),
        ("{}", "error.draw2.document"),
        (
            r#"{"format":"mzdraw/2","next":1,"atoms":[],"bonds":[]}"#,
            "error.draw2.documentNewer",
        ),
    ] {
        assert_eq!(mzdraw::io::saved_summary(bad), None, "{bad}");
        assert_eq!(again.open(bad).unwrap_err(), key, "{bad}");
        assert_eq!(again.doc().to_json(), saved);
    }
    assert_eq!(
        mzdraw::io::saved_summary(&Editor::default().doc().to_json()),
        None
    );
    assert!(Editor::default()
        .open(&Editor::default().doc().to_json())
        .is_ok());
}

#[test]
fn the_chain_tool_follows_the_pointer_and_shows_the_count() {
    let mut e = Editor::default();
    assert_eq!(e.set_tool("chain") & TOOL, TOOL);
    assert_eq!(e.tool.name(), "chain");
    // a drag to the right: one bond advances 0.866 of a bond length along the axis, 40 px a bond length
    e.pointer_down(100.0, 300.0, MOUSE, 0);
    e.pointer_move(200.0, 300.0, MOUSE);
    e.pointer_move(100.0 + 4.0 * 0.866 * 40.0, 300.0, MOUSE);
    // the preview holds 4 bonds and shows the number 4 next to its end; nothing is in the drawing yet
    assert_eq!(e.doc().atoms.len(), 0);
    assert_eq!(e.over.chain.as_ref().map(|c| c.len()), Some(5));
    assert!(e.scene().strings.contains(&"4".to_string()));
    assert_eq!(
        e.pointer_up(100.0 + 4.0 * 0.866 * 40.0, 300.0) & CHANGED,
        CHANGED
    );
    assert!(e.over.chain.is_none());
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (5, 4));
    assert_eq!(e.said, ("draw2.say.chain", "4".to_string()));
    // the zigzag runs along the horizontal: the atoms alternate between two levels
    let ys: Vec<f64> = e.doc().atoms.iter().map(|a| a.y).collect();
    assert!((ys[0] - ys[2]).abs() < 1e-3 && (ys[2] - ys[4]).abs() < 1e-3);
    assert!((ys[1] - ys[0]).abs() > 0.4);
    // one undo takes the whole chain away
    e.undo();
    assert_eq!(e.doc().atoms.len(), 0);
    // the count is at least 1 and at most 50
    drag(&mut e, (100.0, 300.0), (101.0, 330.0));
    assert_eq!(e.doc().bonds.len(), 1);
    e.undo();
    drag(&mut e, (50.0, 300.0), (50.0 + 4000.0, 300.0));
    assert_eq!(e.doc().bonds.len(), 50);
}

#[test]
fn the_chain_tool_clicks_two_bonds_and_starts_on_an_atom() {
    let mut e = Editor::default();
    e.set_tool("chain");
    click(&mut e, 200.0, 300.0);
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (3, 2));
    // a drag from the end atom adds bonds to it, no second atom on the start
    let end = px(&e, 3);
    drag(&mut e, end, (end.0 + 3.0 * 0.866 * 40.0, end.1));
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (6, 5));
    // a click on an atom grows two bonds from it
    let end = px(&e, 6);
    click(&mut e, end.0, end.1);
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (8, 7));
}

#[test]
fn the_chain_tool_takes_a_finger_and_a_pen() {
    for kind in [TOUCH, PEN] {
        let mut e = Editor::default();
        e.set_tool("chain");
        e.pointer_event(0, 1, 100.0, 300.0, kind, 0, 0.0);
        e.pointer_event(1, 1, 150.0, 300.0, kind, 0, 20.0);
        e.pointer_event(1, 1, 240.0, 300.0, kind, 0, 40.0);
        assert!(e.over.chain.is_some(), "the preview follows the {kind}");
        assert_eq!(
            e.pointer_event(2, 1, 240.0, 300.0, kind, 0, 60.0) & CHANGED,
            CHANGED
        );
        assert_eq!(e.doc().bonds.len(), 4);
    }
}

#[test]
fn the_chain_tool_by_keyboard_types_a_count_and_draws_from_the_focus() {
    use mzdraw::editor::FOCUS;
    let mut e = Editor::default();
    e.set_tool("chain");
    // no atom yet: Enter draws two bonds, the focus is on the end
    assert_eq!(e.key("Enter", 0) & CHANGED, CHANGED);
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (3, 2));
    assert!(e.over.focus.is_some());
    // digits type the count, shown as a preview with its number; Backspace takes a digit; the drawing has not changed
    e.key("4", 0);
    assert_eq!(e.over.chain.as_ref().map(Vec::len), Some(5));
    assert_eq!(e.said, ("draw2.say.chainCount", "4".to_string()));
    assert!(e.scene().strings.contains(&"4".to_string()));
    e.key("1", 0);
    assert_eq!(e.said.1, "41", "two digits");
    // more than 50 is held at 50
    e.key("Backspace", 0);
    e.key("9", 0);
    assert_eq!(e.said.1, "49");
    e.key("Backspace", 0);
    e.key("Backspace", 0);
    assert!(e.over.chain.is_none());
    e.key("5", 0);
    e.key("8", 0);
    assert_eq!(e.said.1, "50");
    e.key("Escape", 0);
    assert!(
        e.over.chain.is_none() && e.tool.name() == "chain",
        "Escape forgets the count and keeps the tool"
    );
    // 4 bonds from the focused atom, one undo step, the focus on the new end
    let start = e.over.focus.unwrap();
    e.key("4", 0);
    assert_eq!(e.key("Enter", 0) & (CHANGED | FOCUS), CHANGED | FOCUS);
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (7, 6));
    assert_ne!(e.over.focus, Some(start));
    assert!(e.over.chain.is_none());
    e.undo();
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (3, 2));
    // a second Escape leaves the tool, as for every tool
    e.key("Escape", 0);
    assert_eq!(e.tool.name(), "select");
}

#[test]
fn a_chain_from_a_stereo_centre_keeps_what_was_drawn_and_says_nothing() {
    use mzdraw::replay::replay;
    use serde_json::json;
    let log = json!([
        {"op":"atom.add","el":"C","x":0.0,"y":0.0},
        {"op":"atom.add","el":"F","x":0.0,"y":1.0},
        {"op":"atom.add","el":"Cl","x":-0.9,"y":-0.5},
        {"op":"atom.add","el":"Br","x":0.9,"y":-0.5},
        {"op":"bond.add","a":1,"b":2,"o":1,"s":1},
        {"op":"bond.add","a":1,"b":3,"o":1,"s":0},
        {"op":"bond.add","a":1,"b":4,"o":1,"s":0}
    ]);
    let mut e = Editor::default();
    e.import(&replay(&json!({"log": log, "out": "mzdraw"}).to_string()).unwrap())
        .unwrap();
    let before: Vec<(f64, f64)> = e.doc().atoms.iter().map(|a| (a.x, a.y)).collect();
    e.set_tool("chain");
    let (x, y) = e.view.to_px(0.0, 0.0);
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_up(x, y);
    assert_eq!(e.doc().atoms.len(), 6);
    // nothing that was drawn moved, the line of the guard is empty, the wedge is still there
    let after: Vec<(f64, f64)> = e.doc().atoms.iter().take(4).map(|a| (a.x, a.y)).collect();
    assert_eq!(before, after);
    assert_eq!(e.notice.0, "");
    assert!(e.doc().bonds.iter().any(|b| b.s == 1));
    assert!(e.doc().to_smiles().is_ok());
}
