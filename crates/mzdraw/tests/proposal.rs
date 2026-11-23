//! An agent's proposal (`proposal.rs`): held apart, accepted or rejected hunk by hunk, one undo step each; the bytes of the result are
//! those of `apply` of the same hunks.

use mzdraw::diff::hunks;
use mzdraw::editor::{CHANGED, CTRL, PROPOSAL};
use mzdraw::{Document, Editor, Op, Session};
use serde_json::{json, Value};

#[path = "support/timing.rs"]
mod timing;

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
    // a refusal is cheaper than the work it saves: the same number of operations, one fewer, is held and marked for real
    let valid: Vec<Op> = big[..mzdraw::proposal::MAX_OPS].to_vec();
    let refused = best_with(
        1,
        5,
        || (drawing(), big.clone()),
        |(mut e, big)| {
            assert_eq!(e.propose(big).unwrap_err().key, "error.draw2.proposalSize");
        },
    );
    let held = best_with(
        1,
        5,
        || (drawing(), valid.clone()),
        |(mut e, v)| {
            e.propose(v).unwrap();
        },
    );
    assert!(
        refused <= held,
        "refusing {} operations cost {refused:.3} ms, holding {} cost {held:.3} ms",
        big.len(),
        valid.len()
    );
    assert_eq!(e.propose(big).unwrap_err().key, "error.draw2.proposalSize");
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
fn the_page_flags_are_distinct() {
    assert_eq!(
        PROPOSAL & (mzdraw::editor::MENU | mzdraw::editor::ASK | mzdraw::editor::PALETTES),
        0
    );
}

// ---- hostile sizes: the size is in the id lists and in the number of hunks, not only in the number of operations ----

/// A document of `n` carbons on a grid, in one chain (`chained`) or each alone, built from its JSON (no operation is run, so the
/// building is linear).
fn grid(n: u32, chained: bool) -> Document {
    let atoms: Vec<Value> = (1..=n)
        .map(|i| json!({"id": i, "el": "C", "x": f64::from(i % 100) * 2.0, "y": f64::from(i / 100) * 2.0}))
        .collect();
    let bonds: Vec<Value> = if chained {
        (1..n)
            .map(|i| json!({"id": n + i, "a": i, "b": i + 1, "o": 1}))
            .collect()
    } else {
        Vec::new()
    };
    let doc = json!({"format": "mzdraw/1", "next": 2 * n + 1, "atoms": atoms, "bonds": bonds});
    Document::from_json(&doc.to_string()).unwrap()
}

fn editor(doc: Document) -> Editor {
    let mut e = Editor::default();
    e.session = Session::new(doc);
    e
}

/// The least CPU time of the thread over seven runs of `f` after one that is not timed, in milliseconds (`support/timing.rs`: the
/// time the thread waited for a core on a busy machine is not counted, and noise only ever adds time).
fn best(mut f: impl FnMut()) -> f64 {
    timing::best_of(1, 7, &mut f).as_secs_f64() * 1000.0
}

/// The least CPU time, in milliseconds, of `runs` runs of `run` after `warm` that are not timed. Each run gets a fresh input from
/// `setup`, built outside the clock (a run may consume or change its input). Noise only ever adds time, so the least is the cost.
fn best_with<S>(warm: usize, runs: usize, setup: impl Fn() -> S, run: impl Fn(S)) -> f64 {
    for _ in 0..warm {
        run(setup());
    }
    (0..runs)
        .map(|_| {
            let s = setup();
            let w = timing::Watch::start();
            run(s);
            w.elapsed().as_secs_f64() * 1000.0
        })
        .fold(f64::MAX, f64::min)
}

/// The time of `run` on the input of `n` and of `4n` (built by `setup`, outside the clock) must not differ by more than a factor 8:
/// linear work gives 4 and quadratic work 16, and the bound has no floor, so it is the same at every speed; `n` is large enough that
/// quadratic work is seconds in a release build too.
fn scales<T>(what: &str, n: u32, setup: impl Fn(u32) -> T, run: impl Fn(&T)) {
    let (small, big) = (setup(n), setup(4 * n));
    let (a, b) = (best(|| run(&small)), best(|| run(&big)));
    assert!(
        b <= a * 8.0,
        "{what}: {a:.1} ms for {n}, {b:.1} ms for {}",
        4 * n
    );
}

/// One remove naming `n` atoms: held, marked (scene, words) and accepted, on a drawing of `n` atoms.
fn remove_all(n: u32) -> (Editor, Op) {
    let remove = Op::Remove {
        atoms: (1..=n).collect(),
        bonds: Vec::new(),
        objects: Vec::new(),
    };
    (editor(grid(n, true)), remove)
}

#[test]
fn one_remove_naming_every_atom_of_a_big_drawing_is_held_marked_and_accepted() {
    // the result, once, on the big drawing
    let n = 20_000;
    let (mut e, remove) = remove_all(n);
    assert_eq!(e.propose(vec![remove]).unwrap(), 1);
    let _ = (e.scene(), e.proposal_json());
    e.accept_hunk(1).unwrap();
    assert!(e.doc().atoms.is_empty());
    // the cost of the proposal layer: four times the atoms may cost at most eight times as much (linear work gives 4, quadratic work
    // 16); a ratio of two CPU times of this thread, so the speed or the load of the machine is not in it. `scene()` is called above
    // for its result and left out of the clock: drawing a plain chain of this many atoms is not linear in a debug build (it takes
    // as long without any proposal, see docs/built/flaky-tests-2.md); the marks and words of the proposal, which are the layer's own
    // share of the scene, are timed.
    let run = |(mut e, remove): (Editor, Op)| {
        assert_eq!(e.propose(vec![remove]).unwrap(), 1);
        let _ = (e.proposal_marks(), e.proposal_json());
        e.accept_hunk(1).unwrap();
    };
    let small = best_with(1, 3, || remove_all(n / 4), run);
    let big = best_with(1, 3, || remove_all(n), run);
    assert!(
        big <= small * 8.0,
        "one remove: {small:.1} ms for {} atoms, {big:.1} ms for {n}",
        n / 4
    );
}

/// One move of `n` atoms, over a drawing of `n` atoms (held, accepted, undone are timed apart): the three times in milliseconds.
fn move_all_times(n: u32) -> [f64; 3] {
    let mv = || Op::Move {
        atoms: (1..=n).collect(),
        objects: Vec::new(),
        dx: 1.0,
        dy: 0.0,
    };
    let mut t = [f64::MAX; 3];
    for run in 0..4 {
        let mut e = editor(grid(n, true));
        let at = timing::Watch::start();
        assert_eq!(e.propose(vec![mv()]).unwrap(), 1);
        let held = at.elapsed().as_secs_f64() * 1000.0;
        let before = e.doc().to_json();
        let at = timing::Watch::start();
        e.accept_all().unwrap();
        let accepted = at.elapsed().as_secs_f64() * 1000.0;
        let at = timing::Watch::start();
        assert!(e.undo() != 0);
        let undone = at.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(e.doc().to_json(), before);
        if run > 0 {
            // the first run is a warm-up
            t = [t[0].min(held), t[1].min(accepted), t[2].min(undone)];
        }
    }
    t
}

#[test]
fn one_move_of_all_atoms_is_one_hunk_and_the_ids_are_capped() {
    let n = 20_000;
    // four times the atoms: each of the three steps may cost at most eight times as much (linear 4, quadratic 16)
    let (small, big) = (move_all_times(n / 4), move_all_times(n));
    for (step, (a, b)) in ["held", "accepted", "undone"]
        .iter()
        .zip(small.iter().zip(big.iter()))
    {
        assert!(
            b <= &(a * 8.0),
            "{step}: {a:.1} ms for {} atoms moved, {b:.1} ms for {n}",
            n / 4
        );
    }
    // more ids than the cap in one operation is refused at once, however few operations there are: it costs less than holding a move
    // of as many ids as the drawing has atoms (the work it saves)
    let big_move = |k: u32| Op::Move {
        atoms: (1..=k).collect(),
        objects: Vec::new(),
        dx: 1.0,
        dy: 0.0,
    };
    let over = mzdraw::proposal::MAX_IDS as u32 + 1;
    let refused = best_with(
        1,
        5,
        || (editor(grid(n, true)), big_move(over)),
        |(mut e, mv)| {
            assert_eq!(
                e.propose(vec![mv]).unwrap_err().key,
                "error.draw2.proposalIds"
            );
        },
    );
    let held = best_with(
        1,
        5,
        || (editor(grid(n, true)), big_move(n)),
        |(mut e, mv)| {
            e.propose(vec![mv]).unwrap();
        },
    );
    assert!(
        refused <= held,
        "refusing {over} ids cost {refused:.3} ms, holding {n} cost {held:.3} ms"
    );
}

#[test]
fn grouping_and_diffing_scale_with_the_operations_and_the_ids() {
    // `diff::hunks` and `diff::diff` themselves (the proposal caps the operations; the functions do not). A search of the hunks, of the
    // groups of moves or of a pool for each operation or each id is quadratic here: 80 000 of them are billions of comparisons, seconds
    // in a release build too.
    // n operations in n hunks (one atom each)
    scales(
        "n operations in n hunks",
        20_000,
        |n| {
            (
                grid(n, false),
                (0..n)
                    .map(|i| Op::parse(&json!({"op":"atom.set","id":1 + i,"chg":1})).unwrap())
                    .collect::<Vec<Op>>(),
            )
        },
        |(doc, ops)| assert_eq!(mzdraw::diff::hunks(doc, ops).len(), ops.len()),
    );
    // n hunks of n different displacements: the moves of a patch, each its own group and its own hunk
    scales(
        "n moves of n displacements",
        20_000,
        |n| {
            let moves: Vec<Op> = (0..n)
                .map(|i| Op::Move {
                    atoms: vec![1 + i],
                    objects: Vec::new(),
                    dx: f64::from(i + 1) * 0.001,
                    dy: 0.0,
                })
                .collect();
            (grid(n, false), moves)
        },
        |(doc, ops)| assert_eq!(mzdraw::diff::hunks(doc, ops).len(), ops.len()),
    );
    // the patch between two big documents whose atoms all moved by different displacements: n groups of moves, n lookups per pool
    scales(
        "diff of n moved atoms",
        20_000,
        |n| {
            let a = grid(n, true);
            let mut b = a.clone();
            for (i, x) in b.atoms.iter_mut().enumerate() {
                x.x += (i as f64 + 1.0) * 0.001;
            }
            (a, b)
        },
        |(a, b)| assert_eq!(mzdraw::diff::diff(a, b).len(), a.atoms.len()),
    );
    // one move and one remove that name the same long list: one hunk, however long the list
    scales(
        "one move and one remove over a list of n ids",
        20_000,
        |n| {
            let ids: Vec<u32> = (1..=n).collect();
            (
                grid(n, false),
                vec![
                    Op::Move {
                        atoms: ids.clone(),
                        objects: Vec::new(),
                        dx: 0.5,
                        dy: 0.0,
                    },
                    Op::Remove {
                        atoms: ids,
                        bonds: Vec::new(),
                        objects: Vec::new(),
                    },
                ],
            )
        },
        |(doc, ops)| assert_eq!(mzdraw::diff::hunks(doc, ops).len(), 1),
    );
    // the language itself on a long list: a move of every atom, applied and undone
    scales(
        "a move of n atoms, applied and undone",
        10_000,
        |n| {
            (
                grid(n, true),
                Op::Move {
                    atoms: (1..=n).collect(),
                    objects: Vec::new(),
                    dx: 0.5,
                    dy: 0.0,
                },
            )
        },
        |(doc, mv)| {
            let mut s = Session::new(doc.clone());
            s.run(std::slice::from_ref(mv)).unwrap();
            assert!(s.undo());
        },
    );
}

#[test]
fn marking_and_saying_a_proposal_scales() {
    // the parts of the proposal itself, on a proposal held once (holding it applies it to a copy, and drawing the drawing under it
    // is the painter's: the cost of the operation `move` and of the painter on a big drawing is theirs, not this layer's)
    scales(
        "marks and words of n hunks",
        400,
        |n| {
            let mut e = editor(grid(4 * n, false));
            e.propose(
                (0..n)
                    .map(|i| Op::parse(&json!({"op":"atom.set","id":1 + i,"chg":1})).unwrap())
                    .collect(),
            )
            .unwrap();
            e
        },
        |e| {
            // repeated, so that the small run is tens of milliseconds and not a few (a few is inside the noise of the clock)
            for _ in 0..20 {
                let _ = (e.proposal_marks(), e.proposal_json());
            }
        },
    );
    scales(
        "marks and words of one hunk over a list of n ids",
        5_000,
        |n| {
            let mut e = editor(grid(n, true));
            e.propose(vec![Op::Move {
                atoms: (1..=n).collect(),
                objects: Vec::new(),
                dx: 0.5,
                dy: 0.0,
            }])
            .unwrap();
            e
        },
        |e| {
            // repeated, so that the small run is tens of milliseconds and not a few (a few is inside the noise of the clock)
            for _ in 0..20 {
                let _ = (e.proposal_marks(), e.proposal_json());
            }
        },
    );
}

/// The request of `hunks` hunks of one atom each on a drawing of 20,000 atoms (given as a document: running the adds is the
/// language's own cost), accepting all of them one by one.
fn accept_request(hunks: u32) -> String {
    let doc = grid(20_000, false).to_json();
    let set: Vec<Value> = (0..hunks)
        .map(|i| json!({"op":"atom.set","id":1 + 10 * i,"chg":1}))
        .collect();
    let accept: Vec<u32> = (1..=hunks).collect();
    json!({"import": doc, "propose": set, "accept": accept, "out": "log"}).to_string()
}

#[test]
fn two_thousand_separate_accepts_through_the_request_are_fast_and_all_or_none() {
    // 2,000 hunks of one atom each on a drawing of 20,000 atoms: the request runs
    let n = mzdraw::proposal::MAX_OPS as u32;
    let doc = grid(20_000, false).to_json();
    let set: Vec<Value> = (0..n)
        .map(|i| json!({"op":"atom.set","id":1 + 10 * i,"chg":1}))
        .collect();
    mzdraw::replay::replay(&accept_request(n)).unwrap();
    // and the hunks cost in proportion: over the same drawing, four times the hunks may cost at most eight times as much (linear 4,
    // quadratic 16). What the drawing alone costs (the same request with one hunk) is taken off, so that the figures are those of the
    // hunks and not of the import of the drawing (which is not linear in a debug build); the three are taken in turns, so that a slow
    // phase of the machine falls on all of them, and each is the least of its runs.
    let texts = [accept_request(1), accept_request(n / 4), accept_request(n)];
    let mut least = [f64::MAX; 3];
    for round in 0..5 {
        for (t, text) in texts.iter().enumerate() {
            let w = timing::Watch::start();
            mzdraw::replay::replay(text).unwrap();
            if round > 0 {
                // the first round is a warm-up
                least[t] = least[t].min(w.elapsed().as_secs_f64() * 1000.0);
            }
        }
    }
    let base = least[0];
    let (small, big) = ((least[1] - base).max(1.0), least[2] - base);
    assert!(
        big <= small * 8.0,
        "accepting one by one: {small:.1} ms for {} hunks, {big:.1} ms for {n} (drawing alone {base:.1} ms)",
        n / 4
    );
    // one number out of range at the end: nothing at all is accepted
    let mut bad: Vec<u32> = (1..n).collect();
    bad.push(n + 1);
    let r = json!({"import": doc, "propose": set, "accept": bad, "out": "log"});
    let e = mzdraw::replay::replay(&r.to_string()).unwrap_err();
    assert_eq!(e.key, "error.draw2.proposalHunk");
    let r = json!({"import": doc, "propose": set, "accept": [1, 2, n + 1], "out": "log", "report": true});
    let v: Value = serde_json::from_str(&mzdraw::replay::replay(&r.to_string()).unwrap()).unwrap();
    assert_eq!(
        v["failed"],
        json!({"stage": "proposal", "op": 2, "key": "error.draw2.proposalHunk", "id": n + 1})
    );
}

#[test]
fn the_accept_list_of_a_request_is_all_or_none() {
    let mut e = drawing();
    e.propose(patch()).unwrap();
    let before = e.doc().to_json();
    let (k, x) = e.accept_hunks(&[2, 4, 9]).unwrap_err();
    assert_eq!((k, x.key), (2, "error.draw2.proposalHunk"));
    let (k, x) = e.accept_hunks(&[2, 2]).unwrap_err();
    assert_eq!((k, x.key), (1, "error.draw2.proposalHunk"));
    assert_eq!(e.doc().to_json(), before, "nothing was accepted");
    e.accept_hunks(&[3, 1]).unwrap();
    assert_ne!(e.doc().to_json(), before);
}

#[test]
fn enter_and_delete_are_the_drawings_until_a_hunk_has_the_keyboard() {
    let mut e = drawing();
    e.propose(patch()).unwrap();
    // with an atom focused, Delete removes it and Enter grows a bond, as ever: the proposal is not touched (and so ends, said at once)
    e.over.focus = Some(1);
    let atoms = e.doc().atoms.len();
    e.key("Enter", 0);
    assert_eq!(e.doc().atoms.len(), atoms + 1, "Enter grew a bond");
    assert!(
        e.said.0 == "draw2.say.proposalStale" || e.said.0 == "draw2.say.bond",
        "{:?}",
        e.said
    );
    assert_eq!(e.proposal_json(), "null");
    // once `]` gave a hunk the keyboard, Enter is the proposal's even with an atom focused; Escape gives the keys back
    let mut f = drawing();
    f.propose(patch()).unwrap();
    f.over.focus = Some(1);
    f.key("]", 0);
    f.key("Escape", 0); // ends the review (and leaves the focus, as ever)
    f.over.focus = Some(1);
    f.key("Delete", 0);
    assert!(
        f.doc().atom(1).is_none(),
        "Delete deleted the focused atom after Escape"
    );
    assert_eq!(f.proposal_json(), "null");
}

#[test]
fn an_edit_ends_the_proposal_and_says_so_at_once() {
    let mut e = drawing();
    e.propose(patch()).unwrap();
    e.set_tool("atom:N");
    e.pointer_event(0, 1, 400.0, 400.0, 0, 0, 0.0);
    e.pointer_event(2, 1, 400.0, 400.0, 0, 0, 10.0);
    assert_eq!(e.proposal_json(), "null");
    assert_eq!(e.said.0, "draw2.say.proposalStale");
}

#[test]
fn a_second_finger_is_not_swallowed_after_the_proposal_ended_mid_press() {
    let mut e = drawing();
    e.propose(patch()).unwrap();
    let marks = e.proposal_marks();
    let Some(mzdraw::scene::Mark::Num { p, .. }) = marks
        .iter()
        .find(|m| matches!(m, mzdraw::scene::Mark::Num { .. }))
    else {
        panic!("no number");
    };
    let (x, y) = e.view.to_px(p.0, p.1);
    e.pointer_event(0, 7, x, y, 2, 0, 0.0);
    assert_eq!(e.eat, Some(7));
    // the proposal ends while the finger is down; another finger is the tool's
    e.reject_all().unwrap();
    let f = e.pointer_event(0, 8, 5.0, 5.0, 2, 0, 50.0);
    assert!(e.eat.is_none());
    let _ = f;
}

#[test]
fn an_add_that_names_no_id_is_marked_where_it_will_stand() {
    use mzdraw::scene::Mark;
    let mut e = drawing();
    e.propose(ops(json!([{"op":"atom.add","el":"O","x":20.0,"y":2.0}])))
        .unwrap();
    let marks = e.proposal_marks();
    assert!(marks
        .iter()
        .any(|m| matches!(m, Mark::Dot { p, .. } if *p == (20.0, 2.0))));
    let at = marks
        .iter()
        .find_map(|m| match m {
            Mark::Num { p, .. } => Some(*p),
            _ => None,
        })
        .unwrap();
    assert!(
        at.0 > 19.0,
        "the number is by the atom, not at the origin: {at:?}"
    );
    // a hunk with no point to mark takes the middle of the drawing
    let mut e = drawing();
    e.propose(ops(
        json!([{"op":"rotate","atoms":[1,2],"deg":30.0,"x":0.0,"y":0.0}]),
    ))
    .unwrap();
    let _ = e.proposal_marks();
}
