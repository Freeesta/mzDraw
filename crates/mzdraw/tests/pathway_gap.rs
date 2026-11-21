//! tp-05b: the gap rule `crossing` of the pathway layout (step 5): a gap between two columns is widened until no primary arrow meets
//! the route box of another structure of its two layers. Synthetic structures only.

use mzdraw::pathway::{self, Layout};
use mzdraw::{Document, Style};
use serde_json::json;

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

/// The caps are checked before the work (1000 arrows, a cycle); the rule's own work is bounded by `CROSSING_STEPS` per gap, so the
/// widest allowed fan (999 leaves) and a chain of 1000 stay fast.
#[test]
fn scaling_the_widest_fan_and_a_chain_stay_fast() {
    let s = style(&[]);
    let d = fan(pathway::MAX_NODES - 1);
    let t = std::time::Instant::now();
    let lay = pathway::layout(&d, &s).unwrap();
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    println!(
        "fan of {} leaves: layout {ms:.0} ms, gaps {:?}",
        pathway::MAX_NODES - 1,
        lay.gaps.iter().map(|g| (g.rule, g.gap)).collect::<Vec<_>>()
    );
    if !cfg!(debug_assertions) {
        assert!(ms < 2000.0, "{ms} ms");
    }
    // one more is refused before any work
    let e: Vec<(usize, usize)> = (1..=pathway::MAX_ARROWS + 1).map(|i| (0, i)).collect();
    assert!(pathway::layout(&synthetic(pathway::MAX_ARROWS + 2, &e), &s).is_err());
}
