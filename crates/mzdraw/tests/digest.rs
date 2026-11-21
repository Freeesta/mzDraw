//! The digest `digest/1` (plan-23): a few hundred bytes that say where things are. Synthetic structures only.

use mzdraw::replay::replay;
use serde_json::{json, Value};
use std::path::PathBuf;

fn digest_of(doc: &Value) -> String {
    replay(&json!({"import": doc.to_string(), "out": "digest"}).to_string()).unwrap()
}

fn parsed(doc: &Value) -> Value {
    serde_json::from_str(&digest_of(doc)).unwrap()
}

/// A zigzag chain of `n` atoms (ids 1..=n, x step 0.87, y alternating 0 and 0.5), a hetero atom at every `every`-th place (O, N, S in
/// turn; none when `every` is 0), a reaction arrow and a text.
fn chain(n: usize, every: usize, extras: bool) -> Value {
    let els = ["O", "N", "S"];
    let atoms: Vec<Value> = (0..n)
        .map(|i| {
            let el = if every > 0 && i % every == every - 1 {
                els[(i / every) % 3]
            } else {
                "C"
            };
            json!({"id": i + 1, "el": el, "x": i as f64 * 0.87, "y": (i % 2) as f64 * 0.5})
        })
        .collect();
    let bonds: Vec<Value> = (1..n)
        .map(|i| json!({"id": n + i, "a": i, "b": i + 1, "o": 1}))
        .collect();
    let mut objects = Vec::new();
    if extras {
        objects.push(json!({"id": 2 * n + 1, "t": "arrow", "x": n as f64 * 0.87 + 1.0, "y": 0.0, "x2": n as f64 * 0.87 + 3.0, "y2": 0.0, "text": ""}));
        objects
            .push(json!({"id": 2 * n + 2, "t": "text", "x": 0.0, "y": -3.0, "text": "stir 2 h"}));
    }
    json!({"format": "mzdraw/1", "next": 2 * n + 3, "atoms": atoms, "bonds": bonds, "objects": objects})
}

/// The synthetic drawing of the golden: a labelled six-ring with a wedge and a wavy bond, an abbreviation, a charged ion, an arrow and
/// a text, with ids that are not in pool order.
fn fixed_document() -> Value {
    json!({"format": "mzdraw/1", "next": 40,
        "atoms": [
            {"id": 7, "el": "C", "x": 0.0, "y": 0.0}, {"id": 3, "el": "C", "x": 0.87, "y": 0.5},
            {"id": 4, "el": "C", "x": 1.73, "y": 0.0}, {"id": 5, "el": "N", "x": 1.73, "y": -1.0},
            {"id": 6, "el": "C", "x": 0.87, "y": -1.5}, {"id": 2, "el": "C", "x": 0.0, "y": -1.0},
            {"id": 8, "el": "O", "x": -0.87, "y": 0.5}, {"id": 9, "el": "C", "x": 2.6, "y": 0.5, "lab": "Ph"},
            {"id": 10, "el": "C", "x": 0.87, "y": 1.5}, {"id": 11, "el": "F", "x": 0.0, "y": -2.0}, {"id": 12, "el": "Na", "x": 6.0, "y": 0.0, "chg": 1},
            {"id": 13, "el": "Cl", "x": 8.0, "y": 0.0, "chg": -1}],
        "bonds": [
            {"id": 14, "a": 7, "b": 3, "o": 1}, {"id": 15, "a": 3, "b": 4, "o": 1}, {"id": 16, "a": 4, "b": 5, "o": 1},
            {"id": 17, "a": 5, "b": 6, "o": 1}, {"id": 18, "a": 6, "b": 2, "o": 1}, {"id": 19, "a": 2, "b": 7, "o": 1},
            {"id": 20, "a": 7, "b": 8, "o": 1}, {"id": 21, "a": 4, "b": 9, "o": 1, "s": 1},
            {"id": 22, "a": 3, "b": 10, "o": 1, "s": 3}, {"id": 23, "a": 2, "b": 11, "o": 1, "s": 2}],
        "objects": [
            {"id": 30, "t": "arrow", "x": 10.0, "y": 0.0, "x2": 12.0, "y2": 0.0},
            {"id": 31, "t": "plus", "x": 4.5, "y": 0.0},
            {"id": 32, "t": "text", "x": 1.0, "y": -4.0, "text": "stir 2 h"}]})
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/digest/golden.txt")
}

#[test]
fn the_digest_of_a_fixed_document_is_the_recorded_bytes_and_the_same_twice() {
    let got = digest_of(&fixed_document());
    assert_eq!(got, digest_of(&fixed_document()));
    if std::env::var("MZDRAW_BLESS").is_ok() {
        std::fs::create_dir_all(golden_path().parent().unwrap()).unwrap();
        std::fs::write(golden_path(), format!("{got}\n")).unwrap();
    }
    let want = std::fs::read_to_string(golden_path()).unwrap();
    assert_eq!(want.trim_end(), got);
    let v: Value = serde_json::from_str(&got).unwrap();
    assert_eq!(v["format"], "digest/1");
    // the keys are in the fixed order
    let order = |text: &str, keys: &[&str]| -> bool {
        let at: Vec<usize> = keys
            .iter()
            .map(|k| text.find(&format!("\"{k}\":")).unwrap())
            .collect();
        at.windows(2).all(|w| w[0] < w[1])
    };
    assert!(
        order(&got, &["format", "pieces", "objects", "clearance", "std"]),
        "{got}"
    );
    assert!(
        order(
            &got,
            &["ids", "formula", "box", "labels", "stereo", "rings"]
        ),
        "{got}"
    );
}

#[test]
fn the_digest_says_the_structures_left_to_right_with_what_is_not_plain_carbon_and_the_marks() {
    let v = parsed(&fixed_document());
    let pieces = v["pieces"].as_array().unwrap();
    assert_eq!(pieces.len(), 3);
    assert_eq!(pieces[0]["ids"], "2-11");
    assert_eq!(pieces[0]["rings"], 1);
    // N, O and the abbreviation, by atom id as written in the pool order
    let labels = &pieces[0]["labels"];
    assert_eq!(labels, &json!([[5, "N"], [8, "O"], [9, "Ph"], [11, "F"]]));
    assert_eq!(
        pieces[0]["stereo"],
        json!([[21, "wedge", 4], [22, "wavy", 3], [23, "hash", 2]])
    );
    assert_eq!(pieces[1]["formula"], "Na+");
    assert_eq!(pieces[2]["formula"], "Cl-");
    assert_eq!(
        v["objects"],
        json!([
            [30, "arrow", [10.0, 0.0, 12.0, 0.0]],
            [31, "plus", v["objects"][1][2].clone()],
            [32, "text", v["objects"][2][2].clone()]
        ])
    );
    // the smallest gap between two boxes is the clearance: here the one between the two ions
    let (b0, b1) = (&pieces[1]["box"], &pieces[2]["box"]);
    let gap = b1[0].as_f64().unwrap() - b0[2].as_f64().unwrap();
    assert!((v["clearance"].as_f64().unwrap() - gap).abs() < 0.11, "{v}");
}

#[test]
fn the_digest_of_an_empty_drawing_and_a_lone_structure() {
    let empty = replay(r#"{"out":"digest"}"#).unwrap();
    assert_eq!(
        empty,
        r#"{"format":"digest/1","pieces":[],"objects":[],"clearance":null,"std":{}}"#
    );
    let one = parsed(&chain(3, 0, false));
    assert_eq!(one["clearance"], Value::Null);
}

#[test]
fn the_formula_of_each_structure_is_the_one_of_the_outline_line() {
    let doc = fixed_document();
    let v = parsed(&doc);
    let mut e = mzdraw::Editor::default();
    e.import(&doc.to_string()).unwrap();
    let mut outline: Vec<String> = mzdraw::outline::pieces(e.doc())
        .into_iter()
        .map(|p| p.formula.replace('\u{2212}', "-"))
        .collect();
    let mut digest: Vec<String> = v["pieces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["formula"].as_str().unwrap().to_string())
        .collect();
    outline.sort();
    digest.sort();
    assert_eq!(outline, digest);
}

#[test]
fn thirty_atoms_with_an_arrow_and_a_text_fit_in_twelve_hundred_bytes() {
    // a ring of six, a chain of 24 with heteroatoms, an abbreviation and two stereo bonds: the worst honest 30-atom drawing
    let mut doc = chain(30, 4, true);
    doc["next"] = json!(80);
    doc["atoms"][5]["lab"] = json!("Ph");
    doc["bonds"][3]["s"] = json!(1);
    doc["bonds"][9]["s"] = json!(2);
    doc["bonds"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": 70, "a": 1, "b": 6, "o": 1}));
    let text = digest_of(&doc);
    assert!(text.len() <= 1200, "{} bytes: {text}", text.len());
    eprintln!("30 atoms, one arrow, one text: {} bytes", text.len());
}

#[test]
fn two_hundred_atoms_still_list_every_hetero_atom() {
    let doc = chain(200, 3, true);
    let v = parsed(&doc);
    let hetero: Vec<u64> = doc["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["el"] != "C")
        .map(|a| a["id"].as_u64().unwrap())
        .collect();
    assert!(hetero.len() > 60);
    let listed: Vec<u64> = v["pieces"][0]["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l[0].as_u64().unwrap())
        .collect();
    assert_eq!(listed, hetero);
}

/// The atom ids a `ranges` text names.
fn expand(ids: &str) -> Vec<u64> {
    let mut out = Vec::new();
    for part in ids.split(',').filter(|p| !p.is_empty()) {
        match part.split_once('-') {
            Some((a, b)) => out.extend(a.parse::<u64>().unwrap()..=b.parse::<u64>().unwrap()),
            None => out.push(part.parse().unwrap()),
        }
    }
    out
}

/// A random drawing from a fixed seed: atoms of a few elements with scattered ids, bonds between random pairs, some marks, and objects.
fn random_document(seed: u64) -> Value {
    let mut s = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let mut next = move |m: u64| {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 33) % m
    };
    let n = 2 + next(40) as usize;
    let els = ["C", "C", "C", "N", "O", "S", "Cl"];
    let mut id = 1;
    let mut atoms = Vec::new();
    for _ in 0..n {
        id += 1 + next(3);
        let mut a = json!({"id": id, "el": els[next(7) as usize], "x": next(200) as f64 / 20.0, "y": next(100) as f64 / 20.0});
        if next(9) == 0 {
            a["lab"] = json!("Me");
        }
        atoms.push(a);
    }
    let mut bonds = Vec::new();
    for _ in 0..next(n as u64 + 5) {
        let (a, b) = (next(n as u64) as usize, next(n as u64) as usize);
        if a == b {
            continue;
        }
        let (ia, ib) = (
            atoms[a]["id"].as_u64().unwrap(),
            atoms[b]["id"].as_u64().unwrap(),
        );
        if bonds
            .iter()
            .any(|x: &Value| (x["a"] == ia && x["b"] == ib) || (x["a"] == ib && x["b"] == ia))
        {
            continue;
        }
        id += 1;
        let mut bond = json!({"id": id, "a": ia, "b": ib, "o": 1});
        if next(4) == 0 {
            bond["s"] = json!(1 + next(3));
        }
        bonds.push(bond);
    }
    let mut objects = Vec::new();
    for k in 0..next(4) {
        id += 1;
        objects.push(if k % 2 == 0 {
            json!({"id": id, "t": "arrow", "x": 0.0, "y": 6.0 + k as f64, "x2": 3.0, "y2": 6.0 + k as f64})
        } else {
            json!({"id": id, "t": "text", "x": 1.0, "y": 8.0, "text": "t"})
        });
    }
    json!({"format": "mzdraw/1", "next": id + 1, "atoms": atoms, "bonds": bonds, "objects": objects})
}

#[test]
fn every_id_of_the_digest_exists_in_the_document_and_the_ranges_cover_every_atom_once() {
    for seed in 0..300 {
        let doc = random_document(seed);
        let v = parsed(&doc);
        let ids = |key: &str| -> Vec<u64> {
            doc[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a["id"].as_u64().unwrap())
                .collect()
        };
        let (atoms, bonds): (Vec<u64>, Vec<u64>) = (ids("atoms"), ids("bonds"));
        let mut seen = Vec::new();
        for p in v["pieces"].as_array().unwrap() {
            let own = expand(p["ids"].as_str().unwrap());
            assert!(own.iter().all(|i| atoms.contains(i)), "seed {seed}: {p}");
            seen.extend(own.iter().copied());
            for l in p["labels"].as_array().unwrap() {
                assert!(own.contains(&l[0].as_u64().unwrap()), "seed {seed}: {l}");
            }
            for m in p["stereo"].as_array().unwrap() {
                assert!(bonds.contains(&m[0].as_u64().unwrap()), "seed {seed}: {m}");
                assert!(own.contains(&m[2].as_u64().unwrap()), "seed {seed}: {m}");
            }
        }
        seen.sort_unstable();
        let mut all = atoms.clone();
        all.sort_unstable();
        assert_eq!(
            seen, all,
            "seed {seed}: every atom is in exactly one structure"
        );
        let objects = ids("objects");
        for o in v["objects"].as_array().unwrap() {
            assert!(
                objects.contains(&o[0].as_u64().unwrap()),
                "seed {seed}: {o}"
            );
        }
    }
}

/// The numbers of the `x1=`, `y1=`, `x2=`, `y2=` attributes of the `<line>` elements of an SVG.
fn svg_lines(svg: &str) -> Vec<[f64; 4]> {
    let attr = |tag: &str, name: &str| -> f64 {
        let tag = format!(" {tag}");
        let at = tag.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
        tag[at..].split('"').next().unwrap().parse().unwrap()
    };
    svg.split("<line ")
        .skip(1)
        .map(|t| [attr(t, "x1"), attr(t, "y1"), attr(t, "x2"), attr(t, "y2")])
        .collect()
}

/// The box is not computed from the SVG's code: the lines of the SVG of a structure with no label are read as the SVG states them
/// (points, `viewBox`), and their extent, in bond lengths, is the width and the height of the box of the digest.
#[test]
fn the_box_of_a_structure_is_the_extent_of_the_lines_of_its_svg() {
    for doc in [chain(9, 0, false), random_carbon_ring()] {
        let v = parsed(&doc);
        let b = &v["pieces"][0]["box"];
        let svg = replay(&json!({"import": doc.to_string(), "out": "svg"}).to_string()).unwrap();
        let lines = svg_lines(&svg);
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for l in &lines {
            x0 = x0.min(l[0]).min(l[2]);
            x1 = x1.max(l[0]).max(l[2]);
            y0 = y0.min(l[1]).min(l[3]);
            y1 = y1.max(l[1]).max(l[3]);
        }
        let bond = mzdraw::Style::default().bond_length;
        let (w, h) = ((x1 - x0) / bond, (y1 - y0) / bond);
        let (bw, bh) = (
            b[2].as_f64().unwrap() - b[0].as_f64().unwrap(),
            b[3].as_f64().unwrap() - b[1].as_f64().unwrap(),
        );
        // the digest keeps one decimal: each edge is off by at most 0.05, the SVG keeps two decimals of a point
        assert!(
            (w - bw).abs() <= 0.101 && (h - bh).abs() <= 0.101,
            "svg {w} x {h}, digest {bw} x {bh}: {b}"
        );
    }
}

fn random_carbon_ring() -> Value {
    let pts: Vec<(f64, f64)> = (0..6)
        .map(|i| {
            let t = i as f64 * std::f64::consts::PI / 3.0;
            (2.0 + t.cos(), 3.0 + t.sin())
        })
        .collect();
    let atoms: Vec<Value> = pts
        .iter()
        .enumerate()
        .map(|(i, p)| json!({"id": i + 1, "el": "C", "x": p.0, "y": p.1}))
        .collect();
    let bonds: Vec<Value> = (0..6)
        .map(|i| json!({"id": 7 + i, "a": i + 1, "b": (i + 1) % 6 + 1, "o": 1}))
        .collect();
    json!({"format": "mzdraw/1", "next": 20, "atoms": atoms, "bonds": bonds})
}

#[test]
fn the_ink_of_a_label_widens_the_box_by_less_than_a_bond_length() {
    let mut doc = chain(4, 0, false);
    let plain = parsed(&doc)["pieces"][0]["box"].clone();
    doc["atoms"][3]["el"] = json!("Cl");
    let inked = parsed(&doc)["pieces"][0]["box"].clone();
    let grow = inked[2].as_f64().unwrap() - plain[2].as_f64().unwrap();
    assert!(grow > 0.05 && grow < 1.0, "{plain} {inked}");
}

#[test]
fn the_findings_of_std_are_counted_by_key() {
    // an arrow with only one structure anchored is a std/1 finding of the pathway; the digest counts it under its key
    let mut doc = chain(3, 0, false);
    doc["next"] = json!(30);
    doc["objects"] = json!([{"id": 20, "t": "arrow", "x": 5.0, "y": 0.0, "x2": 7.0, "y2": 0.0, "from": 1, "to": 0}]);
    let v = parsed(&doc);
    let std = replay(&json!({"import": doc.to_string(), "out": "std"}).to_string()).unwrap();
    let want: Value = serde_json::from_str(&std).unwrap();
    let mut counts = serde_json::Map::new();
    for c in want["checks"].as_array().unwrap() {
        let n = counts
            .get(c["key"].as_str().unwrap())
            .map_or(0, |x: &Value| x.as_u64().unwrap());
        counts.insert(c["key"].as_str().unwrap().to_string(), json!(n + 1));
    }
    assert_eq!(v["std"], Value::Object(counts), "{v}");
}

#[test]
fn a_hostile_drawing_is_listed_up_to_the_caps_and_counted_beyond() {
    // 1000 structures of one atom and 1000 texts: 200 and 200 listed, the rest counted
    let atoms: Vec<Value> = (0..1000).map(|i| json!({"id": i + 1, "el": "N", "x": (i % 50) as f64 * 3.0, "y": (i / 50) as f64 * 3.0})).collect();
    let objects: Vec<Value> = (0..1000)
        .map(|i| json!({"id": 1001 + i, "t": "text", "x": 0.0, "y": 0.0, "text": "a"}))
        .collect();
    let doc = json!({"format": "mzdraw/1", "next": 3000, "atoms": atoms, "bonds": [], "objects": objects});
    let v = parsed(&doc);
    assert_eq!(v["pieces"].as_array().unwrap().len(), 200);
    assert_eq!(v["objects"].as_array().unwrap().len(), 200);
    assert_eq!(v["more"], json!([800, 800]));
    assert!(digest_of(&doc).len() < 40_000);
}

/// `n` atoms, no bond (so the engine calls the digest makes cost nothing per atom beyond their own size): every atom is a structure.
fn bondless(n: usize) -> String {
    let atoms: Vec<Value> = (0..n)
        .map(|i| json!({"id": i + 1, "el": if i % 3 == 0 { "N" } else { "C" }, "x": (i % 200) as f64 * 3.0, "y": (i / 200) as f64 * 3.0}))
        .collect();
    json!({"format": "mzdraw/1", "next": n + 2, "atoms": atoms, "bonds": []}).to_string()
}

/// A zigzag chain of `n` atoms with a nitrogen at every third place.
fn long_chain(n: usize) -> String {
    let atoms: Vec<Value> = (0..n)
        .map(|i| json!({"id": i + 1, "el": if i % 3 == 0 { "N" } else { "C" }, "x": i as f64 * 0.87, "y": (i % 2) as f64 * 0.5}))
        .collect();
    let bonds: Vec<Value> = (1..n)
        .map(|i| json!({"id": n + i, "a": i, "b": i + 1, "o": 1}))
        .collect();
    json!({"format": "mzdraw/1", "next": 2 * n + 2, "atoms": atoms, "bonds": bonds}).to_string()
}

/// The time the digest takes on a document text (the import is not counted: it is not this slice's).
fn digest_time(text: &str) -> (std::time::Duration, String) {
    let mut e = mzdraw::Editor::default();
    e.import(text).unwrap();
    let t0 = std::time::Instant::now();
    let out = e.save("digest").unwrap();
    (t0.elapsed(), out)
}

/// The cost grows with the size of the drawing, not with its square: doubling the number of structures at most triples the time (a
/// quadratic version, such as one that looks every atom up in a list of the atoms seen, quadruples it), and 40,000 structures stay
/// within a bound. Best of three runs against noise.
#[test]
fn the_time_grows_with_the_size_not_with_its_square() {
    let best = |text: &str| (0..3).map(|_| digest_time(text).0).min().unwrap();
    let (small, big) = (best(&bondless(20_000)), best(&bondless(40_000)));
    eprintln!("20,000 structures {small:?}, 40,000 structures {big:?}");
    assert!(
        big.as_secs_f64() < 3.0 * small.as_secs_f64() + 0.02,
        "20,000 structures {small:?}, 40,000 structures {big:?}"
    );
    let bound = std::time::Duration::from_secs(if cfg!(debug_assertions) { 5 } else { 1 });
    assert!(big < bound, "40,000 structures took {big:?}");
    // only 200 are listed, the rest counted, and `std` is not counted above `MAX_STD_ATOMS` atoms
    let (_, out) = digest_time(&bondless(40_000));
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        (v["pieces"].as_array().unwrap().len(), &v["more"], &v["std"]),
        (200, &json!([39_800, 0]), &Value::Null)
    );
    assert!(out.len() < 60_000, "{} bytes", out.len());
}

/// A long chain, bonds and all: the engine calls under the digest (the record, the label ink) rescan the bonds of an atom, so this
/// is a bound, not a proof of linearity. The `std` findings are counted up to `MAX_STD_ATOMS` atoms and are `null` above.
#[test]
fn a_long_chain_stays_within_a_bound_and_std_is_not_counted_above_its_cap() {
    let bound = std::time::Duration::from_secs(if cfg!(debug_assertions) { 20 } else { 3 });
    let (took, out) = digest_time(&long_chain(10_000));
    eprintln!("10,000-atom chain: {took:?}");
    assert!(took < bound, "10,000-atom chain took {took:?}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["std"], Value::Null);
    assert_eq!(v["pieces"][0]["rings"], 0);
    assert_eq!(v["pieces"][0]["ids"], "1-10000");
    let (took, out) = digest_time(&long_chain(mzdraw::digest::MAX_STD_ATOMS));
    assert!(took < bound / 2, "{took:?}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert!(v["std"].is_object(), "{v}");
}

#[test]
fn the_page_and_the_replay_route_give_the_same_bytes_for_an_editor_session() {
    // a session of the editor (the page) and the log of it replayed (the CLI route) end in the same digest
    let mut e = mzdraw::Editor::default();
    e.import(&fixed_document().to_string()).unwrap();
    let log = e.save("log").unwrap();
    let page = e.save("digest").unwrap();
    let via = replay(
        &json!({"log": serde_json::from_str::<Value>(&log).unwrap(), "out": "digest"}).to_string(),
    )
    .unwrap();
    assert_eq!(page, via);
}
