// GENERATED from brand.toml by crates/mzdraw/tests/brand.rs (MZDRAW_BLESS=1 cargo test -p mzdraw --test brand): do not edit.
// Every name of the product the page uses, in one place: the name will change. The ids of the page carry the prefix `d2-`
// (HTML, CSS and tests), the i18n keys `draw2.`; what the scripts write or store is named here only. The `*Old` lists are the
// names of earlier products: read when the current one is not there, never written.
export const NAMES = Object.freeze({
  name: "mzDraw",                             // the visible name (the texts say {app}, see appname.js)
  slug: "mzdraw",                             // the `out` kind of the document
  version: "mzdraw/1",                           // the document format of the day
  bridge: "draw-bridge/1",                         // what the module must speak to this page
  global: "MZDRAW",                            // the editor module on window, for hosts and the tests
  tag: "mzdraw-editor",                               // the custom element
  wasm: "mzdraw_wasm",                             // the module of pkg/
  autosave: "mzdraw-autosave.json",           // the drawing kept in the browser's private file system (OPFS)
  autosaveOld: [],
  settings: "mzdraw-settings.json",           // the settings file kept there, and its download name
  settingsOld: [],
  clip: "application/x-mzdraw+json",          // our own clipboard type: the document
  clipOld: [],
  doc: "drawing.mzdraw.json",                 // the download name of the document
  id: "d2-",                                  // the prefix of the ids the scripts make
  tokens: "--mzd-",                           // the prefix of the colour tokens (web/mzdraw.css), which a host may set
});
