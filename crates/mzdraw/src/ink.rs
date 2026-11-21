//! The explicit colour of an atom or a bond: the field `col` of the document (`"#rrggbb"`, lower case; absent means automatic: the
//! element colour, or the ink in the mono style). It is kept in the unknown-field list (`doc::More`) of the atom or the bond, so a
//! document without it reads and writes as before and an older reader keeps it. The operation `colour` (`ops::Op::Colour`) sets
//! and clears it.
//!
//! In the display list an explicit colour is a negative colour index: `-(rgb + 1)` (exact in an `f32`, 24 bits), so that no
//! surface needs a table to paint it; an index of 0 or more is a token of the surface, as before.

use crate::doc::More;

/// The palette of the colour picker: the key of the name (i18n `draw2.colour.<key>`), the English name, the colour. Dark enough on
/// white paper to read as text (contrast 3:1 or more for each, 4.5:1 for most); the picker also shows the name as text.
pub const PALETTE: [(&str, &str, &str); 11] = [
    ("black", "Black", "#000000"),
    ("red", "Red", "#d00000"),
    ("orange", "Orange", "#d86000"),
    ("yellow", "Yellow", "#a88000"),
    ("green", "Green", "#1a7f1a"),
    ("teal", "Teal", "#007a7a"),
    ("blue", "Blue", "#1f3fd0"),
    ("purple", "Purple", "#7b2fbe"),
    ("magenta", "Magenta", "#c0189c"),
    ("brown", "Brown", "#8b5a2b"),
    ("grey", "Grey", "#6b6b6b"),
];

/// The canonical text of a colour; the empty text is `Some("")`, which means automatic (no explicit colour), not an error: `#rgb` or `#rrggbb` in any case gives `#rrggbb` in lower case; the empty text (automatic) stays
/// empty; anything else is `None`.
pub fn canon(text: &str) -> Option<String> {
    let t = text.trim();
    if t.is_empty() {
        return Some(String::new());
    }
    let h = t.strip_prefix('#')?;
    if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    match h.len() {
        6 => Some(format!("#{}", h.to_ascii_lowercase())),
        3 => {
            let mut s = String::from("#");
            for c in h.chars() {
                s.push(c.to_ascii_lowercase());
                s.push(c.to_ascii_lowercase());
            }
            Some(s)
        }
        _ => None,
    }
}

/// The value of a canonical colour (`#rrggbb`) as 24 bits.
fn bits(hex: &str) -> Option<u32> {
    u32::from_str_radix(hex.strip_prefix('#')?, 16)
        .ok()
        .filter(|_| hex.len() == 7)
}

/// The colour index of the display list for a canonical colour.
pub fn index(hex: &str) -> Option<f32> {
    bits(hex).map(|v| -((v + 1) as f32))
}

/// The canonical colour of an index of the display list, `None` for a token of the surface (0 or more).
pub fn hex_of(index: f32) -> Option<String> {
    if !(-16_777_216.0..0.0).contains(&index) {
        return None;
    }
    Some(format!("#{:06x}", (-index) as u32 - 1))
}

/// The name of a colour of the palette (its key), `None` for a custom colour.
pub fn name(hex: &str) -> Option<&'static str> {
    PALETTE.iter().find(|p| p.2 == hex).map(|p| p.0)
}

/// The colour kept in the fields of an atom or a bond: the canonical text, empty for automatic (also when the field is not a
/// colour: it is left alone).
pub(crate) fn get(more: &More) -> String {
    more.iter()
        .find(|m| m.0 == "col")
        .and_then(|m| serde_json::from_str::<String>(&m.1).ok())
        .and_then(|s| canon(&s))
        .unwrap_or_default()
}

/// Replaces the colour kept in the fields (`hex` is canonical); the empty text takes the field away.
pub(crate) fn put(more: &mut More, hex: &str) {
    more.retain(|m| m.0 != "col");
    if !hex.is_empty() {
        more.push(("col".into(), serde_json::Value::from(hex).to_string()));
        crate::m::sort(more);
    }
}

/// The formats of `Editor::save` that have no place for an explicit colour: they are written without it and the loss is said
/// (`Editor::lost_on_save`, the page's note, the replay's `lost`). CDXML, SVG, PNG, PDF and the document carry it.
pub const LOST_IN: [&str; 8] = [
    "molfile",
    "molfile-v3000",
    "smiles",
    "sdf",
    "ket",
    "cml",
    "rxn",
    "trasf",
];

impl crate::doc::Document {
    /// How many atoms, bonds and objects have an explicit colour that the output `kind` of `Editor::save` cannot hold: all of them
    /// for the formats of `LOST_IN`; only the objects for CDXML (it carries the colour of atoms and bonds, not of the objects).
    pub fn colours_lost(&self, kind: &str) -> usize {
        let objects = self.objects.iter().filter(|o| !o.col().is_empty()).count();
        if kind == "cdxml" {
            return objects;
        }
        if !LOST_IN.contains(&kind) {
            return 0;
        }
        self.atoms.iter().filter(|a| !a.col().is_empty()).count()
            + self.bonds.iter().filter(|b| !b.col().is_empty()).count()
            + objects
    }
}
