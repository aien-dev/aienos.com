'use strict';
/* Runs the compiled Omega kernel off the main thread. The page can stop this
   worker at any time, so a kernel that loops for a long time never freezes the tab.
   The worker only executes wasm it fetched itself: the prebuilt /lab/axpy.wasm or
   the reply of the compile gateway. Messages carry Omega source text and numbers,
   never code. The wasm has no imports: it cannot touch the page, the network or storage. */
const API = '/aien/api/omega';
const BLOCK = 999999;
let kernel = null;
function median(xs) { const s = xs.slice().sort((p, q) => p - q); return s[(s.length - 1) >> 1]; }
async function install(bytes) {
  const module = await WebAssembly.compile(bytes);
  const imports = WebAssembly.Module.imports(module).length;
  if (imports) throw new Error(`This module asks for ${imports} imports; the lab runs only self-contained kernels.`);
  kernel = await WebAssembly.instantiate(module, {});
  return { type: 'loaded', bytes, imports, exports: WebAssembly.Module.exports(module).map((e) => e.name) };
}
async function compile(source) {
  const t = performance.now();
  let reply;
  try {
    const response = await fetch(API, { method: 'POST', credentials: 'omit', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ source }) });
    reply = (response.headers.get('content-type') || '').includes('json') ? await response.json() : { ok: false, message: `HTTP ${response.status}` };
  } catch (err) {
    reply = { ok: false, message: `The compile service is unreachable (${err.message}). The prebuilt kernel still runs.` };
  }
  const trip = performance.now() - t;
  if (!reply.ok) return { type: 'refused', trip, stage: reply.stage, code: reply.code, line: reply.line, message: reply.message, hint: reply.hint };
  const loaded = await install(Uint8Array.from(atob(reply.wasm), (c) => c.charCodeAt(0)));
  return Object.assign(loaded, { trip, compileMs: reply.compile_ms, omc: reply.omc_sha256, irwasm: reply.irwasm_sha256 });
}
function run(a, n) {
  if (!kernel || typeof kernel.exports.run !== 'function') {
    return { type: 'error', message: 'This module has no run export. Keep fn run a long lo long hi long to long.' };
  }
  const go = kernel.exports.run;
  const big = BigInt(a);
  const wasmMs = [];
  const jsMs = [];
  let sum = 0n;
  let ref = 0;
  for (let rep = 0; rep < 3; rep++) {
    let t = performance.now();
    let s = 0n;
    for (let lo = 0; lo < n; lo += BLOCK) s += go(big, BigInt(lo), BigInt(Math.min(n, lo + BLOCK)));
    wasmMs.push(performance.now() - t);
    sum = s;
    t = performance.now();
    let r = 0;
    for (let i = 0; i < n; i++) r += a * i + (2 * i + 1);
    jsMs.push(performance.now() - t);
    ref = r;
  }
  return { type: 'ran', sum: sum.toString(), ref: BigInt(ref).toString(), wasmMs, jsMs, wasm: median(wasmMs), js: median(jsMs) };
}
self.onmessage = async (event) => {
  const job = event.data || {};
  try {
    if (job.type === 'prebuilt') self.postMessage(await install(new Uint8Array(await (await fetch('/lab/axpy.wasm')).arrayBuffer())));
    else if (job.type === 'compile') self.postMessage(await compile(String(job.source)));
    else if (job.type === 'run') self.postMessage(run(Math.max(1, Math.min(1000, Math.round(Number(job.a)) || 3)), Math.max(1, Math.min(4194304, Math.round(Number(job.n)) || 1))));
  } catch (err) {
    const message = String((err && err.message) || err);
    self.postMessage({ type: 'error', message: err instanceof WebAssembly.RuntimeError ? `The kernel trapped: ${message}. Omega stops on integer overflow instead of wrapping.` : message });
  }
};
