//! Commands on the selection: rotate and flip (the whole drawing when nothing is selected), and the clipboard (copy, cut, paste,
//! duplicate). The clipboard holds an `mzdraw/1` document; the page mirrors it to the system clipboard.

use crate::doc::Document;
use crate::editor::{Editor, CHANGED, REPAINT, TOOL};
use crate::io;
use crate::ops::{Axis, Op};

/// How far a pasted copy lands from its source, in bond lengths, per paste.
const PASTE_STEP: f64 = 0.5;

impl Editor {
    /// The selected atoms, or every atom when nothing is selected.
    fn target(&self) -> Vec<u32> {
        if self.over.sel_atoms.is_empty() {
            self.doc().atoms.iter().map(|a| a.id).collect()
        } else {
            self.over.sel_atoms.clone()
        }
    }

    /// The centre of the bounding box of some atoms.
    pub(crate) fn centre(&self, atoms: &[u32]) -> (f64, f64) {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for a in atoms.iter().filter_map(|&i| self.doc().atom(i)) {
            (x0, y0, x1, y1) = (x0.min(a.x), y0.min(a.y), x1.max(a.x), y1.max(a.y));
        }
        ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
    }

    /// Sets the explicit colour of the selected atoms and bonds (the bonds between selected atoms count), or of the atom with the
    /// keyboard focus when nothing is selected, as one undo step. `col` is `#rgb` / `#rrggbb`, or empty for automatic (the element
    /// colour, or the mono ink). A colour that is not valid is `error.draw2.colour`; nothing to colour is `error.draw2.colourNone`;
    /// a selection that has the colour already changes nothing.
    pub fn colour(&mut self, col: &str) -> u32 {
        let Some(col) = crate::ink::canon(col) else {
            self.error = "error.draw2.colour";
            return REPAINT;
        };
        let d = self.doc();
        let mut atoms = self.over.sel_atoms.clone();
        if atoms.is_empty() && self.over.sel_bonds.is_empty() && self.over.sel_objs.is_empty() {
            atoms.extend(self.over.focus);
        }
        let mut objects = self.over.sel_objs.clone();
        let mut bonds = self.over.sel_bonds.clone();
        bonds.extend(
            (d.bonds.iter())
                .filter(|b| atoms.contains(&b.a) && atoms.contains(&b.b))
                .map(|b| b.id),
        );
        atoms.retain(|&i| d.atom(i).is_some_and(|a| a.col() != col));
        bonds.retain(|&i| d.bond(i).is_some_and(|b| b.col() != col));
        objects.retain(|&i| d.object(i).is_some_and(|o| o.col() != col));
        bonds.sort_unstable();
        bonds.dedup();
        if atoms.is_empty() && bonds.is_empty() && objects.is_empty() {
            let any = !self.over.sel_atoms.is_empty()
                || !self.over.sel_bonds.is_empty()
                || !self.over.sel_objs.is_empty()
                || self.over.focus.is_some();
            self.error = if any { "" } else { "error.draw2.colourNone" };
            return REPAINT;
        }
        let what = if col.is_empty() {
            "automatic".to_string()
        } else {
            col.clone()
        };
        self.run(
            &[Op::Colour {
                atoms,
                bonds,
                objects,
                col,
            }],
            "draw2.say.coloured",
            &what,
        )
    }

    /// The colour the selected atoms and bonds (or the focused atom) have in common: `Some("")` when they are all automatic,
    /// `None` when nothing is selected or the colours differ.
    pub fn selection_colour(&self) -> Option<String> {
        let d = self.doc();
        let o = &self.over;
        let mut cols: Vec<String> = (d.atoms.iter())
            .filter(|a| o.sel_atoms.contains(&a.id))
            .map(|a| a.col())
            .chain(
                (d.bonds.iter())
                    .filter(|b| o.sel_bonds.contains(&b.id))
                    .map(|b| b.col()),
            )
            .chain(
                (d.objects.iter())
                    .filter(|x| o.sel_objs.contains(&x.id))
                    .map(|x| x.col()),
            )
            .collect();
        if cols.is_empty() {
            cols.extend(o.focus.and_then(|f| d.atom(f)).map(|a| a.col()));
        }
        let first = cols.first()?.clone();
        cols.iter().all(|c| *c == first).then_some(first)
    }

    /// Turns the selection (or everything) by `deg` degrees counter-clockwise around its centre.
    pub fn rotate(&mut self, deg: f64) -> u32 {
        let atoms = self.target();
        if atoms.is_empty() {
            return 0;
        }
        let (x, y) = self.centre(&atoms);
        self.run(
            &[Op::Rotate { atoms, deg, x, y }],
            "draw2.say.rotated",
            &crate::svg::num(deg as f32),
        )
    }

    /// Mirrors the selection (or everything) across the vertical (`Axis::H`) or horizontal (`Axis::V`) line through its centre.
    pub fn flip(&mut self, axis: Axis) -> u32 {
        let atoms = self.target();
        if atoms.is_empty() {
            return 0;
        }
        let (x, y) = self.centre(&atoms);
        let at = if axis == Axis::H { x } else { y };
        self.run(&[Op::Flip { atoms, axis, at }], "draw2.say.flipped", "")
    }

    /// Lays the structures, arrows, plus signs and texts of the selection (or of the drawing) out on one row, one undo step
    /// (`scheme::row`, the command `scheme:row` with its default `gap` and no `wrap`; what the page's button runs).
    pub fn scheme_row(&mut self) -> u32 {
        self.scheme_row_with(1.0, 0.0)
    }

    /// The same with a `gap` and a `wrap` (bond lengths). When everything stands where the row puts it, nothing changes and the
    /// status line says so.
    pub fn scheme_row_with(&mut self, gap: f64, wrap: f64) -> u32 {
        let opts = crate::scheme::Opts {
            gap,
            wrap,
            atoms: self.over.sel_atoms.clone(),
            objects: self.over.sel_objs.clone(),
        };
        match crate::scheme::row(self.doc(), &self.style, &opts) {
            Ok(ops) if ops.is_empty() => {
                self.said = ("draw2.say.schemeRowDone", String::new());
                self.error = "";
                REPAINT
            }
            Ok(ops) => self.run(&ops, "draw2.say.schemeRow", ""),
            Err(key) => {
                self.error = key;
                REPAINT
            }
        }
    }

    /// Arranges the whole pathway of anchored arrows as a tree, one undo step: the command `pathway:tree` (`pathway::command`; what
    /// the page's button "Arrange pathway" and the item `pathway.tree` of the context menu run). The line says what it did, in
    /// numbers kept in `said_params` (structures, arrows, gaps widened); a refusal is kept in `error` and nothing changes.
    pub fn pathway_tree(&mut self) -> u32 {
        let lay = match crate::pathway::layout(self.doc(), &self.style) {
            Ok(l) => l,
            Err(r) => {
                self.error = r.key();
                return REPAINT;
            }
        };
        let counts = (lay.nodes.len(), lay.edges.len(), lay.gaps.len());
        match crate::pathway::ops(self.doc(), &self.style, &lay) {
            // the operations already carry the routes of the arrows: `run` would add the follow of `route` a second time, and the log
            // would differ from the log of the command `pathway:tree`
            Ok(ops) => match self.session.run(&ops) {
                Ok(()) => {
                    self.said = ("draw2.say.pathway", String::new());
                    self.said_params = vec![
                        ("structures", counts.0 as u32),
                        ("arrows", counts.1 as u32),
                        ("gaps", counts.2 as u32),
                    ];
                    self.error = "";
                    self.prune();
                    REPAINT | CHANGED
                }
                Err(e) => {
                    self.error = e.key;
                    REPAINT
                }
            },
            Err(r) => {
                self.error = r.key();
                REPAINT
            }
        }
    }

    /// The selected atoms, the bonds between them and the selected objects as a document of their own, ids renumbered from 1
    /// (atoms, then bonds, then objects).
    pub(crate) fn selection_doc(&self) -> Document {
        let d = self.doc();
        let sel = &self.over.sel_atoms;
        let mut out = Document::default();
        let mut ids: Vec<(u32, u32)> = Vec::new();
        for a in d.atoms.iter().filter(|a| sel.contains(&a.id)) {
            let id = out.next;
            out.next += 1;
            ids.push((a.id, id));
            out.atoms.push(crate::doc::Atom { id, ..a.clone() });
        }
        let new = |old: u32| ids.iter().find(|p| p.0 == old).map(|p| p.1);
        for b in &d.bonds {
            if let (Some(a2), Some(b2)) = (new(b.a), new(b.b)) {
                let id = out.next;
                out.next += 1;
                out.bonds.push(crate::doc::Bond {
                    id,
                    a: a2,
                    b: b2,
                    ..b.clone()
                });
            }
        }
        for o in (d.objects.iter()).filter(|o| self.over.sel_objs.contains(&o.id)) {
            let id = out.next;
            out.next += 1;
            out.objects.push(crate::doc::Obj {
                id,
                from: 0,
                to: 0,
                ..o.clone()
            });
        }
        out
    }

    /// Copies the selection to the editor's clipboard; false when nothing is selected.
    pub fn copy(&mut self) -> bool {
        let d = self.selection_doc();
        if d.is_empty() {
            return false;
        }
        self.clip = d.to_json();
        self.pasted = 0;
        true
    }

    /// The clipboard as `mzdraw` (the document) or `molfile` (for other programs); empty when nothing was copied.
    pub fn clipboard(&self, kind: &str) -> String {
        match (kind, Document::from_json(&self.clip)) {
            ("molfile", Ok(d)) => d.to_molfile(),
            (k, Ok(_)) if crate::brand::is_doc_kind(k) => self.clip.clone(),
            _ => String::new(),
        }
    }

    /// Copies the selection, then removes it (one undo step).
    pub fn cut(&mut self) -> u32 {
        if !self.copy() {
            return 0;
        }
        self.delete_selection()
    }

    /// `scheme:orient` from the page: each structure after an arrow is turned onto the one before it, in one undo step. A refusal
    /// (no common core, too large, would overlap) is kept in `error` and changes nothing; a scheme that is oriented already is said and not changed.
    pub fn scheme_orient(&mut self) -> u32 {
        match crate::commands::run(self, "scheme:orient", &serde_json::json!({})) {
            Ok(()) => {
                self.error = "";
                let turned =
                    (self.orient_steps.iter()).any(|s| s.status == crate::orient::Status::Oriented);
                if turned {
                    REPAINT | CHANGED
                } else {
                    REPAINT
                }
            }
            Err(e) => {
                self.error = e.key;
                REPAINT
            }
        }
    }

    /// Pastes a structure text (`None`: the editor's own clipboard) as one undo step. What comes from this editor lands next to its
    /// source, half a bond further at each paste; a text from elsewhere lands to the right of the drawing. The pasted part becomes
    /// the selection and the tool becomes Select, so that it can be dragged into place.
    pub fn paste(&mut self, text: Option<&str>) -> u32 {
        let own = text.is_none_or(|t| t == self.clip);
        let text = text.map_or(self.clip.clone(), str::to_string);
        if text.trim().is_empty() {
            return 0;
        }
        let placed = if own {
            let step = PASTE_STEP * (self.pasted + 1) as f64;
            io::import(&text).and_then(|(new, _)| self.place(&new, step, -step))
        } else {
            self.import_placed(&text).map(|r| r.1)
        };
        match placed {
            Ok(p) => {
                if own {
                    self.pasted += 1;
                }
                self.said = (
                    "draw2.say.pasted",
                    (p.atoms.len() + p.objects.len()).to_string(),
                );
                self.error = "";
                let o = &mut self.over;
                (o.sel_atoms, o.sel_bonds, o.sel_objs) = (p.atoms, p.bonds, p.objects);
                self.set_tool("select") | REPAINT | CHANGED | TOOL
            }
            Err(key) => {
                self.error = key;
                REPAINT
            }
        }
    }

    /// Copies the selection and pastes it at once.
    pub fn duplicate(&mut self) -> u32 {
        if !self.copy() {
            return 0;
        }
        self.paste(None)
    }
}
