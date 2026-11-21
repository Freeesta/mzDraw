//! The text panel in the editor: a selection inside the text of a text object (a caret, a range), the keys that move it and type,
//! the pointer that sets it, and the style buttons. Every change is an `obj.set` of the log (`runs` or a plain `text`), so it undoes
//! exactly and replays natively; the selection itself is the user's view and no step of the history. The page only forwards events
//! and paints `state().text` (the style under the caret and the lists of the controls), it decides nothing.

use crate::doc::{Obj, ObjKind};
use crate::editor::{Editor, CHANGED, CTRL, HANDLED, REPAINT, SHIFT};
use crate::ops::Op;
use crate::runs::Run;
use crate::scene::TextSel;
use crate::textedit::{self as te, Change};

/// What the panel acts on: the text object, the range (`lo..hi`, equal for a caret) and whether keys type into it.
pub(crate) struct Target {
    pub id: u32,
    pub lo: usize,
    pub hi: usize,
    pub caret: usize,
    pub editing: bool,
    pub runs: Vec<Run>,
}

impl Editor {
    fn text_object(&self, id: u32) -> Option<&Obj> {
        self.doc().object(id).filter(|o| o.t == ObjKind::Text)
    }

    /// The panel's target: the text being edited, or the one text object that is the whole selection (then the range is all of it).
    pub(crate) fn text_target(&self) -> Option<Target> {
        let o = &self.over;
        let alone = o.sel_atoms.is_empty() && o.sel_bonds.is_empty() && o.sel_objs.len() == 1;
        let id = *o.sel_objs.first().filter(|_| alone)?;
        let obj = self.text_object(id)?;
        let runs = te::runs_of(obj);
        let n = te::len(&runs);
        match o.text.as_ref().filter(|t| t.id == id) {
            Some(t) => {
                let (a, c) = (t.anchor.min(n), t.caret.min(n));
                Some(Target {
                    id,
                    lo: a.min(c),
                    hi: a.max(c),
                    caret: c,
                    editing: t.editing,
                    runs,
                })
            }
            None => Some(Target {
                id,
                lo: 0,
                hi: n,
                caret: n,
                editing: false,
                runs,
            }),
        }
    }

    /// Whether an arrow is the whole selection (its text is not styled by the panel).
    pub(crate) fn arrow_selected_alone(&self) -> bool {
        let o = &self.over;
        o.sel_atoms.is_empty()
            && o.sel_bonds.is_empty()
            && o.sel_objs.len() == 1
            && self
                .doc()
                .object(o.sel_objs[0])
                .is_some_and(|x| x.t == ObjKind::Arrow)
    }

    /// Drops a selection inside a text that is not the selected text any more.
    pub(crate) fn text_sync(&mut self) {
        if let Some(t) = &self.over.text
            && self.text_target().is_none_or(|x| x.id != t.id)
        {
            self.over.text = None;
        }
    }

    /// Whether keys type into a text now.
    pub fn text_editing(&self) -> bool {
        self.text_target().is_some_and(|t| t.editing)
    }

    /// The font of the screen and the ratio of a script to it, as the display list uses them.
    fn text_metrics(&self) -> (f64, f64) {
        let m = self.style.metrics(self.view.scale, true);
        (m.font, if m.font > 0.0 { m.small / m.font } else { 0.7 })
    }

    fn set_sel(&mut self, id: u32, anchor: usize, caret: usize, editing: bool) {
        let keep = self.over.text.take().filter(|t| t.id == id);
        self.over.text = Some(TextSel {
            id,
            anchor,
            caret,
            editing,
            dragging: keep.as_ref().is_some_and(|t| t.dragging),
            pending: None,
            join: keep.and_then(|t| t.join),
        });
    }

    /// Starts editing the selected text: the caret goes to its end.
    pub fn text_open(&mut self) -> u32 {
        let Some(t) = self.text_target() else {
            self.error = "error.draw2.textNone";
            return REPAINT;
        };
        let n = te::len(&t.runs);
        self.set_sel(t.id, n, n, true);
        self.said = ("draw2.say.textEdit", String::new());
        self.error = "";
        REPAINT | CHANGED
    }

    /// Leaves the editing of the text; the text stays selected.
    pub fn text_close(&mut self) -> u32 {
        if self.over.text.is_none() {
            return 0;
        }
        self.over.text = None;
        self.said = ("draw2.say.textDone", String::new());
        REPAINT | CHANGED
    }

    /// The object as one operation: a single plain line is a plain `text`, anything else `runs`.
    fn text_op(id: u32, runs: Vec<Run>) -> Op {
        let plain = runs.iter().all(|r| !r.br && !r.styled()) && te::len(&runs) <= 200;
        Op::ObjSet {
            id,
            text: plain.then(|| crate::runs::plain(&runs)),
            runs: (!plain).then_some(runs),
            x2: None,
            y2: None,
            from: None,
            to: None,
        }
    }

    /// Writes new runs to the text as one undo step (or joined to the last one, for typing). An empty text is removed with its
    /// object, in the same step. `false` in the second place: refused, the key is in `error`.
    fn text_write(
        &mut self,
        id: u32,
        runs: Vec<Run>,
        say: &'static str,
        what: &str,
        join: bool,
    ) -> (u32, bool) {
        let op = if runs.is_empty() {
            Op::Remove {
                atoms: Vec::new(),
                bonds: Vec::new(),
                objects: vec![id],
            }
        } else {
            Self::text_op(id, runs)
        };
        let done = if join {
            self.session.run_into_last(&[op])
        } else {
            self.session.run(&[op])
        };
        match done {
            Ok(()) => {
                self.said = (say, what.to_string());
                self.error = "";
                self.prune();
                self.text_sync();
                (REPAINT | CHANGED, true)
            }
            Err(e) => {
                self.error = e.key;
                (REPAINT, false)
            }
        }
    }

    /// The style a typed character takes at the caret: the pending style of a button pressed with no range, else the style before the caret.
    fn text_style_at(&self, t: &Target) -> Run {
        let pending = self.over.text.as_ref().and_then(|s| s.pending.clone());
        pending.unwrap_or_else(|| te::template(&t.runs, if t.lo < t.hi { t.lo + 1 } else { t.lo }))
    }

    /// A style button of the panel (`kind`: `b`, `i`, `u`, `sub`, `sup`, `font`, `size`, `colour`): applied to the selected range; to
    /// the whole text when it is selected and not edited; with a caret and no range it is the style of the next typed character.
    pub fn text_style(&mut self, kind: &str, value: &str) -> u32 {
        let Some(change) = Change::parse(kind, value) else {
            self.error = "error.draw2.textStyle";
            return REPAINT;
        };
        let Some(t) = self.text_target() else {
            self.error = "error.draw2.textNone";
            return REPAINT;
        };
        self.error = "";
        if t.editing && t.lo == t.hi {
            // no range: the next characters take the style
            let base = Run {
                t: "x".into(),
                ..self.text_style_at(&t)
            };
            let one = te::apply(&[base], 0, 1, &change);
            let mut next = one.into_iter().next().unwrap_or_default();
            next.t.clear();
            if let Some(s) = self.over.text.as_mut() {
                s.pending = Some(next);
            }
            return REPAINT | CHANGED;
        }
        let new = te::apply(&t.runs, t.lo, t.hi, &change);
        if new == t.runs {
            return REPAINT;
        }
        let f = self
            .text_write(t.id, new, "draw2.say.textStyled", kind, false)
            .0;
        if let Some(s) = self.over.text.as_mut() {
            s.pending = None;
            s.join = None;
        }
        f
    }

    /// Typed text, a paste or an input method's result goes in place of the range, in the style of the character before it.
    pub fn text_input(&mut self, text: &str) -> u32 {
        let Some(t) = self.text_target().filter(|t| t.editing) else {
            return 0;
        };
        if text.is_empty() {
            return 0;
        }
        let style = self.text_style_at(&t);
        let (new, n) = te::splice(&t.runs, t.lo, t.hi, text, &style);
        if n == 0 {
            // nothing went in: the range stays and no step is written; the note says why (control characters only, or a cap)
            self.error = if te::insertable(text) {
                "error.draw2.runsCount"
            } else {
                "error.draw2.textChars"
            };
            return REPAINT;
        }
        self.text_replace(&t, new, t.lo + n, "draw2.say.textTyped")
    }

    /// Writes the runs after a typed or deleted range and puts the caret at `at`; typing joins the undo step of the one before it.
    fn text_replace(&mut self, t: &Target, new: Vec<Run>, at: usize, say: &'static str) -> u32 {
        let join = self.over.text.as_ref().and_then(|s| s.join) == Some(self.session.generation());
        let (f, ok) = self.text_write(t.id, new, say, "", join);
        if ok {
            if self.text_object(t.id).is_some() {
                let after = self.session.generation();
                self.set_sel(t.id, at, at, true);
                if let Some(s) = self.over.text.as_mut() {
                    s.join = Some(after);
                }
            } else {
                // the last character went: the text goes with it
                self.over.text = None;
                self.said = ("draw2.say.textGone", String::new());
            }
        }
        f
    }

    /// The plain text of the range, for the clipboard.
    pub fn text_selected(&self) -> String {
        let Some(t) = self.text_target().filter(|t| t.editing) else {
            return String::new();
        };
        crate::runs::plain(&t.runs)
            .chars()
            .skip(t.lo)
            .take(t.hi - t.lo)
            .collect()
    }

    /// Removes the range (cut), or the character before (`back`) or after the caret; with nothing to remove, nothing happens.
    pub fn text_delete(&mut self, back: bool) -> u32 {
        let Some(t) = self.text_target().filter(|t| t.editing) else {
            return 0;
        };
        let n = te::len(&t.runs);
        let (lo, hi) = match (t.lo < t.hi, back) {
            (true, _) => (t.lo, t.hi),
            (false, true) if t.lo > 0 => (te::cluster_before(&t.runs, t.lo), t.lo),
            (false, false) if t.lo < n => (t.lo, te::cluster_after(&t.runs, t.lo)),
            _ => return REPAINT,
        };
        let new = te::remove(&t.runs, lo, hi);
        self.text_replace(&t, new, lo, "draw2.say.textTyped")
    }

    /// Moves the caret to `at` (the range grows from the anchor with `extend`).
    fn text_move(&mut self, t: &Target, at: usize, extend: bool) -> u32 {
        let anchor = if extend {
            self.over.text.as_ref().map_or(t.caret, |s| s.anchor)
        } else {
            at
        };
        // `set_sel` keeps the join of the typing before; moving ends it: the next character is a step of its own
        self.set_sel(t.id, anchor, at, true);
        if let Some(s) = self.over.text.as_mut() {
            s.join = None;
        }
        REPAINT | CHANGED
    }

    /// A key while a text is edited. `None`: not a key of the text (the other bindings act on it).
    pub(crate) fn text_key(&mut self, key: &str, mods: u32) -> Option<u32> {
        let t = self.text_target().filter(|t| t.editing)?;
        let (ctrl, shift) = (mods & CTRL != 0, mods & SHIFT != 0);
        let alt = mods & crate::editor::ALT != 0;
        let lower = key.to_ascii_lowercase();
        let n = te::len(&t.runs);
        let (font, script) = self.text_metrics();
        let at = |p: usize| te::caret_at(&t.runs, font, script, p);
        let f = if ctrl && !alt {
            match lower.as_str() {
                "b" => self.text_style("b", ""),
                "i" => self.text_style("i", ""),
                "u" => self.text_style("u", ""),
                "," => self.text_style("sub", ""),
                "." => self.text_style("sup", ""),
                "a" => {
                    self.set_sel(t.id, 0, n, true);
                    REPAINT | CHANGED
                }
                // undo, redo and the rest keep their bindings
                _ => return None,
            }
        } else if ctrl {
            // Ctrl with Alt is AltGr on many layouts: a character typed with it is text; another key with it is not ours
            if key.chars().count() == 1 {
                self.text_input(key)
            } else {
                return None;
            }
        } else if alt {
            // Alt alone belongs to the nudge and turn keys
            return None;
        } else {
            match lower.as_str() {
                "escape" => self.text_close(),
                "enter" => self.text_input("\n"),
                "backspace" => self.text_delete(true),
                "delete" => self.text_delete(false),
                "arrowleft" | "arrowright" => {
                    let to = if lower == "arrowleft" {
                        if !shift && t.lo < t.hi {
                            t.lo
                        } else {
                            te::cluster_before(&t.runs, t.caret)
                        }
                    } else if !shift && t.lo < t.hi {
                        t.hi
                    } else {
                        te::cluster_after(&t.runs, t.caret)
                    };
                    self.text_move(&t, to, shift)
                }
                "arrowup" | "arrowdown" => {
                    let (line, x) = at(t.caret);
                    let to = if lower == "arrowup" {
                        if line == 0 {
                            0
                        } else {
                            te::pos_at(&t.runs, font, script, line as i64 - 1, x)
                        }
                    } else if line + 1 > t.runs.iter().filter(|r| r.br).count() {
                        n
                    } else {
                        te::pos_at(&t.runs, font, script, line as i64 + 1, x)
                    };
                    self.text_move(&t, to, shift)
                }
                "home" => {
                    let to = te::line_span(&t.runs, t.caret).0;
                    self.text_move(&t, to, shift)
                }
                "end" => {
                    let to = te::line_span(&t.runs, t.caret).1;
                    self.text_move(&t, to, shift)
                }
                // a character; `Tab` and the function keys are not text (AltGr characters are the branch above)
                _ if key.chars().count() == 1 => self.text_input(key),
                _ => return None,
            }
        };
        Some(f | HANDLED)
    }

    /// The position of the pixel (`px`, `py`) in the text object `o`.
    fn text_pos(&self, o: &Obj, px: f64, py: f64) -> usize {
        let (font, script) = self.text_metrics();
        let (x, y) = self.view.to_px(o.x, o.y);
        let line = ((py - y) / (font * 1.25)).round() as i64;
        let runs = te::runs_of(o);
        te::snap(&runs, te::pos_at(&runs, font, script, line, px - x))
    }

    /// The width and the number of lines of a text, in pixels, for the hit test: what is drawn, not the first line of the plain text.
    pub(crate) fn text_box(&self, o: &Obj) -> (f64, usize) {
        let (font, script) = self.text_metrics();
        let laid = crate::runs::layout(&te::runs_of(o), font, script);
        (laid.width(), laid.widths.len())
    }

    /// A press while a text is edited: inside the text it sets the caret (Shift extends the range) and a drag grows the range;
    /// outside, the editing ends and the press goes on to the tool. `None`: not for the text.
    pub(crate) fn text_press(&mut self, px: f64, py: f64, mods: u32) -> Option<u32> {
        let t = self.text_target().filter(|t| t.editing)?;
        let obj = self.text_object(t.id)?.clone();
        if !self.hit_text(px, py, &obj) {
            self.over.text = None;
            return None;
        }
        let pos = self.text_pos(&obj, px, py);
        let anchor = if mods & SHIFT != 0 {
            self.over.text.as_ref().map_or(pos, |s| s.anchor)
        } else {
            pos
        };
        self.set_sel(t.id, anchor, pos, true);
        if let Some(s) = self.over.text.as_mut() {
            s.dragging = true;
            s.join = None;
        }
        Some(REPAINT | CHANGED)
    }

    /// Whether the pixel is on the drawn text of `o` (with a margin).
    pub(crate) fn hit_text(&self, px: f64, py: f64, o: &Obj) -> bool {
        let (font, _) = self.text_metrics();
        let (x, y) = self.view.to_px(o.x, o.y);
        let (w, lines) = self.text_box(o);
        let r = 6.0;
        px >= x - r
            && px <= x + w + r
            && py >= y - font * 0.6 - r * 0.5
            && py <= y + (lines - 1) as f64 * font * 1.25 + font * 0.6 + r * 0.5
    }

    /// A drag while a text is edited: the caret follows the pointer. `false`: no drag of a text.
    pub(crate) fn text_drag(&mut self, px: f64, py: f64) -> bool {
        let Some(t) = self.over.text.as_ref().filter(|t| t.dragging) else {
            return false;
        };
        let (id, anchor) = (t.id, t.anchor);
        if let Some(obj) = self.text_object(id).cloned() {
            let pos = self.text_pos(&obj, px, py);
            self.set_sel(id, anchor, pos, true);
        }
        true
    }

    /// The release ends a drag of the range.
    pub(crate) fn text_drag_end(&mut self) -> bool {
        match self.over.text.as_mut().filter(|t| t.dragging) {
            Some(t) => {
                t.dragging = false;
                true
            }
            None => false,
        }
    }

    /// A double click or double tap on a text: it becomes the selection, is edited, and the word under the pointer is selected.
    pub fn text_double(&mut self, px: f64, py: f64) -> u32 {
        let hit =
            self.doc().objects.iter().rev().find_map(|o| {
                (o.t == ObjKind::Text && self.hit_text(px, py, o)).then(|| o.clone())
            });
        let Some(obj) = hit else {
            return 0;
        };
        let o = &mut self.over;
        o.sel_atoms.clear();
        o.sel_bonds.clear();
        o.sel_objs = vec![obj.id];
        let (a, b) = te::word_span(&te::runs_of(&obj), self.text_pos(&obj, px, py));
        self.set_sel(obj.id, a, b, true);
        self.said = ("draw2.say.textEdit", String::new());
        REPAINT | CHANGED
    }

    /// The font and size of the preferences for a new text, as the family and the multiple of the style's size (`None` for the style's own).
    fn text_defaults(&self) -> (String, f64) {
        let font = te::FONTS
            .iter()
            .find(|(k, _)| *k == self.textp.font)
            .map_or("", |f| f.1);
        let pct: f64 = self.textp.size.parse().unwrap_or(100.0);
        (
            font.to_string(),
            if pct == 100.0 { 0.0 } else { pct / 100.0 },
        )
    }

    /// The text a new text object gets: the typed text with the preferred font and size of the Preferences (`text.font`, `text.size`);
    /// with the defaults it is unchanged, so a log without those settings reads as before.
    pub(crate) fn text_new(&self, plain: String, runs: Vec<Run>) -> (String, Vec<Run>) {
        let (font, sz) = self.text_defaults();
        if font.is_empty() && sz == 0.0 {
            return (plain, runs);
        }
        let mut runs = if runs.is_empty() {
            vec![Run::plain(&plain)]
        } else {
            runs
        };
        for r in runs.iter_mut().filter(|r| !r.br) {
            if r.font.is_empty() {
                r.font.clone_from(&font);
            }
            if r.sz == 0.0 {
                r.sz = sz;
            }
        }
        (String::new(), crate::runs::canon(runs))
    }

    /// The panel as JSON for the page: `{"id", "editing", "from", "to", "len", "style": {"b", "i", "u", "sub", "sup", "font", "size",
    /// "colour"} (each `null` where the range differs), "range": {"kind": "caret" | "all" | "some", "n", "at", "total"}, "fontKey" (the
    /// key of the font shown, or `mixed` / `other`), "fontExtra" (the entry for those two, or `null`), "fonts": [{"key", "name"}],
    /// "sizes": [percent, with the size in use among them], "max"}`, or `null` when no text
    /// is selected. The style is that of the range, or of the character before the caret (the pending style of a button when it was
    /// pressed with no range).
    pub fn text_json(&self) -> String {
        let Some(t) = self.text_target() else {
            return "null".into();
        };
        let q = |s: &str| serde_json::Value::from(s).to_string();
        let tri = |v: Option<bool>| v.map_or("null".to_string(), |b| b.to_string());
        let pending = self.over.text.as_ref().and_then(|s| s.pending.clone());
        let s = match pending.filter(|_| t.editing && t.lo == t.hi) {
            Some(p) => te::style_of(&[Run { t: "x".into(), ..p }], 0, 1),
            None => te::style_of(&t.runs, t.lo, t.hi),
        };
        let fonts: Vec<String> = te::FONTS
            .iter()
            .map(|(k, n)| format!(r#"{{"key":{},"name":{}}}"#, q(k), q(n)))
            .collect();
        // the control shows one of the listed fonts, or an entry of its own for a font outside the list or a mixed range
        let (font_key, font_extra) = match &s.font {
            None => (
                "mixed".to_string(),
                r#"{"key":"mixed","name":""}"#.to_string(),
            ),
            Some(f) => match te::FONTS.iter().find(|x| x.1 == f) {
                Some((k, _)) => (k.to_string(), "null".to_string()),
                None => (
                    "other".to_string(),
                    format!(r#"{{"key":"other","name":{}}}"#, q(f)),
                ),
            },
        };
        let pct = s.sz.map(|z| (z * 100.0).round() as i64);
        let mut sizes: Vec<i64> = te::SIZES.iter().map(|&n| i64::from(n)).collect();
        sizes.extend(pct.filter(|p| !sizes.contains(p)));
        sizes.sort_unstable();
        let sizes: Vec<String> = sizes.iter().map(i64::to_string).collect();
        let size = pct.map_or("null".to_string(), |p| p.to_string());
        let n = te::len(&t.runs);
        let kind = if t.lo == t.hi {
            "caret"
        } else if t.hi - t.lo == n {
            "all"
        } else {
            "some"
        };
        format!(
            r#"{{"id":{},"editing":{},"from":{},"to":{},"len":{n},"range":{{"kind":"{kind}","n":{},"at":{},"total":{n}}},"style":{{"b":{},"i":{},"u":{},"sub":{},"sup":{},"font":{},"size":{size},"colour":{}}},"fontKey":{},"fontExtra":{font_extra},"fonts":[{}],"sizes":[{}],"max":{}}}"#,
            t.id,
            t.editing,
            t.lo,
            t.hi,
            t.hi - t.lo,
            t.hi,
            tri(s.b),
            tri(s.i),
            tri(s.u),
            tri(s.sub),
            tri(s.sup),
            s.font.as_ref().map_or("null".to_string(), |f| q(f)),
            s.color.map_or("null".to_string(), |c| q(&c)),
            q(&font_key),
            fonts.join(","),
            sizes.join(","),
            crate::runs::MAX_CHARS
        )
    }
}
