//! The stereo guard (plan-21): the signature of a piece, the live line after a geometry step, the `expect` arm `stereo`, the
//! property that a rigid move or turn of a whole piece never changes a configuration, and the kill test of the plan.

use mzdraw::editor::MOUSE;
use mzdraw::replay::replay;
use mzdraw::stereo_guard::{pieces, signature};
use mzdraw::{Axis, Document, Editor, Op, Session};
use serde_json::{json, Value};

/// A chiral centre drawn with a wedge: C with F (wedge), Cl and Br, one hydrogen implied.
fn centre() -> Vec<Value> {
    vec![
        json!({"op":"atom.add","el":"C","x":0.0,"y":0.0}),
        json!({"op":"atom.add","el":"F","x":0.0,"y":1.0}),
        json!({"op":"atom.add","el":"Cl","x":-0.9,"y":-0.5}),
        json!({"op":"atom.add","el":"Br","x":0.9,"y":-0.5}),
        json!({"op":"bond.add","a":1,"b":2,"o":1,"s":1}),
        json!({"op":"bond.add","a":1,"b":3,"o":1,"s":0}),
        json!({"op":"bond.add","a":1,"b":4,"o":1,"s":0}),
    ]
}

fn editor_with(log: &[Value]) -> Editor {
    let mut e = Editor::default();
    let req = json!({"log":log,"out":"mzdraw"}).to_string();
    e.import(&replay(&req).unwrap()).unwrap();
    e
}

fn smiles(e: &Editor) -> String {
    e.doc().to_smiles().unwrap()
}

#[test]
fn the_signature_is_the_smiles_of_each_piece_with_a_wedge() {
    let mut log = centre();
    log.push(json!({"op":"atom.add","id":8,"el":"C","x":5.0,"y":0.0}));
    let e = editor_with(&log);
    let all = pieces(e.doc());
    assert_eq!(all, vec![vec![1, 2, 3, 4], vec![8]]);
    let sig = signature(e.doc(), &all);
    // the lone carbon holds no wedge: only the first piece is signed, and in the canonical form of the drawing
    assert_eq!(sig.len(), 1);
    assert_eq!(sig[0].0, vec![1, 2, 3, 4]);
    assert_eq!(sig[0].1, "[Br][C@@H]([Cl])[F]");
    let only = signature(e.doc(), &all[..1]);
    assert_eq!(only, sig);
}

#[test]
fn a_plain_flip_of_a_chiral_piece_changes_it_and_the_live_line_says_so() {
    let mut e = editor_with(&centre());
    let before = smiles(&e);
    e.flip(Axis::H);
    assert_ne!(smiles(&e), before);
    assert_eq!(e.said, ("draw2.say.stereoChanged", "1".to_string()));
    // report only: the step stands, and an undo brings the configuration back
    assert_eq!(e.error, "");
    e.undo();
    assert_eq!(smiles(&e), before);
}

#[test]
fn a_rigid_move_or_turn_says_nothing_about_the_configuration() {
    let mut e = editor_with(&centre());
    e.rotate(37.0);
    assert_eq!(e.said.0, "draw2.say.rotated");
    e.select_all();
    e.nudge(1.0, 0.5, 0.0);
    assert_eq!(e.said.0, "draw2.announce.moved");
}

#[test]
fn dragging_one_neighbour_across_the_others_changes_it() {
    let mut e = editor_with(&centre());
    let before = smiles(&e);
    // Cl goes over to the other side of F: the order of the three neighbours around the centre turns the other way
    e.set_tool("select");
    let (x, y) = {
        let a = e.doc().atom(3).unwrap();
        e.view.to_px(a.x, a.y)
    };
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_up(x, y);
    let (tx, ty) = {
        let a = e.doc().atom(3).unwrap();
        e.view.to_px(a.x + 1.8, a.y + 1.5)
    };
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_move((x + tx) / 2.0, (y + ty) / 2.0, MOUSE);
    e.pointer_move(tx, ty, MOUSE);
    e.pointer_up(tx, ty);
    assert_ne!(smiles(&e), before);
    assert_eq!(e.said, ("draw2.say.stereoChanged", "1".to_string()));
}

// ----- the arm `expect` { "stereo": "kept" } -----

fn verdict(request: Value) -> Value {
    serde_json::from_str(&replay(&request.to_string()).unwrap()).unwrap()
}

#[test]
fn the_stereo_arm_is_true_for_a_rigid_log_and_false_for_a_plain_flip() {
    let kept = verdict(json!({"import":"C[C@H](F)Cl","log":[
        {"op":"move","atoms":[1,2,3,4],"dx":2.0,"dy":1.0}],"expect":[{"stereo":"kept"}]}));
    assert_eq!(kept["ok"], true);
    let flipped = verdict(json!({"import":"C[C@H](F)Cl","log":[
        {"op":"flip","atoms":[1,2,3,4],"axis":"h","at":0.0}],"expect":[{"stereo":"kept"}]}));
    assert_eq!(flipped["ok"], false);
    let detail = flipped["expect"][0]["detail"].as_str().unwrap();
    assert!(detail.contains("atoms 2 changed"), "{detail}");
    assert!(
        detail.contains("[C@H]") && detail.contains("[C@@H]"),
        "{detail}"
    );
}

#[test]
fn the_stereo_arm_needs_the_value_kept() {
    for bad in [
        json!({"stereo":"lost"}),
        json!({"stereo":1}),
        json!({"stereo":true}),
    ] {
        let r = replay(&json!({"import":"CC","expect":[bad]}).to_string());
        assert_eq!(r.unwrap_err().key, "error.draw2.expect");
    }
}

#[test]
fn the_stereo_arm_leaves_a_piece_with_other_atoms_alone() {
    // an edit of the structure changes the SMILES legitimately: the arm compares only the pieces that keep their atoms
    let r = verdict(json!({"import":"C[C@H](F)Cl","log":[
        {"op":"atom.add","id":100,"el":"O","x":9.0,"y":9.0},
        {"op":"bond.add","a":1,"b":100,"o":1,"s":0}],"expect":[{"stereo":"kept"}]}));
    assert_eq!(r["ok"], true);
}

// ----- random documents -----

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / (1u64 << 31) as f64
    }
}

/// A random chiral piece: a centre with four distinct neighbours around it (drawn spread out, one wedge), plus a tail.
fn random_piece(r: &mut Rng) -> Vec<Value> {
    let els = ["F", "Cl", "Br", "I", "O", "N", "S"];
    let mut log = vec![json!({"op":"atom.add","el":"C","x":0.0,"y":0.0})];
    let start = r.next() * 360.0;
    for k in 0..3 {
        let ang = (start + 120.0 * k as f64 + r.next() * 30.0).to_radians();
        let el = els[(k * 2 + (r.next() * 2.0) as usize) % els.len()];
        log.push(json!({"op":"atom.add","el":el,"x":ang.cos(),"y":ang.sin()}));
    }
    for k in 0..3u32 {
        log.push(json!({"op":"bond.add","a":1,"b":k+2,"o":1,"s":if k == 0 {1 + (r.next() * 2.0) as u32} else {0}}));
    }
    log
}

#[test]
fn a_rigid_move_or_turn_of_a_whole_piece_never_changes_the_signature() {
    let mut r = Rng(7);
    for _ in 0..200 {
        let log = random_piece(&mut r);
        let mut e = editor_with(&log);
        let before = signature(e.doc(), &pieces(e.doc()));
        let all: Vec<u32> = e.doc().atoms.iter().map(|a| a.id).collect();
        let op = if r.next() < 0.5 {
            Op::Move {
                atoms: all,
                objects: vec![],
                dx: r.next() * 20.0 - 10.0,
                dy: r.next() * 20.0 - 10.0,
            }
        } else {
            Op::Rotate {
                atoms: all,
                deg: r.next() * 360.0,
                x: r.next() * 4.0,
                y: r.next() * 4.0,
            }
        };
        e.session.run(&[op]).unwrap();
        assert_eq!(signature(e.doc(), &pieces(e.doc())), before);
    }
}

#[test]
fn kill_test_one_thousand_documents_five_geometry_edits_each() {
    let mut r = Rng(2026);
    let (mut flips, mut flagged, mut rigid, mut rigid_flagged) = (0, 0, 0, 0);
    for _ in 0..1000 {
        let mut e = editor_with(&random_piece(&mut r));
        for _ in 0..5 {
            let all: Vec<u32> = e.doc().atoms.iter().map(|a| a.id).collect();
            let was = signature(e.doc(), &pieces(e.doc()));
            let kind = (r.next() * 3.0) as u32;
            let op = match kind {
                0 => Op::Flip {
                    atoms: all,
                    axis: if r.next() < 0.5 { Axis::H } else { Axis::V },
                    at: r.next() * 4.0 - 2.0,
                },
                1 => Op::Move {
                    atoms: all,
                    objects: vec![],
                    dx: r.next() * 6.0 - 3.0,
                    dy: r.next() * 6.0 - 3.0,
                },
                _ => Op::Rotate {
                    atoms: all,
                    deg: r.next() * 360.0,
                    x: r.next(),
                    y: r.next(),
                },
            };
            let guard = mzdraw::stereo_guard::before(e.doc(), std::slice::from_ref(&op)).unwrap();
            assert_eq!(guard, was);
            e.session.run(std::slice::from_ref(&op)).unwrap();
            let flag = !mzdraw::stereo_guard::changed(e.doc(), &guard).is_empty();
            if kind == 0 {
                flips += 1;
                flagged += usize::from(flag);
            } else {
                rigid += 1;
                rigid_flagged += usize::from(flag);
            }
        }
    }
    println!("flips {flips}, flagged {flagged}; rigid {rigid}, flagged {rigid_flagged}");
    assert_eq!(
        flagged, flips,
        "every plain flip of a chiral piece is flagged"
    );
    assert_eq!(rigid_flagged, 0, "no rigid move or turn is flagged");
}

// ----- the budget -----

/// A piece of 100 atoms (a chain with 10 wedged branches): the time of one signature, median of 20. The bound is generous (the test
/// build is not optimised); `examples/stereo_bench.rs` prints the figure of the release build.
#[test]
fn a_signature_of_a_100_atom_piece_with_10_wedges_is_fast() {
    let d = big_piece();
    let all = pieces(&d);
    assert_eq!(all.len(), 1);
    assert_eq!(d.atoms.len(), 100);
    let mut ms: Vec<f64> = (0..20)
        .map(|_| {
            let t = std::time::Instant::now();
            let s = signature(&d, &all);
            assert_eq!(s.len(), 1);
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    ms.sort_by(f64::total_cmp);
    println!("median {:.3} ms (test build)", ms[10]);
    assert!(ms[10] < 100.0);
}

pub fn big_piece() -> Document {
    let mut s = Session::default();
    let mut ops = Vec::new();
    for i in 0..50 {
        ops.push(Op::AtomAdd {
            id: None,
            el: "C".into(),
            x: i as f64,
            y: 0.0,
            map: None,
        });
    }
    for i in 0..50u32 {
        let (y, el) = (
            if i % 2 == 0 { 1.0 } else { -1.0 },
            if i % 5 == 0 { "F" } else { "O" },
        );
        ops.push(Op::AtomAdd {
            id: None,
            el: el.into(),
            x: i as f64,
            y,
            map: None,
        });
    }
    s.run(&ops).unwrap();
    let mut bonds = Vec::new();
    for i in 1..50u32 {
        bonds.push(Op::BondAdd {
            id: None,
            a: i,
            b: i + 1,
            o: 1,
            s: 0,
        });
    }
    for i in 0..50u32 {
        let wedge = if i % 5 == 0 { 1 } else { 0 };
        bonds.push(Op::BondAdd {
            id: None,
            a: i + 1,
            b: 51 + i,
            o: 1,
            s: wedge,
        });
    }
    s.run(&bonds).unwrap();
    s.doc
}

// ----- the independent proof, kept honest without Python -----

/// `tests/data/fidelity/stereo.json` (written by `tools/fidelity/stereo.py`, RDKit's InChI stereo layer): the same edits are replayed
/// here and our verdict must be the one the file recorded, which RDKit agreed with. Without this the file could go stale.
#[test]
fn the_reference_of_the_independent_proof_is_the_verdict_of_this_build() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/fidelity");
    let reference: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("stereo.json")).unwrap()).unwrap();
    assert_eq!(reference["cases"], reference["agree"]);
    let set = std::fs::read_to_string(root.join("set.smi")).unwrap();
    let mut seen = 0;
    for line in set.lines().filter(|l| l.contains('@')) {
        let (smi, name) = line.split_once(' ').unwrap();
        let start: Value = serde_json::from_str(
            &replay(&json!({"import":smi,"out":"mzdraw"}).to_string()).unwrap(),
        )
        .unwrap();
        let ids: Vec<u64> = start["atoms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["id"].as_u64().unwrap())
            .collect();
        let at = |id: u64| {
            start["atoms"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["id"] == id)
                .unwrap()
                .clone()
        };
        let bonds = start["bonds"].as_array().unwrap();
        let mut drag = Vec::new();
        let mut centres: Vec<u64> = bonds
            .iter()
            .filter(|b| b["s"].as_u64().unwrap_or(0) != 0)
            .map(|b| b["a"].as_u64().unwrap())
            .collect();
        centres.sort_unstable();
        centres.dedup();
        for c in centres {
            let n = bonds
                .iter()
                .find(|b| (b["a"] == c || b["b"] == c) && b["s"].as_u64().unwrap_or(0) == 0)
                .map(|b| {
                    if b["a"] == c {
                        b["b"].as_u64().unwrap()
                    } else {
                        b["a"].as_u64().unwrap()
                    }
                });
            if let Some(n) = n {
                let (p, q) = (at(c), at(n));
                drag.push(json!({"op":"move","atoms":[n],"dx":2.0*(p["x"].as_f64().unwrap()-q["x"].as_f64().unwrap()),"dy":2.0*(p["y"].as_f64().unwrap()-q["y"].as_f64().unwrap())}));
            }
        }
        for (edit, log) in [
            (
                "flip",
                vec![json!({"op":"flip","atoms":ids,"axis":"h","at":0.0})],
            ),
            ("drag", drag),
        ] {
            let r = verdict(json!({"import":smi,"log":log,"expect":[{"stereo":"kept"}]}));
            let case = reference["results"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == name && c["edit"] == edit)
                .unwrap();
            assert_eq!(
                case["ours_changed"],
                json!(r["ok"] == false),
                "{name} {edit}"
            );
            assert_eq!(case["agree"], true);
            seen += 1;
        }
    }
    assert_eq!(seen, reference["cases"]);
}
