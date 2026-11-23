//! Arranging the selection: align on an edge, distribute with equal gaps, turn by a typed angle, nudge (a scale waits for `atoms.xy`). Each command is
//! planned here as existing operations only (`move`, `rotate`), so the log of a page replays natively
//! to the same bytes and every step has the exact inverse of its operations. The commands are `align:left|centre|right|top|middle|
//! bottom`, `distribute:h|v`, `rotate:<deg>` and `nudge:<dx>,<dy>`; the arguments `atoms` and `objects` of a command
//! name what it acts on (the page leaves them out: it acts on the selection, or on everything when nothing is selected).
//!
//! The unit that moves is an *item*: a connected piece (an atom of the selection stands for its whole structure), or one selected object. An item's box is the box of
//! its anchor points (atoms; the ends of an arrow, the anchor of a plus or a text), in document axes (y up): `top` is the largest y.
//! Everything is deterministic: items are ordered by position, then by their lowest id, and every shift is rounded to the document grid.

use crate::doc::{round4, Document, MAX_COORD};
use crate::ops::Op;
use std::collections::HashMap;

/// Why a command cannot run (an i18n key).
pub type Refusal = &'static str;

/// The edge an `align` command lines the items up on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Centre,
    Right,
    Top,
    Middle,
    Bottom,
}

/// A parsed arrange command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cmd {
    Align(Edge),
    /// Equal gaps along x (`true`) or along y (`false`).
    Distribute(bool),
    Rotate(f64),
    Nudge(f64, f64),
}

/// The largest angle or shift a command takes; a larger value is an invalid request.
const LIMIT: f64 = MAX_COORD;
/// Parses `align:left`, `distribute:h`, `rotate:45`, `nudge:0.5,-1`; `None` for anything else (a number that is not finite,
/// a factor outside `0.01..=100`, an unknown edge).
pub fn parse(name: &str) -> Option<Cmd> {
    let (head, rest) = name.split_once(':')?;
    let num = |s: &str| {
        s.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && v.abs() <= LIMIT)
    };
    match head {
        "align" => Some(Cmd::Align(match rest {
            "left" => Edge::Left,
            "centre" => Edge::Centre,
            "right" => Edge::Right,
            "top" => Edge::Top,
            "middle" => Edge::Middle,
            "bottom" => Edge::Bottom,
            _ => return None,
        })),
        "distribute" => match rest {
            "h" => Some(Cmd::Distribute(true)),
            "v" => Some(Cmd::Distribute(false)),
            _ => None,
        },
        "rotate" => num(rest).map(Cmd::Rotate),
        "nudge" => {
            let (dx, dy) = rest.split_once(',')?;
            Some(Cmd::Nudge(num(dx)?, num(dy)?))
        }
        _ => None,
    }
}

/// One thing that moves as a whole.
struct Item {
    atoms: Vec<u32>,
    objects: Vec<u32>,
    /// x0, y0, x1, y1 of the anchor points.
    b: (f64, f64, f64, f64),
    /// The lowest id, to order items that stand alike.
    key: (u8, u32),
}

fn grow(b: &mut Option<(f64, f64, f64, f64)>, x: f64, y: f64) {
    *b = Some(match *b {
        None => (x, y, x, y),
        Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
    });
}

/// The items of a selection: a structure is moved whole, so an atom of the selection stands for the connected piece it is in; an
/// object is an item of its own. An atom or object that is not in the document is `error.draw2.noAtom` / `error.draw2.noObject`.
fn items(doc: &Document, atoms: &[u32], objects: &[u32]) -> Result<Vec<Item>, Refusal> {
    let pieces = crate::layout::pieces(doc);
    let mut piece: HashMap<u32, usize> = HashMap::new();
    for (k, p) in pieces.iter().enumerate() {
        for &a in p {
            piece.insert(a, k);
        }
    }
    let mut taken = std::collections::HashSet::new();
    let mut by_piece: Vec<Item> = Vec::new();
    for &id in atoms {
        doc.atom(id).ok_or("error.draw2.noAtom")?;
        let k = piece[&id];
        if !taken.insert(k) {
            continue;
        }
        let mut ids = pieces[k].clone();
        ids.sort_unstable();
        let mut b = None;
        for &i in &ids {
            let a = doc.atom(i).ok_or("error.draw2.noAtom")?;
            grow(&mut b, a.x, a.y);
        }
        by_piece.push(Item {
            b: b.unwrap_or((0.0, 0.0, 0.0, 0.0)),
            key: (0, ids[0]),
            atoms: ids,
            objects: Vec::new(),
        });
    }
    let mut out = by_piece;
    let mut seen = std::collections::HashSet::new();
    for &id in objects {
        let o = doc.object(id).ok_or("error.draw2.noObject")?;
        if !seen.insert(id) {
            continue;
        }
        let mut b = None;
        grow(&mut b, o.x, o.y);
        if o.t == crate::doc::ObjKind::Arrow {
            grow(&mut b, o.x2, o.y2);
        }
        out.push(Item {
            atoms: Vec::new(),
            objects: vec![id],
            b: b.unwrap_or((o.x, o.y, o.x, o.y)),
            key: (1, id),
        });
    }
    Ok(out)
}

fn mv(it: &Item, dx: f64, dy: f64) -> Option<Op> {
    let (dx, dy) = (round4(dx), round4(dy));
    (dx != 0.0 || dy != 0.0).then(|| Op::Move {
        atoms: it.atoms.clone(),
        objects: it.objects.clone(),
        dx,
        dy,
    })
}

/// The operations of a command on the given atoms and objects (the whole drawing when both are empty). An empty list is not a
/// refusal: the drawing is arranged that way already.
pub fn plan(doc: &Document, cmd: Cmd, atoms: &[u32], objects: &[u32]) -> Result<Vec<Op>, Refusal> {
    let whole = atoms.is_empty() && objects.is_empty();
    let all_atoms: Vec<u32>;
    let all_objs: Vec<u32>;
    let (atoms, objects) = if whole {
        all_atoms = doc.atoms.iter().map(|a| a.id).collect();
        all_objs = doc.objects.iter().map(|o| o.id).collect();
        (&all_atoms[..], &all_objs[..])
    } else {
        (atoms, objects)
    };
    if atoms.is_empty() && objects.is_empty() {
        return Err("error.draw2.arrangeNone");
    }
    match cmd {
        Cmd::Nudge(dx, dy) => Ok(vec![Op::Move {
            atoms: atoms.to_vec(),
            objects: objects.to_vec(),
            dx: round4(dx),
            dy: round4(dy),
        }]),
        Cmd::Rotate(deg) => {
            if atoms.is_empty() {
                return Err("error.draw2.arrangeAtoms");
            }
            let (x, y) = centre(doc, atoms)?;
            Ok(vec![Op::Rotate {
                atoms: atoms.to_vec(),
                deg,
                x,
                y,
            }])
        }
        Cmd::Align(edge) => {
            let its = items(doc, atoms, objects)?;
            if its.len() < 2 {
                return Err("error.draw2.arrangeTwo");
            }
            let u = union(&its);
            Ok(its
                .iter()
                .filter_map(|it| {
                    let (b, w) = (it.b, ((it.b.0 + it.b.2) / 2.0, (it.b.1 + it.b.3) / 2.0));
                    match edge {
                        Edge::Left => mv(it, u.0 - b.0, 0.0),
                        Edge::Right => mv(it, u.2 - b.2, 0.0),
                        Edge::Centre => mv(it, (u.0 + u.2) / 2.0 - w.0, 0.0),
                        Edge::Bottom => mv(it, 0.0, u.1 - b.1),
                        Edge::Top => mv(it, 0.0, u.3 - b.3),
                        Edge::Middle => mv(it, 0.0, (u.1 + u.3) / 2.0 - w.1),
                    }
                })
                .collect())
        }
        Cmd::Distribute(h) => {
            let mut its = items(doc, atoms, objects)?;
            if its.len() < 3 {
                return Err("error.draw2.arrangeThree");
            }
            // by position along the axis, then by id: the same drawing always gives the same order
            let lo = |it: &Item| if h { it.b.0 } else { it.b.1 };
            let hi = |it: &Item| if h { it.b.2 } else { it.b.3 };
            its.sort_by(|a, b| {
                lo(a)
                    .partial_cmp(&lo(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.key.cmp(&b.key))
            });
            let n = its.len();
            let end = its.iter().map(hi).fold(f64::MIN, f64::max);
            let span = end - lo(&its[0]);
            let size: f64 = its.iter().map(|it| hi(it) - lo(it)).sum();
            let gap = (span - size) / (n - 1) as f64;
            // the first item stays; each next one starts a gap after the end of the one before it
            let mut at = lo(&its[0]);
            let mut ops = Vec::new();
            for it in &its {
                let d = at - lo(it);
                ops.extend(if h { mv(it, d, 0.0) } else { mv(it, 0.0, d) });
                at += hi(it) - lo(it) + gap;
            }
            Ok(ops)
        }
    }
}

/// The union of the boxes of some items.
fn union(its: &[Item]) -> (f64, f64, f64, f64) {
    its.iter()
        .fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |u, it| {
            (
                u.0.min(it.b.0),
                u.1.min(it.b.1),
                u.2.max(it.b.2),
                u.3.max(it.b.3),
            )
        })
}

/// The centre of the box of some atoms, on the document grid (the point a turn leaves where it is).
fn centre(doc: &Document, atoms: &[u32]) -> Result<(f64, f64), Refusal> {
    let mut b = None;
    for &i in atoms {
        let a = doc.atom(i).ok_or("error.draw2.noAtom")?;
        grow(&mut b, a.x, a.y);
    }
    let (x0, y0, x1, y1) = b.ok_or("error.draw2.arrangeNone")?;
    Ok((round4((x0 + x1) / 2.0), round4((y0 + y1) / 2.0)))
}

use crate::editor::{Editor, CHANGED, REPAINT};

impl Editor {
    /// Runs the arrange command `name` (`align:left`, `rotate:45`, ...) on the selection, or on everything when nothing is selected,
    /// as one undo step (the arrows anchored to what moves follow). A refusal is in `state().error`; a drawing that is arranged
    /// already is not changed and says so.
    pub fn arrange(&mut self, name: &str) -> u32 {
        let (a, o) = (self.over.sel_atoms.clone(), self.over.sel_objs.clone());
        match self.arrange_on(name, &a, &o) {
            Ok(()) => REPAINT | CHANGED,
            Err(e) => {
                self.error = e.key;
                REPAINT
            }
        }
    }

    /// `arrange` on named atoms and objects (the command of a request). Nothing is selected and nothing named: everything.
    pub(crate) fn arrange_on(
        &mut self,
        name: &str,
        atoms: &[u32],
        objects: &[u32],
    ) -> Result<(), crate::ops::OpError> {
        let bad = |key| crate::ops::OpError { key, id: 0 };
        let cmd = parse(name).ok_or(bad("error.draw2.command"))?;
        let ops = plan(self.doc(), cmd, atoms, objects).map_err(bad)?;
        if ops.is_empty() {
            self.said = ("draw2.say.arrangedSame", String::new());
            self.error = "";
            return Ok(());
        }
        let say = match cmd {
            Cmd::Align(_) => "draw2.say.aligned",
            Cmd::Distribute(_) => "draw2.say.distributed",
            Cmd::Rotate(_) => "draw2.say.rotated",
            Cmd::Nudge(..) => "draw2.say.nudged",
        };
        let what = name.split_once(':').map_or("", |p| p.1).to_string();
        let guard = crate::stereo_guard::before(self.doc(), &ops);
        let ops = self.following(&ops);
        self.session.run(&ops)?;
        self.said = (say, what);
        self.say_stereo(guard);
        self.error = "";
        self.prune();
        Ok(())
    }
}
