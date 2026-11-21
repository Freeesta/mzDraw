//! Named steps (tp-10): the table, the sites, derive by step, `name+delta`. Synthetic structures only (SMILES written here).

use mzdraw::tp::{self, Kind, STEPS};
use mzdraw::Editor;
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[path = "support/timing.rs"]
mod timing;

fn replay(request: Value) -> Value {
    serde_json::from_str(&mzdraw::replay::replay(&request.to_string()).unwrap()).unwrap()
}

fn err_key(request: Value) -> &'static str {
    mzdraw::replay::replay(&request.to_string())
        .unwrap_err()
        .key
}

fn derive(of: u32, step: &str, site: u32) -> Value {
    json!({"command":"tp:derive","args":{"of":of,"step":step,"site":site}})
}

fn sites(smiles: &str, step: &str) -> Vec<u64> {
    let r = replay(json!({"import": smiles, "out": "sites", "args": {"of": 1, "step": step}}));
    assert_eq!(r["format"], "sites/1");
    assert_eq!(r["step"], step);
    r["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_u64().unwrap())
        .collect()
}

/// Element counts of a Hill formula text such as `C3H8O` or `C2H3ClO2` (no charge).
fn counts(formula: &str) -> BTreeMap<String, i64> {
    let mut out = BTreeMap::new();
    let ch: Vec<char> = formula.chars().collect();
    let mut i = 0;
    while i < ch.len() {
        let mut el = ch[i].to_string();
        i += 1;
        while i < ch.len() && ch[i].is_ascii_lowercase() {
            el.push(ch[i]);
            i += 1;
        }
        let mut n = String::new();
        while i < ch.len() && ch[i].is_ascii_digit() {
            n.push(ch[i]);
            i += 1;
        }
        *out.entry(el).or_default() += n.parse::<i64>().unwrap_or(1);
    }
    out
}

/// The Hill formulas of the structures of a drawing, left to right, as the digest says them (a code path of its own).
fn formulas(doc_json: &str) -> Vec<BTreeMap<String, i64>> {
    let d = mzdraw::Document::from_json(doc_json).unwrap();
    let digest: Value = serde_json::from_str(
        &mzdraw::replay::replay(&json!({"import": d.to_json(), "out": "digest"}).to_string())
            .unwrap(),
    )
    .unwrap();
    digest["pieces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| counts(s["formula"].as_str().unwrap()))
        .collect()
}

/// Parses a delta such as `+H-Cl` or `-CH2` with the element counts of the parent and the TP: the difference of two formulas.
fn delta_counts(delta: &str) -> BTreeMap<String, i64> {
    let mut out: BTreeMap<String, i64> = BTreeMap::new();
    let mut sign = 1;
    let ch: Vec<char> = delta.chars().collect();
    let mut i = 0;
    while i < ch.len() {
        match ch[i] {
            '+' => sign = 1,
            '-' => sign = -1,
            _ => {
                let mut el = ch[i].to_string();
                while i + 1 < ch.len() && ch[i + 1].is_ascii_lowercase() {
                    i += 1;
                    el.push(ch[i]);
                }
                let mut n = String::new();
                while i + 1 < ch.len() && ch[i + 1].is_ascii_digit() {
                    i += 1;
                    n.push(ch[i]);
                }
                *out.entry(el).or_default() += sign * n.parse::<i64>().unwrap_or(1);
            }
        }
        i += 1;
    }
    out
}

/// For every row: a parent that has the site (SMILES, a site id), and one that lacks it. Atom ids follow the SMILES atoms from 1,
/// bond ids come after the atoms in bond order.
const CASES: &[(&str, &str, u32, &str)] = &[
    ("hydroxylation", "Cc1ccccc1", 3, "O=C=O"),
    ("n-demethylation", "CN(C)C", 1, "CC"),
    ("o-demethylation", "COc1ccccc1", 1, "CC"),
    ("n-oxidation", "CN(C)C", 2, "CCC"),
    ("dehydrogenation", "CCC", 4, "C#C"),
    ("hydration", "CC=C", 5, "CC"),
    ("dechlorination", "CCCl", 3, "CC"),
    ("debromination", "CCBr", 3, "CC"),
    ("hydrolysis", "CC(=O)OC", 8, "CC"),
    ("acetylation", "CCO", 3, "CC"),
    ("sulfation", "CCO", 3, "CC"),
    ("glucuronidation", "CC(=O)O", 4, "CC"),
    ("decarboxylation", "CC(=O)O", 2, "CC"),
];

#[test]
fn the_table_has_every_row_with_a_canonical_delta_and_a_kind() {
    let names: Vec<&str> = STEPS.iter().map(|s| s.name).collect();
    assert_eq!(
        names,
        [
            "hydroxylation",
            "n-demethylation",
            "o-demethylation",
            "n-oxidation",
            "dehydrogenation",
            "hydration",
            "dechlorination",
            "debromination",
            "hydrolysis",
            "acetylation",
            "sulfation",
            "glucuronidation",
            "decarboxylation"
        ]
    );
    for s in STEPS {
        let parsed = mzlab_mol::delta::parse(s.delta).unwrap();
        assert_eq!(mzlab_mol::delta::print(&parsed), s.delta, "{}", s.name);
        let bond = matches!(s.name, "dehydrogenation" | "hydration" | "hydrolysis");
        assert_eq!(
            s.kind,
            if bond { Kind::Bond } else { Kind::Atom },
            "{}",
            s.name
        );
    }
    assert_eq!(CASES.len(), STEPS.len(), "a case for every row");
    // the output `steps`
    let r = replay(json!({"out": "steps"}));
    assert_eq!(r["format"], "steps/1");
    assert_eq!(r["steps"].as_array().unwrap().len(), STEPS.len());
    assert_eq!(
        r["steps"][0],
        json!({"name":"hydroxylation","delta":"+O","kind":"atom"})
    );
}

#[test]
fn every_step_adds_its_delta_at_a_site_it_lists_and_is_refused_where_there_is_none() {
    for &(step, smiles, site, lacking) in CASES {
        let listed = sites(smiles, step);
        assert!(
            listed.contains(&u64::from(site)),
            "{step}: {site} in {listed:?}"
        );
        let r = replay(json!({
            "import": smiles, "commands": [derive(1, step, site)], "report": true, "out": "mzdraw"
        }));
        assert_eq!(r["ok"], true, "{step}: {r}");
        let f = formulas(r["out"].as_str().unwrap());
        // the parent is the first structure, the TP the second (one column to the right)
        assert!(f.len() >= 2, "{step}: parent and TP");
        let mut sum = f[0].clone();
        for (el, n) in delta_counts(STEPS.iter().find(|s| s.name == step).unwrap().delta) {
            *sum.entry(el).or_default() += n;
        }
        // hydrolysis cuts the structure in two: the TP is every structure after the parent
        let mut tp: BTreeMap<String, i64> = BTreeMap::new();
        let parts = if step == "hydrolysis" || step == "decarboxylation" {
            &f[1..]
        } else {
            &f[1..2]
        };
        for p in parts {
            for (el, n) in p {
                *tp.entry(el.clone()).or_default() += n;
            }
        }
        sum.retain(|_, n| *n != 0);
        tp.retain(|_, n| *n != 0);
        assert_eq!(
            tp, sum,
            "{step} on {smiles}: formula of the TP = parent + delta"
        );

        // a parent without the site: no sites, and the derive is a verdict (exit 1), nothing changes
        assert_eq!(sites(lacking, step), Vec::<u64>::new(), "{step} lacking");
        let r = replay(json!({
            "import": lacking, "commands": [derive(1, step, 1)], "report": true, "out": "mzdraw"
        }));
        assert_eq!(r["ok"], false, "{step}: {r}");
        assert_eq!(r["failed"]["key"], "error.draw2.tpSite");
        assert_eq!(r["failed"]["stage"], "command");
    }
}

/// Atom ids (or bond ids) of the structure that holds the atom 1, by the document of an import.
fn ids_of(smiles: &str, kind: Kind) -> Vec<u32> {
    let mut e = Editor::default();
    e.import(smiles).unwrap();
    let d = e.doc();
    match kind {
        Kind::Atom => d.atoms.iter().map(|a| a.id).collect(),
        Kind::Bond => d.bonds.iter().map(|b| b.id).collect(),
    }
}

#[test]
fn sites_are_ascending_and_equal_what_the_derive_accepts_over_all_atoms_or_all_bonds() {
    // brute force through the public route: every atom id (atom steps) or bond id (bond steps) of a parent that holds several
    // kinds of site is tried with the derive; the accepted ones are the sites, ascending
    let parents = [
        "CN(C)CC(=O)OCc1ccncc1Cl",
        "COc1ccc(CCBr)cc1C(=O)NC",
        "CCC(=O)O.CC=CC(O)=O",
        "OC(=O)CC(N)C(=O)O",
    ];
    for smiles in parents {
        for s in STEPS {
            let listed = sites(smiles, s.name);
            assert!(
                listed.windows(2).all(|w| w[0] < w[1]),
                "{} ascending",
                s.name
            );
            let mut accepted = Vec::new();
            let mut all = ids_of(smiles, s.kind);
            all.extend(ids_of(
                smiles,
                if s.kind == Kind::Atom {
                    Kind::Bond
                } else {
                    Kind::Atom
                },
            ));
            all.sort_unstable();
            for id in all {
                let r = replay(json!({
                    "import": smiles, "commands": [derive(1, s.name, id)], "report": true
                }));
                if r["ok"] == true {
                    accepted.push(u64::from(id));
                } else {
                    assert_eq!(
                        r["failed"]["key"], "error.draw2.tpSite",
                        "{} {id}: {r}",
                        s.name
                    );
                }
            }
            // `sites` lists the structure of atom 1 only: keep the accepted ones that it can list
            let first: Vec<u32> = {
                let mut e = Editor::default();
                e.import(smiles).unwrap();
                let d = e.doc();
                let mut piece = vec![1u32];
                loop {
                    let n = piece.len();
                    for b in &d.bonds {
                        if piece.contains(&b.a) != piece.contains(&b.b) {
                            piece.push(if piece.contains(&b.a) { b.b } else { b.a });
                        }
                    }
                    if piece.len() == n {
                        break;
                    }
                }
                let mut ids = piece.clone();
                ids.extend(
                    d.bonds
                        .iter()
                        .filter(|b| piece.contains(&b.a))
                        .map(|b| b.id),
                );
                ids
            };
            // the derive of an atom of another structure of the drawing is a site of that structure: not listed here
            accepted.retain(|i| first.contains(&(*i as u32)));
            assert_eq!(listed, accepted, "{} on {smiles}", s.name);
        }
    }
    // each step has at least one parent in this list with a site, so the comparison is not empty everywhere
    assert!(!sites("CN(C)CC(=O)OCc1ccncc1Cl", "n-oxidation").is_empty());
}

#[test]
fn the_rule_of_every_row_holds_on_a_parent_that_tells_the_cases_apart() {
    // hydroxylation: C with an H, aromatic or not; not a quaternary C, not an O
    assert_eq!(
        sites("CC(C)(C)c1ccccc1", "hydroxylation"),
        [1, 3, 4, 6, 7, 8, 9, 10]
    );
    // demethylation needs a CH3 on N / on O, not a CH3 on C
    assert_eq!(sites("CN(C)Cc1ccccc1OC", "n-demethylation"), [1, 3]);
    assert_eq!(sites("CN(C)Cc1ccccc1OC", "o-demethylation"), [12]);
    // N-oxidation: a tertiary amine and a pyridine-type N; not an amine with an H, not a pyrrole-type N, not a charged N
    assert_eq!(sites("CN(C)C", "n-oxidation"), [2]);
    assert_eq!(sites("c1ccncc1", "n-oxidation"), [4]);
    assert_eq!(sites("CNC", "n-oxidation"), Vec::<u64>::new());
    assert_eq!(sites("Cn1cccc1", "n-oxidation"), Vec::<u64>::new());
    assert_eq!(sites("C[N+](C)(C)C", "n-oxidation"), Vec::<u64>::new());
    // dehydrogenation: a single C-C with an H on each end, not in an aromatic ring
    assert_eq!(sites("CC(C)(C)C", "dehydrogenation"), Vec::<u64>::new());
    assert_eq!(sites("c1ccccc1", "dehydrogenation"), Vec::<u64>::new());
    assert_eq!(sites("CCC", "dehydrogenation"), [4, 5]);
    // hydration: a C=C, not an aromatic ring bond, not C=O
    assert_eq!(sites("C=CC=O", "hydration"), [5]);
    assert_eq!(sites("c1ccccc1", "hydration"), Vec::<u64>::new());
    // hydrolysis: an ester or amide single bond next to C=O, not the acid C-OH, not a ketone
    assert_eq!(sites("CC(=O)O", "hydrolysis"), Vec::<u64>::new());
    assert_eq!(sites("CC(=O)C", "hydrolysis"), Vec::<u64>::new());
    assert_eq!(sites("CC(=O)N(C)C", "hydrolysis"), [9]);
    // N-H / O-H donors
    assert_eq!(sites("CC(=O)NCO", "acetylation"), [4, 6]);
    assert_eq!(sites("CC(=O)NCO", "sulfation"), [6]);
    // decarboxylation: the carbon of a CO2H, not an ester, not a carboxylate
    assert_eq!(sites("CC(=O)OC", "decarboxylation"), Vec::<u64>::new());
    assert_eq!(sites("CC(=O)[O-]", "decarboxylation"), Vec::<u64>::new());
    assert_eq!(sites("CC(=O)O", "decarboxylation"), [2]);
}

#[test]
fn the_n_oxide_has_its_charges_and_a_demethylated_nitrogen_gets_its_hydrogen() {
    let r =
        replay(json!({"import":"CN(C)C","commands":[derive(1,"n-oxidation",2)],"out":"mzdraw"}));
    let d = mzdraw::Document::from_json(&r.to_string()).unwrap();
    let copy: Vec<_> = d.atoms.iter().filter(|a| a.id > 4).collect();
    assert_eq!(copy.iter().filter(|a| a.chg == 1 && a.el == "N").count(), 1);
    assert_eq!(
        copy.iter().filter(|a| a.chg == -1 && a.el == "O").count(),
        1
    );
    // the parent keeps its atoms untouched
    assert!(d.atoms.iter().filter(|a| a.id <= 4).all(|a| a.chg == 0));
    let f = formulas(&d.to_json());
    assert_eq!((f[0].get("H"), f[1].get("H")), (Some(&9), Some(&9)));
    let r = replay(
        json!({"import":"CN(C)C","commands":[derive(1,"n-demethylation",1)],"out":"mzdraw"}),
    );
    let d = mzdraw::Document::from_json(&r.to_string()).unwrap();
    let f = formulas(&d.to_json());
    assert_eq!(
        (f[0].get("C"), f[1].get("C"), f[1].get("H")),
        (Some(&3), Some(&2), Some(&7))
    );
}

#[test]
fn a_step_at_an_atom_that_is_not_a_site_is_refused_with_its_key_and_changes_nothing() {
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    let before = e.doc().to_json();
    for (step, site) in [
        ("hydroxylation", 3),
        ("hydroxylation", 999),
        ("hydrolysis", 1),
    ] {
        let err = e
            .run_command("tp:derive", &json!({"of": 1, "step": step, "site": site}))
            .unwrap_err();
        assert_eq!(err.key, "error.draw2.tpSite", "{step} {site}");
        assert_eq!(e.doc().to_json(), before);
    }
}

#[test]
fn bad_shapes_are_invalid_requests() {
    let base =
        |args: Value| json!({"import": "CCO", "commands": [{"command":"tp:derive","args":args}]});
    for args in [
        json!({"of":1,"step":"hydroxylation"}),
        json!({"of":1,"site":1}),
        json!({"of":1,"step":"hydroxylation","site":1,"patch":[]}),
        json!({"of":1,"step":"hydroxylation","site":0}),
        json!({"of":1,"step":7,"site":1}),
    ] {
        assert_eq!(err_key(base(args.clone())), "error.draw2.request", "{args}");
    }
    assert_eq!(
        err_key(base(json!({"of":1,"step":"levitation","site":1}))),
        "error.draw2.tpStep"
    );
    assert_eq!(
        err_key(json!({"import":"CCO","out":"sites","args":{"of":1,"step":"levitation"}})),
        "error.draw2.tpStep"
    );
    assert_eq!(
        err_key(json!({"import":"CCO","out":"sites"})),
        "error.draw2.request"
    );
    assert_eq!(
        err_key(json!({"import":"CCO","out":"sites","args":{"of":1,"step":"hydrolysis","x":1}})),
        "error.draw2.request"
    );
    assert_eq!(
        err_key(json!({"import":"CCO","args":{"of":1,"step":"hydrolysis"}})),
        "error.draw2.request",
        "args without a command or `out` sites"
    );
    assert_eq!(
        err_key(json!({"import":"CCO","out":"sites","args":{"of":99,"step":"hydrolysis"}})),
        "error.draw2.noAtom"
    );
}

#[test]
fn undo_of_a_step_derive_returns_the_identical_document_and_the_log_replays() {
    for &(step, smiles, site, _) in CASES {
        let mut e = Editor::default();
        e.import(smiles).unwrap();
        let before = e.doc().to_json();
        e.run_command("tp:derive", &json!({"of": 1, "step": step, "site": site}))
            .unwrap();
        let after = e.doc().to_json();
        assert_ne!(after, before, "{step}");
        e.undo();
        assert_eq!(e.doc().to_json(), before, "{step}: one undo step");
        e.redo();
        assert_eq!(e.doc().to_json(), after, "{step}: redo");
        // the log holds plain operations: the page and the CLI route give the same bytes
        let log = e.save("log").unwrap();
        let ops: Value = serde_json::from_str(&log).unwrap();
        let again = replay(json!({"log": ops, "out": "mzdraw"}));
        let after: Value = serde_json::from_str(&after).unwrap();
        assert_eq!(again, after, "{step}");
    }
}

#[test]
fn a_wrong_edit_cannot_ship_the_delta_of_the_copy_is_checked() {
    // a patch that adds a nitrogen where the delta says +O, and one that removes nothing where -CH2 is said, are refused and write
    // nothing; the right patch is accepted
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    let style = mzdraw::settings::Style::default();
    let add = |el: &str| {
        mzdraw::ops::Op::parse(&json!({"op":"atom.add","el":el,"x":-1.0,"y":-1.7321})).unwrap()
    };
    let bond =
        mzdraw::ops::Op::parse(&json!({"op":"bond.add","a":1,"b":mzdraw::tp::new_ref(1),"o":1}))
            .unwrap();
    let ok = tp::derive_expecting(
        e.doc(),
        &style,
        1,
        &[add("O"), bond.clone()],
        None,
        Some("+O"),
    );
    assert!(ok.is_ok(), "{ok:?}");
    let bad = tp::derive_expecting(
        e.doc(),
        &style,
        1,
        &[add("N"), bond.clone()],
        None,
        Some("+O"),
    );
    assert_eq!(bad.unwrap_err(), "error.draw2.tpDelta");
    let bad = tp::derive_expecting(e.doc(), &style, 1, &[], None, Some("-CH2"));
    assert_eq!(bad.unwrap_err(), "error.draw2.tpDelta");
    let none = tp::derive_expecting(e.doc(), &style, 1, &[add("N"), bond], None, None);
    assert!(none.is_ok(), "no expected delta: nothing to check");
}

#[test]
fn name_and_delta_names_a_step_only_when_one_step_has_the_delta() {
    // a TP pair: ethanol and acetaldehyde-like pairs built by hand: the label of an arrow is the name and the delta
    let scheme = |a: &str, b: &str| {
        let mut e = Editor::default();
        e.import(a).unwrap();
        e.import(b).unwrap();
        e
    };
    assert_eq!(tp::name_of("-H2"), Some("dehydrogenation"));
    assert_eq!(tp::name_of("+H-Cl"), Some("dechlorination"));
    assert_eq!(
        tp::name_of("+O"),
        None,
        "hydroxylation and n-oxidation share +O"
    );
    assert_eq!(tp::name_of("-CH2"), None);
    assert_eq!(tp::name_of("+H2O"), None);
    assert_eq!(tp::name_of("+N"), None, "no step");
    let _ = scheme("CC", "C=C");
}

/// `n` two-atom structures on a wide grid (atom ids 1..=2n, bond ids after them).
fn many(n: u32) -> String {
    let mut atoms = Vec::new();
    let mut bonds = Vec::new();
    for k in 0..n {
        let (x, y) = ((k % 60) as f64 * 6.0, -((k / 60) as f64) * 6.0);
        atoms.push(json!({"id": 2 * k + 1, "el": "C", "x": x, "y": y}));
        atoms.push(json!({"id": 2 * k + 2, "el": "O", "x": x + 1.0, "y": y}));
        bonds.push(json!({"id": 2 * n + k + 1, "a": 2 * k + 1, "b": 2 * k + 2, "o": 1}));
    }
    json!({"format":"mzdraw/1","next":3 * n + 1,"atoms":atoms,"bonds":bonds}).to_string()
}

/// A chain of `n` carbons ending in a chlorine (atom ids 1..=n+1; bond ids after them).
fn chain(n: u32) -> String {
    let mut atoms: Vec<Value> = (1..=n)
        .map(|i| json!({"id": i, "el": "C", "x": f64::from(i), "y": 0.0}))
        .collect();
    atoms.push(json!({"id": n + 1, "el": "Cl", "x": f64::from(n + 1), "y": 0.0}));
    let bonds: Vec<Value> = (1..=n)
        .map(|i| json!({"id": n + 1 + i, "a": i, "b": i + 1, "o": 1}))
        .collect();
    json!({"format":"mzdraw/1","next":2 * n + 2,"atoms":atoms,"bonds":bonds}).to_string()
}

fn best_of_three(run: impl Fn()) -> f64 {
    // the least CPU time of the thread over nine runs after one that is not timed: noise only ever adds time
    timing::best_of(1, 9, run).as_secs_f64()
}

/// The hydroxylation of atom 1 written as a plain patch (what the step expands to), for the comparison with the step.
fn hydroxylation_patch() -> Vec<mzdraw::ops::Op> {
    [
        json!({"op":"atom.add","el":"O","x":-1.0,"y":-1.7321}),
        json!({"op":"bond.add","a":1,"b":mzdraw::tp::new_ref(1),"o":1}),
    ]
    .iter()
    .map(|o| mzdraw::ops::Op::parse(o).unwrap())
    .collect()
}

#[test]
fn a_step_derive_costs_no_more_than_the_plain_derive_of_the_same_edit() {
    // The shared part (the import, the pieces and boxes of the drawing, the placement, the arrow route) is `tp::derive` of tp-03 and
    // is not what this slice adds. The document is built once, outside the timing; the step derive and the plain derive of the
    // same edit are timed on it, at two sizes, so only the code of the step (the structure, the record, the tests, the delta check)
    // is measured. A step that scanned the drawing per atom would grow with the size and break the bound at the larger one.
    let style = mzdraw::Style::default();
    for n in [2000u32, 8000] {
        let doc = mzdraw::Document::from_json(&many(n)).unwrap();
        let patch = hydroxylation_patch();
        let plain = best_of_three(|| {
            for _ in 0..4 {
                tp::derive(&doc, &style, 1, &patch, None).unwrap();
            }
        });
        let step = best_of_three(|| {
            for _ in 0..4 {
                tp::derive_step(&doc, &style, 1, "hydroxylation", 1, None).unwrap();
            }
        });
        assert!(
            step <= 1.5 * plain + 0.03,
            "{n} structures: step derive {step:.3} s, plain derive {plain:.3} s"
        );
    }
}

#[test]
fn the_sites_of_a_big_structure_cost_its_size_not_its_square() {
    // The site computation alone, on a document built once: no import. A chain of n atoms and one of 4n: linear is 4 times, a
    // quadratic search 16; the bound is 9 with a floor against noise. Best of three.
    let doc_of = |n: u32| mzdraw::Document::from_json(&chain(n)).unwrap();
    let (small, big) = (doc_of(1200), doc_of(4800));
    for step in [
        "hydroxylation",
        "dehydrogenation",
        "dechlorination",
        "hydrolysis",
        "n-oxidation",
    ] {
        let time = |d: &mzdraw::Document| {
            best_of_three(|| {
                tp::sites(d, 1, step).unwrap();
            })
        };
        let (ts, tb) = (time(&small), time(&big));
        assert!(
            tb < 9.0 * ts + 0.02,
            "sites of {step}: 1200 atoms {ts:.4} s, 4800 atoms {tb:.4} s"
        );
    }
}

#[test]
fn a_structure_above_the_size_cap_is_refused_before_any_work() {
    let sites = |n: u32| {
        mzdraw::replay::replay(
            &json!({"import": chain(n), "out": "sites", "args": {"of": 1, "step": "hydroxylation"}})
                .to_string(),
        )
    };
    // 5000 atoms are fine (n carbons and the chlorine), 5001 are refused with the key
    assert_eq!(
        serde_json::from_str::<Value>(&sites(4999).unwrap()).unwrap()["ids"]
            .as_array()
            .unwrap()
            .len(),
        4999
    );
    assert_eq!(sites(5000).unwrap_err().key, "error.draw2.tpSize");
    let derive_big =
        json!({"import": chain(5000), "report": true, "commands": [derive(1, "hydroxylation", 1)]});
    assert_eq!(replay(derive_big)["failed"]["key"], "error.draw2.tpSize");
}
