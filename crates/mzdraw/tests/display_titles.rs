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
