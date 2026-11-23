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
    // one general tools palette under the top row, a column (the groups of tools are its own: `tool_groups`)
    let left: Vec<(&str, f64)> = r
        .iter()
        .filter(|x| x.dock == Dock::Left)
        .map(|x| (x.id.as_str(), x.y))
        .collect();
    assert_eq!(left, vec![("tools", 44.0)]);
    assert!(r
        .iter()
        .filter(|x| x.dock == Dock::Left)
        .all(|x| x.x == 0.0 && !x.horizontal));
    // the side groups at the right edge, shown on a desktop
    let s = rect(&r, "structure");
    assert_eq!((s.x, s.y, s.hidden), (1000.0, 44.0, false));
    // the canvas gets what is left
    assert_eq!(area, [44.0, 44.0, 1280.0 - 44.0 - 280.0, 720.0 - 44.0]);
    assert!(rect(&r, "edit").collapsed && !rect(&r, "tools").collapsed);
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
    // grab the tools palette 10 px inside its handle and drop it in the middle: it floats there, a column still
    assert!(l.press("tools", 10.0, 52.0));
    assert!(l.release(610.0, 600.0));
    let (r, area) = l.arrange(1280.0, 720.0, &[]);
    let b = rect(&r, "tools");
    assert_eq!(
        (b.dock, b.x, b.y, b.horizontal),
        (Dock::Float, 600.0, 592.0, false)
    );
    // nothing is left in the left column: the canvas takes its width
    assert_eq!(area[0], 0.0);
    // dragged next to the view palette, within 12 px of its right side: flush against it, top lined up
    l.press("tools", 605.0, 595.0);
    l.drag(3.0 * 132.0 + 132.0 + 5.0 + 7.0, 3.0 + 5.0);
    let (r, _) = l.arrange(1280.0, 720.0, &[]);
    assert_eq!(
        (rect(&r, "tools").x, rect(&r, "tools").y),
        (4.0 * 132.0, 0.0)
    );
    // dropped at the bottom edge: docked there, laid out as a row
    l.drag(600.0, 710.0);
    l.release(600.0, 715.0);
    let (r, area) = l.arrange(1280.0, 720.0, &[]);
    let b = rect(&r, "tools");
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
    l.press("tools", b.x + 1.0, b.y + 1.0);
    l.release(1270.0, 2.0);
    let (r, _) = l.arrange(1280.0, 720.0, &[]);
    let top: Vec<&str> = {
        let mut t: Vec<&Rect> = r.iter().filter(|x| x.dock == Dock::Top).collect();
        t.sort_by(|a, b| a.x.total_cmp(&b.x));
        t.iter().map(|x| x.id.as_str()).collect()
    };
    assert_eq!(top, vec!["file", "history", "edit", "view", "tools"]);
    // a grab that is cancelled puts the palette back
    l.press("tools", 600.0, 10.0);
    l.drag(300.0, 300.0);
    assert!(l.cancel());
    assert_eq!(
        rect(&l.arrange(1280.0, 720.0, &[]).0, "tools").dock,
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
        {"id":"view","collapsed":true},{"id":"newer-panel","dock":"left"}]},
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
        rect(&r, "view").collapsed && rect(&r, "structure").hidden,
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

/// The sizes the page measures at 1280x720: the side groups as tall as they are drawn, the history group as a column.
fn page_sizes() -> Vec<(String, f64, f64)> {
    let side = [
        ("structure", 165.0),
        ("style", 162.0),
        ("data", 68.0),
        ("annotation", 68.0),
        ("check", 133.0),
        ("keys", 56.0),
    ];
    let mut s = sizes();
    for e in s.iter_mut() {
        if let Some(h) = side.iter().find(|x| x.0 == e.0) {
            (e.1, e.2) = (280.0, h.1);
        }
    }
    s
}

fn read_back(l: &Layout) -> Layout {
    let text = l.to_text().unwrap_or_else(|| "{}".into());
    Layout::read(&serde_json::from_str(&text).unwrap()).expect("the layout reads its own text")
}

#[test]
fn page_input_that_is_not_a_finite_number_is_refused_and_the_settings_read_back() {
    let mut l = Layout::default();
    l.arrange(1280.0, 720.0, &sizes());
    assert!(!l.press("edit", f64::NAN, 10.0));
    assert!(l.press("edit", 400.0, 10.0));
    assert!(!l.drag(f64::NAN, 300.0) && !l.drag(f64::INFINITY, 300.0));
    // a release at no point ends the grab where the palette was
    let before = l.palettes();
    assert!(l.release(f64::NAN, 0.0));
    assert_eq!(l.palettes(), before);
    // a viewport that is not a number keeps the last one; a huge one is kept within 100,000 px
    let (_, area) = l.arrange(f64::NAN, f64::INFINITY, &[]);
    assert!(area[2] + area[0] <= 1280.0);
    let json = l.arranged_json(1e300, 1e300, &[]);
    assert!(!json.contains("null") && !json.contains("e+"), "{json}");
    // a palette dragged far out is written at a place the reader takes
    assert!(l.press("edit", 400.0, 10.0) && l.release(1e9, 1e9));
    let read = read_back(&l);
    assert_eq!(read, l);
    let edit = l.palettes().into_iter().find(|p| p.id == "edit").unwrap();
    assert!(edit.x.is_finite() && edit.x <= 1e6 && edit.y <= 1e6);
}

#[test]
fn unknown_keys_inside_a_palette_entry_are_kept() {
    let text =
        r#"{"palettes":[{"id":"edit","dock":"float","x":10,"y":20,"tint":"red","pin":{"a":1}}]}"#;
    let mut l = Layout::read(&serde_json::from_str(text).unwrap()).unwrap();
    let out = l.to_text().unwrap();
    assert!(
        out.contains(r#""tint":"red""#) && out.contains(r#""pin":{"a":1}"#),
        "{out}"
    );
    // a change of the palette keeps them too
    l.arrange(1280.0, 720.0, &sizes());
    assert!(l.toggle("edit", "collapsed"));
    let out = l.to_text().unwrap();
    assert!(
        out.contains(r#""tint":"red""#) && out.contains(r#""collapsed":false"#),
        "{out}"
    );
    assert_eq!(read_back(&l), l);
}

#[test]
fn a_viewport_smaller_than_its_palettes_leaves_an_empty_area_inside_it() {
    let mut l = Layout::default();
    for (w, h) in [(10.0, 10.0), (1.0, 1.0), (50.0, 300.0), (300.0, 40.0)] {
        let (rects, [x, y, aw, ah]) = l.arrange(w, h, &sizes());
        assert!(
            x >= 0.0 && y >= 0.0 && x <= w && y <= h && aw >= 0.0 && ah >= 0.0,
            "{w}x{h}: {x} {y} {aw} {ah}"
        );
        assert!(x + aw <= w && y + ah <= h, "{w}x{h}: {x} {y} {aw} {ah}");
        for r in rects.iter().filter(|r| !r.hidden) {
            assert!(
                r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= w + 1e-9 && r.y + r.h <= h + 1e-9,
                "{w}x{h}: {r:?}"
            );
        }
    }
}

#[test]
fn a_key_during_a_pointer_grab_is_ignored() {
    let mut l = Layout::default();
    let (r, _) = l.arrange(1280.0, 720.0, &sizes());
    let t = rect(&r, "tools").clone();
    assert!(l.press("tools", t.x + 10.0, t.y + 5.0));
    assert!(!l.key("tools", "arrowright", false) && !l.key("edit", "enter", false));
    assert!(l.drag(500.0, 300.0) && l.release(500.0, 300.0));
    let p = l.palettes().into_iter().find(|p| p.id == "tools").unwrap();
    // where the pointer let it go, give or take the snap to a neighbour
    assert!(
        p.dock == Dock::Float && (p.x - 490.0).abs() < 12.0 && (p.y - 295.0).abs() < 12.0,
        "{p:?}"
    );
    // once let go, the keyboard moves it again
    assert!(l.key("tools", "arrowright", false));
}

#[test]
fn docking_on_a_full_column_keeps_its_columns_apart_and_numbers_hidden_palettes_last() {
    let mut l = Layout::default();
    let sizes = page_sizes();
    l.arrange(1280.0, 720.0, &sizes);
    assert!(l.toggle("keys", "hidden"));
    let (r, _) = l.arrange(1280.0, 720.0, &[]);
    let h = rect(&r, "history").clone();
    assert!(l.press("history", h.x + 10.0, h.y + 10.0));
    assert!(l.drag(1279.0, 300.0) && l.release(1279.0, 300.0));
    let (r, _) = l.arrange(1280.0, 720.0, &[]);
    let right: Vec<&Rect> = r
        .iter()
        .filter(|r| r.dock == Dock::Right && !r.hidden)
        .collect();
    assert!(right.iter().any(|r| r.id == "history"));
    // the column at the edge is numbered first, from the top down, then the column beside it: never interleaved
    let order = |id: &str| l.palettes().into_iter().find(|p| p.id == id).unwrap().order;
    let mut by_order = right.clone();
    by_order.sort_by_key(|r| order(&r.id));
    for w in by_order.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (ca, cb) = (1280.0 - a.x - a.w, 1280.0 - b.x - b.w);
        assert!(
            ca < cb || (ca == cb && a.y < b.y),
            "{} before {}",
            a.id,
            b.id
        );
    }
    // the side groups keep their order among themselves
    let side = ["structure", "style", "data", "annotation", "check"].map(order);
    assert!(side.windows(2).all(|w| w[0] < w[1]), "{side:?}");
    // the hidden palette of the edge comes after the shown ones, and no two share a number
    let mut all: Vec<u32> = l
        .palettes()
        .into_iter()
        .filter(|p| p.dock == Dock::Right)
        .map(|p| p.order)
        .collect();
    assert!(order("keys") > *all.iter().filter(|&&o| o != order("keys")).max().unwrap());
    all.sort();
    all.dedup();
    assert_eq!(all.len(), 7);
    for (i, a) in right.iter().enumerate() {
        for b in &right[i + 1..] {
            let apart =
                a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
            assert!(apart, "{} over {}", a.id, b.id);
        }
    }
}

#[test]
fn a_click_with_a_little_jitter_moves_nothing_and_a_drag_past_the_threshold_does() {
    let mut l = Layout::default();
    l.arrange(1280.0, 720.0, &sizes());
    let before = l.palettes();
    // the pointer lands, jitters by a pixel or two, lets go: a click, the layout is as it was and nothing says it changed
    assert!(l.press("edit", 400.0, 10.0));
    assert!(!l.drag(401.0, 11.0));
    assert_eq!(l.palettes(), before);
    assert!(!l.release(402.0, 10.0));
    assert_eq!(l.palettes(), before);
    // a cancel after a click is no change either
    assert!(l.press("edit", 400.0, 10.0));
    assert!(!l.cancel());
    assert_eq!(l.palettes(), before);
    // past the threshold the palette follows the pointer, and the release keeps it where it is let go
    assert!(l.press("edit", 400.0, 10.0));
    assert!(l.drag(400.0 + mzdraw::palettes::DRAG_START + 1.0, 200.0));
    assert!(l.release(500.0, 300.0));
    let edit = l.palettes().into_iter().find(|p| p.id == "edit").unwrap();
    assert_eq!(edit.dock, Dock::Float);
    // once the drag has begun, coming back near the start still moves it (the threshold is for the start only)
    assert!(l.press("edit", 500.0, 300.0));
    assert!(l.drag(510.0, 300.0));
    assert!(l.drag(500.0, 300.0));
    assert!(l.release(500.0, 300.0));
}

#[test]
fn the_module_says_the_held_palette_the_edge_it_docks_on_and_which_palettes_turn() {
    let mut l = Layout::default();
    let json = |l: &mut Layout| -> serde_json::Value {
        serde_json::from_str(&l.arranged_json(1280.0, 720.0, &sizes())).unwrap()
    };
    let first = json(&mut l);
    assert_eq!(
        (first["held"].as_str(), first["hint_edge"].as_str()),
        (Some(""), Some(""))
    );
    // only a floating palette turns
    let can = |v: &serde_json::Value, id: &str| {
        v["palettes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == id)
            .unwrap()["can_turn"]
            .as_bool()
            .unwrap()
    };
    assert!(!can(&first, "tools"));
    // held in the middle: held says which, no edge; flush with the bottom edge: the edge it docks on when let go
    let (r, _) = l.arrange(1280.0, 720.0, &sizes());
    let edit = rect(&r, "edit");
    assert!(l.press("edit", edit.x + 10.0, edit.y + 10.0));
    assert!(l.drag(640.0, 360.0));
    let mid = json(&mut l);
    assert_eq!(
        (mid["held"].as_str(), mid["hint_edge"].as_str()),
        (Some("edit"), Some(""))
    );
    assert!(can(&mid, "edit"));
    assert!(l.drag(640.0, 719.0));
    let low = json(&mut l);
    assert_eq!(low["hint_edge"].as_str(), Some("bottom"));
    assert!(l.release(640.0, 719.0));
    let after = json(&mut l);
    assert_eq!(
        (after["held"].as_str(), after["hint_edge"].as_str()),
        (Some(""), Some(""))
    );
    assert_eq!(
        rect(&l.arrange(1280.0, 720.0, &sizes()).0, "edit").dock,
        Dock::Bottom
    );
}

// ---- the general tools palette: groups, the face of each, and a stored layout of the palettes before it ----

fn slots(l: &Layout) -> Vec<serde_json::Value> {
    serde_json::from_str::<Vec<serde_json::Value>>(&l.slots_json()).unwrap()
}

fn face(l: &Layout, group: &str) -> String {
    slots(l)
        .iter()
        .find(|s| s["group"] == group)
        .map(|s| s["face"].as_str().unwrap().to_string())
        .unwrap()
}

#[test]
fn the_tools_palette_has_sixteen_places_in_a_fixed_order_and_one_button_per_group() {
    let l = Layout::default();
    let s = slots(&l);
    assert_eq!(s.len(), 16);
    let names: Vec<String> = s
        .iter()
        .map(|x| {
            x["cmd"]
                .as_str()
                .or(x["group"].as_str())
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(
        names,
        "select move-here erase bonds bond:2 rings ring:6 ring:5 atom:C atom:N atom:O atoms charges objects plus text"
            .split(' ')
            .collect::<Vec<_>>()
    );
    // the faces of a fresh layout: the single bond, benzene, the plus charge, the arrow
    assert_eq!(
        ["bonds", "rings", "charges", "objects"].map(|g| face(&l, g)),
        ["bond:1", "ring:6a", "charge:+", "arrow"]
    );
    // a group lists every member: the bonds are 13, in the order of their list, the kept double bond among them
    let bonds = s.iter().find(|x| x["group"] == "bonds").unwrap();
    let members: Vec<&str> = bonds["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap())
        .collect();
    assert_eq!(members.len(), 13);
    assert_eq!(&members[..4], ["bond:1", "bond:2", "bond:3", "bond:up"]);
    assert_eq!(bonds["keep"][0], "bond:2");
    // the same layout writes the same list, byte for byte
    assert_eq!(l.slots_json(), Layout::default().slots_json());
    // the answer for the page carries the list and the rows of a column
    let a: serde_json::Value =
        serde_json::from_str(&Layout::default().arranged_json(1280.0, 720.0, &[])).unwrap();
    assert_eq!(
        (a["rows"].clone(), a["slots"].as_array().unwrap().len()),
        (8.into(), 16)
    );
}

#[test]
fn a_tool_used_becomes_the_face_of_its_group_is_kept_and_forgotten_with_the_layout() {
    let mut e = Editor::default();
    assert_eq!(e.set_tool("bond:wavy") & PALETTES, PALETTES);
    assert_eq!(face(&e.layout, "bonds"), "bond:wavy");
    // the same tool again, a tool of no group, and a kind that is in view on its own change no face: no new layout
    for t in ["bond:wavy", "select", "bond:2", "ring:6", "atom:S", "arrow"] {
        assert_eq!(e.set_tool(t) & PALETTES, 0, "{t}");
    }
    assert_eq!(face(&e.layout, "bonds"), "bond:wavy");
    assert_eq!(e.set_tool("ring:3") & PALETTES, PALETTES);
    assert_eq!(e.set_tool("charge:-") & PALETTES, PALETTES);
    // the faces are in the settings file, in the order of the groups, and read back the same
    let out = e.settings_json();
    assert!(
        out.contains(r#""heads":{"bonds":"bond:wavy","rings":"ring:3","charges":"charge:-"}"#),
        "{out}"
    );
    let mut again = Editor::default();
    again.set_style(&out).unwrap();
    assert_eq!(again.settings_json(), out);
    assert_eq!(face(&again.layout, "rings"), "ring:3");
    // the first member is the default: using it again leaves nothing to write
    e.set_tool("bond:1");
    assert!(!e.settings_json().contains("bonds"));
    // reset forgets them all, as it forgets every move of a palette
    assert_eq!(e.set("layout.reset=1"), Ok(PALETTES));
    assert_eq!(face(&e.layout, "rings"), "ring:6a");
    assert_eq!(e.layout.to_text(), None);
}

#[test]
fn the_face_is_a_setting_with_its_cli_twin_and_a_wrong_one_is_refused() {
    let mut e = Editor::default();
    assert_eq!(e.set("layout.heads.bonds=bond:hash"), Ok(PALETTES));
    assert_eq!(face(&e.layout, "bonds"), "bond:hash");
    // not a member, in view on its own, a command that selects no tool, a group with a fixed face, a group that is none
    for bad in [
        "layout.heads.bonds=ring:3",
        "layout.heads.bonds=bond:2",
        "layout.heads.objects=scheme-row",
        "layout.heads.atoms=atom:S",
        "layout.heads.nope=bond:1",
    ] {
        assert_eq!(e.set(bad), Err("error.draw2.layout"), "{bad}");
    }
    assert_eq!(face(&e.layout, "bonds"), "bond:hash");
    // the whole object is read the same way, an unknown group is kept and written back
    let file =
        r#"{"format":"mzdraw-settings/1","layout":{"heads":{"rings":"ring:7","future":"x"}}}"#;
    let mut f = Editor::default();
    f.set_style(file).unwrap();
    assert_eq!(face(&f.layout, "rings"), "ring:7");
    assert!(f.settings_json().contains(r#""future":"x""#));
    // a face that is not one refuses the file and changes nothing
    let before = f.settings_json();
    assert!(f
        .set_style(r#"{"format":"mzdraw-settings/1","layout":{"heads":{"rings":"bond:1"}}}"#)
        .is_err());
    assert_eq!(f.settings_json(), before);
}

#[test]
fn the_general_tools_palette_cannot_be_collapsed_and_says_so() {
    let mut l = Layout::default();
    l.arrange(1280.0, 720.0, &sizes());
    assert!(!l.toggle("tools", "collapsed"));
    assert!(l.toggle("edit", "collapsed"));
    let a: serde_json::Value = serde_json::from_str(&l.arranged_json(1280.0, 720.0, &[])).unwrap();
    let can = |id: &str| {
        a["palettes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == id)
            .unwrap()["can_collapse"]
            .clone()
    };
    assert_eq!((can("tools"), can("edit")), (false.into(), true.into()));
}

#[test]
fn a_stored_layout_of_the_palettes_before_the_general_one_is_migrated_with_a_note() {
    // the old default had six tool palettes on the left; this one moved bonds, floated rings and collapsed the tools
    let old = r#"{"format":"mzdraw-settings/1","style":{"preset":"acs-1996"},
      "layout":{"snap":20,"palettes":[
        {"id":"tools","collapsed":true,"dock":"right"},
        {"id":"bonds","dock":"float","x":300,"y":200},{"id":"rings","dock":"bottom"},
        {"id":"atoms","collapsed":false},{"id":"charges","hidden":true},{"id":"objects","order":9},
        {"id":"view","hidden":true},{"id":"newer-panel","dock":"left"}]}}"#;
    let mut e = Editor::default();
    let (flags, note) = e.set_style_stored(old).unwrap();
    assert_eq!(flags & PALETTES, PALETTES);
    assert_eq!(note, "draw2.layout.migrated");
    let (r, _) = e.layout.arrange(1280.0, 720.0, &sizes());
    // the old tool palettes are gone, the tools keep the place the user gave them and are whole, what is not theirs is kept
    for id in mzdraw::palettes::LEGACY {
        assert!(r.iter().all(|x| x.id != id), "{id}");
    }
    let tools = rect(&r, "tools");
    assert_eq!((tools.dock, tools.collapsed), (Dock::Right, false));
    assert!(rect(&r, "view").hidden && e.layout.palettes().len() == 11);
    let out = e.settings_json();
    for gone in LEGACY_IDS {
        assert!(!out.contains(gone), "{gone} in {out}");
    }
    assert!(
        out.contains("newer-panel") && out.contains(r#""snap":20"#),
        "{out}"
    );
    // what is written reads back with no note: the migration happens once
    let mut again = Editor::default();
    assert_eq!(again.set_style_stored(&out).unwrap().1, "");
    assert_eq!(again.settings_json(), out);
    // a layout that never had the old palettes gives no note, and neither does one with no layout at all
    for plain in [
        r#"{"format":"mzdraw-settings/1","layout":{"palettes":[{"id":"edit","collapsed":false}]}}"#,
        r#"{"format":"mzdraw-settings/1","style":{"preset":"mzlab"}}"#,
    ] {
        assert_eq!(Editor::default().set_style_stored(plain).unwrap().1, "");
    }
    // a layout whose only change was an old palette is the default again
    let mut only = Editor::default();
    only.set_style_stored(
        r#"{"format":"mzdraw-settings/1","layout":{"palettes":[{"id":"rings","dock":"bottom"}]}}"#,
    )
    .unwrap();
    assert_eq!(only.layout.to_text(), None);
}

const LEGACY_IDS: [&str; 5] = [
    "\"bonds\"",
    "\"rings\"",
    "\"atoms\"",
    "\"charges\"",
    "\"objects\"",
];
