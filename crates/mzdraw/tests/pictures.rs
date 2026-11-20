//! Pictures that carry their drawing: the SVG and the PNG mzDraw saves open again as the document they were made from.

use mzdraw::io::{import, import_bytes};
use mzdraw::{png, svg, Document, Editor};

/// A drawing with what an XML or JSON carrier could damage: text with markup characters, quotes and non-ASCII, unknown fields.
fn tricky() -> Document {
    Document::from_json(
        r#"{"format":"mzdraw/1","next":9,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0,"ann":{"level":"2a"}},{"id":2,"el":"N","x":1.0,"y":0.5,"chg":1,"lab":""},{"id":3,"el":"C","x":2.0,"y":0.0,"lab":"R1"}],"bonds":[{"id":4,"a":1,"b":2,"o":1,"s":1},{"id":5,"a":2,"b":3,"o":1}],"objects":[{"id":6,"t":"arrow","x":3.0,"y":0.0,"x2":5.0,"y2":0.0,"text":"H2 <Pd/C> & \"heat\" </metadata> Δ"},{"id":7,"t":"text","x":3.0,"y":1.0,"text":"a&amp;b &lt; ]]> 30 °C"},{"id":8,"t":"plus","x":6.0,"y":0.0}],"meta":{"title":"<synthetic>"}}"#,
    )
    .unwrap()
}

fn crc(bytes: &[u8]) -> u32 {
    // table-free reference, written apart from the one of the crate
    let mut c = 0xFFFF_FFFFu32;
    for &b in bytes {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 {
                (c >> 1) ^ 0xEDB8_8320
            } else {
                c >> 1
            };
        }
    }
    c ^ 0xFFFF_FFFF
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut body = kind.to_vec();
    body.extend(data);
    let mut out = (data.len() as u32).to_be_bytes().to_vec();
    out.extend(&body);
    out.extend(crc(&body).to_be_bytes());
    out
}

/// A synthetic PNG file: the chunks a canvas writes (the pixels are not looked at by this crate).
fn canvas_png(extra: &[Vec<u8>]) -> Vec<u8> {
    let mut f = vec![137, 80, 78, 71, 13, 10, 26, 10];
    f.extend(chunk(b"IHDR", &[0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0, 0, 0]));
    for c in extra {
        f.extend(c);
    }
    f.extend(chunk(b"IDAT", &[120, 156, 99, 0, 1, 0, 0, 5, 0, 1]));
    f.extend(chunk(b"IEND", &[]));
    f
}

/// The chunk types of a PNG, in order, with the offset of each (a strict walk of its own).
fn kinds(png: &[u8]) -> Vec<(String, usize)> {
    let (mut at, mut out) = (8, Vec::new());
    while at < png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        let stored = u32::from_be_bytes(png[at + 8 + len..at + 12 + len].try_into().unwrap());
        assert_eq!(
            stored,
            crc(&png[at + 4..at + 8 + len]),
            "the CRC of every chunk is right"
        );
        out.push((String::from_utf8(png[at + 4..at + 8].to_vec()).unwrap(), at));
        at += 12 + len;
    }
    out
}

#[test]
fn the_svg_carries_its_document() {
    let d = tricky();
    let plain = svg::to_svg(&d);
    let file = svg::carrying(plain.clone(), &d);
    // the picture is the same: one line was added after the opening tag, and it is well-formed text content
    let lines: Vec<&str> = file.lines().collect();
    assert!(lines[1].starts_with("<metadata id=\"mzdraw\">") && lines[1].ends_with("</metadata>"));
    assert_eq!([&lines[..1], &lines[2..]].concat().join("\n") + "\n", plain);
    let inside = &lines[1]["<metadata id=\"mzdraw\">".len()..lines[1].len() - "</metadata>".len()];
    assert!(!inside.contains('<') && !inside.contains('>'), "{inside}");
    // it opens as the same document, byte for byte, unknown fields included
    assert_eq!(svg::carried(&file).unwrap().unwrap().to_json(), d.to_json());
    let (back, info) = import(&file).unwrap();
    assert_eq!((back.to_json(), info.format), (d.to_json(), "svg"));
    // what the editor saves is the carrying SVG, in any style, and opening it adds the drawing as one undo step
    let mut e = Editor::default();
    e.import("N[C@@H](C)C(=O)O").unwrap();
    e.set_style("acs-1996").unwrap();
    let saved = e.save("svg").unwrap();
    let mut other = Editor::default();
    assert_eq!(other.import(&saved).unwrap().format, "svg");
    assert_eq!(other.doc().to_json(), e.doc().to_json());
    other.undo();
    assert!(other.doc().is_empty());
    // an empty drawing carries nothing; an SVG of another program is refused in words; a damaged document says so
    assert!(!Editor::default().save("svg").unwrap().contains("metadata"));
    assert_eq!(
        import("<svg xmlns=\"http://www.w3.org/2000/svg\"><circle r=\"4\"/></svg>").unwrap_err(),
        "error.draw2.svgPlain"
    );
    assert_eq!(
        import("<?xml version=\"1.0\"?><svg></svg>").unwrap_err(),
        "error.draw2.svgPlain"
    );
    assert_eq!(
        import(&file.replace("mzdraw/1", "mzdraw/9")).unwrap_err(),
        "error.draw2.documentNewer"
    );
    assert_eq!(
        import(
            &file
                .replace("&quot;", "")
                .replace("\"next\":9", "\"next\":2")
        )
        .unwrap_err(),
        "error.draw2.document"
    );
    assert_eq!(
        import("<metadata id=\"mzdraw\">{\"format\"").unwrap_err(),
        "error.draw2.svgPlain"
    );
}

#[test]
fn the_png_carries_its_document_and_its_resolution() {
    let d = tricky();
    let file = png::finish(&canvas_png(&[]), &d, 11811).unwrap();
    // a valid PNG: resolution right after the header, the document right before the end, the pixels untouched
    let order: Vec<String> = kinds(&file).into_iter().map(|k| k.0).collect();
    assert_eq!(order, ["IHDR", "pHYs", "IDAT", "iTXt", "IEND"]);
    let at = kinds(&file)[1].1;
    assert_eq!(
        &file[at + 8..at + 17],
        [0, 0, 0x2E, 0x23, 0, 0, 0x2E, 0x23, 1],
        "300 dpi, in pixels per metre"
    );
    assert!(file
        .windows(10)
        .any(|w| w == [120, 156, 99, 0, 1, 0, 0, 5, 0, 1]));
    // it opens as the same document
    assert_eq!(png::carried(&file).unwrap().unwrap().to_json(), d.to_json());
    let (back, info) = import_bytes(&file).unwrap();
    assert_eq!((back.to_json(), info.format), (d.to_json(), "png"));
    let mut e = Editor::default();
    e.import_bytes(&file).unwrap();
    assert_eq!((e.doc().atoms.len(), e.doc().objects.len()), (3, 3));
    // saving again replaces the document and keeps the resolution the file has
    let mut small = Editor::default();
    small.import("CO").unwrap();
    let again = png::finish(&file, small.doc(), 1).unwrap();
    let order: Vec<String> = kinds(&again).into_iter().map(|k| k.0).collect();
    assert_eq!(order, ["IHDR", "pHYs", "IDAT", "iTXt", "IEND"]);
    assert_eq!(png::carried(&again).unwrap().unwrap().atoms.len(), 2);
    assert_eq!(&again[at + 8..at + 17], &file[at + 8..at + 17]);
    // an empty drawing carries nothing; a PNG of another program, or with a text of its own, is refused in words
    let empty = png::finish(&canvas_png(&[]), &Document::default(), 11811).unwrap();
    assert!(png::carried(&empty).is_none());
    assert_eq!(import_bytes(&empty).unwrap_err(), "error.draw2.pngPlain");
    let foreign = canvas_png(&[
        chunk(b"iTXt", b"Comment\0\0\0\0\0made elsewhere"),
        chunk(b"tEXt", b"mzdraw\0{}"),
    ]);
    assert_eq!(import_bytes(&foreign).unwrap_err(), "error.draw2.pngPlain");
    // bytes that are not a PNG are not finished; a text file still goes the text way
    assert!(png::finish(b"GIF89a", &d, 1).is_none());
    assert!(png::finish(&file[..file.len() - 5], &d, 1).is_none());
    assert_eq!(import_bytes(b"CCO").unwrap().0.atoms.len(), 3);
    assert_eq!(
        import_bytes(&[0xFF, 0xFE, 0x00]).unwrap_err(),
        "error.draw2.import"
    );
}

#[test]
fn the_plan_of_a_png_is_300_dpi_of_the_svg_within_a_bound() {
    let mut e = Editor::default();
    e.import("c1ccccc1").unwrap();
    let plan = e.png_plan();
    let size = |name: &str| -> f64 {
        let t = plan.svg.split(&format!("{name}=\"")).nth(1).unwrap();
        t[..t.find("pt\"").unwrap()].parse().unwrap()
    };
    // the nearest whole pixel: 128.4 pt is 535 px, whatever the order of the float operations
    assert_eq!(plan.width, (size("width") * 300.0 / 72.0).round() as u32);
    assert_eq!(plan.height, (size("height") * 300.0 / 72.0).round() as u32);
    assert_eq!((size("height"), plan.height), (128.4, 535));
    assert_eq!((plan.density, plan.paper), (11811, "#ffffff"));
    assert!(
        !plan.svg.contains("metadata"),
        "the picture to rasterise is the plain SVG"
    );
    // a very wide drawing is drawn at fewer pixels per inch, and says so in the file
    e.import(&"C".repeat(400)).unwrap();
    let wide = e.png_plan();
    assert!(wide.width.max(wide.height) <= png::MAX_SIDE as u32 && wide.density < 11811);
    assert!(wide.width > 4000, "{}", wide.width);
    // nothing drawn: the empty SVG is one point wide, which is four pixels at 300 per inch
    let none = Editor::default().png_plan();
    assert_eq!((none.width, none.height, none.density), (4, 4, 11811));
}

#[test]
fn a_damaged_png_is_refused_or_read_but_never_a_panic() {
    let d = tricky();
    let good = png::finish(&canvas_png(&[]), &d, 11811).unwrap();
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = |n: usize| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % n as u64) as usize
    };
    let (mut read, mut refused) = (0, 0);
    for round in 0..4000 {
        let mut b = good.clone();
        match round % 4 {
            0 => b.truncate(next(b.len())),
            1 => {
                let at = next(b.len());
                b[at] ^= 1 << next(8);
            }
            2 => {
                // a length field that points far outside the file
                let at = 8 + next(b.len() - 12);
                b[at..at + 4].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xF0 | next(16) as u8]);
            }
            _ => {
                let at = next(b.len());
                b.insert(at, next(256) as u8);
            }
        }
        match import_bytes(&b) {
            // one flipped bit in the document is caught by the CRC: what is read is the document that was saved
            Ok((doc, _)) => {
                assert_eq!(doc.to_json(), d.to_json());
                read += 1;
            }
            Err(_) => refused += 1,
        }
        let _ = png::finish(&b, &d, 1);
    }
    assert!(
        read > 100 && refused > 1000,
        "{read} read, {refused} refused"
    );
}
