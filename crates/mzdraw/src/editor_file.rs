//! The unsaved-changes guard: whether the drawing holds work that was not saved, and the actions that would lose it (a new drawing,
//! opening another one in its place). The decision is made here; the page only shows the question and forwards the answer.
//!
//! The saved point is a mark on the history (`Session::dirty`): saving, exporting, opening and making a new drawing set it, any edit
//! clears it, and undoing back to it makes the drawing clean again. A drawing is *unsaved* when it is dirty and holds something: an
//! empty canvas has nothing to lose, so it never asks.

use crate::editor::Editor;

/// What the page asks of the user before an action that replaces the drawing, as `Editor::guard` says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guard {
    /// Nothing would be lost: do the action.
    Go,
    /// Unsaved work would be lost: ask "Save changes?".
    Ask,
}

/// The answers to the question, as the page forwards them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Save,
    Discard,
    Cancel,
}

impl Choice {
    /// `save`, `discard` (the page says "Don't save") or `cancel`.
    pub fn parse(name: &str) -> Option<Choice> {
        match name {
            "save" => Some(Choice::Save),
            "discard" => Some(Choice::Discard),
            "cancel" => Some(Choice::Cancel),
            _ => None,
        }
    }

    /// What the page does for this answer: `save` (write the file, then ask the editor to go on), `go` (do the action, forgetting
    /// the unsaved work) or `stop` (do nothing).
    pub fn does(self) -> &'static str {
        match self {
            Choice::Save => "save",
            Choice::Discard => "go",
            Choice::Cancel => "stop",
        }
    }
}

impl Editor {
    /// Whether the drawing differs from the last saved, exported, opened or new state.
    pub fn dirty(&self) -> bool {
        self.session.dirty()
    }

    /// Whether the drawing is dirty and holds something that a new or an opened drawing would lose.
    pub fn unsaved(&self) -> bool {
        self.dirty() && !self.doc().is_empty()
    }

    /// Whether to ask before an action that replaces the drawing.
    pub fn guard(&self) -> Guard {
        if self.unsaved() {
            Guard::Ask
        } else {
            Guard::Go
        }
    }

    /// The drawing as it is now is the saved point (opened, made new, or the native document written).
    pub fn mark_saved(&mut self) {
        self.session.mark_saved();
    }

    /// The file kind the question's Save writes: the native document, which holds everything.
    pub fn save_kind() -> &'static str {
        crate::brand::SLUG
    }

    /// A file of this kind (the names of `save`) was written. Only the native document is loss-free, so only it is the saved point;
    /// a molfile, SMILES, SVG, PNG, PDF or any other export leaves the drawing dirty. Returns whether the point was set.
    pub fn exported(&mut self, kind: &str) -> bool {
        let native = crate::brand::is_doc_kind(kind);
        if native {
            self.mark_saved();
        }
        native
    }

    /// A new, empty drawing, as one undo step (Undo brings the old one back); the new state is the saved point. Unsaved work is
    /// refused (`error.draw2.unsaved`, nothing changes) unless `discard` says it may go. The flags are those of `clear`.
    pub fn new_drawing(&mut self, discard: bool) -> Result<u32, &'static str> {
        if !discard && self.unsaved() {
            return Err("error.draw2.unsaved");
        }
        let flags = self.clear();
        self.mark_saved();
        Ok(flags)
    }

    /// Opens a saved `mzdraw/1` document in place of the drawing (`Editor::open`), refusing unsaved work unless `discard`.
    pub fn open_drawing(&mut self, text: &str, discard: bool) -> Result<(), &'static str> {
        if !discard && self.unsaved() {
            return Err("error.draw2.unsaved");
        }
        self.open(text)
    }
}
