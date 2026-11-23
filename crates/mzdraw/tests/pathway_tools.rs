//! Anchoring by hand (tp-02): the arrow tool anchors an arrow drawn from one structure to another, `pathway:anchor` anchors a free
//! arrow, `pathway:route` routes every anchored arrow, and the context-menu items run the same Rust. Synthetic structures only.

use mzdraw::editor::{ALT, MOUSE};
use mzdraw::route::{nearest_structure, structure_at, structure_box};
use mzdraw::{Document, Editor, Style};
use serde_json::{json, Value};

/// Two structures, each a chain of two carbons, with ids that are not in pool order: A has the atoms 6 and 2 at x 0 and 1, B the
/// atoms 9 and 7 at x `gap` + 1 and `gap` + 2 (the smallest ids are 2 and 7). `arrows` is the list of objects.
fn scheme(gap: f64, arrows: Value) -> String {
    json!({"format":"mzdraw/1","next":20,
        "atoms":[{"id":6,"el":"C","x":0,"y":0},{"id":2,"el":"C","x":1,"y":0},
                 {"id":9,"el":"C","x":gap+1.0,"y":0},{"id":7,"el":"C","x":gap+2.0,"y":0}],
        "bonds":[{"id":3,"a":6,"b":2,"o":1},{"id":4,"a":9,"b":7,"o":1}],
        "objects":arrows})
    .to_string()
}

fn editor(text: &str) -> Editor {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e
}

fn px(e: &Editor, x: f64, y: f64) -> (f64, f64) {
    e.view.to_px(x, y)
}

fn drag(e: &mut Editor, from: (f64, f64), to: (f64, f64)) {
    e.pointer_down(from.0, from.1, MOUSE, 0);
    e.pointer_move((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0, MOUSE);
    e.pointer_move(to.0, to.1, MOUSE);
    e.pointer_up(to.0, to.1);
}

fn arrows(e: &Editor) -> Vec<(u32, u32, u32, [f64; 4])> {
    (e.doc().objects.iter())
        .map(|o| (o.id, o.from, o.to, [o.x, o.y, o.x2, o.y2]))
        .collect()
}

fn replay(request: Value) -> Value {
    serde_json::from_str(&mzdraw::replay::replay(&request.to_string()).unwrap()).unwrap()
}

#[test]
fn the_tool_anchors_an_arrow_drawn_from_one_structure_to_another_in_one_step() {
    let mut e = editor(&scheme(5.0, json!([])));
    let before = e.doc().to_json();
    e.set_tool("arrow");
    let (a, b) = (px(&e, 0.5, 0.2), px(&e, 6.5, 0.2));
    drag(&mut e, a, b);
    let list = arrows(&e);
    assert_eq!(list.len(), 1);
    let (_, from, to, [x, y, x2, y2]) = list[0];
    assert_eq!((from, to), (2, 7), "the smallest atom id of each structure");
    // routed: the ends are on the line between the centres, outside both boxes, and not where the hand left them
    let st = Style::default();
    let (ba, bb) = (
        structure_box(e.doc(), 2, &st).unwrap(),
        structure_box(e.doc(), 7, &st).unwrap(),
    );
    assert!(x > ba[2] && x2 < bb[0], "tail {x} head {x2}");
    assert!(
        (y - y2).abs() < 1e-9 && y.abs() < 1e-9,
        "level between the centres: {y} {y2}"
    );
    assert_eq!(e.said.0, "draw2.say.anchored");
    assert_eq!(e.said.1, "C2H6 \u{2192} C2H6");
    // one undo step takes the arrow, anchors and route together
    assert_eq!(e.undo() & mzdraw::editor::CHANGED, mzdraw::editor::CHANGED);
    assert_eq!(e.doc().to_json(), before);
}

#[test]
fn an_arrow_that_ends_on_empty_canvas_or_in_its_own_structure_is_free() {
    for (from, to) in [
        ((0.5, 0.2), (3.0, 0.2)),
        ((0.5, 0.2), (0.9, 0.1)),
        ((3.0, 2.0), (6.5, 0.2)),
    ] {
        let mut e = editor(&scheme(5.0, json!([])));
        e.set_tool("arrow");
        let (a, b) = (px(&e, from.0, from.1), px(&e, to.0, to.1));
        drag(&mut e, a, b);
        let list = arrows(&e);
        assert_eq!(list.len(), 1, "{from:?} -> {to:?}");
        assert_eq!((list[0].1, list[0].2), (0, 0), "{from:?} -> {to:?}");
        assert_eq!(e.said.0, "draw2.say.arrow");
    }
    // a click gives the standard free arrow
    let mut e = editor(&scheme(5.0, json!([])));
    e.set_tool("arrow");
    let a = px(&e, 0.5, 0.0);
    e.pointer_down(a.0, a.1, MOUSE, 0);
    e.pointer_up(a.0, a.1);
    assert_eq!((arrows(&e)[0].1, arrows(&e)[0].2), (0, 0));
}

#[test]
fn the_nearest_structure_is_by_box_distance_within_reach_and_ties_go_to_the_smallest_id() {
    let st = Style::default();
    let e = editor(&scheme(2.0, json!([])));
    let (ba, bb) = (
        structure_box(e.doc(), 2, &st).unwrap(),
        structure_box(e.doc(), 7, &st).unwrap(),
    );
    let mid = (ba[2] + bb[0]) / 2.0;
    // the same distance from both boxes: the smaller smallest-id (A: 2) wins
    let (id, d) = nearest_structure(e.doc(), mid, 0.0, 1.0, &st).unwrap();
    assert_eq!(id, 2);
    assert!((d - (bb[0] - ba[2]) / 2.0).abs() < 1e-9);
    // nearer to B: B
    assert_eq!(
        nearest_structure(e.doc(), mid + 0.2, 0.0, 1.0, &st)
            .unwrap()
            .0,
        7
    );
    // out of reach
    assert!(nearest_structure(e.doc(), mid, 5.0, 1.0, &st).is_none());
    // inside a box: the point is held, at distance 0
    assert_eq!(structure_at(e.doc(), 0.5, 0.0, &st), Some(2));
    assert_eq!(structure_at(e.doc(), mid, 0.0, &st), None);
}

#[test]
fn the_ids_of_a_tie_do_not_depend_on_the_order_of_the_atoms() {
    // B first in the pool with the smaller id: the winner is the smaller id, not the earlier atom
    let text = json!({"format":"mzdraw/1","next":20,
        "atoms":[{"id":9,"el":"C","x":3,"y":0},{"id":4,"el":"C","x":4,"y":0},{"id":6,"el":"C","x":0,"y":0},{"id":5,"el":"C","x":1,"y":0}],
        "bonds":[{"id":10,"a":9,"b":4,"o":1},{"id":11,"a":6,"b":5,"o":1}],"objects":[]})
    .to_string();
    let st = Style::default();
    let e = editor(&text);
    let (ba, bb) = (
        structure_box(e.doc(), 5, &st).unwrap(),
        structure_box(e.doc(), 4, &st).unwrap(),
    );
    let mid = (ba[2] + bb[0]) / 2.0;
    assert_eq!(nearest_structure(e.doc(), mid, 0.0, 1.0, &st).unwrap().0, 4);
}

fn free_arrow_scheme() -> String {
    // the tail is one bond length from A (its box ends at x = 1), the head is far from everything
    scheme(
        2.0,
        json!([{"id":10,"t":"arrow","x":1.8,"y":0.0,"x2":12.0,"y2":0.0}]),
    )
}

#[test]
fn pathway_anchor_anchors_the_ends_in_reach_and_says_what_stays_free() {
    let mut e = editor(&free_arrow_scheme());
    let before = e.doc().to_json();
    let steps = e.session.steps();
    let (mx, my) = {
        let p = px(&e, 8.0, 0.0);
        (p.0, p.1)
    };
    let menu: Value = serde_json::from_str(&e.context_menu(mx, my)).unwrap();
    assert_eq!(menu["target"], "object");
    let ids: Vec<&str> = menu["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert!(
        ids.contains(&"pathway.anchor") && ids.contains(&"pathway.free"),
        "{ids:?}"
    );
    let free = menu["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == "pathway.free")
        .unwrap();
    assert_eq!(free["enabled"], false, "nothing to free yet");
    e.context_run("pathway.anchor");
    let list = arrows(&e);
    assert_eq!(
        (list[0].1, list[0].2),
        (2, 0),
        "tail anchored to A, head free"
    );
    assert_eq!(e.said.0, "draw2.say.anchorOne");
    assert_eq!(e.session.steps(), steps + 1);
    assert_eq!(e.undo() & mzdraw::editor::CHANGED, mzdraw::editor::CHANGED);
    assert_eq!(e.doc().to_json(), before);
}

#[test]
fn nothing_in_reach_is_no_step() {
    let mut e = editor(&scheme(
        2.0,
        json!([{"id":10,"t":"arrow","x":5.0,"y":4.0,"x2":9.0,"y2":4.0}]),
    ));
    let before = e.doc().to_json();
    let steps = e.session.steps();
    let p = px(&e, 7.0, 4.0);
    e.context_menu(p.0, p.1);
    e.context_run("pathway.anchor");
    assert_eq!(e.said.0, "draw2.say.anchorNone");
    assert_eq!(e.session.steps(), steps, "no step");
    assert_eq!(e.doc().to_json(), before);
}

#[test]
fn pathway_anchor_anchors_both_ends_and_a_second_run_changes_nothing() {
    let text = scheme(
        5.0,
        json!([{"id":10,"t":"arrow","x":1.6,"y":0.5,"x2":5.4,"y2":-0.5}]),
    );
    let req = |commands: Value| replay(json!({"import": text, "commands": commands, "out": "log"}));
    let once = req(json!([{"command":"pathway:anchor","args":{"arrow":10}}]));
    let twice = req(json!([
        {"command":"pathway:anchor","args":{"arrow":10}},
        {"command":"pathway:anchor","args":{"arrow":10}},
        {"command":"pathway:route"},
        {"command":"pathway:route"}
    ]));
    assert_eq!(
        once, twice,
        "a second anchor and a second route emit no operation"
    );
    let set = (once.as_array().unwrap().iter())
        .find(|o| o["op"] == "obj.set")
        .expect("the anchors are an obj.set of the log");
    assert_eq!(
        (set["from"].clone(), set["to"].clone()),
        (json!(2), json!(7))
    );
    // the arrow now leaves A and reaches B on the line between the centres
    let doc = replay(
        json!({"import": text, "commands":[{"command":"pathway:anchor","args":{"arrow":10}}], "out":"mzdraw"}),
    );
    let a = &doc["objects"][0];
    assert_eq!((a["from"].clone(), a["to"].clone()), (json!(2), json!(7)));
    let (y, y2) = (
        a["y"].as_f64().unwrap_or(0.0),
        a["y2"].as_f64().unwrap_or(0.0),
    );
    assert!((y - y2).abs() < 1e-9, "{y} {y2}");
}

#[test]
fn pathway_route_puts_every_anchored_arrow_back_and_is_idempotent() {
    // an anchored arrow moved by a bare operation is routed by the command, in one step
    let mut e = editor(&scheme(
        5.0,
        json!([{"id":10,"t":"arrow","x":1.6,"y":0.0,"x2":5.4,"y2":0.0,"from":2,"to":7}]),
    ));
    let routed = e.doc().to_json();
    e.session
        .run(&[mzdraw::Op::ObjSet {
            id: 10,
            text: None,
            runs: None,
            x2: Some(9.0),
            y2: Some(3.0),
            from: None,
            to: None,
            cv: Default::default(),
            pt: [None, None],
        }])
        .unwrap();
    let moved = e.doc().to_json();
    assert_ne!(moved, routed);
    let steps = e.session.steps();
    let p = px(&e, 20.0, 20.0);
    let menu: Value = serde_json::from_str(&e.context_menu(p.0, p.1)).unwrap();
    assert_eq!(menu["target"], "canvas");
    e.context_run("pathway.route");
    assert_eq!(e.said.0, "draw2.say.routed");
    assert_eq!(e.session.steps(), steps + 1);
    let once = e.doc().to_json();
    assert_ne!(once, moved);
    // a second run has nothing to move: no step
    e.context_menu(p.0, p.1);
    e.context_run("pathway.route");
    assert_eq!(e.said.0, "draw2.say.routeNone");
    assert_eq!(e.session.steps(), steps + 1);
    assert_eq!(e.doc().to_json(), once);
    // undo returns the identical document
    e.undo();
    assert_eq!(e.doc().to_json(), moved);
}

#[test]
fn free_ends_frees_both_anchors_and_undo_restores_them() {
    let mut e = editor(&scheme(
        5.0,
        json!([{"id":10,"t":"arrow","x":1.6,"y":0.0,"x2":5.4,"y2":0.0,"from":2,"to":7}]),
    ));
    let before = e.doc().to_json();
    let p = px(&e, 3.5, 0.0);
    e.context_menu(p.0, p.1);
    e.context_run("pathway.free");
    assert_eq!((arrows(&e)[0].1, arrows(&e)[0].2), (0, 0));
    e.undo();
    assert_eq!(e.doc().to_json(), before);
}

#[test]
fn a_command_on_something_that_is_not_an_arrow_is_refused() {
    let text = scheme(5.0, json!([{"id":10,"t":"plus","x":3.0,"y":0.0}]));
    // an id that is no object, and an object that is no arrow: the keys differ
    for (args, key) in [
        (json!({"arrow": 3}), "error.draw2.noObject"),
        (json!({"arrow": 99}), "error.draw2.noObject"),
        (json!({"arrow": 10}), "error.draw2.object"),
    ] {
        let r = replay(
            json!({"import": text, "commands":[{"command":"pathway:anchor","args":args}], "report": true}),
        );
        assert_eq!(r["ok"], false);
        assert_eq!(r["failed"]["stage"], "command");
        assert_eq!(r["failed"]["key"], key, "{args}");
        assert_eq!(r["failed"]["id"], args["arrow"]);
    }
    for bad in [
        json!({}),
        json!({"arrow": 0}),
        json!({"arrow": 1, "x": 2}),
        json!({"arrow": "a"}),
    ] {
        assert!(mzdraw::replay::replay(
            &json!({"import": text, "commands":[{"command":"pathway:anchor","args":bad}]})
                .to_string()
        )
        .is_err());
    }
    assert!(mzdraw::replay::replay(
        &json!({"import": text, "commands":[{"command":"pathway:route","args":{"x":1}}]})
            .to_string()
    )
    .is_err());
}

#[test]
fn the_page_and_the_native_replay_give_the_same_bytes() {
    // the page: the tool draws an arrow between two structures, the parent is moved by the keyboard, the arrow follows
    let mut e = editor(&scheme(5.0, json!([])));
    e.set_tool("arrow");
    let (a, b) = (px(&e, 0.5, 0.2), px(&e, 6.5, 0.2));
    drag(&mut e, a, b);
    e.set_tool("select");
    let p = px(&e, 0.0, 0.0);
    e.pointer_down(p.0, p.1, MOUSE, 0);
    e.pointer_up(p.0, p.1);
    e.key_at("ArrowUp", ALT, 0.0);
    let page_doc = e.doc().to_json();
    let log = e.save("log").unwrap();
    // the log replays natively to the same document
    let replayed = mzdraw::replay::replay(
        &json!({"log": serde_json::from_str::<Value>(&log).unwrap(), "out": "mzdraw"}).to_string(),
    )
    .unwrap();
    assert_eq!(replayed, page_doc);
    let d = Document::from_json(&replayed).unwrap();
    assert_eq!((d.objects[0].from, d.objects[0].to), (2, 7));
}

/// A drawing of `n` two-atom structures whose first atom sits at the given points (bond lengths); the smallest atom id of structure
/// `i` is `10 * i + 1`.
fn layout(points: &[(f64, f64)]) -> String {
    let mut atoms = Vec::new();
    let mut bonds = Vec::new();
    for (i, (x, y)) in points.iter().enumerate() {
        let a = 10 * i as u32 + 1;
        atoms.push(json!({"id": a, "el": "C", "x": x, "y": y}));
        atoms.push(json!({"id": a + 1, "el": "C", "x": x + 1.0, "y": *y}));
        bonds.push(json!({"id": a + 2, "a": a, "b": a + 1, "o": 1}));
    }
    json!({"format":"mzdraw/1","next":100,"atoms":atoms,"bonds":bonds,"objects":[]}).to_string()
}

/// The kill test of the slice: the arrow tasks (a two-step chain, a star of four products, a parent with three products), each arrow
/// drawn by ONE drag of the tool from the middle of one structure to the middle of another. Every arrow must be anchored to the
/// structures it was drawn between and to no other; the number of actions is the number of arrows.
#[test]
fn the_arrow_tasks_never_anchor_a_wrong_structure() {
    type Task = (&'static str, Vec<(f64, f64)>, Vec<(usize, usize)>);
    let tasks: [Task; 3] = [
        (
            "two-step chain",
            vec![(0.0, 0.0), (5.0, 0.0), (10.0, 0.0)],
            vec![(0, 1), (1, 2)],
        ),
        (
            "star of four",
            vec![
                (0.0, 0.0),
                (5.0, 4.0),
                (5.0, -4.0),
                (-6.0, 4.0),
                (-6.0, -4.0),
            ],
            vec![(0, 1), (0, 2), (0, 3), (0, 4)],
        ),
        (
            "parent with three products",
            vec![(0.0, 0.0), (6.0, 4.0), (6.0, 0.0), (6.0, -4.0)],
            vec![(0, 1), (0, 2), (0, 3)],
        ),
    ];
    let mut anchored = 0;
    let mut total = 0;
    for (name, points, pairs) in tasks {
        let mut e = editor(&layout(&points));
        e.set_tool("arrow");
        for &(a, b) in &pairs {
            // from the middle of structure a to the middle of structure b
            let from = px(&e, points[a].0 + 0.5, points[a].1);
            let to = px(&e, points[b].0 + 0.5, points[b].1);
            drag(&mut e, from, to);
            let o = e.doc().objects.last().unwrap();
            total += 1;
            if (o.from, o.to) == (10 * a as u32 + 1, 10 * b as u32 + 1) {
                anchored += 1;
            } else {
                panic!(
                    "{name}: arrow {a} -> {b} anchored {:?} -> {:?}",
                    o.from, o.to
                );
            }
        }
        // every anchored arrow is routed: its ends are outside the boxes it joins
        let st = Style::default();
        for o in &e.doc().objects {
            let (ba, bb) = (
                structure_box(e.doc(), o.from, &st).unwrap(),
                structure_box(e.doc(), o.to, &st).unwrap(),
            );
            for (x, y) in [(o.x, o.y), (o.x2, o.y2)] {
                for b in [ba, bb] {
                    let inside = x > b[0] && x < b[2] && y > b[1] && y < b[3];
                    assert!(!inside, "{name}: an end of arrow {} is inside a box", o.id);
                }
            }
        }
        assert_eq!(
            e.session.steps(),
            1 + pairs.len(),
            "{name}: one step per arrow (and the import)"
        );
    }
    eprintln!("kill test: {anchored}/{total} arrows anchored to the structures they were drawn between; 1 action each");
    assert_eq!(anchored, total);
}

#[test]
fn a_pathway_item_that_changes_nothing_is_no_change_and_does_not_say_it_anchored() {
    use mzdraw::editor::CHANGED;
    // already anchored and routed: Anchor and Route arrows and Free ends (once freed) move nothing
    let mut e = editor(&scheme(
        5.0,
        json!([{"id":10,"t":"arrow","x":1.6,"y":0.0,"x2":5.4,"y2":0.0,"from":2,"to":7}]),
    ));
    let p = px(&e, 3.5, 0.0);
    e.context_menu(p.0, p.1);
    assert_eq!(
        e.context_run("pathway.anchor") & CHANGED,
        CHANGED,
        "the first run routes it"
    );
    let (steps, doc) = (e.session.steps(), e.doc().to_json());
    e.context_menu(p.0, p.1);
    assert_eq!(e.context_run("pathway.anchor") & CHANGED, 0);
    assert_eq!(e.said.0, "draw2.say.anchorSame");
    let c = px(&e, 20.0, 20.0);
    e.context_menu(c.0, c.1);
    assert_eq!(e.context_run("pathway.route") & CHANGED, 0);
    assert_eq!(e.said.0, "draw2.say.routeNone");
    assert_eq!((e.session.steps(), e.doc().to_json()), (steps, doc));
    // Free ends on an arrow with no anchor is not offered enabled, and a run that reaches the module changes nothing
    let mut f = editor(&free_arrow_scheme());
    let p = px(&f, 8.0, 0.0);
    f.context_menu(p.0, p.1);
    assert_eq!(f.context_run("pathway.free") & CHANGED, 0);
    assert_eq!(f.session.steps(), 1);
}

#[test]
fn both_ends_in_reach_of_the_same_structure_anchor_only_the_nearer_end() {
    // the tail is 0.5 and the head 0.9 above structure A (B is far away): only the tail is anchored, the head stays free
    let text = scheme(
        9.0,
        json!([{"id":10,"t":"arrow","x":0.2,"y":0.5,"x2":0.9,"y2":0.9}]),
    );
    let r = replay(
        json!({"import": text, "commands":[{"command":"pathway:anchor","args":{"arrow":10}}], "out":"mzdraw"}),
    );
    let a = &r["objects"][0];
    assert_eq!(
        (a["from"].clone(), a["to"].as_u64().unwrap_or(0)),
        (json!(2), 0)
    );
}

#[test]
fn a_dry_run_reports_and_writes_nothing_and_an_expect_arm_proves_the_commands_ran() {
    let text = free_arrow_scheme();
    let cmd = json!([{"command":"pathway:anchor","args":{"arrow":10}},{"command":"pathway:route"}]);
    // the tail is anchored to the structure of atom 2, the head (far from everything) stays free: the arm says so, so the dry run
    // proves the commands ran, and a wrong arm is false in the same run
    let dry = replay(json!({"import": text, "commands": cmd, "dry_run": true,
        "expect": [{"anchored":{"arrow":10,"from":2}}, {"anchored":{"arrow":10,"from":2,"to":7}}]}));
    assert_eq!(dry["dry_run"], true);
    assert!(dry.get("out").is_none(), "a dry run writes no document");
    assert_eq!(dry["created"], json!({}));
    assert_eq!(dry["changed"], true);
    assert_eq!(
        (
            dry["expect"][0]["ok"].clone(),
            dry["expect"][1]["ok"].clone()
        ),
        (json!(true), json!(false))
    );
    assert_eq!(dry["ok"], false, "one false expectation is a verdict");
    // without the commands the first arm is false: the arm tells the commands from nothing
    let none = replay(
        json!({"import": text, "dry_run": true, "expect": [{"anchored":{"arrow":10,"from":2}}]}),
    );
    assert_eq!(none["expect"][0]["ok"], false);
    assert!(
        none.get("changed").is_none(),
        "no pathway command: no `changed`"
    );
    // the same request without dry_run does anchor
    let real = replay(json!({"import": text, "commands": cmd, "out": "mzdraw"}));
    assert_eq!(real["objects"][0]["from"], 2);
    // a refused command is a verdict in a dry run too
    let bad = replay(
        json!({"import": text, "commands":[{"command":"pathway:anchor","args":{"arrow":3}}], "dry_run": true}),
    );
    assert_eq!(
        (bad["ok"].clone(), bad["failed"]["stage"].clone()),
        (json!(false), json!("command"))
    );
    // an arm with a field outside its table is an invalid request
    let r = mzdraw::replay::replay(
        &json!({"import": text, "expect":[{"anchored":{"arrow":10,"x":1}}]}).to_string(),
    );
    assert_eq!(r.unwrap_err().key, "error.draw2.expect");
}

#[test]
fn changed_tells_a_real_route_from_a_no_op() {
    // an anchored arrow that is not on its route: the first route changes the drawing, the second does nothing
    let text = scheme(
        5.0,
        json!([{"id":10,"t":"arrow","x":1.2,"y":1.0,"x2":5.0,"y2":1.0,"from":2,"to":7}]),
    );
    let once =
        replay(json!({"import": text, "commands":[{"command":"pathway:route"}], "report": true}));
    assert_eq!(once["changed"], true);
    let twice = replay(json!({"import": text, "report": true,
        "commands":[{"command":"pathway:route"},{"command":"pathway:route"}]}));
    assert_eq!(
        twice["changed"], true,
        "the run as a whole changed the drawing"
    );
    let routed =
        replay(json!({"import": text, "commands":[{"command":"pathway:route"}], "out": "mzdraw"}));
    let again = replay(
        json!({"import": routed.to_string(), "commands":[{"command":"pathway:route"}], "report": true}),
    );
    assert_eq!(
        again["changed"], false,
        "an already routed drawing: a no-op"
    );
    let anchor_same = replay(json!({"import": routed.to_string(), "report": true,
        "commands":[{"command":"pathway:anchor","args":{"arrow":10}}]}));
    assert_eq!(anchor_same["changed"], false);
    // `changed` stands where the envelope puts it, before `expect`
    let raw = mzdraw::replay::replay(
        &json!({"import": text, "commands":[{"command":"pathway:route"}],
        "expect":[{"pieces":2}]})
        .to_string(),
    )
    .unwrap();
    assert!(
        raw.find("\"changed\"").unwrap() < raw.find("\"expect\"").unwrap(),
        "{raw}"
    );
}

/// A hostile size (2000 structures, 2000 arrows): many structures and as many arrows. Everything that asks for a box used to recompute the pieces of the whole
/// document each time (quadratic per question); now the pieces are computed once per command, so the route of every arrow, the
/// anchoring of one arrow and an arrow drawn with the tool stay quick. The bound is generous (a debug build is slower than release)
/// and the byte-for-byte results are those of the goldens.
#[test]
fn two_thousand_structures_and_arrows_stay_quick() {
    const N: usize = 2000;
    let mut atoms = Vec::new();
    let mut objects = Vec::new();
    // structures on a grid, one atom each (ids 1..=N), arrows chained from one to the next, anchored but displaced
    for i in 0..N {
        let (x, y) = ((i % 40) as f64 * 4.0, (i / 40) as f64 * 4.0);
        atoms.push(json!({"id": i + 1, "el": "C", "x": x, "y": y}));
    }
    for i in 0..N - 1 {
        let (x, y) = ((i % 40) as f64 * 4.0, (i / 40) as f64 * 4.0);
        objects.push(json!({"id": N + 1 + i, "t":"arrow","x":x+0.3,"y":y+0.4,"x2":x+3.0,"y2":y+0.4,"from":i+1,"to":i+2}));
    }
    let text = json!({"format":"mzdraw/1","next":2*N+2,"atoms":atoms,"bonds":[],"objects":objects})
        .to_string();
    let t0 = std::time::Instant::now();
    let r =
        replay(json!({"import": text, "report": true, "commands":[{"command":"pathway:route"}]}));
    let route = t0.elapsed();
    assert_eq!(r["ok"], true);
    let t1 = std::time::Instant::now();
    let r = replay(json!({"import": text, "report": true,
        "commands":[{"command":"pathway:anchor","args":{"arrow": N + 1}}]}));
    let anchor = t1.elapsed();
    assert_eq!(r["ok"], true);
    let mut e = editor(&text);
    e.set_tool("arrow");
    let t2 = std::time::Instant::now();
    let (a, b) = (px(&e, 0.0, 0.0), px(&e, 8.0, 0.0));
    drag(&mut e, a, b);
    let tool = t2.elapsed();
    let bound = std::time::Duration::from_secs(if cfg!(debug_assertions) { 20 } else { 2 });
    assert!(
        route < bound && anchor < bound && tool < bound,
        "route {route:?}, anchor {anchor:?}, tool {tool:?}"
    );
    eprintln!("{N} structures and arrows: route {route:?}, anchor {anchor:?}, tool {tool:?}");
}

/// `scheme:row` on 2000 structures chained by 1999 anchored arrows: the boxes are computed once per document state (it used to rebuild
/// the pieces for every structure and every arrow). The row is laid out and every arrow routed within the bound.
#[test]
fn scheme_row_on_two_thousand_structures_stays_quick() {
    const N: usize = 2000;
    let atoms: Vec<Value> = (0..N)
        .map(|i| json!({"id": i + 1, "el": "C", "x": (i % 40) as f64 * 4.0, "y": (i / 40) as f64 * 4.0}))
        .collect();
    let objects: Vec<Value> = (0..N - 1)
        .map(|i| json!({"id": N + 1 + i, "t":"arrow","x":0.0,"y":0.0,"x2":1.0,"y2":0.0,"from":i+1,"to":i+2}))
        .collect();
    let text = json!({"format":"mzdraw/1","next":2*N+2,"atoms":atoms,"bonds":[],"objects":objects})
        .to_string();
    let t0 = std::time::Instant::now();
    let r = replay(json!({"import": text, "report": true, "commands":[{"command":"scheme:row"}]}));
    let took = t0.elapsed();
    assert_eq!(r["ok"], true, "{r}");
    let bound = std::time::Duration::from_secs(if cfg!(debug_assertions) { 10 } else { 1 });
    assert!(took < bound, "scheme:row on {N} structures took {took:?}");
    eprintln!("scheme:row on {N} structures and arrows: {took:?}");
}
