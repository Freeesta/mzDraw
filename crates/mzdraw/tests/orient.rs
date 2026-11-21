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

/// The answer of a request with `report`: the `draw-result/1` document.
fn report(text: &str, commands: Value) -> Value {
    let r = json!({"import": text, "commands": commands, "report": true});
    serde_json::from_str(&mzdraw::replay::replay(&r.to_string()).unwrap()).unwrap()
}

#[test]
fn an_oriented_scheme_is_a_no_op_not_a_refusal_and_twice_is_the_same_as_once() {
    let (_, src, tgt, _) = pairs().remove(1);
    // oriented already: ok, changed false, the step says `already`
    let text = scheme(&src, &tgt, 0.0, 12.0);
    let v = report(&text, json!([{"command": "scheme:orient"}]));
    assert_eq!(
        (v["ok"].clone(), v["changed"].clone()),
        (json!(true), json!(false)),
        "{v}"
    );
    assert_eq!(v["orient"][0]["status"], "already");
    // turned: orient twice is one turn; the second is a no-op, not an orientDone failure
    let text = scheme(&src, &tgt, 70.0, 12.0);
    let once = report(&text, json!([{"command": "scheme:orient"}]));
    let twice = report(
        &text,
        json!([{"command": "scheme:orient"}, {"command": "scheme:orient"}]),
    );
    assert_eq!(
        (twice["ok"].clone(), twice["changed"].clone()),
        (json!(true), json!(true)),
        "{twice}"
    );
    assert_eq!(
        once["out"], twice["out"],
        "the second command changes nothing"
    );
    assert_eq!(once["orient"][0]["status"], "oriented");
    assert_eq!(
        twice["orient"][0]["status"], "already",
        "the report is the one of the last run"
    );
    // the bare request writes the same document
    let bare = run(&text).unwrap();
    assert_eq!(
        bare,
        serde_json::from_str::<Value>(once["out"].as_str().unwrap()).unwrap()
    );
}

#[test]
fn the_result_document_says_each_pair_dry_run_and_report() {
    let (_, src, tgt, _) = pairs().remove(0);
    let text = scheme(&src, &tgt, 70.0, 12.0);
    let v = report(&text, json!([{"command": "scheme:orient"}]));
    let s = &v["orient"][0];
    assert_eq!(
        (
            s["status"].clone(),
            s["from"].clone(),
            s["to"].clone(),
            s["core"].clone()
        ),
        (json!("oriented"), json!(1), json!(101), json!(7))
    );
    assert!(s["before"].as_f64().unwrap() > 0.5 && s["after"].as_f64().unwrap() < FIT);
    // the keys are written in the order of ORIENT_KEYS (checked on the raw text)
    let raw = mzdraw::replay::replay(
        &json!({"import": text, "commands": [{"command": "scheme:orient"}], "report": true})
            .to_string(),
    )
    .unwrap();
    let at = raw.find("\"orient\":[").unwrap();
    let mut last = at;
    for k in &mzdraw::outcome::ORIENT_KEYS[..9] {
        let i = raw[last..]
            .find(&format!("\"{k}\":"))
            .map(|i| i + last)
            .unwrap_or_else(|| panic!("{k} missing or out of order in {raw}"));
        last = i;
    }
    // a dry run reports the same and writes nothing
    let r = json!({"import": text, "commands": [{"command": "scheme:orient"}], "dry_run": true});
    let d: Value = serde_json::from_str(&mzdraw::replay::replay(&r.to_string()).unwrap()).unwrap();
    assert_eq!(d["orient"], v["orient"]);
    assert!(d.get("out").is_none() && d["changed"] == json!(true));
    // a request without the command has neither key
    let none = report(&text, json!([]));
    assert!(none.get("orient").is_none() && none.get("changed").is_none());
    // a refusal keeps the steps: no core, and the key
    let text = scheme(
        &ring(5, 0.0, 0.0, 90.0, &[]),
        &ring(6, 0.0, 0.0, 0.0, &[]),
        70.0,
        12.0,
    );
    let v = report(&text, json!([{"command": "scheme:orient"}]));
    assert_eq!(v["ok"], json!(false));
    assert_eq!(v["failed"]["key"], "error.draw2.orientNone");
    assert_eq!(v["orient"][0]["status"], "no_core");
}

#[test]
fn several_angles_and_a_mirrored_target_end_within_the_fit() {
    // every pair, every angle (not only 70), and the same with the target drawn as a mirror image (no wedge: a flip is allowed)
    for deg in [15.0, 90.0, 170.0, 200.0, 300.0] {
        for mirrored in [false, true] {
            for (name, src, tgt, core) in pairs() {
                // the mirror image of the target about the vertical axis (a symmetric core stays a valid match)
                let tgt = if mirrored {
                    Mol {
                        atoms: tgt.atoms.iter().map(|a| (a.0, -a.1, a.2)).collect(),
                        bonds: tgt.bonds.clone(),
                    }
                } else {
                    tgt
                };
                let text = scheme(&src, &tgt, deg, 12.0);
                let out = match run(&text) {
                    Ok(v) => doc(&v.to_string()),
                    Err(e) => panic!("{name} at {deg} mirrored {mirrored}: {e}"),
                };
                let after = rms(&out, &core);
                assert!(
                    after < FIT,
                    "{name} at {deg} mirrored {mirrored}: rms {after}"
                );
            }
        }
    }
}

/// A + B -> C: A has no core with C, B has: C is oriented on B (the structures are classified by the one rule of `to_rxn`).
#[test]
fn a_product_is_oriented_on_the_reactant_that_has_a_core() {
    let a = Mol {
        atoms: vec![("Cl", 0.0, 0.0), ("Cl", 1.0, 0.0)],
        bonds: vec![(0, 1, 1)],
    };
    let b = with_tail(ring(6, 0.0, 0.0, 0.0, &["N"]), 1, &[("C", 120.0)]);
    let c = with_tail(
        with_tail(ring(6, 0.0, 0.0, 0.0, &["N"]), 1, &[("C", 120.0)]),
        6,
        &[("O", 30.0)],
    );
    let mk = |reactants: [&Mol; 2], deg: f64| {
        let mut atoms = Vec::new();
        let mut bonds = Vec::new();
        for (k, (m, ox, d)) in [
            (reactants[0], 0.0, 0.0),
            (reactants[1], 6.0, 0.0),
            (&c, 20.0, deg),
        ]
        .into_iter()
        .enumerate()
        {
            let n = m.atoms.len() as f64;
            let (cx, cy) = (
                m.atoms.iter().map(|p| p.1).sum::<f64>() / n,
                m.atoms.iter().map(|p| p.2).sum::<f64>() / n,
            );
            let t = d.to_radians();
            for (i, p) in m.atoms.iter().enumerate() {
                let (dx, dy) = (p.1 - cx, p.2 - cy);
                atoms.push(json!({"id": 100 * k + i + 1, "el": p.0, "x": ((ox + dx * t.cos() - dy * t.sin()) * 1e4).round() / 1e4, "y": ((dx * t.sin() + dy * t.cos()) * 1e4).round() / 1e4}));
            }
            for (i, bd) in m.bonds.iter().enumerate() {
                bonds.push(json!({"id": 1000 + 100 * k + i, "a": 100 * k + bd.0 + 1, "b": 100 * k + bd.1 + 1, "o": bd.2}));
            }
        }
        json!({"format":"mzdraw/1","next":3000,"atoms":atoms,"bonds":bonds,"objects":[
            {"id":900,"t":"plus","x":3,"y":0},{"id":901,"t":"arrow","x":9,"y":0,"x2":15,"y2":0}]})
        .to_string()
    };
    for (first, second) in [(&a, &b), (&b, &a)] {
        let text = mk([first, second], 70.0);
        let v = report(&text, json!([{"command": "scheme:orient"}]));
        assert_eq!(v["ok"], json!(true), "{v}");
        assert_eq!(v["orient"].as_array().unwrap().len(), 1);
        assert_eq!(v["orient"][0]["status"], "oriented");
        // the reactant it was oriented on is the methylpyridine (ids 100..), whatever its place before the arrow
        let out = doc(v["out"].as_str().unwrap());
        let bk = if std::ptr::eq(first, &b) { 0 } else { 1 };
        let ang = |d: &Document, a: u32, b: u32| {
            let (p, q) = (d.atom(a).unwrap(), d.atom(b).unwrap());
            (q.y - p.y).atan2(q.x - p.x).to_degrees()
        };
        let turn = ((ang(&out, 201, 207) - ang(&out, 100 * bk + 1, 100 * bk + 7) + 540.0) % 360.0
            - 180.0)
            .abs();
        assert!(turn < 0.1, "off by {turn}");
    }
}

/// Orient and row do not fight: run in either order, a second run of each changes nothing.
#[test]
fn orient_and_row_in_either_order_are_stable() {
    let (_, src, tgt, _) = pairs().remove(0);
    let text = scheme(&src, &tgt, 70.0, 12.0);
    let run_cmds = |cmds: &[&str]| -> String {
        let list: Vec<Value> = cmds.iter().map(|c| json!({"command": c})).collect();
        let v = report(&text, Value::Array(list));
        assert_eq!(v["ok"], json!(true), "{cmds:?}: {v}");
        v["out"].as_str().unwrap().to_string()
    };
    // orient, then row: the order the note says; a second round changes nothing
    let once = run_cmds(&["scheme:orient", "scheme:row"]);
    assert_eq!(
        once,
        run_cmds(&["scheme:orient", "scheme:row", "scheme:orient", "scheme:row"])
    );
    assert!(plan(&doc(&once)).ops.is_empty());
    // row, then orient: the turn changes the box, so a row after it may still move the structures (it never undoes the turn);
    // from there nothing changes
    let first = run_cmds(&["scheme:row", "scheme:orient"]);
    assert!(plan(&doc(&first)).ops.is_empty(), "oriented");
    let settled = run_cmds(&["scheme:row", "scheme:orient", "scheme:row"]);
    assert_eq!(
        settled,
        run_cmds(&[
            "scheme:row",
            "scheme:orient",
            "scheme:row",
            "scheme:orient",
            "scheme:row"
        ])
    );
    assert!(
        plan(&doc(&settled)).ops.is_empty(),
        "still oriented after the row"
    );
}

#[test]
fn a_turn_that_grows_the_box_into_an_object_is_refused() {
    // a target whose turn grows its box into a text is refused, though no atom touches an atom
    let tail: Vec<(&'static str, f64)> = vec![("C", 0.0), ("C", 30.0), ("C", 0.0), ("C", 30.0)];
    let long = with_tail(ring(6, 0.0, 0.0, 0.0, &["N"]), 0, &tail);
    // the target is drawn turned by 70 degrees; turning it back grows its box on some side
    let turned = scheme(&long, &long, 70.0, 12.0);
    let base = doc(&turned);
    let bounds = |d: &Document| {
        (101..=300)
            .filter_map(|i| d.atom(i))
            .fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |b, a| {
                (b.0.min(a.x), b.1.min(a.y), b.2.max(a.x), b.3.max(a.y))
            })
    };
    let (_, ops) = solve(&base, 1, 101);
    assert!(!ops.is_empty());
    let mut s = mzdraw::Session::default();
    s.doc = base.clone();
    s.run(&ops).unwrap();
    let (old, new) = (bounds(&base), bounds(&s.doc));
    // a corner of the new box that is farther than the reach (0.3) from the old box
    let far = |p: (f64, f64)| {
        p.0 < old.0 - 0.31 || p.0 > old.2 + 0.31 || p.1 < old.1 - 0.31 || p.1 > old.3 + 0.31
    };
    let corner = [
        (new.0, new.1),
        (new.2, new.1),
        (new.0, new.3),
        (new.2, new.3),
    ]
    .into_iter()
    .find(|&p| far(p))
    .expect("the box grows");
    let mut v: Value = serde_json::from_str(&turned).unwrap();
    v["objects"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": 950, "t": "text", "x": corner.0, "y": corner.1, "text": "cat"}));
    let with_text = v.to_string();
    let p = plan(&doc(&with_text));
    assert_eq!(p.steps[0].status, Status::Overlap, "{:?}", p.steps);
    assert_eq!(p.steps[0].at, 950);
    assert!(p.ops.is_empty());
    assert_eq!(run(&with_text), Err("error.draw2.orientOverlap"));
    // without the text, the same drawing is oriented
    assert_eq!(plan(&base).steps[0].status, Status::Oriented);
}

#[test]
fn a_chain_with_a_wedged_product_is_turned_not_mirrored() {
    // A -> B -> C, C drawn as a mirror image with a wedge: it is turned as far as a rotation fits, never flipped
    let core: Vec<(usize, usize)> = (0..4).map(|i| (i, i)).collect();
    let text = scheme(&chain(false), &chain(false), 130.0, 12.0);
    let mut v: Value = serde_json::from_str(&text).unwrap();
    v["bonds"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|b| b["id"] == 151)
        .unwrap()["s"] = json!(1);
    let out = report(&v.to_string(), json!([{"command": "scheme:orient"}]));
    assert_eq!(out["orient"][0]["flip"], json!(false));
    assert_eq!(out["orient"][0]["status"], "oriented");
    assert!(rms(&doc(out["out"].as_str().unwrap()), &core) < FIT);
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

/// The cases a maximum-common-substructure search would orient and this slice does not (they are listed in `docs/built/scheme-orient.md`
/// for COO 16): reported as no core, nothing changed.
#[test]
fn cases_for_a_real_mcs_are_reported_as_no_core() {
    // 1. a shared propyl chain on two different rings: propylbenzene -> propylcyclohexane (core C-C-C and the ring carbon: 4 atoms)
    let tail = [("C", 0.0), ("C", 30.0), ("C", 0.0)];
    let benzene = with_tail(ring(6, 0.0, 0.0, 0.0, &[]), 0, &tail);
    let mut cyclohexane = with_tail(ring(6, 0.0, 0.0, 0.0, &[]), 0, &tail);
    for b in &mut cyclohexane.bonds {
        b.2 = 1;
    }
    let text = scheme(&benzene, &cyclohexane, 70.0, 12.0);
    assert_eq!(plan(&doc(&text)).steps[0].status, Status::NoCore);
    assert_eq!(run(&text), Err("error.draw2.orientNone"));
    // 2. two ring systems that share only a part: indane -> tetralin share the benzene ring (6 atoms)
    let hexa = ring(6, 0.0, 0.0, 0.0, &[]);
    let indane = fused(&hexa, 5, &[]);
    let tetralin = fused(&ring(6, 0.0, 0.0, 0.0, &[]), 6, &[]);
    let text = scheme(&indane, &tetralin, 70.0, 12.0);
    assert_eq!(plan(&doc(&text)).steps[0].status, Status::NoCore);
}
