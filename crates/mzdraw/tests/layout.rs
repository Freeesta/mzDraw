//! The positions of a structure that arrives without coordinates (a SMILES): the 2D layout of `mzlab-mol`, in bond lengths of the
//! drawing. Rings are regular polygons fused along their shared bonds, chains a zigzag, every bond one bond length, pieces side by
//! side. Synthetic structures only.

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

/// The angle at `b` between its bonds to `a` and `c`, in degrees.
fn angle(e: &Editor, a: u32, b: u32, c: u32) -> f64 {
    let p = |id| {
        let a = e.doc().atom(id).unwrap();
        (a.x, a.y)
    };
    let (a, b, c) = (p(a), p(b), p(c));
    let (u, v) = ((a.0 - b.0, a.1 - b.1), (c.0 - b.0, c.1 - b.1));
    ((u.0 * v.0 + u.1 * v.1) / (u.0.hypot(u.1) * v.0.hypot(v.1)))
        .acos()
        .to_degrees()
}

fn every_bond_is_one_long(e: &Editor, what: &str) {
    for b in &e.doc().bonds {
        let l = length(e, b.a, b.b);
        assert!((l - 1.0).abs() < 2e-3, "{what}: a bond of length {l}");
    }
}

#[test]
fn a_ring_is_a_regular_polygon() {
    for (smiles, n) in [("c1ccccc1", 6), ("C1CCCC1", 5), ("C1CCCCCCC1", 8)] {
        let e = drawn(smiles);
        assert_eq!(e.doc().bonds.len(), n);
        every_bond_is_one_long(&e, smiles);
        assert_eq!(crossings(&e), 0, "{smiles}");
        // every corner of the ring has the angle of the regular polygon
        let ids: Vec<u32> = e.doc().atoms.iter().map(|a| a.id).collect();
        for b in &ids {
            let near = e.doc().neighbours(*b);
            let got = angle(&e, near[0], *b, near[1]);
            let want = 180.0 - 360.0 / n as f64;
            assert!((got - want).abs() < 0.5, "{smiles}: {got} for {want}");
        }
    }
}

#[test]
fn fused_rings_share_a_bond_and_keep_their_shape() {
    // naphthalene, indole, decalin: every bond one bond length, no bond crossing another, every ring corner near its polygon
    for (smiles, bonds) in [
        ("c1ccc2ccccc2c1", 11),
        ("c1ccc2[nH]ccc2c1", 10),
        ("C1CCC2CCCCC2C1", 11),
    ] {
        let e = drawn(smiles);
        assert_eq!(e.doc().bonds.len(), bonds);
        every_bond_is_one_long(&e, smiles);
        assert_eq!(crossings(&e), 0, "{smiles}");
        // no two atoms on top of each other
        let at: Vec<(f64, f64)> = e.doc().atoms.iter().map(|a| (a.x, a.y)).collect();
        for i in 0..at.len() {
            for j in i + 1..at.len() {
                let d = (at[i].0 - at[j].0).hypot(at[i].1 - at[j].1);
                assert!(d > 0.8, "{smiles}: atoms {i} and {j} are {d} apart");
            }
        }
    }
    // the six-ring corners of naphthalene are all 120 degrees
    let e = drawn("c1ccc2ccccc2c1");
    for a in &e.doc().atoms {
        let near = e.doc().neighbours(a.id);
        if near.len() == 2 {
            let got = angle(&e, near[0], a.id, near[1]);
            assert!((got - 120.0).abs() < 0.5, "{got}");
        }
    }
}

#[test]
fn chains_are_a_zigzag_and_nothing_crosses() {
    for smiles in [
        "CCCCCC",
        "CCO",
        "Cc1ccccc1",
        "C[C@H]1CC[C@@H](O)CC1",
        "OC(=O)CCN",
        "CC(C)(C)c1ccc(O)cc1",
    ] {
        let e = drawn(smiles);
        every_bond_is_one_long(&e, smiles);
        assert_eq!(crossings(&e), 0, "{smiles}");
    }
    // hexane: every inner corner is 120 degrees and the chain runs on, it does not curl
    let e = drawn("CCCCCC");
    let ids: Vec<u32> = e.doc().atoms.iter().map(|a| a.id).collect();
    for w in ids.windows(3) {
        let got = angle(&e, w[0], w[1], w[2]);
        assert!((got - 120.0).abs() < 0.5, "{got}");
    }
    assert!(length(&e, ids[0], ids[5]) > 4.0, "a stretched chain");
}

#[test]
fn pieces_sit_side_by_side_without_touching() {
    // a salt, a mixture: each piece keeps its own stretch of the x axis, with room between
    for smiles in ["CCO.c1ccccc1", "[Na+].[Cl-]", "CC(=O)[O-].[NH4+].O"] {
        let e = drawn(smiles);
        let pieces = mzdraw::layout::pieces(e.doc());
        let mut spans: Vec<(f64, f64)> = (pieces.iter())
            .map(|p| {
                let xs = p.iter().map(|id| e.doc().atom(*id).unwrap().x);
                (
                    xs.clone().fold(f64::MAX, f64::min),
                    xs.fold(f64::MIN, f64::max),
                )
            })
            .collect();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        for w in spans.windows(2) {
            assert!(w[1].0 - w[0].1 >= 1.0, "{smiles}: {spans:?}");
        }
    }
}

#[test]
fn the_same_text_gives_the_same_positions() {
    for smiles in [
        "CC(=O)Oc1ccccc1C(=O)O",
        "C1CCC2CCCCC2C1",
        "N[C@@H](C)C(=O)O",
    ] {
        assert_eq!(
            drawn(smiles).save("mzdraw").unwrap(),
            drawn(smiles).save("mzdraw").unwrap()
        );
    }
    // a structure with coordinates keeps them: nothing is laid out again
    let mut e = drawn("CC(C)O");
    let ids: Vec<u32> = e.doc().atoms.iter().map(|a| a.id).collect();
    let spread = |e: &Editor, ids: &[u32]| -> Vec<f64> {
        ids.iter()
            .map(|i| (length(e, ids[0], *i) * 1e3).round())
            .collect()
    };
    let (molfile, before) = (e.save("molfile").unwrap(), spread(&e, &ids));
    e.clear();
    assert!(!e.import(&molfile).unwrap().placeholder);
    let again: Vec<u32> = e.doc().atoms.iter().map(|a| a.id).collect();
    assert_eq!(spread(&e, &again), before);
}
