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
    /// The font table (id, family name) of the file.
    fonts: std::collections::HashMap<String, String>,
    /// The colour table of the file (`colour_table`), for the colours of the runs.
    table: Vec<String>,
    /// `s` objects beyond what a text holds, left out.
    runs_cut: usize,
    /// The index into the colour table of each node and each bond (`None`: the default ink), in the order they were read.
    atom_col: Vec<Option<usize>>,
    bond_col: Vec<Option<usize>>,
    /// The side of the second line (`bonddisp`: `c`, `l`, `r`, empty for none) and the display kind (`dash`, `hash`, empty for
    /// plain) of each bond, in the order they were read.
    bond_disp: Vec<(&'static str, &'static str)>,
}

/// The colour index of an object: `color` as a number; 0 and 1 (black and white) are the default ink and background, the table
/// starts at 2.
fn colour_index(e: &Element) -> Option<usize> {
    e.attr("color")?.parse::<usize>().ok().filter(|i| *i >= 2)
}

/// The colours of the `colortable` of the root as written: entry `i` is the colour `i` of the file (a file lists white and black as
/// entries 0 and 1, so the first colour of a drawing is entry 2), as canonical `#rrggbb` (an entry that is not three numbers of 0..1
/// is empty).
fn colour_table(root: &Element) -> Vec<String> {
    let Some(t) = root.each("colortable").next() else {
        return Vec::new();
    };
    t.each("color")
        .map(|c| {
            let v = |k: &str| {
                c.attr(k)
                    .and_then(|x| x.parse::<f64>().ok())
                    .filter(|x| (0.0..=1.0).contains(x))
                    .map(|x| (x * 255.0).round() as u32)
            };
            match (v("r"), v("g"), v("b")) {
                (Some(r), Some(g), Some(b)) => format!("#{r:02x}{g:02x}{b:02x}"),
                _ => String::new(),
            }
        })
        .collect()
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

/// The size of a run the file does not state, and the one the writer gives a text, in points.
const TEXT_SIZE: f64 = 10.0;

impl Read {
    /// The runs of a `t` object: each `s` with its `face` (bold 1, italic 2, underline 4, subscript 32, superscript 64, both: the
    /// formula face, which lowers the digits after a symbol), `size`, `font` and `color`; a new line in a run is a line break.
    /// Empty for a text with no style and no line break (it is the plain text).
    fn runs_of(&self, e: &Element) -> Vec<crate::runs::Run> {
        let mut out = Vec::new();
        for s in e.each("s").take(2 * crate::runs::MAX_RUNS) {
            let face: u32 = s.attr("face").and_then(|f| f.parse().ok()).unwrap_or(0);
            let size = s
                .attr("size")
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v > 0.0)
                .map_or(0.0, |v| v / TEXT_SIZE);
            let font = s.attr("font").and_then(|id| self.fonts.get(id));
            let color = s
                .attr("color")
                .and_then(|c| c.parse::<usize>().ok())
                .filter(|c| *c >= 2)
                .and_then(|c| self.table.get(c).filter(|t| !t.is_empty()));
            let style = crate::runs::Run {
                sub: face & 96 == 32,
                sup: face & 96 == 64,
                b: face & 1 != 0,
                i: face & 2 != 0,
                u: face & 4 != 0,
                font: font.filter(|f| *f != "Arial").cloned().unwrap_or_default(),
                sz: if (size - 1.0).abs() < 1e-9 { 0.0 } else { size },
                color: color.cloned().unwrap_or_default(),
                ..crate::runs::Run::default()
            };
            let lowered = face & 96 == 96;
            let mut piece = Vec::new();
            for (k, line) in s.text.split('\n').enumerate() {
                if k > 0 {
                    piece.push(crate::runs::Run::line());
                }
                piece.push(crate::runs::Run {
                    t: line.into(),
                    ..style.clone()
                });
            }
            out.extend(if lowered {
                crate::runs::formula(&piece)
            } else {
                piece
            });
        }
        let out = crate::runs::canon(out);
        if out.iter().any(|r| r.br || r.styled()) {
            out
        } else {
            Vec::new()
        }
    }

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
                "fonttable" => {
                    for f in c.each("font") {
                        if let (Some(id), Some(name)) = (f.attr("id"), f.attr("name")) {
                            self.fonts.insert(id.into(), name.into());
                        }
                    }
                }
                "t" => match numbers(c, "p").or_else(|| numbers(c, "BoundingBox")) {
                    Some(p) => {
                        self.runs_cut += c
                            .each("s")
                            .count()
                            .saturating_sub(2 * crate::runs::MAX_RUNS);
                        let (text, runs) = (label_of(c), self.runs_of(c));
                        let text = if runs.is_empty() {
                            text
                        } else {
                            crate::runs::plain(&runs)
                        };
                        self.objects
                            .push((ObjKind::Text, [p[0], -p[1], 0.0, 0.0], text, runs))
                    }
                    None => self.other += 1,
                },
                "arrow" => match (numbers(c, "Tail3D"), numbers(c, "Head3D")) {
                    (Some(t), Some(h)) => self.objects.push((
                        ObjKind::Arrow,
                        [t[0], -t[1], h[0], -h[1]],
                        String::new(),
                        Vec::new(),
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
                                Vec::new(),
                            ));
                        }
                        (Some(b), (Some("Symbol"), _, Some("Plus"))) => {
                            self.objects.push((
                                ObjKind::Plus,
                                [b[0], -b[1], 0.0, 0.0],
                                String::new(),
                                Vec::new(),
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
        self.atom_col.push(colour_index(n));
    }

    fn bond(&mut self, b: &Element) {
        let end = |k: &str| b.attr(k).unwrap_or_default().to_string();
        let order = match b.attr("Order") {
            None | Some("1") => 1,
            Some("2") => 2,
            Some("3") => 3,
            Some("1.5") => 4,
            // a dative and a hydrogen bond run from the begin atom to the end atom (types 9 and 10 of the reader's door)
            Some("dative") => 9,
            Some("hydrogen") => 10,
            // ionic, half orders, "single or double": a query bond for the reader
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
        self.bond_col.push(colour_index(b));
        // the display and the side of the second line, as the begin atom sees them (a wedge turned at its ends keeps neither)
        let kind = match b.attr("Display") {
            Some("Dash") => "dash",
            Some("Hash") => "hash",
            Some("Bold") => "bold",
            _ => "",
        };
        let side = match (order, b.attr("DoublePosition"), turned) {
            (2, Some("Left"), false) | (2, Some("Right"), true) => "l",
            (2, Some("Right"), false) | (2, Some("Left"), true) => "r",
            (2, Some("Center"), _) => "c",
            _ => "",
        };
        self.bond_disp.push((side, kind));
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
    let mut r = Read {
        table: colour_table(root),
        ..Read::default()
    };
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
    info.runs_adjusted += r.runs_cut;
    let mut d = crate::formats::assemble(&r.atoms, &bonds, r.objects, r.far, info, key)?;
    for (i, lab) in r.groups {
        if let Some(a) = d.atoms.get_mut(i) {
            a.lab = lab;
        }
    }
    // the colours: the atoms are in file order; the bonds only when the reader kept every one of them (it may refuse some)
    let table = std::mem::take(&mut r.table);
    let hex = |c: &Option<usize>| c.and_then(|i| table.get(i)).cloned().unwrap_or_default();
    for (a, c) in d.atoms.iter_mut().zip(&r.atom_col) {
        crate::ink::put(&mut a.more, &hex(c));
    }
    if d.bonds.len() == r.bond_col.len() {
        for (b, c) in d.bonds.iter_mut().zip(&r.bond_col) {
            crate::ink::put(&mut b.more, &hex(c));
        }
        for (b, (side, kind)) in d.bonds.iter_mut().zip(&r.bond_disp) {
            crate::bonddisp::pos_put(&mut b.more, side);
            // (a dative or hydrogen bond read by the door of the reader has its kind already, and keeps it: a line that is also
            // `Display="Bold"` stays what the order says)
            if !kind.is_empty() && !b.is_link() {
                crate::bonddisp::k_put(&mut b.more, kind);
            }
        }
    } else {
        // a bond was refused: the bonds of the file and the drawing cannot be paired, so none keeps its display; it is said
        info.bond_display_left += r.bond_disp.iter().filter(|d| **d != ("", "")).count();
    }
    Ok(d)
}

impl Document {
    /// The attributes `Display` and `DoublePosition` of the bond `k` (in the order of the bonds written; `from` is the begin atom of
    /// the bond as written, an index of the written atoms).
    fn display(
        &self,
        k: usize,
        from: usize,
        index: &std::collections::HashMap<u32, usize>,
    ) -> String {
        let Some(d) = self.bonds.get(k) else {
            return String::new();
        };
        let mut out = String::new();
        if d.s == 0 {
            match d.kind().as_str() {
                "dash" => out += " Display=\"Dash\"",
                "hash" => out += " Display=\"Hash\"",
                "bold" => out += " Display=\"Bold\"",
                _ => {}
            }
        }
        if d.o == 2 {
            // the side is the viewer's at the begin atom `a` of the bond; the written begin atom may be the other one
            let turned = index.get(&d.a).is_some_and(|&i| i != from);
            let name = match (d.pos().as_str(), turned) {
                ("c", _) => "Center",
                ("l", false) | ("r", true) => "Left",
                ("r", false) | ("l", true) => "Right",
                _ => "",
            };
            if !name.is_empty() {
                out += &format!(" DoublePosition=\"{name}\"");
            }
        }
        out
    }
}

impl Document {
    /// The whole canvas as a CDXML text: one fragment with every atom (abbreviations expanded, R groups as generic nicknames),
    /// wedges as bond displays from their narrow end, then texts, arrows and plus signs.
    pub fn to_cdxml(&self) -> String {
        self.to_cdxml_with(&[])
    }

    /// `to_cdxml` with the lines written under the molecules (`values::captions`) as one centred text object each, below the
    /// lowest atom of its molecule, one line per row.
    pub fn to_cdxml_with(&self, captions: &[crate::values::Caption]) -> String {
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
        // the font table of the formatted texts (Arial is 3)
        let mut fonts: Vec<&str> = Vec::new();
        for r in self.objects.iter().flat_map(|o| &o.runs) {
            if !r.font.is_empty() && r.font != "Arial" && !fonts.contains(&r.font.as_str()) {
                fonts.push(&r.font);
            }
        }
        let run = |s: &str| {
            format!(
                "<s font=\"3\" size=\"10\" face=\"0\">{}</s>",
                xml::escape(s)
            )
        };
        // the colours of the atoms and bonds, in order of first use; the table lists white and black as entries 0 and 1 (as the files of
        // tests/data do), so the first of these is entry 2
        let mut table: Vec<String> = Vec::new();
        for c in (self.atoms.iter().map(|a| a.col())).chain(self.bonds.iter().map(|b| b.col())) {
            if !c.is_empty() && !table.contains(&c) {
                table.push(c);
            }
        }
        // then the colours of the runs of the texts
        for r in self.objects.iter().flat_map(|o| &o.runs) {
            if !r.color.is_empty() && !table.contains(&r.color) {
                table.push(r.color.clone());
            }
        }
        let paint = |c: String| match table.iter().position(|t| *t == c) {
            Some(i) => format!(" color=\"{}\"", i + 2),
            None => String::new(),
        };
        // the `s` objects of the runs of a text (a line break is a new line in the text of a run)
        let styled = |runs: &[crate::runs::Run]| -> String {
            runs.iter()
                .map(|r| {
                    let font = match fonts.iter().position(|f| *f == r.font) {
                        _ if r.font.is_empty() || r.font == "Arial" => 3,
                        Some(k) => 4 + k,
                        None => 3,
                    };
                    let face = u32::from(r.b)
                        | u32::from(r.i) << 1
                        | u32::from(r.u) << 2
                        | u32::from(r.sub) << 5
                        | u32::from(r.sup) << 6;
                    let size = fixed(TEXT_SIZE * if r.sz == 0.0 { 1.0 } else { r.sz }, 2);
                    let color = match table.iter().position(|c| *c == r.color) {
                        Some(k) => format!(" color=\"{}\"", 2 + k),
                        None => String::new(),
                    };
                    let text = if r.br { "\n" } else { r.t.as_str() };
                    format!(
                        "<s font=\"{font}\" size=\"{size}\" face=\"{face}\"{color}>{}</s>",
                        xml::escape(text)
                    )
                })
                .collect()
        };
        let body = |o: &crate::doc::Obj| {
            if o.runs.is_empty() {
                run(&o.text)
            } else {
                styled(&o.runs)
            }
        };
        let mut font_table =
            String::from("<fonttable><font id=\"3\" charset=\"iso-8859-1\" name=\"Arial\"/>");
        for (k, f) in fonts.iter().enumerate() {
            font_table += &format!(
                "<font id=\"{}\" charset=\"iso-8859-1\" name=\"{}\"/>",
                4 + k,
                xml::escape(f)
            );
        }
        font_table += "</fonttable>";
        let mut out = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<!DOCTYPE CDXML SYSTEM \"http://www.camsoft.com/xml/cdxml.dtd\">\n<CDXML CreationProgram=\"{}\" BondLength=\"{}\">\n{font_table}\n{}<page>\n",
            crate::NAME,
            // as an integer: an `f64` given to the formatter of the standard library costs 22 kB of module
            BOND as u32,
            if table.is_empty() {
                String::new()
            } else {
                let rows: String = (table.iter())
                    .map(|h| {
                        let v = |i: usize| {
                            let x = u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
                            let t = fixed(f64::from(x) / 255.0, 3);
                            match t.trim_end_matches('0').trim_end_matches('.') {
                                "" => "0".to_string(),
                                t => t.to_string(),
                            }
                        };
                        format!("<color r=\"{}\" g=\"{}\" b=\"{}\"/>", v(1), v(3), v(5))
                    })
                    .collect();
                format!("<colortable><color r=\"1\" g=\"1\" b=\"1\"/><color r=\"0\" g=\"0\" b=\"0\"/>{rows}</colortable>\n")
            }
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
            out += &paint(self.atoms.get(i).map_or_else(String::new, |d| d.col()));
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
            // a dative and a hydrogen bond run from the begin atom (the donor) to the end atom
            let link = (self.bonds.get(k)).and_then(|d| {
                let name = match d.kind().as_str() {
                    "dative" => "dative",
                    "hbond" => "hydrogen",
                    _ => return None,
                };
                Some((name, *index.get(&d.a)?, *index.get(&d.b)?))
            });
            let (from, to, more) = match ends {
                Some((s, narrow, wide)) => {
                    let display = match s {
                        WEDGE_UP => "WedgeBegin",
                        WEDGE_DOWN => "WedgedHashBegin",
                        _ => "Wavy",
                    };
                    (narrow, wide, format!(" Display=\"{display}\""))
                }
                None if link.is_some() => {
                    let (name, i, j) = link.unwrap_or_default();
                    (i, j, format!(" Order=\"{name}\""))
                }
                // a double bond whose E/Z is unknown: drawn "either" (ChemDraw's wavy display of a double bond); a bond with a
                // display kind of its own keeps that one (the file has one display per bond) and the loss manifest says it
                None if self
                    .bonds
                    .get(k)
                    .is_some_and(|d| d.either() && d.kind().is_empty()) =>
                {
                    (
                        b.a as usize,
                        b.b as usize,
                        " Order=\"2\" Display=\"Wavy\"".to_string(),
                    )
                }
                None if b.o > 1 => (b.a as usize, b.b as usize, format!(" Order=\"{}\"", b.o)),
                None => (b.a as usize, b.b as usize, String::new()),
            };
            // the display kind (`Dash`, `Hash`; a bold dashed bond has no place) and the side of the second line, as seen from the
            // begin atom of the bond as written (CDX SDK pages Bond_Display and Bond_DoublePosition)
            let more = format!("{more}{}", self.display(k, b.a as usize, &index));
            let more = format!(
                "{more}{}",
                paint(self.bonds.get(k).map_or_else(String::new, |d| d.col()))
            );
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
                        out += &format!("<t id=\"{}\" p=\"{above}\">{}</t>\n", next(), body(o));
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
                        body(o)
                    )
                }
                // the public CDXML documents no curve between atoms and lone pairs: listed in the loss manifest, not written
                ObjKind::Curve => {}
            }
        }
        for c in captions.iter().filter(|c| !c.lines.is_empty()) {
            // 9 pt text, 11 pt rows; the first row stands one bond length below the lowest atom (two with a level written there)
            let at = pt(c.x, c.y - if c.below { 1.5 } else { 1.0 });
            let text = (c.lines.iter())
                .map(|l| xml::escape(l))
                .collect::<Vec<_>>()
                .join("&#xa;");
            out += &format!(
                "<t id=\"{}\" p=\"{at}\" Justification=\"Center\" LineHeight=\"auto\"><s font=\"3\" size=\"9\" color=\"0\">{text}</s></t>\n",
                next()
            );
        }
        out + "</page>\n</CDXML>\n"
    }
}
