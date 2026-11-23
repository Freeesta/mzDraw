//! Align, distribute, turn by a typed angle and nudge: existing operations only, one undo step each, deterministic.

use mzdraw::{Editor, Op};
use serde_json::json;

/// Three synthetic structures of different sizes at known places: ids of their atoms by structure.
fn three() -> (Editor, Vec<Vec<u32>>) {
    let mut e = Editor::default();
    let mut ids = Vec::new();
    for (k, smi) in ["CCO", "C1CCCCC1", "CC"].iter().enumerate() {
        let before: Vec<u32> = e.doc().atoms.iter().map(|a| a.id).collect();
        e.import(smi).unwrap();
        let mine: Vec<u32> = (e.doc().atoms.iter().map(|a| a.id))
            .filter(|i| !before.contains(i))
            .collect();
        // put each structure at its own place: left edge at 0, 7.3 and 13.9; its y centre at 1, -2 and 4
        let (x0, y0, y1) = {
            let a: Vec<_> = mine.iter().filter_map(|&i| e.doc().atom(i)).collect();
            (
                a.iter().map(|p| p.x).fold(f64::MAX, f64::min),
                a.iter().map(|p| p.y).fold(f64::MAX, f64::min),
                a.iter().map(|p| p.y).fold(f64::MIN, f64::max),
            )
        };
        let (tx, ty) = ([0.0, 7.3, 13.9][k], [1.0, -2.0, 4.0][k]);
        e.session
            .run(&[Op::Move {
                atoms: mine.clone(),
                objects: Vec::new(),
                dx: tx - x0,
                dy: ty - (y0 + y1) / 2.0,
            }])
            .unwrap();
        ids.push(mine);
    }
    (e, ids)
}

fn bx(e: &Editor, ids: &[u32]) -> (f64, f64, f64, f64) {
    let a: Vec<_> = ids.iter().filter_map(|&i| e.doc().atom(i)).collect();
    (
        a.iter().map(|p| p.x).fold(f64::MAX, f64::min),
        a.iter().map(|p| p.y).fold(f64::MAX, f64::min),
        a.iter().map(|p| p.x).fold(f64::MIN, f64::max),
        a.iter().map(|p| p.y).fold(f64::MIN, f64::max),
    )
}

fn select(e: &mut Editor, ids: &[Vec<u32>]) {
    e.over.sel_atoms = ids.iter().flatten().copied().collect();
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1.1e-4
}

#[test]
fn align_puts_every_structure_on_one_edge_of_the_box_of_the_selection() {
    for (cmd, pick) in [
        ("align:left", 0usize),
        ("align:right", 2),
        ("align:bottom", 1),
        ("align:top", 3),
    ] {
        let (mut e, ids) = three();
        let boxes: Vec<_> = ids.iter().map(|i| bx(&e, i)).collect();
        select(&mut e, &ids);
        let steps = e.session.steps();
        assert_ne!(e.arrange(cmd), 0);
        assert_eq!(e.session.steps(), steps + 1, "{cmd}: one undo step");
        let want = match pick {
            0 => boxes.iter().map(|b| b.0).fold(f64::MAX, f64::min),
            1 => boxes.iter().map(|b| b.1).fold(f64::MAX, f64::min),
            2 => boxes.iter().map(|b| b.2).fold(f64::MIN, f64::max),
            _ => boxes.iter().map(|b| b.3).fold(f64::MIN, f64::max),
        };
        for i in &ids {
            let b = bx(&e, i);
            let got = [b.0, b.1, b.2, b.3][pick];
            assert!(close(got, want), "{cmd}: {got} vs {want}");
        }
    }
}

#[test]
fn align_centre_and_middle_share_the_centre_of_the_union() {
    let (mut e, ids) = three();
    select(&mut e, &ids);
    let u: Vec<_> = ids.iter().map(|i| bx(&e, i)).collect();
    let cx = (u.iter().map(|b| b.0).fold(f64::MAX, f64::min)
        + u.iter().map(|b| b.2).fold(f64::MIN, f64::max))
        / 2.0;
    let cy = (u.iter().map(|b| b.1).fold(f64::MAX, f64::min)
        + u.iter().map(|b| b.3).fold(f64::MIN, f64::max))
        / 2.0;
    e.arrange("align:centre");
    e.arrange("align:middle");
    for i in &ids {
        let b = bx(&e, i);
        assert!(close((b.0 + b.2) / 2.0, cx) && close((b.1 + b.3) / 2.0, cy));
    }
}

#[test]
fn distribute_gives_equal_gaps_and_keeps_the_ends() {
    let (mut e, ids) = three();
    select(&mut e, &ids);
    let (first, last) = (bx(&e, &ids[0]), bx(&e, &ids[2]));
    e.arrange("distribute:h");
    let b: Vec<_> = ids.iter().map(|i| bx(&e, i)).collect();
    assert!(close(b[0].0, first.0) && close(b[2].2, last.2));
    assert!(close(b[1].0 - b[0].2, b[2].0 - b[1].2), "{b:?}");
    // vertically, in the other order of the ids
    let (mut e, ids) = three();
    select(&mut e, &[ids[2].clone(), ids[0].clone(), ids[1].clone()]);
    e.arrange("distribute:v");
    let mut b: Vec<_> = ids.iter().map(|i| bx(&e, i)).collect();
    b.sort_by(|p, q| p.1.partial_cmp(&q.1).unwrap());
    assert!(close(b[1].1 - b[0].3, b[2].1 - b[1].3), "{b:?}");
}

#[test]
fn the_result_does_not_depend_on_the_order_of_the_selection() {
    let (mut a, ids) = three();
    let (mut b, _) = three();
    select(&mut a, &ids);
    select(&mut b, &[ids[2].clone(), ids[0].clone(), ids[1].clone()]);
    a.arrange("distribute:h");
    b.arrange("distribute:h");
    assert_eq!(a.doc().to_json(), b.doc().to_json());
}

#[test]
fn two_items_cannot_be_distributed_and_one_cannot_be_aligned() {
    let (mut e, ids) = three();
    select(&mut e, &ids[..2]);
    let before = e.doc().to_json();
    e.arrange("distribute:h");
    assert_eq!(e.error, "error.draw2.arrangeThree");
    select(&mut e, &ids[..1]);
    e.arrange("align:left");
    assert_eq!(e.error, "error.draw2.arrangeTwo");
    assert_eq!(e.doc().to_json(), before);
}

#[test]
fn an_arrow_is_aligned_with_a_structure_and_keeps_its_length() {
    let (mut e, ids) = three();
    e.session
        .run(&[Op::ObjAdd {
            id: Some(900),
            t: mzdraw::ObjKind::Arrow,
            x: 3.0,
            y: 9.0,
            x2: 5.0,
            y2: 9.0,
            text: String::new(),
            runs: Vec::new(),
            from: 0,
            to: 0,
        }])
        .unwrap();
    e.over.sel_atoms = ids[0].clone();
    e.over.sel_objs = vec![900];
    e.arrange("align:bottom");
    let arrow = e.doc().object(900).unwrap().clone();
    assert!(close(arrow.y, bx(&e, &ids[0]).1) && close(arrow.y2, arrow.y));
    assert!(close(arrow.x2 - arrow.x, 2.0));
}

#[test]
fn turning_by_90_four_times_returns_the_document_exactly() {
    let (mut e, ids) = three();
    select(&mut e, &ids);
    let was = e.doc().to_json();
    for _ in 0..4 {
        e.arrange("rotate:90");
    }
    assert_eq!(e.doc().to_json(), was, "four quarter turns drift");
}

#[test]
fn a_huge_angle_is_taken_modulo_360_and_an_impossible_one_is_refused() {
    let (mut a, ids) = three();
    let (mut b, _) = three();
    select(&mut a, &ids);
    select(&mut b, &ids);
    a.arrange("rotate:1000000");
    b.arrange("rotate:280");
    assert_eq!(a.doc().to_json(), b.doc().to_json(), "1e6 = 280 degrees");
    let before = a.doc().to_json();
    for bad in [
        "rotate:1e300",
        "rotate:inf",
        "rotate:nan",
        "rotate:1000001",
        "rotate:",
        "rotate:abc",
    ] {
        a.arrange(bad);
        assert_eq!(a.error, "error.draw2.arrangeAngle", "{bad}");
    }
    for bad in [
        "nudge:1",
        "nudge:1,x",
        "nudge:inf,0",
        "nudge:1e300,0",
        "nudge:,",
    ] {
        a.arrange(bad);
        assert_eq!(a.error, "error.draw2.arrangeShift", "{bad}");
    }
    assert_eq!(a.doc().to_json(), before);
}

#[test]
fn a_shift_or_turn_of_nothing_changes_nothing_adds_no_step_and_says_so() {
    let (mut e, ids) = three();
    select(&mut e, &ids);
    let (before, steps) = (e.doc().to_json(), e.session.steps());
    for cmd in [
        "nudge:0,0",
        "nudge:0.00001,-0.00002",
        "rotate:0",
        "rotate:360",
        "rotate:-720",
        "rotate:0.00004",
    ] {
        e.arrange(cmd);
        assert_eq!(e.said.0, "draw2.say.arrangedSame", "{cmd}");
        assert_eq!(e.error, "", "{cmd}");
        assert_eq!(e.doc().to_json(), before, "{cmd}");
        assert_eq!(e.session.steps(), steps, "{cmd}: no undo step");
    }
    // one atom turned about itself
    e.over.sel_atoms = vec![ids[0][0]];
    e.arrange("rotate:90");
    assert_eq!(e.said.0, "draw2.say.arrangedSame");
    assert_eq!(e.session.steps(), steps);
}

/// Nudge and turn act on the atoms as named: a part of a structure moves alone (align and distribute move whole structures).
#[test]
fn a_nudge_acts_on_the_atoms_given_and_an_align_on_whole_structures() {
    let (mut e, ids) = three();
    let (first, other) = (ids[0][0], ids[0][1]);
    let (x0, x1) = (
        e.doc().atom(first).unwrap().x,
        e.doc().atom(other).unwrap().x,
    );
    e.over.sel_atoms = vec![first];
    e.arrange("nudge:1,0");
    assert!(close(e.doc().atom(first).unwrap().x, x0 + 1.0));
    assert!(
        close(e.doc().atom(other).unwrap().x, x1),
        "the other atoms stay"
    );
    e.undo();
    // align: one atom of each stands for its whole structure
    e.over.sel_atoms = vec![first, ids[1][0]];
    e.arrange("align:left");
    assert!(close(bx(&e, &ids[0]).0, bx(&e, &ids[1]).0));
    assert!(
        close(
            e.doc().atom(other).unwrap().x - e.doc().atom(first).unwrap().x,
            x1 - x0
        ),
        "the structure kept its shape"
    );
}

/// Boxes nested in one another: A [0,10], B [1,2], C [5,6] along x. The ends are the first and the last by centre (B and C), not the
/// largest high edge; they do not fit evenly (gap -3.5), so the command is refused and nothing moves.
#[test]
fn nested_boxes_are_ordered_by_centre_and_a_negative_gap_is_refused() {
    let mut e = Editor::default();
    let log = json!({"log":[
        {"op":"atom.add","el":"C","x":0.0,"y":0.0},{"op":"atom.add","el":"C","x":10.0,"y":0.0},
        {"op":"atom.add","el":"C","x":1.0,"y":5.0},{"op":"atom.add","el":"C","x":2.0,"y":5.0},
        {"op":"atom.add","el":"C","x":5.0,"y":9.0},{"op":"atom.add","el":"C","x":6.0,"y":9.0},
        {"op":"bond.add","a":1,"b":2,"o":1},{"op":"bond.add","a":3,"b":4,"o":1},{"op":"bond.add","a":5,"b":6,"o":1}],"out":"mzdraw"});
    e.import(&mzdraw::replay::replay(&log.to_string()).unwrap())
        .unwrap();
    e.select_all();
    let before = e.doc().to_json();
    e.arrange("distribute:h");
    assert_eq!(e.error, "error.draw2.arrangeFit");
    assert_eq!(e.doc().to_json(), before);
    // with room (C moved right to [17,18]): B and C, the ends by centre, stay and A goes between them with equal gaps of 2.5
    e.session
        .run(&[Op::Move {
            atoms: vec![5, 6],
            objects: vec![],
            dx: 12.0,
            dy: 0.0,
        }])
        .unwrap();
    e.select_all();
    e.arrange("distribute:h");
    let x = |id: u32| e.doc().atom(id).unwrap().x;
    assert_eq!(
        (x(3), x(4), x(5), x(6)),
        (1.0, 2.0, 17.0, 18.0),
        "the ends stay"
    );
    assert_eq!((x(1), x(2)), (4.5, 14.5));
}

#[test]
fn the_ops_do_not_depend_on_the_order_of_the_selection() {
    let (mut a, ids) = three();
    let (mut b, _) = three();
    select(&mut a, &ids);
    select(&mut b, &[ids[2].clone(), ids[0].clone(), ids[1].clone()]);
    for cmd in ["align:left", "align:middle", "distribute:h"] {
        a.arrange(cmd);
        b.arrange(cmd);
    }
    assert_eq!(
        format!("{:?}", a.session.log()),
        format!("{:?}", b.session.log()),
        "the log"
    );
}

/// The stereo guard speaks when an arrange command changes a configuration: one neighbour of a stereocentre moved across the others
/// by `nudge`. (The same test fails with the guard call taken out of `arrange_on`; checked by hand, see the PR.)
#[test]
fn the_stereo_guard_speaks_when_a_nudge_moves_a_neighbour_across_a_centre() {
    let log = json!({"log":[
        {"op":"atom.add","el":"C","x":0.0,"y":0.0},{"op":"atom.add","el":"F","x":0.0,"y":1.0},
        {"op":"atom.add","el":"Cl","x":-0.9,"y":-0.5},{"op":"atom.add","el":"Br","x":0.9,"y":-0.5},
        {"op":"bond.add","a":1,"b":2,"o":1,"s":1},{"op":"bond.add","a":1,"b":3,"o":1,"s":0},{"op":"bond.add","a":1,"b":4,"o":1,"s":0}],"out":"mzdraw"});
    let mut e = Editor::default();
    e.import(&mzdraw::replay::replay(&log.to_string()).unwrap())
        .unwrap();
    // the whole piece: nothing to say
    e.select_all();
    e.arrange("nudge:1,0");
    assert_eq!(e.notice.0, "");
    // Cl alone, two bond lengths right: across the Br
    e.over.sel_atoms = vec![3];
    e.arrange("nudge:0.5,0");
    assert_eq!(e.notice.0, "", "not across yet");
    e.arrange("nudge:1.5,0");
    assert_eq!(e.notice, ("draw2.say.stereoChanged", "1".to_string()));
    e.undo();
    assert_eq!(e.notice.0, "");
}

/// An arrow anchored to two structures follows them when an arrange command moves one of them (`following`).
fn anchored() -> (Editor, Vec<Vec<u32>>) {
    let (mut e, ids) = three();
    e.session
        .run(&[Op::ObjAdd {
            id: Some(900),
            t: mzdraw::ObjKind::Arrow,
            x: 3.0,
            y: 0.0,
            x2: 6.0,
            y2: 0.0,
            text: String::new(),
            runs: Vec::new(),
            from: ids[0][0],
            to: ids[1][0],
        }])
        .unwrap();
    (e, ids)
}

#[test]
fn an_anchored_arrow_follows_what_an_arrange_command_moves() {
    for cmd in ["align:top", "nudge:0,2.5"] {
        let (mut e, ids) = anchored();
        let before = e.doc().object(900).unwrap().clone();
        select(&mut e, &[ids[0].clone(), ids[1].clone()]);
        if cmd.starts_with("nudge") {
            e.over.sel_atoms = ids[0].clone();
        }
        e.arrange(cmd);
        let after = e.doc().object(900).unwrap().clone();
        assert_ne!(
            (before.y, before.y2),
            (after.y, after.y2),
            "{cmd}: the arrow did not follow"
        );
        // one undo step brings back the structures and the arrow
        e.undo();
        assert_eq!(e.doc().object(900).unwrap(), &before, "{cmd}: undo");
    }
}

#[test]
fn every_arrange_command_is_undone_exactly() {
    for cmd in [
        "align:left",
        "align:centre",
        "align:right",
        "align:top",
        "align:middle",
        "align:bottom",
        "distribute:h",
        "distribute:v",
        "rotate:37.5",
        "nudge:0.3,-1.2",
    ] {
        let (mut e, ids) = three();
        select(&mut e, &ids);
        let before = e.doc().to_json();
        assert_ne!(e.arrange(cmd), 0, "{cmd}");
        assert_ne!(e.doc().to_json(), before, "{cmd} changed nothing");
        e.undo();
        assert_eq!(e.doc().to_json(), before, "{cmd}: undo");
    }
}

#[test]
fn arranging_what_is_arranged_changes_nothing_and_says_so() {
    let (mut e, ids) = three();
    select(&mut e, &ids);
    e.arrange("align:left");
    let steps = e.session.steps();
    e.arrange("align:left");
    assert_eq!(e.session.steps(), steps);
    assert_eq!(e.said.0, "draw2.say.arrangedSame");
}

#[test]
fn bad_commands_are_not_commands() {
    for bad in [
        "align:",
        "align:up",
        "distribute:x",
        "scale:2",
        "arrange:left",
        "rotate",
        "nudge",
    ] {
        assert_eq!(
            mzdraw::arrange::parse(bad),
            Err("error.draw2.command"),
            "{bad}"
        );
    }
    assert!(mzdraw::arrange::parse("rotate:-45.5").is_ok());
    assert!(mzdraw::arrange::parse("nudge:0.5,-1").is_ok());
    assert!(!mzdraw::arrange::is_arrange("scale:2"));
}

#[test]
fn nudge_and_turn_sort_and_dedupe_the_atoms_they_are_given() {
    let (mut a, ids) = three();
    let (mut b, _) = three();
    let mut l = ids[0].clone();
    l.reverse();
    let mut rep = l.clone();
    rep.extend(ids[0].iter().copied());
    for (e, set) in [(&mut a, ids[0].clone()), (&mut b, rep)] {
        e.over.sel_atoms = set;
        e.arrange("nudge:1,0");
        e.arrange("rotate:30");
    }
    assert_eq!(
        a.doc().to_json(),
        b.doc().to_json(),
        "a repeated id moved or turned an atom twice"
    );
    assert_eq!(
        format!("{:?}", a.session.log()),
        format!("{:?}", b.session.log())
    );
    // two orders give the same log
    let (mut c, _) = three();
    c.over.sel_atoms = l;
    c.arrange("nudge:1,0");
    c.arrange("rotate:30");
    assert_eq!(
        format!("{:?}", a.session.log()),
        format!("{:?}", c.session.log())
    );
}

#[test]
fn a_turn_that_rounds_to_a_whole_turn_is_no_change() {
    let (mut e, ids) = three();
    select(&mut e, &ids);
    let (before, steps) = (e.doc().to_json(), e.session.steps());
    for cmd in ["rotate:359.99996", "rotate:-359.99996", "rotate:719.99996"] {
        e.arrange(cmd);
        assert_eq!(e.said.0, "draw2.say.arrangedSame", "{cmd}");
        assert_eq!(
            (e.doc().to_json(), e.session.steps()),
            (before.clone(), steps),
            "{cmd}"
        );
    }
}

/// Boxes that touch exactly (gap 0) are distributed, not refused for a rounding error: for the edges 0.2067, 0.3863, 0.8358 and 2.5027
/// the gap comes out as -2.2e-16 in floating point.
#[test]
fn touching_boxes_are_distributed_and_not_refused() {
    let mut e = Editor::default();
    let edges = [0.2067, 0.3863, 0.8358, 2.5027];
    let mut ops = Vec::new();
    for k in 0..3u32 {
        ops.push(Op::AtomAdd {
            id: Some(k * 2 + 1),
            el: "C".into(),
            x: edges[k as usize],
            y: f64::from(k),
            map: None,
        });
        ops.push(Op::AtomAdd {
            id: Some(k * 2 + 2),
            el: "C".into(),
            x: edges[k as usize + 1],
            y: f64::from(k),
            map: None,
        });
        ops.push(Op::BondAdd {
            id: Some(10 + k),
            a: k * 2 + 1,
            b: k * 2 + 2,
            o: 1,
            s: 0,
        });
    }
    e.session.run(&ops).unwrap();
    e.select_all();
    e.arrange("distribute:h");
    assert_eq!(e.error, "", "refused: {}", e.error);
}
