//! The saved SVG with a text alternative (role, title, desc, language) and only SVG 1.1 elements. Synthetic structures only.

mod common;

use common::{golden_files, section};
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
    for a in ["role=\"img\"", "aria-labelledby=\"t-", "xml:lang=\"en\""] {
        assert!(root.contains(a), "{a}");
    }
    assert_eq!(text_of(&s, "title"), "C2H6O");
    assert_eq!(text_of(&s, "desc"), "C2H6O.");
    let id = s
        .split("<title id=\"t-")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert!(
        root.contains(&format!("aria-labelledby=\"t-{id} d-{id}\""))
            && s.contains(&format!("<desc id=\"d-{id}\">"))
    );
    assert_eq!(id.len(), 8);
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
    assert_eq!(
        text_of(&s, "desc"),
        "Reaction scheme: C2H6O and CH4, with a reaction arrow."
    );
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
    e.annotate(at[0], "", "2b", "ethanol", "", "");
    let s = svg::to_svg(e.doc());
    assert_eq!(text_of(&s, "desc"), "C2H6O (level 2b) and C3H6O.");
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

fn id_of(s: &str) -> String {
    s.split("<title id=\"t-")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string()
}

#[test]
fn ids_differ_between_drawings_and_repeat_for_the_same_one() {
    let (a, b) = (editor("CCO"), editor("CCC"));
    let (sa, sb) = (svg::to_svg(a.doc()), svg::to_svg(b.doc()));
    assert_ne!(
        id_of(&sa),
        id_of(&sb),
        "two SVGs inlined in one page keep their ids apart"
    );
    assert_eq!(
        sa,
        svg::to_svg(editor("CCO").doc()),
        "same drawing, same bytes"
    );
    // the same formula in two different drawings still gets different ids (the hash takes the document)
    let (c, d) = (editor("OCC"), editor("CCO"));
    assert_ne!(id_of(&svg::to_svg(c.doc())), id_of(&svg::to_svg(d.doc())));
    // every id of a page is unique
    let page = format!("{sa}{sb}");
    for id in [id_of(&sa), id_of(&sb)] {
        assert_eq!(page.matches(&format!("id=\"t-{id}\"")).count(), 1);
        assert_eq!(page.matches(&format!("id=\"d-{id}\"")).count(), 1);
    }
}

fn bad_xml_char(c: char) -> bool {
    (c < ' ' && !matches!(c, '\n' | '\t' | '\r')) || c == '\u{fffe}' || c == '\u{ffff}'
}

#[test]
fn text_is_escaped_and_never_invalid_xml() {
    // a text object is the free text of a drawing: markup, quotes, control characters and the longest text allowed (200 characters)
    let long = "X".repeat(200);
    let mut opened = 0;
    for text in [
        "A&B",
        "A<B>",
        "A\"B'",
        "A\u{1}B\u{8}C\u{1f}",
        "A\u{fffe}B\u{ffff}",
        "\u{7f}\u{85}tab\there",
        long.as_str(),
    ] {
        let json = serde_json::json!({
            "format": "mzdraw/1", "next": 5,
            "atoms": [{"id": 1, "el": "C", "x": 0.0, "y": 0.0}, {"id": 2, "el": "O", "x": 1.0, "y": 0.0}],
            "bonds": [{"id": 3, "a": 1, "b": 2, "o": 1}],
            "objects": [{"id": 4, "t": "text", "x": 0.0, "y": 2.0, "text": text}]
        })
        .to_string();
        let d = mzdraw::Document::from_json(&json).unwrap();
        opened += 1;
        let s = svg::carrying(svg::to_svg(&d), &d);
        assert!(!s.chars().any(bad_xml_char), "{text:?}");
        // the text element holds the text with its markup characters escaped, never raw
        for t in s.split("<text ").skip(1) {
            let body = &t[t.find('>').unwrap() + 1..t.find("</text>").unwrap()];
            assert!(!body.contains('<') && !body.contains('>'), "{body}");
            assert!(
                !body
                    .replace("&amp;", "")
                    .replace("&lt;", "")
                    .replace("&gt;", "")
                    .replace("&quot;", "")
                    .contains('&'),
                "{body}"
            );
        }
        // the title and desc are clean too, and the carried document opens again, whatever the text was
        for t in [text_of(&s, "title"), text_of(&s, "desc")] {
            assert!(!t.contains(['<', '>', '"']), "{t}");
        }
        assert_eq!(svg::carried(&s).unwrap().unwrap().to_json(), d.to_json());
    }
    assert_eq!(opened, 7);
    let one = mzdraw::Document::from_json("{\"format\":\"mzdraw/1\",\"next\":2,\"atoms\":[],\"bonds\":[],\"objects\":[{\"id\":1,\"t\":\"text\",\"x\":0,\"y\":0,\"text\":\"A&B\"}]}").unwrap();
    assert!(svg::to_svg(&one).contains(">A&amp;B</text>"));
}

#[test]
fn escaping_of_the_outline_text_is_deterministic() {
    use mzdraw::xml::escape;
    assert_eq!(escape("a&b<c>d\"e"), "a&amp;b&lt;c&gt;d&quot;e");
    assert_eq!(escape("a\u{1}b\u{ffff}c"), "a b c");
    assert_eq!(escape("tab\there\nline"), "tab\there\nline");
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
    let mut seen = 0;
    for (_, fx) in golden_files() {
        for case in section(&fx, "cases") {
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
    }
    assert!(seen >= 10);
}
