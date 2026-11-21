//! The lines kept under a molecule (struct-props): choosing them, and copying the values as text. The numbers are those of `values`
//! and `props`; this file only edits the choice (the operation `atom.cap`) and asks for the answers.

use crate::editor::{Editor, REPAINT};
use crate::ops::Op;
use crate::props::LINES;

impl Editor {
    /// The first atom (pool order) of every molecule that has an atom among `ids`, with the choice it follows now: the choice kept on
    /// one of its atoms, or empty.
    fn molecules_of(&self, ids: &[u32]) -> Vec<(Vec<u32>, Vec<String>)> {
        let sc = crate::values::scan(self.doc());
        (sc.structures.iter())
            .filter(|s| ids.iter().any(|i| s.ids.contains(i)))
            .map(|s| (s.ids.clone(), s.cap.clone()))
            .collect()
    }

    /// The molecules the choice of lines is for: those of the selected atoms, else the one of the atom with the keyboard focus.
    fn caption_target(&self) -> Vec<(Vec<u32>, Vec<String>)> {
        let mut ids = self.over.sel_atoms.clone();
        if ids.is_empty() {
            ids.extend(self.over.focus);
        }
        self.molecules_of(&ids)
    }

    /// What a molecule with no choice writes under it now, as keys (so that a first click on a line adds to what is shown).
    fn shown_keys(&self) -> Vec<String> {
        use crate::values::Mode;
        match self.style.values {
            Mode::Off => Vec::new(),
            Mode::Formula => vec!["formula".into()],
            Mode::Mass => vec!["formula".into(), "exact".into()],
            Mode::Mz => vec!["mz".into()],
        }
    }

    /// The lines kept under the molecule of the selection (or the keyboard focus): its choice, or what `display.values` writes when it
    /// has none; `None` when there is no molecule or the molecules differ.
    pub fn selection_lines(&self) -> Option<Vec<String>> {
        let target = self.caption_target();
        let first = target.first()?;
        let of = |c: &Vec<String>| {
            if c.is_empty() {
                self.shown_keys()
            } else {
                c.clone()
            }
        };
        let keys = of(&first.1);
        target.iter().all(|t| of(&t.1) == keys).then_some(keys)
    }

    /// Sets the lines kept under each molecule of the selection (or the focused atom's): one undo step. The empty list gives the
    /// choice back to `display.values`. A list that is not valid is `error.draw2.lines`; no molecule is `error.draw2.linesNone`.
    pub fn set_lines(&mut self, keys: &[String]) -> u32 {
        let Some(keys) = crate::props::canon(keys) else {
            self.error = "error.draw2.lines";
            return REPAINT;
        };
        let target = self.caption_target();
        if target.is_empty() {
            self.error = "error.draw2.linesNone";
            return REPAINT;
        }
        let mut ops: Vec<Op> = Vec::new();
        for (ids, cap) in &target {
            if *cap == keys {
                continue;
            }
            // the choice lives on the first atom of the molecule; the others lose theirs (as the annotation does)
            for (k, id) in ids.iter().enumerate() {
                let held = self.doc().atom(*id).map(|a| a.cap()).unwrap_or_default();
                if k == 0 {
                    if held != keys {
                        ops.push(Op::AtomCap {
                            id: *id,
                            lines: keys.clone(),
                        });
                    }
                } else if !held.is_empty() {
                    ops.push(Op::AtomCap {
                        id: *id,
                        lines: Vec::new(),
                    });
                }
            }
        }
        if ops.is_empty() {
            self.error = "";
            return REPAINT;
        }
        self.run(&ops, "draw2.say.lines", &keys.join(","))
    }

    /// `set_lines` from a JSON list of texts; a text that is not one is `error.draw2.lines`.
    pub fn set_lines_json(&mut self, keys: &str) -> u32 {
        match serde_json::from_str::<Vec<String>>(keys) {
            Ok(k) => self.set_lines(&k),
            Err(_) => {
                self.error = "error.draw2.lines";
                REPAINT
            }
        }
    }

    /// Adds or takes away one line key in the choice of the selection's molecule (the first one decides what it was), keeping the
    /// order of `props::LINES`. A key that is not one is `error.draw2.lines`.
    pub fn toggle_line(&mut self, key: &str, on: bool) -> u32 {
        if !LINES.contains(&key) {
            self.error = "error.draw2.lines";
            return REPAINT;
        }
        let Some(mut keys) = self.selection_lines() else {
            let none = self.caption_target().is_empty();
            self.error = if none {
                "error.draw2.linesNone"
            } else {
                "error.draw2.linesMixed"
            };
            return REPAINT;
        };
        keys.retain(|k| k != key);
        if on {
            keys.push(key.to_string());
        }
        let rank = |k: &String| LINES.iter().position(|l| l == k).unwrap_or(LINES.len());
        keys.sort_by_key(rank);
        self.set_lines(&keys)
    }

    /// The results the editor holds for the structures (`refresh_values`), for tests and tools that draw with them.
    pub fn measured_entries(&self) -> &[crate::values::Entry] {
        &self.measured
    }

    /// The `props/1` of the page: the values of every structure and of the selection, as the editor holds them.
    pub fn props_json(&self) -> String {
        let sel = self
            .measured_sel
            .as_ref()
            .map(|(ids, _, r)| (ids.as_slice(), Some(r)));
        crate::values::props_json(
            self.doc(),
            &self.style.adducts,
            self.measurer.is_some(),
            self.measured_error,
            &self.measured,
            sel,
        )
    }

    /// The values copied as text (`values::FORMS`): of the selection when there is one, else of every molecule; the lines are each
    /// molecule's choice, or all of them. `error.draw2.propsCopy` for a form that is none or nothing to copy.
    pub fn copy_values(&self, form: &str) -> Result<String, &'static str> {
        let sel = self
            .measured_sel
            .as_ref()
            .map(|(ids, _, r)| (ids.as_slice(), Some(r)));
        crate::values::props_text(
            form,
            self.doc(),
            &self.style.adducts,
            &[],
            &self.measured,
            sel,
        )
        .ok_or("error.draw2.propsCopy")
    }
}
