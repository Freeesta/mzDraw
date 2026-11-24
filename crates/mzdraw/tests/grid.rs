//! `grid` (rdkit-compare 8): the structures of a drawing in rows and columns, each with its legend under it. Every cell is as wide as
//! the widest structure or caption, the legends of a row stand on one line, nothing of one cell reaches another, the order is that of
//! the drawing, of reading or of the titles, a second grid gives no operation, one undo step reverts it, and the CLI replay gives the
//! page's bytes. Synthetic structures and names only.

use mzdraw::grid::{grid, Opts, Order};
use mzdraw::route::structure_box;
use mzdraw::{Document, Editor, Style};
use serde_json::{json, Value};

#[path = "support/timing.rs"]
mod timing;

/// Five named structures (a SMILES file: each name is the title of its structure), drawn where the import puts them.
const NAMED: &str = "CCO TP10\nc1ccccc1O TP2\nCC(C)(C)C(=O)O parent compound with a long name\nOCC(O)CO TP1\nNC(=O)c1ccccc1 TP3";

fn editor(text: &str, set: &[&str]) -> Editor {
    let mut e = Editor::default();
    for s in set {
        e.set(s).unwrap();
    }
    e.import(text).unwrap();
    e
}

fn run(e: &mut Editor, args: Value) -> Result<(), &'static str> {
    let ops = grid(e.doc(), &e.style, &[], &Opts::parse(&args)?)?.ops;
    e.session.run(&ops).map_err(|x| x.key)
}

/// The legend of each structure (its title), by its first atom.
fn title_at(d: &Document, atom: u32) -> String {
    d.atom(atom).unwrap().title()
}

/// Each structure: its first atom, the middle of its atoms, its lowest atom, and the box of its ink and of its caption in document
/// units (the caption as deep and as wide as the pictures give it).
fn cells(d: &Document, st: &Style) -> Vec<(u32, f64, f64, [f64; 4])> {
    let scale = st.bond_length;
    let cf = mzdraw::values::caption_font(st.metrics(scale, false).font);
    let caps = mzdraw::values::captions_titled(d, st.values, &st.adducts, &[], st.titles);
    mzdraw::layout::pieces(d)
        .into_iter()
        .map(|p| {
            let first = *p
                .iter()
                .min_by_key(|id| d.atoms.iter().position(|a| a.id == **id))
                .unwrap();
            let xs: Vec<f64> = p.iter().map(|id| d.atom(*id).unwrap().x).collect();
            let low = p
                .iter()
                .map(|id| d.atom(*id).unwrap().y)
                .fold(f64::MAX, f64::min);
            let cx = (xs.iter().cloned().fold(f64::MAX, f64::min)
                + xs.iter().cloned().fold(f64::MIN, f64::max))
                / 2.0;
            let mut b = structure_box(d, first, st).unwrap();
            if let Some(c) = caps.iter().find(|c| c.anchor == first) {
                let depth = (mzdraw::values::caption_first(scale, cf, c.below)
                    + (c.lines.len() - 1) as f64 * mzdraw::values::caption_step(cf)
                    + cf * 0.3)
                    / scale;
                let half = (c.lines.iter())
                    .map(|l| mzdraw::scene::width(l, cf))
                    .fold(0.0, f64::max)
                    / 2.0
                    / scale;
                b = [
                    b[0].min(c.x - half),
                    b[1].min(c.y - depth),
                    b[2].max(c.x + half),
                    b[3],
                ];
            }
            (first, cx, low, b)
        })
        .collect()
}

fn overlap(a: &[f64; 4], b: &[f64; 4]) -> bool {
    a[0] < b[2] - 1e-6 && b[0] < a[2] - 1e-6 && a[1] < b[3] - 1e-6 && b[1] < a[3] - 1e-6
}

#[test]
fn the_cells_are_in_rows_and_columns_with_the_legends_of_a_row_on_one_line_and_nothing_reaches_another_cell(
) {
    for (set, args, cols) in [
        (vec!["display.titles=on"], json!({}), 3),
        (vec!["display.titles=on"], json!({"cols": 2, "gap": 0.5}), 2),
        (
            vec!["display.titles=on", "display.values=formula"],
            json!({"cols": 4}),
            4,
        ),
        (vec![], json!({"cols": 1}), 1),
        (vec!["display.titles=on"], json!({"cols": 9}), 5),
    ] {
        let mut e = editor(NAMED, &set);
        run(&mut e, args.clone()).unwrap();
        let st = e.style.clone();
        let c = cells(e.doc(), &st);
        assert_eq!(c.len(), 5);
        // rows: the lowest atoms of a row on one line (so its legends are), `cols` structures each, the rows going down
        let mut lows: Vec<f64> = c.iter().map(|x| x.2).collect();
        lows.sort_by(|a, b| b.partial_cmp(a).unwrap());
        lows.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
        assert_eq!(lows.len(), 5usize.div_ceil(cols), "{args} {lows:?}");
        // columns: the middles of the structures stand on equally spaced columns, one per place in a row
        let mut xs: Vec<f64> = c.iter().map(|x| x.1).collect();
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        xs.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
        assert_eq!(xs.len(), cols.min(5), "{args} {xs:?}");
        for w in xs.windows(3) {
            assert!(
                ((w[2] - w[1]) - (w[1] - w[0])).abs() < 2e-4,
                "{args} {xs:?}"
            );
        }
        // nothing of a cell (ink, labels, legend, values) reaches another
        for i in 0..c.len() {
            for j in i + 1..c.len() {
                assert!(!overlap(&c[i].3, &c[j].3), "{args}: {:?} {:?}", c[i], c[j]);
            }
        }
        // the gap between two neighbours of a row is at least `gap`
        let gap = args.get("gap").and_then(Value::as_f64).unwrap_or(1.0);
        for a in &c {
            for b in &c {
                if (a.2 - b.2).abs() < 1e-3 && b.1 > a.1 + 1e-3 {
                    assert!(b.3[0] - a.3[2] >= gap - 1e-3 || cols == 1, "{args}");
                }
            }
        }
    }
}

#[test]
fn the_order_is_that_of_the_drawing_of_the_titles_or_of_reading_and_a_second_grid_moves_nothing() {
    let reading = |d: &Document, st: &Style| -> Vec<String> {
        let mut c = cells(d, st);
        c.sort_by(|a, b| (-a.2, a.1).partial_cmp(&(-b.2, b.1)).unwrap());
        c.iter().map(|x| title_at(d, x.0)).collect()
    };
    let mut e = editor(NAMED, &["display.titles=on"]);
    run(&mut e, json!({"order": "title"})).unwrap();
    let st = e.style.clone();
    // numbers compared as numbers: TP2 before TP10; the parent (no number) after them, by its letters
    assert_eq!(
        reading(e.doc(), &st),
        [
            "TP1",
            "TP2",
            "TP3",
            "TP10",
            "parent compound with a long name"
        ]
    );
    // laid out again, nothing moves: by title, by reading (the grid read row by row is the same order), with the same arguments
    for args in [json!({"order": "title"}), json!({"order": "reading"})] {
        let ops = grid(e.doc(), &st, &[], &Opts::parse(&args).unwrap())
            .unwrap()
            .ops;
        assert!(ops.is_empty(), "{args}: {ops:?}");
    }
    // the order of the drawing: the order of the SMILES lines
    run(&mut e, json!({"order": "drawing"})).unwrap();
    assert_eq!(
        reading(e.doc(), &st),
        [
            "TP10",
            "TP2",
            "parent compound with a long name",
            "TP1",
            "TP3"
        ]
    );
    assert!(grid(e.doc(), &st, &[], &Opts::default())
        .unwrap()
        .ops
        .is_empty());
    // titles are the order key even when they are not drawn; structures without a title come last, in the order of the drawing
    let mut e = editor("CCN\nCCO b\nCCC\nCCCl a", &[]);
    run(&mut e, json!({"order": "title", "cols": 4})).unwrap();
    let mut c = cells(e.doc(), &Style::default());
    c.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let el = |x: &(u32, f64, f64, [f64; 4])| {
        let mut els: Vec<String> = (e.doc().atoms.iter())
            .filter(|a| {
                mzdraw::layout::pieces(e.doc())
                    .iter()
                    .any(|p| p.contains(&x.0) && p.contains(&a.id))
            })
            .map(|a| a.el.clone())
            .collect();
        els.sort();
        els.join("")
    };
    assert_eq!(
        c.iter().map(el).collect::<Vec<_>>(),
        ["CCCl", "CCO", "CCN", "CCC"]
    );
}

#[test]
fn the_selection_alone_moves_and_a_bad_request_is_refused() {
    let mut e = editor(NAMED, &["display.titles=on"]);
    let before = e.doc().clone();
    // the atoms of two structures (ethanol: atom 1, benzamide: its first atom)
    let last = e.doc().atoms.last().unwrap().id;
    run(&mut e, json!({"atoms": [1, last]})).unwrap();
    let moved: Vec<u32> = (e.doc().atoms.iter())
        .filter(|a| before.atom(a.id).is_some_and(|b| (b.x, b.y) != (a.x, a.y)))
        .map(|a| a.id)
        .collect();
    let pieces = mzdraw::layout::pieces(&before);
    assert!(!moved.is_empty());
    for p in &pieces {
        // a structure outside the selection stays; one in it moves as a whole or not at all
        let chosen = p.contains(&1) || p.contains(&last);
        let n = p.iter().filter(|a| moved.contains(a)).count();
        assert!(
            if chosen {
                n == 0 || n == p.len()
            } else {
                n == 0
            },
            "{p:?}"
        );
    }
    let d = e.doc().clone();
    assert_eq!(
        run(&mut e, json!({"atoms": [9999]})),
        Err("error.draw2.noAtom")
    );
    for bad in [
        json!({"cols": 0}),
        json!({"gap": 0}),
        json!({"order": "size"}),
        json!({"wrap": 1}),
    ] {
        assert_eq!(
            run(&mut e, bad.clone()),
            Err("error.draw2.request"),
            "{bad}"
        );
    }
    assert_eq!(*e.doc(), d, "a refused grid changes nothing");
    // an empty drawing: nothing to do
    assert!(grid(
        &Document::default(),
        &Style::default(),
        &[],
        &Opts::default()
    )
    .unwrap()
    .ops
    .is_empty());
    let o = Opts::parse(&json!({"order": "reading"})).unwrap();
    assert_eq!(o.order, Order::Reading);
}

#[test]
fn the_button_is_one_undo_step_and_the_cli_replay_gives_the_page_bytes() {
    let mut e = editor(NAMED, &["display.titles=on"]);
    let start = e.save("mzdraw").unwrap();
    assert!(e.grid() != 0);
    let page = e.save("mzdraw").unwrap();
    assert_ne!(page, start);
    // a second press changes nothing and says so
    e.grid();
    assert_eq!(e.said.0, "draw2.say.gridDone");
    assert_eq!(e.save("mzdraw").unwrap(), page);
    // one undo step back to the identical document
    e.undo();
    assert_eq!(e.save("mzdraw").unwrap(), start);
    // the CLI: the command with its defaults, and the log the page wrote, replay to the page's bytes
    let request =
        json!({"import": NAMED, "set": ["display.titles=on"], "command": "grid", "args": {}});
    assert_eq!(mzdraw::replay::replay(&request.to_string()).unwrap(), page);
    // redo gives the page's bytes again
    e.redo();
    assert_eq!(e.save("mzdraw").unwrap(), page);
    let log: Value = serde_json::from_str(&e.session.log_json()).unwrap();
    assert_eq!(
        mzdraw::replay::replay(&json!({"log": log}).to_string()).unwrap(),
        page
    );
    let names: Vec<&str> = (log.as_array().unwrap().iter())
        .map(|o| o["op"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"move"), "{names:?}");
    // a dry run answers `draw-result/1` and writes nothing; a refused argument is a verdict
    let dry = json!({"import": NAMED, "command": "grid", "args": {"cols": 2}, "dry_run": true});
    let r: Value =
        serde_json::from_str(&mzdraw::replay::replay(&dry.to_string()).unwrap()).unwrap();
    assert_eq!(
        (r["format"].as_str(), r["ok"].as_bool()),
        (Some("draw-result/1"), Some(true))
    );
    let bad = json!({"import": NAMED, "command": "grid", "args": {"cols": "two"}});
    assert_eq!(
        mzdraw::replay::replay(&bad.to_string()).unwrap_err().key,
        "error.draw2.request"
    );
}

#[test]
fn an_anchored_arrow_follows_its_structures() {
    let mut e = editor("CCO.c1ccccc1", &[]);
    let (a, b) = (1, e.doc().atoms.last().unwrap().id);
    let arrow = json!({"op": "obj.add", "t": "arrow", "x": 0.0, "y": 0.0, "x2": 1.0, "y2": 0.0, "from": a, "to": b});
    e.session
        .run(&[mzdraw::Op::parse(&arrow).unwrap()])
        .unwrap();
    e.over.sel_atoms.clear();
    let before = e.doc().objects[0].clone();
    assert!(e.grid() != 0);
    let after = &e.doc().objects[0];
    assert_ne!(
        (before.x, before.y, before.x2, before.y2),
        (after.x, after.y, after.x2, after.y2)
    );
    let routed = mzdraw::route::route(e.doc(), after.id, &e.style);
    assert!(
        routed.is_empty(),
        "the arrow is on its routed ends: {routed:?}"
    );
    // it keeps its atoms, and each end stands by the structure of its atom where the grid put it (re-routed as a straight line)
    assert_eq!((after.from, after.to), (a, b));
    let gap = |p: (f64, f64), r: [f64; 4]| {
        let dx = (r[0] - p.0).max(p.0 - r[2]).max(0.0);
        let dy = (r[1] - p.1).max(p.1 - r[3]).max(0.0);
        dx.hypot(dy)
    };
    let (ba, bb) = (
        structure_box(e.doc(), a, &e.style).unwrap(),
        structure_box(e.doc(), b, &e.style).unwrap(),
    );
    assert!(
        gap((after.x, after.y), ba) < 1.0 && gap((after.x2, after.y2), bb) < 1.0,
        "{after:?} {ba:?} {bb:?}"
    );
    assert!(
        e.grid_left().is_empty(),
        "an anchored arrow is not left behind"
    );
}

#[test]
fn the_work_grows_with_the_structures_not_with_their_square() {
    // the same named structure 100 and 400 times: one pass over the structures and their captions (a search of the captions per
    // structure would make four times the structures cost sixteen times as much). The least CPU time of runs taken in turns
    let many = |n: usize| -> Editor {
        let text: Vec<String> = (0..n)
            .map(|k| format!("OC1CCC(N)CC1C=CC#N TP{k}"))
            .collect();
        editor(&text.join("\n"), &["display.titles=on"])
    };
    let (a, b) = (many(100), many(400));
    let opts = Opts {
        order: Order::Title,
        ..Opts::default()
    };
    let (small, large) = timing::best_of_pair(
        1,
        5,
        || drop(grid(a.doc(), &a.style, &[], &opts).unwrap()),
        || drop(grid(b.doc(), &b.style, &[], &opts).unwrap()),
    );
    assert!(
        large.as_nanos() <= 8 * small.as_nanos(),
        "100 structures {small:?}, 400 structures {large:?}"
    );
}

#[test]
fn a_grid_of_a_selection_keeps_off_the_structures_outside_it() {
    // the case of the review: benzene left out, hexane and ethanol selected; and more: titles, columns, orders
    for (text, set, args) in [
        ("CCCCCC.c1ccccc1.CCO", vec![], json!({})),
        ("CCCCCC.c1ccccc1.CCO", vec![], json!({"cols": 1})),
        (
            "CCCCCC.c1ccccc1.CCO",
            vec!["display.titles=on"],
            json!({"cols": 2, "order": "reading"}),
        ),
        (
            "c1ccccc1.CCCCCC.CCO.CCN.CC(C)C",
            vec![],
            json!({"cols": 3, "gap": 0.2}),
        ),
    ] {
        let mut e = editor(text, &set);
        let pieces = mzdraw::layout::pieces(e.doc());
        // the ring: the piece with as many bonds as atoms
        let benzene = (pieces.iter())
            .find(|p| e.doc().bonds.iter().filter(|b| p.contains(&b.a)).count() == p.len())
            .unwrap()
            .clone();
        let selected: Vec<u32> = pieces
            .iter()
            .filter(|p| **p != benzene)
            .map(|p| p[0])
            .collect();
        let mut args = args.clone();
        args["atoms"] = json!(selected);
        let before = e.doc().clone();
        run(&mut e, args.clone()).unwrap();
        let st = e.style.clone();
        let c = cells(e.doc(), &st);
        let outside = c.iter().find(|x| benzene.contains(&x.0)).unwrap();
        // the structure outside is where it was
        for a in &benzene {
            assert_eq!(before.atom(*a), e.doc().atom(*a));
        }
        assert_ne!(before, *e.doc(), "{text} {args}: the selection moved");
        for x in c.iter().filter(|x| !benzene.contains(&x.0)) {
            assert!(
                !overlap(&x.3, &outside.3),
                "{text} {args}: {x:?} over the benzene {outside:?}"
            );
        }
        // a second grid moves nothing
        let again = grid(e.doc(), &st, &[], &Opts::parse(&args).unwrap()).unwrap();
        assert!(again.ops.is_empty(), "{text} {args}: {:?}", again.ops);
    }
}

#[test]
fn free_objects_stay_where_they_are_are_kept_off_and_are_reported() {
    let mut e = editor(NAMED, &["display.titles=on"]);
    // by ethanol (TP10): a text, a free arrow and a plus sign; far from everything: a text
    let (x, y) = {
        let a = e.doc().atom(1).unwrap();
        (a.x, a.y)
    };
    let add = |v: Value| mzdraw::Op::parse(&v).unwrap();
    e.session
        .run(&[
            add(json!({"op": "obj.add", "t": "text", "x": x, "y": y - 1.5, "text": "a note"})),
            add(json!({"op": "obj.add", "t": "arrow", "x": x + 0.5, "y": y + 1.5, "x2": x + 2.5, "y2": y + 1.5})),
            add(json!({"op": "obj.add", "t": "plus", "x": x - 1.5, "y": y})),
            add(json!({"op": "obj.add", "t": "text", "x": x - 60.0, "y": y + 60.0, "text": "far away"})),
        ])
        .unwrap();
    let objects = e.doc().objects.clone();
    let near: Vec<u32> = objects[..3].iter().map(|o| o.id).collect();
    let start = e.doc().to_json();
    // the page: the live line says how many stayed
    assert!(e.grid() != 0);
    assert_eq!(e.said, ("draw2.say.gridLeft", "3".to_string()));
    assert_eq!(e.grid_left(), near.as_slice());
    // they did not move, and no cell covers one of them
    assert_eq!(e.doc().objects, objects);
    let st = e.style.clone();
    for o in &objects {
        let b = match o.t {
            mzdraw::ObjKind::Text => mzdraw::scheme::text_box(e.doc(), o.id, &st).unwrap(),
            mzdraw::ObjKind::Plus => mzdraw::scheme::plus_box(e.doc(), o.id).unwrap(),
            _ => [o.x.min(o.x2), o.y.min(o.y2), o.x.max(o.x2), o.y.max(o.y2)],
        };
        for x in cells(e.doc(), &st) {
            assert!(!overlap(&x.3, &b), "{o:?} under {x:?}");
        }
    }
    // the CLI: the result names them (`grid.left`), the same moves as the page
    let request = json!({"import": start, "set": ["display.titles=on"], "command": "grid", "args": {}, "report": true});
    let r: Value =
        serde_json::from_str(&mzdraw::replay::replay(&request.to_string()).unwrap()).unwrap();
    assert_eq!(r["grid"]["left"], json!(near), "{r}");
    let out = json!({"import": start, "set": ["display.titles=on"], "command": "grid", "args": {}, "out": "mzdraw"});
    assert_eq!(
        mzdraw::replay::replay(&out.to_string()).unwrap(),
        e.save("mzdraw").unwrap()
    );
    // no free object by the structures: an empty list, and the plain live line
    let mut e = editor(NAMED, &[]);
    e.grid();
    assert_eq!(e.said.0, "draw2.say.grid");
    let request = json!({"import": NAMED, "command": "grid", "args": {}, "dry_run": true});
    let r: Value =
        serde_json::from_str(&mzdraw::replay::replay(&request.to_string()).unwrap()).unwrap();
    assert_eq!(r["grid"], json!({"left": []}));
}
