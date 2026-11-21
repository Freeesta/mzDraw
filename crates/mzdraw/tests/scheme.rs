//! `scheme:row` (plan-13): the one-line layout of a reaction scheme. Synthetic structures only.

use mzdraw::route::structure_box;
use mzdraw::scheme::{plus_box, row, text_box, Opts};
use mzdraw::{Document, Editor, ObjKind, Session, Style};
use serde_json::{json, Value};

type Rect = [f64; 4];

/// A messy scheme drawn by hand: two reactants and a plus sign on top of each other, an arrow with an agent (a lone chlorine) and two
/// texts about it, and a product. Ids: atoms 1 to 6 and 12 to 13, bonds 7, 8, 14, objects 9 to 11 and 15 to 16.
fn messy() -> Value {
    json!({"format":"mzdraw/1","next":17,
        "atoms":[
            {"id":1,"el":"C","x":0,"y":0},{"id":2,"el":"C","x":1,"y":0},{"id":3,"el":"O","x":1.5,"y":0.87},
            {"id":4,"el":"N","x":1.2,"y":-0.2},{"id":5,"el":"C","x":2.2,"y":-0.2},
            {"id":6,"el":"Cl","x":5.2,"y":1.6},
            {"id":12,"el":"C","x":8.4,"y":0},{"id":13,"el":"O","x":9.4,"y":0}],
        "bonds":[{"id":7,"a":1,"b":2,"o":1},{"id":8,"a":2,"b":3,"o":1},{"id":10,"a":4,"b":5,"o":1},{"id":14,"a":12,"b":13,"o":2}],
        "objects":[
            {"id":9,"t":"plus","x":1.3,"y":0.1},
            {"id":11,"t":"arrow","x":4,"y":0.5,"x2":7.5,"y2":0.5},
            {"id":15,"t":"text","x":5.0,"y":1.0,"text":"Pd/C, H2"},
            {"id":16,"t":"text","x":4.6,"y":-0.3,"text":"THF, 25 C"}]})
}

fn load(v: &Value) -> Document {
    Document::from_json(&v.to_string()).unwrap()
}

/// The boxes of everything that is not an arrow (structures, plus signs, texts), and the arrows as their segments.
fn boxes(d: &Document, style: &Style) -> (Vec<Rect>, Vec<Rect>) {
    let mut things = Vec::new();
    for p in mzdraw::layout::pieces(d) {
        things.push(structure_box(d, p[0], style).unwrap());
    }
    let mut arrows = Vec::new();
    for o in &d.objects {
        match o.t {
            ObjKind::Plus => things.push(plus_box(d, o.id).unwrap()),
            ObjKind::Text => things.push(text_box(d, o.id, style).unwrap()),
            ObjKind::Arrow => {
                arrows.push([o.x.min(o.x2), o.y.min(o.y2), o.x.max(o.x2), o.y.max(o.y2)])
            }
        }
    }
    (things, arrows)
}

fn overlap(a: Rect, b: Rect, eps: f64) -> bool {
    a[0] < b[2] - eps && b[0] < a[2] - eps && a[1] < b[3] - eps && b[1] < a[3] - eps
}

fn overlaps(d: &Document, style: &Style) -> usize {
    let (things, arrows) = boxes(d, style);
    let all: Vec<Rect> = things.iter().chain(&arrows).copied().collect();
    let mut n = 0;
    for i in 0..all.len() {
        for j in i + 1..all.len() {
            // a line of no height still crosses a box
            let thick = |r: Rect| [r[0], r[1] - 1e-6, r[2], r[3] + 1e-6];
            n += overlap(thick(all[i]), thick(all[j]), 1e-9) as usize;
        }
    }
    n
}

fn run(d: &Document, opts: &Opts) -> Document {
    let style = Style::default();
    let ops = row(d, &style, opts).unwrap();
    let mut s = Session::new(d.clone());
    s.run(&ops).unwrap();
    s.doc
}

/// The element symbols of each molfile of a reaction file, by role.
fn roles(rxn: &str) -> Vec<Vec<String>> {
    rxn.split("$MOL\n")
        .skip(1)
        .map(|m| {
            let n: usize = m.lines().nth(3).unwrap()[0..3].trim().parse().unwrap();
            (m.lines().skip(4).take(n))
                .map(|l| l[31..34].trim().to_string())
                .collect()
        })
        .collect()
}

#[test]
fn the_messy_scheme_overlaps_before_and_not_after() {
    let style = Style::default();
    let d = load(&messy());
    assert!(
        overlaps(&d, &style) >= 3,
        "the scheme is laid out by hand with overlaps"
    );
    let after = run(&d, &Opts::default());
    assert_eq!(overlaps(&after, &style), 0);
}

#[test]
fn gaps_are_equal_and_every_structure_is_on_the_arrow_line() {
    let style = Style::default();
    for gap in [1.0, 0.6, 1.7] {
        let after = run(
            &load(&messy()),
            &Opts {
                gap,
                ..Opts::default()
            },
        );
        let arrow = after
            .objects
            .iter()
            .find(|o| o.t == ObjKind::Arrow)
            .unwrap();
        assert!((arrow.y - arrow.y2).abs() < 1e-9 && arrow.x2 > arrow.x);
        // the row: reactants, plus, reactant, arrow, product (the agent and the texts are notes of the arrow)
        let mut row_items: Vec<Rect> = Vec::new();
        for p in mzdraw::layout::pieces(&after) {
            let b = structure_box(&after, p[0], &style).unwrap();
            let (cx, cy) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
            if cx > arrow.x && cx < arrow.x2 {
                // an agent: centred on the arrow, above or below it
                assert!((cx - (arrow.x + arrow.x2) / 2.0).abs() < 2e-4);
                assert!((cy - arrow.y).abs() > 0.2);
                continue;
            }
            assert!((cy - arrow.y).abs() < 2e-4, "structure centred on the line");
            row_items.push(b);
        }
        row_items.push(plus_box(&after, 9).unwrap());
        row_items.push([arrow.x, arrow.y, arrow.x2, arrow.y]);
        row_items.sort_by(|a, b| a[0].total_cmp(&b[0]));
        assert_eq!(row_items.len(), 5);
        for w in row_items.windows(2) {
            // coordinates are kept to four decimals: a gap is exact to that
            assert!((w[1][0] - w[0][2] - gap).abs() < 3e-4, "gap {gap}: {:?}", w);
        }
        // the notes of the arrow are inside its length and clear of its line
        for id in [15, 16] {
            let t = text_box(&after, id, &style).unwrap();
            assert!(t[0] >= arrow.x && t[2] <= arrow.x2);
            assert!(t[1] > arrow.y || t[3] < arrow.y);
        }
        assert_eq!(overlaps(&after, &style), 0);
    }
}

#[test]
fn the_roles_of_the_reaction_file_are_kept() {
    let d = load(&messy());
    let before = roles(&d.to_rxn().unwrap());
    let after = roles(&run(&d, &Opts::default()).to_rxn().unwrap());
    assert_eq!(before, after);
    assert_eq!(
        after.len(),
        4,
        "two reactants, one agent, one product; the plus has no place"
    );
}

#[test]
fn one_undo_step_and_a_second_row_changes_nothing() {
    let d = load(&messy());
    let style = Style::default();
    let mut s = Session::new(d.clone());
    s.run(&row(&d, &style, &Opts::default()).unwrap()).unwrap();
    let laid = s.doc.clone();
    assert!(s.undo());
    assert_eq!(s.doc, d, "undo returns the identical document");
    assert!(!s.undo(), "the row was one step");
    assert!(s.redo());
    assert_eq!(s.doc, laid);
    assert!(
        row(&laid, &style, &Opts::default()).unwrap().is_empty(),
        "idempotent"
    );
}

#[test]
fn the_page_command_and_the_cli_command_give_the_same_bytes() {
    let mut e = Editor::default();
    e.import(&messy().to_string()).unwrap();
    assert!(e.scheme_row() != 0);
    let page = e.save("mzdraw").unwrap();
    for request in [
        json!({"import": messy().to_string(), "command": "scheme:row", "args": {}}),
        json!({"import": messy().to_string(), "commands": [{"command": "scheme:row", "args": {"gap": 1, "wrap": 0}}]}),
        // the log the page wrote, replayed with no command
        json!({"log": serde_json::from_str::<Value>(&e.session.log_json()).unwrap()}),
    ] {
        assert_eq!(mzdraw::replay::replay(&request.to_string()).unwrap(), page);
    }
    // the log carries moves and head sets after the import, no new op
    let log: Value = serde_json::from_str(&e.session.log_json()).unwrap();
    let names: Vec<&str> = log
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["op"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"move") && names.contains(&"obj.set"));
    assert!(
        names.iter().all(
            |n| ["add", "bond", "obj.add", "move", "obj.set", "atom.add", "bond.add"].contains(n)
        ),
        "{names:?}"
    );
}

#[test]
fn wrap_starts_the_next_line_under_the_arrow() {
    let style = Style::default();
    // three steps: A -> B -> C -> D
    let mut v = json!({"format":"mzdraw/1","next":20,"atoms":[],"bonds":[],"objects":[]});
    for k in 0..4 {
        let (a, x) = (1 + 2 * k, 3.0 * k as f64 + (k % 2) as f64 * 0.3);
        v["atoms"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":a,"el":"C","x":x,"y":0.1 * k as f64}));
        v["atoms"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":a + 1,"el":"O","x":x + 1.0,"y":0.1 * k as f64}));
        v["bonds"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":10 + k,"a":a,"b":a + 1,"o":1}));
    }
    for k in 0..3 {
        v["objects"].as_array_mut().unwrap().push(json!({"id":14 + k,"t":"arrow","x":3.0 * k as f64 + 1.5,"y":0,"x2":3.0 * k as f64 + 2.5,"y2":0}));
    }
    let d = load(&v);
    let flat = run(&d, &Opts::default());
    let wrapped = run(
        &d,
        &Opts {
            wrap: 8.0,
            ..Opts::default()
        },
    );
    let ys = |d: &Document| {
        let mut y: Vec<i64> = d
            .objects
            .iter()
            .map(|o| (o.y * 1000.0).round() as i64)
            .collect();
        y.dedup();
        y.len()
    };
    assert_eq!(ys(&flat), 1, "one line");
    assert!(ys(&wrapped) >= 2, "a second line");
    assert_eq!(overlaps(&wrapped, &style), 0);
    // the second line is under the first and starts at its left end
    let left = |d: &Document| d.atoms.iter().map(|a| a.x).fold(f64::MAX, f64::min);
    assert!((left(&wrapped) - left(&flat)).abs() < 1e-9 || left(&wrapped) <= left(&flat) + 1e-9);
    assert!(wrapped.bounds().unwrap().1 < flat.bounds().unwrap().1);
}

#[test]
fn a_selection_moves_only_itself_and_an_anchored_arrow_is_routed() {
    let style = Style::default();
    let mut v = messy();
    // anchor the arrow to the first reactant and the product
    v["objects"][1]["from"] = json!(2);
    v["objects"][1]["to"] = json!(12);
    let d = load(&v);
    let after = run(&d, &Opts::default());
    let arrow = after.object(11).unwrap();
    assert_eq!((arrow.from, arrow.to), (2, 12));
    assert!(
        mzdraw::route::route(&after, 11, &style).is_empty(),
        "the anchored arrow stands on its routed ends"
    );
    // the selection: the product alone is the only thing of its row; nothing else moves
    let sel = Opts {
        atoms: vec![12],
        ..Opts::default()
    };
    let ops = row(&d, &style, &sel).unwrap();
    assert!(
        ops.iter()
            .all(|o| matches!(o, mzdraw::Op::Move { atoms, .. } if atoms == &vec![12, 13]))
            || ops.is_empty()
    );
}

#[test]
fn bad_arguments_are_an_invalid_request() {
    for args in [
        json!({"gap": 0}),
        json!({"gap": -1}),
        json!({"wrap": -2}),
        json!({"gap": "1"}),
        json!({"x": 1}),
        json!({"atoms": [-1]}),
    ] {
        let r = json!({"import": messy().to_string(), "command": "scheme:row", "args": args});
        assert_eq!(
            mzdraw::replay::replay(&r.to_string()).unwrap_err().key,
            "error.draw2.request",
            "{args}"
        );
    }
}
