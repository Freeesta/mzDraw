//! The document `mzdraw/1`: atoms and bonds in flat pools with stable ids and 2D coordinates in bond lengths (y up, as in a molfile).
//! A structure is a connected piece; nothing chemical is derived here except the implicit hydrogens a label shows.

use crate::ops::Fields;
use serde_json::Value;

/// The value of the `format` field.
pub const FORMAT: &str = crate::brand::DOCUMENT_ID;

/// Ids stay below this, so that id arithmetic (the next id, the ids of a pasted copy) can never overflow.
pub const MAX_ID: u32 = 1 << 31;

/// Coordinates stay within this many bond lengths of the origin, and finite: a number that cannot be written as JSON never
/// enters a document.
pub const MAX_COORD: f64 = 1e6;

fn in_range(v: f64) -> bool {
    v.is_finite() && v.abs() <= MAX_COORD
}

/// The fields of a document, an atom, a bond or an object that this version does not know: (key, value as JSON text) in the
/// order of the keys. They are kept on read and written back unchanged after the known fields, so a drawing saved by a later
/// mzDraw of the same format version survives a round through this one. Kept as text because they are only written back.
pub type More = Vec<(String, String)>;

/// The unknown fields of a JSON object: every key not in `known`.
fn more_of(v: &Value, known: &[&str]) -> More {
    let mut out: More = match v.as_object() {
        Some(o) => (o.iter())
            .filter(|(k, _)| !known.contains(&k.as_str()))
            .map(|(k, x)| (k.clone(), x.to_string()))
            .collect(),
        None => More::new(),
    };
    crate::m::sort(&mut out);
    out
}

/// Writes `"key":value` pairs after what is already in `out` (which ends inside an open object).
pub(crate) struct Writer(pub(crate) String);

impl Writer {
    pub(crate) fn key(&mut self, k: &str) {
        if !self.0.ends_with('{') {
            self.0.push(',');
        }
        self.0.push_str(&Value::from(k).to_string());
        self.0.push(':');
    }
    pub(crate) fn raw(&mut self, k: &str, json: &str) {
        self.key(k);
        self.0.push_str(json);
    }
    pub(crate) fn num(&mut self, k: &str, v: f64) {
        self.raw(k, &Value::from(v).to_string());
    }
    pub(crate) fn int(&mut self, k: &str, v: i64) {
        self.raw(k, &v.to_string());
    }
    pub(crate) fn text(&mut self, k: &str, v: &str) {
        self.raw(k, &Value::from(v).to_string());
    }
    pub(crate) fn more(&mut self, more: &More) {
        for (k, v) in more {
            self.raw(k, v);
        }
    }
}

/// One atom. `h` is set only when the hydrogens were given explicitly (a file said so); otherwise they follow the valence rules.
/// A labelled atom (`lab`) shows its label alone: no hydrogens, charge or isotope of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct Atom {
    pub id: u32,
    pub el: String,
    pub x: f64,
    pub y: f64,
    pub chg: i8,
    pub iso: u16,
    pub rad: u8,
    pub h: Option<u8>,
    /// A label drawn instead of the element: an abbreviation of `label::ABBREVIATIONS` (expanded to its atoms at export) or an
    /// R-group placeholder. Empty for a plain atom.
    pub lab: String,
    /// The atom-to-atom map number of a reaction, 1..=999; 0 for none (`MAX_MAP`).
    pub map: u16,
    /// Fields of the atom this version does not know, kept to be written back (see `More`).
    pub more: More,
}

/// The data of a structure (the data fields of an SDF record): name and value pairs, in order. They are kept on one atom of the
/// structure, as its field `data` (`[["name", "value"], ...]`), so they follow that atom through every edit and every undo; the
/// data of a structure is the data of its atoms in pool order.
pub type Data = Vec<(String, String)>;

/// The largest atom-to-atom map number (the three digits of the molfile field).
pub const MAX_MAP: u16 = 999;

/// The longest title of a structure, in characters (the title line of a molfile is 80 characters wide).
pub const MAX_TITLE: usize = 80;

/// Whether a text is a title a document and a molfile can hold: one line of at most `MAX_TITLE` characters, no control character.
pub fn title_valid(title: &str) -> bool {
    title.chars().count() <= MAX_TITLE && !title.chars().any(char::is_control)
}

/// Fields one atom can carry, and the longest name and value, in characters.
pub const MAX_DATA: (usize, usize, usize) = (100, 80, 10_000);

/// Whether a list is data a document and an SDF can hold: a name is one line without `<` and `>` and none of the fields an
/// annotation is written as (`ann::FIELDS`); a value has no empty line
/// (it would end the field in an SDF) and no line that starts as a header (`>`) or as the end of a record (`$$$$`).
pub fn data_valid(data: &[(String, String)]) -> bool {
    let plain = |s: &str| !s.chars().any(|c| c.is_control() && c != '\n');
    data.len() <= MAX_DATA.0
        && data.iter().all(|(name, value)| {
            let n = name.chars().count();
            let lines = || value.split('\n');
            (1..=MAX_DATA.1).contains(&n)
                && name.trim() == name
                && !crate::ann::FIELDS.contains(&name.as_str())
                && !name.contains(['<', '>', '\n'])
                && plain(name)
                && value.chars().count() <= MAX_DATA.2
                && plain(value)
                && (value.is_empty()
                    || lines().all(|l| {
                        !l.trim().is_empty() && !l.starts_with('>') && !l.starts_with("$$$$")
                    }))
        })
}

impl Atom {
    /// The explicit colour (`ink`): `#rrggbb`, empty for automatic.
    pub fn col(&self) -> String {
        crate::ink::get(&self.more)
    }

    /// The lines kept under the molecule this atom holds the choice of (`props`), empty for none.
    pub fn cap(&self) -> Vec<String> {
        crate::props::get(&self.more)
    }

    /// The data kept on this atom (empty for most atoms; a field `data` of another shape is not data and is left alone).
    pub fn data(&self) -> Data {
        let text = self
            .more
            .iter()
            .find(|m| m.0 == "data")
            .map(|m| m.1.as_str());
        let v: Option<Value> = text.and_then(|t| serde_json::from_str(t).ok());
        let pairs = v.as_ref().and_then(Value::as_array).map(|rows| {
            rows.iter()
                .map(|r| {
                    Some((
                        r.get(0)?.as_str()?.to_string(),
                        r.get(1)?.as_str()?.to_string(),
                    ))
                })
                .collect::<Option<Data>>()
        });
        pairs
            .flatten()
            .filter(|d| data_valid(d))
            .unwrap_or_default()
    }

    /// Replaces the data kept on this atom (`data` must be `data_valid`); an empty list takes the field away.
    pub(crate) fn set_data(&mut self, data: &[(String, String)]) {
        self.more.retain(|m| m.0 != "data");
        if !data.is_empty() {
            let rows: Vec<String> = data
                .iter()
                .map(|(k, v)| format!("[{},{}]", Value::from(k.as_str()), Value::from(v.as_str())))
                .collect();
            self.more
                .push(("data".into(), format!("[{}]", rows.join(","))));
            crate::m::sort(&mut self.more);
        }
    }

    /// The annotation kept on this atom as its text (`ann::canon`), empty for none; a field `ann` of another shape is not an
    /// annotation and is left alone.
    pub fn ann(&self) -> String {
        (self.more.iter().find(|m| m.0 == "ann"))
            .and_then(|m| crate::ann::canon(&m.1))
            .filter(|t| t != "[]")
            .unwrap_or_default()
    }

    /// The annotation of some atoms as one text (`ann::join` of theirs in pool order): the annotation of a structure.
    pub(crate) fn ann_of(atoms: &[Atom], ids: &[u32]) -> String {
        let texts: Vec<String> = (atoms.iter())
            .filter(|a| ids.contains(&a.id))
            .map(Atom::ann)
            .collect();
        crate::ann::join(texts.iter().map(String::as_str))
    }

    /// The title kept on this atom (the name a SMILES line carried), empty for none.
    pub fn title(&self) -> String {
        let text = self.more.iter().find(|m| m.0 == "title");
        let v: Option<Value> = text.and_then(|m| serde_json::from_str(&m.1).ok());
        v.as_ref()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    /// Replaces the title kept on this atom (`MAX_TITLE` characters at most, one line); the empty text takes the field away.
    pub(crate) fn set_title(&mut self, title: &str) {
        self.more.retain(|m| m.0 != "title");
        if !title.is_empty() {
            self.more
                .push(("title".into(), Value::from(title).to_string()));
            crate::m::sort(&mut self.more);
        }
    }

    /// Replaces the annotation kept on this atom (`text` is `ann::canon` text); the empty list takes the field away.
    pub(crate) fn set_ann(&mut self, text: &str) {
        self.more.retain(|m| m.0 != "ann");
        if text != "[]" {
            self.more.push(("ann".into(), text.to_string()));
            crate::m::sort(&mut self.more);
        }
    }
}

/// Wedge marks of a single bond, as drawn: the narrow end is the atom `a` of the bond.
pub const WEDGE_UP: u8 = 1;
pub const WEDGE_DOWN: u8 = 2;
pub const WEDGE_WAVY: u8 = 3;

/// One bond between the atoms with ids `a` and `b`, Kekulé order 1-3. `s` is the wedge mark of a single bond (0 none, 1 solid wedge,
/// 2 hashed wedge, 3 wavy), drawn from `a` (narrow end) to `b`: the marks are the drawing, the stereo of the record is derived from
/// them by the molfile rules of `mzlab-mol` at export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bond {
    pub id: u32,
    pub a: u32,
    pub b: u32,
    pub o: u8,
    pub s: u8,
    pub more: More,
}

impl Bond {
    /// The explicit colour (`ink`): `#rrggbb`, empty for automatic.
    pub fn col(&self) -> String {
        crate::ink::get(&self.more)
    }
}

/// What a drawing object other than a molecule is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjKind {
    /// A reaction arrow from (`x`, `y`) to (`x2`, `y2`), with an optional text above it.
    Arrow,
    /// A plus sign centred on (`x`, `y`).
    Plus,
    /// A line of text starting at (`x`, `y`).
    Text,
    /// An electron-pushing curved arrow (`curve.rs`): the ends are points (`x`, `y`, `x2`, `y2`) or anchors (`from`, `to`).
    Curve,
}

/// An object next to the molecules: a reaction arrow, a plus sign or a text. Coordinates in bond lengths, as the atoms.
#[derive(Clone, Debug, PartialEq)]
pub struct Obj {
    pub id: u32,
    pub t: ObjKind,
    pub x: f64,
    pub y: f64,
    pub x2: f64,
    pub y2: f64,
    /// The plain text; with `runs` it is the text of the runs (a line break as `\n`), kept in step by every operation.
    pub text: String,
    /// The formatted text (`runs.rs`); empty for a plain one-line `text`.
    pub runs: Vec<crate::runs::Run>,
    /// Arrows only: the id of one atom of the structure the arrow leaves, and of the one it reaches (the structure is the connected
    /// piece that holds the atom). `0` is a free end. An id that no atom carries any more is kept as written and used as a free end.
    pub from: u32,
    pub to: u32,
    pub more: More,
}

impl ObjKind {
    /// The field `t` as written.
    pub(crate) fn name(self) -> &'static str {
        match self {
            ObjKind::Arrow => "arrow",
            ObjKind::Plus => "plus",
            ObjKind::Text => "text",
            ObjKind::Curve => "curve",
        }
    }

    /// The field `t` of an object, read by hand.
    pub(crate) fn read(g: &Fields) -> Result<ObjKind, ()> {
        match g.s("t")?.as_deref() {
            Some("arrow") => Ok(ObjKind::Arrow),
            Some("plus") => Ok(ObjKind::Plus),
            Some("text") => Ok(ObjKind::Text),
            Some("curve") => Ok(ObjKind::Curve),
            _ => Err(()),
        }
    }
}

impl Obj {
    /// The explicit colour (`ink`): `#rrggbb`, empty for automatic.
    pub fn col(&self) -> String {
        crate::ink::get(&self.more)
    }

    /// An arrow has a length, a text has a text of one line, a plus has neither a second point nor a text.
    pub fn valid(&self) -> bool {
        let one_line = if self.runs.is_empty() {
            !self.text.contains(['\n', '\r']) && self.text.chars().count() <= 200
        } else {
            crate::runs::valid(&self.runs) && self.text == crate::runs::plain(&self.runs)
        };
        let no_end = self.x2 == 0.0 && self.y2 == 0.0;
        let anchors_ok =
            matches!(self.t, ObjKind::Arrow | ObjKind::Curve) || (self.from == 0 && self.to == 0);
        one_line
            && anchors_ok
            && match self.t {
                ObjKind::Arrow => (self.x2 - self.x).abs() + (self.y2 - self.y).abs() > 1e-3,
                ObjKind::Plus => no_end && self.text.is_empty(),
                ObjKind::Text => no_end && !self.text.trim().is_empty(),
                // (the length of a curve between two points is checked when the operation runs: removing the anchors of both ends
                // can leave the two points together, and the document stays readable)
                ObjKind::Curve => {
                    self.text.is_empty()
                        && self.curve().valid()
                        && crate::curve::curve_fields_ok(self)
                }
            }
    }
}

/// The document. `next` is the next free id (atoms, bonds and objects share one counter), so replaying a log gives the same ids.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub format: String,
    pub next: u32,
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    /// Reaction arrows, plus signs and texts.
    pub objects: Vec<Obj>,
    /// Fields of the document this version does not know (a `style`, a `meta`, annotations of a later version).
    pub more: More,
}

impl Default for Document {
    fn default() -> Self {
        Document {
            format: FORMAT.into(),
            next: 1,
            atoms: Vec::new(),
            bonds: Vec::new(),
            objects: Vec::new(),
            more: More::new(),
        }
    }
}

/// Coordinates are kept to 4 decimals so that the same edits give the same bytes everywhere.
pub fn round4(v: f64) -> f64 {
    let r = (v * 1e4).round() / 1e4;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

/// The modes of the document field `labels` (the words of `pathway.label`).
pub const LABEL_MODES: [&str; 4] = ["delta", "mass", "delta+mass", "name+delta"];

/// The canonical `labels` choice: `mode` one of `LABEL_MODES` (empty: no choice, and then no adduct), `adduct` empty or a name of
/// `values::ADDUCTS`, only with `mass` or `delta+mass`. `None` for anything else: nothing falls back silently.
pub(crate) fn labels_canon(mode: &str, adduct: &str) -> Option<(String, String)> {
    let (mode, adduct) = (mode.trim(), adduct.trim());
    if mode.is_empty() {
        return adduct.is_empty().then(|| (String::new(), String::new()));
    }
    if !LABEL_MODES.contains(&mode) {
        return None;
    }
    if !adduct.is_empty()
        && !(matches!(mode, "mass" | "delta+mass") && crate::values::ADDUCTS.contains(&adduct))
    {
        return None;
    }
    Some((mode.into(), adduct.into()))
}

impl Document {
    /// The title the file of this drawing is written with: the title of the first atom (pool order) that has one, empty for none.
    pub fn title(&self) -> String {
        (self.atoms.iter().map(Atom::title))
            .find(|t| !t.is_empty())
            .unwrap_or_default()
    }

    /// The mass-label choice of this scheme (`mode`, `adduct`), empty strings for none. A field `labels` of another shape, or a
    /// choice `labels_canon` refuses, is no choice (and is left alone and written back).
    pub fn labels(&self) -> (String, String) {
        let v: Option<Value> = (self.more.iter().find(|m| m.0 == "labels"))
            .and_then(|m| serde_json::from_str(&m.1).ok());
        let get = |k: &str| {
            (v.as_ref().and_then(|v| v.get(k)))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        labels_canon(&get("mode"), &get("adduct")).unwrap_or_default()
    }

    /// Sets the choice (`labels_canon` text); the empty mode takes it away. The keys of `labels` this version does not know are kept.
    pub(crate) fn set_labels(&mut self, mode: &str, adduct: &str) {
        let mut obj = (self.more.iter().find(|m| m.0 == "labels"))
            .and_then(|m| serde_json::from_str::<Value>(&m.1).ok())
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        obj.remove("mode");
        obj.remove("adduct");
        if !mode.is_empty() {
            obj.insert("mode".into(), mode.into());
        }
        if !adduct.is_empty() {
            obj.insert("adduct".into(), adduct.into());
        }
        self.more.retain(|m| m.0 != "labels");
        if !obj.is_empty() {
            self.more
                .push(("labels".into(), Value::Object(obj).to_string()));
            crate::m::sort(&mut self.more);
        }
    }

    /// Compact JSON with a fixed field order: same document, same bytes. Written by hand: the known fields in their order
    /// (optional ones only when set), then the unknown ones.
    pub fn to_json(&self) -> String {
        let list = |items: Vec<String>| format!("[{}]", items.join(","));
        let atoms = (self.atoms.iter())
            .map(|a| {
                let mut w = Writer("{".into());
                w.int("id", a.id as i64);
                w.text("el", &a.el);
                w.num("x", a.x);
                w.num("y", a.y);
                if a.chg != 0 {
                    w.int("chg", a.chg as i64);
                }
                if a.iso != 0 {
                    w.int("iso", a.iso as i64);
                }
                if a.rad != 0 {
                    w.int("rad", a.rad as i64);
                }
                if let Some(h) = a.h {
                    w.int("h", h as i64);
                }
                if !a.lab.is_empty() {
                    w.text("lab", &a.lab);
                }
                if a.map != 0 {
                    w.int("map", a.map as i64);
                }
                w.more(&a.more);
                w.0 + "}"
            })
            .collect();
        let bonds = (self.bonds.iter())
            .map(|b| {
                let mut w = Writer("{".into());
                w.int("id", b.id as i64);
                w.int("a", b.a as i64);
                w.int("b", b.b as i64);
                w.int("o", b.o as i64);
                if b.s != 0 {
                    w.int("s", b.s as i64);
                }
                w.more(&b.more);
                w.0 + "}"
            })
            .collect();
        let mut w = Writer("{".into());
        w.text("format", &self.format);
        w.int("next", self.next as i64);
        w.raw("atoms", &list(atoms));
        w.raw("bonds", &list(bonds));
        if !self.objects.is_empty() {
            let objects = (self.objects.iter())
                .map(|o| {
                    let mut w = Writer("{".into());
                    w.int("id", o.id as i64);
                    w.text("t", o.t.name());
                    w.num("x", o.x);
                    w.num("y", o.y);
                    if o.x2 != 0.0 {
                        w.num("x2", o.x2);
                    }
                    if o.y2 != 0.0 {
                        w.num("y2", o.y2);
                    }
                    if !o.text.is_empty() {
                        w.text("text", &o.text);
                    }
                    if !o.runs.is_empty() {
                        w.raw("runs", &crate::runs::to_json(&o.runs));
                    }
                    for (k, v) in [("from", o.from), ("to", o.to)] {
                        if v != 0 {
                            w.int(k, v as i64);
                        }
                    }
                    w.more(&o.more);
                    w.0 + "}"
                })
                .collect();
            w.raw("objects", &list(objects));
        }
        w.more(&self.more);
        w.0 + "}"
    }

    /// Reads and checks a document; the error is an i18n key. Read by hand from a JSON value, as the operations are: a derived
    /// reader costs wasm bytes per field. A missing optional field takes its default, an unknown field is ignored, a field of the
    /// wrong type makes the document unreadable.
    pub fn from_json(text: &str) -> Result<Document, &'static str> {
        let v: Value = serde_json::from_str(text).map_err(|_| "error.draw2.document")?;
        // the version rule, as for the settings: version 1 is read, a higher number is the file of a newer mzDraw
        match v
            .get("format")
            .and_then(Value::as_str)
            .map(crate::brand::document_version)
        {
            Some(Ok(1)) => {}
            Some(Ok(_)) => return Err("error.draw2.documentNewer"),
            _ => return Err("error.draw2.document"),
        }
        let mut d = Self::read(&v).map_err(|()| "error.draw2.document")?;
        if !d.consistent() {
            return Err("error.draw2.document");
        }
        // a document written under an earlier name (`brand::LEGACY_IDS`) is read and written again under the current id
        d.format = FORMAT.into();
        Ok(d)
    }

    fn read(v: &Value) -> Result<Document, ()> {
        fn list<T>(
            v: &Value,
            k: &str,
            need: bool,
            f: impl Fn(&Fields) -> Result<T, ()>,
        ) -> Result<Vec<T>, ()> {
            match v.get(k) {
                Some(Value::Array(a)) => a.iter().map(|x| f(&Fields(x))).collect(),
                None if !need => Ok(Vec::new()),
                _ => Err(()),
            }
        }
        let g = Fields(v);
        let num = |x: Option<f64>| x.ok_or(());
        Ok(Document {
            more: more_of(v, &["format", "next", "atoms", "bonds", "objects"]),
            format: g.s("format")?.ok_or(())?,
            next: g.u("next")?.ok_or(())?,
            atoms: list(v, "atoms", true, |a| {
                Ok(Atom {
                    id: a.u("id")?.ok_or(())?,
                    el: a.s("el")?.ok_or(())?,
                    x: num(a.f("x")?)?,
                    y: num(a.f("y")?)?,
                    chg: a.i8("chg")?.unwrap_or(0),
                    iso: a.u("iso")?.unwrap_or(0),
                    rad: a.u("rad")?.unwrap_or(0),
                    h: a.u("h")?,
                    lab: a.s("lab")?.unwrap_or_default(),
                    map: a.u("map")?.unwrap_or(0),
                    more: more_of(
                        a.0,
                        &["id", "el", "x", "y", "chg", "iso", "rad", "h", "lab", "map"],
                    ),
                })
            })?,
            bonds: list(v, "bonds", true, |b| {
                Ok(Bond {
                    id: b.u("id")?.ok_or(())?,
                    a: b.u("a")?.ok_or(())?,
                    b: b.u("b")?.ok_or(())?,
                    o: b.u("o")?.ok_or(())?,
                    s: b.u("s")?.unwrap_or(0),
                    more: more_of(b.0, &["id", "a", "b", "o", "s"]),
                })
            })?,
            objects: list(v, "objects", false, |o| {
                let runs = o.runs("runs")?.unwrap_or_default();
                Ok(Obj {
                    id: o.u("id")?.ok_or(())?,
                    t: ObjKind::read(o)?,
                    x: num(o.f("x")?)?,
                    y: num(o.f("y")?)?,
                    x2: o.f("x2")?.unwrap_or(0.0),
                    y2: o.f("y2")?.unwrap_or(0.0),
                    text: if runs.is_empty() {
                        o.s("text")?.unwrap_or_default()
                    } else {
                        crate::runs::plain(&runs)
                    },
                    runs,
                    from: o.u("from")?.unwrap_or(0),
                    to: o.u("to")?.unwrap_or(0),
                    more: more_of(
                        o.0,
                        &[
                            "id", "t", "x", "y", "x2", "y2", "text", "runs", "from", "to",
                        ],
                    ),
                })
            })?,
        })
    }

    /// Ids unique and below `next`, bonds between two different existing atoms, orders 1-3, one bond per pair, wedge marks on single
    /// bonds only.
    pub(crate) fn consistent(&self) -> bool {
        let mut ids: Vec<u32> = self.atoms.iter().map(|a| a.id).collect();
        ids.extend(self.bonds.iter().map(|b| b.id));
        ids.extend(self.objects.iter().map(|o| o.id));
        crate::m::sort(&mut ids);
        let unique = ids.windows(2).all(|w| w[0] != w[1]);
        let mut pairs: Vec<(u32, u32)> = self
            .bonds
            .iter()
            .map(|b| (b.a.min(b.b), b.a.max(b.b)))
            .collect();
        crate::m::sort(&mut pairs);
        let known: std::collections::HashSet<u32> = self.atoms.iter().map(|a| a.id).collect();
        unique
            && self.next <= MAX_ID
            && self.coords_ok()
            && self.objects.iter().all(Obj::valid)
            && self.atoms.iter().all(crate::curve::atom_fields_ok)
            && self
                .objects
                .iter()
                .all(|o| crate::curve::anchors_exist(self, o))
            && ids.iter().all(|&i| i < self.next)
            && pairs.windows(2).all(|w| w[0] != w[1])
            && self.atoms.iter().all(|a| {
                mzlab_mol::is_element(&a.el)
                    && a.map <= MAX_MAP
                    && (a.lab.is_empty() || crate::label::kind(&a.lab).is_some())
            })
            && self.bonds.iter().all(|b| {
                b.a != b.b
                    && (1..=3).contains(&b.o)
                    && (b.s == 0 || (b.s <= 3 && b.o == 1))
                    && known.contains(&b.a)
                    && known.contains(&b.b)
            })
    }

    pub fn atom_index(&self, id: u32) -> Option<usize> {
        self.atoms.iter().position(|a| a.id == id)
    }

    pub fn atom(&self, id: u32) -> Option<&Atom> {
        self.atoms.iter().find(|a| a.id == id)
    }

    /// Every coordinate finite and within `MAX_COORD`.
    pub(crate) fn coords_ok(&self) -> bool {
        self.atoms.iter().all(|a| in_range(a.x) && in_range(a.y))
            && (self.objects.iter()).all(|o| [o.x, o.y, o.x2, o.y2].into_iter().all(in_range))
    }

    pub fn object(&self, id: u32) -> Option<&Obj> {
        self.objects.iter().find(|o| o.id == id)
    }

    /// Whether nothing is drawn.
    pub fn is_empty(&self) -> bool {
        self.atoms.is_empty() && self.objects.is_empty()
    }

    /// The box around everything drawn (the anchor points of the objects), as (x0, y0, x1, y1); `None` for an empty document.
    pub fn bounds(&self) -> Option<(f64, f64, f64, f64)> {
        let mut pts: Vec<(f64, f64)> = self.atoms.iter().map(|a| (a.x, a.y)).collect();
        for o in &self.objects {
            pts.push((o.x, o.y));
            if o.t == ObjKind::Arrow {
                pts.push((o.x2, o.y2));
            }
        }
        let first = *pts.first()?;
        Some(
            pts.iter()
                .fold((first.0, first.1, first.0, first.1), |b, p| {
                    (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1))
                }),
        )
    }

    pub fn bond_index(&self, id: u32) -> Option<usize> {
        self.bonds.iter().position(|b| b.id == id)
    }

    pub fn bond(&self, id: u32) -> Option<&Bond> {
        self.bonds.iter().find(|b| b.id == id)
    }

    /// The bond between two atoms, in either direction.
    pub fn bond_between(&self, a: u32, b: u32) -> Option<&Bond> {
        self.bonds
            .iter()
            .find(|x| (x.a == a && x.b == b) || (x.a == b && x.b == a))
    }

    /// Ids of the atoms bonded to `id`, in bond order.
    pub fn neighbours(&self, id: u32) -> Vec<u32> {
        self.bonds
            .iter()
            .filter_map(|b| match (b.a == id, b.b == id) {
                (true, _) => Some(b.b),
                (_, true) => Some(b.a),
                _ => None,
            })
            .collect()
    }

    /// Sum of the bond orders on an atom (a dative or hydrogen bond adds nothing: it takes no part in the valence).
    pub fn order_sum(&self, id: u32) -> u32 {
        self.bonds
            .iter()
            .filter(|b| (b.a == id || b.b == id) && !b.is_link())
            .map(|b| b.o as u32)
            .sum()
    }

    /// Hydrogens the atom carries: the explicit count, or what the lowest allowed valence leaves free; none on a labelled atom
    /// (the label says what is there).
    pub fn hydrogens(&self, a: &Atom) -> u32 {
        if !a.lab.is_empty() {
            return 0;
        }
        if let Some(h) = a.h {
            return h as u32;
        }
        Self::free_hydrogens(a, self.order_sum(a.id))
    }

    /// What the lowest allowed valence leaves free on an atom whose bonds add up to `orders`.
    fn free_hydrogens(a: &Atom, orders: u32) -> u32 {
        let used = orders + a.rad as u32;
        mzlab_mol::allowed_valences(&a.el, a.chg as i32)
            .into_iter()
            .find(|&v| v >= used)
            .map_or(0, |v| v - used)
    }

    /// `hydrogens` of every atom in pool order, in one pass over the bonds (the per-atom form scans all bonds each time).
    pub(crate) fn all_hydrogens(&self) -> Vec<u32> {
        let sums = self.order_sums();
        (self.atoms.iter())
            .map(|a| match (a.lab.is_empty(), a.h) {
                (false, _) => 0,
                (true, Some(h)) => h as u32,
                (true, None) => Self::free_hydrogens(a, sums.get(&a.id).copied().unwrap_or(0)),
            })
            .collect()
    }

    /// The sum of the orders of the bonds at each atom id (an atom without a bond is absent).
    pub(crate) fn order_sums(&self) -> std::collections::HashMap<u32, u32> {
        let mut sums = std::collections::HashMap::with_capacity(self.atoms.len());
        // a dative or hydrogen bond takes no part in the valence of its ends
        for b in self.bonds.iter().filter(|b| !b.is_link()) {
            *sums.entry(b.a).or_insert(0) += b.o as u32;
            if b.b != b.a {
                *sums.entry(b.b).or_insert(0) += b.o as u32;
            }
        }
        sums
    }

    /// The pool index of every atom id (the first one for an id that is there twice).
    pub(crate) fn index_map(&self) -> std::collections::HashMap<u32, usize> {
        let mut map = std::collections::HashMap::with_capacity(self.atoms.len());
        for (i, a) in self.atoms.iter().enumerate().rev() {
            map.insert(a.id, i);
        }
        map
    }
}
