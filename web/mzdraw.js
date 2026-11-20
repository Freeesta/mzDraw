// mzDraw page: forwards pointer, wheel and key events to the editor in the mzdraw .wasm and paints the display list it returns.
// No chemistry and no decision here: what a click does is decided in Rust (crates/mzdraw).

const I = /** @type {any} */ (window).I18N, t = (k, p = {}) => I.t(k, p);
const $ = id => /** @type {HTMLElement} */ (document.getElementById(id));
const REPAINT = 1, CHANGED = 2, TOOL = 4, HANDLED = 8, ASK_LABEL = 16, ASK_TEXT = 32, FOCUS = 64;
// toolbar: [command or tool name, key of the short label, key of the tooltip]; "|" is a separator
const TOOLS = ["select", "erase", "|", "bond:1", "bond:2", "bond:3", "bond:up", "bond:down", "bond:wavy", "|", "atom:C", "atom:N", "atom:O", "atom:S", "atom:P", "atom:F", "atom:Cl", "atom:Br", "atom:I", "atom:H", "|",
  "ring:6a", "ring:6", "ring:5", "|", "label", "|", "arrow", "plus", "text", "|", "charge:+", "charge:-", "|", "undo", "redo", "|", "copy", "cut", "paste", "duplicate", "|", "rotate-l", "rotate-r", "flip-h", "flip-v", "|", "zoom-in", "zoom-out", "fit", "|", "clear"];
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

/** the text of one check/1 warning, from the Drawing check catalog @param {any} w */
function warningText(w) {
  const allowed = (w.allowed || []).join(", ");
  return t(`draw.check.${w.code}.what`, { el: w.el || "", valence: w.valence ?? "", allowed, q: w.q ?? "", fix: w.fix ?? "", n: w.n ?? "", iso: w.iso ?? "", list: allowed });
}

function refresh() {
  const st = JSON.parse(ed.state());
  for (const b of $("d2-tools").querySelectorAll("button")) {
    const c = /** @type {HTMLElement} */ (b).dataset.cmd || "";
    if (c.includes(":") || ["select", "erase", "arrow", "plus"].includes(c)) b.setAttribute("aria-pressed", String(c === st.tool));
    if (c === "undo") /** @type {HTMLButtonElement} */ (b).disabled = !st.undo;
    if (c === "redo") /** @type {HTMLButtonElement} */ (b).disabled = !st.redo;
    if (c === "copy" || c === "cut" || c === "duplicate") /** @type {HTMLButtonElement} */ (b).disabled = !st.selected;
    if (c === "paste") /** @type {HTMLButtonElement} */ (b).disabled = !st.clip;
  }
  $("d2-outline").replaceChildren(...st.outline.map((/** @type {any} */ s, /** @type {number} */ i) => {
    const li = document.createElement("li");
    li.textContent = t("draw2.outline.piece", { n: i + 1, formula: s.formula, atoms: s.atoms, bonds: s.bonds });
    return li;
  }));
  if (!st.outline.length) { const li = document.createElement("li"); li.textContent = t("draw2.outline.empty"); $("d2-outline").append(li); }
  $("d2-focus").textContent = st.focus ? focusText(st.focus) : "";
  for (const [id, kind] of [["d2-label", "label:"], ["d2-text-tool", "text:"]]) {
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

/** a toolbar field that picks a tool with an argument: the label field (an abbreviation or an R group, with its list) and the text field
 * (a line of text to place, or to set on an arrow); Enter or a choice picks the tool @param {"label" | "text"} kind */
function field(kind) {
  const box = document.createElement("label"), input = document.createElement("input"), out = [box];
  box.className = "field"; box.append(t(`draw2.tool.${kind}`) + " ", input);
  input.id = kind === "label" ? "d2-label" : "d2-text-tool"; input.type = "text"; input.size = kind === "label" ? 7 : 12; input.autocomplete = "off"; input.spellcheck = false; input.maxLength = 200;
  input.title = t(`draw2.tool.${kind}.title`); input.placeholder = t(`draw2.tool.${kind}.hint`);
  if (kind === "label") {
    const list = document.createElement("datalist");
    list.id = "d2-labels"; input.setAttribute("list", list.id);
    for (const name of ed.labels()) { const o = document.createElement("option"); o.value = name; list.append(o); }
    out.push(list);
  }
  const pick = () => {
    const v = kind === "label" ? input.value.trim() : input.value;
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

function toolbar() {
  const bar = $("d2-tools");
  for (const c of TOOLS) {
    if (c === "|") { const s = document.createElement("span"); s.className = "sep"; s.setAttribute("role", "separator"); bar.append(s); continue; }
    if (c === "label" || c === "text") { bar.append(...field(c)); continue; }
    const b = document.createElement("button"), k = c.replace(":", ".").replace("+", "plus").replace("charge.-", "charge.minus").replace("zoom-in", "zoomIn").replace("zoom-out", "zoomOut");
    b.type = "button"; b.dataset.cmd = c; b.dataset.testid = "d2-" + c;
    b.textContent = c.startsWith("atom:") ? c.slice(5) : t(`draw2.tool.${k}`);
    b.title = t(`draw2.tool.${k}.title`); b.setAttribute("aria-label", b.title);
    // a pointer click gives the keyboard back to the canvas (shortcuts keep working); a keyboard activation stays on the button
    b.addEventListener("click", e => { run(c); if (e.detail) cv.focus({ preventScroll: true }); });
    bar.append(b);
  }
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
    if (f & (ASK_LABEL | ASK_TEXT)) { const l = /** @type {HTMLInputElement} */ ($(f & ASK_LABEL ? "d2-label" : "d2-text-tool")); l.focus(); l.select(); }
  });
  addEventListener("resize", size);
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", paint);
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

/** the style choice: the presets Rust names; the choice changes the screen and the saved SVG, not the document. The settings file the
 * module writes for the chosen style is kept in the browser (OPFS) and given back to the module on the next visit; a file the module
 * refuses is ignored. */
async function styles() {
  const sel = /** @type {HTMLSelectElement} */ ($("d2-style")), NAME = "mzdraw-settings.json";
  for (const name of ed.styles()) { const o = document.createElement("option"); o.value = name; o.textContent = t(`draw2.style.${name}`); sel.append(o); }
  const keep = async () => {
    try {
      const w = await (await (await navigator.storage.getDirectory()).getFileHandle(NAME, { create: true })).createWritable();
      await w.write(ed.style()); await w.close();
      document.documentElement.dataset.d2Style = String(Number(document.documentElement.dataset.d2Style || 0) + 1);
    } catch { /* no storage: the style lasts for this visit */ }
  };
  sel.addEventListener("change", () => {
    try { after(ed.set_style(sel.value) | CHANGED); $("d2-style-note").textContent = t(`draw2.style.${sel.value}.note`); $("d2-live").textContent = t("draw2.say.style", { what: sel.selectedOptions[0].textContent || "" }); keep(); }
    catch (e) { $("d2-style-note").textContent = I.err(JSON.parse(String(e))).message; }
  });
  try {
    ed.set_style(await (await (await (await navigator.storage.getDirectory()).getFileHandle(NAME)).getFile()).text());
    sel.value = JSON.parse(ed.state()).style;
    if (sel.selectedIndex < 0) sel.selectedIndex = 0;
  } catch { /* nothing kept, or a file the module refuses */ }
  $("d2-style-note").textContent = t(`draw2.style.${sel.value}.note`);
}

function panels() {
  $("d2-add").addEventListener("click", () => add(/** @type {HTMLTextAreaElement} */ ($("d2-text")).value));
  $("d2-open").addEventListener("click", () => $("d2-file").click());
  $("d2-file").addEventListener("change", async e => { const f = /** @type {HTMLInputElement} */ (e.target).files?.[0]; if (f) add(new Uint8Array(await f.arrayBuffer())); });
  $("d2-ex-molfile").addEventListener("click", () => save("molfile", "drawing.mol", "chemical/x-mdl-molfile"));
  $("d2-ex-v3000").addEventListener("click", () => save("molfile-v3000", "drawing-v3000.mol", "chemical/x-mdl-molfile"));
  $("d2-ex-smiles").addEventListener("click", () => save("smiles", "drawing.smi", "chemical/x-daylight-smiles"));
  $("d2-ex-ket").addEventListener("click", () => save("ket", "drawing.ket", "application/json"));
  $("d2-ex-cdxml").addEventListener("click", () => save("cdxml", "drawing.cdxml", "chemical/x-cdxml"));
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
  toolbar(); events(); clipboardEvents(); panels(); await styles(); size(); refresh();
  document.addEventListener("visibilitychange", () => { if (document.hidden) autosave.flush(); });
  await autosave.offer();
  document.documentElement.dataset.d2Ready = "1";
} catch (e) {
  $("d2-status").textContent = t("draw2.unavailable");
  document.documentElement.dataset.d2Ready = "0";
}
