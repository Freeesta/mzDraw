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

fn plain_text(text: &str) -> Op {
    Op::ObjAdd {
        id: None,
        t: ObjKind::Text,
        x: 1.0,
        y: 2.0,
        x2: 0.0,
        y2: 0.0,
        text: text.into(),
        runs: vec![],
        from: 0,
        to: 0,
    }
}

fn text_rect(d: &Document) -> Vec<f64> {
    let v: serde_json::Value =
        serde_json::from_str(&mzdraw::digest::json(d, &mzdraw::Style::default()).unwrap()).unwrap();
    v["objects"][0][2]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_f64().unwrap())
        .collect()
}

#[test]
fn the_digest_box_of_a_formatted_text_is_its_widest_line_and_its_lines() {
    let long = "x".repeat(80);
    let two = doc_with(vec![Run::plain(&long), Run::line(), Run::plain("yy")]);
    let one_line = {
        let mut s = Session::default();
        s.run(&[plain_text(&long)]).unwrap();
        s.doc
    };
    let joined = {
        let mut s = Session::default();
        s.run(&[plain_text(&format!("{long}yy"))]).unwrap();
        s.doc
    };
    let (a, b, c) = (text_rect(&two), text_rect(&one_line), text_rect(&joined));
    assert_eq!(a[2], b[2], "the right edge is the widest line's");
    assert!(a[2] < c[2], "not the width of all the characters");
    assert!(a[1] < b[1], "the second line lowers the box");
}

fn rejected(op: Op) -> &'static str {
    let mut s = Session::default();
    s.run(&[plain_text("t")]).unwrap();
    s.run(&[op]).unwrap_err().key
}

fn add_runs(runs: Vec<Run>) -> Op {
    add_text(runs)
}

#[test]
fn every_refusal_of_runs_has_its_key_and_the_caps_hold_through_the_op_path() {
    let sub_sup = Run {
        sub: true,
        sup: true,
        ..Run::plain("x")
    };
    assert_eq!(
        rejected(add_runs(vec![sub_sup.clone()])),
        "error.draw2.runStyle"
    );
    assert_eq!(
        rejected(add_runs(vec![Run {
            b: true,
            ..Run::line()
        }])),
        "error.draw2.runStyle"
    );
    assert_eq!(
        rejected(add_runs(vec![Run {
            color: "red".into(),
            ..Run::plain("x")
        }])),
        "error.draw2.runColor"
    );
    assert_eq!(
        rejected(add_runs(vec![Run {
            color: "#12345".into(),
            ..Run::plain("x")
        }])),
        "error.draw2.runColor"
    );
    for sz in [9.0, 0.1, f64::INFINITY] {
        assert_eq!(
            rejected(add_runs(vec![Run {
                sz,
                ..Run::plain("x")
            }])),
            "error.draw2.runSize",
            "{sz}"
        );
    }
    for font in ["Arial\"; x", "a;b", "a\\b", "x\ny", &"f".repeat(65)] {
        assert_eq!(
            rejected(add_runs(vec![Run {
                font: font.into(),
                ..Run::plain("x")
            }])),
            "error.draw2.runFont",
            "{font:?}"
        );
    }
    // the caps: 400 runs, 1000 characters, 20 lines (a run of two styles alternating keeps the runs apart)
    let alternating = |n: usize| {
        (0..n)
            .map(|k| {
                if k % 2 == 0 {
                    Run::plain("a")
                } else {
                    Run::sub("b")
                }
            })
            .collect::<Vec<_>>()
    };
    let mut s = Session::default();
    s.run(&[add_text(alternating(400))]).unwrap();
    assert_eq!(
        rejected(add_runs(alternating(401))),
        "error.draw2.runsCount"
    );
    let mut s2 = Session::default();
    s2.run(&[add_text(vec![Run::plain(&"c".repeat(200)); 1])])
        .unwrap();
    let chars = |n: usize| {
        (0..n / 200)
            .map(|k| Run {
                sz: 1.0 + k as f64 / 100.0,
                ..Run::plain(&"c".repeat(200))
            })
            .collect::<Vec<_>>()
    };
    assert!(s2.run(&[add_text(chars(1000))]).is_ok());
    assert_eq!(rejected(add_runs(chars(1200))), "error.draw2.runsCount");
    let lines = |n: usize| {
        (0..n - 1)
            .flat_map(|_| [Run::plain("a"), Run::line()])
            .chain([Run::plain("z")])
            .collect::<Vec<_>>()
    };
    assert!(Session::default().run(&[add_text(lines(20))]).is_ok());
    assert_eq!(rejected(add_runs(lines(21))), "error.draw2.runsCount");
    // obj.set takes the same keys
    let mut s = Session::default();
    s.run(&[plain_text("t")]).unwrap();
    let id = s.doc.objects[0].id;
    assert_eq!(
        s.run(&[set_runs(id, vec![sub_sup])]).unwrap_err().key,
        "error.draw2.runStyle"
    );
    assert_eq!(
        s.run(&[set_runs(id, alternating(401))]).unwrap_err().key,
        "error.draw2.runsCount"
    );
    assert_eq!(s.doc.objects[0].text, "t");
}

#[test]
fn colours_are_written_in_one_form_and_a_later_runs_key_survives() {
    let mut s = Session::default();
    s.run(&[add_text(vec![Run {
        color: " #F00 ".into(),
        ..Run::plain("x")
    }])])
    .unwrap();
    assert_eq!(s.doc.objects[0].runs[0].color, "#ff0000");
    let old = r##"{"format":"mzdraw/1","next":2,"atoms":[],"bonds":[],"objects":[{"id":1,"t":"text","x":0.0,"y":0.0,"text":"a","runs":[{"t":"a","wave":{"n":2},"color":"#00FF00"}]}]}"##;
    let d = Document::from_json(old).unwrap();
    assert_eq!(d.objects[0].runs[0].color, "#00ff00");
    assert!(
        d.to_json().contains(r#""wave":{"n":2}"#),
        "the key of a later version is written back"
    );
}

#[test]
fn a_foreign_text_that_cannot_be_held_is_clamped_and_said() {
    let file = r#"<CDXML><page><fragment id="1"><n id="2" p="0 0"/><n id="3" p="14.4 0"/><b id="4" B="2" E="3"/></fragment><t p="10 10"><s font="3" size="100" face="1">Big</s><s font="3" size="10" face="32">2</s></t></page></CDXML>"#;
    let (d, info) = import(file).unwrap();
    let r = &d.objects[0].runs;
    assert_eq!((r[0].t.as_str(), r[0].b, r[0].sz), ("Big", true, 8.0));
    assert!(r[1].sub);
    assert_eq!(info.runs_adjusted, 1);
    assert_eq!(info.notes(), vec![("draw2.import.runsAdjusted", 1)]);
    // a valid text says nothing
    let ok = file.replace("size=\"100\"", "size=\"12\"");
    assert!(import(&ok).unwrap().1.notes().is_empty());
    // KET: a text with more characters than a text holds keeps its words and says the styles went
    let rich = serde_json::json!({"blocks": [{"text": "w".repeat(1500), "type": "unstyled", "depth": 0, "inlineStyleRanges": [{"offset": 0, "length": 3, "style": "BOLD"}], "entityRanges": [], "data": {}}], "entityMap": {}});
    let ket = serde_json::json!({"root": {"nodes": [{"type": "text", "data": {"content": rich.to_string(), "position": {"x": 1.0, "y": 1.0, "z": 0}}}]}}).to_string();
    let (d, info) = import(&ket).unwrap();
    assert_eq!(info.runs_adjusted, 1);
    assert!(d.objects.iter().all(|o| o.runs.is_empty()));
}

#[test]
fn the_svg_escapes_what_a_text_can_hold() {
    let d = doc_with(vec![
        Run {
            b: true,
            ..Run::plain("A&B <x> \"q\" ")
        },
        Run {
            font: "Courier New".into(),
            ..Run::plain("שלום ")
        },
        Run::sub("a<b"),
    ]);
    let svg = mzdraw::svg::to_svg(&d);
    assert!(svg.contains("A&amp;B &lt;x&gt; &quot;q&quot; "));
    assert!(svg.contains("שלום"));
    assert!(svg.contains(">a&lt;b</text>"));
    assert!(!svg.contains("<x>"));
}

/// The best of a few runs, in seconds.
fn best(mut f: impl FnMut()) -> f64 {
    (0..3)
        .map(|_| {
            let t = std::time::Instant::now();
            f();
            t.elapsed().as_secs_f64()
        })
        .fold(f64::MAX, f64::min)
}

/// The time of `work` at two sizes four times apart is within a bound of linear (4): a quadratic one is 16.
fn assert_linear(what: &str, mut work: impl FnMut(usize)) {
    let (small, large) = (best(|| work(500)), best(|| work(2000)));
    assert!(
        large / small < 9.0,
        "{what}: x4 the input took x{:.1} the time",
        large / small
    );
}

#[test]
fn many_formatted_texts_paint_in_linear_time() {
    let texts = |n: usize| {
        let mut d = Document::default();
        for k in 0..n {
            let id = d.next;
            d.next += 1;
            d.objects.push(mzdraw::Obj {
                id,
                t: ObjKind::Text,
                x: (k % 50) as f64,
                y: (k / 50) as f64,
                x2: 0.0,
                y2: 0.0,
                text: runs::plain(&formatted()),
                runs: formatted(),
                from: 0,
                to: 0,
                more: vec![],
            });
        }
        d
    };
    let docs = [texts(500), texts(2000)];
    let pick = |n: usize| if n == 500 { &docs[0] } else { &docs[1] };
    assert_linear("svg", |n| drop(mzdraw::svg::to_svg(pick(n))));
    assert_linear("loss", |n| {
        drop(mzdraw::loss::loss(pick(n), "cdxml").unwrap())
    });
    assert_linear("cdxml", |n| drop(pick(n).to_cdxml()));
}

#[test]
fn a_hostile_ket_text_is_read_in_linear_time() {
    let ket = |chars: usize, ranges: usize| {
        let styles = ["BOLD", "ITALIC", "SUBSCRIPT", "SUPERSCRIPT"];
        let r: Vec<serde_json::Value> = (0..ranges).map(|k| serde_json::json!({"offset": k % chars.max(1), "length": chars, "style": styles[k % 4]})).collect();
        let rich = serde_json::json!({"blocks": [{"text": "w".repeat(chars), "type": "unstyled", "depth": 0, "inlineStyleRanges": r, "entityRanges": [], "data": {}}], "entityMap": {}});
        serde_json::json!({"root": {"nodes": [{"type": "text", "data": {"content": rich.to_string(), "position": {"x": 1.0, "y": 1.0, "z": 0}}}]}}).to_string()
    };
    // more characters than a text holds (checked before any work), and a small text with very many ranges
    let big = [ket(5000, 5000), ket(20000, 20000)];
    let (a, b) = (
        best(|| drop(import(&big[0]))),
        best(|| drop(import(&big[1]))),
    );
    assert!(b / a < 9.0, "characters: x4 took x{:.1}", b / a);
    let many = [ket(900, 20000), ket(900, 80000)];
    let (a, b) = (
        best(|| drop(import(&many[0]))),
        best(|| drop(import(&many[1]))),
    );
    assert!(b / a < 9.0, "ranges: x4 took x{:.1}", b / a);
}
