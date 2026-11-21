//! The editor commands of a request (`"commands":[{"command":NAME,"args":{...}}]`): the same commands the page runs, so a raw-op
//! agent gets the follow of anchored arrows (`route`) that a bare operation of the log does not give. The commands of this slice
//! are `move`, `rotate`, `flip` and `scheme:row` (plan-13: the one-line scheme layout, `gap`, `wrap`, `atoms`, `objects`); their `args` are the fields of the operation of the same name, and a `rotate` without `x`,
//! `y` (or a `flip` without `at`) works about the centre of the bounding box of its atoms, as the buttons of the page do.

use crate::editor::Editor;
use crate::ops::{Op, OpError};
use crate::orient::Status;
use serde_json::{json, Value};

/// Whether `name` is a command of this editor.
pub(crate) fn known(name: &str) -> bool {
    matches!(
        name,
        "move" | "rotate" | "flip" | "scheme:row" | "scheme:orient"
    )
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
    if name == "scheme:row" {
        return crate::scheme::Opts::parse(args).map(|_| ());
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
    if name == "scheme:row" {
        let opts = crate::scheme::Opts::parse(args).map_err(bad)?;
        let ops = crate::scheme::row(e.doc(), &e.style, &opts).map_err(bad)?;
        let ops = e.following(&ops);
        return e.session.run(&ops);
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

/// `scheme:orient`: one undo step with the turns of `orient::plan` (and the route of the arrows that follow). Nothing is changed, and
/// the command is refused, when a structure would overlap (`orientOverlap`), when none has a core (`orientNone`) or is too large
/// (`orientSize`). A scheme already oriented is not a refusal: the command changes nothing and says so (`changed` false).
/// The steps of the last run are kept in `e.orient_steps` for the result document.
fn orient(e: &mut Editor) -> Result<(), OpError> {
    let plan = crate::orient::plan(e.doc());
    e.orient_steps = plan.steps.clone();
    if let Some(s) = (plan.steps.iter()).find(|s| s.status == Status::Overlap) {
        return Err(OpError {
            key: s.status.key(),
            id: s.at,
        });
    }
    if plan.ops.is_empty() {
        let done = (plan.steps.iter()).any(|s| s.status == Status::Already);
        if !done {
            let key = (plan.steps.iter())
                .find(|s| s.status == Status::TooLarge)
                .map_or(Status::NoCore.key(), |s| s.status.key());
            return Err(OpError { key, id: 0 });
        }
        e.said = ("draw2.say.orientDone", String::new());
        return Ok(());
    }
    let n = (plan.steps.iter())
        .filter(|s| s.status == Status::Oriented)
        .count();
    let ops = e.following(&plan.ops);
    e.session.run(&ops)?;
    e.said = ("draw2.say.oriented", n.to_string());
    Ok(())
}
