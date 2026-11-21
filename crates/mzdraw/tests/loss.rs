//! plan-22: what a format loses. The table of `loss.rs` is held to the real round trip (write, read back, compare feature by
//! feature with `loss::observed`, which does not look at the table) over random drawings and every format.

use mzdraw::doc::{Atom, Bond, Document, Obj, ObjKind};
use mzdraw::loss::{loss, observed, FORMATS};
use mzdraw::{io, Editor};
use std::collections::BTreeSet;

/// xorshift: repeatable pseudo-random choices, not a dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as usize
    }
}

fn atom(id: u32, el: &str, x: f64, y: f64) -> Atom {
    Atom {
        id,
        el: el.into(),
        x,
        y,
        chg: 0,
        iso: 0,
        rad: 0,
        h: None,
        lab: String::new(),
        map: 0,
        more: vec![],
    }
}

/// A drawing of unit bonds on a hexagonal lattice, one or two pieces, with features of every kind in random places; `rich` adds
/// arrows, plus signs, texts, data, an annotation, wedges and an unknown field.
fn random_doc(seed: u64, rich: bool) -> Document {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut d = Document::default();
    let els = ["C", "C", "C", "N", "O", "S"];
    let n = 2 + r.next(8);
    let mut id = 1u32;
    let mut taken: Vec<(i64, i64)> = Vec::new();
    for piece in 0..1 + r.next(2) {
        let (ox, oy) = (piece as f64 * 20.0, 0.0);
        let first = d.atoms.len();
        for k in 0..n {
            let (x, y, from) = if k == 0 {
                (ox, oy, None)
            } else {
                let parent = &d.atoms[first + r.next(d.atoms.len() - first)];
                let ang = (r.next(6) as f64) * std::f64::consts::PI / 3.0;
                (
                    mzdraw::doc::round4(parent.x + ang.cos()),
                    mzdraw::doc::round4(parent.y + ang.sin()),
                    Some(parent.id),
                )
            };
            let key = ((x * 1000.0).round() as i64, (y * 1000.0).round() as i64);
            if taken.contains(&key) {
                continue;
            }
            taken.push(key);
            d.atoms.push(atom(id, els[r.next(els.len())], x, y));
            if let Some(p) = from {
                let o = 1 + (r.next(4) == 0) as u8;
                // a mark is on a single bond
                let s = if rich && o == 1 && r.next(3) == 0 {
                    1 + r.next(3) as u8
                } else {
                    0
                };
                d.bonds.push(Bond {
                    id: id + 1000,
                    a: p,
                    b: id,
                    o,
                    s,
                    more: vec![],
                });
            }
            id += 1;
        }
    }
    // atom features
    let degree = |d: &Document, id: u32| d.bonds.iter().filter(|b| b.a == id || b.b == id).count();
    let mut map = 0;
    let mut ann_done = false;
    for i in 0..d.atoms.len() {
        let leaf = degree(&d, d.atoms[i].id) == 1;
        let a_id = d.atoms[i].id;
        let pick = r.next(10);
        let a = &mut d.atoms[i];
        match pick {
            0 => a.chg = if r.next(2) == 0 { 1 } else { -1 },
            1 => a.iso = [2, 13, 15, 18][r.next(4)],
            2 if a.el == "C" => a.rad = 2,
            3 => {
                map += 1;
                a.map = map;
            }
            4 if leaf => a.lab = ["R1", "R2", "R", "Me", "Ph", "Boc"][r.next(6)].into(),
            5 if rich => a
                .more
                .push(("data".into(), format!("[[\"name\",\"v{}\"]]", r.next(9)))),
            6 if rich && !ann_done => {
                ann_done = true;
                a.more.push((
                    "ann".into(),
                    r#"[{"evidence":[{"kind":"formula"}],"level":"2b"}]"#.into(),
                ));
            }
            7 if rich && r.next(3) == 0 => a.more.push(("zz".into(), "1".into())),
            _ => {}
        }
        let _ = a_id;
    }
    for a in &mut d.atoms {
        a.more.sort();
    }
    // objects, far to the right
    if rich {
        let mut oid = id + 2000;
        let first = d.atoms[0].id;
        for k in 0..r.next(3) {
            let y = 5.0 + k as f64 * 2.0;
            d.objects.push(Obj {
                id: oid,
                t: ObjKind::Arrow,
                x: 40.0,
                y,
                x2: 43.0,
                y2: y,
                text: if r.next(2) == 0 {
                    "cat".into()
                } else {
                    String::new()
                },
                from: if r.next(2) == 0 { first } else { 0 },
                to: 0,
                more: vec![],
            });
            oid += 1;
        }
        if r.next(2) == 0 {
            d.objects.push(Obj {
                id: oid,
                t: ObjKind::Plus,
                x: 38.0,
                y: 5.0,
                x2: 0.0,
                y2: 0.0,
                text: String::new(),
                from: 0,
                to: 0,
                more: vec![],
            });
            oid += 1;
        }
        if r.next(2) == 0 {
            d.objects.push(Obj {
                id: oid,
                t: ObjKind::Text,
                x: 44.0,
                y: 9.0,
                x2: 0.0,
                y2: 0.0,
                text: "note".into(),
                from: 0,
                to: 0,
                more: vec![],
            });
            oid += 1;
        }
        d.next = oid + 1;
    } else {
        d.next = id + 2000;
    }
    d
}

fn round_trip(d: &Document, format: &str) -> Result<Document, String> {
    let mut e = Editor::default();
    e.session.doc = d.clone();
    let text = e.save(format).map_err(|k| k.to_string())?;
    io::import(&text).map(|r| r.0).map_err(|k| k.to_string())
}

/// For random drawings and every format: what the round trip lost is in `loss`, and what `loss` lists is a real difference.
/// For random drawings and every format: what the round trip lost is in `loss`, and what `loss` lists is a real difference.
/// For random drawings and every format: what the round trip lost is in `loss`, and what `loss` lists is a real difference.
/// For random drawings and every format: what the round trip lost is in `loss`, and what `loss` lists is a real difference.
#[test]
fn the_table_equals_the_round_trip() {
    let mut checked = 0;
    for seed in 1..=500u64 {
        let d = random_doc(seed, seed % 3 != 0);
        for format in FORMATS {
            let listed = match loss(&d, format) {
                Ok(l) => l,
                // a drawing the writer refuses (a reaction needs one arrow) has no file to lose anything from
                Err(k) => {
                    assert!(
                        matches!(k, "error.draw2.rxnArrow" | "error.draw2.smilesOut"),
                        "seed {seed} {format}: {k}"
                    );
                    continue;
                }
            };
            let back = match round_trip(&d, format) {
                Ok(b) => b,
                Err(k) => panic!("seed {seed} {format}: the file does not read back: {k}"),
            };
            let seen: BTreeSet<&str> = observed(&d, &back).into_iter().collect();
            let said: BTreeSet<&str> = listed.iter().map(|l| l.what).collect();
            assert_eq!(
                seen,
                said,
                "seed {seed} {format}: observed {seen:?}, listed {said:?}\n{}",
                d.to_json()
            );
            checked += 1;
        }
    }
    assert!(checked > 1500, "{checked}");
}

fn parse(json: &str) -> Document {
    Document::from_json(json).unwrap()
}

/// A drawing with an arrow, an annotation and a wavy bond: each is listed with its id where the format drops it.
const SAMPLE: &str = r#"{"format":"mzdraw/1","next":9,"atoms":[
  {"id":1,"el":"C","x":0.0,"y":0.0,"ann":[{"evidence":[{"kind":"formula"}],"level":"2b"}]},
  {"id":2,"el":"C","x":1.0,"y":0.0},{"id":3,"el":"O","x":1.5,"y":0.866}],
 "bonds":[{"id":4,"a":1,"b":2,"o":1},{"id":5,"a":2,"b":3,"o":1,"s":3}],
 "objects":[{"id":6,"t":"arrow","x":4.0,"y":0.0,"x2":6.0,"y2":0.0}]}"#;

type Item = (&'static str, Vec<u32>, Vec<u32>, Vec<u32>);

fn items(format: &str) -> Vec<Item> {
    loss(&parse(SAMPLE), format)
        .unwrap()
        .into_iter()
        .map(|l| (l.what, l.atoms, l.bonds, l.objects))
        .collect()
}

#[test]
fn an_arrow_in_a_molfile_an_annotation_in_smiles_and_a_wavy_bond_are_listed_with_their_ids() {
    let v2000 = items("molfile");
    assert!(
        v2000.contains(&("object.arrow", vec![], vec![], vec![6])),
        "{v2000:?}"
    );
    assert!(
        v2000.contains(&("annotation", vec![1], vec![], vec![])),
        "{v2000:?}"
    );
    let smiles = items("smiles");
    assert!(
        smiles.contains(&("annotation", vec![1], vec![], vec![])),
        "{smiles:?}"
    );
    assert!(
        smiles.contains(&("object.arrow", vec![], vec![], vec![6])),
        "{smiles:?}"
    );
    // the wavy bond leaves a carbon with no stereo of its own: the file writes the mark, the reader does not derive it again
    assert!(
        v2000.contains(&("wedge.wavy", vec![], vec![5], vec![])),
        "{v2000:?}"
    );
    // this small chain is laid out by the SMILES reader as it was drawn: no geometry is listed (a bigger drawing off the lattice is)
    assert!(smiles.iter().all(|i| i.0 != "geometry"), "{smiles:?}");
    // the reaction file carries the arrow, not the annotation
    let rxn = items("rxn");
    assert!(rxn.iter().all(|i| i.0 != "object.arrow"), "{rxn:?}");
    assert!(rxn.iter().any(|i| i.0 == "annotation"), "{rxn:?}");
    // our own formats carry everything
    for f in ["svg", "mzdraw"] {
        assert!(items(f).is_empty(), "{f}");
    }
    // the SDF carries the annotation
    assert!(items("sdf").iter().all(|i| i.0 != "annotation"));
}

#[test]
fn the_items_come_ordered_by_what_then_by_the_smallest_id() {
    let d = parse(
        r#"{"format":"mzdraw/1","next":9,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0}],"bonds":[],
        "objects":[{"id":7,"t":"plus","x":3.0,"y":0.0},{"id":5,"t":"plus","x":4.0,"y":0.0},{"id":6,"t":"text","x":5.0,"y":0.0,"text":"a"}]}"#,
    );
    let l = loss(&d, "molfile").unwrap();
    let got: Vec<(&str, Vec<u32>)> = l.iter().map(|i| (i.what, i.objects.clone())).collect();
    assert_eq!(
        got,
        vec![
            ("object.plus", vec![5]),
            ("object.plus", vec![7]),
            ("object.text", vec![6])
        ]
    );
    let text = mzdraw::loss::json("molfile", &l);
    assert!(text.starts_with(r#"{"format":"loss/1","target":"molfile","items":[{"what":"object.plus","atoms":[],"bonds":[],"objects":[5]}"#));
}

/// A table that forgets a feature of the drawing (here: says the SMILES carries the objects) fails the comparison with the round trip.
#[test]
fn a_table_that_forgets_objects_for_smiles_fails_the_comparison() {
    let d = parse(SAMPLE);
    let back = round_trip(&d, "smiles").unwrap();
    let seen = observed(&d, &back);
    assert!(seen.contains(&"object.arrow"));
    // the honest table lists it, so a table that says "carried" would differ from `observed`
    let said: Vec<&str> = loss(&d, "smiles").unwrap().iter().map(|l| l.what).collect();
    assert!(said.contains(&"object.arrow"));
    let forgotten: Vec<&str> = said.into_iter().filter(|w| *w != "object.arrow").collect();
    assert_ne!(
        forgotten,
        seen.iter()
            .copied()
            .filter(|w| *w != "geometry")
            .collect::<Vec<_>>()
    );
}

#[test]
fn every_feature_the_document_can_hold_has_a_row_and_every_format_a_column() {
    for what in [
        "abbreviation",
        "annotation",
        "arrow.anchor",
        "arrow.text",
        "atom.charge",
        "atom.isotope",
        "atom.map",
        "atom.radical",
        "data",
        "geometry",
        "object.arrow",
        "object.plus",
        "object.text",
        "rlabel",
        "unknown",
        "wedge.hashed",
        "wedge.solid",
        "wedge.wavy",
    ] {
        for f in FORMATS {
            assert!(mzdraw::loss::carry(what, f).is_some(), "{what} {f}");
        }
    }
    assert_eq!(mzdraw::loss::TABLE.len(), 18);
}

#[test]
fn an_unknown_format_and_a_refusing_writer_are_errors() {
    let d = parse(SAMPLE);
    assert_eq!(loss(&d, "pdf").unwrap_err(), "error.draw2.request");
    // a reaction file needs one arrow
    let none = parse(
        r#"{"format":"mzdraw/1","next":2,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0}],"bonds":[]}"#,
    );
    assert_eq!(loss(&none, "rxn").unwrap_err(), "error.draw2.rxnArrow");
}

/// Hostile size: a drawing above the cap is refused at once; one at the cap is listed in a bounded time.
#[test]
fn a_huge_drawing_is_refused_and_one_at_the_cap_is_fast() {
    let mut d = Document::default();
    for i in 1..=100_000u32 {
        d.atoms
            .push(atom(i, "C", f64::from(i % 300), f64::from(i / 300)));
    }
    d.next = 100_001;
    let t = std::time::Instant::now();
    let l = loss(&d, "molfile").unwrap();
    assert!(l.is_empty());
    d.atoms.push(atom(100_001, "C", 0.0, 0.0));
    assert_eq!(loss(&d, "molfile").unwrap_err(), "error.draw2.lossSize");
    assert!(t.elapsed().as_secs() < 5, "{:?}", t.elapsed());
}

/// The table of `docs/mzdraw-formats.md` is the table of the code (`MZDRAW_BLESS=1` rewrites it).
#[test]
fn the_doc_table_is_the_table_of_the_code() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/mzdraw-formats.md");
    let text = std::fs::read_to_string(&path).unwrap();
    let (begin, end) = ("<!-- loss-table:begin -->\n", "<!-- loss-table:end -->");
    let from = text.find(begin).unwrap() + begin.len();
    let to = text.find(end).unwrap();
    let want = mzdraw::loss::table_markdown();
    if std::env::var("MZDRAW_BLESS").is_ok() {
        std::fs::write(&path, format!("{}{want}{}", &text[..from], &text[to..])).unwrap();
        return;
    }
    assert_eq!(
        &text[from..to],
        want,
        "docs/mzdraw-formats.md drifted from loss.rs"
    );
}
