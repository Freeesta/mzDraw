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
/** a chiral centre drawn with the tools at (cx, cy): a wedge to an O, two plain bonds to an N and a C; returns the id of the centre and
 * `px(id)`, the pixel of an atom (the view is at its start: 40 px to the bond length, y up in the document) */
async function chiral(pg, cx, cy, tapAt) {
  const doc = async () => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')"));
  await tool(pg, "clear"); await want(pg, { atoms: 0 });
  await tool(pg, "bond:up"); await tapAt(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
  const d0 = await doc(), a = d0.bonds[0].a, o = d0.bonds[0].b;
  const px = async id => { const d = await doc(), A = d.atoms.find(x => x.id === a), P = d.atoms.find(x => x.id === id); return [cx + 40 * (P.x - A.x), cy - 40 * (P.y - A.y)]; };
  await tool(pg, "bond:1"); await tapAt(cx, cy); await tapAt(cx, cy); await want(pg, { atoms: 4, bonds: 3 });
  const ids = (await doc()).atoms.map(x => x.id).filter(id => id !== a && id !== o), [n, c] = ids;
  await tool(pg, "atom:O"); await tapAt(...await px(o));
  await tool(pg, "atom:N"); await tapAt(...await px(n));
  const smiles = await pg.evaluate("window.MZDRAW.save('smiles')");
  assert(/@/.test(smiles) && smiles.includes("N") && smiles.includes("O"), "a chiral centre is drawn: " + smiles);
  return { centre: a, o, n, c, px, smiles: () => pg.evaluate("window.MZDRAW.save('smiles')") };
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
async function context(browser, opts, init = null) {
  const ctx = await browser.newContext({ acceptDownloads: true, ...opts });
  if (init) await ctx.addInitScript(init);
  const pg = await ctx.newPage();
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
/** a scheme for `scheme:orient`: a methylpyridine, and its copy drawn turned by `deg` degrees 10 bond lengths to the right, free arrow between */
const orientScheme = deg => {
  const atoms = [], bonds = [], mol = (base, ox, d) => {
    const pts = [0, 1, 2, 3, 4, 5].map(i => [Math.cos(i * Math.PI / 3), Math.sin(i * Math.PI / 3)]);
    pts.push([pts[0][0] + 1, pts[0][1] + 0.0]);
    const cx = pts.reduce((a, p) => a + p[0], 0) / 7, cy = pts.reduce((a, p) => a + p[1], 0) / 7, t = d * Math.PI / 180;
    pts.forEach((p, i) => {
      const dx = p[0] - cx, dy = p[1] - cy, r4 = v => Math.round(v * 1e4) / 1e4;
      atoms.push({ id: base + i + 1, el: i === 3 ? "N" : "C", x: r4(ox + dx * Math.cos(t) - dy * Math.sin(t)), y: r4(dx * Math.sin(t) + dy * Math.cos(t)) });
    });
    for (let i = 0; i < 6; i++) bonds.push({ id: base + 50 + i, a: base + i + 1, b: base + (i + 1) % 6 + 1, o: 1 + i % 2 });
    bonds.push({ id: base + 56, a: base + 1, b: base + 7, o: 1 });
  };
  mol(0, 0, 0); mol(100, 10, deg);
  return JSON.stringify({ format: "mzdraw/1", next: 1000, atoms, bonds, objects: [{ id: 900, t: "arrow", x: 4, y: 0, x2: 6, y2: 0 }] });
};
/** the angle of the bond from atom `a` to atom `b` of a saved document, in degrees */
const bondAngle = (d, a, b) => { const p = d.atoms.find(x => x.id === a), q = d.atoms.find(x => x.id === b); return Math.atan2(q.y - p.y, q.x - p.x) * 180 / Math.PI; };

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
/** holds a finger or a pen still: waits until the PAGE's own clock (the one of the event timestamps, which the module reads the press time
 *  from) is `ms` past the moment the press was sent, so the press is long on the clock that decides it, whatever the speed of this
 *  machine or of the driver (a fixed sleep of the driver is not a duration of the page) */
const hold = async (pg, ms = 700) => { const t0 = await pg.evaluate("performance.now()"); await until(pg, `performance.now() >= ${t0 + ms}`); };
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

/** a SMILES file of the OpenSMILES file rules (CR/LF, a blank line, an indented line, a tab before a name, a broken line between good ones)
 * is put in the Add field and added with `press` (the keyboard or a tap): two structures open, the note names line 4, one undo step */
async function smilesFile(pg, press) {
  const before = await pg.evaluate("window.MZDRAW.save('mzdraw')");
  await tool(pg, "clear"); await want(pg, { atoms: 0 });
  await fillIn(pg, "#d2-text", "CCO\tethanol sample\r\n\r\n  CC(=O)O ignored\r\nC(C\r\nCC=O\r\n");
  await press();
  await want(pg, { atoms: 6, bonds: 4 });
  await until(pg, "document.querySelector('#d2-note').textContent.length > 0");
  const said = await pg.evaluate("I18N.t('draw2.import.smilesLine', { n: 4 })");
  assert(said.includes("4") && (await pg.innerText("#d2-note")).includes(said), "the note names line 4: " + await pg.innerText("#d2-note"));
  assert((await pg.evaluate("window.MZDRAW.save('molfile')")).startsWith("ethanol sample\n"), "the name is the title of the molfile");
  await tool(pg, "undo"); await want(pg, { atoms: 0 });
  await pg.evaluate(text => /** @type {any} */ (window).MZDRAW.open(text), before);       // the drawing the step found
}

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
    eq(await pg.evaluate("window.MZDRAW.version()"), NAMES.bridge);
    eq(await pg.title(), NAMES.name);                         // the tab carries the name from brand.toml (appname.js), never blank
    eq(await pg.innerText("h1"), NAMES.name);                 // and so does the heading, through {app} of the catalog
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

  await step("a SMILES file: lines read by the OpenSMILES file rules, the broken line named, typed into the field and added with the keyboard", async () => {
    await smilesFile(pg, async () => { await pg.focus("#d2-add"); await pg.keyboard.press("Enter"); });
  });

  await step("molfile (V2000 and V3000), SMILES, SVG, PNG and the mzdraw/1 document are saved as the module writes them; the pictures open again", async () => {
    const got = {};
    for (const [button, kind, name] of [["molfile", "molfile", "drawing.mol"], ["v3000", "molfile-v3000", "drawing-v3000.mol"], ["smiles", "smiles", "drawing.smi"], ["ket", "ket", "drawing.ket"], ["cdxml", "cdxml", "drawing.cdxml"], ["cml", "cml", "drawing.cml"],
      ["svg", "svg", "drawing.svg"], ["doc", "mzdraw", "drawing.mzdraw.json"]]) {
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
    // three lines are three records: the note says it, as it does for an SDF
    eq(await pg.innerText("#d2-note"), (await say(["draw2.import.placeholder", 1])) + " " + (await say(["draw2.import.records", 3])));
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

  await step("stereo guard, mouse and keyboard: a rigid nudge and a turn say nothing, a mirror and a drag of one neighbour say where the configuration changed (said and shown), and undo takes it back", async () => {
    const say = what => pg.evaluate(`I18N.t('draw2.say.stereoChanged', { what: ${JSON.stringify(String(what))} })`);
    const shown = () => pg.evaluate("document.querySelector('#d2-status .d2-notice')?.textContent.trim() || ''");
    const g = await chiral(pg, cx, cy, (x, y) => pg.mouse.click(x, y)), was = await g.smiles();
    await pg.focus("#d2-canvas"); await tool(pg, "select");
    await pg.keyboard.press("Control+a"); await pg.keyboard.press("ArrowRight");
    eq(await shown(), ""); assert(!(await pg.innerText("#d2-live")).includes("onfiguration"), "a nudge of the whole piece says nothing");
    await tool(pg, "rotate-l"); eq(await shown(), "");
    eq(await g.smiles(), was);
    const mid = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    await tool(pg, "flip-h");
    eq(await shown(), await say(g.centre)); eq(await pg.innerText("#d2-live"), await say(g.centre));
    assert((await g.smiles()) !== was, "the mirror image is the other enantiomer");
    await shot(pg, "plan-21-stereo-guard");
    await pg.keyboard.press("Control+z"); eq(await shown(), "");
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), mid); eq(await g.smiles(), was);
    // the mouse: the C neighbour is dragged to the other side of the centre
    await pg.mouse.click(cx + 250, cy + 250);
    const from = await g.px(g.c), at = await g.px(g.centre);
    await pg.mouse.move(...from); await pg.mouse.down();
    for (let i = 1; i <= 8; i++) await pg.mouse.move(from[0] + (2 * (at[0] - from[0])) * i / 8, from[1] + (2 * (at[1] - from[1])) * i / 8);
    eq(await shown(), "", "nothing is said while the drag is under way");
    await pg.mouse.up();
    assert((await g.smiles()) !== was, "the drag changed the configuration");
    eq(await shown(), await say(g.centre));
    // the keys: Alt+Shift+Arrow moves the selected atom one bond length; the C neighbour goes to the mirror point through the centre
    await pg.keyboard.press("Control+z"); eq(await g.smiles(), was);
    await pg.mouse.click(cx + 250, cy + 250);
    const f2 = await g.px(g.c), a2 = await g.px(g.centre);
    await pg.mouse.click(...f2); await want(pg, { selected: 1 });
    const nx = Math.round(2 * (a2[0] - f2[0]) / 40), ny = Math.round(2 * (a2[1] - f2[1]) / 40);
    let said = false;
    for (const [key, n] of [[nx > 0 ? "Alt+Shift+ArrowRight" : "Alt+Shift+ArrowLeft", Math.abs(nx)], [ny > 0 ? "Alt+Shift+ArrowDown" : "Alt+Shift+ArrowUp", Math.abs(ny)]])
      for (let i = 0; i < n; i++) { await pg.keyboard.press(key); said = said || (await shown()) === await say(g.centre); }
    assert(said, "a key nudge of one neighbour across the centre says where the configuration changed");
    assert((await g.smiles()) !== was, "the key nudges changed the configuration");
  });

  await step("scheme orient by keyboard: the button reached with the keys turns the structure after the arrow onto the one before it, one undo step; a second press says it is already oriented", async () => {
    const doc = async () => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')"));
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.evaluate(`(() => { const d = new DataTransfer(); d.setData('text/plain', ${JSON.stringify(orientScheme(70))}); document.dispatchEvent(new ClipboardEvent('paste', { clipboardData: d, bubbles: true, cancelable: true })); })()`);
    await want(pg, { atoms: 14, objects: 1 });
    const before = await doc(), ids = before.atoms.map(a => a.id).sort((a, b) => a - b);
    // the pasted atoms have new ids; the first seven are the source, the next seven the product
    assert(Math.abs(bondAngle(before, ids[7], ids[13]) - bondAngle(before, ids[0], ids[6])) > 30, "the product is drawn turned");
    const sel = '[data-testid="d2-scheme-orient"]'; await reveal(pg, sel); await pg.focus(sel); await pg.keyboard.press("Enter");
    const after = await doc();
    const turn = Math.abs((bondAngle(after, ids[7], ids[13]) - bondAngle(after, ids[0], ids[6]) + 540) % 360 - 180);
    assert(turn < 0.1, `the methyl bond of the product lies as the source's (off by ${turn} degrees)`);
    eq(await pg.innerText("#d2-live"), await pg.evaluate(`I18N.t("draw2.say.oriented", { what: "1" })`));
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+z"); eq(await doc(), before);                       // one undo step
    await pg.keyboard.press("Control+y"); await reveal(pg, sel); await pg.focus(sel); await pg.keyboard.press("Enter");
    await until(pg, "document.querySelector('#d2-live').textContent === I18N.t('draw2.say.orientDone')").catch(async e => { throw new Error(e.message + " | live: " + await pg.innerText("#d2-live") + " | " + await pg.evaluate("document.documentElement.dataset.d2")); });
    await tool(pg, "fit"); await shot(pg, "plan-14-scheme-orient");
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

  await step("text runs: a text with subscripts and a superscript is typed by keyboard and placed by mouse, a two-line text by the same field, one undo step each, the page draws them as runs", async () => {
    const objs = async () => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')")).objects || [];
    await tool(pg, "clear"); await want(pg, { atoms: 0, objects: 0 }); await tool(pg, "fit");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("t");
    await pg.keyboard.type("CH_3CO_2H, ^{13}C"); await pg.keyboard.press("Enter"); await want(pg, { tool: "text:CH_3CO_2H, ^{13}C" });
    await pg.mouse.click(cx, cy); await want(pg, { objects: 1 });
    let o = (await objs())[0];
    eq(o.runs.map(r => r.t), ["CH", "3", "CO", "2", "H, ", "13", "C"]);
    eq([o.runs.filter(r => r.sub).length, o.runs.filter(r => r.sup).length, o.text], [2, 1, "CH3CO2H, 13C"]);
    const svg = await pg.evaluate("window.MZDRAW.save('svg')");
    const fontOf = t => Number(new RegExp(`font-size="([0-9.]+)"[^>]*>${t}</text>`).exec(svg)?.[1]);
    assert(fontOf("3") < fontOf("CH") && fontOf("13") < fontOf("CH"), "scripts are drawn smaller than the text");
    await shot(pg, "text-runs");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("t");
    await pg.keyboard.type("Pd(OAc)_2\\n80 °C"); await pg.keyboard.press("Enter");
    await pg.mouse.click(cx, cy + 100); await want(pg, { objects: 2 });
    o = (await objs())[1];
    eq([o.text, o.runs.some(r => r.br)], ["Pd(OAc)2\n80 °C", true]);
    await tool(pg, "undo"); await want(pg, { objects: 1 });
    await tool(pg, "undo"); await want(pg, { objects: 0 });
  });

  await step("scheme row: a messy scheme is laid out on one line by the button, with the keyboard (select all, Enter on the button) and with the mouse; one undo step, the SVG equals the golden, a second press says it is done", async () => {
    const gold = JSON.parse(readFileSync(path.join(root, "tests/data/log/scheme-row.json"), "utf8")).cases[0];
    // the drawing is opened with its ids (as a saved file is), so the bytes can be the golden ones
    const openMessy = async () => { await pg.evaluate(text => /** @type {any} */ (window).MZDRAW.open(text), gold.request.import); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); await want(pg, { atoms: 8, objects: 4 }); };       // (the page repaints on the next command)
    await openMessy();
    const messy = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    // keyboard: select all, the button reached by its focus, Enter
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+a"); await want(pg, { selected: 16 });
    await reveal(pg, '[data-testid="d2-scheme-row"]'); await pg.focus('[data-testid="d2-scheme-row"]'); await pg.keyboard.press("Enter"); await frames(pg);
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), gold.out.mzdraw);
    eq(await pg.evaluate("window.MZDRAW.save('svg')"), gold.out.svg);
    await shot(pg, "plan-13-scheme-row");
    await tool(pg, "undo"); eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), messy);          // one step
    // mouse: nothing selected, everything is laid out
    await tool(pg, "select"); await pg.mouse.click(cx, cy + 300); await want(pg, { selected: 0 });
    await tool(pg, "scheme-row"); await frames(pg);
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), gold.out.mzdraw);
    eq(await pg.evaluate("window.MZDRAW.save('svg')"), gold.out.svg);
    // a second press changes nothing and says so: no new undo step
    await tool(pg, "scheme-row"); await frames(pg);
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), gold.out.mzdraw);
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('draw2.say.schemeRowDone')"));
    await tool(pg, "undo"); eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), messy);
  });

  // tp-06: "Arrange pathway", the button of the objects palette and the item of the canvas menu, on the messy star of tp-05 and the others of tests/data/log/pathway-page.json
  const pgold = JSON.parse(readFileSync(path.join(root, "tests/data/log/pathway-page.json"), "utf8")).cases;       // the page's golden: bytes of the command `pathway:tree`
  const cycle = JSON.parse(readFileSync(path.join(root, "tests/data/log/pathway.json"), "utf8")).refused[0].request.import;
  const PT = '[data-testid="d2-pathway-tree"]';
  const saved = (pg, what = "mzdraw") => pg.evaluate(`window.MZDRAW.save(${JSON.stringify(what)})`);
  const openDoc = async (pg, text, atoms) => { await pg.evaluate(t => /** @type {any} */ (window).MZDRAW.open(t), text); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); await want(pg, { atoms }); };

  await step("pathway: the button Arrange pathway lays the messy star out by mouse and by keyboard (the button, the context menu); the SVG equals the golden, one undo step, the line says what it did", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    // an empty drawing: the button says why and nothing changes
    await tool(pg, "pathway-tree"); await frames(pg);
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('error.draw2.schemeEmpty')"));
    const g = pgold[0];
    await openDoc(pg, g.request.import, 12);
    const messy = await saved(pg);
    await shot(pg, "tp-06-messy-star-before");
    // mouse
    await tool(pg, "pathway-tree"); await frames(pg);
    eq(await saved(pg), g.out.mzdraw); eq(await saved(pg, "svg"), g.out.svg);
    eq(await pg.innerText("#d2-live"), "Pathway arranged: 4 structures, 3 arrows, no gap widened.");
    // the page's log (a drawing that was opened has none before) is the end of the log of the replay of the command (its first operations make the opened drawing)
    const log = JSON.parse(await saved(pg, "log")), native = JSON.parse(g.out.log);
    assert(log.length > 0 && log.every(o => o.op === "move" || o.op === "obj.set"), "the log holds the moves and the routes of the command");
    eq(log, native.slice(native.length - log.length));
    await tool(pg, "fit"); await frames(pg);
    await shot(pg, "tp-06-messy-star-after");
    await tool(pg, "undo"); eq(await saved(pg), messy);                    // one step
    // keyboard: the focus on the button, Enter
    await reveal(pg, PT); await pg.focus(PT); await pg.keyboard.press("Enter"); await frames(pg);
    eq(await saved(pg), g.out.mzdraw); eq(await saved(pg, "svg"), g.out.svg);
    await tool(pg, "undo"); eq(await saved(pg), messy);
    // keyboard: the context menu of the canvas (the Menu key) and its item
    await pg.evaluate("window.MZDRAW.pan(0, 500)"); await tool(pg, "select"); await frames(pg);          // the keyboard menu opens at the middle of the canvas: bring an empty spot there
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Escape"); await pg.keyboard.press("ContextMenu");
    await until(pg, 'document.getElementById("d2-menu").hidden === false');
    const item = '[data-testid="d2-ctx-pathway.tree"]';
    assert((await pg.evaluate("document.getElementById('d2-menu').dataset.target")) === "canvas", "the menu of the keyboard is the one of the canvas: " + await pg.evaluate("document.getElementById('d2-menu').dataset.target"));
    eq(await pg.innerText(item + " .n"), await pg.evaluate("I18N.t('draw2.ctx.pathway.tree')"));
    await pg.focus(item); await pg.keyboard.press("Enter"); await frames(pg);
    eq(await saved(pg), g.out.mzdraw);
    await tool(pg, "undo"); eq(await saved(pg), messy);
    // mouse: the right button on an empty spot of the canvas, then the item
    const box = await pg.locator("#d2-canvas").boundingBox();
    await pg.mouse.click(box.x + 60, cy - 300, { button: "right" }); await until(pg, 'document.getElementById("d2-menu").hidden === false');
    await pg.click(item); await frames(pg);
    eq(await saved(pg), g.out.mzdraw);
    await tool(pg, "undo"); eq(await saved(pg), messy);
  });

  await step("pathway: a cycle is refused with its text and nothing moves; the other pathways of the golden are laid out (binary tree down, steeper limit, convergent DAG, unbalanced tree)", async () => {
    await openDoc(pg, cycle, 12);
    const before = await saved(pg);
    await tool(pg, "pathway-tree"); await frames(pg);
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('error.draw2.schemeCycle')"));
    assert(/cycle/.test(await pg.innerText("#d2-live")), "the text names the cycle");
    eq(await saved(pg), before);
    // the settings panel: the direction down, set from the select the schema gave
    await pg.click("#d2-style-more summary");
    eq(await pg.locator('[id="d2-style-pathway.dir"] option').allInnerTexts(), await pg.evaluate("['right', 'down'].map(v => I18N.t('draw2.style.choice.pathway.dir.' + v))"));
    eq(await pg.inputValue('[id="d2-style-pathway.dir"]'), "right");
    eq(await pg.inputValue('[id="d2-style-pathway.order"]'), "id");
    await pg.selectOption('[id="d2-style-pathway.dir"]', "down");
    await until(pg, "JSON.parse(window.MZDRAW.style()).style['pathway.dir'] === 'down'");
    await openDoc(pg, pgold[1].request.import, 45);
    const bin = await saved(pg);
    await shot(pg, "tp-06-binary-tree-before");
    await tool(pg, "pathway-tree"); await frames(pg);
    eq(await saved(pg), pgold[1].out.mzdraw); eq(await saved(pg, "svg"), pgold[1].out.svg);
    await tool(pg, "fit"); await frames(pg);
    await shot(pg, "tp-06-binary-tree-down");
    await tool(pg, "undo"); eq(await saved(pg), bin);
    await pg.selectOption('[id="d2-style-pathway.dir"]', "right");
    await until(pg, "!('pathway.dir' in JSON.parse(window.MZDRAW.style()).style)");                // the default is not written
    // a steeper limit: one gap is widened and the line says so
    await pg.fill('[id="d2-style-pathway.max_angle"]', "20"); await pg.keyboard.press("Enter");
    await until(pg, "JSON.parse(window.MZDRAW.style()).style['pathway.max_angle'] === 20");
    await openDoc(pg, pgold[3].request.import, 12);
    await tool(pg, "pathway-tree"); await frames(pg);
    eq(await saved(pg), pgold[3].out.mzdraw); eq(await saved(pg, "svg"), pgold[3].out.svg);
    eq(await pg.innerText("#d2-live"), "Pathway arranged: 4 structures, 3 arrows, 1 gap widened.");
    await pg.fill('[id="d2-style-pathway.max_angle"]', "40"); await pg.keyboard.press("Enter");
    await until(pg, "JSON.parse(window.MZDRAW.style()).style['pathway.max_angle'] === 40");
    // the DAG and the unbalanced tree (the kill test: before and after)
    for (const [i, name, atoms] of [[4, "dag", 15], [2, "unbalanced", 27]]) {
      await openDoc(pg, pgold[i].request.import, atoms);
      await shot(pg, `tp-06-${name}-before`);
      await tool(pg, "pathway-tree"); await frames(pg);
      eq(await saved(pg), pgold[i].out.mzdraw);
      await tool(pg, "fit"); await frames(pg);
      await shot(pg, `tp-06-${name}-after`);
    }
    await tool(pg, "clear");
    await pg.click("#d2-style-more summary");                                  // folded again, as it was found
  });

  await step("pathway: the button Label arrows writes the formula differences by mouse, keyboard and the canvas menu; the SVG equals the golden; the person's text is kept and painted as typed; the minus is painted as U+2212 and stored as ASCII; one undo step (tp-07)", async () => {
    const lg = JSON.parse(readFileSync(path.join(root, "tests/data/log/pathway-label.json"), "utf8"));
    const g = lg.cases[0], PL = '[data-testid="d2-pathway-label"]';
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "pathway-label"); await frames(pg);                          // nothing anchored
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('draw2.say.labelNone', {})"));
    await openDoc(pg, g.request.import, 13);
    const before = await saved(pg);
    await shot(pg, "tp-07-labels-before");
    await tool(pg, "pathway-label"); await frames(pg);                          // mouse
    eq(await saved(pg), g.out.mzdraw); eq(await saved(pg, "svg"), g.out.svg);
    // the data keeps the ASCII minus (the document and its log), the painted text has the true one (U+2212), and the person's texts are painted as typed
    const svg = await saved(pg, "svg");
    assert(svg.includes(">\u2212O<") && !svg.includes(">-O<") && svg.includes(">+CH2<") && svg.includes(">hydroxylation (CYP)<"), "the labels are painted, the minus is the true one");
    assert(JSON.parse(await saved(pg)).objects.some(o => o.text === "-O"), "the data holds the ASCII minus");
    eq(await pg.innerText("#d2-live"), "Arrows labelled: 4 arrows, 2 kept with your text, none skipped.");
    const log = JSON.parse(await saved(pg, "log")), native = JSON.parse(g.out.log);
    assert(log.length > 0 && log.every(o => o.op === "obj.set"), "the log holds obj.set operations only");
    eq(log, native.slice(native.length - log.length));
    await tool(pg, "fit"); await frames(pg);
    await shot(pg, "tp-07-labels-delta");
    await tool(pg, "undo"); eq(await saved(pg), before);                        // one step
    await reveal(pg, PL); await pg.focus(PL); await pg.keyboard.press("Enter"); await frames(pg);       // keyboard
    eq(await saved(pg), g.out.mzdraw);
    await tool(pg, "undo"); eq(await saved(pg), before);
    await pg.evaluate("window.MZDRAW.pan(0, 500)"); await tool(pg, "select"); await frames(pg);          // an empty spot at the middle of the canvas
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Escape"); await pg.keyboard.press("ContextMenu");   // the canvas menu
    await until(pg, 'document.getElementById("d2-menu").hidden === false');
    const item = '[data-testid="d2-ctx-pathway.label"]';
    eq(await pg.innerText(item + " .n"), await pg.evaluate("I18N.t('draw2.ctx.pathway.label')"));
    await pg.focus(item); await pg.keyboard.press("Enter"); await frames(pg);
    eq(await saved(pg), g.out.mzdraw);
    await tool(pg, "undo"); eq(await saved(pg), before);
    // `pathway.label` offers no select: its other words always fail until the mass callback exists (the refusals are in the golden results)
    await pg.click("#d2-style-more summary");
    eq(await pg.locator('[id="d2-style-pathway.label"]').count(), 0);
    await tool(pg, "clear");
    await pg.click("#d2-style-more summary");
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
    const schema = JSON.parse(await pg.evaluate("window.MZDRAW.settings_schema()"));
    eq(fields, schema.fields.map(f => f.key).concat(schema.choices.map(c => c.key), ["display.lens"], ["colour"]));          // the numbers, then the words of the pathway, then the diff lens, then the colours
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
    assert(!(await pg.evaluate("document.querySelector('#d2-keys').open")) && await pg.locator("#d2-keys li").count() === 7, "the keyboard help");
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
  await step("tablet with touch: the SMILES file is added by tapping Add, the broken line is named", async () => {
    await tapPalette(pg, "structure", true);                 // the field is in the Structure palette, which the tablet preset keeps hidden
    await smilesFile(pg, async () => { await pg.tap("#d2-add"); });
    await shot(pg, "arch-std-09-smiles-file-touch");
    await tapPalette(pg, "structure", false);               // as the next steps found it
  });
  await step("tablet with touch and a pen: a messy scheme is selected and laid out by the button, with a finger and with a pen, and each press is undone", async () => {
    const gold = JSON.parse(readFileSync(path.join(root, "tests/data/log/scheme-row.json"), "utf8")).cases[0];
    const openMessy = async () => { await pg.evaluate(text => /** @type {any} */ (window).MZDRAW.open(text), gold.request.import); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); await want(pg, { atoms: 8, objects: 4 }); };       // (the page repaints on the next command)
    await openMessy();
    const messy = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    await tool(pg, "all"); await want(pg, { selected: 16 });             // by tapping the select-all button
    await reveal(pg, '[data-testid="d2-scheme-row"]'); await pg.tap('[data-testid="d2-scheme-row"]'); await frames(pg);
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), gold.out.mzdraw);
    eq(await pg.evaluate("window.MZDRAW.save('svg')"), gold.out.svg);
    await shot(pg, "plan-13-scheme-row-touch");
    await tool(pg, "undo"); eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), messy);
    // a pen: the pointer of the pen presses the same button
    const cdp = await pg.context().newCDPSession(pg);
    await reveal(pg, '[data-testid="d2-scheme-row"]');
    const b = await pg.locator('[data-testid="d2-scheme-row"]').boundingBox(), px = b.x + b.width / 2, py = b.y + b.height / 2;
    await cdp.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: px, y: py, pointerType: "pen", button: "none", buttons: 0 });
    await cdp.send("Input.dispatchMouseEvent", { type: "mousePressed", x: px, y: py, pointerType: "pen", button: "left", buttons: 1, clickCount: 1 });
    await cdp.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: px, y: py, pointerType: "pen", button: "left", buttons: 0, clickCount: 1 });
    await frames(pg);
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), gold.out.mzdraw);
    await tool(pg, "undo"); eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), messy);
  });

  await step("tablet with touch and a pen: on a reloaded page the button Arrange pathway lays the messy star out by a tap and by a pen; a cycle is refused with its text; each press is undone", async () => {
    const g = JSON.parse(readFileSync(path.join(root, "tests/data/log/pathway-page.json"), "utf8")).cases[0];
    const cyc = JSON.parse(readFileSync(path.join(root, "tests/data/log/pathway.json"), "utf8")).refused[0].request.import;
    const PT = '[data-testid="d2-pathway-tree"]';
    await pg.reload(); await until(pg, "document.documentElement.dataset.d2Ready === '1'");
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    const open = async text => { await pg.evaluate(t => /** @type {any} */ (window).MZDRAW.open(t), text); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); };
    const saved = what => pg.evaluate(`window.MZDRAW.save(${JSON.stringify(what)})`);
    await open(g.request.import); await want(pg, { atoms: 12 });
    const messy = await saved("mzdraw");
    await reveal(pg, PT); await pg.tap(PT); await frames(pg);
    eq(await saved("mzdraw"), g.out.mzdraw); eq(await saved("svg"), g.out.svg);
    eq(await pg.innerText("#d2-live"), "Pathway arranged: 4 structures, 3 arrows, no gap widened.");
    await shot(pg, "tp-06-messy-star-touch");
    await tool(pg, "undo"); eq(await saved("mzdraw"), messy);
    // a pen: the pointer of the pen presses the same button
    const cdp = await pg.context().newCDPSession(pg);
    await reveal(pg, PT);
    const b = await pg.locator(PT).boundingBox(), px = b.x + b.width / 2, py = b.y + b.height / 2;
    await cdp.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: px, y: py, pointerType: "pen", button: "none", buttons: 0 });
    await cdp.send("Input.dispatchMouseEvent", { type: "mousePressed", x: px, y: py, pointerType: "pen", button: "left", buttons: 1, clickCount: 1 });
    await cdp.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: px, y: py, pointerType: "pen", button: "left", buttons: 0, clickCount: 1 });
    await frames(pg);
    eq(await saved("mzdraw"), g.out.mzdraw);
    await tool(pg, "undo"); eq(await saved("mzdraw"), messy);
    // a cycle: the refusal in words, nothing moves
    await open(cyc); await want(pg, { atoms: 12 });
    const before = await saved("mzdraw");
    await reveal(pg, PT); await pg.tap(PT); await frames(pg);
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('error.draw2.schemeCycle')"));
    eq(await saved("mzdraw"), before);
    await tool(pg, "clear");
  });

  await step("tablet with touch and a pen: a text with a subscript is placed by a tap and by a pen, as runs", async () => {
    await pg.reload(); await until(pg, "document.documentElement.dataset.d2Ready === '1'");
    await tool(pg, "clear"); await want(pg, { atoms: 0, objects: 0 });
    const saved = what => pg.evaluate(`window.MZDRAW.save(${JSON.stringify(what)})`);
    const objs = async () => JSON.parse(await saved("mzdraw")).objects || [];
    await reveal(pg, "#d2-text-tool"); await pg.fill("#d2-text-tool", "H_2O"); await pg.press("#d2-text-tool", "Enter");
    const b = await pg.locator("#d2-canvas").boundingBox();
    await pg.tap("#d2-canvas", { position: { x: b.width / 2, y: b.height / 3 } }); await want(pg, { objects: 1 });
    eq((await objs())[0].runs, [{ t: "H" }, { t: "2", sub: true }, { t: "O" }]);
    // a pen: the same text on another spot
    const cdp = await pg.context().newCDPSession(pg);
    const px = b.x + b.width / 2, py = b.y + 2 * b.height / 3;
    await cdp.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: px, y: py, pointerType: "pen", button: "none", buttons: 0 });
    await cdp.send("Input.dispatchMouseEvent", { type: "mousePressed", x: px, y: py, pointerType: "pen", button: "left", buttons: 1, clickCount: 1 });
    await cdp.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: px, y: py, pointerType: "pen", button: "left", buttons: 0, clickCount: 1 });
    await frames(pg); await want(pg, { objects: 2 });
    eq((await objs())[1].runs.length, 3);
    await shot(pg, "text-runs-touch");
    await tool(pg, "clear");
  });

  await step("tablet with touch and a pen: the button Label arrows writes the labels by a tap and by a pen, keeps the person's text, and each press is undone (tp-07)", async () => {
    const lg = JSON.parse(readFileSync(path.join(root, "tests/data/log/pathway-label.json"), "utf8")).cases[0];
    const PL = '[data-testid="d2-pathway-label"]';
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.evaluate(t => /** @type {any} */ (window).MZDRAW.open(t), lg.request.import); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg);
    const before = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    await reveal(pg, PL); await pg.tap(PL); await frames(pg);
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), lg.out.mzdraw); eq(await pg.evaluate("window.MZDRAW.save('svg')"), lg.out.svg);
    await shot(pg, "tp-07-labels-touch");
    await tool(pg, "undo"); eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), before);
    const cdp = await pg.context().newCDPSession(pg);
    await reveal(pg, PL);
    const b = await pg.locator(PL).boundingBox(), px = b.x + b.width / 2, py = b.y + b.height / 2;
    await cdp.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: px, y: py, pointerType: "pen", button: "none", buttons: 0 });
    await cdp.send("Input.dispatchMouseEvent", { type: "mousePressed", x: px, y: py, pointerType: "pen", button: "left", buttons: 1, clickCount: 1 });
    await cdp.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: px, y: py, pointerType: "pen", button: "left", buttons: 0, clickCount: 1 });
    await frames(pg);
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), lg.out.mzdraw);
    await tool(pg, "undo"); eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), before);
    await tool(pg, "clear");
  });

  await step("atom map by touch: tap the atom, tap the field, type, tap Set; the number is drawn and written to SMILES and RXN files", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "bond:1"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    await tool(pg, "select"); await pg.touchscreen.tap(cx, cy); await want(pg, { selected: 1 });
    await tapPalette(pg, "data", true);                 // the field is in the Data palette, which the tablet preset keeps hidden
    if (!(await pg.evaluate("document.querySelector('#d2-data-more').open"))) await pg.locator("#d2-data-more summary").evaluate(el => el.click());
    await until(pg, "document.querySelector('#d2-data-more').open"); await frames(pg);

    // the palettes lay themselves out again after a toggle or a scroll: tap where the target has stopped moving (two frames apart, the same box)
    const tapOn = async sel => {
      await pg.locator(sel).scrollIntoViewIfNeeded();
      let b = null;
      for (let i = 0; i < 200; i++) {
        await frames(pg); const c = await pg.locator(sel).boundingBox();
        if (b && c && b.x === c.x && b.y === c.y && b.width === c.width && b.height === c.height) break;
        b = c;
      }
      await pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2);
    };
    await tapOn("#d2-map"); await pg.keyboard.type("4"); await tapOn("#d2-map-set");
    await until(pg, "document.querySelector('#d2-map-note').textContent === I18N.t('draw2.map.done', { n: 4 })");
    assert((await pg.evaluate("window.MZDRAW.save('mzdraw')")).includes('"map":4'), "the document keeps the map");
    assert((await pg.evaluate("window.MZDRAW.save('smiles')")).includes(":4]"), "SMILES carries the class");
    assert((await pg.evaluate("window.MZDRAW.save('svg')")).includes(">4</text>"), "the number is drawn");
    await pg.locator("#d2-canvas").scrollIntoViewIfNeeded(); await shot(pg, "atom-map");
    await tool(pg, "undo"); assert(!(await pg.evaluate("window.MZDRAW.save('mzdraw')")).includes('"map"'), "undo takes it away");
    // a number out of range is refused in words and changes nothing
    await tool(pg, "select"); await pg.touchscreen.tap(cx, cy); await want(pg, { selected: 1 });
    await pg.fill("#d2-map", "1000"); await tapOn("#d2-map-set");
    await until(pg, "document.querySelector('#d2-map-note').textContent === I18N.t('error.draw2.map')");
    assert(!(await pg.evaluate("window.MZDRAW.save('mzdraw')")).includes('"map"'), "refused");
  });

  await step("atom map by keyboard: focus an atom with Home, Tab to the field, type, Enter; display.maps turns the number off", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "bond:1"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    await tool(pg, "select");
    await pg.locator("#d2-canvas").focus(); await pg.keyboard.press("Home"); await until(pg, "!!" + STATE + ".focus");
    await tapPalette(pg, "data", true);
    if (!(await pg.evaluate("document.querySelector('#d2-data-more').open"))) await pg.locator("#d2-data-more summary").evaluate(el => el.click());
    // a fraction is refused by the module in words (the page sends the raw text), nothing changes
    await pg.fill("#d2-map", "1.5"); await pg.locator("#d2-map").focus(); await pg.keyboard.press("Enter");
    await until(pg, "document.querySelector('#d2-map-note').textContent === I18N.t('error.draw2.map')");
    assert(!(await pg.evaluate("window.MZDRAW.save('mzdraw')")).includes('"map"'), "1.5 is no map");
    await pg.fill("#d2-map", ""); await pg.locator("#d2-map").focus(); await pg.keyboard.type("12"); await pg.keyboard.press("Enter");
    await until(pg, "document.querySelector('#d2-map-note').textContent === I18N.t('draw2.map.done', { n: 12 })");
    assert((await pg.evaluate("window.MZDRAW.save('molfile')")).includes("  0 12  0  0"), "the molfile atom line carries the number");
    assert((await pg.evaluate("window.MZDRAW.save('svg')")).includes(">12</text>"), "the number is drawn");
    await pg.evaluate("window.MZDRAW.set('display.maps=off')");
    assert(!(await pg.evaluate("window.MZDRAW.save('svg')")).includes(">12</text>"), "off: not drawn");
    assert((await pg.evaluate("window.MZDRAW.save('molfile')")).includes("  0 12  0  0"), "off: still written");
    // a format with no place for maps says so (the module counts, the page shows)
    eq(await pg.evaluate("window.MZDRAW.maps_lost('cdxml')"), 1); eq(await pg.evaluate("window.MZDRAW.maps_lost('molfile')"), 0);
    // saving as CDXML says what it cannot hold, under Save, by the keyboard path of the page (the file is still written)
    const [dl] = await Promise.all([pg.waitForEvent("download"), saveAs(pg, "#d2-ex-cdxml")]);
    eq(dl.suggestedFilename(), "drawing.cdxml");
    await until(pg, "document.querySelector('#d2-ex-note').textContent === I18N.t('draw2.export.mapsLost', { n: 1 })");
    await tapPalette(pg, "data", false);
  });

  await step("transformation by mouse and keyboard: parent, arrow and product are drawn, two atoms on each side are mapped by keyboard, trasf/1 is saved and opens again; an unmapped drawing is refused with its atoms", async () => {
    const doc = async () => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')")), key = k => pg.keyboard.press(k);
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "bond:1"); await pg.mouse.click(cx - 288, cy); await want(pg, { atoms: 2, bonds: 1 });
    await tool(pg, "arrow"); await pg.mouse.click(cx - 223, cy); await want(pg, { objects: 1 });
    await tool(pg, "bond:1"); await pg.mouse.click(cx - 108, cy); await want(pg, { atoms: 4, bonds: 2 });
    // nothing is mapped yet: the save is refused and the message names the four atoms
    const ids = (await doc()).atoms.map(a => a.id).join(", ");
    await saveAs(pg, "#d2-ex-trasf");
    await until(pg, "document.querySelector('#d2-ex-note').textContent.length > 0");
    eq(await pg.innerText("#d2-ex-note"), await pg.evaluate(`I18N.t('error.draw2.trasfMap', { atoms: ${JSON.stringify(ids)} })`));
    // the keyboard walks to each atom; the end atom of the product becomes oxygen
    await tapPalette(pg, "data", true);
    if (!(await pg.evaluate("document.querySelector('#d2-data-more').open"))) await pg.locator("#d2-data-more summary").evaluate(el => el.click());
    const goto = async i => {
      const a = (await doc()).atoms, base = i >= 2 ? 2 : 0, dx = a[i].x - a[base].x, dy = a[i].y - a[base].y;
      await pg.locator("#d2-canvas").focus(); await key("Home"); if (base) await key("PageDown");
      if (i !== base) await key(Math.abs(dx) >= Math.abs(dy) ? (dx > 0 ? "ArrowRight" : "ArrowLeft") : (dy > 0 ? "ArrowUp" : "ArrowDown"));
    };
    await goto(3); await key("o");
    for (const [i, n] of [[0, 1], [1, 2], [2, 1], [3, 2]]) {
      await goto(i); await pg.fill("#d2-map", String(n)); await pg.locator("#d2-map").focus(); await key("Enter");
      await until(pg, `document.querySelector('#d2-map-note').textContent === I18N.t('draw2.map.done', { n: ${n} })`);
    }
    // Save by keyboard: focus the button, Enter
    await reveal(pg, "#d2-ex-trasf"); await pg.focus("#d2-ex-trasf");
    const [dl] = await Promise.all([pg.waitForEvent("download"), key("Enter")]);
    eq(dl.suggestedFilename(), "drawing.trasf.json");
    const file = readFileSync(await dl.path(), "utf8");
    eq(file, await pg.evaluate("window.MZDRAW.save('trasf')"));
    const lines = file.trim().split("\n"), rec = JSON.parse(lines[0]);
    eq([lines.length, rec.format, rec.delta, rec.map.length, rec.added.length, rec.centre.length], [3, "trasf/1", "+O-CH2", 2, 0, 1]);
    assert(lines.slice(1).every(l => JSON.parse(l).format === "mol/1"), "the two records beside it");
    await shot(pg, "trasf-mapped");
    // opened again: parent, arrow, product, the maps drawn
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.setInputFiles("#d2-file", { name: "drawing.trasf.json", mimeType: "application/json", buffer: Buffer.from(file) }); await want(pg, { atoms: 4, bonds: 2, objects: 1 });
    eq((await doc()).atoms.map(a => a.map || 0).join(","), "1,2,1,2");
    eq((await pg.evaluate("window.MZDRAW.save('trasf')")).split("\n")[0], lines[0]);       // the same record (the layout of an opened file is its own, so the coordinates may differ)
    // a record that is not a pair is refused in words
    await pg.setInputFiles("#d2-file", { name: "bad.json", mimeType: "application/json", buffer: Buffer.from(lines.slice(0, 2).join("\n") + "\n") });
    await until(pg, "document.querySelector('#d2-note').textContent.length > 0");
    eq(await pg.innerText("#d2-note"), await pg.evaluate("I18N.t('error.draw2.trasf')"));
    await tapPalette(pg, "data", false);
  });

  await step("transformation by touch: the maps are set by tapping atoms, the field and Set; trasf/1 is saved by tapping", async () => {
    const doc = async () => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')"));
    const tap = (x, y) => pg.touchscreen.tap(x, y);       // the tablet: the Data palette, once shown, takes the right of the canvas (x 488 on, y 54 on), so the drawing stays left of it
    const tapOn = async sel => { await pg.locator(sel).scrollIntoViewIfNeeded(); const b = await pg.locator(sel).boundingBox(); await tap(b.x + b.width / 2, b.y + b.height / 2); };
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "bond:1"); await tap(cx - 288, cy); await want(pg, { atoms: 2, bonds: 1 });
    await tool(pg, "arrow"); await tap(cx - 223, cy); await want(pg, { objects: 1 });
    await tool(pg, "bond:1"); await tap(cx - 108, cy); await want(pg, { atoms: 4, bonds: 2 });
    const at = async i => { const a = (await doc()).atoms, base = i >= 2 ? 2 : 0, px = base ? cx - 108 : cx - 288; return [px + (a[i].x - a[base].x) * 40, cy - (a[i].y - a[base].y) * 40]; };
    await tool(pg, "atom:O"); await tap(...await at(3)); await want(pg, { atoms: 4, bonds: 2 });         // the end atom of the product becomes oxygen
    await tapPalette(pg, "data", true);
    if (!(await pg.evaluate("document.querySelector('#d2-data-more').open"))) await pg.locator("#d2-data-more summary").evaluate(el => el.click());
    await tool(pg, "select");
    for (const [i, n] of [[0, 1], [1, 2], [2, 1], [3, 2]]) {
      const [x, y] = await at(i);
      await pg.locator("#d2-canvas").scrollIntoViewIfNeeded(); await tap(x, y); await want(pg, { selected: 1 });
      await pg.fill("#d2-map", String(n)); await tapOn("#d2-map-set");
      await until(pg, `document.querySelector('#d2-map-note').textContent === I18N.t('draw2.map.done', { n: ${n} })`);
    }
    eq((await doc()).atoms.map(a => a.map || 0).join(","), "1,2,1,2");
    await reveal(pg, "#d2-ex-trasf");
    const [dl] = await Promise.all([pg.waitForEvent("download"), tapOn("#d2-ex-trasf")]);
    const lines = readFileSync(await dl.path(), "utf8").trim().split("\n");
    eq([lines.length, JSON.parse(lines[0]).format, JSON.parse(lines[0]).delta], [3, "trasf/1", "+O-CH2"]);
    // the same button pressed with a pen gives the same record
    const cdp = await pg.context().newCDPSession(pg);
    await reveal(pg, "#d2-ex-trasf");
    const bb = await pg.locator("#d2-ex-trasf").boundingBox(), pen = { x: bb.x + bb.width / 2, y: bb.y + bb.height / 2, button: "left", clickCount: 1, pointerType: "pen" };
    const [dl2] = await Promise.all([pg.waitForEvent("download"), (async () => { await cdp.send("Input.dispatchMouseEvent", { type: "mousePressed", ...pen }); await cdp.send("Input.dispatchMouseEvent", { type: "mouseReleased", ...pen }); })()]);
    eq(readFileSync(await dl2.path(), "utf8").trim().split("\n")[0], lines[0]);
    await shot(pg, "trasf-touch");
    await tapPalette(pg, "data", false);
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

  await step("style: the pathway fields hold their defaults, are set by touch and by keyboard, and a value out of bounds is refused with its key", async () => {
    const t = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, hasTouch: true });
    try {
      const f = k => `[id="d2-style-pathway.${k}"]`;
      const tap = async sel => { await t.pg.locator(sel).scrollIntoViewIfNeeded(); const b = await t.pg.locator(sel).boundingBox(); await t.pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2); };
      const style = async () => JSON.parse(await t.pg.evaluate("window.MZDRAW.style()")).style;
      await tap("#d2-style-more summary");
      eq([await t.pg.inputValue(f("col_gap")), await t.pg.inputValue(f("row_gap")), await t.pg.inputValue(f("max_angle")), await t.pg.inputValue(f("min_arrow"))], ["2.5", "1", "40", "1.2"]);
      // by touch: tap the field, enter the value
      await tap(f("col_gap")); await t.pg.fill(f("col_gap"), "3"); await t.pg.press(f("col_gap"), "Enter");
      await until(t.pg, "JSON.parse(window.MZDRAW.style()).style['pathway.col_gap'] === 3");
      // by keyboard: focus, type, Enter
      await t.pg.focus(f("max_angle")); await t.pg.fill(f("max_angle"), "30"); await t.pg.keyboard.press("Enter");
      await until(t.pg, "JSON.parse(window.MZDRAW.style()).style['pathway.max_angle'] === 30");
      // out of bounds: refused with its key, nothing changes
      await t.pg.fill(f("max_angle"), "95"); await t.pg.keyboard.press("Enter");
      await until(t.pg, "document.querySelector('[id=\"d2-style-pathway.max_angle\"]').getAttribute('aria-invalid') === 'true'");
      eq(await t.pg.innerText("#d2-style-note"), await t.pg.evaluate("I18N.t('error.draw2.styleValue')"));
      eq((await style())["pathway.max_angle"], 30);
      // the labels and the unit come from the language file
      eq(await t.pg.evaluate("I18N.t('draw2.style.unit.deg')"), "degrees");
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

  await step("scheme orient by touch and by pen: a tap and a pen press on the button turn the structure after the arrow onto the one before it, each one undo step", async () => {
    const doc = async () => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')"));
    const sel = '[data-testid="d2-scheme-orient"]';
    const load = async deg => {
      await tool(pg, "clear"); await want(pg, { atoms: 0 });
      await pg.evaluate(`(() => { const d = new DataTransfer(); d.setData('text/plain', ${JSON.stringify(orientScheme(deg))}); document.dispatchEvent(new ClipboardEvent('paste', { clipboardData: d, bubbles: true, cancelable: true })); })()`);
      await want(pg, { atoms: 14, objects: 1 });
    };
    const off = async () => { const d = await doc(), ids = d.atoms.map(a => a.id).sort((a, b) => a - b); return Math.abs((bondAngle(d, ids[7], ids[13]) - bondAngle(d, ids[0], ids[6]) + 540) % 360 - 180); };
    await load(70); assert(await off() > 30, "drawn turned");
    await reveal(pg, sel); const b = await pg.locator(sel).boundingBox();
    await pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2);
    await until(pg, "document.querySelector('#d2-live').textContent.length > 0");
    assert(await off() < 0.1, `a tap orients it (off by ${await off()})`);
    await tool(pg, "undo"); assert(await off() > 30, "one undo step");
    await load(130);
    const cdp = await pg.context().newCDPSession(pg);
    await reveal(pg, sel); const c = await pg.locator(sel).boundingBox();
    const at = { x: c.x + c.width / 2, y: c.y + c.height / 2, button: "left", clickCount: 1, pointerType: "pen" };
    await cdp.send("Input.dispatchMouseEvent", { type: "mousePressed", ...at }); await cdp.send("Input.dispatchMouseEvent", { type: "mouseReleased", ...at });
    assert(await off() < 0.1, `a pen orients it (off by ${await off()})`);
    await tool(pg, "fit"); await shot(pg, "plan-14-scheme-orient-touch");
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
    await hold(pg);
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
  await step("stereo guard by touch and pen: a tap on the mirror button, a finger drag and a pen drag of one neighbour across the others say where the configuration changed", async () => {
    const say = what => pg.evaluate(`I18N.t('draw2.say.stereoChanged', { what: ${JSON.stringify(String(what))} })`);
    const shown = () => pg.evaluate("document.querySelector('#d2-status .d2-notice')?.textContent.trim() || ''");
    const tapOn = async sel => { await reveal(pg, sel); await pg.locator(sel).scrollIntoViewIfNeeded(); const b = await pg.locator(sel).boundingBox(); await pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2); };
    const g = await chiral(pg, cx, cy, (x, y) => pg.touchscreen.tap(x, y)), was = await g.smiles();
    await tool(pg, "select"); await pg.keyboard.press("Control+a");
    await tapOn('[data-testid="d2-flip-h"]');
    eq(await shown(), await say(g.centre)); assert((await g.smiles()) !== was, "the mirror image");
    await tapOn('[data-testid="d2-undo"]'); eq(await shown(), ""); eq(await g.smiles(), was);
    // a finger drags the C neighbour to the other side of the centre
    const cdp = await pg.context().newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    await tool(pg, "select"); await pg.touchscreen.tap(cx + 250, cy + 250);
    const from = await g.px(g.c), at = await g.px(g.centre), to = i => [from[0] + 2 * (at[0] - from[0]) * i / 8, from[1] + 2 * (at[1] - from[1]) * i / 8];
    await touch("touchStart", [from]); for (let i = 1; i <= 8; i++) await touch("touchMove", [to(i)]);
    eq(await shown(), "", "nothing is said while the finger is down");
    await touch("touchEnd", []);
    assert((await g.smiles()) !== was, "the finger drag changed the configuration"); eq(await shown(), await say(g.centre));
    // (after a synthetic touch drag the test protocol drops the click of the next tap: the next presses are clicks)
    await tool(pg, "undo"); eq(await g.smiles(), was);
    // the pen does the same
    await pg.mouse.click(cx + 250, cy + 250);
    const pen = (type, x, y, buttons) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: "left", buttons, clickCount: 1 });
    await pen("mouseMoved", ...from, 0); await pen("mousePressed", ...from, 1);
    for (let i = 1; i <= 8; i++) await pen("mouseMoved", ...to(i), 1);
    eq(await shown(), "", "nothing is said while the pen is down");
    await pen("mouseReleased", ...to(8), 0);
    assert((await g.smiles()) !== was, "the pen drag changed the configuration"); eq(await shown(), await say(g.centre));
    // (last step of this context: the tap that follows a synthetic drag is dropped, so nothing that needs a tap comes after it)
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
    eq((await menuNow(pg)).ids, ["bond.o.1", "bond.o.2", "bond.o.3", "bond.s.up", "bond.s.down", "bond.s.wavy", "bond.reverse", "edit.colour", "colour.auto", "colour.black", "colour.red", "colour.orange", "colour.yellow", "colour.green", "colour.teal", "colour.blue", "colour.purple", "colour.magenta", "colour.brown", "colour.grey", "colour.custom", "bond.delete"]);   // the group Colour of colour-selection is folded in the menu and listed here with its children
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
    await open(cx + 250, cy + 100, "object"); eq((await menuNow(pg)).ids, ["obj.edit", "pathway.anchor", "pathway.free", "edit.colour", "colour.auto", "colour.black", "colour.red", "colour.orange", "colour.yellow", "colour.green", "colour.teal", "colour.blue", "colour.purple", "colour.magenta", "colour.brown", "colour.grey", "colour.custom", "edit.delete"]);
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
    await touch("touchStart", [atom]); await hold(pg); await touch("touchEnd", []);
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
    await touch("touchStart", [[cx - 300, cy - 300]]); await hold(pg); await touch("touchEnd", []);
    await shown(pg); eq((await menuNow(pg)).target, "canvas"); eq(await doc(), before);
    // a press elsewhere closes it and still draws
    await touch("touchStart", [[cx - 300, cy - 300]]); await touch("touchEnd", []);
    await shown(pg, false); await want(pg, { atoms: 7 }); await pg.keyboard.press("Control+z");
    // the pen: held still opens it (the pen kind), a stroke does not
    const pen = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1 });
    await pen("mouseMoved", ...atom); await pen("mousePressed", ...atom); await hold(pg); await pen("mouseReleased", ...atom);
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

  // tp-02: arrows anchored by hand. Two benzene rings 500 px apart (a bond length is 40 px); the arrow tool anchors an arrow drawn from one to the other.
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 }));
  const pathDoc = async pg => JSON.parse(await pg.evaluate("window.MZDRAW.save('mzdraw')"));
  /** the smallest atom id of the left ring and of the right ring (the ids a tool-made anchor carries) */
  const ringIds = async pg => { const d = await pathDoc(pg), mid = (Math.min(...d.atoms.map(a => a.x)) + Math.max(...d.atoms.map(a => a.x))) / 2; const m = f => Math.min(...d.atoms.filter(f).map(a => a.id)); return [m(a => a.x < mid), m(a => a.x > mid)]; };
  const twoRings = async (pg, cx, cy, dx) => {
    await tool(pg, "clear"); await tool(pg, "fit");
    await tool(pg, "ring:6a"); await pg.mouse.click(cx - dx, cy); await pg.mouse.click(cx + dx, cy); await want(pg, { atoms: 12, bonds: 12 });
  };
  /** the page's own log replayed by the module gives the page's document and picture: the CLI route gives the bytes of the page */
  const replayEq = async pg => pg.evaluate(`(() => {
    const log = JSON.parse(window.MZDRAW.save("log")), out = {};
    for (const k of ["mzdraw", "svg"]) out[k] = window.MZDRAW.replay(JSON.stringify({ log, out: k })) === window.MZDRAW.save(k);
    return out; })()`);
  await step("arrow tool, mouse and keyboard: an arrow drawn between two structures is anchored and routed at once; the parent moved with Alt+Arrow, the arrow follows; the page equals the replay of its log", async () => {
    await twoRings(pg, cx, cy, 250);
    await tool(pg, "arrow"); await pg.mouse.move(cx - 250, cy); await pg.mouse.down(); await pg.mouse.move(cx, cy + 6, { steps: 6 }); await pg.mouse.move(cx + 250, cy + 4, { steps: 6 }); await pg.mouse.up();
    await want(pg, { objects: 1 });
    let a = (await pathDoc(pg)).objects[0];
    eq([a.from, a.to], await ringIds(pg));                       // the smallest atom id of each ring
    assert(a.x > -250 / 40 + 1 && a.x2 < 250 / 40 - 1, "routed: the ends are outside the boxes of the rings");
    assert((await pg.innerText("#d2-live")).includes("\u2192"), "the live line says what was anchored");
    await shot(pg, "tp02-arrow-anchored");
    const before = await pathDoc(pg);
    // a free arrow (ended on empty canvas) stays free
    await tool(pg, "arrow"); await pg.mouse.move(cx - 60, cy + 120); await pg.mouse.down(); await pg.mouse.move(cx, cy + 120, { steps: 4 }); await pg.mouse.move(cx + 60, cy + 120, { steps: 4 }); await pg.mouse.up();
    await want(pg, { objects: 2 });
    const free = (await pathDoc(pg)).objects[1];
    eq([free.from || 0, free.to || 0], [0, 0]);
    await tool(pg, "undo"); await want(pg, { objects: 1 }); eq(await pathDoc(pg), before);
    // the parent is selected by a band and moved by the keyboard; the arrow follows (its tail moves up with the structure)
    await tool(pg, "select"); await pg.mouse.move(cx - 340, cy - 90); await pg.mouse.down(); await pg.mouse.move(cx - 250, cy, { steps: 4 }); await pg.mouse.move(cx - 160, cy + 90, { steps: 4 }); await pg.mouse.up();
    await until(pg, `${STATE}.selected >= 6`);
    await pg.focus("#d2-canvas");
    for (let i = 0; i < 3; i++) await pg.keyboard.press("Alt+ArrowUp");
    const moved = (await pathDoc(pg)).objects[0];
    assert(moved.y > a.y + 1e-6 || moved.y2 !== a.y2, "the arrow followed the move: " + JSON.stringify([a, moved]));
    eq([moved.from, moved.to], await ringIds(pg));
    await shot(pg, "tp02-arrow-followed");
    eq(await replayEq(pg), { mzdraw: true, svg: true });
  });
  await step("pathway: a cycle drawn by hand with the arrow tool (A to B, then B to A) is refused by Arrange pathway with its text and nothing moves; once the second arrow is undone the layout runs", async () => {
    await twoRings(pg, cx, cy, 250);
    const drag = async (x0, y0, x1, y1) => { await tool(pg, "arrow"); await pg.mouse.move(x0, y0); await pg.mouse.down(); await pg.mouse.move((x0 + x1) / 2, (y0 + y1) / 2, { steps: 5 }); await pg.mouse.move(x1, y1, { steps: 5 }); await pg.mouse.up(); };
    await drag(cx - 250, cy - 15, cx + 250, cy - 15); await want(pg, { objects: 1 });
    await drag(cx + 250, cy + 15, cx - 250, cy + 15); await want(pg, { objects: 2 });
    const [l, r] = await ringIds(pg), arrows = (await pathDoc(pg)).objects;
    eq(arrows.map(a => [a.from, a.to]), [[l, r], [r, l]]);                      // anchored both ways: a cycle
    const before = await pathDoc(pg);
    await tool(pg, "pathway-tree"); await frames(pg);
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('error.draw2.schemeCycle')"));
    eq(await pathDoc(pg), before);
    eq(await replayEq(pg), { mzdraw: true, svg: true });
    await tool(pg, "undo"); await want(pg, { objects: 1 });                     // one direction only: a pathway of two structures
    await tool(pg, "pathway-tree"); await frames(pg);
    eq(await pg.innerText("#d2-live"), "Pathway arranged: 2 structures, 1 arrow, no gap widened.");
    eq(await replayEq(pg), { mzdraw: true, svg: true });
  });
  await step("pathway:anchor by keyboard: a free arrow is selected, the context menu opens with Shift+F10, A then Enter anchors it; Route arrows (Menu key, R, Enter) puts displaced arrows back and the page equals the golden; one undo each", async () => {
    await twoRings(pg, cx, cy, 250);
    await tool(pg, "arrow"); await pg.mouse.move(cx - 180, cy + 10); await pg.mouse.down(); await pg.mouse.move(cx, cy + 10, { steps: 4 }); await pg.mouse.move(cx + 180, cy + 10, { steps: 4 }); await pg.mouse.up();
    await want(pg, { objects: 1 });
    let a = (await pathDoc(pg)).objects[0];
    eq([a.from || 0, a.to || 0], [0, 0]);                       // 30 px (3/4 of a bond length) from each ring: out of the box, so free
    await tool(pg, "select"); await pg.mouse.click(cx, cy + 10); await want(pg, { selected: 1 });
    const before = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Shift+F10"); await shown(pg);
    eq((await menuNow(pg)).target, "object");
    await pg.keyboard.press("a"); eq((await menuNow(pg)).active, "pathway.anchor");
    await pg.keyboard.press("Enter"); await shown(pg, false);
    a = (await pathDoc(pg)).objects[0];
    eq([a.from, a.to], await ringIds(pg));
    assert((await pg.innerText("#d2-live")).includes("\u2192"), "the live line says what was anchored");
    await pg.keyboard.press("Control+z"); eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), before);
    await pg.keyboard.press("Control+Shift+z"); eq([(await pathDoc(pg)).objects[0].from, (await pathDoc(pg)).objects[0].to], await ringIds(pg));
    // Route arrows (canvas menu) on arrows that are already routed moves nothing and says so
    eq(await replayEq(pg), { mzdraw: true, svg: true });
    const routed = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    await pg.mouse.click(cx, cy - 250, { button: "right" }); await shown(pg); eq((await menuNow(pg)).target, "canvas");
    assert(await pg.isVisible(item("pathway.route")), "the canvas menu has Route arrows");
    await pg.click(item("pathway.route")); await shown(pg, false);
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('draw2.say.routeNone')"));
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), routed);
    // a really displaced arrow: a document opened with raw arrows (anchored, but not where the route puts them); Route arrows by the keyboard
    // moves them, and the page equals the golden case of tests/data/log/pathway-tools.json (the native replay of the same request), not only its own log
    const gold = JSON.parse(readFileSync(path.join(root, "tests/data/log/pathway-tools.json"), "utf8")).cases.find(c => c.name.startsWith("pathway:route puts displaced"));
    await pg.evaluate(text => /** @type {any} */ (window).MZDRAW.open(text), gold.request.import);
    await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); await want(pg, { atoms: 6, objects: 2 });
    const raw = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    assert(raw !== gold.out.mzdraw, "the opened arrows are displaced");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Escape"); await pg.keyboard.press("ContextMenu"); await shown(pg);
    eq((await menuNow(pg)).target, "canvas");
    await pg.keyboard.press("r"); eq((await menuNow(pg)).active, "pathway.route");
    await pg.keyboard.press("Enter"); await shown(pg, false); await frames(pg);
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), gold.out.mzdraw);
    eq(await pg.evaluate("window.MZDRAW.save('svg')"), gold.out.svg);
    // the module replays the same request (the opened document and the command) to the same bytes: the CLI route of the page
    eq(await pg.evaluate(r => window.MZDRAW.replay(JSON.stringify({ ...r, out: "svg" })), gold.request), gold.out.svg);
    await pg.keyboard.press("Control+z"); eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), raw);          // one step
    await pg.keyboard.press("Control+Shift+z"); eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), gold.out.mzdraw);
  });
  await ctx.close();

  // the same on a tablet: a finger and a pen draw the arrow between two structures
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 768, height: 1024 }, deviceScaleFactor: 2, hasTouch: true, isMobile: true }));
  await step("arrow tool by touch and by pen: an arrow dragged from one structure to another is anchored and routed; the page equals the replay of its log", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    const pen = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1 });
    await tool(pg, "clear"); await tool(pg, "fit"); await tool(pg, "ring:6a");
    await pg.touchscreen.tap(cx - 160, cy); await pg.touchscreen.tap(cx + 160, cy); await want(pg, { atoms: 12, bonds: 12 });
    await tool(pg, "arrow");
    await touch("touchStart", [[cx - 160, cy]]); await touch("touchMove", [[cx - 60, cy + 4]]); await touch("touchMove", [[cx + 60, cy + 4]]); await touch("touchMove", [[cx + 160, cy]]); await touch("touchEnd", []);
    await want(pg, { objects: 1 });
    let a = (await pathDoc(pg)).objects[0];
    eq([a.from, a.to], await ringIds(pg));
    assert(a.x > -160 / 40 + 1 && a.x2 < 160 / 40 - 1, "routed");
    await shot(pg, "tp02-arrow-touch");
    await pen("mouseMoved", cx + 160, cy); await pen("mousePressed", cx + 160, cy); await pen("mouseMoved", cx, cy + 6); await pen("mouseMoved", cx - 160, cy); await pen("mouseReleased", cx - 160, cy);
    await want(pg, { objects: 2 });
    a = (await pathDoc(pg)).objects[1];
    eq([a.from, a.to], (await ringIds(pg)).reverse());           // right to left: from the second ring to the first
    eq(await replayEq(pg), { mzdraw: true, svg: true });
  });
  await ctx.close();

  // tp-03: Derive TP (menu item, Shift+D; off by default, setting pathway.derive) copies the structure with its maps and an anchored,
  // routed arrow; the page equals the golden after each way of asking for it
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, hasTouch: true }));
  const tpGold = JSON.parse(readFileSync(path.join(root, "tests/data/log/tp-derive.json"), "utf8")).cases.find(c => c.name.startsWith("a plain derive"));
  const tpDoc = () => pg.evaluate("window.MZDRAW.save('mzdraw')"), tpSvg = () => pg.evaluate("window.MZDRAW.save('svg')");
  const tpMenu = () => pg.evaluate(`(() => { const m = document.getElementById("d2-menu"); return m.hidden ? null : { target: m.dataset.target, kind: m.dataset.kind, ids: [...m.querySelectorAll("button[data-id]")].map(b => b.dataset.id) }; })()`);
  const tpShown = (on = true) => until(pg, `document.getElementById("d2-menu").hidden === ${!on}`);
  const tpOpen = async () => { await pg.evaluate(text => /** @type {any} */ (window).MZDRAW.open(text), tpGold.request.import); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); await want(pg, { atoms: 8, objects: 0 }); return tpDoc(); };
  const tpAtom = async () => { const d = await pg.evaluate("window.MZDRAW.scene().slice(0, 5)"), b = await pg.locator("#d2-canvas").boundingBox(); return [b.x + d[1], b.y + d[2]]; };
  const tpSame = async what => { eq(await tpDoc(), tpGold.out.mzdraw, what + ": document"); eq(await tpSvg(), tpGold.out.svg, what + ": picture"); await want(pg, { atoms: 16, objects: 1 }); };
  await step("tp:derive is off by default: no menu item, Shift+D does nothing; the setting pathway.derive=on offers both", async () => {
    const before = await tpOpen(); await shot(pg, "tp03-parent");
    const at = await tpAtom(); await pg.mouse.click(...at, { button: "right" }); await tpShown();
    assert(!(await tpMenu()).ids.includes("tp.derive"), "no Derive TP while the setting is off");
    await pg.keyboard.press("Escape"); await tpShown(false);
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Home"); await until(pg, `${STATE}.focus === 1`); await pg.keyboard.press("Shift+D"); await frames(pg);
    eq(await tpDoc(), before);
    await pg.evaluate("window.MZDRAW.set('pathway.derive=on')");
    await pg.mouse.click(...at, { button: "right" }); await tpShown();
    assert((await tpMenu()).ids.includes("tp.derive"), "Derive TP once it is on");
    await pg.keyboard.press("Escape"); await tpShown(false);
  });
  await step("tp:derive by mouse (right-click) and keyboard (Shift+D): the page equals the golden (document and picture), the copy is selected, a chemical edit of the copy through the page replays from the log, one undo each", async () => {
    await pg.evaluate("window.MZDRAW.set('pathway.derive=on')");
    let before = await tpOpen(); const at = await tpAtom();
    await pg.mouse.click(...at, { button: "right" }); await tpShown(); eq((await tpMenu()).target, "atom");
    eq((await pg.innerText(`[data-testid="d2-ctx-tp.derive"]`)).split("\n"), [await pg.evaluate("I18N.t('draw2.ctx.tp.derive')"), "Shift+D"]);
    await pg.click('[data-testid="d2-ctx-tp.derive"]'); await tpShown(false); await frames(pg);
    await tpSame("mouse"); await want(pg, { selected: 16 }); await shot(pg, "tp03-derived");
    // the next edit applies to the copy: Escape clears the selection, the menu of an atom of the copy changes its element, the parent stays
    const derived = await tpDoc(); await pg.focus("#d2-canvas"); await pg.keyboard.press("Escape");
    const d = await pg.evaluate("window.MZDRAW.scene()"), b = await pg.locator("#d2-canvas").boundingBox();
    const copyAtom = (() => { let best = null; for (let i = 0; i < d.length;) { const op = d[i]; if (op === 1) { for (const [x, y] of [[d[i + 1], d[i + 2]], [d[i + 3], d[i + 4]]]) if (!best || x > best[0]) best = [x, y]; i += 7; } else i += op === 2 ? 7 : op === 3 ? 6 : op === 6 ? 8 : 5; } return [b.x + best[0], b.y + best[1]]; })();
    await pg.mouse.click(...copyAtom, { button: "right" }); await tpShown(); eq((await tpMenu()).target, "atom");
    await pg.click('[data-testid="d2-ctx-atom.element"]'); await pg.click('[data-testid="d2-ctx-atom.el.N"]'); await tpShown(false);
    const edited = JSON.parse(await tpDoc());
    eq(edited.atoms.filter(a => a.id <= 8).map(a => a.el), JSON.parse(derived).atoms.filter(a => a.id <= 8).map(a => a.el));
    const nitrogens = ids => edited.atoms.filter(a => ids(a.id) && a.el === "N").length;
    eq([nitrogens(i => i <= 8), nitrogens(i => i > 8)], [1, 2], "the copy has one more nitrogen, the parent is as it was");
    eq(JSON.stringify(edited.atoms.filter(a => a.id <= 8)), JSON.stringify(JSON.parse(derived).atoms.filter(a => a.id <= 8)), "the parent atoms are unchanged");
    await shot(pg, "tp03-edited");
    eq(await pg.evaluate(`(() => { const log = JSON.parse(window.MZDRAW.save("log")); return window.MZDRAW.replay(JSON.stringify({ import: ${JSON.stringify(before)}, log, out: "svg" })) === window.MZDRAW.save("svg"); })()`), true);
    await pg.keyboard.press("Control+z"); eq(await tpDoc(), derived); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
    // keyboard: Home focuses the first atom, Shift+D derives
    before = await tpOpen(); await pg.focus("#d2-canvas"); await pg.keyboard.press("Home"); await until(pg, `${STATE}.focus === 1`);
    await pg.keyboard.press("Shift+D"); await frames(pg); await tpSame("keyboard");
    eq([JSON.parse(await tpDoc()).objects[0].from, JSON.parse(await tpDoc()).objects[0].to], [1, 17]);
    await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
  });
  await step("tp:derive by touch (long press, tap on the item) and by pen (barrel button, then the item): the page equals the golden; one undo each", async () => {
    await pg.evaluate("window.MZDRAW.set('pathway.derive=on')");
    const cdp = await ctx.newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    let before = await tpOpen(); let at = await tpAtom();
    await touch("touchStart", [at]); await hold(pg); await touch("touchEnd", []);
    await tpShown(); eq((await tpMenu()).kind, "touch");
    const [bx, by] = await center(pg, '[data-testid="d2-ctx-tp.derive"]');
    await pg.touchscreen.tap(bx, by); await tpShown(false); await frames(pg);
    await tpSame("touch"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
    before = await tpOpen(); at = await tpAtom();
    const barrel = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "right", buttons: type === "mousePressed" ? 2 : 0, clickCount: 1 });
    await barrel("mouseMoved", ...at); await barrel("mousePressed", ...at); await tpShown(); eq((await tpMenu()).kind, "pen"); await barrel("mouseReleased", ...at);
    const [px, py] = await center(pg, '[data-testid="d2-ctx-tp.derive"]');
    const pen = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1 });
    await pen("mouseMoved", px, py); await pen("mousePressed", px, py); await pen("mouseReleased", px, py);
    await tpShown(false); await frames(pg); await tpSame("pen");
    await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
  });
  // tp-11: the step field (the item "Apply step..." of the context menu; no key of its own): type a step, pick a site, derive. The rings and the ghost
  // are an overlay; the page equals the goldens of tests/data/log/step-field.json after the keyboard, the touch and the pen way
  const stGold = JSON.parse(readFileSync(path.join(root, "tests/data/log/step-field.json"), "utf8")).cases;
  const stField = async () => JSON.parse(await pg.evaluate("window.MZDRAW.step_field()"));
  const stOpen = async () => { await pg.evaluate("window.MZDRAW.set('pathway.derive=on')"); await pg.evaluate(text => { const m = /** @type {any} */ (window).MZDRAW; m.open(m.replay(JSON.stringify({ import: text, out: "mzdraw" }))); }, stGold[0].request.import); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); await want(pg, { atoms: 4, objects: 0 }); return tpDoc(); };
  /** the rings of the sites, in page pixels, in site order (the accent rings of the scene, the focused site's second ring dropped) */
  const stRings = async () => {
    const d = await pg.evaluate("Array.from(window.MZDRAW.scene())"), b = await pg.locator("#d2-canvas").boundingBox(), out = [];
    for (let i = 0; i < d.length;) { const op = d[i]; if (op === 5 && d[i + 4] === 11 && !out.some(p => Math.abs(p[0] - d[i + 1]) < 1 && Math.abs(p[1] - d[i + 2]) < 1)) out.push([d[i + 1], d[i + 2]]); i += op === 1 ? 7 : op === 2 ? 7 : op === 3 ? 6 : op === 6 ? 8 : 5; }
    return out.map(([x, y]) => [b.x + x, b.y + y]);
  };
  const stSame = async (gold, what) => { eq(await tpDoc(), gold.out.mzdraw, what + ": document"); eq(await tpSvg(), gold.out.svg, what + ": picture"); eq((await stField()).open, false); await want(pg, { atoms: 9, objects: 1 }); };
  const stMenuKey = async () => { await pg.focus("#d2-canvas"); await pg.keyboard.press("Home"); await until(pg, `${STATE}.focus === 1`); await pg.keyboard.press("Shift+F10"); await tpShown(); await pg.keyboard.press("A"); await pg.keyboard.press("A"); eq(await pg.evaluate("document.activeElement.dataset.id"), "tp.step"); await pg.keyboard.press("Enter"); await tpShown(false); await until(pg, `document.getElementById("d2-step").hidden === false && document.activeElement.id === "d2-step-text"`); };
  await step("step field by keyboard: Shift+F10 and the item Apply step... open it on the focused structure (Shift+T stays the text tool: the kill test of tp-11 did not pass), +O lists hydroxylation and n-oxidation, ] twice moves to the third site, Enter derives; the page equals the golden, the overlay is no operation, one undo", async () => {
    const before = await stOpen();
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Home"); await until(pg, `${STATE}.focus === 1`);
    eq((await stField()).open, false);
    await pg.keyboard.press("["); await pg.keyboard.press("]"); eq(await tpDoc(), before);                    // bare [ and ] are unbound with the field closed
    await pg.keyboard.press("Shift+T"); await frames(pg); eq(await pg.evaluate(`document.getElementById("d2-step").hidden`), true);   // no key of its own
    await pg.keyboard.press("Escape"); await pg.keyboard.press("Home");
    await stMenuKey();
    await pg.keyboard.type("+O"); await frames(pg);
    eq((await stField()).rows.map(r => r.name), ["hydroxylation", "n-oxidation"]);
    eq(await pg.locator("#d2-step-list li").allInnerTexts(), ["hydroxylation\n+O +15.9949", "n-oxidation\n+O +15.9949"]);
    eq((await stField()).sites, [1, 3, 4]);
    eq((await stRings()).length, 3, "a ring on each site");
    await shot(pg, "tp11-field");
    await pg.keyboard.press("]"); await pg.keyboard.press("]"); await frames(pg);
    eq((await stField()).site, 2);
    eq(await pg.innerText("#d2-step-line"), "hydroxylation, +O, at C4, site 3 of 3");
    eq(await tpDoc(), before, "the overlay is no operation");
    await pg.keyboard.press("Enter"); await frames(pg); await stSame(stGold[0], "keyboard");
    await until(pg, `document.getElementById("d2-step").hidden`);
    await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
    // Escape closes the field and leaves the drawing as it was
    await stMenuKey();
    await pg.keyboard.press("Escape"); await until(pg, `document.getElementById("d2-step").hidden`); eq(await tpDoc(), before);
    // Alt+[ and Alt+] still turn the selection with the field open
    await stMenuKey(); await pg.keyboard.press("Alt+["); eq((await stField()).open, true);
    // "Apply step..." chosen again while the field is open: a fresh field, the input shows the module's (empty) text and the list is whole
    await pg.keyboard.type("+O"); await frames(pg); eq(await pg.inputValue("#d2-step-text"), "+O");
    await pg.mouse.click(...(await stRings())[0], { button: "right" }); await tpShown(); await pg.click('[data-testid="d2-ctx-tp.step"]'); await tpShown(false); await frames(pg);
    eq(await pg.inputValue("#d2-step-text"), ""); eq((await stField()).rows.length, 13); eq(await pg.locator("#d2-step-list li").count(), 13);
    await pg.keyboard.press("Escape");
  });
  await step("step field by touch: long press the structure, Apply step..., tap the second ring, tap Derive; the page equals the golden, one undo", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    const before = await stOpen();
    await stMenuKey(); await pg.keyboard.type("+O");
    const rings = await stRings(); await pg.keyboard.press("Escape"); await until(pg, `document.getElementById("d2-step").hidden`);
    await touch("touchStart", [rings[0]]); await hold(pg); await touch("touchEnd", []);
    await tpShown(); eq((await tpMenu()).kind, "touch");
    eq((await pg.innerText(`[data-testid="d2-ctx-tp.step"]`)).split("\n"), [await pg.evaluate("I18N.t('draw2.ctx.tp.step')")]);
    const [bx, by] = await center(pg, '[data-testid="d2-ctx-tp.step"]');
    await pg.touchscreen.tap(bx, by); await tpShown(false); await until(pg, `document.getElementById("d2-step").hidden === false`);
    await pg.fill("#d2-step-text", "+O"); await frames(pg);
    const two = await stRings(); eq(two.length, 3);
    await pg.touchscreen.tap(...two[1]); await frames(pg);
    eq((await stField()).site, 1); eq(await tpDoc(), before, "a tap on a ring makes no operation");
    await shot(pg, "tp11-field-touch");
    const [dx, dy] = await center(pg, "#d2-step-derive"); await pg.touchscreen.tap(dx, dy); await frames(pg);
    await stSame(stGold[1], "touch"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
  });
  await step("step field by pen: barrel button on the structure, Apply step..., a pen tap on the second ring, a pen tap on Derive; the page equals the golden, one undo", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const pen = (type, x, y, barrel = false) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : barrel ? "right" : "left", buttons: type === "mousePressed" ? (barrel ? 2 : 1) : 0, clickCount: 1 });
    const tap = async (x, y) => { await pen("mouseMoved", x, y); await pen("mousePressed", x, y); await pen("mouseReleased", x, y); };
    const before = await stOpen();
    await stMenuKey(); await pg.keyboard.type("+O");
    const rings = await stRings(); await pg.keyboard.press("Escape"); await until(pg, `document.getElementById("d2-step").hidden`);
    await pen("mouseMoved", ...rings[0], true); await pen("mousePressed", ...rings[0], true); await tpShown(); eq((await tpMenu()).kind, "pen"); await pen("mouseReleased", ...rings[0], true);
    await tap(...await center(pg, '[data-testid="d2-ctx-tp.step"]')); await tpShown(false); await until(pg, `document.getElementById("d2-step").hidden === false`);
    await pg.fill("#d2-step-text", "+O"); await frames(pg);
    await tap(...(await stRings())[1]); await frames(pg);
    eq((await stField()).site, 1); eq(await tpDoc(), before, "a tap on a ring makes no operation");
    await tap(...await center(pg, "#d2-step-derive")); await frames(pg);
    await stSame(stGold[1], "pen"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
  });
  await step("step field: a mass shift filters too (+15.99 lists the same two rows with their shifts), a row is picked by a tap and gives n-oxidation; with the pathway features off the menu has no such item", async () => {
    const before = await stOpen();
    await stMenuKey(); await pg.keyboard.type("+15.99"); await frames(pg);
    eq((await stField()).rows.map(r => [r.name, r.mass]), [["hydroxylation", "+15.9949"], ["n-oxidation", "+15.9949"]]);
    await pg.click("#d2-step-list li:nth-child(2)"); await frames(pg);
    eq((await stField()).step, "n-oxidation");
    await pg.click("#d2-step-derive"); await frames(pg); await stSame(stGold[2], "pick");
    await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
    await pg.evaluate("window.MZDRAW.set('pathway.derive=off')");
    const at = (await stRings())[0] ?? null; eq(at, null, "no ring with the field closed");
    await pg.mouse.click(...(await tpAtom()), { button: "right" }); await tpShown();
    assert(!(await tpMenu()).ids.includes("tp.step"), "no Apply step while the setting is off");
    await pg.keyboard.press("Escape"); await tpShown(false);
  });
  await step("step field by keyboard: Ctrl+Alt+Z, A, D and Y typed in the open input (AltGr arrives as Ctrl and Alt) are not the drawing's undo, select-all, duplicate or redo: the document is unchanged and the field still takes text, a bracket key and Escape", async () => {
    const before = await stOpen();
    await stMenuKey(); await pg.keyboard.type("+O"); await frames(pg);
    for (const k of ["Control+Alt+z", "Control+Alt+a", "Control+Alt+d", "Control+Alt+y", "Control+Alt+Shift+z"]) { await pg.keyboard.press(k); await frames(pg); eq(await tpDoc(), before, k); eq((await stField()).open, true, k); }
    eq(await pg.inputValue("#d2-step-text"), "+O");
    eq((await stField()).rows.map(r => r.name), ["hydroxylation", "n-oxidation"]);
    await pg.keyboard.press("Control+Alt+]"); await frames(pg); eq((await stField()).site, 1);   // the bracket keys of AltGr are still the field's
    await pg.keyboard.press("Escape"); await until(pg, `document.getElementById("d2-step").hidden`); eq(await tpDoc(), before);
  });
  // colour-selection: a part of the drawing is selected and a colour is clicked (the toolbar flyout, the field for a typed colour, the context
  // menu). The module judges and applies it as one undo step; the page equals the goldens of tests/data/log/colour.json after each way
  const clGold = JSON.parse(readFileSync(path.join(root, "tests/data/log/colour.json"), "utf8")).cases;
  const clOpen = async () => { await pg.evaluate(text => /** @type {any} */ (window).MZDRAW.open(text), clGold[0].request.import); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); await want(pg, { atoms: 4 }); return tpDoc(); };
  const clSame = async (gold, what) => { eq(await tpDoc(), gold.out.mzdraw, what + ": document"); eq(await tpSvg(), gold.out.svg, what + ": picture"); };
  /** the pixel of the oxygen of the drawing (its label in the display list) */
  const clOxygen = async () => {
    const d = await pg.evaluate("Array.from(window.MZDRAW.scene())"), t = await pg.evaluate("Array.from(window.MZDRAW.strings())"), b = await pg.locator("#d2-canvas").boundingBox();
    for (let i = 0; i < d.length;) { const op = d[i]; if (op === 2 && t[d[i + 5]] === "O") return [b.x + d[i + 1], b.y + d[i + 2]]; i += op === 1 ? 7 : op === 2 ? 7 : op === 3 ? 6 : op === 6 ? 8 : 5; }
    throw new Error("no oxygen in the display list");
  };
  await step("colour by keyboard: select all (Ctrl+A), reach the Colour flyout of the toolbar by the arrow key, Enter on Red; the page equals the golden, the name is text, the colour in use is pressed, one undo; Automatic gives the element colours back", async () => {
    const before = await clOpen();
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+a"); await until(pg, `${STATE}.selected > 0`);
    await reveal(pg, '[data-testid="d2-colour-red"]');
    // the name of each colour is text, not colour only
    eq(await pg.locator("#d2-colour-list .txt").allInnerTexts(), ["Automatic", "Black", "Red", "Orange", "Yellow", "Green", "Teal", "Blue", "Purple", "Magenta", "Brown", "Grey"]);
    await pg.focus('[data-testid="d2-colour-red"]'); await pg.keyboard.press("Enter"); await frames(pg);
    await clSame(clGold[2], "keyboard, palette");
    eq(await pg.getAttribute('[data-testid="d2-colour-red"]', "aria-pressed"), "true");
    eq(await pg.innerText("#d2-colour-note"), "Colour set: Red.");
    await shot(pg, "colour-flyout"); await shotEl(pg, "#d2-fly-colour", "colour-flyout-el");
    await pg.keyboard.press("Escape"); await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
    // Automatic takes the colour away again: the document is as it was before the first colour
    await pg.keyboard.press("Control+y"); await clSame(clGold[2], "redo");
    await reveal(pg, '[data-testid="d2-colour-auto"]'); await pg.focus('[data-testid="d2-colour-auto"]'); await pg.keyboard.press("Enter"); await frames(pg);
    eq(await tpDoc(), before); eq(await pg.getAttribute('[data-testid="d2-colour-auto"]', "aria-pressed"), "true");
  });
  await step("colour by keyboard, typed: the focused atom (Home) takes #0F8 from the field and Enter; the page equals the golden; a word that is no colour is refused with its text and changes nothing", async () => {
    const before = await clOpen();
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Home"); await until(pg, `${STATE}.focus === 1`);
    await fillIn(pg, "#d2-colour-hex", "red"); await pg.keyboard.press("Enter"); await frames(pg);
    eq(await tpDoc(), before); eq(await pg.getAttribute("#d2-colour-hex", "aria-invalid"), "true");
    eq(await pg.innerText("#d2-colour-note"), await pg.evaluate("I18N.t('error.draw2.colour')"));
    await pg.fill("#d2-colour-hex", "#0F8"); await pg.keyboard.press("Enter"); await frames(pg);
    await clSame(clGold[3], "typed");
    eq(await pg.inputValue("#d2-colour-hex"), "");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
    // nothing selected, nothing focused: the word says so
    const area = await pg.locator("#d2-canvas").boundingBox(); await pg.mouse.click(area.x + 12, area.y + area.height - 12); await frames(pg);
    await pg.keyboard.press("Escape");
    await fillIn(pg, "#d2-colour-hex", "#d00000"); await pg.keyboard.press("Enter"); await frames(pg);
    eq(await tpDoc(), before); eq(await pg.innerText("#d2-colour-note"), await pg.evaluate("I18N.t('error.draw2.colourNone')"));
  });
  await step("colour: the mono style does not erase an explicit colour (the page equals the golden picture), and the files with no place for colours say what they left out", async () => {
    await pg.evaluate(text => { const m = /** @type {any} */ (window).MZDRAW; m.open(text); m.set("style.colour=mono"); }, clGold[1].out.mzdraw);
    await frames(pg); eq(await tpSvg(), clGold[1].out.svg);
    await pg.evaluate("window.MZDRAW.set('style.colour=element')");
    eq(await pg.evaluate("window.MZDRAW.colours_lost('molfile')"), 4); eq(await pg.evaluate("window.MZDRAW.colours_lost('cdxml')"), 0); eq(await pg.evaluate("window.MZDRAW.colours_lost('svg')"), 0);
    await saveAs(pg, "#d2-ex-molfile"); await until(pg, `document.getElementById("d2-ex-note").textContent.includes("colours")`);
    eq(await pg.innerText("#d2-ex-note"), "This format has no place for colours: 4 atoms, bonds and objects are written without theirs.");
  });
  await step("colour by mouse: right-click the oxygen, the group Colour, Blue; the page equals the golden, the item that is in use is checked, one undo", async () => {
    const before = await clOpen(), at = await clOxygen();
    await pg.mouse.click(...at, { button: "right" }); await tpShown();
    assert((await tpMenu()).ids.includes("colour.blue") && (await tpMenu()).ids.includes("colour.auto") && (await tpMenu()).ids.includes("colour.custom"), "the colour items");
    await pg.click('[data-testid="d2-ctx-edit.colour"]'); await pg.click('[data-testid="d2-ctx-colour.blue"]'); await tpShown(false); await frames(pg);
    await clSame(clGold[4], "mouse");
    await pg.mouse.click(...at, { button: "right" }); await tpShown(); await pg.click('[data-testid="d2-ctx-edit.colour"]');
    eq(await pg.getAttribute('[data-testid="d2-ctx-colour.blue"]', "aria-checked"), "true");
    // Custom... gives the keyboard to the field of the flyout
    await pg.click('[data-testid="d2-ctx-colour.custom"]'); await tpShown(false); await until(pg, `document.activeElement.id === "d2-colour-hex"`);
    await pg.keyboard.press("Escape"); await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
  });
  /** the Colour flyout of the toolbar open (a tap on its opener when it is shut: one tap toggles it) */
  const clFly = async tap => { const red = '[data-testid="d2-colour-red"]'; if (await pg.isVisible(red)) return; await reveal(pg, '[data-testid="d2-colour"]'); await tap(...await center(pg, '[data-testid="d2-colour"]')); await until(pg, `!document.getElementById("d2-fly-colour").hidden`); };
  await step("colour by touch: long press the oxygen, tap Colour, tap Blue; the page equals the golden, one undo", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    const before = await clOpen(), at = await clOxygen();
    await touch("touchStart", [at]); await hold(pg); await touch("touchEnd", []);
    await tpShown(); eq((await tpMenu()).kind, "touch");
    await pg.touchscreen.tap(...await center(pg, '[data-testid="d2-ctx-edit.colour"]'));
    await pg.touchscreen.tap(...await center(pg, '[data-testid="d2-ctx-colour.blue"]')); await tpShown(false); await frames(pg);
    await clSame(clGold[4], "touch"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
    // the flyout of the toolbar by touch: select all by the keyboard-free route is the toolbar's Select all, then a tap on Red
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+a");
    await clFly((x, y) => pg.touchscreen.tap(x, y));
    const box = await pg.locator('[data-testid="d2-colour-red"]').boundingBox(); assert(box.height >= 40, "a finger-size swatch: " + box.height);
    await shot(pg, "colour-flyout-touch");
    await pg.touchscreen.tap(...await center(pg, '[data-testid="d2-colour-red"]')); await frames(pg);
    await clSame(clGold[2], "touch, toolbar");
    await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
  });
  await step("colour by pen: barrel button on the oxygen, Colour, Blue; a pen tap on Red of the flyout after Select all; the page equals the goldens, one undo each", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const pen = (type, x, y, barrel = false) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : barrel ? "right" : "left", buttons: type === "mousePressed" ? (barrel ? 2 : 1) : 0, clickCount: 1 });
    const tap = async (x, y) => { await pen("mouseMoved", x, y); await pen("mousePressed", x, y); await pen("mouseReleased", x, y); };
    const before = await clOpen(), at = await clOxygen();
    await pen("mouseMoved", ...at, true); await pen("mousePressed", ...at, true); await tpShown(); eq((await tpMenu()).kind, "pen"); await pen("mouseReleased", ...at, true);
    await tap(...await center(pg, '[data-testid="d2-ctx-edit.colour"]')); await tap(...await center(pg, '[data-testid="d2-ctx-colour.blue"]')); await tpShown(false); await frames(pg);
    await clSame(clGold[4], "pen"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+a");
    await clFly(tap); await tap(...await center(pg, '[data-testid="d2-colour-red"]')); await frames(pg);
    await clSame(clGold[2], "pen, toolbar"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
  });
  // mass-label-ui: the mass-label choice of a scheme (document field `labels`, operation `doc.labels`). The items of the canvas menu choose the mode and
  // the adduct; the module writes the field and the labels in one undo step; the page equals the goldens of tests/data/log/mass-label-ui.json
  const mlGold = JSON.parse(readFileSync(path.join(root, "tests/data/log/mass-label-ui.json"), "utf8")).values;
  const mlOpen = async () => { await pg.evaluate(text => { const m = /** @type {any} */ (window).MZDRAW; m.open(m.replay(JSON.stringify({ import: text, out: "mzdraw" }))); }, mlGold[0].request.import); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); await want(pg, { atoms: 9 }); return tpDoc(); };
  const mlItem = id => `[data-testid="d2-ctx-${id}"]`;
  const mlLabels = () => pg.evaluate("JSON.parse(window.MZDRAW.state()).labels");
  await step("mass labels by mouse: right-click the canvas, the group Label mode, delta+mass, then the group Label adduct; the page equals the goldens, one undo each; the button says, for assistive tools, what the mode means", async () => {
    const before = await mlOpen(), area = await pg.locator("#d2-canvas").boundingBox(), at = [area.x + 12, area.y + area.height - 12];
    eq((await mlLabels()).mode, "delta"); eq((await mlLabels()).own, false);
    const PL = '[data-testid="d2-pathway-label"]';
    eq(await pg.getAttribute(PL, "aria-description"), await pg.evaluate("I18N.t('draw2.labels.now.delta')"));
    await pg.mouse.click(...at, { button: "right" }); await tpShown();
    const ids = (await tpMenu()).ids;
    assert(["pathway.labelmode", "labelmode.delta", "labelmode.mass", "labelmode.delta+mass", "labelmode.name+delta", "pathway.labeladduct", "labeladduct.none", "labeladduct.[M+H]+"].every(i => ids.includes(i)), "the items: " + ids);
    eq(await pg.getAttribute(mlItem("labelmode.delta"), "aria-checked"), "true");
    eq(await pg.getAttribute(mlItem("pathway.labeladduct"), "aria-disabled"), "true");     // no numbers in the delta mode: no adduct
    eq(await pg.innerText(mlItem("labelmode.delta+mass") + " .n"), await pg.evaluate("I18N.t('draw2.ctx.labelmode.delta+mass')"));
    await pg.click(mlItem("pathway.labelmode")); await pg.click(mlItem("labelmode.mass")); await tpShown(false); await frames(pg);
    eq(await tpSvg(), mlGold[0].out); eq((await mlLabels()).own, true);
    await tool(pg, "undo"); eq(await tpDoc(), before);
    // delta+mass, then the adduct [M+H]+
    await pg.mouse.click(...at, { button: "right" }); await tpShown();
    await pg.click(mlItem("pathway.labelmode")); await pg.click(mlItem("labelmode.delta+mass")); await tpShown(false); await frames(pg);
    await pg.mouse.click(...at, { button: "right" }); await tpShown();
    eq(await pg.getAttribute(mlItem("labelmode.delta+mass"), "aria-checked"), "true");
    await pg.click(mlItem("pathway.labeladduct")); await pg.click(mlItem("labeladduct.[M+H]+")); await tpShown(false); await frames(pg);
    eq(await tpSvg(), mlGold[1].out);
    eq(await pg.getAttribute(PL, "aria-description"), await pg.evaluate("I18N.t('draw2.labels.nowAdduct.delta+mass', { adduct: '[M+H]+' })"));
    await shot(pg, "mass-label-ui-menu");
    // the file keeps the choice: saved, cleared, opened again
    const saved = await tpDoc(); assert(saved.includes('"labels":{"adduct":"[M+H]+","mode":"delta+mass"}'), "the document holds the choice");
    await tool(pg, "undo"); await tool(pg, "undo"); eq(await tpDoc(), before);
    await pg.evaluate(text => /** @type {any} */ (window).MZDRAW.open(text), saved); await frames(pg);
    eq((await mlLabels()).mode, "delta+mass"); eq((await mlLabels()).adduct, "[M+H]+"); eq(await tpSvg(), mlGold[1].out);
  });
  await step("mass labels by keyboard: the Menu key, the group, Enter on the mode; Escape leaves it; the page equals the golden, one undo", async () => {
    const before = await mlOpen();
    await pg.evaluate("window.MZDRAW.pan(0, 500)"); await tool(pg, "select"); await frames(pg);
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Escape"); await pg.keyboard.press("ContextMenu"); await tpShown();
    await pg.focus(mlItem("pathway.labelmode")); await pg.keyboard.press("Enter");
    eq(await pg.getAttribute(mlItem("pathway.labelmode"), "aria-expanded"), "true");
    await pg.focus(mlItem("labelmode.mass")); await pg.keyboard.press("Enter"); await tpShown(false); await frames(pg);
    eq((await mlLabels()).mode, "mass");
    assert((await tpDoc()).includes('"mode":"mass"') && (await tpDoc()).includes('"text":"+15.9949"'), "the keyboard wrote the field and the labels");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
  });
  await step("mass labels by touch: long press the canvas, tap the group, tap the mode and the adduct; the page equals the golden, one undo each", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    const before = await mlOpen(), area = await pg.locator("#d2-canvas").boundingBox(), at = [area.x + 12, area.y + area.height - 12];
    const menu = async () => { await touch("touchStart", [at]); await pg.waitForTimeout(700); await touch("touchEnd", []); await tpShown(); eq((await tpMenu()).kind, "touch"); };
    const tap = (x, y) => pg.touchscreen.tap(x, y);
    await menu(); await tap(...await center(pg, mlItem("pathway.labelmode")));
    const b = await pg.locator(mlItem("labelmode.mass")).boundingBox(); assert(b.height >= 40, "a finger-size item: " + b.height);
    await tap(...await center(pg, mlItem("labelmode.delta+mass"))); await tpShown(false); await frames(pg);
    await menu(); await tap(...await center(pg, mlItem("pathway.labeladduct"))); await tap(...await center(pg, mlItem("labeladduct.[M+H]+"))); await tpShown(false); await frames(pg);
    eq(await tpSvg(), mlGold[1].out);
    await shot(pg, "mass-label-ui-touch");
    await pg.keyboard.press("Control+z"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
  });
  await step("mass labels by pen: the barrel button on the canvas, the group, the mode and the adduct; the page equals the golden, one undo each", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const pen = (type, x, y, barrel = false) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : barrel ? "right" : "left", buttons: type === "mousePressed" ? (barrel ? 2 : 1) : 0, clickCount: 1 });
    const tap = async (x, y) => { await pen("mouseMoved", x, y); await pen("mousePressed", x, y); await pen("mouseReleased", x, y); };
    const before = await mlOpen(), area = await pg.locator("#d2-canvas").boundingBox(), at = [area.x + 12, area.y + area.height - 12];
    const menu = async () => { await pen("mouseMoved", ...at, true); await pen("mousePressed", ...at, true); await tpShown(); eq((await tpMenu()).kind, "pen"); await pen("mouseReleased", ...at, true); };
    await menu(); await tap(...await center(pg, mlItem("pathway.labelmode"))); await tap(...await center(pg, mlItem("labelmode.delta+mass"))); await tpShown(false); await frames(pg);
    await menu(); await tap(...await center(pg, mlItem("pathway.labeladduct"))); await tap(...await center(pg, mlItem("labeladduct.[M+H]+"))); await tpShown(false); await frames(pg);
    eq(await tpSvg(), mlGold[1].out);
    await shot(pg, "mass-label-ui-pen");
    await pg.keyboard.press("Control+z"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
  });
  // the same groups are in the menu of an anchored arrow: the right button, the Menu key (the arrow selected), a long press and the barrel button, on the arrow itself
  const mlArrow = async () => { const d = await pg.evaluate("window.MZDRAW.scene().slice(0, 5)"), b = await pg.locator("#d2-canvas").boundingBox(); return [b.x + d[1] + 3.75 * (d[3] - d[1]), b.y + d[2]]; };
  await step("mass labels on an anchored arrow: its menu has the groups (mouse, keyboard, touch, pen); the page equals the golden after each, one undo each", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    const pen = (type, x, y, barrel = false) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : barrel ? "right" : "left", buttons: type === "mousePressed" ? (barrel ? 2 : 1) : 0, clickCount: 1 });
    const penTap = async (x, y) => { await pen("mouseMoved", x, y); await pen("mousePressed", x, y); await pen("mouseReleased", x, y); };
    const ways = {
      mouse: { menu: async at => { await pg.mouse.click(...at, { button: "right" }); }, tap: (x, y) => pg.mouse.click(x, y) },
      keyboard: { menu: async at => { await pg.mouse.click(...at); await pg.focus("#d2-canvas"); await pg.keyboard.press("ContextMenu"); }, tap: (x, y) => pg.mouse.click(x, y) },
      touch: { menu: async at => { await touch("touchStart", [at]); await pg.waitForTimeout(700); await touch("touchEnd", []); }, tap: (x, y) => pg.touchscreen.tap(x, y) },
      pen: { menu: async at => { await pen("mouseMoved", ...at, true); await pen("mousePressed", ...at, true); await tpShown(); await pen("mouseReleased", ...at, true); }, tap: penTap },
    };
    for (const [way, w] of Object.entries(ways)) {
      const before = await mlOpen(), at = await mlArrow();
      await w.menu(at); await tpShown();
      eq((await tpMenu()).target, "object", way);
      assert((await tpMenu()).ids.includes("labelmode.mass") && (await tpMenu()).ids.includes("labeladduct.[M+H]+"), way + ": the groups are in the menu of the arrow");
      await w.tap(...await center(pg, mlItem("pathway.labelmode"))); await w.tap(...await center(pg, mlItem("labelmode.delta+mass"))); await tpShown(false); await frames(pg);
      await w.menu(await mlArrow()); await tpShown();
      await w.tap(...await center(pg, mlItem("pathway.labeladduct"))); await w.tap(...await center(pg, mlItem("labeladduct.[M+H]+"))); await tpShown(false); await frames(pg);
      eq(await tpSvg(), mlGold[1].out, way);
      if (way === "mouse") await shot(pg, "mass-label-ui-arrow-menu");
      await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+z"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before, way);
    }
  });
  await step("mass labels default: the Preferences hold the mode and the adduct for a scheme with no choice of its own; the scheme's own choice wins; the page equals the golden", async () => {
    await pg.evaluate("window.MZDRAW.set_style('mzlab')"); const before = await mlOpen();
    await pg.focus('[data-testid="d2-prefs"]'); await pg.keyboard.press("Enter"); await until(pg, `!document.getElementById("d2-fly-prefs").hidden`);
    const sel = setting => `#d2-fly-prefs [data-setting="${setting}"]`;
    eq(await pg.locator(sel("pathway.label") + " option").allInnerTexts(), await pg.evaluate("['delta', 'mass', 'delta+mass', 'name+delta'].map(w => I18N.t('draw2.prefs.pathway.label.' + w))"));
    eq(await pg.locator(sel("pathway.adduct") + " option").count(), 8);
    await pg.selectOption(sel("pathway.label"), "delta+mass"); await pg.selectOption(sel("pathway.adduct"), "[M-H]-"); await frames(pg);
    eq(await pg.evaluate("window.MZDRAW.get('pathway.label') + '|' + window.MZDRAW.get('pathway.adduct')"), "delta+mass|[M-H]-");
    eq((await mlLabels()).mode, "delta+mass"); eq((await mlLabels()).own, false);
    await shot(pg, "mass-label-ui-prefs");
    await pg.keyboard.press("Escape"); await until(pg, `document.getElementById("d2-fly-prefs").hidden`);
    // "Label arrows" writes the default for this scheme; the document gets no field (the choice is not the scheme's)
    const PL = '[data-testid="d2-pathway-label"]'; await reveal(pg, PL); await pg.focus(PL); await pg.keyboard.press("Enter"); await frames(pg);
    assert((await tpDoc()).includes("[M-H]-") && !(await tpDoc()).includes('"labels"'), "the default was used, no field written");
    eq(await tpSvg(), mlGold[3].out);
    await tool(pg, "undo"); eq(await tpDoc(), before);
    await pg.evaluate("window.MZDRAW.set('pathway.label=delta'); window.MZDRAW.set('pathway.adduct=none')");
    await tool(pg, "clear");
  });
  // struct-props: the owner chooses which lines stay written under a molecule. A tick of the Check palette goes to the module, which keeps the choice
  // on the molecule in one undo step; the page equals the goldens of tests/data/log/struct-props.json after each way of ticking
  const spGold = JSON.parse(readFileSync(path.join(root, "tests/data/log/struct-props.json"), "utf8")).cases;
  const spPlain = spGold.find(c => c.name.includes("taken back")), spChosen = spGold.find(c => c.name.includes("one tick at a time"));
  const spOpen = async () => { await pg.evaluate(text => /** @type {any} */ (window).MZDRAW.open(text), spPlain.out.mzdraw); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); await want(pg, { atoms: 3 }); await setOpen(pg, "#d2-props", true); return tpDoc(); };
  const spSame = async what => { eq(await tpDoc(), spChosen.out.mzdraw, what + ": document"); eq(await tpSvg(), spChosen.out.svg, what + ": picture"); };
  const spBox = key => `[data-testid="d2-props-${key}"]`;
  /** a tap (touch or pen) on the tick of a line, the row scrolled into view first (the palette scrolls) */
  const spTap = async (tap, key) => { await pg.locator(spBox(key)).scrollIntoViewIfNeeded(); await tap(...await center(pg, spBox(key))); await frames(pg); };
  const spTicked = () => pg.evaluate("[...document.querySelectorAll('#d2-props-lines input')].filter(i => i.checked).map(i => i.dataset.key)");
  await step("struct-props by keyboard: with nothing selected the ticks are off and the words say why; select all (Ctrl+A), Space on Formula and on Hydrogen-bond donors; the page equals the golden, each tick is one undo step, the numbers beside the ticks are the module's", async () => {
    const before = await spOpen();
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-props-lines input')].every(i => i.disabled)"), true);
    eq(await pg.innerText("#d2-props-note"), await pg.evaluate("I18N.t('draw2.props.none')"));
    eq(await pg.locator("#d2-props-lines .txt > span:not(.val)").allInnerTexts(), await pg.evaluate("['formula','mw','exact','ea','mz','logp','tpsa','hbd','hba','rotb','heavy'].map(k => I18N.t('draw2.props.line.' + k))"));
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+a"); await until(pg, `${STATE}.selected > 0`);
    await until(pg, `!document.querySelector('${spBox("formula")}').disabled`);
    eq(await pg.innerText("#d2-props-note"), "");        // the words for "no molecule" are gone when there is one
    eq(await pg.innerText(`${spBox("exact")} + .txt .val`), "Exact mass 46.0419");
    eq(await pg.innerText(`${spBox("ea")} + .txt .val`), "%C 52.14; %H 13.13; %O 34.73");
    // the method is visible text beside each value (not a tooltip only), the words of the catalog
    eq(await pg.locator("#d2-props-lines .how").allInnerTexts(), await pg.evaluate("['formula','mw','exact','ea','mz','logp','tpsa','hbd','hba','rotb','heavy'].map(k => I18N.t('draw2.props.method.' + k))"));
    assert(await pg.isVisible(`${spBox("logp")} + .txt .how`), "the method of logP is visible");
    await pg.focus(spBox("formula")); await pg.keyboard.press("Space"); await frames(pg);
    eq(await spTicked(), ["formula"]); eq(await pg.innerText("#d2-props-note"), await pg.evaluate("I18N.t('draw2.say.lines', { what: 'formula' })"));
    await pg.focus(spBox("hbd")); await pg.keyboard.press("Space"); await frames(pg);
    eq(await spTicked(), ["formula", "hbd"]);
    await spSame("keyboard");
    await shotEl(pg, "#d2-props", "struct-props-panel"); await shot(pg, "struct-props");
    // the values copied as text: the module's own forms, three of them, the choice of the molecule
    await pg.selectOption("#d2-props-form", "tsv"); await pg.click("#d2-props-copy");
    eq(await pg.evaluate("window.MZDRAW.copy_values('tsv')"), "structure\tname\tvalue\tmethod\nC2H6O\tHBD\t1\tmzlab-mol-hbd\n");
    eq(await pg.evaluate("window.MZDRAW.copy_values('line')"), "C2H6O; HBD 1\n");
    eq(await pg.evaluate("window.MZDRAW.copy_values('text')"), "C2H6O\nHBD: 1 [mzlab-mol-hbd]\n");
    eq(await pg.innerText("#d2-props-note"), await pg.evaluate("I18N.t('draw2.props.copied', { form: I18N.t('draw2.props.form.tsv') })"));
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+z"); eq(await spTicked(), ["formula"]); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
    await pg.keyboard.press("Control+y"); await pg.keyboard.press("Control+y"); await spSame("redo");
    // a tick that is taken away gives the line back; the file that has no place for the lines says so
    await pg.focus(spBox("hbd")); await pg.keyboard.press("Space"); await frames(pg); eq(await spTicked(), ["formula"]);
    eq(await pg.evaluate("window.MZDRAW.captions_lost('molfile')"), 1); eq(await pg.evaluate("window.MZDRAW.captions_lost('cdxml')"), 0);
    await saveAs(pg, "#d2-ex-molfile"); await until(pg, `document.getElementById("d2-ex-note").textContent.includes("lines")`);
    eq(await pg.innerText("#d2-ex-note"), await pg.evaluate("I18N.t('draw2.export.captionsLost', { n: 1 })"));
    await pg.keyboard.press("Escape"); await pg.focus("#d2-canvas");        // the Save as list is left open by a note: close it
  });
  await step("struct-props by touch: select all, tap Formula and Hydrogen-bond donors (a finger-size row); the page equals the golden, one undo each", async () => {
    const before = await spOpen();
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+a"); await until(pg, `${STATE}.selected > 0`);
    await until(pg, `!document.querySelector('${spBox("formula")}').disabled`);
    const row = await pg.locator(`${spBox("formula")}`).locator("xpath=..").boundingBox(); assert(row.height >= 40, "a finger-size row: " + row.height);
    const fingerTap = (x, y) => pg.touchscreen.tap(x, y);
    await spTap(fingerTap, "formula"); await spTap(fingerTap, "hbd");
    await spSame("touch"); await shot(pg, "struct-props-touch");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+z"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
  });
  await step("struct-props by pen: a pen tap on Formula and on Hydrogen-bond donors after Select all; the page equals the golden, a refused tick (no molecule) changes nothing", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const pen = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1 });
    const tap = async (x, y) => { await pen("mouseMoved", x, y); await pen("mousePressed", x, y); await pen("mouseReleased", x, y); };
    const before = await spOpen();
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+a"); await until(pg, `${STATE}.selected > 0`);
    await until(pg, `!document.querySelector('${spBox("formula")}').disabled`);
    await spTap(tap, "formula"); await spTap(tap, "hbd");
    await spSame("pen");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+z"); await pg.keyboard.press("Control+z"); eq(await tpDoc(), before);
    // nothing selected, nothing focused: the ticks are off, and a real pen tap on one changes nothing
    const area = await pg.locator("#d2-canvas").boundingBox(); await pg.mouse.click(area.x + 12, area.y + area.height - 12); await frames(pg); await pg.keyboard.press("Escape");
    await until(pg, `document.querySelector('${spBox("formula")}').disabled`);
    await spTap(tap, "formula");
    eq(await tpDoc(), before); eq(await spTicked(), []);
  });
  await ctx.close();

  // text-panel: a text is selected, edited and styled by part (the panel of the page forwards to the module: `text_style`, `text_edit`, `text_input`)
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, hasTouch: true }));
  const txDoc = '{"format":"mzdraw/1","next":3,"atoms":[],"bonds":[],"objects":[{"id":2,"t":"text","x":0.0,"y":0.0,"text":"Hello world"}]}';
  const txSave = () => pg.evaluate("window.MZDRAW.save('mzdraw')");
  const txState = async () => JSON.parse(await pg.evaluate("window.MZDRAW.state()"));
  const txText = async () => (await txState()).text;
  const txRuns = async () => (JSON.parse(await txSave()).objects.find(o => o.id === 2) || {}).runs || [];
  /** the pixel of the start of the first line of the text, read from the display list (the canvas as a user sees it) */
  const txOrigin = async () => {
    const d = await pg.evaluate("Array.from(window.MZDRAW.scene())"), b = await pg.locator("#d2-canvas").boundingBox();
    for (let i = 0; i < d.length;) { const op = d[i]; if (op === 2 && d[i + 6] === 1) return [b.x + d[i + 1], b.y + d[i + 2]]; i += op === 1 ? 7 : op === 2 ? 7 : op === 3 ? 6 : op === 6 ? 8 : op === 7 ? 13 : 5; }
    throw new Error("no text in the display list");
  };
  const txOpen = async () => { await pg.evaluate(text => /** @type {any} */ (window).MZDRAW.open(text), txDoc); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); await pg.mouse.click(5, 5); await frames(pg); return txOrigin(); };
  const txShown = async open => { await until(pg, `document.getElementById("d2-textbar").hidden === ${!open}`); await frames(pg); };
  await step("text panel by keyboard: a click selects the text and the panel appears; Enter edits; Shift+arrows select a range; Ctrl+B styles only the range; the shortcuts; typing is one undo step; the panel reads the style from the module; Escape stops", async () => {
    const [ox, oy] = await txOpen(); await txShown(false);
    await pg.mouse.click(ox + 6, oy); await txShown(true);
    eq(await pg.innerText("#d2-text-edit"), "Edit text");
    eq(await pg.getAttribute('#d2-textbar [data-text="b"]', "aria-pressed"), "false");
    const plain = await txSave();
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Enter"); await until(pg, "window.MZDRAW.state().includes('\"editing\":true')");
    eq(await pg.innerText("#d2-text-edit"), "Done");
    eq(await pg.evaluate("document.activeElement.id"), "d2-text-input");
    await pg.keyboard.press("Home"); for (let i = 0; i < 5; i++) await pg.keyboard.press("Shift+ArrowRight");
    let x = await txText(); eq([x.from, x.to, x.len], [0, 5, 11]);
    eq(await pg.innerText("#d2-text-state"), "Text: 5 of 11 characters selected.");
    await shot(pg, "text-panel-range"); await shotEl(pg, "#d2-textbar", "text-panel-bar");
    await pg.keyboard.press("Control+b"); await frames(pg);
    eq((await txRuns()).map(r => [r.t, !!r.b]), [["Hello", true], [" world", false]]);
    eq(await pg.getAttribute('#d2-textbar [data-text="b"]', "aria-pressed"), "true");
    await pg.keyboard.press("Control+Shift+Z".replace("Shift+", "")); await frames(pg);       // Ctrl+Z (the undo keeps its binding while a text is edited)
    eq(await txSave(), plain);
    await pg.keyboard.press("Control+y"); await frames(pg);
    // the mixed range shows "mixed": select all, bold is on some of it
    await pg.keyboard.press("Control+a"); await frames(pg);
    eq(await pg.getAttribute('#d2-textbar [data-text="b"]', "aria-pressed"), "mixed");
    await pg.keyboard.press("Control+b"); await frames(pg); eq((await txRuns()).every(r => r.b), true);
    await pg.keyboard.press("Control+b"); await frames(pg); eq(await txRuns(), []);
    // Ctrl+, and Ctrl+. (the sub and the superscript), Ctrl+I, Ctrl+U on the first character
    await pg.keyboard.press("Home"); await pg.keyboard.press("Shift+ArrowRight");
    for (const k of ["Control+i", "Control+u", "Control+Comma"]) await pg.keyboard.press(k);
    let r = (await txRuns())[0]; eq([r.t, !!r.i, !!r.u, !!r.sub], ["H", true, true, true]);
    await pg.keyboard.press("Control+Period"); r = (await txRuns())[0]; eq([!!r.sub, !!r.sup], [false, true]);
    // typing at the end takes the style before the caret, joins one undo step, and a new line is a break
    await pg.keyboard.press("End"); const styled = await txSave();
    await pg.keyboard.type("!?"); await pg.keyboard.press("Enter"); await pg.keyboard.type("ok"); await frames(pg);
    eq(JSON.parse(await txSave()).objects[0].text, "Hello world!?\nok");
    await pg.keyboard.press("Control+z"); await frames(pg); eq(await txSave(), styled);
    await shot(pg, "text-panel-keyboard");
    await pg.keyboard.press("Escape"); await frames(pg);
    eq((await txText()).editing, false); eq(await pg.innerText("#d2-text-edit"), "Edit text");
    // the text is still selected: Escape again clears the selection and the panel goes
    await pg.keyboard.press("Escape"); await txShown(false);
    // the replay of the log of the session is the document of the page (rule 15)
    const log = await pg.evaluate("window.MZDRAW.save('log')");
    assert(log.includes('"obj.set"'), "the log holds the text ops");
    const again = await pg.evaluate(([l, d]) => /** @type {any} */ (window).MZDRAW.replay(JSON.stringify({ import: d, log: JSON.parse(l), out: "mzdraw" })), [log, txDoc]);
    eq(again, await txSave());
  });
  await step("text panel by mouse: a drag inside the text selects a range, the buttons, the font, the size and the colour change only that range; a double click selects a word; the font and size lists come from the module", async () => {
    const [ox, oy] = await txOpen();
    await pg.mouse.click(ox + 6, oy); await txShown(true);
    await pg.click("#d2-text-edit"); await until(pg, "window.MZDRAW.state().includes('\"editing\":true')");
    await pg.mouse.move(ox + 1, oy); await pg.mouse.down(); await pg.mouse.move(ox + 70, oy, { steps: 5 }); await pg.mouse.up(); await frames(pg);
    let x = await txText(); eq(x.from, 0); assert(x.to >= 3 && x.to < 11, "a range by a drag: " + JSON.stringify([x.from, x.to]));
    const to = x.to;
    await pg.click('#d2-textbar [data-text="i"]'); await frames(pg);
    eq((await txRuns()).map(r => !!r.i), [true, false]); eq((await txRuns())[0].t.length, to);
    // the lists are the module's
    eq(await pg.locator("#d2-text-font option").allInnerTexts(), ["Default font", "Arial", "Helvetica", "Times New Roman", "Courier New", "Georgia", "Verdana"]);
    eq(await pg.locator("#d2-text-size option").allInnerTexts(), ["50 %", "75 %", "100 %", "125 %", "150 %", "200 %", "300 %", "400 %"]);
    await pg.selectOption("#d2-text-font", "courier"); await pg.selectOption("#d2-text-size", "150"); await frames(pg);
    let r = (await txRuns())[0]; eq([r.font, r.sz], ["Courier New", 1.5]);
    eq(await pg.inputValue("#d2-text-font"), "courier"); eq(await pg.inputValue("#d2-text-size"), "150");
    // the colour: the palette of the module, each with its name as text
    await pg.click("#d2-text-colour"); eq(await pg.getAttribute("#d2-text-colour", "aria-expanded"), "true");
    eq((await pg.locator("#d2-text-colours span:not(.sw)").allInnerTexts()).slice(0, 3), ["Automatic", "Black", "Red"]);
    await shot(pg, "text-panel-colour");
    await pg.click('[data-testid="d2-text-colour-red"]'); await frames(pg);
    eq((await txRuns())[0].color, "#d00000"); eq((await txRuns())[1].color, undefined);
    eq(await pg.getAttribute("#d2-text-colours", "hidden"), "");
    await shot(pg, "text-panel-mouse");
    // a double click selects the word under it; one undo step per button
    await pg.mouse.dblclick(ox + 3, oy); await frames(pg);
    x = await txText(); assert(x.from === 0 && x.to === 5, "the word: " + JSON.stringify([x.from, x.to]));
    for (let i = 0; i < 4; i++) await pg.keyboard.press("Control+z");
    eq(await txRuns(), []);
  });
  await step("text panel by touch: a tap selects, the Edit button edits, a drag with a finger selects a range, a tap on Bold styles only it; the controls are finger-size", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    const [ox, oy] = await txOpen();
    await pg.touchscreen.tap(ox + 6, oy); await txShown(true);
    for (const sel of ["#d2-text-edit", '#d2-textbar [data-text="b"]', "#d2-text-colour", "#d2-text-font"]) { const b = await pg.locator(sel).boundingBox(); assert(b.height >= 36 && b.width >= 36, `${sel} is finger-size: ${b.width}x${b.height}`); }
    await pg.touchscreen.tap(...await center(pg, "#d2-text-edit")); await until(pg, "window.MZDRAW.state().includes('\"editing\":true')");
    await touch("touchStart", [[ox + 1, oy]]); for (let k = 1; k <= 6; k++) await touch("touchMove", [[ox + 1 + k * 12, oy]]); await touch("touchEnd", []); await frames(pg);
    const x = await txText(); eq(x.from, 0); assert(x.to >= 3 && x.to < 11, "a range by a finger: " + JSON.stringify([x.from, x.to]));
    // (after a drag the emulated touch needs one tap on the bar's own edge before the next tap is a click: a quirk of the emulation, no page code)
    const bar = await pg.locator("#d2-textbar").boundingBox(); await pg.touchscreen.tap(bar.x + 2, bar.y + 2);
    await pg.touchscreen.tap(...await center(pg, '#d2-textbar [data-text="b"]')); await frames(pg);
    const runs = await txRuns(); eq([!!runs[0].b, runs[1] ? !!runs[1].b : null], [true, false]);
    await shot(pg, "text-panel-touch");
    await pg.touchscreen.tap(...await center(pg, "#d2-text-edit")); await frames(pg);
    eq((await txText()).editing, false);
    await pg.keyboard.press("Control+z"); await frames(pg); eq(await txRuns(), []);
  });
  await step("text panel, a cancelled press: a finger press taken away by the system (pointercancel) ends the drag of the range, and a later move over the text does not grow it", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    const [ox, oy] = await txOpen();
    await pg.mouse.click(ox + 6, oy); await txShown(true);
    await pg.click("#d2-text-edit"); await until(pg, "window.MZDRAW.state().includes('\"editing\":true')");
    await touch("touchStart", [[ox + 1, oy]]); await touch("touchMove", [[ox + 20, oy]]);
    await touch("touchCancel", []); await frames(pg);
    const x = await txText(), at = [x.from, x.to];
    await pg.mouse.move(ox + 90, oy, { steps: 4 }); await frames(pg);
    eq([(await txText()).from, (await txText()).to], at);
  });
  await step("text panel by pen: a pen drag selects a range inside the edited text and a pen tap on Subscript sets only it", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const pen = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" || type === "mouseMoved" ? 1 : 0, clickCount: 1 });
    const tap = async (x, y) => { await pen("mouseMoved", x, y); await pen("mousePressed", x, y); await pen("mouseReleased", x, y); };
    const [ox, oy] = await txOpen();
    await tap(ox + 6, oy); await txShown(true);
    await tap(...await center(pg, "#d2-text-edit")); await until(pg, "window.MZDRAW.state().includes('\"editing\":true')");
    await pen("mouseMoved", ox + 1, oy); await pen("mousePressed", ox + 1, oy); for (let k = 1; k <= 5; k++) await pen("mouseMoved", ox + 1 + k * 10, oy); await pen("mouseReleased", ox + 51, oy); await frames(pg);
    const x = await txText(); eq(x.from, 0); assert(x.to >= 2 && x.to < 11, "a range by a pen: " + JSON.stringify([x.from, x.to]));
    await tap(...await center(pg, '#d2-textbar [data-text="sub"]')); await frames(pg);
    const runs = await txRuns(); eq([!!runs[0].sub, runs[1] ? !!runs[1].sub : null], [true, false]);
    await shot(pg, "text-panel-pen");
    await pg.keyboard.press("Control+z"); await frames(pg); eq(await txRuns(), []);
  });
  await step("text preferences: the Text group of the Preferences gives a new text its font and size (a setting of the settings file, replayed by the CLI as `text.font`); a bad value is refused", async () => {
    await pg.evaluate(() => { const m = /** @type {any} */ (window).MZDRAW; m.open('{"format":"mzdraw/1","next":1,"atoms":[],"bonds":[],"objects":[]}'); });
    await tool(pg, "fit"); await tool(pg, "select");
    await pg.focus('[data-testid="d2-prefs"]'); await pg.keyboard.press("Enter"); await until(pg, `!document.getElementById("d2-fly-prefs").hidden`);
    eq(await pg.evaluate("document.querySelector('#d2-fly-prefs fieldset[data-group=\"text\"] > legend').textContent"), "Text");
    eq(await pg.locator('#d2-fly-prefs [data-setting="text.size"] option').allInnerTexts(), ["50 %", "75 %", "100 %", "125 %", "150 %", "200 %", "300 %", "400 %"]);
    await pg.selectOption('#d2-fly-prefs [data-setting="text.font"]', "times"); await pg.selectOption('#d2-fly-prefs [data-setting="text.size"]', "150"); await frames(pg);
    eq(await pg.evaluate("JSON.parse(window.MZDRAW.style()).text"), { font: "times", size: "150" });
    await shot(pg, "text-prefs");
    await pg.keyboard.press("Escape"); await until(pg, `document.getElementById("d2-fly-prefs").hidden`);
    await pg.focus("#d2-canvas"); await pg.keyboard.press("t"); await pg.keyboard.type("CO_2"); await pg.keyboard.press("Enter");
    await pg.mouse.click(cx, cy); await want(pg, { objects: 1 });
    const o = JSON.parse(await txSave()).objects[0];
    eq(o.runs.map(r => [r.t, r.font, r.sz]), [["CO", "Times New Roman", 1.5], ["2", "Times New Roman", 1.5]]);
    // the defaults put back by the reset of the group leave a new text plain
    await pg.focus('[data-testid="d2-prefs"]'); await pg.keyboard.press("Enter"); await until(pg, `!document.getElementById("d2-fly-prefs").hidden`);
    await pg.click('#d2-prefs-text [data-reset="text"]'); await frames(pg);
    eq(await pg.evaluate("JSON.parse(window.MZDRAW.style()).text"), undefined);
    await pg.keyboard.press("Escape");
    eq(await pg.evaluate(() => { try { /** @type {any} */ (window).MZDRAW.set("text.font=comic"); return "no error"; } catch (e) { return String(e).includes("textPref") ? "refused" : String(e); } }), "refused");
  });
  await ctx.close();

  // prefs: the Preferences flyout of the File palette (the gear), built from the module's schema; every option is a setting, so the page equals the goldens
  // of tests/data/log/prefs.json after the keyboard, the touch and the pen way, and the settings are kept and saved
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, hasTouch: true }));
  const pfGold = JSON.parse(readFileSync(path.join(root, "tests/data/log/prefs.json"), "utf8")).cases;
  const pfSvg = () => pg.evaluate("window.MZDRAW.save('svg')");
  /** the picture equals the golden, or says where it first differs */
  const pfSame = async (gold, what) => { const s = await pfSvg(); let i = 0; while (i < s.length && s[i] === gold[i]) i++; assert(s === gold, `${what}: differs at ${i}: ${JSON.stringify(s.slice(Math.max(0, i - 60), i + 80))} vs ${JSON.stringify(gold.slice(Math.max(0, i - 60), i + 80))}`); };
  const pfOpenDoc = async () => { await pg.evaluate(text => { const m = /** @type {any} */ (window).MZDRAW; m.open(m.replay(JSON.stringify({ import: text, out: "mzdraw" }))); }, pfGold[0].request.import); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); };
  const pfShown = async open => { await until(pg, `document.getElementById("d2-fly-prefs").hidden === ${!open}`); await frames(pg); };
  const pfSel = setting => `#d2-fly-prefs [data-setting="${setting}"]`;
  /** where the centre of a control of the panel is, after the panel has scrolled it into view (a finger or a pen scrolls it by hand) */
  const pfAt = async sel => { await pg.locator(sel).scrollIntoViewIfNeeded(); return center(pg, sel); };
  const pfAria = async () => pg.evaluate("document.querySelector('[data-testid=\"d2-prefs\"]').getAttribute('aria-label')");
  await step("preferences by keyboard: the gear in the File palette opens the panel without blocking the canvas, atom colours toggle to the golden pictures, Escape closes and gives the focus back; the panel is built from the schema", async () => {
    await pfOpenDoc();
    await pfSame(pfGold[0].out.svg, "the default colours");
    eq(await pfAria(), await pg.evaluate("I18N.t('draw2.tool.prefs.title')"));
    assert((await pg.locator('[data-testid="d2-prefs"] svg[data-icon="gear"]').count()) === 1, "a gear icon on a real button");
    await pg.focus('[data-testid="d2-prefs"]'); await pg.keyboard.press("Enter"); await pfShown(true);
    // the groups and the rows are the schema's, in its order, with its texts
    const schema = JSON.parse(await pg.evaluate("window.MZDRAW.settings_schema()"));
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-fly-prefs fieldset[data-group] > legend')].map(l => l.textContent)"), await pg.evaluate(g => g.map(x => I18N.t(x.label)), schema.groups));
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-fly-prefs [data-setting]')].map(e => e.dataset.setting).filter(Boolean)"), schema.prefs.map(p => p.setting));
    eq(await pg.inputValue(pfSel("style.colour")), "element");
    await pg.focus(pfSel("style.colour")); await pg.keyboard.press("ArrowDown"); await frames(pg);
    eq(await pg.inputValue(pfSel("style.colour")), "mono");
    await pfSame(pfGold[1].out.svg, "one colour: the golden picture");
    assert(await pg.evaluate("(() => JSON.parse(window.MZDRAW.style()).style.colour === 'mono')()"), "the setting is in the settings file");
    eq(await pg.inputValue("#d2-style-colour"), "mono", "the style panel shows the same");
    await shot(pg, "prefs-keyboard");
    // the canvas still takes the keyboard: the panel is no modal dialog
    eq(await pg.evaluate("document.getElementById('d2-fly-prefs').getAttribute('aria-modal')"), null);
    await pg.keyboard.press("Escape"); await pfShown(false);
    eq(await pg.evaluate("document.activeElement.dataset.testid"), "d2-prefs");
    // a number of the style, kept and refused in words
    await pg.keyboard.press("Enter"); await pfShown(true);
    await pg.fill(pfSel("style.line_width"), "999"); await pg.press(pfSel("style.line_width"), "Enter"); await until(pg, `document.querySelector('[data-setting="style.line_width"]').getAttribute("aria-invalid") === "true"`);
    eq(await pg.innerText("#d2-prefs-note"), await pg.evaluate("I18N.t('error.draw2.styleValue')"));
    await pg.fill(pfSel("style.line_width"), "2"); await pg.press(pfSel("style.line_width"), "Enter"); await until(pg, "JSON.parse(window.MZDRAW.style()).style.line_width === 2");
    eq(await pg.inputValue("#d2-style-line_width"), "2");
    // the group Display is reset to its defaults: the first golden again, and the line width of the group Style is left alone
    await pg.click('#d2-fly-prefs [data-reset="display"]'); await frames(pg);
    assert((await pfSvg()).includes('stroke-width="2"'), "the other groups are left alone");
    eq(await pg.inputValue(pfSel("style.colour")), "element");
    await pg.click('#d2-fly-prefs [data-reset="style"]'); await frames(pg);
    await pfSame(pfGold[0].out.svg, "all groups reset: the first golden again");
    await pg.keyboard.press("Escape"); await pfShown(false);
  });
  await step("preferences: the theme, the default download and the adducts are settings too; they are kept in the browser, saved in the settings file and come back on the next visit", async () => {
    await pg.click('[data-testid="d2-prefs"]'); await pfShown(true);
    await pg.selectOption(pfSel("ui.theme"), "dark"); await until(pg, "document.documentElement.dataset.theme === 'dark'");
    await pg.selectOption(pfSel("export.default"), "png"); await pg.selectOption(pfSel("export.background"), "none");
    eq(JSON.parse(await pg.evaluate("window.MZDRAW.export_settings()")).default, "png");
    await pg.selectOption(pfSel("display.values"), "formula");
    await pg.locator('#d2-fly-prefs fieldset[data-setting="display.adducts"] input[value="[M+Na]+"]').check(); await frames(pg);
    eq(JSON.parse(await pg.evaluate("window.MZDRAW.style()")).style["display.adducts"], "[M+H]+,[M+Na]+,[M-H]-");
    await shot(pg, "prefs-theme");
    const file = JSON.parse(await pg.evaluate("window.MZDRAW.style()"));
    eq(file.ui, { theme: "dark" }); eq(file.export, { background: "none", default: "png" });
    // a file of a newer version keeps its unknown keys of `ui` and the theme goes with the file
    await pg.evaluate("window.MZDRAW.set_style(JSON.stringify({ format: 'mzdraw-settings/1', style: {}, ui: { theme: 'light', density: 'tight' } }))");
    await pg.evaluate("window.MZDRAW.set('ui.theme=light')");
    eq(JSON.parse(await pg.evaluate("window.MZDRAW.style()")).ui, { density: "tight", theme: "light" });
    await pg.selectOption(pfSel("ui.theme"), "dark");
    const kept = await pg.evaluate("Number(document.documentElement.dataset.d2Style || 0)");
    await until(pg, `Number(document.documentElement.dataset.d2Style || 0) > ${kept}`);
    await pg.reload(); await until(pg, "document.documentElement.dataset.d2Ready === '1'"); await frames(pg);
    eq(await pg.evaluate("document.documentElement.dataset.theme"), "dark", "the theme is kept");
    eq(await pg.evaluate("window.MZDRAW.get('ui.theme')"), "dark");
    await pg.click('[data-testid="d2-prefs"]'); await pfShown(true);
    eq(await pg.inputValue(pfSel("ui.theme")), "dark"); eq(await pg.inputValue(pfSel("export.default")), "svg");     // the settings opened again are those of the file the module wrote last
    await pg.click('#d2-fly-prefs [data-reset="appearance"]'); await until(pg, "document.documentElement.dataset.theme === undefined");
    await pg.keyboard.press("Escape");
  });
  await step("preferences by touch: tap the gear, switch the colours and reset the group Display with a finger; the page equals the goldens", async () => {
    await pg.evaluate("window.MZDRAW.set_style('mzlab')"); await pfOpenDoc();
    await pfSame(pfGold[0].out.svg, "picture");
    await pg.touchscreen.tap(...await center(pg, '[data-testid="d2-prefs"]')); await pfShown(true);
    await pg.selectOption(pfSel("style.colour"), "mono"); await frames(pg);
    await pfSame(pfGold[1].out.svg, "one colour");
    await shot(pg, "prefs-touch");
    await pg.touchscreen.tap(...await pfAt('#d2-fly-prefs [data-reset="display"]')); await frames(pg);
    await pfSame(pfGold[0].out.svg, "reset by a tap");
    await pg.touchscreen.tap(...await pfAt("#d2-prefs-close")); await pfShown(false);
    eq(await pg.evaluate("document.activeElement.dataset.testid"), "d2-prefs", "Close gives the focus back to the gear");
  });
  await step("preferences by pen: a pen tap on the gear, on the colours and on the reset of the group Display; the page equals the goldens", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const pen = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1 });
    const tap = async (x, y) => { await pen("mouseMoved", x, y); await pen("mousePressed", x, y); await pen("mouseReleased", x, y); };
    await pg.evaluate("window.MZDRAW.set_style('mzlab')"); await pfOpenDoc();
    await tap(...await center(pg, '[data-testid="d2-prefs"]')); await pfShown(true);
    await pg.selectOption(pfSel("style.colour"), "mono"); await frames(pg);
    await pfSame(pfGold[1].out.svg, "one colour");
    await tap(...await pfAt('#d2-fly-prefs [data-reset="display"]')); await frames(pg);
    await pfSame(pfGold[0].out.svg, "reset by a pen tap");
    await shot(pg, "prefs-pen");
    await tap(...await pfAt("#d2-prefs-close")); await pfShown(false);
    eq(await pg.evaluate("document.activeElement.dataset.testid"), "d2-prefs", "Close gives the focus back to the gear");
  });
  await ctx.close();

  await step("preferences: a stored dark theme is on the page before the first paint and before the module loads; a corrupt stored theme drops only itself with a note; without storage the page still starts", async () => {
    // the stored word is on the root when the body appears, before the module is ready
    let c = await context(browser, { viewport: { width: 1280, height: 800 }, colorScheme: "light" }, () => {
      localStorage.setItem("d2-theme", "dark");
      new MutationObserver((_, o) => { if (document.body) { /** @type {any} */ (window).__atBody = [document.documentElement.dataset.theme || "", document.documentElement.dataset.d2Ready || ""]; o.disconnect(); } }).observe(document, { childList: true, subtree: true });
    });
    eq(await c.pg.evaluate("window.__atBody"), ["dark", ""], "the first paint has the stored theme, the module is not ready yet");
    eq(await c.pg.evaluate("document.documentElement.dataset.theme"), undefined, "the module's settings say auto: the stored word is taken off once it is ready");
    await c.ctx.close();
    // a corrupt stored theme: the file's style and layout are kept, the note says what was dropped
    c = await context(browser, { viewport: { width: 1280, height: 800 } });
    await c.pg.evaluate(async name => { const w = await (await (await navigator.storage.getDirectory()).getFileHandle(name, { create: true })).createWritable(); await w.write(JSON.stringify({ format: "mzdraw-settings/1", style: { colour: "mono" }, layout: { preset: "tablet" }, ui: { theme: "sepia" } })); await w.close(); }, "mzdraw-settings.json");
    await c.pg.reload(); await until(c.pg, "document.documentElement.dataset.d2Ready === '1'");
    eq(JSON.parse(await c.pg.evaluate("window.MZDRAW.style()")).style.colour, "mono");
    eq(await c.pg.evaluate("window.MZDRAW.get('ui.theme')"), "auto");
    eq(await c.pg.innerText("#d2-style-note"), await c.pg.evaluate("I18N.t('error.draw2.ui')"));
    eq(await c.pg.evaluate("document.documentElement.dataset.theme"), undefined);
    await c.pg.evaluate(() => localStorage.setItem("d2-theme", "dark")); await c.pg.reload(); await until(c.pg, "document.documentElement.dataset.d2Ready === '1'");
    await c.pg.evaluate(async name => { const w = await (await (await navigator.storage.getDirectory()).getFileHandle(name, { create: true })).createWritable(); await w.write(JSON.stringify({ format: "mzdraw-settings/1", style: {} })); await w.close(); }, "mzdraw-settings.json");
    await c.pg.evaluate(() => localStorage.setItem("d2-theme", "dark")); await c.pg.reload(); await until(c.pg, "document.documentElement.dataset.d2Ready === '1'");
    eq(await c.pg.evaluate("document.documentElement.dataset.theme"), undefined, "a stored dark theme the module's settings do not have is taken off");
    await c.ctx.close();
    // storage that throws: the page starts, in the theme of the system
    c = await context(browser, { viewport: { width: 1280, height: 800 } }, () => {
      const boom = () => { throw new Error("no storage"); };
      Object.defineProperty(window, "localStorage", { get: boom });
      Object.defineProperty(navigator, "storage", { get: () => ({ getDirectory: boom }) });
    });
    eq(await c.pg.evaluate("document.documentElement.dataset.d2Ready"), "1");
    await c.pg.click('[data-testid="d2-prefs"]'); await c.pg.selectOption('#d2-fly-prefs [data-setting="ui.theme"]', "dark");
    await until(c.pg, "document.documentElement.dataset.theme === 'dark'");
    await c.ctx.close();
  });

  // tp-04: the diff lens (display.lens: off, tint, tint+ghost) on the products of anchored arrows, from the keyboard (Shift+L), the
  // Style panel, the canvas menu by touch and by pen; the page equals the goldens of tests/data/log/tp-lens.json after each way
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, hasTouch: true }));
  const lensGold = JSON.parse(readFileSync(path.join(root, "tests/data/log/tp-lens.json"), "utf8")).cases.slice(0, 3);
  const lensSvg = () => pg.evaluate("window.MZDRAW.save('svg')"), lensDoc = () => pg.evaluate("window.MZDRAW.save('mzdraw')");
  const lensState = () => pg.evaluate("JSON.parse(window.MZDRAW.state())");
  const lensOpen = async () => { await pg.evaluate(text => /** @type {any} */ (window).MZDRAW.open(text), lensGold[0].out.mzdraw); await tool(pg, "fit"); await tool(pg, "select"); await frames(pg); await want(pg, { atoms: JSON.parse(lensGold[0].out.mzdraw).atoms.length, objects: 3 }); return lensDoc(); };
  /** the colours the display list paints: lines of the accent token (11) and of the muted ink (12) */
  const lensInk = () => pg.evaluate(() => { const d = window.MZDRAW.scene(); let accent = 0, muted = 0; for (let i = 0; i < d.length;) { const op = d[i]; if (op === 1) { if (d[i + 6] === 11) accent++; if (d[i + 6] === 12) muted++; } i += op === 1 || op === 2 ? 7 : op === 3 ? 6 : op === 6 ? 8 : 5; } return { accent, muted }; });
  const lensAt = async (mode, what) => {
    const i = ["off", "tint", "tint+ghost"].indexOf(mode);
    eq(await lensSvg(), lensGold[i].out.svg, what + ": picture " + mode);
    eq((await lensState()).lens, mode, what + ": state");
    const ink = await lensInk();
    eq([ink.accent > 0, ink.muted > 0], [mode !== "off", mode === "tint+ghost"], what + ": colours of the display list " + JSON.stringify(ink));
  };
  await step("diff lens from the keyboard: Shift+L goes off, tint, tint+ghost, off; the page equals the golden of each mode, the document and the undo history are never touched", async () => {
    const before = await lensOpen(); await lensAt("off", "start"); await shot(pg, "tp04-off");
    eq((await lensState()).undo, false);
    await pg.focus("#d2-canvas");
    await pg.keyboard.press("Shift+L"); await frames(pg); await lensAt("tint", "key"); await shot(pg, "tp04-tint");
    await pg.keyboard.press("Shift+L"); await frames(pg); await lensAt("tint+ghost", "key"); await shot(pg, "tp04-ghost");
    assert((await pg.innerText("#d2-live")).includes(await pg.evaluate("I18N.t('draw2.say.lens', { what: 'tint+ghost' })")), "the live line says the mode");
    eq(await lensDoc(), before, "the document is the same");
    eq((await lensState()).undo, false, "no undo step");
    await pg.keyboard.press("Shift+L"); await frames(pg); await lensAt("off", "key again");
    // the plain key still asks for a label
    await pg.keyboard.press("l"); await frames(pg); await pg.keyboard.press("Escape");
    eq((await lensState()).lens, "off");
  });
  await step("diff lens in words: with the lens on, each product has a line in the outline, from its parent's formula to the delta; the Style panel sets the mode and follows the key", async () => {
    await lensOpen();
    eq((await lensState()).outline.every(o => o.lens === null), true, "no lens words while the lens is off");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Shift+L"); await frames(pg);
    const st = await lensState();
    eq(st.outline.map(o => o.lens === null), [true, false, false, false, false, false].slice(0, st.outline.length).map((x, i) => i === 0 ? true : x));
    const lines = await pg.locator("#d2-outline li").allInnerTexts();
    for (const [i, o] of st.outline.entries()) if (o.lens) {
      const text = await pg.evaluate(([k, v]) => I18N.t(k, v), [o.lens.delta ? "draw2.outline.lens" : "draw2.outline.lens.same", o.lens]);
      assert(lines[i].includes(text), `outline line ${i}: ${lines[i]} should hold ${text}`);
    }
    assert(lines.some(l => l.includes("+O")) && lines.some(l => l.includes("-CH2")) && lines.some(l => l.includes("1 bond cleaved")), lines.join(" | "));
    // the Style panel: a choice among the words of the module; the key moves it too
    const sel = pg.locator('[id="d2-style-display.lens"]');
    await setOpen(pg, "#d2-style-more", true);
    eq(await sel.inputValue(), "tint");
    eq(await pg.innerText("#d2-lens-note"), "", "nothing is skipped in a fully mapped drawing");
    await sel.selectOption("tint+ghost"); await frames(pg); await lensAt("tint+ghost", "panel");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Shift+L"); await frames(pg); await lensAt("off", "key after the panel");
    eq(await sel.inputValue(), "off", "the panel follows the key");
    // a drawing without maps: the lens is skipped for each arrow and the page says so in words, never nothing
    const plain = JSON.parse(lensGold[0].out.mzdraw); for (const x of plain.atoms) delete x.map;
    await pg.evaluate(text => /** @type {any} */ (window).MZDRAW.open(text), JSON.stringify(plain)); await frames(pg);
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Shift+L"); await frames(pg);
    eq((await lensState()).lens, "tint");
    eq(await pg.innerText("#d2-lens-note"), await pg.evaluate("I18N.t('draw2.notice.lensSkipped', { n: 3 })"));
    eq((await lensInk()).accent, 0, "nothing is tinted without maps");
  });
  await step("diff lens by touch (long press on the canvas, tap on the item) and by pen (barrel button, then the item): the page equals the golden of each mode", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const touch = (type, points) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: points.map(([x, y]) => ({ id: 1, x, y })) });
    await lensOpen(); await pg.evaluate("window.MZDRAW.set('display.lens=off')");
    // a point of the canvas with nothing of the drawing and none of the palettes near it
    const spot = await pg.evaluate(() => {
      const d = window.MZDRAW.scene(), c = document.getElementById("d2-canvas"), r = c.getBoundingClientRect(), pts = [];
      for (let i = 0; i < d.length;) { const op = d[i]; if (op === 1) pts.push([d[i + 1], d[i + 2]], [d[i + 3], d[i + 4]]); else if (op === 2 || op === 4 || op === 5) pts.push([d[i + 1], d[i + 2]]); i += op === 1 || op === 2 ? 7 : op === 3 ? 6 : op === 6 ? 8 : 5; }
      for (let y = r.height - 30; y > 30; y -= 20) for (let x = r.width / 2; x > 30 && x < r.width - 30; x += 20 * (x > r.width / 2 ? 1 : -1) * 1) {
        const e = document.elementFromPoint(r.x + x, r.y + y);
        if (e === c && pts.every(([px, py]) => Math.hypot(px - x, py - y) > 60)) return [r.x + x, r.y + y];
      }
      return null;
    });
    assert(spot, "an empty spot of the canvas");
    const menu = () => pg.evaluate(`(() => { const m = document.getElementById("d2-menu"); return m.hidden ? null : { target: m.dataset.target, kind: m.dataset.kind, ids: [...m.querySelectorAll("button[data-id]")].map(b => b.dataset.id) }; })()`);
    const shown = (on = true) => until(pg, `document.getElementById("d2-menu").hidden === ${!on}`);
    for (const mode of ["tint", "tint+ghost", "off"]) {
      await touch("touchStart", [spot]); await hold(pg); await touch("touchEnd", []);
      await shown(); eq((await menu()).kind, "touch"); eq((await menu()).target, "canvas");
      assert((await menu()).ids.includes("canvas.lens"), "the lens item is in the menu of a canvas that has an anchored arrow");
      const [bx, by] = await center(pg, '[data-testid="d2-ctx-canvas.lens"]');
      await pg.touchscreen.tap(bx, by); await shown(false); await frames(pg); await lensAt(mode, "touch");
    }
    const barrel = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "right", buttons: type === "mousePressed" ? 2 : 0, clickCount: 1 });
    const pen = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1 });
    await barrel("mouseMoved", ...spot); await barrel("mousePressed", ...spot); await shown(); eq((await menu()).kind, "pen"); await barrel("mouseReleased", ...spot);
    const [px, py] = await center(pg, '[data-testid="d2-ctx-canvas.lens"]');
    await pen("mouseMoved", px, py); await pen("mousePressed", px, py); await pen("mouseReleased", px, py);
    await shown(false); await frames(pg); await lensAt("tint", "pen");
    eq(await lensDoc(), lensGold[0].out.mzdraw, "the document is as it was");
  });
  await ctx.close();

  // tp-08: a list of transformation products (CSV) pasted in the Add field (keyboard) or opened by the file button (touch) becomes the
  // pathway of tests/data/log/tp-import.json in one undo step; a list with errors adds nothing and says so
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, hasTouch: true }));
  const tp8 = JSON.parse(readFileSync(path.join(root, "tests/data/log/tp-import.json"), "utf8")), tp8Csv = readFileSync(path.join(root, "tests/data/tp/tp12.csv"), "utf8");
  const tp8Gold = tp8.cases[0], tp8Bad = tp8.results[0].request.commands[0].args.csv;
  const tp8Doc = () => pg.evaluate("window.MZDRAW.save('mzdraw')"), tp8Svg = () => pg.evaluate("window.MZDRAW.save('svg')");
  const tp8Same = async what => { eq(await tp8Doc(), tp8Gold.out.mzdraw, what + ": document"); eq(await tp8Svg(), tp8Gold.out.svg, what + ": picture"); };
  await step("tp:import by keyboard: the CSV is put in the Add field and the button is pressed with Enter; the page equals the golden (document and picture), the names and levels are drawn, one undo step", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.fill("#d2-text", tp8Csv); await pg.focus("#d2-add"); await pg.keyboard.press("Enter");
    await until(pg, `${STATE}.atoms > 0`); await tool(pg, "fit"); await frames(pg);
    await tp8Same("keyboard");
    const svg = await tp8Svg(); assert(svg.includes("M2 \u00b7 L2b") && svg.includes("TP1 \u00b7 L"), "the name and the level are drawn under the structure");
    await shot(pg, "tp08-pathway");
    await tool(pg, "undo"); await want(pg, { atoms: 0, objects: 0 });                 // one import is one undo step
    await tool(pg, "redo"); await tp8Same("redo");
  });
  await step("tp:import by touch (a tablet): Open a file is tapped and the CSV chosen; the page equals the golden; one undo step", async () => {
    await ctx.close(); ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 768, height: 1024 }, deviceScaleFactor: 2, hasTouch: true, isMobile: true }));   // a tablet page of its own: the ids start at 1 as in the golden
    await want(pg, { atoms: 0 });
    const ob = await pg.locator('[data-testid="d2-open"]').boundingBox();
    const [chooser] = await Promise.all([pg.waitForEvent("filechooser"), pg.touchscreen.tap(ob.x + ob.width / 2, ob.y + ob.height / 2)]);      // Open a file, by a tap
    await chooser.setFiles({ name: "tps.csv", mimeType: "text/csv", buffer: Buffer.from(tp8Csv) });
        await until(pg, `${STATE}.atoms > 0`); await tool(pg, "fit"); await frames(pg);
    await tp8Same("touch");
    await tool(pg, "undo"); await want(pg, { atoms: 0, objects: 0 });
  });
  await step("tp:import refuses a list with a bad SMILES, an unknown parent and a cycle in words, adds nothing, and the drawing before it stays", async () => {
    await ctx.close(); ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, hasTouch: true }));   // the Add field is on the page of a desktop
    await pg.fill("#d2-text", "CCO"); await pg.click("#d2-add"); await want(pg, { atoms: 3 });
    const before = await tp8Doc();
    await pg.fill("#d2-text", tp8Bad); await pg.focus("#d2-add"); await pg.keyboard.press("Enter");
    await until(pg, "document.querySelector('#d2-note').textContent.length > 0");
    eq(await pg.innerText("#d2-note"), await pg.evaluate("I18N.t('error.draw2.tpRows')"));
    eq(await tp8Doc(), before);                                                       // nothing was added
    eq(JSON.parse(await pg.evaluate("window.MZDRAW.state()")).notice.what, "3, 4, 5, 6", "the lines of the failing rows");
  });
  // tp-import-equal: a list of eight equal molecules (a parent and seven equal products) is imported at once, by mouse, by keyboard and by touch:
  // the page finishes within the wait of the step and holds the golden of tests/data/log/tp-import.json (the last case), one undo step
  const tpe = tp8.cases[tp8.cases.length - 1], tpeCsv = tpe.request.commands[0].args.csv;
  const tpeSame = async what => { eq(await tp8Doc(), tpe.out.mzdraw, what + ": document"); eq(await tp8Svg(), tpe.out.svg, what + ": picture"); };
  await step("tp:import of eight equal molecules by mouse (the Add button clicked) and by keyboard (Enter): the page answers and equals the golden, one undo step", async () => {
    await ctx.close(); ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, hasTouch: true }));   // ids start at 1 as in the golden
    await pg.fill("#d2-text", tpeCsv); await pg.click("#d2-add");                     // mouse
    await want(pg, { atoms: 48 }); await tool(pg, "fit"); await frames(pg);
    await tpeSame("mouse");
    await tool(pg, "undo"); await want(pg, { atoms: 0, objects: 0 });
    await tool(pg, "clear"); await tool(pg, "redo"); await want(pg, { atoms: 48 });
    await tool(pg, "undo"); await want(pg, { atoms: 0 });
    await pg.fill("#d2-text", tpeCsv); await pg.focus("#d2-add"); await pg.keyboard.press("Enter");     // keyboard
    await want(pg, { atoms: 48 }); await tool(pg, "fit"); await frames(pg);
    await tpeSame("keyboard");
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Control+a"); await until(pg, `${STATE}.selected >= 48`);                           // the page still answers: select all, then the live check
    await tool(pg, "undo"); await want(pg, { atoms: 0 });
  });
  await step("tp:import of eight equal molecules by touch (a tablet): Open a file is tapped and the CSV chosen; the page equals the golden", async () => {
    await ctx.close(); ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 768, height: 1024 }, deviceScaleFactor: 2, hasTouch: true, isMobile: true }));
    const ob = await pg.locator('[data-testid="d2-open"]').boundingBox();
    const [chooser] = await Promise.all([pg.waitForEvent("filechooser"), pg.touchscreen.tap(ob.x + ob.width / 2, ob.y + ob.height / 2)]);
    await chooser.setFiles({ name: "equal.csv", mimeType: "text/csv", buffer: Buffer.from(tpeCsv) });
    await want(pg, { atoms: 48 }); await tool(pg, "fit"); await frames(pg);
    await tpeSame("touch");
    await shot(pg, "tp-import-equal");
  });
  await ctx.close();

  // tp-09: the TP table of the pathway, written by the Save list (JSON and CSV): the download is the bytes of `save` and of the golden of
  // tests/data/log/tp-table.json (the masses come from the engine of the bridge), by keyboard, by touch and by a pen; what a table could not fill is said in words
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, hasTouch: true }));
  const tp9 = JSON.parse(readFileSync(path.join(root, "tests/data/log/tp-table.json"), "utf8")).values;
  const tp9Gold = { "tptable-csv": tp9[0].out, tptable: tp9[1].out }, tp9File = { "tptable-csv": "drawing.tptable.csv", tptable: "drawing.tptable.json" };
  const tp9Bad = tp9[4].request.commands[0].args.csv;
  await step("TP table by keyboard: the list is put in the Add field, the two table entries of the Save list are focused and pressed with Enter; each download is the bytes of save() and of the golden", async () => {
    await pg.fill("#d2-text", tp8Csv); await pg.focus("#d2-add"); await pg.keyboard.press("Enter");
    await until(pg, `${STATE}.atoms > 0`); await tool(pg, "fit"); await frames(pg);
    await reveal(pg, "#d2-ex-tptable-csv"); await shotEl(pg, "#d2-fly-saveas", "tp09-save-list");           // the Save list with the two table entries (deviceScaleFactor 2)
    for (const kind of ["tptable-csv", "tptable"]) {
      await reveal(pg, `#d2-ex-${kind}`); await pg.focus(`#d2-ex-${kind}`);
      const [dl] = await Promise.all([pg.waitForEvent("download"), pg.keyboard.press("Enter")]);
      eq(dl.suggestedFilename(), tp9File[kind]);
      const file = readFileSync(await dl.path(), "utf8");
      eq(file, await pg.evaluate(`window.MZDRAW.save(${JSON.stringify(kind)})`), kind + ": the download is save()");
      eq(file, tp9Gold[kind], kind + ": the golden");
      eq(await pg.innerText("#d2-ex-note"), "", "nothing was left empty: no note");
    }
    const rows = tp9Gold["tptable-csv"].trim().split("\n"); eq([rows.length, rows[0].split(",").length], [14, 15]);
  });
  await step("TP table says what it left empty: a salt, a charged structure and an adduct the engine lacks are three notes in words, and the file is still written", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.fill("#d2-text", tp9Bad); await pg.focus("#d2-add"); await pg.keyboard.press("Enter");
    await until(pg, `${STATE}.atoms > 0`);
    await reveal(pg, "#d2-ex-tptable-csv"); await pg.focus("#d2-ex-tptable-csv");
    const [dl] = await Promise.all([pg.waitForEvent("download"), pg.keyboard.press("Enter")]);
    const file = readFileSync(await dl.path(), "utf8"); eq(file, await pg.evaluate("window.MZDRAW.save('tptable-csv')"));
    const words = await pg.evaluate("['tpMass', 'tpCharged', 'tpAdduct'].map(k => I18N.t('draw2.export.' + k, { n: 1 })).join(' ')");
    eq(await pg.innerText("#d2-ex-note"), words); eq(await pg.innerText("#d2-live"), words);
    assert(words.includes("mass engine refused") && words.includes("Charged structures") && words.includes("adduct is not one of"), "the three notes have words");
    await shotEl(pg, "#d2-fly-saveas", "tp09-save-notes");
  });
  await step("TP table by touch (a tablet) and by a pen: the table entry is tapped, then pressed with a pen; both downloads are the bytes of the golden", async () => {
    await ctx.close(); ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 768, height: 1024 }, deviceScaleFactor: 2, hasTouch: true, isMobile: true }));
    await want(pg, { atoms: 0 });
    const ob = await pg.locator('[data-testid="d2-open"]').boundingBox();
    const [chooser] = await Promise.all([pg.waitForEvent("filechooser"), pg.touchscreen.tap(ob.x + ob.width / 2, ob.y + ob.height / 2)]);
    await chooser.setFiles({ name: "tps.csv", mimeType: "text/csv", buffer: Buffer.from(tp8Csv) });
    await until(pg, `${STATE}.atoms > 0`); await tool(pg, "fit"); await frames(pg);
    await reveal(pg, "#d2-ex-tptable-csv");
    const b = await pg.locator("#d2-ex-tptable-csv").boundingBox();
    const [dl] = await Promise.all([pg.waitForEvent("download"), pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2)]);
    eq(readFileSync(await dl.path(), "utf8"), tp9Gold["tptable-csv"]);
    await reveal(pg, "#d2-ex-tptable-csv"); await shotEl(pg, "#d2-fly-saveas", "tp09-save-list-tablet");
    const cdp = await pg.context().newCDPSession(pg);
    await reveal(pg, "#d2-ex-tptable");
    const bb = await pg.locator("#d2-ex-tptable").boundingBox(), pen = { x: bb.x + bb.width / 2, y: bb.y + bb.height / 2, button: "left", clickCount: 1, pointerType: "pen" };
    const [dl2] = await Promise.all([pg.waitForEvent("download"), (async () => { await cdp.send("Input.dispatchMouseEvent", { type: "mousePressed", ...pen }); await cdp.send("Input.dispatchMouseEvent", { type: "mouseReleased", ...pen }); })()]);
    eq(readFileSync(await dl2.path(), "utf8"), tp9Gold.tptable);
  });
  await ctx.close();

  // plan-16: an agent's proposal shown as a layer, accepted or rejected hunk by hunk
  const SMI = "CCO.CC(=O)N.CCCl.c1ccccc1";
  const EDIT = [{ op: "atom.set", id: 3, el: "S" }, { op: "atom.add", id: 30, el: "C", x: 5.2321, y: 1.866 }, { op: "bond.add", id: 31, a: 7, b: 30, o: 1 },
    { op: "remove", atoms: [10], bonds: [23] }, { op: "move", atoms: [11, 12, 13, 14, 15, 16], dx: 0.0, dy: -2.0 }];
  const R = (pg, r) => pg.evaluate(r => window.MZDRAW.replay(JSON.stringify(r)), r);
  /** the drawing of four structures, and the patch that `diff` writes for the edit above: its four hunks */
  const proposalSetup = async pg => {
    await tool(pg, "clear");
    await pg.setInputFiles("#d2-file", { name: "four.smi", mimeType: "text/plain", buffer: Buffer.from(SMI) }); await want(pg, { atoms: 16 });
    await tool(pg, "fit");
    const a = await R(pg, { import: SMI, out: "mzdraw" });
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), a);
    const b = await R(pg, { import: SMI, log: EDIT, out: "mzdraw" });
    return { a, patch: JSON.parse(await R(pg, { diff: { a: JSON.parse(a), b: JSON.parse(b) } })) };
  };
  const proposing = pg => pg.evaluate("JSON.parse(window.MZDRAW.state()).proposal");
  /** the pixel of the number of a hunk on the page, read from the display list (a TEXT in the accent colour, 11, with that number) */
  const numberAt = (pg, n) => pg.evaluate(n => {
    const d = window.MZDRAW.scene(), s = window.MZDRAW.strings(), r = document.getElementById("d2-canvas").getBoundingClientRect();
    for (let i = 0; i < d.length;) {
      const op = d[i];
      if (op === 2 && d[i + 4] === 11 && s[d[i + 5]] === String(n)) return [r.x + d[i + 1], r.y + d[i + 2]];
      i += op === 1 || op === 2 ? 7 : op === 3 ? 6 : op === 6 ? 8 : 5;
    }
    return null;
  }, n);
  /** the patch is given through the field of the structure palette (on a tablet that palette is shown by its menu and tapped) */
  const propose = async (pg, patch, touch = false) => {
    if (touch) await tapPalette(pg, "structure", true);
    if (!await pg.evaluate("document.getElementById('d2-patch-more').open")) await setOpen(pg, "#d2-patch-more", true);
    await pg.fill("#d2-patch", JSON.stringify(patch.ops));
    if (touch) await pg.locator("#d2-propose").tap(); else await pg.click("#d2-propose");
    await until(pg, "!document.getElementById('d2-proposal').hidden");
    if (touch) await tapPalette(pg, "structure", false);
  };
  const accentMarks = pg => pg.evaluate(() => { const d = window.MZDRAW.scene(); let n = 0; for (let i = 0; i < d.length;) { const op = d[i]; const c = op === 1 ? d[i + 6] : op === 2 ? d[i + 4] : op === 3 ? d[i + 5] : op === 6 ? d[i + 7] : d[i + 4]; if (c === 11) n++; i += op === 1 || op === 2 ? 7 : op === 3 ? 6 : op === 6 ? 8 : 5; } return n; });
  const hunkOps = (patch, ns) => ns.flatMap(n => patch.hunks[n - 1].ops);
  const logTail = async (pg, k) => JSON.parse(await pg.evaluate("window.MZDRAW.save('log')")).slice(-k);

  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 }));
  await step("proposal, mouse and keyboard: a patch is shown over the drawing and changes nothing; keys walk, accept and reject hunks, each accept one undo step; the page equals the replay of its log", async () => {
    const { a, patch } = await proposalSetup(pg);
    eq(patch.hunks.length, 4);
    const steps0 = JSON.parse(await pg.evaluate("window.MZDRAW.save('log')")).length;
    assert(await accentMarks(pg) === 0, "no mark before the proposal");
    await propose(pg, patch);
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), a);                                  // held apart: the drawing is the same
    eq((await proposing(pg)).hunks.map(h => h.n), [1, 2, 3, 4]);
    assert(await accentMarks(pg) >= 4, "the hunks are marked in the accent colour, numbered");
    eq(await pg.locator("#d2-proposal-list li").count(), 4);
    assert((await pg.innerText("#d2-live")).includes(await pg.evaluate("I18N.t('draw2.proposal.at', { n: 1, of: 4 })")), "the live line says where the hunk is and what it does");
    await shot(pg, "plan16-proposal-desktop");
    // the keyboard: ] walks to hunk 2, Enter accepts it (one undo step), [ wraps, Delete rejects the one the keyboard is on
    await pg.focus("#d2-canvas");
    await pg.keyboard.press("]");
    eq((await proposing(pg)).cur, 2);
    assert((await pg.innerText("#d2-live")).includes(await pg.evaluate("I18N.t('draw2.hunk.atomsAdded', { what: '30' })")), "hunk 2 says it adds atom 30: " + await pg.innerText("#d2-live"));
    await pg.keyboard.press("Enter");
    eq((await proposing(pg)).hunks.map(h => h.n), [1, 3, 4]);
    eq(await logTail(pg, patch.hunks[1].ops.length), patch.hunks[1].ops);
    await pg.keyboard.press("[");
    eq((await proposing(pg)).cur, 1);                                                           // hunk 1 is the one before 3 in the ring
    await pg.keyboard.press("Delete");                                                          // rejects hunk 1: nothing in the log
    eq((await proposing(pg)).hunks.map(h => h.n), [3, 4]);
    // a mouse click on the number of hunk 4 gives it the keyboard; a click on Accept in the panel accepts it
    const [x, y] = await numberAt(pg, 4);
    assert(await pg.evaluate(([x, y]) => document.elementFromPoint(x, y)?.id === "d2-canvas", [x, y]), "the number is not covered by the panel");
    await pg.mouse.click(x, y);
    eq((await proposing(pg)).cur, 4);
    await pg.click('#d2-proposal-list button[data-act="accept:4"]');
    eq((await proposing(pg)).hunks.map(h => h.n), [3]);
    await pg.keyboard.press("Control+Enter");                                                   // the last one
    eq(await proposing(pg), null);
    assert(!await pg.isVisible("#d2-proposal"), "the panel is gone with the proposal");
    // the page equals the replay of the request: propose, accept 2, 4, 3 (hunk 1 was rejected)
    const want0 = await R(pg, { import: SMI, propose: patch.ops, accept: [2, 4, 3], out: "mzdraw" });
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), want0);
    eq(await replayEq(pg), { mzdraw: true, svg: true });
    eq(JSON.parse(await pg.evaluate("window.MZDRAW.save('log')")).length - steps0, hunkOps(patch, [2, 4, 3]).length);
    // each accept was one undo step; Ctrl+Z three times brings the drawing back
    for (let i = 0; i < 3; i++) await pg.keyboard.press("Control+z");
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), a);
    // reject all leaves the drawing and the log as they were
    await propose(pg, patch);
    await pg.click("#d2-reject-all");
    eq(await proposing(pg), null); eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), a);
    // a text that is not a patch, and a patch that does not apply, are refused in words
    await pg.fill("#d2-patch", "[1,2]"); await pg.click("#d2-propose");
    eq(await pg.innerText("#d2-patch-note"), await pg.evaluate("I18N.t('error.draw2.proposalPatch')"));
    await pg.fill("#d2-patch", '[{"op":"atom.set","id":99,"el":"N"}]'); await pg.click("#d2-propose");
    assert((await pg.innerText("#d2-patch-note")).length > 0 && await proposing(pg) === null, "refused");
  });
  // a fresh page: Clear leaves the id counter where it was, and the patch is written against the ids of a fresh import
  await ctx.close();
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 }));
  await step("proposal keys: Enter, Delete and Alt+[ act on the DRAWING until a hunk has the keyboard (and end the proposal, said at once); Ctrl+Delete rejects all; Escape ends the review", async () => {
    const { a, patch } = await proposalSetup(pg);
    const stale = await pg.evaluate("I18N.t('draw2.say.proposalStale')"), live = async () => (await pg.innerText("#d2-status")).trim().replace(/^.*?(The proposal)/s, "$1");
    const doc = () => pg.evaluate("window.MZDRAW.save('mzdraw')");
    // Enter grows a bond from the focused atom: the drawing, not the proposal
    await propose(pg, patch); await pg.focus("#d2-canvas"); await pg.keyboard.press("Home"); await pg.keyboard.press("Enter");
    await want(pg, { atoms: 17 });
    eq(await proposing(pg), null); eq(await live(), stale); assert(!await pg.isVisible("#d2-proposal"), "no panel");
    // (each edit is one undo step: Ctrl+Z gives the drawing back, with the ids the patch was written against)
    await pg.keyboard.press("Control+z"); eq(await doc(), a);
    // Delete removes the focused atom
    await propose(pg, patch); await pg.focus("#d2-canvas"); await pg.keyboard.press("Home"); await pg.keyboard.press("Delete");
    await want(pg, { atoms: 15 });
    eq(await proposing(pg), null); eq(await live(), stale);
    await pg.keyboard.press("Control+z"); eq(await doc(), a);
    // Alt+[ still turns the selection while a proposal is open
    await propose(pg, patch); await pg.focus("#d2-canvas");
    await pg.keyboard.press("Control+a"); eq((await proposing(pg)).hunks.length, 4);
    await pg.keyboard.press("Alt+["); await until(pg, `${STATE}.selected > 0`);
    assert(await doc() !== a, "the selection was turned");
    eq(await proposing(pg), null); eq(await live(), stale);
    await pg.keyboard.press("Control+z"); eq(await doc(), a);
    // Ctrl+Delete rejects all at any time, with no hunk having the keyboard: the drawing is as it was
    await propose(pg, patch); await pg.focus("#d2-canvas");
    await pg.keyboard.press("Control+Delete");
    eq(await proposing(pg), null); eq(await doc(), a);
    // with a hunk on the keyboard Enter is the proposal's, even with an atom focused; Escape ends that review and gives Enter back
    await propose(pg, patch); await pg.focus("#d2-canvas"); await pg.keyboard.press("Home"); await pg.keyboard.press("]");
    eq((await proposing(pg)).cur, 2);
    await pg.keyboard.press("Escape"); await pg.keyboard.press("Home"); await pg.keyboard.press("Enter");
    await want(pg, { atoms: 17 });
    eq(await proposing(pg), null);
  });
  await ctx.close();

  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 768, height: 1024 }, deviceScaleFactor: 2, hasTouch: true, isMobile: true }));
  await step("proposal by touch and by pen: a tap on a hunk number gives it the keyboard and says what it does, a tap on Accept or Reject acts, each accept is one undo step", async () => {
    const cdp = await ctx.newCDPSession(pg);
    const pen = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1 });
    const { a, patch } = await proposalSetup(pg);
    await propose(pg, patch, true);
    const [x, y] = await numberAt(pg, 3);
    assert(await pg.evaluate(([x, y]) => document.elementFromPoint(x, y)?.id === "d2-canvas", [x, y]), "the number is not covered by the panel " + [x, y] + await pg.evaluate(([x, y]) => document.elementFromPoint(x, y)?.outerHTML.slice(0, 80), [x, y]));
    await pg.touchscreen.tap(x, y);
    eq((await proposing(pg)).cur, 3);
    assert((await pg.innerText("#d2-live")).includes(await pg.evaluate("I18N.t('draw2.hunk.atomsChanged', { what: '3' })")), "the live line says what hunk 3 does: " + await pg.innerText("#d2-live"));
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), a);                                  // a tap on a number draws nothing
    await shot(pg, "plan16-proposal-tablet");
    const box = await pg.locator('#d2-proposal-list button[data-act="accept:3"]').boundingBox();
    assert(box.height >= 32 && box.width >= 32, "a finger can hit it: " + JSON.stringify(box));
    await pg.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
    eq((await proposing(pg)).hunks.map(h => h.n), [1, 2, 4]);
    eq(await logTail(pg, patch.hunks[2].ops.length), patch.hunks[2].ops);
    // a pen presses the number of hunk 4, then Reject
    const [px, py] = await numberAt(pg, 4);
    await pen("mouseMoved", px, py); await pen("mousePressed", px, py); await pen("mouseReleased", px, py);
    eq((await proposing(pg)).cur, 4);
    const rb = await pg.locator('#d2-proposal-list button[data-act="reject:4"]').boundingBox();
    await pen("mouseMoved", rb.x + rb.width / 2, rb.y + rb.height / 2); await pen("mousePressed", rb.x + rb.width / 2, rb.y + rb.height / 2); await pen("mouseReleased", rb.x + rb.width / 2, rb.y + rb.height / 2);
    eq((await proposing(pg)).hunks.map(h => h.n), [1, 2]);
    await pg.touchscreen.tap(...await pg.evaluate(() => { const r = document.getElementById("d2-accept-all").getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; }));
    eq(await proposing(pg), null);
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), await R(pg, { import: SMI, propose: patch.ops, accept: [3, 1, 2], out: "mzdraw" }));
    eq(await replayEq(pg), { mzdraw: true, svg: true });
    await tool(pg, "undo"); await tool(pg, "undo");                                            // the accept-all (hunks 1 and 2) and the accept of hunk 3
    eq(await pg.evaluate("window.MZDRAW.save('mzdraw')"), a);
  });
  // ---- arch-web-17: calculated values (formula, masses, m/z of adducts); the numbers are the engine's, hand-summed in the tests of the crates ----
  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 }));
  const addSmiles = (pg, smiles) => pg.setInputFiles("#d2-file", { name: "values.smi", mimeType: "text/plain", buffer: Buffer.from(smiles) });
  const VT = (key, p) => pg.evaluate(([k, q]) => I18N.t(k, q), [key, p]);
  const sentence = async (mass, average, rest) => `${await VT("draw2.values.mono", { mass })}; ${await VT("draw2.values.average", { mass: average })}; ${rest}.`;
  const valueRows = pg => pg.evaluate("[...document.querySelectorAll('#d2-values li')].map(li => li.textContent)");
  await step("calculated values by mouse and keyboard: the panel gives the masses and the m/z of each structure, one atom selected by keyboard gives its own (cut bond closed with hydrogen), select all gives the whole, Escape clears it; no number is made in JS", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    eq(await valueRows(pg), []);
    await addSmiles(pg, "Cn1cnc2c1c(=O)n(C)c(=O)n2C.CCO"); await want(pg, { atoms: 17 });
    // caffeine C8H10N4O2 194.0804 (8 * 12 + 10 * 1.00782503 + 4 * 14.00307400 + 2 * 15.99491462), ethanol C2H6O 46.0419
    eq(await valueRows(pg), [
      `${await VT("draw2.values.piece", { n: 1 })} ${await sentence("194.0804", "194.1940", "[M+H]+ 195.0877; [M-H]- 193.0731")}`,
      `${await VT("draw2.values.piece", { n: 2 })} ${await sentence("46.0419", "46.0690", "[M+H]+ 47.0491; [M-H]- 45.0346")}`]);
    eq(await pg.innerText("#d2-values-sel"), "");
    await shotEl(pg, "#d2-values", "values-panel");
    // the keyboard: Home focuses the first atom (the methyl of caffeine), Space selects it: CH4 with its bond to the nitrogen closed with hydrogen
    await pg.focus("#d2-canvas"); await pg.keyboard.press("Home"); await pg.keyboard.press(" "); await want(pg, { selected: 1 });
    await until(pg, "document.querySelector('#d2-values-sel').textContent !== ''");
    eq(await pg.innerText("#d2-values-sel"), `${await VT("draw2.values.selection", { formula: "CH4", atoms: 1 })} ${await sentence("16.0313", "16.0430", "[M+H]+ 17.0386; [M-H]- 15.0240")}`);
    await pg.keyboard.press("Control+a"); await want(pg, { selected: 17 + 17 });
    eq(await pg.innerText("#d2-values-sel"), `${await VT("draw2.values.selection", { formula: "C10H16N4O3", atoms: 17 })} ${await sentence("240.1222", "240.2630", "[M+H]+ 241.1295; [M-H]- 239.1150")}`);
    await shot(pg, "values-selection");
    await pg.keyboard.press("Escape"); await want(pg, { selected: 0 });
    await until(pg, "document.querySelector('#d2-values-sel').textContent === ''");
    // what the engine cannot read shows its words, never the key: a metal ion, an explicit hydrogen atom, a structure of 201 atoms
    for (const [smiles, key] of [["CC(=O)[O-].[Na+]", "error.grafo.element"], ["[2H]C([2H])([2H])O", "error.grafo.element"], ["C".repeat(201), "error.grafo.tooBig"]]) {
      await tool(pg, "clear"); await want(pg, { atoms: 0 });
      await addSmiles(pg, smiles); await until(pg, "document.querySelectorAll('#d2-values li').length > 0");
      const rows = await valueRows(pg), text = await VT(key, {});
      assert(text !== key && !text.startsWith("error."), "the catalog has a text for " + key);
      assert(rows.some(r => r.includes(text)) && rows.every(r => !r.includes("error.")), `${smiles.slice(0, 20)}: the panel says it in words: ${JSON.stringify(rows)}`);
    }
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await addSmiles(pg, "Cn1cnc2c1c(=O)n(C)c(=O)n2C.CCO"); await want(pg, { atoms: 17 });
    // the document is untouched by all of it, and the same numbers come from the command line door (replay_glue, the example of the parity)
    const doc = await pg.evaluate("window.MZDRAW.save('mzdraw')");
    const v = JSON.parse(await pg.evaluate(d => window.MZDRAW.replay_glue(JSON.stringify({ import: d, values: true })), doc));
    eq(v.structures.map(s => s.mass), ["194.0804", "46.0419"]);
  });

  await step("calculated values under each structure: the style panel chooses what is drawn and the adducts by keyboard; the page SVG equals the replay of its log through the glue door", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await addSmiles(pg, "CCO.CC(=O)O"); await want(pg, { atoms: 7 });
    const plain = await pg.evaluate("window.MZDRAW.save('svg')");
    if (!(await pg.evaluate("document.querySelector('#d2-style-more').open"))) await pg.click("#d2-style-more summary");
    const SEL = '[id="d2-style-display.values"]';
    eq(await pg.locator(`${SEL} option`).evaluateAll(o => o.map(x => x.value)), ["off", "formula", "mass", "mz"]);
    eq(await pg.inputValue(SEL), "off");
    // by keyboard: the select reached with the focus, a letter picks "Formula and mass"
    await pg.focus(SEL); await pg.keyboard.press("ArrowDown"); await want(pg, { atoms: 7 });
    eq(await pg.inputValue(SEL), "formula");
    assert((await pg.evaluate("window.MZDRAW.save('svg')")).includes(">C2H6O</text>"), "the formula is drawn under the structure");
    await pg.selectOption(SEL, "mass");
    const page = await pg.evaluate("window.MZDRAW.save('svg')");
    assert(page.includes(">C2H6O  46.0419</text>") && page.includes(">C2H4O2  60.0211</text>"), "the mass is drawn under each structure");
    // the page equals the replay of its log through the glue door (the command line twin, same engine and same bytes)
    eq(await pg.evaluate("window.MZDRAW.replay_glue(JSON.stringify({ log: JSON.parse(window.MZDRAW.save('log')), set: ['display.values=mass'], out: 'svg' }))"), page);
    // the display list the canvas paints has the same text
    assert(await pg.evaluate("window.MZDRAW.scene() && window.MZDRAW.strings().includes('C2H6O  46.0419')"), "the canvas paints it");
    await shot(pg, "values-under-structures");
    // the adducts: a checkbox each, the module holds the list; the m/z mode draws one line per adduct
    await pg.selectOption(SEL, "mz");
    const boxes = await pg.locator("#d2-style-adducts input").evaluateAll(b => b.map(x => [x.value, x.checked]));
    eq(boxes.filter(b => b[1]).map(b => b[0]), ["[M+H]+", "[M-H]-"]);
    await pg.focus('#d2-style-adducts input[value="[M+Na]+"]'); await pg.keyboard.press(" ");
    await until(pg, "JSON.parse(window.MZDRAW.state()).values.adducts.includes('[M+Na]+')");
    eq(await pg.evaluate("JSON.parse(window.MZDRAW.state()).values.adducts"), ["[M+H]+", "[M+Na]+", "[M-H]-"]);          // the order of the schema
    const mz = await pg.evaluate("window.MZDRAW.save('svg')");
    assert(mz.includes(">[M+H]+ 47.0491</text>") && mz.includes(">[M+Na]+ 69.0311</text>") && mz.includes(">[M-H]- 45.0346</text>"), "a line per adduct: " + mz.slice(-600));
    eq(await pg.evaluate("window.MZDRAW.replay_glue(JSON.stringify({ log: JSON.parse(window.MZDRAW.save('log')), set: ['display.values=mz', 'display.adducts=[M+H]+,[M+Na]+,[M-H]-'], out: 'svg' }))"), mz);
    await pg.selectOption(SEL, "mass");
    // the settings file keeps the words, and Rust gives the page back what it saved
    const file = await pg.evaluate("window.MZDRAW.style()");
    assert(file.includes('"display.values":"mass"') && file.includes('"display.adducts":"[M+H]+,[M+Na]+,[M-H]-"'), "kept in the settings file");
    // off draws nothing
    await pg.selectOption(SEL, "off");
    eq(await pg.evaluate("window.MZDRAW.save('svg')"), plain);
    eq(await pg.evaluate("JSON.parse(window.MZDRAW.save('mzdraw')).atoms.length"), 7);       // a style is no edit
  });


  await step("calculated values by touch and by pen: on a tablet a tap on an atom with the select tool gives the values of the selection, a pen press on a style checkbox changes the adducts, and the panel follows", async () => {
    const t = await context(browser, { viewport: { width: 768, height: 1024 }, deviceScaleFactor: 2, hasTouch: true, isMobile: true });
    try {
      const p = t.pg, cdp = await t.ctx.newCDPSession(p);
      await addSmiles(p, "CCO"); await want(p, { atoms: 3 });
      await tool(p, "fit"); await tool(p, "select");
      // a tap on the first atom of the ethanol: the selection is the methyl with its bond closed with hydrogen
      // the atom is found from the display list: the first text or line end nearest the document point of the atom
      const at = await p.evaluate(() => { const c = document.querySelector("#d2-canvas").getBoundingClientRect(), s = window.MZDRAW.scene(); const pts = []; for (let i = 0; i < s.length;) { const k = s[i]; if (k === 1) { pts.push([s[i + 1], s[i + 2]], [s[i + 3], s[i + 4]]); i += 7; } else if (k === 2) i += 7; else if (k === 3) i += 6; else if (k === 6) i += 8; else i += 5; } pts.sort((u, v) => u[0] - v[0]); return [c.x + pts[0][0], c.y + pts[0][1]]; });
      await p.touchscreen.tap(at[0], at[1]); await want(p, { selected: 1 });
      await until(p, "document.querySelector('#d2-values-sel').textContent !== ''");
      eq(await p.innerText("#d2-values-sel"), `${await p.evaluate(() => I18N.t("draw2.values.selection", { formula: "CH4", atoms: 1 }))} ${await p.evaluate(() => `${I18N.t("draw2.values.mono", { mass: "16.0313" })}; ${I18N.t("draw2.values.average", { mass: "16.0430" })}; [M+H]+ 17.0386; [M-H]- 15.0240.`)}`);
      await shot(p, "values-tablet");
      // a pen press on a checkbox of the style panel adds the adduct to the list
      await tapPalette(p, "style", true);
      if (!(await p.evaluate("document.querySelector('#d2-style-more').open"))) await p.locator("#d2-style-more summary").tap();
      const cb = '#d2-style-adducts input[value="[M+K]+"]';
      await p.locator(cb).scrollIntoViewIfNeeded();
      const b = await p.locator(cb).boundingBox(), x = b.x + b.width / 2, y = b.y + b.height / 2;
      const pen = (type) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1 });
      await pen("mouseMoved"); await pen("mousePressed"); await pen("mouseReleased");
      await until(p, "JSON.parse(window.MZDRAW.state()).values.adducts.includes('[M+K]+')");
      assert((await p.innerText("#d2-values-sel")).includes("[M+K]+ 54.9945"), "the selection has the new adduct: " + await p.innerText("#d2-values-sel"));
      // a tap on a checkbox takes it away again
      await p.locator(cb).scrollIntoViewIfNeeded();
      const b2 = await p.locator(cb).boundingBox(); await p.touchscreen.tap(b2.x + b2.width / 2, b2.y + b2.height / 2);
      await until(p, "!JSON.parse(window.MZDRAW.state()).values.adducts.includes('[M+K]+')");
      // the select that chooses what is drawn is reached by a tap too
      assert(await p.locator('[id="d2-style-display.values"]').evaluate(el => el.getBoundingClientRect().height >= 24), "a finger can hit the select");
    } finally { await t.ctx.close(); }
  });

  // feedback-about: the About box and the report box. The report is a prefilled new issue of the repository opened in a NEW TAB by a link; the URL is
  // made in Rust (`feedback`), nothing is sent by the page itself, and the command log is off unless the box is ticked. Keyboard, touch and pen.
  {
    const { ctx: fb, pg: fp } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, hasTouch: true });
    await fb.grantPermissions(["clipboard-read", "clipboard-write"], { origin: new URL(base).origin });
    // the new tab is intercepted: nothing reaches GitHub, and the page itself must never ask for it
    await fb.route(/^https:\/\/github\.com\//, r => r.fulfill({ status: 200, contentType: "text/html", body: "<title>issue</title>" }));
    const ghReq = [];
    fp.on("request", r => { if (/github\.com/.test(r.url())) ghReq.push(r.url()); });
    const opened = async act => { const [pop] = await Promise.all([fb.waitForEvent("page"), act()]); await pop.waitForURL(/github\.com/); const u = new URL(pop.url()); await pop.close(); return u; };
    const shown = (id, on = true) => until(fp, `document.getElementById("${id}").hidden === ${!on}`);
    const about = async () => JSON.parse(await fp.evaluate("window.MZDRAW.about()"));
    const log = () => fp.evaluate("window.MZDRAW.save('log')");
    const UA = await fp.evaluate("navigator.userAgent");
    /** what an opened issue must say: the repository, the form, the title, the page version, the user agent and the language */
    const readIssue = async (u, template, hasLog) => {
      eq(u.origin + u.pathname, (await about()).repository + "/issues/new");
      eq(u.searchParams.get("template"), template);
      eq(u.searchParams.get("title"), template === "bug.yml" ? "Bug:" : "Improvement:");
      const page = u.searchParams.get("page");
      assert(page.includes("Version: " + (await about()).version) && page.includes("Browser: " + UA.slice(0, 300)) && /Language: \S+/.test(page), page);
      eq(u.searchParams.has("log"), hasLog);
      if (hasLog) eq(u.searchParams.get("log"), await log());
      assert(u.href.length <= 6000, "the URL stays under the limit");
    };
    await step("feedback by keyboard: the report button opens a popover (no modal), Bug / Improvement by the arrows, the log box is off by default; Enter opens the prefilled issue of the right form in a new tab; Escape closes and gives the focus back", async () => {
      await tool(fp, "clear"); await want(fp, { atoms: 0 });
      await fp.focus("#d2-canvas"); await fp.keyboard.press("Enter"); await want(fp, { atoms: 1 });           // one operation in the log
      assert((await log()).length > 4, "the log has an operation");
      await fp.focus("#d2-report-open"); await fp.keyboard.press("Enter"); await shown("d2-report");
      eq(await fp.getAttribute("#d2-report-open", "aria-expanded"), "true");
      eq(await fp.evaluate("document.querySelector('dialog[open]') === null && document.getElementById('d2-report').getAttribute('role')"), "group");
      eq(await fp.evaluate("document.activeElement.value"), "bug");
      eq(await fp.isChecked("#d2-report-log"), false, "the log is off by default");
      assert((await fp.innerText("#d2-report")).includes("visible publicly on GitHub"), "the box says the log is public");
      await shot(fp, "feedback-report");
      await fp.keyboard.press("ArrowRight"); eq(await fp.evaluate("document.activeElement.value"), "improvement");
      await fp.keyboard.press("Tab"); await fp.keyboard.press("Tab");                                         // the log box, then the button
      eq(await fp.evaluate("document.activeElement.id"), "d2-report-go");
      await readIssue(await opened(() => fp.keyboard.press("Enter")), "improvement.yml", false);
      // ticked: the log of the drawing is in the link, as the module wrote it
      await fp.keyboard.press("Shift+Tab"); eq(await fp.evaluate("document.activeElement.id"), "d2-report-log"); await fp.keyboard.press("Space");
      eq(await fp.isChecked("#d2-report-log"), true);
      await fp.keyboard.press("Tab");
      await readIssue(await opened(() => fp.keyboard.press("Enter")), "improvement.yml", true);
      eq(await fp.innerText("#d2-report-note"), "");
      await fp.keyboard.press("Escape"); await shown("d2-report", false);
      eq(await fp.evaluate("document.activeElement.id"), "d2-report-open");
      eq(ghReq, [], "the page asked nothing of GitHub");
    });
    await step("feedback: a log too long for the link is left out with a keyed note and Copy the log; no operation gives the other note; the copied text is the log", async () => {
      await tool(fp, "clear"); await want(fp, { atoms: 0 }); await fillIn(fp, "#d2-text", "C".repeat(90)); await saveAs(fp, "#d2-add"); await want(fp, { atoms: 90 });
      await fp.click("#d2-report-open"); await shown("d2-report");
      await fp.check("input[value=bug]"); await fp.check("#d2-report-log");
      const u = await opened(() => fp.click("#d2-report-go"));
      await readIssue(u, "bug.yml", false);
      eq(await fp.innerText("#d2-report-note"), await fp.evaluate("I18N.t('feedback.logTooLong')"));
      eq(await fp.isVisible("#d2-report-copy"), true);
      await shot(fp, "feedback-report-long");
      await fp.click("#d2-report-copy");
      await until(fp, `document.getElementById("d2-report-note").textContent === I18N.t("draw2.report.copied")`);
      eq(await fp.evaluate("navigator.clipboard.readText()"), await log());
      await fp.click("#d2-report-close"); await shown("d2-report", false);
      await fp.evaluate("window.MZDRAW.open(window.MZDRAW.save('mzdraw'))");                                  // the drawing stays, its history is empty: no operation to log
      await fp.click("#d2-report-open"); await shown("d2-report"); await fp.check("#d2-report-log");
      await readIssue(await opened(() => fp.click("#d2-report-go")), "bug.yml", false);
      eq(await fp.innerText("#d2-report-note"), await fp.evaluate("I18N.t('feedback.noLog')"));
      eq(ghReq, [], "the page asked nothing of GitHub");
      await fp.click("#d2-report-close");
    });
    await step("About by keyboard: the name from the catalog, one line of what it is, the version, the links to the repository and the licence, the privacy sentence and the report button; Escape closes", async () => {
      await fp.focus("#d2-about-open"); await fp.keyboard.press("Enter"); await shown("d2-about");
      const a = await about();
      eq(await fp.innerText("#d2-about-name"), await fp.evaluate("I18N.t('draw2.name')"));
      eq(await fp.innerText("#d2-about-name"), a.name);
      assert((await fp.innerText("#d2-about-version")).includes(a.version), "the version");
      eq(await fp.getAttribute("#d2-about-repo", "href"), a.repository);
      eq(await fp.getAttribute("#d2-about-licence", "href"), a.licence);
      eq(await fp.getAttribute("#d2-about-repo", "rel"), "noopener noreferrer");
      const text = await fp.innerText("#d2-about");
      assert(text.includes("Once the page is loaded it works offline and sends nothing") && text.includes("Report a bug / suggest an improvement"), text);
      await shot(fp, "feedback-about");
      await fp.keyboard.press("Tab"); await fp.keyboard.press("Tab");
      // the report button of the box opens the report box in its place
      await fp.focus("#d2-about-report"); await fp.keyboard.press("Enter"); await shown("d2-report"); await shown("d2-about", false);
      await fp.keyboard.press("Escape"); await shown("d2-report", false);
      await fp.focus("#d2-about-open"); await fp.keyboard.press("Enter"); await shown("d2-about");
      await fp.keyboard.press("Escape"); await shown("d2-about", false);
      eq(await fp.evaluate("document.activeElement.id"), "d2-about-open");
      // Escape closes an open box also when the focus is elsewhere (on the canvas); the focus stays there
      await fp.keyboard.press("Enter"); await shown("d2-about"); await fp.focus("#d2-canvas"); await fp.keyboard.press("Escape"); await shown("d2-about", false);
      eq(await fp.evaluate("document.activeElement.id"), "d2-canvas");
      eq(ghReq, [], "the page asked nothing of GitHub");
    });
    await step("feedback by touch: tap About, tap its report button, tap Improvement, tap Open on GitHub; the targets are 44 px; the issue is the improvement form", async () => {
      const tap = async sel => { const [x, y] = await center(fp, sel); await fp.touchscreen.tap(x, y); };
      await tap("#d2-about-open"); await shown("d2-about");
      await tap("#d2-about-report"); await shown("d2-report"); await shown("d2-about", false);
      for (const sel of ["#d2-report-go", "#d2-report-close"]) assert((await fp.locator(sel).boundingBox()).height >= 43, sel + " is a target for a finger");
      await tap("input[value=improvement]");
      await readIssue(await opened(() => tap("#d2-report-go")), "improvement.yml", false);
      await tap("#d2-report-close"); await shown("d2-report", false);
      eq(ghReq, [], "the page asked nothing of GitHub");
    });
    await step("feedback by pen: a pen tap on the report button, on Bug, on the log box and on Open on GitHub; the issue is the bug form with the log", async () => {
      await tool(fp, "clear"); await fp.focus("#d2-canvas"); await fp.keyboard.press("Enter"); await want(fp, { atoms: 1 });
      const cdp = await fb.newCDPSession(fp);
      const pen = (type, x, y) => cdp.send("Input.dispatchMouseEvent", { type, x, y, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left", buttons: type === "mousePressed" ? 1 : 0, clickCount: 1 });
      const tap = async sel => { const [x, y] = await center(fp, sel); await pen("mouseMoved", x, y); await pen("mousePressed", x, y); await pen("mouseReleased", x, y); };
      await tap("#d2-report-open"); await shown("d2-report");
      await fp.setChecked("#d2-report-log", false);
      await tap("input[value=bug]"); await tap("#d2-report-log"); assert(await fp.isChecked("#d2-report-log"), "a pen tap ticks the log box");
      assert((await log()).length > 4, "pen step: the log has an operation " + await log());
      await readIssue(await opened(() => tap("#d2-report-go")), "bug.yml", true);
      await tap("#d2-report-close"); await shown("d2-report", false);
      eq(ghReq, [], "the page asked nothing of GitHub");
    });
    await fb.close();
  }

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
