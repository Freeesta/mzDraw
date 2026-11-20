// mzDraw page: forwards pointer, wheel and key events to the editor in the mzdraw .wasm and paints the display list it returns.
// No chemistry and no decision here: what a click does is decided in Rust (crates/mzdraw).

import { icon } from "./icons.js";

const I = /** @type {any} */ (window).I18N, t = (k, p = {}) => I.t(k, p);
const $ = id => /** @type {HTMLElement} */ (document.getElementById(id));
const REPAINT = 1, CHANGED = 2, TOOL = 4, HANDLED = 8, ASK_LABEL = 16, ASK_TEXT = 32, FOCUS = 64;
// The toolbars, after the hierarchy of docs/mzdraw-ui.md: a tool or command name, "|" a separator, or a group (a head button and
// its flyout). "P:" and `p` mark the 12 primary buttons. A tool group's head shows the kind last used from it: a click on the head
// picks that kind, a click on the head of the tool in use (or the arrow key) opens the flyout.
const COLUMN = ["P:select", "P:erase", "|",
  { g: "bond", p: 1, tools: ["bond:1", "bond:2", "bond:3", "bond:up", "bond:down", "bond:wavy"] },
  { g: "ring", p: 1, tools: ["ring:6a", "ring:3", "ring:4", "ring:5", "ring:6", "ring:7", "ring:8"] }, "|",
  "P:atom:C", "P:atom:N", "P:atom:O",
  { g: "atom", p: 1, tools: ["atom:S", "atom:P", "atom:F", "atom:Cl", "atom:Br", "atom:I", "atom:H", "atom:B", "atom:Si"], fields: ["atom", "label"] }, "|",
  { g: "charge", tools: ["charge:+", "charge:-"] },
  { g: "obj", tools: ["arrow", "plus"], fields: ["text"] }];
const BAR = ["P:open", { g: "save", p: 1, adopt: "d2-save" }, "|", "P:undo", "P:redo", "|",
  { g: "edit", cmds: ["cut", "copy", "paste", "duplicate", "all", "rotate-l", "rotate-r", "flip-h", "flip-v", "clear"] }];
const VIEW = ["zoom-out", "zoom-in", "fit", "|", "panel"];
/** the groups by name: head, flyout, the tools it holds and the kind last used @type {Record<string, {g: string, head: HTMLButtonElement, fly: HTMLElement, tools: string[], last: string, owns: (c: string) => boolean}>} */
const groups = {};
const COLOURS = ["--d2-ink", "--d2-select", "--d2-hover", "--d2-warn", "--d2-preview", "--d2-n", "--d2-o", "--d2-s", "--d2-x", "--d2-p", "--d2-focus"];

/** @type {any} */ let ed = null;
const cv = /** @type {HTMLCanvasElement} */ ($("d2-canvas")), g = /** @type {CanvasRenderingContext2D} */ (cv.getContext("2d"));

async function load() {
  const mod = await import("./pkg/mzdraw_wasm.js");
  await mod.default();
  if (mod.version() !== "mzdraw/1") throw new Error("version");
  return mod;
}

function paint() {
  const d = ed.scene(), s = ed.strings(), css = getComputedStyle(document.documentElement), col = COLOURS.map(c => css.getPropertyValue(c).trim());
  const r = devicePixelRatio || 1;
  g.setTransform(r, 0, 0, r, 0, 0);
  g.clearRect(0, 0, cv.clientWidth, cv.clientHeight);
  g.lineCap = "round"; g.textBaseline = "middle";
  for (let i = 0; i < d.length;) {
    const op = d[i];
    if (op === 1) { g.strokeStyle = col[d[i + 6]]; g.lineWidth = d[i + 5]; g.beginPath(); g.moveTo(d[i + 1], d[i + 2]); g.lineTo(d[i + 3], d[i + 4]); g.stroke(); i += 7; }
    else if (op === 2) { g.fillStyle = col[d[i + 4]]; g.font = d[i + 3] + "px Arial, Helvetica, sans-serif"; g.textAlign = d[i + 6] ? "left" : "center"; g.fillText(s[d[i + 5]], d[i + 1], d[i + 2]); i += 7; }
    else if (op === 3) { g.strokeStyle = col[d[i + 5]]; g.lineWidth = 1; g.setLineDash([4, 3]); g.strokeRect(d[i + 1], d[i + 2], d[i + 3], d[i + 4]); g.setLineDash([]); i += 6; }
    else if (op === 6) { g.fillStyle = col[d[i + 7]]; g.beginPath(); g.moveTo(d[i + 1], d[i + 2]); g.lineTo(d[i + 3], d[i + 4]); g.lineTo(d[i + 5], d[i + 6]); g.closePath(); g.fill(); i += 8; }
    else { g.beginPath(); g.arc(d[i + 1], d[i + 2], d[i + 3], 0, 2 * Math.PI); if (op === 4) { g.fillStyle = col[d[i + 4]]; g.fill(); } else { g.strokeStyle = col[d[i + 4]]; g.lineWidth = 2; g.stroke(); } i += 5; }
  }
}

function size() {
  const r = devicePixelRatio || 1;
  cv.width = Math.round(cv.clientWidth * r); cv.height = Math.round(cv.clientHeight * r);
  paint();
}

/** the data of each structure (the data fields of an SDF record) as rows of name and value that can be edited, added and, by
 * emptying the name, taken away; "Apply" gives the rows of one structure to the module, which judges them. Not rebuilt while a
 * field of it is being typed in. @param {any[]} outline */
function dataFields(outline) {
  const box = $("d2-data"), active = document.activeElement;
  if (active && box.contains(active) && active.tagName !== "BUTTON") return;
  const row = (/** @type {string} */ k, /** @type {string} */ v) => {
    const r = document.createElement("div"), name = document.createElement("input"), value = document.createElement("textarea");
    r.className = "row d2-datum"; name.type = "text"; name.value = k; name.maxLength = 80; name.setAttribute("aria-label", t("draw2.data.name")); name.placeholder = t("draw2.data.name");
    value.rows = 1 + v.split("\n").length - 1; value.value = v; value.setAttribute("aria-label", t("draw2.data.value")); value.placeholder = t("draw2.data.value");
    r.append(name, value);
    return r;
  };
  box.replaceChildren(...outline.map((s, i) => {
    const set = document.createElement("fieldset"), legend = document.createElement("legend"), rows = document.createElement("div"), buttons = document.createElement("div");
    const add = document.createElement("button"), apply = document.createElement("button");
    set.dataset.anchor = String(s.anchor);
    legend.textContent = t("draw2.data.piece", { n: i + 1, formula: s.formula });
    rows.append(...s.data.map((/** @type {[string, string]} */ d) => row(d[0], d[1])));
    add.type = apply.type = "button"; add.textContent = t("draw2.data.add"); apply.textContent = t("draw2.data.apply"); buttons.className = "row";
    add.addEventListener("click", () => { const r = row("", ""); rows.append(r); /** @type {HTMLInputElement} */ (r.firstChild).focus(); });
    apply.addEventListener("click", () => {
      const data = [...rows.children].map(r => [/** @type {HTMLInputElement} */ (r.children[0]).value.trim(), /** @type {HTMLTextAreaElement} */ (r.children[1]).value.replace(/\r/g, "").replace(/\n+$/, "")]).filter(d => d[0]);
      const f = after(ed.set_data(s.anchor, JSON.stringify(data))), st = JSON.parse(ed.state());
      $("d2-data-note").textContent = f & CHANGED ? t("draw2.data.done", { n: data.length }) : t(st.error || "error.draw2.data");
    });
    buttons.append(add, apply); set.append(legend, rows, buttons);
    return set;
  }));
}

/** the annotation of each structure: level, identifier, evidence, note; what is typed is given to the module, which decides @param {any[]} outline */
function annFields(outline) {
  const box = $("d2-ann"), active = document.activeElement;
  if (active && box.contains(active) && active.tagName !== "BUTTON") return;
  const schema = JSON.parse(ed.ann_schema());
  $("d2-ann-kinds").textContent = t("draw2.ann.kinds", { list: schema.kinds.join(", ") });
  box.replaceChildren(...outline.map((s, i) => {
    const set = document.createElement("fieldset"), legend = document.createElement("legend"), apply = document.createElement("button");
    const level = document.createElement("select"), id = document.createElement("input"), evidence = document.createElement("textarea"), note = document.createElement("textarea");
    const a = s.ann || { level: "", known: true, identifier: "", evidence: "", note: "", entries: 0 };
    legend.textContent = t("draw2.data.piece", { n: i + 1, formula: s.formula });
    // a level of another scale is kept and shown as it is written
    for (const l of ["", ...schema.levels, ...(a.known ? [] : [a.level])]) level.append(new Option(l ? t("draw2.ann.level.n", { level: l }) : t("draw2.ann.level.none"), l));
    level.value = a.level; id.type = "text"; id.value = a.identifier; id.maxLength = 200; evidence.value = a.evidence; note.value = a.note;
    evidence.rows = 2; note.rows = 1;
    const field = (/** @type {string} */ key, /** @type {HTMLElement} */ el) => { const l = document.createElement("label"); l.className = "d2-annf"; l.append(t(key), el); return l; };
    apply.type = "button"; apply.textContent = t("draw2.data.apply");
    apply.addEventListener("click", () => {
      const f = after(ed.annotate(s.anchor, level.value, id.value, evidence.value.replace(/\r/g, ""), note.value.replace(/\r/g, ""))), st = JSON.parse(ed.state());
      $("d2-ann-note").textContent = f & CHANGED ? t(level.value ? "draw2.ann.done" : "draw2.ann.gone", { level: level.value }) : t(st.error || "error.draw2.ann");
    });
    const history = document.createElement("p"); history.className = "hint"; history.textContent = a.entries > 1 ? t("draw2.ann.history", { n: a.entries }) : "";
    set.append(legend, field("draw2.ann.level", level), field("draw2.ann.identifier", id), field("draw2.ann.evidence", evidence), field("draw2.ann.note", note), apply, history);
    return set;
  }));
}

/** the text of one check/1 warning, from the Drawing check catalog @param {any} w */
function warningText(w) {
  const allowed = (w.allowed || []).join(", ");
  return t(`draw.check.${w.code}.what`, { el: w.el || "", valence: w.valence ?? "", allowed, q: w.q ?? "", fix: w.fix ?? "", n: w.n ?? "", iso: w.iso ?? "", list: allowed });
}

function refresh() {
  const st = JSON.parse(ed.state());
  for (const g of Object.values(groups)) {
    if (g.owns(st.tool)) setHead(g, st.tool);
    if (g.tools.length) g.head.setAttribute("aria-pressed", String(g.owns(st.tool)));
  }
  for (const b of document.querySelectorAll("button[data-cmd]")) {
    const c = /** @type {HTMLElement} */ (b).dataset.cmd || "";
    if (c.includes(":") || ["select", "erase", "arrow", "plus"].includes(c)) b.setAttribute("aria-pressed", String(c === st.tool));
    if (c === "undo") /** @type {HTMLButtonElement} */ (b).disabled = !st.undo;
    if (c === "redo") /** @type {HTMLButtonElement} */ (b).disabled = !st.redo;
    if (c === "copy" || c === "cut" || c === "duplicate") /** @type {HTMLButtonElement} */ (b).disabled = !st.selected;
    if (c === "paste") /** @type {HTMLButtonElement} */ (b).disabled = !st.clip;
  }
  $("d2-outline").replaceChildren(...st.outline.map((/** @type {any} */ s, /** @type {number} */ i) => {
    const li = document.createElement("li");
    li.textContent = t("draw2.outline.piece", { n: i + 1, formula: s.formula, atoms: s.atoms, bonds: s.bonds }) + (s.ann ? " " + t("draw2.outline.level", { level: s.ann.level }) : "");
    return li;
  }));
  if (!st.outline.length) { const li = document.createElement("li"); li.textContent = t("draw2.outline.empty"); $("d2-outline").append(li); }
  dataFields(st.outline); annFields(st.outline);
  $("d2-focus").textContent = st.focus ? focusText(st.focus) : "";
  for (const [id, kind] of [["d2-element", "atom:"], ["d2-label", "label:"], ["d2-text-tool", "text:"]]) {
    const input = /** @type {HTMLInputElement} */ ($(id)), on = st.tool.startsWith(kind);
    input.classList.toggle("on", on);
    // the field shows the tool: what it holds when another tool is chosen is taken away (a text still being typed is left, a
    // refused one is not)
    if (!on && input.value && (document.activeElement !== input || input.hasAttribute("aria-invalid"))) { input.value = ""; input.removeAttribute("aria-invalid"); }
  }
  $("d2-status").textContent = t(st.objects ? "draw2.count.objects" : "draw2.count", { atoms: st.atoms, bonds: st.bonds, objects: st.objects });
  const list = $("d2-warnings"), ws = (st.check && st.check.warnings) || [];
  list.replaceChildren(...ws.map((/** @type {any} */ w) => {
    const li = document.createElement("li"), b = document.createElement("span");
    b.className = "lvl"; b.textContent = t(`draw.check.level.${w.level}`) + ": ";
    li.append(b, warningText(w)); li.dataset.code = w.code;
    return li;
  }));
  $("d2-summary").textContent = !st.atoms ? t("draw.check.empty") : ws.length ? t("draw.check.found", { n: ws.length }) : t("draw.check.ok");
  if (st.error) $("d2-live").textContent = I.err({ error_key: st.error, params: {} }).message;
  else if (st.say.key) $("d2-live").textContent = t(`draw2.say.${st.say.key.slice(10)}`, { what: st.say.what });
  document.documentElement.dataset.d2 = JSON.stringify({ focus: st.focus ? st.focus.index : 0, atoms: st.atoms, bonds: st.bonds, objects: st.objects, tool: st.tool, style: st.style, warnings: ws.length, selected: st.selected });
}

/** where the keyboard focus is, in words @param {any} f */
function focusText(f) {
  // one whole sentence per case: nothing is glued together
  const key = f.charge > 0 ? "draw2.focus.atom.plus" : f.charge < 0 ? "draw2.focus.atom.minus" : "draw2.focus.atom";
  return t(key, { name: f.name, index: f.index, of: f.of, bonds: f.bonds, hydrogens: f.hydrogens, n: Math.abs(f.charge) });
}

/** acts on the flags an editor call returned @param {number} f */
function after(f) {
  if (f & REPAINT) paint();
  if (f & (CHANGED | TOOL)) refresh();
  if (f & CHANGED) autosave.touch();
  if (f & FOCUS) { refresh(); const st = JSON.parse(ed.state()); $("d2-live").textContent = st.focus ? focusText(st.focus) : t("draw2.focus.none"); }
  return f;
}

// Autosave: the document is written to the browser's private file system (OPFS) one second after the last change, and offered back
// on the next visit. What is worth offering and what a saved text holds is decided in Rust (saved_summary, open). While an offer
// waits for its answer nothing is written, so the saved drawing cannot be lost to a stray click. Without OPFS there is no autosave.
const autosave = (() => {
  const NAME = "mzdraw-autosave.json";
  let timer = 0, last = "", paused = false;
  const file = async (/** @type {boolean} */ create) => (await (await navigator.storage.getDirectory()).getFileHandle(NAME, { create }));
  const mark = (/** @type {string} */ k, /** @type {string} */ v) => { document.documentElement.dataset[k] = v; };
  async function write() {
    timer = 0;
    const text = ed.save("mzdraw");
    if (paused || text === last) return;
    try {
      const w = await (await file(true)).createWritable();
      await w.write(text); await w.close();
      last = text; mark("d2Saved", String(Number(document.documentElement.dataset.d2Saved || 0) + 1));
    } catch { mark("d2Autosave", "off"); }
  }
  async function read() {
    try { return await (await (await file(false)).getFile()).text(); } catch { return ""; }
  }
  async function drop() {
    try { await (await navigator.storage.getDirectory()).removeEntry(NAME); } catch { /* nothing saved */ }
  }
  /** shows the offer when a drawing worth restoring was saved */
  async function offer() {
    const text = await read(), found = text && ed.saved_summary(text);
    mark("d2Autosave", "on");
    if (!found) return;
    paused = true;
    const n = JSON.parse(found), bar = $("d2-restore");
    $("d2-restore-text").textContent = t(n.objects ? "draw2.restore.found.objects" : "draw2.restore.found", n);
    const done = (/** @type {string} */ say) => { bar.hidden = true; paused = false; $("d2-live").textContent = t(say); size(); cv.focus({ preventScroll: true }); };
    $("d2-restore-yes").addEventListener("click", () => {
      try { last = text; after(ed.open(text)); after(ed.fit(cv.clientWidth, cv.clientHeight)); done("draw2.restore.done"); }
      catch (e) { $("d2-restore-text").textContent = I.err(JSON.parse(String(e))).message; }
    }, { once: true });
    $("d2-restore-no").addEventListener("click", async () => { await drop(); done("draw2.restore.dropped"); }, { once: true });
    bar.hidden = false; size();
  }
  return {
    offer,
    touch() { if (!timer) timer = setTimeout(write, 1000); else { clearTimeout(timer); timer = setTimeout(write, 1000); } },
    flush() { if (timer) { clearTimeout(timer); write(); } },
  };
})();

/** a toolbar command: view commands are handled here, the rest is the editor's @param {string} c */
function run(c) {
  const w = cv.clientWidth, h = cv.clientHeight;
  if (c === "zoom-in") after(ed.zoom(w / 2, h / 2, 1.25));
  else if (c === "zoom-out") after(ed.zoom(w / 2, h / 2, 0.8));
  else if (c === "fit") after(ed.fit(w, h));
  else after(ed.command(c) | CHANGED);
  if (c === "copy" || c === "cut") navigator.clipboard?.writeText(ed.clipboard("molfile")).catch(() => { });       // best effort: the editor keeps its own copy
}

// The system clipboard: plain text is a molfile (for other programs), our own type is the document (nothing lost between two mzDraw pages).
const CLIP = "application/x-mzdraw+json";
function clipboardEvents() {
  const here = () => document.activeElement === cv;
  const put = (/** @type {ClipboardEvent} */ e, /** @type {string} */ c) => {
    if (!here() || !e.clipboardData) return;
    const f = ed.command(c);
    if (!ed.clipboard("mzdraw")) return;
    e.clipboardData.setData("text/plain", ed.clipboard("molfile")); e.clipboardData.setData(CLIP, ed.clipboard("mzdraw"));
    e.preventDefault(); after(f | CHANGED);
  };
  document.addEventListener("copy", e => put(e, "copy"));
  document.addEventListener("cut", e => put(e, "cut"));
  document.addEventListener("paste", e => {
    if (!here() || !e.clipboardData) return;
    const text = e.clipboardData.getData(CLIP) || e.clipboardData.getData("text/plain");
    if (text) { e.preventDefault(); after(ed.paste(text) | CHANGED); }
  });
}

/** a flyout field that picks a tool with an argument: the element field (any symbol), the label field (an abbreviation or an R group,
 * with its list) and the text field (a line of text to place, or to set on an arrow); Enter or a choice picks the tool; the module
 * judges what is typed @param {"atom" | "label" | "text"} kind */
function field(kind) {
  const box = document.createElement("label"), input = document.createElement("input"), out = [box];
  box.className = "field"; box.append(t(`draw2.tool.${kind}`) + " ", input);
  input.id = { atom: "d2-element", label: "d2-label", text: "d2-text-tool" }[kind]; input.type = "text"; input.size = kind === "text" ? 12 : 7; input.autocomplete = "off"; input.spellcheck = false; input.maxLength = kind === "atom" ? 3 : 200;
  input.title = t(`draw2.tool.${kind}.title`); input.placeholder = t(`draw2.tool.${kind}.hint`);
  if (kind === "label") {
    const list = document.createElement("datalist");
    list.id = "d2-labels"; input.setAttribute("list", list.id);
    for (const name of ed.labels()) { const o = document.createElement("option"); o.value = name; list.append(o); }
    out.push(list);
  }
  const pick = () => {
    const v = kind === "text" ? input.value : input.value.trim();
    if (!v.trim()) return;
    if (after(ed.command(kind + ":" + v))) { input.removeAttribute("aria-invalid"); $("d2-live").textContent = t(`draw2.say.${kind}Tool`, { what: v }); cv.focus({ preventScroll: true }); }
    // a refused entry stays to be corrected, marked as not valid and selected: it is never shown as if it were the tool
    else { input.setAttribute("aria-invalid", "true"); input.select(); $("d2-live").textContent = t(`error.draw2.${kind}`); }
  };
  input.addEventListener("input", () => input.removeAttribute("aria-invalid"));
  input.addEventListener("change", pick);
  input.addEventListener("keydown", e => { if (e.key === "Enter") { e.preventDefault(); pick(); } else if (e.key === "Escape") cv.focus({ preventScroll: true }); });
  return out;
}

/** the name of a command in the i18n keys @param {string} c */
const keyOf = c => c.replace(":", ".").replace("+", "plus").replace("charge.-", "charge.minus").replace("zoom-in", "zoomIn").replace("zoom-out", "zoomOut");

/** the face of a button: the icon of the command, or an element symbol in its colour @param {string} c */
function face(c) {
  if (!c.startsWith("atom:")) return icon(c);
  const s = document.createElement("span");
  s.className = "sym"; s.dataset.el = c.slice(5); s.textContent = c.slice(5); s.setAttribute("aria-hidden", "true");
  return s;
}

/** one button of a toolbar or a flyout; `text`: the short label is written next to the icon (flyouts) @param {string} c @param {boolean} primary @param {boolean} text */
function button(c, primary, text) {
  const b = document.createElement("button"), k = keyOf(c);
  b.type = "button"; b.dataset.cmd = c; b.dataset.testid = "d2-" + c;
  if (primary) b.dataset.level = "primary";
  b.append(face(c));
  if (text) { const s = document.createElement("span"); s.className = "txt"; s.textContent = c.startsWith("atom:") ? t(`draw2.tool.${k}.title`).replace(/ \(.*$/, "") : t(`draw2.tool.${k}`); b.append(s); }
  b.title = t(`draw2.tool.${k}.title`); b.setAttribute("aria-label", b.title);
  // a pointer click gives the keyboard back to the canvas (shortcuts keep working); a keyboard activation stays on the button
  b.addEventListener("click", e => {
    const grp = b.closest(".grp");
    if (c === "open") $("d2-file").click();
    else if (c === "panel") panel(!b.classList.contains("on"));
    else run(c);
    if (grp) { flyout(groups[/** @type {HTMLElement} */ (grp).dataset.group], false); if (!e.detail) groups[/** @type {HTMLElement} */ (grp).dataset.group].head.focus(); }
    if (e.detail && c !== "open" && c !== "panel") cv.focus({ preventScroll: true });
  });
  return b;
}

/** a head and its flyout @param {any} spec @param {boolean} down the flyout opens below the head (top bar), else to its right */
function group(spec, down) {
  const wrap = document.createElement("div"), head = document.createElement("button"), fly = document.createElement("div");
  const tools = spec.tools || [], grp = { g: spec.g, head, fly, tools, last: spec.g === "atom" ? "" : tools[0] || "", owns: (/** @type {string} */ c) => false };
  grp.owns = spec.g === "atom" ? c => (c.startsWith("atom:") && !["atom:C", "atom:N", "atom:O"].includes(c)) || c.startsWith("label:")
    : spec.g === "obj" ? c => tools.includes(c) || c.startsWith("text:") : c => tools.includes(c);
  groups[spec.g] = grp;
  wrap.className = "grp" + (down ? " down" : ""); wrap.dataset.group = spec.g;
  head.type = "button"; head.className = "head"; head.dataset.testid = "d2-g-" + spec.g; head.dataset.group = spec.g;
  if (spec.p) head.dataset.level = "primary";
  head.title = t(`draw2.group.${spec.g}.title`); head.setAttribute("aria-label", head.title);
  head.setAttribute("aria-expanded", "false"); head.setAttribute("aria-controls", "d2-fly-" + spec.g);
  head.append(grp.last ? face(grp.last) : icon(spec.g === "atom" ? "elements" : spec.g === "save" ? "save" : "more"));
  fly.id = "d2-fly-" + spec.g; fly.className = "fly" + (spec.g === "atom" ? " grid" : ""); fly.hidden = true;
  fly.setAttribute("role", "group"); fly.setAttribute("aria-label", t(`draw2.group.${spec.g}`));
  for (const c of [...tools, ...(spec.cmds || [])]) fly.append(button(c, false, spec.g !== "atom"));
  for (const k of spec.fields || []) fly.append(...field(k));
  if (spec.adopt) {
    // the save buttons of the page move into the flyout (closeAfterSave)
    const sec = $(spec.adopt);
    fly.append(...sec.querySelectorAll("button"), $("d2-ex-note")); sec.remove();
  }
  head.addEventListener("click", e => {
    const now = JSON.parse(ed.state()).tool;
    if (grp.last && now !== grp.last && fly.hidden) { run(grp.last); if (e.detail) cv.focus({ preventScroll: true }); }
    else flyout(grp, fly.hidden, !e.detail);
  });
  fly.addEventListener("keydown", e => {
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); flyout(grp, false); head.focus(); return; }
    const inField = /** @type {HTMLElement} */ (e.target).tagName === "INPUT";
    const step = { ArrowDown: 1, ArrowUp: -1, ...(inField ? {} : { ArrowRight: 1, ArrowLeft: -1 }) }[e.key];
    if (!step) return;
    e.preventDefault();
    const all = /** @type {HTMLElement[]} */ ([...fly.querySelectorAll("button:not(:disabled), input")]), i = all.indexOf(/** @type {HTMLElement} */ (document.activeElement));
    all[(i + step + all.length) % all.length]?.focus();
  });
  wrap.addEventListener("focusout", e => { if (!fly.hidden && e.relatedTarget && !wrap.contains(/** @type {Node} */ (e.relatedTarget))) flyout(grp, false); });
  wrap.append(head, fly);
  return wrap;
}

/** the save flyout closes after a save and stays open to show a refusal; listened after the save itself (panels) */
function closeAfterSave() {
  const grp = groups.save;
  for (const b of grp.fly.querySelectorAll("button")) b.addEventListener("click", e => {
    if ($("d2-ex-note").textContent) return;
    flyout(grp, false);
    if (!(/** @type {MouseEvent} */ (e).detail)) grp.head.focus();
  });
}

/** opens or closes a flyout; an opened one closes the others; `focus`: the keyboard goes to the kind in use, else the first item
 * @param {any} grp @param {boolean} open @param {boolean} [focus] */
function flyout(grp, open, focus = false) {
  if (open) for (const o of Object.values(groups)) if (o !== grp) flyout(o, false);
  grp.fly.hidden = !open; grp.head.setAttribute("aria-expanded", String(open));
  if (open && focus) /** @type {HTMLElement} */ (grp.fly.querySelector('[aria-pressed="true"]') || grp.fly.querySelector("button:not(:disabled), input"))?.focus();
}

/** shows the kind last used from a tool group on its head @param {any} grp @param {string} c */
function setHead(grp, c) {
  if (grp.last === c || c.startsWith("label:") || c.startsWith("text:")) return;
  grp.last = c; grp.head.firstChild.replaceWith(face(c));
  grp.head.title = t(`draw2.group.${grp.g}.title`) ; grp.head.setAttribute("aria-label", t(`draw2.group.${grp.g}.title`));
}

/** the side panel: beside the canvas on a wide screen, a drawer over it on a narrow one @param {boolean} open */
function panel(open) {
  const p = $("d2-panel"), b = /** @type {HTMLElement} */ (document.querySelector('[data-cmd="panel"]'));
  p.hidden = !open; b.classList.toggle("on", open); b.setAttribute("aria-expanded", String(open));
  size();
}

/** a toolbar: one tab stop; the arrow keys move along it, and across it they open a flyout @param {HTMLElement} bar @param {any[]} spec @param {boolean} vertical */
function toolbar(bar, spec, vertical) {
  for (const c of spec) {
    if (c === "|") { const s = document.createElement("span"); s.className = "sep"; s.setAttribute("role", "separator"); bar.append(s); continue; }
    bar.append(typeof c === "string" ? button(c.replace(/^P:/, ""), c.startsWith("P:"), false) : group(c, !vertical));
  }
  const items = () => /** @type {HTMLElement[]} */ ([...bar.querySelectorAll(":scope > button, :scope > .grp > .head")]).filter(b => !/** @type {HTMLButtonElement} */ (b).disabled);
  const rove = (/** @type {HTMLElement} */ to) => { for (const b of bar.querySelectorAll(":scope > button, :scope > .grp > .head")) b.setAttribute("tabindex", b === to ? "0" : "-1"); };
  rove(items()[0]);
  bar.addEventListener("focusin", e => { if (items().includes(/** @type {HTMLElement} */ (e.target))) rove(/** @type {HTMLElement} */ (e.target)); });
  bar.addEventListener("keydown", e => {
    const all = items(), i = all.indexOf(/** @type {HTMLElement} */ (e.target));
    if (i < 0) return;
    const [next, prev, across] = vertical ? ["ArrowDown", "ArrowUp", "ArrowRight"] : ["ArrowRight", "ArrowLeft", "ArrowDown"];
    const to = e.key === next ? all[(i + 1) % all.length] : e.key === prev ? all[(i - 1 + all.length) % all.length] : e.key === "Home" ? all[0] : e.key === "End" ? all[all.length - 1] : null;
    const grp = /** @type {HTMLElement} */ (e.target).dataset.group;
    if (to) { e.preventDefault(); to.focus(); }
    else if (e.key === across && grp) { e.preventDefault(); flyout(groups[grp], true, true); }
  });
}

/** every toolbar of the page; a press outside a flyout closes it */
function toolbars() {
  toolbar($("d2-tools"), COLUMN, true); toolbar($("d2-bar"), BAR, false); toolbar($("d2-view"), VIEW, false);
  document.addEventListener("pointerdown", e => { for (const g of Object.values(groups)) if (!g.fly.hidden && !g.head.parentElement?.contains(/** @type {Node} */ (e.target))) flyout(g, false); });
  panel(!matchMedia("(max-width: 900px)").matches);
  $("d2-panel").addEventListener("keydown", e => { if (e.key === "Escape" && matchMedia("(max-width: 900px)").matches) { panel(false); /** @type {HTMLElement} */ (document.querySelector('[data-cmd="panel"]')).focus(); } });
}

// bit 4: the eraser end or eraser button of a pen (button 5, buttons bit 32)
const mods = (/** @type {MouseEvent | KeyboardEvent} */ e) => (e.shiftKey ? 1 : 0) | (e.ctrlKey || e.metaKey ? 2 : 0) | ("buttons" in e && (e.buttons & 32 || e.button === 5) ? 4 : 0);
const kind = (/** @type {PointerEvent} */ e) => (e.pointerType === "pen" ? 1 : e.pointerType === "touch" ? 2 : 0);

function events() {
  /** @type {{x: number, y: number} | null} */ let panning = null;
  const at = (/** @type {PointerEvent | WheelEvent} */ e) => { const r = cv.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
  // every pointer goes to Rust with its id and its time stamp: which finger is a palm, what two fingers do and what a long press
  // means is decided there
  const send = (/** @type {number} */ phase, /** @type {PointerEvent} */ e) => { const [x, y] = at(e); return ed.pointer(phase, x, y, kind(e), mods(e), e.pointerId, e.timeStamp); };
  cv.addEventListener("pointerdown", e => {
    cv.focus();
    try { cv.setPointerCapture(e.pointerId); } catch { /* a pointer that is already gone cannot be captured: the event still counts */ }
    if (e.pointerType === "mouse" && (e.button === 1 || e.button === 2)) { panning = { x: e.clientX, y: e.clientY }; e.preventDefault(); return; }
    after(send(0, e));
  });
  cv.addEventListener("pointermove", e => {
    if (panning) { after(ed.pan(e.clientX - panning.x, e.clientY - panning.y)); panning = { x: e.clientX, y: e.clientY }; return; }
    after(send(1, e));
  });
  cv.addEventListener("pointerup", e => {
    if (panning) { panning = null; return; }
    after(send(2, e) | CHANGED);                              // a release may have changed the selection alone
  });
  cv.addEventListener("pointercancel", e => { panning = null; after(send(3, e)); });       // taken away by the system: nothing is committed
  cv.addEventListener("contextmenu", e => e.preventDefault());
  cv.addEventListener("wheel", e => { e.preventDefault(); const [x, y] = at(e); after(ed.zoom(x, y, e.deltaY < 0 ? 1.1 : 1 / 1.1)); }, { passive: false });
  cv.addEventListener("keydown", e => {
    const f = ed.key(e.key, mods(e));
    if (after(f ? f | CHANGED : 0) & HANDLED) e.preventDefault();
    if (f & (ASK_LABEL | ASK_TEXT)) {
      flyout(groups[f & ASK_LABEL ? "atom" : "obj"], true);
      const l = /** @type {HTMLInputElement} */ ($(f & ASK_LABEL ? "d2-label" : "d2-text-tool")); l.focus(); l.select();
    }
  });
  addEventListener("resize", size);
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", paint);
  // a host page may set the theme on the root (data-theme="light|dark"): the canvas is painted again in its colours
  new MutationObserver(paint).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
}

/** adds a structure text, or the bytes of a file, to the drawing and says how it went @param {string | Uint8Array} text */
function add(text) {
  const note = $("d2-note");
  try {
    const info = JSON.parse(typeof text === "string" ? ed.add(text) : ed.add_bytes(text));
    note.textContent = info.notes.map((/** @type {[string, number]} */ [key, n]) => t(key, { n })).join(" ");
    after(ed.fit(cv.clientWidth, cv.clientHeight) | CHANGED);
  } catch (e) { note.textContent = I.err(JSON.parse(String(e))).message; }
}

/** downloads a text as a file @param {string} what @param {string} name @param {string} type */
function save(what, name, type) {
  const note = $("d2-ex-note"); note.textContent = "";
  try { download(ed.save(what), name, type); } catch (e) { note.textContent = I.err(JSON.parse(String(e))).message; }
}

/** @param {BlobPart} data @param {string} name @param {string} type */
function download(data, name, type) {
  const a = document.createElement("a"), url = URL.createObjectURL(new Blob([data], { type }));
  a.href = url; a.download = name; document.body.append(a); a.click(); a.remove();
  URL.revokeObjectURL(url);
}

/** saves a PNG: the SVG the module writes is drawn on a canvas of the size the module asks for; the module finishes the file */
async function savePng() {
  const note = $("d2-ex-note"); note.textContent = "";
  try {
    const plan = JSON.parse(ed.png_plan()), img = new Image(), url = URL.createObjectURL(new Blob([plan.svg], { type: "image/svg+xml" }));
    await new Promise((ok, no) => { img.onload = ok; img.onerror = no; img.src = url; });
    const c = document.createElement("canvas"); c.width = plan.width; c.height = plan.height;
    const x = /** @type {CanvasRenderingContext2D} */ (c.getContext("2d"));
    x.fillStyle = plan.paper; x.fillRect(0, 0, c.width, c.height); x.drawImage(img, 0, 0, c.width, c.height);
    URL.revokeObjectURL(url);
    const blob = /** @type {Blob} */ (await new Promise(ok => c.toBlob(ok, "image/png")));
    download(ed.png_finish(new Uint8Array(await blob.arrayBuffer())), "drawing.png", "image/png");
  } catch (e) { note.textContent = I.err(JSON.parse(String(e))).message; }
}

/** the style: the presets and the fields the module's schema names; a choice changes the screen and the saved pictures, not the
 * document. The settings file the module writes for the style is kept in the browser (OPFS) and given back to the module on the
 * next visit; it can be saved as a file and a settings file can be opened. The module judges every value and every file. */
async function styles() {
  const sel = /** @type {HTMLSelectElement} */ ($("d2-style")), note = $("d2-style-note"), NAME = "mzdraw-settings.json";
  const schema = JSON.parse(ed.settings_schema()), box = $("d2-style-fields");
  /** @type {Record<string, HTMLInputElement | HTMLSelectElement>} */ const inputs = {};
  for (const name of schema.presets) { const o = document.createElement("option"); o.value = name; o.textContent = t(`draw2.style.${name}`); sel.append(o); }
  const keep = async () => {
    try {
      const w = await (await (await navigator.storage.getDirectory()).getFileHandle(NAME, { create: true })).createWritable();
      await w.write(ed.style()); await w.close();
      document.documentElement.dataset.d2Style = String(Number(document.documentElement.dataset.d2Style || 0) + 1);
    } catch { /* no storage: the style lasts for this visit */ }
  };
  /** shows the style the module holds: the preset it was made from and every value */
  const show = () => {
    const style = JSON.parse(ed.style()).style;
    sel.value = style.preset; if (sel.selectedIndex < 0) sel.selectedIndex = 0;
    for (const [key, input] of Object.entries(inputs)) { input.value = String(style[key]); input.removeAttribute("aria-invalid"); }
  };
  /** one change asked of the module; a refusal is said in words, the field is marked and nothing changes */
  const ask = (/** @type {() => number} */ change, /** @type {string} */ said, /** @type {Element | null} */ field) => {
    try { after(change() | CHANGED); show(); note.textContent = said; $("d2-live").textContent = t("draw2.say.style", { what: sel.selectedOptions[0].textContent || "" }); keep(); }
    catch (e) { note.textContent = I.err(JSON.parse(String(e))).message; field?.setAttribute("aria-invalid", "true"); }
  };
  const row = (/** @type {string} */ key, /** @type {HTMLInputElement | HTMLSelectElement} */ input, /** @type {string} */ unit) => {
    const label = document.createElement("label"), name = document.createElement("span");
    label.className = "field"; name.textContent = t(`draw2.style.field.${key}`);
    input.id = `d2-style-${key}`; inputs[key] = input;
    input.addEventListener("change", () => ask(() => ed.set(`style.${key}=${input.value}`), t("draw2.style.custom.note"), input));
    label.append(name, input, unit); box.append(label);
  };
  for (const f of schema.fields) {
    const input = document.createElement("input");
    input.type = "number"; input.min = String(f.min); input.max = String(f.max); input.step = "any"; input.inputMode = "decimal";
    row(f.key, input, t(`draw2.style.unit.${f.unit}`));
  }
  const colour = document.createElement("select");
  for (const c of schema.colours) { const o = document.createElement("option"); o.value = c; o.textContent = t(`draw2.style.colour.${c}`); colour.append(o); }
  row("colour", colour, "");
  sel.addEventListener("change", () => ask(() => ed.set_style(sel.value), t(`draw2.style.${sel.value}.note`), null));
  $("d2-style-save").addEventListener("click", () => download(ed.style(), "mzdraw-settings.json", "application/json"));
  $("d2-style-open").addEventListener("click", () => $("d2-style-file").click());
  $("d2-style-file").addEventListener("change", async e => {
    const f = /** @type {HTMLInputElement} */ (e.target).files?.[0];
    if (f) { const text = await f.text(); ask(() => ed.set_style(text), t("draw2.style.opened.note"), null); }
    /** @type {HTMLInputElement} */ (e.target).value = "";
  });
  try { ed.set_style(await (await (await (await navigator.storage.getDirectory()).getFileHandle(NAME)).getFile()).text()); } catch { /* nothing kept, or a file the module refuses */ }
  show();
  note.textContent = t(`draw2.style.${sel.value}.note`);
}

function panels() {
  $("d2-add").addEventListener("click", () => add(/** @type {HTMLTextAreaElement} */ ($("d2-text")).value));
  $("d2-open").addEventListener("click", () => $("d2-file").click());
  $("d2-file").addEventListener("change", async e => { const f = /** @type {HTMLInputElement} */ (e.target).files?.[0]; if (f) add(new Uint8Array(await f.arrayBuffer())); });
  $("d2-ex-molfile").addEventListener("click", () => save("molfile", "drawing.mol", "chemical/x-mdl-molfile"));
  $("d2-ex-v3000").addEventListener("click", () => save("molfile-v3000", "drawing-v3000.mol", "chemical/x-mdl-molfile"));
  $("d2-ex-sdf").addEventListener("click", () => save("sdf", "drawing.sdf", "chemical/x-mdl-sdfile"));
  $("d2-ex-smiles").addEventListener("click", () => save("smiles", "drawing.smi", "chemical/x-daylight-smiles"));
  $("d2-ex-ket").addEventListener("click", () => save("ket", "drawing.ket", "application/json"));
  $("d2-ex-cdxml").addEventListener("click", () => save("cdxml", "drawing.cdxml", "chemical/x-cdxml"));
  $("d2-ex-cml").addEventListener("click", () => save("cml", "drawing.cml", "chemical/x-cml"));
  $("d2-ex-rxn").addEventListener("click", () => save("rxn", "drawing.rxn", "chemical/x-mdl-rxnfile"));
  $("d2-ex-png").addEventListener("click", savePng);
  $("d2-ex-svg").addEventListener("click", () => save("svg", "drawing.svg", "image/svg+xml"));
  $("d2-ex-mzdraw").addEventListener("click", () => save("mzdraw", "drawing.mzdraw.json", "application/json"));
}

I.apply(document);
document.title = t("draw2.name");
try {
  ed = await load();
  /** @type {any} */ (window).MZDRAW = ed;
  toolbars(); events(); clipboardEvents(); panels(); closeAfterSave(); await styles(); size(); refresh();
  document.addEventListener("visibilitychange", () => { if (document.hidden) autosave.flush(); });
  await autosave.offer();
  document.documentElement.dataset.d2Ready = "1";
} catch (e) {
  $("d2-status").textContent = t("draw2.unavailable");
  document.documentElement.dataset.d2Ready = "0";
}
