//! CDXML (ChemDraw XML): written and read from the public CDX SDK documentation. Synthetic structures only.

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
fn a_drawing_goes_out_as_cdxml_by_the_documentation() {
    let mut d = drawn("N[C@@H](C)C(=O)[O-]");
    d.atoms[2].lab = "R2".into();
    let text = d.to_cdxml();
    assert!(text.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<!DOCTYPE CDXML SYSTEM \"http://www.camsoft.com/xml/cdxml.dtd\">\n<CDXML "));
    assert!(text.ends_with("</page>\n</CDXML>\n"));
    // well-formed, and shaped as the documentation says: nodes and bonds inside a fragment inside a page
    let root = xml::parse(&text).unwrap();
    assert_eq!(
        (root.name.as_str(), root.attr("BondLength")),
        ("CDXML", Some("30"))
    );
    let page = root.each("page").next().unwrap();
    let fragment = page.each("fragment").next().unwrap();
    let nodes: Vec<_> = fragment.each("n").collect();
    let bonds: Vec<_> = fragment.each("b").collect();
    assert_eq!((nodes.len(), bonds.len()), (6, 5));
    // ids are unique over the file; every bond names two nodes of the fragment
    let mut ids: Vec<&str> = nodes
        .iter()
        .chain(&bonds)
        .filter_map(|e| e.attr("id"))
        .collect();
    ids.push(fragment.attr("id").unwrap());
    let n = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), n);
    for b in &bonds {
        for end in ["B", "E"] {
            assert!(nodes.iter().any(|n| n.attr("id") == b.attr(end)), "{b:?}");
        }
    }
    // a carbon is a bare node; a hetero atom carries its element, hydrogens and label; the charge is an attribute
    assert_eq!(
        (nodes[1].attr("Element"), nodes[1].children.len()),
        (None, 0)
    );
    assert_eq!(
        (nodes[0].attr("Element"), nodes[0].attr("NumHydrogens")),
        (Some("7"), Some("2"))
    );
    assert_eq!(
        nodes[0]
            .each("t")
            .next()
            .unwrap()
            .each("s")
            .next()
            .unwrap()
            .text,
        "NH2"
    );
    assert_eq!(
        (nodes[5].attr("Element"), nodes[5].attr("Charge")),
        (Some("8"), Some("-1"))
    );
    // the R group is a generic nickname with its label
    assert_eq!(
        (nodes[2].attr("NodeType"), nodes[2].attr("GenericNickname")),
        (Some("GenericNickname"), Some("R2"))
    );
    // one wedge, from the stereocentre; one double bond; the coordinates are points, y down, inside the page
    let wedge: Vec<_> = bonds
        .iter()
        .filter(|b| b.attr("Display").is_some())
        .collect();
    assert_eq!(wedge.len(), 1);
    assert_eq!(wedge[0].attr("B"), nodes[1].attr("id"));
    assert!(["WedgeBegin", "WedgedHashBegin"].contains(&wedge[0].attr("Display").unwrap()));
    assert_eq!(
        bonds
            .iter()
            .filter(|b| b.attr("Order") == Some("2"))
            .count(),
        1
    );
    let p = |n: &xml::Element| -> Vec<f64> {
        n.attr("p")
            .unwrap()
            .split(' ')
            .map(|v| v.parse().unwrap())
            .collect()
    };
    assert!(nodes.iter().all(|n| p(n).iter().all(|v| *v >= 29.99)));
    let (top, bottom) = (
        d.atoms.iter().map(|a| a.y).fold(f64::MIN, f64::max),
        d.atoms.iter().map(|a| a.y).fold(f64::MAX, f64::min),
    );
    let high = d.atoms.iter().position(|a| a.y == top).unwrap();
    let low = d.atoms.iter().position(|a| a.y == bottom).unwrap();
    assert!(
        p(nodes[high])[1] < p(nodes[low])[1],
        "the highest atom has the smallest y"
    );
    // nothing drawn: an empty page
    assert!(xml::parse(&Document::default().to_cdxml())
        .unwrap()
        .each("page")
        .next()
        .unwrap()
        .children
        .is_empty());
}

#[test]
fn cdxml_comes_back_as_the_same_drawing() {
    for s in [
        "CC(=O)O",
        "c1ccccc1",
        "N[C@@H](C)C(=O)O",
        "C[C@H]1CC[C@@H](O)CC1",
        "[NH4+].[Cl-]",
        "C[13CH3]",
        "[CH2]C",
        "CC[*:5].[*]CO",
        "C[Si](C)(C)C",
    ] {
        let d = drawn(s);
        let (back, info) = import(&d.to_cdxml()).unwrap();
        assert_eq!(info.format, "cdxml");
        assert!(info.notes().is_empty(), "{s}: {:?}", info.notes());
        assert_eq!(back.to_smiles().unwrap(), d.to_smiles().unwrap(), "{s}");
        if !s.contains('*') {
            assert_eq!(canonical(&back.to_smiles().unwrap()), canonical(s), "{s}");
        }
        // the shape is the drawn one: the file is scaled to its own bond length, so every bond keeps its share of the first
        let len = |doc: &Document, x: &mzdraw::Bond| {
            let (p, q) = (doc.atom(x.a).unwrap(), doc.atom(x.b).unwrap());
            ((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt()
        };
        for (a, b) in d.bonds.iter().zip(&back.bonds) {
            let (was, is) = (
                len(&d, a) / len(&d, &d.bonds[0]),
                len(&back, b) / len(&back, &back.bonds[0]),
            );
            assert!((was - is).abs() < 2e-3, "{s}: {was} {is}");
        }
    }
    // a reaction: arrow with its text, plus, text; markup characters and non-ASCII in the texts
    let mut e = Editor::default();
    e.import(r#"{"format":"mzdraw/1","next":10,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0},{"id":2,"el":"O","x":1.0,"y":0.0},{"id":3,"el":"C","x":6.0,"y":0.0},{"id":4,"el":"N","x":7.0,"y":0.0}],"bonds":[{"id":5,"a":1,"b":2,"o":1},{"id":6,"a":3,"b":4,"o":1}],"objects":[{"id":7,"t":"arrow","x":2.0,"y":0.0,"x2":5.0,"y2":0.0,"text":"NH3, \"heat\" <Δ> & more"},{"id":8,"t":"plus","x":3.5,"y":-1.0},{"id":9,"t":"text","x":3.0,"y":2.0,"text":"second text"}]}"#).unwrap();
    let (back, info) = import(&e.save("cdxml").unwrap()).unwrap();
    assert!(info.notes().is_empty(), "{:?}", info.notes());
    assert_eq!(back.atoms.len(), 4);
    let kinds: Vec<ObjKind> = back.objects.iter().map(|o| o.t).collect();
    // the arrow is read once (its older graphic form points at the arrow object); its text is a text of its own
    assert_eq!(
        kinds,
        [ObjKind::Arrow, ObjKind::Text, ObjKind::Plus, ObjKind::Text]
    );
    let near = |a: f64, b: f64| (a - b).abs() < 1e-3;
    let arrow = &back.objects[0];
    // the same picture: the arrow runs left to right between the two molecules, as long as it was
    assert!(near(arrow.x2 - arrow.x, 3.0) && near(arrow.y, arrow.y2));
    assert!(
        near(arrow.x - back.atoms[1].x, 1.0)
            && near(back.atoms[2].x - arrow.x2, 1.0)
            && near(arrow.y, back.atoms[1].y)
    );
    assert_eq!(back.objects[1].text, "NH3, \"heat\" <Δ> & more");
    assert!(near(back.objects[1].y - arrow.y, 0.5) && near(back.objects[1].x - arrow.x, 1.5));
    assert!(near(arrow.y - back.objects[2].y, 1.0));
    assert_eq!(back.objects[3].text, "second text");
    assert_eq!(
        Document::from_json(&back.to_json()).unwrap(),
        back,
        "a valid document"
    );
}

/// A file as ChemDraw-like programs write it (synthetic, by the documentation): two fragments in a group, an aromatic ring, a
/// nickname with its inner fragment, a generic nickname, an element list, wedges of both ends, a line graphic as an arrow, a
/// plus symbol, a caption of two style runs, a reaction scheme, and things mzDraw does not draw.
const FOREIGN: &str = r##"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML CreationProgram="synthetic" BondLength="14.4" LabelFont="3">
<colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
<fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
<page id="900" BoundingBox="0 0 540 720">
 <group id="800">
  <fragment id="1">
   <n id="2" p="100 100"/><n id="3" p="112.47 107.2"/><n id="4" p="112.47 121.6"/><n id="5" p="100 128.8"/><n id="6" p="87.53 121.6"/><n id="7" p="87.53 107.2"/>
   <n id="8" p="100 85.6" Element="7" NumHydrogens="3" Charge="1"><t p="96 89"><s font="3" size="10" face="96">NH3+</s></t></n>
   <n id="9" p="124.94 100" NodeType="Nickname"><fragment id="700"><n id="701" p="0 0" NodeType="ExternalConnectionPoint"/><n id="702" p="1 1"/><b id="703" B="701" E="702"/></fragment><t p="121 103"><s font="3" size="10" face="96">Ph</s></t></n>
   <n id="10" p="124.94 128.8" NodeType="GenericNickname" GenericNickname="R3"><t><s font="3" size="10" face="96">R3</s></t></n>
   <n id="11" p="100 143.2" NodeType="ElementList" ElementList="8 16"><t><s font="3" size="10" face="96">[O,S]</s></t></n>
   <n id="12" p="75.06 128.8" NodeType="Unspecified"><t><s font="3" size="10" face="96">Whatever</s></t></n>
   <b id="20" B="2" E="3" Order="1.5"/><b id="21" B="3" E="4" Order="1.5"/><b id="22" B="4" E="5" Order="1.5"/><b id="23" B="5" E="6" Order="1.5"/><b id="24" B="6" E="7" Order="1.5"/><b id="25" B="7" E="2" Order="1.5"/>
   <b id="26" B="2" E="8"/><b id="27" B="3" E="9" Display="Bold"/><b id="28" B="4" E="10"/><b id="29" B="5" E="11" Order="dative"/><b id="30" B="6" E="12"/>
  </fragment>
  <fragment id="40">
   <n id="41" p="250 114.4"/><n id="42" p="262.47 107.2"/><n id="43" p="274.94 114.4" Element="8" NumHydrogens="1"><t><s font="3" size="10" face="96">OH</s></t></n><n id="44" p="262.47 92.8" Element="9"><t><s font="3" size="10" face="96">F</s></t></n><n id="45" p="250 128.8" Element="17"><t><s font="3" size="10" face="96">Cl</s></t></n>
   <b id="46" B="41" E="42"/><b id="47" B="42" E="43"/><b id="48" B="42" E="44" Display="WedgeBegin"/><b id="49" B="45" E="41" Display="WedgedHashEnd"/>
  </fragment>
 </group>
 <graphic id="60" BoundingBox="230 114.4 150 114.4" GraphicType="Line" ArrowType="FullHead" HeadSize="1000"/>
 <graphic id="61" BoundingBox="140 114.4 144 114.4" GraphicType="Symbol" SymbolType="Plus"/>
 <graphic id="62" BoundingBox="10 10 50 50" GraphicType="Rectangle"/>
 <graphic id="63" BoundingBox="10 10 50 50" GraphicType="Line"/>
 <t id="64" p="170 104" Justification="Center"><s font="3" size="10" face="1">H</s><s font="3" size="10" face="32">2</s><s font="3" size="10" face="0">O, &lt;heat&gt;
second line</s></t>
 <curve id="65" CurvePoints="1 2 3 4"/>
 <scheme id="66"><step id="67" ReactionStepReactants="1" ReactionStepProducts="40" ReactionStepArrows="60"/></scheme>
</page>
</CDXML>
"##;

#[test]
fn a_foreign_cdxml_opens_with_what_is_out_of_scope_listed() {
    let (d, info) = import(FOREIGN).unwrap();
    assert_eq!(info.format, "cdxml");
    assert_eq!((d.atoms.len(), d.bonds.len()), (16, 15));
    // the ring is kekulised; labels: the nickname of the table, the R group; the list and the unknown label are query atoms
    assert_eq!(d.bonds.iter().filter(|b| b.o == 2).count(), 3);
    let labs: Vec<&str> = d.atoms.iter().map(|a| a.lab.as_str()).collect();
    assert_eq!(
        &labs[..11],
        ["", "", "", "", "", "", "", "Ph", "R3", "R", "R"]
    );
    assert_eq!((d.atoms[6].el.as_str(), d.atoms[6].chg), ("N", 1));
    // the inner fragment of the nickname is its expansion: its atoms are not atoms of the drawing
    assert!(
        d.to_smiles().unwrap().contains("C=1C=CC=CC1"),
        "{}",
        d.to_smiles().unwrap()
    );
    // wedges: the centre of the second fragment is read with its wedge. A solid and a hashed wedge give the two mirror
    // images, and an End display is the Begin display with the atoms the other way round
    let wedge = r#"<b id="48" B="42" E="44" Display="WedgeBegin"/>"#;
    let second = |bond: &str| {
        let (doc, _) = import(&FOREIGN.replace(wedge, bond)).unwrap();
        let all = doc.to_smiles().unwrap();
        all.split('.')
            .find(|p| p.contains("Cl"))
            .unwrap()
            .to_string()
    };
    let solid = second(wedge);
    let hashed = second(r#"<b id="48" B="42" E="44" Display="WedgedHashBegin"/>"#);
    assert!(
        solid.contains('@') && hashed.contains('@') && solid != hashed,
        "{solid} {hashed}"
    );
    assert_eq!(
        second(r#"<b id="48" B="44" E="42" Display="WedgeEnd"/>"#),
        solid
    );
    assert_eq!(
        second(r#"<b id="48" B="44" E="42" Display="WedgedHashEnd"/>"#),
        hashed
    );
    assert!(!second(r#"<b id="48" B="42" E="44"/>"#).contains('@'));
    assert!(
        d.bonds.iter().any(|b| b.s == 1 && b.a == 13),
        "the wedge starts at the centre"
    );
    assert_eq!(
        info.notes(),
        [
            ("draw2.import.queryAtoms", 2),
            ("draw2.import.queryBonds", 1),
            ("draw2.import.other", 3)
        ]
    );
    // objects: the arrow points from the plus side to the second fragment; the caption is one text per line, runs joined
    let kinds: Vec<ObjKind> = d.objects.iter().map(|o| o.t).collect();
    assert_eq!(
        kinds,
        [ObjKind::Arrow, ObjKind::Plus, ObjKind::Text, ObjKind::Text]
    );
    let arrow = &d.objects[0];
    assert!(arrow.x < arrow.x2 && d.atoms[7].x < arrow.x && arrow.x2 < d.atoms[11].x);
    assert_eq!(
        (d.objects[2].text.as_str(), d.objects[3].text.as_str()),
        ("H2O, <heat>", "second line")
    );
    assert!(d.objects[3].y < d.objects[2].y);
    // y is turned: the ammonium, drawn above the ring in the file (smaller y), is above it in the drawing
    assert!(d.atoms[6].y > d.atoms[0].y);
    // one bond length of the file (14.4 pt) is one unit here
    let (p, q) = (&d.atoms[0], &d.atoms[6]);
    assert!((((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt() - 1.0).abs() < 0.01);
    assert_eq!(Document::from_json(&d.to_json()).unwrap(), d);
}

#[test]
fn a_damaged_cdxml_is_refused_or_read_but_never_a_panic() {
    for bad in [
        "<CDXML>",
        "<CDXML><page></CDXML>",
        "<notCDXML><CDXML/></notCDXML>",
        "<CDXML><page><fragment><n id=\"1\" p=\"0 0\"/><b B=\"1\" E=\"2\"/></fragment></page></CDXML>",
        "<CDXML><page><fragment><n id=\"1\" p=\"0 0\"/><n id=\"2\" p=\"1e300 0\"/><n id=\"3\" p=\"1 0\"/><b B=\"1\" E=\"3\"/></fragment></page></CDXML>",
    ] {
        assert_eq!(import(bad).unwrap_err(), "error.draw2.cdxml", "{bad}");
    }
    // odd but readable: missing positions, an element number outside the table, a label that would break its line
    let odd = "<CDXML><page><fragment><n id=\"1\"/><n id=\"2\" p=\"x y\" Element=\"999\"/><n id=\"3\" p=\"5 5\" Element=\"0\"/><n id=\"4\" p=\"9 9\" NodeType=\"Fragment\"><t><s>C CHG=5\nM  END</s></t></n><n id=\"1\" p=\"3 3\" Charge=\"100000\"/></fragment><t><s>no place</s></t><arrow Head3D=\"1 1 0\"/><graphic GraphicType=\"Symbol\" SymbolType=\"Plus\"/></page></CDXML>";
    let (d, info) = import(odd).unwrap();
    assert_eq!((d.atoms.len(), d.objects.len()), (5, 0));
    assert_eq!((info.query_atoms, info.other), (3, 3), "{info:?}");
    assert!(d.atoms.iter().all(|a| a.chg == 0));
    let good = drawn("N[C@@H](C)C(=O)O").to_cdxml();
    let mut x = 0xD6E8_FEB8_6659_FD93u64;
    let mut next = |n: usize| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % n as u64) as usize
    };
    let (mut read, mut refused) = (0, 0);
    for text in [good.as_str(), FOREIGN] {
        for _ in 0..2500 {
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
