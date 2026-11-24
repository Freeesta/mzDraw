//! The title of a structure drawn under it as its legend (`display.titles`, rdkit-compare 5): off by default, the first line of the
//! caption, under its own structure, before the calculated values, in the SVG, the PDF and the CDXML, kept in the settings file.
//! Synthetic structures and names only.

use mzdraw::{svg, Document, Editor, Op, Session, Style};

fn titled() -> Document {
    // ethanol (atoms 1-3) and benzene (atoms 4-9), each with a title; laid out side by side
    let mut e = Editor::default();
    e.import("CCO.c1ccccc1").unwrap();
    let mut s = Session::new(e.doc().clone());
    let title = |id, t: &str| {
        Op::parse(&serde_json::json!({"op": "atom.title", "id": id, "title": t})).unwrap()
    };
    s.run(&[title(1, "compound 1"), title(4, "compound 2 (ring)")])
        .unwrap();
    s.doc
}

fn style(set: &[&str]) -> Style {
    let mut e = Editor::default();
    for x in set {
        e.set(x).unwrap();
    }
    e.style.clone()
}

/// The `(x, y, text)` of every `<text>` of an SVG.
fn texts(svg: &str) -> Vec<(f64, f64, String)> {
    let attr = |e: &str, k: &str| -> f64 {
        e.split(&format!(" {k}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    svg.split("<text")
        .skip(1)
        .map(|e| {
            let t = e
                .split('>')
                .nth(1)
                .unwrap()
                .split('<')
                .next()
                .unwrap()
                .to_string();
            (attr(e, "x"), attr(e, "y"), t)
        })
        .collect()
}

#[test]
fn the_titles_are_off_by_default_and_each_stands_under_its_own_structure() {
    let d = titled();
    let off = svg::to_svg(&d);
    assert!(
        !texts(&off).iter().any(|t| t.2.starts_with("compound")),
        "nothing by default"
    );
    let st = style(&["display.titles=on"]);
    let on = svg::to_svg_styled(&d, &st);
    let t = texts(&on);
    let at = |name: &str| t.iter().find(|x| x.2 == name).cloned().unwrap();
    let (one, two) = (at("compound 1"), at("compound 2 (ring)"));
    // each title is under its own structure: centred on its atoms, below its lowest atom; the page grew to hold them
    let o = t.iter().find(|x| x.2 == "O").unwrap();
    let mean_x = |ids: std::ops::RangeInclusive<u32>| {
        let xs: Vec<f64> = ids.filter_map(|id| d.atom(id)).map(|a| a.x).collect();
        xs.iter().sum::<f64>() / xs.len() as f64
    };
    // each title stands under its own structure: left to right in the order of the structures
    assert_eq!(
        one.0 < two.0,
        mean_x(1..=3) < mean_x(4..=9),
        "{one:?} {two:?}"
    );
    assert!(one.1 > o.1, "the title of ethanol is below its oxygen");
    let height = |s: &str| -> f64 {
        s.split("height=\"")
            .nth(1)
            .unwrap()
            .split("pt")
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    assert!(height(&on) > height(&off), "the page holds the titles");
    // a word that is none is refused and changes nothing
    let mut e = Editor::default();
    assert_eq!(e.set("display.titles=yes"), Err("error.draw2.styleValue"));
    assert_eq!(e.get("display.titles").as_deref(), Some("off"));
}

#[test]
fn the_title_is_the_first_line_before_the_values_and_a_structure_without_one_has_none() {
    let d = titled();
    let t = texts(&svg::to_svg_styled(
        &d,
        &style(&["display.titles=on", "display.values=formula"]),
    ));
    let line = |name: &str| t.iter().find(|x| x.2 == name).cloned().unwrap();
    // under ethanol: its title, then its formula, one line lower
    let (title, formula) = (line("compound 1"), line("C2H6O"));
    assert!(
        (title.0 - formula.0).abs() < 0.01 && formula.1 > title.1,
        "{title:?} {formula:?}"
    );
    // a structure without a title has no title line: only its formula
    let mut e = Editor::default();
    e.import("CCO.CN").unwrap();
    let mut s = Session::new(e.doc().clone());
    s.run(&[
        Op::parse(&serde_json::json!({"op": "atom.title", "id": 1, "title": "only one"})).unwrap(),
    ])
    .unwrap();
    let t = texts(&svg::to_svg_styled(&s.doc, &style(&["display.titles=on"])));
    assert_eq!(t.iter().filter(|x| x.2 == "only one").count(), 1);
    assert!(!t.iter().any(|x| x.2 == "CH5N"), "no values were asked for");
}

#[test]
fn the_pdf_and_the_cdxml_have_the_titles_and_the_settings_file_keeps_the_choice() {
    let d = titled();
    let st = style(&["display.titles=on"]);
    let pdf = mzdraw::pdf::to_pdf(&d, &st);
    let text: String = pdf
        .iter()
        .map(|&c| if c < 128 { c as char } else { '?' })
        .collect();
    assert!(
        text.contains("(compound 1) Tj") && text.contains("(compound 2 \\(ring\\)) Tj"),
        "titles in the PDF"
    );
    let mut e = Editor::default();
    e.import(&d.to_json()).unwrap();
    e.set("display.titles=on").unwrap();
    let cdxml = e.save("cdxml").unwrap();
    assert!(
        cdxml.contains("compound 1") && cdxml.contains("compound 2 (ring)"),
        "titles in the CDXML captions"
    );
    let file = e.save("settings").unwrap();
    assert!(file.contains("\"titles\":\"on\""), "{file}");
    assert!(Style::from_text(&file).unwrap().titles);
    assert!(!Editor::default()
        .save("settings")
        .unwrap()
        .contains("\"titles\""));
}

/// One structure per title: `CCO`, `CCN`, ... each titled on its first atom, side by side.
fn each_titled(titles: &[&str]) -> Document {
    let parts = ["CCO", "CCN", "CCC", "CCCl", "CCBr", "CCS", "CCF", "CCI"];
    let mut e = Editor::default();
    e.import(&parts[..titles.len()].join(".")).unwrap();
    let firsts: Vec<u32> = mzdraw::layout::pieces(e.doc())
        .iter()
        .map(|p| *p.iter().min().unwrap())
        .collect();
    let mut s = Session::new(e.doc().clone());
    // a title the operation takes goes through it; one it refuses (a control character, more than 80 characters) is written in
    // the document held in memory, as a program using the library can
    let ops: Vec<Op> = (firsts.iter().zip(titles))
        .filter(|(_, t)| mzdraw::doc::title_valid(t))
        .map(|(id, t)| {
            Op::parse(&serde_json::json!({"op": "atom.title", "id": id, "title": t})).unwrap()
        })
        .collect();
    s.run(&ops).unwrap();
    let mut d = s.doc;
    for (id, t) in firsts.iter().zip(titles) {
        if !mzdraw::doc::title_valid(t) {
            let a = d.atoms.iter_mut().find(|a| a.id == *id).unwrap();
            a.more
                .push(("title".into(), serde_json::Value::from(*t).to_string()));
        }
    }
    d
}

fn pdf_text(d: &Document, st: &Style) -> String {
    (mzdraw::pdf::to_pdf(d, st).iter())
        .map(|&c| if c < 128 { c as char } else { '?' })
        .collect()
}

fn cdxml_of(d: &Document, set: &[&str]) -> String {
    // the document held in memory, as it is (a file of it would be refused when its title is)
    let mut e = Editor::default();
    e.session = Session::new(d.clone());
    for s in set {
        e.set(s).unwrap();
    }
    e.save("cdxml").unwrap()
}

/// The width and the height of an SVG page, in points.
fn page(svg: &str) -> (f64, f64) {
    let attr = |k: &str| -> f64 {
        svg.split(&format!("{k}=\""))
            .nth(1)
            .unwrap()
            .split("pt")
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    (attr("width"), attr("height"))
}

/// Every text of an XML file is well formed: no raw `<` in a text, each `&` starts one of the entities the writers use, no control
/// character but the line feed and the tab.
fn well_formed(xml: &str) {
    for body in xml.split('>').skip(1).map(|t| t.split('<').next().unwrap()) {
        for (k, _) in body.match_indices('&') {
            let rest = &body[k..];
            assert!(
                ["&amp;", "&lt;", "&gt;", "&quot;", "&#xa;", "&apos;"]
                    .iter()
                    .any(|e| rest.starts_with(e)),
                "a bare & in {body:?}"
            );
        }
    }
    assert!(
        !xml.chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t'),
        "a control character"
    );
}

#[test]
fn odd_titles_are_escaped_in_the_svg_the_pdf_and_the_cdxml_and_an_empty_one_draws_nothing() {
    // the routes of the CLI and the page refuse a control character and more than 80 characters: the operation and the file
    let mut s = Session::new(each_titled(&[""]));
    for bad in ["x\u{1}y", "a\nb", &"L".repeat(81)] {
        let refused =
            match Op::parse(&serde_json::json!({"op": "atom.title", "id": 1, "title": bad})) {
                Ok(op) => s.run(&[op]).unwrap_err().key,
                Err(key) => key,
            };
        assert_eq!(refused, "error.draw2.title", "{bad:?}");
    }
    let file = serde_json::json!({"format": "mzdraw/1", "next": 2, "atoms": [{"id": 1, "el": "C", "x": 0, "y": 0, "title": "a\nb"}]});
    assert!(Document::from_json(&file.to_string()).is_err());
    let d = each_titled(&[
        "a<b> & \"c\" 'd'",
        "\u{3b2}-Ala, caf\u{e9}, \u{4e2d}",
        "x\u{1}y\u{7f}z",
        "",
        "\t\n ",
        "(R)-2\\3",
    ]);
    let st = style(&["display.titles=on"]);
    let svg = svg::to_svg_styled(&d, &st);
    well_formed(&svg);
    let t: Vec<String> = texts(&svg).into_iter().map(|x| x.2).collect();
    for want in [
        "a&lt;b&gt; &amp; &quot;c&quot; 'd'",
        "\u{3b2}-Ala, caf\u{e9}, \u{4e2d}",
        "x y z",
        "(R)-2\\3",
    ] {
        assert_eq!(
            t.iter().filter(|x| *x == want).count(),
            1,
            "{want:?} in {t:?}"
        );
    }
    // the empty title and the one of blanks and control characters draw no line: the four others are the only legends
    let legends = |d: &Document, st: &Style| -> usize {
        mzdraw::values::captions_titled(d, st.values, &st.adducts, &[], st.titles)
            .iter()
            .map(|c| c.lines.len())
            .sum()
    };
    assert_eq!(legends(&d, &st), 4);
    // the PDF: WinAnsi, the three special characters escaped, what WinAnsi does not have as `?`, the control characters gone
    let pdf = pdf_text(&d, &st);
    for want in [
        "(a<b> & \"c\" 'd') Tj",
        "(?-Ala, caf\\351, ?) Tj",
        "(x y z) Tj",
        "(\\(R\\)-2\\\\3) Tj",
    ] {
        assert!(pdf.contains(want), "{want} in the PDF");
    }
    // the CDXML: the same lines as the SVG, escaped the same way
    let cdxml = cdxml_of(&d, &["display.titles=on"]);
    well_formed(&cdxml);
    for want in [
        ">a&lt;b&gt; &amp; &quot;c&quot; 'd'<",
        ">\u{3b2}-Ala, caf\u{e9}, \u{4e2d}<",
        ">x y z<",
    ] {
        assert!(cdxml.contains(want), "{want} in the CDXML");
    }
}

#[test]
fn a_title_with_line_breaks_is_one_line_in_the_svg_the_pdf_and_the_cdxml() {
    assert_eq!(mzdraw::values::legend("first\nsecond"), "first second");
    assert_eq!(mzdraw::values::legend(" a\r\n\tb \n"), "a b");
    assert_eq!(mzdraw::values::legend("\n\n"), "");
    let broken = each_titled(&["first line\nsecond line"]);
    let joined = each_titled(&["first line second line"]);
    let st = style(&["display.titles=on"]);
    let (b, j) = (
        svg::to_svg_styled(&broken, &st),
        svg::to_svg_styled(&joined, &st),
    );
    // the same picture: one line, and a page as tall as one line makes it (after the head, whose ids are a digest of the document)
    let body = |svg: &str| svg.split("</desc>").nth(1).unwrap().to_string();
    assert_eq!(body(&b), body(&j));
    assert_eq!(
        texts(&b)
            .iter()
            .filter(|x| x.2 == "first line second line")
            .count(),
        1
    );
    assert_eq!(page(&b), page(&j));
    let shown = |pdf: String| -> Vec<String> {
        // the texts shown (the rest of the file carries the document, which differs)
        let mut parts: Vec<String> = pdf
            .split(") Tj")
            .map(|t| t.rsplit('(').next().unwrap().to_string())
            .collect();
        parts.pop();
        parts
    };
    assert_eq!(shown(pdf_text(&broken, &st)), shown(pdf_text(&joined, &st)));
    assert!(pdf_text(&broken, &st).contains("(first line second line) Tj"));
    // the molfile the PDF carries has the title on its one title line
    assert!(pdf_text(&broken, &st).contains("stream\nfirst line second line\n  "));
    assert_eq!(broken.title(), "first line second line");
    // the CDXML writes the one line: no line feed in the legend (a kept one would break it into two lines there)
    let c = cdxml_of(&broken, &["display.titles=on"]);
    assert!(c.contains(">first line second line<"), "{c}");
    assert!(!c.contains("first line&#xa;") && !c.contains("first line\n"));
    assert_eq!(c, cdxml_of(&joined, &["display.titles=on"]));
}

#[test]
fn a_long_title_widens_the_page_centred_on_its_structure_and_nothing_clips_it() {
    // the longest title a document holds (80 characters)
    let long = "a long name of a synthetic transformation product, as long as a title can be: 80";
    assert_eq!(long.chars().count(), mzdraw::doc::MAX_TITLE);
    for titles in [vec![long], vec![long, "short"], vec!["short", long]] {
        let d = each_titled(&titles);
        let st = style(&["display.titles=on"]);
        let short = each_titled(&vec!["x"; titles.len()]);
        let (svg_long, svg_short) = (svg::to_svg_styled(&d, &st), svg::to_svg_styled(&short, &st));
        let (w, _) = page(&svg_long);
        assert!(w > page(&svg_short).0 + 100.0, "the page is wider");
        let t = texts(&svg_long);
        let at = t.iter().find(|x| x.2 == long).unwrap();
        // the whole line is on the page, with the page's margin left at both ends
        let half = mzdraw::scene::width(
            long,
            mzdraw::values::caption_font(st.metrics(st.bond_length, false).font),
        ) / 2.0;
        assert!(at.0 - half >= 0.0 && at.0 + half <= w, "{at:?} {half} {w}");
        // centred on its structure: the middle of its atoms, where the page draws them (the first bond, between atoms 1 and 2
        // (carbons, so the line runs from centre to centre), gives the origin of the page)
        let k = titles.iter().position(|x| *x == long).unwrap();
        let piece = &mzdraw::layout::pieces(&d)[k];
        let xs: Vec<f64> = piece.iter().map(|id| d.atom(*id).unwrap().x).collect();
        let mid = (xs.iter().cloned().fold(f64::MAX, f64::min)
            + xs.iter().cloned().fold(f64::MIN, f64::max))
            / 2.0;
        let b = &d.bonds[0];
        assert_eq!((b.a, b.b), (1, 2));
        let x1: f64 = svg_long
            .split("<line x1=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let origin = x1 - d.atom(1).unwrap().x * st.bond_length;
        assert!(
            (at.0 - (origin + mid * st.bond_length)).abs() < 0.02,
            "{titles:?}: {} {}",
            at.0,
            origin + mid * st.bond_length
        );
        // the PDF has the same page
        let pdf = pdf_text(&d, &st);
        let media = pdf
            .split("/MediaBox [0 0 ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap();
        assert!(
            (media.parse::<f64>().unwrap() - w).abs() < 0.01,
            "{media} {w}"
        );
    }
}

/// The boxes of the texts of a display list: `(text, muted, [x0, y0, x1, y1])` (a text is centred on its `y`, `size` tall).
fn text_boxes(sc: &mzdraw::scene::Scene) -> Vec<(String, bool, [f64; 4])> {
    use mzdraw::scene::{self, colour};
    let (v, mut out, mut i) = (&sc.data, Vec::new(), 0);
    while i < v.len() {
        let op = v[i];
        i += if op == scene::LINE {
            7
        } else if op == scene::TEXT {
            let (x, y, size) = (
                f64::from(v[i + 1]),
                f64::from(v[i + 2]),
                f64::from(v[i + 3]),
            );
            let t = sc.strings[v[i + 5] as usize].clone();
            let w = scene::width(&t, size);
            let x0 = if v[i + 6] == 0.0 { x - w / 2.0 } else { x };
            out.push((
                t,
                v[i + 4] == colour::MUTED,
                [x0, y - size / 2.0, x0 + w, y + size / 2.0],
            ));
            7
        } else if op == scene::TEXTX {
            13
        } else if op == scene::TRI {
            8
        } else if op == scene::RECT {
            6
        } else {
            5
        };
    }
    out
}

/// The boxes of the texts of an SVG, the same way.
fn svg_boxes(svg: &str) -> Vec<(String, bool, [f64; 4])> {
    let attr = |e: &str, k: &str| -> String {
        e.split(&format!(" {k}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .to_string()
    };
    svg.split("<text")
        .skip(1)
        .map(|e| {
            let t = e
                .split('>')
                .nth(1)
                .unwrap()
                .split('<')
                .next()
                .unwrap()
                .to_string();
            let (x, y): (f64, f64) = (attr(e, "x").parse().unwrap(), attr(e, "y").parse().unwrap());
            let size: f64 = attr(e, "font-size").parse().unwrap();
            let w = mzdraw::scene::width(&t, size);
            let x0 = if attr(e, "text-anchor") == "middle" {
                x - w / 2.0
            } else {
                x
            };
            (
                t,
                attr(e, "fill") == "#5c5850",
                [x0, y - size / 2.0, x0 + w, y + size / 2.0],
            )
        })
        .collect()
}

fn hits(a: &[f64; 4], b: &[f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// No id (muted) covers a caption line, in a display list: the ids keep off the legends and the values.
fn ids_clear_of_captions(
    boxes: &[(String, bool, [f64; 4])],
    lines: &[String],
    what: &str,
) -> usize {
    let ids: Vec<_> = boxes.iter().filter(|b| b.1).collect();
    let caps: Vec<_> = boxes
        .iter()
        .filter(|b| !b.1 && lines.contains(&b.0))
        .collect();
    assert!(
        !ids.is_empty() && !caps.is_empty(),
        "{what}: ids and captions are drawn"
    );
    for i in &ids {
        for c in &caps {
            assert!(
                !hits(&i.2, &c.2),
                "{what}: id {:?} covers caption {:?}",
                i,
                c
            );
        }
    }
    caps.len()
}

#[test]
fn the_ids_keep_off_the_legends_and_the_values_with_the_lens_on_in_the_canvas_and_the_svg() {
    // the structures whose lowest atoms have ids beside them (rings, a triol, an amide), titled; and a parent with three TPs under
    // the diff lens (the golden of `tp-lens.json`), each titled
    let named = each_titled(&["TP1 phenol", "TP2", "TP3", "TP4"]);
    let mut e = Editor::default();
    e.import("c1ccccc1O.OCC(O)CO.NC(=O)c1ccccc1.C1CC1").unwrap();
    let firsts: Vec<u32> = mzdraw::layout::pieces(e.doc())
        .iter()
        .map(|p| *p.iter().min().unwrap())
        .collect();
    let rings = {
        let mut s = Session::new(e.doc().clone());
        let ops: Vec<Op> = (firsts.iter().enumerate())
            .map(|(k, id)| Op::parse(&serde_json::json!({"op": "atom.title", "id": id, "title": format!("ring {k}")})).unwrap())
            .collect();
        s.run(&ops).unwrap();
        s.doc
    };
    let lens: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/data/log/tp-lens.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let case = (lens["cases"].as_array().unwrap().iter())
        .find(|c| c["name"].as_str().unwrap().ends_with("lens tint+ghost"))
        .unwrap();
    let pathway = {
        let d = Document::from_json(case["out"]["mzdraw"].as_str().unwrap()).unwrap();
        let firsts: Vec<u32> = mzdraw::layout::pieces(&d)
            .iter()
            .map(|p| *p.iter().min().unwrap())
            .collect();
        let mut s = Session::new(d);
        let ops: Vec<Op> = (firsts.iter().enumerate())
            .map(|(k, id)| Op::parse(&serde_json::json!({"op": "atom.title", "id": id, "title": format!("TP{k} of the parent")})).unwrap())
            .collect();
        s.run(&ops).unwrap();
        s.doc
    };
    let mut checked = 0;
    for (name, d) in [("named", named), ("rings", rings), ("pathway", pathway)] {
        for set in [
            vec!["display.ids=both", "display.titles=on"],
            vec![
                "display.ids=atoms",
                "display.titles=on",
                "display.values=formula",
            ],
            vec![
                "display.ids=both",
                "display.titles=on",
                "display.values=formula",
                "display.lens=tint+ghost",
            ],
            vec![
                "display.ids=bonds",
                "display.titles=on",
                "display.lens=tint",
            ],
        ] {
            let mut e = Editor::default();
            e.session = Session::new(d.clone());
            for s in &set {
                e.set(s).unwrap();
            }
            let lines: Vec<String> = mzdraw::values::captions_titled(
                e.doc(),
                e.style.values,
                &e.style.adducts,
                &[],
                true,
            )
            .into_iter()
            .flat_map(|c| c.lines)
            .collect();
            let what = format!("{name} {set:?}");
            // the canvas (the editor's display list) and the SVG (the page of the export)
            checked +=
                ids_clear_of_captions(&text_boxes(&e.scene()), &lines, &format!("canvas {what}"));
            checked += ids_clear_of_captions(
                &svg_boxes(&e.save("svg").unwrap()),
                &lines,
                &format!("svg {what}"),
            );
        }
    }
    assert!(checked > 40, "{checked} caption lines checked");
}
