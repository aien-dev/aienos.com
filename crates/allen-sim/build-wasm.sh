#!/bin/sh
# Rebuilds allen.wasm from crates/allen-sim and checks it against the kernel's own frame hashes.
# Run from the repository root. Needs the wasm32-unknown-unknown target (rustup target add ...).
set -e
cargo build --release --target wasm32-unknown-unknown -p allen-sim
cp target/wasm32-unknown-unknown/release/allen_sim.wasm public/allen/allen.wasm
node evidence/allen-sim/wasm-check.mjs public/allen/allen.wasm evidence/allen-sim/reference.txt
