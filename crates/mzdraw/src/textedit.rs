//! The range logic of the text panel: the style under a caret or over a selection, a change of one style over a range, typing into
//! runs, and the geometry of a caret (where a position is drawn, which position a pointer is on). Positions are counted in characters
//! of the plain text of the runs (`runs::plain`), a line break counting as one. Nothing here knows the page: the editor
//! (`editor_text.rs`) keeps the selection, and every change it makes is an `obj.set` `runs` of the log, so it undoes exactly and
//! replays natively.

use crate::runs::{canon, Run, BOLD, MAX_CHARS, MAX_LINES, MAX_RUNS};

/// The families the panel offers, as (key of the setting `text.font`, the family written in a run); the empty family is the font of
/// the drawing's style. This is the list of the panel and of the preference, not a limit of the document: a run may name any family
/// that `runs::check` accepts, and a file with another one is kept as it is.
pub const FONTS: [(&str, &str); 7] = [
    ("default", ""),
    ("arial", "Arial"),
    ("helvetica", "Helvetica"),
    ("times", "Times New Roman"),
    ("courier", "Courier New"),
    ("georgia", "Georgia"),
    ("verdana", "Verdana"),
];

/// The sizes the panel offers, as percent of the size of the style (a run's `sz` is this divided by 100).
pub const SIZES: [u32; 8] = [50, 75, 100, 125, 150, 200, 300, 400];

/// One property of a run the panel changes.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Bold,
    Italic,
    Underline,
    Sub,
    Sup,
    Font(String),
    /// The size as a multiple of the style's (1 is the style's size).
    Size(f64),
    Colour(String),
}

impl Change {
    /// The change of a panel control: `b`, `i`, `u`, `sub`, `sup` (toggles, no value), `font`, `size` (percent) and `colour`.
    pub fn parse(kind: &str, value: &str) -> Option<Change> {
        Some(match kind {
            "b" => Change::Bold,
            "i" => Change::Italic,
            "u" => Change::Underline,
            "sub" => Change::Sub,
            "sup" => Change::Sup,
            "font" => Change::Font(family(value)?),
            "size" => {
                let pct: f64 = value.trim().parse().ok()?;
                Change::Size(pct / 100.0).filter_size()?
            }
            "colour" => Change::Colour(crate::ink::canon(value)?),
            _ => return None,
        })
    }

    fn filter_size(self) -> Option<Change> {
        match self {
            Change::Size(s) if s.is_finite() && (0.25..=8.0).contains(&s) => Some(self),
            _ => None,
        }
    }

    fn on(&self, r: &Run) -> bool {
        match self {
            Change::Bold => r.b,
            Change::Italic => r.i,
            Change::Underline => r.u,
            Change::Sub => r.sub,
            Change::Sup => r.sup,
            _ => false,
        }
    }

    fn put(&self, r: &mut Run, on: bool) {
        match self {
            Change::Bold => r.b = on,
            Change::Italic => r.i = on,
            Change::Underline => r.u = on,
            Change::Sub => {
                r.sub = on;
                r.sup &= !on;
            }
            Change::Sup => {
                r.sup = on;
                r.sub &= !on;
            }
            Change::Font(f) => r.font.clone_from(f),
            Change::Size(s) => r.sz = if (s - 1.0).abs() < 1e-9 { 0.0 } else { *s },
            Change::Colour(c) => r.color.clone_from(c),
        }
    }
}

/// The family of a font key of `FONTS` or of a family name of the list (`""` and `default` are the style's font); `None` for another.
pub fn family(v: &str) -> Option<String> {
    let v = v.trim();
    FONTS
        .iter()
        .find(|(k, f)| v.eq_ignore_ascii_case(k) || (!f.is_empty() && v == *f) || v.is_empty())
        .map(|(_, f)| f.to_string())
}

/// What a range of text has in common: `Some` where every character has the property, `None` where they differ.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub b: Option<bool>,
    pub i: Option<bool>,
    pub u: Option<bool>,
    pub sub: Option<bool>,
    pub sup: Option<bool>,
    pub font: Option<String>,
    /// The size as a multiple of the style's (1: the style's size).
    pub sz: Option<f64>,
    /// `#rrggbb`, or empty for the ink of the drawing.
    pub color: Option<String>,
}

/// The text as one run per character (a line break is a run), so that a range is a slice; `canon` joins them again.
fn cells(runs: &[Run]) -> Vec<Run> {
    let mut out = Vec::new();
    for r in runs {
        if r.br {
            out.push(r.clone());
        } else {
            for c in r.t.chars() {
                out.push(Run {
                    t: c.to_string(),
                    ..r.clone()
                });
            }
        }
    }
    out
}

/// The runs of a text object: its `runs`, or its plain `text` as one run.
pub fn runs_of(o: &crate::doc::Obj) -> Vec<Run> {
    if o.runs.is_empty() {
        canon(vec![Run::plain(&o.text)])
    } else {
        o.runs.clone()
    }
}

/// The length of the text in characters (a line break is one).
pub fn len(runs: &[Run]) -> usize {
    runs.iter()
        .map(|r| if r.br { 1 } else { r.t.chars().count() })
        .sum()
}

/// The style of the characters `lo..hi`; for an empty range (a caret) the style of the character before it, or after it at the start
/// of the text.
pub fn style_of(runs: &[Run], lo: usize, hi: usize) -> Style {
    let all = cells(runs);
    let (lo, hi) = (lo.min(all.len()), hi.min(all.len()));
    let picked: Vec<&Run> = if lo < hi {
        all[lo..hi].iter().filter(|r| !r.br).collect()
    } else {
        // the character before the caret, else the one after, not a line break
        let before = lo.checked_sub(1).and_then(|i| all.get(i)).filter(|r| !r.br);
        before
            .or_else(|| all.get(lo).filter(|r| !r.br))
            .into_iter()
            .collect()
    };
    let mut it = picked.into_iter();
    let Some(first) = it.next() else {
        return Style {
            b: Some(false),
            i: Some(false),
            u: Some(false),
            sub: Some(false),
            sup: Some(false),
            font: Some(String::new()),
            sz: Some(1.0),
            color: Some(String::new()),
        };
    };
    let sz = |r: &Run| if r.sz == 0.0 { 1.0 } else { r.sz };
    let mut s = Style {
        b: Some(first.b),
        i: Some(first.i),
        u: Some(first.u),
        sub: Some(first.sub),
        sup: Some(first.sup),
        font: Some(first.font.clone()),
        sz: Some(sz(first)),
        color: Some(first.color.clone()),
    };
    for r in it {
        let keep = |a: &mut Option<bool>, v: bool| *a = a.filter(|x| *x == v);
        keep(&mut s.b, r.b);
        keep(&mut s.i, r.i);
        keep(&mut s.u, r.u);
        keep(&mut s.sub, r.sub);
        keep(&mut s.sup, r.sup);
        s.font = s.font.filter(|f| *f == r.font);
        s.sz = s.sz.filter(|z| (*z - sz(r)).abs() < 1e-9);
        s.color = s.color.filter(|c| *c == r.color);
    }
    s
}

/// The runs with `change` applied to the characters `lo..hi` (clamped to the text; a line break takes no style). A toggle (bold,
/// italic, underline, sub, sup) turns the property off when every character of the range has it and on otherwise, so a mixed range
/// becomes all on, as in a word processor; sub and sup exclude each other. The text and its length stay as they were.
pub fn apply(runs: &[Run], lo: usize, hi: usize, change: &Change) -> Vec<Run> {
    let mut all = cells(runs);
    let (lo, hi) = (lo.min(all.len()), hi.min(all.len()));
    let toggle = matches!(
        change,
        Change::Bold | Change::Italic | Change::Underline | Change::Sub | Change::Sup
    );
    let on = !all[lo..hi].iter().filter(|r| !r.br).all(|r| change.on(r))
        || all[lo..hi].iter().all(|r| r.br);
    for r in all[lo..hi].iter_mut().filter(|r| !r.br) {
        change.put(r, if toggle { on } else { true });
    }
    canon(all)
}

/// The style a typed character takes at the caret `pos`: that of the character before it (or after it at the start), as a run with no
/// text.
pub fn template(runs: &[Run], pos: usize) -> Run {
    let all = cells(runs);
    let before = pos
        .checked_sub(1)
        .and_then(|i| all.get(i))
        .filter(|r| !r.br);
    let base = before.or_else(|| all.get(pos).filter(|r| !r.br));
    Run {
        t: String::new(),
        ..base.cloned().unwrap_or_default()
    }
}

/// The runs with the characters `lo..hi` replaced by `text` in the style `style` (`template`). A `\n` is a line break and any other
/// control character is left out. What does not fit the caps of a text (characters, lines, runs) is cut off the end of `text`; the
/// number is how many characters were put in; a result of more runs than the cap leaves the text as it was (0 put in).
pub fn splice(runs: &[Run], lo: usize, hi: usize, text: &str, style: &Run) -> (Vec<Run>, usize) {
    let mut all = cells(runs);
    let (lo, hi) = (lo.min(all.len()), hi.min(all.len()));
    let rest = all.len() - (hi - lo);
    let mut lines = all[..lo].iter().chain(&all[hi..]).filter(|r| r.br).count() + 1;
    let mut room = MAX_CHARS.saturating_sub(rest);
    let mut put = Vec::new();
    for c in text.chars() {
        if room == 0 {
            break;
        }
        if c == '\n' {
            if lines >= MAX_LINES {
                continue;
            }
            lines += 1;
            put.push(Run::line());
        } else if !c.is_control() {
            put.push(Run {
                t: c.to_string(),
                ..style.clone()
            });
        } else {
            continue;
        }
        room -= 1;
    }
    let n = put.len();
    all.splice(lo..hi, put);
    let mut out = canon(all);
    // a run count over the cap (a text of many one-letter styles) loses the typed characters, never the text
    if out.len() > MAX_RUNS {
        return (canon(cells(runs)), 0);
    }
    out.shrink_to_fit();
    (out, n)
}

/// The runs with the characters `lo..hi` removed.
pub fn remove(runs: &[Run], lo: usize, hi: usize) -> Vec<Run> {
    splice(runs, lo, hi, "", &Run::default()).0
}

/// The width of one character of a run, as `runs::layout` measures it.
fn char_width(r: &Run, c: char, base: f64, script: f64) -> f64 {
    let own = base * if r.sz == 0.0 { 1.0 } else { r.sz };
    let size = if r.sub || r.sup { own * script } else { own };
    crate::scene::width(&c.to_string(), size) * if r.b { BOLD } else { 1.0 }
}

/// Where the position `pos` is drawn: the line and the distance from the start of the line, in pixels, at the font size `base`.
pub fn caret_at(runs: &[Run], base: f64, script: f64, pos: usize) -> (usize, f64) {
    let (mut line, mut x, mut i) = (0, 0.0, 0);
    for r in runs {
        if r.br {
            if i >= pos {
                break;
            }
            i += 1;
            line += 1;
            x = 0.0;
            continue;
        }
        for c in r.t.chars() {
            if i >= pos {
                return (line, x);
            }
            x += char_width(r, c, base, script);
            i += 1;
        }
    }
    (line, x)
}

/// The position of the pointer: the line `line` (clamped) and the distance `x` from its start, in pixels; the nearest boundary
/// between two characters.
pub fn pos_at(runs: &[Run], base: f64, script: f64, line: i64, x: f64) -> usize {
    let lines = runs.iter().filter(|r| r.br).count() as i64;
    let want = line.clamp(0, lines);
    let (mut l, mut acc, mut i) = (0i64, 0.0, 0usize);
    let mut best = 0usize;
    let mut found = false;
    for r in runs {
        if r.br {
            if l == want {
                return best_of(best, found, i);
            }
            i += 1;
            l += 1;
            acc = 0.0;
            if l == want {
                best = i;
                found = true;
            }
            continue;
        }
        if l == 0 && want == 0 && !found {
            best = 0;
            found = true;
        }
        for c in r.t.chars() {
            if l == want {
                let w = char_width(r, c, base, script);
                if x >= acc + w / 2.0 {
                    best = i + 1;
                }
                acc += w;
            }
            i += 1;
        }
    }
    best_of(best, found, i)
}

fn best_of(best: usize, found: bool, end: usize) -> usize {
    if found {
        best
    } else {
        end
    }
}

/// The start and the end of the line that holds the position `pos`.
pub fn line_span(runs: &[Run], pos: usize) -> (usize, usize) {
    let (mut start, mut i) = (0, 0);
    for r in runs {
        if r.br {
            if i >= pos {
                return (start, i);
            }
            start = i + 1;
            i += 1;
        } else {
            i += r.t.chars().count();
        }
    }
    (start, i)
}

/// The start and the end of the word that holds `pos` (letters and digits), or the single character at it.
pub fn word_span(runs: &[Run], pos: usize) -> (usize, usize) {
    let text: Vec<char> = crate::runs::plain(runs).chars().collect();
    let at = pos.min(text.len().saturating_sub(1));
    let word = |c: char| c.is_alphanumeric();
    if text.is_empty() {
        return (0, 0);
    }
    if !word(text[at]) {
        return (at, (at + 1).min(text.len()));
    }
    let (mut a, mut b) = (at, at + 1);
    while a > 0 && word(text[a - 1]) {
        a -= 1;
    }
    while b < text.len() && word(text[b]) {
        b += 1;
    }
    (a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bold(t: &str) -> Run {
        Run {
            b: true,
            ..Run::plain(t)
        }
    }

    #[test]
    fn the_style_under_a_caret_is_that_of_the_character_before_it() {
        let runs = vec![Run::plain("ab"), bold("cd")];
        assert_eq!(style_of(&runs, 2, 2).b, Some(false));
        assert_eq!(style_of(&runs, 3, 3).b, Some(true));
        assert_eq!(style_of(&runs, 0, 0).b, Some(false));
        // at the start of a text that begins bold, the style of the first character
        assert_eq!(style_of(&[bold("x")], 0, 0).b, Some(true));
        assert_eq!(style_of(&[], 0, 0).sz, Some(1.0));
    }

    #[test]
    fn a_mixed_range_has_no_common_value() {
        let runs = vec![Run::plain("ab"), bold("cd")];
        let s = style_of(&runs, 1, 3);
        assert_eq!((s.b, s.i, s.font), (None, Some(false), Some(String::new())));
        assert_eq!(style_of(&runs, 2, 4).b, Some(true));
    }

    #[test]
    fn a_toggle_over_a_mixed_range_turns_it_on_then_off() {
        let runs = vec![Run::plain("ab"), bold("cd")];
        let on = apply(&runs, 0, 4, &Change::Bold);
        assert_eq!(on, vec![bold("abcd")]);
        let off = apply(&on, 1, 3, &Change::Bold);
        assert_eq!(off, vec![bold("a"), Run::plain("bc"), bold("d")]);
        assert_eq!(apply(&on, 0, 4, &Change::Bold), vec![Run::plain("abcd")]);
    }

    #[test]
    fn only_the_range_changes_and_the_text_stays() {
        let runs = vec![Run::plain("H2O and more")];
        let out = apply(&runs, 1, 2, &Change::Sub);
        assert_eq!(
            out,
            vec![Run::plain("H"), Run::sub("2"), Run::plain("O and more")]
        );
        assert_eq!(crate::runs::plain(&out), "H2O and more");
        // sub and sup exclude each other
        let sup = apply(&out, 1, 2, &Change::Sup);
        assert_eq!(sup[1], Run::sup("2"));
    }

    #[test]
    fn adjacent_runs_of_one_style_merge() {
        let runs = vec![bold("a"), Run::plain("b"), bold("c")];
        assert_eq!(apply(&runs, 1, 2, &Change::Bold), vec![bold("abc")]);
        let sized = apply(&[Run::plain("ab")], 0, 1, &Change::Size(2.0));
        let back = apply(&sized, 0, 1, &Change::Size(1.0));
        assert_eq!(back, vec![Run::plain("ab")]);
    }

    #[test]
    fn font_size_and_colour_set_the_whole_range() {
        let runs = vec![Run::plain("abc")];
        let out = apply(&runs, 1, 3, &Change::Colour("#d00000".into()));
        assert_eq!(out[1].color, "#d00000");
        assert_eq!(out[0].color, "");
        let out = apply(&out, 0, 3, &Change::Font("Courier New".into()));
        assert!(out.iter().all(|r| r.font == "Courier New"));
        // the colour back to automatic merges the runs again
        let out = apply(&out, 0, 3, &Change::Colour(String::new()));
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn a_line_break_takes_no_style() {
        let runs = vec![Run::plain("a"), Run::line(), Run::plain("b")];
        let out = apply(&runs, 0, 3, &Change::Bold);
        assert_eq!(out, vec![bold("a"), Run::line(), bold("b")]);
        assert!(crate::runs::valid(&out));
    }

    #[test]
    fn typing_takes_the_style_before_the_caret_and_a_newline_breaks_the_line() {
        let runs = vec![Run::plain("a"), bold("b")];
        let (out, n) = splice(&runs, 2, 2, "c\nd", &template(&runs, 2));
        assert_eq!(n, 3);
        assert_eq!(
            out,
            vec![Run::plain("a"), bold("bc"), Run::line(), bold("d")]
        );
        // replacing a range
        let (out, _) = splice(&runs, 0, 2, "x", &template(&runs, 0));
        assert_eq!(out, vec![Run::plain("x")]);
        assert_eq!(remove(&runs, 0, 1), vec![bold("b")]);
    }

    #[test]
    fn typing_stops_at_the_caps_and_a_range_that_crosses_the_character_cap_is_cut() {
        let full = vec![Run::plain(&"a".repeat(MAX_CHARS - 2))];
        // two characters of room: of five typed, two go in
        let (out, n) = splice(
            &full,
            MAX_CHARS - 2,
            MAX_CHARS - 2,
            "vwxyz",
            &Run::default(),
        );
        assert_eq!(n, 2);
        assert_eq!(len(&out), MAX_CHARS);
        assert!(crate::runs::valid(&out));
        // a replaced range frees its room: replacing 10 characters of a full text by 12 puts in 10, never more than the cap
        let (out, n) = splice(&out, 0, 10, &"b".repeat(12), &Run::default());
        assert_eq!((n, len(&out)), (10, MAX_CHARS));
        // a range that runs past the end of the text is the rest of it
        let styled = apply(&full, MAX_CHARS - 10, MAX_CHARS + 50, &Change::Bold);
        assert_eq!(len(&styled), MAX_CHARS - 2);
        assert!(crate::runs::valid(&styled));
        // lines: a text of the most lines takes no more
        let mut lines = vec![Run::plain("x")];
        for _ in 1..MAX_LINES {
            lines.push(Run::line());
            lines.push(Run::plain("x"));
        }
        let (out, n) = splice(&lines, 1, 1, "\n", &Run::default());
        assert_eq!((n, out.iter().filter(|r| r.br).count()), (0, MAX_LINES - 1));
    }

    #[test]
    fn a_styling_that_would_pass_the_run_cap_is_refused_whole_by_the_check() {
        // 500 one-letter runs of alternating styles are more than the cap: the list is not valid, and the editor says so
        let text: String = "ab".repeat(250);
        let runs = vec![Run::plain(&text)];
        let mut out = runs.clone();
        for i in (0..500).step_by(2) {
            out = apply(&out, i, i + 1, &Change::Bold);
        }
        assert!(out.len() > MAX_RUNS);
        assert_eq!(crate::runs::check(&out), Err("error.draw2.runsCount"));
    }

    #[test]
    fn the_caret_and_the_pointer_agree() {
        let runs = vec![Run::plain("ab"), Run::line(), bold("cd")];
        let (base, script) = (20.0, 0.7);
        let (line, x) = caret_at(&runs, base, script, 4);
        assert_eq!(line, 1);
        for pos in 0..=len(&runs) {
            let (l, x) = caret_at(&runs, base, script, pos);
            assert_eq!(pos_at(&runs, base, script, l as i64, x), pos, "{pos}");
        }
        assert!(x > 0.0);
        // far right of a line is its end, a line below the last is the last
        assert_eq!(pos_at(&runs, base, script, 0, 1e6), 2);
        assert_eq!(pos_at(&runs, base, script, 9, 1e6), 5);
        assert_eq!(pos_at(&runs, base, script, 1, -5.0), 3);
        assert_eq!(line_span(&runs, 1), (0, 2));
        assert_eq!(line_span(&runs, 4), (3, 5));
        assert_eq!(word_span(&runs, 3), (3, 5));
    }

    #[test]
    fn a_panel_control_is_read_in_one_place() {
        assert_eq!(Change::parse("b", ""), Some(Change::Bold));
        assert_eq!(Change::parse("size", "150"), Some(Change::Size(1.5)));
        assert_eq!(Change::parse("size", "9000"), None);
        assert_eq!(Change::parse("size", "x"), None);
        assert_eq!(
            Change::parse("colour", "#D00"),
            Some(Change::Colour("#dd0000".into()))
        );
        assert_eq!(Change::parse("colour", "red"), None);
        assert_eq!(
            Change::parse("font", "times"),
            Some(Change::Font("Times New Roman".into()))
        );
        assert_eq!(Change::parse("font", "Comic Sans"), None);
        assert_eq!(Change::parse("font", ""), Some(Change::Font(String::new())));
    }
}
