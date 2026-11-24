//! The ids of the document drawn in the picture (`display.ids`, rdkit-compare 4): off by default, the atom ids at their atoms, the bond
//! ids beside the middle of their bonds, the same in the SVG and the PDF, kept in the settings file. Synthetic structures only.

use mzdraw::settings::Ids;
use mzdraw::{svg, Document, Editor, Style};

#[path = "support/timing.rs"]
mod timing;

fn doc(text: &str) -> Document {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e.doc().clone()
}

fn style(ids: &str) -> Style {
    let mut s = Style::default();
    s.set_display("ids", ids).unwrap();
    s
}

/// The `(x, y, text)` of every `<text>` of an SVG drawn in the muted ink, in points.
fn muted(svg: &str) -> Vec<(f64, f64, String)> {
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
    // the muted ink of the print palette (`svg::PALETTE`, index `scene::colour::MUTED`)
    let ink = "#5c5850";
    svg.split("<text")
        .skip(1)
        .filter(|e| e.contains(&format!("fill=\"{ink}\"")))
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
fn the_ids_are_off_by_default_and_each_word_draws_its_own() {
    let d = doc("CCO");
    assert_eq!(Style::default().ids, Ids::Off);
    assert!(muted(&svg::to_svg(&d)).is_empty(), "nothing by default");
    let texts = |w: &str| -> Vec<String> {
        let mut t: Vec<String> = muted(&svg::to_svg_styled(&d, &style(w)))
            .into_iter()
            .map(|x| x.2)
            .collect();
        t.sort();
        t
    };
    // ethanol: atoms 1, 2, 3, bonds 4, 5 (the ids of the document, the ones `atom.set` and `bond.set` name)
    assert_eq!(texts("atoms"), ["1", "2", "3"]);
    assert_eq!(texts("bonds"), ["4", "5"]);
    assert_eq!(texts("both"), ["1", "2", "3", "4", "5"]);
    assert_eq!(texts("off"), Vec::<String>::new());
    // the PDF draws the same words: atom ids only, bond ids only
    let pdf = |w: &str| -> String {
        (mzdraw::pdf::to_pdf(&d, &style(w)).iter())
            .map(|&c| if c < 128 { c as char } else { '?' })
            .collect()
    };
    let (atoms, bonds) = (pdf("atoms"), pdf("bonds"));
    assert!(atoms.contains("(3) Tj") && !atoms.contains("(4) Tj"));
    assert!(bonds.contains("(4) Tj") && !bonds.contains("(3) Tj"));
    // a word that is none is refused and changes nothing
    let mut s = style("atoms");
    assert_eq!(s.set_display("ids", "all"), Err("error.draw2.styleValue"));
    assert_eq!(s.ids, Ids::Atoms);
}

/// The display list of a drawing at one point per bond-length unit times `scale`, origin at the document's origin: every muted text
/// `(x, y, id)` and every line `(x1, y1, x2, y2)`.
/// A muted text of the display list, `(x, y, id)`, and a line, `(x1, y1, x2, y2)`.
type IdText = (f64, f64, u32);
type Line = (f64, f64, f64, f64);

/// What `display` reads off the display list: the ids, the lines, the anchors of the label texts and the scale.
type Display = (Vec<IdText>, Vec<Line>, Vec<(f64, f64)>, f64);

fn display(d: &Document, ids: &str) -> Display {
    use mzdraw::scene::{self, colour, Overlay, View};
    let st = style(ids);
    let scale = st.bond_length;
    let sc = scene::build(
        d,
        View {
            scale,
            ox: 0.0,
            oy: 0.0,
        },
        &Overlay::default(),
        st.metrics(scale, false),
    );
    let (mut texts, mut lines, mut labels) = (Vec::new(), Vec::new(), Vec::new());
    let v = &sc.data;
    let mut i = 0;
    while i < v.len() {
        let (op, f) = (v[i], |k: usize| f64::from(v[i + k]));
        let size = if op == scene::LINE {
            lines.push((f(1), f(2), f(3), f(4)));
            7
        } else if op == scene::TEXT {
            if v[i + 4] == colour::MUTED {
                texts.push((f(1), f(2), sc.strings[v[i + 5] as usize].parse().unwrap()));
            } else {
                labels.push((f(1), f(2)));
            }
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
        i += size;
    }
    (texts, lines, labels, scale)
}

fn seg_dist(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (ax, ay) = (b.0 - a.0, b.1 - a.1);
    let l2 = ax * ax + ay * ay;
    let t = if l2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * ax + (p.1 - a.1) * ay) / l2).clamp(0.0, 1.0)
    };
    ((p.0 - a.0 - t * ax).powi(2) + (p.1 - a.1 - t * ay).powi(2)).sqrt()
}

#[test]
fn each_id_sits_by_its_own_atom_or_bond_clear_of_the_lines_and_of_the_other_ids() {
    // a ring, a chain, labels with hydrogens and a charge, a double bond: the cases where an id could land on a line or a label
    for smiles in [
        "CCO",
        "c1ccc(cc1)C(=O)[NH3+]",
        "CC(C)(C)C=CC#N",
        "OC1CCC(N)CC1",
    ] {
        let d = doc(smiles);
        let (texts, lines, labels, scale) = display(&d, "both");
        assert_eq!(
            texts.len(),
            d.atoms.len() + d.bonds.len(),
            "{smiles}: one id per atom and per bond"
        );
        // where the atoms and the middles of the bonds are, in the same frame (`View::to_px`: y down)
        let px = |id: u32| d.atom(id).map(|a| (a.x * scale, -a.y * scale)).unwrap();
        let mid = |b: &mzdraw::doc::Bond| {
            let (p, q) = (px(b.a), px(b.b));
            ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0)
        };
        let dist =
            |p: (f64, f64), q: (f64, f64)| ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();
        for &(x, y, id) in &texts {
            if let Some(a) = d.atom(id) {
                // an atom id is nearer its own atom than any other atom
                let own = dist((x, y), px(a.id));
                for o in d.atoms.iter().filter(|o| o.id != id) {
                    assert!(
                        own < dist((x, y), px(o.id)),
                        "{smiles}: atom id {id} is nearer atom {}",
                        o.id
                    );
                }
            } else {
                // a bond id is nearer the middle of its own bond than the middle of any other bond
                let b = d.bonds.iter().find(|b| b.id == id).unwrap();
                let own = dist((x, y), mid(b));
                for o in d.bonds.iter().filter(|o| o.id != id) {
                    assert!(
                        own < dist((x, y), mid(o)),
                        "{smiles}: bond id {id} is nearer bond {}",
                        o.id
                    );
                }
            }
            // no id lies on a line of the drawing
            for &(x1, y1, x2, y2) in &lines {
                assert!(
                    seg_dist((x, y), (x1, y1), (x2, y2)) > 0.15 * scale,
                    "{smiles}: id {id} on a line"
                );
            }
        }
        // no id on a label (the anchor of each text of a label: the symbol, its hydrogens, a charge)
        for &(x, y, id) in &texts {
            for l in &labels {
                assert!(
                    dist((x, y), *l) > 0.25 * scale,
                    "{smiles}: id {id} on a label at {l:?}"
                );
            }
        }
        // no two ids on top of each other
        for (k, p) in texts.iter().enumerate() {
            for q in &texts[k + 1..] {
                assert!(
                    dist((p.0, p.1), (q.0, q.1)) > 0.2 * scale,
                    "{smiles}: ids {} and {} overlap",
                    p.2,
                    q.2
                );
            }
        }
        // the same drawing gives the same places
        assert_eq!(display(&d, "both").0, texts);
    }
}

#[test]
fn the_pdf_has_the_ids_and_the_settings_file_keeps_the_choice() {
    let d = doc("CCO");
    let pdf = mzdraw::pdf::to_pdf(&d, &style("both"));
    let text: String = pdf
        .iter()
        .map(|&c| if c < 128 { c as char } else { '?' })
        .collect();
    for id in ["1", "2", "3", "4", "5"] {
        assert!(text.contains(&format!("({id}) Tj")), "{id}");
    }
    assert!(
        !String::from_utf8_lossy(&mzdraw::pdf::to_pdf(&d, &Style::default())).contains("(4) Tj")
    );
    // the settings file writes the key only when it is not off, and reads it back
    let mut e = Editor::default();
    e.set("display.ids=bonds").unwrap();
    assert_eq!(e.get("display.ids").as_deref(), Some("bonds"));
    let file = e.save("settings").unwrap();
    assert!(file.contains("\"ids\":\"bonds\""), "{file}");
    let back = Style::from_text(&file).unwrap();
    assert_eq!(back.ids, Ids::Bonds);
    assert!(!Editor::default()
        .save("settings")
        .unwrap()
        .contains("\"ids\""));
}

#[test]
fn an_id_keeps_its_place_when_drawn_again_and_when_a_structure_is_added_far_away() {
    // stable means: the same drawing gives the same places, and an id only looks at what is drawn within two bond lengths of it, so
    // a structure added farther away moves no id of the others
    let d = doc("OC1CCC(N)CC1");
    let (first, ..) = display(&d, "both");
    assert_eq!(display(&d, "both").0, first);
    // ethanol drawn 20 bond lengths to the right (new atoms and a bond by operations)
    let mut s = mzdraw::Session::new(d.clone());
    let op = |v: serde_json::Value| mzdraw::Op::parse(&v).unwrap();
    s.run(&[
        op(serde_json::json!({"op": "atom.add", "id": 100, "el": "C", "x": 20.0, "y": 0.0})),
        op(serde_json::json!({"op": "atom.add", "id": 101, "el": "C", "x": 21.0, "y": 0.0})),
        op(serde_json::json!({"op": "atom.add", "id": 102, "el": "O", "x": 21.5, "y": 0.866})),
        op(serde_json::json!({"op": "bond.add", "id": 103, "a": 100, "b": 101})),
        op(serde_json::json!({"op": "bond.add", "id": 104, "a": 101, "b": 102})),
    ])
    .unwrap();
    let far = s.doc;
    let (after, ..) = display(&far, "both");
    for t in &first {
        assert!(
            after.contains(t),
            "id {} moved when a structure was added far away",
            t.2
        );
    }
}

#[test]
fn the_work_grows_with_the_drawing_not_with_its_square() {
    // the same structure 40 and 160 times: the work the ids add (the picture with them less the picture without) is linear: indexed
    // lookups, one label test per atom. A scan of every bond or of every placed id per id would make four times the structures cost
    // sixteen times as much. The rest of the painter is not measured here (its label test scans the bonds: a queued follow-up). The
    // least CPU time of runs taken in turns, no wall-clock bound
    let one = "OC1CCC(N)CC1C=CC#N";
    let (a, b) = (
        doc(&vec![one; 40].join(".")),
        doc(&vec![one; 160].join(".")),
    );
    let (on, off) = (style("both"), style("off"));
    let ids_cost = |d: &Document| {
        let (without, with) = timing::best_of_pair(
            1,
            5,
            || drop(svg::to_svg_styled(d, &off)),
            || drop(svg::to_svg_styled(d, &on)),
        );
        with.as_nanos().saturating_sub(without.as_nanos())
    };
    let (small, large) = (ids_cost(&a), ids_cost(&b));
    assert!(
        small > 0 && large <= 6 * small,
        "the ids of 40 structures {small} ns, of 160 structures {large} ns"
    );
}
