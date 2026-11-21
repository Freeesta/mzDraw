// The page: forwards pointer, wheel and key events to the editor in the mzdraw .wasm and paints the display list it returns.
// No chemistry and no decision here: what a click does is decided in Rust (crates/mzdraw).

import { build, layout, show, open, PALETTES } from "./palettes.js";
import { NAMES } from "./names.js";
import { contextMenu } from "./context.js";

const I = /** @type {any} */ (window).I18N, t = (k, p = {}) => I.t(k, p);
const $ = id => /** @type {HTMLElement} */ (document.getElementById(id));
const REPAINT = 1, CHANGED = 2, TOOL = 4, HANDLED = 8, ASK_LABEL = 16, ASK_TEXT = 32, FOCUS = 64, MENU = 512;
// what each save format is called when downloaded: file name and media type (png is drawn by the page, savePng; pdf is bytes, savePdf)
const FILES = /** @type {Record<string, [string, string]>} */ ({ molfile: ["drawing.mol", "chemical/x-mdl-molfile"], "molfile-v3000": ["drawing-v3000.mol", "chemical/x-mdl-molfile"],
  sdf: ["drawing.sdf", "chemical/x-mdl-sdfile"], smiles: ["drawing.smi", "chemical/x-daylight-smiles"], ket: ["drawing.ket", "application/json"], cdxml: ["drawing.cdxml", "chemical/x-cdxml"],
  cml: ["drawing.cml", "chemical/x-cml"], rxn: ["drawing.rxn", "chemical/x-mdl-rxnfile"], trasf: ["drawing.trasf.json", "application/json"], svg: ["drawing.svg", "image/svg+xml"], [NAMES.slug]: [NAMES.doc, "application/json"], log: ["drawing.log.json", "application/json"] });
const COLOURS = ["--d2-ink", "--d2-select", "--d2-hover", "--d2-warn", "--d2-preview", "--d2-n", "--d2-o", "--d2-s", "--d2-x", "--d2-p", "--d2-focus"];

/** @type {any} */ let ed = null;
/** the context menu of the canvas (context.js) @type {ReturnType<typeof contextMenu>} */ let menu;
const cv = /** @type {HTMLCanvasElement} */ ($("d2-canvas")), g = /** @type {CanvasRenderingContext2D} */ (cv.getContext("2d"));

async function load() {
  const mod = await import(`./pkg/${NAMES.wasm}.js`);
  await mod.default();
  if (mod.version() !== NAMES.bridge) throw new Error("version");
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

/** the canvas takes the area the palettes leave free @param {number[]} a [x, y, width, height] */
function place(a) {
  const s = $("d2-stage").style, [x, y, w, h] = a.map(v => v + "px");
  if (s.left === x && s.top === y && s.width === w && s.height === h) return;
  Object.assign(s, { left: x, top: y, width: w, height: h });
  size();
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

/** the atom-to-atom map field: what is typed is given to the module, which picks the atom (the focused or the only selected one) and judges the number */
function mapField() {
  const input = /** @type {HTMLInputElement} */ ($("d2-map")), note = $("d2-map-note");
  const apply = () => {
    const f = after(ed.set_map(input.value)), st = JSON.parse(ed.state());
    note.textContent = f & CHANGED ? t("draw2.map.done", { n: Number(input.value.trim()) }) : t(st.error || "error.draw2.map");
  };
  $("d2-map-set").addEventListener("click", apply);
  input.addEventListener("keydown", e => { if (e.key === "Enter") { e.preventDefault(); apply(); } });
}

/** the annotation of each structure: level, identifier, evidence, note; what is typed is given to the module, which decides @param {any[]} outline */
function annFields(outline) {
  const box = $("d2-ann"), active = document.activeElement;
  if (active && box.contains(active) && active.tagName !== "BUTTON") return;
  const schema = JSON.parse(ed.ann_schema());
  $("d2-ann-kinds").textContent = t("draw2.ann.kinds", { list: schema.kinds.join(", ") });
  box.replaceChildren(...outline.map((s, i) => {
    const set = document.createElement("fieldset"), legend = document.createElement("legend"), apply = document.createElement("button");
    const scale = document.createElement("select"), level = document.createElement("select"), id = document.createElement("input"), evidence = document.createElement("textarea"), note = document.createElement("textarea");
    set.dataset.anchor = String(s.anchor);
    const a = s.ann || { scale: schema.scale, level: "", known: true, identifier: "", evidence: "", note: "", entries: 0 };
    legend.textContent = t("draw2.data.piece", { n: i + 1, formula: s.formula });
    // the scales are those of the module (and the scale of a file, as it is written); the levels follow the scale, and a level
    // not of the scale (as read from a file) is kept and offered as "of the file"
    const scales = schema.scales.map((/** @type {any} */ x) => x.name);
    for (const n of [...scales, ...(scales.includes(a.scale) ? [] : [a.scale])]) scale.append(new Option(scales.includes(n) ? t(`draw2.ann.scale.${n}`) : t("draw2.ann.scale.file", { scale: n }), n));
    scale.value = a.scale;
    const fill = (/** @type {string} */ keep = level.value) => {
      // the list (and which level is "of the file") is the module's; the chosen level stays if the new list has it
      const list = JSON.parse(ed.ann_levels(scale.value, a.level));
      level.replaceChildren(new Option(t("draw2.ann.level.none"), ""), ...list.map((/** @type {any} */ x) => new Option(x.ofFile ? t("draw2.ann.level.file", { level: x.level }) : t("draw2.ann.level.m", { level: x.level, meaning: t(`draw2.ann.level.${scale.value}.${x.level}`) }), x.level)));
      level.value = list.some((/** @type {any} */ x) => x.level === keep) ? keep : "";
    };
    scale.addEventListener("change", () => fill());
    fill(a.level);
    id.type = "text"; id.value = a.identifier; id.maxLength = 200; evidence.value = a.evidence; note.value = a.note;
    evidence.rows = 2; note.rows = 1;
    const field = (/** @type {string} */ key, /** @type {HTMLElement} */ el) => { const l = document.createElement("label"); l.className = "d2-annf"; l.append(t(key), el); return l; };
    apply.type = "button"; apply.textContent = t("draw2.data.apply");
    apply.addEventListener("click", () => {
      const f = after(ed.annotate(s.anchor, scale.value, level.value, id.value, evidence.value.replace(/\r/g, ""), note.value.replace(/\r/g, ""))), st = JSON.parse(ed.state());
      $("d2-ann-note").textContent = f & CHANGED ? t(level.value ? "draw2.ann.done" : "draw2.ann.gone", { level: level.value }) : t(st.error || "error.draw2.ann");
    });
    const history = document.createElement("p"); history.className = "hint"; history.textContent = a.entries > 1 ? t("draw2.ann.history", { n: a.entries }) : "";
    set.append(legend, field("draw2.ann.scale", scale), field("draw2.ann.level", level), field("draw2.ann.identifier", id), field("draw2.ann.evidence", evidence), field("draw2.ann.note", note), apply, history);
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
  show(st);
  for (const b of document.querySelectorAll("button[data-cmd]")) {
    const c = /** @type {HTMLElement} */ (b).dataset.cmd || "";
    if (c.includes(":") || ["select", "erase", "arrow", "plus"].includes(c)) b.setAttribute("aria-pressed", String(c === st.tool));
    if (c === "move-here") b.setAttribute("aria-pressed", String(st.moveHere));
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
  // the drawing checks: Rust names the checks that have findings, the page lists them (a line selects what it is about)
  const std = st.std.lines;
  $("d2-std-list").replaceChildren(...std.map((/** @type {any} */ c) => {
    const li = document.createElement("li"), b = document.createElement("button"), g = document.createElement("span");
    g.className = "lvl"; g.textContent = t(`draw2.std.grade.${c.grade}`);
    b.type = "button"; b.dataset.key = c.key; b.dataset.grade = c.grade; b.append(g, " ", t(`draw2.${c.key}`, { n: c.n }));
    li.append(b); return li;
  }));
  $("d2-std-note").textContent = st.std.none ? t("draw2.std.none") : "";
  $("d2-summary").textContent = !st.atoms ? t("draw.check.empty") : ws.length ? t("draw.check.found", { n: ws.length }) : t("draw.check.ok");
  if (st.error) $("d2-live").textContent = I.err({ error_key: st.error, params: {} }).message;
  else if (st.say.key) $("d2-live").textContent = t(st.say.key, { what: st.say.what });
  document.documentElement.dataset.d2 = JSON.stringify({ focus: st.focus ? st.focus.index : 0, atoms: st.atoms, bonds: st.bonds, objects: st.objects, tool: st.tool, style: st.style, warnings: ws.length, std: std.length, selected: st.selected });
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
  if (f & PALETTES) { layout(); keep(); }
  menu?.sync();
  if (f & FOCUS) { refresh(); const st = JSON.parse(ed.state()); $("d2-live").textContent = st.focus ? focusText(st.focus) : t("draw2.focus.none"); }
  return f;
}

// Autosave: the document is written to the browser's private file system (OPFS) one second after the last change, and offered back
// on the next visit. What is worth offering and what a saved text holds is decided in Rust (saved_summary, open). While an offer
// waits for its answer nothing is written, so the saved drawing cannot be lost to a stray click. Without OPFS there is no autosave.
const autosave = (() => {
  const NAME = NAMES.autosave;
  let timer = 0, last = "", paused = false;
  // the file of the name in use, and the files an earlier name wrote (read when the current one is not there)
  const OLD = NAMES.autosaveOld;
  const file = async (/** @type {boolean} */ create, /** @type {string} */ name = NAME) => (await (await navigator.storage.getDirectory()).getFileHandle(name, { create }));
  const mark = (/** @type {string} */ k, /** @type {string} */ v) => { document.documentElement.dataset[k] = v; };
  async function write() {
    timer = 0;
    const text = ed.save(NAMES.slug);
    if (paused || text === last) return;
    try {
      const w = await (await file(true)).createWritable();
      await w.write(text); await w.close();
      last = text; mark("d2Saved", String(Number(document.documentElement.dataset.d2Saved || 0) + 1));
    } catch { mark("d2Autosave", "off"); }
  }
  async function read() {
    for (const name of [NAME, ...OLD]) { try { return await (await (await file(false, name)).getFile()).text(); } catch { /* not there */ } }
    return "";
  }
  async function drop() {
    for (const name of [NAME, ...OLD]) { try { await (await navigator.storage.getDirectory()).removeEntry(name); } catch { /* nothing saved */ } }
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

// The system clipboard: plain text is a molfile (for other programs), our own type is the document (nothing lost between two pages of the editor).
const CLIP = NAMES.clip;
function clipboardEvents() {
  const here = () => document.activeElement === cv;
  const put = (/** @type {ClipboardEvent} */ e, /** @type {string} */ c) => {
    if (!here() || !e.clipboardData) return;
    const f = ed.command(c);
    if (!ed.clipboard(NAMES.slug)) return;
    e.clipboardData.setData("text/plain", ed.clipboard("molfile")); e.clipboardData.setData(CLIP, ed.clipboard(NAMES.slug));
    e.preventDefault(); after(f | CHANGED);
  };
  document.addEventListener("copy", e => put(e, "copy"));
  document.addEventListener("cut", e => put(e, "cut"));
  document.addEventListener("paste", e => {
    if (!here() || !e.clipboardData) return;
    const own = [CLIP, ...NAMES.clipOld].map(k => /** @type {DataTransfer} */ (e.clipboardData).getData(k)).find(Boolean);
    const text = own || e.clipboardData.getData("text/plain");
    if (text) { e.preventDefault(); after(ed.paste(text) | CHANGED); }
  });
}

/** a flyout field that picks a tool with an argument: the element field (any symbol), the label field (an abbreviation or an R group,
 * with its list) and the text field (a line of text to place, or to set on an arrow); Enter or a choice picks the tool; the module
 * judges what is typed @param {string} kind */
function field(kind) {
  const box = document.createElement("label"), input = document.createElement("input"), out = [box];
  box.className = "field"; box.append(t(`draw2.tool.${kind}`) + " ", input);
  input.id = { atom: "d2-element", label: "d2-label", text: "d2-text-tool" }[kind] || ""; input.type = "text"; input.size = kind === "text" ? 12 : 7; input.autocomplete = "off"; input.spellcheck = false; input.maxLength = kind === "atom" ? 3 : 200;
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
    if (after(ed.command(kind + ":" + v))) { input.removeAttribute("aria-invalid"); $("d2-live").textContent = t(`draw2.say.${kind}Tool`, { what: v }); open(input.closest(".fly")?.getAttribute("data-fly") || "", false); cv.focus({ preventScroll: true }); }
    // a refused entry stays to be corrected, marked as not valid and selected: it is never shown as if it were the tool
    else { input.setAttribute("aria-invalid", "true"); input.select(); $("d2-live").textContent = t(`error.draw2.${kind}`); }
  };
  input.addEventListener("input", () => input.removeAttribute("aria-invalid"));
  input.addEventListener("change", pick);
  input.addEventListener("keydown", e => { if (e.key === "Enter") { e.preventDefault(); pick(); } else if (e.key === "Escape") cv.focus({ preventScroll: true }); });
  return out;
}

// bit 8: Alt (Option); bit 4: the eraser end or eraser button of a pen (button 5, buttons bit 32)
const mods = (/** @type {MouseEvent | KeyboardEvent} */ e) => (e.shiftKey ? 1 : 0) | (e.ctrlKey || e.metaKey ? 2 : 0) | (e.altKey ? 8 : 0) | (e.button === 2 || ("buttons" in e && e.buttons & 2) ? 16 : 0) | (e.button === 1 || ("buttons" in e && e.buttons & 4) ? 32 : 0) | ("buttons" in e && (e.buttons & 32 || e.button === 5) ? 4 : 0);
const kind = (/** @type {PointerEvent} */ e) => (e.pointerType === "pen" ? 1 : e.pointerType === "touch" ? 2 : 0);

function events() {
  const at = (/** @type {PointerEvent | WheelEvent} */ e) => { const r = cv.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
  // every pointer goes to Rust with its id, its buttons and its time stamp: which finger is a palm, what two fingers do, what a long
  // press means, what the middle and the secondary button do (pan, menu) and what the barrel button of a pen does is decided there
  const send = (/** @type {number} */ phase, /** @type {PointerEvent} */ e) => { const [x, y] = at(e); return ed.pointer(phase, x, y, kind(e), mods(e), e.pointerId, e.timeStamp); };
  /** the flag MENU: the module asks for the context menu at the point of the event @param {number} f @param {PointerEvent} e */
  const ask = (f, e) => { if (f & MENU) { const [x, y] = at(e); menu.open(x, y, kind(e)); } };
  cv.addEventListener("pointerdown", e => {
    cv.focus();
    try { cv.setPointerCapture(e.pointerId); } catch { /* a pointer that is already gone cannot be captured: the event still counts */ }
    const f = send(0, e);
    after(f);
    if (f & HANDLED) e.preventDefault();                     // a button that is not the main one: no autoscroll, no selection
    ask(f, e);                                               // the barrel button of a pen asks at the press
  });
  cv.addEventListener("pointermove", e => { after(send(1, e)); });
  cv.addEventListener("pointerup", e => {
    const f = send(2, e);
    after(f | CHANGED);                                       // a release may have changed the selection alone
    ask(f, e);                                                // a finger or a pen held still, the secondary button released in place
  });
  cv.addEventListener("pointercancel", e => { after(send(3, e)); });       // taken away by the system: nothing is committed
  // no menu of the browser on the canvas or on our menu: the event is cancelled and nothing else is decided here
  cv.addEventListener("contextmenu", e => e.preventDefault());
  cv.addEventListener("wheel", e => { e.preventDefault(); const [x, y] = at(e); after(ed.zoom(x, y, e.deltaY < 0 ? 1.1 : 1 / 1.1)); }, { passive: false });
  cv.addEventListener("keydown", e => {
    const f = ed.key(e.key, mods(e), e.timeStamp);
    if (after(f ? f | CHANGED : 0) & HANDLED) e.preventDefault();
    if (f & MENU) menu.openAtAnchor();
    if (f & (ASK_LABEL | ASK_TEXT)) { open(f & ASK_LABEL ? "charges" : "objects", true); const l = /** @type {HTMLInputElement} */ ($(f & ASK_LABEL ? "d2-label" : "d2-text-tool")); l.focus(); l.select(); }
  });
  // the key that hides every palette also works from a palette button or a handle (the canvas forwards its own keys above); the
  // module decides what the key does, a text field keeps its own keys
  document.addEventListener("keydown", e => {
    const n = e.target;
    if (e.key !== "\\" || e.defaultPrevented || n === cv || e.ctrlKey || e.metaKey || e.altKey) return;
    if (n instanceof HTMLElement && (n.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(n.tagName))) return;
    if (after(ed.key(e.key, mods(e), e.timeStamp)) & HANDLED) e.preventDefault();
  });
  addEventListener("resize", () => { menu.close(); layout(); });
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

/** one click on Save: the default format of the settings (`export.default`) @returns {void | Promise<void>} */
function saveDefault() {
  const what = JSON.parse(ed.export_settings()).default;
  return what === "png" ? savePng() : what === "pdf" ? savePdf() : save(what, ...FILES[what]);
}

/** downloads a text as a file @param {string} what @param {string} name @param {string} type */
function save(what, name, type) {
  const note = $("d2-ex-note"); note.textContent = "";
  try {
    download(ed.save(what), name, type);
    const lost = ed.maps_lost(what);   // the module says what the format cannot hold; nothing is dropped in silence
    if (lost) { note.textContent = t("draw2.export.mapsLost", { n: lost }); $("d2-live").textContent = note.textContent; } else open("saveas", false);
  } catch (e) { note.textContent = I.err(JSON.parse(String(e))).message; }
}

/** @param {BlobPart} data @param {string} name @param {string} type */
function download(data, name, type) {
  const a = document.createElement("a"), url = URL.createObjectURL(new Blob([data], { type }));
  a.href = url; a.download = name; document.body.append(a); a.click(); a.remove();
  URL.revokeObjectURL(url);
}

/** the PNG a plan asks for: its SVG drawn on a canvas of the size the module gives, on its paper colour @param {any} plan @returns {Promise<Blob>} */
async function drawPng(plan) {
  const img = new Image(), url = URL.createObjectURL(new Blob([plan.svg], { type: "image/svg+xml" }));
  await new Promise((ok, no) => { img.onload = ok; img.onerror = no; img.src = url; });
  const c = document.createElement("canvas"); c.width = plan.width; c.height = plan.height;
  const x = /** @type {CanvasRenderingContext2D} */ (c.getContext("2d"));
  x.fillStyle = plan.paper; x.fillRect(0, 0, c.width, c.height); x.drawImage(img, 0, 0, c.width, c.height);
  URL.revokeObjectURL(url);
  return /** @type {Blob} */ (await new Promise(ok => c.toBlob(ok, "image/png")));
}

/** downloads the PDF the module writes (bytes, not text) */
function savePdf() {
  const note = $("d2-ex-note"); note.textContent = "";
  try { download(ed.save_pdf(), "drawing.pdf", "application/pdf"); open("saveas", false); } catch (e) { note.textContent = I.err(JSON.parse(String(e))).message; }
}

/** saves a PNG: the SVG the module writes is drawn on a canvas of the size the module asks for; the module finishes the file */
async function savePng() {
  const note = $("d2-ex-note"); note.textContent = "";
  try {
    const blob = await drawPng(JSON.parse(ed.png_plan()));
    download(ed.png_finish(new Uint8Array(await blob.arrayBuffer())), "drawing.png", "image/png"); open("saveas", false);
  } catch (e) { note.textContent = I.err(JSON.parse(String(e))).message; }
}

/** copies the selection as a PNG (the context menu): drawn as above, finished by the module with the selection inside, given to the clipboard */
async function copyPng() {
  const blob = await drawPng(JSON.parse(ed.context_png_plan()));
  const png = ed.context_png_finish(new Uint8Array(await blob.arrayBuffer()));
  await navigator.clipboard.write([new ClipboardItem({ "image/png": new Blob([png], { type: "image/png" }) })]);
}

/** the style: the presets and the fields the module's schema names; a choice changes the screen and the saved pictures, not the
 * document. The settings file the module writes for the style is kept in the browser (OPFS) and given back to the module on the
 * next visit; it can be saved as a file and a settings file can be opened. The module judges every value and every file. */
/** keeps the settings file the module writes (the style, the layout of the palettes, the export defaults) in the browser (OPFS),
 * half a second after the last change; without storage they last for this visit */
let keeping = 0;
function keep() {
  clearTimeout(keeping);
  keeping = setTimeout(async () => {
    try {
      const w = await (await (await navigator.storage.getDirectory()).getFileHandle(NAMES.settings, { create: true })).createWritable();
      await w.write(ed.style()); await w.close();
      document.documentElement.dataset.d2Style = String(Number(document.documentElement.dataset.d2Style || 0) + 1);
    } catch { /* no storage: the settings last for this visit */ }
  }, 500);
}

async function styles() {
  const sel = /** @type {HTMLSelectElement} */ ($("d2-style")), note = $("d2-style-note"), NAME = NAMES.settings;
  const schema = JSON.parse(ed.settings_schema()), box = $("d2-style-fields");
  /** @type {Record<string, HTMLInputElement | HTMLSelectElement>} */ const inputs = {};
  for (const name of schema.presets) { const o = document.createElement("option"); o.value = name; o.textContent = t(`draw2.style.${name}`); sel.append(o); }
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
  $("d2-style-save").addEventListener("click", () => download(ed.style(), NAMES.settings, "application/json"));
  $("d2-style-open").addEventListener("click", () => $("d2-style-file").click());
  $("d2-style-file").addEventListener("change", async e => {
    const f = /** @type {HTMLInputElement} */ (e.target).files?.[0];
    if (f) { const text = await f.text(); ask(() => ed.set_style(text), t("draw2.style.opened.note"), null); }
    /** @type {HTMLInputElement} */ (e.target).value = "";
  });
  for (const name of [NAME, ...NAMES.settingsOld]) {       // the file of the name in use, else one an earlier name wrote
    try { ed.set_style(await (await (await (await navigator.storage.getDirectory()).getFileHandle(name)).getFile()).text()); break; } catch { /* nothing kept, or a file the module refuses */ }
  }
  show();
  note.textContent = t(`draw2.style.${sel.value}.note`);
}

function panels() {
  $("d2-std-list").addEventListener("click", e => {
    const b = /** @type {HTMLElement} */ (e.target).closest("button");
    if (!b) return;
    after(ed.select_check(b.dataset.key || "")); refresh();
    $("d2-live").textContent = t("draw2.std.selected", { n: JSON.parse(ed.state()).selected });
  });
  $("d2-add").addEventListener("click", () => add(/** @type {HTMLTextAreaElement} */ ($("d2-text")).value));
  $("d2-open").addEventListener("click", () => $("d2-file").click());
  $("d2-file").addEventListener("change", async e => { const f = /** @type {HTMLInputElement} */ (e.target).files?.[0]; if (f) add(new Uint8Array(await f.arrayBuffer())); });
  for (const [id, what] of [["molfile", "molfile"], ["v3000", "molfile-v3000"], ["sdf", "sdf"], ["smiles", "smiles"], ["ket", "ket"], ["cdxml", "cdxml"], ["cml", "cml"], ["rxn", "rxn"], ["trasf", "trasf"], ["svg", "svg"], ["doc", NAMES.slug]])
    $(`d2-ex-${id}`).addEventListener("click", () => save(what, ...FILES[what]));
  $("d2-ex-png").addEventListener("click", savePng);
  $("d2-ex-pdf").addEventListener("click", savePdf);
}

I.apply(document);
document.title = t("draw2.name");
try {
  ed = await load();
  /** @type {any} */ (window)[NAMES.global] = ed;
  build({ ed, t, run, field, focusCanvas: () => cv.focus({ preventScroll: true }), saveDefault, keep, area: place, say: (/** @type {string} */ s) => { $("d2-live").textContent = s; } });
  mapField();
  menu = contextMenu({ ed, t, cv, after, saveDefault, copyPng, say: (/** @type {string} */ s) => { $("d2-live").textContent = s; },
    pick: (/** @type {string} */ palette, /** @type {string} */ id) => { open(palette, true); const l = /** @type {HTMLInputElement} */ ($(id)); l.focus(); l.select(); } });
  events(); clipboardEvents(); panels(); await styles(); layout(); refresh();
  document.addEventListener("visibilitychange", () => { if (document.hidden) autosave.flush(); });
  await autosave.offer();
  document.documentElement.dataset.d2Ready = "1";
} catch (e) {
  $("d2-status").textContent = t("draw2.unavailable");
  document.documentElement.dataset.d2Ready = "0";
}
