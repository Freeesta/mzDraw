//! The unsaved-changes guard in the core: the saved point of the history (`Editor::dirty`), the question the page asks (`guard`,
//! `unsaved`) and the commands `new`, `open` and `save` that refuse to lose unsaved work unless `discard` says so.

use mzdraw::editor::{CHANGED, MOUSE};
use mzdraw::editor_file::{Choice, Guard};
use mzdraw::Editor;
use serde_json::{json, Value};

/// One atom drawn the way the page does it: the tool, a click, the naming after the action.
fn draw(e: &mut Editor, x: f64, y: f64) {
    e.set_tool("atom:C");
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_up(x, y);
    e.name_steps();
}

fn result(request: Value) -> Value {
    serde_json::from_str(&mzdraw::replay::replay(&request.to_string()).unwrap()).unwrap()
}

#[test]
fn a_new_editor_is_clean_and_asks_nothing() {
    let e = Editor::default();
    assert!(!e.dirty() && !e.unsaved());
    assert_eq!(e.guard(), Guard::Go);
}

#[test]
fn an_edit_makes_it_dirty_and_undo_back_to_the_saved_point_makes_it_clean_again() {
    let mut e = Editor::default();
    draw(&mut e, 100.0, 100.0);
    assert!(e.dirty() && e.unsaved());
    assert_eq!(e.guard(), Guard::Ask);
    e.undo();
    assert!(!e.dirty() && !e.unsaved(), "undo to the start is clean");
    e.redo();
    assert!(e.dirty(), "redo leaves the saved point again");
}

#[test]
fn a_save_is_the_saved_point_and_undo_past_it_is_dirty() {
    let mut e = Editor::default();
    draw(&mut e, 100.0, 100.0);
    draw(&mut e, 200.0, 100.0);
    e.mark_saved();
    assert!(!e.dirty() && e.guard() == Guard::Go);
    e.undo();
    assert!(e.dirty(), "one step before the saved point differs from it");
    e.redo();
    assert!(!e.dirty(), "redo to the saved point is clean");
    draw(&mut e, 300.0, 100.0);
    assert!(e.dirty());
    e.undo();
    assert!(!e.dirty(), "undo of the step made after the save is clean");
}

#[test]
fn a_new_step_from_before_the_saved_point_makes_the_point_unreachable() {
    let mut e = Editor::default();
    draw(&mut e, 100.0, 100.0);
    draw(&mut e, 200.0, 100.0);
    e.mark_saved(); // two steps
    e.undo(); // one step: the saved point waits in the redo stack
    draw(&mut e, 300.0, 100.0); // two steps again, but not the saved ones
    assert!(e.dirty(), "the same number of steps is not the saved state");
    e.undo();
    e.undo();
    assert!(e.dirty(), "the start is not the saved state either");
    e.redo();
    e.redo();
    assert!(e.dirty());
}

#[test]
fn a_step_that_grows_in_place_after_a_save_is_dirty() {
    // repeated nudges within a second are one undo step: the saved step changes under the saved point
    let mut e = Editor::default();
    draw(&mut e, 100.0, 100.0);
    e.select_all();
    e.key_at("ArrowLeft", 8, 1000.0);
    e.name_steps();
    e.mark_saved();
    assert!(!e.dirty());
    e.key_at("ArrowLeft", 8, 1200.0);
    assert!(e.dirty());
    e.undo();
    assert!(
        e.dirty(),
        "the merged step is gone, the saved state with it"
    );
}

#[test]
fn an_empty_drawing_never_asks_even_when_it_is_dirty() {
    let mut e = Editor::default();
    draw(&mut e, 100.0, 100.0);
    e.clear();
    assert!(e.dirty(), "the clear is a step after the start");
    assert!(
        !e.unsaved() && e.guard() == Guard::Go,
        "nothing is drawn: nothing to lose"
    );
}

#[test]
fn new_refuses_unsaved_work_unless_discard_and_is_clean_after() {
    let mut e = Editor::default();
    draw(&mut e, 100.0, 100.0);
    assert_eq!(e.new_drawing(false), Err("error.draw2.unsaved"));
    assert_eq!(e.doc().atoms.len(), 1, "a refusal changes nothing");
    let flags = e.new_drawing(true).unwrap();
    assert!(flags & CHANGED != 0);
    assert!(e.doc().is_empty() && !e.dirty());
    // Undo brings the old drawing back, and that differs from the new one
    e.undo();
    assert_eq!(e.doc().atoms.len(), 1);
    assert!(e.dirty() && e.unsaved());
    // after a save the new drawing needs no discard
    e.mark_saved();
    assert!(e.new_drawing(false).is_ok() && e.doc().is_empty());
}

#[test]
fn open_replaces_the_drawing_with_an_empty_history_and_is_clean() {
    let mut a = Editor::default();
    draw(&mut a, 100.0, 100.0);
    let saved = a.doc().to_json();
    let mut e = Editor::default();
    draw(&mut e, 300.0, 300.0);
    assert_eq!(e.open_drawing(&saved, false), Err("error.draw2.unsaved"));
    assert_eq!(e.doc().atoms.len(), 1);
    e.open_drawing(&saved, true).unwrap();
    assert_eq!(e.doc().to_json(), saved);
    assert!(!e.dirty() && !e.session.can_undo());
    // a text that is not a document is refused and changes nothing
    assert!(e.open_drawing("not a document", true).is_err());
    assert_eq!(e.doc().to_json(), saved);
}

#[test]
fn the_answers_say_what_the_page_does() {
    let does = |s: &str| Choice::parse(s).map(Choice::does);
    assert_eq!(does("save"), Some("save"));
    assert_eq!(does("discard"), Some("go"));
    assert_eq!(does("cancel"), Some("stop"));
    assert_eq!(does("maybe"), None);
}

#[test]
fn the_replay_says_dirty_and_the_commands_follow_the_same_rules() {
    let atom = json!({"op":"atom.add","el":"N","x":3.0,"y":0.0});
    // a start text is clean, an edit is dirty, undo back is clean
    assert_eq!(result(json!({"import":"CC","report":true}))["dirty"], false);
    assert_eq!(
        result(json!({"import":"CC","log":[atom],"report":true}))["dirty"],
        true
    );
    let undone = json!({"import":"CC","log":[atom],"commands":[{"command":"undo:1","args":{}}],"report":true});
    assert_eq!(result(undone)["dirty"], false);
    // new and open on unsaved work: a verdict with the key in a report, an error without
    let new = json!({"import":"CC","log":[atom],"commands":[{"command":"new","args":{}}]});
    let mut with_report = new.clone();
    with_report["report"] = json!(true);
    let r = result(with_report);
    assert_eq!(r["ok"], false);
    assert_eq!(r["failed"]["key"], "error.draw2.unsaved");
    assert_eq!(
        mzdraw::replay::replay(&new.to_string()).unwrap_err().key,
        "error.draw2.unsaved"
    );
    // after a save, or with discard, they run
    for ok in [
        json!([{"command":"save","args":{}},{"command":"new","args":{}}]),
        json!([{"command":"new","args":{"discard":true}}]),
    ] {
        let r = result(json!({"import":"CC","log":[atom],"commands":ok,"report":true}));
        assert_eq!(
            (r["ok"].clone(), r["dirty"].clone()),
            (json!(true), json!(false))
        );
        assert!(r["out"].as_str().unwrap().contains(r#""atoms":[]"#));
    }
    // the same arguments, wrong: a request error
    for bad in [json!({"discard": 1}), json!({"extra": true})] {
        let q = json!({"commands":[{"command":"new","args":bad}]});
        assert_eq!(
            mzdraw::replay::replay(&q.to_string()).unwrap_err().key,
            "error.draw2.request"
        );
    }
}

#[test]
fn only_the_native_document_is_a_saved_point() {
    let native = Editor::save_kind();
    for lossy in [
        "molfile",
        "molfile-v3000",
        "smiles",
        "sdf",
        "svg",
        "png",
        "pdf",
        "ket",
        "cdxml",
        "cml",
        "log",
    ] {
        let mut e = Editor::default();
        draw(&mut e, 100.0, 100.0);
        assert!(!e.exported(lossy), "{lossy}");
        assert!(
            e.dirty() && e.unsaved(),
            "a {lossy} file leaves the drawing unsaved"
        );
    }
    let mut e = Editor::default();
    draw(&mut e, 100.0, 100.0);
    assert!(e.exported(native));
    assert!(!e.dirty(), "the native document is the saved point");
    // the command: no format is the native document; a named lossy format changes nothing
    let atom = json!({"op":"atom.add","el":"N","x":3.0,"y":0.0});
    for (format, clean) in [
        (json!(null), true),
        (json!("svg"), false),
        (json!("smiles"), false),
    ] {
        let args = if format.is_null() {
            json!({})
        } else {
            json!({ "format": format })
        };
        let r = result(
            json!({"import":"CC","log":[atom],"commands":[{"command":"save","args":args}],"report":true}),
        );
        assert_eq!(r["dirty"], !clean, "{format}");
    }
}

#[test]
fn save_with_an_unknown_format_is_a_bad_request() {
    for format in [json!("bogus"), json!(3)] {
        let q = json!({"commands":[{"command":"save","args":{"format":format}}]});
        let key = mzdraw::replay::replay(&q.to_string()).unwrap_err().key;
        assert_eq!(key, "error.draw2.request", "{format}");
    }
    assert!(
        Editor::is_save_kind("svg")
            && Editor::is_save_kind(Editor::save_kind())
            && !Editor::is_save_kind("bogus")
    );
}
