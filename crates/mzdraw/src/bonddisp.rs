//! How a bond is drawn beyond its order: the side of the second line of a double bond (`pos`) and the display of the line (`k`).
//!
//! Both live in the unknown-field list (`doc::More`) of the bond, as the explicit colour does (`ink`), so a document without them
//! reads and writes as before (same document, same bytes) and an older reader keeps them. The operation `bond.set` sets and clears
//! them (`ops::Op::BondSet`).
//!
//! `pos`, seen from the atom `a` to the atom `b` of the bond (the page paints with y down; "left" is the left hand of someone
//! walking from `a` to `b` on the screen):
//! - absent (`auto`): inside a ring the second line goes to the side of the ring; a double bond with a free end (C=O, =CH2) is
//!   centred; in a chain it goes to the side with more substituents (the sum over both ends of the neighbours on each side).
//! - `c` centred (two lines astride the axis), `l` left, `r` right. Only a double bond has a `pos`; a new order other than 2
//!   takes it away.
//!
//! `k` is the one field of the display of the line (COO decision 16): `dash` (a dashed line), `bdash` (a bold dashed line), `hash`
//! (plain hashed: short ticks across the line, not a stereo mark), `bold` (a heavy line), `dative` (an arrow from the donor `a` to
//! the acceptor `b`) and `hbond` (a dotted line, a hydrogen bond). `dative` and `hbond` are bonds of order 1 only; a new order
//! other than 1 takes them away, and neither counts for the implicit hydrogens of its ends. A bond with a wedge mark has no `k`
//! (the mark is its display). A value this version does not know is kept as it is and drawn as a plain line.

use crate::doc::More;

/// The values of `pos` that are written (`auto` is the absence of the field).
pub const POS: [&str; 3] = ["c", "l", "r"];
/// The values of `k` of this slice.
pub const KINDS: [&str; 6] = ["dash", "bdash", "hash", "bold", "dative", "hbond"];

/// Whether the kind is a bond of its own (order 1 only, and it does not count for the valence of its ends): `dative`, `hbond`.
pub fn single_only(k: &str) -> bool {
    matches!(k, "dative" | "hbond")
}

fn get(more: &More, key: &str) -> String {
    more.iter()
        .find(|m| m.0 == key)
        .and_then(|m| serde_json::from_str::<String>(&m.1).ok())
        .unwrap_or_default()
}

fn put(more: &mut More, key: &str, value: &str) {
    more.retain(|m| m.0 != key);
    if !value.is_empty() {
        more.push((key.into(), serde_json::Value::from(value).to_string()));
        crate::m::sort(more);
    }
}

/// The side of the second line of a double bond kept in the fields: `c`, `l` or `r`, empty for auto.
pub(crate) fn pos_get(more: &More) -> String {
    get(more, "pos")
}

/// Sets the side (`c`, `l`, `r`); empty text (auto) takes the field away.
pub(crate) fn pos_put(more: &mut More, v: &str) {
    put(more, "pos", v)
}

/// The display kind kept in the fields, empty for a plain line.
pub(crate) fn k_get(more: &More) -> String {
    get(more, "k")
}

/// Sets the display kind; empty text takes the field away.
pub(crate) fn k_put(more: &mut More, v: &str) {
    put(more, "k", v)
}

/// The canonical `pos` of an operation: `auto` (or empty) gives the empty text; `None` if the value is not one of the sides.
pub fn pos_canon(v: &str) -> Option<String> {
    match v {
        "" | "auto" => Some(String::new()),
        _ if POS.contains(&v) => Some(v.to_string()),
        _ => None,
    }
}

/// The canonical `k` of an operation: empty or `solid` gives the empty text; `None` if the value is not a kind of this version.
pub fn k_canon(v: &str) -> Option<String> {
    match v {
        "" | "solid" => Some(String::new()),
        _ if KINDS.contains(&v) => Some(v.to_string()),
        _ => None,
    }
}

impl crate::doc::Bond {
    /// Whether the bond is a dative or a hydrogen bond: it takes no part in the valence of its ends.
    pub fn is_link(&self) -> bool {
        single_only(&self.kind())
    }
}

impl crate::doc::Bond {
    /// `c`, `l` or `r`: where the second line of a double bond goes; empty for auto.
    pub fn pos(&self) -> String {
        pos_get(&self.more)
    }
    /// The display of the line (one of `KINDS`); empty for a plain line.
    pub fn kind(&self) -> String {
        k_get(&self.more)
    }
}
