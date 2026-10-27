'use strict';
self.addEventListener('message', async (event) => {
  const job = event.data || {};
  try {
    const instance = await WebAssembly.instantiate(job.module, {});
    const fn = instance.exports[job.name];
    if (typeof fn !== 'function') throw new Error('no exported function ' + job.name);
    const value = fn(...(Array.isArray(job.args) ? job.args : []));
    self.postMessage({ id: job.id, ok: true, value });
  } catch (error) {
    self.postMessage({ id: job.id, ok: false, trap: error instanceof WebAssembly.RuntimeError, message: String((error && error.message) || error) });
  }
});
