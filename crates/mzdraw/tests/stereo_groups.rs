//! Enhanced-stereo groups kept on their stereocentres (rdkit-compare 3 part 2, the staged first step of arch-std-12/19/20): the atom
//! field `stg`, the operation `stereo.group` with its exact inverse, read from and written to CXSMILES and V3000, drawn next to their
//! atoms, said lost where a format cannot hold them; no group is ever assumed on read (E20). Synthetic structures only.

use mzdraw::{Document, Editor, Op, Session};
use serde_json::{json, Value};

fn import(text: &str) -> Document {
    let mut e = Editor::default();
    e.import(text).unwrap();
    e.doc().clone()
}

fn groups(d: &Document) -> Vec<(u32, String)> {
    (d.atoms.iter())
        .filter(|a| !a.stg().is_empty())
        .map(|a| (a.id, a.stg()))
        .collect()
}

fn op(v: Value) -> Op {
    Op::parse(&v).unwrap()
}

/// Two stereocentres (atoms 2 and 4) drawn with wedges, no group.
const TWO: &str = "C[C@H](O)[C@@H](F)Cl";

#[test]
fn a_group_goes_on_stereocentres_and_one_undo_gives_the_identical_document() {
    let d = import(TWO);
    assert_eq!(
        d.stereocentres()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        [2, 4].into()
    );
    let mut s = Session::new(d.clone());
    let before = s.doc.to_json();
    s.run(&[op(
        json!({"op": "stereo.group", "atoms": [2, 4], "g": "and1"}),
    )])
    .unwrap();
    assert_eq!(groups(&s.doc), [(2, "and1".into()), (4, "and1".into())]);
    // changed for one atom, then taken away with `none`
    // (or2 is the only `or` group: it is numbered or1 at the end of the patch)
    s.run(&[op(json!({"op": "stereo.group", "atoms": [4], "g": "or2"}))])
        .unwrap();
    assert_eq!(groups(&s.doc), [(2, "and1".into()), (4, "or1".into())]);
    s.run(&[op(
        json!({"op": "stereo.group", "atoms": [2, 4], "g": "none"}),
    )])
    .unwrap();
    assert!(groups(&s.doc).is_empty());
    // each undo gives the document before it, byte for byte
    s.undo();
    assert_eq!(groups(&s.doc), [(2, "and1".into()), (4, "or1".into())]);
    s.undo();
    s.undo();
    assert_eq!(s.doc.to_json(), before);
    // the log writes the operation back as it was read
    let text = mzdraw::ops::log_text(&[op(
        json!({"op": "stereo.group", "atoms": [2, 4], "g": "abs"}),
    )]);
    assert_eq!(text, r#"[{"op":"stereo.group","atoms":[2,4],"g":"abs"}]"#);
}

#[test]
fn a_group_that_is_none_or_an_atom_that_is_no_stereocentre_is_refused_and_changes_nothing() {
    let d = import(TWO);
    let mut s = Session::new(d.clone());
    for (v, key, id) in [
        (
            json!({"op": "stereo.group", "atoms": [1], "g": "and1"}),
            "error.draw2.stereoGroup",
            1,
        ),
        (
            json!({"op": "stereo.group", "atoms": [2, 3], "g": "and1"}),
            "error.draw2.stereoGroup",
            3,
        ),
        (
            json!({"op": "stereo.group", "atoms": [2], "g": "and0"}),
            "error.draw2.stereoGroup",
            2,
        ),
        (
            json!({"op": "stereo.group", "atoms": [2], "g": "and2001"}),
            "error.draw2.stereoGroup",
            2,
        ),
        (
            json!({"op": "stereo.group", "atoms": [2], "g": "&1"}),
            "error.draw2.stereoGroup",
            2,
        ),
        (
            json!({"op": "stereo.group", "atoms": [2], "g": ""}),
            "error.draw2.stereoGroup",
            2,
        ),
        (
            json!({"op": "stereo.group", "atoms": [], "g": "abs"}),
            "error.draw2.stereoGroup",
            0,
        ),
        (
            json!({"op": "stereo.group", "atoms": [99], "g": "abs"}),
            "error.draw2.noAtom",
            99,
        ),
    ] {
        let e = s.run(&[op(v.clone())]).unwrap_err();
        assert_eq!((e.key, e.id), (key, id), "{v}");
        assert_eq!(s.doc, d, "{v}: nothing changed");
    }
}

#[test]
fn cxsmiles_and_v3000_groups_are_read_and_written_back() {
    // CXSMILES: an `and` and an `or` group, read on their atoms and written back in the tail
    let d = import("C[C@H](O)[C@@H](F)Cl |&1:1,o1:3|");
    assert_eq!(groups(&d), [(2, "and1".into()), (4, "or1".into())]);
    assert_eq!(d.to_smiles().unwrap(), "C[C@H](O)[C@@H](F)Cl |&1:1,o1:3|");
    // V3000: the collections, and read back to the same groups
    let v3 = d.to_molfile_v3000();
    assert!(
        v3.contains("MDLV30/STERAC1 ATOMS=(1 2)") && v3.contains("MDLV30/STEREL1 ATOMS=(1 4)"),
        "{v3}"
    );
    assert_eq!(groups(&import(&v3)), groups(&d));
    // `abs` with another group, in the file order: abs, and, or
    let mut s = Session::new(import(TWO));
    s.run(&[
        op(json!({"op": "stereo.group", "atoms": [4], "g": "abs"})),
        op(json!({"op": "stereo.group", "atoms": [2], "g": "and3"})),
    ])
    .unwrap();
    let v3 = s.doc.to_molfile_v3000();
    let abs = v3.find("STEABS").unwrap();
    assert!(abs < v3.find("STERAC1").unwrap(), "{v3}");
    // the number of a file is the rank of the drawing's number: and3 alone is written &1 and read back and1
    assert_eq!(
        s.doc.to_smiles().unwrap(),
        "C[C@H](O)[C@@H](F)Cl |a:3,&1:1|"
    );
    assert_eq!(
        groups(&import(&s.doc.to_smiles().unwrap())),
        [(2, "and1".into()), (4, "abs".into())]
    );
}

#[test]
fn no_group_is_assumed_on_read() {
    // E20: a wedged centre outside every collection, and a V2000 file (a chiral flag, no collection), have no group
    let d = import("C[C@H](O)[C@@H](F)Cl |&1:1|");
    assert_eq!(
        groups(&d),
        [(2, "and1".into())],
        "atom 4 is wedged and in no group"
    );
    let v2 = import(TWO).to_molfile();
    let chiral = v2.replacen("  0  0999 V2000", "  1  0999 V2000", 1);
    for text in [v2, chiral] {
        assert!(groups(&import(&text)).is_empty(), "{text}");
    }
}

#[test]
fn the_labels_are_drawn_next_to_their_atoms_and_abs_only_beside_another_kind() {
    let svg = |d: &Document| mzdraw::svg::to_svg(d);
    let d = import("C[C@H](O)[C@@H](F)Cl |&1:1,o1:3|");
    let s = svg(&d);
    assert_eq!(
        (s.matches(">and1<").count(), s.matches(">or1<").count()),
        (1, 1)
    );
    // all absolute: the wedges say it, no label; with an `and` group beside it, `abs` is drawn
    let d = import("C[C@H](O)[C@@H](F)Cl |a:1,3|");
    assert_eq!(groups(&d).len(), 2);
    assert_eq!(svg(&d).matches(">abs<").count(), 0);
    let d = import("C[C@H](O)[C@@H](F)Cl |a:1,&1:3|");
    let s = svg(&d);
    assert_eq!(
        (s.matches(">abs<").count(), s.matches(">and1<").count()),
        (1, 1)
    );
    // none: no label
    assert!(!svg(&import(TWO)).contains(">abs<"));
}

#[test]
fn a_format_that_has_no_place_for_them_says_so_and_the_cli_gives_the_page_bytes() {
    let text = "C[C@H](O)[C@@H](F)Cl |&1:1,o1:3|";
    // V2000, SDF, KET, CDXML, CML: the groups are lost and the result says it (the loss manifest)
    for out in ["molfile", "sdf", "ket", "cdxml", "cml"] {
        let r = json!({"import": text, "out": format!("loss:{out}")});
        let v: Value =
            serde_json::from_str(&mzdraw::replay::replay(&r.to_string()).unwrap()).unwrap();
        let lost = v.to_string();
        assert!(lost.contains("atom.stereoGroup"), "{out}: {lost}");
    }
    for out in ["molfile-v3000", "smiles", "mzdraw"] {
        let r = json!({"import": text, "out": format!("loss:{out}")});
        let v: Value =
            serde_json::from_str(&mzdraw::replay::replay(&r.to_string()).unwrap()).unwrap();
        assert!(!v.to_string().contains("atom.stereoGroup"), "{out}: {v}");
    }
    // the page: an import then the operation; the log replays natively to the same bytes
    let mut e = Editor::default();
    e.import(TWO).unwrap();
    e.session
        .run(&[op(
            json!({"op": "stereo.group", "atoms": [2, 4], "g": "or1"}),
        )])
        .unwrap();
    let page = e.save("mzdraw").unwrap();
    let log: Value = serde_json::from_str(&e.session.log_json()).unwrap();
    assert_eq!(
        mzdraw::replay::replay(&json!({"log": log}).to_string()).unwrap(),
        page
    );
    let cli = json!({"import": TWO, "log": [{"op": "stereo.group", "atoms": [2, 4], "g": "or1"}], "out": "mzdraw"});
    assert_eq!(mzdraw::replay::replay(&cli.to_string()).unwrap(), page);
}

#[test]
fn a_patch_is_judged_at_its_end_numbers_are_compacted_and_a_lost_centre_leaves_its_group() {
    let d = import(TWO);
    // the numbers are literal while the patch runs, then each kind is numbered 1, 2, ... in its order: and3 + and2000 -> and1 + and2
    let mut s = Session::new(d.clone());
    s.run(&[
        op(json!({"op": "stereo.group", "atoms": [2], "g": "and3"})),
        op(json!({"op": "stereo.group", "atoms": [4], "g": "and2000"})),
    ])
    .unwrap();
    assert_eq!(groups(&s.doc), [(2, "and1".into()), (4, "and2".into())]);
    s.undo();
    assert_eq!(s.doc, d, "the compaction is in the inverse");
    // a word that is none: and2001, a leading zero
    for g in ["and2001", "or02"] {
        let e = s
            .run(&[op(json!({"op": "stereo.group", "atoms": [2], "g": g}))])
            .unwrap_err();
        assert_eq!(e.key, "error.draw2.stereoGroup", "{g}");
    }
    // validation on the final record: a group on an atom that becomes a centre later in the same patch is accepted; without the
    // wedge the patch fails and changes nothing
    let mut s = Session::new(d.clone());
    let wedge = |id: u32, w: u8| op(json!({"op": "bond.set", "id": id, "s": w}));
    let centre_bond = s
        .doc
        .bonds
        .iter()
        .find(|b| b.a == 2 && b.s != 0)
        .unwrap()
        .id;
    s.run(&[wedge(centre_bond, 0)]).unwrap();
    let flat = s.doc.clone();
    assert!(!flat.stereocentres().contains(&2));
    let e = s
        .run(&[op(json!({"op": "stereo.group", "atoms": [2], "g": "or1"}))])
        .unwrap_err();
    assert_eq!((e.key, e.id), ("error.draw2.stereoGroup", 2));
    assert_eq!(s.doc, flat);
    s.run(&[
        op(json!({"op": "stereo.group", "atoms": [2], "g": "or1"})),
        wedge(centre_bond, 1),
    ])
    .unwrap();
    assert_eq!(groups(&s.doc), [(2, "or1".into())]);
    // a bond change that makes it no centre: the atom leaves its group, and one undo gives it back
    let kept = s.doc.clone();
    s.run(&[wedge(centre_bond, 0)]).unwrap();
    assert!(groups(&s.doc).is_empty(), "no centre, no group");
    s.undo();
    assert_eq!(s.doc, kept);
    // `none` never fails on an existing atom, a centre or not
    s.run(&[op(
        json!({"op": "stereo.group", "atoms": [1, 2], "g": "none"}),
    )])
    .unwrap();
    assert!(groups(&s.doc).is_empty());
}

#[path = "support/timing.rs"]
mod timing;

#[test]
fn the_check_at_the_end_of_a_patch_costs_a_bounded_multiple_of_a_plain_edit() {
    // 25 copies of a structure with two wedged centres (some 200 atoms): the same move, on the drawing without groups (no check runs)
    // and on the drawing whose 50 centres all have a group (the check reads the centres of the whole drawing). The least CPU time of
    // runs taken in turns, no wall-clock bound
    let one = "C[C@H](O)[C@@H](F)CCC";
    let plain = import(&vec![one; 25].join("."));
    assert!(plain.atoms.len() >= 200);
    let mut s = Session::new(plain.clone());
    let centres: Vec<u32> = plain.stereocentres().into_iter().collect();
    assert_eq!(centres.len(), 50);
    s.run(&[op(
        json!({"op": "stereo.group", "atoms": centres, "g": "and1"}),
    )])
    .unwrap();
    let grouped = s.doc.clone();
    let step = [op(
        json!({"op": "move", "atoms": [1], "objects": [], "dx": 0.5, "dy": 0.0}),
    )];
    let edit = |d: &Document| {
        let mut s = Session::new(d.clone());
        s.run(&step).unwrap();
    };
    let (without, with) = timing::best_of_pair(3, 15, || edit(&plain), || edit(&grouped));
    let (a, b) = (without.as_nanos().max(1), with.as_nanos());
    assert!(b <= 10 * a, "a plain edit {a} ns, with groups {b} ns");
}

#[test]
fn a_word_with_a_sign_a_space_or_another_digit_is_no_group() {
    let mut s = Session::new(import(TWO));
    let d = s.doc.clone();
    // `parse` would read `+1`; only ASCII digits are a number of the formats
    for g in [
        "and+1",
        "or+1",
        "and 1",
        "or 1",
        "and1 ",
        "and-1",
        "and\u{661}",
        "or\u{ff11}",
        "and",
        "or",
    ] {
        assert!(!mzdraw::doc::stg_valid(g), "{g}");
        let e = s
            .run(&[op(json!({"op": "stereo.group", "atoms": [2], "g": g}))])
            .unwrap_err();
        assert_eq!((e.key, e.id), ("error.draw2.stereoGroup", 2), "{g}");
        assert_eq!(s.doc, d, "{g}: nothing changed");
    }
    for g in ["abs", "and1", "or2000", ""] {
        assert!(mzdraw::doc::stg_valid(g), "{g}");
    }
}

#[test]
fn a_group_dropped_on_read_leaves_no_gap_in_the_numbers() {
    // a V3000 file whose first `and` collection holds atom 1, no stereocentre: that group goes (the reader of `mzlab-mol` takes the
    // atom out of the collection; the drop of a group on an atom that is no centre of the drawing is `io::labelled`'s, tested there),
    // and the second is read and1 (no `and2` alone), written STERAC1
    let mut s = Session::new(import(TWO));
    s.run(&[
        op(json!({"op": "stereo.group", "atoms": [2], "g": "and1"})),
        op(json!({"op": "stereo.group", "atoms": [4], "g": "and2"})),
    ])
    .unwrap();
    let v3 = s.doc.to_molfile_v3000();
    assert!(
        v3.contains("STERAC1 ATOMS=(1 2)") && v3.contains("STERAC2 ATOMS=(1 4)"),
        "{v3}"
    );
    let text = v3.replace("STERAC1 ATOMS=(1 2)", "STERAC1 ATOMS=(1 1)");
    let d = import(&text);
    assert_eq!(groups(&d), [(4, "and1".into())]);
    let back = d.to_molfile_v3000();
    assert!(
        back.contains("STERAC1 ATOMS=(1 4)") && !back.contains("STERAC2"),
        "{back}"
    );
    assert_eq!(d.to_smiles().unwrap(), "C[C@H](O)[C@@H](F)Cl |&1:3|");
}

#[test]
fn only_the_pieces_a_patch_touches_are_judged_and_a_group_gone_anywhere_compacts_the_rest() {
    // two structures: atoms 2 and 4 (the first), 9 and 11 (the second) are centres
    let d = import(&format!("{TWO}.{TWO}"));
    let mut s = Session::new(d);
    let c: Vec<u32> = s.doc.stereocentres().into_iter().collect();
    assert_eq!(c.len(), 4);
    let second = *c.iter().max().unwrap();
    s.run(&[
        op(json!({"op": "stereo.group", "atoms": [2], "g": "and1"})),
        op(json!({"op": "stereo.group", "atoms": [second], "g": "and2"})),
    ])
    .unwrap();
    // an edit of the first structure that keeps its centre: the group of the second stays
    s.run(&[op(
        json!({"op": "move", "atoms": [1], "objects": [], "dx": 0.5, "dy": 0.0}),
    )])
    .unwrap();
    assert_eq!(
        groups(&s.doc),
        [(2, "and1".into()), (second, "and2".into())]
    );
    // `none` on the first: the second's and2 is numbered and1 (the numbers of the whole drawing are compacted), one undo gives it back
    let kept = s.doc.clone();
    s.run(&[op(json!({"op": "stereo.group", "atoms": [2], "g": "none"}))])
        .unwrap();
    assert_eq!(groups(&s.doc), [(second, "and1".into())]);
    s.undo();
    assert_eq!(s.doc, kept);
}

#[test]
fn a_subset_of_the_hunks_of_a_diff_keeps_a_group_with_its_wedge() {
    // the diff of a drawing to the same with a wedge added and a group set on that new centre: the group's operation and the wedge
    // touch the same atom, so they fall in one hunk, and no subset of the hunks applies the group without its wedge
    let a = import(&format!("{TWO}.CC"));
    let far = a.atoms.last().unwrap().id;
    let mut s = Session::new(a.clone());
    let wedged = (s.doc.bonds.iter())
        .find(|b| b.a == 2 && b.s != 0)
        .unwrap()
        .id;
    s.run(&[op(json!({"op": "bond.set", "id": wedged, "s": 0}))])
        .unwrap();
    let flat = s.doc.clone();
    s.run(&[
        op(json!({"op": "bond.set", "id": wedged, "s": 1})),
        op(json!({"op": "stereo.group", "atoms": [2], "g": "or1"})),
        op(json!({"op": "atom.set", "id": far, "el": "N"})),
    ])
    .unwrap();
    let b = s.doc.clone();
    let ops = mzdraw::diff::diff(&flat, &b);
    let hunks = mzdraw::diff::hunks(&flat, &ops);
    assert!(hunks.len() >= 2, "{hunks:?}");
    let with_group: Vec<_> = (hunks.iter())
        .filter(|h| h.ops.iter().any(|o| matches!(o, Op::StereoGroup { .. })))
        .collect();
    assert_eq!(with_group.len(), 1);
    assert!(
        (with_group[0].ops.iter()).any(|o| matches!(o, Op::BondSet { id, .. } if *id == wedged)),
        "{hunks:?}"
    );
    // each hunk alone applies; the group's hunk gives the centre and its group
    for h in &hunks {
        let mut t = Session::new(flat.clone());
        t.run(&h.ops).unwrap();
    }
    // the group's operation alone, without the wedge, fails whole (the rule of the end of a patch) and changes nothing
    let lone: Vec<Op> = (with_group[0].ops.iter())
        .filter(|o| matches!(o, Op::StereoGroup { .. }))
        .cloned()
        .collect();
    let mut t = Session::new(flat.clone());
    assert_eq!(t.run(&lone).unwrap_err().key, "error.draw2.stereoGroup");
    assert_eq!(t.doc, flat);
}
