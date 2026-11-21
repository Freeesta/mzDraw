//! KET, the JSON drawing format of Ketcher and Indigo, written and read from its published schema. Out: one molecule node with
//! every atom (abbreviations expanded, R groups as `rg-label` atoms, wedges as bond `stereo`), arrows, plus signs and texts as
//! root nodes. In: the molecules of the file become one V3000 record for the strict reader of `mzlab-mol`, through the same door
//! as a molfile (`formats::prepare`), so R groups, query atoms and bonds and S-groups are treated and reported the same way; the
//! root nodes mzDraw does not draw are counted, never dropped in silence.

use crate::doc::{round4, Document, ObjKind, Writer, WEDGE_DOWN, WEDGE_UP};
use crate::io::Imported;
use crate::label::{self, Kind};
use serde_json::Value;

/// `"x":..,"y":..,"z":0`, the inside of a KET point.
fn point(x: f64, y: f64) -> String {
    format!(r#""x":{},"y":{},"z":0"#, Value::from(x), Value::from(y))
}

/// `[x,y,0]`, a KET location.
fn location(x: f64, y: f64) -> String {
    format!("[{},{},0]", Value::from(x), Value::from(y))
}

/// A KET text node: the content is the raw content of the rich-text editor Ketcher keeps its texts in, one unstyled block per line.
fn text_node(s: &str, x: f64, y: f64) -> String {
    let blocks: Vec<String> = (s.lines())
        .map(|l| {
            format!(
                r#"{{"text":{},"type":"unstyled","depth":0,"inlineStyleRanges":[],"entityRanges":[],"data":{{}}}}"#,
                Value::from(l)
            )
        })
        .collect();
    let content = format!(r#"{{"blocks":[{}],"entityMap":{{}}}}"#, blocks.join(","));
    format!(
        r#"{{"type":"text","data":{{"content":{},"position":{{{}}}}}}}"#,
        Value::from(content),
        point(x, y)
    )
}

impl Document {
    /// The whole canvas as a KET text (written by hand, as the document is).
    pub fn to_ket(&self) -> String {
        let m = self.to_mol();
        let mut atoms = Vec::new();
        for (i, a) in m.atoms.iter().enumerate() {
            let p = m.coords.as_ref().and_then(|c| c.xy.get(i));
            let (x, y) = p.map_or((0.0, 0.0), |p| (p[0], p[1]));
            let drawn = self.atoms.get(i);
            let mut w = Writer("{".into());
            match drawn.and_then(|d| label::kind(&d.lab)) {
                Some(Kind::R(n)) => {
                    w.text("type", "rg-label");
                    let refs = if n == 0 {
                        String::new()
                    } else {
                        format!("\"rg-{n}\"")
                    };
                    w.raw("$refs", &format!("[{refs}]"));
                }
                _ => {
                    w.text("label", &a.el);
                    // the molfile codes of a radical: 2 one unpaired electron, 3 two
                    for (key, v) in [
                        ("charge", i64::from(a.chg)),
                        ("isotope", i64::from(a.iso)),
                        ("radical", i64::from(a.rad.min(2)) + i64::from(a.rad > 0)),
                    ] {
                        if v != 0 {
                            w.int(key, v);
                        }
                    }
                    if let Some(h) = drawn.filter(|d| d.lab.is_empty()).and_then(|d| d.h) {
                        w.int("implicitHCount", i64::from(h));
                    }
                }
            }
            if let Some(d) = drawn.filter(|d| d.map != 0) {
                w.int("mapping", i64::from(d.map));
            }
            w.raw("location", &location(x, y));
            atoms.push(w.0 + "}");
        }
        let mut bonds = Vec::new();
        for (k, b) in m.bonds.iter().enumerate() {
            let ends = (self.bonds.get(k).filter(|d| d.s != 0))
                .and_then(|d| Some((d.s, self.atom_index(d.a)?, self.atom_index(d.b)?)));
            bonds.push(match ends {
                // a wedge starts at its narrow end
                Some((s, narrow, wide)) => {
                    let stereo = match s {
                        WEDGE_UP => 1,
                        WEDGE_DOWN => 6,
                        _ => 4,
                    };
                    format!(r#"{{"type":1,"atoms":[{narrow},{wide}],"stereo":{stereo}}}"#)
                }
                None => format!(r#"{{"type":{},"atoms":[{},{}]}}"#, b.o, b.a, b.b),
            });
        }
        let mut nodes = Vec::new();
        let mut molecule = String::new();
        if !atoms.is_empty() {
            nodes.push(r#"{"$ref":"mol0"}"#.to_string());
            molecule = format!(
                r#","mol0":{{"type":"molecule","atoms":[{}],"bonds":[{}]}}"#,
                atoms.join(","),
                bonds.join(",")
            );
        }
        for o in &self.objects {
            match o.t {
                ObjKind::Arrow => {
                    nodes.push(format!(
                        r#"{{"type":"arrow","data":{{"mode":"open-angle","pos":[{{{}}},{{{}}}]}}}}"#,
                        point(o.x, o.y),
                        point(o.x2, o.y2)
                    ));
                    if !o.text.is_empty() {
                        // KET has no text on an arrow: it is written as a text above its middle
                        let (x, y) = ((o.x + o.x2) / 2.0, (o.y + o.y2) / 2.0 + 0.5);
                        nodes.push(text_node(&o.text, round4(x), round4(y)));
                    }
                }
                ObjKind::Plus => nodes.push(format!(
                    r#"{{"type":"plus","location":{}}}"#,
                    location(o.x, o.y)
                )),
                ObjKind::Text => nodes.push(text_node(&o.text, o.x, o.y)),
            }
        }
        format!(r#"{{"root":{{"nodes":[{}]}}{molecule}}}"#, nodes.join(","))
    }
}

/// A coordinate of a KET point, by name or by place.
fn coord(p: &Value, name: &str, place: usize) -> f64 {
    let v = p.get(name).or_else(|| p.get(place));
    v.and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .unwrap_or(0.0)
}

/// The text of a KET text node: the lines of its rich-text content, or the content as it is.
fn content(data: &Value) -> String {
    let raw = data["content"].as_str().unwrap_or_default();
    let rich: Option<Value> = serde_json::from_str(raw).ok();
    match rich.as_ref().and_then(|r| r["blocks"].as_array()) {
        Some(blocks) => {
            let lines: Vec<&str> = blocks.iter().filter_map(|b| b["text"].as_str()).collect();
            lines.join("\n")
        }
        None => raw.into(),
    }
}

/// Reads a KET text (already parsed: `v` has a `root`). The error is an i18n key.
pub(crate) fn from_ket(v: &Value, info: &mut Imported) -> Result<Document, &'static str> {
    let nodes = v["root"]["nodes"].as_array().ok_or("error.draw2.ket")?;
    let (mut atoms, mut bonds, mut objects) = (Vec::new(), Vec::new(), Vec::new());
    let mut far: (f64, usize, bool) = (0.0, 0, false);
    for node in nodes {
        let kind = node["type"].as_str().unwrap_or_default();
        let data = &node["data"];
        if let Some(mol) = node["$ref"]
            .as_str()
            .map(|r| &v[r])
            .filter(|m| m["type"] == "molecule")
        {
            let base = atoms.len();
            for a in mol["atoms"].as_array().ok_or("error.draw2.ket")? {
                let (x, y) = (coord(&a["location"], "x", 0), coord(&a["location"], "y", 1));
                for (c, is_y) in [(x, false), (y, true)] {
                    if c.abs() > far.0.abs() {
                        far = (c, atoms.len(), is_y);
                    }
                }
                // as text through `fixed`: an `f64` given to the formatter of the standard library costs 22 kB of module
                let (x, y) = (mzlab_mol::fixed(x, 4), mzlab_mol::fixed(y, 4));
                let int = |k: &str| {
                    a[k].as_i64()
                        .filter(|v| (-999..=999).contains(v))
                        .unwrap_or(0)
                };
                // the atom-to-atom map of a reaction
                let map = match a.get("mapping").filter(|v| !v.is_null()) {
                    None => 0,
                    Some(v) => match v.as_u64() {
                        Some(m) if m <= u64::from(crate::doc::MAX_MAP) => m,
                        // negative, above the limit or no whole number: counted, read as no map
                        _ => {
                            info.maps_left += 1;
                            0
                        }
                    },
                };
                let label = a["label"].as_str().unwrap_or("A");
                let plain = mzlab_mol::is_element(label);
                let mut line = match a["type"].as_str() {
                    Some("rg-label") => {
                        let n = a["$refs"][0].as_str().and_then(|r| r.strip_prefix("rg-"));
                        let n = n.filter(|n| n.bytes().all(|b| b.is_ascii_digit()));
                        format!(
                            "R# {x} {y} 0 {map}{}",
                            n.map_or(String::new(), |n| format!(" RGROUPS=(1 {n})"))
                        )
                    }
                    // an atom list, or a label that is no element, is a query atom
                    Some(_) => format!("A {x} {y} 0 {map}"),
                    None => format!("{} {x} {y} 0 {map}", if plain { label } else { "A" }),
                };
                for (key, name) in [("charge", "CHG"), ("radical", "RAD"), ("isotope", "MASS")] {
                    if int(key) != 0 {
                        line += &format!(" {name}={}", int(key));
                    }
                }
                atoms.push(line);
            }
            for b in mol["bonds"].as_array().map_or(&[][..], |b| b) {
                let end = |k: usize| b["atoms"][k].as_u64().map(|i| i as usize + base + 1);
                let (Some(i), Some(j)) = (end(0), end(1)) else {
                    return Err("error.draw2.ket");
                };
                // a bond without a type, or of a type beyond the molfile ones, is a query bond
                let order = b["type"]
                    .as_u64()
                    .filter(|t| (1..=8).contains(t))
                    .unwrap_or(8);
                let cfg = match b["stereo"].as_u64() {
                    Some(1) => " CFG=1",
                    Some(6) => " CFG=3",
                    Some(4) => " CFG=2",
                    _ => "",
                };
                bonds.push(format!("{order} {i} {j}{cfg}"));
            }
            info.sgroups += mol["sgroups"].as_array().map_or(0, Vec::len);
        } else if kind == "arrow" || kind == "plus" || kind == "text" {
            let p = &data["pos"];
            let (t, from, to, text) = match kind {
                "arrow" => (ObjKind::Arrow, &p[0], &p[1], String::new()),
                "plus" => (
                    ObjKind::Plus,
                    &node["location"],
                    &Value::Null,
                    String::new(),
                ),
                _ => (
                    ObjKind::Text,
                    data.get("position").unwrap_or(&p[0]),
                    &Value::Null,
                    content(data),
                ),
            };
            objects.push((
                t,
                [
                    coord(from, "x", 0),
                    coord(from, "y", 1),
                    coord(to, "x", 0),
                    coord(to, "y", 1),
                ],
                text,
            ));
        } else {
            // an R-group definition, a shape, an image, a multi-tailed arrow
            info.other += 1;
        }
    }
    crate::formats::assemble(&atoms, &bonds, objects, far, info, "error.draw2.ket")
}
