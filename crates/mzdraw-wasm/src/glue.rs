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

/// The engine behind `display.values` and the request field `values` (`mzdraw::values::Measurer`): `sel` on the molfile of one
/// structure gives the monoisotopic mass (`label.mass`), the average weight and, for a charged structure, its own m/z; each adduct's m/z is
/// that mass plus the adduct shift of `mzlab_feat::chem::sdf::adduct_shift` (a charged structure has no adduct: `None`). No number is
/// computed here beyond that one sum; no mass table is in this repository.
#[cfg(feature = "sel")]
pub fn measure(
    molfile: &str,
    adducts: &[String],
) -> Result<mzdraw::values::Measured, &'static str> {
    use mzlab_feat::chem;
    let request = serde_json::json!({ "molfile": molfile }).to_string();
    let answer = chem::sel::sel_json(&request).map_err(|e| e.key)?;
    let v: serde_json::Value =
        serde_json::from_str(&answer).map_err(|_| "error.draw2.valuesEngine")?;
    let label = &v["label"];
    let mass = label["mass"].as_f64().ok_or("error.draw2.valuesEngine")?;
    // a mass of zero is no measure: `sel` reads the V2000 molfile, and a structure of 1000 atoms or more is written as V3000, which it
    // reads as nothing (mass 0.0000, no error). Said as a key, so that no number is written from it.
    if mass == 0.0 {
        return Err("error.draw2.valuesZero");
    }
    let charge = label["charge"].as_i64().unwrap_or(0);
    let mz = adducts
        .iter()
        .map(|name| match chem::sdf::adduct_shift(name) {
            _ if charge != 0 => Ok(None),
            Some(shift) => Ok(Some(mass + shift)),
            None => Err("error.draw2.valuesAdduct"),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(mzdraw::values::Measured {
        mass,
        average: v["average"].as_f64(),
        ion_mz: label["mz"].as_f64(),
        mz,
        pct: composition(molfile),
    })
}

/// The elemental analysis of a structure: the mass per cent of each element in Hill order, from `analysis/1` of mzLab
/// (`mzlab_feat::chem::analysis`) on the atoms the engine reads from the molfile (open valences filled with hydrogen, as `sel` does).
/// Empty when the engine does not give it (an element with no weight): the other numbers stand.
#[cfg(feature = "sel")]
fn composition(molfile: &str) -> Vec<(String, f64)> {
    use mzlab_feat::chem;
    let Ok(g) = chem::graph::Mol::from_molfile(molfile) else {
        return Vec::new();
    };
    let atoms: Vec<serde_json::Value> = (g.atoms.iter())
        .map(|a| serde_json::json!({"el": a.el, "iso": a.iso, "h": a.h, "q": a.charge}))
        .collect();
    let request = serde_json::json!({ "atoms": atoms }).to_string();
    let Ok(answer) = chem::analysis::analysis_json(&request) else {
        return Vec::new();
    };
    let v: serde_json::Value = serde_json::from_str(&answer).unwrap_or_default();
    (v["composition"].as_array().into_iter().flatten())
        .filter_map(|c| Some((c["el"].as_str()?.to_string(), c["pct"].as_f64()?)))
        .collect()
}

/// The headless door with engines: a replay request (see `mzdraw::replay`) plus `"engines": [{"engine", "args"?}]`. Gives
/// `{"engines": [{"engine", "answer"} | {"engine", "error": {"error_key", "params"}}]}` in request order, where `answer` is the engine's JSON.
pub fn replay(request: &str) -> Result<String, GlueError> {
    let bad = || GlueError::new("error.draw2.request");
    let mut v: serde_json::Value = serde_json::from_str(request).map_err(|_| bad())?;
    let obj = v.as_object_mut().ok_or_else(bad)?;
    // no `engines`: the plain replay with the engine of `display.values` attached, so the request field `values` and an export with
    // the masses under the structures give numbers (the bare output, as `mzdraw::replay::replay`)
    #[cfg(feature = "sel")]
    if !obj.contains_key("engines") {
        return mzdraw::replay::replay_with(request, 0, Some(measure))
            .map(|r| r.text)
            .map_err(|e| GlueError {
                key: e.key.to_string(),
                params: vec![("id".to_string(), e.id.to_string())],
            });
    }
    let wanted = obj
        .remove("engines")
        .unwrap_or_else(|| serde_json::json!([]));
    obj.insert("out".to_string(), mzdraw::brand::SLUG.into());
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
