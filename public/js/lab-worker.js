'use strict';
/* Runs the compiled Omega kernel off the main thread. The page can stop this
   worker at any time, so a kernel that loops for a long time never freezes the tab.
   The wasm has no imports: it cannot touch the page, the network or storage. */
let kernel = null;
const BLOCK = 999999;
function median(xs) { const s = xs.slice().sort((p, q) => p - q); return s[(s.length - 1) >> 1]; }
self.onmessage = async (event) => {
  const job = event.data;
  try {
    if (job.type === 'load') {
      const module = await WebAssembly.compile(job.bytes);
      const exports = WebAssembly.Module.exports(module).map((e) => e.name);
      const imports = WebAssembly.Module.imports(module).length;
      kernel = await WebAssembly.instantiate(module, {});
      self.postMessage({ type: 'loaded', exports, imports });
      return;
    }
    if (job.type !== 'run') return;
    if (!kernel || typeof kernel.exports.run !== 'function') {
      self.postMessage({ type: 'error', message: 'This module has no run export. Keep fn run a long lo long hi long to long.' });
      return;
    }
    const run = kernel.exports.run;
    const a = BigInt(job.a);
    const n = job.n;
    const wasmMs = [];
    const jsMs = [];
    let sum = 0n;
    let ref = 0;
    for (let rep = 0; rep < 3; rep++) {
      let t = performance.now();
      let s = 0n;
      for (let lo = 0; lo < n; lo += BLOCK) s += run(a, BigInt(lo), BigInt(Math.min(n, lo + BLOCK)));
      wasmMs.push(performance.now() - t);
      sum = s;
      t = performance.now();
      let r = 0;
      for (let i = 0; i < n; i++) r += job.a * i + (2 * i + 1);
      jsMs.push(performance.now() - t);
      ref = r;
    }
    self.postMessage({ type: 'ran', sum: sum.toString(), ref: BigInt(ref).toString(), wasmMs, jsMs, wasm: median(wasmMs), js: median(jsMs) });
  } catch (err) {
    const message = String((err && err.message) || err);
    self.postMessage({ type: 'error', message: err instanceof WebAssembly.RuntimeError ? `The kernel trapped: ${message}. Omega stops on integer overflow instead of wrapping.` : message });
  }
};
