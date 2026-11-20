//! Labels that do not crowd: a bond stops short of the charge, the hydrogens or the mass number that stand on its side of an
//! element symbol, and the hydrogens on the left stand clear of the mass number. Synthetic drawings only.

use mzdraw::Document;

/// A two-atom drawing: the first atom with its extra fields at the origin, a carbon at (`x`, `y`), one single bond.
fn pair(first: &str, x: f64, y: f64) -> Document {
    Document::from_json(&format!(
        r#"{{"format":"mzdraw/1","next":4,"atoms":[{{"id":1,{first},"x":0.0,"y":0.0}},{{"id":2,"el":"C","x":{x:?},"y":{y:?}}}],"bonds":[{{"id":3,"a":1,"b":2,"o":1}}]}}"#
    ))
    .unwrap()
}

fn attr(tag: &str, name: &str) -> f64 {
    let from = tag.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
    tag[from..from + tag[from..].find('"').unwrap()]
        .parse()
        .unwrap()
}

/// The texts of an SVG as (string, x, font size, whether anchored at its start) and its one bond line as (x1, y1, x2, y2).
type Drawn = (Vec<(String, f64, f64, bool)>, (f64, f64, f64, f64));

fn drawn(doc: &Document) -> Drawn {
    let svg = mzdraw::svg::to_svg(doc);
    let texts = (svg.lines().filter(|l| l.starts_with("<text")))
        .map(|l| {
            let s = &l[l.find('>').unwrap() + 1..l.rfind("</text>").unwrap()];
            (
                s.to_string(),
                attr(l, "x"),
                attr(l, "font-size"),
                l.contains("text-anchor=\"start\""),
            )
        })
        .collect();
    let lines: Vec<&str> = svg.lines().filter(|l| l.starts_with("<line")).collect();
    assert_eq!(lines.len(), 1, "{svg}");
    let l = lines[0];
    (
        texts,
        (attr(l, "x1"), attr(l, "y1"), attr(l, "x2"), attr(l, "y2")),
    )
}

/// Where a text anchored at its start ends at the least: a sign or a digit is more than half its font size wide.
fn end(t: &(String, f64, f64, bool)) -> f64 {
    assert!(t.3, "{t:?}");
    t.1 + t.0.chars().count() as f64 * t.2 * 0.5
}

fn text<'a>(d: &'a Drawn, s: &str) -> &'a (String, f64, f64, bool) {
    d.0.iter()
        .find(|t| t.0 == s)
        .unwrap_or_else(|| panic!("no {s} in {d:?}"))
}

#[test]
fn a_bond_stops_short_of_the_charge_on_its_side() {
    // O(-) with its bond leaving to the right: the minus stands at the upper right, the bond starts after it
    let plain = drawn(&pair(r#""el":"O""#, 1.0, 0.0));
    let charged = drawn(&pair(r#""el":"O","chg":-1"#, 1.0, 0.0));
    let minus = text(&charged, "\u{2212}");
    assert!(charged.1 .0 >= end(minus), "{charged:?}");
    assert!(
        charged.1 .0 > plain.1 .0 + 1.0,
        "further than without a charge"
    );
    assert_eq!(charged.1 .2, plain.1 .2, "the other end does not move");
    // a charge of two is wider and is cleared too
    let two = drawn(&pair(r#""el":"O","chg":-2"#, 1.0, 0.0));
    assert!(two.1 .0 >= end(text(&two, "2\u{2212}")) && two.1 .0 > charged.1 .0);
    // a bond that leaves downwards or to the left passes clear of the raised charge: it starts where it did
    for (x, y) in [(0.0, -1.0), (0.5, -0.866), (-1.0, 0.0), (-0.866, 0.5)] {
        let (a, b) = (
            drawn(&pair(r#""el":"O""#, x, y)),
            drawn(&pair(r#""el":"O","chg":-1"#, x, y)),
        );
        assert_eq!(a.1, b.1, "towards ({x}, {y})");
    }
    // going up to the right it is shortened, and never by more than the charge is wide
    let (a, b) = (
        drawn(&pair(r#""el":"O""#, 0.866, 0.5)),
        drawn(&pair(r#""el":"O","chg":-1"#, 0.866, 0.5)),
    );
    assert!(b.1 .0 > a.1 .0 && b.1 .0 - a.1 .0 <= minus.2);
}

#[test]
fn a_bond_stops_short_of_the_hydrogens_and_their_charge() {
    // H3N(+) with its bond to the right: the hydrogens go left, the plus stays right, the bond starts after the plus
    let d = drawn(&pair(r#""el":"N","chg":1"#, 1.0, 0.0));
    let (h, n, plus) = (text(&d, "H"), text(&d, "N"), text(&d, "+"));
    assert!(end(h) < n.1 && d.1 .0 >= end(plus), "{d:?}");
    // NH3(+) with its bond to the left: nothing stands on the left, the bond is as for a bare N
    let left = drawn(&pair(r#""el":"N","chg":1"#, -1.0, 0.0));
    let bare = drawn(&pair(r#""el":"N","h":0"#, -1.0, 0.0));
    assert_eq!(left.1, bare.1);
    // a short bond keeps more than half its length whatever stands at its end
    let short = drawn(&pair(r#""el":"N","chg":1"#, 0.3, 0.0));
    let whole = drawn(&pair(r#""el":"C","h":4"#, 0.3, 0.0));
    assert!(
        (short.1 .2 - short.1 .0) > 0.0
            && (short.1 .2 - short.1 .0) * 2.0 > (whole.1 .2 - whole.1 .0) * 0.5
    );
}

#[test]
fn hydrogens_on_the_left_stand_clear_of_the_mass_number() {
    // H3 13C with its bond to the right: H, its count, the mass number, the symbol, in that order without overlap
    let d = drawn(&pair(r#""el":"C","iso":13"#, 1.0, 0.0));
    let (h, count, iso, c) = (text(&d, "H"), text(&d, "3"), text(&d, "13"), text(&d, "C"));
    assert!(
        h.1 < count.1 && end(count) <= iso.1 && end(iso) <= c.1,
        "{d:?}"
    );
    // without a mass number the hydrogens are where they were: next to the symbol
    let plain = drawn(&pair(r#""el":"N""#, 1.0, 0.0));
    let heavy = drawn(&pair(r#""el":"N","iso":15"#, 1.0, 0.0));
    let shift = text(&plain, "H").1 - text(&heavy, "H").1;
    assert!(
        (shift
            - (text(&heavy, "N").1 - text(&plain, "N").1)
            - (end(text(&heavy, "15")) - text(&heavy, "15").1) * 1.12)
            .abs()
            < 0.3,
        "moved left by the width of the mass number: {shift}"
    );
    // hydrogens on the right are not moved by a mass number
    let right = drawn(&pair(r#""el":"N","iso":15"#, -1.0, 0.0));
    let right_plain = drawn(&pair(r#""el":"N""#, -1.0, 0.0));
    assert_eq!(
        text(&right, "H").1 - text(&right, "N").1,
        text(&right_plain, "H").1 - text(&right_plain, "N").1
    );
}
