//! CDXML, the XML form of ChemDraw files, written and read from the public CDX SDK documentation (object and property pages;
//! no code of any other program). In: the nodes and bonds of every fragment become one record for the strict reader of
//! `mzlab-mol`, through the same door as a molfile and a KET file (`formats::assemble`), so query atoms and bonds are treated
//! and reported the same way; texts, arrows and plus signs keep their place; everything else ChemDraw can hold is counted and
//! said, never dropped in silence. Out: one fragment, texts, arrows and plus signs, in points, y down, 30 pt to the bond.
//! Two details are not stated in the pages read and follow the files ChemDraw is known to write: the head of a line `graphic`
//! is the first point of its `BoundingBox`, and `ArrowheadHead="Full"` is a full head.

use crate::doc::{Document, ObjKind, WEDGE_DOWN, WEDGE_UP};
use crate::formats::Placed;
use crate::io::Imported;
use crate::label::{self, Kind};
use crate::xml::{self, Element};
use mzlab_mol::fixed;

/// Points to one bond length in a written file.
const BOND: f64 = 30.0;
/// Element symbols by atomic number, from 1.
const SYMBOLS: &str = "H He Li Be B C N O F Ne Na Mg Al Si P S Cl Ar K Ca Sc Ti V Cr Mn Fe Co Ni Cu Zn Ga Ge As Se Br Kr Rb Sr Y Zr Nb Mo Tc Ru Rh Pd Ag Cd In Sn Sb Te I Xe Cs Ba La Ce Pr Nd Pm Sm Eu Gd Tb Dy Ho Er Tm Yb Lu Hf Ta W Re Os Ir Pt Au Hg Tl Pb Bi Po At Rn Fr Ra Ac Th Pa U Np Pu Am Cm Bk Cf Es Fm Md No Lr Rf Db Sg Bh Hs Mt Ds Rg Cn Nh Fl Mc Lv Ts Og";
/// Objects of a file that are not content of the drawing: they are passed over without a word.
const QUIET: &[&str] = &[
    "colortable",
    "fonttable",
    "scheme",
    "step",
    "color",
    "font",
    "s",
    "b",
    "n",
];

/// What is gathered from a file before it becomes a document.
#[derive(Default)]
struct Read {
    atoms: Vec<String>,
    /// Node id and index of its atom.
    ids: Vec<(String, usize)>,
    /// Begin, end (node ids), order, `CFG`.
    bonds: Vec<(String, String, u8, u8)>,
    objects: Vec<Placed>,
    /// Abbreviations of the table found as labels: atom index and label.
    groups: Vec<(usize, String)>,
    far: (f64, usize, bool),
    other: usize,
}

/// The numbers of an attribute such as `p="72 144"`, y turned up (CDXML counts y downwards).
fn numbers(e: &Element, name: &str) -> Option<Vec<f64>> {
    let v: Option<Vec<f64>> = (e.attr(name)?.split_whitespace())
        .map(|t| t.parse::<f64>().ok().filter(|v| v.is_finite()))
        .collect();
    v.filter(|v| v.len() >= 2)
}

/// The text of a `t` object: its style runs, joined.
fn label_of(e: &Element) -> String {
    let runs: Vec<&str> = e.each("s").map(|s| s.text.as_str()).collect();
    runs.concat()
}

impl Read {
    fn walk(&mut self, e: &Element, depth: usize) {
        for c in &e.children {
            match c.name.as_str() {
                "page" | "group" if depth < xml::MAX_DEPTH => self.walk(c, depth + 1),
                "fragment" => {
                    for n in c.each("n") {
                        self.node(n);
                    }
                    for b in c.each("b") {
                        self.bond(b);
                    }
                    // a fragment may hold texts and graphics of its own; the fragments inside its nodes are their expansions
                    self.walk(c, depth + 1);
                }
                "t" => match numbers(c, "p").or_else(|| numbers(c, "BoundingBox")) {
                    Some(p) => {
                        self.objects
                            .push((ObjKind::Text, [p[0], -p[1], 0.0, 0.0], label_of(c)))
                    }
                    None => self.other += 1,
                },
                "arrow" => match (numbers(c, "Tail3D"), numbers(c, "Head3D")) {
                    (Some(t), Some(h)) => self.objects.push((
                        ObjKind::Arrow,
                        [t[0], -t[1], h[0], -h[1]],
                        String::new(),
                    )),
                    _ => self.other += 1,
                },
                "graphic" => {
                    let b = numbers(c, "BoundingBox").filter(|b| b.len() >= 4);
                    let kind = (
                        c.attr("GraphicType"),
                        c.attr("ArrowType"),
                        c.attr("SymbolType"),
                    );
                    match (b, kind) {
                        // an `arrow` object of the file says the same thing in the newer way
                        _ if c.attr("SupersededBy").is_some() => {}
                        (Some(b), (Some("Line"), Some(head), _)) if head != "NoHead" => {
                            self.objects.push((
                                ObjKind::Arrow,
                                [b[2], -b[3], b[0], -b[1]],
                                String::new(),
                            ));
                        }
                        (Some(b), (Some("Symbol"), _, Some("Plus"))) => {
                            self.objects.push((
                                ObjKind::Plus,
                                [b[0], -b[1], 0.0, 0.0],
                                String::new(),
                            ));
                        }
                        _ => self.other += 1,
                    }
                }
                name if QUIET.contains(&name) => {}
                _ => self.other += 1,
            }
        }
    }

    fn node(&mut self, n: &Element) {
        let p = numbers(n, "p").unwrap_or(vec![0.0, 0.0]);
        let (x, y) = (p[0], -p[1]);
        let index = self.atoms.len();
        for (c, is_y) in [(x, false), (y, true)] {
            if c.abs() > self.far.0.abs() {
                self.far = (c, index, is_y);
            }
        }
        let int = |k: &str| {
            n.attr(k)
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|v| (-999..=999).contains(v))
        };
        let text = n.each("t").next().map(label_of).unwrap_or_default();
        let text = text.trim();
        let mut props = String::new();
        let symbol = match n.attr("NodeType") {
            None | Some("Element") => {
                for (key, name) in [("Charge", "CHG"), ("Radical", "RAD"), ("Isotope", "MASS")] {
                    if let Some(v) = int(key).filter(|v| *v != 0) {
                        props += &format!(" {name}={v}");
                    }
                }
                // no element: carbon
                let z = int("Element").unwrap_or(6);
                (SYMBOLS
                    .split(' ')
                    .nth((z - 1).max(0) as usize)
                    .filter(|_| z > 0))
                .unwrap_or("A")
            }
            // a label: an R group, an abbreviation of the table (its atoms are not read from the file), or what mzDraw has no
            // word for, which is read as a query atom
            Some(_) => match label::kind(text) {
                Some(Kind::R(n)) => {
                    if n > 0 {
                        props = format!(" RGROUPS=(1 {n})");
                    }
                    "R#"
                }
                Some(Kind::Group(_)) => {
                    self.groups.push((index, text.to_string()));
                    "R#"
                }
                None => "A",
            },
        };
        self.atoms.push(format!(
            "{symbol} {} {} 0 0{props}",
            fixed(x, 4),
            fixed(y, 4)
        ));
        self.ids
            .push((n.attr("id").unwrap_or_default().to_string(), index));
    }

    fn bond(&mut self, b: &Element) {
        let end = |k: &str| b.attr(k).unwrap_or_default().to_string();
        let order = match b.attr("Order") {
            None | Some("1") => 1,
            Some("2") => 2,
            Some("3") => 3,
            Some("1.5") => 4,
            // dative, ionic, half orders, "single or double": a query bond for the reader
            Some(_) => 8,
        };
        // the narrow end of a wedge is the begin atom for the Begin displays, the end atom for the End ones
        let (cfg, turned) = match b.attr("Display") {
            Some("WedgeBegin") => (1, false),
            Some("WedgeEnd") => (1, true),
            Some("WedgedHashBegin") => (3, false),
            Some("WedgedHashEnd") => (3, true),
            Some("Wavy") => (2, false),
            _ => (0, false),
        };
        let (from, to) = if turned {
            (end("E"), end("B"))
        } else {
            (end("B"), end("E"))
        };
        self.bonds.push((from, to, order, cfg));
    }
}

/// Reads a CDXML text. The error is an i18n key.
pub(crate) fn from_cdxml(text: &str, info: &mut Imported) -> Result<Document, &'static str> {
    const KEY: &str = "error.draw2.cdxml";
    let root = xml::parse(text).ok_or(KEY)?;
    from_tree(&root, info, KEY)
}

/// The document of an element tree whose root is `CDXML`: the text form, and the binary form (`cdx.rs`), which builds the same
/// tree, go through this one mapping. `key` is the error of the form being read.
pub(crate) fn from_tree(
    root: &Element,
    info: &mut Imported,
    key: &'static str,
) -> Result<Document, &'static str> {
    if root.name != "CDXML" {
        return Err(key);
    }
    let mut r = Read::default();
    r.walk(root, 0);
    crate::m::sort(&mut r.ids);
    let mut bonds = Vec::new();
    for (from, to, order, cfg) in &r.bonds {
        let at = |id: &String| {
            let k = r.ids.binary_search_by(|x| x.0.cmp(id)).ok()?;
            Some(r.ids[k].1 + 1)
        };
        let (i, j) = (at(from).ok_or(key)?, at(to).ok_or(key)?);
        let cfg = if *cfg == 0 {
            String::new()
        } else {
            format!(" CFG={cfg}")
        };
        bonds.push(format!("{order} {i} {j}{cfg}"));
    }
    info.other += r.other;
    let mut d = crate::formats::assemble(&r.atoms, &bonds, r.objects, r.far, info, key)?;
    for (i, lab) in r.groups {
        if let Some(a) = d.atoms.get_mut(i) {
            a.lab = lab;
        }
    }
    Ok(d)
}

impl Document {
    /// The whole canvas as a CDXML text: one fragment with every atom (abbreviations expanded, R groups as generic nicknames),
    /// wedges as bond displays from their narrow end, then texts, arrows and plus signs.
    pub fn to_cdxml(&self) -> String {
        let m = self.to_mol();
        let (x0, y1) = self.bounds().map_or((0.0, 0.0), |b| (b.0, b.3));
        // points, y down, one bond length of margin
        let pt = |x: f64, y: f64| {
            format!(
                "{} {}",
                fixed((x - x0 + 1.0) * BOND, 2),
                fixed((y1 - y + 1.0) * BOND, 2)
            )
        };
        let run = |s: &str| {
            format!(
                "<s font=\"3\" size=\"10\" face=\"0\">{}</s>",
                xml::escape(s)
            )
        };
        let mut out = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<!DOCTYPE CDXML SYSTEM \"http://www.camsoft.com/xml/cdxml.dtd\">\n<CDXML CreationProgram=\"{}\" BondLength=\"{}\">\n<fonttable><font id=\"3\" charset=\"iso-8859-1\" name=\"Arial\"/></fonttable>\n<page>\n",
            crate::NAME,
            // as an integer: an `f64` given to the formatter of the standard library costs 22 kB of module
            BOND as u32
        );
        let mut id = 0;
        let mut next = || {
            id += 1;
            id
        };
        // the fragment takes the first id; its nodes follow, so atom `i` of the record is `first + 1 + i`
        let first = next();
        if !m.atoms.is_empty() {
            out += &format!("<fragment id=\"{first}\">\n");
        }
        for (i, a) in m.atoms.iter().enumerate() {
            let p = m.coords.as_ref().and_then(|c| c.xy.get(i));
            let (x, y) = p.map_or((0.0, 0.0), |p| (p[0], p[1]));
            out += &format!("<n id=\"{}\" p=\"{}\"", next(), pt(x, y));
            let drawn = self.atoms.get(i).map_or("", |d| d.lab.as_str());
            if let Some(Kind::R(_)) = label::kind(drawn) {
                out += &format!(
                    " NodeType=\"GenericNickname\" GenericNickname=\"{drawn}\"><t>{}</t></n>\n",
                    run(drawn)
                );
                continue;
            }
            let z = mzlab_mol::atomic_number(&a.el).unwrap_or(6);
            if z != 6 {
                out += &format!(" Element=\"{z}\" NumHydrogens=\"{}\"", a.h);
            }
            // the molfile codes of a radical are the ones of CDXML: 2 one unpaired electron, 3 two
            for (name, v) in [
                ("Charge", i64::from(a.chg)),
                ("Isotope", i64::from(a.iso)),
                ("Radical", i64::from(a.rad.min(2)) + i64::from(a.rad > 0)),
            ] {
                if v != 0 {
                    out += &format!(" {name}=\"{v}\"");
                }
            }
            // a carbon is drawn as a corner; any other atom carries its label
            if z == 6 {
                out += "/>\n";
            } else {
                let h = match a.h {
                    0 => String::new(),
                    1 => "H".into(),
                    n => format!("H{n}"),
                };
                out += &format!("><t>{}</t></n>\n", run(&format!("{}{h}", a.el)));
            }
        }
        let index = self.index_map();
        for (k, b) in m.bonds.iter().enumerate() {
            let ends = (self.bonds.get(k).filter(|d| d.s != 0))
                .and_then(|d| Some((d.s, *index.get(&d.a)?, *index.get(&d.b)?)));
            let (from, to, more) = match ends {
                Some((s, narrow, wide)) => {
                    let display = match s {
                        WEDGE_UP => "WedgeBegin",
                        WEDGE_DOWN => "WedgedHashBegin",
                        _ => "Wavy",
                    };
                    (narrow, wide, format!(" Display=\"{display}\""))
                }
                None if b.o > 1 => (b.a as usize, b.b as usize, format!(" Order=\"{}\"", b.o)),
                None => (b.a as usize, b.b as usize, String::new()),
            };
            out += &format!(
                "<b id=\"{}\" B=\"{}\" E=\"{}\"{more}/>\n",
                next(),
                first + 1 + from,
                first + 1 + to
            );
        }
        if !m.atoms.is_empty() {
            out += "</fragment>\n";
        }
        for o in &self.objects {
            match o.t {
                ObjKind::Arrow => {
                    let (tail, head) = (pt(o.x, o.y), pt(o.x2, o.y2));
                    let (old, new) = (next(), next());
                    // the line graphic of older readers, pointing at the arrow object that says it in the newer way
                    out += &format!("<graphic id=\"{old}\" SupersededBy=\"{new}\" BoundingBox=\"{head} {tail}\" GraphicType=\"Line\" ArrowType=\"FullHead\"/>\n<arrow id=\"{new}\" Head3D=\"{head} 0\" Tail3D=\"{tail} 0\" ArrowheadHead=\"Full\"/>\n");
                    if !o.text.is_empty() {
                        let above = pt((o.x + o.x2) / 2.0, (o.y + o.y2) / 2.0 + 0.5);
                        out +=
                            &format!("<t id=\"{}\" p=\"{above}\">{}</t>\n", next(), run(&o.text));
                    }
                }
                ObjKind::Plus => {
                    let (c, size) = (pt(o.x, o.y), pt(o.x + 0.25, o.y));
                    out += &format!("<graphic id=\"{}\" BoundingBox=\"{c} {size}\" GraphicType=\"Symbol\" SymbolType=\"Plus\"/>\n", next());
                }
                ObjKind::Text => {
                    out += &format!(
                        "<t id=\"{}\" p=\"{}\">{}</t>\n",
                        next(),
                        pt(o.x, o.y),
                        run(&o.text)
                    )
                }
            }
        }
        out + "</page>\n</CDXML>\n"
    }
}
