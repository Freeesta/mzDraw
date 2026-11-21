// The palettes of the page: every group of actions is a small palette that is docked on an edge of the page or floats over it
// (docs/mzdraw-ui.md §4). Where each palette is, its orientation, its order, snapping, docking, the keyboard moves and the presets
// are decided in Rust (crates/mzdraw/src/palettes.rs, `palettes` and `palette` of the module). This file builds the palettes,
// measures them, forwards the pointer and key events on their handles, and paints the answer; it decides nothing.

import { icon } from "./icons.js";
import { NAMES } from "./names.js";

export const PALETTES = 128;
const P = NAMES.id;
/** What each palette holds. `keep`: always in view; `items`: in view when the palette is expanded, in its flyout when it is
 * collapsed (then its head stands for them); `tool`: the head shows the kind last used; `owns`: the tools whose name starts so
 * count as the palette's (the head is pressed); `fields`: typed entries, always in the flyout; `more`: the icon of the head of an
 * expanded palette that has fields; `head`: the icon of the head of a collapsed palette that is not a tool palette. The six side
 * palettes are written in index.html. */
const SPEC = /** @type {Record<string, any>} */ ({
  tools: { items: ["select", "erase"], tool: true },
  bonds: { items: ["bond:1", "bond:2", "bond:3", "bond:up", "bond:down", "bond:wavy"], tool: true },
  rings: { items: ["ring:6a", "ring:3", "ring:4", "ring:5", "ring:6", "ring:7", "ring:8"], tool: true },
  atoms: { keep: ["atom:C", "atom:N", "atom:O"], items: ["atom:S", "atom:P", "atom:F", "atom:Cl", "atom:Br", "atom:I", "atom:H", "atom:B", "atom:Si"], head: "elements", more: "elements", owns: "atom:", fields: ["atom"], grid: true },
  charges: { items: ["charge:+", "charge:-"], tool: true, owns: "label:", fields: ["label"], more: "label" },
  objects: { items: ["arrow", "plus"], tool: true, owns: "text:", fields: ["text"], more: "text" },
  file: { items: ["open", "save", "saveas", "clear", "palettes"], head: "open" },
  history: { items: ["undo", "redo"], head: "undo" },
  edit: { items: ["cut", "copy", "paste", "duplicate", "all", "rotate-l", "rotate-r", "flip-h", "flip-v"], head: "more" },
  view: { items: ["zoom-out", "zoom-in", "fit"], head: "fit" },
});
/** the buttons that open a flyout of their own, and the icon of each */
const OPENERS = { saveas: "formats", palettes: "palettes" };

/** @type {any} */ let ctx = null;
/** the palettes by id: element, its parts, and what it holds @type {Record<string, any>} */
const pals = {};
/** the flyouts by name (a palette id, `saveas`, `palettes`): the button that opens it, the flyout, the palette it belongs to @type {Record<string, {btn: HTMLElement, fly: HTMLElement, pal: string}>} */
const flys = {};
/** the last layout the module gave @type {any} */ let last = null;
/** the palette held by the pointer @type {{id: string, moved: boolean} | null} */ let held = null;
const $ = (/** @type {string} */ id) => /** @type {HTMLElement} */ (document.getElementById(id));

/** the name of a command in the i18n keys @param {string} c */
export const keyOf = (/** @type {string} */ c) => c.replace(":", ".").replace("+", "plus").replace("charge.-", "charge.minus").replace("zoom-in", "zoomIn").replace("zoom-out", "zoomOut");

/** the face of a button: the icon of the command, or an element symbol @param {string} c */
function face(c) {
  if (!c.startsWith("atom:")) return icon(c);
  const s = document.createElement("span");
  s.className = "sym"; s.dataset.el = c.slice(5); s.textContent = c.slice(5); s.setAttribute("aria-hidden", "true");
  return s;
}

/** one button of a palette or a flyout @param {string} c @param {string} pal */
function button(c, pal) {
  const b = document.createElement("button"), k = keyOf(c), t = ctx.t;
  b.type = "button"; b.dataset.testid = P + c;
  if (c in OPENERS) { b.dataset.opens = c; b.setAttribute("aria-expanded", "false"); b.setAttribute("aria-controls", P + "fly-" + c); }
  else b.dataset.cmd = c;
  b.append(c in OPENERS ? icon(OPENERS[/** @type {"saveas" | "palettes"} */ (c)]) : face(c));
  b.title = t(`draw2.tool.${k}.title`); b.setAttribute("aria-label", b.title);
  const s = document.createElement("span"); s.className = "txt"; s.textContent = c.startsWith("atom:") ? b.title.replace(/ \(.*$/, "") : t(`draw2.tool.${k}`); b.append(s);
  // a pointer click gives the keyboard back to the canvas (shortcuts keep working); a keyboard activation stays on the button
  b.addEventListener("click", e => {
    if (c in OPENERS) { open(c, !!flys[c].fly.hidden, !e.detail); return; }
    if (c === "open") $(P + "file").click();
    else if (c === "save") ctx.saveDefault();
    else ctx.run(c);
    const f = b.closest(".fly");
    if (f) { const name = /** @type {HTMLElement} */ (f).dataset.fly || ""; open(name, false); if (!e.detail) flys[name].btn.focus(); }
    if (e.detail && c !== "open") ctx.focusCanvas();
  });
  return b;
}

/** a flyout, kept in the one layer of flyouts of the page @param {string} name @param {HTMLElement} btn @param {string} pal @param {HTMLElement} [fly] */
function flyout(name, btn, pal, fly) {
  fly = fly || document.createElement("div");
  fly.id = P + "fly-" + name; fly.className = "fly"; fly.dataset.fly = name; fly.hidden = true;
  fly.setAttribute("role", "group"); fly.setAttribute("aria-label", ctx.t(`draw2.fly.${name}`));
  $(P + "flys").append(fly);
  flys[name] = { btn, fly, pal };
  fly.addEventListener("keydown", e => {
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); open(name, false); btn.focus(); return; }
    const t = /** @type {HTMLElement} */ (e.target), typing = t.tagName === "INPUT" && /** @type {HTMLInputElement} */ (t).type !== "checkbox";
    const step = { ArrowDown: 1, ArrowUp: -1, ...(typing ? {} : { ArrowRight: 1, ArrowLeft: -1 }) }[e.key];
    if (!step || t.tagName === "SELECT") return;
    e.preventDefault();
    const all = /** @type {HTMLElement[]} */ ([...fly.querySelectorAll("button:not(:disabled), input")]).filter(x => x.offsetParent), i = all.indexOf(t);
    all[(i + step + all.length) % all.length]?.focus();
  });
  fly.addEventListener("focusout", e => { const to = /** @type {Node | null} */ (e.relatedTarget); if (to && !fly.contains(to) && !btn.contains(to) && !inside(to, name)) open(name, false); });
  return fly;
}

/** the node is in a flyout opened from inside the flyout `name`, which then stays open */
const inside = (/** @type {Node} */ node, /** @type {string} */ name) => Object.values(flys).some(o => !o.fly.hidden && flys[name].fly.contains(o.btn) && o.fly.contains(node));

/** opens or closes a flyout beside its button, on the side away from the edge the palette is docked on; an opened one closes
 * the others but the one it sits in. `focus`: the keyboard goes to the kind in use, else the first item
 * @param {string} name @param {boolean} show @param {boolean} [focus] */
export function open(name, show, focus = false) {
  const f = flys[name];
  if (!f) return;
  if (show) for (const [n, o] of Object.entries(flys)) if (n !== name && !o.fly.hidden && !o.fly.contains(f.btn)) open(n, false);
  f.fly.hidden = !show; f.btn.setAttribute("aria-expanded", String(show));
  if (!show) return;
  const r = f.btn.getBoundingClientRect(), el = pals[f.pal].el, dock = el.dataset.dock, row = el.dataset.orient === "h", w = f.fly.offsetWidth, h = f.fly.offsetHeight;
  const [x, y] = dock === "right" ? [r.left - w - 6, r.top] : dock === "bottom" ? [r.left, r.top - h - 6] : row || f.btn.closest(".fly") ? [r.left, r.bottom + 6] : [r.right + 6, r.top];
  f.fly.style.left = Math.max(4, Math.min(x, innerWidth - w - 4)) + "px"; f.fly.style.top = Math.max(4, Math.min(y, innerHeight - h - 4)) + "px";
  if (focus) /** @type {HTMLElement} */ (f.fly.querySelector('[aria-pressed="true"]') || f.fly.querySelector("button:not(:disabled), input"))?.focus();
}

/** the handle of a palette: drag it with a mouse, a finger or a pen; on the keyboard the arrows move it, Enter docks or floats
 * it, Escape puts it back; a double click collapses or expands it. Every event goes to the module. @param {string} id */
function grip(id) {
  const g = document.createElement("button"), t = ctx.t, name = t(`draw2.pal.${id}`);
  g.type = "button"; g.className = "grip"; g.dataset.testid = P + "grip-" + id;
  g.title = t("draw2.pal.grip.title", { name }); g.setAttribute("aria-label", g.title);
  g.addEventListener("pointerdown", e => {
    if (e.button) return;
    e.preventDefault();
    try { g.setPointerCapture(e.pointerId); } catch { /* a pointer already gone cannot be captured */ }
    held = { id, moved: false }; g.focus({ preventScroll: true });
    ed().palette("press", id, e.clientX, e.clientY, false);
  });
  g.addEventListener("pointermove", e => { if (held?.id !== id) return; held.moved = true; after(ed().palette("drag", id, e.clientX, e.clientY, false), false); });
  g.addEventListener("pointerup", e => {
    if (held?.id !== id) return;
    const moved = held.moved; held = null;
    after(moved ? ed().palette("release", id, e.clientX, e.clientY, false) : ed().palette("cancel", id, 0, 0, false), moved);
  });
  g.addEventListener("pointercancel", () => { if (held?.id === id) { held = null; after(ed().palette("cancel", id, 0, 0, false), false); } });
  g.addEventListener("keydown", e => {
    const f = ed().palette("key:" + e.key, id, 0, 0, e.shiftKey);
    if (f) { e.preventDefault(); after(f, true); g.focus({ preventScroll: true }); ctx.say(t("draw2.pal.moved", { name })); }
  });
  g.addEventListener("dblclick", () => after(ed().palette("collapsed", id, 0, 0, false), true));
  return g;
}

const ed = () => ctx.ed;

/** acts on what the module answered to a palette event: lays the palettes out again, and keeps the settings when asked
 * @param {number} f @param {boolean} keep */
function after(f, keep) {
  if (!(f & PALETTES)) return;
  layout();
  if (keep) ctx.keep();
}

/** a palette of tools or commands @param {string} id */
function palette(id) {
  const spec = SPEC[id], el = document.createElement("div"), items = document.createElement("div"), head = document.createElement("button");
  el.className = "pal"; el.id = P + "pal-" + id; el.dataset.pal = id;
  el.setAttribute("role", "toolbar"); el.setAttribute("aria-label", ctx.t(`draw2.pal.${id}`));
  items.className = "items";
  head.type = "button"; head.className = "head"; head.dataset.testid = P + "head-" + id; head.setAttribute("aria-expanded", "false"); head.setAttribute("aria-controls", P + "fly-" + id);
  const keep = (spec.keep || []).map((/** @type {string} */ c) => button(c, id)), rest = spec.items.map((/** @type {string} */ c) => button(c, id));
  const fields = (spec.fields || []).flatMap((/** @type {string} */ k) => ctx.field(k));
  const p = { id, spec, el, items, head, keep, rest, fields, last: spec.tool ? spec.items[0] : "", collapsed: /** @type {boolean | null} */ (null),
    owns: (/** @type {string} */ c) => spec.items.includes(c) || (!!spec.owns && c.startsWith(spec.owns) && !(spec.keep || []).includes(c)) };
  pals[id] = p;
  const fly = flyout(id, head, id);
  if (spec.grid) fly.classList.add("grid");
  head.addEventListener("click", e => {
    const now = JSON.parse(ed().state()).tool;
    if (p.collapsed && spec.tool && now !== p.last && fly.hidden) { ctx.run(p.last); if (e.detail) ctx.focusCanvas(); }
    else open(id, fly.hidden, !e.detail);
  });
  // one tab stop: the arrow keys move along the palette, and across it they open the flyout of a head
  items.addEventListener("keydown", e => {
    const all = /** @type {HTMLElement[]} */ ([...items.children]).filter(b => !(/** @type {HTMLButtonElement} */ (b).disabled)), i = all.indexOf(/** @type {HTMLElement} */ (e.target));
    if (i < 0) return;
    const row = el.dataset.orient === "h", [next, prev] = row ? ["ArrowRight", "ArrowLeft"] : ["ArrowDown", "ArrowUp"];
    const across = row ? (el.dataset.dock === "bottom" ? "ArrowUp" : "ArrowDown") : (el.dataset.dock === "right" ? "ArrowLeft" : "ArrowRight");
    const to = e.key === next ? all[(i + 1) % all.length] : e.key === prev ? all[(i - 1 + all.length) % all.length] : e.key === "Home" ? all[0] : e.key === "End" ? all[all.length - 1] : null;
    const name = /** @type {HTMLElement} */ (e.target).dataset.opens || (e.target === head ? id : "");
    if (to) { e.preventDefault(); to.focus(); }
    else if (e.key === across && name) { e.preventDefault(); open(name, true, true); }
  });
  items.addEventListener("focusin", e => rove(p, /** @type {HTMLElement} */ (e.target)));
  el.append(grip(id), items);
  for (const [name] of Object.entries(OPENERS)) { const b = rest.find((/** @type {HTMLElement} */ x) => x.dataset.opens === name); if (b) flyout(name, b, id, $(P + "fly-" + name) || undefined); }
  return el;
}

/** one tab stop per palette: the item that had the focus last @param {any} p @param {HTMLElement} [to] */
function rove(p, to) {
  const all = /** @type {HTMLElement[]} */ ([...p.items.children]);
  if (!to || !all.includes(to)) to = all.find(b => b.getAttribute("tabindex") === "0") || all[0];
  for (const b of all) b.setAttribute("tabindex", b === to ? "0" : "-1");
}

/** puts the buttons of a palette where its state wants them: a collapsed palette shows what it keeps and its head, the rest
 * goes to the flyout; an expanded one shows everything, and a head for its fields @param {any} p @param {boolean} collapsed */
function fold(p, collapsed) {
  if (p.collapsed === collapsed) return;
  p.collapsed = collapsed;
  const fly = flys[p.id].fly, t = ctx.t, hasHead = collapsed || p.fields.length > 0;
  p.items.replaceChildren(...p.keep, ...(collapsed ? [] : p.rest), ...(hasHead ? [p.head] : []));
  fly.replaceChildren(...(collapsed ? p.rest : []), ...p.fields);
  p.head.replaceChildren(collapsed && p.spec.tool ? face(p.last) : icon(collapsed ? p.spec.head : p.spec.more));
  p.head.title = t(`draw2.pal.${p.id}.${collapsed ? "head" : "more"}`); p.head.setAttribute("aria-label", p.head.title);
  if (!hasHead) open(p.id, false);
  rove(p);
}

/** what the palettes show of the editor's state: the kind last used on the head, the head pressed when the tool is the palette's
 * @param {any} st */
export function show(st) {
  for (const p of Object.values(pals)) {
    if (!p.spec) continue;
    if (p.spec.tool && p.spec.items.includes(st.tool) && p.last !== st.tool) { p.last = st.tool; if (p.collapsed) p.head.replaceChildren(face(p.last)); }
    const mine = p.owns(st.tool) && (p.collapsed || !p.spec.items.includes(st.tool));
    p.head.setAttribute("aria-pressed", String(mine));
  }
}

/** the size of every palette as it is drawn now, for the module @returns {[string, number, number][]} */
function measure() {
  return Object.values(pals).filter(p => !p.el.hidden).map(p => { const r = p.el.getBoundingClientRect(); return [p.id, Math.ceil(r.width), Math.ceil(r.height)]; });
}

/** lays the palettes out: the module says where each one is and how it is drawn; the page draws it so, measures it and asks
 * again with the sizes; then the canvas takes the free area */
export function layout() {
  const w = innerWidth, h = innerHeight;
  let a = JSON.parse(ed().palettes(w, h, ""));
  for (const r of a.palettes) shape(r);
  a = JSON.parse(ed().palettes(w, h, JSON.stringify(measure())));
  last = a;
  for (const r of a.palettes) { shape(r); const el = pals[r.id]?.el; if (el) { el.style.left = r.x + "px"; el.style.top = r.y + "px"; } }
  // the edge a held palette is flush with: it docks there when it is let go
  const p = held && a.palettes.find((/** @type {any} */ r) => r.id === held?.id), hint = $(P + "dock-hint");
  const edge = !p || p.dock !== "float" ? "" : p.x <= 0 ? "left" : p.y <= 0 ? "top" : p.x + p.w >= w ? "right" : p.y + p.h >= h ? "bottom" : "";
  hint.hidden = !edge; hint.dataset.edge = edge;
  for (const q of Object.values(pals)) q.el.classList.toggle("moving", q.id === held?.id);
  $(P + "show-all").hidden = !a.all_hidden;
  menuState(a);
  for (const [n, f] of Object.entries(flys)) if (!f.fly.hidden) open(n, true);
  ctx.area(a.area);
}

/** draws one palette as the module says: shown or hidden, its edge, row or column, collapsed or expanded @param {any} r */
function shape(r) {
  const p = pals[r.id];
  if (!p) return;
  p.el.hidden = r.hidden; p.el.dataset.dock = r.dock; p.el.dataset.orient = r.horizontal ? "h" : "v";
  p.el.setAttribute("aria-orientation", r.horizontal ? "horizontal" : "vertical");
  if (p.spec) fold(p, r.collapsed);
  else if (p.body.open === r.collapsed) p.body.open = !r.collapsed;
}

/** a side palette of index.html: its handle, and its disclosure as the collapsed state of the module @param {HTMLElement} el */
function side(el) {
  const id = el.dataset.pal || "", body = /** @type {HTMLDetailsElement} */ (el.querySelector(":scope > details"));
  pals[id] = { id, el, body, spec: null };
  el.setAttribute("aria-label", ctx.t(`draw2.pal.${id}`));
  el.prepend(grip(id));
  body.addEventListener("toggle", () => {
    const r = last?.palettes.find((/** @type {any} */ x) => x.id === id);
    if (r && r.collapsed === body.open) after(ed().palette("collapsed", id, 0, 0, false), true);
  });
}

/** the Palettes menu: each palette shown or hidden, collapsed or expanded, a floating one as a row or a column; Reset layout;
 * every palette hidden at once */
function menu() {
  const fly = flys.palettes.fly, t = ctx.t, grid = document.createElement("div");
  grid.className = "menu";
  for (const k of ["name", "shown", "collapsed", "row"]) { const s = document.createElement("span"); s.className = "mh"; s.textContent = t(`draw2.menu.${k}`); grid.append(s); }
  for (const r of JSON.parse(ed().palettes(innerWidth, innerHeight, "")).palettes) {
    const name = document.createElement("span"); name.textContent = t(`draw2.pal.${r.id}`); grid.append(name);
    for (const act of ["hidden", "collapsed", "orient"]) {
      const c = document.createElement("input");
      c.type = "checkbox"; c.dataset.act = act; c.dataset.pal = r.id; c.dataset.testid = `${P}menu-${act}-${r.id}`;
      c.setAttribute("aria-label", t(`draw2.menu.${act}.label`, { name: t(`draw2.pal.${r.id}`) }));
      c.addEventListener("change", () => after(ed().palette(act, r.id, 0, 0, false), true));
      grid.append(c);
    }
  }
  const reset = document.createElement("button"), all = document.createElement("button"), row = document.createElement("div");
  for (const [b, k, act] of [[reset, "reset", "reset"], [all, "all", "all"]]) {
    const btn = /** @type {HTMLButtonElement} */ (b);
    btn.type = "button"; btn.dataset.testid = `${P}menu-${k}`; btn.textContent = t(`draw2.menu.${k}`); btn.title = t(`draw2.menu.${k}.title`);
    btn.addEventListener("click", () => { open("palettes", false); after(ed().palette(/** @type {string} */ (act), "", 0, 0, false), true); ctx.say(t(`draw2.menu.${k}.done`)); });
  }
  row.className = "row"; row.append(reset, all);
  fly.append(grid, row);
}

/** the boxes of the menu say what the module says @param {any} a */
function menuState(a) {
  for (const c of /** @type {NodeListOf<HTMLInputElement>} */ (document.querySelectorAll(".menu input"))) {
    const r = a.palettes.find((/** @type {any} */ x) => x.id === c.dataset.pal);
    if (!r) continue;
    c.checked = c.dataset.act === "hidden" ? !r.hidden : c.dataset.act === "collapsed" ? r.collapsed : r.horizontal;
    c.disabled = c.dataset.act === "orient" && r.dock !== "float";
  }
}

/** builds every palette and its flyouts. `c`: the page's side of things: `ed` (the module), `t`, `run(command)`, `field(kind)`,
 * `focusCanvas()`, `saveDefault()`, `keep()` (store the settings), `area([x, y, w, h])` (the canvas's place), `say(text)`
 * @param {any} c */
export function build(c) {
  ctx = c;
  const box = $(P + "palettes");
  for (const id of Object.keys(SPEC)) box.append(palette(id));
  for (const el of /** @type {NodeListOf<HTMLElement>} */ (box.querySelectorAll(".pal.side"))) side(el);
  menu();
  document.addEventListener("pointerdown", e => {
    const n = /** @type {Node} */ (e.target);
    for (const [name, f] of Object.entries(flys)) if (!f.fly.hidden && !f.fly.contains(n) && !f.btn.contains(n) && !inside(n, name)) open(name, false);
  });
  // a palette whose content changes size (a list that grows, a section opened) is laid out again, once per frame
  let pending = 0;
  const ro = new ResizeObserver(() => { if (!pending) pending = requestAnimationFrame(() => { pending = 0; layout(); }); });
  for (const p of Object.values(pals)) ro.observe(p.el);
  $(P + "show-all").addEventListener("click", () => { after(ed().palette("all", "", 0, 0, false), true); });
}
