// SPDX-License-Identifier: Apache-2.0
// Read-only verification oracle; no code/IR/wasm generation.
'use strict';
const fs = require('node:fs');
const assert = require('node:assert/strict');
const [wasmPath, tablePath, statusPath] = process.argv.slice(2);
const bytes = fs.readFileSync(wasmPath);
assert(WebAssembly.validate(bytes));
const moduleObject = new WebAssembly.Module(bytes);
assert.deepEqual(WebAssembly.Module.imports(moduleObject), []);
const { fresh } = new WebAssembly.Instance(moduleObject).exports;
const rows = fs.readFileSync(tablePath, 'utf8').split('\n')
  .filter(line => line && !line.startsWith('#')).map(line => line.split(' '));
const native = fs.readFileSync(statusPath, 'utf8').trim().split('\n').map(Number);
assert.equal(native.length, rows.length);
for (const [index, [now, generated, previous, stale, ok, expected]] of rows.entries()) {
  const result = fresh(BigInt(now), BigInt(generated), BigInt(previous), BigInt(stale), ok === 'true' ? 1 : 0);
  assert.equal(result, native[index], `native/wasm case ${index + 1}`);
  assert.equal(result, Number(expected), `specified boundary case ${index + 1}`);
}
console.log(`PASS freshness native/wasm boundary cases=${rows.length}; all four states`);
