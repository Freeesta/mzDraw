//! The settings file `mzdraw-settings/1`: the presets, what a file may say, and what a style does to the screen and to the SVG.

use mzdraw::editor::MOUSE;
use mzdraw::settings::{Style, PRESETS};
use mzdraw::{svg, Document, Editor};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn the_default_style_is_the_look_the_editor_had() {
    // on the screen at 40 px per bond: the numbers that were constants before the style existed
    let m = Style::default().metrics(40.0, true);
    for (got, want) in [
        (m.line, 1.8),
        (m.spacing, 5.2),
        (m.wedge, 4.0),
        (m.hash, 4.8),
        (m.font, 16.0),
        (m.gap, 9.92),
        (m.small, 11.2),
    ] {
        assert!(close(got, want), "{got} {want}");
    }
    assert!(!m.mono);
    // far zoomed out a line stays one pixel wide and the font readable; a file takes the numbers as they are
    let tiny = Style::default().metrics(8.0, true);
    assert_eq!((tiny.line, tiny.font), (1.0, 9.0));
    let file = Style::default().metrics(8.0, false);
    assert!(file.line < 1.0 && file.font < 9.0);
    assert_eq!(Editor::default().style, Style::default());
}

#[test]
fn presets_are_named_and_write_themselves_as_a_settings_file() {
    assert_eq!(PRESETS, ["mzlab", "acs-1996", "slides"]);
    for name in PRESETS {
        let s = Style::preset(name).unwrap();
        assert_eq!(s.name, name);
        assert_eq!(Style::from_text(name).unwrap(), s);
        // a style written as a file reads back as itself, byte for byte the second time
        let text = s.to_json();
        let again = Style::from_text(&text).unwrap();
        assert_eq!(again, s, "{text}");
        assert_eq!(again.to_json(), text);
    }
    assert_eq!(
        Style::preset("acs-1996").unwrap().to_json(),
        r#"{"format":"mzdraw-settings/1","style":{"bold_width":2.0,"bond_length":14.4,"bond_spacing":0.18,"colour":"mono","font_size":10.0,"hash_spacing":2.5,"line_width":0.6,"margin":1.6,"preset":"acs-1996","subscript_scale":0.7}}"#
    );
    assert!(Style::preset("nature").is_none());
}

#[test]
fn a_settings_file_overrides_its_preset_and_bad_ones_are_refused() {
    let s = Style::from_text(
        r#" {"format":"mzdraw-settings/1","later":[1],"style":{"preset":"acs-1996","bond_length":20,"colour":"element","unknown":true}} "#,
    )
    .unwrap();
    let acs = Style::preset("acs-1996").unwrap();
    assert_eq!(
        (s.bond_length, s.mono, s.line_width, s.name.as_str()),
        (20.0, false, acs.line_width, "acs-1996")
    );
    // every field optional: an empty style is the default
    assert_eq!(
        Style::from_text(r#"{"format":"mzdraw-settings/1"}"#).unwrap(),
        Style::default()
    );
    assert_eq!(
        Style::from_text(r#"{"format":"mzdraw-settings/1","style":{}}"#).unwrap(),
        Style::default()
    );
    for bad in [
        "",
        "ACS",
        "{}",
        r#"{"format":"mzdraw/1","next":1,"atoms":[],"bonds":[]}"#,
        r#"{"format":"mzdraw-settings/1","style":7}"#,
        r#"{"format":"mzdraw-settings/1","style":{"preset":"nature"}}"#,
        r#"{"format":"mzdraw-settings/1","style":{"bond_length":"14.4"}}"#,
        r#"{"format":"mzdraw-settings/1","style":{"bond_length":0}}"#,
        r#"{"format":"mzdraw-settings/1","style":{"bond_length":1e9}}"#,
        r#"{"format":"mzdraw-settings/1","style":{"line_width":-1}}"#,
        r#"{"format":"mzdraw-settings/1","style":{"font_size":1e300}}"#,
        r#"{"format":"mzdraw-settings/1","style":{"subscript_scale":0}}"#,
        r#"{"format":"mzdraw-settings/1","style":{"colour":"rainbow"}}"#,
        r#"{"format":"mzdraw-settings/1","style":{"#,
    ] {
        assert_eq!(
            Style::from_text(bad).unwrap_err(),
            "error.draw2.style",
            "{bad}"
        );
    }
    // a refused style leaves the editor as it was
    let mut e = Editor::default();
    e.set_style("slides").unwrap();
    assert_eq!(e.set_style("nope").unwrap_err(), "error.draw2.style");
    assert_eq!(e.style.name, "slides");
}

/// The attribute `name` of the first `tag` of an SVG, as a number.
fn attr(svg: &str, tag: &str, name: &str) -> f64 {
    let t = svg.split(&format!("<{tag} ")).nth(1).unwrap();
    let v = t
        .split(&format!("{name}=\""))
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    v.trim_end_matches("pt").parse().unwrap()
}

#[test]
fn a_style_gives_the_file_its_true_size_in_points() {
    // ethanol drawn level: C-C-O, one bond length apart
    let mut e = Editor::default();
    e.import(r#"{"format":"mzdraw/1","next":6,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0},{"id":2,"el":"C","x":1.0,"y":0.0},{"id":3,"el":"O","x":2.0,"y":0.0}],"bonds":[{"id":4,"a":1,"b":2,"o":1},{"id":5,"a":2,"b":3,"o":1}]}"#)
        .unwrap();
    assert_eq!(
        e.svg(),
        svg::to_svg(e.doc()),
        "the default style is the default file"
    );
    let mut widths = Vec::new();
    for (name, bond, line, font, colour) in [
        ("mzlab", 30.0, 1.35, 12.0, "#c02020"),
        ("acs-1996", 14.4, 0.6, 10.0, "#000000"),
        ("slides", 21.6, 1.5, 16.0, "#000000"),
    ] {
        e.set_style(name).unwrap();
        let out = e.svg();
        // the first bond runs from the first carbon to the second: its length is the bond length of the style, in points
        let len = attr(&out, "line", "x2") - attr(&out, "line", "x1");
        assert!((len - bond).abs() < 0.011, "{name}: {len}");
        assert_eq!(attr(&out, "line", "stroke-width"), line, "{name}");
        assert_eq!(attr(&out, "text", "font-size"), font, "{name}");
        assert!(
            out.contains(&format!(
                "fill=\"{colour}\" text-anchor=\"middle\" dominant-baseline=\"central\">O<"
            )),
            "{name}: {out}"
        );
        assert!(
            out.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\""),
            "{name}"
        );
        assert!(
            out.split('>').next().unwrap().contains("pt\" height="),
            "{name}: the size is in points"
        );
        widths.push(attr(&out, "svg", "width"));
    }
    assert!(widths[1] < widths[2] && widths[2] < widths[0], "{widths:?}");
    // the screen follows the style too: thinner lines in acs-1996 than in slides at the same zoom
    let width_of_first_line = |e: &Editor| e.scene().data[5];
    e.set_style("acs-1996").unwrap();
    let thin = width_of_first_line(&e);
    e.set_style("slides").unwrap();
    assert!(width_of_first_line(&e) > thin);
    // and the style is not part of the document
    let doc = e.doc().to_json();
    e.set_style("mzlab").unwrap();
    assert_eq!(e.doc().to_json(), doc);
    assert!(!e.session.can_undo() || e.session.steps() == 1);
    e.pointer_move(0.0, 0.0, MOUSE);
}

#[test]
fn unknown_keys_survive_a_round_and_newer_versions_are_said_to_be_newer() {
    // a file as a later mzDraw of the same version might write it: keys this one does not know, at both levels
    let newer = r#"{"editor":{"snap":15},"format":"mzdraw-settings/1","shortcuts":{"w":"bond:up"},"style":{"arrow":{"head":4.5},"bond_length":20.0,"labels":"terminal","preset":"acs-1996"},"templates":[]}"#;
    let s = Style::from_text(newer).unwrap();
    assert_eq!(s.bond_length, 20.0);
    assert_eq!(s.unknown.file.len(), 3);
    assert_eq!(s.unknown.style.len(), 2);
    // written back: every unknown key with its value, every known field, alphabetical order, the same bytes each time
    let out = s.to_json();
    assert_eq!(
        out,
        r#"{"editor":{"snap":15},"format":"mzdraw-settings/1","shortcuts":{"w":"bond:up"},"style":{"arrow":{"head":4.5},"bold_width":2.0,"bond_length":20.0,"bond_spacing":0.18,"colour":"mono","font_size":10.0,"hash_spacing":2.5,"labels":"terminal","line_width":0.6,"margin":1.6,"preset":"acs-1996","subscript_scale":0.7},"templates":[]}"#
    );
    let again = Style::from_text(&out).unwrap();
    assert_eq!(again, s);
    assert_eq!(again.to_json(), out);
    // a known key never comes back from the unknown ones, and the format is always this version's
    let mut edited = again.clone();
    edited.bond_length = 25.0;
    assert!(
        edited.to_json().contains(r#""bond_length":25.0"#)
            && edited.to_json().contains(r#""labels":"terminal""#)
    );
    // a preset has nothing unknown
    assert!(Style::preset("slides").unwrap().unknown.file.is_empty());
    // the version rule: version 1 is read; a higher version is a file of a newer mzDraw; anything else is no settings file
    for (format, key) in [
        ("mzdraw-settings/2", "error.draw2.styleNewer"),
        ("mzdraw-settings/17", "error.draw2.styleNewer"),
        ("mzdraw-settings/0", "error.draw2.style"),
        ("mzdraw-settings/01", "error.draw2.style"),
        ("mzdraw-settings/1.5", "error.draw2.style"),
        ("mzdraw-settings/", "error.draw2.style"),
        ("mzdraw-settings", "error.draw2.style"),
        ("mzdraw-settings/-1", "error.draw2.style"),
        ("mzdraw-settings/99999999999999999999", "error.draw2.style"),
        ("mzdraw/1", "error.draw2.style"),
        ("other/2", "error.draw2.style"),
    ] {
        let text = format!(r#"{{"format":"{format}","style":{{}}}}"#);
        assert_eq!(Style::from_text(&text).unwrap_err(), key, "{format}");
    }
    // the same rule for the document
    let doc = |format: &str| format!(r#"{{"format":"{format}","next":1,"atoms":[],"bonds":[]}}"#);
    assert!(Document::from_json(&doc("mzdraw/1")).is_ok());
    assert_eq!(
        Document::from_json(&doc("mzdraw/2")).unwrap_err(),
        "error.draw2.documentNewer"
    );
    for format in [
        "mzdraw/0",
        "mzdraw/01",
        "mzdraw",
        "mzdraw/1.0",
        "mzdraw-settings/1",
        "mzdraw/x",
    ] {
        assert_eq!(
            Document::from_json(&doc(format)).unwrap_err(),
            "error.draw2.document",
            "{format}"
        );
    }
}
