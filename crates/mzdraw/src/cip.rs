//! CIP labels drawn in the picture (`display.cip`, rdkit-compare 6): R/S at a stereocentre, E/Z beside a double bond. The labels are
//! computed by the engine `cip` of mzLab, which this crate does not have: the editor is given a `Labeller` (a plain function pointer,
//! as the masses are given a `values::Measurer`) by the bridge crate (`mzdraw-wasm`, the glue). Without one, `display.cip=on` draws
//! nothing and a `draw-result/1` says so in `lost` (`draw2.export.cipEngine`): no label is ever made up here.

use crate::doc::Document;

/// One label: on an atom (`atom` its id, `bond` 0) or on a bond (`bond` its id, `atom` 0), `text` the descriptor as the engine
/// gives it (`R`, `S`, `r`, `s`, `E`, `Z`, ...). The picture writes it in brackets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CipLabel {
    pub atom: u32,
    pub bond: u32,
    pub text: String,
}

/// The labels of a drawing, by document ids; the error is an i18n key.
pub type Labeller = fn(&Document) -> Result<Vec<CipLabel>, &'static str>;

/// The key of the `lost` item of a `draw-result/1` when `display.cip` is on and no labeller is attached (the plain core replay).
pub const NO_ENGINE: &str = "draw2.export.cipEngine";

/// The labels in a stable order: atoms by id, then bonds by id (the engine's order is the molfile's, which follows the pool).
pub fn sorted(mut labels: Vec<CipLabel>) -> Vec<CipLabel> {
    labels.sort_by(|a, b| (a.bond != 0, a.atom, a.bond).cmp(&(b.bond != 0, b.atom, b.bond)));
    labels
}
