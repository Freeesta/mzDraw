//! The palettes of the page and the export defaults: the objects `layout` and `export` of the settings file `mzdraw-settings/1`
//! (`docs/mzdraw-ui.md` §4-§5). Every group of actions is a palette that is docked on an edge of the page or floats over it.
//! Where each palette is, its orientation, its order along an edge, snapping and clamping are decided here; the page sends
//! the measured size of each palette and the pointer or key events on its handle, and paints the rectangles it gets back.
//!
//! A layout is a preset (chosen by the width of the viewport unless one is named) with the user's changes on top, one entry
//! per palette that differs; only the changes are written, so a file stays short and a new preset still applies to what the
//! user did not move. Keys and palette ids this version does not know are kept and written back.

use crate::doc::Writer;
use serde_json::Value;

/// An edge of the page a palette is docked on, or none (`Float`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dock {
    Left,
    Right,
    Top,
    Bottom,
    Float,
}

impl Dock {
    pub fn name(self) -> &'static str {
        match self {
            Dock::Left => "left",
            Dock::Right => "right",
            Dock::Top => "top",
            Dock::Bottom => "bottom",
            Dock::Float => "float",
        }
    }

    fn parse(s: &str) -> Option<Dock> {
        [
            Dock::Left,
            Dock::Right,
            Dock::Top,
            Dock::Bottom,
            Dock::Float,
        ]
        .into_iter()
        .find(|d| d.name() == s)
    }

    /// A palette docked at the top or the bottom is a row; left or right, a column.
    fn horizontal(self) -> Option<bool> {
        match self {
            Dock::Top | Dock::Bottom => Some(true),
            Dock::Left | Dock::Right => Some(false),
            Dock::Float => None,
        }
    }
}

/// The built-in layouts, by the width of the viewport: `desktop` from 900 px, `tablet` from 600 px, `phone` below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Desktop,
    Tablet,
    Phone,
}

impl Preset {
    pub fn name(self) -> &'static str {
        match self {
            Preset::Desktop => "desktop",
            Preset::Tablet => "tablet",
            Preset::Phone => "phone",
        }
    }

    fn parse(s: &str) -> Option<Preset> {
        [Preset::Desktop, Preset::Tablet, Preset::Phone]
            .into_iter()
            .find(|p| p.name() == s)
    }

    pub fn for_width(w: f64) -> Preset {
        if w >= 900.0 {
            Preset::Desktop
        } else if w >= 600.0 {
            Preset::Tablet
        } else {
            Preset::Phone
        }
    }
}

/// `v` kept between `lo` and `hi` (`lo <= hi`); NaN gives `lo`. Not `f64::clamp`, whose check of its bounds links the
/// formatting of floats into the module: 22,762 B of `.wasm`, measured on 2026-11-21 (620,312 B with it, 597,550 B without).
fn within(v: f64, lo: f64, hi: f64) -> f64 {
    // a NaN compares false both ways: it is taken as the low bound, never passed on
    if v.is_nan() || v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

/// The largest viewport side, in pixels: what the page sends is kept within it, so every position written stays within what
/// `Layout::read` takes (1e6).
const MAX_VIEW: f64 = 100_000.0;

/// A point of the page, or none when a coordinate is not a finite number.
fn point(x: f64, y: f64) -> Option<(f64, f64)> {
    (x.is_finite() && y.is_finite()).then(|| {
        (
            within(x, -MAX_VIEW, MAX_VIEW),
            within(y, -MAX_VIEW, MAX_VIEW),
        )
    })
}

/// The palettes of the page in their desktop place: id, edge, order along it, collapsed to its head.
pub const PALETTES: [(&str, Dock, u32, bool); 16] = [
    ("tools", Dock::Left, 1, false),
    ("bonds", Dock::Left, 2, true),
    ("rings", Dock::Left, 3, true),
    ("atoms", Dock::Left, 4, true),
    ("charges", Dock::Left, 5, true),
    ("objects", Dock::Left, 6, true),
    ("file", Dock::Top, 1, false),
    ("history", Dock::Top, 2, false),
    ("edit", Dock::Top, 3, true),
    ("view", Dock::Top, 4, false),
    ("structure", Dock::Right, 1, false),
    ("style", Dock::Right, 2, false),
    ("data", Dock::Right, 3, true),
    ("annotation", Dock::Right, 4, true),
    ("check", Dock::Right, 5, false),
    ("keys", Dock::Right, 6, true),
];

const SIDE: [&str; 6] = ["structure", "style", "data", "annotation", "check", "keys"];

/// One palette as it is: where, how and whether it shows. `x`, `y` are page pixels of a floating palette.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub id: String,
    pub dock: Dock,
    pub order: u32,
    pub x: f64,
    pub y: f64,
    pub horizontal: bool,
    pub collapsed: bool,
    pub hidden: bool,
}

/// The palettes of a preset.
pub fn preset(p: Preset) -> Vec<Palette> {
    PALETTES
        .iter()
        .map(|&(id, dock, order, collapsed)| {
            let side = SIDE.contains(&id);
            // a phone docks the tools in a second row at the top; tablet and phone hide the side groups (a drawer)
            let (dock, order) = match (p, dock) {
                (Preset::Phone, Dock::Left) => (Dock::Top, order + 10),
                _ => (dock, order),
            };
            Palette {
                id: id.to_string(),
                dock,
                order,
                x: 0.0,
                y: 0.0,
                horizontal: dock.horizontal().unwrap_or(false),
                collapsed,
                hidden: side && p != Preset::Desktop,
            }
        })
        .collect()
}

/// What the user changed of one palette, field by field, over the place its preset gives it. Kept apart from the preset, so a
/// change reads the same whatever preset the width of the viewport picks.
#[derive(Clone, Debug, Default, PartialEq)]
struct Change {
    id: String,
    dock: Option<Dock>,
    order: Option<u32>,
    at: Option<(f64, f64)>,
    horizontal: Option<bool>,
    collapsed: Option<bool>,
    hidden: Option<bool>,
    /// Keys of the entry this version does not know, as JSON text, written back as read.
    unknown: Vec<(String, String)>,
}

impl Change {
    fn apply(&self, p: &mut Palette) {
        p.dock = self.dock.unwrap_or(p.dock);
        p.order = self.order.unwrap_or(p.order);
        (p.x, p.y) = self.at.unwrap_or((p.x, p.y));
        p.horizontal = p
            .dock
            .horizontal()
            .or(self.horizontal)
            .unwrap_or(p.horizontal);
        p.collapsed = self.collapsed.unwrap_or(p.collapsed);
        p.hidden = self.hidden.unwrap_or(p.hidden);
    }
}

/// A palette laid out: its rectangle in page pixels and how to paint it.
#[derive(Clone, Debug, PartialEq)]
pub struct Rect {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub dock: Dock,
    pub horizontal: bool,
    pub collapsed: bool,
    pub hidden: bool,
}

/// A palette held by the pointer or moved by the keyboard: where the grab is on it and the palette as it was before.
#[derive(Clone, Debug)]
struct Grab {
    id: String,
    dx: f64,
    dy: f64,
    start: Palette,
    /// Held by a pointer (not a keyboard move): a key does not take it over.
    pointer: bool,
}

/// The layout: a preset (`None`: chosen by the width of the viewport) and the user's changes, by palette.
#[derive(Clone, Debug)]
pub struct Layout {
    pub preset: Option<Preset>,
    /// Distance in pixels under which a palette snaps to an edge or to another palette.
    pub snap: f64,
    /// Every palette hidden at once (the key `\`), to see the whole canvas.
    pub all_hidden: bool,
    /// What the user changed, by palette; a palette in its preset place has no entry.
    changed: Vec<Change>,
    /// Entries of palettes this version does not know and keys of the object it does not know, as JSON text.
    unknown_entries: Vec<String>,
    unknown_keys: Vec<(String, String)>,
    /// The viewport and the sizes of the palettes the page measured last.
    view: (f64, f64),
    sizes: Vec<(String, f64, f64)>,
    grab: Option<Grab>,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            preset: None,
            snap: 12.0,
            all_hidden: false,
            changed: Vec::new(),
            unknown_entries: Vec::new(),
            unknown_keys: Vec::new(),
            view: (1280.0, 800.0),
            sizes: Vec::new(),
            grab: None,
        }
    }
}

impl PartialEq for Layout {
    /// Two layouts are the same when they write the same settings.
    fn eq(&self, o: &Self) -> bool {
        self.to_text() == o.to_text()
    }
}

const ERR: &str = "error.draw2.layout";

impl Layout {
    /// Takes the settings of another layout and keeps what the page measured (the viewport and the sizes).
    pub fn replace(&mut self, other: Layout) {
        let (view, sizes) = (self.view, std::mem::take(&mut self.sizes));
        *self = Layout {
            view,
            sizes,
            ..other
        };
    }

    /// The preset in use.
    pub fn current(&self) -> Preset {
        self.preset.unwrap_or(Preset::for_width(self.view.0))
    }

    /// Every palette as it is now: the preset with the user's changes.
    pub fn palettes(&self) -> Vec<Palette> {
        let mut all = preset(self.current());
        for c in &self.changed {
            if let Some(p) = all.iter_mut().find(|p| p.id == c.id) {
                c.apply(p);
            }
        }
        all
    }

    fn palette(&self, id: &str) -> Option<Palette> {
        self.palettes().into_iter().find(|p| p.id == id)
    }

    /// Keeps what differs between a palette and its preset place, or forgets the change when there is no difference.
    fn put(&mut self, p: Palette) {
        let Some(b) = preset(self.current()).into_iter().find(|b| b.id == p.id) else {
            return;
        };
        let differs = |x: bool| x.then_some(());
        let float = p.dock == Dock::Float;
        let c = Change {
            id: p.id.clone(),
            dock: differs(p.dock != b.dock).map(|_| p.dock),
            order: differs(!float && p.order != b.order).map(|_| p.order),
            at: float.then_some((p.x, p.y)),
            horizontal: float.then_some(p.horizontal),
            collapsed: differs(p.collapsed != b.collapsed).map(|_| p.collapsed),
            hidden: differs(p.hidden != b.hidden).map(|_| p.hidden),
            unknown: (self.changed.iter().find(|x| x.id == p.id))
                .map_or(Vec::new(), |x| x.unknown.clone()),
        };
        self.changed.retain(|x| x.id != p.id);
        if c != (Change {
            id: p.id,
            ..Default::default()
        }) {
            self.changed.push(c);
        }
    }

    /// Reads the `layout` object of a settings file; a value of the wrong kind is refused with `error.draw2.layout`.
    pub fn read(v: &Value) -> Result<Layout, &'static str> {
        let o = v.as_object().ok_or(ERR)?;
        let mut l = Layout::default();
        for (k, x) in o {
            match k.as_str() {
                "preset" => {
                    let s = x.as_str().ok_or(ERR)?;
                    l.preset = if s == "auto" {
                        None
                    } else {
                        Some(Preset::parse(s).ok_or(ERR)?)
                    };
                }
                "snap" => l.snap = x.as_f64().filter(|s| (0.0..=64.0).contains(s)).ok_or(ERR)?,
                "all_hidden" => l.all_hidden = x.as_bool().ok_or(ERR)?,
                "palettes" => {
                    for e in x.as_array().ok_or(ERR)? {
                        let id = e.get("id").and_then(Value::as_str).ok_or(ERR)?;
                        if PALETTES.iter().all(|p| p.0 != id) {
                            l.unknown_entries.push(e.to_string());
                            continue;
                        }
                        let entry = e.as_object().ok_or(ERR)?;
                        let mut c = Change {
                            id: id.to_string(),
                            ..Default::default()
                        };
                        let (mut x, mut y) = (None, None);
                        for (f, v) in entry {
                            let num = || {
                                v.as_f64()
                                    .filter(|n| n.is_finite() && n.abs() <= 1e6)
                                    .ok_or(ERR)
                            };
                            match f.as_str() {
                                "dock" => {
                                    c.dock = Some(Dock::parse(v.as_str().ok_or(ERR)?).ok_or(ERR)?)
                                }
                                "order" => {
                                    c.order =
                                        Some(v.as_u64().filter(|n| *n <= 1000).ok_or(ERR)? as u32)
                                }
                                "x" => x = Some(num()?),
                                "y" => y = Some(num()?),
                                "orient" => {
                                    c.horizontal = Some(match v.as_str() {
                                        Some("h") => true,
                                        Some("v") => false,
                                        _ => return Err(ERR),
                                    })
                                }
                                "collapsed" => c.collapsed = Some(v.as_bool().ok_or(ERR)?),
                                "hidden" => c.hidden = Some(v.as_bool().ok_or(ERR)?),
                                "id" => {}
                                _ => c.unknown.push((f.clone(), v.to_string())),
                            }
                        }
                        if c.dock == Some(Dock::Float) {
                            c.at = Some((x.unwrap_or(0.0), y.unwrap_or(0.0)));
                            c.horizontal = Some(c.horizontal.unwrap_or(false));
                        } else {
                            c.horizontal = None;
                        }
                        l.changed.retain(|o| o.id != c.id);
                        l.changed.push(c);
                    }
                }
                _ => l.unknown_keys.push((k.clone(), x.to_string())),
            }
        }
        Ok(l)
    }

    /// The `layout` object as JSON text: what was kept unknown, the preset, the snap distance and the changes only. `None`
    /// for a layout with nothing to say (no file grows a `layout` it does not need).
    pub fn to_text(&self) -> Option<String> {
        let mut o = Writer("{".into());
        for (k, v) in &self.unknown_keys {
            o.raw(k, v);
        }
        if let Some(p) = self.preset {
            o.text("preset", p.name());
        }
        if self.snap != 12.0 {
            o.num("snap", self.snap);
        }
        if self.all_hidden {
            o.raw("all_hidden", "true");
        }
        let mut list: Vec<String> = Vec::new();
        for c in &self.changed {
            let mut e = Writer("{".into());
            e.text("id", &c.id);
            if let Some(d) = c.dock {
                e.text("dock", d.name());
            }
            if let Some(n) = c.order {
                e.int("order", n as i64);
            }
            if let Some((x, y)) = c.at {
                e.num("x", x);
                e.num("y", y);
            }
            if let Some(h) = c.horizontal {
                e.text("orient", if h { "h" } else { "v" });
            }
            if let Some(b) = c.collapsed {
                e.raw("collapsed", if b { "true" } else { "false" });
            }
            if let Some(b) = c.hidden {
                e.raw("hidden", if b { "true" } else { "false" });
            }
            for (k, v) in &c.unknown {
                e.raw(k, v);
            }
            list.push(e.0 + "}");
        }
        list.extend(self.unknown_entries.iter().cloned());
        if !list.is_empty() {
            o.raw("palettes", &format!("[{}]", list.join(",")));
        }
        (o.0.len() > 1).then(|| o.0 + "}")
    }

    /// One setting, as `--set layout.<key>=<value>` gives it: `preset` (`auto`, `desktop`, `tablet`, `phone`), `snap`, or
    /// `reset`. The error is an i18n key and changes nothing.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
        let v = value.trim();
        match key {
            "preset" if v == "auto" => self.preset = None,
            "preset" => self.preset = Some(Preset::parse(v).ok_or(ERR)?),
            "snap" => {
                self.snap = v
                    .parse()
                    .ok()
                    .filter(|s| (0.0..=64.0).contains(s))
                    .ok_or(ERR)?
            }
            "reset" => self.reset(),
            _ => return Err("error.draw2.set"),
        }
        Ok(())
    }

    /// Back to the preset: every change forgotten, every palette shown.
    pub fn reset(&mut self) {
        self.changed.clear();
        self.all_hidden = false;
        self.grab = None;
    }

    /// Lays the palettes out in a viewport of `w` x `h` pixels, with the sizes the page measured (`id`, width, height; a
    /// palette not measured yet counts as 44 x 44). Docked palettes line up along their edge in their order, wrapping to a
    /// new row or column when the edge is full; the top and bottom rows span the width, the columns fill what is between.
    /// A floating palette is kept inside the viewport. Returns every palette, hidden ones too, and the free area left for
    /// the canvas (x, y, width, height).
    pub fn arrange(
        &mut self,
        w: f64,
        h: f64,
        sizes: &[(String, f64, f64)],
    ) -> (Vec<Rect>, [f64; 4]) {
        // a viewport that is not a finite number keeps the last one; any other is kept between 1 and MAX_VIEW pixels
        let (w, h) = match point(w, h) {
            Some((w, h)) => (within(w, 1.0, MAX_VIEW), within(h, 1.0, MAX_VIEW)),
            None => self.view,
        };
        self.view = (w, h);
        if !sizes.is_empty() {
            self.sizes = sizes.to_vec();
        }
        let size = |id: &str| {
            self.sizes
                .iter()
                .find(|s| s.0 == id)
                .map_or((44.0, 44.0), |s| (within(s.1, 0.0, w), within(s.2, 0.0, h)))
        };
        let mut all = self.palettes();
        crate::m::sort_by(&mut all, |a, b| (a.order, &a.id) < (b.order, &b.id));
        let shown = |p: &Palette| !p.hidden && !self.all_hidden;
        let mut out: Vec<Rect> = Vec::new();
        let rect = |p: &Palette, x: f64, y: f64, (pw, ph): (f64, f64)| Rect {
            id: p.id.clone(),
            x,
            y,
            w: pw,
            h: ph,
            dock: p.dock,
            horizontal: p.horizontal,
            collapsed: p.collapsed,
            hidden: !shown(p),
        };
        // rows: the top from the top down, the bottom from the bottom up
        let row = |edge: Dock, out: &mut Vec<Rect>| -> f64 {
            let (mut x, mut used, mut line) = (0.0, 0.0, 0.0f64);
            for p in all.iter().filter(|p| p.dock == edge && shown(p)) {
                let s = size(&p.id);
                if x > 0.0 && x + s.0 > w {
                    (x, used, line) = (0.0, used + line, 0.0);
                }
                let y = if edge == Dock::Top {
                    used
                } else {
                    h - used - s.1
                };
                out.push(rect(p, x, y, s));
                x += s.0;
                line = line.max(s.1);
            }
            used + line
        };
        let top = row(Dock::Top, &mut out);
        let bottom = row(Dock::Bottom, &mut out).min(h - top);
        let column = |edge: Dock, out: &mut Vec<Rect>| -> f64 {
            let (mut y, mut used, mut line) = (top, 0.0, 0.0f64);
            for p in all.iter().filter(|p| p.dock == edge && shown(p)) {
                let s = size(&p.id);
                if y > top && y + s.1 > h - bottom {
                    (y, used, line) = (top, used + line, 0.0);
                }
                let x = if edge == Dock::Left {
                    used
                } else {
                    w - used - s.0
                };
                out.push(rect(p, x, y, s));
                y += s.1;
                line = line.max(s.0);
            }
            used + line
        };
        let left = column(Dock::Left, &mut out);
        let right = column(Dock::Right, &mut out).min(w - left);
        for p in all.iter().filter(|p| p.dock == Dock::Float && shown(p)) {
            let s = size(&p.id);
            out.push(rect(
                p,
                within(p.x, 0.0, (w - s.0).max(0.0)),
                within(p.y, 0.0, (h - s.1).max(0.0)),
                s,
            ));
        }
        // a viewport too small for every edge to wrap in: the palettes overlap rather than leave it
        for r in out.iter_mut() {
            r.x = within(r.x, 0.0, (w - r.w).max(0.0));
            r.y = within(r.y, 0.0, (h - r.h).max(0.0));
        }
        for p in all.iter().filter(|p| !shown(p)) {
            out.push(rect(p, 0.0, 0.0, size(&p.id)));
        }
        // on a viewport smaller than its palettes the free area is empty, and it starts inside the viewport
        let area = [
            left.min(w),
            top.min(h),
            (w - left - right).max(0.0),
            (h - top - bottom).max(0.0),
        ];
        (out, area)
    }

    /// The rectangle of a palette in the last layout.
    fn rect_of(&mut self, id: &str) -> Option<Rect> {
        let (w, h) = self.view;
        self.arrange(w, h, &[])
            .0
            .into_iter()
            .find(|r| r.id == id && !r.hidden)
    }

    /// The pointer lands on the handle of a palette at (`x`, `y`) page pixels. False when there is no such palette shown.
    pub fn press(&mut self, id: &str, x: f64, y: f64) -> bool {
        let (Some((x, y)), Some(r), Some(p)) = (point(x, y), self.rect_of(id), self.palette(id))
        else {
            return false;
        };
        self.grab = Some(Grab {
            id: id.to_string(),
            dx: x - r.x,
            dy: y - r.y,
            start: p,
            pointer: true,
        });
        true
    }

    /// The held palette follows the pointer, floating, snapped to the edges and lined up with the palettes it comes near.
    pub fn drag(&mut self, x: f64, y: f64) -> bool {
        let (Some((x, y)), Some(g)) = (point(x, y), self.grab.clone()) else {
            return false;
        };
        let Some(mut p) = self.palette(&g.id) else {
            return false;
        };
        let (pw, ph) = self.rect_of(&g.id).map_or((44.0, 44.0), |r| (r.w, r.h));
        let (nx, ny) = self.snapped(&g.id, x - g.dx, y - g.dy, pw, ph);
        p.dock = Dock::Float;
        (p.x, p.y) = (nx, ny);
        self.put(p);
        true
    }

    /// Snaps a palette of `pw` x `ph` at (`x`, `y`) to the edges of the viewport and to the sides of the other shown palettes
    /// within the snap distance (flush side by side, or aligned on the same line), then keeps it inside the viewport.
    fn snapped(&mut self, id: &str, x: f64, y: f64, pw: f64, ph: f64) -> (f64, f64) {
        let (w, h) = self.view;
        let (rects, _) = self.arrange(w, h, &[]);
        let s = self.snap;
        let near = |v: f64, targets: &[f64]| {
            targets
                .iter()
                .map(|t| (t, (v - t).abs()))
                .filter(|(_, d)| *d < s)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(t, _)| *t)
        };
        let mut xs = vec![0.0, w - pw];
        let mut ys = vec![0.0, h - ph];
        for r in rects.iter().filter(|r| r.id != id && !r.hidden) {
            xs.extend([r.x + r.w, r.x - pw, r.x, r.x + r.w - pw]);
            ys.extend([r.y + r.h, r.y - ph, r.y, r.y + r.h - ph]);
        }
        let nx = near(x, &xs).unwrap_or(x);
        let ny = near(y, &ys).unwrap_or(y);
        (
            within(nx, 0.0, (w - pw).max(0.0)),
            within(ny, 0.0, (h - ph).max(0.0)),
        )
    }

    /// The pointer lets go: a palette within the snap distance of an edge docks there, in the place along the edge that its
    /// position gives; else it stays where it floats.
    pub fn release(&mut self, x: f64, y: f64) -> bool {
        // a release at a point that is not a number ends the grab where the palette was
        if !self.drag(x, y) {
            return self.cancel();
        }
        let g = self.grab.take();
        if let Some(g) = g {
            self.dock_if_near(&g.id);
        }
        true
    }

    /// Puts the held palette back where it was before the grab.
    pub fn cancel(&mut self) -> bool {
        match self.grab.take() {
            Some(g) => {
                self.put(g.start);
                true
            }
            None => false,
        }
    }

    /// The edge a floating palette touches (within the snap distance), the nearest one first.
    fn touching(&mut self, id: &str) -> Option<Dock> {
        let r = self.rect_of(id)?;
        let (w, h) = self.view;
        let s = self.snap.max(1.0);
        [
            (Dock::Left, r.x),
            (Dock::Top, r.y),
            (Dock::Right, w - r.x - r.w),
            (Dock::Bottom, h - r.y - r.h),
        ]
        .into_iter()
        .filter(|(_, d)| *d < s)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(e, _)| e)
    }

    fn dock_if_near(&mut self, id: &str) {
        if let Some(edge) = self.touching(id) {
            self.dock_at(id, edge);
        }
    }

    /// Docks a palette on an edge, in the place its position along the edge gives, and numbers that edge again.
    fn dock_at(&mut self, id: &str, edge: Dock) {
        let Some(here) = self.rect_of(id) else { return };
        let (w, h) = self.view;
        let (rects, _) = self.arrange(w, h, &[]);
        // a full edge wraps into rows or columns: the place along the edge is (which row or column, from the edge out; where
        // along it), so the palettes of two columns are never interleaved
        let place = |r: &Rect| match edge {
            Dock::Left => (r.x, r.y),
            Dock::Right => (w - r.x - r.w, r.y),
            Dock::Top => (r.y, r.x),
            _ => (h - r.y - r.h, r.x),
        };
        let mut line: Vec<((f64, f64), String)> = rects
            .iter()
            .filter(|r| r.dock == edge && r.id != id && !r.hidden)
            .map(|r| (place(r), r.id.clone()))
            .collect();
        let (cross, along) = place(&here);
        line.push(((cross.max(0.0), along - 0.5), id.to_string()));
        crate::m::sort_by(&mut line, |a, b| {
            let (ca, cb) = ((a.0 .0).round(), (b.0 .0).round());
            ca < cb || (ca == cb && a.0 .1 < b.0 .1)
        });
        // the hidden palettes of the edge are numbered after the shown ones, in their old order, so no two share a number
        let mut hidden: Vec<(u32, String)> = (self.palettes().into_iter())
            .filter(|p| p.dock == edge && p.id != id && (p.hidden || self.all_hidden))
            .map(|p| (p.order, p.id))
            .collect();
        crate::m::sort_by(&mut hidden, |a, b| a.0 < b.0);
        let mut n = 0;
        for pid in line.iter().map(|l| &l.1).chain(hidden.iter().map(|h| &h.1)) {
            n += 1;
            if let Some(mut p) = self.palette(pid) {
                p.dock = edge;
                p.order = n;
                p.horizontal = edge.horizontal().unwrap_or(p.horizontal);
                self.put(p);
            }
        }
    }

    /// A key on the focused handle of a palette: the arrows move it 8 pixels (32 with Shift), floating; `enter` docks it on
    /// the nearest edge, or floats a docked one where it is; `escape` puts it back where the keyboard move began. False for
    /// any other key.
    pub fn key(&mut self, id: &str, key: &str, shift: bool) -> bool {
        // a palette held by the pointer is the pointer's until it is let go
        if self.grab.as_ref().is_some_and(|g| g.pointer) {
            return false;
        }
        let step = if shift { 32.0 } else { 8.0 };
        let (dx, dy) = match key {
            "arrowleft" => (-step, 0.0),
            "arrowright" => (step, 0.0),
            "arrowup" => (0.0, -step),
            "arrowdown" => (0.0, step),
            "enter" => {
                self.grab = None;
                let Some(r) = self.rect_of(id) else {
                    return false;
                };
                let Some(mut p) = self.palette(id) else {
                    return false;
                };
                if p.dock == Dock::Float {
                    let (w, h) = self.view;
                    let edge = [
                        (Dock::Left, r.x),
                        (Dock::Top, r.y),
                        (Dock::Right, w - r.x - r.w),
                        (Dock::Bottom, h - r.y - r.h),
                    ]
                    .into_iter()
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map_or(Dock::Left, |e| e.0);
                    self.dock_at(id, edge);
                } else {
                    p.dock = Dock::Float;
                    (p.x, p.y) = (r.x, r.y);
                    self.put(p);
                }
                return true;
            }
            "escape" => return self.cancel(),
            _ => return false,
        };
        let Some(r) = self.rect_of(id) else {
            return false;
        };
        let Some(mut p) = self.palette(id) else {
            return false;
        };
        if self.grab.as_ref().is_none_or(|g| g.id != id) {
            self.grab = Some(Grab {
                id: id.to_string(),
                dx: 0.0,
                dy: 0.0,
                start: p.clone(),
                pointer: false,
            });
        }
        // a keyboard step is smaller than the snap distance: it is only kept inside the viewport
        let (w, h) = self.view;
        let (x, y) = (
            within(r.x + dx, 0.0, (w - r.w).max(0.0)),
            within(r.y + dy, 0.0, (h - r.h).max(0.0)),
        );
        p.dock = Dock::Float;
        (p.x, p.y) = (x, y);
        self.put(p);
        true
    }

    /// Shows or hides a palette, collapses or expands it, or turns a floating one between row and column.
    pub fn toggle(&mut self, id: &str, what: &str) -> bool {
        let Some(mut p) = self.palette(id) else {
            return false;
        };
        match what {
            "hidden" => p.hidden = !p.hidden,
            "collapsed" => p.collapsed = !p.collapsed,
            "orient" if p.dock == Dock::Float => p.horizontal = !p.horizontal,
            _ => return false,
        }
        self.put(p);
        true
    }

    /// The layout as JSON for the page: `{"preset", "all_hidden", "area": [x, y, w, h], "palettes": [{"id", "x", "y", "w", "h",
    /// "dock", "horizontal", "collapsed", "hidden"}]}`.
    pub fn arranged_json(&mut self, w: f64, h: f64, sizes: &[(String, f64, f64)]) -> String {
        let (rects, area) = self.arrange(w, h, sizes);
        let bool = |b: bool| if b { "true" } else { "false" };
        let list: Vec<String> = rects
            .iter()
            .map(|r| {
                let mut e = Writer("{".into());
                e.text("id", &r.id);
                for (k, v) in [("x", r.x), ("y", r.y), ("w", r.w), ("h", r.h)] {
                    e.num(k, v);
                }
                e.text("dock", r.dock.name());
                e.raw("horizontal", bool(r.horizontal));
                e.raw("collapsed", bool(r.collapsed));
                e.raw("hidden", bool(r.hidden));
                e.0 + "}"
            })
            .collect();
        let mut o = Writer("{".into());
        o.text("preset", self.current().name());
        o.raw("all_hidden", bool(self.all_hidden));
        let area: Vec<String> = area.iter().map(|v| Value::from(*v).to_string()).collect();
        o.raw("area", &format!("[{}]", area.join(",")));
        o.raw("palettes", &format!("[{}]", list.join(",")));
        o.0 + "}"
    }
}

/// Reads the sizes the page measured, `[[id, width, height], ...]`; anything else is no size.
pub fn sizes_of(json: &str) -> Vec<(String, f64, f64)> {
    let v: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    (v.as_array().into_iter().flatten())
        .filter_map(|e| {
            let id = e.get(0)?.as_str()?;
            let (w, h) = (e.get(1)?.as_f64()?, e.get(2)?.as_f64()?);
            (w.is_finite() && h.is_finite()).then(|| (id.to_string(), w.max(0.0), h.max(0.0)))
        })
        .collect()
}

/// The formats a drawing is saved in (`Editor::save`, and `png` through the page's rasteriser).
pub const FORMATS: [&str; 12] = [
    "svg",
    "png",
    "mzdraw",
    "molfile",
    "molfile-v3000",
    "sdf",
    "smiles",
    "ket",
    "cdxml",
    "cml",
    "rxn",
    "log",
];

/// The `export` object: what one click on Save writes (`default`) and on what (`background`: `white` or `none`).
#[derive(Clone, Debug, PartialEq)]
pub struct Export {
    pub default: String,
    pub background: String,
    unknown: Vec<(String, String)>,
}

impl Default for Export {
    fn default() -> Self {
        Export {
            default: "svg".into(),
            background: "white".into(),
            unknown: Vec::new(),
        }
    }
}

impl Export {
    /// Reads the `export` object of a settings file; a format or background that is none is refused with
    /// `error.draw2.export`.
    pub fn read(v: &Value) -> Result<Export, &'static str> {
        let mut e = Export::default();
        for (k, x) in v.as_object().ok_or("error.draw2.export")? {
            match k.as_str() {
                "default" | "background" => {
                    let s = x.as_str().ok_or("error.draw2.export")?;
                    e.set(k, s)?;
                }
                _ => e.unknown.push((k.clone(), x.to_string())),
            }
        }
        Ok(e)
    }

    /// One setting, as `--set export.<key>=<value>` gives it. The error is an i18n key and changes nothing.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
        let v = value.trim();
        match key {
            "default" if FORMATS.contains(&v) => self.default = v.into(),
            "background" if v == "white" || v == "none" => self.background = v.into(),
            "default" | "background" => return Err("error.draw2.export"),
            _ => return Err("error.draw2.set"),
        }
        Ok(())
    }

    /// The `export` object as JSON text, `None` when it holds only the defaults.
    pub fn to_text(&self) -> Option<String> {
        let mut o = Writer("{".into());
        for (k, v) in &self.unknown {
            o.raw(k, v);
        }
        if self.background != "white" {
            o.text("background", &self.background);
        }
        if self.default != "svg" {
            o.text("default", &self.default);
        }
        (o.0.len() > 1).then(|| o.0 + "}")
    }

    /// What one click on Save writes, for the page: `{"background", "default"}`.
    pub fn to_json(&self) -> String {
        let mut o = Writer("{".into());
        o.text("background", &self.background);
        o.text("default", &self.default);
        o.0 + "}"
    }
}
