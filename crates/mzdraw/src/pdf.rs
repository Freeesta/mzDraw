//! The display list as a PDF file (ISO 32000-1, PDF 1.7): one page of the size of the drawing in points on a white ground, lines
//! and polygons as path operators, text as text (the base-14 font Helvetica, WinAnsi encoding), element colours as RGB. The
//! document (`mzdraw/1`) travels inside as an embedded file (`mzdraw.json`, in the name tree `/EmbeddedFiles` of the catalog) and
//! so does its V3000 molfile (`mzdraw.mol`), so the PDF opens again as an editable drawing. Streams are stored, not compressed:
//! no compressor is needed and the bytes are the same on every build (no date; `/ID` is a hash of the content).
//! Reading back looks for `mzdraw.json` through the cross-reference table; every offset and length is checked, so a damaged or
//! hostile file is refused, never a panic. A PDF of another program has no such file and is refused in words.

use crate::brand::{attachment, attachments};
use crate::doc::Document;
use crate::scene;
use crate::settings::Style;
use crate::svg::{num, PALETTE};
use mzlab_mol::fixed;

/// Where the baseline of a text is below its vertical centre, in units of the font size (the SVG centres the text).
const BASELINE: f32 = 0.35;

/// A colour of the palette as the three operands of `rg` / `RG` (0..1, three decimals).
fn rgb(index: f32) -> String {
    let hex = PALETTE[index as usize % PALETTE.len()].trim_start_matches('#');
    let part = |i: usize| {
        let v = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap_or(0);
        let s = fixed(f64::from(v) / 255.0, 3);
        let s = s.trim_end_matches('0').trim_end_matches('.');
        if s.is_empty() { "0" } else { s }.to_string()
    };
    format!("{} {} {}", part(0), part(1), part(2))
}

/// The byte of a character in WinAnsiEncoding (Windows-1252), `?` for a character the encoding does not have.
fn win_ansi(c: char) -> u8 {
    match c {
        ' '..='~' | '\u{a0}'..='\u{ff}' => c as u8,
        '\u{20ac}' => 0x80,
        '\u{2026}' => 0x85,
        '\u{2018}' => 0x91,
        '\u{2019}' => 0x92,
        '\u{201c}' => 0x93,
        '\u{201d}' => 0x94,
        '\u{2022}' => 0x95,
        // the en dash is the closest glyph to a minus sign
        '\u{2013}' | '\u{2212}' => 0x96,
        '\u{2014}' => 0x97,
        _ => b'?',
    }
}

/// A PDF string `(...)` of a text: WinAnsi bytes, the three special characters escaped, the others outside printable ASCII as octal.
fn string(s: &str) -> String {
    let mut out = String::from("(");
    for b in s.chars().map(win_ansi) {
        match b {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(b as char);
            }
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\{:03o}", b)),
        }
    }
    out.push(')');
    out
}

/// The content stream of the page: the display list as operators, `h` the height of the page (PDF counts y upwards).
fn content(sc: &scene::Scene, h: f32) -> String {
    let y = |v: f32| num(h - v);
    let mut s = String::from("1 J\n");
    let d = &sc.data;
    let mut i = 0;
    while i < d.len() {
        let op = d[i];
        if op == scene::LINE {
            s.push_str(&format!(
                "{} RG {} w {} {} m {} {} l S\n",
                rgb(d[i + 6]),
                num(d[i + 5]),
                num(d[i + 1]),
                y(d[i + 2]),
                num(d[i + 3]),
                y(d[i + 4])
            ));
            i += 7;
        } else if op == scene::TEXT {
            let text = &sc.strings[d[i + 5] as usize];
            let size = d[i + 3];
            // the SVG centres on the anchor with `text-anchor: middle`, or starts at it
            let x = if d[i + 6] == 0.0 {
                d[i + 1] - scene::width(text, f64::from(size)) as f32 / 2.0
            } else {
                d[i + 1]
            };
            s.push_str(&format!(
                "BT /F1 {} Tf {} rg {} {} Td {} Tj ET\n",
                num(size),
                rgb(d[i + 4]),
                num(x),
                y(d[i + 2] + BASELINE * size),
                string(text)
            ));
            i += 7;
        } else if op == scene::TRI {
            s.push_str(&format!(
                "{} rg {} {} m {} {} l {} {} l h f\n",
                rgb(d[i + 7]),
                num(d[i + 1]),
                y(d[i + 2]),
                num(d[i + 3]),
                y(d[i + 4]),
                num(d[i + 5]),
                y(d[i + 6])
            ));
            i += 8;
        } else if op == scene::RECT {
            i += 6;
        } else {
            i += 5;
        }
    }
    s
}

/// The objects of a file with the byte offset of each, written in order from 1.
struct Writer {
    out: Vec<u8>,
    offsets: Vec<usize>,
}

impl Writer {
    fn new() -> Writer {
        // the second line (bytes above 127) marks the file as binary for the programs that look
        Writer {
            out: b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec(),
            offsets: Vec::new(),
        }
    }

    /// The next object, a dictionary or any other body.
    fn object(&mut self, body: &str) {
        self.offsets.push(self.out.len());
        let n = self.offsets.len();
        self.out
            .extend(format!("{n} 0 obj\n{body}\nendobj\n").into_bytes());
    }

    /// The next object, a stream with its dictionary entries (`/Length` is added).
    fn stream(&mut self, entries: &str, data: &[u8]) {
        self.offsets.push(self.out.len());
        let n = self.offsets.len();
        self.out.extend(
            format!(
                "{n} 0 obj\n<< {entries} /Length {} >>\nstream\n",
                data.len()
            )
            .into_bytes(),
        );
        self.out.extend(data);
        self.out.extend(b"\nendstream\nendobj\n");
    }
}

/// 128 bits of FNV-1a (two lanes of 64 bits with different starts) as 32 hex digits: the `/ID` of the file.
fn id_of(bytes: &[u8]) -> String {
    let mut h = [0xcbf2_9ce4_8422_2325u64, 0x8422_2325_cbf2_9ce4u64];
    for &b in bytes {
        for lane in &mut h {
            *lane = (*lane ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("{:016x}{:016x}", h[0], h[1])
}

/// The PDF of a document in a style: the same bytes for the same document, with the document and its V3000 molfile inside.
pub fn to_pdf(doc: &Document, style: &Style) -> Vec<u8> {
    to_pdf_with(doc, style, &[])
}

/// `to_pdf` with the calculated values the editor holds (`svg::to_svg_measured`).
pub fn to_pdf_with(doc: &Document, style: &Style, measured: &[crate::values::Entry]) -> Vec<u8> {
    let (body, w, h) = match crate::svg::laid_out_with(doc, style, measured) {
        Some((sc, w, h)) => {
            // the size as the SVG states it (two decimals)
            let (w, h) = (
                ((w * 100.0).round() / 100.0) as f32,
                ((h * 100.0).round() / 100.0) as f32,
            );
            (content(&sc, h), w, h)
        }
        None => (String::new(), 1.0, 1.0),
    };
    let json = doc.to_json();
    // the names of the embedded document and molfile come from the name in use (`brand`)
    let (doc_name, mol_name) = (attachment("json"), attachment("mol"));
    let mut files = vec![(doc_name.as_str(), "application#2Fjson", json.into_bytes())];
    if !doc.is_empty() {
        files.push((
            mol_name.as_str(),
            "chemical#2Fx-mdl-molfile",
            doc.to_molfile_v3000().into_bytes(),
        ));
    }
    let names: Vec<String> = (0..files.len())
        .map(|k| format!("{} {} 0 R", string(files[k].0), 6 + 2 * k))
        .collect();
    let mut f = Writer::new();
    f.object(&format!(
        "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [{}] >> >> >>",
        names.join(" ")
    ));
    f.object("<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    f.object(&format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
        num(w),
        num(h)
    ));
    f.object("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    f.stream("", body.as_bytes());
    for (k, (name, subtype, data)) in files.iter().enumerate() {
        f.object(&format!(
            "<< /Type /Filespec /F {0} /UF {0} /EF << /F {1} 0 R >> >>",
            string(name),
            7 + 2 * k
        ));
        f.stream(
            &format!(
                "/Type /EmbeddedFile /Subtype /{subtype} /Params << /Size {} >>",
                data.len()
            ),
            data,
        );
    }
    let xref = f.out.len();
    let id = id_of(&f.out);
    let mut s = format!("xref\n0 {}\n0000000000 65535 f \n", f.offsets.len() + 1);
    for o in &f.offsets {
        s.push_str(&format!("{o:010} 00000 n \n"));
    }
    s.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R /ID [<{id}> <{id}>] >>\nstartxref\n{xref}\n%%EOF\n",
        f.offsets.len() + 1
    ));
    f.out.extend(s.into_bytes());
    f.out
}

// ---- reading ----

/// The number written at the start of `b` (digits only), and what follows it.
fn number(b: &[u8]) -> Option<(usize, &[u8])> {
    let n = b.iter().take_while(|c| c.is_ascii_digit()).count();
    if n == 0 || n > 10 {
        return None;
    }
    let v = std::str::from_utf8(&b[..n]).ok()?.parse().ok()?;
    Some((v, &b[n..]))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// The number `N` of the first `N 0 R` that follows `key` in `dict`.
fn reference(dict: &[u8], key: &[u8]) -> Option<usize> {
    let rest = &dict[find(dict, key)? + key.len()..];
    let rest = &rest[rest.iter().take_while(|c| **c == b' ').count()..];
    let (n, rest) = number(rest)?;
    rest.starts_with(b" 0 R").then_some(n)
}

/// The cross-reference offsets of the file (object 0 is the head of the free list and has none).
fn offsets(bytes: &[u8]) -> Option<Vec<usize>> {
    let tail = bytes.len().saturating_sub(64);
    let at = tail + find(&bytes[tail..], b"startxref\n")? + 10;
    let (start, _) = number(bytes.get(at..)?)?;
    let table = bytes.get(start..)?.strip_prefix(b"xref\n0 ")?;
    let (count, rest) = number(table)?;
    let mut rest = rest.strip_prefix(b"\n")?;
    let mut out = Vec::new();
    for k in 0..count {
        let line = rest.get(..20)?;
        rest = &rest[20..];
        if k > 0 {
            if !line.ends_with(b" 00000 n \n") {
                return None;
            }
            out.push(number(line)?.0);
        }
    }
    Some(out)
}

/// The object `n`: its dictionary text (up to its stream or its end) and the offset just after it.
fn object<'a>(bytes: &'a [u8], offsets: &[usize], n: usize) -> Option<(&'a [u8], usize)> {
    let at = *offsets.get(n.checked_sub(1)?)?;
    let rest = bytes.get(at..)?;
    let head = format!("{n} 0 obj\n");
    let rest = rest.strip_prefix(head.as_bytes())?;
    let end = [find(rest, b"stream\n"), find(rest, b"endobj")]
        .into_iter()
        .flatten()
        .min()?;
    Some((&rest[..end], at + head.len() + end))
}

/// The bytes of the stream of the object `n`, its `/Length` checked against the end of the stream.
fn stream_of<'a>(bytes: &'a [u8], offsets: &[usize], n: usize) -> Option<&'a [u8]> {
    let (dict, after) = object(bytes, offsets, n)?;
    let (len, _) = number(&dict[find(dict, b"/Length ")? + 8..])?;
    let data = bytes.get(after + 7..)?.get(..len)?; // "stream\n"
    bytes
        .get(after + 7 + len..)?
        .starts_with(b"\nendstream")
        .then_some(data)
}

/// The bytes of the embedded file called `name`, found through the name tree of the catalog.
fn embedded<'a>(bytes: &'a [u8], offsets: &[usize], name: &str) -> Option<&'a [u8]> {
    if !bytes.starts_with(b"%PDF-") {
        return None;
    }
    let root = reference(bytes.get(find(bytes, b"trailer\n")?..)?, b"/Root")?;
    let (catalog, _) = object(bytes, offsets, root)?;
    let spec = reference(catalog, format!("{} ", string(name)).as_bytes())?;
    let (spec, _) = object(bytes, offsets, spec)?;
    stream_of(bytes, offsets, reference(spec, b"/EF << /F")?)
}

/// The document a PDF carries: `None` when it carries none (a PDF of another program), else the document or the key of what is
/// wrong with it.
pub fn carried(bytes: &[u8]) -> Option<Result<Document, &'static str>> {
    let offsets = offsets(bytes)?;
    // the document is attached under the name in use or under an earlier one
    let json = attachments("json").find_map(|name| embedded(bytes, &offsets, &name))?;
    Some(
        std::str::from_utf8(json)
            .map_err(|_| "error.draw2.import")
            .and_then(Document::from_json),
    )
}

// ---- base64, for the replay request (`"out":"pdf"`) ----

/// The bytes as base64 (RFC 4648, with padding, no line breaks).
pub fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = c
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for k in 0..4 {
            s.push(if k <= c.len() {
                A[(n >> (18 - 6 * k) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    s
}
