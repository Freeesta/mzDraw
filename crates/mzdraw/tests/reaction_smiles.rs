//! Reaction SMILES and dative bonds in SMILES (rdkit-compare 7, study F4 and F5): `reactants>agents>products` is drawn as a reaction
//! (reactants with plus signs, the arrow, the agents above it, the products), and `->` / `<-` are bonds of the kind `dative` from the
//! donor to the acceptor, which take no part in the valence. Synthetic structures only.

use mzdraw::{Document, Editor, ObjKind};
use serde_json::{json, Value};

fn import(text: &str) -> Result<Document, &'static str> {
    let mut e = Editor::default();
    e.import(text)?;
    Ok(e.doc().clone())
}

/// The roles a reaction file gives the structures of a drawing: the SMILES of its reactants, products and agents.
fn roles(d: &Document) -> (Vec<String>, Vec<String>, Vec<String>) {
    let rxn = d.to_rxn().unwrap();
    let back = import(&rxn).unwrap();
    // each molecule of the file, by its role, as the SMILES the drawing writes for it
    let text = |part: &str| -> Vec<String> {
        let blocks: Vec<&str> = part.split("$MOL\n").skip(1).collect();
        blocks
            .iter()
            .map(|m| import(m).unwrap().to_smiles().unwrap())
            .collect()
    };
    let counts: Vec<usize> = rxn
        .lines()
        .nth(4)
        .unwrap()
        .split_whitespace()
        .map(|n| n.parse().unwrap())
        .collect();
    let all = text(&rxn);
    let (r, rest) = all.split_at(counts[0]);
    let (p, a) = rest.split_at(counts[1]);
    assert!(back.objects.iter().any(|o| o.t == ObjKind::Arrow));
    (r.to_vec(), p.to_vec(), a.to_vec())
}

#[test]
fn a_reaction_smiles_is_drawn_as_a_reaction_with_its_roles() {
    let d = import("CC(=O)O.OCC>[H+]>CC(=O)OCC.O esterification").unwrap();
    let arrows = d.objects.iter().filter(|o| o.t == ObjKind::Arrow).count();
    let pluses = d.objects.iter().filter(|o| o.t == ObjKind::Plus).count();
    // one arrow; a plus sign between the two reactants and between the two products (none between agents)
    assert_eq!((arrows, pluses), (1, 2));
    let (r, p, a) = roles(&d);
    assert_eq!(r, ["CC(=O)O", "OCC"]);
    assert_eq!(p, ["CC(=O)OCC", "O"]);
    assert_eq!(a, ["[H+]"]);
    // the name of the line is the title of the reaction's first structure
    assert_eq!(d.title(), "esterification");
    // no agents, and one side empty
    let (r, p, a) = roles(&import("CC=C>>CC(Br)CBr").unwrap());
    assert_eq!((r.len(), p.len(), a.len()), (1, 1, 0));
    let d = import("CCO>>").unwrap();
    assert_eq!(
        d.objects.iter().filter(|o| o.t == ObjKind::Arrow).count(),
        1
    );
    // atom maps of a mapped reaction are kept
    let d = import("[CH3:1][OH:2]>>[CH2:1]=[O:2]").unwrap();
    let mut maps: Vec<u16> = d.atoms.iter().map(|a| a.map).filter(|m| *m > 0).collect();
    maps.sort();
    assert_eq!(maps, [1, 1, 2, 2]);
}

#[test]
fn a_dative_bond_points_from_the_donor_and_takes_no_part_in_the_valence() {
    // the study's case: two ammonia donors on palladium
    let d = import("[NH3]->[Pd](Cl)(Cl)<-[NH3]").unwrap();
    let datives: Vec<(String, String)> = (d.bonds.iter())
        .filter(|b| b.kind() == "dative")
        .map(|b| {
            (
                d.atom(b.a).unwrap().el.clone(),
                d.atom(b.b).unwrap().el.clone(),
            )
        })
        .collect();
    assert_eq!(
        datives,
        [("N".into(), "Pd".into()), ("N".into(), "Pd".into())]
    );
    // an organic-subset donor keeps its hydrogens: N->[Cu+2] is ammonia on copper, written as such
    let d = import("N->[Cu+2]<-N").unwrap();
    assert_eq!(d.bonds.iter().filter(|b| b.kind() == "dative").count(), 2);
    assert_eq!(d.to_smiles().unwrap(), "[NH3][Cu+2][NH3]");
    // `<-` points from the atom after it
    let d = import("[Fe]<-[C-]#[O+]").unwrap();
    let b = d.bonds.iter().find(|b| b.kind() == "dative").unwrap();
    assert_eq!(
        (
            d.atom(b.a).unwrap().el.as_str(),
            d.atom(b.b).unwrap().el.as_str()
        ),
        ("C", "Fe")
    );
    // a dative bond inside a reaction
    let d = import("[Cu+2].N>>N->[Cu+2]").unwrap();
    assert_eq!(d.bonds.iter().filter(|b| b.kind() == "dative").count(), 1);
    assert_eq!(
        d.objects.iter().filter(|o| o.t == ObjKind::Arrow).count(),
        1
    );
}

#[test]
fn a_line_that_is_neither_is_refused_and_the_others_of_the_file_are_read() {
    for bad in [
        "CC>CO", "C>C>C>C", "N->1CC1", "N->", "CC>>C(", ">>", "N->(C)C",
    ] {
        assert_eq!(import(bad).unwrap_err(), "error.draw2.smiles", "{bad}");
    }
    // in a file of SMILES lines, the broken line is named and the others are drawn
    let r = json!({"import": "CCO\nCC>CO\nN->[Cu+2]", "report": true});
    let v: Value = serde_json::from_str(&mzdraw::replay::replay(&r.to_string()).unwrap()).unwrap();
    let notes = v["imported"].as_array().unwrap();
    assert!(
        notes.contains(&json!({"key": "draw2.import.smilesLine", "n": 2})),
        "{v}"
    );
    assert!(
        notes.contains(&json!({"key": "draw2.import.records", "n": 2})),
        "{v}"
    );
}

#[test]
fn the_cli_and_the_page_draw_the_same_reaction() {
    let text = "CC(=O)O.OCC>[H+]>CC(=O)OCC.O";
    let mut e = Editor::default();
    e.import(text).unwrap();
    let page = e.save("mzdraw").unwrap();
    let cli =
        mzdraw::replay::replay(&json!({"import": text, "out": "mzdraw"}).to_string()).unwrap();
    assert_eq!(cli, page);
    // `scheme:row` lays it out again by the same rule of roles: reactants, products and agents stay what they are
    let laid = mzdraw::replay::replay(
        &json!({"import": text, "command": "scheme:row", "out": "mzdraw"}).to_string(),
    )
    .unwrap();
    assert_eq!(roles(&Document::from_json(&laid).unwrap()), roles(e.doc()));
}

#[test]
fn the_cxsmiles_tail_of_a_reaction_line_is_said_not_read() {
    // a molecule line reads its tail (an enhanced-stereo group is said as not kept); a reaction line does not read it and says so
    let say = |text: &str| -> Value {
        let r = json!({"import": text, "dry_run": true});
        let v: Value =
            serde_json::from_str(&mzdraw::replay::replay(&r.to_string()).unwrap()).unwrap();
        v["imported"].clone()
    };
    assert_eq!(
        say("C[C@H](O)F |&1:1|"),
        json!([{"key": "draw2.import.stereoGroups", "n": 1}])
    );
    assert_eq!(
        say("C[C@H](O)F>>CC |&1:1|"),
        json!([{"key": "draw2.import.reactionTail", "n": 1}])
    );
    // a name is not a tail; two reaction lines with tails are counted
    assert_eq!(say("C[C@H](O)F>>CC esterification"), Value::Null);
    let two = say("CC>>CO |&1:1|\nCN>>CO |$;;$|");
    assert!(
        two.as_array()
            .unwrap()
            .contains(&json!({"key": "draw2.import.reactionTail", "n": 2})),
        "{two}"
    );
    // the reaction is drawn all the same
    let d = import("C[C@H](O)F>>CC |&1:1|").unwrap();
    assert_eq!(
        d.objects.iter().filter(|o| o.t == ObjKind::Arrow).count(),
        1
    );
}

#[test]
fn an_arrow_in_a_cxsmiles_label_is_not_a_dative_bond() {
    // the tail is not the SMILES: a label holding `->` is read as before, on a molecule line and on a reaction line
    let d = import("CC |$a->b;;$|").unwrap();
    assert_eq!(d.atoms.len(), 2);
    assert!(d.bonds.iter().all(|b| b.kind() != "dative"));
    let d = import("CC>>CO |$a->b;;;;$|").unwrap();
    assert_eq!(
        d.objects.iter().filter(|o| o.t == ObjKind::Arrow).count(),
        1
    );
    // a dative bond with a tail after it still reads its dative bond
    let d = import("N->[Cu+2] |$a->b;$|").unwrap();
    assert_eq!(d.bonds.iter().filter(|b| b.kind() == "dative").count(), 1);
}
