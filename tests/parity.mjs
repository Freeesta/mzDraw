// Golden parity: the editor module (.wasm, target/pkg-node) replays every command log of the files of tests/data/log/ (one per area, read in the order of their names) to the same bytes as
// the native crate (its example `replay`) and as the recorded outputs (held natively by crates/mzdraw/tests/replay.rs too), the context menu of a right-click on each kind of target (`contexts`) and the settings file after `set` of the layout and export defaults (`settings`); a refused
// request is the same keyed JSON error on both sides.
// Run after tools/build_site.sh and `cargo build -p mzdraw --example replay`:  node tests/parity.mjs
import { createRequire } from "node:module";
import { readFileSync, readdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const wasm = createRequire(import.meta.url)(path.join(root, "target/pkg-node/mzdraw_wasm.js"));
// the golden files: every *.json of tests/data/log/, sorted by name, each with some of the sections below (as crates/mzdraw/tests/common reads them)
const logDir = path.join(root, "tests/data/log");
const fx = { cases: [], contexts: [], refused: [], results: [], schemas: [], settings: [] };
for (const f of readdirSync(logDir).filter(f => f.endsWith(".json")).sort()) {
  const part = JSON.parse(readFileSync(path.join(logDir, f), "utf8"));
  for (const k of Object.keys(part)) {
    if (!(k in fx)) throw new Error(`${f}: unknown section ${k}`);
    fx[k].push(...part[k]);
  }
}
const native = request => spawnSync(path.join(root, "target/debug/examples/replay"), { input: request, encoding: "utf8" });
const site = request => { try { return { status: 0, stdout: wasm.replay(request), stderr: "" }; } catch (e) { return { status: 1, stdout: "", stderr: String(e) }; } };

// the engines of mzLab (src/glue.rs): the example `replay_glue` is built here when the command above did not build it
const glueBin = path.join(root, "target/debug/examples/replay_glue");
spawnSync("cargo", ["build", "--locked", "-q", "-p", "mzdraw-wasm", "--example", "replay_glue"], { cwd: root, stdio: "inherit" });
const nativeGlue = request => spawnSync(glueBin, { input: request, encoding: "utf8" });
const siteGlue = request => { try { return { status: 0, stdout: wasm.replay_glue(request), stderr: "" }; } catch (e) { return { status: e.includes?.("engineMissing") ? 2 : 1, stdout: "", stderr: String(e) }; } };

let n = 0, bad = 0;
const check = (ok, what) => { n++; if (!ok) { bad++; console.error("FAIL " + what); } };
// every recorded output of every case (a reaction file is recorded only for the drawings that are a reaction)
for (const c of fx.cases) for (const out of Object.keys(c.out)) {
  const request = JSON.stringify({ ...c.request, out }), a = site(request), b = native(request);
  check(a.status === 0 && b.status === 0, `${c.name} as ${out}: runs on both sides (${a.stderr} ${b.stderr} ${b.error || ""})`);
  check(a.stdout === b.stdout, `${c.name} as ${out}: wasm == native`);
  check(a.stdout === c.out[out], `${c.name} as ${out}: the recorded bytes`);
}
// the context menu (JSON) of a right-click on each kind of target
for (const c of fx.contexts) {
  const request = JSON.stringify(c.request), a = site(request), b = native(request);
  check(a.status === 0 && b.status === 0, `${c.name}: runs on both sides (${a.stderr} ${b.stderr})`);
  check(a.stdout === b.stdout, `${c.name}: wasm == native`);
  check(a.stdout === c.out && JSON.parse(c.out).items.length > 0, `${c.name}: the recorded menu`);
}
// the table of the operations (`draw-edit-schema/1`)
for (const c of fx.schemas) {
  const request = JSON.stringify(c.request), a = site(request), b = native(request);
  check(a.status === 0 && b.status === 0, `${c.name}: runs on both sides (${a.stderr} ${b.stderr})`);
  check(a.stdout === b.stdout, `${c.name}: wasm == native`);
  check(a.stdout === c.out && JSON.parse(c.out).format === "draw-edit-schema/1", `${c.name}: the recorded table`);
}
// the settings file after layout and export defaults were set through the route of Editor::set (`set`, the --set of the CLI)
for (const c of fx.settings) {
  const request = JSON.stringify(c.request), a = site(request), b = native(request);
  check(a.status === 0 && b.status === 0, `${c.name}: runs on both sides (${a.stderr} ${b.stderr})`);
  check(a.stdout === b.stdout, `${c.name}: wasm == native`);
  const file = JSON.parse(c.out);
  check(a.stdout === c.out && file.layout && file.export, `${c.name}: the recorded settings`);
}
// the verdict of a request that asks for one (`draw-result/1`): the same bytes on both sides, exit 0 when `ok` and 1 when not; a request number `n` of a batch is the `id` of an answer without one
for (const c of fx.results) {
  const request = JSON.stringify(c.request), n = c.n ?? 0;
  const a = site(request), b = spawnSync(path.join(root, "target/debug/examples/replay"), { input: request, encoding: "utf8", env: { ...process.env, MZDRAW_REQUEST_N: String(n) } });
  const want = JSON.parse(c.out);
  check(a.status === 0 && b.status === (want.ok ? 0 : 1), `${c.name}: exit codes (wasm ${a.status}, native ${b.status})`);
  check(want.format === "draw-result/1", `${c.name}: a draw-result/1 document`);
  // the .wasm has no batch: its answer carries the id of the request or 0, so only a request with an id or n = 0 is held to the recorded bytes there
  if (c.request.id !== undefined || n === 0) check(a.stdout === c.out, `${c.name}: wasm == the recorded bytes`);
  check(b.stdout === c.out, `${c.name}: native == the recorded bytes`);
}
for (const c of fx.refused) {
  const request = JSON.stringify(c.request), a = site(request), b = native(request);
  check(a.status === 1 && b.status === 1, `${c.error_key}: refused on both sides`);
  check(a.stderr === b.stderr && JSON.parse(a.stderr || "{}").error_key === c.error_key, `${c.error_key}: the same keyed error`);
}
// each engine on a drawing whose ids are not 0..n (a ring with three atoms deleted, a chain O-S-Cl added): wasm == native, bytes
const drawn = [{ op: "ring", n: 6, arom: false, x: 0, y: 0 }, { op: "remove", atoms: [2, 3, 5] }, { op: "atom.add", el: "O", x: 3, y: 0 },
  { op: "atom.add", el: "S", x: 4, y: 1 }, { op: "atom.add", el: "Cl", x: 5, y: 2 }, { op: "bond.add", a: 13, b: 14 }, { op: "bond.add", a: 14, b: 15 }];
const engines = [["check", {}, "check/1"], ["cip", {}, "cip/1"], ["cut", { bonds: [[3, 4]] }, "cut/1"], ["mech", { arrows: [] }, "mechanism/1"],
  ["hilite", { layers: [] }, "highlight/1"], ["sel", { part: false }, "selection/1"]];
for (const [engine, args, schema] of engines) {
  const request = JSON.stringify({ log: drawn, engines: [{ engine, args }] }), a = siteGlue(request), b = nativeGlue(request);
  check(a.status === 0 && b.status === 0, `engine ${engine}: runs on both sides (${a.stderr} ${b.stderr})`);
  check(a.stdout === b.stdout, `engine ${engine}: wasm == native`);
  check(JSON.parse(a.stdout || "{}").engines?.[0]?.answer?.schema === schema, `engine ${engine}: answers ${schema}`);
}
{ // several engines in one request keep the request order; a missing engine is the same keyed refusal, exit 2, on both sides
  const request = JSON.stringify({ log: drawn, engines: engines.map(([engine, args]) => ({ engine, args })) }), a = siteGlue(request), b = nativeGlue(request);
  check(a.stdout === b.stdout && JSON.parse(a.stdout).engines.map(e => e.engine).join() === engines.map(e => e[0]).join(), "engines: one request, request order, wasm == native");
  const missing = JSON.stringify({ log: drawn, engines: [{ engine: "nope" }] }), c = siteGlue(missing), d = nativeGlue(missing);
  check(c.status === 2 && d.status === 2 && c.stderr === d.stderr && JSON.parse(d.stderr).error_key === "error.draw2.engineMissing", "engine missing: same keyed refusal, exit 2");
}
check(fx.cases.length >= 3 && fx.refused.length >= 1 && fx.contexts.length >= 7 && fx.settings.length >= 2, "the fixture has cases");
console.log(`parity: ${n - bad}/${n} checks ok`);
process.exit(bad ? 1 : 0);
