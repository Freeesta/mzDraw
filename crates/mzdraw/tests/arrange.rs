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
fn turning_by_90_four_times_returns_the_document() {
    let (mut e, ids) = three();
    select(&mut e, &ids);
    let was = e.doc().clone();
    for _ in 0..4 {
        e.arrange("rotate:90");
    }
    for (a, b) in e.doc().atoms.iter().zip(&was.atoms) {
        assert!(close(a.x, b.x) && close(a.y, b.y), "{a:?} {b:?}");
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
        "rotate:",
        "rotate:nan",
        "rotate:1e999",
        "scale:0",
        "scale:-2",
        "scale:1000",
        "nudge:1",
        "nudge:1,x",
        "nudge:inf,0",
    ] {
        assert!(mzdraw::arrange::parse(bad).is_none(), "{bad}");
    }
    assert!(mzdraw::arrange::parse("rotate:-45.5").is_some());
    let _ = json!(null);
}

#[test]
fn the_stereo_guard_still_speaks_after_a_turn_of_a_structure_with_a_wedge() {
    let mut e = Editor::default();
    e.import("C[C@H](F)Cl").unwrap();
    e.select_all();
    let _ = e.arrange("rotate:90");
    // a turn keeps the configuration: no notice
    assert_eq!(e.notice.0, "");
}
