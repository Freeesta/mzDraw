//! Reaction files: RXN V2000 / V3000 and RD files in, RXN V2000 out. Synthetic structures only.

use mzdraw::io::import;
use mzdraw::{Document, Editor, ObjKind};
use mzlab_mol::Mol;

fn drawn(text: &str) -> Document {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e.doc().clone()
}

fn canonical(smiles: &str) -> String {
    Mol::from_smiles(smiles)
        .unwrap()
        .canonical()
        .to_smiles()
        .unwrap()
}

fn mol(smiles: &str) -> String {
    drawn(smiles).to_molfile() + "\n"
}

/// A V2000 reaction file as other programs write it: the header, the counts, a `$MOL` block per component.
fn rxn(reactants: &[&str], products: &[&str], agents: &[&str]) -> String {
    let mut t = format!(
        "$RXN\n\n      synthetic\n\n{:>3}{:>3}",
        reactants.len(),
        products.len()
    );
    if !agents.is_empty() {
        t += &format!("{:>3}", agents.len());
    }
    t.push('\n');
    for s in reactants.iter().chain(products).chain(agents) {
        t += "$MOL\n";
        t += &mol(s);
    }
    t
}

/// The CTAB block of a V3000 molfile of the crate, as it stands inside a V3000 reaction file.
fn ctab(smiles: &str) -> String {
    let v3 = drawn(smiles).to_molfile_v3000();
    let from = v3.find("M  V30 BEGIN CTAB").unwrap();
    v3[from..v3.find("M  END").unwrap()].to_string()
}

/// The structures of a drawing by where they stand: (left of the arrow, right of it, above it), each as canonical SMILES.
fn sides(d: &Document) -> [Vec<String>; 3] {
    let arrow = d.objects.iter().find(|o| o.t == ObjKind::Arrow).unwrap();
    let mut out: [Vec<(f64, String)>; 3] = Default::default();
    for ids in mzdraw::layout::pieces(d) {
        let atoms: Vec<_> = d.atoms.iter().filter(|a| ids.contains(&a.id)).collect();
        let x = atoms.iter().map(|a| a.x).sum::<f64>() / atoms.len() as f64;
        let mut part = d.clone();
        part.atoms.retain(|a| ids.contains(&a.id));
        part.bonds.retain(|b| ids.contains(&b.a));
        part.objects.clear();
        let k = if x < arrow.x {
            0
        } else if x > arrow.x2 {
            1
        } else {
            2
        };
        out[k].push((x, canonical(&part.to_smiles().unwrap())));
    }
    out.map(|mut v| {
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        v.into_iter().map(|p| p.1).collect()
    })
}

fn want(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| canonical(s)).collect()
}

#[test]
fn a_reaction_file_is_drawn_with_its_roles() {
    let (reactants, products, agents) = (["CC(=O)O", "OCC"], ["CC(=O)OCC", "O"], ["OS(=O)(=O)O"]);
    let (d, info) = import(&rxn(&reactants, &products, &agents)).unwrap();
    assert_eq!((info.format, info.records), ("rxn", 1));
    assert!(info.notes().is_empty(), "{:?}", info.notes());
    assert_eq!(
        sides(&d),
        [want(&reactants), want(&products), want(&agents)]
    );
    // one arrow, a plus between the two reactants and between the two products, all on one line; the agent above the arrow
    let kinds: Vec<ObjKind> = d.objects.iter().map(|o| o.t).collect();
    assert_eq!(kinds.iter().filter(|k| **k == ObjKind::Arrow).count(), 1);
    assert_eq!(kinds.iter().filter(|k| **k == ObjKind::Plus).count(), 2);
    let arrow = d.objects.iter().find(|o| o.t == ObjKind::Arrow).unwrap();
    assert!(arrow.x < arrow.x2 && arrow.y == arrow.y2);
    let plus: Vec<_> = d.objects.iter().filter(|o| o.t == ObjKind::Plus).collect();
    assert!(plus[0].x < arrow.x && plus[1].x > arrow.x2);
    assert!(plus.iter().all(|p| (p.y - arrow.y).abs() < 1e-3));
    let sulfur = d.atoms.iter().find(|a| a.el == "S").unwrap();
    assert!(sulfur.y > arrow.y && arrow.x < sulfur.x && sulfur.x < arrow.x2);
    // nothing sits on anything: every atom keeps a distance from the atoms of the other structures
    let pieces = mzdraw::layout::pieces(&d);
    for (i, p) in pieces.iter().enumerate() {
        for q in &pieces[i + 1..] {
            for a in d.atoms.iter().filter(|a| p.contains(&a.id)) {
                for b in d.atoms.iter().filter(|b| q.contains(&b.id)) {
                    assert!((a.x - b.x).hypot(a.y - b.y) > 0.9, "{} {}", a.id, b.id);
                }
            }
        }
    }
    assert_eq!(
        Document::from_json(&d.to_json()).unwrap(),
        d,
        "a valid document"
    );
    // no reactant, or no product, is a reaction too; the counts must be met
    let (d, _) = import(&rxn(&[], &["CCO"], &[])).unwrap();
    assert_eq!(sides(&d), [vec![], want(&["CCO"]), vec![]]);
    let short = rxn(&["CC", "CO"], &["CCO"], &[]).replace("  2  1", "  2  3");
    assert_eq!(import(&short).unwrap_err(), "error.draw2.rxn");
    // in the editor a reaction file is one undo step
    let mut e = Editor::default();
    e.import(&rxn(&reactants, &products, &agents)).unwrap();
    assert_eq!(e.doc().objects.len(), 3);
    e.undo();
    assert!(e.doc().is_empty());
}

#[test]
fn v3000_reactions_and_rd_files_are_read() {
    let v3 = format!(
        "$RXN V3000\n\n      synthetic\n\nM  V30 COUNTS 2 1 1\nM  V30 BEGIN REACTANT\n{}{}M  V30 END REACTANT\nM  V30 BEGIN PRODUCT\n{}M  V30 END PRODUCT\nM  V30 BEGIN AGENT\n{}M  V30 END AGENT\nM  END\n",
        ctab("N[C@@H](C)C(=O)O"),
        ctab("CO"),
        ctab("N[C@@H](C)C(=O)OC"),
        ctab("Cl")
    );
    let (d, info) = import(&v3).unwrap();
    assert_eq!(info.format, "rxn");
    assert_eq!(
        sides(&d),
        [
            want(&["N[C@@H](C)C(=O)O", "CO"]),
            want(&["N[C@@H](C)C(=O)OC"]),
            want(&["Cl"])
        ]
    );
    // an RD file: two reactions and a molecule, with data that is not structure; one record under the other
    let rd = format!(
        "$RDFILE 1\n$DATM 01/01/26 00:00\n$RFMT $RIREG 1\n{}$DTYPE yield\n$DATUM 87\n$RFMT $RIREG 2\n{}$DTYPE note\n$DATUM two lines\nM  END is not a record here\n$MFMT $MIREG 3\n{}$DTYPE name\n$DATUM synthetic\n",
        rxn(&["CCO"], &["CC=O"], &[]),
        v3,
        mol("c1ccccc1")
    );
    let (d, info) = import(&rd).unwrap();
    assert_eq!(
        (info.records, info.notes()),
        (3, vec![("draw2.import.records", 3)])
    );
    assert_eq!(
        d.objects.iter().filter(|o| o.t == ObjKind::Arrow).count(),
        2
    );
    let arrows: Vec<_> = d.objects.iter().filter(|o| o.t == ObjKind::Arrow).collect();
    assert!(
        arrows[1].y < arrows[0].y - 2.0,
        "the second reaction is under the first"
    );
    let ring_low = d
        .atoms
        .iter()
        .rev()
        .take(6)
        .map(|a| a.y)
        .fold(f64::MIN, f64::max);
    assert!(
        ring_low < arrows[1].y,
        "the molecule is under the second reaction"
    );
    assert_eq!(d.atoms.len(), 3 + 3 + 6 + 2 + 7 + 1 + 6);
    assert_eq!(Document::from_json(&d.to_json()).unwrap(), d);
    // Windows line ends
    let (w, _) = import(&rd.replace('\n', "\r\n")).unwrap();
    assert_eq!(w.to_smiles().unwrap(), d.to_smiles().unwrap());
    // an RD file without records, and a reaction with a molecule whose bond names an atom that is not there, are refused
    assert_eq!(
        import("$RDFILE 1\n$DATM x\n").unwrap_err(),
        "error.draw2.rxn"
    );
    assert_eq!(
        import(&rxn(&["CC"], &["CC"], &[]).replace("  1  2  1  0", "  1  9  1  0")).unwrap_err(),
        "error.draw2.rxn"
    );
}

#[test]
fn a_drawing_with_one_arrow_goes_out_as_a_reaction_file() {
    // drawn by the reader, then moved about: roles are where the structures stand along the arrow
    let (reactants, products, agents) = (["CC(=O)O", "OCC"], ["CC(=O)OCC", "O"], ["OS(=O)(=O)O"]);
    let (d, _) = import(&rxn(&reactants, &products, &agents)).unwrap();
    let out = d.to_rxn().unwrap();
    assert!(
        out.starts_with("$RXN\n\n  mzDraw\n\n  2  2  1\n$MOL\n"),
        "{out}"
    );
    assert_eq!(out.matches("$MOL\n").count(), 5);
    assert_eq!(out.matches("M  END\n").count(), 5);
    // read back: the same reaction
    let (back, _) = import(&out).unwrap();
    assert_eq!(
        sides(&back),
        [want(&reactants), want(&products), want(&agents)]
    );
    // written, read and written again: the same counts and the same molecules in the same order (the coordinates are scaled
    // to the bond length at each reading, so the last digit may move)
    let again = back.to_rxn().unwrap();
    let mols = |t: &str| -> Vec<String> {
        let all = t.split("$MOL\n").skip(1);
        all.map(|m| {
            Mol::from_molfile(m)
                .unwrap()
                .canonical()
                .to_smiles()
                .unwrap()
        })
        .collect()
    };
    assert_eq!(
        (again.lines().nth(4), mols(&again)),
        (out.lines().nth(4), mols(&out))
    );
    // a slanted arrow: the roles follow the arrow, not the x axis (the product is up and to the left of the reactant)
    let slanted = Document::from_json(
        r#"{"format":"mzdraw/1","next":9,"atoms":[{"id":1,"el":"C","x":4.0,"y":0.0},{"id":2,"el":"O","x":5.0,"y":0.0},{"id":3,"el":"C","x":0.0,"y":4.0},{"id":4,"el":"N","x":1.0,"y":4.0},{"id":5,"el":"Cl","x":3.5,"y":3.0}],"bonds":[{"id":6,"a":1,"b":2,"o":1},{"id":7,"a":3,"b":4,"o":1}],"objects":[{"id":8,"t":"arrow","x":3.5,"y":1.0,"x2":1.5,"y2":3.0}]}"#,
    )
    .unwrap();
    let out = slanted.to_rxn().unwrap();
    assert!(out.contains("\n  1  1  1\n"), "{out}");
    let mols: Vec<&str> = out.split("$MOL\n").skip(1).collect();
    let smiles = |m: &str| Mol::from_molfile(m).unwrap().to_smiles().unwrap();
    assert_eq!(
        [smiles(mols[0]), smiles(mols[1]), smiles(mols[2])],
        ["[CH3][OH]", "[CH3][NH2]", "[ClH]"]
    );
    // without agents the counts line is the standard one
    let (plain, _) = import(&rxn(&["CC"], &["C=C"], &[])).unwrap();
    assert!(plain.to_rxn().unwrap().contains("\n  1  1\n$MOL"));
    // no arrow, or two, is not a reaction: said with a key; through the editor door too
    assert_eq!(drawn("CCO").to_rxn().unwrap_err(), "error.draw2.rxnArrow");
    let mut two = slanted.clone();
    let mut second = two.objects[0].clone();
    second.id = 9;
    two.next = 10;
    two.objects.push(second);
    assert_eq!(two.to_rxn().unwrap_err(), "error.draw2.rxnArrow");
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    assert_eq!(e.save("rxn").unwrap_err(), "error.draw2.rxnArrow");
}

#[test]
fn a_damaged_reaction_file_is_refused_or_read_but_never_a_panic() {
    let good = rxn(&["N[C@@H](C)C(=O)O", "CO"], &["N[C@@H](C)C(=O)OC"], &["Cl"]);
    let rd = format!(
        "$RDFILE 1\n$DATM x\n$RFMT\n{good}$DTYPE a\n$DATUM b\n$MFMT\n{}",
        mol("CC")
    );
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = |n: usize| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % n as u64) as usize
    };
    let (mut read, mut refused) = (0, 0);
    for text in [good.as_str(), rd.as_str()] {
        for _ in 0..2500 {
            let mut b = text.as_bytes().to_vec();
            for _ in 0..1 + next(3) {
                if b.len() < 6 {
                    break;
                }
                let at = 4 + next(b.len() - 4);
                match next(4) {
                    0 => b[at] = b" 9$MRV0-\n\xC3"[next(10)],
                    1 => {
                        b.remove(at);
                    }
                    2 => b.insert(at, b"\n 9$"[next(4)]),
                    _ => b.truncate(at),
                }
            }
            match import(&String::from_utf8_lossy(&b)) {
                Ok((doc, _)) => {
                    assert_eq!(Document::from_json(&doc.to_json()).as_ref(), Ok(&doc));
                    read += 1;
                }
                Err(_) => refused += 1,
            }
        }
    }
    assert!(
        read > 100 && refused > 500,
        "{read} read, {refused} refused"
    );
}
