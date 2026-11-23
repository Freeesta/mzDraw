//! Reaction arrows, plus signs and texts: objects of the document next to the molecules, with the same exact undo, selection,
//! clipboard and painting as atoms.

use mzdraw::editor::{ASK_TEXT, CHANGED, HANDLED, MOUSE, SHIFT, TOOL};
use mzdraw::{svg, Document, Editor, ObjKind, Op, Session};

fn click(e: &mut Editor, x: f64, y: f64) -> u32 {
    e.pointer_down(x, y, MOUSE, 0);
    e.pointer_up(x, y)
}

fn drag(e: &mut Editor, from: (f64, f64), to: (f64, f64)) -> u32 {
    e.pointer_down(from.0, from.1, MOUSE, 0);
    e.pointer_move((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0, MOUSE);
    e.pointer_move(to.0, to.1, MOUSE);
    e.pointer_up(to.0, to.1)
}

fn obj(t: ObjKind, x: f64, y: f64, x2: f64, y2: f64, text: &str) -> Op {
    Op::ObjAdd {
        id: None,
        t,
        x,
        y,
        x2,
        y2,
        text: text.into(),
        runs: Vec::new(),
        from: 0,
        to: 0,
        cv: Default::default(),
    }
}

#[test]
fn objects_are_added_set_moved_removed_and_undone() {
    let mut s = Session::default();
    let empty = s.doc.to_json();
    s.run(&[
        obj(ObjKind::Arrow, 0.0, 0.0, 2.0, 0.0, ""),
        obj(ObjKind::Plus, -1.0, 0.0, 0.0, 0.0, ""),
        obj(ObjKind::Text, 0.0, -1.0, 0.0, 0.0, "25 °C, 2 h"),
    ])
    .unwrap();
    assert_eq!(
        s.doc.to_json(),
        r#"{"format":"mzdraw/1","next":4,"atoms":[],"bonds":[],"objects":[{"id":1,"t":"arrow","x":0.0,"y":0.0,"x2":2.0},{"id":2,"t":"plus","x":-1.0,"y":0.0},{"id":3,"t":"text","x":0.0,"y":-1.0,"text":"25 °C, 2 h"}]}"#
    );
    assert_eq!(Document::from_json(&s.doc.to_json()).unwrap(), s.doc);
    let set = |id, text: Option<&str>, x2| Op::ObjSet {
        id,
        text: text.map(str::to_string),
        runs: None,
        x2,
        y2: None,
        from: None,
        to: None,
        cv: Default::default(),
        pt: [None, None],
    };
    s.run(&[set(1, Some("Pd/C, H2"), Some(3.0))]).unwrap();
    assert_eq!(
        (s.doc.objects[0].text.as_str(), s.doc.objects[0].x2),
        ("Pd/C, H2", 3.0)
    );
    s.run(&[Op::Move {
        atoms: vec![],
        objects: vec![1, 3],
        dx: 1.0,
        dy: 0.5,
    }])
    .unwrap();
    let o = &s.doc.objects;
    assert_eq!((o[0].x, o[0].y, o[0].x2, o[0].y2), (1.0, 0.5, 4.0, 0.5));
    assert_eq!((o[2].x, o[2].y, o[2].x2, o[2].y2), (1.0, -0.5, 0.0, 0.0));
    // refusals change nothing
    let before = s.doc.to_json();
    for (op, key) in [
        (
            obj(ObjKind::Arrow, 1.0, 1.0, 1.0, 1.0, ""),
            "error.draw2.object",
        ),
        (
            obj(ObjKind::Plus, 0.0, 0.0, 0.0, 0.0, "no"),
            "error.draw2.object",
        ),
        (
            obj(ObjKind::Text, 0.0, 0.0, 0.0, 0.0, "  "),
            "error.draw2.object",
        ),
        (
            obj(ObjKind::Text, 0.0, 0.0, 0.0, 0.0, "two\nlines"),
            "error.draw2.object",
        ),
        (set(3, Some(""), None), "error.draw2.object"),
        (set(2, Some("text on a plus"), None), "error.draw2.object"),
        (set(9, Some("x"), None), "error.draw2.noObject"),
        (
            Op::Remove {
                atoms: vec![],
                bonds: vec![],
                objects: vec![9],
            },
            "error.draw2.noObject",
        ),
    ] {
        assert_eq!(s.run(&[op]).unwrap_err().key, key);
        assert_eq!(s.doc.to_json(), before);
    }
    s.run(&[Op::Remove {
        atoms: vec![],
        bonds: vec![],
        objects: vec![2],
    }])
    .unwrap();
    assert_eq!(s.doc.objects.len(), 2);
    // the log replays to the same document; the ids come from the one counter
    let mut again = Session::default();
    again.replay(&s.log_json()).unwrap();
    assert_eq!(again.doc.to_json(), s.doc.to_json());
    assert!(s.log_json().contains(r#"{"op":"remove","objects":[2]}"#));
    assert!(s
        .log_json()
        .contains(r#"{"op":"move","objects":[1,3],"dx":1.0,"dy":0.5}"#));
    while s.undo() {}
    assert_eq!(s.doc.to_json(), empty);
}

#[test]
fn a_reaction_is_drawn_with_the_tools_and_painted() {
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    e.fit(900.0, 600.0);
    // a click gives a level arrow of two bond lengths; a nearly level drag is levelled
    assert_eq!(e.set_tool("arrow") & TOOL, TOOL);
    assert_eq!(click(&mut e, 560.0, 300.0) & CHANGED, CHANGED);
    let a = e.doc().objects[0].clone();
    assert_eq!((a.t, a.x2 - a.x, a.y2 - a.y), (ObjKind::Arrow, 2.0, 0.0));
    drag(&mut e, (560.0, 400.0), (680.0, 410.0));
    let b = e.doc().objects[1].clone();
    assert_eq!(b.y2, b.y, "levelled");
    assert_eq!(b.x2 - b.x, 3.0);
    drag(&mut e, (560.0, 450.0), (600.0, 500.0));
    assert_ne!(
        e.doc().objects[2].y2,
        e.doc().objects[2].y,
        "a steep arrow keeps its slope"
    );
    e.undo();
    e.undo();
    e.set_tool("plus");
    click(&mut e, 300.0, 300.0);
    assert_eq!(e.key("t", 0), ASK_TEXT | HANDLED);
    assert_eq!(e.set_tool("text:   "), 0);
    assert_eq!(e.set_tool("text:H2, Pd/C") & TOOL, TOOL);
    assert_eq!(e.tool.name(), "text:H2, Pd/C");
    // on the arrow the text tool sets its text; on an empty spot it places a text
    click(&mut e, 600.0, 300.0);
    assert_eq!(e.doc().objects[0].text, "H2, Pd/C");
    click(&mut e, 560.0, 500.0);
    assert_eq!(e.doc().objects.len(), 3);
    assert_eq!(
        (e.doc().objects[2].t, e.doc().objects[2].text.as_str()),
        (ObjKind::Text, "H2, Pd/C")
    );
    assert_eq!(e.key("A", SHIFT) & TOOL, TOOL);
    assert_eq!(e.tool.name(), "arrow");
    // painted: a shaft and a head, the plus, both texts; the file is wide enough for the text
    let out = svg::to_svg(e.doc());
    assert_eq!(out.matches("<polygon").count(), 1);
    assert_eq!(out.matches(">H2, Pd/C<").count(), 2);
    assert!(out.contains(">+<"));
    let width: f64 = out
        .split("width=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .strip_suffix("pt")
        .expect("the size of the file is written in points")
        .parse()
        .unwrap();
    let text_x: f64 = out
        .rsplit("<text x=\"")
        .next()
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(width > text_x + 60.0, "{width} {text_x}");
    // the molfile holds the molecule only
    assert!(e.doc().to_molfile().contains("  3  2  0"));
    // objects alone are a drawing too
    let mut only = Editor::default();
    only.set_tool("plus");
    click(&mut only, 100.0, 100.0);
    assert!(svg::to_svg(only.doc()).contains(">+<"));
    assert_eq!(only.fit(400.0, 300.0) & 1, 1);
    assert_eq!(only.clear() & CHANGED, CHANGED);
    assert!(only.doc().is_empty());
}

#[test]
fn objects_are_selected_dragged_erased_copied_and_pasted() {
    let mut e = Editor::default();
    e.import("CC").unwrap();
    e.fit(900.0, 600.0);
    e.set_tool("arrow");
    click(&mut e, 560.0, 300.0);
    e.set_tool("text:reflux");
    click(&mut e, 560.0, 380.0);
    e.set_tool("select");
    // hover, select, drag
    e.pointer_move(600.0, 300.0, MOUSE);
    assert_eq!(e.over.hover_obj, Some(e.doc().objects[0].id));
    click(&mut e, 600.0, 300.0);
    assert_eq!(e.over.sel_objs, vec![e.doc().objects[0].id]);
    let x = e.doc().objects[0].x;
    drag(&mut e, (600.0, 300.0), (640.0, 300.0));
    assert_eq!(e.doc().objects[0].x - x, 1.0);
    e.undo();
    // a text is hit anywhere along its line
    click(&mut e, 590.0, 380.0);
    assert_eq!(e.over.sel_objs, vec![e.doc().objects[1].id]);
    // everything: copy, paste, the copy is the selection
    e.select_all();
    assert_eq!((e.over.sel_atoms.len(), e.over.sel_objs.len()), (2, 2));
    assert!(e.copy());
    e.paste(None);
    assert_eq!((e.doc().atoms.len(), e.doc().objects.len()), (4, 4));
    assert_eq!(e.over.sel_objs.len(), 2);
    let (o, c) = (e.doc().objects[0].clone(), e.doc().objects[2].clone());
    assert_eq!(
        (c.x - o.x, c.y - o.y, c.x2 - o.x2, c.y2 - o.y2),
        (0.5, -0.5, 0.5, -0.5)
    );
    assert_eq!(e.doc().objects[3].text, "reflux");
    e.undo();
    // a band takes the objects whose anchor it holds; Delete removes them with the atoms
    e.over.sel_atoms.clear();
    e.over.sel_bonds.clear();
    e.over.sel_objs.clear();
    drag(&mut e, (540.0, 270.0), (700.0, 420.0));
    assert_eq!(e.over.sel_objs.len(), 2);
    assert_eq!(e.delete_selection() & CHANGED, CHANGED);
    assert_eq!((e.doc().atoms.len(), e.doc().objects.len()), (2, 0));
    e.undo();
    // the eraser takes one object
    e.over.sel_objs.clear();
    e.set_tool("erase");
    click(&mut e, 600.0, 300.0);
    assert_eq!(e.doc().objects.len(), 1);
    assert_eq!(e.doc().objects[0].t, ObjKind::Text);
}
