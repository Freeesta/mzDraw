//! Carbon labels, hydrogen modes and the hash style (`style.carbon_labels`, `style.hydrogens`, `style.hash_style`): the words, the
//! file, and what the SVG (the same display list the screen paints) draws for each of them. Synthetic structures only.

use mzdraw::settings::{CarbonLabels, Style, PRESETS};
use mzdraw::Editor;
use serde_json::{json, Value};

const KEYS: [(&str, &[&str]); 3] = [
    ("style.carbon_labels", &["none", "terminal", "all"]),
    ("style.hydrogens", &["hetero", "all"]),
    ("style.hash_style", &["wedge", "lines"]),
];

/// The SVG of a structure under some settings, through the replay request (the route of `mzlab-cli mol draw`).
fn svg(import: &str, log: Value, set: &[&str]) -> String {
    let r = json!({"import": import, "log": log, "set": set, "out": "svg"});
    mzdraw::replay::replay(&r.to_string()).unwrap()
}

fn std_report(import: &str, log: Value, set: &[&str]) -> Value {
    let r = json!({"import": import, "log": log, "set": set, "out": "std"});
    serde_json::from_str(&mzdraw::replay::replay(&r.to_string()).unwrap()).unwrap()
}

/// The text of the atom labels, in drawing order, as one string (`CH3OH`).
fn words(svg: &str) -> String {
    svg.split("<text ")
        .skip(1)
        .map(|t| {
            let a = t.find('>').unwrap() + 1;
            t[a..t.find("</text>").unwrap()].to_string()
        })
        .collect()
}

fn attr(tag: &str, name: &str) -> f64 {
    let k = format!(" {name}=\"");
    let i = tag.find(&k).unwrap() + k.len();
    tag[i..i + tag[i..].find('"').unwrap()].parse().unwrap()
}

/// Every `<line>` of the SVG as (x1, y1, x2, y2, stroke width).
fn lines(svg: &str) -> Vec<[f64; 5]> {
    svg.split("<line ")
        .skip(1)
        .map(|t| {
            let t = format!(" {}", &t[..t.find("/>").unwrap()]);
            let t = t.as_str();
            [
                attr(t, "x1"),
                attr(t, "y1"),
                attr(t, "x2"),
                attr(t, "y2"),
                attr(t, "stroke-width"),
            ]
        })
        .collect()
}

/// The rungs of the hash across the horizontal bond to the right of the centre: the vertical lines to the right of the vertical
/// bonds (to the fluorine and the bromine), which stand on the centre.
fn hashes(svg: &str) -> Vec<[f64; 5]> {
    let vertical: Vec<[f64; 5]> = lines(svg)
        .into_iter()
        .filter(|l| l[0] == l[2] && l[1] != l[3])
        .collect();
    let centre = vertical.iter().map(|l| l[0]).fold(f64::MAX, f64::min);
    vertical
        .into_iter()
        .filter(|l| l[0] > centre + 0.5)
        .collect()
}

fn gaps_are_in_the_limits(rungs: &[[f64; 5]], what: &str) {
    let w = rungs[0][4];
    for pair in rungs.windows(2) {
        let gap = pair[1][0] - pair[0][0] - w;
        assert!(
            gap >= 2.0 * w - 0.02 && gap <= 4.0 * w + 0.02,
            "{what}: gap {gap} for line width {w}"
        );
    }
}

#[test]
fn every_word_is_taken_and_a_word_outside_the_schema_is_refused_with_its_key() {
    for (key, values) in KEYS {
        for v in values {
            let mut s = Style::default();
            s.apply(key, v).unwrap();
            assert_eq!(s.value(key).as_deref(), Some(*v), "{key}={v}");
            assert_eq!(s.choice(key), Some(*v));
        }
        let mut s = Style::default();
        assert_eq!(s.apply(key, "none-such"), Err("error.draw2.styleValue"));
        assert_eq!(s, Style::default(), "a refusal changes nothing");
    }
    // there is no `none` for the hydrogens: a label without hydrogens says the atom has none (GR-2.1.1)
    assert_eq!(
        Style::default().apply("style.hydrogens", "none"),
        Err("error.draw2.styleValue")
    );
    assert_eq!(
        Style::default().apply("style.carbon_label", "all"),
        Err("error.draw2.styleKey")
    );
    // the schema offers exactly these words, the default first
    let schema: Value = serde_json::from_str(&mzdraw::settings::schema_json()).unwrap();
    for (key, values) in KEYS {
        let row = schema["choices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["key"] == key)
            .unwrap();
        assert_eq!(row["values"], json!(values));
        assert_eq!(row["default"], values[0]);
    }
}

#[test]
fn the_default_file_keeps_its_bytes_and_a_set_style_round_trips() {
    for name in PRESETS {
        let s = Style::preset(name).unwrap();
        assert_eq!(s.carbon_labels, CarbonLabels::None, "{name}");
        assert!(!s.hydrogens_all && !s.hash_lines, "{name}");
        let text = s.to_json();
        for key in ["carbon_labels", "hydrogens", "hash_style"] {
            assert!(!text.contains(key), "{name}: {text}");
        }
    }
    let mut s = Style::preset("acs-1996").unwrap();
    for (k, v) in [
        ("carbon_labels", "all"),
        ("hydrogens", "all"),
        ("hash_style", "lines"),
    ] {
        s.set(k, v).unwrap();
    }
    let text = s.to_json();
    assert!(text.contains(r#""carbon_labels":"all""#), "{text}");
    assert!(text.contains(r#""hash_style":"lines""#), "{text}");
    assert!(text.contains(r#""hydrogens":"all""#), "{text}");
    let back = Style::from_text(&text).unwrap();
    assert_eq!(back, s);
    assert_eq!(back.to_json(), text);
    // a file with a word outside the schema is refused, not half read
    for bad in [
        r#"{"format":"mzdraw-settings/1","style":{"hash_style":"dots"}}"#,
        r#"{"format":"mzdraw-settings/1","style":{"hydrogens":"none"}}"#,
        r#"{"format":"mzdraw-settings/1","style":{"carbon_labels":"some"}}"#,
    ] {
        assert_eq!(Style::from_text(bad), Err("error.draw2.style"), "{bad}");
    }
    // `--set` and the file give the same style
    let mut by_set = Style::default();
    by_set.apply("style.carbon_labels", "terminal").unwrap();
    let file = r#"{"format":"mzdraw-settings/1","style":{"carbon_labels":"terminal"}}"#;
    assert_eq!(Style::from_text(file).unwrap(), by_set);
}

#[test]
fn a_terminal_carbon_is_written_as_ch3_and_the_bond_stops_short_of_it() {
    let none = svg("CCCO", json!([]), &[]);
    let term = svg("CCCO", json!([]), &["style.carbon_labels=terminal"]);
    assert_eq!(words(&none), "OH");
    assert_eq!(words(&term), "CH3OH", "the chain end, not the middle");
    // the first bond runs from the carbon at the left: it starts at the atom without a label, short of the label with one
    let (n, t) = (lines(&none)[0], lines(&term)[0]);
    assert!(t[0] > n[0] + 5.0, "{n:?} {t:?}");
    assert_eq!(t[1..], n[1..], "only the start of that bond moves");
}

#[test]
fn every_carbon_labelled_shows_its_hydrogens_only_where_the_style_says() {
    let hetero = svg("CCCO", json!([]), &["style.carbon_labels=all"]);
    let all = svg(
        "CCCO",
        json!([]),
        &["style.carbon_labels=all", "style.hydrogens=all"],
    );
    // the carbon with two bonds is the bare symbol under `hetero`; the end carbon always carries its hydrogens
    assert_eq!(words(&hetero), "CH3CCOH");
    assert_eq!(words(&all), "CH3CH2CH2OH");
    // hydrogens=all does nothing where no carbon is labelled by the style
    assert_eq!(
        svg("CCCO", json!([]), &["style.hydrogens=all"]),
        svg("CCCO", json!([]), &[])
    );
    // acetic acid: the carboxyl carbon has no hydrogen to write
    let acid = svg("CC(=O)O", json!([]), &["style.carbon_labels=all"]);
    assert_eq!(words(&acid), "CH3COOH");
}

#[test]
fn a_carbon_with_something_to_read_is_labelled_whatever_the_style_says() {
    for mode in ["none", "terminal", "all"] {
        let set = format!("style.carbon_labels={mode}");
        assert_eq!(words(&svg("C", json!([]), &[&set])), "CH4", "{mode}");
        assert_eq!(words(&svg("[CH3+]", json!([]), &[&set])), "CH3+", "{mode}");
    }
}

#[test]
fn a_carbon_between_collinear_bonds_is_labelled_with_the_carbon_labels_on() {
    let none = svg("CC#CC", json!([]), &[]);
    let term = svg("CC#CC", json!([]), &["style.carbon_labels=terminal"]);
    assert_eq!(words(&none), "", "the look the editor had");
    assert_eq!(words(&term), "CH3CCCH3");
}

#[test]
fn a_hash_as_lines_has_equal_rungs_across_the_bond_in_the_gaps_of_gr_1_3() {
    let centre = json!([{"id": 7, "op": "bond.set", "s": 1}, {"id": 8, "op": "bond.set", "s": 2}]);
    // bond 8 runs to the chlorine, to the right
    let wedge = svg("CC(F)(Cl)Br", centre.clone(), &[]);
    let by_lines = svg("CC(F)(Cl)Br", centre, &["style.hash_style=lines"]);
    let rungs = hashes(&by_lines);
    assert!(rungs.len() >= 3, "{rungs:?}");
    let len = |l: &[f64; 5]| (l[3] - l[1]).abs();
    assert!(
        rungs
            .iter()
            .all(|l| (len(l) - len(&rungs[0])).abs() < 0.011),
        "equal length: {rungs:?}"
    );
    gaps_are_in_the_limits(&rungs, "default bond");
    // the hashed wedge of the default widens; its rungs differ
    let widening = hashes(&wedge);
    assert!(widening.len() >= 3);
    assert!(len(&widening[widening.len() - 1]) > len(&widening[0]) + 1.0);
    assert_ne!(wedge, by_lines);
    // the solid wedge is no hash: the style leaves it alone
    let solid = json!([{"id": 7, "op": "bond.set", "s": 1}]);
    assert_eq!(
        svg("CC(F)(Cl)Br", solid.clone(), &[]),
        svg("CC(F)(Cl)Br", solid, &["style.hash_style=lines"])
    );
}

#[test]
fn a_longer_bond_keeps_the_gaps_of_gr_1_3_by_having_more_rungs() {
    let hash = json!([{"id": 8, "op": "bond.set", "s": 2}]);
    let mut counts = Vec::new();
    // a bond too short for three rungs 2 to 4 widths apart keeps three rungs (GR-1.3 asks for at least three): the gap gives way
    let short = svg(
        "CC(F)(Cl)Br",
        hash.clone(),
        &["style.hash_style=lines", "style.bond_length=20"],
    );
    assert_eq!(hashes(&short).len(), 3);
    for bond in ["30", "60", "120"] {
        let s = svg(
            "CC(F)(Cl)Br",
            hash.clone(),
            &[
                "style.hash_style=lines",
                &format!("style.bond_length={bond}"),
            ],
        );
        let rungs = hashes(&s);
        gaps_are_in_the_limits(&rungs, &format!("bond {bond}"));
        counts.push(rungs.len());
    }
    assert!(
        counts.windows(2).all(|p| p[0] <= p[1]) && counts[2] > counts[0],
        "{counts:?}"
    );
}

#[test]
fn the_drawing_checks_warn_for_parallel_lines_only_towards_a_non_terminal_atom() {
    // atoms 1 C, 2 C, 3 O, 4 C, 5 O, 6 C, 7 C; bond 12 runs from the centre 4 to the CH2 (6), bond 11 to the hydroxyl (5)
    let find = |r: &Value| {
        r["checks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["key"] == "std.stereo.hashLines")
            .cloned()
            .collect::<Vec<_>>()
    };
    let inner = json!([{"id": 12, "op": "bond.set", "s": 2}]);
    let end = json!([{"id": 11, "op": "bond.set", "s": 2}]);
    let lines_ = ["style.hash_style=lines"];
    let found = find(&std_report("CC(O)C(O)CC", inner.clone(), &lines_));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["bonds"], json!([12]));
    assert_eq!(found[0]["atoms"], json!([6]));
    assert_eq!(found[0]["grade"], "warn");
    assert!(find(&std_report("CC(O)C(O)CC", end, &lines_)).is_empty());
    // a hashed wedge is allowed anywhere
    assert!(find(&std_report("CC(O)C(O)CC", inner, &[])).is_empty());
}

#[test]
fn the_editor_follows_the_keys_on_the_screen_and_in_the_file_by_one_code_path() {
    let mut e = Editor::default();
    e.import("CCCO").unwrap();
    let before = e.scene();
    e.set("style.carbon_labels=terminal").unwrap();
    let after = e.scene();
    assert_ne!(
        before.strings, after.strings,
        "the label is a string of the scene"
    );
    assert!(after.strings.iter().any(|s| s == "C"));
    // the saved SVG is the scene's twin
    assert_eq!(words(&e.save("svg").unwrap()), "CH3OH");
    // a refusal leaves the style as it was
    assert_eq!(
        e.set("style.carbon_labels=some"),
        Err("error.draw2.styleValue")
    );
    assert_eq!(e.style.carbon_labels, CarbonLabels::Terminal);
}
