//! `scheme:orient` on large and hostile inputs: a bound on the size, a time bound, and what happens at the edge. Synthetic only.

use mzdraw::orient::{plan, Status, MAX_HEAVY};
use mzdraw::Document;
use serde_json::json;
use std::time::Instant;

/// Two copies of an `n` x `n` grid (bond length 1), the second turned by `deg` and `gap` to the right, with a free arrow between.
fn grids(n: usize, deg: f64, gap: f64) -> Document {
    let mut atoms = Vec::new();
    let mut bonds = Vec::new();
    let t = deg.to_radians();
    let c = (n as f64 - 1.0) / 2.0;
    for k in 0..2 {
        for i in 0..n {
            for j in 0..n {
                let (dx, dy) = (i as f64 - c, j as f64 - c);
                let (x, y) = if k == 0 {
                    (dx, dy)
                } else {
                    (
                        gap + dx * t.cos() - dy * t.sin(),
                        dx * t.sin() + dy * t.cos(),
                    )
                };
                let id = k * n * n + i * n + j + 1;
                atoms.push(json!({"id": id, "el": "C", "x": (x * 1e4).round() / 1e4, "y": (y * 1e4).round() / 1e4}));
                let o = (n * n) * k;
                if i > 0 {
                    bonds.push(
                        json!({"id": 100000 + id, "a": id, "b": o + (i - 1) * n + j + 1, "o": 1}),
                    );
                }
                if j > 0 {
                    bonds.push(json!({"id": 200000 + id, "a": id, "b": o + i * n + j, "o": 1}));
                }
            }
        }
    }
    let m = n as f64 / 2.0 + 2.0;
    Document::from_json(
        &json!({"format":"mzdraw/1","next":300000,"atoms":atoms,"bonds":bonds,
            "objects":[{"id":299999,"t":"arrow","x":m,"y":0,"x2":gap - m,"y2":0}]})
        .to_string(),
    )
    .unwrap()
}

/// The root-mean-square distance of the two grids' atoms (atom `i` of the first with atom `i` of the second), the shift taken away.
fn grid_rms(d: &Document, n: usize) -> f64 {
    let p = |id: usize| {
        let a = d.atom(id as u32).unwrap();
        (a.x, a.y)
    };
    let m = n * n;
    let (a, b): (Vec<_>, Vec<_>) = (0..m).map(|i| (p(i + 1), p(m + i + 1))).unzip();
    let c = |v: &[(f64, f64)]| {
        (
            v.iter().map(|q| q.0).sum::<f64>() / m as f64,
            v.iter().map(|q| q.1).sum::<f64>() / m as f64,
        )
    };
    let (ca, cb) = (c(&a), c(&b));
    let sum: f64 = a
        .iter()
        .zip(&b)
        .map(|(x, y)| ((x.0 - ca.0) - (y.0 - cb.0)).powi(2) + ((x.1 - ca.1) - (y.1 - cb.1)).powi(2))
        .sum();
    (sum / m as f64).sqrt()
}

fn apply(d: &Document, ops: &[mzdraw::Op]) -> Document {
    let mut s = mzdraw::Session::default();
    s.doc = d.clone();
    s.run(ops).unwrap();
    s.doc
}

/// Identical copies of a grid are the worst case of the search (every row of every column matches). The edge, said exactly:
/// 16 x 16 (256 atoms) is oriented; 20 x 20 (400 atoms) runs out of the search steps and is reported `too_large`.
#[test]
fn identical_grids_are_oriented_up_to_the_search_limit_and_then_said() {
    let d = grids(16, 70.0, 48.0);
    let p = plan(&d);
    assert_eq!(p.steps[0].status, Status::Oriented, "{:?}", p.steps);
    assert!(grid_rms(&d, 16) > 0.5);
    assert!(grid_rms(&apply(&d, &p.ops), 16) < 0.05);
    let d = grids(20, 70.0, 60.0);
    let p = plan(&d);
    assert_eq!(p.steps[0].status, Status::TooLarge);
    assert!(p.ops.is_empty());
    let r = json!({"import": d.to_json(), "commands": [{"command": "scheme:orient"}]});
    assert_eq!(
        mzdraw::replay::replay(&r.to_string()).unwrap_err().key,
        "error.draw2.orientSize"
    );
}

#[test]
fn a_structure_over_the_size_bound_is_refused_quickly() {
    // 24 x 24 = 576 heavy atoms, and 60 x 60 = 3600: never searched
    for n in [24usize, 60] {
        let d = grids(n, 70.0, 3.0 * n as f64);
        let t = Instant::now();
        let p = plan(&d);
        assert_eq!(p.steps[0].status, Status::TooLarge, "{n}");
        assert!(n * n > MAX_HEAVY && p.ops.is_empty());
        assert!(t.elapsed().as_secs() < 5, "{n}: {:?}", t.elapsed());
    }
}

#[test]
fn a_drawing_of_thousands_of_pieces_has_a_bounded_cost() {
    let atoms: Vec<_> = (0..6000).map(|i| json!({"id": i + 1, "el": "C", "x": (i % 100) as f64 * 2.0, "y": (i / 100) as f64 * 2.0})).collect();
    let d = Document::from_json(&json!({"format":"mzdraw/1","next":7000,"atoms":atoms,"bonds":[],"objects":[{"id":6999,"t":"arrow","x":50,"y":-5,"x2":150,"y2":-5}]}).to_string()).unwrap();
    let t = Instant::now();
    let p = plan(&d);
    assert!(t.elapsed().as_secs() < 10, "{:?}", t.elapsed());
    // single atoms have no core of three atoms: no pair is oriented, and the budget of products is said
    assert!(p.ops.is_empty());
    assert!(
        p.steps
            .iter()
            .filter(|s| s.status == Status::TooLarge)
            .count()
            > 0
            || p.steps.is_empty()
    );
}

#[test]
fn long_rings_and_chains_are_bounded() {
    // a ring and a chain of 3000 atoms next to a small ring: over the bound, said, fast
    let n = 3000;
    let mut atoms = Vec::new();
    let mut bonds = Vec::new();
    for i in 0..n {
        let t = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
        let r = n as f64 / (2.0 * std::f64::consts::PI);
        atoms.push(json!({"id": i + 1, "el": "C", "x": r * t.cos(), "y": r * t.sin()}));
        bonds.push(json!({"id": 10000 + i, "a": i + 1, "b": (i + 1) % n + 1, "o": 1}));
    }
    for k in 0..6 {
        let t = std::f64::consts::PI / 3.0 * k as f64;
        atoms.push(json!({"id": 5000 + k, "el": "C", "x": 800.0 + t.cos(), "y": t.sin()}));
        bonds.push(json!({"id": 20000 + k, "a": 5000 + k, "b": 5000 + (k + 1) % 6, "o": 1}));
    }
    let d = Document::from_json(&json!({"format":"mzdraw/1","next":30000,"atoms":atoms,"bonds":bonds,"objects":[{"id":29999,"t":"arrow","x":-300,"y":0,"x2":300,"y2":0}]}).to_string()).unwrap();
    let t = Instant::now();
    let p = plan(&d);
    assert!(t.elapsed().as_secs() < 5, "{:?}", t.elapsed());
    assert!(p.ops.is_empty());
}
