//! An agent's proposal (`proposal.rs`): held apart, accepted or rejected hunk by hunk, one undo step each; the bytes of the result are
//! those of `apply` of the same hunks.

use mzdraw::diff::hunks;
use mzdraw::editor::{CHANGED, CTRL, PROPOSAL};
use mzdraw::{Document, Editor, Op, Session};
use serde_json::{json, Value};
use std::time::Instant;

fn ops(v: Value) -> Vec<Op> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|o| Op::parse(o).unwrap())
        .collect()
}

/// Four two-atom pieces (atoms 1-8, bonds 9-12) in a row, drawn through the editor so that there is a history.
fn drawing() -> Editor {
    let mut e = Editor::default();
    let mut log = Vec::new();
    for k in 0..4u32 {
        let (a, b) = (1 + 2 * k, 2 + 2 * k);
        let x = f64::from(k) * 3.0;
        log.push(json!({"op":"atom.add","id":a,"el":"C","x":x,"y":0.0}));
        log.push(json!({"op":"atom.add","id":b,"el":"C","x":x + 1.0,"y":0.0}));
        log.push(json!({"op":"bond.add","id":9 + k,"a":a,"b":b,"o":1}));
    }
    e.session.run(&ops(Value::Array(log))).unwrap();
    e
}

/// A patch with four hunks, one per piece.
fn patch() -> Vec<Op> {
    ops(json!([
        {"op":"atom.set","id":1,"el":"N"},
        {"op":"bond.set","id":10,"o":2},
        {"op":"atom.set","id":5,"chg":1},
        {"op":"atom.add","id":13,"el":"O","x":9.0,"y":1.0},
        {"op":"bond.add","id":14,"a":8,"b":13,"o":1},
        {"op":"move","atoms":[7],"dx":0.0,"dy":0.5}
    ]))
}

fn applied(a: &Document, hs: &[usize], patch: &[Op]) -> String {
    let hunks = hunks(a, patch);
    let mut s = Session::new(a.clone());
    for &n in hs {
        s.run(&hunks[n - 1].ops).unwrap();
    }
    s.doc.to_json()
}

#[test]
fn accepting_hunks_two_and_four_gives_a_plus_those_hunks_and_two_undos_give_a() {
    let mut e = drawing();
    let a = e.doc().clone();
    let steps = e.session.steps();
    let p = patch();
    assert_eq!(e.propose(p.clone()).unwrap(), 4);
    // held apart: the drawing and the log are as they were
    assert_eq!(e.doc().to_json(), a.to_json());
    assert_eq!(e.session.steps(), steps);
    e.accept_hunk(2).unwrap();
    e.accept_hunk(4).unwrap();
    let want = applied(&a, &[2, 4], &p);
    assert_eq!(e.doc().to_json(), want);
    assert_eq!(e.session.steps(), steps + 2);
    // the log carries exactly the operations of the two hunks
    let hs = hunks(&a, &p);
    let tail: Vec<Op> = [&hs[1], &hs[3]]
        .iter()
        .flat_map(|h| h.ops.clone())
        .collect();
    let log = e.session.log();
    assert_eq!(log[log.len() - tail.len()..], tail[..]);
    // the two others are still open and can still be accepted
    assert_eq!(e.current_hunk(), 1);
    e.accept_hunk(3).unwrap();
    e.accept_hunk(1).unwrap();
    assert!(e.proposal_json() == "null");
    assert_eq!(e.doc().to_json(), applied(&a, &[2, 4, 3, 1], &p));
    for _ in 0..4 {
        assert!(e.undo() != 0);
    }
    assert_eq!(e.doc().to_json(), a.to_json());
}

#[test]
fn two_undos_after_two_accepts_return_a() {
    let mut e = drawing();
    let a = e.doc().to_json();
    e.propose(patch()).unwrap();
    e.accept_hunk(2).unwrap();
    e.accept_hunk(4).unwrap();
    e.undo();
    e.undo();
    assert_eq!(e.doc().to_json(), a);
}

#[test]
fn reject_all_leaves_the_document_and_the_log_unchanged() {
    let mut e = drawing();
    let (doc, log, steps) = (e.doc().to_json(), e.session.log_json(), e.session.steps());
    e.propose(patch()).unwrap();
    e.reject_hunk(3).unwrap();
    assert!(e.proposal_json().contains("\"n\":4"));
    e.reject_all().unwrap();
    assert_eq!(e.proposal_json(), "null");
    assert_eq!(
        (e.doc().to_json(), e.session.log_json(), e.session.steps()),
        (doc, log, steps)
    );
}

#[test]
fn rejecting_the_last_open_hunk_ends_the_proposal() {
    let mut e = drawing();
    e.propose(patch()).unwrap();
    for n in 1..=3 {
        e.reject_hunk(n).unwrap();
    }
    assert!(e.proposal_json().contains("\"cur\":4"));
    e.reject_hunk(4).unwrap();
    assert_eq!(e.proposal_json(), "null");
    assert_eq!(e.accept_hunk(1).unwrap_err().key, "error.draw2.noProposal");
}

#[test]
fn accept_all_is_one_undo_step_with_every_open_hunk_in_order() {
    let mut e = drawing();
    let a = e.doc().clone();
    let base = e.session.steps();
    let p = patch();
    e.propose(p.clone()).unwrap();
    e.reject_hunk(2).unwrap();
    e.accept_all().unwrap();
    assert_eq!(e.doc().to_json(), applied(&a, &[1, 3, 4], &p));
    assert_eq!(e.session.steps(), base + 1);
    e.undo();
    assert_eq!(e.doc().to_json(), a.to_json());
}

#[test]
fn a_patch_that_does_not_apply_or_is_too_big_is_refused_and_nothing_is_held() {
    let mut e = drawing();
    let bad = ops(json!([{"op":"atom.set","id":99,"el":"N"}]));
    assert_eq!(e.propose(bad).unwrap_err().key, "error.draw2.noAtom");
    assert_eq!(
        e.propose(Vec::new()).unwrap_err().key,
        "error.draw2.proposalEmpty"
    );
    let big: Vec<Op> = (0..mzdraw::proposal::MAX_OPS as u32 + 1)
        .map(|i| {
            Op::parse(&json!({"op":"atom.add","id":100 + i,"el":"C","x":0.0,"y":0.0})).unwrap()
        })
        .collect();
    let at = Instant::now();
    assert_eq!(e.propose(big).unwrap_err().key, "error.draw2.proposalSize");
    assert!(at.elapsed().as_millis() < 500);
    assert_eq!(e.proposal_json(), "null");
    // a hunk that is not there
    e.propose(patch()).unwrap();
    assert_eq!(
        e.accept_hunk(5).unwrap_err().key,
        "error.draw2.proposalHunk"
    );
    assert_eq!(
        e.accept_hunk(0).unwrap_err().key,
        "error.draw2.proposalHunk"
    );
    e.accept_hunk(1).unwrap();
    assert_eq!(
        e.accept_hunk(1).unwrap_err().key,
        "error.draw2.proposalHunk"
    );
}

#[test]
fn a_new_proposal_replaces_the_old_and_another_edit_makes_it_stale() {
    let mut e = drawing();
    e.propose(patch()).unwrap();
    e.propose(ops(json!([{"op":"atom.set","id":1,"el":"O"}])))
        .unwrap();
    assert_eq!(e.proposal_json().matches("\"n\"").count(), 1);
    // an edit of the person: the hunks were written against another drawing
    e.session
        .run(&ops(json!([{"op":"atom.set","id":3,"el":"S"}])))
        .unwrap();
    assert_eq!(e.proposal_json(), "null");
    assert!(e.proposal_marks().is_empty());
    assert_eq!(e.accept_hunk(1).unwrap_err().key, "error.draw2.noProposal");
}

#[test]
fn keys_walk_accept_and_reject() {
    let mut e = drawing();
    let a = e.doc().to_json();
    let base = e.session.steps();
    e.propose(patch()).unwrap();
    assert!(e.key("]", 0) & PROPOSAL != 0);
    assert_eq!(e.current_hunk(), 2);
    e.key("[", 0);
    e.key("[", 0);
    assert_eq!(e.current_hunk(), 4, "the ring wraps");
    // Delete rejects hunk 4 and the keyboard moves on
    e.key("Delete", 0);
    assert_eq!(e.current_hunk(), 1);
    e.key("]", 0);
    assert!(e.key("Enter", 0) & CHANGED != 0);
    assert_eq!(e.session.steps(), base + 1);
    assert_ne!(e.doc().to_json(), a);
    // Ctrl+Enter accepts what is left, Ctrl+Delete would have rejected it
    e.key("Enter", CTRL);
    assert_eq!(e.proposal_json(), "null");
    let mut f = drawing();
    f.propose(patch()).unwrap();
    f.key("Backspace", CTRL);
    assert_eq!((f.proposal_json().as_str(), f.doc().to_json()), ("null", a));
}

#[test]
fn a_tap_on_a_number_gives_it_the_keyboard_and_the_release_is_swallowed() {
    let mut e = drawing();
    e.propose(patch()).unwrap();
    let marks = e.proposal_marks();
    let at = |n: u32| {
        marks.iter().find_map(|m| match m {
            mzdraw::scene::Mark::Num { p, n: k, .. } if *k == n => Some(e.view.to_px(p.0, p.1)),
            _ => None,
        })
    };
    let (x, y) = at(3).unwrap();
    // a finger
    let f = e.pointer_event(0, 1, x + 3.0, y - 2.0, 2, 0, 0.0);
    assert!(f & PROPOSAL != 0);
    assert_eq!(e.current_hunk(), 3);
    let steps = e.session.steps();
    e.pointer_event(2, 1, x + 3.0, y - 2.0, 2, 0, 40.0);
    assert_eq!(e.session.steps(), steps, "the tool did not act");
    // a press elsewhere is the tool's
    e.pointer_event(0, 1, 5.0, 5.0, 0, 0, 100.0);
}

#[test]
fn the_scene_marks_added_removed_and_changed_things_and_numbers_each_hunk() {
    use mzdraw::scene::{colour, Mark};
    let mut e = drawing();
    e.propose(ops(json!([
        {"op":"atom.set","id":1,"el":"N"},
        {"op":"remove","atoms":[3,4],"bonds":[],"objects":[]},
        {"op":"atom.add","id":13,"el":"O","x":9.0,"y":1.0}
    ])))
    .unwrap();
    let marks = e.proposal_marks();
    let nums = marks
        .iter()
        .filter(|m| matches!(m, Mark::Num { .. }))
        .count();
    assert_eq!(nums, 3);
    assert!(marks
        .iter()
        .any(|m| matches!(m, Mark::Dot { c, .. } if *c == colour::ACCENT)));
    assert!(marks
        .iter()
        .any(|m| matches!(m, Mark::Line { c, heavy: true, .. } if *c == colour::WARN)));
    assert!(marks
        .iter()
        .any(|m| matches!(m, Mark::Ring { c, .. } if *c == colour::ACCENT)));
    // the scene is longer than the plain one, and the plain one comes back when the proposal ends
    let with = e.scene().data.len();
    e.reject_all().unwrap();
    assert!(with > e.scene().data.len());
}

#[test]
fn a_hunk_is_said_in_words() {
    let mut e = drawing();
    e.propose(patch()).unwrap();
    let v: Value = serde_json::from_str(&e.proposal_json()).unwrap();
    let said = |i: usize| v["hunks"][i]["say"].to_string();
    assert_eq!(said(0), r#"[["draw2.hunk.atomsChanged","1"]]"#);
    assert_eq!(said(1), r#"[["draw2.hunk.bondsChanged","10"]]"#);
    assert_eq!(
        said(3),
        r#"[["draw2.hunk.atomsAdded","13"],["draw2.hunk.bondsAdded","14"],["draw2.hunk.atomsMoved","7"]]"#
    );
}

#[test]
fn a_big_proposal_is_held_shown_and_accepted_in_time() {
    // as many independent hunks as the cap allows: nothing in the marks, the numbers or the accept may be quadratic in the hunks
    let n = mzdraw::proposal::MAX_OPS as u32;
    let big: Vec<Op> = (0..n)
        .map(|i| {
            Op::parse(&json!({"op":"atom.add","id":1 + i,"el":"C","x":f64::from(i % 50),"y":f64::from(i / 50)}))
                .unwrap()
        })
        .collect();
    let mut e = Editor::default();
    let at = Instant::now();
    assert_eq!(e.propose(big).unwrap(), n as usize);
    let scene = e.scene();
    let json = e.proposal_json();
    assert!(scene.data.len() > 7 * n as usize && json.len() > n as usize);
    e.accept_all().unwrap();
    assert_eq!(e.doc().atoms.len(), n as usize);
    let ms = at.elapsed().as_millis();
    assert!(ms < 3000, "{ms} ms for {n} hunks");
}

#[test]
fn the_page_flags_are_distinct() {
    assert_eq!(
        PROPOSAL & (mzdraw::editor::MENU | mzdraw::editor::ASK | mzdraw::editor::PALETTES),
        0
    );
}
