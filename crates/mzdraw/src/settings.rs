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
    /// Gap between a routed arrow (`route`) and the box of the structure it leaves or reaches, in bond lengths.
    pub pathway_margin: f64,
    /// One ink for every element instead of the element colours.
    pub mono: bool,
    /// Whether atom-to-atom map numbers are drawn at the atoms (`display.maps`, `on` by default).
    pub maps: bool,
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
    crate::m::sort(&mut pairs);
    let body: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!("{}:{v}", Value::from(k.as_str())))
        .collect();
    format!("{{{}}}", body.join(","))
}

/// One number of the style, as the schema says it: the one place that names the field, its unit and its bounds. The reader,
/// the writer, the validation, `Style::set` (the `--set style.<key>=<value>` of the CLI) and the fields of the page all go
/// through this table, in this order.
pub struct Field {
    pub key: &'static str,
    /// `pt` (points, 1/72 inch), `of_bond` (a fraction of the bond length) or `of_font` (a fraction of the font size).
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    get: fn(&Style) -> f64,
    put: fn(&mut Style, f64),
}

impl Field {
    /// The value of the field in a style.
    pub fn of(&self, s: &Style) -> f64 {
        (self.get)(s)
    }
}

/// The numbers of the style. Next to them the `style` object holds `preset` (the built-in preset the other fields start from)
/// and `colour` (`element` or `mono`).
pub const FIELDS: [Field; 9] = [
    Field {
        key: "bond_length",
        unit: "pt",
        min: 4.0,
        max: 200.0,
        get: |s| s.bond_length,
        put: |s, v| s.bond_length = v,
    },
    Field {
        key: "line_width",
        unit: "pt",
        min: 0.1,
        max: 20.0,
        get: |s| s.line_width,
        put: |s, v| s.line_width = v,
    },
    Field {
        key: "bold_width",
        unit: "pt",
        min: 0.1,
        max: 40.0,
        get: |s| s.bold_width,
        put: |s, v| s.bold_width = v,
    },
    Field {
        key: "hash_spacing",
        unit: "pt",
        min: 0.5,
        max: 40.0,
        get: |s| s.hash_spacing,
        put: |s, v| s.hash_spacing = v,
    },
    Field {
        key: "bond_spacing",
        unit: "of_bond",
        min: 0.02,
        max: 0.5,
        get: |s| s.bond_spacing,
        put: |s, v| s.bond_spacing = v,
    },
    Field {
        key: "margin",
        unit: "pt",
        min: 0.0,
        max: 20.0,
        get: |s| s.margin,
        put: |s, v| s.margin = v,
    },
    Field {
        key: "font_size",
        unit: "pt",
        min: 3.0,
        max: 100.0,
        get: |s| s.font_size,
        put: |s, v| s.font_size = v,
    },
    Field {
        key: "subscript_scale",
        unit: "of_font",
        min: 0.3,
        max: 1.0,
        get: |s| s.subscript_scale,
        put: |s, v| s.subscript_scale = v,
    },
    Field {
        key: "pathway.margin",
        unit: "of_bond",
        min: 0.0,
        max: 5.0,
        get: |s| s.pathway_margin,
        put: |s, v| s.pathway_margin = v,
    },
];

/// Whether `key` is a key of the `style` object this version reads and writes.
fn known(key: &str) -> bool {
    key == "preset" || key == "colour" || key == "maps" || FIELDS.iter().any(|f| f.key == key)
}

/// The schema of the settings file as JSON, for the surfaces that show or document it (the fields of the page, the help of the
/// CLI): `{"format", "presets", "colours", "fields": [{"key", "unit", "min", "max", "default"}]}`, the defaults being the ones of
/// the preset `mzlab`.
pub fn schema_json() -> String {
    let base = Style::default();
    let num = |v: f64| Value::from(v).to_string();
    let fields: Vec<String> = FIELDS
        .iter()
        .map(|f| {
            format!(
                r#"{{"key":"{}","unit":"{}","min":{},"max":{},"default":{}}}"#,
                f.key,
                f.unit,
                num(f.min),
                num(f.max),
                num(f.of(&base))
            )
        })
        .collect();
    format!(
        r#"{{"format":"{FORMAT}","presets":["{}"],"colours":["element","mono"],"display":{{"maps":["on","off"]}},"fields":[{}]}}"#,
        PRESETS.join("\",\""),
        fields.join(",")
    )
}

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
            pathway_margin: 0.35,
            mono: false,
            maps: true,
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
    /// Whether the map numbers of the atoms are drawn.
    pub maps: bool,
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
            pathway_margin: 0.35,
            mono: true,
            maps: true,
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

    /// Whether every number is one a drawing can be made with: finite and within the bounds of its field.
    fn valid(&self) -> bool {
        FIELDS.iter().all(|f| {
            let v = f.of(self);
            v.is_finite() && v >= f.min && v <= f.max
        })
    }

    /// Sets one key of the style from its text, as the `--set style.<key>=<value>` of the CLI and the fields of the page give
    /// it: a number for a field of `FIELDS`, `element` or `mono` for `colour`, a preset name for `preset` (which replaces the
    /// whole style and keeps the unknown keys). A value outside the bounds, or a key that is none, changes nothing; the error is
    /// an i18n key.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
        let mut next = self.clone();
        match (key, value.trim()) {
            ("preset", name) => {
                next = Style::preset(name).ok_or("error.draw2.style")?;
                next.unknown = self.unknown.clone();
            }
            ("colour", "mono") => next.mono = true,
            ("colour", "element") => next.mono = false,
            ("colour", _) => return Err("error.draw2.styleValue"),
            (key, v) => {
                let field = FIELDS
                    .iter()
                    .find(|f| f.key == key)
                    .ok_or("error.draw2.styleKey")?;
                (field.put)(&mut next, v.parse().map_err(|_| "error.draw2.styleValue")?);
            }
        }
        if !next.valid() {
            return Err("error.draw2.styleValue");
        }
        *self = next;
        Ok(())
    }

    /// Sets a display option from its text: `maps` is `on` or `off` (`display.maps`).
    pub fn set_display(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
        match (key, value.trim()) {
            ("maps", "on") => self.maps = true,
            ("maps", "off") => self.maps = false,
            ("maps", _) => return Err("error.draw2.styleValue"),
            _ => return Err("error.draw2.styleKey"),
        }
        Ok(())
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
                .filter(|(k, _)| !known(k))
                .map(|(k, x)| (k.clone(), x.to_string()))
                .collect(),
        };
        let g = Fields(style);
        let base = match g.s("preset").ok()? {
            Some(name) => Style::preset(&name)?,
            None => Style::default(),
        };
        let colour = match g.s("colour").ok()?.as_deref() {
            None => base.mono,
            Some("mono") => true,
            Some("element") => false,
            Some(_) => return None,
        };
        let maps = match g.s("maps").ok()?.as_deref() {
            None => base.maps,
            Some("on") => true,
            Some("off") => false,
            Some(_) => return None,
        };
        let mut s = Style {
            mono: colour,
            maps,
            unknown,
            ..base
        };
        for f in &FIELDS {
            if let Some(v) = g.f(f.key).ok()? {
                (f.put)(&mut s, v);
            }
        }
        s.valid().then_some(s)
    }

    /// The style as a settings file: fixed key order (alphabetical), every known field written, the unknown keys it was read
    /// with written back unchanged (a known key always wins over an unknown one of the same name), so the same style gives the
    /// same bytes.
    pub fn to_json(&self) -> String {
        let text = |v: &str| Value::from(v).to_string();
        let num = |v: f64| Value::from(v).to_string();
        let mut style = self.unknown.style.clone();
        style.push(("preset".to_string(), text(&self.name)));
        style.push((
            "colour".to_string(),
            text(if self.mono { "mono" } else { "element" }),
        ));
        // written only when off, so that a file that never mentions it keeps its bytes
        if !self.maps {
            style.push(("maps".to_string(), text("off")));
        }
        for f in &FIELDS {
            style.push((f.key.to_string(), num(f.of(self))));
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
            maps: self.maps,
        }
    }
}
