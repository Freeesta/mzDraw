//! The saved SVG with a text alternative (role, title, desc, language) and only SVG 1.1 elements. Synthetic structures only.

use mzdraw::doc::{Obj, ObjKind};
use mzdraw::{svg, Editor};

fn editor(text: &str) -> Editor {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e
}

/// The text of the first element of that name.
fn text_of(s: &str, tag: &str) -> String {
    let from = s.find(&format!("<{tag} ")).unwrap();
    let from = from + s[from..].find('>').unwrap() + 1;
    s[from..from + s[from..].find(&format!("</{tag}>")).unwrap()].to_string()
}

#[test]
fn one_structure_is_named_by_its_formula() {
    let s = svg::to_svg(editor("CCO").doc());
    let root = s.lines().next().unwrap();
    assert!(root.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\""));
    for a in [
        "role=\"img\"",
        "aria-labelledby=\"svg-title svg-desc\"",
        "xml:lang=\"en\"",
    ] {
        assert!(root.contains(a), "{a}");
    }
    assert_eq!(text_of(&s, "title"), "C2H6O");
    assert_eq!(text_of(&s, "desc"), "C2H6O.");
    // both elements come right after the opening tag, before the drawing
    let lines: Vec<&str> = s.lines().collect();
    assert!(
        lines[1].starts_with("<title ")
            && lines[2].starts_with("<desc ")
            && lines[3].starts_with("<")
    );
}

#[test]
fn two_structures_and_an_arrow() {
    let mut e = editor("CCO.C");
    assert_eq!(text_of(&svg::to_svg(e.doc()), "title"), "C2H6O and CH4");
    let mut d = e.doc().clone();
    d.objects.push(Obj {
        id: 900,
        t: ObjKind::Arrow,
        x: 0.0,
        y: 3.0,
        x2: 2.0,
        y2: 3.0,
        text: String::new(),
        more: Default::default(),
    });
    let s = svg::to_svg(&d);
    assert_eq!(text_of(&s, "title"), "C2H6O and CH4 with a reaction arrow");
    assert_eq!(text_of(&s, "desc"), "C2H6O. CH4. with a reaction arrow.");
    e.import("CCO.C.CC").unwrap();
    assert!(text_of(&svg::to_svg(e.doc()), "title").contains(", "));
}

#[test]
fn an_annotated_structure_says_its_level() {
    let mut e = editor("CCO.CC(C)=O");
    let at: Vec<u32> = mzdraw::outline::pieces(e.doc())
        .iter()
        .map(|p| p.anchor)
        .collect();
    e.annotate(at[0], "2b", "ethanol", "", "");
    let s = svg::to_svg(e.doc());
    assert_eq!(text_of(&s, "desc"), "C2H6O (level 2b). C3H6O.");
    assert_eq!(text_of(&s, "title"), "C2H6O and C3H6O");
}

#[test]
fn an_empty_drawing_says_so() {
    let s = svg::to_svg(Editor::default().doc());
    assert_eq!(text_of(&s, "title"), "Empty drawing");
    assert!(s.contains("role=\"img\"") && s.ends_with("</svg>\n"));
}

#[test]
fn a_saved_svg_with_a_text_alternative_opens_as_the_same_document() {
    let e = editor("CC[N+](C)(C)C");
    let file = svg::carrying(svg::to_svg(e.doc()), e.doc());
    assert_eq!(
        svg::carried(&file).unwrap().unwrap().to_json(),
        e.doc().to_json()
    );
    let mut other = Editor::default();
    other.import(&file).unwrap();
    assert_eq!(other.doc().to_json(), e.doc().to_json());
}

/// The elements and attributes of SVG 1.1 that the writer may use (and `xml:lang`, `role`, `aria-labelledby`, which older readers
/// ignore).
const ELEMENTS: [&str; 7] = [
    "svg", "title", "desc", "metadata", "line", "text", "polygon",
];
const ATTRIBUTES: [&str; 23] = [
    "xmlns",
    "width",
    "height",
    "viewBox",
    "font-family",
    "stroke-linecap",
    "role",
    "aria-labelledby",
    "xml:lang",
    "id",
    "x1",
    "y1",
    "x2",
    "y2",
    "stroke",
    "stroke-width",
    "x",
    "y",
    "font-size",
    "fill",
    "text-anchor",
    "dominant-baseline",
    "points",
];

#[test]
fn every_golden_svg_uses_only_svg_1_1() {
    let fx: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/data/mzdraw_log.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut seen = 0;
    for case in fx["cases"].as_array().unwrap() {
        let s = case["out"]["svg"].as_str().unwrap();
        seen += 1;
        // the carried document is text: drop it, then every tag is a known element with known attributes
        let (a, b) = (s.find("<metadata"), s.find("</metadata>"));
        let s = match (a, b) {
            (Some(a), Some(b)) => format!("{}<metadata/>{}", &s[..a], &s[b + 11..]),
            _ => s.to_string(),
        };
        let mut rest = s.as_str();
        while let Some(i) = rest.find('<') {
            let end = rest[i..].find('>').unwrap() + i;
            let tag = rest[i + 1..end].trim_end_matches('/');
            rest = &rest[end + 1..];
            if tag.starts_with('/') {
                continue;
            }
            let name = tag.split([' ', '/']).next().unwrap();
            assert!(ELEMENTS.contains(&name), "{} : <{name}>", case["name"]);
            let mut attrs = tag[name.len()..].trim();
            while let Some(eq) = attrs.find("=\"") {
                let key = attrs[..eq].trim();
                assert!(ATTRIBUTES.contains(&key), "{} : {name} {key}", case["name"]);
                let close = attrs[eq + 2..].find('"').unwrap() + eq + 2;
                attrs = attrs[close + 1..].trim();
            }
        }
    }
    assert!(seen >= 10);
}
