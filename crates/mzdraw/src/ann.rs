//! The annotation of a structure: how sure the identification is, and on what evidence. It is a list of entries (a history: the
//! last entry is the current one), kept on one atom of the structure as its field `ann` of the document, next to `data`.
//!
//! One entry (the field list is `docs/mzdraw-annotations.md`):
//! `{"level", "scale", "identifier", "evidence": [{"kind", "ref", "note"}], "scores": [{"name", "v", "from"}],
//! "source": {"tool", "run"}, "by", "note"}`; only `level` is required. `level` is a string, never a number: the confidence
//! levels of Schymanski et al. 2014 are `"1"`, `"2a"`, `"2b"`, `"3"`, `"4"`, `"5"` (`LEVELS`); any other string is kept as it
//! is and written back unchanged. A key this version does not know is kept with its value.

use serde_json::Value;

/// The confidence levels of Schymanski et al. 2014 (Environ. Sci. Technol. 48, 2097), most confident first.
pub const LEVELS: [&str; 6] = ["1", "2a", "2b", "3", "4", "5"];

/// The name of that scale, the default `scale` of an entry made here.
pub const SCALE: &str = "schymanski-2014";

/// The kinds of evidence the study names; any other kind of the same plain shape is kept.
pub const KINDS: [&str; 7] = [
    "ms2-library",
    "ms2-insilico",
    "rt-standard",
    "diagnostic-fragment",
    "formula",
    "exact-mass",
    "context",
];

/// The SDF data fields an annotation is written as: the level, the identifier and the evidence of the current entry (one line
/// per evidence, `kind: ref`), and the whole annotation as one line of JSON. They are not data fields of the user.
pub const FIELDS: [&str; 4] = [
    "MZLAB_LEVEL",
    "MZLAB_IDENTIFIER",
    "MZLAB_EVIDENCE",
    "MZLAB_ANN",
];

/// Entries of one annotation, items of one list of an entry, and the characters of the whole annotation as text.
pub const MAX: (usize, usize, usize) = (50, 50, 20_000);

/// A text of one line: no control character, no space around it, 1 to `max` characters. An absent one is fine unless needed.
fn line(v: Option<&Value>, max: usize, needed: bool) -> bool {
    match v {
        None => !needed,
        Some(v) => v.as_str().is_some_and(|s| {
            (1..=max).contains(&s.chars().count())
                && s.trim() == s
                && !s.chars().any(char::is_control)
        }),
    }
}

/// A list of at most `MAX.1` objects that each pass `item`; absent is fine.
fn list(v: Option<&Value>, item: impl Fn(&Value) -> bool) -> bool {
    match v {
        None => true,
        Some(v) => v
            .as_array()
            .is_some_and(|l| l.len() <= MAX.1 && l.iter().all(|i| i.is_object() && item(i))),
    }
}

/// A level or a kind of evidence: a short plain word (ASCII letters, digits, `-`, `_`, `.`), so a line `kind: ref` reads back and
/// neither can be taken for a line of an SDF.
fn word(v: Option<&Value>, max: usize) -> bool {
    line(v, max, true)
        && v.and_then(Value::as_str).is_some_and(|s| {
            s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
}

fn entry_valid(e: &Value) -> bool {
    e.is_object()
        && word(e.get("level"), 16)
        && line(e.get("scale"), 40, false)
        && line(e.get("identifier"), 200, false)
        // an identifier is written as a line of an SDF: it cannot start as a field header or as the end of a record
        && !(e.get("identifier").and_then(Value::as_str))
            .is_some_and(|s| s.starts_with('>') || s.starts_with("$$$$"))
        && line(e.get("by"), 40, false)
        && match e.get("note") {
            None => true,
            Some(n) => n.as_str().is_some_and(|s| {
                s.chars().count() <= 2000 && !s.chars().any(|c| c.is_control() && c != '\n')
            }),
        }
        && list(e.get("evidence"), |i| {
            word(i.get("kind"), 40) && line(i.get("ref"), 200, false) && line(i.get("note"), 200, false)
        })
        && list(e.get("scores"), |i| {
            line(i.get("name"), 40, true)
                && i.get("v").is_some_and(Value::is_number)
                && line(i.get("from"), 200, false)
        })
        && match e.get("source") {
            None => true,
            Some(s) => {
                s.is_object() && line(s.get("tool"), 200, false) && line(s.get("run"), 200, false)
            }
        }
}

/// The entries of an annotation given as a JSON value, if it is one: a list of valid entries within the limits.
fn entries(v: &Value) -> Option<&Vec<Value>> {
    let l = v.as_array()?;
    (l.len() <= MAX.0 && l.iter().all(entry_valid) && v.to_string().chars().count() <= MAX.2)
        .then_some(l)
}

/// The one text of an annotation given as JSON text (keys in order, no spaces): what the document, the log and an SDF hold.
/// `None` when the text is not an annotation; the empty list is one (no annotation).
pub fn canon(text: &str) -> Option<String> {
    let v: Value = serde_json::from_str(text).ok()?;
    entries(&v)?;
    Some(v.to_string())
}

/// The evidence of an entry as lines `kind: ref` (`kind` alone without a reference).
pub fn evidence_lines(entry: &Value) -> String {
    let items = entry.get("evidence").and_then(Value::as_array);
    let lines: Vec<String> = (items.into_iter().flatten())
        .map(|i| {
            let word = |k| i.get(k).and_then(Value::as_str).unwrap_or("");
            match word("ref") {
                "" => word("kind").to_string(),
                r => format!("{}: {r}", word("kind")),
            }
        })
        .collect();
    lines.join("\n")
}

/// An annotation of one entry (its `canon` text) from what a person types or an SDF holds: a level, an identifier (may be empty), the evidence as lines
/// `kind: ref` (empty lines are skipped), a note (may be empty) and who decided (`by`, may be empty). `None` when a part is
/// not of its shape.
pub fn entry(
    level: &str,
    identifier: &str,
    evidence: &str,
    note: &str,
    by: &str,
) -> Option<String> {
    let q = |s: &str| Value::from(s).to_string();
    let mut parts = vec![
        format!(r#""level":{}"#, q(level.trim())),
        format!(r#""scale":{}"#, q(SCALE)),
    ];
    for (key, text) in [
        ("identifier", identifier.trim()),
        ("note", note.trim()),
        ("by", by),
    ] {
        if !text.is_empty() {
            parts.push(format!(r#""{key}":{}"#, q(text)));
        }
    }
    let items: Vec<String> = (evidence.lines().map(str::trim).filter(|l| !l.is_empty()))
        .map(|l| match l.split_once(':') {
            Some((k, r)) if !r.trim().is_empty() => {
                format!(r#"{{"kind":{},"ref":{}}}"#, q(k.trim()), q(r.trim()))
            }
            Some((k, _)) => format!(r#"{{"kind":{}}}"#, q(k.trim())),
            None => format!(r#"{{"kind":{}}}"#, q(l)),
        })
        .collect();
    if !items.is_empty() {
        parts.push(format!(r#""evidence":[{}]"#, items.join(",")));
    }
    canon(&format!("[{{{}}}]", parts.join(",")))
}

/// Some annotations (`canon` texts, empty for none) as the text of one list, in the order given: the annotation of a structure
/// is the annotation of its atoms in pool order. The text may be over the limits of one annotation (`canon` says).
pub fn join<'a>(texts: impl Iterator<Item = &'a str>) -> String {
    let inner: Vec<&str> = texts
        .filter(|t| t.len() > 2)
        .map(|t| &t[1..t.len() - 1])
        .collect();
    format!("[{}]", inner.join(","))
}

/// The current entry of an annotation, as a surface shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Current {
    pub level: String,
    /// Whether the level is one of `LEVELS`.
    pub known: bool,
    pub identifier: String,
    /// `evidence_lines` of the entry.
    pub evidence: String,
    pub note: String,
    /// How many entries the annotation has (its history).
    pub entries: usize,
}

/// The current (last) entry of an annotation given as its text.
pub fn current(text: &str) -> Option<Current> {
    let v: Value = serde_json::from_str(text).ok()?;
    let list = v.as_array()?;
    let e = list.last()?;
    let word = |k| e.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    Some(Current {
        known: LEVELS.contains(&word("level").as_str()),
        level: word("level"),
        identifier: word("identifier"),
        evidence: evidence_lines(e),
        note: word("note"),
        entries: list.len(),
    })
}
