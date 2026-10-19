// node wasm-check.mjs <allen.wasm> <reference.txt>: runs the built WebAssembly through the same script
// as reference.c at 1280x720 and compares the screen hash every 64 ms with the kernel's own lines.
import { readFileSync } from 'node:fs';
const [, , wasmPath, refPath] = process.argv;
const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
const x = instance.exports;
const enc = new TextEncoder();
const put = (bytes) => { new Uint8Array(x.memory.buffer).set(bytes, x.allen_scratch()); return bytes.length; };
const str = (s) => put(enc.encode(s));
const EA = [0, 0, 1, 2, 3, 3, 4, 5, 6, 7, 8, 9, 10, 2, 5], EB = [1, 2, 3, 3, 4, 5, 6, 7, 8, 9, 10, 11, 11, 7, 9];
const lines = readFileSync(refPath, 'utf8').split('\n');
const first = lines.indexOf('size 1280x720');
const want = lines.slice(first + 1, lines.indexOf('asleep 0 on 1', first));
const W = 1280, H = 720;
if (x.allen_new(W, H) !== 0) throw new Error('start failed');
// FNV-1a style 64-bit hash over the R, G, B bytes of every pixel (alpha ignored), in 32-bit halves
function hash() {
  const px = new Uint8Array(x.memory.buffer, x.allen_screen(), W * H * 4);
  let hi = 0x14650fb0, lo = 0x739d0383; // the reference uses the basis 1469598103934665603
  for (let i = 0; i < px.length; i += 4) {
    for (let b = 0; b < 3; b++) {
      lo = (lo ^ px[i + b]) >>> 0;
      const m = lo * 0x1b3, carry = Math.floor(m / 4294967296);
      const nlo = m >>> 0;
      hi = (Math.imul(hi, 0x1b3) + carry + (lo << 8)) >>> 0;
      lo = nlo;
    }
  }
  return hi.toString(16).padStart(8, '0') + lo.toString(16).padStart(8, '0');
}
const at = (t, v) => t >= v && t < v + 16;
const typed = 'Hello Allen';
let bad = 0, n = 0;
for (let t = 0; t <= 44000; t += 16) {
  if (at(t, 14000)) x.allen_wake(str('Say anything'));
  if (t >= 15000 && t < 15000 + 11 * 140 && (t - 15000) % 140 < 16) {
    x.allen_input(str(typed.slice(0, Math.floor((t - 15000) / 140) + 1)));
  }
  if (at(t, 17000)) { const a = enc.encode('Hello Allen'), r = enc.encode('Hello Humanity. I am here, and the night is clear.'); x.allen_said(0, put(new Uint8Array([...a, ...r])) && a.length, r.length); }
  if (at(t, 25000)) { const l = enc.encode('a note about the night sky'); put(new Uint8Array([...EA, ...EB, ...l])); x.allen_map(12, 15, 3, l.length, 12, 15); }
  if (at(t, 30000)) x.allen_input(str('what is'));
  if (at(t, 31000)) { const a = enc.encode('what is water'); put(a); x.allen_said(2, a.length, 0); }
  if (at(t, 33000)) { const r = enc.encode('Water is where I live.'); put(r); x.allen_said(1, 0, r.length); }
  if (at(t, 36000)) x.allen_sleep(1);
  if (at(t, 41000)) x.allen_sleep(0);
  const drew = x.allen_tick(t);
  if ((t / 16) % 4 === 0) {
    const line = `${t} ${drew} ${hash()}`;
    if (line !== want[n]) { if (!bad) console.log(`first difference at line ${n + 2}: wasm ${line} kernel ${want[n]}`); bad++; }
    n++;
  }
}
console.log(bad ? `WASM_CHECK: FAIL (${bad} of ${n} lines differ)` : `WASM_CHECK: PASS (${n} frames match the kernel's hashes)`);
process.exit(bad ? 1 : 0);
