//! The text panel (`editor_text.rs`, `textedit.rs`): a selection inside a text, the keys and the pointer that move it, the style
//! buttons over a range, the pending style of a caret, typing as one undo step, the exact undo of every change, the replay of the log
//! of a session (rule 15) and the preferences for a new text. Synthetic texts only.

use mzdraw::editor::{CHANGED, SHIFT};
use mzdraw::{Document, Editor};
use serde_json::Value;

const DOC: &str = r#"{"format":"mzdraw/1","next":3,"atoms":[],"bonds":[],"objects":[{"id":2,"t":"text","x":0.0,"y":0.0,"text":"Hello world"}]}"#;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.open(DOC).unwrap();
    e
}

/// Selects the text by a click with the select tool, at a point on its first line.
fn select(e: &mut Editor) {
    e.set_tool("select");
    let (x, y) = e.view.to_px(0.0, 0.0);
    e.pointer_down(x + 5.0, y, 0, 0);
    e.pointer_up(x + 5.0, y);
}

fn panel(e: &Editor) -> Value {
    serde_json::from_str(&e.text_json()).unwrap()
}

fn text_of(e: &Editor) -> String {
    e.doc().object(2).map_or(String::new(), |o| o.text.clone())
}

fn right(e: &mut Editor, n: usize, shift: bool) {
    for _ in 0..n {
        e.key("ArrowRight", if shift { SHIFT } else { 0 });
    }
}

#[test]
fn a_selected_text_shows_the_panel_and_a_button_styles_all_of_it_with_an_exact_undo() {
    let mut e = editor();
    assert_eq!(e.text_json(), "null", "nothing selected: no panel");
    select(&mut e);
    let p = panel(&e);
    assert_eq!(
        (p["id"].as_u64(), p["editing"].as_bool()),
        (Some(2), Some(false))
    );
    assert_eq!(
        (p["from"].as_u64(), p["to"].as_u64(), p["len"].as_u64()),
        (Some(0), Some(11), Some(11))
    );
    assert_eq!(p["style"]["b"], false);
    let before = e.doc().to_json();
    assert_ne!(e.text_style("b", "") & CHANGED, 0);
    assert!(e.doc().object(2).unwrap().runs.iter().all(|r| r.b));
    assert_eq!(panel(&e)["style"]["b"], true);
    // a second press turns it off again: the text is plain once more (no runs left)
    e.text_style("b", "");
    assert!(e.doc().object(2).unwrap().runs.is_empty());
    assert_eq!(e.doc().to_json(), before);
    // and undo is exact, step by step
    e.text_style("sup", "");
    let styled = e.doc().to_json();
    e.text_style("colour", "#D00");
    e.undo();
    assert_eq!(e.doc().to_json(), styled);
    e.undo();
    assert_eq!(e.doc().to_json(), before);
    e.redo();
    e.redo();
    assert!(e.doc().to_json().contains("#dd0000"));
}

#[test]
fn a_range_by_shift_and_arrows_is_the_only_part_that_changes() {
    let mut e = editor();
    select(&mut e);
    e.key("Enter", 0);
    assert!(e.text_editing());
    assert_eq!(panel(&e)["from"], 11, "the caret goes to the end");
    e.key("Home", 0);
    right(&mut e, 5, true);
    let p = panel(&e);
    assert_eq!((p["from"].as_u64(), p["to"].as_u64()), (Some(0), Some(5)));
    e.text_style("b", "");
    let runs = &e.doc().object(2).unwrap().runs;
    assert_eq!(runs.len(), 2);
    assert!(runs[0].b && runs[0].t == "Hello" && !runs[1].b && runs[1].t == " world");
    // the panel reads the style of the range: bold; over the whole text it is mixed
    assert_eq!(panel(&e)["style"]["b"], true);
    e.key("a", 2);
    assert_eq!(panel(&e)["style"]["b"], Value::Null);
    // a toggle over the mixed range turns it all on, the next turns it all off
    e.text_style("b", "");
    assert!(e.doc().object(2).unwrap().runs.iter().all(|r| r.b));
    e.text_style("b", "");
    assert!(e.doc().object(2).unwrap().runs.is_empty());
    // the shortcuts of the panel: Ctrl+I, Ctrl+U, Ctrl+, and Ctrl+.
    e.key("Home", 0);
    right(&mut e, 1, true);
    for k in ["i", "u", ","] {
        e.key(k, 2);
    }
    let r = &e.doc().object(2).unwrap().runs[0];
    assert!(r.i && r.u && r.sub && !r.b);
    e.key(".", 2);
    let r = &e.doc().object(2).unwrap().runs[0];
    assert!(r.sup && !r.sub, "sub and sup exclude each other");
}

#[test]
fn typing_is_one_undo_step_and_takes_the_style_before_the_caret() {
    let mut e = editor();
    select(&mut e);
    e.text_style("b", "");
    let bold = e.doc().to_json();
    e.key("Enter", 0);
    let steps = e.session.steps();
    for c in ["!", "?", "x"] {
        e.key(c, 0);
    }
    assert_eq!(text_of(&e), "Hello world!?x");
    assert_eq!(
        e.session.steps(),
        steps + 1,
        "the typed characters join one step"
    );
    assert!(
        e.doc().object(2).unwrap().runs.iter().all(|r| r.b),
        "bold goes on"
    );
    e.key("Backspace", 0);
    assert_eq!(text_of(&e), "Hello world!?");
    // a move ends the run: the next character is a step of its own
    e.key("ArrowLeft", 0);
    e.key("y", 0);
    assert_eq!(text_of(&e), "Hello world!y?");
    assert_eq!(e.session.steps(), steps + 2);
    e.undo();
    assert_eq!(text_of(&e), "Hello world!?");
    e.undo();
    assert_eq!(e.doc().to_json(), bold);
    // Enter inside the text is a line break, and the panel works on the second line
    e.redo();
    e.redo();
    e.key("Enter", 0);
    e.key("z", 0);
    assert_eq!(text_of(&e), "Hello world!y\nz?");
    e.key("Escape", 0);
    assert!(
        !e.text_editing(),
        "Escape leaves the editing, the text stays selected"
    );
    assert!(panel(&e)["id"] == 2);
}

#[test]
fn a_style_button_with_a_caret_and_no_range_is_the_style_of_the_next_characters() {
    let mut e = editor();
    select(&mut e);
    e.key("Enter", 0);
    let before = e.doc().to_json();
    e.text_style("i", "");
    assert_eq!(
        e.doc().to_json(),
        before,
        "no range: nothing is written yet"
    );
    assert_eq!(
        panel(&e)["style"]["i"],
        true,
        "the panel shows the pending style"
    );
    e.key("A", SHIFT);
    e.key("B", SHIFT);
    let runs = &e.doc().object(2).unwrap().runs;
    assert_eq!((runs.len(), runs[1].t.as_str(), runs[1].i), (2, "AB", true));
    // the colour flyout colours the range of an edited text, not the object
    e.key("Home", 0);
    right(&mut e, 3, true);
    assert_ne!(e.colour("#d00000") & CHANGED, 0);
    assert_eq!(e.doc().object(2).unwrap().runs[0].color, "#d00000");
    assert_eq!(e.doc().object(2).unwrap().col(), "");
    assert_eq!(e.selection_colour().as_deref(), Some("#d00000"));
    // a size and a font from the lists
    e.text_style("size", "150");
    e.text_style("font", "courier");
    let r = &e.doc().object(2).unwrap().runs[0];
    assert_eq!((r.sz, r.font.as_str()), (1.5, "Courier New"));
    assert_eq!(panel(&e)["style"]["size"], 150);
    assert_eq!(e.text_style("size", "9000") & CHANGED, 0);
    assert_eq!(e.error, "error.draw2.textStyle");
    assert_eq!(e.text_style("font", "Comic Sans") & CHANGED, 0);
}

#[test]
fn the_last_character_removes_the_text_in_one_step_and_undo_brings_it_back() {
    let mut e = editor();
    select(&mut e);
    let before = e.doc().to_json();
    e.key("F2", 0);
    e.key("a", 2);
    e.key("Delete", 0);
    assert!(e.doc().object(2).is_none());
    assert!(!e.text_editing());
    e.undo();
    assert_eq!(e.doc().to_json(), before);
}

#[test]
fn the_pointer_sets_the_caret_and_drags_a_range_and_a_press_outside_ends_the_editing() {
    let mut e = editor();
    select(&mut e);
    e.key("Enter", 0);
    let (x, y) = e.view.to_px(0.0, 0.0);
    // a press near the start puts the caret there, a drag to the right grows the range
    e.pointer_down(x + 1.0, y, 0, 0);
    e.pointer_move(x + 60.0, y, 0);
    e.pointer_up(x + 60.0, y);
    let p = panel(&e);
    assert_eq!(p["from"], 0);
    assert!(p["to"].as_u64().unwrap() >= 3, "{p}");
    assert!(p["editing"] == true);
    e.text_style("u", "");
    assert!(e.doc().object(2).unwrap().runs[0].u);
    // Shift+press extends the range
    e.pointer_down(x + 1.0, y, 0, 0);
    e.pointer_up(x + 1.0, y);
    e.pointer_down(x + 40.0, y, 0, SHIFT);
    e.pointer_up(x + 40.0, y);
    assert!(panel(&e)["to"].as_u64().unwrap() > 0);
    // a press far from the text leaves the editing, and goes on as a press of the tool
    let steps = e.session.steps();
    e.pointer_down(x + 300.0, y + 200.0, 0, 0);
    e.pointer_up(x + 300.0, y + 200.0);
    assert!(!e.text_editing());
    assert_eq!(
        e.session.steps(),
        steps,
        "the selection is no step of the history"
    );
    // a double click selects the word and edits
    e.text_double(x + 5.0, y);
    let p = panel(&e);
    assert_eq!(
        (p["editing"].as_bool(), p["from"].as_u64(), p["to"].as_u64()),
        (Some(true), Some(0), Some(5))
    );
}

#[test]
fn a_range_that_crosses_the_character_cap_never_makes_a_text_the_document_refuses() {
    let long = "a".repeat(996);
    let doc = DOC.replace("Hello world", &long);
    let mut e = Editor::default();
    // a plain text is at most 200 characters; one of runs may have 1000
    let runs = format!(r#"[{{"t":"{long}"}}]"#);
    let doc = doc.replace(
        &format!(r#""text":"{long}""#),
        &format!(r#""runs":{runs},"text":"{long}""#),
    );
    e.open(&doc).unwrap();
    select(&mut e);
    e.key("Enter", 0);
    e.text_input("bcdefgh");
    assert_eq!(
        e.doc().object(2).unwrap().text.chars().count(),
        1000,
        "cut at the cap"
    );
    assert_eq!(e.doc().object(2).unwrap().text.chars().last(), Some('e'));
    e.text_input("z");
    assert_eq!(e.error, "error.draw2.runsCount");
    assert_eq!(e.doc().object(2).unwrap().text.chars().count(), 1000);
    // styling a range past the end is the rest of the text, and the document stays valid
    e.key("a", 2);
    e.text_style("b", "");
    assert!(e.doc().object(2).unwrap().valid());
    let again = Document::from_json(&e.doc().to_json()).unwrap();
    assert_eq!(again.to_json(), e.doc().to_json());
}

#[test]
fn the_log_of_a_panel_session_replays_to_the_same_document() {
    let mut e = editor();
    select(&mut e);
    e.text_style("b", "");
    e.key("Enter", 0);
    e.key("Home", 0);
    right(&mut e, 5, true);
    e.text_style("colour", "#1f3fd0");
    e.text_style("sub", "");
    e.key("End", 0);
    e.text_input("\u{b0}C");
    e.undo();
    e.redo();
    let log = e.session.log_json();
    let mut s = mzdraw::Session::new(Document::from_json(DOC).unwrap());
    s.replay(&log).unwrap();
    assert_eq!(s.doc.to_json(), e.doc().to_json());
    assert!(log.contains(r#""op":"obj.set""#) && log.contains(r#""runs""#));
}

#[test]
fn the_preferences_give_a_new_text_its_font_and_size() {
    let mut e = Editor::default();
    assert_eq!(e.get("text.font").as_deref(), Some("default"));
    assert_eq!(e.get("text.size").as_deref(), Some("100"));
    e.set("text.font=times").unwrap();
    e.set("text.size=150").unwrap();
    assert_eq!(e.get("text.font").as_deref(), Some("times"));
    e.set_tool("text:Pd(OAc)_2");
    let (x, y) = e.view.to_px(1.0, 1.0);
    e.pointer_down(x, y, 0, 0);
    e.pointer_up(x, y);
    let o = e.doc().objects.last().unwrap().clone();
    assert!(o
        .runs
        .iter()
        .all(|r| r.font == "Times New Roman" && r.sz == 1.5));
    assert!(
        o.runs.iter().any(|r| r.sub),
        "the markup still gives its scripts"
    );
    // the settings file keeps the preferences and a key this version does not know, and a file without the object is the defaults
    let saved = e.settings_json();
    let v: Value = serde_json::from_str(&saved).unwrap();
    assert_eq!(v["text"]["font"], "times");
    assert_eq!(v["text"]["size"], "150");
    let with_future = saved.replace(r#""text":{"#, r#""text":{"future":[1,2],"#);
    let mut f = Editor::default();
    f.set_style(&with_future).unwrap();
    assert_eq!(f.get("text.font").as_deref(), Some("times"));
    assert!(f.settings_json().contains(r#""future":[1,2]"#));
    let mut d = Editor::default();
    d.set_style(&saved.replace(&v["text"].to_string(), "{}"))
        .unwrap();
    assert_eq!(d.get("text.size").as_deref(), Some("100"));
    // a value that is none is refused and changes nothing
    assert_eq!(
        e.set("text.font=comic").unwrap_err(),
        "error.draw2.textPref"
    );
    assert_eq!(e.set("text.size=12").unwrap_err(), "error.draw2.textPref");
    assert_eq!(e.set("text.colour=red").unwrap_err(), "error.draw2.set");
    // the defaults leave a new text plain, as before
    let mut p = Editor::default();
    p.set_tool("text:Hello");
    let (x, y) = p.view.to_px(0.0, 0.0);
    p.pointer_down(x, y, 0, 0);
    p.pointer_up(x, y);
    assert!(p.doc().objects[0].runs.is_empty());
    // reset of the group puts the defaults back
    let cmds = e.reset_commands("text").unwrap();
    assert_eq!(cmds, ["text.font=default", "text.size=100"]);
}

#[test]
fn a_stored_settings_file_with_a_text_object_this_version_cannot_read_loses_only_that_object() {
    let mut e = Editor::default();
    let (flags, note) = e
        .set_style_stored(r#"{"format":"mzdraw-settings/1","style":{"preset":"mzlab"},"text":{"font":"comic","size":"150"}}"#)
        .unwrap();
    assert_ne!(flags, 0);
    assert_eq!(note, "error.draw2.textPref");
    assert_eq!(e.get("text.font").as_deref(), Some("default"));
    // an explicit setting of a bad value stays an error
    assert!(e.set("text.font=comic").is_err());
}
