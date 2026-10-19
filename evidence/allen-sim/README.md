# allen-sim evidence

`reference.c` drives the kernel's own window code (native/kernel/ui, snapshot 903277b) through a fixed
script (night, merge, Allen forms, typing, answer, thinking, notes map, dusk, dawn) at 1280x720 and 800x500
and prints a hash of the whole screen every 64 ms, then a hash of the chirp sound.
`build-reference.sh` builds and runs it with the host C compiler and writes `reference.txt`.
`crates/allen-sim/tests/reference.rs` runs the Rust port through the same script and compares line by line.
Result: all 1381 lines match (screen pixels, alpha ignored, and the sound).

Other files here: `wasm-check.mjs` (loads the built WebAssembly in Node and repeats the hash check),
`video.c` and `build-video.sh` (write the frames for `public/allen/allen-window.mp4` from the kernel's own
code on the Spark; this is not a capture from the iMac), `video-contact-sheet.png` (frames from that video),
`drive.js` and `page-check.js` (drive the built page in headless Chrome: screenshots, no console errors,
no outside requests, no sideways overflow at 390 px wide).

Screenshots of the built page in headless Chrome (1280 and 390 px wide) are in `screens/`.
