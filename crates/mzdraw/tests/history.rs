//! The history of the session (`Editor::history_json`, the commands `undo:<n>`, `redo:<n>`, `jump:<n>`, the output `history`): the steps
//! are listed newest first with the words the live line said; a jump is a run of undo or redo, so the drawing after it is the drawing
//! after that many steps.

use mzdraw::editor::{ALT, CHANGED, MOUSE};
use mzdraw::Editor;
use serde_json::{json, Value};

/// One step the way the page makes it: the tool, a click, and the naming the bridge does after each action.
fn step(e: &mut Editor, tool: &str, at: (f64, f64)) {
    e.set_tool(tool);
    e.pointer_down(at.0, at.1, MOUSE, 0);
    e.pointer_up(at.0, at.1);
    e.name_steps();
}

fn five(e: &mut Editor) {
    for (i, tool) in ["atom:C", "atom:N", "atom:O", "atom:S", "atom:P"]
        .iter()
        .enumerate()
    {
        step(e, tool, (100.0 + 90.0 * i as f64, 120.0 + 30.0 * i as f64));
    }
}

fn history(e: &Editor) -> Value {
    serde_json::from_str(&e.history_json()).unwrap()
}

fn doc(e: &Editor) -> String {
    e.doc().to_json()
}

#[test]
fn five_steps_listed_newest_first_with_the_current_one_marked() {
    let mut e = Editor::default();
    five(&mut e);
    let h = history(&e);
    assert_eq!(h["format"], "history/1");
    assert_eq!(h["at"], 5);
    let steps = h["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 5);
    let index: Vec<u64> = steps.iter().map(|s| s["index"].as_u64().unwrap()).collect();
    assert_eq!(index, [5, 4, 3, 2, 1]);
    assert_eq!(steps[0]["state"], "current");
    assert!(steps[1..].iter().all(|s| s["state"] == "done"));
    // each step carries the words the live line said when it was made, not a generic name: the element of the atom
    let said: Vec<(&str, &str)> = steps
        .iter()
        .map(|s| (s["say"].as_str().unwrap(), s["what"].as_str().unwrap()))
        .collect();
    assert_eq!(said[0], ("draw2.say.atom", "P"));
    assert_eq!(said[4], ("draw2.say.atom", "C"));
    assert_ne!(said[0], said[1], "the steps are not all named alike");
}

#[test]
fn back_five_and_forward_five_is_the_same_document() {
    let mut e = Editor::default();
    five(&mut e);
    let before = doc(&e);
    let at_start = {
        let mut f = Editor::default();
        f.name_steps();
        doc(&f)
    };
    assert!(e.jump_to(0).unwrap() & CHANGED != 0);
    assert_eq!(doc(&e), at_start, "step 0 is the empty drawing");
    assert_eq!(e.doc().atoms.len(), 0);
    let h = history(&e);
    assert_eq!(h["at"], 0);
    assert_eq!(
        h["steps"].as_array().unwrap().len(),
        5,
        "the steps stay in the list"
    );
    assert!(h["steps"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["state"] == "undone"));
    e.jump_to(5).unwrap();
    assert_eq!(doc(&e), before);
    assert_eq!(history(&e)["steps"][0]["state"], "current");
}

#[test]
fn a_jump_leaves_the_drawing_as_that_step_made_it() {
    let mut e = Editor::default();
    five(&mut e);
    let log: Vec<Value> = serde_json::from_str(&e.save("log").unwrap()).unwrap();
    assert_eq!(log.len(), 5);
    for n in [0usize, 1, 2, 3, 4, 5, 2, 4, 1, 5, 0, 3] {
        e.jump_to(n).unwrap();
        // the first n steps of the log, replayed natively on an empty drawing
        let want =
            mzdraw::replay::replay(&json!({ "log": &log[..n], "out": "mzdraw" }).to_string())
                .unwrap();
        assert_eq!(doc(&e), want, "jump to step {n}");
        assert_eq!(history(&e)["at"], n);
    }
}

#[test]
fn a_new_step_after_an_undo_drops_the_redo_part() {
    let mut e = Editor::default();
    five(&mut e);
    e.jump_to(2).unwrap();
    assert_eq!(history(&e)["steps"].as_array().unwrap().len(), 5);
    step(&mut e, "atom:F", (700.0, 400.0));
    let h = history(&e);
    let steps = h["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 3, "two kept steps and the new one");
    assert_eq!(h["at"], 3);
    assert_eq!(
        (steps[0]["say"].as_str(), steps[0]["what"].as_str()),
        (Some("draw2.say.atom"), Some("F"))
    );
    assert_eq!(steps[0]["state"], "current");
    assert!(steps.iter().all(|s| s["state"] != "undone"));
    // and the forgotten steps cannot be reached
    assert_eq!(e.jump_to(4), Err("error.draw2.request"));
    assert_eq!(history(&e)["at"], 3, "a refused jump changes nothing");
}

#[test]
fn undo_n_and_redo_n_count_steps_and_stop_at_the_ends() {
    let mut e = Editor::default();
    five(&mut e);
    let full = doc(&e);
    e.undo_steps(2).unwrap();
    assert_eq!(history(&e)["at"], 3);
    e.undo_steps(99).unwrap();
    assert_eq!(history(&e)["at"], 0, "no more than there are");
    assert_eq!(e.redo_steps(3).map(|f| f & CHANGED != 0), Ok(true));
    assert_eq!(history(&e)["at"], 3);
    e.redo_steps(99).unwrap();
    assert_eq!(doc(&e), full);
    // nothing to undo or redo is not an error, and says nothing changed
    assert_eq!(e.redo_steps(1), Ok(0));
    assert_eq!(e.undo_steps(0), Err("error.draw2.request"));
}

#[test]
fn the_page_command_and_the_request_command_are_the_same_command() {
    let mut e = Editor::default();
    five(&mut e);
    assert!(
        Editor::is_history_command("jump:3")
            && Editor::is_history_command("undo:1")
            && Editor::is_history_command("redo:12")
    );
    assert!(
        !Editor::is_history_command("undo")
            && !Editor::is_history_command("undo:")
            && !Editor::is_history_command("undo:-1")
            && !Editor::is_history_command("jump:1x")
            && !Editor::is_history_command("pathway:tree")
    );
    assert_ne!(e.history_command("jump:3").unwrap() & CHANGED, 0);
    let by_page = doc(&e);
    let by_request = mzdraw::replay::replay(
        &json!({ "log": serde_json::from_str::<Value>(&e.save("log").unwrap()).unwrap(), "out": "mzdraw" }).to_string(),
    )
    .unwrap();
    assert_eq!(
        by_page, by_request,
        "the log keeps the steps still applied, three"
    );
    assert_eq!(e.doc().atoms.len(), 3);
    // a jump past the end is said in `error` and changes nothing
    e.history_command("jump:9").unwrap();
    assert_eq!(e.error, "error.draw2.request");
    assert_eq!(e.doc().atoms.len(), 3);
}

#[test]
fn a_joined_nudge_is_one_step_in_the_list() {
    // two key nudges within a second are one undo step: the list has one row for them, and a jump takes them back together
    let mut e = Editor::default();
    step(&mut e, "atom:C", (200.0, 200.0));
    e.set_tool("select");
    e.select_all();
    let x0 = e.doc().atoms[0].x;
    e.key_at("ArrowRight", ALT, 1000.0);
    e.name_steps();
    e.key_at("ArrowRight", ALT, 1200.0);
    e.name_steps();
    let h = history(&e);
    assert_eq!(h["steps"].as_array().unwrap().len(), 2);
    e.jump_to(1).unwrap();
    assert!(
        (e.doc().atoms[0].x - x0).abs() < 1e-9,
        "both presses went back together"
    );
    e.jump_to(2).unwrap();
    assert!(
        (e.doc().atoms[0].x - x0 - 0.2).abs() < 1e-9,
        "both presses came back"
    );
}

/// Random walks of jumps over a log of mixed steps (the property of the slice: a jump is undo and redo, so it is exact whatever the
/// route): after every jump the drawing is the one the first n steps give, and the history says where it is.
#[test]
fn random_jumps_always_land_on_the_drawing_of_that_step() {
    let mut e = Editor::default();
    five(&mut e);
    step(&mut e, "bond:2", (150.0, 150.0));
    e.set_tool("select");
    e.select_all();
    e.rotate(15.0);
    e.name_steps();
    e.delete_selection();
    e.name_steps();
    let total = history(&e)["at"].as_u64().unwrap() as usize;
    assert!(total >= 7);
    // the document after n steps, found by walking to n from the start
    let mut by_n = Vec::new();
    for n in 0..=total {
        e.jump_to(n).unwrap();
        by_n.push(doc(&e));
    }
    let mut seed = 0x2545F491u64;
    for _ in 0..200 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let n = ((seed >> 33) as usize) % (total + 1);
        e.jump_to(n).unwrap();
        assert_eq!(doc(&e), by_n[n], "jump to step {n}");
        assert_eq!(history(&e)["at"], n);
    }
}
