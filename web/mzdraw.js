// mzDraw page: forwards pointer, wheel and key events to the editor in the mzdraw .wasm and paints the display list it returns.
// No chemistry and no decision here: what a click does is decided in Rust (crates/mzdraw).

const I = /** @type {any} */ (window).I18N, t = (k, p = {}) => I.t(k, p);
const $ = id => /** @type {HTMLElement} */ (document.getElementById(id));
const REPAINT = 1, CHANGED = 2, TOOL = 4, HANDLED = 8, ASK_LABEL = 16;
// toolbar: [command or tool name, key of the short label, key of the tooltip]; "|" is a separator
const TOOLS = ["select", "erase", "|", "bond:1", "bond:2", "bond:3", "bond:up", "bond:down", "bond:wavy", "|", "atom:C", "atom:N", "atom:O", "atom:S", "atom:P", "atom:F", "atom:Cl", "atom:Br", "atom:I", "atom:H", "|",
  "ring:6a", "ring:6", "ring:5", "|", "label", "|", "charge:+", "charge:-", "|", "undo", "redo", "|", "copy", "cut", "paste", "duplicate", "|", "rotate-l", "rotate-r", "flip-h", "flip-v", "|", "zoom-in", "zoom-out", "fit", "|", "clear"];
const COLOURS = ["--d2-ink", "--d2-select", "--d2-hover", "--d2-warn", "--d2-preview", "--d2-n", "--d2-o", "--d2-s", "--d2-x", "--d2-p"];

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

/** the text of one check/1 warning, from the Drawing check catalog @param {any} w */
function warningText(w) {
  const allowed = (w.allowed || []).join(", ");
  return t(`draw.check.${w.code}.what`, { el: w.el || "", valence: w.valence ?? "", allowed, q: w.q ?? "", fix: w.fix ?? "", n: w.n ?? "", iso: w.iso ?? "", list: allowed });
}

function refresh() {
  const st = JSON.parse(ed.state());
  for (const b of $("d2-tools").querySelectorAll("button")) {
    const c = /** @type {HTMLElement} */ (b).dataset.cmd || "";
    if (c.includes(":") || c === "select" || c === "erase") b.setAttribute("aria-pressed", String(c === st.tool));
    if (c === "undo") /** @type {HTMLButtonElement} */ (b).disabled = !st.undo;
    if (c === "redo") /** @type {HTMLButtonElement} */ (b).disabled = !st.redo;
    if (c === "copy" || c === "cut" || c === "duplicate") /** @type {HTMLButtonElement} */ (b).disabled = !st.selected;
    if (c === "paste") /** @type {HTMLButtonElement} */ (b).disabled = !st.clip;
  }
  $("d2-label").classList.toggle("on", st.tool.startsWith("label:"));
  $("d2-status").textContent = t("draw2.count", { atoms: st.atoms, bonds: st.bonds });
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
  document.documentElement.dataset.d2 = JSON.stringify({ atoms: st.atoms, bonds: st.bonds, tool: st.tool, warnings: ws.length, selected: st.selected });
}

/** acts on the flags an editor call returned @param {number} f */
function after(f) {
  if (f & REPAINT) paint();
  if (f & (CHANGED | TOOL)) refresh();
  return f;
}

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

/** the label field: an abbreviation or an R group picked from the list or typed; Enter or a choice picks the label tool */
function labelField() {
  const box = document.createElement("label"), input = document.createElement("input"), list = document.createElement("datalist");
  box.className = "field"; box.append(t("draw2.tool.label") + " ", input);
  input.id = "d2-label"; input.type = "text"; input.size = 7; input.autocomplete = "off"; input.spellcheck = false; input.setAttribute("list", "d2-labels");
  input.title = t("draw2.tool.label.title"); input.placeholder = "Ph, R1";
  list.id = "d2-labels";
  for (const name of ed.labels()) { const o = document.createElement("option"); o.value = name; list.append(o); }
  const pick = () => {
    const v = input.value.trim();
    if (!v) return;
    if (after(ed.command("label:" + v))) { $("d2-live").textContent = t("draw2.say.labelTool", { what: v }); cv.focus({ preventScroll: true }); }
    else $("d2-live").textContent = t("error.draw2.label");
  };
  input.addEventListener("change", pick);
  input.addEventListener("keydown", e => { if (e.key === "Enter") { e.preventDefault(); pick(); } else if (e.key === "Escape") cv.focus({ preventScroll: true }); });
  return [box, list];
}

function toolbar() {
  const bar = $("d2-tools");
  for (const c of TOOLS) {
    if (c === "|") { const s = document.createElement("span"); s.className = "sep"; s.setAttribute("role", "separator"); bar.append(s); continue; }
    if (c === "label") { bar.append(...labelField()); continue; }
    const b = document.createElement("button"), k = c.replace(":", ".").replace("+", "plus").replace("charge.-", "charge.minus").replace("zoom-in", "zoomIn").replace("zoom-out", "zoomOut");
    b.type = "button"; b.dataset.cmd = c; b.dataset.testid = "d2-" + c;
    b.textContent = c.startsWith("atom:") ? c.slice(5) : t(`draw2.tool.${k}`);
    b.title = t(`draw2.tool.${k}.title`); b.setAttribute("aria-label", b.title);
    // a pointer click gives the keyboard back to the canvas (shortcuts keep working); a keyboard activation stays on the button
    b.addEventListener("click", e => { run(c); if (e.detail) cv.focus({ preventScroll: true }); });
    bar.append(b);
  }
}

const mods = (/** @type {MouseEvent | KeyboardEvent} */ e) => (e.shiftKey ? 1 : 0) | (e.ctrlKey || e.metaKey ? 2 : 0);
const kind = (/** @type {PointerEvent} */ e) => (e.pointerType === "pen" ? 1 : e.pointerType === "touch" ? 2 : 0);

function events() {
  /** @type {{x: number, y: number} | null} */ let panning = null;
  const at = (/** @type {PointerEvent | WheelEvent} */ e) => { const r = cv.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
  cv.addEventListener("pointerdown", e => {
    cv.focus(); cv.setPointerCapture(e.pointerId);
    if (e.button === 1 || e.button === 2) { panning = { x: e.clientX, y: e.clientY }; e.preventDefault(); return; }
    const [x, y] = at(e); after(ed.pointer(0, x, y, kind(e), mods(e)));
  });
  cv.addEventListener("pointermove", e => {
    if (panning) { after(ed.pan(e.clientX - panning.x, e.clientY - panning.y)); panning = { x: e.clientX, y: e.clientY }; return; }
    const [x, y] = at(e); after(ed.pointer(1, x, y, kind(e), mods(e)));
  });
  const up = (/** @type {PointerEvent} */ e) => {
    if (panning) { panning = null; return; }
    const [x, y] = at(e); after(ed.pointer(2, x, y, kind(e), mods(e)) | CHANGED);          // a release may have changed the selection alone
  };
  cv.addEventListener("pointerup", up); cv.addEventListener("pointercancel", up);
  cv.addEventListener("contextmenu", e => e.preventDefault());
  cv.addEventListener("wheel", e => { e.preventDefault(); const [x, y] = at(e); after(ed.zoom(x, y, e.deltaY < 0 ? 1.1 : 1 / 1.1)); }, { passive: false });
  cv.addEventListener("keydown", e => {
    const f = ed.key(e.key, mods(e));
    if (after(f ? f | CHANGED : 0) & HANDLED) e.preventDefault();
    if (f & ASK_LABEL) { const l = /** @type {HTMLInputElement} */ ($("d2-label")); l.focus(); l.select(); }
  });
  addEventListener("resize", size);
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", paint);
}

/** adds a structure text to the drawing and says how it went @param {string} text */
function add(text) {
  const note = $("d2-note");
  try {
    const info = JSON.parse(ed.add(text));
    note.textContent = info.placeholder ? t("draw2.import.placeholder") : "";
    after(ed.fit(cv.clientWidth, cv.clientHeight) | CHANGED);
  } catch (e) { note.textContent = I.err(JSON.parse(String(e))).message; }
}

/** downloads a text as a file @param {string} what @param {string} name @param {string} type */
function save(what, name, type) {
  const a = document.createElement("a"), url = URL.createObjectURL(new Blob([ed.save(what)], { type }));
  a.href = url; a.download = name; document.body.append(a); a.click(); a.remove();
  URL.revokeObjectURL(url);
}

function panels() {
  $("d2-add").addEventListener("click", () => add(/** @type {HTMLTextAreaElement} */ ($("d2-text")).value));
  $("d2-open").addEventListener("click", () => $("d2-file").click());
  $("d2-file").addEventListener("change", async e => { const f = /** @type {HTMLInputElement} */ (e.target).files?.[0]; if (f) add(await f.text()); });
  $("d2-ex-molfile").addEventListener("click", () => save("molfile", "drawing.mol", "chemical/x-mdl-molfile"));
  $("d2-ex-svg").addEventListener("click", () => save("svg", "drawing.svg", "image/svg+xml"));
  $("d2-ex-mzdraw").addEventListener("click", () => save("mzdraw", "drawing.mzdraw.json", "application/json"));
}

I.apply(document);
document.title = t("draw2.name");
try {
  ed = await load();
  /** @type {any} */ (window).MZDRAW = ed;
  toolbar(); events(); clipboardEvents(); panels(); size(); refresh();
  document.documentElement.dataset.d2Ready = "1";
} catch (e) {
  $("d2-status").textContent = t("draw2.unavailable");
  document.documentElement.dataset.d2Ready = "0";
}
