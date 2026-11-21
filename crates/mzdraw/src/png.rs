//! PNG as a carrier of the drawing. The pixels are made where a rasteriser is (the page draws the SVG of this crate on a canvas);
//! this module writes into the file what makes it more than a picture: the document (`mzdraw/1`, an `iTXt` chunk with the keyword
//! `mzdraw`), so that the PNG opens again as an editable drawing, and the resolution (`pHYs`), so that it prints at the size of
//! its style. Reading walks the chunks with every length checked: a damaged or hostile file is refused, never a panic.

use crate::doc::Document;
use crate::settings::Style;

const SIGNATURE: &[u8] = &[137, 80, 78, 71, 13, 10, 26, 10];
/// The start of the data of an `iTXt` chunk that carries the document: the keyword (the slug of the name in use or of an earlier
/// one) and the five bytes of an uncompressed text with no language (the NUL after the keyword, two flags, two empty strings).
fn head(slug: &str) -> Vec<u8> {
    [slug.as_bytes(), &[0; 5]].concat()
}

/// The length of the head of a chunk that carries a document written under the name in use or an earlier one; `None` for any other.
fn carries(data: &[u8]) -> Option<usize> {
    crate::brand::slugs()
        .map(head)
        .find(|h| data.starts_with(h))
        .map(|h| h.len())
}
/// The longest side of a PNG the page is asked to draw, in pixels.
pub const MAX_SIDE: f64 = 8192.0;
/// Pixels per inch of a saved PNG (less for a drawing that would pass `MAX_SIDE`).
pub const DPI: f64 = 300.0;

/// What the page needs to draw the PNG of a document: the SVG to rasterise, the size in pixels and the paper colour.
pub struct Plan {
    pub svg: String,
    pub width: u32,
    pub height: u32,
    pub paper: &'static str,
    /// Pixels per metre, for `finish`.
    pub density: u32,
}

/// The plan of the PNG of a document in a style: 300 pixels per inch of the SVG, on white.
pub fn plan(doc: &Document, style: &Style) -> Plan {
    plan_with(doc, style, &[])
}

/// `plan` with the calculated values the editor holds.
pub fn plan_with(doc: &Document, style: &Style, measured: &[crate::values::Entry]) -> Plan {
    let (svg, w, h) = crate::svg::drawn_with(doc, style, measured);
    // points to pixels
    let k = (DPI / 72.0).min(MAX_SIDE / w.max(h).max(1.0));
    Plan {
        svg,
        width: (w * k).round().max(1.0) as u32,
        height: (h * k).round().max(1.0) as u32,
        paper: "#ffffff",
        density: (k * 72.0 / 0.0254).round() as u32,
    }
}

fn crc(kind: &[u8], data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in kind.iter().chain(data) {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = (c >> 1) ^ (0xEDB8_8320 & (c & 1).wrapping_neg());
        }
    }
    !c
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    out.extend(kind);
    out.extend(data);
    out.extend(crc(kind, data).to_be_bytes());
}

/// One chunk: its offset in the file, its type, its data.
type Chunk<'a> = (usize, &'a [u8], &'a [u8]);

/// The chunks of a PNG up to `IEND`; `None` for bytes that are not a whole PNG.
fn chunks(png: &[u8]) -> Option<Vec<Chunk<'_>>> {
    let mut at = png.strip_prefix(SIGNATURE).map(|_| SIGNATURE.len())?;
    let mut out = Vec::new();
    loop {
        let len = u32::from_be_bytes(png.get(at..at + 4)?.try_into().ok()?) as usize;
        let kind = png.get(at + 4..at + 8)?;
        let data = png.get(at + 8..(at + 8).checked_add(len)?)?;
        png.get(at + 8 + len..at + 12 + len)?;
        out.push((at, kind, data));
        if kind == b"IEND" {
            return Some(out);
        }
        at += 12 + len;
    }
}

/// The PNG with the document inside and, when the file says nothing about it, the resolution `density` (pixels per metre).
/// `None` for bytes that are not a PNG.
pub fn finish(png: &[u8], doc: &Document, density: u32) -> Option<Vec<u8>> {
    let all = chunks(png)?;
    let mut out = SIGNATURE.to_vec();
    let sized = all.iter().any(|c| c.1 == b"pHYs");
    for (i, &(_, kind, data)) in all.iter().enumerate() {
        if kind == b"IEND" && !doc.is_empty() {
            chunk(
                &mut out,
                b"iTXt",
                &[&head(crate::brand::SLUG)[..], doc.to_json().as_bytes()].concat(),
            );
        }
        // a document the file already carries is replaced
        if !(kind == b"iTXt" && carries(data).is_some()) {
            chunk(&mut out, kind.try_into().ok()?, data);
        }
        // the resolution goes before the pixels: right after the header
        if i == 0 && !sized {
            let d = density.to_be_bytes();
            chunk(&mut out, b"pHYs", &[&d[..], &d[..], &[1]].concat());
        }
    }
    Some(out)
}

/// The document a PNG carries: `None` when the bytes are not a PNG or carry none, else the document or the key of what is wrong.
pub fn carried(png: &[u8]) -> Option<Result<Document, &'static str>> {
    let all = chunks(png)?;
    let (&(at, kind, data), start) = all
        .iter()
        .find_map(|c| Some((c, carries(c.2).filter(|_| c.1 == b"iTXt")?)))?;
    let stored = png.get(at + 8 + data.len()..at + 12 + data.len());
    let text = std::str::from_utf8(&data[start..]).ok();
    Some(match text {
        Some(t) if stored == Some(&crc(kind, data).to_be_bytes()[..]) => Document::from_json(t),
        _ => Err("error.draw2.document"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic PNG whose document sits under the keyword `slug`.
    fn file(slug: &str, doc: &Document) -> Vec<u8> {
        let mut f = SIGNATURE.to_vec();
        chunk(&mut f, b"IHDR", &[0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0, 0, 0]);
        chunk(
            &mut f,
            b"iTXt",
            &[&head(slug)[..], doc.to_json().as_bytes()].concat(),
        );
        chunk(&mut f, b"IDAT", &[120, 156, 99, 0, 1, 0, 0, 5, 0, 1]);
        chunk(&mut f, b"IEND", &[]);
        f
    }

    #[test]
    fn a_document_under_an_earlier_keyword_is_read_and_replaced_by_one_under_the_current_keyword() {
        let id = crate::brand::DOCUMENT_ID;
        let doc = Document::from_json(&format!(
            r#"{{"format":"{id}","next":3,"atoms":[{{"id":1,"el":"N","x":0.0,"y":0.0}}],"bonds":[]}}"#
        ))
        .unwrap();
        let old = file("legacy", &doc);
        assert_eq!(carried(&old).unwrap().unwrap().to_json(), doc.to_json());
        let now = finish(&old, &doc, 11811).unwrap();
        assert_eq!(carried(&now).unwrap().unwrap().to_json(), doc.to_json());
        let has = |name: &str| now.windows(name.len()).any(|w| w == name.as_bytes());
        assert!(has(crate::brand::SLUG));
        assert!(
            !has("legacy"),
            "the chunk of the earlier name is replaced, not kept"
        );
        assert!(carried(&file("other", &doc)).is_none());
    }
}
