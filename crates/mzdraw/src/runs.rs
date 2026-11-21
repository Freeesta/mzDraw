//! Text runs: the formatted text of a text object (and of the text of an arrow). A run is a piece of text with one style, or a line
//! break; a plain one-line text has no runs and stays the `text` field it always was.
//!
//! One run is `{"t":"H"}` with optional keys, written in this order: `br` (a line break, no `t`), `sub` or `sup` (a subscript or a
//! superscript, never both), `b` bold, `i` italic, `u` underline, `font` (a family name; empty is the style's font), `sz` (the size
//! as a multiple of the style's font size, 0.25 to 8; absent is 1) and `color` (`#rrggbb`; absent is the ink of the drawing). A key
//! this version does not know is kept as written and written back (a later version's field survives a round through this one). Runs are kept canonical: no empty run,
//! neighbours of one style merged, so equal texts have equal bytes.
//!
//! The markup of the text field (`_x`, `_{..}`, `^x`, `^{..}`, `\n` or a new line for a line break, a backslash before a character that
//! is meant literally)
//! is read here, in Rust, so that the page, the replay and the CLI give the same runs. The markup carries the scripts and the
//! lines only: the other styles are set with `obj.set` `runs` (the toolbar of the page does it).

use crate::doc::Writer;
use serde_json::Value;

/// Runs of one text, characters in all, and lines.
pub const MAX_RUNS: usize = 400;
pub const MAX_CHARS: usize = 1000;
pub const MAX_LINES: usize = 20;
const MAX_FONT: usize = 64;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    /// The text of the run; empty for a line break.
    pub t: String,
    pub br: bool,
    pub sub: bool,
    pub sup: bool,
    pub b: bool,
    pub i: bool,
    pub u: bool,
    pub font: String,
    /// The size as a multiple of the font of the style; 0 is the style's size.
    pub sz: f64,
    /// `#rrggbb` in lowercase (`ink::canon`), or empty for the ink of the drawing.
    pub color: String,
    /// Keys of a run this version does not know, kept as written (key, value as JSON text, sorted by key) and written back.
    pub more: Vec<(String, String)>,
}

impl Run {
    pub fn plain(t: &str) -> Run {
        Run {
            t: t.into(),
            ..Run::default()
        }
    }

    pub fn sub(t: &str) -> Run {
        Run {
            sub: true,
            ..Run::plain(t)
        }
    }

    pub fn sup(t: &str) -> Run {
        Run {
            sup: true,
            ..Run::plain(t)
        }
    }

    pub fn line() -> Run {
        Run {
            br: true,
            ..Run::default()
        }
    }

    /// Whether two runs have the same style (the text apart).
    fn same_style(&self, o: &Run) -> bool {
        (self.sub, self.sup, self.b, self.i, self.u) == (o.sub, o.sup, o.b, o.i, o.u)
            && self.font == o.font
            && self.sz == o.sz
            && self.color == o.color
            && self.more == o.more
    }

    /// Whether the run has any style (a script, bold, italic, underline, a font, a size or a colour).
    pub fn styled(&self) -> bool {
        !self.same_style(&Run::default())
    }

    /// The key of the first thing wrong with the run, `None` when it is valid.
    fn flaw(&self) -> Option<&'static str> {
        let clean =
            |s: &str, max: usize| s.chars().count() <= max && !s.chars().any(char::is_control);
        // a family name goes into a CSS font shorthand and an XML attribute: letters, digits, space, `-`, `_` and `.` only
        let font_ok = clean(&self.font, MAX_FONT)
            && self
                .font
                .chars()
                .all(|c| c.is_alphanumeric() || " -_.".contains(c));
        if !font_ok {
            return Some("error.draw2.runFont");
        }
        if crate::ink::canon(&self.color).as_deref() != Some(self.color.as_str()) {
            return Some("error.draw2.runColor");
        }
        if !(self.sz == 0.0 || (self.sz.is_finite() && (0.25..=8.0).contains(&self.sz))) {
            return Some("error.draw2.runSize");
        }
        let shape = if self.br {
            self.t.is_empty() && !self.styled()
        } else {
            clean(&self.t, MAX_CHARS) && !self.t.is_empty() && !(self.sub && self.sup)
        };
        (!shape).then_some("error.draw2.runStyle")
    }
}

/// The runs in their canonical form: empty runs dropped, neighbours of one style merged, sizes to two decimals.
pub fn canon(runs: Vec<Run>) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::with_capacity(runs.len());
    for mut r in runs {
        r.sz = (r.sz * 100.0).round() / 100.0;
        // a colour in a short form or in capitals is written in the one form; another text stays and is refused by `check`
        if let Some(c) = crate::ink::canon(&r.color) {
            r.color = c;
        }
        if !r.br && r.t.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(last) if !r.br && !last.br && last.same_style(&r) => last.t.push_str(&r.t),
            _ => out.push(r),
        }
    }
    out
}

/// Whether a list of runs is one a document can hold; the error is the i18n key of the first thing wrong: `error.draw2.runsCount`
/// (more than `MAX_RUNS` runs, `MAX_CHARS` characters or `MAX_LINES` lines), `runFont`, `runColor`, `runSize` or `runStyle` (a
/// script that is both, a break with a style, an empty run, a control character).
pub fn check(runs: &[Run]) -> Result<(), &'static str> {
    // the counts first: nothing is looked at in a list that is too long
    let lines = runs.iter().filter(|r| r.br).count() + 1;
    let chars: usize = runs.iter().map(|r| r.t.len()).sum();
    if runs.len() > MAX_RUNS || chars > 4 * MAX_CHARS || lines > MAX_LINES {
        return Err("error.draw2.runsCount");
    }
    if runs.iter().map(|r| r.t.chars().count()).sum::<usize>() > MAX_CHARS {
        return Err("error.draw2.runsCount");
    }
    match runs.iter().find_map(Run::flaw) {
        Some(key) => Err(key),
        None => Ok(()),
    }
}

/// `check` as a yes or no.
pub fn valid(runs: &[Run]) -> bool {
    check(runs).is_ok()
}

/// A list of runs made valid, for a file that holds one that is not: sizes clamped into 0.25 to 8, a colour that is not a colour,
/// a font name with other characters and a control character left out, a script that is both left as a subscript, a list too long
/// cut. The number is how many runs were changed or cut (0: the list was valid as it was).
pub fn repair(runs: Vec<Run>) -> (Vec<Run>, usize) {
    let mut changed = 0;
    let mut out: Vec<Run> = Vec::new();
    let (mut chars, mut lines) = (0, 1);
    for mut r in canon(runs) {
        if out.len() >= MAX_RUNS {
            changed += 1;
            continue;
        }
        let before = r.clone();
        if r.br {
            lines += 1;
            if lines > MAX_LINES {
                changed += 1;
                continue;
            }
            r = Run::line();
        } else {
            r.t.retain(|c| !c.is_control());
            let room = MAX_CHARS - chars;
            r.t = r.t.chars().take(room).collect();
            chars += r.t.chars().count();
            if r.sub && r.sup {
                r.sup = false;
            }
        }
        if r.sz != 0.0 {
            r.sz = if r.sz.is_finite() {
                r.sz.clamp(0.25, 8.0)
            } else {
                0.0
            };
        }
        if crate::ink::canon(&r.color).is_none() {
            r.color.clear();
        }
        if r.font
            .chars()
            .any(|c| !(c.is_alphanumeric() || " -_.".contains(c)))
            || r.font.chars().count() > MAX_FONT
        {
            r.font.clear();
        }
        changed += usize::from(r != before);
        out.push(r);
    }
    (canon(out), changed)
}

/// The text of the runs without any style, a line break as `\n`.
pub fn plain(runs: &[Run]) -> String {
    let mut s = String::new();
    for r in runs {
        if r.br {
            s.push('\n');
        } else {
            s.push_str(&r.t);
        }
    }
    s
}

/// The runs as JSON text, keys in the order of the module comment.
pub fn to_json(runs: &[Run]) -> String {
    let items: Vec<String> = runs
        .iter()
        .map(|r| {
            let mut w = Writer("{".into());
            if r.br {
                w.raw("br", "true");
            } else {
                w.text("t", &r.t);
            }
            for (k, on) in [
                ("sub", r.sub),
                ("sup", r.sup),
                ("b", r.b),
                ("i", r.i),
                ("u", r.u),
            ] {
                if on {
                    w.raw(k, "true");
                }
            }
            if !r.font.is_empty() {
                w.text("font", &r.font);
            }
            if r.sz != 0.0 {
                w.num("sz", r.sz);
            }
            if !r.color.is_empty() {
                w.text("color", &r.color);
            }
            w.more(&r.more);
            w.0 + "}"
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// The runs of a JSON value (canonical, not yet checked with `check`); `Err` for another shape or a field of the wrong type.
pub(crate) fn from_value(v: &Value) -> Result<Vec<Run>, ()> {
    const KEYS: [&str; 10] = [
        "t", "br", "sub", "sup", "b", "i", "u", "font", "sz", "color",
    ];
    let list = v.as_array().ok_or(())?;
    let mut out = Vec::with_capacity(list.len().min(MAX_RUNS + 1));
    for item in list {
        let o = item.as_object().ok_or(())?;
        let flag = |k: &str| match o.get(k) {
            None | Some(Value::Null) => Ok(false),
            Some(v) => v.as_bool().ok_or(()),
        };
        let text = |k: &str| match o.get(k) {
            None | Some(Value::Null) => Ok(String::new()),
            Some(v) => v.as_str().map(str::to_string).ok_or(()),
        };
        // a key of a later version is kept as written and written back
        let mut more: Vec<(String, String)> = (o.iter())
            .filter(|(k, _)| !KEYS.contains(&k.as_str()))
            .map(|(k, x)| (k.clone(), x.to_string()))
            .collect();
        more.sort();
        out.push(Run {
            t: text("t")?,
            br: flag("br")?,
            sub: flag("sub")?,
            sup: flag("sup")?,
            b: flag("b")?,
            i: flag("i")?,
            u: flag("u")?,
            font: text("font")?,
            sz: match o.get("sz") {
                None | Some(Value::Null) => 0.0,
                Some(v) => v.as_f64().filter(|n| n.is_finite()).ok_or(())?,
            },
            color: text("color")?,
            more,
        });
    }
    Ok(canon(out))
}

/// Whether the text of the field can be edited as markup without losing a style: only scripts and line breaks are used.
pub fn markup_exact(runs: &[Run]) -> bool {
    runs.iter().all(|r| {
        r.br || (!r.b && !r.i && !r.u && r.font.is_empty() && r.sz == 0.0 && r.color.is_empty())
    })
}

/// The runs of the text of the field: `_x` or `_{..}` a subscript, `^x` or `^{..}` a superscript, a new line a line break, a
/// backslash makes the next character literal. A `_` or `^` with nothing to take (the end, a space) is a letter of the text.
pub fn parse(s: &str) -> Vec<Run> {
    let cs: Vec<char> = s.chars().collect();
    let mut out: Vec<Run> = Vec::new();
    let mut plain = String::new();
    let mut i = 0;
    let flush = |out: &mut Vec<Run>, plain: &mut String| {
        if !plain.is_empty() {
            out.push(Run::plain(plain));
            plain.clear();
        }
    };
    while i < cs.len() {
        let c = cs[i];
        match c {
            // `\n` is a line break (the field of the page is one line); another escaped character is literal
            '\\' if cs.get(i + 1) == Some(&'n') => {
                flush(&mut out, &mut plain);
                out.push(Run::line());
                i += 2;
            }
            '\\' if i + 1 < cs.len() => {
                plain.push(cs[i + 1]);
                i += 2;
            }
            '\n' => {
                flush(&mut out, &mut plain);
                out.push(Run::line());
                i += 1;
            }
            '_' | '^' => {
                let mut piece = String::new();
                let mut next = i + 1;
                if cs.get(i + 1) == Some(&'{') {
                    let mut j = i + 2;
                    let mut text = String::new();
                    let mut closed = false;
                    while j < cs.len() {
                        match cs[j] {
                            '}' => {
                                closed = true;
                                break;
                            }
                            '\\' if j + 1 < cs.len() => {
                                text.push(cs[j + 1]);
                                j += 1;
                            }
                            x => text.push(x),
                        }
                        j += 1;
                    }
                    if closed && !text.is_empty() {
                        piece = text;
                        next = j + 1;
                    }
                } else if let Some(&x) = cs.get(i + 1)
                    && !x.is_whitespace()
                    && x != '\\'
                {
                    piece.push(x);
                    next = i + 2;
                }
                if piece.is_empty() {
                    plain.push(c);
                    i += 1;
                } else {
                    flush(&mut out, &mut plain);
                    out.push(if c == '_' {
                        Run::sub(&piece)
                    } else {
                        Run::sup(&piece)
                    });
                    i = next;
                }
            }
            x => {
                plain.push(x);
                i += 1;
            }
        }
    }
    flush(&mut out, &mut plain);
    canon(out)
}

/// The markup that `parse` reads back as these runs (the scripts and the lines only: see `markup_exact`).
pub fn to_markup(runs: &[Run]) -> String {
    let mut s = String::new();
    for r in runs {
        if r.br {
            s.push_str("\\n");
        } else if r.sub || r.sup {
            let mark = if r.sub { '_' } else { '^' };
            let mut it = r.t.chars();
            let single = match (it.next(), it.next()) {
                (Some(c), None) => {
                    Some(c).filter(|c| c.is_alphanumeric() || "+-\u{2212}*'".contains(*c))
                }
                _ => None,
            };
            s.push(mark);
            match single {
                Some(c) => s.push(c),
                None => {
                    s.push('{');
                    for c in r.t.chars() {
                        if matches!(c, '\\' | '}') {
                            s.push('\\');
                        }
                        s.push(c);
                    }
                    s.push('}');
                }
            }
        } else {
            for c in r.t.chars() {
                if matches!(c, '\\' | '_' | '^') {
                    s.push('\\');
                }
                s.push(c);
            }
        }
    }
    s
}

/// The runs a typed text stands for: none for a plain one-line text (the object keeps its `text`; a backslash escape is resolved),
/// else the runs of the markup.
pub fn typed(s: &str) -> (String, Vec<Run>) {
    let runs = parse(s);
    if runs.iter().any(|r| r.br || r.sub || r.sup) {
        (String::new(), runs)
    } else {
        (plain(&runs), Vec::new())
    }
}

/// The digits that follow a letter or a closing parenthesis become subscripts (the formula `CH3CO2H` as `CH`, `3`, `CO`, `2`, `H`);
/// runs that are scripts already are left as they are.
pub fn formula(runs: &[Run]) -> Vec<Run> {
    let mut out = Vec::new();
    let mut prev = ' ';
    for r in runs {
        if r.br || r.sub || r.sup {
            out.push(r.clone());
            prev = ' ';
            continue;
        }
        let mut cur = String::new();
        let mut digits = false;
        for c in r.t.chars() {
            let d = c.is_ascii_digit() && (digits || prev.is_alphabetic() || prev == ')');
            if d != digits && !cur.is_empty() {
                out.push(Run {
                    t: std::mem::take(&mut cur),
                    sub: digits,
                    ..r.clone()
                });
            }
            digits = d;
            cur.push(c);
            prev = c;
        }
        if !cur.is_empty() {
            out.push(Run {
                t: cur,
                sub: digits,
                ..r.clone()
            });
        }
    }
    canon(out)
}

/// Bold is a little wider than the regular face (the one table of advance widths serves all faces).
const BOLD: f64 = 1.07;

/// One piece of laid-out text: a run on a line.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub text: String,
    pub line: usize,
    /// From the start of its line, in pixels.
    pub x: f64,
    /// From the baseline of the first line (the centre line of the display list), in pixels, down.
    pub dy: f64,
    pub size: f64,
    /// 1 bold, 2 italic, 4 underline.
    pub face: u8,
    pub font: String,
    /// `0xrrggbb`, or -1 for the ink of the drawing.
    pub rgb: i32,
    /// The width of the piece, in pixels (the one table of advance widths).
    pub width: f64,
}

/// The runs laid out in lines at the font size `base` pixels, scripts at `script` times their run's size (subscripts lowered,
/// superscripts raised), lines `1.25 * base` apart. `widths` is the width of each line.
pub struct Laid {
    pub pieces: Vec<Piece>,
    pub widths: Vec<f64>,
}

pub fn layout(runs: &[Run], base: f64, script: f64) -> Laid {
    let mut pieces = Vec::new();
    let mut widths = vec![0.0];
    for r in runs {
        if r.br {
            widths.push(0.0);
            continue;
        }
        let line = widths.len() - 1;
        let own = base * if r.sz == 0.0 { 1.0 } else { r.sz };
        let size = if r.sub || r.sup { own * script } else { own };
        let dy = line as f64 * base * 1.25
            + if r.sub {
                own * 0.25
            } else if r.sup {
                -own * 0.4
            } else {
                0.0
            };
        let mut w = crate::scene::width(&r.t, size);
        if r.b {
            w *= BOLD;
        }
        pieces.push(Piece {
            text: r.t.clone(),
            line,
            x: widths[line],
            dy,
            size,
            face: u8::from(r.b) | u8::from(r.i) << 1 | u8::from(r.u) << 2,
            font: r.font.clone(),
            rgb: if r.color.is_empty() {
                -1
            } else {
                i32::from_str_radix(&r.color[1..], 16).unwrap_or(-1)
            },
            width: 0.0,
        });
        pieces.last_mut().expect("just pushed").width = w;
        widths[line] += w;
    }
    Laid { pieces, widths }
}

impl Laid {
    pub fn width(&self) -> f64 {
        self.widths.iter().copied().fold(0.0, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_gives_the_runs_and_back() {
        let cases: [(&str, Vec<Run>); 5] = [
            (
                "H_2O",
                vec![Run::plain("H"), Run::sub("2"), Run::plain("O")],
            ),
            (
                "CH_3CO_2H",
                vec![
                    Run::plain("CH"),
                    Run::sub("3"),
                    Run::plain("CO"),
                    Run::sub("2"),
                    Run::plain("H"),
                ],
            ),
            ("^{13}C", vec![Run::sup("13"), Run::plain("C")]),
            (
                "Pd(OAc)_2, 80 \u{b0}C",
                vec![
                    Run::plain("Pd(OAc)"),
                    Run::sub("2"),
                    Run::plain(", 80 \u{b0}C"),
                ],
            ),
            ("a\\nb", vec![Run::plain("a"), Run::line(), Run::plain("b")]),
        ];
        for (m, want) in cases {
            let got = parse(m);
            assert_eq!(got, want, "{m}");
            assert_eq!(to_markup(&got), m, "{m}");
            assert_eq!(parse(&to_markup(&got)), want);
        }
        // a real new line reads as the same break
        assert_eq!(parse("a\nb"), parse("a\\nb"));
        // braces and escapes
        let r = vec![Run::plain("a_b^c\\"), Run::sub("x y"), Run::sup("}")];
        assert_eq!(parse(&to_markup(&r)), r);
        // nothing to take: literal
        assert_eq!(plain(&parse("a_ b^")), "a_ b^");
        assert_eq!(typed("plain").1, Vec::<Run>::new());
        assert_eq!(typed("a\\_b"), ("a_b".to_string(), Vec::new()));
    }

    #[test]
    fn runs_are_canonical_valid_and_bounded() {
        let c = canon(vec![
            Run::plain("a"),
            Run::plain("b"),
            Run::plain(""),
            Run::sub("2"),
        ]);
        assert_eq!(c, vec![Run::plain("ab"), Run::sub("2")]);
        assert!(valid(&c));
        assert!(!valid(&[Run {
            sub: true,
            sup: true,
            ..Run::plain("x")
        }]));
        assert!(!valid(&[Run {
            color: "red".into(),
            ..Run::plain("x")
        }]));
        assert!(!valid(&[Run {
            sz: 9.0,
            ..Run::plain("x")
        }]));
        assert!(!valid(&[Run {
            b: true,
            ..Run::line()
        }]));
        assert!(!valid(&vec![Run::line(); MAX_LINES]));
        let json: Value = serde_json::from_str(
            r##"[{"t":"H"},{"t":"2","sub":true,"color":"#ff0000","sz":0.8},{"br":true}]"##,
        )
        .unwrap();
        let r = from_value(&json).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&to_json(&r)).unwrap(), json);
        // a key of a later version is kept and written back
        let unknown: Value = serde_json::from_str(r#"[{"t":"H","bold":true}]"#).unwrap();
        let kept = from_value(&unknown).unwrap();
        assert_eq!(kept[0].more, vec![("bold".to_string(), "true".to_string())]);
        assert_eq!(to_json(&kept), r#"[{"t":"H","bold":true}]"#);
    }

    #[test]
    fn the_formula_button_lowers_the_digits_after_symbols() {
        let got = formula(&[Run::plain("CH3CO2H, 5 mol% Pd(OAc)2")]);
        assert_eq!(plain(&got), "CH3CO2H, 5 mol% Pd(OAc)2");
        let subs: Vec<&str> = got.iter().filter(|r| r.sub).map(|r| r.t.as_str()).collect();
        assert_eq!(subs, ["3", "2", "2"]);
    }

    #[test]
    fn layout_lowers_scripts_and_stacks_lines() {
        let l = layout(
            &[Run::plain("H"), Run::sub("2"), Run::line(), Run::sup("+")],
            10.0,
            0.7,
        );
        assert_eq!(l.pieces.len(), 3);
        assert!(l.pieces[1].dy > 0.0 && l.pieces[1].size < 10.0 && l.pieces[1].x > 0.0);
        assert!(l.pieces[2].line == 1 && l.pieces[2].dy > 10.0 - 4.0);
        assert!(l.width() > 0.0);
    }
}
