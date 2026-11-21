//! The step field (tp-11): a one-line field that lists the table of tp-10 (`tp::STEPS`), filtered as the person types; the sites of
//! the chosen step ringed on the canvas, the focused one with a ghost of what the step adds. Everything is decided here: the filter
//! (`tp::filter`), the sites (`tp::sites`), the keys while the field is open (`[` and `]` move between sites, `Enter` derives, `Escape`
//! closes), the hit-test of a tap on a ring, the marks and the live line. The marks are an overlay of the view: not in the document,
//! no operation until `Enter` (or the page's Derive button), which runs `tp::derive_step` as one undo step, as `tp:derive` does with
//! `step` and `site` in the log of `draw_edit/1`. The page only paints `step_json` and forwards keys, text and taps.
//!
//! The field follows the drawing: after a change of the history the sites are found again; it closes only when its atom is gone.

use crate::editor::{Editor, HANDLED, MIDDLE, REPAINT, SECONDARY, STEP};
use crate::editor_touch::{DOWN, MOVE};
use crate::scene::{colour, Mark};
use crate::tp::{self, steps, steps::Kind, Step};
use std::collections::BTreeMap;

/// The state of an open field.
pub(crate) struct StepField {
    /// What was typed.
    pub text: String,
    /// An atom of the structure the step is applied to.
    pub of: u32,
    /// The monoisotopic mass of each element, from the engine (`values::element_masses`), when there is one: the filter then also
    /// matches a mass shift.
    masses: Option<BTreeMap<String, f64>>,
    /// The rows the text selects, in table order, and the chosen one.
    pub matches: Vec<&'static Step>,
    pub pick: usize,
    /// The sites of the chosen row (ids, ascending) and the focused one.
    pub sites: Vec<u32>,
    pub site: usize,
    /// The refusal of the sites of the chosen row (an i18n key), empty when none.
    error: &'static str,
    /// The overlay: rings on the sites, the focused one doubled, and what the chosen step adds or removes there.
    marks: Vec<Mark>,
    /// The history the field was computed for.
    generation: u64,
    /// The pointer whose press chose a site: its move and release are swallowed.
    eat: Option<u32>,
}

impl Editor {
    /// Opens the field on the structure of `atom`, or of the focused atom, or of the first selected atom (the item "Apply step..."
    /// of the context menu, which the Menu key and Shift+F10 open too: no key of its own, the kill test of tp-11 did not pass,
    /// `tools/velocity/step_field.mjs`). Like "Derive TP" it is part of the pathway features (`style.pathway_derive`).
    pub fn step_open(&mut self, atom: Option<u32>) -> u32 {
        if !self.style.pathway_derive {
            return 0;
        }
        let from = atom
            .or(self.over.focus)
            .or_else(|| self.over.sel_atoms.iter().copied().min());
        let Some(of) = from.filter(|&a| self.doc().atom(a).is_some()) else {
            self.error = "error.draw2.tpNone";
            return REPAINT;
        };
        // seven engine calls the first time, then kept: the masses are the engine's, so they do not change in a session
        if self.step_masses.is_none() {
            self.step_masses = self
                .measurer
                .and_then(crate::values::element_masses)
                .filter(|m| !m.is_empty());
        }
        let masses = self.step_masses.clone();
        self.error = "";
        self.step_field = Some(StepField {
            text: String::new(),
            of,
            masses,
            matches: Vec::new(),
            pick: 0,
            sites: Vec::new(),
            site: 0,
            error: "",
            marks: Vec::new(),
            generation: self.session.generation(),
            eat: None,
        });
        self.step_refilter();
        REPAINT | STEP
    }

    /// Closes the field with nothing done.
    pub fn step_close(&mut self) -> u32 {
        match self.step_field.take() {
            Some(_) => REPAINT | STEP,
            None => 0,
        }
    }

    /// The field follows the drawing: after a change of the history (an edit, a turn of the selection, an undo) the sites of the chosen
    /// row are found again on the drawing as it is now (the focused site stays where it still is), and the field closes only when the
    /// atom it was opened on is gone.
    fn step_sync(&mut self) -> bool {
        let Some(f) = self.step_field.as_ref() else {
            return false;
        };
        if f.generation == self.session.generation() {
            return true;
        }
        if self.doc().atom(f.of).is_none() {
            self.step_field = None;
            return false;
        }
        let (pick, site) = (f.pick, f.sites.get(f.site).copied());
        if let Some(f) = self.step_field.as_mut() {
            f.generation = self.session.generation();
        }
        self.step_sites();
        if let Some(f) = self.step_field.as_mut() {
            f.pick = pick.min(f.matches.len().saturating_sub(1));
        }
        if let Some(i) = site.and_then(|s| {
            self.step_field
                .as_ref()
                .and_then(|f| f.sites.iter().position(|&x| x == s))
        }) && let Some(f) = self.step_field.as_mut()
        {
            f.site = i;
        }
        self.step_marks_build();
        true
    }

    /// After any call: the field is brought up to date with the drawing (`step_sync`); the flags say so when it changed or closed.
    pub(crate) fn step_follow(&mut self) -> u32 {
        match self.step_field.as_ref() {
            Some(f) if f.generation != self.session.generation() => {
                self.step_sync();
                STEP | REPAINT
            }
            _ => 0,
        }
    }

    /// The text of the field changed (the page forwards what was typed): the rows are filtered again, the first is chosen.
    pub fn step_text(&mut self, text: &str) -> u32 {
        if !self.step_sync() {
            return STEP | REPAINT;
        }
        if let Some(f) = self.step_field.as_mut() {
            f.text = text.chars().take(64).collect();
            f.pick = 0;
        }
        self.step_refilter();
        REPAINT | STEP
    }

    /// Chooses the row `i` of the list (a tap on a row).
    pub fn step_pick(&mut self, i: usize) -> u32 {
        if !self.step_sync() {
            return STEP | REPAINT;
        }
        if let Some(f) = self.step_field.as_mut()
            && i < f.matches.len()
        {
            f.pick = i;
            f.site = 0;
        }
        self.step_sites();
        REPAINT | STEP
    }

    /// Focuses the site number `i` of the chosen row (a tap on a ring is `step_pointer`).
    pub fn step_site(&mut self, i: usize) -> u32 {
        if !self.step_sync() {
            return STEP | REPAINT;
        }
        if let Some(f) = self.step_field.as_mut()
            && i < f.sites.len()
        {
            f.site = i;
        }
        self.step_marks_build();
        REPAINT | STEP
    }

    fn step_refilter(&mut self) {
        let Some(f) = self.step_field.as_mut() else {
            return;
        };
        let masses = f.masses.clone();
        let shift = move |d: &str| {
            masses
                .as_ref()
                .and_then(|m| crate::values::delta_mass(d, m))
        };
        f.matches = steps::filter(&f.text, &shift);
        f.pick = 0;
        f.site = 0;
        self.step_sites();
    }

    /// The sites of the chosen row, and the marks that show them.
    fn step_sites(&mut self) {
        let Some(f) = self.step_field.as_ref() else {
            return;
        };
        let (of, name) = (f.of, f.matches.get(f.pick).map(|s| s.name));
        let found = match name {
            Some(n) => steps::sites(self.doc(), of, n).map(|s| s.ids),
            None => Ok(Vec::new()),
        };
        if let Some(f) = self.step_field.as_mut() {
            f.site = 0;
            match found {
                Ok(ids) => (f.sites, f.error) = (ids, ""),
                Err(key) => (f.sites, f.error) = (Vec::new(), key),
            }
        }
        self.step_marks_build();
    }

    /// Where a site is, in document units: an atom, or the middle of a bond.
    fn step_site_at(&self, kind: Kind, id: u32) -> Option<(f64, f64)> {
        let d = self.doc();
        match kind {
            Kind::Atom => d.atom(id).map(|a| (a.x, a.y)),
            Kind::Bond => {
                let b = d.bond(id)?;
                let (p, q) = (d.atom(b.a)?, d.atom(b.b)?);
                Some(((p.x + q.x) / 2.0, (p.y + q.y) / 2.0))
            }
        }
    }

    /// The name of a site for the live line: `C7` for an atom (its element and its id), `C7-C8` for a bond.
    fn step_site_name(&self, kind: Kind, id: u32) -> String {
        let d = self.doc();
        let at = |a: u32| {
            d.atom(a)
                .map_or(String::new(), |x| format!("{}{}", x.el, x.id))
        };
        match kind {
            Kind::Atom => at(id),
            Kind::Bond => d
                .bond(id)
                .map_or(String::new(), |b| format!("{}-{}", at(b.a), at(b.b))),
        }
    }

    /// The overlay of the chosen row: a ring on each site, two on the focused one, and there the ghost of the step: a dot and a line
    /// for an atom it adds, a struck ring for an atom it takes away (the patch of the step is read, nothing is run).
    fn step_marks_build(&mut self) {
        let Some(f) = self.step_field.as_ref() else {
            return;
        };
        let Some(&row) = f.matches.get(f.pick) else {
            if let Some(f) = self.step_field.as_mut() {
                f.marks.clear();
            }
            return;
        };
        let mut marks = Vec::new();
        for (i, &id) in f.sites.iter().enumerate() {
            let Some(p) = self.step_site_at(row.kind, id) else {
                continue;
            };
            marks.push(Mark::Ring {
                p,
                c: colour::ACCENT,
                r: 0.9,
            });
            if i == f.site {
                marks.push(Mark::Ring {
                    p,
                    c: colour::ACCENT,
                    r: 1.25,
                });
            }
        }
        if let Some(&site) = f.sites.get(f.site)
            && let Ok((_, ops)) = steps::patch(self.doc(), f.of, row.name, site)
        {
            marks.extend(self.ghost(&ops));
        }
        if let Some(f) = self.step_field.as_mut() {
            f.marks = marks;
        }
    }

    /// The marks of the atoms and bonds a patch adds (accent) and removes (struck, in the warning colour).
    fn ghost(&self, ops: &[crate::ops::Op]) -> Vec<Mark> {
        use crate::ops::Op;
        let d = self.doc();
        let mut added: BTreeMap<u32, (f64, f64)> = BTreeMap::new();
        let mut out = Vec::new();
        let mut k = 0u32;
        let pos = |id: u32, added: &BTreeMap<u32, (f64, f64)>| {
            added
                .get(&id)
                .copied()
                .or_else(|| d.atom(id).map(|a| (a.x, a.y)))
        };
        for op in ops {
            match op {
                Op::AtomAdd { id, x, y, .. } => {
                    // a patch names its k-th new atom `NEW + k` (k from 1), whether or not the operation says so
                    k += 1;
                    added.insert(id.unwrap_or(tp::NEW + k), (*x, *y));
                    out.push(Mark::Dot {
                        p: (*x, *y),
                        c: colour::ACCENT,
                    });
                }
                Op::BondAdd { a, b, .. } if added.contains_key(a) || added.contains_key(b) => {
                    if let (Some(p), Some(q)) = (pos(*a, &added), pos(*b, &added)) {
                        out.push(Mark::Line {
                            a: p,
                            b: q,
                            c: colour::ACCENT,
                            heavy: false,
                        });
                    }
                }
                Op::Remove { atoms, bonds, .. } => {
                    for &a in atoms {
                        if let Some(p) = pos(a, &added) {
                            out.push(Mark::Ring {
                                p,
                                c: colour::WARN,
                                r: 0.6,
                            });
                        }
                    }
                    for &b in bonds {
                        if let Some(bond) = d.bond(b)
                            && let (Some(p), Some(q)) = (d.atom(bond.a), d.atom(bond.b))
                        {
                            out.push(Mark::Line {
                                a: (p.x, p.y),
                                b: (q.x, q.y),
                                c: colour::WARN,
                                heavy: true,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// The overlay for `Editor::scene`: nothing when the history moved under the field.
    pub(crate) fn step_marks(&self) -> Vec<Mark> {
        match self.step_field.as_ref() {
            Some(f) if f.generation == self.session.generation() => f.marks.clone(),
            _ => Vec::new(),
        }
    }

    /// The keys of the open field (`key` lower-cased): `[` and `]` the previous and next site (round), `Enter` derives, `Escape`
    /// closes, the up and down arrows choose the previous and next row. `None`: not a key of the field (the page lets the browser
    /// type it into the text).
    pub(crate) fn step_key(&mut self, key: &str) -> Option<u32> {
        if !self.step_sync() {
            return Some(STEP | REPAINT);
        }
        let (sites, pick, rows) = self
            .step_field
            .as_ref()
            .map(|f| (f.sites.len(), f.pick, f.matches.len()))?;
        Some(match key {
            "]" | "}" | "[" | "{" => {
                if sites > 0 {
                    let f = self.step_field.as_mut()?;
                    f.site = if matches!(key, "]" | "}") {
                        (f.site + 1) % sites
                    } else {
                        (f.site + sites - 1) % sites
                    };
                    self.step_marks_build();
                }
                REPAINT | STEP
            }
            "arrowdown" | "arrowup" if rows > 0 => {
                let next = if key == "arrowdown" {
                    (pick + 1) % rows
                } else {
                    (pick + rows - 1) % rows
                };
                self.step_pick(next)
            }
            "enter" => self.step_derive(),
            "escape" => self.step_close(),
            _ => return None,
        })
    }

    /// Derives the chosen step at the focused site (`tp::derive_step`, one undo step; the copy becomes the selection) and closes the
    /// field. A refusal (a site that no longer holds) is said in the live line and the field stays open.
    pub fn step_derive(&mut self) -> u32 {
        if !self.step_sync() {
            return STEP | REPAINT;
        }
        let Some((of, name, site)) = self
            .step_field
            .as_ref()
            .and_then(|f| Some((f.of, f.matches.get(f.pick)?.name, *f.sites.get(f.site)?)))
        else {
            self.error = "error.draw2.tpSite";
            return REPAINT | STEP;
        };
        let first = self.doc().next;
        match tp::derive_step(self.doc(), &self.style, of, name, site, None) {
            Ok(ops) => {
                self.step_field = None;
                self.apply_derive(first, &ops) | STEP
            }
            Err(key) => {
                self.error = key;
                REPAINT | STEP
            }
        }
    }

    /// A press on a ringed site focuses it (the touch and pen way: tap the ring, then Derive); the move and release of that press are
    /// swallowed. Any other press is the tools' (the field stays open). `None`: not for the field.
    pub(crate) fn step_pointer(
        &mut self,
        phase: u32,
        id: u32,
        x: f64,
        y: f64,
        kind: u32,
        mods: u32,
    ) -> Option<u32> {
        if !self.step_sync() {
            return None;
        }
        let own = self.step_field.as_ref()?.eat;
        if let Some(own) = own {
            if own != id {
                return None;
            }
            if phase != MOVE
                && let Some(f) = self.step_field.as_mut()
            {
                f.eat = None;
            }
            return Some(HANDLED);
        }
        if phase != DOWN || mods & (SECONDARY | MIDDLE) != 0 {
            return None;
        }
        let (row, sites) = {
            let f = self.step_field.as_ref()?;
            (*f.matches.get(f.pick)?, f.sites.clone())
        };
        let reach = Self::reach(kind).max(self.style.metrics(self.view.scale, true).font);
        let hit = sites
            .iter()
            .enumerate()
            .filter_map(|(i, &s)| {
                let p = self.step_site_at(row.kind, s)?;
                let (px, py) = self.view.to_px(p.0, p.1);
                let d = crate::m::hypot(px - x, py - y);
                (d <= reach).then_some((d, i))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))?;
        if let Some(f) = self.step_field.as_mut() {
            f.eat = Some(id);
        }
        Some(self.step_site(hit.1) | HANDLED)
    }

    /// The field for the page: `{"open", "text", "pick", "rows": [{"name", "delta", "kind", "mass"?}], "step", "delta", "kind",
    /// "sites": [id], "site", "at", "error"}`. `mass` is the mass shift with four decimals when an engine gave the element masses.
    /// Closes the field first when the history moved under it.
    pub fn step_json(&mut self) -> String {
        if !self.step_sync() {
            return r#"{"open":false}"#.to_string();
        }
        let Some(f) = self.step_field.as_ref() else {
            return r#"{"open":false}"#.to_string();
        };
        let rows: Vec<serde_json::Value> = f
            .matches
            .iter()
            .map(|s| {
                let mass = f
                    .masses
                    .as_ref()
                    .and_then(|m| crate::values::delta_mass(s.delta, m))
                    .map(|v| {
                        let w = mzlab_mol::fixed(v, 4);
                        if w.starts_with('-') { w } else { format!("+{w}") }
                    });
                serde_json::json!({"name": s.name, "delta": s.delta, "kind": s.kind.name(), "mass": mass})
            })
            .collect();
        let row = f.matches.get(f.pick);
        let at = row
            .zip(f.sites.get(f.site))
            .map(|(r, &id)| self.step_site_name(r.kind, id));
        serde_json::json!({
            "open": true,
            "text": f.text,
            "pick": f.pick,
            "rows": rows,
            "step": row.map(|r| r.name),
            "delta": row.map(|r| r.delta),
            "kind": row.map(|r| r.kind.name()),
            "sites": f.sites,
            "site": f.site,
            "at": at,
            "error": f.error,
        })
        .to_string()
    }
}
