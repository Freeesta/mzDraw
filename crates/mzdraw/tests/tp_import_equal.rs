//! Equal molecules in a TP import (tp-import-equal). Synthetic structures only.
//!
//! A list may hold the same structure on any number of rows: each row keeps its own node, its own number and its own arrow, and the
//! layout gives equal children the same column and an even, symmetric stack. The import never compares structures with each other, so
//! its work depends on the size of the list only; these tests assert completion and exact results, never a wall-clock ratio.

use mzdraw::{layout, Document, Editor};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FLAT: [&str; 3] = ["CCO", "c1ccccc1", "C[C@H](N)C(=O)O"];

/// A parent row `P` and `n - 1` children of it, every row the same structure.
fn star(n: usize, smiles: &str) -> String {
    let mut s = format!("smiles,name,parent\n{smiles},P,\n");
    for _ in 1..n {
        s.push_str(&format!("{smiles},,P\n"));
    }
    s
}

fn import(csv: &str) -> Editor {
    let mut e = Editor::default();
    e.run_command("tp:import", &json!({ "csv": csv }))
        .unwrap_or_else(|err| panic!("{}: {csv}", err.key));
    e
}

/// `(TP_ID, centre x, centre y)` of every structure: the mean of its atoms (equal structures have equal shapes, so means compare).
fn nodes(doc: &Document) -> Vec<(String, f64, f64)> {
    layout::pieces(doc)
        .into_iter()
        .map(|p| {
            let atoms: Vec<_> = doc.atoms.iter().filter(|a| p.contains(&a.id)).collect();
            let n = atoms.len() as f64;
            let name = atoms
                .iter()
                .flat_map(|a| a.data())
                .find(|(k, _)| k == "TP_ID")
                .map(|(_, v)| v)
                .unwrap_or_default();
            (
                name,
                atoms.iter().map(|a| a.x).sum::<f64>() / n,
                atoms.iter().map(|a| a.y).sum::<f64>() / n,
            )
        })
        .collect()
}

fn arrows(doc: &Document) -> usize {
    doc.objects
        .iter()
        .filter(|o| o.from != 0 && o.to != 0)
        .count()
}

/// The shape every import of equal children must have: one node, one arrow and one number each, all children in one column, stacked
/// evenly and symmetrically about the parent.
fn assert_star(doc: &Document, n: usize) {
    let all = nodes(doc);
    assert_eq!(all.len(), n, "one node per row");
    assert_eq!(arrows(doc), n - 1, "one arrow per parent link");
    let root = all
        .iter()
        .find(|r| r.0 == "P")
        .expect("the root keeps its name");
    let mut kids: Vec<_> = all.iter().filter(|r| r.0 != "P").cloned().collect();
    kids.sort_by(|a, b| a.2.total_cmp(&b.2));
    let names: Vec<_> = kids.iter().map(|k| k.0.clone()).collect();
    let want: Vec<_> = (1..n).map(|i| format!("TP{i}")).collect();
    assert_eq!(
        names, want,
        "numbered 1..n-1 by the layout (smallest y first)"
    );
    assert!(
        kids.iter().all(|k| k.1 > root.1 + 1.0),
        "children right of the root"
    );
    assert!(
        kids.iter().all(|k| (k.1 - kids[0].1).abs() < 1e-6),
        "one column"
    );
    let step = if kids.len() > 1 {
        kids[1].2 - kids[0].2
    } else {
        0.0
    };
    assert!(kids.len() == 1 || step > 0.5, "children do not overlap");
    for w in kids.windows(2) {
        assert!((w[1].2 - w[0].2 - step).abs() < 1e-6, "an even stack");
    }
    let mean = kids.iter().map(|k| k.2).sum::<f64>() / kids.len() as f64;
    assert!((mean - root.2).abs() < 1e-6, "symmetric about the parent");
}

#[test]
fn two_to_twenty_four_equal_molecules_each_keep_a_node_a_number_and_an_arrow() {
    for smiles in FLAT {
        for n in 2..=24 {
            assert_star(import(&star(n, smiles)).doc(), n);
        }
    }
}

#[test]
fn equal_molecules_without_any_parent_link_stay_in_a_column_numbered_by_row() {
    for n in [2usize, 8, 24] {
        let mut csv = String::from("smiles,name\n");
        for _ in 0..n {
            csv.push_str("CCO,\n");
        }
        let e = import(&csv);
        assert_eq!(nodes(e.doc()).len(), n);
        assert_eq!(arrows(e.doc()), 0);
        // no pathway: the structures stay in one column and the nameless rows are numbered in the order of the rows
        let mut all = nodes(e.doc());
        all.sort_by(|a, b| a.2.total_cmp(&b.2));
        assert!(
            all.iter().all(|r| (r.1 - all[0].1).abs() < 1e-6),
            "one column"
        );
        let names: Vec<_> = all.iter().map(|r| r.0.clone()).collect();
        assert_eq!(names, (1..=n).map(|i| format!("TP{i}")).collect::<Vec<_>>());
    }
}

#[test]
fn a_chain_of_equal_molecules_is_a_chain() {
    for n in [3usize, 8, 24] {
        let mut csv = String::from("smiles,name,parent\nc1ccccc1,N0,\n");
        for i in 1..n {
            csv.push_str(&format!("c1ccccc1,N{i},N{}\n", i - 1));
        }
        let e = import(&csv);
        let mut all = nodes(e.doc());
        all.sort_by(|a, b| a.1.total_cmp(&b.1));
        assert_eq!(all.len(), n);
        assert_eq!(arrows(e.doc()), n - 1);
        // one layer per row: strictly increasing x, the same y
        assert!(all.windows(2).all(|w| w[1].1 > w[0].1 + 1.0));
        assert!(all.iter().all(|r| (r.2 - all[0].2).abs() < 1e-6));
        let names: Vec<_> = all.iter().map(|r| r.0.clone()).collect();
        assert_eq!(names, (0..n).map(|i| format!("N{i}")).collect::<Vec<_>>());
    }
}

#[test]
fn equal_and_different_molecules_mixed_keep_their_own_rows() {
    // four ethanols and two propanols under one butanol
    let kinds = ["CCO", "CCO", "CCCO", "CCO", "CCCO", "CCO"];
    let mut csv = String::from("smiles,name,parent\nCCCCO,P,\n");
    for k in kinds {
        csv.push_str(&format!("{k},,P\n"));
    }
    let e = import(&csv);
    let doc = e.doc();
    assert_eq!(nodes(doc).len(), 7);
    assert_eq!(arrows(doc), 6);
    let mut names: Vec<_> = nodes(doc).into_iter().map(|r| r.0).collect();
    names.sort();
    assert_eq!(names, ["P", "TP1", "TP2", "TP3", "TP4", "TP5", "TP6"]);
    // each row holds its own SMILES: the rows' atom counts per structure are those of the list
    let mut sizes: BTreeMap<usize, usize> = BTreeMap::new();
    for p in layout::pieces(doc) {
        *sizes.entry(p.len()).or_default() += 1;
    }
    assert_eq!(sizes, BTreeMap::from([(3, 4), (4, 2), (5, 1)]));
}

/// A seeded list of 2..=24 rows over three structures; every row after the first has one parent among the rows before it (named).
fn random_list(seed: u64) -> (String, usize) {
    let mut s = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let mut next = move |m: usize| {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((s >> 33) as usize) % m
    };
    let rows = 2 + next(23);
    let mut csv = String::from("smiles,name,parent\n");
    for i in 0..rows {
        let smiles = FLAT[next(2)];
        if i == 0 {
            csv.push_str(&format!("{smiles},R0,\n"));
        } else {
            csv.push_str(&format!("{smiles},R{i},R{}\n", next(i)));
        }
    }
    (csv, rows)
}

#[test]
fn the_import_of_a_random_list_is_deterministic_and_has_an_exact_undo() {
    for seed in 0..60u64 {
        let (csv, rows) = random_list(seed);
        let mut a = Editor::default();
        let empty = a.doc().to_json();
        a.run_command("tp:import", &json!({ "csv": csv })).unwrap();
        let done = a.doc().to_json();
        // the same text gives the same bytes, in a second editor
        assert_eq!(import(&csv).doc().to_json(), done, "seed {seed}");
        assert_eq!(nodes(a.doc()).len(), rows, "seed {seed}: one node per row");
        assert_eq!(arrows(a.doc()), rows - 1, "seed {seed}: one arrow per link");
        a.undo();
        assert_eq!(
            a.doc().to_json(),
            empty,
            "seed {seed}: undo is the exact inverse"
        );
        a.redo();
        assert_eq!(a.doc().to_json(), done, "seed {seed}: redo");
        // a second import of the same list adds a second drawing and is one step too
        a.run_command("tp:import", &json!({ "csv": csv })).unwrap();
        a.undo();
        assert_eq!(
            a.doc().to_json(),
            done,
            "seed {seed}: the second import undoes alone"
        );
    }
}

#[test]
fn the_log_of_an_import_of_equal_molecules_replays_to_the_same_bytes() {
    let csv = star(8, "c1ccccc1");
    let req = json!({"commands":[{"command":"tp:import","args":{"csv":csv}}],"out":"mzdraw"});
    let a = mzdraw::replay::replay(&req.to_string()).unwrap();
    let b = mzdraw::replay::replay(&req.to_string()).unwrap();
    assert_eq!(a, b);
    assert_eq!(a, import(&csv).doc().to_json());
}

// ---------------------------------------------------------------------------------------------------------------------------------
// The limit of `mzlab-mol`: `Mol::canonical()` on a whole drawing of equal components doubles in time with each equal component
// (measured: benzene, debug build, 6 ms at 4 copies, 12 ms at 5, 25 ms at 6, 45 ms at 7, 87 ms at 8). Nothing here calls it on a whole
// drawing; this test pins the places that call it, so that a new caller (which may meet equal molecules) is noticed and looked at.

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn the_canonical_form_of_mzlab_mol_is_taken_in_two_places_only_and_never_of_a_whole_drawing() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for c in ["mzdraw", "mzdraw-wasm"] {
        sources(&crates.join(c).join("src"), &mut files);
    }
    // (file, text) -> how many lines of code (comments left out) hold it
    let mut found: BTreeMap<(String, &str), usize> = BTreeMap::new();
    for f in &files {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        for line in std::fs::read_to_string(f).unwrap().lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for needle in [".canonical()", "canonical_order(", "canonical_text("] {
                if line.contains(needle) {
                    *found.entry((name.clone(), needle)).or_default() += 1;
                }
            }
        }
    }
    let got: Vec<_> = found
        .iter()
        .map(|((f, n), c)| format!("{f} {n} x{c}"))
        .collect();
    assert_eq!(
        got,
        [
            // the signature of one piece (`sub_document` of a piece, or the neighbourhood of one stereo element)
            "stereo_guard.rs .canonical() x1",
            "stereo_guard.rs canonical_text( x5",
            // the parent and the product of one transformation record
            "trasf.rs canonical_order( x1",
        ],
        "a new caller of Mol::canonical / canonical_order: it doubles in time per equal component on a whole drawing \
         (docs/built/tp-import-equal.md); call it on one piece only, then update this list"
    );
}
