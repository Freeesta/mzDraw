// Runs in the head, before the first paint: puts the theme the page stored last time (`d2-theme` in localStorage, written by prefs.js from
// what the module said) on the root, so a stored dark theme does not flash light while the module loads. It applies the stored text and
// decides nothing (it marks `data-d2-own` so the page knows the attribute is its own and not a host's); without storage, or with nothing stored, the system or the host decides as before.
try { const v = localStorage.getItem("d2-theme"); if (v) { document.documentElement.dataset.theme = v; document.documentElement.dataset.d2Own = "1"; } } catch { /* no storage */ }
