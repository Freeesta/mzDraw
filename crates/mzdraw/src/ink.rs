//! The text of a colour. One parser for every field that holds a colour (`#rrggbb`, lower case).

/// The canonical text of a colour: `#rgb` or `#rrggbb` in any case gives `#rrggbb` in lower case; the empty text (automatic) stays
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
