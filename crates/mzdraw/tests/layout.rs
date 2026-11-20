//! Placeholder positions of a structure that arrives without coordinates (a SMILES): until a layout engine exists every piece sits on
//! a regular polygon, its atoms in the order of a walk along the bonds, so rings close and chains do not cross themselves.

use mzdraw::Editor;

fn drawn(smiles: &str) -> Editor {
    let mut e = Editor::default();
    assert!(e.import(smiles).unwrap().placeholder);
    e
}

fn length(e: &Editor, a: u32, b: u32) -> f64 {
    let (p, q) = (e.doc().atom(a).unwrap(), e.doc().atom(b).unwrap());
    (p.x - q.x).hypot(p.y - q.y)
}

/// Whether two bonds without a common atom cross.
fn cross(e: &Editor, i: usize, j: usize) -> bool {
    let d = e.doc();
    let (b1, b2) = (&d.bonds[i], &d.bonds[j]);
    if [b1.a, b1.b].iter().any(|x| *x == b2.a || *x == b2.b) {
        return false;
    }
    let p = |id| {
        let a = d.atom(id).unwrap();
        (a.x, a.y)
    };
    let side = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
        ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).signum()
    };
    let (a, b, c, e2) = (p(b1.a), p(b1.b), p(b2.a), p(b2.b));
    side(a, b, c) != side(a, b, e2) && side(c, e2, a) != side(c, e2, b)
}

fn crossings(e: &Editor) -> usize {
    let n = e.doc().bonds.len();
    (0..n)
        .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
        .filter(|&(i, j)| cross(e, i, j))
        .count()
}

#[test]
fn a_ring_is_a_regular_polygon() {
    for (smiles, n) in [("c1ccccc1", 6), ("C1CCCC1", 5), ("C1CCCCCCC1", 8)] {
        let e = drawn(smiles);
        assert_eq!(e.doc().bonds.len(), n);
        for b in &e.doc().bonds {
            assert!(
                (length(&e, b.a, b.b) - 1.0).abs() < 1e-3,
                "{smiles}: every ring bond has length 1"
            );
        }
        assert_eq!(crossings(&e), 0, "{smiles}");
    }
}

#[test]
fn chains_and_substituted_rings_do_not_cross_themselves() {
    for smiles in [
        "CCCCCC",
        "CCO",
        "Cc1ccccc1",
        "C[C@H]1CC[C@@H](O)CC1",
        "OC(=O)CCN",
    ] {
        let e = drawn(smiles);
        assert_eq!(crossings(&e), 0, "{smiles}");
    }
    // pieces sit side by side, left to right
    let e = drawn("CCO.c1ccccc1");
    let right_of_first = e.doc().atoms[..3]
        .iter()
        .map(|a| a.x)
        .fold(f64::MIN, f64::max);
    assert!(e.doc().atoms[3..].iter().all(|a| a.x > right_of_first));
}
