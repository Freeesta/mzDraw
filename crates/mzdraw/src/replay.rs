//! The headless door: a request with a starting document or text, a command log and the output wanted gives the output text. The
//! `.wasm`, the app and the CLI call this one function, so a session of the editor replays to the same bytes anywhere.

use crate::editor::Editor;
use crate::ops::{Op, OpError};
use crate::outcome::{read_expect, Expectation, Failed, IdSets, Outcome};

/// The request, read by hand from its JSON value (as the operations and the document are).
struct Request {
    /// A structure text to start from: `mzdraw/1` JSON, a molfile or a SMILES.
    import: Option<String>,
    log: Vec<Op>,
    /// What `Editor::save` writes: `mzdraw` (default), `molfile`, `molfile-v3000`, `smiles`, `ket`, `cdxml`, `cml`, `rxn`, `sdf`, `svg`, `pdf` (the PDF file as base64; the CLI `--raw` writes the file), `log`, `std` (the drawing checks `std/1`), `schema` (the table of the operations, `draw-edit-schema/1`) or `settings` (the settings file with the style, `layout` and `export`: what `set` left).
    out: Option<String>,
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
}

/// The fields of a request. A field outside this list is refused (`error.draw2.requestField`); `engines` belongs to the glue door.
const FIELDS: [&str; 12] = [
    "import", "style", "set", "log", "commands", "command", "args", "out", "report", "dry_run",
    "expect", "id",
];

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
        if v.get("args").is_some() && v.get("command").is_none() {
            return Err(BAD);
        }
        let flag = |k: &str| match v.get(k) {
            None => Ok(false),
            Some(x) => x.as_bool().ok_or(BAD),
        };
        let id = match v.get("id") {
            None => None,
            Some(x) if x.is_string() || x.is_i64() || x.is_u64() => Some(x.clone()),
            Some(_) => return Err(BAD),
        };
        Ok(Request {
            commands,
            report: flag("report")?,
            dry_run: flag("dry_run")?,
            expect: v.get("expect").map_or(Ok(Vec::new()), read_expect)?,
            id,
            import: text_of("import")?,
            log,
            out: text_of("out")?,
            set,
            style: match v.get("style") {
                // a settings object may be given in place: it is read as its own text
                Some(s) if s.is_object() => Some(s.to_string()),
                _ => text_of("style")?,
            },
        })
    }
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
}

/// The output `Editor::save` writes, or the context menu of `context:<x>,<y>` (for the golden parity of the menu).
fn produce(e: &mut Editor, out: &str) -> Result<String, &'static str> {
    // `context:<x>,<y>`: the context menu (JSON) of a right-click at that pixel of the default view
    if let Some(at) = out.strip_prefix("context:") {
        let xy = at
            .split_once(',')
            .and_then(|(x, y)| Some((x.trim().parse().ok()?, y.trim().parse().ok()?)));
        let (x, y) = xy.ok_or("error.draw2.request")?;
        return Ok(e.context_menu(x, y));
    }
    e.save(out)
}

/// Runs a request `{"import", "style", "set", "log", "out", "report", "dry_run", "expect", "id"}`; the error is an i18n key and the
/// id it is about. Without `report`, `dry_run` and `expect` the answer is the bare output; with one of them it is `draw-result/1`.
pub fn replay(request: &str) -> Result<String, OpError> {
    replay_at(request, 0).map(|r| r.text)
}

/// Like `replay`, for the request number `n` (0-based) of a batch: `n` is the `id` of the document when the request has none, and
/// the reply says whether the run is good.
pub fn replay_at(request: &str, n: u64) -> Result<Reply, OpError> {
    let fail = |key| OpError { key, id: 0 };
    let r = Request::read(request).map_err(fail)?;
    let mut e = Editor::default();
    if let Some(text) = &r.import {
        e.import(text).map_err(fail)?;
    }
    if let Some(style) = &r.style {
        e.set_style(style).map_err(fail)?;
    }
    for setting in &r.set {
        e.set(setting).map_err(fail)?;
    }
    let before = IdSets::of(e.doc());
    let mut failed = None;
    for (i, op) in r.log.iter().enumerate() {
        if let Err(err) = e.session.run(std::slice::from_ref(op)) {
            if !r.envelope() {
                return Err(err);
            }
            failed = Some(Failed::log(i, err.key, err.id));
            break;
        }
    }
    if failed.is_none() {
        for (i, (name, args)) in r.commands.iter().enumerate() {
            if let Err(err) = crate::commands::run(&mut e, name, args) {
                if !r.envelope() {
                    return Err(err);
                }
                failed = Some(Failed::command(i, err.key, err.id));
                break;
            }
        }
    }
    let kind = r.out.as_deref().unwrap_or("mzdraw");
    let written = produce(&mut e, kind);
    if !r.envelope() {
        return written.map_err(fail).map(|text| Reply { text, ok: true });
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
                    let (ok, detail) = x.check.run(e.doc());
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
        failed,
        expect,
    };
    if outcome.ok() {
        let text = written.map_err(fail)?;
        outcome.out = (!r.dry_run).then_some(text);
    }
    Ok(Reply {
        ok: outcome.ok(),
        text: outcome.json(),
    })
}
