//! PNG as a carrier of the drawing. The pixels are made where a rasteriser is (the page draws the SVG of this crate on a canvas);
//! this module writes into the file what makes it more than a picture: the document (`mzdraw/1`, an `iTXt` chunk with the keyword
//! `mzdraw`), so that the PNG opens again as an editable drawing, and the resolution (`pHYs`), so that it prints at the size of
//! its style. Reading walks the chunks with every length checked: a damaged or hostile file is refused, never a panic.

use crate::doc::Document;
use crate::settings::Style;

const SIGNATURE: &[u8] = &[137, 80, 78, 71, 13, 10, 26, 10];
/// Keyword, its end, no compression (flag and method), no language, no translated keyword.
const HEAD: &[u8] = b"mzdraw\0\0\0\0\0";
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
    let (svg, w, h) = crate::svg::drawn(doc, style);
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
                &[HEAD, doc.to_json().as_bytes()].concat(),
            );
        }
        // a document the file already carries is replaced
        if !(kind == b"iTXt" && data.starts_with(HEAD)) {
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
    let &(at, kind, data) = all
        .iter()
        .find(|c| c.1 == b"iTXt" && c.2.starts_with(HEAD))?;
    let stored = png.get(at + 8 + data.len()..at + 12 + data.len());
    let text = std::str::from_utf8(&data[HEAD.len()..]).ok();
    Some(match text {
        Some(t) if stored == Some(&crc(kind, data).to_be_bytes()[..]) => Document::from_json(t),
        _ => Err("error.draw2.document"),
    })
}
