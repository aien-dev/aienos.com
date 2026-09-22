# AIENOS Web Platform (aienos.com)

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![GitHub Pages](https://github.com/aien-dev/aienos.com/actions/workflows/pages.yml/badge.svg)](https://github.com/aien-dev/aienos.com/actions/workflows/pages.yml)

The official web platform and documentation portal for **AIENOS**, the GPU-Native Neural Operating Environment.

Live: [https://aienos.com](https://aienos.com)

---

## Mission & Architecture

AIENOS eliminates the 40-year CPU von Neumann bottleneck by pairing Grace Blackwell GB10 unified memory with pure compiled native systems:

- **Coherent Unified Memory**: 128 GB shared physical address space between ARM Cortex controller cores and the Blackwell GPU.
- **Pure Native Silicon Priority**: Core daemons written in native Rust and Mojo 1.0. Zero Python or Node interpreter in the core runtime path.
- **Hardware TPM 2.0 Vault**: Bound to `/dev/tpmrm0` ECDSA P-256 keys. Zero plaintext `.env` secrets on disk.
- **Sub-5MB RSS Footprint**: Sovereign gateway (`openclaw-rs`) operating at 4.56 MB resident set size.

---

## Gandi.net DNS Record Configuration Reference

To bind `aienos.com` on Gandi.net to GitHub Pages:

| Type | Name | Target Value | TTL |
| :--- | :--- | :--- | :--- |
| `A` | `@` | `185.199.108.153` | 1800 |
| `A` | `@` | `185.199.109.153` | 1800 |
| `A` | `@` | `185.199.110.153` | 1800 |
| `A` | `@` | `185.199.111.153` | 1800 |
| `AAAA` | `@` | `2606:50c0:8000::153` | 1800 |
| `AAAA` | `@` | `2606:50c0:8001::153` | 1800 |
| `AAAA` | `@` | `2606:50c0:8002::153` | 1800 |
| `AAAA` | `@` | `2606:50c0:8003::153` | 1800 |
| `CNAME` | `www` | `aien-dev.github.io.` | 1800 |

---

## Universal Developer Installer

```bash
curl -fsSL https://aienos.com/install.sh | bash
```

---

## Local Development & Build

```bash
npm install
npm run dev
npm run build
npm run preview
```

---

## License

Licensed under the Apache License, Version 2.0 and the Sovereign Reciprocal Commons License (SRCL-1.0).
