# mzDraw: the annotation of a structure (field list)

An annotation says how sure the identification of a drawn structure is, and on what evidence. It is built in
`crates/mzdraw/src/ann.rs`; this file is its field list and is kept true with the code.

## Where it lives

- **Document `mzdraw/1`**: the field `ann` of one atom of the structure (the atom the annotation was set on), next to `data`.
  The annotation of a structure is the annotations of its atoms in document order, one after the other. A new optional
  field does not change the format version; a mzDraw that does not know `ann` keeps it and writes it back.
- **Command log `draw_edit/1`**: `{"op":"atom.ann","id":<atom id>,"ann":[<entry>, ...]}` replaces the annotation kept on
  that atom; `"ann":[]` takes it away. The operation has an exact inverse (undo gives the same bytes).
- **Text**: an annotation has one text (JSON, keys in alphabetical order, no spaces); the same annotation gives the same bytes
  in the document, the log and an SDF.

## The annotation: a list of entries

`ann` is a list of entries. It is a history: **the last entry is the current one**. At most 50 entries and 20,000
characters of text.

## One entry

| Field | Required | Type | Shape | Meaning |
|---|---|---|---|---|
| `level` | yes | string | one word of 1 to 16 characters: ASCII letters, digits, `-`, `_`, `.` | The confidence level. On the scale of Schymanski et al. 2014: `"1"`, `"2a"`, `"2b"`, `"3"`, `"4"`, `"5"`. **Always a string, never a number.** Any other word (`"3a"`, `"0"`, a level of another scale) is not refused: it is kept as read and written back unchanged. |
| `scale` | no | string | one line, up to 40 characters | The scale `level` is read on. Entries made in mzDraw say `"schymanski-2014"`. |
| `identifier` | no | string | one line, up to 200 characters, not starting with `>` or `$$$$` | What the structure is identified as: a name, an InChIKey, a database id. |
| `evidence` | no | list of objects, up to 50 | see below | What the level rests on. |
| `scores` | no | list of objects, up to 50 | see below | Numbers behind the evidence. |
| `source` | no | object | `tool`, `run`: one line each, up to 200 characters, both optional | The program and the run that made the entry. |
| `by` | no | string | one line, up to 40 characters | Who decided: `"chemist"` (every entry made on the page) or `"auto"`. |
| `note` | no | string | up to 2,000 characters, lines allowed, no other control character | Free text. |
| any other key | no | any JSON | | Kept with its value and written back unchanged. |

One item of `evidence`:

| Field | Required | Shape | Meaning |
|---|---|---|---|
| `kind` | yes | one word of 1 to 40 characters: ASCII letters, digits, `-`, `_`, `.` | The kind of evidence. Named kinds: `ms2-library`, `ms2-insilico`, `rt-standard`, `diagnostic-fragment`, `formula`, `exact-mass`, `context`; another word of the same shape is kept. |
| `ref` | no | one line, up to 200 characters | The reference: a library entry, a fragment, a standard. |
| `note` | no | one line, up to 200 characters | Free text. |

One item of `scores`:

| Field | Required | Shape | Meaning |
|---|---|---|---|
| `name` | yes | one line, up to 40 characters | The name of the score (`cosine`). |
| `v` | yes | a number | Its value. |
| `from` | no | one line, up to 200 characters | Where it comes from. |

One line means: no control character, no space at either end, not empty. An annotation that is not of this shape is refused
with the key `error.draw2.ann` and changes nothing.

## The levels of the scale (Schymanski et al., Environ. Sci. Technol. 2014, 48, 2097)

| `level` | Meaning |
|---|---|
| `"1"` | Confirmed structure (reference standard) |
| `"2a"` | Probable structure, by library spectrum match |
| `"2b"` | Probable structure, by diagnostic evidence |
| `"3"` | Tentative candidate(s) |
| `"4"` | Unequivocal molecular formula |
| `"5"` | Exact mass of interest |

## The scales the page offers

One table in Rust (`ann::SCALES`, given to the page by `ann_schema`) lists the scales and their levels; the meaning of each level is
an i18n text of the page, `draw2.ann.level.<scale>.<level>`, a short paraphrase in our own words. The page offers a scale before
the level; the level list follows the scale and the entry is written with that `scale`. Validation does not change: any word of the
level shape is kept. A level that is not in the chosen scale stays allowed and is shown as "of the file". The picture is unchanged
(`L` and the level: `L3b`, `L5a`).

| `scale` | Levels (most confident first) |
|---|---|
| `schymanski-2014` (default) | `1`, `2a`, `2b`, `3`, `4`, `5` |
| `charbonnet-2022` (PFAS; Charbonnet et al., Environ. Sci. Technol. Lett. 2022, 9, 473) | `1a`, `1b`, `2a`, `2b`, `2c`, `3a`, `3b`, `3c`, `3d`, `4`, `5a`, `5b` |

The level meanings for the PFAS scale are paraphrased; the Table 1 of that paper is an image that was not read here, so they are to be
checked against it before a release.

## Where it is exported

| Format | What is written | Read back |
|---|---|---|
| `mzdraw/1` (and the SVG and PNG that carry it) | the whole annotation, on its atom | yes, whole |
| SVG, PNG (the picture) | `L` and the current level (`L2b`) centred under the structure | (from the carried document) |
| SDF | after the data fields of the record, four data fields: `MZLAB_LEVEL` (current level), `MZLAB_IDENTIFIER` (current identifier, if any), `MZLAB_EVIDENCE` (current evidence, one line each: `kind: ref`, or `kind`), `MZLAB_ANN` (the whole annotation as one line of JSON) | yes: `MZLAB_ANN` when it is an annotation (whole history); else one entry from `MZLAB_LEVEL`, `MZLAB_IDENTIFIER`, `MZLAB_EVIDENCE`. Fields that give no annotation are counted in the import report (`draw2.import.dataLeft`), never dropped silently. The four names cannot be data fields of the user. |
| Command log | the `atom.ann` operations | yes |
| molfile, SMILES, KET, CDXML, CML, RXN | not written: these writers have no place for it in this version | no |

## On the page and from the CLI route

- Page: "Annotation of the structures" has, for each structure, the scale, the level (those of the chosen scale, plus the level of the file
  when it is not in it), the identifier, the evidence (one per line, `kind: reference`) and a note. Apply adds an
  entry after the ones the structure has (`Editor::annotate`, one undo step, `by` = `chemist`); "Not annotated" takes the
  annotation away. The list "What is drawn" says the level.
- Headless (`replay`, the route of `mzlab-cli mol draw`): the `atom.ann` operation in the `log` of a request; `out: "sdf"`,
  `"svg"` or `"mzdraw"` gives the same bytes as the page (`tests/parity.mjs`).

## Not built

Scores, source and several-item notes cannot be typed on the page (they are kept when a file or a log brings them). The
caption line of the study (evidence kinds, best score under the structure) and a style section for the badge are not built.
Annotations are not written into KET, CDXML or CML.
