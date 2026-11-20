//! The palettes of the page and the export defaults (`mzdraw::palettes`): the presets, docking, snapping, the keyboard, the
//! settings file, and a property test that no sequence of moves takes a palette out of the viewport or loses the settings.

use mzdraw::editor::{Editor, HANDLED, PALETTES};
use mzdraw::palettes::{Dock, Layout, Rect};

/// The sizes the page would measure: a column item 44 px wide, a top-bar group 132 px, a side group 280 px.
fn sizes() -> Vec<(String, f64, f64)> {
    mzdraw::palettes::PALETTES
        .iter()
        .map(|&(id, dock, _, _)| {
            let (w, h) = match dock {
                Dock::Left => (44.0, 88.0),
                Dock::Top => (132.0, 44.0),
                _ => (280.0, 100.0),
            };
            (id.to_string(), w, h)
        })
        .collect()
}

fn rect<'a>(rects: &'a [Rect], id: &str) -> &'a Rect {
    rects.iter().find(|r| r.id == id).unwrap()
}

#[test]
fn desktop_preset_docks_tools_left_actions_on_top_and_side_groups_right() {
    let mut l = Layout::default();
    let (r, area) = l.arrange(1280.0, 720.0, &sizes());
    // the top row: file, history, edit, view from the left
    let (file, view) = (rect(&r, "file"), rect(&r, "view"));
    assert_eq!(
        (file.x, file.y, file.dock, file.horizontal),
        (0.0, 0.0, Dock::Top, true)
    );
    assert_eq!(view.x, 3.0 * 132.0);
    // the tool column under the top row, in order, as a column
    let ys: Vec<f64> = ["tools", "bonds", "rings", "atoms", "charges", "objects"]
        .iter()
        .map(|id| rect(&r, id).y)
        .collect();
    assert_eq!(ys, vec![44.0, 132.0, 220.0, 308.0, 396.0, 484.0]);
    assert!(r
        .iter()
        .filter(|x| x.dock == Dock::Left)
        .all(|x| x.x == 0.0 && !x.horizontal));
    // the side groups at the right edge, shown on a desktop
    let s = rect(&r, "structure");
    assert_eq!((s.x, s.y, s.hidden), (1000.0, 44.0, false));
    // the canvas gets what is left
    assert_eq!(area, [44.0, 44.0, 1280.0 - 44.0 - 280.0, 720.0 - 44.0]);
    assert!(rect(&r, "bonds").collapsed && !rect(&r, "tools").collapsed);
    assert_eq!(
        l.to_text(),
        None,
        "the preset needs no layout in the settings file"
    );
}

#[test]
fn tablet_hides_the_side_groups_and_phone_docks_the_tools_on_top() {
    let mut l = Layout::default();
    let (r, area) = l.arrange(768.0, 1024.0, &sizes());
    assert!(r.iter().filter(|x| x.dock == Dock::Right).all(|x| x.hidden));
    assert_eq!(area[2], 768.0 - 44.0, "a hidden side group takes no room");
    let (r, _) = l.arrange(390.0, 800.0, &sizes());
    let tools = rect(&r, "tools");
    assert_eq!((tools.dock, tools.horizontal), (Dock::Top, true));
    assert!(tools.y > 0.0, "the tools wrap into a row under the actions");
    assert!(
        r.iter().filter(|x| !x.hidden).all(|x| x.x + x.w <= 390.0),
        "every row fits the width"
    );
}

#[test]
fn a_dragged_palette_floats_snaps_and_docks_on_the_edge_it_is_dropped_on() {
    let mut l = Layout::default();
    l.arrange(1280.0, 720.0, &sizes());
    // grab the bonds palette 10 px inside its handle and drop it in the middle: it floats there, a column still
    assert!(l.press("bonds", 10.0, 140.0));
    assert!(l.release(610.0, 600.0));
    let (r, _) = l.arrange(1280.0, 720.0, &[]);
    let b = rect(&r, "bonds");
    assert_eq!(
        (b.dock, b.x, b.y, b.horizontal),
        (Dock::Float, 600.0, 592.0, false)
    );
    // the column closes the gap
    assert_eq!(rect(&r, "rings").y, 132.0);
    // dragged next to the rings palette, within 12 px of its right side: flush against it, top lined up
    l.press("bonds", 605.0, 595.0);
    l.drag(5.0 + 44.0 + 7.0, 3.0 + 132.0 + 5.0);
    let (r, _) = l.arrange(1280.0, 720.0, &[]);
    assert_eq!((rect(&r, "bonds").x, rect(&r, "bonds").y), (44.0, 132.0));
    // dropped at the bottom edge: docked there, laid out as a row
    l.drag(600.0, 710.0);
    l.release(600.0, 715.0);
    let (r, area) = l.arrange(1280.0, 720.0, &[]);
    let b = rect(&r, "bonds");
    assert_eq!(
        (b.dock, b.horizontal, b.y + b.h),
        (Dock::Bottom, true, 720.0)
    );
    assert_eq!(
        area[3],
        720.0 - 44.0 - b.h,
        "the bottom row takes its height from the canvas"
    );
    // dropped on the top edge at the far right: the last of the top row
    l.press("bonds", b.x + 1.0, b.y + 1.0);
    l.release(1270.0, 2.0);
    let (r, _) = l.arrange(1280.0, 720.0, &[]);
    let top: Vec<&str> = {
        let mut t: Vec<&Rect> = r.iter().filter(|x| x.dock == Dock::Top).collect();
        t.sort_by(|a, b| a.x.total_cmp(&b.x));
        t.iter().map(|x| x.id.as_str()).collect()
    };
    assert_eq!(top, vec!["file", "history", "edit", "view", "bonds"]);
    // a grab that is cancelled puts the palette back
    l.press("bonds", 600.0, 10.0);
    l.drag(300.0, 300.0);
    assert!(l.cancel());
    assert_eq!(
        rect(&l.arrange(1280.0, 720.0, &[]).0, "bonds").dock,
        Dock::Top
    );
}

#[test]
fn the_keyboard_moves_docks_and_puts_back_a_palette() {
    let mut l = Layout::default();
    l.arrange(1280.0, 720.0, &sizes());
    let at = |l: &mut Layout| {
        let r = l.arrange(1280.0, 720.0, &[]).0;
        let e = rect(&r, "edit");
        (e.dock, e.x, e.y)
    };
    let start = at(&mut l);
    assert!(l.key("edit", "arrowdown", false));
    assert_eq!(at(&mut l), (Dock::Float, 264.0, 8.0));
    l.key("edit", "arrowdown", true);
    l.key("edit", "arrowright", true);
    assert_eq!(at(&mut l), (Dock::Float, 296.0, 40.0));
    assert!(l.key("edit", "escape", false));
    assert_eq!(
        at(&mut l),
        start,
        "Escape: back where the keyboard move began"
    );
    // Enter floats a docked palette where it is and docks a floating one on the nearest edge
    l.key("edit", "enter", false);
    assert_eq!(at(&mut l), (Dock::Float, 264.0, 0.0));
    for _ in 0..40 {
        l.key("edit", "arrowdown", true);
    }
    assert_eq!(at(&mut l).2, 720.0 - 44.0, "kept inside the viewport");
    l.key("edit", "enter", false);
    assert_eq!(at(&mut l).0, Dock::Bottom);
    assert!(!l.key("edit", "x", false));
}

#[test]
fn layout_and_export_live_in_the_settings_file_with_unknown_keys_kept() {
    let mut e = Editor::default();
    let plain = e.settings_json();
    assert!(
        !plain.contains("layout") && !plain.contains("export"),
        "defaults write nothing new: {plain}"
    );
    let file = r#"{"format":"mzdraw-settings/1","style":{"preset":"acs-1996"},
      "layout":{"preset":"tablet","snap":16,"future":1,"palettes":[{"id":"edit","dock":"float","x":300,"y":200,"orient":"v"},
        {"id":"rings","collapsed":false},{"id":"newer-panel","dock":"left"}]},
      "export":{"default":"png","background":"none","dpi":600}}"#;
    assert_eq!(e.set_style(file).unwrap() & PALETTES, PALETTES);
    assert_eq!(
        (e.export.default.as_str(), e.export.background.as_str()),
        ("png", "none")
    );
    let (r, _) = e.layout.arrange(1280.0, 720.0, &sizes());
    let edit = rect(&r, "edit");
    assert_eq!(
        (edit.dock, edit.x, edit.y, edit.horizontal),
        (Dock::Float, 300.0, 200.0, false)
    );
    assert!(
        !rect(&r, "rings").collapsed && rect(&r, "structure").hidden,
        "the tablet preset, with the change on top"
    );
    // written back: the same settings, the unknown key, palette and export field kept
    let out = e.settings_json();
    assert!(
        out.contains("\"future\":1") && out.contains("newer-panel") && out.contains("\"dpi\":600"),
        "{out}"
    );
    let mut again = Editor::default();
    again.set_style(&out).unwrap();
    assert_eq!(again.settings_json(), out);
    assert_eq!(again.layout, e.layout);
    // a preset name changes the style and keeps the layout
    e.set_style("mzlab").unwrap();
    assert_eq!(e.export.default, "png");
    // refused: a dock that is none, a format that is none; nothing changes
    let before = e.settings_json();
    for bad in [
        r#"{"format":"mzdraw-settings/1","layout":{"palettes":[{"id":"edit","dock":"middle"}]}}"#,
        r#"{"format":"mzdraw-settings/1","layout":{"snap":"far"}}"#,
        r#"{"format":"mzdraw-settings/1","export":{"default":"gif"}}"#,
    ] {
        assert!(e.set_style(bad).is_err(), "{bad}");
        assert_eq!(e.settings_json(), before);
    }
}

#[test]
fn settings_are_set_one_by_one_as_the_cli_does() {
    let mut e = Editor::default();
    assert_eq!(e.set("export.default=molfile"), Ok(0));
    assert_eq!(e.set("export.background=none"), Ok(0));
    assert_eq!(e.set("export.default=gif"), Err("error.draw2.export"));
    assert_eq!(e.set("layout.preset=phone"), Ok(PALETTES));
    assert_eq!(e.set("layout.snap=99"), Err("error.draw2.layout"));
    assert_eq!(
        e.set(r#"layout={"palettes":[{"id":"view","hidden":true}]}"#),
        Ok(PALETTES)
    );
    assert!(rect(&e.layout.arrange(1280.0, 720.0, &[]).0, "view").hidden);
    assert_eq!(e.set("layout.reset=1"), Ok(PALETTES));
    assert!(!rect(&e.layout.arrange(1280.0, 720.0, &[]).0, "view").hidden);
    assert_eq!(e.set("style.line_width=2"), Ok(mzdraw::editor::REPAINT));
    assert_eq!(e.set("colour.x=1"), Err("error.draw2.set"));
    assert_eq!(
        e.export.to_json(),
        r#"{"background":"none","default":"molfile"}"#
    );
}

#[test]
fn backslash_hides_and_shows_every_palette() {
    let mut e = Editor::default();
    let f = e.key("\\", 0);
    assert_eq!(f & (PALETTES | HANDLED), PALETTES | HANDLED);
    let (r, area) = e.layout.arrange(1280.0, 720.0, &sizes());
    assert!(r.iter().all(|x| x.hidden));
    assert_eq!(area, [0.0, 0.0, 1280.0, 720.0], "the whole canvas");
    e.key("\\", 0);
    assert!(!e
        .layout
        .arrange(1280.0, 720.0, &[])
        .0
        .iter()
        .all(|x| x.hidden));
}

/// xorshift: repeatable pseudo-random choices without a dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn f(&mut self, max: f64) -> f64 {
        self.below(10_000) as f64 / 10_000.0 * max
    }
}

#[test]
fn property_no_sequence_of_moves_takes_a_palette_out_of_view_or_loses_the_settings() {
    let ids: Vec<&str> = mzdraw::palettes::PALETTES.iter().map(|p| p.0).collect();
    let keys = [
        "arrowleft",
        "arrowright",
        "arrowup",
        "arrowdown",
        "enter",
        "escape",
    ];
    for seed in 1..=60u64 {
        let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut l = Layout::default();
        for step in 0..80 {
            let (w, h) = (320.0 + r.f(1600.0), 320.0 + r.f(900.0));
            let id = ids[r.below(ids.len() as u64) as usize];
            match r.below(7) {
                0 => {
                    l.press(id, r.f(w), r.f(h));
                    l.drag(r.f(w * 1.4) - w * 0.2, r.f(h * 1.4) - h * 0.2);
                    l.release(r.f(w * 1.4) - w * 0.2, r.f(h * 1.4) - h * 0.2);
                }
                1 => {
                    l.key(id, keys[r.below(6) as usize], r.below(2) == 0);
                }
                2 => {
                    l.toggle(id, ["hidden", "collapsed", "orient"][r.below(3) as usize]);
                }
                3 => {
                    l.press(id, r.f(w), r.f(h));
                    l.drag(r.f(w), r.f(h));
                    l.cancel();
                }
                _ => {}
            }
            let (rects, area) = l.arrange(w, h, &sizes());
            for x in rects.iter().filter(|x| !x.hidden) {
                assert!(
                    x.x >= 0.0 && x.y >= 0.0,
                    "seed {seed} step {step}: {x:?} starts outside"
                );
                assert!(
                    x.x + x.w <= w + 1e-9 && x.y + x.h <= h + 1e-9,
                    "seed {seed} step {step}: {x:?} past the viewport {w} x {h}"
                );
                assert_eq!(
                    x.horizontal,
                    x.dock.name() == "top"
                        || x.dock.name() == "bottom"
                        || (x.dock == Dock::Float && x.horizontal)
                );
            }
            assert!(area.iter().all(|v| v.is_finite() && *v >= 0.0));
            // the settings survive a round through their text
            let text = l.to_text().unwrap_or("{}".into());
            let back = Layout::read(&serde_json::from_str(&text).unwrap()).unwrap();
            assert_eq!(back, l, "seed {seed} step {step}: {text}");
        }
        l.reset();
        assert_eq!(
            l.to_text(),
            None,
            "seed {seed}: a reset is the preset again"
        );
    }
}
