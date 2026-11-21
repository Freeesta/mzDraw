//! Drawing checks, the conformance report `std/1` (docs/study/arch-std.md section 4). A check never blocks and never changes the
//! document: it names a key, a grade and the ids it is about. This slice holds the nine stereo checks (`std.stereo.*`): they read
//! the wedge marks the way the InChI Technical Manual (software 1.07, section "Stereochemical layer") reads them, because that
//! is the reading the other programs share.
//!
//! What is a possible stereocentre or stereo double bond is not decided here: it is `Mol::stereo_sites` of `mzlab-mol` on the
//! record of the drawing (`Document::to_mol`), the same perception the molfile and the SMILES go through. A site on an atom that
//! only the record has (the atoms an abbreviation expands to) or on a labelled atom has no drawn position and is left out.
//!
//! Conventions the checks rest on (a summary of the Technical Manual, not a copy):
//! - a solid or hashed wedge acts on its narrow end (the atom `a` of the bond) and on that atom only; the other reading, in which
//!   it acts on both ends, gives another identifier when a wedge joins two centres (`wideEndCentre`);
//! - the parity of a centre is the volume of the tetrahedron of its neighbours, all drawn at one length, the wide end of a solid
//!   wedge lifted 45 degrees out of the plane and that of a hashed wedge lowered 45 degrees; a centre outside that tetrahedron is
//!   not fixed by the drawing (`sameKindThree`);
//! - a wavy bond means "unknown" and is a statement, not a gap; an allene is shown by two opposite wedges at one end.
//!
//! The thresholds (10 degrees) are ours, chosen to flag clear cases only.

use crate::doc::{Document, WEDGE_DOWN, WEDGE_UP, WEDGE_WAVY};
use crate::m;
use mzlab_mol::Site;

/// How serious a finding is: `error` a reader will get the chemistry wrong, `warn` ambiguous or against a recommendation, `info`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Error,
    Warn,
    Info,
}

impl Grade {
    pub fn name(self) -> &'static str {
        match self {
            Grade::Error => "error",
            Grade::Warn => "warn",
            Grade::Info => "info",
        }
    }
}

/// The checks of this slice, in the order of the study: key and grade. The page, the documentation and the tests read this list;
/// the text of a key is the i18n key `draw2.` + the key (`draw2.std.stereo.notCentre`).
pub const CHECKS: [(&str, Grade); 13] = [
    ("std.stereo.wideEndCentre", Grade::Warn),
    ("std.stereo.notCentre", Grade::Warn),
    ("std.stereo.opposite", Grade::Warn),
    ("std.stereo.sameKindThree", Grade::Error),
    ("std.stereo.collinear", Grade::Warn),
    ("std.stereo.wavyNotCentre", Grade::Info),
    ("std.stereo.undefined", Grade::Info),
    ("std.stereo.doubleCollinear", Grade::Warn),
    ("std.stereo.allene", Grade::Warn),
    ("std.scheme.anchor", Grade::Warn),
    ("std.scheme.gapRaised", Grade::Info),
    ("std.scheme.free", Grade::Info),
    ("std.scheme.crossing", Grade::Warn),
];

/// Two bonds at an atom count as opposite (or collinear) when the angle between them is within this many degrees of 180.
pub const OPPOSITE_DEG: f64 = 10.0;

/// The format field of the report.
pub const FORMAT: &str = "std/1";

/// One finding: a check and what it is about (ids ascending, each list possibly empty).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub key: &'static str,
    pub grade: Grade,
    pub atoms: Vec<u32>,
    pub bonds: Vec<u32>,
    pub objects: Vec<u32>,
}

impl Finding {
    pub(crate) fn new(key: &'static str, atoms: &[u32], bonds: &[u32]) -> Finding {
        let grade = CHECKS
            .iter()
            .find(|c| c.0 == key)
            .map_or(Grade::Info, |c| c.1);
        let ids = |l: &[u32]| {
            let mut v = l.to_vec();
            m::sort(&mut v);
            v.dedup();
            v
        };
        Finding {
            key,
            grade,
            atoms: ids(atoms),
            bonds: ids(bonds),
            objects: Vec::new(),
        }
    }

    /// The smallest id this finding is about (the order of the report within a key).
    fn first(&self) -> u32 {
        let all = self.atoms.iter().chain(&self.bonds).chain(&self.objects);
        all.copied().min().unwrap_or(0)
    }
}

/// The report: findings ordered by key, then by their smallest id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub checks: Vec<Finding>,
}

impl Report {
    /// What a panel lists: each key that has findings, with its grade and how many, in the order of the report.
    pub fn counts(&self) -> Vec<(&'static str, Grade, usize)> {
        let mut out: Vec<(&'static str, Grade, usize)> = Vec::new();
        for f in &self.checks {
            match out.last_mut() {
                Some(last) if last.0 == f.key => last.2 += 1,
                _ => out.push((f.key, f.grade, 1)),
            }
        }
        out
    }

    /// `counts` as JSON, for the state of a page: `[{"key","grade","n"}]`.
    pub fn counts_json(&self) -> String {
        let items: Vec<String> = (self.counts().iter())
            .map(|(k, g, n)| format!("{{\"key\":\"{k}\",\"grade\":\"{}\",\"n\":{n}}}", g.name()))
            .collect();
        format!("[{}]", items.join(","))
    }

    /// `{"format":"std/1","checks":[{"key","grade","atoms","bonds","objects"}]}`: written by hand, the ids ascending, an empty
    /// list left out. The same document gives the same bytes on every surface.
    pub fn to_json(&self) -> String {
        let list = |name: &str, l: &[u32]| {
            if l.is_empty() {
                return String::new();
            }
            let ids: Vec<String> = l.iter().map(u32::to_string).collect();
            format!(",\"{name}\":[{}]", ids.join(","))
        };
        let items: Vec<String> = (self.checks.iter())
            .map(|f| {
                format!(
                    "{{\"key\":\"{}\",\"grade\":\"{}\"{}{}{}}}",
                    f.key,
                    f.grade.name(),
                    list("atoms", &f.atoms),
                    list("bonds", &f.bonds),
                    list("objects", &f.objects)
                )
            })
            .collect();
        format!(
            "{{\"format\":\"{FORMAT}\",\"checks\":[{}]}}",
            items.join(",")
        )
    }
}

/// One neighbour of an atom: its id, the id of the bond, and the direction of the bond in degrees (`None` when both ends sit on
/// one point, so there is no direction).
struct Nb {
    atom: u32,
    bond: u32,
    deg: Option<f64>,
}

/// What the drawing says about stereo, gathered once.
struct Ctx<'a> {
    d: &'a Document,
    /// Atom ids of the possible stereocentres.
    centres: Vec<u32>,
    /// Atom ids (`a` < `b`) of the possible stereo double bonds.
    doubles: Vec<(u32, u32)>,
}

impl Ctx<'_> {
    fn is_centre(&self, id: u32) -> bool {
        self.centres.contains(&id)
    }

    /// Whether the atom is an end of a possible stereo double bond.
    fn on_double(&self, id: u32) -> bool {
        self.doubles.iter().any(|&(a, b)| a == id || b == id)
    }

    /// The bonds of an atom with the direction of each, in bond order.
    fn nbs(&self, id: u32) -> Vec<Nb> {
        let d = self.d;
        let here = d.atom(id);
        (d.bonds.iter())
            .filter_map(|b| {
                let other = if b.a == id {
                    b.b
                } else if b.b == id {
                    b.a
                } else {
                    return None;
                };
                let (p, q) = (here?, d.atom(other)?);
                let (dx, dy) = (q.x - p.x, q.y - p.y);
                let deg = (m::hypot(dx, dy) > 1e-9).then(|| m::atan2(dy, dx).to_degrees());
                Some(Nb {
                    atom: other,
                    bond: b.id,
                    deg,
                })
            })
            .collect()
    }

    /// The solid or hashed wedges whose narrow end is `id`: (bond id, mark).
    fn wedges_at(&self, id: u32) -> Vec<(u32, u8)> {
        (self.d.bonds.iter())
            .filter(|b| b.a == id && (b.s == WEDGE_UP || b.s == WEDGE_DOWN))
            .map(|b| (b.id, b.s))
            .collect()
    }

    /// Whether a bond is a solid or hashed wedge that acts on `id`.
    fn marked(&self, bond: u32, id: u32) -> bool {
        (self.d.bond(bond)).is_some_and(|b| b.a == id && (b.s == WEDGE_UP || b.s == WEDGE_DOWN))
    }

    /// The atom of an allene whose two double bonds are the only bonds of it, with the ends: (middle, end, end).
    fn allenes(&self) -> Vec<(u32, u32, u32)> {
        let d = self.d;
        let mut out = Vec::new();
        for a in &d.atoms {
            let bonds: Vec<&crate::doc::Bond> = (d.bonds.iter())
                .filter(|b| b.a == a.id || b.b == a.id)
                .collect();
            if let [p, q] = bonds[..]
                && a.el == "C"
                && p.o == 2
                && q.o == 2
            {
                let end = |b: &crate::doc::Bond| if b.a == a.id { b.b } else { b.a };
                out.push((a.id, end(p), end(q)));
            }
        }
        out
    }
}

/// Angle between two directions in degrees, 0 to 180.
fn between(a: f64, b: f64) -> f64 {
    let mut x = (a - b).abs() % 360.0;
    if x > 180.0 {
        x = 360.0 - x;
    }
    x
}

/// Whether the origin lies strictly inside the tetrahedron of four points: solves `sum(l_i p_i) = 0, sum(l_i) = 1` and asks for
/// every `l_i` above a small margin. A flat tetrahedron has no inside.
fn inside(p: [[f64; 3]; 4]) -> bool {
    let col = |i: usize| [p[i][0] - p[3][0], p[i][1] - p[3][1], p[i][2] - p[3][2]];
    let (u, v, w) = (col(0), col(1), col(2));
    let det = |a: [f64; 3], b: [f64; 3], c: [f64; 3]| {
        a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
            + a[2] * (b[0] * c[1] - b[1] * c[0])
    };
    let d = det(u, v, w);
    if d.abs() < 1e-9 {
        return false;
    }
    let rhs = [-p[3][0], -p[3][1], -p[3][2]];
    let l = [det(rhs, v, w) / d, det(u, rhs, w) / d, det(u, v, rhs) / d];
    let l4 = 1.0 - l[0] - l[1] - l[2];
    l.iter().chain([&l4]).all(|&x| x > 1e-9)
}

/// The construction of the Technical Manual for a centre of three drawn neighbours and an implicit hydrogen, when every mark at the
/// centre is of one kind (`up`: solid): the neighbours at one length, the marked ones lifted (or lowered) 45 degrees, the hydrogen
/// straight out of the plane on the other side. Whether the centre is inside that tetrahedron.
fn three_fixed(nbs: &[(f64, bool)], up: bool) -> bool {
    let (s, c) = (m::sin(45f64.to_radians()), m::cos(45f64.to_radians()));
    let side = if up { 1.0 } else { -1.0 };
    let pt = |&(deg, marked): &(f64, bool)| {
        let (k, z) = if marked { (c, side * s) } else { (1.0, 0.0) };
        let r = deg.to_radians();
        [k * m::cos(r), k * m::sin(r), z]
    };
    inside([pt(&nbs[0]), pt(&nbs[1]), pt(&nbs[2]), [0.0, 0.0, -side]])
}

/// The drawing checks of a document, never changing it.
pub fn report(doc: &Document) -> Report {
    let mut out: Vec<Finding> = Vec::new();
    let ctx = sites(doc);
    let allenes = ctx.allenes();
    let at_allene_end = |id: u32| allenes.iter().any(|&(_, e, f)| e == id || f == id);

    for b in &doc.bonds {
        let wedge = b.s == WEDGE_UP || b.s == WEDGE_DOWN;
        if wedge && ctx.is_centre(b.b) {
            out.push(Finding::new("std.stereo.wideEndCentre", &[b.b], &[b.id]));
        }
        if wedge && !ctx.is_centre(b.a) && !at_allene_end(b.a) {
            out.push(Finding::new("std.stereo.notCentre", &[b.a], &[b.id]));
        }
        if b.s == WEDGE_WAVY
            && !ctx.is_centre(b.a)
            && !ctx.on_double(b.a)
            && !ctx.on_double(b.b)
            && !at_allene_end(b.a)
        {
            out.push(Finding::new("std.stereo.wavyNotCentre", &[b.a], &[b.id]));
        }
    }

    for &c in &ctx.centres {
        let nbs = ctx.nbs(c);
        let marks = ctx.wedges_at(c);
        let angle = |bond: u32| nbs.iter().find(|n| n.bond == bond).and_then(|n| n.deg);
        // two marks on bonds that point away from each other
        for (i, &(p, _)) in marks.iter().enumerate() {
            for &(q, _) in &marks[i + 1..] {
                if let (Some(x), Some(y)) = (angle(p), angle(q))
                    && between(x, y) >= 180.0 - OPPOSITE_DEG
                {
                    out.push(Finding::new("std.stereo.opposite", &[c], &[p, q]));
                }
            }
        }
        // two bonds drawn as one straight line through the centre, not both marked (two marks are `opposite`)
        for (i, p) in nbs.iter().enumerate() {
            for q in &nbs[i + 1..] {
                let straight = match (p.deg, q.deg) {
                    (Some(x), Some(y)) => between(x, y) >= 180.0 - OPPOSITE_DEG,
                    _ => false,
                };
                if straight && !(ctx.marked(p.bond, c) && ctx.marked(q.bond, c)) {
                    out.push(Finding::new(
                        "std.stereo.collinear",
                        &[c],
                        &[p.bond, q.bond],
                    ));
                }
            }
        }
        // three drawn neighbours, marks of one kind, the centre outside the tetrahedron
        if let (3, Some(&(_, first))) = (nbs.len(), marks.first()) {
            let same = marks.iter().all(|&(_, s)| s == first);
            let seen: Option<Vec<(f64, bool)>> = (nbs.iter())
                .map(|n| Some((n.deg?, ctx.marked(n.bond, c))))
                .collect();
            if let Some(seen) = seen.filter(|_| same)
                && !three_fixed(&seen, first == WEDGE_UP)
            {
                let ids: Vec<u32> = marks.iter().map(|x| x.0).collect();
                out.push(Finding::new("std.stereo.sameKindThree", &[c], &ids));
            }
        }
        // no mark of any kind (a wavy bond is a statement that it is not known)
        let stated = doc.bonds.iter().any(|b| b.a == c && b.s != 0);
        if !stated {
            out.push(Finding::new("std.stereo.undefined", &[c], &[]));
        }
    }

    for &(a, b) in &ctx.doubles {
        let Some(db) = doc.bond_between(a, b) else {
            continue;
        };
        let (na, nb) = (ctx.nbs(a), ctx.nbs(b));
        let others = |l: &[Nb], skip: u32| l.iter().filter(|n| n.atom != skip).count();
        // an end with nothing drawn on it: the double bond cannot be read as E or Z
        let stated = (doc.bonds.iter())
            .any(|x| x.s == WEDGE_WAVY && [x.a, x.b].iter().any(|e| [a, b].contains(e)));
        if !stated && (others(&na, b) == 0 || others(&nb, a) == 0) {
            out.push(Finding::new("std.stereo.undefined", &[a, b], &[db.id]));
        }
        // a substituent drawn on the line of the double bond
        for (partner, list) in [(b, &na), (a, &nb)] {
            let axis = list.iter().find(|n| n.atom == partner).and_then(|n| n.deg);
            for n in list.iter().filter(|n| n.atom != partner) {
                if let (Some(x), Some(y)) = (axis, n.deg)
                    && between(x, y) >= 180.0 - OPPOSITE_DEG
                {
                    out.push(Finding::new(
                        "std.stereo.doubleCollinear",
                        &[a, b],
                        &[db.id, n.bond],
                    ));
                }
            }
        }
    }

    for &(mid, e1, e2) in &allenes {
        let (w1, w2) = (ctx.wedges_at(e1), ctx.wedges_at(e2));
        let all: Vec<(u32, u8)> = w1.iter().chain(&w2).copied().collect();
        // the one way an allene is shown: two marks at one end, one solid and one hashed
        let shown = matches!(
            (&w1[..], &w2[..]),
            ([x, y], []) | ([], [x, y]) if x.1 != y.1
        );
        if !all.is_empty() && !shown {
            let ids: Vec<u32> = all.iter().map(|x| x.0).collect();
            out.push(Finding::new("std.stereo.allene", &[mid], &ids));
        }
    }

    // arrows anchored to an atom that is gone: kept as written, used as a free end
    let mut dangling: Vec<u32> = (doc.objects.iter())
        .filter(|o| {
            [o.from, o.to]
                .iter()
                .any(|&a| a != 0 && doc.atom(a).is_none())
        })
        .map(|o| o.id)
        .collect();
    if !dangling.is_empty() {
        let mut f = Finding::new("std.scheme.anchor", &[], &[]);
        crate::m::sort(&mut dangling);
        f.objects = dangling;
        out.push(f);
    }

    sort_findings(&mut out);
    Report { checks: out }
}

/// The order of a report: by key, then by the smallest id, the ids; equal findings once.
pub(crate) fn sort_findings(out: &mut Vec<Finding>) {
    out.sort_by(|x, y| {
        (x.key, x.first(), &x.atoms, &x.bonds, &x.objects).cmp(&(
            y.key,
            y.first(),
            &y.atoms,
            &y.bonds,
            &y.objects,
        ))
    });
    out.dedup();
}

/// The sites of the record of the drawing, as ids of the drawing.
fn sites(doc: &Document) -> Ctx<'_> {
    let mut ctx = Ctx {
        d: doc,
        centres: Vec::new(),
        doubles: Vec::new(),
    };
    if doc.atoms.is_empty() {
        return ctx;
    }
    // the atoms of the record come in pool order, then the atoms an abbreviation expands to, which have no place of their own
    let drawn = |i: u32| (doc.atoms.get(i as usize)).filter(|a| a.lab.is_empty());
    for site in doc.to_mol().stereo_sites() {
        match site {
            Site::Tetra(c) => ctx.centres.extend(drawn(c).map(|a| a.id)),
            Site::Db(a, b) => {
                if let (Some(p), Some(q)) = (drawn(a), drawn(b)) {
                    ctx.doubles.push((p.id.min(q.id), p.id.max(q.id)));
                }
            }
        }
    }
    ctx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_has_unique_keys_in_the_stereo_family() {
        for (i, (k, _)) in CHECKS.iter().enumerate() {
            assert!(
                k.starts_with("std.stereo.") || k.starts_with("std.scheme."),
                "{k}"
            );
            assert!(CHECKS[i + 1..].iter().all(|c| c.0 != *k), "{k}");
        }
    }

    #[test]
    fn the_empty_report_is_written_in_full() {
        assert_eq!(
            Report::default().to_json(),
            r#"{"format":"std/1","checks":[]}"#
        );
    }

    #[test]
    fn angles_between_directions_wrap() {
        assert_eq!(between(170.0, -170.0), 20.0);
        assert_eq!(between(0.0, 180.0), 180.0);
        assert_eq!(between(90.0, 90.0), 0.0);
    }

    #[test]
    fn a_flat_or_open_tetrahedron_has_no_inside() {
        let ok = [
            [-0.87, -0.5, 0.0],
            [0.87, -0.5, 0.0],
            [0.0, 0.7, 0.7],
            [0.0, 0.0, -1.0],
        ];
        assert!(inside(ok));
        let flat = [ok[0], ok[1], [0.0, 0.7, 0.0], [0.0, 0.0, 0.0]];
        assert!(!inside(flat));
    }
}
