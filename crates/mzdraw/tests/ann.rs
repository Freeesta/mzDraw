//! The annotation of a structure (confidence level, identifier, evidence): its shape, its operation with an exact inverse, the
//! document, the SDF fields, the picture. Synthetic structures and annotations only.

use mzdraw::ann::{self, FIELDS, LEVELS};
use mzdraw::io::import;
use mzdraw::{Document, Editor, Op, Session};

fn editor(text: &str) -> Editor {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e
}

fn anchors(e: &Editor) -> Vec<u32> {
    (mzdraw::outline::pieces(e.doc()).iter())
        .map(|p| p.anchor)
        .collect()
}

#[test]
fn the_level_is_a_string_of_the_scale_and_any_other_string_is_kept() {
    assert_eq!(LEVELS, ["1", "2a", "2b", "3", "4", "5"]);
    for level in LEVELS {
        let text = format!(r#"[{{"level":"{level}"}}]"#);
        assert_eq!(ann::canon(&text).as_deref(), Some(text.as_str()));
    }
    // a level of another scale is not refused: kept as read, and said not to be of the scale
    let other = r#"[{"level":"3a"},{"level":"0"}]"#;
    assert_eq!(ann::canon(other).as_deref(), Some(other));
    let now = ann::current(other).unwrap();
    assert_eq!(
        (now.level.as_str(), now.known, now.entries),
        ("0", false, 2)
    );
    // never a number, never absent, never a text an SDF line could not be
    for bad in [
        r#"[{"level":2}]"#,
        r#"[{"level":null}]"#,
        r#"[{}]"#,
        r#"[{"level":""}]"#,
        r#"[{"level":" 2a"}]"#,
        r#"[{"level":">2a"}]"#,
        r#"[{"level":"2a\n$$$$"}]"#,
        r#"[{"level":"12345678901234567"}]"#,
        r#"{"level":"1"}"#,
        r#"["1"]"#,
        "not json",
    ] {
        assert_eq!(ann::canon(bad), None, "{bad}");
    }
    assert_eq!(ann::canon(" [ ] ").as_deref(), Some("[]"));
}

#[test]
fn an_entry_holds_its_fields_and_keeps_what_it_does_not_know() {
    let full = r#"[{"by":"auto","evidence":[{"kind":"ms2-library","note":"synthetic","ref":"LIB:0001"},{"kind":"rt-standard"}],"future":{"a":[1,2.5,null]},"identifier":"InChIKey=AAAAAAAAAAAAAA-BBBBBBBBBB-N","level":"2a","note":"two\nlines","scale":"schymanski-2014","scores":[{"from":"library search","name":"cosine","v":0.91}],"source":{"run":"r1","tool":"mzlab-cli flow"}}]"#;
    assert_eq!(
        ann::canon(full).as_deref(),
        Some(full),
        "already in its one text"
    );
    // the same entry with its keys in another order and spaces gives the same text
    let loose = r#"[ {"level":"2a", "scale":"schymanski-2014", "identifier":"InChIKey=AAAAAAAAAAAAAA-BBBBBBBBBB-N", "by":"auto",
        "evidence":[{"ref":"LIB:0001","kind":"ms2-library","note":"synthetic"},{"kind":"rt-standard"}], "note":"two\nlines",
        "scores":[{"name":"cosine","v":0.91,"from":"library search"}], "source":{"tool":"mzlab-cli flow","run":"r1"},
        "future":{"a":[1,2.5,null]}} ]"#;
    assert_eq!(ann::canon(loose).as_deref(), Some(full));
    let now = ann::current(full).unwrap();
    assert_eq!(now.identifier, "InChIKey=AAAAAAAAAAAAAA-BBBBBBBBBB-N");
    assert_eq!(now.evidence, "ms2-library: LIB:0001\nrt-standard");
    assert_eq!((now.note.as_str(), now.known), ("two\nlines", true));
    // every part has its shape
    for bad in [
        r#"[{"level":"1","identifier":">x"}]"#,
        r#"[{"level":"1","identifier":"$$$$"}]"#,
        r#"[{"level":"1","identifier":"a\nb"}]"#,
        r#"[{"level":"1","identifier":7}]"#,
        r#"[{"level":"1","evidence":{"kind":"formula"}}]"#,
        r#"[{"level":"1","evidence":[{"ref":"x"}]}]"#,
        r#"[{"level":"1","evidence":[{"kind":"a: b"}]}]"#,
        r#"[{"level":"1","evidence":["formula"]}]"#,
        r#"[{"level":"1","scores":[{"name":"cosine","v":"0.9"}]}]"#,
        r#"[{"level":"1","scores":[{"v":0.9}]}]"#,
        r#"[{"level":"1","source":"tool"}]"#,
        r#"[{"level":"1","note":"a\u0007"}]"#,
        r#"[{"level":"1","by":""}]"#,
    ] {
        assert_eq!(ann::canon(bad), None, "{bad}");
    }
    // the limits: entries, items of a list, the whole text
    let many = |n: usize| format!("[{}]", vec![r#"{"level":"3"}"#; n].join(","));
    assert!(ann::canon(&many(ann::MAX.0)).is_some());
    assert!(ann::canon(&many(ann::MAX.0 + 1)).is_none());
    let items = |n: usize| {
        format!(
            r#"[{{"level":"3","evidence":[{}]}}]"#,
            vec![r#"{"kind":"context"}"#; n].join(",")
        )
    };
    assert!(ann::canon(&items(ann::MAX.1)).is_some());
    assert!(ann::canon(&items(ann::MAX.1 + 1)).is_none());
    let long = format!(r#"[{{"level":"3","future":"{}"}}]"#, "x".repeat(ann::MAX.2));
    assert!(ann::canon(&long).is_none());
}

#[test]
fn the_operation_sets_the_annotation_and_undoes_to_the_same_bytes() {
    let mut s = Session::default();
    s.run(&[Op::AtomAdd {
        id: None,
        el: "C".into(),
        x: 0.0,
        y: 0.0,
    }])
    .unwrap();
    let plain = s.doc.to_json();
    let set = Op::AtomAnn {
        id: 1,
        ann: r#"[{"level":"2b","scale":"schymanski-2014"}]"#.into(),
    };
    s.run(std::slice::from_ref(&set)).unwrap();
    assert_eq!(
        s.doc.atoms[0].ann(),
        r#"[{"level":"2b","scale":"schymanski-2014"}]"#
    );
    let with = s.doc.to_json();
    assert_eq!(
        with,
        r#"{"format":"mzdraw/1","next":2,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0,"ann":[{"level":"2b","scale":"schymanski-2014"}]}],"bonds":[]}"#
    );
    assert_eq!(Document::from_json(&with).unwrap().to_json(), with);
    // in the log as JSON, read back as itself; given loosely, logged in its one text
    assert_eq!(
        set.to_json(),
        r#"{"op":"atom.ann","id":1,"ann":[{"level":"2b","scale":"schymanski-2014"}]}"#
    );
    assert_eq!(serde_json::from_str::<Op>(&set.to_json()).unwrap(), set);
    s.run(&[Op::AtomAnn {
        id: 1,
        ann: r#"[ {"scale":"schymanski-2014", "level":"2b"}, {"level":"1"} ]"#.into(),
    }])
    .unwrap();
    assert!(mzdraw::ops::log_text(&s.log()).ends_with(
        r#"{"op":"atom.ann","id":1,"ann":[{"level":"2b","scale":"schymanski-2014"},{"level":"1"}]}]"#
    ));
    s.run(&[Op::AtomAnn {
        id: 1,
        ann: "[]".into(),
    }])
    .unwrap();
    assert_eq!(s.doc.to_json(), plain, "no annotation, no field");
    assert!(s.undo() && s.undo());
    assert_eq!(s.doc.to_json(), with);
    assert!(s.undo());
    assert_eq!(s.doc.to_json(), plain);
    // refused with its key, nothing changed: a level that is a number, an atom that is none
    let e = (s.run(&[Op::AtomAnn {
        id: 1,
        ann: r#"[{"level":2}]"#.into(),
    }]))
    .unwrap_err();
    assert_eq!((e.key, e.id), ("error.draw2.ann", 1));
    let e = (s.run(&[Op::AtomAnn {
        id: 9,
        ann: "[]".into(),
    }]))
    .unwrap_err();
    assert_eq!(e.key, "error.draw2.noAtom");
    assert_eq!(s.doc.to_json(), plain);
    // an `ann` of another shape in a document is not an annotation: left where it is, written back
    let odd = r#"{"format":"mzdraw/1","next":2,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0,"ann":{"level":2}}],"bonds":[]}"#;
    let d = Document::from_json(odd).unwrap();
    assert_eq!(
        (d.atoms[0].ann(), d.to_json()),
        (String::new(), odd.to_string())
    );
}

#[test]
fn the_editor_annotates_a_structure_as_a_history_in_one_undo_step() {
    let mut e = editor("CCO.c1ccccc1");
    let at = anchors(&e);
    assert_eq!(at.len(), 2);
    let before = e.save("mzdraw").unwrap();
    e.annotate(at[1], "3", "", "formula\nexact-mass: 78.04695", "");
    assert_eq!(
        (e.error, e.said.0, e.said.1.as_str()),
        ("", "draw2.say.ann", "3")
    );
    e.annotate(
        at[1],
        " 2a ",
        " benzene (synthetic) ",
        "ms2-library: LIB:0002\n\n  rt-standard  ",
        " seen twice ",
    );
    let pieces = mzdraw::outline::pieces(e.doc());
    assert_eq!(pieces[0].ann, None);
    let now = pieces[1].ann.clone().unwrap();
    assert_eq!(
        (now.level.as_str(), now.known, now.entries),
        ("2a", true, 2)
    );
    assert_eq!(now.identifier, "benzene (synthetic)");
    assert_eq!(now.evidence, "ms2-library: LIB:0002\nrt-standard");
    assert_eq!(now.note, "seen twice");
    let a = e.doc().atom(at[1]).unwrap().ann();
    assert_eq!(
        a,
        r#"[{"by":"chemist","evidence":[{"kind":"formula"},{"kind":"exact-mass","ref":"78.04695"}],"level":"3","scale":"schymanski-2014"},{"by":"chemist","evidence":[{"kind":"ms2-library","ref":"LIB:0002"},{"kind":"rt-standard"}],"identifier":"benzene (synthetic)","level":"2a","note":"seen twice","scale":"schymanski-2014"}]"#
    );
    // set on another atom of the structure, the history moves to that atom whole
    let other = e
        .doc()
        .atoms
        .iter()
        .map(|a| a.id)
        .find(|&i| i > at[1])
        .unwrap();
    e.annotate(other, "1", "", "", "");
    assert_eq!(e.doc().atom(at[1]).unwrap().ann(), "");
    let pieces = mzdraw::outline::pieces(e.doc());
    assert_eq!(
        pieces[1]
            .ann
            .as_ref()
            .map(|n| (n.level.as_str(), n.entries)),
        Some(("1", 3))
    );
    assert!(e.undo() > 0);
    assert_eq!(e.doc().atom(at[1]).unwrap().ann(), a, "one step back");
    // refused in words, nothing changed: a level that could not be a line, evidence of no kind, an atom that is none
    let held = e.save("mzdraw").unwrap();
    for (level, evidence, anchor) in [
        ("2 a", "", at[1]),
        ("2a", "a b: c", at[1]),
        ("2a", "", 9999),
    ] {
        e.annotate(anchor, level, "", evidence, "");
        assert_eq!(e.error, "error.draw2.ann");
        assert_eq!(e.save("mzdraw").unwrap(), held);
    }
    // an empty level takes the annotation away; undo to the start gives the first bytes
    e.annotate(at[1], "", "", "", "");
    assert_eq!(e.error, "");
    assert_eq!(mzdraw::outline::pieces(e.doc())[1].ann, None);
    while e.undo() > 0 && e.save("mzdraw").unwrap() != before {}
    assert_eq!(e.save("mzdraw").unwrap(), before);
    // copy and paste carries the annotation with the structure
    e.annotate(at[0], "4", "", "formula", "");
    e.select_all();
    e.copy();
    e.paste(None);
    let levels: Vec<Option<String>> = (mzdraw::outline::pieces(e.doc()).into_iter())
        .map(|p| p.ann.map(|n| n.level))
        .collect();
    assert_eq!(
        levels.iter().filter(|l| l.as_deref() == Some("4")).count(),
        2,
        "{levels:?}"
    );
}

#[test]
fn an_sdf_carries_the_annotation_and_reads_it_back() {
    let mut e = editor("CCO.CC(C)=O");
    let at = anchors(&e);
    e.set_data(at[0], vec![("name".into(), "ethanol (synthetic)".into())]);
    e.annotate(at[0], "3", "", "formula", "");
    e.annotate(
        at[0],
        "2b",
        "ethanol",
        "diagnostic-fragment: m/z 31.0178\nrt-standard",
        "a note\nof two lines",
    );
    let sdf = e.save("sdf").unwrap();
    let records: Vec<&str> = sdf.split("$$$$\n").collect();
    assert_eq!(records.len(), 3);
    let whole = e.doc().atom(at[0]).unwrap().ann();
    assert!(records[0].ends_with(&format!(
        "M  END\n>  <name>\nethanol (synthetic)\n\n>  <MZLAB_LEVEL>\n2b\n\n>  <MZLAB_IDENTIFIER>\nethanol\n\n>  <MZLAB_EVIDENCE>\ndiagnostic-fragment: m/z 31.0178\nrt-standard\n\n>  <MZLAB_ANN>\n{whole}\n\n"
    )));
    assert!(records[1].ends_with("M  END\n") && !sdf.contains("\n\n\n"));
    assert_eq!(e.save("sdf").unwrap(), sdf, "the same bytes again");
    // read back: the whole history, and the four fields are not data of the user
    let (d, info) = import(&sdf).unwrap();
    assert_eq!(info.data_left, 0);
    assert_eq!(d.atoms[0].ann(), whole);
    assert_eq!(
        d.atoms[0].data(),
        vec![("name".to_string(), "ethanol (synthetic)".to_string())]
    );
    // written again, each record has the same fields (the structures are set side by side on reading, so their coordinates move)
    let fields = |s: &str| -> Vec<String> {
        s.split("$$$$\n")
            .map(|r| r.split_once("M  END\n").map_or("", |p| p.1).to_string())
            .collect()
    };
    assert_eq!(fields(&d.to_sdf()), fields(&sdf));
    // an SDF of another program with the level alone gives one entry; unknown levels are kept
    let mol = records[1];
    let level_only = format!("{mol}>  <MZLAB_LEVEL>\n3a\n\n>  <MZLAB_IDENTIFIER>\nacetone\n\n>  <MZLAB_EVIDENCE>\nformula\nms2-insilico: tool 1\n\n$$$$\n");
    let (d, info) = import(&level_only).unwrap();
    assert_eq!(info.data_left, 0);
    assert_eq!(
        d.atoms[0].ann(),
        r#"[{"evidence":[{"kind":"formula"},{"kind":"ms2-insilico","ref":"tool 1"}],"identifier":"acetone","level":"3a","scale":"schymanski-2014"}]"#
    );
    // what gives no annotation is counted, never taken silently: no level, a level of no shape, a damaged whole one
    for (fields, left) in [
        (">  <MZLAB_IDENTIFIER>\nacetone\n\n", 1),
        (
            ">  <MZLAB_LEVEL>\nlevel two\n\n>  <MZLAB_EVIDENCE>\nformula\n\n",
            2,
        ),
        (">  <MZLAB_ANN>\n[{\"level\":2}]\n\n", 1),
        (">  <MZLAB_LEVEL>\n2a\n\n>  <MZLAB_LEVEL>\n3\n\n", 1),
    ] {
        let (d, info) = import(&format!("{mol}{fields}$$$$\n")).unwrap();
        assert_eq!(info.data_left, left, "{fields}");
        assert_eq!(d.atoms[0].data(), vec![], "{fields}");
        assert_eq!(
            d.atoms[0].ann().is_empty(),
            left != 1 || !fields.contains("LEVEL"),
            "{fields}"
        );
    }
    // a damaged whole annotation next to a good level: the level is read
    let (d, _) = import(&format!(
        "{mol}>  <MZLAB_ANN>\n[oops\n\n>  <MZLAB_LEVEL>\n4\n\n$$$$\n"
    ))
    .unwrap();
    assert_eq!(
        d.atoms[0].ann(),
        r#"[{"level":"4","scale":"schymanski-2014"}]"#
    );
    // the four names cannot be set as data
    for name in FIELDS {
        assert!(!mzdraw::doc::data_valid(&[(
            name.to_string(),
            "x".to_string()
        )]));
    }
}

#[test]
fn the_level_is_drawn_under_its_structure() {
    let mut e = editor("CCO.c1ccccc1");
    let at = anchors(&e);
    let plain = e.save("svg").unwrap();
    assert!(!plain.contains(">L"));
    e.annotate(at[1], "2b", "", "", "");
    let svg = e.save("svg").unwrap();
    assert_eq!(svg.matches(">L2b</text>").count(), 1);
    // nothing else of the picture moves: the same lines, the same size
    let head = |s: &str| s.lines().next().unwrap().to_string();
    assert_eq!(head(&svg), head(&plain));
    assert_eq!(svg.matches("<line").count(), plain.matches("<line").count());
    // a level of another scale is drawn as it is written
    e.annotate(at[0], "3a", "", "", "");
    let svg = e.save("svg").unwrap();
    assert!(svg.contains(">L3a</text>") && svg.contains(">L2b</text>"));
    // the picture opens again with its annotations
    let (d, _) = import(&svg).unwrap();
    assert_eq!(d.to_json(), e.save("mzdraw").unwrap());
}
