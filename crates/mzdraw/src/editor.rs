//! The interactive editor: a session, a view, a tool and the pointer state machine. The page forwards pointer, wheel and key events
//! as numbers and paints the display list; every decision (what is under the pointer, what a click does) is taken here.

use crate::doc::{Document, ObjKind, WEDGE_DOWN, WEDGE_UP, WEDGE_WAVY};
use crate::io;
use crate::m;
use crate::ops::{Op, Session};
use crate::scene::{self, Overlay, Scene, View};
use crate::settings::Style;

/// Flags an event returns.
pub const REPAINT: u32 = 1;
pub const CHANGED: u32 = 2;
pub const TOOL: u32 = 4;
pub const HANDLED: u32 = 8;
/// The page should move the keyboard to its label field.
pub const ASK_LABEL: u32 = 16;
/// The page should move the keyboard to its text field.
pub const ASK_TEXT: u32 = 32;
/// The keyboard focus moved or what it is on changed: the page should say where it is.
pub const FOCUS: u32 = 64;
/// The layout of the palettes changed (`Layout`): the page should lay them out again.
pub const PALETTES: u32 = 128;
/// An item of the context menu needs the page to finish it: read `context_ask` (`context`).
pub const ASK: u32 = 256;
/// The page should open the context menu: a finger or a pen held still, the secondary button of a mouse or the barrel button of a
/// pen, the Menu key or Shift+F10. The page calls `context_menu` at the point of the event, or at `context_anchor` for a key.
pub const MENU: u32 = 512;
/// A proposal (`proposal`) appeared, changed or went: the page should show its panel and say where the current hunk is.
pub const PROPOSAL: u32 = 1024;
/// The step field (tp-11) opened, closed or changed: the page repaints it (`step_field` of the bridge).
pub const STEP: u32 = 2048;

/// Modifier bits of an event.
pub const SHIFT: u32 = 1;
pub const CTRL: u32 = 2;
/// Bit 4 of a pointer event is the eraser end of a pen; bit 8 is Alt (Option) on a key.
pub const ALT: u32 = 8;
/// The secondary button of a mouse (right) or the barrel button of a pen: the context menu, not the tool.
pub const SECONDARY: u32 = 16;
/// The middle button of a mouse: it pans the view, with the secondary one when it is dragged.
pub const MIDDLE: u32 = 32;

/// Pointer kinds: mouse, pen, finger. A finger needs a wider reach.
pub const MOUSE: u32 = 0;
pub const PEN: u32 = 1;
pub const TOUCH: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    Atom(String),
    /// Order 1-3 and wedge mark 0-3 (a mark only on order 1).
    Bond(u8, u8),
    Erase,
    Ring(u8, bool),
    /// A zigzag chain: dragged from an atom or from empty space, or two bonds on a click.
    Chain,
    Charge(i8),
    /// An abbreviation or an R group of `label`.
    Label(String),
    /// A reaction arrow, dragged or of a standard length.
    Arrow,
    Plus,
    /// Lone pairs: a click on an atom adds a pair (up to four, then none).
    LonePair,
    /// An electron-pushing curve from a press to a release; `true` for a half head (one electron).
    Curve(bool),
    /// A line of text to place, or to set on a text or an arrow.
    Text(String),
}

/// The second field of `Tool::Bond` above the wedge marks: a bond tool that draws a display kind (`bonddisp`) instead of a mark;
/// the document never holds these numbers.
pub const KIND_DASH: u8 = 4;
pub const KIND_BDASH: u8 = 5;
pub const KIND_HASH: u8 = 6;
pub const KIND_BOLD: u8 = 7;
pub const KIND_DATIVE: u8 = 8;
pub const KIND_HBOND: u8 = 9;

/// The display kind (`bonddisp::KINDS`) a `Tool::Bond` mark field stands for, empty for a wedge mark or none.
pub(crate) fn kind_of_tool(s: u8) -> &'static str {
    match s {
        KIND_DASH => "dash",
        KIND_BDASH => "bdash",
        KIND_HASH => "hash",
        KIND_BOLD => "bold",
        KIND_DATIVE => "dative",
        KIND_HBOND => "hbond",
        _ => "",
    }
}

impl Tool {
    /// `select`, `atom:N`, `bond:2`, `bond:up` / `bond:down` / `bond:wavy` (wedge marks), `bond:dash` / `bond:bdash` / `bond:hash` / `bond:bold` / `bond:dative` / `bond:hbond` (display kinds), `erase`, `ring:6`, `ring:6a` (alternating
    /// double bonds), `chain`, `charge:+`, `charge:-`, `label:Ph` / `label:R1` (an abbreviation or an R group of `label`), `arrow`, `plus`,
    /// `text:<one line>`.
    pub fn parse(s: &str) -> Option<Tool> {
        let (name, arg) = s.split_once(':').unwrap_or((s, ""));
        Some(match name {
            "select" => Tool::Select,
            "erase" => Tool::Erase,
            "atom" if mzlab_mol::is_element(arg) => Tool::Atom(arg.into()),
            "bond" => match arg {
                "up" => Tool::Bond(1, WEDGE_UP),
                "down" => Tool::Bond(1, WEDGE_DOWN),
                "wavy" => Tool::Bond(1, WEDGE_WAVY),
                "dash" => Tool::Bond(1, KIND_DASH),
                "bdash" => Tool::Bond(1, KIND_BDASH),
                "hash" => Tool::Bond(1, KIND_HASH),
                "bold" => Tool::Bond(1, KIND_BOLD),
                "dative" => Tool::Bond(1, KIND_DATIVE),
                "hbond" => Tool::Bond(1, KIND_HBOND),
                _ => Tool::Bond(arg.parse().ok().filter(|o| (1..=3).contains(o))?, 0),
            },
            "chain" => Tool::Chain,
            "ring" => {
                let n = arg
                    .trim_end_matches('a')
                    .parse()
                    .ok()
                    .filter(|n| (3..=8).contains(n))?;
                Tool::Ring(n, arg.ends_with('a'))
            }
            "charge" => Tool::Charge(if arg == "-" { -1 } else { 1 }),
            "label" if crate::label::kind(arg).is_some() => Tool::Label(arg.into()),
            "arrow" => Tool::Arrow,
            "plus" => Tool::Plus,
            "lonepair" => Tool::LonePair,
            "curve" if matches!(arg, "" | "full" | "half") => Tool::Curve(arg == "half"),
            "text"
                if !arg.trim().is_empty() && arg.chars().count() <= 200 && !arg.contains('\n') =>
            {
                Tool::Text(arg.into())
            }
            _ => return None,
        })
    }

    pub fn name(&self) -> String {
        match self {
            Tool::Select => "select".into(),
            Tool::Erase => "erase".into(),
            Tool::Atom(e) => format!("atom:{e}"),
            Tool::Bond(_, WEDGE_UP) => "bond:up".into(),
            Tool::Bond(_, WEDGE_DOWN) => "bond:down".into(),
            Tool::Bond(_, WEDGE_WAVY) => "bond:wavy".into(),
            Tool::Bond(_, KIND_DASH) => "bond:dash".into(),
            Tool::Bond(_, KIND_BDASH) => "bond:bdash".into(),
            Tool::Bond(_, KIND_HASH) => "bond:hash".into(),
            Tool::Bond(_, KIND_BOLD) => "bond:bold".into(),
            Tool::Bond(_, KIND_DATIVE) => "bond:dative".into(),
            Tool::Bond(_, KIND_HBOND) => "bond:hbond".into(),
            Tool::Bond(o, _) => format!("bond:{o}"),
            Tool::Chain => "chain".into(),
            Tool::Ring(n, a) => format!("ring:{n}{}", if *a { "a" } else { "" }),
            Tool::Charge(c) => format!("charge:{}", if *c < 0 { "-" } else { "+" }),
            Tool::Label(l) => format!("label:{l}"),
            Tool::Arrow => "arrow".into(),
            Tool::Plus => "plus".into(),
            Tool::LonePair => "lonepair".into(),
            Tool::Curve(half) => format!("curve:{}", if *half { "half" } else { "full" }),
            Tool::Text(t) => format!("text:{t}"),
        }
    }
}

/// What is under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    None,
    Atom(u32),
    Bond(u32),
    /// An arrow, a plus sign or a text.
    Obj(u32),
}

/// The ids of what an import or a paste added.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Placed {
    pub atoms: Vec<u32>,
    pub bonds: Vec<u32>,
    pub objects: Vec<u32>,
}

/// A press in progress: where it started (pixels), what it hit, whether it became a drag.
pub(crate) struct Press {
    pub start: (f64, f64),
    pub now: (f64, f64),
    pub hit: Hit,
    pub moved: bool,
    pub kind: u32,
}

pub struct Editor {
    pub session: Session,
    /// The steps of the last `scheme:orient` command (for the result document), empty before the first.
    pub(crate) orient_steps: Vec<crate::orient::Step>,
    /// The rows `tp:import` refused in its last run, `(line, key)` in line order (`failed.rows` of the result document); empty otherwise.
    pub(crate) failed_rows: Vec<(u32, String)>,
    pub view: View,
    /// The size of the canvas in pixels as the page last told it (`set_canvas`, `fit`): the view centre a paste lands at.
    pub(crate) canvas: (f64, f64),
    pub tool: Tool,
    pub over: Overlay,
    pub(crate) press: Option<Press>,
    /// What the last change did, as an i18n key with one parameter, for the screen reader line.
    pub said: (&'static str, String),
    /// The numbers of a line that has more than one (`draw2.say.pathway`: structures, arrows, gaps), by name; valid only with the `said` they were set with.
    pub said_params: Vec<(&'static str, u32)>,
    /// What the last `pathway:label` left out for a reason, as (i18n key, arrows): said by `lost_on_save` (`lost` of a `draw-result/1`).
    pub label_notes: Vec<(&'static str, usize)>,
    /// The document (`to_json`) as the last `pathway:label` left it: its notes are reported only while the drawing is that very document, so
    /// an edit, an undo or a load that changes it hides them (they are about arrows that may be gone).
    pub(crate) label_notes_doc: String,
    /// A report that stays visible until the next step (the stereo guard), as an i18n key with one parameter; empty when none.
    pub notice: (&'static str, String),
    /// The last refused operation, an i18n key.
    pub error: &'static str,
    /// The fingers and the pen at work (pinch, palm rejection, long press).
    pub(crate) gesture: crate::editor_touch::Gesture,
    /// The atom the keyboard focus came from (the bond walked last).
    pub(crate) focus_from: Option<u32>,
    /// The curve whose middle handle is being dragged.
    pub(crate) bend_drag: Option<u32>,
    /// The source of a curve being made with the keyboard: the atom and whether it is its lone pair.
    pub(crate) curve_from: Option<(u32, bool)>,
    /// The digits typed with the chain tool (the count of the chain the Enter key draws), at most two.
    pub(crate) chain_keys: String,
    /// The drawing style: what the screen and the saved SVG look like. Not part of the document.
    pub style: Style,
    /// Where the palettes of the page are, and what Save writes: the `layout` and `export` objects of the settings file.
    pub layout: crate::palettes::Layout,
    pub export: crate::palettes::Export,
    /// The interface preferences (`ui.theme`).
    pub ui: crate::prefs::Ui,
    /// The default font and size of a new text (`text.font`, `text.size`).
    pub textp: crate::prefs::TextPrefs,
    /// The editor's own clipboard: an `mzdraw/1` document of what was copied, and how many times it was pasted since.
    pub(crate) clip: String,
    pub(crate) pasted: u32,
    /// The last nudge or turn by keys, to join the next press to it as one undo step.
    pub(crate) nudge: Option<crate::editor_nudge::Nudge>,
    /// The "move here" mode: with a selection, a click on an empty spot brings the selection there.
    pub move_here: bool,
    /// The open context menu and what the page was asked to do after its last item (`context`).
    pub(crate) ctx: crate::context::Context,
    /// An agent's patch held apart from the drawing (`proposal`), and whether a press on a hunk number is being swallowed.
    pub(crate) proposal: Option<crate::proposal::Proposal>,
    pub eat: Option<u32>,
    /// The open step field (tp-11): the typed text, the rows it selects, the sites of the chosen row (`editor_step`).
    pub(crate) step_field: Option<crate::editor_step::StepField>,
    /// The element masses the mass filter of the step field uses, solved once from the engine's own numbers (`values::element_masses`).
    pub(crate) step_masses: Option<std::collections::BTreeMap<String, f64>>,
    /// The engine that gives masses (`values`): none in the core, set by the bridge. Without it no mass is ever shown.
    pub measurer: Option<crate::values::Measurer>,
    /// What the last `save("tptable")` or `save("tptable-csv")` left empty, as (i18n key, count): said by `lost_on_save` and the page.
    pub table_notes: std::cell::RefCell<Vec<(&'static str, usize)>>,
    /// Whether the last `save("pathway-check")` found an error (the CLI answers with its own exit code).
    pub check_failed: std::cell::Cell<bool>,
    /// The tolerances and the polarity of `save("pathway-check")` (the `args` of the request).
    pub check_opts: crate::pathcheck::Opts,
    /// The words a pathway node that is tentative or not detected shows next to its name, in the language of the page (`set_status_words`).
    pub status_words: Option<(String, String)>,
    /// The calculated values of the structures as `refresh_values` last found them, and of the selection with the atoms it was for.
    pub(crate) measured: Vec<crate::values::Entry>,
    pub(crate) measured_error: &'static str,
    pub(crate) measured_adducts: Vec<String>,
    pub(crate) measured_sel: Option<(
        Vec<u32>,
        crate::values::Selected,
        Result<crate::values::Measured, &'static str>,
    )>,
}

/// The operations that add every atom, bond and object of `new`, moved by (`dx`, `dy`), with ids `base + id` (data and annotation come
/// along with the atom that keeps them). One list, so a caller may join it with more operations in one undo step.
pub(crate) fn place_ops(base: u32, new: &Document, dx: f64, dy: f64) -> Vec<Op> {
    let mut ops = Vec::new();
    for a in &new.atoms {
        ops.push(Op::AtomAdd {
            map: None,
            id: Some(base + a.id),
            el: a.el.clone(),
            x: a.x + dx,
            y: a.y + dy,
        });
        if a.chg != 0
            || a.iso != 0
            || a.rad != 0
            || a.map != 0
            || a.h.is_some()
            || !a.lab.is_empty()
            || a.lp() != 0
        {
            let some = |v: bool| v.then_some(());
            ops.push(Op::AtomSet {
                id: base + a.id,
                el: None,
                chg: some(a.chg != 0).map(|_| a.chg),
                iso: some(a.iso != 0).map(|_| a.iso),
                rad: some(a.rad != 0).map(|_| a.rad),
                h: a.h,
                lab: (!a.lab.is_empty()).then(|| a.lab.clone()),
                map: (a.map != 0).then_some(a.map),
                lp: (a.lp() != 0).then_some(a.lp()),
            });
        }
        // the data of a structure (the fields of an SDF record) comes along with the atom that keeps it
        let data = a.data();
        if !data.is_empty() {
            ops.push(Op::AtomData {
                id: base + a.id,
                data,
            });
        }
        let ann = a.ann();
        if !ann.is_empty() {
            ops.push(Op::AtomAnn {
                id: base + a.id,
                ann,
            });
        }
        if !a.title().is_empty() {
            ops.push(Op::AtomTitle {
                id: base + a.id,
                title: a.title(),
            });
        }
        if !a.cap().is_empty() {
            ops.push(Op::AtomCap {
                id: base + a.id,
                lines: a.cap(),
            });
        }
        if !a.col().is_empty() {
            ops.push(Op::Colour {
                atoms: vec![base + a.id],
                bonds: Vec::new(),
                objects: Vec::new(),
                col: a.col(),
            });
        }
    }
    for b in &new.bonds {
        ops.push(Op::BondAdd {
            id: Some(base + b.id),
            a: base + b.a,
            b: base + b.b,
            o: b.o,
            s: b.s,
        });
        // the side of the second line, the display kind and the "either" mark of a double bond come along with the bond
        if !b.pos().is_empty() || !b.kind().is_empty() || b.either() {
            ops.push(Op::BondSet {
                id: base + b.id,
                o: None,
                s: b.either().then_some(3),
                pos: (!b.pos().is_empty()).then(|| b.pos()),
                k: (!b.kind().is_empty()).then(|| b.kind()),
                rev: false,
            });
        }
        if !b.col().is_empty() {
            ops.push(Op::Colour {
                atoms: Vec::new(),
                bonds: vec![base + b.id],
                objects: Vec::new(),
                col: b.col(),
            });
        }
    }
    for o in &new.objects {
        let arrow = o.t == ObjKind::Arrow;
        let curve = o.t == ObjKind::Curve;
        // an anchored end of a curve goes with its anchor; a free one is shifted like the rest
        let (shift_a, shift_b) = (!curve || o.from == 0, !curve || o.to == 0);
        ops.push(Op::ObjAdd {
            id: Some(base + o.id),
            t: o.t,
            x: if shift_a { o.x + dx } else { 0.0 },
            y: if shift_a { o.y + dy } else { 0.0 },
            x2: if (arrow || curve) && shift_b {
                o.x2 + dx
            } else {
                0.0
            },
            y2: if (arrow || curve) && shift_b {
                o.y2 + dy
            } else {
                0.0
            },
            text: if o.runs.is_empty() {
                o.text.clone()
            } else {
                String::new()
            },
            runs: o.runs.clone(),
            from: if o.from == 0 { 0 } else { base + o.from },
            to: if o.to == 0 { 0 } else { base + o.to },
            cv: if curve { o.curve() } else { Default::default() },
        });
        if !o.col().is_empty() {
            ops.push(Op::Colour {
                atoms: Vec::new(),
                bonds: Vec::new(),
                objects: vec![base + o.id],
                col: o.col(),
            });
        }
    }
    ops
}

impl Default for Editor {
    fn default() -> Self {
        Editor {
            session: Session::default(),
            view: View {
                scale: 40.0,
                ox: 200.0,
                oy: 200.0,
            },
            canvas: (400.0, 400.0),
            tool: Tool::Bond(1, 0),
            over: Overlay::default(),
            press: None,
            said: ("", String::new()),
            said_params: Vec::new(),
            label_notes: Vec::new(),
            label_notes_doc: String::new(),
            notice: ("", String::new()),
            error: "",
            gesture: Default::default(),
            focus_from: None,
            bend_drag: None,
            curve_from: None,
            chain_keys: String::new(),
            style: Style::default(),
            layout: Default::default(),
            export: Default::default(),
            ui: Default::default(),
            textp: Default::default(),
            measurer: None,
            table_notes: std::cell::RefCell::default(),
            check_failed: std::cell::Cell::new(false),
            check_opts: crate::pathcheck::Opts::default(),
            status_words: None,
            measured: Vec::new(),
            measured_error: "",
            measured_adducts: Vec::new(),
            measured_sel: None,
            clip: String::new(),
            pasted: 0,
            nudge: None,
            move_here: false,
            orient_steps: Vec::new(),
            failed_rows: Vec::new(),
            ctx: Default::default(),
            proposal: None,
            eat: None,
            step_field: None,
            step_masses: None,
        }
    }
}

impl Editor {
    pub fn doc(&self) -> &Document {
        &self.session.doc
    }

    /// The rows `tp:import` refused in its last run, `(line, key)` in line order; empty when it ran well or did not run.
    pub fn failed_rows(&self) -> &[(u32, String)] {
        &self.failed_rows
    }

    /// Runs an editor command of a request (`commands::run`: `move`, `rotate`, `flip`, `pathway:tree`) as one undo step; the refusal is
    /// returned and changes nothing.
    pub fn run_command(
        &mut self,
        name: &str,
        args: &serde_json::Value,
    ) -> Result<(), crate::ops::OpError> {
        crate::commands::run(self, name, args)
    }

    /// The patch (`draw_edit/1`) that turns the drawing in the editor into `other`; see `diff::diff`.
    pub fn diff_with(&self, other: &Document) -> Vec<crate::ops::Op> {
        crate::diff::diff(self.doc(), other)
    }

    /// Runs one undo step; a refusal is kept in `error` and changes nothing.
    pub(crate) fn run(&mut self, ops: &[Op], say: &'static str, what: &str) -> u32 {
        let guard = crate::stereo_guard::before(self.doc(), ops);
        let ops = self.following(ops);
        match self.session.run(&ops) {
            Ok(()) => {
                self.said = (say, what.to_string());
                self.say_stereo(guard);
                self.error = "";
                self.prune();
                REPAINT | CHANGED
            }
            Err(e) => {
                self.error = e.key;
                REPAINT
            }
        }
    }

    /// The guard of the configuration: after a geometry step, when a piece that held a wedge now has another signature, the live
    /// line says at which atoms (report only, the step stands).
    pub(crate) fn say_stereo(&mut self, before: Option<crate::stereo_guard::Snapshot>) {
        self.notice = ("", String::new());
        if let Some(then) = before {
            let atoms = crate::stereo_guard::changed(self.doc(), &then, false);
            if !atoms.is_empty() {
                let list: Vec<String> = atoms.iter().map(u32::to_string).collect();
                self.said = ("draw2.say.stereoChanged", list.join(", "));
                self.notice = self.said.clone();
            }
        }
    }

    /// Runs operations as one undo step with the stereo guard around them (what `run` does without the words of the live line): the
    /// route of the editor commands.
    pub(crate) fn run_guarded(&mut self, ops: &[Op]) -> Result<(), crate::ops::OpError> {
        let guard = crate::stereo_guard::before(self.doc(), ops);
        self.session.run(ops)?;
        self.say_stereo(guard);
        Ok(())
    }

    /// The operations of a command and, after them, the route operations of the anchored arrows whose structures they change
    /// (`route::follow`): the one place where a command makes the arrows follow. The log carries the route operations, so a
    /// replay of the log gives the same bytes; an operation sent bare never follows.
    pub(crate) fn following(&self, ops: &[Op]) -> Vec<Op> {
        let s = &self.session.doc;
        let mut all = ops.to_vec();
        all.extend(crate::route::follow(s, ops, &self.style));
        all
    }

    /// Drops selection, hover and flags that point at things that no longer exist.
    pub(crate) fn prune(&mut self) {
        let d = &self.session.doc;
        self.over.sel_atoms.retain(|&i| d.atom(i).is_some());
        self.over.sel_bonds.retain(|&i| d.bond(i).is_some());
        self.over.sel_objs.retain(|&i| d.object(i).is_some());
        self.over.focus = self.over.focus.filter(|&i| d.atom(i).is_some());
        self.focus_from = self.focus_from.filter(|&i| d.atom(i).is_some());
        self.over.hover_obj = self.over.hover_obj.filter(|&i| d.object(i).is_some());
        self.over.warn_atoms.retain(|&i| d.atom(i).is_some());
        self.over.hover_atom = self.over.hover_atom.filter(|&i| d.atom(i).is_some());
        self.over.hover_bond = self.over.hover_bond.filter(|&i| d.bond(i).is_some());
    }

    pub fn set_tool(&mut self, name: &str) -> u32 {
        match Tool::parse(name) {
            Some(t) => {
                self.tool = t;
                self.curve_from = None;
                self.press = None;
                self.over.preview = None;
                self.over.chain = None;
                self.chain_keys.clear();
                self.over.band = None;
                // the tool becomes the face of its group on the tools palette (the page lays the palettes out again)
                let face = self.layout.note_tool(name);
                REPAINT | TOOL | if face { PALETTES } else { 0 }
            }
            None => 0,
        }
    }

    /// Names the steps made since the last call with what the live line said (`said`), so that the history list reads like the line
    /// did; steps that nothing named, or more than one at once, get the plain `draw2.history.step`. Called by the page after each
    /// action and by the replay after each operation and command; `history` calls it too.
    pub fn name_steps(&mut self) {
        let said = match self.said.0 {
            "" | "draw2.say.undo" | "draw2.say.redo" | "draw2.say.jump" => {
                ("draw2.history.step", String::new())
            }
            k => (k, self.said.1.clone()),
        };
        self.session.name_new_steps(said, "draw2.history.step");
    }

    /// Like `name_steps` for a replay operation: the step is named after the operation (`draw2.history.op`, `atom.add`).
    pub(crate) fn name_op_step(&mut self, op: &Op) {
        let name = serde_json::from_str::<serde_json::Value>(&op.to_json())
            .ok()
            .and_then(|v| v["op"].as_str().map(str::to_string))
            .unwrap_or_default();
        self.session
            .name_new_steps(("draw2.history.op", name), "draw2.history.step");
    }

    /// The history of the session: `history/1`, `{"format":"history/1","at":N,"start","steps":[{"say","what","index","state"}]}`, newest
    /// first. `index` is the number of the step (1 is the first, `jump:<index>` leaves the drawing as that step made it, `jump:0` is
    /// the start); `state` is `done`, `current` (the newest done step: where the drawing is) or `undone` (a step `redo` brings back).
    /// `at` is the number of the current step, 0 at the start.
    pub fn history_json(&self) -> String {
        let list = self.session.history();
        let at = self.session.steps();
        let rows: Vec<String> = list
            .iter()
            .enumerate()
            .rev()
            .map(|(i, ((say, what), done))| {
                let state = if i + 1 == at {
                    "current"
                } else if *done {
                    "done"
                } else {
                    "undone"
                };
                let say = if say.is_empty() {
                    "draw2.history.step"
                } else {
                    say
                };
                format!(
                    r#"{{"say":{},"what":{},"index":{},"state":"{state}"}}"#,
                    serde_json::Value::from(say),
                    serde_json::Value::from(what.as_str()),
                    i + 1
                )
            })
            .collect();
        format!(
            r#"{{"format":"history/1","at":{at},"start":"{}","steps":[{}]}}"#,
            if at == 0 { "current" } else { "done" },
            rows.join(",")
        )
    }

    /// Whether `name` is one of the history commands `undo:<n>`, `redo:<n>`, `jump:<n>`.
    pub fn is_history_command(name: &str) -> bool {
        crate::commands::steps_of(name).is_some()
    }

    /// The page command `undo:<n>`, `redo:<n>` or `jump:<n>` (the same commands the request runs): the flags, `None` when `name` is none
    /// of them, and 0 (and the refusal in `error`) when it cannot run.
    pub fn history_command(&mut self, name: &str) -> Option<u32> {
        crate::commands::steps_of(name)?;
        let before = self.session.generation();
        Some(
            match crate::commands::run(self, name, &serde_json::json!({})) {
                Ok(()) => {
                    self.error = "";
                    // a jump that changed nothing is not a change of the drawing
                    if self.session.generation() == before {
                        0
                    } else {
                        REPAINT | CHANGED
                    }
                }
                Err(e) => {
                    self.error = e.key;
                    REPAINT
                }
            },
        )
    }

    /// Undoes `n` steps (`undo:<n>`), at most as many as there are; zero steps is a refusal (`error.draw2.request`).
    pub fn undo_steps(&mut self, n: usize) -> Result<u32, &'static str> {
        let at = self.session.steps();
        if n == 0 {
            return Err("error.draw2.request");
        }
        self.jump_to(at.saturating_sub(n))
    }

    /// Redoes `n` steps (`redo:<n>`), at most as many as there are.
    pub fn redo_steps(&mut self, n: usize) -> Result<u32, &'static str> {
        let at = self.session.steps();
        if n == 0 {
            return Err("error.draw2.request");
        }
        self.jump_to((at + n).min(at + self.session.redo_steps()))
    }

    /// Leaves the drawing as the step number `n` made it (`jump:<n>`; 0 is the start): a run of undo or redo, no new operation, the
    /// steps stay in the list. A number past the newest step is `error.draw2.request` and changes nothing. Returns the flags.
    pub fn jump_to(&mut self, n: usize) -> Result<u32, &'static str> {
        self.name_steps();
        let at = self.session.steps();
        if !self.session.jump(n) {
            return Err("error.draw2.request");
        }
        if self.session.steps() == at {
            return Ok(0);
        }
        self.said = ("draw2.say.jump", n.to_string());
        self.notice = ("", String::new());
        self.prune();
        Ok(REPAINT | CHANGED)
    }

    pub fn undo(&mut self) -> u32 {
        self.name_steps();
        if !self.session.undo() {
            return 0;
        }
        self.said = ("draw2.say.undo", String::new());
        self.notice = ("", String::new());
        self.prune();
        REPAINT | CHANGED
    }

    pub fn redo(&mut self) -> u32 {
        self.name_steps();
        if !self.session.redo() {
            return 0;
        }
        self.said = ("draw2.say.redo", String::new());
        self.notice = ("", String::new());
        self.prune();
        REPAINT | CHANGED
    }

    /// Reach of the pointer in pixels.
    pub(crate) fn reach(kind: u32) -> f64 {
        match kind {
            TOUCH => 22.0,
            PEN => 16.0,
            _ => 10.0,
        }
    }

    /// What is under the pointer: the nearest atom within reach, unless the pointer is over the middle of a bond and nearer to
    /// the bond than to that atom (a finger reaches further than half a bond, and the middle of a bond must still be the
    /// bond); else the nearest bond; else an object.
    pub fn hit(&self, px: f64, py: f64, kind: u32) -> Hit {
        let r = Self::reach(kind);
        let d = self.doc();
        let mut atom = (r, Hit::None);
        for a in &d.atoms {
            let (x, y) = self.view.to_px(a.x, a.y);
            let dist = m::hypot(x - px, y - py);
            if dist < atom.0 {
                atom = (dist, Hit::Atom(a.id));
            }
        }
        // the nearest bond, and the nearest one whose middle part (30 % to 70 % of its length) the pointer is over
        let (mut bond, mut middle) = ((r * 0.7, Hit::None), (r * 0.7, Hit::None));
        for b in &d.bonds {
            let (Some(p), Some(q)) = (d.atom(b.a), d.atom(b.b)) else {
                continue;
            };
            let (s, e) = (self.view.to_px(p.x, p.y), self.view.to_px(q.x, q.y));
            let (dx, dy) = (e.0 - s.0, e.1 - s.1);
            let along = ((px - s.0) * dx + (py - s.1) * dy) / (dx * dx + dy * dy).max(1e-9);
            let t = along.clamp(0.0, 1.0);
            let dist = m::hypot(s.0 + t * dx - px, s.1 + t * dy - py);
            if dist < bond.0 {
                bond = (dist, Hit::Bond(b.id));
            }
            if (0.3..=0.7).contains(&along) && dist < middle.0 {
                middle = (dist, Hit::Bond(b.id));
            }
        }
        if atom.1 != Hit::None {
            return if middle.1 != Hit::None && middle.0 < atom.0 {
                middle.1
            } else {
                atom.1
            };
        }
        if bond.1 != Hit::None {
            return bond.1;
        }
        self.hit_object(px, py, r)
    }

    /// Whether the pointer is within `r` pixels of a curve (sampled as the painter draws it).
    fn hit_curve(&self, o: &crate::doc::Obj, px: f64, py: f64, r: f64) -> bool {
        let (p, c, q) = crate::curve::shape(self.doc(), o);
        let pts: Vec<(f64, f64)> = (0..=crate::curve::CURVE_STEPS)
            .map(|i| {
                let t = f64::from(i) / f64::from(crate::curve::CURVE_STEPS);
                let d = crate::curve::at(p, c, q, t);
                self.view.to_px(d.0, d.1)
            })
            .collect();
        pts.windows(2).any(|w| {
            let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            let t = (((px - w[0].0) * dx + (py - w[0].1) * dy) / (dx * dx + dy * dy).max(1e-9))
                .clamp(0.0, 1.0);
            m::hypot(w[0].0 + t * dx - px, w[0].1 + t * dy - py) < r
        })
    }

    /// The object under the pointer: near the shaft of an arrow, on a plus sign, or on the line of a text.
    fn hit_object(&self, px: f64, py: f64, r: f64) -> Hit {
        let font = self.style.metrics(self.view.scale, true).font;
        for o in self.doc().objects.iter().rev() {
            let (x, y) = self.view.to_px(o.x, o.y);
            let on = match o.t {
                ObjKind::Arrow => {
                    let e = self.view.to_px(o.x2, o.y2);
                    let (dx, dy) = (e.0 - x, e.1 - y);
                    let t = (((px - x) * dx + (py - y) * dy) / (dx * dx + dy * dy).max(1e-9))
                        .clamp(0.0, 1.0);
                    m::hypot(x + t * dx - px, y + t * dy - py) < r
                }
                ObjKind::Curve => self.hit_curve(o, px, py, r),
                ObjKind::Plus => m::hypot(x - px, y - py) < r + font * 0.4,
                ObjKind::Text => {
                    let (w, lines) = self.text_box(o);
                    px >= x - r
                        && px <= x + w + r
                        && py >= y - font * 0.6 - r * 0.5
                        && py <= y + (lines - 1) as f64 * font * 1.25 + font * 0.6 + r * 0.5
                }
            };
            if on {
                return Hit::Obj(o.id);
            }
        }
        Hit::None
    }

    /// Zooms by `factor` keeping the point under the pointer still (scale 8-240 px per bond).
    pub fn zoom(&mut self, px: f64, py: f64, factor: f64) -> u32 {
        // a number that is not finite (or a factor that is not positive) would poison the view for good
        if ![px, py, factor].iter().all(|v| v.is_finite()) || factor <= 0.0 {
            return 0;
        }
        let (x, y) = self.view.to_doc(px, py);
        self.view.scale = (self.view.scale * factor).clamp(8.0, 240.0);
        self.view.ox = px - x * self.view.scale;
        self.view.oy = py + y * self.view.scale;
        REPAINT
    }

    pub fn pan(&mut self, dx: f64, dy: f64) -> u32 {
        self.view.ox += dx;
        self.view.oy += dy;
        REPAINT
    }

    /// Tells the editor the size of the canvas in pixels (a size that is not finite and positive is ignored). The view does not change.
    pub fn set_canvas(&mut self, w: f64, h: f64) {
        if w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0 {
            self.canvas = (w, h);
        }
    }

    /// Centres the drawing in a `w` x `h` canvas, at most 40 px per bond.
    pub fn fit(&mut self, w: f64, h: f64) -> u32 {
        self.set_canvas(w, h);
        let Some((x0, y0, x1, y1)) = self.doc().bounds() else {
            self.view = View {
                scale: 40.0,
                ox: w / 2.0,
                oy: h / 2.0,
            };
            return REPAINT;
        };
        let scale = ((w - 80.0) / (x1 - x0).max(0.1))
            .min((h - 80.0) / (y1 - y0).max(0.1))
            .clamp(8.0, 40.0);
        self.view = View {
            scale,
            ox: w / 2.0 - (x0 + x1) / 2.0 * scale,
            oy: h / 2.0 + (y0 + y1) / 2.0 * scale,
        };
        REPAINT
    }

    /// The drawing as a text: `mzdraw` (the document), `molfile` (V2000, V3000 when it has to be), `molfile-v3000`, `smiles`, `ket`, `cdxml`, `cml`, `rxn` (a drawing with one arrow), `trasf` (the record `trasf/1` of one structure before and one after the arrow, with its two `mol/1` records), `sdf` (one record per structure, with its data),
    /// `svg` (in the style of the editor), `pdf` (base64 of the PDF file), `log` (the command log), `std` (the drawing checks, `conform::report`, read-only) or `loss:FORMAT` (what that format drops of the drawing, `loss/1`). The error is an i18n key.
    pub fn save(&self, kind: &str) -> Result<String, &'static str> {
        Ok(match kind {
            k if crate::brand::is_doc_kind(k) => self.doc().to_json(),
            "molfile" => self.doc().to_molfile(),
            "molfile-v3000" => self.doc().to_molfile_v3000(),
            "smiles" => self.doc().to_smiles()?,
            "ket" => self.doc().to_ket(),
            "cdxml" => self.doc().to_cdxml_with(&crate::values::captions_titled(
                self.doc(),
                self.style.values,
                &self.style.adducts,
                &self.measured,
                self.style.titles,
            )),
            "cml" => self.doc().to_cml(),
            "rxn" => self.doc().to_rxn()?,
            "trasf" => self.doc().to_trasf().map_err(|e| e.key)?,
            "sdf" => self.doc().to_sdf(),
            "svg" => self.svg(),
            "pdf" => crate::pdf::base64(&self.pdf()),
            "log" => self.session.log_json(),
            "history" => self.history_json(),
            // the settings file: the style, the layout of the palettes and the export defaults
            "settings" => self.settings_json(),
            "std" => crate::pathway::report_with(self.doc(), &self.style).to_json(),
            "boxes" => crate::pathway::boxes_json(self.doc(), &self.style),
            "lens" => crate::tp::lenses(self.doc())?.to_json(),
            "digest" => crate::digest::json(self.doc(), &self.style)?,
            "schema" => crate::schema::json(),
            "steps" => crate::tp::steps_json(),
            // the TP table of the pathway (tp-09): `tptable/1` and the same rows as CSV; the notes of the last one are kept for `lost_on_save`
            "tptable" | "tptable-csv" => {
                let t = crate::tptable::build(self.doc(), &self.style, self.measurer)?;
                let mut notes = t.counts();
                let text = if kind == "tptable" {
                    crate::tptable::json(&t)
                } else {
                    let (text, guarded) = crate::tptable::csv_counted(&t);
                    if guarded > 0 {
                        notes.push(("draw2.export.tpGuard", guarded));
                    }
                    text
                };
                *self.table_notes.borrow_mut() = notes;
                text
            }
            // the checks of a pathway drawing (`pathway-check/1`): formulas, deltas against the labels, expected ions, layout
            "pathway-check" => self.pathway_check()?,
            // `loss:FORMAT`: what that format would drop or change of this drawing (`loss/1`)
            k if k.starts_with("loss:") => {
                let target = &k["loss:".len()..];
                crate::loss::json(target, &crate::loss::loss(self.doc(), target)?)
            }
            _ => return Err("error.draw2.request"),
        })
    }

    /// Why `save("trasf")` refuses the drawing, if it does: the i18n key and the ids of the atoms it is about (the unmapped ones for
    /// `error.draw2.trasfMap`).
    pub fn trasf_problem(&self) -> Option<(&'static str, Vec<u32>)> {
        self.doc().trasf().err().map(|e| (e.key, e.atoms))
    }

    /// What `save(kind)` leaves out because the format has no place for it, as (i18n key, count): atom maps for CDXML and CML.
    pub fn lost_on_save(&self, kind: &str) -> Vec<(&'static str, usize)> {
        // the notes of the TP table written by the last `save` (cells left empty, text cells guarded): nothing is dropped in silence
        if kind.starts_with("tptable") {
            return self.table_notes.borrow().clone();
        }
        // the notes of the last `pathway:label` (a structure the engine could not measure, a charged product with no m/z)
        [
            ("draw2.export.mapsLost", self.doc().maps_lost(kind)),
            ("draw2.export.coloursLost", self.doc().colours_lost(kind)),
            ("draw2.export.captionsLost", self.doc().caps_lost(kind)),
        ]
        .into_iter()
        .filter(|l| l.1 > 0)
        .chain(
            self.label_notes
                .iter()
                .copied()
                .filter(|_| self.doc().to_json() == self.label_notes_doc),
        )
        .collect()
    }

    /// Adds the structure of a text (SMILES, molfile or SDF, `mzdraw/1`) to the right of the drawing, as one undo step.
    pub fn import(&mut self, text: &str) -> Result<io::Imported, &'static str> {
        if crate::pathmodel::looks_like(text) {
            return self.import_model(text);
        }
        if crate::tp_list::looks_like(text) {
            return self.import_list(text);
        }
        self.import_placed(text).map(|r| r.0)
    }

    /// A list of transformation products (`tp_list`: a CSV with the header `smiles,name,parent,...`) as the Add field and the file
    /// button take it: the command `tp:import` as one undo step. A refusal adds nothing and says the lines of the failing rows.
    fn import_list(&mut self, text: &str) -> Result<io::Imported, &'static str> {
        let rows = text.lines().count().saturating_sub(1);
        match crate::commands::run(self, "tp:import", &serde_json::json!({ "csv": text })) {
            Ok(()) => {
                self.notice = ("", String::new());
                Ok(io::Imported {
                    format: "tp-list",
                    records: rows,
                    table_ignored: crate::tp_list::table_ignored(text),
                    ..io::Imported::default()
                })
            }
            Err(e) => {
                if !self.failed_rows.is_empty() {
                    let lines: Vec<String> = (self.failed_rows.iter().take(10))
                        .map(|r| r.0.to_string())
                        .collect();
                    let more = if self.failed_rows.len() > 10 {
                        ", \u{2026}"
                    } else {
                        ""
                    };
                    self.notice = ("draw2.say.tpRows", format!("{}{more}", lines.join(", ")));
                }
                Err(e.key)
            }
        }
    }

    /// The words (tentative, not detected) the next pathway model is drawn with; the page passes those of its language. Empty texts or texts
    /// above 32 characters are ignored (the painter's English stays).
    pub fn set_status_words(&mut self, tentative: &str, not_detected: &str) {
        let ok =
            |s: &str| !s.is_empty() && s.chars().count() <= 32 && !s.chars().any(char::is_control);
        self.status_words = (ok(tentative) && ok(not_detected))
            .then(|| (tentative.to_string(), not_detected.to_string()));
    }

    /// A pathway model `pathway/1` (`pathmodel`) as the Add field and the file button take it: the command `pathway:build`, one undo step. A
    /// refusal adds nothing; the lines of the nodes and edges that failed are said as for a TP list.
    fn import_model(&mut self, text: &str) -> Result<io::Imported, &'static str> {
        let model: serde_json::Value =
            serde_json::from_str(text).map_err(|_| crate::pathmodel::KEY_MODEL)?;
        let nodes = model["nodes"].as_array().map_or(0, Vec::len);
        let mut args = serde_json::json!({ "model": model });
        if let Some((t, n)) = &self.status_words {
            args["words"] = serde_json::json!({ "tentative": t, "not_detected": n });
        }
        match crate::commands::run(self, "pathway:build", &args) {
            Ok(()) => {
                self.notice = ("", String::new());
                Ok(io::Imported {
                    format: "pathway",
                    records: nodes,
                    ..io::Imported::default()
                })
            }
            Err(e) => {
                if !self.failed_rows.is_empty() {
                    let lines: Vec<String> = (self.failed_rows.iter().take(10))
                        .map(|r| r.0.to_string())
                        .collect();
                    let more = if self.failed_rows.len() > 10 {
                        ", \u{2026}"
                    } else {
                        ""
                    };
                    self.notice = ("draw2.say.tpRows", format!("{}{more}", lines.join(", ")));
                }
                Err(e.key)
            }
        }
    }

    /// `import` for the bytes of a file: also a PNG saved by mzDraw, which carries its document.
    pub fn import_bytes(&mut self, bytes: &[u8]) -> Result<io::Imported, &'static str> {
        if let Ok(text) = std::str::from_utf8(bytes) {
            if crate::pathmodel::looks_like(text) {
                return self.import_model(text);
            }
            if crate::tp_list::looks_like(text) {
                return self.import_list(text);
            }
        }
        let read = io::import_bytes(bytes)?;
        self.add_read(read).map(|r| r.0)
    }

    /// The PNG of the drawing in the editor's style, for the surface that has a rasteriser (`png::plan`).
    pub fn png_plan(&self) -> crate::png::Plan {
        crate::png::plan_with(self.doc(), &self.style, &self.measured)
    }

    /// `import`, also giving the ids of what was added.
    pub(crate) fn import_placed(
        &mut self,
        text: &str,
    ) -> Result<(io::Imported, Placed), &'static str> {
        let read = io::import(text)?;
        self.add_read(read)
    }

    fn add_read(
        &mut self,
        (new, info): (Document, io::Imported),
    ) -> Result<(io::Imported, Placed), &'static str> {
        let right = self.doc().bounds().map(|b| b.2);
        let left = new.bounds().map(|b| b.0);
        let dx = match (right, left) {
            (Some(r), Some(l)) => r + 2.0 - l,
            _ => 0.0,
        };
        let placed = self.place(&new, dx, 0.0)?;
        self.said = ("draw2.say.imported", new.atoms.len().to_string());
        self.notice = ("", String::new());
        Ok((info, placed))
    }

    /// Adds every atom, bond and object of `new`, moved by (`dx`, `dy`), with fresh ids, as one undo step; returns the new ids.
    pub(crate) fn place(
        &mut self,
        new: &Document,
        dx: f64,
        dy: f64,
    ) -> Result<Placed, &'static str> {
        let base = self.doc().next - 1;
        let ops = place_ops(base, new, dx, dy);
        self.session.run(&ops).map_err(|e| e.key)?;
        Ok(Placed {
            atoms: new.atoms.iter().map(|a| base + a.id).collect(),
            bonds: new.bonds.iter().map(|b| base + b.id).collect(),
            objects: new.objects.iter().map(|o| base + o.id).collect(),
        })
    }

    /// Opens a saved `mzdraw/1` document in place of the drawing: the same bytes, ids included, with an empty history (there is
    /// nothing to undo back to). The view, the tool and the clipboard stay. The error is an i18n key and changes nothing.
    pub fn open(&mut self, text: &str) -> Result<(), &'static str> {
        let doc = Document::from_json(text.trim())?;
        self.session = Session::new(doc);
        self.over = Overlay::default();
        self.press = None;
        self.nudge = None;
        self.said = ("draw2.say.opened", self.doc().atoms.len().to_string());
        self.notice = ("", String::new());
        self.error = "";
        Ok(())
    }

    /// Empties the canvas as one undo step.
    pub fn clear(&mut self) -> u32 {
        let atoms: Vec<u32> = self.doc().atoms.iter().map(|a| a.id).collect();
        let objects: Vec<u32> = self.doc().objects.iter().map(|o| o.id).collect();
        if atoms.is_empty() && objects.is_empty() {
            return 0;
        }
        self.run(
            &[Op::Remove {
                atoms,
                bonds: Vec::new(),
                objects,
            }],
            "draw2.say.cleared",
            "",
        )
    }

    /// Sets the drawing style from a preset name, or the whole settings from the text of a settings file (its `layout` and
    /// `export` too: a file without them gives their defaults); a refusal is an i18n key and changes nothing.
    pub fn set_style(&mut self, text: &str) -> Result<u32, &'static str> {
        let mut style = Style::from_text(text)?;
        let mut flags = REPAINT;
        if text.trim_start().starts_with('{') {
            let mut take = |key: &str| {
                let file = &mut style.unknown.file;
                let i = file.iter().position(|(k, _)| k == key)?;
                Some(file.remove(i).1)
            };
            let value = |t: String, e| serde_json::from_str::<serde_json::Value>(&t).map_err(|_| e);
            let layout = match take("layout") {
                Some(t) => crate::palettes::Layout::read(&value(t, "error.draw2.layout")?)?,
                None => Default::default(),
            };
            let ui = match take("ui") {
                Some(t) => crate::prefs::Ui::read(&value(t, "error.draw2.ui")?)?,
                None => Default::default(),
            };
            let textp = match take("text") {
                Some(t) => crate::prefs::TextPrefs::read(&value(t, "error.draw2.textPref")?)?,
                None => Default::default(),
            };
            let export = match take("export") {
                Some(t) => crate::palettes::Export::read(&value(t, "error.draw2.export")?)?,
                None => Default::default(),
            };
            self.layout.replace(layout);
            self.export = export;
            self.ui = ui;
            self.textp = textp;
            flags |= PALETTES;
        }
        self.style = style;
        self.said = ("draw2.say.style", self.style.name.clone());
        Ok(flags)
    }

    /// What a click on the button of a group of the tools palette does: it selects the face when that is not the tool in use already
    /// (a group with a face of its own icon selects nothing); the page then opens the list. The flags like `set_tool`.
    pub fn group_click(&mut self, group: &str) -> u32 {
        match self.layout.face(group).filter(|f| !f.is_empty()) {
            Some(face) if face != self.tool.name() => self.set_tool(&face),
            _ => 0,
        }
    }

    /// `set_style` for a settings file kept in the browser: a `ui` object that is not one this version can read (a theme that is none, a
    /// value that is not text) is dropped with the note `error.draw2.ui` and the rest of the file is taken; a file that is refused for
    /// any other reason is refused whole. An explicit `set` of a bad value stays an error. Returns the flags and the note ("" for none).
    pub fn set_style_stored(&mut self, text: &str) -> Result<(u32, &'static str), &'static str> {
        let (flags, notes) = self.set_style_stored_notes(text)?;
        Ok((flags, notes.first().copied().unwrap_or("")))
    }

    /// `set_style_stored` with every note it has, in a fixed order (an unreadable `ui`, `text`, a face of a group of the tools palette
    /// that this version cannot show, then the layout of the earlier palettes): one note never hides another.
    pub fn set_style_stored_notes(
        &mut self,
        text: &str,
    ) -> Result<(u32, Vec<&'static str>), &'static str> {
        let mut notes: Vec<&'static str> = Vec::new();
        let mut text = text.to_string();
        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&text) {
            let bad = v
                .get("ui")
                .is_some_and(|ui| crate::prefs::Ui::read(ui).is_err());
            if let (true, Some(o)) = (bad, v.as_object_mut()) {
                o.remove("ui");
                text = v.to_string();
                notes.push("error.draw2.ui");
            }
            // a `text` object of a newer file (a font or a size this version does not know) is dropped the same way: the rest is taken
            let bad = v
                .get("text")
                .is_some_and(|t| crate::prefs::TextPrefs::read(t).is_err());
            if let (true, Some(o)) = (bad, v.as_object_mut()) {
                o.remove("text");
                text = v.to_string();
                notes.push("error.draw2.textPref");
            }
            // a face of a group that a stored file names but this version cannot show (a member that was removed or is in view on its
            // own now, a group with no memory) is dropped; the other faces and the rest of the file are taken. `--set` stays strict.
            if let Some(heads) = v.pointer_mut("/layout/heads") {
                let before = heads.clone();
                match heads.as_object_mut() {
                    Some(o) => o.retain(|g, m| {
                        crate::tool_groups::Group::named(g).is_none()
                            || m.as_str().is_some_and(|m| {
                                crate::tool_groups::Heads::default().set(g, m).is_ok()
                            })
                    }),
                    None => *heads = serde_json::json!({}),
                }
                if *heads != before {
                    text = v.to_string();
                    notes.push("draw2.layout.heads");
                }
            }
        }
        let flags = self.set_style(&text)?;
        // a layout of the palettes before the general tools palette: its tools take the new default place, and the page says so
        if self.layout.migrated {
            notes.push("draw2.layout.migrated");
        }
        Ok((flags, notes))
    }

    /// The settings file of the editor: the style, the layout of the palettes and the export defaults (the last two only
    /// when they differ from their defaults).
    pub fn settings_json(&self) -> String {
        let mut s = self.style.clone();
        for (key, v) in [
            ("layout", self.layout.to_text()),
            ("export", self.export.to_text()),
            ("ui", self.ui.to_text()),
            ("text", self.textp.to_text()),
        ] {
            if let Some(v) = v {
                s.unknown.file.retain(|(k, _)| k != key);
                s.unknown.file.push((key.to_string(), v));
            }
        }
        s.to_json()
    }

    /// `set_map` from the text of a field: it must be a whole number (digits only); anything else (empty, a fraction, a sign, a word) is
    /// refused with `error.draw2.map` and changes nothing.
    pub fn set_map_text(&mut self, text: &str) -> u32 {
        let t = text.trim();
        match t.parse::<u16>() {
            Ok(n) if t.bytes().all(|b| b.is_ascii_digit()) => self.set_map(n),
            _ => {
                self.error = "error.draw2.map";
                REPAINT
            }
        }
    }

    /// Sets the atom-to-atom map of the atom that has the focus, or else of the only selected atom, as one undo step; 0 takes it
    /// away. A number above `doc::MAX_MAP`, or no atom to act on, changes nothing (`error.draw2.map` in `error`).
    pub fn set_map(&mut self, map: u16) -> u32 {
        let target = (self.over.focus)
            .or_else(|| match self.over.sel_atoms[..] {
                [only] => Some(only),
                _ => None,
            })
            .filter(|&a| self.doc().atom(a).is_some());
        let Some(id) = target.filter(|_| map <= crate::doc::MAX_MAP) else {
            self.error = "error.draw2.map";
            return REPAINT;
        };
        let set = Op::AtomSet {
            id,
            el: None,
            chg: None,
            iso: None,
            rad: None,
            h: None,
            lab: None,
            map: Some(map),
            lp: None,
        };
        self.run(&[set], "draw2.say.map", &map.to_string())
    }

    /// Sets the data of the structure the atom `anchor` belongs to (the data fields of its SDF record), as one undo step: the
    /// list is kept on `anchor` and taken off the other atoms of the structure. A list an SDF could not hold is refused
    /// (`error.draw2.data` in `error`) and changes nothing.
    pub fn set_data(&mut self, anchor: u32, data: Vec<(String, String)>) -> u32 {
        let piece = crate::layout::pieces(self.doc())
            .into_iter()
            .find(|p| p.contains(&anchor));
        let mut ops: Vec<Op> = (self.doc().atoms.iter())
            .filter(|a| {
                a.id != anchor
                    && piece.as_ref().is_some_and(|p| p.contains(&a.id))
                    && !a.data().is_empty()
            })
            .map(|a| Op::AtomData {
                id: a.id,
                data: Vec::new(),
            })
            .collect();
        let n = data.len().to_string();
        ops.push(Op::AtomData { id: anchor, data });
        self.run(&ops, "draw2.say.data", &n)
    }

    /// Annotates the structure the atom `anchor` belongs to, as one undo step: a new entry (`ann::entry`: the level, an
    /// identifier, the evidence as lines `kind: ref`, a note; decided by the chemist) is added after the entries the structure
    /// has, and the whole annotation is kept on `anchor`. The `scale` the level is read on (`ann::SCALES`; empty: the default) is
    /// written with the entry. An empty `level` takes the annotation of the structure away. An entry
    /// that is not of its shape is refused (`error.draw2.ann` in `error`) and changes nothing.
    pub fn annotate(
        &mut self,
        anchor: u32,
        scale: &str,
        level: &str,
        identifier: &str,
        evidence: &str,
        note: &str,
    ) -> u32 {
        let scale = Some(scale.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or(crate::ann::SCALE);
        let piece = crate::layout::pieces(self.doc())
            .into_iter()
            .find(|p| p.contains(&anchor))
            .unwrap_or_default();
        let held: Vec<(u32, String)> = (self.doc().atoms.iter())
            .filter(|a| piece.contains(&a.id))
            .map(|a| (a.id, a.ann()))
            .filter(|a| !a.1.is_empty())
            .collect();
        let level = level.trim();
        let new = crate::ann::entry(scale, level, identifier, evidence, note, "chemist");
        let list = if level.is_empty() {
            "[]".to_string()
        } else {
            let old = held.iter().map(|a| a.1.as_str());
            crate::ann::join(old.chain(new.as_deref()))
        };
        // refused: an entry that is not of its shape, a history over its limits, an atom that is none
        let fine = (level.is_empty() || new.is_some()) && !piece.is_empty();
        let Some(ann) = crate::ann::canon(&list).filter(|_| fine) else {
            self.error = "error.draw2.ann";
            return REPAINT;
        };
        let mut ops: Vec<Op> = (held.iter().filter(|a| a.0 != anchor))
            .map(|a| Op::AtomAnn {
                id: a.0,
                ann: "[]".into(),
            })
            .collect();
        ops.push(Op::AtomAnn { id: anchor, ann });
        self.run(&ops, "draw2.say.ann", level)
    }

    /// Sets one setting from its text, `<section>.<key>=<value>` as the `--set` of the CLI writes it. The sections are `style`
    /// (`Style::set`), `export` (`Export::set`) and `layout` (`Layout::set`; `layout=<object>` sets the whole object). The
    /// error is an i18n key and changes nothing.
    pub fn set(&mut self, setting: &str) -> Result<u32, &'static str> {
        let (name, value) = setting.split_once('=').ok_or("error.draw2.set")?;
        let name = name.trim();
        if name == "layout" {
            let v = serde_json::from_str(value).map_err(|_| "error.draw2.layout")?;
            self.layout.replace(crate::palettes::Layout::read(&v)?);
            return Ok(PALETTES);
        }
        let (section, key) = name.split_once('.').ok_or("error.draw2.set")?;
        match section {
            "style" => {
                self.style.set(key, value)?;
                self.said = ("draw2.say.style", self.style.name.clone());
                Ok(REPAINT)
            }
            "display" => self.style.set_display(key, value).map(|_| REPAINT),
            "pathway" => self.style.set_pathway(key, value).map(|_| 0),
            "export" => self.export.set(key, value).map(|_| 0),
            "ui" => self.ui.set(key, value).map(|_| 0),
            "text" => self.textp.set(key, value).map(|_| 0),
            "layout" => self.layout.set(key, value).map(|_| PALETTES),
            _ => Err("error.draw2.set"),
        }
    }

    /// The value of a setting as text (`<section>.<key>`: the shape of `set`), `None` for one that is none.
    pub fn get(&self, setting: &str) -> Option<String> {
        match setting.split_once('.')? {
            ("export", "default") => Some(self.export.default.clone()),
            ("export", "background") => Some(self.export.background.clone()),
            ("ui", "theme") => Some(self.ui.theme.clone()),
            ("text", "font") => Some(self.textp.font.clone()),
            ("text", "size") => Some(self.textp.size.clone()),
            _ => self.style.value(setting),
        }
    }

    /// The settings that put a group of the preferences back to its defaults, in the order to apply them: the default of each
    /// option of the group, and then, when the group holds the preset (which replaces the whole style), the current value of every
    /// option of the other groups that the preset would change. `None` for a group that is none. The page applies the list with
    /// `set` one by one; a script can send the same list as the `set` field of a replay request.
    pub fn reset_commands(&self, group: &str) -> Option<Vec<String>> {
        if !crate::prefs::GROUPS.iter().any(|g| g.0 == group) {
            return None;
        }
        let rows = crate::prefs::all();
        let mut out: Vec<String> = Vec::new();
        let mut after = self.style.clone();
        for r in rows.iter().filter(|r| r.group == group) {
            let def = crate::prefs::default_of(&r.setting)?;
            let _ = after.apply(&r.setting, &def);
            out.push(format!("{}={def}", r.setting));
        }
        for r in rows.iter().filter(|r| r.group != group) {
            let (now, then) = (self.style.value(&r.setting), after.value(&r.setting));
            if now != then {
                out.push(format!("{}={}", r.setting, now?));
            }
        }
        Some(out)
    }

    /// Puts a group of the preferences back to its defaults (`reset_commands`), the settings applied one by one; a refusal stops
    /// at the setting that failed and says its error key.
    pub fn reset_group(&mut self, group: &str) -> Result<u32, &'static str> {
        let list = self.reset_commands(group).ok_or("error.draw2.set")?;
        let mut flags = 0;
        for s in list {
            flags |= self.set(&s)?;
        }
        Ok(flags | REPAINT)
    }

    /// The drawing as an SVG in the editor's style, carrying its document (`svg::carrying`).
    pub fn svg(&self) -> String {
        crate::svg::carrying(
            crate::svg::to_svg_measured(self.doc(), &self.style, &self.measured),
            self.doc(),
        )
    }

    /// The drawing as a PDF file in the editor's style, carrying its document (`pdf::to_pdf`).
    pub fn pdf(&self) -> Vec<u8> {
        crate::pdf::to_pdf_with(self.doc(), &self.style, &self.measured)
    }

    /// Marks the atoms a check flagged (ids), replacing the previous marks.
    pub fn set_warnings(&mut self, ids: Vec<u32>) {
        self.over.warn_atoms = ids;
    }

    pub fn scene(&self) -> Scene {
        let metrics = self.style.metrics(self.view.scale, true);
        let over = Overlay {
            marks: if self.live_proposal().is_some() {
                self.proposal_marks()
            } else if self.step_field.is_some() {
                self.step_marks()
            } else {
                self.over.marks.clone()
            },
            captions: crate::values::captions_titled(
                self.doc(),
                self.style.values,
                &self.style.adducts,
                &self.measured,
                self.style.titles,
            ),
            ..self.over.clone()
        };
        scene::build(self.doc(), self.view, &over, metrics)
    }

    /// Asks the engine for the calculated values of every structure and of the selection (`values`), after a change or a change of
    /// the adducts. Without an engine, or above the caps, nothing is held (the formulas are always there, from the record).
    pub fn refresh_values(&mut self) {
        self.measured_sel = None;
        let Some(engine) = self.measurer else {
            self.measured.clear();
            self.measured_error = "";
            return;
        };
        if self.doc().atoms.is_empty() {
            self.measured.clear();
            self.measured_error = "";
            return;
        }
        let scan = crate::values::scan(self.doc());
        // a structure that is as it was (anchor, formula, isotopes) and the same adducts keep what the engine gave: a mass does not
        // depend on where an atom is drawn, and an edit of one structure does not ask again for the others
        let prev = if self.measured_adducts == self.style.adducts && self.measured_error.is_empty()
        {
            std::mem::take(&mut self.measured)
        } else {
            Vec::new()
        };
        self.measured.clear();
        self.measured_error = "";
        self.measured_adducts = self.style.adducts.clone();
        match scan.measure_cached(&self.style.adducts, engine, &prev) {
            Ok(entries) => self.measured = entries,
            Err(key) => self.measured_error = key,
        }
        let ids = self.over.sel_atoms.clone();
        if let Some(sel) = scan.selection(self.doc(), &ids) {
            let r = sel.measure(&self.style.adducts, engine);
            self.measured_sel = Some((ids, sel, r));
        }
    }

    /// Recomputes only the numbers of the selection, when the atoms selected are not the ones they were found for (a selection is not a
    /// change of the drawing, so `refresh_values` did not run).
    pub fn refresh_selection_values(&mut self) {
        let Some(engine) = self.measurer else {
            return;
        };
        let held = self.measured_sel.as_ref().map(|s| &s.0);
        if held == Some(&self.over.sel_atoms) || (held.is_none() && self.over.sel_atoms.is_empty())
        {
            return;
        }
        self.measured_sel = None;
        let ids = self.over.sel_atoms.clone();
        let scan = crate::values::scan(self.doc());
        if let Some(sel) = scan.selection(self.doc(), &ids) {
            let r = sel.measure(&self.style.adducts, engine);
            self.measured_sel = Some((ids, sel, r));
        }
    }

    /// The `values` of the page: `{"adducts", "error", "structures": [{numbers of each structure, in the order of the outline}],
    /// "selection": {...} | null}`; the numbers are the ones `refresh_values` found, `null` where there are none.
    pub fn values_json(&self) -> String {
        let doc = self.doc();
        let scan = crate::values::scan(doc);
        let rows: Vec<String> = (scan.structures.iter())
            .map(|s| {
                let r = (self.measured.iter())
                    .find(|e| e.matches(s))
                    .map(|e| &e.result);
                crate::values::structure_json(s, r, &self.style.adducts)
            })
            .collect();
        let selection = match &self.measured_sel {
            Some((ids, sel, r)) => {
                crate::values::selection_json(sel, ids, Some(r), &self.style.adducts)
            }
            None => "null".to_string(),
        };
        let names: Vec<String> = (self.style.adducts.iter())
            .map(|a| serde_json::Value::from(a.as_str()).to_string())
            .collect();
        format!(
            r#"{{"adducts":[{}],"error":{},"structures":[{}],"selection":{selection}}}"#,
            names.join(","),
            serde_json::Value::from(self.measured_error),
            rows.join(",")
        )
    }
}
