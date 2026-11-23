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
    matches!(
        name,
        "scheme:orient" | "pathway:anchor" | "pathway:route" | "pathway:label"
    )
}

/// The history commands `undo:<n>`, `redo:<n>` (n steps, n from 1) and `jump:<n>` (to step number n, 0 is the start): the kind
/// and the number; `None` for any other name or a number that is not a plain unsigned integer.
pub(crate) fn steps_of(name: &str) -> Option<(&str, usize)> {
    let (kind, n) = name.split_once(':')?;
    let plain = !n.is_empty() && n.len() <= 9 && n.bytes().all(|b| b.is_ascii_digit());
    (matches!(kind, "undo" | "redo" | "jump") && plain).then(|| (kind, n.parse().unwrap_or(0)))
}

/// `group:<name>`: the click on the button of a group of the tools palette (`Editor::group_click`): it selects the member the button
/// shows, unless that is the tool already. The group, `None` for any other name.
pub(crate) fn group_of(name: &str) -> Option<&str> {
    let g = name.strip_prefix("group:")?;
    crate::tool_groups::Group::named(g).map(|_| g)
}

/// Whether `name` is a command of this editor.
pub(crate) fn known(name: &str) -> bool {
    steps_of(name).is_some()
        || group_of(name).is_some()
        || crate::arrange::is_arrange(name)
        || matches!(
            name,
            "move"
                | "rotate"
                | "flip"
                | "scheme:row"
                | "pathway:anchor"
                | "pathway:route"
                | "pathway:tree"
                | "pathway:label"
                | "scheme:orient"
                | "tp:derive"
                | "tp:import"
                | "tp:number"
                | "pathway:build"
                | "new"
                | "open"
                | "save"
                | "lonepairs"
        )
}

/// Checks the shape of a command before any command runs: a known name, `args` an object whose fields are the fields of the
/// operation of that name, the required ones present (`x`, `y` of a rotate and `at` of a flip may be left out). The error is an
/// invalid request, never a verdict about a document.
pub(crate) fn check(name: &str, args: &Value) -> Result<(), &'static str> {
    if !known(name) {
        return Err("error.draw2.command");
    }
    if let Some((kind, n)) = steps_of(name) {
        // no argument; undoing or redoing no step at all is no command
        return match args.as_object() {
            Some(o) if o.is_empty() && (n > 0 || kind == "jump") => Ok(()),
            _ => Err("error.draw2.request"),
        };
    }
    if group_of(name).is_some() {
        // no argument
        return match args.as_object() {
            Some(o) if o.is_empty() => Ok(()),
            _ => Err("error.draw2.request"),
        };
    }
    if crate::arrange::is_arrange(name) {
        crate::arrange::parse(name)?;
        return arrange_args(args).map(|_| ());
    }
    if name == "pathway:anchor" || name == "pathway:route" {
        return pathway_args(name, args).map(|_| ());
    }
    if matches!(name, "new" | "open" | "save") {
        return file_args(name, args).map(|_| ());
    }
    if name == "tp:derive" {
        return derive_args(args).map(|_| ());
    }
    if name == "tp:import" {
        return import_args(args).map(|_| ());
    }
    if name == "tp:number" {
        return number_args(args).map(|_| ());
    }
    if name == "pathway:build" {
        return build_args(args).map(|_| ());
    }
    if name == "lonepairs" {
        return lonepairs_args(args).map(|_| ());
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
    if name == "pathway:label" {
        return crate::steplabel::Opts::parse(args).map(|_| ());
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

/// The arguments of the file commands: `new` and `open` take `discard` (bool, default false: unsaved work is refused), `open` also
/// `text` (an `mzdraw/1` document, required), `save` takes none. Returns `(text, discard)`.
fn file_args(name: &str, args: &Value) -> Result<(String, bool), &'static str> {
    const BAD: &str = "error.draw2.request";
    let obj = args.as_object().ok_or(BAD)?;
    let allowed: &[&str] = match name {
        "new" => &["discard"],
        "open" => &["text", "discard"],
        _ => &["format"],
    };
    if obj.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err(BAD);
    }
    let discard = match obj.get("discard") {
        None => false,
        Some(v) => v.as_bool().ok_or(BAD)?,
    };
    let text = match obj.get("text").or_else(|| obj.get("format")) {
        None if name == "open" => return Err(BAD),
        None => String::new(),
        Some(v) => v.as_str().ok_or(BAD)?.to_string(),
    };
    // a format that is no file kind of the editor is a bad request, not a silent no-op
    if name == "save" && !text.is_empty() && !Editor::is_save_kind(&text) {
        return Err(BAD);
    }
    Ok((text, discard))
}

/// The arguments of an arrange command: `{"atoms": [ids], "objects": [ids]}`, both optional (nothing named: the selection, or the
/// whole drawing).
type Named = (Vec<u32>, Vec<u32>);

fn arrange_args(args: &Value) -> Result<Named, &'static str> {
    const BAD: &str = "error.draw2.request";
    let obj = args.as_object().ok_or(BAD)?;
    if obj.keys().any(|k| k != "atoms" && k != "objects") {
        return Err(BAD);
    }
    let list = |k: &str| -> Result<Vec<u32>, &'static str> {
        match obj.get(k) {
            None => Ok(Vec::new()),
            Some(v) => (v.as_array().ok_or(BAD)?.iter())
                .map(|n| {
                    n.as_u64()
                        .filter(|&n| n > 0 && n < u64::from(crate::doc::MAX_ID))
                        .map(|n| n as u32)
                        .ok_or(BAD)
                })
                .collect(),
        }
    };
    Ok((list("atoms")?, list("objects")?))
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

/// The arguments of `pathway:build`: `{"model": {"format":"pathway/1", ...}}` and, optionally, `"words": {"tentative": TEXT, "not_detected": TEXT}`
/// (the words the drawing shows next to the name of such a node, in the language of the caller; the page passes the words of its catalog).
/// The model itself is checked when it runs, with every problem listed.
pub(crate) type Words = (String, String);

fn build_args(args: &Value) -> Result<(&Value, Option<Words>), &'static str> {
    const BAD: &str = "error.draw2.request";
    let obj = args.as_object().ok_or(BAD)?;
    let model = match obj.get("model") {
        Some(m @ Value::Object(_)) => m,
        _ => return Err(BAD),
    };
    let word = |v: &Value| -> Option<String> {
        let s = v.as_str()?;
        (!s.is_empty() && s.chars().count() <= 32 && !s.chars().any(char::is_control))
            .then(|| s.to_string())
    };
    match (obj.get("words"), obj.len()) {
        (None, 1) => Ok((model, None)),
        (Some(Value::Object(w)), 2) if w.len() == 2 => {
            let t = w.get("tentative").and_then(word).ok_or(BAD)?;
            let n = w.get("not_detected").and_then(word).ok_or(BAD)?;
            Ok((model, Some((t, n))))
        }
        _ => Err(BAD),
    }
}

/// The arguments of `tp:import`: `{"csv": text}` or `{"rows": [objects]}`, one of them.
fn import_args(args: &Value) -> Result<crate::tp_list::Input<'_>, &'static str> {
    const BAD: &str = "error.draw2.request";
    let obj = args.as_object().ok_or(BAD)?;
    match (obj.get("csv"), obj.get("rows"), obj.len()) {
        (Some(Value::String(t)), None, 1) => Ok(crate::tp_list::Input::Csv(t)),
        (None, Some(r @ Value::Array(_)), 1) => Ok(crate::tp_list::Input::Rows(r)),
        _ => Err(BAD),
    }
}

/// The arguments of `lonepairs`: `{"atoms": [ids]}`, which may be left out (every atom).
fn lonepairs_args(args: &Value) -> Result<Vec<u32>, &'static str> {
    const BAD: &str = "error.draw2.request";
    let obj = args.as_object().ok_or(BAD)?;
    match (obj.get("atoms"), obj.len()) {
        (None, 0) => Ok(Vec::new()),
        (Some(Value::Array(l)), 1) => l
            .iter()
            .map(|v| {
                v.as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .filter(|&n| n > 0)
            })
            .collect::<Option<Vec<u32>>>()
            .ok_or(BAD),
        _ => Err(BAD),
    }
}

/// The arguments of `tp:number`: `{"force": bool}`, which may be left out.
fn number_args(args: &Value) -> Result<bool, &'static str> {
    const BAD: &str = "error.draw2.request";
    let obj = args.as_object().ok_or(BAD)?;
    match (obj.get("force"), obj.len()) {
        (None, 0) => Ok(false),
        (Some(Value::Bool(b)), 1) => Ok(*b),
        _ => Err(BAD),
    }
}

/// The arguments of `tp:derive`: `{"of": atom id, "patch": [operations on the parent's ids], "at": [x, y]}` (`patch` and `at` may be
/// left out) or `{"of", "step": NAME, "site": ID, "at"}` (a named step at a site in place of a `patch`: tp-10). In a patch a negative
/// id names a new atom of the patch: `-1` the first `atom.add`, `-2` the second, and so on.
type Derive = (u32, Patch, Option<(f64, f64)>);

/// What a derive applies to the copy.
pub(crate) enum Patch {
    Ops(Vec<Op>),
    /// A step of the table `tp::STEPS` and the atom or bond id of its site.
    Step(String, u32),
}

fn derive_args(args: &Value) -> Result<Derive, &'static str> {
    const BAD: &str = "error.draw2.request";
    let obj = args.as_object().ok_or(BAD)?;
    if !obj
        .keys()
        .all(|k| matches!(k.as_str(), "of" | "patch" | "at" | "step" | "site"))
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
    // a step and its site, in place of a patch
    match (obj.get("step"), obj.get("site")) {
        (None, None) => {}
        (Some(step), Some(site)) if !obj.contains_key("patch") => {
            let name = step.as_str().ok_or(BAD)?;
            if crate::tp::step(name).is_none() {
                return Err("error.draw2.tpStep");
            }
            let site = match site.as_u64() {
                Some(i) if i > 0 && i < u64::from(crate::doc::MAX_ID) => i as u32,
                _ => return Err(BAD),
            };
            return Ok((of, Patch::Step(name.to_string(), site), at));
        }
        _ => return Err(BAD),
    }
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
    Ok((of, Patch::Ops(patch), at))
}

/// Runs one command as one undo step, with the route operations of the arrows that follow. The error is an i18n key and the id it
/// is about.
pub(crate) fn run(e: &mut Editor, name: &str, args: &Value) -> Result<(), OpError> {
    let bad = |key| OpError { key, id: 0 };
    if !known(name) {
        return Err(bad("error.draw2.command"));
    }
    if let Some((kind, n)) = steps_of(name) {
        let ran = match kind {
            "undo" => e.undo_steps(n),
            "redo" => e.redo_steps(n),
            _ => e.jump_to(n),
        };
        return ran.map(|_| ()).map_err(bad);
    }
    if crate::arrange::is_arrange(name) {
        let (mut atoms, mut objects) = arrange_args(args).map_err(bad)?;
        if atoms.is_empty() && objects.is_empty() {
            (atoms, objects) = (e.over.sel_atoms.clone(), e.over.sel_objs.clone());
        }
        return e.arrange_on(name, &atoms, &objects);
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
    if matches!(name, "new" | "open" | "save") {
        let (text, discard) = file_args(name, args).map_err(bad)?;
        if name == "save" {
            // the native document unless a format is named; only the native document is the saved point
            e.exported(if text.is_empty() {
                Editor::save_kind()
            } else {
                &text
            });
            return Ok(());
        }
        return match name {
            "new" => e.new_drawing(discard).map(|_| ()),
            _ => e.open_drawing(&text, discard),
        }
        .map_err(bad);
    }
    if name == "tp:import" {
        let input = import_args(args).map_err(bad)?;
        e.failed_rows.clear();
        return match crate::tp_list::import_ops(e.doc(), &e.style, &input) {
            Ok(ops) => {
                let n = ops.len().to_string();
                e.session.run(&ops)?;
                e.said = ("draw2.say.tpImported", n);
                Ok(())
            }
            Err(crate::tp_list::Failure::Key(key, id)) => Err(OpError { key, id }),
            Err(crate::tp_list::Failure::Rows(rows)) => {
                e.failed_rows = rows;
                Err(bad(crate::tp_list::KEY_ROWS))
            }
        };
    }
    if name == "pathway:build" {
        let (model, words) = build_args(args).map_err(bad)?;
        e.failed_rows.clear();
        return match crate::pathmodel::build_ops(e.doc(), &e.style, model, words.as_ref()) {
            Ok(ops) => {
                let n = ops.len().to_string();
                e.session.run(&ops)?;
                e.said = ("draw2.say.tpImported", n);
                Ok(())
            }
            Err(crate::tp_list::Failure::Key(key, id)) => Err(OpError { key, id }),
            Err(crate::tp_list::Failure::Rows(rows)) => {
                e.failed_rows = rows;
                Err(bad(crate::pathmodel::KEY_ROWS))
            }
        };
    }
    if name == "lonepairs" {
        let atoms = lonepairs_args(args).map_err(bad)?;
        if let Some(&id) = atoms.iter().find(|&&i| e.doc().atom(i).is_none()) {
            return Err(OpError {
                key: "error.draw2.noAtom",
                id,
            });
        }
        let ops = crate::curve::lone_pair_ops(e.doc(), &atoms);
        if ops.is_empty() {
            e.said = ("draw2.say.lonepairsNone", String::new());
            return Ok(());
        }
        e.said = ("draw2.say.lonepairsPlaced", ops.len().to_string());
        return e.session.run(&ops);
    }
    if name == "tp:number" {
        let force = number_args(args).map_err(bad)?;
        let ops = crate::tp_list::number_ops(e.doc(), &e.style, e.style.pathway_number, force)
            .map_err(|(key, id)| OpError { key, id })?;
        e.said = ("draw2.say.tpNumbered", String::new());
        return e.session.run(&ops);
    }
    if name == "tp:derive" {
        let (of, patch, at) = derive_args(args).map_err(bad)?;
        let ops = match &patch {
            Patch::Ops(p) => crate::tp::derive(e.doc(), &e.style, of, p, at),
            Patch::Step(name, site) => {
                crate::tp::derive_step(e.doc(), &e.style, of, name, *site, at)
            }
        }
        .map_err(bad)?;
        return e.run_guarded(&ops);
    }
    if name == "scheme:row" {
        let opts = crate::scheme::Opts::parse(args).map_err(bad)?;
        let ops = crate::scheme::row(e.doc(), &e.style, &opts).map_err(bad)?;
        let ops = e.following(&ops);
        return e.run_guarded(&ops);
    }
    if name == "pathway:label" {
        let opts = crate::steplabel::Opts::parse(args).map_err(bad)?;
        return e.pathway_label(&opts).map(|_| ());
    }
    if let Some(g) = group_of(name) {
        e.group_click(g);
        return Ok(());
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
