//! Touch and pen as the page forwards them: every pointer with its id and its time stamp. Two fingers move the view and never
//! draw, a palm near the pen is ignored, a long press selects, the eraser end of a pen erases, a cancelled pointer commits nothing.

use mzdraw::editor::{CHANGED, MOUSE, PEN, TOUCH};
use mzdraw::editor_touch::{CANCEL, DOWN, ERASER, MOVE, UP};
use mzdraw::Editor;

fn benzene() -> Editor {
    let mut e = Editor::default();
    e.set_tool("ring:6a");
    e.pointer_event(DOWN, 1, 300.0, 300.0, MOUSE, 0, 0.0);
    e.pointer_event(UP, 1, 300.0, 300.0, MOUSE, 0, 10.0);
    assert_eq!(e.doc().atoms.len(), 6);
    e.set_tool("bond:1");
    e
}

fn px(e: &Editor, id: u32) -> (f64, f64) {
    let a = e.doc().atom(id).unwrap();
    e.view.to_px(a.x, a.y)
}

#[test]
fn one_finger_uses_the_tool_as_a_mouse_does() {
    let mut e = benzene();
    let p = px(&e, 1);
    assert_eq!(e.pointer_event(DOWN, 7, p.0, p.1, TOUCH, 0, 1000.0) & 1, 1);
    assert_eq!(
        e.pointer_event(UP, 7, p.0, p.1, TOUCH, 0, 1100.0) & CHANGED,
        CHANGED
    );
    assert_eq!(
        (e.doc().atoms.len(), e.doc().bonds.len()),
        (7, 7),
        "a tap on an atom sprouts a bond"
    );
}

#[test]
fn two_fingers_pan_and_zoom_and_never_draw() {
    let mut e = benzene();
    let doc = e.doc().to_json();
    let (scale, ox) = (e.view.scale, e.view.ox);
    // the first finger starts a drag from an atom; the second one lands: the drag is dropped
    let p = px(&e, 1);
    e.pointer_event(DOWN, 1, p.0, p.1, TOUCH, 0, 0.0);
    e.pointer_event(MOVE, 1, p.0 + 30.0, p.1, TOUCH, 0, 20.0);
    assert!(e.over.preview.is_some(), "a bond is being drawn");
    e.pointer_event(DOWN, 2, p.0 + 130.0, p.1, TOUCH, 0, 40.0);
    assert!(e.over.preview.is_none(), "the second finger drops it");
    // spread: the distance goes from 100 to 200 px, the middle stays: the view doubles around the middle
    let mid_doc = e.view.to_doc(p.0 + 80.0, p.1);
    e.pointer_event(MOVE, 1, p.0 - 20.0, p.1, TOUCH, 0, 60.0);
    e.pointer_event(MOVE, 2, p.0 + 180.0, p.1, TOUCH, 0, 80.0);
    assert!(
        (e.view.scale / scale - 2.0).abs() < 1e-9,
        "{}",
        e.view.scale / scale
    );
    let mid_px = e.view.to_px(mid_doc.0, mid_doc.1);
    assert!(
        (mid_px.0 - (p.0 + 80.0)).abs() < 1e-6 && (mid_px.1 - p.1).abs() < 1e-6,
        "the point between the fingers stays under them"
    );
    // both fingers move together: the view pans by the same amount, the scale stays
    let (s2, ox2, oy2) = (e.view.scale, e.view.ox, e.view.oy);
    e.pointer_event(MOVE, 1, p.0 - 20.0 + 50.0, p.1 + 30.0, TOUCH, 0, 100.0);
    e.pointer_event(MOVE, 2, p.0 + 180.0 + 50.0, p.1 + 30.0, TOUCH, 0, 120.0);
    assert!((e.view.scale - s2).abs() < 1e-9);
    assert!((e.view.ox - ox2 - 50.0).abs() < 1e-6 && (e.view.oy - oy2 - 30.0).abs() < 1e-6);
    // one finger lifts, the other moves on and lifts: still nothing drawn
    e.pointer_event(UP, 2, p.0 + 230.0, p.1 + 30.0, TOUCH, 0, 140.0);
    e.pointer_event(MOVE, 1, p.0 + 400.0, p.1 + 200.0, TOUCH, 0, 160.0);
    e.pointer_event(UP, 1, p.0 + 400.0, p.1 + 200.0, TOUCH, 0, 180.0);
    assert_eq!(e.doc().to_json(), doc, "a view gesture never edits");
    assert!(!e.session.can_redo() && e.session.steps() == 1);
    assert_ne!(e.view.ox, ox);
    // the next single finger draws again
    let p = px(&e, 1);
    e.pointer_event(DOWN, 3, p.0, p.1, TOUCH, 0, 1000.0);
    e.pointer_event(UP, 3, p.0, p.1, TOUCH, 0, 1050.0);
    assert_eq!(e.doc().atoms.len(), 7);
    // a third finger is not followed and does nothing
    let doc = e.doc().to_json();
    for (id, x) in [(4, 100.0), (5, 200.0), (6, 250.0)] {
        e.pointer_event(DOWN, id, x, 100.0, TOUCH, 0, 2000.0);
    }
    e.pointer_event(MOVE, 6, 500.0, 500.0, TOUCH, 0, 2010.0);
    for id in [6, 5, 4] {
        e.pointer_event(UP, id, 0.0, 0.0, TOUCH, 0, 2020.0);
    }
    assert_eq!(e.doc().to_json(), doc);
}

/// What looked like a lost first tap on the page: after a pen stroke a finger tap within the palm window is taken whole as a palm, a
/// second one inside the window too, and nothing stays held: the first tap after the window draws, with the same finger id.
#[test]
fn a_tap_inside_the_palm_window_after_the_pen_is_lost_whole_and_the_first_one_after_it_draws() {
    let mut e = Editor::default();
    e.set_tool("chain");
    // a pen stroke ends at t = 1000
    e.pointer_event(DOWN, 20, 300.0, 300.0, PEN, 0, 900.0);
    e.pointer_event(UP, 20, 300.0, 300.0, PEN, 0, 1000.0);
    let after_pen = (e.doc().atoms.len(), e.doc().bonds.len());
    assert_eq!(
        after_pen,
        (3, 2),
        "a pen tap with the chain tool draws two bonds"
    );
    // finger taps with the same id, each lifted: inside the window nothing is drawn, and none of them is held
    for (down, up) in [(1100.0, 1150.0), (1300.0, 1350.0), (1480.0, 1490.0)] {
        e.pointer_event(DOWN, 11, 100.0, 100.0, TOUCH, 0, down);
        e.pointer_event(UP, 11, 100.0, 100.0, TOUCH, 0, up);
        assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), after_pen);
    }
    // the first tap after the window draws, and so does the next one
    for (n, t) in [(1, 1600.0), (2, 1800.0)] {
        e.pointer_event(DOWN, 11, 100.0 + 200.0 * n as f64, 100.0, TOUCH, 0, t);
        e.pointer_event(UP, 11, 100.0 + 200.0 * n as f64, 100.0, TOUCH, 0, t + 20.0);
        assert_eq!(
            e.doc().atoms.len(),
            3 + 3 * n,
            "tap {n} after the window draws"
        );
    }
}

#[test]
fn a_palm_near_the_pen_is_ignored() {
    let mut e = benzene();
    let doc = e.doc().to_json();
    let view = e.view;
    // the pen hovers; a hand lands within half a second: not a tap
    e.pointer_event(MOVE, 20, 500.0, 500.0, PEN, 0, 1000.0);
    e.pointer_event(DOWN, 21, 520.0, 560.0, TOUCH, 0, 1200.0);
    e.pointer_event(MOVE, 21, 525.0, 565.0, TOUCH, 0, 1300.0);
    e.pointer_event(UP, 21, 525.0, 565.0, TOUCH, 0, 1400.0);
    assert_eq!(e.doc().to_json(), doc, "the palm drew nothing");
    // the pen writes while the palm rests and a second part of the hand lands: only the pen draws, the view stays
    e.pointer_event(DOWN, 22, 520.0, 560.0, TOUCH, 0, 1500.0);
    e.pointer_event(DOWN, 20, 500.0, 500.0, PEN, 0, 1600.0);
    e.pointer_event(DOWN, 23, 560.0, 600.0, TOUCH, 0, 1650.0);
    e.pointer_event(MOVE, 23, 600.0, 640.0, TOUCH, 0, 1700.0);
    e.pointer_event(UP, 20, 500.0, 500.0, PEN, 0, 1800.0);
    e.pointer_event(UP, 22, 520.0, 560.0, TOUCH, 0, 1900.0);
    e.pointer_event(UP, 23, 600.0, 640.0, TOUCH, 0, 1950.0);
    assert_eq!(
        (e.doc().atoms.len(), e.doc().bonds.len()),
        (8, 7),
        "the pen drew one bond, the hand nothing"
    );
    assert_eq!(
        (e.view.scale, e.view.ox, e.view.oy),
        (view.scale, view.ox, view.oy)
    );
    // well after the pen left, a finger draws again
    let n = e.doc().atoms.len();
    e.pointer_event(DOWN, 24, 100.0, 100.0, TOUCH, 0, 3000.0);
    e.pointer_event(UP, 24, 100.0, 100.0, TOUCH, 0, 3050.0);
    assert_eq!(e.doc().atoms.len(), n + 2);
}

#[test]
fn a_long_press_selects_and_a_cancelled_pointer_commits_nothing() {
    let mut e = benzene();
    let doc = e.doc().to_json();
    let p = px(&e, 2);
    // held still for 600 ms on an atom with the bond tool: the atom is selected, nothing is drawn
    e.pointer_event(DOWN, 1, p.0, p.1, TOUCH, 0, 0.0);
    assert_eq!(
        e.pointer_event(UP, 1, p.0, p.1, TOUCH, 0, 600.0) & CHANGED,
        CHANGED
    );
    assert_eq!(e.doc().to_json(), doc);
    assert_eq!(e.over.sel_atoms, vec![2]);
    assert_eq!(e.tool.name(), "bond:1", "the tool is kept");
    // a long press on another atom replaces the selection; on a bond it selects the bond and its two atoms
    let q = px(&e, 4);
    e.pointer_event(DOWN, 1, q.0, q.1, TOUCH, 0, 1000.0);
    e.pointer_event(UP, 1, q.0, q.1, TOUCH, 0, 1700.0);
    assert_eq!(e.over.sel_atoms, vec![4]);
    let (a, b) = (px(&e, 5), px(&e, 6));
    let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    e.pointer_event(DOWN, 1, m.0, m.1, TOUCH, 0, 2000.0);
    e.pointer_event(UP, 1, m.0, m.1, TOUCH, 0, 2600.0);
    assert_eq!((e.over.sel_bonds.len(), e.over.sel_atoms.len()), (1, 2));
    assert_eq!(e.doc().to_json(), doc);
    // held long but dragged (away from the middle of the ring, where no atom is): it is a drag, the tool acts
    let out = (p.0 + (p.0 - 300.0) * 1.5, p.1 + (p.1 - 300.0) * 1.5);
    e.pointer_event(DOWN, 1, p.0, p.1, TOUCH, 0, 3000.0);
    e.pointer_event(MOVE, 1, out.0, out.1, TOUCH, 0, 3300.0);
    e.pointer_event(UP, 1, out.0, out.1, TOUCH, 0, 3700.0);
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (7, 7));
    // a mouse held as long is still a click
    e.pointer_event(DOWN, 1, p.0, p.1, MOUSE, 0, 5000.0);
    e.pointer_event(UP, 1, p.0, p.1, MOUSE, 0, 6000.0);
    assert_eq!(e.doc().atoms.len(), 8);
    // cancelled in the middle of a drag: nothing is committed, by finger, pen or mouse
    let doc = e.doc().to_json();
    for kind in [TOUCH, PEN, MOUSE] {
        e.pointer_event(DOWN, 9, p.0, p.1, kind, 0, 9000.0);
        e.pointer_event(MOVE, 9, p.0 - 60.0, p.1 + 20.0, kind, 0, 9050.0);
        assert!(e.over.preview.is_some());
        e.pointer_event(CANCEL, 9, p.0 - 60.0, p.1 + 20.0, kind, 0, 9100.0);
        assert!(e.over.preview.is_none());
        assert_eq!(e.doc().to_json(), doc, "kind {kind}");
    }
}

#[test]
fn the_eraser_end_of_a_pen_erases_for_one_stroke() {
    let mut e = benzene();
    let p = px(&e, 3);
    e.pointer_event(DOWN, 30, p.0, p.1, PEN, ERASER, 0.0);
    assert_eq!(e.tool.name(), "erase");
    assert_eq!(
        e.pointer_event(UP, 30, p.0, p.1, PEN, ERASER, 50.0) & CHANGED,
        CHANGED
    );
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (5, 4));
    assert_eq!(
        e.tool.name(),
        "bond:1",
        "the tool comes back after the stroke"
    );
    // the tip draws again; one undo brings the erased atom back
    let q = px(&e, 1);
    e.pointer_event(DOWN, 30, q.0, q.1, PEN, 0, 100.0);
    e.pointer_event(UP, 30, q.0, q.1, PEN, 0, 150.0);
    assert_eq!(e.doc().atoms.len(), 6);
    e.undo();
    e.undo();
    assert_eq!((e.doc().atoms.len(), e.doc().bonds.len()), (6, 6));
}

#[test]
fn a_finger_on_the_middle_of_a_bond_hits_the_bond() {
    use mzdraw::editor::Hit;
    let e = benzene();
    let b = e.doc().bonds[0].clone();
    let (p, q) = (px(&e, b.a), px(&e, b.b));
    let at = |t: f64| (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
    // a bond is 40 px long here and a finger reaches 22 px: both ends are within reach of the middle
    for kind in [TOUCH, PEN, MOUSE] {
        let m = at(0.5);
        assert_eq!(
            e.hit(m.0, m.1, kind),
            Hit::Bond(b.id),
            "the middle, kind {kind}"
        );
        let (near_a, near_b) = (at(0.1), at(0.9));
        assert_eq!(
            e.hit(near_a.0, near_a.1, kind),
            Hit::Atom(b.a),
            "near one end, kind {kind}"
        );
        assert_eq!(
            e.hit(near_b.0, near_b.1, kind),
            Hit::Atom(b.b),
            "near the other end, kind {kind}"
        );
        assert_eq!(
            e.hit(p.0, p.1, kind),
            Hit::Atom(b.a),
            "on the atom, kind {kind}"
        );
    }
    assert_eq!(e.hit(900.0, 900.0, TOUCH), Hit::None);
}
