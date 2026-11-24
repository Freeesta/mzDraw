//! `grid`: lays the structures of a drawing (or of the selection) out in rows and columns, each with its legend under it: the grid
//! of a table of compounds or of transformation products (rdkit-compare 8, RDKit `MolsToGridImage`).
//!
//! Every structure gets a cell of the same width: the widest structure or caption of the set, plus `gap` bond lengths between
//! columns. A row is as tall as its tallest structure above its lowest atom and its deepest caption below it, plus `gap` between
//! rows. In a row the lowest atoms of the structures stand on one line, so the legends (`display.titles`) and the values
//! (`display.values`, `atom.cap`) of a row stand on one line too, each centred under its structure; the room kept for them is the
//! room the pictures give them (`svg::laid_out_with`), so no caption reaches the next row or the next column.
//!
//! The order: `drawing` (the order of the structures in the document), `reading` (as they stand now: rows from the top, left to
//! right; a grid read again gives the same order) or `title` (by the legend, numbers in a title compared as numbers, so `TP2` comes
//! before `TP10`; the structures without a title last, in the order of the drawing). `cols` is the number of columns (by default
//! the smallest number whose square holds the structures). The grid starts at the highest lowest atom and the leftmost middle of the
//! structures as they stand, so a second `grid` gives no operation.
//!
//! The result is a list of `move` operations, one per structure that moves: the log carries them, a native replay gives the
//! page's bytes, and one undo step reverts them. Arrows, plus signs and texts are not laid out; an arrow anchored at its structures
//! follows them (`Editor::following`).

use crate::doc::{round4, Document};
use crate::ops::Op;
use crate::route::Structures;
use crate::settings::Style;
use serde_json::Value;
use std::cmp::Ordering;

/// The order of the cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Drawing,
    Reading,
    Title,
}

impl Order {
    pub fn parse(v: &str) -> Option<Order> {
        match v {
            "drawing" => Some(Order::Drawing),
            "reading" => Some(Order::Reading),
            "title" => Some(Order::Title),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Order::Drawing => "drawing",
            Order::Reading => "reading",
            Order::Title => "title",
        }
    }
}

/// The words of `order`, for the schema.
pub const ORDERS: [&str; 3] = ["drawing", "reading", "title"];

/// The largest number of columns asked (more than any drawing has structures).
pub const MAX_COLS: u64 = 100_000;

/// What `grid` is asked: the number of columns (`0`: the default), the gap between cells (bond lengths), the order, and the
/// selection (empty: every structure of the drawing).
#[derive(Clone, Debug, PartialEq)]
pub struct Opts {
    pub cols: u32,
    pub gap: f64,
    pub order: Order,
    pub atoms: Vec<u32>,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            cols: 0,
            gap: 1.0,
            order: Order::Drawing,
            atoms: Vec::new(),
        }
    }
}

impl Opts {
    /// Reads the `args` of the command (`cols`, `gap`, `order`, `atoms`, each optional). The error is an i18n key.
    pub fn parse(args: &Value) -> Result<Opts, &'static str> {
        const BAD: &str = "error.draw2.request";
        let obj = args.as_object().ok_or(BAD)?;
        let mut o = Opts::default();
        for (k, v) in obj {
            match k.as_str() {
                "cols" => {
                    o.cols = v
                        .as_u64()
                        .filter(|c| (1..=MAX_COLS).contains(c))
                        .ok_or(BAD)? as u32
                }
                "gap" => {
                    o.gap = v
                        .as_f64()
                        .filter(|g| g.is_finite() && *g > 0.0)
                        .ok_or(BAD)?
                }
                "order" => o.order = v.as_str().and_then(Order::parse).ok_or(BAD)?,
                "atoms" => {
                    o.atoms = (v.as_array().ok_or(BAD)?.iter())
                        .map(|i| i.as_u64().and_then(|i| u32::try_from(i).ok()).ok_or(BAD))
                        .collect::<Result<Vec<u32>, _>>()?
                }
                _ => return Err(BAD),
            }
        }
        Ok(o)
    }
}

/// Two titles in natural order: runs of digits compared as numbers (without their leading zeros, then by their length), the rest
/// character by character.
pub fn natural(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let run = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut s = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        s.push(c);
                        it.next();
                    }
                    s
                };
                let (p, q) = (run(&mut x), run(&mut y));
                let (pt, qt) = (p.trim_start_matches('0'), q.trim_start_matches('0'));
                let by = (pt.len().cmp(&qt.len()))
                    .then_with(|| pt.cmp(qt))
                    .then_with(|| p.len().cmp(&q.len()));
                if by != Ordering::Equal {
                    return by;
                }
            }
            (Some(c), Some(d)) => {
                if c != d {
                    return c.cmp(&d);
                }
                x.next();
                y.next();
            }
        }
    }
}

/// `round4` with a tie always going up (as `scheme::row`): float noise must not send the same tie either way in two runs.
fn snap(v: f64) -> f64 {
    ((v * 1e4 + 0.5 + 1e-6).floor()) / 1e4
}

/// One structure of the grid: its atoms, the point its caption hangs from (the middle of its atoms and its lowest atom) and how far
/// its ink and its caption reach from that point.
struct Cell {
    atoms: Vec<u32>,
    /// the place of the structure in the drawing (the pool index of its first atom)
    rank: usize,
    title: String,
    box_: [f64; 4],
    cx: f64,
    low: f64,
    left: f64,
    right: f64,
    up: f64,
    down: f64,
}

/// The operations that put the structures in a grid. Empty when there is no structure or every structure stands where the grid
/// puts it. `measured` are the values the editor holds (the captions with numbers are as wide as the pictures draw them). The error
/// is an i18n key (a bad argument, or a selection that names an atom the drawing does not have).
pub fn grid(
    doc: &Document,
    style: &Style,
    measured: &[crate::values::Entry],
    o: &Opts,
) -> Result<Vec<Op>, &'static str> {
    if !(o.gap.is_finite() && o.gap > 0.0) || u64::from(o.cols) > MAX_COLS {
        return Err("error.draw2.request");
    }
    if o.atoms.iter().any(|&a| doc.atom(a).is_none()) {
        return Err("error.draw2.noAtom");
    }
    let st = Structures::new(doc, style);
    let chosen: std::collections::HashSet<u32> = o.atoms.iter().copied().collect();
    let pool: std::collections::HashMap<u32, usize> = (doc.atoms.iter().enumerate())
        .map(|(k, a)| (a.id, k))
        .collect();
    // the captions the pictures draw, by the first atom of their structure
    let scale = style.bond_length;
    let cf = crate::values::caption_font(style.metrics(scale, false).font);
    let captions: std::collections::HashMap<u32, crate::values::Caption> =
        crate::values::captions_titled(doc, style.values, &style.adducts, measured, style.titles)
            .into_iter()
            .map(|c| (c.anchor, c))
            .collect();
    let mut cells: Vec<Cell> = Vec::new();
    for piece in st.pieces() {
        if !chosen.is_empty() && !piece.iter().any(|a| chosen.contains(a)) {
            continue;
        }
        let mut atoms = piece.clone();
        atoms.sort_by_key(|a| pool[a]);
        let rank = pool[&atoms[0]];
        let (mut x0, mut x1, mut low) = (f64::MAX, f64::MIN, f64::MAX);
        for a in atoms.iter().filter_map(|&id| doc.atom(id)) {
            x0 = x0.min(a.x);
            x1 = x1.max(a.x);
            low = low.min(a.y);
        }
        let cx = (x0 + x1) / 2.0;
        let b = st.box_of(atoms[0]).expect("a structure of the drawing");
        let (mut left, mut right, up, mut down) = (cx - b[0], b[2] - cx, b[3] - low, low - b[1]);
        // the legend is the title, whether the pictures draw it or not (the order by title needs it)
        let title = (atoms.iter())
            .filter_map(|&id| doc.atom(id))
            .map(|a| crate::values::legend(&a.title()))
            .find(|t| !t.is_empty())
            .unwrap_or_default();
        if let Some(c) = captions.get(&atoms[0]).filter(|c| !c.lines.is_empty()) {
            // as deep and as wide as `svg::laid_out_with` makes the page for it
            let depth = crate::values::caption_first(scale, cf, c.below)
                + c.lines.len().saturating_sub(1) as f64 * crate::values::caption_step(cf)
                + cf * 0.3;
            let half = (c.lines.iter())
                .map(|l| crate::scene::width(l, cf))
                .fold(0.0, f64::max)
                / 2.0;
            down = down.max(depth / scale);
            left = left.max(half / scale);
            right = right.max(half / scale);
        }
        cells.push(Cell {
            atoms,
            rank,
            title,
            box_: b,
            cx,
            low,
            left,
            right,
            up,
            down,
        });
    }
    if cells.is_empty() {
        return Ok(Vec::new());
    }
    cells.sort_by_key(|c| c.rank);
    match o.order {
        Order::Drawing => {}
        Order::Title => cells.sort_by(|a, b| {
            (a.title.is_empty().cmp(&b.title.is_empty()))
                .then_with(|| natural(&a.title, &b.title))
                .then_with(|| a.rank.cmp(&b.rank))
        }),
        Order::Reading => {
            // lines from the top: structures whose heights overlap are one line, each line left to right
            let mut top: Vec<usize> = (0..cells.len()).collect();
            crate::m::sort_by(&mut top, |&a, &b| {
                (-cells[a].box_[3], cells[a].rank) < (-cells[b].box_[3], cells[b].rank)
            });
            let mut order: Vec<usize> = Vec::new();
            let mut line: Vec<usize> = Vec::new();
            let mut floor = f64::MAX;
            let by_x = |line: &mut Vec<usize>, cells: &[Cell]| {
                crate::m::sort_by(line, |&a, &b| {
                    (cells[a].cx, cells[a].rank) < (cells[b].cx, cells[b].rank)
                });
            };
            for i in top {
                if !line.is_empty() && cells[i].box_[3] < floor {
                    by_x(&mut line, &cells);
                    order.append(&mut line);
                    floor = f64::MAX;
                }
                floor = floor.min(cells[i].box_[1]);
                line.push(i);
            }
            by_x(&mut line, &cells);
            order.append(&mut line);
            let mut slots: Vec<Option<Cell>> = cells.into_iter().map(Some).collect();
            cells = (order.into_iter())
                .map(|i| slots[i].take().expect("each cell once"))
                .collect();
        }
    }
    let n = cells.len();
    let cols = if o.cols == 0 {
        let mut c = 1;
        while c * c < n {
            c += 1;
        }
        c
    } else {
        (o.cols as usize).min(n)
    };
    // one width for every column: the cell is centred on the middle of the atoms, so it is twice the larger side
    let half = snap(
        cells
            .iter()
            .map(|c| c.left.max(c.right))
            .fold(0.0, f64::max),
    );
    let pitch = snap(2.0 * half + o.gap);
    let x0 = snap(cells.iter().map(|c| c.cx).fold(f64::MAX, f64::min));
    let mut y = snap(cells.iter().map(|c| c.low).fold(f64::MIN, f64::max));
    let mut ops = Vec::new();
    let mut below_prev = 0.0;
    for (r, row) in cells.chunks(cols).enumerate() {
        let above = row.iter().map(|c| c.up).fold(0.0, f64::max);
        if r > 0 {
            y = snap(y - (below_prev + o.gap + above));
        }
        below_prev = row.iter().map(|c| c.down).fold(0.0, f64::max);
        for (k, c) in row.iter().enumerate() {
            let x = x0 + pitch * k as f64;
            shift(&mut ops, c.atoms.clone(), x - c.cx, y - c.low);
        }
    }
    Ok(ops)
}

/// A `move` of `dx`, `dy`, rounded to the four decimals the document keeps; nothing when both are within half the last decimal (so
/// a structure already in its cell is not moved again and a second grid gives no operation).
fn shift(ops: &mut Vec<Op>, atoms: Vec<u32>, dx: f64, dy: f64) {
    let still = |v: f64| {
        if v.abs() <= 5e-5 + 1e-9 {
            0.0
        } else {
            round4(v)
        }
    };
    let (dx, dy) = (still(dx), still(dy));
    if dx != 0.0 || dy != 0.0 {
        ops.push(Op::Move {
            atoms,
            objects: Vec::new(),
            dx,
            dy,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_compare_with_their_numbers_as_numbers() {
        let mut t = vec!["TP10", "TP2", "TP1b", "TP1", "parent", "TP02", "tp3"];
        t.sort_by(|a, b| natural(a, b));
        assert_eq!(t, ["TP1", "TP1b", "TP2", "TP02", "TP10", "parent", "tp3"]);
        assert_eq!(natural("", "a"), Ordering::Less);
        assert_eq!(natural("a9", "a10"), Ordering::Less);
    }

    #[test]
    fn the_arguments_are_read_and_a_bad_one_is_refused() {
        let o = Opts::parse(
            &serde_json::json!({"cols": 3, "gap": 0.5, "order": "title", "atoms": [1, 2]}),
        )
        .unwrap();
        assert_eq!(
            (o.cols, o.gap, o.order, o.atoms),
            (3, 0.5, Order::Title, vec![1, 2])
        );
        assert_eq!(
            Opts::parse(&serde_json::json!({})).unwrap(),
            Opts::default()
        );
        for bad in [
            serde_json::json!({"cols": 0}),
            serde_json::json!({"cols": 1.5}),
            serde_json::json!({"cols": MAX_COLS + 1}),
            serde_json::json!({"gap": 0}),
            serde_json::json!({"gap": -1}),
            serde_json::json!({"order": "size"}),
            serde_json::json!({"atoms": [-1]}),
            serde_json::json!({"wrap": 3}),
            serde_json::json!([]),
        ] {
            assert_eq!(Opts::parse(&bad), Err("error.draw2.request"), "{bad}");
        }
        for w in ORDERS {
            assert_eq!(Order::parse(w).map(Order::name), Some(w));
        }
    }
}
