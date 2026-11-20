//! The headless door: a request with a starting document or text, a command log and the output wanted gives the output text. The
//! `.wasm`, the app and the CLI call this one function, so a session of the editor replays to the same bytes anywhere.

use crate::editor::Editor;
use crate::ops::{Op, OpError};

/// The request, read by hand from its JSON value (as the operations and the document are).
struct Request {
    /// A structure text to start from: `mzdraw/1` JSON, a molfile or a SMILES.
    import: Option<String>,
    log: Vec<Op>,
    /// `mzdraw` (default), `molfile`, `svg` or `log`.
    out: Option<String>,
    /// The style of the SVG: a preset name or the text of a settings file; default `mzlab`.
    style: Option<String>,
}

impl Request {
    fn read(text: &str) -> Option<Request> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        let text_of = |k: &str| match v.get(k) {
            None | Some(serde_json::Value::Null) => Some(None),
            Some(x) => x.as_str().map(|s| Some(s.to_string())),
        };
        let log = match v.get("log") {
            None => Vec::new(),
            Some(l) => l
                .as_array()?
                .iter()
                .map(Op::from_value)
                .collect::<Option<Vec<Op>>>()?,
        };
        v.is_object().then_some(())?;
        Some(Request {
            import: text_of("import")?,
            log,
            out: text_of("out")?,
            style: match v.get("style") {
                // a settings object may be given in place: it is read as its own text
                Some(s) if s.is_object() => Some(s.to_string()),
                _ => text_of("style")?,
            },
        })
    }
}

/// Runs a request `{"import", "log", "out"}`; the error is an i18n key and the id it is about.
pub fn replay(request: &str) -> Result<String, OpError> {
    let fail = |key| OpError { key, id: 0 };
    let r = Request::read(request).ok_or_else(|| fail("error.draw2.request"))?;
    let mut e = Editor::default();
    if let Some(text) = &r.import {
        e.import(text).map_err(fail)?;
    }
    if let Some(style) = &r.style {
        e.set_style(style).map_err(fail)?;
    }
    for op in &r.log {
        e.session.run(std::slice::from_ref(op))?;
    }
    Ok(match r.out.as_deref().unwrap_or("mzdraw") {
        "mzdraw" => e.doc().to_json(),
        "molfile" => e.doc().to_molfile(),
        "svg" => e.svg(),
        "log" => e.session.log_json(),
        _ => return Err(fail("error.draw2.request")),
    })
}
