//! Binary CDX, the native file of ChemDraw, read from the public CDX SDK documentation (the format overview, the data types and the
//! object and property pages of the IUPAC copy; no code of any other program). The tagged records become the element tree that
//! the CDXML reader works on (`cdxml::from_tree`), so every object that reader understands is read by the same mapping and
//! everything else is counted and said, as for CDXML. Reading only; writing CDX is not built.
//!
//! Layout (little-endian): the header `VjCD0100`, four reserved bytes and sixteen zero bytes, then tagged items: an object is
//! a tag with bit 15 set, a 4-byte id, its items, and a closing `00 00`; a property is a tag with bit 15 clear, a 2-byte length
//! (`FFFF`: a 4-byte length follows) and the data. A property of an unknown tag is skipped by its length; an object of an
//! unknown tag is kept as an element the CDXML mapping counts. Every length is checked against the file, the nesting and the
//! number of objects are bounded, and a damaged or cut file is refused, never a panic.
//!
//! Not stated on the pages read, and so chosen here: a CDXString is decoded as UTF-8 when the font of its run has the code
//! page 65001 (the page of the font table defers the list of code pages to a page that was not found), as Latin-1 otherwise;
//! `ArrowHeadHead` and the other arrowhead properties are not read (the page of the property was not available, and the
//! CDXML mapping takes the ends of an arrow from `Tail3D` and `Head3D` only).

use crate::xml::{Element, MAX_DEPTH, MAX_ELEMENTS};
use mzlab_mol::fixed;

const MAGIC: &[u8] = b"VjCD0100";
/// Magic, the four reserved bytes and the sixteen zero bytes.
const HEADER: usize = 28;
/// The error, an i18n key.
pub(crate) const KEY: &str = "error.draw2.cdx";

/// Whether the bytes start as a CDX file.
pub(crate) fn is_cdx(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

/// The CDXML name of an object tag; `unknown` for a tag the pages do not list (the mapping counts it).
fn object_name(tag: u16) -> &'static str {
    match tag {
        0x8000 => "CDXML",
        0x8001 => "page",
        0x8002 => "group",
        0x8003 => "fragment",
        0x8004 => "n",
        0x8005 => "b",
        0x8006 => "t",
        0x8007 => "graphic",
        0x8008 => "curve",
        0x8009 => "embeddedobject",
        0x800A => "altgroup",
        0x800B => "templategrid",
        0x800C => "regnum",
        0x800D => "scheme",
        0x800E => "step",
        0x8010 => "spectrum",
        0x8011 => "objecttag",
        0x8013 => "sequence",
        0x8014 => "crossreference",
        0x8015 => "splitter",
        0x8016 => "table",
        0x8017 => "bracketedgroup",
        0x8018 => "bracketattachment",
        0x8019 => "crossingbond",
        0x8020 => "border",
        0x8021 => "geometry",
        0x8022 => "constraint",
        0x8023 => "tlcplate",
        0x8024 => "tlclane",
        0x8025 => "tlcspot",
        0x8026 => "chemicalproperty",
        0x8027 => "arrow",
        _ => "unknown",
    }
}

/// An enumerated value as its CDXML name; the number itself when the pages list no name for it.
fn named(names: &[&str], v: i64) -> String {
    match usize::try_from(v).ok().and_then(|i| names.get(i)) {
        Some(n) => (*n).to_string(),
        None => v.to_string(),
    }
}

const NODE_TYPES: &[&str] = &[
    "Unspecified",
    "Element",
    "ElementList",
    "ElementListNickname",
    "Nickname",
    "Fragment",
    "Formula",
    "GenericNickname",
    "AnonymousAlternativeGroup",
    "NamedAlternativeGroup",
    "MultiAttachment",
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
    "HollowWedgeBegin",
    "HollowWedgeEnd",
    "WavyWedgeBegin",
    "WavyWedgeEnd",
    "Dot",
    "DashDot",
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
    "Racemic",
    "Absolute",
    "Relative",
    "LonePair",
];
/// The bits of a bond order, lowest first, as the CDXML names of the page.
const ORDERS: [&str; 16] = [
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "0.5",
    "1.5",
    "2.5",
    "3.5",
    "4.5",
    "5.5",
    "dative",
    "ionic",
    "hydrogen",
    "threecenter",
];
/// The names of the values of an arrow type, as bit flags (the value of the pages: 0 is a headless line).
const ARROWS: [(i64, &str); 8] = [
    (0, "NoHead"),
    (1, "HalfHead"),
    (2, "FullHead"),
    (4, "Resonance"),
    (8, "Equilibrium"),
    (16, "Hollow"),
    (32, "RetroSynthetic"),
    (64, "NoGo"),
];

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        b.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// An integer of 1, 2 or 4 bytes, signed or not, by the size of the field (the pages ask readers to look at the size).
fn int(data: &[u8], signed: bool) -> Option<i64> {
    Some(match *data {
        [a] if signed => i64::from(a as i8),
        [a] => i64::from(a),
        [a, b] if signed => i64::from(i16::from_le_bytes([a, b])),
        [a, b] => i64::from(u16::from_le_bytes([a, b])),
        [a, b, c, d] if signed => i64::from(i32::from_le_bytes([a, b, c, d])),
        [a, b, c, d] => i64::from(u32::from_le_bytes([a, b, c, d])),
        _ => return None,
    })
}

/// The CDXCoordinates of a field, in the order of the file, as points (a CDX unit is 1/65536 point), four decimals.
fn coords(data: &[u8], n: usize) -> Option<Vec<String>> {
    if data.len() != 4 * n {
        return None;
    }
    Some(
        (data.chunks_exact(4))
            .map(|c| {
                let v = i32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                fixed(f64::from(v) / 65536.0, 4)
            })
            .collect(),
    )
}

/// The value of an arrow type: a head name, plus `NoGo` when that bit is set (CDXML lists no combination on the pages read).
fn arrow_type(v: i64) -> String {
    let heads = v & !(64 | 128);
    match ARROWS.iter().find(|a| a.0 == heads) {
        Some(a) if v & !64 == heads => a.1.to_string(),
        _ => v.to_string(),
    }
}

/// The order of a bond: the names of its bits, joined by blanks; `None` for 0xFFFF (unspecified, omitted in CDXML).
fn order(v: i64) -> Option<String> {
    if v == 0xFFFF || v <= 0 {
        return None;
    }
    let names: Vec<&str> = (0..16)
        .filter(|i| (v >> i) & 1 == 1)
        .map(|i| ORDERS[i])
        .collect();
    Some(names.join(" "))
}

/// The characters of the bytes 0x80..=0x9F in code page 1252 (Latin-1 has controls there); a byte the page leaves undefined is the
/// control of the same value.
const CP1252: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
];

/// What is gathered while reading: the code pages of the fonts of the font table (font id, code page).
#[derive(Default)]
struct Fonts(Vec<(u16, u16)>);

impl Fonts {
    /// The font table: platform, count, then per font its id, code page, name length and name.
    fn table(&mut self, data: &[u8]) {
        let n = u16_at(data, 2).unwrap_or(0);
        let mut at = 4;
        for _ in 0..n {
            let (Some(id), Some(page), Some(len)) =
                (u16_at(data, at), u16_at(data, at + 2), u16_at(data, at + 4))
            else {
                return;
            };
            self.0.push((id, page));
            at += 6 + usize::from(len);
        }
    }

    /// The text of a run of bytes in the font `id`, and whether its code page is one this reader cannot decode (UTF-8 65001,
    /// 1252, ISO 8859-1 28591 and an undefined page 0 are read; any other is read as Latin-1 and reported).
    fn decode(&self, id: u16, bytes: &[u8]) -> (String, bool) {
        match self.0.iter().find(|f| f.0 == id).map(|f| f.1) {
            Some(65001) => (String::from_utf8_lossy(bytes).into_owned(), false),
            Some(1252) => (
                bytes
                    .iter()
                    .map(|b| match *b {
                        0x80..=0x9F => CP1252[usize::from(*b - 0x80)],
                        b => char::from(b),
                    })
                    .collect(),
                false,
            ),
            page => (
                bytes.iter().map(|b| char::from(*b)).collect(),
                !matches!(page, None | Some(0) | Some(28591)),
            ),
        }
    }
}

/// A CDXString as `s` elements, one per style run: the count of runs, then per run its start, font, face, size and color (ten
/// bytes), then the text; with no run the text is one run. Also the number of runs in a code page that is not decoded. `None`
/// for a string that is cut short, whose first run does not start at 0, whose starts decrease or lie beyond the text, or whose
/// runs would not fit in `budget` more elements: the file is refused (the runs cut disjoint pieces of the text, so what is
/// decoded is never more than what the file holds).
fn text_runs(data: &[u8], fonts: &Fonts, budget: usize) -> Option<(Vec<Element>, usize)> {
    let runs = usize::from(u16_at(data, 0)?);
    if runs > budget {
        return None;
    }
    let text = data.get(2usize.checked_add(runs.checked_mul(10)?)?..)?;
    let mut starts = Vec::with_capacity(runs + 1);
    for i in 0..runs {
        starts.push(usize::from(u16_at(data, 2 + 10 * i)?));
    }
    starts.push(text.len());
    if (runs > 0 && starts[0] != 0) || starts.windows(2).any(|w| w[0] > w[1]) {
        return None;
    }
    let mut pieces = Vec::new();
    let mut unknown = 0;
    let mut piece = |bytes: &[u8], font: Option<(u16, u16)>| {
        let mut s = Element {
            name: "s".into(),
            ..Element::default()
        };
        if let Some((font, face)) = font {
            s.attrs.push(("font".into(), font.to_string()));
            s.attrs.push(("face".into(), face.to_string()));
        }
        let (t, odd) = fonts.decode(font.map_or(0, |f| f.0), bytes);
        s.text = t;
        unknown += usize::from(odd);
        pieces.push(s);
    };
    if runs == 0 {
        piece(text, None);
    }
    for i in 0..runs {
        let at = 2 + 10 * i;
        let font = (u16_at(data, at + 2)?, u16_at(data, at + 4)?);
        piece(&text[starts[i]..starts[i + 1]], Some(font));
    }
    Some((pieces, unknown))
}

/// The attribute CDXML gives a listed property: its name and value; `Some(None)` when it says nothing to write (an unspecified bond
/// order); `None` for a size the pages do not give the property (the file is refused).
fn attribute(tag: u16, data: &[u8]) -> Option<Option<(&'static str, String)>> {
    Some(Some(match tag {
        0x0012 => ("SupersededBy", int(data, false)?.to_string()),
        // 2D position: y, x -> "x y"
        0x0200 => {
            let c = coords(data, 2)?;
            ("p", format!("{} {}", c[1], c[0]))
        }
        // rectangle: top, left, bottom, right -> "left top right bottom" (for a graphic, the pair of points the page says)
        0x0204 => {
            let c = coords(data, 4)?;
            (
                "BoundingBox",
                format!("{} {} {} {}", c[1], c[0], c[3], c[2]),
            )
        }
        // 3D point: z, y, x -> "x y z"; a 2D point is read as such
        0x0207 | 0x0208 => {
            let name = if tag == 0x0207 { "Head3D" } else { "Tail3D" };
            if data.len() == 12 {
                let c = coords(data, 3)?;
                (name, format!("{} {} {}", c[2], c[1], c[0]))
            } else {
                let c = coords(data, 2)?;
                (name, format!("{} {}", c[1], c[0]))
            }
        }
        0x0400 => ("NodeType", named(NODE_TYPES, int(data, true)?)),
        0x0402 => ("Element", int(data, true)?.to_string()),
        0x0420 => ("Isotope", int(data, true)?.to_string()),
        // a charge: one byte; ChemDraw 8.0 wrote four, the charge being the high byte
        0x0421 if data.len() == 4 => ("Charge", (int(data, true)? >> 24).to_string()),
        0x0421 => ("Charge", int(data, true)?.to_string()),
        0x0422 => ("Radical", int(data, false)?.to_string()),
        0x0600 => match order(int(data, false)?) {
            Some(o) => ("Order", o),
            None => return Some(None),
        },
        0x0601 => ("Display", named(DISPLAYS, int(data, true)?)),
        0x0604 => ("B", int(data, false)?.to_string()),
        0x0605 => ("E", int(data, false)?.to_string()),
        0x0A00 => ("GraphicType", named(GRAPHICS, int(data, true)?)),
        0x0A02 => ("ArrowType", arrow_type(int(data, true)?)),
        0x0A07 => ("SymbolType", named(SYMBOLS, int(data, true)?)),
        _ => return Some(None),
    }))
}

/// The tags `attribute` reads.
const LISTED: [u16; 17] = [
    0x0012, 0x0200, 0x0204, 0x0207, 0x0208, 0x0400, 0x0402, 0x0420, 0x0421, 0x0422, 0x0600, 0x0601,
    0x0604, 0x0605, 0x0A00, 0x0A02, 0x0A07,
];

/// What a CDX file gave: the element tree, and what was passed over.
pub(crate) struct Tree {
    pub root: Element,
    /// Properties of tags the pages read do not list (styles, colours, widths ...), skipped by their length.
    pub props_skipped: usize,
    /// Text runs of a font whose code page is not decoded (read as Latin-1).
    pub code_page_runs: usize,
}

/// The element tree of a CDX file: the document object is the root, named `CDXML` (objects with no document around them are
/// given one). `None` for anything that is not a whole, well-formed file within the bounds: cut short, an item longer than the
/// file, nested too deep, too many objects, a known property of a size the pages do not give, a damaged string, or bytes after
/// the end of the document other than the two-byte end mark of the file.
pub(crate) fn tree(bytes: &[u8]) -> Option<Tree> {
    if !is_cdx(bytes) || bytes.len() < HEADER {
        return None;
    }
    let mut at = HEADER;
    // the open objects, innermost last; the bottom one collects the document
    let mut open = vec![Element::default()];
    let mut count = 0;
    let (mut props_skipped, mut code_page_runs) = (0, 0);
    let mut fonts = Fonts::default();
    let end_ok = |at: usize| bytes[at..].is_empty() || bytes[at..] == [0, 0];
    loop {
        let tag = u16_at(bytes, at)?;
        at += 2;
        if tag == 0 {
            if open.len() == 1 {
                // the end mark of the file, for a file whose objects are not inside a document object (the example of the
                // overview page is one): they are read as the content of one
                let mut top = open.pop()?;
                top.name = "CDXML".to_string();
                return (!top.children.is_empty() && at == bytes.len()).then_some(Tree {
                    root: top,
                    props_skipped,
                    code_page_runs,
                });
            }
            let done = open.pop()?;
            if open.len() == 1 && done.name == "CDXML" {
                // the document is closed: nothing may follow but the end mark of the file
                return end_ok(at).then_some(Tree {
                    root: done,
                    props_skipped,
                    code_page_runs,
                });
            }
            open.last_mut()?.children.push(done);
        } else if tag & 0x8000 != 0 {
            let id = u32_at(bytes, at)?;
            at += 4;
            count += 1;
            if count > MAX_ELEMENTS || open.len() > MAX_DEPTH {
                return None;
            }
            // a document is a top-level object
            if tag == 0x8000 && open.len() > 1 {
                return None;
            }
            open.push(Element {
                name: object_name(tag).to_string(),
                attrs: vec![("id".to_string(), id.to_string())],
                ..Element::default()
            });
        } else {
            let mut len = usize::from(u16_at(bytes, at)?);
            at += 2;
            if len == 0xFFFF {
                len = usize::try_from(u32_at(bytes, at)?).ok()?;
                at += 4;
            }
            let data = bytes.get(at..at.checked_add(len)?)?;
            at += len;
            // a property belongs to an object: something is open besides the bottom collector
            if open.len() < 2 {
                return None;
            }
            let top = open.last_mut()?;
            match tag {
                // the font table gives the code pages of the fonts
                0x0100 => fonts.table(data),
                0x0700 => {
                    // the cap is checked before any run is built
                    let (runs, odd) = text_runs(data, &fonts, MAX_ELEMENTS - count)?;
                    count += runs.len();
                    code_page_runs += odd;
                    top.children.extend(runs);
                }
                t if LISTED.contains(&t) => {
                    let found = attribute(t, data)?;
                    top.attrs.extend(found.map(|(k, v)| (k.to_string(), v)));
                }
                _ => props_skipped += 1,
            }
        }
    }
}
