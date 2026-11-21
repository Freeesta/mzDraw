//! `scheme:row` (plan-13): the one-line layout of a reaction scheme. Synthetic structures only.

use mzdraw::route::structure_box;
use mzdraw::scene::width;
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
    fn pick<'a, T>(&mut self, l: &'a [T]) -> &'a T {
        &l[(self.next() * l.len() as f64) as usize % l.len()]
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum K {
    Mol,
    Plus,
    Arrow,
    Text,
}

/// One thing of a generated scheme: its kind, the id that names it (the first atom of a structure, the id of an object), the atoms
/// and objects it holds, and, for an arrow, its notes.
#[derive(Clone, Debug)]
struct Elem {
    kind: K,
    id: u32,
    atoms: Vec<u32>,
    objs: Vec<u32>,
    notes: Vec<Elem>,
}

/// A synthetic scheme laid out by hand, messy (items on top of each other, notes off the arrow, an agent far from the line), with
/// the things of its row in reading order.
struct Gen {
    v: Value,
    row: Vec<Elem>,
    next: u32,
}

impl Gen {
    fn id(&mut self) -> u32 {
        self.next += 1;
        self.next
    }
    fn push(&mut self, key: &str, o: Value) {
        self.v[key].as_array_mut().unwrap().push(o);
    }
    /// A chain of two or three atoms near (`x`, `y`) as one structure; returns the element and the x past its end.
    fn mol(&mut self, r: &mut Rng, x: f64, y: f64) -> (Elem, f64) {
        let n = if r.next() < 0.5 { 2 } else { 3 };
        let mut atoms = Vec::new();
        for i in 0..n {
            let id = self.id();
            let el = *r.pick(&["C", "C", "N", "O", "Cl"]);
            let (ax, ay) = (round(x + 0.9 * i as f64), round(y + r.range(-0.3, 0.3)));
            self.push("atoms", json!({"id": id, "el": el, "x": ax, "y": ay}));
            atoms.push(id);
        }
        for w in atoms.windows(2) {
            let id = self.id();
            self.push("bonds", json!({"id": id, "a": w[0], "b": w[1], "o": 1}));
        }
        let end = x + 0.9 * (n - 1) as f64;
        (
            Elem {
                kind: K::Mol,
                id: atoms[0],
                atoms,
                objs: vec![],
                notes: vec![],
            },
            end,
        )
    }
}

fn round(v: f64) -> f64 {
    (v * 1e4).round() / 1e4
}

/// `stages` arrows with a group of one to three structures (plus signs between them) before and after each, in reading order left to
/// right, mirrored when `rtl`.
fn generate(r: &mut Rng, stages: usize, rtl: bool) -> Gen {
    let mut g = Gen {
        v: json!({"format":"mzdraw/1","next":0,"atoms":[],"bonds":[],"objects":[]}),
        row: vec![],
        next: 0,
    };
    let y0 = r.range(-1.0, 1.0);
    let mut x = 0.0;
    for st in 0..=stages {
        if st > 0 {
            // the arrow, with notes: texts above and below, and maybe an agent, near the line or far from it
            let (tail, len) = (x + 0.3, r.range(2.0, 4.0));
            let (head, id) = (tail + len, g.id());
            let ay = round(y0 + r.range(-0.2, 0.2));
            g.push("objects", json!({"id": id, "t": "arrow", "x": round(tail), "y": ay, "x2": round(head), "y2": ay}));
            let mut arrow = Elem {
                kind: K::Arrow,
                id,
                atoms: vec![],
                objs: vec![id],
                notes: vec![],
            };
            for _ in 0..(r.next() * 3.0) as usize {
                let (tid, side) = (g.id(), if r.next() < 0.5 { 1.0 } else { -1.0 });
                let text = *r.pick(&["Pd/C, H2", "THF", "25 C, 2 h", "base"]);
                let style = Style::default();
                let w = width(text, style.font_size / style.bond_length);
                g.push("objects", json!({"id": tid, "t": "text", "x": round(tail + (len - w.min(len)) * r.next()), "y": round(ay + side * r.range(0.4, 2.0)), "text": text}));
                arrow.objs.push(tid);
                arrow.notes.push(Elem {
                    kind: K::Text,
                    id: tid,
                    atoms: vec![],
                    objs: vec![tid],
                    notes: vec![],
                });
            }
            if r.next() < 0.6 {
                let (aid, side) = (g.id(), if r.next() < 0.5 { 1.0 } else { -1.0 });
                let el = *r.pick(&["Cl", "Br", "O"]);
                g.push("atoms", json!({"id": aid, "el": el, "x": round(tail + len * r.range(0.2, 0.8)), "y": round(ay + side * r.range(1.5, if stages == 1 { 7.0 } else { 2.8 }))}));
                arrow.atoms.push(aid);
                arrow.notes.push(Elem {
                    kind: K::Mol,
                    id: aid,
                    atoms: vec![aid],
                    objs: vec![],
                    notes: vec![],
                });
            }
            g.row.push(arrow);
            x = head + 0.4;
        }
        for k in 0..1 + (r.next() * 3.0) as usize {
            if k > 0 {
                let id = g.id();
                g.push("objects", json!({"id": id, "t": "plus", "x": round(x + 0.1), "y": round(y0 + r.range(-0.3, 0.3))}));
                g.row.push(Elem {
                    kind: K::Plus,
                    id,
                    atoms: vec![],
                    objs: vec![id],
                    notes: vec![],
                });
                x += 0.5;
            }
            let (m, end) = g.mol(r, x, y0);
            g.row.push(m);
            x = end + r.range(0.3, 0.9);
        }
    }
    g.v["next"] = json!(g.next + 1);
    if rtl {
        let style = Style::default();
        let f = style.font_size / style.bond_length;
        for a in g.v["atoms"].as_array_mut().unwrap() {
            a["x"] = json!(-a["x"].as_f64().unwrap());
        }
        for o in g.v["objects"].as_array_mut().unwrap() {
            match o["t"].as_str().unwrap() {
                "arrow" => {
                    o["x"] = json!(-o["x"].as_f64().unwrap());
                    o["x2"] = json!(-o["x2"].as_f64().unwrap());
                }
                "text" => {
                    o["x"] = json!(round(
                        -o["x"].as_f64().unwrap() - width(o["text"].as_str().unwrap(), f)
                    ))
                }
                _ => o["x"] = json!(-o["x"].as_f64().unwrap()),
            }
        }
    }
    g
}

fn rect_of(d: &Document, e: &Elem, style: &Style) -> Rect {
    match e.kind {
        K::Mol => structure_box(d, e.id, style).unwrap(),
        K::Plus => plus_box(d, e.id).unwrap(),
        K::Text => text_box(d, e.id, style).unwrap(),
        K::Arrow => {
            let o = d.object(e.id).unwrap();
            [o.x.min(o.x2), o.y.min(o.y2), o.x.max(o.x2), o.y.max(o.y2)]
        }
    }
}

/// The bound of every distance below. A coordinate is stored to four decimals; a move is the rounded difference of two such numbers
/// added to a stored number, so a thing lands within 5e-5 of its ideal place along each axis, and a gap or a centring (the
/// difference of two such places) is exact within 1e-4. The head of an arrow is set to its rounded ideal (5e-5) and the tail is moved
/// (5e-5): the same bound. The extra 1e-9 is the float error of the sums.
const BOUND: f64 = 1e-4 + 1e-9;

/// The invariants of a laid-out generated scheme (`rows`: the things that were laid out; the first `n` of the row when a selection).
fn check_row(after: &Document, row: &[Elem], gap: f64, wrap: f64, s: f64, style: &Style) {
    // the line of each unit: its centre height; the lines from the top
    let cy = |e: &Elem| {
        let r = rect_of(after, e, style);
        (r[1] + r[3]) / 2.0
    };
    let mut ys: Vec<f64> = Vec::new();
    for e in row {
        if !ys.iter().any(|y| (y - cy(e)).abs() < 1e-3) {
            ys.push(cy(e));
        }
    }
    ys.sort_by(|a, b| b.total_cmp(a));
    let line_of = |e: &Elem| ys.iter().position(|y| (y - cy(e)).abs() < 1e-3).unwrap();
    // every unit is centred on the baseline of its line
    for e in row {
        let k = line_of(e);
        assert!((cy(e) - ys[k]).abs() <= BOUND, "{:?} centred", e.kind);
    }
    // the order of reading is the order of the row, line after line
    let mut order: Vec<&Elem> = row.iter().collect();
    order.sort_by(|a, b| {
        let (ra, rb) = (rect_of(after, a, style), rect_of(after, b, style));
        (line_of(a), s * (ra[0] + ra[2]))
            .partial_cmp(&(line_of(b), s * (rb[0] + rb[2])))
            .unwrap()
    });
    assert_eq!(
        order.iter().map(|e| e.id).collect::<Vec<_>>(),
        row.iter().map(|e| e.id).collect::<Vec<_>>(),
        "order kept"
    );
    // the lines the row must have, counted here from the widths and the rule (a break after the first arrow that ends past `wrap`)
    let mut expected = 1;
    let mut run = 0.0;
    let mut fresh = true;
    for e in &order {
        let r = rect_of(after, e, style);
        run += if fresh { 0.0 } else { gap } + (r[2] - r[0]);
        fresh = false;
        if wrap > 0.0 && run > wrap && e.kind == K::Arrow {
            if e.id != order.last().unwrap().id {
                expected += 1;
            }
            run = 0.0;
            fresh = true;
        }
    }
    assert_eq!(
        ys.len(),
        expected,
        "the lines the rule asks for (wrap {wrap})"
    );
    if wrap == 0.0 {
        assert_eq!(ys.len(), 1, "one line");
    }
    let mut starts = Vec::new();
    for k in 0..ys.len() {
        let line: Vec<&&Elem> = order.iter().filter(|e| line_of(e) == k).collect();
        let first = rect_of(after, line[0], style);
        starts.push(if s > 0.0 { first[0] } else { first[2] });
        for w in line.windows(2) {
            let (a, b) = (rect_of(after, w[0], style), rect_of(after, w[1], style));
            let g = if s > 0.0 { b[0] - a[2] } else { a[0] - b[2] };
            assert!(
                (g - gap).abs() <= BOUND,
                "gap {g} for {gap}: {:?} {} then {:?} {} (line {k}, {} lines)",
                w[0].kind,
                w[0].id,
                w[1].kind,
                w[1].id,
                ys.len()
            );
        }
        if k + 1 < ys.len() {
            assert_eq!(
                line.last().unwrap().kind,
                K::Arrow,
                "a line breaks after an arrow"
            );
            let (a, b) = (
                rect_of(after, line[0], style),
                rect_of(after, line.last().unwrap(), style),
            );
            assert!(
                (b[2].max(a[2]) - a[0].min(b[0])).abs() > wrap - BOUND,
                "a line breaks past `wrap`"
            );
        }
    }
    // measured: each line is under the one before, the baselines at least the reach of a note apart, nothing of one line in the other
    for k in 1..ys.len() {
        assert!(
            ys[k - 1] - ys[k] >= mzdraw::scheme::NOTE_REACH + 0.1 - BOUND,
            "line {k} under line {}: {} {}",
            k - 1,
            ys[k - 1],
            ys[k]
        );
        let spans = |line: usize| {
            let mut lo_hi = (f64::MAX, f64::MIN);
            for e in row.iter().filter(|e| line_of(e) == line) {
                for r in std::iter::once(rect_of(after, e, style))
                    .chain(e.notes.iter().map(|n| rect_of(after, n, style)))
                {
                    lo_hi = (lo_hi.0.min(r[1]), lo_hi.1.max(r[3]));
                }
            }
            lo_hi
        };
        assert!(
            spans(k).1 < spans(k - 1).0,
            "the second line is clear of the first"
        );
    }
    for w in starts.windows(2) {
        assert!(
            (w[0] - w[1]).abs() <= BOUND,
            "every line starts at the same end"
        );
    }
    // the notes: inside the length of the arrow, clear of its line, centred on it
    for e in row.iter().filter(|e| e.kind == K::Arrow) {
        let a = rect_of(after, e, style);
        let o = after.object(e.id).unwrap();
        assert!(
            (o.y - o.y2).abs() < 1e-9 && (o.x2 - o.x) * s > 0.0,
            "level, along the direction"
        );
        for n in &e.notes {
            let r = rect_of(after, n, style);
            assert!(
                r[0] >= a[0] - BOUND && r[2] <= a[2] + BOUND,
                "note inside the arrow"
            );
            assert!(r[1] > a[1] || r[3] < a[1], "note off the line");
            assert!(
                (((r[0] + r[2]) - (a[0] + a[2])) / 2.0).abs() <= BOUND || n.kind == K::Text,
                "agent centred"
            );
        }
    }
    // nothing overlaps: the row, the notes, the arrows
    let mut all: Vec<Rect> = Vec::new();
    for e in row {
        all.push(rect_of(after, e, style));
        all.extend(e.notes.iter().map(|n| rect_of(after, n, style)));
    }
    for i in 0..all.len() {
        for j in i + 1..all.len() {
            let thick = |r: Rect| [r[0], r[1] - 1e-6, r[2], r[3] + 1e-6];
            assert!(!overlap(thick(all[i]), thick(all[j]), 1e-9), "overlap");
        }
    }
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
fn generated_schemes_are_laid_out_with_equal_gaps_on_one_line_in_all_modes() {
    let style = Style::default();
    for seed in 1..=60u64 {
        let mut r = Rng(seed * 7919);
        let gap = round(r.range(0.4, 2.0));
        for (stages, rtl, wrap) in [
            (1, false, 0.0),
            (1, true, 0.0),
            (2, false, 0.0),
            (3, false, round(r.range(6.0, 13.0))),
            (3, true, round(r.range(6.0, 13.0))),
        ] {
            let g = generate(&mut r, stages, rtl);
            let before = load(&g.v);
            let opts = Opts {
                gap,
                wrap,
                ..Opts::default()
            };
            let ops = row(&before, &style, &opts).unwrap();
            let mut sess = Session::new(before.clone());
            sess.run(&ops).unwrap();
            let after = sess.doc.clone();
            check_row(
                &after,
                &g.row,
                gap,
                wrap,
                if rtl { -1.0 } else { 1.0 },
                &style,
            );
            // the roles of a one-arrow scheme (a far agent included) are those of the reaction file
            if stages == 1 && wrap == 0.0 {
                assert_eq!(
                    roles(&before.to_rxn().unwrap()),
                    roles(&after.to_rxn().unwrap()),
                    "seed {seed}"
                );
            }
            // undo returns the identical document; a second row changes nothing
            assert!(sess.undo());
            assert_eq!(sess.doc, before);
            let again = row(&after, &style, &opts).unwrap();
            assert!(again.is_empty(), "idempotent, seed {seed}: {:?}", again);
        }
    }
}

#[test]
fn an_agent_far_from_the_arrow_line_stays_an_agent() {
    // the agent is 6 bond lengths above the line: between tail and head, so an agent by the rule of the reaction file
    let mut v = messy();
    v["atoms"][5]["y"] = json!(6.5);
    let before = load(&v);
    let rxn = before.to_rxn().unwrap();
    let after = run(&before, &Opts::default());
    assert_eq!(roles(&rxn), roles(&after.to_rxn().unwrap()));
    let (agent_before, agent_after) = (before.atom(6).unwrap().y, after.atom(6).unwrap().y);
    let arrow = after.object(11).unwrap();
    assert!(
        agent_before > 6.0 && (agent_after - arrow.y).abs() < 3.0,
        "brought next to the arrow"
    );
}

#[test]
fn a_selection_is_laid_out_alone_and_the_rest_does_not_move() {
    let style = Style::default();
    for seed in 1..=40u64 {
        let mut r = Rng(seed * 104729);
        let g = generate(&mut r, 2, false);
        let before = load(&g.v);
        // the first m things of the row, with the notes of the arrows among them
        let m = 2 + (r.next() * (g.row.len() - 2) as f64) as usize;
        let chosen = &g.row[..m];
        let atoms: Vec<u32> = chosen.iter().flat_map(|e| e.atoms.clone()).collect();
        let objects: Vec<u32> = chosen.iter().flat_map(|e| e.objs.clone()).collect();
        let sel = Opts {
            atoms: atoms.clone(),
            objects: objects.clone(),
            ..Opts::default()
        };
        let after = run(&before, &sel);
        // the things outside the selection stand where they stood
        for a in before.atoms.iter().filter(|a| !atoms.contains(&a.id)) {
            assert_eq!(after.atom(a.id), Some(a), "atom {} outside", a.id);
        }
        for o in before.objects.iter().filter(|o| !objects.contains(&o.id)) {
            assert_eq!(after.object(o.id), Some(o), "object {} outside", o.id);
        }
        check_row(&after, chosen, 1.0, 0.0, 1.0, &style);
        // the same as laying out a drawing that holds nothing else
        let mut only = g.v.clone();
        only["atoms"]
            .as_array_mut()
            .unwrap()
            .retain(|a| atoms.contains(&(a["id"].as_u64().unwrap() as u32)));
        only["bonds"]
            .as_array_mut()
            .unwrap()
            .retain(|b| atoms.contains(&(b["a"].as_u64().unwrap() as u32)));
        only["objects"]
            .as_array_mut()
            .unwrap()
            .retain(|o| objects.contains(&(o["id"].as_u64().unwrap() as u32)));
        let alone = run(&load(&only), &Opts::default());
        for a in &atoms {
            assert_eq!(after.atom(*a), alone.atom(*a), "seed {seed}");
        }
        for o in &objects {
            assert_eq!(after.object(*o), alone.object(*o), "seed {seed}");
        }
    }
    // a selection that is not in order and not yet laid out does move: the operations are not empty
    let mut r = Rng(5);
    let g = generate(&mut r, 1, false);
    let before = load(&g.v);
    let all: Vec<u32> = before.atoms.iter().map(|a| a.id).collect();
    let ops = row(
        &before,
        &style,
        &Opts {
            atoms: all,
            objects: vec![],
            ..Opts::default()
        },
    )
    .unwrap();
    assert!(!ops.is_empty());
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

/// The page's command and the replay request give the same bytes: for the plain scheme, an anchored arrow, a selection and a wrap.
#[test]
fn the_page_command_and_the_cli_command_give_the_same_bytes() {
    let mut anchored = messy();
    anchored["objects"][1]["from"] = json!(5);
    anchored["objects"][1]["to"] = json!(12);
    let mut r = Rng(11);
    let g = generate(&mut r, 3, false);
    let (sel_atoms, sel_objs): (Vec<u32>, Vec<u32>) = (
        g.row[..4].iter().flat_map(|e| e.atoms.clone()).collect(),
        g.row[..4].iter().flat_map(|e| e.objs.clone()).collect(),
    );
    // (the start, the page's selection, gap, wrap)
    type Case = (Value, Vec<u32>, Vec<u32>, f64, f64);
    let cases: Vec<Case> = vec![
        (messy(), vec![], vec![], 1.0, 0.0),
        (anchored, vec![], vec![], 1.0, 0.0),
        (g.v.clone(), sel_atoms, sel_objs, 1.0, 0.0),
        (g.v.clone(), vec![], vec![], 1.5, 9.0),
    ];
    for (start, atoms, objects, gap, wrap) in cases {
        let mut e = Editor::default();
        e.import(&start.to_string()).unwrap();
        let ids_before = e.doc().clone();
        e.over.sel_atoms = atoms.clone();
        e.over.sel_objs = objects.clone();
        assert!(e.scheme_row_with(gap, wrap) != 0);
        assert!(*e.doc() != ids_before, "something moved");
        let page = e.save("mzdraw").unwrap();
        // the import, then the command, in one replay: the page imported with its own ids, so the request imports the same text
        let mut e2 = Editor::default();
        e2.import(&start.to_string()).unwrap();
        let first = e2.doc().clone();
        let ids = |d: &Document| {
            (
                d.atoms.iter().map(|a| a.id).collect::<Vec<_>>(),
                d.objects.iter().map(|o| o.id).collect::<Vec<_>>(),
            )
        };
        assert_eq!(ids(&first), ids(&ids_before));
        let (a2, o2) = (atoms, objects);
        let request = json!({"import": start.to_string(), "command": "scheme:row", "args": {"gap": gap, "wrap": wrap, "atoms": a2, "objects": o2}});
        // the page numbers what it imports from its own counter (the same one the replay uses), so the ids agree
        assert_eq!(mzdraw::replay::replay(&request.to_string()).unwrap(), page);
        // and the log the page wrote replays to the page's bytes, with no command
        let log: Value = serde_json::from_str(&e.session.log_json()).unwrap();
        assert_eq!(
            mzdraw::replay::replay(&json!({"log": log}).to_string()).unwrap(),
            page
        );
        let names: Vec<&str> = log
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["op"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"move"));
        assert!(
            names.iter().all(|n| [
                "atom.add", "bond.add", "obj.add", "add", "bond", "move", "obj.set"
            ]
            .contains(n)),
            "{names:?}"
        );
    }
}

#[test]
fn a_dry_run_a_report_and_an_expectation_work_for_the_command() {
    let r = json!({"import": messy().to_string(), "command": "scheme:row", "args": {}, "dry_run": true, "expect": [{"pieces": 4}]});
    let reply: Value =
        serde_json::from_str(&mzdraw::replay::replay(&r.to_string()).unwrap()).unwrap();
    assert_eq!(reply["format"], "draw-result/1");
    assert_eq!(reply["ok"], true);
    assert!(reply.get("out").is_none(), "a dry run writes nothing");
    // a refused command is a verdict of stage `command`
    let bad = json!({"import": messy().to_string(), "command": "scheme:row", "args": {"atoms": [999]}, "report": true});
    let reply: Value =
        serde_json::from_str(&mzdraw::replay::replay_at(&bad.to_string(), 0).unwrap().text)
            .unwrap();
    assert_eq!(reply["ok"], false);
    assert_eq!(reply["failed"]["stage"], "command");
}

#[test]
fn an_arrow_without_length_makes_no_nan() {
    let style = Style::default();
    let mut d = load(&messy());
    for o in d.objects.iter_mut().filter(|o| o.t == ObjKind::Arrow) {
        (o.x2, o.y2) = (o.x, o.y);
    }
    let ops = row(&d, &style, &Opts::default()).unwrap();
    let json = serde_json::to_string(&ops.iter().map(|o| o.to_json()).collect::<Vec<_>>()).unwrap();
    assert!(!json.contains("NaN") && !json.contains("null"), "{json}");
    // the roles of the reaction file do not change either: with no length every structure stands at the tail, an agent, as before
    // (all of them tie at t, so the order inside the agents is not a fact: compared as sets)
    let sorted = |mut r: Vec<Vec<String>>| {
        r.sort();
        r
    };
    let before_roles = sorted(roles(&d.to_rxn().unwrap()));
    let mut s = Session::new(d);
    s.run(&ops).unwrap();
    assert_eq!(before_roles, sorted(roles(&s.doc.to_rxn().unwrap())));
    let a = s
        .doc
        .objects
        .iter()
        .find(|o| o.t == ObjKind::Arrow)
        .unwrap();
    assert!(a.x2 - a.x >= mzdraw::scheme::MIN_ARROW - 1e-3 && a.x.is_finite());
}

#[test]
fn an_anchored_arrow_is_routed_not_set() {
    let style = Style::default();
    let mut v = messy();
    v["objects"][1]["from"] = json!(5);
    v["objects"][1]["to"] = json!(12);
    let d = load(&v);
    let after = run(&d, &Opts::default());
    let arrow = after.object(11).unwrap();
    assert_eq!((arrow.from, arrow.to), (5, 12));
    assert_eq!(
        roles(&d.to_rxn().unwrap()),
        roles(&after.to_rxn().unwrap()),
        "the roles of the file are kept"
    );
    assert!(
        mzdraw::route::route(&after, 11, &style).is_empty(),
        "the anchored arrow stands on its routed ends"
    );
    // no operation sets an end of it: the ops that touch it are the routing's own, of `route::route` on the moved structures
    let ops = row(&d, &style, &Opts::default()).unwrap();
    let routed_at = ops.iter().position(|o| {
        matches!(o, mzdraw::Op::ObjSet { id: 11, .. })
            || matches!(o, mzdraw::Op::Move { objects, .. } if objects == &vec![11])
    });
    let routed_at = routed_at.expect("the arrow is routed");
    let mut moved = Session::new(d.clone());
    moved.run(&ops[..routed_at]).unwrap();
    assert_eq!(
        ops[routed_at..].to_vec(),
        mzdraw::route::route(&moved.doc, 11, &style),
        "the tail of the list is the route"
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
