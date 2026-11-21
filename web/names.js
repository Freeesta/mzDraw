// Every name of the product the page uses, in one place: the name will change. The ids of the page carry the prefix `d2-`
// (HTML, CSS and tests), the i18n keys `draw2.`; what the scripts write or store is named here only.
export const NAMES = Object.freeze({
  version: "mzdraw/1",                        // the document format the module must speak
  global: "MZDRAW",                           // the editor module on window, for hosts and the tests
  autosave: "mzdraw-autosave.json",           // the drawing kept in the browser's private file system (OPFS)
  settings: "mzdraw-settings.json",           // the settings file kept there, and its download name
  clip: "application/x-mzdraw+json",          // our own clipboard type: the document
  doc: "drawing.mzdraw.json",                 // the download name of the document
  id: "d2-",                                  // the prefix of the ids the scripts make
  tokens: "--mzd-",                           // the prefix of the colour tokens (web/mzdraw.css), which a host may set
});
