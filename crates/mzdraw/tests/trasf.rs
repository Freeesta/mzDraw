//! A drawn parent → arrow → product saved as `trasf/1` (plan-12): three synthetic transformations read back with `mzlab-mol`, the
//! refusals with their keys, the way back. Synthetic structures only.

use mzdraw::io::import;
use mzdraw::{Document, Editor};
use mzlab_mol::{Mol, Transformation};

fn atom(x: f64, el: &str, map: u16) -> String {
    format!(
        "{x:>10.4}{:>10.4}{:>10.4} {el:<3} 0  0  0  0  0  0  0  0  0{map:>3}  0  0\n",
        0.0, 0.0
    )
}

/// One molfile of a reaction file: atoms (element, map) on a line with bonds (a, b, order), from 1.
fn mol(atoms: &[(&str, u16)], bonds: &[(usize, usize, u8)]) -> String {
    let mut t = format!(
        "$MOL\nsynthetic\n  mzdraw\n\n{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
        atoms.len(),
        bonds.len()
    );
    for (i, (el, m)) in atoms.iter().enumerate() {
        t += &atom(i as f64 * 1.5, el, *m);
    }
    for (a, b, o) in bonds {
        t += &format!("{a:>3}{b:>3}{o:>3}  0\n");
    }
    t + "M  END\n"
}

type Side<'a> = (&'a [(&'a str, u16)], &'a [(usize, usize, u8)]);

fn rxn(parents: &[Side], products: &[Side]) -> String {
    let all = |v: &[Side]| v.iter().map(|(a, b)| mol(a, b)).collect::<String>();
    format!(
        "$RXN\n\n  synthetic\n\n{:>3}{:>3}\n{}{}",
        parents.len(),
        products.len(),
        all(parents),
        all(products)
    )
}

fn doc(text: &str) -> Document {
    import(text).expect("synthetic reaction").0
}

/// Propane -> propan-1-ol (hydroxylation, the oxygen is added).
fn hydroxylation() -> String {
    rxn(
        &[(&[("C", 1), ("C", 2), ("C", 3)], &[(1, 2, 1), (2, 3, 1)])],
        &[(
            &[("C", 1), ("C", 2), ("C", 3), ("O", 0)],
            &[(1, 2, 1), (2, 3, 1), (3, 4, 1)],
        )],
    )
}

/// N-methylethylamine -> ethylamine (N-demethylation, the methyl is lost).
fn demethylation() -> String {
    rxn(
        &[(
            &[("C", 1), ("C", 2), ("N", 3), ("C", 0)],
            &[(1, 2, 1), (2, 3, 1), (3, 4, 1)],
        )],
        &[(&[("C", 1), ("C", 2), ("N", 3)], &[(1, 2, 1), (2, 3, 1)])],
    )
}

/// Methyl acetate -> acetic acid (ester hydrolysis, the methanol is lost).
fn hydrolysis() -> String {
    rxn(
        &[(
            &[("C", 1), ("C", 2), ("O", 3), ("O", 4), ("C", 0)],
            &[(1, 2, 1), (2, 3, 2), (2, 4, 1), (4, 5, 1)],
        )],
        &[(
            &[("C", 1), ("C", 2), ("O", 3), ("O", 4)],
            &[(1, 2, 1), (2, 3, 2), (2, 4, 1)],
        )],
    )
}

/// The record and the two records of a saved file, checked against each other by `mzlab-mol`.
fn read_back(file: &str) -> (Transformation, Mol, Mol) {
    let lines: Vec<&str> = file.lines().collect();
    assert_eq!(lines.len(), 3);
    let t = Transformation::from_json(lines[0]).expect("mzlab-mol reads the record");
    let (p, q) = (
        Mol::from_json(lines[1]).unwrap(),
        Mol::from_json(lines[2]).unwrap(),
    );
    t.validate_against(&p, Some(&q))
        .expect("the record fits its two records");
    (t, p, q)
}

#[test]
fn three_transformations_read_back_with_their_delta() {
    for (text, delta, lost, added) in [
        (hydroxylation(), "+O", 0, 1),
        (demethylation(), "-CH2", 1, 0),
        (hydrolysis(), "-CH2", 1, 0),
    ] {
        let file = doc(&text).to_trasf().expect("a complete map");
        let (t, p, q) = read_back(&file);
        assert_eq!(t.delta, delta);
        assert!(t.is_certain() && t.sites.is_empty() && t.region.is_none());
        assert_eq!(t.map.iter().filter(|m| m.1.is_none()).count(), lost);
        assert_eq!(t.added.len(), added);
        assert_eq!(t.map.len(), p.atoms.len());
        assert_eq!(p.atoms.len() - lost + added, q.atoms.len());
        // the same bytes every time
        assert_eq!(file, doc(&text).to_trasf().unwrap());
    }
}

#[test]
fn the_centre_is_where_the_bonds_change() {
    let (t, p, _) = read_back(&doc(&hydroxylation()).to_trasf().unwrap());
    // the new oxygen is bonded to the end carbon: that carbon is the centre, on the canonical indices of the parent
    assert_eq!(t.centre.len(), 1);
    let c = t.centre[0] as usize;
    assert_eq!(p.atoms[c].el, "C");
    assert_eq!(
        p.bonds
            .iter()
            .filter(|b| b.a as usize == c || b.b as usize == c)
            .count(),
        1,
        "a terminal carbon"
    );
}

#[test]
fn what_is_not_one_arrow_with_one_structure_each_side_is_refused() {
    let key = |d: &Document| d.to_trasf().unwrap_err().key;
    // no arrow: the two structures as one drawing without it
    let mut d = doc(&hydroxylation());
    d.objects.clear();
    assert_eq!(key(&d), "error.draw2.trasfShape");
    // two products
    let two = rxn(
        &[(&[("C", 1), ("O", 2)], &[(1, 2, 1)])],
        &[(&[("C", 1)], &[]), (&[("O", 2)], &[])],
    );
    assert_eq!(key(&doc(&two)), "error.draw2.trasfShape");
    // an empty drawing
    assert_eq!(key(&Document::default()), "error.draw2.trasfShape");
}

#[test]
fn an_incomplete_map_names_its_atoms() {
    // no map at all: every atom is named, the parent's first
    let none = rxn(
        &[(&[("C", 0), ("C", 0)], &[(1, 2, 1)])],
        &[(&[("C", 0), ("C", 0), ("O", 0)], &[(1, 2, 1), (2, 3, 1)])],
    );
    let d = doc(&none);
    let e = d.to_trasf().unwrap_err();
    assert_eq!(e.key, "error.draw2.trasfMap");
    assert_eq!(e.atoms.len(), d.atoms.len());
    // a number only on the parent
    let orphan = rxn(
        &[(&[("C", 1), ("C", 2)], &[(1, 2, 1)])],
        &[(&[("C", 1), ("C", 0), ("O", 0)], &[(1, 2, 1), (2, 3, 1)])],
    );
    let d = doc(&orphan);
    let e = d.to_trasf().unwrap_err();
    assert_eq!(e.key, "error.draw2.trasfMap");
    assert_eq!(e.atoms, vec![d.atoms[1].id]);
    // the smaller structure is not covered: the second carbon of the parent has no partner
    let part = rxn(
        &[(&[("C", 1), ("C", 0)], &[(1, 2, 1)])],
        &[(&[("C", 1), ("C", 0), ("O", 0)], &[(1, 2, 1), (2, 3, 1)])],
    );
    let d = doc(&part);
    let e = d.to_trasf().unwrap_err();
    assert_eq!(e.key, "error.draw2.trasfMap");
    assert_eq!(e.atoms.len(), 3);
    // the editor gives the same key and the same atoms
    let mut ed = Editor::default();
    ed.import(&part).unwrap();
    assert_eq!(ed.save("trasf"), Err("error.draw2.trasfMap"));
    assert_eq!(ed.trasf_problem().unwrap().1, e.atoms);
}

#[test]
fn a_number_twice_on_one_side_is_refused() {
    let twice = rxn(
        &[(&[("C", 1), ("C", 1)], &[(1, 2, 1)])],
        &[(&[("C", 1), ("C", 2)], &[(1, 2, 1)])],
    );
    assert_eq!(
        doc(&twice).to_trasf().unwrap_err().key,
        "error.draw2.trasfMap"
    );
}

#[test]
fn an_element_may_change_at_the_centre() {
    // chloroethane -> ethanol: the chlorine is mapped to the oxygen
    let sub = rxn(
        &[(&[("C", 1), ("C", 2), ("Cl", 3)], &[(1, 2, 1), (2, 3, 1)])],
        &[(&[("C", 1), ("C", 2), ("O", 3)], &[(1, 2, 1), (2, 3, 1)])],
    );
    let (t, _, _) = read_back(&doc(&sub).to_trasf().unwrap());
    assert_eq!(t.delta, "+HO-Cl");
    assert_eq!(t.centre.len(), 1);
}

#[test]
fn the_file_opens_as_parent_arrow_product_with_its_maps() {
    for text in [hydroxylation(), demethylation(), hydrolysis()] {
        let file = doc(&text).to_trasf().unwrap();
        let (d, info) = import(&file).expect("a trasf file opens");
        assert_eq!(info.format, "trasf");
        assert_eq!(
            d.objects
                .iter()
                .filter(|o| o.t == mzdraw::ObjKind::Arrow)
                .count(),
            1
        );
        let (t, p, q) = read_back(&file);
        assert_eq!(d.atoms.len(), p.atoms.len() + q.atoms.len());
        assert_eq!(
            d.mapped(),
            2 * t.map.iter().filter(|m| m.1.is_some()).count()
        );
        // saving what was opened gives the same record (the opened drawing has its own layout, so the coordinates of the two records may differ)
        let again = d.to_trasf().unwrap();
        assert_eq!(again.lines().next(), file.lines().next());
    }
}

#[test]
fn the_records_beside_the_record_come_in_any_order() {
    let file = doc(&hydrolysis()).to_trasf().unwrap();
    let l: Vec<&str> = file.lines().collect();
    let shuffled = format!("{}\n{}\n{}\n", l[2], l[0], l[1]);
    assert_eq!(
        import(&shuffled)
            .unwrap()
            .0
            .to_trasf()
            .unwrap()
            .lines()
            .next(),
        file.lines().next()
    );
    // a missing record, a second record, a record that is not the one named
    assert!(import(&format!("{}\n{}\n", l[0], l[1])).is_err());
    assert!(import(&format!("{}\n{}\n{}\n", l[0], l[1], l[1])).is_err());
    assert!(import(&format!("{}\n{}\n{}\n{}\n", l[0], l[0], l[1], l[2])).is_err());
}
