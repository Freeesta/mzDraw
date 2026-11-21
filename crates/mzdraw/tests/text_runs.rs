//! Text runs (`arch-std-21`): the markup, the operation `runs` of `obj.add` and `obj.set` with its exact inverse, the drawing, the
//! SVG and the PDF, KET and CDXML in and out, and the route of a log (the CLI). Synthetic texts only.

use mzdraw::io::import;
use mzdraw::runs::{self, Run};
use mzdraw::{Document, Editor, ObjKind, Op, Session};

fn add_text(runs: Vec<Run>) -> Op {
    Op::ObjAdd {
        id: None,
        t: ObjKind::Text,
        x: 1.0,
        y: 2.0,
        x2: 0.0,
        y2: 0.0,
        text: String::new(),
        runs,
        from: 0,
        to: 0,
    }
}

fn set_runs(id: u32, runs: Vec<Run>) -> Op {
    Op::ObjSet {
        id,
        text: None,
        runs: Some(runs),
        x2: None,
        y2: None,
        from: None,
        to: None,
    }
}

fn formatted() -> Vec<Run> {
    vec![
        Run {
            b: true,
            ..Run::plain("Pd")
        },
        Run::sup("2+"),
        Run::line(),
        Run {
            i: true,
            u: true,
            color: "#cc3300".into(),
            ..Run::plain("in ")
        },
        Run {
            font: "Courier New".into(),
            sz: 1.5,
            ..Run::plain("THF")
        },
        Run::sub("2"),
    ]
}

#[test]
fn obj_set_runs_has_an_exact_inverse_and_the_text_follows_the_runs() {
    let mut s = Session::default();
    s.run(&[add_text(runs::parse("H_2O"))]).unwrap();
    let before = s.doc.clone();
    let id = s.doc.objects[0].id;
    assert_eq!(s.doc.objects[0].text, "H2O");
    s.run(&[set_runs(id, formatted())]).unwrap();
    assert_eq!(s.doc.objects[0].text, "Pd2+\nin THF2");
    assert_eq!(s.doc.objects[0].runs, formatted());
    // undo gives the identical document (bytes), redo the identical result
    let after = s.doc.clone();
    assert!(s.undo());
    assert_eq!(s.doc.to_json(), before.to_json());
    assert!(s.redo());
    assert_eq!(s.doc.to_json(), after.to_json());
    // a plain text takes the formatting away; an invalid list, or runs and text together, change nothing
    s.run(&[Op::ObjSet {
        id,
        text: Some("plain".into()),
        runs: None,
        x2: None,
        y2: None,
        from: None,
        to: None,
    }])
    .unwrap();
    assert!(s.doc.objects[0].runs.is_empty());
    let kept = s.doc.to_json();
    for bad in [
        set_runs(
            id,
            vec![Run {
                sub: true,
                sup: true,
                ..Run::plain("x")
            }],
        ),
        set_runs(
            id,
            vec![Run {
                color: "red".into(),
                ..Run::plain("x")
            }],
        ),
        set_runs(id, vec![Run::line()]),
        Op::ObjSet {
            id,
            text: Some("a".into()),
            runs: Some(vec![Run::plain("b")]),
            x2: None,
            y2: None,
            from: None,
            to: None,
        },
    ] {
        assert!(s.run(&[bad]).is_err());
        assert_eq!(s.doc.to_json(), kept);
    }
}

#[test]
fn runs_are_in_the_document_the_log_and_the_arrow_text() {
    let mut s = Session::default();
    s.run(&[
        add_text(formatted()),
        Op::ObjAdd {
            id: None,
            t: ObjKind::Arrow,
            x: 0.0,
            y: 0.0,
            x2: 4.0,
            y2: 0.0,
            text: String::new(),
            runs: runs::parse("CH_3CN\nreflux"),
            from: 0,
            to: 0,
        },
    ])
    .unwrap();
    let json = s.doc.to_json();
    assert!(json.contains(r##""runs":[{"t":"Pd","b":true},{"t":"2+","sup":true},{"br":true},{"t":"in ","i":true,"u":true,"color":"#cc3300"},{"t":"THF","font":"Courier New","sz":1.5},{"t":"2","sub":true}]"##));
    assert_eq!(Document::from_json(&json).unwrap().to_json(), json);
    // the log reads back to the same operations
    let mut again = Session::default();
    again.replay(&s.log_json()).unwrap();
    assert_eq!(again.doc.to_json(), json);
    assert_eq!(again.log_json(), s.log_json());
    // an old document with a plain `text` opens unchanged
    let old = r#"{"format":"mzdraw/1","next":2,"atoms":[],"bonds":[],"objects":[{"id":1,"t":"text","x":0.0,"y":0.0,"text":"old"}]}"#;
    let d = Document::from_json(old).unwrap();
    assert!(d.objects[0].runs.is_empty());
    assert_eq!(d.to_json(), old);
    // a key of a run this version does not know is an error, not a silent drop
    let odd = old.replace(r#""text":"old""#, r#""runs":[{"t":"x","bold":true}]"#);
    assert!(Document::from_json(&odd).is_err());
}

#[test]
fn the_text_tool_reads_the_markup_in_rust() {
    let mut e = Editor::default();
    e.import("CC").unwrap();
    // the same ops a log would carry: obj.add with runs
    let (plain, r) = runs::typed("Pd(OAc)_2, 80 °C");
    assert!(plain.is_empty() && runs::plain(&r) == "Pd(OAc)2, 80 °C");
    let (plain, r) = runs::typed("no markup");
    assert_eq!((plain.as_str(), r.len()), ("no markup", 0));
}

fn doc_with(runs: Vec<Run>) -> Document {
    let mut s = Session::default();
    s.run(&[add_text(runs)]).unwrap();
    s.doc
}

#[test]
fn the_svg_draws_scripts_lines_and_styles() {
    let d = doc_with(formatted());
    let svg = mzdraw::svg::to_svg(&d);
    // scripts are smaller and shifted; a second line is lower; styles are attributes
    assert!(svg.contains("font-weight=\"bold\""));
    assert!(svg.contains("font-style=\"italic\"") && svg.contains("text-decoration=\"underline\""));
    assert!(svg.contains("font-family=\"Courier New\""));
    assert!(svg.contains("fill=\"#cc3300\""));
    let size = |text: &str| {
        let at = svg.find(&format!(">{text}</text>")).unwrap();
        let tag = &svg[svg[..at].rfind("<text").unwrap()..at];
        let num = |key: &str| {
            let k = tag.find(key).unwrap() + key.len();
            tag[k..k + tag[k..].find('"').unwrap()]
                .parse::<f64>()
                .unwrap()
        };
        (num("y=\""), num("font-size=\""))
    };
    let (y_pd, f_pd) = size("Pd");
    let (y_sup, f_sup) = size("2+");
    let (y_in, _) = size("in ");
    let (y_sub, f_sub) = size("2");
    assert!(f_sup < f_pd && f_sub < f_pd);
    assert!(y_sup < y_pd, "a superscript is raised");
    assert!(y_in > y_pd, "the second line is lower");
    assert!(y_sub > y_in, "a subscript is lowered");
    // deterministic bytes, and the PDF holds the bold face only when it is used
    assert_eq!(svg, mzdraw::svg::to_svg(&doc_with(formatted())));
    let pdf =
        String::from_utf8_lossy(&mzdraw::pdf::to_pdf(&d, &mzdraw::Style::default())).to_string();
    assert!(
        pdf.contains("/BaseFont /Helvetica-Bold ") && pdf.contains("/BaseFont /Helvetica-Oblique ")
    );
    assert!(
        !pdf.contains("Helvetica-BoldOblique"),
        "only the faces that are used are named"
    );
    let plain = mzdraw::pdf::to_pdf(&doc_with(runs::parse("H_2O")), &mzdraw::Style::default());
    assert!(!String::from_utf8_lossy(&plain).contains("Helvetica-Bold"));
}

#[test]
fn ket_and_cdxml_carry_the_runs_they_can_and_say_what_they_cannot() {
    let d = doc_with(formatted());
    // KET: bold, italic, scripts and lines; not underline, font, size or colour
    let (back, _) = import(&d.to_ket()).unwrap();
    let o = &back.objects[0];
    assert_eq!(o.t, ObjKind::Text);
    assert_eq!(
        o.runs,
        vec![
            Run {
                b: true,
                ..Run::plain("Pd")
            },
            Run::sup("2+"),
            Run::line(),
            Run {
                i: true,
                ..Run::plain("in ")
            },
            Run::plain("THF"),
            Run::sub("2"),
        ]
    );
    // CDXML: all of it
    let (back, _) = import(&d.to_cdxml()).unwrap();
    assert_eq!(back.objects[0].runs, formatted());
    assert_eq!(back.objects[0].text, d.objects[0].text);
    // the loss manifest names what each format cannot carry
    let lost = |format: &str| -> Vec<&'static str> {
        mzdraw::loss::loss(&d, format)
            .unwrap()
            .iter()
            .map(|l| l.what)
            .collect()
    };
    assert_eq!(lost("ket"), ["text.colour", "text.font", "text.underline"]);
    assert!(lost("cdxml").is_empty() && lost("svg").is_empty());
    assert!(lost("molfile").contains(&"text.script") && lost("molfile").contains(&"text.lines"));
}

#[test]
fn a_native_replay_of_a_log_gives_the_runs_of_the_page() {
    let request = serde_json::json!({
        "log": [
            {"op": "obj.add", "t": "text", "x": 1, "y": 1, "runs": [{"t": "H"}, {"t": "2", "sub": true}, {"t": "O"}]},
            {"op": "obj.set", "id": 1, "runs": [{"t": "CH"}, {"t": "3", "sub": true}, {"br": true}, {"t": "OH", "b": true}]}
        ],
        "out": "mzdraw"
    })
    .to_string();
    let out = mzdraw::replay::replay(&request).unwrap();
    assert!(out.contains(r#""text":"CH3\nOH","runs":[{"t":"CH"},{"t":"3","sub":true},{"br":true},{"t":"OH","b":true}]"#));
}

#[test]
fn the_formula_face_lowers_the_digits_after_symbols() {
    let r = runs::formula(&[Run::plain("CH3CO2H")]);
    assert_eq!(runs::to_markup(&r), "CH_3CO_2H");
}
