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
}

/// One event of the request field `events`.
enum Event {
    Pointer([f64; 7]),
    Key(String, u32, f64),
}

/// The fields of a request. A field outside this list is refused (`error.draw2.requestField`); `engines` belongs to the glue door.
const FIELDS: [&str; 17] = [
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
        Ok(Request {
            diff: None,
            propose,
            accept,
            events,
            commands,
            report: flag("report")?,
            dry_run: flag("dry_run")?,
            expect: v.get("expect").map_or(Ok(Vec::new()), read_expect)?,
            id,
            import: text_of("import")?,
            import_base64,
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
    let fail = |key| OpError { key, id: 0 };
    let r = Request::read(request).map_err(fail)?;
    // the diff of two documents stands alone and answers its own document (not `draw-result/1`)
    if let Some((a, b)) = &r.diff {
        let text = crate::diff::json(a, &crate::diff::diff(a, b));
        return Ok(Reply { text, ok: true });
    }
    let mut e = Editor::default();
    if let Some(text) = &r.import {
        e.import(text).map_err(fail)?;
    }
    if let Some(bytes) = &r.import_base64 {
        e.import_bytes(bytes).map_err(fail)?;
    }
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
    }
    let orients = r.commands.iter().any(|c| c.0 == "scheme:orient");
    let reports = r
        .commands
        .iter()
        .any(|c| crate::commands::reports_change(&c.0));
    let before_commands = (r.envelope() && reports).then(|| e.doc().to_json());
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
    let flags: Vec<u32> = (r.events.iter())
        .map(|ev| match ev {
            Event::Pointer(n) => {
                let [phase, id, x, y, kind, mods, t] = *n;
                e.pointer_event(phase as u32, id as u32, x, y, kind as u32, mods as u32, t)
            }
            Event::Key(key, mods, t) => e.key_at(key, *mods, *t),
        })
        .collect();
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
        return written
            .map_err(fail_out)
            .map(|text| Reply { text, ok: true });
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
        lost: Vec::new(),
        failed,
        changed: before_commands.map(|b| b != e.doc().to_json()),
        orient: orients.then(|| e.orient_steps.clone()),
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
    })
}
