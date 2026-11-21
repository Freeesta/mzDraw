//! Arrows that know their structures (tp-01): the anchors `from` and `to`, the routed ends (`route`), and the follow of the editor
//! commands. Synthetic structures only.

use mzdraw::editor::ALT;
use mzdraw::route::{clip, exit_distance, route, structure_box};
use mzdraw::{Document, Editor, Op, Session};
use serde_json::{json, Value};

/// A tiny seeded generator (the same numbers everywhere).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / ((1u64 << 31) as f64)
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

/// Two structures (a chain of two atoms and a chain of an atom and an oxygen) with an arrow anchored on both, as a document text.
fn scheme() -> String {
    json!({"format":"mzdraw/1","next":8,
        "atoms":[{"id":1,"el":"C","x":0,"y":0},{"id":2,"el":"C","x":1,"y":0},{"id":3,"el":"C","x":6,"y":0},{"id":4,"el":"O","x":7,"y":0}],
        "bonds":[{"id":5,"a":1,"b":2,"o":1},{"id":6,"a":3,"b":4,"o":1}],
        "objects":[{"id":7,"t":"arrow","x":1.35,"y":0,"x2":5.65,"y2":0,"from":2,"to":3}]})
    .to_string()
}

fn replay(request: Value) -> Value {
    serde_json::from_str(&mzdraw::replay::replay(&request.to_string()).unwrap()).unwrap()
}

const FONT: f64 = 0.4;
const MARGIN: f64 = 0.35;

/// The distance of a point to the boundary of an inflated box.
fn off_boundary(b: [f64; 4], m: f64, p: (f64, f64)) -> f64 {
    let (x0, y0, x1, y1) = (b[0] - m, b[1] - m, b[2] + m, b[3] + m);
    let inside = p.0 >= x0 - 1e-12 && p.0 <= x1 + 1e-12 && p.1 >= y0 - 1e-12 && p.1 <= y1 + 1e-12;
    if !inside {
        let dx = (x0 - p.0).max(p.0 - x1).max(0.0);
        let dy = (y0 - p.1).max(p.1 - y1).max(0.0);
        return dx.hypot(dy);
    }
    (p.0 - x0).min(x1 - p.0).min(p.1 - y0).min(y1 - p.1)
}

fn outside(b: [f64; 4], m: f64, p: (f64, f64)) -> bool {
    p.0 < b[0] - m || p.0 > b[2] + m || p.1 < b[1] - m || p.1 > b[3] + m
}

#[test]
fn the_clip_agrees_with_a_brute_force_sampler() {
    let mut r = Rng(7);
    let mut found = 0;
    for _ in 0..200 {
        let rect = |r: &mut Rng, cx: f64, cy: f64| {
            let (w, h) = (r.range(0.2, 3.0), r.range(0.2, 3.0));
            [cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0]
        };
        let (ax, ay) = (r.range(-2.0, 2.0), r.range(-2.0, 2.0));
        let a = rect(&mut r, ax, ay);
        let (bx, by) = (r.range(-9.0, 9.0), r.range(-9.0, 9.0));
        let b = rect(&mut r, bx, by);
        let m = r.range(0.0, 1.0);
        let Some((tail, head)) = clip(a, b, m) else {
            continue;
        };
        found += 1;
        let ca = ((a[0] + a[2]) / 2.0, (a[1] + a[3]) / 2.0);
        let cb = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
        let n = 10_000;
        let at = |i: usize| {
            let t = i as f64 / n as f64;
            (ca.0 + (cb.0 - ca.0) * t, ca.1 + (cb.1 - ca.1) * t)
        };
        let step = (cb.0 - ca.0).hypot(cb.1 - ca.1) / n as f64;
        // the first sample outside the inflated box of the tail structure, the last one outside that of the head structure
        let first = (0..=n).find(|&i| outside(a, m, at(i))).unwrap();
        let last = (0..=n).rev().find(|&i| outside(b, m, at(i))).unwrap();
        let (p, q) = (at(first), at(last));
        assert!(
            (tail.0 - p.0).hypot(tail.1 - p.1) <= step * 1.01,
            "tail {tail:?} vs {p:?}"
        );
        assert!(
            (head.0 - q.0).hypot(head.1 - q.1) <= step * 1.01,
            "head {head:?} vs {q:?}"
        );
        // on the boundary and on the line through the centres, to 1e-9 before any rounding
        assert!(off_boundary(a, m, tail) < 1e-9 && off_boundary(b, m, head) < 1e-9);
        let cross = (cb.0 - ca.0) * (tail.1 - ca.1) - (cb.1 - ca.1) * (tail.0 - ca.0);
        assert!(cross.abs() < 1e-9);
        // a ray along a direction that leaves a box leaves it only once
        let len = (cb.0 - ca.0).hypot(cb.1 - ca.1);
        assert!(exit_distance(a, m, (cb.0 - ca.0) / len, (cb.1 - ca.1) / len) > 0.0);
    }
    assert!(found >= 150, "only {found} pairs left room for an arrow");
}

#[test]
fn a_bare_move_does_not_follow_and_a_command_does() {
    let moved = |request: Value| replay(request)["objects"][0].clone();
    let bare =
        moved(json!({"import": scheme(), "log": [{"op":"move","atoms":[3,4],"dx":0,"dy":2}]}));
    assert_eq!(
        (bare["x"].as_f64(), bare["y2"].as_f64()),
        (Some(1.35), None)
    );
    let cmd = moved(
        json!({"import": scheme(), "commands": [{"command":"move","args":{"atoms":[3,4],"dx":0,"dy":2}}]}),
    );
    assert!(cmd["y2"].as_f64().unwrap() > 1.0);
}

/// Every anchored arrow of a document: its tail on the inflated box of its `from` structure, its head on that of its `to`
/// structure, the whole on the line through the two centres (ε as the saved document, rounded to 4 decimals, allows).
fn assert_routed(d: &Document, eps: f64) {
    for o in d.objects.iter().filter(|o| o.from != 0 && o.to != 0) {
        let (a, b) = (
            structure_box(d, o.from, FONT).unwrap(),
            structure_box(d, o.to, FONT).unwrap(),
        );
        assert!(
            off_boundary(a, MARGIN, (o.x, o.y)) < eps,
            "tail of {}",
            o.id
        );
        assert!(
            off_boundary(b, MARGIN, (o.x2, o.y2)) < eps,
            "head of {}",
            o.id
        );
        let ca = ((a[0] + a[2]) / 2.0, (a[1] + a[3]) / 2.0);
        let cb = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
        let len = (cb.0 - ca.0).hypot(cb.1 - ca.1);
        for p in [(o.x, o.y), (o.x2, o.y2)] {
            let cross = ((cb.0 - ca.0) * (p.1 - ca.1) - (cb.1 - ca.1) * (p.0 - ca.0)) / len;
            assert!(
                cross.abs() < eps,
                "{p:?} is {cross} off the line of {}",
                o.id
            );
        }
    }
}

#[test]
fn after_any_move_rotate_or_flip_command_the_arrows_are_on_their_routed_ends() {
    // font and margin as the default style gives them: 12 pt over a bond of 30 pt, 0.35 bond lengths
    let mut r = Rng(11);
    for _ in 0..60 {
        let atoms =
            [vec![1, 2], vec![3, 4], vec![1, 2, 3, 4]][(r.next() * 3.0) as usize % 3].clone();
        let axis = ["h", "v"][(r.next() * 2.0) as usize % 2];
        let command = match (r.next() * 3.0) as usize % 3 {
            0 => {
                json!({"command":"move","args":{"atoms":atoms,"dx":r.range(-4.0,4.0),"dy":r.range(-4.0,4.0)}})
            }
            1 => json!({"command":"rotate","args":{"atoms":atoms,"deg":r.range(-180.0,180.0)}}),
            _ => json!({"command":"flip","args":{"atoms":atoms,"axis":axis}}),
        };
        let out = mzdraw::replay::replay(&json!({"import": scheme(), "commands":[command.clone()], "set":["style.font_size=12"]}).to_string()).unwrap();
        let d = Document::from_json(&out).unwrap();
        let arrow = &d.objects[0];
        let routed = route(&d, arrow.id, MARGIN, 12.0 / 30.0);
        // a command that leaves the structures with no room for an arrow leaves the arrow where it was (no operation is not "off")
        if !routed.is_empty() {
            panic!("{command}: the arrow is not on its routed ends: {routed:?}");
        }
        if d.object(7)
            .is_some_and(|o| (o.x2 - o.x).abs() + (o.y2 - o.y).abs() > 1e-3)
        {
            assert_routed(&d, 3e-4);
        }
    }
}

#[test]
fn removing_an_anchored_atom_keeps_the_anchor_and_undo_is_exact() {
    let doc = Document::from_json(&scheme()).unwrap();
    let mut s = Session::new(doc.clone());
    s.run(&[Op::Remove {
        atoms: vec![3],
        bonds: vec![],
        objects: vec![],
    }])
    .unwrap();
    let o = s.doc.object(7).unwrap();
    assert_eq!((o.from, o.to), (2, 3), "a remove never rewrites the anchor");
    // used as a free end: nothing is routed
    assert!(route(&s.doc, 7, MARGIN, FONT).is_empty());
    assert!(s.undo());
    assert_eq!(s.doc, doc);
    assert_eq!(s.doc.object(7).unwrap().to, 3);
}

#[test]
fn anchors_are_set_changed_and_cleared_and_each_step_undoes_exactly() {
    let mut s = Session::new(Document::from_json(&scheme()).unwrap());
    let before = s.doc.clone();
    for (from, to) in [
        (Some(1), Some(4)),
        (Some(0), None),
        (None, Some(0)),
        (Some(2), Some(3)),
    ] {
        let was = s.doc.clone();
        s.run(&[Op::ObjSet {
            id: 7,
            text: None,
            x2: None,
            y2: None,
            from,
            to,
        }])
        .unwrap();
        assert!(s.undo());
        assert_eq!(s.doc, was);
        s.run(&[Op::ObjSet {
            id: 7,
            text: None,
            x2: None,
            y2: None,
            from,
            to,
        }])
        .unwrap();
    }
    // the last pair is the starting pair: the document is the one it began with
    assert_eq!(s.doc, before);
    // on a plus or a text an anchor is refused
    let mut t = Session::new(Document::default());
    let add = |t| Op::ObjAdd {
        id: None,
        t,
        x: 0.0,
        y: 0.0,
        x2: 0.0,
        y2: 0.0,
        text: "a".into(),
        from: 1,
        to: 0,
    };
    assert!(t.run(&[add(mzdraw::ObjKind::Text)]).is_err());
}

#[test]
fn the_log_of_a_keyboard_move_replays_to_the_page_bytes() {
    let mut e = Editor::default();
    e.import(&scheme()).unwrap();
    e.set_tool("select");
    e.over.sel_atoms = vec![3, 4];
    e.key_at("ArrowUp", ALT, 0.0);
    e.key_at("ArrowUp", ALT, 100.0);
    e.key_at("ArrowRight", ALT, 200.0);
    let page = e.save("mzdraw").unwrap();
    let log: Value = serde_json::from_str(&e.session.log_json()).unwrap();
    assert!(
        log.as_array().unwrap().iter().any(|o| o["op"] == "obj.set"),
        "the route ops are in the log"
    );
    let replayed = mzdraw::replay::replay(&json!({"log": log}).to_string()).unwrap();
    assert_eq!(replayed, page);
    // and the arrow did follow
    let d = Document::from_json(&page).unwrap();
    assert!(d.object(7).unwrap().y2 > 0.1);
}

#[test]
fn a_document_without_anchors_is_written_as_before() {
    let mut v: Value = serde_json::from_str(&scheme()).unwrap();
    let arrow = v["objects"][0].as_object_mut().unwrap();
    arrow.remove("from");
    arrow.remove("to");
    let d = Document::from_json(&v.to_string()).unwrap();
    assert!(!d.to_json().contains("\"from\""));
    let anchored = Document::from_json(&scheme()).unwrap();
    assert_eq!(Document::from_json(&anchored.to_json()).unwrap(), anchored);
}

#[test]
fn a_refused_command_is_a_verdict_of_stage_command_and_an_unknown_one_an_invalid_request() {
    let request = json!({"import": scheme(), "report": true, "commands": [
        {"command":"move","args":{"atoms":[3,4],"dx":1,"dy":0}},
        {"command":"move","args":{"atoms":[99],"dx":1,"dy":0}}]});
    let reply = mzdraw::replay::replay_at(&request.to_string(), 0).unwrap();
    assert!(!reply.ok);
    let v: Value = serde_json::from_str(&reply.text).unwrap();
    assert_eq!(v["failed"]["stage"], "command");
    assert_eq!(v["failed"]["op"], 1);
    assert_eq!(v["failed"]["key"], "error.draw2.noAtom");
    assert!(v.get("out").is_none(), "nothing is written");
    let unknown = json!({"import": scheme(), "report": true, "commands": [{"command":"explode"}]});
    let e = mzdraw::replay::replay(&unknown.to_string()).unwrap_err();
    assert_eq!(e.key, "error.draw2.command");
}
