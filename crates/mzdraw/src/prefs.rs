//! The preferences: every option of the page that is a setting, in one table with its group and its text key. The page builds the
//! whole Preferences panel from `prefs_json()` (inside `settings::schema_json()`): it paints and forwards, it decides nothing.
//! Every option is a setting written `<section>.<key>=<value>` (the `--set` of the CLI), so the replay request field `set` gives
//! the same bytes as the page. A reset of a group is the list of settings `Editor::reset_commands` returns, applied one by one.

use crate::settings::{Style, FIELDS, PRESETS};
use serde_json::Value;

/// The groups of the panel, in order: the id and the text key of its title.
pub const GROUPS: [(&str, &str); 6] = [
    ("display", "draw2.prefs.group.display"),
    ("style", "draw2.prefs.group.style"),
    ("text", "draw2.prefs.group.text"),
    ("pathway", "draw2.prefs.group.pathway"),
    ("files", "draw2.prefs.group.files"),
    ("appearance", "draw2.prefs.group.appearance"),
];

/// The themes of `ui.theme`: `light` is the default (a first visit opens white, whatever the system prefers); `auto` follows the page
/// (the system or the host).
pub const THEMES: [&str; 3] = ["auto", "light", "dark"];

/// What an option takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// One of these words (the default is `Pref::default`).
    Words(&'static [&'static str]),
    /// A number of `settings::FIELDS` (unit and bounds come from there).
    Number,
    /// A comma list of the adducts of `values::ADDUCTS`.
    Adducts,
}

/// One option: the setting, its group and the text key of its label (the words have the key `<label>.<word>`).
pub struct Pref {
    pub setting: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    pub kind: Kind,
}

const COLOURS: &[&str] = &["element", "mono"];
const ONOFF: &[&str] = &["on", "off"];
const OFFON: &[&str] = &["off", "on"];
const VALUES: &[&str] = &["off", "formula", "mass", "mz"];
const LABEL: &[&str] = &["delta", "mass", "delta+mass", "name+delta"];
const ADDUCT_WORDS: &[&str] = &[
    "none",
    "[M+H]+",
    "[M+NH4]+",
    "[M+Na]+",
    "[M+K]+",
    "[M-H]-",
    "[M+Cl]-",
    "[M+HCOO]-",
];
const LENS: &[&str] = &["off", "tint", "tint+ghost"];
const DIR: &[&str] = &["right", "down"];
const ORDER: &[&str] = &["id", "mz", "rt"];
const NUMBER: &[&str] = &["layout", "mz", "rt"];
const BACKGROUND: &[&str] = &["white", "none"];
const FORMATS: &[&str] = &crate::palettes::FORMATS;
const PRESET_WORDS: &[&str] = &PRESETS;
const THEME_WORDS: &[&str] = &THEMES;
pub(crate) const FONT_WORDS: &[&str] = &crate::textedit::FONT_KEYS;
pub(crate) const SIZE_WORDS: &[&str] = &["50", "75", "100", "125", "150", "200", "300", "400"];

const fn p(setting: &'static str, group: &'static str, label: &'static str, kind: Kind) -> Pref {
    Pref {
        setting,
        group,
        label,
        kind,
    }
}

/// The options, in the order of the panel. The numbers of the style come after this table (`all`), from `FIELDS`.
const WORDS: [Pref; 18] = [
    p(
        "style.colour",
        "display",
        "draw2.prefs.style.colour",
        Kind::Words(COLOURS),
    ),
    p(
        "display.maps",
        "display",
        "draw2.prefs.display.maps",
        Kind::Words(ONOFF),
    ),
    p(
        "display.titles",
        "display",
        "draw2.prefs.display.titles",
        Kind::Words(OFFON),
    ),
    p(
        "display.values",
        "display",
        "draw2.prefs.display.values",
        Kind::Words(VALUES),
    ),
    p(
        "display.adducts",
        "display",
        "draw2.prefs.display.adducts",
        Kind::Adducts,
    ),
    p(
        "display.lens",
        "display",
        "draw2.prefs.display.lens",
        Kind::Words(LENS),
    ),
    p(
        "pathway.label",
        "display",
        "draw2.prefs.pathway.label",
        Kind::Words(LABEL),
    ),
    p(
        "pathway.adduct",
        "display",
        "draw2.prefs.pathway.adduct",
        Kind::Words(ADDUCT_WORDS),
    ),
    p(
        "style.preset",
        "style",
        "draw2.prefs.style.preset",
        Kind::Words(PRESET_WORDS),
    ),
    p(
        "text.font",
        "text",
        "draw2.prefs.text.font",
        Kind::Words(FONT_WORDS),
    ),
    p(
        "text.size",
        "text",
        "draw2.prefs.text.size",
        Kind::Words(SIZE_WORDS),
    ),
    p(
        "pathway.dir",
        "pathway",
        "draw2.prefs.pathway.dir",
        Kind::Words(DIR),
    ),
    p(
        "pathway.order",
        "pathway",
        "draw2.prefs.pathway.order",
        Kind::Words(ORDER),
    ),
    p(
        "pathway.number",
        "pathway",
        "draw2.prefs.pathway.number",
        Kind::Words(NUMBER),
    ),
    p(
        "pathway.derive",
        "pathway",
        "draw2.prefs.pathway.derive",
        Kind::Words(OFFON),
    ),
    p(
        "export.default",
        "files",
        "draw2.prefs.export.default",
        Kind::Words(FORMATS),
    ),
    p(
        "export.background",
        "files",
        "draw2.prefs.export.background",
        Kind::Words(BACKGROUND),
    ),
    p(
        "ui.theme",
        "appearance",
        "draw2.prefs.ui.theme",
        Kind::Words(THEME_WORDS),
    ),
];

/// One option of the panel, with the setting of a number as the page writes it.
pub struct Row {
    pub setting: String,
    pub group: &'static str,
    pub label: String,
    pub kind: Kind,
}

/// Every option, in the order of the panel: the words, then the numbers of the style by group (the `pathway.*` numbers are the
/// group `pathway`, the others the group `style`).
pub fn all() -> Vec<Row> {
    let mut rows: Vec<Row> = (WORDS.iter())
        .map(|w| Row {
            setting: w.setting.to_string(),
            group: w.group,
            label: w.label.to_string(),
            kind: w.kind,
        })
        .collect();
    for f in &FIELDS {
        let path = f.key.starts_with("pathway.");
        rows.push(Row {
            setting: if path {
                f.key.to_string()
            } else {
                format!("style.{}", f.key)
            },
            group: if path { "pathway" } else { "style" },
            label: format!(
                "draw2.prefs.{}",
                if path {
                    f.key.to_string()
                } else {
                    format!("style.{}", f.key)
                }
            ),
            kind: Kind::Number,
        });
    }
    // a stable order inside the panel: by group, then as listed
    let rank = |g: &str| GROUPS.iter().position(|x| x.0 == g).unwrap_or(usize::MAX);
    rows.sort_by_key(|r| rank(r.group));
    rows
}

/// The text key of a word of an option: `<label>.<word>`, with `doc` for the word of the document kind (its name is the product's,
/// which no text key carries).
pub fn word_key(label: &str, word: &str) -> String {
    format!(
        "{label}.{}",
        if crate::brand::is_doc_kind(word) {
            "doc"
        } else {
            word
        }
    )
}

/// The default word or number of an option, as text: the value of the preset `mzlab`, `svg` on `white` for the export, `light`
/// for the theme.
pub fn default_of(setting: &str) -> Option<String> {
    match setting {
        "export.default" => Some("svg".into()),
        "export.background" => Some("white".into()),
        "ui.theme" => Some("light".into()),
        "text.font" => Some("default".into()),
        "text.size" => Some("100".into()),
        _ => Style::default().value(setting),
    }
}

/// The preferences as JSON, for the schema: `{"groups": [{"id", "label"}], "prefs": [{"setting", "group", "label", "kind",
/// "default", "values" | "unit", "min", "max"}]}`; `kind` is `words` (`values`: `{"value", "label"}`, the label being a text key),
/// `number` or `adducts` (`values`: the names).
pub fn json() -> String {
    let text = |s: &str| Value::from(s).to_string();
    let num = |v: f64| Value::from(v).to_string();
    let groups: Vec<String> = GROUPS
        .iter()
        .map(|(id, label)| format!(r#"{{"id":{},"label":{}}}"#, text(id), text(label)))
        .collect();
    let rows: Vec<String> = all()
        .iter()
        .map(|r| {
            let def = text(&default_of(&r.setting).unwrap_or_default());
            let head = format!(
                r#""setting":{},"group":{},"label":{},"default":{def}"#,
                text(&r.setting),
                text(r.group),
                text(&r.label)
            );
            match r.kind {
                Kind::Words(w) => {
                    let list: Vec<String> = (w.iter())
                        .map(|x| {
                            format!(
                                r#"{{"value":{},"label":{}}}"#,
                                text(x),
                                text(&word_key(&r.label, x))
                            )
                        })
                        .collect();
                    format!(r#"{{{head},"kind":"words","values":[{}]}}"#, list.join(","))
                }
                Kind::Adducts => {
                    let list: Vec<String> =
                        crate::values::ADDUCTS.iter().map(|x| text(x)).collect();
                    format!(
                        r#"{{{head},"kind":"adducts","values":[{}]}}"#,
                        list.join(",")
                    )
                }
                Kind::Number => {
                    let key = r.setting.strip_prefix("style.").unwrap_or(&r.setting);
                    let f = FIELDS.iter().find(|f| f.key == key);
                    let (unit, min, max) = f.map_or(("", 0.0, 0.0), |f| (f.unit, f.min, f.max));
                    format!(
                        r#"{{{head},"kind":"number","unit":{},"min":{},"max":{}}}"#,
                        text(unit),
                        num(min),
                        num(max)
                    )
                }
            }
        })
        .collect();
    format!(
        r#""groups":[{}],"prefs":[{}]"#,
        groups.join(","),
        rows.join(",")
    )
}

/// The theme of the page (`ui.theme`) and the keys of the `ui` object this version does not know, kept to be written back.
#[derive(Clone, Debug, PartialEq)]
pub struct Ui {
    pub theme: String,
    unknown: Vec<(String, String)>,
}

impl Default for Ui {
    fn default() -> Self {
        Ui {
            theme: "light".into(),
            unknown: Vec::new(),
        }
    }
}

impl Ui {
    /// Reads the `ui` object of a settings file; a theme that is none is refused with `error.draw2.ui`.
    pub fn read(v: &Value) -> Result<Ui, &'static str> {
        let mut u = Ui::default();
        for (k, x) in v.as_object().ok_or("error.draw2.ui")? {
            if k == "theme" {
                u.set(k, x.as_str().ok_or("error.draw2.ui")?)?;
            } else {
                u.unknown.push((k.clone(), x.to_string()));
            }
        }
        Ok(u)
    }

    /// One setting, as `--set ui.<key>=<value>` gives it. The error is an i18n key and changes nothing.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
        match (key, value.trim()) {
            ("theme", v) if THEMES.contains(&v) => self.theme = v.into(),
            ("theme", _) => return Err("error.draw2.ui"),
            _ => return Err("error.draw2.set"),
        }
        Ok(())
    }

    /// The value the page puts in `data-theme` for the theme: `light` or `dark`, empty for `auto` (the page leaves the attribute to the
    /// system or to the host).
    pub fn attr(&self) -> &'static str {
        match self.theme.as_str() {
            "light" => "light",
            "dark" => "dark",
            _ => "",
        }
    }

    /// The `ui` object as JSON text, `None` when it holds only the defaults.
    pub fn to_text(&self) -> Option<String> {
        let mut pairs = self.unknown.clone();
        if self.theme != "light" {
            pairs.push(("theme".into(), Value::from(self.theme.as_str()).to_string()));
        }
        if pairs.is_empty() {
            return None;
        }
        pairs.sort();
        let body: Vec<String> = pairs
            .iter()
            .map(|(k, v)| format!("{}:{v}", Value::from(k.as_str())))
            .collect();
        Some(format!("{{{}}}", body.join(",")))
    }
}

/// The default font and size of a new text (`text.font`, `text.size`) and the keys of the object `text` of the settings file this version
/// does not know, kept to be written back. The font is a key of `textedit::FONTS`, the size a percent of the style's size.
#[derive(Clone, Debug, PartialEq)]
pub struct TextPrefs {
    pub font: String,
    pub size: String,
    unknown: Vec<(String, String)>,
}

impl Default for TextPrefs {
    fn default() -> Self {
        TextPrefs {
            font: "default".into(),
            size: "100".into(),
            unknown: Vec::new(),
        }
    }
}

impl TextPrefs {
    /// Reads the `text` object of a settings file; a key this version does not know is kept, a value that is none is refused with
    /// `error.draw2.textPref`.
    pub fn read(v: &Value) -> Result<TextPrefs, &'static str> {
        let mut t = TextPrefs::default();
        for (k, x) in v.as_object().ok_or("error.draw2.textPref")? {
            if k == "font" || k == "size" {
                let word = match x {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => return Err("error.draw2.textPref"),
                };
                t.set(k, &word)?;
            } else {
                t.unknown.push((k.clone(), x.to_string()));
            }
        }
        Ok(t)
    }

    /// One setting, as `--set text.<key>=<value>` gives it. The error is an i18n key and changes nothing.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
        let v = value.trim();
        match key {
            "font" if FONT_WORDS.contains(&v) => self.font = v.into(),
            "size" if SIZE_WORDS.contains(&v) => self.size = v.into(),
            "font" | "size" => return Err("error.draw2.textPref"),
            _ => return Err("error.draw2.set"),
        }
        Ok(())
    }

    /// The `text` object as JSON text, `None` when it holds only the defaults.
    pub fn to_text(&self) -> Option<String> {
        let mut pairs = self.unknown.clone();
        if self.font != "default" {
            pairs.push(("font".into(), Value::from(self.font.as_str()).to_string()));
        }
        if self.size != "100" {
            pairs.push(("size".into(), Value::from(self.size.as_str()).to_string()));
        }
        if pairs.is_empty() {
            return None;
        }
        pairs.sort();
        let body: Vec<String> = pairs
            .iter()
            .map(|(k, v)| format!("{}:{v}", Value::from(k.as_str())))
            .collect();
        Some(format!("{{{}}}", body.join(",")))
    }
}
