# allen-sim

Allen's window, running in your browser. This crate is the AIENOS kernel's own drawing code
(`native/kernel/ui` in aien-dev/aienos: the stroke typeface, the night sky, the black holes, the dusk,
the water, the orb, the chirp), rewritten in Rust line by line and built to WebAssembly. It keeps the
kernel's integer arithmetic exactly, so it draws the same pixels as the C code that runs on the iMac.
`tests/` compares its frames with frames written by the original C code at fixed times.

Status: every module is ported (math kit, typeface, night sky, water, orb, black holes, dusk and dawn, typing,
answer, notes map, the chirp). `tests/reference.rs` compares the Rust frames and the chirp with the C code at
fixed times: all 1381 lines match. `build-wasm.sh` builds `public/allen/allen.wasm` and checks it in Node.
Evidence and how to rerun it: `evidence/allen-sim/README.md`.
