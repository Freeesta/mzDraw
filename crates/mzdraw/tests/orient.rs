//! `scheme:orient` (plan-14): the structure after an arrow is turned onto the one before it. Synthetic structures only; the truth
//! of each pair (which atom of the product is which atom of the reactant) is written here, not read from the code under test.

use mzdraw::orient::{plan, solve, Status, FIT};
use mzdraw::{Document, Editor};
use serde_json::{json, Value};

/// A structure: atoms `(element, x, y)` in bond lengths, bonds `(a, b, order)`, and the atoms that are the shared core.
struct Mol {
    atoms: Vec<(&'static str, f64, f64)>,
    bonds: Vec<(usize, usize, u8)>,
}

/// A regular polygon of `n` atoms, circumradius `1 / (2 sin(pi/n))`, first vertex at angle `a0` (degrees), centre `(cx, cy)`.
fn ring(n: usize, cx: f64, cy: f64, a0: f64, els: &[&'static str]) -> Mol {
    let r = 0.5 / (std::f64::consts::PI / n as f64).sin();
    let atoms = (0..n)
        .map(|i| {
            let t = (a0 + 360.0 * i as f64 / n as f64).to_radians();
            (
                *els.get(i).unwrap_or(&"C"),
                cx + r * t.cos(),
                cy + r * t.sin(),
            )
        })
        .collect();
    let bonds = (0..n)
        .map(|i| (i, (i + 1) % n, 1 + (i % 2) as u8))
        .collect();
    Mol { atoms, bonds }
}

/// The hexagon of a ring `a0` degrees turned, with a substituent chain on atom `at`: `(element, bond angle in degrees)` each.
fn with_tail(mut m: Mol, at: usize, tail: &[(&'static str, f64)]) -> Mol {
    let mut from = at;
    for &(el, ang) in tail {
        let (x, y) = (m.atoms[from].1, m.atoms[from].2);
        let t = ang.to_radians();
        m.atoms.push((el, x + t.cos(), y + t.sin()));
        m.bonds.push((from, m.atoms.len() - 1, 1));
        from = m.atoms.len() - 1;
    }
    m
}

/// Two hexagons or a hexagon and a pentagon sharing a bond: ring B is reflected across the shared edge (atoms 0 and 1 of the first).
fn fused(a: &Mol, n: usize, els: &[&'static str]) -> Mol {
    // the edge between atom 0 and atom 1 of `a`; the new ring lies on the other side of it
    let (p, q) = (a.atoms[0], a.atoms[1]);
    let (mx, my) = ((p.1 + q.1) / 2.0, (p.2 + q.2) / 2.0);
    let r = 0.5 / (std::f64::consts::PI / n as f64).tan();
    // the centre of `a` and the direction away from it
    let (ax, ay) = (
        a.atoms.iter().map(|x| x.1).sum::<f64>() / a.atoms.len() as f64,
        a.atoms.iter().map(|x| x.2).sum::<f64>() / a.atoms.len() as f64,
    );
    let (dx, dy) = (mx - ax, my - ay);
    let l = dx.hypot(dy);
    let (cx, cy) = (mx + dx / l * r, my + dy / l * r);
    let start = (p.2 - cy).atan2(p.1 - cx).to_degrees();
    let sign = if (q.1 - cx) * (p.2 - cy) - (q.2 - cy) * (p.1 - cx) > 0.0 {
        1.0
    } else {
        -1.0
    };
    // atoms of the new ring, in order: p, then going away from q
    let mut b = a.atoms.clone();
    let mut bonds = a.bonds.clone();
    let mut prev = 0;
    let mut first = true;
    for i in 1..n {
        let t = (start + sign * 360.0 * i as f64 / n as f64).to_radians();
        let el = *els.get(i - 1).unwrap_or(&"C");
        b.push((
            el,
            cx + (p.1 - cx).hypot(p.2 - cy) * t.cos(),
            cy + (p.1 - cx).hypot(p.2 - cy) * t.sin(),
        ));
        let id = b.len() - 1;
        bonds.push((prev, id, 1));
        prev = id;
        first = false;
    }
    let _ = first;
    bonds.push((prev, 1, 1));
    Mol { atoms: b, bonds }
}

/// The text of a drawing with a source, a free arrow and a target (rotated `deg` degrees about its own centre and moved
/// to `(ox, oy)`). Source atoms are ids 1.., target atoms 101..; the arrow is object 900.
fn scheme(src: &Mol, tgt: &Mol, deg: f64, ox: f64) -> String {
    let mut atoms = Vec::new();
    let mut bonds = Vec::new();
    for (i, a) in src.atoms.iter().enumerate() {
        atoms.push(json!({"id": 1 + i, "el": a.0, "x": (a.1 * 1e4).round() / 1e4, "y": (a.2 * 1e4).round() / 1e4}));
    }
    for (i, b) in src.bonds.iter().enumerate() {
        bonds.push(json!({"id": 50 + i, "a": 1 + b.0, "b": 1 + b.1, "o": b.2}));
    }
    let n = tgt.atoms.len() as f64;
    let (cx, cy) = (
        tgt.atoms.iter().map(|a| a.1).sum::<f64>() / n,
        tgt.atoms.iter().map(|a| a.2).sum::<f64>() / n,
    );
    let t = deg.to_radians();
    for (i, a) in tgt.atoms.iter().enumerate() {
        let (dx, dy) = (a.1 - cx, a.2 - cy);
        let (x, y) = (
            ox + dx * t.cos() - dy * t.sin(),
            dx * t.sin() + dy * t.cos(),
        );
        atoms.push(json!({"id": 101 + i, "el": a.0, "x": (x * 1e4).round() / 1e4, "y": (y * 1e4).round() / 1e4}));
    }
    for (i, b) in tgt.bonds.iter().enumerate() {
        bonds.push(json!({"id": 150 + i, "a": 101 + b.0, "b": 101 + b.1, "o": b.2}));
    }
    json!({"format":"mzdraw/1","next":1000,"atoms":atoms,"bonds":bonds,
        "objects":[{"id":900,"t":"arrow","x":ox / 2.0 - 1.0,"y":0,"x2":ox / 2.0 + 1.0,"y2":0}]})
    .to_string()
}

fn doc(text: &str) -> Document {
    Document::from_json(text).unwrap()
}

/// The root-mean-square distance of the matched atoms (source id `1 + i`, target id `101 + j`), the shift taken away.
fn rms(d: &Document, core: &[(usize, usize)]) -> f64 {
    let p = |id: u32| {
        let a = d.atom(id).unwrap();
        (a.x, a.y)
    };
    let s: Vec<_> = core.iter().map(|c| p(1 + c.0 as u32)).collect();
    let t: Vec<_> = core.iter().map(|c| p(101 + c.1 as u32)).collect();
    let n = core.len() as f64;
    let (sx, sy) = (
        s.iter().map(|a| a.0).sum::<f64>() / n,
        s.iter().map(|a| a.1).sum::<f64>() / n,
    );
    let (tx, ty) = (
        t.iter().map(|a| a.0).sum::<f64>() / n,
        t.iter().map(|a| a.1).sum::<f64>() / n,
    );
    let sum: f64 = s
        .iter()
        .zip(&t)
        .map(|(a, b)| ((a.0 - sx) - (b.0 - tx)).powi(2) + ((a.1 - sy) - (b.1 - ty)).powi(2))
        .sum();
    (sum / n).sqrt()
}

/// What a request of the command gives: the bare output.
fn run(text: &str) -> Result<Value, &'static str> {
    let r = json!({"import": text, "commands": [{"command": "scheme:orient"}]});
    mzdraw::replay::replay(&r.to_string())
        .map(|s| serde_json::from_str(&s).unwrap())
        .map_err(|e| e.key)
}

/// Ten synthetic pairs: (name, source, product, the matched atoms `(source index, product index)`).
type Pair = (&'static str, Mol, Mol, Vec<(usize, usize)>);

fn pairs() -> Vec<Pair> {
    let hexa = |tail: &[(&'static str, f64)]| with_tail(ring(6, 0.0, 0.0, 0.0, &[]), 0, tail);
    let all6: Vec<(usize, usize)> = (0..6).map(|i| (i, i)).collect();
    let nitro = [("N", 0.0), ("O", 60.0)];
    let naph = fused(&ring(6, 0.0, 0.0, 0.0, &[]), 6, &[]);
    let naph_me = with_tail(
        fused(&ring(6, 0.0, 0.0, 0.0, &[]), 6, &[]),
        4,
        &[("C", 90.0)],
    );
    let steroid = |tail: &[(&'static str, f64)]| {
        let a = ring(6, 0.0, 0.0, 0.0, &[]);
        let b = fused(&a, 6, &[]);
        let c = fused(
            &Mol {
                atoms: b.atoms.clone(),
                bonds: b.bonds.clone(),
            },
            5,
            &[],
        );
        with_tail(c, 3, tail)
    };
    let all_n = |n: usize| -> Vec<(usize, usize)> { (0..n).map(|i| (i, i)).collect() };
    vec![
        (
            "toluene to nitrotoluene",
            hexa(&[("C", 180.0)]),
            with_tail(hexa(&[("C", 180.0)]), 3, &nitro),
            (0..7).map(|i| (i, i)).collect(),
        ),
        (
            "chlorobenzene to nitrobenzene",
            hexa(&[("Cl", 180.0)]),
            hexa(&nitro),
            all6.clone(),
        ),
        (
            "benzene to toluene",
            ring(6, 0.0, 0.0, 0.0, &[]),
            hexa(&[("C", 120.0)]),
            all6.clone(),
        ),
        ("naphthalene to methylnaphthalene", naph, naph_me, all_n(10)),
        (
            "steroid-like to hydroxylated",
            steroid(&[]),
            steroid(&[("O", 70.0)]),
            all_n(14),
        ),
        (
            "phenol to anisole",
            hexa(&[("O", 120.0)]),
            hexa(&[("O", 120.0), ("C", 30.0)]),
            (0..7).map(|i| (i, i)).collect(),
        ),
        (
            "pyridine to aminopyridine",
            ring(6, 0.0, 0.0, 0.0, &["N"]),
            with_tail(ring(6, 0.0, 0.0, 0.0, &["N"]), 1, &[("N", 120.0)]),
            all6.clone(),
        ),
        (
            "cyclopentane to hydroxy",
            ring(5, 0.0, 0.0, 90.0, &[]),
            with_tail(ring(5, 0.0, 0.0, 90.0, &[]), 0, &[("O", 90.0)]),
            all_n(5),
        ),
        (
            "fluorobenzene to biphenyl",
            hexa(&[("F", 180.0)]),
            with_tail(hexa(&[("C", 180.0)]), 1, &[]),
            all6.clone(),
        ),
        (
            "indane-like to bromo",
            fused(&ring(6, 0.0, 0.0, 0.0, &[]), 5, &[]),
            with_tail(
                fused(&ring(6, 0.0, 0.0, 0.0, &[]), 5, &[]),
                3,
                &[("Br", 90.0)],
            ),
            all_n(9),
        ),
    ]
}

#[test]
fn ten_pairs_rotated_by_seventy_degrees_end_within_the_fit() {
    for (name, src, tgt, core) in pairs() {
        let text = scheme(&src, &tgt, 70.0, 12.0);
        let before = rms(&doc(&text), &core);
        // symmetric cores (a ring of one element) have a rotation that fits too: the bar is the one of the plan
        let out = run(&text);
        let after_doc = match out {
            Ok(v) => doc(&v.to_string()),
            Err(e) => panic!("{name}: refused with {e}"),
        };
        let after = rms(&after_doc, &core);
        assert!(after < FIT, "{name}: rms after {after} (before {before})");
        // the source did not move
        assert_eq!(
            doc(&text)
                .atoms
                .iter()
                .filter(|a| a.id < 100)
                .map(|a| (a.x, a.y))
                .collect::<Vec<_>>(),
            after_doc
                .atoms
                .iter()
                .filter(|a| a.id < 100)
                .map(|a| (a.x, a.y))
                .collect::<Vec<_>>(),
            "{name}: the source moved"
        );
    }
}

#[test]
fn it_fails_without_the_command() {
    // the same inputs, no command: every pair that is not a symmetric ring is off by more than 0.5 bond lengths
    let mut off = 0;
    for (_, src, tgt, core) in pairs() {
        let text = scheme(&src, &tgt, 70.0, 12.0);
        if rms(&doc(&text), &core) > 0.5 {
            off += 1;
        }
    }
    assert!(off >= 8, "only {off} of the ten inputs are visibly off");
}

#[test]
fn undo_takes_the_command_back_in_one_step() {
    let (_, src, tgt, _) = pairs().remove(0);
    let text = scheme(&src, &tgt, 70.0, 12.0);
    let mut e = Editor::default();
    e.open(&text).unwrap();
    let before = e.save("mzdraw").unwrap();
    assert_ne!(e.scheme_orient() & mzdraw::editor::CHANGED, 0);
    let after = e.save("mzdraw").unwrap();
    assert_ne!(before, after);
    e.undo();
    assert_eq!(
        e.save("mzdraw").unwrap(),
        before,
        "one undo step restores the document"
    );
    e.redo();
    assert_eq!(e.save("mzdraw").unwrap(), after);
}

#[test]
fn the_page_and_the_request_give_the_same_bytes() {
    let (_, src, tgt, _) = pairs().remove(3);
    let text = scheme(&src, &tgt, 70.0, 12.0);
    let mut e = Editor::default();
    e.import(&text).unwrap();
    e.scheme_orient();
    let page = e.save("mzdraw").unwrap();
    let cli = mzdraw::replay::replay(
        &json!({"import": text, "command": "scheme:orient", "args": {}}).to_string(),
    )
    .unwrap();
    assert_eq!(page, cli);
}

/// A bent four-atom chain C-N-O-S (all different, so one match), drawn as a zigzag; the product is its mirror image.
fn chain(mirror: bool) -> Mol {
    let s = if mirror { -1.0 } else { 1.0 };
    Mol {
        atoms: vec![
            ("C", 0.0, 0.0),
            ("N", 1.0, 0.0),
            ("O", 1.5, s * 0.866),
            ("S", 2.5, s * 0.866),
        ],
        bonds: vec![(0, 1, 1), (1, 2, 1), (2, 3, 1)],
    }
}

#[test]
fn a_mirror_drawing_is_flipped_only_without_a_wedge() {
    let core: Vec<(usize, usize)> = (0..4).map(|i| (i, i)).collect();
    // without a wedge: a flip is chosen and the core fits
    let text = scheme(&chain(false), &chain(true), 70.0, 12.0);
    let d = doc(&text);
    let (step, ops) = solve(&d, 1, 101);
    assert_eq!(step.status, Status::Oriented);
    assert!(step.flip, "a mirror drawing needs a flip");
    assert!(ops.iter().any(|o| matches!(o, mzdraw::Op::Flip { .. })));
    let out = doc(&run(&text).unwrap().to_string());
    assert!(rms(&out, &core) < FIT);
    // with a wedge on the target: no flip is chosen, nothing is changed (a rotation cannot fit it)
    let mut v: Value = serde_json::from_str(&text).unwrap();
    v["bonds"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|b| b["id"] == 150)
        .unwrap()["s"] = json!(1);
    let wedged = v.to_string();
    let (step, ops) = solve(&doc(&wedged), 1, 101);
    assert!(!step.flip && ops.is_empty(), "no flip with a wedge");
    assert_eq!(run(&wedged), Err("error.draw2.orientNone"));
    // the same wedge on a target that a rotation fits: oriented, never mirrored
    let text = scheme(&chain(false), &chain(false), 70.0, 12.0);
    let mut v: Value = serde_json::from_str(&text).unwrap();
    v["bonds"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|b| b["id"] == 150)
        .unwrap()["s"] = json!(1);
    let (step, ops) = solve(&doc(&v.to_string()), 1, 101);
    assert_eq!(step.status, Status::Oriented);
    assert!(!ops.iter().any(|o| matches!(o, mzdraw::Op::Flip { .. })));
}

#[test]
fn no_common_ring_is_reported_and_changes_nothing() {
    // a five-ring and a six-ring share no core of three atoms
    let text = scheme(
        &ring(5, 0.0, 0.0, 90.0, &[]),
        &ring(6, 0.0, 0.0, 0.0, &[]),
        70.0,
        12.0,
    );
    let d = doc(&text);
    let (step, ops) = solve(&d, 1, 101);
    assert_eq!(step.status, Status::NoCore);
    assert!(ops.is_empty());
    assert_eq!(run(&text), Err("error.draw2.orientNone"));
    // a chain and a ring
    let chain = Mol {
        atoms: vec![
            ("C", 0.0, 0.0),
            ("C", 1.0, 0.0),
            ("C", 2.0, 0.0),
            ("C", 3.0, 0.0),
        ],
        bonds: vec![(0, 1, 1), (1, 2, 1), (2, 3, 1)],
    };
    let text = scheme(&chain, &ring(6, 0.0, 0.0, 0.0, &[]), 70.0, 12.0);
    assert_eq!(run(&text), Err("error.draw2.orientNone"));
}

#[test]
fn an_oriented_pair_is_left_alone_and_said() {
    let (_, src, tgt, _) = pairs().remove(1);
    let text = scheme(&src, &tgt, 0.0, 12.0);
    assert_eq!(run(&text), Err("error.draw2.orientDone"));
}

#[test]
fn a_result_that_overlaps_is_not_applied() {
    // the target sits so near the source that turning it brings its atoms onto the source's: refused, nothing changed
    let (_, src, tgt, _) = pairs().remove(0);
    let text = scheme(&src, &tgt, 70.0, 2.2);
    let p = plan(&doc(&text));
    if p.steps.iter().any(|s| s.status == Status::Overlap) {
        assert!(p.ops.is_empty());
        assert_eq!(run(&text), Err("error.draw2.orientOverlap"));
    } else {
        // not close enough to collide on this input: the geometry of the test is wrong, not the code
        panic!("the overlap input does not overlap: {:?}", p.steps);
    }
}

#[test]
fn a_chain_of_two_arrows_orients_each_on_the_one_before() {
    // A -> B -> C with B and C drawn turned by 70 and 130 degrees
    let a = ring(6, 0.0, 0.0, 0.0, &["N"]);
    let a = with_tail(a, 1, &[("C", 120.0)]);
    let rot = |m: &Mol, deg: f64, ox: f64| -> Vec<Value> {
        let n = m.atoms.len() as f64;
        let (cx, cy) = (
            m.atoms.iter().map(|p| p.1).sum::<f64>() / n,
            m.atoms.iter().map(|p| p.2).sum::<f64>() / n,
        );
        let t = deg.to_radians();
        m.atoms
            .iter()
            .map(|p| {
                let (dx, dy) = (p.1 - cx, p.2 - cy);
                json!([
                    p.0,
                    ((ox + dx * t.cos() - dy * t.sin()) * 1e4).round() / 1e4,
                    ((dx * t.sin() + dy * t.cos()) * 1e4).round() / 1e4
                ])
            })
            .collect()
    };
    let mut atoms = Vec::new();
    let mut bonds = Vec::new();
    for (k, (deg, ox)) in [(0.0, 0.0), (70.0, 10.0), (130.0, 20.0)]
        .into_iter()
        .enumerate()
    {
        for (i, p) in rot(&a, deg, ox).iter().enumerate() {
            atoms.push(json!({"id": 100 * k + i + 1, "el": p[0], "x": p[1], "y": p[2]}));
        }
        for (i, b) in a.bonds.iter().enumerate() {
            bonds.push(json!({"id": 1000 + 100 * k + i, "a": 100 * k + b.0 + 1, "b": 100 * k + b.1 + 1, "o": b.2}));
        }
    }
    let text = json!({"format":"mzdraw/1","next":2000,"atoms":atoms,"bonds":bonds,"objects":[
        {"id":900,"t":"arrow","x":4,"y":0,"x2":6,"y2":0},{"id":901,"t":"arrow","x":14,"y":0,"x2":16,"y2":0}]})
    .to_string();
    let out = doc(&run(&text).unwrap().to_string());
    let angle = |k: usize| {
        let (p, q) = (
            out.atom(100 * k as u32 + 1).unwrap(),
            out.atom(100 * k as u32 + 7).unwrap(),
        );
        (q.y - p.y).atan2(q.x - p.x).to_degrees()
    };
    for k in 1..3 {
        let d = (angle(k) - angle(0)).rem_euclid(360.0);
        assert!(
            !(0.1..=359.9).contains(&d),
            "structure {k} is {d} degrees from the first"
        );
    }
}
