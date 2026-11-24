//! Every operation, its refusals, and the undo property: any sequence of steps undone gives back the identical document bytes, and
//! redone gives the identical final bytes; a replayed log rebuilds the same document.

use mzdraw::{Axis, Document, ObjKind, Op, Session};

fn add(el: &str, x: f64, y: f64) -> Op {
    Op::AtomAdd {
        map: None,
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
        pos: None,
        k: None,
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
                map: None,
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
        (
            vec![Op::Chain {
                n: 0,
                atom: None,
                x: 0.0,
                y: 0.0,
                deg: 0.0,
            }],
            "error.draw2.chain",
        ),
        (
            vec![Op::Chain {
                n: 51,
                atom: None,
                x: 0.0,
                y: 0.0,
                deg: 0.0,
            }],
            "error.draw2.chain",
        ),
        (
            vec![Op::Chain {
                n: 3,
                atom: Some(99),
                x: 0.0,
                y: 0.0,
                deg: 0.0,
            }],
            "error.draw2.noAtom",
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
        map: None,
        id: 3,
        el: Some("N".into()),
        chg: Some(1),
        iso: None,
        rad: None,
        h: None,
        lab: None,
        lp: None,
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
            map: None,
            id: 3,
            el: None,
            chg: Some(-1),
            iso: None,
            rad: None,
            h: None,
            lab: None,
            lp: None,
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

#[test]
fn a_chain_that_cannot_be_drawn_is_refused_whole_with_its_own_text() {
    let mut s = Session::default();
    s.run(&[add("C", 0.0, 0.0)]).unwrap();
    let before = s.doc.to_json();
    let chain = |n, x, y, deg| Op::Chain {
        n,
        atom: None,
        x,
        y,
        deg,
    };
    for (op, key) in [
        (chain(3, 0.0, 0.0, f64::NAN), "error.draw2.chainAngle"),
        (chain(3, 0.0, 0.0, f64::INFINITY), "error.draw2.chainAngle"),
        (chain(3, f64::NAN, 0.0, 0.0), "error.draw2.chainAngle"),
        (
            chain(3, 0.0, f64::NEG_INFINITY, 0.0),
            "error.draw2.chainAngle",
        ),
        (chain(0, 0.0, 0.0, 0.0), "error.draw2.chain"),
    ] {
        assert_eq!(s.run(&[op]).unwrap_err().key, key);
        assert_eq!(s.doc.to_json(), before);
    }
    // a chain that leaves the range of coordinates is refused as any far atom is, and leaves nothing behind
    assert!(s.run(&[chain(5, 1e9, 0.0, 0.0)]).is_err());
    assert_eq!(s.doc.to_json(), before);
    // in a log: a count above 255 is a count the operation refuses, a number too big for a double an unreadable log
    let mut r = Session::default();
    assert_eq!(
        r.replay(r#"[{"op":"chain","n":300,"x":0,"y":0}]"#)
            .unwrap_err()
            .key,
        "error.draw2.chain"
    );
    assert_eq!(
        r.replay(r#"[{"op":"chain","n":3,"x":1e999,"y":0}]"#)
            .unwrap_err()
            .key,
        "error.draw2.log"
    );
    assert!(r.doc.atoms.is_empty());
}

#[test]
fn chains_zigzag_from_a_point_and_from_an_atom() {
    let mut s = Session::default();
    let chain = |n, atom, deg| Op::Chain {
        n,
        atom,
        x: 1.0,
        y: 2.0,
        deg,
    };
    // five bonds from a point: six carbons, one bond length each, 120 degrees at every inner atom
    s.run(&[chain(5, None, 30.0)]).unwrap();
    assert_eq!((s.doc.atoms.len(), s.doc.bonds.len()), (6, 5));
    let p: Vec<(f64, f64)> = s.doc.atoms.iter().map(|a| (a.x, a.y)).collect();
    assert_eq!(p[0], (1.0, 2.0));
    for w in p.windows(3) {
        let (u, v) = (
            (w[0].0 - w[1].0, w[0].1 - w[1].1),
            (w[2].0 - w[1].0, w[2].1 - w[1].1),
        );
        let angle = ((u.0 * v.0 + u.1 * v.1) / (u.0.hypot(u.1) * v.0.hypot(v.1))).acos();
        assert!((angle.to_degrees() - 120.0).abs() < 0.01, "{angle}");
    }
    for w in p.windows(2) {
        assert!(((w[1].0 - w[0].0).hypot(w[1].1 - w[0].1) - 1.0).abs() < 1e-4);
    }
    // the first bond leaves at 30 degrees, the chain advances along the horizontal
    assert!((p[1].0 - p[0].0 - 3.0_f64.sqrt() / 2.0).abs() < 1e-4);
    assert!((p[1].1 - p[0].1 - 0.5).abs() < 1e-4);
    assert!((p[2].1 - p[0].1).abs() < 1e-4);
    // from the last atom: three more bonds, the start atom is not added again
    let last = s.doc.atoms[5].id;
    s.run(&[chain(3, Some(last), 150.0)]).unwrap();
    assert_eq!((s.doc.atoms.len(), s.doc.bonds.len()), (9, 8));
    // a second chain over the same path adds nothing new
    let before = s.doc.to_json();
    s.run(&[chain(5, None, 30.0)]).unwrap();
    assert_eq!(s.doc.to_json(), before);
    // one step, one undo
    // (the repeated chain changed nothing and left no step)
    assert!(s.undo());
    assert_eq!(s.doc.atoms.len(), 6);
    assert!(s.undo());
    assert!(s.doc.atoms.is_empty());
    // the largest chain is 50 bonds
    s.run(&[chain(50, None, 0.0)]).unwrap();
    assert_eq!(s.doc.bonds.len(), 50);
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
    match r.next(20) {
        18 => {
            // a curved arrow: anchors on atoms, bonds, lone pairs and points; many of these are refused (no pair, same end twice)
            let ((x, y), (x2, y2)) = (xy(r), xy(r));
            let end = |r: &mut Rng| match r.next(4) {
                0 => 0,
                1 => bond_id(r),
                _ => atom(r),
            };
            vec![Op::ObjAdd {
                id: None,
                t: ObjKind::Curve,
                x,
                y,
                x2,
                y2,
                text: String::new(),
                runs: Vec::new(),
                from: end(r),
                to: end(r),
                cv: mzdraw::curve::Curve {
                    half: r.next(2) == 1,
                    bend: [0.0, 0.4, -0.7, 5.0][r.next(4)],
                    fl: r.next(3) == 0,
                    tl: r.next(3) == 0,
                },
            }]
        }
        19 => vec![Op::ObjSet {
            id: obj_id(r),
            text: None,
            runs: None,
            x2: None,
            y2: None,
            from: [None, Some(0), Some(atom(r))][r.next(3)],
            to: [None, Some(0), Some(bond_id(r))][r.next(3)],
            cv: mzdraw::curve::CurveSet {
                half: [None, Some(true), Some(false)][r.next(3)],
                bend: [None, Some(0.2), Some(-1.3)][r.next(3)],
                fl: [None, Some(false), Some(true)][r.next(3)],
                tl: None,
            },
            pt: [None, None],
        }],
        0 | 1 => {
            let (x, y) = xy(r);
            let mut a = add(els[r.next(els.len())], x, y);
            if let Op::AtomAdd { map, .. } = &mut a {
                *map = [None, None, Some(3), Some(1000)][r.next(4)];
            }
            vec![a]
        }
        2 => vec![bond(atom(r), atom(r), 1 + r.next(3) as u8)],
        3 => vec![Op::AtomSet {
            // atom-to-atom maps: set, changed, taken away (0) and refused (above 999)
            map: [None, Some(0), Some(1), Some(7), Some(999), Some(1000)][r.next(6)],
            id: atom(r),
            el: Some(els[r.next(els.len())].into()),
            chg: Some(r.next(5) as i8 - 2),
            iso: None,
            rad: None,
            h: None,
            lab: None,
            // lone pairs: set, taken away, refused (above four)
            lp: [None, None, Some(0), Some(1), Some(2), Some(5)][r.next(6)],
        }],
        4 => vec![set_order(bond_id(r), 1 + r.next(3) as u8)],
        15 if r.next(3) == 0 => {
            // an annotation on an atom: set, a history, taken away
            let levels = ["1", "2a", "2b", "3", "4", "5", "3a"];
            let entries: Vec<String> = (0..r.next(3))
                .map(|_| {
                    format!(
                        r#"{{"evidence":[{{"kind":"formula"}}],"level":"{}"}}"#,
                        levels[r.next(7)]
                    )
                })
                .collect();
            vec![Op::AtomAnn {
                id: atom(r),
                ann: format!("[{}]", entries.join(",")),
            }]
        }
        15 if r.next(4) == 0 => {
            // an enhanced-stereo group on an atom: set, replaced, taken away, refused (a group that is none)
            let groups = ["", "abs", "and1", "or2", "and99", "or0", "and100", "&1"];
            vec![Op::AtomSg {
                id: atom(r),
                sg: groups[r.next(8)].into(),
            }]
        }
        15 if r.next(3) == 0 => {
            // a title on an atom: set, replaced, taken away
            let words = ["", "ethanol", "a name with spaces", "caf\u{e9} 2"];
            vec![Op::AtomTitle {
                id: atom(r),
                title: words[r.next(4)].into(),
            }]
        }
        15 if r.next(2) == 0 => {
            // data on an atom: set, replaced, taken away
            let fields = ["name", "CAS (synthetic)", "note"];
            let data = (0..r.next(3))
                .map(|k| {
                    (
                        fields[k].to_string(),
                        format!("value {}\nline two", r.next(9)),
                    )
                })
                .collect();
            vec![Op::AtomData { id: atom(r), data }]
        }
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
                // a formatted text (some are refused: a plus has no text, a text with only a line break is blank)
                runs: if r.next(3) == 0 {
                    vec![
                        mzdraw::Run::plain("H"),
                        mzdraw::Run::sub("2"),
                        mzdraw::Run::line(),
                        mzdraw::Run {
                            color: "#0033cc".into(),
                            b: true,
                            ..mzdraw::Run::sup("+")
                        },
                    ]
                } else {
                    Vec::new()
                },
                // anchors on arrows: an atom of the document, or none; a wild one is anchored on a plus or a text (refused whole)
                from: if (arrow || wild) && r.next(2) == 1 {
                    atom(r)
                } else {
                    0
                },
                to: if (arrow || wild) && r.next(2) == 1 {
                    atom(r)
                } else {
                    0
                },
                cv: Default::default(),
            }]
        }
        15 => {
            let text = (r.next(2) == 1).then(|| ["Pd/C", "", "Δ"][r.next(3)].to_string());
            // runs with a text too are refused whole; runs alone replace the text, an empty list leaves none
            let runs = (r.next(3) == 0).then(|| match r.next(3) {
                0 => vec![],
                1 => vec![
                    mzdraw::Run::plain("a"),
                    mzdraw::Run::line(),
                    mzdraw::Run::sub("b"),
                ],
                _ => vec![
                    mzdraw::Run {
                        i: true,
                        sz: 1.5,
                        ..mzdraw::Run::plain("Pd")
                    },
                    mzdraw::Run::sup("2+"),
                ],
            });
            vec![Op::ObjSet {
                id: obj_id(r),
                text,
                runs,
                x2: (r.next(3) == 1).then(|| xy(r).0),
                y2: None,
                // set, change and clear (0) the anchors; a set on a plus or a text is refused whole
                from: (r.next(3) == 1).then(|| if r.next(2) == 1 { atom(r) } else { 0 }),
                to: (r.next(3) == 1).then(|| if r.next(2) == 1 { atom(r) } else { 0 }),
                cv: Default::default(),
                pt: [None, None],
            }]
        }
        8 => {
            // colour of atoms and bonds: set (short and long hex, any case), changed, cleared (automatic), and refused ones
            // (not a colour, an id the document lacks) that must leave the document as it was
            let cols = ["#d00000", "#0F8", "#1F3FD0", "", "red", "#12345", "#7b2fbe"];
            // the lines kept under a molecule: set, reordered, repeated keys, an unknown key (kept), cleared (empty), and a refused
            // one (a key of 41 characters, an atom the document lacks)
            if r.next(3) == 0 {
                let keys = [
                    "formula",
                    "logp",
                    "mw",
                    "formula",
                    "future",
                    "",
                    &"k".repeat(41),
                ];
                let lines: Vec<String> = (0..r.next(4))
                    .map(|_| keys[r.next(keys.len())].into())
                    .collect();
                let id = if r.next(6) == 0 { 9999 } else { atom(r) };
                return vec![Op::AtomCap { id, lines }];
            }
            // the mass-label choice: every word, an adduct with a mode that has none, a bad word (refused), cleared
            if r.next(5) == 0 {
                let modes = ["delta", "mass", "delta+mass", "name+delta", "", "bogus"];
                let adducts = ["", "", "[M+H]+", "[M-H]-", "Na"];
                return vec![Op::DocLabels {
                    mode: modes[r.next(modes.len())].into(),
                    adduct: adducts[r.next(adducts.len())].into(),
                }];
            }
            vec![Op::Colour {
                atoms: (0..r.next(4)).map(|_| atom(r)).collect(),
                bonds: (0..r.next(3)).map(|_| bond_id(r)).collect(),
                objects: (0..r.next(2)).map(|_| obj_id(r)).collect(),
                col: cols[r.next(cols.len())].into(),
            }]
        }
        9 => vec![Op::BondSet {
            id: bond_id(r),
            o: (r.next(2) == 1).then(|| 1 + r.next(3) as u8),
            s: (r.next(3) > 0).then(|| r.next(4) as u8),
            pos: (r.next(3) == 0).then(|| ["auto", "c", "l", "r", ""][r.next(5)].to_string()),
            k: (r.next(3) == 0).then(|| {
                [
                    "solid", "dash", "bdash", "hash", "bold", "dative", "hbond", "",
                ][r.next(8)]
                .to_string()
            }),
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
            map: None,
            id: atom(r),
            el: None,
            chg: None,
            iso: None,
            rad: None,
            h: None,
            lab: Some(["Ph", "R1", "", "Nope", "CO2Me"][r.next(5)].into()),
            lp: None,
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
        16 | 17 => {
            let (x, y) = xy(r);
            let on = r.next(3) == 1;
            vec![Op::Chain {
                n: 1 + r.next(9) as u8,
                atom: on.then(|| atom(r)),
                x,
                y,
                deg: (r.next(24) * 15) as f64,
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
    // the random steps reach lone pairs and anchored curves, and curves whose anchor went (or the property would say nothing of them)
    let (mut pairs, mut anchored, mut freed) = (0, 0, 0);
    let mut anchored_once = std::collections::HashMap::new();
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
            pairs += usize::from(s.doc.atoms.iter().any(|a| a.lp() > 0));
            anchored += usize::from(
                (s.doc.objects.iter()).any(|o| o.t == ObjKind::Curve && (o.from != 0 || o.to != 0)),
            );
            // a curve that had an anchored end and has a free one now: freed by `obj.set`, or by the removal of its anchor
            for o in s.doc.objects.iter().filter(|o| o.t == ObjKind::Curve) {
                let was = anchored_once.entry((seed, o.id)).or_insert((false, false));
                was.0 |= o.from != 0;
                was.1 |= o.to != 0;
                if (was.0 && o.from == 0) || (was.1 && o.to == 0) {
                    freed += 1;
                    break;
                }
            }
            assert!(
                Document::from_json(&s.doc.to_json()).is_ok(),
                "always a consistent document (seed {seed}): {:?} after {ops:?}\n{}",
                Document::from_json(&s.doc.to_json()).err(),
                s.doc.to_json()
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
    assert!(
        pairs > 20 && anchored > 20 && freed > 5,
        "{pairs} {anchored} {freed}"
    );
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
    s.replay(r#"[{"op":"atom.add","el":"C","x":0,"y":0},{"op":"atom.add","el":"N","x":1,"y":0},{"op":"bond.add","a":1,"b":2},{"op":"atom.set","id":2,"chg":1},{"op":"bond.set","id":3,"o":2},{"op":"move","atoms":[1],"dx":0.5,"dy":0},{"op":"ring","n":6,"arom":true,"bond":3},{"op":"chain","n":4,"atom":2,"deg":60},{"op":"chain","n":2,"x":5,"y":5},{"op":"remove","atoms":[1]}]"#).unwrap();
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
        pos: None,
        k: None,
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
        mzdraw::ops::log_text(&[set_order(4, 2), wedge(4, None, Some(3), true)]),
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

#[test]
fn the_log_reader_takes_what_the_writer_writes_and_refuses_the_rest() {
    // every operation of a random session survives JSON, value by value
    let mut r = Rng(0xD1B5_4A32_D192_ED03);
    let mut s = Session::default();
    for _ in 0..300 {
        for op in random_step(&s, &mut r) {
            let text = op.to_json();
            assert_eq!(serde_json::from_str::<Op>(&text).unwrap(), op, "{text}");
        }
        let _ = s.run(&random_step(&s, &mut r));
    }
    // defaults of fields left out, null as absent; a field outside the table of the operation is refused (`schema::OPS`)
    assert_eq!(
        serde_json::from_str::<Op>(r#"{"op":"bond.add","a":1,"b":2,"id":null}"#).unwrap(),
        bond(1, 2, 1)
    );
    for bad in [
        r#"{"op":"nope"}"#,
        r#"{"op":"bond.add","a":1,"b":2,"later":true}"#,
        r#"{"op":"merge","from":1,"into":2}"#,
        r#"{"a":1,"b":2}"#,
        r#"{"op":"bond.add","a":1}"#,
        r#"{"op":"bond.add","a":1,"b":"2"}"#,
        r#"{"op":"bond.add","a":1,"b":2,"o":300}"#,
        r#"{"op":"bond.add","a":-1,"b":2}"#,
        r#"{"op":"atom.add","el":"C","x":"0","y":0}"#,
        r#"{"op":"atom.add","el":7,"x":0,"y":0}"#,
        r#"{"op":"atom.set","id":1,"chg":200}"#,
        r#"{"op":"move","atoms":[1,"2"],"dx":0,"dy":0}"#,
        r#"{"op":"move","atoms":[1],"dx":0}"#,
        r#"{"op":"flip","atoms":[1],"axis":"x","at":0}"#,
        r#"{"op":"bond.set","id":1,"rev":1}"#,
        r#"{"op":"obj.add","t":"cloud","x":0,"y":0}"#,
        r#"{"op":"obj.add","t":"text","x":0,"y":0,"text":5}"#,
        r#"{"op":"obj.set","text":"x"}"#,
        r#"{"op":"remove","objects":[1.5]}"#,
        r#"[1,2]"#,
    ] {
        assert!(serde_json::from_str::<Op>(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_document_reader_takes_what_the_writer_writes_and_refuses_the_rest() {
    // every state of a random session survives its own JSON
    let mut r = Rng(0x2545_F491_4F6C_DD1D);
    let mut s = Session::default();
    for _ in 0..400 {
        let _ = s.run(&random_step(&s, &mut r));
        let text = s.doc.to_json();
        assert_eq!(Document::from_json(&text).unwrap(), s.doc, "{text}");
    }
    assert!(!s.doc.atoms.is_empty() && !s.doc.bonds.is_empty());
    // optional fields left out take their defaults, unknown fields are ignored
    let d = Document::from_json(
        r#"{"format":"mzdraw/1","next":4,"later":{"x":1},"atoms":[{"id":1,"el":"N","x":0,"y":0,"note":"n"},{"id":2,"el":"C","x":1,"y":0}],"bonds":[{"id":3,"a":1,"b":2,"o":1}]}"#,
    )
    .unwrap();
    assert_eq!(
        (d.atoms[0].chg, d.atoms[0].h, d.bonds[0].s, d.objects.len()),
        (0, None, 0, 0)
    );
    let good = r#"{"format":"mzdraw/1","next":4,"atoms":[{"id":1,"el":"N","x":0.0,"y":0.0},{"id":2,"el":"C","x":1.0,"y":0.0}],"bonds":[{"id":3,"a":1,"b":2,"o":1}]}"#;
    assert_eq!(Document::from_json(good).unwrap().to_json(), good);
    for (from, to) in [
        (r#""next":4"#, r#""next":"4""#),
        (r#""next":4,"#, ""),
        (r#""format":"mzdraw/1","#, ""),
        (r#""id":1,"el":"N""#, r#""id":1.5,"el":"N""#),
        (r#""el":"N""#, r#""el":7"#),
        (r#""x":1.0"#, r#""x":"1.0""#),
        (r#""x":1.0,"#, ""),
        (r#""o":1"#, r#""o":1,"s":"up""#),
        (r#""o":1"#, r#""o":999"#),
        (r#","bonds":[{"id":3,"a":1,"b":2,"o":1}]"#, ""),
        (r#""bonds":["#, r#""objects":{},"bonds":["#),
        (
            r#""bonds":["#,
            r#""objects":[{"id":9,"t":"cloud","x":0,"y":0}],"bonds":["#,
        ),
        (r#""atoms":["#, r#""atoms":[7,"#),
    ] {
        assert!(good.contains(from), "{from}");
        let bad = good.replace(from, to);
        assert!(Document::from_json(&bad).is_err(), "{bad}");
    }
    for bad in ["", "[]", "7", "null", "{"] {
        assert!(Document::from_json(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn hostile_input_is_refused_without_a_panic() {
    let doc = |next: &str, id: &str, x: &str| {
        format!(
            r#"{{"format":"mzdraw/1","next":{next},"atoms":[{{"id":{id},"el":"C","x":{x},"y":0.0}}],"bonds":[]}}"#
        )
    };
    assert!(Document::from_json(&doc("2", "1", "0.0")).is_ok());
    // ids and counters at the edge of the integer range, coordinates no JSON can carry back
    for (next, id, x) in [
        ("4294967295", "4294967294", "0.0"),
        ("4294967296", "1", "0.0"),
        ("2147483649", "2147483648", "0.0"),
        ("-1", "1", "0.0"),
        ("2", "18446744073709551615", "0.0"),
        ("2", "1", "1e300"),
        ("2", "1", "1e7"),
        ("2", "1", "-1e7"),
        ("2", "1", "1e999"),
        ("2", "1", "NaN"),
    ] {
        assert!(
            Document::from_json(&doc(next, id, x)).is_err(),
            "{next} {id} {x}"
        );
    }
    // the largest document that is allowed still takes edits, and the id that would pass the limit is refused
    let mut s = Session::new(Document::from_json(&doc("2147483648", "2147483647", "1e6")).unwrap());
    let before = s.doc.to_json();
    assert_eq!(
        s.run(&[add("C", 0.0, 0.0)]).unwrap_err().key,
        "error.draw2.idTaken"
    );
    assert_eq!(
        s.run(&[bond(2147483647, 2147483647, 1)]).unwrap_err().key,
        "error.draw2.noAtom"
    );
    let ring = Op::Ring {
        n: 6,
        arom: true,
        atom: Some(2147483647),
        bond: None,
        x: 0.0,
        y: 0.0,
    };
    assert!(s.run(&[ring]).is_err());
    // nothing leaves the range of coordinates: a move, a rotation, a flip or a far atom is refused whole
    for op in [
        Op::Move {
            atoms: vec![2147483647],
            objects: vec![],
            dx: 1.0,
            dy: 0.0,
        },
        Op::Move {
            atoms: vec![2147483647],
            objects: vec![],
            dx: f64::INFINITY,
            dy: 0.0,
        },
        Op::Rotate {
            atoms: vec![2147483647],
            deg: 45.0,
            x: -1e6,
            y: 1e6,
        },
        Op::Flip {
            atoms: vec![2147483647],
            axis: Axis::H,
            at: -1e6,
        },
        Op::Rotate {
            atoms: vec![2147483647],
            deg: f64::NAN,
            x: 0.0,
            y: 0.0,
        },
    ] {
        assert_eq!(s.run(&[op]).unwrap_err().key, "error.draw2.coordinate");
        assert_eq!(s.doc.to_json(), before);
    }
    let mut far = Session::default();
    for op in [
        add("C", 1e300, 0.0),
        add("C", f64::NAN, 0.0),
        Op::ObjAdd {
            id: None,
            t: ObjKind::Arrow,
            x: 0.0,
            y: 0.0,
            x2: f64::INFINITY,
            y2: 0.0,
            text: String::new(),
            runs: Vec::new(),
            from: 0,
            to: 0,
            cv: Default::default(),
        },
        Op::AtomAdd {
            map: None,
            id: Some(u32::MAX),
            el: "C".into(),
            x: 0.0,
            y: 0.0,
        },
    ] {
        assert!(far.run(&[op]).is_err());
        assert_eq!(far.doc.to_json(), Session::default().doc.to_json());
    }
    // JSON nested far deeper than any document, a huge array, a long string, stray bytes: read errors, never a panic
    let deep = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    let deep_obj = format!("{}1{}", r#"{"a":"#.repeat(50_000), "}".repeat(50_000));
    let many = format!(
        r#"{{"format":"mzdraw/1","next":2,"atoms":[{}1],"bonds":[]}}"#,
        "1,".repeat(200_000)
    );
    let long = format!(
        r#"{{"format":"mzdraw/1","next":2,"atoms":[{{"id":1,"el":"{}","x":0,"y":0}}],"bonds":[]}}"#,
        "C".repeat(1_000_000)
    );
    for text in [
        deep.as_str(),
        deep_obj.as_str(),
        many.as_str(),
        long.as_str(),
        "\u{0}\u{1}{",
        "{\"format\":",
    ] {
        assert!(Document::from_json(text).is_err());
        assert!(serde_json::from_str::<Op>(text).is_err());
        // as a request the text may be an object whose unknown fields are ignored: it only must not panic
        let _ = mzdraw::replay::replay(text);
        let request = serde_json::json!({ "import": text }).to_string();
        assert!(mzdraw::replay::replay(&request).is_err());
    }
    let log = format!(
        r#"{{"log":[{}{{"op":"nope"}}]}}"#,
        r#"{"op":"atom.add","el":"C","x":0,"y":0},"#.repeat(20_000)
    );
    assert_eq!(
        mzdraw::replay::replay(&log).unwrap_err().key,
        "error.draw2.request"
    );
}

#[test]
fn unknown_fields_of_a_document_survive_reading_editing_and_writing() {
    // a drawing as a later mzDraw of the same format version might save it: fields this one does not know, at every level
    let newer = r##"{"format":"mzdraw/1","next":8,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0,"colour":"#c00","ann":{"level":"2b"}},{"id":2,"el":"O","x":1.0,"y":0.0,"chg":-1}],"bonds":[{"id":3,"a":1,"b":2,"o":1,"display":"bold"}],"objects":[{"id":4,"t":"arrow","x":3.0,"y":0.0,"x2":5.0,"kind":"equilibrium"}],"meta":{"title":"sample","tags":["a","b"]},"style":"acs-1996"}"##;
    let d = Document::from_json(newer).unwrap();
    assert_eq!(d.more.len(), 2);
    assert_eq!(d.atoms[0].more.len(), 2);
    assert_eq!((d.bonds[0].more.len(), d.objects[0].more.len()), (1, 1));
    // written back: the known fields in their order, then the unknown ones by key; the same bytes on every later round
    let out = d.to_json();
    assert_eq!(
        out,
        r##"{"format":"mzdraw/1","next":8,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0,"ann":{"level":"2b"},"colour":"#c00"},{"id":2,"el":"O","x":1.0,"y":0.0,"chg":-1}],"bonds":[{"id":3,"a":1,"b":2,"o":1,"display":"bold"}],"objects":[{"id":4,"t":"arrow","x":3.0,"y":0.0,"x2":5.0,"kind":"equilibrium"}],"meta":{"tags":["a","b"],"title":"sample"},"style":"acs-1996"}"##
    );
    assert_eq!(Document::from_json(&out).unwrap(), d);
    assert_eq!(Document::from_json(&out).unwrap().to_json(), out);
    // they stay on the things they belong to through edits and come back with undo
    let mut s = Session::new(d);
    s.run(&[
        Op::Move {
            atoms: vec![1],
            objects: vec![4],
            dx: 0.5,
            dy: 0.5,
        },
        set_order(3, 2),
        Op::AtomSet {
            map: None,
            id: 1,
            el: Some("N".into()),
            chg: None,
            iso: None,
            rad: None,
            h: None,
            lab: None,
            lp: None,
        },
        add("C", 9.0, 9.0),
    ])
    .unwrap();
    let edited = s.doc.to_json();
    for kept in [
        r##""ann":{"level":"2b"},"colour":"#c00""##,
        r#""display":"bold""#,
        r#""kind":"equilibrium""#,
        r#""meta":{"tags":["a","b"],"title":"sample"},"style":"acs-1996""#,
    ] {
        assert!(edited.contains(kept), "{kept} in {edited}");
    }
    assert!(
        edited.contains(r#"{"id":8,"el":"C","x":9.0,"y":9.0}"#),
        "a new atom has none"
    );
    // a removed atom takes its unknown fields with it, and undo brings them back in place
    s.run(&[Op::Remove {
        atoms: vec![1],
        bonds: vec![],
        objects: vec![4],
    }])
    .unwrap();
    assert!(!s.doc.to_json().contains("colour") && !s.doc.to_json().contains("equilibrium"));
    assert!(
        s.doc.to_json().contains(r#""style":"acs-1996""#),
        "the document keeps its own"
    );
    s.undo();
    assert_eq!(s.doc.to_json(), edited);
    s.undo();
    assert_eq!(s.doc.to_json(), out);
    // a known field never comes back from an unknown one: a document that repeats a key is read by the known reader alone
    let plain = Document::from_json(
        r#"{"format":"mzdraw/1","next":2,"atoms":[{"id":1,"el":"C","x":0,"y":0}],"bonds":[]}"#,
    )
    .unwrap();
    assert!(plain.more.is_empty() && plain.atoms[0].more.is_empty());
}

// ---- the semantic diff (`diff.rs`): the random sessions above, compared in pairs ----

/// A random session of `steps` more random steps after `from`.
fn evolve(from: &Session, steps: usize, r: &mut Rng) -> Session {
    let mut s = Session::new(from.doc.clone());
    for _ in 0..steps {
        let ops = random_step(&s, r);
        let _ = s.run(&ops);
    }
    s
}

/// The document as the bytes `mzdraw/1`, with the counter `next` taken from `like`: no operation sets the counter.
fn bytes_like(d: &Document, like: &Document) -> String {
    let mut d = d.clone();
    d.next = like.next;
    d.to_json()
}

/// The document with its pools sorted by id and the counter of `like`.
fn by_id(d: &Document, like: &Document) -> String {
    let mut d = d.clone();
    d.next = like.next;
    d.atoms.sort_by_key(|a| a.id);
    d.bonds.sort_by_key(|b| b.id);
    d.objects.sort_by_key(|o| o.id);
    d.to_json()
}

fn pairs() -> Vec<(Document, Document, Rng)> {
    let mut out = Vec::new();
    for seed in 1..=60u64 {
        let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let a = evolve(&Session::default(), 5 + r.next(40), &mut r);
        let b = evolve(&a, 1 + r.next(25), &mut r);
        out.push((a.doc, b.doc, r));
    }
    out
}

#[test]
fn applying_the_diff_gives_the_second_document_and_a_diff_of_equals_is_empty() {
    let mut nonempty = 0;
    for (n, (a, b, _)) in pairs().into_iter().enumerate() {
        assert!(
            mzdraw::diff::diff(&a, &a).is_empty(),
            "pair {n}: {:?}",
            mzdraw::diff::diff(&a, &a)
        );
        let ops = mzdraw::diff::diff(&a, &b);
        nonempty += usize::from(!ops.is_empty());
        let mut s = Session::new(a.clone());
        s.run(&ops)
            .unwrap_or_else(|e| panic!("pair {n}: {} {}", e.key, mzdraw::ops::log_text(&ops)));
        assert_eq!(
            bytes_like(&s.doc, &b),
            b.to_json(),
            "pair {n}: {}",
            mzdraw::ops::log_text(&ops)
        );
        // the patch is the same text twice, and its log parses back to the same operations
        assert_eq!(ops, mzdraw::diff::diff(&a, &b));
        let text = mzdraw::ops::log_text(&ops);
        let mut again = Session::new(a);
        again.replay(&text).unwrap();
        assert_eq!(again.doc.to_json(), s.doc.to_json(), "pair {n}");
        // the SMILES the same, as `mzlab-mol` writes it
        assert_eq!(s.doc.to_smiles(), b.to_smiles(), "pair {n}");
    }
    assert!(nonempty >= 50, "the pairs differ: {nonempty}");
}

#[test]
fn the_patch_is_in_the_fixed_order() {
    let rank = |op: &Op| match op {
        Op::Remove {
            atoms,
            bonds,
            objects,
        } => {
            if !bonds.is_empty() {
                0
            } else if !atoms.is_empty() {
                1
            } else {
                assert!(!objects.is_empty());
                2
            }
        }
        Op::AtomAdd { .. } => 3,
        Op::BondAdd { .. } => 4,
        Op::ObjAdd { .. } => 5,
        Op::AtomSet { .. }
        | Op::AtomData { .. }
        | Op::AtomAnn { .. }
        | Op::AtomTitle { .. }
        | Op::AtomSg { .. } => 6,
        // the colour of an atom is said with the atom, the colour of a bond with the bond
        Op::AtomCap { .. } => 6,
        Op::Colour { atoms, .. } if !atoms.is_empty() => 6,
        Op::Colour { objects, .. } if !objects.is_empty() => 8,
        Op::BondSet { .. } | Op::Colour { .. } => 7,
        Op::ObjSet { .. } => 8,
        Op::Move { .. } => 9,
        Op::DocLabels { .. } => 10,
        other => panic!("not an operation of a diff: {other:?}"),
    };
    for (n, (a, b, _)) in pairs().into_iter().enumerate() {
        let ranks: Vec<i32> = mzdraw::diff::diff(&a, &b).iter().map(rank).collect();
        assert!(
            ranks.windows(2).all(|w| w[0] <= w[1]),
            "pair {n}: {ranks:?}"
        );
    }
}

#[test]
fn any_subset_of_hunks_applies_in_order_to_a_valid_document() {
    let (mut many, mut total) = (0, 0);
    for (n, (a, b, mut r)) in pairs().into_iter().enumerate() {
        let ops = mzdraw::diff::diff(&a, &b);
        let hunks = mzdraw::diff::hunks(&a, &ops);
        // the hunks hold the operations of the patch, split: all of them in order give the second document
        let mut all = Session::new(a.clone());
        for h in &hunks {
            all.run(&h.ops)
                .unwrap_or_else(|e| panic!("pair {n}: {}", e.key));
        }
        // (a hunk adds its things at the end of the pools: the order of two pieces may differ from the patch's)
        assert_eq!(by_id(&all.doc, &b), by_id(&b, &b), "pair {n}: every hunk");
        // each hunk alone, and random subsets
        let mut masks: Vec<Vec<bool>> = (0..hunks.len())
            .map(|i| (0..hunks.len()).map(|j| i == j).collect())
            .collect();
        for _ in 0..6 {
            masks.push((0..hunks.len()).map(|_| r.next(2) == 1).collect());
        }
        for mask in masks {
            let mut s = Session::new(a.clone());
            for (h, _) in hunks.iter().zip(&mask).filter(|&(_, &on)| on) {
                s.run(&h.ops)
                    .unwrap_or_else(|e| panic!("pair {n}: a hunk of a subset: {} {:?}", e.key, h));
            }
            assert!(
                Document::from_json(&s.doc.to_json()).is_ok(),
                "pair {n}: a valid document"
            );
            total += 1;
        }
        many += usize::from(hunks.len() > 1);
    }
    assert!(many >= 20 && total > 200, "{many} pairs with several hunks");
}

#[test]
fn the_diff_says_what_changed_with_the_operations_that_say_it() {
    let mut s = ethanol();
    s.run(&[Op::AtomSet {
        id: 1,
        el: None,
        chg: None,
        iso: None,
        rad: None,
        h: Some(3),
        lab: None,
        map: None,
        lp: None,
    }])
    .unwrap();
    let a = s.doc.clone();
    // a count of hydrogens taken away, a new element, a label, a wedge turned around, an atom moved, a new carbon
    s.run(&[
        Op::AtomSet {
            id: 1,
            el: Some("C".into()),
            chg: None,
            iso: None,
            rad: None,
            h: None,
            lab: None,
            map: None,
            lp: None,
        },
        Op::AtomSet {
            id: 3,
            el: Some("N".into()),
            chg: Some(1),
            iso: None,
            rad: None,
            h: None,
            lab: None,
            map: Some(4),
            lp: None,
        },
        Op::BondSet {
            id: 4,
            o: None,
            s: Some(1),
            pos: None,
            k: None,
            rev: true,
        },
        Op::Move {
            atoms: vec![2],
            objects: vec![],
            dx: 0.25,
            dy: -0.5,
        },
        add("Cl", 3.0, 1.0),
        bond(3, 6, 1),
    ])
    .unwrap();
    let b = s.doc.clone();
    let ops = mzdraw::diff::diff(&a, &b);
    let names: Vec<String> = (ops.iter())
        .map(|o| serde_json::from_str::<serde_json::Value>(&o.to_json()).unwrap()["op"].to_string())
        .collect();
    let names: Vec<&str> = names.iter().map(|n| n.trim_matches('"')).collect();
    assert_eq!(
        names,
        ["atom.add", "bond.add", "atom.set", "atom.set", "bond.set", "move"]
    );
    let mut check = Session::new(a);
    check.run(&ops).unwrap();
    assert_eq!(bytes_like(&check.doc, &b), b.to_json());
    // atoms moved by the same displacement are one move; an arrow moved with its ends is one move too
    let mut s = ethanol();
    s.run(&[Op::ObjAdd {
        id: None,
        t: ObjKind::Arrow,
        x: 4.0,
        y: 0.0,
        x2: 6.0,
        y2: 0.0,
        text: String::new(),
        runs: Vec::new(),
        from: 0,
        to: 0,
        cv: Default::default(),
    }])
    .unwrap();
    let a = s.doc.clone();
    s.run(&[Op::Move {
        atoms: vec![1, 2, 3],
        objects: vec![6],
        dx: 1.0,
        dy: 2.0,
    }])
    .unwrap();
    s.run(&[Op::ObjSet {
        id: 6,
        text: None,
        runs: None,
        x2: Some(9.0),
        y2: None,
        from: None,
        to: None,
        cv: Default::default(),
        pt: [None, None],
    }])
    .unwrap();
    let ops = mzdraw::diff::diff(&a, &s.doc);
    assert!(
        matches!(ops.last(), Some(Op::Move { atoms, objects, .. }) if atoms.len() == 3 && objects == &[6])
    );
    let mut check = Session::new(a);
    check.run(&ops).unwrap();
    assert_eq!(bytes_like(&check.doc, &s.doc), s.doc.to_json());
}

#[test]
fn an_id_that_changes_kind_is_removed_and_added() {
    let a = Document::from_json(r#"{"format":"mzdraw/1","next":4,"atoms":[{"id":1,"el":"C","x":0,"y":0},{"id":2,"el":"C","x":1,"y":0}],"bonds":[{"id":3,"a":1,"b":2,"o":1}]}"#).unwrap();
    // the id 3 was the bond and is an atom now; two new bonds
    let b = Document::from_json(r#"{"format":"mzdraw/1","next":6,"atoms":[{"id":1,"el":"C","x":0,"y":0},{"id":2,"el":"C","x":1,"y":0},{"id":3,"el":"O","x":2,"y":0}],"bonds":[{"id":4,"a":2,"b":3,"o":2},{"id":5,"a":1,"b":3,"o":1}]}"#).unwrap();
    let ops = mzdraw::diff::diff(&a, &b);
    let mut s = Session::new(a.clone());
    s.run(&ops).unwrap();
    assert_eq!(s.doc.to_json(), b.to_json());
    // one hunk: the three things are joined by the bonds
    let hunks = mzdraw::diff::hunks(&a, &ops);
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].ids, [1, 2, 3, 4, 5]);
}

#[test]
fn a_patch_with_an_operation_that_names_no_id_is_one_hunk_and_never_panics() {
    let a = ethanol().doc;
    let arrow = Op::ObjAdd {
        id: None,
        t: ObjKind::Arrow,
        x: 4.0,
        y: 0.0,
        x2: 6.0,
        y2: 0.0,
        text: String::new(),
        runs: Vec::new(),
        from: 0,
        to: 0,
        cv: Default::default(),
    };
    let patches: Vec<Vec<Op>> = vec![
        vec![add("N", 3.0, 3.0)],
        vec![arrow],
        vec![bond(1, 3, 1)],
        vec![Op::Move {
            atoms: vec![],
            objects: vec![],
            dx: 1.0,
            dy: 0.0,
        }],
        vec![Op::Remove {
            atoms: vec![],
            bonds: vec![],
            objects: vec![],
        }],
        // a named op beside one that names nothing: still one hunk, nothing dropped
        vec![
            Op::AtomSet {
                id: 1,
                el: None,
                chg: Some(1),
                iso: None,
                rad: None,
                h: None,
                lab: None,
                map: None,
                lp: None,
            },
            add("N", 3.0, 3.0),
        ],
    ];
    for ops in patches {
        let hunks = mzdraw::diff::hunks(&a, &ops);
        assert_eq!(hunks.len(), 1, "{}", mzdraw::ops::log_text(&ops));
        assert_eq!(hunks[0].ops, ops);
    }
    assert!(mzdraw::diff::hunks(&a, &[]).is_empty());
}

#[test]
fn the_editor_gives_the_patch_to_another_document() {
    let mut e = mzdraw::Editor::default();
    e.import(r#"{"format":"mzdraw/1","next":3,"atoms":[{"id":1,"el":"C","x":0,"y":0},{"id":2,"el":"C","x":1,"y":0}],"bonds":[]}"#)
        .unwrap();
    let other = Document::from_json(r#"{"format":"mzdraw/1","next":4,"atoms":[{"id":1,"el":"C","x":0,"y":0},{"id":2,"el":"O","x":1,"y":0},{"id":3,"el":"N","x":2,"y":0}],"bonds":[]}"#).unwrap();
    let ops = e.diff_with(&other);
    assert_eq!(ops, mzdraw::diff::diff(e.doc(), &other));
    assert_eq!(ops.len(), 2);
    let mut s = Session::new(e.doc().clone());
    s.run(&ops).unwrap();
    assert_eq!(s.doc.to_json(), other.to_json());
    assert!(e.diff_with(e.doc()).is_empty());
}

#[test]
fn a_proposal_accepted_hunk_by_hunk_has_the_bytes_of_apply_of_the_same_hunks_and_each_accept_is_one_undo_step(
) {
    // plan-16: the editor holds the patch of `diff` as a proposal; any order of any subset of its hunks, accepted one by one, gives
    // what `Session::run` of those hunks gives on its own (the reference reads no output of the proposal), and undo returns `a`
    let (mut accepted, mut proposals) = (0, 0);
    for (n, (a, b, mut r)) in pairs().into_iter().enumerate() {
        let ops = mzdraw::diff::diff(&a, &b);
        if ops.is_empty() {
            continue;
        }
        let hunks = mzdraw::diff::hunks(&a, &ops);
        let mut e = mzdraw::Editor::default();
        e.session = Session::new(a.clone());
        assert_eq!(e.propose(ops.clone()).unwrap(), hunks.len(), "pair {n}");
        proposals += 1;
        assert_eq!(e.doc().to_json(), a.to_json(), "pair {n}: held apart");
        let mut order: Vec<usize> = (1..=hunks.len()).filter(|_| r.next(3) != 0).collect();
        // a random order
        for i in (1..order.len()).rev() {
            order.swap(i, r.next(i + 1));
        }
        let mut want = Session::new(a.clone());
        for &h in &order {
            e.accept_hunk(h)
                .unwrap_or_else(|x| panic!("pair {n}: hunk {h}: {}", x.key));
            want.run(&hunks[h - 1].ops).unwrap();
            assert_eq!(
                e.doc().to_json(),
                want.doc.to_json(),
                "pair {n}: after hunk {h}"
            );
        }
        accepted += order.len();
        assert_eq!(
            e.session.steps(),
            order.len(),
            "pair {n}: one undo step per accept"
        );
        for _ in &order {
            assert!(e.undo() != 0);
        }
        assert_eq!(e.doc().to_json(), a.to_json(), "pair {n}: undone");
    }
    assert!(
        proposals >= 50 && accepted > 60,
        "{proposals} proposals, {accepted} accepts"
    );
}

#[test]
fn either_on_a_double_bond_applies_then_undoes_to_the_identical_bytes() {
    // but-2-ene without E/Z (the double bond 6 is "either" on import) and drawn E (no mark): every step on the mark, and on the
    // order that drops it, then its undo, gives back the identical document; redo gives the identical result (rule 11)
    let bond = |o: Option<u8>, s: Option<u8>| Op::BondSet {
        id: 6,
        o,
        s,
        pos: None,
        k: None,
        rev: false,
    };
    for start in ["CC=CC", "C/C=C/C"] {
        let mut e = mzdraw::Editor::default();
        e.import(start).unwrap();
        for step in [
            vec![bond(None, Some(3))],
            vec![bond(None, Some(0))],
            vec![bond(Some(1), None)],
            vec![bond(Some(3), None)],
            vec![bond(Some(1), None), bond(Some(2), None)],
            vec![bond(None, Some(3)), bond(Some(2), Some(0))],
        ] {
            let mut s = Session::new(e.doc().clone());
            let before = s.doc.to_json();
            s.run(&step).unwrap();
            let after = s.doc.to_json();
            assert!(s.undo());
            assert_eq!(s.doc.to_json(), before, "{start} {step:?} undone");
            assert!(s.redo());
            assert_eq!(s.doc.to_json(), after, "{start} {step:?} redone");
        }
    }
}
