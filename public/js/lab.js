'use strict';
(() => {
  /* /lab/: compile Omega on the shared gateway, run the wasm in a worker in this
     tab, check it against plain JavaScript. Every number here is measured in this
     tab or read from /lab/record.tsv, which record.sh wrote on the Spark. */
  const $ = (id) => document.getElementById(id);
  const src = $('lab-src');
  if (!src) return;
  const ORIGINAL = src.value;
  const LINE = 'ret add mul a x y';
  const API = '/aien/api/omega';
  const LIMIT_MS = 15000;
  const fmt = (x) => Number(x).toLocaleString('en-US');
  const ms = (x) => `${x < 10 ? x.toFixed(2) : x < 100 ? x.toFixed(1) : Math.round(x)} ms`;
  const state = { bytes: null, recorded: '', worker: null, timer: 0, label: 'prebuilt' };
  const status = (text, kind) => {
    const box = $('lab-status');
    box.textContent = text;
    box.dataset.kind = kind || '';
  };
  async function sha256(bytes) {
    if (!crypto.subtle) return '';
    const d = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
    return Array.from(d, (b) => b.toString(16).padStart(2, '0')).join('');
  }
  function hexdump(bytes) {
    const rows = [];
    for (let i = 0; i < Math.min(bytes.length, 1024); i += 16) {
      const row = Array.from(bytes.subarray(i, i + 16), (b) => b.toString(16).padStart(2, '0')).join(' ');
      rows.push(`${i.toString(16).padStart(4, '0')}  ${row}`);
    }
    if (bytes.length > 1024) rows.push(`... ${fmt(bytes.length - 1024)} more bytes`);
    return rows.join('\n');
  }
  function worker() {
    if (state.worker) return state.worker;
    state.worker = new Worker('/js/lab-worker.js');
    return state.worker;
  }
  function stop() {
    clearTimeout(state.timer);
    if (state.worker) state.worker.terminate();
    state.worker = null;
  }
  function ask(message) {
    return new Promise((resolve) => {
      const w = worker();
      state.timer = setTimeout(() => {
        stop();
        resolve({ type: 'error', message: `Stopped after ${LIMIT_MS / 1000} s. The worker was ended, so this tab stays responsive.` });
      }, LIMIT_MS);
      w.onmessage = (event) => { clearTimeout(state.timer); resolve(event.data); };
      w.onerror = (event) => { clearTimeout(state.timer); stop(); resolve({ type: 'error', message: event.message || 'worker failed' }); };
      w.postMessage(message);
    });
  }
  async function load(bytes, label) {
    stop();
    const reply = await ask({ type: 'load', bytes });
    if (reply.type !== 'loaded') { status(reply.message, 'fail'); return false; }
    state.bytes = bytes;
    state.label = label;
    const hash = await sha256(bytes);
    const same = hash && hash === state.recorded;
    $('lab-wasm').textContent = hexdump(bytes);
    $('lab-wasm-meta').textContent = `${label}: ${fmt(bytes.length)} bytes, ${reply.imports} imports, exports ${reply.exports.join(', ')}. sha256 ${hash || 'unavailable'}${same ? ', byte for byte the file record.sh built on the Spark' : ''}.`;
    return true;
  }
  function choice() {
    const a = Math.max(1, Math.min(1000, Math.round(Number($('lab-a').value) || 3)));
    const n = Number($('lab-n').value);
    $('lab-a').value = a;
    return { a, n };
  }
  async function run() {
    if (!state.bytes) { status('The kernel is still loading.', 'wait'); return; }
    const { a, n } = choice();
    status(`Running ${fmt(n)} elements three times in a worker...`, 'wait');
    $('lab-run').disabled = true;
    const reply = await ask({ type: 'run', a, n });
    $('lab-run').disabled = false;
    if (reply.type !== 'ran') { status(reply.message, 'fail'); return; }
    const match = reply.sum === reply.ref;
    $('lab-r-sum').textContent = fmt(reply.sum);
    $('lab-r-ref').textContent = fmt(reply.ref);
    $('lab-r-wasm').textContent = `${ms(reply.wasm)} (${reply.wasmMs.map(ms).join(', ')})`;
    $('lab-r-js').textContent = `${ms(reply.js)} (${reply.jsMs.map(ms).join(', ')})`;
    $('lab-r-rate').textContent = `${fmt(Math.round(n / reply.wasm / 1000))} million elements per second`;
    $('lab-verdict').textContent = match
      ? `MATCH. ${fmt(n)} elements, a = ${a}, ${state.label}.`
      : `MISMATCH. The kernel and the reference disagree. ${state.label}.`;
    $('lab-verdict').dataset.kind = match ? 'pass' : 'fail';
    for (const cell of document.querySelectorAll('[data-live]')) {
      const key = cell.dataset.live;
      cell.textContent = key === 'n' ? fmt(n) : key === 'time' ? ms(reply.wasm) : match ? 'MATCH' : 'MISMATCH';
    }
    status(match ? 'Checked against JavaScript: every element went into the checksum.' : 'The check caught a wrong kernel.', match ? 'pass' : 'fail');
  }
  async function compile(label) {
    const source = src.value;
    status('Compiling on the gateway: omc check, IR, irwasm...', 'wait');
    $('lab-compile').disabled = true;
    const t = performance.now();
    let reply;
    try {
      const response = await fetch(API, { method: 'POST', credentials: 'omit', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ source }) });
      reply = response.headers.get('content-type')?.includes('json') ? await response.json() : { ok: false, message: `HTTP ${response.status}` };
    } catch (err) {
      reply = { ok: false, message: `The compile service is unreachable (${err.message}). The prebuilt kernel still runs.` };
    }
    const trip = performance.now() - t;
    $('lab-compile').disabled = false;
    if (!reply.ok) {
      const where = reply.line ? `line ${reply.line}: ` : '';
      status(`${reply.stage ? `${reply.stage} ` : ''}${reply.code ? `error ${reply.code} ` : ''}${where}${reply.message || 'compile failed'}${reply.hint ? `. ${reply.hint}` : ''}`, 'fail');
      if (reply.line) {
        const lines = source.split('\n');
        const start = lines.slice(0, reply.line - 1).join('\n').length + (reply.line > 1 ? 1 : 0);
        src.focus();
        src.setSelectionRange(start, start + (lines[reply.line - 1] || '').length);
      }
      return false;
    }
    const bytes = Uint8Array.from(atob(reply.wasm), (c) => c.charCodeAt(0));
    $('lab-c-server').textContent = `${fmt(reply.compile_ms)} ms`;
    $('lab-c-trip').textContent = ms(trip);
    $('lab-c-pin').textContent = `omc ${String(reply.omc_sha256 || '').slice(0, 12)}, irwasm ${String(reply.irwasm_sha256 || '').slice(0, 12)}`;
    const ok = await load(bytes, label || 'your compile');
    if (ok) await run();
    return ok;
  }
  async function start() {
    $('lab-run').addEventListener('click', run);
    $('lab-compile').addEventListener('click', () => compile('your compile'));
    $('lab-break').addEventListener('click', () => {
      if (!src.value.includes(LINE)) src.value = ORIGINAL;
      src.value = src.value.replace(LINE, 'ret sub mul a x y');
      compile('broken on purpose: add changed to sub');
    });
    $('lab-reset').addEventListener('click', () => { src.value = ORIGINAL; status('Source reset. Compile or run.', ''); });
    const copy = $('lab-copy');
    copy.addEventListener('click', async () => {
      try { await navigator.clipboard.writeText($('lab-home-cmd').textContent); copy.textContent = 'Copied'; } catch { copy.textContent = 'Select and copy'; }
      setTimeout(() => { copy.textContent = 'Copy'; }, 2000);
    });
    try {
      const tsv = await (await fetch('/lab/record.tsv', { cache: 'no-cache' })).text();
      const rec = Object.fromEntries(tsv.trim().split('\n').map((l) => l.split('\t')));
      state.recorded = rec.wasm_sha256 || '';
      for (const cell of document.querySelectorAll('[data-us]')) if (rec[cell.dataset.us]) cell.textContent = ms(Number(rec[cell.dataset.us]) / 1000);
      for (const cell of document.querySelectorAll('[data-bytes]')) if (rec[cell.dataset.bytes]) cell.textContent = `${fmt(rec[cell.dataset.bytes])} bytes`;
    } catch { /* the cells keep their link to record.tsv */ }
    try {
      const bytes = new Uint8Array(await (await fetch('/lab/axpy.wasm')).arrayBuffer());
      if (await load(bytes, 'prebuilt /lab/axpy.wasm')) status('Ready. Run uses the prebuilt kernel; Compile builds your edit on the gateway.', 'pass');
    } catch (err) {
      status(`Could not load the prebuilt kernel: ${err.message}`, 'fail');
    }
  }
  start();
})();
