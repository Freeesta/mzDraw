//! plan-22: what a format loses. The table of `loss.rs` is held to the real round trip (write, read back, compare feature by
//! feature with `loss::observed`, which does not look at the table) over random drawings and every format.

use mzdraw::doc::{Atom, Bond, Document, Obj, ObjKind};
use mzdraw::loss::{loss, observed, FORMATS};
use mzdraw::{io, Editor};
use std::collections::BTreeSet;

#[path = "support/timing.rs"]
mod timing;

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
                runs: Vec::new(),
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
                runs: Vec::new(),
                from: 0,
                to: 0,
                more: vec![],
            });
            oid += 1;
        }
        if r.next(2) == 0 {
            // a formatted text: every kind of run the table has a row for
            use mzdraw::Run;
            let runs = vec![
                Run {
                    b: true,
                    ..Run::plain("B")
                },
                Run::sub("2"),
                Run {
                    i: true,
                    u: true,
                    ..Run::plain("iu")
                },
                Run::line(),
                Run {
                    color: "#cc3300".into(),
                    font: "Courier New".into(),
                    sz: 1.5,
                    ..Run::plain("big")
                },
                Run::sup("+"),
            ];
            d.objects.push(Obj {
                id: oid,
                t: ObjKind::Text,
                x: 44.0,
                y: 12.0,
                x2: 0.0,
                y2: 0.0,
                text: mzdraw::runs::plain(&runs),
                runs,
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
                runs: Vec::new(),
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
            // the answer is per kind: every occurrence of a listed kind is listed once (with its own id)
            let found = mzdraw::loss::instances(&d);
            for what in &said {
                let n = found.iter().filter(|i| i.what == *what).count();
                let m = listed.iter().filter(|l| l.what == *what).count();
                // the shape of the drawing is one item for all its atoms
                let want = if *what == "geometry" { 1 } else { n };
                assert_eq!(m, want, "seed {seed} {format} {what}");
            }
            for l in &listed {
                let ids = l.atoms.len() + l.bonds.len() + l.objects.len();
                assert!(ids >= 1, "seed {seed} {format}: {} has no id", l.what);
            }
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
        "atom.lp",
        "atom.map",
        "atom.radical",
        "bond.bdash",
        "bond.bold",
        "bond.dash",
        "bond.dative",
        "bond.either",
        "bond.hash",
        "bond.hbond",
        "bond.pos",
        "caption",
        "colour",
        "colour.object",
        "data",
        "geometry",
        "labels",
        "object.arrow",
        "object.curve",
        "object.plus",
        "object.text",
        "rlabel",
        "text.colour",
        "text.font",
        "text.lines",
        "text.script",
        "text.style",
        "text.underline",
        "unknown",
        "wedge.hashed",
        "wedge.solid",
        "wedge.wavy",
    ] {
        for f in FORMATS {
            assert!(mzdraw::loss::carry(what, f).is_some(), "{what} {f}");
        }
    }
    assert_eq!(mzdraw::loss::TABLE.len(), 38);
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

/// A zigzag chain of `n` carbons (unit-ish bonds, no two atoms on one point) with a solid wedge on every second bond.
fn chain(n: u32, wedges: bool) -> Document {
    let mut d = Document::default();
    for i in 1..=n {
        let y = f64::from(i / 1000) * 2.0 + if i % 2 == 0 { 0.5 } else { 0.0 };
        d.atoms.push(atom(i, "C", f64::from(i % 1000), y));
        if i > 1 {
            let s = u8::from(wedges && i % 2 == 0);
            d.bonds.push(Bond {
                id: n + i,
                a: i - 1,
                b: i,
                o: 1,
                s,
                more: vec![],
            });
        }
    }
    d.next = 2 * n + 2;
    d
}

/// The seconds a listing may take: a release build does the largest drawing in about a second and must stay under two; a debug build
/// (about 0.7 s per format here) is allowed ten. The scaling test below is what fails a quadratic step.
fn bound() -> u64 {
    if cfg!(debug_assertions) {
        10
    } else {
        2
    }
}

/// Hostile size: a drawing at the cap, with wedge marks (so every probe format writes and reads the whole drawing), is listed in a
/// bounded time in every format that has no atom limit of its own; one thing more is refused at once.
#[test]
fn a_drawing_at_the_cap_with_wedges_is_listed_in_time_and_one_above_it_is_refused() {
    // 25,000 atoms and 24,999 bonds are 49,999 things
    let d = chain(25_000, true);
    for format in ["molfile", "molfile-v3000", "ket", "cdxml", "cml"] {
        let t = std::time::Instant::now();
        let l = loss(&d, format).unwrap();
        assert_eq!(
            l.len(),
            12_500,
            "{format}: every wedge is a loss in a chain with no centre"
        );
        assert!(
            t.elapsed().as_secs() < bound(),
            "{format}: {:?}",
            t.elapsed()
        );
    }
    let mut over = chain(25_000, true);
    over.atoms.push(atom(60_000, "C", 5.0, 50.0));
    over.atoms.push(atom(60_001, "C", 6.0, 50.0));
    for format in FORMATS {
        let t = std::time::Instant::now();
        assert_eq!(
            loss(&over, format).unwrap_err(),
            "error.draw2.lossSize",
            "{format}"
        );
        assert!(t.elapsed().as_secs() < 1, "{format}: refused at once");
    }
}

/// The cost is linear in the drawing: four times the atoms take about four times the time (a quadratic step makes it sixteen; a bound in
/// seconds does not fail a step that is merely slow, a ratio does, and a ratio of eight leaves room for a noisy machine).
#[test]
fn four_times_the_drawing_takes_about_four_times_the_time() {
    let best = |n: u32, format: &str| {
        let d = chain(n, true);
        timing::best_of(1, 3, || {
            loss(&d, format).unwrap();
        })
        .as_secs_f64()
    };
    for format in ["molfile", "cdxml", "ket"] {
        let (small, big) = (best(6_250, format), best(25_000, format));
        // below this the clock is noise and the bound above is all there is
        if small > 0.02 {
            assert!(big / small < 8.0, "{format}: {small:.3} s then {big:.3} s");
        }
        assert!(big < f64::from(bound() as u32), "{format}: {big:.3} s");
    }
}

/// Abbreviations are written as their atoms (seven for `Boc`) and read back: a drawing of 25,000 of them would be 175,000 atoms through the
/// stereo perception, so where a round trip is needed (wedge marks) it is refused at once; below the limit it is listed in time; and
/// with no round trip needed (no mark) the cap on the drawing is all there is.
#[test]
fn abbreviations_are_counted_as_the_atoms_they_are_written_as() {
    let labelled = |n: u32, wedges: bool| {
        let mut d = chain(n, wedges);
        for a in &mut d.atoms {
            a.lab = "Boc".into();
        }
        d
    };
    let formats = ["molfile", "molfile-v3000", "ket", "cdxml", "cml"];
    // 25,000 abbreviations with marks: 175,000 atoms to write and read
    for format in formats {
        let t = std::time::Instant::now();
        assert_eq!(
            loss(&labelled(25_000, true), format).unwrap_err(),
            "error.draw2.lossSize",
            "{format}"
        );
        assert!(t.elapsed().as_secs() < 1, "{format}: refused at once");
    }
    // 5,000 with marks: 35,000 atoms, listed in time
    for format in formats {
        let t = std::time::Instant::now();
        let l = loss(&labelled(5_000, true), format).unwrap();
        assert!(l.len() >= 5_000, "{format}: {}", l.len());
        assert!(
            t.elapsed().as_secs() < bound(),
            "{format}: {:?}",
            t.elapsed()
        );
    }
    // 25,000 with no mark: nothing is written, the list is the abbreviations
    for format in formats {
        let t = std::time::Instant::now();
        let l = loss(&labelled(25_000, false), format).unwrap();
        assert_eq!(l.len(), 25_000, "{format}");
        assert!(
            t.elapsed().as_secs() < bound(),
            "{format}: {:?}",
            t.elapsed()
        );
    }
}

/// A change of scale is not a loss and is never listed, with or without a wedge mark in the drawing (the formats that rescale to a bond
/// length of 1 are not compared for shape, and the ones that are compare in bond lengths).
#[test]
fn the_scale_of_a_drawing_gives_the_same_answer_with_and_without_a_wedge() {
    // an arrow makes a reaction of it, for the reaction file
    let scaled = |wedge: u8| {
        parse(&format!(
            r#"{{"format":"mzdraw/1","next":9,"atoms":[{{"id":1,"el":"C","x":0.0,"y":0.0}},
            {{"id":2,"el":"C","x":2.0,"y":0.0}},{{"id":3,"el":"O","x":3.0,"y":1.7321}}],
            "bonds":[{{"id":4,"a":1,"b":2,"o":1}},{{"id":5,"a":2,"b":3,"o":1,"s":{wedge}}}],
            "objects":[{{"id":6,"t":"arrow","x":6.0,"y":0.0,"x2":9.0,"y2":0.0}}]}}"#
        ))
    };
    for format in [
        "molfile",
        "molfile-v3000",
        "ket",
        "cdxml",
        "cml",
        "sdf",
        "smiles",
        "rxn",
    ] {
        let plain = loss(&scaled(0), format).unwrap();
        let marked = loss(&scaled(1), format).unwrap();
        assert!(
            plain.iter().all(|l| l.what != "geometry"),
            "{format}: {plain:?}"
        );
        assert!(
            marked.iter().all(|l| l.what != "geometry"),
            "{format}: {marked:?}"
        );
        // the only difference the mark makes is the mark itself
        let kinds: Vec<&str> = marked
            .iter()
            .map(|l| l.what)
            .filter(|w| *w != "wedge.solid")
            .collect();
        assert_eq!(
            kinds,
            plain.iter().map(|l| l.what).collect::<Vec<_>>(),
            "{format}"
        );
    }
}

/// A SMILES of a drawing with a connected piece above 500 atoms is refused by every way to one (`out`, `expect`; the page's copy and its
/// menu entry call the same `Document::to_smiles`) before the recursive writer can overflow a 1 MiB stack; many small pieces are safe.
#[test]
fn every_way_to_a_smiles_refuses_a_long_piece_with_a_key_and_not_many_small_ones() {
    let run = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            // 100 pieces of 10 atoms: 1,000 atoms, each piece short
            let mut many = Document::default();
            for p in 0..100u32 {
                for k in 0..10u32 {
                    let id = p * 10 + k + 1;
                    many.atoms
                        .push(atom(id, "C", f64::from(p) * 20.0 + f64::from(k), 0.0));
                    if k > 0 {
                        many.bonds.push(Bond {
                            id: 5000 + id,
                            a: id - 1,
                            b: id,
                            o: 1,
                            s: 0,
                            more: vec![],
                        });
                    }
                }
            }
            many.next = 9000;
            let many_ok = many.to_smiles().is_ok();
            let ok = chain(500, false).to_smiles().is_ok();
            let over = chain(501, false).to_smiles().unwrap_err();
            let huge = chain(30_000, false);
            let mut e = Editor::default();
            e.session.doc = huge.clone();
            let saved = e.save("smiles").unwrap_err();
            let request = format!(
                r#"{{"import":{},"expect":[{{"smiles":"C"}}]}}"#,
                serde_json::Value::from(huge.to_json())
            );
            let verdict = mzdraw::replay::replay_at(&request, 0).unwrap();
            (many_ok, ok, over, saved, verdict.ok)
        })
        .unwrap();
    let (many_ok, ok, over, saved, verdict_ok) = run.join().unwrap();
    assert!(many_ok && ok);
    assert_eq!(
        (over, saved),
        ("error.draw2.smilesSize", "error.draw2.smilesSize")
    );
    assert!(!verdict_ok);
}

/// The SMILES writer recurses along a chain: `loss` refuses above 500 atoms before it calls it, so a 1 MiB stack (the `.wasm`'s) is enough.
#[test]
fn a_long_chain_is_refused_for_smiles_before_the_writer_can_overflow_the_stack() {
    let run = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            let ok = loss(&chain(500, true), "smiles").map(|l| l.len());
            let over = loss(&chain(501, true), "smiles").unwrap_err();
            let huge = loss(&chain(25_000, true), "smiles").unwrap_err();
            (ok.is_ok(), over, huge)
        })
        .unwrap();
    let (ok, over, huge) = run.join().unwrap();
    assert!(ok);
    assert_eq!(
        (over, huge),
        ("error.draw2.lossSize", "error.draw2.lossSize")
    );
}

/// Where the shape is compared (all distances, quadratic) the atoms are limited, and a file that reads back as too many atoms is refused.
#[test]
fn the_shape_limits_are_keyed_not_silent() {
    for format in ["sdf", "rxn"] {
        let mut d = chain(600, false);
        // a reaction file needs one arrow
        d.objects.push(Obj {
            id: 5000,
            t: ObjKind::Arrow,
            x: 700.0,
            y: 0.0,
            x2: 702.0,
            y2: 0.0,
            text: String::new(),
            runs: Vec::new(),
            from: 0,
            to: 0,
            more: vec![],
        });
        d.next = 5001;
        assert!(loss(&d, format).is_ok(), "{format}: 600 atoms");
        d.atoms.push(atom(6000, "C", 3000.0, 0.0));
        assert_eq!(
            loss(&d, format).unwrap_err(),
            "error.draw2.lossSize",
            "{format}: 601 atoms"
        );
    }
    // 600 atoms, each an abbreviation of seven atoms: the file reads back as 4,200
    let mut d = chain(600, false);
    for a in &mut d.atoms {
        a.lab = "Boc".into();
    }
    assert_eq!(loss(&d, "sdf").unwrap_err(), "error.draw2.lossSize");
    // the same drawing in `expect lossless` is false, with the key in the detail
    let json = d.to_json();
    let request = format!(
        r#"{{"import":{},"expect":[{{"lossless":"sdf"}}]}}"#,
        serde_json::Value::from(json)
    );
    let reply = mzdraw::replay::replay_at(&request, 0).unwrap();
    assert!(!reply.ok);
    assert!(
        reply.text.contains("error.draw2.lossSize"),
        "{}",
        reply.text
    );
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
