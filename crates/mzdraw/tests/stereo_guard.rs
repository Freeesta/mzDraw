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

/// Selects one atom, then drags it by (`dx`, `dy`) document units with the mouse (the move is one step at the release).
fn drag_atom(e: &mut Editor, id: u32, dx: f64, dy: f64) {
    e.set_tool("select");
    let a = e.doc().atom(id).unwrap().clone();
    let (x, y) = e.view.to_px(a.x, a.y);
    let (tx, ty) = e.view.to_px(a.x + dx, a.y + dy);
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_up(x, y);
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_move((x + tx) / 2.0, (y + ty) / 2.0, MOUSE);
    e.pointer_move(tx, ty, MOUSE);
    e.pointer_up(tx, ty);
}

fn id_of(e: &Editor, el: &str, nth: usize) -> u32 {
    e.doc()
        .atoms
        .iter()
        .filter(|a| a.el == el)
        .nth(nth)
        .unwrap()
        .id
}

fn imported(smiles: &str) -> Editor {
    let mut e = Editor::default();
    e.import(smiles).unwrap();
    e
}

#[test]
fn the_signature_is_the_smiles_of_each_piece_with_a_wedge() {
    let mut log = centre();
    log.push(json!({"op":"atom.add","id":8,"el":"C","x":5.0,"y":0.0}));
    let e = editor_with(&log);
    let all = pieces(e.doc());
    assert_eq!(all, vec![vec![1, 2, 3, 4], vec![8]]);
    let sig = signature(e.doc(), &all);
    // the lone carbon holds no wedge and no double bond: only the first piece is signed, in the canonical form
    assert_eq!(sig.len(), 1);
    assert_eq!(sig[0].0, vec![1, 2, 3, 4]);
    assert_eq!(sig[0].1, "[Br][C@@H]([Cl])[F]");
}

#[test]
fn a_plain_flip_of_a_chiral_piece_changes_it_and_the_live_line_says_so() {
    let mut e = editor_with(&centre());
    let before = smiles(&e);
    e.flip(Axis::H);
    assert_ne!(smiles(&e), before);
    assert_eq!(e.said, ("draw2.say.stereoChanged", "1".to_string()));
    // the report stays visible until the next step: Rust keeps it, the page shows it beside the counts
    assert_eq!(e.notice, ("draw2.say.stereoChanged", "1".to_string()));
    assert_eq!(e.error, "");
    e.undo();
    assert_eq!(smiles(&e), before);
    assert_eq!(e.notice.0, "", "an undo clears it");
}

#[test]
fn a_rigid_move_or_turn_says_nothing() {
    let mut e = editor_with(&centre());
    e.rotate(37.0);
    assert_eq!(e.said.0, "draw2.say.rotated");
    e.select_all();
    e.nudge(1.0, 0.5, 0.0);
    assert_eq!(e.said.0, "draw2.announce.moved");
    assert_eq!(e.notice.0, "");
}

#[test]
fn dragging_one_neighbour_across_the_others_changes_it() {
    let mut e = editor_with(&centre());
    let before = smiles(&e);
    drag_atom(&mut e, 3, 1.8, 1.5);
    assert_ne!(smiles(&e), before);
    assert_eq!(e.said, ("draw2.say.stereoChanged", "1".to_string()));
}

/// The keys: `Alt+Shift+Arrow` nudges the selected atoms by a bond length, `Alt+[` turns them; the guard is in their hook too.
#[test]
fn a_key_nudge_or_turn_of_one_neighbour_across_a_centre_says_so() {
    use mzdraw::editor::{ALT, SHIFT};
    let mut e = editor_with(&centre());
    let was = smiles(&e);
    // a nudge of the whole piece: nothing
    e.select_all();
    e.key_at("ArrowRight", ALT | SHIFT, 0.0);
    assert_eq!(e.said.0, "draw2.announce.moved");
    assert_eq!(e.notice.0, "");
    // Cl alone, one bond length to the right twice: across Br (the second press, a second later, is a step of its own)
    e.over.sel_atoms = vec![3];
    e.key_at("ArrowRight", ALT | SHIFT, 5000.0);
    assert_eq!(e.notice.0, "", "not across yet: {:?}", e.said);
    e.key_at("ArrowRight", ALT | SHIFT, 9000.0);
    assert_eq!(e.notice, ("draw2.say.stereoChanged", "1".to_string()));
    assert_ne!(smiles(&e), was);

    // a key turn of two neighbours (Cl and Br) about their middle: a half turn in 15 degree presses swaps them
    let mut e = editor_with(&centre());
    e.over.sel_atoms = vec![3, 4];
    let mut said = false;
    for k in 0..12 {
        e.key_at("[", ALT, 5000.0 * f64::from(k + 1));
        said |= e.notice.0 == "draw2.say.stereoChanged";
    }
    assert!(
        said,
        "the half turn of two neighbours is said by a key turn"
    );
    assert_ne!(smiles(&e), was);
}

#[test]
fn only_the_centres_that_changed_are_named() {
    let mut e = imported("C[C@H](F)[C@H](Cl)Br");
    let f = id_of(&e, "F", 0);
    let (a, b) = {
        let c: Vec<u32> = e
            .doc()
            .bonds
            .iter()
            .filter(|b| b.s != 0)
            .map(|b| b.a)
            .collect();
        (c[0], c[1])
    };
    // F hangs on the first centre: dragged over to the other side of the chain, only that centre is arranged otherwise
    let at = e.doc().atom(f).unwrap().clone();
    let c = e.doc().atom(a).unwrap().clone();
    drag_atom(&mut e, f, 2.0 * (c.x - at.x), 2.0 * (c.y - at.y));
    assert_eq!(e.said.0, "draw2.say.stereoChanged");
    let named: Vec<u32> = e.said.1.split(", ").map(|n| n.parse().unwrap()).collect();
    assert!(
        named.contains(&a) && !named.contains(&b),
        "named {named:?}, centres {a} and {b}"
    );
}

#[test]
fn a_double_bond_moved_across_is_said_with_its_own_atoms() {
    let mut e = imported("C/C=C/C");
    let (c1, c2, c3) = (1, 2, 3);
    let n = e.doc().atom(c1).unwrap().clone();
    let (p, q) = (
        e.doc().atom(c2).unwrap().clone(),
        e.doc().atom(c3).unwrap().clone(),
    );
    // atom 1 goes to the other side of the double bond (mirrored across the line through 2 and 3)
    let (ux, uy) = (q.x - p.x, q.y - p.y);
    let l2 = ux * ux + uy * uy;
    let t = ((n.x - p.x) * ux + (n.y - p.y) * uy) / l2;
    let foot = (p.x + t * ux, p.y + t * uy);
    drag_atom(&mut e, c1, 2.0 * (foot.0 - n.x), 2.0 * (foot.1 - n.y));
    assert_eq!(e.said, ("draw2.say.stereoChanged", "2, 3".to_string()));
    // the same with a wedge elsewhere in the piece: the double bond is named, not the wedge centre
    let mut e = imported("C/C=C/[C@H](F)Cl");
    let wedge_centre = e.doc().bonds.iter().find(|b| b.s != 0).unwrap().a;
    let n = e.doc().atom(1).unwrap().clone();
    let (p, q) = (
        e.doc().atom(2).unwrap().clone(),
        e.doc().atom(3).unwrap().clone(),
    );
    let (ux, uy) = (q.x - p.x, q.y - p.y);
    let t = ((n.x - p.x) * ux + (n.y - p.y) * uy) / (ux * ux + uy * uy);
    let foot = (p.x + t * ux, p.y + t * uy);
    drag_atom(&mut e, 1, 2.0 * (foot.0 - n.x), 2.0 * (foot.1 - n.y));
    assert_eq!(e.said, ("draw2.say.stereoChanged", "2, 3".to_string()));
    assert_ne!(wedge_centre, 2);
}

// ----- the arm `expect` { "stereo": "kept" } -----

fn verdict(request: Value) -> Value {
    serde_json::from_str(&replay(&request.to_string()).unwrap()).unwrap()
}

fn kept(request: Value) -> (bool, String) {
    let v = verdict(request);
    (
        v["ok"].as_bool().unwrap(),
        v["expect"][0]["detail"].as_str().unwrap_or("").to_string(),
    )
}

#[test]
fn the_stereo_arm_is_true_for_a_rigid_log_and_false_for_a_plain_flip() {
    let k = json!([{"stereo":"kept"}]);
    let (ok, _) = kept(json!({"import":"C[C@H](F)Cl","log":[
        {"op":"move","atoms":[1,2,3,4],"dx":2.0,"dy":1.0}],"expect":k}));
    assert!(ok);
    let (ok, detail) = kept(json!({"import":"C[C@H](F)Cl","log":[
        {"op":"flip","atoms":[1,2,3,4],"axis":"h","at":0.0}],"expect":k}));
    assert!(!ok);
    assert_eq!(detail, "the configuration changed at atoms 2");
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

/// The ways round the first version of the arm: a mirrored centre must be said whatever else happens to its piece.
#[test]
fn a_mirror_cannot_be_hidden_by_a_structure_edit() {
    let k = json!([{"stereo":"kept"}]);
    let flip = json!({"op":"flip","atoms":[1,2,3,4],"axis":"h","at":0.0});
    let cases = [
        // a substituent added on the methyl, on the fluorine, on the centre itself; a second piece joined to the first
        (
            "on the methyl",
            json!([flip, {"op":"atom.add","id":100,"el":"O","x":9.0,"y":9.0},{"op":"bond.add","a":1,"b":100,"o":1,"s":0}]),
        ),
        (
            "on the fluorine",
            json!([flip, {"op":"atom.add","id":100,"el":"O","x":9.0,"y":9.0},{"op":"bond.add","a":3,"b":100,"o":1,"s":0}]),
        ),
        (
            "on the centre",
            json!([flip, {"op":"atom.add","id":100,"el":"O","x":9.0,"y":9.0},{"op":"bond.add","a":2,"b":100,"o":1,"s":0}]),
        ),
        (
            "a piece joined",
            json!([flip, {"op":"bond.add","a":1,"b":5,"o":1,"s":0}]),
        ),
        // the methyl removed after the mirror: the centre is not one any more, which is not "kept"
        (
            "the methyl removed",
            json!([flip, {"op":"remove","atoms":[1],"bonds":[],"objects":[]}]),
        ),
    ];
    for (name, log) in cases {
        let (ok, detail) = kept(json!({"import":"C[C@H](F)Cl.O","log":log,"expect":k}));
        assert!(!ok, "{name}: {detail}");
        assert!(detail.contains("atom"), "{name}: {detail}");
    }
    // and the same edits without the mirror keep the configuration
    for (name, log) in [
        (
            "a substituent on the methyl",
            json!([{"op":"atom.add","id":100,"el":"O","x":9.0,"y":9.0},{"op":"bond.add","a":1,"b":100,"o":1,"s":0}]),
        ),
        (
            "a piece joined",
            json!([{"op":"bond.add","a":1,"b":5,"o":1,"s":0}]),
        ),
    ] {
        let (ok, detail) = kept(json!({"import":"C[C@H](F)Cl.O","log":log,"expect":k}));
        assert!(ok, "{name}: {detail}");
    }
}

#[test]
fn a_meso_mirror_is_no_change_and_a_double_bond_move_is() {
    let k = json!([{"stereo":"kept"}]);
    // meso-tartaric acid: its mirror image is the same molecule
    let meso = "O[C@H](C(=O)O)[C@@H](O)C(=O)O";
    let ids: Vec<u32> = (1..=10).collect();
    let (ok, d) = kept(
        json!({"import":meso,"log":[{"op":"flip","atoms":ids,"axis":"h","at":0.0}],"expect":k}),
    );
    assert!(ok, "{d}");
    // the chiral twin is not
    let chiral = "O[C@H](C(=O)O)[C@H](O)C(=O)O";
    let (ok, _) = kept(
        json!({"import":chiral,"log":[{"op":"flip","atoms":ids,"axis":"h","at":0.0}],"expect":k}),
    );
    assert!(!ok);
    // E-butene with a methyl taken across the double bond: atom 1 goes to the mirror point across the line through atoms 2 and 3
    let doc: Value = serde_json::from_str(
        &replay(&json!({"import":"C/C=C/C","out":"mzdraw"}).to_string()).unwrap(),
    )
    .unwrap();
    let xy = |id: u64| {
        let a = doc["atoms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == id)
            .unwrap();
        (a["x"].as_f64().unwrap(), a["y"].as_f64().unwrap())
    };
    let (n, p, q) = (xy(1), xy(2), xy(3));
    let (ux, uy) = (q.0 - p.0, q.1 - p.1);
    let t = ((n.0 - p.0) * ux + (n.1 - p.1) * uy) / (ux * ux + uy * uy);
    let foot = (p.0 + t * ux, p.1 + t * uy);
    let (ok, d) = kept(
        json!({"import":"C/C=C/C","log":[{"op":"move","atoms":[1],"dx":2.0*(foot.0-n.0),"dy":2.0*(foot.1-n.1)}],"expect":k}),
    );
    assert!(!ok);
    assert_eq!(d, "the configuration changed at atoms 2, 3");
    let (ok, _) = kept(
        json!({"import":"C/C=C/C","log":[{"op":"move","atoms":[1,2,3,4],"dx":1.0,"dy":1.0}],"expect":k}),
    );
    assert!(ok);
}

#[test]
fn the_stereo_arm_leaves_a_piece_with_other_atoms_alone() {
    // an edit of the structure that leaves every centre as it is keeps the configuration
    let (ok, d) = kept(json!({"import":"C[C@H](F)Cl","log":[
        {"op":"atom.add","id":100,"el":"O","x":9.0,"y":9.0},
        {"op":"bond.add","a":1,"b":100,"o":1,"s":0}],"expect":[{"stereo":"kept"}]}));
    assert!(ok, "{d}");
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
    fn below(&mut self, n: usize) -> usize {
        ((self.next() * n as f64) as usize).min(n - 1)
    }
}

/// A random chiral piece: a centre with three distinct neighbours around it (one wedge).
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
        let mut e = editor_with(&random_piece(&mut r));
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
        assert!(mzdraw::stereo_guard::before(e.doc(), std::slice::from_ref(&op)).is_none());
        e.session.run(&[op]).unwrap();
        assert_eq!(signature(e.doc(), &pieces(e.doc())), before);
    }
}

/// Molecules with one or two centres, a ring, a double bond, a meso compound: the SMILES has `@` and `@@` drawn at random.
fn template(r: &mut Rng) -> String {
    let at = |r: &mut Rng| if r.next() < 0.5 { "@" } else { "@@" };
    let (a, b) = (at(r), at(r));
    match r.below(8) {
        0 => format!("C[C{a}H](F)Cl"),
        1 => format!("C[C{a}H](F)[C{b}H](Cl)Br"),
        2 => format!("C[C{a}H]1CC[C{b}H](O)CC1"),
        3 => format!("O[C{a}H](C(=O)O)[C{b}H](O)C(=O)O"),
        4 => "C/C=C/C".into(),
        5 => "F/C=C\\Cl".into(),
        6 => format!("C/C=C/[C{a}H](F)Cl"),
        _ => format!("N[C{a}H](C)C(=O)O"),
    }
}

/// 1,000 random drawings, 5 geometry edits each (a flip of the whole piece or of part of it, a rigid move, a rigid turn, a drag of
/// one atom). The guard must say what the canonical signature of the piece says: every edit that changes the molecule, and none of
/// the rigid ones. The rates are printed.
#[test]
fn kill_test_one_thousand_documents_five_geometry_edits_each() {
    let mut r = Rng(2026);
    let (mut n, mut flips, mut flips_flagged) = (0, 0, 0);
    let (mut other, mut other_changed, mut other_flagged, mut rigid, mut rigid_flagged) =
        (0, 0, 0, 0, 0);
    let mut disagree = Vec::new();
    for _ in 0..1000 {
        let smi = template(&mut r);
        let mut e = imported(&smi);
        for _ in 0..5 {
            n += 1;
            let ids: Vec<u32> = e.doc().atoms.iter().map(|a| a.id).collect();
            let was = signature(e.doc(), &pieces(e.doc()));
            let kind = r.below(5);
            let pick: Vec<u32> = if kind == 1 {
                ids.iter().copied().filter(|_| r.next() < 0.5).collect()
            } else {
                ids.clone()
            };
            let op = match kind {
                0 => Op::Flip {
                    atoms: ids,
                    axis: if r.next() < 0.5 { Axis::H } else { Axis::V },
                    at: r.next() * 4.0 - 2.0,
                },
                1 => Op::Flip {
                    atoms: pick,
                    axis: Axis::H,
                    at: r.next() * 4.0 - 2.0,
                },
                2 => Op::Move {
                    atoms: ids,
                    objects: vec![],
                    dx: r.next() * 6.0 - 3.0,
                    dy: r.next() * 6.0 - 3.0,
                },
                3 => Op::Rotate {
                    atoms: ids,
                    deg: r.next() * 360.0,
                    x: r.next(),
                    y: r.next(),
                },
                _ => Op::Move {
                    atoms: vec![ids[r.below(ids.len())]],
                    objects: vec![],
                    dx: r.next() * 4.0 - 2.0,
                    dy: r.next() * 4.0 - 2.0,
                },
            };
            if matches!(&op, Op::Flip { atoms, .. } | Op::Move { atoms, .. } if atoms.is_empty()) {
                continue;
            }
            let guard = mzdraw::stereo_guard::before(e.doc(), std::slice::from_ref(&op));
            e.session.run(std::slice::from_ref(&op)).unwrap();
            let flag = guard
                .as_ref()
                .is_some_and(|g| !mzdraw::stereo_guard::changed(e.doc(), g, false).is_empty());
            let now = signature(e.doc(), &pieces(e.doc()));
            let changed = was != now;
            match kind {
                0 => {
                    flips += 1;
                    flips_flagged += usize::from(flag);
                    if flag != changed {
                        disagree.push((smi.clone(), kind));
                    }
                }
                2 | 3 => {
                    rigid += 1;
                    rigid_flagged += usize::from(flag);
                }
                _ => {
                    other += 1;
                    other_changed += usize::from(changed);
                    other_flagged += usize::from(flag);
                    if flag != changed {
                        disagree.push((smi.clone(), kind));
                    }
                }
            }
        }
    }
    println!(
        "edits {n}: whole flips {flips} (flagged {flips_flagged}); partial flips and drags {other} (molecule changed {other_changed}, flagged {other_flagged}); rigid {rigid} (flagged {rigid_flagged}); disagreements with the canonical signature {}",
        disagree.len()
    );
    assert_eq!(rigid_flagged, 0, "no rigid move or turn is flagged");
    assert!(
        disagree.is_empty(),
        "{:?}",
        &disagree[..disagree.len().min(5)]
    );
    assert!(
        other_changed > 100,
        "the generator must produce changes: {other_changed}"
    );
}

/// The floor under the kill test: the plain flip of a fresh chiral, non-meso drawing (one centre, two centres, a centre with a
/// double bond, an amino acid; `@` or `@@` at random) is the other enantiomer, and the guard must say it every time.
#[test]
fn a_plain_flip_of_a_fresh_chiral_drawing_is_always_flagged() {
    let mut r = Rng(11);
    let mut n = 0;
    for _ in 0..1000 {
        let at = |r: &mut Rng| if r.next() < 0.5 { "@" } else { "@@" };
        let (a, b) = (at(&mut r), at(&mut r));
        let smi = match r.below(4) {
            0 => format!("C[C{a}H](F)Cl"),
            1 => format!("C[C{a}H](F)[C{b}H](Cl)Br"),
            2 => format!("C/C=C/[C{a}H](F)Cl"),
            _ => format!("N[C{a}H](C)C(=O)O"),
        };
        let mut e = imported(&smi);
        let ids: Vec<u32> = e.doc().atoms.iter().map(|a| a.id).collect();
        let op = Op::Flip {
            atoms: ids,
            axis: if r.next() < 0.5 { Axis::H } else { Axis::V },
            at: r.next() * 4.0 - 2.0,
        };
        let guard = mzdraw::stereo_guard::before(e.doc(), std::slice::from_ref(&op))
            .expect("a chiral drawing has elements");
        e.session.run(std::slice::from_ref(&op)).unwrap();
        assert!(
            !mzdraw::stereo_guard::changed(e.doc(), &guard, false).is_empty(),
            "{smi}"
        );
        n += 1;
    }
    assert!(n >= 1000);
}

/// Separate pieces cost in proportion to their number: k chiral molecules, all mirrored by one step, take k times one (a quadratic
/// version took 1.3 s for 4,000 in a release build).
#[test]
fn the_guard_scales_with_the_number_of_pieces() {
    let time = |k: u32| {
        let mut s = Session::default();
        let mut atoms = Vec::new();
        let mut bonds = Vec::new();
        for i in 0..k {
            let (x, y) = (f64::from(i % 50) * 4.0, f64::from(i / 50) * 4.0);
            for (el, dx, dy) in [
                ("C", 0.0, 0.0),
                ("F", 0.0, 1.0),
                ("Cl", -0.9, -0.5),
                ("Br", 0.9, -0.5),
            ] {
                atoms.push(Op::AtomAdd {
                    id: None,
                    el: el.into(),
                    x: x + dx,
                    y: y + dy,
                    map: None,
                });
            }
            let c = i * 4 + 1;
            bonds.push(Op::BondAdd {
                id: None,
                a: c,
                b: c + 1,
                o: 1,
                s: 1,
            });
            bonds.push(Op::BondAdd {
                id: None,
                a: c,
                b: c + 2,
                o: 1,
                s: 0,
            });
            bonds.push(Op::BondAdd {
                id: None,
                a: c,
                b: c + 3,
                o: 1,
                s: 0,
            });
        }
        s.run(&atoms).unwrap();
        s.run(&bonds).unwrap();
        let ids: Vec<u32> = s.doc.atoms.iter().map(|a| a.id).collect();
        let op = Op::Flip {
            atoms: ids,
            axis: Axis::H,
            at: 100.0,
        };
        let t = std::time::Instant::now();
        let guard = mzdraw::stereo_guard::before(&s.doc, std::slice::from_ref(&op)).unwrap();
        s.run(std::slice::from_ref(&op)).unwrap();
        let named = mzdraw::stereo_guard::changed(&s.doc, &guard, false);
        assert_eq!(named.len(), k as usize, "every centre is said");
        t.elapsed().as_secs_f64() * 1000.0
    };
    let (small, big) = (time(500), time(2000));
    println!("guarded mirror of 500 pieces: {small:.1} ms, of 2000: {big:.1} ms");
    // linear is 4 times, quadratic 16 times; 9 leaves room for noise
    assert!(
        big < 9.0 * small.max(1.0),
        "{small} ms for 500, {big} ms for 2000"
    );
    if !cfg!(debug_assertions) {
        assert!(big < 600.0, "{big} ms for 2000 pieces in a release build");
    }
}

// ----- the budget -----

/// A piece of 100 atoms (a chain with 10 wedged branches): one signature, median of 20. The bound is generous (the test build is not
/// optimised); `examples/stereo_bench.rs` prints the figure of the release build.
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

/// A step on a big drawing with no wedge and no double bond costs one pass over its bonds, whatever the number of atoms.
#[test]
fn a_geometry_step_on_a_big_wedge_free_drawing_is_cheap() {
    let mut s = Session::default();
    let atoms: Vec<Op> = (0..20_000)
        .map(|i| Op::AtomAdd {
            id: None,
            el: "C".into(),
            x: i as f64,
            y: 0.0,
            map: None,
        })
        .collect();
    s.run(&atoms).unwrap();
    let bonds: Vec<Op> = (1..20_000u32)
        .map(|i| Op::BondAdd {
            id: None,
            a: i,
            b: i + 1,
            o: 1,
            s: 0,
        })
        .collect();
    s.run(&bonds).unwrap();
    let op = Op::Move {
        atoms: vec![5, 6],
        objects: vec![],
        dx: 0.5,
        dy: 0.5,
    };
    let t = std::time::Instant::now();
    for _ in 0..20 {
        assert!(mzdraw::stereo_guard::before(&s.doc, std::slice::from_ref(&op)).is_none());
    }
    let ms = t.elapsed().as_secs_f64() * 1000.0 / 20.0;
    println!("before() on 20,000 atoms without wedge: {ms:.3} ms (test build)");
    assert!(ms < 5.0, "{ms} ms");
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
    let f = |v: &Value, k: &str| v[k].as_f64().unwrap();
    let u = |v: &Value, k: &str| v[k].as_u64().unwrap();
    let mut seen = 0;
    for line in set.lines().filter(|l| l.contains(['@', '/', '\\'])) {
        let (smi, name) = line.split_once(' ').unwrap();
        let start: Value = serde_json::from_str(
            &replay(&json!({"import":smi,"out":"mzdraw"}).to_string()).unwrap(),
        )
        .unwrap();
        let atoms = start["atoms"].as_array().unwrap();
        let bonds = start["bonds"].as_array().unwrap();
        let at = |id: u64| atoms.iter().find(|a| a["id"] == id).unwrap().clone();
        let ids: Vec<u64> = atoms.iter().map(|a| u(a, "id")).collect();
        let mut drag = Vec::new();
        let mut centres: Vec<u64> = bonds
            .iter()
            .filter(|b| b["s"].as_u64().unwrap_or(0) != 0)
            .map(|b| u(b, "a"))
            .collect();
        centres.sort_unstable();
        centres.dedup();
        for c in centres {
            let n = bonds
                .iter()
                .find(|b| (b["a"] == c || b["b"] == c) && b["s"].as_u64().unwrap_or(0) == 0)
                .map(|b| if b["a"] == c { u(b, "b") } else { u(b, "a") });
            if let Some(n) = n {
                let (p, q) = (at(c), at(n));
                drag.push(json!({"op":"move","atoms":[n],"dx":2.0*(f(&p,"x")-f(&q,"x")),"dy":2.0*(f(&p,"y")-f(&q,"y"))}));
            }
        }
        for b in bonds.iter().filter(|b| b["o"].as_u64() == Some(2)) {
            let (ba, bb, bid) = (u(b, "a"), u(b, "b"), &b["id"]);
            let other = |end: u64| {
                bonds
                    .iter()
                    .find(|x| (u(x, "a") == end || u(x, "b") == end) && &x["id"] != bid)
            };
            let (Some(x), Some(_)) = (other(ba), other(bb)) else {
                continue;
            };
            let n = if u(x, "a") == ba {
                u(x, "b")
            } else {
                u(x, "a")
            };
            let (p, q, r) = (at(ba), at(bb), at(n));
            let (ux, uy) = (f(&q, "x") - f(&p, "x"), f(&q, "y") - f(&p, "y"));
            let t = ((f(&r, "x") - f(&p, "x")) * ux + (f(&r, "y") - f(&p, "y")) * uy)
                / (ux * ux + uy * uy);
            let (fx, fy) = (f(&p, "x") + t * ux, f(&p, "y") + t * uy);
            drag.push(
                json!({"op":"move","atoms":[n],"dx":2.0*(fx-f(&r,"x")),"dy":2.0*(fy-f(&r,"y"))}),
            );
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
