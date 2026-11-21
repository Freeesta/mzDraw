//! Moving and turning without dragging (WCAG 2.2 SC 2.5.7): keys on a selection, coalescing into one undo step, "move here".

use mzdraw::editor::{ALT, CHANGED, HANDLED, SHIFT};
use mzdraw::{Editor, Op};

/// A drawn chain of three atoms (synthetic), all selected.
fn chain() -> Editor {
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    e.set_tool("select");
    e.select_all();
    e
}

fn positions(e: &Editor) -> Vec<(f64, f64)> {
    e.doc().atoms.iter().map(|a| (a.x, a.y)).collect()
}

fn near(a: &[(f64, f64)], b: &[(f64, f64)], tol: f64) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(p, q)| (p.0 - q.0).abs() < tol && (p.1 - q.1).abs() < tol)
}

#[test]
fn each_key_gives_one_move_or_rotate_op_in_the_log() {
    let mut e = chain();
    let start = positions(&e);
    let f = e.key_at("ArrowRight", ALT, 0.0);
    assert!(f & HANDLED != 0 && f & CHANGED != 0);
    e.key_at("ArrowUp", ALT | SHIFT, 5000.0);
    e.key_at("ArrowLeft", ALT, 10000.0);
    e.key_at("ArrowDown", ALT, 15000.0);
    let log = e.session.log();
    let moves: Vec<(f64, f64)> = log
        .iter()
        .filter_map(|o| match o {
            Op::Move { dx, dy, .. } => Some((*dx, *dy)),
            _ => None,
        })
        .collect();
    assert_eq!(
        moves,
        vec![(0.1, 0.0), (0.0, 1.0), (-0.1, 0.0), (0.0, -0.1)]
    );
    e.key_at("[", ALT, 20000.0);
    e.key_at("]", ALT, 25000.0);
    let turns: Vec<f64> = e
        .session
        .log()
        .iter()
        .filter_map(|o| match o {
            Op::Rotate { deg, .. } => Some(*deg),
            _ => None,
        })
        .collect();
    assert_eq!(turns, vec![15.0, -15.0]);
    assert!(!near(&positions(&e), &start, 0.5));
    assert_eq!(e.said.0, "draw2.announce.rotated");
}

#[test]
fn three_presses_are_one_undo_step_and_undo_restores_the_document() {
    let mut e = chain();
    let before = e.doc().to_json();
    let steps = e.session.steps();
    let logged = e.session.log().len();
    for t in [0.0, 200.0, 400.0] {
        e.key_at("ArrowRight", ALT, t);
    }
    assert_eq!(e.session.steps(), steps + 1);
    assert_eq!(
        e.session.log().len(),
        logged + 3,
        "the log keeps every press"
    );
    assert_eq!(e.said, ("draw2.announce.moved", "right 0.3".to_string()));
    e.undo();
    assert_eq!(e.doc().to_json(), before);
    e.redo();
    assert_ne!(e.doc().to_json(), before);
    e.undo();
    assert_eq!(e.doc().to_json(), before);
}

#[test]
fn a_pause_another_command_or_another_selection_starts_a_new_step() {
    let mut e = chain();
    let steps = e.session.steps();
    e.key_at("ArrowRight", ALT, 0.0);
    e.key_at("ArrowRight", ALT, 1500.0); // a pause of more than a second
    assert_eq!(e.session.steps(), steps + 2);
    e.key_at("ArrowRight", ALT, 1600.0);
    assert_eq!(e.session.steps(), steps + 2);
    e.rotate(15.0); // another command in between
    e.key_at("ArrowRight", ALT, 1700.0);
    assert_eq!(e.session.steps(), steps + 4);
    e.key_at("ArrowRight", ALT, 1800.0);
    assert_eq!(e.session.steps(), steps + 4);
    // another selection
    let first = e.doc().atoms[0].id;
    e.over.sel_atoms = vec![first];
    e.key_at("ArrowRight", ALT, 1900.0);
    assert_eq!(e.session.steps(), steps + 5);
    // undo, then a nudge: not joined to what was undone
    e.undo();
    e.key_at("ArrowRight", ALT, 2000.0);
    assert_eq!(e.session.steps(), steps + 5);
}

#[test]
fn twenty_four_turns_of_fifteen_degrees_come_back_to_the_start() {
    let mut e = chain();
    let start = positions(&e);
    let steps = e.session.steps();
    for i in 0..24 {
        e.key_at("[", ALT, f64::from(i) * 10.0);
    }
    // every turn rounds the coordinates to the 4 decimals of the document (at most 5e-5 each way, and the centre moves by as
    // much), so 24 turns are back within 24 times 1e-4
    assert!(
        near(&positions(&e), &start, 24.0 * 1e-4),
        "{:?} {:?}",
        positions(&e),
        start
    );
    assert_eq!(e.session.steps(), steps + 1);
    assert_eq!(
        e.said,
        (
            "draw2.announce.rotated",
            "360 counter-clockwise".to_string()
        )
    );
    e.undo();
    assert!(near(&positions(&e), &start, 1e-12));
}

#[test]
fn without_a_selection_the_keys_do_nothing_and_are_still_swallowed() {
    let mut e = Editor::default();
    e.import("CC").unwrap();
    let before = e.doc().to_json();
    let f = e.key_at("ArrowLeft", ALT, 0.0);
    assert!(f & HANDLED != 0 && f & CHANGED == 0);
    let f = e.key_at("]", ALT, 0.0);
    assert!(f & HANDLED != 0 && f & CHANGED == 0);
    assert_eq!(e.doc().to_json(), before);
    assert_eq!(e.session.steps(), 1);
}

#[test]
fn shift_arrow_extends_the_selection_along_the_bonds() {
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    let ids: Vec<u32> = e.doc().atoms.iter().map(|a| a.id).collect();
    e.over.focus = Some(ids[0]);
    // walk towards the neighbour with Shift held: whichever arrow reaches it
    let mut walked = false;
    for k in ["ArrowRight", "ArrowUp", "ArrowDown", "ArrowLeft"] {
        e.over.focus = Some(ids[0]);
        e.over.sel_atoms.clear();
        e.key_at(k, SHIFT, 0.0);
        if e.over.focus != Some(ids[0]) {
            walked = true;
            assert!(e.over.sel_atoms.contains(&ids[0]));
            assert!(e.over.sel_atoms.contains(&e.over.focus.unwrap()));
            assert_eq!(e.over.sel_bonds.len(), 1);
            break;
        }
    }
    assert!(walked);
    // a plain arrow does not select
    e.over.sel_atoms.clear();
    e.over.sel_bonds.clear();
    e.over.focus = Some(ids[0]);
    e.key_at("ArrowRight", 0, 0.0);
    e.key_at("ArrowUp", 0, 0.0);
    assert!(e.over.sel_atoms.is_empty());
}

#[test]
fn move_here_brings_the_selection_centre_to_the_click() {
    let mut e = chain();
    e.view.scale = 40.0;
    e.view.ox = 200.0;
    e.view.oy = 200.0;
    e.key("m", 0);
    assert!(e.move_here);
    let steps = e.session.steps();
    let (px, py) = e.view.to_px(5.0, 3.0);
    assert_eq!(e.hit(px, py, 0), mzdraw::editor::Hit::None);
    e.pointer_down(px, py, 0, 0);
    let f = e.pointer_up(px, py);
    assert!(f & CHANGED != 0);
    assert_eq!(e.session.steps(), steps + 1);
    let xs: Vec<f64> = e.doc().atoms.iter().map(|a| a.x).collect();
    let ys: Vec<f64> = e.doc().atoms.iter().map(|a| a.y).collect();
    let min = |v: &[f64]| v.iter().cloned().fold(f64::MAX, f64::min);
    let max = |v: &[f64]| v.iter().cloned().fold(f64::MIN, f64::max);
    assert!(((min(&xs) + max(&xs)) / 2.0 - 5.0).abs() < 1e-9);
    assert!(((min(&ys) + max(&ys)) / 2.0 - 3.0).abs() < 1e-9);
    // the selection stays, the mode stays; one undo puts it back
    assert_eq!(e.over.sel_atoms.len(), 3);
    assert!(e.move_here);
    let moved = e.doc().to_json();
    e.undo();
    assert_ne!(e.doc().to_json(), moved);
    // the mode off: a click on an empty spot clears the selection again
    e.key("m", 0);
    assert!(!e.move_here);
    let steps = e.session.steps();
    e.pointer_down(px, py, 0, 0);
    e.pointer_up(px, py);
    assert!(e.over.sel_atoms.is_empty());
    assert_eq!(e.session.steps(), steps);
}

#[test]
fn move_here_without_a_selection_selects_nothing_and_moves_nothing() {
    let mut e = Editor::default();
    e.import("CC").unwrap();
    e.set_tool("select");
    e.over.sel_atoms.clear();
    e.key("m", 0);
    assert_eq!(e.tool, mzdraw::editor::Tool::Select);
    let before = e.doc().to_json();
    let (px, py) = e.view.to_px(9.0, 9.0);
    e.pointer_down(px, py, 0, 0);
    e.pointer_up(px, py);
    assert_eq!(e.doc().to_json(), before);
}

#[test]
fn move_here_works_with_touch() {
    let mut e = chain();
    e.key("m", 0);
    let (px, py) = e.view.to_px(4.0, -2.0);
    e.pointer_event(0, 1, px, py, mzdraw::editor::TOUCH, 0, 100.0);
    let f = e.pointer_event(2, 1, px, py, mzdraw::editor::TOUCH, 0, 150.0);
    assert!(f & CHANGED != 0);
    let a = &e.doc().atoms;
    let (x0, x1) = (
        a.iter().map(|a| a.x).fold(f64::MAX, f64::min),
        a.iter().map(|a| a.x).fold(f64::MIN, f64::max),
    );
    assert!(((x0 + x1) / 2.0 - 4.0).abs() < 1e-9);
}
