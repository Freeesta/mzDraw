//! The document `mzdraw/1`: atoms and bonds in flat pools with stable ids and 2D coordinates in bond lengths (y up, as in a molfile).
//! A structure is a connected piece; nothing chemical is derived here except the implicit hydrogens a label shows.

use crate::ops::Fields;
use serde::Serialize;
use serde_json::Value;

/// The value of the `format` field.
pub const FORMAT: &str = "mzdraw/1";

/// Ids stay below this, so that id arithmetic (the next id, the ids of a pasted copy) can never overflow.
pub const MAX_ID: u32 = 1 << 31;

/// Coordinates stay within this many bond lengths of the origin, and finite: a number that cannot be written as JSON never
/// enters a document.
pub const MAX_COORD: f64 = 1e6;

fn in_range(v: f64) -> bool {
    v.is_finite() && v.abs() <= MAX_COORD
}

fn is_zero<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// One atom. `h` is set only when the hydrogens were given explicitly (a file said so); otherwise they follow the valence rules.
/// A labelled atom (`lab`) shows its label alone: no hydrogens, charge or isotope of its own.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Atom {
    pub id: u32,
    pub el: String,
    pub x: f64,
    pub y: f64,
    #[serde(skip_serializing_if = "is_zero")]
    pub chg: i8,
    #[serde(skip_serializing_if = "is_zero")]
    pub iso: u16,
    #[serde(skip_serializing_if = "is_zero")]
    pub rad: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<u8>,
    /// A label drawn instead of the element: an abbreviation of `label::ABBREVIATIONS` (expanded to its atoms at export) or an
    /// R-group placeholder. Empty for a plain atom.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lab: String,
}

/// Wedge marks of a single bond, as drawn: the narrow end is the atom `a` of the bond.
pub const WEDGE_UP: u8 = 1;
pub const WEDGE_DOWN: u8 = 2;
pub const WEDGE_WAVY: u8 = 3;

/// One bond between the atoms with ids `a` and `b`, Kekulé order 1-3. `s` is the wedge mark of a single bond (0 none, 1 solid wedge,
/// 2 hashed wedge, 3 wavy), drawn from `a` (narrow end) to `b`: the marks are the drawing, the stereo of the record is derived from
/// them by the molfile rules of `mzlab-mol` at export.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Bond {
    pub id: u32,
    pub a: u32,
    pub b: u32,
    pub o: u8,
    #[serde(skip_serializing_if = "is_zero")]
    pub s: u8,
}

/// What a drawing object other than a molecule is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjKind {
    /// A reaction arrow from (`x`, `y`) to (`x2`, `y2`), with an optional text above it.
    Arrow,
    /// A plus sign centred on (`x`, `y`).
    Plus,
    /// A line of text starting at (`x`, `y`).
    Text,
}

/// An object next to the molecules: a reaction arrow, a plus sign or a text. Coordinates in bond lengths, as the atoms.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Obj {
    pub id: u32,
    pub t: ObjKind,
    pub x: f64,
    pub y: f64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x2: f64,
    #[serde(skip_serializing_if = "is_zero")]
    pub y2: f64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
}

impl ObjKind {
    /// The field `t` of an object, read by hand.
    pub(crate) fn read(g: &Fields) -> Result<ObjKind, ()> {
        match g.s("t")?.as_deref() {
            Some("arrow") => Ok(ObjKind::Arrow),
            Some("plus") => Ok(ObjKind::Plus),
            Some("text") => Ok(ObjKind::Text),
            _ => Err(()),
        }
    }
}

impl Obj {
    /// An arrow has a length, a text has a text of one line, a plus has neither a second point nor a text.
    pub fn valid(&self) -> bool {
        let one_line = !self.text.contains(['\n', '\r']) && self.text.chars().count() <= 200;
        let no_end = self.x2 == 0.0 && self.y2 == 0.0;
        one_line
            && match self.t {
                ObjKind::Arrow => (self.x2 - self.x).abs() + (self.y2 - self.y).abs() > 1e-3,
                ObjKind::Plus => no_end && self.text.is_empty(),
                ObjKind::Text => no_end && !self.text.trim().is_empty(),
            }
    }
}

/// The document. `next` is the next free id (atoms, bonds and objects share one counter), so replaying a log gives the same ids.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Document {
    pub format: String,
    pub next: u32,
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    /// Reaction arrows, plus signs and texts.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<Obj>,
}

impl Default for Document {
    fn default() -> Self {
        Document {
            format: FORMAT.into(),
            next: 1,
            atoms: Vec::new(),
            bonds: Vec::new(),
            objects: Vec::new(),
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

impl Document {
    /// Compact JSON with a fixed field order: same document, same bytes.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Reads and checks a document; the error is an i18n key. Read by hand from a JSON value, as the operations are: a derived
    /// reader costs wasm bytes per field. A missing optional field takes its default, an unknown field is ignored, a field of the
    /// wrong type makes the document unreadable.
    pub fn from_json(text: &str) -> Result<Document, &'static str> {
        let v: Value = serde_json::from_str(text).map_err(|_| "error.draw2.document")?;
        let d = Self::read(&v).map_err(|()| "error.draw2.document")?;
        if d.format != FORMAT || !d.consistent() {
            return Err("error.draw2.document");
        }
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
                })
            })?,
            bonds: list(v, "bonds", true, |b| {
                Ok(Bond {
                    id: b.u("id")?.ok_or(())?,
                    a: b.u("a")?.ok_or(())?,
                    b: b.u("b")?.ok_or(())?,
                    o: b.u("o")?.ok_or(())?,
                    s: b.u("s")?.unwrap_or(0),
                })
            })?,
            objects: list(v, "objects", false, |o| {
                Ok(Obj {
                    id: o.u("id")?.ok_or(())?,
                    t: ObjKind::read(o)?,
                    x: num(o.f("x")?)?,
                    y: num(o.f("y")?)?,
                    x2: o.f("x2")?.unwrap_or(0.0),
                    y2: o.f("y2")?.unwrap_or(0.0),
                    text: o.s("text")?.unwrap_or_default(),
                })
            })?,
        })
    }

    /// Ids unique and below `next`, bonds between two different existing atoms, orders 1-3, one bond per pair, wedge marks on single
    /// bonds only.
    fn consistent(&self) -> bool {
        let mut ids: Vec<u32> = self.atoms.iter().map(|a| a.id).collect();
        ids.extend(self.bonds.iter().map(|b| b.id));
        ids.extend(self.objects.iter().map(|o| o.id));
        ids.sort_unstable();
        let unique = ids.windows(2).all(|w| w[0] != w[1]);
        let mut pairs: Vec<(u32, u32)> = self
            .bonds
            .iter()
            .map(|b| (b.a.min(b.b), b.a.max(b.b)))
            .collect();
        pairs.sort_unstable();
        unique
            && self.next <= MAX_ID
            && self.coords_ok()
            && self.objects.iter().all(Obj::valid)
            && ids.iter().all(|&i| i < self.next)
            && pairs.windows(2).all(|w| w[0] != w[1])
            && self.atoms.iter().all(|a| {
                mzlab_mol::is_element(&a.el)
                    && (a.lab.is_empty() || crate::label::kind(&a.lab).is_some())
            })
            && self.bonds.iter().all(|b| {
                b.a != b.b
                    && (1..=3).contains(&b.o)
                    && (b.s == 0 || (b.s <= 3 && b.o == 1))
                    && self.atom(b.a).is_some()
                    && self.atom(b.b).is_some()
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

    /// Sum of the bond orders on an atom.
    pub fn order_sum(&self, id: u32) -> u32 {
        self.bonds
            .iter()
            .filter(|b| b.a == id || b.b == id)
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
        let used = self.order_sum(a.id) + a.rad as u32;
        mzlab_mol::allowed_valences(&a.el, a.chg as i32)
            .into_iter()
            .find(|&v| v >= used)
            .map_or(0, |v| v - used)
    }
}
