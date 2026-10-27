# Site build: host tools outside Omega

First-party site code belongs in seeded Omega. Everything below is a recorded host tool: what it does, why it is not `.om` yet and the `.om` program that replaces it. The public Aien services keep their own list in `scripts/allen/DEPENDENCIES.md`.

| Tool | Version | Used for | Run | Why not `.om` yet | `.om` successor |
|---|---|---|---|---|---|
| `qrencode` | 4.1.1 (Ubuntu noble `qrencode` 4.1.1-1build2, `libqrencode4` 4.1.1-1build2) | Generated `public/assets/support-btc-qr.svg`, the QR of `bitcoin:bc1qtfzkt7un99e3hrmc6uh3z7ccy6hny7axvxkgd2` on `/support/` | Once, by hand, on 2026-10-27: `qrencode -t SVG --svg-path -l M -m 4 -s 8 -o public/assets/support-btc-qr.svg 'bitcoin:<address>'`. The output is committed; neither CI nor the build runs it | No QR encoder exists in seeded Omega yet | `om/support-page/qr.om`: byte-mode QR encoder (Reed-Solomon over GF(256), masking, SVG path emit) |

Verification oracle (read-only, not in the build path): `zbarimg` 0.23.93 (Ubuntu noble `zbar-tools` 0.23.93-4build3) decodes a headless Chrome rasterization of the committed SVG. The decoded text must equal `bitcoin:` followed by the address on the page, byte for byte. Rerun that decode whenever the SVG or the address changes.
