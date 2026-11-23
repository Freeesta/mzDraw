//! A list of transformation products (tp-08): the rows of a suspect-screening result (SMILES, name, parent, m/z, retention time,
//! adduct, level) become the drawing in one step, and the products are numbered.
//!
//! **Rows.** `smiles,name,parent,mz,rt,adduct,level` (and the optional column `area`) as CSV (RFC 4180: quoted fields, `""` for a
//! quote, line breaks inside quotes) or as JSON objects with the same keys. `parent` names other rows by their `name`; several
//! parents are separated by `;` (a convergent product); empty is a root. The line of a row is the line of the CSV on which its
//! record starts (the header is line 1), or the 1-based index of a JSON row.
//!
//! **Data kept on the structure** (the SDF data of the structure, `atom.data`, on one atom of it; the names are a document
//! convention that mzLab's CLI reads, listed in `FIELDS`, in this order): `TP_ID` (the shown name), `MZ` (observed m/z, as typed),
//! `RT` (minutes, as typed), `ADDUCT` (a word of the form `[M+H]+`, `adduct_ok`), `AREA` (optional, never drawn). The confidence level is the annotation
//! (`ann`), on the scale it belongs to.
//!
//! **Atomic.** Every row is checked first; when any fails nothing is built and the failing rows are returned, every one of them, in
//! line order, each with its i18n key. Then one list of plain operations (`draw_edit/1`) is made on a copy of the drawing: the
//! structures and the anchored arrows, `scheme:orient` on each parent -> product pair, the tree layout of `pathway:tree`, the labels
//! of the arrows (`pathway:label`) and the names of the rows that had none. The caller runs that list as one undo step, so a log of it replays to the same bytes.
//!
//! **Numbering** (`number_ops`, the command `tp:number`): `TP1`, `TP2`, ... in the order of the setting `pathway.number` (`layout`:
//! by layer, then by position in the layer; `mz`; `rt`). A root keeps its name; a name that is not `TP<n>` is kept unless `force`.

use crate::doc::{Data, Document, ObjKind};
use crate::ops::Op;
use crate::settings::Style;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// The data fields of a product, in the order they are written.
pub const FIELDS: [&str; 5] = ["TP_ID", "MZ", "RT", "ADDUCT", "AREA"];

/// The columns of a list, in the order of the header `smiles,name,parent,mz,rt,adduct,level`, then the optional one.
pub const COLUMNS: [&str; 8] = [
    "smiles", "name", "parent", "mz", "rt", "adduct", "level", "area",
];

/// Whether a word has the form of an adduct, `[M<terms>]<n><+|->` (`[M+H]+`, `[M+H-H2O]+`, `[M+2H]2+`, `[M-H]-`, `[M]-`): at most 32 characters of letters, digits and signs, every sign followed by a letter or
/// a digit, a charge of up to two digits and a sign after the bracket. It is a syntax check only: whether the adduct is a known one,
/// and whether the m/z fits the structure, are not judged here (the table of adducts is chemistry and is not held by this crate).
pub fn adduct_ok(a: &str) -> bool {
    if a.len() > 32 || !a.starts_with("[M") {
        return false;
    }
    let Some(close) = a.find(']') else {
        return false;
    };
    let inner = &a.as_bytes()[2..close];
    if !inner
        .iter()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-'))
    {
        return false;
    }
    for (i, &c) in inner.iter().enumerate() {
        let sign = matches!(c, b'+' | b'-');
        if (i == 0 && !sign) || (sign && !inner.get(i + 1).is_some_and(u8::is_ascii_alphanumeric)) {
            return false;
        }
    }
    let tail = &a[close + 1..];
    let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
    digits <= 2 && matches!(&tail[digits..], "+" | "-")
}

/// Rows and bytes of one list, the atoms of one row and of all rows.
pub const MAX_ROWS: usize = 10_000;
pub const MAX_BYTES: usize = 1_000_000;
pub const MAX_ATOMS: usize = 500;
/// (The document finds an atom or a bond by scanning, so adding n atoms costs about n squared: 15,000 atoms import in under a second in
/// release, 30,000 in five. The cap is that cost's, not a limit of the format.)
pub const MAX_TOTAL_ATOMS: usize = 15_000;
/// The longest SMILES of a row, in characters (500 atoms with their rings, branches and bonds fit well within it).
pub const MAX_SMILES: usize = 5_000;

/// The keys of a refused list (the rows have their own, `error.draw2.tpRow...`).
pub const KEY_ROWS: &str = "error.draw2.tpRows";
const KEY_EMPTY: &str = "error.draw2.tpEmpty";
const KEY_HEADER: &str = "error.draw2.tpHeader";
const KEY_SIZE: &str = "error.draw2.tpListSize";
const KEY_ARGS: &str = "error.draw2.request";

const ROW_SHAPE: &str = "error.draw2.tpRowShape";
const ROW_SMILES: &str = "error.draw2.tpRowSmiles";
const ROW_SIZE: &str = "error.draw2.tpRowSize";
const ROW_NAME: &str = "error.draw2.tpRowName";
const ROW_DUP: &str = "error.draw2.tpRowDup";
const ROW_NUMBER: &str = "error.draw2.tpRowNumber";
const ROW_ADDUCT: &str = "error.draw2.tpRowAdduct";
const ROW_LEVEL: &str = "error.draw2.tpRowLevel";
const ROW_PARENT: &str = "error.draw2.tpRowParent";
const ROW_CYCLE: &str = "error.draw2.tpRowCycle";

/// The order `tp:number` numbers the products in (the setting `pathway.number`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Number {
    /// By layer, then by position in the layer.
    Layout,
    Mz,
    Rt,
}

impl Number {
    pub fn parse(v: &str) -> Option<Number> {
        match v {
            "layout" => Some(Number::Layout),
            "mz" => Some(Number::Mz),
            "rt" => Some(Number::Rt),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Number::Layout => "layout",
            Number::Mz => "mz",
            Number::Rt => "rt",
        }
    }
}

/// One row of a list.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub line: u32,
    pub smiles: String,
    pub name: String,
    pub parents: Vec<String>,
    pub mz: String,
    pub rt: String,
    pub adduct: String,
    pub level: String,
    pub area: String,
    /// The identifier of the annotation (only a TP table, `tptable`, has this column).
    pub identifier: String,
}

/// Why a list is not imported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The list as a whole: an i18n key and the id it is about (0).
    Key(&'static str, u32),
    /// The failing rows, `(line, key)`, every one of them, in line order.
    Rows(Vec<(u32, String)>),
}

/// The text of a list in CSV, or the rows as JSON.
#[derive(Clone, Debug)]
pub enum Input<'a> {
    Csv(&'a str),
    Rows(&'a serde_json::Value),
}

/// Whether a text is a list as the Add field takes it: its first line is a CSV header that starts with `smiles` and names only
/// columns of `COLUMNS`.
pub fn looks_like(text: &str) -> bool {
    let t = text.trim_start_matches('\u{feff}').trim_start();
    let first = t.lines().next().unwrap_or("");
    let cols: Vec<String> = first
        .split(',')
        .map(|c| c.trim().to_ascii_lowercase())
        .collect();
    let list =
        cols.len() > 1 && cols[0] == "smiles" && cols.iter().all(|c| COLUMNS.contains(&c.as_str()));
    list || is_table_header(&cols)
}

/// The columns of a TP table (`tptable`, tp-09) in the order it writes them. A TP table is read as a list: `tp_id` is the name,
/// `parent_id` the parent, `mz_obs` the m/z; the columns in `DERIVED` are recomputed by the table and are not read.
pub const TABLE_COLUMNS: [&str; 15] = crate::tptable::COLUMNS;

/// The columns of a TP table that the table computes from the drawing: they are never read back.
pub const DERIVED: [&str; 7] = crate::tptable::DERIVED;

/// Whether the (lower-cased) columns are the header of a TP table: `tp_id` and `smiles` among them, and only columns of the table.
fn is_table_header(cols: &[String]) -> bool {
    cols.iter().any(|c| c == "tp_id")
        && cols.iter().any(|c| c == "smiles")
        && cols.iter().all(|c| TABLE_COLUMNS.contains(&c.as_str()))
}

/// How many derived columns (`DERIVED`) the header of a TP table has: they are not read, and the page says so. 0 for a list.
pub fn table_ignored(text: &str) -> usize {
    let t = text.trim_start_matches('\u{feff}').trim_start();
    let cols: Vec<String> = t
        .lines()
        .next()
        .unwrap_or("")
        .split(',')
        .map(|c| c.trim().to_ascii_lowercase())
        .collect();
    if !is_table_header(&cols) {
        return 0;
    }
    cols.iter()
        .filter(|c| DERIVED.contains(&c.as_str()))
        .count()
}

// ---------------------------------------------------------------------------------------------------------------------------
// reading

/// The records of a CSV text (RFC 4180) as (line, fields). A blank line is no record. `None`: a quote is never closed.
fn records(text: &str) -> Option<Vec<(u32, Vec<String>)>> {
    let mut out = Vec::new();
    let mut fields: Vec<String> = Vec::new();
    let mut cur = String::new();
    let (mut line, mut start) = (1u32, 1u32);
    let (mut quoted, mut was_quoted, mut any) = (false, false, false);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    cur.push('"');
                }
                '"' => quoted = false,
                '\n' => {
                    line += 1;
                    cur.push('\n');
                }
                c => cur.push(c),
            }
            continue;
        }
        match c {
            '"' if cur.is_empty() && !was_quoted => {
                quoted = true;
                was_quoted = true;
                any = true;
            }
            ',' => {
                fields.push(std::mem::take(&mut cur));
                was_quoted = false;
                any = true;
            }
            '\r' => {}
            '\n' => {
                if any || !cur.is_empty() {
                    fields.push(std::mem::take(&mut cur));
                    out.push((start, std::mem::take(&mut fields)));
                }
                (any, was_quoted) = (false, false);
                line += 1;
                start = line;
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if quoted {
        return None;
    }
    if any || !cur.is_empty() {
        fields.push(cur);
        out.push((start, fields));
    }
    Some(out)
}

fn split_parents(text: &str) -> Vec<String> {
    text.split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

fn set_cell(row: &mut Row, col: &str, value: &str) {
    let v = value.trim();
    match col {
        "smiles" => row.smiles = v.to_string(),
        "name" | "tp_id" => row.name = v.to_string(),
        "parent" | "parent_id" => row.parents = split_parents(v),
        "mz" | "mz_obs" => row.mz = v.to_string(),
        "rt" => row.rt = v.to_string(),
        "adduct" => row.adduct = v.to_string(),
        "level" => row.level = v.to_string(),
        "identifier" => row.identifier = v.to_string(),
        "area" => row.area = v.to_string(),
        // a derived column of a TP table: not read
        _ => {}
    }
}

/// The rows read, and the rows that could not be read as `(line, key)`; or the list refused as a whole.
type Read = Result<(Vec<Row>, Vec<(u32, String)>), Failure>;

/// The rows of a CSV text. A header with a column that is not in `COLUMNS`, a repeated one or none called `smiles` is
/// `error.draw2.tpHeader`; a record with another number of fields than the header, or an unclosed quote, is a failing row (the
/// second value: the rows that failed to be read).
fn read_csv(text: &str) -> Read {
    if text.len() > MAX_BYTES {
        return Err(Failure::Key(KEY_SIZE, 0));
    }
    let text = text.trim_start_matches('\u{feff}');
    let Some(recs) = records(text) else {
        let line = text.lines().count().max(1) as u32;
        return Err(Failure::Rows(vec![(line, ROW_SHAPE.to_string())]));
    };
    let mut it = recs.into_iter();
    let Some((_, head)) = it.next() else {
        return Err(Failure::Key(KEY_EMPTY, 0));
    };
    let cols: Vec<String> = head.iter().map(|c| c.trim().to_ascii_lowercase()).collect();
    let uniq: HashSet<&String> = cols.iter().collect();
    // a TP table (`tptable`) has its own column names; a header that mixes the two kinds is refused
    let table = is_table_header(&cols);
    if uniq.len() != cols.len()
        || !cols.iter().any(|c| c == "smiles")
        || !(table || cols.iter().all(|c| COLUMNS.contains(&c.as_str())))
    {
        return Err(Failure::Key(KEY_HEADER, 0));
    }
    let (mut rows, mut bad) = (Vec::new(), Vec::new());
    for (line, fields) in it {
        if rows.len() + bad.len() >= MAX_ROWS {
            return Err(Failure::Key(KEY_SIZE, 0));
        }
        if fields.len() != cols.len() {
            bad.push((line, ROW_SHAPE.to_string()));
            continue;
        }
        let mut row = Row {
            line,
            ..Row::default()
        };
        for (c, v) in cols.iter().zip(&fields) {
            set_cell(&mut row, c, v);
        }
        rows.push(row);
    }
    if table {
        // the spreadsheet guard of the table writer (`tptable::unguard`): the exact inverse of what it wrote in every text cell
        for r in &mut rows {
            let mut cells = vec![
                &mut r.name,
                &mut r.adduct,
                &mut r.level,
                &mut r.identifier,
                &mut r.mz,
                &mut r.rt,
            ];
            cells.extend(r.parents.iter_mut());
            for c in cells {
                if crate::tptable::unguard(c).len() != c.len() {
                    c.remove(0);
                }
            }
        }
        merge_pairs(&mut rows, &mut bad);
    }
    Ok((rows, bad))
}

/// A TP table has one row per parent-TP pair, so a product with two parents is on two rows with the same `tp_id`. Rows with the
/// same non-empty name become one: the parents are joined, every other cell that two rows both have must be equal (a row that
/// says otherwise is a failing row, `tpRowDup`, as two different rows with one name are in a list).
fn merge_pairs(rows: &mut Vec<Row>, bad: &mut Vec<(u32, String)>) {
    let mut at: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<Row> = Vec::with_capacity(rows.len());
    for r in std::mem::take(rows) {
        let first = (!r.name.is_empty())
            .then(|| at.get(&r.name).copied())
            .flatten();
        let Some(k) = first else {
            if !r.name.is_empty() {
                at.insert(r.name.clone(), out.len());
            }
            out.push(r);
            continue;
        };
        let m = &mut out[k];
        let same = |a: &str, b: &str| a.is_empty() || b.is_empty() || a == b;
        if !(same(&m.smiles, &r.smiles)
            && same(&m.mz, &r.mz)
            && same(&m.rt, &r.rt)
            && same(&m.adduct, &r.adduct)
            && same(&m.level, &r.level)
            && same(&m.identifier, &r.identifier)
            && same(&m.area, &r.area))
        {
            bad.push((r.line, ROW_DUP.to_string()));
            continue;
        }
        for (a, b) in [
            (&mut m.smiles, &r.smiles),
            (&mut m.mz, &r.mz),
            (&mut m.rt, &r.rt),
            (&mut m.adduct, &r.adduct),
            (&mut m.level, &r.level),
            (&mut m.identifier, &r.identifier),
            (&mut m.area, &r.area),
        ] {
            if a.is_empty() {
                a.clone_from(b);
            }
        }
        for p in r.parents {
            if !m.parents.contains(&p) {
                m.parents.push(p);
            }
        }
    }
    *rows = out;
}

/// The rows of a JSON list: an array of objects with the keys of `COLUMNS`; a value is a string or a number (`parent` may also be
/// an array of names). Another key or another shape is a failing row.
fn read_json(v: &serde_json::Value) -> Read {
    let list = v.as_array().ok_or(Failure::Key(KEY_ARGS, 0))?;
    if list.len() > MAX_ROWS || v.to_string().len() > MAX_BYTES {
        return Err(Failure::Key(KEY_SIZE, 0));
    }
    let (mut rows, mut bad) = (Vec::new(), Vec::new());
    'rows: for (i, item) in list.iter().enumerate() {
        let line = i as u32 + 1;
        let Some(obj) = item.as_object() else {
            bad.push((line, ROW_SHAPE.to_string()));
            continue;
        };
        let mut row = Row {
            line,
            ..Row::default()
        };
        for (k, val) in obj {
            if !COLUMNS.contains(&k.as_str()) {
                bad.push((line, ROW_SHAPE.to_string()));
                continue 'rows;
            }
            let text = match val {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Null => String::new(),
                serde_json::Value::Array(a) if k == "parent" => {
                    let names: Option<Vec<&str>> = a.iter().map(|x| x.as_str()).collect();
                    match names {
                        Some(n) => n.join(";"),
                        None => {
                            bad.push((line, ROW_SHAPE.to_string()));
                            continue 'rows;
                        }
                    }
                }
                _ => {
                    bad.push((line, ROW_SHAPE.to_string()));
                    continue 'rows;
                }
            };
            set_cell(&mut row, k, &text);
        }
        rows.push(row);
    }
    Ok((rows, bad))
}

// ---------------------------------------------------------------------------------------------------------------------------
// checking

fn number_ok(text: &str, min_exclusive: bool) -> bool {
    text.parse::<f64>()
        .is_ok_and(|x| x.is_finite() && if min_exclusive { x > 0.0 } else { x >= 0.0 })
}

/// The data of a row, in the order of `FIELDS`; empty fields are left out. `id` is the shown name.
fn row_data(id: &str, row: &Row) -> Data {
    [
        ("TP_ID", id),
        ("MZ", &row.mz),
        ("RT", &row.rt),
        ("ADDUCT", &row.adduct),
        ("AREA", &row.area),
    ]
    .into_iter()
    .filter(|(_, v)| !v.is_empty())
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

/// The annotation text of a level (`ann::canon`): on the scale of Schymanski et al. when the level belongs to it, on the scale for
/// PFAS when it belongs to that one, else on the first (a level of neither is kept as written, which the page shows as "of the file").
fn level_ann(level: &str, identifier: &str) -> Option<String> {
    let scale = if crate::ann::PFAS_LEVELS.contains(&level) && !crate::ann::LEVELS.contains(&level)
    {
        crate::ann::PFAS_SCALE
    } else {
        crate::ann::SCALE
    };
    crate::ann::entry(scale, level, identifier, "", "", "")
}

/// The first key a row fails on, without its structure (read separately) and its parents (read for the whole list).
fn row_problem(row: &Row) -> Option<&'static str> {
    if !row.name.is_empty()
        && (row.name.contains(';')
            || !crate::doc::data_valid(&[("TP_ID".to_string(), row.name.clone())]))
    {
        return Some(ROW_NAME);
    }
    if (!row.mz.is_empty() && !number_ok(&row.mz, true))
        || (!row.rt.is_empty() && !number_ok(&row.rt, false))
        || (!row.area.is_empty() && !number_ok(&row.area, false))
    {
        return Some(ROW_NUMBER);
    }
    if !row.adduct.is_empty() && !adduct_ok(&row.adduct) {
        return Some(ROW_ADDUCT);
    }
    // an identifier belongs to the annotation, which has a level
    if (!row.level.is_empty() && level_ann(&row.level, &row.identifier).is_none())
        || (row.level.is_empty() && !row.identifier.is_empty())
    {
        return Some(ROW_LEVEL);
    }
    None
}

/// An upper bound of the atoms a SMILES draws, counted in its text without reading it as a molecule: a bracket atom is one, so are
/// `*`, a letter that starts an element (`Cl` and `Br` are one) and an aromatic `b c n o p s`. It lets a row that is too large be
/// refused before any structure is built (the reader is not linear in a row, and a hostile row must not be slow).
fn atoms_in(smiles: &str) -> usize {
    let b = smiles.as_bytes();
    let (mut n, mut i) = (0usize, 0usize);
    while i < b.len() {
        match b[i] {
            b'[' => {
                n += 1;
                while i < b.len() && b[i] != b']' {
                    i += 1;
                }
            }
            b'*' => n += 1,
            b'C' if b.get(i + 1) == Some(&b'l') => {
                n += 1;
                i += 1;
            }
            b'B' if b.get(i + 1) == Some(&b'r') => {
                n += 1;
                i += 1;
            }
            c if c.is_ascii_uppercase() || matches!(c, b'b' | b'c' | b'n' | b'o' | b'p' | b's') => {
                n += 1
            }
            _ => {}
        }
        i += 1;
    }
    n
}

/// The structure of a row: the drawing of its SMILES (coordinates computed by the door a SMILES text takes), one connected piece
/// of 1 to `MAX_ATOMS` atoms.
fn structure(row: &Row) -> Result<Document, &'static str> {
    let s = &row.smiles;
    if s.len() > MAX_SMILES || atoms_in(s) > MAX_ATOMS {
        return Err(ROW_SIZE);
    }
    if s.is_empty() || s.chars().any(char::is_whitespace) || s.starts_with(['{', '<']) {
        return Err(ROW_SMILES);
    }
    let (d, info) = crate::io::import(s).map_err(|_| ROW_SMILES)?;
    if info.format != "smiles" || d.atoms.is_empty() || !d.objects.is_empty() {
        return Err(ROW_SMILES);
    }
    if d.atoms.len() > MAX_ATOMS {
        return Err(ROW_SIZE);
    }
    if crate::layout::pieces(&d).len() != 1 {
        return Err(ROW_SMILES);
    }
    Ok(d)
}

/// Rows that lie on a directed cycle of parent links (a row that is its own parent included), by index.
fn on_cycles(parents: &[Vec<usize>]) -> BTreeSet<usize> {
    // Tarjan's strongly connected components, iterative (a list holds up to `MAX_ROWS` rows)
    let n = parents.len();
    let (mut index, mut low) = (vec![usize::MAX; n], vec![0usize; n]);
    let (mut on, mut stack) = (vec![false; n], Vec::new());
    let (mut counter, mut cyc) = (0usize, BTreeSet::new());
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut work = vec![(root, 0usize)];
        while let Some(&mut (v, ref mut k)) = work.last_mut() {
            if *k == 0 {
                index[v] = counter;
                low[v] = counter;
                counter += 1;
                stack.push(v);
                on[v] = true;
            }
            if let Some(&w) = parents[v].get(*k) {
                *k += 1;
                if index[w] == usize::MAX {
                    work.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(index[w]);
                }
                continue;
            }
            work.pop();
            if let Some(&(u, _)) = work.last() {
                low[u] = low[u].min(low[v]);
            }
            if low[v] == index[v] {
                let mut comp = Vec::new();
                while let Some(w) = stack.pop() {
                    on[w] = false;
                    comp.push(w);
                    if w == v {
                        break;
                    }
                }
                if comp.len() > 1 || parents[v].contains(&v) {
                    cyc.extend(comp);
                }
            }
        }
    }
    cyc
}

// ---------------------------------------------------------------------------------------------------------------------------
// the drawing

/// The atoms of a drawing by id, built once: a lookup in the document is a scan of its atoms, and the structures are many.
type Index<'a> = HashMap<u32, &'a crate::doc::Atom>;

fn index(doc: &Document) -> Index<'_> {
    doc.atoms.iter().map(|a| (a.id, a)).collect()
}

/// The data of a structure: the anchor atom (the first atom that has data, the one that has `TP_ID` first, else the smallest id)
/// and the union of the data of its atoms (ascending ids).
fn data_of(ix: &Index, piece: &[u32]) -> (u32, Data) {
    let mut ids: Vec<u32> = piece.to_vec();
    ids.sort_unstable();
    let mut anchor = ids.first().copied().unwrap_or(0);
    let (mut data, mut found, mut with_id) = (Data::new(), false, false);
    for a in ids.iter().filter_map(|i| ix.get(i)) {
        let d = a.data();
        if d.is_empty() {
            continue;
        }
        let has_id = d.iter().any(|(k, _)| k == "TP_ID");
        if !found || (has_id && !with_id) {
            (anchor, found, with_id) = (a.id, true, has_id);
        }
        data.extend(d);
    }
    (anchor, data)
}

/// The operations that give the structure `piece` the shown name `id`: `TP_ID` set in the data of the anchor atom, first in the
/// list, and taken off any other atom of the structure.
fn name_ops(ix: &Index, piece: &[u32], id: &str) -> Vec<Op> {
    let (anchor, _) = data_of(ix, piece);
    let mut ids: Vec<u32> = piece.to_vec();
    ids.sort_unstable();
    let mut ops = Vec::new();
    for a in ids.iter().filter_map(|i| ix.get(i)) {
        let mut d = a.data();
        let had = d.iter().any(|(k, _)| k == "TP_ID");
        if a.id != anchor && !had {
            continue;
        }
        d.retain(|(k, _)| k != "TP_ID");
        if a.id == anchor {
            d.insert(0, ("TP_ID".to_string(), id.to_string()));
        }
        if d != a.data() {
            ops.push(Op::AtomData { id: a.id, data: d });
        }
    }
    ops
}

/// Whether a name is `TP` and a number.
fn is_tp_name(name: &str) -> bool {
    name.strip_prefix("TP")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// The numeric data field of a structure (the first one in atom id order), when it is a finite number.
fn field(ix: &Index, atoms: &[u32], name: &str) -> Option<f64> {
    let mut ids: Vec<u32> = atoms.to_vec();
    ids.sort_unstable();
    for a in ids.iter().filter_map(|i| ix.get(i)) {
        for (k, v) in a.data() {
            if k.trim().eq_ignore_ascii_case(name) {
                return v.trim().parse::<f64>().ok().filter(|x| x.is_finite());
            }
        }
    }
    None
}

/// The structures of a pathway in an order: the atoms of each and its layer.
pub type Ordered = Vec<(Vec<u32>, usize)>;

/// The structures of the pathway (a root only when `roots`; else the products, the structures an anchored arrow reaches), in the
/// order `number` gives, as the atoms of each with its layer (tp-05). The error is the key of the layout's refusal (no anchored arrow,
/// a cycle...).
fn ordered(
    doc: &Document,
    style: &Style,
    number: Number,
    roots: bool,
) -> Result<Ordered, (&'static str, u32)> {
    let lay = crate::pathway::layout(doc, style).map_err(|r| (r.key(), r.id()))?;
    let ix = index(doc);
    let reached: HashSet<usize> = lay.edges.iter().map(|e| e.to).collect();
    let mut list: Vec<usize> = (0..lay.nodes.len())
        .filter(|i| roots || reached.contains(i))
        .collect();
    // the key of each node, computed once
    let key: HashMap<usize, f64> = list
        .iter()
        .map(|&i| {
            let r = lay.nodes[i].rect;
            let v = match number {
                Number::Layout if style.pathway_down => (r[0] + r[2]) / 2.0,
                Number::Layout => (r[1] + r[3]) / 2.0,
                Number::Mz => field(&ix, &lay.nodes[i].atoms, "MZ").unwrap_or(f64::INFINITY),
                Number::Rt => field(&ix, &lay.nodes[i].atoms, "RT").unwrap_or(f64::INFINITY),
            };
            (i, v)
        })
        .collect();
    list.sort_by(|a, b| {
        let (x, y) = (&lay.nodes[*a], &lay.nodes[*b]);
        let layer = if number == Number::Layout {
            x.layer.cmp(&y.layer)
        } else {
            std::cmp::Ordering::Equal
        };
        layer.then(key[a].total_cmp(&key[b])).then(x.id.cmp(&y.id))
    });
    Ok(list
        .into_iter()
        .map(|i| (lay.nodes[i].atoms.clone(), lay.nodes[i].layer))
        .collect())
}

/// The products of the drawing, in the order `number` gives, as the atoms of each.
fn products(
    doc: &Document,
    style: &Style,
    number: Number,
) -> Result<Vec<Vec<u32>>, (&'static str, u32)> {
    Ok(ordered(doc, style, number, false)?
        .into_iter()
        .map(|p| p.0)
        .collect())
}

/// Every structure of the pathway (roots too) with its layer, in the order of `pathway.number` as `tp:number` numbers the products
/// (the table of tp-09 is written in this order); `None` when the drawing has no anchored arrow, so no pathway.
pub fn table_order(doc: &Document, style: &Style) -> Result<Option<Ordered>, (&'static str, u32)> {
    match ordered(doc, style, style.pathway_number, true) {
        Ok(v) => Ok(Some(v)),
        Err(("error.draw2.schemeEmpty", _)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// The operations of `tp:number`: `TP1`, `TP2`, ... on the products in the order of `number`. A root keeps its name; a product whose
/// name is not `TP<n>` keeps it unless `force` (and does not use up a number).
pub fn number_ops(
    doc: &Document,
    style: &Style,
    number: Number,
    force: bool,
) -> Result<Vec<Op>, (&'static str, u32)> {
    let mut ops = Vec::new();
    let mut k = 0u32;
    let ix = index(doc);
    for piece in products(doc, style, number)? {
        let (_, data) = data_of(&ix, &piece);
        let now = data
            .iter()
            .find(|(n, _)| n == "TP_ID")
            .map_or("", |(_, v)| v.as_str());
        if !now.is_empty() && !is_tp_name(now) && !force {
            continue;
        }
        k += 1;
        let id = format!("TP{k}");
        if now != id {
            ops.extend(name_ops(&ix, &piece, &id));
        }
    }
    Ok(ops)
}

/// The operations that import the rows into `doc`: the structures with their data and annotation, an anchored arrow for each
/// parent link, `scheme:orient`, the layout of `pathway:tree`, and `TP<n>` for the rows that had no name. Nothing is changed here.
pub fn import_ops(doc: &Document, style: &Style, input: &Input) -> Result<Vec<Op>, Failure> {
    let (rows, mut bad) = match input {
        Input::Csv(t) => read_csv(t)?,
        Input::Rows(v) => read_json(v)?,
    };
    if rows.is_empty() && bad.is_empty() {
        return Err(Failure::Key(KEY_EMPTY, 0));
    }
    // the names, then each row on its own
    let mut at: HashMap<&str, usize> = HashMap::new();
    let mut fail: BTreeMap<u32, &'static str> = BTreeMap::new();
    for (i, r) in rows.iter().enumerate() {
        if !r.name.is_empty() && at.insert(&r.name, i).is_some() {
            fail.insert(r.line, ROW_DUP);
        }
    }
    // the atoms of the whole list, counted in the text before any structure is built
    // (a row above the row cap is refused by itself below: it counts for the cap, not for its size)
    let pre: usize = rows
        .iter()
        .map(|r| atoms_in(&r.smiles).min(MAX_ATOMS))
        .sum();
    if pre > MAX_TOTAL_ATOMS {
        return Err(Failure::Key(KEY_SIZE, 0));
    }
    let mut docs: Vec<Option<Document>> = Vec::with_capacity(rows.len());
    let mut total = 0usize;
    for r in &rows {
        let d = structure(r);
        let key = d.as_ref().err().copied().or_else(|| row_problem(r));
        if let Some(k) = key {
            fail.entry(r.line).or_insert(k);
        }
        let d = d.ok();
        total += d.as_ref().map_or(0, |d| d.atoms.len());
        if total > MAX_TOTAL_ATOMS {
            return Err(Failure::Key(KEY_SIZE, 0));
        }
        docs.push(d);
    }
    // the parents: a name no row has, and the rows on a cycle
    let mut links: Vec<Vec<usize>> = vec![Vec::new(); rows.len()];
    for (i, r) in rows.iter().enumerate() {
        for p in &r.parents {
            match at.get(p.as_str()) {
                Some(&j) => links[i].push(j),
                None => {
                    fail.entry(r.line).or_insert(ROW_PARENT);
                }
            }
        }
    }
    for i in on_cycles(&links) {
        fail.entry(rows[i].line).or_insert(ROW_CYCLE);
    }
    bad.extend(fail.into_iter().map(|(l, k)| (l, k.to_string())));
    if !bad.is_empty() {
        bad.sort_by_key(|b| b.0);
        return Err(Failure::Rows(bad));
    }
    // the structures, in a column to the right of the drawing, then the arrows
    let right = doc.bounds().map_or(0.0, |b| b.2 + 2.0);
    let top = doc.bounds().map_or(0.0, |b| b.1);
    let mut base = doc.next - 1;
    let mut y = top;
    let mut ops = Vec::new();
    let mut first: Vec<u32> = Vec::new();
    for (r, d) in rows.iter().zip(docs) {
        let mut d = d.unwrap_or_default();
        let b = d.bounds().unwrap_or((0.0, 0.0, 0.0, 0.0));
        let anchor = d.atoms.iter().map(|a| a.id).min().unwrap_or(0);
        for a in &mut d.atoms {
            if a.id == anchor {
                a.set_data(&row_data(&r.name, r));
                if !r.level.is_empty() {
                    a.set_ann(&level_ann(&r.level, &r.identifier).unwrap_or_default());
                }
            }
        }
        let n = d.next - 1;
        ops.extend(crate::editor::place_ops(base, &d, right - b.0, y - b.1));
        first.push(base + anchor);
        base += n;
        y += (b.3 - b.1) + 1.5;
    }
    let mut next = base + 1;
    let mut arrows = 0usize;
    for i in 0..rows.len() {
        let mut seen = BTreeSet::new();
        for &j in &links[i] {
            if !seen.insert(j) {
                continue;
            }
            ops.push(Op::ObjAdd {
                id: Some(next),
                t: ObjKind::Arrow,
                x: 0.0,
                y: 0.0,
                x2: 1.0,
                y2: 0.0,
                text: String::new(),
                runs: Vec::new(),
                from: first[j],
                to: first[i],
                cv: Default::default(),
            });
            next += 1;
            arrows += 1;
        }
    }
    // on a copy, so that each step sees the drawing the steps before it made
    let mut scratch = doc.clone();
    let key = |e: crate::ops::OpError| Failure::Key(e.key, e.id);
    scratch.apply(&ops).map_err(key)?;
    let pathway = arrows > 0 || scratch.objects.iter().any(|o| o.from != 0 || o.to != 0);
    if pathway {
        let plan = crate::orient::plan(&scratch);
        if !plan.ops.is_empty() {
            scratch.apply(&plan.ops).map_err(key)?;
            ops.extend(plan.ops);
        }
        let tree =
            crate::pathway::command(&scratch, style).map_err(|r| Failure::Key(r.key(), r.id()))?;
        scratch.apply(&tree).map_err(key)?;
        ops.extend(tree);
        // the label of each arrow: the formula delta of the two structures (`pathway:label`, the mode `pathway.label`)
        // (the import writes the delta also when the mode has numbers: it has no engine, and `pathway:label` rewrites the label after it)
        let mode = match style.pathway_label {
            m if m.wants_mass() => crate::steplabel::Mode::Delta,
            m => m,
        };
        let labels =
            crate::steplabel::plan(&scratch, mode, &crate::steplabel::Opts::default(), None)
                .map_err(key)?;
        scratch.apply(&labels.ops).map_err(key)?;
        ops.extend(labels.ops);
    }
    // the rows with no name: the smallest numbers that no structure has, in the order of `pathway.number` (the order of the rows
    // when there is no pathway)
    let pieces = crate::layout::pieces(&scratch);
    let owner: HashMap<u32, usize> = (pieces.iter().enumerate())
        .flat_map(|(k, p)| p.iter().map(move |&a| (a, k)))
        .collect();
    let piece_of = |atom: u32| owner.get(&atom).map(|&k| pieces[k].clone());
    let ix = index(&scratch);
    let mut used: HashSet<u32> = HashSet::new();
    for p in &pieces {
        let (_, d) = data_of(&ix, p);
        if let Some((_, v)) = d.iter().find(|(k, _)| k == "TP_ID") {
            used.extend(v.strip_prefix("TP").and_then(|n| n.parse::<u32>().ok()));
        }
    }
    let nameless: HashSet<u32> = rows
        .iter()
        .zip(&first)
        .filter(|(r, _)| r.name.is_empty())
        .map(|(_, a)| *a)
        .collect();
    let mut order: Vec<Vec<u32>> = if pathway {
        products(&scratch, style, style.pathway_number).map_err(|(k, i)| Failure::Key(k, i))?
    } else {
        first.iter().filter_map(|&a| piece_of(a)).collect()
    };
    order.retain(|p| p.iter().any(|a| nameless.contains(a)));
    let mut k = 0u32;
    for piece in order {
        loop {
            k += 1;
            if !used.contains(&k) {
                break;
            }
        }
        ops.extend(name_ops(&ix, &piece, &format!("TP{k}")));
    }
    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tptable::{self, Table, COLUMNS};

    /// xorshift: repeatable pseudo-random cells, not a dependency.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self, n: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % n.max(1) as u64) as usize
        }
    }

    /// A synthetic cell over the characters that matter to the guard and to RFC 4180. The import trims a cell, so none starts or ends
    /// with white space; `;` is the separator of parents, so a `parent_id` has none.
    fn cell(r: &mut Rng, semicolon: bool) -> String {
        const ALPHABET: [char; 14] = [
            '=', '+', '-', '@', '\'', '"', ',', '\n', '\r', 'a', '1', '.', ' ', ';',
        ];
        let len = r.next(6);
        let s: String = (0..len)
            .map(|_| ALPHABET[r.next(ALPHABET.len() - usize::from(!semicolon))])
            .collect();
        // no edge white space: what the importer trims is not part of a cell (the drawing's data fields are trimmed too)
        s.trim().to_string()
    }

    fn read_one(cells: &[(usize, &str)]) -> Row {
        let mut t = Table::default();
        let mut row = vec![String::new(); COLUMNS.len()];
        row[3] = "CCO".into();
        for (c, v) in cells {
            row[*c] = (*v).to_string();
        }
        t.rows.push(row);
        let (rows, bad) = read_csv(&tptable::csv(&t)).expect("the table header is accepted");
        assert!(bad.is_empty(), "{bad:?}");
        assert_eq!(rows.len(), 1);
        rows.into_iter().next().unwrap()
    }

    #[test]
    fn export_then_import_returns_every_text_cell_exactly() {
        let mut r = Rng(0x9E37_79B9_7F4A_7C15);
        for _ in 0..3000 {
            let c: Vec<String> = (0..7).map(|k| cell(&mut r, k != 1)).collect();
            let got = read_one(&[
                (0, &c[0]),
                (1, &c[1]),
                (6, &c[2]),
                (8, &c[3]),
                (10, &c[4]),
                (13, &c[5]),
                (14, &c[6]),
            ]);
            let parents: Vec<String> = if c[1].is_empty() {
                Vec::new()
            } else {
                vec![c[1].clone()]
            };
            let want = (&c[0], &parents, &c[2], &c[3], &c[4], &c[5], &c[6]);
            let have = (
                &got.name,
                &got.parents,
                &got.adduct,
                &got.mz,
                &got.rt,
                &got.level,
                &got.identifier,
            );
            assert_eq!(have, want, "cells {c:?}");
        }
    }

    #[test]
    fn the_guard_is_injective_on_the_cases_that_failed_before() {
        // a cell that really starts with `'` and a guarded one were the same text once written
        for v in ["'=x", "'a", "''", "'", "'+1", "=x", "-CH2", "@x", "'@"] {
            assert_eq!(read_one(&[(0, v)]).name, v, "{v:?}");
        }
        assert_eq!(tptable::unguard("'=x"), "=x");
        assert_eq!(tptable::unguard("''=x"), "'=x");
        // a file made by hand: a `'` before an ordinary character is as written
        assert_eq!(tptable::unguard("'a"), "'a");
        // a number is not guarded, a text that starts like one is
        assert_eq!(read_one(&[(8, "-5.25")]).mz, "-5.25");
        assert_eq!(read_one(&[(8, "-x")]).mz, "-x");
    }
}
