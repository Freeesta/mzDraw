// End-to-end test of the page in Chromium (Playwright): the page loads its module, a benzene template lands with one click, undo and redo
// walk back and forth, atoms and bonds are drawn with the pointer, a five-bonded carbon raises the live valence mistake of check/1,
// charge, erase, select all + Delete and the keyboard shortcuts work, SMILES and molfile are added, molfile / SVG / document are saved by
// the Rust module, wedge bonds are drawn and turned around, the selection is copied, cut, pasted (system clipboard), duplicated, rotated
// and mirrored, and the same loop works on a tablet viewport with touch. No page error, console error or failed request at any point.
// usage: node tests/e2e.mjs            serves site/ (tools/build_site.sh) on a local port
//        node tests/e2e.mjs <URL>      tests a published site (the live check of pages.yml)
//        MZDRAW_SHOTS=<dir>            also writes screenshots at deviceScaleFactor 2
import { createRequire } from "node:module";
import http from "node:http";
import { readFile } from "node:fs/promises";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { NAMES } from "../web/names.js";

const { chromium } = createRequire(import.meta.url)("playwright");       // require, so that a global install (NODE_PATH) is found too
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
// a CDX file made of the example of the CDX documentation (dimethylamine: a nitrogen and two carbons, two bonds), as printed there byte by byte
const CDX = Buffer.from("566A4344303130300403020100000000000000000000000000000000018001000000038008000000048005000000000208000040A40000809400020402000700000004800600000000020800BA03AC004F7AB100000005800200000004060400050000000506040006000000000004800400000000020800BA03AC00B08577000000058003000000040604000500000005060400040000000000000000000000", "hex");
const TYPES = { ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".json": "application/json" };
let server = null, base = process.argv[2];
if (!base) {
  server = http.createServer(async (q, s) => {
    let p = decodeURIComponent(new URL(q.url || "/", "http://x").pathname);
    if (p.endsWith("/")) p += "index.html";
    const f = path.join(root, "site", path.normalize(p));
    try { const b = await readFile(f); s.writeHead(200, { "content-type": TYPES[path.extname(f)] || "application/octet-stream" }); s.end(b); }
    catch { s.writeHead(404); s.end(); }
  });
  await new Promise(r => server.listen(0, "127.0.0.1", () => r(null)));
  base = `http://127.0.0.1:${server.address().port}/`;
}
const shots = process.env.MZDRAW_SHOTS;
if (shots) mkdirSync(shots, { recursive: true });

const steps = [], errs = [];
const assert = (ok, what) => { if (!ok) throw new Error(typeof what === "string" ? what : JSON.stringify(what)); };
const eq = (a, b) => assert(JSON.stringify(a) === JSON.stringify(b), `${JSON.stringify(a)} != ${JSON.stringify(b)}`);
async function step(name, fn) {
  try { await fn(); steps.push([name, "ok"]); }
  catch (e) {
    // the first line says what failed; the call log of the page (what was waited for, what covered the target) and the line of this
    // file that threw say where, which is what a failure on a machine other than this one needs
    const lines = String(e && e.message || e).split("\n").map(l => l.trim()).filter(Boolean), at = String(e && e.stack || "").split("\n").find(l => l.includes("e2e.mjs"));
    steps.push([name, "FAIL " + lines.slice(0, 8).join(" | ").slice(0, 900) + (at ? " @ " + at.trim().replace(/^at /, "").replace(/^.*\//, "") : "")]);
  }
}
const STATE = "JSON.parse(document.documentElement.dataset.d2 || '{}')";

/** waits until a JS expression of the page is true (polled with evaluate: the page's CSP forbids the string eval of waitForFunction) */
async function until(pg, expr, ms = 10000) {
  for (const t0 = Date.now(); ;) {
    if (await pg.evaluate(expr)) return;
    if (Date.now() - t0 > ms) throw new Error("timeout: " + expr);
    await new Promise(r => setTimeout(r, 25));
  }
}
async function context(browser, opts) {
  const ctx = await browser.newContext({ acceptDownloads: true, ...opts }), pg = await ctx.newPage();
  pg.on("pageerror", e => errs.push(["pageerror", String(e)]));
  pg.on("console", m => { if (m.type() === "error") errs.push(["console.error", m.text()]); });
  pg.on("response", r => { if (r.status() >= 400) errs.push(["http " + r.status(), r.url()]); });
  pg.on("requestfailed", r => errs.push(["requestfailed", r.url()]));
  await pg.goto(base);
  await until(pg, "document.documentElement.dataset.d2Ready === '1'");
  const b = await pg.locator("#d2-canvas").boundingBox();
  return { ctx, pg, cx: b.x + b.width / 2, cy: b.y + b.height / 2 };
}
const state = pg => pg.evaluate(STATE);
/** opens, by keyboard, the flyout that holds an element when it is not in view: the focus on the button that opens it, then the
 * arrow key across its palette (or Enter on a button that is itself in a flyout); a flyout opened this way changes no tool */
async function reveal(pg, sel) {
  if (await pg.isVisible(sel)) return;
  const o = await pg.evaluate(s => {
    const f = document.querySelector(s).closest(".fly"), btn = document.querySelector(`[aria-controls="d2-fly-${f.dataset.fly}"]`), pal = btn.closest(".pal");
    const key = !pal ? "" : pal.dataset.orient === "h" ? (pal.dataset.dock === "bottom" ? "ArrowUp" : "ArrowDown") : (pal.dataset.dock === "right" ? "ArrowLeft" : "ArrowRight");
    return { opener: `[data-testid="${btn.dataset.testid}"]`, key: btn.closest(".fly") ? "Enter" : key };
  }, sel);
  await reveal(pg, o.opener);
  await pg.focus(o.opener); await pg.keyboard.press(o.key);
  await until(pg, `!!document.querySelector(${JSON.stringify(sel)}).offsetParent`);
}
const tool = async (pg, name) => { const sel = `[data-testid="d2-${name}"]`; await reveal(pg, sel); await pg.click(sel); };
const fillIn = async (pg, sel, text) => { await reveal(pg, sel); await pg.fill(sel, text); };
const saveAs = async (pg, sel) => { await reveal(pg, sel); await pg.click(sel); };
const want = (pg, kv) => until(pg, `(() => { const s = ${STATE}, w = ${JSON.stringify(kv)}; return Object.keys(w).every(k => s[k] === w[k]); })()`);
/** a screenshot of one element at deviceScaleFactor 2 (the drawing checks panel, light, dark and tablet: docs/shots/) */
const shotEl = async (pg, sel, name) => { if (shots) await pg.locator(sel).screenshot({ path: path.join(shots, name + ".png") }); };
// the palettes: where each one is drawn, as the page shows it
const RECTS = `Object.fromEntries([...document.querySelectorAll(".pal")].map(p => [p.dataset.pal, p.hidden ? null
  : (r => ({ x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height), dock: p.dataset.dock, orient: p.dataset.orient }))(p.getBoundingClientRect())]))`;
const rects = pg => pg.evaluate(RECTS);
/** shows or hides a palette from the Palettes menu by tapping (a tablet's preset keeps the side palettes hidden) */
const tapPalette = async (pg, id, show) => {
  if (!!(await rects(pg))[id] === show) return;
  await pg.tap('[data-testid="d2-palettes"]'); await pg.tap(`[data-testid="d2-menu-hidden-${id}"]`); await pg.tap('[data-testid="d2-palettes"]'); await frames(pg);
  assert(!!(await rects(pg))[id] === show, `the ${id} palette is ${show ? "shown" : "hidden"}`);
};
const frames = pg => pg.evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))");
/** opens or closes a palette `<details>` by its summary and waits for the page to say so (the palettes lay themselves out again after
 *  a toggle: a click made before that lands on what moved). On failure the message carries the state of every such element. */
async function setOpen(pg, sel, open) {
  const now = () => pg.evaluate(`document.querySelector(${JSON.stringify(sel)}).open`);
  try {
    if ((await now()) !== open) { await frames(pg); await pg.locator(sel + " summary").click({ timeout: 5000 }); }
    await until(pg, `document.querySelector(${JSON.stringify(sel)}).open === ${open}`, 5000);
    await frames(pg);
  } catch (e) {
    const state = await pg.evaluate(() => [...document.querySelectorAll("details")].map(d => [d.id, d.open, JSON.stringify(d.querySelector("summary").getBoundingClientRect())]));
    throw new Error(`${sel} did not become ${open ? "open" : "closed"}: ${String(e && e.message || e).split("\n")[0]} | details: ${JSON.stringify(state)}`);
  }
}
const center = async (pg, sel) => { const b = await pg.locator(sel).boundingBox(); return [b.x + b.width / 2, b.y + b.height / 2]; };
/** the shown palettes lie inside the viewport, no two docked ones overlap, and the canvas is in the area they leave free */
async function tidy(pg, w, h) {
  const all = Object.entries(await rects(pg)).filter(([, r]) => r), c = await pg.locator("#d2-canvas").boundingBox();
  eq(all.filter(([, r]) => r.x < 0 || r.y < 0 || r.x + r.w > w || r.y + r.h > h).map(([id]) => id), []);
  const cut = (a, b) => a.x < b.x + b.w - 1 && b.x < a.x + a.w - 1 && a.y < b.y + b.h - 1 && b.y < a.y + a.h - 1;
  const docked = all.filter(([, r]) => r.dock !== "float"), over = [];
  for (let i = 0; i < docked.length; i++) for (let j = i + 1; j < docked.length; j++) if (cut(docked[i][1], docked[j][1])) over.push([docked[i][0], docked[j][0]]);
  eq(over, []);
  eq(docked.filter(([, r]) => cut(r, { x: c.x, y: c.y, w: c.width, h: c.height })).map(([id]) => id), []);
}
// the keys the module binds today (Editor::key): a tooltip names no other key
const BUILT = new Set(["V", "E", "1", "2", "3", "W", "Shift+W", "Y", "C", "N", "O", "S", "P", "F", "H", "B", "I", "Shift+C", "Shift+B", "Shift+S", "+", "-", "L", "Shift+A", "T", "M",
  "Ctrl+Z", "Ctrl+Y", "Ctrl+Shift+Z", "Ctrl+X", "Ctrl+C", "Ctrl+V", "Ctrl+D", "Ctrl+A", "\\"]);
// WCAG 2 contrast of the theme tokens: a colour with alpha is laid over its background first
const rgba = c => { if (c[0] === "#") return [1, 3, 5].map(i => parseInt(c.slice(i, i + 2), 16)).concat(1); const m = c.match(/rgba?\(([\d.]+)[ ,]+([\d.]+)[ ,]+([\d.]+)(?:[ ,/]+([\d.]+))?\)/); return [+m[1], +m[2], +m[3], m[4] === undefined ? 1 : +m[4]]; };
const over = (f, b) => f.slice(0, 3).map((v, i) => v * f[3] + b[i] * (1 - f[3]));
const linear = c => c.map(v => { v /= 255; return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; });
const lum = c => { const [r, g, b] = linear(c); return 0.2126 * r + 0.7152 * g + 0.0722 * b; };
const contrast = (fg, bg) => { const b = rgba(bg), f = over(rgba(fg), b), [x, y] = [lum(f), lum(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
const lab = c => { const [r, g, b] = linear(rgba(c).slice(0, 3)), f = v => (v > 0.008856 ? Math.cbrt(v) : 7.787 * v + 16 / 116);
  const [x, y, z] = [(r * 0.4124 + g * 0.3576 + b * 0.1805) / 0.95047, r * 0.2126 + g * 0.7152 + b * 0.0722, (r * 0.0193 + g * 0.1192 + b * 0.9505) / 1.08883];
  return [116 * f(y) - 16, 500 * (f(x) - f(y)), 200 * (f(y) - f(z))]; };
const TOKENS = ["surface", "surface-raised", "paper", "border", "control", "ink", "ink-muted", "accent", "accent-soft", "hover", "selection", "focus", "danger", "preview", "atom-n", "atom-o", "atom-s", "atom-x", "atom-p"];
// [foreground, background, least ratio]: text 4.5:1, icons, borders of controls and the focus ring 3:1 (WCAG 2.2 AA, 1.4.3 and 1.4.11)
const PAIRS = [["ink", "surface", 4.5], ["ink", "surface-raised", 4.5], ["ink", "paper", 4.5], ["ink", "hover", 4.5], ["ink", "accent-soft", 4.5], ["ink", "selection", 4.5],
  ["ink-muted", "surface", 4.5], ["ink-muted", "surface-raised", 4.5], ["ink-muted", "paper", 4.5], ["ink-muted", "hover", 4.5], ["ink-muted", "accent-soft", 4.5], ["ink-muted", "selection", 4.5],
  ["danger", "surface", 4.5], ["danger", "surface-raised", 4.5], ["danger", "paper", 4.5], ["danger", "hover", 4.5], ["danger", "selection", 4.5],
  ["control", "surface", 3], ["control", "surface-raised", 3], ["control", "hover", 3],
  ["focus", "surface", 3], ["focus", "surface-raised", 3], ["focus", "paper", 3], ["focus", "accent-soft", 3], ["focus", "hover", 3], ["focus", "selection", 3],
  ["accent", "accent-soft", 3], ["accent", "surface", 3], ["accent", "selection", 3], ["preview", "paper", 3],
  // an atom colour is read on the paper, on the chrome, and on the halo of a selection (the label is drawn over it)
  ...["atom-n", "atom-o", "atom-s", "atom-x", "atom-p"].flatMap(a => [[a, "paper", 4.5], [a, "surface", 4.5], [a, "surface-raised", 4.5], [a, "selection", 4.5], [a, "hover", 4.5]])];
/** the canvas, as a user sees it: whether an opaque pixel of the colour `want` ([r, g, b]) lies within `rad` CSS pixels of each point
 * [x, y] of the canvas (read back from the 2D context), for the points of the display list that the colour index `col` paints */
const inkAt = (pg, col, ops, want) => pg.evaluate(([col, ops, want]) => {
  const d = Array.from(window.MZDRAW.scene()), cv = document.querySelector("#d2-canvas"), g = cv.getContext("2d"), dpr = devicePixelRatio || 1;
  const found = (x, y, rad) => {
    const r = Math.max(1, Math.round(rad * dpr)), x0 = Math.max(0, Math.round(x * dpr) - r), y0 = Math.max(0, Math.round(y * dpr) - r);
    const w = Math.min(2 * r + 1, cv.width - x0), h = Math.min(2 * r + 1, cv.height - y0);
    if (w < 1 || h < 1) return false;
    const im = g.getImageData(x0, y0, w, h).data;
    for (let i = 0; i < im.length; i += 4) if (im[i + 3] > 250 && Math.abs(im[i] - want[0]) <= 1 && Math.abs(im[i + 1] - want[1]) <= 1 && Math.abs(im[i + 2] - want[2]) <= 1) return true;
    return false;
  };
  let n = 0, hit = 0;
  for (let i = 0; i < d.length;) {
    const op = d[i];
    if (op === 1) { if (d[i + 6] === col && ops.includes(1)) { n++; if (found((d[i + 1] + d[i + 3]) / 2, (d[i + 2] + d[i + 4]) / 2, 2)) hit++; } i += 7; }
    else if (op === 2) { if (d[i + 4] === col && ops.includes(2)) { n++; if (found(d[i + 1], d[i + 2], d[i + 3] / 2)) hit++; } i += 7; }
    else if (op === 3) i += 6;
    else if (op === 6) i += 8;
    else { if (d[i + 4] === col && ops.includes(op)) { n++; if (found(d[i + 1], d[i + 2], d[i + 3] * 0.8)) hit++; } i += 5; }
  }
  return [n, hit];
}, [col, ops, want]);
/** the colour of the page at a point, from a screenshot (the paper under the canvas is CSS, not canvas ink) */
const pixelAt = async (pg, x, y) => {
  const png = await pg.screenshot({ clip: { x, y, width: 2, height: 2 } });
  return pg.evaluate(async b64 => {
    const bytes = Uint8Array.from(atob(b64), c => c.charCodeAt(0)), bm = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
    const c = document.createElement("canvas"); c.width = bm.width; c.height = bm.height; const g = c.getContext("2d"); g.drawImage(bm, 0, 0);
    return Array.from(g.getImageData(0, 0, 1, 1).data).slice(0, 3);
  }, png.toString("base64"));
};
const shot = async (pg, name) => { if (shots) await pg.screenshot({ path: path.join(shots, name + ".png") }); };

/** the PDF round trip: acetic acid is drawn, the PDF is downloaded the way `get` says (keyboard or touch), it is a vector PDF with the
 * text as text and the document inside, and Open a file brings back the same drawing; a PDF of another program is refused in words */
async function pdfRoundTrip(pg, get) {
  await tool(pg, "clear"); await want(pg, { atoms: 0 });
  await pg.setInputFiles("#d2-file", { name: "acetic.smi", mimeType: "text/plain", buffer: Buffer.from("CC(=O)O") }); await want(pg, { atoms: 4, bonds: 3 });   // the file field is in every layout
  const [p] = await Promise.all([pg.waitForEvent("download"), get()]);
  eq(p.suggestedFilename(), "drawing.pdf");
  const pdf = readFileSync(await p.path()), text = pdf.toString("latin1");
  eq(text.slice(0, 8), "%PDF-1.7");
  assert(text.endsWith("%%EOF\n") && text.includes("/BaseFont /Helvetica") && text.includes("(O) Tj") && text.includes("(H) Tj") && text.includes("mzdraw.json") && text.includes('"format":"mzdraw/1"'), "a vector PDF: text as text, the document inside");
  assert(!text.includes("/Image") && !text.includes("/CreationDate"), "no picture and no date");
  eq(pdf.toString("base64"), await pg.evaluate("(() => { const b = window.MZDRAW.save('pdf'); return b; })()"));
  if (shots) writeFileSync(path.join(shots, "drawing.pdf"), pdf);
  await tool(pg, "clear"); await want(pg, { atoms: 0 });
  await pg.setInputFiles("#d2-file", { name: "drawing.pdf", mimeType: "application/pdf", buffer: pdf }); await want(pg, { atoms: 4, bonds: 3 });
  eq(await pg.evaluate("window.MZDRAW.save('smiles')"), "CC(=O)O");
  eq(await pg.innerText("#d2-note"), "");
  const plain = Buffer.from("%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n");
  await pg.setInputFiles("#d2-file", { name: "other.pdf", mimeType: "application/pdf", buffer: plain });
  await until(pg, "document.querySelector('#d2-note').textContent.length > 0");
  eq(await pg.innerText("#d2-note"), await pg.evaluate("I18N.t('error.draw2.pdfPlain')"));
  eq((await state(pg)).atoms, 4);
  return pdf;
}

/** the drawing loop every pointer kind must manage: template, undo/redo, atom, sprouted bonds, the valence mistake, erase */
async function loop(pg, cx, cy, tap, name) {
  await tool(pg, "ring:6a"); await tap(cx, cy); await want(pg, { atoms: 6, bonds: 6, warnings: 0 });
  await tool(pg, "undo"); await want(pg, { atoms: 0, bonds: 0 });
  assert(await pg.isDisabled('[data-testid="d2-undo"]') && await pg.isEnabled('[data-testid="d2-redo"]'), "undo off, redo on");
  await tool(pg, "redo"); await want(pg, { atoms: 6, bonds: 6 });
  await tool(pg, "atom:C"); await tap(cx - 220, cy - 150); await want(pg, { atoms: 7, bonds: 6 });
  await tool(pg, "bond:1");
  for (let n = 1; n < 6; n++) { await tap(cx - 220, cy - 150); await want(pg, { atoms: 7 + n, bonds: 6 + n }); }
  await until(pg, "!!document.querySelector('#d2-warnings li[data-code=valence]')");       // five bonds on a carbon: the mistake shows without pressing anything
  assert((await pg.innerText("#d2-warnings li[data-code=valence]")).includes(await pg.evaluate("I18N.t('draw.check.level.warn')")), "the level in words, not only a colour");
  await shot(pg, name);
  await tool(pg, "erase"); await tap(cx - 220, cy - 150); await want(pg, { atoms: 11, bonds: 6 });      // the atom goes with its five bonds
  await until(pg, "!document.querySelector('#d2-warnings li[data-code=valence]')");
}

const browser = await chromium.launch();
try {
  let { ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 });

  await step("the page loads the module and starts empty on the single-bond tool", async () => {
    eq(await state(pg), { focus: 0, atoms: 0, bonds: 0, objects: 0, tool: "bond:1", style: "mzlab", warnings: 0, std: 0, selected: 0 });
    eq(await pg.evaluate("window.MZDRAW.version()"), "mzdraw/1");
    eq(await pg.innerText("#d2-summary"), await pg.evaluate("I18N.t('draw.check.empty')"));
    assert(await pg.locator("#d2-palettes .items button, #d2-flys button[data-cmd]").count() >= 26, "the palettes");
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-palettes .items button, #d2-flys button[data-cmd], #d2-flys button[data-opens]')].filter(b => !b.title || b.title.length > 90 || !b.getAttribute('aria-label')).map(b => b.dataset.testid)"), []);      // every tool says in one short sentence what it does
    eq(await pg.evaluate("[...document.querySelectorAll('button')].filter(b => b.offsetParent && (b.offsetWidth < 24 || b.offsetHeight < 24)).map(b => b.id || b.dataset.cmd)"), []);
    eq(await pg.evaluate("[...I18N.missing || []]"), []);
  });

  await step("theme: the token pairs reach WCAG AA in light and in dark, the atom colours stay apart, data-theme and the system setting agree, the focus ring shows", async () => {
    // its own browser context: the drawing it makes and selects is not part of the history of the steps that follow
    const { ctx: own, pg } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 });
    try {
    const tokens = () => pg.evaluate(([names, prefix]) => { const c = getComputedStyle(document.documentElement); return Object.fromEntries(names.map(n => [n, c.getPropertyValue(prefix + n).trim()])); }, [TOKENS, NAMES.tokens]);
    const sets = {};
    // a small structure with every element colour in it, all selected: the canvas shows ink, the halo of the selection and the atom colours
    await pg.fill("#d2-text", "NC(=O)C(S)(P)Cl"); await pg.click("#d2-add"); await want(pg, { atoms: 7, bonds: 6 });
    const area = await pg.locator("#d2-canvas").boundingBox(), spot = [area.x + area.width - 30, area.y + area.height - 30];
    // five bonds on one carbon: the mistake the page marks in the colour of a warning
    await tool(pg, "atom:C"); await pg.mouse.click(area.x + 150, area.y + area.height - 150); await tool(pg, "bond:1");
    for (let n = 0; n < 5; n++) await pg.mouse.click(area.x + 150, area.y + area.height - 150);
    await want(pg, { atoms: 13, bonds: 11 });
    await until(pg, "!!document.querySelector('#d2-warnings li[data-code=valence]')");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+a"); await want(pg, { selected: 24 });
    for (const theme of ["light", "dark"]) {
      await pg.evaluate(t => { document.documentElement.dataset.theme = t; }, theme);
      const t = sets[theme] = await tokens();
      eq(TOKENS.filter(n => !t[n]), []);
      eq(PAIRS.map(([f, b, min]) => [theme, f, b, Math.round(contrast(t[f], t[b]) * 100) / 100, min]).filter(r => r[3] < r[4]), []);
      // the colours of the elements, and the ink of carbon, stay apart from each other (CIE76 distance of at least 20)
      const atoms = ["ink", "atom-n", "atom-o", "atom-s", "atom-x", "atom-p"], near = [];
      for (let i = 0; i < atoms.length; i++) for (let j = i + 1; j < atoms.length; j++) {
        const [a, b] = [lab(t[atoms[i]]), lab(t[atoms[j]])];
        if (Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]) < 20) near.push([theme, atoms[i], atoms[j]]);
      }
      eq(near, []);
      // ink is never pure black on light nor pure white on dark; the paper of the canvas differs from the chrome
      assert(!["#000000", "#ffffff"].includes(t.ink.toLowerCase()) && t.paper !== t.surface, "calm ink, paper apart from the chrome");
      // no yellow field: every surface is near neutral (CIELAB b* under 12)
      eq(["surface", "surface-raised", "paper", "hover", "accent-soft", "selection"].filter(n => lab(t[n])[2] >= 12).map(n => [theme, n]), []);
      // the canvas is painted with the theme's colours: what the 2D context holds is read back and compared with the tokens (a theme that
      // did not repaint, or a canvas that read the wrong variable, would show the other theme's pixels)
      await pg.evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))");
      const rgb = n => rgba(t[n]).slice(0, 3).map(Math.round);
      eq(await pixelAt(pg, ...spot), rgb("paper"));
      for (const [what, col, ops, token] of [["ink", 0, [1], "ink"], ["the halo of the selection", 1, [4], "selection"], ["nitrogen", 5, [2], "atom-n"], ["oxygen", 6, [2], "atom-o"], ["sulfur", 7, [2], "atom-s"],
        ["chlorine", 8, [2], "atom-x"], ["phosphorus", 9, [2], "atom-p"], ["the mistake", 3, [4, 5], "danger"]]) {
        const [n, hit] = await inkAt(pg, col, ops, rgb(token));
        assert(n > 0 && hit === n, `${theme}: ${what} is drawn in ${token} (${hit} of ${n} marks)`);
      }
      // the focus ring: drawn, in the focus colour, on every surface
      await pg.focus('[data-testid="d2-erase"]'); await pg.keyboard.press("ArrowUp");
      const ring = await pg.evaluate("(s => [s.outlineStyle, s.outlineWidth, s.outlineColor])(getComputedStyle(document.activeElement))");
      assert(ring[0] !== "none" && parseFloat(ring[1]) >= 2, "the focus ring " + ring);
      assert(["surface", "surface-raised", "paper", "selection", "hover"].every(n => contrast(ring[2], t[n]) >= 3), "the focus ring stands out " + ring[2]);
      await shot(pg, "theme-" + theme);
    }
    await pg.evaluate("delete document.documentElement.dataset.theme");
    await pg.emulateMedia({ colorScheme: "dark" }); eq(await tokens(), sets.dark);
    await pg.emulateMedia({ colorScheme: "light" }); eq(await tokens(), sets.light);
    await pg.emulateMedia({ colorScheme: "dark" }); await pg.evaluate("document.documentElement.dataset.theme = 'light'"); eq(await tokens(), sets.light);       // the host's choice wins
    await pg.evaluate("delete document.documentElement.dataset.theme"); await pg.emulateMedia({ colorScheme: "light" });
    } finally { await own.close(); }
  });

  await step("mouse: template, undo and redo, atom, bonds, live valence mistake, erase", () => loop(pg, cx, cy, (x, y) => pg.mouse.click(x, y), "desktop"));

  await step("a bond is dragged out of an atom; a charge is set with a click; an atom is selected", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0, bonds: 0 });
    await tool(pg, "atom:N"); await pg.mouse.click(cx, cy); await want(pg, { atoms: 1 });
    await tool(pg, "bond:2");
    await pg.mouse.move(cx, cy); await pg.mouse.down(); await pg.mouse.move(cx + 40, cy + 5, { steps: 4 }); await pg.mouse.move(cx + 90, cy, { steps: 4 }); await pg.mouse.up();
    await want(pg, { atoms: 2, bonds: 1 });
    await tool(pg, "charge:+"); await pg.mouse.click(cx, cy);
    await until(pg, "window.MZDRAW.save('molfile').includes('M  CHG')");
    await tool(pg, "select"); await pg.mouse.click(cx, cy); await want(pg, { selected: 1 });
  });

  await step("keyboard: select all, Delete, undo and redo, tool shortcuts", async () => {
    await pg.focus("#d2-canvas");
    await pg.keyboard.press("Control+a"); await want(pg, { selected: 3 });
    await pg.keyboard.press("Delete"); await want(pg, { atoms: 0, bonds: 0 });
    await pg.keyboard.press("Control+z"); await want(pg, { atoms: 2, bonds: 1 });
    await pg.keyboard.press("Control+y"); await want(pg, { atoms: 0 });
    await pg.keyboard.press("Control+z"); await want(pg, { atoms: 2 });
    await pg.keyboard.press("Control+Shift+Z"); await want(pg, { atoms: 0 });
    await pg.keyboard.press("e"); await want(pg, { tool: "erase" });
    await pg.keyboard.press("2"); await want(pg, { tool: "bond:2" });
    await pg.keyboard.press("o"); await want(pg, { tool: "atom:O" });
    await pg.keyboard.press("Escape"); await want(pg, { tool: "select" });
    eq(await pg.getAttribute('[data-testid="d2-select"]', "aria-pressed"), "true");
  });

  await step("SMILES and molfile are added, a broken text is refused in words, an import is one undo step", async () => {
    await tool(pg, "clear");
    await pg.fill("#d2-text", "CC(=O)O"); await pg.click("#d2-add"); await want(pg, { atoms: 4, bonds: 3, warnings: 0 });
    const mol = await pg.evaluate("window.MZDRAW.save('molfile')");
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.fill("#d2-text", mol); await pg.click("#d2-add"); await want(pg, { atoms: 4, bonds: 3 });
    eq(await pg.innerText("#d2-note"), "");                 // a molfile brings its own coordinates
    await pg.fill("#d2-text", "C1CC"); await pg.click("#d2-add");
    await until(pg, "document.querySelector('#d2-note').textContent.length > 0");
    eq(await pg.innerText("#d2-note"), await pg.evaluate("I18N.t('error.draw2.smiles')"));       // the sentence alone, no "Error:" in front
    eq((await state(pg)).atoms, 4);                         // a refused text changes nothing
    await tool(pg, "undo"); await want(pg, { atoms: 0 });  // one import is one undo step
    await tool(pg, "redo"); await want(pg, { atoms: 4 });
  });

  await step("molfile (V2000 and V3000), SMILES, SVG, PNG and the mzdraw/1 document are saved as the module writes them; the pictures open again", async () => {
    const got = {};
    for (const [button, kind, name] of [["molfile", "molfile", "drawing.mol"], ["v3000", "molfile-v3000", "drawing-v3000.mol"], ["smiles", "smiles", "drawing.smi"], ["ket", "ket", "drawing.ket"], ["cdxml", "cdxml", "drawing.cdxml"], ["cml", "cml", "drawing.cml"],
      ["svg", "svg", "drawing.svg"], ["mzdraw", "mzdraw", "drawing.mzdraw.json"]]) {
      const [d] = await Promise.all([pg.waitForEvent("download"), saveAs(pg, `#d2-ex-${button}`)]);
      eq(d.suggestedFilename(), name);
      got[kind] = readFileSync(await d.path(), "utf8");
      eq(got[kind], await pg.evaluate(`window.MZDRAW.save('${kind}')`));
    }
    assert(got.molfile.includes("V2000") && got.molfile.trimEnd().endsWith("M  END"), "a V2000 molfile");
    assert(got["molfile-v3000"].includes("V3000") && got["molfile-v3000"].includes("M  V30 COUNTS 4 3"), "a V3000 molfile");
    eq(got.smiles, "CC(=O)O");
    assert(got.cdxml.startsWith('<?xml version="1.0"') && got.cdxml.includes("<CDXML ") && (got.cdxml.match(/<n /g) || []).length === 4 && (got.cdxml.match(/<b /g) || []).length === 3, "a CDXML file");
    assert(got.cml.includes('<cml xmlns="http://www.xml-cml.org/schema"') && (got.cml.match(/<atom /g) || []).length === 4 && (got.cml.match(/<bond /g) || []).length === 3, "a CML file");
    const ket = JSON.parse(got.ket);
    assert(ket.root.nodes[0].$ref === "mol0" && ket.mol0.type === "molecule" && ket.mol0.atoms.length === 4 && ket.mol0.bonds.length === 3, "a KET file");
    assert(got.svg.startsWith("<svg") && got.svg.includes("<line") && got.svg.replaceAll("OH", "O").includes(">O<"), "an SVG with lines and labels");
    assert(got.svg.includes('role="img"') && /<title id="t-[0-9a-f]{8}">C2H4O2<\/title>/.test(got.svg) && got.svg.includes('xml:lang="en"'), "the SVG has a text alternative: the formula as its title");
    // pictures that open again: the SVG and the PNG carry the document; a PNG is 300 dpi of the SVG, drawn by the browser
    const [p] = await Promise.all([pg.waitForEvent("download"), saveAs(pg, "#d2-ex-png")]);
    eq(p.suggestedFilename(), "drawing.png");
    const png = readFileSync(await p.path()), plan = JSON.parse(await pg.evaluate("window.MZDRAW.png_plan()"));
    eq(png.subarray(1, 4).toString("latin1"), "PNG");
    if (shots) writeFileSync(path.join(shots, "drawing.png"), png);       // the saved file itself, next to the screenshots
    await shot(pg, "pictures");
    eq([png.readUInt32BE(16), png.readUInt32BE(20)].join("x"), `${plan.width}x${plan.height}`);
    assert(png.includes("pHYs") && png.includes("iTXt") && png.includes('"format":"mzdraw/1"'), "resolution and document are in the PNG");
    const ink = await pg.evaluate(async bytes => {
      const img = await createImageBitmap(new Blob([new Uint8Array(bytes)], { type: "image/png" })), c = new OffscreenCanvas(img.width, img.height), x = c.getContext("2d");
      x.drawImage(img, 0, 0); const d = x.getImageData(0, 0, img.width, img.height).data; let dark = 0, clear = 0;
      for (let i = 0; i < d.length; i += 4) { if (d[i + 3] < 255) clear++; if (d[i] + d[i + 1] + d[i + 2] < 300) dark++; }
      return { dark, clear, n: d.length / 4 };
    }, [...png]);
    assert(ink.clear === 0 && ink.dark > 200 && ink.dark < ink.n / 4, `white paper with ink on it (${JSON.stringify(ink)})`);
    for (const [name, mimeType, buffer] of [["drawing.svg", "image/svg+xml", Buffer.from(got.svg)], ["drawing.png", "image/png", png], ["drawing.ket", "application/json", Buffer.from(got.ket)], ["drawing.cdxml", "chemical/x-cdxml", Buffer.from(got.cdxml)], ["drawing.cml", "chemical/x-cml", Buffer.from(got.cml)]]) {
      await tool(pg, "clear"); await want(pg, { atoms: 0 });
      await pg.setInputFiles("#d2-file", { name, mimeType, buffer }); await want(pg, { atoms: 4, bonds: 3 });
      eq(await pg.evaluate("window.MZDRAW.save('smiles')"), "CC(=O)O");
      eq(await pg.innerText("#d2-note"), "");
    }
    await pg.setInputFiles("#d2-file", { name: "other.svg", mimeType: "image/svg+xml", buffer: Buffer.from('<svg xmlns="http://www.w3.org/2000/svg"><circle r="4"/></svg>') });
    await until(pg, "document.querySelector('#d2-note').textContent.length > 0");
    eq(await pg.innerText("#d2-note"), await pg.evaluate("I18N.t('error.draw2.svgPlain')"));
    eq((await state(pg)).atoms, 4);
    const doc = JSON.parse(got.mzdraw);
    assert(doc.format === "mzdraw/1" && doc.atoms.length === 4 && doc.bonds.length === 3, "an mzdraw/1 document");
  });

  await step("formats: an SDF adds every record, R groups come back, what is out of scope is listed in words, the files are read back", async () => {
    const before = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    // three synthetic records: ethanol, a ring with a numbered R group, a chain with a query atom, a query bond and an S-group
    await pg.fill("#d2-text", "CCO"); await pg.click("#d2-add"); await want(pg, { atoms: 3 });
    const ethanol = await pg.evaluate("window.MZDRAW.save('molfile')");
    const ring = ethanol.replace("M  END", "M  RGP  1   3   5\nM  END").split("\n").map((l, i) => i === 6 ? l.slice(0, 31) + "R# " + l.slice(34) : l).join("\n");
    const query = ethanol.replace("M  END", "M  STY  1   1 SUP\nM  END").split("\n")
      .map((l, i) => i === 4 ? l.slice(0, 31) + "A  " + l.slice(34) : i === 8 ? l.slice(0, 6) + "  6" + l.slice(9) : l).join("\n");
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.fill("#d2-text", [ethanol, ring, query].join("\n$$$$\n") + "\n$$$$\n"); await pg.click("#d2-add");
    await want(pg, { atoms: 9, bonds: 6 });
    const say = k => pg.evaluate(([key, n]) => globalThis.I18N.t(key, { n }), k);
    eq(await pg.innerText("#d2-note"), [await say(["draw2.import.records", 3]), await say(["draw2.import.queryAtoms", 1]), await say(["draw2.import.queryBonds", 1]),
      await say(["draw2.import.sgroups", 1])].join(" "));
    assert((await pg.innerText("#d2-note")).includes("3 records"), "the count is in the sentence");
    const doc = JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')"));
    eq(doc.atoms.map(a => a.lab || "").join(","), ",,,,,R5,R,,");
    await shot(pg, "formats");
    // each form written is read back to the same SMILES: by the Add field for the molfiles, as a file for the SMILES
    const smiles = await pg.evaluate("window.MZDRAW.save('smiles')");
    eq(smiles, "CCO.CC[*:5].[*]CO");
    for (const kind of ["molfile", "molfile-v3000"]) {
      const text = await pg.evaluate(`window.MZDRAW.save('${kind}')`);
      await tool(pg, "clear"); await want(pg, { atoms: 0 });
      await pg.fill("#d2-text", text); await pg.click("#d2-add"); await want(pg, { atoms: 9, bonds: 6 });
      eq(await pg.evaluate("window.MZDRAW.save('smiles')"), smiles);
      eq(await pg.innerText("#d2-note"), "");
    }
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.setInputFiles("#d2-file", { name: "three.smi", mimeType: "text/plain", buffer: Buffer.from("CCO ethanol\nCC[*:5] ring\n[*]CO query\n") });
    await want(pg, { atoms: 9, bonds: 6 });
    eq(await pg.evaluate("window.MZDRAW.save('smiles')"), smiles);
    eq(await pg.innerText("#d2-note"), await say(["draw2.import.placeholder", 1]));
    // the data of a structure: the data fields of an SDF record are read, shown, edited, undone and written
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.fill("#d2-text", `${ethanol}\n>  <name>\nethanol (synthetic)\n\n>  <note>\nline one\nline two\n\n$$$$\n${ethanol}\n$$$$\n`); await pg.click("#d2-add");
    await want(pg, { atoms: 6, bonds: 4 });
    await setOpen(pg, "#d2-data-more", true);
    const rows = () => pg.evaluate("[...document.querySelectorAll('#d2-data fieldset')].map(f => [...f.querySelectorAll('.d2-datum')].map(r => [r.children[0].value, r.children[1].value]))");
    eq(await rows(), [[["name", "ethanol (synthetic)"], ["note", "line one\nline two"]], []]);
    const first = "#d2-data fieldset:nth-of-type(1)", second = "#d2-data fieldset:nth-of-type(2)";
    await pg.fill(`${first} .d2-datum:nth-of-type(1) textarea`, "ethanol, absolute");
    await pg.click(`${first} button:nth-of-type(1)`);    // Add a field
    await pg.keyboard.type("purity"); await pg.keyboard.press("Tab"); await pg.keyboard.type("99.8 %");       // by keyboard
    await pg.click(`${first} button:nth-of-type(2)`);    // Apply
    await until(pg, "document.querySelector('#d2-data-note').textContent.length > 0");
    eq(await pg.innerText("#d2-data-note"), await pg.evaluate("I18N.t('draw2.data.done', { n: 3 })"));
    eq(await rows(), [[["name", "ethanol, absolute"], ["note", "line one\nline two"], ["purity", "99.8 %"]], []]);
    await shot(pg, "data");
    const [sd] = await Promise.all([pg.waitForEvent("download"), saveAs(pg, "#d2-ex-sdf")]);
    eq(sd.suggestedFilename(), "drawing.sdf");
    const sdfOut = readFileSync(await sd.path(), "utf8");
    eq(sdfOut, await pg.evaluate("window.MZDRAW.save('sdf')"));
    assert(sdfOut.split("$$$$\n").length === 3 && sdfOut.includes("M  END\n>  <name>\nethanol, absolute\n\n>  <note>\nline one\nline two\n\n>  <purity>\n99.8 %\n\n$$$$\n"), "an SDF of two records, the first with its three fields");
    // a name an SDF could not hold is refused in words and nothing changes; emptying a name takes the field away; undo brings it back
    await pg.fill(`${second} .d2-datum:nth-of-type(1) input`, "x").catch(() => { });
    await pg.click(`${second} button:nth-of-type(1)`); await pg.keyboard.type("a<b"); await pg.keyboard.press("Tab"); await pg.keyboard.type("v");
    await pg.click(`${second} button:nth-of-type(2)`);
    await until(pg, "document.querySelector('#d2-data-note').textContent === I18N.t('error.draw2.data')");
    eq(await pg.evaluate("window.MZDRAW.save('sdf')"), sdfOut);
    await pg.fill(`${first} .d2-datum:nth-of-type(2) input`, ""); await pg.click(`${first} button:nth-of-type(2)`);
    await until(pg, "document.querySelector('#d2-data-note').textContent === I18N.t('draw2.data.done', { n: 2 })");
    eq((await rows())[0], [["name", "ethanol, absolute"], ["purity", "99.8 %"]]);
    await tool(pg, "undo");
    await until(pg, "document.querySelectorAll('#d2-data fieldset:nth-of-type(1) .d2-datum').length === 3");
    eq(await pg.evaluate("window.MZDRAW.save('sdf')"), sdfOut);
    // the saved file opens again with its data
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.setInputFiles("#d2-file", { name: "drawing.sdf", mimeType: "chemical/x-mdl-sdfile", buffer: Buffer.from(sdfOut) }); await want(pg, { atoms: 6, bonds: 4 });
    eq((await rows())[0], [["name", "ethanol, absolute"], ["note", "line one\nline two"], ["purity", "99.8 %"]]);
    await setOpen(pg, "#d2-data-more", false);
    // the annotation of a structure: a confidence level of the scale, an identifier, evidence; drawn, saved, read back, undone
    await setOpen(pg, "#d2-ann-more", true);
    const af = "#d2-ann fieldset:nth-of-type(1)", lv = `${af} .d2-annf:nth-of-type(2) select`, sc = `${af} .d2-annf:nth-of-type(1) select`;
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-ann fieldset:nth-of-type(1) .d2-annf:nth-of-type(2) select option')].map(o => o.value)"), ["", "1", "2a", "2b", "3", "4", "5"]);
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-ann fieldset:nth-of-type(1) .d2-annf:nth-of-type(1) select option')].map(o => o.value)"), ["schymanski-2014", "charbonnet-2022"]);
    eq(await pg.innerText("#d2-ann-kinds"), await pg.evaluate("I18N.t('draw2.ann.kinds', { list: JSON.parse(window.MZDRAW.ann_schema()).kinds.join(', ') })"));
    await pg.selectOption(lv, "2b");
    await pg.fill(`${af} input`, "ethanol (synthetic)");
    await pg.focus(`${af} .d2-annf:nth-of-type(4) textarea`); await pg.keyboard.type("diagnostic-fragment: m/z 31.0178"); await pg.keyboard.press("Enter"); await pg.keyboard.type("rt-standard");   // by keyboard
    await pg.click(`${af} button`);
    await until(pg, "document.querySelector('#d2-ann-note').textContent === I18N.t('draw2.ann.done', { level: '2b' })");
    const levels = () => pg.evaluate("JSON.parse(window.MZDRAW.state()).outline.map(s => s.ann && [s.ann.level, s.ann.identifier, s.ann.evidence, s.ann.entries])");
    eq(await levels(), [["2b", "ethanol (synthetic)", "diagnostic-fragment: m/z 31.0178\nrt-standard", 1], null]);
    assert((await pg.innerText("#d2-outline li:nth-of-type(1)")).endsWith(await pg.evaluate("I18N.t('draw2.outline.level', { level: '2b' })")), "the level is said in the list of what is drawn");
    assert((await pg.evaluate("window.MZDRAW.strings()")).includes("L2b"), "the level is painted under its structure");
    await shot(pg, "annotation");
    const annSdf = await pg.evaluate("window.MZDRAW.save('sdf')");
    assert(annSdf.includes(">  <MZLAB_LEVEL>\n2b\n\n>  <MZLAB_IDENTIFIER>\nethanol (synthetic)\n\n>  <MZLAB_EVIDENCE>\ndiagnostic-fragment: m/z 31.0178\nrt-standard\n\n>  <MZLAB_ANN>\n[{"), "the annotation is in the SDF");
    assert((await pg.evaluate("window.MZDRAW.save('svg')")).includes(">L2b</text>"), "and in the picture");
    // a level that is not one word is refused in words and nothing changes; a second entry is a history; undo goes back one entry
    await pg.fill(`${af} .d2-annf:nth-of-type(4) textarea`, "a b: c"); await pg.click(`${af} button`);
    await until(pg, "document.querySelector('#d2-ann-note').textContent === I18N.t('error.draw2.ann')");
    eq(await pg.evaluate("window.MZDRAW.save('sdf')"), annSdf);
    await pg.fill(`${af} .d2-annf:nth-of-type(4) textarea`, "rt-standard: synthetic standard"); await pg.selectOption(lv, "1"); await pg.click(`${af} button`);
    await until(pg, "document.querySelector('#d2-ann-note').textContent === I18N.t('draw2.ann.done', { level: '1' })");
    eq((await levels())[0], ["1", "ethanol (synthetic)", "rt-standard: synthetic standard", 2]);
    eq(await pg.innerText(`${af} p.hint`), await pg.evaluate("I18N.t('draw2.ann.history', { n: 2 })"));
    await tool(pg, "undo");
    await until(pg, "JSON.parse(window.MZDRAW.state()).outline[0].ann.level === '2b'");
    eq(await pg.evaluate("window.MZDRAW.save('sdf')"), annSdf);
    // the SDF opens again with the annotation; a level of another scale in a file is kept and offered as it is written
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.fill("#d2-text", annSdf.replace("<MZLAB_LEVEL>\n2b", "<MZLAB_LEVEL>\n3a").replace(/>  <MZLAB_ANN>\n.*\n\n/, "")); await pg.click("#d2-add"); await want(pg, { atoms: 6, bonds: 4 });
    eq((await levels())[0], ["3a", "ethanol (synthetic)", "diagnostic-fragment: m/z 31.0178\nrt-standard", 1]);
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-ann fieldset:nth-of-type(1) .d2-annf:nth-of-type(2) select option')].map(o => o.value)"), ["", "1", "2a", "2b", "3", "4", "5", "3a"]);
    eq(await pg.inputValue(lv), "3a");
    // the PFAS scale, chosen with the keyboard: the level list follows the scale, the entry says its scale
    await pg.focus(sc); await pg.keyboard.press("ArrowDown");
    eq(await pg.inputValue(sc), "charbonnet-2022");
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-ann fieldset:nth-of-type(1) .d2-annf:nth-of-type(2) select option')].map(o => o.value)"), ["", "1a", "1b", "2a", "2b", "2c", "3a", "3b", "3c", "3d", "4", "5a", "5b"]);
    await pg.focus(lv); await pg.keyboard.press("Home"); for (let k = 0; k < 5; k++) await pg.keyboard.press("ArrowDown");
    eq(await pg.inputValue(lv), "2c");
    await pg.click(`${af} button`);
    await until(pg, "document.querySelector('#d2-ann-note').textContent === I18N.t('draw2.ann.done', { level: '2c' })");
    eq(await pg.evaluate("(a => a.scale + '/' + a.known)(JSON.parse(window.MZDRAW.state()).outline[0].ann)"), "charbonnet-2022/true");
    assert((await pg.innerText("#d2-outline li:nth-of-type(1)")).endsWith(await pg.evaluate("I18N.t('draw2.outline.level', { level: '2c' })")), "the PFAS level is said in the list of what is drawn");
    assert((await pg.evaluate("window.MZDRAW.strings()")).includes("L2c"), "and painted");
    assert((await pg.evaluate("window.MZDRAW.save('sdf')")).includes('"scale":"charbonnet-2022"'), "the scale is in the SDF");
    await pg.selectOption(lv, ""); await pg.click(`${af} button`);
    await until(pg, "document.querySelector('#d2-ann-note').textContent === I18N.t('draw2.ann.gone', { level: '' })");
    eq(await levels(), [null, null]);
    await setOpen(pg, "#d2-ann-more", false);
    // labels that do not crowd: the bond of an O(-) stops after the minus sign, and starts earlier without the charge
    const bondStart = async (/** @type {string} */ first) => {
      await tool(pg, "clear"); await want(pg, { atoms: 0 });
      await pg.fill("#d2-text", `{"format":"mzdraw/1","next":4,"atoms":[{"id":1,${first},"x":0.0,"y":0.0},{"id":2,"el":"C","x":1.0,"y":0.0}],"bonds":[{"id":3,"a":1,"b":2,"o":1}]}`); await pg.click("#d2-add"); await want(pg, { atoms: 2, bonds: 1 });
      return pg.evaluate("(() => { const d = window.MZDRAW.scene(), s = window.MZDRAW.strings(), out = { line: 0, minus: 0, size: 0 }; for (let i = 0; i < d.length;) { if (d[i] === 1) out.line = Math.min(d[i + 1], d[i + 3]); if (d[i] === 2 && s[d[i + 5]] === '\u2212') { out.minus = d[i + 1]; out.size = d[i + 3]; } i += d[i] === 1 || d[i] === 2 ? 7 : d[i] === 3 ? 6 : d[i] === 6 ? 8 : 5; } return out; })()");
    };
    const bare = await bondStart('"el":"O"'), anion = await bondStart('"el":"O","chg":-1');
    assert(anion.size > 0 && anion.line >= anion.minus + anion.size * 0.5, `the bond starts after the minus sign: ${JSON.stringify(anion)}`);
    assert(anion.line > bare.line + 1, "and later than without a charge");
    // the NH between two bonds (the bottom of a ring) has its H under the N; a Cl keeps its bond clear of both letters
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.fill("#d2-text", '{"format":"mzdraw/1","next":8,"atoms":[{"id":1,"el":"N","x":0.0,"y":0.0},{"id":2,"el":"C","x":-0.866,"y":0.5},{"id":3,"el":"C","x":0.866,"y":0.5},{"id":4,"el":"Cl","x":1.732,"y":0.0}],"bonds":[{"id":5,"a":1,"b":2,"o":1},{"id":6,"a":1,"b":3,"o":1},{"id":7,"a":3,"b":4,"o":1}]}'); await pg.click("#d2-add"); await want(pg, { atoms: 4, bonds: 3 });
    const placed = await pg.evaluate("(() => { const d = window.MZDRAW.scene(), s = window.MZDRAW.strings(), out = {}; let far = 0; for (let i = 0; i < d.length;) { if (d[i] === 1) far = Math.max(far, d[i + 1], d[i + 3]); if (d[i] === 2) out[s[d[i + 5]]] = [d[i + 1], d[i + 2], d[i + 3]]; i += d[i] === 1 || d[i] === 2 ? 7 : d[i] === 3 ? 6 : d[i] === 6 ? 8 : 5; } out.far = far; return out; })()");
    assert(placed.H[1] > placed.N[1] + placed.N[2] * 0.6 && Math.abs(placed.H[0] + placed.H[2] * 0.35 - placed.N[0]) < 1, `the H stands under the N: ${JSON.stringify(placed)}`);
    assert(placed.far <= placed.Cl[0] - placed.Cl[2] * 0.45, `the bond stops before the Cl: ${JSON.stringify(placed)}`);
    await shot(pg, "crowding");
    // a reaction file: read with its roles (reactant, arrow, product), saved again; a drawing without an arrow is refused in words
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.fill("#d2-text", `$RXN\n\n  synthetic\n\n  1  1\n$MOL\n${ethanol}\n$MOL\n${ethanol}\n`); await pg.click("#d2-add");
    await want(pg, { atoms: 6, bonds: 4, objects: 1 });
    eq(await pg.innerText("#d2-note"), "");
    const [r] = await Promise.all([pg.waitForEvent("download"), saveAs(pg, "#d2-ex-rxn")]);
    eq(r.suggestedFilename(), "drawing.rxn");
    const rxn = readFileSync(await r.path(), "utf8");
    eq(rxn, await pg.evaluate("window.MZDRAW.save('rxn')"));
    assert(rxn.startsWith("$RXN\n") && rxn.includes("\n  1  1\n$MOL\n") && rxn.split("$MOL\n").length === 3, "a reaction file with one reactant and one product");
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.setInputFiles("#d2-file", { name: "drawing.rxn", mimeType: "chemical/x-mdl-rxnfile", buffer: Buffer.from(rxn) }); await want(pg, { atoms: 6, bonds: 4, objects: 1 });
    await tool(pg, "clear"); await pg.fill("#d2-text", "CCO"); await pg.click("#d2-add"); await want(pg, { atoms: 3 });
    await saveAs(pg, "#d2-ex-rxn");
    await until(pg, "document.querySelector('#d2-ex-note').textContent.length > 0");
    eq(await pg.innerText("#d2-ex-note"), await pg.evaluate("I18N.t('error.draw2.rxnArrow')"));
    await saveAs(pg, "#d2-ex-smiles"); await until(pg, "document.querySelector('#d2-ex-note').textContent === ''");       // the next save clears the note
    // back to what the next steps expect
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.fill("#d2-text", before); await pg.click("#d2-add"); await want(pg, { atoms: 4, bonds: 3 });
  });

  await step("zoom, fit and the wheel change the view, not the document; the canvas has ink", async () => {
    const before = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    for (const t of ["zoom-in", "zoom-in", "zoom-out", "fit"]) await tool(pg, t);
    await pg.mouse.move(cx, cy); await pg.mouse.wheel(0, -300);
    await pg.evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))");
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), before);       // the view never changes the document
    const ink = await pg.evaluate("(() => { const c = document.querySelector('#d2-canvas'), d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data; let n = 0; for (let i = 3; i < d.length; i += 4) if (d[i]) n++; return n; })()");
    assert(ink > 200, "ink " + ink);
    await shot(pg, "acetic-acid");
  });

  await step("wedge tools, rotate and flip, copy, cut, paste and duplicate by keyboard and by buttons", async () => {
    const doc = async () => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')"));
    /** per bond: the position of its narrow-end atom in the atom list, and its mark (ids go on after a cleared canvas) */
    const marks = async () => { const d = await doc(); return d.bonds.map(b => [d.atoms.findIndex(a => a.id === b.a), b.s || 0]); };
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "bond:up"); await pg.mouse.click(cx, cy); await want(pg, { atoms: 2, bonds: 1, tool: "bond:up" });
    eq(await marks(), [[0, 1]]);
    assert((await pg.evaluate("window.MZDRAW.save('molfile')")).includes("  1  2  1  1"), "the molfile carries the wedge");
    assert((await pg.evaluate("window.MZDRAW.save('svg')")).includes("<polygon"), "the SVG paints the wedge");
    await pg.mouse.click(cx + 17, cy - 10);                 // the middle of the bond: the same wedge again turns it around
    await until(pg, "(() => { const d = JSON.parse(window.MZDRAW.save('mzdraw')); return d.bonds[0].a === d.atoms[1].id; })()");
    assert((await pg.evaluate("window.MZDRAW.save('molfile')")).includes("  2  1  1  1"), "the narrow end is written first");
    await pg.focus("#d2-canvas");
    await pg.keyboard.press("Shift+W"); await want(pg, { tool: "bond:down" });
    await pg.mouse.click(cx, cy); await want(pg, { atoms: 3, bonds: 2 });      // a hashed bond sprouts from the atom, narrow end there
    eq(await marks(), [[1, 1], [0, 2]]);
    await pg.keyboard.press("y"); await want(pg, { tool: "bond:wavy" });
    assert(await pg.isDisabled('[data-testid="d2-copy"]') && await pg.isDisabled('[data-testid="d2-paste"]'), "nothing selected, nothing copied yet");
    // keyboard: the system clipboard carries the document and a molfile
    await pg.keyboard.press("Control+a"); await want(pg, { selected: 5 });
    await pg.keyboard.press("Control+c");
    await until(pg, "!document.querySelector('[data-testid=\"d2-paste\"]').disabled");
    await pg.keyboard.press("Control+v"); await want(pg, { atoms: 6, bonds: 4, tool: "select", selected: 5 });
    eq((await doc()).bonds.map(b => b.s || 0), [1, 2, 1, 2]);
    await pg.keyboard.press("Control+x"); await want(pg, { atoms: 3, bonds: 2, selected: 0 });
    await pg.keyboard.press("Control+z"); await want(pg, { atoms: 6 });
    await pg.keyboard.press("Control+z"); await want(pg, { atoms: 3 });
    // buttons: duplicate, rotate, flip; each is one undo step
    await pg.keyboard.press("Control+a");
    await tool(pg, "duplicate"); await want(pg, { atoms: 6, bonds: 4, selected: 5 });
    const before = await doc();
    await tool(pg, "rotate-l");
    const turned = await doc();
    assert(turned.atoms[3].x !== before.atoms[3].x && turned.atoms[0].x === before.atoms[0].x, "only the selected copy turns");
    await tool(pg, "flip-h"); await tool(pg, "flip-v");
    await tool(pg, "undo"); await tool(pg, "undo"); await tool(pg, "undo");
    eq(await doc(), before);
    await pg.keyboard.press("Control+d"); await want(pg, { atoms: 9, bonds: 6 });
    await tool(pg, "copy"); await tool(pg, "paste"); await want(pg, { atoms: 12, bonds: 8 });
    await shot(pg, "wedges");
    // a text from another program is pasted as a structure; a text that is none says why
    await pg.evaluate("(() => { const d = new DataTransfer(); d.setData('text/plain', 'C[C@H](N)O'); document.dispatchEvent(new ClipboardEvent('paste', { clipboardData: d, bubbles: true, cancelable: true })); })()");
    await want(pg, { atoms: 16, bonds: 11, selected: 7 });
    assert((await doc()).bonds.slice(8).some(b => b.s), "the stereo of the SMILES arrives as a wedge");
    await pg.evaluate("(() => { const d = new DataTransfer(); d.setData('text/plain', 'not a structure ((('); document.dispatchEvent(new ClipboardEvent('paste', { clipboardData: d, bubbles: true, cancelable: true })); })()");
    await until(pg, "document.querySelector('#d2-live').textContent === I18N.t('error.draw2.smiles')");
    eq((await state(pg)).atoms, 16);
  });

  await step("labels: an abbreviation and an R group are chosen, typed, refused in words, drawn and expanded in the molfile", async () => {
    const labs = async () => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')")).atoms.map(a => a.lab || "");
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    assert(await pg.locator("#d2-labels option").count() >= 30, "the list of labels");
    await fillIn(pg, "#d2-label", "Ph"); await pg.press("#d2-label", "Enter"); await want(pg, { tool: "label:Ph" });
    assert(await pg.evaluate("document.activeElement === document.querySelector('#d2-canvas')"), "the keyboard is back on the canvas");
    await pg.mouse.click(cx, cy); await want(pg, { atoms: 1, bonds: 0 });
    await tool(pg, "bond:1"); await pg.mouse.click(cx, cy); await want(pg, { atoms: 2, bonds: 1, warnings: 0 });
    await pg.keyboard.press("l");                             // L moves the keyboard to the label field
    assert(await pg.evaluate("document.activeElement === document.querySelector('#d2-label')"), "L focuses the label field");
    await pg.keyboard.type("R1"); await pg.keyboard.press("Enter"); await want(pg, { tool: "label:R1" });
    await pg.mouse.click(cx + 35, cy - 20); await want(pg, { atoms: 2, bonds: 1 });
    eq(await labs(), ["Ph", "R1"]);
    const mol = await pg.evaluate("window.MZDRAW.save('molfile')");
    assert(mol.includes("  7  7  0") && mol.includes(" R# ") && mol.includes("M  RGP  1   2   1"), "phenyl expanded to six atoms and six ring bonds, R1 as an R# atom");
    const svg = await pg.evaluate("window.MZDRAW.save('svg')");
    assert(svg.includes(">Ph<") && svg.includes(">R<") && svg.includes(">1<"), "the labels are drawn as text");
    await fillIn(pg, "#d2-label", "Nope"); await pg.press("#d2-label", "Enter");
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('error.draw2.label')"));
    eq((await state(pg)).tool, "label:R1");                 // a refused label changes nothing
    // the refused entry stays to be corrected, marked and selected; typing takes the mark away; a good entry is taken
    eq(await pg.getAttribute("#d2-label", "aria-invalid"), "true");
    eq(await pg.evaluate("(() => { const i = document.querySelector('#d2-label'); return i.value.slice(i.selectionStart, i.selectionEnd); })()"), "Nope");
    await shot(pg, "label-refused");
    await pg.keyboard.type("R2"); eq(await pg.getAttribute("#d2-label", "aria-invalid"), null);
    await pg.keyboard.press("Enter"); await want(pg, { tool: "label:R2" });
    await fillIn(pg, "#d2-label", "R1"); await pg.press("#d2-label", "Enter"); await want(pg, { tool: "label:R1" });
    await fillIn(pg, "#d2-label", "Nope"); await pg.press("#d2-label", "Enter");
    await tool(pg, "atom:N");                              // another tool: the field does not keep what was refused
    eq(await pg.inputValue("#d2-label"), ""); eq(await pg.getAttribute("#d2-label", "aria-invalid"), null);
    await pg.mouse.click(cx + 35, cy - 20);                // an element takes the label away
    eq(await labs(), ["Ph", ""]);
    await tool(pg, "undo"); eq(await labs(), ["Ph", "R1"]);
    await shot(pg, "labels");
  });

  await step("labels: a group on the left of its bond is written reversed (CO2H as HO2C), by keyboard and by pointer; the bond on its other side keeps it as written", async () => {
    const labs = async () => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')")).atoms.map(a => a.lab || "");
    const drawn = () => pg.evaluate("Array.from(window.MZDRAW.strings())");       // the display list the canvas paints
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "bond:1"); await pg.mouse.click(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    await pg.keyboard.press("l");                             // the label by keyboard: L, the text, Enter
    await pg.keyboard.type("CO2H"); await pg.keyboard.press("Enter"); await want(pg, { tool: "label:CO2H" });
    await pg.mouse.click(cx, cy);     // the first atom: its bond leaves to the right
    eq(await labs(), ["CO2H", ""]);
    eq((await drawn()).filter(t => ["HO", "2", "C", "CO", "H"].includes(t)), ["HO", "2", "C"]);
    const svg = await pg.evaluate("window.MZDRAW.save('svg')");
    assert(svg.indexOf(">HO<") > 0 && svg.indexOf(">HO<") < svg.indexOf(">2<") && svg.indexOf(">2<") < svg.indexOf(">C<") && !svg.includes(">CO<"), "the SVG says HO2C, as the canvas does");
    await shot(pg, "label-reversal");
    // the same group on the atom the bond leaves to the left: as written
    await tool(pg, "undo"); eq(await labs(), ["", ""]);
    await fillIn(pg, "#d2-label", "CO2H"); await pg.press("#d2-label", "Enter"); await want(pg, { tool: "label:CO2H" });
    await pg.mouse.click(cx + 35, cy - 20);                   // the second atom: its bond leaves to the left
    eq(await labs(), ["", "CO2H"]);
    eq((await drawn()).filter(t => ["HO", "2", "C", "CO", "H"].includes(t)), ["CO", "2", "H"]);
    await shot(pg, "label-written");
  });

  await step("a reaction: arrow by click and by drag, plus, text typed and set on the arrow, selected, moved, copied, erased", async () => {
    const objs = async () => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')")).objects || [];
    await tool(pg, "clear"); await want(pg, { atoms: 0, objects: 0 });
    await tool(pg, "fit");                                    // an empty canvas: 40 px per bond length again
    await tool(pg, "arrow"); await want(pg, { tool: "arrow" });
    eq(await pg.getAttribute('[data-testid="d2-arrow"]', "aria-pressed"), "true");
    await pg.mouse.click(cx, cy); await want(pg, { objects: 1 });
    await pg.mouse.move(cx, cy + 80); await pg.mouse.down(); await pg.mouse.move(cx + 60, cy + 84, { steps: 4 }); await pg.mouse.move(cx + 120, cy + 86, { steps: 4 }); await pg.mouse.up();
    await want(pg, { objects: 2 });
    let o = await objs();
    eq([o[0].t, o[0].x2 - o[0].x, o[1].y2], ["arrow", 2, o[1].y]);       // a click: two bond lengths; a nearly level drag: levelled
    assert(o[1].x2 - o[1].x > 2.2, "the dragged arrow has the length of the drag");
    await tool(pg, "plus"); await pg.mouse.click(cx - 120, cy); await want(pg, { objects: 3 });
    await pg.focus("#d2-canvas"); await pg.keyboard.press("t");
    assert(await pg.evaluate("document.activeElement === document.querySelector('#d2-text-tool')"), "T focuses the text field");
    await pg.keyboard.type("H2, Pd/C"); await pg.keyboard.press("Enter"); await want(pg, { tool: "text:H2, Pd/C" });
    await pg.mouse.click(cx + 40, cy);                         // on the arrow: its text
    await until(pg, "JSON.parse(window.MZDRAW.save('mzdraw')).objects[0].text === 'H2, Pd/C'");
    await pg.mouse.click(cx, cy - 120); await want(pg, { objects: 4 });       // on an empty spot: a text of its own
    eq(await pg.innerText("#d2-status"), await pg.evaluate("I18N.t('draw2.count.objects', { atoms: 0, bonds: 0, objects: 4 })"));
    const svg = await pg.evaluate("window.MZDRAW.save('svg')");
    assert(svg.split(">H2, Pd/C<").length === 3 && svg.includes(">+<") && svg.split("<polygon").length === 3, "arrows, plus and texts are in the SVG");
    await shot(pg, "reaction");
    // select the plus, move it, copy and paste it, erase the copy
    await tool(pg, "select"); await pg.mouse.click(cx - 120, cy); await want(pg, { selected: 1 });
    await pg.mouse.move(cx - 120, cy); await pg.mouse.down(); await pg.mouse.move(cx - 100, cy, { steps: 3 }); await pg.mouse.move(cx - 80, cy, { steps: 3 }); await pg.mouse.up();
    const plus = (await objs())[2];
    assert(plus.x - o[0].x > -120 / 40 + 0.5 && plus.y === o[0].y, "the plus moved to the right with the drag");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+c"); await pg.keyboard.press("Control+v"); await want(pg, { objects: 5, selected: 1 });
    await pg.keyboard.press("Delete"); await want(pg, { objects: 4 });
    await tool(pg, "erase"); await pg.mouse.click(cx + 60, cy + 80); await want(pg, { objects: 3 });
    await fillIn(pg, "#d2-text-tool", "   "); await pg.press("#d2-text-tool", "Enter");
    eq((await state(pg)).tool, "erase");                    // an empty text picks nothing
    await tool(pg, "undo"); await want(pg, { objects: 4 });
  });

  await step("style: a preset or your own values change the screen and the saved SVG, in points, and not the document; the settings file is saved and opened", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0, objects: 0, style: "mzlab" });
    await pg.fill("#d2-text", "C[C@H](N)C(=O)O"); await pg.click("#d2-add"); await want(pg, { atoms: 6 });
    eq(await pg.locator("#d2-style option").allInnerTexts(), await pg.evaluate("window.MZDRAW.styles().map(n => I18N.t('draw2.style.' + n))"));
    const doc = await pg.evaluate("window.MZDRAW.save('mzdraw')"), plain = await pg.evaluate("window.MZDRAW.save('svg')");
    const ink = () => pg.evaluate("(() => { const c = document.querySelector('#d2-canvas'), d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data; let n = 0; for (let i = 3; i < d.length; i += 4) if (d[i]) n++; return n; })()");
    // the width of the first line of the display list, in pixels, and the size of its first text
    const lineWidth = () => pg.evaluate("(() => { const d = window.MZDRAW.scene(); for (let i = 0; i < d.length;) { if (d[i] === 1) return d[i + 5]; i += d[i] === 1 || d[i] === 2 ? 7 : d[i] === 3 ? 6 : d[i] === 6 ? 8 : 5; } return 0; })()");
    const fontSize = () => pg.evaluate("(() => { const d = window.MZDRAW.scene(); for (let i = 0; i < d.length;) { if (d[i] === 2) return d[i + 3]; i += d[i] === 1 || d[i] === 2 ? 7 : d[i] === 3 ? 6 : d[i] === 6 ? 8 : 5; } return 0; })()");
    const before = { ink: await ink(), line: await lineWidth(), font: await fontSize() };
    assert(plain.includes('stroke-width="1.35"') && /width="[0-9.]+pt"/.test(plain), "the default file: lines of 1.35 pt, a size in points");
    await pg.selectOption("#d2-style", "acs-1996"); await want(pg, { style: "acs-1996", atoms: 6 });
    eq(await pg.innerText("#d2-style-note"), await pg.evaluate("I18N.t('draw2.style.acs-1996.note')"));
    const acs = await pg.evaluate("window.MZDRAW.save('svg')");
    assert(acs.includes('stroke-width="0.6"') && acs.includes('font-size="10"') && !acs.includes("#2040c0"), "ACS 1996: 0.6 pt lines, 10 pt labels, black only");
    // on the screen at the same zoom: ACS lines are thinner and its labels larger relative to the bond, and the canvas is repainted
    const now = { ink: await ink(), line: await lineWidth(), font: await fontSize() };
    assert(now.line < before.line && now.line >= 1, `thinner lines on the screen too: ${now.line} < ${before.line}`);
    assert(now.font > before.font, `larger labels relative to the bond: ${now.font} > ${before.font}`);
    assert(now.ink !== before.ink && now.ink > 200, "the canvas is repainted in the style");
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), doc);       // the style is not in the document
    assert(await pg.isDisabled('[data-testid="d2-redo"]') && JSON.parse(await pg.evaluate("window.MZDRAW.style()")).style.preset === "acs-1996", "a style is no undo step; it reads back as a settings file");
    await shot(pg, "style-acs");
    const [d] = await Promise.all([pg.waitForEvent("download"), saveAs(pg, "#d2-ex-svg")]);
    eq(readFileSync(await d.path(), "utf8"), acs);          // the saved file is in the chosen style
    await pg.focus("#d2-style"); await pg.keyboard.press("ArrowDown"); await want(pg, { style: "slides" });       // by keyboard
    await pg.selectOption("#d2-style", "mzlab"); await want(pg, { style: "mzlab" });
    eq(await pg.evaluate("window.MZDRAW.save('svg')"), plain);
    assert(await pg.evaluate("(() => { try { window.MZDRAW.set_style('nope'); return false; } catch (e) { return JSON.parse(String(e)).error_key === 'error.draw2.style'; } })()"), "a style that is none is refused with its key");
    eq((await state(pg)).style, "mzlab");
    // your own values: the fields come from the module's schema; the module judges each value
    await pg.click("#d2-style-more summary");
    const fields = await pg.evaluate("[...document.querySelectorAll('#d2-style-fields input, #d2-style-fields select')].map(i => i.id.replace('d2-style-', ''))");
    eq(fields, JSON.parse(await pg.evaluate("window.MZDRAW.settings_schema()")).fields.map(f => f.key).concat(["colour"]));
    eq(await pg.inputValue("#d2-style-line_width"), "1.35");
    await pg.fill("#d2-style-line_width", "3"); await pg.press("#d2-style-line_width", "Enter");
    await until(pg, "JSON.parse(window.MZDRAW.style()).style.line_width === 3");
    assert((await pg.evaluate("window.MZDRAW.save('svg')")).includes('stroke-width="3"'), "the saved SVG has the new line width");
    eq(await pg.innerText("#d2-style-note"), await pg.evaluate("I18N.t('draw2.style.custom.note')"));
    await pg.fill("#d2-style-line_width", "999"); await pg.press("#d2-style-line_width", "Enter");
    await until(pg, "document.querySelector('#d2-style-line_width').getAttribute('aria-invalid') === 'true'");
    eq(await pg.innerText("#d2-style-note"), await pg.evaluate("I18N.t('error.draw2.styleValue')"));
    eq(JSON.parse(await pg.evaluate("window.MZDRAW.style()")).style.line_width, 3);       // a refused value changes nothing
    await pg.selectOption("#d2-style-colour", "mono");
    await until(pg, "JSON.parse(window.MZDRAW.style()).style.colour === 'mono'");
    eq(await pg.inputValue("#d2-style-line_width"), "3");  // the fields show what the module holds: the refused entry is gone
    eq(await pg.getAttribute("#d2-style-line_width", "aria-invalid"), null);
    await shot(pg, "style-fields");
    // the settings file: saved as the module writes it, opened again after a change, a damaged one refused in words
    const [sf] = await Promise.all([pg.waitForEvent("download"), pg.click("#d2-style-save")]);
    eq(sf.suggestedFilename(), "mzdraw-settings.json");
    const settings = readFileSync(await sf.path(), "utf8");
    eq(settings, await pg.evaluate("window.MZDRAW.style()"));
    await pg.selectOption("#d2-style", "slides"); await want(pg, { style: "slides" });
    eq(await pg.inputValue("#d2-style-font_size"), "16");
    await pg.setInputFiles("#d2-style-file", { name: "mine.json", mimeType: "application/json", buffer: Buffer.from(settings) });
    await until(pg, "JSON.parse(window.MZDRAW.style()).style.line_width === 3");
    eq(await pg.evaluate("window.MZDRAW.style()"), settings);
    eq([await pg.inputValue("#d2-style"), await pg.inputValue("#d2-style-colour"), await pg.inputValue("#d2-style-font_size")], ["mzlab", "mono", "12"]);
    await pg.setInputFiles("#d2-style-file", { name: "bad.json", mimeType: "application/json", buffer: Buffer.from('{"format":"mzdraw-settings/1","style":{"font_size":1}}') });
    await until(pg, "document.querySelector('#d2-style-note').textContent === I18N.t('error.draw2.style')");
    eq(await pg.evaluate("window.MZDRAW.style()"), settings);
    // back to the default look for the steps that follow
    await pg.selectOption("#d2-style", "slides"); await pg.selectOption("#d2-style", "mzlab"); await want(pg, { style: "mzlab" });
    eq(await pg.evaluate("window.MZDRAW.save('svg')"), plain);
    await pg.click("#d2-style-more summary");
  });

  await step("keyboard alone: a molecule is drawn, walked and edited without the pointer; the outline and the focus are said in words", async () => {
    const live = () => pg.innerText("#d2-live"), key = k => pg.keyboard.press(k);
    await tool(pg, "clear"); await want(pg, { atoms: 0, focus: 0 });
    eq(await pg.innerText("#d2-outline"), await pg.evaluate("I18N.t('draw2.outline.empty')"));
    eq(await pg.getAttribute("#d2-canvas", "aria-describedby"), "d2-outline d2-focus");
    await pg.mouse.move(5, 5);                                // the pointer is away: nothing below is under it
    await pg.focus("#d2-canvas");
    await key("Enter"); await want(pg, { atoms: 1, bonds: 0, focus: 1 });
    eq(await live(), await pg.evaluate("I18N.t('draw2.focus.atom', { name: 'C', index: 1, of: 1, bonds: 0, hydrogens: 4 })"));
    await key("Enter"); await key("Enter"); await want(pg, { atoms: 3, bonds: 2, focus: 3 });
    await key("o"); await key("2");                           // the end atom becomes O, the bond walked becomes double
    // back to the middle carbon with the arrow that points from the oxygen to it (document axes: y up)
    const [, mid, end] = JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')")).atoms;
    const dx = mid.x - end.x, dy = mid.y - end.y;
    await key(Math.abs(dx) >= Math.abs(dy) ? (dx > 0 ? "ArrowRight" : "ArrowLeft") : (dy > 0 ? "ArrowUp" : "ArrowDown"));
    await want(pg, { focus: 2 });
    eq(await live(), await pg.evaluate("I18N.t('draw2.focus.atom', { name: 'C', index: 2, of: 3, bonds: 2, hydrogens: 1 })"));
    await key("Shift+Enter"); await key("1"); await key("n"); await want(pg, { atoms: 4, bonds: 3, focus: 4, warnings: 0 });
    eq(await pg.innerText("#d2-outline"), await pg.evaluate("I18N.t('draw2.outline.piece', { n: 1, formula: 'C2H5NO', atoms: 4, bonds: 3 })"));
    eq(await pg.innerText("#d2-focus"), await pg.evaluate("I18N.t('draw2.focus.atom', { name: 'N', index: 4, of: 4, bonds: 1, hydrogens: 2 })"));
    await key("+");
    eq(await live(), await pg.evaluate("I18N.t('draw2.focus.atom.plus', { name: 'N', index: 4, of: 4, bonds: 1, hydrogens: 3, n: 1 })"));
    await shot(pg, "keyboard");
    await key("-");
    // Space selects the focused atom, so copy and paste work by keyboard; the copy is a second structure in the outline
    await key("Home"); await key(" "); await want(pg, { selected: 1, focus: 1 });
    await key("Control+c"); await key("Control+v"); await want(pg, { atoms: 5 });
    eq(await pg.locator("#d2-outline li").count(), 2);
    await key("Control+z"); await want(pg, { atoms: 4 });
    // Delete removes the focused atom when nothing is selected; Escape leaves the focus; Tab leaves the canvas
    await key("Escape"); await want(pg, { focus: 0, selected: 0 });
    eq(await live(), await pg.evaluate("I18N.t('draw2.focus.none')"));
    await key("End"); await key("Home"); await key("Delete"); await want(pg, { atoms: 3 });
    await pg.focus("#d2-canvas"); await key("Tab");
    assert(await pg.evaluate("document.activeElement !== document.querySelector('#d2-canvas')"), "Tab moves on: the canvas is no keyboard trap");
    await key("Shift+Tab");
    assert(await pg.evaluate("document.activeElement === document.querySelector('#d2-canvas')"), "and Shift+Tab comes back");
    // the help is on the page, closed until asked for
    assert(!(await pg.evaluate("document.querySelector('#d2-keys').open")) && await pg.locator("#d2-keys li").count() === 5, "the keyboard help");
  });

  await step("PDF out by keyboard: a vector file with the text as text and the document inside, opened again with Open a file", async () => {
    await pdfRoundTrip(pg, async () => { await reveal(pg, "#d2-ex-pdf"); await pg.focus("#d2-ex-pdf"); await pg.keyboard.press("Enter"); });
    await reveal(pg, "#d2-ex-pdf"); await shot(pg, "pdf-download");       // the Save as list, with PDF
  });
  await ctx.close();

  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, hasTouch: true }));
  await step("no dragging needed (WCAG 2.5.7): a selection is moved and turned by keys, and brought to a click by Move here with the mouse and with a finger", async () => {
    const atoms = async () => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')")).atoms;
    const centre = async () => { const a = await atoms(), xs = a.map(p => p.x), ys = a.map(p => p.y); return [(Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2]; };
    const live = () => pg.innerText("#d2-live"), say = (key, what) => pg.evaluate(`I18N.t(${JSON.stringify(key)}, { what: ${JSON.stringify(what)} })`);
    const near = (a, b, tol, what) => assert(a.every((v, i) => Math.abs(v - b[i]) < tol), `${what}: ${JSON.stringify(a)} vs ${JSON.stringify(b)}`);
    await tool(pg, "clear"); await tool(pg, "ring:6a"); await pg.mouse.click(cx, cy); await want(pg, { atoms: 6, bonds: 6 });
    await tool(pg, "select"); await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+a"); await want(pg, { selected: 18 - 6 });
    const c0 = await centre(), key = k => pg.keyboard.press(k);
    // Alt+Arrow: 0.1 bond length per press, three presses in a row are one undo step
    for (let n = 0; n < 3; n++) await key("Alt+ArrowRight");
    near(await centre(), [c0[0] + 0.3, c0[1]], 2e-4, "three small steps right");
    eq(await live(), await say("draw2.announce.moved", "right 0.3"));
    await key("Alt+Shift+ArrowUp"); near(await centre(), [c0[0] + 0.3, c0[1] + 1], 2e-4, "one bond length up, joined to the steps before");
    await key("Control+z"); near(await centre(), c0, 1e-9, "presses in a row, whatever their size, are one undo step");
    // Alt+[ and Alt+]: 15 degrees about the centre, 24 presses come back; the announcement says the amount
    const d0 = JSON.stringify(await atoms());
    await key("Alt+["); eq(await live(), await say("draw2.announce.rotated", "15 counter-clockwise"));
    await key("Alt+]"); await key("Alt+]"); eq(await live(), await say("draw2.announce.rotated", "15 clockwise"));
    await key("Alt+["); near((await atoms()).flatMap(p => [p.x, p.y]), JSON.parse(d0).flatMap(p => [p.x, p.y]), 5e-4, "left, right, right, left is the start");
    for (let n = 0; n < 24; n++) await key("Alt+[");
    near((await atoms()).flatMap(p => [p.x, p.y]), JSON.parse(d0).flatMap(p => [p.x, p.y]), 3e-3, "24 turns of 15 degrees are a full turn");
    // without a selection the keys do nothing and the browser does not go back
    await key("Escape"); await key("Alt+ArrowLeft"); eq((await state(pg)).selected, 0);
    // Move here: a real toggle button (and the key M); a click on an empty spot brings the selection there, one undo step
    const toggle = '[data-testid="d2-move-here"]';
    assert((await pg.evaluate(`document.querySelector('${toggle}').tagName`)) === "BUTTON", "a real button");
    eq(await pg.getAttribute(toggle, "aria-pressed"), "false");
    await pg.keyboard.press("Control+a"); await want(pg, { selected: 12 });
    await pg.click(toggle); eq(await pg.getAttribute(toggle, "aria-pressed"), "true");
    const before = JSON.stringify(await atoms()), c1 = await centre();
    await pg.mouse.click(cx + 200, cy + 100); await want(pg, { selected: 12 });
    const c2 = await centre(), k = (c2[0] - c1[0]) / 200;                       // document units per pixel
    assert(k > 0 && Math.abs((c1[1] - c2[1]) / 100 - k) < 1e-3 * k + 1e-4, "right and down on the screen is right and down in the document");
    const ring = (await atoms()).length; eq(ring, 6);
    await shot(pg, "move-here");
    await pg.keyboard.press("Control+z"); eq(JSON.stringify(await atoms()), before);
    await pg.keyboard.press("Control+Shift+z");
    // a finger does the same
    await pg.touchscreen.tap(cx - 300, cy - 150); await want(pg, { selected: 12 });
    const c3 = await centre();
    near([(c3[0] - c2[0]) / -500, (c3[1] - c2[1]) / 250], [k, k], 1e-3 * k + 1e-4, "a finger brings the selection to the tap");
    eq(await live(), await say("draw2.say.moved", "6"));
    // M turns the mode off: a click on an empty spot clears the selection again
    await key("m"); eq(await pg.getAttribute(toggle, "aria-pressed"), "false");
    await pg.mouse.click(cx + 300, cy + 200); await want(pg, { selected: 0 });
  });
  await ctx.close();
  // the desktop palettes in a context with a mouse and keyboard only (a touch screen gets the 44 px handles, tested on the tablet)
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 }));
  await step("palettes: docked on the four edges, floating and snapped by mouse, moved by keyboard, collapsed, hidden and reset from the Palettes menu, kept in the settings; tooltips name built keys only", async () => {
    await pg.setViewportSize({ width: 1280, height: 720 }); await frames(pg);
    await tool(pg, "ring:6a"); await pg.mouse.click(cx - 150, cy - 60); await want(pg, { atoms: 6, bonds: 6 }); await tool(pg, "select");
    await pg.mouse.move(cx, cy); await frames(pg);
    const home = await rects(pg);
    eq(Object.keys(home).length, 16);
    eq([home.tools.dock, home.tools.orient, home.file.dock, home.file.orient, home.structure.dock], ["left", "v", "top", "h", "right"]);
    await tidy(pg, 1280, 720);
    // the kinds drawn most are in view in the collapsed palettes, and no palette overlaps another or the canvas at 720 px
    const KEPT = ["bond:1", "bond:2", "bond:up", "bond:down", "ring:6a", "ring:6", "ring:5", "charge:+", "charge:-", "arrow", "plus"];
    eq(await pg.evaluate(k => k.filter(c => { const b = document.querySelector(`[data-testid="d2-${c}"]`); return !b || !b.offsetParent; }), KEPT), []);
    // by keyboard: a kept button is a stop of its palette, Enter runs it
    await pg.focus('[data-testid="d2-ring:5"]'); await pg.keyboard.press("Enter"); await want(pg, { tool: "ring:5" });
    await pg.focus('[data-testid="d2-bond:down"]'); await pg.keyboard.press("Enter"); await want(pg, { tool: "bond:down" });
    await tool(pg, "select");
    // every button of the palettes says what it does; a key in brackets is a key the module binds
    const titles = await pg.evaluate(`[...document.querySelectorAll("#d2-palettes .items button, #d2-palettes .grip, #d2-flys button[data-cmd], #d2-flys button[data-opens], #d2-show-all, .menu ~ .row button")].map(b => [b.dataset.testid || b.id, b.title, b.getAttribute("aria-label") || b.textContent])`);
    eq(titles.filter(([, title, name]) => !title || !name).map(([id]) => id), []);
    eq(titles.flatMap(([id, title]) => [...title.matchAll(/\(([^)]*)\)/g)].flatMap(m => m[1].split(/,\s*|\s+/)).filter(k => !BUILT.has(k)).map(k => [id, k])), []);
    await shot(pg, "palettes-desktop");
    // every handle says what it is and what its keys do
    const grip = await pg.evaluate(`(g => [g.getAttribute("aria-label"), g.getAttribute("aria-description")])(document.querySelector('[data-testid="d2-grip-edit"]'))`);
    eq(grip, [await pg.evaluate("I18N.t('draw2.pal.grip.name', { name: I18N.t('draw2.pal.edit') })"), await pg.evaluate("I18N.t('draw2.pal.grip.keys')")]);
    assert(/Enter/.test(grip[1]) && /Escape/.test(grip[1]), "the handle names its keys");
    // a click on a handle with a little jitter is a click: nothing moves, nothing is stored
    const stored = await pg.evaluate("Number(document.documentElement.dataset.d2Style || 0)");
    { const [gx, gy] = await center(pg, '[data-testid="d2-grip-edit"]'); await pg.mouse.move(gx, gy); await pg.mouse.down(); await pg.mouse.move(gx + 1, gy + 1); await pg.mouse.up(); await frames(pg); }
    eq(await rects(pg), home);
    eq(await pg.evaluate("Number(document.documentElement.dataset.d2Style || 0)"), stored);
    /** drags a palette by its handle with the mouse to a page point; `hint`: the edge it says it will dock on before it is let go */
    const drag = async (id, x, y, hint) => {
      const [gx, gy] = await center(pg, `[data-testid="d2-grip-${id}"]`);
      await pg.mouse.move(gx, gy); await pg.mouse.down(); await pg.mouse.move(x, y, { steps: 8 });
      if (hint !== undefined) eq(await pg.evaluate("(h => h.hidden ? '' : h.dataset.edge)(document.querySelector('#d2-dock-hint'))"), hint);
      await pg.mouse.up(); await frames(pg);
      return [gx, gy];
    };
    // floating: the palette follows the pointer
    // a spot for its top edge that is farther than the snap distance from every line a palette would snap to
    const [, gy0] = await center(pg, '[data-testid="d2-grip-edit"]');
    const lines = Object.entries(home).filter(([id, q]) => q && id !== "edit").flatMap(([, q]) => [q.y + q.h, q.y - home.edit.h, q.y, q.y + q.h - home.edit.h]).concat([0, 720 - home.edit.h]);
    let top = 100, room = -1;
    for (let t = 100; t < 600; t++) { const d = Math.min(...lines.map(l => Math.abs(t - l))); if (d > room) { room = d; top = t; } }
    assert(room > 16, "a free spot to drop the palette on");
    const [ex, ey] = await drag("edit", 300, top + (gy0 - home.edit.y), "");
    let r = await rects(pg);
    eq(r.edit.dock, "float");
    assert(Math.abs(r.edit.x - (300 - (ex - home.edit.x))) <= 1 && Math.abs(r.edit.y - top) <= 1, "the palette follows the pointer");
    // docked on the bottom as a row, on the right as a column
    await drag("view", 640, 719, "bottom");
    await drag("history", 1279, 300, "right");
    r = await rects(pg);
    eq([r.view.dock, r.view.orient, r.view.y + r.view.h, r.history.dock, r.history.orient], ["bottom", "h", 720, "right", "v"]);
    // a full right column wraps: the palette stands at the right edge, or flush against the column beside it
    assert(r.history.x + r.history.w === 1280 || Object.values(r).some(q => q && q.dock === "right" && q.x === r.history.x + r.history.w), "docked in the right column");
    // two floating palettes: one dropped 6 px from the other snaps flush to it, on the same line
    await drag("rings", 500, 300);
    r = await rects(pg);
    const [kx, ky] = await center(pg, '[data-testid="d2-grip-charges"]');
    await drag("charges", r.rings.x + r.rings.w + 6 + (kx - r.charges.x), r.rings.y + 4 + (ky - r.charges.y));
    const s2 = await rects(pg);
    eq([s2.charges.dock, s2.charges.x, s2.charges.y], ["float", s2.rings.x + s2.rings.w, s2.rings.y]);
    await tidy(pg, 1280, 720);
    await shot(pg, "palettes-moved");
    await pg.emulateMedia({ colorScheme: "dark" }); await frames(pg); await shot(pg, "palettes-moved-dark"); await pg.emulateMedia({ colorScheme: "light" });
    // the layout is in the settings file the module writes, and the page keeps it
    await until(pg, "Number(document.documentElement.dataset.d2Style || 0) >= 1");
    // and after a reload the palettes are where they were left
    { const left = await rects(pg); await pg.reload(); await until(pg, "document.documentElement.dataset.d2Ready === '1'"); await frames(pg); eq(await rects(pg), left); }
    const layout = JSON.parse(await pg.evaluate("window.MZDRAW.style()")).layout;
    // (docking a palette in a column numbers that column again: its neighbours are written too)
    const ids = layout.palettes.map(p => p.id);
    eq(["charges", "edit", "history", "rings", "view"].filter(id => !ids.includes(id)), []);
    eq(layout.palettes.filter(p => p.dock === "float").map(p => p.id).sort(), ["charges", "edit", "rings"]);
    // the keyboard: the arrows move a palette 8 px (32 with Shift), Escape puts it back, Enter floats it and docks it again
    await pg.focus('[data-testid="d2-grip-tools"]');
    await pg.keyboard.press("ArrowRight"); await pg.keyboard.press("Shift+ArrowDown"); await frames(pg);
    r = await rects(pg);
    eq([r.tools.dock, r.tools.x, r.tools.y], ["float", home.tools.x + 8, home.tools.y + 32]);
    await pg.keyboard.press("Escape"); await frames(pg);
    eq((await rects(pg)).tools, home.tools);
    await pg.keyboard.press("Enter"); await frames(pg); eq((await rects(pg)).tools.dock, "float");
    await pg.keyboard.press("Enter"); await frames(pg); eq((await rects(pg)).tools, home.tools);
    eq(await pg.evaluate("document.activeElement.dataset.testid"), "d2-grip-tools");
    // a double click on the handle expands a collapsed palette: every bond kind is in view
    await pg.dblclick('[data-testid="d2-grip-bonds"]'); await frames(pg);
    assert(await pg.isVisible('#d2-pal-bonds [data-testid="d2-bond:wavy"]'), "the bonds palette expanded");
    // the Palettes menu: hide one, collapse one again, then hide all and show them, then reset the layout
    await pg.click('[data-testid="d2-palettes"]');
    await pg.click('[data-testid="d2-menu-hidden-keys"]'); await frames(pg);
    eq((await rects(pg)).keys, null);
    await pg.click('[data-testid="d2-menu-collapsed-bonds"]'); await frames(pg);
    assert(await pg.isHidden('[data-testid="d2-bond:wavy"]'), "the bonds palette collapsed");
    assert(await pg.isDisabled('[data-testid="d2-menu-orient-tools"]') && await pg.isEnabled('[data-testid="d2-menu-orient-edit"]'), "only a floating palette turns");
    const was = (await rects(pg)).edit.orient;       // a floating palette keeps the orientation of its edge: the edit row
    await pg.click('[data-testid="d2-menu-orient-edit"]'); await frames(pg); eq([was, (await rects(pg)).edit.orient], ["h", "v"]);
    // the key \ with the focus on a handle (not the canvas) hides every palette too; the Palettes button brings them back
    await pg.focus('[data-testid="d2-grip-tools"]'); await pg.keyboard.press("\\"); await frames(pg);
    eq(Object.values(await rects(pg)).filter(Boolean), []);
    await pg.click("#d2-show-all"); await frames(pg);
    assert(await pg.isHidden("#d2-show-all") && (await rects(pg)).tools, "the palettes are back");
    // the key \ on the canvas: every palette hidden, the whole page is canvas; the Palettes button brings them back
    await pg.focus("#d2-canvas"); await pg.keyboard.press("\\"); await frames(pg);
    eq(Object.values(await rects(pg)).filter(Boolean), []);
    eq(await pg.evaluate("[document.querySelector('#d2-canvas').clientWidth, document.querySelector('#d2-canvas').clientHeight]"), [1280, 720]);
    await pg.click("#d2-show-all"); await frames(pg);
    assert(await pg.isHidden("#d2-show-all") && (await rects(pg)).tools, "the palettes are back");
    await pg.click('[data-testid="d2-palettes"]'); await pg.click('[data-testid="d2-menu-reset"]'); await frames(pg);
    eq(await rects(pg), home);
    eq(JSON.parse(await pg.evaluate("window.MZDRAW.style()")).layout, undefined);
    await pg.setViewportSize({ width: 1280, height: 800 }); await frames(pg);
  });
  await ctx.close();

  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 }));
  await step("collapsed palettes: the head of a fresh page runs the single bond and benzene, then the kind used last, kept in view or not", async () => {
    // a fresh page: nothing has been chosen yet, so the head shows what the palette was made to offer first
    eq((await state(pg)).tool, "bond:1");
    await tool(pg, "select"); await want(pg, { tool: "select" });
    await pg.click('[data-testid="d2-head-bonds"]'); await want(pg, { tool: "bond:1" });          // not the triple bond that heads the flyout
    await tool(pg, "select"); await pg.click('[data-testid="d2-head-rings"]'); await want(pg, { tool: "ring:6a" });          // benzene, not the ring of three
    // the head follows the last used kind: one kept in view, then one from the flyout, then back
    await pg.click('[data-testid="d2-bond:2"]'); await want(pg, { tool: "bond:2" });
    await tool(pg, "select"); await pg.click('[data-testid="d2-head-bonds"]'); await want(pg, { tool: "bond:2" });
    await tool(pg, "bond:wavy"); await want(pg, { tool: "bond:wavy" });
    await tool(pg, "select"); await pg.click('[data-testid="d2-head-bonds"]'); await want(pg, { tool: "bond:wavy" });
    await pg.click('[data-testid="d2-bond:1"]'); await want(pg, { tool: "bond:1" });
    await tool(pg, "select"); await pg.click('[data-testid="d2-head-bonds"]'); await want(pg, { tool: "bond:1" });
    await tool(pg, "bond:1"); await want(pg, { tool: "bond:1" });
  });
  await step("side palette: a click on its title is not undone by a layout that runs before the toggle event arrives", async () => {
    // The disclosure changes at once when the title is clicked, but the module hears of it in the `toggle` event, which comes in a later task.
    // A layout that runs in between (a resize, here) must not put the old state back. All in one evaluate, so there is no timing to hope for.
    const r = await pg.evaluate(async () => {
      const d = /** @type {HTMLDetailsElement} */ (document.querySelector("#d2-data-more")), summary = /** @type {HTMLElement} */ (d.querySelector(":scope > summary"));
      const was = d.open;
      summary.click();
      window.dispatchEvent(new Event("resize"));                                   // a layout, before the toggle event
      const during = d.open;
      await new Promise(done => d.addEventListener("toggle", done, { once: true }));
      await new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)));
      const data = JSON.parse(window.MZDRAW.palettes(innerWidth, innerHeight, "")).palettes.find((/** @type {any} */ p) => p.id === "data");
      return { was, during, after: d.open, module_collapsed: data.collapsed };
    });
    eq(r, { was: false, during: true, after: true, module_collapsed: false });
    // and the same going back: closed by a click, it stays closed through a layout before the event
    const back = await pg.evaluate(async () => {
      const d = /** @type {HTMLDetailsElement} */ (document.querySelector("#d2-data-more"));
      /** @type {HTMLElement} */ (d.querySelector(":scope > summary")).click();
      window.dispatchEvent(new Event("resize"));
      const during = d.open;
      await new Promise(done => d.addEventListener("toggle", done, { once: true }));
      await new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)));
      const data = JSON.parse(window.MZDRAW.palettes(innerWidth, innerHeight, "")).palettes.find((/** @type {any} */ p) => p.id === "data");
      return { during, after: d.open, module_collapsed: data.collapsed };
    });
    eq(back, { during: false, after: false, module_collapsed: true });
  });
  await step("autosave: the drawing is saved in the browser, offered on the next visit, restored with the same bytes, or discarded; the style is remembered", async () => {
    const ready = () => until(pg, "document.documentElement.dataset.d2Ready === '1'");
    const saves = () => pg.evaluate("Number(document.documentElement.dataset.d2Saved || 0)");
    eq(await pg.evaluate("document.documentElement.dataset.d2Autosave"), "on");
    assert(await pg.isHidden("#d2-restore"), "nothing saved yet, nothing offered");
    await pg.fill("#d2-text", "C[C@H](N)C(=O)O"); await pg.click("#d2-add"); await want(pg, { atoms: 6, bonds: 5 });
    await tool(pg, "arrow"); await pg.mouse.click(cx + 200, cy + 150); await want(pg, { objects: 1 });
    await until(pg, "Number(document.documentElement.dataset.d2Saved || 0) >= 1");       // one second after the last change
    const doc = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    await pg.reload(); await ready();
    eq(await state(pg), { focus: 0, atoms: 0, bonds: 0, objects: 0, tool: "bond:1", style: "mzlab", warnings: 0, std: 0, selected: 0 });
    assert(await pg.isVisible("#d2-restore"), "the saved drawing is offered");
    eq(await pg.innerText("#d2-restore-text"), await pg.evaluate("I18N.t('draw2.restore.found.objects', { atoms: 6, bonds: 5, objects: 1 })"));
    // while the offer waits nothing is written: a stray drawing does not replace the saved one
    await pg.mouse.click(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    await pg.waitForTimeout(1500);
    eq(await saves(), 0);
    await shot(pg, "restore");
    await pg.click("#d2-restore-yes"); await want(pg, { atoms: 6, bonds: 5, objects: 1 });
    assert(await pg.isHidden("#d2-restore"), "the offer is gone");
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), doc);       // the same bytes, ids included
    assert(await pg.isDisabled('[data-testid="d2-undo"]'), "an opened drawing has no history");
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('draw2.restore.done')"));
    // work goes on and is saved again; on the next visit Discard deletes it for good
    await tool(pg, "atom:O"); await pg.mouse.click(cx - 250, cy - 150); await want(pg, { atoms: 7 });
    await until(pg, "Number(document.documentElement.dataset.d2Saved || 0) >= 1");
    await pg.reload(); await ready();
    eq(await pg.innerText("#d2-restore-text"), await pg.evaluate("I18N.t('draw2.restore.found.objects', { atoms: 7, bonds: 5, objects: 1 })"));
    await pg.click("#d2-restore-no");
    await until(pg, "document.querySelector('#d2-restore').hidden");       // the file is deleted first, then the offer goes
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('draw2.restore.dropped')"));
    eq((await state(pg)).atoms, 0);
    await pg.reload(); await ready();
    assert(await pg.isHidden("#d2-restore"), "a discarded drawing is not offered again");
    // an emptied canvas is saved as empty and is not offered
    await tool(pg, "ring:6a"); await pg.mouse.click(cx, cy); await until(pg, "Number(document.documentElement.dataset.d2Saved || 0) >= 1");
    await tool(pg, "clear"); await until(pg, "Number(document.documentElement.dataset.d2Saved || 0) >= 2");
    await pg.reload(); await ready();
    assert(await pg.isHidden("#d2-restore"), "an empty drawing is not offered");
    // the style is remembered too: as the settings file the module writes, read back by the module on the next visit
    const kept = () => pg.evaluate("Number(document.documentElement.dataset.d2Style || 0)");
    await pg.selectOption("#d2-style", "acs-1996"); await want(pg, { style: "acs-1996" });
    await until(pg, "Number(document.documentElement.dataset.d2Style || 0) >= 1");
    await pg.reload(); await ready();
    eq((await state(pg)).style, "acs-1996");
    eq(await pg.inputValue("#d2-style"), "acs-1996");
    eq(await pg.innerText("#d2-style-note"), await pg.evaluate("I18N.t('draw2.style.acs-1996.note')"));
    eq(await kept(), 0);                                    // reading it back is not a change
    // a kept file the module refuses (damaged, or of a newer mzDraw) is ignored: the page starts in the default style
    for (const bad of ['{"format":"mzdraw-settings/1","style":{"bondLength":', '{"format":"mzdraw-settings/9","style":{}}']) {
      await pg.evaluate(async text => { const w = await (await (await navigator.storage.getDirectory()).getFileHandle("mzdraw-settings.json", { create: true })).createWritable(); await w.write(text); await w.close(); }, bad);
      await pg.reload(); await ready();
      eq((await state(pg)).style, "mzlab");
      eq(await pg.inputValue("#d2-style"), "mzlab");
    }
    await pg.selectOption("#d2-style", "slides"); await until(pg, "Number(document.documentElement.dataset.d2Style || 0) >= 1");
    await pg.reload(); await ready();
    eq((await state(pg)).style, "slides");
    await pg.selectOption("#d2-style", "mzlab"); await until(pg, "Number(document.documentElement.dataset.d2Style || 0) >= 1");
  });
  await step("drawing checks: a wedge drawn onto a CH2 gives one line; Enter on it selects its atom and bond; the drawing is not changed", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0, std: 0 });
    await pg.click("#d2-std summary");                                   // the panel is closed until it is opened
    eq(await pg.innerText("#d2-std-note"), "");                           // nothing drawn, nothing said
    await tool(pg, "bond:1"); await pg.mouse.click(cx, cy); await want(pg, { atoms: 2, bonds: 1, std: 0 });
    eq(await pg.innerText("#d2-std-note"), await pg.evaluate("I18N.t('draw2.std.none')"));
    await tool(pg, "bond:up"); await pg.mouse.click(cx, cy); await want(pg, { atoms: 3, bonds: 2, std: 1 });      // a wedge starts on the first atom, which is now a CH2
    eq(await pg.locator("#d2-std-list li").count(), 1);
    eq(await pg.locator("#d2-std-list button").getAttribute("data-key"), "std.stereo.notCentre");
    eq(await pg.innerText("#d2-std-list button"), (await pg.evaluate("I18N.t('draw2.std.grade.warn')")) + " " + (await pg.evaluate("I18N.t('draw2.std.stereo.notCentre', { n: 1 })")));
    const report = JSON.parse(await pg.evaluate("window.MZDRAW.save('std')"));
    eq(report.checks.map(c => [c.key, c.grade, c.atoms.length, c.bonds.length]), [["std.stereo.notCentre", "warn", 1, 1]]);
    await pg.locator("#d2-std-list").scrollIntoViewIfNeeded();
    await shot(pg, "checks");
    await shotEl(pg, "#d2-std-panel", "std-light");
    await pg.emulateMedia({ colorScheme: "dark" }); await shotEl(pg, "#d2-std-panel", "std-dark");
    await pg.emulateMedia({ colorScheme: "light" });
    const before = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    await pg.focus("#d2-std-list button"); await pg.keyboard.press("Enter");
    await want(pg, { selected: 2 });                                      // the atom and the bond the finding is about
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), before);       // a selection is no change and no history step
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Escape"); await want(pg, { selected: 0 });
    await pg.keyboard.press("Control+z"); await want(pg, { atoms: 2, bonds: 1, std: 0 });      // the line goes with the wedge
  });
  await step("binary CDX: a file opened by the keyboard (focus on Open, Enter) is read, a cut one is refused in words", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    const open = async name => { const [fc] = await Promise.all([pg.waitForEvent("filechooser"), (async () => { await pg.focus("#d2-open"); await pg.keyboard.press("Enter"); })()]); return fc; };
    await (await open()).setFiles({ name: "dimethylamine.cdx", mimeType: "chemical/x-cdx", buffer: CDX }); await want(pg, { atoms: 3, bonds: 2 });
    await tool(pg, "undo"); await want(pg, { atoms: 0 });
    await (await open()).setFiles({ name: "cut.cdx", mimeType: "chemical/x-cdx", buffer: CDX.subarray(0, 90) }); await until(pg, "document.querySelector('#d2-note').textContent.length > 0");
    eq(await pg.innerText("#d2-note"), await pg.evaluate("I18N.t('error.draw2.cdx')"));
    await want(pg, { atoms: 0 });
  });

  await ctx.close();

  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 768, height: 1024 }, deviceScaleFactor: 2, hasTouch: true, isMobile: true }));
  await step("tablet 768x1024 with touch: the same loop by tapping, no sideways scroll", async () => {
    assert(await pg.evaluate("document.documentElement.scrollWidth <= innerWidth"), "the page scrolls sideways at 768 px");
    await loop(pg, cx, cy, (x, y) => pg.touchscreen.tap(x, y), "tablet");
    // the clipboard and the transforms by tapping the buttons
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "bond:down"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    await tool(pg, "select"); await pg.touchscreen.tap(cx, cy); await want(pg, { selected: 1 });
    await tool(pg, "copy"); await tool(pg, "paste"); await want(pg, { atoms: 3, bonds: 1, selected: 1 });
    await tool(pg, "rotate-r"); await tool(pg, "cut"); await want(pg, { atoms: 2, bonds: 1 });
    // a reaction arrow and a plus by tapping
    await tool(pg, "arrow"); await pg.touchscreen.tap(cx + 100, cy + 100); await tool(pg, "plus"); await pg.touchscreen.tap(cx - 100, cy + 100); await want(pg, { objects: 2 });
  });

  await step("tablet with touch: a CDX file is opened by tapping Open", async () => {
    // a page of its own; the Open button of the file palette is on the page from the start
    const t = await context(browser, { viewport: { width: 768, height: 1024 }, deviceScaleFactor: 2, hasTouch: true, isMobile: true });
    try {
      const tap = async sel => { const b = await t.pg.locator(sel).boundingBox(); await t.pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2); };
      const [fc] = await Promise.all([t.pg.waitForEvent("filechooser"), tap('[data-testid="d2-open"]')]);
      await fc.setFiles({ name: "dimethylamine.cdx", mimeType: "chemical/x-cdx", buffer: CDX }); await want(t.pg, { atoms: 3, bonds: 2 });
    } finally { await t.ctx.close(); }
  });

  await step("tablet with touch: the PFAS scale and level 2c of the annotation, set by tapping", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "bond:down"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    const tapOn = async sel => { const b = await pg.locator(sel).boundingBox(); await pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2); };
    await tapPalette(pg, "annotation", true);
    if (!(await pg.evaluate("document.querySelector('#d2-ann-more').open"))) { await pg.locator("#d2-ann-more summary").scrollIntoViewIfNeeded(); await tapOn("#d2-ann-more summary"); }
    const af = "#d2-ann fieldset:nth-of-type(1)";
    await pg.selectOption(`${af} .d2-annf:nth-of-type(1) select`, "charbonnet-2022");   // the native picker of a select is the system's: the choice is made on the element
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-ann fieldset:nth-of-type(1) .d2-annf:nth-of-type(2) select option')].map(o => o.value)"), ["", "1a", "1b", "2a", "2b", "2c", "3a", "3b", "3c", "3d", "4", "5a", "5b"]);
    await pg.selectOption(`${af} .d2-annf:nth-of-type(2) select`, "2c");
    await pg.locator(`${af} button`).scrollIntoViewIfNeeded(); await tapOn(`${af} button`);
    await until(pg, "document.querySelector('#d2-ann-note').textContent === I18N.t('draw2.ann.done', { level: '2c' })");
    eq(await pg.evaluate("(a => a.scale + '/' + a.level)(JSON.parse(window.MZDRAW.state()).outline[0].ann)"), "charbonnet-2022/2c");
    assert((await pg.innerText("#d2-outline li:nth-of-type(1)")).endsWith(await pg.evaluate("I18N.t('draw2.outline.level', { level: '2c' })")), "the outline says the level");
    await shot(pg, "annotation-scale");
    await tapPalette(pg, "annotation", false);
  });

  await step("PDF out by touch: the same file, downloaded with a tap and opened again", async () => {
    const pdf = await pdfRoundTrip(pg, async () => { await reveal(pg, "#d2-ex-pdf"); const b = await pg.locator("#d2-ex-pdf").boundingBox(); await pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2); });
    eq(pdf.subarray(0, 5).toString("latin1"), "%PDF-");
  });

  await step("PDF out by pen: the button pressed with a pen gives the same file", async () => {
    const cdp = await pg.context().newCDPSession(pg);
    const pdf = await pdfRoundTrip(pg, async () => {
      await reveal(pg, "#d2-ex-pdf");
      const b = await pg.locator("#d2-ex-pdf").boundingBox(), at = { x: b.x + b.width / 2, y: b.y + b.height / 2, button: "left", clickCount: 1, pointerType: "pen" };
      await cdp.send("Input.dispatchMouseEvent", { type: "mousePressed", ...at }); await cdp.send("Input.dispatchMouseEvent", { type: "mouseReleased", ...at });
    });
    eq(pdf.subarray(0, 5).toString("latin1"), "%PDF-");
  });

  await step("drawing checks by touch: a tap on the line selects the atom and bond it is about", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0, std: 0 });
    await tool(pg, "bond:1"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    await tool(pg, "bond:up"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 3, bonds: 2, std: 1 });
    await tapPalette(pg, "check", true);
    if (!(await pg.evaluate("document.querySelector('#d2-std').open"))) await pg.locator("#d2-std summary").tap();
    await pg.locator("#d2-std-list button").tap();
    await want(pg, { selected: 2 });
    await shotEl(pg, "#d2-std-panel", "std-tablet");
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('draw2.std.selected', { n: 2 })"));
    assert(await pg.evaluate("document.documentElement.scrollWidth <= innerWidth"), "the panel does not scroll the page sideways");
    await tapPalette(pg, "check", false);
  });

  await step("tablet with touch: a group label on the left of its bond is written reversed", async () => {
    const drawn = () => pg.evaluate("Array.from(window.MZDRAW.strings())");
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "bond:1"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    await fillIn(pg, "#d2-label", "CO2H"); await pg.press("#d2-label", "Enter"); await want(pg, { tool: "label:CO2H" });
    await pg.touchscreen.tap(cx, cy);
    eq(JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')")).atoms.map(a => a.lab || ""), ["CO2H", ""]);
    eq((await drawn()).filter(t => ["HO", "2", "C", "CO", "H"].includes(t)), ["HO", "2", "C"]);
    await shot(pg, "label-reversal-touch");
  });

  await step("two fingers pan and zoom without drawing; a long press selects; a cancelled touch commits nothing; a pen draws and its eraser end erases", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([id, x, y]) => ({ id, x, y })) });
    const doc = () => pg.evaluate("window.MZDRAW.save('mzdraw')");
    // the width of the first line of the display list grows with the zoom: it tells that the view changed
    const firstX = () => pg.evaluate("window.MZDRAW.scene()[1]");
    await tool(pg, "clear"); await tool(pg, "fit"); await tool(pg, "ring:6a"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 6, bonds: 6 });
    await tool(pg, "bond:1");
    const before = await doc(), x0 = await firstX();
    // spread two fingers, then move them together, then lift: the view changed, the drawing did not
    await touch("touchStart", [[1, cx - 60, cy + 150]]);
    await touch("touchStart", [[1, cx - 60, cy + 150], [2, cx + 60, cy + 150]]);
    await touch("touchMove", [[1, cx - 120, cy + 150], [2, cx + 120, cy + 150]]);
    await touch("touchMove", [[1, cx - 100, cy + 190], [2, cx + 140, cy + 190]]);
    await touch("touchEnd", [[1, cx - 100, cy + 190]]);
    await touch("touchMove", [[1, cx - 20, cy + 100]]);
    await touch("touchEnd", []);
    eq(await doc(), before);
    assert((await firstX()) !== x0, "the view moved");
    eq((await state(pg)).atoms, 6);
    // a finger held still for 700 ms on an atom selects it instead of sprouting a bond
    await tool(pg, "fit");
    const atom = await pg.evaluate("(() => { const d = window.MZDRAW.scene(); return [d[1], d[2]]; })()");       // the first line starts at an atom
    const box = await pg.locator("#d2-canvas").boundingBox();
    const [ax, ay] = [box.x + atom[0], box.y + atom[1]];
    await touch("touchStart", [[1, ax, ay]]);
    await pg.waitForTimeout(700);
    await touch("touchEnd", []);
    await want(pg, { selected: 1, atoms: 6, bonds: 6 });
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('draw2.say.selected')"));
    // a drag taken away by the system commits nothing
    await touch("touchStart", [[1, ax, ay]]);
    await touch("touchMove", [[1, ax + 40, ay - 60]]);
    await touch("touchCancel", []);
    eq(await doc(), before);
    // a quick tap still draws
    await touch("touchStart", [[1, ax, ay]]); await touch("touchEnd", []);
    await want(pg, { atoms: 7, bonds: 7 });
    // the pen: its tip draws with the tool, its eraser end (button 5, buttons 32) erases, and the tool comes back
    const pen = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1 });
    await pen("mouseMoved", ax, ay); await pen("mousePressed", ax, ay); await pen("mouseReleased", ax, ay);
    await want(pg, { atoms: 8, bonds: 8 });
    // (the browser's test protocol cannot press button 5, so the eraser end is sent as pointer events built in the page)
    await pg.evaluate(`(() => { const c = document.querySelector("#d2-canvas"), r = c.getBoundingClientRect();
      for (const [type, buttons] of [["pointerdown", 32], ["pointerup", 0]])
        c.dispatchEvent(new PointerEvent(type, { pointerType: "pen", pointerId: 77, button: 5, buttons, clientX: ${ax}, clientY: ${ay}, bubbles: true, cancelable: true })); })()`);
    // the erased atom had its two ring bonds and the two bonds sprouted above: 8 atoms and 8 bonds become 7 and 4
    await want(pg, { atoms: 7, bonds: 4, tool: "bond:1" });
    await shot(pg, "touch");
  });
  await step("tablet 768x1024: handles of 44 px; palettes dragged by a finger and by a pen, docked and floating; the menu by tapping; reset", async () => {
    const cdp = await ctx.newCDPSession(pg);
    await tool(pg, "clear"); await frames(pg);
    const home = await rects(pg);
    eq(Object.entries(home).filter(([, r]) => r).map(([id]) => id).sort(), ["atoms", "bonds", "charges", "edit", "file", "history", "objects", "rings", "tools", "view"]);
    eq(await pg.evaluate("[...document.querySelectorAll('.grip')].filter(g => g.offsetParent && Math.min(g.offsetWidth, g.offsetHeight) < 44).map(g => g.dataset.testid)"), []);
    await tidy(pg, 768, 1024);
    // the kept kinds are in view here too, 44 px each, and a finger runs them
    const KEPT = ["bond:1", "bond:2", "bond:up", "bond:down", "ring:6a", "ring:6", "ring:5", "charge:+", "charge:-", "arrow", "plus"];
    eq(await pg.evaluate(k => k.filter(c => { const b = document.querySelector(`[data-testid="d2-${c}"]`); return !b || !b.offsetParent || Math.min(b.offsetWidth, b.offsetHeight) < 44; }), KEPT), []);
    for (const [c, tl] of [["ring:6", "ring:6"], ["bond:up", "bond:up"], ["plus", "plus"]]) {
      const b = await pg.locator(`[data-testid="d2-${c}"]`).boundingBox();
      await pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2); await want(pg, { tool: tl });
    }
    await tool(pg, "select");
    await tool(pg, "ring:6a"); await pg.mouse.click(cx, cy + 100); await want(pg, { atoms: 6 });
    await shot(pg, "palettes-tablet");
    await pg.emulateMedia({ colorScheme: "dark" }); await frames(pg); await shot(pg, "palettes-tablet-dark"); await pg.emulateMedia({ colorScheme: "light" });
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    // the menu by tapping: a side palette hidden on a tablet is shown, and hidden again
    await pg.tap('[data-testid="d2-palettes"]'); await pg.tap('[data-testid="d2-menu-hidden-check"]'); await frames(pg);
    assert((await rects(pg)).check, "a side palette shown from the menu");
    // the boxes of the menu are 44 px on a touch screen
    eq(await pg.evaluate("[...document.querySelectorAll('.menu input')].filter(i => Math.min(i.offsetWidth, i.offsetHeight) < 44).length"), 0);
    await shot(pg, "palettes-tablet-menu");
    await pg.tap('[data-testid="d2-palettes"]'); await pg.mouse.move(1, 1); await frames(pg);       // the menu closed: the palette shown is not covered
    await shot(pg, "palettes-tablet-side");
    await pg.tap('[data-testid="d2-palettes"]'); await frames(pg);
    await pg.tap('[data-testid="d2-menu-hidden-check"]'); await frames(pg);
    eq((await rects(pg)).check, null);
    await pg.tap('[data-testid="d2-palettes"]'); eq(await pg.getAttribute('[data-testid="d2-palettes"]', "aria-expanded"), "false");
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    const finger = async (id, x, y) => {
      const [gx, gy] = await center(pg, `[data-testid="d2-grip-${id}"]`);
      await touch("touchStart", [[gx, gy]]);
      for (let i = 1; i <= 6; i++) await touch("touchMove", [[gx + (x - gx) * i / 6, gy + (y - gy) * i / 6]]);
      await touch("touchEnd", []); await frames(pg);
    };
    const pen = async (id, x, y) => {
      const [gx, gy] = await center(pg, `[data-testid="d2-grip-${id}"]`), send = (type, px, py, buttons) => cdp.send("Input.dispatchMouseEvent", { type, x: px, y: py, pointerType: "pen", button: "left", buttons, clickCount: 1 });
      await send("mouseMoved", gx, gy, 0); await send("mousePressed", gx, gy, 1);
      for (let i = 1; i <= 6; i++) await send("mouseMoved", gx + (x - gx) * i / 6, gy + (y - gy) * i / 6, 1);
      await send("mouseReleased", x, y, 0); await frames(pg);
    };
    await finger("edit", 384, 500);
    await finger("view", 384, 1023);
    await pen("rings", 767, 400);
    const r = await rects(pg);
    eq([r.edit.dock, r.view.dock, r.view.orient, r.view.y + r.view.h, r.rings.dock, r.rings.orient, r.rings.x + r.rings.w], ["float", "bottom", "h", 1024, "right", "v", 768]);
    await tidy(pg, 768, 1024);
    await shot(pg, "palettes-tablet-moved");
    // a tap on the canvas still draws: the palettes take nothing from it
    await tool(pg, "bond:1"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    // (after a synthetic touch drag, on a palette or on the canvas alike, the test protocol drops the click of the next tap: the
    // reset is clicked)
    await pg.click('[data-testid="d2-palettes"]'); await pg.click('[data-testid="d2-menu-reset"]'); await frames(pg);
    eq(await rects(pg), home);
  });
  await ctx.close();
  // The context menu of the canvas (web/context.js paints what Editor::context_menu builds; Rust decides every item)
  const ring = async (pg, cx, cy) => { await tool(pg, "clear"); await tool(pg, "ring:6a"); await pg.mouse.click(cx, cy); await want(pg, { atoms: 6, bonds: 6 }); await tool(pg, "select"); };
  /** the pixel of the first line of the display list (it starts at an atom) and its middle (the bond) */
  const firstLine = async pg => {
    const d = await pg.evaluate("window.MZDRAW.scene().slice(0, 5)"), b = await pg.locator("#d2-canvas").boundingBox();
    return { atom: [b.x + d[1], b.y + d[2]], end: [b.x + d[3], b.y + d[4]], bond: [b.x + (d[1] + d[3]) / 2, b.y + (d[2] + d[4]) / 2] };
  };
  /** the pixel of the right-most end of a line of the display list, an atom on the right of the drawing: a menu opened there leaves the structure in view */
  const rightmost = async pg => {
    const d = await pg.evaluate("window.MZDRAW.scene()"), b = await pg.locator("#d2-canvas").boundingBox(); let best = null;
    for (let i = 0; i < d.length;) {
      const op = d[i];
      if (op === 1) { for (const [x, y] of [[d[i + 1], d[i + 2]], [d[i + 3], d[i + 4]]]) if (!best || x > best[0]) best = [x, y]; i += 7; }
      else i += op === 2 ? 7 : op === 3 ? 6 : op === 6 ? 8 : 5;
    }
    return [b.x + best[0], b.y + best[1]];
  };
  const doc = () => pg.evaluate("window.MZDRAW.save('mzdraw')");
  const menuNow = pg => pg.evaluate(`(() => { const m = document.getElementById("d2-menu"); return m.hidden ? null : { target: m.dataset.target, kind: m.dataset.kind,
    ids: [...m.querySelectorAll("button[data-id]")].map(b => b.dataset.id), active: document.activeElement.dataset.id || document.activeElement.className }; })()`);
  const shown = (pg, on = true) => until(pg, `document.getElementById("d2-menu").hidden === ${!on}`);
  const item = id => `[data-testid="d2-ctx-${id}"]`;
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, permissions: ["clipboard-read", "clipboard-write"] }));
  await step("context menu, mouse: each target has its menu, the items run as one undo step, the keys are shown, the menu closes on any change and a stale one is refused", async () => {
    const live = () => pg.innerText("#d2-live");
    // an empty spot of the canvas, 60 px inside its left edge (the palettes at the sides take as much as they need, so it is not cx - 450)
    const box = await pg.locator("#d2-canvas").boundingBox(), empty = [box.x + 60, cy - 300];
    const open = async (x, y, target) => { await pg.mouse.click(x, y, { button: "right" }); await shown(pg); eq((await menuNow(pg)).target, target); };
    // the menu of the browser never opens on the canvas (the event is cancelled) and the page opens nothing for it: the module decides
    const cancelled = sel => pg.evaluate(`(() => { const e = new MouseEvent("contextmenu", { cancelable: true, bubbles: true }); document.querySelector(${JSON.stringify(sel)}).dispatchEvent(e); return e.defaultPrevented; })()`);
    eq(await cancelled("#d2-canvas"), true); await shown(pg, false);
    await ring(pg, cx, cy);
    await tool(pg, "arrow"); await pg.mouse.click(cx + 250, cy + 100); await want(pg, { objects: 1 }); await tool(pg, "select");
    const { atom, bond } = await firstLine(pg), before = await doc();
    // an atom: its element is changed from the group, one undo step; the keys of the module are shown; it asks the page for its fields
    await open(...atom, "atom");
    eq((await menuNow(pg)).ids.slice(0, 9), ["atom.element", "atom.el.C", "atom.el.N", "atom.el.O", "atom.el.S", "atom.el.P", "atom.el.F", "atom.el.Cl", "atom.el.Br"]);
    eq(await pg.innerText(`${item("edit.copy")} .k`), "Ctrl+C"); eq(await pg.innerText(`${item("atom.el.Cl")} .k`), "Shift+C");
    eq(await pg.getAttribute(item("atom.el.C"), "aria-checked"), "true");
    assert(!(await pg.isVisible(item("atom.el.N"))), "the group is folded");
    eq(await cancelled("#d2-menu .ctx button"), true);                   // no menu of the browser on ours either
    await pg.click(item("atom.element")); await pg.click(item("atom.el.N")); await shown(pg, false);
    assert((await doc()) !== before, "the element changed"); eq(await pg.evaluate("document.activeElement.id"), "d2-canvas");
    // it is the atom that was clicked that changed, and only that one: another atom gets its own element from its own menu
    const els = async () => JSON.parse(await doc()).atoms.map(a => a.el).join("");
    const changed = (await els()).indexOf("N"); assert(changed >= 0 && (await els()).split("N").length === 2, "one atom is nitrogen: " + (await els()));
    const other = (await firstLine(pg)).end;
    await open(...other, "atom"); await pg.click(item("atom.element")); await pg.click(item("atom.el.O")); await shown(pg, false);
    const now = await els(); eq([now.indexOf("N"), now.split("O").length - 1], [changed, 1]); assert(now.indexOf("O") !== changed, "another atom: " + now);
    await pg.keyboard.press("Control+z"); await pg.keyboard.press("Control+z"); eq(await doc(), before);
    await open(...atom, "atom"); await pg.click(item("atom.label")); await shown(pg, false);
    eq(await pg.evaluate("document.activeElement.id"), "d2-label");
    await pg.focus("#d2-canvas");
    // a bond: its order and its wedge; the item that is already the state is checked
    await open(...bond, "bond");
    eq((await menuNow(pg)).ids, ["bond.o.1", "bond.o.2", "bond.o.3", "bond.s.up", "bond.s.down", "bond.s.wavy", "bond.reverse", "bond.delete"]);
    // an item that is not offered is not on the keyboard walk
    await pg.focus("#d2-menu .ctx"); const walk = [];
    for (let i = 0; i < 9; i++) { await pg.keyboard.press("ArrowDown"); walk.push((await menuNow(pg)).active); }
    assert(!walk.includes("bond.reverse") && walk.includes("bond.delete") && walk.includes("bond.o.1"), "the walk skips the disabled item: " + walk);
    eq(await pg.getAttribute(item("bond.reverse"), "aria-disabled"), "true");
    await pg.click(item("bond.s.up")); await shown(pg, false);
    assert((await doc()) !== before, "the wedge was set");
    await open(...bond, "bond"); eq(await pg.getAttribute(item("bond.s.up"), "aria-checked"), "true"); eq(await pg.getAttribute(item("bond.reverse"), "aria-disabled"), null, "a marked bond can be reversed");
    await pg.keyboard.press("Escape"); await pg.keyboard.press("Control+z"); eq(await doc(), before);
    // an object: edit asks for the text field; delete is one undo step
    await open(cx + 250, cy + 100, "object"); eq((await menuNow(pg)).ids, ["obj.edit", "edit.delete"]);
    await pg.click(item("edit.delete")); await want(pg, { objects: 0 }); await pg.keyboard.press("Control+z"); await want(pg, { objects: 1 });
    // a selection: a click inside it keeps it; rotate is one undo step; copy as SMILES reaches the clipboard of the system
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+a"); await want(pg, { selected: 13 });
    await open(...atom, "selection"); eq((await state(pg)).selected, 13);
    const ids = (await menuNow(pg)).ids;
    for (const id of ["edit.cut", "edit.copy", "edit.paste", "edit.duplicate", "sel.rotate.l", "sel.flip.h", "edit.delete", "sel.copyas.molfile", "sel.copyas.smiles", "sel.copyas.svg", "sel.copyas.png"]) assert(ids.includes(id), id);
    await pg.click(item("sel.rotate.l")); assert((await doc()) !== before, "rotated"); await pg.keyboard.press("Control+z"); eq(await doc(), before);
    await open(...atom, "selection"); await pg.click(item("sel.copyas")); await pg.click(item("sel.copyas.smiles")); await shown(pg, false);
    await until(pg, "document.getElementById('d2-live').textContent.startsWith('Copied')");
    assert((await pg.evaluate("navigator.clipboard.readText()")).length > 3, "the SMILES is on the clipboard");
    // the menu closes when the module says it no longer describes the state: here the keys of the canvas change the selection, the
    // clipboard and the drawing while the menu is open (the focus is put back on the canvas by hand: the menu would keep it)
    const closesOn = async (target, keys, prepare = async () => { }) => {
      await pg.keyboard.press("Escape"); await pg.focus("#d2-canvas"); await prepare();
      await open(...atom, target); await pg.focus("#d2-canvas"); await pg.keyboard.press(keys); await shown(pg, false);
    };
    await closesOn("atom", "Control+a"); await want(pg, { selected: 13 });                  // the selection changed
    await closesOn("selection", "Control+c", async () => { await pg.keyboard.press("Control+a"); });       // the clipboard changed
    await pg.keyboard.press("Escape");
    const steps0 = await doc();
    await closesOn("atom", "Control+z"); assert((await doc()) !== steps0, "the drawing changed"); // the drawing changed
    await pg.keyboard.press("Control+Shift+z"); eq(await doc(), steps0);
    // a right-button drag pans the view and opens no menu; a click in place does
    await pg.keyboard.press("Escape"); await pg.focus("#d2-canvas");
    const where = (await firstLine(pg)).atom;
    // (the drag starts inside the canvas, 60 px from its left edge: the palettes at the sides take as much as they need)
    const [px, py] = empty;
    await pg.mouse.move(px, py); await pg.mouse.down({ button: "right" }); await pg.mouse.move(px + 60, py + 20, { steps: 5 }); await pg.mouse.up({ button: "right" });
    await shown(pg, false);
    const moved = (await firstLine(pg)).atom; eq([moved[0] - where[0], moved[1] - where[1]], [60, 20]);
    await pg.mouse.move(px + 60, py + 20); await pg.mouse.down({ button: "middle" }); await pg.mouse.move(px + 90, py + 20, { steps: 3 }); await pg.mouse.up({ button: "middle" });
    await shown(pg, false); eq((await firstLine(pg)).atom[0] - moved[0], 30);
    await pg.keyboard.press("Control+a"); await pg.keyboard.press("Escape");
    // an empty spot: the canvas menu; the menu closes on any change (here the wheel) and a stale one is refused
    await open(...empty, "canvas"); eq((await state(pg)).selected, 0);
    await pg.mouse.move(cx, cy); await pg.mouse.wheel(0, -120); await shown(pg, false);
    const atom2 = (await firstLine(pg)).atom;                           // the wheel moved the view
    await open(...atom2, "atom");
    await pg.evaluate("window.MZDRAW.command('undo')");                // a change the page does not see: the module still knows
    const after = await doc();
    await pg.click(item("atom.chg.plus")); await shown(pg, false);
    eq(await doc(), after); eq(await live(), await pg.evaluate("I18N.err({ error_key: 'error.draw2.contextStale', params: {} }).message"));
    await pg.keyboard.press("Control+Shift+z");
    // Clear all, then the empty document: no export item, a notice in words instead
    await open(...empty, "canvas"); await pg.click(item("canvas.clear")); await want(pg, { atoms: 0, objects: 0 });
    await open(...empty, "canvas");
    assert(!(await menuNow(pg)).ids.includes("canvas.save"), "no export on an empty canvas");
    eq(await pg.innerText("#d2-menu .note"), await pg.evaluate("I18N.t('draw2.ctx.empty')"));
    eq(await pg.getAttribute(item("canvas.clear"), "aria-disabled"), "true");
    await pg.keyboard.press("Escape");
    await ring(pg, cx, cy);
    await open(...empty, "canvas"); eq((await menuNow(pg)).ids.includes("canvas.save"), true);
    await shot(pg, "context-menu-light");
    await pg.keyboard.press("Escape");
  });

  await step("context menu, keyboard: the Menu key and Shift+F10 open it on the focused atom, arrows, Home, End and letters walk it, Enter runs, Escape gives the canvas back", async () => {
    await ring(pg, cx, cy); await pg.focus("#d2-canvas");
    await pg.keyboard.press("Home"); await until(pg, `${STATE}.focus === 1`);
    const before = await doc();
    await pg.keyboard.press("ContextMenu"); await shown(pg);
    let m = await menuNow(pg);
    eq([m.target, m.kind, m.active], ["atom", "key", "atom.element"]);
    eq([await pg.getAttribute("#d2-menu .ctx", "role"), await pg.getAttribute("#d2-menu .ctx", "aria-orientation")], ["menu", "vertical"]);
    await pg.keyboard.press("ArrowDown"); eq((await menuNow(pg)).active, "atom.chg.plus", "the focus moved to the next item");
    await pg.keyboard.press("ArrowUp"); eq((await menuNow(pg)).active, "atom.element");
    await pg.keyboard.press("End"); eq((await menuNow(pg)).active, "atom.connected");
    await pg.keyboard.press("ArrowDown"); eq((await menuNow(pg)).active, "atom.element", "the walk wraps");
    await pg.keyboard.press("ArrowRight"); eq((await menuNow(pg)).active, "atom.el.C", "Right opens the group");
    await pg.keyboard.press("ArrowLeft"); eq((await menuNow(pg)).active, "atom.element"); assert(!(await pg.isVisible(item("atom.el.N"))), "folded again");
    await pg.keyboard.press("c"); eq((await menuNow(pg)).active, "atom.chg.plus", "a letter goes to the next item that starts with it");
    await pg.keyboard.press("Home"); await pg.keyboard.press("ArrowDown"); await pg.keyboard.press("Enter");
    await shown(pg, false);
    eq(await pg.evaluate("document.activeElement.id"), "d2-canvas");
    assert((await doc()) !== before, "Enter ran the charge item");
    await pg.keyboard.press("Control+z"); eq(await doc(), before);
    // Enter on a group opens it and moves the focus in; Space runs an item as Enter does
    await pg.keyboard.press("ContextMenu"); await shown(pg);
    await pg.keyboard.press("Enter"); eq((await menuNow(pg)).active, "atom.el.C", "Enter on a group opens it and moves the focus in");
    await pg.keyboard.press("ArrowDown"); await pg.keyboard.press(" "); await shown(pg, false);
    assert((await doc()) !== before, "Space ran the element item"); await pg.keyboard.press("Control+z"); eq(await doc(), before);
    // Shift+F10 does the same; Escape closes it and nothing changed
    await pg.keyboard.press("Shift+F10"); await shown(pg); eq((await menuNow(pg)).target, "atom");
    await pg.keyboard.press("Escape"); await shown(pg, false); eq(await pg.evaluate("document.activeElement.id"), "d2-canvas"); eq(await doc(), before);
    // no focus and no selection: the middle of the canvas; the menu of an empty spot
    await pg.keyboard.press("Escape"); await pg.keyboard.press("ContextMenu"); await shown(pg);
    const c = await menuNow(pg);
    assert(["canvas", "atom", "bond"].includes(c.target), c.target);
    await pg.keyboard.press("Escape");
  });
  await ctx.close();

  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, colorScheme: "dark" }));
  await step("context menu, dark: the colours come from the page's variables", async () => {
    await ring(pg, cx, cy);
    const { atom } = await firstLine(pg);
    await pg.mouse.click(...atom, { button: "right" }); await shown(pg);
    await pg.click(item("atom.element"));
    const bg = await pg.evaluate("getComputedStyle(document.querySelector('#d2-menu .ctx')).backgroundColor");
    assert(bg !== "rgb(243, 244, 246)" && bg !== "rgba(0, 0, 0, 0)", "the dark panel colour " + bg);
    // the group is open and the menu grew: it still lies inside the window
    const r = await pg.locator("#d2-menu .ctx").boundingBox();
    assert(r.x >= 0 && r.y >= 0 && r.x + r.width <= 1280 && r.y + r.height <= 800, "inside the window: " + JSON.stringify(r));
    await shot(pg, "context-menu-dark");
    await pg.keyboard.press("Escape");
  });
  await ctx.close();

  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 768, height: 1024 }, deviceScaleFactor: 2, hasTouch: true, isMobile: true }));
  await step("context menu, touch and pen: a long press opens it (on an atom and on an empty canvas), a short tap draws, the items are 44 px and a tap runs one", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    await tool(pg, "ring:6a"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 6, bonds: 6 }); await tool(pg, "select");
    const atom = await rightmost(pg), before = await doc();
    // a short tap opens nothing; a finger held still opens the menu of what is under it and selects it
    await touch("touchStart", [atom]); await pg.waitForTimeout(100); await touch("touchEnd", []);
    await shown(pg, false);
    await touch("touchStart", [atom]); await pg.waitForTimeout(700); await touch("touchEnd", []);
    await shown(pg); let m = await menuNow(pg); eq([m.target, m.kind], ["atom", "touch"]); await want(pg, { selected: 1 });
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-menu .ctx > button')].filter(b => b.getBoundingClientRect().height < 44).length"), 0);
    const r = await pg.locator("#d2-menu .ctx").boundingBox();
    assert(r.x >= 0 && r.y >= 0 && r.x + r.width <= 768 && r.y + r.height <= 1024, "the menu is inside the window");
    await shot(pg, "context-menu-tablet");
    // a tap on an item runs it
    const [bx, by] = await center(pg, item("atom.chg.plus"));
    await pg.touchscreen.tap(bx, by); await shown(pg, false);
    assert((await doc()) !== before, "the charge item ran"); await pg.keyboard.press("Control+z"); eq(await doc(), before);
    // an empty spot held still: the canvas menu, nothing is drawn even with a tool that draws
    await tool(pg, "atom:N");
    await touch("touchStart", [[cx - 300, cy - 300]]); await pg.waitForTimeout(700); await touch("touchEnd", []);
    await shown(pg); eq((await menuNow(pg)).target, "canvas"); eq(await doc(), before);
    // a press elsewhere closes it and still draws
    await touch("touchStart", [[cx - 300, cy - 300]]); await touch("touchEnd", []);
    await shown(pg, false); await want(pg, { atoms: 7 }); await pg.keyboard.press("Control+z");
    // the pen: held still opens it (the pen kind), a stroke does not
    const pen = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1 });
    await pen("mouseMoved", ...atom); await pen("mousePressed", ...atom); await pg.waitForTimeout(700); await pen("mouseReleased", ...atom);
    await shown(pg); eq((await menuNow(pg)).kind, "pen");
    await pg.keyboard.press("Escape"); await shown(pg, false);
    // the barrel button of a pen asks for the menu at the press, and the tool does not act (no stray atom)
    await tool(pg, "atom:N"); const d0 = await doc();
    const barrel = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "right", buttons: type === "mousePressed" ? 2 : 0, clickCount: 1 });
    await barrel("mouseMoved", ...atom); await barrel("mousePressed", ...atom);
    await shown(pg); eq([(await menuNow(pg)).kind, (await menuNow(pg)).target], ["pen", "atom"]);
    await barrel("mouseReleased", ...atom); await pg.waitForTimeout(100);
    eq(await doc(), d0); assert((await menuNow(pg)) !== null, "the menu stays after the release");
    await pg.keyboard.press("Escape"); await shown(pg, false);
    // a menu from the keyboard on a device that also has a finger has the large targets
    await pg.focus("#d2-canvas"); await pg.keyboard.press("ContextMenu"); await shown(pg); eq((await menuNow(pg)).kind, "key");
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-menu .ctx button')].filter(b => b.offsetParent && b.getBoundingClientRect().height < 44).length"), 0);
    await pg.keyboard.press("Escape"); await shown(pg, false);
  });
  await ctx.close();
} finally {
  await browser.close();
  if (server) server.close();
}
for (const s of steps) console.log(" ", s[1] === "ok" ? "ok  " : "FAIL", s[0], s[1] === "ok" ? "" : s[1]);
for (const e of errs) console.log("  page:", e.join(" "));
console.log(`e2e ${base}: ${steps.filter(s => s[1] === "ok").length}/${steps.length} steps ok, ${errs.length} page errors`);
// the steps this file declares: one that does not run is a failure, and a PR that adds a step needs no edit here
const declared = (readFileSync(fileURLToPath(import.meta.url), "utf8").match(/await step\(/g) || []).length;
process.exit(steps.length === declared && !errs.length && steps.every(s => s[1] === "ok") ? 0 : 1);
