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
        let (ops, now) = route::anchor(self.doc(), id, REACH, &self.style)?;
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

    /// `pathway:label`: writes the computed label (`steplabel::plan`, the mode `pathway.label`) on every anchored arrow whose text is
    /// empty or its own, one undo step; none when nothing changes. The counts for the live line are kept in `said_params`.
    pub(crate) fn pathway_label(&mut self, opts: &crate::steplabel::Opts) -> Result<bool, OpError> {
        let plan = crate::steplabel::plan(self.doc(), self.style.pathway_label, opts)?;
        let changed = !plan.ops.is_empty();
        if changed {
            self.session.run(&plan.ops)?;
        }
        self.error = "";
        self.said = if plan.labelled + plan.kept + plan.skipped == 0 {
            ("draw2.say.labelNone", String::new())
        } else {
            ("draw2.say.labelled", String::new())
        };
        self.said_params = vec![
            ("labelled", plan.labelled as u32),
            ("kept", plan.kept as u32),
            ("skipped", plan.skipped as u32),
        ];
        self.prune();
        Ok(changed)
    }

    /// The page's button "Label arrows": `pathway:label` with the isomer word of the page's language (empty: the default word).
    pub fn pathway_label_page(&mut self, isomer: &str) -> u32 {
        let mut opts = crate::steplabel::Opts::default();
        if !isomer.is_empty() {
            let json = serde_json::json!({ "isomer": isomer });
            match crate::steplabel::Opts::parse(&json) {
                Ok(o) => opts = o,
                Err(key) => {
                    self.error = key;
                    return crate::editor::REPAINT;
                }
            }
        }
        let r = self.pathway_label(&opts);
        self.pathway_flags(r)
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

impl Editor {
    /// `tp:derive` from the page (the menu item "Derive TP" and Shift+D): the structure of `atom`, or of the focused atom, or of the
    /// first selected atom, is copied with its maps and an arrow to the copy (`tp::derive` with no patch), one undo step. The copy
    /// becomes the selection and the tool Select, so the next edit applies to it.
    pub fn tp_derive(&mut self, atom: Option<u32>) -> u32 {
        if !self.style.pathway_derive {
            return 0;
        }
        let from = atom
            .or(self.over.focus)
            .or_else(|| self.over.sel_atoms.iter().copied().min());
        let Some(of) = from else {
            self.error = "error.draw2.tpNone";
            return REPAINT;
        };
        let first = self.doc().next;
        let ops = match crate::tp::derive(self.doc(), &self.style, of, &[], None) {
            Ok(ops) => ops,
            Err(key) => {
                self.error = key;
                return REPAINT;
            }
        };
        self.apply_derive(first, &ops)
    }

    /// Runs the operations of a derive (`tp::derive`, `tp::derive_step`) as one undo step; the copy (every atom and bond from id
    /// `first` on) becomes the selection and the tool is Select.
    pub(crate) fn apply_derive(&mut self, first: u32, ops: &[Op]) -> u32 {
        if let Err(e) = self.run_guarded(ops) {
            self.error = e.key;
            return REPAINT;
        }
        let d = self.doc();
        let atoms: Vec<u32> = (d.atoms.iter())
            .map(|a| a.id)
            .filter(|&i| i >= first)
            .collect();
        let bonds: Vec<u32> = (d.bonds.iter())
            .map(|b| b.id)
            .filter(|&i| i >= first)
            .collect();
        let what = atoms
            .first()
            .map_or(String::new(), |&a| route::formula_of(d, a));
        let o = &mut self.over;
        (o.sel_atoms, o.sel_bonds, o.sel_objs) = (atoms, bonds, Vec::new());
        self.said = ("draw2.say.derived", what);
        self.error = "";
        self.set_tool("select") | REPAINT | CHANGED | crate::editor::TOOL
    }

    /// Shift+L: the diff lens goes to the next mode (`off`, `tint`, `tint+ghost`, `off`...). A display setting like `display.lens`:
    /// not a step of the log, not in the document; said in the live line.
    pub fn cycle_lens(&mut self) -> u32 {
        use crate::settings::LensMode::{Off, Tint, TintGhost};
        self.style.lens = match self.style.lens {
            Off => Tint,
            Tint => TintGhost,
            TintGhost => Off,
        };
        self.said = ("draw2.say.lens", self.style.lens.name().to_string());
        self.error = "";
        REPAINT
    }
}
