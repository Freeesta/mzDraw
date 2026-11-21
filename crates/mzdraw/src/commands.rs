//! The editor commands of a request (`"commands":[{"command":NAME,"args":{...}}]`): the same commands the page runs, so a raw-op
//! agent gets the follow of anchored arrows (`route`) that a bare operation of the log does not give. The commands of this slice
//! are `move`, `rotate` and `flip`; their `args` are the fields of the operation of the same name, and a `rotate` without `x`,
//! `y` (or a `flip` without `at`) works about the centre of the bounding box of its atoms, as the buttons of the page do.

use crate::editor::Editor;
use crate::ops::{Op, OpError};
use crate::orient::Status;
use serde_json::{json, Value};

/// Whether `name` is a command of this editor.
pub(crate) fn known(name: &str) -> bool {
    matches!(name, "move" | "rotate" | "flip" | "scheme:orient")
}

/// Checks the shape of a command before any command runs: a known name, `args` an object whose fields are the fields of the
/// operation of that name, the required ones present (`x`, `y` of a rotate and `at` of a flip may be left out). The error is an
/// invalid request, never a verdict about a document.
pub(crate) fn check(name: &str, args: &Value) -> Result<(), &'static str> {
    if !known(name) {
        return Err("error.draw2.command");
    }
    if name == "scheme:orient" {
        // no argument: it orients every structure after an arrow on the one before it
        return match args.as_object() {
            Some(o) if o.is_empty() => Ok(()),
            _ => Err("error.draw2.request"),
        };
    }
    let mut v = args.clone();
    let obj = v.as_object_mut().ok_or("error.draw2.request")?;
    obj.insert("op".into(), json!(name));
    for (k, on) in [
        ("x", name == "rotate"),
        ("y", name == "rotate"),
        ("at", name == "flip"),
    ] {
        if on {
            obj.entry(k).or_insert(json!(0.0));
        }
    }
    Op::parse(&v).map(|_| ())
}

/// Runs one command as one undo step, with the route operations of the arrows that follow. The error is an i18n key and the id it
/// is about.
pub(crate) fn run(e: &mut Editor, name: &str, args: &Value) -> Result<(), OpError> {
    let bad = |key| OpError { key, id: 0 };
    if !known(name) {
        return Err(bad("error.draw2.command"));
    }
    if name == "scheme:orient" {
        return orient(e);
    }
    let mut v = args.clone();
    let obj = v.as_object_mut().ok_or(bad("error.draw2.request"))?;
    obj.insert("op".into(), json!(name));
    let atoms: Vec<u32> = (obj.get("atoms").and_then(Value::as_array))
        .map(|l| {
            l.iter()
                .filter_map(Value::as_u64)
                .map(|i| i as u32)
                .collect()
        })
        .unwrap_or_default();
    // the centre of the atoms, when the arguments leave it out
    if !atoms.is_empty() && atoms.iter().all(|&i| e.doc().atom(i).is_some()) {
        let (cx, cy) = e.centre(&atoms);
        match name {
            "rotate" => {
                obj.entry("x").or_insert(json!(cx));
                obj.entry("y").or_insert(json!(cy));
            }
            "flip" => {
                let at = if obj.get("axis").and_then(Value::as_str) == Some("h") {
                    cx
                } else {
                    cy
                };
                obj.entry("at").or_insert(json!(at));
            }
            _ => {}
        }
    }
    let op = Op::parse(&v).map_err(bad)?;
    let ops = e.following(&[op]);
    e.session.run(&ops)
}

/// `scheme:orient`: one undo step with the turns of `orient::plan` (and the route of the arrows that follow). Refused, and nothing
/// changed, when a pair would overlap itself, when there is no pair with a core, or when every pair is oriented already.
fn orient(e: &mut Editor) -> Result<(), OpError> {
    let plan = crate::orient::plan(e.doc());
    if let Some(s) = (plan.steps.iter()).find(|s| s.status == Status::Overlap) {
        return Err(OpError {
            key: s.status.key(),
            id: s.at,
        });
    }
    if plan.ops.is_empty() {
        // the reason of the first pair that has one; no pair at all is no core
        let s = (plan.steps.iter()).find(|s| s.status == Status::Already);
        let key = s.map_or(Status::NoCore.key(), |s| s.status.key());
        return Err(OpError { key, id: 0 });
    }
    let n = (plan.steps.iter())
        .filter(|s| s.status == Status::Oriented)
        .count();
    let ops = e.following(&plan.ops);
    e.session.run(&ops)?;
    e.said = ("draw2.say.oriented", n.to_string());
    Ok(())
}
