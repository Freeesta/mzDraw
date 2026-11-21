//! The glue between the drawing and the engines of mzLab (`mzlab_feat::chem`). It builds the molfile with the atoms in document order once,
//! calls the engine by name and gives the answer back unchanged; `ids` maps the atom numbers of an answer back to the ids of the document.
//! Native code (the example `replay_glue`) and the `.wasm` call these same functions, so the golden parity covers the engines.
//! The engines are called only here, never from `lib.rs`, and never re-implemented.

use mzdraw::Document;

/// A refusal: an i18n key and its parameters (`error.draw2.engineMissing` for an engine this build does not carry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlueError {
    pub key: String,
    pub params: Vec<(String, String)>,
}

impl GlueError {
    fn new(key: &str) -> Self {
        GlueError {
            key: key.to_string(),
            params: Vec::new(),
        }
    }

    /// `{"error_key", "params": {..}}`, the shape of every refusal of the bridge.
    pub fn to_json(&self) -> String {
        let params: serde_json::Map<String, serde_json::Value> = (self.params.iter())
            .map(|(k, v)| (k.clone(), serde_json::Value::from(v.clone())))
            .collect();
        serde_json::json!({"error_key": self.key, "params": params}).to_string()
    }
}

/// The engines of this build, in the order the page lists them.
pub const ENGINES: &[&str] = &["check", "cip", "cut", "mech", "hilite", "sel"];

/// Document id of each atom number of an engine answer: engine atom `i` (0-based, molfile order, which is the pool order of the
/// document) is `ids(doc)[i]`. After deletions the ids are not `0..n`, so an answer must never be read as an id directly.
pub fn ids(doc: &Document) -> Vec<u32> {
    doc.atoms.iter().map(|a| a.id).collect()
}

/// The molfile every engine receives (the text `check/1` always used).
pub fn molfile(doc: &Document) -> String {
    doc.to_molfile_for_check()
}

/// The request of an engine: the object `args` (empty text is `{}`) with `molfile` set to the drawing.
fn request(doc: &Document, args: &str) -> Result<String, GlueError> {
    let bad = || GlueError::new("error.draw2.request");
    let mut v: serde_json::Value = if args.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(args).map_err(|_| bad())?
    };
    v.as_object_mut()
        .ok_or_else(bad)?
        .insert("molfile".to_string(), molfile(doc).into());
    Ok(v.to_string())
}

/// `check/1` of the drawing, as the page shows it, or `None` for an empty drawing or one the check refuses.
pub fn check(doc: &Document) -> Option<String> {
    if doc.atoms.is_empty() {
        return None;
    }
    let molfile = serde_json::Value::from(molfile(doc));
    mzlab_feat::chem::check::check_json(&format!(r#"{{"molfile":{molfile}}}"#)).ok()
}

/// Runs the engine `engine` (one of `ENGINES`) on the drawing; `args` is a JSON object with the engine's own fields, without `molfile`.
/// The answer is the engine's JSON text, byte for byte what `mzlab-cli mol <engine>` writes for the same molfile.
pub fn run(engine: &str, doc: &Document, args: &str) -> Result<String, GlueError> {
    use mzlab_feat::chem;
    if !ENGINES.contains(&engine) {
        return Err(GlueError::new("error.draw2.engineMissing"));
    }
    let missing = || GlueError::new("error.draw2.engineMissing");
    let r = request(doc, args)?;
    let out = match engine {
        "check" => chem::check::check_json(&r),
        "cip" => chem::cip::cip_json(&r),
        "cut" => chem::cut::cut_json(&r),
        "mech" => chem::mech::mech_json(&r),
        #[cfg(feature = "hilite")]
        "hilite" => chem::hilite::hilite_json(&r),
        #[cfg(feature = "sel")]
        "sel" => chem::sel::sel_json(&r),
        _ => return Err(missing()),
    };
    out.map_err(|e| GlueError {
        key: e.key.to_string(),
        params: (e.params.iter())
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    })
}

/// The headless door with engines: a replay request (see `mzdraw::replay`) plus `"engines": [{"engine", "args"?}]`. Gives
/// `{"engines": [{"engine", "answer"} | {"engine", "error": {"error_key", "params"}}]}` in request order, where `answer` is the engine's JSON.
pub fn replay(request: &str) -> Result<String, GlueError> {
    let bad = || GlueError::new("error.draw2.request");
    let mut v: serde_json::Value = serde_json::from_str(request).map_err(|_| bad())?;
    let obj = v.as_object_mut().ok_or_else(bad)?;
    let wanted = obj
        .remove("engines")
        .unwrap_or_else(|| serde_json::json!([]));
    obj.insert("out".to_string(), "mzdraw".into());
    let doc_json = mzdraw::replay::replay(&v.to_string()).map_err(|e| GlueError {
        key: e.key.to_string(),
        params: vec![("id".to_string(), e.id.to_string())],
    })?;
    let doc = Document::from_json(&doc_json).map_err(|_| bad())?;
    let mut out = Vec::new();
    for w in wanted.as_array().ok_or_else(bad)? {
        let engine = w["engine"].as_str().ok_or_else(bad)?;
        let args = match w.get("args") {
            None | Some(serde_json::Value::Null) => String::new(),
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(a) => a.to_string(),
        };
        let entry = match run(engine, &doc, &args) {
            Ok(a) => format!(
                r#"{{"engine":{},"answer":{a}}}"#,
                serde_json::Value::from(engine)
            ),
            Err(e) if e.key == "error.draw2.engineMissing" => return Err(e),
            Err(e) => format!(
                r#"{{"engine":{},"error":{}}}"#,
                serde_json::Value::from(engine),
                e.to_json()
            ),
        };
        out.push(entry);
    }
    Ok(format!(r#"{{"engines":[{}]}}"#, out.join(",")))
}
