// Drawing-speed benchmark `velocity/1`: plays the hand-written input path of every task of tests/data/velocity/tasks.json in the page
// (Chromium through Playwright; mouse, touch and keyboard), counts the input actions and times the path up to the first moment the
// drawing is the target. The agent half replays the hand-written `draw_edit/1` patch of every task natively (no browser).
//
// usage: node tests/velocity.mjs > velocity.json     serves site/ (tools/build_site.sh); needs `cargo build -p mzdraw --example replay --example canon`
//        node tests/velocity.mjs --check             also fails when a task needs more actions than tests/data/velocity/baseline.json
//        options: --runs N (5)  --agent-runs N (20)  --only <task id>  --baseline <file>  --paths <file>  --url <published site>
// exit code: 0 every task reaches its target (and, with --check, no regression); 1 a task does not (its id on stderr) or an action
// count rose; 2 usage or a missing build. Data on stdout, diagnostics on stderr.
//
// Path steps (coordinates are pixels from the centre of the canvas at the default view; a bond is 40 px):
//   ["tool", NAME]        press a button of the toolbar (data-testid d2-NAME)          1 action
//   ["click", x, y]       press and release (a tap with touch)                         1 action
//   ["drag", x0, y0, x1, y1]   press, move, release                                    1 action
//   ["key", K]            one key on the canvas ("Enter", "Shift+Enter", "o", "Ctrl+z")  1 action
//   (a path may carry "set": ["section.key=value", ...]: settings applied before its first step, not counted as actions)
//   ["type", TEXT]        text typed into the field the last key opened, committed with Enter   1 action
// Times are wall-clock of the page (the harness adds a few ms per step); a snapshot of the drawing after every step is taken outside
// the timed window, and the time of a path is the sum of the timed windows up to the first step after which the drawing is the target.
import { createRequire } from "node:module";
import http from "node:http";
import { readFile } from "node:fs/promises";
import { readFileSync, existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const data = path.join(root, "tests/data/velocity");
const arg = (name, dflt) => { const i = process.argv.indexOf("--" + name); return i < 0 ? dflt : process.argv[i + 1]; };
const flag = name => process.argv.includes("--" + name);
const RUNS = Number(arg("runs", 5)), AGENT_RUNS = Number(arg("agent-runs", 20)), ONLY = arg("only", null);
const die = (code, msg) => { console.error(msg); process.exit(code); };

const replayBin = path.join(root, "target/debug/examples/replay"), canonBin = path.join(root, "target/debug/examples/canon");
for (const b of [replayBin, canonBin]) if (!existsSync(b)) die(2, `missing ${b}: run  cargo build --locked -p mzdraw --example replay --example canon`);
const native = (bin, input) => spawnSync(bin, { input, encoding: "utf8" });
const median = xs => { const s = [...xs].sort((a, b) => a - b), n = s.length; return n % 2 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2; };
const round = (x, d = 2) => Math.round(x * 10 ** d) / 10 ** d;

/** the identity of structure texts (mzlab-mol `Mol::can`: atom order and Kekulé form do not matter, stereo does), one per input; null when unreadable */
function identities(smiles) {
  const r = native(canonBin, smiles.join("\n") + "\n");
  const out = r.stdout.split("\n").slice(0, smiles.length);
  return out.map(x => (x === "ERR" || !x ? null : x));
}
/** a target as the editor holds it: opened from the target SMILES and written back (kekulised, laid out), then its identity */
function targetIdentity(t) {
  const r = native(replayBin, JSON.stringify({ import: t.target_smiles, out: "smiles" }));
  if (r.status !== 0) return null;
  return identities([r.stdout])[0];
}
/** the objects of a document as sorted "kind|text" strings; an anchored arrow adds "|i>j", the creation-order index of the structure
 *  (rank of its smallest atom id among the structures, a structure being a connected set of atoms) at its tail and at its head */
function objectsOf(doc) {
  const parent = new Map((doc.atoms || []).map(a => [a.id, a.id]));
  const find = x => { while (parent.get(x) !== x) { parent.set(x, parent.get(parent.get(x))); x = parent.get(x); } return x; };
  for (const b of doc.bonds || []) parent.set(find(b.a), find(b.b));
  const smallest = new Map();
  for (const a of doc.atoms || []) { const r = find(a.id); smallest.set(r, Math.min(smallest.get(r) ?? Infinity, a.id)); }
  const order = [...smallest.values()].sort((a, b) => a - b);
  const index = id => order.indexOf(smallest.get(find(id)));
  return (doc.objects || []).map(o => `${o.t}|${o.text || ""}` + (o.from || o.to ? `|${o.from ? index(o.from) : "-"}>${o.to ? index(o.to) : "-"}` : "")).sort();
}
const wantObjects = t => (t.objects || []).map(o => `${o.t}|${o.text || ""}` + (o.from !== undefined || o.to !== undefined ? `|${o.from ?? "-"}>${o.to ?? "-"}` : "")).sort();

const tasks = JSON.parse(readFileSync(path.join(data, "tasks.json"), "utf8")).tasks;
const paths = JSON.parse(readFileSync(arg("paths", path.join(data, "mzdraw.json")), "utf8")).tasks;
const agentPatches = JSON.parse(readFileSync(path.join(data, "agent.json"), "utf8")).tasks;
const selected = tasks.filter(t => !ONLY || t.id === ONLY);
if (!selected.length) die(2, `no task ${ONLY}`);

// `--guard-test`: the regression guard tested on itself with a doctored baseline in a temp dir (one action lower must fail, the true one must pass)
if (flag("guard-test")) {
  const { mkdtempSync, writeFileSync, rmSync } = await import("node:fs");
  const { tmpdir } = await import("node:os");
  const dir = mkdtempSync(path.join(tmpdir(), "velocity-"));
  const real = JSON.parse(readFileSync(path.join(data, "baseline.json"), "utf8"));
  const GUARD = "anthracene", have = real.rows.find(r => r.task === GUARD && !r.variant).actions;
  const doctored = { ...real, rows: real.rows.map(r => (r.task === GUARD && !r.variant ? { ...r, actions: have - 1 } : r)) };
  writeFileSync(path.join(dir, "low.json"), JSON.stringify(doctored)); writeFileSync(path.join(dir, "true.json"), JSON.stringify(real));
  const go = file => spawnSync(process.execPath, [fileURLToPath(import.meta.url), "--check", "--runs", "1", "--agent-runs", "1", "--only", "anthracene", "--baseline", path.join(dir, file)], { encoding: "utf8" });
  const low = go("low.json"), ok = go("true.json");
  rmSync(dir, { recursive: true, force: true });
  const good = low.status === 1 && low.stderr.includes(`regression: ${GUARD}: ${have} actions, baseline ${have - 1}`) && ok.status === 0;
  console.error(good ? "guard-test: ok (a baseline one action lower fails, the true baseline passes)" : `guard-test: FAILED (low ${low.status} ${low.stderr}; true ${ok.status} ${ok.stderr})`);
  process.exit(good ? 0 : 1);
}

const { chromium } = createRequire(import.meta.url)("playwright");
const TYPES = { ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".json": "application/json" };
let server = null, base = arg("url", null);
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

const sleep = ms => new Promise(r => setTimeout(r, ms));
async function until(pg, expr, ms = 10000) {
  for (const t0 = Date.now(); ;) {
    if (await pg.evaluate(expr)) return;
    if (Date.now() - t0 > ms) throw new Error("timeout: " + expr);
    await sleep(25);
  }
}
const KEYS = { Enter: "Enter", Escape: "Escape", Delete: "Delete" };
const press = (mods, k) => (mods + (KEYS[k] || k)).replace(/^\+/, "");

/** plays one step; `touch` sends touch events through the protocol, else the mouse */
async function play(pg, cdp, cx, cy, step, touch) {
  const [kind, ...a] = step;
  const touchEvent = (type, pts) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: pts.map(([id, x, y]) => ({ id, x, y })) });
  let extra = 0;
  if (kind === "tool") {
    const sel = `[data-testid="d2-${a[0]}"]`, press = async s => { if (touch) { const b = await pg.locator(s).boundingBox(); await pg.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2); } else await pg.click(s); };
    // a tool that is not in view is in the flyout of its palette: opening the flyout is one more action, and it is counted
    if (!(await pg.isVisible(sel))) {
      const opener = await pg.evaluate(s => { const f = document.querySelector(s)?.closest(".fly"); return f ? `[aria-controls="d2-fly-${f.dataset.fly}"]` : null; }, sel);
      if (!opener) throw new Error("a tool that no button reaches: " + a[0]);
      // the head of a collapsed palette first runs the kind last used, and opens the flyout when that kind is already the tool: each press counts
      for (extra = 1; ; extra++) {
        await press(opener);
        if (await pg.isVisible(sel)) break;
        if (extra >= 3) throw new Error("the flyout of " + a[0] + " does not open");
      }
    }
    await press(sel);
  }
  else if (kind === "focus") await pg.evaluate("document.getElementById('d2-canvas').focus()");
  else if (kind === "click") { if (touch) await pg.touchscreen.tap(cx + a[0], cy + a[1]); else await pg.mouse.click(cx + a[0], cy + a[1]); }
  else if (kind === "drag") {
    const [x0, y0, x1, y1] = [cx + a[0], cy + a[1], cx + a[2], cy + a[3]];
    if (touch) { await touchEvent("touchStart", [[1, x0, y0]]); await touchEvent("touchMove", [[1, (x0 + x1) / 2, (y0 + y1) / 2]]); await touchEvent("touchMove", [[1, x1, y1]]); await touchEvent("touchEnd", []); }
    else { await pg.mouse.move(x0, y0); await pg.mouse.down(); await pg.mouse.move((x0 + x1) / 2, (y0 + y1) / 2); await pg.mouse.move(x1, y1); await pg.mouse.up(); }
  }
  else if (kind === "key") { await pg.keyboard.press(a[0].includes("+") && a[0].length > 1 ? a[0] : press("", a[0])); }
  else if (kind === "type") { await pg.keyboard.type(a[0]); await pg.keyboard.press("Enter"); }
  else throw new Error("unknown step " + JSON.stringify(step));
  return extra;
}
const isTool = s => s[0] === "tool";

const browser = await chromium.launch();
const errs = [];
async function session(touch) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1, ...(touch ? { hasTouch: true } : {}) });
  const pg = await ctx.newPage();
  pg.on("pageerror", e => errs.push(String(e)));
  await pg.goto(base);
  await until(pg, "document.documentElement.dataset.d2Ready === '1'");
  const b = await pg.locator("#d2-canvas").boundingBox();
  const cdp = touch ? await ctx.newCDPSession(pg) : null;
  return { ctx, pg, cdp, cx: b.x + b.width / 2, cy: b.y + b.height / 2 };
}
const snap = pg => pg.evaluate("[window.MZDRAW.save('smiles'), window.MZDRAW.save('mzdraw')]");

/** one run of a path: the actions, the time up to the step after which the drawing is the target (null when it never is) */
async function run(task, want, touch) {
  const { ctx, pg, cdp, cx, cy } = await session(touch);
  const times = [], smiles = [], objects = [];
  let extra = 0;
  try {
    // settings a path needs before its first action (`"set": ["pathway.derive=on"]`): configuration, not an action, so it is not counted
    for (const setting of task.set || []) await pg.evaluate(x => /** @type {any} */ (window).MZDRAW.set(x), setting);
    for (const step of task.steps) {
      const t0 = performance.now();
      extra += await play(pg, cdp, cx, cy, step, touch);
      times.push(performance.now() - t0);
      const [s, d] = await snap(pg);
      smiles.push(s); objects.push(objectsOf(JSON.parse(d)));
    }
  } finally { await ctx.close(); }
  const ids = identities(smiles);
  const at = ids.findIndex((x, i) => x !== null && x === want.id && JSON.stringify(objects[i]) === JSON.stringify(want.objects));
  return { ms: at < 0 ? null : times.slice(0, at + 1).reduce((a, b) => a + b, 0), reached: at >= 0, final: smiles[smiles.length - 1], steps: at < 0 ? task.steps.length : at + 1, extra };
}

const rows = [], failed = [];
for (const t of selected) {
  const entry = paths[t.id];
  const want = { id: targetIdentity(t), objects: wantObjects(t) };
  // the main path, then its variants (touch, keyboard only): one row each
  const list = entry ? [[null, entry], ...Object.entries(entry.variants || {})] : [[null, null]];
  for (const [variant, task] of list) {
    const label = variant ? `${t.id}/${variant}` : t.id;
    if (!task || !want.id) { failed.push(label); rows.push({ task: t.id, ...(variant ? { variant } : {}), actions: task ? task.steps.length : 0, ms_median: null, ok: false }); console.error(`${label}: ${task ? "target not readable" : "no path"}`); continue; }
    const touch = task.pointer === "touch";
    const ms = []; let ok = true, last = null;
    for (let i = 0; i < RUNS; i++) { last = await run(task, want, touch); if (!last.reached) { ok = false; break; } ms.push(last.ms); }
    if (!ok) { failed.push(label); console.error(`${label}: the path does not reach the target (ends at ${last.final})`); }
    rows.push({ task: t.id, ...(variant ? { variant } : {}), pointer: variant === "keyboard" ? "keyboard" : task.pointer || "mouse", actions: task.steps.length + (last ? last.extra : 0), ms_median: ok ? round(median(ms), 1) : null, ok });
  }
}
await browser.close();
if (server) server.close();
if (errs.length) console.error("page errors:\n" + errs.join("\n"));

// the agent half: the patch of every task replayed natively, no browser
const agent = [];
for (const t of selected) {
  const patch = agentPatches[t.id], want = { id: targetIdentity(t), objects: wantObjects(t) };
  if (!patch || !want.id) { failed.push(t.id + " (agent)"); console.error(`${t.id}: no agent patch or target not readable`); continue; }
  const request = JSON.stringify({ log: patch, out: "smiles" }), requestDoc = JSON.stringify({ log: patch, out: "mzdraw" });
  const times = []; let r = null;
  for (let i = 0; i < AGENT_RUNS; i++) { const t0 = performance.now(); r = native(replayBin, request); times.push(performance.now() - t0); }
  const doc = native(replayBin, requestDoc);
  const ok = r.status === 0 && doc.status === 0 && identities([r.stdout])[0] === want.id && JSON.stringify(objectsOf(JSON.parse(doc.stdout))) === JSON.stringify(want.objects);
  if (!ok) { failed.push(t.id + " (agent)"); console.error(`${t.id}: the agent patch does not reach the target (${r.stdout} ${r.stderr})`); }
  agent.push({ task: t.id, ops: patch.length, bytes: Buffer.byteLength(JSON.stringify({ log: patch })), ms_median: round(median(times), 1), ok });
}

const commit = (() => { const r = spawnSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }); return r.status === 0 ? r.stdout.trim() : "unknown"; })();
const report = { format: "velocity/1", tool: "mzdraw", commit, rows, agent };
console.log(JSON.stringify(report, null, 1));

let regress = [];
if (flag("check")) {
  const base0 = JSON.parse(readFileSync(arg("baseline", path.join(data, "baseline.json")), "utf8"));
  for (const r of rows) {
    const b = base0.rows.find(x => x.task === r.task && x.variant === r.variant);
    if (!b) regress.push(`${r.task}${r.variant ? "/" + r.variant : ""}: not in the baseline`);
    else if (r.actions > b.actions) regress.push(`${r.task}${r.variant ? "/" + r.variant : ""}: ${r.actions} actions, baseline ${b.actions}`);
  }
  for (const m of regress) console.error("regression: " + m);
}
process.exit(failed.length || regress.length ? 1 : 0);
