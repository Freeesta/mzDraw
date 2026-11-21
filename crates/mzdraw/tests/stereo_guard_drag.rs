//! No signature is computed while a drag is under way: the counter of `stereo_guard` moves once, at the release of the pointer.
//! (Alone in its file, so the process counter is not shared with another test.)

use mzdraw::editor::MOUSE;
use mzdraw::replay::replay;
use mzdraw::stereo_guard::computed;
use mzdraw::Editor;
use serde_json::json;

#[test]
fn a_drag_computes_the_signature_once_at_the_release_and_never_per_move() {
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
    e.set_tool("select");
    e.select_all();
    let a = e.doc().atom(3).unwrap().clone();
    let (x, y) = e.view.to_px(a.x, a.y);
    let n0 = computed();
    e.pointer_down(x, y, MOUSE, 0);
    for k in 1..=40 {
        e.pointer_move(x + k as f64, y - k as f64, MOUSE);
        assert_eq!(computed(), n0, "no signature during the drag (move {k})");
    }
    e.pointer_up(x + 40.0, y - 40.0);
    // one signature before the step (the piece holds a wedge); the one after is the comparison
    assert_eq!(computed(), n0 + 2);
}
