//! Binary CDX, read from the public CDX SDK documentation. The files here are synthetic: a small writer of this test makes them
//! from the CDXML fixtures (the writer keeps its own tables of the tag values, as the documentation lists them), so the reader is
//! checked against a second copy of the layout, and the one example the documentation prints byte by byte is read as it stands.

use mzdraw::io::{import, import_bytes};
use mzdraw::xml::{self, Element};
use mzdraw::{Document, Editor};

const FOREIGN: &str = include_str!("../../../tests/data/foreign.cdxml");

// ---- the writer: CDXML text -> CDX bytes -------------------------------------------------------------------------------

const OBJECTS: &[(&str, u16)] = &[
    ("CDXML", 0x8000),
    ("page", 0x8001),
    ("group", 0x8002),
    ("fragment", 0x8003),
    ("n", 0x8004),
    ("b", 0x8005),
    ("t", 0x8006),
    ("graphic", 0x8007),
    ("curve", 0x8008),
    ("embeddedobject", 0x8009),
    ("scheme", 0x800D),
    ("step", 0x800E),
    ("arrow", 0x8027),
];
const NODE_TYPES: &[&str] = &[
    "Unspecified",
    "Element",
    "ElementList",
    "ElementListNickname",
    "Nickname",
    "Fragment",
    "Formula",
    "GenericNickname",
];
const DISPLAYS: &[&str] = &[
    "Solid",
    "Dash",
    "Hash",
    "WedgedHashBegin",
    "WedgedHashEnd",
    "Bold",
    "WedgeBegin",
    "WedgeEnd",
    "Wavy",
];
const GRAPHICS: &[&str] = &[
    "Undefined",
    "Line",
    "Arc",
    "Rectangle",
    "Oval",
    "Orbital",
    "Bracket",
    "Symbol",
];
const SYMBOLS: &[&str] = &[
    "LonePair",
    "Electron",
    "RadicalCation",
    "RadicalAnion",
    "CirclePlus",
    "CircleMinus",
    "Dagger",
    "DoubleDagger",
    "Plus",
    "Minus",
];
const ARROWS: &[(&str, u16)] = &[
    ("NoHead", 0),
    ("HalfHead", 1),
    ("FullHead", 2),
    ("Resonance", 4),
    ("Equilibrium", 8),
];
const ORDERS: &[(&str, u16)] = &[
    ("1", 0x1),
    ("2", 0x2),
    ("3", 0x4),
    ("4", 0x8),
    ("1.5", 0x80),
    ("dative", 0x1000),
];

fn index(names: &[&str], v: &str) -> Option<u16> {
    names.iter().position(|n| *n == v).map(|i| i as u16)
}

fn property(out: &mut Vec<u8>, tag: u16, data: &[u8]) {
    out.extend(tag.to_le_bytes());
    if data.len() < 0xFFFF {
        out.extend((data.len() as u16).to_le_bytes());
    } else {
        out.extend(0xFFFFu16.to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
    }
    out.extend(data);
}

fn coord(v: f64) -> [u8; 4] {
    ((v * 65536.0).round() as i32).to_le_bytes()
}

fn numbers(v: &str) -> Option<Vec<f64>> {
    v.split_whitespace().map(|t| t.parse().ok()).collect()
}

/// One attribute as its property; what the documentation gives no tag for is not written.
fn attribute(out: &mut Vec<u8>, name: &str, v: &str) {
    let int = || v.parse::<i64>().ok();
    match name {
        "p" => {
            if let Some(n) = numbers(v).filter(|n| n.len() == 2) {
                let data = [coord(n[1]), coord(n[0])].concat();
                property(out, 0x0200, &data);
            }
        }
        // left, top, right, bottom -> top, left, bottom, right
        "BoundingBox" => {
            if let Some(n) = numbers(v).filter(|n| n.len() == 4) {
                let data = [coord(n[1]), coord(n[0]), coord(n[3]), coord(n[2])].concat();
                property(out, 0x0204, &data);
            }
        }
        "Head3D" | "Tail3D" => {
            if let Some(n) = numbers(v).filter(|n| n.len() == 3) {
                let data = [coord(n[2]), coord(n[1]), coord(n[0])].concat();
                property(out, if name == "Head3D" { 0x0207 } else { 0x0208 }, &data);
            }
        }
        "SupersededBy" | "B" | "E" => {
            let tag = match name {
                "SupersededBy" => 0x0012,
                "B" => 0x0604,
                _ => 0x0605,
            };
            if let Some(n) = int().and_then(|n| u32::try_from(n).ok()) {
                property(out, tag, &n.to_le_bytes());
            }
        }
        "NodeType" => {
            if let Some(i) = index(NODE_TYPES, v) {
                property(out, 0x0400, &i.to_le_bytes());
            }
        }
        "Element" | "Isotope" => {
            if let Some(n) = int().and_then(|n| i16::try_from(n).ok()) {
                property(
                    out,
                    if name == "Element" { 0x0402 } else { 0x0420 },
                    &n.to_le_bytes(),
                );
            }
        }
        "Charge" => {
            if let Some(n) = int().and_then(|n| i8::try_from(n).ok()) {
                property(out, 0x0421, &n.to_le_bytes());
            }
        }
        "Radical" => {
            if let Some(n) = int().and_then(|n| u8::try_from(n).ok()) {
                property(out, 0x0422, &[n]);
            }
        }
        "Order" => {
            let bits: Option<u16> = v
                .split_whitespace()
                .map(|t| ORDERS.iter().find(|o| o.0 == t).map(|o| o.1))
                .try_fold(0, |acc, b| Some(acc | b?));
            if let Some(b) = bits {
                property(out, 0x0600, &b.to_le_bytes());
            }
        }
        "Display" => {
            if let Some(i) = index(DISPLAYS, v) {
                property(out, 0x0601, &i.to_le_bytes());
            }
        }
        "GraphicType" => {
            if let Some(i) = index(GRAPHICS, v) {
                property(out, 0x0A00, &i.to_le_bytes());
            }
        }
        "ArrowType" => {
            if let Some(a) = ARROWS.iter().find(|a| a.0 == v) {
                property(out, 0x0A02, &a.1.to_le_bytes());
            }
        }
        "SymbolType" => {
            if let Some(i) = index(SYMBOLS, v) {
                property(out, 0x0A07, &i.to_le_bytes());
            }
        }
        _ => {}
    }
}

/// The fonts of a document: id and name; one that carries a character outside Latin-1 is declared as code page 65001 (UTF-8).
fn font_table(root: &Element) -> Vec<u8> {
    let mut wide: Vec<String> = Vec::new();
    fn scan(e: &Element, wide: &mut Vec<String>) {
        if e.name == "s" && e.text.chars().any(|c| c as u32 > 255) {
            wide.extend(e.attr("font").map(str::to_string));
        }
        e.children.iter().for_each(|c| scan(c, wide));
    }
    scan(root, &mut wide);
    let mut data = Vec::new();
    let fonts: Vec<&Element> = (root.each("fonttable"))
        .flat_map(|t| t.each("font"))
        .collect();
    data.extend(1u16.to_le_bytes());
    data.extend((fonts.len() as u16).to_le_bytes());
    for f in fonts {
        let id = f.attr("id").unwrap_or("0");
        let page: u16 = if wide.iter().any(|w| w == id) {
            65001
        } else {
            1252
        };
        data.extend(id.parse::<u16>().unwrap_or(0).to_le_bytes());
        data.extend(page.to_le_bytes());
        let name = f.attr("name").unwrap_or("");
        data.extend((name.len() as u16).to_le_bytes());
        data.extend(name.as_bytes());
    }
    data
}

/// The string of a `t` object: its `s` runs; text outside Latin-1 is written as UTF-8 (its font is declared as such).
fn string(t: &Element) -> Vec<u8> {
    let runs: Vec<&Element> = t.each("s").collect();
    let mut data = (runs.len() as u16).to_le_bytes().to_vec();
    let (mut text, mut start) = (Vec::new(), 0u16);
    for r in &runs {
        let font: u16 = r.attr("font").and_then(|f| f.parse().ok()).unwrap_or(0);
        let face: u16 = r.attr("face").and_then(|f| f.parse().ok()).unwrap_or(0);
        data.extend(start.to_le_bytes());
        data.extend([font.to_le_bytes(), face.to_le_bytes()].concat());
        data.extend(200u16.to_le_bytes());
        data.extend(0u16.to_le_bytes());
        let wide = r.text.chars().any(|c| c as u32 > 255);
        let bytes: Vec<u8> = if wide {
            r.text.as_bytes().to_vec()
        } else {
            r.text.chars().map(|c| c as u8).collect()
        };
        start += bytes.len() as u16;
        text.extend(bytes);
    }
    data.extend(text);
    data
}

fn object(out: &mut Vec<u8>, e: &Element, root: &Element) {
    let Some(tag) = OBJECTS.iter().find(|o| o.0 == e.name).map(|o| o.1) else {
        // a table the format keeps as a property, or an element with no tag here
        return;
    };
    out.extend(tag.to_le_bytes());
    out.extend(
        e.attr("id")
            .and_then(|i| i.parse::<u32>().ok())
            .unwrap_or(0)
            .to_le_bytes(),
    );
    for (k, v) in &e.attrs {
        attribute(out, k, v);
    }
    if tag == 0x8000 {
        property(out, 0x0100, &font_table(root));
    }
    if e.name == "t" {
        property(out, 0x0700, &string(e));
    }
    for c in &e.children {
        if e.name != "t" {
            object(out, c, root);
        }
    }
    out.extend([0, 0]);
}

/// The CDX file of a CDXML text, with the end mark of the document.
fn cdx_of(cdxml: &str) -> Vec<u8> {
    let root = xml::parse(cdxml).unwrap();
    let mut out = b"VjCD0100".to_vec();
    out.extend([4, 3, 2, 1]);
    out.extend([0u8; 16]);
    object(&mut out, &root, &root);
    out
}

fn bytes_of(hex: &str) -> Vec<u8> {
    hex.split_whitespace()
        .map(|h| u8::from_str_radix(h, 16).unwrap())
        .collect()
}

// ---- the tests ---------------------------------------------------------------------------------------------------------

fn drawn(smiles: &str) -> Document {
    let mut e = Editor::default();
    e.import(smiles).unwrap();
    e.doc().clone()
}

/// The CDXML fixtures of `tests/cdxml.rs`: what the writer of mzDraw makes of some structures, a reaction with texts, the
/// foreign file (with the variants of its wedge), and the odd file.
fn fixtures() -> Vec<(String, String)> {
    let mut all = Vec::new();
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
        all.push((s.to_string(), drawn(s).to_cdxml()));
    }
    let mut e = Editor::default();
    e.import(r#"{"format":"mzdraw/1","next":10,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0},{"id":2,"el":"O","x":1.0,"y":0.0},{"id":3,"el":"C","x":6.0,"y":0.0},{"id":4,"el":"N","x":7.0,"y":0.0}],"bonds":[{"id":5,"a":1,"b":2,"o":1},{"id":6,"a":3,"b":4,"o":1}],"objects":[{"id":7,"t":"arrow","x":2.0,"y":0.0,"x2":5.0,"y2":0.0,"text":"NH3, \"heat\" <Δ> & more"},{"id":8,"t":"plus","x":3.5,"y":-1.0},{"id":9,"t":"text","x":3.0,"y":2.0,"text":"second text"}]}"#).unwrap();
    all.push(("reaction".into(), e.save("cdxml").unwrap()));
    all.push(("foreign".into(), FOREIGN.to_string()));
    let wedge = r#"<b id="48" B="42" E="44" Display="WedgeBegin"/>"#;
    for bond in [
        r#"<b id="48" B="42" E="44" Display="WedgedHashBegin"/>"#,
        r#"<b id="48" B="44" E="42" Display="WedgeEnd"/>"#,
        r#"<b id="48" B="44" E="42" Display="WedgedHashEnd"/>"#,
        r#"<b id="48" B="42" E="44"/>"#,
        r#"<b id="48" B="42" E="44" Order="1 2"/>"#,
        r#"<b id="48" B="42" E="44" Order="dative"/>"#,
    ] {
        all.push((bond.to_string(), FOREIGN.replace(wedge, bond)));
    }
    all.push(("odd".into(), "<CDXML><page><fragment><n id=\"1\"/><n id=\"2\" p=\"x y\" Element=\"999\"/><n id=\"3\" p=\"5 5\" Element=\"0\"/><n id=\"4\" p=\"9 9\" NodeType=\"Fragment\"><t><s>C CHG=5\nM  END</s></t></n><n id=\"1\" p=\"3 3\" Charge=\"100000\"/></fragment><t><s>no place</s></t><arrow Head3D=\"1 1 0\"/><graphic GraphicType=\"Symbol\" SymbolType=\"Plus\"/></page></CDXML>".into()));
    all
}

#[test]
fn every_cdxml_fixture_opens_from_cdx_as_the_same_document() {
    for (name, cdxml) in fixtures() {
        let (want, info) = import(&cdxml).unwrap();
        let file = cdx_of(&cdxml);
        let (got, cdx_info) = import_bytes(&file).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(cdx_info.format, "cdx", "{name}");
        // the same notes, the same counts: the format is the only difference
        assert_eq!(
            mzdraw::io::Imported {
                format: "cdx",
                ..info
            },
            cdx_info,
            "{name}"
        );
        assert_eq!(got.to_json(), want.to_json(), "{name}");
        assert_eq!(Document::from_json(&got.to_json()).unwrap(), got, "{name}");
    }
}

/// The example of the documentation (dimethylamine), as printed there byte by byte: not made by the writer above.
#[test]
fn the_documented_example_is_read_as_printed() {
    let file = bytes_of(
        "56 6A 43 44 30 31 30 30 04 03 02 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 \
         01 80 01 00 00 00 03 80 08 00 00 00 04 80 05 00 00 00 00 02 08 00 00 40 A4 00 00 80 94 00 \
         02 04 02 00 07 00 00 00 04 80 06 00 00 00 00 02 08 00 BA 03 AC 00 4F 7A B1 00 00 00 \
         05 80 02 00 00 00 04 06 04 00 05 00 00 00 05 06 04 00 06 00 00 00 00 00 \
         04 80 04 00 00 00 00 02 08 00 BA 03 AC 00 B0 85 77 00 00 00 \
         05 80 03 00 00 00 04 06 04 00 05 00 00 00 05 06 04 00 04 00 00 00 00 00 00 00 00 00 00 00",
    );
    let (d, info) = import_bytes(&file).unwrap();
    assert_eq!(info.format, "cdx");
    assert_eq!((d.atoms.len(), d.bonds.len()), (3, 2));
    assert!(info.notes().is_empty(), "{:?}", info.notes());
    // two carbons and a nitrogen between them
    let els: Vec<&str> = d.atoms.iter().map(|a| a.el.as_str()).collect();
    assert_eq!(els, ["N", "C", "C"]);
    let canonical = |smiles: &str| {
        mzlab_mol::Mol::from_smiles(smiles)
            .unwrap()
            .canonical()
            .to_smiles()
            .unwrap()
    };
    assert_eq!(canonical(&d.to_smiles().unwrap()), canonical("CNC"));
    // y is turned: the nitrogen (y 164.25) is above the carbons (172.01)
    assert!(d.atoms[0].y > d.atoms[1].y);
}

#[test]
fn what_cdx_holds_that_is_not_drawn_is_counted_and_unknown_items_are_skipped_by_length() {
    // header, a document with a page: a rectangle graphic and a curve (not drawn), an object of a tag the pages do not list, a
    // property of a tag the pages do not list (with a long, 4-byte length), and a node
    let mut f = b"VjCD0100".to_vec();
    f.extend([4, 3, 2, 1]);
    f.extend([0u8; 16]);
    f.extend([0x00, 0x80, 1, 0, 0, 0]);
    f.extend([0x01, 0x80, 2, 0, 0, 0]);
    property(&mut f, 0x3FFE, &vec![7u8; 70_000]);
    f.extend([0x07, 0x80, 3, 0, 0, 0]);
    property(&mut f, 0x0A00, &3u16.to_le_bytes());
    f.extend([0, 0]);
    f.extend([0x08, 0x80, 4, 0, 0, 0, 0, 0]);
    f.extend([0xFE, 0xBF, 5, 0, 0, 0]);
    property(&mut f, 0x3FFD, &[1, 2, 3]);
    f.extend([0, 0]);
    f.extend([0x03, 0x80, 6, 0, 0, 0]);
    f.extend([0x04, 0x80, 7, 0, 0, 0]);
    property(&mut f, 0x0200, &[coord(30.0), coord(60.0)].concat());
    property(&mut f, 0x0402, &7i16.to_le_bytes());
    f.extend([0, 0]);
    f.extend([0, 0, 0, 0, 0, 0, 0, 0]);
    let (d, info) = import_bytes(&f).unwrap();
    assert_eq!((d.atoms.len(), d.atoms[0].el.as_str()), (1, "N"));
    assert_eq!(info.other, 3, "{info:?}");
    // the two properties of unlisted tags were skipped by their length and counted
    assert_eq!(info.props_skipped, 2, "{info:?}");
}

#[test]
fn text_is_decoded_by_the_code_page_of_its_font() {
    let doc = |page: u16, text: &[u8]| {
        let mut f = b"VjCD0100".to_vec();
        f.extend([4, 3, 2, 1]);
        f.extend([0u8; 16]);
        f.extend([0x00, 0x80, 1, 0, 0, 0]);
        let mut fonts = vec![1, 0, 1, 0, 3, 0];
        fonts.extend(page.to_le_bytes());
        fonts.extend([1, 0, b'F']);
        property(&mut f, 0x0100, &fonts);
        f.extend([0x01, 0x80, 2, 0, 0, 0]);
        f.extend([0x06, 0x80, 3, 0, 0, 0]);
        property(&mut f, 0x0200, &[coord(10.0), coord(20.0)].concat());
        // one run of font 3, then the text
        let mut s = vec![1, 0, 0, 0, 3, 0, 0, 0, 200, 0, 0, 0];
        s.extend(text);
        property(&mut f, 0x0700, &s);
        f.extend([0, 0, 0, 0, 0, 0, 0, 0]);
        import_bytes(&f).unwrap().0.objects[0].text.clone()
    };
    assert_eq!(doc(1252, b"caf\xE9 \xB0C"), "café °C");
    assert_eq!(doc(65001, "Δ → β".as_bytes()), "Δ → β");
    // a string with no style run is Latin-1
    let mut f = b"VjCD0100".to_vec();
    f.extend([4, 3, 2, 1]);
    f.extend([0u8; 16]);
    f.extend([
        0x00, 0x80, 1, 0, 0, 0, 0x01, 0x80, 2, 0, 0, 0, 0x06, 0x80, 3, 0, 0, 0,
    ]);
    property(&mut f, 0x0200, &[coord(1.0), coord(2.0)].concat());
    property(&mut f, 0x0700, &[0, 0, b'N', 0xE9]);
    f.extend([0; 8]);
    assert_eq!(import_bytes(&f).unwrap().0.objects[0].text, "Né");
}

#[test]
fn a_cut_file_is_refused_at_every_point_and_a_corrupt_one_is_never_a_panic() {
    let (_, cdxml) = &fixtures()[2];
    let file = cdx_of(cdxml);
    // every cut of the file, from the magic on, is refused with the key of the format
    for n in 8..file.len() {
        assert_eq!(
            import_bytes(&file[..n]).unwrap_err(),
            "error.draw2.cdx",
            "cut at {n} of {}",
            file.len()
        );
    }
    assert!(import_bytes(&file).is_ok());
    // the same for the foreign file (objects, texts, long records)
    let foreign = cdx_of(FOREIGN);
    for n in 8..foreign.len() {
        assert_eq!(
            import_bytes(&foreign[..n]).unwrap_err(),
            "error.draw2.cdx",
            "cut at {n}"
        );
    }
    // corruption: a byte changed, removed or inserted at random places
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = |n: usize| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % n as u64) as usize
    };
    let (mut read, mut refused) = (0, 0);
    for base in [&file, &foreign] {
        for _ in 0..4000 {
            let mut b = base.clone();
            for _ in 0..1 + next(4) {
                let at = 8 + next(b.len() - 8);
                match next(3) {
                    0 => b[at] = [0, 0xFF, 0x80, 1, 0x7F][next(5)],
                    1 => {
                        b.remove(at);
                    }
                    _ => b.insert(at, [0, 0xFF, 0x80, 4][next(4)]),
                }
            }
            match import_bytes(&b) {
                Ok((doc, _)) => {
                    assert_eq!(Document::from_json(&doc.to_json()).as_ref(), Ok(&doc));
                    read += 1;
                }
                Err(_) => refused += 1,
            }
        }
    }
    assert!(
        read > 100 && refused > 100,
        "{read} read, {refused} refused"
    );
}

#[test]
fn lengths_depth_and_structure_are_bounded() {
    let head = || {
        let mut f = b"VjCD0100".to_vec();
        f.extend([4, 3, 2, 1]);
        f.extend([0u8; 16]);
        f
    };
    let key = |f: &[u8]| import_bytes(f).unwrap_err();
    // a length longer than the file, in the 2-byte and the 4-byte form
    let mut f = head();
    f.extend([0x00, 0x80, 1, 0, 0, 0, 0x00, 0x02, 0xF0, 0xFF]);
    assert_eq!(key(&f), "error.draw2.cdx");
    f.truncate(28 + 6);
    f.extend([0x00, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]);
    assert_eq!(key(&f), "error.draw2.cdx");
    // a header with nothing after it, and a header cut short
    assert_eq!(key(&head()), "error.draw2.cdx");
    assert_eq!(key(&head()[..20]), "error.draw2.cdx");
    // a stray end, a property outside any object, a document inside a document, an object before the document
    for body in [
        vec![0, 0],
        vec![0x00, 0x02, 0, 0],
        vec![0x00, 0x80, 1, 0, 0, 0, 0x00, 0x80, 2, 0, 0, 0, 0, 0, 0, 0],
        vec![0x01, 0x80, 1, 0, 0, 0, 0, 0],
    ] {
        let mut f = head();
        f.extend(body);
        assert_eq!(key(&f), "error.draw2.cdx");
    }
    // nesting: as deep as the XML reader allows is read, one more is refused
    let nested = |levels: usize| {
        let mut f = head();
        f.extend([0x00, 0x80, 1, 0, 0, 0]);
        for _ in 0..levels {
            f.extend([0x02, 0x80, 1, 0, 0, 0]);
        }
        f.extend(vec![0; 2 * (levels + 1)]);
        import_bytes(&f)
    };
    assert!(nested(60).is_ok());
    assert_eq!(nested(100).unwrap_err(), "error.draw2.cdx");
    // the number of objects is bounded as well
    let mut f = head();
    f.extend([0x00, 0x80, 1, 0, 0, 0]);
    for _ in 0..mzdraw::xml::MAX_ELEMENTS + 1 {
        f.extend([0x08, 0x80, 0, 0, 0, 0, 0, 0]);
    }
    f.extend([0, 0]);
    assert_eq!(key(&f), "error.draw2.cdx");
    // a file with nothing to draw is an empty drawing
    let mut f = head();
    f.extend([0x00, 0x80, 1, 0, 0, 0, 0, 0]);
    let (d, _) = import_bytes(&f).unwrap();
    assert!(d.is_empty());
    // the magic alone does not make a CDX of a text
    assert!(import("VjCD0100").is_err());
}

fn base64_of(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = c
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            out.push(if i <= c.len() {
                A[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    out
}

/// The golden file of the area holds the CDX the writer makes of the fixtures, and a request carries it to the same document as
/// the CDXML text of the fixture.
#[test]
fn the_golden_cdx_is_what_the_writer_makes_and_a_request_reads_it() {
    let golden: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/data/log/cdx.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let cases = golden["cases"].as_array().unwrap();
    let fixtures = fixtures();
    for (case, fixture) in [(&cases[0], &fixtures[10].1), (&cases[1], &fixtures[9].1)] {
        assert!(
            case["request"]["import_base64"] == base64_of(&cdx_of(fixture)),
            "the golden file is not what the writer makes"
        );
        let request = |key: &str, value: &str, out: &str| {
            serde_json::json!({ key: value, "out": out }).to_string()
        };
        for out in ["mzdraw", "molfile", "cdxml"] {
            assert_eq!(
                mzdraw::replay::replay(&request(
                    "import_base64",
                    &base64_of(&cdx_of(fixture)),
                    out
                ))
                .unwrap(),
                mzdraw::replay::replay(&request("import", fixture, out)).unwrap(),
                "{out}"
            );
        }
    }
    assert_eq!(fixtures[10].0, "foreign");
}

fn head() -> Vec<u8> {
    let mut f = b"VjCD0100".to_vec();
    f.extend([4, 3, 2, 1]);
    f.extend([0u8; 16]);
    f
}

/// A document, a page and a text object with the given property of tag 0x0700, then the ends and the end mark.
fn with_string(string: &[u8]) -> Vec<u8> {
    let mut f = head();
    f.extend([
        0x00, 0x80, 1, 0, 0, 0, 0x01, 0x80, 2, 0, 0, 0, 0x06, 0x80, 3, 0, 0, 0,
    ]);
    property(&mut f, 0x0200, &[coord(1.0), coord(2.0)].concat());
    property(&mut f, 0x0700, string);
    f.extend([0; 8]);
    f
}

/// A string of runs with the given starts (font 3, face 0), then the text.
fn string_of(starts: &[u16], text: &[u8]) -> Vec<u8> {
    let mut s = (starts.len() as u16).to_le_bytes().to_vec();
    for st in starts {
        s.extend(st.to_le_bytes());
        s.extend([3, 0, 0, 0, 200, 0, 0, 0]);
    }
    s.extend(text);
    s
}

#[test]
fn a_hostile_string_is_refused_before_it_costs_anything() {
    // the starts of the runs jump between 0 and 65535: each run would copy 64 kB of a text that holds a few bytes (10 000 runs in
    // a 100 kB file); such a string is refused at once, and so is every string whose starts decrease, lie beyond the text or do
    // not begin at 0
    let hostile: Vec<u16> = (0..10_000)
        .map(|i| if i % 2 == 0 { 0 } else { 65535 })
        .collect();
    let t0 = std::time::Instant::now();
    for starts in [
        hostile,
        vec![0, 5, 3],
        vec![0, 9],
        vec![1, 2],
        vec![0; 7000],
    ] {
        let f = with_string(&string_of(&starts, b"abcd"));
        let e = import_bytes(&f).err();
        // a run list that is non-decreasing and inside the text is the one that is read
        let fine = starts.iter().all(|s| *s <= 4)
            && starts[0] == 0
            && starts.windows(2).all(|w| w[0] <= w[1]);
        assert_eq!(e.is_none(), fine, "{:?}", &starts[..starts.len().min(4)]);
        if !fine {
            assert_eq!(e, Some("error.draw2.cdx"));
        }
    }
    assert!(
        t0.elapsed().as_secs() < 2,
        "refused fast, not after the copies"
    );
    // more runs than the file may hold elements: refused before any is built
    let many = vec![0u16; 6000];
    let mut f = head();
    f.extend([0x00, 0x80, 1, 0, 0, 0]);
    for _ in 0..70 {
        f.extend([0x06, 0x80, 0, 0, 0, 0]);
        property(&mut f, 0x0200, &[coord(1.0), coord(2.0)].concat());
        property(&mut f, 0x0700, &string_of(&many, b""));
        f.extend([0, 0]);
    }
    f.extend([0, 0]);
    // 70 texts of 6000 runs: 420 000 elements, over the bound of the XML reader
    assert_eq!(import_bytes(&f).unwrap_err(), "error.draw2.cdx");
}

#[test]
fn a_listed_property_of_the_wrong_size_refuses_the_file_and_trailing_bytes_are_not_read_in_silence()
{
    let node = |tag: u16, data: &[u8]| {
        let mut f = head();
        f.extend([
            0x00, 0x80, 1, 0, 0, 0, 0x01, 0x80, 2, 0, 0, 0, 0x03, 0x80, 3, 0, 0, 0, 0x04, 0x80, 4,
            0, 0, 0,
        ]);
        property(&mut f, 0x0200, &[coord(1.0), coord(2.0)].concat());
        property(&mut f, tag, data);
        f.extend([0; 8]);
        f
    };
    assert!(import_bytes(&node(0x0402, &6i16.to_le_bytes())).is_ok());
    // an element number of 3 bytes, a position of 5, a charge of 0 bytes, a bond order of 6: refused, not defaulted
    for (tag, data) in [
        (0x0402, &[1u8, 2, 3][..]),
        (0x0200, &[0; 5]),
        (0x0421, &[]),
        (0x0600, &[0; 6]),
        (0x0400, &[1, 2, 3]),
    ] {
        assert_eq!(
            import_bytes(&node(tag, data)).unwrap_err(),
            "error.draw2.cdx",
            "{tag:#x}"
        );
    }
    // the charge of ChemDraw 8.0, four bytes with the charge in the high byte, is read
    let (d, _) = import_bytes(&node(0x0421, &0x0100_0000i32.to_le_bytes())).unwrap();
    assert_eq!(d.atoms[0].chg, 1);
    // after the end of the document: nothing, or the end mark of the file; any other byte is refused
    let ok = node(0x0402, &6i16.to_le_bytes());
    let mut with_mark = ok.clone();
    with_mark.extend([0, 0]);
    assert!(import_bytes(&with_mark).is_ok());
    for tail in [&[0u8][..], &[0, 0, 0], &[1, 2, 3], &[0x80, 0]] {
        let mut f = ok.clone();
        f.extend(tail);
        assert_eq!(import_bytes(&f).unwrap_err(), "error.draw2.cdx", "{tail:?}");
    }
}

#[test]
fn a_code_page_that_is_not_decoded_is_said() {
    let doc = |page: u16, text: &[u8]| {
        let mut f = head();
        f.extend([0x00, 0x80, 1, 0, 0, 0]);
        let mut fonts = vec![1, 0, 1, 0, 3, 0];
        fonts.extend(page.to_le_bytes());
        fonts.extend([1, 0, b'F']);
        property(&mut f, 0x0100, &fonts);
        f.extend([0x01, 0x80, 2, 0, 0, 0, 0x06, 0x80, 3, 0, 0, 0]);
        property(&mut f, 0x0200, &[coord(10.0), coord(20.0)].concat());
        property(&mut f, 0x0700, &string_of(&[0], text));
        f.extend([0; 8]);
        let (d, info) = import_bytes(&f).unwrap();
        (d.objects[0].text.clone(), info)
    };
    // 1252 gives its own characters for 0x80..=0x9F; ISO 8859-1 and 65001 are read; nothing is said for them
    for (page, text, want) in [
        (1252, &b"\x80 5 \x99"[..], "€ 5 ™"),
        (28591, b"\xE9", "é"),
        (65001, "β".as_bytes(), "β"),
    ] {
        let (got, info) = doc(page, text);
        assert_eq!(got, want);
        assert_eq!((info.code_page_runs, info.notes().len()), (0, 0), "{page}");
    }
    // Mac Roman (10000) or Shift-JIS (932) is read as Latin-1 and the page says so
    for page in [10000, 932] {
        let (_, info) = doc(page, b"ab");
        assert_eq!(info.code_page_runs, 1, "{page}");
        assert_eq!(info.notes(), [("draw2.import.codePage", 1)], "{page}");
    }
}
