# allen-sim

Allen's window, running in your browser. This crate is the AIENOS kernel's own drawing code
(`native/kernel/ui` in aien-dev/aienos: the stroke typeface, the night sky, the black holes, the dusk,
the water, the orb, the chirp), rewritten in Rust line by line and built to WebAssembly. It keeps the
kernel's integer arithmetic exactly, so it draws the same pixels as the C code that runs on the iMac.
`tests/` compares its frames with frames written by the original C code at fixed times.

Status: work in progress. Done so far: math kit, typeface, night sky, black holes, dusk.
