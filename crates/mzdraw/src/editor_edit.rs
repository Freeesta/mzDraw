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
    fn centre(&self, atoms: &[u32]) -> (f64, f64) {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for a in atoms.iter().filter_map(|&i| self.doc().atom(i)) {
            (x0, y0, x1, y1) = (x0.min(a.x), y0.min(a.y), x1.max(a.x), y1.max(a.y));
        }
        ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
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
            out.objects.push(crate::doc::Obj { id, ..o.clone() });
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
            ("mzdraw", Ok(_)) => self.clip.clone(),
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
