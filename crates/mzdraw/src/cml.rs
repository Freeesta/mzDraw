//! CML (Chemical Markup Language, molecular convention) and MRV (the Marvin documents of ChemAxon, a CML dialect), from their
//! published convention, documentation and schema. One reader for both: every `molecule` with an `atomArray`, in the element
//! form (`atom` children) or the array form (lists on `atomArray`), goes through the same door as a molfile and the other
//! canvas files (`formats::assemble`); of an MRV file the reaction arrow, the reaction signs, the text boxes and the polyline
//! arrows are read too, and what mzDraw does not draw (S-groups as nested molecules, R-group definitions, rectangles, brackets)
//! is counted and said. CML is written (`Document::to_cml`); it has no place for arrows and texts, so only structures go out.
//! Not stated in the pages read: the direction of y for `x2` / `y2` (taken upwards, as in a molfile, which is how files of both
//! kinds are known to be written).

use crate::doc::{Document, ObjKind, WEDGE_DOWN, WEDGE_UP};
use crate::formats::Placed;
use crate::io::Imported;
use crate::label::{self, Kind};
use crate::xml::{self, Element};
use mzlab_mol::fixed;

const KEY: &str = "error.draw2.cml";

/// The name of an element without its namespace prefix.
fn local(e: &Element) -> &str {
    e.name.rsplit(':').next().unwrap_or("")
}

fn child<'a>(e: &'a Element, name: &str) -> Option<&'a Element> {
    e.children.iter().find(|c| local(c) == name)
}

fn number(v: Option<&str>) -> f64 {
    let n = v.and_then(|t| t.trim().parse::<f64>().ok());
    n.filter(|n| n.is_finite()).unwrap_or(0.0)
}

#[derive(Default)]
struct Read {
    atoms: Vec<String>,
    bonds: Vec<String>,
    objects: Vec<Placed>,
    far: (f64, usize, bool),
    other: usize,
    sgroups: usize,
}

/// One atom as its attributes say, whatever the form they came in.
#[derive(Default)]
struct AtomIn<'a> {
    element: &'a str,
    x: f64,
    y: f64,
    charge: &'a str,
    isotope: &'a str,
    spin: &'a str,
    radical: &'a str,
    rgroup: &'a str,
}

impl Read {
    fn walk(&mut self, e: &Element, depth: usize) -> Result<(), &'static str> {
        for c in &e.children {
            let points: Vec<(f64, f64)> = (c.children.iter())
                .filter(|p| local(p) == "MPoint")
                .map(|p| (number(p.attr("x")), number(p.attr("y"))))
                .collect();
            match local(c) {
                "molecule" if child(c, "atomArray").is_some() => self.molecule(c)?,
                // a drawing is its molecules wherever they stand: in a document, a structure, a reaction and its lists
                "molecule" | "cml" | "MDocument" | "MChemicalStruct" | "reaction"
                | "reactantList" | "productList" | "agentList"
                    if depth < xml::MAX_DEPTH =>
                {
                    self.walk(c, depth + 1)?
                }
                "arrow" => {
                    let p = ["x1", "y1", "x2", "y2"].map(|k| number(c.attr(k)));
                    self.objects.push((ObjKind::Arrow, p, String::new()));
                }
                "MReactionSign" if !points.is_empty() => {
                    let n = points.len() as f64;
                    let (x, y) = points
                        .iter()
                        .fold((0.0, 0.0), |s, p| (s.0 + p.0 / n, s.1 + p.1 / n));
                    self.objects
                        .push((ObjKind::Plus, [x, y, 0.0, 0.0], String::new()));
                }
                "MTextBox" if !points.is_empty() => {
                    let field = child(c, "Field").map_or("", |f| f.text.as_str());
                    // the formatting of a Marvin text is written in braces before the words: `{D font=Arial,size=11}text`
                    let mut text = String::new();
                    let mut depth = 0usize;
                    for ch in field.chars() {
                        match ch {
                            '{' => depth += 1,
                            '}' => depth = depth.saturating_sub(1),
                            _ if depth == 0 => text.push(ch),
                            _ => {}
                        }
                    }
                    let top = points.iter().map(|p| p.1).fold(f64::MIN, f64::max);
                    let left = points.iter().map(|p| p.0).fold(f64::MAX, f64::min);
                    self.objects
                        .push((ObjKind::Text, [left, top - 0.5, 0.0, 0.0], text));
                }
                // a polyline with a head is an arrow from its first point to its last
                "MPolyline" if points.len() >= 2 && c.attr("headLength").is_some() => {
                    let (a, b) = (points[0], points[points.len() - 1]);
                    self.objects
                        .push((ObjKind::Arrow, [a.0, a.1, b.0, b.1], String::new()));
                }
                name if name.starts_with('M') && name != "MHead" || name == "Rgroup" => {
                    self.other += 1
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn atom(&mut self, a: &AtomIn) {
        let index = self.atoms.len();
        for (c, is_y) in [(a.x, false), (a.y, true)] {
            if c.abs() > self.far.0.abs() {
                self.far = (c, index, is_y);
            }
        }
        let int = |v: &str| {
            v.trim()
                .parse::<i64>()
                .ok()
                .filter(|v| (-999..=999).contains(v) && *v != 0)
        };
        let mut props = String::new();
        let r = int(a.rgroup).filter(|n| (1..=32).contains(n));
        let symbol = if a.element == "R" || r.is_some() {
            if let Some(n) = r {
                props = format!(" RGROUPS=(1 {n})");
            }
            "R#"
        } else if mzlab_mol::is_element(a.element) {
            // the molfile codes of a radical: 2 one unpaired electron (a doublet), 3 two (a triplet), 1 a singlet
            let radical = match (a.radical, int(a.spin)) {
                ("monovalent", _) | (_, Some(2)) => 2,
                ("divalent3" | "triplet", _) | (_, Some(3)) => 3,
                ("divalent" | "divalent1", _) => 1,
                _ => 0,
            };
            for (name, v) in [
                ("CHG", int(a.charge)),
                ("RAD", Some(radical).filter(|r| *r != 0)),
                ("MASS", int(a.isotope)),
            ] {
                if let Some(v) = v {
                    props += &format!(" {name}={v}");
                }
            }
            a.element
        } else {
            // a dummy, a list, a pseudo atom: a query atom for the reader
            "A"
        };
        self.atoms.push(format!(
            "{symbol} {} {} 0 0{props}",
            fixed(a.x, 4),
            fixed(a.y, 4)
        ));
    }

    fn molecule(&mut self, m: &Element) -> Result<(), &'static str> {
        // the nested molecules of a Marvin molecule are its S-groups
        self.sgroups += m.children.iter().filter(|c| local(c) == "molecule").count();
        let array = child(m, "atomArray").ok_or(KEY)?;
        let base = self.atoms.len();
        let mut ids: Vec<(&str, usize)> = Vec::new();
        if let Some(list) = array.attr("atomID") {
            // the array form: one list per property, blank-separated, in the order of `atomID`
            let col = |name: &str| -> Vec<&str> {
                array.attr(name).unwrap_or("").split_whitespace().collect()
            };
            let cols = [
                "elementType",
                "x2",
                "y2",
                "formalCharge",
                "isotope",
                "radical",
                "rgroupRef",
            ]
            .map(col);
            for (i, id) in list.split_whitespace().enumerate() {
                let at = |k: usize| cols[k].get(i).copied().unwrap_or("");
                ids.push((id, self.atoms.len()));
                self.atom(&AtomIn {
                    element: at(0),
                    x: number(Some(at(1))),
                    y: number(Some(at(2))),
                    charge: at(3),
                    isotope: at(4),
                    radical: at(5),
                    rgroup: at(6),
                    ..AtomIn::default()
                });
            }
        }
        for a in array.children.iter().filter(|c| local(c) == "atom") {
            let get = |k: &str| a.attr(k).unwrap_or("");
            ids.push((get("id"), self.atoms.len()));
            // an R group written by this module carries its number in a label
            let lab = child(a, "label")
                .and_then(|l| l.attr("value"))
                .unwrap_or("");
            self.atom(&AtomIn {
                element: get("elementType"),
                x: number(a.attr("x2").or(a.attr("x3"))),
                y: number(a.attr("y2").or(a.attr("y3"))),
                charge: get("formalCharge"),
                isotope: a.attr("isotopeNumber").or(a.attr("isotope")).unwrap_or(""),
                spin: get("spinMultiplicity"),
                radical: get("radical"),
                rgroup: a
                    .attr("rgroupRef")
                    .unwrap_or(lab.strip_prefix('R').unwrap_or("")),
            });
        }
        if self.atoms.len() == base {
            return Err(KEY);
        }
        crate::m::sort(&mut ids);
        let at = |id: &str| -> Result<usize, &'static str> {
            let k = ids.binary_search_by(|x| x.0.cmp(id)).map_err(|_| KEY)?;
            Ok(ids[k].1 + 1)
        };
        let bonds = child(m, "bondArray").map_or(&[][..], |b| &b.children[..]);
        for b in bonds.iter().filter(|c| local(c) == "bond") {
            let stereo = child(b, "bondStereo");
            // a wedge or a hatch starts at the first atom of its own `atomRefs2`, else of the bond's
            let refs = stereo
                .and_then(|s| s.attr("atomRefs2"))
                .or(b.attr("atomRefs2"))
                .unwrap_or("");
            let mut ends = refs.split_whitespace();
            let (i, j) = (
                at(ends.next().unwrap_or(""))?,
                at(ends.next().unwrap_or(""))?,
            );
            let order = match b.attr("order").unwrap_or("1") {
                "1" | "S" => 1,
                "2" | "D" => 2,
                "3" | "T" => 3,
                "A" => 4,
                // partial orders, hydrogen and coordinate bonds: a query bond for the reader
                _ => 8,
            };
            let mark = stereo.map(|s| {
                (
                    s.text.trim(),
                    s.attr("conventionValue"),
                    s.attr("dictRef").unwrap_or(""),
                )
            });
            let cfg = match mark {
                Some(("W", ..)) | Some((_, Some("1"), _)) => " CFG=1",
                Some(("H", ..)) | Some((_, Some("6"), _)) => " CFG=3",
                Some((_, Some("4"), _)) => " CFG=2",
                Some((_, _, d)) if d.ends_with("wavy") => " CFG=2",
                _ => "",
            };
            self.bonds.push(format!("{order} {i} {j}{cfg}"));
        }
        Ok(())
    }
}

/// Reads a CML or an MRV text. The error is an i18n key.
pub(crate) fn from_cml(text: &str, info: &mut Imported) -> Result<Document, &'static str> {
    let root = xml::parse(text).ok_or(KEY)?;
    let mut r = Read::default();
    // the root is a `cml` (or a bare `molecule`): it is walked as the only child of a holder
    let holder = Element {
        children: vec![root],
        ..Element::default()
    };
    if !matches!(local(&holder.children[0]), "cml" | "molecule") {
        return Err(KEY);
    }
    r.walk(&holder, 0)?;
    info.other += r.other;
    info.sgroups += r.sgroups;
    crate::formats::assemble(&r.atoms, &r.bonds, r.objects, r.far, info, KEY)
}

impl Document {
    /// The structures of the canvas as CML (molecular convention): one `molecule` with every atom (abbreviations expanded, R
    /// groups as the element `R` with their label), bonds with order `S` / `D` / `T`, wedges as `bondStereo` `W` / `H` from
    /// their narrow end. CML has no place for arrows, plus signs and texts: they are not written.
    pub fn to_cml(&self) -> String {
        let m = self.to_mol();
        // the namespace of our own terms (R-group labels, wavy bonds) is the slug of the name in use, at the address of the page
        let (slug, url) = (crate::brand::SLUG, crate::brand::PAGES_URL);
        let mut out = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<cml xmlns=\"http://www.xml-cml.org/schema\" xmlns:convention=\"http://www.xml-cml.org/convention/\" xmlns:{slug}=\"{url}\" convention=\"convention:molecular\">\n",
        );
        if !m.atoms.is_empty() {
            out += "<molecule id=\"m1\">\n<atomArray>\n";
        }
        for (i, a) in m.atoms.iter().enumerate() {
            let p = m.coords.as_ref().and_then(|c| c.xy.get(i));
            let (x, y) = p.map_or((0.0, 0.0), |p| (p[0], p[1]));
            let place = format!("x2=\"{}\" y2=\"{}\"", fixed(x, 4), fixed(y, 4));
            let drawn = self.atoms.get(i);
            let lab = drawn.map_or("", |d| d.lab.as_str());
            if let Some(Kind::R(_)) = label::kind(lab) {
                out += &format!("<atom id=\"a{}\" elementType=\"R\" {place}><label dictRef=\"{slug}:rgroup\" value=\"{lab}\"/></atom>\n", i + 1);
                continue;
            }
            out += &format!("<atom id=\"a{}\" elementType=\"{}\" {place}", i + 1, a.el);
            // the spin multiplicity of one unpaired electron is 2, of two 3
            for (name, v) in [
                ("formalCharge", i64::from(a.chg)),
                ("isotopeNumber", i64::from(a.iso)),
                (
                    "spinMultiplicity",
                    if a.rad > 0 {
                        i64::from(a.rad.min(2)) + 1
                    } else {
                        0
                    },
                ),
            ] {
                if v != 0 {
                    out += &format!(" {name}=\"{v}\"");
                }
            }
            if let Some(h) = drawn.filter(|d| d.lab.is_empty()).and_then(|d| d.h) {
                out += &format!(" hydrogenCount=\"{h}\"");
            }
            out += "/>\n";
        }
        if !m.atoms.is_empty() {
            out += "</atomArray>\n";
        }
        if !m.bonds.is_empty() {
            out += "<bondArray>\n";
        }
        let index = self.index_map();
        for (k, b) in m.bonds.iter().enumerate() {
            let ends = (self.bonds.get(k).filter(|d| d.s != 0))
                .and_then(|d| Some((d.s, *index.get(&d.a)?, *index.get(&d.b)?)));
            let order = ["S", "D", "T"][usize::from(b.o.clamp(1, 3)) - 1];
            match ends {
                Some((s, narrow, wide)) => {
                    let refs = format!("atomRefs2=\"a{} a{}\"", narrow + 1, wide + 1);
                    let mark = match s {
                        WEDGE_UP => format!("<bondStereo {refs}>W</bondStereo>"),
                        WEDGE_DOWN => format!("<bondStereo {refs}>H</bondStereo>"),
                        _ => format!("<bondStereo dictRef=\"{slug}:wavy\">other</bondStereo>"),
                    };
                    out += &format!("<bond id=\"b{}\" {refs} order=\"S\">{mark}</bond>\n", k + 1);
                }
                None => {
                    out += &format!(
                        "<bond id=\"b{}\" atomRefs2=\"a{} a{}\" order=\"{order}\"/>\n",
                        k + 1,
                        b.a + 1,
                        b.b + 1
                    )
                }
            }
        }
        if !m.bonds.is_empty() {
            out += "</bondArray>\n";
        }
        if !m.atoms.is_empty() {
            out += "</molecule>\n";
        }
        out + "</cml>\n"
    }
}
