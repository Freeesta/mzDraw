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
import { readFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const { chromium } = createRequire(import.meta.url)("playwright");       // require, so that a global install (NODE_PATH) is found too
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
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
  catch (e) { steps.push([name, "FAIL " + String(e && e.message || e).split("\n")[0].slice(0, 300)]); }
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
const tool = (pg, name) => pg.click(`[data-testid="d2-${name}"]`);
const want = (pg, kv) => until(pg, `(() => { const s = ${STATE}, w = ${JSON.stringify(kv)}; return Object.keys(w).every(k => s[k] === w[k]); })()`);
const shot = async (pg, name) => { if (shots) await pg.screenshot({ path: path.join(shots, name + ".png") }); };

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
    eq(await state(pg), { atoms: 0, bonds: 0, objects: 0, tool: "bond:1", warnings: 0, selected: 0 });
    eq(await pg.evaluate("window.MZDRAW.version()"), "mzdraw/1");
    eq(await pg.innerText("#d2-summary"), await pg.evaluate("I18N.t('draw.check.empty')"));
    assert(await pg.locator("#d2-tools button").count() >= 26, "the toolbar");
    eq(await pg.evaluate("[...document.querySelectorAll('#d2-tools button')].filter(b => !b.title || b.title.length > 90 || !b.getAttribute('aria-label')).map(b => b.dataset.cmd)"), []);      // every tool says in one short sentence what it does
    eq(await pg.evaluate("[...document.querySelectorAll('button')].filter(b => b.offsetParent && (b.offsetWidth < 24 || b.offsetHeight < 24)).map(b => b.id || b.dataset.cmd)"), []);
    eq(await pg.evaluate("[...I18N.missing || []]"), []);
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

  await step("molfile, SVG and the mzdraw/1 document are saved as the module writes them", async () => {
    const got = {};
    for (const [kind, name] of [["molfile", "drawing.mol"], ["svg", "drawing.svg"], ["mzdraw", "drawing.mzdraw.json"]]) {
      const [d] = await Promise.all([pg.waitForEvent("download"), pg.click(`#d2-ex-${kind}`)]);
      eq(d.suggestedFilename(), name);
      got[kind] = readFileSync(await d.path(), "utf8");
      eq(got[kind], await pg.evaluate(`window.MZDRAW.save('${kind}')`));
    }
    assert(got.molfile.includes("V2000") && got.molfile.trimEnd().endsWith("M  END"), "a V2000 molfile");
    assert(got.svg.startsWith("<svg") && got.svg.includes("<line") && got.svg.replaceAll("OH", "O").includes(">O<"), "an SVG with lines and labels");
    const doc = JSON.parse(got.mzdraw);
    assert(doc.format === "mzdraw/1" && doc.atoms.length === 4 && doc.bonds.length === 3, "an mzdraw/1 document");
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
    await pg.fill("#d2-label", "Ph"); await pg.press("#d2-label", "Enter"); await want(pg, { tool: "label:Ph" });
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
    await pg.fill("#d2-label", "Nope"); await pg.press("#d2-label", "Enter");
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('error.draw2.label')"));
    eq((await state(pg)).tool, "label:R1");                 // a refused label changes nothing
    await tool(pg, "atom:N"); await pg.mouse.click(cx + 35, cy - 20);       // an element takes the label away
    eq(await labs(), ["Ph", ""]);
    await tool(pg, "undo"); eq(await labs(), ["Ph", "R1"]);
    await shot(pg, "labels");
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
    await pg.fill("#d2-text-tool", "   "); await pg.press("#d2-text-tool", "Enter");
    eq((await state(pg)).tool, "erase");                    // an empty text picks nothing
    await tool(pg, "undo"); await want(pg, { objects: 4 });
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
  await ctx.close();
} finally {
  await browser.close();
  if (server) server.close();
}
for (const s of steps) console.log(" ", s[1] === "ok" ? "ok  " : "FAIL", s[0], s[1] === "ok" ? "" : s[1]);
for (const e of errs) console.log("  page:", e.join(" "));
console.log(`e2e ${base}: ${steps.filter(s => s[1] === "ok").length}/${steps.length} steps ok, ${errs.length} page errors`);
process.exit(steps.length === 11 && !errs.length && steps.every(s => s[1] === "ok") ? 0 : 1);
