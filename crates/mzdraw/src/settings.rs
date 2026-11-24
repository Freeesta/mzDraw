//! The settings file `mzdraw-settings/1`: the drawing style (bond length, line widths, font, colours) with its built-in presets.
//! Units are points, so a preset prints at its true size; the screen scales the same numbers by the zoom. The file is read by
//! hand from a JSON value (as the document is); every field is optional and defaults to the preset `mzlab`.

use crate::ops::Fields;
use serde_json::{json, Value};

/// The value of the `format` field of a settings file.
pub const FORMAT: &str = crate::brand::SETTINGS_ID;

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
    /// The pathway layout (`pathway.*`, `pathway::layout`): the least gap between columns, the least gap between two boxes of a
    /// column (bond lengths), the steepest primary arrow (degrees) and the least visible arrow length (bond lengths).
    pub pathway_col_gap: f64,
    pub pathway_row_gap: f64,
    pub pathway_max_angle: f64,
    pub pathway_min_arrow: f64,
    /// `pathway.dir`: whether the layers go down (`down`) instead of to the right (`right`).
    pub pathway_down: bool,
    /// `pathway.order`: the sibling order key.
    pub pathway_order: crate::pathway::Order,
    /// `pathway.number`: the order `tp:number` numbers the transformation products in.
    pub pathway_number: crate::tp_list::Number,
    /// `pathway.label`: what `pathway:label` writes on an arrow (`delta`, `mass`, `delta+mass`, `name+delta`).
    pub pathway_label: crate::steplabel::Mode,
    /// `pathway.adduct`: the adduct whose m/z a mass label adds when the scheme has no choice of its own (empty: none, written `none`).
    pub pathway_adduct: String,
    /// `pathway.derive`: whether the page offers "Derive TP" (menu item and Shift+D). Off by default: the kill test of tp-03 did not
    /// show the page route needs half the actions of copy-paste-and-edit. The command `tp:derive` is not gated.
    pub pathway_derive: bool,
    /// One ink for every element instead of the element colours.
    pub mono: bool,
    /// `style.carbon_labels`: which carbons are drawn as text (`none` today, `terminal`, `all`).
    pub carbon_labels: CarbonLabels,
    /// `style.hydrogens`: `true` for `all` (the hydrogens are also written on every carbon the style labels), `false` for `hetero`.
    pub hydrogens_all: bool,
    /// `style.hash_style`: `true` for `lines` (a hashed wedge is a row of parallel lines of one length), `false` for `wedge`.
    pub hash_lines: bool,
    /// Whether atom-to-atom map numbers are drawn at the atoms (`display.maps`, `on` by default).
    pub maps: bool,
    /// Which ids of the document are drawn (`display.ids`, `off` by default): the atom ids, the bond ids or both, the numbers the
    /// operations of `draw_edit/1` name.
    pub ids: Ids,
    /// What is drawn under each structure (`display.values`): nothing (default), the formula, the formula and the monoisotopic mass,
    /// or the m/z of the adducts.
    pub values: crate::values::Mode,
    /// The adducts whose m/z is calculated (`display.adducts`), names of `values::ADDUCTS`, in the order shown.
    pub adducts: Vec<String>,
    /// What the diff lens shows on a transformation product (`display.lens`, `off` by default).
    pub lens: LensMode,
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
pub(crate) fn object(mut pairs: Vec<(String, String)>) -> String {
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
pub const FIELDS: [Field; 13] = [
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
    Field {
        key: "pathway.col_gap",
        unit: "of_bond",
        min: 0.0,
        max: 50.0,
        get: |s| s.pathway_col_gap,
        put: |s, v| s.pathway_col_gap = v,
    },
    Field {
        key: "pathway.row_gap",
        unit: "of_bond",
        min: 0.0,
        max: 50.0,
        get: |s| s.pathway_row_gap,
        put: |s, v| s.pathway_row_gap = v,
    },
    Field {
        key: "pathway.max_angle",
        unit: "deg",
        min: 1.0,
        max: 89.0,
        get: |s| s.pathway_max_angle,
        put: |s, v| s.pathway_max_angle = v,
    },
    Field {
        key: "pathway.min_arrow",
        unit: "of_bond",
        min: 0.0,
        max: 50.0,
        get: |s| s.pathway_min_arrow,
        put: |s, v| s.pathway_min_arrow = v,
    },
];

/// Whether `key` is a key of the `style` object this version reads and writes.
fn known(key: &str) -> bool {
    matches!(
        key,
        "preset"
            | "colour"
            | "carbon_labels"
            | "hydrogens"
            | "hash_style"
            | "maps"
            | "ids"
            | "display.values"
            | "display.adducts"
            | "lens"
            | "pathway.dir"
            | "pathway.order"
            | "pathway.label"
            | "pathway.adduct"
            | "pathway.derive"
            | "pathway.number"
    ) || FIELDS.iter().any(|f| f.key == key)
}

/// The keys of the settings that take a word: the key, the words (the first is the default) as `--set` and the settings file read
/// them (`pathway.label` is not offered here: only `delta` can run, and a select of words that always fail is not shown). `pathway.order` has `id`, `mz` and `rt` here; `reverse-id` is for the tests of the layout and is not offered.
const CHOICES: &str = r#"{"key":"style.carbon_labels","values":["none","terminal","all"],"default":"none"},{"key":"style.hydrogens","values":["hetero","all"],"default":"hetero"},{"key":"style.hash_style","values":["wedge","lines"],"default":"wedge"},{"key":"pathway.dir","values":["right","down"],"default":"right"},{"key":"pathway.order","values":["id","mz","rt"],"default":"id"},{"key":"pathway.number","values":["layout","mz","rt"],"default":"layout"}"#;

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
        r#"{{"format":"{FORMAT}","presets":["{}"],"colours":["element","mono"],"display":{{"maps":["on","off"],"ids":["off","atoms","bonds","both"],"values":["off","formula","mass","mz"],"adducts":[{}],"lens":["off","tint","tint+ghost"]}},"choices":[{CHOICES}],{},"fields":[{}]}}"#,
        PRESETS.join("\",\""),
        crate::values::ADDUCTS
            .iter()
            .map(|a| format!("\"{a}\""))
            .collect::<Vec<_>>()
            .join(","),
        crate::prefs::json(),
        fields.join(",")
    )
}

fn default_adducts() -> Vec<String> {
    crate::values::DEFAULT_ADDUCTS
        .iter()
        .map(ToString::to_string)
        .collect()
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
            pathway_col_gap: 2.5,
            pathway_row_gap: 1.0,
            pathway_max_angle: 40.0,
            pathway_min_arrow: 1.2,
            pathway_down: false,
            pathway_order: crate::pathway::Order::Id,
            pathway_number: crate::tp_list::Number::Layout,
            pathway_label: crate::steplabel::Mode::Delta,
            pathway_adduct: String::new(),
            pathway_derive: false,
            mono: false,
            carbon_labels: CarbonLabels::None,
            hydrogens_all: false,
            hash_lines: false,
            maps: true,
            ids: Ids::Off,
            values: crate::values::Mode::Off,
            adducts: default_adducts(),
            lens: LensMode::Off,
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
    /// Which carbons are drawn as text (`style.carbon_labels`).
    pub carbons: CarbonLabels,
    /// Whether the hydrogens are written on every labelled carbon (`style.hydrogens=all`).
    pub h_all: bool,
    /// Whether a hashed wedge is drawn as parallel lines (`style.hash_style=lines`).
    pub hash_lines: bool,
    /// Whether the map numbers of the atoms are drawn.
    pub maps: bool,
    /// Which ids are drawn (`display.ids`).
    pub ids: Ids,
    /// The diff lens mode (`display.lens`).
    pub lens: LensMode,
}

/// Which carbons are drawn as text (`style.carbon_labels`): `none` (a carbon is a bare vertex unless the rules of `scene::has_label`
/// ask for a label), `terminal` (also a carbon with one bond: `CH3`, `CH2`) or `all`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CarbonLabels {
    #[default]
    None,
    Terminal,
    All,
}

impl CarbonLabels {
    /// The word of the setting.
    pub fn name(self) -> &'static str {
        match self {
            CarbonLabels::None => "none",
            CarbonLabels::Terminal => "terminal",
            CarbonLabels::All => "all",
        }
    }

    pub fn parse(v: &str) -> Option<CarbonLabels> {
        [
            CarbonLabels::None,
            CarbonLabels::Terminal,
            CarbonLabels::All,
        ]
        .into_iter()
        .find(|m| m.name() == v)
    }
}

/// What the diff lens draws on the product of an anchored arrow (`display.lens`): `off`, `tint` (what was added or changed, in the
/// accent token) or `tint+ghost` (also each cleaved bond, dashed and muted, behind the bonds).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LensMode {
    #[default]
    Off,
    Tint,
    TintGhost,
}

impl LensMode {
    /// The word of the setting.
    pub fn name(self) -> &'static str {
        match self {
            LensMode::Off => "off",
            LensMode::Tint => "tint",
            LensMode::TintGhost => "tint+ghost",
        }
    }

    pub fn parse(v: &str) -> Option<LensMode> {
        [LensMode::Off, LensMode::Tint, LensMode::TintGhost]
            .into_iter()
            .find(|m| m.name() == v)
    }
}

/// Which ids of the document a drawing shows (`display.ids`): none (default), the atom ids, the bond ids, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Ids {
    #[default]
    Off,
    Atoms,
    Bonds,
    Both,
}

impl Ids {
    /// The word of the setting.
    pub fn name(self) -> &'static str {
        match self {
            Ids::Off => "off",
            Ids::Atoms => "atoms",
            Ids::Bonds => "bonds",
            Ids::Both => "both",
        }
    }

    pub fn parse(v: &str) -> Option<Ids> {
        [Ids::Off, Ids::Atoms, Ids::Bonds, Ids::Both]
            .into_iter()
            .find(|m| m.name() == v)
    }

    /// Whether the atom ids are drawn.
    pub fn atoms(self) -> bool {
        matches!(self, Ids::Atoms | Ids::Both)
    }

    /// Whether the bond ids are drawn.
    pub fn bonds(self) -> bool {
        matches!(self, Ids::Bonds | Ids::Both)
    }
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
            pathway_col_gap: 2.5,
            pathway_row_gap: 1.0,
            pathway_max_angle: 40.0,
            pathway_min_arrow: 1.2,
            pathway_down: false,
            pathway_order: crate::pathway::Order::Id,
            pathway_number: crate::tp_list::Number::Layout,
            pathway_label: crate::steplabel::Mode::Delta,
            pathway_adduct: String::new(),
            pathway_derive: false,
            mono: true,
            carbon_labels: CarbonLabels::None,
            hydrogens_all: false,
            hash_lines: false,
            maps: true,
            ids: Ids::Off,
            values: crate::values::Mode::Off,
            adducts: default_adducts(),
            lens: LensMode::Off,
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
            ("carbon_labels", v) => {
                next.carbon_labels = CarbonLabels::parse(v).ok_or("error.draw2.styleValue")?
            }
            ("hydrogens", "hetero") => next.hydrogens_all = false,
            ("hydrogens", "all") => next.hydrogens_all = true,
            ("hydrogens", _) => return Err("error.draw2.styleValue"),
            ("hash_style", "wedge") => next.hash_lines = false,
            ("hash_style", "lines") => next.hash_lines = true,
            ("hash_style", _) => return Err("error.draw2.styleValue"),
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

    /// Sets a display option from its text: `maps` is `on` or `off` (`display.maps`), `lens` is `off`, `tint` or `tint+ghost` (`display.lens`).
    pub fn set_display(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
        match (key, value.trim()) {
            ("maps", "on") => self.maps = true,
            ("maps", "off") => self.maps = false,
            ("maps", _) => return Err("error.draw2.styleValue"),
            ("ids", v) => self.ids = Ids::parse(v).ok_or("error.draw2.styleValue")?,
            ("values", v) => {
                self.values = crate::values::Mode::parse(v).ok_or("error.draw2.styleValue")?
            }
            ("adducts", v) => {
                self.adducts = crate::values::parse_adducts(v).ok_or("error.draw2.styleValue")?
            }
            ("lens", v) => self.lens = LensMode::parse(v).ok_or("error.draw2.styleValue")?,
            _ => return Err("error.draw2.styleKey"),
        }
        Ok(())
    }

    /// Sets one key of the section `pathway` from its text (`--set pathway.<key>=<value>`): `dir` is `right` or `down`, `derive` is `on` or `off` (the page's "Derive TP"), `order` is
    /// `id`, `mz` or `rt` (and `reverse-id`, for tests), the numbers are the fields `pathway.<key>` of `FIELDS`.
    pub fn set_pathway(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
        match (key, value.trim()) {
            ("dir", "right") => self.pathway_down = false,
            ("dir", "down") => self.pathway_down = true,
            ("dir", _) => return Err("error.draw2.styleValue"),
            ("derive", "on") => self.pathway_derive = true,
            ("derive", "off") => self.pathway_derive = false,
            ("derive", _) => return Err("error.draw2.styleValue"),
            ("number", v) => {
                self.pathway_number =
                    crate::tp_list::Number::parse(v).ok_or("error.draw2.styleValue")?
            }
            ("order", v) => {
                self.pathway_order =
                    crate::pathway::Order::parse(v).ok_or("error.draw2.styleValue")?
            }
            ("adduct", "none") => self.pathway_adduct.clear(),
            ("adduct", v) if crate::values::ADDUCTS.contains(&v) => self.pathway_adduct = v.into(),
            ("adduct", _) => return Err("error.draw2.styleValue"),
            ("label", v) => {
                self.pathway_label =
                    crate::steplabel::Mode::parse(v).ok_or("error.draw2.styleValue")?
            }
            (k, _) => {
                let full = format!("pathway.{k}");
                if !FIELDS.iter().any(|f| f.key == full) {
                    return Err("error.draw2.styleKey");
                }
                return self.set(&full, value);
            }
        }
        Ok(())
    }

    /// Sets a setting of the sections `style`, `display` and `pathway` from its name `<section>.<key>` and its text, as the `--set`
    /// of the CLI gives it; the error is an i18n key and changes nothing.
    pub fn apply(&mut self, name: &str, value: &str) -> Result<(), &'static str> {
        let (section, key) = name.split_once('.').ok_or("error.draw2.set")?;
        match section {
            "style" => self.set(key, value),
            "display" => self.set_display(key, value),
            "pathway" => self.set_pathway(key, value),
            _ => Err("error.draw2.set"),
        }
    }

    /// The value of a setting of the sections `style`, `display` and `pathway` as the text `apply` takes back (a number as it is
    /// shown, a word, the adducts as a comma list); `None` for a setting that is none.
    pub fn value(&self, name: &str) -> Option<String> {
        let on = |b: bool| if b { "on" } else { "off" }.to_string();
        let number = |key: &str| {
            FIELDS
                .iter()
                .find(|f| f.key == key)
                .map(|f| f.of(self).to_string())
        };
        match name {
            "style.preset" => Some(self.name.clone()),
            "style.colour" => Some(if self.mono { "mono" } else { "element" }.into()),
            "style.carbon_labels" => Some(self.carbon_labels.name().into()),
            "style.hydrogens" => Some(if self.hydrogens_all { "all" } else { "hetero" }.into()),
            "style.hash_style" => Some(if self.hash_lines { "lines" } else { "wedge" }.into()),
            "display.maps" => Some(on(self.maps)),
            "display.ids" => Some(self.ids.name().into()),
            "display.values" => Some(self.values.name().into()),
            "display.adducts" => Some(self.adducts.join(",")),
            "display.lens" => Some(self.lens.name().into()),
            "pathway.dir" => self.choice(name).map(Into::into),
            "pathway.order" | "pathway.number" | "pathway.label" => {
                self.choice(name).map(Into::into)
            }
            "pathway.derive" => Some(on(self.pathway_derive)),
            "pathway.adduct" => Some(if self.pathway_adduct.is_empty() {
                "none".into()
            } else {
                self.pathway_adduct.clone()
            }),
            _ => number(name.strip_prefix("style.").unwrap_or(name)),
        }
    }

    /// The effective word of a key of `CHOICES` (`pathway.dir`, `pathway.order`), the default when the file leaves it out; `None` for any other key.
    pub fn choice(&self, key: &str) -> Option<&'static str> {
        match key {
            "style.carbon_labels" => Some(self.carbon_labels.name()),
            "style.hydrogens" => Some(if self.hydrogens_all { "all" } else { "hetero" }),
            "style.hash_style" => Some(if self.hash_lines { "lines" } else { "wedge" }),
            "pathway.dir" => Some(if self.pathway_down { "down" } else { "right" }),
            "pathway.order" => Some(self.pathway_order.name()),
            "display.lens" => Some(self.lens.name()),
            "pathway.number" => Some(self.pathway_number.name()),
            "pathway.label" => Some(self.pathway_label.name()),
            "display.values" => Some(self.values.name()),
            _ => None,
        }
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
            .map(crate::brand::settings_version)
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
        let carbon_labels = match g.s("carbon_labels").ok()?.as_deref() {
            None => base.carbon_labels,
            Some(v) => CarbonLabels::parse(v)?,
        };
        let hydrogens_all = match g.s("hydrogens").ok()?.as_deref() {
            None => base.hydrogens_all,
            Some("hetero") => false,
            Some("all") => true,
            Some(_) => return None,
        };
        let hash_lines = match g.s("hash_style").ok()?.as_deref() {
            None => base.hash_lines,
            Some("wedge") => false,
            Some("lines") => true,
            Some(_) => return None,
        };
        let maps = match g.s("maps").ok()?.as_deref() {
            None => base.maps,
            Some("on") => true,
            Some("off") => false,
            Some(_) => return None,
        };
        let ids = match g.s("ids").ok()?.as_deref() {
            None => base.ids,
            Some(v) => Ids::parse(v)?,
        };
        let values = match g.s("display.values").ok()? {
            None => base.values,
            Some(v) => crate::values::Mode::parse(&v)?,
        };
        let adducts = match g.s("display.adducts").ok()? {
            None => base.adducts.clone(),
            Some(v) => crate::values::parse_adducts(&v)?,
        };
        let lens = match g.s("lens").ok()?.as_deref() {
            None => base.lens,
            Some(v) => LensMode::parse(v)?,
        };
        let down = match g.s("pathway.dir").ok()?.as_deref() {
            None | Some("right") => false,
            Some("down") => true,
            Some(_) => return None,
        };
        let derive = match g.s("pathway.derive").ok()?.as_deref() {
            None => base.pathway_derive,
            Some("on") => true,
            Some("off") => false,
            Some(_) => return None,
        };
        let order = match g.s("pathway.order").ok()? {
            None => crate::pathway::Order::Id,
            Some(v) => crate::pathway::Order::parse_public(&v)?,
        };
        let number = match g.s("pathway.number").ok()? {
            None => crate::tp_list::Number::Layout,
            Some(v) => crate::tp_list::Number::parse(&v)?,
        };
        let label = match g.s("pathway.label").ok()?.as_deref() {
            None => crate::steplabel::Mode::Delta,
            Some(v) => crate::steplabel::Mode::parse(v)?,
        };
        let adduct = match g.s("pathway.adduct").ok()?.as_deref() {
            None | Some("none") => String::new(),
            Some(v) if crate::values::ADDUCTS.contains(&v) => v.to_string(),
            Some(_) => return None,
        };
        let mut s = Style {
            pathway_adduct: adduct,
            pathway_number: number,
            mono: colour,
            carbon_labels,
            hydrogens_all,
            hash_lines,
            maps,
            ids,
            values,
            adducts,
            lens,
            pathway_down: down,
            pathway_order: order,
            pathway_label: label,
            pathway_derive: derive,
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
        // written only when not the default, so that a file that never mentions them keeps its bytes
        if self.carbon_labels != CarbonLabels::None {
            style.push(("carbon_labels".to_string(), text(self.carbon_labels.name())));
        }
        if self.hydrogens_all {
            style.push(("hydrogens".to_string(), text("all")));
        }
        if self.hash_lines {
            style.push(("hash_style".to_string(), text("lines")));
        }
        if !self.maps {
            style.push(("maps".to_string(), text("off")));
        }
        if self.ids != Ids::Off {
            style.push(("ids".to_string(), text(self.ids.name())));
        }
        if self.values != crate::values::Mode::Off {
            style.push(("display.values".to_string(), text(self.values.name())));
        }
        if self.adducts != default_adducts() {
            style.push(("display.adducts".to_string(), text(&self.adducts.join(","))));
        }
        if self.lens != LensMode::Off {
            style.push(("lens".to_string(), text(self.lens.name())));
        }
        if self.pathway_down {
            style.push(("pathway.dir".to_string(), text("down")));
        }
        if self.pathway_derive {
            style.push(("pathway.derive".to_string(), text("on")));
        }
        if !matches!(
            self.pathway_order,
            crate::pathway::Order::Id | crate::pathway::Order::ReverseId
        ) {
            style.push(("pathway.order".to_string(), text(self.pathway_order.name())));
        }
        if self.pathway_number != crate::tp_list::Number::Layout {
            style.push((
                "pathway.number".to_string(),
                text(self.pathway_number.name()),
            ));
        }
        if self.pathway_label != crate::steplabel::Mode::Delta {
            style.push(("pathway.label".to_string(), text(self.pathway_label.name())));
        }
        if !self.pathway_adduct.is_empty() {
            style.push(("pathway.adduct".to_string(), text(&self.pathway_adduct)));
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
            carbons: self.carbon_labels,
            h_all: self.hydrogens_all,
            hash_lines: self.hash_lines,
            maps: self.maps,
            ids: self.ids,
            lens: self.lens,
        }
    }
}
