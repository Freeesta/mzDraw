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
            nf(
                "lp",
                "uint",
                false,
                "",
                "the lone pairs drawn on the atom, 0..=4 (above four is error.draw2.lp); 0 takes them away (a curve that starts or ends on one is then drawn at the atom)",
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
        name: "atom.title",
        fields: &[
            f("id", "uint", true, "", ""),
            nf(
                "title",
                "string",
                true,
                "",
                "one line of at most 80 characters with no control character; the empty text takes the title away; the title is the first line of the molfile and of each SDF record the structure is written as (the name of a SMILES line)",
            ),
        ],
        inverse: "SetAtom",
        since: crate::brand::FIRST_DOCUMENT_ID,
        mzdraw: true,
        mzlab: false,
    },
    OpSpec {
        name: "stereo.group",
        fields: &[
            nf("atoms", "uint[]", true, "", "the atoms, each a stereocentre of the record (a defined tetrahedral centre: the wedges as drawn); not empty"),
            nf(
                "g",
                "string",
                true,
                "",
                "the enhanced-stereo group: `abs` (absolute), `andN` (`&N` of CXSMILES, `STERACn` of V3000: the drawn isomer and its enantiomer, unknown ratio) or `orN` (`oN`, `STERELn`: one of the two, unknown which), N from 1 to 99; `none` takes the atoms out of any group; kept in the atom field `stg`; drawn next to its atom (`abs` only when the drawing also has an `and` or `or` group); written to V3000 and to the CXSMILES tail of a SMILES; another word, or an atom that is no stereocentre, is `error.draw2.stereoGroup`",
            ),
        ],
        inverse: "SetAtom",
        since: crate::brand::FIRST_DOCUMENT_ID,
        mzdraw: true,
        mzlab: false,
    },
    OpSpec {
        name: "atom.cap",
        fields: &[
            nf("id", "uint", true, "", "an atom of the molecule; the choice is kept on it"),
            nf(
                "lines",
                "json[]",
                true,
                "",
                "the line keys written under the molecule, in order: `formula`, `mw`, `exact`, `ea`, `mz`, `logp`, `tpsa`, `hbd`, `hba`, `rotb`, `heavy`; at most 24, each at most 40 characters; the empty list takes the choice away (the molecule follows `display.values`); a key this version does not know is kept and writes no line",
            ),
        ],
        inverse: "SetAtom",
        since: crate::brand::FIRST_DOCUMENT_ID,
        mzdraw: true,
        mzlab: false,
    },
    OpSpec {
        name: "doc.labels",
        fields: &[
            nf(
                "mode",
                "string",
                true,
                "",
                "the mass-label choice of the scheme (the document field `labels`): `delta`, `mass`, `delta+mass` or `name+delta`, what `pathway:label` writes; the empty text takes the choice away (the settings `pathway.label` and `pathway.adduct` apply again); any other word is `error.draw2.labels`, never a fallback",
            ),
            nf(
                "adduct",
                "string",
                false,
                "",
                "one of the seven of `display.adducts`, only with `mass` or `delta+mass`: the m/z of the product is written after the label; empty or left out: none; any other value, or an adduct with another mode, is `error.draw2.labels`",
            ),
        ],
        inverse: "SetMore",
        since: crate::brand::FIRST_DOCUMENT_ID,
        mzdraw: true,
        mzlab: false,
    },
    OpSpec {
        name: "colour",
        fields: &[
            nf("atoms", "uint[]", false, "", "atom ids"),
            nf("bonds", "uint[]", false, "", "bond ids"),
            nf("objects", "uint[]", false, "", "object ids: arrows, plus signs, texts"),
            nf(
                "col",
                "string",
                true,
                "",
                "`#rgb` or `#rrggbb` in any case, kept as `#rrggbb`; the empty text is automatic and takes the colour away (the element colour, or the ink of the mono style, shows again)",
            ),
        ],
        inverse: "SetAtom, SetBond, SetObj",
        since: crate::brand::FIRST_DOCUMENT_ID,
        mzdraw: true,
        mzlab: false,
    },
    OpSpec {
        name: "bond.set",
        fields: &[
            f("id", "uint", true, "", ""),
            f("o", "uint", false, "", U8),
            f(
                "s",
                "uint",
                false,
                "",
                "0..=255: the wedge mark of a single bond, 0 none, 1 solid, 2 hashed, 3 wavy; on a double bond 3 is \"either\" (its E/Z is unknown: bond stereo 3 in a V2000 molfile, `CFG=2` in V3000, `Display=\"Wavy\"` in CDXML) and 0 takes it away; a new order other than 2 takes \"either\" away",
            ),
            nf(
                "pos",
                "string",
                false,
                "",
                "where the second line of a double bond goes, seen from the atom `a` to the atom `b`: `auto` (the default: inside a ring, centred at a free end, else the side with more substituents), `c`, `l` or `r`; a new order other than 2 takes it away",
            ),
            nf(
                "k",
                "string",
                false,
                "",
                "how the line is drawn: `solid` (the default), `dash`, `bdash` (bold dashed), `hash` (short ticks across the line, not a stereo mark), `bold` (a heavy line), `dative` (an arrow from the atom `a`, the donor, to the atom `b`) or `hbond` (a dotted line, a hydrogen bond); `dative` and `hbond` are for a single bond (a new order other than 1 takes them away); a bond with a wedge mark has none, and a new mark takes it away",
            ),
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
            f("t", "enum", true, "", "arrow, plus, text, curve"),
            f("x", "number", true, BL, ""),
            f("y", "number", true, BL, ""),
            f("x2", "number", false, BL, "default 0"),
            f("y2", "number", false, BL, "default 0"),
            f("text", "string", false, "", "default empty"),
            f(
                "runs",
                "array",
                false,
                "",
                "the formatted text: a list of runs {t, br, sub, sup, b, i, u, font, sz, color} (see docs/built/text-runs.md); when not empty it is the text and `text` is ignored",
            ),
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
            nf(
                "head",
                "enum",
                false,
                "",
                "curves: full (an electron pair, the default) or half (one electron)",
            ),
            nf(
                "bend",
                "number",
                false,
                "",
                "curves: the bow as a fraction of the chord, -3 to 3, positive to the left of the way the curve goes (y up); default 0",
            ),
            nf(
                "fl",
                "bool",
                false,
                "",
                "curves: the start is the lone pair of the atom `from`; default false",
            ),
            nf(
                "tl",
                "bool",
                false,
                "",
                "curves: the end is the lone pair of the atom `to`; default false",
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
            f(
                "runs",
                "array",
                false,
                "",
                "the formatted text, as for obj.add; `text` and `runs` together are refused (error.draw2.object); an invalid list has its key (error.draw2.runsCount, runFont, runColor, runSize, runStyle); an empty list leaves no text; a plain `text` takes the runs away",
            ),
            f("x2", "number", false, BL, ""),
            f("y2", "number", false, BL, ""),
            f("from", "uint", false, "", "an atom id; 0 frees the end"),
            f("to", "uint", false, "", "an atom id; 0 frees the end"),
            nf("x", "number", false, BL, "curves only: the first point, kept for an end that is a point; an end freed with `from: 0` keeps where it stood"),
            nf("y", "number", false, BL, "curves only: with `x`"),
            nf(
                "head",
                "enum",
                false,
                "",
                "curves: full (an electron pair, the default) or half (one electron)",
            ),
            nf(
                "bend",
                "number",
                false,
                "",
                "curves: the bow as a fraction of the chord, -3 to 3, positive to the left of the way the curve goes (y up)",
            ),
            nf(
                "fl",
                "bool",
                false,
                "",
                "curves: the start is the lone pair of the atom `from`; freeing the start (`from: 0`) clears it",
            ),
            nf(
                "tl",
                "bool",
                false,
                "",
                "curves: the end is the lone pair of the atom `to`; freeing the end (`to: 0`) clears it",
            ),
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
    OpSpec {
        name: "chain",
        fields: &[
            f("n", "uint", true, "", "bonds, 1..=50"),
            f("atom", "uint", false, "", "the atom the chain starts from; without it the chain starts at `x`,`y`"),
            f("x", "number", false, BL, "the start when there is no `atom`, default 0"),
            f("y", "number", false, BL, "the start when there is no `atom`, default 0"),
            f("deg", "number", false, "degree", "the direction of the first bond, 0 = right, 90 = up, default 0; the next bond turns by -60, the one after by +60 (120 degrees at every atom)"),
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
    r("request", "paste", "string", "a structure text pasted as the page pastes it, after `commands`: at the view centre when that place is free, else at the nearest free place to its right and below"),
    r("request", "canvas", "number[2]", "the canvas size `[w, h]` in pixels the paste takes the view centre from (default `[400, 400]`)"),
    r("request", "style", "string|object", "the style of the SVG: a preset name or a settings file"),
    r("request", "set", "string[]", "settings applied after `style`, each `<section>.<key>=<value>`"),
    r("set", "display.titles", "off|on", "draw the title of each structure (`atom.title`, the first line of its molfile and SDF record) under it as its legend, the first line of its caption, before the values of `display.values` or `atom.cap` (default off; key `titles` of the settings file, written when on); in the canvas, the SVG, the PDF, the PNG and the CDXML captions; a value that is none is `error.draw2.styleValue`"),
    r("set", "display.cip", "off|on", "draw the CIP labels in brackets: R/S at a stereocentre, E/Z beside a double bond, placed as the ids are (default off; key `cip` of the settings file, written when on); the labels come from the engine `cip` of mzLab, which the page and the glue replay have: the plain core replay draws none and its `draw-result/1` says so in `lost` (`draw2.export.cipEngine`); the engine is given one structure at a time, a structure with more than 16 open stereo sites or 500 atoms gets no label (`lost` `draw2.export.cipCapped`) and one it refuses none (`draw2.export.cipFailed`); in the canvas, the SVG, the PDF and the PNG; a value that is none is `error.draw2.styleValue`"),
    r("set", "display.maps", "on|off", "draw the atom-to-atom map numbers at the atoms (default on; key `maps` of the settings file)"),
    r("set", "display.ids", "off|atoms|bonds|both", "draw the ids of the document, the numbers the operations of `draw_edit/1` name: each where it is farthest from what is drawn near it (an atom's on one of 16 places round the atom, clear of its label; a bond's on either side of its middle), small and muted (default `off`; key `ids` of the settings file, written when not off); in the canvas and in every picture (SVG, PDF, PNG); a value that is none is `error.draw2.styleValue`"),
    r("set", "display.values", "off|formula|mass|mz", "what is drawn under each structure (default off; key `display.values` of the settings file): the Hill formula, the formula and the monoisotopic mass, or one line per adduct with its m/z; the numbers need the engine `sel` of mzLab (the page and the glue replay have it; the plain core replay draws the formula only)"),
    r("set", "display.adducts", "[M+H]+,[M+Na]+,...", "the adducts whose m/z is calculated, a comma list of `[M+H]+`, `[M+NH4]+`, `[M+Na]+`, `[M+K]+`, `[M-H]-`, `[M+Cl]-`, `[M+HCOO]-`, each at most once, in the order shown (default `[M+H]+,[M-H]-`; empty is none; key `display.adducts` of the settings file)"),
    r("set", "display.lens", "off|tint|tint+ghost", "the diff lens on the product of an anchored arrow (default `off`; key `lens` of the settings file, written when not off): `tint` draws the atoms and bonds the product added or changed in the accent colour, `tint+ghost` also draws each bond of the parent that the product no longer has as a thin dashed muted line between its two atoms, behind the bonds. Display only: not in any molfile, SMILES, KET, CDXML or RXN; in an SVG, PDF or PNG only when set at export"),
    r("set", "ui.theme", "auto|light|dark", "the theme of the page (default `light`; `auto`: the system or the host decides; key `theme` of the object `ui` of the settings file, written when not `light`). Page only: it changes no drawing, no export and no replay output; a value that is none is `error.draw2.ui`"),
    r("set", "text.font", "default|arial|helvetica|times|courier|georgia|verdana", "the font of a new text (default `default`: the font of the drawing's style; key `font` of the object `text` of the settings file, written when not `default`). A new text made by the text tool gets runs with that family; a value that is none is `error.draw2.textPref`"),
    r("set", "text.size", "50|75|100|125|150|200|300|400", "the size of a new text as a percent of the size of the style (default `100`; key `size` of the object `text` of the settings file, written when not `100`). A value that is none is `error.draw2.textPref`"),
    r("command", "group:<name>", "object", "no `args`; `name` is a group of the tools palette (`bonds`, `rings`, `atoms`, `charges`, `objects`, `text`): the click on its button, which selects the member the button shows (the member last used, else the first: `bond:1`, `ring:6a`, `charge:+`, `arrow`) unless that is the tool already; a group with a face of its own icon (`atoms`, `text`) selects nothing. It changes no drawing and no operation; the faces are the object `heads` of `layout` (`--set layout.heads.<group>=<tool>`, written to the settings file `out: settings`). An unknown group is `error.draw2.command`"),
    r("command", "pathway:tree", "object", "lay out the whole pathway of the anchored arrows (layers, tidy tree across them, columns, routed arrows) as one undo step, with no `args`; a directed cycle, two arrows between the same structures or no anchored arrow is `error.draw2.schemeCycle`, `error.draw2.schemeDuplicate`, `error.draw2.schemeEmpty` (exit 1, `failed.stage` `command`)"),
    r("command", "undo:N", "object", "no `args`; `undo:<n>`, n from 1: undoes n steps (at most as many as there are; the steps stay in the history list, `redo` brings them back); the same as `n` presses of undo, no new operation, nothing added to the log. `redo:<n>` redoes n steps the same way. `jump:<n>` (n from 0, 0 is the start) leaves the drawing as step number n of the `history` output made it, by undoing or redoing; a number past the newest step is `error.draw2.request` (exit 2) and changes nothing"),
    r("command", "pathway:anchor", "object", "`args` `{\"arrow\":ID}`: anchors each end of that arrow to the nearest structure within 1 bond length (box distance; ties by the smallest atom id) and routes it; an end with nothing in reach stays as it was"),
    r("command", "tp:derive", "object", "`args` `{\"of\":ATOM_ID,\"patch\":[OPS],\"at\":[X,Y]}` (`patch`, `at` optional) or `{\"of\":ATOM_ID,\"step\":NAME,\"site\":ID,\"at\":[X,Y]}`: a named step of the table `steps` at a site (an atom id, or a bond id for a bond step; the sites of a step are listed by `out` `sites`) in place of a `patch`, never together with one; the step expands to a patch and the delta of the copy is checked against the table (`error.draw2.tpDelta` when it is not the step's, nothing written), a site where the test of the step fails is `error.draw2.tpSite` (a verdict, exit 1, `failed.stage` `command`), an unknown step name is `error.draw2.tpStep` (an invalid request); copies the structure that holds `of` (atoms, bonds, labels, wedges; no data or annotation) with its exact coordinates and atom maps, applies `patch` to the copy, and anchors a routed arrow from the parent to the copy, as one step. The parent atoms get maps 1..n (after the largest map of the drawing) when they have none; each copy atom keeps its source's map; a new atom takes the next free map. `patch` is `draw_edit/1` operations written on the PARENT's atom and bond ids and coordinates (atom.add, bond.add, atom.set, bond.set, remove, move, rotate, flip, atom.data, atom.ann; no object operations); a negative id names a new atom of the patch (`-1` the first atom.add, `-2` the second). `at` is the centre of the copy's box; without it the copy stands one column right of the parent (`pathway.col_gap`), or below the last structure in the way (`pathway.row_gap`). Refused, nothing written: `error.draw2.tpPatch` (an invalid valence, an id the parent lacks), `error.draw2.tpMaps` (a map above 999 would be needed), `error.draw2.tpMapDup` (a patch gives a map number that another atom of the drawing has), `error.draw2.tpSize` (a parent above 5000 atoms); the log holds the plain operations (no new operation: a step is a patch); more than 10000 operations in `patch` is `error.draw2.tpPatch`"),
    r("command", "pathway:label", "object", "`args` `{\"force\":bool,\"isomer\":WORD,\"adduct\":NAME}`, all optional: writes the formula delta of the two structures (`+O`, `-CH2`; the isomer word for equal formulas) as the text of every arrow anchored at both ends, one undo step (`obj.set`); an arrow whose text is not empty and not made only of such tokens (a signed delta, a signed number with 4 decimals, the isomer word) is kept unless `force`; the charge is not part of a delta, and two structures with the same atoms and another net charge are left alone; `pathway.label` `name+delta` writes the name of the one step of the table `steps` whose delta is the label, then the delta (`dehydrogenation -H2`; a delta that two steps share, such as `+O`, or none has, is written alone); a step name is one of the tokens this command owns; `pathway.label` `mass` writes the mass shift `TP - parent` (`+15.9949`; the difference of the two unrounded masses of the engine `sel` of mzLab, rounded once, 4 decimals ties to even: the digits of `mass_diff` of `tptable`) and `delta+mass` the delta and the shift (`+O +15.9949`); an isomer is the isomer word with no shift; `adduct` (one of the seven of `display.adducts`; only with `mass` or `delta+mass`, else `error.draw2.request`) adds the product's m/z (`[M+H]+ 151.1117`, the digits of `mz_calc`); the numbers need the engine (the page and the glue replay have it): without it `mass` and `delta+mass` are `error.draw2.labelMass` exit 1, `failed.stage` `command`; a structure the engine refuses (an R group, `Na+`, over 200 atoms: its own key) leaves its arrow unlabelled, counted as skipped, and a charged product has no m/z (`draw2.label.mzCharged`): both are in `lost` of a `draw-result/1` as (key, arrows); more than 15000 atoms or 1000 structures is `error.draw2.valuesSize`; `changed` is reported"),
    r("command", "tp:import", "object", "`args` `{\"csv\":TEXT}` or `{\"rows\":[OBJECT]}` (exactly one): a list of transformation products becomes the drawing in one step. A row is `smiles,name,parent,mz,rt,adduct,level` (the optional column `area` too): CSV with RFC 4180 quoting (the header names the columns, `smiles` is required; line 1 is the header), or JSON objects with those keys (the line is the 1-based index); `parent` is the `name` of another row (`;` between several parents: a convergent product), empty is a root; at most 10000 rows, 1 MB, 500 atoms (5000 characters) a row and 15000 atoms in all, counted in the text before any structure is built. Each row is a structure (coordinates computed) with the data `TP_ID` (its `name`), `MZ`, `RT`, `ADDUCT` (a word of the form `[M<terms>]<n><+|->`, such as `[M+H]+`, `[M-H]-`, `[M+H-NH3]+`, `[M+2H]2+`: the syntax is checked, the adduct is not looked up, and the m/z is not checked against the structure) and `AREA` (never drawn) in `atom.data` of its smallest atom, and the level as its annotation; each parent link is an anchored arrow; then `scheme:orient`, the `pathway:tree` layout, the formula delta on each arrow (`pathway:label`), and `TP<n>` for the rows with no name (the smallest numbers not in use, in the order of `pathway.number`). All or nothing: when any row fails, nothing is applied and no `out` is written, the answer is exit 1 with `failed.key` `error.draw2.tpRows` and `failed.rows` `[{\"line\",\"key\"}]` listing every failing row in line order; the row keys are `error.draw2.tpRowShape`, `tpRowSmiles`, `tpRowSize`, `tpRowName`, `tpRowDup`, `tpRowNumber`, `tpRowAdduct`, `tpRowLevel`, `tpRowParent`, `tpRowCycle`; a list refused as a whole is `error.draw2.tpHeader`, `tpEmpty`, `tpListSize` (rows, bytes or atoms) or the layout's `error.draw2.schemeSize` (more than 1000 linked structures); `dry_run` validates and writes nothing; the page takes the same CSV in its Add field and file button"),
    r("command", "pathway:build", "object", "`args` `{\"model\":OBJECT,\"words\":{\"tentative\":TEXT,\"not_detected\":TEXT}}` (`words` optional, 1 to 32 characters each, kept as data `STATUS_TEXT` and drawn next to the name of such a node; English when left out; nothing else): the declarative pathway `pathway/1` becomes the drawing in one step: `{\"format\":\"pathway/1\",\"nodes\":[{\"id\":TEXT,\"smiles\":TEXT,\"formula\":TEXT,\"label\":TEXT,\"ion\":TEXT,\"mz\":NUMBER,\"rt\":NUMBER,\"status\":\"observed\"|\"tentative\"|\"not_detected\",\"generation\":INT,\"branch\":TEXT}],\"edges\":[{\"from\":ID,\"to\":ID,\"label\":TEXT}]}`, 1 to 500 nodes (`id` and `smiles` required; a node with a formula alone is refused: no structure to draw yet) and up to 1000 edges, no other key. The nodes are the rows of `tp:import` (structures, anchored arrows, orientation, the tree layout of `pathway:tree`, the formula delta on each arrow) and then, on the same copy, the data of each structure (`TP_ID` the label else the id, `NODE_ID`, `FORMULA`, `STATUS` when not observed, `GENERATION`, `BRANCH`, with `MZ`, `RT`, `ADDUCT` of `tp:import`) and the text of each edge that has a `label`; all or nothing, every problem listed in `failed.rows` as `[{\"line\",\"key\"}]` (the line is the place in `nodes`, then in `edges`; a model refused as a whole is `error.draw2.pathModel`, one with problems `error.draw2.pathRows` with the row keys `pathNodeShape`, `pathNodeId`, `pathNodeDup`, `pathNodeSmiles`, `pathNodeFormula`, `pathNodeStatus`, `pathNodeNumber`, `pathEdgeShape`, `pathEdgeEnd`, `pathEdgeLoop`, `pathEdgeDup`, then the row keys of `tp:import` on the line of the node); `dry_run` validates and writes nothing"),
    r("command", "tp:number", "object", "`args` `{\"force\":BOOL}` (optional, default false): sets `TP_ID` = `TP1`, `TP2`, ... on the products of the pathway (the structures an anchored arrow reaches) in the order of `pathway.number` (`layout`: by layer, then by position in the layer; `mz`; `rt`; a missing `MZ` or `RT` sorts last). A root keeps its name; a name that is not `TP<n>` is kept unless `force`, and does not use a number; one undo step, no change when nothing differs; a drawing with no anchored arrow is `error.draw2.schemeEmpty`"),
    r("command", "pathway:route", "object", "no `args`: routes every anchored arrow in one step; no operation when nothing moves"),
    r("command", "new", "object", "`args` `{\"discard\":BOOL}` (optional, default false): a new, empty drawing as one undo step, and the saved point. A drawing with unsaved work (dirty and not empty: an edit, `log` operation or command since the start text, the last `save` or the last `open`) is refused with `error.draw2.unsaved` and nothing changes, unless `discard` is true or a `save` came first. A refusal is the keyed error `error.draw2.unsaved` of the request (with `report`, `dry_run` or `expect`: a verdict `ok` false with `failed` `{\"stage\":\"command\",\"op\":INDEX,\"key\":\"error.draw2.unsaved\",\"id\":0}` and `dirty` true); the exit code and the stdout/stderr split are those of `mzlab-cli mol draw` (not verified here, mzLab side). The page asks \"Save changes?\" before it runs this command and gives `discard` after \"Don't save\""),
    r("command", "open", "object", "`args` `{\"text\":TEXT,\"discard\":BOOL}` (`text` required, `discard` optional, default false): the saved document in `text` (the text the page writes with Save) replaces the drawing with an empty history, and is the saved point; same refusal of unsaved work as `new`. Other formats are read by `import` (the start) or added by the page"),
    r("command", "save", "object", "`args` `{\"format\":KIND}` (optional, default the native document): the file of that kind was written. Only the native document is loss-free, so only it is the saved point (`dirty` is false in the result until the next edit); a molfile, SMILES, SVG, PNG, PDF or any other format leaves the drawing dirty. The bytes are what `out` writes. The page runs it after each download"),
    r("set", "pathway.*", "string|number", "the pathway layout: `dir` (`right`|`down`), `col_gap` (2.5), `row_gap` (1), `max_angle` (40 degrees), `min_arrow` (1.2), `order` (`id`|`mz`|`rt`), `number` (`layout`|`mz`|`rt`: the order `tp:number` numbers the TPs in), `margin` (0.35), in bond lengths, and `derive` (`on`|`off`, default off: the page's \"Derive TP\" item and Shift+D; the command `tp:derive` is always available)"),
    r("set", "pathway.adduct", "string", "the adduct whose m/z a mass label adds when the scheme has no choice of its own (`doc.labels`): `none` (default) or one of the seven of `display.adducts`; used only with `pathway.label` `mass` or `delta+mass`"),
    r("set", "pathway.label", "string", "what `pathway:label` writes: `delta` (default), `mass`, `delta+mass` or `name+delta`; `mass` and `delta+mass` need the engine of mzLab (`error.draw2.labelMass` without it: the plain core replay has none)"),
    r("request", "log", "json[]", "the command log, operations of `draw_edit/1`"),
    r("request", "out", "string", "the output wanted (`{slug}` by default): a format name, `log`, `history`, `std`, `lens`, `schema`, `settings`, `steps`, `sites` (with `args`), `tptable` or `tptable-csv`; `smiles` of a drawing with a connected piece above 500 atoms is `error.draw2.smilesSize`"),
    r("request", "commands", "object[]", "editor commands `[{\"command\":NAME,\"args\":{...}}]` (move, rotate, flip, `undo:<n>`, `redo:<n>`, `jump:<n>`, scheme:row, grid, pathway:tree, `pathway:anchor`, `pathway:route`, `pathway:label`, `tp:import`, `tp:number`, `pathway:build`, `tp:derive`, `scheme:orient`: every structure after an arrow turned onto the one before it, no `args`; a scheme oriented already is a no-op, `changed` false), run after `log` and before `out`; the route of the anchored arrows follows; not with `command`; the section `command` lists the pathway commands, which have no operation of the same name (`move`, `rotate`, `flip` and `scheme:row` take the fields of `args`)"),
    r("request", "command", "string", "one editor command, with `args`: the same as a `commands` list of length one; not with `commands`"),
    r("request", "args", "object", "the fields of the operation of the same name as `command` (a rotate without x, y or a flip without at works about the centre of its atoms); `scheme:row` takes `gap` (bond lengths between neighbours, default 1.0, above 0), `wrap` (a width in bond lengths that breaks the row after an arrow, default 0: never) and `atoms`, `objects` (the selection, default: everything) and writes `move` and `obj.set` operations; `grid` takes `cols` (columns, 1 to 100000, default the smallest whose square holds the structures), `gap` (bond lengths between cells, default 1.0, above 0), `order` (`drawing`, `reading` or `title`: by the legend, numbers compared as numbers, untitled last; default `drawing`) and `atoms` (the selection, default: every structure) and writes `move` operations, each cell as wide as the widest structure or caption and the legends of a row on one line; the grid never covers a structure outside the selection or an object it does not move (it goes under them), and the result key `grid` lists the free objects it left"),
    r("request", "events", "array[]", "pointer and key events the page forwards, run after `commands` and before `out`: `[\"pointer\",phase,id,x,y,kind,mods,t]` and `[\"key\",key,mods,t]`; their flags and what they leave are read with `out` `context-state:<w>,<h>` (the context menu of a point is `context:<x>,<y>`)"),
    r("request", "keys", "object[]", "keys pressed over an object, run after `events` and before `out`, each through `Editor::key_over` (the hover keys of `hotkeys::TABLE`): `{\"key\":\"2\",\"mods\":0,\"hover\":{\"bond\":3},\"focus\":{\"atom\":2,\"from\":1}}`; `mods` is the sum of Shift 1, Ctrl 2, Alt 8; `hover` is `{\"atom\":id}` or `{\"bond\":id}`; `focus` is the atom with the keyboard focus and the atom it came from (the walked bond); each of the last two is optional; a bad entry, or an id that is not in the drawing, is `error.draw2.keys`"),
    r("request", "report", "bool", "answer with one `draw-result/1` document instead of the bare output; its `imported` lists what the import of `import` or `import_base64` did not keep or changed, the page's import notes as `{\"key\",\"n\"}` (`draw2.import.stereoGroups` for enhanced-stereo groups, query atoms, S-groups, skipped records; the computed layout is no item): `docs/schemas/draw-result-1.json`"),
    r("request", "dry_run", "bool", "run and report everything, write nothing: the document has no `out`; implies `report`"),
    r("request", "expect", "object[]", "postconditions on the final drawing, one arm each; a non-empty list implies `report`"),
    r("request", "id", "string|uint", "echoed as the `id` of the document (default: the index of the request in its batch)"),
    r("request", "propose", "json[]", "a patch of `draw_edit/1` held as a proposal after `commands` and before `events` (`Editor::propose`): not in the drawing and not in the log until a hunk is accepted. It must apply to the drawing as it is (else the refusal of its operation), have 1 to 2000 operations (`error.draw2.proposalEmpty`, `error.draw2.proposalSize`) that name at most 50000 ids in all (every atom, bond and object an operation names counts, so one `remove` or `move` with a long list does too: `error.draw2.proposalIds`); its hunks are those of `diff::hunks`, numbered from 1 in the order of their first operation"),
    r("request", "accept", "uint[]", "with `propose`: the hunks (numbers from 1) accepted in that order, one undo step each, with exactly the operations of the hunk (no route operation follows), so the log holds those and nothing else; the hunks left are dropped, `[]` or no `accept` leaves the drawing and the log as they were; a number outside the hunks, or one given twice, is `error.draw2.proposalHunk` (a verdict at `failed.stage` `proposal`, `op` the index in `accept`; a refused `propose` has no `op`); without `propose` it is an invalid request (`error.draw2.request`)"),
    r("request", "values", "true|object", "the answer is `values/1` (the calculated values), not a drawing: for each structure the Hill formula, the composition, the charge, and, from the engine `sel` of mzLab, the monoisotopic mass, the average mass, the m/z of the adducts of `display.adducts` (null for a charged structure, which has its own `ionMz`), as strings with 4 decimals; `{\"atoms\":[ID,...]}` adds the selection of those atoms as one structure, a bond cut by the selection closed with hydrogen. Stands alone: with `out`, `report`, `dry_run` or `expect` it is `error.draw2.valuesAlone`. `engine` says whether the engine was there (false: formulas only, every number null); `error` is `error.draw2.valuesSize` above 5000 atoms or 200 structures; a structure with an R group has `error.draw2.valuesOpen`, no number"),
    r("request", "props", "true|object", "the answer is `props/1`, not a drawing: for each structure (and, with `atoms`, for those atoms as one selection) every line of `atom.cap` as `[name, value]` pairs: `formula`, `mw`, `exact`, `ea` (mass per cent of each element), `mz` (one pair per adduct of `display.adducts`), `logp` (Crippen), `tpsa`, `hbd`, `hba`, `rotb`, `heavy`, with the choice of lines kept under the molecule (`cap`); the numbers of the first four need the engine `sel`/`analysis` of mzLab (the page and the glue replay have it; the plain core replay gives the other lines and `\"engine\": false`). The object is `{\"atoms\": [ID...], \"text\": \"text|tsv|line\", \"lines\": [KEY...]}`, each optional: with `text` the answer is the values copied as plain text (blocks of `name: value` lines, tab-separated rows with a header, or one line per structure), of the keys in `lines`, else of the choice kept under each molecule, else of every line; a form that is none, a key list that is not one or nothing to copy is `error.draw2.props` or `error.draw2.propsCopy`. The answer also has `methods` (per line key: a stable `id` and the `text` of the method) and, per structure, `propsError`: the key that says why there is no descriptor number (`error.draw2.propsElement`, `error.draw2.valuesOpen`, `error.draw2.valuesSize`, or the engine key); the engine lines of a structure the engine refused, or gave a mass of zero for (`error.draw2.valuesZero`), are empty and so are its descriptors. The `text` and `tsv` copies end a block with `note` and that key, and carry the method id (`[id]` after the value, a fourth column); the `line` form has no method. An id the drawing lacks is `error.draw2.noAtom`, a key of `lines` that is none `error.draw2.propsLines`. It stands alone, as `values` does (`error.draw2.valuesAlone`)"),
    r("request", "feedback", "string|object", "the answer is `feedback/1`: a prefilled new issue (`url`, `template`, `title`, `body`, `truncated`, `notes`); `\"bug\"`, `\"improvement\"` or `{\"kind\", \"log\", \"agent\", \"lang\"}`; `log` puts the command log in the URL if it fits 6000 bytes; stands alone: no `out`, `report`, `dry_run`, `expect`, `values`, `propose` or `id` (`error.draw2.feedbackAlone`)"),
    r("request", "diff", "object", "two documents `{\"a\":DOC,\"b\":DOC}` (each a document object or its JSON text): answers the patch from `a` to `b` and its hunks (section `diff`), not a `draw-result/1`; the request stands alone: any other field, `report`, `dry_run`, `expect` and `id` included, is `error.draw2.diffAlone`; an unreadable document is `error.draw2.document`"),
    r("diff", "ops", "json[]", "answer: the patch, operations of `draw_edit/1` (`diff::diff(a, b)`): removes, adds, sets, moves, in that order; empty when the drawings are the same"),
    r("diff", "hunks", "object[]", "answer: the patch grouped into hunks (`diff::hunks(a, ops)`), in the order of their first operation; any subset of them applies in order to a valid document"),
    r("hunk", "ids", "uint[]", "the ids (atoms, bonds, objects) the hunk touches, ascending"),
    r("hunk", "ops", "json[]", "the operations of the hunk, in the order of the patch: a self-contained patch of `a`"),
    r("out", "loss:FORMAT", "string", "`loss/1`: what the format would drop or change of this drawing, `{\"format\":\"loss/1\",\"target\":FORMAT,\"items\":[{\"what\",\"atoms\",\"bonds\",\"objects\"}]}`, one item per thing, ordered by `what` then smallest id, the lists ascending; the kinds are the rows of the table \"What a format loses\" of the formats document; for a kind that depends on the drawing (a wedge mark, a SMILES radical, the plus signs of a reaction file, the shape in a SMILES, an SDF or a reaction file, compared in bond lengths) the file is written and read back, and when some of the kind do not come back every occurrence of it is listed (the answer is per kind, not per id), since the reader does not say which; the scale a molfile, V3000, KET or CDXML reader sets (bond length 1) is not checked and not a loss; a drawing the writer refuses is its key; `error.draw2.lossSize` is a drawing above 50000 things, a connected piece above 500 atoms for `smiles`, more than 600 atoms for `smiles`, `sdf` or `rxn` (their shape is compared), one a writer would make more than 40000 atoms of (abbreviations counted as the atoms they stand for) where a round trip is needed, or a file that reads back as more than 3000 atoms or that its reader refuses (the key is `lossSize`, not the reader's)"),
    r("out", "steps", "string", "`steps/1`: the table of named steps, `{\"format\":\"steps/1\",\"steps\":[{\"name\",\"delta\",\"kind\"}]}` in table order: `name` is what `tp:derive` takes as `step` and `pathway.label=name+delta` writes, `delta` the formula delta in the canonical spelling (`+O`, `-CH2`, `+H-Cl`, `+O3S`), `kind` `atom` or `bond` (what the site is); the mass shift is not there (arch-web-17 is not built)"),
    r("out", "sites", "string", "`sites/1`, with the request field `args` `{\"of\":ATOM_ID,\"step\":NAME}`: where the step applies in the structure that holds `of`, `{\"format\":\"sites/1\",\"step\",\"delta\",\"kind\",\"ids\":[...]}`, the atom ids (kind `atom`) or bond ids (kind `bond`) where the site test holds, ascending: a list to choose from, never a ranking; hydrogen counts are those of the `mzlab-mol` record; an atom with a label is never a site; an unknown step is `error.draw2.tpStep`, a structure above 5000 atoms `error.draw2.tpSize`, an `of` that is no atom `error.draw2.noAtom`, any other shape of `args` `error.draw2.request` (exit 2)"),
    r("out", "history", "string", "`history/1`: the steps of the session, newest first, `{\"format\":\"history/1\",\"at\":N,\"start\":\"current\"|\"done\",\"steps\":[{\"say\",\"what\",\"index\",\"state\"}]}`: `say` the i18n key of the step and `what` its parameter (after `log`, a step is named `draw2.history.op` with the operation name; after a command, with the words the page would say), `index` its number from 1 (the oldest), `state` `current` (the newest step still applied: where the drawing is), `done` (older ones) or `undone` (a `redo` brings it back); `at` is the index of the current step, 0 at the start; `start` is `current` when the drawing is at the start (no step applied), else `done`. A new step after an undo forgets the undone ones (no branching)"),
    r("out", "pathway-check", "string", "`pathway-check/1`: the automatic checks of a pathway drawing, read from the drawing (structures, anchored arrows, the data `NODE_ID`, `FORMULA`, `STATUS`, `GENERATION`, `MZ`, `ADDUCT`), keys `format`, `ok` (no finding of severity error), `engine`, `checked` (false without the engine: no mass, ion or numeric label was looked at), `polarity`, `tolerance` (`ppm`, `da`), `counts` (`nodes`, `edges`, `error`, `warn`, `info`, `unchecked`, `arrow_box_crossings`, `edge_crossings`, `label_box_overlaps` (estimated boxes)), `nodes` (`id`, `label`, `formula`, `monoisotopic_mass`, `status`, `generation`, `branch`, `ions` = the expected `{adduct, mz}` of the polarity, `observed` = `{adduct, mz, expected, ppm}`), `edges` (`from`, `to`, `label`, `delta` the formula delta of the two structures, `delta_mass`, `result` `ok`, `mismatch`, `unparsed`, `unlabelled`, `unchecked`) and `findings` (`severity`, `key`, `item`, `detail`; keys `pathway.edgeMismatch`, `pathway.formulaMismatch`, `pathway.ionMismatch` errors, `pathway.mzUnreadable` (an observed m/z that is not a finite number above 0: an error) and `pathway.edgeUnparsed` (a label that is given and cannot be read: an error unless `lenient_labels`, then a warning; a label is read as signed terms with an optional leading count and spaces, `+O`, `-CH2`, `+2H`, `+O-2H`, `+O2-2H`, `+2 H`, with a free word before the first term or a comment in parentheses after a term (a bare token after a term, `+O 2H`, is unreadable), a signed number against the delta of the masses, or the word isomer); `pathway.generationMismatch`, `pathway.ionUnmatched`, `pathway.edgeUnparsed`, `pathway.crossing`, `pathway.edgeCrossing`, `pathway.labelOverlap`, `pathway.isotopesNotCompared` (a structure with isotopes: formulas and deltas count elements only) warnings; `pathway.edgeUnlabelled`, `pathway.edgeUnchecked`, `pathway.ionNotComputed`, `pathway.engine` info; `pathway.layout` error; `pathway.engine` is an error with `require_engine`). A charged structure is its own ion (`[M]+`, from the engine's ion m/z). The expected m/z decides unrounded; only the printed number is rounded. The request `args` `{\"polarity\":\"both\"|\"positive\"|\"negative\",\"ppm\":N,\"da\":N,\"lenient_labels\":BOOL,\"require_engine\":BOOL}` (all optional; defaults both, 10, 0.005, false, false). Masses and m/z come from the engine (none in the plain replay: `null` and `pathway.engine`). The replay example exits 3 when `ok` is false (1 stays for a refusal), the answer being written all the same"),
    r("out", "tptable", "string", "`tptable/1` (tp-09): the pathway as a table, `{\"format\":\"tptable/1\",\"columns\":[15 names],\"engine\":BOOL,\"rows\":[{\"tp_id\",\"parent_id\",\"generation\",\"smiles\",\"formula\",\"monoisotopic_mass\",\"adduct\",\"mz_calc\",\"mz_obs\",\"ppm\",\"rt\",\"form_diff\",\"mass_diff\",\"level\",\"identifier\"}],\"notes\":[{\"key\",\"count\",\"items\":[TP_IDS]}]}`; every value is a string, empty for no value (never 0); one row per structure and parent (a product with two parents has two rows, a root one row with empty `parent_id`, `form_diff`, `mass_diff`), in the order `tp:number` numbers the products in (layer, then position; `pathway.number`), roots first, structures no anchored arrow touches last; `smiles` is the isomeric SMILES of `smiles` for that structure, `formula` its Hill formula, `form_diff` the delta `TP - parent` of `pathway:label` (empty for an isomer), `generation` the layer of the pathway (empty with no anchored arrow); `tp_id`, `mz_obs`, `rt`, `adduct` are the data `TP_ID`, `MZ`, `RT`, `ADDUCT` of the structure as typed, `level` and `identifier` its annotation; masses (4 decimals, ties to even, `mass_diff` signed from the unrounded masses) and the m/z of the typed adduct (one of the seven of `display.adducts`) come from the engine of mzLab through the bridge, `ppm` is `(mz_obs - mz_calc) / mz_calc * 10^6` from the printed `mz_calc`, 1 decimal; without the engine (the plain replay) those cells are empty, never made up, and `engine` is false; what is left empty for a reason is a note (`draw2.export.tpEngine`, `tpMass`, `tpAdduct`, `tpCharged`, `tpSmiles`, `tpParentName`, `tpDelta`, `tpNumber`) and is in `lost` of a `draw-result/1`; more than 15,000 atoms or 1,000 structures is `error.draw2.tpTableSize`, an anchored cycle or more than 1000 arrows the layout's key; the key table is `docs/schemas/tptable-1.json`"),
    r("out", "tptable-csv", "string", "the rows of `tptable` as CSV (RFC 4180, header on line 1, lines end in LF), NORMAN-style column names (`form_diff`, `mass_diff`, one row per parent-TP pair; not validated by NORMAN); a text cell (`tp_id`, `parent_id`, `adduct`, `level`, `identifier`) that starts with `=`, `+`, `-` or `@` gets a leading `'` (counted in `lost` as `draw2.export.tpGuard`); the cells `form_diff` and `mass_diff` are not guarded and a spreadsheet may read `+O` as a formula; a CSV of this form is read back as a list by `import` (the computed columns are not read)"),
    r("out", "boxes", "string", "`boxes/1`: the label-inclusive box of every structure (by its smallest atom id) and every arrow with the structures it joins, in bond lengths at 4 decimals, ordered by id: the boxes the pathway layout uses"),
    r("out", "lens", "string", "`lens/1`: `{\"format\":\"lens/1\",\"arrows\":[{\"arrow\":ID,\"added\":[ATOM_IDS],\"added_bonds\":[BOND_IDS],\"changed\":[ATOM_IDS],\"changed_bonds\":[BOND_IDS],\"cleaved\":[[A,B]],\"lost\":[MAPS],\"delta\":\"+O\"}],\"skipped\":[{\"arrow\":ID,\"why\":\"no maps\"}]}`, one entry per anchored arrow by id, from the structure the arrow leaves (P) to the one it reaches (T), compared by atom map number. `added`: atoms of T whose map P lacks; `added_bonds`: bonds of T with an added end, and bonds between two atoms both have that P did not bond; `changed`: atoms in both whose element, label, charge, isotope, radical or hydrogen count differ; `changed_bonds`: bonds in both whose order differs; `cleaved`: bonds of P between two maps T has but does not bond, as the two T atom ids (smaller first); `lost`: maps of P that T lacks; `delta`: the signed element counts from P to T as `pathway:label` writes them (empty for equal formulas; `null` where there is no honest delta: a free-text label, or equal counts with a different net charge). Ids ascending. Nothing is dropped in silence: an anchored arrow between two structures that has no entry in `arrows` is in `skipped` (by arrow id) with `why`: `no maps` (neither structure has a map; no `std/1` finding), `unmapped` (partly mapped, an atom without a map or two atoms of a structure sharing one: also the finding `std.tp.unmapped`, never a partial lens) or `over cap` (a structure above 5000 atoms, or more than 2000000 atoms compared in all, the rest are skipped). A drawing above 6000 atoms is `error.draw2.lensSize` (exit 1, nothing computed); with `display.lens` not off the page shows a note instead of nothing for a skipped arrow or an over-size drawing"),
    r("out", "digest", "string", "`digest/1`: a few hundred bytes that say where things are: per structure, left to right, `ids` (atom ids as ranges), `formula` (Hill), `box` (x0, y0, x1, y1 in bond lengths, one decimal, label ink included), `labels` (`[id, text]` of every atom that is not a plain carbon and every abbreviation), `stereo` (`[bond, wedge|hash|wavy, narrow-end atom]`) and `rings`; then `objects` (`[id, kind, box]`), `clearance` (the smallest gap between two structure boxes, bond lengths; null with fewer than two) and `std` (the number of `std/1` findings per key; null with `stdSkipped` `atoms>1000` above 1000 atoms); at most 200 structures (and about 12,000 bytes of them) and 200 objects are listed, the rest counted in `more`; at most 150 labels, 50 stereo entries and 400 bytes of `ids` per structure, the rest counted in `truncated` (`[labels, stereo, atoms]`); a drawing whose atoms times bonds is above 25 million is refused (`error.draw2.digestSize`, exit 2)"),
    r("std", "std.scheme.gapRaised", "info", "a gap between two columns of a pathway was widened (`pathway:tree`); `atoms`: the first root of the component, `objects`: the primary arrows between the two layers"),
    r("std", "std.scheme.free", "info", "arrows without two structures, plus signs and texts, which `pathway:tree` does not move (`objects`)"),
    r("std", "std.scheme.gapLimit", "warn", "a gap between two columns of a pathway could not be widened enough to keep arrows clear of the neighbouring boxes: the bound of the rule was reached and the gap was left as it was, or, for the secondary arrows and the arrows that skip a layer, kept at the widening that left the fewest of them meeting a box (`pathway:tree`; `atoms`: the first root of the component, `objects`: the primary arrows between the two layers, or the long arrows that span the gap and still meet a box; the arrows that meet a box are also listed as `std.scheme.crossing`)"),
    r("std", "std.scheme.crossing", "warn", "an arrow whose visible segment meets the route box of a structure other than its two ends (`objects`: the arrow, `atoms`: the structures it meets)"),
    r("std", "std.tp.unmapped", "info", "an anchored arrow between two partly mapped structures that the diff lens cannot compare by atom map (`objects`: the arrow, `atoms`: the atoms of both structures that have no map or share one; a pair with no map at all is not reported); the lens has no entry for it"),
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
            r#"{{"op":{},"since":{},"{}":{},"mzlab":{},"inverse":{},"fields":["#,
            q(o.name),
            q(o.since),
            crate::brand::SCHEMA_KEY,
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
