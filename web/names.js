// GENERATED from brand.toml by crates/mzdraw/tests/brand.rs (MZDRAW_BLESS=1 cargo test -p mzdraw --test brand): do not edit.
// Every name of the product the page uses, in one place: the name will change. The ids of the page carry the prefix `d2-`
// (HTML, CSS and tests), the i18n keys `draw2.`; what the scripts write or store is named here only. The `*Old` lists are the
// names of earlier products: read when the current one is not there, never written. (The comments are above the lines, not after
// them, so that the file does not depend on the length of a name.)
export const NAMES = Object.freeze({
  // the visible name (the texts say {app}, see appname.js)
  name: "mzDraw",
  // the `out` kind of the document
  slug: "mzdraw",
  // the document format of the day
  version: "mzdraw/1",
  // what the module must speak to this page
  bridge: "draw-bridge/1",
  // the editor module on window, for hosts and the tests
  global: "MZDRAW",
  // the custom element
  tag: "mzdraw-editor",
  // the module of pkg/
  wasm: "mzdraw_wasm",
  // the drawing kept in the browser's private file system (OPFS), then the names of earlier products
  autosave: "mzdraw-autosave.json",
  autosaveOld: [],
  // the settings file kept there, and its download name
  settings: "mzdraw-settings.json",
  settingsOld: [],
  // our own clipboard type: the document
  clip: "application/x-mzdraw+json",
  clipOld: [],
  // the download name of the document
  doc: "drawing.mzdraw.json",
  // the prefix of the ids the scripts make
  id: "d2-",
  // the prefix of the colour tokens (web/mzdraw.css), which a host may set
  tokens: "--mzd-",
});
