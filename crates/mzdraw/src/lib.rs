//! mzDraw: the core of a 2D molecule editor (docs/agenti/mzdraw.md). The document `mzdraw/1`, the edit operations with their exact
//! inverses (`ops`), the interactive editor (`editor`: tools, hit-testing, view), the display list every surface paints (`scene`) and
//! its SVG form (`svg`). It depends on `mzlab-mol` and serde only, so it can leave this workspace; the mzLab glue lives in `mzdraw-wasm`.

pub mod ann;
pub mod brand;
pub mod build;
pub mod cdx;
pub mod cdxml;
pub mod cml;
mod commands;
pub mod conform;
pub mod context;
pub mod diff;
pub mod digest;
pub mod doc;
pub mod editor;
mod editor_edit;
mod editor_keys;
mod editor_nudge;
mod editor_pathway;
mod editor_step;
mod editor_text;
mod editor_tools;
pub mod editor_touch;
pub mod feedback;
pub mod formats;
pub mod ink;
pub mod io;
pub mod ket;
pub mod label;
pub mod layout;
pub mod loss;
mod m;
pub mod maps;
pub mod ops;
pub mod orient;
pub mod outcome;
pub mod outline;
pub mod palettes;
pub mod pathway;
pub mod pdf;
pub mod png;
pub mod prefs;
pub mod proposal;
pub mod replay;
pub mod route;
pub mod runs;
pub mod rxn;
pub mod scene;
pub mod schema;
pub mod scheme;
pub mod settings;
pub mod steplabel;
pub mod stereo_guard;
pub mod svg;
pub mod textedit;
pub mod tp;
pub mod tp_list;
pub mod tptable;
pub mod trasf;
pub mod values;
pub mod xml;

pub use doc::{Atom, Bond, Document, Obj, ObjKind, FORMAT};
pub use editor::Editor;
pub use ops::{Axis, Op, OpError, Session};
pub use runs::Run;
pub use settings::Style;

/// The product name, from the one place that holds it (`brand`).
pub use brand::{NAME, VERSION};
