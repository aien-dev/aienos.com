/* Drives headless Chrome over its debugging port with node's built-in WebSocket. No packages. Copied from the Allen phone app tests.
 * Exports launch(): returns {open(url, {w,h,dpr,mobile}), shot(file, full), eval(js), logs, close()}. */
"use strict";
const { spawn } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");
const http = require("http");

function getJSON(url) {
  return new Promise((resolve, reject) => {
    http.get(url, (r) => { let d = ""; r.on("data", (c) => (d += c)); r.on("end", () => { try { resolve(JSON.parse(d)); } catch (e) { reject(e); } }); }).on("error", reject);
  });
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function launch(port = 9333) {
  const dir = fs.mkdtempSync(path.join(process.env.CLAUDE_JOB_DIR || os.tmpdir(), "chrome-"));
  const proc = spawn(process.env.CHROME_BIN || "google-chrome", ["--headless=new", "--no-sandbox", "--hide-scrollbars", "--disable-gpu", "--disable-gpu-sandbox", "--disable-background-networking", "--disable-sync", "--no-first-run",
    "--remote-debugging-port=" + port, "--user-data-dir=" + dir, "--autoplay-policy=no-user-gesture-required", "about:blank"], { stdio: "ignore" });
  let ver;
  for (let i = 0; i < 60; i++) {
    try { ver = await getJSON("http://127.0.0.1:" + port + "/json/version"); break; } catch (e) { /* not up yet */ }
    await sleep(250);
  }
  const logs = [];
  let id = 0, ws = null, pageId = null;
  const pending = new Map();
  const waiters = [];
  const browser = new WebSocket(ver.webSocketDebuggerUrl);
  await new Promise((r, j) => { browser.onopen = r; browser.onerror = j; });
  browser.onmessage = (m) => { const msg = JSON.parse(m.data); if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg); pending.delete(msg.id); } };
  const onPage = (m) => {
    const msg = JSON.parse(m.data);
    if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg); pending.delete(msg.id); return; }
    if (msg.method === "Runtime.consoleAPICalled") logs.push(msg.params.type + ": " + msg.params.args.map((a) => a.value ?? a.description ?? "").join(" "));
    if (msg.method === "Runtime.exceptionThrown") logs.push("EXCEPTION: " + (msg.params.exceptionDetails.exception?.description || msg.params.exceptionDetails.text));
    if (msg.method === "Log.entryAdded") logs.push("log " + msg.params.entry.level + ": " + msg.params.entry.text + " " + (msg.params.entry.url || ""));
    for (const w of waiters.slice()) if (w.method === msg.method) { waiters.splice(waiters.indexOf(w), 1); w.resolve(msg.params); }
  };
  const raw = (sock, method, params = {}) => new Promise((resolve, reject) => {
    const my = ++id;
    pending.set(my, (msg) => (msg.error ? reject(new Error(method + ": " + msg.error.message)) : resolve(msg.result)));
    sock.send(JSON.stringify({ id: my, method, params }));
  });
  const send = (method, params) => raw(ws, method, params);
  const waitFor = (method) => new Promise((resolve) => waiters.push({ method, resolve }));
  /* a fresh tab for every page: navigating the first tab of a headless run hangs on this machine */
  async function freshTab() {
    if (ws) { try { ws.close(); } catch (e) { /* gone */ } await raw(browser, "Target.closeTarget", { targetId: pageId }).catch(() => {}); }
    const t = await raw(browser, "Target.createTarget", { url: "about:blank" });
    pageId = t.targetId;
    ws = new WebSocket("ws://127.0.0.1:" + port + "/devtools/page/" + pageId);
    await new Promise((r, j) => { ws.onopen = r; ws.onerror = j; });
    ws.onmessage = onPage;
    await send("Page.enable"); await send("Runtime.enable"); await send("Log.enable");
  }

  return {
    logs,
    send: (m, p) => send(m, p),
    async open(url, { w = 390, h = 844, dpr = 3, mobile = true } = {}) {
      await freshTab();
      await send("Emulation.setDeviceMetricsOverride", { width: w, height: h, deviceScaleFactor: dpr, mobile, screenWidth: w, screenHeight: h });
      if (mobile) {
        await send("Emulation.setUserAgentOverride", { userAgent: "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/153.0.0.0 Mobile Safari/537.36" });
        await send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 });
      }
      const loaded = waitFor("Page.loadEventFired");
      await send("Page.navigate", { url });
      await loaded;
      await sleep(300);
    },
    /* loads a page by writing its HTML into a blank tab (no navigation, which hangs on this machine);
       query stands in for location.search */
    async openInline(html, query, { w = 390, h = 844, dpr = 3, mobile = true } = {}) {
      await freshTab();
      await send("Emulation.setDeviceMetricsOverride", { width: w, height: h, deviceScaleFactor: dpr, mobile, screenWidth: w, screenHeight: h });
      if (mobile) {
        await send("Emulation.setUserAgentOverride", { userAgent: "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/153.0.0.0 Mobile Safari/537.36" });
        await send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 });
      }
      const tree = await send("Page.getFrameTree");
      await send("Page.setDocumentContent", { frameId: tree.frameTree.frame.id, html: html.replace("<head>", "<head><script>window.__q=" + JSON.stringify(query) + "</script>") });
      await sleep(600);
    },
    async eval(js) {
      const r = await send("Runtime.evaluate", { expression: js, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error("eval: " + (r.exceptionDetails.exception?.description || r.exceptionDetails.text));
      return r.result.value;
    },
    async shot(file, full) {
      await send("Page.bringToFront");
      if (full) {
        const m = await send("Runtime.evaluate", { expression: "[document.documentElement.scrollHeight, window.innerWidth, window.devicePixelRatio, /Mobi/.test(navigator.userAgent)]", returnByValue: true });
        const [sh, iw, dpr, mob] = m.result.value;
        await send("Emulation.setDeviceMetricsOverride", { width: iw, height: sh, deviceScaleFactor: dpr, mobile: mob, screenWidth: iw, screenHeight: sh });
        await sleep(400);
      }
      const r = await send("Page.captureScreenshot", { format: "png", fromSurface: false, captureBeyondViewport: !!full });
      fs.writeFileSync(file, Buffer.from(r.data, "base64"));
    },
    async tap(x, y) {
      for (const type of ["mousePressed", "mouseReleased"]) await send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1 });
    },
    async type(text) { await send("Input.insertText", { text }); },
    async key(k) {
      await send("Input.dispatchKeyEvent", { type: "keyDown", key: k, code: k, windowsVirtualKeyCode: k === "Enter" ? 13 : 27, text: k === "Enter" ? "\r" : undefined });
      await send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: k, windowsVirtualKeyCode: k === "Enter" ? 13 : 27 });
    },
    sleep,
    async close() { try { ws && ws.close(); browser.close(); } catch (e) { /* done */ } proc.kill(); await sleep(300); fs.rmSync(dir, { recursive: true, force: true }); }
  };
}
module.exports = { launch, sleep };
