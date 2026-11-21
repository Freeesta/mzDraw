//! The explicit colour of atoms and bonds: the operation `colour` (exact undo, canonical text, refusals), the field in the document
//! (a document without it reads as before, an unknown value is kept), the editor command on a selection, the display list, SVG,
//! PDF and CDXML, the loss notes, the context menu, and the cost of the new code.

use mzdraw::doc::Document;
use mzdraw::editor::{CHANGED, REPAINT};
use mzdraw::{Editor, Op, Session};

#[path = "support/timing.rs"]
mod timing;

const DOC: &str = r##"{"format":"mzdraw/1","next":8,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0},{"id":2,"el":"C","x":1.0,"y":0.5},{"id":3,"el":"O","x":2.0,"y":0.0},{"id":4,"el":"N","x":1.0,"y":1.5}],"bonds":[{"id":5,"a":1,"b":2,"o":1},{"id":6,"a":2,"b":3,"o":2},{"id":7,"a":2,"b":4,"o":1}]}"##;

fn session() -> Session {
    Session::new(Document::from_json(DOC).unwrap())
}

fn colour(atoms: &[u32], bonds: &[u32], col: &str) -> Op {
    Op::Colour {
        atoms: atoms.into(),
        bonds: bonds.into(),
        objects: Vec::new(),
        col: col.into(),
    }
}

#[test]
fn the_text_of_a_colour_is_canonical_and_anything_else_is_refused() {
    use mzdraw::ink::canon;
    assert_eq!(canon("#D00000").as_deref(), Some("#d00000"));
    assert_eq!(canon("#0F8").as_deref(), Some("#00ff88"));
    assert_eq!(canon("  #1f3fd0 ").as_deref(), Some("#1f3fd0"));
    assert_eq!(canon("").as_deref(), Some(""));
    for bad in [
        "red",
        "#12345",
        "#1234567",
        "d00000",
        "#gg0000",
        "#",
        "rgb(1,2,3)",
    ] {
        assert_eq!(canon(bad), None, "{bad}");
    }
}

#[test]
fn the_index_of_the_display_list_and_back() {
    use mzdraw::ink::{hex_of, index};
    for hex in ["#000000", "#ffffff", "#d00000", "#00ff88"] {
        let i = index(hex).unwrap();
        assert!(i < 0.0, "an explicit colour is a negative index");
        assert_eq!(hex_of(i).as_deref(), Some(hex));
    }
    // a token of the surface is not a colour
    assert_eq!(hex_of(0.0), None);
    assert_eq!(hex_of(12.0), None);
    assert_eq!(hex_of(f32::NAN), None);
    // every colour of the palette keeps its name
    for (key, _, hex) in mzdraw::ink::PALETTE {
        assert_eq!(mzdraw::ink::name(hex), Some(key));
    }
}

#[test]
fn colour_sets_clears_and_undoes_exactly() {
    let mut s = session();
    let before = s.doc.to_json();
    s.run(&[colour(&[1, 2], &[5], "#D00000")]).unwrap();
    assert_eq!(s.doc.atom(1).unwrap().col(), "#d00000");
    assert_eq!(s.doc.bond(5).unwrap().col(), "#d00000");
    assert_eq!(s.doc.atom(3).unwrap().col(), "");
    // the field is in the document, after the known fields
    assert!(s.doc.to_json().contains(r##""col":"#d00000""##));
    // the resolved operation is in the log with the canonical colour
    assert!(s.log_json().contains(r##""col":"#d00000""##));
    s.run(&[colour(&[1], &[], "")]).unwrap();
    assert_eq!(s.doc.atom(1).unwrap().col(), "");
    assert!(!s.doc.atom(1).unwrap().more.iter().any(|m| m.0 == "col"));
    assert!(s.undo());
    assert_eq!(s.doc.atom(1).unwrap().col(), "#d00000");
    assert!(s.undo());
    assert_eq!(s.doc.to_json(), before);
    assert!(s.redo() && s.redo());
    assert_eq!(s.doc.atom(2).unwrap().col(), "#d00000");
    assert_eq!(s.doc.atom(1).unwrap().col(), "");
}

#[test]
fn a_refused_colour_changes_nothing() {
    let mut s = session();
    let before = s.doc.to_json();
    for (op, key) in [
        (colour(&[1], &[], "red"), "error.draw2.colour"),
        (colour(&[1], &[], "#12345"), "error.draw2.colour"),
        (colour(&[1, 99], &[], "#000000"), "error.draw2.noAtom"),
        (colour(&[1], &[1], "#000000"), "error.draw2.noBond"),
        (
            Op::Colour {
                atoms: vec![],
                bonds: vec![],
                objects: vec![99],
                col: "#000000".into(),
            },
            "error.draw2.noObject",
        ),
    ] {
        assert_eq!(s.run(&[op]).unwrap_err().key, key);
        assert_eq!(s.doc.to_json(), before, "{key}: all or nothing");
    }
    assert_eq!(s.steps(), 0);
}

#[test]
fn a_document_without_a_colour_reads_and_writes_as_before_and_a_colour_survives_the_round() {
    let d = Document::from_json(DOC).unwrap();
    assert!(!d.to_json().contains("\"col\""));
    assert!(d.atoms.iter().all(|a| a.col().is_empty()));
    let mut s = Session::new(d);
    s.run(&[colour(&[3], &[6], "#1f3fd0")]).unwrap();
    let again = Document::from_json(&s.doc.to_json()).unwrap();
    assert_eq!(again, s.doc);
    assert_eq!(again.atom(3).unwrap().col(), "#1f3fd0");
    // a value that is not a colour is not one, and is kept as it was read (an older or a newer file)
    let odd = DOC.replacen(r#""id":1,"el":"C""#, r#""id":1,"col":["x"],"el":"C""#, 1);
    let odd = Document::from_json(&odd).unwrap();
    assert_eq!(odd.atom(1).unwrap().col(), "");
    assert!(odd.to_json().contains(r#""col":["x"]"#));
}

#[test]
fn the_editor_colours_the_selection_in_one_step_and_a_second_click_changes_nothing() {
    let mut e = Editor::default();
    e.open(DOC).unwrap();
    e.over.sel_atoms = vec![1, 2];
    let f = e.colour("#d00000");
    assert!(f & CHANGED != 0);
    // the bond between the two selected atoms is part of the selection
    assert_eq!(e.doc().bond(5).unwrap().col(), "#d00000");
    assert_eq!(e.doc().bond(6).unwrap().col(), "");
    assert_eq!(e.session.steps(), 1);
    assert_eq!(e.said.0, "draw2.say.coloured");
    // the same colour again: no new step
    let f = e.colour("#D00000");
    assert!(f & CHANGED == 0 && f & REPAINT != 0);
    assert_eq!(e.session.steps(), 1);
    assert_eq!(e.selection_colour().as_deref(), Some("#d00000"));
    // automatic gives the element colour back
    assert!(e.colour("") & CHANGED != 0);
    assert_eq!(e.selection_colour().as_deref(), Some(""));
    assert!(e.undo() & CHANGED != 0);
    assert_eq!(e.selection_colour().as_deref(), Some("#d00000"));
    // a bond alone
    e.over.sel_atoms.clear();
    e.over.sel_bonds = vec![7];
    assert!(e.colour("#0f8") & CHANGED != 0);
    assert_eq!(e.doc().bond(7).unwrap().col(), "#00ff88");
    assert_eq!(e.doc().atom(4).unwrap().col(), "");
    // not a colour, nothing selected
    e.over.sel_bonds.clear();
    assert!(e.colour("red") & CHANGED == 0);
    assert_eq!(e.error, "error.draw2.colour");
    assert!(e.colour("#000000") & CHANGED == 0);
    assert_eq!(e.error, "error.draw2.colourNone");
    // a mixed selection has no common colour
    e.over.sel_atoms = vec![1, 3];
    assert_eq!(e.selection_colour(), None);
}

#[test]
fn the_keyboard_focus_is_colourable_with_no_selection() {
    let mut e = Editor::default();
    e.open(DOC).unwrap();
    e.over.focus = Some(3);
    assert!(e.colour("#7b2fbe") & CHANGED != 0);
    assert_eq!(e.doc().atom(3).unwrap().col(), "#7b2fbe");
    assert_eq!(e.doc().atom(1).unwrap().col(), "");
}

/// The colour tokens of the lines and texts of a display list, in order.
fn colours(e: &Editor) -> Vec<f32> {
    let (d, mut out, mut i) = (e.scene().data, Vec::new(), 0);
    while i < d.len() {
        if d[i] == mzdraw::scene::LINE {
            out.push(d[i + 6]);
            i += 7;
        } else if d[i] == mzdraw::scene::TEXT {
            out.push(d[i + 4]);
            i += 7;
        } else if d[i] == mzdraw::scene::TRI {
            i += 8;
        } else if d[i] == mzdraw::scene::RECT {
            i += 6;
        } else {
            i += 5;
        }
    }
    out
}

#[test]
fn an_explicit_colour_overrides_the_element_colour_and_mono_does_not_erase_it() {
    let mut e = Editor::default();
    e.open(DOC).unwrap();
    let plain = colours(&e);
    // the oxygen is drawn in its element colour, the nitrogen too
    assert!(plain.contains(&mzdraw::scene::colour::O) && plain.contains(&mzdraw::scene::colour::N));
    e.over.sel_atoms = vec![3];
    e.colour("#00ff88");
    let green = mzdraw::ink::index("#00ff88").unwrap();
    let with = colours(&e);
    assert!(with.contains(&green));
    assert!(
        !with.contains(&mzdraw::scene::colour::O),
        "the explicit colour replaces the element colour"
    );
    assert!(
        with.contains(&mzdraw::scene::colour::N),
        "the other atoms keep theirs"
    );
    // mono: the element colours go, the explicit one stays
    e.set("style.colour=mono").unwrap();
    let mono = colours(&e);
    assert!(
        mono.contains(&green),
        "mono does not erase an explicit colour"
    );
    assert!(!mono.contains(&mzdraw::scene::colour::N));
    let svg = e.svg();
    assert!(svg.contains("fill=\"#00ff88\""));
    // automatic: back to the ink in mono, to the element colour with the element style
    e.colour("");
    assert!(!colours(&e).contains(&green));
    e.set("style.colour=element").unwrap();
    assert!(colours(&e).contains(&mzdraw::scene::colour::O));
}

#[test]
fn a_bond_is_drawn_in_its_colour_in_every_stroke_and_the_selection_halo_is_not_a_colour() {
    let mut e = Editor::default();
    e.open(DOC).unwrap();
    e.over.sel_bonds = vec![6];
    e.colour("#d00000");
    // the double bond is two lines, both red; the halo of the selection keeps its own token
    let red = mzdraw::ink::index("#d00000").unwrap();
    assert_eq!(colours(&e).iter().filter(|c| **c == red).count(), 2);
    assert!(colours(&e).contains(&mzdraw::scene::colour::SELECT));
    let svg = e.svg();
    assert_eq!(svg.matches("stroke=\"#d00000\"").count(), 2);
}

#[test]
fn pdf_and_cdxml_carry_the_colour_and_the_other_files_say_what_they_left_out() {
    let mut e = Editor::default();
    e.open(DOC).unwrap();
    e.over.sel_atoms = vec![2, 3];
    e.colour("#d00000");
    // PDF: an RGB operator of the colour
    let pdf = String::from_utf8_lossy(&e.pdf()).to_string();
    assert!(
        pdf.contains("0.816 0 0 RG") || pdf.contains("0.816 0 0 rg"),
        "{}",
        &pdf[..pdf.len().min(400)]
    );
    // CDXML: a colour table and the colour of the nodes and bonds, read back
    let x = e.save("cdxml").unwrap();
    assert!(
        x.contains("<colortable><color r=\"1\" g=\"1\" b=\"1\"/><color r=\"0\" g=\"0\" b=\"0\"/><color r=\"0.816\" g=\"0\" b=\"0\"/></colortable>"),
        "{x}"
    );
    assert_eq!(
        x.matches(" color=\"2\"").count(),
        3,
        "two atoms and the bond between them"
    );
    let mut back = Editor::default();
    back.import(&x).unwrap();
    let coloured: Vec<String> = back.doc().atoms.iter().map(|a| a.col()).collect();
    assert_eq!(coloured, ["", "#d00000", "#d00000", ""]);
    assert_eq!(
        back.doc()
            .bonds
            .iter()
            .filter(|b| b.col() == "#d00000")
            .count(),
        1
    );
    // the files with no place for it: the note names the count; the carriers say nothing
    for kind in ["molfile", "molfile-v3000", "smiles", "sdf", "ket", "cml"] {
        assert_eq!(
            e.lost_on_save(kind),
            [("draw2.export.coloursLost", 3)],
            "{kind}"
        );
    }
    for kind in ["cdxml", "svg", "pdf", "mzdraw"] {
        assert!(e.lost_on_save(kind).is_empty(), "{kind}");
    }
    // the loss manifest agrees: it lists the coloured things for KET and nothing for CDXML
    let loss = e.save("loss:ket").unwrap();
    assert!(loss.contains("\"what\":\"colour\""));
    assert!(e.save("loss:cdxml").unwrap().contains("\"items\":[]"));
}

#[test]
fn the_svg_carries_the_document_so_the_colour_comes_back() {
    let mut e = Editor::default();
    e.open(DOC).unwrap();
    e.over.sel_atoms = vec![4];
    e.colour("#7b2fbe");
    let svg = e.svg();
    let mut back = Editor::default();
    back.import(&svg).unwrap();
    assert_eq!(back.doc().atom(4).unwrap().col(), "#7b2fbe");
}

#[test]
fn copy_and_paste_keep_the_colour_and_a_diff_says_it() {
    let mut e = Editor::default();
    e.open(DOC).unwrap();
    e.over.sel_atoms = vec![2, 3];
    e.colour("#1f3fd0");
    assert!(e.copy());
    e.paste(None);
    let blue: Vec<_> = e
        .doc()
        .atoms
        .iter()
        .filter(|a| a.col() == "#1f3fd0")
        .collect();
    assert_eq!(blue.len(), 4, "the copy is blue as well");
    assert_eq!(
        e.doc()
            .bonds
            .iter()
            .filter(|b| b.col() == "#1f3fd0")
            .count(),
        2
    );
    // the patch between two documents says a colour with the operation
    let a = Document::from_json(DOC).unwrap();
    let mut b = a.clone();
    let mut s = Session::new(a.clone());
    s.run(&[colour(&[1], &[5], "#d00000")]).unwrap();
    b.atoms = s.doc.atoms.clone();
    b.bonds = s.doc.bonds.clone();
    let patch = mzdraw::diff::diff(&a, &b);
    assert_eq!(patch.len(), 2);
    assert!(patch.iter().all(|op| matches!(op, Op::Colour { .. })));
    let mut t = Session::new(a);
    t.run(&patch).unwrap();
    assert_eq!(t.doc, b);
}

#[test]
fn the_context_menu_offers_the_colours_on_each_target_and_runs_one_as_one_step() {
    let mut e = Editor::default();
    e.open(DOC).unwrap();
    let px = |e: &Editor, id| {
        let a = e.doc().atom(id).unwrap();
        e.view.to_px(a.x, a.y)
    };
    let (x, y) = px(&e, 3);
    let menu: serde_json::Value = serde_json::from_str(&e.context_menu(x, y)).unwrap();
    assert_eq!(menu["target"], "atom");
    let group = menu["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == "edit.colour")
        .unwrap();
    let kids: Vec<&str> = group["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["id"].as_str().unwrap())
        .collect();
    assert_eq!(kids.first(), Some(&"colour.auto"));
    assert_eq!(kids.last(), Some(&"colour.custom"));
    assert_eq!(kids.len(), mzdraw::ink::PALETTE.len() + 2);
    let steps = e.session.steps();
    e.context_run("colour.blue");
    assert_eq!(e.session.steps(), steps + 1);
    assert_eq!(e.doc().atom(3).unwrap().col(), "#1f3fd0");
    // the item that asks for a custom colour is the page's to finish
    let (x, y) = px(&e, 3);
    e.context_menu(x, y);
    assert!(e.context_run("colour.custom") & mzdraw::editor::ASK != 0);
    assert_eq!(e.context_ask(), "colour");
    // the colour in use is checked
    let menu: serde_json::Value = serde_json::from_str(&e.context_menu(x, y)).unwrap();
    let group = menu["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == "edit.colour")
        .unwrap();
    let checked: Vec<&str> = group["children"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|k| k["checked"] == true)
        .map(|k| k["id"].as_str().unwrap())
        .collect();
    assert_eq!(checked, ["colour.blue"]);
    // a bond and a selection have the group too
    e.over.sel_atoms = vec![1, 2];
    let (x, y) = px(&e, 1);
    let menu: serde_json::Value = serde_json::from_str(&e.context_menu(x, y)).unwrap();
    assert_eq!(menu["target"], "selection");
    assert!(menu["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["id"] == "edit.colour"));
}

/// A chain of `n` carbons.
fn chain(n: u32) -> Document {
    let mut s = String::from(r#"{"format":"mzdraw/1","next":"#);
    s += &(2 * n + 1).to_string();
    s += r#","atoms":["#;
    let atoms: Vec<String> = (1..=n)
        .map(|i| {
            format!(
                r#"{{"id":{i},"el":"C","x":{},"y":{}}}"#,
                i as f64,
                (i % 2) as f64 * 0.5
            )
        })
        .collect();
    s += &atoms.join(",");
    s += r#"],"bonds":["#;
    let bonds: Vec<String> = (1..n)
        .map(|i| format!(r#"{{"id":{},"a":{i},"b":{},"o":1}}"#, n + i, i + 1))
        .collect();
    s += &bonds.join(",");
    s += "]}";
    Document::from_json(&s).unwrap()
}

/// The new code (the operation, its undo and the writers of the field) costs time in proportion to the number of things coloured:
/// twice the atoms take less than three times as long (a quadratic version, one search of the pool per atom, takes four).
#[test]
fn colouring_grows_with_the_size_not_with_its_square() {
    let cost = |n: u32| {
        let d = chain(n);
        let atoms: Vec<u32> = d.atoms.iter().map(|a| a.id).collect();
        let bonds: Vec<u32> = d.bonds.iter().map(|b| b.id).collect();
        timing::best_of(1, 7, || {
            let mut s = Session::new(d.clone());
            s.run(&[colour(&atoms, &bonds, "#d00000")]).unwrap();
            let _ = s.doc.to_json();
            assert!(s.undo());
        })
    };
    let (small, big) = (cost(20_000), cost(40_000));
    eprintln!("20,000 atoms {small:?}, 40,000 atoms {big:?}");
    assert!(
        big.as_secs_f64() < 3.0 * small.as_secs_f64(),
        "20,000 atoms {small:?}, 40,000 atoms {big:?}"
    );
}

/// A drawing with an arrow, a plus and a text next to the molecule: ids 8, 9 and 10.
const WITH_OBJECTS: &str = r##"{"format":"mzdraw/1","next":11,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0},{"id":2,"el":"O","x":1.0,"y":0.5}],"bonds":[{"id":3,"a":1,"b":2,"o":1}],"objects":[{"id":8,"t":"arrow","x":3.0,"y":0.0,"x2":5.0,"y2":0.0,"text":"H2"},{"id":9,"t":"plus","x":6.0,"y":0.0},{"id":10,"t":"text","x":7.0,"y":0.0,"text":"note"}]}"##;

#[test]
fn one_operation_colours_atoms_bonds_and_objects_and_undoes_exactly() {
    let mut s = Session::new(Document::from_json(WITH_OBJECTS).unwrap());
    let before = s.doc.to_json();
    s.run(&[Op::Colour {
        atoms: vec![2],
        bonds: vec![3],
        objects: vec![8, 10],
        col: "#7B2FBE".into(),
    }])
    .unwrap();
    assert_eq!(s.steps(), 1, "one step for all three kinds");
    assert_eq!(s.doc.object(8).unwrap().col(), "#7b2fbe");
    assert_eq!(s.doc.object(9).unwrap().col(), "");
    assert!(s.log_json().contains(r##""objects":[8,10]"##));
    // the field round trips through the document text
    assert_eq!(Document::from_json(&s.doc.to_json()).unwrap(), s.doc);
    assert!(s.undo());
    assert_eq!(s.doc.to_json(), before);
}

#[test]
fn objects_are_drawn_in_their_colour_with_the_arrow_head_and_the_text() {
    let mut e = Editor::default();
    e.open(WITH_OBJECTS).unwrap();
    let ink = |e: &Editor| {
        colours(e)
            .iter()
            .filter(|c| **c == mzdraw::scene::colour::INK)
            .count()
    };
    let plain = ink(&e);
    e.over.sel_objs = vec![8, 9, 10];
    assert!(e.colour("#d00000") & CHANGED != 0);
    assert_eq!(e.selection_colour().as_deref(), Some("#d00000"));
    let red = mzdraw::ink::index("#d00000").unwrap();
    // the shaft, the text above the arrow, the plus and the text are red; the head is a triangle (checked in the SVG)
    assert!(colours(&e).iter().filter(|c| **c == red).count() >= 4);
    assert!(ink(&e) < plain);
    let svg = e.svg();
    assert!(svg.contains("<polygon") && svg.contains("fill=\"#d00000\""));
    assert!(svg.matches("#d00000").count() >= 5, "{svg}");
}

#[test]
fn digest_names_the_colours_and_a_drawing_without_any_is_unchanged() {
    let mut e = Editor::default();
    e.open(WITH_OBJECTS).unwrap();
    let plain = e.save("digest").unwrap();
    assert!(!plain.contains("colours") && !plain.contains("#"));
    e.over.sel_atoms = vec![2];
    e.over.sel_bonds = vec![3];
    e.over.sel_objs = vec![8];
    e.colour("#d00000");
    let d: serde_json::Value = serde_json::from_str(&e.save("digest").unwrap()).unwrap();
    assert_eq!(
        d["pieces"][0]["colours"],
        serde_json::json!([["#d00000", "2", "3"]])
    );
    assert_eq!(d["objects"][0][3], "#d00000");
    assert_eq!(
        d["objects"][1].as_array().unwrap().len(),
        3,
        "an object with no colour says none"
    );
}

#[test]
fn the_loss_notes_count_objects_and_cdxml_leaves_only_them_out() {
    let mut e = Editor::default();
    e.open(WITH_OBJECTS).unwrap();
    e.over.sel_atoms = vec![1, 2];
    e.over.sel_objs = vec![8];
    e.colour("#d00000");
    // atoms 1 and 2, the bond between them and the arrow
    assert_eq!(e.lost_on_save("molfile"), [("draw2.export.coloursLost", 4)]);
    assert_eq!(e.lost_on_save("cdxml"), [("draw2.export.coloursLost", 1)]);
    assert!(e.lost_on_save("svg").is_empty() && e.lost_on_save("mzdraw").is_empty());
    let cdxml = e.save("loss:cdxml").unwrap();
    assert!(cdxml.contains(r#""what":"colour.object""#) && !cdxml.contains(r#""what":"colour""#));
    assert!(e
        .save("loss:ket")
        .unwrap()
        .contains(r#""what":"colour.object""#));
    // the context menu of an object has the group
    let (x, y) = {
        let o = e.doc().object(9).unwrap();
        e.view.to_px(o.x, o.y)
    };
    let menu: serde_json::Value = serde_json::from_str(&e.context_menu(x, y)).unwrap();
    assert_eq!(menu["target"], "object");
    assert!(menu["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["id"] == "edit.colour"));
    e.context_run("colour.green");
    assert_eq!(e.doc().object(9).unwrap().col(), "#1a7f1a");
}

#[test]
fn a_colour_field_of_a_newer_file_is_kept_and_never_an_error() {
    // a structured value, a wrong type, a long text: not colours, read as automatic, written back as they were
    for odd in [r#"{"a":1}"#, "7", r#""not a colour""#, "null"] {
        let text = WITH_OBJECTS.replacen(r#""id":8,"#, &format!(r#""id":8,"col":{odd},"#), 1);
        let d = Document::from_json(&text).unwrap();
        assert_eq!(d.object(8).unwrap().col(), "");
        assert!(d.to_json().contains(&format!(r#""col":{odd}"#)), "{odd}");
    }
}

#[test]
fn cdxml_lists_white_and_black_first_and_reads_the_table_as_written() {
    let mut e = Editor::default();
    e.open(DOC).unwrap();
    e.over.sel_atoms = vec![3];
    e.colour("#d00000");
    let x = e.save("cdxml").unwrap();
    // entries 0 and 1 are white and black, as in tests/data/foreign.cdxml; the first colour of the drawing is entry 2
    assert!(x.contains(r#"<colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/><color r="0.816" g="0" b="0"/></colortable>"#), "{x}");
    assert!(x.contains(r#"color="2""#));
    // a file of another program, with its own table: entry 2 is its first colour, entry 0 and 1 are not colours of the drawing
    let foreign = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="30"><colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/><color r="0" g="0" b="1"/><color r="0" g="1" b="0"/></colortable>
<page><fragment id="1"><n id="2" p="30 30" color="3"/><n id="3" p="60 30" color="2"/><n id="4" p="90 30" color="1"/><n id="5" p="120 30" color="0"/><n id="6" p="150 30" color="9"/>
<b id="7" B="2" E="3" color="2"/><b id="8" B="3" E="4"/></fragment></page></CDXML>"#;
    let mut back = Editor::default();
    back.import(foreign).unwrap();
    let cols: Vec<String> = back.doc().atoms.iter().map(|a| a.col()).collect();
    assert_eq!(
        cols,
        ["#00ff00", "#0000ff", "", "", ""],
        "white and black are the default ink; an entry that is missing is automatic"
    );
    assert_eq!(back.doc().bonds[0].col(), "#0000ff");
}

#[test]
fn a_value_that_is_not_a_colour_is_listed_as_lost_by_the_formats_without_a_place_for_it() {
    for odd in [r#""rgba(1,2,3,1)""#, r#"{"r":1}"#, r#""red""#] {
        let text = DOC
            .replacen(
                r#""id":1,"el":"C""#,
                &format!(r#""id":1,"col":{odd},"el":"C""#),
                1,
            )
            .replacen(r#""id":5,"#, &format!(r#""id":5,"col":{odd},"#), 1);
        let d = Document::from_json(&text).unwrap();
        let said = mzdraw::loss::loss(&d, "molfile").unwrap();
        let unknown: Vec<(&[u32], &[u32])> = (said.iter())
            .filter(|l| l.what == "unknown")
            .map(|l| (&l.atoms[..], &l.bonds[..]))
            .collect();
        assert_eq!(
            unknown,
            [(&[1][..], &[][..]), (&[][..], &[5][..])],
            "{odd}: {said:?}"
        );
        assert!(!said.iter().any(|l| l.what == "colour"));
    }
    // a colour has its own row and is not listed twice
    let mut s = session();
    s.run(&[colour(&[1], &[], "#d00000")]).unwrap();
    let said = mzdraw::loss::loss(&s.doc, "molfile").unwrap();
    assert!(said.iter().any(|l| l.what == "colour") && !said.iter().any(|l| l.what == "unknown"));
}

#[test]
fn every_text_of_the_colour_picker_exists_in_the_catalog() {
    let lang = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/lang/mzdraw.js"),
    )
    .unwrap();
    // the English catalog as (key, text) pairs: the lines `"key": "text",`
    let catalog: std::collections::BTreeMap<String, String> = lang
        .lines()
        .filter_map(|l| {
            let l = l.trim().trim_end_matches(',');
            let (k, v) = l.split_once("\": \"")?;
            Some((
                k.strip_prefix('"')?.to_string(),
                v.strip_suffix('"')?.to_string(),
            ))
        })
        .collect();
    // every key of the colour picker is in the catalog with a text: the families, then the keys the module and the page use
    let family = |p: &str| catalog.keys().filter(|k| k.starts_with(p)).count();
    assert!(family("draw2.colour.") >= 20 && family("draw2.ctx.colour.") >= 12);
    for (k, v) in &catalog {
        if k.starts_with("draw2.colour.") || k.starts_with("draw2.ctx.colour.") {
            assert!(!v.trim().is_empty(), "{k}");
        }
    }
    let mut keys: Vec<String> = [
        "draw2.tool.colour",
        "draw2.tool.colour.title",
        "draw2.fly.colour",
        "draw2.say.coloured",
        "draw2.ctx.edit.colour",
        "draw2.ctx.say.colour",
        "error.draw2.colour",
        "error.draw2.colourNone",
        "draw2.export.coloursLost",
        "draw2.colour.current",
        "draw2.colour.mixed",
        "draw2.colour.customName",
    ]
    .map(String::from)
    .to_vec();
    for (key, name, _) in mzdraw::ink::PALETTE {
        keys.push(format!("draw2.colour.{key}"));
        keys.push(format!("draw2.ctx.colour.{key}"));
        // the English text is the module's own name
        assert_eq!(catalog[&format!("draw2.colour.{key}")], name);
        assert_eq!(catalog[&format!("draw2.ctx.colour.{key}")], name);
    }
    for k in ["auto", "custom"] {
        keys.push(format!("draw2.ctx.colour.{k}"));
    }
    for k in &keys {
        assert!(catalog.get(k).is_some_and(|v| !v.trim().is_empty()), "{k}");
    }
    // the words follow the scope: atoms, bonds and objects
    assert!(catalog["draw2.export.coloursLost"].contains("atoms, bonds and objects"));
    assert!(catalog["error.draw2.colourNone"].contains("atoms, bonds or objects"));
}
