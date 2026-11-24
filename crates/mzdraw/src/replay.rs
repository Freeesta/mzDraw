//! The headless door: a request with a starting document or text, a command log and the output wanted gives the output text. The
//! `.wasm`, the app and the CLI call this one function, so a session of the editor replays to the same bytes anywhere.

use crate::doc::Document;
use crate::editor::Editor;
use crate::ops::{Op, OpError};
use crate::outcome::{read_expect, Expectation, Failed, IdSets, Outcome};

/// The request, read by hand from its JSON value (as the operations and the document are).
#[derive(Default)]
struct Request {
    /// `diff`: two documents; the answer is the patch from the first to the second and its hunks (`diff::json`). The request stands alone.
    diff: Option<(crate::doc::Document, crate::doc::Document)>,
    /// A structure text to start from: `mzdraw/1` JSON, a molfile or a SMILES.
    import: Option<String>,
    /// The bytes of a file to start from, in base64 (RFC 4648, padding optional, blanks ignored): a binary CDX file, or a PNG or a PDF
    /// saved by mzDraw. Read after `import` when both are given.
    import_base64: Option<Vec<u8>>,
    /// `paste`: a structure text pasted as the page pastes it (`Editor::paste`: the free place nearest to the view centre), after
    /// `commands` and before `events`; `canvas`: the size of the canvas in pixels `[w, h]` the view centre is taken from (default 400 x 400,
    /// which puts the centre at the origin of the default view). A refused paste is the error of the request.
    paste: Option<String>,
    canvas: Option<(f64, f64)>,
    log: Vec<Op>,
    /// `propose`: a patch held as a proposal (`Editor::propose`) after `commands`; `accept`: the hunks (1-based, as `diff::hunks` numbers
    /// them) accepted in that order, one undo step each. The log then holds exactly their operations; the hunks left are dropped.
    propose: Option<Vec<Op>>,
    accept: Vec<usize>,
    /// What `Editor::save` writes: `mzdraw` (default), `molfile`, `molfile-v3000`, `smiles`, `ket`, `cdxml`, `cml`, `rxn`, `trasf` (the record `trasf/1` and its two records, one per line), `sdf`, `svg`, `pdf` (the PDF file as base64; the CLI `--raw` writes the file), `log`, `std` (the drawing checks `std/1`), `schema` (the table of the operations, `draw-edit-schema/1`) or `settings` (the settings file with the style, `layout` and `export`: what `set` left).
    out: Option<String>,
    /// The `args` of an output that takes some (`sites`: `{"of": atom id, "step": NAME}`), read when there is no `command`.
    out_args: Option<serde_json::Value>,
    /// The style of the SVG: a preset name or the text of a settings file; default `mzlab`.
    style: Option<String>,
    /// Settings applied after `style`, each `<section>.<key>=<value>` (`Editor::set`): the `--set` of the CLI.
    set: Vec<String>,
    /// Editor commands run after `log` and before `out`: `(name, args)`. The singular `"command"` + `"args"` is a list of one.
    commands: Vec<(String, serde_json::Value)>,
    /// `report`: answer with one `draw-result/1` document (see `outcome`).
    report: bool,
    /// `dry_run`: everything runs and is reported, nothing is written (`out` is left out); implies the document.
    dry_run: bool,
    /// `expect`: postconditions on the final drawing; a non-empty list implies the document.
    expect: Vec<Expectation>,
    /// `id`: echoed in the document (a string or an integer).
    id: Option<serde_json::Value>,
    /// `events`: pointer and key events the page would forward, run after `commands` and before `out`: `["pointer", phase, id, x, y,
    /// kind, mods, t]` (`Editor::pointer_event`) and `["key", key, mods, t]` (`Editor::key_at`). Their flags are kept for the
    /// output `context-state:<w>,<h>`.
    events: Vec<Event>,
    /// `keys`: keys pressed over an object, run after `events` and before `out`, each through `Editor::key_over`: `{"key", "mods",
    /// "hover": {"atom": id} or {"bond": id}, "focus": {"atom": id, "from": id}}` (`mods` 0 when absent; `hover` and `focus` optional).
    keys: Vec<KeyReq>,
    /// `values`: the answer is `values/1` (`values::document_json`): the calculated values of every structure and, when `atoms` is
    /// given, of those atoms as one selection. `true` is no selection. It stands alone with the editing fields (no `out`, no `report`).
    values: Option<Option<Vec<u32>>>,
    /// `props`: the answer is `props/1` (`values::props_json`), or, with `text`, the values copied as text (`values::props_text`):
    /// `(atoms of a selection, form, line keys)`. Like `values` it stands alone.
    props: Option<PropsRequest>,
    /// `feedback`: the answer is `feedback/1` (`feedback::build`): `(kind, with the command log, user agent, language)`.
    feedback: Option<(String, bool, String, String)>,
}

/// One entry of the request field `keys`: the key, its modifiers, the pointer (atom: true, else bond) and the focus (atom and origin).
struct KeyReq {
    key: String,
    mods: u32,
    hover: Option<(bool, u32)>,
    focus: Option<(u32, Option<u32>)>,
}

/// One event of the request field `events`.
enum Event {
    Pointer([f64; 7]),
    Key(String, u32, f64),
}

/// The fields of a request. A field outside this list is refused (`error.draw2.requestField`); `engines` belongs to the glue door.
const FIELDS: [&str; 23] = [
    "keys",
    "propose",
    "accept",
    "diff",
    "import",
    "import_base64",
    "style",
    "set",
    "log",
    "commands",
    "command",
    "args",
    "out",
    "report",
    "dry_run",
    "expect",
    "id",
    "events",
    "values",
    "props",
    "feedback",
    "paste",
    "canvas",
];

/// The request field `props`: the atoms of a selection, the form of a copy as text, the line keys to copy.
type PropsRequest = (Option<Vec<u32>>, Option<String>, Vec<String>);

impl Request {
    fn read(text: &str) -> Result<Request, &'static str> {
        const BAD: &str = "error.draw2.request";
        let v: serde_json::Value = serde_json::from_str(text).map_err(|_| BAD)?;
        let text_of = |k: &str| match v.get(k) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(x) => x.as_str().map(|s| Some(s.to_string())).ok_or(BAD),
        };
        let log = match v.get("log") {
            None => Vec::new(),
            Some(l) => l
                .as_array()
                .ok_or(BAD)?
                .iter()
                .map(Op::parse)
                .collect::<Result<Vec<Op>, _>>()?,
        };
        let set = match v.get("set") {
            None => Vec::new(),
            Some(l) => (l.as_array().ok_or(BAD)?.iter())
                .map(|s| s.as_str().map(str::to_string))
                .collect::<Option<Vec<String>>>()
                .ok_or(BAD)?,
        };
        let obj = v.as_object().ok_or(BAD)?;
        if obj.keys().any(|k| !FIELDS.contains(&k.as_str())) {
            return Err("error.draw2.requestField");
        }
        if let Some(d) = v.get("diff") {
            // alone: no other field, not even `report`, `dry_run`, `expect` or `id` (the answer is not a `draw-result/1`)
            if obj.len() > 1 {
                return Err("error.draw2.diffAlone");
            }
            let pair = d.as_object().filter(|o| o.len() == 2).ok_or(BAD)?;
            // a document is an `mzdraw/1` object or its JSON text
            let read = |k: &str| {
                let doc = pair.get(k).ok_or(BAD)?;
                match doc.as_str() {
                    Some(text) => Document::from_json(text),
                    None => Document::from_json(&doc.to_string()),
                }
            };
            return Ok(Request {
                diff: Some((read("a")?, read("b")?)),
                ..Request::default()
            });
        }
        let propose = match v.get("propose") {
            None => None,
            Some(l) => Some(
                (l.as_array().ok_or(BAD)?.iter())
                    .map(Op::parse)
                    .collect::<Result<Vec<Op>, _>>()?,
            ),
        };
        let accept = match v.get("accept") {
            None => Vec::new(),
            // the hunks of a proposal: there must be one
            Some(_) if propose.is_none() => return Err(BAD),
            Some(l) => (l.as_array().ok_or(BAD)?.iter())
                .map(|n| {
                    n.as_u64()
                        .filter(|&n| (1..=u64::from(u32::MAX)).contains(&n))
                })
                .map(|n| n.map(|n| n as usize))
                .collect::<Option<Vec<usize>>>()
                .ok_or(BAD)?,
        };
        let one = |c: &serde_json::Value, args: Option<&serde_json::Value>| {
            let name = c.as_str().ok_or(BAD)?;
            let args = args.cloned().unwrap_or_else(|| serde_json::json!({}));
            crate::commands::check(name, &args)?;
            Ok((name.to_string(), args))
        };
        let commands = match (v.get("command"), v.get("commands")) {
            (Some(_), Some(_)) => return Err(BAD),
            (Some(c), None) => vec![one(c, v.get("args"))?],
            (None, Some(l)) => (l.as_array().ok_or(BAD)?.iter())
                .map(|c| {
                    // a bare name is a command without arguments (`"align:left"`)
                    if c.is_string() {
                        return one(c, None);
                    }
                    // an item is `{"command", "args"}` and nothing else
                    let item = c.as_object().ok_or(BAD)?;
                    if item.keys().any(|k| k != "command" && k != "args") {
                        return Err("error.draw2.requestField");
                    }
                    one(c.get("command").ok_or(BAD)?, c.get("args"))
                })
                .collect::<Result<_, _>>()?,
            (None, None) => Vec::new(),
        };
        // `args` belongs to `command`, except for the output `sites`, which takes `{"of", "step"}`
        let out_args = match (v.get("args"), v.get("command")) {
            (Some(a), None) if v.get("out").and_then(|o| o.as_str()) == Some("sites") => {
                crate::tp::sites_args(a)?;
                Some(a.clone())
            }
            (Some(a), None) if v.get("out").and_then(|o| o.as_str()) == Some("pathway-check") => {
                crate::pathcheck::Opts::parse(Some(a))?;
                Some(a.clone())
            }
            (Some(_), None) => return Err(BAD),
            _ => None,
        };
        let flag = |k: &str| match v.get(k) {
            None => Ok(false),
            Some(x) => x.as_bool().ok_or(BAD),
        };
        let id = match v.get("id") {
            None => None,
            Some(x) if x.is_string() || x.is_i64() || x.is_u64() => Some(x.clone()),
            Some(_) => return Err(BAD),
        };
        let import_base64 = match text_of("import_base64")? {
            Some(t) => Some(base64(&t).ok_or(BAD)?),
            None => None,
        };
        let canvas = match v.get("canvas") {
            None | Some(serde_json::Value::Null) => None,
            Some(c) => match c
                .as_array()
                .map(|a| a.iter().map(serde_json::Value::as_f64).collect::<Vec<_>>())
            {
                Some(n) if n.len() == 2 => match (n[0], n[1]) {
                    (Some(w), Some(h)) if w > 0.0 && h > 0.0 => Some((w, h)),
                    _ => return Err(BAD),
                },
                _ => return Err(BAD),
            },
        };
        let events = match v.get("events") {
            None => Vec::new(),
            Some(l) => (l.as_array().ok_or(BAD)?.iter())
                .map(|e| {
                    let a = e.as_array().ok_or(BAD)?;
                    let num = |i: usize| a.get(i).and_then(serde_json::Value::as_f64).ok_or(BAD);
                    match (a.first().and_then(|k| k.as_str()), a.len()) {
                        (Some("pointer"), 8) => {
                            let mut n = [0.0; 7];
                            for (i, v) in n.iter_mut().enumerate() {
                                *v = num(i + 1)?;
                            }
                            Ok(Event::Pointer(n))
                        }
                        (Some("key"), 4) => Ok(Event::Key(
                            a[1].as_str().ok_or(BAD)?.to_string(),
                            num(2)? as u32,
                            num(3)?,
                        )),
                        _ => Err(BAD),
                    }
                })
                .collect::<Result<Vec<Event>, _>>()?,
        };
        let keys = match v.get("keys") {
            None => Vec::new(),
            Some(l) => (l.as_array().ok_or(BAD)?.iter())
                .map(|k| {
                    let bad = "error.draw2.keys";
                    let o = k.as_object().ok_or(bad)?;
                    if o.keys()
                        .any(|x| !["key", "mods", "hover", "focus"].contains(&x.as_str()))
                    {
                        return Err(bad);
                    }
                    let id = |x: &serde_json::Value| {
                        x.as_u64().and_then(|n| u32::try_from(n).ok()).ok_or(bad)
                    };
                    let hover = match o.get("hover") {
                        None => None,
                        Some(h) => {
                            match (h.get("atom"), h.get("bond"), h.as_object().map(|h| h.len())) {
                                (Some(a), None, Some(1)) => Some((true, id(a)?)),
                                (None, Some(b), Some(1)) => Some((false, id(b)?)),
                                _ => return Err(bad),
                            }
                        }
                    };
                    let focus = match o.get("focus") {
                        None => None,
                        Some(f) => {
                            let f = f
                                .as_object()
                                .filter(|f| f.keys().all(|k| k == "atom" || k == "from"))
                                .ok_or(bad)?;
                            let from = f.get("from").map(id).transpose()?;
                            Some((id(f.get("atom").ok_or(bad)?)?, from))
                        }
                    };
                    let mods = match o.get("mods") {
                        None => 0,
                        Some(m) => u32::try_from(m.as_u64().ok_or(bad)?).map_err(|_| bad)?,
                    };
                    Ok(KeyReq {
                        key: o
                            .get("key")
                            .and_then(|k| k.as_str())
                            .ok_or(bad)?
                            .to_string(),
                        mods,
                        hover,
                        focus,
                    })
                })
                .collect::<Result<Vec<KeyReq>, _>>()?,
        };
        let values = match v.get("values") {
            None | Some(serde_json::Value::Bool(false)) => None,
            Some(serde_json::Value::Bool(true)) => Some(None),
            Some(o) => {
                // `{"atoms": [id, ...]}` and nothing else
                let o = o.as_object().filter(|o| o.len() == 1).ok_or(BAD)?;
                let ids = (o.get("atoms").and_then(|a| a.as_array()).ok_or(BAD)?.iter())
                    .map(|n| n.as_u64().filter(|&n| n <= u64::from(u32::MAX)))
                    .map(|n| n.map(|n| n as u32))
                    .collect::<Option<Vec<u32>>>()
                    .ok_or(BAD)?;
                Some(Some(ids))
            }
        };
        let props = match v.get("props") {
            None | Some(serde_json::Value::Bool(false)) => None,
            Some(serde_json::Value::Bool(true)) => Some((None, None, Vec::new())),
            // `{"atoms": [id, ...], "text": "text|tsv|line", "lines": [key, ...]}`, each optional
            Some(o) => {
                let bad = "error.draw2.props";
                let o = o.as_object().ok_or(bad)?;
                if o.keys()
                    .any(|k| !["atoms", "text", "lines"].contains(&k.as_str()))
                {
                    return Err(bad);
                }
                let atoms = match o.get("atoms") {
                    None => None,
                    Some(a) => Some(
                        (a.as_array().ok_or(bad)?.iter())
                            .map(|n| n.as_u64().and_then(|n| u32::try_from(n).ok()))
                            .collect::<Option<Vec<u32>>>()
                            .ok_or(bad)?,
                    ),
                };
                let text = match o.get("text") {
                    None => None,
                    Some(t) => Some(t.as_str().ok_or(bad)?.to_string()),
                };
                if text
                    .as_deref()
                    .is_some_and(|t| !crate::values::FORMS.contains(&t))
                {
                    return Err(bad);
                }
                let lines = match o.get("lines") {
                    None => Vec::new(),
                    Some(l) => (l.as_array().ok_or(bad)?.iter())
                        .map(|x| x.as_str().map(str::to_string))
                        .collect::<Option<Vec<String>>>()
                        .ok_or(bad)?,
                };
                Some((atoms, text, lines))
            }
        };
        let feedback = match v.get("feedback") {
            None => None,
            // `"bug"`, or `{"kind", "log", "agent", "lang"}` and nothing else (`kind` is the only one required)
            Some(f) => {
                let bad = "error.draw2.feedback";
                let text = |o: &serde_json::Value, k: &str| match o.get(k) {
                    None => Ok(String::new()),
                    Some(x) => x.as_str().map(str::to_string).ok_or(bad),
                };
                match f {
                    serde_json::Value::String(kind) => {
                        Some((kind.clone(), false, String::new(), String::new()))
                    }
                    serde_json::Value::Object(o) => {
                        if o.keys()
                            .any(|k| !["kind", "log", "agent", "lang"].contains(&k.as_str()))
                        {
                            return Err(bad);
                        }
                        let log = match o.get("log") {
                            None => false,
                            Some(x) => x.as_bool().ok_or(bad)?,
                        };
                        Some((text(f, "kind")?, log, text(f, "agent")?, text(f, "lang")?))
                    }
                    _ => return Err(bad),
                }
            }
        };
        if feedback.is_some()
            && (values.is_some()
                || props.is_some()
                || propose.is_some()
                || v.get("id").is_some()
                || v.get("out").is_some()
                || flag("report")?
                || flag("dry_run")?
                || v.get("expect").is_some())
        {
            return Err("error.draw2.feedbackAlone");
        }
        if (values.is_some() || props.is_some())
            && (v.get("out").is_some()
                || (values.is_some() && props.is_some())
                || flag("report")?
                || flag("dry_run")?
                || v.get("expect").is_some())
        {
            return Err("error.draw2.valuesAlone");
        }
        Ok(Request {
            diff: None,
            values,
            props,
            feedback,
            propose,
            accept,
            events,
            keys,
            commands,
            report: flag("report")?,
            dry_run: flag("dry_run")?,
            expect: v.get("expect").map_or(Ok(Vec::new()), read_expect)?,
            id,
            import: text_of("import")?,
            import_base64,
            paste: text_of("paste")?,
            canvas,
            log,
            out: text_of("out")?,
            out_args,
            set,
            style: match v.get("style") {
                // a settings object may be given in place: it is read as its own text
                Some(s) if s.is_object() => Some(s.to_string()),
                _ => text_of("style")?,
            },
        })
    }
}

/// The bytes of a standard-alphabet base64 text; blanks are ignored and the padding is optional. `None` for any other character,
/// for a misplaced `=` or for a length no base64 text has.
fn base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut acc, mut bits, mut pad) = (0u32, 0, 0);
    for c in text.bytes().filter(|c| !c.is_ascii_whitespace()) {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => {
                pad += 1;
                continue;
            }
            _ => return None,
        };
        // a character after the padding
        if pad > 0 {
            return None;
        }
        acc = acc << 6 | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    // what is left is under one byte; a single character cannot be a quantum
    (pad <= 2 && bits < 6 && acc == 0).then_some(out)
}

impl Request {
    /// The answer is a `draw-result/1` document, not the bare output.
    fn envelope(&self) -> bool {
        self.report || self.dry_run || !self.expect.is_empty()
    }
}

/// What a request gave: the text to print and whether the run is good. A bare request is always good (its refusal is an `Err`); a
/// `draw-result/1` document with `ok` false is a verdict, which a CLI reports with exit code 1.
pub struct Reply {
    pub text: String,
    pub ok: bool,
    /// The output was `pathway-check` and it found an error: a CLI answers with its own exit code (3), apart from a refusal (1).
    pub check_failed: bool,
}

/// The output `Editor::save` writes, or the context menu of `context:<x>,<y>` (for the golden parity of the menu).
fn produce(
    e: &mut Editor,
    out: &str,
    flags: &[u32],
    args: Option<&serde_json::Value>,
) -> Result<String, &'static str> {
    // `sites` (with `args` `{"of", "step"}`): where the step applies in the structure of that atom (`sites/1`)
    if out == "sites" {
        let (of, step) = crate::tp::sites_args(args.ok_or("error.draw2.request")?)?;
        return crate::tp::sites(e.doc(), of, &step).map(|s| s.json());
    }
    // `context:<x>,<y>`: the context menu (JSON) of a right-click at that pixel of the default view
    if let Some(at) = out.strip_prefix("context:") {
        let xy = at
            .split_once(',')
            .and_then(|(x, y)| Some((x.trim().parse().ok()?, y.trim().parse().ok()?)));
        let (x, y) = xy.ok_or("error.draw2.request")?;
        return Ok(e.context_menu(x, y));
    }
    // `context-state:<w>,<h>`: what the events left, as the page asks it after them: the flags each returned, the selection, where a
    // menu opened from the keyboard goes in a canvas of that size, the target of the menu there, and whether that menu is still
    // valid right after it was built and after the view moved
    if let Some(size) = out.strip_prefix("context-state:") {
        let wh = size
            .split_once(',')
            .and_then(|(w, h)| Some((w.trim().parse().ok()?, h.trim().parse().ok()?)));
        let (w, h): (f64, f64) = wh.ok_or("error.draw2.request")?;
        let (ax, ay) = e.context_anchor(w, h);
        let menu: serde_json::Value =
            serde_json::from_str(&e.context_menu(ax, ay)).unwrap_or_default();
        let opened = e.context_valid();
        e.pan(1.0, 0.0);
        let o = &e.over;
        return Ok(serde_json::json!({
            "flags": flags, "selected": [o.sel_atoms, o.sel_bonds, o.sel_objs], "anchor": [ax, ay],
            "target": menu["target"], "valid": [opened, e.context_valid()],
        })
        .to_string());
    }
    e.save(out)
}

/// Runs a request `{"import", "import_base64", "style", "set", "log", "out", "report", "dry_run", "expect", "id"}`; the error is an i18n key and the
/// id it is about. Without `report`, `dry_run` and `expect` the answer is the bare output; with one of them it is `draw-result/1`.
pub fn replay(request: &str) -> Result<String, OpError> {
    replay_at(request, 0).map(|r| r.text)
}

/// Like `replay`, for the request number `n` (0-based) of a batch: `n` is the `id` of the document when the request has none, and
/// the reply says whether the run is good.
pub fn replay_at(request: &str, n: u64) -> Result<Reply, OpError> {
    replay_with(request, n, None)
}

/// Like `replay_at`, with the engine that gives masses (`values::Measurer`; the bridge crate passes the one of mzLab). Without it the
/// request field `values` has the formulas and `"engine": false`, and the numbers of `display.values` mass and mz are not drawn.
pub fn replay_with(
    request: &str,
    n: u64,
    engine: Option<crate::values::Measurer>,
) -> Result<Reply, OpError> {
    let fail = |key| OpError { key, id: 0 };
    let r = Request::read(request).map_err(fail)?;
    // the diff of two documents stands alone and answers its own document (not `draw-result/1`)
    if let Some((a, b)) = &r.diff {
        let text = crate::diff::json(a, &crate::diff::diff(a, b));
        return Ok(Reply {
            text,
            ok: true,
            check_failed: false,
        });
    }
    let mut e = Editor::default();
    // the notes of the import, as the page shows them: the start text, then the file
    let mut notes: Vec<(&'static str, usize)> = Vec::new();
    if let Some(text) = &r.import {
        notes.extend(e.import(text).map_err(fail)?.notes());
    }
    if let Some(bytes) = &r.import_base64 {
        notes.extend(e.import_bytes(bytes).map_err(fail)?.notes());
    }
    let imported = crate::outcome::import_notes(notes);
    // a start text or file is an opened drawing: clean, until something is done to it
    e.mark_saved();
    if let Some(style) = &r.style {
        e.set_style(style).map_err(fail)?;
    }
    for setting in &r.set {
        e.set(setting).map_err(fail)?;
    }
    let before = IdSets::of(e.doc());
    let stereo_start = (r
        .expect
        .iter()
        .any(|x| x.check == crate::outcome::Check::StereoKept))
    .then(|| crate::stereo_guard::Snapshot::all(e.doc()))
    .flatten();
    let mut failed = None;
    for (i, op) in r.log.iter().enumerate() {
        if let Err(err) = e.session.run(std::slice::from_ref(op)) {
            if !r.envelope() {
                return Err(err);
            }
            failed = Some(Failed::log(i, err.key, err.id));
            break;
        }
        e.name_op_step(op);
    }
    let orients = r.commands.iter().any(|c| c.0 == "scheme:orient");
    let reports = r
        .commands
        .iter()
        .any(|c| crate::commands::reports_change(&c.0));
    let before_commands = (r.envelope() && reports).then(|| e.doc().to_json());
    // the mass modes of `pathway:label` take their numbers from the engine
    if engine.is_some() && r.commands.iter().any(|c| c.0 == "pathway:label") {
        e.measurer = engine;
    }
    if failed.is_none() {
        for (i, (name, args)) in r.commands.iter().enumerate() {
            if let Err(err) = crate::commands::run(&mut e, name, args) {
                if !r.envelope() {
                    return Err(err);
                }
                let mut f = Failed::command(i, err.key, err.id);
                f.rows = e.failed_rows.clone();
                failed = Some(f);
                break;
            }
            e.name_steps();
        }
    }
    if let (None, Some(ops)) = (&failed, &r.propose) {
        let mut run = e
            .propose(ops.clone())
            .map(|_| ())
            .map_err(|err| (None, err));
        if run.is_ok() {
            // all or none: the list is checked on a copy before the first hunk is accepted
            run = e.accept_hunks(&r.accept).map_err(|(i, err)| (Some(i), err));
        }
        if let Err((i, err)) = run {
            if !r.envelope() {
                return Err(err);
            }
            failed = Some(Failed::proposal(i, err.key, err.id));
        }
    }
    if let Some((w, h)) = r.canvas {
        e.set_canvas(w, h);
    }
    if let Some(text) = &r.paste {
        e.paste(Some(text));
        if !e.error.is_empty() {
            return Err(fail(e.error));
        }
    }
    let flags: Vec<u32> = (r.events.iter())
        .map(|ev| match ev {
            Event::Pointer(n) => {
                let [phase, id, x, y, kind, mods, t] = *n;
                e.pointer_event(phase as u32, id as u32, x, y, kind as u32, mods as u32, t)
            }
            Event::Key(key, mods, t) => e.key_at(key, *mods, *t),
        })
        .collect();
    let mut flags = flags;
    for k in &r.keys {
        // an id that is not in the drawing (as the keys before it left it) is a bad entry, never a silent fall-through to the tool keys
        let d = e.doc();
        let known = match k.hover {
            Some((true, id)) => d.atom(id).is_some(),
            Some((false, id)) => d.bond(id).is_some(),
            None => true,
        } && k.focus.is_none_or(|(a, from)| {
            d.atom(a).is_some() && from.is_none_or(|f| d.atom(f).is_some())
        });
        if !known {
            return Err(fail("error.draw2.keys"));
        }
        flags.push(e.key_over(&k.key, k.mods, k.hover, k.focus));
    }
    if let Some((kind, with_log, agent, lang)) = &r.feedback {
        let log = e.session.log_json();
        let params = crate::feedback::Params {
            kind,
            with_log: *with_log,
            log: &log,
            agent,
            lang,
        };
        let text = crate::feedback::build(&params).map_err(fail)?.json();
        return Ok(Reply {
            text,
            ok: true,
            check_failed: false,
        });
    }
    if let Some(select) = &r.values {
        let text =
            crate::values::document_json(e.doc(), &e.style.adducts, select.as_deref(), engine);
        return Ok(Reply {
            text,
            ok: true,
            check_failed: false,
        });
    }
    if let Some((select, form, keys)) = &r.props {
        let text = crate::values::document_props(
            e.doc(),
            &e.style.adducts,
            select.as_deref(),
            engine,
            form.as_deref().map(|f| (f, keys.as_slice())),
        )
        .map_err(fail)?;
        return Ok(Reply {
            text,
            ok: true,
            check_failed: false,
        });
    }
    // the numbers an export draws under the structures (`display.values` mass and mz, or a molecule's chosen lines)
    if engine.is_some()
        && (matches!(
            e.style.values,
            crate::values::Mode::Mass | crate::values::Mode::Mz
        ) || e
            .doc()
            .atoms
            .iter()
            .any(|a| a.cap().iter().any(|k| crate::props::needs_engine(k))))
    {
        e.measurer = engine;
        e.refresh_values();
    }
    // the TP table and the pathway check take their masses from the engine too
    if engine.is_some()
        && r.out
            .as_deref()
            .is_some_and(|o| o.starts_with("tptable") || o == "pathway-check")
    {
        e.measurer = engine;
    }
    if let Some(a) = &r.out_args
        && r.out.as_deref() == Some("pathway-check")
    {
        e.check_opts = crate::pathcheck::Opts::parse(Some(a)).map_err(fail)?;
    }
    let kind = r.out.as_deref().unwrap_or(crate::brand::SLUG);
    let written = produce(&mut e, kind, &flags, r.out_args.as_ref());
    // a refused transformation names its first atom, as a refused operation does
    let fail_out = |key| OpError {
        key,
        id: if kind == "trasf" {
            (e.trasf_problem())
                .and_then(|p| p.1.first().copied())
                .unwrap_or(0)
        } else {
            0
        },
    };
    if !r.envelope() {
        return written.map_err(fail_out).map(|text| Reply {
            text,
            ok: true,
            check_failed: e.check_failed.get(),
        });
    }
    // an `out` the request cannot mean is an invalid request whatever the verdict; a refusal of the drawing (no arrow, no SMILES) only counts when it is written
    if matches!(written, Err("error.draw2.request")) {
        return Err(fail("error.draw2.request"));
    }
    let after = IdSets::of(e.doc());
    let expect = (!r.expect.is_empty()).then(|| {
        (r.expect.iter())
            .map(|x| match &failed {
                Some(_) => (
                    x.request.clone(),
                    false,
                    "not checked: the log failed".to_string(),
                ),
                None => {
                    let (ok, detail) = x.check.run(e.doc(), stereo_start.as_ref(), &e.style);
                    (x.request.clone(), ok, detail)
                }
            })
            .collect()
    });
    let mut outcome = Outcome {
        id: r
            .id
            .as_ref()
            .map_or_else(|| n.to_string(), ToString::to_string),
        dry_run: r.dry_run,
        out: None,
        created: after.minus(&before),
        removed: before.minus(&after),
        imported,
        lost: Vec::new(),
        failed,
        changed: before_commands.map(|b| b != e.doc().to_json()),
        dirty: e.dirty(),
        orient: orients.then(|| e.orient_steps.clone()),
        grid: r
            .commands
            .iter()
            .any(|c| c.0 == "grid")
            .then(|| e.grid_left.clone()),
        expect,
    };
    if outcome.ok() {
        outcome.lost = e.lost_on_save(r.out.as_deref().unwrap_or(crate::brand::SLUG));
        let text = written.map_err(fail_out)?;
        outcome.out = (!r.dry_run).then_some(text);
    }
    Ok(Reply {
        ok: outcome.ok(),
        text: outcome.json(),
        check_failed: e.check_failed.get(),
    })
}
