// Kill test of tp-11 (the step field): the action counts of three hydroxylation products of methylcyclohexane made with the step
// field (right-click the parent atom, the item Apply step..., tap the ring of the site, Enter; with a key of its own, which the kill
// test below removes, it would be click, Shift+T, ring, Enter: the same 4 actions). The field arm is PLAYED here in Chromium on site/
// (tools/build_site.sh) and the atoms it draws are checked (4 oxygens, 28 carbons: the parent and three copies of C7 with 1 + 1 + 2
// hydroxyls, the third product being a product of the first). The derive-then-edit arm is NOT replayed here: its counts are those of the committed path `parent-five-tps` of
// tests/data/velocity/mzdraw.json (per hydroxylation: click the parent atom, Shift+D, the O tool, a drag, Select = 5; the last
// product of a chain has no Select = 4), measured by `node tests/velocity.mjs --check`. Actions are counted as that harness counts
// them: one per tool press, click, drag or key. The parent (ring, methyl: 6 actions) is the same in both arms.
// usage: node tools/velocity/step_field.mjs        data on stdout (JSON), diagnostics on stderr; exit 0 both arms reach the target, 1 not
import { createRequire } from "node:module";
import http from "node:http";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const { chromium } = createRequire(path.join(root, "tests/e2e.mjs"))("playwright");
const TYPES = { ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".json": "application/json" };
const server = http.createServer(async (q, s) => {
  let p = decodeURIComponent(new URL(q.url || "/", "http://x").pathname); if (p.endsWith("/")) p += "index.html";
  try { const b = await readFile(path.join(root, "site", path.normalize(p))); s.writeHead(200, { "content-type": TYPES[path.extname(p)] || "application/octet-stream" }); s.end(b); } catch { s.writeHead(404); s.end(); }
});
await new Promise(r => server.listen(0, "127.0.0.1", () => r(null)));
const browser = await chromium.launch();

/** a path is a list of [kind, ...]: "tool" NAME, "click" x y, "context" x y (right button), "item" ID (an item of the menu), "drag" x0 y0 x1 y1,
 * "key" K (pixels from the middle of the canvas) */
async function play(steps) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 }), pg = await ctx.newPage();
  await pg.goto(`http://127.0.0.1:${server.address().port}/`);
  await pg.waitForFunction(() => document.documentElement.dataset.d2Ready === "1");
  const b = await pg.locator("#d2-canvas").boundingBox(), cx = b.x + b.width / 2, cy = b.y + b.height / 2;
  await pg.evaluate(() => /** @type {any} */ (window).MZDRAW.set("pathway.derive=on"));
  for (const [kind, ...a] of steps) {
    if (kind === "tool") await pg.click(`[data-testid="d2-${a[0]}"]`);
    else if (kind === "click") await pg.mouse.click(cx + a[0], cy + a[1]);
    else if (kind === "context") await pg.mouse.click(cx + a[0], cy + a[1], { button: "right" });
    else if (kind === "item") await pg.click(`[data-testid="d2-ctx-${a[0]}"]`);
    else if (kind === "drag") { await pg.mouse.move(cx + a[0], cy + a[1]); await pg.mouse.down(); await pg.mouse.move(cx + (a[0] + a[2]) / 2, cy + (a[1] + a[3]) / 2); await pg.mouse.move(cx + a[2], cy + a[3]); await pg.mouse.up(); }
    else if (kind === "key") await pg.keyboard.press(a[0]);
    await pg.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
  }
  const doc = JSON.parse(await pg.evaluate(() => /** @type {any} */ (window).MZDRAW.save("mzdraw")));
  await ctx.close();
  return doc;
}
/** the pixel of an atom of the document, from atom 1 of the parent that is clicked at (-330, -200) (40 px a bond, y up in the document) */
const px = (d, id) => { const a = d.atoms.find(x => x.id === id), o = d.atoms.find(x => x.id === 1); return [Math.round(-330 + 40 * (a.x - o.x)), Math.round(-200 - 40 * (a.y - o.y))]; };

const parent = [["tool", "ring:6"], ["click", -330, -240], ["tool", "bond:1"], ["click", -330, -280], ["key", "v"], ["click", -330, -200]];
// the copies of a derive: ids 15..21 for the parent atoms 1..6 and 13, copy 2 starts at 31 (the ids are those of the page; checked below)
const base = await play(parent);
const at = id => px(base, id);
// step field: a right-click on the parent atom selects it and opens the menu, its item opens the field, a ring tapped picks the site, Enter derives
const fieldTP = (parentAtom, site) => [["context", ...parentAtom], ["item", "tp.step"], ["click", ...site], ["key", "Enter"]];
// the three products: hydroxylation at the ring carbon 2, at the ring carbon 6, and of the first product at its carbon next to the new OH
const A = [...parent, ...fieldTP(at(1), at(2)), ...fieldTP(at(1), at(6))];
const docA2 = await play(A);

const a3 = [...A, ...fieldTP(px(docA2, 15), px(docA2, 17))];
const docA = await play(a3);
const oxygens = docA.atoms.filter(a => a.el === "O").length, carbons = docA.atoms.filter(a => a.el === "C").length;
const result = {
  format: "step-field-velocity/1",
  task: "three hydroxylation products of methylcyclohexane (two of the parent, one of the first product)",
  parent_actions: parent.length,
  step_field: { per_product: 4, total: a3.length, steps: "right-click the parent atom, the item Apply step..., tap the ring of the site, Enter" },
  derive_then_edit: { per_product: 5, last_product_of_a_chain: 4, total: parent.length + 5 + 5 + 4, steps: "click the parent atom, Shift+D, press the O tool, drag a bond to a new O, press Select (the same edit as tests/data/velocity/mzdraw.json parent-five-tps)" },
  reached: { oxygens, carbons, structures: 4, atoms: docA.atoms.length },
};
result.cut = Math.round(100 * (1 - result.step_field.total / result.derive_then_edit.total)) / 100;
// the kill test of tp-11: the field must cut the actions by at least 30 %, else the key Shift+T goes and the field stays behind the context menu
result.kill_test = { bar: 0.3, passed: result.cut >= 0.3, decision: result.cut >= 0.3 ? "keep Shift+T" : "remove Shift+T, keep the field behind the context menu only" };
console.log(JSON.stringify(result, null, 1));
await browser.close(); server.close();
process.exit(oxygens === 4 && carbons === 28 ? 0 : 1);
