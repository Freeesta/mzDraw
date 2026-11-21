//! The editor commands of a request (`"commands":[{"command":NAME,"args":{...}}]`): the same commands the page runs, so a raw-op
//! agent gets the follow of anchored arrows (`route`) that a bare operation of the log does not give. The commands of this slice
//! are `move`, `rotate`, `flip`, `scheme:row` (plan-13: the one-line scheme layout, `gap`, `wrap`, `atoms`, `objects`) and (tp-02) `pathway:anchor` (`{"arrow": id}`) and `pathway:route`; their `args` are the fields of the operation of the same name, and a `rotate` without `x`,
//! `y` (or a `flip` without `at`) works about the centre of the bounding box of its atoms, as the buttons of the page do.

use crate::editor::Editor;
use crate::ops::{Op, OpError};
use crate::orient::Status;
use serde_json::{json, Value};

/// Whether a command tells, in the result document, whether it changed the drawing (`changed`): `scheme:orient`, `pathway:anchor` and `pathway:route` do.
pub(crate) fn reports_change(name: &str) -> bool {
    matches!(name, "scheme:orient" | "pathway:anchor" | "pathway:route")
}

/// Whether `name` is a command of this editor.
pub(crate) fn known(name: &str) -> bool {
    matches!(
        name,
        "move"
            | "rotate"
            | "flip"
            | "scheme:row"
            | "pathway:anchor"
            | "pathway:route"
            | "pathway:tree"
            | "scheme:orient"
            | "tp:derive"
    )
}

/// Checks the shape of a command before any command runs: a known name, `args` an object whose fields are the fields of the
/// operation of that name, the required ones present (`x`, `y` of a rotate and `at` of a flip may be left out). The error is an
/// invalid request, never a verdict about a document.
pub(crate) fn check(name: &str, args: &Value) -> Result<(), &'static str> {
    if !known(name) {
        return Err("error.draw2.command");
    }
    if name == "pathway:anchor" || name == "pathway:route" {
        return pathway_args(name, args).map(|_| ());
    }
    if name == "tp:derive" {
        return derive_args(args).map(|_| ());
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
    if name == "pathway:tree" {
        return (args.as_object().is_some_and(|o| o.is_empty()))
            .then_some(())
            .ok_or("error.draw2.request");
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

/// The arguments of `tp:derive`: `{"of": atom id, "patch": [operations on the parent's ids], "at": [x, y]}` (`patch` and `at` may be
/// left out). In a patch a negative id names a new atom of the patch: `-1` the first `atom.add`, `-2` the second, and so on.
type Derive = (u32, Vec<Op>, Option<(f64, f64)>);

fn derive_args(args: &Value) -> Result<Derive, &'static str> {
    const BAD: &str = "error.draw2.request";
    let obj = args.as_object().ok_or(BAD)?;
    if !obj
        .keys()
        .all(|k| matches!(k.as_str(), "of" | "patch" | "at"))
    {
        return Err(BAD);
    }
    let of = match obj.get("of").and_then(Value::as_u64) {
        Some(i) if i > 0 && i < u64::from(crate::doc::MAX_ID) => i as u32,
        _ => return Err(BAD),
    };
    let at = match obj.get("at") {
        None => None,
        Some(v) => match v.as_array().map(Vec::as_slice) {
            Some([x, y]) => match (x.as_f64(), y.as_f64()) {
                (Some(x), Some(y))
                    if x.abs() <= crate::doc::MAX_COORD && y.abs() <= crate::doc::MAX_COORD =>
                {
                    Some((x, y))
                }
                _ => return Err(BAD),
            },
            _ => return Err(BAD),
        },
    };
    let list = match obj.get("patch") {
        None => &Vec::new(),
        Some(v) => v.as_array().ok_or(BAD)?,
    };
    if list.len() > crate::tp::MAX_PATCH {
        return Err("error.draw2.tpPatch");
    }
    let named = |v: &Value| match v.as_i64() {
        Some(n) if (-1_000_000..0).contains(&n) => {
            json!(crate::tp::new_ref(n.unsigned_abs() as u32))
        }
        _ => v.clone(),
    };
    let mut patch = Vec::with_capacity(list.len());
    for item in list {
        let mut op = item.clone();
        if let Some(o) = op.as_object_mut() {
            for k in ["id", "a", "b"] {
                if let Some(v) = o.get_mut(k) {
                    *v = named(v);
                }
            }
            for k in ["atoms"] {
                if let Some(l) = o.get_mut(k).and_then(Value::as_array_mut) {
                    l.iter_mut().for_each(|v| *v = named(v));
                }
            }
        }
        patch.push(Op::parse(&op)?);
    }
    Ok((of, patch, at))
}

/// Runs one command as one undo step, with the route operations of the arrows that follow. The error is an i18n key and the id it
/// is about.
pub(crate) fn run(e: &mut Editor, name: &str, args: &Value) -> Result<(), OpError> {
    let bad = |key| OpError { key, id: 0 };
    if !known(name) {
        return Err(bad("error.draw2.command"));
    }
    if name == "pathway:anchor" || name == "pathway:route" {
        let id = pathway_args(name, args).map_err(bad)?;
        return match id {
            Some(arrow) => e.pathway_anchor(arrow).map(|_| ()),
            None => e.pathway_route().map(|_| ()),
        };
    }
    if name == "scheme:orient" {
        return orient(e);
    }
    if name == "tp:derive" {
        let (of, patch, at) = derive_args(args).map_err(bad)?;
        let ops = crate::tp::derive(e.doc(), &e.style, of, &patch, at).map_err(bad)?;
        return e.run_guarded(&ops);
    }
    if name == "scheme:row" {
        let opts = crate::scheme::Opts::parse(args).map_err(bad)?;
        let ops = crate::scheme::row(e.doc(), &e.style, &opts).map_err(bad)?;
        let ops = e.following(&ops);
        return e.run_guarded(&ops);
    }
    if name == "pathway:tree" {
        let ops = crate::pathway::command(e.doc(), &e.style).map_err(|r| OpError {
            key: r.key(),
            id: r.id(),
        })?;
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
    e.run_guarded(&ops)
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
    let count = |st: Status| plan.steps.iter().filter(|s| s.status == st).count();
    let (turned, same, big, none) = (
        count(Status::Oriented),
        count(Status::Already),
        count(Status::TooLarge),
        count(Status::NoCore),
    );
    if plan.ops.is_empty() {
        if same == 0 {
            let key = if big > 0 {
                Status::TooLarge.key()
            } else {
                Status::NoCore.key()
            };
            return Err(OpError { key, id: 0 });
        }
        // nothing to turn: oriented already, and the ones that could not be said as well
        e.said = if big + none == 0 {
            ("draw2.say.orientDone", String::new())
        } else {
            ("draw2.say.orientDonePart", (big + none).to_string())
        };
        return Ok(());
    }
    let ops = e.following(&plan.ops);
    e.session.run(&ops)?;
    e.said = if big + none == 0 {
        ("draw2.say.oriented", turned.to_string())
    } else {
        (
            "draw2.say.orientedPart",
            format!("{turned}/{}", turned + same + big + none),
        )
    };
    Ok(())
}
