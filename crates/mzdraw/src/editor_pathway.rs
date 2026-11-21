//! The pathway commands of the editor: `pathway:anchor` (the ends of one arrow to the nearest structures), `pathway:route` (every
//! anchored arrow back on its routed ends) and the freeing of the ends. The page runs them from the context menu; the replay
//! request runs the same commands (`commands.rs`), so the log carries plain operations (`obj.set`, `move`).

use crate::doc::ObjKind;
use crate::editor::{Editor, CHANGED, REPAINT};
use crate::ops::{Op, OpError};
use crate::route;

/// How near (bond lengths, by distance to the box of a structure) an end of an arrow must be to be anchored by `pathway:anchor`.
pub const REACH: f64 = 1.0;

impl Editor {
    fn arrow_check(&self, id: u32) -> Result<(), OpError> {
        match self.doc().object(id) {
            None => Err(OpError {
                key: "error.draw2.noObject",
                id,
            }),
            Some(o) if o.t != ObjKind::Arrow => Err(OpError {
                key: "error.draw2.object",
                id,
            }),
            Some(_) => Ok(()),
        }
    }

    /// `pathway:anchor`: anchors each end of the arrow `id` to the nearest structure within one bond length, routes it, one undo
    /// step. An end with nothing in reach is reported in the live line and left as it was.
    pub(crate) fn pathway_anchor(&mut self, id: u32) -> Result<bool, OpError> {
        self.arrow_check(id)?;
        let (ops, now) = route::anchor(self.doc(), id, REACH, &self.style).unwrap_or_default();
        let any = now != [0, 0];
        let changed = !ops.is_empty();
        if changed {
            self.session.run(&ops)?;
        }
        self.error = "";
        self.said = match (now[0] != 0, now[1] != 0) {
            (true, true) if !changed => (
                "draw2.say.anchorSame",
                format!(
                    "{} \u{2192} {}",
                    route::formula_of(self.doc(), now[0]),
                    route::formula_of(self.doc(), now[1])
                ),
            ),
            (true, true) => (
                "draw2.say.anchored",
                format!(
                    "{} \u{2192} {}",
                    route::formula_of(self.doc(), now[0]),
                    route::formula_of(self.doc(), now[1])
                ),
            ),
            _ if any => (
                "draw2.say.anchorOne",
                route::formula_of(self.doc(), now[0].max(now[1])),
            ),
            _ => ("draw2.say.anchorNone", String::new()),
        };
        self.prune();
        Ok(changed)
    }

    /// `pathway:route`: routes every anchored arrow in one undo step; no step when nothing moves.
    pub(crate) fn pathway_route(&mut self) -> Result<bool, OpError> {
        let ops = route::route_all(self.doc(), &self.style);
        let n = (ops.iter())
            .filter_map(|o| match o {
                Op::ObjSet { id, .. } => Some(*id),
                Op::Move { objects, .. } => objects.first().copied(),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let changed = !ops.is_empty();
        if changed {
            self.session.run(&ops)?;
        }
        self.error = "";
        self.said = if n == 0 {
            ("draw2.say.routeNone", String::new())
        } else {
            ("draw2.say.routed", n.to_string())
        };
        self.prune();
        Ok(changed)
    }

    /// Frees both ends of the arrow `id` (`obj.set` with `from` and `to` 0), one undo step; none when it has no anchor.
    pub(crate) fn pathway_free(&mut self, id: u32) -> Result<bool, OpError> {
        self.arrow_check(id)?;
        let changed = (self.doc().object(id)).is_some_and(|o| o.from != 0 || o.to != 0);
        if changed {
            self.session.run(&[Op::ObjSet {
                id,
                text: None,
                x2: None,
                y2: None,
                from: Some(0),
                to: Some(0),
            }])?;
        }
        self.error = "";
        self.said = if changed {
            ("draw2.say.freed", String::new())
        } else {
            ("draw2.say.freeNone", String::new())
        };
        Ok(changed)
    }

    /// Flags of a pathway command run from the page.
    pub(crate) fn pathway_flags(&mut self, r: Result<bool, OpError>) -> u32 {
        match r {
            Ok(true) => REPAINT | CHANGED,
            Ok(false) => REPAINT,
            Err(e) => {
                self.error = e.key;
                REPAINT
            }
        }
    }
}
