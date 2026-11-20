// Golden parity: the editor module (.wasm, target/pkg-node) replays every command log of tests/data/mzdraw_log.json to the same bytes as
// the native crate (its example `replay`) and as the recorded outputs (held natively by crates/mzdraw/tests/replay.rs too); a refused
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
for (const c of fx.cases) for (const out of ["mzdraw", "molfile", "svg", "log"]) {
  const request = JSON.stringify({ ...c.request, out }), a = site(request), b = native(request);
  check(a.status === 0 && b.status === 0, `${c.name} as ${out}: runs on both sides (${a.stderr} ${b.stderr} ${b.error || ""})`);
  check(a.stdout === b.stdout, `${c.name} as ${out}: wasm == native`);
  check(a.stdout === c.out[out], `${c.name} as ${out}: the recorded bytes`);
}
for (const c of fx.refused) {
  const request = JSON.stringify(c.request), a = site(request), b = native(request);
  check(a.status === 1 && b.status === 1, `${c.error_key}: refused on both sides`);
  check(a.stderr === b.stderr && JSON.parse(a.stderr || "{}").error_key === c.error_key, `${c.error_key}: the same keyed error`);
}
check(fx.cases.length >= 3 && fx.refused.length >= 1, "the fixture has cases");
console.log(`parity: ${n - bad}/${n} checks ok`);
process.exit(bad ? 1 : 0);
