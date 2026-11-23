// Runs in the head, before the first paint: puts the theme on the root so that no theme flashes while the module loads. What the page stored
// last time (`d2-theme` in localStorage, written by prefs.js from what the module said) wins; a first visit has nothing stored and opens the
// light theme, whatever the system prefers; `auto` (the system or the host decides) is a stored word like the others and puts nothing on the root.
// It applies text and decides nothing (it marks `data-d2-own` so the page knows the attribute is its own and not a host's); without storage
// the light theme is the default too.
let v = "";
try { v = localStorage.getItem("d2-theme") || ""; } catch { /* no storage */ }
// only the words the module knows count (light, dark, auto): anything else stored is the default
if (v !== "light" && v !== "dark" && v !== "auto") v = "";
if (v !== "auto") { document.documentElement.dataset.theme = v || "light"; document.documentElement.dataset.d2Own = "1"; }
