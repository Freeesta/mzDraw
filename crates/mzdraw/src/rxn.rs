//! Reaction files of the CTfile family: RXN (V2000 and V3000) and RD files, read; RXN V2000 written. A reaction file holds
//! molecules and their roles, not a picture: read, the reactants are set left of an arrow and the products right of it, with
//! plus signs between them and the agents above the arrow; written, the roles are taken from where each structure stands with
//! respect to the one arrow of the drawing. Every molecule goes through the same door as a molfile (`io::record_of`).

use crate::doc::{round4, Document, Obj, ObjKind};
use crate::io::{record_of, Imported, MAX_RECORDS};

const KEY: &str = "error.draw2.rxn";

/// The components of one reaction, as molfile texts: reactants, products, agents.
type Roles = [Vec<String>; 3];

fn count(s: &str) -> Option<usize> {
    s.trim().parse().ok()
}

/// `$RXN` (V2000): the counts line `rrrppp` (and `aaa`, an extension several programs write) after the four header lines, then
/// one `$MOL` block per component in that order.
fn v2000(text: &str) -> Option<Roles> {
    let head = text.lines().nth(4)?;
    let n = [
        count(head.get(0..3)?)?,
        count(head.get(3..6)?)?,
        head.get(6..9).and_then(count).unwrap_or(0),
    ];
    let mut mols = text.split("$MOL\n").skip(1).map(str::to_string);
    let mut roles = Roles::default();
    for (role, n) in roles.iter_mut().zip(n) {
        for _ in 0..n {
            role.push(mols.next()?);
        }
    }
    Some(roles)
}

/// `$RXN V3000`: every `CTAB` block inside `BEGIN REACTANT`, `BEGIN PRODUCT` or `BEGIN AGENT`, made a V3000 molfile of its own.
fn v3000(text: &str) -> Option<Roles> {
    let mut roles = Roles::default();
    let (mut role, mut open) = (None, None::<String>);
    for line in text.lines() {
        let body = line.strip_prefix("M  V30 ").unwrap_or("").trim();
        match body {
            "BEGIN REACTANT" => role = Some(0),
            "BEGIN PRODUCT" => role = Some(1),
            "BEGIN AGENT" => role = Some(2),
            "END REACTANT" | "END PRODUCT" | "END AGENT" => role = None,
            _ => {}
        }
        if body.starts_with("BEGIN CTAB") {
            open = Some("reaction\n\n\n  0  0  0     0  0            999 V3000\n".into());
        }
        if let Some(mol) = &mut open {
            mol.push_str(line);
            mol.push('\n');
        }
        if body == "END CTAB" {
            roles[role?].push(open.take()? + "M  END\n");
        }
    }
    open.is_none().then_some(roles)
}

/// Width and vertical middle of a drawing.
fn frame(d: &Document) -> (f64, f64, f64) {
    d.bounds()
        .map_or((0.0, 0.0, 0.0), |b| (b.0, b.2, (b.1 + b.3) / 2.0))
}

fn object(d: &mut Document, t: ObjKind, p: [f64; 4]) {
    let [x, y, x2, y2] = p.map(round4);
    d.objects.push(Obj {
        id: d.next.max(1),
        t,
        x,
        y,
        x2,
        y2,
        text: String::new(),
        from: 0,
        to: 0,
        more: Vec::new(),
    });
    d.next = d.next.max(1) + 1;
}

/// The structures of one side, left to right on one line, with plus signs between them.
fn side(mols: &[String], info: &mut Imported) -> Result<Document, &'static str> {
    let mut all = Document::default();
    for (i, mol) in mols.iter().enumerate() {
        let (d, placeholder) = record_of(mol, info).map_err(|_| KEY)?;
        info.placeholder |= placeholder;
        let (left, _, mid) = frame(&d);
        let (_, right, line) = frame(&all);
        if i > 0 {
            object(&mut all, ObjKind::Plus, [right + 1.0, line, 0.0, 0.0]);
            all.join(d, right + 2.0 - left, line - mid);
        } else {
            all.join(d, 0.0, 0.0);
        }
    }
    Ok(all)
}

/// The drawing of one reaction.
fn reaction(text: &str, info: &mut Imported) -> Result<Document, &'static str> {
    let roles = if text.starts_with("$RXN V3000") {
        v3000(text)
    } else {
        v2000(text)
    };
    let [reactants, products, agents] = roles.ok_or(KEY)?;
    let mut all = side(&reactants, info)?;
    let (products, mut agents) = (side(&products, info)?, side(&agents, info)?);
    // the plus signs of the agents' line say nothing: they stand above the arrow, side by side
    agents.objects.clear();
    let (a0, a1, _) = frame(&agents);
    let (_, right, line) = frame(&all);
    let length = (a1 - a0 + 1.0).max(2.0);
    let (tail, head) = (right + 1.0, right + 1.0 + length);
    object(&mut all, ObjKind::Arrow, [tail, line, head, line]);
    let low = agents.bounds().map_or(0.0, |b| b.1);
    all.join(
        agents,
        (tail + head) / 2.0 - (a0 + a1) / 2.0,
        line + 0.8 - low,
    );
    let (left, _, mid) = frame(&products);
    all.join(products, head + 1.0 - left, line - mid);
    Ok(all)
}

/// Reads a reaction file or an RD file (its reactions and molecules one under the other). The error is an i18n key.
pub(crate) fn from_rxn(text: &str, info: &mut Imported) -> Result<Document, &'static str> {
    let text = text.replace("\r\n", "\n");
    // the records of an RD file start at `$RFMT` (a reaction) or `$MFMT` (a molecule); a reaction file is one record
    let mut records: Vec<String> = Vec::new();
    if text.starts_with("$RDFILE") {
        // the data of a record (`$DTYPE`, `$DATUM` and what follows them) is not structure
        let mut data = false;
        for line in text.lines() {
            if line.starts_with("$RFMT") || line.starts_with("$MFMT") {
                records.push(String::new());
                data = false;
            } else if line.starts_with("$DTYPE") || line.starts_with("$DATUM") {
                data = true;
            } else if let Some(r) = records.last_mut().filter(|_| !data) {
                r.push_str(line);
                r.push('\n');
            }
        }
    } else {
        records.push(text);
    }
    let mut all = Document::default();
    for r in &records {
        if info.records == MAX_RECORDS {
            info.skipped += 1;
            continue;
        }
        let d = match r.find("$RXN") {
            Some(at) => reaction(&r[at..], info)?,
            None => {
                let (d, placeholder) =
                    record_of(r.strip_prefix("$MOL\n").unwrap_or(r), info).map_err(|_| KEY)?;
                info.placeholder |= placeholder;
                d
            }
        };
        info.records += 1;
        // one record under the other, left edges in line
        let dy = match (all.bounds(), d.bounds()) {
            (Some(mine), Some(new)) => mine.1 - 2.0 - new.3,
            _ => 0.0,
        };
        let dx = all.bounds().map_or(0.0, |b| b.0) - d.bounds().map_or(0.0, |b| b.0);
        all.join(d, dx, dy);
    }
    if records.is_empty() {
        return Err(KEY);
    }
    Ok(all)
}

impl Document {
    /// The atoms `ids` with the bonds between them, as a document of their own.
    pub(crate) fn part(&self, ids: &[u32]) -> Document {
        Document {
            atoms: (self.atoms.iter())
                .filter(|a| ids.contains(&a.id))
                .cloned()
                .collect(),
            bonds: (self.bonds.iter())
                .filter(|b| ids.contains(&b.a) && ids.contains(&b.b))
                .cloned()
                .collect(),
            next: self.next,
            ..Document::default()
        }
    }

    /// The drawing as a reaction file (`$RXN`, V2000). It must hold exactly one arrow: the structures before its tail are the
    /// reactants, the ones past its head the products, the ones alongside it (above or below) the agents, each side from left
    /// to right along the arrow. Agents are written as a third count, an extension of the format that several programs read;
    /// plus signs and texts have no place in the file. The error is an i18n key.
    pub fn to_rxn(&self) -> Result<String, &'static str> {
        let mut arrows = self.objects.iter().filter(|o| o.t == ObjKind::Arrow);
        let (Some(arrow), None) = (arrows.next(), arrows.next()) else {
            return Err("error.draw2.rxnArrow");
        };
        let (dx, dy) = (arrow.x2 - arrow.x, arrow.y2 - arrow.y);
        let mut roles: [Vec<(f64, String)>; 3] = Default::default();
        for ids in crate::layout::pieces(self) {
            let piece = self.part(&ids);
            let n = piece.atoms.len() as f64;
            let (cx, cy) = piece
                .atoms
                .iter()
                .fold((0.0, 0.0), |c, a| (c.0 + a.x / n, c.1 + a.y / n));
            // where the middle of the structure stands along the arrow: 0 at the tail, 1 at the head
            let t = ((cx - arrow.x) * dx + (cy - arrow.y) * dy) / (dx * dx + dy * dy);
            let role = if t < 0.0 {
                0
            } else if t > 1.0 {
                1
            } else {
                2
            };
            let mut mol = piece.to_molfile();
            if !mol.ends_with('\n') {
                mol.push('\n');
            }
            roles[role].push((t, mol));
        }
        let mut out = format!("$RXN\n\n  {}\n\n", crate::NAME);
        for (k, role) in roles.iter_mut().enumerate() {
            crate::m::sort_by(role, |a, b| a.0 < b.0);
            if role.len() > 999 {
                return Err(KEY);
            }
            if k < 2 || !role.is_empty() {
                out += &format!("{:>3}", role.len());
            }
        }
        out.push('\n');
        for (_, mol) in roles.iter().flatten() {
            out += "$MOL\n";
            out += mol;
        }
        Ok(out)
    }
}
