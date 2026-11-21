//! The editor commands of a request (`"commands":[{"command":NAME,"args":{...}}]`): the same commands the page runs, so a raw-op
//! agent gets the follow of anchored arrows (`route`) that a bare operation of the log does not give. The commands of this slice
//! are `move`, `rotate`, `flip` and (tp-02) `pathway:anchor` (`{"arrow": id}`) and `pathway:route`; their `args` are the fields of the operation of the same name, and a `rotate` without `x`,
//! `y` (or a `flip` without `at`) works about the centre of the bounding box of its atoms, as the buttons of the page do.

use crate::editor::Editor;
use crate::ops::{Op, OpError};
use serde_json::{json, Value};

/// Whether `name` is a command of this editor.
pub(crate) fn known(name: &str) -> bool {
    matches!(
        name,
        "move" | "rotate" | "flip" | "pathway:anchor" | "pathway:route"
    )
}

/// Checks the shape of a command before any command runs: a known name, `args` an object whose fields are the fields of the
/// operation of that name, the required ones present (`x`, `y` of a rotate and `at` of a flip may be left out). The error is an
/// invalid request, never a verdict about a document.
pub(crate) fn check(name: &str, args: &Value) -> Result<(), &'static str> {
    if !known(name) {
        return Err("error.draw2.command");
    }
    if name.starts_with("pathway:") {
        return pathway_args(name, args).map(|_| ());
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

/// The arrow of `pathway:anchor` (`args` is `{"arrow": id}`); `pathway:route` takes no argument. Anything else is an invalid request.
fn pathway_args(name: &str, args: &Value) -> Result<Option<u32>, &'static str> {
    let obj = args.as_object().ok_or("error.draw2.request")?;
    if name == "pathway:route" {
        return if obj.is_empty() {
            Ok(None)
        } else {
            Err("error.draw2.request")
        };
    }
    let id = obj.get("arrow").and_then(Value::as_u64);
    match id {
        Some(i) if obj.len() == 1 && i > 0 && i <= u64::from(u32::MAX) => Ok(Some(i as u32)),
        _ => Err("error.draw2.request"),
    }
}

/// Runs one command as one undo step, with the route operations of the arrows that follow. The error is an i18n key and the id it
/// is about.
pub(crate) fn run(e: &mut Editor, name: &str, args: &Value) -> Result<(), OpError> {
    let bad = |key| OpError { key, id: 0 };
    if !known(name) {
        return Err(bad("error.draw2.command"));
    }
    if name.starts_with("pathway:") {
        let id = pathway_args(name, args).map_err(bad)?;
        return match id {
            Some(arrow) => e.pathway_anchor(arrow),
            None => e.pathway_route(),
        };
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
