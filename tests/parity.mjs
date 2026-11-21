// Golden parity: the editor module (.wasm, target/pkg-node) replays every command log of tests/data/mzdraw_log.json to the same bytes as
// the native crate (its example `replay`) and as the recorded outputs (held natively by crates/mzdraw/tests/replay.rs too), the context menu of a right-click on each kind of target (`contexts`) and the settings file after `set` of the layout and export defaults (`settings`); a refused
// request is the same keyed JSON error on both sides.
// Run after tools/build_site.sh and `cargo build -p mzdraw --example replay`:  node tests/parity.mjs
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const wasm = createRequire(import.meta.url)(path.join(root, "target/pkg-node/mzdraw_wasm.js"));
const fx = JSON.parse(readFileSync(path.join(root, "tests/data/mzdraw_log.json"), "utf8"));
const native = request => spawnSync(path.join(root, "target/debug/examples/replay"), { input: request, encoding: "utf8" });
const site = request => { try { return { status: 0, stdout: wasm.replay(request), stderr: "" }; } catch (e) { return { status: 1, stdout: "", stderr: String(e) }; } };

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
// the settings file after layout and export defaults were set through the route of Editor::set (`set`, the --set of the CLI)
for (const c of fx.settings) {
  const request = JSON.stringify(c.request), a = site(request), b = native(request);
  check(a.status === 0 && b.status === 0, `${c.name}: runs on both sides (${a.stderr} ${b.stderr})`);
  check(a.stdout === b.stdout, `${c.name}: wasm == native`);
  const file = JSON.parse(c.out);
  check(a.stdout === c.out && file.layout && file.export, `${c.name}: the recorded settings`);
}
for (const c of fx.refused) {
  const request = JSON.stringify(c.request), a = site(request), b = native(request);
  check(a.status === 1 && b.status === 1, `${c.error_key}: refused on both sides`);
  check(a.stderr === b.stderr && JSON.parse(a.stderr || "{}").error_key === c.error_key, `${c.error_key}: the same keyed error`);
}
check(fx.cases.length >= 3 && fx.refused.length >= 1 && fx.contexts.length >= 7 && fx.settings.length >= 2, "the fixture has cases");
console.log(`parity: ${n - bad}/${n} checks ok`);
process.exit(bad ? 1 : 0);
