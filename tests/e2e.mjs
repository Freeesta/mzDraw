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
/** a screenshot of one element at deviceScaleFactor 2 (the drawing checks panel, light, dark and tablet: docs/shots/) */
const shotEl = async (pg, sel, name) => { if (shots) await pg.locator(sel).screenshot({ path: path.join(shots, name + ".png") }); };
const shot = async (pg, name) => { if (shots) await pg.screenshot({ path: path.join(shots, name + ".png") }); };

/** the PDF round trip: acetic acid is drawn, the PDF is downloaded the way `get` says (keyboard or touch), it is a vector PDF with the
 * text as text and the document inside, and Open a file brings back the same drawing; a PDF of another program is refused in words */
async function pdfRoundTrip(pg, get) {
  await tool(pg, "clear"); await want(pg, { atoms: 0 });
  await pg.fill("#d2-text", "CC(=O)O"); await pg.click("#d2-add"); await want(pg, { atoms: 4, bonds: 3 });
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

  await step("molfile (V2000 and V3000), SMILES, SVG, PNG and the mzdraw/1 document are saved as the module writes them; the pictures open again", async () => {
    const got = {};
    for (const [button, kind, name] of [["molfile", "molfile", "drawing.mol"], ["v3000", "molfile-v3000", "drawing-v3000.mol"], ["smiles", "smiles", "drawing.smi"], ["ket", "ket", "drawing.ket"], ["cdxml", "cdxml", "drawing.cdxml"], ["cml", "cml", "drawing.cml"],
      ["svg", "svg", "drawing.svg"], ["mzdraw", "mzdraw", "drawing.mzdraw.json"]]) {
      const [d] = await Promise.all([pg.waitForEvent("download"), pg.click(`#d2-ex-${button}`)]);
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
    const [p] = await Promise.all([pg.waitForEvent("download"), pg.click("#d2-ex-png")]);
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
    await pg.click("#d2-data-more summary");
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
    const [sd] = await Promise.all([pg.waitForEvent("download"), pg.click("#d2-ex-sdf")]);
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
    await pg.click("#d2-data-more summary");
    // the annotation of a structure: a confidence level of the scale, an identifier, evidence; drawn, saved, read back, undone
    await pg.click("#d2-ann-more summary");
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
    await pg.click("#d2-ann-more summary");
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
    const [r] = await Promise.all([pg.waitForEvent("download"), pg.click("#d2-ex-rxn")]);
    eq(r.suggestedFilename(), "drawing.rxn");
    const rxn = readFileSync(await r.path(), "utf8");
    eq(rxn, await pg.evaluate("window.MZDRAW.save('rxn')"));
    assert(rxn.startsWith("$RXN\n") && rxn.includes("\n  1  1\n$MOL\n") && rxn.split("$MOL\n").length === 3, "a reaction file with one reactant and one product");
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await pg.setInputFiles("#d2-file", { name: "drawing.rxn", mimeType: "chemical/x-mdl-rxnfile", buffer: Buffer.from(rxn) }); await want(pg, { atoms: 6, bonds: 4, objects: 1 });
    await tool(pg, "clear"); await pg.fill("#d2-text", "CCO"); await pg.click("#d2-add"); await want(pg, { atoms: 3 });
    await pg.click("#d2-ex-rxn");
    await until(pg, "document.querySelector('#d2-ex-note').textContent.length > 0");
    eq(await pg.innerText("#d2-ex-note"), await pg.evaluate("I18N.t('error.draw2.rxnArrow')"));
    await pg.click("#d2-ex-smiles"); await until(pg, "document.querySelector('#d2-ex-note').textContent === ''");       // the next save clears the note
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
    // the refused entry stays to be corrected, marked and selected; typing takes the mark away; a good entry is taken
    eq(await pg.getAttribute("#d2-label", "aria-invalid"), "true");
    eq(await pg.evaluate("(() => { const i = document.querySelector('#d2-label'); return i.value.slice(i.selectionStart, i.selectionEnd); })()"), "Nope");
    await shot(pg, "label-refused");
    await pg.keyboard.type("R2"); eq(await pg.getAttribute("#d2-label", "aria-invalid"), null);
    await pg.keyboard.press("Enter"); await want(pg, { tool: "label:R2" });
    await pg.fill("#d2-label", "R1"); await pg.press("#d2-label", "Enter"); await want(pg, { tool: "label:R1" });
    await pg.fill("#d2-label", "Nope"); await pg.press("#d2-label", "Enter");
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
    await pg.fill("#d2-label", "CO2H"); await pg.press("#d2-label", "Enter"); await want(pg, { tool: "label:CO2H" });
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
    await pg.fill("#d2-text-tool", "   "); await pg.press("#d2-text-tool", "Enter");
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
    const [d] = await Promise.all([pg.waitForEvent("download"), pg.click("#d2-ex-svg")]);
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
    await pdfRoundTrip(pg, async () => { await pg.focus("#d2-ex-pdf"); await pg.keyboard.press("Enter"); });
    await shot(pg, "pdf-download");
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

  ({ ctx, pg, cx, cy } = await context(browser, { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 }));
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

  await step("tablet with touch: the PFAS scale and level 2c of the annotation, set by tapping", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "bond:down"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    const tapOn = async sel => { const b = await pg.locator(sel).boundingBox(); await pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2); };
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
  });

  await step("PDF out by touch: the same file, downloaded with a tap and opened again", async () => {
    await pg.locator("#d2-ex-pdf").scrollIntoViewIfNeeded();
    const pdf = await pdfRoundTrip(pg, async () => { await pg.locator("#d2-ex-pdf").scrollIntoViewIfNeeded(); const b = await pg.locator("#d2-ex-pdf").boundingBox(); await pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2); });
    eq(pdf.subarray(0, 5).toString("latin1"), "%PDF-");
  });

  await step("PDF out by pen: the button pressed with a pen gives the same file", async () => {
    await pg.locator("#d2-ex-pdf").scrollIntoViewIfNeeded();
    const cdp = await pg.context().newCDPSession(pg);
    const pdf = await pdfRoundTrip(pg, async () => {
      const b = await pg.locator("#d2-ex-pdf").boundingBox(), at = { x: b.x + b.width / 2, y: b.y + b.height / 2, button: "left", clickCount: 1, pointerType: "pen" };
      await cdp.send("Input.dispatchMouseEvent", { type: "mousePressed", ...at }); await cdp.send("Input.dispatchMouseEvent", { type: "mouseReleased", ...at });
    });
    eq(pdf.subarray(0, 5).toString("latin1"), "%PDF-");
  });

  await step("drawing checks by touch: a tap on the line selects the atom and bond it is about", async () => {
    await tool(pg, "clear"); await want(pg, { atoms: 0, std: 0 });
    await tool(pg, "bond:1"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    await tool(pg, "bond:up"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 3, bonds: 2, std: 1 });
    if (!(await pg.evaluate("document.querySelector('#d2-std').open"))) await pg.locator("#d2-std summary").tap();
    await pg.locator("#d2-std-list button").tap();
    await want(pg, { selected: 2 });
    await shotEl(pg, "#d2-std-panel", "std-tablet");
    eq(await pg.innerText("#d2-live"), await pg.evaluate("I18N.t('draw2.std.selected', { n: 2 })"));
    assert(await pg.evaluate("document.documentElement.scrollWidth <= innerWidth"), "the panel does not scroll the page sideways");
  });

  await step("tablet with touch: a group label on the left of its bond is written reversed", async () => {
    const drawn = () => pg.evaluate("Array.from(window.MZDRAW.strings())");
    await tool(pg, "clear"); await want(pg, { atoms: 0 });
    await tool(pg, "bond:1"); await pg.touchscreen.tap(cx, cy); await want(pg, { atoms: 2, bonds: 1 });
    await pg.fill("#d2-label", "CO2H"); await pg.press("#d2-label", "Enter"); await want(pg, { tool: "label:CO2H" });
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
