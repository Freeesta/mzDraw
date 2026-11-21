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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
        since: crate::brand::FIRST_DOCUMENT_ID,
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
    r("request", "import", "string", "a structure text to start from: `{doc}` JSON, a molfile or a SMILES"),
    r("request", "import_base64", "string", "the bytes of a file to start from in base64 (a binary CDX, or a PNG or PDF saved by {name}), read after `import`"),
    r("request", "style", "string|object", "the style of the SVG: a preset name or a settings file"),
    r("request", "set", "string[]", "settings applied after `style`, each `<section>.<key>=<value>`"),
    r("set", "display.maps", "on|off", "draw the atom-to-atom map numbers at the atoms (default on; key `maps` of the settings file)"),
    r("command", "pathway:tree", "object", "lay out the whole pathway of the anchored arrows (layers, tidy tree across them, columns, routed arrows) as one undo step, with no `args`; a directed cycle, two arrows between the same structures or no anchored arrow is `error.draw2.schemeCycle`, `error.draw2.schemeDuplicate`, `error.draw2.schemeEmpty` (exit 1, `failed.stage` `command`)"),
    r("command", "pathway:anchor", "object", "`args` `{\"arrow\":ID}`: anchors each end of that arrow to the nearest structure within 1 bond length (box distance; ties by the smallest atom id) and routes it; an end with nothing in reach stays as it was"),
    r("command", "tp:derive", "object", "`args` `{\"of\":ATOM_ID,\"patch\":[OPS],\"at\":[X,Y]}` (`patch`, `at` optional): copies the structure that holds `of` (atoms, bonds, labels, wedges; no data or annotation) with its exact coordinates and atom maps, applies `patch` to the copy, and anchors a routed arrow from the parent to the copy, as one step. The parent atoms get maps 1..n (after the largest map of the drawing) when they have none; each copy atom keeps its source's map; a new atom takes the next free map. `patch` is `draw_edit/1` operations written on the PARENT's atom and bond ids and coordinates (atom.add, bond.add, atom.set, bond.set, remove, move, rotate, flip, atom.data, atom.ann; no object operations); a negative id names a new atom of the patch (`-1` the first atom.add, `-2` the second). `at` is the centre of the copy's box; without it the copy stands one column right of the parent (`pathway.col_gap`), or below the last structure in the way (`pathway.row_gap`). Refused, nothing written: `error.draw2.tpPatch` (an invalid valence, an id the parent lacks), `error.draw2.tpMaps` (a map above 999 would be needed), `error.draw2.tpMapDup` (a patch gives a map number that another atom of the drawing has), `error.draw2.tpSize` (a parent above 5000 atoms); the log holds the plain operations; more than 10000 operations in `patch` is `error.draw2.tpPatch`"),
    r("command", "pathway:label", "object", "`args` `{\"force\":bool,\"isomer\":WORD}`, both optional: writes the formula delta of the two structures (`+O`, `-CH2`; the isomer word for equal formulas) as the text of every arrow anchored at both ends, one undo step (`obj.set`); an arrow whose text is not empty and not made only of such tokens (a signed delta, a signed number with 4 decimals, the isomer word) is kept unless `force`; the charge is not part of a delta, and two structures with the same atoms and another net charge are left alone; `pathway.label` other than `delta` is refused: `error.draw2.labelMass` (`mass`, `delta+mass`: the core has no mass callback to the glue yet) or `error.draw2.labelSteps` (`name+delta`: no table of steps yet), exit 1, `failed.stage` `command`; `changed` is reported"),
    r("command", "pathway:route", "object", "no `args`: routes every anchored arrow in one step; no operation when nothing moves"),
    r("set", "pathway.*", "string|number", "the pathway layout: `dir` (`right`|`down`), `col_gap` (2.5), `row_gap` (1), `max_angle` (40 degrees), `min_arrow` (1.2), `order` (`id`|`mz`|`rt`), `margin` (0.35), in bond lengths, and `derive` (`on`|`off`, default off: the page's \"Derive TP\" item and Shift+D; the command `tp:derive` is always available)"),
    r("set", "pathway.label", "string", "what `pathway:label` writes: `delta` (default), `mass`, `delta+mass` or `name+delta`; only `delta` can be run in this build"),
    r("request", "log", "json[]", "the command log, operations of `draw_edit/1`"),
    r("request", "out", "string", "the output wanted (`{slug}` by default): a format name, `log`, `std`, `schema` or `settings`; `smiles` of a drawing with a connected piece above 500 atoms is `error.draw2.smilesSize`"),
    r("request", "commands", "object[]", "editor commands `[{\"command\":NAME,\"args\":{...}}]` (move, rotate, flip, scheme:row, pathway:tree, `pathway:anchor`, `pathway:route`, `pathway:label`, `tp:derive`, `scheme:orient`: every structure after an arrow turned onto the one before it, no `args`; a scheme oriented already is a no-op, `changed` false), run after `log` and before `out`; the route of the anchored arrows follows; not with `command`; the section `command` lists the pathway commands, which have no operation of the same name (`move`, `rotate`, `flip` and `scheme:row` take the fields of `args`)"),
    r("request", "command", "string", "one editor command, with `args`: the same as a `commands` list of length one; not with `commands`"),
    r("request", "args", "object", "the fields of the operation of the same name as `command` (a rotate without x, y or a flip without at works about the centre of its atoms); `scheme:row` takes `gap` (bond lengths between neighbours, default 1.0, above 0), `wrap` (a width in bond lengths that breaks the row after an arrow, default 0: never) and `atoms`, `objects` (the selection, default: everything) and writes `move` and `obj.set` operations"),
    r("request", "events", "array[]", "pointer and key events the page forwards, run after `commands` and before `out`: `[\"pointer\",phase,id,x,y,kind,mods,t]` and `[\"key\",key,mods,t]`; their flags and what they leave are read with `out` `context-state:<w>,<h>` (the context menu of a point is `context:<x>,<y>`)"),
    r("request", "report", "bool", "answer with one `draw-result/1` document instead of the bare output"),
    r("request", "dry_run", "bool", "run and report everything, write nothing: the document has no `out`; implies `report`"),
    r("request", "expect", "object[]", "postconditions on the final drawing, one arm each; a non-empty list implies `report`"),
    r("request", "id", "string|uint", "echoed as the `id` of the document (default: the index of the request in its batch)"),
    r("request", "propose", "json[]", "a patch of `draw_edit/1` held as a proposal after `commands` and before `events` (`Editor::propose`): not in the drawing and not in the log until a hunk is accepted. It must apply to the drawing as it is (else the refusal of its operation), have 1 to 2000 operations (`error.draw2.proposalEmpty`, `error.draw2.proposalSize`) that name at most 50000 ids in all (every atom, bond and object an operation names counts, so one `remove` or `move` with a long list does too: `error.draw2.proposalIds`); its hunks are those of `diff::hunks`, numbered from 1 in the order of their first operation"),
    r("request", "accept", "uint[]", "with `propose`: the hunks (numbers from 1) accepted in that order, one undo step each, with exactly the operations of the hunk (no route operation follows), so the log holds those and nothing else; the hunks left are dropped, `[]` or no `accept` leaves the drawing and the log as they were; a number outside the hunks, or one given twice, is `error.draw2.proposalHunk` (a verdict at `failed.stage` `proposal`, `op` the index in `accept`; a refused `propose` has no `op`); without `propose` it is an invalid request (`error.draw2.request`)"),
    r("request", "diff", "object", "two documents `{\"a\":DOC,\"b\":DOC}` (each a document object or its JSON text): answers the patch from `a` to `b` and its hunks (section `diff`), not a `draw-result/1`; the request stands alone: any other field, `report`, `dry_run`, `expect` and `id` included, is `error.draw2.diffAlone`; an unreadable document is `error.draw2.document`"),
    r("diff", "ops", "json[]", "answer: the patch, operations of `draw_edit/1` (`diff::diff(a, b)`): removes, adds, sets, moves, in that order; empty when the drawings are the same"),
    r("diff", "hunks", "object[]", "answer: the patch grouped into hunks (`diff::hunks(a, ops)`), in the order of their first operation; any subset of them applies in order to a valid document"),
    r("hunk", "ids", "uint[]", "the ids (atoms, bonds, objects) the hunk touches, ascending"),
    r("hunk", "ops", "json[]", "the operations of the hunk, in the order of the patch: a self-contained patch of `a`"),
    r("out", "loss:FORMAT", "string", "`loss/1`: what the format would drop or change of this drawing, `{\"format\":\"loss/1\",\"target\":FORMAT,\"items\":[{\"what\",\"atoms\",\"bonds\",\"objects\"}]}`, one item per thing, ordered by `what` then smallest id, the lists ascending; the kinds are the rows of the table \"What a format loses\" of the formats document; for a kind that depends on the drawing (a wedge mark, a SMILES radical, the plus signs of a reaction file, the shape in a SMILES, an SDF or a reaction file, compared in bond lengths) the file is written and read back, and when some of the kind do not come back every occurrence of it is listed (the answer is per kind, not per id), since the reader does not say which; the scale a molfile, V3000, KET or CDXML reader sets (bond length 1) is not checked and not a loss; a drawing the writer refuses is its key; `error.draw2.lossSize` is a drawing above 50000 things, a connected piece above 500 atoms for `smiles`, more than 600 atoms for `smiles`, `sdf` or `rxn` (their shape is compared), one a writer would make more than 40000 atoms of (abbreviations counted as the atoms they stand for) where a round trip is needed, or a file that reads back as more than 3000 atoms or that its reader refuses (the key is `lossSize`, not the reader's)"),
    r("out", "boxes", "string", "`boxes/1`: the label-inclusive box of every structure (by its smallest atom id) and every arrow with the structures it joins, in bond lengths at 4 decimals, ordered by id: the boxes the pathway layout uses"),
    r("out", "digest", "string", "`digest/1`: a few hundred bytes that say where things are: per structure, left to right, `ids` (atom ids as ranges), `formula` (Hill), `box` (x0, y0, x1, y1 in bond lengths, one decimal, label ink included), `labels` (`[id, text]` of every atom that is not a plain carbon and every abbreviation), `stereo` (`[bond, wedge|hash|wavy, narrow-end atom]`) and `rings`; then `objects` (`[id, kind, box]`), `clearance` (the smallest gap between two structure boxes, bond lengths; null with fewer than two) and `std` (the number of `std/1` findings per key; null with `stdSkipped` `atoms>1000` above 1000 atoms); at most 200 structures (and about 12,000 bytes of them) and 200 objects are listed, the rest counted in `more`; at most 150 labels, 50 stereo entries and 400 bytes of `ids` per structure, the rest counted in `truncated` (`[labels, stereo, atoms]`); a drawing whose atoms times bonds is above 25 million is refused (`error.draw2.digestSize`, exit 2)"),
    r("std", "std.scheme.gapRaised", "info", "a gap between two columns of a pathway was widened (`pathway:tree`); `atoms`: the first root of the component, `objects`: the primary arrows between the two layers"),
    r("std", "std.scheme.free", "info", "arrows without two structures, plus signs and texts, which `pathway:tree` does not move (`objects`)"),
    r("std", "std.scheme.crossing", "warn", "an arrow whose visible segment meets the route box of a structure other than its two ends (`objects`: the arrow, `atoms`: the structures it meets)"),
    r("expect", "std", "string", "`\"none\"`: the drawing has no `std/1` finding"),
    r("expect", "std_absent", "string[]", "none of these `std/1` keys is found"),
    r("expect", "pieces", "uint", "the atoms form exactly this many connected pieces"),
    r("expect", "lossless", "string", "the format (`molfile`, `molfile-v3000`, `smiles`, `sdf`, `ket`, `cdxml`, `cml`, `rxn`, `svg` or `{slug}`) drops and changes nothing of the drawing (`out` `loss:FORMAT` lists what it would); `detail` names the kinds lost, and a drawing the format cannot write or is too large to check (`error.draw2.lossSize`) is false"),
    r("expect", "smiles", "string", "the SMILES of the whole drawing equals this text; a drawing with a connected piece above 500 atoms has none (`error.draw2.smilesSize`), so the verdict is false"),
    r("expect", "anchored", "object", "`{\"arrow\":ID,\"from\":A,\"to\":B}`: the arrow is anchored to the structures of the atoms A and B (0 or left out: that end is free)"),
    r("expect", "stereo", "string", "`\"kept\"`: every stereocentre and double bond of the drawing after `import` is arranged as at the end; `detail` names the atoms"),
];

/// The row of an operation by name.
pub fn spec(name: &str) -> Option<&'static OpSpec> {
    OPS.iter().find(|o| o.name == name)
}

fn q(s: &str) -> String {
    serde_json::Value::from(s).to_string()
}

/// A meaning with the names of the product written in: `{doc}` the document id, `{name}` the product name, `{slug}` its slug.
fn meaning(text: &str) -> String {
    use crate::brand::{DOCUMENT_ID, NAME, SLUG};
    text.replace("{doc}", DOCUMENT_ID)
        .replace("{name}", NAME)
        .replace("{slug}", SLUG)
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
            q(&meaning(x.meaning))
        ));
    }
    out.push_str("]}");
    out
}
