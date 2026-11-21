//! The diff lens (tp-04): what changed from the parent, computed from the atom maps; the `lens/1` output, the tint and the ghost,
//! the finding of a pair that cannot be compared, the formula delta, the cost and the caps. Synthetic structures only.

use mzdraw::{Document, Editor};
use serde_json::{json, Value};

/// N-methyl caprolactam as a hexagon, atoms 1..8, bonds 9..16 (ring bonds 9..14), next 17.
fn lactam() -> Value {
    let p = |k: f64| {
        (
            (k * 60f64.to_radians()).cos(),
            (k * 60f64.to_radians()).sin(),
        )
    };
    let mut atoms = Vec::new();
    for (i, el) in ["N", "C", "C", "C", "C", "C"].iter().enumerate() {
        let q = p(i as f64);
        atoms.push(json!({"id": i + 1, "el": el, "x": q.0, "y": q.1}));
    }
    atoms.push(json!({"id":7,"el":"C","x":2.0,"y":0.0}));
    atoms.push(json!({"id":8,"el":"O","x":0.5,"y":1.7320508}));
    let mut bonds = Vec::new();
    for i in 0..6 {
        bonds.push(json!({"id": 9 + i, "a": i + 1, "b": (i + 1) % 6 + 1, "o": 1}));
    }
    bonds.push(json!({"id":15,"a":1,"b":7,"o":1}));
    bonds.push(json!({"id":16,"a":2,"b":8,"o":2}));
    json!({"format":"mzdraw/1","next":17,"atoms":atoms,"bonds":bonds})
}

fn replay(request: Value) -> Value {
    serde_json::from_str(&mzdraw::replay::replay(&request.to_string()).unwrap()).unwrap()
}

fn derive(patch: Value) -> Value {
    json!({"command":"tp:derive","args":{"of":1,"patch":patch}})
}

/// The `lens/1` document of the lactam after a derive with `patch`.
fn lens_after(patch: Value) -> Value {
    let r = replay(
        json!({"import": lactam().to_string(), "commands": [derive(patch)], "report": true, "out": "lens"}),
    );
    assert_eq!(r["ok"], true, "{r}");
    serde_json::from_str(r["out"].as_str().unwrap()).unwrap()
}

fn doc_after(patch: Value) -> Document {
    let r = replay(
        json!({"import": lactam().to_string(), "commands": [derive(patch)], "report": true, "out": "mzdraw"}),
    );
    assert_eq!(r["ok"], true, "{r}");
    Document::from_json(r["out"].as_str().unwrap()).unwrap()
}

fn ids(v: &Value) -> Vec<u64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap())
        .collect()
}

#[test]
fn a_copy_has_an_empty_lens() {
    let l = lens_after(json!([]));
    let a = &l["arrows"].as_array().unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(
        a[0],
        json!({"arrow": a[0]["arrow"], "added": [], "added_bonds": [], "changed": [], "changed_bonds": [],
               "cleaved": [], "lost": [], "delta": ""})
    );
}

#[test]
fn the_keys_come_in_a_fixed_order_and_the_format_is_named() {
    let r = replay(
        json!({"import": lactam().to_string(), "commands": [derive(json!([]))], "report": true, "out": "lens"}),
    );
    let text = r["out"].as_str().unwrap();
    assert!(
        text.starts_with(r#"{"format":"lens/1","arrows":[{"arrow":"#),
        "{text}"
    );
    assert!(text.contains(
        r#""added":[],"added_bonds":[],"changed":[],"changed_bonds":[],"cleaved":[],"lost":[],"delta":""}],"skipped":[]}"#
    ));
}

#[test]
fn a_hydroxylation_adds_an_oxygen_and_a_bond_and_changes_the_carbon() {
    // O on atom 5 (a CH2): the new atom takes the next free map, 9
    let patch = json!([{"op":"atom.add","el":"O","x":-1.0,"y":-1.7321},{"op":"bond.add","a":5,"b":-1,"o":1}]);
    let l = lens_after(patch.clone());
    let d = doc_after(patch);
    let at = |map: u16| (d.atoms.iter().rev()).find(|a| a.map == map).unwrap().id as u64;
    let a = &l["arrows"][0];
    assert_eq!(ids(&a["added"]), vec![at(9)]);
    assert_eq!(a["added_bonds"].as_array().unwrap().len(), 1);
    assert_eq!(
        ids(&a["changed"]),
        vec![at(5)],
        "the carbon lost a hydrogen"
    );
    assert_eq!(a["cleaved"], json!([]));
    assert_eq!(a["lost"], json!([]));
    assert_eq!(a["delta"], "+O");
}

#[test]
fn k_added_atoms_are_k_added_atoms() {
    for k in 1..=6u32 {
        // a chain of k carbons on atom 4
        let mut patch = vec![
            json!({"op":"atom.add","el":"C","x":-1.0,"y":-1.0}),
            json!({"op":"bond.add","a":4,"b":-1,"o":1}),
        ];
        for j in 2..=k {
            patch.push(json!({"op":"atom.add","el":"C","x":-1.0,"y":-1.0 - j as f64}));
            patch.push(json!({"op":"bond.add","a":-(j as i64 - 1),"b":-(j as i64),"o":1}));
        }
        let l = lens_after(json!(patch));
        let a = &l["arrows"][0];
        assert_eq!(a["added"].as_array().unwrap().len(), k as usize, "k = {k}");
        assert_eq!(
            a["added_bonds"].as_array().unwrap().len(),
            k as usize,
            "k = {k}"
        );
        // k carbons and 2k hydrogens more (the atom they hang on gives up one hydrogen, the last carbon is a methyl)
        let want = if k == 1 {
            "+CH2".to_string()
        } else {
            format!("+C{k}H{}", 2 * k)
        };
        assert_eq!(a["delta"], want, "k = {k}");
    }
}

#[test]
fn a_ring_opening_cleaves_exactly_the_removed_bond() {
    // bond 10 joins atoms 2 and 3
    let patch = json!([{"op":"remove","bonds":[10]}]);
    let l = lens_after(patch.clone());
    let d = doc_after(patch);
    let at = |map: u16| (d.atoms.iter().rev()).find(|a| a.map == map).unwrap().id as u64;
    let a = &l["arrows"][0];
    let (x, y) = (at(2).min(at(3)), at(2).max(at(3)));
    assert_eq!(a["cleaved"], json!([[x, y]]));
    assert_eq!(a["added"], json!([]));
    assert_eq!(a["lost"], json!([]));
    // each end gains a hydrogen
    assert_eq!(ids(&a["changed"]), {
        let mut v = vec![at(2), at(3)];
        v.sort();
        v
    });
    assert_eq!(a["delta"], "+H2");
}

#[test]
fn a_changed_bond_order_a_lost_atom_and_a_formed_bond_are_each_listed() {
    // C=O reduced to C-O (changed bond, changed oxygen hydrogen count), the methyl removed (lost map 7), a ring closure is not tried here
    let patch = json!([{"op":"bond.set","id":16,"o":1},{"op":"remove","atoms":[7]}]);
    let l = lens_after(patch.clone());
    let d = doc_after(patch);
    let a = &l["arrows"][0];
    assert_eq!(a["lost"], json!([7]));
    assert_eq!(a["changed_bonds"].as_array().unwrap().len(), 1);
    assert!(a["changed"].as_array().unwrap().len() >= 2, "{a}");
    assert_eq!(a["added"], json!([]));
    // the bond kept its place: it is the copy of bond 16, between maps 2 and 8
    let b = d
        .bonds
        .iter()
        .find(|b| b.id as u64 == a["changed_bonds"][0].as_u64().unwrap())
        .unwrap();
    let maps = [d.atom(b.a).unwrap().map, d.atom(b.b).unwrap().map];
    assert!(maps.contains(&2) && maps.contains(&8), "{maps:?}");
    // a bond formed between two atoms both structures have (a ring closure) is an added bond
    let patch = json!([{"op":"bond.add","a":3,"b":6,"o":1}]);
    let l = lens_after(patch);
    let a = &l["arrows"][0];
    assert_eq!(a["added"], json!([]));
    assert_eq!(a["added_bonds"].as_array().unwrap().len(), 1);
}

/// A tiny xorshift: repeatable pseudo-random choices, no dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

#[test]
fn the_formula_of_the_parent_plus_the_delta_is_the_formula_of_the_product() {
    let mut accepted = 0;
    for seed in 1..=50u64 {
        let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut patch: Vec<Value> = Vec::new();
        let mut added = 0i64;
        for _ in 0..1 + r.next(4) {
            match r.next(4) {
                0 | 1 => {
                    added += 1;
                    let el = ["C", "N", "O", "S"][r.next(4)];
                    patch.push(json!({"op":"atom.add","el":el,"x":-3.0,"y":-3.0 - added as f64}));
                    patch.push(json!({"op":"bond.add","a":1 + r.next(6),"b":-added,"o":1}));
                }
                2 => patch.push(json!({"op":"remove","bonds":[9 + r.next(8)]})),
                _ => patch.push({
                    let els = ["C", "N", "O", "S"];
                    json!({"op":"atom.set","id":1 + r.next(8),"el":els[r.next(4)]})
                }),
            }
        }
        let req = json!({"import": lactam().to_string(), "commands": [derive(json!(patch))], "report": true, "out": "mzdraw"});
        let out: Value =
            serde_json::from_str(&mzdraw::replay::replay(&req.to_string()).unwrap()).unwrap();
        if out["ok"] != true {
            continue; // a valence the patch makes is refused, nothing is written
        }
        let d = Document::from_json(out["out"].as_str().unwrap()).unwrap();
        let lens = mzdraw::tp::lenses(&d).unwrap();
        assert_eq!(lens.lenses.len(), 1, "seed {seed}");
        let pieces = mzdraw::outline::pieces(&d);
        // the product is the structure the lens names by its anchor (a patch may split it)
        let anchor = lens.lenses[0].anchor;
        let t = &pieces.iter().find(|x| x.anchor == anchor).unwrap().formula;
        let p = &pieces[0].formula;
        let sum = mzlab_mol::delta::add(&format!("+{p}"), lens.lenses[0].delta.as_deref().unwrap())
            .unwrap();
        assert_eq!(
            sum,
            format!("+{t}"),
            "seed {seed}: {p} {} {t}",
            lens.lenses[0].delta.as_deref().unwrap()
        );
        assert_eq!(lens.lenses[0].from, *p, "seed {seed}");
        accepted += 1;
    }
    assert!(accepted >= 25, "only {accepted} of 50 patches were valid");
}

/// The text of an export of the lactam and its hydroxylated copy, with the lens in the mode asked.
fn export(mode: &str, kind: &str) -> String {
    let patch = json!([{"op":"atom.add","el":"O","x":-1.0,"y":-1.7321},{"op":"bond.add","a":5,"b":-1,"o":1},{"op":"remove","bonds":[10]}]);
    let r = replay(json!({
        "import": lactam().to_string(), "commands": [derive(patch)], "set": [format!("display.lens={mode}")],
        "report": true, "out": kind
    }));
    assert_eq!(r["ok"], true, "{r}");
    r["out"].as_str().unwrap().to_string()
}

#[test]
fn the_lens_is_in_no_file_format_and_in_the_picture_only_when_it_is_on() {
    for kind in [
        "molfile", "smiles", "ket", "cdxml", "cml", "rxn", "mzdraw", "sdf",
    ] {
        let off = export("off", kind);
        for mode in ["tint", "tint+ghost"] {
            assert_eq!(off, export(mode, kind), "{kind} with {mode}");
        }
    }
    let (off, tint, ghost) = (
        export("off", "svg"),
        export("tint", "svg"),
        export("tint+ghost", "svg"),
    );
    assert!(
        !off.contains("#2d5d8c") && !off.contains("#5c5850"),
        "off: plain"
    );
    assert!(
        tint.contains("#2d5d8c") && !tint.contains("#5c5850"),
        "tint: accent, no ghost"
    );
    assert!(
        ghost.contains("#2d5d8c") && ghost.contains("#5c5850"),
        "ghost: both tokens"
    );
    assert_ne!(off, tint);
    assert_ne!(tint, ghost);
    // deterministic bytes
    assert_eq!(ghost, export("tint+ghost", "svg"));
}

#[test]
fn the_setting_is_validated_and_written_only_when_on() {
    let mut e = Editor::default();
    assert!(e.set("display.lens=sparkle").is_err());
    assert!(e.set("display.lens=").is_err());
    assert!(!e.style.to_json().contains("lens"), "off is not written");
    e.set("display.lens=tint+ghost").unwrap();
    let file = e.style.to_json();
    assert!(file.contains(r#""lens":"tint+ghost""#), "{file}");
    let back = mzdraw::settings::Style::from_text(&file).unwrap();
    assert_eq!(back.lens.name(), "tint+ghost");
    // an unknown word in a file is an unreadable file
    assert!(mzdraw::settings::Style::from_text(&file.replace("tint+ghost", "x")).is_err());
    assert_eq!(e.style.choice("display.lens"), Some("tint+ghost"));
}

#[test]
fn shift_l_cycles_the_lens_from_the_keyboard() {
    use mzdraw::editor::{CHANGED, REPAINT, SHIFT};
    let mut e = Editor::default();
    e.import(&lactam().to_string()).unwrap();
    let doc = e.doc().to_json();
    let mut seen = Vec::new();
    for _ in 0..3 {
        let f = e.key("L", SHIFT);
        assert_ne!(f & REPAINT, 0);
        assert_eq!(f & CHANGED, 0, "a display setting is no document change");
        seen.push(e.style.lens.name());
    }
    assert_eq!(seen, ["tint", "tint+ghost", "off"]);
    assert_eq!(e.doc().to_json(), doc);
    assert_eq!(e.said.0, "draw2.say.lens");
    // the plain key still asks for a label
    assert_ne!(e.key("l", 0) & mzdraw::editor::ASK_LABEL, 0);
}

/// Two structures joined by an anchored arrow: P has atoms with `p_maps`, T has atoms with `t_maps` (chains of carbons).
fn pair(p_maps: &[u16], t_maps: &[u16]) -> Document {
    let mut atoms = Vec::new();
    let mut bonds = Vec::new();
    let mut id = 0;
    let chain =
        |maps: &[u16], x0: f64, atoms: &mut Vec<Value>, bonds: &mut Vec<Value>, id: &mut u32| {
            let first = *id + 1;
            for (i, m) in maps.iter().enumerate() {
                *id += 1;
                atoms.push(json!({"id": *id, "el": "C", "x": x0 + i as f64, "y": 0.0, "map": m}));
                if i > 0 {
                    bonds.push(json!({"id": 100 + *id, "a": *id - 1, "b": *id, "o": 1}));
                }
            }
            first
        };
    let a = chain(p_maps, 0.0, &mut atoms, &mut bonds, &mut id);
    let b = chain(t_maps, 10.0, &mut atoms, &mut bonds, &mut id);
    let doc = json!({"format":"mzdraw/1","next":300,"atoms":atoms,"bonds":bonds,
        "objects":[{"id":200,"t":"arrow","x":4.0,"y":0.0,"x2":9.0,"y2":0.0,"from":a,"to":b}]});
    Document::from_json(&doc.to_string()).unwrap()
}

#[test]
fn a_pair_that_cannot_be_compared_is_a_finding_and_never_a_partial_lens() {
    // an atom without a map, a map shared by two atoms of one structure
    for (p, t) in [
        (vec![1, 2, 3], vec![1, 2, 0]),
        (vec![1, 2, 3], vec![1, 2, 2]),
        (vec![1, 0, 3], vec![1, 2, 3]),
    ] {
        let d = pair(&p, &t);
        let l = mzdraw::tp::lenses(&d).unwrap();
        assert!(l.lenses.is_empty(), "{p:?} {t:?}");
        assert_eq!(l.unmapped.len(), 1);
        assert_eq!(l.unmapped[0].arrow, 200);
        assert!(!l.unmapped[0].atoms.is_empty());
        assert!(matches!(
            mzdraw::tp::lens(&d, 200),
            Err(mzdraw::tp::NoLens::Unmapped(_))
        ));
        let mut e = Editor::default();
        e.open(&d.to_json()).unwrap();
        let std = e.save("std").unwrap();
        assert!(std.contains("std.tp.unmapped"), "{std}");
        assert!(e
            .save("lens")
            .unwrap()
            .ends_with(r#""arrows":[],"skipped":[{"arrow":200,"why":"unmapped"}]}"#));
    }
    // a pair with no map at all has nothing to compare, and nothing is wrong with it
    let d = pair(&[0, 0], &[0, 0, 0]);
    let mut e = Editor::default();
    e.open(&d.to_json()).unwrap();
    assert!(!e.save("std").unwrap().contains("std.tp.unmapped"));
    assert!(e
        .save("lens")
        .unwrap()
        .ends_with(r#""arrows":[],"skipped":[{"arrow":200,"why":"no maps"}]}"#));
    // a fully mapped pair is compared: the third atom of T is added, the third of P is lost
    let d = pair(&[1, 2, 3], &[1, 2, 4]);
    let l = mzdraw::tp::lens(&d, 200).unwrap();
    assert_eq!((l.added.len(), l.lost.clone()), (1, vec![3]));
    assert_eq!(l.added_bonds.len(), 1);
    // a free arrow, or one inside a structure, has no lens
    assert!(matches!(
        mzdraw::tp::lens(&d, 999),
        Err(mzdraw::tp::NoLens::NotAnchored)
    ));
}

/// A mapped chain of `n` atoms (every atom the same map is not allowed, so the maps wrap at 999: such a structure is only for the caps)
/// and one more atom, joined by an anchored arrow.
fn big(n: u32) -> String {
    let atoms: Vec<Value> = (1..=n)
        .map(|i| json!({"id": i, "el": "C", "x": i as f64, "y": 0.0, "map": 1}))
        .chain([json!({"id": 2 * n, "el": "C", "x": 0.0, "y": 9.0, "map": 1})])
        .collect();
    let bonds: Vec<Value> = (1..n)
        .map(|i| json!({"id": n + i, "a": i, "b": i + 1, "o": 1}))
        .collect();
    json!({"format":"mzdraw/1","next":4 * n,"atoms":atoms,"bonds":bonds,
        "objects":[{"id":3 * n,"t":"arrow","x":0.0,"y":4.0,"x2":0.0,"y2":8.0,"from":1,"to":2 * n}]})
    .to_string()
}

#[test]
fn a_structure_over_the_cap_is_skipped_with_its_reason_and_a_drawing_over_the_cap_is_a_keyed_error()
{
    // a structure above 5000 atoms: the arrow is listed as skipped, over cap, not dropped in silence
    let doc = big(5001);
    let r = replay(json!({"import": doc, "report": true, "out": "lens"}));
    let l: Value = serde_json::from_str(r["out"].as_str().unwrap()).unwrap();
    assert_eq!(l["arrows"], json!([]));
    assert_eq!(l["skipped"], json!([{"arrow": 15003, "why": "over cap"}]));
    // the page is told, in a keyed note, that the lens is not showing it (the painter has older quadratic passes of its own, so the
    // picture of a 5000-atom structure is not timed here)
    let ed = Document::from_json(&doc).unwrap();
    let st = mzdraw::tp::LensState::of(&ed);
    assert_eq!(st.note, Some(("draw2.notice.lensSkipped", 1)));
    // a drawing above 6000 atoms is the keyed error of the output, before any work, and the page is told
    let doc = big(6_001);
    let ed = Document::from_json(&doc).unwrap();
    assert_eq!(mzdraw::tp::lenses(&ed), Err("error.draw2.lensSize"));
    assert_eq!(
        mzdraw::tp::LensState::of(&ed).note.map(|n| n.0),
        Some("draw2.notice.lensSize")
    );
    // and the web text of each key is in the page's language file
    let lang = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../web/lang/mzdraw.js"
    ))
    .unwrap();
    for key in [
        "error.draw2.lensSize",
        "draw2.notice.lensSize",
        "draw2.notice.lensSkipped",
        "draw2.std.tp.unmapped",
        "draw2.outline.lens",
        "draw2.outline.lens.same",
        "draw2.say.lens",
        "draw2.keys.lens",
        "draw2.style.field.display.lens",
        "draw2.style.choice.display.lens.off",
        "draw2.style.choice.display.lens.tint",
        "draw2.style.choice.display.lens.tint+ghost",
    ] {
        assert!(lang.contains(&format!("\"{key}\"")), "{key}");
    }
}

/// `n` pairs of small structures (a chain of 3 and a chain of 4 with the same maps), each joined by an anchored arrow.
fn pairs(n: u32) -> Document {
    let (mut atoms, mut bonds, mut objects) = (Vec::new(), Vec::new(), Vec::new());
    let (mut a, mut b, mut o) = (0u32, 0u32, 0u32);
    for k in 0..n {
        let first = a + 1;
        let x0 = k as f64 * 20.0;
        for (len, dx) in [(3u32, 0.0), (4u32, 8.0)] {
            let start = a + 1;
            for i in 0..len {
                a += 1;
                atoms.push(
                    json!({"id": a, "el": "C", "x": x0 + dx + i as f64, "y": 0.0, "map": i + 1}),
                );
                if i > 0 {
                    b += 1;
                    bonds.push(json!({"id": 1_000_000 + b, "a": a - 1, "b": a, "o": 1}));
                }
            }
            let _ = start;
        }
        o += 1;
        objects.push(json!({"id": 2_000_000 + o, "t":"arrow","x":x0 + 3.0,"y":0.0,"x2":x0 + 7.5,"y2":0.0,"from":first,"to":first + 3}));
    }
    let doc =
        json!({"format":"mzdraw/1","next":3_000_000,"atoms":atoms,"bonds":bonds,"objects":objects});
    Document::from_json(&doc.to_string()).unwrap()
}

fn time(f: impl Fn()) -> f64 {
    let t = std::time::Instant::now();
    f();
    t.elapsed().as_secs_f64()
}

/// The time of `reps` runs of `f` over `n` pairs, the drawing built outside the clock.
fn reps_time(n: u32, reps: u32, f: impl Fn(&Document)) -> f64 {
    let d = pairs(n);
    time(|| {
        for _ in 0..reps {
            f(&d);
        }
    })
}

#[test]
fn the_cost_of_the_lens_grows_linearly_with_the_drawing() {
    // four times the pairs must cost about four times as much, not sixteen: one table for the whole drawing, one pass per arrow. The
    // drawing is capped at 6000 atoms, so the runs are repeated to be long enough to measure; the same test fails (ratio 16 and
    // more, in release too) when the tables are rebuilt per arrow.
    let run = |d: &Document| {
        let l = mzdraw::tp::lenses(d).unwrap();
        assert_eq!(l.lenses.len(), d.objects.len());
    };
    let (small, big) = (reps_time(200, 10, run), reps_time(800, 10, run));
    assert!(
        big < 8.0 * small + 0.05,
        "200 pairs {small:.3} s, 800 pairs {big:.3} s"
    );
    // the machine form is linear too (the display list has its own test in `scene.rs`: the whole painter has older quadratic
    // passes, the SVG text alternative one more, which are not this slice's)
    let json = |d: &Document| {
        let mut e = Editor::default();
        e.open(&d.to_json()).unwrap();
        for _ in 0..20 {
            e.save("lens").unwrap();
        }
    };
    let (small, big) = (reps_time(200, 1, json), reps_time(800, 1, json));
    assert!(
        big < 8.0 * small + 0.05,
        "200 pairs {small:.3} s, 800 pairs {big:.3} s"
    );
}

#[test]
fn a_hostile_number_of_comparisons_is_refused_by_the_work_cap() {
    // one parent of 4000 atoms and 501 arrows that leave it: 501 x 4001 atoms compared is over the cap
    let (np, nt) = (4000u32, 501u32);
    let mut atoms: Vec<Value> = (1..=np)
        .map(|i| json!({"id": i, "el": "C", "x": i as f64, "y": 0.0}))
        .collect();
    let bonds: Vec<Value> = (1..np)
        .map(|i| json!({"id": 100_000 + i, "a": i, "b": i + 1, "o": 1}))
        .collect();
    let mut objects = Vec::new();
    for k in 0..nt {
        atoms.push(json!({"id": 10_000 + k, "el": "C", "x": k as f64 * 3.0, "y": 50.0}));
        objects.push(json!({"id": 200_000 + k, "t":"arrow","x":0.0,"y":1.0,"x2":0.0,"y2":2.0,"from":1,"to":10_000 + k}));
    }
    let doc = Document::from_json(
        &json!({"format":"mzdraw/1","next":300_000,"atoms":atoms,"bonds":bonds,"objects":objects})
            .to_string(),
    )
    .unwrap();
    let t = std::time::Instant::now();
    let l = mzdraw::tp::lenses(&doc).unwrap();
    assert!(t.elapsed().as_secs_f64() < 5.0);
    // the first ones are compared (and have no maps here), the rest are left out over cap, each one listed
    let over = l.skipped.iter().filter(|k| k.why == "over cap").count();
    assert!(over > 0 && over < 501, "{over}");
    assert_eq!(l.skipped.len(), 501, "no arrow is dropped in silence");
}

#[test]
fn the_canvas_menu_offers_the_lens_only_where_an_arrow_is_anchored_and_each_press_goes_on() {
    let item = |e: &mut Editor| -> Option<Value> {
        let menu: Value = serde_json::from_str(&e.context_menu(500.0, 500.0)).unwrap();
        (menu["items"].as_array().unwrap().iter())
            .find(|i| i["id"] == "canvas.lens")
            .cloned()
    };
    // a drawing without an anchored arrow: no item
    let mut e = Editor::default();
    e.import(&lactam().to_string()).unwrap();
    assert!(item(&mut e).is_none());
    // an anchored pair: the item is there, unchecked, and runs the next mode
    let mut e = Editor::default();
    e.open(&pair(&[1, 2], &[1, 2, 3]).to_json()).unwrap();
    assert_eq!(item(&mut e).unwrap()["checked"], false);
    let before = e.doc().to_json();
    assert_eq!(e.context_run("canvas.lens") & mzdraw::editor::CHANGED, 0);
    assert_eq!(e.style.lens.name(), "tint");
    assert_eq!(item(&mut e).unwrap()["checked"], true);
    // a menu runs one item and closes: open it again for each press
    for _ in 0..2 {
        item(&mut e);
        e.context_run("canvas.lens");
    }
    assert_eq!(e.style.lens.name(), "off");
    assert_eq!(e.doc().to_json(), before, "the document is never touched");
}

#[test]
fn the_state_of_the_lens_for_the_page_finds_a_product_by_its_anchor_in_one_lookup() {
    // the drawing is capped at 6000 atoms, so the table is tested at the size of a hostile list of lenses: four times the products must
    // cost about four times as much, not sixteen (a scan per outline piece takes it to a ratio of 16)
    let run = |n: u32| {
        let l = mzdraw::tp::Lenses {
            lenses: (1..=n)
                .map(|i| mzdraw::tp::Lens {
                    arrow: i,
                    anchor: i,
                    delta: Some("+O".into()),
                    from: "C".into(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let t = std::time::Instant::now();
        let st = mzdraw::tp::LensState::from_lenses(&l);
        let found = (1..=n).filter(|&i| st.line(i).is_some()).count();
        assert_eq!(found, n as usize, "every product has its words");
        t.elapsed().as_secs_f64()
    };
    let (small, big) = (run(16_000), run(64_000));
    assert!(
        big < 8.0 * small + 0.05,
        "16000 products {small:.3} s, 64000 {big:.3} s"
    );
    // from a drawing: each product of a pathway has its words and nothing is noted
    let d = pairs(200);
    let st = mzdraw::tp::LensState::of(&d);
    let found = mzdraw::layout::pieces(&d)
        .iter()
        .filter(|p| st.line(p[0]).is_some())
        .count();
    assert_eq!(found, 200);
    assert!(st.note.is_none());
}

#[test]
fn the_words_of_a_product_come_from_the_formulas_of_label_and_follow_its_rules() {
    // the same counts as `pathway:label`: P and T of the derive, and the words of the page
    let patch = json!([{"op":"atom.add","el":"O","x":-1.0,"y":-1.7321},{"op":"bond.add","a":5,"b":-1,"o":1}]);
    let d = doc_after(patch);
    let st = mzdraw::tp::LensState::of(&d);
    let anchor = mzdraw::tp::lenses(&d).unwrap().lenses[0].anchor;
    let line = st.line(anchor).unwrap();
    assert_eq!(
        (line.from.as_str(), line.delta.as_str(), line.cleaved),
        ("C6H11NO", "+O", 0)
    );
    let r = replay(
        json!({"import": lactam().to_string(), "commands": [derive(json!([{"op":"atom.add","el":"O","x":-1.0,"y":-1.7321},{"op":"bond.add","a":5,"b":-1,"o":1}])), {"command":"pathway:label","args":{}}], "report": true, "out": "mzdraw"}),
    );
    let labelled = Document::from_json(r["out"].as_str().unwrap()).unwrap();
    assert!(
        labelled.objects.iter().any(|o| o.text == "+O"),
        "pathway:label writes the same delta"
    );
    // equal counts and a different net charge: no honest delta, as for `pathway:label` (it leaves such an arrow alone)
    let charge = json!({"format":"mzdraw/1","next":10,"atoms":[
        {"id":1,"el":"C","x":0,"y":0,"h":0,"map":1},{"id":2,"el":"C","x":6,"y":0,"h":0,"chg":-1,"map":1}],
        "bonds":[],"objects":[{"id":3,"t":"arrow","x":1.5,"y":0,"x2":4.5,"y2":0,"from":1,"to":2}]});
    let r = replay(
        json!({"import": charge.to_string(), "commands": [{"command":"pathway:label","args":{}}], "report": true, "out": "lens"}),
    );
    assert!(
        r["out"].as_str().unwrap().contains(r#""delta":null"#),
        "{r}"
    );
    let d = Document::from_json(&charge.to_string()).unwrap();
    assert!(mzdraw::tp::LensState::of(&d).line(1).is_none());
    let r = replay(
        json!({"import": charge.to_string(), "commands": [{"command":"pathway:label","args":{}}], "report": true, "out": "mzdraw"}),
    );
    assert!(
        Document::from_json(r["out"].as_str().unwrap())
            .unwrap()
            .objects[0]
            .text
            .is_empty(),
        "label leaves it alone"
    );
}
