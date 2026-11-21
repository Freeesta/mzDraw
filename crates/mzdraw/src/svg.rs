//! The display list as an SVG file: the same primitives the page paints, with the palette of the file. Deterministic text (numbers
//! to 2 decimals), no editor overlay. The size of everything comes from a `Style`.

use crate::doc::{Document, ObjKind};
use crate::scene::{self, Overlay, View};
use crate::settings::Style;
use mzlab_mol::fixed;

/// File colours by colour index (ink, then the overlay colours that a file never uses, then N, O, S, halogens, P).
const PALETTE: [&str; 10] = [
    "#000000", "#000000", "#000000", "#000000", "#000000", "#2040c0", "#c02020", "#8a6d00",
    "#1f7a1f", "#b05a00",
];

/// A number with at most two decimals, trailing zeros left out. Written through `mzlab_mol::fixed`: one `f64` given to the
/// formatter of the standard library (`{}`, `to_string`) costs 22 kB of module.
pub(crate) fn num(v: f32) -> String {
    let s = fixed(v as f64, 2);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The attributes of the root that name the picture for assistive technology (SVG 1.1 plus WAI-ARIA Graphics: the role, the
/// two elements that label it, the language of their text).
const ACCESS: &str = " role=\"img\" aria-labelledby=\"svg-title svg-desc\" xml:lang=\"en\"";

/// The `title` and `desc` elements, first children of the root.
fn alternative(title: &str, desc: &str) -> String {
    format!(
        "<title id=\"svg-title\">{}</title>\n<desc id=\"svg-desc\">{}</desc>\n",
        escape(title),
        escape(desc)
    )
}

/// Where an SVG written by mzDraw carries its document.
const CARRIED: (&str, &str) = ("<metadata id=\"mzdraw\">", "</metadata>");

/// The SVG with its document inside (`mzdraw/1` as the text of a `metadata` element, after the opening tag): the picture can be
/// opened again and edited. A viewer does not show the element.
pub fn carrying(svg: String, doc: &Document) -> String {
    let Some(at) = svg.find('\n').filter(|_| !doc.is_empty()) else {
        return svg;
    };
    let (head, body) = svg.split_at(at + 1);
    format!(
        "{head}{}{}{}\n{body}",
        CARRIED.0,
        escape(&doc.to_json()),
        CARRIED.1
    )
}

/// The document an SVG carries: `None` when it carries none (an SVG of another program), else the document or the key of what is
/// wrong with it.
pub fn carried(svg: &str) -> Option<Result<Document, &'static str>> {
    let from = svg.find(CARRIED.0)? + CARRIED.0.len();
    let to = from + svg[from..].find(CARRIED.1)?;
    let json = (svg[from..to].replace("&lt;", "<"))
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    Some(Document::from_json(&json))
}

/// The whole document as an SVG of its own size in the default style (an empty document gives an empty 1x1 drawing).
pub fn to_svg(doc: &Document) -> String {
    to_svg_styled(doc, &Style::default())
}

/// The whole document as an SVG in a style. One unit of the drawing is one point and the size is written in points, so the file
/// prints at the true size of the style (a bond of `acs-1996` is 14.4 pt long on paper).
pub fn to_svg_styled(doc: &Document, style: &Style) -> String {
    drawn(doc, style).0
}

/// `to_svg_styled` with the width and the height of the drawing in points.
pub(crate) fn drawn(doc: &Document, style: &Style) -> (String, f64, f64) {
    let Some((x0, y0, mut x1, y1)) = doc.bounds() else {
        let (t, d) = crate::outline::alternative(doc);
        let empty = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\" viewBox=\"0 0 1 1\"{ACCESS}>\n{}</svg>\n",
            alternative(&t, &d)
        );
        return (empty, 1.0, 1.0);
    };
    let scale = style.bond_length;
    let m = style.metrics(scale, false);
    // a text runs to the right of its anchor
    for o in doc.objects.iter().filter(|o| o.t == ObjKind::Text) {
        x1 = x1.max(o.x + scene::width(&o.text, m.font) / scale);
    }
    // labels can stick out by about two characters on each side
    let pad = scale * 0.5 + m.font * 1.6;
    let view = View {
        scale,
        ox: pad - x0 * scale,
        oy: pad + y1 * scale,
    };
    let (w, h) = ((x1 - x0) * scale + 2.0 * pad, (y1 - y0) * scale + 2.0 * pad);
    let sc = scene::build(doc, view, &Overlay::default(), m);
    // the size as the file states it (two decimals)
    let size = ((w * 100.0).round() / 100.0, (h * 100.0).round() / 100.0);
    let (w, h) = (num(w as f32), num(h as f32));
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}pt\" height=\"{h}pt\" viewBox=\"0 0 {w} {h}\" font-family=\"Arial, Helvetica, sans-serif\" stroke-linecap=\"round\"{ACCESS}>\n"
    );
    let (t, d) = crate::outline::alternative(doc);
    s.push_str(&alternative(&t, &d));
    let d = &sc.data;
    let mut i = 0;
    while i < d.len() {
        let op = d[i];
        if op == scene::LINE {
            s.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>\n",
                num(d[i + 1]), num(d[i + 2]), num(d[i + 3]), num(d[i + 4]), PALETTE[d[i + 6] as usize % 10], num(d[i + 5])
            ));
            i += 7;
        } else if op == scene::TEXT {
            let anchor = if d[i + 6] == 0.0 { "middle" } else { "start" };
            s.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\" text-anchor=\"{anchor}\" dominant-baseline=\"central\">{}</text>\n",
                num(d[i + 1]), num(d[i + 2]), num(d[i + 3]), PALETTE[d[i + 4] as usize % 10], escape(&sc.strings[d[i + 5] as usize])
            ));
            i += 7;
        } else if op == scene::TRI {
            s.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>\n",
                num(d[i + 1]),
                num(d[i + 2]),
                num(d[i + 3]),
                num(d[i + 4]),
                num(d[i + 5]),
                num(d[i + 6]),
                PALETTE[d[i + 7] as usize % 10]
            ));
            i += 8;
        } else if op == scene::RECT {
            i += 6;
        } else {
            i += 5;
        }
    }
    s.push_str("</svg>\n");
    (s, size.0, size.1)
}
