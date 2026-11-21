//! Derive a transformation product (tp-03): the copy with its exact coordinates and maps, the patch on the parent's ids, the routed
//! arrow, the refusals, the undo and the speed. Synthetic structures only.

use mzdraw::{Document, Editor};
use serde_json::{json, Value};

#[path = "support/timing.rs"]
mod timing;

/// N-methyl caprolactam drawn as a hexagon: N1, C2 (C=O, O 8), C3..C6; methyl 7 on N. Atom ids 1..8, bond ids 9..16, next 17.
fn lactam(dx: f64) -> Value {
    let p = |k: f64| {
        (
            dx + (k * 60f64.to_radians()).cos(),
            (k * 60f64.to_radians()).sin(),
        )
    };
    let ring: Vec<(f64, f64)> = (0..6).map(|k| p(k as f64)).collect();
    let mut atoms = Vec::new();
    for (i, el) in ["N", "C", "C", "C", "C", "C"].iter().enumerate() {
        atoms.push(json!({"id": i + 1, "el": el, "x": ring[i].0, "y": ring[i].1}));
    }
    atoms.push(json!({"id":7,"el":"C","x":dx+2.0,"y":0.0}));
    atoms.push(json!({"id":8,"el":"O","x":dx+0.5,"y":1.7320508}));
    let mut bonds = Vec::new();
    for i in 0..6 {
        bonds.push(json!({"id": 9 + i, "a": i + 1, "b": (i + 1) % 6 + 1, "o": 1}));
    }
    bonds.push(json!({"id":15,"a":1,"b":7,"o":1}));
    bonds.push(json!({"id":16,"a":2,"b":8,"o":2}));
    json!({"format":"mzdraw/1","next":17,"atoms":atoms,"bonds":bonds})
}

/// A chain of `n` carbons, atom ids 1..=n.
fn chain(n: u32) -> String {
    let atoms: Vec<Value> = (1..=n)
        .map(|i| json!({"id": i, "el": "C", "x": i as f64, "y": 0.0}))
        .collect();
    let bonds: Vec<Value> = (1..n)
        .map(|i| json!({"id": n + i, "a": i, "b": i + 1, "o": 1}))
        .collect();
    json!({"format":"mzdraw/1","next":2 * n + 1,"atoms":atoms,"bonds":bonds}).to_string()
}

fn replay(request: Value) -> Value {
    serde_json::from_str(&mzdraw::replay::replay(&request.to_string()).unwrap()).unwrap()
}

fn derive(of: u32, patch: Value) -> Value {
    json!({"command":"tp:derive","args":{"of":of,"patch":patch}})
}

fn hydroxylate() -> Value {
    json!([{"op":"atom.add","el":"O","x":-1.0,"y":-1.7321},{"op":"bond.add","a":5,"b":-1,"o":1}])
}

fn doc_of(r: &Value) -> Document {
    Document::from_json(r["out"].as_str().unwrap()).unwrap()
}

/// The atom of the structure `k` (in the order of `layout::pieces`) that carries the map `m`.
fn mapped(d: &Document, k: usize, m: u16) -> &mzdraw::doc::Atom {
    let piece = &mzdraw::layout::pieces(d)[k];
    (piece.iter().filter_map(|&i| d.atom(i)))
        .find(|a| a.map == m)
        .unwrap()
}

fn run(commands: Value) -> Document {
    let r = replay(
        json!({"import": lactam(0.0).to_string(), "commands": commands, "report": true, "out": "mzdraw"}),
    );
    assert_eq!(r["ok"], true, "{r}");
    doc_of(&r)
}

#[test]
fn the_copy_is_a_translation_with_the_parent_maps_and_one_routed_arrow() {
    let d = run(json!([derive(1, json!([]))]));
    // the parent got 1..8 in id order, the copy carries the same numbers
    for id in 1..=8u32 {
        assert_eq!(d.atom(id).unwrap().map, id as u16);
    }
    assert_eq!(d.atoms.len(), 16);
    let (dx, dy) = {
        let (p, c) = (d.atom(1).unwrap(), d.atom(17).unwrap());
        (c.x - p.x, c.y - p.y)
    };
    assert!(dx > 2.5, "one column to the right, {dx}");
    assert!(dy.abs() < 1e-3, "at the parent's height");
    for id in 1..=8u32 {
        let (p, c) = (d.atom(id).unwrap(), d.atom(16 + id).unwrap());
        assert!((c.x - p.x - dx).abs() < 3e-4 && (c.y - p.y - dy).abs() < 3e-4);
        assert_eq!((c.map, &c.el), (p.map, &p.el));
    }
    assert_eq!(d.bonds.len(), 16);
    let arrow = &d.objects[0];
    assert_eq!((arrow.from, arrow.to), (1, 17));
    assert!(arrow.x > 1.0 && arrow.x2 < dx, "routed between the boxes");
}

#[test]
fn an_empty_patch_gives_the_same_canonical_smiles() {
    let d = run(json!([derive(1, json!([]))]));
    let e = |ids: std::ops::RangeInclusive<u32>| {
        let one = Document {
            next: d.next,
            atoms: (d.atoms.iter())
                .filter(|a| ids.contains(&a.id))
                .cloned()
                .collect(),
            bonds: (d.bonds.iter())
                .filter(|b| ids.contains(&b.a))
                .cloned()
                .collect(),
            ..Document::default()
        };
        one.to_smiles().unwrap()
    };
    assert_eq!(e(1..=8), e(17..=24));
}

#[test]
fn hydroxylation_demethylation_and_ring_opening_keep_the_scaffold() {
    let d = run(json!([
        derive(1, hydroxylate()),
        derive(1, json!([{"op":"remove","atoms":[7],"bonds":[15]}])),
        derive(
            1,
            json!([
                {"op":"remove","atoms":[],"bonds":[9]},
                {"op":"atom.add","el":"O","x":1.366,"y":1.366},
                {"op":"bond.add","a":2,"b":-1,"o":1}
            ])
        ),
    ]));
    // three copies and three arrows; the unchanged atoms of each copy are the parent's, translated
    assert_eq!(d.objects.len(), 3);
    assert_eq!(d.atoms.len(), 8 + 9 + 7 + 9);
    // the three TPs do not overlap: the second and third go below the first
    let ys: Vec<f64> = (1..=3).map(|k| mapped(&d, k, 1).y).collect();
    assert!(ys[0] > ys[1] && ys[1] > ys[2], "{ys:?}");
    // the new oxygen of the first TP is mapped after the parent's 8 atoms
    let top = |k: usize| {
        mzdraw::layout::pieces(&d)[k]
            .iter()
            .filter_map(|&i| d.atom(i))
            .map(|a| a.map)
            .max()
            .unwrap()
    };
    // a map names one atom across the pathway: 1..8 for the parent and in every TP; the new atoms of TP1 and TP3 get 9 and 10
    assert_eq!((top(0), top(1), top(2), top(3)), (8, 9, 8, 10));
}

#[test]
fn a_tp_can_be_the_parent_of_the_next_one() {
    let d = run(json!([
        derive(1, hydroxylate()),
        derive(17, json!([{"op":"remove","atoms":[23],"bonds":[]}])),
    ]));
    // TP1 is atoms 17..25 (copy 17..24, new O 25); TP2 copies TP1 with its maps; atom 23 of TP1 is its methyl (parent 7)
    let maps = |k: usize| -> Vec<u16> {
        let mut m: Vec<u16> = mzdraw::layout::pieces(&d)[k]
            .iter()
            .filter_map(|&i| d.atom(i))
            .map(|a| a.map)
            .collect();
        m.sort_unstable();
        m
    };
    assert_eq!(maps(1), (1..=9).collect::<Vec<u16>>());
    assert_eq!(
        maps(2),
        vec![1, 2, 3, 4, 5, 6, 8, 9],
        "TP2 is TP1 without the methyl (map 7), new atom 9 kept"
    );
    assert!(d.atoms.iter().all(|a| a.map > 0));
    // within each structure a map is unique
    for piece in mzdraw::layout::pieces(&d) {
        let mut m: Vec<u16> = piece.iter().map(|&i| d.atom(i).unwrap().map).collect();
        m.sort_unstable();
        m.dedup();
        assert_eq!(m.len(), piece.len());
    }
    assert_eq!(d.objects.len(), 2);
    assert_eq!(d.objects[1].from, 17);
}

#[test]
fn at_puts_the_centre_of_the_copy_where_asked() {
    let r = replay(
        json!({"import": lactam(0.0).to_string(), "out": "mzdraw", "commands":
        [{"command":"tp:derive","args":{"of":1,"at":[10.0,5.0]}}]}),
    );
    let d = Document::from_json(&r.to_string()).unwrap();
    let (xs, ys): (Vec<f64>, Vec<f64>) = (1..=8)
        .map(|m| (mapped(&d, 1, m).x, mapped(&d, 1, m).y))
        .unzip();
    let mid = |v: &[f64]| {
        (v.iter().cloned().fold(f64::MAX, f64::min) + v.iter().cloned().fold(f64::MIN, f64::max))
            / 2.0
    };
    assert!((mid(&xs) - 10.0).abs() < 0.5 && (mid(&ys) - 5.0).abs() < 0.5);
}

#[test]
fn undo_returns_the_identical_document_with_the_maps_of_the_parent() {
    let before = lactam(0.0).to_string();
    let mut ed = Editor::default();
    ed.import(&before).unwrap();
    let before = ed.doc().to_json();
    ed.run_command("tp:derive", &json!({"of": 1, "patch": []}))
        .unwrap();
    assert_ne!(ed.doc().to_json(), before);
    ed.undo();
    assert_eq!(ed.doc().to_json(), before, "one undo step, maps included");
    ed.redo();
    assert_eq!(ed.doc().atoms.len(), 16);
}

#[test]
fn refusals_change_nothing_and_name_their_key() {
    let pentavalent = json!([{"op":"atom.add","el":"C","x":0.0,"y":-1.0},{"op":"bond.add","a":3,"b":-1,"o":2},{"op":"bond.add","a":3,"b":-1,"o":2}]);
    let five = json!([{"op":"atom.add","el":"C","x":0.0,"y":-1.0},{"op":"bond.add","a":3,"b":-1,"o":1},{"op":"atom.add","el":"C","x":1.0,"y":-1.0},{"op":"bond.add","a":3,"b":-2,"o":1},{"op":"atom.add","el":"C","x":2.0,"y":-1.0},{"op":"bond.add","a":3,"b":-3,"o":1}]);
    for (patch, key) in [
        (five, "error.draw2.tpPatch"),
        (
            json!([{"op":"bond.add","a":3,"b":99,"o":1}]),
            "error.draw2.tpPatch",
        ),
        (
            json!([{"op":"obj.add","t":"text","x":0,"y":0,"x2":0,"y2":0,"text":"t"}]),
            "error.draw2.tpPatch",
        ),
        (pentavalent, "error.draw2.tpPatch"),
    ] {
        let r = replay(
            json!({"import": lactam(0.0).to_string(), "report": true, "commands": [derive(1, patch)]}),
        );
        assert_eq!(r["ok"], false, "{r}");
        assert_eq!(r["failed"]["stage"], "command");
        assert_eq!(r["failed"]["key"], key, "{r}");
        assert!(r.get("out").is_none() || r["created"] == json!({}));
    }
    // an unknown atom
    let r = replay(
        json!({"import": lactam(0.0).to_string(), "report": true, "commands": [derive(99, json!([]))]}),
    );
    assert_eq!(r["failed"]["key"], "error.draw2.noAtom");
}

#[test]
fn a_parent_of_a_thousand_atoms_is_refused_for_its_maps() {
    let n = 1000u32;
    let atoms: Vec<Value> = (1..=n)
        .map(|i| json!({"id": i, "el": "C", "x": i as f64, "y": 0.0}))
        .collect();
    let bonds: Vec<Value> = (1..n)
        .map(|i| json!({"id": n + i, "a": i, "b": i + 1, "o": 1}))
        .collect();
    let doc = json!({"format":"mzdraw/1","next":2 * n + 1,"atoms":atoms,"bonds":bonds}).to_string();
    let r = replay(json!({"import": doc, "report": true, "commands": [derive(1, json!([]))]}));
    assert_eq!(
        (r["ok"].clone(), r["failed"]["key"].clone()),
        (json!(false), json!("error.draw2.tpMaps"))
    );
    // 999 atoms need exactly the numbers 1..999
    let ok =
        replay(json!({"import": chain(999), "report": true, "commands": [derive(1, json!([]))]}));
    assert_eq!(ok["ok"], true, "{ok}");
}

#[test]
fn the_log_holds_plain_operations_and_replays_to_the_same_bytes() {
    let req = json!({"import": lactam(0.0).to_string(), "out": "log", "commands": [derive(1, hydroxylate())]});
    let log = replay(req);
    let ops: Vec<&str> = log
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["op"].as_str().unwrap())
        .collect();
    assert!(!ops.contains(&"tp:derive"));
    assert!(ops.contains(&"atom.set") && ops.contains(&"atom.add") && ops.contains(&"obj.add"));
    let direct = replay(
        json!({"import": lactam(0.0).to_string(), "commands": [derive(1, hydroxylate())], "out": "mzdraw"}),
    );
    let by_log = replay(json!({"log": log, "out": "mzdraw"}));
    assert_eq!(direct, by_log);
}

#[test]
fn a_dry_run_validates_without_writing_and_a_bad_shape_is_a_request_error() {
    let r = replay(
        json!({"import": lactam(0.0).to_string(), "dry_run": true, "commands": [derive(1, hydroxylate())]}),
    );
    assert_eq!(
        (r["ok"].clone(), r["dry_run"].clone()),
        (json!(true), json!(true))
    );
    assert!(r.get("out").is_none());
    for bad in [
        json!({"of": 0}),
        json!({"of": 1, "x": 2}),
        json!({"of": 1, "at": [1]}),
        json!({"of": 1, "patch": 3}),
    ] {
        let e = mzdraw::replay::replay(&json!({"import": lactam(0.0).to_string(), "commands":[{"command":"tp:derive","args":bad}]}).to_string());
        assert_eq!(e.unwrap_err().key, "error.draw2.request");
    }
}

#[test]
fn a_thousand_atom_derive_stays_quick() {
    let n = 900u32;
    let atoms: Vec<Value> = (1..=n)
        .map(|i| json!({"id": i, "el": "C", "x": i as f64, "y": 0.0}))
        .collect();
    let bonds: Vec<Value> = (1..n)
        .map(|i| json!({"id": n + i, "a": i, "b": i + 1, "o": 1}))
        .collect();
    let doc = json!({"format":"mzdraw/1","next":2 * n + 1,"atoms":atoms,"bonds":bonds}).to_string();
    let t = std::time::Instant::now();
    let r = replay(
        json!({"import": doc, "report": true, "commands": [derive(1, json!([])), derive(1, json!([]))]}),
    );
    assert_eq!(r["ok"], true, "{r}");
    assert!(t.elapsed().as_secs_f64() < 5.0, "{:?}", t.elapsed());
}

#[test]
fn shift_d_and_the_menu_item_derive_the_focused_structure_and_select_the_copy() {
    use mzdraw::editor::{CHANGED, SHIFT};
    let mut e = Editor::default();
    e.import(&lactam(0.0).to_string()).unwrap();
    let before = e.doc().to_json();
    // off by default: the key does nothing and the menu has no item
    e.over.sel_atoms = vec![3];
    assert_eq!(e.key("D", SHIFT) & CHANGED, 0);
    assert_eq!(e.doc().to_json(), before);
    let (px, py) = e.view.to_px(1.0, 0.0);
    assert!(
        !e.context_menu(px, py).contains("tp.derive"),
        "no item while the setting is off"
    );
    e.over.sel_atoms.clear();
    e.set("pathway.derive=on").unwrap();
    assert!(e.set("pathway.derive=maybe").is_err());
    // nothing focused or selected: refused, nothing written
    assert_eq!(e.key("D", SHIFT) & CHANGED, 0);
    assert_eq!(e.doc().to_json(), before);
    e.over.sel_atoms = vec![3];
    assert_ne!(e.key("D", SHIFT) & CHANGED, 0);
    assert_eq!(e.doc().atoms.len(), 16);
    assert_eq!(e.over.sel_atoms.len(), 8, "the copy is selected");
    assert!(e.over.sel_atoms.iter().all(|&i| i >= 17));
    e.undo();
    assert_eq!(e.doc().to_json(), before);
    // the menu item of an atom runs the same
    let (px, py) = e.view.to_px(1.0, 0.0);
    let menu: Value = serde_json::from_str(&e.context_menu(px, py)).unwrap();
    let ids: Vec<&str> = menu["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"tp.derive"), "{ids:?}");
    assert_ne!(e.context_run("tp.derive") & CHANGED, 0);
    assert_eq!(e.doc().atoms.len(), 16);
    assert_eq!(e.doc().objects.len(), 1);
}

/// `n` two-atom structures on a wide grid (atom ids 1..=2n, bond ids after them).
fn many(n: u32) -> String {
    let mut atoms = Vec::new();
    let mut bonds = Vec::new();
    for k in 0..n {
        let (x, y) = ((k % 60) as f64 * 6.0, -((k / 60) as f64) * 6.0);
        atoms.push(json!({"id": 2 * k + 1, "el": "C", "x": x, "y": y}));
        atoms.push(json!({"id": 2 * k + 2, "el": "C", "x": x + 1.0, "y": y}));
        bonds.push(json!({"id": 2 * n + k + 1, "a": 2 * k + 1, "b": 2 * k + 2, "o": 1}));
    }
    json!({"format":"mzdraw/1","next":3 * n + 1,"atoms":atoms,"bonds":bonds}).to_string()
}

/// The least CPU time of the thread over five runs (after a warm-up) of `derives` derives from the first structure of a drawing of `n` structures, in seconds.
fn derive_time(n: u32, derives: usize) -> f64 {
    let doc = many(n);
    let cmds: Vec<Value> = (0..derives).map(|_| derive(1, json!([]))).collect();
    let req = json!({"import": doc, "commands": cmds, "report": true}).to_string();
    timing::best_of(1, 5, || {
        let r: Value = serde_json::from_str(&mzdraw::replay::replay(&req).unwrap()).unwrap();
        assert_eq!(r["ok"], true, "{r}");
    })
    .as_secs_f64()
}

#[test]
fn the_cost_of_a_derive_grows_linearly_with_the_drawing() {
    // four times the structures must cost about four times as much, not sixteen: the pieces, the placement and the checks are linear.
    // The sizes are fixed (they were searched with a clock, so the size compared moved with the speed of the machine) and the bound
    // is a pure ratio, with no additive floor. Measured, the cost is a little over linear (the memory is not a pure scan) and the
    // ratio moves from run to run between about 4 and 8; a quadratic one is 16, and the bound 10 sits between the two.
    let (n, small) = (1000u32, derive_time(1000, 6));
    let big = derive_time(4 * n, 6);
    assert!(
        big < 10.0 * small,
        "{n} structures {small:.3} s, {} structures {big:.3} s",
        4 * n
    );
}

#[test]
fn a_parent_above_the_size_cap_and_a_patch_above_the_operation_cap_are_refused() {
    // 5001 atoms that all carry a map (the same one, which a file may hold): no map is needed, so the size cap is what refuses it
    let n = 5001u32;
    let atoms: Vec<Value> = (1..=n)
        .map(|i| json!({"id": i, "el": "C", "x": i as f64, "y": 0.0, "map": 1}))
        .collect();
    let bonds: Vec<Value> = (1..n)
        .map(|i| json!({"id": n + i, "a": i, "b": i + 1, "o": 1}))
        .collect();
    let doc = json!({"format":"mzdraw/1","next":2 * n + 1,"atoms":atoms,"bonds":bonds}).to_string();
    let r = replay(json!({"import": doc, "report": true, "commands": [derive(1, json!([]))]}));
    assert_eq!(r["failed"]["key"], "error.draw2.tpSize", "{r}");
    // the operation cap: in the request (an invalid request names the key) and in the function
    let many_ops: Vec<Value> = (0..10_001)
        .map(|_| json!({"op":"atom.set","id":1,"chg":0}))
        .collect();
    let e = mzdraw::replay::replay(
        &json!({"import": lactam(0.0).to_string(), "commands": [derive(1, json!(many_ops))]})
            .to_string(),
    );
    assert_eq!(e.unwrap_err().key, "error.draw2.tpPatch");
    let mut ed = Editor::default();
    ed.import(&lactam(0.0).to_string()).unwrap();
    let ops: Vec<mzdraw::ops::Op> = (0..10_001)
        .map(|_| mzdraw::ops::Op::parse(&json!({"op":"atom.set","id":1,"chg":0})).unwrap())
        .collect();
    assert_eq!(
        mzdraw::tp::derive(ed.doc(), &ed.style, 1, &ops, None).unwrap_err(),
        "error.draw2.tpPatch"
    );
}

#[test]
fn a_map_number_names_one_atom_a_patch_cannot_give_it_to_another() {
    // 3 is the map of atom 3 of the parent (and of its copy): a new atom may not take it, nor may two new atoms share one
    let take = |patch: Value| {
        replay(
            json!({"import": lactam(0.0).to_string(), "report": true, "commands": [derive(1, patch)]}),
        )
    };
    let r = take(
        json!([{"op":"atom.add","el":"O","x":-1.0,"y":-1.7321,"map":3},{"op":"bond.add","a":5,"b":-1,"o":1}]),
    );
    assert_eq!(r["failed"]["key"], "error.draw2.tpMapDup", "{r}");
    let r = take(
        json!([{"op":"atom.add","el":"O","x":-1.0,"y":-1.7321,"map":50},{"op":"atom.add","el":"O","x":0.0,"y":-3.0,"map":50}]),
    );
    assert_eq!(r["failed"]["key"], "error.draw2.tpMapDup", "{r}");
    let r = take(json!([{"op":"atom.set","id":4,"map":3}]));
    assert_eq!(r["failed"]["key"], "error.draw2.tpMapDup", "{r}");
    // a free number, or the atom's own, or none, is fine
    let r = take(
        json!([{"op":"atom.add","el":"O","x":-1.0,"y":-1.7321,"map":50},{"op":"atom.set","id":4,"map":4},{"op":"atom.set","id":6,"map":0}]),
    );
    assert_eq!(r["ok"], true, "{r}");
    // a map above 999 is the refusal of the operation itself
    let r = take(json!([{"op":"atom.set","id":4,"map":1000}]));
    assert_eq!(r["ok"], false, "{r}");
}

#[test]
fn the_copy_must_be_a_record_the_chemistry_layer_accepts() {
    // an order the record cannot hold (0 or 4), a bond to the atom itself, a second bond between two atoms: refused, nothing written
    for patch in [
        json!([{"op":"atom.add","el":"O","x":-1.0,"y":-1.7321},{"op":"bond.add","a":5,"b":-1,"o":4}]),
        json!([{"op":"atom.add","el":"O","x":-1.0,"y":-1.7321},{"op":"bond.add","a":5,"b":-1,"o":0}]),
        json!([{"op":"bond.add","a":5,"b":5,"o":1}]),
        json!([{"op":"bond.add","a":5,"b":4,"o":1}]),
        json!([{"op":"bond.set","id":9,"o":3},{"op":"bond.set","id":10,"o":3}]),
    ] {
        let r = replay(
            json!({"import": lactam(0.0).to_string(), "report": true, "commands": [derive(1, patch.clone())]}),
        );
        assert_eq!(r["ok"], false, "{patch}: {r}");
    }
    // a charge makes the valence allowed (N+ with four bonds): the allowed set of mzlab-mol, with the charge
    let r = replay(
        json!({"import": lactam(0.0).to_string(), "report": true, "commands": [derive(1, json!([
        {"op":"atom.add","el":"C","x":1.0,"y":-1.0},{"op":"bond.add","a":1,"b":-1,"o":1},{"op":"atom.set","id":1,"chg":1}]))]}),
    );
    assert_eq!(r["ok"], true, "{r}");
}
