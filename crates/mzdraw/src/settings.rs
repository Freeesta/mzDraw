//! The settings file `mzdraw-settings/1`: the drawing style (bond length, line widths, font, colours) with its built-in presets.
//! Units are points, so a preset prints at its true size; the screen scales the same numbers by the zoom. The file is read by
//! hand from a JSON value (as the document is); every field is optional and defaults to the preset `mzlab`.

use crate::ops::Fields;
use serde_json::{json, Value};

/// The value of the `format` field of a settings file.
pub const FORMAT: &str = "mzdraw-settings/1";

/// The drawing style, in points (1/72 inch). `bond_spacing` is a fraction of the bond length.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    /// The preset this style is, or was made from; empty for a file that names none.
    pub name: String,
    pub bond_length: f64,
    pub line_width: f64,
    /// Width of the wide end of a wedge.
    pub bold_width: f64,
    /// Distance between the rungs of a hashed wedge.
    pub hash_spacing: f64,
    /// Distance between the lines of a double bond, as a fraction of the bond length.
    pub bond_spacing: f64,
    /// Gap between a label and the bonds that end at it.
    pub margin: f64,
    pub font_size: f64,
    /// Size of subscripts and superscripts relative to the font.
    pub subscript_scale: f64,
    /// One ink for every element instead of the element colours.
    pub mono: bool,
    /// What the settings file held that this version does not know, kept to be written back: the unknown keys of the file and of
    /// its `style` object. A file edited by a newer mzDraw survives a round through this one.
    pub unknown: Unknown,
}

/// The keys of a settings file that this version does not know (`file`: next to `format` and `style`; `style`: inside `style`),
/// each with its value as JSON text, kept as text because it is only ever written back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Unknown {
    pub file: Vec<(String, String)>,
    pub style: Vec<(String, String)>,
}

/// A JSON object from (key, value as JSON text) pairs, keys in alphabetical order.
fn object(mut pairs: Vec<(String, String)>) -> String {
    pairs.sort();
    let body: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!("{}:{v}", Value::from(k.as_str())))
        .collect();
    format!("{{{}}}", body.join(","))
}

/// The keys of the `style` object this version reads and writes.
const STYLE_KEYS: [&str; 10] = [
    "preset",
    "bond_length",
    "line_width",
    "bold_width",
    "hash_spacing",
    "bond_spacing",
    "margin",
    "font_size",
    "subscript_scale",
    "colour",
];

/// The version of a format name `<name>/<n>`: `Ok(n)` when the name is `name`, `Err(())` otherwise.
pub(crate) fn version(format: &str, name: &str) -> Result<u32, ()> {
    let n = format
        .strip_prefix(name)
        .and_then(|r| r.strip_prefix('/'))
        .ok_or(())?;
    let digits = !n.is_empty() && !n.starts_with('0') && n.bytes().all(|b| b.is_ascii_digit());
    n.parse().ok().filter(|_| digits).ok_or(())
}

/// The names of the built-in presets: `mzlab` (the default look), `acs-1996` (ACS "Document 1996": bond 14.4 pt, line 0.6 pt,
/// bond spacing 18 %, font 10 pt; bold 2 pt, hash 2.5 pt and margin 1.6 pt as in the format study, not confirmed on a primary
/// source), `slides` (1.5 times `acs-1996` with heavier lines, ours).
pub const PRESETS: [&str; 3] = ["mzlab", "acs-1996", "slides"];

impl Default for Style {
    fn default() -> Self {
        Style {
            name: "mzlab".into(),
            bond_length: 30.0,
            line_width: 1.35,
            bold_width: 6.0,
            hash_spacing: 3.6,
            bond_spacing: 0.13,
            margin: 2.04,
            font_size: 12.0,
            subscript_scale: 0.7,
            mono: false,
            unknown: Unknown::default(),
        }
    }
}

/// The style scaled to an output: every length in output pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub line: f64,
    /// Distance between the lines of a double bond.
    pub spacing: f64,
    /// Half the width of the wide end of a wedge.
    pub wedge: f64,
    pub hash: f64,
    /// Distance from the centre of a labelled atom to where its bonds stop.
    pub gap: f64,
    pub font: f64,
    pub small: f64,
    pub mono: bool,
}

impl Style {
    /// A built-in preset by name.
    pub fn preset(name: &str) -> Option<Style> {
        let acs = Style {
            name: "acs-1996".into(),
            bond_length: 14.4,
            line_width: 0.6,
            bold_width: 2.0,
            hash_spacing: 2.5,
            bond_spacing: 0.18,
            margin: 1.6,
            font_size: 10.0,
            subscript_scale: 0.7,
            mono: true,
            unknown: Unknown::default(),
        };
        match name {
            "mzlab" => Some(Style::default()),
            "acs-1996" => Some(acs),
            "slides" => Some(Style {
                name: "slides".into(),
                bond_length: 21.6,
                line_width: 1.5,
                bold_width: 4.0,
                hash_spacing: 4.0,
                margin: 2.5,
                font_size: 16.0,
                ..acs
            }),
            _ => None,
        }
    }

    /// Whether every number is one a drawing can be made with.
    fn valid(&self) -> bool {
        let within = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
        within(self.bond_length, 4.0, 200.0)
            && within(self.line_width, 0.1, 20.0)
            && within(self.bold_width, 0.1, 40.0)
            && within(self.hash_spacing, 0.5, 40.0)
            && within(self.bond_spacing, 0.02, 0.5)
            && within(self.margin, 0.0, 20.0)
            && within(self.font_size, 3.0, 100.0)
            && within(self.subscript_scale, 0.3, 1.0)
    }

    /// A style from a preset name or from the text of a settings file (`{"format": "mzdraw-settings/1", "style": {...}}`; the
    /// field `preset` of `style` names the preset the other fields start from, default `mzlab`). The error is an i18n key.
    pub fn from_text(text: &str) -> Result<Style, &'static str> {
        let t = text.trim();
        if !t.starts_with('{') {
            return Style::preset(t).ok_or("error.draw2.style");
        }
        let v: Value = serde_json::from_str(t).map_err(|_| "error.draw2.style")?;
        // the version rule: this version reads version 1; a higher number is a file of a newer mzDraw and is said so, never
        // half read; within a version unknown keys are kept and written back
        match v
            .get("format")
            .and_then(Value::as_str)
            .map(|f| version(f, "mzdraw-settings"))
        {
            Some(Ok(1)) => Self::read(&v).ok_or("error.draw2.style"),
            Some(Ok(_)) => Err("error.draw2.styleNewer"),
            _ => Err("error.draw2.style"),
        }
    }

    fn read(v: &Value) -> Option<Style> {
        let file = v.as_object()?;
        let none = json!({});
        let style = v.get("style").unwrap_or(&none);
        let unknown = Unknown {
            file: (file.iter())
                .filter(|(k, _)| k.as_str() != "format" && k.as_str() != "style")
                .map(|(k, x)| (k.clone(), x.to_string()))
                .collect(),
            style: (style.as_object()?.iter())
                .filter(|(k, _)| !STYLE_KEYS.contains(&k.as_str()))
                .map(|(k, x)| (k.clone(), x.to_string()))
                .collect(),
        };
        let g = Fields(style);
        let base = match g.s("preset").ok()? {
            Some(name) => Style::preset(&name)?,
            None => Style::default(),
        };
        let num = |k: &str, d: f64| g.f(k).ok().map(|x| x.unwrap_or(d));
        let colour = match g.s("colour").ok()?.as_deref() {
            None => base.mono,
            Some("mono") => true,
            Some("element") => false,
            Some(_) => return None,
        };
        let s = Style {
            bond_length: num("bond_length", base.bond_length)?,
            line_width: num("line_width", base.line_width)?,
            bold_width: num("bold_width", base.bold_width)?,
            hash_spacing: num("hash_spacing", base.hash_spacing)?,
            bond_spacing: num("bond_spacing", base.bond_spacing)?,
            margin: num("margin", base.margin)?,
            font_size: num("font_size", base.font_size)?,
            subscript_scale: num("subscript_scale", base.subscript_scale)?,
            mono: colour,
            name: base.name,
            unknown,
        };
        s.valid().then_some(s)
    }

    /// The style as a settings file: fixed key order (alphabetical), every known field written, the unknown keys it was read
    /// with written back unchanged (a known key always wins over an unknown one of the same name), so the same style gives the
    /// same bytes.
    pub fn to_json(&self) -> String {
        let text = |v: &str| Value::from(v).to_string();
        let num = |v: f64| Value::from(v).to_string();
        let mut style = self.unknown.style.clone();
        for (k, v) in [
            ("preset", text(&self.name)),
            ("bond_length", num(self.bond_length)),
            ("line_width", num(self.line_width)),
            ("bold_width", num(self.bold_width)),
            ("hash_spacing", num(self.hash_spacing)),
            ("bond_spacing", num(self.bond_spacing)),
            ("margin", num(self.margin)),
            ("font_size", num(self.font_size)),
            ("subscript_scale", num(self.subscript_scale)),
            ("colour", text(if self.mono { "mono" } else { "element" })),
        ] {
            style.push((k.to_string(), v));
        }
        let mut file = self.unknown.file.clone();
        file.push(("format".to_string(), text(FORMAT)));
        file.push(("style".to_string(), object(style)));
        object(file)
    }

    /// The style at `scale` output pixels per bond length. On the screen (`screen`) lines never go below one pixel and the
    /// font stays between 9 and 40 pixels, whatever the zoom; a file takes the numbers as they are.
    pub fn metrics(&self, scale: f64, screen: bool) -> Metrics {
        let k = scale / self.bond_length;
        let (mut line, mut font) = (self.line_width * k, self.font_size * k);
        if screen {
            line = line.max(1.0);
            font = font.clamp(9.0, 40.0);
        }
        Metrics {
            line,
            spacing: self.bond_spacing * scale,
            wedge: self.bold_width * k / 2.0,
            hash: self.hash_spacing * k,
            gap: font * 0.45 + self.margin * k,
            font,
            small: font * self.subscript_scale,
            mono: self.mono,
        }
    }
}
