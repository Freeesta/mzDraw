//! What the guard computes, counted: nothing while a drag is under way, nothing for a rigid edit or a drawing without wedge, and the
//! elements near the edit at the release. (Alone in its file, one test, so the process counter is not shared with another test.)

use mzdraw::editor::MOUSE;
use mzdraw::replay::replay;
use mzdraw::stereo_guard::computed;
use mzdraw::Editor;
use serde_json::json;

#[test]
fn what_the_guard_computes_is_counted() {
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
    e.import(&replay(&json!({"log":log,"out":"mzdraw"}).to_string()).unwrap())
        .unwrap();

    // a rigid edit of the whole piece: nothing is computed, by key or by drag
    e.set_tool("select");
    e.select_all();
    let n0 = computed();
    e.nudge(1.0, 0.0, 0.0);
    e.rotate(30.0);
    let a = e.doc().atom(3).unwrap().clone();
    let (x, y) = e.view.to_px(a.x, a.y);
    e.pointer_down(x, y, MOUSE, 0);
    for k in 1..=40 {
        e.pointer_move(x + f64::from(k), y - f64::from(k), MOUSE);
    }
    e.pointer_up(x + 40.0, y - 40.0);
    assert_eq!(computed(), n0, "a rigid piece has nothing to compare");

    // one atom dragged: nothing during the 40 moves, the elements near it once before and once after at the release
    e.set_tool("select");
    e.pointer_down(5.0, 5.0, MOUSE, 0);
    e.pointer_up(5.0, 5.0);
    let a = e.doc().atom(3).unwrap().clone();
    let (x, y) = e.view.to_px(a.x, a.y);
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_up(x, y);
    let n1 = computed();
    e.pointer_down(x, y, MOUSE, 0);
    for k in 1..=40 {
        e.pointer_move(x + f64::from(k) * 2.0, y - f64::from(k) * 2.0, MOUSE);
        assert_eq!(computed(), n1, "no signature during the drag (move {k})");
    }
    e.pointer_up(x + 80.0, y - 80.0);
    // one centre: its signature before the step and after it, and, because it changed, the canonical text of its piece before and
    // after (to tell a mirror image of the same molecule from a change)
    assert_eq!(computed(), n1 + 4);
    assert_ne!(e.doc().atom(3).unwrap().x, a.x, "the drag moved the atom");

    // a drawing with no wedge and no double bond: nothing at all
    let mut p = Editor::default();
    p.import("CCCCO").unwrap();
    p.select_all();
    let n2 = computed();
    p.nudge(0.5, 0.5, 0.0);
    p.flip(mzdraw::Axis::H);
    assert_eq!(computed(), n2);
}
