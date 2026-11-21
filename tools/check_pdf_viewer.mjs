// Chromium's own PDF viewer opens the PDFs of the golden cases (tests/data/log/pdf.json): the file is served as application/pdf, the
// viewer (the full Chromium, `channel: "chromium"`; the headless shell has none and would download the file) shows it and does not
// download it, the page raises no error, and what it draws differs from what it draws for the same file damaged (an error page).
// usage: node tools/check_pdf_viewer.mjs          (playwright from NODE_PATH, as tests/e2e.mjs)
//        MZDRAW_SHOTS=<dir>                       also writes the screenshot of the viewer
import { createRequire } from "node:module";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import http from "node:http";
import path from "node:path";

const { chromium } = createRequire(import.meta.url)("playwright");
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const cases = JSON.parse(readFileSync(path.join(root, "tests/data/log/pdf.json"), "utf8")).pdf;
const files = cases.map(c => Buffer.from(c.out, "base64"));
const server = http.createServer((q, s) => { const i = Number(q.url.match(/(\d+)\.pdf/)?.[1] ?? 0); s.writeHead(200, { "content-type": "application/pdf" }); s.end(q.url.includes("bad") ? Buffer.concat([Buffer.from("%PDF-1.7\n"), files[i].subarray(40, 400)]) : files[i]); });
await new Promise(r => server.listen(0, "127.0.0.1", () => r(null)));
const base = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ channel: "chromium" });
let bad = 0;
const fail = what => { console.log("FAIL", what); bad = 1; };
try {
  for (const [i, c] of cases.entries()) {
    const shot = async url => {
      const pg = await browser.newPage({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 2 });
      const errs = []; let downloaded = false;
      pg.on("pageerror", e => errs.push(String(e))); pg.on("download", () => { downloaded = true; });
      const r = await pg.goto(base + url);
      await pg.waitForTimeout(1200);
      const out = { status: r.status(), downloaded, errs, png: await pg.screenshot() };
      await pg.close();
      return out;
    };
    const good = await shot(`/${i}.pdf`), damaged = await shot(`/bad/${i}.pdf`);
    if (good.status !== 200 || good.downloaded || good.errs.length) fail(`${c.name}: the viewer did not open the file (${JSON.stringify({ ...good, png: 0 })})`);
    else if (good.png.equals(damaged.png)) fail(`${c.name}: the viewer shows the same for a damaged file`);
    else console.log("ok  ", c.name);
    if (process.env.MZDRAW_SHOTS && i === 0) { mkdirSync(process.env.MZDRAW_SHOTS, { recursive: true }); writeFileSync(path.join(process.env.MZDRAW_SHOTS, "pdf-viewer.png"), good.png); }
  }
} finally { await browser.close(); server.close(); }
process.exit(bad);
