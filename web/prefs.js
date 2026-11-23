// The Preferences panel of the page, built from the schema of the module (`settings_schema().prefs`: every option with its group, its
// text key, its words or its bounds and its default). Every option is a setting written `<section>.<key>=<value>`; the module judges every
// value and holds every default, and a reset of a group is the module's (`reset_group`). The page paints the rows and forwards the changes.

/** the theme of the settings (`ui.theme`, `light` unless chosen otherwise) on the root of the page: the module says the value for `data-theme` (empty for `auto`, which leaves the
 * choice to the system or to the host that set it); the page applies it and keeps it for the head script `theme.js`, which runs before the first paint
 * @param {any} ed @param {() => void} [before] called just before a different theme goes on the root */
let ours = false;
export function applyTheme(ed, before = () => {}) {
  const attr = ed.theme_attr(), root = document.documentElement;
  // the canvas is copied while it still shows the old theme, so that its old picture can fade out over the new one
  if ((root.dataset.theme || "") !== attr) before();
  // a `data-theme` that a host set stays alone; one the page set (here, or `theme.js` before the first paint, which marks it) is taken off for `auto`
  if (attr) { root.dataset.theme = attr; ours = true; }
  else if (ours || root.dataset.d2Own === "1") { delete root.dataset.theme; ours = false; }
  delete root.dataset.d2Own;
  if (attr) root.dataset.d2Own = "1";
  try { localStorage.setItem("d2-theme", attr || "auto"); } catch { /* no storage */ }
}

/** builds the panel in `into` and returns `sync`, which shows what the module holds.
 * `c`: `ed` (the module), `t`, `change(fn, said)` (runs a change asked of the module, repaints, keeps the settings, and says a refusal),
 * `files(kind)`: nothing here, the settings file buttons are the style panel's
 * @param {HTMLElement} into @param {any} c @returns {() => void} */
export function buildPrefs(into, c) {
  const { ed, t } = c, schema = JSON.parse(ed.settings_schema());
  /** @type {Array<() => void>} */ const syncs = [];
  const note = document.createElement("p"); note.id = "d2-prefs-note"; note.setAttribute("role", "status");
  const hint = document.createElement("p"); hint.className = "hint"; hint.textContent = t("draw2.prefs.hint");
  const head = document.createElement("div"); head.className = "row";
  const close = document.createElement("button"); close.type = "button"; close.id = "d2-prefs-close"; close.textContent = t("draw2.prefs.close"); close.title = t("draw2.prefs.close.title");
  close.addEventListener("click", () => c.close());
  head.append(hint, close);
  into.append(head);
  for (const g of schema.groups) {
    const set = document.createElement("fieldset"), legend = document.createElement("legend");
    set.dataset.group = g.id; set.id = `d2-prefs-${g.id}`;
    legend.textContent = t(g.label); set.append(legend);
    for (const p of schema.prefs.filter((/** @type {any} */ x) => x.group === g.id)) set.append(row(p));
    if (g.id === "appearance") { const lang = document.createElement("p"); lang.className = "hint"; lang.textContent = t("draw2.prefs.language"); set.append(lang); }
    const reset = document.createElement("button");
    reset.type = "button"; reset.dataset.reset = g.id; reset.textContent = t("draw2.prefs.reset"); reset.title = t("draw2.prefs.reset.title");
    reset.addEventListener("click", () => c.change(() => ed.reset_group(g.id), t("draw2.prefs.reset.done", { group: t(g.label) })));
    set.append(reset);
    into.append(set);
  }
  into.append(note);

  /** one option @param {any} p */
  function row(p) {
    const label = document.createElement("label"), name = document.createElement("span");
    label.className = "field"; name.textContent = t(p.label);
    if (p.kind === "adducts") {
      const box = document.createElement("fieldset"), legend = document.createElement("legend");
      box.dataset.setting = p.setting; legend.textContent = t(p.label); box.append(legend);
      for (const a of p.values) {
        const l = document.createElement("label"), b = document.createElement("input");
        b.type = "checkbox"; b.value = a;
        b.addEventListener("change", () => c.change(() => ed.set(`${p.setting}=${[...box.querySelectorAll("input:checked")].map(x => /** @type {HTMLInputElement} */ (x).value).join(",")}`), t("draw2.prefs.changed", { name: t(p.label) })));
        l.append(b, " " + a); box.append(l);
      }
      syncs.push(() => { const on = ed.get(p.setting).split(","); for (const b of box.querySelectorAll("input")) /** @type {HTMLInputElement} */ (b).checked = on.includes(/** @type {HTMLInputElement} */ (b).value); });
      return box;
    }
    const input = document.createElement(p.kind === "words" ? "select" : "input");
    input.dataset.setting = p.setting; input.id = `d2-pref-${p.setting}`;
    if (p.kind === "words") for (const v of p.values) { const o = document.createElement("option"); o.value = v.value; o.textContent = t(v.label); input.append(o); }
    else { const n = /** @type {HTMLInputElement} */ (input); n.type = "number"; n.min = String(p.min); n.max = String(p.max); n.step = "any"; n.inputMode = "decimal"; }
    input.addEventListener("change", () => c.change(() => ed.set(`${p.setting}=${/** @type {HTMLInputElement} */ (input).value}`), t("draw2.prefs.changed", { name: t(p.label) }), input));
    label.append(name, input);
    if (p.kind === "number") label.append(t(`draw2.style.unit.${p.unit}`));
    syncs.push(() => { /** @type {HTMLInputElement} */ (input).value = ed.get(p.setting); input.removeAttribute("aria-invalid"); });
    return label;
  }
  return () => { for (const s of syncs) s(); };
}
