//! `scheme:orient` on large and hostile inputs: a bound on the size, a time bound, and what happens at the edge. Synthetic only.

use mzdraw::orient::{plan, Status, MAX_HEAVY, MAX_PRODUCTS};
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
    // 60 rows of 24 columns lie past the head (1440 products), 25 columns before the tail: single atoms have no core of three atoms,
    // so no pair is oriented; the first 200 products are `no_core` and the 1240 over the cap are `too_large`, each with a step
    assert!(p.ops.is_empty());
    assert_eq!(p.steps.len(), 1440);
    assert_eq!(
        p.steps
            .iter()
            .filter(|s| s.status == Status::NoCore)
            .count(),
        MAX_PRODUCTS
    );
    assert_eq!(
        p.steps
            .iter()
            .filter(|s| s.status == Status::TooLarge)
            .count(),
        1440 - MAX_PRODUCTS
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

/// A methylpyridine (7 atoms, no symmetry): atom 1..=7, drawn turned by `deg` about its centre at `(ox, oy)`, ids from `base`.
fn small(
    base: u32,
    ox: f64,
    oy: f64,
    deg: f64,
    atoms: &mut Vec<serde_json::Value>,
    bonds: &mut Vec<serde_json::Value>,
) {
    let mut pts: Vec<(f64, f64)> = (0..6)
        .map(|i| {
            (
                (i as f64 * std::f64::consts::PI / 3.0).cos(),
                (i as f64 * std::f64::consts::PI / 3.0).sin(),
            )
        })
        .collect();
    pts.push((2.0, 0.0));
    let (cx, cy) = (
        pts.iter().map(|p| p.0).sum::<f64>() / 7.0,
        pts.iter().map(|p| p.1).sum::<f64>() / 7.0,
    );
    let t = deg.to_radians();
    for (i, p) in pts.iter().enumerate() {
        let (dx, dy) = (p.0 - cx, p.1 - cy);
        atoms.push(json!({"id": base + i as u32, "el": if i == 3 { "N" } else { "C" }, "x": ((ox + dx * t.cos() - dy * t.sin()) * 1e4).round() / 1e4, "y": ((oy + dx * t.sin() + dy * t.cos()) * 1e4).round() / 1e4}));
    }
    for i in 0..6u32 {
        bonds.push(json!({"id": 500_000 + base + i, "a": base + i, "b": base + (i + 1) % 6, "o": 1 + i % 2}));
    }
    bonds.push(json!({"id": 500_006 + base, "a": base, "b": base + 6, "o": 1}));
}

fn drawing(
    atoms: Vec<serde_json::Value>,
    bonds: Vec<serde_json::Value>,
    arrows: Vec<serde_json::Value>,
) -> Document {
    Document::from_json(
        &json!({"format":"mzdraw/1","next":900_000,"atoms":atoms,"bonds":bonds,"objects":arrows})
            .to_string(),
    )
    .unwrap()
}

fn angle(d: &Document, base: u32) -> f64 {
    let (p, q) = (d.atom(base).unwrap(), d.atom(base + 6).unwrap());
    (q.y - p.y).atan2(q.x - p.x).to_degrees()
}

fn same_angle(a: f64, b: f64) -> bool {
    ((a - b + 540.0) % 360.0 - 180.0).abs() < 0.1
}

/// 250 products of one reactant: exactly 200 are oriented and exactly 50 are reported `too_large` (the cap of products), none skipped.
#[test]
fn the_product_cap_reports_exactly_the_products_over_it() {
    let (mut atoms, mut bonds) = (Vec::new(), Vec::new());
    small(1, 0.0, 0.0, 0.0, &mut atoms, &mut bonds);
    for i in 0..250u32 {
        small(
            100 * (i + 1),
            12.0 + 7.0 * (i % 25) as f64,
            7.0 * (i / 25) as f64,
            20.0 + (i % 17) as f64 * 15.0,
            &mut atoms,
            &mut bonds,
        );
    }
    let d = drawing(
        atoms,
        bonds,
        vec![json!({"id": 899_999, "t": "arrow", "x": 3, "y": 0, "x2": 7, "y2": 0})],
    );
    let t = Instant::now();
    let p = plan(&d);
    assert!(t.elapsed().as_secs() < 30, "{:?}", t.elapsed());
    assert_eq!(p.steps.len(), 250);
    assert_eq!(
        p.steps
            .iter()
            .filter(|s| s.status == Status::Oriented)
            .count(),
        MAX_PRODUCTS
    );
    assert_eq!(
        p.steps
            .iter()
            .filter(|s| s.status == Status::TooLarge)
            .count(),
        250 - MAX_PRODUCTS
    );
}

/// Mid-size symmetric products (every row of every column matches) spend the step budget of the command: the products after it are
/// reported `too_large`, none is skipped in silence, and the time is bounded.
#[test]
fn the_step_budget_of_a_command_is_one_and_the_rest_is_said() {
    let g = grids(10, 0.0, 0.0);
    let one = |k: usize| -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
        // grid `k` of a row: the first is the reactant (unturned), the others are products turned by 70 degrees
        let (mut atoms, mut bonds) = (Vec::new(), Vec::new());
        let t = if k == 0 { 0.0f64 } else { 70.0f64 }.to_radians();
        for a in g.atoms.iter().take(100) {
            let (x, y) = (a.x, a.y);
            atoms.push(json!({"id": 1000 * k + a.id as usize, "el": "C", "x": ((14.0 * k as f64 + x * t.cos() - y * t.sin()) * 1e4).round() / 1e4, "y": ((x * t.sin() + y * t.cos()) * 1e4).round() / 1e4}));
        }
        for (n, b) in g
            .bonds
            .iter()
            .filter(|b| b.a <= 100 && b.b <= 100)
            .enumerate()
        {
            bonds.push(json!({"id": 300_000 + 1000 * k + n, "a": 1000 * k + b.a as usize, "b": 1000 * k + b.b as usize, "o": 1}));
        }
        (atoms, bonds)
    };
    let (mut atoms, mut bonds) = (Vec::new(), Vec::new());
    for k in 0..121 {
        let (a, b) = one(k);
        atoms.extend(a);
        bonds.extend(b);
    }
    let d = drawing(
        atoms,
        bonds,
        vec![json!({"id": 899_999, "t": "arrow", "x": 6.0, "y": 0, "x2": 10.0, "y2": 0})],
    );
    let t = Instant::now();
    let p = plan(&d);
    eprintln!(
        "budget case: {:?} in {:?}",
        (p.steps.iter().map(|s| s.status).fold([0; 5], |mut c, s| {
            c[s as usize] += 1;
            c
        })),
        t.elapsed()
    );
    assert!(t.elapsed().as_secs() < 60, "{:?}", t.elapsed());
    assert_eq!(p.steps.len(), 120, "every product has a step");
    assert!(
        p.steps.iter().all(|s| matches!(
            s.status,
            Status::Oriented | Status::Already | Status::TooLarge | Status::Overlap
        )),
        "{:?}",
        p.steps.iter().map(|s| s.status).collect::<Vec<_>>()
    );
    // the split is exact (the search of `mzlab-mol` is deterministic): a 10 x 10 grid costs 111,000 of the budget (1,000 + 10,000 +
    // 100,000 of caps before it is found), so 9 products are oriented and the other 111 are reported
    let count = |st: Status| p.steps.iter().filter(|s| s.status == st).count();
    assert_eq!((count(Status::Oriented), count(Status::TooLarge)), (9, 111));
}

/// Only the nearest 8 reactants are tried: with 8 reactants that have no core the real one (the 9th) is not reached.
#[test]
fn only_the_nearest_eight_reactants_are_tried() {
    for (decoys, expected) in [(8usize, Status::NoCore), (7, Status::Oriented)] {
        let (mut atoms, mut bonds) = (Vec::new(), Vec::new());
        // decoys: triangles of carbon, nearest to the tail first; then the real reactant; the product past the head
        for i in 0..decoys as u32 {
            let x = -5.0 - 4.0 * i as f64;
            for k in 0..3u32 {
                let t = std::f64::consts::TAU / 3.0 * k as f64;
                atoms.push(
                    json!({"id": 100 * (i + 1) + k, "el": "O", "x": x + t.cos(), "y": t.sin()}),
                );
                bonds.push(json!({"id": 600_000 + 100 * (i + 1) + k, "a": 100 * (i + 1) + k, "b": 100 * (i + 1) + (k + 1) % 3, "o": 1}));
            }
        }
        small(
            50_000,
            -5.0 - 4.0 * decoys as f64,
            0.0,
            0.0,
            &mut atoms,
            &mut bonds,
        );
        small(60_000, 40.0, 0.0, 70.0, &mut atoms, &mut bonds);
        let d = drawing(
            atoms,
            bonds,
            vec![json!({"id": 899_999, "t": "arrow", "x": 0, "y": 0, "x2": 30.0, "y2": 0})],
        );
        let p = plan(&d);
        assert_eq!(p.steps.len(), 1, "{decoys}");
        assert_eq!(p.steps[0].status, expected, "{decoys} decoys");
    }
}

/// A chain of 150 arrows drawn right to left, every structure turned differently: each is oriented on the one before it (to its right),
/// in one pass, with a bounded time.
#[test]
fn a_reversed_chain_of_many_arrows_is_oriented_in_order() {
    let n = 150u32;
    let (mut atoms, mut bonds, mut arrows) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..n {
        small(
            100 * (i + 1),
            -10.0 * i as f64,
            0.0,
            if i == 0 {
                0.0
            } else {
                17.0 + (i * 37 % 300) as f64
            },
            &mut atoms,
            &mut bonds,
        );
        if i + 1 < n {
            arrows.push(json!({"id": 800_000 + i, "t": "arrow", "x": -10.0 * i as f64 - 4.0, "y": 0, "x2": -10.0 * i as f64 - 6.5, "y2": 0}));
        }
    }
    let d = drawing(atoms, bonds, arrows);
    let t = Instant::now();
    let p = plan(&d);
    assert!(t.elapsed().as_secs() < 30, "{:?}", t.elapsed());
    assert_eq!(p.steps.len(), (n - 1) as usize);
    assert!(
        p.steps
            .iter()
            .all(|s| matches!(s.status, Status::Oriented | Status::Already)),
        "{:?}",
        p.steps.iter().map(|s| s.status).collect::<Vec<_>>()
    );
    let out = apply(&d, &p.ops);
    let first = angle(&out, 100);
    for i in 1..n {
        assert!(
            same_angle(angle(&out, 100 * (i + 1)), first),
            "structure {i}"
        );
    }
}

/// Arrows over `MAX_ARROWS` are not read, and each is reported `too_large` (not skipped in silence).
#[test]
fn the_arrows_over_the_cap_are_reported() {
    let n = 251u32;
    let (mut atoms, mut bonds, mut arrows) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..n {
        small(
            100 * (i + 1),
            10.0 * i as f64,
            0.0,
            if i == 0 { 0.0 } else { 30.0 },
            &mut atoms,
            &mut bonds,
        );
        if i + 1 < n {
            arrows.push(json!({"id": 800_000 + i, "t": "arrow", "x": 10.0 * i as f64 + 4.0, "y": 0, "x2": 10.0 * i as f64 + 6.5, "y2": 0}));
        }
    }
    let p = plan(&drawing(atoms, bonds, arrows));
    // the 50 arrows over the cap each have a step of their own, with no structure
    let over: Vec<_> = (p.steps.iter())
        .filter(|s| {
            s.status == Status::TooLarge
                && s.from == 0
                && s.to == 0
                && s.arrow >= 800_000 + mzdraw::orient::MAX_ARROWS as u32
        })
        .collect();
    assert_eq!(over.len(), 250 - mzdraw::orient::MAX_ARROWS);
    // the arrows read gave 200 oriented products; the 50 structures past the last arrow read are products of it, over the cap of
    // products, and said as well: one step per product and one per arrow not read
    assert_eq!(
        p.steps
            .iter()
            .filter(|s| s.status == Status::Oriented)
            .count(),
        mzdraw::orient::MAX_ARROWS
    );
    assert_eq!(p.steps.len(), 250 + 50);
}

/// A cycle of anchored arrows (every structure is the product of the next): planned in order, with a step for each, in linear time.
#[test]
fn a_long_cycle_of_anchored_arrows_is_planned_once_each() {
    let n = 150u32;
    let (mut atoms, mut bonds, mut arrows) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..n {
        small(
            100 * (i + 1),
            10.0 * i as f64,
            0.0,
            17.0 + (i * 37 % 300) as f64,
            &mut atoms,
            &mut bonds,
        );
        arrows.push(json!({"id": 800_000 + i, "t": "arrow", "x": 10.0 * i as f64 + 4.0, "y": 0, "x2": 10.0 * i as f64 + 6.5, "y2": 0,
            "from": 100 * (i + 1), "to": 100 * ((i + 1) % n + 1)}));
    }
    let t = Instant::now();
    let p = plan(&drawing(atoms, bonds, arrows));
    assert!(t.elapsed().as_secs() < 30, "{:?}", t.elapsed());
    assert_eq!(p.steps.len(), n as usize, "one step per product");
    // the structures of a cycle are each oriented on their predecessor in the order of the drawing; none is left out
    assert!(
        p.steps
            .iter()
            .all(|s| matches!(s.status, Status::Oriented | Status::Already)),
        "{:?}",
        p.steps.iter().map(|s| s.status).collect::<Vec<_>>()
    );
}

/// The live line says what was not done as well as what was.
#[test]
fn the_live_line_says_the_structures_that_were_too_large() {
    let big = grids(24, 0.0, 40.0);
    let (mut atoms, mut bonds) = (Vec::new(), Vec::new());
    small(1, 0.0, 0.0, 0.0, &mut atoms, &mut bonds);
    for (name, deg, key, what) in [
        ("one turned", 70.0, "draw2.say.orientedPart", "1/2"),
        ("none to turn", 0.0, "draw2.say.orientDonePart", "1"),
    ] {
        let (mut a, mut b) = (atoms.clone(), bonds.clone());
        small(100, 15.0, 0.0, deg, &mut a, &mut b);
        // the big grid, as a product too (its first copy left out)
        a.extend(
            big.atoms
                .iter()
                .filter(|x| x.id > 576)
                .map(|x| json!({"id": x.id, "el": "C", "x": x.x, "y": x.y})),
        );
        b.extend(
            big.bonds
                .iter()
                .filter(|x| x.a > 576)
                .map(|x| json!({"id": x.id, "a": x.a, "b": x.b, "o": 1})),
        );
        let d = drawing(
            a,
            b,
            vec![json!({"id": 899_999, "t": "arrow", "x": 3, "y": 0, "x2": 7, "y2": 0})],
        );
        let mut e = mzdraw::Editor::default();
        e.import(&d.to_json()).unwrap();
        e.scheme_orient();
        assert_eq!((e.said.0, e.said.1.as_str()), (key, what), "{name}");
    }
}
