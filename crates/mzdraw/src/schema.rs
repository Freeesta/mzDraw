//! The table of the operations of the command log `draw_edit/1`: one row per operation, the single source that the reader
//! (`Op::parse`), the docs (section "Edit language" of `docs/mzdraw.md`), the CLI (`"out":"schema"`) and mzLab read.
//!
//! The rows say what the readers of `ops.rs` accept, nothing more; a slice that adds an operation adds its row here.
//!
//! The `mzlab` column is the list of operations that the reader of mzLab (`crates/mzlab-feat/src/chem/edit.rs`) accepts:
//! mzLab `main` at 76a4703e6d7c26adce67d77caee744242317a280 reads `atom.add`, `atom.set`, `bond.add`, `bond.set`, `remove`,
//! `move` and `merge`; the commit pinned by this repository, ccd4680fc35b4ca6029cfead3a8e97f82042fdf7, has no such reader yet
//! (its `interp/edit.rs` is the unrelated edit of a row). The column is by operation name: mzLab reads some fields under other
//! names (`label`, `wedge`, `x_a`) and addresses bonds by their atoms; `tests/data/mzlab_ops.txt` holds the list and its source.

/// One field of an operation.
pub struct FieldSpec {
    pub key: &'static str,
    /// `uint`, `int`, `number`, `string`, `bool`, `uint[]` (ids), `string[][2]` (name and value pairs), `json[]` (a JSON list) or
    /// `enum`.
    pub ty: &'static str,
    /// The reader refuses the operation when the field is absent.
    pub required: bool,
    /// The unit of a number, or `""`.
    pub unit: &'static str,
    /// What the reader accepts besides the type (the range of the integer, the values of an enum), or `""`.
    pub bounds: &'static str,
    /// The reader of mzLab accepts this field (false: a field of mzDraw that mzLab does not read yet).
    pub mzlab: bool,
}

/// One operation of the log.
pub struct OpSpec {
    pub name: &'static str,
    pub fields: &'static [FieldSpec],
    /// What undoes it (the recorded inverse, `ops::Inv`).
    pub inverse: &'static str,
    /// The `mzdraw/1` version that introduced it.
    pub since: &'static str,
    /// This repository reads it.
    pub mzdraw: bool,
    /// The reader of mzLab accepts an operation of this name.
    pub mzlab: bool,
}

const fn f(
    key: &'static str,
    ty: &'static str,
    required: bool,
    unit: &'static str,
    bounds: &'static str,
) -> FieldSpec {
    FieldSpec {
        key,
        ty,
        required,
        unit,
        bounds,
        mzlab: true,
    }
}

/// A field that only mzDraw reads yet (`FieldSpec::mzlab` false).
const fn nf(
    key: &'static str,
    ty: &'static str,
    required: bool,
    unit: &'static str,
    bounds: &'static str,
) -> FieldSpec {
    FieldSpec {
        mzlab: false,
        ..f(key, ty, required, unit, bounds)
    }
}

const BL: &str = "bond length";
const U8: &str = "0..=255";

/// The operations, in the order of the enum `Op`, then those not read yet.
pub const OPS: &[OpSpec] = &[
    OpSpec {
        name: "atom.add",
        fields: &[
            f(
                "id",
                "uint",
                false,
                "",
                "from the document counter when absent",
            ),
            f("el", "string", true, "", "an element symbol"),
            f("x", "number", true, BL, ""),
            f("y", "number", true, BL, ""),
            nf("map", "uint", false, "", "atom-to-atom map number 1..=999"),
        ],
        inverse: "DropAtom, Next",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: true,
    },
    OpSpec {
        name: "bond.add",
        fields: &[
            f(
                "id",
                "uint",
                false,
                "",
                "from the document counter when absent",
            ),
            f("a", "uint", true, "", ""),
            f("b", "uint", true, "", ""),
            f("o", "uint", false, "", "0..=255, default 1"),
            f("s", "uint", false, "", "0..=255, default 0"),
        ],
        inverse: "DropBond, Next",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: true,
    },
    OpSpec {
        name: "atom.set",
        fields: &[
            f("id", "uint", true, "", ""),
            f("el", "string", false, "", "an element symbol"),
            f("chg", "int", false, "", "-128..=127"),
            f("iso", "uint", false, "", "0..=65535"),
            f("rad", "uint", false, "", U8),
            f("h", "uint", false, "", U8),
            f(
                "lab",
                "string",
                false,
                "",
                "the empty text takes the label away",
            ),
            nf(
                "map",
                "uint",
                false,
                "",
                "atom-to-atom map number 1..=999; 0 takes it away",
            ),
        ],
        inverse: "SetAtom",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: true,
    },
    OpSpec {
        name: "atom.data",
        fields: &[
            f("id", "uint", true, "", ""),
            f(
                "data",
                "string[][2]",
                true,
                "",
                "the empty list takes the data away",
            ),
        ],
        inverse: "SetAtom",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: false,
    },
    OpSpec {
        name: "atom.ann",
        fields: &[
            f("id", "uint", true, "", ""),
            f(
                "ann",
                "json[]",
                true,
                "",
                "the empty list takes the annotation away",
            ),
        ],
        inverse: "SetAtom",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: false,
    },
    OpSpec {
        name: "bond.set",
        fields: &[
            f("id", "uint", true, "", ""),
            f("o", "uint", false, "", U8),
            f("s", "uint", false, "", U8),
            f("rev", "bool", false, "", "default false"),
        ],
        inverse: "SetBond",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: true,
    },
    OpSpec {
        name: "rotate",
        fields: &[
            f("atoms", "uint[]", false, "", "default empty"),
            f("deg", "number", true, "degree", "counter-clockwise"),
            f("x", "number", true, BL, ""),
            f("y", "number", true, BL, ""),
        ],
        inverse: "SetAtom per atom",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: false,
    },
    OpSpec {
        name: "flip",
        fields: &[
            f("atoms", "uint[]", false, "", "default empty"),
            f("axis", "enum", true, "", "h, v"),
            f("at", "number", true, BL, ""),
        ],
        inverse: "SetAtom per atom",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: false,
    },
    OpSpec {
        name: "move",
        fields: &[
            f("atoms", "uint[]", false, "", "default empty"),
            f("objects", "uint[]", false, "", "default empty"),
            f("dx", "number", true, BL, ""),
            f("dy", "number", true, BL, ""),
        ],
        inverse: "SetAtom per atom, SetObj per object",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: true,
    },
    OpSpec {
        name: "remove",
        fields: &[
            f("atoms", "uint[]", false, "", "default empty"),
            f("bonds", "uint[]", false, "", "default empty"),
            f("objects", "uint[]", false, "", "default empty"),
        ],
        inverse: "PutAtom, PutBond, PutObj, each back in place",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: true,
    },
    OpSpec {
        name: "obj.add",
        fields: &[
            f(
                "id",
                "uint",
                false,
                "",
                "from the document counter when absent",
            ),
            f("t", "enum", true, "", "arrow, plus, text"),
            f("x", "number", true, BL, ""),
            f("y", "number", true, BL, ""),
            f("x2", "number", false, BL, "default 0"),
            f("y2", "number", false, BL, "default 0"),
            f("text", "string", false, "", "default empty"),
            f(
                "from",
                "uint",
                false,
                "",
                "arrows: an atom of the structure the arrow leaves; 0 or absent is a free end",
            ),
            f(
                "to",
                "uint",
                false,
                "",
                "arrows: an atom of the structure the arrow reaches; 0 or absent is a free end",
            ),
        ],
        inverse: "DropObj, Next",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: false,
    },
    OpSpec {
        name: "obj.set",
        fields: &[
            f("id", "uint", true, "", ""),
            f("text", "string", false, "", ""),
            f("x2", "number", false, BL, ""),
            f("y2", "number", false, BL, ""),
            f("from", "uint", false, "", "an atom id; 0 frees the end"),
            f("to", "uint", false, "", "an atom id; 0 frees the end"),
        ],
        inverse: "SetObj",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: false,
    },
    OpSpec {
        name: "ring",
        fields: &[
            f("n", "uint", true, "", "0..=255 (the editor takes 3..=8)"),
            f("arom", "bool", false, "", "default false"),
            f("atom", "uint", false, "", "a corner of the ring"),
            f("bond", "uint", false, "", "the bond the ring is fused on"),
            f("x", "number", false, BL, "the centre, default 0"),
            f("y", "number", false, BL, "the centre, default 0"),
        ],
        inverse: "DropAtom and DropBond per new item, Next",
        since: "mzdraw/1",
        mzdraw: true,
        mzlab: false,
    },
    // Not read here yet (arch-web-09 adds it); the fields are those mzLab reads.
    OpSpec {
        name: "merge",
        fields: &[
            f("from", "uint", true, "", ""),
            f("into", "uint", true, "", ""),
        ],
        inverse: "",
        since: "",
        mzdraw: false,
        mzlab: true,
    },
];

/// One field of the replay request (`mol draw`), or one arm of its `expect`.
pub struct RequestSpec {
    /// `request` for a field of the request, `expect` for an arm of the list `expect`.
    pub section: &'static str,
    pub key: &'static str,
    pub ty: &'static str,
    /// What the field does, one sentence.
    pub meaning: &'static str,
}

const fn r(
    section: &'static str,
    key: &'static str,
    ty: &'static str,
    meaning: &'static str,
) -> RequestSpec {
    RequestSpec {
        section,
        key,
        ty,
        meaning,
    }
}

/// The fields of the replay request, then the arms of `expect`: what `replay::replay` reads. A field outside the table is an invalid
/// request (`error.draw2.requestField`), an arm outside it too (`error.draw2.expect`).
pub const REQUEST: &[RequestSpec] = &[
    r("request", "import", "string", "a structure text to start from: `mzdraw/1` JSON, a molfile or a SMILES"),
    r("request", "import_base64", "string", "the bytes of a file to start from in base64 (a binary CDX, or a PNG or PDF saved by mzDraw), read after `import`"),
    r("request", "style", "string|object", "the style of the SVG: a preset name or a settings file"),
    r("request", "set", "string[]", "settings applied after `style`, each `<section>.<key>=<value>`"),
    r("set", "display.maps", "on|off", "draw the atom-to-atom map numbers at the atoms (default on; key `maps` of the settings file)"),
    r("request", "log", "json[]", "the command log, operations of `draw_edit/1`"),
    r("request", "out", "string", "the output wanted (`mzdraw` by default): a format name, `log`, `std`, `schema` or `settings`"),
    r("request", "commands", "object[]", "editor commands `[{\"command\":NAME,\"args\":{...}}]` (move, rotate, flip, scheme:row), run after `log` and before `out`; the route of the anchored arrows follows; not with `command`"),
    r("request", "command", "string", "one editor command, with `args`: the same as a `commands` list of length one; not with `commands`"),
    r("request", "args", "object", "the fields of the operation of the same name as `command` (a rotate without x, y or a flip without at works about the centre of its atoms); `scheme:row` takes `gap` (bond lengths between neighbours, default 1.0, above 0), `wrap` (a width in bond lengths that breaks the row after an arrow, default 0: never) and `atoms`, `objects` (the selection, default: everything) and writes `move` and `obj.set` operations"),
    r("request", "events", "array[]", "pointer and key events the page forwards, run after `commands` and before `out`: `[\"pointer\",phase,id,x,y,kind,mods,t]` and `[\"key\",key,mods,t]`; their flags and what they leave are read with `out` `context-state:<w>,<h>` (the context menu of a point is `context:<x>,<y>`)"),
    r("request", "report", "bool", "answer with one `draw-result/1` document instead of the bare output"),
    r("request", "dry_run", "bool", "run and report everything, write nothing: the document has no `out`; implies `report`"),
    r("request", "expect", "object[]", "postconditions on the final drawing, one arm each; a non-empty list implies `report`"),
    r("request", "id", "string|uint", "echoed as the `id` of the document (default: the index of the request in its batch)"),
    r("expect", "std", "string", "`\"none\"`: the drawing has no `std/1` finding"),
    r("expect", "std_absent", "string[]", "none of these `std/1` keys is found"),
    r("expect", "pieces", "uint", "the atoms form exactly this many connected pieces"),
    r("expect", "smiles", "string", "the SMILES of the whole drawing equals this text"),
    r("expect", "stereo", "string", "`\"kept\"`: every stereocentre and double bond of the drawing after `import` is arranged as at the end; `detail` names the atoms"),
];

/// The row of an operation by name.
pub fn spec(name: &str) -> Option<&'static OpSpec> {
    OPS.iter().find(|o| o.name == name)
}

fn q(s: &str) -> String {
    serde_json::Value::from(s).to_string()
}

/// The table as `draw-edit-schema/1`, written by hand with the keys in a fixed order (the same bytes on every build).
pub fn json() -> String {
    let mut out = String::from(r#"{"format":"draw-edit-schema/1","ops":["#);
    for (i, o) in OPS.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            r#"{{"op":{},"since":{},"mzdraw":{},"mzlab":{},"inverse":{},"fields":["#,
            q(o.name),
            q(o.since),
            o.mzdraw,
            o.mzlab,
            q(o.inverse)
        ));
        for (j, fl) in o.fields.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            let opt = |s: &str| if s.is_empty() { "null".into() } else { q(s) };
            out.push_str(&format!(
                r#"{{"key":{},"type":{},"required":{},"unit":{},"bounds":{}{}}}"#,
                q(fl.key),
                q(fl.ty),
                fl.required,
                opt(fl.unit),
                opt(fl.bounds),
                if fl.mzlab { "" } else { r#","mzlab":false"# }
            ));
        }
        out.push_str("]}");
    }
    out.push_str("],\"request\":[");
    for (i, x) in REQUEST.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            r#"{{"section":{},"key":{},"type":{},"meaning":{}}}"#,
            q(x.section),
            q(x.key),
            q(x.ty),
            q(x.meaning)
        ));
    }
    out.push_str("]}");
    out
}
