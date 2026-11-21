//! `digest/1`: a few hundred bytes of text that say where things are in a drawing, for an agent that cannot see it. Per connected
//! structure, left to right: its atom ids, Hill formula, box, the atoms that are not plain carbon, the wedge marks and the ring count;
//! then the objects, the smallest gap between two structures and the number of `std/1` findings per key.
//!
//! Written by hand with the keys in a fixed order, numbers through `mzlab_mol::fixed` (one decimal): the same drawing gives the same
//! bytes everywhere. The code of this module is one pass over the atoms and the bonds (a union-find, no `layout::pieces`, whose
//! rescans are quadratic). One engine call under it is not: `scene::label_extent` looks through the bonds for
//! every atom, so the cost of a drawing with bonds grows as atoms times bonds (`Document::to_mol` is linear). That cost is capped, not removed: a drawing whose
//! `atoms * (bonds + 1)` is above `MAX_WORK` is refused (`error.draw2.digestSize`). The size of the answer is capped too (see the
//! constants), so it can go into a prompt: what is cut is counted, never dropped silently. Boxes are in bond lengths, y up, label ink
//! included (`scene::label_extent`, as the layout uses it).

use crate::doc::{Atom, Document, ObjKind};
use crate::settings::Style;
use mzlab_mol::fixed;
use serde_json::Value;
use std::collections::HashMap;

/// The value of the key `format`.
pub const FORMAT: &str = "digest/1";

/// Structures and objects listed at most; the rest are counted in `more` (`[structures, objects]`). The pairwise `clearance` runs over
/// the structures listed, so its cost stays bounded too.
pub const MAX_LISTED: usize = 200;

/// Bytes after which no further structure is listed (the one being written is finished): a hostile drawing cannot make the answer
/// large. The objects have their own `MAX_LISTED` entries, so the answer stays under `MAX_BYTES` + `MAX_PIECE` + the objects.
pub const MAX_BYTES: usize = 12_000;

/// Entries of one structure: `labels` and `stereo` entries and the bytes of `ids`. What is cut is counted in the key `truncated`
/// (`[labels, stereo, atoms]`, present only then; a cut `ids` ends in `,+K`: K more atoms). A structure of up to 150 atoms that
/// are not carbon is listed whole.
pub const MAX_LABELS: usize = 150;
pub const MAX_STEREO: usize = 50;
pub const MAX_IDS: usize = 400;

/// The work `scene::label_extent` under the digest may cost, as atoms * (bonds + 1) (see the module text): 25 million is a chain of
/// 5,000 atoms. A drawing of one atom per structure (no bond) costs next to nothing per atom and has no limit
/// of its own.
pub const MAX_WORK: usize = 25_000_000;

/// Atoms up to which the `std/1` findings are counted. The check rescans the drawing for every atom (quadratic), so above this many
/// atoms `std` is `null` ("not counted", never `{}`, which says "no finding") and the key `stdSkipped` says why. The cap stays: making
/// the check itself linear is not part of this slice.
pub const MAX_STD_ATOMS: usize = 1000;

/// A box: x0, y0, x1, y1 (bond lengths).
type Rect = [f64; 4];

/// One decimal; a value that rounds to zero is `0.0`, never `-0.0`.
fn d1(v: f64) -> String {
    if (v * 10.0).round() == 0.0 {
        "0.0".into()
    } else {
        fixed(v, 1)
    }
}

fn rect_json(r: Rect) -> String {
    format!("[{},{},{},{}]", d1(r[0]), d1(r[1]), d1(r[2]), d1(r[3]))
}

fn quote(s: &str) -> String {
    Value::from(s).to_string()
}

/// Atom ids as ranges: `1-12,15` (a run of two or more consecutive ids is a range), at most `cap` bytes: a longer text is cut at a
/// comma and ends in `,+K` (K more atoms). Gives the text and the number of atoms it names.
fn ranges(ids: &mut [u32], cap: usize) -> (String, usize) {
    ids.sort_unstable();
    let mut out = String::new();
    let mut i = 0;
    while i < ids.len() {
        let mut j = i;
        while j + 1 < ids.len() && ids[j + 1] == ids[j] + 1 {
            j += 1;
        }
        let mut part = String::new();
        if i > 0 {
            part.push(',');
        }
        part.push_str(&ids[i].to_string());
        if j > i {
            part.push('-');
            part.push_str(&ids[j].to_string());
        }
        if out.len() + part.len() > cap {
            out.push_str(&format!(",+{}", ids.len() - i));
            return (out, i);
        }
        out.push_str(&part);
        i = j + 1;
    }
    (out, ids.len())
}

/// What an atom that is not a plain carbon shows: its label, else mass number, element and charge (`13C`, `N+`, `O2-`).
fn label_text(a: &Atom) -> Option<String> {
    if !a.lab.is_empty() {
        return Some(a.lab.clone());
    }
    if a.el == "C" && a.chg == 0 && a.iso == 0 && a.rad == 0 {
        return None;
    }
    let mut s = String::new();
    if a.iso != 0 {
        s.push_str(&a.iso.to_string());
    }
    s.push_str(&a.el);
    if a.chg != 0 {
        if a.chg.abs() > 1 {
            s.push_str(&a.chg.unsigned_abs().to_string());
        }
        s.push(if a.chg > 0 { '+' } else { '-' });
    }
    if a.rad != 0 {
        s.push('.');
    }
    Some(s)
}

/// One connected structure, filled in a single pass.
struct Piece {
    ids: Vec<u32>,
    counts: Vec<(String, u32)>,
    charge: i32,
    bbox: Rect,
    labels: Vec<(u32, String)>,
    stereo: Vec<(u32, &'static str, u32)>,
    bonds: usize,
}

fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The gap between two boxes, 0 when they touch or overlap.
fn gap(a: Rect, b: Rect) -> f64 {
    let dx = (b[0] - a[2]).max(a[0] - b[2]).max(0.0);
    let dy = (b[1] - a[3]).max(a[1] - b[3]).max(0.0);
    libm::sqrt(dx * dx + dy * dy)
}

/// The digest of a drawing in a style (the style decides the ink of the labels, hence the boxes). A drawing whose work is above
/// `MAX_WORK` is refused: `error.draw2.digestSize`.
pub fn json(doc: &Document, style: &Style) -> Result<String, &'static str> {
    let n = doc.atoms.len();
    if n.saturating_mul(doc.bonds.len() + 1) > MAX_WORK {
        return Err("error.draw2.digestSize");
    }
    let m = doc.to_mol();
    let index: HashMap<u32, usize> = doc
        .atoms
        .iter()
        .enumerate()
        .map(|(i, a)| (a.id, i))
        .collect();
    // structures: union-find over the bonds of the record (the atoms of an abbreviation follow the atoms of the drawing); the
    // smaller index stays the root
    let total_atoms = m.atoms.len().max(n);
    let mut parent: Vec<usize> = (0..total_atoms).collect();
    for b in &m.bonds {
        let (rx, ry) = (
            root(&mut parent, b.a as usize),
            root(&mut parent, b.b as usize),
        );
        parent[rx.max(ry)] = rx.min(ry);
    }
    let mut slot: Vec<Option<usize>> = vec![None; total_atoms];
    let mut pieces: Vec<Piece> = Vec::new();
    for i in 0..total_atoms {
        let r = root(&mut parent, i);
        let k = *slot[r].get_or_insert_with(|| {
            pieces.push(Piece {
                ids: Vec::new(),
                counts: Vec::new(),
                charge: 0,
                bbox: [f64::MAX, f64::MAX, f64::MIN, f64::MIN],
                labels: Vec::new(),
                stereo: Vec::new(),
                bonds: 0,
            });
            pieces.len() - 1
        });
        let p = &mut pieces[k];
        // the formula counts an abbreviation as the atoms it stands for (the record `Document::to_mol`), an R group as `R`
        let r_group = doc
            .atoms
            .get(i)
            .is_some_and(|a| matches!(crate::label::kind(&a.lab), Some(crate::label::Kind::R(_))));
        let mut add = |el: &str, c: u32| match p.counts.iter_mut().find(|x| x.0 == el) {
            Some(x) => x.1 += c,
            None => p.counts.push((el.to_string(), c)),
        };
        if r_group {
            add("R", 1);
        } else if let Some(ma) = m.atoms.get(i) {
            add(&ma.el, 1);
            add("H", u32::from(ma.h));
            p.charge += i32::from(ma.chg);
        }
        let Some(a) = doc.atoms.get(i) else { continue };
        p.ids.push(a.id);
        let [l, rt, below, above] = crate::scene::label_extent(doc, a, style);
        let b = &mut p.bbox;
        *b = [
            b[0].min(a.x - l),
            b[1].min(a.y - below),
            b[2].max(a.x + rt),
            b[3].max(a.y + above),
        ];
        if let Some(t) = label_text(a) {
            p.labels.push((a.id, t));
        }
    }
    for b in &doc.bonds {
        let (Some(&x), Some(&_y)) = (index.get(&b.a), index.get(&b.b)) else {
            continue;
        };
        let k = slot[root(&mut parent, x)].unwrap_or(0);
        pieces[k].bonds += 1;
        let mark = match b.s {
            crate::doc::WEDGE_UP => "wedge",
            crate::doc::WEDGE_DOWN => "hash",
            crate::doc::WEDGE_WAVY => "wavy",
            _ => continue,
        };
        pieces[k].stereo.push((b.id, mark, b.a));
    }
    // left to right: by the left edge of the box, then by the smallest atom id
    let total = pieces.len();
    for p in &mut pieces {
        p.ids.sort_unstable();
    }
    pieces.sort_by(|a, b| {
        (a.bbox[0].total_cmp(&b.bbox[0])).then_with(|| a.ids.first().cmp(&b.ids.first()))
    });
    let mut out = format!("{{\"format\":\"{FORMAT}\",\"pieces\":[");
    // what is cut: labels, stereo entries, atoms of an `ids` text
    let mut cut = (0usize, 0usize, 0usize);
    let mut listed = 0;
    for p in pieces.iter_mut() {
        if listed == MAX_LISTED || (listed > 0 && out.len() >= MAX_BYTES) {
            break;
        }
        if listed > 0 {
            out.push(',');
        }
        listed += 1;
        // the Hill formula of the outline line, with the ASCII minus
        let formula = crate::outline::hill(&mut p.counts, p.charge).replace('\u{2212}', "-");
        let labels: Vec<String> = (p.labels.iter().take(MAX_LABELS))
            .map(|(id, t)| format!("[{id},{}]", quote(t)))
            .collect();
        let stereo: Vec<String> = (p.stereo.iter().take(MAX_STEREO))
            .map(|(id, k, a)| format!("[{id},\"{k}\",{a}]"))
            .collect();
        cut.0 += p.labels.len() - labels.len();
        cut.1 += p.stereo.len() - stereo.len();
        let (all, ids) = (p.ids.len(), ranges(&mut p.ids, MAX_IDS));
        cut.2 += all - ids.1;
        // the cyclomatic number of a connected structure: bonds - atoms + 1
        let rings = (p.bonds + 1).saturating_sub(p.ids.len());
        out.push_str(&format!(
            "{{\"ids\":{},\"formula\":{},\"box\":{},\"labels\":[{}],\"stereo\":[{}],\"rings\":{rings}}}",
            quote(&ids.0),
            quote(&formula),
            rect_json(p.bbox),
            labels.join(","),
            stereo.join(",")
        ));
    }
    out.push_str("],\"objects\":[");
    // objects: arrows by their ends, a plus by its glyph, a text by its width (as the SVG makes room for it)
    let metrics = style.metrics(1.0, false);
    let f = metrics.font;
    for (shown, o) in doc.objects.iter().take(MAX_LISTED).enumerate() {
        let r: Rect = match o.t {
            ObjKind::Arrow => [o.x.min(o.x2), o.y.min(o.y2), o.x.max(o.x2), o.y.max(o.y2)],
            ObjKind::Plus => [
                o.x - f * 0.65,
                o.y - f * 0.65,
                o.x + f * 0.65,
                o.y + f * 0.65,
            ],
            ObjKind::Text => [
                o.x,
                // the lines below the first one
                o.y - f * 0.5 - o.runs.iter().filter(|r| r.br).count() as f64 * 1.25 * f,
                o.x + if o.runs.is_empty() {
                    crate::scene::width(&o.text, f)
                } else {
                    // a formatted text: the widest line (the line breaks are not characters)
                    crate::runs::layout(&o.runs, f, metrics.small / f).width()
                },
                o.y + f * 0.5,
            ],
        };
        if shown > 0 {
            out.push(',');
        }
        out.push_str(&format!("[{},\"{}\",{}]", o.id, o.t.name(), rect_json(r)));
    }
    out.push_str("],\"clearance\":");
    let boxes: Vec<Rect> = pieces[..listed].iter().map(|p| p.bbox).collect();
    let mut best: Option<f64> = None;
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            let g = gap(*a, *b);
            best = Some(best.map_or(g, |x| x.min(g)));
        }
    }
    out.push_str(&best.map_or("null".to_string(), d1));
    out.push_str(",\"std\":");
    if n > MAX_STD_ATOMS {
        out.push_str("null,\"stdSkipped\":\"atoms>1000\"");
    } else {
        out.push('{');
        let report = crate::pathway::report_with(doc, style);
        for (i, (key, _, count)) in report.counts().iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{}:{count}", quote(key)));
        }
        out.push('}');
    }
    let more = (total - listed, doc.objects.len().saturating_sub(MAX_LISTED));
    if more != (0, 0) {
        out.push_str(&format!(",\"more\":[{},{}]", more.0, more.1));
    }
    if cut != (0, 0, 0) {
        out.push_str(&format!(",\"truncated\":[{},{},{}]", cut.0, cut.1, cut.2));
    }
    out.push('}');
    Ok(out)
}
