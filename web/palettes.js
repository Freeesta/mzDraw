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
 * palettes are written in index.html. The tools palette is not here: it is one general palette whose places and groups the module
 * answers (`slots` of `palettes`, `tool_groups.rs`), see `general`. */
const SPEC = /** @type {Record<string, any>} */ ({
  tools: { general: true },
  file: { items: ["open", "save", "saveas", "clear", "prefs", "palettes"], head: "open" },
  history: { items: ["undo", "redo"], head: "undo" },
  edit: { items: ["cut", "copy", "paste", "duplicate", "all", "rotate-l", "rotate-r", "flip-h", "flip-v", "colour", "arrange", "scheme-orient"], head: "more" },
  view: { items: ["zoom-out", "zoom-in", "fit"], head: "fit" },
});
/** the buttons that open a flyout of their own, and the icon of each */
const OPENERS = { saveas: "formats", palettes: "palettes", prefs: "gear", colour: "colour", arrange: "align:left" };
/** what the flyout of Arrange holds: the buttons, then the typed angle */
const ARRANGE = ["align:left", "align:centre", "align:right", "align:top", "align:middle", "align:bottom", "distribute:h", "distribute:v"];

/** @type {any} */ let ctx = null;
/** the palettes by id: element, its parts, and what it holds @type {Record<string, any>} */
const pals = {};
/** the flyouts by name (a palette id, `saveas`, `palettes`): the button that opens it, the flyout, the palette it belongs to @type {Record<string, {btn: HTMLElement, fly: HTMLElement, pal: string}>} */
const flys = {};
/** the last layout the module gave @type {any} */ let last = null;
/** the palette whose handle the pointer is on (the module decides whether that is a click or a drag) @type {string | null} */ let held = null;
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

/** one button of a palette or a flyout; a clone is the second button of a tool that is in view on its own, listed in its group too
 * (it has no test id: the one in view keeps it) @param {string} c @param {string} pal @param {boolean} [clone] */
function button(c, pal, clone = false) {
  const b = document.createElement("button"), k = keyOf(c), t = ctx.t;
  b.type = "button"; if (!clone) b.dataset.testid = P + c;
  if (c in OPENERS) { b.dataset.opens = c; b.setAttribute("aria-expanded", "false"); b.setAttribute("aria-controls", P + "fly-" + c); }
  else b.dataset.cmd = c;
  b.append(c in OPENERS ? icon(OPENERS[/** @type {"saveas" | "palettes" | "prefs" | "colour" | "arrange"} */ (c)]) : face(c));
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
  const pr = el.getBoundingClientRect();       // beside the whole palette, which may be two columns wide
  const [x, y] = dock === "right" ? [pr.left - w - 6, r.top] : dock === "bottom" ? [r.left, r.top - h - 6] : row || f.btn.closest(".fly") ? [r.left, r.bottom + 6] : [pr.right + 6, r.top];
  f.fly.style.left = Math.max(4, Math.min(x, innerWidth - w - 4)) + "px"; f.fly.style.top = Math.max(4, Math.min(y, innerHeight - h - 4)) + "px";
  if (focus) /** @type {HTMLElement} */ (f.fly.querySelector('[aria-pressed="true"]') || f.fly.querySelector("button:not(:disabled), input"))?.focus();
}

/** the pointer on a handle of a palette (its frame, or the title of a side palette): a mouse, a finger or a pen drags it. Every event goes
 * to the module, which applies the drag threshold (a click with a little jitter moves nothing) and answers whether the layout changed;
 * the click that ends a real drag is taken off, so the title that moved a panel does not also open it
 * @param {HTMLElement} h @param {string} id @param {boolean} grab the handle takes the focus and the pointer (the frame); the title keeps its own click */
function pointer(h, id, grab) {
  let moved = false;
  h.addEventListener("pointerdown", e => {
    if (e.button) return;
    if (grab) { e.preventDefault(); h.focus({ preventScroll: true }); }
    try { h.setPointerCapture(e.pointerId); } catch { /* a pointer already gone cannot be captured */ }
    held = id; moved = false;
    ed().palette("press", id, e.clientX, e.clientY, false);
  });
  h.addEventListener("pointermove", e => { if (held === id) { const f = ed().palette("drag", id, e.clientX, e.clientY, false); if (f & PALETTES) moved = true; after(f, false); } });
  h.addEventListener("pointerup", e => { if (held === id) { held = null; after(ed().palette("release", id, e.clientX, e.clientY, false), true); } });
  h.addEventListener("pointercancel", () => { if (held === id) { held = null; after(ed().palette("cancel", id, 0, 0, false), true); } });
  if (!grab) h.addEventListener("click", e => { if (moved) { e.preventDefault(); e.stopPropagation(); } moved = false; }, true);
}

/** the handle of a palette: its frame, a transparent layer behind the buttons that takes the room of no row of dots; drag it with a mouse,
 * a finger or a pen; on the keyboard the arrows move it, Enter docks or floats it, Escape puts it back; a double click collapses or
 * expands it. Every event goes to the module. @param {string} id */
function grip(id) {
  const g = document.createElement("button"), t = ctx.t, name = t(`draw2.pal.${id}`);
  g.type = "button"; g.className = "grip"; g.dataset.testid = P + "grip-" + id;
  g.title = t("draw2.pal.grip.title", { name }); g.setAttribute("aria-label", t("draw2.pal.grip.name", { name }));
  g.setAttribute("aria-description", t("draw2.pal.grip.keys"));
  pointer(g, id, true);
  g.addEventListener("keydown", e => {
    const f = ed().palette("key:" + e.key, id, 0, 0, e.shiftKey);
    if (f) { e.preventDefault(); after(f, true); g.focus({ preventScroll: true }); ctx.say(t("draw2.pal.moved", { name })); }
  });
  g.addEventListener("dblclick", () => after(ed().palette("collapsed", id, 0, 0, false), true));
  return g;
}

/** the keyboard way to move a palette from anywhere inside it: Alt with an arrow moves it (8 pixels, 32 with Shift), Alt+Enter docks or
 * floats it, Alt+Escape puts it back; the module judges every key (the same ones as on the frame) @param {HTMLElement} el @param {string} id */
function altKeys(el, id) {
  el.addEventListener("keydown", e => {
    if (!e.altKey || e.ctrlKey || e.metaKey || !/^(Arrow(Left|Right|Up|Down)|Enter|Escape)$/.test(e.key)) return;
    const f = ed().palette("key:" + e.key, id, 0, 0, e.shiftKey);
    if (!f) return;
    e.preventDefault(); e.stopPropagation(); after(f, true);
    ctx.say(ctx.t("draw2.pal.moved", { name: ctx.t(`draw2.pal.${id}`) }));
  }, true);
  // the end of a keyboard move: the focus left the palette; the module lets it go where it is (beside what it lies over)
  el.addEventListener("focusout", e => { if (!el.contains(/** @type {Node | null} */ (e.relatedTarget))) after(ed().palette("settle", id, 0, 0, false), true); });
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
  if (SPEC[id].general) return general(id);
  const spec = SPEC[id], el = document.createElement("div"), items = document.createElement("div"), head = document.createElement("button");
  el.className = "pal"; el.id = P + "pal-" + id; el.dataset.pal = id;
  el.setAttribute("role", "toolbar"); el.setAttribute("aria-label", ctx.t(`draw2.pal.${id}`));
  items.className = "items";
  head.type = "button"; head.className = "head"; head.dataset.testid = P + "head-" + id; head.setAttribute("aria-expanded", "false"); head.setAttribute("aria-controls", P + "fly-" + id);
  const keep = (spec.keep || []).map((/** @type {string} */ c) => button(c, id)), rest = spec.items.map((/** @type {string} */ c) => button(c, id));
  const fields = (spec.fields || []).flatMap((/** @type {string} */ k) => ctx.field(k));
  const p = { id, spec, el, items, head, keep, rest, fields, last: spec.tool ? (spec.keep || [])[0] || spec.items[0] : "", collapsed: /** @type {boolean | null} */ (null),
    owns: (/** @type {string} */ c) => spec.items.includes(c) || (!!spec.owns && c.startsWith(spec.owns) && !(spec.keep || []).includes(c)) };
  pals[id] = p;
  const fly = flyout(id, head, id);
  if (spec.grid) fly.classList.add("grid");
  // a click on the head of a collapsed tool palette runs the kind last used (what the head shows); another click, or any other
  // palette, opens the flyout. This is the page's reading of its own face, not a decision about the drawing: the tool is the module's.
  head.addEventListener("click", e => {
    const now = JSON.parse(ed().state()).tool;
    if (p.collapsed && spec.tool && now !== p.last && fly.hidden) { ctx.run(p.last); if (e.detail) ctx.focusCanvas(); }
    else open(id, fly.hidden, !e.detail);
  });
  // one tab stop: the arrow keys move along the palette, and across it they open the flyout of a head
  items.addEventListener("keydown", e => {
    const all = /** @type {HTMLElement[]} */ ([...items.children]).filter(b => !(/** @type {HTMLButtonElement} */ (b).disabled)), i = all.indexOf(/** @type {HTMLElement} */ (e.target));
    if (i < 0) return;
    const { next, prev, across } = el.dataset;          // the keys of this palette, as the module says
    const to = e.key === next ? all[(i + 1) % all.length] : e.key === prev ? all[(i - 1 + all.length) % all.length] : e.key === "Home" ? all[0] : e.key === "End" ? all[all.length - 1] : null;
    const name = /** @type {HTMLElement} */ (e.target).dataset.opens || (e.target === head ? id : "");
    if (to) { e.preventDefault(); to.focus(); }
    else if (e.key === across && name) { e.preventDefault(); open(name, true, true); }
  });
  items.addEventListener("focusin", e => rove(p, /** @type {HTMLElement} */ (e.target)));
  el.append(grip(id), items); altKeys(el, id);
  for (const [name] of Object.entries(OPENERS)) { const b = rest.find((/** @type {HTMLElement} */ x) => x.dataset.opens === name); if (b) flyout(name, b, id, $(P + "fly-" + name) || undefined); }
  const arrange = flys.arrange?.fly;
  if (id === "edit" && arrange) arrange.append(...ARRANGE.map(c => button(c, id)), ...ctx.field("rotate"));
  return el;
}

/** the general tools palette: the tools used most, each on its own, and one button per group of tools (bonds, rings, more elements,
 * charges, reaction objects, text). The module says what is in it, in which order and which member stands for each group (`slots`);
 * a click on a group selects the member shown and opens the list of every member; the member picked from the list is the one shown
 * from then on. @param {string} id */
function general(id) {
  const el = document.createElement("div"), items = document.createElement("div"), a = JSON.parse(ed().palettes(innerWidth, innerHeight, ""));
  el.className = "pal"; el.id = P + "pal-" + id; el.dataset.pal = id;
  el.setAttribute("role", "toolbar"); el.setAttribute("aria-label", ctx.t(`draw2.pal.${id}`));
  items.className = "items general"; items.style.setProperty("--rows", a.rows);
  const p = { id, el, items, spec: null, general: true, groups: /** @type {Record<string, any>} */ ({}), collapsed: false };
  pals[id] = p;
  for (const s of a.slots) {
    if (s.cmd) { items.append(button(s.cmd, id)); continue; }
    const head = document.createElement("button"), fly = flyout(s.group, head, id), name = s.group, g = { ...s, head, fly };
    head.type = "button"; head.className = "head"; head.dataset.testid = P + "head-" + name; head.dataset.head = name;
    head.setAttribute("aria-expanded", "false"); head.setAttribute("aria-controls", P + "fly-" + name);
    head.setAttribute("aria-haspopup", "true");          // the list; the group in use is `aria-current`, not a pressed state beside an expanded one
    // the list holds every member (a kind in view on its own is listed again, without its test id) and the typed entries
    fly.append(...s.members.map((/** @type {string} */ c) => button(c, id, s.keep.includes(c))), ...s.fields.flatMap((/** @type {string} */ k) => ctx.field(k)));
    if (s.grid) fly.classList.add("grid");
    // a click on a closed list asks the module to select the member shown (it decides whether that is the tool already, and whether the
    // group has a member to show) and opens the list; a click on a list that is open shuts it
    head.addEventListener("click", e => {
      if (fly.hidden) ctx.run("group:" + name);
      open(name, fly.hidden, !e.detail);
    });
    // Escape on the button of a list that a pointer opened shuts it (the keys of a list that is entered by keyboard are the list's own)
    head.addEventListener("keydown", e => { if (e.key === "Escape" && !fly.hidden) { e.preventDefault(); e.stopPropagation(); open(name, false); } });
    p.groups[name] = g; items.append(head);
  }
  // one tab stop: the arrow keys move along the palette (column after column), and across it they open the list of a group
  items.addEventListener("keydown", e => {
    const all = /** @type {HTMLElement[]} */ ([...items.children]).filter(b => !(/** @type {HTMLButtonElement} */ (b).disabled)), i = all.indexOf(/** @type {HTMLElement} */ (e.target));
    if (i < 0) return;
    const { next, prev, across } = el.dataset;          // the keys of this palette, as the module says
    const to = e.key === next ? all[(i + 1) % all.length] : e.key === prev ? all[(i - 1 + all.length) % all.length] : e.key === "Home" ? all[0] : e.key === "End" ? all[all.length - 1] : null;
    const name = /** @type {HTMLElement} */ (e.target).dataset.head || "";
    if (to) { e.preventDefault(); to.focus(); }
    else if (e.key === across && name) { e.preventDefault(); open(name, true, true); }
  });
  items.addEventListener("focusin", e => rove(p, /** @type {HTMLElement} */ (e.target)));
  el.append(grip(id), items); altKeys(el, id);
  faces(a.slots); rove(p);
  return el;
}

/** the face of each group button as the module says: the member used last, or the icon of the group @param {any[]} slots */
function faces(slots) {
  const p = pals.tools;
  for (const s of slots) {
    const g = s.group && p?.groups[s.group];
    if (!g || g.face === s.face && g.head.firstChild) continue;
    g.face = s.face;
    g.head.replaceChildren(s.face ? face(s.face) : icon(s.icon));
    // the button names the group and its face: "Bonds: Single bond", then what it does
    const t = ctx.t, label = t(`draw2.fly.${s.group}`), what = s.face ? `${label}: ${t(`draw2.tool.${keyOf(s.face)}`)}` : label;
    g.head.setAttribute("aria-label", what); g.head.title = `${what} ${t(`draw2.pal.${s.group}.head`)}`;
  }
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
    // a group button is pressed when the tool is a member that is not in view on its own, or one the group owns (a typed element, a label, a text)
    if (p.groups) { const on = JSON.parse(ed().pressed_groups()); for (const [n, g] of Object.entries(p.groups)) { if (on.includes(n)) g.head.setAttribute("aria-current", "true"); else g.head.removeAttribute("aria-current"); } }
    if (!p.spec) continue;
    // the head shows, and runs, the kind last used, whether that kind is kept in view or in the flyout
    if (p.spec.tool && [...(p.spec.keep || []), ...p.spec.items].includes(st.tool) && p.last !== st.tool) { p.last = st.tool; if (p.collapsed) p.head.replaceChildren(face(p.last)); }
    const mine = p.owns(st.tool) && (p.collapsed || !p.spec.items.includes(st.tool));
    p.head.setAttribute("aria-pressed", String(mine));
  }
}

/** the size of every palette as it is drawn now, for the module @returns {[string, number, number][]} */
function measure() {
  // a side palette is measured as it would be uncut: the module cuts it again, or not, with the room the layout leaves
  return Object.values(pals).filter(p => !p.el.hidden).map(p => { p.el.style.maxHeight = ""; const r = p.el.getBoundingClientRect(); return [p.id, Math.ceil(r.width), Math.ceil(r.height)]; });
}

/** lays the palettes out: the module says where each one is and how it is drawn; the page draws it so, measures it and asks
 * again with the sizes; then the canvas takes the free area */
export function layout() {
  const w = innerWidth, h = innerHeight;
  let a = JSON.parse(ed().palettes(w, h, ""));
  for (const r of a.palettes) shape(r);
  a = JSON.parse(ed().palettes(w, h, JSON.stringify(measure())));
  last = a;
  faces(a.slots);
  for (const r of a.palettes) { shape(r); const el = pals[r.id]?.el; if (el) { el.style.left = r.x + "px"; el.style.top = r.y + "px"; } }
  // the module says which palette is being dragged and the edge it docks on if it is let go now; the page paints both
  const hint = $(P + "dock-hint");
  hint.hidden = !a.hint_edge; hint.dataset.edge = a.hint_edge;
  for (const q of Object.values(pals)) q.el.classList.toggle("moving", q.id === a.held);
  $(P + "show-all").hidden = !a.all_hidden;
  menuState(a);
  for (const [n, f] of Object.entries(flys)) if (!f.fly.hidden) open(n, true);
  ctx.area(a.area);
}

/** draws one palette as the module says: shown or hidden, its edge, row or column, collapsed or expanded @param {any} r */
function shape(r) {
  const p = pals[r.id];
  if (!p) return;
  p.el.dataset.next = r.next; p.el.dataset.prev = r.prev; p.el.dataset.across = r.across;
  p.el.hidden = r.hidden; p.el.style.maxHeight = r.capped ? r.h + "px" : ""; p.el.dataset.dock = r.dock; p.el.dataset.orient = r.horizontal ? "h" : "v";
  p.el.setAttribute("aria-orientation", r.horizontal ? "horizontal" : "vertical");
  if (p.general) return;
  if (p.spec) fold(p, r.collapsed);
  else if (!p.pending && p.body.open === r.collapsed) p.body.open = !r.collapsed;
}

/** a side palette of index.html: its handle, and its disclosure as the collapsed state of the module @param {HTMLElement} el */
function side(el) {
  const id = el.dataset.pal || "", body = /** @type {HTMLDetailsElement} */ (el.querySelector(":scope > details"));
  const p = pals[id] = { id, el, body, spec: null, pending: false };
  el.setAttribute("aria-label", ctx.t(`draw2.pal.${id}`));
  el.prepend(grip(id)); altKeys(el, id);
  // the title row is a handle too: a drag from it moves the panel, a click opens or closes it, and its name says how to move it from the keyboard
  const title = /** @type {HTMLElement | null} */ (body.querySelector(":scope > summary"));
  if (title) { pointer(title, id, false); title.dataset.testid = P + "title-" + id; title.setAttribute("aria-description", ctx.t("draw2.pal.title.keys")); }
  // a click on the title opens or closes the disclosure at once, but the `toggle` event that tells the module comes later: until it has,
  // a layout that runs in between must not put the old state back (that closed a palette the user had just opened)
  body.querySelector(":scope > summary")?.addEventListener("click", () => { p.pending = true; });
  body.addEventListener("toggle", () => {
    p.pending = false;
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
    c.disabled = (c.dataset.act === "orient" && !r.can_turn) || (c.dataset.act === "collapsed" && !r.can_collapse);
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
