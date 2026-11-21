//! The context menu: one menu per target (atom, bond, selection, object, empty canvas, empty document), every item runs as one undo
//! step that undoes exactly, an item that is not offered does nothing, and a menu older than the last change is refused.

use mzdraw::context::EMPTY_NOTICE;
use mzdraw::doc::ObjKind;
use mzdraw::editor::{ASK, CHANGED, HANDLED, MENU, MIDDLE, MOUSE, PEN, SECONDARY, SHIFT, TOUCH};
use mzdraw::editor_touch::{DOWN, ERASER, MOVE, UP};
use mzdraw::{Editor, Op};
use serde_json::Value;

/// A benzene ring with a text next to it, the ring unselected.
fn drawing() -> Editor {
    let mut e = Editor::default();
    e.set_tool("ring:6a");
    e.pointer_event(DOWN, 1, 300.0, 300.0, MOUSE, 0, 0.0);
    e.pointer_event(UP, 1, 300.0, 300.0, MOUSE, 0, 10.0);
    e.set_tool("select");
    let add = |t, x, text: &str| Op::ObjAdd {
        id: None,
        t,
        x,
        y: 0.0,
        x2: 0.0,
        y2: 0.0,
        text: text.into(),
        runs: Vec::new(),
        from: 0,
        to: 0,
    };
    e.session.run(&[add(ObjKind::Text, 6.0, "note")]).unwrap();
    e.session.run(&[add(ObjKind::Plus, 8.0, "")]).unwrap();
    e
}

fn atom_px(e: &Editor, id: u32) -> (f64, f64) {
    let a = e.doc().atom(id).unwrap();
    e.view.to_px(a.x, a.y)
}

fn bond_px(e: &Editor, id: u32) -> (f64, f64) {
    let b = e.doc().bond(id).unwrap();
    let (p, q) = (atom_px(e, b.a), atom_px(e, b.b));
    ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0)
}

fn object_px(e: &Editor, id: u32) -> (f64, f64) {
    let o = e.doc().object(id).unwrap();
    e.view.to_px(o.x, o.y)
}

/// Each scenario puts an editor in a state and gives the pixel of the right-click.
fn scenario(name: &str) -> (Editor, (f64, f64)) {
    let mut e = drawing();
    let obj = e.doc().objects[0].id;
    let at = match name {
        "atom" => atom_px(&e, 1),
        "bond" => bond_px(&e, e.doc().bonds[0].id),
        "selection" => {
            e.select_all();
            atom_px(&e, 2)
        }
        "bond selection" => {
            let b = e.doc().bonds[0].id;
            e.over.sel_bonds = vec![b];
            bond_px(&e, b)
        }
        "text" => object_px(&e, obj),
        "plus" => object_px(&e, e.doc().objects[1].id),
        "canvas" => (40.0, 40.0),
        "canvas with history and clipboard" => {
            e.select_all();
            e.copy();
            e.undo();
            e.over.sel_atoms.clear();
            e.over.sel_bonds.clear();
            e.over.sel_objs.clear();
            (40.0, 40.0)
        }
        "empty document" => {
            e = Editor::default();
            (40.0, 40.0)
        }
        _ => unreachable!(),
    };
    (e, at)
}

const SCENARIOS: [&str; 9] = [
    "atom",
    "bond",
    "selection",
    "bond selection",
    "text",
    "plus",
    "canvas",
    "canvas with history and clipboard",
    "empty document",
];

fn parse(menu: &str) -> Value {
    serde_json::from_str(menu).unwrap()
}

/// Every item with an id, children included.
fn items(menu: &Value) -> Vec<&Value> {
    fn walk<'a>(list: &'a Value, out: &mut Vec<&'a Value>) {
        for i in list.as_array().unwrap() {
            out.push(i);
            if i.get("children").is_some() {
                walk(&i["children"], out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&menu["items"], &mut out);
    out
}

fn find<'a>(menu: &'a Value, id: &str) -> &'a Value {
    items(menu)
        .into_iter()
        .find(|i| i["id"] == id)
        .unwrap_or_else(|| panic!("no item {id}"))
}

fn ids(menu: &Value) -> Vec<String> {
    items(menu)
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn an_atom_menu_has_the_atom_items_and_the_keys_that_are_built() {
    let (mut e, at) = scenario("atom");
    let m = parse(&e.context_menu(at.0, at.1));
    assert_eq!(m["target"], "atom");
    assert_eq!(
        e.over.sel_atoms,
        [1],
        "the atom under the pointer is the selection"
    );
    for id in [
        "atom.element",
        "atom.el.C",
        "atom.el.Cl",
        "atom.el.I",
        "atom.el.more",
        "atom.chg.plus",
        "atom.chg.minus",
        "atom.label",
        "atom.data",
        "atom.ann",
        "edit.copy",
        "edit.cut",
        "edit.delete",
        "atom.connected",
    ] {
        find(&m, id);
    }
    assert_eq!(find(&m, "atom.el.C")["checked"], true);
    assert_eq!(find(&m, "atom.el.N")["checked"], false);
    assert_eq!(find(&m, "atom.el.Cl")["key"], "Shift+C");
    assert_eq!(find(&m, "edit.copy")["key"], "Ctrl+C");
    assert_eq!(find(&m, "edit.delete")["key"], "Del");
    assert_eq!(
        find(&m, "atom.element")["children"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
}

#[test]
fn an_atom_item_changes_the_atom_in_one_undo_step() {
    let (mut e, at) = scenario("atom");
    e.context_menu(at.0, at.1);
    assert_eq!(e.context_run("atom.el.N") & CHANGED, CHANGED);
    assert_eq!(e.doc().atom(1).unwrap().el, "N");
    e.context_menu(at.0, at.1);
    e.context_run("atom.chg.plus");
    assert_eq!(e.doc().atom(1).unwrap().chg, 1);
    assert_eq!(
        e.session.steps(),
        3 + 2,
        "the ring and two objects came first"
    );
    e.undo();
    e.undo();
    assert_eq!(e.doc().atom(1).unwrap().el, "C");
}

#[test]
fn select_connected_selects_the_piece() {
    let (mut e, at) = scenario("atom");
    e.context_menu(at.0, at.1);
    assert_eq!(
        e.context_run("atom.connected") & CHANGED,
        0,
        "a selection is not a change of the document"
    );
    assert_eq!((e.over.sel_atoms.len(), e.over.sel_bonds.len()), (6, 6));
}

#[test]
fn a_bond_menu_marks_the_bond_it_has() {
    let (mut e, at) = scenario("bond");
    let b1 = e.doc().bonds[0].id;
    let m = parse(&e.context_menu(at.0, at.1));
    assert_eq!(m["target"], "bond");
    let b = e.doc().bond(b1).unwrap().o;
    assert_eq!(find(&m, &format!("bond.o.{b}"))["checked"], true);
    assert_eq!(
        find(&m, "bond.reverse")["enabled"],
        false,
        "nothing to reverse on a plain bond"
    );
    assert_eq!(find(&m, "bond.s.down")["key"], "Shift+W");
    e.context_run("bond.s.up");
    assert_eq!(e.doc().bond(b1).unwrap().s, 1);
    let (a0, b0) = (e.doc().bond(b1).unwrap().a, e.doc().bond(b1).unwrap().b);
    let at = bond_px(&e, b1);
    let m = parse(&e.context_menu(at.0, at.1));
    assert_eq!(find(&m, "bond.s.up")["checked"], true);
    assert_eq!(find(&m, "bond.reverse")["enabled"], true);
    e.context_run("bond.reverse");
    let now = e.doc().bond(b1).unwrap();
    assert_eq!(
        (now.a, now.b),
        (b0, a0),
        "the narrow end moved to the other atom"
    );
    // delete removes the bond and keeps its atoms
    let (atoms, bonds) = (e.doc().atoms.len(), e.doc().bonds.len());
    e.context_menu(at.0, at.1);
    e.context_run("bond.delete");
    assert_eq!(
        (e.doc().atoms.len(), e.doc().bonds.len()),
        (atoms, bonds - 1)
    );
}

#[test]
fn a_selection_menu_acts_on_the_whole_selection() {
    let (mut e, at) = scenario("selection");
    let m = parse(&e.context_menu(at.0, at.1));
    assert_eq!(m["target"], "selection");
    assert_eq!(
        (e.over.sel_atoms.len(), e.over.sel_objs.len()),
        (6, 2),
        "a click inside keeps the selection"
    );
    for id in [
        "edit.cut",
        "edit.copy",
        "edit.paste",
        "edit.duplicate",
        "sel.rotate.l",
        "sel.rotate.r",
        "sel.flip.h",
        "sel.flip.v",
        "edit.delete",
        "sel.copyas.molfile",
        "sel.copyas.smiles",
        "sel.copyas.svg",
        "sel.copyas.png",
    ] {
        find(&m, id);
    }
    assert_eq!(
        find(&m, "edit.paste")["enabled"],
        false,
        "nothing was copied"
    );
    let before = e.doc().to_json();
    assert_eq!(e.context_run("sel.rotate.l") & CHANGED, CHANGED);
    assert_ne!(e.doc().to_json(), before);
    e.context_menu(at.0, at.1);
    e.context_run("edit.duplicate");
    assert_eq!(e.doc().atoms.len(), 12);
}

#[test]
fn a_click_outside_the_selection_replaces_it_and_one_on_nothing_clears_it() {
    let (mut e, _) = scenario("selection");
    let far = object_px(&e, e.doc().objects[0].id);
    e.over.sel_atoms = vec![1, 2];
    e.over.sel_bonds.clear();
    e.over.sel_objs.clear();
    let m = parse(&e.context_menu(far.0, far.1));
    assert_eq!(m["target"], "object");
    assert_eq!((e.over.sel_atoms.len(), e.over.sel_objs.len()), (0, 1));
    let m = parse(&e.context_menu(40.0, 40.0));
    assert_eq!(m["target"], "canvas");
    assert_eq!((e.over.sel_atoms.len(), e.over.sel_objs.len()), (0, 0));
}

#[test]
fn copy_as_gives_the_selection_in_each_format() {
    let (mut e, at) = scenario("selection");
    e.context_menu(at.0, at.1);
    assert_eq!(e.context_run("sel.copyas.smiles") & ASK, ASK);
    assert_eq!(e.context_ask(), "copy-as:smiles");
    assert_eq!(e.context_ask(), "", "the ask is read once");
    assert!(e.context_copy_as("molfile").unwrap().contains("V2000"));
    assert!(!e.context_copy_as("smiles").unwrap().is_empty());
    assert!(e.context_copy_as("svg").unwrap().contains("<svg"));
    assert!(e.context_copy_as("pdf").is_err());
    assert!(e.context_png_plan().width > 0);
    assert_eq!(e.context_selection_doc().atoms.len(), 6);
}

#[test]
fn an_object_menu_edits_a_text_and_not_a_plus_sign() {
    let (mut e, at) = scenario("text");
    let m = parse(&e.context_menu(at.0, at.1));
    assert_eq!(m["target"], "object");
    assert_eq!(find(&m, "obj.edit")["enabled"], true);
    let id = e.doc().objects[0].id;
    assert_eq!(e.context_run("obj.edit") & ASK, ASK);
    assert_eq!(e.context_ask(), format!("text:{id}"));
    let (mut e, at) = scenario("plus");
    let m = parse(&e.context_menu(at.0, at.1));
    assert_eq!(find(&m, "obj.edit")["enabled"], false);
    e.context_menu(at.0, at.1);
    e.context_run("edit.delete");
    assert_eq!(e.doc().objects.len(), 1);
}

#[test]
fn the_canvas_menu_offers_the_view_and_the_history() {
    let (mut e, at) = scenario("canvas with history and clipboard");
    let m = parse(&e.context_menu(at.0, at.1));
    assert_eq!(m["target"], "canvas");
    assert!(m.get("notice").is_none());
    assert_eq!(find(&m, "edit.undo")["enabled"], true);
    assert_eq!(
        find(&m, "edit.redo")["enabled"],
        true,
        "the undone step can come back"
    );
    assert_eq!(find(&m, "edit.paste")["enabled"], true);
    assert_eq!(find(&m, "edit.undo")["key"], "Ctrl+Z");
    assert_eq!(find(&m, "edit.redo")["key"], "Ctrl+Shift+Z");
    assert_eq!(find(&m, "edit.all")["key"], "Ctrl+A");
    for id in [
        "canvas.add",
        "canvas.fit",
        "canvas.zoom.in",
        "canvas.zoom.out",
        "canvas.save",
        "canvas.clear",
    ] {
        assert_eq!(find(&m, id)["enabled"], true, "{id}");
    }
    // zoom keeps the point of the click still
    let (x, y) = e.view.to_doc(at.0, at.1);
    let scale = e.view.scale;
    e.context_run("canvas.zoom.in");
    assert!((e.view.scale / scale - 1.25).abs() < 1e-9);
    let back = e.view.to_px(x, y);
    assert!((back.0 - at.0).abs() < 1e-6 && (back.1 - at.1).abs() < 1e-6);
    e.context_menu(at.0, at.1);
    assert_eq!(e.context_run("canvas.save") & ASK, ASK);
    assert_eq!(e.context_ask(), "save-image");
    e.context_menu(at.0, at.1);
    assert_eq!(e.context_run("canvas.clear") & CHANGED, CHANGED);
    assert!(e.doc().is_empty());
}

#[test]
fn the_menu_of_an_empty_document_does_not_offer_to_export() {
    let (mut e, at) = scenario("empty document");
    let m = parse(&e.context_menu(at.0, at.1));
    assert_eq!(m["target"], "canvas");
    assert_eq!(m["notice"], EMPTY_NOTICE);
    assert!(!ids(&m).contains(&"canvas.save".to_string()));
    for id in [
        "edit.all",
        "canvas.clear",
        "edit.undo",
        "edit.redo",
        "edit.paste",
    ] {
        assert_eq!(find(&m, id)["enabled"], false, "{id}");
    }
    assert_eq!(
        e.context_run("canvas.save"),
        0,
        "an export that is not offered does nothing"
    );
    assert_eq!(e.context_ask(), "");
}

#[test]
fn a_menu_older_than_the_last_change_is_refused() {
    for name in SCENARIOS.iter().filter(|n| **n != "empty document") {
        let (mut e, at) = scenario(name);
        let m = parse(&e.context_menu(at.0, at.1));
        let id = (items(&m).into_iter())
            .find(|i| i["enabled"] == true && i.get("children").is_none())
            .map(|i| i["id"].as_str().unwrap().to_string())
            .unwrap();
        // the document changes under the open menu
        e.session
            .run(&[Op::AtomAdd {
                map: None,
                id: None,
                el: "N".into(),
                x: 9.0,
                y: 9.0,
            }])
            .unwrap();
        let doc = e.doc().to_json();
        e.context_run(&id);
        assert_eq!(e.error, "error.draw2.contextStale", "{name}");
        assert_eq!(e.doc().to_json(), doc, "{name}: nothing ran");
        // the selection, the view and a closed menu refuse too
        let (mut e, at) = scenario(name);
        e.context_menu(at.0, at.1);
        e.pan(5.0, 0.0);
        assert_eq!(e.context_run(&id) & CHANGED, 0, "{name}: the view moved");
        e.context_menu(at.0, at.1);
        e.context_close();
        assert_eq!(e.context_run(&id), 0, "{name}: closed");
    }
}

/// The property: every item a menu offers runs; a change of the document is one undo step that restores the identical document and
/// that redo brings back identically; an item that is offered disabled, or not at all, does nothing.
#[test]
fn every_item_runs_and_its_undo_restores_the_identical_document() {
    let mut ran = 0;
    for name in SCENARIOS {
        let (e, at) = scenario(name);
        let mut probe = e;
        let m = parse(&probe.context_menu(at.0, at.1));
        let all = items(&m);
        let leaves: Vec<_> = all.iter().filter(|i| i.get("children").is_none()).collect();
        assert!(!leaves.is_empty(), "{name}");
        let mut seen = ids(&m);
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), all.len(), "{name}: ids are unique");
        for item in leaves {
            let id = item["id"].as_str().unwrap();
            let (mut e, at) = scenario(name);
            e.context_menu(at.0, at.1);
            let (before, steps) = (e.doc().to_json(), e.session.steps());
            let f = e.context_run(id);
            if item["enabled"] != true {
                assert_eq!(f, 0, "{name}: {id} is disabled");
                assert_eq!(e.doc().to_json(), before, "{name}: {id}");
                continue;
            }
            ran += 1;
            assert_eq!(e.error, "", "{name}: {id} is refused");
            if f & ASK != 0 {
                assert_ne!(e.context_ask(), "", "{name}: {id} says what to ask");
            }
            if f & CHANGED == 0 {
                assert_eq!(e.doc().to_json(), before, "{name}: {id} changed nothing");
                continue;
            }
            let after = e.doc().to_json();
            // the history items move along the history: their own undo is the other one
            if id == "edit.undo" || id == "edit.redo" {
                assert_ne!(after, before, "{name}: {id}");
                if id == "edit.undo" {
                    e.redo()
                } else {
                    e.undo()
                };
                assert_eq!(e.doc().to_json(), before, "{name}: the way back of {id}");
                continue;
            }
            assert_eq!(
                e.session.steps(),
                steps + 1,
                "{name}: {id} is one undo step"
            );
            e.undo();
            assert_eq!(e.doc().to_json(), before, "{name}: undo of {id}");
            e.redo();
            assert_eq!(e.doc().to_json(), after, "{name}: redo of {id}");
        }
    }
    assert!(ran >= 60, "{ran} items ran");
}

#[test]
fn a_selection_of_a_bond_alone_has_nothing_to_copy_and_deletes_the_bond_only() {
    let (mut e, at) = scenario("bond selection");
    let b = e.doc().bonds[0].id;
    let m = parse(&e.context_menu(at.0, at.1));
    assert_eq!(m["target"], "selection");
    assert_eq!(
        (e.over.sel_atoms.len(), e.over.sel_bonds.clone()),
        (0, vec![b]),
        "the selection is kept as it was"
    );
    for id in [
        "edit.cut",
        "edit.copy",
        "edit.duplicate",
        "sel.rotate.l",
        "sel.flip.h",
        "sel.copyas.molfile",
        "sel.copyas.smiles",
    ] {
        assert_eq!(find(&m, id)["enabled"], false, "{id}");
    }
    assert_eq!(find(&m, "edit.delete")["enabled"], true);
    let (atoms, bonds) = (e.doc().atoms.len(), e.doc().bonds.len());
    assert_eq!(e.context_run("edit.delete") & CHANGED, CHANGED);
    assert_eq!(
        (e.doc().atoms.len(), e.doc().bonds.len()),
        (atoms, bonds - 1)
    );
}

#[test]
fn the_charge_items_are_disabled_at_the_limit_of_the_charge() {
    let (mut e, at) = scenario("atom");
    for (chg, plus, minus) in [(3, true, true), (4, false, true), (-4, true, false)] {
        let set = Op::AtomSet {
            map: None,
            id: 1,
            el: None,
            chg: Some(chg),
            iso: None,
            rad: None,
            h: None,
            lab: None,
        };
        e.session.run(&[set]).unwrap();
        let m = parse(&e.context_menu(at.0, at.1));
        assert_eq!(find(&m, "atom.chg.plus")["enabled"], plus, "charge {chg}");
        assert_eq!(find(&m, "atom.chg.minus")["enabled"], minus, "charge {chg}");
    }
}

#[test]
fn a_coordinate_that_is_not_finite_never_reaches_the_view() {
    for (x, y) in [
        (f64::NAN, 10.0),
        (10.0, f64::INFINITY),
        (f64::NEG_INFINITY, f64::NAN),
    ] {
        let (mut e, _) = scenario("canvas");
        let m = parse(&e.context_menu(x, y));
        assert_eq!(m["target"], "canvas");
        for id in ["canvas.zoom.in", "canvas.zoom.out"] {
            e.context_menu(x, y);
            e.context_run(id);
            let v = e.view;
            assert!(
                v.scale.is_finite() && v.ox.is_finite() && v.oy.is_finite(),
                "{id} at ({x}, {y}): {v:?}"
            );
        }
    }
    // and the zoom of the editor itself ignores what it cannot use
    let (mut e, _) = scenario("canvas");
    let view = e.view;
    for (x, y, f) in [
        (f64::NAN, 1.0, 1.1),
        (1.0, 1.0, f64::INFINITY),
        (1.0, 1.0, 0.0),
        (1.0, 1.0, -2.0),
    ] {
        assert_eq!(e.zoom(x, y, f), 0);
    }
    assert_eq!(
        (e.view.scale, e.view.ox, e.view.oy),
        (view.scale, view.ox, view.oy)
    );
}

#[test]
fn a_finger_or_a_pen_held_still_asks_for_the_menu_and_a_mouse_does_not() {
    for kind in [TOUCH, PEN] {
        let (mut e, at) = scenario("atom");
        e.set_tool("atom:N");
        let doc = e.doc().to_json();
        // held half a second over an atom: the menu is asked for, the atom is selected, the tool does not act
        e.pointer_event(DOWN, 1, at.0, at.1, kind, 0, 1000.0);
        let f = e.pointer_event(UP, 1, at.0, at.1, kind, 0, 1600.0);
        assert_eq!(f & (MENU | HANDLED), MENU | HANDLED, "kind {kind}");
        assert_eq!(e.over.sel_atoms, [1]);
        assert_eq!(e.doc().to_json(), doc, "the tool did not act");
        // held over an empty spot with a tool that would draw: the menu, no atom
        e.pointer_event(DOWN, 1, 40.0, 40.0, kind, 0, 2000.0);
        let f = e.pointer_event(UP, 1, 40.0, 40.0, kind, 0, 2700.0);
        assert_ne!(f & MENU, 0, "kind {kind}: empty canvas");
        assert_eq!(e.doc().to_json(), doc, "kind {kind}: nothing was drawn");
        // a short press is the tool's, not the menu's
        e.pointer_event(DOWN, 1, 40.0, 40.0, kind, 0, 3000.0);
        let f = e.pointer_event(UP, 1, 40.0, 40.0, kind, 0, 3100.0);
        assert_eq!(f & MENU, 0, "kind {kind}: short");
        assert_ne!(e.doc().to_json(), doc, "kind {kind}: the tool drew");
    }
    // moved: a drag; a mouse held still: a click; the eraser end of a pen held still: no menu
    let (mut e, at) = scenario("atom");
    e.pointer_event(DOWN, 1, at.0, at.1, TOUCH, 0, 0.0);
    e.pointer_event(MOVE, 1, at.0 + 30.0, at.1, TOUCH, 0, 300.0);
    assert_eq!(
        e.pointer_event(UP, 1, at.0 + 30.0, at.1, TOUCH, 0, 900.0) & MENU,
        0
    );
    e.pointer_event(DOWN, 1, at.0, at.1, MOUSE, 0, 2000.0);
    assert_eq!(
        e.pointer_event(UP, 1, at.0, at.1, MOUSE, 0, 3000.0) & MENU,
        0
    );
    e.pointer_event(DOWN, 1, at.0, at.1, PEN, ERASER, 4000.0);
    assert_eq!(
        e.pointer_event(UP, 1, at.0, at.1, PEN, ERASER, 4800.0) & MENU,
        0
    );
}

#[test]
fn the_menu_key_and_shift_f10_ask_for_the_menu_at_the_focus() {
    let (mut e, _) = scenario("canvas");
    assert_eq!(e.key("ContextMenu", 0) & (MENU | HANDLED), MENU | HANDLED);
    assert_eq!(e.key("F10", SHIFT) & MENU, MENU);
    assert_eq!(e.key("F10", 0) & MENU, 0, "F10 alone is not the menu");
    // nothing focused or selected: the middle of the canvas; the focus: that atom; else the first selected object
    assert_eq!(e.context_anchor(800.0, 600.0), (400.0, 300.0));
    assert_eq!(e.context_anchor(f64::NAN, 600.0), (0.0, 300.0));
    e.key("Home", 0);
    let on = atom_px(&e, e.over.focus.unwrap());
    assert_eq!(e.context_anchor(800.0, 600.0), on);
    let m = parse(&e.context_menu(on.0, on.1));
    assert_eq!(
        m["target"], "atom",
        "the menu at the anchor is the menu of the focused atom"
    );
    e.over.focus = None;
    e.over.sel_atoms.clear();
    e.over.sel_bonds.clear();
    let obj = e.doc().objects[0].id;
    e.over.sel_objs = vec![obj];
    assert_eq!(e.context_anchor(800.0, 600.0), object_px(&e, obj));
    e.over.sel_objs.clear();
    e.over.sel_atoms = vec![3, 4];
    assert_eq!(e.context_anchor(800.0, 600.0), atom_px(&e, 3));
}

#[test]
fn the_menu_is_valid_until_the_state_it_was_built_from_changes() {
    let (mut e, at) = scenario("atom");
    assert!(!e.context_valid(), "no menu yet");
    e.context_menu(at.0, at.1);
    assert!(e.context_valid());
    e.pointer_move(1.0, 1.0, MOUSE);
    assert!(
        e.context_valid(),
        "a pointer over the canvas changes nothing the menu depends on"
    );
    e.zoom(at.0, at.1, 1.1);
    assert!(!e.context_valid(), "the view moved");
    e.context_menu(at.0, at.1);
    e.over.sel_atoms.push(2);
    assert!(!e.context_valid(), "the selection changed");
    e.context_menu(at.0, at.1);
    e.undo();
    assert!(!e.context_valid(), "the document changed");
    e.context_menu(at.0, at.1);
    e.context_close();
    assert!(!e.context_valid(), "closed");
}

#[test]
fn data_and_annotation_are_asked_for_the_structure_anchor() {
    let (mut e, _) = scenario("atom");
    let at = atom_px(&e, 3);
    for (id, what) in [("atom.data", "data"), ("atom.ann", "annotation")] {
        e.context_menu(at.0, at.1);
        assert_eq!(e.context_run(id) & ASK, ASK);
        assert_eq!(
            e.context_ask(),
            format!("{what}:1"),
            "the first atom of the ring, not the one clicked"
        );
    }
}

#[test]
fn the_secondary_button_asks_for_the_menu_and_its_drag_pans() {
    let (mut e, at) = scenario("atom");
    e.set_tool("atom:N");
    let doc = e.doc().to_json();
    // pressed and released in place: the menu is asked for on release, the tool does not act
    let f = e.pointer_event(DOWN, 1, at.0, at.1, MOUSE, SECONDARY, 0.0);
    assert_eq!(
        f & (MENU | HANDLED),
        HANDLED,
        "nothing is asked before the release"
    );
    let f = e.pointer_event(UP, 1, at.0, at.1, MOUSE, 0, 50.0);
    assert_eq!(f & (MENU | HANDLED), MENU | HANDLED);
    assert_eq!(e.doc().to_json(), doc);
    // a small shake is still a click; a drag pans the view (the first move past the slop takes the whole way) and asks for nothing
    let view = (e.view.ox, e.view.oy);
    e.pointer_event(DOWN, 1, at.0, at.1, MOUSE, SECONDARY, 100.0);
    e.pointer_event(MOVE, 1, at.0 + 2.0, at.1, MOUSE, SECONDARY, 110.0);
    assert_eq!(
        (e.view.ox, e.view.oy),
        view,
        "within the slop the view stays"
    );
    e.pointer_event(MOVE, 1, at.0 + 30.0, at.1 + 10.0, MOUSE, SECONDARY, 120.0);
    assert_eq!((e.view.ox - view.0, e.view.oy - view.1), (30.0, 10.0));
    e.pointer_event(MOVE, 1, at.0 + 40.0, at.1 + 10.0, MOUSE, SECONDARY, 130.0);
    assert_eq!(e.view.ox - view.0, 40.0);
    let f = e.pointer_event(UP, 1, at.0 + 40.0, at.1 + 10.0, MOUSE, 0, 140.0);
    assert_eq!(f & MENU, 0, "a drag is not a click");
    assert_eq!(e.doc().to_json(), doc);
    // the middle button pans and never asks for the menu, even in place
    let view = (e.view.ox, e.view.oy);
    e.pointer_event(DOWN, 1, 300.0, 300.0, MOUSE, MIDDLE, 200.0);
    e.pointer_event(MOVE, 1, 310.0, 300.0, MOUSE, MIDDLE, 210.0);
    assert_eq!(e.view.ox - view.0, 10.0);
    assert_eq!(
        e.pointer_event(UP, 1, 310.0, 300.0, MOUSE, 0, 220.0) & MENU,
        0
    );
    e.pointer_event(DOWN, 1, 300.0, 300.0, MOUSE, MIDDLE, 300.0);
    assert_eq!(
        e.pointer_event(UP, 1, 300.0, 300.0, MOUSE, 0, 310.0) & MENU,
        0
    );
    // a cancelled press asks for nothing; the left button is a left button again after all of it
    e.pointer_event(DOWN, 1, at.0, at.1, MOUSE, SECONDARY, 400.0);
    assert_eq!(e.pointer_event(3, 1, at.0, at.1, MOUSE, 0, 410.0) & MENU, 0);
    e.pointer_event(DOWN, 1, 40.0, 40.0, MOUSE, 0, 500.0);
    e.pointer_event(UP, 1, 40.0, 40.0, MOUSE, 0, 510.0);
    assert_ne!(e.doc().to_json(), doc, "the tool draws again");
}

#[test]
fn the_barrel_button_of_a_pen_asks_for_the_menu_and_never_uses_the_tool() {
    let (mut e, at) = scenario("atom");
    e.set_tool("atom:N");
    let doc = e.doc().to_json();
    let f = e.pointer_event(DOWN, 1, at.0, at.1, PEN, SECONDARY, 0.0);
    assert_eq!(f & (MENU | HANDLED), MENU | HANDLED, "asked at the press");
    assert_eq!(
        e.pointer_event(UP, 1, at.0, at.1, PEN, 0, 30.0) & MENU,
        0,
        "once"
    );
    assert_eq!(e.doc().to_json(), doc, "no atom, no bond");
    // the tip alone draws, the eraser end (which also reports a button) erases
    e.pointer_event(DOWN, 1, 40.0, 40.0, PEN, 0, 100.0);
    e.pointer_event(UP, 1, 40.0, 40.0, PEN, 0, 130.0);
    assert_ne!(e.doc().to_json(), doc);
    let f = e.pointer_event(
        DOWN,
        1,
        40.0,
        40.0,
        PEN,
        SECONDARY | mzdraw::editor_touch::ERASER,
        200.0,
    );
    assert_eq!(f & MENU, 0);
}
