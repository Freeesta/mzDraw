// Runs the velocity tasks on a pinned standalone Ketcher (EPAM, Apache-2.0), outside CI. Nothing of Ketcher is committed: the release zip
// is fetched at run time into a new directory under the OS temp dir, its SHA-256 is checked, and it is served locally and driven by Playwright.
// Pinned release: v3.21.0, https://github.com/epam/ketcher/releases/download/v3.21.0/ketcher-standalone-3.21.0.zip (published 2026-11-18)
//   sha256 215f5ac6cd280342df3ba3a4f8347e9b72c40ec5cc1e1e9fc2751de5b3e5c73c   (the file fetched on 2026-11-21)
// usage: node tools/velocity/ketcher.mjs > ketcher-velocity.json      (needs curl and unzip; same options as tests/velocity.mjs: --runs N --only ID)
// Paths: tests/data/velocity/ketcher.json, same steps as tests/velocity.mjs except that `tool` takes a `data-testid` of Ketcher's own UI
// (for example "bond-single", "clear-canvas") and the structure is read with `window.ketcher.getSmiles()`. Exit 0 when every task reaches
// its target, 1 when one does not or has no path, 2 on a fetch or build problem.
import { createRequire } from "node:module";
import http from "node:http";
import crypto from "node:crypto";
import { readFile, mkdtemp } from "node:fs/promises";
import { readFileSync, existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import path from "node:path";

const VERSION = "3.21.0", SHA256 = "215f5ac6cd280342df3ba3a4f8347e9b72c40ec5cc1e1e9fc2751de5b3e5c73c";
const URL_ZIP = `https://github.com/epam/ketcher/releases/download/v${VERSION}/ketcher-standalone-${VERSION}.zip`;
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const data = path.join(root, "tests/data/velocity");
const arg = (n, d) => { const i = process.argv.indexOf("--" + n); return i < 0 ? d : process.argv[i + 1]; };
const RUNS = Number(arg("runs", 5)), ONLY = arg("only", null);
const die = (c, m) => { console.error(m); process.exit(c); };
const canonBin = path.join(root, "target/debug/examples/canon"), replayBin = path.join(root, "target/debug/examples/replay");
for (const b of [canonBin, replayBin]) if (!existsSync(b)) die(2, `missing ${b}: run  cargo build --locked -p mzdraw --example replay --example canon`);
const native = (bin, input) => spawnSync(bin, { input, encoding: "utf8" });
const identity = s => { const o = native(canonBin, s + "\n").stdout.trim(); return o && o !== "ERR" ? o : null; };
const targetIdentity = t => { const r = native(replayBin, JSON.stringify({ import: t.target_smiles, out: "smiles" })); return r.status === 0 ? identity(r.stdout) : null; };
const median = xs => { const s = [...xs].sort((a, b) => a - b), n = s.length; return n % 2 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2; };

// fetch into a new, empty directory, check the hash, unpack there (a downloaded archive is untrusted data)
const dir = await mkdtemp(path.join(tmpdir(), "ketcher-"));
const zip = path.join(dir, "ketcher.zip"), site = path.join(dir, "site");
if (spawnSync("curl", ["-sSfL", "-m", "300", "-o", zip, URL_ZIP]).status !== 0) die(2, `cannot fetch ${URL_ZIP}`);
const sum = crypto.createHash("sha256").update(readFileSync(zip)).digest("hex");
if (sum !== SHA256) die(2, `the release zip has sha256 ${sum}, not the pinned ${SHA256}`);
if (spawnSync("unzip", ["-q", zip, "-d", site]).status !== 0) die(2, "cannot unzip");

const tasks = JSON.parse(readFileSync(path.join(data, "tasks.json"), "utf8")).tasks.filter(t => !ONLY || t.id === ONLY);
const paths = JSON.parse(readFileSync(path.join(data, "ketcher.json"), "utf8")).tasks;
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".json": "application/json", ".svg": "image/svg+xml", ".png": "image/png" };
const server = http.createServer(async (q, s) => {
  let p = decodeURIComponent(new URL(q.url || "/", "http://x").pathname); if (p.endsWith("/")) p += "index.html";
  try { const b = await readFile(path.join(site, path.normalize(p))); s.writeHead(200, { "content-type": TYPES[path.extname(p)] || "application/octet-stream" }); s.end(b); } catch { s.writeHead(404); s.end(); }
});
await new Promise(r => server.listen(0, "127.0.0.1", () => r(null)));
const base = `http://127.0.0.1:${server.address().port}/`;
const { chromium } = createRequire(import.meta.url)("playwright");
const browser = await chromium.launch();
const sleep = ms => new Promise(r => setTimeout(r, ms));

async function run(task, want) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, ...(task.pointer === "touch" ? { hasTouch: true } : {}) }), pg = await ctx.newPage();
  await pg.goto(base);
  for (let i = 0; i < 200 && !(await pg.evaluate("typeof window.ketcher !== 'undefined'")); i++) await sleep(100);
  const b = await pg.evaluate("(() => { const r = document.querySelector('[data-testid=ketcher-canvas]')?.getBoundingClientRect() || document.querySelector('canvas,svg').getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; })()");
  const cx = b[0] + b[2] / 2, cy = b[1] + b[3] / 2, times = [], smiles = [];
  try {
    for (const [kind, ...a] of task.steps) {
      const t0 = performance.now();
      if (kind === "tool") await pg.click(`[data-testid="${a[0]}"]`);
      else if (kind === "click") await pg.mouse.click(cx + a[0], cy + a[1]);
      else if (kind === "drag") { await pg.mouse.move(cx + a[0], cy + a[1]); await pg.mouse.down(); await pg.mouse.move(cx + (a[0] + a[2]) / 2, cy + (a[1] + a[3]) / 2); await pg.mouse.move(cx + a[2], cy + a[3]); await pg.mouse.up(); }
      else if (kind === "key") await pg.keyboard.press(a[0]);
      else if (kind === "type") { await pg.keyboard.type(a[0]); await pg.keyboard.press("Enter"); }
      else throw new Error("unknown step " + kind);
      times.push(performance.now() - t0);
      smiles.push(await pg.evaluate("window.ketcher.getSmiles()"));
    }
  } finally { await ctx.close(); }
  const at = smiles.findIndex(s => s && identity(s) === want);
  return { ms: at < 0 ? null : times.slice(0, at + 1).reduce((x, y) => x + y, 0), final: smiles[smiles.length - 1] };
}

const rows = [], failed = [];
for (const t of tasks) {
  const task = paths[t.id], want = targetIdentity(t);
  if (!task || !want) { failed.push(t.id); rows.push({ task: t.id, actions: 0, ms_median: null, ok: false }); console.error(`${t.id}: ${task ? "target not readable" : "no Ketcher path written"}`); continue; }
  const ms = []; let ok = true;
  for (let i = 0; i < RUNS && ok; i++) { const r = await run(task, want); if (r.ms === null) { ok = false; console.error(`${t.id}: does not reach the target (ends at ${r.final})`); } else ms.push(r.ms); }
  if (!ok) failed.push(t.id);
  rows.push({ task: t.id, actions: task.steps.length, ms_median: ok ? Math.round(median(ms) * 10) / 10 : null, ok });
}
await browser.close(); server.close();
console.log(JSON.stringify({ format: "velocity/1", tool: "ketcher", version: VERSION, source: URL_ZIP, sha256: SHA256, rows }, null, 1));
process.exit(failed.length ? 1 : 0);
