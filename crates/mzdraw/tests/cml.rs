//! CML (molecular convention) in and out, MRV (Marvin documents) in. Synthetic structures only.

use mzdraw::io::import;
use mzdraw::xml;
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

#[test]
fn a_drawing_goes_out_as_cml_by_the_convention() {
    let mut d = drawn("N[C@@H](C)C(=O)[O-]");
    d.atoms[2].lab = "R2".into();
    let text = d.to_cml();
    let root = xml::parse(&text).unwrap();
    assert_eq!(
        (root.name.as_str(), root.attr("xmlns")),
        ("cml", Some("http://www.xml-cml.org/schema"))
    );
    assert_eq!(root.attr("convention"), Some("convention:molecular"));
    let molecule = root.each("molecule").next().unwrap();
    assert!(molecule.attr("id").is_some());
    let atoms: Vec<_> = molecule
        .each("atomArray")
        .next()
        .unwrap()
        .each("atom")
        .collect();
    let bonds: Vec<_> = molecule
        .each("bondArray")
        .next()
        .unwrap()
        .each("bond")
        .collect();
    assert_eq!((atoms.len(), bonds.len()), (6, 5));
    // every atom has an id that starts with a letter, an element and both coordinates; every bond two atoms and an order
    for a in &atoms {
        assert!(
            a.attr("id").unwrap().starts_with('a')
                && a.attr("x2").is_some()
                && a.attr("y2").is_some(),
            "{a:?}"
        );
    }
    for b in &bonds {
        let refs: Vec<&str> = b.attr("atomRefs2").unwrap().split(' ').collect();
        assert!(
            refs.len() == 2
                && refs[0] != refs[1]
                && refs
                    .iter()
                    .all(|r| atoms.iter().any(|a| a.attr("id") == Some(r)))
        );
        assert!(["S", "D", "T"].contains(&b.attr("order").unwrap()));
    }
    assert_eq!(
        (atoms[0].attr("elementType"), atoms[5].attr("formalCharge")),
        (Some("N"), Some("-1"))
    );
    assert_eq!(atoms[0].attr("formalCharge"), None);
    // the R group is the element R of the schema, with its label
    assert_eq!(atoms[2].attr("elementType"), Some("R"));
    assert_eq!(
        atoms[2].each("label").next().unwrap().attr("value"),
        Some("R2")
    );
    // one wedge: its `bondStereo` names the two atoms of its bond, the centre first
    let stereo: Vec<_> = bonds
        .iter()
        .filter_map(|b| b.each("bondStereo").next().map(|s| (*b, s)))
        .collect();
    assert_eq!(stereo.len(), 1);
    let (bond, mark) = stereo[0];
    assert!(["W", "H"].contains(&mark.text.as_str()));
    assert_eq!(mark.attr("atomRefs2"), bond.attr("atomRefs2"));
    assert!(mark.attr("atomRefs2").unwrap().starts_with("a2 "));
    assert_eq!(
        bonds
            .iter()
            .filter(|b| b.attr("order") == Some("D"))
            .count(),
        1
    );
    // nothing drawn: an empty `cml`
    assert!(xml::parse(&Document::default().to_cml())
        .unwrap()
        .children
        .is_empty());
}

#[test]
fn cml_comes_back_as_the_same_structures() {
    for s in [
        "CC(=O)O",
        "c1ccccc1",
        "N[C@@H](C)C(=O)O",
        "C[C@H]1CC[C@@H](O)CC1",
        "[NH4+].[Cl-]",
        "C[13CH3]",
        "[CH2]C",
        "CC[*:5].[*]CO",
    ] {
        let d = drawn(s);
        let (back, info) = import(&d.to_cml()).unwrap();
        assert_eq!(info.format, "cml");
        assert!(info.notes().is_empty(), "{s}: {:?}", info.notes());
        assert_eq!(back.to_smiles().unwrap(), d.to_smiles().unwrap(), "{s}");
        if !s.contains('*') {
            assert_eq!(canonical(&back.to_smiles().unwrap()), canonical(s), "{s}");
        }
    }
    // a wavy bond survives
    let mut d = drawn("CC(N)O");
    d.bonds[1].s = 3;
    let (back, _) = import(&d.to_cml()).unwrap();
    assert_eq!(back.bonds.iter().filter(|b| b.s == 3).count(), 1);
    // arrows and texts have no place in CML: the structures go out, the objects do not
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    e.set_tool("plus");
    e.pointer_down(40.0, 40.0, 0, 0);
    e.pointer_up(40.0, 40.0);
    assert_eq!(e.doc().objects.len(), 1);
    let (back, _) = import(&e.save("cml").unwrap()).unwrap();
    assert_eq!((back.atoms.len(), back.objects.len()), (3, 0));
}

/// A CML file as other programs write it (synthetic): namespace prefix, a bare-number order, an aromatic ring, a dummy atom,
/// a partial bond, 3D coordinates only, names and properties around the atoms, child molecules.
const FOREIGN_CML: &str = r#"<?xml version="1.0"?>
<cml:cml xmlns:cml="http://www.xml-cml.org/schema" convention="convention:molecular">
 <cml:molecule id="mix">
  <cml:molecule id="m1" count="1">
   <cml:name dictRef="x:name">synthetic &amp; aromatic</cml:name>
   <cml:formula concise="C 6 H 5 N 1"/>
   <cml:atomArray>
    <cml:atom id="a1" elementType="C" x2="0.0" y2="1.4"/><cml:atom id="a2" elementType="C" x2="1.21" y2="0.7"/><cml:atom id="a3" elementType="C" x2="1.21" y2="-0.7"/>
    <cml:atom id="a4" elementType="C" x2="0.0" y2="-1.4"/><cml:atom id="a5" elementType="C" x2="-1.21" y2="-0.7"/><cml:atom id="a6" elementType="C" x2="-1.21" y2="0.7"/>
    <cml:atom id="a7" elementType="N" x2="0.0" y2="2.8" formalCharge="1" hydrogenCount="3" isotopeNumber="15"/>
    <cml:atom id="a8" elementType="Du" x2="2.42" y2="1.4"/>
   </cml:atomArray>
   <cml:bondArray>
    <cml:bond id="b1" atomRefs2="a1 a2" order="A"/><cml:bond atomRefs2="a2 a3" order="A"/><cml:bond atomRefs2="a3 a4" order="A"/>
    <cml:bond atomRefs2="a4 a5" order="A"/><cml:bond atomRefs2="a5 a6" order="A"/><cml:bond atomRefs2="a6 a1" order="A"/>
    <cml:bond atomRefs2="a1 a7" order="1"/><cml:bond atomRefs2="a2 a8" order="partial12"/>
   </cml:bondArray>
   <cml:property dictRef="x:mass" title="mass"><cml:scalar dataType="xsd:double" units="x:u">93.1</cml:scalar></cml:property>
  </cml:molecule>
  <cml:molecule id="m2" count="2">
   <cml:atomArray><cml:atom id="a1" elementType="O" x3="5.0" y3="0.0" z3="0.3"/><cml:atom id="a2" elementType="C" x3="6.4" y3="0.0" z3="0.0" spinMultiplicity="2"/></cml:atomArray>
   <cml:bondArray><cml:bond atomRefs2="a1 a2" order="S"/></cml:bondArray>
  </cml:molecule>
 </cml:molecule>
</cml:cml>"#;

/// A Marvin document (synthetic, by the documentation and the schema): array-form atoms, an R atom, an S-group as a nested
/// molecule, a wedge, a reaction with its lists and arrow, a reaction sign, a text box, a polyline arrow, a rectangle.
const FOREIGN_MRV: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<cml xmlns="http://www.chemaxon.com" version="ChemAxon file format v20.20.0, generated by vunknown">
<MHead><MarvinGUI><mprop name="x" value="1"/></MarvinGUI></MHead>
<MDocument>
  <MChemicalStruct>
    <reaction>
      <arrow type="DEFAULT" x1="4.5" y1="0" x2="7.5" y2="0"/>
      <reactantList>
        <molecule molID="m1">
          <atomArray atomID="a1 a2 a3 a4 a5" elementType="C C O F R" x2="0 1.54 2.31 2.31 -0.77" y2="0 0 1.33 -1.33 1.33" formalCharge="0 0 -1 0 0" isotope="0 13 0 0 0" rgroupRef="0 0 0 0 2"/>
          <bondArray>
            <bond id="b1" atomRefs2="a1 a2" order="1"/>
            <bond id="b2" atomRefs2="a2 a3" order="1"><bondStereo>W</bondStereo></bond>
            <bond id="b3" atomRefs2="a2 a4" order="1"/>
            <bond id="b4" atomRefs2="a1 a5" order="1"/>
          </bondArray>
          <molecule molID="sg1" id="sg1" role="SuperatomSgroup" title="x" atomRefs="a1 a2"/>
        </molecule>
      </reactantList>
      <agentList/>
      <productList>
        <molecule molID="m2">
          <atomArray><atom id="a1" elementType="C" x2="9" y2="0" radical="monovalent"/><atom id="a2" elementType="N" x2="10.54" y2="0"/><atom id="a3" elementType="*" x2="11.31" y2="1.33"/></atomArray>
          <bondArray><bond id="b1" atomRefs2="a1 a2" order="2"/><bond id="b2" atomRefs2="a2 a3" order="1" convention="cxn:coord"/></bondArray>
        </molecule>
      </productList>
    </reaction>
    <Rgroup rgroupID="2"><molecule molID="m3"><atomArray atomID="a1" elementType="C" x2="0" y2="-9"/></molecule></Rgroup>
  </MChemicalStruct>
  <MReactionSign id="o1"><MPoint x="3.3" y="0.3"/><MPoint x="3.9" y="0.3"/><MPoint x="3.9" y="-0.3"/><MPoint x="3.3" y="-0.3"/></MReactionSign>
  <MTextBox id="o2" autoSize="true"><Field name="text"><![CDATA[{D font=Arial,size=11}{fg=#000000}heat, <2 h>]]></Field><MPoint x="5" y="1.5"/><MPoint x="7" y="1.5"/><MPoint x="7" y="1"/><MPoint x="5" y="1"/></MTextBox>
  <MPolyline id="o3" headLength="0.6" headWidth="0.5"><MPoint x="4.5" y="-3"/><MPoint x="7.5" y="-3"/></MPolyline>
  <MPolyline id="o4"><MPoint x="0" y="-5"/><MPoint x="1" y="-5"/></MPolyline>
  <MRectangle id="o5"><MPoint x="0" y="0"/><MPoint x="1" y="0"/><MPoint x="1" y="1"/><MPoint x="0" y="1"/></MRectangle>
</MDocument>
</cml>"#;

#[test]
fn foreign_cml_and_marvin_documents_open_with_what_is_out_of_scope_listed() {
    let (d, info) = import(FOREIGN_CML).unwrap();
    assert_eq!(info.format, "cml");
    assert_eq!((d.atoms.len(), d.bonds.len(), d.objects.len()), (10, 9, 0));
    assert_eq!(
        d.bonds.iter().filter(|b| b.o == 2).count(),
        3,
        "the aromatic ring is kekulised"
    );
    let n = d.atoms.iter().find(|a| a.el == "N").unwrap();
    assert_eq!((n.chg, n.iso), (1, 15));
    assert_eq!(d.atoms[7].lab, "R", "the dummy atom is a query atom");
    assert_eq!(d.atoms[9].rad, 1, "a doublet is one unpaired electron");
    // the second molecule has its own atom ids (a1, a2 again) and only 3D coordinates: read in place, from x3 / y3
    assert!(d.atoms[8].x > d.atoms[7].x && d.bonds.iter().any(|b| (b.a, b.b) == (9, 10)));
    assert_eq!(
        info.notes(),
        [
            ("draw2.import.queryAtoms", 1),
            ("draw2.import.queryBonds", 1)
        ]
    );
    assert_eq!(Document::from_json(&d.to_json()).unwrap(), d);

    let (d, info) = import(FOREIGN_MRV).unwrap();
    assert_eq!(info.format, "cml");
    // the molecule inside the R-group definition is not a structure of the drawing
    assert_eq!((d.atoms.len(), d.bonds.len()), (8, 6));
    let labs: Vec<&str> = d.atoms.iter().map(|a| a.lab.as_str()).collect();
    assert_eq!(labs, ["", "", "", "", "R2", "", "", "R"]);
    assert_eq!(
        (d.atoms[1].iso, d.atoms[2].chg, d.atoms[5].rad),
        (13, -1, 1)
    );
    // the wedge at the centre: `W` starts at the first atom of the bond; the MDL codes of Marvin say the same (1) or the
    // mirror image (6); a hatch `H` is the mirror image too
    let marks: Vec<(u32, u8)> = d
        .bonds
        .iter()
        .filter(|b| b.s != 0)
        .map(|b| (b.a, b.s))
        .collect();
    assert_eq!(marks, [(2, 1)], "{:?}", d.bonds);
    let with = |mark: &str| {
        let (doc, _) = import(&FOREIGN_MRV.replace("<bondStereo>W</bondStereo>", mark)).unwrap();
        let all = doc.to_smiles().unwrap();
        all.split('.')
            .find(|p| p.contains('F'))
            .unwrap()
            .to_string()
    };
    let up = with("<bondStereo>W</bondStereo>");
    let down = with("<bondStereo>H</bondStereo>");
    assert!(
        up.contains('@') && down.contains('@') && up != down,
        "{up} {down}"
    );
    assert_eq!(
        with(r#"<bondStereo convention="MDL" conventionValue="1"/>"#),
        up
    );
    assert_eq!(
        with(r#"<bondStereo convention="MDL" conventionValue="6"/>"#),
        down
    );
    assert!(!with("").contains('@'));
    assert_eq!(
        info.notes(),
        [
            ("draw2.import.queryAtoms", 1),
            ("draw2.import.queryBonds", 0),
            ("draw2.import.sgroups", 1),
            ("draw2.import.other", 3)
        ]
        .into_iter()
        .filter(|n| n.1 > 0)
        .collect::<Vec<_>>()
    );
    // objects: the reaction arrow between the two molecules, the sign, the text without its formatting, the polyline arrow
    let kinds: Vec<ObjKind> = d.objects.iter().map(|o| o.t).collect();
    assert_eq!(
        kinds,
        [ObjKind::Arrow, ObjKind::Plus, ObjKind::Text, ObjKind::Arrow]
    );
    let arrow = &d.objects[0];
    assert!(d.atoms[3].x < arrow.x && arrow.x < arrow.x2 && arrow.x2 < d.atoms[5].x);
    assert_eq!(d.objects[2].text, "heat, <2 h>");
    assert!(d.objects[3].y < arrow.y);
    assert_eq!(Document::from_json(&d.to_json()).unwrap(), d);
}

#[test]
fn a_damaged_cml_is_refused_or_read_but_never_a_panic() {
    for bad in [
        "<cml><molecule><atomArray/></molecule></cml>",
        "<cml><molecule><atomArray><atom id=\"a1\" elementType=\"C\"/></atomArray><bondArray><bond atomRefs2=\"a1 a9\" order=\"S\"/></bondArray></molecule></cml>",
        "<cml><molecule><atomArray><atom id=\"a1\" elementType=\"C\"/></atomArray><bondArray><bond order=\"S\"/></bondArray></molecule></cml>",
        "<cml><molecule><atomArray>",
        "<molecules><molecule/></molecules>",
        "<cml><molecule><atomArray atomID=\"a1 a2 a3\" elementType=\"C C C\" x2=\"1e300 0 1\" y2=\"0 0 0\"/><bondArray><bond atomRefs2=\"a2 a3\" order=\"1\"/></bondArray></molecule></cml>",
    ] {
        assert_eq!(import(bad).unwrap_err(), "error.draw2.cml", "{bad}");
    }
    // odd but readable: lists of different lengths, missing coordinates, an element that would break its line, no molecules
    let odd = "<cml><molecule><atomArray atomID=\"a1 a2 a3\" elementType=\"C N\" x2=\"0\" formalCharge=\"0 100000\"/></molecule><molecule><atomArray><atom id=\"q\" elementType=\"C CHG=5&#10;M  END\"/><atom/></atomArray></molecule></cml>";
    let (d, info) = import(odd).unwrap();
    assert_eq!(d.atoms.len(), 5);
    assert_eq!(info.query_atoms, 3, "{info:?}");
    assert!(d.atoms.iter().all(|a| a.chg == 0));
    assert!(import("<cml/>").unwrap().0.is_empty());
    let good = drawn("N[C@@H](C)C(=O)O").to_cml();
    let mut x = 0xD6E8_FEB8_6659_FD93u64;
    let mut next = |n: usize| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % n as u64) as usize
    };
    let (mut read, mut refused) = (0, 0);
    for text in [good.as_str(), FOREIGN_CML, FOREIGN_MRV] {
        for _ in 0..2000 {
            let mut b = text.as_bytes().to_vec();
            for _ in 0..1 + next(3) {
                let at = next(b.len());
                match next(3) {
                    0 => b[at] = b"0123456789-.e<>/\"= &x"[next(21)],
                    1 => {
                        b.remove(at);
                    }
                    _ => b.insert(at, b"9<>\"-&"[next(6)]),
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
        read > 200 && refused > 200,
        "{read} read, {refused} refused"
    );
}
