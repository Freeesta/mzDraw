//! tp-05b: the gap rule `crossing` of the pathway layout (step 5): a gap between two columns is widened until no primary arrow meets
//! the route box of another structure of its two layers. Synthetic structures only.

use mzdraw::pathway::{self, Col, Gap, Layout, Long, LongGaps};
use mzdraw::{Document, Style};
use serde_json::json;

#[path = "support/timing.rs"]
mod timing;

/// A pathway of `n` equal structures (C-C-O) joined by the arrows `edges`.
fn synthetic(n: usize, edges: &[(usize, usize)]) -> Document {
    let (mut atoms, mut bonds, mut objs, mut first) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut id = 1;
    for i in 0..n {
        let (cx, cy) = (
            f64::from((i * 7 % 11) as u32) * 3.0,
            f64::from((i * 5 % 13) as u32) * 2.0,
        );
        first.push(id);
        for (k, (x, y, el)) in [(0.0, 0.0, "C"), (0.87, 0.5, "C"), (1.73, 0.0, "O")]
            .iter()
            .enumerate()
        {
            atoms.push(json!({"id": id + k, "el": el, "x": cx + x, "y": cy + y}));
        }
        bonds.push(json!({"id": id + 3, "a": id, "b": id + 1, "o": 1}));
        bonds.push(json!({"id": id + 4, "a": id + 1, "b": id + 2, "o": 1}));
        id += 5;
    }
    for &(u, v) in edges {
        objs.push(json!({"id": id, "t": "arrow", "x": 0, "y": 0, "x2": 1, "y2": 1, "from": first[u], "to": first[v]}));
        id += 1;
    }
    let d =
        json!({"format": "mzdraw/1", "next": id, "atoms": atoms, "bonds": bonds, "objects": objs});
    Document::from_json(&d.to_string()).unwrap()
}

fn style(sets: &[&str]) -> Style {
    let mut s = Style::default();
    for set in sets {
        let (k, v) = set.split_once('=').unwrap();
        s.set_pathway(k, v).unwrap();
    }
    s
}

/// A fan: node 0 with `k` leaves.
fn fan(k: usize) -> Document {
    let e: Vec<(usize, usize)> = (1..=k).map(|i| (0, i)).collect();
    synthetic(k + 1, &e)
}

/// A pseudo-random tree of `n` nodes (parent of i is below i), from a fixed seed.
fn tree(n: usize, seed: u64) -> Document {
    let mut x = seed;
    let e: Vec<(usize, usize)> = (1..n)
        .map(|i| {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (((x >> 33) as usize) % i, i)
        })
        .collect();
    synthetic(n, &e)
}

fn primary_crossings(lay: &Layout) -> usize {
    lay.crossings
        .iter()
        .filter(|c| lay.edges.iter().any(|e| e.arrow == c.0 && e.primary))
        .count()
}

/// The clearance boxes (the structure's box inflated by `row_gap / 2`) of a layout, on the target centres.
fn clearance(lay: &Layout) -> Vec<[f64; 4]> {
    let h = lay.opts.row_gap / 2.0;
    lay.nodes
        .iter()
        .zip(&lay.centres)
        .map(|(n, c)| {
            let (w, hh) = (n.rect[2] - n.rect[0], n.rect[3] - n.rect[1]);
            [
                c.0 - w / 2.0 - h,
                c.1 - hh / 2.0 - h,
                c.0 + w / 2.0 + h,
                c.1 + hh / 2.0 + h,
            ]
        })
        .collect()
}

fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] - 1e-9 && b[0] < a[2] - 1e-9 && a[1] < b[3] - 1e-9 && b[1] < a[3] - 1e-9
}

#[test]
fn a_fan_at_the_steepest_slope_no_longer_grazes_its_neighbours() {
    for k in 3..=12 {
        for sets in [
            vec![],
            vec!["max_angle=60"],
            vec!["max_angle=75"],
            vec!["dir=down"],
        ] {
            let lay = pathway::layout(&fan(k), &style(&sets)).unwrap();
            assert_eq!(primary_crossings(&lay), 0, "fan {k} {sets:?}");
        }
    }
    // the gap that did it is said, with its rule
    let lay = pathway::layout(&fan(5), &style(&[])).unwrap();
    assert!(
        lay.gaps.iter().any(|g| g.rule == "crossing" && g.k == 0),
        "{:?}",
        lay.gaps
    );
}

#[test]
fn a_gap_is_not_widened_when_nothing_grazes() {
    let lay = pathway::layout(&fan(2), &style(&[])).unwrap();
    assert!(lay.gaps.is_empty());
    let lay = pathway::layout(&fan(1), &style(&["col_gap=4"])).unwrap();
    assert!(lay.gaps.is_empty());
}

#[test]
fn property_deterministic_never_overlapping_and_primary_arrows_clean() {
    for seed in 1..=40u64 {
        let n = 3 + (seed as usize * 7) % 40;
        let d = tree(n, seed);
        for sets in [
            vec![],
            vec!["max_angle=60"],
            vec!["row_gap=0.6", "col_gap=1.5"],
            vec!["dir=down", "max_angle=30"],
        ] {
            let s = style(&sets);
            let (a, b) = (
                pathway::layout(&d, &s).unwrap(),
                pathway::layout(&d, &s).unwrap(),
            );
            assert_eq!(
                format!("{:?}", a.centres),
                format!("{:?}", b.centres),
                "seed {seed} {sets:?}"
            );
            assert_eq!(
                format!("{:?}", a.gaps),
                format!("{:?}", b.gaps),
                "seed {seed}"
            );
            let ops = |l: &Layout| format!("{:?}", pathway::ops(&d, &s, l).unwrap());
            assert_eq!(ops(&a), ops(&b), "seed {seed}");
            let boxes = clearance(&a);
            for i in 0..boxes.len() {
                for j in i + 1..boxes.len() {
                    assert!(
                        !overlaps(boxes[i], boxes[j]),
                        "seed {seed} {sets:?}: {i} and {j} overlap"
                    );
                }
            }
            assert_eq!(
                primary_crossings(&a),
                0,
                "seed {seed} {sets:?}: {:?}",
                a.crossings
            );
        }
    }
}

/// The caps are checked before any work: one arrow too many is refused whatever the layout would be.
#[test]
fn the_caps_are_checked_before_the_work() {
    let s = style(&[]);
    let e: Vec<(usize, usize)> = (1..=pathway::MAX_ARROWS + 1).map(|i| (0, i)).collect();
    assert!(pathway::layout(&synthetic(pathway::MAX_ARROWS + 2, &e), &s).is_err());
    let cycle = synthetic(3, &[(0, 1), (1, 2), (2, 0)]);
    assert!(pathway::layout(&cycle, &s).is_err());
}

/// A fan so steep that no gap within the bound clears it: the gap is left as it was, the keyed note `std.scheme.gapLimit` says so,
/// and the arrows that meet a box are still listed (as `std.scheme.crossing`).
#[test]
fn when_the_bound_is_hit_the_gap_is_unchanged_and_the_failure_is_reported() {
    let (d, s) = (fan(300), style(&["max_angle=89"]));
    let lay = pathway::layout(&d, &s).unwrap();
    assert!(
        lay.gaps.iter().all(|g| g.rule != "crossing"),
        "{:?}",
        lay.gaps
    );
    assert_eq!(lay.limits.len(), 1, "{:?}", lay.limits);
    let lim = &lay.limits[0];
    assert_eq!((lim.comp, lim.k, lim.rule), (0, 0, "crossing"));
    assert_eq!(lim.arrows.len(), 300);
    // the gap kept is the column spacing really laid out (not a widened one)
    let (x0, x1) = (lay.centres[0].0, lay.centres[1].0);
    let (w0, w1) = (
        lay.nodes[0].rect[2] - lay.nodes[0].rect[0],
        lay.nodes[1].rect[2] - lay.nodes[1].rect[0],
    );
    assert!(((x1 - x0) - w0 / 2.0 - w1 / 2.0 - lim.gap).abs() < 1e-9);
    // nothing is silent: the crossings are listed and the report carries both keys
    assert!(!lay.crossings.is_empty());
    let rep = pathway::report_with(&d, &s).to_json();
    assert!(
        rep.contains("std.scheme.gapLimit") && rep.contains("std.scheme.crossing"),
        "{rep}"
    );
    // a fan that is cleared has no such note
    let ok = pathway::layout(&fan(5), &style(&[])).unwrap();
    assert!(ok.limits.is_empty());
}

#[test]
fn the_new_key_has_its_english_text_and_a_schema_row() {
    let lang = std::fs::read_to_string(format!(
        "{}/../../web/lang/mzdraw.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(lang.contains("\"draw2.std.scheme.gapLimit\":"));
    assert!(mzdraw::schema::REQUEST
        .iter()
        .any(|r| r.key == "std.scheme.gapLimit"));
}

/// Columns whose last arrow meets a wider box at the same height whatever the gap: every one of the `CROSSING_STEPS + 1` checks
/// runs through all the clean arrows before it fails, so the work is the rule's worst case.
fn forced(n: usize) -> (Vec<Col>, Vec<Col>, Vec<(usize, usize)>) {
    let left: Vec<Col> = (0..n).map(|i| (2.0 * i as f64, 1.0, 1.0)).collect();
    let mut right = left.clone();
    right.push((2.0 * (n - 1) as f64, 6.0, 3.0)); // the wide box on top of the last one
    let edges = (0..n).map(|i| (i, i)).collect();
    (left, right, edges)
}

fn forced_time(n: usize) -> f64 {
    let (l, r, e) = forced(n);
    timing::best_of(1, 7, || {
        let g = pathway::crossing_gap(&l, &r, &e, (1.0, 6.0), 2.5, 0.35);
        assert_eq!(g, Gap::Bound);
    })
    .as_secs_f64()
}

/// The cost of the rule, alone, when it runs all its steps: four times the structures must cost about four times (a linear scan of
/// the column per arrow would cost sixteen), in debug as in release, best of seven on the thread's CPU time, no floor.
#[test]
fn scaling_the_rule_alone_at_its_bound_is_linear_in_the_arrows() {
    let (small, big) = (forced_time(250), forced_time(1000));
    let ratio = big / small;
    println!("rule at its bound: 250 -> {small:.4} s, 1000 -> {big:.4} s, ratio {ratio:.2}");
    assert!(ratio < 7.0, "ratio {ratio}");
    // and it stops at the first step that is clean when one exists
    let (l, r, e) = forced(3);
    let r: Vec<Col> = r[..3].to_vec();
    assert_eq!(
        pathway::crossing_gap(&l, &r, &e, (1.0, 1.0), 2.5, 0.35),
        Gap::Clean(0)
    );
}

/// The rule adds at most `CROSSING_STEPS * CROSSING_STEP` (20 bond lengths) to a gap, so it cannot push a layout off the page by
/// itself: a fan of 999 at 1 degree already has a slope gap near 50,000 and is laid out (the page limit is `MAX_COORD`, 1e6); a
/// layout that does leave the page is refused by `pathway::ops` with the document's own key, as before (`tests/pathway.rs`,
/// `a_layout_that_leaves_the_page_is_refused_with_a_key_and_changes_nothing`). The rule does not clamp a gap to the page.
#[test]
fn the_rule_adds_a_bounded_amount_and_the_page_limit_is_the_documents_own() {
    assert!((f64::from(pathway::CROSSING_STEPS) * pathway::CROSSING_STEP - 20.0).abs() < 1e-9);
    let (d, s) = (fan(pathway::MAX_NODES - 1), style(&["max_angle=1"]));
    let lay = pathway::layout(&d, &s).unwrap();
    assert!(
        lay.gaps.iter().any(|g| g.rule == "slope" && g.gap > 4.0e4),
        "{:?}",
        lay.gaps
    );
    assert!(pathway::ops(&d, &s, &lay).is_ok());
}

/// A pseudo-random DAG of `n` nodes: a spanning tree (the parent of `i` is below `i`) and a few arrows more, so that some skip a
/// layer or leave a node that has another parent, from a fixed seed.
fn dag(n: usize, seed: u64) -> Document {
    let mut x = seed;
    let mut next = |m: usize| {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((x >> 33) as usize) % m
    };
    let mut e: Vec<(usize, usize)> = (1..n).map(|i| (next(i), i)).collect();
    for _ in 0..1 + n / 4 {
        let v = 2 + next(n - 2);
        let u = next(v);
        if !e.contains(&(u, v)) {
            e.push((u, v));
        }
    }
    synthetic(n, &e)
}

fn is_long(lay: &Layout, arrow: u32) -> bool {
    let e = lay.edges.iter().find(|e| e.arrow == arrow).unwrap();
    !e.primary
}

#[test]
fn a_secondary_arrow_and_a_skipping_arrow_widen_the_gaps_they_span() {
    // 0 -> 1 -> 3 and 0 -> 2 -> 4, with the secondary arrow 3 -> 4 and, in the second, the skipping arrow 0 -> 4
    for (edges, name) in [
        (vec![(0, 1), (0, 2), (1, 3), (2, 4), (3, 4)], "secondary"),
        (
            vec![(0, 1), (1, 2), (1, 3), (0, 4), (4, 5), (0, 5), (0, 2)],
            "skipping",
        ),
    ] {
        let n = edges.iter().map(|e| e.1).max().unwrap() + 1;
        let lay = pathway::layout(&synthetic(n, &edges), &style(&[])).unwrap();
        assert!(lay.crossings.is_empty(), "{name}: {:?}", lay.crossings);
        assert!(lay.limits.is_empty(), "{name}: {:?}", lay.limits);
        let g = lay.gaps.iter().find(|g| g.rule == "crossing").unwrap();
        // the gap says every arrow that spans it, the long one included
        assert!(g.arrows.iter().any(|&a| is_long(&lay, a)), "{name}: {g:?}");
    }
}

#[test]
fn what_gaps_cannot_clear_is_the_bound_with_the_gaps_as_they_were() {
    // a skipping arrow in a straight chain passes the box of the middle structure whatever the gaps are
    let lay = pathway::layout(&synthetic(3, &[(0, 1), (1, 2), (0, 2)]), &style(&[])).unwrap();
    assert!(lay.gaps.is_empty(), "{:?}", lay.gaps);
    assert_eq!(lay.crossings.len(), 1);
    assert_eq!(lay.limits.len(), 2, "{:?}", lay.limits);
    let long = lay.edges.iter().find(|e| !e.primary).unwrap().arrow;
    assert!(lay.limits.iter().all(|l| l.arrows == vec![long]));
    assert_eq!(lay.limits.iter().map(|l| l.k).collect::<Vec<_>>(), [0, 1]);
}

#[test]
fn a_long_arrow_that_still_meets_a_box_is_never_unsaid() {
    // every crossing a long arrow has is in a gapLimit note, on every gap it spans; the widening never makes a layout worse
    let (mut widened, mut limited) = (0, 0);
    for seed in 1..=80u64 {
        let d = dag(5 + (seed as usize % 9), seed);
        let lay = pathway::layout(&d, &style(&[])).unwrap();
        assert_eq!(primary_crossings(&lay), 0, "seed {seed}");
        for (arrow, _) in &lay.crossings {
            assert!(
                lay.limits.iter().any(|l| l.arrows.contains(arrow)),
                "seed {seed}: arrow {arrow} crosses and no limit says so"
            );
        }
        widened += usize::from(lay.gaps.iter().any(|g| g.rule == "crossing"));
        limited += usize::from(!lay.limits.is_empty());
    }
    assert!(widened > 10 && limited > 10, "{widened} {limited}");
}

#[test]
fn property_the_layout_of_long_arrows_is_deterministic() {
    for seed in 1..=30u64 {
        let d = dag(5 + (seed as usize % 9), seed);
        for sets in [
            vec![],
            vec!["max_angle=60"],
            vec!["dir=down", "row_gap=0.6"],
        ] {
            let s = style(&sets);
            let (a, b) = (
                pathway::layout(&d, &s).unwrap(),
                pathway::layout(&d, &s).unwrap(),
            );
            assert_eq!(a.centres, b.centres, "seed {seed} {sets:?}");
            assert_eq!(a.gaps, b.gaps);
            assert_eq!(a.limits, b.limits);
            assert_eq!(a.crossings, b.crossings);
            assert_eq!(
                pathway::ops(&d, &s, &a).unwrap(),
                pathway::ops(&d, &s, &b).unwrap()
            );
        }
    }
}

/// Three columns of one box width 1: a long arrow from the box of the first to the box of the last that meets a middle box in the
/// middle of its way, and cleared by a wider gap.
fn three_columns(flip: bool) -> (Vec<Vec<Col>>, Vec<f64>, Vec<f64>) {
    let s = if flip { -1.0 } else { 1.0 };
    (
        vec![
            vec![(0.0, 1.0, 0.74)],
            vec![(s * 1.0, 1.0, 0.74), (s * -3.0, 1.0, 0.74)],
            vec![(s * 3.0, 1.0, 0.74)],
        ],
        vec![1.0; 3],
        vec![2.5, 2.5],
    )
}

#[test]
fn the_long_rule_alone_widens_the_gap_that_moves_the_arrow_off_the_box() {
    let (cols, w, gaps) = three_columns(false);
    let long: Vec<Long> = vec![((0, 0), (2, 0))];
    let r = pathway::long_gaps(&cols, &w, &gaps, &long, 0.35);
    assert!(r.left.is_empty(), "{r:?}");
    // the arrow rises, and meets the box below it: the gap before the box grows, the gap after it does not
    assert!(r.add[0] > 0 && r.add[1] == 0, "{r:?}");
    // clean to begin with: nothing is added
    let none = pathway::long_gaps(&cols, &w, &[2.5 + 1.0, 2.5 + 1.0], &[], 0.35);
    assert_eq!(
        none,
        LongGaps {
            add: vec![0, 0],
            left: vec![]
        }
    );
}

#[test]
fn the_long_rule_alone_is_the_same_for_a_mirror_image_and_for_any_order_of_the_arrows() {
    let (cols, w, gaps) = three_columns(false);
    let (mirror, _, _) = three_columns(true);
    // a second long arrow of the same two layers, to be put before and after the first
    let mut cols2 = cols.clone();
    cols2[0].push((6.0, 1.0, 0.74));
    cols2[2].push((9.0, 1.0, 0.74));
    let mut m2 = mirror.clone();
    m2[0].push((-6.0, 1.0, 0.74));
    m2[2].push((-9.0, 1.0, 0.74));
    let (a, b): (Vec<Long>, Vec<Long>) = (
        vec![((0, 0), (2, 0)), ((0, 1), (2, 1))],
        vec![((0, 1), (2, 1)), ((0, 0), (2, 0))],
    );
    let r1 = pathway::long_gaps(&cols2, &w, &gaps, &a, 0.35);
    let r2 = pathway::long_gaps(&cols2, &w, &gaps, &b, 0.35);
    let r3 = pathway::long_gaps(&m2, &w, &gaps, &a, 0.35);
    assert_eq!(r1.add, r2.add, "order of the arrows");
    assert_eq!(r1.add, r3.add, "mirror image in y");
    assert_eq!(r1.left.len(), r3.left.len());
    let _ = cols;
}

/// Columns whose long arrows are mostly clean and one that no gap clears (a box on its way at its own height), so that the rule
/// runs all its rounds: the work is `CROSSING_STEPS + 1` rounds of the arrows.
fn long_forced(n: usize) -> (Vec<Vec<Col>>, Vec<f64>, Vec<f64>, Vec<Long>) {
    let col = |dy: f64| -> Vec<Col> { (0..n).map(|i| (4.0 * i as f64 + dy, 1.0, 1.0)).collect() };
    let mut mid = col(0.0);
    mid.push((0.0, 1.0, 1.0)); // on the way of the arrow of the first row, level with it
    let cols = vec![col(0.0), mid, col(0.0)];
    // clean: row i to row i of the middle column (one layer), the stuck one: row 0 to row 0 of the last column across the middle
    let mut long: Vec<Long> = (1..n).map(|i| ((0, i), (1, i))).collect();
    long.push(((0, 0), (2, 0)));
    (cols, vec![1.0; 3], vec![2.5, 2.5], long)
}

fn long_time(n: usize) -> f64 {
    let (c, w, g, l) = long_forced(n);
    timing::best_of(1, 7, || {
        let r = pathway::long_gaps(&c, &w, &g, &l, 0.35);
        assert_eq!(r.left.len(), 1);
    })
    .as_secs_f64()
}

#[test]
fn scaling_the_long_rule_alone_is_linear_in_the_arrows() {
    let (small, big) = (long_time(250), long_time(1000));
    let ratio = big / small;
    println!("long rule at its bound: 250 -> {small:.4} s, 1000 -> {big:.4} s, ratio {ratio:.2}");
    assert!(ratio < 7.0, "ratio {ratio}");
}

fn mirrored(cols: &[Vec<Col>]) -> Vec<Vec<Col>> {
    cols.iter()
        .map(|c| c.iter().map(|b| (-b.0, b.1, b.2)).collect())
        .collect()
}

/// The rule on a set of columns and on their mirror image in y: the same steps for every gap and the same number of arrows left.
fn same_in_the_mirror(cols: &[Vec<Col>], long: &[Long]) -> LongGaps {
    let (w, g) = (vec![1.0; cols.len()], vec![2.5; cols.len() - 1]);
    let a = pathway::long_gaps(cols, &w, &g, long, 0.35);
    let b = pathway::long_gaps(&mirrored(cols), &w, &g, long, 0.35);
    assert_eq!(a, b, "{cols:?}");
    a
}

#[test]
fn the_long_rule_is_the_same_in_the_mirror_for_every_branch() {
    let bx = |y: f64| (y, 1.0, 0.74);
    let long: Vec<Long> = vec![((0, 0), (2, 0))];
    // (a) a horizontal arrow, a box off its centre above, below, and both: every spanned gap is the answer, whatever the side
    for mid in [vec![bx(0.3)], vec![bx(-0.3)], vec![bx(0.3), bx(-0.3)]] {
        let r = same_in_the_mirror(&[vec![bx(0.0)], mid, vec![bx(0.0)]], &long);
        assert_eq!(
            r.left,
            vec![0],
            "a horizontal arrow cannot leave a box at its own height"
        );
    }
    // (b) the arrow rises and the box is above it: the gap after the box grows (the arrow stays low), and it is cleared
    let r = same_in_the_mirror(
        &[vec![bx(0.0)], vec![bx(2.0), bx(-3.0)], vec![bx(3.0)]],
        &long,
    );
    assert!(r.left.is_empty() && r.add[0] == 0 && r.add[1] > 0, "{r:?}");
    // (c) the arrow rises and the box is below it: the gap before the box grows
    let r = same_in_the_mirror(
        &[vec![bx(0.0)], vec![bx(1.0), bx(-3.0)], vec![bx(3.0)]],
        &long,
    );
    assert!(r.left.is_empty() && r.add[0] > 0 && r.add[1] == 0, "{r:?}");
    // (d) two boxes equally near, one above and one below the arrow: no index decides, both gaps grow
    let r = same_in_the_mirror(
        &[vec![bx(0.0)], vec![bx(0.5), bx(-0.5)], vec![bx(0.0)]],
        &long,
    );
    assert_eq!(r.left, vec![0]);
}

#[test]
fn the_layout_of_long_arrows_is_the_same_in_the_mirror() {
    // `reverse-id` lays the children of every structure out bottom to top: the layout is the mirror image in y of the one by id
    let rows = |lay: &Layout| -> Vec<(usize, u64, &'static str, usize)> {
        lay.gaps
            .iter()
            .map(|g| (g.k, (g.gap * 1e6).round() as u64, g.rule, g.arrows.len()))
            .collect()
    };
    let (mut widened, mut limited, mut compared) = (0, 0, 0);
    for seed in 1..=200u64 {
        let d = dag(5 + (seed as usize % 9), seed);
        let a = pathway::layout(&d, &style(&[])).unwrap();
        let b = pathway::layout(&d, &style(&["order=reverse-id"])).unwrap();
        // the primary parent of a structure with two parents of one layer is decided by the order, which the mirror reverses:
        // such a layout has another forest, not the mirror image, and is not compared
        let forest = |l: &Layout| {
            l.edges
                .iter()
                .map(|e| (e.arrow, e.primary))
                .collect::<Vec<_>>()
        };
        if forest(&a) != forest(&b) {
            continue;
        }
        compared += 1;
        assert_eq!(rows(&a), rows(&b), "seed {seed}: gaps");
        let lim = |l: &Layout| {
            l.limits
                .iter()
                .map(|g| (g.k, g.arrows.len()))
                .collect::<Vec<_>>()
        };
        assert_eq!(lim(&a), lim(&b), "seed {seed}: limits");
        assert_eq!(
            a.crossings.len(),
            b.crossings.len(),
            "seed {seed}: crossings"
        );
        widened += usize::from(a.gaps.iter().any(|g| g.rule == "crossing"));
        limited += usize::from(!a.limits.is_empty());
    }
    assert!(
        compared > 40 && widened > 5 && limited > 5,
        "{compared} {widened} {limited}"
    );
}
