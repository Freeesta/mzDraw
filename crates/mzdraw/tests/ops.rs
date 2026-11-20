//! Every operation, its refusals, and the undo property: any sequence of steps undone gives back the identical document bytes, and
//! redone gives the identical final bytes; a replayed log rebuilds the same document.

use mzdraw::{Axis, Document, ObjKind, Op, Session};

fn add(el: &str, x: f64, y: f64) -> Op {
    Op::AtomAdd {
        id: None,
        el: el.into(),
        x,
        y,
    }
}

fn bond(a: u32, b: u32, o: u8) -> Op {
    Op::BondAdd {
        id: None,
        a,
        b,
        o,
        s: 0,
    }
}

fn set_order(id: u32, o: u8) -> Op {
    Op::BondSet {
        id,
        o: Some(o),
        s: None,
        rev: false,
    }
}

fn ethanol() -> Session {
    let mut s = Session::default();
    s.run(&[add("C", 0.0, 0.0), add("C", 1.0, 0.0), add("O", 2.0, 0.0)])
        .unwrap();
    s.run(&[bond(1, 2, 1), bond(2, 3, 1)]).unwrap();
    s
}

#[test]
fn atoms_and_bonds_take_ids_from_one_counter() {
    let s = ethanol();
    assert_eq!(
        s.doc.to_json(),
        r#"{"format":"mzdraw/1","next":6,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0},{"id":2,"el":"C","x":1.0,"y":0.0},{"id":3,"el":"O","x":2.0,"y":0.0}],"bonds":[{"id":4,"a":1,"b":2,"o":1},{"id":5,"a":2,"b":3,"o":1}]}"#
    );
    assert_eq!(Document::from_json(&s.doc.to_json()).unwrap(), s.doc);
}

#[test]
fn refused_operations_change_nothing() {
    let mut s = ethanol();
    let before = s.doc.to_json();
    for (ops, key) in [
        (vec![add("Xx", 0.0, 0.0)], "error.draw2.element"),
        (vec![bond(1, 9, 1)], "error.draw2.noAtom"),
        (vec![bond(1, 1, 1)], "error.draw2.noAtom"),
        (vec![bond(1, 2, 1)], "error.draw2.bond"),
        (vec![bond(1, 3, 4)], "error.draw2.bond"),
        (vec![set_order(77, 2)], "error.draw2.noBond"),
        (
            vec![Op::Remove {
                atoms: vec![42],
                bonds: vec![],
                objects: vec![],
            }],
            "error.draw2.noAtom",
        ),
        (
            vec![Op::Move {
                atoms: vec![1, 42],
                objects: vec![],
                dx: 1.0,
                dy: 0.0,
            }],
            "error.draw2.noAtom",
        ),
        (
            vec![Op::AtomAdd {
                id: Some(2),
                el: "C".into(),
                x: 0.0,
                y: 0.0,
            }],
            "error.draw2.idTaken",
        ),
        (
            vec![Op::Ring {
                n: 9,
                arom: false,
                atom: None,
                bond: None,
                x: 0.0,
                y: 0.0,
            }],
            "error.draw2.ring",
        ),
        // the first operation is fine, the second is not: the whole step is rolled back
        (
            vec![add("N", 5.0, 5.0), bond(1, 99, 1)],
            "error.draw2.noAtom",
        ),
    ] {
        assert_eq!(s.run(&ops).unwrap_err().key, key);
        assert_eq!(s.doc.to_json(), before, "{key}");
    }
    assert!(Document::from_json("{}").is_err());
}

#[test]
fn set_move_and_remove() {
    let mut s = ethanol();
    s.run(&[Op::AtomSet {
        id: 3,
        el: Some("N".into()),
        chg: Some(1),
        iso: None,
        rad: None,
        h: None,
        lab: None,
    }])
    .unwrap();
    assert_eq!(
        (
            s.doc.atom(3).unwrap().el.as_str(),
            s.doc.atom(3).unwrap().chg
        ),
        ("N", 1)
    );
    s.run(&[set_order(4, 2)]).unwrap();
    assert_eq!(s.doc.bond(4).unwrap().o, 2);
    s.run(&[Op::Move {
        atoms: vec![1],
        objects: vec![],
        dx: 0.123456,
        dy: -1.0,
    }])
    .unwrap();
    assert_eq!(
        (s.doc.atom(1).unwrap().x, s.doc.atom(1).unwrap().y),
        (0.1235, -1.0)
    );
    // removing an atom takes its bonds
    s.run(&[Op::Remove {
        atoms: vec![2],
        bonds: vec![],
        objects: vec![],
    }])
    .unwrap();
    assert_eq!((s.doc.atoms.len(), s.doc.bonds.len()), (2, 0));
    assert!(s.undo());
    assert_eq!((s.doc.atoms.len(), s.doc.bonds.len()), (3, 2));
    assert_eq!(s.doc.atoms[1].id, 2, "the atom is back in its place");
}

#[test]
fn hydrogens_follow_the_valence_rules() {
    let mut s = ethanol();
    let h = |s: &Session, id| s.doc.hydrogens(s.doc.atom(id).unwrap());
    assert_eq!((h(&s, 1), h(&s, 2), h(&s, 3)), (3, 2, 1));
    s.run(&[set_order(5, 2)]).unwrap();
    assert_eq!((h(&s, 2), h(&s, 3)), (1, 0));
    s.run(&[
        Op::AtomSet {
            id: 3,
            el: None,
            chg: Some(-1),
            iso: None,
            rad: None,
            h: None,
            lab: None,
        },
        set_order(5, 1),
    ])
    .unwrap();
    assert_eq!(h(&s, 3), 0);
}

#[test]
fn rings_on_the_canvas_on_an_atom_and_fused_on_a_bond() {
    let mut s = Session::default();
    let ring = |n, arom, atom, bond| Op::Ring {
        n,
        arom,
        atom,
        bond,
        x: 0.0,
        y: 0.0,
    };
    s.run(&[ring(6, true, None, None)]).unwrap();
    assert_eq!((s.doc.atoms.len(), s.doc.bonds.len()), (6, 6));
    assert_eq!(s.doc.bonds.iter().filter(|b| b.o == 2).count(), 3);
    for b in &s.doc.bonds {
        let (p, q) = (s.doc.atom(b.a).unwrap(), s.doc.atom(b.b).unwrap());
        assert!(((p.x - q.x).hypot(p.y - q.y) - 1.0).abs() < 1e-3);
    }
    // naphthalene: four new atoms, five new bonds, every carbon keeps exactly one double bond
    let fuse = s.doc.bonds[0].id;
    s.run(&[ring(6, true, None, Some(fuse))]).unwrap();
    assert_eq!((s.doc.atoms.len(), s.doc.bonds.len()), (10, 11));
    for a in &s.doc.atoms {
        let doubles = s
            .doc
            .bonds
            .iter()
            .filter(|b| (b.a == a.id || b.b == a.id) && b.o == 2)
            .count();
        assert_eq!(doubles, 1, "atom {}", a.id);
        assert!(
            s.doc
                .atoms
                .iter()
                .all(|o| o.id == a.id || (o.x - a.x).hypot(o.y - a.y) > 0.9),
            "no overlap"
        );
    }
    // a spiro cyclopentane on an atom: four new atoms
    let corner = s.doc.atoms[9].id;
    s.run(&[ring(5, false, Some(corner), None)]).unwrap();
    assert_eq!((s.doc.atoms.len(), s.doc.bonds.len()), (14, 16));
}

/// xorshift: the tests need repeatable pseudo-random choices, not a dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as usize
    }
}

fn random_step(s: &Session, r: &mut Rng) -> Vec<Op> {
    let d = &s.doc;
    let atom = |r: &mut Rng| d.atoms.get(r.next(d.atoms.len())).map_or(0, |a| a.id);
    let bond_id = |r: &mut Rng| d.bonds.get(r.next(d.bonds.len())).map_or(0, |b| b.id);
    let obj_id = |r: &mut Rng| d.objects.get(r.next(d.objects.len())).map_or(0, |o| o.id);
    let xy = |r: &mut Rng| {
        (
            r.next(2000) as f64 / 100.0 - 10.0,
            r.next(2000) as f64 / 100.0 - 10.0,
        )
    };
    let els = ["C", "N", "O", "S", "Cl", "Fe"];
    match r.next(16) {
        0 | 1 => {
            let (x, y) = xy(r);
            vec![add(els[r.next(els.len())], x, y)]
        }
        2 => vec![bond(atom(r), atom(r), 1 + r.next(3) as u8)],
        3 => vec![Op::AtomSet {
            id: atom(r),
            el: Some(els[r.next(els.len())].into()),
            chg: Some(r.next(5) as i8 - 2),
            iso: None,
            rad: None,
            h: None,
            lab: None,
        }],
        4 => vec![set_order(bond_id(r), 1 + r.next(3) as u8)],
        5 => {
            let (dx, dy) = xy(r);
            vec![Op::Move {
                atoms: vec![atom(r)],
                objects: if r.next(2) == 1 {
                    vec![obj_id(r)]
                } else {
                    vec![]
                },
                dx,
                dy,
            }]
        }
        6 => vec![Op::Remove {
            atoms: vec![atom(r)],
            bonds: vec![bond_id(r)],
            objects: if r.next(3) == 1 {
                vec![obj_id(r)]
            } else {
                vec![]
            },
        }],
        14 => {
            let ((x, y), (x2, y2)) = (xy(r), xy(r));
            let t = [ObjKind::Arrow, ObjKind::Plus, ObjKind::Text][r.next(3)];
            // some of these are not valid objects (a plus with a text, a text without one) and must be refused whole
            let wild = r.next(4) == 0;
            let arrow = (t == ObjKind::Arrow) != wild;
            vec![Op::ObjAdd {
                id: None,
                t,
                x,
                y,
                x2: if arrow { x2 } else { 0.0 },
                y2: if arrow { y2 } else { 0.0 },
                text: if (t == ObjKind::Text) != wild {
                    "hν, 2 h".into()
                } else {
                    String::new()
                },
            }]
        }
        15 => vec![Op::ObjSet {
            id: obj_id(r),
            text: (r.next(2) == 1).then(|| ["Pd/C", "", "Δ"][r.next(3)].to_string()),
            x2: (r.next(3) == 1).then(|| xy(r).0),
            y2: None,
        }],
        9 => vec![Op::BondSet {
            id: bond_id(r),
            o: (r.next(2) == 1).then(|| 1 + r.next(3) as u8),
            s: (r.next(3) > 0).then(|| r.next(4) as u8),
            rev: r.next(2) == 1,
        }],
        10 => {
            let (x, y) = xy(r);
            vec![Op::Rotate {
                atoms: vec![atom(r), atom(r), atom(r)],
                deg: r.next(72) as f64 * 5.0 - 180.0,
                x,
                y,
            }]
        }
        11 => vec![Op::Flip {
            atoms: vec![atom(r), atom(r)],
            axis: if r.next(2) == 1 { Axis::H } else { Axis::V },
            at: xy(r).0,
        }],
        13 => vec![Op::AtomSet {
            id: atom(r),
            el: None,
            chg: None,
            iso: None,
            rad: None,
            h: None,
            lab: Some(["Ph", "R1", "", "Nope", "CO2Me"][r.next(5)].into()),
        }],
        12 => vec![Op::BondAdd {
            id: None,
            a: atom(r),
            b: atom(r),
            o: 1,
            s: 1 + r.next(3) as u8,
        }],
        7 => {
            let (x, y) = xy(r);
            let on = r.next(3);
            let atom = (on == 1).then(|| atom(r));
            let bond = (on == 2).then(|| bond_id(r));
            vec![Op::Ring {
                n: 3 + r.next(6) as u8,
                arom: r.next(2) == 1,
                atom,
                bond,
                x,
                y,
            }]
        }
        _ => {
            let (x, y) = xy(r);
            vec![
                add("C", x, y),
                Op::Move {
                    atoms: vec![atom(r)],
                    objects: vec![],
                    dx: 0.5,
                    dy: 0.5,
                },
            ]
        }
    }
}

#[test]
fn undo_restores_the_identical_document_and_redo_the_identical_result() {
    for seed in 1..=40u64 {
        let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut s = Session::default();
        let mut states = vec![s.doc.to_json()];
        for _ in 0..60 {
            let ops = random_step(&s, &mut r);
            let (before, steps) = (s.doc.to_json(), s.steps());
            if s.run(&ops).is_err() {
                assert_eq!(
                    s.doc.to_json(),
                    before,
                    "a refused step changes nothing (seed {seed})"
                );
            }
            if s.steps() > steps {
                states.push(s.doc.to_json());
            }
            assert!(
                Document::from_json(&s.doc.to_json()).is_ok(),
                "always a consistent document (seed {seed})"
            );
        }
        let last = s.doc.to_json();
        let log = s.log_json();
        // all the way back, comparing every intermediate state
        for k in (0..states.len() - 1).rev() {
            assert!(s.undo());
            assert_eq!(
                s.doc.to_json(),
                states[k],
                "undo to state {k} (seed {seed})"
            );
        }
        assert!(!s.undo());
        for st in &states[1..] {
            assert!(s.redo());
            assert_eq!(&s.doc.to_json(), st, "redo (seed {seed})");
        }
        assert_eq!(s.doc.to_json(), last);
        // the log alone rebuilds the document
        let mut again = Session::default();
        again.replay(&log).unwrap();
        assert_eq!(again.doc.to_json(), last, "replay (seed {seed})");
        assert_eq!(again.log_json(), log);
    }
}

#[test]
fn a_new_step_forgets_the_redo_stack() {
    let mut s = ethanol();
    assert!(s.undo());
    assert!(s.can_redo());
    s.run(&[add("N", 3.0, 3.0)]).unwrap();
    assert!(!s.can_redo());
    assert!(!s.redo());
}

#[test]
fn the_log_is_json_with_stable_operation_names() {
    let mut s = Session::default();
    s.replay(r#"[{"op":"atom.add","el":"C","x":0,"y":0},{"op":"atom.add","el":"N","x":1,"y":0},{"op":"bond.add","a":1,"b":2},{"op":"atom.set","id":2,"chg":1},{"op":"bond.set","id":3,"o":2},{"op":"move","atoms":[1],"dx":0.5,"dy":0},{"op":"ring","n":6,"arom":true,"bond":3},{"op":"remove","atoms":[1]}]"#).unwrap();
    assert!(s.log_json().starts_with(r#"[{"op":"atom.add","id":1,"el":"C","x":0.0,"y":0.0},{"op":"atom.add","id":2,"el":"N","x":1.0,"y":0.0},{"op":"bond.add","id":3,"a":1,"b":2,"o":1},{"op":"atom.set","id":2,"chg":1},{"op":"bond.set","id":3,"o":2},{"op":"move","atoms":[1],"dx":0.5,"dy":0.0},"#));
    assert_eq!(
        s.replay("[{\"op\":\"nope\"}]").unwrap_err().key,
        "error.draw2.log"
    );
}

#[test]
fn wedge_marks_sit_on_single_bonds_and_turn_around() {
    let mut s = ethanol();
    let wedge = |id, o, mark, rev| Op::BondSet {
        id,
        o,
        s: mark,
        rev,
    };
    s.run(&[wedge(4, None, Some(1), false)]).unwrap();
    assert_eq!(
        (s.doc.bonds[0].a, s.doc.bonds[0].b, s.doc.bonds[0].s),
        (1, 2, 1)
    );
    assert!(s
        .doc
        .to_json()
        .contains(r#"{"id":4,"a":1,"b":2,"o":1,"s":1}"#));
    // turned around: the narrow end is now atom 2
    s.run(&[wedge(4, None, None, true)]).unwrap();
    assert_eq!(
        (s.doc.bonds[0].a, s.doc.bonds[0].b, s.doc.bonds[0].s),
        (2, 1, 1)
    );
    // a double bond drops the mark; a mark on a double bond is refused
    s.run(&[wedge(4, Some(2), None, false)]).unwrap();
    assert_eq!((s.doc.bonds[0].o, s.doc.bonds[0].s), (2, 0));
    let before = s.doc.to_json();
    for bad in [
        wedge(4, None, Some(2), false),
        wedge(5, Some(3), Some(1), false),
        wedge(5, None, Some(4), false),
    ] {
        assert_eq!(s.run(&[bad]).unwrap_err().key, "error.draw2.bond");
    }
    let marked_double = Op::BondAdd {
        id: None,
        a: 1,
        b: 3,
        o: 2,
        s: 1,
    };
    assert_eq!(s.run(&[marked_double]).unwrap_err().key, "error.draw2.bond");
    assert_eq!(s.doc.to_json(), before);
    // a document with a mark on a double bond is not a document
    assert!(Document::from_json(&before.replace(r#""o":2}"#, r#""o":2,"s":1}"#)).is_err());
    for _ in 0..3 {
        assert!(s.undo());
    }
    assert_eq!(s.doc.to_json(), ethanol().doc.to_json());
    // the log keeps the old spelling of an order change and adds the new fields only when used
    assert_eq!(
        serde_json::to_string(&[set_order(4, 2), wedge(4, None, Some(3), true)]).unwrap(),
        r#"[{"op":"bond.set","id":4,"o":2},{"op":"bond.set","id":4,"s":3,"rev":true}]"#
    );
}

#[test]
fn rotate_and_flip_move_the_atoms_exactly_and_undo() {
    let mut s = ethanol();
    let start = s.doc.to_json();
    s.run(&[Op::Rotate {
        atoms: vec![1, 2, 3],
        deg: 90.0,
        x: 1.0,
        y: 0.0,
    }])
    .unwrap();
    let xy: Vec<(f64, f64)> = s.doc.atoms.iter().map(|a| (a.x, a.y)).collect();
    assert_eq!(xy, [(1.0, -1.0), (1.0, 0.0), (1.0, 1.0)]);
    s.run(&[Op::Flip {
        atoms: vec![1, 3],
        axis: Axis::V,
        at: 0.5,
    }])
    .unwrap();
    let xy: Vec<(f64, f64)> = s.doc.atoms.iter().map(|a| (a.x, a.y)).collect();
    assert_eq!(xy, [(1.0, 2.0), (1.0, 0.0), (1.0, 0.0)]);
    s.run(&[Op::Flip {
        atoms: vec![1],
        axis: Axis::H,
        at: 0.0,
    }])
    .unwrap();
    assert_eq!(s.doc.atoms[0].x, -1.0);
    assert_eq!(
        s.log_json().split("{\"op\":\"rotate\"").nth(1).unwrap(),
        r#","atoms":[1,2,3],"deg":90.0,"x":1.0,"y":0.0},{"op":"flip","atoms":[1,3],"axis":"v","at":0.5},{"op":"flip","atoms":[1],"axis":"h","at":0.0}]"#
    );
    let refused = Op::Rotate {
        atoms: vec![1, 42],
        deg: 10.0,
        x: 0.0,
        y: 0.0,
    };
    let before = s.doc.to_json();
    assert_eq!(s.run(&[refused]).unwrap_err().key, "error.draw2.noAtom");
    assert_eq!(s.doc.to_json(), before);
    for _ in 0..3 {
        assert!(s.undo());
    }
    assert_eq!(s.doc.to_json(), start);
}
