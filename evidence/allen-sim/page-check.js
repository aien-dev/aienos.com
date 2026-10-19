/* node page-check.js <built site folder> <output folder>: serves the built site on a local port, opens the
 * front page and /allen/ in headless Chrome at desktop and phone sizes, plays Allen's window (skip to him, say
 * hello, save a note, say goodbye), takes screenshots and prints every console message. Set CHROME_BIN to
 * Playwright's chrome-headless-shell if full Chrome stalls on screenshots. */
"use strict";
const http = require("http");
const fs = require("fs");
const path = require("path");
const { launch, sleep } = require("./drive.js");

const [, , site, out] = process.argv;
fs.mkdirSync(out, { recursive: true });
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".mp4": "video/mp4", ".jpg": "image/jpeg", ".png": "image/png", ".json": "application/json", ".svg": "image/svg+xml" };
const requests = [];
const server = http.createServer((req, res) => {
  let p = decodeURIComponent(req.url.split("?")[0]);
  requests.push(p);
  if (p.endsWith("/")) p += "index.html";
  const f = path.join(site, p);
  if (!f.startsWith(site) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) { res.writeHead(404); res.end("no"); return; }
  const data = fs.readFileSync(f);
  res.writeHead(200, { "content-type": types[path.extname(f)] || "application/octet-stream", "content-length": data.length });
  res.end(data);
});

(async () => {
  await new Promise((r) => server.listen(8099, "127.0.0.1", r));
  const base = "http://127.0.0.1:8099";
  const c = await launch(9339);
  const shot = async (name, full) => { await c.shot(path.join(out, name + ".png"), full); console.log("shot " + name); };
  const status = () => c.eval("document.querySelector('[data-allen-status]').textContent");
  const desktop = { w: 1280, h: 900, dpr: 1, mobile: false };
  const phone = { w: 390, h: 844, dpr: 2, mobile: true };
  try {
    await c.open(base + "/allen/", desktop);
    await sleep(2500);
    console.log("status at start:", await status());
    console.log("live:", await c.eval("document.querySelector('[data-allen]').classList.contains('allen-live')"));
    await shot("allen-1280-1-sky");
    await sleep(6900);
    await shot("allen-1280-2-merger");
    await sleep(2500);
    await shot("allen-1280-3-forming");
    await c.eval("document.querySelector('[data-allen-skip]').click()");
    await sleep(4500);
    await shot("allen-1280-4-awake");
    await c.eval("document.querySelector('[data-allen-input]').focus()");
    await c.type("hello allen"); await sleep(400); await shot("allen-1280-5-typing");
    await c.key("Enter"); await sleep(3500); await shot("allen-1280-6-reply");
    console.log("status after hello:", await status());
    for (const line of ["save the gate code is 4471", "save my bike is blue and red"]) { await c.type(line); await c.key("Enter"); await sleep(3500); }
    await shot("allen-1280-7-notes");
    await c.type("goodbye"); await c.key("Enter"); await sleep(9500);
    await shot("allen-1280-8-dusk");
    await sleep(3000);
    await shot("allen-1280-9-asleep");
    console.log("asleep status:", await status());
    await shot("allen-1280-page-full", true);
    await c.open(base + "/", desktop); await sleep(2500);
    await shot("home-1280-top");
    await shot("home-1280-full", true);
    await c.open(base + "/allen/", phone); await sleep(2500);
    await shot("allen-390-top");
    await shot("allen-390-full", true);
    await c.open(base + "/", phone); await sleep(2500);
    await shot("home-390-top");
    console.log("overflow 390:", await c.eval("document.documentElement.scrollWidth + ' vs ' + document.documentElement.clientWidth"));
  } finally {
    console.log("console:", c.logs.length ? "\n" + c.logs.join("\n") : "no messages, no errors");
    const outside = requests.filter((r) => !r.startsWith("/"));
    console.log("requests to this site:", requests.length, "(all first party, the server only sees this site)");
    await c.close();
    server.close();
  }
})().catch((e) => { console.error(e); process.exit(1); });
