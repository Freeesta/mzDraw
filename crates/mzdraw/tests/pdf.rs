//! The PDF mzDraw writes: a valid file (the cross-reference offsets are exact, parsed here by hand), text as text, the document
//! inside so that it opens again, the same bytes for the same drawing, and a reader that refuses what is not ours without a panic.

use mzdraw::io::import_bytes;
use mzdraw::{pdf, Document, Editor, Style};

/// The file as text with the same offsets: a byte above 127 is shown as `?`.
fn text(b: &[u8]) -> String {
    b.iter()
        .map(|&c| if c < 128 { c as char } else { '?' })
        .collect()
}

/// A synthetic drawing: labelled atoms (text), a wedge (polygon), a coloured heteroatom, an arrow with text and a free text.
fn drawing() -> Document {
    let mut e = Editor::default();
    e.import("N[C@@H](C)C(=O)O").unwrap();
    e.import(
        r#"{"format":"mzdraw/1","next":3,"atoms":[],"bonds":[],"objects":[{"id":1,"t":"text","x":0.0,"y":4.0,"text":"a (b) \\ 30 °C – ok"},{"id":2,"t":"arrow","x":3.0,"y":4.0,"x2":5.0,"y2":4.0,"text":"H2, Pd/C"}]}"#,
    )
    .unwrap();
    e.doc().clone()
}

/// The objects of a file found the plain way: every `N 0 obj` at the start of a line, by scanning, to check the table against.
fn scan(file: &[u8]) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < file.len() {
        let line_end = file[at..]
            .iter()
            .position(|&c| c == b'\n')
            .map_or(file.len(), |p| at + p + 1);
        let line = text(&file[at..line_end]);
        if let Some(n) = line
            .strip_suffix(" 0 obj\n")
            .and_then(|n| n.parse::<usize>().ok())
        {
            found.push((n, at));
        }
        // skip a stream by its stated length
        if line == "stream\n" {
            let head = &file[..at];
            let h = text(&head[head.len().saturating_sub(200)..]);
            let len: usize = h
                .rsplit("/Length ")
                .next()
                .unwrap()
                .split(' ')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            at += len + 1;
            continue;
        }
        at = line_end;
    }
    found
}

#[test]
fn the_cross_reference_offsets_are_exact() {
    let file = pdf::to_pdf(&drawing(), &Style::default());
    assert!(file.starts_with(b"%PDF-1.7\n") && file.ends_with(b"%%EOF\n"));
    // startxref points at the table
    let s = text(&file);
    let at: usize = s[s.rfind("startxref\n").unwrap() + 10..]
        .lines()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(s[at..].starts_with("xref\n0 "));
    let mut lines = s[at..].lines();
    lines.next();
    let count: usize = lines
        .next()
        .unwrap()
        .strip_prefix("0 ")
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(lines.next().unwrap(), "0000000000 65535 f ");
    let table: Vec<usize> = (1..count)
        .map(|_| {
            let l = lines.next().unwrap();
            assert_eq!(l.len(), 19, "an entry is 20 bytes with its newline: {l:?}");
            assert!(l.ends_with(" 00000 n "));
            l[..10].parse().unwrap()
        })
        .collect();
    // every offset is the start of `N 0 obj`, and the objects found by scanning are exactly the table's
    for (k, &o) in table.iter().enumerate() {
        assert!(
            s[o..].starts_with(&format!("{} 0 obj\n", k + 1)),
            "object {}",
            k + 1
        );
    }
    let mut scanned = scan(&file);
    scanned.sort();
    assert_eq!(
        scanned,
        table
            .iter()
            .enumerate()
            .map(|(k, &o)| (k + 1, o))
            .collect::<Vec<_>>()
    );
    // the trailer names the size and the root; the table is the last thing before it
    assert!(s.contains(&format!("/Size {count} /Root 1 0 R")));
    // every stream states its exact length
    for part in s.split("/Length ").skip(1) {
        let (len, rest) = part.split_once(' ').unwrap();
        let len: usize = len.parse().unwrap();
        let body = rest.split_once("stream\n").unwrap().1;
        assert!(
            body.as_bytes()[len..].starts_with(b"\nendstream"),
            "stream of {len} bytes"
        );
    }
}

#[test]
fn the_page_has_the_size_of_the_drawing_and_its_text_is_text() {
    let d = drawing();
    let file = pdf::to_pdf(&d, &Style::default());
    let s = text(&file);
    // the page: the size of the SVG in points
    let svg = mzdraw::svg::to_svg(&d);
    let w = svg
        .split("width=\"")
        .nth(1)
        .unwrap()
        .split("pt")
        .next()
        .unwrap();
    let h = svg
        .split("height=\"")
        .nth(1)
        .unwrap()
        .split("pt")
        .next()
        .unwrap();
    assert!(s.contains(&format!("/MediaBox [0 0 {w} {h}]")), "{w} {h}");
    // a base-14 font with WinAnsi, no font file
    assert!(
        s.contains("/BaseFont /Helvetica /Encoding /WinAnsiEncoding") && !s.contains("/FontFile")
    );
    // text as text: the label of an atom, the free text with its special characters escaped, the degree sign as its WinAnsi byte
    assert!(
        s.contains("(NH2) Tj") || s.contains("(N) Tj"),
        "a label is a string of the page"
    );
    assert!(s.contains(r"(a \(b\) \\ 30 \260C \226 ok) Tj"), "free text");
    assert!(s.contains("(H2, Pd/C) Tj"), "the text of the arrow");
    // paths, a filled polygon and RGB colours: black ink and the colour of nitrogen
    assert!(s.contains(" l S\n") && s.contains(" l h f\n"));
    assert!(s.contains("0.125 0.251 0.753 rg"), "the colour of N");
    // nothing that depends on time
    assert!(!s.contains("/CreationDate") && !s.contains("/ModDate") && !s.contains("/Info"));
}

#[test]
fn the_document_is_inside_and_opens_again() {
    let d = drawing();
    let file = pdf::to_pdf(&d, &Style::default());
    // the reader finds `mzdraw.json` and gives the same bytes
    assert_eq!(pdf::carried(&file).unwrap().unwrap().to_json(), d.to_json());
    let (back, info) = import_bytes(&file).unwrap();
    assert_eq!((back.to_json(), info.format), (d.to_json(), "pdf"));
    // the document is stored as it is, in the file, and the V3000 molfile is a second embedded file
    let s = text(&file);
    let has = |t: &str| file.windows(t.len()).any(|w| w == t.as_bytes());
    assert!(has(&d.to_json()) && has(&d.to_molfile_v3000()));
    assert!(s.contains("/Names [(mzdraw.json) 6 0 R (mzdraw.mol) 8 0 R]"));
    // an editor opens it as one undo step
    let mut e = Editor::default();
    assert_eq!(e.import_bytes(&file).unwrap().format, "pdf");
    assert_eq!(e.doc().to_json(), d.to_json());
    e.undo();
    assert!(e.doc().is_empty());
}

#[test]
fn the_same_drawing_gives_the_same_bytes_and_a_content_id() {
    let d = drawing();
    let a = pdf::to_pdf(&d, &Style::default());
    assert_eq!(a, pdf::to_pdf(&d, &Style::default()));
    let mut e = Editor::default();
    e.import("CCO").unwrap();
    let b = e.pdf();
    assert_ne!(a, b);
    let id = |f: &[u8]| {
        let s = text(f);
        s.split("/ID [<")
            .nth(1)
            .unwrap()
            .split('>')
            .next()
            .unwrap()
            .to_string()
    };
    assert_eq!(id(&a).len(), 32);
    assert_ne!(id(&a), id(&b));
    // the style changes the page, and so the file
    assert_ne!(a, pdf::to_pdf(&d, &Style::preset("acs-1996").unwrap()));
}

#[test]
fn an_empty_drawing_is_a_valid_one_point_page() {
    let file = Editor::default().pdf();
    let s = text(&file);
    assert!(s.contains("/MediaBox [0 0 1 1]"));
    assert!(!s.contains("mzdraw.mol"));
    let d = pdf::carried(&file).unwrap().unwrap();
    assert!(d.is_empty());
}

#[test]
fn the_replay_request_gives_the_same_bytes_as_base64() {
    let request = r#"{"import":"N[C@@H](C)C(=O)O","out":"pdf"}"#;
    let got = mzdraw::replay::replay(request).unwrap();
    let mut e = Editor::default();
    e.import("N[C@@H](C)C(=O)O").unwrap();
    assert_eq!(got, pdf::base64(&e.pdf()));
    assert!(got.is_ascii() && !got.contains('\n'));
    // base64 against the known vectors of RFC 4648
    for (plain, coded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(pdf::base64(plain.as_bytes()), coded);
    }
}

#[test]
fn a_pdf_of_another_program_is_refused_in_words() {
    // a small synthetic PDF written by hand with a table of its own: a page, no embedded file
    let mut f = b"%PDF-1.4\n".to_vec();
    let mut offs = Vec::new();
    for (n, body) in [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
    ]
    .iter()
    .enumerate()
    {
        offs.push(f.len());
        f.extend(format!("{} 0 obj\n{body}\nendobj\n", n + 1).bytes());
    }
    let xref = f.len();
    f.extend(b"xref\n0 4\n0000000000 65535 f \n");
    for o in offs {
        f.extend(format!("{o:010} 00000 n \n").bytes());
    }
    f.extend(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").bytes());
    assert!(pdf::carried(&f).is_none());
    assert_eq!(import_bytes(&f).unwrap_err(), "error.draw2.pdfPlain");
    // not even a table
    assert_eq!(
        import_bytes(b"%PDF-1.7\nnothing else").unwrap_err(),
        "error.draw2.pdfPlain"
    );
}

#[test]
fn a_damaged_document_inside_says_so() {
    let d = drawing();
    let file = pdf::to_pdf(&d, &Style::default());
    // same length, so the table stays exact: the format name of the document is changed
    let mut bytes = file.clone();
    let at = bytes
        .windows(10)
        .position(|w| w == b"\"mzdraw/1\"")
        .unwrap();
    bytes[at + 8] = b'9';
    assert_eq!(
        import_bytes(&bytes).unwrap_err(),
        "error.draw2.documentNewer"
    );
}

#[test]
fn the_reader_never_panics_on_a_damaged_file() {
    let mut e = Editor::default();
    e.import("CC(=O)O").unwrap();
    let file = e.pdf();
    // every truncation
    for n in 0..file.len() {
        let _ = import_bytes(&file[..n]);
    }
    // every byte changed, in the head, the table, the trailer and a sample of the streams (the whole file when it is small)
    let step = (file.len() / 4000).max(1);
    for at in (0..file.len()).step_by(step) {
        for v in [0u8, b'9', 0xff] {
            let mut m = file.clone();
            m[at] = v;
            let _ = import_bytes(&m);
        }
    }
    // a table and a length that lie
    let at = file.windows(8).position(|w| w == b"/Length ").unwrap();
    let mut lie = file.clone();
    lie.splice(at + 8..at + 8, *b"9");
    let _ = import_bytes(&lie);
    let _ = import_bytes(&file.iter().rev().copied().collect::<Vec<_>>());
    let _ = import_bytes(b"%PDF-");
    let _ = import_bytes(b"%PDF-\nstartxref\n99999999999\n%%EOF");
}
