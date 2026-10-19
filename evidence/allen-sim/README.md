# allen-sim evidence

`reference.c` drives the kernel's own window code (native/kernel/ui, snapshot 903277b) through a fixed
script (night, merge, Allen forms, typing, answer, thinking, notes map, dusk, dawn) at 1280x720 and 800x500
and prints a hash of the whole screen every 64 ms, then a hash of the chirp sound.
`build-reference.sh` builds and runs it with the host C compiler and writes `reference.txt`.
`crates/allen-sim/tests/reference.rs` runs the Rust port through the same script and compares line by line.
Result: all 1381 lines match (screen pixels, alpha ignored, and the sound).
